//! v0.4.0 T23a (plan §5 T23a, R2-5, P3): the cross-file guards. Every page task's pins read only its own
//! files, so the checks that need two owners' files at once live here: no placeholder is left, every
//! Settings row is drawn and hosted, every expected start control is marked, the deck registers for
//! Waiting, every `#tasks/<x>` and `#settings/<x>` link names something its page accepts, and every
//! `window.Knowlu*` member one file calls is one another file defines.
//!
//! These are guards, not red pins: they are expected green on arrival. A failing guard goes back to the
//! task that owns the file it names (`v040/tNN-fix`); this file never changes page code. It reads through
//! `ui_common`, so the swap's `ROOT` change (T23) carries it.
#[path = "support/ui_common.rs"]
mod ui_common;

use std::collections::{BTreeMap, BTreeSet};
use ui_common::*;

const SETTINGS: &str = "js/pages/settings.js";
const TASKS: &str = "js/pages/tasks.js";

fn code_of(rel: &str) -> String { strip_comments(&read(rel)) }

fn all_code() -> Vec<(String, String)> { scripts().into_iter().map(|(f, t)| (f, strip_comments(&t))).collect() }

/// The quoted names of a one-line JS array, `var NAME = ["a", "b"];`, in `code`.
fn js_array(code: &str, name: &str) -> Vec<String> {
    let head = format!("var {name} = [");
    let at = code.find(&head).unwrap_or_else(|| panic!("no `{head}…];`"));
    let body = &code[at + head.len()..];
    let body = &body[..body.find("];").unwrap_or_else(|| panic!("`{head}` is not closed with ];"))];
    body.split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect()
}

fn is_ident_byte(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' || b == b'$' }

/// The identifier that starts `s` (`[A-Za-z0-9_$]*`).
fn ident_at(s: &str) -> &str { &s[..s.bytes().take_while(|b| is_ident_byte(*b)).count()] }

/// The settings slots the frame hosts, and the slot each carried file registers with `addSection`.
fn hosted_slots() -> Vec<String> { js_array(&code_of(SETTINGS), "SLOTS") }

fn sections_registered(code: &str) -> Vec<String> {
    code.match_indices("KnowluRouter.addSection(\"").map(|(i, m)| ident_at(&code[i + m.len()..]).to_string()).collect()
}

/// True when `file` draws into the Settings page: the frame itself, or a carried file that registers a
/// section for one of the frame's hosted slots.
fn drawn_in_settings(file: &str, code: &str, slots: &[String]) -> bool {
    file == SETTINGS || sections_registered(code).iter().any(|s| slots.contains(s))
}

// ---- the four guards of plan T23a ---------------------------------------------------------------

/// R2-8: every page file plan §4 names exists, and no file under `ROOT` is still a placeholder (its
/// whole content `/* TNN owns */`).
#[test]
fn no_placeholder_remains() {
    for (f, owner) in PLACEHOLDERS {
        assert!(try_read(f).is_some(), "{f} ({owner}) is missing");
    }
    let left: Vec<String> = shipped_files().into_iter().filter(|f| try_read(f).is_some_and(|t| is_placeholder(&t))).collect();
    assert!(left.is_empty(), "still placeholders: {left:?}");
}

/// The old Settings panel's rows and the controls the page binds by name (`static_assets.rs`'s
/// `the_settings_panel_has_its_rows_and_one_way_in`, its list of rows and controls), each with the file
/// that draws it. Account, Membership and Delete my data are the frame's own (T16's `settings.js`), not
/// T20's `account.js`; Updates is T19's and Report a problem T20's, each a carried section. Each is drawn
/// by exactly that one shipped script, and that script draws into the Settings page (the frame, or a
/// carried section registered for one of the frame's hosted slots).
#[test]
fn the_settings_rows_all_render() {
    let slots = hosted_slots();
    let code = all_code();
    let rows: [(&str, &str); 14] = [
        ("set-account", SETTINGS),
        ("set-member", SETTINGS),
        ("set-name", SETTINGS),
        ("set-vault", SETTINGS),
        ("set-backup", SETTINGS),
        ("set-autostart", SETTINGS),
        ("set-diag", SETTINGS),
        ("set-delete", SETTINGS),
        ("set-portal", SETTINGS),
        ("set-delete-1", SETTINGS),
        ("set-delete-2", SETTINGS),
        ("set-updates", "js/carried/update.js"),
        ("set-report", "js/carried/report.js"),
        ("set-report-go", "js/carried/report.js"),
    ];
    for (id, owner) in rows {
        let drawn: Vec<&(String, String)> = code.iter().filter(|(_, c)| c.contains(&format!("id=\"{id}\""))).collect();
        assert_eq!(drawn.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), [owner], "#{id} is drawn by {owner} and nothing else");
        let (f, c) = drawn[0];
        assert!(drawn_in_settings(f, c, &slots), "#{id} is drawn by {f}, which Settings never hosts (slots {slots:?})");
    }
    let account = code_of("js/carried/account.js");
    for id in ["set-account", "set-member", "set-delete"] {
        assert!(!account.contains(&format!("id=\"{id}\"")), "account.js draws #{id}: the frame owns it");
    }
}

/// Every control that starts a source or a sync carries `data-start` and the update-owed lock
/// (`KnowluKit.owed()` inside `KnowluKit.owedHtml(`): Sync now, grades Connect and Refresh (Sources and
/// the class page), school email Connect and Rotate, Gmail from the mail sheet, Google Connect and
/// Reconnect.
#[test]
fn every_expected_start_control_is_marked() {
    let expected: [(&str, &str, &str); 8] = [
        ("js/carried/status.js", "data-sync", "Sync now"),
        ("js/carried/sources.js", "data-grades-connect", "grades Connect"),
        ("js/carried/sources.js", "data-grades-refresh", "grades Refresh"),
        ("js/pages/class.js", "data-grades-refresh", "the class page's grades Refresh"),
        ("js/carried/sources.js", "id=\"set-email-start\"", "school email Connect"),
        ("js/carried/mail.js", "id=\"mp-gmail\"", "Connect personal Gmail"),
        ("js/carried/sources.js", "id=\"set-google-connect\"", "Google Connect"),
        ("js/carried/sources.js", "id=\"set-google-reconnect\"", "Google Reconnect"),
    ];
    for (f, needle, what) in expected {
        let code = code_of(f);
        let drawn: Vec<&str> = code.lines().filter(|l| l.contains(needle) && l.contains("<button")).collect();
        assert!(!drawn.is_empty(), "{what}: {f} draws no button with {needle}");
        for line in drawn {
            assert!(line.contains("data-start") && line.contains("KnowluKit.owed()") && line.contains("KnowluKit.owedHtml("), "{what} ({f}) is not a marked start control: {}", line.trim());
        }
    }
    // Rotate is drawn through the sources' start-control helper, never the plain button helper.
    let src = code_of("js/carried/sources.js");
    assert!(src.matches("emStartHtml(\"Rotate\", \"rotate\"").count() >= 1, "Rotate goes through emStartHtml");
    assert!(!src.contains("emBtnHtml(\"Rotate\""), "a Rotate drawn without the start mark");
    let helper = fn_body(&src, "function emStartHtml(");
    assert!(helper.contains("data-start' + KnowluKit.owed()") && helper.contains("KnowluKit.owedHtml("), "emStartHtml marks its control: {helper}");
}

/// Home's Waiting hands its card slot to `KnowluSlots.deck`, which draws nothing until a renderer is
/// registered: exactly one shipped script registers one, a function it defines itself.
#[test]
fn the_deck_registers_for_waiting() {
    let mut registrants = Vec::new();
    for (f, c) in all_code() {
        for (i, m) in c.match_indices("KnowluSlots.addDeck(") {
            let name = ident_at(&c[i + m.len()..]).to_string();
            assert!(!name.is_empty() && c.contains(&format!("function {name}(")), "{f}: addDeck takes a function the file defines ({name:?})");
            registrants.push(f.clone());
        }
    }
    assert_eq!(registrants.len(), 1, "exactly one deck registers: {registrants:?}");
    assert_ne!(registrants[0], "js/shell.js", "the shell only holds the slot");
    assert!(code_of("js/pages/home_aside.js").contains("KnowluSlots.deck("), "Waiting hands its slot to the deck");
    assert!(code_of("js/shell.js").contains("addDeck: function (fn) {"), "the shell defines addDeck");
}

// ---- route links ------------------------------------------------------------------------------

/// Every link to `route/<arg>` in the shipped scripts and the document: an `href="#route/<arg>"` (or
/// any `#route/<arg>` in a string) and a `KnowluRouter.go("route/<arg>")`, as (file, arg).
fn links_to(route: &str) -> Vec<(String, String)> {
    let mut sources = all_code();
    sources.push(("index.html".to_string(), index_html()));
    let mut out = Vec::new();
    for (f, c) in sources {
        for pat in [format!("#{route}/"), format!("go(\"{route}/"), format!("go('{route}/")] {
            for (i, m) in c.match_indices(&pat) {
                out.push((f.clone(), ident_at(&c[i + m.len()..]).to_string()));
            }
        }
    }
    out
}

/// Every `#tasks/<x>` link names a due filter Tasks accepts (`late`, `today`, `week`, `none`); Tasks
/// reads the route's argument against that one list.
#[test]
fn every_tasks_link_is_a_due_filter_tasks_accepts() {
    let tasks = code_of(TASKS);
    let accepted = js_array(&tasks, "DUE_ARGS");
    assert_eq!(accepted, ["late", "today", "week", "none"], "Tasks' due arguments");
    assert!(tasks.contains("DUE_ARGS.indexOf(ctx.arg)"), "Tasks filters by the route's argument when it is one of DUE_ARGS");
    let links: Vec<(String, String)> = links_to("tasks").into_iter().filter(|(f, _)| f != TASKS).collect();
    assert!(!links.is_empty(), "the scan found no #tasks/<x> link (Home's Needs a date links #tasks/none)");
    for (f, arg) in links {
        assert!(!arg.is_empty(), "{f}: a #tasks/ link built at run time cannot be checked; write the filter out");
        assert!(accepted.contains(&arg), "{f}: #tasks/{arg} is not a due filter Tasks accepts ({accepted:?})");
    }
}

/// Every `#settings/<x>` link (Classes links `#settings/sources`; re-consent and the tray go to
/// `settings/data`) names a section Settings draws and scrolls to: one drawn as `data-section="<x>"` by
/// the frame, or by a carried section registered for one of the frame's hosted slots.
#[test]
fn every_settings_link_names_a_section_settings_opens() {
    let slots = hosted_slots();
    let settings = code_of(SETTINGS);
    let mount = fn_body(&settings, "function mount(");
    assert!(mount.contains("goSection(") && mount.contains("ctx.arg"), "Settings opens the route's section");
    assert!(fn_body(&settings, "function goSection(").contains("data-section"), "goSection finds the section by data-section");
    let code = all_code();
    let links: Vec<(String, String)> = links_to("settings").into_iter().filter(|(f, _)| f != SETTINGS).collect();
    assert!(links.iter().any(|(_, a)| a == "sources") && links.iter().any(|(_, a)| a == "data"), "the scan found Classes' and re-consent's links: {links:?}");
    for (f, arg) in links {
        assert!(!arg.is_empty(), "{f}: a #settings/ link built at run time cannot be checked; write the section out");
        let drawn: Vec<&(String, String)> = code.iter().filter(|(_, c)| c.contains(&format!("data-section=\"{arg}\""))).collect();
        assert!(!drawn.is_empty(), "{f}: #settings/{arg} names a section no script draws");
        assert!(drawn.iter().any(|(df, dc)| drawn_in_settings(df, dc, &slots)), "{f}: #settings/{arg} is drawn only outside Settings: {:?}", drawn.iter().map(|(df, _)| df).collect::<Vec<_>>());
    }
}

// ---- shared hooks ------------------------------------------------------------------------------

/// The top-level keys of an object literal `{ a: …, b: … }` (`obj` starts at its `{`).
fn object_keys(obj: &str) -> Vec<String> {
    let shape = strip_strings_and_comments(obj);
    let b = shape.as_bytes();
    let (mut keys, mut depth, mut i, mut expect_key) = (Vec::new(), 0i32, 0usize, false);
    while i < b.len() {
        let c = b[i];
        match c {
            b'{' | b'(' | b'[' => { depth += 1; if depth == 1 { expect_key = true; } }
            b'}' | b')' | b']' => depth -= 1,
            b',' if depth == 1 => expect_key = true,
            _ if c.is_ascii_whitespace() => {}
            _ if depth == 1 && expect_key => {
                let name = ident_at(&shape[i..]);
                let rest = shape[i + name.len()..].trim_start();
                if !name.is_empty() && rest.starts_with(':') { keys.push(name.to_string()); }
                expect_key = false;
                i += name.len().max(1);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    keys
}

/// Every `window.Knowlu<Name>` global, with the file that defines it and its members: the keys of the
/// object assigned (`window.KnowluX = { … }`, or `= KIT` with `var KIT = { … }` in the same file) and
/// any `KnowluX.member =` / `KIT.member =` assignment.
fn globals() -> BTreeMap<String, (String, BTreeSet<String>)> {
    let mut out: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    for (f, c) in all_code() {
        for (i, m) in c.match_indices("window.Knowlu") {
            let name = format!("Knowlu{}", ident_at(&c[i + m.len()..]));
            let after = c[i + m.len() + name.len() - "Knowlu".len()..].trim_start();
            let Some(value) = after.strip_prefix('=').map(str::trim_start) else { continue };
            if value.starts_with('=') { continue; }
            let mut members: BTreeSet<String> = BTreeSet::new();
            let mut aliases = vec![name.clone()];
            if value.starts_with('{') {
                members.extend(object_keys(fn_body(value, "{")));
            } else {
                let alias = ident_at(value).to_string();
                let head = format!("var {alias} = {{");
                assert!(c.contains(&head), "{f}: window.{name} = {alias}, but no `{head}` in the file");
                members.extend(object_keys(&fn_body(&c, &head)[head.len() - 1..]));
                aliases.push(alias);
            }
            for a in &aliases {
                for (j, mm) in c.match_indices(&format!("{a}.")) {
                    if j > 0 && (is_ident_byte(c.as_bytes()[j - 1])) { continue; }
                    let member = ident_at(&c[j + mm.len()..]);
                    let rest = c[j + mm.len() + member.len()..].trim_start();
                    if !member.is_empty() && rest.starts_with('=') && !rest.starts_with("==") { members.insert(member.to_string()); }
                }
            }
            assert!(out.insert(name.clone(), (f.clone(), members)).is_none(), "window.{name} is defined twice");
        }
    }
    out
}

/// `KnowluMail.openApplied` (Home's What changed and Settings call it; the mail sheet defines it) and
/// every other `window.Knowlu*` member one file calls and another defines: the global exists and the
/// member is one its definition carries. Strings and comments are not calls.
#[test]
fn every_shared_hook_a_file_calls_is_defined() {
    let defs = globals();
    let mail = defs.get("KnowluMail").expect("window.KnowluMail is defined");
    assert!(mail.1.contains("openApplied") && mail.1.contains("openProgress"), "KnowluMail carries openApplied and openProgress: {:?}", mail.1);
    let mut checked = 0;
    for (f, raw) in scripts() {
        let shape = strip_strings_and_comments(&raw);
        let b = shape.as_bytes();
        for (i, _) in shape.match_indices("Knowlu") {
            if i > 0 && is_ident_byte(b[i - 1]) { continue; }
            if i > 0 && b[i - 1] == b'.' && !shape[..i - 1].ends_with("window") { continue; }
            let name = ident_at(&shape[i..]);
            let rest = &shape[i + name.len()..];
            let Some(member) = rest.strip_prefix('.').map(ident_at) else { continue };
            if member.is_empty() || !name[6..].starts_with(|c: char| c.is_ascii_uppercase()) { continue; }
            let Some((home, members)) = defs.get(name) else { panic!("{f}: {name}.{member} reads a global no shipped script defines") };
            if *home == f { continue; }
            checked += 1;
            assert!(members.contains(member), "{f}: {name}.{member} is not defined by {home} (it defines {members:?})");
        }
    }
    assert!(checked >= 50, "only {checked} cross-file calls found: the scan stopped working");
}
