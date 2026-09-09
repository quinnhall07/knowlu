//! The crate's YAML emitter — a transcription of PyYAML's `emitter.py`, `representer.py` and
//! `resolver.py` for the two `yaml.safe_dump` shapes `engine/` uses, both at `width=10**6` and
//! `allow_unicode=True`:
//!
//! - **flow, sorted** — `safe_dump(v, default_flow_style=True, width=10**6, allow_unicode=True)`,
//!   `sort_keys` left at its default of `True`. Used by `provenance::judgment_literal` (the
//!   `judgment:` literal is a structured value the write path splices in as one line) and by
//!   `write::to_literal`'s non-scalar branch. [`safe_dump_flow`].
//! - **block, insertion-ordered** — `safe_dump(front, allow_unicode=True, sort_keys=False,
//!   width=10**6)`. Used by `info::open_info` and `issues::open_issue`, which build a **new**
//!   note's frontmatter from a mapping. [`safe_dump_block`].
//!
//! # The invariant this module serves
//!
//! `lib.rs` says a vault file is never rewritten wholesale. That was once phrased "there is no
//! YAML emitter in this crate", which stopped being literally true in wave 1 (the flow emitter
//! lived in `provenance.rs`) and was reworded when this module lifted it out in wave 7. The rule
//! that survives is the one that was always meant: **no note is ever parsed and re-dumped**.
//! Every caller here either produces a single-line literal for `update_frontmatter_fields` to
//! splice in, or writes a brand-new file through `write::create`. Neither round-trips.
//!
//! # Why a transcription rather than a YAML crate
//!
//! Every byte is load-bearing. Judgment literals sit in the live vault and a re-render that
//! quoted differently would rewrite notes for no reason; new `info/` and `issues/` notes must be
//! byte-identical to what the Python engine writes during the dual-run week. `serde_yaml_ng` has
//! no flow-style emitter at all and its block style is not PyYAML's. So this is PyYAML's own
//! state machine — `write_indent`, `write_indicator`, `increase_indent`, and the
//! `column`/`whitespace`/`indention` flags — with the scalar layer (`analyze_scalar`,
//! `choose_scalar_style`, the three `write_*` writers, `check_simple_key`) and `resolver.py`'s
//! implicit-tag regexes. It is measured against PyYAML twice over: every expectation in
//! `provenance`'s tests is a string PyYAML produced, and the block layer is checked byte for byte
//! against `tests/fixtures/pyyaml-safe-dump-reference.json`, 188 cases PyYAML wrote.
//!
//! Two emitter settings shrink the transcription. `width=10**6` means PyYAML's `best_width` is a
//! million columns, so every `column > best_width` folding branch is dead — a 200-character
//! scalar stays on one line. `allow_unicode=True` means printable non-ASCII is never "special".
//! What is left is quoting, ordering, and indentation.
//!
//! # Documented divergences, all unreachable from this codebase
//!
//! - **Float exponent threshold.** PyYAML emits `repr(float).lower()`; Rust's `{:?}` is also
//!   shortest-round-trip but may switch to exponent notation at a different magnitude. The
//!   exponent *shape* (`1.0e+17`, signed, two digits) is normalised; the threshold is not.
//! - **`$` and a trailing newline** in the resolver regexes: Python's `$` also matches before one
//!   trailing newline, Rust's does not. A string ending in `\n` is never plain anyway (it has a
//!   line break), so the resolver's answer cannot change the style.
//! - **`Node::from(&Value)` drops a YAML tag.** PyYAML's `safe_dump` raises `RepresenterError` on
//!   anything it has no representer for; a tagged node cannot reach either caller.

use std::sync::OnceLock;

use jiff::civil::{Date, DateTime};
use regex::Regex;
use serde_yaml_ng::Value;

/// A value the emitter can write. The scalar variants are PyYAML's representable types;
/// [`Node::Date`] and [`Node::DateTime`] exist because `serde_yaml_ng::Value` cannot say "this is
/// a date", and the difference is observable: a `date` dumps plain (`expires: 2026-09-05`) while
/// the equal-looking *string* resolves to a timestamp and is quoted (`'2026-09-05'`).
///
/// Mapping keys may be any node, as in PyYAML; every key in this codebase is a string.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Bool(bool),
    /// `i128` so that both halves of `serde_yaml_ng::Number` (`i64` and `u64`) fit without a
    /// second variant. Python's `int` is unbounded; `str(int)` is all the emitter needs.
    Int(i128),
    Float(f64),
    Str(String),
    Date(Date),
    /// Naive, as PyYAML's `represent_datetime` writes `data.isoformat(' ')`.
    DateTime(DateTime),
    Seq(Vec<Node>),
    /// Insertion-ordered, as a Python `dict`; `sort_keys` is the emitter's decision, not the
    /// value's.
    Map(Vec<(Node, Node)>),
}

// ---------------------------------------------------------------------------------------------
// Why there is no `Node::Anchored` / `Node::Alias` — and what would have to change if a Python
// path ever aliased again.
//
// PyYAML aliases by **object identity**, which a tree cannot express. `safe_dump({"a": d, "b": d})`
// with ONE `date` object writes `a: &id001 2026-09-07` / `b: *id001`, because the representer
// caches the node by `id(data)` and the serializer anchors any node it reaches more than once.
// `ignore_aliases` exempts `None`, `str`, `bytes`, `bool`, `int` and `float`, so only a **date,
// datetime, sequence or mapping** can ever be anchored. The numbering is `'id%03d'` from 1, assigned
// in the order the serializer **detects the second occurrence** in its pre-pass — not the order the
// anchors are emitted. Measured, because the two orders disagree:
//
//     >>> yaml.safe_dump({'a': x, 'b': y, 'c': y, 'd': x})   # x, y two distinct lists
//     'a: &id002\n- 1\nb: &id001\n- 2\nc: *id001\nd: *id002\n'
//
// `write::propose_amendment` reached this for real on 2026-09-07: `proposed_at` and
// `first_proposed_at` were the same `today`, and the card Python wrote carried the anchor. A
// faithful `Node::Anchored`/`Node::Alias` pair reproduced it byte for byte — and was then removed,
// because reproducing it faithfully was the wrong fix.
//
// **The ruling (2026-09-07): no note this system writes may contain a YAML anchor.** Anchors are
// incompatible with single-line frontmatter surgery, which is the core edit invariant — a note is
// never parsed and re-dumped. `approvals::defer_over_budget` rewrites the `proposed_at` line of
// every pending non-digest approval, amend cards included; that leaves `first_proposed_at: *id001`
// pointing at nothing, the card raises `found undefined alias 'id001'`, and every tolerant reader
// then silently drops it — out of the deck AND out of `find_pending_amendment`, so the next run
// mints another. `engine/write.py` now gives the two fields distinct `date` objects, so neither
// engine emits an anchor and both stay byte-identical.
//
// If a Python path ever aliases again, the fix is the same one: give it distinct objects. Only if
// that were impossible would the variants come back — and then they would need what the review
// asked for and this emitter never had: a validator that every `Alias(n)` follows an `Anchored(n)`,
// that ids are unique, and that the numbering matches the detection order above.
// ---------------------------------------------------------------------------------------------

impl Node {
    pub fn text(s: &str) -> Node {
        Node::Str(s.to_string())
    }

    /// `None` → `null`, else the string.
    pub fn opt_text(s: Option<&str>) -> Node {
        s.map(Node::text).unwrap_or(Node::Null)
    }

    /// A string-keyed mapping in the order given.
    pub fn map(pairs: Vec<(&str, Node)>) -> Node {
        Node::Map(pairs.into_iter().map(|(k, v)| (Node::text(k), v)).collect())
    }

    /// `journal.jsonable` over a JSON value: the shape a journal record already has. Object keys
    /// arrive in `serde_json::Map`'s order, which is sorted — and so is every record on disk,
    /// because both engines write ledger lines with `sort_keys=True`.
    pub fn from_json(value: &serde_json::Value) -> Node {
        match value {
            serde_json::Value::Null => Node::Null,
            serde_json::Value::Bool(b) => Node::Bool(*b),
            serde_json::Value::Number(n) => number(n.as_i64(), n.as_u64(), n.as_f64()),
            serde_json::Value::String(s) => Node::Str(s.clone()),
            serde_json::Value::Array(items) => {
                Node::Seq(items.iter().map(Node::from_json).collect())
            }
            serde_json::Value::Object(map) => Node::Map(
                map.iter().map(|(k, v)| (Node::text(k), Node::from_json(v))).collect(),
            ),
        }
    }
}

fn number(i: Option<i64>, u: Option<u64>, f: Option<f64>) -> Node {
    if let Some(i) = i {
        Node::Int(i128::from(i))
    } else if let Some(u) = u {
        Node::Int(i128::from(u))
    } else {
        Node::Float(f.unwrap_or(f64::NAN))
    }
}

/// A YAML-loaded value, keys and all. A `Value::Tagged` contributes its inner value — see the
/// module doc.
impl From<&Value> for Node {
    fn from(value: &Value) -> Node {
        match value {
            Value::Null => Node::Null,
            Value::Bool(b) => Node::Bool(*b),
            Value::Number(n) => number(n.as_i64(), n.as_u64(), n.as_f64()),
            Value::String(s) => Node::Str(s.clone()),
            Value::Sequence(items) => Node::Seq(items.iter().map(Node::from).collect()),
            Value::Mapping(m) => {
                Node::Map(m.iter().map(|(k, v)| (Node::from(k), Node::from(v))).collect())
            }
            Value::Tagged(tagged) => Node::from(&tagged.value),
        }
    }
}

/// The two `safe_dump` keyword settings that vary between callers. `width` and `allow_unicode`
/// do not vary and are baked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// `default_flow_style`. PyYAML applies it to **every** collection in the document; an empty
    /// collection is written in flow style (`[]`, `{}`) regardless.
    pub flow: bool,
    /// `sort_keys`. PyYAML sorts with `try: sorted(mapping) except TypeError: pass`, so a mapping
    /// whose keys cannot be ordered against each other keeps its insertion order.
    pub sort_keys: bool,
}

/// `yaml.safe_dump(v, default_flow_style=True, width=10**6, allow_unicode=True)` — with
/// `sort_keys` at its default of `True` — before the caller's `.strip()`.
///
/// The trailing `...` is PyYAML's `open_ended` document-end marker: a **plain** scalar at the
/// document root leaves the document open-ended, so the stream end writes `...` on its own line.
/// That is why `judgment_literal("hello")` raises while `judgment_literal("true")` — quoted,
/// therefore not open-ended — returns `'true'`. A quirk, faithfully preserved; unreachable while
/// every caller passes a mapping.
pub fn safe_dump_flow(value: &Value) -> String {
    safe_dump(&Node::from(value), Options { flow: true, sort_keys: true })
}

/// `yaml.safe_dump(front, allow_unicode=True, sort_keys=False, width=10**6)` — the frontmatter
/// of a new `info/` or `issues/` note, trailing newline included.
pub fn safe_dump_block(node: &Node) -> String {
    safe_dump(node, Options { flow: false, sort_keys: false })
}

pub fn safe_dump(node: &Node, opts: Options) -> String {
    Emitter::new(opts).document(node)
}

// ---------------------------------------------------------------------------------------------
// The emitter state machine (PyYAML `emitter.py`), driven by recursion over the node instead of
// by an event queue and a state stack. Each `expect_*` below is the PyYAML method of that name.
// ---------------------------------------------------------------------------------------------

/// PyYAML's `best_indent`.
const BEST_INDENT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Single,
    Double,
}

struct Emitter {
    opts: Options,
    out: String,
    /// `self.indent` — `None` at the document root, exactly as PyYAML starts it.
    indent: Option<usize>,
    indents: Vec<Option<usize>>,
    flow_level: usize,
    column: usize,
    whitespace: bool,
    indention: bool,
    open_ended: bool,
    root_context: bool,
    mapping_context: bool,
    simple_key_context: bool,
}

impl Emitter {
    fn new(opts: Options) -> Emitter {
        Emitter {
            opts,
            out: String::new(),
            indent: None,
            indents: Vec::new(),
            flow_level: 0,
            column: 0,
            whitespace: true,
            indention: true,
            open_ended: false,
            root_context: false,
            mapping_context: false,
            simple_key_context: false,
        }
    }

    /// `expect_document_start` (implicit — nothing is written for the first document), the root
    /// node, `expect_document_end`, and the stream end.
    fn document(mut self, node: &Node) -> String {
        self.expect_node(node, true, false, false);
        self.write_indent();
        if self.open_ended {
            self.write_indicator("...", true, false, false);
            self.write_indent();
        }
        self.out
    }

    fn expect_node(&mut self, node: &Node, root: bool, mapping: bool, simple_key: bool) {
        self.root_context = root;
        self.mapping_context = mapping;
        self.simple_key_context = simple_key;
        // PyYAML's `process_anchor` has no counterpart here: nothing this crate emits is ever
        // anchored. See the ruling above `Node`.
        match node {
            Node::Seq(items) => {
                if self.flow_level > 0 || self.opts.flow || items.is_empty() {
                    self.expect_flow_sequence(items);
                } else {
                    self.expect_block_sequence(items);
                }
            }
            Node::Map(items) => {
                if self.flow_level > 0 || self.opts.flow || items.is_empty() {
                    self.expect_flow_mapping(items);
                } else {
                    self.expect_block_mapping(items);
                }
            }
            scalar => self.expect_scalar(scalar),
        }
    }

    /// `process_tag` + `expect_scalar` + `process_scalar`.
    fn expect_scalar(&mut self, node: &Node) {
        let text = scalar_text(node);
        let analysis = analyze_scalar(&text);
        let style = choose_scalar_style(
            &analysis,
            implicit_plain(node),
            self.flow_level,
            self.simple_key_context,
        );
        // `process_tag`: a non-plain scalar whose tag is not `!!str` gets its tag written. Only a
        // timestamp can get here — every other non-string scalar's text is plain in both
        // contexts — and only in flow context, where a datetime's colons forbid the plain style.
        if style != Style::Plain && !matches!(node, Node::Str(_)) {
            let tag = prepared_tag(node);
            self.write_indicator(tag, true, false, false);
        }
        self.increase_indent(true, false);
        match style {
            Style::Plain => self.write_plain(&text),
            Style::Single => self.write_single_quoted(&text),
            Style::Double => self.write_double_quoted(&text),
        }
        self.indent = self.indents.pop().expect("increase_indent pushed");
    }

    fn expect_flow_sequence(&mut self, items: &[Node]) {
        self.write_indicator("[", true, true, false);
        self.flow_level += 1;
        self.increase_indent(true, false);
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.write_indicator(",", false, false, false);
            }
            self.expect_node(item, false, false, false);
        }
        self.indent = self.indents.pop().expect("increase_indent pushed");
        self.flow_level -= 1;
        self.write_indicator("]", false, false, false);
    }

    fn expect_flow_mapping(&mut self, items: &[(Node, Node)]) {
        self.write_indicator("{", true, true, false);
        self.flow_level += 1;
        self.increase_indent(true, false);
        for (i, (key, value)) in self.ordered(items).into_iter().enumerate() {
            if i > 0 {
                self.write_indicator(",", false, false, false);
            }
            if check_simple_key(key) {
                self.expect_node(key, false, true, true);
                // expect_flow_mapping_simple_value
                self.write_indicator(":", false, false, false);
            } else {
                self.write_indicator("?", true, false, false);
                self.expect_node(key, false, true, false);
                // expect_flow_mapping_value
                self.write_indicator(":", true, false, false);
            }
            self.expect_node(value, false, true, false);
        }
        self.indent = self.indents.pop().expect("increase_indent pushed");
        self.flow_level -= 1;
        self.write_indicator("}", false, false, false);
    }

    fn expect_block_sequence(&mut self, items: &[Node]) {
        // A sequence directly under a mapping key is not indented (`a:\n- x`), because the
        // indention flag was cleared by the `:` indicator that preceded it.
        let indentless = self.mapping_context && !self.indention;
        self.increase_indent(false, indentless);
        for item in items {
            self.write_indent();
            self.write_indicator("-", true, false, true);
            self.expect_node(item, false, false, false);
        }
        self.indent = self.indents.pop().expect("increase_indent pushed");
    }

    fn expect_block_mapping(&mut self, items: &[(Node, Node)]) {
        self.increase_indent(false, false);
        for (key, value) in self.ordered(items) {
            self.write_indent();
            if check_simple_key(key) {
                self.expect_node(key, false, true, true);
                // expect_block_mapping_simple_value
                self.write_indicator(":", false, false, false);
            } else {
                // PyYAML's explicit-key form, `? key` on one line and `: value` on the next.
                self.write_indicator("?", true, false, true);
                self.expect_node(key, false, true, false);
                // expect_block_mapping_value
                self.write_indent();
                self.write_indicator(":", true, false, true);
            }
            self.expect_node(value, false, true, false);
        }
        self.indent = self.indents.pop().expect("increase_indent pushed");
    }

    /// `represent_mapping`'s `sort_keys` step: `try: sorted(mapping) except TypeError: pass`.
    ///
    /// Python cannot order a `str` against an `int`, so a mixed-key mapping keeps **insertion
    /// order** — verified: `{1: "a", "b": 2}` dumps as `{1: a, b: 2}`, unsorted. Every judgment
    /// mapping in this codebase is all-string-keyed and sorts by code point.
    fn ordered<'a>(&self, items: &'a [(Node, Node)]) -> Vec<(&'a Node, &'a Node)> {
        let mut pairs: Vec<(&Node, &Node)> = items.iter().map(|(k, v)| (k, v)).collect();
        if !self.opts.sort_keys {
            return pairs;
        }
        if pairs.iter().all(|(k, _)| matches!(k, Node::Str(_))) {
            pairs.sort_by(|a, b| match (a.0, b.0) {
                (Node::Str(x), Node::Str(y)) => x.cmp(y),
                _ => std::cmp::Ordering::Equal,
            });
        } else if pairs
            .iter()
            .all(|(k, _)| matches!(k, Node::Int(_) | Node::Float(_) | Node::Bool(_)))
        {
            // Python orders bools with ints (True == 1); floats and ints compare across types.
            pairs.sort_by(|a, b| {
                numeric_key(a.0)
                    .partial_cmp(&numeric_key(b.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        pairs
    }

    fn increase_indent(&mut self, flow: bool, indentless: bool) {
        self.indents.push(self.indent);
        match self.indent {
            None => self.indent = Some(if flow { BEST_INDENT } else { 0 }),
            Some(current) if !indentless => self.indent = Some(current + BEST_INDENT),
            Some(_) => {}
        }
    }

    fn write_indent(&mut self) {
        let indent = self.indent.unwrap_or(0);
        if !self.indention
            || self.column > indent
            || (self.column == indent && !self.whitespace)
        {
            self.write_line_break(None);
        }
        if self.column < indent {
            self.whitespace = true;
            self.out.push_str(&" ".repeat(indent - self.column));
            self.column = indent;
        }
    }

    fn write_indicator(
        &mut self,
        indicator: &str,
        need_whitespace: bool,
        whitespace: bool,
        indention: bool,
    ) {
        if !self.whitespace && need_whitespace {
            self.out.push(' ');
            self.column += 1;
        }
        self.whitespace = whitespace;
        self.indention = self.indention && indention;
        self.column += indicator.chars().count();
        self.open_ended = false;
        self.out.push_str(indicator);
    }

    /// `write_line_break(data)`: the exotic breaks (U+0085, U+2028, U+2029) are written as
    /// themselves; `None` is `best_line_break`, `\n`.
    fn write_line_break(&mut self, data: Option<char>) {
        self.whitespace = true;
        self.indention = true;
        self.column = 0;
        self.out.push(data.unwrap_or('\n'));
    }

    fn write_text(&mut self, chars: &[char]) {
        self.column += chars.len();
        self.out.extend(chars.iter());
    }

    /// `write_plain` at a width nothing reaches: the folding loop writes the text through
    /// unchanged, and a plain scalar never holds a line break (`analyze_scalar` forbids it).
    fn write_plain(&mut self, text: &str) {
        if self.root_context {
            self.open_ended = true;
        }
        if text.is_empty() {
            return;
        }
        if !self.whitespace {
            self.out.push(' ');
            self.column += 1;
        }
        self.whitespace = false;
        self.indention = false;
        let chars: Vec<char> = text.chars().collect();
        self.write_text(&chars);
    }

    /// PyYAML's `write_single_quoted`: double the quotes, and fold a run of line breaks onto the
    /// current indent.
    ///
    /// The fold is `'a\nb'` → `'a\n\n  b'`: **one extra break when the run starts with `\n`**
    /// (`write_line_break()` is called before the per-character loop), then one per break
    /// character, then `write_indent`. That is why `"one\ntwo"` gains a blank line and
    /// `"u2028\u{2028}here"` does not. The indent is the scalar's own — two deeper than the
    /// collection it sits in — which the reference corpus pins at three depths.
    fn write_single_quoted(&mut self, text: &str) {
        self.write_indicator("'", true, false, false);
        let chars: Vec<char> = text.chars().collect();
        let mut spaces = false;
        let mut breaks = false;
        let mut start = 0;
        let mut end = 0;
        while end <= chars.len() {
            let ch = chars.get(end).copied();
            if spaces {
                if ch != Some(' ') {
                    // The `column > best_width` fold is dead; the run is written as is.
                    self.write_text(&chars[start..end]);
                    start = end;
                }
            } else if breaks {
                if !ch.is_some_and(is_break) {
                    if chars[start] == '\n' {
                        self.write_line_break(None);
                    }
                    for &br in &chars[start..end] {
                        if br == '\n' {
                            self.write_line_break(None);
                        } else {
                            self.write_line_break(Some(br));
                        }
                    }
                    self.write_indent();
                    start = end;
                }
            } else if ch.is_none()
                || ch.is_some_and(|c| c == ' ' || is_break(c) || c == '\'')
            {
                if start < end {
                    self.write_text(&chars[start..end]);
                    start = end;
                }
            }
            if ch == Some('\'') {
                self.out.push_str("''");
                self.column += 2;
                start = end + 1;
            }
            if let Some(c) = ch {
                spaces = c == ' ';
                breaks = is_break(c);
            }
            end += 1;
        }
        self.write_indicator("'", false, false, false);
    }

    fn write_double_quoted(&mut self, text: &str) {
        self.write_indicator("\"", true, false, false);
        let body = double_quoted_body(text);
        self.column += body.chars().count();
        self.out.push_str(&body);
        self.write_indicator("\"", false, false, false);
    }
}

fn numeric_key(node: &Node) -> Option<f64> {
    match node {
        Node::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Node::Int(i) => Some(*i as f64),
        Node::Float(f) => Some(*f),
        _ => None,
    }
}

/// PyYAML's `check_simple_key`. A key that fails this is written as `? key` / `: value`.
///
/// The length budget includes the prepared tag (`!!str` is 5 characters), so a plain string key
/// goes explicit at 123 characters, not 128 — measured against PyYAML, not derived. An empty or
/// multiline key is never simple, which is how the empty key takes the `? ''` form.
fn check_simple_key(key: &Node) -> bool {
    match key {
        Node::Seq(items) => items.is_empty(),
        Node::Map(items) => items.is_empty(),
        scalar => {
            let text = scalar_text(scalar);
            let analysis = analyze_scalar(&text);
            prepared_tag(scalar).len() + text.chars().count() < 128
                && !analysis.empty
                && !analysis.multiline
        }
    }
}

/// `prepare_tag` of the tag PyYAML's representer gives a node: `!!str`, `!!int`, …
fn prepared_tag(node: &Node) -> &'static str {
    match node {
        Node::Null => "!!null",
        Node::Bool(_) => "!!bool",
        Node::Int(_) => "!!int",
        Node::Float(_) => "!!float",
        Node::Str(_) => "!!str",
        Node::Date(_) | Node::DateTime(_) => "!!timestamp",
        Node::Seq(_) => "!!seq",
        Node::Map(_) => "!!map",
    }
}

/// The scalar text PyYAML's representers produce for a node.
fn scalar_text(node: &Node) -> String {
    match node {
        Node::Null => "null".to_string(),
        Node::Bool(true) => "true".to_string(),
        Node::Bool(false) => "false".to_string(),
        Node::Int(i) => i.to_string(),
        Node::Float(f) => python_float_repr(*f),
        Node::Str(s) => s.clone(),
        // `date.isoformat()`.
        Node::Date(d) => d.strftime("%Y-%m-%d").to_string(),
        // `datetime.isoformat(' ')`: microseconds only when non-zero.
        Node::DateTime(dt) => {
            let micros = dt.subsec_nanosecond() / 1_000;
            if micros == 0 {
                dt.strftime("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                format!("{}.{micros:06}", dt.strftime("%Y-%m-%d %H:%M:%S"))
            }
        }
        Node::Seq(_) | Node::Map(_) => String::new(),
    }
}

/// PyYAML's `represent_float`: `.nan` / `.inf` / `-.inf`, else `repr(data).lower()`, and if that
/// has an exponent but no `.` it becomes `1.0e+17` — a bare `1e+17` is not a valid `!!float`.
fn python_float_repr(f: f64) -> String {
    if f.is_nan() {
        return ".nan".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { ".inf" } else { "-.inf" }.to_string();
    }
    let mut value = format!("{f:?}").to_lowercase();
    if let Some(pos) = value.find('e') {
        // Python's repr writes a signed, zero-padded, at-least-two-digit exponent; Rust's does
        // not. The magnitude at which each *switches* to exponent form still differs — the one
        // divergence this module cannot close without reimplementing `repr`.
        let (mantissa, exponent) = value.split_at(pos);
        let exponent = &exponent[1..];
        let (sign, digits) = match exponent.strip_prefix('-') {
            Some(rest) => ('-', rest),
            None => ('+', exponent.trim_start_matches('+')),
        };
        value = format!("{mantissa}e{sign}{digits:0>2}");
    }
    if !value.contains('.') && value.contains('e') {
        value = value.replacen('e', ".0e", 1);
    }
    value
}

/// PyYAML's `event.implicit[0]`: would this node's text, read back as a *plain* scalar, resolve
/// to the tag it already has? If not, it must be quoted.
///
/// For everything but a string the answer is yes by construction (the representer produced the
/// text from the tag). For a string it is the resolver question: `true`, `123`, `2026-08-31`,
/// `~`, `<<`, `=` and the empty string all resolve to something that is not `!!str`, so they get
/// quoted — which is why the live vault holds `at: '2026-08-31T20:02:00.603Z'`.
fn implicit_plain(node: &Node) -> bool {
    match node {
        Node::Str(s) => resolves_to_str(s),
        _ => true,
    }
}

/// PyYAML `resolver.py`'s implicit resolvers, transcribed (the `re.X` whitespace removed).
///
/// Transcribed from **PyYAML 6.0.3**, the pinned version — these patterns do change between
/// releases. The 5.x float pattern read `\.[0-9_]+`, which makes `.__` a float; 6.x requires a
/// digit after the dot, so `.__` is a plain string. A differential run against the installed
/// PyYAML is the only way to catch that class of drift, and it did.
///
/// Two YAML-1.1 quirks worth knowing before you "fix" one of these: the float pattern **requires
/// a signed exponent**, so `1.0e5` is a string while `1.0e+5` is a float; and the int pattern
/// includes sexagesimals, so `1:30` is an integer. Both are quoted-or-not decisions that show up
/// in real notes. A third, from the timestamp pattern: it **requires seconds**, so
/// `2026-08-26T23:59` — the vault's own `due` spelling — stays a plain string.
fn resolves_to_str(s: &str) -> bool {
    static RESOLVERS: OnceLock<Vec<Regex>> = OnceLock::new();
    let resolvers = RESOLVERS.get_or_init(|| {
        [
            // bool
            r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)$",
            // float
            r"^(?:[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$",
            // int
            r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)$",
            // merge
            r"^(?:<<)$",
            // null — note the empty final alternative: "" resolves to null, so it is quoted.
            r"^(?:~|null|Null|NULL|)$",
            // timestamp
            r"^(?:[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]|[0-9][0-9][0-9][0-9]-[0-9][0-9]?-[0-9][0-9]?(?:[Tt]|[ \t]+)[0-9][0-9]?:[0-9][0-9]:[0-9][0-9](?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9][0-9]?(?::[0-9][0-9])?))?)$",
            // value
            r"^(?:=)$",
            // yaml
            r"^(?:!|&|\*)$",
        ]
        .iter()
        .map(|p| Regex::new(p).expect("resolver pattern"))
        .collect()
    });
    !resolvers.iter().any(|r| r.is_match(s))
}

/// PyYAML's `ScalarAnalysis`, reduced to the fields this emitter reads.
struct Analysis {
    empty: bool,
    multiline: bool,
    allow_flow_plain: bool,
    allow_block_plain: bool,
    allow_single_quoted: bool,
}

fn is_break(c: char) -> bool {
    matches!(c, '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// PyYAML's whitespace-or-nothing test inside `analyze_scalar` (`'\0 \t\r\n\x85\u2028\u2029'`).
/// Note this is a *different, smaller* set than `pystr::is_python_space` — it is YAML's, not
/// Python's.
fn is_yaml_space(c: char) -> bool {
    matches!(c, '\0' | ' ' | '\t' | '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// PyYAML's `analyze_scalar`, with `allow_unicode=True` folded in.
///
/// The rules that actually bite, all confirmed against the oracle:
/// `:` **anywhere** forbids a plain scalar in flow context (`agent:routine.enrich` is quoted,
/// but `gmail:1a04a6` in *block* context is not); `?`, `-` and the quote characters only matter
/// at index 0 (`say "hi"` and `-lead` stay plain); `#` only matters when preceded by whitespace
/// (`Homework #1` is quoted, `a#b` is not); a `\t` or a `\r` makes a scalar "special", which
/// forces double quotes, while `\n` is a line break and stays single-quoted with the fold.
fn analyze_scalar(scalar: &str) -> Analysis {
    let chars: Vec<char> = scalar.chars().collect();
    if chars.is_empty() {
        return Analysis {
            empty: true,
            multiline: false,
            allow_flow_plain: false,
            allow_block_plain: true,
            allow_single_quoted: true,
        };
    }

    let mut block_indicators = false;
    let mut flow_indicators = false;
    let mut line_breaks = false;
    let mut special_characters = false;

    let mut leading_space = false;
    let mut leading_break = false;
    let mut trailing_space = false;
    let mut trailing_break = false;
    let mut break_space = false;
    let mut space_break = false;

    if scalar.starts_with("---") || scalar.starts_with("...") {
        block_indicators = true;
        flow_indicators = true;
    }

    let last = chars.len() - 1;
    let mut preceded_by_whitespace = true;
    let mut followed_by_whitespace = chars.len() == 1 || is_yaml_space(chars[1]);
    let mut previous_space = false;
    let mut previous_break = false;

    for index in 0..chars.len() {
        let ch = chars[index];

        if index == 0 {
            if "#,[]{}&*!|>'\"%@`".contains(ch) {
                flow_indicators = true;
                block_indicators = true;
            }
            if ch == '?' || ch == ':' {
                flow_indicators = true;
                if followed_by_whitespace {
                    block_indicators = true;
                }
            }
            if ch == '-' && followed_by_whitespace {
                flow_indicators = true;
                block_indicators = true;
            }
        } else {
            if ",?[]{}".contains(ch) {
                flow_indicators = true;
            }
            if ch == ':' {
                flow_indicators = true;
                if followed_by_whitespace {
                    block_indicators = true;
                }
            }
            if ch == '#' && preceded_by_whitespace {
                flow_indicators = true;
                block_indicators = true;
            }
        }

        if is_break(ch) {
            line_breaks = true;
        }
        if !(ch == '\n' || ('\u{20}'..='\u{7e}').contains(&ch)) {
            // allow_unicode=True, so printable non-ASCII is fine. The upper bound is
            // `< '\U0010ffff'` in PyYAML — exclusive — so U+10FFFF itself is "special".
            let printable_unicode = (ch == '\u{85}'
                || ('\u{a0}'..='\u{d7ff}').contains(&ch)
                || ('\u{e000}'..='\u{fffd}').contains(&ch)
                || ('\u{10000}'..'\u{10ffff}').contains(&ch))
                && ch != '\u{feff}';
            if !printable_unicode {
                special_characters = true;
            }
        }

        if ch == ' ' {
            if index == 0 {
                leading_space = true;
            }
            if index == last {
                trailing_space = true;
            }
            if previous_break {
                break_space = true;
            }
            previous_space = true;
            previous_break = false;
        } else if is_break(ch) {
            if index == 0 {
                leading_break = true;
            }
            if index == last {
                trailing_break = true;
            }
            if previous_space {
                space_break = true;
            }
            previous_space = false;
            previous_break = true;
        } else {
            previous_space = false;
            previous_break = false;
        }

        preceded_by_whitespace = is_yaml_space(ch);
        followed_by_whitespace = index + 2 >= chars.len() || is_yaml_space(chars[index + 2]);
    }

    let mut allow_flow_plain = true;
    let mut allow_block_plain = true;
    let mut allow_single_quoted = true;

    if leading_space || leading_break || trailing_space || trailing_break {
        allow_flow_plain = false;
        allow_block_plain = false;
    }
    if break_space {
        allow_flow_plain = false;
        allow_block_plain = false;
        allow_single_quoted = false;
    }
    if space_break || special_characters {
        allow_flow_plain = false;
        allow_block_plain = false;
        allow_single_quoted = false;
    }
    if line_breaks {
        allow_flow_plain = false;
        allow_block_plain = false;
    }
    if flow_indicators {
        allow_flow_plain = false;
    }
    if block_indicators {
        allow_block_plain = false;
    }

    Analysis {
        empty: false,
        multiline: line_breaks,
        allow_flow_plain,
        allow_block_plain,
        allow_single_quoted,
    }
}

/// PyYAML's `choose_scalar_style` for `style=None, canonical=False`.
fn choose_scalar_style(
    analysis: &Analysis,
    implicit: bool,
    flow_level: usize,
    simple_key_context: bool,
) -> Style {
    if implicit
        && !(simple_key_context && (analysis.empty || analysis.multiline))
        && ((flow_level > 0 && analysis.allow_flow_plain)
            || (flow_level == 0 && analysis.allow_block_plain))
    {
        return Style::Plain;
    }
    if analysis.allow_single_quoted && !(simple_key_context && analysis.multiline) {
        return Style::Single;
    }
    Style::Double
}

/// PyYAML's `ESCAPE_REPLACEMENTS`.
fn escape_replacement(ch: char) -> Option<char> {
    Some(match ch {
        '\0' => '0',
        '\u{7}' => 'a',
        '\u{8}' => 'b',
        '\t' => 't',
        '\n' => 'n',
        '\u{b}' => 'v',
        '\u{c}' => 'f',
        '\r' => 'r',
        '\u{1b}' => 'e',
        '"' => '"',
        '\\' => '\\',
        '\u{85}' => 'N',
        '\u{a0}' => '_',
        '\u{2028}' => 'L',
        '\u{2029}' => 'P',
        _ => return None,
    })
}

/// The inside of PyYAML's `write_double_quoted`, the last-resort style — reached only when a
/// scalar holds a control character (a tab or a `\r` included).
///
/// Note the escape ranges are *narrower* than `analyze_scalar`'s: astral characters are escaped
/// here (`\U0001F600`) even though they are perfectly plain elsewhere.
fn double_quoted_body(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        let must_escape =
            matches!(ch, '"' | '\\' | '\u{85}' | '\u{2028}' | '\u{2029}' | '\u{feff}')
                || !(('\u{20}'..='\u{7e}').contains(&ch)
                    || ('\u{a0}'..='\u{d7ff}').contains(&ch)
                    || ('\u{e000}'..='\u{fffd}').contains(&ch));
        if !must_escape {
            out.push(ch);
            continue;
        }
        match escape_replacement(ch) {
            Some(r) => {
                out.push('\\');
                out.push(r);
            }
            None => {
                let cp = ch as u32;
                if cp <= 0xFF {
                    out.push_str(&format!("\\x{cp:02X}"));
                } else if cp <= 0xFFFF {
                    out.push_str(&format!("\\u{cp:04X}"));
                } else {
                    out.push_str(&format!("\\U{cp:08X}"));
                }
            }
        }
    }
    out
}

/// `write_double_quoted` as a pure function, quotes included — for tests of the escape table.
#[cfg(test)]
pub(crate) fn write_double_quoted(text: &str) -> String {
    format!("\"{}\"", double_quoted_body(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rebuild one corpus value from its tagged JSON form. The tags are the generator's:
    /// `s` str, `n` None, `b` bool, `i` int, `f` float, `d` date, `dt` naive datetime, `l` list,
    /// `m` mapping as `[[key, value], …]` so that insertion order survives JSON.
    fn tagged(value: &serde_json::Value) -> Node {
        let (tag, inner) = value
            .as_object()
            .and_then(|o| o.iter().next())
            .expect("a tagged value is a one-key object");
        match tag.as_str() {
            "s" => Node::Str(inner.as_str().unwrap().to_string()),
            "n" => Node::Null,
            "b" => Node::Bool(inner.as_bool().unwrap()),
            "i" => Node::Int(i128::from(inner.as_i64().unwrap())),
            "f" => Node::Float(match inner {
                serde_json::Value::Number(n) => n.as_f64().unwrap(),
                // Python's json.dumps writes the non-finite floats as bare `Infinity` /
                // `-Infinity`, which serde_json refuses; `corpus()` rewrites those two tokens
                // to strings before parsing.
                serde_json::Value::String(s) => s.parse::<f64>().unwrap(),
                other => panic!("float case {other:?}"),
            }),
            "d" => Node::Date(inner.as_str().unwrap().parse().unwrap()),
            "dt" => Node::DateTime(inner.as_str().unwrap().parse().unwrap()),
            "l" => Node::Seq(inner.as_array().unwrap().iter().map(tagged).collect()),
            "m" => Node::Map(
                inner
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pair| {
                        let pair = pair.as_array().unwrap();
                        (Node::Str(pair[0].as_str().unwrap().to_string()), tagged(&pair[1]))
                    })
                    .collect(),
            ),
            other => panic!("unknown corpus tag {other:?}"),
        }
    }

    fn corpus() -> Vec<(Node, String)> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pyyaml-safe-dump-reference.json");
        let text = std::fs::read_to_string(&path).expect("frozen corpus present");
        let text = text
            .replace("{\"f\": -Infinity}", "{\"f\": \"-inf\"}")
            .replace("{\"f\": Infinity}", "{\"f\": \"inf\"}");
        let cases: Vec<(serde_json::Value, String)> = serde_json::from_str(&text).unwrap();
        cases.into_iter().map(|(input, expected)| (tagged(&input), expected)).collect()
    }

    /// The oracle: 188 `[tagged input, expected]` pairs PyYAML produced with the exact arguments
    /// `info` and `issues` use. Every case is compared as a whole string, and the corpus is
    /// never regenerated — if this emitter disagrees with it, the emitter is wrong.
    #[test]
    fn block_emitter_matches_every_frozen_pyyaml_case() {
        let cases = corpus();
        assert_eq!(cases.len(), 188, "the frozen corpus has 188 cases");
        let mut failures = Vec::new();
        for (index, (node, expected)) in cases.iter().enumerate() {
            let got = safe_dump_block(node);
            if &got != expected {
                failures.push(format!(
                    "case {index}: {node:?}\n  want {expected:?}\n  got  {got:?}"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} of 188 differ:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// The two frontmatter shapes the corpus ends with, re-checked as documents rather than as
    /// members of the loop, so a regression there names itself.
    #[test]
    fn the_real_frontmatter_shapes_are_in_the_corpus() {
        let cases = corpus();
        let info = &cases[184].1;
        assert!(info.starts_with("type: info\nid: info_3d9521e0b7\n"), "{info}");
        assert!(info.contains("\nexpires: 2026-09-05\nclosed_at: null\n"), "{info}");
        let issue = &cases[186].1;
        assert!(issue.contains("\ncategories:\n- wrong-effort\n- wrong-course\n"), "{issue}");
        assert!(
            issue.contains(
                "\nsnapshot:\n  title: CS 100 HW 01 Introduction to C\n  fields:\n    title:"
            ),
            "{issue}"
        );
    }

    #[test]
    fn control_characters_force_double_quotes() {
        // The styles are PyYAML's, measured.
        assert_eq!(&write_double_quoted("a\u{7}b"), "\"a\\ab\"");
        assert_eq!(&write_double_quoted("\u{1b}[0m"), "\"\\e[0m\"");
        assert_eq!(&write_double_quoted("\u{feff}bom"), "\"\\uFEFFbom\"");
        assert_eq!(&write_double_quoted("a\tb"), "\"a\\tb\"");
        assert_eq!(&write_double_quoted("😀"), "\"\\U0001F600\"");
    }

    /// A datetime in flow context is the one scalar that gets a tag written: its colons forbid
    /// the plain style, and a quoted timestamp would read back as a string. PyYAML:
    /// `{k: !!timestamp '2026-09-05 10:00:00'}`. Unreachable from any caller — no flow caller
    /// passes a datetime — but the transcription is honest about it.
    #[test]
    fn a_quoted_timestamp_carries_its_tag() {
        let dt: DateTime = "2026-09-05T10:00:00".parse().unwrap();
        let node = Node::map(vec![("k", Node::DateTime(dt))]);
        assert_eq!(
            safe_dump(&node, Options { flow: true, sort_keys: true }),
            "{k: !!timestamp '2026-09-05 10:00:00'}\n"
        );
        // In block context the same value is plain.
        assert_eq!(safe_dump_block(&node), "k: 2026-09-05 10:00:00\n");
        // A date has no colon and is plain in both.
        let date = Node::map(vec![("k", Node::Date("2026-09-05".parse().unwrap()))]);
        assert_eq!(
            safe_dump(&date, Options { flow: true, sort_keys: true }),
            "{k: 2026-09-05}\n"
        );
    }

    /// After an explicit key the `:` indicator is written from a fresh line, so `indention` is
    /// still set and the sequence that follows is **indented**, not indentless — the one place
    /// the `indentless` rule flips. Measured: PyYAML 6.0.3.
    #[test]
    fn a_sequence_after_an_explicit_key_is_indented() {
        let node = Node::map(vec![("", Node::Seq(vec![Node::Int(1), Node::Int(2)]))]);
        assert_eq!(safe_dump_block(&node), "? ''\n: - 1\n  - 2\n");
    }

    #[test]
    fn from_json_and_from_yaml_agree_on_the_scalar_types() {
        let json: serde_json::Value =
            serde_json::json!({"a": 1, "b": 2.5, "c": null, "d": [true, "x"]});
        let yaml: Value =
            serde_yaml_ng::from_str("a: 1\nb: 2.5\nc: null\nd: [true, x]\n").unwrap();
        assert_eq!(Node::from_json(&json), Node::from(&yaml));
        assert_eq!(
            safe_dump_block(&Node::from(&yaml)),
            "a: 1\nb: 2.5\nc: null\nd:\n- true\n- x\n"
        );
    }

    /// The same date twice is two plain scalars — this emitter has no anchor machinery, and after
    /// the 2026-09-07 ruling neither engine produces one. The guard that actually proves the bug
    /// is dead lives in `write::tests`, where the card is minted, its `proposed_at` line rewritten
    /// the way `defer_over_budget` does, and then loaded.
    #[test]
    fn a_repeated_date_emits_twice_and_never_an_anchor() {
        let d: Date = "2026-09-07".parse().unwrap();
        let node = Node::map(vec![
            ("proposed_at", Node::Date(d)),
            ("first_proposed_at", Node::Date(d)),
        ]);
        assert_eq!(
            safe_dump_block(&node),
            "proposed_at: 2026-09-07\nfirst_proposed_at: 2026-09-07\n"
        );
    }
}
