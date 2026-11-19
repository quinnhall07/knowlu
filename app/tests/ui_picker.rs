//! v0.4.0 T21 (T22 folded in): the picker's pins (note §6; plan §5 T21, §7's `ui_picker.rs` row). The picker
//! moves out of the old `index.html` into `js/picker.js` as one module constant, `PICKER_HTML`, rendered into
//! `#boot` by `renderPicker(launch, host)`, with the same ids and handlers. The re-homed pins read the markup
//! from that constant and the code from the rest of the file, comments stripped.
#[path = "support/ui_common.rs"]
mod ui_common;

use ui_common::*;

fn raw() -> String { read("js/picker.js") }

fn js() -> String { strip_comments(&raw()) }

/// The picker's markup: the text of the one `var PICKER_HTML =` constant.
fn html() -> String {
    let js = js();
    let at = js.find("var PICKER_HTML =").expect("picker.js renders from one constant, PICKER_HTML (note §6)");
    let rest = &js[at..];
    let end = rest.find("';\n").expect("PICKER_HTML ends with a quoted line and a semicolon");
    rest[..end + 2].to_string()
}

fn code() -> String { js().replacen(&html(), "", 1) }

/// Re-homed from static_assets.rs. Changed (plan T5): the boot and its `launch_state` ask are boot.js's
/// (`ui_shell` pins them); the picker registers its render for the boot. And `.app[hidden]` is gone: the
/// picker draws into `#boot` under `html.mode-boot`, which hides the sidebar and the center (`shell.css`), so
/// the old "hiding .app needs a rule of its own" has nothing left to hide; its CSS half is that Open sits
/// at the right of its row, in picker.css.
#[test]
fn the_picker_is_in_the_page_and_carries_no_data_id() {
    let html = html();
    assert!(html.contains("id=\"picker\"") && html.contains("id=\"pick-list\"") && html.contains("id=\"pick-adopt\""));
    assert!(html.contains("id=\"pick-lede\"") && html.contains("id=\"pick-add\""), "the lede and the new-vault door");
    let js = code();
    assert!(js.contains("function renderPicker("), "renderPicker");
    assert!(js.contains("window.KnowluPicker = {") && js.contains("render: renderPicker"), "the picker registers its render for boot.js");
    assert!(strip_comments(&read("js/boot.js")).contains("KnowluData.call(\"launch_state\""), "the boot asks which window this is");
    // R-T15b's count equality still holds: picker rows are keyed by data-profile, never data-id. The leading
    // space is the repo's spelling: it excludes CSS selectors like `[data-id="` and prose.
    assert_eq!(raw().matches(" data-id=\"").count(), raw().matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    assert!(!raw().contains(" data-id=\""), "a picker row carries no data-id at all");
    assert!(js.contains("data-profile=\""), "rows are keyed by profile");
    assert!(js.contains("data-open=\""), "each row's Open carries its profile id");
    assert!(js.contains("invoke(\"open_profile\", { id: b.getAttribute(\"data-open\") })"), "Open opens that profile");
    for field in ["h(p.id)", "h(p.name)", "h(p.vault)"] {
        assert!(js.contains(field), "a profile's {field} reaches the markup through h()");
    }
    assert!(markup_violations(&raw()).is_empty(), "picker.js builds markup only from safe operands: {:?}", markup_violations(&raw()));
    let css = strip_css_comments(&read("css/picker.css"));
    assert!(css.contains(".pick-row .pick-open { margin-left: auto; }"), "Open belongs at the right of the row");
    assert!(js.contains("\"Which vault?\""), "the lede asks which vault when there are profiles");
    assert!(js.contains("\"No profile on this machine yet.\""), "and says so when there are none");
    let add = fn_body(&js, "EL(\"pick-add\").addEventListener(\"click\"");
    assert!(add.contains("invoke(\"launch_state\", {})") && add.contains("window.KnowluWizard.start("), "Create a new vault opens the same wizard, in place");
}

/// C3' Task 9, Step 6: the picker's third door, a LOCAL `Backups\` folder, never the account. Changed (Quinn:
/// no `title=` tooltips): the old button named what it is in its own `title`; that sentence is now visible
/// text under the doors, and the button points at it with `aria-describedby`, so a screen reader still reads
/// it with the button.
#[test]
fn the_picker_offers_a_local_backup_restore_link_and_says_it_is_not_the_account() {
    let html = html();
    assert!(html.contains("id=\"pick-restore-backup\""), "picker.js has no #pick-restore-backup");
    let button = html.split("id=\"pick-restore-backup\"").nth(1).and_then(|s| s.split('>').next()).expect("the button's own attributes");
    assert!(!button.contains("title=\""), "no title tooltip (Quinn's ruling)");
    assert!(button.contains("aria-describedby=\"pick-restore-why\""), "the button points at its sentence");
    let why = html.split("id=\"pick-restore-why\"").nth(1).and_then(|s| s.split("</p>").next()).expect("#pick-restore-why is a visible paragraph");
    assert!(
        why.to_lowercase().contains("never from your account") || why.to_lowercase().contains("nothing to do with the account"),
        "the sentence says this is the local mirror, not the account: {why}"
    );
    for id in ["pick-restore-options", "pick-restore-name", "pick-restore-autostart", "pick-restore-go"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "picker.js has no #{id}");
    }
    assert!(html.contains("id=\"pick-restore-autostart\" checked"), "autostart defaults on, like the wizard's own checkbox");
    let js = code();
    let handler = js
        .find("EL(\"pick-restore-backup\").addEventListener(")
        .map(|i| &js[i..])
        .and_then(|s| s.split("EL(\"pick-add\").addEventListener(").next())
        .expect("the pick-restore-backup and pick-restore-go click handlers");
    assert!(handler.contains("invoke(\"pick_folder\""), "it picks a folder first");
    assert!(handler.contains("invoke(\"restore_vault\""), "…then restores from it");
    assert!(handler.contains("invoke(\"open_profile\""), "…and opens the restored profile, like pick-adopt does");
    assert!(handler.contains("EL(\"pick-restore-name\").value = \"Knowlu\""), "the name field defaults from the wizard's own convention");
    assert!(!handler.contains("autostart: true"), "autostart is read from the checkbox, never hardcoded true");
    assert!(handler.contains("EL(\"pick-restore-autostart\").checked"), "autostart is read from the checkbox");
    assert!(handler.contains("name: name"), "the typed name is sent, not a literal");
    let go_handler = js
        .find("EL(\"pick-restore-go\").addEventListener(")
        .map(|i| &js[i..])
        .and_then(|s| s.split("EL(\"pick-add\").addEventListener(").next())
        .expect("the pick-restore-go click handler");
    let disable_at = go_handler.find("go.disabled = true").expect("disabled before the call");
    let call_at = go_handler.find("invoke(\"restore_vault\"").expect("the call itself");
    assert!(disable_at < call_at, "the button is disabled BEFORE restore_vault is invoked");
    assert_eq!(go_handler.matches("go.disabled = false").count(), 3, "re-enabled on the restore_vault refusal, the open_profile refusal, AND the catch: {go_handler}");
    let open_profile_call = go_handler.find("invoke(\"open_profile\"").expect("open_profile is still called");
    let after_open_profile = &go_handler[open_profile_call..];
    assert!(after_open_profile.contains("o.ok === false"), "open_profile's own refusal is checked: {after_open_profile}");
    assert!(after_open_profile.contains("EL(\"pick-lede\").textContent = o.error"), "…and its message is shown: {after_open_profile}");
}

/// Wave-B review M7: every row's Open is described by its own profile's name, so two rows never read "Open, Open". The
/// name's id is built from the row's index and the profile id with every character outside `[A-Za-z0-9_-]` replaced, so
/// it is one token (aria-describedby is a list of ids) and unique, and it reaches the markup through `h()`.
#[test]
fn each_open_is_described_by_its_profiles_name() {
    let js = code();
    let id = fn_body(&js, "function nameId(");
    assert!(id.contains("\"pick-name-\" + i + \"-\""), "the id carries the row index: {id}");
    assert!(id.contains(".replace(/[^A-Za-z0-9_-]/g, \"_\")"), "and the profile id reduced to one token: {id}");
    let row = fn_body(&js, "function rowHtml(");
    assert!(row.contains("function rowHtml(p, i)"), "a row knows its index");
    assert!(row.contains("<span class=\"pick-name\" id=\"' + h(nameId(p, i)) + '\">"), "the name carries the id: {row}");
    assert!(row.contains("aria-describedby=\"' + h(nameId(p, i)) + '\""), "Open points at it: {row}");
    assert!(markup_violations(&raw()).is_empty(), "built by the markup rule: {:?}", markup_violations(&raw()));
}
