//! Python string and newline semantics, in one place.
//!
//! Not a port of any Python module — these come free with `str` in Python. They live here because
//! **three modules have now needed them independently** (`ledger`, `eventledger`, `ingest`), and
//! newline handling is the single place in this port where a divergence is both silent and
//! destructive:
//!
//! - Get `splitlines` wrong and a title carrying U+2028 is one line to the writer and two to the
//!   reader, which silently destroys a ledger record (see the preserved-defects report).
//! - Get [`NEWLINE`] wrong and a bare-LF writer changes a file's bytes on first append; on the
//!   `merge=union` ledgers that resurfaces as phantom duplicates after a two-device sync.
//! - Get [`universal_newlines`] wrong and `guard_block_style` stops guarding **entirely**, because
//!   no line ever equals `"---"`.
//!
//! One definition, four rules, all pinned by tests.

/// What Python's text-mode write emits for `\n`: `os.linesep`.
///
/// Every file in this repo is CRLF because Python has always written it that way on Windows.
pub const NEWLINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// Python's universal-newline decoding, which `read_text` applies before anything sees the text.
///
/// Both `\r\n` and a lone `\r` become `\n`. This is why the CRLF the writer produces on Windows is
/// invisible to every parser, and it must be applied on **every** read of a repo file.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The characters Python's `str.splitlines()` treats as line boundaries.
///
/// Wider than `\n`: `\x0b`, `\x0c`, `\x1c`, `\x1d`, `\x1e`, U+0085, U+2028 and U+2029 all split.
/// Notably **`\x1f` does not**, even though `str.isspace()` is true for it — hence the separate
/// [`is_python_space`]. That asymmetry is not a typo in either language; it is why the two
/// predicates exist side by side.
pub fn is_line_boundary(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}'
            | '\u{2029}'
    )
}

/// Python's `str.splitlines()`.
///
/// Rust's `str::lines()` is **not** equivalent — it splits on `\n` and `\r\n` only.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if !is_line_boundary(c) {
            continue;
        }
        out.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        // `\r\n` is one boundary, not two.
        if c == '\r' {
            if let Some(&(j, '\n')) = chars.peek() {
                chars.next();
                end = j + 1;
            }
        }
        start = end;
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Python's `str.isspace()`. Wider than Rust's `char::is_whitespace` by exactly `\x1c`–`\x1f`.
pub fn is_python_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

/// Python's bare `str.strip()`.
pub fn strip(text: &str) -> &str {
    text.trim_matches(|c: char| is_python_space(c))
}

/// Read a repo file the way Python's `read_text(encoding="utf-8")` does: decode, then apply
/// universal-newline translation. **Use this for every read of a note, ledger or config.**
pub fn read_text(path: &std::path::Path) -> std::io::Result<String> {
    Ok(universal_newlines(&std::fs::read_to_string(path)?))
}

/// Write a repo file the way Python's `write_text(encoding="utf-8")` does: translate `\n` to
/// `os.linesep` on the way out. **Use this for every write of a note.**
pub fn write_text(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let out = if NEWLINE == "\n" {
        text.to_string()
    } else {
        // Normalise first so an already-CRLF string does not become CRCRLF.
        universal_newlines(text).replace('\n', NEWLINE)
    };
    std::fs::write(path, out)
}

/// Python's `str(value)` over a JSON scalar.
///
/// A fourth module needed this in wave 6 (`zybooks`, after `eventfeed`), which is what moved it
/// here. Two facts it encodes: Python's `None`/`True`/`False` are not Rust's spellings, and a
/// whole float renders `1.0` where serde_json would give `1` -- while an integer id must still
/// render `52709738382567`, not `52709738382567.0`.
pub fn json_str(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(true) => "True".to_string(),
        serde_json::Value::Bool(false) => "False".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{f:.1}")
                } else {
                    f.to_string()
                }
            }
        }
        other => other.to_string(),
    }
}

/// Python's `str(x)` on a YAML-loaded value.
///
/// **`None` renders as `"None"`, not the empty string.** `process_approvals` reads
/// `str(meta.get("status", "")).strip()`, and a note carrying `status: null` therefore has status
/// `"None"` -- which matches nothing and falls through to `unknown status:`. Rendering it as `""`
/// would silently reclassify those notes.
pub fn yaml_str(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::Null => "None".to_string(),
        serde_yaml_ng::Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

/// `type(x).__name__` for a value that came out of `yaml.safe_load` — the half of an
/// `AttributeError`/`TypeError` message that names what the config actually held.
pub fn yaml_type_name(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "NoneType",
        serde_yaml_ng::Value::Bool(_) => "bool",
        serde_yaml_ng::Value::Number(n) => {
            if n.is_f64() {
                "float"
            } else {
                "int"
            }
        }
        serde_yaml_ng::Value::String(_) => "str",
        serde_yaml_ng::Value::Sequence(_) => "list",
        serde_yaml_ng::Value::Mapping(_) => "dict",
        serde_yaml_ng::Value::Tagged(_) => "str",
    }
}

/// Python's truthiness over a YAML value. Drives every `x or default` in the originals.
pub fn yaml_truthy(value: &serde_yaml_ng::Value) -> bool {
    match value {
        serde_yaml_ng::Value::Null => false,
        serde_yaml_ng::Value::Bool(b) => *b,
        serde_yaml_ng::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        serde_yaml_ng::Value::String(s) => !s.is_empty(),
        serde_yaml_ng::Value::Sequence(s) => !s.is_empty(),
        serde_yaml_ng::Value::Mapping(m) => !m.is_empty(),
        _ => true,
    }
}

/// `int(x)` over a JSON value, as `int(s.get("total_points") or 0)` needs it.
///
/// Shared by `zybooks` (points) and `vhl` (percentage_complete, assignment_count).
///
/// Python's `int` parses a *string* of digits as well as truncating a float, and `int(True)` is
/// `1`. All three shapes are legal JSON; the fixture's `total_points` are floats.
pub fn json_int(value: Option<&serde_json::Value>) -> Result<i64, String> {
    let value = match value {
        Some(v) if json_truthy(v) => v,
        // Absent, null, false, 0, 0.0 or "" — every one of these is `x or 0`.
        _ => return Ok(0),
    };
    match value {
        serde_json::Value::Bool(_) => Ok(1), // only `true` reaches here; `false` is falsy above
        serde_json::Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f.trunc() as i64))
            .ok_or_else(|| format!("cannot convert {n} to int")),
        serde_json::Value::String(s) => s
            .trim()
            .replace('_', "")
            .parse::<i64>()
            .map_err(|_| format!("invalid literal for int() with base 10: '{s}'")),
        other => Err(format!("int() argument must be a number, not {other}")),
    }
}

/// Python's truthiness over a JSON value -- the same rule, the other value type.
pub fn json_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universal_newlines_folds_both_forms() {
        assert_eq!(universal_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
    }

    #[test]
    fn splitlines_breaks_on_the_wide_boundary_set() {
        // The U+2028 case is the one that silently destroys an event ledger record.
        assert_eq!(splitlines("a\u{2028}b"), vec!["a", "b"]);
        assert_eq!(splitlines("a\u{85}b"), vec!["a", "b"]);
        assert_eq!(splitlines("a\u{b}b"), vec!["a", "b"]);
        // ...but \x1f is NOT a boundary, even though it IS python-space.
        assert_eq!(splitlines("a\u{1f}b"), vec!["a\u{1f}b"]);
        assert!(is_python_space('\u{1f}'));
    }

    #[test]
    fn splitlines_treats_crlf_as_one_boundary() {
        assert_eq!(splitlines("a\r\nb"), vec!["a", "b"]);
    }

    #[test]
    fn rust_lines_is_not_python_splitlines() {
        // Pinning the difference that motivated this module.
        let text = "a\u{2028}b";
        assert_eq!(text.lines().count(), 1);
        assert_eq!(splitlines(text).len(), 2);
    }

    #[test]
    fn strip_removes_the_separators_rust_does_not_call_whitespace() {
        assert_eq!(strip("\u{1c}\u{1f} x \u{1e}"), "x");
    }

    #[test]
    fn write_then_read_round_trips_through_the_platform_newline() {
        let dir = std::env::temp_dir().join(format!("qo-pystr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.md");
        write_text(&path, "---\ntitle: x\n---\n\nbody\n").unwrap();

        let raw = std::fs::read(&path).unwrap();
        if cfg!(windows) {
            assert!(raw.windows(2).any(|w| w == b"\r\n"), "must write CRLF on Windows");
        }
        assert_eq!(read_text(&path).unwrap(), "---\ntitle: x\n---\n\nbody\n");
    }

    #[test]
    fn writing_an_already_crlf_string_does_not_double_the_cr() {
        let dir = std::env::temp_dir().join(format!("qo-pystr2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.md");
        write_text(&path, "a\r\nb\r\n").unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(3).any(|w| w == b"\r\r\n"), "CRCRLF is corruption");
        assert_eq!(read_text(&path).unwrap(), "a\nb\n");
    }
}
