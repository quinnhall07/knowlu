//! Port of `engine/provenance.py` — judgment provenance (spec §2.2). Records who judged what
//! from which inputs; never judges.
//!
//! # The `judgment:` literal is produced, not spliced from text
//!
//! `judgment_literal` genuinely has to *emit* YAML: `judgment:` is a structured value that
//! `update_frontmatter_fields` then splices in as one line. The Python is one call —
//! `yaml.safe_dump(v, default_flow_style=True, width=10**6, allow_unicode=True, sort_keys=True)`
//! — and every byte of its output is load-bearing, because these literals are already sitting in
//! the live vault and a re-render that quotes differently would rewrite notes for no reason.
//!
//! Through wave 6 the PyYAML transcription that produces it lived here, and this module called
//! itself "the crate's one deliberate YAML emitter". Wave 7 lifted it into [`crate::yamlemit`]
//! so that `info` and `issues` could share the scalar layer and add block style over it. Every
//! expectation in the tests at the bottom is still a string PyYAML actually produced on this
//! machine, and that is what guards the lift: the flow output must not have moved by a byte.
//! What stays here is the rule the literal enforces — it is a **flow mapping on exactly one
//! line**, and `judgment_literal` raises if it ever fails to be.

use std::collections::BTreeSet;

use serde_yaml_ng::{Mapping, Value};

use crate::yamlemit::safe_dump_flow;

/// Python: `ValueError`, raised by two functions with two different messages. `Display` returns
/// the Python message verbatim — `guard_block_style`'s in particular explains itself to whoever
/// hits it, and that explanation is the point of the guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    /// `judgment_literal` produced something with a newline in it. Never mint a value the write
    /// path's line surgery cannot later replace.
    NotSingleLine,
    /// `guard_block_style` found a `judgment:` that is not a single-line flow mapping.
    BlockStyle,
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvenanceError::NotSingleLine => write!(f, "judgment literal is not a single line"),
            ProvenanceError::BlockStyle => write!(
                f,
                "judgment block is not a single-line flow mapping; \
                 engine.write only writes single-line flow mappings"
            ),
        }
    }
}

/// Python: `JUDGED_FIELDS["task"]`.
pub const JUDGED_FIELDS_TASK: [&str; 6] = [
    "effort_hours",
    "effort_confidence",
    "importance",
    "importance_reason",
    "course",
    "domain",
];

/// Python: `JUDGED_FIELDS["appr"]`.
pub const JUDGED_FIELDS_APPR: [&str; 4] = ["kind", "tier", "verdict", "strength"];

/// The fields judge-once protects for a note kind.
///
/// **An unknown kind is an empty set, not an error** — Python is `JUDGED_FIELDS.get(kind,
/// frozenset())`. `write` calls this for every kind it can resolve, including `course`, `info`
/// and `issue`, and relies on the empty set to mean "nothing here is a judgment, write freely".
pub fn judged_fields_for(kind: &str) -> BTreeSet<&'static str> {
    match kind {
        "task" => JUDGED_FIELDS_TASK.iter().copied().collect(),
        "appr" => JUDGED_FIELDS_APPR.iter().copied().collect(),
        _ => BTreeSet::new(),
    }
}

/// Python: `str(actor).startswith("agent:")` — a plain prefix test, nothing parsed.
///
/// `quinn`, `system:migration` and `system:idfix` are all non-agents, so none of them trip
/// judge-once or get a provenance block. The `str()` coercion is a no-op for every live caller
/// (`WriteContext.actor` is typed `str`), so this takes `&str`.
pub fn is_agent(actor: &str) -> bool {
    actor.starts_with("agent:")
}

/// Python's `str.split()` whitespace set, which is **not** Rust's `char::is_whitespace`.
///
/// `Py_UNICODE_ISSPACE` additionally covers U+001C–U+001F (the file/group/record/unit
/// separators). Everything else — including U+0085 and U+00A0, both of which Python splits on —
/// is already in Rust's White_Space property. Verified against CPython for
/// `"a\x1cb \x85 c\xa0d\te"` → `"a b c d e"`.
fn is_python_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `" ".join(value.split())`.
///
/// Split on *runs* of any whitespace and rejoin with single spaces — not a newline replace. It
/// also drops leading and trailing whitespace, and turns a whitespace-only string into `""`.
fn collapse_whitespace(s: &str) -> String {
    s.split(is_python_space)
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Python's `str.strip()` — the same whitespace set as `split()`.
fn python_strip(s: &str) -> String {
    s.trim_matches(is_python_space).to_string()
}

/// Python: `_flatten` — recursively collapse whitespace (including newlines) in strings.
///
/// Two traps:
///
/// 1. It recurses through **every** dict and list at **every** depth, so a newline buried three
///    levels down is still removed. That is what makes the one-line guarantee hold.
/// 2. It flattens **values only — never keys** (`{k: _flatten(v) for k, v in value.items()}`).
///    A key with a newline in it therefore survives into the emitter, where it forces PyYAML's
///    explicit-key form and a real line break, and `judgment_literal` raises. Preserved: see
///    `a_multiline_key_still_raises`.
pub(crate) fn flatten(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(collapse_whitespace(s)),
        Value::Mapping(m) => {
            Value::Mapping(m.iter().map(|(k, v)| (k.clone(), flatten(v))).collect())
        }
        Value::Sequence(items) => Value::Sequence(items.iter().map(flatten).collect()),
        other => other.clone(),
    }
}

/// Build the `judgment:` block: who judged, when, from what, over which fields.
///
/// `run_id` is genuinely nullable (a local run has none). `inputs` is Python's `inputs or {}`,
/// so `None` and an empty mapping are the same thing. Key insertion order matches Python's dict
/// literal — it survives into the journal record, though `judgment_literal` sorts before it
/// emits.
pub fn judgment_value<S: AsRef<str>>(
    run_id: Option<&str>,
    actor: &str,
    at: &str,
    inputs: Option<&Mapping>,
    fields: &[S],
) -> Mapping {
    let mut sorted: Vec<String> = fields.iter().map(|f| f.as_ref().to_string()).collect();
    sorted.sort(); // Python `sorted()` on str is code-point order; Rust's byte order agrees.

    let mut out = Mapping::new();
    out.insert(
        "run_id".into(),
        run_id.map_or(Value::Null, |r| Value::String(r.to_string())),
    );
    out.insert("actor".into(), Value::String(actor.to_string()));
    out.insert("at".into(), Value::String(at.to_string()));
    out.insert(
        "inputs".into(),
        flatten(&Value::Mapping(inputs.cloned().unwrap_or_default())),
    );
    out.insert(
        "fields".into(),
        Value::Sequence(sorted.into_iter().map(Value::String).collect()),
    );
    out
}

/// Render a judgment mapping as a one-line flow mapping.
///
/// Python defensively flattens the whole mapping first (so this is safe on a value read back
/// from the journal, which `passes.heal` does), dumps it flow-style with sorted keys, strips,
/// and **raises if a newline survived**. That last check is not paranoia:
/// `update_frontmatter_fields` replaces one line, so a two-line value would corrupt the note.
///
/// Callers holding a [`Mapping`] pass `&Value::Mapping(m)`; `passes.heal` passes an arbitrary
/// journal value, which is why this takes a `Value`.
pub fn judgment_literal(value: &Value) -> Result<String, ProvenanceError> {
    let flattened = flatten(value);
    let literal = python_strip(&safe_dump_flow(&flattened));
    if literal.contains('\n') {
        return Err(ProvenanceError::NotSingleLine);
    }
    Ok(literal)
}

/// Refuse a note whose `judgment:` is not a single-line flow mapping.
///
/// Four traps, all deliberate:
///
/// 1. **Only the frontmatter is scanned** — between a first line starting with `---` and the
///    first later line that is *exactly* `---`. Body prose beginning `judgment:` must not trip
///    the guard.
/// 2. **No frontmatter, or no closing `---`, returns Ok.** Nothing to guard.
/// 3. **`judgment: null` and `judgment: ~` are allowed.** An explicit null scalar is
///    unambiguously *absent*, so there is no block to guard.
/// 4. **A bare `judgment:` with nothing after the colon is NOT allowed**, and that distinction
///    is the whole point of the function: `judgment:\n  migrated: true` is exactly how a
///    block-style continuation starts, so it must fall through to the error.
pub fn guard_block_style(text: &str) -> Result<(), ProvenanceError> {
    let lines: Vec<&str> = text.split('\n').collect();
    // Python's `if not lines` can never fire ("".split("\n") == [""]); kept for shape.
    if lines.is_empty() || !lines[0].starts_with("---") {
        return Ok(());
    }

    let mut frontmatter_end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            frontmatter_end = Some(i);
            break;
        }
    }
    let Some(end) = frontmatter_end else {
        return Ok(()); // No closing ---, nothing to guard.
    };

    for line in &lines[1..end] {
        if line.starts_with("judgment:") {
            // `line[9:]` — the slice after "judgment:", which is 9 ASCII bytes, so a byte
            // slice here is the same slice Python takes by character.
            let after_colon = python_strip(&line[9..]);
            if after_colon == "null" || after_colon == "~" {
                continue;
            }
            // `not after_colon` is redundant with the `startswith("{")` test that follows it in
            // Python — an empty string starts with nothing — but it is preserved as written.
            if after_colon.is_empty()
                || !after_colon.starts_with('{')
                || !after_colon.ends_with('}')
            {
                return Err(ProvenanceError::BlockStyle);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }

    /// `{k: <value>}` — the shape every emitter expectation below is measured in.
    fn one(value: Value) -> Value {
        let mut m = Mapping::new();
        m.insert(s("k"), value);
        Value::Mapping(m)
    }

    fn literal(value: &Value) -> String {
        judgment_literal(value).expect("single line")
    }

    #[test]
    fn test_judged_fields_match_spec() {
        assert_eq!(
            judged_fields_for("task"),
            BTreeSet::from([
                "effort_hours",
                "effort_confidence",
                "importance",
                "importance_reason",
                "course",
                "domain"
            ])
        );
        assert_eq!(judged_fields_for("course"), BTreeSet::new());
    }

    #[test]
    fn test_judged_fields_appr_match_spec() {
        assert_eq!(
            judged_fields_for("appr"),
            BTreeSet::from(["kind", "tier", "verdict", "strength"])
        );
    }

    #[test]
    fn test_is_agent() {
        assert!(is_agent("agent:routine.gmail"));
        assert!(!is_agent("quinn"));
        assert!(!is_agent("system:idfix"));
    }

    #[test]
    fn test_judgment_literal_is_one_line_and_round_trips() {
        let mut inputs = Mapping::new();
        inputs.insert(s("source_uid"), s("gmail:1a03"));
        inputs.insert(s("title_seen"), s("AEMX: intro slide"));
        let value = judgment_value(
            Some("cloud-2026-08-28T13:00:29Z"),
            "agent:routine.gmail",
            "2026-08-28T13:04:11.000Z",
            Some(&inputs),
            &["importance", "effort_hours"],
        );
        let lit = literal(&Value::Mapping(value.clone()));
        assert!(!lit.contains('\n'));
        // Byte-for-byte what PyYAML emits for this input.
        assert_eq!(
            lit,
            "{actor: 'agent:routine.gmail', at: '2026-08-28T13:04:11.000Z', \
             fields: [effort_hours, importance], \
             inputs: {source_uid: 'gmail:1a03', title_seen: 'AEMX: intro slide'}, \
             run_id: 'cloud-2026-08-28T13:00:29Z'}"
        );

        // `yaml.safe_load(f"judgment: {literal}")["judgment"] == value`
        let parsed: Value = serde_yaml_ng::from_str(&format!("judgment: {lit}")).unwrap();
        assert_eq!(parsed["judgment"], Value::Mapping(value.clone()));
        assert_eq!(
            value["fields"],
            Value::Sequence(vec![s("effort_hours"), s("importance")])
        );
    }

    #[test]
    fn test_judgment_literal_flattens_newlines_and_never_spans_lines() {
        let mut nested = Mapping::new();
        nested.insert(s("k"), s("a\nb"));
        let mut inputs = Mapping::new();
        inputs.insert(s("title_seen"), s("Line1\nLine2\r\n\tLine3"));
        inputs.insert(s("nested"), Value::Mapping(nested));

        let value = judgment_value(
            Some("run-1"),
            "agent:routine.gmail",
            "2026-08-28T13:04:11.000Z",
            Some(&inputs),
            &["importance"],
        );

        let mut expected_nested = Mapping::new();
        expected_nested.insert(s("k"), s("a b"));
        let mut expected = Mapping::new();
        expected.insert(s("title_seen"), s("Line1 Line2 Line3"));
        expected.insert(s("nested"), Value::Mapping(expected_nested));
        assert_eq!(value["inputs"], Value::Mapping(expected));

        let lit = literal(&Value::Mapping(value.clone()));
        assert!(!lit.contains('\n'));
        let parsed: Value = serde_yaml_ng::from_str(&format!("judgment: {lit}")).unwrap();
        assert_eq!(parsed["judgment"], Value::Mapping(value));
    }

    #[test]
    fn test_guard_block_style_rejects_multiline_judgment() {
        assert!(guard_block_style("---\ntitle: A\njudgment: {migrated: true}\n---\n").is_ok());
        assert_eq!(
            guard_block_style("---\ntitle: A\njudgment:\n  migrated: true\n---\n"),
            Err(ProvenanceError::BlockStyle)
        );
    }

    #[test]
    fn test_guard_block_style_rejects_folded_or_block_values_and_ignores_body() {
        assert_eq!(
            guard_block_style("---\ntitle: A\njudgment: {actor: 'x', at: 'Line1\n\n      Line2'}\n---\n"),
            Err(ProvenanceError::BlockStyle)
        );
        // A bare `judgment:` is NOT an explicit null — it is how a block continuation starts.
        assert_eq!(
            guard_block_style("---\ntitle: A\njudgment:\n---\n"),
            Err(ProvenanceError::BlockStyle)
        );
        // Body prose that happens to start with 'judgment:' is not frontmatter.
        assert!(guard_block_style(
            "---\ntitle: A\njudgment: {migrated: true}\n---\n\njudgment: I think this matters\n  because of context\n"
        )
        .is_ok());
        // No frontmatter at all: nothing to guard.
        assert!(guard_block_style("plain text\njudgment:\n  nested: true\n").is_ok());
    }

    #[test]
    fn an_explicit_null_judgment_is_allowed_but_a_bare_key_is_not() {
        for allowed in ["judgment: null", "judgment: ~", "judgment:   null  "] {
            assert!(
                guard_block_style(&format!("---\n{allowed}\n---\n")).is_ok(),
                "{allowed}"
            );
        }
        for rejected in ["judgment:", "judgment: ", "judgment: 'x'", "judgment: {a: 1} # c"] {
            assert_eq!(
                guard_block_style(&format!("---\n{rejected}\n---\n")),
                Err(ProvenanceError::BlockStyle),
                "{rejected}"
            );
        }
    }

    #[test]
    fn guard_ignores_a_frontmatter_block_that_never_closes() {
        assert!(guard_block_style("---\njudgment:\n  migrated: true\n").is_ok());
        // The opening line only has to *start* with ---; the closing line must be exactly ---.
        assert!(guard_block_style("--- x\njudgment:\n--- y\n").is_ok());
    }

    #[test]
    fn the_live_vault_literal_reproduces_byte_for_byte() {
        // From a real note, written by the cloud routine on 2026-08-31.
        let mut inputs = Mapping::new();
        inputs.insert(
            s("source_uid"),
            s("_blackboard.platform.gradebook2.GradableItem-_4750386_1"),
        );
        inputs.insert(s("title_seen"), s("Homework #1"));
        let value = judgment_value(
            Some("cloud-2026-08-31T20:01:29Z"),
            "agent:routine.enrich",
            "2026-08-31T20:02:00.603Z",
            Some(&inputs),
            &["course", "effort_hours", "importance_reason"],
        );
        assert_eq!(
            literal(&Value::Mapping(value)),
            "{actor: 'agent:routine.enrich', at: '2026-08-31T20:02:00.603Z', \
             fields: [course, effort_hours, importance_reason], \
             inputs: {source_uid: _blackboard.platform.gradebook2.GradableItem-_4750386_1, \
             title_seen: 'Homework #1'}, run_id: 'cloud-2026-08-31T20:01:29Z'}"
        );
    }

    #[test]
    fn migrated_true_is_the_shape_most_of_the_vault_carries() {
        let mut m = Mapping::new();
        m.insert(s("migrated"), Value::Bool(true));
        assert_eq!(literal(&Value::Mapping(m)), "{migrated: true}");
    }

    /// Every expectation here is a string PyYAML produced for `{"k": <the input>}`.
    #[test]
    fn scalar_quoting_matches_pyyaml() {
        let cases: &[(&str, &str)] = &[
            // Plain: nothing forces a quote and the text resolves back to a string.
            ("_blackboard.platform.gradebook2.GradableItem-_4750386_1", "{k: _blackboard.platform.gradebook2.GradableItem-_4750386_1}"),
            ("hello world", "{k: hello world}"),
            ("a#b", "{k: a#b}"),          // '#' only counts after whitespace
            ("-lead", "{k: -lead}"),      // '-' only counts before whitespace
            ("-x-", "{k: -x-}"),
            ("say \"hi\"", "{k: say \"hi\"}"), // quote chars only count at index 0
            ("it's", "{k: it's}"),
            ("?q", "{k: '?q'}"),          // '?' at index 0 always blocks flow-plain
            ("None", "{k: None}"),        // Python's None is not a YAML null
            ("y", "{k: y}"),              // y/n are not YAML 1.1 booleans
            ("n", "{k: n}"),
            ("1e5", "{k: 1e5}"),          // float needs a '.' …
            ("1.0e5", "{k: 1.0e5}"),      // … and a *signed* exponent
            ("café — dash", "{k: café — dash}"),
            ("😀", "{k: 😀}"),
            // Quoted because a colon is a flow indicator anywhere in the scalar.
            ("agent:routine.gmail", "{k: 'agent:routine.gmail'}"),
            ("a:b", "{k: 'a:b'}"),
            ("end:", "{k: 'end:'}"),
            ("http://x.y/z?a=b", "{k: 'http://x.y/z?a=b'}"),
            // Quoted because the plain form would resolve to something other than a string.
            ("true", "{k: 'true'}"),
            ("OFF", "{k: 'OFF'}"),
            ("123", "{k: '123'}"),
            ("0", "{k: '0'}"),
            ("00", "{k: '00'}"),
            ("017", "{k: '017'}"),
            ("0x1f", "{k: '0x1f'}"),
            ("1_000", "{k: '1_000'}"),
            (".5", "{k: '.5'}"),
            ("1.", "{k: '1.'}"),
            (".__", "{k: .__}"), // PyYAML 6's float pattern needs a digit after the dot
            (".0_", "{k: '.0_'}"),
            (".inf", "{k: '.inf'}"),
            ("null", "{k: 'null'}"),
            ("Null", "{k: 'Null'}"),
            ("~", "{k: '~'}"),
            ("=", "{k: '='}"),
            ("<<", "{k: '<<'}"),
            ("2026-08-31", "{k: '2026-08-31'}"),
            ("2026-08-31T20:02:00.603Z", "{k: '2026-08-31T20:02:00.603Z'}"),
            ("", "{k: ''}"),
            // Quoted because of a leading indicator character.
            ("Homework #1", "{k: 'Homework #1'}"),
            ("- lead", "{k: '- lead'}"),
            ("-", "{k: '-'}"),
            ("@home", "{k: '@home'}"),
            ("*star", "{k: '*star'}"),
            ("&amp", "{k: '&amp'}"),
            ("!bang", "{k: '!bang'}"),
            ("%pct", "{k: '%pct'}"),
            ("`tick", "{k: '`tick'}"),
            ("|pipe", "{k: '|pipe'}"),
            (">gt", "{k: '>gt'}"),
            ("{x}", "{k: '{x}'}"),
            ("[x]", "{k: '[x]'}"),
            ("a,b", "{k: 'a,b'}"),
            ("--- x", "{k: '--- x'}"),
            ("... x", "{k: '... x'}"),
            // The apostrophe doubles inside single quotes.
            ("'", "{k: ''''}"),
        ];
        for (input, expected) in cases {
            assert_eq!(&literal(&one(s(input))), expected, "input {input:?}");
        }
    }

    /// Control characters reach the emitter only through a mapping *key*, since `flatten`
    /// strips them from every value. The escape table itself is tested in `yamlemit`; this pins
    /// that the double-quoted style is chosen for them here, through the literal.
    #[test]
    fn control_characters_in_a_key_force_double_quotes() {
        let mut m = Mapping::new();
        m.insert(s("a\tb"), Value::Number(1.into()));
        assert_eq!(literal(&Value::Mapping(m)), "{\"a\\tb\": 1}");
    }

    #[test]
    fn non_string_scalars_and_empty_collections() {
        let cases: &[(Value, &str)] = &[
            (Value::Null, "{k: null}"),
            (Value::Bool(false), "{k: false}"),
            (Value::Number(1.into()), "{k: 1}"),
            (Value::Number((-1).into()), "{k: -1}"),
            (Value::Number(2.5.into()), "{k: 2.5}"),
            (Value::Number(1.0.into()), "{k: 1.0}"),
            (Value::Number(f64::INFINITY.into()), "{k: .inf}"),
            (Value::Number(f64::NEG_INFINITY.into()), "{k: -.inf}"),
            (Value::Number(f64::NAN.into()), "{k: .nan}"),
            (Value::Sequence(vec![]), "{k: []}"),
            (Value::Mapping(Mapping::new()), "{k: {}}"),
        ];
        for (input, expected) in cases {
            assert_eq!(&literal(&one(input.clone())), expected, "input {input:?}");
        }
        assert_eq!(literal(&Value::Mapping(Mapping::new())), "{}");
    }

    #[test]
    fn nesting_and_key_sorting_match_pyyaml() {
        let doc: Value = serde_yaml_ng::from_str("a: 1\nb:\n  c:\n  - 1\n  - d: null\n").unwrap();
        assert_eq!(literal(&doc), "{a: 1, b: {c: [1, {d: null}]}}");

        // Sorted by code point: '0' < 'M' < '_' < 'a' < 'z'. The int-looking key gets quoted.
        let doc: Value = serde_yaml_ng::from_str("z: 1\na: 2\nM: 3\n_: 4\n'0': 5\n").unwrap();
        assert_eq!(literal(&doc), "{'0': 5, M: 3, _: 4, a: 2, z: 1}");
    }

    #[test]
    fn an_unsortable_mixed_key_mapping_keeps_insertion_order() {
        // Python's `sorted()` raises TypeError comparing 1 to "b"; PyYAML swallows it.
        let mut m = Mapping::new();
        m.insert(Value::Number(1.into()), s("a"));
        m.insert(s("b"), Value::Number(2.into()));
        assert_eq!(literal(&Value::Mapping(m)), "{1: a, b: 2}");
    }

    #[test]
    fn a_long_or_empty_key_takes_pyyaml_s_explicit_key_form() {
        // check_simple_key budgets len(scalar) + len("!!str"); 123 characters is the cliff.
        let mut m = Mapping::new();
        m.insert(s(""), Value::Number(1.into()));
        assert_eq!(literal(&Value::Mapping(m)), "{? '' : 1}");

        for (n, explicit) in [(122, false), (123, true)] {
            let mut m = Mapping::new();
            m.insert(s(&"k".repeat(n)), Value::Number(1.into()));
            assert_eq!(
                literal(&Value::Mapping(m)).starts_with("{? "),
                explicit,
                "key length {n}"
            );
        }
    }

    #[test]
    fn a_multiline_key_still_raises() {
        // `flatten` never touches keys, so this newline survives to the emitter, which folds it
        // onto a second line — and the guard catches it. PyYAML: "{? 'a\n\n    b' : 1}".
        let mut m = Mapping::new();
        m.insert(s("a\nb"), Value::Number(1.into()));
        assert_eq!(
            judgment_literal(&Value::Mapping(m)),
            Err(ProvenanceError::NotSingleLine)
        );
    }

    #[test]
    fn a_plain_scalar_at_the_document_root_is_open_ended_and_raises() {
        // PyYAML appends "...", so the stripped literal spans two lines. A *quoted* root scalar
        // does not, and comes back fine. Unreachable while every caller passes a mapping.
        for open_ended in [s("hello"), Value::Null, Value::Number(5.into())] {
            assert_eq!(
                judgment_literal(&open_ended),
                Err(ProvenanceError::NotSingleLine),
                "{open_ended:?}"
            );
        }
        assert_eq!(literal(&s("true")), "'true'");
        assert_eq!(
            literal(&Value::Sequence(vec![
                Value::Number(1.into()),
                Value::Number(2.into())
            ])),
            "[1, 2]"
        );
    }

    #[test]
    fn flatten_uses_pythons_whitespace_set_at_every_depth() {
        // \x1c is whitespace to Python but not to Rust's char::is_whitespace; \x85 and \xa0 are
        // whitespace to both.
        assert_eq!(
            flatten(&s("a\u{1c}b \u{85} c\u{a0}d\te")),
            s("a b c d e")
        );
        let doc: Value =
            serde_yaml_ng::from_str("- \"  x  \"\n- k: \"  v   v \"\n").unwrap();
        let expected: Value = serde_yaml_ng::from_str("- x\n- k: v v\n").unwrap();
        assert_eq!(flatten(&doc), expected);
        // Keys are left alone.
        let mut m = Mapping::new();
        m.insert(s("k "), s(" v v "));
        let mut want = Mapping::new();
        want.insert(s("k "), s("v v"));
        assert_eq!(flatten(&Value::Mapping(m)), Value::Mapping(want));
    }

    #[test]
    fn judgment_value_shape() {
        let value = judgment_value(None, "agent:x", "2026-08-28", None, &[] as &[&str]);
        assert_eq!(value["run_id"], Value::Null);
        assert_eq!(value["inputs"], Value::Mapping(Mapping::new()));
        assert_eq!(value["fields"], Value::Sequence(vec![]));
        assert_eq!(
            literal(&Value::Mapping(value)),
            "{actor: 'agent:x', at: '2026-08-28', fields: [], inputs: {}, run_id: null}"
        );
        // Insertion order is Python's dict-literal order, which the journal record preserves.
        let ordered = judgment_value(Some("r"), "a", "t", None, &["b", "a"]);
        assert_eq!(
            ordered.keys().map(|k| k.as_str().unwrap()).collect::<Vec<_>>(),
            vec!["run_id", "actor", "at", "inputs", "fields"]
        );
        assert_eq!(ordered["fields"], Value::Sequence(vec![s("a"), s("b")]));
    }
}
