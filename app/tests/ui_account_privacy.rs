//! v0.4.0 T20: the account check, the upgrade sheet, privacy re-consent and the report (note §5 "Sign-in, upgrade",
//! "Privacy re-consent", "Report a problem"; plan §5 T20, §7's `ui_account_privacy.rs` row). Three files:
//! `js/carried/account.js` asks `account_status` once a launch and raises the `upgrade` sheet for an install with no
//! account; `js/carried/reconsent.js` asks `privacy_status` for a signed-in one and shows the re-consent panel in
//! `#layer`; `js/carried/report.js` is Data and privacy's Report a problem row and the `report` sheet. T16 draws
//! Account, Membership and Delete my data; nothing here redraws them. Every pin reads only these three files and
//! `css/sheets.css` (P3), plus `account.rs`, whose words the page must match.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

const ACCOUNT: &str = "js/carried/account.js";
const RECONSENT: &str = "js/carried/reconsent.js";
const REPORT: &str = "js/carried/report.js";
const CSS: &str = "css/sheets.css";

fn account() -> String { strip_comments(&read(ACCOUNT)) }
fn reconsent() -> String { strip_comments(&read(RECONSENT)) }
fn report() -> String { strip_comments(&read(REPORT)) }

/// `account.rs`'s `pub const NAME: &str = "…";`, the words the page must carry byte for byte.
fn rust_const(name: &str) -> String {
    let rs = read_app("src/account.rs");
    let head = format!("pub const {name}: &str = \"");
    rs.split(&head).nth(1).and_then(|s| s.split('"').next()).unwrap_or_else(|| panic!("account.rs must declare {head}…\";")).to_string()
}

/// The `var PRIVACY_CHANGES = { … };` table's text.
fn changes_table(js: &str) -> &str {
    js.split("var PRIVACY_CHANGES = {").nth(1).and_then(|s| s.split("};").next()).expect("reconsent.js needs `var PRIVACY_CHANGES = { … };`")
}

/// Re-homed (spec §6): the upgrade sheet has the same two doors the wizard has, Google first, and no password; it
/// guards its own busy state with a flag declared outside the click handler (review R2).
#[test]
fn the_upgrade_overlay_offers_the_same_two_doors_and_no_password() {
    let js = account();
    let panel = fn_body(&js, "function upgradeHtml(");
    let g = panel.find("id=\"up-google\"").expect("the sheet's Google button");
    let e = panel.find("id=\"up-email\"").expect("the sheet's email field");
    assert!(g < e, "Continue with Google comes first");
    assert!(!panel.to_lowercase().contains("password"), "never a password field");
    for id in ["up-magic", "up-code-row", "up-code", "up-code-go"] {
        assert!(panel.contains(&format!("id=\"{id}\"")), "the sheet needs #{id}");
    }
    let listener = fn_body(&js, "function onUpgradeClick(");
    for sel in ["#up-google", "#up-magic", "#up-code-go"] {
        assert!(listener.contains(sel), "the sheet's listener must handle {sel}");
    }
    assert!(js.contains("function upBusy("), "the sheet has a busy guard of its own");
    assert!(listener.contains("if (UP_BUSY) { return; }"), "…and the listener reads it before starting a sign-in");
    let before = js.split("function onUpgradeClick(").next().unwrap();
    assert!(before.contains("var UP_BUSY"), "UP_BUSY is declared at the file's scope, not inside the handler");
    assert!(listener.contains("{ ageAttested: el(\"up-18\").checked }") && listener.contains("ageAttested: el(\"up-18\").checked })"), "the age box rides as ageAttested (Tauri camel-cases)");
}

/// Re-homed (spec §11a): an install made before accounts is upgraded in place, never re-onboarded and never asked
/// about a folder. The sheet is dismissable (Not now, and Escape, which the shell gives it), only raised when the
/// account status says the vault needs one, stays down once dismissed, and stands itself down when the account
/// service cannot be reached. It is a sheet, so the shell holds it back while Settings is the route.
#[test]
fn the_console_can_sign_an_existing_install_in_without_re_onboarding_it() {
    let js = account();
    let panel = fn_body(&js, "function upgradeHtml(");
    for id in ["up-email", "up-google", "up-magic", "up-code-row", "up-code", "up-code-go", "up-18", "up-terms", "up-subscribe", "up-later", "up-error"] {
        assert!(panel.contains(&format!("id=\"{id}\"")), "the upgrade sheet needs {id}");
    }
    assert!(!panel.to_lowercase().contains("folder"), "an existing install is never asked about a folder");
    assert_eq!(panel.matches("data-policy=\"").count(), 2, "the terms and the privacy policy, each opened by open_policy");
    let listener = fn_body(&js, "function onUpgradeClick(");
    assert!(listener.contains("closest(\"[data-policy]\")") && listener.contains("preventDefault()") && listener.contains("KnowluData.call(\"open_policy\", { which: "), "the sheet's own listener opens them in the browser");
    let maybe = fn_body(&js, "function maybeUpgrade(");
    assert!(maybe.contains("s.needs_account") && maybe.contains("UPGRADE_DISMISSED") && maybe.contains("UPGRADE_UNREACHABLE"), "raised only when needed, never after a dismiss or an unreachable service");
    assert!(maybe.contains("KnowluSheets.request(\"upgrade\", "), "the upgrade is a sheet (the shell waits while Settings is the route)");
    assert!(js.contains("KnowluData.call(\"attach_account\", {})"), "the upgrade ends by attaching the account to this vault");
    assert!(js.contains("function upgradeUnreachable(") && fn_body(&js, "function upgradeUnreachable(").contains("UPGRADE_UNREACHABLE = true"), "a dead network stands it down for the session");
    assert!(js.contains("indexOf(UNREACHABLE) === 0"), "the guard tests the shared clause");
    assert!(fn_body(&js, "function onUpgradeClosed(").contains("UPGRADE_DISMISSED = true"), "Escape and Not now both dismiss it for the session");
    assert!(listener.contains("#up-later"), "Not now is handled");
    assert!(js.contains("KnowluBoot.onConsole(checkAccount)"), "one account check a launch, on the console only");
}

/// Re-homed (C1 T17's R-C1-57, I2): Subscribe reads open_checkout's envelope and says a refusal on the sheet before any
/// entitlement poll can start; the poll stops when the sheet closes.
#[test]
fn a_refused_link_is_said_out_loud_and_escape_closes_the_topmost_panel() {
    let js = account();
    let sub = fn_body(&js, "function subscribe(");
    let refused = sub.find("if (!r.ok)").expect("Subscribe must read open_checkout's envelope");
    assert!(sub.contains("el(\"up-error\").textContent = r.error"), "…and paint the reason on the sheet");
    let polls = sub.find("setTimeout(tick").expect("the entitlement poll");
    assert!(refused < polls && sub[refused..polls].contains("return"), "a refusal returns before the poll");
    assert!(sub.matches("if (!UP.root) { return; }").count() >= 2, "a closed sheet stops the poll, before and after each answer");
    assert!(sub.contains("Still not subscribed. When the payment page is done, press Subscribe again."), "the poll's end says so");
    assert!(!sub.contains("$") && !fn_body(&js, "function upgradeHtml(").contains("$"), "no price in the app (A2)");
}

/// Re-homed (L8 S5): the sheet asks before it shows Subscribe; an entitled answer goes straight to finishUpgrade.
#[test]
fn the_upgrade_overlay_shows_subscribe_only_for_a_not_entitled_answer() {
    let js = account();
    let body = fn_body(&js, "function afterUpgradeSignIn(");
    let ask = body.find("checkEntitled(").expect("afterUpgradeSignIn asks checkEntitled()");
    let unhide = body.find("el(\"up-subscribe\").hidden = false").expect("it un-hides #up-subscribe on a no");
    assert!(unhide > ask && body[ask..unhide].contains("!yes"), "the un-hide sits inside the not-entitled branch");
    assert!(body.contains("finishUpgrade()"), "an entitled answer goes straight on");
    let ent = fn_body(&js, "function checkEntitlement(");
    assert!(ent.contains("r.status === \"active\" || r.status === \"trialing\"") && ent.contains("r && r.ok"), "entitled is ok plus active or trialing");
}

/// Re-homed: account.rs emits every transport failure as "{UNREACHABLE} (…)"; the sheet stands down by testing that
/// prefix, so the clause is one string on both sides.
#[test]
fn the_unreachable_clause_is_one_string_on_both_sides() {
    let clause = rust_const("UNREACHABLE");
    assert!(!clause.is_empty());
    assert!(account().contains(&format!("var UNREACHABLE = \"{clause}\";")), "account.js must carry account.rs's UNREACHABLE exactly");
}

/// Mail's first open runs after the account and privacy check settles (M10, note §5): account.js says when, once.
#[test]
fn the_account_check_tells_mail_when_it_has_settled() {
    let js = account();
    let check = fn_body(&js, "function checkAccount(");
    assert!(check.contains("KnowluData.call(\"account_status\", {})"), "the launch's account_status");
    assert!(check.contains("KnowluReconsent.ask(s)"), "a signed-in vault is asked about privacy (reconsent.js)");
    assert!(check.contains(".then(settled, settled)"), "and the check settles either way");
    assert!(js.contains("window.KnowluAccount = {") && js.contains("after: function (fn)"), "KnowluAccount.after(fn) runs fn once the check settled");
}

/// T17's mail sheet (M10) waits on `KnowluData.last("account_status")` and, for a signed-in vault,
/// `KnowluData.last("privacy_status")`, and never asks for either itself: so the launch check asks both through
/// `KnowluData.call` (which remembers each answer), `privacy_status` only when the account status says signed in.
#[test]
fn the_launch_check_leaves_both_answers_for_mail() {
    let acc = account();
    let check = fn_body(&acc, "function checkAccount(");
    assert!(check.contains("KnowluData.call(\"account_status\", {})"), "account_status through KnowluData.call, so last() holds it");
    assert!(acc.contains("KnowluBoot.onConsole(checkAccount)"), "asked at every console launch");
    let rc = reconsent();
    let ask = fn_body(&rc, "function ask(");
    let guard = ask.find("s.needs_account").expect("ask tests needs_account");
    let call = ask.find("KnowluData.call(\"privacy_status\", {})").expect("privacy_status through KnowluData.call, so last() holds it");
    assert!(guard < call && ask[..call].contains("return null;"), "privacy_status only for a signed-in vault, after the guard");
    assert!(!acc.contains("invoke(") && !rc.contains("invoke("), "never around KnowluData");
}

/// Re-homed (legal note §9): the report is shown, editable, before anything is sent, and what is sent is what was
/// shown. The tray's way in (router.js) requests the sheet this file defines.
#[test]
fn the_issue_report_is_previewed_edited_and_sent_verbatim() {
    let js = report();
    let sheet = fn_body(&js, "function reportHtml(");
    for id in ["report-text", "report-send", "report-cancel", "report-note"] {
        assert!(sheet.contains(&format!("id=\"{id}\"")), "the report sheet needs #{id}");
    }
    assert!(sheet.contains("<textarea"), "an editable text");
    assert!(js.contains("function openReport("), "openReport");
    assert!(fn_body(&js, "function openReport(").contains("KnowluData.call(\"report_preview\", {})"), "the preview, with the page's view (data.js adds it)");
    assert!(js.contains("KnowluData.call(\"report_send\", { text: el(\"report-text\").value })"), "send exactly what is on screen");
    assert!(js.contains("KnowluSheets.define(\"report\", "), "the tray's KnowluSheets.request(\"report\") finds this sheet");
    assert!(js.contains("This is exactly what will be sent. Emails, links and long keys are already replaced. Edit anything you do not want to send, and add what went wrong at the bottom."), "the shipped sentence");
    assert!(js.contains("\"Sent. Thank you.\""), "a sent report says so");
    let row = fn_body(&js, "function rowHtml(");
    assert!(row.contains("id=\"set-report\"") && row.contains("id=\"set-report-go\""), "Data and privacy's row and its button");
    assert!(js.contains("KnowluRouter.addSection(\"report\", "), "report.js registers the Settings section `report`");
}

/// Re-homed (L4 T7, now in #layer): Accept, Settings and a privacy-policy link, and no way to dismiss it: no word for
/// one, no id, and no key handler (the shell's Escape never reaches the layer).
#[test]
fn the_reconsent_panel_has_accept_settings_a_policy_link_and_no_way_to_dismiss_it() {
    let js = reconsent();
    let panel = fn_body(&js, "function panelHtml(");
    assert!(panel.contains("<button") && panel.contains("id=\"rc-accept\""), "an Accept button");
    assert!(panel.contains("id=\"rc-settings\""), "a Settings button (the way to Delete my data)");
    assert!(panel.contains("data-policy=\"privacy\""), "the panel links the privacy policy");
    let low = panel.to_lowercase();
    for banned in ["not now", "close", "later", "dismiss", "skip"] {
        assert!(!low.contains(banned), "the panel must not offer `{banned}`");
    }
    assert!(!js.contains("keydown") && !js.contains("Escape"), "no key closes it");
    let click = fn_body(&js, "function onPanelClick(");
    let at = click.find("#rc-settings").expect("the listener handles #rc-settings");
    assert!(click[at..].contains("KnowluRouter.go(\"settings/data\")"), "#rc-settings routes to Data and privacy (I5)");
    assert!(click.contains("closest(\"[data-policy]\")") && click.contains("KnowluData.call(\"open_policy\", { which: "), "the link goes through open_policy");
}

/// Re-homed: one privacy_status a launch, for a signed-in vault only, and one accept_privacy behind a busy flag; on
/// ok the panel goes, the owed flag clears, Settings redraws and the page refreshes.
#[test]
fn the_console_asks_privacy_status_once_at_launch_and_accept_privacy_on_accept() {
    let js = reconsent();
    assert_eq!(js.matches("\"privacy_status\"").count(), 1, "one place asks privacy_status");
    let ask = fn_body(&js, "function ask(");
    assert!(ask.contains("KnowluData.call(\"privacy_status\", {})") && ask.contains("PRIVACY_ASKED"), "once a launch, behind PRIVACY_ASKED");
    assert!(ask.contains("s.needs_account") && ask.contains("!s.ok"), "only a signed-in vault whose status answered is asked");
    assert!(js.contains("window.KnowluReconsent = { ask: ask }"), "account.js reaches it");
    assert_eq!(js.matches("\"accept_privacy\"").count(), 1, "one place accepts");
    let accept = fn_body(&js, "function acceptPrivacy(");
    assert!(accept.contains("RECONSENT_BUSY"), "two presses on Accept are one command");
    let after = accept.split("\"accept_privacy\"").nth(1).unwrap();
    for step in ["KnowluKit.setOwed(false)", "KnowluLayer.hide()", "redrawSettings()", "KnowluData.refresh()"] {
        assert!(after.contains(step), "on ok, Accept runs {step}");
    }
    assert!(fn_body(&js, "function onPanelClick(").contains("acceptPrivacy()"), "Accept calls acceptPrivacy");
}

/// Re-homed: only an exact `needs_accept === true` raises the panel and sets the owed flag, in exactly one place; a
/// throw while raising it never strands the flag (it clears it and takes the panel down).
#[test]
fn an_unreachable_privacy_check_never_raises_the_panel() {
    let js = reconsent();
    let ask = fn_body(&js, "function ask(");
    assert!(ask.contains("p.needs_accept === true"), "only needs_accept === true raises the panel");
    assert_eq!(js.matches("KnowluKit.setOwed(true)").count(), 1, "the flag is set in exactly one place");
    let set_at = ask.find("KnowluKit.setOwed(true)").expect("ask sets it");
    assert!(ask[..set_at].contains("needs_accept === true"), "…and only after that test");
    let catch = ask.split("catch (").nth(1).expect("raising the panel has a catch");
    assert!(catch.contains("KnowluKit.setOwed(false)") && catch.contains("KnowluLayer.hide()"), "a throw clears the flag and hides the panel");
}

/// Re-homed: the table describes this build's PRIVACY_VERSION and has that one key; it carries no typed year, and line 1
/// is built from the reply's `current`.
#[test]
fn the_panels_changes_describe_this_privacy_version() {
    let version = rust_const("PRIVACY_VERSION");
    let js = reconsent();
    let table = changes_table(&js);
    let keys: Vec<&str> = table.lines().map(str::trim).filter(|l| l.ends_with("\": [")).map(|l| l.trim_end_matches(": [").trim_matches('"')).collect();
    assert_eq!(keys, [version.as_str()], "PRIVACY_CHANGES has exactly one key, the build's PRIVACY_VERSION");
}

/// Re-homed: the changes name Gmail and Amazon (the email the student forwards is received by Amazon Web Services).
#[test]
fn the_panels_changes_name_gmail_and_amazon() {
    let js = reconsent();
    let table = changes_table(&js);
    for w in ["Gmail", "Amazon"] {
        assert!(table.contains(w), "the changes must name {w}");
    }
}

#[test]
fn the_panels_first_line_names_the_version_from_the_key_never_a_literal_date() {
    let js = reconsent();
    for l in changes_table(&js).lines().filter(|l| !l.contains("\": [")) {
        assert!(!l.chars().collect::<Vec<_>>().windows(4).any(|w| w.iter().all(|c| c.is_ascii_digit())), "a changes line carries no typed year: {l}");
    }
    let fill = fn_body(&js, "function fillReconsent(");
    assert!(!fill.contains("Object.keys"), "no fallback to another key's date or lines");
    assert!(fill.contains("var key = p.current;"), "the key is the reply's current");
    assert!(fill.contains("[\"Knowlu's privacy policy changed on \" + key + \".\"]"), "line 1 is built from the key");
}

#[test]
fn the_panels_line_one_names_privacy_version_and_the_table_has_one_key() {
    let rust = read_app("src/account.rs");
    let status = rust.split("pub fn privacy_status_in(").nth(1).and_then(|s| s.split("\n}\n").next()).expect("account.rs has privacy_status_in");
    assert!(status.matches("json!(").count() > 0, "privacy_status_in answers with json!");
    assert_eq!(status.matches("json!(").count(), status.matches("\"current\": PRIVACY_VERSION").count(), "every privacy_status reply carries current: PRIVACY_VERSION");
    let version = rust_const("PRIVACY_VERSION");
    let js = reconsent();
    let keys: Vec<&str> = changes_table(&js).lines().map(str::trim).filter(|l| l.ends_with("\": [")).map(|l| l.trim_end_matches(": [").trim_matches('"')).collect();
    assert_eq!(keys, [version.as_str()], "one key, the build's PRIVACY_VERSION");
    let line1 = format!("Knowlu's privacy policy changed on {}.", keys[0]);
    assert!(line1.contains(&version), "line 1 reads {line1:?} and names {version}");
}

/// Re-homed (Q-A, I5): while the update is owed, every control marked as a start control in any shipped script takes
/// KnowluKit.owed() and the owed tip wrapper, and re-consent redraws Settings each time it sets or clears the flag.
#[test]
fn every_start_control_reads_the_owed_flag_and_a_throw_never_strands_it() {
    let mut seen = 0;
    for (f, raw) in scripts() {
        let code = strip_comments(&raw);
        for line in code.lines().filter(|l| l.contains("data-start")) {
            seen += 1;
            assert!(line.contains("KnowluKit.owed()") && line.contains("KnowluKit.owedHtml("), "{f}: a start control without the owed lock: {}", line.trim());
        }
    }
    assert!(seen >= 1, "Sync now at least is a start control");
    let js = reconsent();
    let ask = fn_body(&js, "function ask(");
    let set_at = ask.find("KnowluKit.setOwed(true)").unwrap();
    assert!(ask[set_at..].contains("redrawSettings()"), "setting the flag redraws Settings");
    assert!(fn_body(&js, "function redrawSettings(").contains("KnowluSettings.redraw()"), "through KnowluSettings.redraw");
}

/// Re-homed, T20's half (plan §7; the Settings half is `ui_settings::delete_stays_free_while_owed`): while the update is
/// owed, `KnowluKit.owed()` disables every control marked `data-start`, in every shipped script, and the ways out stay
/// free: Delete my data, Disconnect Google, Turn off email and Forget never carry the mark or read the flag. Settings
/// says why in words at the top of Sources and Status (review M5), and the panel's own words name Delete my data.
#[test]
fn settings_starts_nothing_new_while_the_update_is_owed() {
    let mut starts = 0;
    for (f, raw) in scripts() {
        let code = strip_comments(&raw);
        for line in code.lines() {
            if line.contains("data-start") {
                starts += 1;
                assert!(line.contains("KnowluKit.owed()"), "{f}: a start control the owed flag does not disable: {}", line.trim());
            }
            let way_out = ["set-delete-1", "set-delete-2", "set-google-disconnect-1", "set-google-disconnect-2", "set-grades-forget"].iter().any(|id| line.contains(&format!("id=\"{id}\"")))
                || (line.contains("emBtnHtml(\"") && (line.contains("\"off1\"") || line.contains("\"off2\"")));
            if way_out {
                for bad in ["data-start", "KnowluKit.owed", "isOwed"] {
                    assert!(!line.contains(bad), "{f}: a way out waits for consent ({bad}): {}", line.trim());
                }
            }
        }
    }
    assert!(starts >= 8, "the sources' and Status's start controls: {starts}");
    let kit = strip_comments(&read("js/kit.js"));
    assert!(fn_body(&kit, "function owed(").contains(" disabled"), "KnowluKit.owed() disables the control");
    for f in ["js/carried/sources.js", "js/carried/status.js"] {
        let code = strip_comments(&read(f));
        let owed = fn_body(&code, "function drawOwed(");
        assert!(owed.contains("KnowluKit.isOwed()") && owed.contains("KnowluKit.OWED_LINE"), "{f} says why in words while owed: {owed}");
    }
    assert!(changes_table(&reconsent()).contains("Settings → Delete my data"), "the panel names the way to leave");
}

/// The `report*`/`up-*` half of `every_control_this_task_added_is_in_the_markup_and_named_by_the_page` (plan §7): each
/// control the report and upgrade sheets draw is in the markup and named by the code that binds it, and no id is drawn
/// twice across the three files.
#[test]
fn every_report_and_upgrade_control_is_in_the_markup_and_named_by_the_page() {
    let acc = account();
    let rep = report();
    let rc = reconsent();
    for id in ["up-google", "up-magic", "up-code-go", "up-later", "up-subscribe", "up-18", "up-terms", "up-email", "up-code", "up-code-row", "up-error"] {
        assert!(acc.contains(&format!("id=\"{id}\"")), "the upgrade sheet has #{id}");
        let named = acc.matches(&format!("el(\"{id}\")")).count() + acc.matches(&format!("closest(\"#{id}\")")).count() + acc.matches(&format!("[\"{id}\"")).count() + acc.matches(&format!(", \"{id}\"")).count();
        assert!(named >= 1, "account.js names #{id}");
    }
    for id in ["set-report-go", "report-text", "report-send", "report-cancel", "report-note", "report-retry"] {
        assert!(rep.contains(&format!("id=\"{id}\"")), "the report has #{id}");
        let named = rep.matches(&format!("el(\"{id}\")")).count() + rep.matches(&format!("closest(\"#{id}\")")).count() + rep.matches(&format!("querySelector(\"#{id}\")")).count();
        assert!(named >= 1, "report.js names #{id}");
    }
    for id in ["rc-accept", "rc-settings", "rc-error"] {
        assert!(rc.contains(&format!("id=\"{id}\"")), "the panel has #{id}");
    }
    let mut ids: Vec<String> = Vec::new();
    for js in [&acc, &rep, &rc] {
        ids.extend(js.match_indices("id=\"").map(|(i, _)| js[i + 4..].split('"').next().unwrap_or("").to_string()).filter(|s| !s.is_empty() && !s.contains('\'')));
    }
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "account.js, report.js and reconsent.js draw a duplicate id");
}

/// Review M3, M4: the panel keeps the shipped sentence (the page underneath still works), and announces itself politely,
/// without taking focus: line 1 sits in a polite live region, written a frame after the panel is drawn (a region that
/// arrives already holding its text is not announced), so a screen reader hears why the start controls lock.
#[test]
fn the_reconsent_panel_says_the_page_still_works_and_announces_itself() {
    let js = reconsent();
    assert!(js.contains("var STILL_WORKS = \"Today's page is underneath and still works.\";"), "the shipped sentence, word for word");
    let panel = fn_body(&js, "function panelHtml(");
    assert!(panel.contains("Read the privacy policy</button>. ' + h(STILL_WORKS)"), "after the policy link");
    assert!(js.contains("var LINE1_HTML = '<p id=\"rc-line1\" aria-live=\"polite\"></p>';"), "line 1's place is a polite live region");
    assert!(panel.contains("LINES.map(lineHtml)") && fn_body(&js, "function lineHtml(").contains("return LINE1_HTML;"), "drawn in line 1's place");
    let render = fn_body(&js, "function renderPanel(");
    let frame = render.find("requestAnimationFrame(").expect("written a frame later");
    assert!(render[frame..].contains("el(\"rc-line1\")") && render[frame..].contains("LINES[0]"), "line 1 is written into it: {render}");
}

/// Review M6: a preview that could not be built leaves Send off (nothing unseen is ever sent) and offers Try again, which
/// asks for the preview again.
#[test]
fn a_failed_report_preview_offers_try_again() {
    let js = report();
    assert!(fn_body(&js, "function reportHtml(").contains("id=\"report-retry\""), "a Try again button on the sheet");
    let open = fn_body(&js, "function openReport(");
    assert!(open.contains("el(\"report-retry\").hidden = false"), "shown when the preview fails");
    assert!(open.contains("el(\"report-retry\").hidden = true"), "and hidden while one is asked");
    let click = fn_body(&js, "function onReportClick(");
    let at = click.find("#report-retry").expect("the click handles #report-retry");
    assert!(click[at..].contains("openReport()"), "Try again asks for the preview again");
    let fail = open.find("if (!r || !r.ok)").expect("the failed branch");
    let good = open.find("ta.value = r.text").expect("the good branch");
    assert!(!open[fail..good].contains("disabled = false"), "a failed preview never enables Send");
}

/// Review M7: Not now or Escape during the browser round trip closes the sheet; a late sign-in answer then finds no
/// sheet and does nothing (no TypeError on a missing element).
#[test]
fn a_sign_in_that_answers_after_the_sheet_closed_does_nothing() {
    let js = account();
    let after = fn_body(&js, "function afterUpgradeSignIn(");
    assert!(after[after.find('{').unwrap() + 1..].trim_start().starts_with("if (!UP.root) { return null; }"), "afterUpgradeSignIn first checks the sheet: {after}");
    let click = fn_body(&js, "function onUpgradeClick(");
    let magic = &click[click.find("\"send_magic_link\"").expect("the magic link call")..];
    let answer = &magic[magic.find("function (r) {").expect("its answer")..];
    let guard = answer.find("if (!UP.root) { return; }").expect("the magic-link answer checks the sheet");
    assert!(guard < answer.find("el(\"up-code-row\")").unwrap(), "before it touches the code row");
}

/// Q-A: the panel never hides the page: it is the layer's (never a sheet, never inert), it routes to Settings rather
/// than covering it, and it shows again when the shell brings the layer back (a fresh render each time).
#[test]
fn reconsent_never_hides_the_page_and_stands_down_for_settings() {
    let js = reconsent();
    assert!(fn_body(&js, "function ask(").contains("KnowluLayer.show(PANEL)"), "the panel is shown in #layer");
    assert!(!js.contains("KnowluSheets") && !js.contains("inert"), "never a sheet and never inert: the page stays usable");
    assert!(js.contains("var PANEL = { render: renderPanel }"), "the layer draws it afresh each time it returns");
    let render = fn_body(&js, "function renderPanel(");
    assert!(render.contains("panelHtml()") && !render.contains("focus("), "drawing it takes no focus from the page");
    assert!(fn_body(&js, "function panelHtml(").contains("LINES.map("), "it shows the lines fillReconsent kept");
}

/// The three files keep the page's rules: markup only from constants and h(), commands from the console's list, no
/// title= tooltip or hand-sent view, no price, no gradient.
#[test]
fn account_reconsent_and_report_keep_the_page_rules() {
    let console = console_commands();
    for f in [ACCOUNT, RECONSENT, REPORT] {
        let raw = read(f);
        assert!(!is_placeholder(&raw), "{f} is still a placeholder");
        assert!(markup_violations(&raw).is_empty(), "{f}: {:?}", markup_violations(&raw));
        for cmd in commands_sent(&raw) {
            assert!(console.contains(&cmd), "{f} sends {cmd}, which run_console does not register");
        }
        let code = strip_comments(&raw);
        assert!(!code.contains(" title=") && !code.contains("view:"), "{f}: no title= tooltip, no hand-sent view");
        assert!(!code.contains("setInterval(") && !code.contains(" trial"), "{f}: nothing polls on an interval, no trial length (A2)");
    }
    let css = read(CSS);
    assert!(!is_placeholder(&css), "sheets.css is still a placeholder");
    assert!(!strip_css_comments(&css).contains("gradient"), "sheets.css: no gradient");
}
