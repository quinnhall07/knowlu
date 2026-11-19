//! v0.4.0 T16: the Settings frame (note §4 "Settings", §5; plan §5 T16, §7's `ui_settings.rs` rows, §6's
//! Settings table). Settings is one page, `#settings[/<section>]`, one column of sections; the frame
//! (`js/pages/settings.js`) draws Account, Membership, Grades, Appearance, About you, This computer and Data
//! and privacy itself, and hosts the carried sections other tasks register with `KnowluRouter.addSection`
//! (Sources T17, Your week T18, Status and the Updates row T19, the Report row T20). Every pin reads only
//! T16's own files (P3); the rows the other tasks render are `ui_cross::the_settings_rows_all_render`'s.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

const JS: &str = "js/pages/settings.js";
const CSS: &str = "css/settings.css";

/// The page's script with its comments stripped: what a pin reads is what runs.
fn js() -> String { strip_comments(&read(JS)) }

/// The bodies of the named functions, joined: the code one pin reads.
fn bodies(js: &str, heads: &[&str]) -> String { heads.iter().map(|h| fn_body(js, h)).collect::<Vec<_>>().join("\n") }

fn prefs_code(js: &str) -> String {
    bodies(js, &["function loadPrefs(", "function prefsFill(", "function prefsConflict(", "function savePrefs(", "function offerPrefsUndo(", "function bindPrefs("])
}

fn ints_code(js: &str) -> String {
    bodies(js, &["function loadInterests(", "function intFill(", "function saveInterests(", "function offerIntUndo(", "function bindInterests("])
}

/// The lines of `code` that send a usage event.
fn ev_lines(code: &str) -> Vec<&str> { code.lines().filter(|l| l.contains("ui_event") || is_ev_call(l)).collect() }

/// M6: Settings reads `get_settings` and `settings_context` when it mounts, with `account_status`, `grades_status`,
/// `mail_status` and the profile (the preferences and the interests). `google_status` and `your_week` are read by the
/// sections that show them (T17, T18), mounted by the frame; nothing here is read at launch.
#[test]
fn settings_reads_get_settings_and_settings_context_on_mount() {
    let js = js();
    assert!(js.contains("KnowluRouter.add(\"settings\", { mount: mount, update: update, unmount: unmount });"), "Settings registers its route");
    let mount = fn_body(&js, "function mount(");
    for cmd in ["get_settings", "settings_context", "account_status", "grades_status", "mail_status"] {
        assert!(mount.contains(&format!("KnowluData.call(\"{cmd}\", {{}})")), "mount reads {cmd}");
    }
    assert!(mount.contains("loadPrefs()") && mount.contains("loadInterests()"), "mount reads the profile for About you");
    assert!(mount.contains("mountSlots("), "mount mounts the carried sections");
    let slots = fn_body(&js, "function mountSlots(");
    assert!(slots.contains("KnowluRouter.section(") && slots.contains(".mount("), "each carried section is found by name and mounted into its host");
    assert!(js.contains("var SLOTS = [\"sources\", \"week\", \"status\", \"updates\", \"report\"];"), "the five hosts the carried sections fill, in page order");
    assert!(!js.contains("\"google_status\"") && !js.contains("\"your_week\""), "the frame never asks Google or the week itself: their sections do (P4)");
    let un = fn_body(&js, "function unmount(");
    assert!(un.contains("SEQ += 1") && un.contains(".unmount("), "leaving Settings drops late answers and unmounts every section");
}

/// A2, pin 13's Settings half: Membership shows the real entitlement (status, plan, the period's end) and no
/// price, trial or trial length, anywhere in Settings' own files.
#[test]
fn membership_shows_no_price() {
    for f in [JS, CSS] {
        let code = if f.ends_with(".css") { strip_css_comments(&read(f)) } else { strip_comments(&read(f)) };
        let low = code.to_ascii_lowercase();
        assert!(!code.contains('$'), "{f}: a $ sign");
        /* "trial" as a word or a prefix of anything but the status code `trialing` (the cloud's word, never shown) */
        for (i, _) in low.match_indices("trial") {
            assert!(low[i..].starts_with("trialing"), "{f}: \"trial\" outside the status code trialing: {}", &low[i.saturating_sub(30)..(i + 30).min(low.len())]);
        }
        for banned in ["7-day", "seven days", "7 days", "per month", "a month", "/month", "/mo", "per year", "12.99", "9.99", "69.99", "pell"] {
            assert!(!low.contains(banned), "{f}: {banned:?}");
        }
    }
    let js = js();
    let member = fn_body(&js, "function drawMember(");
    for field in ["a.status", "a.plan", "a.current_period_end", "a.needs_account"] {
        assert!(member.contains(field), "Membership reads account_status's {field}");
    }
    assert!(member.contains("\"tester\"") && member.contains("Free tester access"), "a comped account reads the tester line and has no subscription to manage");
    assert!(member.contains("set-portal") && member.contains(".hidden"), "the tester has no Manage subscription");
}

/// Review I1, I2: Membership says only what `account_status` carries. Every status the cloud can answer has its words
/// (`trialing` reads Active, an account with no subscription reads No subscription and has nothing to manage), a status
/// the table does not know reads neutral, never Active, and no date is called a renewal: a subscription cancelled at
/// the period's end stays `active` until it lapses, so the period's end is "Through <date>" for every status.
#[test]
fn membership_says_only_what_the_status_carries() {
    let js = js();
    let words = js.split("var STATUS_WORDS = {").nth(1).and_then(|s| s.split("};").next()).expect("settings.js needs `var STATUS_WORDS = { … };`");
    for (code, said) in [("active", "Active"), ("trialing", "Active"), ("none", "No subscription"), ("past_due", "Payment due"), ("canceled", "Ended")] {
        assert!(words.contains(&format!("{code}: \"{said}\"")), "STATUS_WORDS: {code} reads {said}");
    }
    let member = fn_body(&js, "function drawMember(");
    assert!(!member.contains(": \"Active\""), "a status the table does not know never falls back to Active: {member}");
    assert!(!js.contains("Renews") && !js.contains("renews"), "no date is called a renewal: the status cannot say a subscription renews");
    assert!(member.contains("\"Through \""), "the period's end reads Through <date>");
    assert!(member.contains("a.status === \"none\""), "no subscription has no Manage subscription");
}

/// D4: Appearance sets the three preferences through prefs.js (theme, the reading font, the text size) and
/// shows the stored choice; OpenDyslexic and Larger text are Settings preferences (D7).
#[test]
fn appearance_sets_the_three_prefs() {
    let js = js();
    let bind = fn_body(&js, "function bindAppearance(");
    for call in ["KnowluPrefs.setTheme(", "KnowluPrefs.setFont(", "KnowluPrefs.setText("] {
        assert!(bind.contains(call), "Appearance calls {call}");
    }
    let html = fn_body(&js, "function appearanceHtml(");
    for read in ["KnowluPrefs.theme()", "KnowluPrefs.font()", "KnowluPrefs.text()"] {
        assert!(html.contains(read), "the stored choice is shown: {read}");
    }
    for value in ["\"system\"", "\"light\"", "\"dark\"", "\"default\"", "\"dyslexic\"", "\"larger\""] {
        assert!(js.contains(value), "the choice {value}");
    }
    for label in ["Theme", "Reading font", "OpenDyslexic", "Text size", "Larger"] {
        assert!(html.contains(label) || js.contains(&format!("\"{label}\"")), "the label {label}");
    }
    assert!(html.contains("type=\"radio\""), "native radios: the arrow keys move between the choices");
    assert!(!js.contains("localStorage"), "Settings keeps nothing itself: prefs.js owns the six keys (D4)");
}

/// Re-homed (now: T16's own sections and the one way in; the other tasks' rows are `ui_cross`'s, P3). The page is a
/// route, reached from the sidebar and the tray (`KNOWLU_OPEN_SETTINGS`, the router's); `#settings/<section>` scrolls to
/// that section after the router's own scroll reset. The rows T16 draws, the controls it binds by id, and the
/// subscription button's own words, which `site/terms.html` and the cancel mails name.
#[test]
fn the_settings_panel_has_its_rows_and_one_way_in() {
    let js = js();
    for row in ["set-account", "set-member", "set-name", "set-vault", "set-backup", "set-autostart", "set-diag", "set-grades-hide", "set-prefs-sec", "set-ints", "set-policy", "set-delete"] {
        assert!(js.contains(&format!("id=\"{row}\"")), "the row #{row}");
    }
    for id in ["set-portal", "set-delete-1", "set-delete-2", "set-account-state"] {
        assert!(js.contains(&format!("id=\"{id}\"")), "the control #{id}");
    }
    assert!(js.contains("id=\"set-portal\">Manage subscription<"), "the button says Manage subscription, word for word");
    for sec in ["account", "membership", "grades", "appearance", "about", "computer", "data"] {
        assert!(js.contains(&format!("data-section=\"{sec}\"")), "the section {sec} can be routed to (#settings/{sec})");
    }
    /* each host by name (review M14): a slot drawn through slotHtml is named in pageHtml, the two inside a section's list
       are literal */
    let page = fn_body(&js, "function pageHtml(");
    for slot in ["sources", "week", "status"] {
        assert!(page.contains(&format!("slotHtml(\"{slot}\")")), "pageHtml draws the host for {slot}");
    }
    for slot in ["updates", "report"] {
        assert!(js.contains(&format!("data-slot=\"{slot}\"")), "a host for {slot}");
    }
    let mount = fn_body(&js, "function mount(");
    assert!(mount.contains("ctx.arg") && mount.contains("goSection("), "the route's section is shown");
    let go = fn_body(&js, "function goSection(");
    assert!(go.contains("data-section") && go.contains("scrollIntoView"), "goSection scrolls to the section");
    assert!(mount.contains("requestAnimationFrame(") || mount.contains("setTimeout("), "after the router resets #center's scroll");
    assert!(js.contains("\"switch_profile\""), "back to the picker");
    assert!(js.contains("c.profile_name"), "the profile label comes from settings_context, not a guess");
    assert!(!js.to_lowercase().contains("channel"), "there is one channel, so no row names one");
    for absent in ["set-slots", "set-timezone", "set-sources", "set-coursemap"] {
        assert!(!js.contains(absent), "{absent}: slots, timezone, sources and the course map stay the wizard's");
    }
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "a data-id without a data-kind");
    assert!(js.contains("<h1 class=\"page-h\">Settings</h1>"), "the page's title");
}

/// Re-homed: Hide grades is a switch whose state is `grades_status`'s `hidden`, saved through `set_settings`, and
/// Settings re-reads `grades_status` when it mounts.
#[test]
fn the_settings_panel_hides_grades_through_set_settings() {
    let js = js();
    assert!(js.contains("id=\"set-grades-hide\"") && js.contains("Hide grades") && js.contains("id=\"set-grades-hide-in\""), "the row and its switch");
    assert!(js.contains("role=\"switch\""), "a switch with its role");
    let bind = fn_body(&js, "function bindGrades(");
    assert!(bind.contains("\"set_settings\", { patch: { grades_hidden: "), "through set_settings, like the other rows");
    let draw = fn_body(&js, "function drawGrades(");
    assert!(draw.contains("g.hidden") && draw.contains("aria-checked"), "the switch shows grades_status's hidden");
    assert!(fn_body(&js, "function mount(").contains("KnowluData.call(\"grades_status\", {})"), "mounting Settings re-reads grades_status");
}

/// Re-homed: "How you like to work", read from `profile`, saved with `expected`; a conflict keeps the draft; a refusal
/// is said and the draft kept.
#[test]
fn settings_reads_and_saves_preferences() {
    let js = js();
    assert!(js.contains("id=\"set-prefs-ta\"") && js.contains("id=\"set-prefs-save\"") && js.contains("id=\"set-prefs-count\""), "a text box, a counter and its own Save");
    assert!(js.contains("Your preferences") && js.contains("How you like to work"), "the section and its part");
    for fact in ["600 characters", "one call", "keep them", "stays on this computer"] {
        assert!(js.contains(fact), "the copy keeps its fact: {fact}");
    }
    let p = prefs_code(&js);
    assert!(p.contains("KnowluData.call(\"profile\", {})"), "About you reads profile");
    assert!(p.contains("\"set_preferences\"") && p.contains("expected: PREFS.loaded"), "Save sends expected");
    assert!(p.contains("env.conflict") && p.contains("data-prefs-conflict") && p.contains("Your text is still here"), "a conflict keeps the draft");
    assert!(p.contains("KnowluKit.refusal("), "other refusals are said in the engine's words");
    assert!(fn_body(&js, "function mount(").contains("loadPrefs()"), "mounting Settings loads the preferences");
}

#[test]
fn no_settings_ui_event_carries_preferences_text() {
    let js = js();
    let p = prefs_code(&js);
    for e in ["ev(\"edit_started\", null, \"preferences\")", "ev(\"edit_committed\", null, \"preferences\")", "ev(\"edit_cancelled\", null, \"preferences\")"] {
        assert!(p.contains(e), "preferences events use object kind preferences: {e}");
    }
    for line in ev_lines(&p) {
        for bad in ["ta.value", "PREFS", "before", "after", "text", "expected", "draft", ".value"] {
            assert!(!line.contains(bad), "a usage event carries preferences text ({bad}): {line}");
        }
    }
}

#[test]
fn settings_offers_undo_for_preferences() {
    let js = js();
    let u = fn_body(&js, "function offerPrefsUndo(");
    assert!(u.contains("Saved") && u.contains("Undo") && u.contains("undoToast("), "a Saved, Undo toast");
    assert!(fn_body(&js, "function undoToast(").contains("10000"), "held ten seconds");
    assert!(u.contains("\"set_preferences\"") && u.contains("expected: after") && u.contains("text: before"), "undo's expected is the text profile returned after the save");
    let p = prefs_code(&js);
    assert!(p.contains("offerPrefsUndo(before, PREFS.loaded)"), "after is the re-read text");
    assert!(!u.contains(".value"), "never the text box's value");
}

/// T05's toast duration, adopted: the Saved, Undo toast is the kit's toast held ten seconds, one at a time (the last
/// one is removed when a new save lands), not a toast of Settings' own.
#[test]
fn the_undo_toast_is_the_kits_toast_held_ten_seconds() {
    let js = js();
    let u = fn_body(&js, "function undoToast(");
    assert!(u.contains("KnowluKit.toast(text, action, fn, false, 10000)"), "the kit's toast, ten seconds: {u}");
    assert!(u.contains("TOAST.parentNode.removeChild(TOAST)") && u.contains("TOAST = "), "one Saved toast at a time");
    assert!(!u.contains("createElement") && !u.contains("setTimeout("), "no toast of its own: {u}");
}

/// Re-homed: "Campus events", the four lists, read from `profile` and saved as four lists; the disclosure keeps every
/// fixed fact on its own line.
#[test]
fn settings_reads_and_saves_interests() {
    let js = js();
    for id in ["set-int-strong", "set-int-mild", "set-int-never", "set-int-clubs", "set-int-save", "set-int-note"] {
        assert!(js.contains(&format!("id=\"{id}\"")) || js.contains(&format!("\"{id}\"")) || js.contains("id=\"set-int-' + "), "{id}");
    }
    for label in ["Campus events", "Always show me", "Maybe", "Never show me", "Clubs I\u{2019}m in"] {
        assert!(js.contains(label), "the section keeps: {label}");
    }
    let n = js.find("var INT_NOTE = ").expect("the interests note is one constant");
    let note = &js[n..n + js[n..].find(";\n").expect("note end")];
    for fact in ["600", "one call", "each event it judges", "neither we nor the model\u{2019}s host keep", "stay on this computer"] {
        assert!(note.contains(fact), "the interests note keeps the disclosure: {fact}");
    }
    let p = ints_code(&js);
    assert!(p.contains("\"set_interests\"") && p.contains("strong:") && p.contains("mild:") && p.contains("never:") && p.contains("clubs:"), "Save sends the four lists");
    assert!(p.contains("KnowluKit.refusal("), "a refusal is said, the draft kept");
    assert!(fn_body(&js, "function mount(").contains("loadInterests()"), "mounting Settings loads the lists");
}

#[test]
fn no_settings_ui_event_carries_interests_text() {
    let js = js();
    let p = ints_code(&js);
    for e in ["ev(\"edit_started\", null, \"interests\")", "ev(\"edit_committed\", null, \"interests\")", "ev(\"edit_cancelled\", null, \"interests\")"] {
        assert!(p.contains(e), "interests events use object kind interests: {e}");
    }
    for line in ev_lines(&p) {
        for bad in ["value", "INT", "before", "after", "lists", "draft", "strong", "clubs"] {
            assert!(!line.contains(bad), "a usage event carries interests text ({bad}): {line}");
        }
    }
}

#[test]
fn settings_offers_undo_for_interests() {
    let js = js();
    let u = fn_body(&js, "function offerIntUndo(");
    assert!(u.contains("Saved") && u.contains("Undo") && u.contains("undoToast("), "a Saved, Undo toast, held ten seconds");
    assert!(u.contains("\"set_interests\"") && u.contains("before.strong") && u.contains("before.clubs"), "undo sends the four lists held before the save");
}

#[test]
fn interests_are_read_only_when_not_editable() {
    let js = js();
    let p = ints_code(&js);
    assert!(p.contains("interests_editable"), "the page reads the flag");
    assert!(p.contains("readOnly") && p.contains("set-int-save"), "the lists lock and Save is disabled");
    assert!(js.contains("Knowlu can only edit a list written on one line"), "D10's reason is shown");
}

/// Re-homed: Delete my data's first press shows the confirm, with the forwarding sentence added while a mailbox is
/// connected (or `mail_status` could not say).
#[test]
fn delete_my_data_confirm_adds_the_forwarding_sentence() {
    let js = js();
    assert!(js.contains("If Knowlu cannot turn forwarding off, your mail keeps going to Knowlu's address and is deleted unread; you can turn it off in your mail settings."));
    let ask = fn_body(&js, "function askDelete(");
    assert!(ask.contains("This deletes your vault, your backups, your account and everything we hold. It cannot be undone."));
    assert!(ask.contains("mailDeleteSentence("), "the sentence is added to the first press's confirm");
    let sentence = fn_body(&js, "function mailDeleteSentence(");
    assert!(sentence.contains("mailboxes") && sentence.contains("EM_FORWARD_SENTENCE"), "added when mail_status lists a mailbox or could not answer");
    let del = fn_body(&js, "function deleteAll(");
    assert!(del.contains("KnowluData.call(\"delete_my_data\", {})") && del.contains("Deleted. Knowlu will close."), "the second press deletes, and says so");
}

/// Re-homed: a saved email sign-in that could not be deleted is named every time Settings opens, from `mail_status`.
#[test]
fn a_failed_profile_delete_is_named_until_it_succeeds() {
    let js = js();
    let draw = fn_body(&js, "function drawMail(");
    assert!(draw.contains("s.profile_delete_pending"), "read from mail_status");
    assert!(js.contains("could not be deleted from this computer yet"));
    assert!(fn_body(&js, "function mount(").contains("KnowluData.call(\"mail_status\", {})"), "every time Settings opens");
}

/// R2-1, `settings_starts_nothing_new_while_the_update_is_owed`'s Settings half: Delete my data stays free while the
/// privacy update is owed. Its two controls are never start controls and read no owed flag.
#[test]
fn delete_stays_free_while_owed() {
    let js = js();
    for line in js.lines().filter(|l| l.contains("set-delete-1") || l.contains("set-delete-2")) {
        for bad in ["KnowluKit.owed", "owedHtml", "data-start", "isOwed"] {
            assert!(!line.contains(bad), "a delete control reads the owed flag ({bad}): {}", line.trim());
        }
    }
    for head in ["function askDelete(", "function deleteAll("] {
        let b = fn_body(&js, head);
        assert!(!b.contains("isOwed") && !b.contains("KnowluKit.owed"), "{head} never waits for consent");
    }
    assert!(!js.contains("data-start"), "the frame draws no start control: those are the sources' and Status's (T17, T19)");
}

/// The `set-*` half of `every_control_this_task_added_is_in_the_markup_and_named_by_the_page` (plan §7): each control is
/// in the markup and named by the code that binds it, and no id is drawn twice.
#[test]
fn every_settings_control_is_in_the_markup_and_named_by_the_page() {
    let raw = read(JS);
    let js = strip_comments(&raw);
    for id in ["set-account-state", "set-portal", "set-delete-1", "set-delete-2", "set-delete-note", "set-name-save", "set-vault-copy", "set-switch", "set-bdir-pick", "set-backup-now", "set-autostart-in", "set-diag-copy", "set-prefs-cancel", "set-int-cancel"] {
        assert!(js.contains(&format!("id=\"{id}\"")), "the markup has #{id}");
        let named = js.matches(&format!("byId(\"{id}\")")).count() + js.matches(&format!("closest(\"#{id}\")")).count();
        assert!(named >= 1, "the code names #{id} (byId or closest)");
    }
    let mut ids: Vec<&str> = js.match_indices("id=\"").map(|(i, _)| js[i + 4..].split('"').next().unwrap_or("")).filter(|s| !s.is_empty() && !s.contains('\'')).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "settings.js draws a duplicate id");
}

/// Review M15: every page's stylesheet shares one document, so Settings' choice list is scoped to the page.
#[test]
fn settings_css_scopes_every_selector() {
    let css = strip_css_comments(&read(CSS));
    let mut seen = 0;
    for (i, _) in css.match_indices(".opts") {
        seen += 1;
        assert!(css[..i].ends_with(".st "), "a bare .opts selector: {}", css[i..].lines().next().unwrap_or(""));
    }
    assert!(seen >= 1, "the choice list is styled as .st .opts");
}
