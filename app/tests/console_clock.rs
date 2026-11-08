//! T4 of the twelve-hour clock plan (`docs/plans/2026-11-07-twelve-hour-clock-plan.md`, §4): the
//! console's time formatter, pinned before `app/static/console.js` changes. T5 makes these pass.
//!
//! Three layers, so that nothing slips past by being on a row the others do not reach:
//! - **Layer 1, the render scan.** The page's own `paint` runs under node against a recording
//!   `document`; every `innerHTML`, `textContent` and `title` it writes is scanned for a 24-hour
//!   form, a bare 12-hour clock with no am/pm, and a list of known strings.
//! - **Layer 2, positive pins.** One row per site: an anchor, and the helper call required on its line.
//! - **Layer 3, forbidden forms.** Hand-rolled time slicing, anywhere outside the four helpers.
//!
//! Layers 2 and 3 read the page's code through `strip`, a small state machine whose own self-checks
//! (`the_scanner_*`, `every_layer_2_anchor_*`) pass today. Everything else is red until T5, and each
//! failure names its site or its missing helper.
//!
//! The helpers are lifted out of the page's IIFE under `node`, the way
//! `onboarding_static.rs::main_handover_html` lifts `handoverHtml`: one line is appended inside the
//! wrapper, the source is otherwise as shipped. The `document` here is NOT that harness's discarding
//! Proxy: it records writes. Tests never leave the machine: no `fetch`, no `invoke` that settles.

use regex::Regex;
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const HARNESS: &str = r####"
const fs = require("fs"), vm = require("vm");
process.on("unhandledRejection", () => {});
const input = JSON.parse(fs.readFileSync(0, "utf8"));
const log = [];
const RECORDED = new Set(["innerHTML", "textContent", "title"]);
const stub = () => new Proxy(function () {}, {
  get: (t, k) => (k === Symbol.toPrimitive ? () => "" : k === "then" || typeof k === "symbol" ? undefined : stub()),
  apply: () => stub(), construct: () => stub(), set: () => true });
const noop = () => undefined;
const classList = new Proxy({}, { get: (t, k) => (k === "contains" ? () => false : noop) });
// A recorder records innerHTML / textContent / title on `set` and lets every other member through.
function recorder(id) {
  const store = {};
  const rec = new Proxy(function () {}, {
    get(t, k) {
      if (k === Symbol.toPrimitive) return () => "";
      if (k === "then" || typeof k === "symbol") return undefined;
      if (Object.prototype.hasOwnProperty.call(store, k)) return store[k];
      switch (k) {
        case "querySelectorAll": return () => [];
        case "querySelector": return () => recorder(id + ">q");
        case "closest": case "prepend": case "remove": case "append": return noop;
        case "contains": return () => false;
        case "classList": return classList;
        case "appendChild": return (c) => {
          for (const p of ["title", "textContent"]) { const v = c && c[p]; if (typeof v === "string") log.push({ id, prop: "child." + p, value: v }); }
          return c; };
      }
      return stub();
    },
    set(t, k, v) { store[k] = v; if (RECORDED.has(k)) log.push({ id, prop: k, value: String(v) }); return true; },
    apply: () => stub(), construct: () => stub() });
  return rec;
}
const els = new Map();
const doc = new Proxy({}, {
  get(t, k) {
    if (k === "getElementById") return (id) => { if (!els.has(id)) els.set(id, recorder(id)); return els.get(id); };
    if (k === "createElement") return () => recorder("created");
    if (k === "querySelectorAll") return () => [];
    if (k === "querySelector") return () => recorder("document>q");
    if (k === Symbol.toPrimitive) return () => "";
    if (k === "then" || typeof k === "symbol") return undefined;
    return stub();
  },
  set: () => true });
const g = { document: doc, location: { search: "", hash: "", href: "http://localhost/", pathname: "/", origin: "http://localhost" },
  URLSearchParams, URL, console, Date, JSON, Math, navigator: { userAgent: "node", language: "en" },
  localStorage: { getItem: () => null, setItem() {}, removeItem() {} },
  requestAnimationFrame: () => 0, setTimeout: () => 0, setInterval: () => 0, clearTimeout() {}, clearInterval() {},
  addEventListener() {}, removeEventListener() {}, fetch: () => { throw new Error("tests never leave the machine"); },
  __TAURI__: { core: { invoke: () => new Promise(() => {}) } } };
g.window = g;
const ctx = vm.createContext(g);
let src = fs.readFileSync(process.argv[1], "utf8");
const at = src.lastIndexOf("})();");
if (at < 0) { console.log(JSON.stringify({ harness_error: "no IIFE wrapper in console.js" })); process.exit(0); }
src = src.slice(0, at) + `
globalThis.__lift = { paint: typeof paint === "function" ? paint : null, current: typeof current === "object" ? current : null,
  clock12: typeof clock12 === "function" ? clock12 : null, clockRange: typeof clockRange === "function" ? clockRange : null,
  timeOf: typeof timeOf === "function" ? timeOf : null, stampOf: typeof stampOf === "function" ? stampOf : null };
` + src.slice(at);
try { vm.runInContext(src, ctx); } catch (e) { console.log(JSON.stringify({ harness_error: "page startup threw: " + e.stack })); process.exit(0); }
const lift = g.__lift;
if (!lift) { console.log(JSON.stringify({ harness_error: "the lift line did not run" })); process.exit(0); }
if (input.mode === "calls") {
  console.log(JSON.stringify(input.calls.map(([name, args]) => {
    if (!lift[name]) return { missing: name };
    try { return { value: lift[name](...args) }; } catch (e) { return { error: String(e) }; }
  })));
} else if (input.mode === "tz") {
  const d = new Date(input.iso);
  console.log(JSON.stringify({ h: d.getHours(), m: d.getMinutes() }));
} else if (input.mode === "paint") {
  if (!lift.paint || !lift.current) { console.log(JSON.stringify({ harness_error: "paint or current is not in the page" })); process.exit(0); }
  lift.current.view = input.view;
  try { lift.paint(input.state, true); } catch (e) { console.log(JSON.stringify({ harness_error: "paint threw: " + e.stack })); process.exit(0); }
  console.log(JSON.stringify({ log }));
}
"####;

/// Run the harness over `console.js` with the given request on stdin, under `TZ=America/Chicago`.
fn node(request: &Value) -> Value {
    let mut child = Command::new("node").arg("-e").arg(HARNESS).arg(Path::new("static/console.js"))
        .env("TZ", "America/Chicago").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("node must be installed");
    child.stdin.take().unwrap().write_all(request.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "node failed: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("harness output is not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    if let Some(e) = v.get("harness_error") { panic!("HARNESS ERROR (not a T5 failure): {e}"); }
    v
}

/// One call per entry; `None` when the page has no such helper yet (the expected red before T5).
fn call_all(calls: &[(&str, Vec<Value>)]) -> Vec<Option<Value>> {
    let req = json!({ "mode": "calls", "calls": calls.iter().map(|(n, a)| json!([n, a])).collect::<Vec<_>>() });
    node(&req).as_array().expect("array").iter().map(|r| {
        if let Some(e) = r.get("error") { panic!("a helper threw: {e}"); }
        r.get("value").cloned()
    }).collect()
}

/// Every case in `cases` as `(helper, args, expected)`; one failure line per mismatch or missing helper.
fn check_calls(cases: &[(&str, Vec<Value>, &str)]) {
    let calls: Vec<(&str, Vec<Value>)> = cases.iter().map(|(n, a, _)| (*n, a.clone())).collect();
    let got = call_all(&calls);
    let mut bad = Vec::new();
    for ((name, args, want), got) in cases.iter().zip(got) {
        match got {
            None => {
                let line = format!("the helper `{name}` is not in console.js yet");
                if !bad.contains(&line) { bad.push(line); }
            }
            Some(v) if v.as_str() == Some(*want) => {}
            Some(v) => bad.push(format!("{name}({args:?}) = {v}, want {want:?}")),
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// `TZ=America/Chicago` should make the harness's own `Date` read 9:05 for 14:05Z on 2026-09-01. If node
/// on the runner ignores `TZ`, the plan says to assert against the harness's own hours instead.
fn local_clock(iso: &str, literal: &str) -> String {
    let hm = |iso: &str| { let t = node(&json!({ "mode": "tz", "iso": iso })); (t["h"].as_u64().unwrap(), t["m"].as_u64().unwrap()) };
    if hm("2026-09-01T14:05:00.000Z").0 == 9 { return literal.to_string(); }
    let (h, m) = hm(iso);
    format!("{}{}{}", if h % 12 == 0 { 12 } else { h % 12 }, if m == 0 { String::new() } else { format!(":{m:02}") }, if h < 12 { "am" } else { "pm" })
}

fn fixture(name: &str) -> Value {
    let p = Path::new("..").join("engine").join("tests").join("fixtures").join(name);
    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(s.trim()).expect("a surface reference is one JSON line")
}

// ---- the helpers, one table each (the cases are T1's, string for string) ----

fn s(x: &str) -> Value { json!(x) }

#[test]
fn clock12_matches_the_engines_formatter_table() {
    let rows = [("00:00", "12am"), ("00:30", "12:30am"), ("09:00", "9am"), ("12:00", "12pm"), ("12:50", "12:50pm"),
        ("13:05", "1:05pm"), ("23:59", "11:59pm"), ("10:30", "10:30am"), ("19:00", "7pm")];
    let cases: Vec<_> = rows.iter().map(|(a, w)| ("clock12", vec![s(a)], *w)).collect();
    check_calls(&cases);
}

#[test]
fn clock_range_matches_the_engines_formatter_table() {
    // The dash is U+2013. The end always carries am/pm; the start only when it is on the other side of noon.
    let rows = [("10:00", "15:00", "10am–3pm"), ("19:00", "21:00", "7–9pm"), ("10:30", "11:15", "10:30–11:15am"),
        ("11:00", "12:00", "11am–12pm"), ("12:00", "13:00", "12–1pm"), ("11:30", "12:15", "11:30am–12:15pm"),
        ("08:00", "22:00", "8am–10pm"),
        // the blocks the references carry
        ("08:00", "09:00", "8–9am"), ("10:00", "12:00", "10am–12pm"), ("12:00", "12:50", "12–12:50pm"),
        ("15:00", "18:00", "3–6pm"), ("08:00", "12:00", "8am–12pm"), ("13:00", "13:45", "1–1:45pm"), ("13:45", "18:00", "1:45–6pm"),
        ("13:00", "14:15", "1–2:15pm")];
    let cases: Vec<_> = rows.iter().map(|(a, b, w)| ("clockRange", vec![s(a), s(b)], *w)).collect();
    check_calls(&cases);
}

#[test]
fn stamp_of_reads_a_naive_stamp_as_written_and_never_shifts_it() {
    // One case per shape the page receives (review r1, Minor 4). The zone does not move any of them.
    let rows = [("2026-09-01 20:55", "2026-09-01 8:55pm"),      // run and journal-derived stamps
        ("2026-08-26T23:59", "2026-08-26 11:59pm"),             // task `due`, amend from/to
        ("2026-08-26T23:59:00", "2026-08-26 11:59pm"),          // with seconds: a mail due/old/new
        ("2026-09-25 09:00", "2026-09-25 9am"),
        ("2026-08-26", "2026-08-26")];                          // a date only comes back unchanged
    let cases: Vec<_> = rows.iter().map(|(a, w)| ("stampOf", vec![s(a)], *w)).collect();
    check_calls(&cases);
}

#[test]
fn zoned_stamps_convert_to_the_machines_local_time() {
    // `node` runs with TZ=America/Chicago: 14:05Z on 2026-09-01 is 9:05am CDT.
    let z = "2026-09-01T14:05:00.000Z";
    let nine = local_clock(z, "9:05am");
    let two = local_clock("2026-09-01T19:05:00Z", "2:05pm");
    let day = |c: &str| format!("2026-09-01 {c}");
    check_calls(&[
        ("stampOf", vec![s(z)], &day(&nine)),                    // the drawer history's `journal::now_ts` shape
        ("timeOf", vec![s(z)], &nine),                           // the sync line's `last_slot`
        ("stampOf", vec![s("2026-09-01T14:05:00-05:00")], &day(&two)), // an offset form
        ("timeOf", vec![s("2026-09-01T14:05:00-05:00")], &two),
    ]);
}

#[test]
fn empty_null_and_unparseable_input_gives_nothing_from_time_of_and_itself_from_stamp_of() {
    check_calls(&[
        ("timeOf", vec![s("")], ""), ("timeOf", vec![Value::Null], ""), ("timeOf", vec![s("not a time")], ""),
        ("stampOf", vec![s("")], ""), ("stampOf", vec![Value::Null], ""), ("stampOf", vec![s("not a time")], "not a time"),
    ]);
}

/// The rows `paint` never reaches (plan T4, Layer 1's last table): each is a direct call of the
/// helper it uses, on that row's input shape.
#[test]
fn the_rows_paint_never_reaches_are_covered_by_direct_helper_calls() {
    let z = "2026-09-01T14:05:00.000Z";
    let nine = local_clock(z, "9:05am");
    let day = format!("2026-09-01 {nine}");
    check_calls(&[
        ("timeOf", vec![s(z)], &nine),                                       // update :422, :474; sync toast :588
        ("clockRange", vec![s("13:00"), s("14:15")], "1–2:15pm"),            // registrar regMeets :1057
        ("stampOf", vec![s(z)], &day),                                       // the drawer history :1659
        ("stampOf", vec![s("2026-08-26T23:59:00")], "2026-08-26 11:59pm"),   // mail applied :2803, :2806, :2808
        ("stampOf", vec![s("2026-08-26T23:59")], "2026-08-26 11:59pm"),
    ]);
}

// ---- Layer 1: the render scan over `paint` ----

/// The three views the scan paints, in turn. `paint` is the only renderer called.
const VIEWS: [&str; 3] = ["today", "runs", "decisions"];

/// Every recorded write of `paint(state, true)` on `view`, joined per element id. Attribute values
/// (`title`) are an allowed region and are left out.
fn paint_view(state: &Value, view: &str) -> Vec<(String, String)> {
    let out = node(&json!({ "mode": "paint", "state": state, "view": view }));
    out["log"].as_array().expect("log").iter()
        .filter(|w| w["prop"] == "innerHTML" || w["prop"] == "textContent")
        .map(|w| (w["id"].as_str().unwrap().to_string(), w["value"].as_str().unwrap().to_string())).collect()
}

/// element id -> the markup written to it across all three views.
fn paint_all(state: &Value) -> std::collections::BTreeMap<String, String> {
    let mut by_id = std::collections::BTreeMap::new();
    for v in VIEWS {
        for (id, html) in paint_view(state, v) { by_id.entry(id).or_insert_with(String::new).push_str(&format!("{html}\n")); }
    }
    by_id
}

/// The supplement: only rows `paint` reaches, each with its own known string (plan T4, Layer 1's table).
/// Merged into the full reference: `decisions.cards` and `must_do.amendments` are appended to; the
/// closed list and the runs panel's two lists are replaced.
fn full_with_supplement() -> Value {
    let mut st = fixture("surface-today-full.json");
    let z = "2026-09-01T14:05:00.000Z";
    st["topline"]["generated_at"] = s("2026-09-01 20:55");
    st["topline"]["generated_by"] = s("manual");
    st["topline"]["last_slot"] = json!({ "ended": z, "ok": true });
    st["topline"]["sync"] = json!({ "at": z });
    st["closed_this_week"] = json!([
        { "mark": "me", "title": "Closed one", "when": "2026-09-01 20:55", "id": "t1", "undo": false },
        { "mark": "me", "title": "Closed two", "when": "2026-08-31 09:30", "id": "t2", "undo": false }]);
    st["must_do"]["amendments"].as_array_mut().unwrap().push(json!({ "age_days": 0, "due_move": "10/9 11:59pm → 10/9 1pm (EARLIER same day)",
        "proposal_path": "approvals/amend-clock12.md", "stale": false, "target_title": "Clock test task", "urgency": "increases" }));
    st["decisions"]["cards"].as_array_mut().unwrap().push(json!({ "id": "appr_clock12_importance", "kind": "amend", "age_days": 0,
        "title": "Clock test card", "why": "**Why proposed:** test.", "changes": { "importance": 4 }, "slug": "amend-clock12-importance",
        "target": null, "urgency": null, "expires": null, "snooze_until": null, "source_uid": null, "first_proposed_at": "2026-08-28" }));
    st["runs_panel"]["expected"] = json!([{ "due": "2026-08-27 15:00", "runner": "cloud", "status": "late" }]);
    st["runs_panel"]["recent"] = json!([{ "ended": "2026-08-27 15:02", "started": "2026-08-27 15:01", "result": "ok", "runner": "cloud",
        "run_id": "rank-2026-08-27T20:00:00Z", "summary": "ok" }]);
    st
}

/// Strings that stay as the engine or the journal wrote them: delta `text`, run ids, run summaries.
fn allowed_strings(state: &Value) -> Vec<String> {
    fn texts(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => for (k, x) in m { if k == "text" { if let Some(t) = x.as_str() { out.push(t.to_string()); } } texts(x, out); },
            Value::Array(a) => a.iter().for_each(|x| texts(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    texts(&state["delta"], &mut out);
    for r in state["runs_panel"]["recent"].as_array().into_iter().flatten() {
        for k in ["run_id", "summary"] { if let Some(t) = r[k].as_str() { out.push(t.to_string()); } }
    }
    out.retain(|t| !t.is_empty());
    out
}

/// What the student reads: `<pre class="summary">` contents and tags gone, entities decoded, the
/// allowed strings removed, whitespace collapsed.
fn plain(html: &str, allowed: &[String]) -> String {
    let pre = Regex::new(r#"(?s)<pre class="summary">.*?</pre>"#).unwrap();
    let tags = Regex::new(r"(?s)<[^>]*>").unwrap();
    let mut t = tags.replace_all(&pre.replace_all(html, " "), " ").to_string();
    for (e, c) in [("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&middot;", "·"), ("&rarr;", "→"), ("&hellip;", "…"),
        ("&times;", "×"), ("&nbsp;", " "), ("&amp;", "&")] { t = t.replace(e, c); }
    for a in allowed { t = t.replace(a.as_str(), " "); }
    Regex::new(r"\s+").unwrap().replace_all(&t, " ").to_string()
}

fn around(text: &str, at: usize, to: usize) -> String {
    let a = text[..at].char_indices().rev().nth(20).map_or(0, |(i, _)| i);
    let b = text[to..].char_indices().nth(20).map_or(text.len(), |(i, _)| to + i);
    text[a..b].to_string()
}

/// A 24-hour form, an ISO `T`, or a 12-hour clock not followed by am/pm (or by a dash and a time that is).
fn clock_faults(text: &str) -> Vec<String> {
    let h24 = Regex::new(r"\b(?:0\d|1[3-9]|2[0-3]):[0-5]\d\b").unwrap();
    let iso = Regex::new(r"T\d\d:\d\d").unwrap();
    let twelve = Regex::new(r"\b(?:1[0-2]|[1-9]):[0-5]\d\b").unwrap();
    let tail = Regex::new(r"^–\d{1,2}(?::[0-5]\d)?(?:am|pm)").unwrap();
    let mut bad = Vec::new();
    for m in h24.find_iter(text) { bad.push(format!("24-hour form `{}` in `…{}…`", m.as_str(), around(text, m.start(), m.end()))); }
    for m in iso.find_iter(text) { bad.push(format!("ISO form `{}` in `…{}…`", m.as_str(), around(text, m.start(), m.end()))); }
    for m in twelve.find_iter(text) {
        let rest = &text[m.end()..];
        if !(rest.starts_with("am") || rest.starts_with("pm") || tail.is_match(rest)) {
            bad.push(format!("12-hour clock without am/pm `{}` in `…{}…`", m.as_str(), around(text, m.start(), m.end())));
        }
    }
    bad
}

/// `(element id, known string)`: each must be in that element's text.
fn assert_scan(state: &Value, known: &[(&str, String)]) {
    let by_id = paint_all(state);
    let allowed = allowed_strings(state);
    let mut bad = Vec::new();
    for (id, html) in &by_id {
        // The page repaints an element on every view, so the same fault repeats; say it once.
        for f in clock_faults(&plain(html, &allowed)) { let line = format!("#{id}: {f}"); if !bad.contains(&line) { bad.push(line); } }
    }
    for (id, want) in known {
        let text = by_id.get(*id).map(|h| plain(h, &allowed)).unwrap_or_default();
        if !text.contains(want.as_str()) { bad.push(format!("#{id}: the known string `{want}` is missing")); }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// The harness itself, so that a red scan below is the page's doing and not the harness's: `paint`
/// runs on every reference, reaches every element the scan reads, and the reference's rows arrive.
#[test]
fn the_paint_harness_reaches_every_region_the_scan_reads() {
    let by_id = paint_all(&full_with_supplement());
    for id in ["theday", "cu", "mustdo", "closed", "topline", "syncline", "runs", "runs-view", "dec-list"] {
        assert!(by_id.get(id).is_some_and(|h| !h.trim().is_empty()), "paint wrote nothing to #{id}; wrote to {:?}", by_id.keys().collect::<Vec<_>>());
    }
    assert!(by_id["theday"].contains("class=\"blk "), "the day's blocks are drawn: {}", by_id["theday"]);
    assert!(by_id["dec-list"].contains("appr_clock12_importance"), "the supplement's decision card is drawn");
    assert!(by_id["runs-view"].contains("rank-2026-08-27T20:00:00Z"), "the supplement's run is drawn");
    assert!(by_id["closed"].contains("Closed two"), "the supplement's closed rows are drawn");
}

#[test]
fn render_scan_full_reference_and_supplement_show_a_12_hour_clock_everywhere() {
    let nine = local_clock("2026-09-01T14:05:00.000Z", "9:05am");
    assert_scan(&full_with_supplement(), &[
        ("theday", "8–9am".into()), ("theday", "10am–12pm".into()), ("theday", "12–12:50pm".into()), ("theday", "3–6pm".into()),
        ("cu", "Wed 9/2 4pm".into()),                                   // after T3
        ("mustdo", "2026-08-26 11:59pm".into()),                        // a task's due chip
        ("dec-list", "2026-09-25 9am → 2026-09-22 9am".into()),         // decisions.cards[1]'s due change
        ("topline", "generated 2026-09-01 8:55pm by manual".into()),
        ("syncline", format!("last slot {nine} ok")), ("syncline", format!("synced {nine}")),
        ("closed", "8:55pm".into()), ("closed", "9:30am".into()),
        ("mustdo", "due 10/9 11:59pm → 10/9 1pm (EARLIER same day)".into()),
        ("dec-list", "importance → 4".into()),                          // a key other than `due` stays as today
        ("runs-view", "2026-08-27 3pm".into()), ("runs", "3pm".into()), ("runs", "expected 2026-08-27 3pm".into()),
        ("runs-view", "2026-08-27 3:02pm".into()), ("runs", "3:02pm".into()),
    ]);
}

#[test]
fn render_scan_s1_reference_shows_a_12_hour_clock_in_its_blocks() {
    assert_scan(&fixture("surface-today-s1-migrated.json"), &[
        ("theday", "8am–12pm".into()), ("theday", "1–1:45pm".into()), ("theday", "1:45–6pm".into()),
    ]);
}

// ---- the scanner behind Layers 2 and 3 ----

/// The page's source in two blanked forms of the same byte length (newlines kept, so a position in
/// one is the same line in the other):
/// - `code`: comments blanked; strings, template literals and regex literals kept intact;
/// - `skel`: comments, string contents and regex bodies blanked, for brace matching.
struct Stripped { code: String, skel: String }

/// One walk, a state for each string type (`"`, `'`, template; `\` escapes) and for regex literals. A
/// regex literal is a `/` after `(`, `,`, `=`, `:`, `[`, `!`, `&`, `|`, `?`, `{`, `;` or `return`
/// (or at the start); it runs to the unescaped `/` outside a character class. `//` and `/* */` are
/// comments only in code state.
fn strip(src: &str) -> Stripped {
    let b = src.as_bytes();
    let (mut code, mut skel) = (Vec::with_capacity(b.len()), Vec::with_capacity(b.len()));
    let blank = |c: u8| if c == b'\n' { b'\n' } else { b' ' };
    let (mut i, mut prev) = (0usize, b';');
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied().unwrap_or(0);
        if c == b'"' || c == b'\'' || c == b'`' {
            code.push(c); skel.push(c); i += 1;
            while i < b.len() {
                let d = b[i];
                if d == b'\\' { for k in i..(i + 2).min(b.len()) { code.push(b[k]); skel.push(blank(b[k])); } i += 2; continue; }
                code.push(d); i += 1;
                if d == c { skel.push(d); break; }
                skel.push(blank(d));
            }
            prev = c;
        } else if c == b'/' && next == b'/' {
            while i < b.len() && b[i] != b'\n' { code.push(b' '); skel.push(b' '); i += 1; }
        } else if c == b'/' && next == b'*' {
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) { code.push(blank(b[i])); skel.push(blank(b[i])); i += 1; }
            for _ in 0..2 { if i < b.len() { code.push(b' '); skel.push(b' '); i += 1; } }
        } else if c == b'/' && (b"(,=:[!&|?{;".contains(&prev) || ends_with_return(&code)) {
            code.push(c); skel.push(c); i += 1;
            let mut in_class = false;
            while i < b.len() && b[i] != b'\n' {
                let d = b[i];
                if d == b'\\' { for k in i..(i + 2).min(b.len()) { code.push(b[k]); skel.push(blank(b[k])); } i += 2; continue; }
                code.push(d); i += 1;
                if d == b'[' { in_class = true; } else if d == b']' { in_class = false; }
                if d == b'/' && !in_class { skel.push(d); break; }
                skel.push(blank(d));
            }
            prev = b')';
        } else {
            code.push(c); skel.push(c); i += 1;
            if !c.is_ascii_whitespace() { prev = c; }
        }
    }
    Stripped { code: String::from_utf8(code).expect("blanking keeps whole characters"), skel: String::from_utf8(skel).expect("utf-8") }
}

fn ends_with_return(code: &[u8]) -> bool {
    let t: Vec<u8> = code.iter().rev().skip_while(|c| c.is_ascii_whitespace()).take(7).cloned().collect();
    let t: Vec<u8> = t.into_iter().rev().collect();
    t.ends_with(b"return") && (t.len() == 6 || !(t[t.len() - 7].is_ascii_alphanumeric() || t[t.len() - 7] == b'_'))
}

/// The `{ … }` that follows `from` in `skel`, as a byte range in the (same-length) code.
fn braces_after(skel: &str, from: usize) -> Option<(usize, usize)> {
    let open = from + skel[from..].find('{')?;
    let mut depth = 0i32;
    for (i, c) in skel[open..].bytes().enumerate() {
        if c == b'{' { depth += 1; } else if c == b'}' { depth -= 1; if depth == 0 { return Some((open, open + i + 1)); } }
    }
    None
}

fn line_of(code: &str, at: usize) -> usize { code[..at].bytes().filter(|&c| c == b'\n').count() + 1 }

// ---- Layer 3: forbidden forms ----

/// One array lists every form, so a new one is a one-line addition: `(name, regex, a sample that must be a hit)`.
/// The two regex-literal forms are checked further in `forbidden_hits` (the source must hold `T` or `:`).
const FORMS: &[(&str, &str, &str)] = &[
    ("slice with a first argument from 10 to 19", r"\.slice\(\s*1\d\b", "x.slice(11, 16)"),
    ("slice from the end", r"\.slice\(\s*-", "x.slice(-2)"),
    ("slice(0, n) with n from 13 to 19", r"\.slice\(\s*0\s*,\s*1[3-9]\s*\)", "x.slice(0, 16)"),
    ("substr / substring", r"\.(?:substr|substring)\(", "x.substring(11)"),
    ("replace or split on a literal T, space or colon", r#"\.(?:replace|split)\(\s*(?:"T"|'T'|" "|' '|":"|':')"#, r#"x.replace("T", " ")"#),
    ("replace or split on a regex literal holding T or a colon", r"\.(?:replace|split)\(\s*/((?:[^/\\\n]|\\.)+)/[a-z]*", "x.split(/:/)"),
    ("split(\" \")[1]", r#"\.split\(" "\)\[1\]"#, r#"x.split(" ")[1]"#),
    ("a local-time API", r"getHours\(|getMinutes\(|getUTCHours\(|getUTCMinutes\(|toTimeString\(|toLocaleTimeString\(|toLocaleString\(|Intl\.DateTimeFormat", "d.getHours()"),
    ("toISOString().slice", r"toISOString\(\)\.slice", "d.toISOString().slice(0, 10)"),
];

/// `(form, 1-based line, trimmed line)` for every forbidden form in already-stripped `code`.
fn forbidden_hits(code: &str) -> Vec<(&'static str, usize, String)> {
    let mut hits = Vec::new();
    for (name, re, _) in FORMS {
        for m in Regex::new(re).unwrap().captures_iter(code) {
            let whole = m.get(0).unwrap();
            // A regex-literal argument counts only when its source holds `T` or `:` (`/[ (]/` and `h()`'s class stay legal).
            if name.starts_with("replace or split on a regex literal") && !m[1].contains('T') && !m[1].contains(':') { continue; }
            let line = line_of(code, whole.start());
            hits.push((*name, line, code.lines().nth(line - 1).unwrap_or("").trim().chars().take(110).collect()));
        }
    }
    hits.sort_by_key(|h| h.1);
    hits
}

/// `code` with the four helpers' bodies blanked (when the page has them yet).
fn without_helpers(st: &Stripped) -> String {
    let mut code = st.code.clone().into_bytes();
    for name in ["clock12", "clockRange", "timeOf", "stampOf"] {
        for pat in [format!("function {name}("), format!("{name} = function")] {
            if let Some(at) = st.skel.find(&pat) {
                if let Some((a, z)) = braces_after(&st.skel, at) { for c in &mut code[a..z] { if *c != b'\n' { *c = b' '; } } }
            }
        }
    }
    String::from_utf8(code).unwrap()
}

#[test]
fn the_scanner_reads_strings_regexes_and_comments_correctly() {
    let hits = |src: &str| forbidden_hits(&strip(src).code).len();
    // a `//` inside a string is not a comment
    assert_eq!(hits(r#"var u = "https://x.example"; x.slice(11, 16)"#), 1);
    // comments are not hits
    assert_eq!(hits("// x.slice(11, 16)"), 0);
    assert_eq!(hits("/* x.slice(11, 16) */"), 0);
    assert_eq!(hits("a(); /* x.slice(11, 16)\n more */ b();"), 0);
    // a regex literal holding `//`, and a template literal holding `//`, do not start a comment
    assert_eq!(hits(r"/\/\//.test(s); x.slice(11, 16)"), 1);
    assert_eq!(hits("`a//b`; x.slice(11, 16)"), 1);
    assert_eq!(hits("if (a) { y = /[/]\\//.test(s); } x.slice(11, 16)"), 1);
    // division is not a regex
    assert_eq!(hits("var r = a / b / c; x.slice(11, 16)"), 1);
    // every form in Layer 3's array is a hit, and the legal neighbours are not
    for (name, _, sample) in FORMS { assert!(!forbidden_hits(&strip(sample).code).is_empty(), "form `{name}` must hit `{sample}`"); }
    for ok in [r#"x.slice(0, 10)"#, r#"s.replace(/[&<>"']/g, f)"#, "n.split(/[ (]/)", "x.slice(1, 3)", r#"s.split(",")"#] {
        assert_eq!(hits(ok), 0, "`{ok}` stays legal");
    }
    // the helper cut: a body is blanked, the rest of the file is not
    let st = strip("function timeOf(a) { return a.slice(11, 16); }\nfunction other() { return b.slice(11, 16); }");
    assert_eq!(forbidden_hits(&without_helpers(&st)).len(), 1);
}

// ---- Layer 2: positive pins ----

/// Where the required helper calls must be found, from the anchor.
enum Scope {
    /// On the anchor's own source line (T5 must not reflow these lines).
    Line,
    /// In the body of the function the anchor names.
    Body,
    /// In the block from the anchor through the next `.join(" · ")`: a `k === "due"` branch that calls `stampOf(`.
    Changes,
}

/// `(§2.2 row, anchor, scope, required substrings)`. An anchor is the exact substring, quotes and all
/// where the plan shows them, and occurs once in console.js today.
const PINS: &[(&str, &str, Scope, &[&str])] = &[
    (":92", "\"generated \"", Scope::Line, &["stampOf("]),
    (":151", "\"no due date\"", Scope::Line, &["stampOf("]),
    (":272", "<div class=\"blk ", Scope::Line, &["clockRange("]),
    (":396", "EL(\"closed\").innerHTML", Scope::Line, &["timeOf("]),
    (":422", "\", checked \"", Scope::Line, &["timeOf("]),
    (":474", "\"up to date (checked \"", Scope::Line, &["timeOf("]),
    (":588", "showNote(\"synced \"", Scope::Line, &["timeOf("]),
    (":610", "\"tried \"", Scope::Line, &["timeOf("]),
    (":622", "t.last_slot.ended", Scope::Line, &["timeOf("]),
    (":639", "function renderRunsView", Scope::Body, &["stampOf(e.due)"]),
    (":643", "<pre class=\"summary\">", Scope::Line, &["stampOf("]),
    (":674-675", "class=\"changes\"", Scope::Changes, &["k === \"due\"", "stampOf("]),
    (":1057", "function regMeets", Scope::Body, &["clockRange("]),
    (":1286", "\" — expected \"", Scope::Line, &["timeOf(", "stampOf("]),
    (":1287", "r.recent.slice(0, 6)", Scope::Line, &["timeOf("]),
    (":1659", "<div>no journal history</div>", Scope::Line, &["stampOf("]),
    (":2803", "h(x.field) + \": \"", Scope::Line, &["stampOf("]),
    (":2806", "x.opened_at ?", Scope::Line, &["stampOf("]),
    (":2808", "\" · due \"", Scope::Line, &["stampOf("]),
];

/// These raw passthroughs are forbidden anywhere.
const RAW: &[&str] = &["h(e.due)", "h(x.due)", "h(x.opened_at)", "h(t.generated_at)"];

fn page() -> (String, Stripped) {
    let raw = std::fs::read_to_string(Path::new("static").join("console.js")).expect("static/console.js");
    let st = strip(&raw);
    (raw, st)
}

#[test]
fn every_layer_2_anchor_survives_the_stripper_and_occurs_once() {
    let (raw, st) = page();
    let mut bad = Vec::new();
    for (row, anchor, _, _) in PINS {
        let (r, c) = (raw.matches(anchor).count(), st.code.matches(anchor).count());
        if r != 1 { bad.push(format!("{row}: anchor `{anchor}` occurs {r} times in console.js, want exactly 1")); }
        // The lossless guard: a stripper that eats code fails here, loudly.
        if c != r { bad.push(format!("{row}: anchor `{anchor}` is in the source {r} times but {c} in the stripped text (the stripper ate code, or a comment holds it)")); }
    }
    // A stripper that loses track of a string or regex leaves braces unbalanced.
    let depth: i64 = st.skel.bytes().map(|c| match c { b'{' => 1, b'}' => -1, _ => 0 }).sum();
    if depth != 0 { bad.push(format!("the stripped skeleton's braces are off by {depth}")); }
    assert_eq!(st.code.len(), raw.len(), "stripping keeps every byte position");
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// Each pin names its site when the call is missing.
#[test]
fn layer_2_every_site_calls_its_helper_on_its_own_line() {
    let (_, st) = page();
    let mut bad = Vec::new();
    for (row, anchor, scope, required) in PINS {
        let Some(at) = st.code.find(anchor) else { bad.push(format!("{row}: anchor `{anchor}` not found")); continue; };
        let (from, to) = match scope {
            Scope::Line => (st.code[..at].rfind('\n').map_or(0, |i| i + 1), at + st.code[at..].find('\n').unwrap_or(st.code.len() - at)),
            Scope::Body => match braces_after(&st.skel, at) { Some(r) => r, None => { bad.push(format!("{row}: no body after `{anchor}`")); continue; } },
            Scope::Changes => match st.code[at..].find(".join(\" · \")") { Some(i) => (at, at + i), None => { bad.push(format!("{row}: no `.join(\" · \")` after `{anchor}`")); continue; } },
        };
        let seg = &st.code[from..to];
        let line = line_of(&st.code, at);
        for req in *required {
            if !seg.contains(req) { bad.push(format!("{row} (console.js:{line}): `{anchor}` needs `{req}` {}", match scope { Scope::Line => "on the same line", Scope::Body => "in its body", Scope::Changes => "in its block" })); }
        }
        if let Scope::Changes = scope {
            // The `due` branch must reach `stampOf(` before any `JSON.stringify(`; the other keys keep JSON.stringify.
            if let Some(d) = seg.find("k === \"due\"") {
                let branch = &seg[d..];
                let end = branch.find("JSON.stringify(").unwrap_or(branch.len());
                if !branch[..end].contains("stampOf(") { bad.push(format!("{row} (console.js:{line}): the `k === \"due\"` branch reaches JSON.stringify(v) before any stampOf(")); }
            }
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

#[test]
fn layer_2_raw_time_passthroughs_are_gone() {
    let (_, st) = page();
    let mut bad = Vec::new();
    for raw in RAW { for (at, _) in st.code.match_indices(raw) { bad.push(format!("console.js:{}: raw passthrough `{raw}`", line_of(&st.code, at))); } }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

// ---- Layer 3: forbidden forms, outside the four helpers ----

#[test]
fn layer_3_no_hand_rolled_time_slicing_outside_the_helpers() {
    let (_, st) = page();
    let bad: Vec<String> = forbidden_hits(&without_helpers(&st)).iter().map(|(form, line, text)| format!("console.js:{line}: {form}: {text}")).collect();
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// The helpers are in the page, each as one function, so Layer 3 has something to cut out.
#[test]
fn the_four_helpers_are_defined_in_the_page() {
    let (_, st) = page();
    let missing: Vec<_> = ["clock12", "clockRange", "timeOf", "stampOf"].into_iter()
        .filter(|n| !st.code.contains(&format!("function {n}(")) && !st.code.contains(&format!("{n} = function"))).collect();
    assert!(missing.is_empty(), "console.js defines no {missing:?} yet; `localHM` is still the page's only time helper");
}
