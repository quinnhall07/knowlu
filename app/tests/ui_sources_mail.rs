//! v0.4.0 T17: Settings > Sources and the mail sheet (note §4 "Settings" Sources row, §5 "Mail setup on first open",
//! §11 item 4; plan §5 T17, §7's `ui_sources_mail.rs` row and the R2-1 split rows that name this file).
//!
//! `js/carried/sources.js` registers the Sources section with the Settings frame (`KnowluRouter.addSection`): the Google
//! row, the School email row with each mailbox and the saved-sign-in switch, the Blackboard (grades) row and a line per
//! source issue (`S.issues_panel.sources`). `js/carried/mail.js` holds the one mail sheet (`KnowluSheets` "mail"): the
//! progress view and "From your email" as a second view inside it, `window.KnowluMail`, and the first open at launch.
//! `css/mail.css` styles both. Every pin reads only these three files, plus the Settings frame's merged `settings.js`
//! where a pin is about where the section sits (P3).
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

const SOURCES: &str = "js/carried/sources.js";
const MAIL: &str = "js/carried/mail.js";
const CSS: &str = "css/mail.css";

/// A script with its comments stripped: what a pin reads is what runs.
fn code(rel: &str) -> String { strip_comments(&read(rel)) }

/// The code between two marker comments of a file (`/* ---- <start>` up to `/* ---- <end>`), comments stripped.
fn region(rel: &str, start: &str, end: &str) -> String {
    let raw = read(rel);
    let a = raw.find(&format!("/* ---- {start}")).unwrap_or_else(|| panic!("{rel} has no region marker {start:?}"));
    let b = a + raw[a..].find(&format!("/* ---- {end}")).unwrap_or_else(|| panic!("{rel} has no region end {end:?}"));
    strip_comments(&raw[a..b])
}

fn google() -> String { region(SOURCES, "Google row", "end Google row") }
fn mail_settings() -> String { region(SOURCES, "mail settings", "end mail settings") }
fn grades() -> String { region(SOURCES, "grades row", "end grades row") }
fn progress() -> String { region(MAIL, "mail progress view", "end mail progress view") }
fn applied() -> String { region(MAIL, "mail applied", "end mail applied") }

/// The lines of `code` that send a usage event.
fn ev_lines(code: &str) -> Vec<&str> { code.lines().filter(|l| l.contains("ui_event") || is_ev_call(l)).collect() }

// ---- Google (re-homed, plan §7) ------------------------------------------------------------------

/// Gmail connect T9a: the Google row sits in Sources, which the Settings frame hosts right after Account and Membership,
/// with every id the Connect, Reconnect and Disconnect handlers bind to.
#[test]
fn the_google_row_sits_after_account_with_its_ids() {
    let js = code(SOURCES);
    assert!(js.contains("KnowluRouter.addSection(\"sources\","), "sources.js registers the Sources section with the frame");
    let frame = code("js/pages/settings.js");
    let page = fn_body(&frame, "function pageHtml(");
    let acct = page.find("accountHtml()").expect("the frame draws Account");
    let src = page.find("slotHtml(\"sources\")").expect("the frame hosts Sources");
    assert!(acct < src, "Sources comes after Account and Membership");
    let markup = fn_body(&js, "function sectionHtml(");
    let row = markup.find("id=\"set-google\"").expect("#set-google is in the section");
    for id in ["set-google-state", "set-google-note", "set-google-c"] {
        let at = markup.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("the section has no #{id}"));
        assert!(row < at, "#{id} sits in the Google row");
    }
    let g = google();
    for id in ["set-google-connect", "set-google-reconnect", "set-google-disconnect-1", "set-google-disconnect-2", "set-google-retry", "set-google-check"] {
        assert!(g.contains(&format!("id=\"{id}\"")), "the Google row draws #{id}");
        assert!(g.contains(&format!("closest(\"#{id}\")")), "the Google row binds #{id}");
    }
    assert!(g.contains("KnowluData.call(\"google_status\", {})"), "the row asks google_status");
}

/// Split (R2-1): the Settings Google row invokes `google_status`, `google_connect` and `google_disconnect`; "no
/// `open_external` outside the wizard" is `ui_shell::commands_split_by_shell_and_console`'s.
#[test]
fn the_console_invokes_the_three_google_commands_and_never_open_external_outside_the_wizard() {
    let js = code(SOURCES);
    for cmd in ["google_status", "google_connect", "google_disconnect"] {
        assert!(js.contains(&format!("KnowluData.call(\"{cmd}\"")), "sources.js never invokes {cmd}");
    }
    for f in [SOURCES, MAIL] {
        assert!(!code(f).contains("open_external"), "{f}: open_external is the wizard's alone");
    }
}

/// While polling, the state line (not the note) reads the plan's exact sentence.
#[test]
fn the_google_row_shows_the_plan_polling_sentence_in_the_state_line() {
    let g = google();
    assert!(g.contains("var GOOGLE_POLLING = \"Waiting for Google\u{2026} finish in your browser.\";"));
    let draw = fn_body(&g, "function drawGoogle(");
    assert!(draw.contains("busy ? GOOGLE_POLLING : (SET.googleWaitedOut ? GOOGLE_NO_ANSWER + \" \" : \"\") + googleSentence(g)"), "{draw}");
    assert!(draw.contains("set-google-state"), "the sentence goes to the state line: {draw}");
    let c = fn_body(&g, "async function connectGoogle(");
    assert!(!c.contains("SET.googleNote = GOOGLE_POLLING"), "the note stays empty while polling");
}

/// The poll carries `SET.googleSeq`, bumped on every new click and when Settings is left (the section's unmount).
#[test]
fn the_google_row_poll_is_cancelled_when_settings_closes() {
    let js = code(SOURCES);
    let g = google();
    let f = fn_body(&g, "async function connectGoogle(");
    assert!(f.contains("var mySeq = ++SET.googleSeq"), "every click takes a new token: {f}");
    assert!(f.matches("SET.googleSeq !== mySeq").count() >= 2, "the loop checks the token after each await: {f}");
    assert!(f.contains("SET.googlePolling = true") && f.contains("SET.googlePolling = false"), "{f}");
    let un = fn_body(&js, "function unmount(");
    assert!(un.contains("SET.googleSeq += 1") && un.contains("SET.googlePolling = false"), "leaving Settings bumps the token and ends the poll: {un}");
    assert!(g.contains("\"google_connect\", { scope:"), "the row starts the flow with google_connect");
    assert!(!js.contains("gmail.readonly"), "no scope string on the page");
}

/// The poll runs ten minutes; a student who cancelled on Google's page must be able to mint a new link, so Connect and
/// Reconnect stay enabled while polling (only a disconnect in flight locks them).
#[test]
fn the_google_row_connect_buttons_stay_enabled_while_polling() {
    let g = google();
    let c = fn_body(&g, "function googleControlsHtml(");
    assert!(c.contains("var lock = SET.leaving || busy;"), "the other controls wait out the poll: {c}");
    for id in ["set-google-connect", "set-google-reconnect"] {
        let line = c.lines().find(|l| l.contains(&format!("\"{id}\""))).unwrap_or_else(|| panic!("{id} is drawn"));
        assert!(line.contains("SET.leaving") && !line.contains("lock"), "#{id} is locked only by a disconnect in flight: {}", line.trim());
    }
    for id in ["set-google-disconnect-1", "set-google-retry", "set-google-check"] {
        let line = c.lines().find(|l| l.contains(&format!("id=\"{id}\""))).unwrap_or_else(|| panic!("{id} is drawn"));
        assert!(line.contains("lock"), "#{id} waits out the poll: {}", line.trim());
    }
}

/// D6: step 1 names Calendar, step 2 is the only call to `google_disconnect`, and leaving Settings puts the confirm away.
#[test]
fn the_disconnect_confirm_names_calendar() {
    let js = code(SOURCES);
    let g = google();
    let text = g.find("var GOOGLE_DISCONNECT_CONFIRM").expect("the confirm text is one constant");
    assert!(g[text..text + 120].contains("This disconnects Google Calendar too"), "step 1 opens with the spec's sentence");
    assert!(g.contains("closest(\"#set-google-disconnect-1\")") && g.contains("closest(\"#set-google-disconnect-2\")"), "both steps are wired");
    let step2 = g.find("closest(\"#set-google-disconnect-2\")").expect("step 2 handler");
    assert!(g[step2..(step2 + 80).min(g.len())].contains("disconnectGoogle()"), "step 2 runs disconnectGoogle");
    assert_eq!(js.matches("\"google_disconnect\"").count(), 1, "one call to google_disconnect");
    assert!(fn_body(&g, "async function disconnectGoogle(").contains("KnowluData.call(\"google_disconnect\""), "from disconnectGoogle only");
    assert!(fn_body(&js, "function unmount(").contains("SET.confirming = false"), "leaving Settings hides step 2");
}

/// Live proof 2026-10-02 (F2): the row polls until the consent link's own lifetime, backing off, then offers "Check
/// again", which re-reads the status and never mints a new link.
#[test]
fn the_google_row_waits_out_the_link_lifetime_then_offers_check_again() {
    let js = code(SOURCES);
    let g = google();
    assert!(g.contains("var GOOGLE_LINK_LIFETIME_MS = 600000;"), "ten minutes, the nonce's lifetime");
    assert!(g.contains("var GOOGLE_NO_ANSWER = \"Didn\u{2019}t hear back from Google.\";"));
    let f = fn_body(&g, "async function connectGoogle(");
    assert!(!f.contains("i < 20"), "no fixed count of polls: {f}");
    assert!(f.contains("GOOGLE_LINK_LIFETIME_MS") && f.contains("Date.now()"), "{f}");
    assert!(f.contains("delay = Math.min("), "the wait grows between polls: {f}");
    assert!(f.contains("SET.googleWaitedOut = !googleDone("), "{f}");
    let c = fn_body(&g, "function googleControlsHtml(");
    let check = c.lines().find(|l| l.contains("id=\"set-google-check\"")).expect("Check again is drawn");
    assert!(check.contains("SET.googleWaitedOut") && check.contains(">Check again<"), "shown only after the wait ran out: {}", check.trim());
    assert!(fn_body(&g, "function drawGoogle(").contains("GOOGLE_NO_ANSWER"));
    let again = fn_body(&g, "async function checkGoogleAgain(");
    assert!(again.contains("KnowluData.call(\"google_status\"") && !again.contains("google_connect"), "re-polls status, never re-mints");
    assert!(g.contains("closest(\"#set-google-check\")) { checkGoogleAgain(); return; }"));
    assert_eq!(js.matches("\"google_connect\", { scope: scope }").count(), 1, "only connectGoogle mints a link");
}

/// L7 (the ruling of 2026-10-19, plan Q2 and Q3): a connected Gmail the service says went quiet reads as the quiet sentence.
#[test]
fn the_settings_google_row_names_a_quiet_gmail() {
    let body = fn_body(&google(), "function googleSentence(").to_string();
    assert!(body.contains("g.state === \"quiet\""), "googleSentence branches on the quiet state: {body}");
    assert!(
        body.contains(", but Knowlu has found no mail in it for a week. If this is the wrong account, disconnect it and connect the right one."),
        "the quiet sentence is the spec's words: {body}"
    );
}

/// Blackboard first (the mockup's order), then the Google row, the email row directly after it, and the mailbox rows
/// and the saved-sign-in switch under the email row.
#[test]
fn the_google_row_and_the_email_row_sit_side_by_side() {
    let markup = fn_body(&code(SOURCES), "function sectionHtml(").to_string();
    let at = |id: &str| markup.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("the section has no #{id}"));
    let (g, e) = (at("set-google"), at("set-email"));
    assert!(at("set-grades") < g, "Blackboard comes first");
    assert!(g < e && markup[g..e].matches("class=\"st-row").count() == 1, "the email row follows the Google row directly");
    assert!(e < at("set-email-boxes") && at("set-email-boxes") < at("set-email-keeprow"), "the mailboxes sit under the email row, then the switch");
}

/// Split (R2-1), positive half of `the_launch_account_check_does_not_reach_google_status` and of
/// `ui_shell::google_status_only_on_settings_mount` (M5 as shipped, P4): the Settings Google row reads `google_status`
/// when Settings mounts, and only then: from the section's `account` handler (the frame's mount read of
/// `account_status`), guarded on a signed-in answer, with the token bumped when Settings is left.
#[test]
fn the_google_row_reads_google_status_on_settings_mount() {
    let js = code(SOURCES);
    let reg = js.find("KnowluRouter.addSection(\"sources\",").expect("the section registers");
    let module = &js[reg..];
    assert!(module.contains("account: function (a) { gateGoogleRow(a); }"), "the frame's account answer gates the row");
    let gate = fn_body(&js, "function gateGoogleRow(");
    let guard = gate.find("a && a.ok && !a.needs_account").expect("signed-in students only");
    let load = gate.find("loadGoogleRow()").expect("the gate loads the row");
    assert!(guard < load, "the signed-in guard comes before loadGoogleRow: {gate}");
    assert!(gate.contains("SET.googleSeq += 1") && gate.contains("hidden = true"), "no account: the row hides and a late answer draws nothing");
    let load = fn_body(&js, "function loadGoogleRow(");
    assert!(load.contains("var seq = ++SET.googleSeq") && load.contains("seq !== SET.googleSeq"), "a stale answer never repaints the row");
    // google_status is asked only from the row's own functions, never at launch.
    for (n, line) in js.lines().enumerate().filter(|(_, l)| l.contains("\"google_status\"")) {
        assert!(line.contains("KnowluData.call(\"google_status\", {})"), "line {}: google_status only through KnowluData.call: {}", n + 1, line.trim());
    }
    let calls = js.matches("\"google_status\"").count();
    let in_row = ["function loadGoogleRow(", "async function connectGoogle(", "async function checkGoogleAgain("].iter().map(|h| fn_body(&js, h).matches("\"google_status\"").count()).sum::<usize>();
    assert_eq!(calls, in_row, "google_status is asked from the row's load, connect and check-again only");
    for f in [SOURCES, MAIL] {
        assert!(!code(f).contains("KnowluBoot.onConsole(function () { load") , "{f}: no launch hook loads the row");
    }
    assert!(!code(MAIL).contains("google_status"), "the mail sheet never asks Google");
    assert!(!js.contains("KnowluBoot"), "the Sources section has no launch hook of its own");
}

// ---- grades (re-homed, plan §7: now the Sources row) ---------------------------------------------

/// The Blackboard row's four connection states (not available, not connected, signed out, connected), read from
/// `grades_status` alone; with no grades in `state.grades` yet, a connected row says they come after the next refresh.
/// A signed-out session is a named state in the page's quiet voice, never amber or red.
#[test]
fn the_grades_strip_renders_from_state_grades_and_has_its_four_states() {
    let gr = grades();
    let draw = fn_body(&gr, "function drawGrades(");
    assert!(draw.contains("KnowluData.state()") && draw.contains("st.grades"), "the row reads state.grades: {draw}");
    assert!(draw.contains("s.available === false") && draw.contains("gradesUnavailableHtml("), "the not-available state");
    assert!(draw.contains("!s.connected") && gr.contains("Connect Blackboard to see your grades"), "the not-connected state");
    assert!(draw.contains("s.signed_out") && gr.contains("Sign in to Blackboard again to update grades"), "the signed-out state");
    assert!(gr.contains("Blackboard is connected. Your grades appear after the next refresh."), "connected, nothing read yet");
    assert!(draw.contains("KnowluFmt.read(s.fetched_at)"), "connected: when it was read, by the one date rule");
    for bad in ["st-bad", "st-warn", "error", "amber"] {
        assert!(!draw.contains(bad), "the Blackboard row is never amber or red ({bad}): {draw}");
    }
}

#[test]
fn the_not_available_state_is_one_line_and_no_button() {
    let gr = grades();
    let line = fn_body(&gr, "function gradesUnavailableHtml(");
    assert!(line.contains("Grades from Blackboard are not available at your school yet"), "{line}");
    assert!(!line.contains("<button") && !line.contains("<input") && !line.contains("<a "), "no control on the not-available line: {line}");
    assert!(gr.contains("KnowluData.call(\"grades_status\", {})"), "the row asks grades_status");
    for word in ["campus", "unitid", "lms_host", "policy_read", "lms_kind"] {
        assert!(!gr.contains(word), "the grades row names {word}: the page never decides availability itself");
    }
}

#[test]
fn the_console_has_no_address_entry_and_never_names_grades_set_host() {
    for f in [SOURCES, MAIL, CSS] {
        assert!(!read(f).contains("grades_set_host"), "{f} names grades_set_host");
    }
    let gr = grades();
    assert!(!gr.contains("<input") && !gr.contains("prompt("), "no address input in the grades row");
    for id in ["grades-host", "grades-address", "grades-url"] {
        assert!(!code(SOURCES).contains(id), "sources.js carries {id}");
    }
    for call in ["KnowluData.call(\"grades_status\"", "gradesAct(\"grades_connect\"", "gradesAct(\"grades_refresh\"", "KnowluData.call(\"grades_forget\""] {
        assert!(gr.contains(call), "{call} is called");
    }
    assert!(fn_body(&gr, "function gradesAct(").contains("KnowluData.call(cmd, {})"), "gradesAct forwards to the named command");
}

#[test]
fn the_grades_buttons_and_the_settings_row_are_in_the_page() {
    let gr = grades();
    for hook in ["data-grades-connect", "data-grades-refresh"] {
        assert!(gr.contains(hook), "{hook}");
    }
    assert!(gr.contains("KnowluData.call(\"grades_forget\""), "Forget calls grades_forget");
    let markup = fn_body(&code(SOURCES), "function sectionHtml(").to_string();
    assert!(markup.contains("id=\"set-grades\"") && markup.contains("id=\"set-grades-note\""), "the Blackboard row and its note");
    assert!(gr.contains("id=\"set-grades-forget\"") && gr.contains("Forget Blackboard sign-in"), "Settings carries Forget Blackboard sign-in");
    let css = strip_css_comments(&read(CSS));
    assert!(css.contains(".st-src") && css.contains(".st-logo"), "the source rows are styled");
}

// ---- the mail progress view (re-homed, plan §7) --------------------------------------------------

#[test]
fn the_progress_view_shows_one_row_per_step_with_its_state() {
    let js = code(MAIL);
    let v = progress();
    for id in ["mail-progress", "mp-sentence", "mp-rows", "mp-backfill", "mp-ask", "mp-end", "mp-leave", "mp-stop"] {
        assert!(js.contains(&format!("id=\"{id}\"")), "the sheet has #{id}");
    }
    for step in ["sign_in", "step_aside", "forwarding", "add_address", "confirm_address", "turn_on", "canary", "backfill", "finish"] {
        assert!(v.contains(&format!("{step}:")), "label for {step}");
    }
    for st in ["waiting", "working", "done", "needs_you", "done_by_hand", "skipped", "failed"] {
        assert!(v.contains(&format!("{st}:")), "label for state {st}");
    }
    assert!(v.contains("p.rows.forEach") && v.contains("class=\"mp-row mp-"), "one row per step, the state as a class");
    assert!(v.contains("KnowluData.call(\"mail_setup_progress\", {})"), "polled");
}

#[test]
fn needs_you_names_its_reason_and_offers_its_answers() {
    let v = progress();
    for r in ["guided", "forwarding_elsewhere", "not_found", "script_failed", "blocked"] {
        assert!(v.contains(&format!("{r}:")), "reason {r}");
    }
    assert!(v.contains("row.reason") && v.contains("MP_REASON["));
    assert!(v.contains("KnowluData.call(\"mail_setup_answer\", { choice: \"done\" })"), "I've done this");
    assert!(v.contains("{ choice: \"retry\" }") && v.contains("{ choice: \"replace\" }") && v.contains("{ choice: \"stop\" }"));
    assert!(v.contains("{ choice: { school_address:"), "the one question of an uncurated school");
    assert!(v.contains("personal_gmail") && v.contains("KnowluData.call(\"mail_setup_start\", { provider: \"gmail\" })"));
}

/// T18c (2026-10-05): a sign-in that fails is a failed row with a reason in sign-in's own words.
#[test]
fn a_failed_sign_in_row_says_why_in_its_own_words() {
    let v = progress();
    let words = v.split("var MP_STEP_REASON = { sign_in: {").nth(1).expect("sign-in's own reasons").split("} };").next().unwrap();
    for r in ["not_found:", "script_failed:"] {
        assert!(words.contains(r), "sign-in reason {r}");
    }
    assert!(words.contains("address"), "not_found on sign-in names the mailbox address it could not read");
    assert!(v.contains("(MP_STEP_REASON[row.step] || {})[row.reason] || MP_REASON[row.reason]"), "a row reads its step's own reasons first");
}

#[test]
fn a_sign_in_the_service_refuses_says_so_and_points_at_settings() {
    let v = progress();
    let words = v.split("var MP_STEP_REASON = { sign_in: {").nth(1).expect("sign-in's own reasons").split("} };").next().unwrap();
    let already = words.split("already_connected:").nth(1).expect("already_connected has words").split("\",").next().unwrap();
    assert!(already.contains("already set up in Knowlu"), "{already}");
    assert!(already.contains("Settings") && already.contains("Email"), "it points at Settings: {already}");
    assert!(already.contains("resume") && already.contains("turn it off"), "{already}");
    assert!(words.contains("refused:"), "a refusal of another kind has words too");
}

#[test]
fn a_refused_sign_in_never_promises_that_starting_again_fixes_it() {
    let v = progress();
    let words = v.split("var MP_STEP_REASON = { sign_in: {").nth(1).expect("sign-in's own reasons").split("} };").next().unwrap();
    let refused = words.split("refused:").nth(1).expect("refused has words").split('"').nth(1).unwrap();
    assert!(!refused.contains("again"), "{refused}");
    assert!(refused.contains("below"), "it points at the service's reason: {refused}");
    assert!(v.contains("if (!kept && p.rows.some(function (x) { return x.reason === \"already_connected\"; })) { end = \"\"; }"), "the already_connected line is dropped");
    assert!(v.contains("var kept = p.kept === true;"), "kept reads the flag");
    assert!(!v.contains("was kept because Settings showed it"), "no text match on the kept line");
}

#[test]
fn an_ended_setup_never_shows_the_same_sentence_twice() {
    assert!(progress().contains("end = p.line && p.line !== p.sentence ? p.line : \"\";"), "the line is shown only when it adds something");
}

/// A guided step: the Knowlu address with Copy (the console's own copy command) and the provider's path, four lines at most.
#[test]
fn a_guided_step_shows_the_address_with_copy_and_its_path() {
    let v = progress();
    assert!(v.contains("data-mp-copy") && v.contains("KnowluData.call(\"copy_text\", { text: MPA.address })"), "the address with a Copy button");
    assert!(v.contains("KnowluData.call(\"mail_status\", {})") && v.contains("p.address_id"), "the address is read from the service by id");
    assert!(v.contains("MP_PATH[p.provider]") && v.contains("row.step"), "the path is keyed on provider and step");
    for prov in ["m365:", "google:", "other:"] {
        assert!(v.contains(prov), "a path for {prov}");
    }
    let paths = &v[v.find("var MP_PATH = {").unwrap()..v.find("var MP_PATH_CONFIRM").unwrap()];
    for line in paths.lines().filter(|l| l.contains('[')) {
        assert!(line.matches("\", \"").count() < 4, "four lines or fewer: {line}");
    }
}

#[test]
fn the_backfill_row_names_its_start_date_and_asks_nothing() {
    let v = progress();
    assert!(v.contains("mail from the last 120 days, since "), "the plan's copy");
    assert!(v.contains("b.since"));
    assert!(!v.contains("type=\"date\"") && !v.contains("term start") && !v.contains("confirm the date"), "no date is asked");
}

/// Leave closes the sheet and never stops the setup; Settings > Sources > Email carries the progress.
#[test]
fn leave_closes_the_panel_and_settings_carries_the_progress() {
    let v = progress();
    let leave = fn_body(&v, "function mailLeave(");
    assert!(leave.contains("KnowluSheets.close(\"mail\")"), "Leave closes the mail sheet: {leave}");
    assert!(!leave.contains("mail_setup_stop"), "leaving never stops the setup");
    assert!(v.contains("closest(\"#mp-leave\")) { mailLeave(); return; }"));
    let src = code(SOURCES);
    let markup = fn_body(&src, "function sectionHtml(");
    assert!(markup.contains("id=\"set-email\"") && markup.contains("id=\"set-email-state\"") && markup.contains("id=\"set-email-c\""));
    let m = mail_settings();
    assert!(m.contains("id=\"set-email-open\"") && m.contains("closest(\"#set-email-open\")"), "Show progress opens the sheet");
    assert!(fn_body(&src, "function mount(").contains("loadEmail()"), "Settings reads the progress when it opens");
    assert!(fn_body(&m, "function loadEmail(").contains("KnowluData.call(\"mail_setup_progress\", {})"));
}

#[test]
fn stop_invokes_mail_setup_stop_and_shows_what_was_done() {
    let v = progress();
    assert_eq!(code(MAIL).matches("\"mail_setup_stop\"").count(), 1);
    assert!(v.contains("KnowluData.call(\"mail_setup_stop\", {})"));
    assert!(v.contains("p.done") && v.contains("p.left") && v.contains("not_brought_in_before"));
    assert!(v.contains(" was not brought in"));
    assert!(v.contains("closest(\"#mp-stop\")"));
}

#[test]
fn backfill_shows_n_of_m_and_a_time_left() {
    let v = progress();
    assert!(v.contains("b.handled") && v.contains("b.expected") && v.contains(" of "));
    assert!(v.contains("b.eta_secs") && v.contains(" left"));
    assert!(v.contains("b.paused"));
}

/// D6: the wizard's announcement was taken and setup has not begun: it begins at launch, in this window. Two calls start
/// a school setup: the first open and Settings' Connect school email.
#[test]
fn the_consoles_first_open_starts_school_mail_setup_when_offer_pending() {
    let js = code(MAIL);
    assert!(js.contains("KnowluBoot.onConsole(function () {"), "a launch hook, on the console path only");
    let f = fn_body(&js, "function mailFirstOpen(");
    assert!(f.contains("KnowluData.call(\"mail_status\", {})") && f.contains("offer_pending") && f.contains("setup_running"));
    assert!(f.contains("KnowluData.call(\"mail_setup_start\", { provider: \"school\" })") && f.contains("openMailProgress()"));
    let both = js.matches("\"mail_setup_start\", { provider: \"school\" }").count() + code(SOURCES).matches("\"mail_setup_start\", { provider: \"school\" }").count();
    assert_eq!(both, 2, "first open and the Settings offer");
}

#[test]
fn the_progress_view_never_reaches_ui_event_and_every_string_goes_through_h() {
    let v = progress();
    assert!(ev_lines(&v).is_empty(), "no usage events from the mail view: {:?}", ev_lines(&v));
    assert!(v.contains("textContent = p.sentence || \"\""), "the engine's sentence is set as text");
    assert!(!v.contains("innerHTML = p.") && !v.contains("+ p.sentence"), "never markup from the progress");
    for f in [MAIL, SOURCES] {
        let bad = markup_violations(&read(f));
        assert!(bad.is_empty(), "{f}: markup built outside h() or a constant: {bad:?}");
    }
}

// ---- Settings > Sources > Email (re-homed, plan §7) ----------------------------------------------

#[test]
fn settings_email_lists_each_mailbox_with_its_state_and_its_address_with_copy() {
    let m = mail_settings();
    let markup = fn_body(&code(SOURCES), "function sectionHtml(").to_string();
    assert!(markup.contains("id=\"set-email-boxes\"") && markup.contains("id=\"set-email-keeprow\""));
    assert!(m.contains("id=\"set-email-keep\""), "the saved sign-in switch");
    assert!(m.contains("KnowluData.call(\"mail_status\", {})") && m.contains("s.mailboxes.map(mailBoxHtml)"), "one row per mailbox");
    for st in ["pending", "confirmed", "active", "quiet", "blocked", "off", "paused_send", "lapsed", "revoked"] {
        assert!(m.contains(&format!("{st}:")), "a sentence for state {st}");
    }
    assert!(m.contains("m.address") && m.contains("data-em-copy") && m.contains("KnowluData.call(\"copy_text\", { text: addr })"), "the address with Copy");
    assert!(m.contains("m.turned_off") && m.contains("m.open_backfill"));
    assert!(m.contains("recent mail"), "an open backfill is said");
    assert!(m.contains("m.provider === \"m365\"") && m.contains("New mail is arriving. Knowlu will bring in older Outlook mail on a later run."), "a Microsoft 365 backfill says so and offers no Resume");
}

#[test]
fn resume_rotate_and_the_saved_sign_in_switch_invoke_their_commands() {
    let m = mail_settings();
    assert!(m.contains("KnowluData.call(\"mail_sign_in\", { addressId: id })"), "Resume");
    assert!(m.contains("KnowluData.call(\"mail_rotate\", { addressId: id })"), "Rotate");
    assert!(m.contains("KnowluData.call(\"mail_keep_session\", { on: "), "the switch");
    assert!(m.contains("profiles_left"), "a profile the switch could not delete is named");
    let rows = fn_body(&m, "function mailBoxHtml(");
    assert!(rows.contains("emStartHtml(\"Resume\", \"resume\"") && rows.contains("emStartHtml(\"Rotate\", \"rotate\""));
}

#[test]
fn turn_off_email_is_two_steps_with_the_confirm_text() {
    let m = mail_settings();
    assert!(m.contains("Knowlu turns off forwarding in this mailbox, stops reading it and deletes what it holds to serve it. What it already added stays in your plan."));
    assert!(m.contains("emBtnHtml(\"Turn off email\", \"off1\"") && m.contains("emBtnHtml(\"Yes, turn off email\", \"off2\""));
    assert_eq!(m.matches("KnowluData.call(\"mail_disconnect\", { addressId: id })").count(), 1, "only the second press invokes it");
    let first = m.split("act === \"off1\"").nth(1).unwrap().split("act ===").next().unwrap();
    assert!(!first.contains("mail_disconnect"), "the first press only reveals the second");
}

#[test]
fn turn_off_email_names_a_forwarding_address_it_left_for_the_student() {
    let m = mail_settings();
    assert!(m.contains("remove_address") && m.contains("needs_you"));
    assert!(m.contains("Knowlu\u{2019}s forwarding address is still in your Gmail forwarding list"), "says it is still there");
    assert!(m.contains("Settings, then See all settings, then Forwarding and POP/IMAP"), "names where to remove it");
    assert!(m.contains("r.address") && m.contains("data-em-copy"), "the address is shown to find it");
}

#[test]
fn an_unbound_reason_shows_amber_with_its_count_and_rotate_only_when_suggested() {
    let m = mail_settings();
    assert!(m.contains("m.unbound_reason") && m.contains("m.unbound_note") && m.contains("m.unbound_since_live"));
    assert!(m.contains("em-amber") && strip_css_comments(&read(CSS)).contains(".em-amber"));
    assert!(m.contains("m.suggest_rotate === true") && m.contains("m.suggest_rotate !== true"), "Rotate sits in the amber line only when suggested");
    assert!(ev_lines(&m).is_empty(), "nothing mail-derived reaches usage events");
    assert!(!m.contains("innerHTML = m.") && !m.contains("innerHTML += m."), "every mail string goes through h()");
}

// ---- "From your email" (re-homed, plan §7; a second view inside the mail sheet, §11 item 4) -------

#[test]
fn from_your_email_lists_by_course_items_created_fields_changed_past_archived_and_notices() {
    let js = code(MAIL);
    let v = applied();
    for id in ["mail-applied", "ma-body", "ma-close"] {
        assert!(js.contains(&format!("id=\"{id}\"")), "the sheet has #{id}");
    }
    assert!(v.contains("KnowluData.call(\"mail_applied\", {})"), "the engine's read-only list");
    assert!(v.contains("r.courses.forEach") && v.contains("g.course"), "by course");
    assert!(v.contains("g.created") && v.contains("g.changed") && v.contains("g.archived.count") && v.contains("g.archived.items") && v.contains("g.notices"));
    assert!(v.contains("<details") && v.contains("<summary"), "the archived count expands");
    for part in ["Items created", "Fields changed", "Past items archived", "Notices opened"] {
        assert!(v.contains(part), "heading {part}");
    }
    assert!(v.contains("x.sender") && v.contains("x.date") && v.contains("x.old") && v.contains("x.new") && v.contains("x.due"));
    assert!(!v.contains("innerHTML = x.") && !v.contains("innerHTML += x."), "every mail string goes through h()");
}

#[test]
fn undo_invokes_delete_note_set_fields_with_the_old_value_or_close_info() {
    let v = applied();
    assert!(v.contains("KnowluData.call(\"delete_note\", { id: row.id })"), "a created item");
    assert!(v.contains("KnowluData.call(\"set_fields\", { id: row.id, fields: fields })") && v.contains("fields[row.field] = row.old"), "a changed field, the journal's old value");
    assert!(v.contains("KnowluData.call(\"close_info\", { id: row.id })"), "a notice");
    assert!(v.contains("Could not undo that: ") && v.contains("KnowluKit.refusal("), "a refused Undo is said");
    assert!(ev_lines(&v).is_empty(), "no usage event carries mail text");
}

#[test]
fn a_changed_since_row_shows_no_undo() {
    let v = applied();
    assert!(v.contains("x.changed_since ?") && v.contains("changed since"), "the fixed words");
    assert!(v.contains("data-ma-undo"), "the Undo button");
    let row = fn_body(&v, "function maRowHtml(");
    assert!(row.contains("changed_since") && row.find("changed since").unwrap() < row.find("data-ma-undo").unwrap(), "the branch picks one or the other");
}

/// "From your email" opens from the progress view, from Settings, and from Home through `window.KnowluMail.openApplied()`.
#[test]
fn from_your_email_opens_from_the_last_progress_row_and_from_settings() {
    let js = code(MAIL);
    assert!(js.contains("id=\"mp-applied\"") && js.contains("closest(\"#mp-applied\")) { openMailApplied(); return; }"));
    assert!(js.contains("window.KnowluMail = { openProgress: openMailProgress, openApplied: openMailApplied };"), "Home and Settings open the sheet through window.KnowluMail");
    let m = mail_settings();
    assert!(m.contains("id=\"set-email-applied\"") && m.contains("closest(\"#set-email-applied\")"), "Settings' From your email");
    assert!(m.contains("window.KnowluMail.openApplied()"));
    assert!(strip_css_comments(&read(CSS)).contains(".ma-row"));
}

/// §11 item 4: "From your email" is a second view inside the one mail sheet, so two sheets never stack. Opening either view
/// while the sheet is open switches it in place; only a closed sheet is requested from the shell.
#[test]
fn from_your_email_opens_above_the_progress_panel() {
    let js = code(MAIL);
    assert_eq!(js.matches("KnowluSheets.define(\"mail\",").count(), 1, "one mail sheet");
    assert!(!js.contains("KnowluSheets.request(\"report\"") && !js.contains("KnowluSheets.request(\"applied\""), "no second sheet");
    for head in ["function openMailProgress(", "function openMailApplied("] {
        let f = fn_body(&js, head);
        let host = f.find("if (MP.host)").unwrap_or_else(|| panic!("{head} checks for an open sheet"));
        let req = f.find("KnowluSheets.request(\"mail\")").unwrap_or_else(|| panic!("{head} requests a closed sheet"));
        assert!(host < req, "{head} switches an open sheet in place before asking for one");
        assert!(f.contains("showView("), "{head} shows its view");
    }
    let show = fn_body(&js, "function showView(");
    assert!(show.contains("mail-progress") && show.contains("mail-applied") && show.contains(".hidden"), "one view at a time: {show}");
}

#[test]
fn the_held_backfill_step_says_so_and_shows_no_forwarding_guide() {
    let g = fn_body(&progress(), "function mpGuideHtml(").to_string();
    let at = g.find("row.step === \"backfill\"").expect("mpGuide has a backfill branch");
    assert!(g[..at].contains("var row ="), "the branch reads the guided row");
    assert!(g[at..].starts_with("row.step === \"backfill\" && p.provider === \"m365\""), "only Microsoft 365 holds the backfill");
    assert!(g.contains("Knowlu can\u{2019}t bring in Outlook\u{2019}s older mail yet. New mail is already arriving \u{2014} press I\u{2019}ve done this to finish."), "the backfill sentence");
    let branch = &g[at..g.find("var lines =").expect("the path lines")];
    assert!(branch.contains("return"), "the backfill step returns its own sentence before any address or path");
    assert!(branch.find("return").unwrap() < branch.find("mpAddressFor(").unwrap(), "and before the address is even fetched");
    assert!(!branch.contains("data-mp-copy") && !branch.contains("MP_PATH"), "no address, Copy or forwarding path on the backfill step");
}

// ---- the update-owed lock and the first open (re-homed and new) ----------------------------------

/// The first-open setup waits for the account check (M10) and stands down while the update is owed; the progress
/// view's Gmail switch is a setup start too.
#[test]
fn no_email_setup_starts_while_the_update_is_owed() {
    let js = code(MAIL);
    assert!(fn_body(&js, "function mailFirstOpen(").contains("KnowluKit.isOwed()"), "mailFirstOpen reads the owed flag");
    let hook = js.split("KnowluBoot.onConsole(function () {").nth(1).expect("the launch hook");
    let hook = &hook[..hook.find("});").expect("the hook closes")];
    assert!(hook.contains("KnowluAccount.after("), "mailFirstOpen runs only after the account and privacy check settles: {hook}");
    let line = js.lines().find(|l| l.contains("id=\"mp-gmail\"")).expect("mp-gmail is built in mail.js");
    assert!(line.contains("data-start") && line.contains("KnowluKit.owed()"), "#mp-gmail is a start control: {}", line.trim());
}

/// M10: mail's first open runs after the launch account check (T20's `carried/account.js`, which asks `account_status`
/// and, for a signed-in vault, `privacy_status`, through `KnowluData.call` like every command). Review M2: it waits on
/// `KnowluAccount.after(fn)`, which runs once that check has settled, however long the privacy read takes, and polls
/// nothing; it reads the answers with `KnowluData.last`, asks neither itself (the privacy read calls the network), and
/// starts nothing when the check did not answer.
#[test]
fn mail_first_open_runs_after_the_account_check() {
    let js = code(MAIL);
    let ready = fn_body(&js, "function accountAnswered(");
    assert!(ready.contains("KnowluData.last(\"account_status\")") && ready.contains("KnowluData.last(\"privacy_status\")"), "it reads the account check's answers: {ready}");
    assert!(ready.contains("a.needs_account"), "an install with no account has no privacy answer to wait for: {ready}");
    assert!(!js.contains("ACCOUNT_WAIT_MS") && !js.contains("function afterAccountCheck("), "no timer of its own: the account check says when (M2)");
    for cmd in ["account_status", "privacy_status"] {
        assert!(!js.contains(&format!(".call(\"{cmd}\"")), "mail.js never asks {cmd} itself: the account check does");
    }
    let hook = js.split("KnowluBoot.onConsole(function () {").nth(1).expect("the launch hook");
    let hook = &hook[..hook.find("});").expect("the hook closes")];
    let a = hook.find("KnowluAccount.after(").expect("the hook waits for the check");
    let m = hook.find("mailFirstOpen()").expect("then opens");
    assert!(a < m && hook.contains("accountAnswered()"), "the first open runs once the check is in, and only when it answered: {hook}");
    let f = fn_body(&js, "function mailFirstOpen(");
    assert!(f.contains("FIRST_DONE = true"), "once a launch: {f}");
}

/// Review M9: the mail sheet is named by the shell's one dialog (`#sheet`, which copies the inner `aria-labelledby`), so it
/// draws no dialog of its own, and switching views points `#sheet`'s name at the view's heading.
#[test]
fn the_mail_sheet_nests_no_dialog_and_names_the_sheet() {
    let js = code(MAIL);
    let sheet = js.split("var SHEET_HTML = ").nth(1).and_then(|s| s.split(";\n").next()).expect("SHEET_HTML");
    assert!(!sheet.contains("role=\"dialog\"") && !sheet.contains("aria-modal"), "no second dialog inside #sheet: {sheet}");
    assert!(sheet.contains("aria-labelledby=\"mp-h\""), "the sheet names its first heading for the shell to copy");
    let view = fn_body(&js, "function showView(");
    assert!(view.contains("MP.host.setAttribute(\"aria-labelledby\""), "a view switch renames #sheet: {view}");
}

/// Review M5: while the privacy update is owed, Sources says in words why its start controls are locked (the kit's one
/// line), redrawn whenever the flag changes.
#[test]
fn sources_say_why_their_start_controls_are_locked() {
    let js = code(SOURCES);
    let owed = fn_body(&js, "function drawOwed(");
    assert!(owed.contains("KnowluKit.isOwed()") && owed.contains("KnowluKit.OWED_LINE"), "the kit's line while owed: {owed}");
    assert!(fn_body(&js, "function sectionHtml(").contains("id=\"set-src-owed\""), "at the top of Sources");
    assert!(fn_body(&js, "function drawAll(").contains("drawOwed()"), "drawn with the section and on every redraw");
}

/// Note §5 (I5): every control that starts a source or a setup carries `data-start` and the update-owed lock through
/// the one kit helper (`KnowluKit.owed()` inside `KnowluKit.owedHtml`); and the Sources section draws again when the
/// flag changes (the frame's `KnowluSettings.redraw()` reaches its `redraw`).
#[test]
fn start_controls_use_kit_owed() {
    let mut total = 0usize;
    for (f, min) in [(SOURCES, 7usize), (MAIL, 2usize)] {
        let js = code(f);
        let lines: Vec<&str> = js.lines().filter(|l| l.contains("data-start")).collect();
        for line in &lines {
            assert!(line.contains("data-start' + KnowluKit.owed()") || line.contains("data-start\\\"' + KnowluKit.owed()"), "{f}: a data-start control without KnowluKit.owed(): {}", line.trim());
            assert!(line.contains("KnowluKit.owedHtml("), "{f}: a data-start control the owed tip cannot reach: {}", line.trim());
        }
        let n: usize = lines.iter().map(|l| l.matches("data-start").count()).sum();
        assert!(n >= min, "{f}: {n} start controls, expected at least {min}");
        total += n;
    }
    assert!(total >= 9);
    let js = code(SOURCES);
    let reg = &js[js.find("KnowluRouter.addSection(\"sources\",").expect("registers")..];
    assert!(reg.contains("redraw: function () { drawAll(); loadEmail(); }"), "the section draws again when the owed flag changes");
}

/// Split (R2-1), the controls half of `settings_starts_nothing_new_while_the_update_is_owed`: Google Connect and
/// Reconnect, Connect school email, a mailbox's Resume and Rotate, Blackboard's Connect, Sign in again and Refresh are
/// start controls; Disconnect, Forget, Turn off email and Stop stay free while the update is owed.
#[test]
fn sources_start_controls_are_marked_and_owed() {
    let js = code(SOURCES);
    for marker in ["id=\"set-google-connect\"", "id=\"set-google-reconnect\"", "id=\"set-email-start\"", "data-grades-connect", "data-grades-refresh"] {
        let line = js.lines().find(|l| l.contains(marker)).unwrap_or_else(|| panic!("sources.js draws {marker}"));
        assert!(line.contains("data-start") && line.contains("KnowluKit.owed()"), "{marker} is a start control: {}", line.trim());
    }
    let start = fn_body(&js, "function emStartHtml(");
    assert!(start.contains("data-start' + KnowluKit.owed()") && start.contains("KnowluKit.owedHtml("), "Resume and Rotate are start controls: {start}");
    for marker in ["id=\"set-google-disconnect-1\"", "id=\"set-google-disconnect-2\"", "id=\"set-grades-forget\""] {
        let line = js.lines().find(|l| l.contains(marker)).unwrap_or_else(|| panic!("sources.js draws {marker}"));
        assert!(!line.contains("data-start") && !line.contains("KnowluKit.owed"), "{marker} stays free while owed: {}", line.trim());
    }
    let em = fn_body(&js, "function emBtnHtml(");
    assert!(!em.contains("data-start") && !em.contains("KnowluKit.owed"), "Turn off email stays free while owed: {em}");
    let stop = code(MAIL).lines().find(|l| l.contains("id=\"mp-stop\"")).map(str::to_string).unwrap_or_default();
    assert!(!stop.contains("data-start"), "Stop stays free while owed");
    let mp = fn_body(&progress(), "function mpBtnHtml(").to_string();
    assert!(mp.contains("choice === \"stop\""), "the progress view's Keep it as it is (stop) is never locked: {mp}");
}

// ---- split from the shell's untrusted-text scans: T17's positive halves (R2-1) --------------------

/// `mail_derived_text_renders_inert`'s T17 half: a "From your email" row, its notices too, builds every mail string
/// through `h()`.
#[test]
fn from_your_email_rows_use_h() {
    let row = fn_body(&applied(), "function maRowHtml(").to_string();
    for part in ["h(x.title)", "h(x.sender)", "h(KnowluFmt.day(x.date))"] {
        assert!(row.contains(part), "{part}: {row}");
    }
    assert!(!row.contains("innerHTML") && !row.contains("insertAdjacentHTML"), "the row builder builds a string, never writes the DOM");
}

/// `no_mail_field_reaches_the_dom_without_h`'s T17 half: the applied rows read mail fields, and every read sits inside
/// `h()` or is only a test; a row's title likewise.
#[test]
fn mail_fields_checked_in_the_applied_rows() {
    let js = code(MAIL);
    let mut checked = 0usize;
    for (n, line, upto) in field_reads(&js, &MAIL_FIELDS) {
        checked += 1;
        assert!(read_is_safe(line, &upto), "line {n}: a mail field outside h(): {}", line.trim());
    }
    assert!(checked >= 2, "the scan found the applied rows' sender ({checked})");
    let mut titles = 0usize;
    for (n, line, upto) in field_reads(&js, &["title", "why"]) {
        let var = upto.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).next().unwrap_or("");
        if !var.starts_with("x.") { continue; }
        titles += 1;
        assert!(read_is_safe(line, &upto), "line {n}: {var} not inside h(): {}", line.trim());
    }
    assert!(titles >= 2, "the scan found the applied rows' titles ({titles})");
}

/// `no_ui_event_carries_mail_text`'s T17 half: the mail sheet and the Sources section send no usage event at all, so
/// no mail text, mailbox address or school address can ride one.
#[test]
fn mail_sections_send_no_mail_text() {
    for f in [MAIL, SOURCES] {
        let js = code(f);
        assert!(ev_lines(&js).is_empty(), "{f} sends a usage event: {:?}", ev_lines(&js));
    }
    let raw = read(MAIL);
    for marker in ["/* ---- mail progress view", "/* ---- end mail progress view", "/* ---- mail applied", "/* ---- end mail applied"] {
        assert!(raw.contains(marker), "mail.js keeps its section marker {marker:?}");
    }
    let raw = read(SOURCES);
    for marker in ["/* ---- mail settings", "/* ---- end mail settings"] {
        assert!(raw.contains(marker), "sources.js keeps its section marker {marker:?}");
    }
}
