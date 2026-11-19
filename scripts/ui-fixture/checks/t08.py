"""T8's DOM check: Home's aside, Waiting and today's schedule and free time (v0.4.0 note §4 Home, plan T8).

    python scripts/ui-fixture/checks/t08.py           # every t08 set, 820x600 and 1280x800, light and dark, both text sizes
    python scripts/ui-fixture/checks/t08.py --make    # rebuild envelopes/t08-*.json from the engine first (-j 2 build)
    python scripts/ui-fixture/checks/t08.py --no-build --make

The envelopes come from the engine, never by hand: --make builds invented vaults with make_base_courses.py's own
functions (its base vault, plus the changes below), runs `knowlu-engine surface` with the oracle's pins and wraps each
answer as the app's `state` does.

| Set | The vault |
|---|---|
| t08-day | the base, plus an accepted all-day date (a lane date the student can remove), a confirmed work shift and a confirmed ABC 101 class today, a planning-day note that widened the day since its first run (so work moved), an open notice (good to know), a task with no date and one pending proposal |
| t08-digest-only | the base and make_base_courses' digest-only change: Waiting holds only an events digest |
| t08-quiet | the base: nothing waiting, so Waiting says the engine's own empty text |

For every set x size x theme x text size: no console or page error, no horizontal scroll, only #center scrolls,
then the set's own assertions. The ten-second Remove from my day runs once, at 1280x800 light. Shots go to
target/ui-shots/t08-<set>-<w>x<h>-<theme>-<text>.png. Exit 1 on any failure.
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
TAG = "t08"

# ---- the envelopes ------------------------------------------------------------------------------

LANE = "\n".join([
    "---", "type: approval", "kind: event-accept", "title: " + mb.yaml_text("Career week"), "status: executed",
    "executed_at: 2026-08-26 10:00", "source_uid: " + mb.yaml_text("fixture:lane-1"), "events:", "  - " + mb.yaml_text("fixture:lane-1"),
    "instances:", "  - uid: " + mb.yaml_text("fixture:lane-1"), "    title: " + mb.yaml_text("Career week"),
    "    start: " + mb.yaml_text("2026-08-27T00:00:00"), "    end: " + mb.yaml_text("2026-08-30T00:00:00"),
    "    location: " + mb.yaml_text("Student center"), "    url: " + mb.yaml_text(""), "    registration: false",
    "    registration_deadline: null", "created_by: events", "id: appr_e8e8e8e801", "---", "", "Invented for the fixture harness.", "",
])
SHIFT = "\n".join([
    "---", "id: cmt_e8e8e8e802", "type: commitment", "kind: work", "title: " + mb.yaml_text("Shift"),
    "meets: [{days: [fri], start: \"16:30\", end: \"18:00\"}]", "status: confirmed", "---", "", "Invented for the fixture harness.", "",
])
CLASS = "\n".join([
    "---", "id: cmt_e8e8e8e803", "type: commitment", "kind: class", "title: " + mb.yaml_text("ABC 101"), "course: abc-101",
    "meets: [{days: [mon, wed, fri], start: \"10:10\", end: \"11:00\"}]", "status: confirmed", "---", "", "Invented for the fixture harness.", "",
])
NOTICE = "\n".join([
    "---", "type: info", "kind: notice", "title: " + mb.yaml_text("Library hours change next week"), "status: open",
    "opened_at: 2026-08-27", "expires: 2026-09-04", "id: info_e8e8e8e804", "---", "", "Invented for the fixture harness.", "",
])
PLANNING = "\n".join([
    "---", "id: cmt_e8e8e8e807", "type: commitment", "kind: planning-day", "status: confirmed",
    "window: [{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]", "---", "", "Invented for the fixture harness.", "",
])
# the day's first window, as a run earlier today left it: the planning-day note has since widened it, so work moved
FIRST_WINDOW = '{"date": "2026-08-28", "end": "12:00", "start": "08:00"}\n'
PROPOSAL = "\n".join([
    "---", "type: approval", "kind: task", "title: " + mb.yaml_text("Join a study group"), "status: pending",
    "proposed_at: 2026-08-28", "expires: 2026-09-30", "snooze_until: null", "created_by: fixture", "id: appr_e8e8e8e805",
    "first_proposed_at: 2026-08-28", "---", "", "**Why proposed:** invented for the fixture harness.", "", "```task", "---",
    "title: " + mb.yaml_text("Join a study group"), "course: null", "domain: school", "due: 2026-09-04", "effort_hours: 0.5",
    "effort_confidence: low", "effort_source: inferred", "importance: 2", "status: active", "progress: 0", "---", "```", "",
])


def day(root):
    mb.write(root / "archive" / "event-career-week.md", LANE)
    mb.write(root / "commitments" / "shift.md", SHIFT)
    mb.write(root / "commitments" / "abc-101.md", CLASS)
    mb.write(root / "info" / "library-hours.md", NOTICE)
    mb.write(root / "approvals" / "task-study-group.md", PROPOSAL)
    mb.write(root / "commitments" / "planning-day.md", PLANNING)
    mb.write(root / "state" / "plan.json", FIRST_WINDOW)
    mb.write(root / "tasks" / "pick-a-lab-partner.md", mb.task_note("task_e8e8e8e806", "Pick a lab partner", None, "null", 0.5, 2, "active", 0))


VARIANTS = [("day", day), ("digest-only", mb.digest_only), ("quiet", None)]


def make(build):
    if build:
        import subprocess
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    written = []
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-t08-") as tmp:
        for name, change in VARIANTS:
            vault = pathlib.Path(tmp) / name
            mb.base_vault(vault)
            if change:
                change(vault)
            for view in ["today", "all"]:
                env = mb.envelope(mb.surface(vault, view))
                if tmp in json.dumps(env) or tmp.replace("\\", "/") in json.dumps(env):
                    sys.exit("the engine's answer names the temporary vault")
                path = OUT / ("%s-%s-%s.json" % (TAG, name, view))
                mb.dump(path, env)
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

ASIDE = """() => {
  const q = (s) => Array.from(document.querySelectorAll(s));
  const vis = (el) => !!el && !el.closest('[hidden]') && el.getClientRects().length > 0;
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  return {
    counts: txt(document.getElementById('ha-counts')),
    clear: vis(document.getElementById('ha-clear')) ? txt(document.getElementById('ha-clear')) : null,
    changed: vis(document.getElementById('ha-changed')),
    gtk: q('#ha-gtk li').map(li => ({ id: li.getAttribute('data-id'), kind: li.getAttribute('data-kind'), text: txt(li) })),
    moved: vis(document.getElementById('ha-moved')) ? txt(document.getElementById('ha-moved')) : null,
    undated: vis(document.getElementById('ha-undated')) ? { text: txt(document.getElementById('ha-undated')), href: (document.querySelector('#ha-undated a') || { getAttribute: () => null }).getAttribute('href') } : null,
    sched: q('#ha-sched .ha-sc').map(li => ({ start: li.getAttribute('data-from'), end: li.getAttribute('data-to'), name: txt(li.querySelector('.ha-sc-nm')), hidden: !vis(li), slot: (li.className.match(/\\bc\\d+\\b/) || [''])[0], remove: !!li.querySelector('[data-lane-remove]') })),
    early: vis(document.querySelector('[data-early]')) ? txt(document.querySelector('[data-early]')) : null,
    freeLead: txt(document.querySelector('#ha-free .ha-free-lead')),
    free: q('#ha-free .ha-free-row').map(li => ({ start: li.getAttribute('data-from'), end: li.getAttribute('data-to'), text: txt(li) })),
    calls: window.__fixture.calls.map(c => [c.cmd, c.args])
  };
}"""


def minutes(n):
    n = int(round(n))
    if n < 60:
        return "%d min" % n
    return "%d h" % (n // 60) + (" %d min" % (n % 60) if n % 60 else "")


def day_common(page, env, fails):
    st = env["state"]
    d = st["the_day"]
    p = page.evaluate(ASIDE)
    # review I2: data-start is the start controls' marker (plan §3); nothing in the aside carries it
    if page.evaluate("() => document.querySelectorAll('#hm-aside-wait [data-start], #hm-aside-day [data-start]').length"):
        fails.append("an aside element carries data-start, the start controls' marker")
    timed = [b for b in d["blocks"] if b["kind"] in ("class", "busy")]
    got = [(r["start"], r["end"]) for r in p["sched"] if r["start"]]
    if got != [(b["start"], b["end"]) for b in timed]:
        fails.append("schedule rows %s, want the class and busy blocks %s" % (got, [(b["start"], b["end"]) for b in timed]))
    frees = [b for b in d["blocks"] if b["kind"] == "free"]
    if [(r["start"], r["end"]) for r in p["free"]] != [(b["start"], b["end"]) for b in frees]:
        fails.append("free windows %s" % p["free"])
    if p["freeLead"] != minutes(d["open_hours"] * 60) + " free":
        fails.append("free time reads %r, want %r" % (p["freeLead"], minutes(d["open_hours"] * 60) + " free"))
    # free time agrees with the schedule: no free window overlaps a class or a busy block
    for f in p["free"]:
        for r in p["sched"]:
            if r["start"] and f["start"] < r["end"] and r["start"] < f["end"]:
                fails.append("free window %s-%s overlaps %s %s-%s" % (f["start"], f["end"], r["name"], r["start"], r["end"]))
    return p


def day_check(page, env, fails, deep, timed):
    st = env["state"]
    p = day_common(page, env, fails)
    lane = [r for r in p["sched"] if r["remove"]]
    if len(lane) != 1 or lane[0]["name"] != "Career week":
        fails.append("the all-day lane date with Remove: %s" % lane)
    cls = [r for r in p["sched"] if r["name"] == "ABC 101"]
    if not cls or cls[0]["slot"] != "c1":
        fails.append("the ABC 101 class block is not drawn in its slot: %s" % cls)
    if "1 pending" not in p["counts"]:
        fails.append("Waiting's header reads %r" % p["counts"])
    if not p["changed"] or len(p["gtk"]) != 1 or p["gtk"][0]["kind"] != "info" or "Library hours change next week" not in p["gtk"][0]["text"]:
        fails.append("What changed: %s" % p["gtk"])
    n = st["nav_counts"]["undated"]["count"]
    if not p["undated"] or p["undated"]["href"] != "#tasks/none" or str(n) not in p["undated"]["text"]:
        fails.append("Needs a date: %s" % p["undated"])
    if p["moved"] != st["moved"]["text"]:
        fails.append("the moved line reads %r, want the engine's %r" % (p["moved"], st["moved"]["text"]))
    if p["early"] is None and any(r["hidden"] for r in p["sched"]):
        fails.append("a past row is folded with no way to open it")
    if not deep:
        return
    # a domain filter leaves the schedule and free time alone
    before = (p["sched"], p["free"])
    page.click("#hm-doms .hm-dom")
    page.wait_for_timeout(150)
    q = page.evaluate(ASIDE)
    if (q["sched"], q["free"]) != before:
        fails.append("the domain filter changed the schedule or free time")
    page.click("#hm-doms .hm-dom")
    # Remove from my day hides the row at once; Undo inside the toast never calls remove_lane_date
    page.click("[data-lane-remove]")
    page.wait_for_timeout(100)
    q = page.evaluate(ASIDE)
    if any(r["remove"] for r in q["sched"]):
        fails.append("Remove from my day did not hide the date at once")
    page.click("#toasts [data-lane-undo]")
    page.wait_for_timeout(100)
    q = page.evaluate(ASIDE)
    if not any(r["remove"] for r in q["sched"]) or any(c[0] == "remove_lane_date" for c in q["calls"]):
        fails.append("Undo did not put the date back, or remove_lane_date was called")
    # Close on a good-to-know row calls close_info with the row's id
    page.click("#ha-gtk [data-close-info]")
    page.wait_for_timeout(150)
    q = page.evaluate(ASIDE)
    closes = [c[1] for c in q["calls"] if c[0] == "close_info"]
    if closes != [{"id": "info_e8e8e8e804", "view": "today"}]:
        fails.append("Close sent %s" % closes)
    if not timed:
        return
    page.click("[data-lane-remove]")
    page.wait_for_timeout(9000)
    if any(c[0] == "remove_lane_date" for c in page.evaluate(ASIDE)["calls"]):
        fails.append("remove_lane_date was called before the toast closed")
    page.wait_for_timeout(1600)
    calls = [c[1] for c in page.evaluate(ASIDE)["calls"] if c[0] == "remove_lane_date"]
    if calls != [{"uid": "fixture:lane-1", "view": "today"}]:
        fails.append("after ten seconds remove_lane_date sent %s" % calls)


def digest_check(page, env, fails, deep, timed):
    p = page.evaluate(ASIDE)
    text = page.evaluate(PROBE)["text"]
    if p["clear"] != "2 events in today's digest":
        fails.append("with only a digest Waiting says %r" % p["clear"])
    if "Nothing waiting" in text or "Queue clear" in text:
        fails.append("with a digest waiting the page says nothing is waiting")
    day_common(page, env, fails)


def quiet_check(page, env, fails, deep, timed):
    p = page.evaluate(ASIDE)
    want = env["state"]["decisions"]["empty_text"]
    if p["clear"] != want:
        fails.append("with nothing waiting Waiting says %r, want the engine's %r" % (p["clear"], want))
    if p["counts"]:
        fails.append("with nothing waiting the header counts %r" % p["counts"])
    if p["undated"]:
        fails.append("Needs a date shows with nothing undated")
    day_common(page, env, fails)


CHECKS = {"t08-day": day_check, "t08-digest-only": digest_check, "t08-quiet": quiet_check}


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
        page.wait_for_selector("#center .home #ha-sched", timeout=5000)
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
        page.evaluate("() => { const a = document.getElementById('hm-aside-wait'); if (a) { a.scrollIntoView({ block: 'start' }); } }") if size[0] < 1000 else None
        page.screenshot(path=str(shot))
        page.evaluate("() => { document.getElementById('center').scrollTop = 0; }")
        CHECKS[name](page, env, fails, deep=(text == "default"), timed=(size == (1280, 800) and theme == "light" and text == "default"))
    except Exception as e:  # a timeout is a failure of this combination, not of the run
        fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
    ctx.close()
    return fails + errors, shot


def main():
    ap = argparse.ArgumentParser(description="T8's DOM check: Home's aside against the t08 envelopes.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t08-*.json from the engine first")
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
