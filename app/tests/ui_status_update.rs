//! v0.4.0 T19: Status and the update line (note §4 Settings "Status" and "This computer", §5 "Update offer"; plan §5
//! T19, §7's `ui_status_update.rs` row). Two files: `js/carried/status.js` is the Settings section `status` (the sync
//! line with Sync now, the backup and the last update, the sources that need a look, flagged issues with Resolve,
//! warnings and unreadable notes, and the runs), and `js/carried/update.js` is the update line in the top band
//! (`#top-note`) plus This computer's Updates row (the Settings section `updates`). Every pin reads only these two
//! files and `css/status.css` (P3), and the engine and updater sources whose words the page must match.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

const STATUS: &str = "js/carried/status.js";
const UPDATE: &str = "js/carried/update.js";
const CSS: &str = "css/status.css";

fn status() -> String { strip_comments(&read(STATUS)) }
fn update() -> String { strip_comments(&read(UPDATE)) }

/// Re-homed (fix round 1, I1): `SyncStatus::of` stamps `at` on every run, a skipped one too, so `s.skipped` is read
/// before `s.at`, or a lapsed subscription or a signed-out machine reads "in step with your account".
#[test]
fn the_sync_line_reads_skipped_before_at_and_never_says_in_step_over_a_skip() {
    let js = status();
    let line = fn_body(&js, "function renderSyncLine(");
    let skipped_pos = line.find("else if (s.skipped)").expect("the sync line tests s.skipped");
    let at_pos = line.find("else if (s.at)").expect("the sync line tests s.at");
    assert!(skipped_pos < at_pos, "s.skipped must be tested before s.at");
    assert!(js.contains("\"no entitlement\"") && js.contains("\"no session\""), "the engine's own words name the skips");
    assert!(js.contains("sync skipped"), "the reason is said in plain words");
    assert!(!line[skipped_pos..at_pos].contains("in step with your account"), "a skip never falls through to the in-step copy");
}

/// Re-homed (R-C3'-exec-39 N1, N4, -43): every word the engine can leave in `SyncStatus.skipped` or `last_error` is
/// read out of the engine's source and must be a key of the page's `SYNC_SAYS`, with the ruled copy pinned; a
/// student offline past the grace is never called inactive. Both halves of the line and Sync now's refusal go
/// through the one table.
#[test]
fn the_sync_lines_words_are_the_engines_own_mapped_to_what_a_student_reads() {
    let engine = app_dir().join("..").join("engine").join("src");
    let cloudmodel = std::fs::read_to_string(engine.join("cloudmodel.rs")).expect("engine/src/cloudmodel.rs");
    let sync = std::fs::read_to_string(engine.join("sync.rs")).expect("engine/src/sync.rs");
    fn literals_after(src: &str, anchor: &str) -> Vec<String> {
        let found: Vec<String> = src.match_indices(anchor).map(|(i, _)| src[i + anchor.len()..].split('"').next().unwrap_or_default().to_string()).collect();
        assert!(!found.is_empty(), "the engine no longer has {anchor:?}; find where its word moved");
        found
    }
    let mut words: Vec<String> = Vec::new();
    for anchor in ["Unavailable::NoConfig => \"", "Unavailable::NoSession(_) => \""] {
        words.extend(literals_after(&cloudmodel, anchor));
    }
    for anchor in ["totals.skipped = Some(\"", "skipped: Some(\"", "CloudError::Transport(_) => \"", "CloudError::Status { code: 401, .. } => \"", "CloudError::Status { code: 402, .. } => \""] {
        words.extend(literals_after(&sync, anchor));
    }
    words.sort();
    words.dedup();
    assert!(words.len() >= 6, "the scan stopped finding the engine's words: {words:?}");
    let js = status();
    for w in &words {
        assert!(js.contains(&format!("\"{w}\": [")), "the engine says {w:?} and SYNC_SAYS has no entry for it");
    }
    for (word, said) in [
        ("no session", "[\"amber\", \"signed out — sign in to sync\"]"),
        ("signed out", "[\"amber\", \"signed out — sign in to sync\"]"),
        ("no entitlement", "[\"amber\", \"can't confirm your subscription — changes stay on this computer\"]"),
        ("no account", "[\"calm\", \"sync skipped — no account\"]"),
        ("another sync is running", "[\"calm\", \"sync skipped — another sync is running\"]"),
    ] {
        assert!(js.contains(&format!("\"{word}\": {said}")), "{word} must read {said}");
    }
    assert!(!js.contains("subscription inactive"), "a student offline past the grace is not inactive");
    let line = fn_body(&js, "function renderSyncLine(");
    assert!(line.contains("syncSays(s.last_error, \"amber\")") && line.contains("syncSays(s.skipped, \"calm\", \"sync skipped — \")"), "the line maps both fields");
    assert!(fn_body(&js, "function syncRefusal(").contains("syncSays(err, \"amber\")[1]"), "Sync now's refusal says the line's words");
}

/// Re-homed: the calm copy is verbatim, the last slot names its attempt and a refused slot says refused and why
/// (`(when ? " " + when : "")`, `" refused — "`, from `the_page_says_it_once_and_says_it_honestly`), and the
/// git-shaped copy stays gone.
#[test]
fn sync_line_copy_is_verbatim_and_object_kind_is_never_guessed() {
    let js = status();
    let line = fn_body(&js, "function renderSyncLine(");
    for gone in ["pending push", "conflict — auto-sync stopped", "auto-sync off"] {
        assert!(!js.contains(gone), "{gone} left with git");
    }
    assert!(line.contains("in step with your account") && line.contains("not synced yet"), "the calm copy");
    assert!(line.contains("last slot") && line.contains("attempt ") && line.contains("t.last_slot"), "the last slot and its attempt");
    assert!(line.contains("(when ? \" \" + when : \"\")"), "no time when the slot carries none");
    assert!(line.contains("\" refused — \""), "a refused slot renders its reason after the state");
    assert!(js.contains("[data-sync]"), "Sync now is found by its attribute");
}

/// Re-homed: Sync now is busy for the length of the call and every outcome says something: the time, the skip in
/// the line's own words, the refusal, or that the call could not start.
#[test]
fn sync_now_says_what_happened() {
    let js = status();
    let run = fn_body(&js, "function runSync(");
    assert!(run.contains("setSyncBusy(true)") && run.contains("setSyncBusy(false)"), "busy for the length of the call");
    assert!(run.contains("KnowluData.call(\"sync\", {})"), "the one sync call; data.js adds the view");
    assert!(run.contains("\"Synced \"") && run.contains("KnowluKit.toast("), "success toasts the time");
    assert!(run.contains("syncSays(s.skipped"), "a skip toasts the line's own words");
    assert!(run.contains("syncRefusal"), "an error keeps the refusal");
    assert!(run.contains("sync could not start — "), "no answer says so");
    let busy = fn_body(&js, "function setSyncBusy(");
    assert!(busy.contains("disabled") && busy.contains("Syncing…"), "the button is disabled and reads syncing…");
    let click = js.find("var syncBtn = e.target.closest(\"[data-sync]\")").expect("the sync click branch");
    assert!(js[click..click + 120].contains("runSync()"), "the click calls runSync");
}

/// Re-homed: the line shows the last sync time (local, through the one formatter) and a repaint mid-sync keeps the
/// busy button.
#[test]
fn the_sync_line_shows_the_last_sync_time_and_survives_a_repaint_mid_sync() {
    let js = status();
    assert!(fn_body(&js, "function localHM(").contains("KnowluFmt.clock("), "the time goes through fmt.js's clock");
    let line = fn_body(&js, "function renderSyncLine(");
    assert!(line.contains("localHM(s.at)"), "the status's `at` as a local time");
    assert!(line.contains("\"synced \"") && line.contains("\"tried \""), "ok says synced, a skip or error says tried");
    let btn = fn_body(&js, "function syncButtonHtml(");
    assert!(btn.contains("syncing ? \" disabled\"") && btn.contains("Syncing…"), "a repaint during a sync keeps the busy button");
}

/// Plan §5 T19 and note §5: Sync now is a start control: it carries `data-start` and `KnowluKit.owed()`, wrapped by
/// `KnowluKit.owedHtml`, and the section redraws when the owed flag changes (`KnowluSettings.redraw`).
#[test]
fn sync_now_is_a_start_control() {
    let js = status();
    let btn = fn_body(&js, "function syncButtonHtml(");
    assert!(btn.contains("data-sync") && btn.contains("data-start"), "Sync now is marked as a start control");
    assert!(btn.contains("KnowluKit.owed()") && btn.contains("KnowluKit.owedHtml("), "…and takes the owed lock with its tip");
    assert!(fn_body(&js, "function setSyncBusy(").contains("KnowluKit.isOwed()"), "freeing the button after a sync keeps the lock");
    assert!(js.contains("redraw: function ("), "the section redraws when the flag changes");
    let start_count = read(STATUS).matches("data-start").count();
    assert_eq!(start_count, 1, "Sync now is status.js's one start control");
}

/// Re-homed positive half of `seen_is_stamped_at_the_end_of_the_look_and_events_are_emitted` (R2-1): the sync line
/// and the runs have their renderers, Sync now sends `sync` and emits `sync_run`, and the event carries no text.
#[test]
fn the_sync_line_and_runs_emit_their_events() {
    let js = status();
    for f in ["renderSyncLine", "renderRunsView", "renderIssues", "renderWarnings", "runSync", "resolveIssue"] {
        assert!(js.contains(&format!("function {f}(")), "status.js lacks function {f}");
    }
    assert!(js.contains("ev(\"sync_run\", null, null, null)"), "Sync now emits sync_run, nothing else");
    let sec = fn_body(&js, "function sectionHtml(");
    for id in ["sx-sync", "sx-runs", "sx-issues", "sx-warns"] {
        assert!(sec.contains(id), "the section has #{id}");
    }
    assert!(js.contains("KnowluRouter.addSection(\"status\", "), "status.js registers the Settings section `status`");
}

/// I1: Status lists the journal warnings (`S.warnings`, with the runs' own), and every note Knowlu could not read
/// (`S.unreadable`), each through `h()`.
#[test]
fn status_lists_warnings() {
    let js = status();
    let warn = fn_body(&js, "function renderWarnings(");
    assert!(warn.contains("st.warnings") && warn.contains("st.unreadable"), "both the journal's warnings and the unreadable notes");
    assert!(warn.contains("runs_panel") && warn.contains(".warnings"), "the runs' warnings too");
    assert!(warn.contains("h(w)") && warn.contains("h(p)"), "each line escaped");
    assert!(warn.contains("could not read"), "an unreadable note is said in words");
}

/// Re-homed (L7 3.6): the sources in trouble come from `issues_panel.sources`, each observed as `info`; a flagged issue
/// is observed as `issue` and takes a resolution through `resolve_issue`. Its categories are words, never chips. Review
/// M8: Settings draws the sources in trouble once, under Sources (note §4; T17's `sources.js`), so Status draws none.
#[test]
fn the_issues_view_renders_source_issues_from_the_payload() {
    let js = status();
    let sources = strip_comments(&read("js/carried/sources.js"));
    let drawn = fn_body(&sources, "function drawIssues(");
    assert!(drawn.contains("ip.sources") && drawn.contains("issueRowHtml"), "Sources draws the rows from issues_panel.sources");
    assert!(fn_body(&sources, "function issueRowHtml(").contains("data-kind=\"info\""), "a source row is observed as info");
    assert!(!js.contains(".sources") && !js.contains("sx-sources") && !js.contains("data-kind=\"info\""), "Status draws no source issue: Sources does (M8)");
    assert!(fn_body(&js, "function issueHtml(").contains("data-kind=\"issue\""), "a flagged row is observed as issue");
    let sec = fn_body(&js, "function sectionHtml(");
    assert!(sec.contains("sx-issues"), "the flagged rows");
    assert!(fn_body(&js, "function resolveIssue(").contains("KnowluData.call(\"resolve_issue\", { id: id, resolution: text })"), "Resolve sends the typed resolution");
    assert!(!read(STATUS).contains("chip") && !read(CSS).contains("chip"), "no chips");
    assert_eq!(read(STATUS).matches(" data-id=\"").count(), read(STATUS).matches(" data-kind=\"").count(), "every observed row names its kind");
}

/// Re-homed (plan 4a T8): the offer is the engine's, rendered in the top band's `#top-note`; the release host is not
/// in the page.
#[test]
fn the_update_offer_is_in_the_page_and_the_endpoint_is_not() {
    let js = update();
    assert!(js.contains("function renderUpdateOffer("));
    assert!(fn_body(&js, "function renderUpdateOffer(").contains("Restart to update"), "the exact offer text");
    assert!(js.contains("KnowluData.call(\"check_for_updates\", {})") && js.contains("KnowluData.call(\"install_update\", {})"));
    assert!(js.contains("getElementById(\"top-note\")"), "the line lives in #top-note (plan §3)");
    for f in [STATUS, UPDATE, CSS] {
        assert!(!read(f).contains("knowlu.com"), "{f} must not name the release host");
    }
    assert!(js.contains("KnowluBoot.onConsole("), "one check at launch, on the console only");
    assert!(js.contains("KnowluRouter.addSection(\"updates\", "), "update.js registers This computer's Updates row");
    let row = fn_body(&js, "function rowHtml(");
    assert!(row.contains("id=\"set-updates\"") && row.contains("id=\"set-update-state\"") && row.contains("id=\"set-update-check\""), "the row and its control");
}

/// Re-homed: Check now renders every outcome in the Updates row, a check that lost the single flight asks again on
/// one timer, and no retry runs once an install was pressed or while the line shows an install message.
#[test]
fn check_now_renders_every_outcome_in_the_updates_row() {
    let js = update();
    let chk = fn_body(&js, "function checkForUpdates(");
    assert!(chk.contains("checking…") && chk.contains("disabled"), "busy while it runs");
    let out = fn_body(&js, "function updateOutcome(");
    assert!(out.contains("up to date (checked ") && out.contains(" is available") && out.contains("u.error"), "up to date, available, error");
    assert!(out.contains("could not check — "), "a failed check says why");
    assert!(out.contains("localHM("), "the check time is local");
    assert!(chk.contains("renderUpdateOffer(u, manual !== true)"), "a quiet check is told it is quiet");
    assert!(chk.contains("ALREADY_CHECKING") && chk.contains("setTimeout("), "a check that lost the flight asks again");
    assert!(js.contains("checkForUpdates(true)"), "the button's press is the manual path");
    assert!(chk.contains("!UPD.attempted") && chk.contains("!UPD.note"), "no retry after an install attempt or while a note shows");
    assert!(chk.contains("clearTimeout(UPD.timer)") && chk.contains("UPD.timer = setTimeout("), "one shared retry timer");
    let offer = fn_body(&js, "function renderUpdateOffer(");
    assert!(offer.contains("quiet && UPD.note"), "a quiet check never repaints a line that shows an install message");
    assert!(offer.contains("localHM(u.last_check)"), "the row shows the check time");
    assert!(offer.contains("UPD.installing"), "a later check does not repaint the line mid-install");
}

/// Review I3: every `check_for_updates` reply carries the updater's whole state (`version`, `staged`, `held`,
/// `last_check`, `last_error`, `action_error`; commands.rs), error or not, so a failed check renders like any other: the
/// Updates row names the failure, and `#top-note` follows the reply's `staged`, so an offer the updater no longer makes
/// (a slot started and holds the bundle back) goes. The install guard still keeps an install message.
#[test]
fn a_failed_check_still_renders_the_updaters_reply() {
    let js = update();
    let offer = fn_body(&js, "function renderUpdateOffer(");
    assert!(!offer.contains("if (u.error)"), "renderUpdateOffer never skips an error reply: {offer}");
    let text = offer.find("UPD.text = ").expect("the row's line is set from the reply");
    let guard = offer.find("UPD.installing || (quiet && UPD.note)").expect("the install guard");
    assert!(text < guard, "the row's line is set before the install guard, on every reply");
    assert!(offer[..text].contains("u.last_error"), "a failed check names its error in the row");
    assert!(offer[guard..].contains("u.staged ?"), "#top-note follows the reply's staged");
}

/// Review M5: while the privacy update is owed, Status says in words why Sync now is locked (the kit's one line),
/// redrawn whenever the flag changes.
#[test]
fn status_says_why_its_start_control_is_locked() {
    let js = status();
    let owed = fn_body(&js, "function drawOwed(");
    assert!(owed.contains("KnowluKit.isOwed()") && owed.contains("KnowluKit.OWED_LINE"), "the kit's line while owed: {owed}");
    assert!(fn_body(&js, "function sectionHtml(").contains("id=\"sx-owed\""), "at the top of Status");
    let redraw = js.split("redraw: function () {").nth(1).expect("the section's redraw");
    assert!(redraw[..redraw.find('}').unwrap()].contains("drawOwed()"), "redrawn when the flag changes");
}

/// Re-homed: Restart to update shows progress, waits out a running slot and retries on its own, and keeps a failure on
/// the line with a way to try again.
#[test]
fn the_update_button_shows_progress_and_keeps_a_failure_on_the_banner() {
    let js = update();
    let ins = fn_body(&js, "function installUpdate(");
    assert!(ins.contains("disabled = true") && ins.contains("Installing…"), "disabled and says installing…");
    assert!(ins.contains("update failed") && ins.contains("Try again"), "a failure stays on the line with a way to retry");
    assert!(ins.contains("UPD.installing") && ins.contains("UPD.attempted = true"), "one install at a time; it ends the retries");
    assert!(ins.contains("waiting for the current sync to finish") && ins.contains("a slot is running"), "a running slot is a wait");
    assert!(ins.contains("setTimeout(attempt"), "and the install retries on its own");
    assert!(ins.contains("restart Knowlu to finish"), "ok without an exit says so");
    let click = js.find("var ins = e.target.closest(\"[data-install]\")").expect("the install click branch");
    assert!(js[click..click + 120].contains("installUpdate("), "the click calls installUpdate");
    assert!(fn_body(&js, "function noteHtml(").contains("data-install"), "the line's button is found by its attribute");
}

/// Re-homed: the page matches the updater's two refusals by their words; the words live in `updates.rs`.
#[test]
fn the_page_matches_the_updaters_refusal_words() {
    let rs = read_app("src/updates.rs");
    let js = update();
    assert!(rs.contains("pub const ALREADY_CHECKING: &str = \"a check is already running\";"), "updates.rs ALREADY_CHECKING");
    assert!(js.contains("var ALREADY_CHECKING = \"a check is already running\";"), "update.js matches it exactly");
    assert!(rs.contains("pub const SLOT_RUNNING: &str = \"a slot is running"), "updates.rs SLOT_RUNNING");
    assert!(js.contains("msg.indexOf(\"a slot is running\") === 0"), "update.js matches its start");
}

/// Re-homed: a manual check that will not be retried does not promise a result.
#[test]
fn a_manual_check_that_will_not_retry_does_not_promise_a_result() {
    let js = update();
    let out = fn_body(&js, "function updateOutcome(");
    assert!(out.contains("retrying") && out.contains("try again shortly"), "the outcome knows whether a retry follows");
    assert!(fn_body(&js, "function checkForUpdates(").contains("updateOutcome(u, retry)"), "and the check tells it");
}

/// The two files keep the page's rules: markup only from constants and `h()`, commands from the console's list, no
/// `title=` tooltip, nothing on a timer but the updater's own retries.
#[test]
fn status_and_update_keep_the_page_rules() {
    let console = console_commands();
    for f in [STATUS, UPDATE] {
        let raw = read(f);
        assert!(!is_placeholder(&raw), "{f} is still a placeholder");
        assert!(markup_violations(&raw).is_empty(), "{f}: {:?}", markup_violations(&raw));
        for cmd in commands_sent(&raw) {
            assert!(console.contains(&cmd), "{f} sends {cmd}, which run_console does not register");
        }
        let code = strip_comments(&raw);
        assert!(!code.contains(" title=") && !code.contains("view:"), "{f}: no title= tooltip, no hand-sent view");
        assert!(!code.contains("setInterval("), "{f}: nothing polls on its own");
    }
    assert!(!is_placeholder(&read(CSS)), "status.css is still a placeholder");
    assert!(!strip_css_comments(&read(CSS)).contains("gradient"), "status.css: no gradient");
}
