//! Port of `engine/journal.py` — the change journal, one record per write.
//!
//! Every note write in the system records itself here **before** touching a file. That ordering is
//! the invariant the whole S1 design rests on, and it is enforced in `write.rs`, not here; this
//! module owns the record shape, the read cache, and the queries built on top.
//!
//! # Documented deviation: `jsonable`
//!
//! Python's `jsonable()` exists because a Python dict can hold arbitrary objects — a `datetime`, a
//! `Path` — that `json.dumps` would reject. A `serde_json::Value` is already JSON by construction,
//! so the recursive walk has nothing to do and is not ported. What IS ported are the three
//! conversions it performs, as explicit constructors ([`json_datetime`], [`json_date`],
//! [`json_path`]), because their exact formats are observable in every record ever written:
//! a datetime is `%Y-%m-%dT%H:%M:%S` with **no** trailing `Z` and **no** subsecond, a date is ISO,
//! and a path is **POSIX-separated** even on Windows.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::Timestamp;
use serde_json::{Map, Value};

use crate::ledger::{JsonlLedger, Record};

pub const OPS: [&str; 6] = ["set", "create", "delete", "move", "append_body", "supersede"];

/// Declared in Python and, notably, **never enforced** — `make_record` validates `op` but not
/// `via`. Preserved: a typo'd `via` is written without complaint, exactly as today.
pub const VIAS: [&str; 5] = ["dashboard", "local-runner", "cloud-routine", "cli", "external"];

/// Python's `itertools.count(1)` — process-wide, starting at 1, never reset.
static SEQ: AtomicI64 = AtomicI64::new(1);

fn next_seq() -> i64 {
    SEQ.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    UnknownOp(String),
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JournalError::UnknownOp(op) => write!(f, "unknown op: {op}"),
        }
    }
}

/// Millisecond-precision UTC, `Z`-suffixed: `2026-08-29T12:00:00.123Z`.
///
/// Python builds this by hand (`strftime` then `microsecond // 1000`), so the millisecond field is
/// **truncated, not rounded**, and is always exactly three digits. This string is the ledger's day
/// key and its primary sort key, so the format is load-bearing rather than cosmetic.
pub fn now_ts(now: Option<Timestamp>) -> String {
    let moment = now.unwrap_or_else(Timestamp::now);
    let zoned = moment.to_zoned(TimeZone::UTC);
    let millis = moment.subsec_nanosecond() / 1_000_000;
    format!("{}.{:03}Z", zoned.strftime("%Y-%m-%dT%H:%M:%S"), millis)
}

/// `KNOWLU_DEVICE` (the cloud sandbox sets it to `cloud`), else the hostname, else `unknown`.
///
/// An **empty** env var falls through, because Python's `or` treats `""` as falsy. Verified on this
/// machine that `platform.node()`, `socket.gethostname()` and `%COMPUTERNAME%` all agree, so
/// `COMPUTERNAME` is a faithful source for `platform.node()` on Windows. The live journal contains
/// exactly two device values, `QuinnsPC` and `cloud`.
pub fn device_name() -> String {
    for key in ["KNOWLU_DEVICE", "COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return value;
            }
        }
    }
    "unknown".to_string()
}

/// Serialises every test in the crate that touches `KNOWLU_DEVICE`.
///
/// The variable is **process-global**, and `cargo test` runs the whole crate's tests in one
/// process, in parallel. A test that only ever *sets* it is not enough to guard against the race:
/// any test anywhere in the crate that *reads* [`device_name`] — directly, or through a function
/// that calls it, such as `runs::add_step`/`end_run` or `issues::open_issue` — can observe another
/// thread's in-flight `set_var`/`remove_var` mid-test. So this mutex is not module-local, and it is
/// not only for writers: **any test that reads `device_name()` must hold this lock too**, for the
/// whole span from its first read (including one made inside the production code it calls) through
/// whatever it compares that value against, or the read races the write.
#[cfg(test)]
pub(crate) static DEVICE_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `%Y-%m-%dT%H:%M:%S` — no `Z`, no subsecond. Python's `jsonable` datetime branch.
pub fn json_datetime(value: DateTime) -> Value {
    Value::String(value.strftime("%Y-%m-%dT%H:%M:%S").to_string())
}

pub fn json_date(value: Date) -> Value {
    Value::String(value.strftime("%Y-%m-%d").to_string())
}

/// POSIX separators even on Windows — `Path.as_posix()`.
pub fn json_path(value: &Path) -> Value {
    Value::String(value.to_string_lossy().replace('\\', "/"))
}

/// `quinn` 0, `agent` 1, `system` 2, anything else 3. Ranks on the part **before the first colon**,
/// so `agent:ingest.blackboard` ranks as `agent`.
pub fn actor_rank(actor: &str) -> i64 {
    match actor.split(':').next().unwrap_or("") {
        "quinn" => 0,
        "agent" => 1,
        "system" => 2,
        _ => 3,
    }
}

/// Fields of a journal record, in Python's parameter order.
pub struct NewRecord<'a> {
    pub op: &'a str,
    pub id: Option<&'a str>,
    pub path: &'a str,
    pub actor: &'a str,
    pub via: &'a str,
    pub field: Option<&'a str>,
    pub old: Value,
    pub new: Value,
    pub run_id: Option<&'a str>,
    pub evidence: Value,
    pub ts: Option<String>,
    pub device: Option<String>,
}

impl<'a> NewRecord<'a> {
    pub fn new(op: &'a str, path: &'a str, actor: &'a str, via: &'a str) -> NewRecord<'a> {
        NewRecord {
            op,
            id: None,
            path,
            actor,
            via,
            field: None,
            old: Value::Null,
            new: Value::Null,
            run_id: None,
            evidence: Value::Null,
            ts: None,
            device: None,
        }
    }
}

/// Build a record. **Validates `op` and nothing else** (see [`VIAS`]).
pub fn make_record(spec: NewRecord<'_>) -> Result<Record, JournalError> {
    if !OPS.contains(&spec.op) {
        return Err(JournalError::UnknownOp(spec.op.to_string()));
    }
    let mut rec = Map::new();
    let opt = |v: Option<&str>| v.map(|s| Value::String(s.to_string())).unwrap_or(Value::Null);
    rec.insert("ts".into(), Value::String(spec.ts.unwrap_or_else(|| now_ts(None))));
    rec.insert("device".into(), Value::String(spec.device.unwrap_or_else(device_name)));
    rec.insert("actor".into(), Value::String(spec.actor.to_string()));
    rec.insert("via".into(), Value::String(spec.via.to_string()));
    rec.insert("run_id".into(), opt(spec.run_id));
    rec.insert("op".into(), Value::String(spec.op.to_string()));
    rec.insert("id".into(), opt(spec.id));
    rec.insert("path".into(), Value::String(spec.path.to_string()));
    rec.insert("field".into(), opt(spec.field));
    rec.insert("old".into(), spec.old);
    rec.insert("new".into(), spec.new);
    rec.insert("evidence".into(), spec.evidence);
    Ok(rec)
}

fn str_of(rec: &Record, key: &str) -> Option<String> {
    match rec.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// `(id, field)` -> the latest record that set that field.
///
/// A `create` expands to **one entry per field of its `new` mapping**, rewritten with that field's
/// name, a `None` old and that field's value — because a note minted with a value already set is
/// as much a decision as a later `set`. Input must already be sorted by `(ts, device, seq)`, which
/// `ledger.read` guarantees; the last writer for a key simply wins by overwrite.
pub fn latest_by_field(records: &[Record]) -> std::collections::BTreeMap<(String, String), Record> {
    let mut latest = std::collections::BTreeMap::new();
    for rec in records {
        let Some(note_id) = str_of(rec, "id").filter(|s| !s.is_empty()) else {
            continue;
        };
        let op = str_of(rec, "op").unwrap_or_default();
        if op == "create" {
            if let Some(Value::Object(fields)) = rec.get("new") {
                for (field, value) in fields {
                    let mut expanded = rec.clone();
                    expanded.insert("field".into(), Value::String(field.clone()));
                    expanded.insert("old".into(), Value::Null);
                    expanded.insert("new".into(), value.clone());
                    latest.insert((note_id.clone(), field.clone()), expanded);
                }
            }
        } else if op == "set" {
            if let Some(field) = str_of(rec, "field").filter(|s| !s.is_empty()) {
                latest.insert((note_id.clone(), field), rec.clone());
            }
        }
    }
    latest
}

pub struct Journal {
    pub vault: PathBuf,
    pub ledger: JsonlLedger,
    cache: Option<Vec<Record>>,
}

impl Journal {
    pub fn new(vault: impl Into<PathBuf>) -> Journal {
        let vault = vault.into();
        let ledger = JsonlLedger::new(vault.join("state").join("journal"));
        Journal { vault, ledger, cache: None }
    }

    pub fn warnings(&self) -> &[String] {
        &self.ledger.warnings
    }

    /// Appends, stamping `seq` only if the caller has not. Invalidates the read cache.
    pub fn append(&mut self, rec: &mut Record) -> Result<(), crate::ledger::LedgerError> {
        rec.entry("seq".to_string()).or_insert_with(|| Value::from(next_seq()));
        self.ledger.append(rec)?;
        self.cache = None;
        Ok(())
    }

    /// Cached **only** for the unbounded read — a bounded read is always a fresh ledger scan, and
    /// returns a copy so a caller cannot mutate the cache.
    pub fn read(&mut self, since: Option<&str>, until: Option<&str>) -> Vec<Record> {
        if since.is_none() && until.is_none() {
            if self.cache.is_none() {
                self.cache = Some(self.ledger.read(None, None));
            }
            return self.cache.clone().unwrap_or_default();
        }
        self.ledger.read(since, until)
    }

    pub fn records_for(&mut self, note_id: &str, field: Option<&str>) -> Vec<Record> {
        self.read(None, None)
            .into_iter()
            .filter(|r| str_of(r, "id").as_deref() == Some(note_id))
            .filter(|r| match field {
                None => true,
                Some(f) => str_of(r, "field").as_deref() == Some(f),
            })
            .collect()
    }

    /// Did Quinn set this field? Returns the latest such record.
    ///
    /// A human set shows up **two** ways: an explicit `op: set`, or the field arriving inside a
    /// `create` record's `new` mapping (the note was minted with the value already there, e.g. by
    /// hand in Obsidian). Both count — a create is as much a human decision as a set. This is the
    /// query S3's field freeze is built on, so missing the `create` half would let a source clobber
    /// a hand-entered value.
    pub fn human_set(&mut self, note_id: &str, field: &str) -> Option<Record> {
        self.records_for(note_id, None)
            .into_iter()
            .filter(|r| str_of(r, "actor").as_deref() == Some("quinn"))
            .filter(|r| {
                let op = str_of(r, "op").unwrap_or_default();
                (op == "set" && str_of(r, "field").as_deref() == Some(field))
                    || (op == "create"
                        && matches!(r.get("new"), Some(Value::Object(m)) if m.contains_key(field)))
            })
            .next_back()
    }

    /// One warning per record timestamped more than `minutes` after `now`.
    ///
    /// The record is still honoured — the later-timestamp-wins rule needs a total order and both
    /// machines are NTP-synced — but a device whose clock runs fast wins every conflict it takes
    /// part in, so the run must say so out loud. An unparseable `ts` is skipped silently here
    /// because `ledger.read` has already warned about it.
    pub fn skew_warnings(&mut self, now: Timestamp, minutes: i64) -> Vec<String> {
        let limit = now.as_second() + minutes * 60;
        let mut out = Vec::new();
        for rec in self.read(None, None) {
            let Some(raw) = str_of(&rec, "ts") else { continue };
            let Ok(ts) = raw.parse::<Timestamp>() else { continue };
            if ts.as_second() > limit {
                let ahead = (ts.as_second() - now.as_second()).div_euclid(60);
                // `rec.get("device")` may be absent; Python formats that as the literal "None".
                let device = match rec.get("device") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => "None".to_string(),
                };
                out.push(format!("{raw} from {device} is {ahead} min in the future"));
            }
        }
        out
    }

    /// Drop the read cache — call after a git pull or any external change to `state/journal/`.
    pub fn invalidate(&mut self) {
        self.cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-journal-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("state").join("journal")).unwrap();
        dir
    }

    fn set_rec(id: &str, field: &str, actor: &str, ts: &str) -> Record {
        let mut spec = NewRecord::new("set", "tasks/x.md", actor, "cli");
        spec.id = Some(id);
        spec.field = Some(field);
        spec.new = Value::String("v".into());
        spec.ts = Some(ts.to_string());
        spec.device = Some("TestPC".into());
        make_record(spec).unwrap()
    }

    #[test]
    fn now_ts_is_millisecond_utc_with_a_z() {
        let ts = "2026-08-29T12:00:00.123456Z".parse::<Timestamp>().unwrap();
        assert_eq!(now_ts(Some(ts)), "2026-08-29T12:00:00.123Z");
    }

    #[test]
    fn now_ts_truncates_milliseconds_rather_than_rounding() {
        let ts = "2026-08-29T12:00:00.999999Z".parse::<Timestamp>().unwrap();
        assert_eq!(now_ts(Some(ts)), "2026-08-29T12:00:00.999Z");
    }

    #[test]
    fn make_record_rejects_an_unknown_op() {
        let spec = NewRecord::new("frobnicate", "tasks/x.md", "quinn", "cli");
        assert_eq!(
            make_record(spec).unwrap_err(),
            JournalError::UnknownOp("frobnicate".into())
        );
    }

    #[test]
    fn make_record_does_not_validate_via() {
        // Preserved quirk: VIAS is declared and never enforced.
        let spec = NewRecord::new("set", "tasks/x.md", "quinn", "not-a-real-via");
        assert!(make_record(spec).is_ok());
    }

    #[test]
    fn actor_rank_ranks_on_the_prefix() {
        assert_eq!(actor_rank("quinn"), 0);
        assert_eq!(actor_rank("agent:ingest.blackboard"), 1);
        assert_eq!(actor_rank("system:migration"), 2);
        assert_eq!(actor_rank("someone-else"), 3);
    }

    #[test]
    fn json_path_uses_posix_separators_even_on_windows() {
        assert_eq!(json_path(Path::new(r"tasks\cs-100.md")), Value::String("tasks/cs-100.md".into()));
    }

    #[test]
    fn json_datetime_has_no_zone_and_no_subsecond() {
        let dt = DateTime::constant(2026, 8, 26, 23, 59, 0, 0);
        assert_eq!(json_datetime(dt), Value::String("2026-08-26T23:59:00".into()));
    }

    #[test]
    fn append_then_read_round_trips() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut rec = set_rec("id1", "due", "quinn", "2026-08-29T12:00:00.000Z");
        j.append(&mut rec).unwrap();
        assert!(rec.contains_key("seq"), "append must stamp seq");
        let back = j.read(None, None);
        assert_eq!(back.len(), 1);
        assert_eq!(str_of(&back[0], "id").as_deref(), Some("id1"));
    }

    #[test]
    fn append_invalidates_the_read_cache() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut a = set_rec("id1", "due", "quinn", "2026-08-29T12:00:00.000Z");
        j.append(&mut a).unwrap();
        assert_eq!(j.read(None, None).len(), 1);
        let mut b = set_rec("id2", "due", "quinn", "2026-08-29T12:00:01.000Z");
        j.append(&mut b).unwrap();
        assert_eq!(j.read(None, None).len(), 2, "a stale cache would still say 1");
    }

    #[test]
    fn human_set_finds_an_explicit_set() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut rec = set_rec("id1", "due", "quinn", "2026-08-29T12:00:00.000Z");
        j.append(&mut rec).unwrap();
        assert!(j.human_set("id1", "due").is_some());
        assert!(j.human_set("id1", "effort_hours").is_none());
    }

    #[test]
    fn human_set_also_finds_a_field_minted_inside_a_create() {
        // The half that S3's freeze depends on: a note hand-created in Obsidian with the value
        // already set counts as a human decision.
        let v = vault();
        let mut j = Journal::new(&v);
        let mut spec = NewRecord::new("create", "tasks/x.md", "quinn", "dashboard");
        spec.id = Some("id1");
        spec.new = serde_json::json!({"due": "2026-09-01T23:59", "title": "x"});
        spec.ts = Some("2026-08-29T12:00:00.000Z".into());
        spec.device = Some("TestPC".into());
        let mut rec = make_record(spec).unwrap();
        j.append(&mut rec).unwrap();
        assert!(j.human_set("id1", "due").is_some());
        assert!(j.human_set("id1", "missing").is_none());
    }

    #[test]
    fn human_set_ignores_agent_writes() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut rec = set_rec("id1", "due", "agent:ingest.blackboard", "2026-08-29T12:00:00.000Z");
        j.append(&mut rec).unwrap();
        assert!(j.human_set("id1", "due").is_none());
    }

    #[test]
    fn latest_by_field_expands_a_create_into_one_entry_per_field() {
        let mut spec = NewRecord::new("create", "tasks/x.md", "quinn", "dashboard");
        spec.id = Some("id1");
        spec.new = serde_json::json!({"due": "2026-09-01T23:59", "title": "x"});
        spec.ts = Some("2026-08-29T12:00:00.000Z".into());
        let created = make_record(spec).unwrap();
        let latest = latest_by_field(&[created]);
        assert_eq!(latest.len(), 2);
        let due = &latest[&("id1".to_string(), "due".to_string())];
        assert_eq!(str_of(due, "field").as_deref(), Some("due"));
        assert_eq!(due.get("old"), Some(&Value::Null));
    }

    #[test]
    fn latest_by_field_keeps_the_last_writer() {
        let a = set_rec("id1", "due", "quinn", "2026-08-29T12:00:00.000Z");
        let mut b = set_rec("id1", "due", "quinn", "2026-08-29T13:00:00.000Z");
        b.insert("new".into(), Value::String("later".into()));
        let latest = latest_by_field(&[a, b]);
        assert_eq!(latest.len(), 1);
        assert_eq!(
            latest[&("id1".to_string(), "due".to_string())].get("new"),
            Some(&Value::String("later".into()))
        );
    }

    #[test]
    fn skew_warnings_flag_a_record_from_the_future() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut rec = set_rec("id1", "due", "quinn", "2026-08-29T12:30:00.000Z");
        j.append(&mut rec).unwrap();
        let now = "2026-08-29T12:00:00.000Z".parse::<Timestamp>().unwrap();
        let warnings = j.skew_warnings(now, 5);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("is 30 min in the future"), "{}", warnings[0]);
        assert!(warnings[0].contains("from TestPC"));
    }

    #[test]
    fn skew_warnings_are_silent_inside_the_tolerance() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut rec = set_rec("id1", "due", "quinn", "2026-08-29T12:02:00.000Z");
        j.append(&mut rec).unwrap();
        let now = "2026-08-29T12:00:00.000Z".parse::<Timestamp>().unwrap();
        assert!(j.skew_warnings(now, 5).is_empty());
    }
}
