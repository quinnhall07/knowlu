//! The judgment seam (Knowlu spec §5.2, decision 11): one module answers every judgment the same
//! way, in three tiers — deterministic heuristics in the engine's own terms, then a promoted rule
//! table, then a local model, and only when the tiers above it left a gap.
//!
//! **Pure.** Nothing here starts a process, opens a socket, reads the clock or writes a file. The
//! model is a trait (`Model`), the rule table is a trait (`Rules`), and what the vault knows is a
//! value (`Heuristics`) the caller loads once per run. That is what lets every test drive a
//! scripted fake and what keeps `rank` — which never calls a model (decision 11) — structurally
//! unable to reach one from here.
//!
//! **This module answers; it never writes.** `enrich` turns an `Outcome` into a judged write
//! through `write`, with an agent actor, so judge-once holds and a field Quinn set comes back as a
//! `kind: amend` card instead of being overwritten.
//!
//! The three agent actors this programme uses are `agent:knowlu.enrich` (plan 3a),
//! `agent:knowlu.events` (3b) and `agent:knowlu.gmail` (3c). They start with `agent:` because
//! `provenance::is_agent` is a plain `starts_with("agent:")` test and nothing else — an actor
//! spelled `knowlu/enrich` would skip judge-once silently and write no provenance block.

use std::collections::BTreeMap;
use std::path::Path;

/// Under this, the model's answer is refused: it is logged, and nothing is written.
///
/// 0.6 rather than a higher bar because the alternative to a merely-plausible estimate is **no
/// estimate at all** — `ingest`'s template leaves `effort_hours: 1.0, importance: 3` on every
/// Blackboard task, which is worse than a considered 2.5, and every field written this way is a
/// judged write Quinn can overrule in the console at any time.
pub const CONFIDENCE_FLOOR: f64 = 0.6;

/// How much of a note's body reaches the prompt. A bound, not a guess: an assignment description
/// that carries a whole syllabus would otherwise push the real question out of the context window.
pub const MAX_BODY_CHARS: usize = 1200;
/// How much of a course note's `## Grade weights` section reaches the prompt.
pub const MAX_WEIGHTS_CHARS: usize = 600;
/// How much of `profile/preferences.md` reaches the prompt.
pub const MAX_PREFS_CHARS: usize = 600;
/// The cap on a generated `importance_reason`. It becomes one frontmatter line, so it must survive
/// `write::single_line_problem`, and a paragraph in a note's frontmatter is unreadable anyway.
pub const MAX_REASON_CHARS: usize = 140;

/// One thing to judge, flattened out of a task note by `enrich::pending`.
///
/// `id` is the note's opaque `id:` — the key judge-once and the judgment log are both written
/// against. An item with no id never reaches here: `enrich` skips it and says so, because without
/// one `journal::human_set` cannot answer "did Quinn set this?" and the whole protection is void.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub rel_path: String,
    pub title: String,
    pub body: String,
    pub source_uid: String,
    pub created_by: String,
    pub course: Option<String>,
    pub due: Option<String>,
    pub effort_hours: f64,
    pub effort_source: String,
}

/// What a tier answered. **Every field is optional**, because a tier answers what it can: tier 1
/// attributes a course and accepts a vendor's effort and stops there, and that partial answer is
/// still worth writing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Verdict {
    pub course: Option<String>,
    pub effort_hours: Option<f64>,
    pub importance: Option<i64>,
    pub importance_reason: Option<String>,
    /// 0–1. Tier 1 and tier 2 are deterministic and answer 1.0; only tier 3's is a judgement.
    pub confidence: f64,
    /// 0 nothing, 1 heuristics, 2 a promoted rule, 3 the model.
    pub tier: u8,
}

impl Verdict {
    /// Enough to clear `needs_enrichment`.
    ///
    /// **`course` is deliberately not required.** The cloud routine writes `needs_enrichment=false`
    /// even when it could not attribute a course, saying so in `importance_reason`; an item held
    /// open forever because no course matched would be re-judged twice a day for the rest of the
    /// semester.
    pub fn complete(&self) -> bool {
        self.effort_hours.is_some() && self.importance.is_some() && self.importance_reason.is_some()
    }
}

/// Why an answer did not become a verdict. Three causes, one variant — but the cause is
/// **structural**, not parsed back out of `why`, because the log records the cause and must never
/// record `why` (which carries the runtime's stderr on the `ModelFailed` branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowCause {
    /// The model answered, and the answer was under `CONFIDENCE_FLOOR`.
    BelowFloor,
    /// The model answered above the floor, but the merged verdict is still missing a field.
    Incomplete,
    /// The model did not answer at all: the process failed, timed out, or replied unparseably.
    ModelFailed,
}

impl LowCause {
    /// Stable, lower case, no punctuation — what `judgelog` records; `why`'s full text goes to
    /// stdout instead (Task 7 prints it, Task 9 captures it), never to the log.
    pub fn label(&self) -> &'static str {
        match self {
            LowCause::BelowFloor => "below floor",
            LowCause::Incomplete => "incomplete",
            LowCause::ModelFailed => "model failed",
        }
    }
}

/// The four ways a judgment ends. Three of them are normal (spec §5.3: "the app runs with no model
/// present"), and none of them is a failed slot.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Answered(Verdict),
    /// The model was reached and its answer was not used: under the floor, incomplete, or the call
    /// itself failed. `seed` is what the tiers ABOVE the model had — the refused answer is never
    /// merged in. `cause` is which of the three it was, structurally; `why` says the same thing in
    /// words, verbatim — including the runtime's stderr on the `ModelFailed` branch, which is
    /// exactly why `cause`, not `why`, is what a log may ever record.
    LowConfidence { seed: Verdict, why: String, cause: LowCause },
    ModelNotInstalled(Verdict),
    RuntimeNotInstalled(Verdict),
}

impl Outcome {
    /// What may be written, whatever the outcome. Always the tiers that succeeded, never a refused
    /// model answer.
    pub fn verdict(&self) -> &Verdict {
        match self {
            Outcome::Answered(v) => v,
            Outcome::LowConfidence { seed, .. } => seed,
            Outcome::ModelNotInstalled(v) => v,
            Outcome::RuntimeNotInstalled(v) => v,
        }
    }

    /// The word `enrich` prints and `judgelog` records. Stable, lower case, no punctuation.
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Answered(_) => "answered",
            Outcome::LowConfidence { .. } => "low confidence",
            Outcome::ModelNotInstalled(_) => "model not installed",
            Outcome::RuntimeNotInstalled(_) => "runtime not installed",
        }
    }
}

/// Why there is no model to call. Resolved once per run, before any item is judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    Runtime,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    Failed(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::Failed(m) => write!(f, "{m}"),
        }
    }
}

/// Tier 3. The implementation owns the transport, the grammar and the reply format; this module
/// owns only the tiers — which is what makes `runtime::PerCall` and a scripted fake substitutable.
pub trait Model {
    fn judge(&self, item: &Item, h: &Heuristics, seed: &Verdict) -> Result<Verdict, ModelError>;
}

/// Tier 2 — **the seam plan 3b fills** (spec §5.4). A rule that reproduces the model's output
/// across repeated cases is proposed for promotion as an approval card; once approved it enters
/// this tier and the model stops being called for that pattern. Nothing in plan 3a promotes
/// anything, so the only implementation here is [`NoRules`].
pub trait Rules {
    fn lookup(&self, item: &Item) -> Option<Verdict>;
}

/// The empty rule table plan 3a ships. Deliberately not a stub with an opinion: a heuristic
/// invented here would be one 3b's promotion loop then had to argue with.
pub struct NoRules;

impl Rules for NoRules {
    fn lookup(&self, _item: &Item) -> Option<Verdict> {
        None
    }
}

/// What tier 1 knows and what the prompt is grounded in — loaded once per run from the vault, so
/// judging a hundred items reads `config/` and `courses/` once rather than a hundred times.
#[derive(Debug, Clone, PartialEq)]
pub struct Heuristics {
    /// `config/ingest.yaml`'s `course_map`, **in file order** — `match_course_fields` walks it in
    /// order and the first match wins, exactly as the ICS ingest does.
    pub course_map: Vec<(String, String)>,
    /// `config/planning.yaml`'s `slice_hours`. Never an answer — an anchor in the prompt, so an
    /// estimate is expressed in the units this vault actually works in.
    pub slice_hours: f64,
    /// slug -> the course note's `## Grade weights` section, clipped. **The key set is "the
    /// courses this vault has notes for"; an empty value means "that note has no `## Grade
    /// weights` section"** — a course note that just doesn't have one is still a known course.
    /// Keyed by the note's own `slug:` frontmatter field when present, else the file stem. This is
    /// what makes an importance answer grounded rather than invented; the routine's step 3 reads
    /// the same thing.
    pub weights: BTreeMap<String, String>,
    /// `profile/preferences.md`, clipped. Absent on a friend's fresh vault, which is fine.
    pub preferences: String,
}

/// Python's `str[:n]` by **characters**, never bytes: `&s[..n]` panics in the middle of a
/// multi-byte character, and a note title with an em dash in it is not an edge case here.
pub fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Collapse every run of the writer's own whitespace to one space, trim, and clip — so the result
/// can be written as one frontmatter line.
///
/// Splits on `pystr::is_python_space`, not Rust's `char::is_whitespace`, to agree with
/// `write::single_line_problem`'s notion of "one line" — which is built on `pystr::splitlines`,
/// and `splitlines` breaks on U+001C–U+001F as well as `char::is_whitespace`'s set.
/// **This is not Python fidelity**: `judge.rs` has no Python counterpart, so matching CPython is
/// not a goal here; it is agreeing with this crate's own writer predicate, which happens to be
/// built on `pystr::splitlines`. It does **not** guard `single_line_problem`'s other refusal
/// ("must not start a frontmatter block") — that does not matter on this plan's only path, because
/// Task 7 writes `importance_reason` through `write::to_literal`, which quotes it.
pub fn one_line(text: &str, max: usize) -> String {
    clip(
        &text
            .split(crate::pystr::is_python_space)
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        max,
    )
}

/// The `## <heading>` section of a markdown body, up to the next heading of any level, clipped.
fn section(text: &str, heading: &str, max: usize) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim() == heading {
            inside = true;
            continue;
        }
        if inside {
            if line.trim_start().starts_with('#') {
                break;
            }
            out.push(line);
        }
    }
    clip(out.join("\n").trim(), max)
}

impl Heuristics {
    /// Reads four things and **never fails**: a vault with no `courses/`, no `profile/` and no
    /// `course_map` still judges, on the model alone. Every path is derived from `vault` (R1).
    pub fn load(vault: &Path) -> Heuristics {
        let course_map: Vec<(String, String)> = crate::pystr::read_text(
            &vault.join("config").join("ingest.yaml"),
        )
        .ok()
        .and_then(|t| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&t).ok())
        .and_then(|v| v.get("course_map").and_then(|m| m.as_mapping()).cloned())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();

        let slice_hours =
            crate::planning::load_planning(&vault.join("config").join("planning.yaml")).slice_hours;

        let mut weights = BTreeMap::new();
        if let Ok(rd) = std::fs::read_dir(vault.join("courses")) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.extension().map(|x| !x.eq_ignore_ascii_case("md")).unwrap_or(true) {
                    continue;
                }
                let Ok(text) = crate::pystr::read_text(&path) else { continue };
                let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                    continue;
                };
                // Task notes carry the note's `slug:` field, not its filename — a stem that
                // disagrees with `slug:` would silently drop that course's attributions now that
                // the key set (not the weights text) is what `knows_course` tests. `load` must
                // stay infallible, so unparseable frontmatter just falls back to the stem.
                let slug = crate::models::split_frontmatter(&text)
                    .ok()
                    .and_then(|(meta, _)| crate::yaml::get(&meta, "slug").and_then(|v| v.as_str()).map(str::to_string))
                    .filter(|s| !s.is_empty())
                    .unwrap_or(stem);
                let w = section(&text, "## Grade weights", MAX_WEIGHTS_CHARS);
                weights.insert(slug, w);
            }
        }

        let preferences = crate::pystr::read_text(&vault.join("profile").join("preferences.md"))
            .map(|t| clip(t.trim(), MAX_PREFS_CHARS))
            .unwrap_or_default();

        Heuristics { course_map, slice_hours, weights, preferences }
    }

    /// Is this a slug the vault itself knows? Tests whether a course **note exists** for it (the
    /// `weights` key set), not whether that note has a `## Grade weights` section — the guard on a
    /// model that invents a slug with no note behind it at all.
    pub fn knows_course(&self, slug: &str) -> bool {
        self.weights.contains_key(slug) || self.course_map.iter().any(|(_, s)| s == slug)
    }
}

/// **Tier 1 — deterministic, in the engine's own terms.**
///
/// Two answers and no more:
///
/// - **course** — whatever the note already carries, else `ingest::match_course_fields` over the
///   `course_map`: a case-sensitive uid pin, then a bounded course code in the title. The same
///   rule the ICS ingest applies, reached through the same function.
/// - **effort_hours** — the note's own value when `effort_source: vendor`, because zyBooks and VHL
///   state it and nothing may re-state it (the rule `coursework::sync_coursework` already keeps).
///
/// **Importance is never a heuristic.** It is grounded in a course's grade weights, which is a
/// judgement, so tier 1 alone is never `complete()` — which is exactly what sends an item onward.
pub fn tier1(item: &Item, h: &Heuristics) -> Verdict {
    let course = match item.course.as_ref().map(|c| c.trim()).filter(|c| !c.is_empty()) {
        Some(c) => Some(c.to_string()),
        None => crate::ingest::match_course_fields(&item.source_uid, &item.title, &h.course_map),
    };
    let effort_hours = (item.effort_source == "vendor").then_some(item.effort_hours);
    Verdict { course, effort_hours, importance: None, importance_reason: None, confidence: 1.0, tier: 1 }
}

/// `lower` wins every field it answered; `upper` fills only the gaps. The tier and confidence come
/// from whichever actually contributed.
pub fn merge(lower: Verdict, upper: Verdict) -> Verdict {
    let contributed = (lower.course.is_none() && upper.course.is_some())
        || (lower.effort_hours.is_none() && upper.effort_hours.is_some())
        || (lower.importance.is_none() && upper.importance.is_some())
        || (lower.importance_reason.is_none() && upper.importance_reason.is_some());
    Verdict {
        course: lower.course.or(upper.course),
        effort_hours: lower.effort_hours.or(upper.effort_hours),
        importance: lower.importance.or(upper.importance),
        importance_reason: lower.importance_reason.or(upper.importance_reason),
        confidence: if contributed { upper.confidence } else { lower.confidence },
        tier: if contributed { upper.tier.max(lower.tier) } else { lower.tier },
    }
}

/// The seam. Tier 1, then tier 2, then — only if a gap is left, and only if there is one to reach —
/// tier 3.
///
/// `model` is `Ok(m)` or the reason there is none, resolved once per run rather than per item: a
/// hundred items must not each re-stat the disk for a model file that is not there.
pub fn judge_task(
    item: &Item,
    h: &Heuristics,
    rules: &dyn Rules,
    model: Result<&dyn Model, Missing>,
) -> Outcome {
    let mut seed = tier1(item, h);
    if seed.complete() {
        return Outcome::Answered(seed);
    }
    if let Some(rule) = rules.lookup(item) {
        seed = merge(seed, rule);
        if seed.complete() {
            return Outcome::Answered(seed);
        }
    }
    let model = match model {
        Err(Missing::Runtime) => return Outcome::RuntimeNotInstalled(seed),
        Err(Missing::Model) => return Outcome::ModelNotInstalled(seed),
        Ok(m) => m,
    };
    let mut answer = match model.judge(item, h, &seed) {
        Ok(a) => a,
        Err(e) => {
            return Outcome::LowConfidence { seed, why: e.to_string(), cause: LowCause::ModelFailed }
        }
    };
    if answer.confidence < CONFIDENCE_FLOOR {
        let why = format!("confidence {:.2} below {:.2}", answer.confidence, CONFIDENCE_FLOOR);
        return Outcome::LowConfidence { seed, why, cause: LowCause::BelowFloor };
    }
    // A slug the vault does not know would put the note in a group nothing renders and no course
    // note explains. Dropped rather than refused: the effort and importance answers are still good.
    if let Some(c) = &answer.course {
        if !h.knows_course(c) {
            answer.course = None;
        }
    }
    let merged = merge(seed.clone(), answer);
    if merged.complete() {
        Outcome::Answered(merged)
    } else {
        Outcome::LowConfidence {
            seed,
            why: "the model left a required field empty".to_string(),
            cause: LowCause::Incomplete,
        }
    }
}

/// The GBNF grammar every model call is constrained by (D6, product plan §2.3: grammar-constrained
/// decoding on every call without exception).
///
/// **Five fields, and the grammar is what enforces four of the five rules.** `importance` is an
/// alternation of the five literals rather than a number the prompt asks to be in range, so a 7 is
/// not merely unlikely but unrepresentable. `course` is a string or the literal `null`, so "I could
/// not tell" has a spelling. `confidence` is the field the tiers key on and is generated last, after
/// the answer it is about.
///
/// Written as a raw string with real newlines: llama.cpp's parser wants one rule per line, and a
/// grammar that fails to parse fails every judgment identically and silently.
///
/// **Not in here:** `effort_source` (the engine sets it), `effort_confidence` (always `low`, as the
/// routine writes it) and `needs_enrichment` (the pass's decision about its own completeness, never
/// the model's opinion of it).
pub const GRAMMAR: &str = r#"root ::= "{" ws "\"course\":" ws course "," ws "\"effort_hours\":" ws number "," ws "\"importance\":" ws importance "," ws "\"importance_reason\":" ws string "," ws "\"confidence\":" ws number ws "}"
course ::= "null" | string
importance ::= "1" | "2" | "3" | "4" | "5"
string ::= "\"" ([^"\\] | "\\" ["\\/bfnrt])* "\""
number ::= [0-9]+ ("." [0-9]+)?
ws ::= " "?
"#;

/// What tier 3 is asked. Built from the note, the course's grade weights and
/// `profile/preferences.md`, and from nothing else (R6).
///
/// It is the cloud routine's step 3, written down: attribute the course if you can, estimate a
/// realistic effort, ground importance in the grade weights, default to 3 and say so when there are
/// none, and write one line of reason. The differences from the routine are that the rules are
/// stated to a 1–4 B model rather than to Claude, and that the output shape is a grammar rather
/// than a command line.
///
/// **Every free-text input is clipped HERE, at the point of use** — `MAX_BODY_CHARS`,
/// `MAX_WEIGHTS_CHARS`, `MAX_PREFS_CHARS` — and not only inside `Heuristics::load`. Clipping in
/// `load` alone would make this function's bound a property of one caller rather than of the
/// function: a test, or any future caller that builds a `Heuristics` by hand, would get a prompt of
/// whatever size the vault happened to hold. Clipped here, a note carrying a whole syllabus can
/// never push the question out of a 4096-token context, whoever assembled the inputs.
pub fn prompt_for(item: &Item, h: &Heuristics, seed: &Verdict) -> String {
    let slug = seed.course.as_deref().unwrap_or("null");
    let weights = seed.course.as_deref().and_then(|c| h.weights.get(c)).map(String::as_str).unwrap_or("");
    let mut p = String::new();
    p.push_str("You estimate effort and importance for one university assignment.\n");
    p.push_str("Answer with the JSON object and nothing else.\n\n");
    p.push_str("Rules:\n");
    p.push_str("- effort_hours: the realistic time in hours to finish this one assignment, 0.25 to 40.\n");
    p.push_str(&format!("- a typical work session in this student's plan is {} hours.\n", h.slice_hours));
    p.push_str("- importance: 1 to 5, grounded in the grade weights below. With no weights given, answer 3 and say so in importance_reason.\n");
    p.push_str("- importance_reason: one line, under 140 characters, no line breaks.\n");
    p.push_str("- course: the slug given below, or null when the slug given is null.\n");
    p.push_str("- confidence: 0 to 1, how sure you are of effort_hours and importance.\n\n");
    if !h.preferences.is_empty() {
        p.push_str("The student's stated preferences:\n");
        p.push_str(&clip(&h.preferences, MAX_PREFS_CHARS));
        p.push_str("\n\n");
    }
    p.push_str(&format!("Course slug: {slug}\n"));
    if !weights.is_empty() {
        p.push_str("Grade weights:\n");
        p.push_str(&clip(weights, MAX_WEIGHTS_CHARS));
        p.push('\n');
    }
    p.push_str(&format!("Title: {}\n", one_line(&item.title, 200)));
    // An absent due date is absent, never the word `None`: a model shown "Due: None" reliably
    // treats it as a date it failed to read rather than as an assignment without one.
    if let Some(due) = item.due.as_deref().filter(|d| !d.is_empty()) {
        p.push_str(&format!("Due: {due}\n"));
    }
    let body = clip(item.body.trim(), MAX_BODY_CHARS);
    if !body.is_empty() {
        p.push_str("Body:\n");
        p.push_str(&body);
        p.push('\n');
    }
    p.push_str("\nJSON:");
    p
}

/// Find the answer inside whatever the runtime wrote, and read it back into a `Verdict`.
///
/// **Under outcome B this is not a clean response body.** `llama-cli` echoes the prompt — which
/// carries the note's own title and body, arbitrary text — to stdout ahead of the completion, and
/// may print more after it. So neither the first `{` nor the last is a reliable anchor: the first
/// can be a brace sitting in the note's own text, and the last can be a brace **inside a string
/// value**, which `GRAMMAR`'s `string` rule explicitly permits (a model answering
/// `"importance_reason": "worth {5} points"` defeats a `rfind('{')`).
///
/// So this does not anchor on a brace at all. It tries every `{` in the text, left to right; at
/// each one it parses the first complete JSON value that starts there (trailing text — the rest of
/// the echo, or anything the runtime prints afterwards — is simply not consumed and so never
/// matters) and accepts the first one that is an object carrying both `effort_hours` and
/// `importance`. A `{"a": 1}` sitting in the echoed prompt is syntactically a JSON object but is
/// missing those fields, so it is skipped rather than mistaken for the answer; a JSON-shaped but
/// invalid fragment such as `{TBD}` fails to parse at all and is skipped the same way.
///
/// **Tolerant about the envelope, strict about the contents.** Once a candidate is accepted,
/// `effort_hours` and `importance` are required (a verdict without them answers nothing), the two
/// numbers are **clamped rather than refused** (a 900-hour estimate is a bad answer, not a broken
/// one, and clamping keeps the confidence floor as the single place a bad answer is rejected), and
/// the reason is collapsed to one clipped line so `write`'s single-line guard can never refuse it.
///
/// A blank `course` or a blank reason is `None`, not `Some("")`: an empty string would be written
/// into frontmatter and would then read as "attributed to nothing", which is a different claim.
pub fn parse_reply(text: &str) -> Result<Verdict, ModelError> {
    let bad = |m: &str| ModelError::Failed(format!("the model's reply {m}"));
    // Left to right, never `.rev()`. Right-to-left looks like the obvious speed-up — the real
    // answer is always near the end, after the echoed prompt, so scanning backwards would usually
    // find it in one or two candidates instead of walking past the whole prompt first — and it is
    // wrong in a way that is not obvious: an `importance_reason` can itself contain a JSON-looking
    // fragment that carries our own field names, e.g. `"like {\"effort_hours\": 9, \"importance\":
    // 5}"`. Read right to left, that inner fragment's own `{` is found first, parses cleanly, passes
    // the required-field check, and is returned — silently replacing the real answer with a quoted
    // example inside it. Read left to right, the true object's own opening brace is always reached
    // first, so that case is harmless.
    //
    // The scan is quadratic in principle (`O(n)` candidate braces, each re-parsed from scratch), but
    // bounded in practice by the prompt clipping (`MAX_BODY_CHARS` and friends) on one end and the
    // runtime's 1 MiB stdout cap on the other, and left deliberately unbounded rather than capped at
    // some small candidate count: a candidate cap would be a false-negative risk on exactly this
    // product's own users — a CS assignment body that pastes code with sixty-odd braces in it
    // before the real answer ever appears.
    let value = text
        .char_indices()
        .filter(|&(_, c)| c == '{')
        .find_map(|(i, _)| {
            let mut de = serde_json::Deserializer::from_str(&text[i..]).into_iter::<serde_json::Value>();
            let v = de.next()?.ok()?;
            let obj = v.as_object()?;
            (obj.contains_key("effort_hours") && obj.contains_key("importance")).then_some(v)
        })
        .ok_or_else(|| bad("carried no JSON object with the expected fields"))?;

    let effort = value
        .get("effort_hours")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| bad("has no effort_hours"))?;
    let importance = value
        .get("importance")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| bad("has no importance"))?;
    let course = value
        .get("course")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let reason = value.get("importance_reason").and_then(serde_json::Value::as_str).unwrap_or("");
    let reason = one_line(reason, MAX_REASON_CHARS);
    let confidence = value.get("confidence").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    Ok(Verdict {
        course,
        effort_hours: Some(effort.clamp(0.25, 40.0)),
        importance: Some(importance.clamp(1, 5)),
        importance_reason: (!reason.is_empty()).then_some(reason),
        confidence: confidence.clamp(0.0, 1.0),
        tier: 3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str) -> Item {
        Item {
            id: "task_0123456789".to_string(),
            rel_path: "tasks/x.md".to_string(),
            title: title.to_string(),
            body: "Submit through the course page.".to_string(),
            source_uid: String::new(),
            created_by: "blackboard".to_string(),
            course: None,
            due: Some("2026-10-01T23:59".to_string()),
            effort_hours: 1.0,
            effort_source: "inferred".to_string(),
        }
    }

    fn heur() -> Heuristics {
        Heuristics {
            course_map: vec![
                ("CS-100".to_string(), "cs-100".to_string()),
                ("_884411_1".to_string(), "ph-106".to_string()),
            ],
            slice_hours: 2.0,
            weights: std::collections::BTreeMap::new(),
            preferences: String::new(),
        }
    }

    /// A scripted model: one canned verdict, and a record of what it was asked. Nothing in this
    /// crate's tests ever starts a process (spec §8).
    struct Scripted {
        reply: std::sync::Mutex<Result<Verdict, ModelError>>,
        calls: std::sync::Mutex<usize>,
    }
    impl Scripted {
        fn ok(v: Verdict) -> Scripted {
            Scripted { reply: std::sync::Mutex::new(Ok(v)), calls: std::sync::Mutex::new(0) }
        }
        fn err(why: &str) -> Scripted {
            Scripted {
                reply: std::sync::Mutex::new(Err(ModelError::Failed(why.to_string()))),
                calls: std::sync::Mutex::new(0),
            }
        }
        fn calls(&self) -> usize { *self.calls.lock().unwrap() }
    }
    impl Model for Scripted {
        fn judge(&self, _i: &Item, _h: &Heuristics, _s: &Verdict) -> Result<Verdict, ModelError> {
            *self.calls.lock().unwrap() += 1;
            self.reply.lock().unwrap().clone()
        }
    }

    fn full(conf: f64) -> Verdict {
        Verdict {
            course: None,
            effort_hours: Some(2.5),
            importance: Some(4),
            importance_reason: Some("Worth 15% of the grade.".to_string()),
            confidence: conf,
            tier: 3,
        }
    }

    /// The uid pin wins over the course code, exactly as `ingest::match_course` orders them: a
    /// gradebook item carries no course code at all, and the pin is the only thing that attributes
    /// it.
    #[test]
    fn tier_one_attributes_a_course_by_uid_pin_then_by_code() {
        let h = heur();
        let mut i = item("Homework 3");
        i.source_uid = "blackboard:_884411_1".to_string();
        assert_eq!(tier1(&i, &h).course.as_deref(), Some("ph-106"));

        let mut i = item("CS-100 Homework 3");
        i.source_uid = "blackboard:nothing".to_string();
        assert_eq!(tier1(&i, &h).course.as_deref(), Some("cs-100"));

        // The boundary test survives the move: CS-100 must not match inside a longer token.
        let mut i = item("STATISTICS-1000 reading");
        i.source_uid = "blackboard:nothing".to_string();
        assert_eq!(tier1(&i, &h).course, None);
    }

    /// A note that already names a course is never re-attributed — enrichment fills gaps, it does
    /// not second-guess what is there.
    #[test]
    fn tier_one_keeps_a_course_the_note_already_carries() {
        let mut i = item("CS-100 Homework 3");
        i.course = Some("gn-103".to_string());
        assert_eq!(tier1(&i, &heur()).course.as_deref(), Some("gn-103"));
    }

    /// `effort_source: vendor` means zyBooks or VHL stated the effort. Tier 1 answers it and the
    /// model is never asked — the same rule `coursework::sync_coursework` applies when it declines
    /// to overwrite a non-vendor estimate.
    #[test]
    fn tier_one_answers_effort_only_when_the_vendor_stated_it() {
        let mut i = item("zyBooks 4.2");
        i.effort_source = "vendor".to_string();
        i.effort_hours = 0.75;
        assert_eq!(tier1(&i, &heur()).effort_hours, Some(0.75));
        assert_eq!(tier1(&item("Homework 3"), &heur()).effort_hours, None);
    }

    /// Importance is never a heuristic: it needs the course's grade weights, which is judgment.
    /// So tier 1 alone is never complete, and that is what sends an item to the model.
    #[test]
    fn tier_one_never_completes_on_its_own() {
        let mut i = item("zyBooks 4.2");
        i.effort_source = "vendor".to_string();
        i.course = Some("cs-100".to_string());
        let v = tier1(&i, &heur());
        assert_eq!(v.importance, None);
        assert!(!v.complete(), "importance is judgment, not a heuristic");
    }

    #[test]
    fn the_model_is_asked_only_when_the_tiers_above_it_left_a_gap() {
        let m = Scripted::ok(full(0.9));
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert!(matches!(out, Outcome::Answered(_)), "{out:?}");
        assert_eq!(out.label(), "answered");
        assert_eq!(m.calls(), 1);
        assert_eq!(out.verdict().tier, 3);
        assert_eq!(out.verdict().effort_hours, Some(2.5));
    }

    /// D7: not installed is a normal outcome, and it still carries whatever tier 1 managed — a
    /// course attributed from the map is worth writing even with no model on the machine.
    #[test]
    fn a_missing_runtime_or_model_is_an_outcome_that_still_carries_tier_one() {
        let mut i = item("CS-100 Homework 3");
        i.source_uid = "blackboard:nothing".to_string();
        let r = judge_task(&i, &heur(), &NoRules, Err(Missing::Runtime));
        assert!(matches!(r, Outcome::RuntimeNotInstalled(_)), "{r:?}");
        assert_eq!(r.verdict().course.as_deref(), Some("cs-100"));
        assert_eq!(r.label(), "runtime not installed");
        let m = judge_task(&i, &heur(), &NoRules, Err(Missing::Model));
        assert!(matches!(m, Outcome::ModelNotInstalled(_)), "{m:?}");
        assert_eq!(m.verdict().course.as_deref(), Some("cs-100"));
        assert_eq!(m.label(), "model not installed");
    }

    /// The floor is the whole point of tier 3 being conditional: an unsure answer is not written,
    /// and the reason says which number failed.
    #[test]
    fn an_answer_under_the_floor_is_refused_and_says_why() {
        let m = Scripted::ok(full(CONFIDENCE_FLOOR - 0.01));
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert_eq!(out.label(), "low confidence");
        match out {
            Outcome::LowConfidence { seed, why, cause } => {
                assert_eq!(seed.tier, 1, "the model's rejected answer is never merged in");
                assert!(seed.effort_hours.is_none(), "nothing of the refused answer survives");
                assert_eq!(why, "confidence 0.59 below 0.60");
                assert_eq!(cause, LowCause::BelowFloor);
            }
            other => panic!("{other:?}"),
        }
    }

    /// m7: this branch is what stops Task 7 writing a partial answer — a model that answers with
    /// confidence above the floor but leaves a required field empty is still refused.
    #[test]
    fn a_model_answer_missing_a_required_field_is_refused_with_its_own_reason() {
        let mut v = full(0.9);
        v.importance = None;
        let m = Scripted::ok(v);
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        match out {
            Outcome::LowConfidence { why, cause, .. } => {
                assert_eq!(why, "the model left a required field empty");
                assert_eq!(cause, LowCause::Incomplete);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A transport failure is not a low-confidence answer, and the outcome must not pretend it is:
    /// same variant, different `why`, so "failures visible, silence never ambiguous" holds.
    #[test]
    fn a_model_error_is_reported_as_itself() {
        let m = Scripted::err("llama-cli: connection refused");
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        match out {
            Outcome::LowConfidence { why, cause, .. } => {
                assert!(why.contains("connection refused"), "{why}");
                assert_eq!(cause, LowCause::ModelFailed);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A course the model invented is dropped. A slug that is not in the vault's own course map
    /// would send the note into a group `rank` cannot render and no course note explains.
    #[test]
    fn a_course_slug_the_vault_does_not_know_is_dropped() {
        let mut v = full(0.9);
        v.course = Some("phys-999".to_string());
        let m = Scripted::ok(v);
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert_eq!(out.verdict().course, None, "an unknown slug never reaches a note");

        let mut v = full(0.9);
        v.course = Some("cs-100".to_string());
        let m = Scripted::ok(v);
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert_eq!(out.verdict().course.as_deref(), Some("cs-100"));
    }

    /// D10: tier 2 is 3b's, and its seam is real — a `Rules` that answers stops the model being
    /// asked at all, which is the whole economic point of promotion.
    #[test]
    fn a_rule_that_answers_stops_the_model_being_asked() {
        struct Always;
        impl Rules for Always {
            fn lookup(&self, _i: &Item) -> Option<Verdict> {
                Some(Verdict {
                    course: Some("cs-100".to_string()),
                    effort_hours: Some(1.5),
                    importance: Some(3),
                    importance_reason: Some("Promoted rule: weekly homework.".to_string()),
                    confidence: 1.0,
                    tier: 2,
                })
            }
        }
        let m = Scripted::ok(full(0.9));
        let out = judge_task(&item("Homework 3"), &heur(), &Always, Ok(&m));
        assert!(matches!(out, Outcome::Answered(_)), "{out:?}");
        assert_eq!(m.calls(), 0, "a promoted rule means the model is not called");
        assert_eq!(out.verdict().tier, 2);
        assert!(NoRules.lookup(&item("x")).is_none(), "this plan promotes nothing");
    }

    /// `Heuristics::load` reads four things and never fails: a vault with none of them still
    /// judges (R1 — nothing here assumes a particular vault's shape).
    #[test]
    fn heuristics_load_reads_the_vault_and_survives_an_empty_one() {
        let v = std::env::temp_dir().join(format!("qo-judge-heur-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::create_dir_all(v.join("courses")).unwrap();
        std::fs::create_dir_all(v.join("profile")).unwrap();
        let empty = Heuristics::load(&v);
        assert!(empty.course_map.is_empty());
        assert_eq!(empty.slice_hours, 2.0, "planning's own default");

        crate::pystr::write_text(
            &v.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100\n",
        ).unwrap();
        crate::pystr::write_text(&v.join("config").join("planning.yaml"), "slice_hours: 1.5\n").unwrap();
        crate::pystr::write_text(
            &v.join("courses").join("cs-100.md"),
            "---\ntitle: CS 100\n---\n\n## Grade weights\n- Homework: 20%\n\n## Policies\n- No late work.\n",
        ).unwrap();
        crate::pystr::write_text(&v.join("profile").join("preferences.md"), "- German is daily.\n").unwrap();
        let h = Heuristics::load(&v);
        assert_eq!(h.course_map, vec![("CS-100".to_string(), "cs-100".to_string())]);
        assert_eq!(h.slice_hours, 1.5);
        let w = h.weights.get("cs-100").expect("cs-100 weights");
        assert!(w.contains("Homework: 20%"), "{w}");
        assert!(!w.contains("No late work"), "the section stops at the next heading: {w}");
        assert!(h.preferences.contains("German is daily"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// M1: a course note with no `## Grade weights` section is still a known course. The friends
    /// wizard writes an empty `courses/` and no course notes at all, but a hand-written course
    /// note that simply hasn't gotten a weights section yet must not silently drop the course's
    /// attributions — `knows_course` tests note existence, not section content.
    #[test]
    fn a_course_note_with_no_weights_section_is_still_a_known_course() {
        let v = std::env::temp_dir().join(format!("qo-judge-noweights-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::create_dir_all(v.join("courses")).unwrap();
        crate::pystr::write_text(&v.join("config").join("ingest.yaml"), "course_map: {}\n").unwrap();
        crate::pystr::write_text(
            &v.join("courses").join("cs-100.md"),
            "---\ntitle: CS 100\n---\n\nJust a course note, no weights section yet.\n",
        ).unwrap();
        let h = Heuristics::load(&v);
        assert!(h.knows_course("cs-100"));
        assert_eq!(h.weights.get("cs-100").map(String::as_str), Some(""));

        let mut answer = full(0.9);
        answer.course = Some("cs-100".to_string());
        let m = Scripted::ok(answer);
        let out = judge_task(&item("Homework 3"), &h, &NoRules, Ok(&m));
        assert!(matches!(out, Outcome::Answered(_)), "{out:?}");
        assert_eq!(out.verdict().course.as_deref(), Some("cs-100"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// m5: task notes carry the note's `slug:` field, not its filename, so the weights map keys on
    /// `slug:` when present and falls back to the file stem only when it is absent.
    #[test]
    fn weights_are_keyed_by_the_notes_own_slug_not_the_filename() {
        let v = std::env::temp_dir().join(format!("qo-judge-slug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::create_dir_all(v.join("courses")).unwrap();
        crate::pystr::write_text(&v.join("config").join("ingest.yaml"), "course_map: {}\n").unwrap();
        crate::pystr::write_text(
            &v.join("courses").join("cs100.md"),
            "---\ntitle: CS 100\nslug: cs-100\n---\n\n## Grade weights\n- Homework: 20%\n",
        ).unwrap();
        let h = Heuristics::load(&v);
        assert!(h.knows_course("cs-100"), "keyed by slug:, not the stem");
        assert!(!h.knows_course("cs100"), "the stem must not also be a key");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// m9: `section` must stop at a heading of any level, not only `## `, or a `# Policies`
    /// heading after `## Grade weights` leaks unrelated prose into the prompt.
    #[test]
    fn section_stops_at_a_heading_of_any_level() {
        assert_eq!(
            section(
                "## Grade weights\n- Homework: 20%\n# Policies\n- No late work.\n",
                "## Grade weights",
                600,
            ),
            "- Homework: 20%",
        );
    }

    /// `clip` and `one_line` bound what reaches a prompt and a frontmatter line. `clip` must be
    /// char-safe: slicing a multi-byte character in half panics. `one_line` must agree with
    /// `write::single_line_problem` on characters Rust's `char::is_whitespace` does not treat as
    /// whitespace but `pystr::splitlines` still breaks a line on (M2).
    #[test]
    fn clip_is_char_safe_and_one_line_is_single_line() {
        assert_eq!(clip("abcdef", 3), "abc");
        assert_eq!(clip("abc", 10), "abc");
        assert_eq!(clip("é".repeat(10).as_str(), 3), "ééé");
        let messy = "  two\nlines\tand   spaces  ";
        assert_eq!(one_line(messy, 100), "two lines and spaces");
        assert_eq!(one_line("abcdef", 4), "abcd");
        assert!(crate::write::single_line_problem(&one_line(messy, 100)).is_none());
        assert_eq!(one_line("a\u{1c}b", 100), "a b");
        assert!(crate::write::single_line_problem(&one_line("a\u{1c}b", 100)).is_none());
    }

    /// D6: grammar-constrained decoding on every call without exception. The grammar has to name
    /// all five fields, in the order the emitter will produce them, and it must be plain GBNF —
    /// llama-cli rejects a malformed rule set and every judgment then fails identically.
    #[test]
    fn the_grammar_names_the_five_fields_and_only_those() {
        for f in ["course", "effort_hours", "importance", "importance_reason", "confidence"] {
            assert!(GRAMMAR.contains(&format!("\\\"{f}\\\"")), "grammar must pin {f}: {GRAMMAR}");
        }
        assert!(GRAMMAR.starts_with("root ::= "), "GBNF starts at root");
        assert!(GRAMMAR.contains("importance ::= \"1\" | \"2\" | \"3\" | \"4\" | \"5\""), "1-5, in the grammar, not in a prompt");
        assert!(!GRAMMAR.contains("effort_source"), "effort_source is the engine's, never the model's");
        assert!(!GRAMMAR.contains("needs_enrichment"), "the flag is the pass's decision, not the model's");
    }

    /// R6: nothing but the note, the course's weights and the preferences reaches the model. In
    /// particular no config file, no path, no credential target.
    #[test]
    fn the_prompt_carries_the_note_and_its_grounding_and_nothing_else() {
        let mut h = heur();
        h.weights.insert("cs-100".to_string(), "- Homework: 20% of the grade.".to_string());
        h.preferences = "- German is near-daily small work.".to_string();
        let mut i = item("CS-100 Homework 3");
        i.body = "Chapters 4 and 5. Submit on the course page.".to_string();
        i.source_uid = "blackboard:nothing".to_string();
        let seed = tier1(&i, &h);
        let p = prompt_for(&i, &h, &seed);

        assert!(p.contains("CS-100 Homework 3"), "the title");
        assert!(p.contains("Chapters 4 and 5"), "the body");
        assert!(p.contains("2026-10-01T23:59"), "the due date");
        assert!(p.contains("cs-100"), "the resolved slug");
        assert!(p.contains("Homework: 20% of the grade."), "the grade weights ground importance");
        assert!(p.contains("German is near-daily"), "preferences are honoured, as the routine honours them");
        assert!(p.contains("2 hours"), "the vault's own session length anchors the estimate");
        assert!(!p.contains("ingest.yaml") && !p.contains("credential"), "R6");
        assert!(p.ends_with("JSON:"), "the model is asked for the object and nothing else");
    }

    /// A note whose body is a whole syllabus must not push the question out of the context window.
    #[test]
    fn the_prompt_is_bounded_however_long_the_note_is() {
        let mut i = item("Long one");
        i.body = "x".repeat(50_000);
        let mut h = heur();
        h.preferences = "p".repeat(50_000);
        h.weights.insert("cs-100".to_string(), "w".repeat(50_000));
        i.course = Some("cs-100".to_string());
        let p = prompt_for(&i, &h, &tier1(&i, &h));
        assert!(p.len() < 6_000, "prompt was {} chars", p.len());
        assert!(p.contains("Long one"), "the title survives the clipping");
    }

    /// A course with no note, and a note with no due date and no body: the prompt still stands up.
    #[test]
    fn the_prompt_survives_a_note_with_nothing_in_it() {
        let mut i = item("Untitled");
        i.due = None;
        i.body = String::new();
        i.source_uid = "blackboard:nothing".to_string();
        let h = Heuristics {
            course_map: Vec::new(),
            slice_hours: 2.0,
            weights: std::collections::BTreeMap::new(),
            preferences: String::new(),
        };
        let p = prompt_for(&i, &h, &tier1(&i, &h));
        assert!(p.contains("Course slug: null"));
        assert!(!p.contains("Due:"), "an absent due date is absent, never the word None");
        assert!(p.ends_with("JSON:"));
    }

    #[test]
    fn parse_reply_reads_the_object_clamps_it_and_makes_the_reason_one_line() {
        let v = parse_reply(
            r#" here you go {"course": "cs-100", "effort_hours": 2.5, "importance": 4, "importance_reason": "Worth  15%\nof the grade.", "confidence": 0.82} thanks"#,
        )
        .expect("an object anywhere in the reply");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(2.5));
        assert_eq!(v.importance, Some(4));
        assert_eq!(v.importance_reason.as_deref(), Some("Worth 15% of the grade."));
        assert_eq!(v.confidence, 0.82);
        assert_eq!(v.tier, 3);

        // Clamped, never refused: the grammar bounds importance, and effort and confidence are
        // bounded here so a runaway number cannot reach a note or a ranking.
        let v = parse_reply(r#"{"course": null, "effort_hours": 900, "importance": 9, "importance_reason": "x", "confidence": 5}"#).unwrap();
        assert_eq!(v.effort_hours, Some(40.0));
        assert_eq!(v.importance, Some(5));
        assert_eq!(v.confidence, 1.0);
        assert_eq!(v.course, None);

        let v = parse_reply(r#"{"course": "  ", "effort_hours": 0.01, "importance": 0, "importance_reason": "   ", "confidence": -1}"#).unwrap();
        assert_eq!(v.course, None, "a blank slug is no slug");
        assert_eq!(v.effort_hours, Some(0.25));
        assert_eq!(v.importance, Some(1));
        assert_eq!(v.importance_reason, None, "a blank reason is no reason, so the verdict is incomplete");
        assert_eq!(v.confidence, 0.0);
    }

    #[test]
    fn parse_reply_refuses_what_it_cannot_read() {
        assert!(parse_reply("no object here").is_err());
        assert!(parse_reply("{not json}").is_err());
        assert!(parse_reply(r#"{"importance": 3, "importance_reason": "x", "confidence": 1}"#).is_err(), "effort_hours is required");
        assert!(parse_reply(r#"{"effort_hours": 2, "importance_reason": "x", "confidence": 1}"#).is_err(), "importance is required");
    }

    /// The reason becomes a frontmatter line, so it has to survive the write path's own guard.
    #[test]
    fn a_generated_reason_is_always_writable_as_one_line() {
        let v = parse_reply(&format!(
            r#"{{"course": null, "effort_hours": 1, "importance": 3, "importance_reason": "{}", "confidence": 1}}"#,
            "long ".repeat(200)
        ))
        .unwrap();
        let reason = v.importance_reason.unwrap();
        assert!(reason.chars().count() <= MAX_REASON_CHARS);
        assert!(crate::write::single_line_problem(&reason).is_none(), "{reason}");
    }

    /// R-3a-7: `llama-cli` echoes the prompt before the completion, and a note's own text can
    /// contain a brace that means nothing. A `{TBD}` sitting in the echoed body must not be
    /// mistaken for the start of the answer.
    #[test]
    fn parse_reply_skips_a_brace_in_the_echoed_prompt_that_is_not_json_at_all() {
        let text = r#"Title: Homework 3
Body:
Fill in the blank: {TBD} before Friday.

JSON:{"course": "cs-100", "effort_hours": 1.5, "importance": 3, "importance_reason": "Weekly homework.", "confidence": 0.75}"#;
        let v = parse_reply(text).expect("the real object, past the noise");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(1.5));
    }

    /// The field check is what does the work here: a scanner that accepted the first *parseable*
    /// object, rather than the first one with the required fields, would return this one instead
    /// of the real answer.
    #[test]
    fn parse_reply_skips_a_valid_but_incomplete_object_before_the_answer() {
        let text = r#"Body:
See the syllabus, section {"a": 1} for details.

JSON:{"course": "cs-100", "effort_hours": 1.5, "importance": 3, "importance_reason": "Weekly homework.", "confidence": 0.75}"#;
        let v = parse_reply(text).expect("the object with the required fields");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(1.5));
    }

    /// The case that would have broken an anchor on the LAST `{`: `GRAMMAR`'s `string` rule
    /// permits a brace inside a string value, so `importance_reason` can legally contain one.
    #[test]
    fn parse_reply_reads_an_answer_whose_own_reason_contains_braces() {
        let v = parse_reply(
            r#"{"course": "cs-100", "effort_hours": 1.5, "importance": 3, "importance_reason": "worth {5} points", "confidence": 0.75}"#,
        )
        .expect("the brace inside the string must not defeat the scan");
        assert_eq!(v.importance_reason.as_deref(), Some("worth {5} points"));
    }

    /// Whatever the runtime prints after the completion — end-of-text markers, timing lines — is
    /// simply never consumed once the object itself parses.
    #[test]
    fn parse_reply_ignores_trailing_text_after_the_object() {
        let text = r#"{"course": "cs-100", "effort_hours": 1.5, "importance": 3, "importance_reason": "Weekly homework.", "confidence": 0.75}
[end of text]
"#;
        let v = parse_reply(text).expect("trailing text must not matter");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(1.5));
    }

    /// The scan walks `char_indices()`, never a raw byte offset, so a multi-byte character sitting
    /// directly beside a candidate `{` must not panic — it must simply make that candidate fail to
    /// parse (as intended, since it carries no JSON at all) and let the scan continue to the real
    /// object. An em dash immediately before the brace and an accented word immediately after it
    /// are realistic in a note's own body text, and a byte-offset regression here would panic
    /// mid-character rather than merely return the wrong answer.
    #[test]
    fn parse_reply_survives_a_multibyte_character_beside_a_candidate_brace() {
        let text = "Body:\nRead the reading—{étude} before Friday.\n\nJSON:{\"course\": \"cs-100\", \"effort_hours\": 1.5, \"importance\": 3, \"importance_reason\": \"Weekly homework.\", \"confidence\": 0.75}";
        let v = parse_reply(text).expect("the multi-byte neighbours must not panic the scan");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(1.5));
    }
}
