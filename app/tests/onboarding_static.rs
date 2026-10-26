//! F11t: the onboarding screens' static pins (wave B long-onboarding plan, F11t and F11a-F11f).
//!
//! Written before `app/static/onboarding.{js,css}` exist, so every test here fails first on the
//! missing file. Nothing here is satisfiable by a comment: every pin reads the page's code with its
//! comments removed (`code`), and a behaviour pin runs the page's own helper under `node`.
//!
//! **Conventions these tests fix, for F11a-F11e to follow** (the plan names behaviour, not ids):
//! - `onboarding.js` is a classic script (no modules), loaded by `index.html` beside `console.js`.
//! - Pure helpers hang on `window.KNOWLU_ONBOARDING`: `exitLine`, `elapsedText`, `reasonWords`,
//!   `costWords`, `openKnowluEnabled`, `isClosing`, `step4Model`, `revealModel`, `railRow`.
//! - Named handlers: `finishLater`, `signOut`, `goNext`, `goBack`, `closeSchoolWindows`,
//!   `clearCredentialFields`, `removeCourse`, `renderRestart`.
//! - Panels are `id="ob-step-N"` (N = 2..8, 10, 11; there is no step 9 in wave B); skip controls
//!   are `ob-skip-5`, `ob-skip-6`, `ob-skip-7`, `ob-skip-8-zybooks`, `ob-skip-8-vhl`.
//! - Every Tauri call is `invoke("<literal name>", ...)`. The one exception is a single wrapper,
//!   either `var invoke = window.__TAURI__.core.invoke;` or one `function invoke(cmd, args) {...}`
//!   whose body holds the only `__TAURI__.core.invoke`; `invoke` is never aliased or passed on.
//! - Handlers are named `function`s (the pins find the smallest enclosing function; arrow bodies
//!   count too). A call placed straight in the file-wide IIFE wrapper fails the pin on purpose.
//! - Regex literals in `onboarding.js` must not contain quotes, `//` or `/*` (the comment stripper
//!   reads simple regex literals only).
//! - Style the pins rely on: markup is built as HTML strings or `createElement("button")`;
//!   promise chains use `.then(` / `.catch(` / `.finally(`; step 8 declares `var discovering`;
//!   no `reduce(`; the four login fields are looked up by their literal ids
//!   (`ob-zybooks-username`, `ob-zybooks-password`, `ob-vhl-username`, `ob-vhl-password`) only
//!   inside the store handler and `clearCredentialFields`.
//! - `onboarding_status` carries `finish_ready` (server side); the page never re-derives it.

use std::fs;
use std::path::Path;
use std::process::Command;

fn read(rel: &str) -> String {
    fs::read_to_string(Path::new("static").join(rel)).unwrap_or_else(|e| panic!("static/{rel}: {e} (the onboarding screens are not written yet)"))
}

/// `src` with every comment turned to spaces, strings kept, length and newlines unchanged.
fn code(src: &str) -> String { blank(src, false) }
/// `src` with comments and string contents turned to spaces, length unchanged (for brace matching).
fn skeleton(src: &str) -> String { blank(src, true) }

fn blank(src: &str, strings_too: bool) -> String {
    // Every blanked char becomes `len_utf8()` spaces (a newline stays one), so byte offsets in the
    // result are byte offsets in `src`, even past a multibyte char such as an ellipsis.
    fn pad(out: &mut String, ch: char) {
        if ch == '\n' { out.push('\n'); } else { for _ in 0..ch.len_utf8() { out.push(' '); } }
    }
    const AFTER: &str = "(,=:[!&|?{};+-*%<>~^}";
    const KEYWORDS: [&str; 13] = ["return", "typeof", "case", "in", "of", "delete", "void", "throw", "new", "else", "do", "yield", "await"];
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let (mut i, mut prev, mut word) = (0, '\0', String::new());
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' { pad(&mut out, c[i]); i += 1; }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) { pad(&mut out, c[i]); i += 1; }
            for _ in 0..2 { if i < c.len() { pad(&mut out, c[i]); i += 1; } }
        } else if ch == '/' && (prev == '\0' || AFTER.contains(prev) || KEYWORDS.contains(&word.as_str())) {
            // A regex literal: to the next unescaped `/` outside a class.
            out.push('/');
            i += 1;
            let mut class = false;
            while i < c.len() && c[i] != '\n' && (class || c[i] != '/') {
                if c[i] == '\\' && i + 1 < c.len() { if strings_too { pad(&mut out, c[i]); } else { out.push(c[i]); } i += 1; }
                if c[i] == '[' { class = true; }
                if c[i] == ']' { class = false; }
                if strings_too { pad(&mut out, c[i]); } else { out.push(c[i]); }
                i += 1;
            }
            if i < c.len() && c[i] == '/' { out.push('/'); i += 1; }
            prev = ')';
            word.clear();
        } else if ch == '"' || ch == '\'' || ch == '`' {
            out.push(ch);
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' && i + 1 < c.len() { if strings_too { pad(&mut out, c[i]); } else { out.push(c[i]); } i += 1; }
                if c[i] == '\n' && ch != '`' { break; }
                if strings_too { pad(&mut out, c[i]); } else { out.push(c[i]); }
                i += 1;
            }
            if i < c.len() && c[i] == ch { out.push(ch); i += 1; }
            prev = ')';
            word.clear();
        } else {
            out.push(ch);
            if ch.is_alphanumeric() || ch == '_' || ch == '$' { word.push(ch); } else if !ch.is_whitespace() { word.clear(); }
            if !ch.is_whitespace() { prev = ch; }
            i += 1;
        }
    }
    out
}

#[test]
fn the_stripper_keeps_byte_offsets_and_reads_regex_literals() {
    let src = "var s = \"Finishing\u{2026}\"; // n\u{e9} here\nvar re = /[\"']x/; function f() { return /\"/.test(s) ? 1 : 2; }\nfunction g() { invoke(\"a\"); /* \u{2026} */ }\n";
    assert_eq!(code(src).len(), src.len(), "code() keeps byte offsets");
    assert_eq!(skeleton(src).len(), src.len(), "skeleton() keeps byte offsets");
    assert!(!code(src).contains("n\u{e9} here") && code(src).contains("Finishing\u{2026}"), "comments go, strings stay");
    assert!(!skeleton(src).contains("Finishing"), "the skeleton blanks strings");
    // Two functions and a long file: the whole-file refusal does not apply.
    let at = src.find("return").unwrap();
    assert_eq!(enclosing_fn(src, at), "function f() { return /\"/.test(s) ? 1 : 2; }");
    let at = src.find("invoke").unwrap();
    assert_eq!(enclosing_fn(src, at), "function g() { invoke(\"a\"); /* \u{2026} */ }");
    assert_eq!(invoked(&code(src)), ["a"]);
    assert!(invoked("var t = \"call invoke now\"; function h() { invoke(\"b\"); }").contains(&"b".to_string()), "invoke inside a string is not a call");
}

/// The source of the smallest `function` (or block-bodied arrow) whose body holds byte `at`,
/// anonymous callbacks included. Refuses a span that is the whole file (an IIFE wrapper).
fn enclosing_fn(src: &str, at: usize) -> &str {
    let sk = skeleton(src);
    let mut best: Option<(usize, usize)> = None;
    let mut starts: Vec<(usize, usize)> = Vec::new(); // (span start, body `{`)
    for (start, _) in sk.match_indices("function") {
        if let Some(o) = sk[start..].find('{') { starts.push((start, start + o)); }
    }
    for (arrow, _) in sk.match_indices("=>") {
        let rest = &sk[arrow + 2..];
        let ws = rest.len() - rest.trim_start().len();
        if rest.trim_start().starts_with('{') { starts.push((arrow, arrow + 2 + ws)); }
    }
    for (start, open) in starts {
        let (mut depth, mut end) = (0i32, None);
        for (k, b) in sk[open..].bytes().enumerate() {
            if b == b'{' { depth += 1; }
            if b == b'}' { depth -= 1; if depth == 0 { end = Some(open + k + 1); break; } }
        }
        if let Some(end) = end { if start < at && at < end && best.is_none_or(|(s, _)| start > s) { best = Some((start, end)); } }
    }
    let (s, e) = best.unwrap_or_else(|| panic!("no function encloses byte {at}"));
    assert!(
        (e - s) * 10 < src.trim().len() * 9,
        "the only function around byte {at} is the whole file (an IIFE wrapper): put the code in a named handler"
    );
    &src[s..e]
}

/// The body of `function name(...) {...}`.
fn function<'a>(src: &'a str, name: &str) -> &'a str {
    let at = src.find(&format!("function {name}(")).unwrap_or_else(|| panic!("onboarding.js has no function {name}("));
    let open = at + src[at..].find('{').expect("a body");
    enclosing_fn(src, open + 1)
}

/// Every command name passed to `invoke(`. A call must name its command by a literal, wherever
/// `invoke` is written (`.invoke(` included). Exempt: a `function invoke(` definition with its body
/// (the one wrapper) and the one `var invoke = ...__TAURI__.core.invoke` assignment. Any other
/// use of the word (an alias, a pass-through) panics.
fn invoked(js: &str) -> Vec<String> {
    let sk = skeleton(js);
    let wrapper = sk.find("function invoke(").map(|at| {
        let open = at + sk[at..].find('{').expect("a wrapper body");
        let mut depth = 0i32;
        let mut end = sk.len();
        for (k, b) in sk[open..].bytes().enumerate() {
            if b == b'{' { depth += 1; }
            if b == b'}' { depth -= 1; if depth == 0 { end = open + k + 1; break; } }
        }
        (at, end)
    });
    let mut names = Vec::new();
    for (at, _) in sk.match_indices("invoke") {
        let edge = |ch: Option<char>| ch.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if !edge(sk[..at].chars().last()) || !edge(sk[at + 6..].chars().next()) { continue; }
        if wrapper.is_some_and(|(s, e)| at >= s && at < e) { continue; }
        if !sk[at + 6..].trim_start().starts_with('(') {
            let line = format!("{}{}", js[..at].rsplit('\n').next().unwrap_or(""), js[at..].split('\n').next().unwrap_or(""));
            assert!(line.contains("__TAURI__.core.invoke") && line.trim_start().starts_with("var invoke"), "invoke is used without a call (an alias?): {line}");
            continue;
        }
        let arg = js[at + 6..].trim_start()[1..].trim_start();
        let quote = arg.chars().next().unwrap_or(' ');
        assert!(quote == '"' || quote == '\'', "invoke( must take a literal command name, got: {}", &arg[..arg.len().min(40)]);
        names.push(arg[1..].chars().take_while(|&ch| ch != quote).collect());
    }
    names
}

/// Runs `expr` with `h` bound to `window.KNOWLU_ONBOARDING` after loading `onboarding.js` under a
/// stub page; the result comes back as JSON. No network, no real DOM.
fn node(expr: &str) -> serde_json::Value {
    read("onboarding.js");
    const HARNESS: &str = r#"
const fs = require("fs"), vm = require("vm");
process.on("unhandledRejection", () => {});
const stub = () => new Proxy(function () {}, { get: (t, k) => (k === Symbol.toPrimitive ? () => "" : stub()), apply: () => stub(), construct: () => stub(), set: () => true });
const store = new Map();
const g = {
  document: stub(),
  location: { search: "", hash: "", href: "http://localhost/", pathname: "/", origin: "http://localhost" },
  URLSearchParams, URL, console, Date, JSON, Math,
  navigator: { userAgent: "node", language: "en" },
  localStorage: { getItem: (k) => (store.has(k) ? store.get(k) : null), setItem: (k, v) => { store.set(k, String(v)); }, removeItem: (k) => { store.delete(k); }, clear: () => store.clear() },
  requestAnimationFrame: () => 0, cancelAnimationFrame() {},
  setTimeout: () => 0, setInterval: () => 0, clearTimeout() {}, clearInterval() {},
  addEventListener() {}, removeEventListener() {},
  fetch: () => { throw new Error("tests never leave the machine"); },
  XMLHttpRequest: function () { throw new Error("tests never leave the machine"); },
  WebSocket: function () { throw new Error("tests never leave the machine"); },
  KNOWLU_COPY: { GOOGLE_TESTING: "GT" }, KNOWLU_SHOTS: {},
  __TAURI__: { core: { invoke: () => new Promise(() => {}) } },
};
g.window = g;
const ctx = vm.createContext(g);
vm.runInContext(fs.readFileSync(process.argv[1], "utf8"), ctx);
const h = g.KNOWLU_ONBOARDING;
// `eval` runs only the expression this test file passes in; it is not page or user input.
if (!h) { console.log(JSON.stringify({ __missing: true })); } else { console.log(JSON.stringify(eval(process.env.OB_EXPR))); }
"#;
    let out = Command::new("node").arg("-e").arg(HARNESS).arg(Path::new("static/onboarding.js"))
        .env("OB_EXPR", expr).output().expect("node must be installed to run the onboarding page's helpers");
    assert!(out.status.success(), "node failed: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("helper output is JSON");
    assert!(v.get("__missing").is_none(), "onboarding.js must set window.KNOWLU_ONBOARDING");
    v
}

// ---- F11a: the assets, the frame, the commands the page may call ---------------------------------

const BANNED: &[&str] = &[
    "entitlement_now", "open_checkout", "attach_account", "google_connect_url", "google_connected", "sync", "backup_now",
    "check_for_updates", "install_update", "hold_for_install", "capture_calendar_link", "paste_calendar_link",
    "sign_out", "switch_profile", "delete_my_data", "open_external", "retarget_credentials",
    // The queue's mail.setup owns the run; a page-started run would not be drained (ruling, review r2).
    "mail_setup_start",
    // F11c1 review m-3: the path-taking originals stay off the page in any form (section 14.57.1).
    "store_credentials", "discover_coursework",
];

/// Only commands the plan names: section 4.5's list, F5's, the step-2 sign-in, step 5's
/// `mail_setup_progress` (read-only; plan line 643), step 10's confirm screen and `launch_state`
/// (plan line 3053). Step 5 never starts mail work: `mail_setup_start` is banned.
const ALLOWED: &[&str] = &[
    "onboarding_status", "onboarding_answer", "onboarding_skip", "onboarding_retry", "onboarding_finish", "onboarding_quit",
    "onboarding_sign_out", "onboarding_attach_session", "onboarding_entitlement", "onboarding_checkout",
    "onboarding_capture_link", "onboarding_paste_link", "campus_search",
    "open_lms_window", "capture_courses", "close_lms_window", "google_sign_in", "send_magic_link", "verify_email_code",
    "google_connect", "google_status", "mail_setup_progress",
    "your_week", "commitment_proposals", "commitments_confirm", "launch_state",
    // F11p1 (section 14.58.4): the rail's account and payment lines, read from this computer only.
    "account_status",
    // F11c review I-1: the console's own edit of a confirmed commitment (its kind and its level).
    "set_fields",
    // F11c1 (section 14.57.1): step 8's two console wrappers, which read the vault from ConsoleState.
    "onboarding_discover_coursework", "onboarding_store_credentials",
];

#[test]
fn the_onboarding_assets_exist_and_index_html_loads_them() {
    let js = read("onboarding.js");
    let css = read("onboarding.css");
    assert!(!js.trim().is_empty() && !css.trim().is_empty());
    let html = read("index.html");
    assert!(html.contains("onboarding.js") && html.contains("onboarding.css"), "index.html loads both new files");
    assert!(!html.contains("type=\"module\""), "a classic script, like console.js");
    let (con, onb) = (html.find("console.js").expect("console.js is loaded"), html.find("onboarding.js").unwrap());
    assert!(con < onb, "onboarding.js loads after console.js, which defines KNOWLU_COPY and KNOWLU_SHOTS");
    for (name, text) in [("onboarding.js", code(&js)), ("onboarding.css", code(&css))] {
        let text = text.replace("http://www.w3.org/2000/svg", "");
        assert!(!text.contains("http://") && !text.contains("https://"), "{name} references the network");
    }
    let c = code(&js);
    assert_eq!(c.matches("__TAURI__.core.invoke").count(), 1, "calls go through the one invoke (var or wrapper)");
    assert!(c.contains("KNOWLU_ONBOARDING"), "the pure helpers are exposed on window.KNOWLU_ONBOARDING");
}

#[test]
fn every_onboarding_call_is_a_literal_on_the_allowed_list_and_none_is_banned() {
    let c = code(&read("onboarding.js"));
    let names = invoked(&c);
    assert!(names.len() >= 12, "the screens call their commands ({} found)", names.len());
    for n in &names {
        assert!(!BANNED.contains(&n.as_str()), "onboarding must not call {n}");
        assert!(ALLOWED.contains(&n.as_str()), "{n} is outside section 4.5's list and F5's");
    }
    // A name hidden in a table or a variable would dodge invoke(; no string may be a banned name.
    for b in BANNED.iter().filter(|b| **b != "sync") {
        assert!(!c.contains(&format!("\"{b}\"")) && !c.contains(&format!("'{b}'")), "{b} appears as a string in onboarding.js");
    }
}

#[test]
fn each_step_calls_the_commands_the_plan_gives_it() {
    let names = invoked(&code(&read("onboarding.js")));
    for (step, cmds) in [
        ("every screen", &["onboarding_status", "onboarding_quit", "onboarding_sign_out", "onboarding_answer", "onboarding_skip", "onboarding_retry", "onboarding_finish"][..]),
        ("step 2 again", &["onboarding_attach_session", "google_sign_in", "send_magic_link", "verify_email_code"]),
        ("step 3", &["campus_search", "open_lms_window", "close_lms_window", "onboarding_capture_link", "onboarding_paste_link"]),
        ("step 4", &["onboarding_checkout", "onboarding_entitlement"]),
        ("step 6", &["google_connect", "google_status"]),
        ("step 7", &["onboarding_paste_link"]),
        ("step 8", &["onboarding_store_credentials", "onboarding_discover_coursework"]),
        ("step 10", &["your_week", "commitment_proposals", "commitments_confirm"]),
    ] {
        for c in cmds { assert!(names.iter().any(|n| n == c), "{step} must call {c}"); }
    }
}

#[test]
fn no_onboarding_screen_updates_installs_or_reads_a_slots_step_rows() {
    let c = code(&read("onboarding.js"));
    assert!(!c.contains("install_update") && !c.contains("hold_for_install"), "the onboarding page never installs an update");
    for x in ["renderFirstRun", "first_run", "RunSummary", ".steps"] {
        assert!(!c.contains(x), "the rail reads onboarding_status jobs only, not {x}");
    }
    assert!(c.contains(".jobs"), "the rail reads onboarding_status's jobs");
}

#[test]
fn finish_later_and_sign_out_are_on_every_frame_and_call_only_their_commands() {
    let c = code(&read("onboarding.js"));
    for id in ["ob-finish-later", "ob-sign-out", "ob-rail", "ob-why", "ob-ask", "ob-proof"] {
        assert!(c.contains(id), "the step frame needs #{id}");
    }
    for st in ["intro", "asking", "reading", "proof", "failed", "skipped", "already-done"] {
        assert!(c.contains(&format!("\"{st}\"")), "section 4.1's state {st} is rendered");
    }
    let later = function(&c, "finishLater");
    assert!(invoked(later).contains(&"onboarding_quit".to_string()) && !later.contains("onboarding_sign_out"), "Finish later calls onboarding_quit");
    let out = function(&c, "signOut");
    assert!(invoked(out).contains(&"onboarding_sign_out".to_string()) && !out.contains("onboarding_quit"), "Sign out calls onboarding_sign_out");
    assert!(c.contains("finishLater") && c.contains("signOut"));
}

#[test]
fn a_restart_answer_shows_one_screen_whose_only_button_quits() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains(".restart"), "a {{restart: true}} answer is recognised");
    let screen = function(&c, "renderRestart");
    assert!(c.contains("ob-restart") && screen.contains("ob-restart"), "the restart screen has its id");
    assert_eq!(screen.matches("<button").count() + screen.matches("createElement(\"button\")").count(), 1, "the restart screen has exactly one button");
    assert!(screen.contains("onboarding_quit") || screen.contains("finishLater"), "its button exits through onboarding_quit");
}

#[test]
fn the_shots_hook_opens_the_frame_at_a_step() {
    let ours = code(&read("onboarding.js"));
    let flat = ours.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("KNOWLU_SHOTS.startOnboarding =") || flat.contains("Object.assign(window.KNOWLU_SHOTS"), "KNOWLU_SHOTS.startOnboarding(step) exists");
    assert!(!flat.contains("KNOWLU_SHOTS = {") && !flat.contains("KNOWLU_SHOTS={"), "onboarding.js extends console.js's KNOWLU_SHOTS, never replaces it");
    assert!(ours.contains("fixture"), "the screens render from the console's ?fixture= path");
}

#[test]
fn a_not_yet_row_in_the_first_run_view_reads_as_skipped_never_done() {
    let js = code(&read("console.js"));
    let render = js.split("function renderFirstRun(").nth(1).and_then(|s| s.split("function hideFirstRun(").next()).expect("renderFirstRun");
    assert!(render.contains("(not yet:"), "renderFirstRun classes a `(not yet:` row (review r6, M1)");
}

// ---- F11a: the exit lines, the elapsed time, the words for reasons and costs ----------------------

const SINCE: &str = "2026-10-24T12:00:00.000Z";

fn exit_line(job: &str, long: bool, after_ms: i64) -> String {
    let expr = format!(
        "h.exitLine({{job: \"{job}\", long: {long}, since: \"{SINCE}\"}}, Date.parse(\"{SINCE}\") + {after_ms})"
    );
    node(&expr).as_str().expect("exitLine returns a string").to_string()
}

#[test]
fn a_job_being_finished_is_named_from_the_start_and_never_by_its_raw_key() {
    for (job, long) in [("mail.setup", false), ("school.capture", false), ("slot.partial", true)] {
        let line = exit_line(job, long, 3_000);
        assert!(line.starts_with("Finishing ") && line.ends_with('\u{2026}'), "{line:?}");
        assert!(!line.contains(job), "the raw job key {job} is never shown: {line:?}");
    }
}

#[test]
fn a_lane_job_past_fifteen_seconds_says_knowlu_closes_by_itself() {
    let line = exit_line("school.capture", false, 15_000);
    assert!(line.starts_with("Still finishing "), "{line:?}");
    assert!(line.ends_with(". Knowlu closes by itself within a minute or two, once it's saved."), "{line:?}");
    assert!(!line.contains("20 minutes"), "only a slot can take that long: {line:?}");
    assert!(exit_line("school.capture", false, 14_999).starts_with("Finishing "), "before 15 s the first line stands");
}

#[test]
fn a_slot_past_fifteen_seconds_says_twenty_minutes_and_counts_up_from_since() {
    let line = exit_line("slot.partial", true, 75_000);
    assert!(
        line.contains("Saving what Knowlu has read so far. This can take up to 20 minutes, and Knowlu closes by itself when it's done."),
        "{line:?}"
    );
    assert!(line.contains("1 min 15 s"), "the elapsed time since `since`, 75 s ago: {line:?}");
    assert!(exit_line("slot.partial", true, 135_000).contains("2 min 15 s"), "it ticks up");
    assert!(exit_line("slot.partial", true, 15_000).contains("0 min 15 s"));
    assert!(!line.contains("remaining") && !line.contains("left") && !line.contains("estimate"), "it shows no estimate: {line:?}");
    assert!(!exit_line("slot.partial", true, 3_000).contains("20 minutes"), "the slot line starts at 15 s");
}

#[test]
fn the_elapsed_time_counts_up_only_and_never_goes_negative() {
    let f = |ms: i64| node(&format!("h.elapsedText(\"{SINCE}\", Date.parse(\"{SINCE}\") + {ms})")).as_str().unwrap().to_string();
    assert_eq!(f(75_000), "1 min 15 s");
    assert_eq!(f(0), "0 min 0 s");
    assert_eq!(f(3_725_000), "62 min 5 s");
    assert_eq!(f(-5_000), "0 min 0 s", "a clock behind `since` reads zero, never negative");
}

#[test]
fn a_null_exiting_while_an_exit_call_is_outstanding_is_still_closing() {
    let f = |outstanding: bool, exiting: &str| node(&format!("h.isClosing({outstanding}, {exiting})")).as_bool().unwrap();
    assert!(f(true, "null"), "after the drain returns, save, flush and relaunch still run (F8f2 review)");
    assert!(f(true, "{job: \"slot.partial\", long: true, since: \"x\"}"));
    assert!(f(false, "{job: \"mail.setup\", long: false, since: \"x\"}"));
    assert!(!f(false, "null"), "only with no call outstanding and no drain is the page not closing");
}

#[test]
fn every_reason_key_reaches_the_student_as_words() {
    let keys = ["first_sync_failed", "job_crashed", "grades_read_failed", "courses_not_recorded", "feeds_not_settled", "lms_link_missing", "calendar_link_missing"];
    for k in keys {
        let w = node(&format!("h.reasonWords(\"{k}\")")).as_str().unwrap().to_string();
        assert!(w.contains(' ') && !w.contains('_') && w != k && !w.contains(k), "{k} -> {w:?}");
    }
    let unknown = node("h.reasonWords(\"some_new_reason_key\")").as_str().unwrap().to_string();
    assert!(!unknown.contains('_') && !unknown.is_empty(), "an unmapped key still never shows raw: {unknown:?}");
    let sentence = node("h.reasonWords(\"waiting to reach your account\")").as_str().unwrap().to_string();
    assert!(sentence.contains("waiting to reach your account"), "a reason that is already a sentence passes through: {sentence:?}");
}

#[test]
fn the_crash_guard_and_grades_failures_use_the_plans_exact_fixes() {
    let w = |k: &str| node(&format!("h.reasonWords(\"{k}\")")).as_str().unwrap().to_string();
    assert_eq!(w("job_crashed"), "Knowlu stopped twice while this step was running, so it has paused it. Press Try again to run it once more.");
    assert_eq!(
        w("grades_read_failed"),
        "Knowlu couldn't read your grades this time. It will try again on its next run, or press Try again now. If it keeps failing, reconnect your school in Settings after setup."
    );
}

#[test]
fn no_reason_is_written_to_the_page_without_passing_through_the_words_table() {
    let c = code(&read("onboarding.js"));
    for (at, _) in c.match_indices(".reason") {
        let from = c[..at].rfind([';', '{', '}']).map_or(0, |n| n + 1);
        let to = at + c[at..].find(';').unwrap_or(c.len() - at);
        let mut stmt = c[from..to].trim_start();
        if stmt.starts_with("if (") {
            // Skip only the condition: `if (j.reason) el.textContent = j.reason;` still writes.
            let (mut depth, mut cut) = (0i32, stmt.len());
            for (k, ch) in stmt.char_indices().skip(3) {
                if ch == '(' { depth += 1; }
                if ch == ')' { depth -= 1; if depth == 0 { cut = k + 1; break; } }
            }
            stmt = &stmt[cut..];
            if !stmt.contains(".reason") { continue; }
        }
        if stmt.contains("textContent") || stmt.contains("innerHTML") || stmt.contains("insertAdjacent") || stmt.contains("+=") || stmt.contains("setAttribute") {
            assert!(stmt.contains("reasonWords("), "a raw reason is written to the page: {stmt}");
        }
    }
}

#[test]
fn every_skip_cost_key_has_words_and_none_shows_the_key() {
    let w = |k: &str| node(&format!("h.costWords(\"{k}\")")).as_str().unwrap().to_string();
    assert!(w("mail_dates_missed").contains("date changes and new work announced by email are missed"));
    assert!(w("gmail_and_calendar_missed").contains("club, job and internship tasks from Gmail are missed; your own events are not counted as busy time"));
    assert!(w("calendar_busy_time_missed").contains("busy time"));
    for site in ["zybooks", "vhl"] {
        let words = w(&format!("homework_due_dates_missing_{site}"));
        assert!(words.contains("due dates"), "{words:?}");
    }
    for k in ["mail_dates_missed", "gmail_and_calendar_missed", "calendar_busy_time_missed", "homework_due_dates_missing_zybooks"] {
        assert!(!w(k).contains('_'), "{k} shows raw");
    }
}

// ---- F11a/F11d: the rail and the reveal -------------------------------------------------------------

#[test]
fn the_reveals_button_follows_the_servers_one_rule_and_re_derives_nothing() {
    // `finish_ready` is `finish_refusal(state).is_none()`, computed in Rust; the page reads it.
    let status = |patch: &str| {
        let mut v = serde_json::json!({"ok": true, "finish_ready": true, "exiting": null,
            "gate": {"open": true, "still_coming": [], "failed": [], "not_connected": []},
            "jobs": {"school.apply": {"state": "done"}, "school.courses": {"state": "done"}, "config.ingest": {"state": "done"}}});
        let patch: serde_json::Value = serde_json::from_str(patch).unwrap();
        for (k, val) in patch.as_object().unwrap() { v[k] = val.clone(); }
        v
    };
    let enabled = |s: serde_json::Value| node(&format!("h.openKnowluEnabled({s})")).as_bool().unwrap();
    assert!(enabled(status("{}")), "ok, finish_ready, gate open, not exiting");
    assert!(!enabled(status(r#"{"finish_ready": false}"#)), "finish_ready false");
    assert!(!enabled(status(r#"{"finish_ready": null}"#)), "an absent finish_ready is not ready");
    assert!(!enabled(status(r#"{"ok": false}"#)), "a failed status is not ready");
    // Amended by F11d (plan section 14.53.1, ruled): "`finish_ready` is `finish_refusal(state, now).is_none()`.
    // `finish_refusal` refuses whenever the gate is closed ... So the server never answers `finish_ready: true`
    // with `gate.open: false`, and the extra read only re-derives part of the rule on the page, which
    // section 14.52.6 forbids." The old case pinned that re-derivation; the page now reads `finish_ready` alone.
    assert!(enabled(status(r#"{"gate": {"open": false}}"#)), "the page reads finish_ready, not the gate");
    assert!(!enabled(status(r#"{"exiting": {"job": "slot.partial", "long": true, "since": "x"}}"#)), "while an exit drains");
    // The page does not re-derive job states: an absent registrar.confirm and a running
    // school.store change nothing when the server says ready; all-done jobs change nothing when not.
    assert!(enabled(status(r#"{"jobs": {"school.apply": {"state": "done"}, "school.store": {"state": "running"}}}"#)));
    assert!(!enabled(status(r#"{"finish_ready": false, "jobs": {"registrar.confirm": {"state": "done"}, "school.apply": {"state": "done"}, "school.courses": {"state": "done"}, "config.ingest": {"state": "done"}}}"#)));
    let c = code(&read("onboarding.js"));
    let f = function(&c, "openKnowluEnabled");
    assert!(f.contains("finish_ready") && !f.contains("jobs") && !f.contains("registrar.confirm") && !f.contains("gate"), "it reads finish_ready only");
}

#[test]
fn the_reveal_names_a_failed_item_in_words_with_a_try_again_and_a_long_chain_as_still_coming() {
    let status = serde_json::json!({"ok": true, "gate": {
        "open": true,
        "still_coming": ["first.grades", "backfill.older_mail"],
        "failed": [{"key": "first.sync", "reason": "first_sync_failed", "fix": "Try again"}],
        "not_connected": [{"key": "mail", "cost_key": "mail_dates_missed"}]}});
    let m = node(&format!("h.revealModel({status})"));
    let failed = m["failed"].as_array().expect("failed list");
    assert_eq!(failed.len(), 1, "a failed item is named, never hidden");
    let words = node("h.reasonWords(\"first_sync_failed\")");
    assert_eq!(failed[0]["text"], words, "the shown text is reasonWords(reason), not the server's one-word fix: {failed:?}");
    assert_ne!(failed[0]["text"], "Try again");
    assert_eq!(failed[0]["fix"], "Try again", "M7: the server's fix is carried as written, never re-worded: {failed:?}");
    assert_eq!(failed[0]["retry"], true, "beside a Try again control");
    let coming = m["stillComing"].as_array().expect("stillComing list");
    assert_eq!(coming.len(), 2, "every unfinished item is listed while the chain runs");
    for c in coming {
        let w = c["words"].as_str().expect("each item has words");
        assert!(!w.contains("first.") && !w.contains("backfill") && !w.contains('_'), "{w:?}");
    }
    let nc = m["notConnected"].as_array().expect("notConnected list");
    let cost = node("h.costWords(\"mail_dates_missed\")");
    assert_eq!(nc[0]["cost"], cost, "each skip shows its cost words");
}

#[test]
fn a_failed_job_on_the_rail_shows_its_reason_in_words_and_a_try_again() {
    let job = r#"{state: "failed", reason: "job_crashed"}"#;
    let row = node(&format!("h.railRow(\"slot.partial\", {job})"));
    assert_eq!(row["state"], "failed");
    let words = node("h.reasonWords(\"job_crashed\")");
    assert_eq!(row["line"], words, "the row's line is reasonWords(reason): {row}");
    assert_eq!(row["retry"], true);
    let done = node(r#"h.railRow("mail.setup", {state: "done"})"#);
    assert_eq!(done["retry"], false);
    let raw = row["line"].as_str().unwrap();
    assert!(!raw.contains("job_crashed") && !raw.contains("slot.partial"), "{raw}");
}

#[test]
fn step_ten_asks_for_proposals_only_when_the_status_says_they_are_ready() {
    let c = code(&read("onboarding.js"));
    let at = c.find("invoke(\"commitment_proposals\"").expect("step 10 reads the proposals");
    let f = enclosing_fn(&c, at);
    assert!(f.contains("proposals_ready"), "the call sits behind onboarding_status's proposals_ready");
    assert!(c.contains("reading your calendars"), "until then that part reads `reading your calendars...`");
    let confirm = enclosing_fn(&c, c.find("invoke(\"commitments_confirm\"").expect("confirm"));
    assert!(confirm.contains(".error") && c.contains("ob-step10-retry"), "a refused confirm shows its sentence and a Try again");
}

#[test]
fn the_main_pages_still_coming_line_reads_handover() {
    let js = code(&read("console.js"));
    assert!(js.contains("handover"), "console.js reads the hand-over list (F11d)");
    assert!(js.to_lowercase().contains("still coming"), "and says `still coming`");
}

// ---- F11d: the reveal's words, the not-connected mail line, the main page's "Not now", the seam --------

fn reveal_status() -> serde_json::Value {
    serde_json::json!({"ok": true, "finish_ready": true, "gate": {
        "open": true,
        "still_coming": ["backfill.older_mail"],
        "failed": [{"key": "first.sync", "reason": "first_sync_failed", "fix": "Check your connection, then try again"}],
        "not_connected": [{"key": "mail.connect", "cost_key": "mail_setup_unfinished"},
                          {"key": "mail", "cost_key": "mail_dates_missed"}]}})
}

#[test]
fn the_reveal_shows_the_servers_fix_beside_a_failed_items_words_and_its_try_again() {
    let html = node(&format!("h.revealHtml(h.revealModel({}), true)", reveal_status()));
    let html = html.as_str().expect("revealHtml(model, openEnabled) returns the reveal's markup as a string");
    let words = node("h.reasonWords(\"first_sync_failed\")");
    assert!(html.contains(words.as_str().unwrap()), "the item's words are shown: {html}");
    assert!(html.contains("Check your connection, then try again"), "M7: the server's fix is shown as written: {html}");
    assert!(html.contains("data-retry=\"first.sync\""), "beside the Try again control, which names the job it retries: {html}");
    assert!(!html.contains("first_sync_failed"), "no raw reason key: {html}");
}

#[test]
fn mail_connect_is_a_not_connected_line_with_a_settings_button_never_still_coming() {
    let m = node(&format!("h.revealModel({})", reveal_status()));
    let nc = m["notConnected"].as_array().expect("notConnected list");
    let mail = nc.iter().find(|n| n["key"] == "mail.connect").expect("mail.connect is listed as not connected");
    assert_eq!(mail["cost"], "Finish connecting your email in Settings.", "{mail}");
    assert_eq!(mail["settings"], true, "it carries its Settings button: {mail}");
    let other = nc.iter().find(|n| n["key"] == "mail").expect("the other skipped item");
    assert_eq!(other["settings"], false, "only mail.connect gets the Settings button: {other}");
    let coming: Vec<_> = m["stillComing"].as_array().unwrap().iter().map(|c| c["key"].as_str().unwrap().to_string()).collect();
    assert!(!coming.iter().any(|k| k == "mail.connect"), "never in still coming: {coming:?}");
    let html = node(&format!("h.revealHtml(h.revealModel({}), true)", reveal_status()));
    let html = html.as_str().unwrap();
    assert!(html.contains("Finish connecting your email in Settings."), "{html}");
    assert!(html.contains("data-act=\"settings\""), "the Settings button: {html}");
    assert!(!html.contains("mail.connect") && !html.contains("mail_setup_unfinished"), "no raw key: {html}");
}

#[test]
fn the_reveals_open_knowlu_button_is_disabled_exactly_when_the_rule_says() {
    let model = format!("h.revealModel({})", reveal_status());
    let on = node(&format!("h.revealHtml({model}, true)"));
    let off = node(&format!("h.revealHtml({model}, false)"));
    let (on, off) = (on.as_str().unwrap(), off.as_str().unwrap());
    let button = |h: &str| { let at = h.find("id=\"ob-open-knowlu\"").expect("an Open Knowlu button with id ob-open-knowlu"); h[h[..at].rfind('<').unwrap()..at + h[at..].find('>').unwrap()].to_string() };
    assert!(!button(on).contains("disabled"), "enabled: {}", button(on));
    assert!(button(off).contains("disabled"), "disabled: {}", button(off));
    assert!(on.contains("Open Knowlu"), "{on}");
}

#[test]
fn the_reveal_registers_through_the_seam_and_its_poll_never_redraws_the_pane() {
    let c = code(&read("onboarding.js"));
    let flat = c.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("OB.registerPanel(11, { render: render11, onStatus: onStatus11 })"), "the reveal registers through the seam");
    let body = function(&c, "onStatus11");
    assert!(!body.contains("refresh") && !body.contains(".show("), "onStatus11 must not redraw the pane (a poll wipes #ob-ask)");
    assert!(flat.matches("OB.registerLeave(").count() >= 2, "the reveal registers its own leave hook beside step 8's");
    assert!(function(&c, "render11").contains("revealHtml("), "the reveal is drawn by revealHtml");
}

#[test]
fn the_main_page_offers_not_now_beside_the_unfinished_mail_line_and_calls_the_command() {
    let js = code(&read("console.js"));
    let at = js.find("invoke(\"onboarding_mail_not_now\"").expect("console.js calls onboarding_mail_not_now (Q8, section 14.57.9)");
    assert!(js[at..].contains("{}") || js[at..at + 80].contains("onboarding_mail_not_now\")"), "it takes no argument");
    assert!(js.contains("Not now"), "the button reads `Not now`");
    assert!(js.contains("Finish connecting your email in Settings."), "beside the unfinished-mail line, in the page's own words");
    assert!(js.contains("mail.connect"), "the line is the hand-over's mail.connect item");
    assert!(js.contains("Settings"), "with its Settings button");
}

// ---- F11e1/F11e2: steps 2, 3 and 4 ----------------------------------------------------------------

#[test]
fn every_step_has_its_panel_and_step_nine_is_absent() {
    let c = code(&read("onboarding.js"));
    for n in [2, 3, 4, 5, 6, 7, 8, 10, 11] { assert!(c.contains(&format!("ob-step-{n}")), "missing #ob-step-{n}"); }
    assert!(!c.contains("ob-step-9"), "wave B has no step 9 (not shown, not skipped, not on the rail)");
}

#[test]
fn step_two_again_attaches_the_session_and_shows_the_wrong_account_way_out() {
    let c = code(&read("onboarding.js"));
    let attach = enclosing_fn(&c, c.find("invoke(\"onboarding_attach_session\"").expect("attach"));
    assert!(attach.contains("account_id"), "attached with the sign-in's account_id");
    assert!(attach.contains("wrong_account") && attach.contains("signOut"), "the refusal names the way back to the picker (onboarding_sign_out)");
}

#[test]
fn step_three_captures_only_on_the_second_press_and_shows_a_refusal() {
    let c = code(&read("onboarding.js"));
    let open = enclosing_fn(&c, c.find("invoke(\"open_lms_window\"").expect("open"));
    for cap in ["capture_courses", "onboarding_capture_link", "onboarding_paste_link"] {
        assert!(!open.contains(cap), "R-C1-42: the window-open handler must not call {cap}");
    }
    let cap = enclosing_fn(&c, c.find("invoke(\"capture_courses\"").expect("capture"));
    assert_ne!(open, cap, "the capture is its own handler, the second press");
    assert!(cap.contains(".error") && cap.contains(".ok"), "a refusal on the identity provider's page is shown");
}

#[test]
fn every_exit_from_step_three_closes_the_school_window() {
    let c = code(&read("onboarding.js"));
    let close = function(&c, "closeSchoolWindows");
    assert!(close.contains("close_lms_window"), "R-C1-40");
    for f in ["goNext", "goBack", "finishLater", "signOut"] {
        assert!(function(&c, f).contains("closeSchoolWindows("), "{f} must call closeSchoolWindows()");
    }
}

#[test]
fn step_three_lists_every_course_with_remove_and_takes_counts_from_the_status() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("ob-remove-course"), "each captured course has a Remove control");
    let rm = function(&c, "removeCourse");
    let flat = rm.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("invoke(\"onboarding_answer\"") && (flat.contains("step: 3") || flat.contains("step:3")), "Remove is an onboarding_answer for step 3");
    assert!(flat.contains("remove:") || flat.contains("\"remove\""), "whose answer is {{remove: <key>}} (onboarding_state.rs:484)");
    for field in ["classes", "class_times", "assignments_upcoming", "turned_in", "grades_courses"] {
        assert!(c.contains(&format!("step3.{field}")), "the proof reads step3.{field}");
    }
    assert!(!c.contains("reduce("), "no count is summed on the page");
}

#[test]
fn step_three_has_a_my_school_isnt_listed_field_that_leads_to_the_paste() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("ob-school-unlisted"), "R-OB-4");
    assert!(c.contains("\"paste\""), "and the typed name takes school.path = paste");
    assert!(invoked(&c).contains(&"onboarding_paste_link".to_string()));
}

#[test]
fn step_four_on_a_comp_account_says_nothing_to_pay_and_no_trial_terms() {
    let m = |ent: &str| node(&format!("h.step4Model(\"{ent}\")"));
    let comp = m("comp");
    let text = comp["lines"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_lowercase()).collect::<Vec<_>>().join(" ");
    assert!(text.contains("nothing to pay"), "{text}");
    assert!(!text.contains("no free plan after the trial"), "{text}");
    assert_eq!(comp["checkout"], false, "no checkout button for a comp account");
    let none = m("none");
    let text = none["lines"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect::<Vec<_>>().join(" ");
    assert!(text.contains("no free plan after the trial"), "the terms are shown to a paying student: {text}");
    assert_eq!(none["checkout"], true);
}

// ---- F11b: steps 5, 6, 7 and the skips ----------------------------------------------------------

#[test]
fn a_skip_control_exists_only_where_a_skip_is_allowed_and_each_shows_its_cost() {
    let c = code(&read("onboarding.js"));
    for id in ["ob-skip-5", "ob-skip-6", "ob-skip-7", "ob-skip-8-zybooks", "ob-skip-8-vhl"] {
        assert!(c.contains(id), "step skip #{id} is missing");
    }
    for n in [2, 3, 4, 10] {
        assert!(!c.contains(&format!("ob-skip-{n}")), "step {n} cannot be skipped");
    }
    assert!(!c.contains("\"ob-skip-\" +") && !c.contains("'ob-skip-' +") && !c.contains("ob-skip-${"), "a skip id is never built, so the absence above means something");
    assert!(c.matches("costWords(").count() >= 2, "a skip shows its cost through costWords, not a raw key");
    assert!(invoked(&c).contains(&"onboarding_skip".to_string()));
}

#[test]
fn step_five_shows_the_risk_twelve_line_beside_its_skip() {
    let c = code(&read("onboarding.js"));
    let f = enclosing_fn(&c, c.find("ob-skip-5").expect("skip 5")).replace('\\', "");
    assert!(f.contains("If your school's rules don't allow forwarding your school email, skip this."), "mail risk 12 at step 5's skip");
}

#[test]
fn step_six_shows_the_one_google_testing_sentence_by_reference() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("GOOGLE_TESTING"), "step 6 shows GOOGLE_TESTING beside the consent");
    assert!(!c.contains("While Google reviews Knowlu"), "never a second copy of the sentence");
    assert!(enclosing_fn(&c, c.find("ob-step-6").expect("step 6")).contains("GOOGLE_TESTING"), "in step 6's own render");
    let console = code(&read("console.js"));
    assert_eq!(console.matches("var GOOGLE_TESTING = \"While Google reviews Knowlu").count(), 1, "one definition");
    assert!(console.contains("KNOWLU_COPY") && console.contains("GOOGLE_TESTING: GOOGLE_TESTING"), "console.js shares the constant for onboarding.js to read");
    assert!(c.contains("KNOWLU_COPY"), "onboarding.js reads it from there");
}

#[test]
fn step_six_asks_for_each_scope_through_the_vault_target_commands() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("\"calendar\"") && c.contains("\"gmail\""), "one google_connect per scope (F11b, until O2)");
    assert!(invoked(&c).contains(&"google_status".to_string()), "the grant is polled through google_status");
}

// ---- F11c: step 8, the password fields, credentials ---------------------------------------------

/// Any `type="password"` on the page must be a zyBooks or VHL login field named here by id. This
/// test is never deleted; its allow-list only grows (F11t, section 4.2).
const PASSWORD_FIELD_IDS: [&str; 2] = ["ob-zybooks-password", "ob-vhl-password"];

#[test]
fn no_password_field_on_the_page_but_ours_by_id() {
    for f in ["index.html", "console.js", "onboarding.js"] {
        let text = if f == "index.html" { read(f) } else { code(&read(f)) };
        for (at, _) in text.match_indices("password") {
            // A quoted `password` is a type (`type="password"`, `setAttribute("type", "password")`,
            // `{type: 'password'}`); an id such as ob-vhl-password is preceded by `-`.
            let before = text[..at].trim_end_matches('\\');
            let after = &text[at + "password".len()..];
            let quoted = before.ends_with(['"', '\'']) && after.starts_with(['"', '\'', '\\']);
            let bare = ["type=", "type =", "type:", "type :"].iter().any(|t| before.trim_end().ends_with(t));
            if !(quoted || bare) { continue; }
            let around = &text[at.saturating_sub(200)..(at + 200).min(text.len())];
            assert!(f == "onboarding.js" && PASSWORD_FIELD_IDS.iter().any(|id| around.contains(id)), "{f} has a password field that is not one of ours: {around}");
        }
    }
    let ours = code(&read("onboarding.js"));
    for id in PASSWORD_FIELD_IDS { assert!(ours.contains(id), "the login field #{id} exists"); }
}

#[test]
fn the_login_fields_are_cleared_in_both_arms_the_moment_the_write_returns() {
    let c = code(&read("onboarding.js"));
    let at = c.find("invoke(\"onboarding_store_credentials\"").expect("step 8 stores logins");
    let handler = enclosing_fn(&c, at);
    let after = &handler[handler.find("invoke(\"onboarding_store_credentials\"").unwrap()..];
    let then = after.find(".then(").expect("the answer is read in a then");
    let first_clear = after[then..].find("clearCredentialFields(").expect("the answer's handler clears the fields");
    let first_branch = after[then..].find("if (").unwrap_or(usize::MAX);
    assert!(first_clear < first_branch, "clearCredentialFields runs before anything branches on the answer (ok or refused alike)");
    let rejection = after.find(".catch(").or_else(|| after.find(".finally(")).expect("a rejected call is an arm too");
    assert!(after[rejection..].contains("clearCredentialFields("), "…and in the rejected arm");
    let clear = function(&c, "clearCredentialFields");
    assert!(clear.contains(".value = \"\"") || clear.contains(".value = ''"), "clearing empties the fields");
}

#[test]
fn a_credential_value_is_read_only_inside_the_store_handler_and_goes_nowhere_else() {
    let c = code(&read("onboarding.js"));
    let base = c.as_ptr() as usize;
    let span = |s: &str| (s.as_ptr() as usize - base, s.as_ptr() as usize - base + s.len());
    let handler = span(enclosing_fn(&c, c.find("invoke(\"onboarding_store_credentials\"").unwrap()));
    let clear = span(function(&c, "clearCredentialFields"));
    let mut lookups = 0;
    for id in LOGIN_FIELD_IDS {
        for (at, _) in c.match_indices(id) {
            // A lookup is the id as a whole literal or `"#id"`, not a quoted `id="..."` in markup.
            let (b, a) = (c[..at].chars().last().unwrap_or(' '), c[at + id.len()..].chars().next().unwrap_or(' '));
            let opens = b == '"' || b == '\'' || (b == '#' && matches!(c[..at - 1].chars().last(), Some('"' | '\'')));
            let quote_at = at - if b == '#' { 2 } else { 1 };
            let in_markup = c[..quote_at].trim_end_matches('\\').ends_with('=');
            if !(opens && (a == '"' || a == '\'') && !in_markup) { continue; }
            lookups += 1;
            let inside = |s: (usize, usize)| at >= s.0 && at < s.1;
            assert!(inside(handler) || inside(clear), "login field {id} is looked up outside the store handler and clearCredentialFields: {}", &c[at.saturating_sub(100)..at + 40]);
        }
    }
    assert!(lookups >= 4, "the four login fields are looked up by their literal ids ({lookups} found)");
    for (at, _) in c.match_indices("invoke(\"onboarding_answer\"") {
        assert!(!c[at..(at + 240).min(c.len())].to_lowercase().contains("password"), "no credential in an onboarding_answer payload");
    }
    // Plan ~3032: no credential value is assigned to anything that outlives the handler. Inside the
    // handler, a statement that reads `.value` on its right side assigns only to a local it declares.
    let h = enclosing_fn(&c, c.find("invoke(\"onboarding_store_credentials\"").unwrap());
    for stmt in h.split(';') {
        let Some(eq) = stmt.find('=').filter(|&e| !stmt[e..].starts_with("==") && !stmt[e..].starts_with("=>")) else { continue };
        if !stmt[eq + 1..].contains(".value") { continue; }
        let mut lhs = stmt[..eq].rsplit(['{', '}']).next().unwrap_or("").trim();
        if lhs.starts_with("if (") { lhs = lhs.rsplit(')').next().unwrap_or("").trim(); }
        let name = lhs.trim_start_matches("var ").trim_start_matches("let ").trim_start_matches("const ").trim();
        let declared = lhs.len() != name.len() || h.contains(&format!("var {name}")) || h.contains(&format!("let {name}")) || h.contains(&format!("const {name}"));
        assert!(declared && !name.contains('.') && !name.contains('['), "a login field's value is assigned beyond the handler: {stmt}");
    }
}

/// The four login fields; the first two are also the page's only password inputs.
const LOGIN_FIELD_IDS: [&str; 4] = ["ob-zybooks-username", "ob-zybooks-password", "ob-vhl-username", "ob-vhl-password"];

#[test]
fn discovery_in_flight_disables_next_and_the_sites_skip() {
    let c = code(&read("onboarding.js"));
    let f = enclosing_fn(&c, c.find("invoke(\"onboarding_discover_coursework\"").expect("discover"));
    assert!(f.contains("discovering = true"), "R-C1-55: the flag is raised before the call");
    assert!(f.contains("discovering = false"), "…and lowered when it returns");
    assert!(c.contains("var discovering") && enclosing_fn_or_top(&c, "discovering", "disabled"), "Next and Skip read it into `disabled`");
}

/// True when some line mentions both words (a render line that disables on a flag).
fn enclosing_fn_or_top(c: &str, a: &str, b: &str) -> bool { c.lines().any(|l| l.contains(a) && l.contains(b)) }

#[test]
fn a_blank_row_is_never_ignored_and_no_slug_is_made_on_the_page() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("ob-not-a-class"), "only an explicit `not a class` control ignores a row (R-OB-1)");
    for (at, _) in c.match_indices("zybooks_ignore") {
        assert!(enclosing_fn(&c, at).contains("ob-not-a-class"), "zybooks_ignore is built only from the explicit control");
    }
    for l in c.lines().filter(|l| l.to_lowercase().contains("slug")) {
        assert!(!l.contains(".replace(") && !l.contains("toLowerCase"), "a slug is never derived on the page: {l}");
    }
}

// ---- F11a review r1: entering onboarding, the frame's buttons, the rail's classes -----------------

#[test]
fn launch_state_names_onboarding_and_the_dispatcher_leaves_it_to_the_frame() {
    let rs = fs::read_to_string("src/onboarding.rs").expect("src/onboarding.rs");
    // The function's body alone, comment lines dropped, so a comment can never satisfy the pin (r2 m1).
    let after = rs.split("pub fn launch_state(").nth(1).expect("launch_state");
    let body: String = after[..after.find("\n}\n").expect("launch_state's end")]
        .lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    assert!(body.contains("try_state::<crate::onboarding_state::LaunchMode>") && body.contains("\"onboarding\""), "launch_state answers mode onboarding from the managed LaunchMode");
    let js = code(&read("console.js"));
    let tail = js.split("invoke(\"launch_state\", {}).then(function (l) {").last().expect("the dispatcher");
    let first = tail.trim_start().lines().next().unwrap_or("");
    assert!(first.contains("l.mode === \"onboarding\"") && first.contains("return"), "the dispatcher's first line returns on onboarding: {first}");
    assert!(code(&read("onboarding.css")).find("z-index").is_none(), "no z-index stopgap");
}

#[test]
fn a_not_yet_or_waiting_rail_row_never_wears_the_done_class() {
    let row = |state: &str| node(&format!("h.rowModel(h.rows[1], {{step: 5, jobs: {{\"school.capture\": {{state: \"{state}\"}}}}}})"));
    for state in ["pending", "held"] {
        let r = row(state);
        assert_ne!(r["cls"], "done", "{state}: {r}");
    }
    assert_eq!(row("done")["cls"], "done");
}

#[test]
fn the_frame_re_enables_only_the_buttons_it_disabled_and_a_leave_hook_cannot_block_an_exit() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("data-ob-closed"), "the frame marks the buttons it disables");
    let leaving = function(&c, "leaving");
    assert!(leaving.contains(".catch(") && leaving.contains("3000"), "each hook is caught and the wait is capped");
}

// ---- F11b0: the frame's fixes (plan section 14.53, items 2-4) --------------------------------------

#[test]
fn a_refused_retrys_line_survives_the_polls_and_clears_on_the_next_click() {
    // The frame runs under a stub DOM here, so this reads the code: a poll (`load`) never clears
    // `S.error`; the student's next click and a step change (`OB.show`) do.
    let c = code(&read("onboarding.js"));
    let clears = |body: &str| body.contains("S.error = \"\"");
    // Any assignment to `S.error` (either spacing, either quote), a clearing helper or a bulk assign
    // counts, so the poll can't clear the action line by another spelling (F11b0 review m4).
    let assigns = |body: &str| body.match_indices("S.error").any(|(i, _)| {
        let rest = body[i + "S.error".len()..].trim_start();
        rest.starts_with('=') && !rest.starts_with("==")
    });
    let load = function(&c, "load");
    assert!(!assigns(load) && !load.contains("clearError") && !load.contains("Object.assign(S"), "a status poll must not clear a refused retry's line");
    assert!(clears(function(&c, "onClick")), "the student's next click clears the line");
    let show = c.find("OB.show = function").expect("OB.show");
    assert!(clears(enclosing_fn(&c, show + c[show..].find('{').unwrap() + 1)), "a step change clears the line");
}

#[test]
fn the_rail_row_does_not_read_status_found_which_has_no_producer() {
    let c = code(&read("onboarding.js"));
    assert!(!function(&c, "rowModel").contains("found"), "rowModel reads no `found` until a task names a producer");
}

#[test]
fn a_past_rows_reading_follows_its_job_records() {
    let row = |i: usize, jobs: &str| node(&format!("h.rowModel(h.rows[{i}], {{step: 5, jobs: {jobs}}})"));
    let (account, none) = (row(0, "{}"), row(1, "{}"));
    assert_eq!(account["st"], "Done", "an empty job list reads Done once past: {account}");
    assert_eq!(none["st"], "Not yet", "jobs with no records read Not yet: {none}");
    assert_ne!(none["cls"], "done", "{none}");
    let mixed = row(1, r#"{"school.capture": {"state": "done", "proof": {"classes": 3}}, "school.preview": {"state": "skipped"}}"#);
    assert_eq!((&mixed["st"], &mixed["pf"]), (&serde_json::json!("Done"), &serde_json::json!("3 classes")), "done mixed with skipped reads Done: {mixed}");
    let skipped = row(1, r#"{"school.capture": {"state": "skipped"}, "school.preview": {"state": "skipped"}}"#);
    assert_eq!(skipped["st"], "Skipped", "all skipped reads Skipped: {skipped}");
}

#[test]
fn the_four_mail_and_feed_reasons_have_their_own_words() {
    let w = |k: &str| node(&format!("h.reasonWords(\"{k}\")")).as_str().unwrap().to_string();
    let fallback = w("some_new_reason_key");
    for k in ["mail_setup_cancelled", "mail_setup_failed", "mail_not_yet", "feed_kept_on_device"] {
        let words = w(k);
        assert!(words != fallback && !words.contains('_'), "{k} still answers the generic fallback: {words:?}");
    }
    let waiting = w("mail_not_yet").to_lowercase();
    assert!(!waiting.contains("failed") && !waiting.contains("couldn't"), "mail_not_yet is a wait, not a failure: {waiting:?}");
}

#[test]
fn the_seam_comment_names_both_hooks_of_the_panel_seam() {
    let head: String = read("onboarding.js").lines().take(8).collect::<Vec<_>>().join("\n");
    assert!(head.contains("registerPanel(n, {render, onStatus})"), "the header names {{render, onStatus}}: {head}");
    assert!(head.contains("registerLeave"), "the header names registerLeave");
}

// ---- F11c: steps 8 and 10's rows and the panels' poll rule ----------------------------------------

#[test]
fn steps_eight_and_tens_past_rows_follow_their_job_records() {
    // rows[5] is Homework sites (at step 8), rows[6] is Your week (at step 10); m5's rule, over each.
    let row = |i: usize, step: u8, jobs: &str| node(&format!("h.rowModel(h.rows[{i}], {{step: {step}, jobs: {jobs}}})"));
    let none = row(5, 10, "{}");
    assert_eq!((&none["st"], &none["pf"]), (&serde_json::json!("Not yet"), &serde_json::json!("Not yet")), "a past homework row with no job records: {none}");
    assert_ne!(none["cls"], "done", "{none}");
    let none = row(6, 11, "{}");
    assert_eq!(none["st"], "Not yet", "a past week row with no job records: {none}");
    assert_ne!(none["cls"], "done", "{none}");
    let done = row(5, 10, r#"{"coursework.discover": {"state": "done", "proof": {"books_or_sections": 2, "assignments": 5}}}"#);
    assert_eq!((&done["st"], &done["pf"]), (&serde_json::json!("Done"), &serde_json::json!("2 books or sections, 5 assignments")), "{done}");
    let mixed = row(6, 11, r#"{"config.ingest": {"state": "done", "proof": {"events_this_week": 6}}, "slot.partial": {"state": "skipped"}}"#);
    assert_eq!((&mixed["st"], &mixed["pf"]), (&serde_json::json!("Done"), &serde_json::json!("6 events this week")), "done mixed with skipped reads Done: {mixed}");
    let skipped = row(6, 11, r#"{"config.ingest": {"state": "skipped"}, "slot.partial": {"state": "skipped"}}"#);
    assert_eq!(skipped["st"], "Skipped", "all skipped reads Skipped: {skipped}");
    let site_skip = row(5, 10, r#"{"coursework.discover": {"state": "skipped"}}"#);
    assert_eq!(site_skip["st"], "Skipped", "{site_skip}");
}

#[test]
fn the_step_eight_and_ten_panels_never_redraw_the_pane_from_a_poll() {
    let c = code(&read("onboarding.js"));
    let flat = c.split_whitespace().collect::<Vec<_>>().join(" ");
    for (n, name) in [(8, "onStatus8"), (10, "onStatus10")] {
        assert!(flat.contains(&format!("OB.registerPanel({n}, {{ render: render{n}, onStatus: {name} }})")), "step {n} registers through the seam");
        let body = function(&c, name);
        assert!(!body.contains("refresh") && !body.contains(".show("), "{name} must not redraw the pane (a poll wipes #ob-ask)");
    }
    assert!(flat.contains("OB.registerLeave("), "step 8 clears its login fields when the student leaves");
}

#[test]
fn step_eights_proceed_is_one_answer_sent_after_the_skips_and_every_answer_clears_the_action_line() {
    let c = code(&read("onboarding.js"));
    let leave = function(&c, "leaveStep8");
    let flat = leave.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("invoke(\"onboarding_answer\", { step: 8, answer: { proceed: true } })"), "step 8's proceed: {flat}");
    for f in ["storeLogins", "answerRow", "skipSite", "leaveStep8", "confirm10"] {
        assert!(function(&c, f).contains("OB.clearError()"), "{f} sends an answer, so it clears the action line first");
    }
    let skip = function(&c, "skipSite");
    assert!(skip.contains("onboarding_skip") && skip.contains("leaveStep8()"), "the last skip leaves the step, after the skip call returns");
}

// ---- F11c review r1: step 10's Edit (I-1), the firmness default (M5), the poll pin's reach (M6) ------

#[test]
fn step_tens_class_time_rows_have_an_edit_that_saves_through_the_consoles_set_fields() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("data-act10=\"edit\""), "each class-time row has an Edit control");
    let at = c.find("invoke(\"set_fields\"").expect("Edit saves through the console's set_fields");
    let save = enclosing_fn(&c, at);
    assert!(save.contains("OB.clearError()") && save.contains("fields") && save.contains(".error"), "the save clears the line and shows a refusal");
    assert!(save.contains("view: \"today\""), "the console's view for a write from setup is today's");
}

#[test]
fn a_proposal_with_no_level_defaults_the_firmness_to_soft_as_the_confirm_does() {
    let c = code(&read("onboarding.js"));
    assert!(function(&c, "propHtml").contains("(p.level || \"soft\")"), "the select's default matches confirm10's fallback");
}

#[test]
fn the_poll_pin_follows_the_functions_a_panels_onstatus_calls() {
    let c = code(&read("onboarding.js"));
    for name in ["onStatus10", "askProposals", "setConfirm10", "loadSet10"] {
        let body = function(&c, name);
        assert!(!body.contains("refresh") && !body.contains(".show("), "{name} runs from a poll and must not redraw the pane");
    }
}

#[test]
fn step_eights_empty_field_message_and_the_other_sites_sign_in_do_not_redraw_or_stay_live() {
    let c = code(&read("onboarding.js"));
    let flat = function(&c, "paintNext").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("signin"), "paintNext holds every Sign in while a discovery runs: {flat}");
    let store = function(&c, "storeLogins");
    let empty = &store[store.find("!user || !secret").expect("the empty check")..];
    let branch = &empty[..empty.find('}').unwrap()];
    assert!(!branch.contains("paintSite("), "an empty-field message must not redraw the form and wipe the typed username");
}

#[test]
fn an_edit_menu_keeps_a_kind_it_has_no_label_for_selected_and_sends_only_what_changed() {
    let c = code(&read("onboarding.js"));
    let row = function(&c, "editRowHtml");
    assert!(row.contains("indexOf") || row.contains(".some("), "a kind outside the label list is added as the selected option, in the engine's own word");
    let save = function(&c, "saveEdit10");
    assert!(save.contains("!== row.getAttribute(\"data-kind\")") && save.contains("!== row.getAttribute(\"data-level\")"), "each field is sent only when its control differs from the row's own value");
}

// ---- F11p1 (section 14.58.4): the look, the rail's lines and its foot -------------------------------
// Conventions fixed here for `console-ui`: `rowModel`'s input `s` carries `account` (the answer of
// `account_status`, or null/absent when the call failed); `h.minutesLeft(step)` is the foot's N;
// the one account loader is `loadAccount`.

fn pf(row: usize, account: &str) -> String {
    let v = node(&format!("h.rowModel(h.rows[{row}], {{step: 5, jobs: {{}}, account: {account}}})"));
    v["pf"].as_str().unwrap_or_default().to_string()
}

#[test]
fn the_rails_account_and_payment_lines_come_from_account_status() {
    let acct = |st: &str, end: &str| format!("{{ok: true, email: \"ada@example.test\", status: {st}, current_period_end: {end}}}");
    assert_eq!(pf(0, &acct("\"trialing\"", "\"2026-11-12T12:00:00Z\"")), "Signed in as ada@example.test");
    assert_eq!(pf(2, &acct("\"trialing\"", "\"2026-11-12T12:00:00Z\"")), "Trial on. First charge November 12.");
    assert_eq!(pf(2, &acct("\"active\"", "\"2026-12-03T12:00:00Z\"")), "Paid. Renews December 3.");
    // Review r1 I-1: the engine's only no-charge case is `active` with plan `tester`; `none` and unknown words show nothing.
    assert_eq!(pf(2, "{ok: true, email: \"ada@example.test\", status: \"active\", plan: \"tester\", current_period_end: null}"), "Nothing to pay.");
    assert_eq!(pf(2, &acct("\"none\"", "null")), "");
}

#[test]
fn the_payment_row_has_no_detail_line_without_a_status_an_end_date_or_an_answer() {
    // r1 m-4: a null entitlement, a trial with no end date and a failed call each show the state word only.
    for account in ["{ok: true, email: \"ada@example.test\", status: null, current_period_end: null}",
                    "{ok: true, email: \"ada@example.test\", status: \"trialing\", current_period_end: null}",
                    "{ok: true, email: \"ada@example.test\", status: \"active\", current_period_end: null}",
                    "null", "{ok: false, error: \"x\"}"] {
        assert_eq!(pf(2, account), "", "no payment detail line for {account}");
    }
}

#[test]
fn account_status_is_read_on_mount_and_at_two_step_moves_never_on_the_poll() {
    let c = code(&read("onboarding.js"));
    assert_eq!(invoked(&c).iter().filter(|n| *n == "account_status").count(), 1, "one call site, in one loader");
    assert!(function(&c, "loadAccount").contains("account_status"));
    for name in ["poll", "load"] {
        assert!(!function(&c, name).contains("loadAccount") && !function(&c, name).contains("account_status"), "{name} runs on the poll");
    }
    assert!(c.matches("loadAccount(").count() >= 4, "a definition and the mount, the step 2 move and the step 4 move");
}

#[test]
fn the_rail_head_is_one_labelled_logo_and_the_foot_holds_the_minutes_and_finish_later() {
    let c = code(&read("onboarding.js"));
    let frame = function(&c, "frameHtml");
    assert!(frame.contains("aria-label=\"Knowlu\"") && frame.contains("su-logo") && !frame.contains("su-mark"), "the old square is gone");
    let foot = frame.find("su-rail-foot").expect("the rail has a foot");
    assert!(frame.find("ob-finish-later").is_some_and(|at| at > foot), "Finish later moved to the foot");
    assert!(frame.find("ob-sign-out").is_some_and(|at| at < foot), "Sign out stays in the top bar");
    assert!(c.contains("About ") && c.contains(" minutes left"));
    let m: Vec<i64> = [2, 5, 7, 8, 11].iter().map(|n| node(&format!("h.minutesLeft({n})")).as_i64().expect("a whole number")).collect();
    assert!(m.windows(2).all(|w| w[0] >= w[1]) && m[2] > m[3] && m[4] == 1, "never up, step 7 counts until it is past, at least 1: {m:?}");
}

#[test]
fn the_fixture_path_answers_the_four_commands_and_the_page_follows_windows_light_setting() {
    let c = code(&read("onboarding.js"));
    let inv = function(&c, "invoke");
    for cmd in ["onboarding_status", "account_status", "your_week", "commitment_proposals"] { assert!(inv.contains(cmd), "?fixture= answers {cmd}"); }
    assert!(c.contains("prefers-color-scheme") && c.contains("data-theme"), "the theme starts from Windows' setting");
    assert!(!c.contains("setItem"), "onboarding stores no theme choice");
}

// ---- F11p1s: the shell's minutes line and Finish later (plan §14.59, Quinn's Q-B25) ---------------

/// Evaluates `expr` under node with `SU` bound to `su_json`. For the one expression the page's
/// `renderSetup` assigns to the button's `disabled`; nothing else runs.
fn eval_with_su(expr: &str, su_json: &str) -> bool {
    let out = Command::new("node").arg("-e").arg("const SU = JSON.parse(process.env.SU_JSON); console.log(JSON.stringify(Boolean(eval(process.env.SU_EXPR))));")
        .env("SU_JSON", su_json).env("SU_EXPR", expr).output().expect("node must be installed");
    assert!(out.status.success(), "node failed on `{expr}`: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("a boolean")
}

#[test]
fn the_shells_screens_count_eleven_ten_and_nine_minutes_left() {
    // Welcome asks minutesLeft(1), account/browser/code ask (2), the done screen asks (3): 615, 585
    // and 540 seconds, rounded up (step 1 is 30 s, step 2 is 45 s, and step 9 is absent).
    let got: Vec<i64> = [1, 2, 3].iter().map(|n| node(&format!("h.minutesLeft({n})")).as_i64().expect("a whole number")).collect();
    assert_eq!(got, vec![11, 10, 9], "minutesLeft(1), (2), (3)");
    assert_eq!(node("h.minutesLeft(4)").as_i64(), Some(7), "the count from step 4 on does not move (420 s, unchanged by F11p1s)");
}

#[test]
fn the_shells_foot_holds_the_minutes_line_then_finish_later() {
    let html = read("index.html");
    let at = html.find("class=\"su-rail-foot\"").expect("the shell's rail has a foot");
    let foot = &html[at..at + html[at..].find("</div>").expect("the foot closes")];
    let left = foot.find("id=\"su-left\"").expect("the foot holds su-left");
    let button = foot.find("id=\"su-finish-later\"").expect("the foot holds su-finish-later");
    assert!(left < button, "the button sits after the minutes line");
    assert!(foot.contains("<button") && foot.contains("type=\"button\"") && foot.contains("Finish later</button>"), "a plain button reading Finish later: {foot}");
    assert!(foot.contains("su-link"), "it wears su-link, as the setup frame's does");
}

/// The click handler's text: the delegated `closest("#su-finish-later")` branch of `console.js`, plus
/// the body of any `function` that branch calls.
fn finish_later_handler(c: &str) -> String {
    let at = c.find("closest(\"#su-finish-later\")").expect("console.js handles a click on #su-finish-later");
    let line = c[at..].split('\n').next().unwrap_or("").to_string();
    let mut text = line.clone();
    for part in line.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')) {
        if !part.is_empty() && c.contains(&format!("function {part}(")) { text.push_str(function(c, part)); }
    }
    text
}

#[test]
fn the_shells_finish_later_calls_setup_quit_and_no_onboarding_exit() {
    let c = code(&read("console.js"));
    let handler = finish_later_handler(&c);
    assert_eq!(invoked(&handler), vec!["setup_quit".to_string()], "one command: {handler}");
    for banned in ["onboarding_quit", "onboarding_sign_out", "onboarding_finish", "sign_out", "open_profile", "finish_onboarding"] {
        assert!(!handler.contains(banned), "the shell's Finish later never calls {banned}");
    }
}

/// The `disabled` expression `renderSetup` gives the button.
fn finish_later_disabled(c: &str) -> String {
    let render = function(c, "renderSetup");
    let key = "EL(\"su-finish-later\").disabled =";
    let at = render.find(key).expect("renderSetup sets the button's disabled");
    let rest = &render[at + key.len()..];
    rest[..rest.find(';').expect("a statement")].trim().to_string()
}

#[test]
fn the_shells_finish_later_is_disabled_while_making_and_while_continue_relaunches() {
    let expr = finish_later_disabled(&code(&read("console.js")));
    assert!(eval_with_su(&expr, r#"{"step":"done","making":true,"busy":false}"#), "disabled while the folder is being made: {expr}");
    assert!(eval_with_su(&expr, r#"{"step":"done","making":false,"busy":true}"#), "disabled while Continue relaunches: {expr}");
    assert!(!eval_with_su(&expr, r#"{"step":"done","making":false,"busy":false}"#), "enabled on a settled done screen: {expr}");
    assert!(!eval_with_su(&expr, r#"{"step":"welcome","making":false,"busy":false}"#), "enabled on welcome: {expr}");
}

#[test]
fn the_shells_finish_later_stays_enabled_while_a_sign_in_or_a_code_send_is_in_flight() {
    let expr = finish_later_disabled(&code(&read("console.js")));
    for step in ["welcome", "account", "browser", "code"] {
        let su = format!(r#"{{"step":"{step}","making":false,"busy":true}}"#);
        assert!(!eval_with_su(&expr, &su), "enabled on {step} with SU.busy set (never tied to SU.busy alone): {expr}");
    }
}

#[test]
fn the_rail_foots_styles_are_widened_to_the_shell_and_not_copied() {
    let css = code(&read("onboarding.css"));
    // Rules found by selector, not by counting declarations anywhere in the file.
    let rules: Vec<(Vec<&str>, &str)> = css
        .split('}')
        .filter_map(|chunk| {
            let (sel, body) = chunk.split_once('{')?;
            Some((sel.split(',').map(str::trim).collect(), body))
        })
        .collect();
    for (tail, size) in [(".su-rail-foot .su-link", "font-size: 14.5px"), (".su-rail-foot .su-dim", "font-size: 13.5px")] {
        let hits: Vec<_> = rules.iter().filter(|(list, _)| list.iter().any(|s| s.ends_with(tail))).collect();
        assert_eq!(hits.len(), 1, "{tail} is styled in exactly one rule, which both feet share");
        let (list, body) = hits[0];
        assert!(list.iter().any(|s| s.starts_with(".ob ") && s.ends_with(tail)), "it still serves the setup frame: {list:?}");
        assert!(list.iter().any(|s| (s.starts_with(".setup ") || s.starts_with("#setup ")) && s.ends_with(tail)), "and the shell's foot: {list:?}");
        assert!(body.contains(size), "{tail} carries {size}: {body}");
    }
}

/// Quinn's ruling (2026-10-24): the welcome copy carries no number. The rail foot, from
/// `STEP_SECONDS`, gives the one figure.
#[test]
fn the_welcome_copy_carries_no_minutes_figure() {
    let html = read("index.html");
    let at = html.find("class=\"su-why\"").expect("the su-why element");
    let rest = &html[at..];
    let text = &rest[..rest.find("</p>").expect("its end")].to_lowercase();
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    let spelled = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "fifteen", "twenty", "thirty", "half", "couple", "few"];
    for pair in words.windows(2) {
        if pair[1].starts_with("minute") {
            assert!(!pair[0].chars().any(|c| c.is_ascii_digit()), "a digit before minute: {text}");
            assert!(!spelled.contains(&pair[0]), "a spelled number before minute: {text}");
        }
    }
    assert!(!text.contains("minute"), "no minutes figure in the welcome copy: {text}");
}

// ---- F11c1: step 8's console wrappers (no vault) and step 10's confirmed answer (section 14.57.1-.2) ----

/// The text of every `invoke("name", ...)` call to `name` in `body`, parentheses balanced.
fn invoke_calls<'a>(body: &'a str, name: &str) -> Vec<&'a str> {
    let sk = skeleton(body);
    let mut out = Vec::new();
    for (at, _) in body.match_indices(&format!("invoke(\"{name}\"")) {
        let mut depth = 0i32;
        for (k, b) in sk[at..].bytes().enumerate() {
            if b == b'(' { depth += 1; }
            if b == b')' { depth -= 1; if depth == 0 { out.push(&body[at..at + k + 1]); break; } }
        }
    }
    out
}

#[test]
fn step_eight_calls_the_console_wrappers_and_never_the_path_taking_commands() {
    let names = invoked(&code(&read("onboarding.js")));
    for want in ["onboarding_discover_coursework", "onboarding_store_credentials"] {
        assert!(names.iter().any(|n| n == want), "step 8 must call {want}");
    }
    for old in ["discover_coursework", "store_credentials"] {
        assert!(!names.iter().any(|n| n == old), "onboarding.js must not call the path-taking {old}");
    }
}

#[test]
fn no_step_eight_call_passes_a_vault() {
    let c = code(&read("onboarding.js"));
    for (f, cmd) in [("storeLogins", "onboarding_store_credentials"), ("discoverSite", "onboarding_discover_coursework")] {
        let body = function(&c, f);
        let calls = invoke_calls(body, cmd);
        assert_eq!(calls.len(), 1, "{f} must call {cmd} exactly once");
        assert!(!calls[0].contains("vault"), "{f} sends no vault, the console reads it from its own state: {}", calls[0]);
        // F11c1 review I-1: onboarding_status never carries a vault (section 14.57.1), so no guard may read one.
        assert!(!body.contains("vault"), "{f} reads no vault at all");
        let others = body.matches("invoke(").count() - 1;
        assert_eq!(others, 0, "{f} makes no other call");
    }
}

#[test]
fn step_tens_confirmed_answer_follows_a_successful_confirm_and_precedes_the_next_screen() {
    let c = code(&read("onboarding.js"));
    let flat = function(&c, "confirm10").split_whitespace().collect::<Vec<_>>().join(" ");
    let answer = "invoke(\"onboarding_answer\", { step: 10, answer: { confirmed: true } })";
    assert_eq!(flat.matches(answer).count(), 1, "step 10 sends (10, {{confirmed: true}}) once: {flat}");
    let confirm = flat.find("invoke(\"commitments_confirm\"").expect("confirm10 calls commitments_confirm");
    let guard = flat[confirm..].find("env.ok !== true").map(|i| confirm + i).expect("the refusal guard");
    let guard_end = guard + flat[guard..].find("return;").expect("the guard returns");
    let sent = flat.find(answer).unwrap();
    let show = flat.find("OB.show(11)").expect("confirm10 shows step 11");
    assert!(confirm < guard_end && guard_end < sent, "the answer is sent only after the confirm succeeded (past the refusal's return)");
    assert!(sent < show, "the answer is sent before OB.show(11)");
    let catch = flat.find(".catch(").expect("a catch arm");
    assert!(sent < catch, "the answer is not sent from the failure arm");
}

// ---- F11p2: the bottom week strip and step 8's one-site form (plan section 14.58.5) --------------
//
// Helpers these pins fix, for F11p2 to add to `window.KNOWLU_ONBOARDING` (all pure, none sums):
// - `weekStripModel(status, step)`: `null` when the strip is hidden, else
//   `{label, days: [{name, past, count}] (Monday first, seven), later}`. `count` is `step3.due_*`
//   as given; a past day (index below `week.today`) has `past: true` and `count: null`.
// - `weekStripHtml(status, step)`: `""` when hidden, else the strip's markup (past days wear `ob-past`).
// - `siteOrder(status)`: the sites in the order step 8 shows them.
// - `foundLine(status, site)`: the "Found at your school" sentence, `""` when there is none.

fn helper(name: &str) {
    assert_eq!(node(&format!("typeof h.{name}")), serde_json::json!("function"), "onboarding.js must expose KNOWLU_ONBOARDING.{name} (F11p2)");
}

/// A status with invented counts: Monday 2, Tuesday 0, Wednesday 3, Thursday 1, Friday 4, Saturday 0,
/// Sunday 5, later 9; the week of Monday 2026-10-19, today Wednesday (index 2).
const STATUS_WEEK: &str = r#"{step: 5, week: {of: "2026-10-19", today: 2}, homework_sites: null,
  step3: {classes: 2, due_mon: 2, due_tue: 0, due_wed: 3, due_thu: 1, due_fri: 4, due_sat: 0, due_sun: 5, due_later: 9}}"#;

fn strip_text(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch { '<' => { in_tag = true; out.push(' '); } '>' => in_tag = false, c if !in_tag => out.push(c), _ => {} }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_week_strip_reads_your_week_the_seven_days_and_later_from_the_step3_fields() {
    helper("weekStripModel");
    let m = node(&format!("h.weekStripModel({STATUS_WEEK}, 5)"));
    assert!(m.is_object(), "the strip shows at step 5: {m}");
    let days = m["days"].as_array().expect("seven days");
    assert_eq!(days.len(), 7);
    for (d, want) in days.iter().zip(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]) {
        assert!(d["name"].as_str().unwrap_or_default().starts_with(want), "Monday first: {d}");
    }
    let counts: Vec<_> = days.iter().map(|d| d["count"].clone()).collect();
    assert_eq!(counts[2..], [3, 1, 4, 0, 5].map(|n| serde_json::json!(n)), "today onward carry their own fields (a 0 stays 0)");
    assert_eq!(m["later"], serde_json::json!(9));
    assert!(m["label"].as_str().is_some_and(|l| l.contains("Week of October 19")), "label: {}", m["label"]);
}

#[test]
fn past_days_are_dimmed_with_no_count_and_today_is_not() {
    helper("weekStripModel");
    helper("weekStripHtml");
    let m = node(&format!("h.weekStripModel({STATUS_WEEK}, 5)"));
    let flags: Vec<_> = m["days"].as_array().expect("days").iter().map(|d| (d["past"].as_bool(), d["count"].is_null())).collect();
    assert_eq!(flags, vec![(Some(true), true), (Some(true), true), (Some(false), false), (Some(false), false), (Some(false), false), (Some(false), false), (Some(false), false)]);
    let html = node(&format!("h.weekStripHtml({STATUS_WEEK}, 5)")).as_str().unwrap_or_default().to_string();
    assert_eq!(html.matches("ob-past").count(), 2, "two past days wear ob-past: {html}");
    let text = strip_text(&html);
    for want in ["Your week", "Week of October 19", "Later", "Mon", "Sun"] { assert!(text.contains(want), "{want} in: {text}"); }
    // Counts only: today's and later days' figures, then Later's; Monday's 2 (a past day) is not shown.
    let digits: Vec<&str> = text.split_whitespace().filter(|w| w.chars().all(|c| c.is_ascii_digit())).collect();
    assert_eq!(digits, vec!["19", "3", "1", "4", "0", "5", "9"], "the label's 19, then the five live days, then Later: {text}");
}

#[test]
fn the_week_label_follows_week_of_without_date_arithmetic_across_a_month() {
    helper("weekStripModel");
    let s = r#"{week: {of: "2026-11-02", today: 0}, step3: {due_mon: 1, due_tue: 1, due_wed: 1, due_thu: 1, due_fri: 1, due_sat: 1, due_sun: 1, due_later: 1}}"#;
    let m = node(&format!("h.weekStripModel({s}, 6)"));
    assert!(m["label"].as_str().is_some_and(|l| l.contains("Week of November 2")), "label: {}", m["label"]);
    assert!(m["days"].as_array().expect("days").iter().all(|d| d["past"] == serde_json::json!(false)), "Monday is today, nothing is past");
}

#[test]
fn the_strip_shows_from_step_three_and_on_five_to_eight_and_hides_at_two_four_ten_and_eleven() {
    helper("weekStripModel");
    helper("weekStripHtml");
    for step in [3, 5, 6, 7, 8] {
        assert!(node(&format!("h.weekStripModel({STATUS_WEEK}, {step})")).is_object(), "shown at step {step}");
    }
    for step in [2, 4, 10, 11] {
        assert!(node(&format!("h.weekStripModel({STATUS_WEEK}, {step})")).is_null(), "hidden at step {step}");
        assert_eq!(node(&format!("h.weekStripHtml({STATUS_WEEK}, {step})")), serde_json::json!(""), "no markup at step {step}");
    }
}

#[test]
fn the_strip_is_hidden_without_week_without_the_due_fields_or_with_all_of_them_null() {
    helper("weekStripModel");
    let nulls = r#"{due_mon: null, due_tue: null, due_wed: null, due_thu: null, due_fri: null, due_sat: null, due_sun: null, due_later: null}"#;
    let counts = r#"{due_mon: 1, due_tue: 1, due_wed: 1, due_thu: 1, due_fri: 1, due_sat: 1, due_sun: 1, due_later: 1}"#;
    for (name, status) in [
        ("week is null", format!("{{week: null, step3: {counts}}}")),
        ("no week key", format!("{{step3: {counts}}}")),
        ("step3 has no due fields", r#"{week: {of: "2026-10-19", today: 2}, step3: {classes: 2}}"#.to_string()),
        ("every due field is null", format!("{{week: {{of: \"2026-10-19\", today: 2}}, step3: {nulls}}}")),
        ("step3 is null", r#"{week: {of: "2026-10-19", today: 2}, step3: null}"#.to_string()),
        ("no status yet", "null".to_string()),
    ] {
        assert!(node(&format!("h.weekStripModel({status}, 5)")).is_null(), "hidden when {name}");
    }
}

#[test]
fn the_page_sums_nothing_for_the_strip_and_the_strip_is_pinned_to_the_pane_bottom() {
    helper("weekStripModel");
    let c = code(&read("onboarding.js"));
    let model = function(&c, "weekStripModel");
    assert!(!model.contains("reduce(") && !model.contains("+=") && !model.contains("+ s.") && !model.contains("+ due"), "each figure is a field, never a sum: {model}");
    let css = code(&read("onboarding.css"));
    assert!(css.contains("ob-past"), "past days have a dimmed style");
    assert!(css.contains("ob-week") && (css.contains("sticky") || css.contains("position: absolute") || css.contains("position: fixed")), "the strip is pinned to the bottom of the step pane");
}

fn sites(js_map: &str) -> String { format!("{{homework_sites: {js_map}}}") }

#[test]
fn step_eight_shows_sites_with_evidence_first_then_zybooks_before_vhl() {
    helper("siteOrder");
    let order = |m: &str| node(&format!("h.siteOrder({})", sites(m)));
    assert_eq!(order("null"), serde_json::json!(["zybooks", "vhl"]), "no evidence: zyBooks first");
    assert_eq!(order(r#"{vhl: ["BIO 101"]}"#), serde_json::json!(["vhl", "zybooks"]), "evidence first");
    assert_eq!(order(r#"{vhl: ["BIO 101"], zybooks: ["CS 201"]}"#), serde_json::json!(["zybooks", "vhl"]), "both: zyBooks first");
    assert_eq!(order(r#"{vhl: ["unmatched"]}"#), serde_json::json!(["vhl", "zybooks"]), "unmatched is still evidence of the site");
    assert_eq!(node("h.siteOrder(null)"), serde_json::json!(["zybooks", "vhl"]), "no status");
}

#[test]
fn the_found_line_names_the_codes_in_the_order_given_and_only_for_a_site_with_an_entry() {
    helper("foundLine");
    let line = |m: &str, site: &str| node(&format!("h.foundLine({}, \"{site}\")", sites(m))).as_str().unwrap_or("?").to_string();
    let l = line(r#"{zybooks: ["MAT 150", "CS 201"]}"#, "zybooks");
    assert!(l.starts_with("Found at your school: assignments in ") && l.ends_with(" link to zyBooks."), "{l}");
    let (a, b) = (l.find("MAT 150").expect("first code"), l.find("CS 201").expect("second code"));
    assert!(a < b, "codes keep the order the status gave them (LMS-key order), not alphabetical: {l}");
    assert_eq!(line(r#"{zybooks: ["CS 201"]}"#, "vhl"), "", "a site without an entry has no Found line");
    assert_eq!(line("null", "zybooks"), "", "null homework_sites: no Found line at all");
    assert_eq!(line(r#"{vhl: ["BIO 101"]}"#, "zybooks"), "");
    assert!(line(r#"{vhl: ["BIO 101"]}"#, "vhl").ends_with(" link to VHL."));
    assert_eq!(node("h.foundLine(null, \"vhl\")"), serde_json::json!(""), "no status yet");
}

#[test]
fn the_word_unmatched_never_reaches_the_page_and_an_unmatched_only_site_gets_the_plain_sentence() {
    helper("foundLine");
    let line = |m: &str| node(&format!("h.foundLine({}, \"vhl\")", sites(m))).as_str().unwrap_or("?").to_string();
    assert_eq!(line(r#"{vhl: ["unmatched"]}"#), "Found at your school: some assignments at your school link to VHL.");
    let both = line(r#"{vhl: ["BIO 101", "unmatched"]}"#);
    assert!(both.contains("BIO 101") && !both.to_lowercase().contains("unmatched"), "a code and unmatched: {both}");
    assert!(both.starts_with("Found at your school: assignments in BIO 101 link to VHL"), "{both}");
    let first = line(r#"{vhl: ["unmatched", "BIO 101"]}"#);
    assert!(first.contains("BIO 101") && !first.to_lowercase().contains("unmatched"), "unmatched first: {first}");
}

#[test]
fn one_site_form_shows_at_a_time_and_every_site_stays_reachable_from_a_show_link() {
    helper("siteOrder");
    let c = code(&read("onboarding.js"));
    let render = function(&c, "render8");
    assert!(render.contains("siteOrder("), "render8 orders the sites by siteOrder: {render}");
    let flat = c.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Show "), "every site stays reachable from a Show {{site}} link");
    assert!(c.contains("data-act8=\"show\""), "the link is a step 8 action");
    assert!(function(&c, "onClick8").contains("\"show\""), "onClick8 handles it");
}

#[test]
fn step_eight_sits_in_knowlus_own_frame_with_the_stored_only_here_line() {
    let c = code(&read("onboarding.js")).replace("\\'", "'").replace("&#39;", "'");
    assert!(c.contains("Knowlu's form"), "the frame is labelled Knowlu's form");
    let login = function(&c, "loginHtml");
    assert!(login.contains("Stored only on this computer, in Windows Credential Manager, and used only to read "), "the privacy line stays: {login}");
    assert!(c.contains("foundLine("), "the Found line comes from foundLine");
    let css = code(&read("onboarding.css"));
    assert!(css.contains("ob-form"), "the frame has a style");
}

#[test]
fn the_found_line_joins_codes_with_and_and_no_serial_comma() {
    helper("foundLine");
    let line = |m: &str| node(&format!("h.foundLine({}, \"zybooks\")", sites(m))).as_str().unwrap_or("?").to_string();
    assert_eq!(line(r#"{zybooks: ["CS 201"]}"#), "Found at your school: assignments in CS 201 link to zyBooks.");
    assert_eq!(line(r#"{zybooks: ["CS 201", "BIO 101"]}"#), "Found at your school: assignments in CS 201 and BIO 101 link to zyBooks.");
    assert_eq!(line(r#"{zybooks: ["CS 201", "BIO 101", "MAT 150"]}"#), "Found at your school: assignments in CS 201, BIO 101 and MAT 150 link to zyBooks.");
}

#[test]
fn focus_follows_show_and_advance_onto_the_current_sites_heading() {
    let c = code(&read("onboarding.js"));
    let site = function(&c, "siteHtml");
    assert_eq!(site.matches("<h2 class=\"ob-h2\" tabindex=\"-1\">").count(), 5, "every stage's site heading takes focus: {site}");
    assert!(!site.contains("<h2 class=\"ob-h2\">"), "no site heading without tabindex");
    assert!(function(&c, "focusSite8").contains(".focus()"), "the heading is focused");
    assert!(function(&c, "advance8").contains("focusSite8()"), "Skip, Done and advance focus it");
    assert!(function(&c, "onClick8").contains("focusSite8()"), "Show focuses it");
    assert!(!function(&c, "paintView8").contains("focus"), "a repaint alone moves no focus");
}

#[test]
fn the_week_strip_is_a_named_region_and_the_login_labels_name_their_inputs() {
    let c = code(&read("onboarding.js"));
    assert!(c.contains("role=\"region\" aria-label=\"Your week\""), "the strip is a named region");
    let login = function(&c, "loginHtml");
    for s in ["username", "password"] {
        assert!(login.contains(&format!("for=\"ob-' + site + '-{s}\"")), "label for the {s} input: {login}");
    }
}

#[test]
fn the_form_frame_closes_after_the_stored_only_line() {
    let c = code(&read("onboarding.js"));
    let login = function(&c, "loginHtml");
    let (privacy, close, skip, btn) = (login.find("Stored only on this computer").unwrap(), login.find("</p></div>").expect("frame closes"), login.find("If you skip").unwrap(), login.find("su-actions").unwrap());
    assert!(privacy < close && close < skip && skip < btn, "frame ends after the privacy line, before the consequence and the buttons");
    assert!(!function(&c, "siteHtml").contains("loginHtml(site) + '</div>'"), "siteHtml adds no closing div after the buttons");
}

// ---- F11p3: step 10 as `o9-your-week` (plan section 14.58.6) ------------------------------------
//
// Helpers these pins fix, for F11p3 to add to `window.KNOWLU_ONBOARDING` (all pure, none sums):
// - `summaryLine10(status)`: "{classes} classes and {class_times} class times, set from your school's
//   registrar. Nothing to type." from `step3.classes` and `step3.class_times`; `""` when either is
//   missing or null (a count of 0 is a count).
// - `splitProposals10(reply)`: `{events: [...], window: p | null}` from a `commitment_proposals`
//   reply; `events` are the proposals with no `window: true`, `window` the one that has it.
// - `firmnessHtml10(level)`: the three-way segmented control's markup (a `role="group"` of three
//   buttons, Must keep / Usually / Optional, each with `data-level`, `aria-pressed="true"` on
//   `level`, "soft" when none).
// - `windowFields10(proposal)`: `{start, end}` preselected from the window proposal's first meet,
//   both `""` when there is no proposal.
// - `windowArg10(proposal, win)`: the `window` string for `commitments_confirm`, or `null`. `win` is
//   `{answered, start, end}`. Not answered, or a blank time, is `null`. Times equal to the proposal's
//   own send `JSON.stringify(proposal.meets)` unchanged; otherwise `JSON.stringify([{days, start,
//   end}])` over the proposal's days (all seven, "mon".."sun", with no proposal).
// - `confirmArg10(rows, proposal, win)`: the `confirm` object. `rows` are `{key, answer, level}`;
//   `{mine: [{source_uid, level}], not_mine: [key]}`, plus `window` only when `windowArg10` is not null.
// - `answeredCount10(rows, win)`: `{n, m}`; `m` is the rows plus the one window question, `n` the
//   rows answered "mine" or "not" plus the window if `win.answered` (the student's own presses).
// - `confirmBarLine10(n, m)`: "{n} of {m} answered. Anything you leave blank comes back as a
//   question over the next few days."

fn helper10(name: &str) {
    assert_eq!(node(&format!("typeof h.{name}")), serde_json::json!("function"), "onboarding.js must expose KNOWLU_ONBOARDING.{name} (F11p3)");
}

const WIN_PROP: &str = r#"{source_uid: "win-1", window: true, meets: [{days: ["mon", "tue", "wed", "thu", "fri"], start: "07:30", end: "23:30"}]}"#;
const WIN_MEETS: &str = r#"[{"days":["mon","tue","wed","thu","fri"],"start":"07:30","end":"23:30"}]"#;

#[test]
fn the_summary_card_reads_the_classes_and_class_times_from_step3() {
    helper10("summaryLine10");
    let s = node(r#"h.summaryLine10({step3: {classes: 4, class_times: 11}})"#);
    assert_eq!(s, serde_json::json!("4 classes and 11 class times, set from your school's registrar. Nothing to type."));
    let zero = node(r#"h.summaryLine10({step3: {classes: 0, class_times: 0}})"#);
    assert!(zero.as_str().is_some_and(|t| t.starts_with("0 classes and 0 class times")), "a 0 is a count: {zero}");
    for st in [r#"{step3: {classes: 4}}"#, r#"{step3: {classes: null, class_times: 11}}"#, r#"{step3: null}"#, "null"] {
        assert_eq!(node(&format!("h.summaryLine10({st})")), serde_json::json!(""), "no card text for {st}");
    }
}

#[test]
fn step_tens_pane_has_the_card_the_class_times_toggle_and_the_questions_heading() {
    let c = code(&read("onboarding.js"));
    let r = function(&c, "render10");
    for want in ["summaryLine10", "Show the class times", "Questions only you can answer"] {
        assert!(r.contains(want), "render10 must carry {want}: {r}");
    }
}

#[test]
fn the_proposals_split_into_events_and_the_one_window_proposal() {
    helper10("splitProposals10");
    let v = node(&format!(r#"h.splitProposals10({{proposals: [{{source_uid: "cal-a1", title: "Chess club", window: false}}, {WIN_PROP}, {{source_uid: "cal-b2", title: "Pool shift"}}]}})"#));
    let ev: Vec<_> = v["events"].as_array().expect("events").iter().map(|e| e["source_uid"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(ev, vec!["cal-a1", "cal-b2"], "the window proposal is not an event");
    assert_eq!(v["window"]["source_uid"], serde_json::json!("win-1"));
    for reply in [r#"{proposals: []}"#, "null"] {
        let none = node(&format!("h.splitProposals10({reply})"));
        assert!(none["window"].is_null() && none["events"] == serde_json::json!([]), "no calendar ({reply}): {none}");
    }
}

#[test]
fn the_firmness_control_is_three_buttons_with_the_engines_level_pressed_and_no_select() {
    helper10("firmnessHtml10");
    let pressed_word = |html: &str| -> String {
        let at = html.find("aria-pressed=\"true\"").expect("one button is pressed");
        let end = html[at..].find("</button>").map(|i| at + i).expect("a closing button");
        strip_text(&html[at..end])
    };
    for (lvl, word) in [("hard", "Must keep"), ("soft", "Usually"), ("optional", "Optional")] {
        let html = node(&format!(r#"h.firmnessHtml10("{lvl}")"#)).as_str().unwrap_or_default().to_string();
        assert!(html.contains("role=\"group\"") && !html.contains("<select"), "a segmented group, not a select: {html}");
        let t = strip_text(&html);
        for w in ["Must keep", "Usually", "Optional"] { assert!(t.contains(w), "{w} in: {t}"); }
        assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1, "exactly one pressed: {html}");
        assert!(pressed_word(&html).contains(word), "{lvl} presses {word}: {html}");
        assert!(html.contains(&format!("data-level=\"{lvl}\"")), "each button names its engine level: {html}");
    }
    let none = node(r#"h.firmnessHtml10("")"#).as_str().unwrap_or_default().to_string();
    assert!(pressed_word(&none).contains("Usually"), "soft is the default: {none}");
}

#[test]
fn an_event_row_keeps_mine_and_not_mine_beside_the_segmented_control_in_place_of_the_select() {
    let c = code(&read("onboarding.js"));
    let row = function(&c, "propHtml");
    assert!(row.contains("firmnessHtml10") && !row.contains("<select"), "F11c's select is replaced: {row}");
    assert!(row.contains("data-act10=\"mine\"") && row.contains("data-act10=\"not\""), "Mine and Not mine stay");
    assert!(row.contains("p.level"), "Knowlu's level is preselected from the proposal");
}

#[test]
fn what_the_confirm_sends_for_events_is_unchanged_mine_with_its_level_and_not_mine_keys() {
    helper10("confirmArg10");
    let rows = r#"[{key: "cal-a1", answer: "mine", level: "hard"}, {key: "cal-b2", answer: "not", level: "soft"}, {key: "cal-c3", answer: "", level: "optional"}]"#;
    let v = node(&format!(r#"h.confirmArg10({rows}, null, {{answered: false, start: "", end: ""}})"#));
    assert_eq!(v, serde_json::json!({"mine": [{"source_uid": "cal-a1", "level": "hard"}], "not_mine": ["cal-b2"]}), "no window key; an unanswered row sends nothing");
}

#[test]
fn the_window_key_appears_only_after_an_answer_and_a_blank_question_sends_none() {
    helper10("windowArg10");
    helper10("confirmArg10");
    for win in [r#"{answered: false, start: "07:30", end: "23:30"}"#, r#"{answered: true, start: "", end: "23:30"}"#, r#"{answered: true, start: "07:30", end: ""}"#] {
        assert!(node(&format!("h.windowArg10({WIN_PROP}, {win})")).is_null(), "no window for {win}");
        assert!(node(&format!("h.windowArg10(null, {win})")).is_null(), "no window for {win} without a proposal");
        let v = node(&format!("h.confirmArg10([], {WIN_PROP}, {win})"));
        assert!(v.get("window").is_none(), "confirm has no window key for {win}: {v}");
    }
}

#[test]
fn an_unchanged_window_sends_the_proposals_own_meets_as_json_and_a_changed_one_sends_days_start_end() {
    helper10("windowArg10");
    helper10("confirmArg10");
    let same = node(&format!(r#"h.windowArg10({WIN_PROP}, {{answered: true, start: "07:30", end: "23:30"}})"#));
    assert_eq!(same, serde_json::json!(WIN_MEETS), "JSON.stringify of the proposal's meets, unchanged");
    let changed = node(&format!(r#"h.windowArg10({WIN_PROP}, {{answered: true, start: "08:00", end: "23:30"}})"#));
    assert_eq!(changed, serde_json::json!(r#"[{"days":["mon","tue","wed","thu","fri"],"start":"08:00","end":"23:30"}]"#), "the proposal's days, the student's times");
    let none = node(r#"h.windowArg10(null, {answered: true, start: "06:45", end: "22:15"})"#);
    assert_eq!(none, serde_json::json!(r#"[{"days":["mon","tue","wed","thu","fri","sat","sun"],"start":"06:45","end":"22:15"}]"#), "every day with no proposal");
    let v = node(&format!(r#"h.confirmArg10([], {WIN_PROP}, {{answered: true, start: "07:30", end: "23:30"}})"#));
    assert_eq!(v["window"], serde_json::json!(WIN_MEETS), "confirm carries it under `window`");
}

#[test]
fn the_window_question_preselects_the_guess_and_starts_empty_without_one() {
    helper10("windowFields10");
    assert_eq!(node(&format!("h.windowFields10({WIN_PROP})")), serde_json::json!({"start": "07:30", "end": "23:30"}));
    assert_eq!(node("h.windowFields10(null)"), serde_json::json!({"start": "", "end": ""}));
}

#[test]
fn the_window_goes_to_the_engine_through_commitments_confirm_and_nowhere_else() {
    let c = code(&read("onboarding.js"));
    let body = function(&c, "confirm10");
    assert_eq!(invoke_calls(body, "commitments_confirm").len(), 1, "one confirm call");
    assert!(body.contains("confirmArg10"), "confirm10 builds its confirm object with the pure helper: {body}");
    assert!(!c.contains("planning-day") && !c.contains("planning_day"), "the page names no planning-day note and writes none");
}

#[test]
fn the_bar_counts_the_questions_on_screen_and_the_students_own_presses() {
    helper10("answeredCount10");
    helper10("confirmBarLine10");
    let rows = r#"[{answer: "mine"}, {answer: "not"}, {answer: ""}]"#;
    assert_eq!(node(&format!("h.answeredCount10({rows}, {{answered: false}})")), serde_json::json!({"n": 2, "m": 4}), "three events and the window");
    assert_eq!(node(&format!("h.answeredCount10({rows}, {{answered: true}})")), serde_json::json!({"n": 3, "m": 4}));
    assert_eq!(node("h.answeredCount10([], {answered: false})"), serde_json::json!({"n": 0, "m": 1}), "the window question alone");
    assert_eq!(node("h.confirmBarLine10(2, 4)"), serde_json::json!("2 of 4 answered. Anything you leave blank comes back as a question over the next few days."));
}

#[test]
fn the_sticky_bar_holds_confirm_and_the_count_and_the_segmented_control_is_styled() {
    let c = code(&read("onboarding.js"));
    let r = function(&c, "render10");
    assert!(r.contains("ob-10-bar") && r.contains("Confirm my week") && r.contains("data-act10=\"confirm\""), "the confirm button sits in the bar: {r}");
    assert!(c.contains("confirmBarLine10("), "the page paints the count line from the helper");
    let css = code(&read("onboarding.css"));
    assert!(css.contains("ob-10-bar") && css.contains("position: sticky"), "the bar is pinned to the pane's bottom");
    assert!(css.contains("ob-seg"), "the segmented control has a style");
}

#[test]
fn step_tens_move_on_failure_sentence_stays_and_no_late_rule_question_exists() {
    let c = code(&read("onboarding.js"));
    assert!(function(&c, "confirm10").contains("Your week is saved, but Knowlu couldn't move on."), "F11c1's message stays");
    let lower = c.to_lowercase();
    for banned in ["late rule", "late work", "late_rule", "late-rule", "laterule"] {
        assert!(!lower.contains(banned), "no late-rule question or placeholder until O5: found {banned}");
    }
    assert!(function(&c, "render10").contains("When does your day run?"), "the day-window question is asked in the pane");
}

// ---- F11p3/F11p4 review: the two-entry window, "That's right", names, firmness means Mine --------
//
// Helper shape changes these pins fix, for the implementer (the pins above stay as they are):
// - `windowFields10(proposal)` keeps `{start, end}` (the first entry) and ADDS `entries`: every meet as
//   `{days, start, end}`, in order. (The older pin compares a one-entry proposal with exact equality
//   to `{start, end}`; so `entries` is added only when the proposal has more than one meet, or that
//   older pin is updated by its owner. Either way these pins read `entries` on a two-entry proposal.)
// - `win` may carry `entries: [{start, end}, ...]`, parallel to the proposal's meets. When present it
//   wins over `win.start`/`win.end` (which still mean the first entry, as before).
//   `windowArg10` sends `null` unless `answered` and every time in every entry is set. If no entry
//   differs from the proposal it sends `JSON.stringify(proposal.meets)`; otherwise every meet, each
//   with its own days, only the changed entry's times replaced.
// - `answeredCount10(rows, win)`: `win.answered` counts only when every time is set (the pair, or
//   every `entries` pair); a "That's right" on empty fields counts for nothing.
// - `firmnessHtml10(level, title)`: the group's aria-label is "How firm is {title}" (escaped).
// - A row may carry `picked: true`: the student pressed a firmness level (`data-picked` on the row).
//   `confirmArg10` sends a picked row whose answer is not "not" in `mine` with its level;
//   `answeredCount10` counts it. "not" still overrides.

const WIN2_PROP: &str = r#"{source_uid: "win-1", window: true, meets: [{days: ["mon","tue","wed","thu","fri"], start: "07:30", end: "22:00"}, {days: ["sat","sun"], start: "10:00", end: "22:00"}]}"#;
const WIN2_MEETS: &str = r#"[{"days":["mon","tue","wed","thu","fri"],"start":"07:30","end":"22:00"},{"days":["sat","sun"],"start":"10:00","end":"22:00"}]"#;
const WIN2_SAME: &str = r#"[{start: "07:30", end: "22:00"}, {start: "10:00", end: "22:00"}]"#;

#[test]
fn a_two_entry_window_proposal_exposes_both_entries_through_windowfields() {
    helper10("windowFields10");
    let f = node(&format!("h.windowFields10({WIN2_PROP})"));
    let want: serde_json::Value = serde_json::from_str(WIN2_MEETS).unwrap();
    assert_eq!(f["entries"], want, "both entries, weekdays then the weekend: {f}");
}

#[test]
fn an_untouched_two_entry_window_sends_no_window_key() {
    helper10("confirmArg10");
    let v = node(&format!(r#"h.confirmArg10([], {WIN2_PROP}, {{answered: false, entries: {WIN2_SAME}}})"#));
    assert!(v.get("window").is_none(), "no window key: {v}");
}

#[test]
fn thats_right_on_a_two_entry_window_sends_both_entries_unchanged() {
    helper10("windowArg10");
    let w = node(&format!(r#"h.windowArg10({WIN2_PROP}, {{answered: true, entries: {WIN2_SAME}}})"#));
    assert_eq!(w, serde_json::json!(WIN2_MEETS), "JSON.stringify(proposal.meets), both entries");
    let v = node(&format!(r#"h.confirmArg10([], {WIN2_PROP}, {{answered: true, entries: {WIN2_SAME}}})"#));
    assert_eq!(v["window"], serde_json::json!(WIN2_MEETS));
}

#[test]
fn an_edited_weekday_start_sends_both_entries_with_only_that_one_changed() {
    helper10("windowArg10");
    let edited = r#"[{start: "08:00", end: "22:00"}, {start: "10:00", end: "22:00"}]"#;
    let w = node(&format!(r#"h.windowArg10({WIN2_PROP}, {{answered: true, entries: {edited}}})"#));
    let got: serde_json::Value = serde_json::from_str(w.as_str().unwrap_or("null")).expect("a JSON window string");
    let mut want: serde_json::Value = serde_json::from_str(WIN2_MEETS).unwrap();
    want[0]["start"] = serde_json::json!("08:00");
    assert_eq!(got, want, "the weekend entry is the proposal's own; no day is dropped");
    let weekend = r#"[{start: "07:30", end: "22:00"}, {start: "09:00", end: "23:00"}]"#;
    let w = node(&format!(r#"h.windowArg10({WIN2_PROP}, {{answered: true, entries: {weekend}}})"#));
    let got: serde_json::Value = serde_json::from_str(w.as_str().unwrap_or("null")).expect("a JSON window string");
    let mut want: serde_json::Value = serde_json::from_str(WIN2_MEETS).unwrap();
    want[1]["start"] = serde_json::json!("09:00");
    want[1]["end"] = serde_json::json!("23:00");
    assert_eq!(got, want, "an edited weekend entry changes only that entry");
    let blank = r#"[{start: "07:30", end: "22:00"}, {start: "", end: "22:00"}]"#;
    assert!(node(&format!(r#"h.windowArg10({WIN2_PROP}, {{answered: true, entries: {blank}}})"#)).is_null(), "a blank time in any entry sends none");
}

#[test]
fn thats_right_sets_the_answer_and_never_toggles_it_off() {
    let c = code(&read("onboarding.js"));
    let click = function(&c, "onClick10");
    let at = click.find("\"win\"").expect("onClick10 handles the That's right button");
    let branch = &click[at..at + click[at..].find("return;").expect("the branch returns")];
    assert!(branch.contains("answered = true"), "That's right sets the answer: {branch}");
    assert!(!branch.contains("!w10.win.answered") && !branch.contains("answered = !"), "it never toggles off, so a typed time is still sent: {branch}");
}

#[test]
fn a_blank_window_is_not_counted_as_answered_until_both_times_are_set() {
    helper10("answeredCount10");
    for win in [
        r#"{answered: true, start: "", end: ""}"#,
        r#"{answered: true, start: "07:00", end: ""}"#,
        r#"{answered: true, entries: [{start: "", end: ""}]}"#,
        r#"{answered: true, entries: [{start: "07:30", end: "22:00"}, {start: "", end: ""}]}"#,
    ] {
        assert_eq!(node(&format!("h.answeredCount10([], {win})")), serde_json::json!({"n": 0, "m": 1}), "not counted for {win}");
    }
    for win in [r#"{answered: true, start: "07:00", end: "22:00"}"#, &format!(r#"{{answered: true, entries: {WIN2_SAME}}}"#)] {
        assert_eq!(node(&format!("h.answeredCount10([], {win})")), serde_json::json!({"n": 1, "m": 1}), "counted for {win}");
    }
}

#[test]
fn the_firmness_control_names_its_event() {
    helper10("firmnessHtml10");
    let html = node(r#"h.firmnessHtml10("soft", "Chess club")"#).as_str().unwrap_or_default().to_string();
    assert!(html.contains("aria-label=\"How firm is Chess club\""), "named for the event: {html}");
    let odd = node(r#"h.firmnessHtml10("soft", "Bob's \"<b>\" lab")"#).as_str().unwrap_or_default().to_string();
    assert!(!odd.contains("<b>") && odd.contains("How firm is Bob"), "the title is escaped: {odd}");
    let c = code(&read("onboarding.js"));
    assert!(function(&c, "propHtml").contains("firmnessHtml10(p.level, p.title)"), "each row passes its title");
}

#[test]
fn the_time_inputs_accessible_name_starts_with_their_visible_label() {
    let c = code(&read("onboarding.js"));
    let mut seen = 0;
    for (at, _) in c.match_indices("type=\"time\"") {
        let tag = &c[at..at + c[at..].find('>').expect("a closing >")];
        seen += 1;
        if let Some(i) = tag.find("aria-label=\"") {
            let name = &tag[i + 12..];
            assert!(name.starts_with("Wake") || name.starts_with("Bed"), "the name must start with the visible label (Wake/Bed): {tag}");
        }
    }
    assert!(seen >= 1, "the page has time inputs");
    assert!(c.contains("Wake") && c.contains("Bed"), "the visible labels stay");
}

#[test]
fn picking_a_firmness_level_counts_as_mine_and_not_mine_still_overrides() {
    helper10("confirmArg10");
    helper10("answeredCount10");
    let rows = r#"[{key: "cal-a1", answer: "", level: "hard", picked: true}, {key: "cal-b2", answer: "", level: "soft", picked: false}, {key: "cal-c3", answer: "not", level: "optional", picked: true}, {key: "cal-d4", answer: "mine", level: "soft", picked: true}]"#;
    let v = node(&format!(r#"h.confirmArg10({rows}, null, {{answered: false, start: "", end: ""}})"#));
    assert_eq!(v, serde_json::json!({"mine": [{"source_uid": "cal-a1", "level": "hard"}, {"source_uid": "cal-d4", "level": "soft"}], "not_mine": ["cal-c3"]}),
        "a level press alone is Mine at that level; Not mine overrides; no press sends nothing");
    assert_eq!(node(&format!(r#"h.answeredCount10({rows}, {{answered: false}})"#)), serde_json::json!({"n": 3, "m": 5}), "the level-only row counts as answered");
    let c = code(&read("onboarding.js"));
    assert!(function(&c, "rows10").contains("picked"), "rows10 reads whether a level was pressed");
    assert!(function(&c, "onClick10").contains("data-picked"), "a level press marks its row picked");
}

#[test]
fn step_ten_has_one_intro_sentence_above_the_event_rows() {
    let c = code(&read("onboarding.js"));
    assert!(!c.contains("found these repeating"), "askProposals' second intro is gone");
    assert!(!function(&c, "askProposals").contains("su-small"), "askProposals adds no intro paragraph");
    assert_eq!(c.matches("Your calendar repeats these.").count(), 1, "the one intro stays");
}

#[test]
fn an_answer_typed_into_the_blank_pair_before_the_guess_arrives_sends_all_seven_days_and_no_guess_entry() {
    helper10("confirmArg10");
    // The student typed into the blank pair; the two-entry guess arrived afterwards and was never shown (`from: null`).
    let v = node(&format!(r#"h.confirmArg10([], {WIN2_PROP}, {{answered: true, from: null, start: "08:00", end: "23:00"}})"#));
    let got: serde_json::Value = serde_json::from_str(v["window"].as_str().unwrap_or("null")).expect("a JSON window string");
    let want = serde_json::json!([{"days": ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], "start": "08:00", "end": "23:00"}]);
    assert_eq!(got, want, "one entry over all seven days with the typed times; no 07:30/10:00 guess entry: {v}");
    // A guess that was shown (`from` is that guess) still sends its own days.
    let shown = node(&format!(r#"h.confirmArg10([], null, {{answered: true, from: {WIN2_PROP}, entries: {WIN2_SAME}}})"#));
    assert_eq!(shown["window"], serde_json::json!(WIN2_MEETS), "the shown guess is what an unchanged answer sends");
}

#[test]
fn a_two_entry_pair_puts_its_days_in_each_label_and_a_single_pair_keeps_wake_and_bed() {
    helper10("winPair10");
    let two = node(r#"h.winPair10(0, "Mon–Fri", {start: "07:30", end: "22:00"})"#).as_str().unwrap_or_default().to_string();
    assert!(two.contains("Mon–Fri wake") && two.contains("Mon–Fri bed"), "days inside each label: {two}");
    let one = node(r#"h.winPair10(0, "", {start: "", end: ""})"#).as_str().unwrap_or_default().to_string();
    assert!(one.contains("Wake <input") && one.contains("Bed <input"), "one pair keeps Wake/Bed: {one}");
}

#[test]
fn a_firmness_press_shows_mine_pressed_and_pressing_mine_again_clears_the_row() {
    helper10("rowAfter10");
    let press = |row: &str, how: &str| node(&format!(r#"h.rowAfter10({row}, "{how}")"#));
    let a = press(r#"{answer: "", picked: false}"#, "level");
    assert_eq!(a, serde_json::json!({"answer": "mine", "picked": true}), "a firmness press is Mine, pressed: {a}");
    let b = press(r#"{answer: "mine", picked: true}"#, "mine");
    assert_eq!(b, serde_json::json!({"answer": "", "picked": false}), "Mine again clears the row and its picked mark: {b}");
    let c = press(r#"{answer: "not", picked: false}"#, "level");
    assert_eq!(c["answer"], serde_json::json!("not"), "a firmness press never overrides Not mine: {c}");
    let d = press(r#"{answer: "", picked: true}"#, "not");
    assert_eq!(d, serde_json::json!({"answer": "not", "picked": false}), "Not mine overrides and clears picked: {d}");
    let click = function(&code(&read("onboarding.js")), "onClick10").to_string();
    assert!(click.contains("rowAfter10"), "the click handler uses the one rule");
}
