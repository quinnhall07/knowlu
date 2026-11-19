"""T7's DOM check: Home's main pane (v0.4.0 note §4 Home, plan T7), through T5c's harness pieces.

    python scripts/ui-fixture/checks/t07.py           # every t07 set, 820x600 and 1280x800, light and dark, both text sizes
    python scripts/ui-fixture/checks/t07.py --make    # rebuild envelopes/t07-*.json from the engine first (-j 2 build)
    python scripts/ui-fixture/checks/t07.py --no-build --make   # rebuild with the engine already in target/debug

The envelopes come from the engine, never by hand: --make builds invented vaults with make_base_courses.py's own
functions (its base vault, plus the changes below), runs `knowlu-engine surface` with the oracle's pins and wraps each
answer as the app's `state` does. `first-run` and `first-run-ended` change only what the app adds (the envelope's
`first_run` block), as base-first-run does.

| Set | The vault |
|---|---|
| t07-courses | the base, two more invented courses (one with work far out: Ahead; one with none: no word) and a pending amendment on a Do today task |
| t07-overload | the base and make_base_courses' overload change |
| t07-unreadable | the base and make_base_courses' unreadable note |
| t07-clear | the base with its three Do today tasks done: Do today is empty |
| t07-first-run | the app's first_run block, running, with a failed account check |
| t07-first-run-ended | the app's first_run block, ended with a failed step |

For every set x size x theme x text size: no console or page error, no horizontal scroll, only #center scrolls,
then the set's own assertions (below, in CHECKS). Shots go to target/ui-shots/t07-<set>-<w>x<h>-<theme>-<text>.png.
Exit 1 on any failure.
"""
import argparse
import json
import pathlib
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIX = HERE.parent
sys.path.insert(0, str(FIX))
import make_base_courses as mb  # noqa: E402
import serve  # noqa: E402

REPO = FIX.parent.parent
OUT = FIX / "envelopes"
SHOTS = REPO / "target" / "ui-shots"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]
TAG = "t07"

# ---- the envelopes ------------------------------------------------------------------------------

EXTRA_COURSES = [
    ("mno-130", "MNO 130", "Studio Practice", None, "MNO 130"),
    ("pqr-140", "PQR 140", "Field Notes", None, "PQR 140"),
]
EXTRA_TASKS = [
    ("mno-130-portfolio-review", "task_e7e7e7e701", "MNO 130 Portfolio review", "mno-130", "2026-09-25T17:00", 2.0, 3, "active", 0),
]
AMEND = "\n".join([
    "---", "type: approval", "kind: amend", "title: " + mb.yaml_text("DEF 210 Lab write-up moved"), "status: pending",
    "target: tasks/def-210-lab-write-up.md", "proposed_at: 2026-08-28", "expires: 2026-09-20", "snooze_until: null",
    "created_by: fixture", "changes:", "  due:", "    from: 2026-08-29T17:00", "    to: 2026-08-31T17:00",
    "id: appr_e7e7e7e702", "first_proposed_at: 2026-08-28", "---", "", "Invented for the fixture harness.", "",
])
DONE_TODAY = ["abc-101-problem-set-2", "abc-101-reading-quiz", "def-210-lab-write-up"]
FIRST_RUN = {"running": True, "current": "ingest", "steps": [["sync", 0], ["entitlement (refresh failed: offline)", 0], ["coursework", 0]]}
FIRST_RUN_ENDED = {"running": False, "current": None, "steps": [["sync", 0], ["coursework", 2], ["engine: not found", -1]]}


def courses(root):
    for n, (slug, title, name, status, code) in enumerate(EXTRA_COURSES, start=len(mb.COURSES) + 1):
        mb.write(root / "courses" / (slug + ".md"), mb.course_note(n, slug, title, name, status, code))
    for stem, tid, title, course, due, effort, imp, status, progress in EXTRA_TASKS:
        mb.write(root / "tasks" / (stem + ".md"), mb.task_note(tid, title, course, due, effort, imp, status, progress))
    mb.write(root / "approvals" / "amend-def-210-lab-write-up.md", AMEND)


def clear(root):
    for stem, tid, title, course, due, effort, imp, status, progress in mb.TASKS:
        if stem in DONE_TODAY:
            mb.write(root / "tasks" / (stem + ".md"), mb.task_note(tid, title, course, due, effort, imp, "done", 100))


VARIANTS = [("courses", courses), ("overload", mb.overload), ("unreadable", mb.unreadable), ("clear", clear)]


def make(build):
    if build:
        import subprocess
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    written = []
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-t07-") as tmp:
        for name, change in VARIANTS:
            vault = pathlib.Path(tmp) / name
            mb.base_vault(vault)
            change(vault)
            for view in ["today", "all"]:
                env = mb.envelope(mb.surface(vault, view))
                if tmp in json.dumps(env) or tmp.replace("\\", "/") in json.dumps(env):
                    sys.exit("the engine's answer names the temporary vault")
                path = OUT / ("%s-%s-%s.json" % (TAG, name, view))
                mb.dump(path, env)
                written.append(path)
    for name, block in [("first-run", FIRST_RUN), ("first-run-ended", FIRST_RUN_ENDED)]:
        for view in ["today", "all"]:
            path = OUT / ("%s-%s-%s.json" % (TAG, name, view))
            mb.dump(path, {"ok": True, "error": None, "state": None, "first_run": block})
            written.append(path)
    banned = mb.fixture_slugs()
    for path in written:
        text = path.read_text(encoding="utf-8")
        for slug in banned:
            if ('"%s"' % slug) in text:
                sys.exit("%s carries the fixture vault's course slug %s" % (path.name, slug))
        print(path.relative_to(REPO).as_posix())


# ---- the checks ---------------------------------------------------------------------------------

PROBE = """() => {
  const se = document.scrollingElement, center = document.getElementById('center');
  return {
    hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
    vscroll: se.scrollHeight > window.innerHeight,
    centerOverflowX: center.scrollWidth > center.clientWidth,
    text: center.innerText
  };
}"""

HOME = """() => {
  const q = (s) => Array.from(document.querySelectorAll(s));
  const vis = (el) => !!el && !el.closest('[hidden]') && el.getClientRects().length > 0;
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  const lead = document.querySelector('#hm-day .hm-field');
  return {
    day: vis(document.getElementById('hm-day')),
    first: vis(document.getElementById('hm-first')),
    doms: q('#hm-doms .hm-dom').map(b => ({ slug: b.getAttribute('data-dom'), code: txt(b.querySelector('b')), word: txt(b.querySelector('span')), pressed: b.getAttribute('aria-pressed'), slot: (b.className.match(/\\bc\\d+\\b/) || [''])[0] })),
    lead: lead ? { id: lead.getAttribute('data-id'), clear: lead.classList.contains('is-clear'), name: txt(lead.querySelector('.hm-name')), section: lead.closest('section') ? lead.closest('section').id : '', slot: (lead.className.match(/\\bc\\d+\\b/) || [''])[0], lock: lead.querySelector('.hm-lock') ? lead.querySelector('.hm-lock').getAttribute('href') : null } : null,
    today: q('#hm-sec-today .hm-row').map(r => ({ id: r.getAttribute('data-id'), slot: (r.className.match(/\\bc\\d+\\b/) || [''])[0], name: txt(r.querySelector('.hm-nm')) })),
    rec: q('#hm-sec-rec .hm-row').map(r => ({ id: r.getAttribute('data-id'), slot: (r.className.match(/\\bc\\d+\\b/) || [''])[0] })),
    sumA: txt(document.querySelector('#hm-sec-today .hm-sum-a')),
    sumB: txt(document.querySelector('#hm-sec-today .hm-sum-b')),
    over: vis(document.querySelector('.hm-over')) ? txt(document.querySelector('.hm-over')) : null,
    unread: vis(document.querySelector('#hm-sec-today .hm-unread')) ? txt(document.querySelector('#hm-sec-today .hm-unread')) : null,
    amend: q('#hm-day .hm-amend').map(txt),
    steps: q('#hm-steps .hm-step').map(s => ({ state: s.getAttribute('data-state'), say: txt(s.querySelector('.hm-step-say')), note: txt(s.querySelector('.hm-step-note')) })),
    firstEnd: vis(document.getElementById('hm-first-end')),
    schedRows: q('#hm-day .ha-sc').length,
    calls: window.__fixture.calls.map(c => c.cmd)
  };
}"""


def courses_check(page, env, fails, deep):
    st = env["state"]
    p = page.evaluate(HOME)
    need = fails.append
    active = [c for c in st["courses"] if str(c.get("status") or "").strip() in ("", "active")]
    words = {"behind": "Behind", "tight": "On track", "steady": "On track", "ahead": "Ahead"}
    want = [(c["slug"], c["title"], words.get(c["standing"], ""), c["slot"]) for c in active]
    got = [(d["slug"], d["code"], d["word"], d["slot"]) for d in p["doms"]]
    if got != want:
        need("domain row %s, want %s" % (got, want))
    if not p["day"] or p["first"]:
        need("the day is not showing")
    rows = [r for g in st["must_do"]["groups"] for r in g["rows"]]
    if not p["lead"] or p["lead"]["id"] != rows[0]["id"] or p["lead"]["section"] != "hm-sec-today":
        need("the lead is %s, want %s on Do today" % (p["lead"], rows[0]["id"]))
    elif p["lead"]["lock"] != "#lockin/" + rows[0]["id"]:
        need("Lock in goes to %s" % p["lead"]["lock"])
    if [r["id"] for r in p["today"]] != [r["id"] for r in rows[1:]]:
        need("Do today rows %s" % p["today"])
    rec = [x["row"]["id"] for x in st["recommended"]["rows"]]
    if [r["id"] for r in p["rec"]] != rec:
        need("Recommended rows %s, want %s" % (p["rec"], rec))
    if p["sumA"] != "%d left" % len(rows):
        need("totals read %r" % p["sumA"])
    text = page.evaluate(PROBE)["text"]
    for banned in ["Start by", "start by", "Won't fit", "won't fit", "slack", "Slack"]:
        if banned in text:
            need("Home shows %r" % banned)
    if not p["amend"]:
        need("the pending amendment's marker is not on its row")
    if not deep:
        return
    # the domain row filters Do today and Recommended, never the schedule or free time; a second press clears it
    before = p["schedRows"]
    slug = active[0]["slug"]
    page.click('#hm-doms .hm-dom[data-dom="%s"]' % slug)
    page.wait_for_timeout(150)
    q = page.evaluate(HOME)
    slot = active[0]["slot"]
    pressed = [d["slug"] for d in q["doms"] if d["pressed"] == "true"]
    if pressed != [slug]:
        need("after a press the pressed classes are %s" % pressed)
    shown = ([q["lead"]] if q["lead"] and not q["lead"]["clear"] else []) + q["today"] + q["rec"]
    if not shown or any(r["slot"] != slot for r in shown):
        need("filtered to %s, the lists show %s" % (slug, shown))
    if q["schedRows"] != before:
        need("the filter changed the schedule (%d rows, was %d)" % (q["schedRows"], before))
    page.click('#hm-doms .hm-dom[data-dom="%s"]' % slug)
    page.wait_for_timeout(150)
    q = page.evaluate(HOME)
    if any(d["pressed"] == "true" for d in q["doms"]) or len(q["today"]) != len(rows) - 1:
        need("a second press did not clear the filter")
    # a done check fills with the row's class colour (A8)
    # (the fill has a short transition: read it once it has settled)
    page.evaluate("() => document.querySelector('#hm-sec-today .hm-row .ck').setAttribute('aria-pressed', 'true')")
    page.wait_for_timeout(700)
    fill = page.evaluate("""() => {
      const b = document.querySelector('#hm-sec-today .hm-row .ck');
      const c = getComputedStyle(b.querySelector('circle')).fill, cc = getComputedStyle(b).getPropertyValue('--cc').trim();
      const probe = document.createElement('i'); probe.style.color = cc; document.body.appendChild(probe);
      const want = getComputedStyle(probe).color; probe.remove(); b.setAttribute('aria-pressed', 'false');
      return { fill: c, want: want };
    }""")
    page.wait_for_timeout(400)
    if fill["fill"] != fill["want"]:
        need("a done check fills %s, its class colour is %s" % (fill["fill"], fill["want"]))
    # the check writes progress 100 with done, and Undo puts it back
    target = rows[1]["id"]
    page.click('#hm-sec-today .hm-row[data-id="%s"] .ck' % target)
    page.wait_for_selector("#toasts .toast .tbtn", timeout=3000)
    calls = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'set_fields').map(c => c.args)")
    if not calls or calls[-1] != {"id": target, "fields": {"progress": 100, "status": "done"}, "view": "today"}:
        need("the check sent %s" % calls)
    page.click("#toasts .toast .tbtn")
    page.wait_for_timeout(200)
    calls = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'set_fields').map(c => c.args)")
    if len(calls) < 2 or calls[-1] != {"id": target, "fields": {"progress": 0, "status": "active"}, "view": "today"}:
        need("Undo sent %s" % calls[-1:])


def overload_check(page, env, fails, deep):
    st = env["state"]
    p = page.evaluate(HOME)
    if not p["over"] or not p["over"].startswith("Over by "):
        fails.append("no over-by figure in the totals (%r)" % p["over"])
        return
    if not deep:
        return
    page.click(".hm-over")
    page.wait_for_selector("#panel[data-open]", timeout=3000)
    body = page.evaluate("() => document.getElementById('panel').innerText")
    items = st["meter"]["spill_items"]
    if not all(i["title"] in body for i in items):
        fails.append("the over-by panel does not list every spill item")
    if "exceeds today" not in body:
        fails.append("the over-by panel does not carry the engine's overflow sentence")


def unreadable_check(page, env, fails, deep):
    p = page.evaluate(HOME)
    bad = env["state"]["unreadable"]
    if not p["unread"] or "1 task note can't be read" not in p["unread"] or bad[0] not in p["unread"]:
        fails.append("the unreadable line reads %r" % p["unread"])


def clear_check(page, env, fails, deep):
    st = env["state"]
    p = page.evaluate(HOME)
    if p["today"]:
        fails.append("Do today shows rows %s" % p["today"])
    if "Clear through" not in p["sumA"] + " " + p["sumB"] and "Clear for two weeks" not in p["sumA"] + " " + p["sumB"]:
        fails.append("no clear line in Do today's totals (%r, %r)" % (p["sumA"], p["sumB"]))
    rec = st["recommended"]["rows"]
    if rec and (not p["lead"] or p["lead"]["section"] != "hm-sec-rec" or p["lead"]["id"] != rec[0]["row"]["id"]):
        fails.append("with Do today done the lead is %s, want the first Recommended row" % p["lead"])


def first_run_check(page, env, fails, deep):
    p = page.evaluate(HOME)
    if p["day"] or not p["first"]:
        fails.append("the first-run view does not replace the day")
    text = page.evaluate(PROBE)["text"]
    if "Setting up your day" not in text:
        fails.append("no 'Setting up your day'")
    want = [("done", "Syncing with your account", ""), ("failed", "Checking your account", "couldn't check — will retry"),
            ("done", "Fetching your coursework", ""), ("now", "Reading your school calendar", "")]
    got = [(s["state"], s["say"], s["note"]) for s in p["steps"]]
    if got != want:
        fails.append("first-run steps %s" % got)
    if p["firstEnd"]:
        fails.append("the didn't-finish line shows while the run is going")


def first_run_ended_check(page, env, fails, deep):
    p = page.evaluate(HOME)
    if not p["firstEnd"]:
        fails.append("the didn't-finish line is hidden after a failed first run")
    if [s["state"] for s in p["steps"]] != ["done", "failed"]:
        fails.append("first-run steps %s" % p["steps"])


CHECKS = {
    "t07-courses": courses_check,
    "t07-overload": overload_check,
    "t07-unreadable": unreadable_check,
    "t07-clear": clear_check,
    "t07-first-run": first_run_check,
    "t07-first-run-ended": first_run_ended_check,
}


def check_one(browser, base, name, size, theme, text):
    fails = []
    errors = []
    env = json.loads((OUT / (name + "-today.json")).read_text(encoding="utf-8"))
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
    page = ctx.new_page()
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    shot = None
    try:
        page.goto("%sindex.html?set=%s#home" % (base, name))
        page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
        page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state')", timeout=10000)
        page.wait_for_selector("#center .home", timeout=5000)
        page.wait_for_timeout(400)
        p = page.evaluate(PROBE)
        if p["hscroll"]:
            fails.append("horizontal scroll")
        if p["vscroll"]:
            fails.append("the document scrolls (only #center may)")
        if p["centerOverflowX"]:
            fails.append("#center overflows sideways")
        SHOTS.mkdir(parents=True, exist_ok=True)
        shot = SHOTS / ("%s-%dx%d-%s-%s.png" % (name, size[0], size[1], theme, text))
        page.screenshot(path=str(shot))
        CHECKS[name](page, env, fails, deep=(text == "default"))
    except Exception as e:  # a timeout is a failure of this combination, not of the run
        fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
    ctx.close()
    return fails + errors, shot


def main():
    ap = argparse.ArgumentParser(description="T7's DOM check: Home's main pane against the t07 envelopes.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t07-*.json from the engine first")
    ap.add_argument("--no-build", action="store_true", help="with --make, use the engine already in target/debug")
    args = ap.parse_args()
    if args.make:
        make(build=not args.no_build)
    from playwright.sync_api import sync_playwright
    server, base = serve.start()
    total = failed = 0
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            for name in CHECKS:
                for size in SIZES:
                    for theme in THEMES:
                        for text in TEXTS:
                            total += 1
                            fails, shot = check_one(browser, base, name, size, theme, text)
                            label = "%s %dx%d %s %s" % (name, size[0], size[1], theme, text)
                            if fails:
                                failed += 1
                                print("FAIL " + label)
                                for f in fails:
                                    print("     " + f)
                            else:
                                print("ok   %s  %s" % (label, shot.relative_to(REPO).as_posix()))
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
