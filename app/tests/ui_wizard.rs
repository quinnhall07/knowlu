//! v0.4.0 T21: the wizard's pins (note §6, A2; plan §5 T21, §7's `ui_wizard.rs` row). The ten panels move
//! out of the old `index.html` into `js/wizard.js` as one module constant, `WIZARD_HTML`, rendered into
//! `#boot`, with the same ids, function names and panel order; state stays on `WIZ`. So the re-homed pins
//! read the markup from that constant (`html()`) and the code from the rest of the file (`code()`), comments
//! stripped, so a word in prose never satisfies a pin. Where §7 or Quinn's rulings change what a pin asserts,
//! the test says so beside the change.
#[path = "support/ui_common.rs"]
mod ui_common;

use std::fs;
use ui_common::*;

/// wizard.js as shipped.
fn raw() -> String { read("js/wizard.js") }

/// wizard.js as it runs: comments stripped.
fn js() -> String { strip_comments(&raw()) }

/// The wizard's markup: the text of the one `var WIZARD_HTML =` constant, from its name to its closing `';`.
fn html() -> String {
    let js = js();
    let at = js.find("var WIZARD_HTML =").expect("wizard.js renders its panels from one constant, WIZARD_HTML (note §6)");
    let rest = &js[at..];
    let end = rest.find("';\n").expect("WIZARD_HTML ends with a quoted line and a semicolon");
    rest[..end + 2].to_string()
}

/// wizard.js's code: everything but the markup constant.
fn code() -> String { js().replacen(&html(), "", 1) }

/// The markup between `id="<from>"` and `id="<to>"`.
fn panel(html: &str, from: &str, to: &str) -> String {
    html.split(&format!("id=\"{from}\"")).nth(1).and_then(|s| s.split(&format!("id=\"{to}\"")).next()).unwrap_or_else(|| panic!("the markup from #{from} to #{to}")).to_string()
}

/// `s[from..to]`, each end moved back to a char boundary (the files hold "—" and "…").
fn cut(s: &str, from: usize, to: usize) -> &str {
    let mut a = from.min(s.len());
    while !s.is_char_boundary(a) { a -= 1; }
    let mut b = to.min(s.len());
    while !s.is_char_boundary(b) { b -= 1; }
    &s[a..b.max(a)]
}

const PANELS: [&str; 10] = ["wiz-welcome", "wiz-account", "wiz-subscribe", "wiz-mail", "wiz-vault", "wiz-calendars", "wiz-logins", "wiz-gmail", "wiz-slots", "wiz-finish"];

// ---- re-homed from static_assets.rs (plan §7) ------------------------------------------------------

/// Task 11 re-review N1 (R-C3'-exec-41): what the paste or capture did with each feed reaches the plan, as
/// the four flags `onboarding::WizardPlan` reads, and only for the value the field holds at Finish.
#[test]
fn the_wizard_carries_each_feeds_validated_and_stored_flags_into_the_plan() {
    let js = code();
    for key in ["ics_validated: icsFlags.validated", "ics_stored: icsFlags.stored",
                "personal_calendar_validated: calFlags.validated", "personal_calendar_stored: calFlags.stored"] {
        assert!(js.contains(key), "the plan must carry {key}");
    }
    assert!(js.contains("feedFlags(WIZ.icsFeed, WIZ.ics)") && js.contains("feedFlags(WIZ.calFeed, WIZ.cal)"), "the flags are for the field's value at Finish");
    assert!(js.contains("var same = !!(feed && value && feed.url === value);"), "a changed link is neither validated nor stored");
    assert!(js.contains("WIZ.icsFeed = { url: c.link.url, stored: c.stored === true };"), "the capture");
    assert!(js.contains("WIZ.icsFeed = r.ok ? { url: sent, stored: r.stored === true } : null;"), "the pasted school feed");
    assert!(js.contains("WIZ.calFeed = r.ok ? { url: sent, stored: r.stored === true } : null;"), "the pasted personal calendar");
}

#[test]
fn the_wizard_has_ten_panels_and_the_privacy_words_and_no_live_fetch() {
    let html = html();
    assert!(html.contains("id=\"wizard\""));
    // Spec §4.2, in order. No `wiz-backup`, and no folder anywhere.
    let mut last = 0usize;
    for p in PANELS {
        let at = html.find(&format!("id=\"{p}\"")).unwrap_or_else(|| panic!("panel {p}"));
        assert!(at > last, "panel {p} is out of order");
        last = at;
    }
    assert!(!html.contains("id=\"wiz-backup\""), "the backup-folder panel is gone");
    assert!(!html.contains("id=\"wiz-pick-parent\"") && !html.contains("id=\"wiz-pick-bdir\""), "no folder is picked in the wizard");
    let js = code();
    for f in ["startWizard", "renderWizard", "wizGo", "wizFinish"] {
        assert!(js.contains(&format!("function {f}(")), "missing {f}");
    }
    assert!(js.contains("var PANELS = [\"welcome\", \"account\", \"subscribe\", \"mail\", \"vault\", \"calendars\", \"logins\", \"gmail\", \"slots\", \"finish\"];"));
    // No live fetch in onboarding: the ICS shape is matched, never requested. The escape keeps the
    // network-reference rule true.
    assert!(js.contains("/^https:\\/\\/"), "the ICS check is an escaped regex");
    assert!(!js.contains("fetch(\"http"), "the page never fetches");
    assert!(js.contains("clearCredentialFields("), "the fields are cleared by name");
    // Changed (plan T5c, note D5): the old page's shots seam (`window.KNOWLU_SHOTS`) is a review aid and does
    // not ship (`ui_shell::no_mock_data_or_review_aid_ships` bans it); the harness drives the real boot. The wizard
    // registers itself for boot.js instead.
    assert!(!raw().contains("KNOWLU_SHOTS"), "no shots seam ships");
    assert!(js.contains("window.KnowluWizard = {") && js.contains("start: function (launch, host)"), "the wizard registers its start for boot.js");
    assert!(js.contains("retarget_credentials"), "Finish moves the credentials when the name changed");
    // Changed: a class `display` must never beat `[hidden]`. The old page needed `.wiz-row[hidden]`; the new
    // shell's one rule covers every hidden row (#wiz-code-row, #wiz-school-free, #wiz-lms-kind).
    assert!(strip_css_comments(&read("css/shell.css")).contains("[hidden] { display: none !important; }"), "a hidden row must actually hide");
    assert!(!js.contains("l.documents"), "the retired `documents` key is gone from the page");
    assert!(!raw().to_lowercase().contains("telemetry"), "no telemetry toggle: (c) is not built (D5)");
    assert_eq!(raw().matches(" data-id=\"").count(), raw().matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// C2 final review A-5 (m59+m60): the Google flow's own state lives on `WIZ`, rendered by `renderWizard()`.
#[test]
fn the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it() {
    let js = code();
    let html = html();
    assert!(js.contains("google: false") && js.contains("googleNote:"), "WIZ carries the Google flow's own state");
    assert!(!js.contains("var WIZ_GOOGLE"), "the old bare flag must be gone, not merely unused");
    assert!(html.contains("id=\"wiz-google-note\""), "the Google status line has its own element");
    assert!(js.contains("EL(\"wiz-google-note\").textContent = WIZ.googleNote"), "renderWizard paints it");
    assert!(js.contains("EL(\"wiz-google\").disabled = WIZ.google || WIZ.googlePolling"), "the button's disabled state is derived from WIZ state on every render");
    assert!(js.contains("googleSeq"), "a cancellation token exists");
    let go = js.find("function wizGo(").map(|i| &js[i..]).expect("wizGo");
    assert!(
        go.find("WIZ.googleSeq").map(|i| i < go.find("function wizFinish(").unwrap_or(usize::MAX)).unwrap_or(false),
        "wizGo itself bumps the token on leaving the panel"
    );
    let finish = fn_body(&js, "function wizFinish(");
    let google_connected_call = finish.find("invoke(\"google_connected\"").expect("wizFinish re-checks google_connected");
    let plan_build = finish.find("google_calendar:").expect("the plan is built somewhere in wizFinish");
    assert!(google_connected_call < plan_build, "the re-check happens BEFORE the plan is built, not after");
    assert!(finish.contains("WIZ.google"), "the polled flag is still read, as the fallback");
    assert!(!js.contains("google_calendar: WIZ_GOOGLE"), "the plan no longer reads the old bare flag directly");
    let busy_write = finish.find("WIZ.busy = true").expect("wizFinish latches WIZ.busy");
    assert!(busy_write < google_connected_call, "wiz-next is disabled before the google_connected await, not after it");
}

/// Gmail connect D11: the Gmail panel's Connect Gmail button follows the calendar button's A-5 rule.
#[test]
fn the_wizard_gmail_flow_keeps_its_state_on_wiz_and_renders_it() {
    let js = code();
    let panel = panel(&html(), "wiz-gmail", "wiz-slots");
    assert!(panel.contains("id=\"wiz-gmail-connect\""), "the panel has the button");
    assert!(panel.contains("id=\"wiz-gmail-note\""), "the status line has its own element");
    assert!(!panel.contains("Skip it for now"), "the old copy-only text is gone");
    assert!(panel.contains("While Google reviews Knowlu, this works only for invited testers"), "the Testing sentence");
    assert!(panel.contains("Knowlu can read your Gmail for things you have to do and propose each one for you to approve. You can skip this and connect later in Settings."), "the plan's what-it-does sentence");
    assert_eq!(panel.matches("read your").count(), 1, "one what-it-does sentence");
    assert!(js.contains("WIZ.gmailNote = \"Gmail is connected.\";"), "the plan's success note");
    assert!(js.contains("WIZ.gmailNote = GOOGLE_TIMEOUT;"), "the Settings timeout sentence, unchanged");
    assert!(js.contains("var GOOGLE_TIMEOUT = \"Google did not finish connecting. If Google said Knowlu is not verified, this Google account is not on the tester list yet.\";"), "the timeout sentence is the console's own, word for word");
    assert!(js.contains("gmail: false") && js.contains("gmailNote:") && js.contains("gmailPolling: false") && js.contains("gmailSeq: 0"), "WIZ carries the flow's state");
    assert!(js.contains("EL(\"wiz-gmail-note\").textContent = WIZ.gmailNote"), "renderWizard paints the note");
    assert!(js.contains("EL(\"wiz-gmail-connect\").disabled = WIZ.gmail || WIZ.gmailPolling"), "disabled is derived from WIZ");
    let go = js.find("function wizGo(").map(|i| &js[i..]).expect("wizGo");
    assert!(
        go.find("WIZ.gmailSeq").map(|i| i < go.find("function wizFinish(").unwrap_or(usize::MAX)).unwrap_or(false),
        "wizGo bumps the token on leaving the panel"
    );
}

#[test]
fn the_wizard_gmail_button_uses_the_wizard_commands() {
    let js = code();
    let h = fn_body(&js, "EL(\"wiz-gmail-connect\").addEventListener(\"click\"");
    assert!(h.contains("invoke(\"google_connect_url\", { scope: \"gmail\" })"), "the wizard command, scope gmail");
    assert!(h.contains("invoke(\"open_external\""), "the wizard's opener");
    assert!(h.contains("invoke(\"google_connected\")"), "polls the wizard's status command");
    for console_only in ["google_status", "google_connect\"", "google_disconnect"] {
        assert!(!h.contains(console_only), "no console-only command: {console_only}");
    }
}

#[test]
fn next_never_waits_on_gmail() {
    let js = code();
    let render = fn_body(&js, "function renderWizard(");
    assert!(render.contains("EL(\"wiz-next\").disabled = WIZ.busy;"), "Next depends on busy alone");
    assert!(!render.contains("wiz-next\").disabled = WIZ.busy ||") && !render.contains("gmailPolling ||  WIZ.busy"), "not on polling");
    let finish = fn_body(&js, "function wizFinish(");
    assert!(!finish.contains("gmail"), "nothing about Gmail reaches the plan");
}

/// R-OB-1: the wizard that takes a coursework password must also say what the work is for.
#[test]
fn the_logins_panel_maps_what_it_finds_to_a_course() {
    let panel = panel(&html(), "wiz-logins", "wiz-gmail");
    assert!(panel.contains("id=\"wiz-map\"") && panel.contains("id=\"wiz-map-rows\""), "the mapping block");
    let js = code();
    assert!(js.contains("\"discover_coursework\""), "discovery runs after the credentials are stored");
    assert!(js.contains("function renderMapping("), "renderMapping");
    assert!(js.contains("zybooks_courses:") && js.contains("vhl_sections:") && js.contains("course_map:"), "the plan carries the mapping");
    assert!(!js.contains("slugify"), "slugs are `knowlu_engine::ingest::slugify`'s, never the page's");
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo");
    assert!(go.contains("WIZ.discovering"), "a second Next must not start a second coursework-discover");
    assert!(go.contains("WIZ.busy = true") && go.contains("WIZ.busy = false"), "Next is disabled while discovery is in flight and re-enabled when it settles");
}

/// D5 and D6: the mapping row offers the classes the wizard already captured, a row with nothing to offer
/// says what it needs, and leaving one blank is a choice whose consequence the panel states.
#[test]
fn a_mapping_row_offers_the_captured_classes_and_says_what_a_blank_one_costs() {
    let panel = panel(&html(), "wiz-logins", "wiz-gmail");
    assert!(panel.contains("<datalist id=\"wiz-course-codes\">"), "the panel keeps one datalist");
    let js = code();
    assert!(js.contains("function renderCourseCodes("), "renderCourseCodes fills it");
    assert!(js.contains("list=\"wiz-course-codes\""), "every mapping row's field reads it");
    assert!(js.contains("c.label || c.slug"), "the datalist offers the human code, and the slug otherwise");
    assert!(js.contains("type the course this belongs to"), "a row with no suggestion says what it needs");
    assert!(js.contains(" of these will be asked about in the app"), "and Next says what blank rows cost");
    assert!(js.contains("function noteUnmapped("), "noteUnmapped");
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo/wizStep");
    assert!(go.contains("noteUnmapped()"), "the count is written on the way out of the panel");
    assert!(go.contains("WIZ.mapWarned"), "and the panel stays once, so the student reads it");
    assert!(js.contains("mapWarned: false"), "the latch starts clear on a fresh wizard");
    assert!(js.contains("WIZ.discovered = false; WIZ.mapWarned = false"), "and re-typed logins clear it with the discovery they invalidate");
    assert!(!go.contains("WIZ.error = \"Map"), "a blank row must never block Next");
}

/// Final review, I2: only rows the student ticked as ignored reach `zybooks_ignore`.
#[test]
fn a_blank_zybooks_row_is_never_sent_as_ignored() {
    let finish = fn_body(&code(), "function wizFinish(").to_string();
    assert!(finish.contains("r.source === \"zybooks\" && r.ignore; }"), "zybooks_ignore takes only rows the student ticked as ignored");
    assert!(!finish.contains("r.source === \"zybooks\" && (r.ignore || !r.course)"), "a blank, un-ignored zyBooks row must not reach zybooks_ignore");
}

/// Final review, I4: only a TYPED course (`slug: ""`) contributes its code to `course_map`.
#[test]
fn a_captured_courses_lms_id_never_gets_a_phantom_slug() {
    let finish = fn_body(&code(), "function wizFinish(").to_string();
    assert!(finish.contains("if (c.code && !c.slug)"), "only a typed course (no slug of its own) contributes its code to course_map");
}

/// Spec §4.2 step 1 and §9's minors row: one attestation, one acceptance, both linked to the text.
#[test]
fn the_account_panel_leads_with_google_asks_for_no_password_and_still_gates_on_eighteen() {
    let html = html();
    let panel = panel(&html, "wiz-account", "wiz-subscribe");
    let g = panel.find("id=\"wiz-google-signin\"").expect("the Google button");
    let e = panel.find("id=\"wiz-email\"").expect("the email field");
    assert!(g < e, "Continue with Google comes before the email field");
    assert!(panel.contains("Continue with Google"), "and says so in words");
    assert!(!panel.contains("password"), "the account panel must never carry a password field again");
    assert!(!html.contains("id=\"wiz-pw\""), "the account password is gone");
    assert!(panel.contains("id=\"wiz-magic\"") && panel.contains("Email me a code"), "one button, and it says what it does");
    assert!(!panel.contains("id=\"wiz-create\"") && !panel.contains("id=\"wiz-signin\""), "no create/sign-in split");
    assert!(panel.contains("id=\"wiz-code-row\"") && panel.contains("id=\"wiz-code-go\""), "the code row stays");
    assert!(panel.contains("id=\"wiz-18\"") && panel.contains("18 or older"));
    assert!(panel.contains("id=\"wiz-terms\"") && panel.contains("terms.html") && panel.contains("privacy.html"));
    let js = code();
    assert!(js.contains("\"google_sign_in\"") && js.contains("\"send_magic_link\"") && js.contains("\"verify_email_code\""));
    assert!(!js.contains("\"sign_up\"") && !js.contains("\"sign_in\""), "the password commands are gone from the page too");
    assert!(js.contains("ageAttested:"), "send_magic_link carries the attestation as ageAttested");
    assert!(js.contains("Tick both boxes"), "the page still says why a sign-in was refused");
}

/// F10: the two emailed-code doors get the guard the Google button has.
#[test]
fn the_wizards_email_doors_get_the_same_busy_guard_the_google_button_has() {
    let js = code();
    let render = fn_body(&js, "function renderWizard(");
    assert!(render.contains("EL(\"wiz-magic\").disabled = WIZ.busy"), "the email door is painted from WIZ.busy");
    assert!(render.contains("EL(\"wiz-code-go\").disabled = WIZ.busy"), "the code door is painted from WIZ.busy");
    let listener = fn_body(&js, "EL(\"wizard\").addEventListener(\"click\"");
    for id in ["#wiz-magic", "#wiz-code-go"] {
        let marker = format!("e.target.closest(\"{id}\")");
        let handler = listener.split(&marker).nth(1).and_then(|s| s.split("if (e.target.closest(").next()).unwrap_or_else(|| panic!("the {id} handler"));
        assert!(handler.contains("if (WIZ.busy) { return; }"), "{id} must refuse a second press while one is in flight");
        assert!(handler.contains("WIZ.busy = true"), "{id} must latch WIZ.busy before its invoke");
        assert!(handler.contains("WIZ.busy = false"), "{id} must release WIZ.busy on the outcome");
    }
}

/// R-P4a-21: the wizard's privacy paragraph and the site's are ONE sentence: the raw `<p>` line of
/// `site/privacy.html` against `var PRIVACY = "…";` in wizard.js, byte for byte (`static_assets.rs:1048-1060`).
#[test]
fn the_wizards_privacy_sentence_is_the_sites_privacy_sentence() {
    let site = fs::read_to_string(app_dir().join("../site/privacy.html")).expect("../site/privacy.html");
    let sentence = site
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("<p>") && l.contains("Your tasks and notes live in"))
        .map(|l| l.trim_start_matches("<p>").trim_end_matches("</p>").to_string())
        .expect("site/privacy.html must carry the privacy sentence in one <p>");
    assert!(code().contains(&format!("var PRIVACY = \"{sentence}\";")), "wizard.js's PRIVACY must be site/privacy.html's sentence, word for word");
    assert!(code().contains("EL(\"wiz-privacy\").textContent = PRIVACY;"), "and the welcome panel shows it");
}

/// The wizard's default folders come from `launch_state`; the page never builds a path.
#[test]
fn the_wizard_takes_its_default_folders_from_the_launch_state() {
    let js = code();
    assert!(js.contains("l.default_parent"), "the parent folder still comes from launch_state");
    assert!(!js.contains("l.default_backup"), "the wizard must not read a backups root it cannot show");
}

/// R-C1-55, C1: Tauri v2 lower-camel-cases every argument key.
#[test]
fn the_code_request_spells_its_argument_the_way_tauri_delivers_it() {
    let rust = read_app("src/account.rs");
    assert!(rust.contains("pub fn send_magic_link(email: String, age_attested: bool)"), "account::send_magic_link's signature changed: re-derive the keys the page must send");
    assert!(!rust.contains("rename_all"), "account.rs opts no command out of Tauri's camelCase");
    assert!(raw().contains("ageAttested"), "the attestation travels camel-cased");
    assert!(!raw().contains("age_attested"), "the snake_case spelling must not appear on the page at all");
    assert!(rust.contains("TOS_VERSION") && rust.contains("PRIVACY_VERSION"), "the versions are Rust's");
    for own in ["tos_version", "tosVersion", "privacy_version", "privacyVersion"] {
        assert!(!raw().contains(own), "the consent versions are Rust's, never the page's: {own}");
    }
}

/// Spec §7: Back and Next are rendered state, never a dead control. Changed (plan §7, owners' split): the
/// upgrade overlay's half (`#up-subscribe`, its two stand-down checks, "press Subscribe again") is T20's and
/// lives in `ui_account_privacy.rs`; this file pins the wizard's.
#[test]
fn the_wizards_nav_is_rendered_state_and_never_a_dead_control() {
    let js = code();
    let render = fn_body(&js, "function renderWizard(");
    assert!(render.contains("EL(\"wiz-back\").hidden = WIZ.step === 0"), "Back is hidden at step 0, never disabled");
    assert!(!render.contains("EL(\"wiz-back\").disabled"), "Back is never disabled anywhere");
    assert!(render.contains("EL(\"wiz-next\").disabled = WIZ.busy"), "Next's disabled state is rendered from WIZ.busy");
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo");
    let fin = fn_body(&js, "function wizFinish(");
    for (name, body) in [("wizGo", go), ("wizFinish", fin)] {
        assert!(!body.contains("EL(\"wiz-next\").disabled"), "{name} sets WIZ.busy, never the DOM property directly");
        assert!(body.contains("WIZ.busy"), "{name} still latches re-entry, through WIZ.busy");
    }
    assert_eq!(js.matches("EL(\"wiz-next\").disabled").count(), 1, "renderWizard is the ONLY writer of Next's disabled state");
    let valid = fn_body(&js, "function wizValid(");
    assert!(valid.contains("WIZ.step === 1 && !WIZ.accountId"), "step 1 is still gated on a session");
    assert!(valid.contains("WIZ.step === 2 && !WIZ.entitled"), "and step 2 on an entitlement");
    assert!(go.contains("!wizValid()"), "wizGo refuses a forward step wizValid refuses");
    assert!(js.contains("flashError("), "a repeated refusal is re-announced, not silently unchanged");
    for sentence in ["Sign in first.", "Finish the payment page in your browser, then come back."] {
        assert!(js.contains(sentence), "the refusal names what is missing: {sentence}");
    }
    assert!(go.contains("WIZ.step === 2 && !WIZ.entitled"), "wizGo's own pre-ask is gated on the same panel and the same field wizValid is");
    assert!(go.contains("checkEntitlement("), "wizGo re-asks the service before it refuses");
    let wiz_sub = js
        .find("if (e.target.closest(\"#wiz-sub-month\")")
        .map(|i| &js[i..])
        .and_then(|s| s.split("if (e.target.closest(\"#wiz-lms-open\")").next())
        .expect("the wiz-sub-month/year handler");
    let ask = wiz_sub.find("checkEntitlement(").expect("an entitlement ask appears in the subscribe handler");
    let open = wiz_sub.find("open_checkout").expect("open_checkout is still reachable on a no");
    assert!(ask < open, "#wiz-sub-month asks the service before it ever opens a second Checkout page");
    assert!(js.contains("Still not subscribed. When the payment page is done, press Next."), "the expiry names the way forward");
    assert_eq!(js.matches("status === \"trialing\"").count(), 1, "checkEntitlement is the ONLY copy of the entitlement status test");
    let poll = fn_body(&js, "function pollEntitlement(");
    assert!(poll.contains("WIZ.step !== 2"), "pollEntitlement's tick stands down once the student has left panel 2");
    let ce_in_poll = poll.find("checkEntitlement(").expect("pollEntitlement calls the status-and-plan helper");
    let guards: Vec<_> = poll.match_indices("WIZ.step !== 2").map(|(i, _)| i).collect();
    assert_eq!(guards.len(), 2, "pollEntitlement's stand-down is checked before AND after checkEntitlement()");
    assert!(guards[1] > ce_in_poll, "the second WIZ.step !== 2 check runs after checkEntitlement() resolves");
}

/// R-C1b-exec-10: an empty coursework discovery must not trap the student on the logins panel.
#[test]
fn an_empty_coursework_discovery_lets_next_proceed_and_says_so() {
    let js = code();
    let step = js.split("function wizStep(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizStep");
    assert!(step.contains("WIZ.map.length || WIZ.discovered"), "a finished discovery, rows or none, lets Next proceed");
    let mapping = fn_body(&js, "function renderMapping(");
    assert!(mapping.contains("!WIZ.discovered"), "the mapping panel stays up for a finished-but-empty discovery");
    assert!(js.contains("You can go on — Knowlu will try again on its first run."), "an empty discovery names the way forward rather than dead-ending");
    let onboarding_rs = read_app("src/onboarding.rs");
    for stale in ["fill them in below", "fill those in below"] {
        assert!(!raw().contains(stale), "wizard.js no longer tells the student to fill in a panel with nothing on it: {stale}");
        assert!(!onboarding_rs.contains(stale), "onboarding.rs no longer tells the student to fill in a panel with nothing on it: {stale}");
    }
    let html = html();
    let map_div = html.find("id=\"wiz-map\"").map(|i| &html[i..]).and_then(|s| s.split("</div>").next()).expect("#wiz-map");
    assert!(map_div.contains("id=\"wiz-map-heading\""), "the mapping heading has an id renderMapping can hide");
}

/// C3' Task 9, Step 5: the finish panel says what Finish's own restore found, and nothing before it ran.
#[test]
fn the_finish_panel_says_when_the_account_had_nothing_to_restore() {
    let js = code();
    let sentence_fn = fn_body(&js, "function restoreSentence(");
    for (label, sentence) in [
        ("restored", "Your account already had a vault here"),
        ("could not be reached", "could not be reached just now; your vault will fill in over the next syncs"),
        ("had no vault yet", "Your account had no vault yet"),
    ] {
        assert!(sentence_fn.contains(sentence), "{label}: the sentence is not in restoreSentence: {sentence_fn}");
    }
    assert!(sentence_fn.contains("!restored.ok"), "restoreSentence keys on the explicit ok field: {sentence_fn}");
    assert!(!sentence_fn.contains("warnings"), "restoreSentence no longer reads warnings.length at all: {sentence_fn}");
    assert!(js.contains("restoreNote: \"\""), "WIZ.restoreNote starts blank: nothing is claimed before Finish runs");
    assert!(js.contains("WIZ.restoreNote = restoreSentence("), "wizFinish reads the real result into it");
    assert!(js.contains("EL(\"wiz-restore-note\").textContent = WIZ.restoreNote"), "renderWizard paints it, like every other wizard field");
    let set_at = js.find("WIZ.restoreNote = restoreSentence(").expect("the assignment");
    let entitlement_at = js[set_at..].find("invoke(\"entitlement_now\"").map(|i| i + set_at).expect("entitlement_now follows it");
    assert!(set_at < entitlement_at, "the sentence is set before entitlement_now is ever invoked");
    let between = &js[set_at..entitlement_at];
    assert_eq!(between.matches("WIZ.restoreNote").count(), 1, "nothing between the two touches WIZ.restoreNote again: {between}");
}

/// T15.3 (email spec D6, risk 12): the panel right after sign-in and subscription announces school mail setup.
#[test]
fn the_panel_after_sign_in_reads_next_connect_your_school_email() {
    let html = html();
    assert!(html.find("id=\"wiz-mail\"").unwrap() > html.find("id=\"wiz-subscribe\"").unwrap());
    assert!(html.find("id=\"wiz-mail\"").unwrap() < html.find("id=\"wiz-vault\"").unwrap());
    assert!(panel(&html, "wiz-mail", "wiz-vault").contains("Next: connect your school email"));
    assert!(code().contains("var PANELS = [\"welcome\", \"account\", \"subscribe\", \"mail\", \"vault\", \"calendars\", \"logins\", \"gmail\", \"slots\", \"finish\"];"));
}

#[test]
fn not_now_carries_the_school_rules_sentence() {
    let p = panel(&html(), "wiz-mail", "wiz-vault");
    assert!(p.contains("id=\"wiz-mail-notnow\""), "a one-click Not now");
    let at = p.find("id=\"wiz-mail-notnow\"").unwrap();
    let rule = p.find("If your school's rules don't allow forwarding your school email, skip this.")
        .or_else(|| p.find("If your school&rsquo;s rules don&rsquo;t allow forwarding your school email, skip this."))
        .expect("risk 12's sentence");
    assert!(rule > at, "the sentence sits under Not now");
    assert!(fn_body(&code(), "function wizNotNow(").contains("WIZ.mailOffer = false"));
}

#[test]
fn finish_sends_offer_mail() {
    let js = code();
    assert!(js.contains("offer_mail: WIZ.mailOffer"), "the plan carries the choice");
    assert!(js.contains("mailOffer: true"), "the announcement defaults to on");
    assert!(!fn_body(&js, "function wizFinish(").contains("ui_event"));
}

#[test]
fn the_gmail_panel_button_reads_connect_gmail_directly_google_testers_only() {
    assert!(html().contains("id=\"wiz-gmail-connect\">Connect Gmail directly (Google testers only)</button>"));
}

const TESTER_NOTE: &str = "Your account is already set up. Nothing to pay.";
const TESTER_HEADING: &str = "You're all set";

fn dq(s: &str) -> String { s.replace('\'', "\"") }

/// L8 S5t: the subscribe step asks status and plan on entry, and a comped account never sees a payment
/// prompt. Changed (A2, plan §7): no price for anyone now, so the "a tester sees no price" half holds for
/// every account (now `the_wizard_holds_no_price_and_renders_the_offer`: v0.4.1 shows only the server's offer);
/// the tester rules stay. And (owners' split) the
/// plain `checkEntitled()` the upgrade overlay calls is T20's: wizard.js needs only the status-and-plan helper.
#[test]
fn the_subscribe_step_asks_status_and_plan_and_a_tester_sees_no_price() {
    let js = dq(&code());
    let html = html();
    let helper = fn_body(&js, "function checkEntitlement(");
    assert!(helper.contains("entitlement_now"), "the helper reads the entitlement answer");
    assert!(helper.contains("plan"), "the helper returns the plan beside the status");
    assert!(helper.contains(".catch("), "the helper answers not entitled on any failure");
    assert_eq!(js.matches(TESTER_NOTE).count(), 1, "the nothing-to-pay line is written once");
    assert!(!html.contains(TESTER_NOTE), "the markup does not carry the nothing-to-pay line");
    assert!(js.contains("Your subscription is active."), "a paid account keeps its note");
    assert!(html.contains("<h2 tabindex=\"-1\">Subscribe</h2>"), "the default heading is unchanged");
    assert_eq!(js.matches(dq(TESTER_HEADING).as_str()).count(), 1, "the tester heading is written once");
    let render = fn_body(&js, "function renderWizard(");
    assert!(render.contains("WIZ.plan === \"tester\""), "renderWizard branches on the recorded plan");
    let note_at = render.find("EL(\"wiz-sub-note\")").expect("the note is set in renderWizard");
    let note_stmt = &render[note_at..render[note_at..].find(";\n").map(|i| note_at + i).unwrap()];
    assert!(note_stmt.contains(TESTER_NOTE) && note_stmt.contains("tester"), "#wiz-sub-note carries the nothing-to-pay line for a tester");
    assert!(note_stmt.contains("Your subscription is active."), "and still the paid note for a paid account");
    assert_eq!(render.matches(TESTER_NOTE).count(), 1, "no other element carries it");
    assert!(render.contains("wiz-subscribe") && render.contains(".lede") && render.contains(".hidden"), "the panel's lede is hidden for a tester");
    assert!(render.contains("wiz-subscribe") && render.contains("h2") && render.contains(dq(TESTER_HEADING).as_str()), "the panel's heading is retitled for a tester");
    for id in ["wiz-sub-month", "wiz-sub-year"] {
        let line = render.lines().find(|l| l.contains(&format!("EL(\"{id}\").hidden"))).unwrap_or_else(|| panic!("{id} is hidden in renderWizard"));
        assert!(line.contains("WIZ.entitled"), "{id} is hidden for any entitled account: {}", line.trim());
    }
    let wiz_decl = &js[js.find("var WIZ = {").expect("WIZ")..];
    let wiz_decl = &wiz_decl[..wiz_decl.find("};").unwrap()];
    assert!(wiz_decl.contains("plan: null"), "WIZ.plan starts unknown, so nothing is drawn before the answer");
    let sets: Vec<usize> = js.match_indices("WIZ.entitled = ").map(|(i, _)| i).collect();
    assert!(sets.len() >= 4, "four places set WIZ.entitled, the on-entry check included: {}", sets.len());
    for at in sets {
        let before = cut(&js, at.saturating_sub(900), at);
        let after = cut(&js, at, at + 400);
        assert!(before.contains("checkEntitlement("), "WIZ.entitled set without the helper: {}", cut(&js, at, at + 40));
        assert!(before.contains("WIZ.plan = ") || after.contains("WIZ.plan = "), "WIZ.entitled set without recording the plan: {}", cut(&js, at, at + 40));
    }
    let step = fn_body(&js, "function wizStep(");
    assert!(step.contains("checkEntitlement("), "wizStep asks the status-and-plan helper on entering the subscribe step");
    assert!(!js.contains("checkEntitled("), "the wizard asks only the status-and-plan helper");
}

// ---- split from static_assets.rs: the wizard's positive halves (plan §7, R2-1) ----------------------

/// Spec §11a and VISION: Knowlu never asks for a campus credential. The only password fields are the two
/// coursework logins the student chose to store (D11): exactly two, by id. (The shell keeps the allow-list
/// over every shipped file, `ui_shell::the_page_has_no_lms_credential_field_anywhere`.) The calendars panel
/// holds a button, a status line and a paste field, and nothing to type a school password into.
#[test]
fn the_wizard_has_its_two_password_fields() {
    const OURS: [&str; 2] = ["wiz-zy-pass", "wiz-vhl-pass"];
    let html = html();
    let mut seen = 0usize;
    for (i, _) in html.match_indices("type=\"password\"") {
        let around = cut(&html, i.saturating_sub(200), i + 200);
        assert!(OURS.iter().any(|id| around.contains(&format!("id=\"{id}\""))), "an unknown password field near: {around}");
        seen += 1;
    }
    assert_eq!(seen, 2, "the two coursework logins are the only passwords Knowlu ever asks for");
    let built = code();
    for pat in ["type=\"password\"", "type=\\\"password\\\"", ".type = ", "type: \"password\"", "setAttribute(\"type\""] {
        assert!(!built.contains(pat), "no password field is built outside the markup's two: {pat}");
    }
    for id in OURS {
        assert!(html.contains(&format!("id=\"{id}\"")), "password field {id}");
    }
    let panel = panel(&html, "wiz-calendars", "wiz-logins");
    assert!(!panel.contains("password"), "the calendars panel must never carry a password field");
    assert!(!panel.to_lowercase().contains("username"), "nor a username field");
    assert!(panel.contains("id=\"wiz-lms-open\"") && panel.contains("id=\"wiz-ics\""), "sign-in button and paste fallback");
    assert!(panel.contains("id=\"wiz-lms-capture\""), "the capture button");
    let js = code();
    assert!(js.contains("\"capture_calendar_link\"") && js.contains("wiz-lms-capture"), "the capture fires on the press, not on the open");
    assert!(panel.contains("id=\"wiz-school\"") && panel.contains("id=\"wiz-school-hits\""), "the school typeahead");
    assert!(panel.contains("id=\"wiz-school-none\"") && panel.contains("id=\"wiz-school-free\""), "and the free-text fallback");
    assert!(panel.contains("id=\"wiz-lms-kind\""), "and the two-button LMS question for an uncurated school");
    assert!(js.contains("EL(\"wiz-lms-open\").disabled = !WIZ.campus.unitid;"), "the sign-in button waits for a school");
    assert!(js.contains("if (!WIZ.campus.unitid) { return; }"), "the handler returns early with no school");
    let free = js.split("EL(\"wiz-school-free\").addEventListener(\"input\"").nth(1).and_then(|s| s.split("});").next()).expect("the free-entry handler");
    assert!(free.contains("EL(\"wiz-lms-kind\").hidden = !(typed && !WIZ.campus.lms);"), "free entry asks the LMS once name and state are filled");
    assert!(free.contains("var typed = !!(WIZ.campus.name && WIZ.campus.state);"), "keyed on both fields");
    assert!(free.contains("WIZ.icsNote = typed ?") && free.contains("paste your calendar link below"), "with the paste-link note, cleared otherwise");
    assert!(free.contains("unitid: \"\""), "free entry keeps the unitid empty, so sign-in stays disabled");
    assert!(free.contains("lms: WIZ.campus.unitid ? \"\" : (WIZ.campus.lms || \"\")"), "a curated school's lms is never carried into a free entry");
    assert!(!html.contains("name=\"campus\""), "no campus radios anywhere on the page");
    assert!(!js.contains("input[name=\\\"campus\\\"]"), "and nothing reads one");
    assert!(js.contains("function schoolHits(") && js.contains("\"campus_search\""), "the typeahead asks Rust");
    assert!(!js.contains("campuses.json"), "the page never names the asset; `campus_search` reads it");
    assert!(js.contains("lms: r[4] || \"\""), "pickSchool takes the curated LMS kind from the hit");
    assert!(panel.contains("id=\"wiz-cal-ics\"") && panel.contains("id=\"wiz-cal-note\""), "the personal calendar's field");
    assert!(panel.contains("Secret address in iCal format"), "the panel says where the address is");
    assert!(panel.contains("Reset"), "and advises resetting it first");
    assert!(panel.contains("id=\"wiz-google\""), "the Google sign-in is on the calendars panel");
    assert!(!panel.contains("id=\"wiz-google\" disabled"), "and is live from C2 on");
    assert!(panel.contains("id=\"wiz-courses\"") && panel.contains("id=\"wiz-course-rows\""), "the class list");
    assert!(panel.contains("id=\"wiz-course-add\""), "and the typed fallback beside it");
    assert!(js.contains("\"capture_courses\"") && js.contains("function renderCourses("), "the capture and its rows");
    assert!(!js.contains("\"connect_google\"") && !js.contains("gmail.readonly"), "no Gmail scope string on the wizard page");
    let slots = self::panel(&html, "wiz-slots", "wiz-finish");
    assert!(!slots.contains("id=\"wiz-school\""), "the school must not also be on the slots panel");
}

/// Split from `the_console_invokes_the_three_google_commands_and_never_open_external_outside_the_wizard`
/// (plan §7, R2-1): the wizard's two Google handlers are bound, each opens Google's page through the
/// wizard's own `open_external`, and every `open_external` in wizard.js sits inside one of them.
#[test]
fn the_wizard_google_and_gmail_buttons_are_bound() {
    let js = code();
    let mut spans = Vec::new();
    for (id, scope) in [("wiz-google", "calendar"), ("wiz-gmail-connect", "gmail")] {
        let head = format!("EL(\"{id}\").addEventListener(\"click\"");
        let start = js.find(&head).unwrap_or_else(|| panic!("the wizard binds #{id}"));
        let body = fn_body(&js, &head);
        assert!(body.contains(&format!("invoke(\"google_connect_url\", {{ scope: \"{scope}\" }})")), "#{id} asks for the {scope} scope");
        assert!(body.contains("invoke(\"open_external\", { url: got.url })"), "#{id} opens Google's page in the browser");
        spans.push((start, start + body.len()));
    }
    let mut opens = 0usize;
    for (at, _) in js.match_indices("invoke(\"open_external\"") {
        opens += 1;
        assert!(spans.iter().any(|&(a, b)| a < at && at < b), "open_external is invoked outside the wizard's two Google handlers");
    }
    assert_eq!(opens, 2, "one open_external per Google button");
}

/// Split from `every_control_this_task_added_is_in_the_markup_and_named_by_the_page` (plan §7: `wiz-*` here):
/// every wizard control is in the markup and named by the code, and no id is shared.
#[test]
fn every_control_this_task_added_is_in_the_markup_and_named_by_the_page() {
    let html = html();
    let named: Vec<String> = string_literals(&code());
    for id in [
        "wiz-lms-capture", "wiz-code-row", "wiz-code", "wiz-school-picked", "wiz-courses-note",
        "wiz-course-add-go", "wiz-map-note", "wiz-sub-note", "wiz-account-note",
    ] {
        assert!(html.contains(&format!("id=\"{id}\"")), "the markup has no #{id}");
        assert!(named.iter().any(|s| s == id || *s == format!("#{id}")), "wizard.js never names #{id}");
    }
    let mut ids: Vec<&str> = html.match_indices("id=\"").map(|(i, _)| html[i + 4..].split('"').next().unwrap_or("")).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "the wizard's markup carries a duplicate id");
}

// ---- new pins (note §6, A2; Quinn's M8) ------------------------------------------------------------

/// The words no string of a file may carry, lower-cased (v0.4.1 note D9: the app holds no price, so no figure,
/// cadence-with-a-price or trial length is written into it). "every 4 weeks" is checked apart, case-sensitively,
/// because "Every 4 weeks" is a plan word, and "pell" apart, as a word, because `spellcheck` is markup.
const PRICE_WORDS: [&str; 18] = [
    "9.99", "12.99", "84.37", "6.49", "69.99", "a year", "per year", "7-day", "7 day", "seven day", "14-day", "14 days",
    "fourteen", "per month", "/month", "a month", "/mo", "free first",
];

/// True when `s` holds a decimal money figure: digits, a dot, then exactly two digits.
fn has_money_figure(s: &str) -> bool {
    let b = s.as_bytes();
    (1..b.len()).any(|i| {
        b[i] == b'.' && b[i - 1].is_ascii_digit()
            && b.get(i + 1).is_some_and(u8::is_ascii_digit) && b.get(i + 2).is_some_and(u8::is_ascii_digit)
            && !b.get(i + 3).is_some_and(u8::is_ascii_digit)
    })
}

/// The literal rule (v0.4.1 note D9, r4): no string literal of `f` names a price, a figure, a cadence with a
/// price or a trial; the status word `"trialing"` is the one exception to the trial ban (it is compared, never
/// shown), and "Every 4 weeks" appears in the file exactly `labels` times, each as the button label `label_html`.
fn holds_no_price(f: &str, labels: usize, label_html: &str) {
    let text = read(f);
    let strings = string_literals(&text);
    assert!(!strings.is_empty(), "{f} has strings to read");
    for s in &strings {
        let low = s.to_lowercase();
        assert!(!s.contains('$'), "{f} shows a price: {s:?}");
        assert!(!has_money_figure(s), "{f} holds a money figure: {s:?}");
        if s != "trialing" { assert!(!low.contains("trial"), "{f} names a trial: {s:?}"); }
        for bad in PRICE_WORDS {
            assert!(!low.contains(bad), "{f} names a price or a trial length ({bad}): {s:?}");
        }
        for (i, _) in low.match_indices("pell") {
            assert!(i > 0 && low.as_bytes()[i - 1].is_ascii_alphabetic(), "{f} names Pell: {s:?}");
        }
        assert!(!s.contains("every 4 weeks"), "{f} names a cadence as words (only the label \"Every 4 weeks\" may): {s:?}");
    }
    let code = strip_comments(&text);
    assert_eq!(code.matches("Every 4 weeks").count(), labels, "{f}: \"Every 4 weeks\" is a plan word, the button label only");
    assert_eq!(code.matches(label_html).count(), labels, "{f}: each \"Every 4 weeks\" is the label {label_html}");
}

/// v0.4.1 T4 (note D9, r4), replacing `the_wizard_states_no_price_or_trial`: the wizard holds no price and
/// renders the server's offer. Every string the wizard and the picker can show is read; the subscribe step asks
/// `checkout_offer` on entering, after the entitlement check answers not entitled, and writes the offer's lines
/// as text; the panel's markup carries no price line, only its heading and its lede.
#[test]
fn the_wizard_holds_no_price_and_renders_the_offer() {
    holds_no_price("js/wizard.js", 1, ">Every 4 weeks</button>");
    holds_no_price("js/picker.js", 0, ">Every 4 weeks</button>");
    let html = html();
    let sub = panel(&html, "wiz-subscribe", "wiz-mail");
    assert!(sub.contains("<h2 tabindex=\"-1\">Subscribe</h2>") && sub.contains("class=\"lede\""), "the subscribe step keeps its heading and a lede that names the action");
    assert!(sub.contains("<p class=\"lede\">Pick a plan. The payment page opens in your browser. Cancel any time in Settings.</p>"), "the lede drops \"and shows the price\"");
    for empty in ["id=\"wiz-sub-year-lines\"></span>", "id=\"wiz-sub-month-lines\"></span>", "id=\"wiz-sub-offer-note\"></p>"] {
        assert!(sub.contains(empty), "the markup carries no price line: {empty}");
    }
    assert!(!panel(&html, "wiz-welcome", "wiz-account").contains("costs"), "the welcome panel's price line is gone");
    let js = code();
    let step = fn_body(&js, "function wizStep(");
    let entry = step.find("WIZ.entryCheck = true").expect("the on-entry entitlement check");
    let check = step[entry..].find("checkEntitlement(").map(|i| i + entry).expect("it asks the entitlement");
    let ask = step[check..].find("askOffer()").map(|i| i + check).expect("wizStep asks for the offer after the entry check");
    assert!(step[check..ask].contains("if (r.entitled)"), "the offer is asked once the entitlement has answered");
    let asker = fn_body(&js, "function askOffer(");
    assert!(asker.contains("invoke(\"checkout_offer\")"), "the offer comes from checkout_offer, with no arguments");
    let render = fn_body(&js, "function renderWizard(");
    for to in ["EL(\"wiz-sub-year-lines\").textContent = ", "EL(\"wiz-sub-month-lines\").textContent = ", "EL(\"wiz-sub-offer-note\").textContent = "] {
        assert!(render.contains(to), "renderWizard writes the offer as text: {to}");
    }
    assert!(render.contains("offer.plans.yearly.lines") && render.contains("offer.plans.four_weekly.lines") && render.contains("offer.note"), "the lines and the note are the offer's");
    for line in js.lines().filter(|l| l.contains("innerHTML")) {
        for field in ["offer", ".lines", "amount_cents", "terms_version"] {
            assert!(!line.contains(field), "an innerHTML takes an offer field ({field}): {}", line.trim());
        }
    }
}

/// v0.4.1 T4 (note D9; the amendment's ruling 2): Yearly comes first and red (`ibtn`), Every 4 weeks second
/// (`wbtn`); each sends its signed plan key, that plan's `amount_cents` from the offer and the offer's
/// `terms_version`, which billing-checkout checks against its catalog. The retired keys are gone from the page.
#[test]
fn the_subscribe_buttons_send_the_signed_plans() {
    let html = html();
    let sub = panel(&html, "wiz-subscribe", "wiz-mail");
    let year = sub.find("<button class=\"ibtn\" type=\"button\" id=\"wiz-sub-year\" disabled>Yearly</button>").expect("Yearly is the red button (ibtn)");
    let month = sub.find("<button class=\"wbtn\" type=\"button\" id=\"wiz-sub-month\" disabled>Every 4 weeks</button>").expect("Every 4 weeks is the second fill (wbtn)");
    assert!(year < month, "Yearly comes before Every 4 weeks");
    let js = code();
    assert!(js.contains("var which = e.target.closest(\"#wiz-sub-year\") ? \"yearly\" : \"four_weekly\";"), "each button sends its signed plan");
    assert!(js.contains("invoke(\"open_checkout\", { plan: which, amountCents: offer.plans[which].amount_cents, termsVersion: offer.terms_version })"), "with that plan's cents and the offer's terms version");
    assert!(!raw().contains("\"monthly\"") && !raw().contains("\"academic_year\""), "the retired plan keys are gone from the wizard");
    let billing = fs::read_to_string(app_dir().join("../cloud/supabase/functions/billing-checkout/handler.ts")).expect("billing-checkout/handler.ts");
    assert!(billing.contains("export const PLANS: readonly Plan[] = [\"yearly\", \"four_weekly\"];"), "billing takes exactly these two plans");
}

/// v0.4.1 T4 (note D9, r5: m1): until an offer is held both plan buttons are disabled, beside a Try again that
/// asks again, and the sentence says why: an unreachable service (account.rs's UNREACHABLE clause) asks for the
/// connection, any other refusal is the command's own sentence, and an answer without both plans asks for an
/// update. Nothing is kept: no storage, no saving command, and each entry asks again. A refused checkout says
/// the server's sentence and asks for the offer again.
#[test]
fn without_an_offer_the_plan_buttons_wait() {
    let html = html();
    let sub = panel(&html, "wiz-subscribe", "wiz-mail");
    assert!(sub.contains("id=\"wiz-sub-retry\">Try again</button>"), "a Try again button in the panel");
    assert!(sub.contains("id=\"wiz-sub-why\""), "the sentence beside it");
    let js = code();
    assert!(js.contains("var NO_CONNECTION = \"Prices could not be loaded. Check your connection, then try again.\";"), "the unreachable sentence");
    assert!(js.contains("var UPDATE_KNOWLU = \"Prices could not be loaded. Update Knowlu, then try again.\";"), "the missing-plan sentence");
    let render = fn_body(&js, "function renderWizard(");
    assert!(render.contains("EL(\"wiz-sub-year\").disabled = !WIZ.offer || WIZ.checkoutBusy;") && render.contains("EL(\"wiz-sub-month\").disabled = !WIZ.offer || WIZ.checkoutBusy;"), "both buttons wait for an offer, and while a press is in flight");
    assert!(render.contains("EL(\"wiz-sub-why\").textContent = WIZ.offerWhy;"), "the reason is written as text");
    assert!(render.contains("EL(\"wiz-sub-wait\").hidden = "), "the Try again row shows only with a reason");
    let from = fn_body(&js, "function offerFrom(");
    assert!(from.contains("indexOf(UNREACHABLE) === 0") && from.contains("? NO_CONNECTION : e"), "an unreachable service asks for the connection; any other refusal is said as given");
    assert!(from.contains("!plans.yearly || !plans.four_weekly") && from.contains("UPDATE_KNOWLU"), "an answer without both plans asks for an update");
    // Review M4: a plan with no line is not offered (no offer, no figure), so it reads as the update case.
    assert!(from.contains("Array.isArray(p.lines) && p.lines.length > 0 &&"), "a plan without a line is not offered");
    assert!(commands_sent(from).is_empty(), "reading the answer sends nothing");
    let asker = fn_body(&js, "function askOffer(");
    assert!(asker.contains("WIZ.offer = null;"), "each ask starts with no offer held: nothing is cached");
    assert!(asker.contains(".catch(") && asker.contains("NO_CONNECTION"), "a host that does not answer is the connection sentence");
    assert_eq!(commands_sent(asker), vec!["checkout_offer".to_string()], "the ask sends checkout_offer and nothing else");
    let listener = fn_body(&js, "EL(\"wizard\").addEventListener(\"click\"");
    let retry = listener.split("e.target.closest(\"#wiz-sub-retry\")").nth(1).expect("the Try again handler");
    assert!(retry[..retry.find("return").unwrap_or(retry.len())].contains("askOffer()"), "Try again asks again");
    let sub_handler = listener.split("e.target.closest(\"#wiz-sub-month\") || e.target.closest(\"#wiz-sub-year\")").nth(1).and_then(|s| s.split("if (e.target.closest(\"#wiz-lms-open\")").next()).expect("the plan buttons' handler");
    assert!(sub_handler.contains("if (!r.ok) { WIZ.error = r.error; renderWizard(); askOffer(); return; }"), "a refused checkout says the server's sentence and asks for the offer again");
    for kept in ["localStorage", "sessionStorage"] {
        assert!(!js.contains(kept), "the wizard keeps nothing in {kept}");
    }
}

/// T4 review M1-M3: a plan press is one at a time and always ends in words. A second press while the first is in
/// flight would open a second payment page, and two sessions made before either is paid are both payable, so a busy
/// flag holds both plan buttons disabled until `open_checkout` answers (or the call is rejected). A rejected call says
/// the unreachable clause instead of nothing, and a press that opens the payment page clears an earlier refusal.
#[test]
fn a_plan_press_is_one_at_a_time_and_always_says_why() {
    let js = code();
    let wiz_decl = &js[js.find("var WIZ = {").expect("WIZ")..];
    let wiz_decl = &wiz_decl[..wiz_decl.find("};").unwrap()];
    assert!(wiz_decl.contains("checkoutBusy: false"), "WIZ carries the press in flight, clear at the start");
    let listener = fn_body(&js, "EL(\"wizard\").addEventListener(\"click\"");
    let h = listener.split("e.target.closest(\"#wiz-sub-month\") || e.target.closest(\"#wiz-sub-year\")").nth(1).and_then(|s| s.split("if (e.target.closest(\"#wiz-lms-open\")").next()).expect("the plan buttons' handler");
    assert!(h.contains("if (!offer || WIZ.checkoutBusy) { return; }"), "a press while one is in flight does nothing (the buttons are disabled then; this is the backstop)");
    let busy = h.find("WIZ.checkoutBusy = true;").expect("the press is marked in flight");
    let ask = h.find("checkEntitlement(").expect("the entitlement ask");
    assert!(busy < ask, "the press is marked before the first await");
    assert!(h.contains("if (r.entitled) { WIZ.checkoutBusy = false;"), "an entitled answer ends the press");
    // Re-review N1, checked here too: every way out of a press draws the buttons back (renderWizard reads the flag),
    // and the entitled one then leaves the step.
    let entitled = h.find("if (r.entitled) { WIZ.checkoutBusy = false;").unwrap();
    let entitled_line = &h[entitled..entitled + h[entitled..].find('\n').unwrap_or(h.len() - entitled)];
    assert!(entitled_line.contains("renderWizard(); wizGo(3); return; }"), "an entitled press draws the step, then leaves it: {entitled_line}");
    assert!(h.contains("WIZ.checkoutOpened = true;\n            renderWizard();"), "a payment page that opened draws the buttons back");
    let open = h.find("invoke(\"open_checkout\"").expect("open_checkout");
    let answered = h[open..].find("WIZ.checkoutBusy = false;").map(|i| i + open).expect("open_checkout's answer ends the press");
    let refused = h.find("if (!r.ok) { WIZ.error = r.error; renderWizard(); askOffer(); return; }").expect("the refusal");
    assert!(answered < refused, "the press ends before the refusal or the success is drawn");
    assert!(h.contains(".catch(function () { WIZ.checkoutBusy = false; WIZ.error = UNREACHABLE; renderWizard(); });"), "a rejected call ends the press and says the unreachable clause");
    assert!(!h.contains(".catch(function () {});"), "a rejected press is never swallowed");
    let opened = h.find("WIZ.checkoutOpened = true;").expect("the payment page opened");
    assert!(h[refused..opened].contains("WIZ.error = \"\";"), "an earlier refusal is cleared once the payment page opens");
}

/// M8 (Quinn's ruling): the wizard and the picker run under `run_shell`, which has no `ui_event`, `state` or
/// `mark_seen`. `ui_shell::commands_split_by_shell_and_console` reads only each file's own `.call("x")` and
/// `invoke("x")`, so it must also see what they would reach through a shared helper. They reach none: no
/// `ev()`, no `KnowluKit`, `KnowluRouter`, `KnowluSheets`, `KnowluPanel`, `KnowluLayer`, `KnowluSlots`,
/// `KnowluShell`, `KnowluBoot`; of `KnowluData` only `call` and `last`; and no command they send is one data.js
/// treats as a view-taking write (its answer would make `call` poll `state`). `h()` sends nothing, and the
/// boot's wizard and picker paths reach no console command either.
#[test]
fn the_wizard_reaches_no_console_command_through_a_shared_helper() {
    let shell = shell_commands();
    let data = strip_comments(&read("js/data.js"));
    let viewed = data.lines().find(|l| l.trim_start().starts_with("var VIEWED = [")).expect("data.js's VIEWED list").to_string();
    for f in ["js/wizard.js", "js/picker.js"] {
        let text = read(f);
        let shape = strip_strings_and_comments(&text);
        for (n, line) in shape.lines().enumerate() {
            assert!(!is_ev_call(line), "{f}:{}: ev() sends ui_event, a console command: {}", n + 1, line.trim());
        }
        for banned in ["KnowluKit.", "KnowluRouter.", "KnowluSheets.", "KnowluPanel.", "KnowluLayer.", "KnowluSlots.", "KnowluShell.", "KnowluBoot.", "window.ev"] {
            assert!(!shape.contains(banned), "{f} reaches a console helper: {banned}");
        }
        for (i, _) in shape.match_indices("KnowluData.") {
            let rest = &shape[i + "KnowluData.".len()..];
            assert!(rest.starts_with("call(") || rest.starts_with("last("), "{f}: KnowluData.{} is not the plain wrapper", rest.split('(').next().unwrap_or(""));
        }
        let sent = commands_sent(&strip_comments(&text));
        assert!(!sent.is_empty(), "{f} sends its commands through the wrapper");
        for cmd in &sent {
            assert!(shell.contains(cmd), "{f} sends {cmd}, which run_shell does not register");
            assert!(!viewed.contains(&format!("\"{cmd}\"")), "{f} sends {cmd}, a view-taking write whose answer would poll state");
        }
    }
    let kit = strip_comments(&read("js/kit.js"));
    let h = fn_body(&kit, "function h(");
    assert!(commands_sent(h).is_empty() && !h.contains("KnowluData"), "h() sends nothing");
    let call = fn_body(&data, "function call(");
    assert!(call.contains("if (VIEWED.indexOf(cmd) >= 0) { a.view = stateView(); }"), "call() adds a view only for VIEWED commands");
    let boot = strip_comments(&read("js/boot.js"));
    for head in ["function bootWizard(", "function bootPicker(", "function showBoot("] {
        let body = fn_body(&boot, head);
        assert!(!body.contains("KnowluData") && !is_ev_call(body) && !body.contains("KnowluRouter") && !body.contains("KnowluKit"), "boot.js's {head} reaches a console command");
    }
}

/// One string, two languages: the wizard's copy of `account::UNREACHABLE`, the first clause of every
/// transport failure the Rust side emits, is word for word the Rust constant (as the console's copy is,
/// `ui_account_privacy::the_unreachable_clause_is_one_string_on_both_sides`).
#[test]
fn the_wizards_unreachable_clause_is_account_rs_s() {
    let rust = read_app("src/account.rs");
    let clause = rust.split("pub const UNREACHABLE: &str = \"").nth(1).and_then(|s| s.split('"').next()).expect("account.rs declares UNREACHABLE");
    assert!(!clause.is_empty());
    assert!(code().contains(&format!("var UNREACHABLE = \"{clause}\";")), "wizard.js's UNREACHABLE must be account.rs's, word for word: {clause:?}");
}

/// Wave-B review M6: a panel change moves focus to the new panel's heading, so a screen reader hears it and focus never
/// falls to `<body>` from a control the change just hid (Google or code sign-in moves on to step 2 by itself). Every
/// panel's `h2` takes `tabindex="-1"`, and renderWizard's one new-step branch focuses it; the first paint does not move
/// focus (nothing has changed yet).
#[test]
fn a_new_panel_takes_focus_on_its_heading() {
    let html = html();
    assert_eq!(html.matches("<h2 tabindex=\"-1\">").count(), PANELS.len(), "every panel's heading can take focus");
    assert!(!html.contains("<h2>"), "no panel heading without tabindex=\"-1\"");
    for p in PANELS {
        let at = html.find(&format!("id=\"{p}\" hidden><h2 tabindex=\"-1\">")).is_some();
        assert!(at, "#{p} opens with its focusable heading");
    }
    let render = fn_body(&code(), "function renderWizard(").to_string();
    let branch = render.split("if (SHOWN_STEP !== WIZ.step) {").nth(1).expect("renderWizard's new-step branch");
    let branch = &branch[..branch.find("\n    }").expect("the branch closes")];
    assert!(branch.contains("document.querySelector(\"#wiz-\" + PANELS[WIZ.step] + \" h2\")"), "the branch finds the new panel's heading: {branch}");
    assert!(branch.contains(".focus({ preventScroll: true })"), "and focuses it: {branch}");
    assert!(branch.contains("SHOWN_STEP !== -1") || branch.contains("first"), "the first paint does not move focus: {branch}");
    let css = strip_css_comments(&read("css/wizard.css"));
    assert!(css.contains(".wiz-panel h2:focus { outline: none; }"), "a heading that takes focus draws no ring; it is not a control");
}
