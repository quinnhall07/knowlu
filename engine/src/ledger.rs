//! Port of `engine/ledger.py` — append-only, day-partitioned JSON-lines ledgers.
//!
//! This is the ONLY module that opens files under `state/journal/` and `state/runs/`.
//! Everything above it talks in records (JSON objects), so a hosted store could replace this
//! file store later without touching any consumer (spec §2, "seam"). The exclusivity is not a
//! style preference — it is the seam, and there is a test at the bottom that enforces it.
//!
//! # Documented deviations, stated once
//!
//! 1. **An unreadable *file* warns instead of raising.** Python lets the `OSError` from
//!    `read_text` propagate out of `read()`; the Rust signature returns `Vec<Record>` with no
//!    error channel (giving it one would push a `Result` through every consumer and move the
//!    seam). A file that will not read is therefore skipped with a warning, which is the same
//!    treatment its lines already get. Line-level behaviour — the part the tests and the spec
//!    pin down — is identical.
//! 2. **The diagnostic text inside a `malformed (...)` warning is serde_json's, not Python's.**
//!    Python interpolates `json.JSONDecodeError.msg` ("Expecting value"); no other parser can
//!    reproduce that wording. The `<file>:<line> malformed` prefix, which is what the tests and
//!    every human reader key off, is byte-identical.
//! 3. **`str()` and `int()` of a JSON *container*** (the sort key's `str(device)`, and
//!    `int(seq)`) fall back to compact JSON rather than Python's `repr`. Both are unreachable:
//!    a container `seq` is rejected by the `bad seq` guard before sorting, and no producer has
//!    ever written a non-scalar `device`. Scalars — including the `None`/`True`/`False`
//!    spellings — are exact.
//! 4. **Very large integers saturate.** Python's `int` is arbitrary precision; `i64` is not.
//!    Sequence numbers come from an in-process counter that starts at 1.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};

/// One ledger record. Python passes plain `dict`s around; this is the same thing with a name.
pub type Record = Map<String, Value>;

/// The line terminator `append` writes.
///
/// **Preserved quirk.** Python opens the day file with `open("a", encoding="utf-8")`, i.e.
/// `newline=None`, and that translates every `"\n"` written into `os.linesep` — so the live
/// Windows vault's journal really does hold CRLF, and the cloud sandbox's really does hold LF.
/// Verified against the interpreter, not inferred. `read` normalises both back, so the choice is
/// invisible to consumers and visible only in the bytes on disk — which is exactly why the port
/// keeps it rather than quietly standardising on `\n`.
use crate::pystr::NEWLINE;

/// Python raises `ValueError` / lets `OSError` escape; these are the two shapes that carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Python: `ValueError("ledger record needs an ISO 'ts'")`.
    MissingTs,
    Io(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::MissingTs => write!(f, "ledger record needs an ISO 'ts'"),
            LedgerError::Io(m) => write!(f, "io: {m}"),
        }
    }
}

fn io_err(e: io::Error) -> LedgerError {
    LedgerError::Io(e.to_string())
}

pub struct JsonlLedger {
    pub root: PathBuf,
    /// Warnings from the **latest** `read` only — `read` clears this first, so a second read of
    /// a clean ledger does not inherit the first read's complaints.
    pub warnings: Vec<String>,
}

impl JsonlLedger {
    pub fn new(root: impl Into<PathBuf>) -> JsonlLedger {
        JsonlLedger { root: root.into(), warnings: Vec::new() }
    }

    /// Day = the UTC date embedded in the record's own timestamp, never the wall clock: two
    /// devices appending the same record land in the same file.
    fn file_for(&self, ts: &str) -> PathBuf {
        self.root.join(format!("{}.jsonl", first_chars(ts, 10)))
    }

    /// Append one record, then flush, then fsync.
    ///
    /// The durability intent is the point: a run that records a write and then dies must not lose
    /// the record, because the journal is what makes the next run's attribution correct. The line
    /// and its terminator go out in a **single** `write_all` — two writes could interleave with a
    /// concurrent appender between them, and `merge=union` cannot repair a torn line.
    pub fn append(&self, record: &Record) -> Result<(), LedgerError> {
        // Python: `if not isinstance(ts, str) or len(ts) < 10`. `len` counts code points.
        let ts = match record.get("ts") {
            Some(Value::String(s)) if s.chars().count() >= 10 => s.clone(),
            _ => return Err(LedgerError::MissingTs),
        };
        fs::create_dir_all(&self.root).map_err(io_err)?;
        let line = format!("{}{NEWLINE}", dumps(record));
        let mut fh = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file_for(&ts))
            .map_err(io_err)?;
        fh.write_all(line.as_bytes()).map_err(io_err)?;
        fh.flush().map_err(io_err)?;
        fh.sync_all().map_err(io_err)?;
        Ok(())
    }

    /// Read every record in `[since, until]`, sorted by `(ts, str(device), int(seq or 0))`.
    ///
    /// Both bounds are compared as **strings** against the record's `ts`, which is what makes an
    /// ISO-8601 UTC timestamp usable as an ordering key without parsing anything. An empty-string
    /// bound is falsy in Python and so means "no bound"; [`truthy`] preserves that.
    pub fn read(&mut self, since: Option<&str>, until: Option<&str>) -> Vec<Record> {
        // Cleared before the is_dir bail-out, exactly as in Python: a read of a missing root
        // still resets the warnings.
        self.warnings.clear();
        let mut records: Vec<Record> = Vec::new();
        if !self.root.is_dir() {
            return records;
        }
        let since = truthy(since);
        // Normalize bare-date until to end of day for record-level filtering.
        let until = truthy(until).map(|u| {
            if u.chars().count() == 10 {
                format!("{u}T23:59:59.999Z")
            } else {
                u.to_string()
            }
        });

        for path in self.day_files() {
            let name = file_name(&path);
            let day =
                path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            // Day-level filtering uses [:10] prefixes; record-level filtering, below, uses the
            // full bound. Skipping a whole file is only a shortcut — the record filter repeats it.
            if let Some(s) = since {
                if day.as_str() < first_chars(s, 10).as_str() {
                    continue;
                }
            }
            if let Some(u) = &until {
                if day.as_str() > first_chars(u, 10).as_str() {
                    continue;
                }
            }
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                // Deviation 1 in the module docs: Python raises here.
                Err(err) => {
                    self.warnings.push(format!("{name} unreadable ({err})"));
                    continue;
                }
            };
            // split("\n"), never splitlines(): a record is one line of JSON written with
            // ensure_ascii=False, so U+2028/U+2029 (and \x0b \x0c \x1c-\x1e \x85) survive
            // verbatim INSIDE a value — an em-dashed title pasted from the web, a VHL bucket
            // name. splitlines() breaks on all of them and tears one record into two unparseable
            // halves, which then read as "malformed" and vanish. Rust's `.lines()` is closer but
            // still not close enough: it also splits on "\r\n" and swallows a trailing "\r",
            // which is a different rule again. So: normalise the way Python's text mode does
            // (universal newlines turn "\r\n" and a lone "\r" into "\n" at decode time — that is
            // what the Python comment means by "already normalised"), then split on "\n" alone.
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            let mut lines: Vec<&str> = text.split('\n').collect();
            if lines.last() == Some(&"") {
                lines.pop(); // trailing newline, not an empty record — and only ONE is popped
            }
            for (number, line) in lines.iter().enumerate().map(|(i, l)| (i + 1, *l)) {
                if py_strip(line).is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(err) => {
                        // merge=union keeps lines intact, but a crash mid-append can leave a torn
                        // line. Skip it loudly; never let one bad line cost the run (spec §6).
                        self.warnings.push(format!("{name}:{number} malformed ({err})"));
                        continue;
                    }
                };
                let rec = match value {
                    Value::Object(map) if matches!(map.get("ts"), Some(Value::String(_))) => map,
                    _ => {
                        self.warnings.push(format!("{name}:{number} not a record"));
                        continue;
                    }
                };
                // Validate seq if present. A JSON null is Python's None and skips the check.
                match rec.get("seq") {
                    None | Some(Value::Null) => {}
                    Some(seq) => {
                        if py_int(seq).is_none() {
                            self.warnings.push(format!("{name}:{number} bad seq"));
                            continue;
                        }
                    }
                }
                let ts = match rec.get("ts") {
                    Some(Value::String(ts)) => ts.as_str(),
                    _ => unreachable!("guarded above"),
                };
                if let Some(s) = since {
                    if ts < s {
                        continue;
                    }
                }
                if let Some(u) = &until {
                    if ts > u.as_str() {
                        continue;
                    }
                }
                records.push(rec);
            }
        }
        // (ts, str(device), int(seq or 0)). Python's sort is stable and so is
        // sort_by_cached_key, which matters when all three components tie.
        records.sort_by_cached_key(|r| {
            let ts = match r.get("ts") {
                Some(Value::String(ts)) => ts.clone(),
                _ => String::new(),
            };
            (ts, py_str(r.get("device")), sort_seq(r.get("seq")))
        });
        records
    }

    /// `sorted(self.root.glob("*.jsonl"))`.
    ///
    /// Sorting by file name is what Python's `Path` ordering reduces to when every path shares a
    /// parent. The extension match is case-sensitive; pathlib's glob is case-*insensitive* on
    /// Windows, but `append` only ever writes lowercase names, so the difference is unreachable.
    fn day_files(&self) -> Vec<PathBuf> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };
        let mut found: Vec<(String, PathBuf)> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "jsonl").unwrap_or(false))
            .map(|p| (file_name(&p), p))
            .collect();
        found.sort();
        found.into_iter().map(|(_, p)| p).collect()
    }

    /// The UTC date named by the earliest day file, or `None` when there is none. A file whose
    /// stem is not a date is skipped. `commitments::vault_day`'s fallback when no `ts` parses.
    pub fn first_day(&self) -> Option<jiff::civil::Date> {
        self.day_files()
            .iter()
            .filter_map(|p| p.file_stem()?.to_str()?.parse::<jiff::civil::Date>().ok())
            .min()
    }

    /// The smallest `ts` among the records of the earliest day file, or `None` when there is no
    /// file or no record in it carries a `ts`. A line that is not JSON is skipped. The earliest
    /// record of the whole ledger is in that file, because a file is named by its records' UTC
    /// date. `commitments::vault_day` counts the vault's days from this record's local date
    /// (phase-2 spec D2 as amended). It lives here because only this module names ledger files.
    pub fn first_ts(&self) -> Option<String> {
        let first = self.day_files().into_iter().next()?;
        let text = fs::read_to_string(first).ok()?;
        text.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            .filter_map(|v| v.get("ts").and_then(Value::as_str).map(str::to_string))
            .min()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Python's `s[:n]` — code points, not bytes, and a short string comes back whole.
fn first_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// `if since and ...` — an empty string is falsy in Python, so it means "no bound", not
/// "everything at or after the empty string" (which would be every record, and would also
/// suppress the bare-date normalisation).
fn truthy(bound: Option<&str>) -> Option<&str> {
    bound.filter(|b| !b.is_empty())
}

/// `str.strip()`.
///
/// Rust's `char::is_whitespace` (Unicode White_Space) and Python's `str.isspace()` agree
/// everywhere except U+001C–U+001F, the file/group/record/unit separators, which Python calls
/// whitespace and Rust does not. A line of nothing but those is silently skipped in Python and
/// would otherwise be reported here as malformed.
fn py_strip(line: &str) -> &str {
    crate::pystr::strip(line)
}

/// Python `int(x)`: truncates a real float, refuses a float-shaped *string*, counts `True` as 1.
/// Returning `None` is the `(ValueError, TypeError)` that becomes a `bad seq` warning.
///
/// Python also accepts digit separators (`int("1_000")`) and non-ASCII digits; neither has ever
/// appeared in a `seq`, which is written by an in-process counter.
fn py_int(value: &Value) -> Option<i64> {
    match value {
        Value::Bool(b) => Some(i64::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u as i64))
            .or_else(|| n.as_f64().map(|f| f.trunc() as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

/// Python's truthiness, for the `seq or 0` in the sort key.
fn py_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().map(|f| f == 0.0).unwrap_or(false),
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
    }
}

/// `int(r.get("seq", 0) or 0)`. Absent, null, or any falsy value sorts as 0; anything left has
/// already survived the `bad seq` guard, so the `unwrap_or` is belt and braces.
fn sort_seq(value: Option<&Value>) -> i64 {
    match value {
        None => 0,
        Some(v) if py_falsy(v) => 0,
        Some(v) => py_int(v).unwrap_or(0),
    }
}

/// `str(r.get("device", ""))` — including the fact that a *present but null* device sorts under
/// the literal string `"None"` while an absent one sorts under `""`. Two different buckets.
fn py_str(value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(Value::Null) => "None".to_string(),
        Some(Value::Bool(true)) => "True".to_string(),
        Some(Value::Bool(false)) => "False".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(), // deviation 3: Python's repr differs; unreachable
    }
}

/// `json.dumps(record, ensure_ascii=False, sort_keys=True)`.
///
/// Two things here are visible on disk and neither is serde_json's default:
///
/// - **Python's separators are `", "` and `": "`,** not the compact `,`/`:`. Every line of the
///   live vault's journal is written that way; a compact writer would make every future line
///   differ in shape from every past one inside a union-merged file.
/// - **`sort_keys=True` is recursive.** serde_json's `Map` is a `BTreeMap` by default and would
///   sort for free, but that flips to insertion order the day anything in the dependency graph
///   enables its `preserve_order` feature. Sorting explicitly makes the bytes independent of a
///   feature flag chosen by some other crate.
///
/// `ensure_ascii=False` needs no work: serde_json escapes only `"`, `\` and the C0 controls, so
/// U+2028 and friends go out verbatim — which is the whole reason `read` may not use
/// `splitlines()`.
fn dumps(record: &Record) -> String {
    dumps_value(&Value::Object(record.clone()))
}

/// The same writer, for the handful of places outside the journal that emit JSON.
///
/// **Every JSON this crate writes must come through here**, never `serde_json::to_string`: the
/// separators above are the observable difference, and on a `merge=union` ledger a compact line
/// and a Python line carrying identical data become two lines after a two-device sync.
pub fn dumps_value(value: &Value) -> String {
    let mut out: Vec<u8> = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut out, PythonFormatter);
    with_sorted_keys(value)
        .serialize(&mut ser)
        .expect("serializing a Value into a Vec<u8> cannot fail");
    String::from_utf8(out).expect("serde_json emits UTF-8")
}

fn with_sorted_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                sorted.insert(key.clone(), with_sorted_keys(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(with_sorted_keys).collect()),
        other => other.clone(),
    }
}

/// The `json` module's default separators, which are not the JSON-minimal ones.
struct PythonFormatter;

impl serde_json::ser::Formatter for PythonFormatter {
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Stand-in for pytest's `tmp_path`. No temp-dir crate is available and none may be added.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> TempDir {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let path = std::env::temp_dir().join(format!(
                "knowlu-ledger-{}-{}-{}",
                std::process::id(),
                nanos,
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn rec(json: &str) -> Record {
        match serde_json::from_str::<Value>(json).unwrap() {
            Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        }
    }

    fn field<'a>(record: &'a Record, key: &str) -> &'a Value {
        record.get(key).unwrap_or_else(|| panic!("no {key} in {record:?}"))
    }

    fn strs(records: &[Record], key: &str) -> Vec<String> {
        records.iter().map(|r| field(r, key).as_str().unwrap().to_string()).collect()
    }

    fn ints(records: &[Record], key: &str) -> Vec<i64> {
        records.iter().map(|r| field(r, key).as_i64().unwrap()).collect()
    }

    #[test]
    fn append_creates_day_file_and_read_round_trips() {
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("journal"));
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-29T14:03:12.345Z", "device": "desktop", "seq": 1, "field": "progress"}"#,
            ))
            .unwrap();
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-30T01:00:00.000Z", "device": "laptop", "seq": 1, "field": "due"}"#,
            ))
            .unwrap();
        assert!(tmp.join("journal").join("2026-08-29.jsonl").exists());
        assert!(tmp.join("journal").join("2026-08-30.jsonl").exists());
        let records = ledger.read(None, None);
        assert_eq!(strs(&records, "field"), ["progress", "due"]);
    }

    #[test]
    fn read_sorts_union_interleaved_lines_by_ts_device_seq() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-08-29.jsonl"),
            concat!(
                "{\"ts\": \"2026-08-29T10:00:00.000Z\", \"device\": \"laptop\", \"seq\": 2, \"n\": 3}\n",
                "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"device\": \"desktop\", \"seq\": 1, \"n\": 1}\n",
                "{\"ts\": \"2026-08-29T10:00:00.000Z\", \"device\": \"laptop\", \"seq\": 1, \"n\": 2}\n",
            ),
        )
        .unwrap();
        assert_eq!(ints(&JsonlLedger::new(&root).read(None, None), "n"), [1, 2, 3]);
    }

    #[test]
    fn malformed_line_is_skipped_with_warning_not_crash() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-08-29.jsonl"),
            concat!(
                "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"device\": \"d\", \"seq\": 1}\n",
                "{not json\n",
                "[\"not\", \"a\", \"dict\"]\n",
            ),
        )
        .unwrap();
        let mut ledger = JsonlLedger::new(&root);
        assert_eq!(ledger.read(None, None).len(), 1);
        assert_eq!(ledger.warnings.len(), 2);
        assert!(ledger.warnings[0].contains("2026-08-29.jsonl:2"), "{:?}", ledger.warnings);
        assert!(
            ledger.warnings[1].contains("2026-08-29.jsonl:3 not a record"),
            "{:?}",
            ledger.warnings
        );
    }

    #[test]
    fn since_until_filter_by_timestamp() {
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("j"));
        for day in ["2026-08-28", "2026-08-29", "2026-08-30"] {
            ledger
                .append(&rec(&format!(
                    r#"{{"ts": "{day}T12:00:00.000Z", "device": "d", "seq": 1, "day": "{day}"}}"#
                )))
                .unwrap();
        }
        assert_eq!(
            strs(&ledger.read(Some("2026-08-29"), None), "day"),
            ["2026-08-29", "2026-08-30"]
        );
        assert_eq!(
            strs(&ledger.read(Some("2026-08-29T13:00:00.000Z"), None), "day"),
            ["2026-08-30"]
        );
        assert_eq!(
            strs(&ledger.read(None, Some("2026-08-28T23:59:59.999Z")), "day"),
            ["2026-08-28"]
        );
    }

    #[test]
    fn append_without_ts_is_refused() {
        let tmp = TempDir::new();
        assert_eq!(
            JsonlLedger::new(tmp.join("j")).append(&rec(r#"{"device": "d"}"#)),
            Err(LedgerError::MissingTs)
        );
        // The other two halves of the same guard: a non-string ts, and one too short to slice.
        let ledger = JsonlLedger::new(tmp.join("j"));
        assert!(ledger.append(&rec(r#"{"ts": 20260829}"#)).is_err());
        assert!(ledger.append(&rec(r#"{"ts": "2026-08-2"}"#)).is_err());
        assert!(!tmp.join("j").exists(), "a refused append must not create the root");
    }

    #[test]
    fn read_on_missing_root_is_empty() {
        let tmp = TempDir::new();
        assert!(JsonlLedger::new(tmp.join("nope")).read(None, None).is_empty());
    }

    #[test]
    fn bad_seq_is_skipped_with_warning_not_crash() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-08-29.jsonl"),
            concat!(
                "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"device\": \"d\", \"seq\": \"abc\"}\n",
                "{\"ts\": \"2026-08-29T09:00:01.000Z\", \"device\": \"d\", \"seq\": 2}\n",
            ),
        )
        .unwrap();
        let mut ledger = JsonlLedger::new(&root);
        assert_eq!(ints(&ledger.read(None, None), "seq"), [2]);
        assert_eq!(ledger.warnings, ["2026-08-29.jsonl:1 bad seq"]);
    }

    #[test]
    fn bare_date_until_is_inclusive_of_that_day() {
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("j"));
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-28T12:00:00.000Z", "device": "d", "seq": 1, "day": "28"}"#,
            ))
            .unwrap();
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-29T00:00:00.000Z", "device": "d", "seq": 1, "day": "29"}"#,
            ))
            .unwrap();
        assert_eq!(strs(&ledger.read(None, Some("2026-08-28")), "day"), ["28"]);
    }

    #[test]
    fn record_with_unicode_line_separator_in_a_value_round_trips() {
        // Records are written with ensure_ascii=False, so U+2028 (and U+2029, \x0b, \x0c,
        // \x1c-\x1e, \x85) reach the file verbatim inside a value — a title pasted off a
        // course page, a VHL bucket name. splitlines() treats every one of them as a line
        // break, tearing one record into two halves that both fail to parse: the record does
        // not arrive garbled, it disappears (and WARNs twice). Only "\n" ends a record.
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("journal"));
        let torn = "Lesen \u{2028} Schreiben \u{2029} \u{c} Aufgabe";
        let mut first = rec(r#"{"ts": "2026-08-29T09:00:00.000Z", "device": "d", "seq": 1}"#);
        first.insert("new".into(), Value::String(torn.into()));
        ledger.append(&first).unwrap();
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-29T09:00:01.000Z", "device": "d", "seq": 2, "new": "next"}"#,
            ))
            .unwrap();
        let records = ledger.read(None, None);
        assert_eq!(strs(&records, "new"), [torn, "next"]);
        assert!(ledger.warnings.is_empty(), "{:?}", ledger.warnings);
        // And U+2028 really is on disk verbatim — otherwise this test proves nothing. This whole
        // line is what the interpreter emits for the same record: json.dumps(..., ensure_ascii=
        // False, sort_keys=True) gives U+2028/U+2029 raw and escapes the C0 control as \f.
        let raw = fs::read_to_string(tmp.join("journal").join("2026-08-29.jsonl")).unwrap();
        let want = "{\"device\": \"d\", \"new\": \"Lesen \u{2028} Schreiben \u{2029} \\f Aufgabe\", \
                    \"seq\": 1, \"ts\": \"2026-08-29T09:00:00.000Z\"}";
        assert_eq!(raw.lines().next().unwrap(), want);
        assert!(raw.contains('\u{2028}'), "{raw}");
    }

    #[test]
    fn warnings_reflect_the_latest_read_only() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("2026-08-29.jsonl"), "{not json\n").unwrap();
        let mut ledger = JsonlLedger::new(&root);
        ledger.read(None, None);
        ledger.read(None, None);
        assert_eq!(ledger.warnings.len(), 1);
    }

    // ---- traps the Python suite leaves implicit --------------------------------------------

    #[test]
    fn the_day_file_is_the_records_own_ts_not_the_clock() {
        let tmp = TempDir::new();
        let ledger = JsonlLedger::new(tmp.join("journal"));
        ledger
            .append(&rec(r#"{"ts": "2019-01-02T03:04:05.000Z", "device": "d", "seq": 1}"#))
            .unwrap();
        assert!(tmp.join("journal").join("2019-01-02.jsonl").exists());
    }

    #[test]
    fn the_written_line_matches_pythons_json_dumps() {
        // json.dumps(..., ensure_ascii=False, sort_keys=True) — checked against the interpreter,
        // and against every line of the live vault's journal.
        let tmp = TempDir::new();
        let ledger = JsonlLedger::new(tmp.join("journal"));
        ledger
            .append(&rec(
                r#"{"ts": "2026-08-29T09:00:00.000Z", "device": "d", "b": 1, "a": {"z": 1, "y": [2, 3]}}"#,
            ))
            .unwrap();
        let raw = fs::read_to_string(tmp.join("journal").join("2026-08-29.jsonl")).unwrap();
        let want = r#"{"a": {"y": [2, 3], "z": 1}, "b": 1, "device": "d", "ts": "2026-08-29T09:00:00.000Z"}"#;
        assert_eq!(raw, format!("{want}{NEWLINE}"));
    }

    #[test]
    fn append_then_read_round_trips_the_record_byte_identically() {
        // The union-merge hazard: if the writer's bytes differ from what a Python-written line
        // would be, the same logical record can exist twice in one file and never reconcile. So
        // assert the whole loop — the record comes back equal, and re-serialising it reproduces
        // the exact line on disk, terminator included.
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("journal"));
        let record = rec(
            r#"{"ts": "2026-08-29T09:00:00.000Z", "device": "QuinnsPC", "seq": 3, "op": "set",
                "field": "progress", "old": null, "new": 40, "evidence": {"why": "Aufgabe — 3"}}"#,
        );
        ledger.append(&record).unwrap();
        let back = ledger.read(None, None);
        assert_eq!(back, vec![record.clone()]);
        let raw = fs::read_to_string(tmp.join("journal").join("2026-08-29.jsonl")).unwrap();
        assert_eq!(raw, format!("{}{NEWLINE}", dumps(&back[0])));
        assert_eq!(dumps(&back[0]), dumps(&record));
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_append_writes_crlf_because_pythons_text_mode_does() {
        // Not cosmetic: state/journal/** is merge=union, so a bare-LF line written next to
        // Python's CRLF lines is a byte-different copy of the same record that no merge can
        // collapse. Verified against the interpreter: open(..., "a") with newline=None
        // translates "\n" to os.linesep.
        let tmp = TempDir::new();
        let ledger = JsonlLedger::new(tmp.join("journal"));
        ledger.append(&rec(r#"{"ts": "2026-08-29T09:00:00.000Z", "n": 1}"#)).unwrap();
        ledger.append(&rec(r#"{"ts": "2026-08-29T09:00:01.000Z", "n": 2}"#)).unwrap();
        let raw = fs::read(tmp.join("journal").join("2026-08-29.jsonl")).unwrap();
        assert_eq!(raw.iter().filter(|b| **b == b'\r').count(), 2, "two CRs, one per record");
        assert!(raw.ends_with(b"\r\n"));
        // No lone LF anywhere: every \n is preceded by \r.
        assert!(
            raw.windows(2).filter(|w| w[1] == b'\n').all(|w| w[0] == b'\r'),
            "a bare LF slipped into a CRLF file"
        );
    }

    #[test]
    fn crlf_lines_are_normalised_the_way_pythons_text_mode_normalises_them() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        // Written on Windows by Python, read anywhere: the \r must not reach the parser.
        fs::write(
            root.join("2026-08-29.jsonl"),
            "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"n\": 1}\r\n",
        )
        .unwrap();
        let mut ledger = JsonlLedger::new(&root);
        assert_eq!(ints(&ledger.read(None, None), "n"), [1]);
        assert!(ledger.warnings.is_empty(), "{:?}", ledger.warnings);
    }

    #[test]
    fn only_one_trailing_empty_element_is_popped_and_line_numbers_survive_blanks() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        // Line 1 blank, line 2 good, line 3 blank, line 4 torn, then two newlines: one is line
        // 4's terminator, the other leaves a blank element that is skipped, not reported.
        fs::write(
            root.join("2026-08-29.jsonl"),
            "   \n{\"ts\": \"2026-08-29T09:00:00.000Z\", \"n\": 1}\n\n{oops\n\n",
        )
        .unwrap();
        let mut ledger = JsonlLedger::new(&root);
        assert_eq!(ints(&ledger.read(None, None), "n"), [1]);
        assert_eq!(ledger.warnings.len(), 1, "{:?}", ledger.warnings);
        assert!(
            ledger.warnings[0].starts_with("2026-08-29.jsonl:4 malformed"),
            "{:?}",
            ledger.warnings
        );
    }

    #[test]
    fn an_empty_string_bound_means_no_bound() {
        let tmp = TempDir::new();
        let mut ledger = JsonlLedger::new(tmp.join("j"));
        ledger.append(&rec(r#"{"ts": "2026-08-28T12:00:00.000Z", "seq": 1}"#)).unwrap();
        assert_eq!(ledger.read(Some(""), Some("")).len(), 1);
    }

    #[test]
    fn sort_key_uses_pythons_str_and_int_conversions() {
        // A null device is str()'d to "None"; an absent one to "". "" < "None", so the record
        // that names no device at all sorts first. This is not a nicety — it is the tie-break
        // that keeps a start/step/end trio written in one millisecond in order.
        assert_eq!(py_str(None), "");
        assert_eq!(py_str(Some(&Value::Null)), "None");
        assert_eq!(py_str(Some(&Value::Bool(true))), "True");
        assert!(py_str(None) < py_str(Some(&Value::Null)));

        assert_eq!(py_int(&serde_json::json!(2.7)), Some(2), "int() truncates a float");
        assert_eq!(py_int(&serde_json::json!("2.5")), None, "int('2.5') raises");
        assert_eq!(py_int(&serde_json::json!(" 7 ")), Some(7), "int() strips whitespace");
        assert_eq!(py_int(&serde_json::json!([1])), None, "int([1]) raises");
        assert_eq!(sort_seq(None), 0);
        assert_eq!(sort_seq(Some(&Value::Null)), 0, "None or 0");
        assert_eq!(sort_seq(Some(&serde_json::json!(""))), 0, "'' or 0");
    }

    #[test]
    fn a_float_seq_is_valid_and_sorts_truncated() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-08-29.jsonl"),
            concat!(
                "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"device\": \"d\", \"seq\": 2, \"n\": 2}\n",
                "{\"ts\": \"2026-08-29T09:00:00.000Z\", \"device\": \"d\", \"seq\": 1.9, \"n\": 1}\n",
            ),
        )
        .unwrap();
        let mut ledger = JsonlLedger::new(&root);
        assert_eq!(ints(&ledger.read(None, None), "n"), [1, 2]);
        assert!(ledger.warnings.is_empty(), "{:?}", ledger.warnings);
    }

    #[test]
    fn only_this_module_opens_ledger_files() {
        // The seam (spec §2): everything above talks in records so the file store can be
        // swapped. `.jsonl` appears in exactly one Python module and must appear in exactly one
        // Rust one. Comment lines are exempt — this looks for code.
        let mut offenders: Vec<String> = Vec::new();
        for entry in fs::read_dir("src").expect("run from the crate root") {
            let path = entry.unwrap().path();
            if path.extension().map(|e| e != "rs").unwrap_or(true) {
                continue;
            }
            if file_name(&path) == "ledger.rs" {
                continue;
            }
            // A sibling module may be mid-write by another agent; unreadable is not an offence.
            let Ok(text) = fs::read_to_string(&path) else { continue };
            if text.lines().any(|l| {
                let t = l.trim_start();
                !t.starts_with("//") && !t.starts_with('*') && t.contains(".jsonl")
            }) {
                offenders.push(file_name(&path));
            }
        }
        assert!(offenders.is_empty(), "these modules name a ledger file directly: {offenders:?}");
    }

    #[test]
    fn first_day_is_the_earliest_day_file_by_name() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        assert_eq!(JsonlLedger::new(&root).first_day(), None, "no folder is no day");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("2026-09-25.jsonl"), "").unwrap();
        fs::write(root.join("2026-09-24.jsonl"), "").unwrap();
        fs::write(root.join("notes.jsonl"), "").unwrap();
        fs::write(root.join("2026-09-01.txt"), "").unwrap();
        assert_eq!(JsonlLedger::new(&root).first_day(), Some(jiff::civil::date(2026, 9, 24)));
    }

    #[test]
    fn first_ts_is_the_smallest_ts_in_the_earliest_day_file() {
        let tmp = TempDir::new();
        let root = tmp.join("journal");
        assert_eq!(JsonlLedger::new(&root).first_ts(), None);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("2026-09-23.jsonl"),
            "{\"ts\": \"2026-09-23T23:10:00.000Z\"}\n{\"ts\": \"2026-09-23T22:30:00.000Z\"}\nnot json\n",
        )
        .unwrap();
        fs::write(root.join("2026-09-24.jsonl"), "{\"ts\": \"2026-09-24T01:00:00.000Z\"}\n").unwrap();
        assert_eq!(JsonlLedger::new(&root).first_ts().as_deref(), Some("2026-09-23T22:30:00.000Z"));
    }
}
