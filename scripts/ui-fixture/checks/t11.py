"""The Calendar's DOM check (v0.4.0 plan T11, T12 folded in; note §4 "Calendar", ruling 3, D6).

    python scripts/ui-fixture/checks/t11.py --make    # derive envelopes/t11-calendar-*.json from the base set
    python scripts/ui-fixture/checks/t11.py           # the check: 820x600 and 1280x800, both themes, both text sizes

The envelopes are derived from `base-courses-{today,all}.json` (which make_base_courses.py builds from the engine),
never written from nothing: the base state is copied and only these invented things are added, each the way the
engine would show it:
  - `your_week` (the app's answer, `week` = the engine's commitments overview): six invented commitments, a class by
    slug (`abc-101`), a class by its code (`DEF 210`, matched through `codes`, N7), a lab, a shift and a club with no
    course, a study group whose course matches no entry (neutral), and one that ended on Aug 25;
  - `the_day`: the blocks the engine draws for today's confirmed spans (it labels a commitment's block with the
    commitment's title, clamped to the window), plus one unconfirmed calendar event; the free blocks are the base's
    (the Calendar does not read them);
  - `coming_up`: two accepted events (one with the carry's provenance) and one that is not accepted;
  - `dropped_events`: two events the engine dropped, each with its reason.

The check loads index.html?set=t11-calendar#calendar from serve.py (127.0.0.1, a port the OS picks) in headless Edge
the envelopes' the_day as the engine draws it (no block overlaps another); and, at every size, theme and text size: no console or page error, no horizontal scroll, #center the only scroller;
the page opens at 3 days below 1000 px of center, else the week; class blocks take their slot from `codes`; an
unmatched commitment and today's unconfirmed event draw neutral; a commitment's own block draws once; only accepted
events draw; the Due lane and the day's load; T/J/K/G, Shift+arrows, 1-7, Ctrl+=/-/0 and the Due fold (stored);
nothing drags, creates or deletes; Not shown lists the dropped events; check-off sends the shared check's write.
Then base-courses (no your_week staged) and base-first-run (state null) mount without an error.
Shots go to target/ui-shots/<set>-<w>x<h>-<theme>-<text>.png (never committed). Exit 1 on any failure.
"""
import argparse
import copy
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE.parent
sys.path.insert(0, str(FIXTURE))
REPO = FIXTURE.parent.parent
ENVELOPES = FIXTURE / "envelopes"
SHOTS = REPO / "target" / "ui-shots"
SET = "t11-calendar"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]
TODAY = "2026-08-28"

# ---- the derived envelopes --------------------------------------------------------------------------------------

DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
DAY_NAME = {"mon": "Mon", "tue": "Tue", "wed": "Wed", "thu": "Thu", "fri": "Fri", "sat": "Sat", "sun": "Sun"}

# (id, kind, level, title, course, meets [(days, start, end)], where, from, until)
COMMITMENTS = [
    ("cmt_f1x7ure101", "class", "hard", "ABC 101 Lecture", "abc-101", [(["mon", "wed", "fri"], "10:10", "11:00")], "Hall 2", "2026-08-24", "2026-12-11"),
    ("cmt_f1x7ure102", "class", "hard", "DEF 210 Seminar", "DEF 210", [(["tue", "thu"], "14:20", "15:35")], "Room 14", "2026-08-24", "2026-12-11"),
    ("cmt_f1x7ure103", "lab", "hard", "JKL 120 Workshop", "jkl-120", [(["mon", "wed"], "15:10", "16:00")], "Studio B", "2026-08-24", "2026-12-11"),
    ("cmt_f1x7ure104", "shift", "hard", "Campus job", None, [(["fri"], "16:30", "18:30"), (["sat"], "10:00", "14:00")], "Front desk", "2026-08-01", None),
    ("cmt_f1x7ure105", "club", "soft", "Chess club", None, [(["thu"], "18:00", "19:30")], "Room 3", None, None),
    ("cmt_f1x7ure106", "study", "optional", "Study group", "ZZZ 999", [(["wed"], "19:00", "20:00")], "Library", None, None),
    ("cmt_f1x7ure107", "club", "soft", "Summer seminar", None, [(["mon", "tue", "wed", "thu", "fri"], "12:00", "12:30")], "Room 9", "2026-08-01", "2026-08-25"),
]


def clock(hm, ampm):
    h, m = (int(x) for x in hm.split(":"))
    out = str(h % 12 or 12) + (":%02d" % m if m else "")
    return out + (("am" if h < 12 else "pm") if ampm else "")


def when(meets):
    """The engine's meets_label: `Mon/Wed/Fri 10:10–11am`, ` +1 more time` for the rest (commitments.rs)."""
    days, start, end = meets[0]
    idx = sorted(DAYS.index(d) for d in days)
    parts, i = [], 0
    while i < len(idx):
        j = i
        while j + 1 < len(idx) and idx[j + 1] == idx[j] + 1:
            j += 1
        if j - i >= 2:
            parts.append(DAY_NAME[DAYS[idx[i]]] + "–" + DAY_NAME[DAYS[idx[j]]])
        else:
            parts += [DAY_NAME[DAYS[idx[k]]] for k in range(i, j + 1)]
        i = j + 1
    split = (int(start[:2]) < 12) != (int(end[:2]) < 12)
    out = "/".join(parts) + " " + clock(start, split) + "–" + clock(end, True)
    more = len(meets) - 1
    return out + ("" if more == 0 else " +1 more time" if more == 1 else " +%d more times" % more)


def your_week():
    rows = []
    for cid, kind, level, title, course, meets, where, frm, until in COMMITMENTS:
        rows.append({
            "id": cid, "path": "commitments/%s.md" % cid, "kind": kind, "level": level, "title": title, "course": course,
            "meets": [{"days": d, "start": s, "end": e} for d, s, e in meets], "when": when(meets), "where": where,
            "from": frm, "until": until, "source_uid": None,
        })
    window = [{"day": d, "start": "08:00", "end": "18:00", "source": "template"} for d in DAYS]
    week = {"commitments": rows, "office_hours": [], "window": window, "uncovered_courses": [], "registrar": None,
            "registrar_proposals": [], "registrar_changes": {"applied": [], "ended": [], "kept": [], "status": [], "unread": []},
            "setup": False, "warnings": []}
    return {"ok": True, "error": None, "week": week, "registrar_label": None}


DROPPED = {"ok": True, "error": None, "dropped": [
    {"uid": "fixture:dropped-1", "title": "Alumni mixer", "date": "2026-08-30", "time": "18:00", "reason": "filtered by your interests"},
    {"uid": "fixture:dropped-2", "title": "Trivia night", "date": "2026-09-02", "time": "19:00", "reason": "you declined it"},
]}

COMING_UP = [
    {"uid": "fixture:event-1", "title": "Volunteer fair", "start": "2026-08-29T11:00", "when": "Sat 8/29 11:00", "location": "Student center", "organizer": "Campus life", "accepted": True},
    {"uid": "fixture:event-2", "title": "Open mic night", "start": "2026-08-29T20:00", "when": "Sat 8/29 20:00", "location": "Cafe", "organizer": "Arts club"},
    {"uid": "fixture:event-3", "title": "Career talk", "start": "2026-08-31T12:00", "when": "Mon 8/31 12:00", "location": "Hall 1", "organizer": "Careers office", "accepted": True,
     "provenance": "Accepted · from your answer to the series on Thu 8/27"},
]


def hours(a, b):
    ha, ma = (int(x) for x in a.split(":"))
    hb, mb = (int(x) for x in b.split(":"))
    return round(((hb * 60 + mb) - (ha * 60 + ma)) / 60, 2)


def mins(hm):
    return int(hm[:2]) * 60 + int(hm[3:5])


def hm(m):
    return "%02d:%02d" % (m // 60, m % 60)


def carve(blocks):
    """Each free block with the busy ones taken out, as weekcal::free_blocks subtracts each span (its hours unrounded,
    as weekcal::Block::hours is), and the block's own takes re-filled in their order into its pieces (the Calendar
    reads neither)."""
    busy = [b for b in blocks if b["kind"] != "free"]
    out = list(busy)
    for f in (b for b in blocks if b["kind"] == "free"):
        spans = [(mins(f["start"]), mins(f["end"]))]
        for b in busy:
            bs, be = mins(b["start"]), mins(b["end"])
            nxt = []
            for s, e in spans:
                if be <= s or bs >= e:
                    nxt.append((s, e))
                    continue
                if s < bs:
                    nxt.append((s, bs))
                if be < e:
                    nxt.append((be, e))
            spans = nxt
        takes = [dict(t) for t in f["takes"]]
        for s, e in spans:
            piece = {"end": hm(e), "hours": (e - s) * 60 / 3600, "kind": "free", "label": "open", "start": hm(s), "takes": []}
            room = piece["hours"]
            while takes and room > 0.05:
                t = takes[0]
                part = round(min(t["hours"], room), 1)
                piece["takes"].append({"hours": part, "of_hours": t["of_hours"], "title": t["title"]})
                room -= part
                t["hours"] = round(t["hours"] - part, 1)
                if t["hours"] <= 0:
                    takes.pop(0)
            out.append(piece)
        assert not takes, "a free block's takes no longer fit what is left of it"
    out.sort(key=lambda b: (b["start"], b["end"]))
    return out


def overlaps(blocks):
    """Pairs of the_day blocks that overlap (the engine never draws one)."""
    xs = sorted(blocks, key=lambda b: (b["start"], b["end"]))
    return [(a["label"], b["label"]) for i, a in enumerate(xs) for b in xs[i + 1:] if mins(b["start"]) < mins(a["end"])]


def derive(env):
    """One base envelope with the Calendar's invented things added the engine's way (surface::the_day, coming_up)."""
    env = copy.deepcopy(env)
    st = env["state"]
    day = st["the_day"]
    blocks = [b for b in day["blocks"]]
    # A confirmed span today draws its own block, labelled with its title and clamped to the window (08:00-18:00);
    # a template class gap with the identical span gives way to it.
    for cid, kind, level, title, course, meets, where, frm, until in COMMITMENTS:
        if (frm and frm > TODAY) or (until and until < TODAY):
            continue
        for days, s, e in meets:
            if "fri" not in days:
                continue
            s2, e2 = max(s, "08:00"), min(e, "18:00")
            if s2 >= e2:
                continue
            k = "class" if kind in ("class", "lab") else "busy"
            blocks = [b for b in blocks if not (b["kind"] == "class" and b["label"] == "class" and b["start"] == s2 and b["end"] == e2)]
            blocks.append({"end": e2, "hours": hours(s2, e2), "kind": k, "label": title, "start": s2, "takes": []})
    # One calendar event today that no commitment covers.
    blocks.append({"end": "14:00", "hours": 1.0, "kind": "busy", "label": "Dentist", "start": "13:00", "takes": []})
    day["blocks"] = carve(blocks)
    day["open_hours"] = round(sum(b["hours"] for b in day["blocks"] if b["kind"] == "free"), 1)
    st["coming_up"] = copy.deepcopy(COMING_UP)
    return env


def make():
    out = []
    for view in ["today", "all"]:
        base = json.loads((ENVELOPES / ("base-courses-%s.json" % view)).read_text(encoding="utf-8"))
        path = ENVELOPES / ("%s-%s.json" % (SET, view))
        path.write_bytes((json.dumps(derive(base), ensure_ascii=False, indent=1) + "\n").encode("utf-8"))
        out.append(path)
    answers = ENVELOPES / ("%s-answers.json" % SET)
    answers.write_bytes((json.dumps({"your_week": your_week(), "dropped_events": DROPPED}, ensure_ascii=False, indent=1) + "\n").encode("utf-8"))
    out.append(answers)
    for p in out:
        print(p.relative_to(REPO).as_posix())


# ---- the check --------------------------------------------------------------------------------------------------

PROBE = """() => {
  const se = document.scrollingElement, center = document.getElementById('center'), wk = document.getElementById('cal-wk');
  const blocks = Array.from(document.querySelectorAll('li.blk')).map(b => ({
    name: (b.querySelector('.blk-nm') || {}).textContent, date: b.closest('.day').getAttribute('data-date'),
    cls: b.className, src: b.getAttribute('data-src'), bg: getComputedStyle(b).backgroundColor, s: b.style.getPropertyValue('--s') }));
  const lanes = {};
  document.querySelectorAll('.day').forEach(d => { const l = d.querySelector('.lane'); lanes[d.getAttribute('data-date')] = l ? l.textContent : ''; });
  const loads = {};
  document.querySelectorAll('.day').forEach(d => { const l = d.querySelector('.dn-load'); loads[d.getAttribute('data-date')] = l ? l.textContent : null; });
  return {
    hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
    vscroll: se.scrollHeight > window.innerHeight,
    centerOverflowX: center.scrollWidth > center.clientWidth,
    centerW: center.clientWidth,
    route: document.documentElement.getAttribute('data-route'),
    n: wk ? +wk.getAttribute('data-n') : 0,
    dates: Array.from(document.querySelectorAll('.day')).map(d => d.getAttribute('data-date')),
    hh: wk ? parseFloat(getComputedStyle(wk).getPropertyValue('--hh')) : 0,
    blocks: blocks, lanes: lanes, loads: loads,
    title: (document.getElementById('cal-title') || {}).textContent,
    ns: (() => { const b = document.getElementById('cal-ns'); return b && !b.hidden ? b.textContent : null; })(),
    calls: window.__fixture.calls.map(c => [c.cmd, c.args]),
    zoom: localStorage.getItem('knowlu.cal.zoom'), due: localStorage.getItem('knowlu.cal.due'),
    fold: (document.getElementById('due-fold') || {getAttribute: () => null}).getAttribute('aria-expanded')
  };
}"""


def open_calendar(page, base, set_name):
    page.goto("%sindex.html?set=%s#calendar" % (base, set_name))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state' && c.args.view === 'all')", timeout=10000)
    page.wait_for_timeout(500)


def key(page, k, wait=150):
    page.keyboard.press(k)
    page.wait_for_timeout(wait)


def check_one(page, base, size, theme, text):
    fails, errors = [], []
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    open_calendar(page, base, SET)
    p = page.evaluate(PROBE)

    def need(ok, what):
        if not ok:
            fails.append(what)

    need(p["route"] == "calendar", "route is %s" % p["route"])
    need(any(c[0] == "your_week" for c in p["calls"]), "your_week asked")
    need(sum(1 for c in p["calls"] if c[0] == "dropped_events") == 1, "dropped_events asked once at mount")
    need(not p["hscroll"], "horizontal scroll")
    need(not p["vscroll"], "the document scrolls (only #center may)")
    need(not p["centerOverflowX"], "#center overflows sideways")
    want_n = 3 if p["centerW"] < 1000 else 7
    need(p["n"] == want_n and len(p["dates"]) == want_n, "opens at %d days (center %d px), shows %d %s" % (want_n, p["centerW"], p["n"], p["dates"]))
    need(TODAY in p["dates"], "today is shown at open")
    need(p["loads"].get(TODAY) == "2 h", "today's load reads %r, want '2 h' (the bucket's 2.0 h)" % p["loads"].get(TODAY))
    need("ABC 101 Reading quiz" in p["lanes"].get(TODAY, ""), "today's Due lane lists the reading quiz")
    need("Volunteer fair" in p["lanes"].get("2026-08-29", ""), "the accepted event is in Saturday's lane")
    need(not any("Open mic night" in v for v in p["lanes"].values()), "an event that is not accepted draws")
    need(p["ns"] == "Not shown (2)", "Not shown reads %r" % p["ns"])

    SHOTS.mkdir(parents=True, exist_ok=True)
    shot = SHOTS / ("%s-%dx%d-%s-%s.png" % (SET, size[0], size[1], theme, text))
    page.evaluate("() => { const c = document.getElementById('center'); const wk = document.getElementById('cal-wk'); c.scrollTop = 9.5 * parseFloat(getComputedStyle(wk).getPropertyValue('--hh')); }")
    page.wait_for_timeout(150)
    page.screenshot(path=str(shot))

    # the week: class blocks by slot, neutral ones, once each, today only for unconfirmed events
    key(page, "7")
    p = page.evaluate(PROBE)
    need(p["n"] == 7 and p["dates"][0] == "2026-08-24" and p["dates"][-1] == "2026-08-30", "7 shows Mon Aug 24 to Sun Aug 30: %s" % p["dates"])
    by = lambda name, date: [b for b in p["blocks"] if b["name"] == name and b["date"] == date]
    abc = by("ABC 101 Lecture", TODAY)
    need(len(abc) == 1 and " c1" in " " + abc[0]["cls"] + " " and abc[0]["src"] == "commitment", "ABC 101 Lecture today: one block in c1 (by slug): %s" % abc)
    defs = by("DEF 210 Seminar", "2026-08-27")
    need(len(defs) == 1 and " c2 " in " " + defs[0]["cls"] + " ", "DEF 210 Seminar takes c2 through codes: %s" % defs)
    jkl = by("JKL 120 Workshop", "2026-08-26")
    need(len(jkl) == 1 and " c4 " in " " + jkl[0]["cls"] + " ", "JKL 120 Workshop takes c4: %s" % jkl)
    sg = by("Study group", "2026-08-26")
    need(len(sg) == 1 and "cfix" in sg[0]["cls"] and "lv-optional" in sg[0]["cls"], "an unmatched commitment draws neutral, optional: %s" % sg)
    need(sg and abc and sg[0]["bg"] != abc[0]["bg"], "a class block's fill differs from a neutral one")
    need(len(by("Campus job", TODAY)) == 1, "a commitment's own block draws once today: %s" % by("Campus job", TODAY))
    dent = [b for b in p["blocks"] if b["name"] == "Dentist"]
    need(len(dent) == 1 and dent[0]["date"] == TODAY and dent[0]["src"] == "busy" and "cfix" in dent[0]["cls"], "today's unconfirmed event, once, neutral: %s" % dent)
    old = sorted(b["date"] for b in p["blocks"] if b["name"] == "Summer seminar")
    need(old == ["2026-08-24", "2026-08-25"], "a commitment draws inside from/until only: %s" % old)
    need(not any(p["loads"].get(d) for d in ["2026-08-24", "2026-08-25", "2026-08-26", "2026-08-27"]), "no load before today: %s" % p["loads"])

    # navigation
    key(page, "1")
    p = page.evaluate(PROBE)
    need(p["dates"] == [TODAY], "1 shows today alone: %s" % p["dates"])
    key(page, "j")
    need(page.evaluate(PROBE)["dates"] == ["2026-08-29"], "J moves on a day")
    key(page, "k")
    key(page, "Shift+ArrowRight")
    need(page.evaluate(PROBE)["dates"] == ["2026-08-29"], "Shift+Right moves a day")
    key(page, "Shift+ArrowLeft")
    key(page, "Shift+ArrowLeft")
    need(page.evaluate(PROBE)["dates"] == ["2026-08-27"], "Shift+Left moves back a day")
    key(page, "t")
    need(page.evaluate(PROBE)["dates"] == [TODAY], "T comes back to today")
    key(page, "3")
    p = page.evaluate(PROBE)
    need(p["dates"] == [TODAY, "2026-08-29", "2026-08-30"], "3 shows three days from today: %s" % p["dates"])
    key(page, "j")
    need(page.evaluate(PROBE)["dates"][0] == "2026-08-31", "J moves on by the days shown")
    key(page, "t")
    key(page, "g")
    need(page.evaluate("() => { const d = document.getElementById('cal-datepop'); return !!d && !d.hidden; }"), "G opens Go to date")
    page.click("#cal-datepop [data-date='2026-09-03']")
    page.wait_for_timeout(150)
    need(page.evaluate(PROBE)["dates"][0] == "2026-09-03", "Go to date moves there")
    key(page, "t")
    # zoom
    hh0 = page.evaluate(PROBE)["hh"]
    key(page, "Control+Equal")
    p = page.evaluate(PROBE)
    need(p["hh"] > hh0 and p["zoom"] == "l", "Ctrl+= makes hours taller and stores it: %s %s" % (p["hh"], p["zoom"]))
    key(page, "Control+0")
    key(page, "Control+Minus")
    p = page.evaluate(PROBE)
    need(p["hh"] < hh0 and p["zoom"] == "s", "Ctrl+- makes hours shorter: %s %s" % (p["hh"], p["zoom"]))
    key(page, "Control+0")
    need(page.evaluate(PROBE)["hh"] == hh0, "Ctrl+0 is the default height")
    # the Due fold
    page.click("#due-fold")
    page.wait_for_timeout(150)
    p = page.evaluate(PROBE)
    need(p["fold"] == "false" and p["due"] == "folded", "the Due fold folds and is stored: %s %s" % (p["fold"], p["due"]))
    page.click("#due-fold")
    page.wait_for_timeout(150)
    need(page.evaluate(PROBE)["due"] == "open", "the Due fold opens and is stored")
    # read-only: a drag moves nothing, C and Delete do nothing
    before = page.evaluate(PROBE)
    calls0 = [c for c in before["calls"] if c[0] not in ("ui_event", "state", "grades_status", "mail_status")]
    b = page.query_selector("li.blk[data-src='commitment']")
    if b:
        r = b.bounding_box()
        page.mouse.move(r["x"] + r["width"] / 2, r["y"] + 6)
        page.mouse.down()
        page.mouse.move(r["x"] + r["width"] / 2, r["y"] + 90, steps=6)
        page.mouse.up()
        page.wait_for_timeout(100)
        page.keyboard.press("Escape")
    key(page, "c")
    key(page, "Delete")
    after = page.evaluate(PROBE)
    calls1 = [c for c in after["calls"] if c[0] not in ("ui_event", "state", "grades_status", "mail_status")]
    need(calls1 == calls0, "a drag, C or Delete sent a command: %s" % calls1[len(calls0):])
    need([x["s"] for x in after["blocks"]] == [x["s"] for x in before["blocks"]], "a drag moved a block")
    # the keys are the grid's only: nothing moves while focus is in the right panel or outside #center
    grid = "() => { const w = document.getElementById('cal-wk'); return [w.getAttribute('data-n'), w.getAttribute('data-from'), w.style.getPropertyValue('--hh')]; }"
    page.click("li.blk[data-src='commitment'] .blk-b")
    page.wait_for_timeout(250)
    held = page.evaluate(grid)
    focused = page.evaluate("() => { const p = document.getElementById('panel'); const f = p && p.querySelector('button, [href], [tabindex]:not([tabindex=\"-1\"])'); if (!f) { return false; } f.focus(); return document.activeElement === f; }")
    need(focused, "a block's Details open in the panel with a control to focus")
    for k in ["1", "j", "Shift+ArrowRight", "Control+="]:
        key(page, k, 60)
    need(page.evaluate(grid) == held, "a key moved the grid while focus was in the panel: %s -> %s" % (held, page.evaluate(grid)))
    page.keyboard.press("Escape")
    page.wait_for_timeout(250)
    page.evaluate("() => { const n = document.querySelector('nav a, nav button, #side a, #side button'); if (n) { n.focus(); } }")
    key(page, "1", 60)
    need(page.evaluate(grid) == held, "a key moved the grid while focus was outside #center")
    page.evaluate("() => { if (document.activeElement) { document.activeElement.blur(); } }")
    # Not shown
    page.click("#cal-ns")
    page.wait_for_timeout(100)
    need(page.evaluate("() => { const m = document.getElementById('cal-nsmenu'); return !!m && !m.hidden && m.textContent.indexOf('Alumni mixer') >= 0 && m.textContent.indexOf('filtered by your interests') >= 0; }"), "Not shown lists the dropped events with their reasons")
    page.keyboard.press("Escape")
    # check-off: the shared control's write
    page.click(".day[data-date='%s'] .lane .ck" % TODAY)
    page.wait_for_timeout(200)
    sets = [c[1] for c in page.evaluate(PROBE)["calls"] if c[0] == "set_fields"]
    need(len(sets) == 1 and sets[0].get("fields") == {"progress": 100, "status": "done"} and sets[0].get("view") == "all", "check-off writes progress 100 and done with the view: %s" % sets)
    final = page.evaluate(PROBE)
    need(not final["hscroll"] and not final["vscroll"], "scroll after the steps")
    return fails + errors, shot


def envelope_faults():
    """What in the staged the_day the engine would never produce: overlapping blocks, or free hours not weekcal's."""
    out = []
    for view in ["today", "all"]:
        st = json.loads((ENVELOPES / ("%s-%s.json" % (SET, view))).read_text(encoding="utf-8"))["state"]
        blocks = st["the_day"]["blocks"]
        for a, b in overlaps(blocks):
            out.append("%s: %s overlaps %s" % (view, a, b))
        for b in blocks:
            if b["kind"] == "free" and b["hours"] != (mins(b["end"]) - mins(b["start"])) * 60 / 3600:
                out.append("%s: free %s-%s has hours %s" % (view, b["start"], b["end"], b["hours"]))
            if sum(t["hours"] for t in b["takes"]) > b["hours"] + 1e-9:
                out.append("%s: %s-%s takes more than it holds" % (view, b["start"], b["end"]))
    return out


def mounts_quietly(browser, base, set_name):
    ctx = browser.new_context(viewport={"width": 1280, "height": 800}, color_scheme="light", service_workers="block")
    page = ctx.new_page()
    errors = []
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    try:
        open_calendar(page, base, set_name)
        p = page.evaluate("() => ({ route: document.documentElement.getAttribute('data-route'), hscroll: document.scrollingElement.scrollWidth > window.innerWidth })")
        if p["route"] != "calendar" or p["hscroll"]:
            errors.append("%s: %s" % (set_name, p))
        SHOTS.mkdir(parents=True, exist_ok=True)
        page.screenshot(path=str(SHOTS / ("t11-%s-1280x800-light-default.png" % set_name)))
    except Exception as e:
        errors.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
    ctx.close()
    return errors


def run():
    import serve
    from playwright.sync_api import sync_playwright
    server, base = serve.start()
    failed = total = 0
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            total += 1
            bad = envelope_faults()
            if bad:
                failed += 1
                print("FAIL %s envelopes" % SET)
                for f in bad:
                    print("     " + f)
            else:
                print("ok   %s envelopes: no the_day block overlaps another, free hours as weekcal computes them" % SET)
            for size in SIZES:
                for theme in THEMES:
                    for text in TEXTS:
                        total += 1
                        ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
                        ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
                        page = ctx.new_page()
                        try:
                            fails, shot = check_one(page, base, size, theme, text)
                        except Exception as e:
                            fails, shot = ["%s: %s" % (type(e).__name__, str(e).splitlines()[0])], None
                        ctx.close()
                        label = "%s %dx%d %s %s" % (SET, size[0], size[1], theme, text)
                        if fails:
                            failed += 1
                            print("FAIL " + label)
                            for f in fails:
                                print("     " + f)
                        else:
                            print("ok   %s  %s" % (label, shot.relative_to(REPO).as_posix()))
            for other in ["base-courses", "base-first-run"]:
                total += 1
                errs = mounts_quietly(browser, base, other)
                if errs:
                    failed += 1
                    print("FAIL %s mounts the Calendar" % other)
                    for f in errs:
                        print("     " + f)
                else:
                    print("ok   %s mounts the Calendar" % other)
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


def main():
    ap = argparse.ArgumentParser(description="The Calendar's DOM check (T11).")
    ap.add_argument("--make", action="store_true", help="derive envelopes/t11-calendar-*.json from the base envelopes")
    args = ap.parse_args()
    if args.make:
        make()
    else:
        run()


if __name__ == "__main__":
    main()
