//! v0.4.0 T5: the shell's pins (note §2 D1-D6, §3's drawn chrome, §8 new pins 1-5 and 8-11; plan §3,
//! §7's rows re-homed to this file). Every pin reads the new page under `ui_common::ROOT`; a pin that
//! scans "every shipped script" reads every `.js` file there, so a later task's files join it on
//! their own. Nothing here loads the page: the DOM checks are T5c's harness (`scripts/ui-fixture/`).
#[path = "support/ui_common.rs"]
mod ui_common;

use std::collections::BTreeSet;
use ui_common::*;

/// The scripts with their comments stripped: what a pin reads is what runs.
fn code_of(rel: &str) -> String { strip_comments(&read(rel)) }

fn all_code() -> Vec<(String, String)> { scripts().into_iter().map(|(f, t)| (f, strip_comments(&t))).collect() }

/// The `src` of every `<script>` and the `href` of every `<link>` in the document, in order.
fn index_refs() -> Vec<String> {
    let html = index_html();
    let mut out = Vec::new();
    for (tag, attr) in [("<script", "src=\""), ("<link", "href=\"")] {
        for (i, _) in html.match_indices(tag) {
            let rest = &html[i..];
            let end = rest.find('>').unwrap_or(rest.len());
            let el = &rest[..end];
            if let Some(a) = el.find(attr) {
                let v = &el[a + attr.len()..];
                out.push(v[..v.find('"').unwrap_or(v.len())].to_string());
            }
        }
    }
    out
}

// ---- new pins (note §8) --------------------------------------------------------------------------

/// Pin 1 (D5): no `http(s)://` in a shipped `.html`/`.js`/`.css`; an `.svg` names a URL only as the SVG
/// namespace; no inline script, no `on*=` handler, no `eval`, no `javascript:`; and nothing reaches
/// for the network (`fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`): `connect-src`
/// admits only IPC, and nothing a student sees needs the network.
#[test]
fn shipped_files_carry_no_url_inline_script_or_handler() {
    let files = shipped_with(&["html", "js", "css", "svg"]);
    assert!(files.iter().any(|(f, _)| f == "index.html"), "the shell's index.html");
    for (f, text) in &files {
        if f.ends_with(".svg") {
            let rest = text.replace("xmlns=\"http://www.w3.org/2000/svg\"", "");
            assert!(!rest.contains("http://") && !rest.contains("https://"), "{f}: a URL other than the SVG namespace");
            continue;
        }
        assert!(!text.contains("http://") && !text.contains("https://"), "{f} names a URL");
        let lower = text.to_ascii_lowercase();
        assert!(!lower.contains("javascript:"), "{f}: a javascript: URL");
        let code = if f.ends_with(".css") { strip_css_comments(text) } else { strip_comments(text) };
        // An `on…=` attribute, in markup or in a markup string a script builds.
        let b = code.as_bytes();
        for (i, _) in code.match_indices(" on") {
            let name: String = code[i + 3..].chars().take_while(|c| c.is_ascii_lowercase()).collect();
            let after = &code[i + 3 + name.len()..];
            if name.len() >= 3 && after.trim_start().starts_with('=') && after.trim_start()[1..].trim_start().starts_with(['"', '\'']) {
                panic!("{f}: an inline on{name}= handler near byte {i}");
            }
            let _ = b;
        }
        if f.ends_with(".js") {
            let shape = strip_strings_and_comments(text);
            for banned in ["eval(", "new Function(", "fetch(", "XMLHttpRequest", "WebSocket", "EventSource", "sendBeacon", "document.write("] {
                assert!(!shape.contains(banned), "{f}: {banned}");
            }
            assert!(!has_import_syntax(text) && !shape.contains("require("), "{f}: no modules, no bundler");
        }
    }
    // The document: every <script> has a src (no inline script), none is a module.
    let html = index_html();
    for (i, _) in html.match_indices("<script") {
        let el = &html[i..i + html[i..].find('>').unwrap()];
        assert!(el.contains(" src=\""), "an inline <script> in index.html: {el}");
        assert!(!el.contains("type=\"module\""), "a module script: {el}");
    }
    assert!(!html.contains("<style"), "no inline <style> block: the stylesheets are files");
    assert!(!html.contains(" style=\""), "no inline style attribute in the document");
}

/// Pin 1's second half and P1: every `<script src>` and `<link href>` resolves to a shipped file, and
/// every shipped `.js` and `.css` file is named by the document exactly once (none is orphaned, none
/// loads twice).
#[test]
fn every_script_and_link_resolves() {
    let refs = index_refs();
    assert!(!refs.is_empty(), "the document loads its scripts and stylesheets");
    let files: BTreeSet<String> = shipped_files().into_iter().collect();
    for r in &refs {
        assert!(!r.starts_with('/') && !r.contains("..") && !r.contains(':'), "{r}: a relative path inside the page's folder");
        assert!(files.contains(r), "{r} is named by index.html but is not shipped");
    }
    for f in files.iter().filter(|f| f.ends_with(".js") || f.ends_with(".css")) {
        let n = refs.iter().filter(|r| *r == f).count();
        assert_eq!(n, 1, "{f} must be named by index.html exactly once, found {n}");
    }
}

/// Pin 4 (D4): the page keeps six localStorage keys and no others, holds nothing in
/// `sessionStorage`, `indexedDB` or a cookie, and `setTheme` tells the window too.
#[test]
fn only_the_d4_storage_keys() {
    const KEYS: [&str; 6] = ["knowlu.theme", "knowlu.font", "knowlu.text", "knowlu.pin", "knowlu.cal.zoom", "knowlu.cal.due"];
    let mut seen = BTreeSet::new();
    for (f, code) in all_code() {
        let shape = strip_strings_and_comments(&code);
        for banned in ["sessionStorage", "indexedDB", "document.cookie", "caches.open"] {
            assert!(!shape.contains(banned), "{f}: {banned}");
        }
        for s in string_literals(&code) {
            if s.starts_with("knowlu.") {
                assert!(KEYS.contains(&s.as_str()), "{f}: the storage key {s:?} is not one of D4's six");
                seen.insert(s);
            }
        }
    }
    for k in ["knowlu.theme", "knowlu.font", "knowlu.text", "knowlu.pin"] {
        assert!(seen.contains(k), "the shell's preference {k} is read before paint");
    }
    let prefs = code_of("js/prefs.js");
    let set_theme = fn_body(&prefs, "setTheme: function (");
    assert!(set_theme.contains("getCurrentWindow()") && set_theme.contains(".setTheme("), "setTheme also tells the window (D4)");
    // prefs.js runs before the first paint: in the head, not deferred, before every stylesheet.
    let html = index_html();
    let head = &html[..html.find("</head>").expect("a head")];
    let p = head.find("<script src=\"js/prefs.js\"></script>").expect("prefs.js in the head, blocking (no defer)");
    let css = head.find("<link rel=\"stylesheet\"").expect("the stylesheets");
    assert!(p < css, "prefs.js runs before the first stylesheet");
}

/// Pin 5 (D5, rule 1): no mockup data or review aid ships: the mockups' made-up names, the review
/// strip, the URL switches, the fixed demo clock, the old page's `?fixture=` path.
#[test]
fn no_mock_data_or_review_aid_ships() {
    const BANNED: [&str; 34] = [
        "Maya Chen", "Maya", "CS 150", "CHEM 110", "MATH 220", "SPAN 201", "SPAN 102", "ENGL 210", "PSYC 101", "Library desk", "Lab 6 report",
        "Career Fair Game Plan", "24.5 hours", "DEMO_NOW", "nav.review", "class=\"review\"", "review.js", "show-review", "data-set-state",
        "?state=", "?fixture", "fixture=", "KNOWLU_SHOTS", "?theme=", "?frame=", "?now=", "?sync=", "?lead=", "?panel=", "pywebview",
        "quinn", "Quinn", "--c-cs", "data-when",
    ];
    for (f, text) in shipped_with(&["html", "js", "css"]) {
        for b in BANNED {
            assert!(!text.contains(b), "{f} carries {b:?} (mock data or a review aid)");
        }
    }
    for (f, code) in all_code() {
        let shape = strip_strings_and_comments(&code);
        assert!(!shape.contains("location.search") && !shape.contains("URLSearchParams"), "{f}: no URL switch reads the address");
    }
}

/// Pin 5's other half (ruling 2): Ask is v0.6.0 and hidden: no control, no Ctrl+K handler, no
/// "coming soon" anywhere.
#[test]
fn no_ask_control() {
    for (f, text) in shipped_with(&["html", "js", "css"]) {
        let code = if f.ends_with(".html") { text.clone() } else { strip_comments(&text) };
        for b in ["panel-toggle", "ask-in", "view-ask", "openAsk", ">Ask<", "\"Ask\"", "Ask or tell", "Ctrl+K", "\\u2318K", "Dictate"] {
            assert!(!code.contains(b), "{f}: an Ask control ({b})");
        }
        assert!(!code.to_ascii_lowercase().contains("coming soon"), "{f}: \"coming soon\"");
        if f.ends_with(".js") {
            assert!(!(code.contains("ctrlKey") && (code.contains("=== \"k\"") || code.contains("=== 'k'") || code.contains("\"KeyK\""))), "{f}: a Ctrl+K handler");
        }
    }
}

/// Pin 9 (D5): every `innerHTML` (and `outerHTML`, `insertAdjacentHTML`) takes a module constant or a
/// string built through `h()`, by `ui_common::markup_violations`' rule; and the scan itself rejects a
/// bad line and passes a good one, so a scan that stopped working fails here.
#[test]
fn inner_html_only_from_constants_or_h() {
    for bad in [
        "el.innerHTML = r.title;",
        "el.innerHTML = '<b>' + r.title + '</b>';",
        "var x = '<li>' + name + '</li>';",
        "return '<p>' + s.text + '</p>';",
        "el.insertAdjacentHTML('beforeend', '<i>' + t + '</i>');",
        "var rowHtml = r.title;",
        "el.innerHTML = rows.map(function (r) { return '<li>' + r.title + '</li>'; }).join('');",
        "el.innerHTML = `<b>${r.title}</b>`;",
        "  '<td>' + r.title + '</td>' +",
    ] {
        assert!(!markup_violations(bad).is_empty(), "the scan must reject: {bad}");
    }
    for good in [
        "el.innerHTML = '';",
        "el.innerHTML = WC_HTML;",
        "el.innerHTML = '<b>' + h(r.title) + '</b>';",
        "var x = '<li class=\"' + h(c.slot) + '\">' + h(c.title) + '</li>';",
        "el.innerHTML = rows.map(rowHtml).join('');",
        "el.innerHTML = rows.map(function (r) { return '<li>' + h(r.title) + '</li>'; }).join('');",
        "return '<p>' + (x ? h(a) : '') + '</p>';",
        "  '<td>' + h(r.title) + '</td>' +",
        "var cellHtml = '<td>' + h(n) + '</td>';",
        "el.innerHTML = '<button class=\"btn\" data-start' + KnowluKit.owed() + '>Connect</button>';",
        "el.textContent = r.title;",
    ] {
        assert!(markup_violations(good).is_empty(), "the scan must pass: {good} ({:?})", markup_violations(good));
    }
    let mut sinks = 0usize;
    for (f, text) in scripts() {
        let v = markup_violations(&text);
        assert!(v.is_empty(), "{f}: markup built outside h() or a constant (D5): {v:?}");
        sinks += HTML_SINKS.iter().map(|s| strip_comments(&text).matches(s).count()).sum::<usize>();
    }
    assert!(sinks >= 1, "the shell writes markup somewhere; a scan that finds no sink is not scanning");
}

/// Pin 10 (D1, I9): the telemetry `view` is one of the seven fixed route names, never a slug. The
/// router holds the list; `ev()` is the one `ui_event` sender and takes the router's name.
#[test]
fn telemetry_views_are_the_fixed_route_names() {
    let router = code_of("js/router.js");
    assert!(
        router.contains("var ROUTES = [\"home\", \"tasks\", \"calendar\", \"classes\", \"class\", \"lockin\", \"settings\"];"),
        "router.js names D1's seven routes, in order, as ROUTES"
    );
    let view = fn_body(&router, "function view(");
    assert!(view.contains("return NAME;"), "the router's view() answers the fixed route name");
    let kit = code_of("js/kit.js");
    let ev = fn_body(&kit, "function ev(");
    assert!(ev.contains("\"ui_event\"") && ev.contains("view: KnowluRouter.view()"), "ev() sends the router's fixed name as view");
    assert!(!ev.contains("arg()") && !ev.contains("slug"), "ev() never sends a slug");
    for (f, code) in all_code() {
        if f == "js/kit.js" { continue; }
        assert!(!code.contains("\"ui_event\"") && !code.contains("'ui_event'"), "{f}: ui_event is sent only by kit.js's ev()");
    }
    assert_eq!(kit.matches("\"ui_event\"").count(), 1, "one ui_event sender");
}

/// Pin 11 (D3, M4): every mutating call passes the current view and adopts the returned state;
/// `preview_window`'s state is never adopted. The commands that take a `view` are read from
/// `app/src`, so a new one cannot be missed.
#[test]
fn writes_pass_the_view_and_preview_window_never_replaces_the_poll() {
    let data = code_of("js/data.js");
    let line = data.lines().find(|l| l.trim_start().starts_with("var VIEWED = [")).expect("data.js lists the commands that take a view as VIEWED");
    let listed: BTreeSet<String> = line.split('[').nth(1).unwrap().split(']').next().unwrap().split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect();
    let want: BTreeSet<String> = commands_taking_a_view().into_iter().filter(|c| c != "state" && c != "ui_event").collect();
    assert!(want.len() >= 10, "the scan of app/src found the mutating commands ({want:?})");
    assert_eq!(listed, want, "VIEWED must be exactly the commands whose signature takes view (less state and ui_event)");
    assert!(!listed.contains("preview_window"), "preview_window is a read; its state is never the poll's");
    let call = fn_body(&data, "function call(");
    assert!(call.matches("VIEWED.indexOf(cmd)").count() >= 2, "call() adds the view and adopts the state for VIEWED commands only");
    assert!(call.contains("stateView()") && call.contains("adopt("), "the view is the page's state view; the answer's state is adopted");
    let adopt = fn_body(&data, "function adopt(");
    assert!(adopt.contains("HELD = null"), "a write's state is painted at once, never held as a reorder (F19)");
    for (f, code) in all_code() {
        if f == "js/data.js" { continue; }
        assert!(!code.contains("adopt("), "{f}: only data.js adopts a state");
        assert!(!code.contains("KnowluData.state = ") && !code.contains("KnowluData.S"), "{f}: nobody replaces the poll's state");
    }
}

/// Pin 2's half (D7): every `@font-face` file exists, relative to its stylesheet.
#[test]
fn every_font_face_file_exists() {
    let mut faces = 0usize;
    for (f, text) in styles() {
        let css = strip_css_comments(&text);
        let dir = f.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for block in css.split("@font-face").skip(1) {
            let block = &block[..block.find('}').unwrap_or(block.len())];
            for part in block.split("url(").skip(1) {
                let raw = part[..part.find(')').expect("url( is closed")].trim().trim_matches(['"', '\'']);
                assert!(!raw.contains(':'), "{f}: a font from outside the bundle: {raw}");
                let path = if dir.is_empty() { raw.to_string() } else { format!("{dir}/{raw}") };
                let resolved = normalise(&path);
                assert!(root().join(&resolved).is_file(), "{f}: @font-face names {raw}, which is not at {resolved}");
                faces += 1;
            }
        }
    }
    assert!(faces >= 5, "the three families' faces are declared ({faces})");
}

fn normalise(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => { out.pop(); }
            s => out.push(s),
        }
    }
    out.join("/")
}

/// Pin 3: every command the wizard and the picker send is in `run_shell`'s list, every other script's
/// in `run_console`'s, read from `main.rs`; `open_external` is the wizard's alone; only `data.js`
/// reaches `__TAURI__.core`.
#[test]
fn commands_split_by_shell_and_console() {
    let shell = shell_commands();
    let console = console_commands();
    assert_eq!(shell.len(), 30, "run_shell keeps its 30 commands (note §6; v0.4.1 D9 adds checkout_offer)");
    assert_eq!(console.len(), 72, "run_console keeps its 72 commands (plan §1; v0.4.1 D9 adds checkout_offer)");
    let mut sent = 0usize;
    for (f, text) in scripts() {
        let wizardish = f == "js/wizard.js" || f == "js/picker.js";
        for cmd in commands_sent(&text) {
            sent += 1;
            let list = if wizardish { &shell } else { &console };
            assert!(list.contains(&cmd), "{f} sends {cmd}, which is not in {}'s generate_handler! list", if wizardish { "run_shell" } else { "run_console" });
            if cmd == "open_external" { assert_eq!(f, "js/wizard.js", "open_external is the wizard's only"); }
        }
        let code = strip_comments(&text);
        if f != "js/data.js" {
            assert!(!code.contains("__TAURI__.core") && !code.contains("__TAURI_INTERNALS__") && !code.contains(".invoke("), "{f}: only data.js invokes");
        }
    }
    assert!(sent >= 5, "the shell sends its own commands ({sent}); a scan that finds none is not scanning");
    let data = code_of("js/data.js");
    assert!(data.contains("window.__TAURI__.core.invoke(") || data.contains("core.invoke("), "data.js is the one invoke wrapper");
}

/// §3 and N9: boot.js emits `ui-ready` once, as a plain global emit (T6's Rust side listens with
/// `app.listen`, which an `emitTo` would miss), after the theme class (prefs.js in the head),
/// `DOMContentLoaded` and one `requestAnimationFrame` raced with 300 ms.
#[test]
fn boot_emits_ui_ready_once_after_theme_and_a_frame() {
    let boot = code_of("js/boot.js");
    let emits: Vec<_> = boot.match_indices("emit(\"ui-ready\")").collect();
    assert_eq!(emits.len(), 1, "one emit of ui-ready");
    assert!(boot.contains("window.__TAURI__.event.emit(\"ui-ready\")") || boot.contains("ev.emit(\"ui-ready\")"), "a plain event.emit");
    assert!(!boot.contains("emitTo"), "never emitTo: the Rust side listens with app.listen");
    let ready = fn_body(&boot, "function uiReady(");
    assert!(ready.contains("if (READY_SENT) { return; }") && ready.contains("READY_SENT = true"), "sent once, guarded");
    let frame = fn_body(&boot, "function afterFirstFrame(");
    assert!(frame.contains("requestAnimationFrame(") && frame.contains("setTimeout(") && frame.contains("300"), "one frame, raced with 300 ms");
    assert!(boot.contains("\"DOMContentLoaded\""), "after DOMContentLoaded");
    let prefs = code_of("js/prefs.js");
    assert!(prefs.contains("setAttribute(\"data-theme\""), "the theme is set by prefs.js, which runs in the head before the first paint");
}

/// §5: the tray's two globals stay defined and route: Settings to `#settings`, Report to the report
/// sheet (over Settings > Data and privacy).
#[test]
fn the_tray_globals_route() {
    let router = code_of("js/router.js");
    let s = router.find("window.KNOWLU_OPEN_SETTINGS = function () {").expect("KNOWLU_OPEN_SETTINGS");
    let s_body = &router[s..s + router[s..].find("};").unwrap()];
    assert!(s_body.contains("go(\"settings\")"), "the tray's Settings routes to #settings");
    let r = router.find("window.KNOWLU_OPEN_REPORT = function () {").expect("KNOWLU_OPEN_REPORT");
    let r_body = &router[r..r + router[r..].find("};").unwrap()];
    assert!(r_body.contains("go(\"settings/data\")") && r_body.contains("KnowluSheets.request(\"report\")"), "the tray's Report opens the report sheet over Data and privacy");
    let tray = read_app("src/tray.rs");
    assert!(tray.contains("window.KNOWLU_OPEN_SETTINGS && window.KNOWLU_OPEN_SETTINGS()") && tray.contains("window.KNOWLU_OPEN_REPORT && window.KNOWLU_OPEN_REPORT()"), "the tray still calls the two names");
}

/// §2 and §3: the skeleton has every host, the band is a deep drag region on the sidebar head and the
/// top row, and the three caption buttons call the window with `isMaximized()` for the glyph.
#[test]
fn the_skeleton_has_every_host() {
    let html = index_html();
    for id in ["center", "panel", "sheet", "layer", "boot", "top-note"] {
        assert_eq!(html.matches(&format!("id=\"{id}\"")).count(), 1, "the host #{id}, once");
    }
    assert!(html.find("id=\"sheet\"").unwrap() < html.find("id=\"layer\"").unwrap(), "#layer sits above #sheet");
    assert!(html.contains("<title>Knowlu</title>"));
    for cls in ["class=\"side-head\" data-tauri-drag-region=\"deep\"", "class=\"top\" data-tauri-drag-region=\"deep\""] {
        assert!(html.contains(cls), "the band: {cls}");
    }
    for wc in ["data-wc=\"minimize\"", "data-wc=\"toggle\"", "data-wc=\"close\""] {
        assert!(html.contains(wc), "the caption button {wc}");
    }
    let shell = code_of("js/shell.js");
    for call in [".minimize()", ".toggleMaximize()", ".close()", ".isMaximized()", "getCurrentWindow()"] {
        assert!(shell.contains(call), "shell.js calls {call}");
    }
}

/// D3, I4: while the envelope carries `first_run`, the state is null and the poll runs every 3 s.
#[test]
fn first_run_polls_every_three_seconds_with_state_null() {
    let data = code_of("js/data.js");
    assert!(data.contains("var FIRST_RUN_MS = 3000;"), "the first run's cadence");
    let poll = fn_body(&data, "function poll(");
    assert!(poll.contains("env.first_run"), "the poll reads the envelope's first_run");
    let first = fn_body(&data, "function firstRunArrived(");
    assert!(first.contains("S = null") && first.contains("setTimeout(poll, FIRST_RUN_MS)"), "the state is null and the poll re-arms in 3 s");
    let api = fn_body(&data, "firstRun: function (");
    assert!(api.contains("return FIRST;"), "firstRun() hands the pages the block");
}

/// D1 (re-homed): dispatch only through an own-property check, across every shipped script: no bare
/// `obj.hasOwnProperty(`, no `in` test against a registry, and the router's tables have no prototype.
#[test]
fn own_property_dispatch_only() {
    for (f, code) in all_code() {
        let shape = strip_strings_and_comments(&code);
        for (i, _) in shape.match_indices(".hasOwnProperty(") {
            assert!(shape[..i].ends_with("Object.prototype"), "{f}: hasOwnProperty is called through Object.prototype.hasOwnProperty.call");
        }
        for t in ["PAGES", "SECTIONS", "PANELS", "SHEETS", "KNOWN"] {
            assert!(!shape.contains(&format!(" in {t}")), "{f}: an `in` test against {t}");
        }
    }
    let router = code_of("js/router.js");
    assert!(router.contains("var PAGES = Object.create(null);") && router.contains("var SECTIONS = Object.create(null);"), "the router's tables have no prototype");
    let shell = code_of("js/shell.js");
    assert!(shell.contains("var PANELS = Object.create(null);") && shell.contains("var SHEETS = Object.create(null);"), "the shell's tables have no prototype");
}

/// N5: one sheet at a time, in the stated order; upgrade waits while Settings is the route; the layer
/// holds only re-consent and stands down while a sheet is open or Settings is the route.
#[test]
fn the_sheet_queue_order_and_the_layer_stand_down() {
    let shell = code_of("js/shell.js");
    assert!(shell.contains("var SHEET_RANK = { signin: 0, upgrade: 0, weeksetup: 1, mail: 2, report: 3 };"), "N5's order: sign-in / upgrade, week setup, mail progress, report");
    let next = fn_body(&shell, "function nextSheet(");
    assert!(next.contains("\"upgrade\"") && next.contains("onSettings()"), "upgrade waits while Settings is the route");
    let req = fn_body(&shell, "request: function (");
    assert!(req.contains("SHEET_RANK"), "a sheet with no rank is refused: nothing else renders into #sheet");
    let layer = fn_body(&shell, "function layerStandsDown(");
    assert!(layer.contains("OPEN_SHEET") && layer.contains("onSettings()"), "the layer stands down for a sheet or Settings");
    let esc = fn_body(&shell, "function onEscape(");
    assert!(!esc.contains("KnowluLayer.hide") && !esc.contains("hideLayer"), "Escape never closes the re-consent layer (no dismiss)");
}

/// M5 as shipped (P4), negative half: no launch, boot or poll path calls `google_status`. The shell's
/// Status list shows only what the last Settings visit read (`KnowluData.last`). The Settings-mount
/// call is T17's (`ui_sources_mail::the_google_row_reads_google_status_on_settings_mount`).
#[test]
fn google_status_only_on_settings_mount() {
    for f in ["js/boot.js", "js/router.js", "js/data.js", "js/kit.js", "js/fmt.js", "js/prefs.js"] {
        assert!(!code_of(f).contains("google_status"), "{f} names google_status");
    }
    let shell = code_of("js/shell.js");
    for (i, _) in shell.match_indices("google_status") {
        assert!(shell[..i].ends_with("KnowluData.last(\""), "shell.js reads google_status only through KnowluData.last, never by a call");
    }
}

/// R2-8: a route with no registered page (a placeholder's) leaves `#center` empty and throws nothing.
#[test]
fn an_unregistered_route_leaves_center_empty_without_a_throw() {
    let router = code_of("js/router.js");
    let mount = fn_body(&router, "function mount(");
    let none = mount.find("if (!mod) {").expect("mount() handles a route with no page");
    let branch = &mount[none..none + mount[none..].find('}').unwrap()];
    assert!(branch.contains("CENTER.textContent = \"\"") && branch.contains("return"), "an unregistered route empties #center and returns");
    assert!(!router.contains("throw "), "the router throws nothing");
}

/// P1: every page file plan §4 names exists and is listed exactly once, as a `<script>` or a `<link>`.
#[test]
fn every_page_placeholder_is_listed_once() {
    let refs = index_refs();
    for (file, owner) in PLACEHOLDERS {
        assert!(root().join(file).is_file(), "{file} ({owner}'s) must exist");
        assert_eq!(refs.iter().filter(|r| *r == file).count(), 1, "{file} ({owner}'s) must be listed once in index.html");
    }
    // Shell scripts first, then the pages, carried surfaces, wizard and picker.
    let pos = |f: &str| refs.iter().position(|r| r == f).unwrap_or_else(|| panic!("{f} listed"));
    for s in SHELL_SCRIPTS {
        assert!(pos(s) < pos("js/pages/home.js"), "{s} loads before the pages");
    }
    assert!(pos("js/pages/settings.js") < pos("js/carried/details.js") && pos("js/carried/report.js") < pos("js/wizard.js") && pos("js/wizard.js") < pos("js/picker.js"), "pages, carried surfaces, wizard, picker");
}

/// P11, I1: the Status dot is amber on `S.warnings` (and unreadable notes), the error red only when a
/// source needs the student.
#[test]
fn the_status_dot_is_amber_on_warnings() {
    let shell = code_of("js/shell.js");
    let level = fn_body(&shell, "function statusLevel(");
    assert!(level.contains(".warnings") && level.contains("\"warn\""), "warnings make the level warn");
    assert!(level.contains("\"error\""), "a source that needs a look makes it error");
    assert!(shell.contains("setAttribute(\"data-level\""), "the dot carries its level");
    let css = strip_css_comments(&read("css/shell.css"));
    let warn = css.find("[data-level=\"warn\"]").expect("the warn dot's rule");
    let rule = &css[warn..warn + css[warn..].find('}').unwrap()];
    assert!(rule.contains("var(--amber"), "the warn dot is amber");
    let err = css.find("[data-level=\"error\"]").expect("the error dot's rule");
    assert!(css[err..err + css[err..].find('}').unwrap()].contains("var(--error)"), "the error dot is --error");
}

/// M1, P11: the sidebar lists the courses whose `status` is absent or `active`, as the engine reads
/// it (`commitments.rs`: blank after trim, or `active`).
#[test]
fn the_sidebar_lists_courses_with_status_absent_or_active() {
    let fmt = code_of("js/fmt.js");
    let active = fn_body(&fmt, "function active(");
    assert!(active.contains("var st = String(c.status == null ? \"\" : c.status).trim();") && active.contains("return st === \"\" || st === \"active\";"), "active is the engine's rule");
    let shell = code_of("js/shell.js");
    let side = fn_body(&shell, "function drawClasses(");
    assert!(side.contains("KnowluCourses.active"), "the sidebar filters by KnowluCourses.active");
    assert!(side.contains("h(c.slot)") && side.contains("h(c.title)") && side.contains("h(c.slug)"), "a class link: slot, title and slug, through h()");
}

/// D3, P11: one poll, at launch, on focus and every 60 s; a new `revision` repaints.
#[test]
fn the_poll_runs_at_launch_on_focus_and_every_sixty_seconds() {
    let data = code_of("js/data.js");
    assert!(data.contains("var POLL_MS = 60000;"), "every sixty seconds");
    let start = fn_body(&data, "start: function (");
    assert!(start.contains("setInterval(poll, POLL_MS)") && start.contains("addEventListener(\"focus\", poll)") && start.contains("poll()"), "at launch, on focus, on the minute");
    let poll = fn_body(&data, "function poll(");
    assert!(poll.contains("\"state\"") && poll.contains("stateView()"), "the poll reads state with the page's view");
    assert!(poll.contains("\"grades_status\"") && poll.contains("\"mail_status\""), "grades_status and mail_status ride the poll (P4)");
    let fresh = fn_body(&data, "function arrived(");
    assert!(fresh.contains(".revision === REV"), "only a new revision repaints");
}

/// D3, R28-R30 (re-homed from the old page): a changed order waits for the pointer to leave the list;
/// a route change forgets the shown state; a write's state is never held.
#[test]
fn a_changed_order_waits_for_the_pointer() {
    let data = code_of("js/data.js");
    assert!(data.contains("function orderOf("), "the order of the rows the page shows");
    let arrived = fn_body(&data, "function arrived(");
    assert!(arrived.contains("orderOf(") && arrived.contains("[data-rows]:hover") && arrived.contains("HELD = "), "a reorder under the pointer is held");
    assert!(arrived.contains("\"pointerleave\""), "and painted when the pointer leaves the list");
    let view = fn_body(&data, "view: function (");
    assert!(view.contains("S = null") && view.contains("REV = null") && view.contains("HELD = null"), "a route change forgets the shown state (R28)");
    let router = code_of("js/router.js");
    assert!(router.contains("KnowluData.view("), "the router tells data.js the route changed");
}

/// Pin 8: every `progress: 100` write carries `status: "done"` in the same object, in every shipped
/// script, and the one check control sends exactly that.
#[test]
fn every_progress_100_write_carries_done() {
    let mut found = 0usize;
    for (f, code) in all_code() {
        for (n, line) in code.lines().enumerate() {
            for pat in ["progress: 100", "\"progress\": 100", "progress:100"] {
                if line.contains(pat) {
                    found += 1;
                    assert!(line.contains("status: \"done\"") || line.contains("\"status\": \"done\""), "{f}:{}: a progress of 100 without status done: {}", n + 1, line.trim());
                }
            }
        }
    }
    assert!(found >= 1, "the check control writes progress 100");
    let kit = code_of("js/kit.js");
    let check = fn_body(&kit, "check: function (");
    assert!(check.contains("\"set_fields\"") && check.contains("{ progress: 100, status: \"done\" }"), "KnowluKit.check is set_fields {{progress: 100, status: done}}");
    let uncheck = fn_body(&kit, "uncheck: function (");
    assert!(uncheck.contains("\"set_fields\"") && uncheck.contains("status: \"active\""), "Undo restores the old progress with status active");
}

/// A8: a done check fills with the row's class colour (the slot variable, ink with no entry), never a
/// gray.
#[test]
fn a_done_check_fills_with_the_class_colour_never_gray() {
    let css = strip_css_comments(&read("css/shell.css"));
    let mut rules = 0usize;
    for (i, _) in css.match_indices(".ck[aria-pressed=\"true\"]") {
        let rule = &css[i..i + css[i..].find('}').unwrap()];
        rules += 1;
        for gray in ["--ink-3", "--ink-2", "--line", "--field-line", "gray", "grey", "--shade-", "#"] {
            assert!(!rule.contains(gray), "a done check's rule names {gray}: {rule}");
        }
    }
    assert!(rules >= 1, "the done check has its rule");
    let fill = css.find(".ck[aria-pressed=\"true\"] circle").expect("the done check's fill rule");
    let rule = &css[fill..fill + css[fill..].find('}').unwrap()];
    assert!(rule.contains("fill: var(--cc, var(--ink))"), "the fill is the class colour, ink with no entry");
}

// ---- re-homed from static_assets.rs, whole (plan §7) ---------------------------------------------

#[test]
fn no_network_reference_in_the_shipped_page() {
    for (f, text) in shipped_with(&["html", "js", "css"]) {
        assert!(!text.contains("http://") && !text.contains("https://"), "{f} references the network");
    }
}

/// Every family the stylesheets ask for is vendored with an `@font-face` (or is a system fallback),
/// and the fonts folder carries its `SOURCES.md`. Zilla Slab does not ship (D7).
#[test]
fn every_font_the_page_names_is_vendored() {
    const SYSTEM: [&str; 9] = ["Georgia", "Segoe UI", "system-ui", "serif", "sans-serif", "monospace", "inherit", "Consolas", "ui-monospace"];
    let mut declared = BTreeSet::new();
    let mut named = BTreeSet::new();
    for (_, text) in styles() {
        let css = strip_css_comments(&text);
        for block in css.split("@font-face").skip(1) {
            let block = &block[..block.find('}').unwrap_or(block.len())];
            if let Some(i) = block.find("font-family:") {
                let v = block[i + 12..].split(';').next().unwrap().trim().trim_matches(['"', '\'']).to_string();
                declared.insert(v);
            }
        }
        for decl in ["font-family:", "--display:", "--serif:", "--sans:"] {
            for (i, _) in css.match_indices(decl) {
                let v = css[i + decl.len()..].split([';', '}']).next().unwrap();
                for fam in v.split(',') {
                    let fam = fam.trim().trim_matches(['"', '\'']).trim();
                    if fam.is_empty() || fam.starts_with("var(") { continue; }
                    named.insert(fam.to_string());
                }
            }
        }
    }
    for fam in &named {
        if declared.contains(fam) { continue; }
        assert!(SYSTEM.contains(&fam.as_str()), "the stylesheets name the family {fam:?}, which has no @font-face and is no system fallback");
    }
    for fam in ["PT Serif Caption", "Albert Sans", "OpenDyslexic"] {
        assert!(declared.contains(fam), "{fam} is declared");
    }
    assert!(!named.iter().any(|f| f.contains("Zilla")) && !declared.iter().any(|f| f.contains("Zilla")), "Zilla Slab does not ship (D7)");
    assert!(root().join("fonts/SOURCES.md").is_file(), "fonts/SOURCES.md");
}

#[test]
fn the_import_check_rejects_statements_and_accepts_prose() {
    assert!(has_import_syntax("var a = 1; import b from \"c\";"));
    assert!(has_import_syntax("const m = import(\"x\");"));
    let prose = "const t = \"does not sync, import or read\"; // import x\n/* import y */ `a \\` import z`";
    assert!(!has_import_syntax(prose));
    for (f, text) in scripts() {
        assert!(!has_import_syntax(&text), "{f}: no modules, no bundler");
    }
}

#[test]
fn the_brand_is_knowlu_everywhere_a_user_reads_it() {
    let html = index_html();
    assert!(html.contains("<title>Knowlu</title>") && html.contains("Know<b>lu</b>"), "index.html: the title and the wordmark");
    for (f, text) in shipped_with(&["html", "js", "css"]) {
        assert!(!text.contains("quinn-ops"), "{f}: the old name must not be visible");
    }
    let conf = read_app("tauri.conf.json");
    assert!(conf.contains("\"productName\": \"Knowlu\"") && conf.contains("\"title\": \"Knowlu\""));
}

#[test]
fn nothing_on_the_page_reaches_a_local_model() {
    for (f, text) in shipped_with(&["html", "js"]) {
        for gone in ["set-judge", "wiz-judge", "2 GB", "inference_status", "install_inference", "remove_inference_model", "renderInference", "loadInference", "offer_inference", "manifest.json"] {
            assert!(!text.contains(gone), "{f} still names {gone}");
        }
    }
}

#[test]
fn no_multi_word_command_argument_is_sent_in_the_wrong_case() {
    fn camel(s: &str) -> String {
        let mut out = String::new();
        for (i, part) in s.split('_').enumerate() {
            if i == 0 { out.push_str(part); continue; }
            let mut cs = part.chars();
            if let Some(c) = cs.next() { out.extend(c.to_uppercase()); }
            out.push_str(cs.as_str());
        }
        out
    }
    let js: String = all_code().into_iter().map(|(_, c)| c).collect::<Vec<_>>().join("\n");
    let mut checked = 0usize;
    let src = app_dir().join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("app/src") {
            let path = entry.expect("a dir entry").path();
            if path.is_dir() { stack.push(path); continue; }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
            let rust = std::fs::read_to_string(&path).expect("a source file");
            for block in rust.split("#[tauri::command").skip(1) {
                let Some(close) = block.find(']') else { continue };
                let opted_out = block[..close].contains("rename_all = \"snake_case\"");
                let after = block[close + 1..].trim_start();
                let Some(sig) = after.strip_prefix("pub fn ").or_else(|| after.strip_prefix("pub async fn ")) else { continue };
                let name = sig.split('(').next().unwrap_or("").to_string();
                let Some(args) = sig.split('(').nth(1).and_then(|a| a.split(')').next()) else { continue };
                for arg in args.split(',') {
                    let ident = arg.split(':').next().unwrap_or("").trim();
                    if !ident.contains('_') || ident.starts_with('_') { continue; }
                    checked += 1;
                    let camelled = camel(ident);
                    let wrong = if opted_out { camelled.as_str() } else { ident };
                    let right = if opted_out { ident } else { camelled.as_str() };
                    for shape in [format!("{wrong}:"), format!(".{wrong} =")] {
                        assert!(!js.contains(&shape),
                            "{}::{name} takes `{ident}`, so Tauri delivers it as `{right}` — a shipped script sends `{wrong}`",
                            path.file_name().unwrap_or_default().to_string_lossy());
                    }
                }
            }
        }
    }
    assert!(checked >= 6, "only {checked} multi-word command arguments found — the scan stopped working");
}

#[test]
fn the_untrusted_text_scan_rejects_a_bad_line() {
    for bad in [
        "r = '<div>' + h(c.from) + ' · ' + c.evidence + '</div>';",
        "r = h(c.from) + x.sender;",
        "list.push(c.title);",
        "t = \"\" || c.title;",
        "el.innerHTML = c.title;",
        "r = h(m.x) + c.evidence;",
    ] {
        let reads = field_reads(bad, &["from", "evidence", "sender", "title"]);
        assert!(reads.iter().any(|(_, l, u)| !read_is_safe(l, u)), "should be rejected: {bad}");
    }
    for good in ["r = h(c.from) + h(c.evidence);", "r = c.from ? h(c.from) : \"\";", "if (!c.from && !c.evidence) { return; }", "h(c.why || c.kind)"] {
        let reads = field_reads(good, &["from", "evidence", "why"]);
        assert!(reads.iter().all(|(_, l, u)| read_is_safe(l, u)), "should pass: {good}");
    }
}

/// The view name comes from the URL hash. `{}["constructor"]` and `{}["__proto__"]` are truthy, so
/// a bare table lookup would let an inherited member through (and call it). Now the router.
#[test]
fn console_js_dispatches_on_the_view_name_only_through_own_property_checks() {
    let router = code_of("js/router.js");
    let parse = fn_body(&router, "function parse(");
    assert!(parse.contains("Object.prototype.hasOwnProperty.call(KNOWN, name)"), "the requested route is checked by own property");
    let find = fn_body(&router, "function pageFor(");
    assert!(find.contains("Object.prototype.hasOwnProperty.call(PAGES, name)"), "a page is found only among the table's own keys");
    let sect = fn_body(&router, "section: function (");
    assert!(sect.contains("Object.prototype.hasOwnProperty.call(SECTIONS, name)"), "a Settings section likewise");
    // Every read of a table indexed by a name sits behind its own-property check on the same line.
    for t in ["PAGES[", "SECTIONS[", "KNOWN["] {
        for line in router.lines().filter(|l| l.contains(t)) {
            let assigns = line.contains(&format!("{t}name] = ")) || line.contains(&format!("{t}r] = "));
            assert!(assigns || line.contains("hasOwnProperty.call("), "a bare lookup: {}", line.trim());
        }
    }
}

// ---- split from static_assets.rs: the negative halves stay here (plan §7, R2-1) -----------------

/// `h` escapes the five characters that open a tag or an attribute, and no mail field reaches
/// markup outside `h()`, in any shipped script. The card line and the applied rows are T15's and
/// T17's positive halves.
#[test]
fn mail_derived_text_renders_inert() {
    let kit = code_of("js/kit.js");
    assert!(kit.contains("function h("), "kit.js defines h");
    for ch in ["&amp;", "&lt;", "&gt;", "&quot;", "&#39;"] {
        assert!(kit.contains(ch), "h escapes to {ch}");
    }
    for (f, code) in all_code() {
        for (n, line, upto) in mail_reads(&code) {
            if HTML_SINKS.iter().any(|s| line.contains(s)) || line.contains('<') {
                assert!(read_is_safe(&line, &upto), "{f}:{n}: a mail field in markup outside h(): {}", line.trim());
            }
        }
    }
}

/// Every read of a mail field, and of a card's or row's `title` and `why`, sits inside `h(...)` or is
/// only a test, in every shipped script. No minimum count here (R2-1): the card line's and the
/// applied rows' counts are T15's and T17's.
#[test]
fn no_mail_field_reaches_the_dom_without_h() {
    for (f, code) in all_code() {
        for (n, line, upto) in mail_reads(&code) {
            assert!(read_is_safe(&line, &upto), "{f}:{n}: a mail field outside h(): {}", line.trim());
        }
        for (n, line, upto) in field_reads(&code, &["title", "why"]) {
            let var = upto.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).next().unwrap_or("");
            if !(var.starts_with("c.") || var.starts_with("x.")) { continue; }
            assert!(read_is_safe(line, &upto), "{f}:{n}: {var} not inside h(): {}", line.trim());
        }
    }
}

/// No usage event carries a mail field, a card's words or a mailbox address, in any shipped script.
/// The mail sections' own markers are T17's and the card line's T15's.
#[test]
fn no_ui_event_carries_mail_text() {
    for (f, code) in all_code() {
        for (n, line) in code.lines().enumerate().filter(|(_, l)| l.contains("ui_event") || is_ev_call(l)) {
            for bad in [".from", ".mail_date", ".evidence", ".sender", ".why", ".title", ".address", ".text", ".body", "mpAddr", "mp-addr"] {
                assert!(!line.contains(bad) || line.contains("Array.from"), "{f}:{}: a usage event carries {bad}: {}", n + 1, line.trim());
            }
        }
    }
}

/// `mark_seen` is the end of a look (blur, hidden), never the boot, and invoked from exactly one
/// place; `ev()` emits `view_opened` on a route change and `object_seen` on a 2 s dwell. The sync
/// line's and the runs view's events are T19's half.
#[test]
fn seen_is_stamped_at_the_end_of_the_look_and_events_are_emitted() {
    let total: usize = all_code().iter().map(|(_, c)| c.matches("\"mark_seen\"").count()).sum();
    assert_eq!(total, 1, "mark_seen is invoked from exactly one place");
    let kit = code_of("js/kit.js");
    assert!(kit.contains("\"mark_seen\""), "kit.js's endOfLook sends it");
    assert!(kit.contains("addEventListener(\"blur\", endOfLook)") && kit.contains("\"visibilitychange\"") && kit.contains("document.hidden"), "stamped on blur and on hidden (R37)");
    assert!(!code_of("js/boot.js").contains("mark_seen") && !code_of("js/data.js").contains("mark_seen"), "never at boot or in the poll");
    assert!(kit.contains("IntersectionObserver") && kit.contains("\"object_seen\"") && kit.contains("2000"), "object_seen on a 2 s dwell");
    assert!(code_of("js/router.js").contains("ev(\"view_opened\""), "a route change sends view_opened");
}

/// No launch or boot path reaches `google_status` (it calls the network); the Settings-mount call,
/// its guard and its cancel are T17's.
#[test]
fn the_launch_account_check_does_not_reach_google_status() {
    for f in ["js/boot.js", "js/data.js", "js/router.js", "js/kit.js"] {
        assert!(!code_of(f).contains("google_status"), "{f}: the launch path must not reach google_status");
    }
    let boot = code_of("js/boot.js");
    let console = fn_body(&boot, "function bootConsole(");
    assert!(!console.contains("google"), "the console's boot names no Google call");
}

/// The only password fields anywhere are the wizard's two coursework logins, by id (allow-list, no
/// count: the count is T21's `ui_wizard::the_wizard_has_its_two_password_fields`).
#[test]
fn the_page_has_no_lms_credential_field_anywhere() {
    const OURS: [&str; 2] = ["wiz-zy-pass", "wiz-vhl-pass"];
    for (f, text) in shipped_with(&["html", "js"]) {
        let code = if f.ends_with(".js") { strip_comments(&text) } else { text.clone() };
        for pat in ["type=\"password\"", "type=\\\"password\\\"", "type = \"password\"", ".type = 'password'", "type: \"password\""] {
            for (i, _) in code.match_indices(pat) {
                assert_eq!(f, "js/wizard.js", "{f}: a password field outside the wizard");
                let around = &code[i.saturating_sub(200)..(i + 200).min(code.len())];
                assert!(OURS.iter().any(|id| around.contains(id)), "{f}: an unknown password field near: {around}");
            }
        }
    }
}

// ---- review fixes (docs/reports/2026-11-18-v040-review-t5.md) -----------------------------------

/// The line of `body` holding `needle`, or a panic naming it.
fn line_with<'a>(body: &'a str, needle: &str) -> &'a str {
    body.lines().find(|l| l.contains(needle)).unwrap_or_else(|| panic!("no line with {needle}"))
}

/// I1: the Status dot redraws on every poll, and again when `grades_status` and `mail_status` land
/// after the state's paint, never only on a new revision. data.js tells the poll's outcome through
/// one hook (`KnowluData.onPoll`), and the shell redraws Status from it.
#[test]
fn the_status_redraws_on_every_poll() {
    let data = code_of("js/data.js");
    let poll = fn_body(&data, "function poll(");
    assert!(line_with(poll, "\"grades_status\"").contains("polled("), "grades_status's answer tells the poll hook");
    assert!(line_with(poll, "\"mail_status\"").contains("polled("), "mail_status's answer tells the poll hook");
    assert!(poll.contains("polled({ ok: true"), "a poll that answered tells the hook ok");
    let polled = fn_body(&data, "function polled(");
    assert!(polled.contains("POLL_SUBS"), "polled() calls every onPoll subscriber");
    let on_poll = fn_body(&data, "onPoll: function (");
    assert!(on_poll.contains("POLL_SUBS.push(fn)"), "onPoll subscribes");
    let shell = code_of("js/shell.js");
    let start = fn_body(&shell, "start: function (");
    assert!(start.contains("KnowluData.onPoll(drawStatus)"), "the shell redraws Status on every poll");
}

/// I2: a failed poll (a rejected call, or an envelope with `ok: false`) is recorded and told through
/// the same hook as `{ok: false, error}`, and during the first run the 3 s retry is re-armed after it.
#[test]
fn a_failed_poll_is_told_and_the_first_run_retries() {
    let data = code_of("js/data.js");
    let poll = fn_body(&data, "function poll(");
    assert!(poll.matches("pollFailed(").count() >= 2, "both a rejection and an ok:false envelope are failures");
    let failed = fn_body(&data, "function pollFailed(");
    assert!(failed.contains("polled({ ok: false"), "a failure is told as ok: false");
    assert!(failed.contains("LAST.state = "), "and recorded for last(\"state\")");
    assert!(failed.contains("if (FIRST") && failed.contains("FIRST_TIMER = setTimeout(poll, FIRST_RUN_MS)"), "the first run keeps its 3 s cadence");
}

/// M1: a route change that keeps the state view paints a held reorder rather than dropping it.
#[test]
fn a_same_view_route_change_paints_a_held_reorder() {
    let data = code_of("js/data.js");
    let view = fn_body(&data, "view: function (");
    let same = line_with(view, "next === VIEW");
    assert!(same.contains("if (HELD) { paint(HELD); }"), "the held state is painted: {same}");
}

/// M2: a write's answer is adopted only while its view is still the page's; otherwise the page polls.
#[test]
fn a_write_answer_for_another_view_is_dropped() {
    let data = code_of("js/data.js");
    let call = fn_body(&data, "function call(");
    assert!(call.contains("a.view === stateView()"), "the answer's view is still the page's");
    assert!(call.contains("poll()"), "otherwise the page polls its own view");
}

/// M5: the peek opens after about 350 ms, folds about 400 ms after the pointer leaves (cancelled if
/// it returns), folds on Escape, and a mouse click inside does not hold it (only keyboard focus does).
/// Tooltips wait about 450 ms (60 ms once one has shown) and hide on a press and on Escape.
#[test]
fn the_peek_and_the_tooltips_keep_the_mockups_timing() {
    let shell = code_of("js/shell.js");
    assert!(shell.contains("var PEEK_OPEN_MS = 350;") && shell.contains("var PEEK_CLOSE_MS = 400;"), "the peek's two delays");
    assert!(shell.contains("setTimeout(fold, PEEK_CLOSE_MS)"), "leaving folds after the delay");
    assert!(shell.contains(":focus-visible"), "only keyboard focus holds the peek open");
    let esc = fn_body(&shell, "function onEscape(");
    assert!(esc.contains("fold()"), "Escape folds the peek");
    let kit = code_of("js/kit.js");
    assert!(kit.contains("var TIP_DELAY_MS = 450;") && kit.contains("var TIP_AGAIN_MS = 60;"), "the tooltip's delays");
    assert!(kit.contains("addEventListener(\"pointerdown\", hideTip"), "a press hides the tip");
    assert!(kit.contains("e.key === \"Escape\"") && kit.contains("hideTip()"), "Escape hides the tip");
}

/// M6: `KnowluKit.owed()`'s own output is only literals and `h(OWED_LINE)`: the one exemption the
/// markup rule grants is earned here. `KnowluKit.owedHtml(controlHtml)` wraps a start control while
/// the update is owed in a focusable span that carries the tip, because a disabled control takes
/// neither the pointer nor focus; the disabled control passes the pointer to it.
#[test]
fn the_owed_attributes_are_literals_and_the_tip_reaches_a_disabled_control() {
    let kit = code_of("js/kit.js");
    let owed = fn_body(&kit, "function owed(");
    let ret = owed.split("return ").nth(1).expect("owed() returns").split(';').next().unwrap();
    assert!(ret.trim_start().starts_with("OWED ?"), "owed() answers on the owed flag: {ret}");
    assert!(operand_is_safe(ret), "owed()'s answer is literals and h(): {ret}");
    assert!(ret.contains(" disabled") && ret.contains("h(OWED_LINE)") && ret.contains(": \"\""), "disabled with the line, or nothing: {ret}");
    let wrap = fn_body(&kit, "function owedHtml(");
    assert!(wrap.contains("tabindex=\\\"0\\\"") || wrap.contains("tabindex=\"0\""), "the wrapper takes keyboard focus");
    assert!(wrap.contains("data-tip=") && wrap.contains("h(OWED_LINE)") && wrap.contains("controlHtml"), "the wrapper carries the tip around the control");
    assert!(markup_violations(wrap).is_empty(), "the wrapper is built by the markup rule");
    let css = strip_css_comments(&read("css/shell.css"));
    let at = css.find(".owed > [disabled]").expect("the disabled control inside the wrapper has a rule");
    assert!(css[at..at + css[at..].find('}').unwrap()].contains("pointer-events: none"), "and passes the pointer to the wrapper");
}

/// D1: `logos/` holds the marks of the sources v0.4.0 reads (note §2), each an SVG that names only
/// the SVG namespace: no editor namespace, no script, no handler, no link out.
#[test]
fn the_logos_are_the_sources_v040_reads() {
    const LOGOS: [&str; 7] = ["blackboard", "canvas", "zybooks", "vhl", "gcal", "gmail", "outlook"];
    for name in LOGOS {
        let svg = read(&format!("logos/{name}.svg"));
        assert!(svg.trim_start().starts_with("<svg "), "logos/{name}.svg is an SVG");
        let rest = svg.replace("xmlns=\"http://www.w3.org/2000/svg\"", "");
        assert!(!rest.contains("xmlns:") && !rest.contains("://"), "logos/{name}.svg names another namespace or a URL");
        let lower = rest.to_ascii_lowercase();
        for bad in ["<script", "javascript:", "href=", "<foreignobject", "<!entity", "<?xml-stylesheet"] {
            assert!(!lower.contains(bad), "logos/{name}.svg carries {bad}");
        }
        for (i, _) in lower.match_indices(" on") {
            let name_len = lower[i + 3..].chars().take_while(|c| c.is_ascii_lowercase()).count();
            assert!(!(name_len >= 3 && lower[i + 3 + name_len..].starts_with('=')), "logos/{name}.svg carries a handler");
        }
    }
    let shipped: Vec<String> = shipped_files().into_iter().filter(|f| f.starts_with("logos/")).collect();
    assert_eq!(shipped.len(), LOGOS.len(), "logos/ holds exactly the seven marks: {shipped:?}");
}

/// Every `ev(` kind the new page sends is one `ui_event` accepts: the engine's `uievents::ACTIONS`, read from
/// its source (the app refuses any other name at `record`, so a made-up kind would be dropped silently).
#[test]
fn every_ev_kind_is_an_accepted_ui_event_action() {
    let src = std::fs::read_to_string(app_dir().join("..").join("engine/src/uievents.rs")).expect("engine/src/uievents.rs");
    let start = src.find("pub const ACTIONS").expect("ACTIONS list");
    let list = &src[start..start + src[start..].find("];").expect("end of ACTIONS")];
    let accepted: BTreeSet<String> = string_literals(list).into_iter().collect();
    assert!(accepted.contains("view_opened") && accepted.len() >= 11, "read the list: {accepted:?}");
    let mut seen = BTreeSet::new();
    for (file, code) in scripts() {
        for (i, _) in code.match_indices("ev(") {
            if code[..i].chars().next_back().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '.') { continue; }
            let rest = code[i + 3..].trim_start();
            if let Some(q) = rest.strip_prefix('"') {
                let kind = &q[..q.find('"').unwrap_or(q.len())];
                assert!(accepted.contains(kind), "{file}: ev({kind:?}) is not an accepted ui_event action {accepted:?}");
                seen.insert(kind.to_string());
            }
        }
    }
    assert!(seen.len() >= 5, "found the page's ev( calls: {seen:?}");
}

/// Review M9: `#sheet` is the one dialog, so it carries the sheet's name: `showSheet` copies the `aria-labelledby` the
/// sheet's own surface (`.sheet-in`) names onto `#sheet` after it renders, and clears the last sheet's first.
#[test]
fn the_sheet_takes_its_name_from_the_sheet_it_shows() {
    let shell = code_of("js/shell.js");
    let show = fn_body(&shell, "function showSheet(");
    let render = show.find("req.mod.render(").expect("showSheet renders the sheet");
    let clear = show.find("host.removeAttribute(\"aria-labelledby\")").expect("the last sheet's name is cleared");
    let copy = show.find("host.setAttribute(\"aria-labelledby\"").expect("the name is copied onto #sheet");
    assert!(clear < render && render < copy, "cleared before the render, copied after it: {show}");
    assert!(show.contains(".sheet-in[aria-labelledby]"), "from the sheet's own surface");
}

/// Review M1: in the dark theme a floating surface (--pop) was the very fill of a field, a chosen segment and a hover
/// tint (--line-2), so on a sheet, the panel, Status or the layer those vanished. shell.css points --line-2 elsewhere
/// on every floating surface, once; no page patches it locally.
#[test]
fn dark_floating_surfaces_keep_fields_and_segments_visible() {
    let css = strip_css_comments(&read("css/shell.css"));
    let at = css.find(":root[data-theme=\"dark\"] :is(").expect("one dark-theme rule for the floating surfaces");
    let rule = &css[at..at + css[at..].find('}').unwrap()];
    for surface in [".sbar", ".panel", ".sheet-in", ".layer-in", ".kn-tip"] {
        assert!(rule.contains(surface), "the rule covers {surface}: {rule}");
    }
    assert!(rule.contains("--line-2:") && !rule.contains("--line-2: var(--pop)") && !rule.contains("--line-2: var(--shade-27)"), "--line-2 is not the surface's own fill: {rule}");
    for (f, text) in styles() {
        let code = strip_css_comments(&text);
        for patch in [".ws .field", ".sh .field", ".ws .seg", ".sh .seg"] {
            assert!(!code.contains(patch), "{f}: a local dark-theme patch ({patch})");
        }
    }
}

// ---- shell polish for the swap (T05, before T23) ------------------------------------------------

/// The fields only a mail-derived card or row carries; `from` is a common word (a commitment's first
/// day, a registrar change's old value), so it counts as a mail field only on an object that carries one
/// of these too.
const MAIL_ONLY: [&str; 3] = ["mail_date", "evidence", "sender"];

/// `x["field"]` and `x['field']` read as `x.field` for every mail field, so a bracket read is scanned
/// like a dot read.
fn dot_brackets(code: &str) -> String {
    let mut out = code.to_string();
    for f in MAIL_FIELDS {
        for q in ['"', '\''] {
            out = out.replace(&format!("[{q}{f}{q}]"), &format!(".{f}"));
        }
    }
    out
}

/// The object a read is made on: `c` for `h(c.from` up to `.from`.
fn receiver<'a>(upto: &'a str, field: &str) -> &'a str {
    let base = &upto[..upto.len() - field.len() - 1];
    base.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).next().unwrap_or("")
}

/// Every read of a mail field in `code` (dot or bracket), as (line number, line, the line up to the end
/// of the read): every `mail_date`, `evidence` and `sender`, and `from` only on an object the same file
/// also reads one of those on. The scan exists for the mail card and the applied rows, so a
/// commitment's `cm.from` is not a mail field.
fn mail_reads(code: &str) -> Vec<(usize, String, String)> {
    let code = dot_brackets(code);
    let mut mail_objects = BTreeSet::new();
    for (_, _, upto) in field_reads(&code, &MAIL_ONLY) {
        let f = MAIL_ONLY.iter().find(|f| upto.ends_with(&format!(".{f}"))).unwrap();
        mail_objects.insert(receiver(&upto, f).to_string());
    }
    let mut out: Vec<(usize, String, String)> = field_reads(&code, &MAIL_ONLY).into_iter().map(|(n, l, u)| (n, l.to_string(), u)).collect();
    for (n, line, upto) in field_reads(&code, &["from"]) {
        if mail_objects.contains(receiver(&upto, "from")) { out.push((n, line.to_string(), upto)); }
    }
    out
}

/// The narrowed mail scan: it flags a mail field read outside `h()` on a mail object, dot or bracket, and
/// leaves a `from` on an object that carries no mail field alone, so the Calendar reads a commitment's
/// `cm.from` plainly (the bracket workaround is gone).
#[test]
fn the_mail_scan_reads_mail_objects_and_bracket_reads() {
    for bad in [
        "var who = x[\"from\"] + \" \"; var q = h(x.evidence);",
        "r = '<p>' + c['sender'] + '</p>';",
        "r = h(m.when) + m[\"mail_date\"];",
        "t = c.from; u = c.mail_date ? h(c.mail_date) : \"\";",
    ] {
        let reads = mail_reads(bad);
        assert!(reads.iter().any(|(_, l, u)| !read_is_safe(l, u)), "should be rejected: {bad}");
    }
    for good in ["r = h(c[\"from\"]) + h(c.evidence);", "r = c.from ? h(c.from) : \"\"; s = h(c.sender);", "var first = cm.from ? String(cm.from).slice(0, 10) : \"\";", "var v = e.from + e.to;"] {
        let reads = mail_reads(good);
        assert!(reads.iter().all(|(_, l, u)| read_is_safe(l, u)), "should pass: {good}");
    }
    assert!(mail_reads("var a = cm.from; var b = String(cm[\"from\"]);").is_empty(), "a from on an object with no mail field is not a mail read");
    let deck = code_of("js/carried/deck.js");
    assert!(mail_reads(&deck).iter().filter(|(_, _, u)| u.ends_with("c.from")).count() >= 2, "the deck's card line still has its from reads scanned");
    let cal = code_of("js/pages/calendar_data.js");
    assert!(cal.contains("cm.from ? String(cm.from)") && !cal.contains("cm[\"from\"]") && !cal.contains("cm[\"until\"]"), "the Calendar reads a commitment's from and until plainly");
}

/// `KnowluKit.toast(text, action, fn, isError, ms)`: an optional duration, 6 s (8 s for an error) when it
/// is not a number; it answers the toast's element, so a page that keeps one toast at a time can remove
/// the last one.
#[test]
fn the_toast_takes_an_optional_duration() {
    let kit = code_of("js/kit.js");
    let toast = fn_body(&kit, "function toast(");
    assert!(toast.starts_with("function toast(text, action, fn, isError, ms) {"), "the duration is the fifth argument: {toast}");
    assert!(toast.contains("setTimeout(gone, typeof ms === \"number\" ? ms : (isError ? 8000 : 6000));"), "the duration, else the defaults");
    assert!(toast.contains("return t;"), "it answers the element");
    assert!(read("js/kit.js").contains("KnowluKit.toast(text, action, fn, isError, ms)"), "the header names the new argument");
}

/// `KnowluKit.check(row, {quiet: true})` writes the same `set_fields` and shows no "Done · Undo" toast,
/// for a page with an Undo of its own (Lock in's beat). A refusal is still said.
#[test]
fn a_quiet_check_shows_no_done_toast() {
    let kit = code_of("js/kit.js");
    let check = fn_body(&kit, "check: function (");
    assert!(check.starts_with("check: function (row, opts) {"), "check takes options: {check}");
    let quiet = check.find("!(opts && opts.quiet)").expect("the Done toast is skipped when quiet");
    let done = check.find("toast(\"Done\", \"Undo\"").expect("the Done toast");
    assert!(quiet < done, "the quiet test guards the Done toast: {check}");
    assert!(check.contains("toast(refusal(env), null, null, true)"), "a refusal is still said, quiet or not");
    assert!(read("js/kit.js").contains("KnowluKit.check(row, opts)"), "the header names the option");
}

/// Whole-branch review I2: the engine's `revision` leaves `now` out (`engine/src/surface.rs`), so a
/// poll whose revision is unchanged still carries a later minute. `arrived()` copies the answer's
/// `now` into the state on screen and into a held one before the unchanged-revision return, which
/// tells no `on` listener; the shell redraws the band's clock on every poll. The DOM check is
/// `scripts/ui-fixture/checks/t05.py`.
#[test]
fn now_moves_between_revisions() {
    let data = code_of("js/data.js");
    let arrived = fn_body(&data, "function arrived(");
    let same = arrived.find(".revision === REV").expect("the unchanged-revision test");
    let s_now = arrived.find("S.now = st.now").expect("an unchanged revision still updates the state's now");
    let held_now = arrived.find("HELD.now = st.now").expect("and a held state's now");
    assert!(s_now < same && held_now < same, "now is copied before the unchanged-revision return: {arrived}");
    let ret = line_with(arrived, ".revision === REV");
    assert!(ret.contains("return;") && !ret.contains("emit(") && !ret.contains("paint("), "an unchanged revision tells no on listener: {ret}");
    let shell = code_of("js/shell.js");
    let start = fn_body(&shell, "start: function (");
    assert!(start.contains("KnowluData.onPoll(drawClock)"), "the band's clock redraws on every poll");
}
