//! C10 (onboarding replan §6.6, §6.7, §8): the first-day screen and Schedule treat a `classsearch:` row as
//! its own source. Written before `app/static/console.js` changes; every pin runs the page's own functions
//! under `node` over a small recording DOM, and none string-matches the source.
//!
//! The rules pinned here:
//! - `fromClassSearch(p)` mirrors `fromRegistrar(p)`; the two never both hold.
//! - `setupRow` presets no `classsearch:` row (RQ2: the student presses it); a class, lab or work row
//!   from any other source keeps the Mine preset.
//! - No `classsearch:` row wears the registrar's name, and it has a label of its own ("class search").
//! - Schedule's registrar list and the confirm screen's "added" list never list one.
//! - The review's case: two class-search rows left blank, `setup` true, Finish pressed, neither written.
//!
//! All data is invented.

use std::path::Path;
use std::process::Command;

const HARNESS: &str = r#"
const fs = require("fs"), vm = require("vm");
process.on("unhandledRejection", () => {});
const stub = () => new Proxy(function () {}, { get: (t, k) => (k === Symbol.toPrimitive ? () => "" : stub()), apply: () => stub(), construct: () => stub(), set: () => true });
const els = {}, calls = [];
const el = (id, store) => new Proxy(function () {}, {
  get: (t, k) => (k in store ? store[k] : (k === Symbol.toPrimitive ? () => "" : stub())),
  set: (t, k, v) => { store[k] = v; return true; }, apply: () => stub(), construct: () => stub() });
const rowsOf = () => {
  const out = [];
  for (const id of ["ws-class-rows", "ws-week-rows", "ws-oh-rows"]) {
    const html = (els[id] && els[id].store.innerHTML) || "";
    const re = /<div class="wsrow" data-key="([^"]*)" data-answer="([^"]*)" data-level="([^"]*)"/g; let m;
    while ((m = re.exec(html))) { const a = { "data-key": m[1], "data-answer": m[2], "data-level": m[3] }; out.push({ getAttribute: (n) => a[n] }); }
  }
  return out;
};
const getEl = (id) => {
  if (!els[id]) {
    const store = { innerHTML: "", hidden: false, textContent: "" };
    if (id === "ws-window") { store.querySelector = () => ({ querySelector: (p) => ({ value: p.indexOf("start") >= 0 ? "08:00" : "22:00" }) }); }
    els[id] = { store, proxy: el(id, store) };
  }
  return els[id].proxy;
};
const doc = new Proxy({}, { get: (t, k) => k === "getElementById" ? getEl : k === "querySelectorAll" ? ((sel) => (sel.indexOf(".wsrow[data-key]") >= 0 ? rowsOf() : [])) : stub(), set: () => true });
const g = { document: doc, location: { search: "", hash: "", href: "http://localhost/", pathname: "/", origin: "http://localhost" },
  URLSearchParams, URL, console, Date, JSON, Math, navigator: { userAgent: "node", language: "en" },
  localStorage: { getItem: () => null, setItem() {}, removeItem() {} },
  requestAnimationFrame: () => 0, setTimeout: () => 0, setInterval: () => 0, clearTimeout() {}, clearInterval() {},
  addEventListener() {}, removeEventListener() {}, fetch: () => { throw new Error("tests never leave the machine"); },
  __TAURI__: { core: { invoke: (cmd, args) => { calls.push({ cmd, args }); return Promise.resolve({ ok: true, error: null, result: { warnings: [] } }); } } } };
g.window = g;
const ctx = vm.createContext(g);
let src = fs.readFileSync(process.argv[1], "utf8");
const at = src.lastIndexOf("})();");
if (at < 0) { console.log(JSON.stringify({ __nowrapper: true })); process.exit(0); }
const lift = ["fromRegistrar", "fromClassSearch", "setupRow", "paintSetupRows", "renderRegistrar", "paintRegistrarAdded", "finishWeekSetup"]
  .map((n) => "L." + n + " = typeof " + n + " === 'function' ? " + n + " : null;").join("\n");
src = src.slice(0, at) + "\nglobalThis.L = {}; " + lift + "\nL.setLabel = function (v) { registrarLabel = v; };\n" + src.slice(at);
vm.runInContext(src, ctx);
const L = g.L, html = (id) => (els[id] ? els[id].store.innerHTML : "");
const tick = () => new Promise((r) => setImmediate(r));
(async () => {
  // The scenario text is this file's own literal, never input; eval is the harness's way to run it in scope.
  const out = await eval("(async () => {" + process.env.SCENARIO + "})()");
  console.log(JSON.stringify(out === undefined ? null : out));
})().catch((e) => { console.log(JSON.stringify({ __error: String(e && e.stack || e) })); });
"#;

fn run(scenario: &str) -> serde_json::Value {
    let out = Command::new("node").arg("-e").arg(HARNESS).arg(Path::new("static/console.js"))
        .env("SCENARIO", scenario).output().expect("node must be installed");
    assert!(out.status.success(), "node failed: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert!(v.get("__nowrapper").is_none(), "console.js must keep its IIFE wrapper: {v}");
    assert!(v.get("__error").is_none(), "scenario threw: {v}");
    v
}

/// The proposals the pins draw: one invented row per source and kind.
const PROPOSALS: &str = r#"
const P = (key, kind, title) => ({ source_uid: key, kind, title, when: "Mon Wed 10:00-10:50", where: "Hall 1", level: "soft", window: false });
const cs = P("classsearch:ex:202620-10001", "class", "Intro Basketry 101");
const csLab = P("classsearch:ex:202620-10002", "lab", "Intro Basketry 101 lab");
const csWork = P("classsearch:ex:202620-10003", "work", "Basketry shift");
const reg = P("registrar:ex:202620-20001", "class", "Applied Kilns 201");
const goog = P("google:evt-abc123", "class", "Glassblowing 150");
const gLab = P("google:evt-abc124", "lab", "Glassblowing 150 lab");
const gWork = P("google:evt-abc125", "work", "Studio job");
"#;

#[test]
fn from_class_search_mirrors_from_registrar_and_the_two_never_both_hold() {
    let v = run(&format!("{PROPOSALS}
      if (typeof L.fromClassSearch !== 'function') {{ return {{ missing: true }}; }}
      return {{ cs: L.fromClassSearch(cs), reg: L.fromClassSearch(reg), goog: L.fromClassSearch(goog),
        regIsReg: L.fromRegistrar(reg), csIsReg: L.fromRegistrar(cs) }};"));
    assert!(v.get("missing").is_none(), "console.js must define `function fromClassSearch(p)` beside fromRegistrar");
    assert_eq!(v["cs"], true, "a classsearch: row is from class search: {v}");
    assert_eq!(v["reg"], false, "a registrar: row is not: {v}");
    assert_eq!(v["goog"], false, "{v}");
    assert_eq!(v["regIsReg"], true, "fromRegistrar still holds for registrar: {v}");
    assert_eq!(v["csIsReg"], false, "fromRegistrar never holds for classsearch: {v}");
}

#[test]
fn setup_row_presets_no_class_search_row_but_keeps_the_preset_for_every_other_source() {
    let v = run(&format!("{PROPOSALS}
      const ans = (p) => {{ const m = /data-answer=\"([^\"]*)\"/.exec(L.setupRow(p)); return m && m[1]; }};
      const pressed = (p) => /data-answer-set=\"mine\" aria-pressed=\"(\\w+)\"/.exec(L.setupRow(p))[1];
      return {{ cs: ans(cs), csLab: ans(csLab), csWork: ans(csWork), csPressed: pressed(cs),
        reg: ans(reg), goog: ans(goog), gLab: ans(gLab), gWork: ans(gWork), gPressed: pressed(goog) }};"));
    for k in ["cs", "csLab", "csWork"] {
        assert_eq!(v[k], "", "{k}: a class-search row is never preset to Mine (RQ2): {v}");
    }
    assert_eq!(v["csPressed"], "false", "and its Mine button is not pressed: {v}");
    for k in ["reg", "goog", "gLab", "gWork"] {
        assert_eq!(v[k], "mine", "{k}: every other class, lab or work row keeps today's preset: {v}");
    }
    assert_eq!(v["gPressed"], "true", "{v}");
}

#[test]
fn a_class_search_row_never_wears_the_registrars_name_and_has_its_own_label() {
    let v = run(&format!("{PROPOSALS}
      L.setLabel('Example State University');
      const withReg = L.setupRow(cs), regRow = L.setupRow(reg);
      L.setLabel(null);
      const noReg = L.setupRow(cs);
      return {{ withReg, regRow, noReg }};"));
    let has_label = |h: &str| { let l = h.to_lowercase(); l.contains("class=\"src\"") && l.contains("class search") };
    let w = v["withReg"].as_str().unwrap();
    assert!(!w.contains("Example State University"), "the registrar's name is never on a class-search row: {w}");
    assert!(has_label(w), "it carries its own `src` label that says class search: {w}");
    let n = v["noReg"].as_str().unwrap();
    assert!(has_label(n), "the label does not depend on the school having a registrar: {n}");
    let r = v["regRow"].as_str().unwrap();
    assert!(r.contains("from Example State University") && !r.to_lowercase().contains("class search"), "a registrar row is unchanged: {r}");
}

#[test]
fn schedules_registrar_list_never_lists_a_class_search_row() {
    let v = run(&format!("{PROPOSALS}
      L.setLabel('Example State University');
      L.renderRegistrar({{ registrar: {{ held: [], refresh: false }}, registrar_proposals: [reg, cs], registrar_changes: null, commitments: [] }});
      return html('sched-reg-rows');"));
    let h = v.as_str().unwrap();
    assert!(h.contains("Applied Kilns 201") && h.contains("data-reg-add=\"registrar:ex:202620-20001\""), "the registrar's own row is listed: {h}");
    assert!(!h.contains("Intro Basketry 101") && !h.contains("classsearch:"), "a class-search row is not, so it never wears the registrar's name: {h}");
}

#[test]
fn the_added_list_names_only_registrar_notes_and_never_a_class_search_note() {
    let v = run(&format!("{PROPOSALS}
      L.setLabel('Example State University');
      const note = (key, title) => ({{ source_uid: key, title, when: 'Mon 10:00-10:50' }});
      L.paintRegistrarAdded([note('registrar:ex:202620-20001', 'Applied Kilns 201'), note('classsearch:ex:202620-10001', 'Intro Basketry 101')], null);
      const both = html('ws-reg-added');
      L.paintRegistrarAdded([note('classsearch:ex:202620-10001', 'Intro Basketry 101')], null);
      return {{ both, only: html('ws-reg-added') }};"));
    let both = v["both"].as_str().unwrap();
    assert!(both.contains("Applied Kilns 201") && !both.contains("Intro Basketry 101"), "{both}");
    assert_eq!(v["only"], "", "with only a class-search note there is nothing to list under the registrar's name: {v}");
}

// The review's case (replan §8 C10): step 10 confirmed with two rows blank, no planning-day note, `setup`
// true. The first-day screen paints them, the student presses Finish, and neither is written.
#[test]
fn finish_on_the_first_day_screen_writes_neither_blank_class_search_row() {
    let v = run(&format!("{PROPOSALS}
      L.setLabel('Example State University');
      L.paintSetupRows([cs, csLab, goog, reg], []);
      await L.finishWeekSetup();
      await tick(); await tick();
      const c = calls.filter((x) => x.cmd === 'commitments_confirm');
      return {{ n: c.length, payload: c.length ? c[0].args.confirm : null }};"));
    assert_eq!(v["n"], 1, "Finish presses commitments_confirm once: {v}");
    let p = &v["payload"];
    let keys = |f: &str| p[f].as_array().unwrap().iter().map(|r| r["source_uid"].as_str().unwrap_or_else(|| r.as_str().unwrap()).to_string()).collect::<Vec<_>>();
    let (mine, not) = (keys("mine"), keys("not_mine"));
    assert!(!mine.iter().chain(not.iter()).any(|k| k.starts_with("classsearch:")),
        "a blank class-search row is neither Mine nor Not mine, so Finish writes nothing for it: mine={mine:?} not={not:?}");
    assert_eq!(mine, ["google:evt-abc123", "registrar:ex:202620-20001"], "the other sources keep today's preset, so Finish still writes them");
}
