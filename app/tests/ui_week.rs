//! v0.4.0 T18: Your week, Day and the registrar (note §4 Settings "Your week, Day", §5 "When do your classes meet?";
//! plan §5 T18, §7's `ui_week.rs` row). Two files: `js/carried/week.js` is the Settings section (`week`): the
//! registrar row, the confirmed commitments with their kind and level, office hours, and the Day editor with its
//! preview; it also lends the shared pieces (the window editor, the registrar controls and the registrar's change
//! list) to `js/carried/weeksetup.js`, the first-day sheet ("When do your classes meet?") raised once a launch.
//! Every pin reads only these two files and `css/week.css` (P3).
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

const WEEK: &str = "js/carried/week.js";
const SETUP: &str = "js/carried/weeksetup.js";
const CSS: &str = "css/week.css";

fn week() -> String { strip_comments(&read(WEEK)) }
fn setup() -> String { strip_comments(&read(SETUP)) }

fn bodies(js: &str, heads: &[&str]) -> String { heads.iter().map(|h| fn_body(js, h)).collect::<Vec<_>>().join("\n") }

/// D6 and §4, re-homed from `the_schedule_view_the_window_editor_and_the_moved_line_are_there` (the moved line on
/// Home is T8's): Your week and Day are a Settings section, read from `your_week` when it mounts; the window editor
/// previews 400 ms after an edit, through `preview_window`, and says the moved line or "No change to today's plan"
/// with the first five items of the previewed day; Save and an office-hours Add go through `commitments_confirm`.
/// No remove control (D8), and the page's own route name never reaches a command.
#[test]
fn the_schedule_view_the_window_editor_and_the_moved_line_are_there() {
    let js = week();
    assert!(js.contains("KnowluRouter.addSection(\"week\", "), "week.js registers the Settings section `week`");
    for f in ["loadWeek", "renderWeek", "windowEditorHtml", "windowSequence", "bindWindowEditor", "schedulePreview", "runPreview", "confirmWeek", "saveCommitment", "saveWindow"] {
        assert!(js.contains(&format!("function {f}(")), "week.js lacks function {f}");
    }
    assert!(fn_body(&js, "function loadWeek(").contains("KnowluData.call(\"your_week\", {})"), "the section reads your_week itself");
    assert!(js.contains("var PREVIEW_MS = 400;") && fn_body(&js, "function schedulePreview(").contains("setTimeout(runPreview, PREVIEW_MS)"), "§4: the preview waits 400 ms");
    let preview = fn_body(&js, "function runPreview(");
    assert!(preview.contains("KnowluData.call(\"preview_window\", { window: seq })"), "the preview sends the editor's sequence");
    assert!(preview.contains("\"No change to today's plan\""), "no move says so");
    assert!(preview.contains("items.slice(0, 5)"), "§4: the first five items of the previewed day");
    assert!(fn_body(&js, "function windowEditorHtml(").contains("data-same-as-monday"), "§4: the same-as-Monday shortcut");
    assert!(fn_body(&js, "function confirmWeek(").contains("KnowluData.call(\"commitments_confirm\", { confirm: payload })"), "one way to commitments_confirm; data.js adds the view");
    assert!(fn_body(&js, "function saveCommitment(").contains("KnowluData.call(\"set_fields\", { id: id, fields: fields })"), "kind and level through set_fields");
    let save = fn_body(&js, "function saveWindow(");
    assert!(save.contains("\"Nothing to save\"") && save.contains("env.result.window === \"unchanged\"") && save.contains("\"No change\"") && save.contains("\"Saved\""), "Save says what happened");
    let render = fn_body(&js, "function renderWeek(");
    assert!(render.contains("\"Nothing confirmed yet.\"") && render.contains("Knowlu will ask when it meets"), "the empty line and the uncovered courses");
    assert!(js.contains("\"No office hours found on your calendar.\""), "no office hours says so");
    assert!(js.contains("level: \"optional\""), "§2.2: an office-hours Add confirms at optional");
    assert!(js.contains("WEEK.dirty") && render.contains("!WEEK.dirty"), "Q10 M5: unsaved editor edits survive a re-read");
    assert!(!js.contains("data-remove") && !js.contains("delete_note"), "D8: no remove control");
    assert!(!js.contains("KnowluRouter.view()") && !js.contains("\"schedule\""), "Q10-a: the page's route never reaches a command");
}

/// M4 and D3: the Day editor's preview is a read. `preview_window` is not one of data.js's VIEWED commands, so its
/// state is never adopted; the preview reads only the answer's `moved` and `the_day`, and starts no poll.
#[test]
fn day_editor_previews_without_replacing_state() {
    let js = week();
    let preview = fn_body(&js, "function runPreview(");
    assert_eq!(js.matches("\"preview_window\"").count(), 1, "one preview call");
    assert!(preview.contains("r.state.moved") && preview.contains("r.state.the_day"), "the preview reads the answer's moved line and day");
    for bad in ["KnowluData.refresh", "KnowluData.on(", "adopt(", "view:"] {
        assert!(!preview.contains(bad), "the preview must not replace the poll's state ({bad})");
    }
    assert!(preview.contains("showWindowError("), "an invalid window is the engine's own message, under its row");
    let err = fn_body(&js, "function showWindowError(");
    assert!(err.contains("/planning day (\\w+)/"), "the message lands under the day it names");
    let data = strip_comments(&read("js/data.js"));
    let viewed = data.lines().find(|l| l.contains("var VIEWED = [")).expect("data.js's VIEWED");
    assert!(!viewed.contains("\"preview_window\""), "preview_window's state is never adopted (data.js, M4)");
}

/// §5 and D3: the first-day sheet is raised from the console's boot, once a launch, only while `your_week` says
/// `setup`; it goes through `KnowluSheets` (rank `weeksetup`) and has no Escape: Not now and Finish are the ways out.
#[test]
fn week_setup_sheet_once_a_launch() {
    let js = setup();
    assert!(js.contains("KnowluBoot.onConsole(checkWeekSetup)"), "the check runs once the console boots");
    let check = fn_body(&js, "function checkWeekSetup(");
    assert!(check.contains("weekSetup.asked") && check.contains("weekSetup.asked = true"), "once a launch");
    assert!(check.contains("KnowluData.call(\"your_week\", {})"), "the check reads your_week");
    assert!(check.contains("r.week.setup") && check.contains("!weekSetup.dismissed"), "only while setup, and never after Not now");
    assert!(check.contains("KnowluSheets.request(\"weeksetup\", SHEET)"), "the sheet goes through the queue as weeksetup");
    assert!(js.contains("escape: false"), "Escape does not close it: Not now waits as Finish does");
    assert!(!js.contains("setInterval("), "never on a timer");
    assert!(!js.contains("getElementById(\"sheet\")"), "the shell owns #sheet; the module draws into the host it is given");
}

/// Re-homed: the confirm half of `the_confirm_screen_and_the_ask_form_are_there` (the ask form is T15's). The sheet's
/// parts, nothing typed, class/lab/work preset to Mine, the two reads, Not now for the session, Finish held until the
/// calendar read settles with an edit made meanwhile kept, and the rows Finish skipped said.
#[test]
fn the_confirm_screen_and_the_ask_form_are_there() {
    let js = setup();
    let screen = fn_body(&js, "function sheetHtml(");
    for id in ["ws-status", "ws-classes", "ws-class-rows", "ws-week", "ws-week-rows", "ws-oh", "ws-oh-rows", "ws-window", "ws-later", "ws-finish"] {
        assert!(screen.contains(&format!("id=\\\"{id}\\\"")) || screen.contains(&format!("id=\"{id}\"")), "the sheet lacks #{id}");
    }
    assert!(screen.contains("When do your classes meet?") && screen.contains("Reading your calendar"));
    assert!(screen.contains("Rows you leave blank will come back as questions over the next few days."));
    assert!(!js.contains("type=\"text\"") && !js.contains("type=\\\"text\\\"") && !js.contains("<textarea"), "constraint 8: nothing typed");
    for f in ["checkWeekSetup", "openWeekSetup", "setupRowHtml", "paintSetupRows", "finishWeekSetup", "closeWeekSetup"] {
        assert!(js.contains(&format!("function {f}(")), "weeksetup.js lacks function {f}");
    }
    let row = fn_body(&js, "function setupRowHtml(");
    assert!(row.contains("{ class: 1, lab: 1, work: 1 }[p.kind] ? \"mine\" : \"\""), "§2: class, lab and work start at Mine");
    assert!(js.contains("KnowluData.call(\"commitment_proposals\", {})") && js.contains("KnowluData.call(\"your_week\", {})"));
    assert!(js.contains("weekSetup.dismissed = true"), "D3: Not now hides it for the session");
    assert!(js.contains("no class times found; Knowlu will ask this week"));
    let open = fn_body(&js, "function openWeekSetup(");
    assert!(open.contains("el(\"ws-finish\").disabled = true;") && open.matches("el(\"ws-finish\").disabled = false;").count() == 1, "Finish is held until the read settles");
    assert!(open.contains("if (win && !weekSetup.dirty)"), "an edit made while waiting is kept");
    assert!(js.contains("weekSetup.dirty = true;"), "the editor marks the screen dirty");
    let fin = fn_body(&js, "function finishWeekSetup(");
    assert!(fin.contains("env.result.warnings") && fin.contains("KnowluKit.toast("), "Finish says the skipped rows");
    assert!(fin.contains("\"Set the hours for at least one day\""), "a blank Your day asks for one day first");
}

/// Re-homed (D6, D8, §2): the registrar button sits under Your classes on the sheet and at the top of Your week, with
/// the sign-in steps and nothing typed; the commands, the `registrar:` marker, the button's two words, and the label
/// from `your_week`'s `registrar_label` (never written into the page).
#[test]
fn the_registrar_button_is_on_the_confirm_screen_and_the_schedule_view() {
    let wk = week();
    let ws = setup();
    let screen = fn_body(&ws, "function sheetHtml(");
    let classes = screen.split("ws-classes").nth(1).expect("the classes block").split("ws-class-rows").next().unwrap();
    assert!(classes.contains("ws-reg"), "D6: under Your classes");
    let controls = fn_body(&wk, "function registrarHtml(");
    for part in ["data-reg-open", "data-reg-done", "data-reg-cancel", "data-reg-say", "I'm signed in"] {
        assert!(controls.contains(part), "the registrar controls lack {part}");
    }
    for typed in ["type=\"text\"", "type=\\\"text\\\"", "password", "<input", "<textarea"] {
        assert!(!controls.contains(typed), "nothing is typed in Knowlu ({typed})");
    }
    for f in ["fromRegistrar", "registrarControls", "registrarIdle", "bindRegistrar", "renderRegistrar"] {
        assert!(wk.contains(&format!("function {f}(")), "week.js lacks function {f}");
    }
    assert!(ws.contains("function reloadSetupRows("), "the sheet re-reads its rows after a fetch");
    assert!(wk.contains("KnowluData.call(\"open_registrar_window\", {})"));
    assert!(wk.contains("KnowluData.call(\"capture_registrar\", {})"), "the read model's view is data.js's to add");
    assert!(wk.contains("KnowluData.call(\"close_registrar_window\", {})"));
    assert!(wk.contains("p.source_uid.indexOf(\"registrar:\") === 0"), "R5-a: the marker by key prefix");
    let render = fn_body(&wk, "function renderRegistrar(");
    assert!(render.contains("\"Refresh from \"") && render.contains("\"Get my class times from \""), "R5-c");
    assert!(wk.contains("r.registrar_label") && ws.contains("r.registrar_label"), "R4-g: the button only where the school has a registrar");
    assert!(ws.contains("KnowluWeek.bindRegistrar(el(\"ws-reg\"), reloadSetupRows, setupBusy)"), "the sheet's fetch holds the sheet");
    assert!(fn_body(&wk, "function bindWeek(").contains("bindRegistrar("), "Your week binds its own registrar row");
    assert!(fn_body(&wk, "function bindRegistrar(").contains("e.stopPropagation();"), "a press on the controls stays with them");
    for (f, code) in [(WEEK, &wk), (SETUP, &ws)] {
        assert!(!read(f).to_lowercase().contains("mybama"), "{f}: the label comes from registrar_label, never the page");
        assert!(!code.contains("view:"), "{f}: no command is sent a view by hand");
    }
}

/// Re-homed (R5 I1, M3): while a fetch and the reload after it run, Finish and Not now are disabled; every settle
/// path clears the busy state; no Cancel mid-capture; a rejected open says so; a flex class keeps `[hidden]`.
#[test]
fn a_registrar_fetch_holds_the_confirm_screen_and_no_registrar_hides_the_button() {
    let ws = setup();
    let busy = fn_body(&ws, "function setupBusy(");
    assert!(busy.contains("el(\"ws-finish\").disabled") && busy.contains("el(\"ws-later\").disabled"), "I1: Finish and Not now wait");
    let wk = week();
    let bind = fn_body(&wk, "function bindRegistrar(");
    assert!(bind.matches("setBusy(false)").count() >= 3, "I1: every settle path clears the busy state");
    assert!(bind.contains("c.cancel.hidden = true; setBusy(true);"), "M5: no Cancel mid-capture");
    assert!(!bind.contains("registrarIdle(host, \"\"); });"), "M6: a refused open says so");
    let render = fn_body(&wk, "function renderRegistrar(");
    assert!(render.contains("!registrarLabel"), "no registrar label, no button");
    let css = strip_css_comments(&read(CSS));
    assert!(css.contains(".wk-reg[hidden]") && css.contains("display: none"), "M3: a flex class beats the UA's [hidden]");
}

/// Re-homed (R5 re-review): one helper answers "may Finish be pressed?", every path that re-enables Finish asks it,
/// and a Finish in flight holds the registrar controls.
#[test]
fn finish_is_re_enabled_only_through_one_helper() {
    let js = setup();
    let may = fn_body(&js, "function finishMayRun(");
    for flag in ["weekSetup.reading", "weekSetup.fetching", "weekSetup.finishing"] {
        assert!(may.contains(flag), "finishMayRun ignores {flag}");
    }
    let fin = fn_body(&js, "function finishWeekSetup(");
    assert!(!fin.contains("disabled = false"), "Finish's settle never re-enables it unconditionally");
    // KnowluData.call always resolves (data.js), so the confirm has one settle path, and it frees the sheet first.
    assert!(fin.contains("setupFinishing(true)") && fin.matches("setupFinishing(false)").count() == 1, "one hold, one release");
    let settle = fin.split(".then(function (env) {").nth(1).expect("the confirm's settle");
    assert!(settle.trim_start().starts_with("setupFinishing(false);"), "the settle releases the hold before anything else");
    let held = fn_body(&js, "function setupFinishing(");
    assert!(held.contains("[data-reg-open], [data-reg-done]") && held.contains("!finishMayRun()"), "a Finish holds the registrar controls");
    assert!(fn_body(&js, "function setupBusy(").contains("!finishMayRun()"), "setupBusy asks the helper");
    assert!(fn_body(&js, "function openWeekSetup(").contains("if (finishMayRun()) { el(\"ws-finish\").disabled = false; }"), "the read's settle asks the helper");
}

/// Re-homed (phase 3 I3): after a fetch the sheet lists what the registrar confirmed, from `your_week`'s rows keyed
/// `registrar:`, each string escaped, under Your classes.
#[test]
fn the_confirm_screen_lists_what_mybama_confirmed() {
    let js = setup();
    let screen = fn_body(&js, "function sheetHtml(");
    let classes = screen.split("ws-classes").nth(1).unwrap().split("ws-class-rows").next().unwrap();
    assert!(classes.contains("ws-reg-added") && classes.contains("hidden"), "under Your classes, hidden until a fetch");
    let added = fn_body(&js, "function paintRegistrarAdded(");
    assert!(added.contains("c.source_uid.indexOf(\"registrar:\") === 0"), "only the registrar's confirmed rows");
    let line = fn_body(&js, "function addedHtml(");
    assert!(line.contains("h(c.title)") && line.contains("h(c.when || \"\")"), "escaped");
    assert!(added.contains("h(registrarLabel)") && added.contains("\" — added</p>"), "the heading: From <label> — added");
    let reload = fn_body(&js, "function reloadSetupRows(");
    assert!(reload.contains("KnowluData.call(\"your_week\", {})") && reload.contains("paintRegistrarAdded("), "read after the fetch");
}

/// Re-homed (close-out B5): the registrar's list shows changed, ended, kept, status and unread rows from the
/// overview's `registrar_changes`, on the sheet and in Your week; it computes no change and escapes every string.
#[test]
fn the_mybama_list_shows_changed_ended_kept_status_and_unread_rows() {
    let wk = week();
    let list = fn_body(&wk, "function registrarChangesHtml(");
    for key in ["ch.applied", "ch.ended", "ch.kept", "ch.status", "ch.unread"] {
        assert!(list.contains(key), "the list reads {key}");
    }
    for heading in ["\" — changed\"", "\" — ended\"", "\" — kept yours\"", "\" — status\"", "\" — not read\""] {
        assert!(list.contains(heading), "heading {heading}");
    }
    let lines = bodies(&wk, &["function changedHtml(", "function endedHtml(", "function keptHtml(", "function statusHtml(", "function unreadHtml("]);
    assert!(lines.contains("e.kept_yours") && lines.contains("e.card"), "ended says kept yours; kept says filed, waiting or listed");
    assert!(lines.contains("e.registrar") && lines.contains("e.student"), "kept shows both values");
    for part in ["h(regValue(", "h(e.status)", "h(e.until)"] {
        assert!(lines.contains(part), "escaped: {part}");
    }
    assert!(!lines.contains("h(e.reason"), "the page words an unread row itself and never shows the engine's sentence");
    let title = fn_body(&wk, "function regTitle(");
    assert!(title.contains("h(c.title)") && title.contains("h(String(key)"), "a title is escaped where it is named");
    for f in ["regTitle", "regValue", "regMeets"] {
        assert!(wk.contains(&format!("function {f}(")), "missing {f}");
    }
    assert!(fn_body(&wk, "function renderRegistrar(").contains("registrarChangesHtml(w.registrar_changes, w.commitments"), "Your week paints the block");
    let ws = setup();
    let added = fn_body(&ws, "function paintRegistrarAdded(");
    assert!(added.contains("KnowluWeek.changesHtml(") && added.contains("changes"), "the sheet lists the block too");
    assert!(fn_body(&ws, "function reloadSetupRows(").contains("w.week.registrar_changes"), "read after the fetch, from the overview");
    assert!(strip_css_comments(&read(CSS)).contains(".wk-changes"), "the list has its own style");
}

/// Re-homed (B5, m3, m4): a fetch that reached the engine and failed shows one fixed sentence, never the engine's
/// text, and says a retry needs a fresh sign-in.
#[test]
fn a_failed_fetch_says_a_fixed_sentence_and_a_fresh_sign_in() {
    let js = week();
    let line = js.split("var REG_FAILED = ").nth(1).expect("the fixed sentence is one constant").split(";\n").next().unwrap();
    assert!(line.contains("nothing was changed") && line.contains("sign in"), "m3 and m4: nothing changed; sign in again");
    let bind = fn_body(&js, "function bindRegistrar(");
    assert!(bind.contains("if (env.closed) { registrarIdle(host, REG_FAILED); }"), "a closed failure never shows env.error");
    assert!(!bind.contains("registrarIdle(host, env.error)"), "m3: the engine's text never reaches the page");
}

/// Re-homed (phase 3 m1, m2): Not now is gated as Finish is, and closing the sheet closes the school's window.
#[test]
fn not_now_waits_as_finish_does_and_closing_the_screen_closes_the_school_window() {
    let js = setup();
    for f in ["setupFinishing", "setupBusy"] {
        assert!(fn_body(&js, &format!("function {f}(")).contains("el(\"ws-later\").disabled = !finishMayRun();"), "m1: {f} gates Not now by the helper");
    }
    assert!(fn_body(&js, "function openWeekSetup(").contains("el(\"ws-later\").disabled = true;"), "m1: Not now waits for the first read");
    assert!(fn_body(&js, "function closeWeekSetup(").contains("KnowluData.call(\"close_registrar_window\", {})"), "m2: the school window goes with the screen");
    let click = fn_body(&js, "function onSheetClick(");
    assert!(click.contains("if (finishMayRun()) { weekSetup.dismissed = true; closeWeekSetup(); }"), "Not now waits for the helper");
}

/// The section and the sheet keep the page's rules: markup only from constants and `h()`, commands from the
/// console's list, the shared pieces lent once through `KnowluWeek`, level and kind as the shell's controls, and no
/// `title=` tooltip.
#[test]
fn week_files_keep_the_page_rules() {
    let wk = week();
    let ws = setup();
    let console = console_commands();
    for (f, code) in [(WEEK, &wk), (SETUP, &ws)] {
        let raw = read(f);
        assert!(!is_placeholder(&raw), "{f} is still a placeholder");
        assert!(markup_violations(&raw).is_empty(), "{f}: {:?}", markup_violations(&raw));
        for cmd in commands_sent(&raw) {
            assert!(console.contains(&cmd), "{f} sends {cmd}, which run_console does not register");
        }
        assert!(!code.contains(" title=") && !code.contains("title=\\\""), "{f}: tooltips are data-tip, never title=");
    }
    assert!(wk.contains("window.KnowluWeek = {"), "week.js lends the shared pieces");
    for name in ["editorHtml:", "sequence:", "showError:", "bindEditor:", "registrarHtml:", "bindRegistrar:", "changesHtml:", "levelHtml:"] {
        assert!(wk.contains(name), "KnowluWeek lacks {name}");
    }
    let lvl = fn_body(&wk, "function levelHtml(");
    assert!(lvl.contains("class=\"seg ") || lvl.contains("class=\"seg\""), "the level is the shell's segmented choice");
    assert!(lvl.contains("aria-pressed"), "…pressed, not a chip");
    let kind = fn_body(&wk, "function kindHtml(");
    assert!(kind.contains("class=\"sel ") || kind.contains("class=\"sel\""), "the kind is the shell's select");
    let css = strip_css_comments(&read(CSS));
    assert!(!is_placeholder(&read(CSS)), "week.css is still a placeholder");
    for banned in ["gradient", "border-radius: 999", "box-shadow: 0 0 0 999"] {
        assert!(!css.contains(banned), "week.css: {banned}");
    }
}

/// Review M10: every row's controls carry the row's title in their accessible name, so a screen reader can tell one row's
/// level, kind or answer from the next.
#[test]
fn each_rows_controls_name_their_row() {
    let wk = week();
    assert!(fn_body(&wk, "function levelHtml(").contains("aria-label=\"How much it binds, ' + titleHtml + '\""), "the level names its row");
    assert!(fn_body(&wk, "function kindHtml(").contains("aria-label=\"Kind, ' + titleHtml + '\""), "the kind names its row");
    let row = fn_body(&wk, "function commitmentHtml(");
    assert!(row.contains("kindHtml(c.kind, h(c.title))") && row.contains("levelHtml(c.level, h(c.title))"), "a commitment row passes its title, through h()");
    let ws = setup();
    let set = fn_body(&ws, "function setupRowHtml(");
    assert!(set.contains("aria-label=\"Is this yours? ' + h(p.title) + '\""), "the answer names its row");
    assert!(set.contains("KnowluWeek.levelHtml(p.level, h(p.title))"), "the sheet's level names its row");
}

/// Review M11: a level is drawn as chosen once it is saved; a refused save puts back the level that was chosen before.
#[test]
fn a_refused_level_goes_back() {
    let wk = week();
    let bind = fn_body(&wk, "function bindWeek(");
    let at = bind.find("data-level-set").expect("the level press");
    let branch = &bind[at..bind.find("var add = ").expect("the next branch")];
    assert!(branch.contains("saveCommitment(") && branch.contains("!env || !env.ok"), "the save's answer is read: {branch}");
    let refused = branch.find("!env || !env.ok").unwrap();
    assert!(branch[refused..].contains("aria-pressed"), "a refusal redraws the level held before: {branch}");
}

/// Review M1: the dark theme's floating surfaces are fixed once, in shell.css; week.css patches no field or segment.
#[test]
fn week_css_patches_no_dark_theme_field() {
    let css = strip_css_comments(&read(CSS));
    for patch in [".ws .field", ".ws .sel select", ".ws .seg"] {
        assert!(!css.contains(patch), "week.css: a local dark-theme patch ({patch}); the shell's floating surfaces carry it");
    }
}
