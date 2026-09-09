//! Shared YAML value accessors.
//!
//! Not a port of any Python module — Python gets these from `dict` for free. They exist because
//! every ported module reads frontmatter or config, and without one shared vocabulary each module
//! grows its own subtly different version of "what does a missing key mean, and is null the same
//! as absent?". Those two questions have observable answers in this codebase, so they get answered
//! in exactly one place.
//!
//! # The rule these encode
//!
//! Python's `d.get(k, default)` returns `None` when the key is **present but null**, not the
//! default. `present()` preserves that distinction; `get()` collapses it. Use `present()` wherever
//! Python would go on to call `float()`/`int()` on the result, because there the difference is the
//! difference between a default and a skipped note.

use serde_yaml_ng::{Mapping, Value};

/// Look up a key. Returns the value even when it is null — use [`present`] to distinguish.
pub fn get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

/// `None` = absent. `Some(None)` = present but null. `Some(Some(v))` = present with a value.
pub fn present(map: &Mapping, key: &str) -> Option<Option<Value>> {
    get(map, key).map(|v| match v {
        Value::Null => None,
        other => Some(other.clone()),
    })
}

/// Stringify a scalar the way Python's implicit `str()` conversions would.
pub fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Mirrors Python `float(x)`: a number, or a string that parses as one.
pub fn f64_of(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Mirrors Python `int(x)`: refuses a float-shaped *string* ("2.5"), truncates a real float.
pub fn i64_of(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f.trunc() as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn opt_text(v: Option<&Value>) -> Option<String> {
    v.and_then(text)
}

pub fn opt_f64(v: Option<&Value>, default: f64) -> f64 {
    v.and_then(f64_of).unwrap_or(default)
}

pub fn opt_i64(v: Option<&Value>, default: i64) -> i64 {
    v.and_then(i64_of).unwrap_or(default)
}

/// `journal.jsonable` for a YAML value: the JSON a Python dict of frontmatter would serialise to.
///
/// Python reaches this through PyYAML, which resolves a YAML **1.1** timestamp into a `date` or
/// `datetime` object that `jsonable` then formats. serde_yaml_ng follows YAML **1.2**, whose core
/// schema has no timestamp tag, so those scalars arrive as strings.
///
/// **That gap is closed for free by the vault's own spelling, and only by it.** PyYAML's timestamp
/// pattern requires seconds, so `due: 2026-08-26T23:59` is a *string* in Python too, and a bare
/// `due: 2026-09-01` becomes a `date` whose `isoformat()` is the same text it was written as.
/// Verified against `tests/fixtures/vault-s1-migrated/state/journal/`, whose Python-written
/// records hold `"2026-08-26T23:59"` and `"2026-09-01"` — no seconds, no offsets. Those are the
/// only two shapes in the live vault as well.
///
/// So the two engines agree on every value that exists. A note carrying seconds or a UTC offset
/// would diverge: Python would journal `"2026-10-09T13:00:00"` where this writes the text
/// verbatim. Logged as a cross-cutting port hazard rather than fixed, because `detect_external`
/// compares this output against Python-written journal records and a *unilateral* normalisation
/// here would invent an external edit on every note it touched.
///
/// Mapping keys are stringified as Python's `str(k)` would, which matters only for a non-string
/// key — a shape no note in the vault has, and one PyYAML would need explicit quoting to produce.
pub fn to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => serde_json::to_value(n).unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Sequence(items) => {
            serde_json::Value::Array(items.iter().map(to_json).collect())
        }
        Value::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                out.insert(text(key).unwrap_or_else(|| format!("{key:?}")), to_json(val));
            }
            serde_json::Value::Object(out)
        }
        Value::Tagged(tagged) => to_json(&tagged.value),
    }
}

/// The inverse of [`to_json`], for the one place a journalled value has to become frontmatter
/// again: `passes::verify_tail` re-applies `rec["new"]`, which arrives as JSON.
///
/// JSON is a subset of YAML, so this is total and lossless in that direction. It is deliberately
/// NOT symmetric with `to_json`: a timestamp that PyYAML would have resolved stays a string,
/// which is what `write::to_literal`'s ISO branch then writes back unquoted.
pub fn from_json(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            serde_yaml_ng::from_str::<Value>(&n.to_string()).unwrap_or(Value::Null)
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => {
            Value::Sequence(items.iter().map(from_json).collect())
        }
        serde_json::Value::Object(map) => {
            let mut out = Mapping::new();
            for (key, val) in map {
                out.insert(Value::String(key.clone()), from_json(val));
            }
            Value::Mapping(out)
        }
    }
}

/// Parse a document leniently into a mapping.
///
/// Anything that is not a mapping — including an empty document, which PyYAML loads as `None` —
/// becomes an empty mapping, matching `yaml.safe_load(...) or {}` at every config call site.
pub fn mapping_of(document: &str) -> Mapping {
    match serde_yaml_ng::from_str::<Value>(document) {
        Ok(Value::Mapping(m)) => m,
        _ => Mapping::new(),
    }
}

/// Read a YAML file into a mapping. A missing or unreadable file is an empty mapping.
pub fn mapping_from_file(path: &std::path::Path) -> Mapping {
    match std::fs::read_to_string(path) {
        Ok(text) => mapping_of(&text),
        Err(_) => Mapping::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(src: &str) -> Mapping {
        mapping_of(src)
    }

    #[test]
    fn present_distinguishes_absent_from_null() {
        let map = m("a: 1\nb: null\n");
        assert!(matches!(present(&map, "a"), Some(Some(_))));
        assert_eq!(present(&map, "b"), Some(None));
        assert_eq!(present(&map, "missing"), None);
    }

    #[test]
    fn numeric_coercion_matches_python() {
        assert_eq!(f64_of(&Value::String("2.5".into())), Some(2.5));
        assert_eq!(i64_of(&Value::String("2.5".into())), None, "python int('2.5') raises");
        assert_eq!(i64_of(&serde_yaml_ng::from_str::<Value>("2.9").unwrap()), Some(2));
    }

    #[test]
    fn a_non_mapping_document_is_an_empty_mapping() {
        assert!(mapping_of("").is_empty());
        assert!(mapping_of("just a scalar").is_empty());
        assert!(mapping_of("- a\n- b\n").is_empty());
    }
}
