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
];

/// Only commands the plan names: section 4.5's list, F5's, the step-2 sign-in, step 5's
/// `mail_setup_progress` (read-only; plan line 643), step 10's confirm screen and `launch_state`
/// (plan line 3053). Step 5 never starts mail work: `mail_setup_start` is banned.
const ALLOWED: &[&str] = &[
    "onboarding_status", "onboarding_answer", "onboarding_skip", "onboarding_retry", "onboarding_finish", "onboarding_quit",
    "onboarding_sign_out", "onboarding_attach_session", "onboarding_entitlement", "onboarding_checkout",
    "onboarding_capture_link", "onboarding_paste_link", "campus_search", "store_credentials", "discover_coursework",
    "open_lms_window", "capture_courses", "close_lms_window", "google_sign_in", "send_magic_link", "verify_email_code",
    "google_connect", "google_status", "mail_setup_progress",
    "your_week", "commitment_proposals", "commitments_confirm", "launch_state",
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
        ("step 8", &["store_credentials", "discover_coursework"]),
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
    assert!(!enabled(status(r#"{"gate": {"open": false}}"#)), "a closed gate");
    assert!(!enabled(status(r#"{"exiting": {"job": "slot.partial", "long": true, "since": "x"}}"#)), "while an exit drains");
    // The page does not re-derive job states: an absent registrar.confirm and a running
    // school.store change nothing when the server says ready; all-done jobs change nothing when not.
    assert!(enabled(status(r#"{"jobs": {"school.apply": {"state": "done"}, "school.store": {"state": "running"}}}"#)));
    assert!(!enabled(status(r#"{"finish_ready": false, "jobs": {"registrar.confirm": {"state": "done"}, "school.apply": {"state": "done"}, "school.courses": {"state": "done"}, "config.ingest": {"state": "done"}}}"#)));
    let c = code(&read("onboarding.js"));
    let f = function(&c, "openKnowluEnabled");
    assert!(f.contains("finish_ready") && !f.contains("jobs") && !f.contains("registrar.confirm"), "it reads finish_ready only");
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
    let at = c.find("invoke(\"store_credentials\"").expect("step 8 stores logins");
    let handler = enclosing_fn(&c, at);
    let after = &handler[handler.find("invoke(\"store_credentials\"").unwrap()..];
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
    let handler = span(enclosing_fn(&c, c.find("invoke(\"store_credentials\"").unwrap()));
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
    let h = enclosing_fn(&c, c.find("invoke(\"store_credentials\"").unwrap());
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
    let f = enclosing_fn(&c, c.find("invoke(\"discover_coursework\"").expect("discover"));
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
