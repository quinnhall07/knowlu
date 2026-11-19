"""T13's DOM check: the Classes list and a class's page (v0.4.0 note §4 "Classes list", "Class page", D6; plan T13).

    python scripts/ui-fixture/checks/t13.py           # every t13 set, 820x600 and 1280x800, light and dark, both text sizes
    python scripts/ui-fixture/checks/t13.py --make    # rebuild envelopes/t13-*.json from the engine first (-j 2 build)
    python scripts/ui-fixture/checks/t13.py --no-build --make   # rebuild with the engine already in target/debug

The state envelopes come from the engine, never by hand: --make builds invented vaults with make_base_courses.py's own
functions (its base vault, plus the changes below), runs `knowlu-engine surface` with the oracle's pins and wraps each
answer as the app's `state` does. The grades are the engine's own `S.grades`, read from invented `grades/` notes and an
invented `state/grades.json`. The other answers (`<set>-answers.json`) are the app's commands, staged as T11 staged
them: `your_week` is T11's derived answer (its `your_week()`, the engine's overview shape) with one confirmed
office-hours commitment and one office-hours proposal added through the same `when` label; `grades_status` and
`set_settings` are the app's shapes.

| Set | The vault and the answers |
|---|---|
| t13-classes | the base, one more invented JKL 120 task that ranks above the class's sooner one, invented grade notes (ABC 101 on its Overall Grade, JKL 120 on points, DEF 210 nothing graded yet); grades shown |
| t13-hidden | the same vault; grades_status says the student hid grades |
| t13-empty | the base without its course notes: S.courses is absent |
| t13-unavailable | the base and the JKL 120 task, no grade notes; grades_status: not available at the school |
| t13-unconnected | the same vault; grades_status: available, not connected |

For every set x size x theme x text size: no console or page error, no horizontal scroll, only #center scrolls, then
the set's own assertions (CHECKS). Shots go to target/ui-shots/t13-<set>-<page>-<w>x<h>-<theme>-<text>.png.
Exit 1 on any failure.
"""
import argparse
import copy
import json
import pathlib
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIX = HERE.parent
sys.path.insert(0, str(FIX))
sys.path.insert(0, str(HERE))
import make_base_courses as mb  # noqa: E402
import serve  # noqa: E402
import t11  # noqa: E402  (T11's derived your_week and its `when` label)

REPO = FIX.parent.parent
OUT = FIX / "envelopes"
SHOTS = REPO / "target" / "ui-shots"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]
TAG = "t13"

# ---- the envelopes ------------------------------------------------------------------------------

# A big JKL 120 task due after the Essay draft: the engine ranks it first, so the first JKL row by rank is not the soonest.
EXTRA_TASKS = [
    ("jkl-120-portfolio", "task_c1a55e5101", "JKL 120 Portfolio", "jkl-120", "2026-09-04T23:59", 12.0, 5, "active", 0),
]

# Invented grade notes: (file stem, course, kind, title, category, score, possible, status, due, counts)
GRADES = [
    ("abc-101-overall", "abc-101", "overall", "Overall Grade", None, 95.2, 100, "graded", None, True),
    ("abc-101-quiz-1", "abc-101", "item", "Quiz 1", "Quizzes", 9, 10, "graded", "2026-08-21T23:59", True),
    ("abc-101-problem-set-1", "abc-101", "item", "Problem set 1", "Problem sets", 18, 20, "graded", "2026-08-21T23:59", True),
    ("abc-101-reading-quiz", "abc-101", "item", "Reading quiz", "Quizzes", None, 10, "needs-grading", "2026-08-28T23:59", True),
    ("abc-101-problem-set-2", "abc-101", "item", "Problem set 2", "Problem sets", None, 20, "not-submitted", "2026-08-27T23:59", True),
    ("abc-101-participation", "abc-101", "item", "Participation", None, None, 5, "exempt", None, False),
    ("jkl-120-outline", "jkl-120", "item", "Essay outline", "Essays", 41, 50, "graded", "2026-08-24T23:59", True),
    ("jkl-120-workshop-1", "jkl-120", "item", "Workshop 1", "Workshops", 8, 10, "graded", "2026-08-26T23:59", True),
    ("def-210-lab-1", "def-210", "item", "Lab 1", "Labs", None, 20, "needs-grading", "2026-08-26T17:00", True),
]
FETCHED_AT = "2026-08-28T13:00:00Z"

GRADES_SHOWN = {"ok": True, "available": True, "reason": None, "connected": True, "signed_out": False,
                "host": "lms.example.edu", "fetched_at": "2026-08-28T08:00", "hidden": False}
GRADES_HIDDEN = dict(GRADES_SHOWN, hidden=True)
GRADES_UNAVAILABLE = dict(GRADES_SHOWN, available=False, connected=False, reason="not available", host=None, fetched_at=None)
GRADES_UNCONNECTED = dict(GRADES_SHOWN, connected=False, reason="not connected", fetched_at=None)
SETTINGS_OK = {"ok": True, "error": None, "settings": {"backup_dir": None, "autostart": True}}

# One confirmed office-hours commitment (by slug) and one office-hours proposal (by code), the engine's shapes.
OFFICE = ("cmt_f1x7ure131", "office-hours", "optional", "ABC 101 Office hours", "abc-101", [(["tue"], "13:00", "14:00")], "Hall 2, room 210", "2026-08-24", "2026-12-11")
OFFICE_PROPOSAL = ("office-hours", "optional", "DEF 210 Office hours", "DEF 210", [(["wed"], "11:00", "12:00")], "Room 14")


def grade_note(stem, course, kind, title, category, score, possible, status, due, counts):
    lines = ["---", "type: grade", "course: " + course, "kind: " + kind, "title: " + mb.yaml_text(title)]
    if category:
        lines.append("category: " + mb.yaml_text(category))
    if score is not None:
        lines.append("score: " + str(score))
    lines += ["possible: " + str(possible), "status: " + status]
    if due:
        lines.append("due: " + due)
    if not counts:
        lines.append("counts: false")
    lines += ["created_by: fixture", "id: grd_" + stem.replace("-", "")[:20], "---", "", "Invented for the fixture harness.", ""]
    return "\n".join(lines)


def classes(root):
    for stem, tid, title, course, due, effort, imp, status, progress in EXTRA_TASKS:
        mb.write(root / "tasks" / (stem + ".md"), mb.task_note(tid, title, course, due, effort, imp, status, progress))
    for g in GRADES:
        mb.write(root / "grades" / (g[0] + ".md"), grade_note(*g))
    mb.write(root / "state" / "grades.json", json.dumps({"fetched_at": FETCHED_AT}) + "\n")


def nogrades(root):
    for stem, tid, title, course, due, effort, imp, status, progress in EXTRA_TASKS:
        mb.write(root / "tasks" / (stem + ".md"), mb.task_note(tid, title, course, due, effort, imp, status, progress))


def empty(root):
    for p in sorted((root / "courses").glob("*.md")):
        p.unlink()
    (root / "courses").rmdir()


def your_week():
    """T11's derived answer, with the office hours added the engine's way (an Overview: commitments, office_hours)."""
    yw = copy.deepcopy(t11.your_week())
    cid, kind, level, title, course, meets, where, frm, until = OFFICE
    yw["week"]["commitments"].append({
        "id": cid, "path": "commitments/%s.md" % cid, "kind": kind, "level": level, "title": title, "course": course,
        "meets": [{"days": d, "start": s, "end": e} for d, s, e in meets], "when": t11.when(meets), "where": where,
        "from": frm, "until": until, "source_uid": None,
    })
    kind, level, title, course, meets, where = OFFICE_PROPOSAL
    yw["week"]["office_hours"].append({
        "kind": kind, "level": level, "title": title, "course": course,
        "meets": [{"days": d, "start": s, "end": e} for d, s, e in meets], "when": t11.when(meets), "where": where,
        "from": None, "until": None, "source_uid": None, "window": False,
    })
    return yw


VARIANTS = [("classes", classes), ("empty", empty), ("nogrades", nogrades)]
TARGETS = {"classes": ["classes", "hidden"], "empty": ["empty"], "nogrades": ["unavailable", "unconnected"]}
ANSWERS = {
    "classes": lambda: {"grades_status": GRADES_SHOWN, "set_settings": SETTINGS_OK, "your_week": your_week()},
    "hidden": lambda: {"grades_status": GRADES_HIDDEN, "set_settings": SETTINGS_OK, "your_week": your_week()},
    "empty": lambda: {"grades_status": GRADES_SHOWN},
    "unavailable": lambda: {"grades_status": GRADES_UNAVAILABLE, "set_settings": SETTINGS_OK, "your_week": your_week()},
    "unconnected": lambda: {"grades_status": GRADES_UNCONNECTED, "set_settings": SETTINGS_OK, "your_week": your_week()},
}


def make(build):
    if build:
        import subprocess
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    written = []
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-t13-") as tmp:
        for name, change in VARIANTS:
            vault = pathlib.Path(tmp) / name
            mb.base_vault(vault)
            change(vault)
            for view in ["today", "all"]:
                env = mb.envelope(mb.surface(vault, view))
                text = json.dumps(env, ensure_ascii=False)
                for needle in [tmp, tmp.replace("\\", "/"), tmp.replace("\\", "\\\\")]:
                    if needle in text:
                        sys.exit("the engine's answer names the temporary vault")
                targets = TARGETS[name]
                for t in targets:
                    path = OUT / ("%s-%s-%s.json" % (TAG, t, view))
                    mb.dump(path, env)
                    written.append(path)
    for name, staged in ANSWERS.items():
        path = OUT / ("%s-%s-answers.json" % (TAG, name))
        mb.dump(path, staged())
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
    centerW: center.clientWidth,
    text: center.innerText,
    panel: document.getElementById('panel').hasAttribute('data-open') ? document.getElementById('panel').innerText : null,
    panelH: document.getElementById('panel-h').textContent
  };
}"""

LIST = """() => {
  const q = (s, r) => Array.from((r || document).querySelectorAll(s));
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  const vis = (el) => !!el && !el.closest('[hidden]') && el.getClientRects().length > 0 && getComputedStyle(el).display !== 'none';
  return {
    groups: q('#center .cl-group').map(g => ({ name: txt(g.querySelector('.cl-gname')), open: g.querySelector('.cl-gbtn').getAttribute('aria-expanded'),
      rows: q('.cl-row:not(.cl-head)', g).map(r => ({ code: txt(r.querySelector('.cl-code')), slot: (r.className.match(/\\bc\\d+\\b/) || [''])[0],
        href: r.querySelector('.cl-name a') ? r.querySelector('.cl-name a').getAttribute('href') : null, name: txt(r.querySelector('.cl-name')),
        next: txt(r.querySelector('.cl-next')), nextShown: vis(r.querySelector('.cl-next')), grade: txt(r.querySelector('.cl-grade')),
        dataId: r.hasAttribute('data-id') })) })),
    empty: txt(document.querySelector('#center .cl-none'))
  };
}"""

PAGE = """() => {
  const q = (s, r) => Array.from((r || document).querySelectorAll(s));
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  const col = (el) => el ? Math.round(el.getBoundingClientRect().left) : null;
  return {
    code: txt(document.querySelector('#center .cp-code')),
    title: txt(document.querySelector('#center .cp-title')),
    mark: (document.querySelector('#center .cp-meta').className.match(/\\bc\\d+\\b/) || [''])[0],
    dueFig: txt(document.getElementById('cp-due-fig')),
    due: q('#cp-due .cp-dr').map(r => ({ id: r.getAttribute('data-id'), name: txt(r.querySelector('.cp-dnm')), when: txt(r.querySelector('.cp-ddue')), slot: (r.className.match(/\\bc\\d+\\b/) || [''])[0] })),
    gradeFig: txt(document.getElementById('cp-grade-fig')),
    grade: txt(document.getElementById('cp-grade')),
    hide: !!document.querySelector('#cp-grade [data-grades-hide]'),
    show: !!document.querySelector('#cp-grade [data-grades-show]'),
    open: !!document.querySelector('#cp-grade [data-grades-open]'),
    meetsFig: txt(document.getElementById('cp-meets-fig')),
    meets: q('#cp-meets .cp-mt').map(m => ({ label: txt(m.querySelector('.cp-mlb')), where: txt(m.querySelector('.cp-mwh')), when: txt(m.querySelector('.cp-mwn')) })),
    cols: [col(document.querySelector('#center .cp-due')), col(document.querySelector('#center .cp-grade')), col(document.querySelector('#center .cp-meets'))],
    calls: window.__fixture.calls.map(c => [c.cmd, c.args])
  };
}"""


def courses_of(env):
    return (env.get("state") or {}).get("courses") or []


def active(c):
    return str(c.get("status") or "").strip() in ("", "active")


def grade_text(env, slug, hidden):
    if hidden:
        return "Hidden"
    for g in (env.get("state") or {}).get("grades") or []:
        if g["course"] == slug and isinstance(g.get("pct"), (int, float)):
            return "%.1f%%%s" % (round(g["pct"] * 10) / 10, (" " + g["letter"]) if g.get("letter") else "")
    return "No scores"


def soonest(env, slug):
    rows = [r for r in env["state"]["list"]["rows"] if r.get("course") == slug and r.get("status") != "done" and r.get("due")]
    rows.sort(key=lambda r: r["due"])
    return rows[0] if rows else None


def go(page, hash_, route):
    page.evaluate("(h) => { location.hash = h; }", hash_)
    page.wait_for_function("(r) => document.documentElement.getAttribute('data-route') === r && !document.documentElement.classList.contains('vt')", arg=route, timeout=5000)
    page.wait_for_timeout(250)


def layout(page, fails, where):
    p = page.evaluate(PROBE)
    if p["hscroll"]:
        fails.append("%s: horizontal scroll" % where)
    if p["vscroll"]:
        fails.append("%s: the document scrolls (only #center may)" % where)
    if p["centerOverflowX"]:
        fails.append("%s: #center overflows sideways" % where)
    return p


def shot(page, name, where, size, theme, text):
    SHOTS.mkdir(parents=True, exist_ok=True)
    path = SHOTS / ("%s-%s-%dx%d-%s-%s.png" % (name, where, size[0], size[1], theme, text))
    page.screenshot(path=str(path))
    return path


def list_check(page, env, fails, hidden, deep):
    p = page.evaluate(LIST)
    cs = courses_of(env)
    want_cur = [c for c in cs if active(c)]
    want_other = [c for c in cs if not active(c)]
    names = [g["name"] for g in p["groups"]]
    if names != (["Current"] + (["Not current"] if want_other else [])):
        fails.append("groups %s" % names)
        return
    cur, other = p["groups"][0], (p["groups"][1] if want_other else None)
    if cur["open"] != "true" or (other and other["open"] != "false"):
        fails.append("Current opens and Not current is closed: %s, %s" % (cur["open"], other and other["open"]))
    got = [(r["code"], r["slot"], r["href"]) for r in cur["rows"]] + ([(r["code"], r["slot"], r["href"]) for r in other["rows"]] if other else [])
    want = [(c["title"], c["slot"], "#class/" + c["slug"]) for c in want_cur + want_other]
    if got != want:
        fails.append("rows %s, want %s" % (got, want))
    rows = cur["rows"] + (other["rows"] if other else [])
    if any(r["dataId"] for r in rows):
        fails.append("a class row carries a data-id")
    for c, r in zip(want_cur + want_other, rows):
        g = grade_text(env, c["slug"], hidden)
        if r["grade"] != g:
            fails.append("%s grade reads %r, want %r" % (c["slug"], r["grade"], g))
        if r["name"] != (c.get("name") or c["title"]):
            fails.append("%s name reads %r" % (c["slug"], r["name"]))
    # Next due: the soonest open row of the class (M7), never the first by rank; it drops below 900px of center (D6)
    for c, r in zip(want_cur, cur["rows"]):
        if not r["nextShown"]:
            continue
        n = soonest(env, c["slug"])
        if n and not r["next"].startswith(n["title"]):
            fails.append("%s Next due reads %r, want the soonest %r" % (c["slug"], r["next"], n["title"]))
        if not n and r["next"]:
            fails.append("%s Next due reads %r with nothing open" % (c["slug"], r["next"]))
    center_w = page.evaluate(PROBE)["centerW"]  # the center's own CSS px, the unit its container query reads
    shown = [r["nextShown"] for r in cur["rows"]]
    if center_w <= 890 and any(shown):
        fails.append("Next due shows at %dpx of center (it drops below 900px)" % center_w)
    if center_w >= 910 and not all(shown):
        fails.append("Next due hidden at %dpx of center" % center_w)
    if hidden and "%" in page.evaluate(PROBE)["text"]:
        fails.append("a percent shows while grades are hidden")
    if deep:
        page.focus("#center .cl-row:not(.cl-head) .cl-name a")
        page.evaluate("() => KnowluData.refresh()")
        page.wait_for_timeout(400)
        kept = page.evaluate("() => { const a = document.activeElement; return !!a && a.isConnected && a.matches('#center .cl-name a'); }")
        if not kept:
            fails.append("a poll took the focus off the class row")
    if deep and want_other:
        page.click("#center .cl-group:nth-child(2) .cl-gbtn")
        page.wait_for_timeout(120)
        q = page.evaluate(LIST)
        if q["groups"][1]["open"] != "true":
            fails.append("Not current does not open")


def page_check(page, env, answers, fails, hidden, deep, size, theme, text, name):
    st = env["state"]
    abc = next(c for c in st["courses"] if c["slug"] == "abc-101")
    p = page.evaluate(PAGE)
    if (p["code"], p["title"], p["mark"]) != (abc["title"], abc["name"], abc["slot"]):
        fails.append("head reads %s" % [p["code"], p["title"], p["mark"]])
    open_rows = [r for r in st["list"]["rows"] if r.get("course") == "abc-101" and r.get("status") != "done"]
    want_ids = [r["id"] for r in sorted(open_rows, key=lambda r: (r.get("due") is None, r.get("due") or ""))]
    if [r["id"] for r in p["due"]] != want_ids:
        fails.append("Due rows %s, want %s" % ([r["id"] for r in p["due"]], want_ids))
    if any(r["slot"] != abc["slot"] for r in p["due"]):
        fails.append("a Due row is not in the class colour")
    if not p["dueFig"].startswith("%d · next " % len(want_ids)):
        fails.append("Due line reads %r" % p["dueFig"])
    if hidden:
        if p["gradeFig"] != "Hidden" or not p["show"] or p["hide"] or p["open"]:
            fails.append("hidden: the Grade line %r, Show %s, Hide %s, breakdown %s" % (p["gradeFig"], p["show"], p["hide"], p["open"]))
        if "%" in page.evaluate(PROBE)["text"]:
            fails.append("hidden: a percent shows on the page")
    else:
        g = grade_text(env, "abc-101", False)
        if p["gradeFig"] != g or g not in p["grade"] or not p["hide"] or not p["open"] or "Read " not in p["grade"]:
            fails.append("the Grade reads %r / %r (Hide %s, breakdown %s)" % (p["gradeFig"], p["grade"], p["hide"], p["open"]))
    meets_want = [("Class", "Hall 2"), ("Office hours", "Hall 2, room 210")]
    if [(m["label"], m["where"]) for m in p["meets"]] != meets_want:
        fails.append("Meets %s, want %s" % (p["meets"], meets_want))
    if not any(c[0] == "your_week" for c in p["calls"]):
        fails.append("your_week not read")
    pz = layout(page, fails, "class page")
    cols = p["cols"]
    three = cols[0] is not None and cols[0] < cols[1] < cols[2]
    if pz["centerW"] >= 1010 and not three:
        fails.append("three columns expected at %dpx of center: %s" % (pz["centerW"], cols))
    if pz["centerW"] < 990 and len(set(cols)) != 1:
        fails.append("one column expected at %dpx of center: %s" % (pz["centerW"], cols))
    path = shot(page, name, "class", size, theme, text)
    if not deep:
        return path
    if hidden:
        page.click("#cp-grade [data-grades-show]")
        page.wait_for_timeout(250)
        calls = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'set_settings').map(c => c.args)")
        if calls != [{"patch": {"grades_hidden": False}}]:
            fails.append("Show sent %s" % calls)
        return path
    # a poll leaves the Grade, and the focus on Hide, where they are
    page.focus("#cp-grade [data-grades-hide]")
    page.evaluate("() => KnowluData.refresh()")
    page.wait_for_timeout(400)
    if not page.evaluate("() => { const a = document.activeElement; return !!a && a.isConnected && a.matches('#cp-grade [data-grades-hide]'); }"):
        fails.append("a poll took the focus off Hide")
    # the breakdown opens on a click, read-only, with its foot
    page.click("#cp-grade [data-grades-open]")
    page.wait_for_selector("#panel[data-open]", timeout=3000)
    page.wait_for_timeout(200)
    # a new state with the same entry (a write's answer, adopted) never reopens it
    page.evaluate("() => { window.__gb = document.querySelector('#panel .gb-list'); }")
    page.evaluate("(id) => KnowluData.call('set_fields', { id: id, fields: {} })", want_ids[0])
    page.wait_for_timeout(300)
    if not page.evaluate("() => !!window.__gb && document.querySelector('#panel .gb-list') === window.__gb"):
        fails.append("an unchanged entry reopened the breakdown")
    q = page.evaluate(PROBE)
    body = q["panel"] or ""
    for e in [g for g in st["grades"] if g["course"] == "abc-101"][0]["entries"]:
        if e["title"] not in body:
            fails.append("the breakdown misses %s" % e["title"])
    for words in ["From Blackboard", "Overall Grade", "points so far 27 / 30", "Read ", "needs grading", "not submitted", "exempt", "not counted"]:
        if words not in body:
            fails.append("the breakdown lacks %r" % words)
    if page.evaluate("() => document.querySelectorAll('#panel input, #panel textarea, #panel [contenteditable]').length"):
        fails.append("the breakdown has an editable field")
    shot(page, name, "breakdown", size, theme, text)
    # a repaint redraws the breakdown, and never takes a task's Details panel
    page.evaluate("() => KnowluData.refresh()")
    page.wait_for_timeout(300)
    if "From Blackboard" not in (page.evaluate(PROBE)["panel"] or ""):
        fails.append("a repaint closed the breakdown")
    page.click("#cp-due .cp-dr .cp-dnm")
    page.wait_for_timeout(250)
    page.evaluate("() => KnowluData.refresh()")
    page.wait_for_timeout(400)
    q = page.evaluate(PROBE)
    if q["panel"] is None or "From Blackboard" in q["panel"]:
        fails.append("after opening a task's Details, a repaint showed the breakdown (panel %r)" % (q["panel"] or "")[:60])
    page.click("#panel-close")
    page.wait_for_timeout(150)
    # a done check fills with the class colour (A8)
    page.evaluate("() => document.querySelector('#cp-due .cp-dr .ck').setAttribute('aria-pressed', 'true')")
    page.wait_for_timeout(700)
    fill = page.evaluate("""() => {
      const b = document.querySelector('#cp-due .cp-dr .ck');
      const c = getComputedStyle(b.querySelector('circle')).fill, cc = getComputedStyle(b).getPropertyValue('--cc').trim();
      const probe = document.createElement('i'); probe.style.color = cc; document.body.appendChild(probe);
      const want = getComputedStyle(probe).color; probe.remove(); b.setAttribute('aria-pressed', 'false');
      return { fill: c, want: want };
    }""")
    if fill["fill"] != fill["want"]:
        fails.append("a done check fills %s, its class colour is %s" % (fill["fill"], fill["want"]))
    # Hide goes through set_settings, then grades_status is read again
    before = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'grades_status').length")
    page.click("#cp-grade [data-grades-hide]")
    page.wait_for_timeout(300)
    calls = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'set_settings').map(c => c.args)")
    after = page.evaluate("() => window.__fixture.calls.filter(c => c.cmd === 'grades_status').length")
    if calls != [{"patch": {"grades_hidden": True}}] or after <= before:
        fails.append("Hide sent %s and read grades_status %d more times" % (calls, after - before))
    if page.evaluate("() => document.querySelectorAll('#toasts .toast .tbtn').length"):
        fails.append("Hide left an Undo")
    return path


def other_pages(page, env, fails):
    st = env["state"]
    # DEF 210: nothing graded yet; its seminar matched by code, its office hours a proposal
    go(page, "#class/def-210", "class")
    p = page.evaluate(PAGE)
    if p["gradeFig"] != "No scores":
        fails.append("DEF 210 grade reads %r" % p["gradeFig"])
    if [(m["label"], m["where"]) for m in p["meets"]] != [("Class", "Room 14"), ("Office hours", "Room 14")]:
        fails.append("DEF 210 Meets %s" % p["meets"])
    # JKL 120: the lab, and Due soonest first though the engine ranks the portfolio first
    go(page, "#class/jkl-120", "class")
    p = page.evaluate(PAGE)
    n = soonest(env, "jkl-120")
    if not p["due"] or p["due"][0]["id"] != n["id"]:
        fails.append("JKL 120 Due starts %s, want the soonest %s" % (p["due"][:1], n["id"]))
    if [m["label"] for m in p["meets"]] != ["Lab"]:
        fails.append("JKL 120 Meets %s" % p["meets"])
    # an unknown class: its slug, an ink mark, no error
    go(page, "#class/zzz-999", "class")
    p = page.evaluate(PAGE)
    if p["code"] != "zzz-999" or p["mark"] != "":
        fails.append("an unknown class reads %r with mark %r" % (p["code"], p["mark"]))
    _ = st


UNREAD = """() => {
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  return {
    grades: Array.from(document.querySelectorAll('#center .cl-row:not(.cl-head) .cl-grade')).map(txt),
    center: document.getElementById('center').innerText
  };
}"""


def unread_check(page, env, answers, fails, size, theme, text, name):
    """Review I1: no S.grades entry and grades that cannot be read: nothing says "No scores"."""
    gs = answers["grades_status"]
    if (env["state"].get("grades") or []):
        fails.append("the unread vault has grade entries")
    p = page.evaluate(UNREAD)
    if not p["grades"] or any(p["grades"]):
        fails.append("the list's Grade cells read %s, want all blank" % p["grades"])
    if "No scores" in p["center"]:
        fails.append("the list says No scores while grades cannot be read")
    go(page, "#class/abc-101", "class")
    page.wait_for_timeout(300)
    q = page.evaluate("""() => ({
      fig: document.getElementById('cp-grade-fig').textContent.trim(),
      body: document.getElementById('cp-grade').innerText.replace(/\\s+/g, ' ').trim(),
      link: !!document.querySelector('#cp-grade a[href="#settings/sources"]'),
      controls: document.querySelectorAll('#cp-grade [data-grades-hide], #cp-grade [data-grades-show], #cp-grade [data-grades-open], #cp-grade .cp-gbig').length,
      center: document.getElementById('center').innerText
    })""")
    connect = "Connect Blackboard to see your grades"
    if q["fig"] or q["controls"] or not q["link"] or "No scores" in q["center"]:
        fails.append("the class page's Grade: line %r, %d controls, Sources link %s, body %r" % (q["fig"], q["controls"], q["link"], q["body"]))
    if (connect in q["body"]) != bool(gs["available"]):
        fails.append("the Grade body %r (Connect only when available: %s)" % (q["body"], gs["available"]))
    layout(page, fails, "class page, grades unread")
    return shot(page, name, "class", size, theme, text)


def check_one(browser, base, name, size, theme, text):
    fails, errors = [], []
    env_all = json.loads((OUT / (name + "-all.json")).read_text(encoding="utf-8"))
    answers = json.loads((OUT / (name + "-answers.json")).read_text(encoding="utf-8"))
    hidden = bool((answers.get("grades_status") or {}).get("hidden"))
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
    page = ctx.new_page()
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    shots = []
    deep = text == "default"
    try:
        page.goto("%sindex.html?set=%s#classes" % (base, name))
        page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
        page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state' && c.args.view === 'all')", timeout=10000)
        page.wait_for_selector("#center .cl", timeout=5000)
        page.wait_for_timeout(500)
        layout(page, fails, "classes")
        shots.append(shot(page, name, "classes", size, theme, text))
        if name == "t13-empty":
            p = page.evaluate(LIST)
            if p["groups"] or p["empty"] != "No classes":
                fails.append("with no course notes the list shows %s / %r" % (p["groups"], p["empty"]))
            go(page, "#class/abc-101", "class")
            layout(page, fails, "class page, no courses")
        elif name in UNREAD_SETS:
            shots.append(unread_check(page, env_all, answers, fails, size, theme, text, name))
        else:
            list_check(page, env_all, fails, hidden, deep)
            go(page, "#class/abc-101", "class")
            page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'your_week')", timeout=5000)
            page.wait_for_timeout(300)
            shots.append(page_check(page, env_all, answers, fails, hidden, deep, size, theme, text, name))
            if deep and not hidden:
                other_pages(page, env_all, fails)
    except Exception as e:  # a timeout is a failure of this combination, not of the run
        fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
    ctx.close()
    return fails + errors, [s for s in shots if s]


SETS = ["t13-classes", "t13-hidden", "t13-empty", "t13-unavailable", "t13-unconnected"]
UNREAD_SETS = ["t13-unavailable", "t13-unconnected"]


def main():
    ap = argparse.ArgumentParser(description="T13's DOM check: Classes and a class's page against the t13 envelopes.")
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t13-*.json from the engine first")
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
            for name in SETS:
                for size in SIZES:
                    for theme in THEMES:
                        for text in TEXTS:
                            total += 1
                            fails, shots = check_one(browser, base, name, size, theme, text)
                            label = "%s %dx%d %s %s" % (name, size[0], size[1], theme, text)
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
