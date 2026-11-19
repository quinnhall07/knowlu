"""T10 Tasks: the DOM check, and the driver t14.py and t15.py share (v0.4.0 plan §5 T10, M12).

    python scripts/ui-fixture/checks/t10.py           # check Tasks against the staged t10 envelopes
    python scripts/ui-fixture/checks/t10.py --make    # rebuild envelopes/t10-*.json from the engine first

The envelopes come from the engine, never by hand: --make builds T5c's base vault (make_base_courses.base_vault)
in a temporary folder, changes it the way each set needs (an undated task; two closes written through the
engine's own `write`, as the student and as the completion agent), runs `knowlu-engine surface` with the oracle's
pins and wraps the answer as the app does (make_base_courses.envelope). The journal's two timestamps are moved
onto the pinned week (2026-08-27 UTC), since the engine stamps a record with the real clock and `surface` counts a
close only inside the seven days before its pinned now. Nothing else in a record is touched.

Checks, for every size (820x600, 1280x800) x theme x text size, on #tasks: no horizontal scroll and no console
error; the column head (Task, Class, Due, Estimate; no Priority); the groups in order and the rows in the engine's
order inside each. Then, at 1280x800 light and 820x600 dark larger: Sort Due and back to priority; the class,
search and due filters and Clear; the Done tab (Undo only on the engine's undo row, set_fields status active);
All; New task (create_task with its fields); the unreadable line; the engine's empty strings. Shots go to
target/ui-shots/t10-*.png (never committed). Exit 1 on any failure.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
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
FULL = [((1280, 800), "light", "default"), ((820, 600), "dark", "larger")]


# ---------------------------------------------------------------- envelopes, from the engine


def engine_write(vault, target, fields, actor=None):
    """One `knowlu-engine write ... set` through the engine's own write path (journal first)."""
    args = [str(mb.ENGINE), "write", "--vault", str(vault), "--via", "dashboard"]
    if actor:
        args += ["--actor", actor]
    args += ["set", target] + ["%s=%s" % kv for kv in fields]
    env = dict(os.environ, KNOWLU_DEVICE="fixture")
    out = subprocess.run(args, capture_output=True, env=env)
    if out.returncode != 0:
        sys.exit("write %s failed: %s" % (target, out.stderr.decode("utf-8", "replace")))


def pin_journal(vault, stamps):
    """Move the journal's records onto fixed UTC stamps, one per record, in order (all on one day)."""
    folder = vault / "state" / "journal"
    files = sorted(folder.glob("*.jsonl"))
    lines = [ln for f in files for ln in f.read_text(encoding="utf-8").splitlines() if ln.strip()]
    if len(lines) != len(stamps):
        sys.exit("the journal holds %d records, %d stamps given" % (len(lines), len(stamps)))
    for f in files:
        f.unlink()
    pinned = [re.sub(r'"ts": "[^"]*"', '"ts": "%s"' % ts, ln, count=1) for ln, ts in zip(lines, stamps)]
    (folder / (stamps[0][:10] + ".jsonl")).write_bytes(("\n".join(pinned) + "\n").encode("utf-8"))


def student(vault):
    """The vault's human token, as the app writes it at creation (ruling 11): never a name."""
    mb.write(vault / "config" / "actor.yaml", "human_actor: student\n")


def make_sets(sets):
    """Build each (name, change, answers) set: base vault, change(vault), surface today and all; answers(vault, envs)
    returns the set's staged answers (or None). Writes envelopes/<name>-{today,all}.json and <name>-answers.json."""
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    banned = mb.fixture_slugs()
    written = []
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-fixture-") as tmp:
        for name, change, answers in sets:
            vault = pathlib.Path(tmp) / name
            mb.base_vault(vault)
            student(vault)
            if change:
                change(vault)
            envs = {}
            for view in ["today", "all"]:
                envs[view] = mb.envelope(mb.surface(vault, view))
            staged = answers(vault, envs) if answers else None
            for view, env in envs.items():
                path = OUT / ("%s-%s.json" % (name, view))
                mb.dump(path, env)
                written.append(path)
            if staged is not None:
                path = OUT / ("%s-answers.json" % name)
                mb.dump(path, staged)
                written.append(path)
        for path in written:
            text = path.read_text(encoding="utf-8")
            for needle in [tmp, tmp.replace("\\", "/"), tmp.replace("\\", "\\\\")]:
                if needle in text:
                    sys.exit("%s names the temporary vault" % path.name)
            for slug in sorted(banned):
                if re.search(r"(?<![A-Za-z0-9-])" + re.escape(slug) + r"(?![A-Za-z0-9-])", text):
                    sys.exit("%s carries the fixture vault's course slug %s" % (path.name, slug))
    for path in written:
        print(path.relative_to(REPO).as_posix())


def undated_task(vault):
    """An invented task with no due date: the No date group."""
    text = mb.task_note("task_a1b2c3d408", "Return lab key", None, "2026-01-01", 0.5, 2, "active", 0).replace("due: 2026-01-01", "due: null")
    mb.write(vault / "tasks" / "return-lab-key.md", text)


def t10_tasks(vault):
    undated_task(vault)
    engine_write(vault, "task_a1b2c3d402", [("progress", "100"), ("status", "done")])
    engine_write(vault, "task_a1b2c3d406", [("status", "done")], actor="agent:knowlu.completion")
    pin_journal(vault, ["2026-08-27T15:00:00.000Z", "2026-08-27T15:00:00.000Z", "2026-08-27T18:30:00.000Z"])


def t10_empty(vault):
    """Courses and no task at all: the engine's own empty strings."""
    for p in (vault / "tasks").glob("*.md"):
        p.unlink()


SETS = [("t10-tasks", t10_tasks, None), ("t10-empty", t10_empty, None)]


# ---------------------------------------------------------------- the driver


READY = "() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0"
LAYOUT = """() => {
  const se = document.scrollingElement, c = document.getElementById('center');
  return { hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
           centerX: c.scrollWidth > c.clientWidth };
}"""


def calls(page, cmd=None):
    got = page.evaluate("() => window.__fixture.calls")
    return [c for c in got if cmd is None or c["cmd"] == cmd]


def events(page, action=None):
    return [c["args"] for c in calls(page, "ui_event") if action is None or c["args"].get("action") == action]


def open_page(browser, base, set_name, route, size, theme, text, init=""):
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);%s" % (json.dumps(theme), json.dumps(text), init))
    page = ctx.new_page()
    errors = []
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    page.goto("%sindex.html?set=%s#%s" % (base, set_name, route))
    page.wait_for_function(READY, timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state')", timeout=10000)
    page.wait_for_timeout(350)
    return ctx, page, errors


def run(tag, cases):
    """cases: dicts {name, set, route, check(page, full, need), combos (optional), init (optional)}. For each combination
    the check runs, then the layout is measured and a shot is taken. Exits 1 on any failure."""
    from playwright.sync_api import sync_playwright
    server, base = serve.start()
    failed = total = 0
    SHOTS.mkdir(parents=True, exist_ok=True)
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            for case in cases:
                combos = case.get("combos") or [(s, t, x) for s in SIZES for t in THEMES for x in TEXTS]
                for size, theme, text in combos:
                    total += 1
                    fails = []
                    full = (size, theme, text) in FULL
                    label = "%s %s %dx%d %s %s" % (tag, case["name"], size[0], size[1], theme, text)
                    shot = SHOTS / ("%s-%s-%dx%d-%s-%s.png" % (tag, case["name"], size[0], size[1], theme, text))
                    ctx = None
                    try:
                        ctx, page, errors = open_page(browser, base, case["set"], case["route"], size, theme, text, case.get("init", ""))

                        def need(ok, what):
                            if not ok:
                                fails.append(what)
                        case["check"](page, full, need)
                        lay = page.evaluate(LAYOUT)
                        need(not lay["hscroll"], "horizontal scroll")
                        need(not lay["centerX"], "#center overflows sideways")
                        page.evaluate("() => { document.getElementById('toasts').textContent = ''; }")  # the shot shows the page, not the toasts
                        page.screenshot(path=str(shot))
                        fails += errors
                    except Exception as e:  # a timeout fails this combination, not the run
                        fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0] if str(e) else ""))
                    finally:
                        if ctx:
                            ctx.close()
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


def envelope_of(set_name, view):
    return json.loads((OUT / ("%s-%s.json" % (set_name, view))).read_text(encoding="utf-8"))["state"]


# ---------------------------------------------------------------- the checks


GROUP_ORDER = ["Late", "Today", "This week", "Later", "No date"]
PROBE = """() => {
  const list = document.querySelector('.tk-list');
  if (!list) { return null; }
  const items = Array.from(list.children).map(li => li.classList.contains('tk-grp') ? { grp: li.textContent.trim() } :
    li.classList.contains('tk-row') ? { id: li.getAttribute('data-id'), closed: li.classList.contains('is-closed'), undo: !!li.querySelector('[data-undo-close]'),
      name: (li.querySelector('.tk-nm') || {}).textContent || '' } : { other: li.className });
  const head = document.querySelector('.tk-cols');
  const empty = document.querySelector('.tk-empty');
  const unread = document.querySelector('.tk-unread');
  return { items, head: head ? head.textContent : '', headShown: !!head && getComputedStyle(head).display !== 'none',
           empty: empty && !empty.hidden ? empty.textContent.trim() : '', unread: unread && !unread.hidden ? unread.textContent : '',
           count: (document.querySelector('[data-tk-count]') || {}).textContent || '' };
}"""


def rows_of(p):
    return [i["id"] for i in p["items"] if "id" in i]


def groups_of(p):
    return [i["grp"] for i in p["items"] if "grp" in i]


def expected_open(st):
    """The open rows the page should show, as (group, id) in the engine's order; the grouping rule restated from the
    note (§4: Late, Today, This week, Later, No date by due, against S.now's day)."""
    import datetime
    today = datetime.date.fromisoformat(st["now"][:10])
    out = []
    for r in st["list"]["rows"]:
        if not r.get("due"):
            g = 4
        else:
            d = (datetime.date.fromisoformat(r["due"][:10]) - today).days
            g = 0 if d < 0 else 1 if d == 0 else 2 if d <= 6 else 3
        out.append((g, r["id"]))
    return out


def select(page, sel, value):
    page.select_option(sel, value)
    page.wait_for_timeout(80)


def check_tasks(page, full, need):
    page.wait_for_selector(".tk-list .tk-row", timeout=5000)
    st = envelope_of("t10-tasks", "all")
    p = page.evaluate(PROBE)
    want = expected_open(st)
    groups = [GROUP_ORDER[g] for g in sorted({g for g, _ in want})]
    need(groups_of(p) == groups, "groups %s, want %s" % (groups_of(p), groups))
    order = [i for g in range(5) for gg, i in want if gg == g]
    need(rows_of(p) == order, "rows %s, want the engine's order inside each group %s" % (rows_of(p), order))
    need("Priority" not in p["head"] and "Type" not in p["head"], "a Priority or Type column: %r" % p["head"])
    if p["headShown"]:
        need(all(w in p["head"] for w in ["Task", "Class", "Due", "Estimate"]), "column head %r" % p["head"])
    late = page.locator('.tk-row[data-id="task_a1b2c3d401"] .tk-late')
    need(late.count() == 1 and late.inner_text().strip() == "Late", "the overdue row says Late")
    need(page.locator('.tk-row[data-id="task_a1b2c3d408"]').inner_text().find("No date") >= 0, "the undated row says No date")
    need(page.evaluate("() => { const b = document.querySelector('.tk-row[data-id=\"task_a1b2c3d404\"] .ck'); return b && b.className.indexOf('c') >= 0 && /\\bc\\d+\\b/.test(b.className); }"),
         "a row's check carries its class slot")
    if not full:
        return
    # Sort Due, then back to priority: the engine's order returns.
    select(page, "[data-tk-sort]", "due")
    p2 = page.evaluate(PROBE)
    by_due = [i for g in range(5) for gg, i in sorted(want, key=lambda t: (t[0], next(r.get("due") or "9" for r in st["list"]["rows"] if r["id"] == t[1]))) if gg == g]
    need(rows_of(p2) == by_due, "sorted by due: %s, want %s" % (rows_of(p2), by_due))
    select(page, "[data-tk-sort]", "rank")
    need(rows_of(page.evaluate(PROBE)) == order, "back to priority: the engine's order")
    # Filters: class, search, due; then Clear.
    select(page, "[data-tk-class]", "def-210")
    ids = rows_of(page.evaluate(PROBE))
    need(ids and all(next(r["course"] for r in st["list"]["rows"] if r["id"] == i) == "def-210" for i in ids), "class filter: %s" % ids)
    page.fill("[data-tk-q]", "midterm")
    page.wait_for_timeout(80)
    need(rows_of(page.evaluate(PROBE)) == ["task_a1b2c3d405"], "class + search: %s" % rows_of(page.evaluate(PROBE)))
    page.fill("[data-tk-q]", "zzzz")
    page.wait_for_timeout(80)
    p3 = page.evaluate(PROBE)
    need(not rows_of(p3) and p3["empty"].startswith("No tasks match"), "nothing matches: %r" % p3["empty"])
    page.click("[data-tk-clear]")
    page.wait_for_timeout(80)
    need(rows_of(page.evaluate(PROBE)) == order, "Clear brings every row back")
    select(page, "[data-tk-due]", "none")
    need(rows_of(page.evaluate(PROBE)) == ["task_a1b2c3d408"], "due filter No date")
    select(page, "[data-tk-due]", "late")
    need(rows_of(page.evaluate(PROBE)) == ["task_a1b2c3d401"], "due filter Late")
    select(page, "[data-tk-due]", "")
    # Done: the engine's closes; Undo only where undo.
    page.click('[data-tab="done"]')
    page.wait_for_timeout(80)
    p4 = page.evaluate(PROBE)
    closed = {i["id"]: i for i in p4["items"] if i.get("closed")}
    want_closed = {c["id"]: c["undo"] for c in st["closed_this_week"]}
    need(set(closed) == set(want_closed) and len(want_closed) == 2, "Done lists %s, want %s" % (sorted(closed), sorted(want_closed)))
    need(all(closed[i]["undo"] == want_closed[i] for i in closed if i in want_closed), "Undo only where the engine says undo")
    need(not rows_of(p4) or all(i.get("closed") for i in p4["items"] if "id" in i), "Done shows only closed rows")
    undo = [i for i, u in want_closed.items() if u]
    if undo:
        page.click('[data-undo-close="%s"]' % undo[0])
        page.wait_for_timeout(150)
        sent = calls(page, "set_fields")
        need(sent and sent[-1]["args"] == {"id": undo[0], "fields": {"status": "active"}, "view": "all"}, "Undo sends set_fields status active: %s" % sent[-1:])
    # All: the open groups, then Done.
    page.click('[data-tab="all"]')
    page.wait_for_timeout(80)
    p5 = page.evaluate(PROBE)
    need(groups_of(p5)[-1:] == ["Done"] and groups_of(p5)[:-1] == groups, "All: %s" % groups_of(p5))
    page.click('[data-tab="open"]')
    # New task: create_task with its fields, no view of the page's own.
    page.click("[data-tk-new]")
    page.fill('.tk-form [name="title"]', "Read chapter 4")
    page.select_option('.tk-form [name="course"]', "jkl-120")
    page.fill('.tk-form [name="due"]', "2026-09-04")
    page.fill('.tk-form [name="effort_hours"]', "1.5")
    page.click('.tk-form [type="submit"]')
    page.wait_for_timeout(150)
    made = calls(page, "create_task")
    need(made and made[-1]["args"] == {"fields": {"title": "Read chapter 4", "course": "jkl-120", "due": "2026-09-04", "effort_hours": 1.5}, "view": "all"},
         "create_task: %s" % made[-1:])
    need(page.locator(".tk-form").is_hidden(), "the form closes after an answer")
    # A due link filters that visit only (wave-A review M2): #tasks/late, away, then #tasks shows every group again.
    page.evaluate("() => { location.hash = '#tasks/late'; }")
    page.wait_for_timeout(250)
    need(page.locator("[data-tk-due]").input_value() == "late", "#tasks/late sets the due filter")
    need(groups_of(page.evaluate(PROBE)) == ["Late"], "and shows only Late")
    page.evaluate("() => { location.hash = '#home'; }")
    page.wait_for_timeout(250)
    page.evaluate("() => { location.hash = '#tasks'; }")
    page.wait_for_selector(".tk-list .tk-row", timeout=5000)
    page.wait_for_timeout(250)
    need(page.locator("[data-tk-due]").input_value() == "", "Tasks without an argument shows any due date")
    need(groups_of(page.evaluate(PROBE)) == groups, "every group again: %s" % groups_of(page.evaluate(PROBE)))


def check_unreadable(page, full, need):
    page.wait_for_selector(".tk-list .tk-row", timeout=5000)
    p = page.evaluate(PROBE)
    need("1 task note can't be read" in p["unread"] and "tasks/broken-note.md" in p["unread"], "the unreadable line: %r" % p["unread"])


def check_empty(page, full, need):
    page.wait_for_selector(".tk", timeout=5000)
    page.wait_for_timeout(150)
    st = envelope_of("t10-empty", "all")
    p = page.evaluate(PROBE)
    need(st["list"]["empty_text"] and p["empty"] == st["list"]["empty_text"], "Open says the engine's empty text: %r" % p["empty"])
    page.click('[data-tab="done"]')
    page.wait_for_timeout(80)
    need(page.evaluate(PROBE)["empty"] == st["texts"]["closed"], "Done says the engine's closed text")


CASES = [
    {"name": "tasks", "set": "t10-tasks", "route": "tasks", "check": check_tasks},
    {"name": "unreadable", "set": "base-unreadable", "route": "tasks", "check": check_unreadable, "combos": FULL},
    {"name": "empty", "set": "t10-empty", "route": "tasks", "check": check_empty, "combos": FULL},
]


def main():
    ap = argparse.ArgumentParser(description="T10's DOM check (Tasks) in headless Edge.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t10-*.json from the engine first")
    args = ap.parse_args()
    if args.make:
        make_sets(SETS)
    run("t10", CASES)


if __name__ == "__main__":
    main()
