"""T18 Your week, Day and the registrar: the DOM check (v0.4.0 plan §5 T18; note §4 Settings, §5).

    python scripts/ui-fixture/checks/t18.py           # check against the staged t18 envelopes
    python scripts/ui-fixture/checks/t18.py --make    # rebuild envelopes/t18-*.json first

The state envelopes are base-courses-{today,all}.json (make_base_courses.py builds them from the engine), copied
unchanged. The other answers are the app's commands as app/src shapes them, with the engine's parts derived the way
the engine writes them (T11's precedent): `your_week` (app/src/week.rs; `week` = commitments::overview's to_json,
`when` by the engine's meets_label rule, the registrar block as registrar.rs writes its entries),
`commitment_proposals` (commitments::proposal_value), `preview_window` (the base today state plus a `moved` line by
Moved::from_counts' rule), `commitments_confirm` (ConfirmReport::to_json) and the registrar commands
(app/src/registrar.rs). Invented values only: no name, no real school, no account; the registrar's label is the
invented "Campus portal", and the page reads it from `registrar_label`.

  t18-week          Settings: five confirmed commitments, two office hours, one uncovered course, the registrar's
                    proposals and its change list, the week's window from the template
  t18-setup         Home on the vault's first day: the sheet, with calendar proposals (classes, a lab, a shift, a
                    club, office hours and the window) and the registrar button
  t18-setup-noreg   the same first day at a school with no registrar: no registrar button

Checks, for every size (820x600, 1280x800) x theme x text size: no horizontal scroll, no console error.
  t18-week: Your week and Day drawn from your_week; the registrar button names the label; the change list's five
    headings; office hours with Add; seven day rows; each row's level and kind name the row (review M10). At 1280x800
    light and 820x600 dark larger: a level and a kind save through set_fields, and a refused level goes back (review
    M11: the check swaps in a refusal for that one press); an edit previews after 400 ms through preview_window and shows the moved line and at most
    five items without replacing the page's state; Save sends the window through commitments_confirm and says Saved;
    an office-hours Add confirms at optional; the registrar's open, I'm signed in and the found line.
  t18-setup: the sheet opens over Home once; Finish waits for the calendar read; classes start at Mine; each row's
    answer and level name the row (M10); the window proposal fills the editor. Full: Escape leaves it open; Not mine, then Finish sends mine, not_mine and the window
    and says the skipped rows; a second launch check does not raise it again.
  t18-setup-noreg: no registrar button on the sheet.
Then t16-settings (no your_week staged) shows the refusal in Your week without an error.
Shots go to target/ui-shots/t18-*.png (never committed). Exit 1 on any failure.
"""
import argparse
import copy
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import t16  # noqa: E402
from t16 import FULL, answers_of, base, calls, dump, go, run, shown, text_of  # noqa: E402,F401

OUT = t16.OUT
LABEL = "Campus portal"
DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
DAY_NAME = {"mon": "Mon", "tue": "Tue", "wed": "Wed", "thu": "Thu", "fri": "Fri", "sat": "Sat", "sun": "Sun"}

# ---------------------------------------------------------------- the engine's label rules (commitments.rs)


def clock(hm, ampm):
    h, m = (int(x) for x in hm.split(":"))
    out = str(h % 12 or 12) + (":%02d" % m if m else "")
    return out + (("am" if h < 12 else "pm") if ampm else "")


def days_label(days):
    idx = sorted(set(DAYS.index(d) for d in days))
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
    return "/".join(parts)


def meets_label(meets):
    """meets_label: `Mon/Wed/Fri 10:10–11am` from the first entry, ` +1 more time` for the rest."""
    if not meets:
        return None
    days, start, end = meets[0]
    split = (int(start[:2]) < 12) != (int(end[:2]) < 12)
    out = days_label(days) + " " + clock(start, split) + "–" + clock(end, True)
    more = len(meets) - 1
    return out + ("" if more == 0 else " +1 more time" if more == 1 else " +%d more times" % more)


def meets_json(meets):
    return [{"days": d, "end": e, "start": s} for d, s, e in meets]


def proposal(kind, level, title, course, meets, where, frm, until, uid):
    """commitments::proposal_value."""
    return {"kind": kind, "level": level, "title": title, "course": course, "meets": meets_json(meets), "when": meets_label(meets),
            "where": where, "from": frm, "until": until, "source_uid": uid, "window": uid.startswith("window:")}


# ---------------------------------------------------------------- the derived answers

# (stem, id, kind, level, title, course, meets, where, from, until, source_uid)
COMMITMENTS = [
    ("abc-101-lecture", "cmt_f1x7ure201", "class", "hard", "ABC 101 Lecture", "abc-101", [(["mon", "wed", "fri"], "10:10", "11:00")], "Hall 4",
     "2026-08-24", "2026-12-11", "registrar:campus:2026-fall:abc-101-001"),
    ("def-210-seminar", "cmt_f1x7ure202", "class", "hard", "DEF 210 Seminar", "def-210", [(["tue", "thu"], "14:20", "15:35")], "Room 14",
     "2026-08-24", "2026-12-04", "registrar:campus:2026-fall:def-210-002"),
    ("jkl-120-workshop", "cmt_f1x7ure203", "lab", "hard", "JKL 120 Workshop", "jkl-120", [(["mon", "wed"], "15:10", "16:00")], "Studio B",
     "2026-08-24", "2026-12-11", "gcal-series:f1x7ure-jkl"),
    ("campus-job", "cmt_f1x7ure204", "work", "hard", "Campus job", None, [(["fri"], "16:30", "18:30"), (["sat"], "10:00", "14:00")], "Front desk",
     "2026-08-01", None, "gcal-series:f1x7ure-job"),
    ("chess-club", "cmt_f1x7ure205", "club", "soft", "Chess club", None, [(["thu"], "18:00", "19:30")], "Room 3", None, None, "gcal-series:f1x7ure-chess"),
]

OFFICE_HOURS = [
    proposal("office-hours", "optional", "ABC 101 office hours", "abc-101", [(["tue"], "13:00", "14:00")], "Hall 4, room 210", None, None, "gcal-series:f1x7ure-oh1"),
    proposal("office-hours", "optional", "DEF 210 office hours", "def-210", [(["wed"], "11:00", "12:00")], "Room 14", None, None, "gcal-series:f1x7ure-oh2"),
]

REGISTRAR_PROPOSALS = [
    proposal("lab", "hard", "JKL 120 Lab", "jkl-120", [(["fri"], "13:00", "14:50")], "Studio C", "2026-08-24", "2026-12-11", "registrar:campus:2026-fall:jkl-120-lab"),
]

REGISTRAR_CHANGES = {
    "applied": [{"day": "2026-08-27", "field": "where", "from": "Hall 2", "note": "commitments/abc-101-lecture.md", "to": "Hall 4"}],
    "ended": [{"day": "2026-08-27", "kept_yours": True, "note": "commitments/def-210-seminar.md", "until": "2026-12-04"}],
    "kept": [{"card": "filed", "field": "meets", "note": "commitments/jkl-120-workshop.md",
              "registrar": meets_json([(["mon", "wed"], "15:30", "16:20")]), "student": meets_json([(["mon", "wed"], "15:10", "16:00")])}],
    "status": [{"key": "pqr-300-001", "note": None, "status": "waitlisted"}],
    "unread": [{"key": "stu-101-004", "note": None, "reason": LABEL + " lists this course with no class time; nothing changed"}],
}

NO_CHANGES = {"applied": [], "ended": [], "kept": [], "status": [], "unread": []}


def commitment_rows():
    rows = []
    for stem, cid, kind, level, title, course, meets, where, frm, until, uid in COMMITMENTS:
        rows.append({"id": cid, "path": "commitments/%s.md" % stem, "kind": kind, "level": level, "title": title, "course": course,
                     "meets": meets_json(meets), "when": meets_label(meets), "where": where, "from": frm, "until": until, "source_uid": uid})
    return rows


def template_window():
    return [{"day": d, "end": "18:00", "source": "template", "start": "08:00"} for d in DAYS]


def your_week(setup, label):
    if setup:
        week = {"commitments": [], "office_hours": [], "window": template_window(), "uncovered_courses": [{"slug": "jkl-120", "title": "JKL 120"}],
                "registrar": None, "registrar_proposals": [], "registrar_changes": NO_CHANGES, "setup": True, "warnings": []}
    else:
        week = {"commitments": commitment_rows(), "office_hours": OFFICE_HOURS, "window": template_window(),
                "uncovered_courses": [{"slug": "mno-140", "title": "MNO 140"}],
                "registrar": {"current": "2026-fall", "held": ["2026-fall"], "refresh": False, "school": "campus"},
                "registrar_proposals": REGISTRAR_PROPOSALS, "registrar_changes": REGISTRAR_CHANGES, "setup": False, "warnings": []}
    return {"ok": True, "error": None, "week": week, "registrar_label": label}


SETUP_PROPOSALS = [
    proposal("class", "hard", "ABC 101 Lecture", "abc-101", [(["mon", "wed", "fri"], "10:10", "11:00")], "Hall 4", None, None, "gcal-series:f1x7ure-abc"),
    proposal("class", "hard", "DEF 210 Seminar", "def-210", [(["tue", "thu"], "14:20", "15:35")], "Room 14", None, None, "gcal-series:f1x7ure-def"),
    proposal("lab", "hard", "JKL 120 Workshop", "jkl-120", [(["mon", "wed"], "15:10", "16:00")], "Studio B", None, None, "gcal-series:f1x7ure-jkl"),
    proposal("work", "hard", "Campus job", None, [(["fri"], "16:30", "18:30"), (["sat"], "10:00", "14:00")], "Front desk", None, None, "gcal-series:f1x7ure-job"),
    proposal("club", "soft", "Chess club", None, [(["thu"], "18:00", "19:30")], "Room 3", None, None, "gcal-series:f1x7ure-chess"),
    OFFICE_HOURS[0],
    proposal("planning-day", "hard", "Your day", None, [(["mon", "tue", "wed", "thu", "fri"], "07:30", "22:00"), (["sat", "sun"], "09:00", "23:00")], None, None, None,
             "window:gcal-series:f1x7ure-wake,gcal-series:f1x7ure-bed"),
]


def proposals_answer():
    return {"ok": True, "error": None, "proposals": SETUP_PROPOSALS, "uncovered_courses": [{"slug": "jkl-120", "title": "JKL 120"}], "warnings": []}


def moved_text(morning, afternoon, evening, dropped):
    """Moved::from_counts' text."""
    groups = sorted([(n, p) for n, p in [(morning, "this morning"), (afternoon, "this afternoon"), (evening, "this evening")] if n > 0], key=lambda g: -g[0])
    items = lambda n: "item" if n == 1 else "items"  # noqa: E731
    fits = lambda n: "fits" if n == 1 else "fit"  # noqa: E731
    text = ""
    for i, (n, part) in enumerate(groups):
        text += ("%d %s moved to %s" % (n, items(n), part)) if i == 0 else (", %d to %s" % (n, part))
    if dropped:
        text = ("%d %s no longer %s today" % (dropped, items(dropped), fits(dropped))) if not text else text + ("; %d no longer %s today" % (dropped, fits(dropped)))
    return {"dropped": dropped, "text": text, "to": {"afternoon": afternoon, "evening": evening, "morning": morning}}


def preview_answer():
    st = copy.deepcopy(base("today")["state"])
    st["moved"] = moved_text(0, 2, 0, 1)
    return {"ok": True, "error": None, "state": st}


def confirm_answer(window, warnings):
    return {"ok": True, "error": None, "result": {"created": 3, "declined": 1, "warnings": warnings, "window": window}, "state": base("today")["state"]}


CAPTURE = {"ok": True, "error": None, "closed": True, "result": {"rows": 3, "confirmed": 2, "proposed": 1}}
OPENED = {"ok": True, "error": None, "opened": True}
CLOSED = {"ok": True, "error": None}
SKIPPED = ["Chess club is no longer on your calendar"]


def make():
    capture = dict(CAPTURE, state=base("today")["state"])
    common = {"account_status": t16.ACCOUNT_MEMBER, "get_settings": t16.SETTINGS, "settings_context": t16.CONTEXT, "grades_status": t16.GRADES,
              "profile": t16.PROFILE, "mail_status": t16.MAIL_NONE}
    sets = {
        "t18-week": dict(common, your_week=your_week(False, LABEL), preview_window=preview_answer(), commitments_confirm=confirm_answer("updated", []),
                         open_registrar_window=OPENED, capture_registrar=capture, close_registrar_window=CLOSED),
        "t18-setup": dict(common, your_week=your_week(True, LABEL), commitment_proposals=proposals_answer(),
                          commitments_confirm=confirm_answer("created", SKIPPED), open_registrar_window=OPENED, capture_registrar=capture,
                          close_registrar_window=CLOSED),
        "t18-setup-noreg": dict(common, your_week=your_week(True, None), commitment_proposals=proposals_answer(),
                                commitments_confirm=confirm_answer("created", []), close_registrar_window=CLOSED),
    }
    for name, answers in sets.items():
        t16.write_set(name, answers)


# ---------------------------------------------------------------- the checks


def wait_week(page):
    page.wait_for_selector('.st-page [data-section="week"]', timeout=5000)
    page.wait_for_timeout(250)


def heads(page):
    return page.evaluate("() => Array.from(document.querySelectorAll('.st-page .st-sec > .st-h')).map(h => h.textContent.trim())")


def section_shot(page):
    """The two sections themselves (the driver's own shot shows the top of Settings)."""
    tag = page.evaluate("() => [innerWidth, innerHeight, document.documentElement.getAttribute('data-theme'), localStorage.getItem('knowlu.text')].join('-')")
    w, hgt, theme, text = tag.split("-")
    for sec, name in [("week", "yourweek"), ("day", "day")]:
        page.evaluate("(s) => document.querySelector('.st-page [data-section=\"' + s + '\"]').scrollIntoView({ block: 'start' })", sec)
        page.wait_for_timeout(120)
        page.screenshot(path=str(t16.SHOTS / ("t18-%s-%sx%s-%s-%s.png" % (name, w, hgt, theme, text))))
    page.evaluate("() => { document.getElementById('center').scrollTop = 0; }")


def check_week(page, full, need):
    wait_week(page)
    a = answers_of("t18-week")
    w = a["your_week"]["week"]
    need(any(c["cmd"] == "your_week" for c in calls(page)), "the section reads your_week")
    hs = heads(page)
    need("Your week" in hs and "Day" in hs, "Your week and Day are Settings sections: %s" % hs)
    need(hs.index("Your week") < hs.index("Grades") if "Grades" in hs and "Your week" in hs else False, "Your week sits before Grades")
    rows = page.evaluate("() => document.querySelectorAll('#wk-list [data-id]').length")
    need(rows == len(w["commitments"]), "one row per commitment: %d" % rows)
    need(text_of(page, "#wk-list") and "MNO 140" in text_of(page, "#wk-list") and "Knowlu will ask when it meets" in text_of(page, "#wk-list"), "the uncovered course")
    first = w["commitments"][0]
    need(first["when"] in (text_of(page, '#wk-list [data-id="%s"]' % first["id"]) or ""), "the engine's when line")
    need((text_of(page, "#wk-reg [data-reg-open]") or "") == "Refresh from " + LABEL, "the registrar button names the label: %r" % text_of(page, "#wk-reg [data-reg-open]"))
    ch = text_of(page, "#wk-changes") or ""
    for hd in ["changed", "ended", "kept yours", "status", "not read"]:
        need(("From %s — %s" % (LABEL, hd)) in ch, "the change list's heading: %s" % hd)
    need("Hall 2 → Hall 4" in ch, "a change says the old and new values")
    need("asked you in Decisions" in ch, "a kept row says the card is filed")
    need("lists it with no class time" in ch and "nothing changed" in ch, "an unread row is worded by the page")
    need(page.evaluate("() => document.querySelectorAll('#wk-oh [data-oh-add]').length") == 2, "two office hours with Add")
    need(page.evaluate("() => document.querySelectorAll('#wk-window [data-day]').length") == 7, "seven day rows")
    need(page.input_value('#wk-window [data-day="mon"] [data-part="start"]') == "08:00", "Monday starts from the template")
    need([c["cmd"] for c in calls(page)].count("google_status") == 1, "the week asks Google nothing: the one read is Sources' on Settings mount")
    ok, rows = rows_name_their_controls(page, "#wk-list .wk-row[data-id]")
    need(ok, "each row's level and kind name the row: %s" % rows[:2])
    section_shot(page)
    if not full:
        return
    # a level, then a kind, through set_fields; the level's save is refused, so it goes back
    page.evaluate("""() => { const inv = window.__TAURI__.core.invoke; window.__orig = inv;
      window.__TAURI__.core.invoke = (cmd, args) => cmd === 'set_fields' ? (window.__fixture.calls.push({ cmd: cmd, args: JSON.parse(JSON.stringify(args || {})) }), Promise.resolve({ ok: false, error: 'the note changed on disk', state: null })) : inv(cmd, args); }""")
    page.click('#wk-list [data-id="cmt_f1x7ure205"] [data-level-set="optional"]')
    page.wait_for_timeout(200)
    sent = [c["args"] for c in calls(page, "set_fields")]
    need(any(s.get("id") == "cmt_f1x7ure205" and s.get("fields") == {"level": "optional"} for s in sent), "a level saves through set_fields: %s" % sent)
    pressed = page.evaluate("""() => Array.from(document.querySelectorAll('#wk-list [data-id="cmt_f1x7ure205"] [data-level-set]')).filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.getAttribute('data-level-set'))""")
    want = [c["level"] for c in w["commitments"] if c["id"] == "cmt_f1x7ure205"]
    need(pressed == want, "a refused level goes back to %s: %s" % (want, pressed))
    page.evaluate("() => { window.__TAURI__.core.invoke = window.__orig; document.getElementById('toasts').textContent = ''; }")
    page.select_option('#wk-list [data-id="cmt_f1x7ure205"] select', "meeting")
    page.wait_for_timeout(200)
    sent = [c["args"] for c in calls(page, "set_fields")]
    need(any(s.get("fields") == {"kind": "meeting"} for s in sent), "a kind saves through set_fields: %s" % sent)
    need(all(s.get("view") == "today" for s in sent), "data.js adds the page's state view")
    # an edit previews after 400 ms
    revision = page.evaluate("() => KnowluData.state() && KnowluData.state().revision")
    page.fill('#wk-window [data-day="mon"] [data-part="end"]', "20:00")
    page.wait_for_timeout(150)
    need(not calls(page, "preview_window"), "the preview waits")
    page.wait_for_timeout(500)
    pv = calls(page, "preview_window")
    need(len(pv) == 1 and "mon" in pv[0]["args"].get("window", "") and "\"20:00\"" in pv[0]["args"]["window"], "the preview sends the sequence: %s" % pv)
    need((text_of(page, "#wk-moved") or "") == a["preview_window"]["state"]["moved"]["text"], "the preview's moved line")
    n = page.evaluate("() => document.querySelectorAll('#wk-items li').length")
    need(1 <= n <= 5, "at most five items of the previewed day: %d" % n)
    need(page.evaluate("() => KnowluData.state() && KnowluData.state().revision") == revision and page.evaluate("() => !KnowluData.state().moved"), "the preview never replaces the page's state")
    # Save sends the window
    page.click("#wk-save")
    page.wait_for_timeout(250)
    cf = [c["args"] for c in calls(page, "commitments_confirm")]
    need(cf and isinstance(cf[-1].get("confirm", {}).get("window"), str) and "20:00" in cf[-1]["confirm"]["window"], "Save sends the window: %s" % cf)
    need((text_of(page, "#wk-say") or "") == "Saved", "Save says Saved: %r" % text_of(page, "#wk-say"))
    # an office-hours Add confirms at optional
    page.click('#wk-oh [data-oh-add="gcal-series:f1x7ure-oh1"]')
    page.wait_for_timeout(250)
    cf = [c["args"] for c in calls(page, "commitments_confirm")]
    need(cf[-1].get("confirm") == {"mine": [{"source_uid": "gcal-series:f1x7ure-oh1", "level": "optional"}]}, "office hours confirm at optional: %s" % cf[-1])
    # the registrar: open, signed in, found
    page.click("#wk-reg [data-reg-open]")
    page.wait_for_timeout(200)
    need(len(calls(page, "open_registrar_window")) == 1, "open_registrar_window")
    need(shown(page, "#wk-reg [data-reg-done]") and ("Sign in to " + LABEL) in (text_of(page, "#wk-reg [data-reg-say]") or ""), "the sign-in step")
    page.click("#wk-reg [data-reg-done]")
    page.wait_for_timeout(300)
    need(len(calls(page, "capture_registrar")) == 1, "capture_registrar")
    need((text_of(page, "#wk-reg [data-reg-say]") or "") == "Found 3 classes in %s: 2 confirmed, 1 to check." % LABEL, "the found line: %r" % text_of(page, "#wk-reg [data-reg-say]"))


def sheet_open(page):
    return page.evaluate("() => !document.getElementById('sheet').hidden && !!document.querySelector('#sheet #ws-finish')")


LABELS = """() => Array.from(document.querySelectorAll(%s)).map(r => {
  const t = r.querySelector('.st-n').textContent;
  return [t, Array.from(r.querySelectorAll('[aria-label]')).map(e => e.getAttribute('aria-label'))];
})"""


def rows_name_their_controls(page, sel):
    """Every labelled control in each row names the row's title (review M10)."""
    rows = page.evaluate(LABELS % repr(sel))
    return bool(rows) and all(labels and all(t in l for l in labels) for t, labels in rows), rows


def check_setup(page, full, need):
    page.wait_for_selector("#sheet #ws-finish", timeout=5000)
    page.wait_for_timeout(300)
    need(sheet_open(page), "the sheet opens over Home")
    need("When do your classes meet?" in (text_of(page, "#sheet") or ""), "the sheet's title")
    need(len(calls(page, "commitment_proposals")) == 1, "the calendar read")
    need(not page.is_disabled("#ws-finish") and not page.is_disabled("#ws-later"), "Finish and Not now after the read settles")
    need((text_of(page, "#ws-status") or "") == "Knowlu found these on your calendar. Mark each one.", "the status line")
    mine = page.evaluate("() => Array.from(document.querySelectorAll('#ws-class-rows [data-key]')).map(r => r.getAttribute('data-answer'))")
    need(mine and all(m == "mine" for m in mine), "classes and labs start at Mine: %s" % mine)
    ok, rows = rows_name_their_controls(page, "#sheet .ws-row[data-key]")
    need(ok, "each row's answer and level name the row: %s" % rows[:2])
    need(page.input_value('#ws-window [data-day="mon"] [data-part="start"]') == "07:30", "the window proposal fills the editor")
    need(page.input_value('#ws-window [data-day="sat"] [data-part="end"]') == "23:00", "…every day of it")
    need(shown(page, "#ws-reg [data-reg-open]") and (text_of(page, "#ws-reg [data-reg-open]") or "") == "Get my class times from " + LABEL, "the registrar button under Your classes")
    need("no class times found" in (text_of(page, "#ws-class-rows") or ""), "the uncovered course")
    need(len(calls(page, "your_week")) == 1, "one launch check")
    if not full:
        return
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)
    need(sheet_open(page), "Escape leaves the sheet open")
    page.click('#ws-week-rows [data-key="gcal-series:f1x7ure-chess"] [data-answer-set="not"]')
    page.click("#ws-finish")
    page.wait_for_timeout(300)
    cf = [c["args"]["confirm"] for c in calls(page, "commitments_confirm")]
    need(len(cf) == 1, "Finish confirms once")
    if cf:
        p = cf[0]
        need("gcal-series:f1x7ure-chess" in p.get("not_mine", []), "Not mine goes in not_mine: %s" % p)
        need({"source_uid": "gcal-series:f1x7ure-abc", "level": "hard"} in p.get("mine", []), "a class goes in mine at its level")
        need(isinstance(p.get("window"), str) and "07:30" in p["window"], "the window goes with it")
    need(not sheet_open(page), "the sheet closes")
    toasts = page.evaluate("() => Array.from(document.querySelectorAll('#toasts .toast')).map(t => t.textContent)")
    need(any("will come back as questions" in t for t in toasts), "the skipped rows are said: %s" % toasts)
    go(page, "#tasks", "tasks")
    go(page, "#home", "home")
    need(len(calls(page, "your_week")) == 1 and not sheet_open(page), "never raised twice in a launch")


def check_noreg(page, full, need):
    page.wait_for_selector("#sheet #ws-finish", timeout=5000)
    page.wait_for_timeout(300)
    need(not shown(page, "#ws-reg"), "no registrar, no button")
    need(not shown(page, "#ws-reg-added"), "nothing added from a registrar")
    if full:
        page.click("#ws-later")
        page.wait_for_timeout(150)
        need(not sheet_open(page), "Not now closes the sheet")
        need(len(calls(page, "close_registrar_window")) == 1, "closing the sheet closes the school window")


def check_unstaged(page, full, need):
    wait_week(page)
    need("not staged" in (text_of(page, '[data-section="week"]') or ""), "a refused your_week is said in Your week")


CASES = [
    {"name": "week", "set": "t18-week", "route": "settings/week", "check": check_week},
    {"name": "setup", "set": "t18-setup", "route": "home", "check": check_setup},
    {"name": "setup-noreg", "set": "t18-setup-noreg", "route": "home", "check": check_noreg, "combos": FULL},
    {"name": "unstaged", "set": "t16-settings", "route": "settings", "check": check_unstaged, "combos": FULL},
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t18-*.json first")
    args = ap.parse_args()
    if args.make:
        make()
    sys.exit(1 if run("t18", CASES) else 0)


if __name__ == "__main__":
    main()
