//! Completion detection, tier 1: a vendor's own number says a task is finished, and Knowlu proposes
//! `status: done` (stream J Task T8; design note `docs/notes/2026-09-22-completion-detection-design.md`
//! §2, §3, §7a).
//!
//! **Deterministic, no model, no new write path.** The output is an ordinary `kind: amend` approval
//! moving `status` from `active` to `done` — `status` is one of the nine amendable fields
//! (`approvals::AMENDABLE_FIELDS`), so the card is applied, capped (15 a day, overflow snoozed by
//! `approvals::defer_over_budget` on the next `rank`), surfaced and archived by machinery that
//! already exists. Nothing here ever writes a task note; it only mints approval cards through
//! [`crate::write::create`], journal first.
//!
//! The rules [`propose_done`] enforces, in order:
//!
//! 1. only a note under `tasks/` (never `archive/`) is a candidate;
//! 2. only a note whose frontmatter says `status: active` — a task already done or archived is left
//!    alone, and a note with no `status:` line is skipped because an amend `from: active` could
//!    never validate against it;
//! 3. a note whose `status` the journal shows the student set by hand (`op: set`) is left alone —
//!    they have already taken a position on whether it is finished;
//! 4. **one proposal per task, ever**: any `kind: amend` card for this target that moves `status`
//!    to `done` — pending, snoozed, approved, rejected, executed, in `approvals/` or `archive/` —
//!    means the question has been asked. A rejection therefore sticks: `process_approvals` moves a
//!    rejected card to `archive/` keeping its frontmatter, and this scan reads `archive/` too.
//!    That is the same shape `coursework::asked_map_keys` gives coursework-map cards.
//!
//! [`propose_done`] is the one public entry point a second evidence source (T9's email receipts)
//! calls; [`propose_vendor_completions`] is the tier-1 caller that maps vendor uids to task notes.

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use serde_json::{Map, Value as Json};
use serde_yaml_ng::{Mapping, Value};

use crate::journal::Journal;
use crate::write::{WriteContext, WriteError};

/// The actor every completion card is filed under. `agent:` prefix, so `provenance::is_agent` holds.
pub const ACTOR: &str = "agent:knowlu.completion";

/// One vendor-reported completion figure, keyed by the same `source_uid` the vendor's parser gives
/// the task note (`zybooks:<assignment_id>`, `vhl:<section_id>:<YYYY-MM-DD>`).
///
/// `earned` and `possible` are points for zyBooks and percentage points out of `100` for VHL.
#[derive(Debug, Clone, PartialEq)]
pub struct VendorCompletion {
    pub source: String,
    pub uid: String,
    pub earned: f64,
    pub possible: f64,
}

impl VendorCompletion {
    /// Tier 1 fires at **exactly 100%** and nothing below: the 80–99% band is ambiguous residue for
    /// a later tier (design note §7a). `possible == 0` is never complete — an assignment with no
    /// point-bearing section certifies nothing.
    pub fn is_complete(&self) -> bool {
        self.possible > 0.0 && self.earned >= self.possible
    }

    /// Whole percent, floored, so 99.6% reads as 99 and never as a rounded-up 100.
    pub fn percent(&self) -> i64 {
        if self.possible <= 0.0 {
            return 0;
        }
        (self.earned / self.possible * 100.0).floor() as i64
    }

    /// The evidence a card carries for this figure.
    pub fn evidence(&self) -> Evidence {
        let summary = match self.source.as_str() {
            "zybooks" => format!(
                "zyBooks reports {} of {} points earned ({}%)",
                number_text(self.earned),
                number_text(self.possible),
                self.percent()
            ),
            "vhl" => format!("VHL reports this assignment bucket {}% complete", self.percent()),
            other => format!("{other} reports {}% complete", self.percent()),
        };
        let mut detail = Map::new();
        detail.insert("uid".to_string(), Json::String(self.uid.clone()));
        detail.insert("percent".to_string(), Json::from(self.percent()));
        detail.insert("earned".to_string(), json_number(self.earned));
        detail.insert("possible".to_string(), json_number(self.possible));
        Evidence { source: self.source.clone(), summary, detail }
    }
}

/// `193` for a whole number, `12.5` otherwise — the figure as a person would write it.
fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn json_number(value: f64) -> Json {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        Json::from(value as i64)
    } else {
        serde_json::Number::from_f64(value).map(Json::Number).unwrap_or(Json::Null)
    }
}

/// Why a task looks finished. `source` names where the evidence came from (`zybooks`, `vhl`, and
/// for T9 an email source); `summary` is the one human sentence the card shows; `detail` is the
/// machine-readable rest. The journal's `evidence` on the card's `create` record is `detail` with
/// `source` added.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub source: String,
    pub summary: String,
    pub detail: Map<String, Json>,
}

impl Evidence {
    /// `detail` plus `source`, as it is journaled and quoted on the card.
    pub fn to_json(&self) -> Json {
        let mut map = self.detail.clone();
        map.insert("source".to_string(), Json::String(self.source.clone()));
        Json::Object(map)
    }
}

/// What [`propose_done`] did.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// A card was filed at this path.
    Proposed(PathBuf),
    /// A dry run: a card would have been filed. Nothing was written.
    WouldPropose,
    /// Nothing to do, and why — never an error.
    Skipped(String),
}

fn read_meta(path: &Path) -> Option<Mapping> {
    let text = crate::pystr::read_text(path).ok()?;
    crate::models::split_frontmatter(&text).ok().map(|(meta, _)| meta)
}

fn text_of(meta: &Mapping, key: &str) -> Option<String> {
    crate::yaml::get(meta, key).and_then(crate::yaml::text)
}

/// Has a `status → done` card ever been filed for `target_rel`? Reads `approvals/` **and**
/// `archive/`, whatever the card's own status, so a rejected card keeps the question closed.
pub fn already_proposed(vault: &Path, target_rel: &str) -> bool {
    for folder in ["approvals", "archive"] {
        let folder = vault.join(folder);
        if !folder.is_dir() {
            continue;
        }
        for path in crate::approvals::sorted_md(&folder) {
            let Some(meta) = read_meta(&path) else { continue };
            if text_of(&meta, "type").as_deref() != Some("approval")
                || text_of(&meta, "kind").as_deref() != Some("amend")
                || text_of(&meta, "target").as_deref() != Some(target_rel)
            {
                continue;
            }
            let Some(Value::Mapping(changes)) = crate::yaml::get(&meta, "changes") else { continue };
            let Some(Value::Mapping(status)) = crate::yaml::get(changes, "status") else { continue };
            if crate::yaml::get(status, "to").and_then(crate::yaml::text).as_deref() == Some("done") {
                return true;
            }
        }
    }
    false
}

/// Propose `status: active → done` for one task note, on one piece of evidence.
///
/// `task` is the note's path, absolute or vault-relative. `ctx` is used as given — tier 1 passes
/// `ctx.with_actor(ACTOR)`. A dry run checks everything and writes nothing. `Err` only for a
/// write that was attempted and failed; every "nothing to do" is `Ok(Outcome::Skipped(..))`.
pub fn propose_done(
    vault: &Path,
    task: &Path,
    evidence: &Evidence,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
    dry_run: bool,
) -> Result<Outcome, WriteError> {
    let path = if task.is_absolute() { task.to_path_buf() } else { vault.join(task) };
    let target_rel = crate::ids::rel(vault, &path);
    let in_tasks = target_rel.starts_with("tasks/") && !target_rel[6..].contains('/');
    if !in_tasks {
        return Ok(Outcome::Skipped(format!("not a task note: {target_rel}")));
    }
    let Some(meta) = read_meta(&path) else {
        return Ok(Outcome::Skipped(format!("unreadable: {target_rel}")));
    };
    match text_of(&meta, "status").as_deref() {
        Some("active") => {}
        Some(other) => return Ok(Outcome::Skipped(format!("status is {other}"))),
        None => return Ok(Outcome::Skipped("no status field".to_string())),
    }
    if let Some(id) = text_of(&meta, "id") {
        let by_hand = journal
            .human_set(&id, "status")
            .filter(|r| r.get("op").and_then(Json::as_str) == Some("set"));
        if by_hand.is_some() {
            return Ok(Outcome::Skipped("status set by hand".to_string()));
        }
    }
    if already_proposed(vault, &target_rel) {
        return Ok(Outcome::Skipped("already proposed".to_string()));
    }
    if dry_run {
        return Ok(Outcome::WouldPropose);
    }

    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let title = text_of(&meta, "title").unwrap_or_else(|| stem.clone());
    use crate::yamlemit::Node;
    let front = Node::map(vec![
        ("type", Node::text("approval")),
        ("kind", Node::text("amend")),
        ("title", Node::text(&format!("Mark done: {title}"))),
        ("status", Node::text("pending")),
        ("target", Node::text(&target_rel)),
        // Honest, and what `derive_urgency` falls back on: finishing work lowers urgency.
        ("urgency", Node::text("decreases")),
        // Two distinct dates, never an anchor — see the ruling in `src/yamlemit.rs`.
        ("proposed_at", Node::Date(today)),
        ("first_proposed_at", Node::Date(today)),
        ("expires", Node::Null),
        ("snooze_until", Node::Null),
        ("created_by", Node::text(&ctx.actor)),
        (
            "changes",
            Node::Map(vec![(
                Node::text("status"),
                Node::map(vec![("from", Node::text("active")), ("to", Node::text("done"))]),
            )]),
        ),
    ]);
    let evidence_json = evidence.to_json();
    let why = format!(
        "{}, so this looks finished. Approve to mark it done; reject and it will not be proposed \
         again. Evidence: {}",
        crate::judge::one_line(&evidence.summary, 200),
        crate::ledger::dumps_value(&evidence_json)
    );
    let text = format!(
        "---\n{}---\n\n**Why proposed:** {why}\n{}",
        crate::yamlemit::safe_dump_block(&front),
        crate::write::AMEND_BUTTONS
    );
    let card = crate::write::create(
        vault,
        &format!("approvals/amend-{stem}-done.md"),
        &text,
        ctx,
        journal,
        Some(&evidence_json),
    )?;
    Ok(Outcome::Proposed(card))
}

/// Tier 1: every vendor figure at 100% whose uid names a task note gets [`propose_done`].
///
/// Returns `(log, warnings)`: one log line per card filed (`proposed done: <stem> (…)`) or, in a
/// dry run, per card that would be (`would propose done: …`); a warning only for a write that
/// failed. Figures below 100%, uids with no note, and every skip rule are silent. Processed in uid
/// order so the same input files the same cards in the same order.
pub fn propose_vendor_completions(
    vault: &Path,
    completions: &[VendorCompletion],
    today: Date,
    ctx: &WriteContext,
    dry_run: bool,
) -> (Vec<String>, Vec<String>) {
    let mut log = Vec::new();
    let mut warnings = Vec::new();
    let mut complete: Vec<&VendorCompletion> = completions.iter().filter(|c| c.is_complete()).collect();
    if complete.is_empty() {
        return (log, warnings);
    }
    complete.sort_by(|a, b| a.uid.cmp(&b.uid));
    complete.dedup_by(|a, b| a.uid == b.uid);
    let known = crate::ingest::existing_by_uid(vault);
    let ctx = ctx.with_actor(ACTOR);
    let mut journal = Journal::new(vault);
    for figure in complete {
        let Some(path) = known.get(&figure.uid) else { continue };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let evidence = figure.evidence();
        match propose_done(vault, path, &evidence, today, &ctx, &mut journal, dry_run) {
            Ok(Outcome::Proposed(_)) => {
                log.push(format!("proposed done: {stem} ({} {}%)", figure.source, figure.percent()))
            }
            Ok(Outcome::WouldPropose) => log.push(format!(
                "would propose done: {stem} ({} {}%)",
                figure.source,
                figure.percent()
            )),
            Ok(Outcome::Skipped(_)) => {}
            Err(err) => warnings.push(format!("completion: proposal not written for {stem} ({err})")),
        }
    }
    (log, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn scratch(tag: &str) -> PathBuf {
        let vault = std::env::temp_dir().join(format!(
            "qo-done-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&vault);
        for folder in ["tasks", "approvals", "archive", "state", "config"] {
            std::fs::create_dir_all(vault.join(folder)).unwrap();
        }
        vault
    }

    fn today() -> Date {
        date(2026, 9, 22)
    }

    /// A task note minted the way `sync_coursework` mints one: through `write::create`, under the
    /// vendor's agent actor, so the journal holds its `create` record.
    fn task(vault: &Path, stem: &str, uid: &str, status: &str) -> PathBuf {
        let text = format!(
            "---\ntitle: \"CS 100 {stem}\"\ncourse: \"cs-100\"\ndomain: school\ndue: 2026-09-30T23:59\n\
             effort_hours: 2.5\neffort_confidence: low\neffort_source: inferred\nimportance: 2\n\
             importance_reason: \"r\"\nstatus: {status}\nprogress: 0\ncreated_by: zybooks\n\
             source_uid: \"{uid}\"\n---\n\nbody\n"
        );
        let ctx = WriteContext::new("agent:coursework.zybooks", "cli");
        let mut journal = Journal::new(vault);
        crate::write::create(vault, &format!("tasks/{stem}.md"), &text, &ctx, &mut journal, None).unwrap()
    }

    fn figure(uid: &str, earned: f64, possible: f64) -> VendorCompletion {
        VendorCompletion { source: "zybooks".to_string(), uid: uid.to_string(), earned, possible }
    }

    fn ctx() -> WriteContext {
        WriteContext::new("agent:coursework", "local-runner")
    }

    fn cards(vault: &Path, folder: &str) -> Vec<PathBuf> {
        crate::approvals::sorted_md(&vault.join(folder))
    }

    fn meta(path: &Path) -> Mapping {
        read_meta(path).expect("readable note")
    }

    fn set_by(vault: &Path, target: &str, actor: &str, field: &str, value: &str) {
        let mut journal = Journal::new(vault);
        crate::write::write_literals(
            vault,
            target,
            &[(field.to_string(), value.to_string())],
            &WriteContext::new(actor, "dashboard"),
            &mut journal,
            &crate::write::WriteOpts::default(),
        )
        .unwrap();
    }

    fn process(vault: &Path) -> crate::approvals::ApprovalsResult {
        let mut journal = Journal::new(vault);
        crate::approvals::process_approvals(
            vault,
            today(),
            today().at(12, 0, 0, 0),
            &crate::approvals::default_ctx(),
            &mut journal,
        )
    }

    fn snapshot(vault: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    out.push((crate::ids::rel(root, &path), bytes));
                }
            }
        }
        let mut out = Vec::new();
        walk(vault, vault, &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_complete_figure_files_one_card_and_a_second_run_files_none() {
        let vault = scratch("once");
        task(&vault, "hw-01", "zybooks:1", "active");
        let figures = [figure("zybooks:1", 193.0, 193.0)];

        let (log, warnings) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert_eq!(log, vec!["proposed done: hw-01 (zybooks 100%)".to_string()]);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cards(&vault, "approvals").len(), 1);

        let (log, warnings) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert!(log.is_empty(), "{log:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cards(&vault, "approvals").len(), 1, "a second run filed a second card");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_card_is_a_status_amend_under_the_completion_agent_with_its_evidence() {
        let vault = scratch("card");
        task(&vault, "hw-01", "zybooks:1", "active");
        propose_vendor_completions(&vault, &[figure("zybooks:1", 193.0, 193.0)], today(), &ctx(), false);
        let card = &cards(&vault, "approvals")[0];
        assert_eq!(card.file_name().unwrap(), "amend-hw-01-done.md");
        let m = meta(card);
        assert_eq!(text_of(&m, "kind").as_deref(), Some("amend"));
        assert_eq!(text_of(&m, "status").as_deref(), Some("pending"));
        assert_eq!(text_of(&m, "target").as_deref(), Some("tasks/hw-01.md"));
        assert_eq!(text_of(&m, "created_by").as_deref(), Some(ACTOR));
        assert!(crate::provenance::is_agent(ACTOR));
        assert_eq!(crate::approvals::as_date(crate::yaml::get(&m, "proposed_at")), Some(today()));
        assert_eq!(
            crate::approvals::as_date(crate::yaml::get(&m, "first_proposed_at")),
            Some(today())
        );
        // `validate_amendment` is the check `process_approvals` runs before applying.
        assert!(crate::approvals::validate_amendment(&vault, &m).is_ok());

        let body = crate::pystr::read_text(card).unwrap();
        assert!(body.contains("zyBooks reports 193 of 193 points earned (100%)"), "{body}");
        let mut journal = Journal::new(&vault);
        let records = journal.read(None, None);
        let create = records
            .iter()
            .find(|r| r.get("path").and_then(Json::as_str) == Some("approvals/amend-hw-01-done.md"))
            .expect("the card's create is journaled");
        assert_eq!(create.get("actor").and_then(Json::as_str), Some(ACTOR));
        assert_eq!(create.get("via").and_then(Json::as_str), Some("local-runner"));
        let evidence = create.get("evidence").expect("evidence on the record");
        assert_eq!(evidence.get("source").and_then(Json::as_str), Some("zybooks"));
        assert_eq!(evidence.get("percent").and_then(Json::as_i64), Some(100));
        assert_eq!(evidence.get("uid").and_then(Json::as_str), Some("zybooks:1"));
        // The task note itself is untouched: a proposal, never a write.
        assert_eq!(
            text_of(&meta(&vault.join("tasks/hw-01.md")), "status").as_deref(),
            Some("active")
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn anything_below_one_hundred_percent_files_nothing() {
        let vault = scratch("partial");
        task(&vault, "project-2", "zybooks:2", "active");
        task(&vault, "hw-03", "zybooks:3", "active");
        task(&vault, "empty", "zybooks:4", "active");
        let figures = [
            figure("zybooks:2", 87.0, 100.0),
            figure("zybooks:3", 292.9, 293.0),
            figure("zybooks:4", 0.0, 0.0),
        ];
        let (log, _) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert!(log.is_empty(), "{log:?}");
        assert!(cards(&vault, "approvals").is_empty());
        assert_eq!(figure("x", 292.9, 293.0).percent(), 99, "99.97% must not read as 100");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_rejected_card_is_never_filed_again() {
        let vault = scratch("reject");
        task(&vault, "hw-01", "zybooks:1", "active");
        let figures = [figure("zybooks:1", 193.0, 193.0)];
        propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        let card = crate::ids::rel(&vault, &cards(&vault, "approvals")[0]);
        set_by(&vault, &card, "quinn", "status", "rejected");
        let result = process(&vault);
        assert_eq!(result.rejected.len(), 1, "{:?}", result.warnings);
        assert!(cards(&vault, "approvals").is_empty());
        assert_eq!(cards(&vault, "archive").len(), 1);

        let (log, _) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert!(log.is_empty(), "a rejected proposal was re-filed: {log:?}");
        assert!(cards(&vault, "approvals").is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn an_approved_card_marks_the_task_done() {
        let vault = scratch("approve");
        task(&vault, "hw-01", "zybooks:1", "active");
        propose_vendor_completions(&vault, &[figure("zybooks:1", 193.0, 193.0)], today(), &ctx(), false);
        let card = crate::ids::rel(&vault, &cards(&vault, "approvals")[0]);
        set_by(&vault, &card, "quinn", "status", "approved");
        let result = process(&vault);
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        assert_eq!(result.executed.len(), 1, "{:?}", result.executed);
        assert_eq!(
            text_of(&meta(&vault.join("tasks/hw-01.md")), "status").as_deref(),
            Some("done")
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_task_already_done_archived_or_reopened_by_hand_is_left_alone() {
        let vault = scratch("alone");
        task(&vault, "done", "zybooks:1", "done");
        task(&vault, "gone", "zybooks:2", "active");
        std::fs::rename(vault.join("tasks/gone.md"), vault.join("archive/gone.md")).unwrap();
        task(&vault, "reopened", "zybooks:3", "active");
        // The student set `status` by hand (done, then back to active): they have taken a position.
        set_by(&vault, "tasks/reopened.md", "quinn", "status", "done");
        set_by(&vault, "tasks/reopened.md", "quinn", "status", "active");
        let figures = [
            figure("zybooks:1", 10.0, 10.0),
            figure("zybooks:2", 10.0, 10.0),
            figure("zybooks:3", 10.0, 10.0),
        ];
        let (log, warnings) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert!(log.is_empty(), "{log:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(cards(&vault, "approvals").is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Final review item 8 (`already_proposed`'s `to == done` filter): another amend card on the
    /// same note — a `due` re-proposal, or a status change to anything but `done` — is not a
    /// "mark done" question and must not close it.
    #[test]
    fn an_unrelated_amend_card_on_the_same_task_does_not_count_as_already_proposed() {
        let vault = scratch("unrelated");
        task(&vault, "hw-01", "zybooks:1", "active");
        let card = |name: &str, changes: &str| {
            let text = format!(
                "---\ntype: approval\nkind: amend\ntitle: \"x\"\nstatus: pending\ntarget: tasks/hw-01.md\n\
                 proposed_at: 2026-09-21\nfirst_proposed_at: 2026-09-21\ncreated_by: agent:knowlu.enrich\n\
                 changes:\n{changes}---\n\nbody\n"
            );
            std::fs::write(vault.join("approvals").join(name), text).unwrap();
        };
        card("amend-hw-01-due.md", "  due:\n    from: 2026-09-30T23:59\n    to: 2026-10-02T23:59\n");
        card("amend-hw-01-archive.md", "  status:\n    from: active\n    to: archived\n");
        assert!(!already_proposed(&vault, "tasks/hw-01.md"));

        let (log, warnings) =
            propose_vendor_completions(&vault, &[figure("zybooks:1", 193.0, 193.0)], today(), &ctx(), false);
        assert_eq!(log, vec!["proposed done: hw-01 (zybooks 100%)".to_string()], "{warnings:?}");
        assert!(already_proposed(&vault, "tasks/hw-01.md"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Final review item 8 (`propose_done`'s `op == set` filter): a note the student created by
    /// hand WITH a status has not taken a position on `status` — only a later explicit `set` is
    /// one — so a vendor's 100% still files the card for it.
    #[test]
    fn a_task_the_student_created_by_hand_with_a_status_is_still_proposed() {
        let vault = scratch("by-hand-create");
        let text = "---\ntitle: \"CS 100 hw-01\"\ncourse: \"cs-100\"\ndomain: school\n\
                    due: 2026-09-30T23:59\nstatus: active\nsource_uid: \"zybooks:1\"\n---\n\nbody\n";
        let mut journal = Journal::new(&vault);
        crate::write::create(&vault, "tasks/hw-01.md", text, &WriteContext::new("quinn", "dashboard"), &mut journal, None)
            .unwrap();
        let id = text_of(&meta(&vault.join("tasks/hw-01.md")), "id").expect("create mints an id");
        assert!(
            Journal::new(&vault).human_set(&id, "status").is_some(),
            "precondition: the hand-made create record names status"
        );

        let (log, warnings) =
            propose_vendor_completions(&vault, &[figure("zybooks:1", 193.0, 193.0)], today(), &ctx(), false);
        assert_eq!(log, vec!["proposed done: hw-01 (zybooks 100%)".to_string()], "{warnings:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_dry_run_says_what_it_would_file_and_writes_nothing() {
        let vault = scratch("dry");
        task(&vault, "hw-01", "zybooks:1", "active");
        let before = snapshot(&vault);
        let (log, _) = propose_vendor_completions(
            &vault,
            &[figure("zybooks:1", 193.0, 193.0)],
            today(),
            &ctx(),
            true,
        );
        assert_eq!(log, vec!["would propose done: hw-01 (zybooks 100%)".to_string()]);
        assert_eq!(snapshot(&vault), before, "a dry run wrote something");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_existing_daily_cap_snoozes_the_overflow_and_deletes_nothing() {
        let vault = scratch("cap");
        let mut figures = Vec::new();
        for n in 0..18 {
            task(&vault, &format!("hw-{n:02}"), &format!("zybooks:{n}"), "active");
            figures.push(figure(&format!("zybooks:{n}"), 5.0, 5.0));
        }
        let (log, _) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert_eq!(log.len(), 18);
        let mut journal = Journal::new(&vault);
        let deferred = crate::approvals::defer_over_budget(
            &vault,
            today(),
            15,
            &crate::approvals::default_ctx(),
            &mut journal,
        );
        assert_eq!(deferred.len(), 3, "{deferred:?}");
        assert_eq!(cards(&vault, "approvals").len(), 18, "overflow is snoozed, never deleted");
        // A snoozed card still counts as asked.
        let (log, _) = propose_vendor_completions(&vault, &figures, today(), &ctx(), false);
        assert!(log.is_empty(), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn propose_done_takes_a_vault_relative_path_and_any_evidence_source() {
        // The T9 shape: one task, one piece of non-vendor evidence.
        let vault = scratch("t9");
        task(&vault, "hw-07", "bb:7", "active");
        let mut detail = Map::new();
        detail.insert("subject".to_string(), Json::String("Submission received: HW 07".to_string()));
        let evidence = Evidence {
            source: "email".to_string(),
            summary: "The LMS emailed a submission receipt".to_string(),
            detail,
        };
        let mut journal = Journal::new(&vault);
        let ctx = ctx().with_actor(ACTOR);
        let target = Path::new("tasks/hw-07.md");
        let first = propose_done(&vault, target, &evidence, today(), &ctx, &mut journal, false).unwrap();
        assert!(matches!(first, Outcome::Proposed(_)), "{first:?}");
        let again = propose_done(&vault, target, &evidence, today(), &ctx, &mut journal, false).unwrap();
        assert_eq!(again, Outcome::Skipped("already proposed".to_string()));
        let body = crate::pystr::read_text(&cards(&vault, "approvals")[0]).unwrap();
        assert!(body.contains("\"source\": \"email\""), "{body}");
        let _ = std::fs::remove_dir_all(&vault);
    }
}
