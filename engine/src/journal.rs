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

/// The human (either token, ruling 11) 0, `agent` 1, `system` 2, anything else 3. Ranks on the
/// part **before the first colon**, so `agent:ingest.blackboard` ranks as `agent`.
pub fn actor_rank(actor: &str) -> i64 {
    match actor.split(':').next().unwrap_or("") {
        HUMAN_ACTOR | LEGACY_HUMAN_ACTOR => 0,
        "agent" => 1,
        "system" => 2,
        _ => 3,
    }
}

// ---------------------------------------------------------------------------
// The human actor (cloud design, Amendment 2026-09-29, ruling 11)
// ---------------------------------------------------------------------------

/// The token a vault the app creates writes for its student: never a person's name (CLAUDE.md
/// rule 1).
pub const HUMAN_ACTOR: &str = "student";

/// The token every vault made before ruling 11 writes, and keeps writing: its journal lines and
/// frontmatter stay byte-identical. Every comparison reads it as the same human as
/// [`HUMAN_ACTOR`], forever. This is the one place the spelling may appear outside tests.
pub const LEGACY_HUMAN_ACTOR: &str = "quinn";

/// Where a vault states its token: vault-relative, POSIX. Absent means [`LEGACY_HUMAN_ACTOR`].
pub const ACTOR_FILE: &str = "config/actor.yaml";

/// Is this actor the vault's human? An exact match on either token: `Quinn`, `student ` and an
/// `agent:`/`system:` actor are not.
pub fn is_human(actor: &str) -> bool {
    actor == HUMAN_ACTOR || actor == LEGACY_HUMAN_ACTOR
}

/// The one key `config/actor.yaml` holds.
const ACTOR_KEY: &str = "human_actor";

/// The refusal every bad `config/actor.yaml` shares: the file, what was found, the two values.
fn actor_file_error(found: &str) -> ActorFileError {
    ActorFileError(format!(
        "{ACTOR_FILE}: {found}; the file must be one line, {ACTOR_KEY}: {HUMAN_ACTOR} or {ACTOR_KEY}: {LEGACY_HUMAN_ACTOR}"
    ))
}

/// A value as the refusal shows it: escaped (so it is one line) and clipped (so a pasted paragraph
/// is not echoed back whole).
fn shown(value: &str) -> String {
    let mut clipped: String = value.chars().take(40).collect();
    if value.chars().count() > 40 {
        clipped.push('…');
    }
    format!("{clipped:?}")
}

/// Why `config/actor.yaml` cannot be used. The text names the file, the value found (clipped,
/// escaped) and the two accepted values, and never carries more of the file than that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorFileError(pub String);

impl std::fmt::Display for ActorFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The vault's human token, from `config/actor.yaml`: [`HUMAN_ACTOR`] or [`LEGACY_HUMAN_ACTOR`].
///
/// **An absent file is the legacy token**, so every vault made before ruling 11 keeps writing
/// exactly what it always wrote. Anything else the file can hold that is not one of the two tokens
/// — a name, an `agent:` actor `provenance::is_agent` would read as an agent, an empty value, no
/// key, not a mapping, not readable — is a named error, and no human write proceeds until the
/// student fixes it (the gate is `write`'s). Read through `pystr` (CRLF-aware), parsed by the
/// crate's YAML reader; nothing is cached, so a hand-edit takes effect on the next call.
pub fn read_human_actor(vault: &Path) -> Result<&'static str, ActorFileError> {
    let text = match crate::pystr::read_text(&vault.join(ACTOR_FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LEGACY_HUMAN_ACTOR),
        Err(e) => return Err(actor_file_error(&format!("it could not be read ({e})"))),
    };
    // The parser's own message is not shown: it can quote the text around the fault.
    let Ok(serde_yaml_ng::Value::Mapping(map)) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
        return Err(actor_file_error("it is not a YAML mapping"));
    };
    use serde_yaml_ng::Value as Y;
    let found = match crate::yaml::get(&map, ACTOR_KEY) {
        None => format!("{ACTOR_KEY} is missing"),
        Some(Y::Null) => format!("{ACTOR_KEY} is empty"),
        Some(Y::String(s)) if s.is_empty() => format!("{ACTOR_KEY} is empty"),
        Some(Y::String(s)) if s == HUMAN_ACTOR => return Ok(HUMAN_ACTOR),
        Some(Y::String(s)) if s == LEGACY_HUMAN_ACTOR => return Ok(LEGACY_HUMAN_ACTOR),
        Some(Y::Sequence(_)) => format!("{ACTOR_KEY} is a list"),
        Some(Y::Mapping(_)) => format!("{ACTOR_KEY} is a mapping"),
        Some(Y::Tagged(_)) => format!("{ACTOR_KEY} is a tagged value"),
        Some(other) => format!("{ACTOR_KEY} is {}", shown(&crate::yaml::text(other).unwrap_or_default())),
    };
    Err(actor_file_error(&found))
}

/// Create `config/actor.yaml` holding `token` — **once**. The file is opened create-new, so a
/// second call is an error and the first file's bytes are never touched; only the two tokens can
/// be written. One line, `human_actor: <token>`, ending in the vault's own newline
/// (`pystr::NEWLINE`, as every file `write` and the wizard make).
pub fn create_actor_file(vault: &Path, token: &str) -> Result<(), ActorFileError> {
    if !is_human(token) {
        return Err(actor_file_error(&format!("{} is not a human token", shown(token))));
    }
    let path = vault.join(ACTOR_FILE);
    let io = |e: std::io::Error| actor_file_error(&format!("it could not be written ({e})"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ActorFileError(format!("{ACTOR_FILE} already exists; it is written once and never rewritten")));
        }
        Err(e) => return Err(io(e)),
    };
    use std::io::Write;
    file.write_all(format!("{ACTOR_KEY}: {token}{}", crate::pystr::NEWLINE).as_bytes()).map_err(io)
}

/// The token an account already carries, from its journal records (ruling 11's restore rule):
/// [`LEGACY_HUMAN_ACTOR`] if **any** record's actor is that token, otherwise [`HUMAN_ACTOR`] —
/// including when there is no human record at all. Reads the records and nothing else; takes them
/// in any iterable of `&Record`, so `sync::pull`'s `(device, record)` pairs map straight in.
pub fn pick_human_actor<'a, I: IntoIterator<Item = &'a Record>>(records: I) -> &'static str {
    if records.into_iter().any(|r| str_of(r, "actor").as_deref() == Some(LEGACY_HUMAN_ACTOR)) {
        LEGACY_HUMAN_ACTOR
    } else {
        HUMAN_ACTOR
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

    /// Did the student set this field? Returns the latest such record, under either human token
    /// ([`is_human`]: ruling 11 reads `student` and `quinn` as the same human, forever).
    ///
    /// A human set shows up **two** ways: an explicit `op: set`, or the field arriving inside a
    /// `create` record's `new` mapping (the note was minted with the value already there, e.g. by
    /// hand in Obsidian). Both count — a create is as much a human decision as a set. This is the
    /// query S3's field freeze is built on, so missing the `create` half would let a source clobber
    /// a hand-entered value.
    pub fn human_set(&mut self, note_id: &str, field: &str) -> Option<Record> {
        self.records_for(note_id, None)
            .into_iter()
            .filter(|r| str_of(r, "actor").is_some_and(|a| is_human(&a)))
            .filter(|r| {
                let op = str_of(r, "op").unwrap_or_default();
                (op == "set" && str_of(r, "field").as_deref() == Some(field))
                    || (op == "create"
                        && matches!(r.get("new"), Some(Value::Object(m)) if m.contains_key(field)))
            })
            .next_back()
    }

    /// Did the student **edit** this field after the note existed? The latest `op: set` record by
    /// either human token ([`is_human`]) for it, or `None`. Unlike [`Journal::human_set`], a
    /// `create` record never counts: commitment change detection (§5.4) asks this, because the
    /// confirm screen mints every confirmed note as the student with the calendar's `meets`,
    /// `where` and `until` already in it — values the calendar chose, which a later calendar change
    /// must still be able to propose.
    pub fn human_edited(&mut self, note_id: &str, field: &str) -> Option<Record> {
        self.records_for(note_id, Some(field))
            .into_iter()
            .filter(|r| str_of(r, "actor").is_some_and(|a| is_human(&a)))
            .filter(|r| str_of(r, "op").as_deref() == Some("set"))
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
    fn human_edited_counts_a_later_set_but_never_the_create() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut spec = NewRecord::new("create", "commitments/x.md", "quinn", "dashboard");
        spec.id = Some("id1");
        spec.new = serde_json::json!({"meets": "x", "until": "2026-12-04"});
        spec.ts = Some("2026-08-29T12:00:00.000Z".into());
        let mut rec = make_record(spec).unwrap();
        j.append(&mut rec).unwrap();
        assert!(j.human_set("id1", "meets").is_some(), "human_set is unchanged");
        assert!(j.human_edited("id1", "meets").is_none());
        let mut agent = set_rec("id1", "meets", "agent:commitments", "2026-08-30T12:00:00.000Z");
        j.append(&mut agent).unwrap();
        assert!(j.human_edited("id1", "meets").is_none(), "an agent's set is not the student's");
        let mut edit = set_rec("id1", "meets", "quinn", "2026-08-31T12:00:00.000Z");
        j.append(&mut edit).unwrap();
        assert!(j.human_edited("id1", "meets").is_some());
        assert!(j.human_edited("id1", "until").is_none());
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

    // -- ruling 11: the human token -----------------------------------------

    /// A fresh scratch folder per call (never `vault()`, which one thread shares and wipes).
    fn scratch(tag: &str) -> PathBuf {
        static N: AtomicI64 = AtomicI64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "qo-journal-actor-{}-{}-{tag}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn actor_vault(tag: &str) -> PathBuf {
        let dir = scratch(tag);
        std::fs::create_dir_all(dir.join("config")).unwrap();
        dir
    }

    fn create_rec(id: &str, actor: &str, new: Value, ts: &str) -> Record {
        let mut spec = NewRecord::new("create", "tasks/x.md", actor, "dashboard");
        spec.id = Some(id);
        spec.new = new;
        spec.ts = Some(ts.to_string());
        spec.device = Some("TestPC".into());
        make_record(spec).unwrap()
    }

    #[test]
    fn the_two_tokens_are_the_constants_and_nothing_else() {
        assert_eq!(HUMAN_ACTOR, "student");
        assert_eq!(LEGACY_HUMAN_ACTOR, "quinn");
        assert!(is_human("student"));
        assert!(is_human("quinn"));
        for not in ["agent:x", "agent:knowlu.enrich", "system:migration", "Quinn", "Student", "student ", " quinn", "", "alice"] {
            assert!(!is_human(not), "{not:?} is not the vault's human");
        }
    }

    #[test]
    fn actor_rank_ranks_student_with_quinn() {
        assert_eq!(actor_rank("student"), 0);
        assert_eq!(actor_rank("quinn"), 0);
        assert_eq!(actor_rank("agent:ingest.blackboard"), 1);
        assert_eq!(actor_rank("system:migration"), 2);
        assert_eq!(actor_rank("someone-else"), 3);
    }

    #[test]
    fn human_set_and_human_edited_count_a_student_record() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut set = set_rec("id1", "due", "student", "2026-08-29T12:00:00.000Z");
        j.append(&mut set).unwrap();
        assert!(j.human_set("id1", "due").is_some(), "a student set counts for human_set");
        assert!(j.human_edited("id1", "due").is_some(), "…and for human_edited");

        let mut created = create_rec("id2", "student", serde_json::json!({"meets": "x"}), "2026-08-29T12:00:00.000Z");
        j.append(&mut created).unwrap();
        assert!(j.human_set("id2", "meets").is_some(), "a student create counts for human_set");
        assert!(j.human_edited("id2", "meets").is_none(), "a create never counts for human_edited");

        // Mixed tokens in one journal: the latest human record wins, whichever token wrote it.
        let mut q = set_rec("id3", "due", "quinn", "2026-08-29T12:00:00.000Z");
        j.append(&mut q).unwrap();
        let mut s = set_rec("id3", "due", "student", "2026-08-29T13:00:00.000Z");
        j.append(&mut s).unwrap();
        let latest = j.human_set("id3", "due").expect("a human set");
        assert_eq!(latest.get("actor"), Some(&Value::String("student".into())));
        assert_eq!(latest.get("ts"), Some(&Value::String("2026-08-29T13:00:00.000Z".into())));
        let edited = j.human_edited("id3", "due").expect("a human edit");
        assert_eq!(edited.get("ts"), Some(&Value::String("2026-08-29T13:00:00.000Z".into())));
    }

    #[test]
    fn an_absent_actor_file_reads_as_quinn() {
        let v = actor_vault("absent");
        assert_eq!(read_human_actor(&v), Ok("quinn"));
        // No `config/` at all is the same absence.
        assert_eq!(read_human_actor(&scratch("bare")), Ok("quinn"));
    }

    #[test]
    fn the_actor_file_accepts_only_student_or_quinn() {
        let v = actor_vault("accepts");
        let file = v.join("config").join("actor.yaml");
        for (bytes, want) in [
            ("human_actor: student\n", "student"),
            ("human_actor: quinn\n", "quinn"),
            ("human_actor: student\r\n", "student"),
            ("human_actor: quinn\r\n", "quinn"),
            ("human_actor: \"student\"\n", "student"),
            ("human_actor: 'quinn'\n", "quinn"),
            ("human_actor: student", "student"),
        ] {
            std::fs::write(&file, bytes).unwrap();
            assert_eq!(read_human_actor(&v), Ok(want), "{bytes:?}");
        }

        // Each refusal names the file and both accepted values, and says what it found.
        let refused = |bytes: &[u8], found: &str| {
            std::fs::write(&file, bytes).unwrap();
            let err = read_human_actor(&v).expect_err(&format!("{bytes:?} must be refused")).to_string();
            assert!(err.starts_with("config/actor.yaml: "), "{err}");
            assert!(err.contains("student") && err.contains("quinn"), "names both values: {err}");
            assert!(err.contains(found), "says what it found ({found}): {err}");
            assert!(!err.contains("PRIVATE"), "never more of the file than the value: {err}");
            err
        };
        refused(b"human_actor: alice\nnote: PRIVATE\n", "\"alice\"");
        refused(b"human_actor: agent:knowlu.enrich\n", "\"agent:knowlu.enrich\"");
        refused(b"human_actor: system:x\n", "\"system:x\"");
        refused(b"human_actor: Student\n", "\"Student\"");
        refused(b"human_actor: \"student \"\n", "\"student \"");
        refused(b"human_actor:\n", "empty");
        refused(b"human_actor: ''\n", "empty");
        refused(b"human_actor: [student]\n", "a list");
        refused(b"note: PRIVATE\n", "missing");
        refused(b"just PRIVATE text\n", "not a YAML mapping");
        refused(b"- PRIVATE\n", "not a YAML mapping");
        refused(b"", "not a YAML mapping");
        refused(b"human_actor: [unclosed PRIVATE\n", "not a YAML mapping");
        refused(b"human_actor: \xff\xfe PRIVATE\n", "could not be read");
        let long = refused(format!("human_actor: {}\n", "x".repeat(500)).as_bytes(), "\"xxxx");
        assert!(long.len() < 200, "a long value is clipped: {long}");

        // A directory where the file should be is unreadable, not absent.
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir_all(&file).unwrap();
        let err = read_human_actor(&v).expect_err("a directory is not a readable file").to_string();
        assert!(err.starts_with("config/actor.yaml: ") && err.contains("could not be read"), "{err}");
    }

    #[test]
    fn the_actor_file_is_written_once_and_never_rewritten() {
        let v = actor_vault("once");
        let file = v.join("config").join("actor.yaml");
        create_actor_file(&v, "student").unwrap();
        let bytes = std::fs::read(&file).unwrap();
        // One line, in the vault's own line ending (`pystr::NEWLINE`), like every file `write` makes.
        assert_eq!(bytes, format!("human_actor: student{}", crate::pystr::NEWLINE).into_bytes());
        assert_eq!(crate::pystr::read_text(&file).unwrap(), "human_actor: student\n");
        assert_eq!(read_human_actor(&v), Ok("student"));

        let again = create_actor_file(&v, "quinn").expect_err("a second call never rewrites");
        assert!(again.to_string().contains("config/actor.yaml"), "{again}");
        assert_eq!(std::fs::read(&file).unwrap(), bytes, "the bytes are exactly as they were");

        // A vault with no `config/` yet gets the folder; the legacy token is a valid file too.
        let fresh = scratch("fresh");
        create_actor_file(&fresh, "quinn").unwrap();
        assert_eq!(read_human_actor(&fresh), Ok("quinn"));

        // Only the two tokens can ever be written.
        for bad in ["alice", "agent:knowlu.enrich", "system:x", "", "Student"] {
            let dir = scratch("bad");
            assert!(create_actor_file(&dir, bad).is_err(), "{bad:?}");
            assert!(!dir.join("config").join("actor.yaml").exists(), "{bad:?}: nothing written");
        }
    }

    #[test]
    fn the_token_is_picked_from_the_accounts_records() {
        let student = set_rec("id1", "due", "student", "2026-08-29T12:00:00.000Z");
        let quinn = set_rec("id2", "due", "quinn", "2026-08-29T13:00:00.000Z");
        let agent = set_rec("id3", "due", "agent:knowlu.enrich", "2026-08-29T14:00:00.000Z");
        let system = create_rec("id4", "system:migration", serde_json::json!({"title": "x"}), "2026-08-29T15:00:00.000Z");
        assert_eq!(pick_human_actor([&student, &quinn]), "quinn", "quinn anywhere wins, even after student");
        assert_eq!(pick_human_actor([&quinn]), "quinn");
        assert_eq!(pick_human_actor([&student, &agent]), "student");
        assert_eq!(pick_human_actor(std::iter::empty::<&Record>()), "student", "no records: student");
        assert_eq!(pick_human_actor([&agent, &system]), "student", "no human record: student");
        // The shape `sync::pull` returns: `(device token, record)`.
        let pulled: Vec<(String, Record)> = vec![("d".into(), student.clone()), ("d".into(), quinn.clone())];
        assert_eq!(pick_human_actor(pulled.iter().map(|(_, r)| r)), "quinn");
    }

    #[test]
    fn a_student_record_is_a_quinn_record_with_one_value_changed() {
        let line = |actor: &str| {
            let mut spec = NewRecord::new("set", "tasks/x.md", actor, "dashboard");
            spec.id = Some("task_0123456789");
            spec.field = Some("due");
            spec.old = Value::Null;
            spec.new = Value::String("2026-09-01".into());
            spec.ts = Some("2026-08-29T12:00:00.000Z".into());
            spec.device = Some("TestPC".into());
            let mut rec = make_record(spec).unwrap();
            rec.insert("seq".into(), Value::from(7));
            crate::ledger::dumps_value(&Value::Object(rec))
        };
        let (q, s) = (line("quinn"), line("student"));
        assert_ne!(q, s);
        assert_eq!(s.replace("\"actor\": \"student\"", "\"actor\": \"quinn\""), q, "only the actor's value differs");
        assert_eq!(s.matches("student").count(), 1);
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
