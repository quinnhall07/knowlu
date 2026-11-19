"""The minute moves between revisions (v0.4.0 whole-branch review I2; note §4 Shell, Home, Calendar; plan T5, T8, T11).

    python scripts/ui-fixture/checks/t05.py    # 1280x800 light, Home on t08-day and the Calendar on t11-calendar

The engine's `revision` leaves `now` out, so a poll whose revision is unchanged still carries a later minute. The
check sets window.__fixture.now (serve.py's stub then answers `state` with that `now` and the same revision) and polls
through KnowluData.refresh(). It reuses the t08-day and t11-calendar envelopes; it makes none of its own.

  Home (t08-day, now 09:00; ABC 101 10:10 to 11:00, Shift 16:30 to 18:00):
    - 09:00 -> 09:01: the band's clock and the aside's now mark read 9:01 AM, the mark is the same node (no redraw),
      the revision on screen is the same, and no KnowluData.on listener ran;
    - 09:01 -> 11:30: the clock reads 11:30 AM, ABC 101 is past ("1 earlier"), Shift is next with the now mark above
      it, and keyboard focus stays on Remove from my day.
  Calendar (t11-calendar):
    - 09:00 -> 11:30: today's now line and the gutter's now label move to 11:30 (11.5 h), ABC 101 Lecture turns past,
      the grid is the same node (no repaint) and focus stays on the focused block.

No console or page error at any step. Exit 1 on any failure.
"""
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIX = HERE.parent
sys.path.insert(0, str(FIX))
import serve  # noqa: E402

SIZE = (1280, 800)

HOME = """() => {
  const txt = (el) => el ? el.textContent.replace(/\\s+/g, ' ').trim() : '';
  const vis = (el) => !!el && !el.closest('[hidden]') && el.getClientRects().length > 0;
  const mark = document.querySelector('#ha-sched .ha-now');
  const after = mark ? mark.nextElementSibling : null;
  const next = document.querySelector('#ha-sched .ha-sc.is-next');
  const f = document.activeElement;
  return {
    clock: Array.from(document.getElementById('clock').children).map(txt).join(' '),
    now: txt(mark),
    before: after ? after.getAttribute('data-from') : null,
    next: next ? next.getAttribute('data-from') : null,
    past: Array.from(document.querySelectorAll('#ha-sched .ha-sc.is-past')).map(li => li.getAttribute('data-from')),
    early: vis(document.querySelector('[data-early]')) ? txt(document.querySelector('[data-early]')) : null,
    sameMark: !!mark && mark === window.__t05mark,
    focus: f ? f.getAttribute('data-lane-remove') : null,
    rev: KnowluData.state() ? KnowluData.state().revision : null,
    painted: window.__t05on
  };
}"""

CAL = """() => {
  const today = document.querySelector('#cal-wk .day.is-today');
  const line = today ? today.querySelector('.now') : null;
  const label = document.querySelector('#cal-wk .hl.is-now');
  const abc = Array.from(today ? today.querySelectorAll('.blk') : []).find(b => (b.querySelector('.blk-nm') || {}).textContent === 'ABC 101 Lecture');
  return {
    clock: Array.from(document.getElementById('clock').children).map(c => c.textContent).join(' '),
    line: line ? line.style.getPropertyValue('--now') : null,
    label: label ? label.textContent : null,
    labelAt: label ? label.style.getPropertyValue('--h') : null,
    abcPast: abc ? abc.classList.contains('is-past') : null,
    sameGrid: !!window.__t05grid && document.querySelector('#cal-wk .day.is-today') === window.__t05grid,
    focus: document.activeElement === window.__t05focus && !!window.__t05focus,
    painted: window.__t05on
  };
}"""


def ready(page, base, set_name, route, view):
    page.goto("%sindex.html?set=%s#%s" % (base, set_name, route))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state' && c.args.view === %s)" % json.dumps(view), timeout=10000)
    page.wait_for_timeout(500)
    page.evaluate("() => { window.__t05on = 0; KnowluData.on(function () { window.__t05on += 1; }); }")


def tick(page, now):
    page.evaluate("(n) => { window.__fixture.now = n; return KnowluData.refresh(); }", now)
    page.wait_for_timeout(300)


def home(page, base, fails):
    ready(page, base, "t08-day", "home", "today")
    page.wait_for_selector("#center .home #ha-sched .ha-now", timeout=5000)
    p = page.evaluate(HOME)
    if p["clock"] != "Fri Aug 28 9 AM" or p["now"] != "9 AM" or p["next"] != "10:10" or p["before"] != "10:10":
        fails.append("Home at 09:00: %s" % p)
    rev = p["rev"]
    page.evaluate("() => { window.__t05mark = document.querySelector('#ha-sched .ha-now'); }")
    tick(page, "2026-08-28T09:01")
    p = page.evaluate(HOME)
    if p["clock"] != "Fri Aug 28 9:01 AM":
        fails.append("the band's clock reads %r after a poll at 09:01 with the same revision" % p["clock"])
    if p["now"] != "9:01 AM":
        fails.append("Home's now mark reads %r after a poll at 09:01" % p["now"])
    if not p["sameMark"]:
        fails.append("a minute with nothing past redrew the schedule (the now mark is a new node)")
    if p["rev"] != rev or p["painted"]:
        fails.append("an unchanged revision repainted: revision %r -> %r, on listeners ran %r times" % (rev, p["rev"], p["painted"]))
    page.focus("[data-lane-remove]")
    tick(page, "2026-08-28T11:30")
    p = page.evaluate(HOME)
    if p["clock"] != "Fri Aug 28 11:30 AM":
        fails.append("the band's clock reads %r after a poll at 11:30" % p["clock"])
    if p["now"] != "11:30 AM" or p["before"] != "16:30" or p["next"] != "16:30":
        fails.append("at 11:30 the now mark reads %r above %r and the next block is %r, want 11:30 AM above Shift (16:30)" % (p["now"], p["before"], p["next"]))
    if p["past"] != ["10:10"] or p["early"] != "1 earlier":
        fails.append("at 11:30 ABC 101 is not past: past %s, early %r" % (p["past"], p["early"]))
    if p["focus"] != "fixture:lane-1":
        fails.append("focus left Remove from my day when the minute moved (now on %r)" % p["focus"])
    if p["painted"]:
        fails.append("on listeners ran %r times with no new revision" % p["painted"])


def calendar(page, base, fails):
    ready(page, base, "t11-calendar", "calendar", "all")
    page.wait_for_selector("#cal-wk .day.is-today .now", timeout=5000)
    p = page.evaluate(CAL)
    if p["line"] != "9" or p["label"] != "9 AM" or p["abcPast"] is not False:
        fails.append("Calendar at 09:00: %s" % p)
    page.evaluate("() => { window.__t05grid = document.querySelector('#cal-wk .day.is-today'); const b = window.__t05grid.querySelector('.blk-b'); b.focus(); window.__t05focus = b; }")
    tick(page, "2026-08-28T11:30")
    p = page.evaluate(CAL)
    if p["line"] != "11.5" or p["labelAt"] != "11.5" or p["label"] != "11:30 AM":
        fails.append("the Calendar's now line did not move to 11:30: line %r, label %r at %r" % (p["line"], p["label"], p["labelAt"]))
    if p["abcPast"] is not True:
        fails.append("ABC 101 Lecture (10:10 to 11:00) is not past at 11:30")
    if not p["sameGrid"]:
        fails.append("the minute repainted the Calendar's grid")
    if not p["focus"]:
        fails.append("focus left the focused block when the minute moved")
    if p["painted"]:
        fails.append("on listeners ran %r times with no new revision" % p["painted"])


def main():
    from playwright.sync_api import sync_playwright
    server, base = serve.start()
    total = failed = 0
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            for name, fn in [("home", home), ("calendar", calendar)]:
                total += 1
                fails, errors = [], []
                ctx = browser.new_context(viewport={"width": SIZE[0], "height": SIZE[1]}, color_scheme="light", service_workers="block")
                page = ctx.new_page()
                page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
                page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
                try:
                    fn(page, base, fails)
                except Exception as e:  # a timeout is a failure of this page, not of the run
                    fails.append("%s: %s" % (type(e).__name__, str(e).splitlines()[0]))
                ctx.close()
                fails += errors
                label = "t05 %s %dx%d light" % (name, SIZE[0], SIZE[1])
                if fails:
                    failed += 1
                    print("FAIL " + label)
                    for f in fails:
                        print("     " + f)
                else:
                    print("ok   " + label)
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
