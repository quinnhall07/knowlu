"""T9's DOM check: Lock in (v0.4.0 note §4 "Lock in", D6; plan T9; DESIGN.md "Lock in session").

    python scripts/ui-fixture/checks/t09.py           # 820x600 and 1280x800, light and dark, both text sizes
    python scripts/ui-fixture/checks/t09.py --make    # rebuild envelopes/t09-*.json from the engine first (-j 2 build)
    python scripts/ui-fixture/checks/t09.py --no-build --make   # rebuild with the engine already in target/debug

The envelopes come from the engine, never by hand: --make builds an invented vault with make_base_courses.py's own
functions (its base vault and one more invented JKL 120 task the engine ranks first in Recommended, so Home's order
there is not the due order), runs `knowlu-engine surface` with the oracle's pins and wraps each answer as the app's
`state` does.

| Set | The vault |
|---|---|
| t09-lockin | the base and a big JKL 120 task due Sep 4 |

For every size x theme x text size: no console or page error, no horizontal scroll, only #center scrolls; the sidebar
steps aside; the queue from the chosen id is Home's order (Do today, then Recommended), walked to the end with Done and
Start next, each Done the kit's set_fields {progress: 100, status: "done"}; the beat shows what is left, the finished
one and the next, with Start next in the next class's colour; Undo puts the assignment back; Hide time hides the clock
and keeps its place; the closing page lists what was done with Home; End goes Home; nothing is written to storage.
After each step the new primary control holds the focus (review M2). The break is driven by moving the page's clock
(review M5). Playwright's page.clock conflicts with the harness's frozen Date: installed before load, the harness's
stub freezes the fake Date again; installed after load, its timers run but its Date.now stays at the frozen instant
(fast_forward moves nothing Lock in reads). So the check holds Date.now itself: after load, Date.now reads a number
the check moves (25 minutes, then 2), and Lock in's own 250 ms interval picks it up. 25:00 on reads "Break ready",
Break now runs the 5-minute break, Skip break comes back to Focus, and the closing page's Focused counts no break.
Shots go to target/ui-shots/t09-<set>-<state>-<w>x<h>-<theme>-<text>.png. Exit 1 on any failure.
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
SET = "t09-lockin"

EXTRA_TASKS = [
    ("jkl-120-portfolio", "task_c1a55e5101", "JKL 120 Portfolio", "jkl-120", "2026-09-04T23:59", 12.0, 5, "active", 0),
]


def lockin(root):
    for stem, tid, title, course, due, effort, imp, status, progress in EXTRA_TASKS:
        mb.write(root / "tasks" / (stem + ".md"), mb.task_note(tid, title, course, due, effort, imp, status, progress))


def make(build):
    if build:
        import subprocess
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    banned = mb.fixture_slugs()
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-t09-") as tmp:
        vault = pathlib.Path(tmp) / "lockin"
        mb.base_vault(vault)
        lockin(vault)
        for view in ["today", "all"]:
            env = mb.envelope(mb.surface(vault, view))
            text = json.dumps(env, ensure_ascii=False)
            if tmp in text or tmp.replace("\\", "/") in text or tmp.replace("\\", "\\\\") in text:
                sys.exit("the engine's answer names the temporary vault")
            for slug in banned:
                if ('"%s"' % slug) in text:
                    sys.exit("the envelope carries the fixture vault's course slug %s" % slug)
            path = OUT / ("%s-%s.json" % (SET, view))
            mb.dump(path, env)
            print(path.relative_to(REPO).as_posix())


# ---- the checks ---------------------------------------------------------------------------------

FOCUSED = """(sel) => { const a = document.activeElement; return !!a && a.isConnected && a.matches(sel); }"""


def focus_is(page, sel, fails, where):
    if not page.evaluate(FOCUSED, sel):
        fails.append("%s: the focus is not on %s (%s)" % (where, sel, page.evaluate("() => document.activeElement ? document.activeElement.outerHTML.slice(0, 60) : null")))


PROBE = """() => {
  const se = document.scrollingElement, center = document.getElementById('center');
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  const vis = (el) => !!el && !el.closest('[hidden]') && el.getClientRects().length > 0 && getComputedStyle(el).visibility !== 'hidden';
  const lk = document.querySelector('#center .lk');
  return {
    hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
    vscroll: se.scrollHeight > window.innerHeight,
    centerOverflowX: center.scrollWidth > center.clientWidth,
    side: getComputedStyle(document.getElementById('side')).display,
    mode: lk ? lk.getAttribute('data-mode') : null,
    name: txt(document.querySelector('#center .lk-name')),
    id: lk ? lk.getAttribute('data-cur') : null,
    slot: (document.querySelector('#center .lk-field') ? document.querySelector('#center .lk-field').className.match(/\\bc\\d+\\b/) || [''] : [''])[0],
    label: txt(document.querySelector('#center .lk-clabel')),
    clock: txt(document.querySelector('#center .lk-clock')),
    clockShown: vis(document.querySelector('#center .lk-clock')),
    place: txt(document.querySelector('#center .lk-place')),
    left: txt(document.querySelector('#center .lk-left')),
    was: txt(document.querySelector('#center .lk-was')),
    next: !!document.querySelector('#center [data-lk-next]'),
    nextSlot: document.querySelector('#center [data-lk-next]') ? (document.querySelector('#center [data-lk-next]').className.match(/\\bc\\d+\\b/) || [''])[0] : null,
    doneList: Array.from(document.querySelectorAll('#center .lk-done-row .lk-done-nm')).map(txt),
    home: !!document.querySelector('#center .lk-home[href="#home"]'),
    end: !!document.querySelector('#center a[href="#home"].lk-end'),
    storage: Object.keys(localStorage).sort(),
    sets: window.__fixture.calls.filter(c => c.cmd === 'set_fields').map(c => c.args)
  };
}"""


def home_order(env):
    st = env["state"]
    out, seen = [], set()
    for g in st["must_do"]["groups"]:
        for r in g["rows"]:
            if r["id"] not in seen:
                seen.add(r["id"])
                out.append(r)
    for x in st["recommended"]["rows"]:
        r = x["row"]
        if r["id"] not in seen:
            seen.add(r["id"])
            out.append(r)
    return [r for r in out if r.get("status") != "done"]


def slot_of(env, course):
    for c in env["state"].get("courses") or []:
        if c["slug"] == course:
            return c["slot"]
    return ""


def shot(page, where, size, theme, text):
    SHOTS.mkdir(parents=True, exist_ok=True)
    path = SHOTS / ("%s-%s-%dx%d-%s-%s.png" % (SET, where, size[0], size[1], theme, text))
    page.screenshot(path=str(path))
    return path


def layout(p, fails, where):
    if p["hscroll"]:
        fails.append("%s: horizontal scroll" % where)
    if p["vscroll"]:
        fails.append("%s: the document scrolls (only #center may)" % where)
    if p["centerOverflowX"]:
        fails.append("%s: #center overflows sideways" % where)


def open_lockin(page, base, hash_):
    page.goto("%sindex.html?set=%s%s" % (base, SET, hash_))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state' && c.args.view === 'today')", timeout=10000)
    page.wait_for_selector("#center .lk[data-mode]", timeout=5000)
    page.wait_for_timeout(400)


def walk(page, env, fails, size, theme, text, deep):
    queue = home_order(env)
    start = 2  # from the third row of Home's order: the last Do today row, then Recommended
    want = queue[start:]
    shots = []
    p = page.evaluate(PROBE)
    layout(p, fails, "focus")
    if p["side"] != "none":
        fails.append("the sidebar shows on Lock in (%s)" % p["side"])
    if p["mode"] != "focus" or p["id"] != want[0]["id"] or p["name"] != want[0]["title"]:
        fails.append("Lock in opens on %s %r, want %s" % (p["id"], p["name"], want[0]["id"]))
    if p["slot"] != slot_of(env, want[0]["course"]):
        fails.append("the field is %r, want %r" % (p["slot"], slot_of(env, want[0]["course"])))
    if p["label"] != "Focus" or p["clock"] != "25:00" or not p["clockShown"]:
        fails.append("the clock half reads %r %r (shown %s)" % (p["label"], p["clock"], p["clockShown"]))
    if p["place"] != "1 of %d today" % len(want):
        fails.append("the place reads %r" % p["place"])
    shots.append(shot(page, "focus", size, theme, text))
    if not deep:
        return shots
    # Hide time: the figure goes, its place and label stay
    page.click("#center [data-lk-hide]")
    page.wait_for_timeout(100)
    q = page.evaluate(PROBE)
    if q["clockShown"] or q["label"] != "Focus":
        fails.append("Hide time left the clock %s, label %r" % (q["clockShown"], q["label"]))
    page.click("#center [data-lk-hide]")
    page.wait_for_timeout(100)
    if not page.evaluate(PROBE)["clockShown"]:
        fails.append("Show time did not bring the clock back")
    # Undo: Done, then Undo in the beat, puts the same assignment back
    page.click("#center .lk-acts [data-lk-done]")
    page.wait_for_selector("#center .lk[data-mode='beat']", timeout=3000)
    focus_is(page, "#center [data-lk-next]", fails, "the beat")
    page.click("#center [data-lk-undo]")
    page.wait_for_selector("#center .lk[data-mode='focus']", timeout=3000)
    page.wait_for_timeout(100)
    focus_is(page, "#center .lk-acts [data-lk-done]", fails, "after Undo")
    q = page.evaluate(PROBE)
    if q["id"] != want[0]["id"]:
        fails.append("Undo came back to %s" % q["id"])
    if not q["sets"] or q["sets"][-1] != {"id": want[0]["id"], "fields": {"progress": want[0].get("progress_pct") or 0, "status": "active"}, "view": "today"}:
        fails.append("Undo sent %s" % q["sets"][-1:])
    base_sets = len(q["sets"])
    # walk the queue to its end: Done, the beat, Start next
    for i, row in enumerate(want):
        page.click("#center .lk-acts [data-lk-done]")
        page.wait_for_function("() => ['beat', 'close'].indexOf(document.querySelector('#center .lk').getAttribute('data-mode')) >= 0", timeout=3000)
        q = page.evaluate(PROBE)
        sent = q["sets"][base_sets + i] if len(q["sets"]) > base_sets + i else None
        if sent != {"id": row["id"], "fields": {"progress": 100, "status": "done"}, "view": "today"}:
            fails.append("Done on %s sent %s" % (row["id"], sent))
        if i + 1 < len(want):
            nxt = want[i + 1]
            left = len(want) - i - 1
            if q["mode"] != "beat" or q["was"] != row["title"] or q["name"] != nxt["title"] or not q["next"]:
                fails.append("the beat after %s: mode %s, was %r, next %r, Start next %s" % (row["id"], q["mode"], q["was"], q["name"], q["next"]))
            if not q["left"].startswith("%d left" % left):
                fails.append("the beat says %r, want %d left" % (q["left"], left))
            if q["nextSlot"] != slot_of(env, nxt["course"]):
                fails.append("Start next is %r, want the next class %r" % (q["nextSlot"], slot_of(env, nxt["course"])))
            layout(q, fails, "beat")
            if i == 0:
                page.wait_for_timeout(450)  # the beat's arrival (opacity, --med) has settled
                shots.append(shot(page, "beat", size, theme, text))
            page.click("#center [data-lk-next]")
            page.wait_for_selector("#center .lk[data-mode='focus']", timeout=3000)
            focus_is(page, "#center .lk-acts [data-lk-done]", fails, "after Start next")
            q = page.evaluate(PROBE)
            if q["id"] != nxt["id"]:
                fails.append("Start next opened %s, want %s" % (q["id"], nxt["id"]))
            if q["place"] != "%d of %d today" % (i + 2, len(want)):
                fails.append("the place reads %r" % q["place"])
        else:
            if q["mode"] != "close" or q["doneList"] != [r["title"] for r in want] or not q["home"]:
                fails.append("the closing page: mode %s, done %s, Home %s" % (q["mode"], q["doneList"], q["home"]))
            layout(q, fails, "close")
            focus_is(page, "#center .lk-home", fails, "the closing page")
            shots.append(shot(page, "close", size, theme, text))
    q = page.evaluate(PROBE)
    if q["storage"] != ["knowlu.text", "knowlu.theme"]:
        fails.append("Lock in left storage keys %s" % q["storage"])
    return shots


def drive_break(page, base, env, fails, size, theme, text):
    """The clock run on a moved Date.now: Break ready, Break now, Skip break, and Focused without the break."""
    queue = home_order(env)
    want = queue[2:]
    page.goto("%sindex.html?set=%s#home" % (base, SET))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_timeout(300)
    page.evaluate("() => { window.__now = Date.now(); Date.now = () => window.__now; }")  # from the harness's frozen instant
    ahead = lambda ms: (page.evaluate("(ms) => { window.__now += ms; }", ms), page.wait_for_timeout(450))
    page.evaluate("(h) => { location.hash = h; }", "#lockin/" + want[0]["id"])
    page.wait_for_selector("#center .lk[data-mode='focus']", timeout=5000)
    probe = "() => ({ label: document.querySelector('#center .lk-clabel').textContent, clock: document.querySelector('#center .lk-clock').textContent, brk: !!document.querySelector('#center [data-lk-break]'), skip: !!document.querySelector('#center [data-lk-skip]'), phase: document.querySelector('#center .lk').getAttribute('data-phase') })"
    page.focus("#center [data-lk-hide]")
    ahead(25 * 60000)
    q = page.evaluate(probe)
    if (q["label"], q["clock"], q["brk"], q["phase"]) != ("Break ready", "5:00", True, "ready"):
        fails.append("25 minutes on: %s" % q)
    focus_is(page, "#center [data-lk-break]", fails, "Break ready")
    page.click("#center [data-lk-break]")
    page.wait_for_timeout(100)
    focus_is(page, "#center [data-lk-skip]", fails, "after Break now")
    ahead(2 * 60000)
    q = page.evaluate(probe)
    if (q["label"], q["clock"], q["skip"], q["phase"]) != ("Break", "3:00", True, "break"):
        fails.append("2 minutes into the break: %s" % q)
    shot(page, "break", size, theme, text)
    page.click("#center [data-lk-skip]")
    page.wait_for_timeout(100)
    focus_is(page, "#center [data-lk-hide]", fails, "after Skip break")
    q = page.evaluate(probe)
    if (q["label"], q["clock"], q["skip"], q["phase"]) != ("Focus", "25:00", False, "focus"):
        fails.append("after Skip break: %s" % q)
    for i in range(len(want)):
        page.click("#center .lk-acts [data-lk-done]")
        page.wait_for_function("() => ['beat', 'close'].indexOf(document.querySelector('#center .lk').getAttribute('data-mode')) >= 0", timeout=3000)
        if i + 1 < len(want):
            page.click("#center [data-lk-next]")
            page.wait_for_selector("#center .lk[data-mode='focus']", timeout=3000)
    spent = page.evaluate("() => document.querySelector('#center .lk-spent').textContent")
    if spent != "25 min":
        fails.append("Focused reads %r, want 25 min (the 2-minute break not counted)" % spent)


def check_one(browser, base, size, theme, text):
    fails, errors, shots = [], [], []
    env = json.loads((OUT / (SET + "-today.json")).read_text(encoding="utf-8"))
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
    page = ctx.new_page()
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    deep = text == "default"
    try:
        queue = home_order(env)
        open_lockin(page, base, "#lockin/" + queue[2]["id"])
        shots += walk(page, env, fails, size, theme, text, deep)
        # with no id, Lock in starts at Home's lead; End goes Home
        page.evaluate("() => { location.hash = '#home'; }")
        page.wait_for_function("() => document.documentElement.getAttribute('data-route') === 'home'", timeout=5000)
        page.evaluate("() => { location.hash = '#lockin'; }")
        page.wait_for_selector("#center .lk[data-mode='focus']", timeout=5000)
        page.wait_for_timeout(200)
        q = page.evaluate(PROBE)
        if q["id"] != queue[0]["id"]:
            fails.append("#lockin with no id opens %s, want the lead %s" % (q["id"], queue[0]["id"]))
        if not q["end"]:
            fails.append("no End")
        page.click("#center a.lk-end")
        page.wait_for_function("() => document.documentElement.getAttribute('data-route') === 'home'", timeout=5000)
        page.wait_for_timeout(200)
        if page.evaluate("() => getComputedStyle(document.getElementById('side')).display") == "none":
            fails.append("the sidebar stays hidden after Lock in")
        if deep:
            clocked = ctx.new_page()
            clocked.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
            clocked.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
            drive_break(clocked, base, env, fails, size, theme, text)
    except Exception as e:  # a timeout is a failure of this combination, not of the run
        fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
    ctx.close()
    return fails + errors, shots


def main():
    ap = argparse.ArgumentParser(description="T9's DOM check: Lock in against the t09 envelopes.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t09-*.json from the engine first")
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
            for size in SIZES:
                for theme in THEMES:
                    for text in TEXTS:
                        total += 1
                        fails, shots = check_one(browser, base, size, theme, text)
                        label = "%s %dx%d %s %s" % (SET, size[0], size[1], theme, text)
                        if fails:
                            failed += 1
                            print("FAIL " + label)
                            for f in fails:
                                print("     " + f)
                        else:
                            print("ok   %s  %s" % (label, "  ".join(s.relative_to(REPO).as_posix() for s in shots)))
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
