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
//! - Every Tauri call is `invoke("<literal name>", ...)`.

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
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let keep = |ch: char| if ch == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' { out.push(' '); i += 1; }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            while i < c.len() && !(c[i] == '*' && c.get(i + 1) == Some(&'/')) { out.push(keep(c[i])); i += 1; }
            for _ in 0..2 { if i < c.len() { out.push(' '); i += 1; } }
        } else if ch == '"' || ch == '\'' || ch == '`' {
            out.push(ch);
            i += 1;
            while i < c.len() && c[i] != ch {
                if c[i] == '\\' && i + 1 < c.len() { out.push(if strings_too { ' ' } else { c[i] }); i += 1; }
                if c[i] == '\n' && ch != '`' { break; }
                out.push(if strings_too { keep(c[i]) } else { c[i] });
                i += 1;
            }
            if i < c.len() && c[i] == ch { out.push(ch); i += 1; }
        } else { out.push(ch); i += 1; }
    }
    out
}

/// The source of the smallest `function` whose body holds byte `at` (anonymous callbacks included).
fn enclosing_fn(src: &str, at: usize) -> &str {
    let sk = skeleton(src);
    let mut best: Option<(usize, usize)> = None;
    for (start, _) in sk.match_indices("function") {
        let Some(open) = sk[start..].find('{').map(|o| start + o) else { continue };
        let (mut depth, mut end) = (0i32, None);
        for (k, b) in sk[open..].bytes().enumerate() {
            if b == b'{' { depth += 1; }
            if b == b'}' { depth -= 1; if depth == 0 { end = Some(open + k + 1); break; } }
        }
        if let Some(end) = end { if start < at && at < end && best.is_none_or(|(s, _)| start > s) { best = Some((start, end)); } }
    }
    let (s, e) = best.unwrap_or_else(|| panic!("no function encloses byte {at}"));
    &src[s..e]
}

/// The body of `function name(...) {...}`.
fn function<'a>(src: &'a str, name: &str) -> &'a str {
    let at = src.find(&format!("function {name}(")).unwrap_or_else(|| panic!("onboarding.js has no function {name}("));
    let open = at + src[at..].find('{').expect("a body");
    enclosing_fn(src, open + 1)
}

/// Every command name passed to `invoke(`; panics on a dynamic first argument.
fn invoked(js: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (at, _) in js.match_indices("invoke") {
        let tail = js[at + 6..].trim_start();
        let before_ok = js[..at].chars().last().is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if !before_ok || !tail.starts_with('(') { continue; }
        let arg = tail[1..].trim_start();
        let quote = arg.chars().next().unwrap_or(' ');
        assert!(quote == '"' || quote == '\'', "invoke( must take a literal command name, got: {}", &arg[..arg.len().min(40)]);
        let name: String = arg[1..].chars().take_while(|&ch| ch != quote).collect();
        names.push(name);
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
const window = { __TAURI__: { core: { invoke: () => new Promise(() => {}) } }, addEventListener() {}, KNOWLU_COPY: { GOOGLE_TESTING: "GT" } };
const ctx = vm.createContext({ window, document: stub(), console, setTimeout: () => 0, setInterval: () => 0, clearTimeout() {}, clearInterval() {}, Date, JSON, Math, location: stub() });
vm.runInContext(fs.readFileSync(process.argv[1], "utf8"), ctx);
const h = window.KNOWLU_ONBOARDING;
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
];

/// §4.5's list, F5's, the step-2 sign-in, step 5's mail window and step 10's confirm screen.
const ALLOWED: &[&str] = &[
    "onboarding_status", "onboarding_answer", "onboarding_skip", "onboarding_retry", "onboarding_finish", "onboarding_quit",
    "onboarding_sign_out", "onboarding_attach_session", "onboarding_entitlement", "onboarding_checkout",
    "onboarding_capture_link", "onboarding_paste_link", "campus_search", "store_credentials", "discover_coursework",
    "open_lms_window", "capture_courses", "close_lms_window", "google_sign_in", "send_magic_link", "verify_email_code",
    "google_connect", "google_status", "mail_setup_start", "mail_setup_progress", "mail_setup_answer", "mail_setup_stop",
    "mail_status", "mail_sign_in", "your_week", "commitment_proposals", "commitments_confirm", "launch_state",
];

#[test]
fn the_onboarding_assets_exist_and_index_html_loads_them() {
    let js = read("onboarding.js");
    let css = read("onboarding.css");
    assert!(!js.trim().is_empty() && !css.trim().is_empty());
    let html = read("index.html");
    assert!(html.contains("onboarding.js") && html.contains("onboarding.css"), "index.html loads both new files");
    assert!(!html.contains("type=\"module\""), "a classic script, like console.js");
    for (name, text) in [("onboarding.js", &js), ("onboarding.css", &css)] {
        assert!(!text.contains("http://") && !text.contains("https://"), "{name} references the network");
    }
    let c = code(&js);
    assert!(c.contains("__TAURI__.core.invoke"), "calls go through the one invoke");
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
    assert_eq!(screen.matches("<button").count(), 1, "the restart screen has exactly one button");
    assert!(screen.contains("onboarding_quit") || screen.contains("finishLater"), "its button exits through onboarding_quit");
}

#[test]
fn the_shots_hook_opens_the_frame_at_a_step() {
    let all = format!("{}{}", code(&read("onboarding.js")), code(&read("console.js")));
    assert!(all.contains("startOnboarding") && all.contains("KNOWLU_SHOTS"), "KNOWLU_SHOTS.startOnboarding(step) exists");
    assert!(code(&read("onboarding.js")).contains("fixture"), "the screens render from the console's ?fixture= path");
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
        let line_start = c[..at].rfind('\n').map_or(0, |n| n + 1);
        let line_end = at + c[at..].find('\n').unwrap_or(c.len() - at);
        let line = &c[line_start..line_end];
        if line.contains("textContent") || line.contains("innerHTML") || line.contains("insertAdjacent") {
            assert!(line.contains("reasonWords("), "a raw reason is written to the page: {line}");
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
fn the_reveals_button_waits_for_the_gate_and_every_local_lane_job() {
    let lane = ["school.apply", "school.courses", "registrar.confirm", "config.ingest"];
    let status = |open: bool, odd: Option<(&str, &str)>| {
        let mut jobs = serde_json::Map::new();
        for j in lane { jobs.insert(j.into(), serde_json::json!({"state": "done"})); }
        if let Some((j, s)) = odd { jobs.insert(j.into(), serde_json::json!({"state": s})); }
        serde_json::json!({"ok": true, "gate": {"open": open, "still_coming": [], "failed": [], "not_connected": []}, "jobs": jobs})
    };
    let enabled = |s: serde_json::Value| node(&format!("h.openKnowluEnabled({s})")).as_bool().unwrap();
    assert!(enabled(status(true, None)));
    assert!(!enabled(status(false, None)), "a closed gate keeps the button disabled");
    for j in lane {
        for s in ["pending", "running", "held", "failed"] {
            assert!(!enabled(status(true, Some((j, s)))), "{j} {s} must disable Open Knowlu");
        }
    }
}

#[test]
fn the_reveal_names_a_failed_item_with_its_fix_and_a_long_chain_as_still_coming() {
    let status = serde_json::json!({"ok": true, "gate": {
        "open": true,
        "still_coming": ["first.grades", "backfill.older_mail"],
        "failed": [{"key": "first.sync", "reason": "first_sync_failed", "fix": "Knowlu couldn't reach your account. Press Try again."}],
        "not_connected": [{"key": "mail", "cost_key": "mail_dates_missed"}]}});
    let m = node(&format!("h.revealModel({status})"));
    let failed = m["failed"].as_array().expect("failed list");
    assert_eq!(failed.len(), 1, "a failed item is named, never hidden");
    assert!(failed[0]["fix"].as_str().unwrap().contains("Press Try again"), "with its fix: {failed:?}");
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
    assert!(row["line"].as_str().unwrap().contains("Press Try again to run it once more."), "{row}");
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
    assert!(cap.contains(".error") && cap.contains("ok"), "a refusal on the identity provider's page is shown");
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
    assert!(rm.contains("onboarding_answer") && rm.contains('3'), "Remove is an onboarding_answer for step 3");
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
            let before = &text[..at];
            let quoted = before.ends_with('"') || before.ends_with("\\\"") || before.ends_with('\'');
            let typed = before.trim_end_matches(['"', '\'', '\\']).trim_end().ends_with("type=")
                || before.trim_end_matches(['"', '\'', '\\']).trim_end().ends_with("type:")
                || before.trim_end_matches(['"', '\'', '\\']).trim_end().ends_with("type =");
            if !(quoted && typed) { continue; }
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
    for (at, _) in c.match_indices(".value") {
        let ctx = c[at.saturating_sub(120)..at].to_lowercase();
        if !(ctx.contains("password") || ctx.contains("username") || ctx.contains("zybooks") || ctx.contains("vhl")) { continue; }
        let inside = |s: (usize, usize)| at >= s.0 && at < s.1;
        assert!(inside(handler) || inside(clear), "a credential field's value is touched outside the store handler: {}", &c[at.saturating_sub(120)..at + 20]);
    }
    for (at, _) in c.match_indices("invoke(\"onboarding_answer\"") {
        assert!(!c[at..(at + 240).min(c.len())].to_lowercase().contains("password"), "no credential in an onboarding_answer payload");
    }
}

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
