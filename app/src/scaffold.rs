//! The vault a wizard makes from nothing (plan 4a, Task 4; spec §3.1).
//!
//! **`create_vault` is the whole of it.** Files and the two seed notes are materialised into a
//! sibling staging folder and moved into place by ONE rename, so the destination either does not
//! exist or is a complete, already-seeded, already-migrated vault — never a folder that
//! `dest_for` refuses to reuse and adoption would greet with `journal has no migration records`
//! (review round 1, Important 2). `seed_writes` is deliberately private for that reason: a second
//! caller running it against a vault `create_vault` already seeded is exactly the stranded-vault
//! failure the staging folder exists to prevent.
//!
//! Adopting or restoring a vault calls NEITHER: writing a seed note into an existing vault would
//! be the app inventing history.
use std::path::{Path, PathBuf};
use knowlu_engine::journal::Journal;
use knowlu_engine::write::{self, WriteContext};
use knowlu_engine::yamlemit::{safe_dump_block, Node};

/// `(key, label, LMS base URL)` — the wizard's radio list, and where Task 14's sign-in window points
/// for that campus. **Adding a campus is adding a file and one line here**; nothing else in the app
/// knows a campus exists. The URL is empty for `none`, which is what makes `lms_link::capture` take
/// its paste-a-link path for a student whose school is not listed yet.
pub const CAMPUSES: [(&str, &str, &str); 2] = [
    ("none", "None", ""),
    ("university-of-alabama", "University of Alabama", "https://ualearn.blackboard.com/"),
];

const CAMPUS_NONE: &str = include_str!("../assets/campus/none.yaml");
const CAMPUS_UA: &str = include_str!("../assets/campus/university-of-alabama.yaml");
const PLANNING: &str = include_str!("../assets/scaffold/planning.yaml");
const WEEK_TEMPLATE: &str = include_str!("../assets/scaffold/week_template.yaml");
const GITIGNORE: &str = include_str!("../assets/scaffold/gitignore.txt");

pub fn campus_yaml(key: &str) -> Option<&'static str> {
    match key { "none" => Some(CAMPUS_NONE), "university-of-alabama" => Some(CAMPUS_UA), _ => None }
}

#[derive(Debug, Clone)]
pub struct VaultPlan {
    pub profile_id: String,
    pub ics_url: Option<String>,
    /// The student's own busy-time calendar, by its secret iCal address (spec §11a). Written into
    /// `config/ingest.yaml`'s `calendars:` list, which is what makes today's page know the day is
    /// already half full — the reason this ruling exists at all.
    pub personal_calendar: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub device: String,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
    /// The three cloud values (C1). `api_base` and `anon_key` are Rust's — the page never sees a URL
    /// (`static_assets.rs` forbids one) — and `account_id` comes from the session the wizard signed
    /// in with, read from Credential Manager, never from the page.
    pub api_base: String,
    pub anon_key: String,
    pub account_id: String,
}

/// **Every** wizard-supplied value goes through this before it reaches a YAML file, so a typed
/// value can never change the file's SHAPE (review round 1, Important 1).
///
/// PyYAML's single-quoted style, and it is the right one precisely because inside `'…'` YAML does
/// no escape processing at all: `: `, ` #`, `{`, `[`, `"`, `\`, a leading `%` or `!` are ordinary
/// characters, and the only special case is `'` itself, escaped by doubling. What a single-quoted
/// scalar cannot carry on one line is a control character — so those are REFUSED, by field name,
/// rather than written out and silently breaking the file.
///
/// Silently is the operative word. `runs::load_runners_config` degrades an unparsable
/// `config/runners.yaml` to `Ok(vec![])` and `runs::runner_settings` to `scheduler: Script`, so an
/// unquoted timezone like `America/Chicago: x` would leave a friend's only runner switched off
/// with nothing on any page saying so — the vault would simply never refresh.
fn yaml_scalar(field: &str, value: &str) -> Result<String, String> {
    if let Some(c) = value.chars().find(|c| c.is_control()) {
        return Err(format!("{field}: control character U+{:04X} is not allowed", c as u32));
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

/// The vault's own settings. **No secret is ever written here** — only the `credential_target`
/// names the engine's `wincred::read_credential` looks up (decision 6).
///
/// `ics_url` is written verbatim (quoted, never rewritten): the engine fetches exactly the string
/// the friend pasted, so any mangling here would be a feed that 404s for a reason nothing explains.
pub fn ingest_yaml(p: &VaultPlan) -> Result<String, String> {
    let mut s = String::new();
    if let Some(u) = &p.ics_url { s.push_str(&format!("ics_url: {}\n", yaml_scalar("LMS feed URL", u)?)); }
    s.push_str(&format!("timezone: {}\n", yaml_scalar("timezone", &p.timezone)?));
    s.push_str("course_map: {}\n");
    // The engine reads `calendars:` as a list of `{name, ics_url}` mappings (`calfeed.rs`), and an
    // empty list is why the first page of a fresh install used to show a day with no busy time in it
    // at all. A block list, through the same `yaml_scalar` every other wizard value goes through.
    match &p.personal_calendar {
        None => s.push_str("calendars: []\n"),
        Some(u) => s.push_str(&format!(
            "calendars:\n  - name: personal\n    ics_url: {}\n",
            yaml_scalar("personal calendar address", u)?
        )),
    }
    if p.zybooks || p.vhl {
        s.push_str("\n# Passwords are NOT here. They live in Windows Credential Manager under the\n");
        s.push_str("# credential_target names below.\ncoursework:\n");
        if p.zybooks {
            let target = yaml_scalar("zybooks credential target", &crate::credentials::target_for(&p.profile_id, "zybooks"))?;
            s.push_str(&format!("  zybooks:\n    enabled: true\n    credential_target: {target}\n    courses: {{}}\n"));
        }
        if p.vhl {
            let target = yaml_scalar("vhl credential target", &crate::credentials::target_for(&p.profile_id, "vhl"))?;
            s.push_str(&format!("  vhl:\n    enabled: true\n    credential_target: {target}\n    sections: {{}}\n"));
        }
    }
    Ok(s)
}

/// Decision 4: `scheduler: app` and `device:` from birth. `grace_minutes: 20` is the local
/// runner's own number, unchanged; `cloud` is deliberately absent — a friend has no cloud runner,
/// and an expected-but-never-seen runner would paint every Runs view amber forever.
pub fn runners_yaml(p: &VaultPlan) -> Result<String, String> {
    let mut times = Vec::with_capacity(p.slots.len());
    for (i, t) in p.slots.iter().enumerate() {
        // A quoted scalar inside a flow sequence keeps its own commas and colons, so `[…]` stays
        // one item per slot however the slot is spelled.
        times.push(yaml_scalar(&format!("slot {}", i + 1), t)?);
    }
    Ok(format!(
        "runners:\n  - name: local\n    times: [{}]\n    tz: {}\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n",
        times.join(", "),
        yaml_scalar("timezone", &p.timezone)?,
        yaml_scalar("device name", &p.device)?,
    ))
}

/// `config/cloud.yaml` — the four keys the C2 contract fixes, in that order, through the same
/// `yaml_scalar` every other wizard value goes through, so a typed value can never change the file's
/// shape.
///
/// **No secret is here.** `anon_key` is published in every Supabase client and grants nothing on its
/// own; the session JWT is *named*, not carried — `session_credential_target` is a Credential Manager
/// target, exactly as `credential_target` is for the coursework logins.
pub fn cloud_yaml(p: &VaultPlan) -> Result<String, String> {
    Ok(format!(
        "api_base: {}\nanon_key: {}\nsession_credential_target: {}\naccount_id: {}\n",
        yaml_scalar("api_base", &p.api_base)?,
        yaml_scalar("anon_key", &p.anon_key)?,
        yaml_scalar("session_credential_target", &crate::credentials::target_for(&p.profile_id, "session"))?,
        yaml_scalar("account_id", &p.account_id)?,
    ))
}

/// Task 18's half: a vault that already exists gains `config/cloud.yaml` and nothing else — no
/// scaffold, no seed note, no rewrite of anything. **Refuses to overwrite one**: an install adopted
/// twice must not silently repoint at a second account, and a user who signed in with the wrong
/// address needs to hear that rather than to lose the first one.
pub fn write_cloud_yaml_if_absent(vault: &Path, plan: &VaultPlan) -> Result<(), String> {
    let path = vault.join("config").join("cloud.yaml");
    if path.exists() {
        return Err(format!("{}: this vault already has an account", path.display()));
    }
    // An adopted vault's `config/` already exists, but Task 18 does not promise that — this is the
    // only writer of `config/cloud.yaml` outside `build_into`, which makes the folder itself.
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write_file(vault, "config/cloud.yaml", &cloud_yaml(plan)?)
}

/// Materialise **everything** — files and the two seed notes — into a sibling staging folder, then
/// **one rename** into place: a crash, a full disk, a rejected value or a failed seed leaves the
/// destination untouched and the staging folder removed. Never a half-vault, and never a whole
/// vault missing its migration record (spec §3.1; review round 1, Important 2).
///
/// Seeding the staging folder is safe because a journal record's `path` is vault-relative
/// (`write::create` → `ids::rel`) and nothing it writes names the vault's own location, so the
/// records the rename carries into `dest` are the records seeding `dest` directly would have
/// written. `the_move_does_not_leak_the_staging_path_into_the_journal` pins that.
pub fn create_vault(dest: &Path, plan: &VaultPlan) -> Result<(), String> {
    if dest.exists() { return Err(format!("{}: already exists", dest.display())); }
    let parent = dest.parent().ok_or_else(|| format!("{}: no parent folder", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let staging: PathBuf = parent.join(format!(".knowlu-new-{}", knowlu_engine::ids::new_id("stage")));
    let built = build_into(&staging, plan).and_then(|()| seed_writes(&staging, plan));
    match built.and_then(|()| std::fs::rename(&staging, dest).map_err(|e| format!("{}: {e}", dest.display()))) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = std::fs::remove_dir_all(&staging); Err(e) }
    }
}

fn write_file(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    // `pystr::write_text`: the repo's files are CRLF and the engine reads them back through the
    // same translation (CLAUDE.md's line-ending rule).
    let path = root.join(rel);
    knowlu_engine::pystr::write_text(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn build_into(root: &Path, plan: &VaultPlan) -> Result<(), String> {
    for folder in ["tasks", "approvals", "archive", "courses", "issues", "info", "state", "state/journal", "config"] {
        let path = root.join(folder);
        std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    // The three files that cannot fail first, the three built from wizard values after — so a
    // rejected value or an unknown campus fails PART-WAY through materialisation, which is the
    // case the staging folder exists for and the case `a_failure_part_way_through_leaves_nothing`
    // drives.
    write_file(root, "config/planning.yaml", PLANNING)?;
    write_file(root, "config/week_template.yaml", WEEK_TEMPLATE)?;
    write_file(root, ".gitignore", GITIGNORE)?;
    let campus = campus_yaml(&plan.campus).ok_or_else(|| format!("unknown campus preset {:?}", plan.campus))?;
    write_file(root, "config/events.yaml", campus)?;
    write_file(root, "config/ingest.yaml", &ingest_yaml(plan)?)?;
    write_file(root, "config/runners.yaml", &runners_yaml(plan)?)?;
    write_file(root, "config/cloud.yaml", &cloud_yaml(plan)?)?;
    Ok(())
}

const SEED_BODY: &str = "This vault was created by Knowlu's onboarding. The note exists so the \
journal has a migration record from the vault's first day; nothing reads its body.";

const FIRST_TASK_BODY: &str = "Open the drawer on this row to see every field Knowlu keeps. Tick \
it off when you have had a look — Today will fill up as your courses do.";

/// The two engine calls, in this order (spec §3.1). Both go through `knowlu_engine::write`, and only
/// the first uses a non-`console_ctx()` context — **the one exception in the whole app** (R2).
///
/// **Private, and it stays private** (review round 1, Important 2): `create_vault` runs it against
/// the staging folder before the rename, so a second call — against a vault that already holds
/// `archive/_migrated.md` — could only ever be an error arriving after the vault is already on
/// disk, which is the stranded vault this design removes.
fn seed_writes(vault: &Path, _plan: &VaultPlan) -> Result<(), String> {
    let mut journal = Journal::new(vault);

    // Decision 5: `passes::detect_external` refuses to attribute edits unless the journal holds a
    // `create` record whose actor starts with `system:migration` — the shape
    // `scripts/migrate_s1.py` left in Quinn's journal. One archived note satisfies it from the
    // vault's first day, so a friend never meets "journal has no migration records".
    // `via: "cli"` — the exact context `scripts/migrate_s1.py` and `seed_migrated` used. The
    // journal's `via` vocabulary does not grow for this (R-P4a-6): the record is recognised by its
    // ACTOR, which is what `passes.rs` keys on.
    let seed = format!("---\ntitle: Vault created by Knowlu\nstatus: archived\n---\n\n{SEED_BODY}\n");
    write::create(vault, "archive/_migrated.md", &seed, &WriteContext::new("system:migration", "cli"), &mut journal, None)
        .map_err(|e| e.to_string())?;

    // …and one real task, so Today is not empty and the first `rank` has something to order.
    let front = Node::map(vec![
        ("title", Node::text("Get to know Knowlu")),
        ("course", Node::Null),
        ("domain", Node::text("school")),
        ("due", Node::Null),
        ("effort_hours", Node::Float(0.5)),
        ("effort_source", Node::text("quinn")),
        ("importance", Node::Int(2)),
        ("status", Node::text("active")),
        ("progress", Node::Int(0)),
        ("created_by", Node::text("quinn")),
    ]);
    let text = format!("---\n{}---\n\n{FIRST_TASK_BODY}\n", safe_dump_block(&front));
    write::create(vault, "tasks/get-to-know-knowlu.md", &text, &crate::commands::console_ctx(), &mut journal, None)
        .map_err(|e| e.to_string())?;
    Ok(())
}
