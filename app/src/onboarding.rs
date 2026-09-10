//! The wizard's and the picker's command surface (plan 4a, Tasks 2 and 5). Like `commands.rs`,
//! nothing here computes. These commands run in a shell that has **no `ConsoleState`** — there is
//! no vault yet — which is why `main.rs` builds two Tauri apps: a console over a resolved profile,
//! and this vault-less shell. `launch_state` is registered in both and is how the page tells them
//! apart.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde_json::{json, Value};
use tauri::Manager;
use crate::profiles::{self, Profile};

/// One row of the mapping panel: a thing the account can reach, and what we think it is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiscoveredRow {
    /// `zybooks` or `vhl`.
    pub source: String,
    /// The vendor's own key: a zyBook code, or a VHL section id.
    pub key: String,
    /// Something to show beside the key when the key alone means nothing to a human.
    pub detail: Option<String>,
    /// What we think the course is. `None` means we do not know and the panel asks.
    pub suggested: Option<String>,
    /// Already placed by this vault's config — the adopt path shows these ticked and quiet.
    pub mapped: bool,
    /// zyBooks' own onboarding book. The panel pre-ticks *ignore* rather than asking.
    pub ignored: bool,
}

/// H10's JSON → rows. **Never fails**: a source that could not be reached, an error list, a truncated
/// reply and outright garbage all come back as "no rows", because the panel's answer to all four is
/// the same — show the student the fields and let them type it.
pub fn rows_from_discovery(json: &str) -> Vec<DiscoveredRow> {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return Vec::new() };
    let mut out = Vec::new();
    for b in v.get("zybooks").and_then(|z| z.as_array()).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(code) = b.get("code").and_then(|c| c.as_str()) else { continue };
        out.push(DiscoveredRow {
            source: "zybooks".into(),
            key: code.to_string(),
            detail: None,
            suggested: crate::scaffold::suggest_course(code),
            mapped: b.get("mapped").and_then(|m| m.as_bool()).unwrap_or(false),
            ignored: b.get("ignored").and_then(|m| m.as_bool()).unwrap_or(false),
        });
    }
    for sec in v.get("vhl").and_then(|z| z.as_array()).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(id) = sec.get("section").and_then(|c| c.as_str()) else { continue };
        out.push(DiscoveredRow {
            source: "vhl".into(),
            key: id.to_string(),
            // The dashboard names no course text — only ids — so this is all there is to show.
            detail: sec.get("course_id").and_then(|c| c.as_str()).map(|c| format!("course {c}")),
            suggested: None,
            mapped: sec.get("mapped").and_then(|m| m.as_bool()).unwrap_or(false),
            ignored: false,
        });
    }
    out
}

/// H10's `errors` array, as sentences. Empty for anything unparseable — the caller already has a
/// sentence for "we got nothing at all", and two of them would be worse than one.
pub fn errors_from_discovery(json: &str) -> Vec<String> {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|v| v.get("errors").and_then(|e| e.as_array()).cloned())
        .map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Run H10's subcommand and turn its answer into rows. The credential targets are derived from the
/// path the wizard is about to create — the same derivation `store_credentials` used a panel ago, so
/// discovery reads the entries that panel just wrote.
///
/// `(async)` and **never fatal**: two vendor logins over a student's wifi is the slowest thing in the
/// wizard, and every failure is an empty list plus a sentence, because the panel can always be typed
/// into. A missing engine is that same empty list.
#[tauri::command(async)]
pub fn discover_coursework(vault: String, zybooks: bool, vhl: bool) -> Value {
    let id = profiles::id_for(Path::new(&vault));
    let mut args: Vec<String> = vec!["coursework-discover".into()];
    if zybooks {
        args.push("--zybooks-target".into());
        args.push(crate::credentials::target_for(&id, "zybooks"));
    }
    if vhl {
        args.push("--vhl-target".into());
        args.push(crate::credentials::target_for(&id, "vhl"));
    }
    let exe = match crate::scheduler::engine_exe() {
        Ok(e) => e,
        Err(e) => return json!({ "ok": true, "error": Value::Null, "rows": [], "note": format!("we could not look up your courses ({e}) — fill them in below") }),
    };
    use knowlu_engine::childproc::NoConsole;
    let out = std::process::Command::new(exe).no_console().args(&args).output();
    let stdout = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => return json!({ "ok": true, "error": Value::Null, "rows": [], "note": format!("we could not look up your courses ({e}) — fill them in below") }),
    };
    let rows = rows_from_discovery(&stdout);
    // The per-source reason, not just "something went wrong": a student whose zyBooks worked and whose
    // VHL did not needs to hear *VHL*, because the fix is their VHL password and not a retry.
    let reasons = errors_from_discovery(&stdout);
    let note = match (rows.is_empty(), reasons.is_empty()) {
        (_, false) => json!(format!("{} — fill those in below.", reasons.join("; "))),
        (true, true) => json!("we could not reach your coursework sites — fill them in below"),
        (false, true) => Value::Null,
    };
    json!({ "ok": true, "error": Value::Null, "rows": rows, "note": note })
}

pub struct Onboarding {
    pub root: PathBuf,
    /// `"wizard"` (no profiles) or `"picker"` (more than one).
    pub mode: &'static str,
    pub profiles: Mutex<Vec<Profile>>,
}

/// `{ mode }` plus, for the shell, the registered profiles and this machine's name (which the
/// wizard writes into a new vault's `runners.yaml` as `device:` — R1: read, never hard-coded).
#[tauri::command]
pub fn launch_state(app: tauri::AppHandle) -> Value {
    match app.try_state::<Onboarding>() {
        Some(o) => json!({
            "ok": true, "error": Value::Null, "mode": o.mode,
            "profiles": serde_json::to_value(&*o.profiles.lock().unwrap_or_else(|e| e.into_inner())).unwrap_or(Value::Null),
            "machine": knowlu_engine::journal::device_name(),
            "tz": jiff::tz::TimeZone::system().iana_name().unwrap_or("America/Chicago"),
            // The wizard's default folders (cloud design §4.1 and §11a, 2026-09-09): the vault's
            // PARENT is `%USERPROFILE%\Knowlu` and the mirror goes to its `Backups` sibling — visible,
            // and not OneDrive-redirected the way `Documents` is. The page never builds a path itself.
            "default_parent": default_folders().map(|(p, _)| p),
            "default_backup": default_folders().map(|(_, b)| b),
            // The wizard's campus radios (Task 6): adding a campus is adding a file and a line in
            // `scaffold::CAMPUSES`, never a string in the page.
            "campuses": crate::scaffold::CAMPUSES.iter().map(|(k, l, _)| json!({ "key": k, "label": l })).collect::<Vec<_>>(),
        }),
        None => json!({ "ok": true, "error": Value::Null, "mode": "console" }),
    }
}

/// The wizard's default vault parent and backup folder under a home directory: `<home>\Knowlu`
/// and `<home>\Knowlu\Backups` (cloud design §4.1). Siblings, so the backup rule (R-P4a-25: the
/// mirror may not be the vault or sit inside it) holds for the defaults by construction. Handle-free
/// so the test can drive it with any home.
pub fn default_folders_in(home: &Path) -> (String, String) {
    let root = home.join("Knowlu");
    (root.to_string_lossy().to_string(), root.join("Backups").to_string_lossy().to_string())
}

/// `None` only when there is no `USERPROFILE` to derive from — the page then shows "no folder
/// chosen yet" and the picker, exactly as before a default existed.
fn default_folders() -> Option<(String, String)> {
    std::env::var("USERPROFILE").ok().map(|h| default_folders_in(Path::new(&h)))
}

/// A native folder picker. `(async)` because the blocking dialog must not run on the webview
/// thread. A cancelled dialog is `{ok: true, path: null}` — not an error.
///
/// It can only return a folder that **already exists** — which is why the wizard's *Create a new
/// vault* panel picks a PARENT here and takes the vault's own name from a text field (R-P4a-11);
/// `create_vault` then refuses a `<parent>\<name>` that exists.
///
/// **`rfd` directly, not `tauri-plugin-dialog`** (see the comment on the dependency): the plugin
/// hard-enables `rfd/common-controls-v6`, whose `TaskDialogIndirect` import makes the GNU binary
/// refuse to load. This is the same `IFileOpenDialog` the plugin would have called, parented to
/// the console window so it cannot open behind it.
#[tauri::command(async)]
pub fn pick_folder(app: tauri::AppHandle, title: String) -> Value {
    let mut d = rfd::FileDialog::new().set_title(&title);
    if let Some(w) = app.get_webview_window("main") { d = d.set_parent(&w); }
    let path = d.pick_folder();
    json!({ "ok": true, "error": Value::Null, "path": path.map(|p| p.to_string_lossy().to_string()) })
}

/// A single file, filtered by one extension. The folder picker's twin — `rfd` is already this
/// crate's dialog (`tauri-plugin-dialog` cannot be used on this toolchain; see `app/Cargo.toml`).
///
/// `(async)`, like `pick_folder`: the blocking dialog must not run on the webview thread. The
/// brief's own code omitted this, which would have frozen the panel for the length of the pick
/// (fix applied here — see the Task 10 report).
#[tauri::command(async)]
pub fn pick_file(app: tauri::AppHandle, title: String, extension: String) -> Value {
    let mut d = rfd::FileDialog::new().set_title(&title).add_filter(&extension, &[extension.as_str()]);
    if let Some(w) = app.get_webview_window("main") { d = d.set_parent(&w); }
    let path = d.pick_file();
    json!({ "ok": true, "error": Value::Null, "path": path.map(|p| p.to_string_lossy().to_string()) })
}

/// The wizard's *show me how after setup* marker.
///
/// A **file in the profile folder, not a `Settings` field**, and deliberately: `state::Settings`
/// derives `Deserialize` with no `#[serde(default)]` on any field, so adding one would make every
/// existing `settings.json` fail to parse — and `Settings::load` falls back to defaults on a parse
/// failure, which would silently reset a friend's backup folder and autostart choice on the first
/// launch after an update. A zero-byte marker costs nothing and breaks nothing.
pub fn offer_marker(profile_dir: &Path) -> PathBuf {
    profile_dir.join("offer-inference")
}

/// Drop or clear the marker for one profile folder, from whichever of the three finishing paths
/// got here — CREATE and RESTORE go through [`finish_profile_in`], ADOPT through
/// [`apply_profile_settings`], and the finish panel's checkbox is the same element on all three
/// (Knowlu plan 3a Task 10 fix: the brief's own code wired this into `apply_profile_settings`
/// only, which would have silently dropped the offer for the two more common paths — creating or
/// restoring a vault — while looking correct for the one path someone happened to test by hand).
///
/// Best effort: an unwritable marker costs the reader one prompt, not their setup.
fn set_offer_marker(dir: &Path, offer: bool) {
    if offer {
        let _ = std::fs::write(offer_marker(dir), b"");
    } else {
        let _ = std::fs::remove_file(offer_marker(dir));
    }
}

/// *Use an existing vault* (spec §3 panel 2): a folder holding `config/planning.yaml` and `tasks/`
/// becomes a profile. **Nothing is written into the vault** — no scaffold, no seed note, no first
/// task.
///
/// Split into a handle-free core so `app/tests/onboarding.rs` can hash the vault tree either side
/// of the call and prove that (spec §8; S7).
pub fn adopt_vault_in(root: &Path, path: &str, name: Option<String>) -> Value {
    let vault = PathBuf::from(path);
    // **The console's own predicate, exactly** (final review, I1): `state::resolve_vault` opens a
    // vault only when `config/planning.yaml` is a file, so adopting on the weaker "there is a
    // `config/` directory" test registered profiles the very next launch would refuse to open.
    if !vault.join("config").join("planning.yaml").is_file() || !vault.join("tasks").is_dir() {
        return json!({ "ok": false, "error": format!("{path}: not a vault — a vault has config/planning.yaml and tasks/"), "profile": Value::Null });
    }
    let label = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| profiles::default_name(&vault));
    match profiles::register(root, &label, &vault) {
        Ok(p) => json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).unwrap_or(Value::Null) }),
        Err(e) => json!({ "ok": false, "error": e, "profile": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn adopt_vault(app: tauri::AppHandle, path: String, name: Option<String>) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let out = adopt_vault_in(&root, &path, name);
    if out["ok"] == true { refresh(&app, &root); }
    out
}

/// Open a profile: **relaunch this exe with `--vault`, then exit.** The console's whole world —
/// `ConsoleState`, the tray, the scheduler, autostart's arguments — is fixed at startup from one
/// vault, so switching profiles is a new process, not a re-`manage()`.
#[tauri::command(async)]
pub fn open_profile(app: tauri::AppHandle, id: String) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding" }) };
    let all = match profiles::load(&root) { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    let Some(p) = all.into_iter().find(|p| p.id == id) else {
        return json!({ "ok": false, "error": format!("no profile {id}") });
    };
    match relaunch_with(&p.vault) {
        Ok(()) => { app.exit(0); json!({ "ok": true, "error": Value::Null }) }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The argv a relaunch hands the child: what the caller is asking for, then the `--after-pid <us>`
/// handshake (R-P4a-1) — without which `tauri-plugin-single-instance`, whose lock is keyed on the
/// app identifier and is still held here, kills the arriving window instead of the leaving one.
///
/// Pure, and taking the pid rather than reading it, so `app/tests/commands.rs` can assert the exact
/// argv for both relaunches without spawning anything. `OsString` throughout: a vault path goes to
/// the child verbatim, never through a lossy `to_string_lossy` round trip.
pub fn relaunch_args<I, S>(args: I, pid: u32) -> Vec<std::ffi::OsString>
where I: IntoIterator<Item = S>, S: Into<std::ffi::OsString> {
    let mut out: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    out.push(std::ffi::OsString::from("--after-pid"));
    out.push(std::ffi::OsString::from(pid.to_string()));
    out
}

/// The one place this exe relaunches itself — *Open this profile* (`--vault <path>`) and
/// *Switch profile…* (`--pick`) differ only in the arguments (review round 1). Spawning is all it
/// does: ENDING this process is the caller's, and they do it differently — `open_profile` has no
/// `ConsoleState` to flush and calls `app.exit(0)`, while `switch_profile` runs the full quit flush
/// the tray's Quit runs.
pub fn relaunch<I, S>(args: I) -> Result<(), String>
where I: IntoIterator<Item = S>, S: Into<std::ffi::OsString> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    use knowlu_engine::childproc::NoConsole;
    std::process::Command::new(exe)
        .no_console()
        .args(relaunch_args(args, std::process::id()))
        .spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// *Open this profile*: one vault, by path, straight from the picker.
pub fn relaunch_with(vault: &Path) -> Result<(), String> {
    relaunch([std::ffi::OsString::from("--vault"), vault.as_os_str().to_os_string()])
}

/// An unreadable registry leaves the cached list alone rather than blanking the picker — the list
/// on screen is the last one that was actually read (review round 1, IMPORTANT 3).
pub fn refresh(app: &tauri::AppHandle, root: &Path) {
    if let (Some(o), Ok(ps)) = (app.try_state::<Onboarding>(), profiles::load(root)) {
        *o.profiles.lock().unwrap_or_else(|e| e.into_inner()) = ps;
    }
}

/// Everything the panels collected. A serde struct rather than a loose map, so a missing field is a
/// refusal at the boundary and not a default nobody chose.
///
/// **C1: there is no `backup_dir` and no parent.** Spec §4.1 — the app creates
/// `%USERPROFILE%\Knowlu\<name>` and `%USERPROFILE%\Knowlu\Backups`, and the wizard has no folder
/// panel to carry an answer from. `offer_inference` stays with a `serde(default)` so an older page
/// that still sends it is not a refusal; the C1 wizard never sets it (the local runtime leaves in C4).
#[derive(Debug, serde::Deserialize)]
pub struct WizardPlan {
    pub ics_url: Option<String>,
    /// The personal calendar's secret iCal address (spec §11a). Same panel as the school feed, and
    /// the same treatment: validated on the device, stored on the account, written into the vault.
    pub personal_calendar: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
    pub autostart: bool,
    /// The finish panel's *Set up local judgment after setup* checkbox. The wizard NEVER installs
    /// anything (spec §5.3, "never automatic", and the wizard's own "Nothing is fetched now"); this
    /// only drops a marker the console reads once on its first launch.
    #[serde(default)]
    pub offer_inference: bool,
    #[serde(default)]
    pub zybooks_courses: Vec<crate::scaffold::BookMapping>,
    #[serde(default)]
    pub vhl_sections: Vec<crate::scaffold::SectionMapping>,
    #[serde(default)]
    pub course_map: Vec<(String, String)>,
    #[serde(default)]
    pub courses: Vec<crate::scaffold::CourseSeed>,
}

/// Every name Windows would REWRITE or refuse (review round 1, R-P4a-23). The device names are
/// matched on the part before the first dot and case-insensitively, because `CON.txt` is `CON` to
/// the filesystem — and `NUL` is not a folder at all, it is a sink that swallows every write.
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul",
    "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
    "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// `<parent>\<name>`, validated once for both `create_vault` and `restore_vault` (R-P4a-11). The
/// folder picker can only return a folder that exists, so the vault's own folder is named here and
/// created by the caller — and a name that would escape its parent, or land on something already
/// there, is refused with a sentence the panel shows.
///
/// **Public because `app/tests/onboarding.rs` drives the refusals directly.** They are not
/// cosmetic: panel 5's credential is keyed to `id_for(<parent>\<name>)`, so a name Windows spells
/// differently on disk than the page spelled it in memory is a login filed under a target the new
/// vault's own `ingest.yaml` does not name — with `enabled: true` beside it (R-P4a-23).
pub fn dest_for(parent: &str, name: &str) -> Result<PathBuf, String> {
    // The RAW name for this one: Windows silently strips a trailing dot or space, so "Fall 2026 "
    // is the folder "Fall 2026" — and the credential would be keyed to the spelling that was typed.
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("a name cannot end in a dot or a space".to_string());
    }
    let name = name.trim();
    if name.is_empty() { return Err("give the vault a name".to_string()); }
    if name.contains(['\\', '/', ':']) || name == "." || name == ".." {
        return Err(format!("{name:?} is not a folder name — no slashes or colons"));
    }
    if name.contains(['*', '?', '"', '<', '>', '|']) {
        return Err(format!("{name:?} is not a folder name — no * ? \" < > or |"));
    }
    if RESERVED.contains(&name.split('.').next().unwrap_or(name).to_ascii_lowercase().as_str()) {
        return Err(format!("{name:?}: that name is reserved by Windows"));
    }
    let parent = PathBuf::from(parent.trim());
    if !parent.is_dir() { return Err(format!("{}: pick a folder that exists", parent.display())); }
    let dest = parent.join(name);
    if dest.exists() { return Err(format!("{} already exists — pick another name", dest.display())); }
    Ok(dest)
}

/// `<home>\Knowlu\<name>`, with the parent and the backups root created if they are not there — the
/// whole of the folder question, answered by the app (spec §4.1, §11a). Returns the vault path and
/// the backup ROOT: `backup::tick` writes `<root>\<profile_id>\vault`, so the setting is the root and
/// the per-profile folder is the engine's, unchanged.
///
/// Handle-free so `app/tests/onboarding.rs` can drive it with any home directory.
pub fn vault_dest_in(home: &Path, name: &str) -> Result<(PathBuf, PathBuf), String> {
    let (parent, backups) = default_folders_in(home);
    std::fs::create_dir_all(&parent).map_err(|e| format!("{parent}: {e}"))?;
    std::fs::create_dir_all(&backups).map_err(|e| format!("{backups}: {e}"))?;
    let dest = dest_for(&parent, name)?;
    Ok((dest, PathBuf::from(backups)))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("USERPROFILE").map(PathBuf::from).map_err(|_| "no USERPROFILE: Knowlu cannot decide where your vault goes".to_string())
}

/// The personal calendar's address, validated on the device before it ever reaches a `VaultPlan`
/// (fix round 1, item 2). Blank → `None` — a friend who never pasted one gets `calendars: []`
/// exactly as before. `webcal://` is rewritten to `https://` (R-C1-22) by
/// `lms_link::https_from_webcal`, which is the **one** implementation of that rule on the device
/// (R-C1-41, I2): the panel's pasted LMS link goes through the same line, so the two device paths to
/// the same `calendar_ics` value cannot disagree about what a student may paste. Anything else that
/// is not `https://` after that rewrite is refused by name, the same way a control character is
/// refused by field name elsewhere.
fn normalize_personal_calendar(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let rewritten = crate::lms_link::https_from_webcal(trimmed);
    if !rewritten.starts_with("https://") {
        return Err("the calendar address must start with https:// (or webcal://)".to_string());
    }
    Ok(Some(rewritten))
}

/// *Finish* for a new vault. Scaffold+seed (one atomic `create_vault`) → the session moves onto this
/// profile → settings → register.
///
/// **The session move is here, and before the settings write**, for the reason `retarget_credentials`
/// exists (R-P4a-23): the wizard signed in seven panels ago, under `account::PENDING_TARGET`, because
/// the profile id is derived from a vault path that did not exist yet. A vault whose `cloud.yaml`
/// names `knowlu/<profile_id>/session` while the token still sits under `knowlu/pending/session` is a
/// vault that cannot reach the cloud, with nothing anywhere saying why.
pub fn create_vault_in(root: &Path, home: &Path, name: &str, plan: &WizardPlan) -> Value {
    let personal_calendar = match &plan.personal_calendar {
        None => None,
        Some(raw) => match normalize_personal_calendar(raw) {
            Ok(v) => v,
            Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }),
        },
    };
    let (dest, backups) = match vault_dest_in(home, name) {
        Ok(d) => d,
        Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }),
    };
    let profile_id = profiles::id_for(&dest);
    let account_id = match crate::account::load_session(crate::account::PENDING_TARGET) {
        Ok((id, _)) => id,
        Err(_) => return json!({ "ok": false, "error": "sign in again — the account this wizard signed in with is no longer on this machine", "profile": Value::Null }),
    };
    // **The page sends course CODES; the slugs are made here.** `CS 100` is what a student types and
    // what a title says; `cs-100` is the vault's own name for it — the note's stem, every task's
    // `course:` field, and the value `judge::Heuristics::knows_course` tests. A page that invented
    // vault identifiers would be a page deciding what the engine may know (R-OB-1, R-OB-2).
    // The engine's `slugify` — the one `judge::Heuristics` reads back (see the note in `scaffold.rs`).
    let slug = knowlu_engine::ingest::slugify;
    let zybooks_courses: Vec<crate::scaffold::BookMapping> = plan.zybooks_courses.iter()
        .filter(|b| !b.label.trim().is_empty())
        .map(|b| crate::scaffold::BookMapping { code: b.code.clone(), course: slug(&b.label), label: b.label.clone() })
        .collect();
    let vhl_sections: Vec<crate::scaffold::SectionMapping> = plan.vhl_sections.iter()
        .filter(|v| !v.label.trim().is_empty())
        .map(|v| crate::scaffold::SectionMapping { section: v.section.clone(), course: slug(&v.label), label: v.label.clone() })
        .collect();
    let course_map: Vec<(String, String)> = plan.course_map.iter()
        .map(|(code, _)| (code.clone(), slug(code)))
        .filter(|(code, s)| !code.trim().is_empty() && !s.is_empty())
        .collect();
    // `courses` is empty until Task 14b's capture fills it, and empty is a correct answer: a student
    // whose campus we cannot read types the list on the panel instead.
    let courses: Vec<crate::scaffold::CourseSeed> = plan.courses.iter()
        .map(|c| crate::scaffold::CourseSeed {
            code: c.code.clone(),
            name: if c.name.trim().is_empty() { c.code.clone() } else { c.name.clone() },
            slug: if c.slug.trim().is_empty() { slug(&c.code) } else { c.slug.clone() },
        })
        .filter(|c| !c.slug.is_empty())
        .collect();
    let vp = crate::scaffold::VaultPlan {
        profile_id: profile_id.clone(),
        ics_url: plan.ics_url.clone().filter(|u| !u.trim().is_empty()),
        personal_calendar,
        zybooks_courses,
        vhl_sections,
        course_map,
        courses,
        timezone: plan.timezone.clone(),
        slots: plan.slots.clone(),
        device: knowlu_engine::journal::device_name(),
        campus: plan.campus.clone(),
        zybooks: plan.zybooks,
        vhl: plan.vhl,
        api_base: crate::account::api_base(),
        anon_key: crate::account::anon_key(),
        account_id,
    };
    if let Err(e) = crate::scaffold::create_vault(&dest, &vp) {
        return json!({ "ok": false, "error": e, "profile": Value::Null });
    }
    if let Err(e) = crate::account::move_session(crate::account::PENDING_TARGET, &crate::account::session_target(&profile_id)) {
        let _ = std::fs::remove_dir_all(&dest);
        return json!({ "ok": false, "error": format!("the sign-in could not be attached to this vault ({e}) — the new vault was removed, so nothing is half-made"), "profile": Value::Null });
    }
    finish_or_roll_back(root, &dest, Some(name.to_string()), plan, Some(backups))
}

#[tauri::command(async)]
pub fn create_vault(app: tauri::AppHandle, name: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let home = match home_dir() { Ok(h) => h, Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }) };
    let out = create_vault_in(&root, &home, &name, &plan);
    if out["ok"] == true { refresh(&app, &root); }
    out
}

/// *Restore from a backup* (spec §3 panel 2): `<backup>\<profile>\vault\` is **copied** to
/// `<home>\Knowlu\<name>` and used there — never used in place, so the backup folder is never
/// written to and never becomes the live vault by accident.
///
/// **Fix round 1, item 1: decides the folder exactly as `create_vault_in` does**, through
/// `vault_dest_in` — not a page-supplied `parent`. Before this, a fresh machine (no
/// `%USERPROFILE%\Knowlu` yet) could not restore at all, because `dest_for` refuses a parent that
/// does not exist and nothing here created it; `vault_dest_in` does.
///
/// The handle-free core, so `app/tests/onboarding.rs` can hash the BACKUP folder either side of a
/// restore and prove it is only ever read (spec §8; S7). Returns the created vault path on success.
pub fn restore_vault_in(backup: &str, home: &Path, name: &str) -> Result<PathBuf, String> {
    let src = PathBuf::from(backup);
    let mirror = find_mirror(&src).ok_or_else(|| format!("{backup}: no <profile>\\vault\\ mirror in there"))?;
    let (dest, _backups) = vault_dest_in(home, name)?;
    let staging = dest.parent().unwrap_or(Path::new(".")).join(format!(".knowlu-restore-{}", knowlu_engine::ids::new_id("stage")));
    match copy_tree(&mirror, &staging).and_then(|()| std::fs::rename(&staging, &dest).map_err(|e| e.to_string())) {
        Ok(()) => Ok(dest),
        Err(e) => { let _ = std::fs::remove_dir_all(&staging); Err(e) }
    }
}

/// **`parent` is gone** (fix round 1, item 1) — the page may keep sending it until Task 17 removes
/// the folder panel entirely; Tauri's argument matching ignores a key that names no parameter of
/// this command, so the extra field is inert rather than a refusal (confirmed by reading
/// `console.js`'s `invoke("restore_vault", …)` call, which still sends `parent`).
#[tauri::command(async)]
pub fn restore_vault(app: tauri::AppHandle, backup: String, name: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let home = match home_dir() { Ok(h) => h, Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }) };
    let out = match restore_vault_in(&backup, &home, &name) {
        // The restored copy is rolled back the same way a created vault is: the backup it came
        // from is untouched, so a failed finish costs nothing but the copy. There is no `WizardPlan`
        // folder answer any more (spec §4.1) — the restored vault's mirror is the same default
        // `Backups` sibling a created vault gets.
        Ok(dest) => finish_or_roll_back(&root, &dest, Some(name), &plan, Some(PathBuf::from(default_folders_in(&home).1))),
        Err(e) => json!({ "ok": false, "error": e, "profile": Value::Null }),
    };
    if out["ok"] == true { refresh(&app, &root); }
    out
}

/// S3: an ADOPTED vault is registered by `adopt_vault` (Task 2, which the picker also uses and
/// which has no wizard answers to apply). This is the wizard's second half of that path — panel
/// 3's backup folder and panel 6's autostart choice, written into the profile's settings file so
/// they survive the relaunch exactly as a created vault's do.
///
/// Split into a handle-free core (Task 10 review, M1) so `app/tests/onboarding.rs` can drive the
/// adopt path's `offer_inference` marker directly, the same way `create_vault_in` and
/// `restore_vault_in` already let it drive theirs.
pub fn apply_profile_settings_in(root: &Path, id: &str, plan: &WizardPlan, backup_dir: Option<PathBuf>) -> Value {
    // The adopted vault's own path, so the backup folder is checked against it here too and not
    // only in the page (review round 1, minor).
    let vault = profiles::load(root).ok().and_then(|v| v.into_iter().find(|p| p.id == id)).map(|p| p.vault);
    if let Some(v) = &vault {
        if let Err(e) = check_backup_dir(v, backup_dir.as_deref()) { return json!({ "ok": false, "error": e }); }
    }
    let settings = crate::state::Settings {
        profile_id: id.to_string(),
        backup_dir,
        autostart: plan.autostart,
        quit_at: None,
    };
    let dir = profiles::profile_dir(root, id);
    match settings.save(&dir.join("settings.json")) {
        Ok(()) => {
            set_offer_marker(&dir, plan.offer_inference);
            json!({ "ok": true, "error": Value::Null })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

#[tauri::command(async)]
pub fn apply_profile_settings(app: tauri::AppHandle, id: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding" }) };
    // An adopted vault keeps whatever backup folder it already had — there is no folder panel to
    // carry a new answer from (spec §4.1).
    apply_profile_settings_in(&root, &id, &plan, None)
}

/// `<backup>` is either the profile folder itself or the folder holding profile folders.
fn find_mirror(root: &Path) -> Option<PathBuf> {
    if root.join("vault").join("tasks").is_dir() { return Some(root.join("vault")); }
    std::fs::read_dir(root).ok()?.flatten()
        .map(|e| e.path().join("vault"))
        .find(|p| p.join("tasks").is_dir())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let (p, t) = (e.path(), to.join(e.file_name()));
        if p.is_dir() { copy_tree(&p, &t)?; } else { std::fs::copy(&p, &t).map(|_| ()).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

/// A mirror inside the thing it mirrors is not a backup: it would copy itself. That is the whole
/// rule (**R-P4a-25**) — the backup folder may not BE the vault and may not sit inside it, and a
/// vault inside the backup folder is **allowed**, because the mirror writes under
/// `<backup>\<profile>\vault` and never into the vault. It has to be allowed: the wizard's own
/// default parent, `%USERPROFILE%\Documents\Knowlu`, holds a vault at `Documents\Knowlu\<name>`,
/// and refusing it made the most obvious backup folder on the machine unpickable.
///
/// Compared the way Windows compares paths — case-insensitively, separators normalised — and only
/// at a separator, so `…\Fall` is never "inside" `…\Fall 2026`.
fn check_backup_dir(vault: &Path, backup: Option<&Path>) -> Result<(), String> {
    let norm = |s: &str| s.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let b = match backup.map(|p| norm(&p.to_string_lossy())).filter(|b| !b.is_empty()) { Some(b) => b, None => return Ok(()) };
    let v = norm(&vault.to_string_lossy());
    if v.is_empty() { return Ok(()); }
    if v == b || b.starts_with(&format!("{v}\\")) {
        return Err("the backup folder cannot be the vault, or inside it".to_string());
    }
    Ok(())
}

/// Write the profile's settings file, then register it — **settings first, deliberately**
/// (review round 1, IMPORTANT 4). Both used to be best-effort, and a settings file that could not
/// be written lost the backup folder and the autostart choice with nothing anywhere saying so.
/// Now the error is the envelope's, and doing the fallible-but-unregistered half first means a
/// failure leaves no profile in the registry to clean up — the id is derived from the vault path,
/// exactly as `profiles::register` derives it, so the file lands in the same place either way.
fn finish_profile_in(root: &Path, vault: &Path, name: Option<String>, plan: &WizardPlan, backup_dir: Option<PathBuf>) -> Value {
    if let Err(e) = check_backup_dir(vault, backup_dir.as_deref()) {
        return json!({ "ok": false, "error": e, "profile": Value::Null });
    }
    let settings = crate::state::Settings {
        profile_id: profiles::id_for(vault),
        backup_dir,
        autostart: plan.autostart,
        quit_at: None,
    };
    let dir = profiles::profile_dir(root, &settings.profile_id);
    let path = dir.join("settings.json");
    if let Err(e) = settings.save(&path) {
        return json!({ "ok": false, "error": format!("{}: {e}", path.display()), "profile": Value::Null });
    }
    set_offer_marker(&dir, plan.offer_inference);
    let label = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| profiles::default_name(vault));
    match profiles::register(root, &label, vault) {
        Ok(p) => json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).unwrap_or(Value::Null) }),
        Err(e) => json!({ "ok": false, "error": e, "profile": Value::Null }),
    }
}

/// …and if that fails, the folder created seconds ago goes with it (review round 1, minor). A
/// vault nothing has opened is worth less than a half-made folder `dest_for` refuses next time
/// round — and the user is told which it was. `restore_vault_in`'s staging rollback is the same
/// pattern.
///
/// **`pub` because `app/tests/onboarding.rs` drives the restore path's `offer_inference` marker
/// directly** (Task 10 review, M1): `restore_vault_in` alone only copies the mirror out and never
/// sees a `WizardPlan` — this is the other half, exactly as the live `restore_vault` command
/// calls it.
pub fn finish_or_roll_back(root: &Path, dest: &Path, name: Option<String>, plan: &WizardPlan, backup_dir: Option<PathBuf>) -> Value {
    let out = finish_profile_in(root, dest, name, plan, backup_dir);
    if out["ok"] == true { return out; }
    let why = out["error"].as_str().unwrap_or("the profile could not be finished").to_string();
    let tail = match std::fs::remove_dir_all(dest) {
        Ok(()) => format!(" — the new vault at {} was removed, so nothing is half-made", dest.display()),
        Err(e) => format!(" — and the new vault at {} could not be removed either ({e}); delete it before trying again", dest.display()),
    };
    json!({ "ok": false, "error": format!("{why}{tail}"), "profile": Value::Null })
}

/// Panel 5's write, made the moment the user leaves the panel (decision 3) — the vault folder need
/// not exist yet: the target is derived from the path the user chose, not from anything on disk.
/// **The secret is never returned, never logged and never echoed back to the page.**
#[tauri::command(async)]
pub fn store_credentials(vault: String, source: String, user: String, secret: String) -> Value {
    if !["zybooks", "vhl"].contains(&source.as_str()) { return json!({ "ok": false, "error": format!("unknown source {source}") }); }
    let target = crate::credentials::target_for(&profiles::id_for(Path::new(&vault)), &source);
    match crate::credentials::write(&target, &user, &secret) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "target": target }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// R-P4a-23. Panel 5's write is keyed to `<parent>\<name>` **as it stood then**, and the wizard
/// invites the user to go back and change it — the refused-Finish panel says "pick another name" in
/// so many words. Coming forward again `storeCredentials` finds the fields blanked and writes
/// nothing, so without this the password would sit under `knowlu/<id of the old path>/…` while the
/// new vault's `ingest.yaml` names the new id, `enabled: true`, and the first coursework run fails
/// to log in with nothing anywhere saying why.
///
/// Read, write, delete — inside this process. **The secret never crosses back to the page**, and
/// any failure is total: the caller refuses Finish rather than build a vault around a login that
/// is not where the vault says it is.
#[tauri::command(async, rename_all = "snake_case")]
pub fn retarget_credentials(from_vault: String, to_vault: String) -> Value {
    // `rename_all` because Tauri v2 lower-camel-cases argument keys by default (tauri-macros'
    // `ArgumentCase::Camel`), and every other command here happens to take single-word arguments
    // where the two spellings coincide. Without this the page's `from_vault` would arrive as a
    // missing argument.
    let (from_id, to_id) = (profiles::id_for(Path::new(&from_vault)), profiles::id_for(Path::new(&to_vault)));
    if from_id == to_id { return json!({ "ok": true, "error": Value::Null, "moved": 0 }); }
    let mut moved = 0;
    for source in ["zybooks", "vhl"] {
        let old = crate::credentials::target_for(&from_id, source);
        if !crate::credentials::exists(&old) { continue; }
        let cred = match knowlu_engine::wincred::read_credential(&old) {
            Ok(c) => c,
            Err(e) => return json!({ "ok": false, "error": format!("{source}: {e}"), "moved": moved }),
        };
        let new = crate::credentials::target_for(&to_id, source);
        if let Err(e) = crate::credentials::write(&new, &cred.username, cred.password.expose()) {
            return json!({ "ok": false, "error": format!("{source}: {e}"), "moved": moved });
        }
        // Only after the new one is safely written: a delete-then-write would lose the login on a
        // failure in between, and the user has no copy of it.
        if let Err(e) = crate::credentials::delete(&old) {
            return json!({ "ok": false, "error": format!("{source}: {e}"), "moved": moved });
        }
        moved += 1;
    }
    json!({ "ok": true, "error": Value::Null, "moved": moved })
}

/// The last click: relaunch into the console over the new profile (see `open_profile`).
#[tauri::command(async)]
pub fn finish_onboarding(app: tauri::AppHandle, id: String) -> Value { open_profile(app, id) }
