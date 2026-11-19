"""Load the new page in headless Edge against staged envelopes, check it and take shots (v0.4.0 plan T5c, M12).

    python scripts/ui-fixture/run.py base              # every set whose name starts with "base"
    python scripts/ui-fixture/run.py base-courses      # one set
    python scripts/ui-fixture/run.py base --scale 1.25 # a device scale (100/125/150%)

For every set x size (820x600, 1280x800) x theme (light, dark) x text size (default, larger) it loads
index.html?set=<set>#home from serve.py (127.0.0.1, a port the OS picks) and checks the shell:

  - no console error and no page error;
  - ui-ready emitted exactly once, launch_state asked once, state asked with view "today";
  - no horizontal scroll, the document itself never scrolls (#center is the only scroller);
  - html[data-theme] and the 88% / 100% zoom follow the stored preference (D4, D6);
  - the sidebar lists the envelope's courses whose status is absent or "active" (M1);
  - the band's clock reads the envelope's now; the window controls show (frameless);
  - the Status dot is amber for journal warnings or unreadable notes, red when a source needs the student, else none;
  - #tasks asks state with view "all", #class/<slug> marks that class, an unknown route falls back to Home, and a
    route with no registered page leaves #center empty, all without an error.

Then the sidebar, on base-courses (1280x800 light and dark, 820x600 light): the peek opens after about 350 ms
and not before, a mouse click inside does not hold it, it folds about 400 ms after the pointer leaves, Escape
folds it; pinned, it is the wide sidebar.

Shots go to target/ui-shots/ (never committed): <set>-<w>x<h>-<theme>-<text>.png, and
sidebar-{peek,pinned}-<w>x<h>-<theme>.png. Exit 1 on any failure.
Uses Playwright with the system's Edge (channel "msedge"), headless; no network beyond 127.0.0.1.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

from playwright.sync_api import sync_playwright

sys.dont_write_bytecode = True  # no __pycache__ beside the scripts
import serve  # noqa: E402

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent
SHOTS = REPO / "target" / "ui-shots"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]


def sets_for(prefix):
    names = sorted(p.name[: -len("-today.json")] for p in (HERE / "envelopes").glob("*-today.json"))
    return [n for n in names if n.startswith(prefix)]


def expected(set_name):
    env = json.loads((HERE / "envelopes" / (set_name + "-today.json")).read_text(encoding="utf-8"))
    answers_path = HERE / "envelopes" / (set_name + "-answers.json")
    answers = json.loads(answers_path.read_text(encoding="utf-8")) if answers_path.is_file() else {}
    if "state" in answers:
        env = answers["state"]
    if env.get("ok") is False:
        return {"classes": 0, "clock": "", "level": "error", "first_class": None}
    st = env.get("state")
    if not st:
        return {"classes": 0, "clock": "", "level": "", "first_class": None}
    active = [c for c in st.get("courses", []) if str(c.get("status") or "").strip() in ("", "active")]
    srcs = (st.get("issues_panel") or {}).get("sources") or []
    g = answers.get("grades_status") or {}
    signed_out = bool(g.get("ok") and g.get("available") and g.get("connected") and g.get("signed_out"))
    level = "error" if srcs or signed_out else ("warn" if st.get("warnings") or st.get("unreadable") else "")
    return {"classes": len(active), "clock": "Fri Aug 28 9 AM" if st.get("now") == "2026-08-28T09:00" else None,
            "level": level, "first_class": active[0]["slug"] if active else None}


PROBE = """() => {
  const app = document.querySelector('.app');
  const center = document.getElementById('center');
  const dot = document.getElementById('status-dot');
  const se = document.scrollingElement;
  return {
    theme: document.documentElement.getAttribute('data-theme'),
    text: document.documentElement.getAttribute('data-text'),
    zoom: getComputedStyle(app).zoom,
    hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
    vscroll: se.scrollHeight > window.innerHeight,
    centerOverflowX: center.scrollWidth > center.clientWidth,
    centerHtml: center.innerHTML,
    classes: document.querySelectorAll('#side-classes li').length,
    clock: document.getElementById('clock').textContent.replace(/(Fri Aug 28)/, '$1 '),
    level: dot.hidden ? '' : dot.getAttribute('data-level'),
    frameless: document.documentElement.classList.contains('frameless'),
    wcShown: getComputedStyle(document.querySelector('.wc')).display !== 'none',
    current: Array.from(document.querySelectorAll('.side [aria-current="page"]')).map(a => a.getAttribute('data-route') || ('class:' + a.getAttribute('data-class'))),
    route: document.documentElement.getAttribute('data-route'),
    calls: window.__fixture.calls.map(c => [c.cmd, c.args.view || null]),
    events: window.__fixture.events.slice()
  };
}"""


def registered_routes():
    """The routes a page registers itself for (`KnowluRouter.add("<name>"` under js/pages/): a route with no page leaves #center empty."""
    found = set()
    for f in (REPO / "app" / "static" / "next" / "js").rglob("*.js"):
        found.update(re.findall(r'KnowluRouter\.add\(\s*"([a-z]+)"', f.read_text(encoding="utf-8")))
    return found


REGISTERED = registered_routes()


def go(page, hash_, route):
    """Set the address's hash and wait until the router shows `route` with no view transition running."""
    page.evaluate("(h) => { location.hash = h; }", hash_)
    page.wait_for_function("(r) => document.documentElement.getAttribute('data-route') === r && !document.documentElement.classList.contains('vt')", arg=route, timeout=5000)


def check_one(page, base, set_name, size, theme, text, want):
    fails = []
    errors = []
    page.on("console", lambda m: errors.append("console: %s (%s)" % (m.text, m.location.get("url", ""))) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    page.goto("%sindex.html?set=%s#home" % (base, set_name))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state')", timeout=10000)
    page.wait_for_timeout(400)
    p = page.evaluate(PROBE)

    def need(ok, what):
        if not ok:
            fails.append(what)

    need(p["events"].count("ui-ready") == 1, "ui-ready emitted %d times" % p["events"].count("ui-ready"))
    need([c for c in p["calls"] if c[0] == "launch_state"] == [["launch_state", None]], "launch_state asked once")
    need(["state", "today"] in p["calls"], "state asked with view today")
    need(not any(c[0] == "google_status" for c in p["calls"]), "google_status asked at launch")
    need(not any(c[0] == "mark_seen" for c in p["calls"]), "mark_seen at boot")
    need(not p["hscroll"], "horizontal scroll")
    need(not p["vscroll"], "the document scrolls (only #center may)")
    need(not p["centerOverflowX"], "#center overflows sideways")
    need(p["theme"] == theme, "data-theme is %s" % p["theme"])
    need((p["text"] == "larger") == (text == "larger"), "data-text is %s" % p["text"])
    need(abs(float(p["zoom"]) - (1.0 if text == "larger" else 0.88)) < 0.001, "zoom is %s" % p["zoom"])
    need(p["classes"] == want["classes"], "sidebar lists %d classes, want %d" % (p["classes"], want["classes"]))
    if want["clock"] is not None:
        need(p["clock"].replace("  ", " ").strip() == want["clock"], "clock reads %r" % p["clock"])
    need(p["level"] == want["level"], "status dot %r, want %r" % (p["level"], want["level"]))
    need(p["frameless"] and p["wcShown"], "window controls")
    need(p["current"] == ["home"], "sidebar marks %s" % p["current"])
    if "home" not in REGISTERED:
        need(p["centerHtml"] == "", "#center is not empty with no Home page registered")

    go(page, "#tasks", "tasks")
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state' && c.args.view === 'all')", timeout=5000)
    q = page.evaluate(PROBE)
    need(q["route"] == "tasks" and q["current"] == ["tasks"], "#tasks: route %s, marks %s" % (q["route"], q["current"]))
    if "tasks" not in REGISTERED:
        need(q["centerHtml"] == "", "#tasks: #center not empty with no Tasks page registered")
    if want["first_class"]:
        go(page, "#class/" + want["first_class"], "class")
        q = page.evaluate(PROBE)
        need(q["route"] == "class" and "class:" + want["first_class"] in q["current"], "#class: marks %s" % q["current"])
    go(page, "#constructor", "home")
    q = page.evaluate(PROBE)
    need(q["route"] == "home", "#constructor resolves to %s" % q["route"])
    go(page, "#home", "home")
    page.wait_for_timeout(300)

    SHOTS.mkdir(parents=True, exist_ok=True)
    shot = SHOTS / ("%s-%dx%d-%s-%s.png" % (set_name, size[0], size[1], theme, text))
    page.screenshot(path=str(shot))
    return fails + errors, shot


def sidebar(browser, base, size, theme):
    """The peek and the pin (SHELL.md "The sidebar"), with a shot of each."""
    fails = []
    SHOTS.mkdir(parents=True, exist_ok=True)
    ready = "() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0"
    peek = "() => document.documentElement.classList.contains('peek')"
    tag = "%dx%d-%s" % (size[0], size[1], theme)

    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s);" % json.dumps(theme))
    page = ctx.new_page()
    page.goto("%sindex.html?set=base-courses#home" % base)
    page.wait_for_function(ready, timeout=10000)
    page.wait_for_timeout(300)
    page.mouse.move(26, 300)
    page.wait_for_timeout(200)
    if page.evaluate(peek):
        fails.append("the peek opened before 350 ms")
    page.wait_for_timeout(300)
    if not page.evaluate(peek):
        fails.append("the peek did not open after 500 ms on the rail")
    page.wait_for_timeout(250)
    shot_peek = SHOTS / ("sidebar-peek-%s.png" % tag)
    page.screenshot(path=str(shot_peek))
    page.click("#classes-toggle")
    page.click("#classes-toggle")
    page.mouse.move(size[0] - 100, size[1] // 2)
    page.wait_for_timeout(150)
    if not page.evaluate(peek):
        fails.append("the peek folded at once on leaving (it waits about 400 ms)")
    page.wait_for_timeout(650)
    if page.evaluate(peek):
        fails.append("a mouse click inside held the peek open after the pointer left")
    page.mouse.move(26, 300)
    page.wait_for_function(peek, timeout=3000)
    page.keyboard.press("Escape")
    page.wait_for_timeout(50)
    if page.evaluate(peek):
        fails.append("Escape did not fold the peek")
    ctx.close()

    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.pin', '1');" % json.dumps(theme))
    page = ctx.new_page()
    page.goto("%sindex.html?set=base-courses#class/abc-101" % base)
    page.wait_for_function(ready, timeout=10000)
    page.wait_for_timeout(400)
    p = page.evaluate("() => ({ rail: document.documentElement.classList.contains('rail'), pressed: document.getElementById('rail-toggle').getAttribute('aria-pressed'), labels: getComputedStyle(document.querySelector('.nav a span')).display, hscroll: document.scrollingElement.scrollWidth > window.innerWidth })")
    if p["rail"] or p["pressed"] != "true" or p["labels"] == "none" or p["hscroll"]:
        fails.append("pinned: %s" % p)
    shot_pinned = SHOTS / ("sidebar-pinned-%s.png" % tag)
    page.screenshot(path=str(shot_pinned))
    ctx.close()
    return fails, [shot_peek, shot_pinned]


def main():
    ap = argparse.ArgumentParser(description="Check the new page against staged envelopes in headless Edge.")
    ap.add_argument("prefix", nargs="?", default="base", help="run every envelope set whose name starts with this")
    ap.add_argument("--checks", action="store_true", help="run every checks/t*.py in turn instead; fail if any fails")
    ap.add_argument("--scale", type=float, default=1.0, help="device scale factor (1, 1.25, 1.5)")
    args = ap.parse_args()
    if args.checks:
        bad = [c.name for c in sorted((HERE / "checks").glob("t*.py")) if print("== " + c.name, flush=True) or subprocess.run([sys.executable, str(c)]).returncode != 0]
        print("checks failed: %s" % ", ".join(bad) if bad else "all checks passed")
        sys.exit(1 if bad else 0)
    names = sets_for(args.prefix)
    if not names:
        sys.exit("no envelope set starts with %r (run make_base_courses.py)" % args.prefix)
    server, base = serve.start()
    failed = 0
    total = 0
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            for name in names:
                want = expected(name)
                for size in SIZES:
                    for theme in THEMES:
                        for text in TEXTS:
                            total += 1
                            ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, device_scale_factor=args.scale,
                                                      color_scheme=theme, service_workers="block")
                            ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
                            page = ctx.new_page()
                            try:
                                fails, shot = check_one(page, base, name, size, theme, text, want)
                            except Exception as e:  # a timeout is a failure of this combination, not of the run
                                fails, shot = ["%s: %s" % (type(e).__name__, str(e).splitlines()[0])], None
                            ctx.close()
                            label = "%s %dx%d %s %s" % (name, size[0], size[1], theme, text)
                            if fails:
                                failed += 1
                                print("FAIL " + label)
                                for f in fails:
                                    print("     " + f)
                            else:
                                print("ok   %s  %s" % (label, shot.relative_to(REPO).as_posix()))
            if "base-courses" in names:
                for size, theme in [((1280, 800), "light"), ((1280, 800), "dark"), ((820, 600), "light")]:
                    total += 1
                    try:
                        fails, shots = sidebar(browser, base, size, theme)
                    except Exception as e:
                        fails, shots = ["%s: %s" % (type(e).__name__, str(e).splitlines()[0])], []
                    label = "sidebar %dx%d %s" % (size[0], size[1], theme)
                    if fails:
                        failed += 1
                        print("FAIL " + label)
                        for f in fails:
                            print("     " + f)
                    else:
                        print("ok   %s  %s" % (label, "  ".join(x.relative_to(REPO).as_posix() for x in shots)))
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
