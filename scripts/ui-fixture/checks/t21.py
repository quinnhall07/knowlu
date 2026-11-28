"""T21's DOM check: the wizard and the picker in headless Edge (v0.4.0 plan §5 T21, T22 folded in; note §6).

    python scripts/ui-fixture/checks/t21.py --make   # write envelopes/t21-*-answers.json
    python scripts/ui-fixture/checks/t21.py          # check every size x theme x text size; exit 1 on a failure

The wizard and the picker run before any vault exists, so they read no read-model envelope: the three sets here are
only `<set>-answers.json` files of the shell commands' answers (run_shell's), in the shapes app/src returns, with
invented values (an example school, `student@example.edu`, `ABC 101`). Nothing here is engine output.

For 820 x 600 and 1280 x 800, light and dark, default and larger text:

  - t21-wizard: the ten panels in order (Next walks them; the account step signs in with an emailed code, the
    logins step stores one coursework login, discovers, shows the mapping and stays once for its blank row);
    "step N of 10" follows; Back is hidden on the first panel; Finish sends create_vault with the plan and then
    finish_onboarding; the password fields are empty once stored;
  - t21-subscribe: an account with no subscription is offered the prices checkout_offer answers (v0.4.1 note D9).
    With each failing answer in envelopes/checkout-offer-failures.json (the service unreachable, a refusal, an offer
    missing four_weekly) both plan buttons are disabled beside Try again and the step says that failure's
    sentence; Try again asks again; with envelopes/checkout-offer.json the buttons read "Yearly" then "Every 4
    weeks", each with exactly the envelope's lines, the note below both, and no "trial" or trial length anywhere
    in the wizard; each button sends open_checkout with its plan, that plan's amount_cents and the offer's
    terms_version; a tester account then reads "You're all set" and "Nothing to pay" with both buttons hidden;
  - t21-picker: two profiles listed, each with Open; Open sends open_profile with that profile's id; the
    restore door's sentence is visible text;

and on every page: html.mode-boot, the sidebar and the center hidden, the band and its caption buttons drawn,
ui-ready emitted once, no console command asked (state, ui_event, mark_seen), no horizontal scroll, no console
error. Shots go to target/ui-shots/t21-*.png.
"""
import argparse
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import serve  # noqa: E402

FIX = HERE.parent
ENV = FIX / "envelopes"
REPO = FIX.parent.parent
SHOTS = REPO / "target" / "ui-shots"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]
PANELS = ["welcome", "account", "subscribe", "mail", "vault", "calendars", "logins", "gmail", "slots", "finish"]
CONSOLE_ONLY = ["state", "ui_event", "mark_seen", "grades_status", "mail_status", "google_status"]

PARENT = "C:\\Users\\student\\Knowlu"
OK = {"ok": True, "error": None}

# checkout_offer's answers (v0.4.1 note D9): an invented offer, and the three ways there is none. Staged on the page
# by stage(), not in a set: the sets here are run_shell's answers, and t20.py stages the same offer on the console.
OFFER = json.loads((ENV / "checkout-offer.json").read_text(encoding="utf-8"))
OFFER_FAILS = json.loads((ENV / "checkout-offer-failures.json").read_text(encoding="utf-8"))
# What the step says for each failure: the connection sentence, the server's own sentence, the update sentence.
FAIL_SAYS = [("unreachable", "Prices could not be loaded. Check your connection, then try again."),
             ("refused", OFFER_FAILS["refused"]["error"]),
             ("no-four-weekly", "Prices could not be loaded. Update Knowlu, then try again.")]
STAGE = """([cmd, answer]) => {
  const F = window.__fixture;
  F.staged = F.staged || {};
  F.staged[cmd] = answer;
  if (F.stageOn) { return; }
  F.stageOn = true;
  const real = window.__TAURI__.core.invoke;
  window.__TAURI__.core.invoke = function (c, args) {
    if (!Object.prototype.hasOwnProperty.call(F.staged, c)) { return real(c, args); }
    F.calls.push({ cmd: c, args: JSON.parse(JSON.stringify(args || {})) });
    return Promise.resolve(JSON.parse(JSON.stringify(F.staged[c])));
  };
}"""


def stage(page, cmd, answer):
    """From now on `cmd` answers `answer` on this page, recorded in window.__fixture.calls like every call."""
    page.evaluate(STAGE, [cmd, answer])


def lines_of(offer, plan):
    """A plan's lines as the page shows them: one text, the sentences joined by a space."""
    return " ".join(next(p["lines"] for p in offer["plans"] if p["plan"] == plan))


def launch(mode, profiles=None):
    return {"ok": True, "error": None, "mode": mode, "profiles": profiles or [], "machine": "EXAMPLE-PC",
            "tz": "America/Chicago", "default_parent": PARENT, "default_backup": PARENT + "\\Backups"}


def wizard_answers(entitlement):
    return {
        "launch_state": launch("wizard"),
        "open_policy": OK,
        "send_magic_link": OK,
        "verify_email_code": {"ok": True, "error": None, "account_id": "acct-example", "email": "student@example.edu"},
        "entitlement_now": entitlement,
        "open_checkout": OK,
        "campus_search": {"ok": True, "error": None, "hits": [
            ["100001", "Example State University", "Springfield", "IL", "canvas"],
            ["100002", "Example Community College", "Riverside", "CA", ""]]},
        "timezone_for_state": {"ok": True, "error": None, "timezone": "America/Chicago"},
        "google_connected": {"ok": True, "connected": False, "calendar": False, "gmail": False},
        "store_credentials": OK,
        "discover_coursework": {"ok": True, "error": None, "note": None, "rows": [
            {"source": "zybooks", "key": "ExampleABC101Fall2026", "detail": "Introduction to Examples", "suggested": "ABC 101", "ignored": False},
            {"source": "zybooks", "key": "ExampleDEF210Fall2026", "detail": "Example Methods", "suggested": "", "ignored": False}]},
        "create_vault": {"ok": True, "error": None, "profile": {"id": "profile-example", "name": "Knowlu", "vault": PARENT + "\\Knowlu"},
                         "restored": {"ok": True, "empty": True, "notes": 0, "records": 0, "warnings": []}},
        "finish_onboarding": OK,
    }


def sets():
    active = {"ok": True, "error": None, "status": "active", "plan": "monthly", "current_period_end": "2026-09-25T00:00:00Z"}
    none = {"ok": True, "error": None, "status": "none", "plan": None, "current_period_end": None}
    profiles = [{"id": "profile-one", "name": "Knowlu", "vault": PARENT + "\\Knowlu"},
                {"id": "profile-two", "name": "Second term", "vault": "D:\\Example\\Second term"}]
    return {
        "t21-wizard": wizard_answers(active),
        "t21-subscribe": wizard_answers(none),
        "t21-picker": {"launch_state": launch("picker", profiles), "open_profile": OK, "pick_folder": {"ok": True, "error": None, "path": None}},
    }


def make():
    for name, answers in sets().items():
        path = ENV / (name + "-answers.json")
        path.write_bytes((json.dumps(answers, indent=1, ensure_ascii=False) + "\n").encode("utf-8"))
        print("wrote " + path.relative_to(REPO).as_posix())


PROBE = """() => {
  const se = document.scrollingElement, boot = document.getElementById('boot');
  const shown = (el) => !!el && getComputedStyle(el).display !== 'none' && getComputedStyle(el).visibility !== 'hidden';
  const wide = Array.from(boot.querySelectorAll('*')).filter(el => el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflowX !== 'visible' && el.tagName !== 'INPUT' && el.tagName !== 'SELECT');
  const panel = Array.from(document.querySelectorAll('.wiz-panel')).filter(p => !p.hidden).map(p => p.id);
  return {
    modeBoot: document.documentElement.classList.contains('mode-boot'),
    bootShown: shown(boot),
    sideShown: shown(document.querySelector('.side')),
    centerShown: shown(document.getElementById('center')),
    topShown: shown(document.querySelector('.top')),
    wcShown: shown(document.querySelector('.wc')),
    hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth || boot.scrollWidth > boot.clientWidth + 1,
    wide: wide.map(el => el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') + (el.className && typeof el.className === 'string' ? '.' + el.className.split(' ').join('.') : '')),
    panel: panel,
    step: (document.getElementById('wiz-step') || {}).textContent || '',
    back: document.getElementById('wiz-back') ? document.getElementById('wiz-back').hidden : null,
    next: document.getElementById('wiz-next') ? document.getElementById('wiz-next').textContent : '',
    error: (document.getElementById('wiz-error') || {}).textContent || '',
    zoom: getComputedStyle(document.querySelector('.app')).zoom,
    calls: window.__fixture.calls.map(c => c.cmd),
    events: window.__fixture.events.slice()
  };
}"""


def open_page(browser, base, set_name, size, theme, text):
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);" % (json.dumps(theme), json.dumps(text)))
    ctx.set_default_timeout(5000)
    page = ctx.new_page()
    errors = []
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    page.goto("%sindex.html?set=%s" % (base, set_name))
    page.wait_for_function("() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0", timeout=10000)
    page.wait_for_timeout(250)
    return ctx, page, errors


def common(p, text, need):
    need(p["modeBoot"] and p["bootShown"], "#boot is not the page (mode-boot %s)" % p["modeBoot"])
    need(not p["sideShown"] and not p["centerShown"], "the sidebar or the center shows in boot mode")
    need(p["topShown"] and p["wcShown"], "the band or its caption buttons are not drawn")
    need(not p["hscroll"], "horizontal scroll")
    need(not p["wide"], "something overflows sideways: %s" % p["wide"][:4])
    need(abs(float(p["zoom"]) - (1.0 if text == "larger" else 0.88)) < 0.001, "zoom is %s" % p["zoom"])
    need(p["events"].count("ui-ready") == 1, "ui-ready emitted %d times" % p["events"].count("ui-ready"))
    asked = [c for c in p["calls"] if c in CONSOLE_ONLY]
    need(not asked, "a console command was asked: %s" % asked)


def calls_of(page, cmd):
    return page.evaluate("(c) => window.__fixture.calls.filter(x => x.cmd === c).map(x => x.args)", cmd)


def wait_panel(page, name, timeout=5000):
    page.wait_for_function("(n) => { const p = document.getElementById('wiz-' + n); return p && !p.hidden; }", arg=name, timeout=timeout)
    page.wait_for_timeout(60)


def sign_in(page):
    page.check("#wiz-18")
    page.check("#wiz-terms")
    page.fill("#wiz-email", "student@example.edu")
    page.click("#wiz-magic")
    page.wait_for_selector("#wiz-code-row:not([hidden])", timeout=5000)
    page.fill("#wiz-code", "123456")
    page.click("#wiz-code-go")


def shot(page, name):
    SHOTS.mkdir(parents=True, exist_ok=True)
    path = SHOTS / name
    page.screenshot(path=str(path))
    return path


def check_wizard(browser, base, size, theme, text):
    fails = []
    need = lambda ok, what: None if ok else fails.append(what)
    tag = "%dx%d-%s-%s" % (size[0], size[1], theme, text)
    ctx, page, errors = open_page(browser, base, "t21-wizard", size, theme, text)
    shots = []
    p = page.evaluate(PROBE)
    common(p, text, need)
    order = [x for x in page.evaluate("() => Array.from(document.querySelectorAll('.wiz-panel')).map(p => p.id)")]
    need(order == ["wiz-" + x for x in PANELS], "the panels in the markup: %s" % order)
    need(p["panel"] == ["wiz-welcome"] and p["step"] == "step 1 of 10" and p["back"] is True, "the first panel: %s %r back hidden %s" % (p["panel"], p["step"], p["back"]))
    need("Your tasks and notes live in" in page.inner_text("#wiz-privacy"), "the privacy sentence is not shown")
    shots.append(shot(page, "t21-wizard-welcome-%s.png" % tag))
    for i, name in enumerate(PANELS):
        wait_panel(page, name)
        p = page.evaluate(PROBE)
        common(p, text, need)
        need(p["panel"] == ["wiz-" + name], "panel %d is %s" % (i + 1, p["panel"]))
        need(p["step"] == "step %d of 10" % (i + 1), "panel %d reads %r" % (i + 1, p["step"]))
        need(p["back"] is (i == 0), "Back hidden %s on panel %d" % (p["back"], i + 1))
        need(p["next"] == ("Finish" if name == "finish" else "Next"), "Next reads %r on %s" % (p["next"], name))
        need(not p["error"], "an error on %s: %s" % (name, p["error"]))
        if i > 0:
            focused = page.evaluate("(n) => document.activeElement === document.querySelector('#wiz-' + n + ' h2')", name)
            need(focused, "focus is not on the %s heading (M6): %s" % (name, page.evaluate("() => document.activeElement.outerHTML.slice(0, 80)")))
        if name == "account":
            sign_in(page)
            continue  # a good code moves on to the subscribe step by itself
        if name == "calendars":
            page.fill("#wiz-school", "Example")
            page.wait_for_selector("#wiz-school-hits [data-school]", timeout=5000)
            shots.append(shot(page, "t21-wizard-calendars-%s.png" % tag))
            page.click("#wiz-school-hits [data-school='0']")
            page.wait_for_timeout(80)
            need(not page.is_disabled("#wiz-lms-open"), "Sign in to my school waits after a school was picked")
        if name == "logins":
            page.fill("#wiz-zy-user", "student@example.edu")
            page.fill("#wiz-zy-pass", "example-not-a-secret")
            page.click("#wiz-next")
            page.wait_for_selector("#wiz-map-rows [data-map]", timeout=5000)
            page.wait_for_function("() => !document.getElementById('wiz-next').disabled", timeout=5000)
            need(page.input_value("#wiz-zy-pass") == "" and page.input_value("#wiz-zy-user") == "", "the coursework login stays in its fields once stored")
            need(len(page.query_selector_all("#wiz-map-rows [data-map]")) == 2, "the mapping lists the two books")
            page.click("#wiz-next")
            page.wait_for_timeout(120)
            need(page.evaluate("() => !document.getElementById('wiz-logins').hidden"), "the first Next with a blank row stays on the panel")
            need("1 of these will be asked about in the app" in page.inner_text("#wiz-map-note"), "the blank row's cost is said")
            p = page.evaluate(PROBE)
            common(p, text, need)
            shots.append(shot(page, "t21-wizard-logins-%s.png" % tag))
        if name == "subscribe":
            need("Your subscription is active." in page.inner_text("#wiz-sub-note"), "the paid note")
            need(not page.is_visible("#wiz-sub-month") and not page.is_visible("#wiz-sub-year"), "an entitled account sees the plan buttons")
            need(not calls_of(page, "checkout_offer"), "an entitled account is not offered prices")
        if name == "finish":
            need("Knowlu" in page.inner_text("#wiz-summary"), "the summary names the folder")
            shots.append(shot(page, "t21-wizard-finish-%s.png" % tag))
            page.click("#wiz-next")
            page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'finish_onboarding')", timeout=5000)
            plan = calls_of(page, "create_vault")
            need(len(plan) == 1, "create_vault sent %d times" % len(plan))
            if plan:
                pl = plan[0].get("plan", {})
                need(plan[0].get("name") == "Knowlu", "create_vault's name %r" % plan[0].get("name"))
                need(pl.get("offer_mail") is True and pl.get("zybooks") is True and pl.get("vhl") is False, "the plan's flags: %s" % {k: pl.get(k) for k in ("offer_mail", "zybooks", "vhl")})
                need(pl.get("zybooks_courses") == [{"code": "ExampleABC101Fall2026", "course": "ABC 101", "label": "ABC 101"}], "the mapping: %s" % pl.get("zybooks_courses"))
                need(pl.get("zybooks_ignore") == [], "a blank row is never sent as ignored: %s" % pl.get("zybooks_ignore"))
                need((pl.get("campus_choice") or {}).get("unitid") == "100001", "the school: %s" % pl.get("campus_choice"))
            need(calls_of(page, "finish_onboarding") == [{"id": "profile-example"}], "finish_onboarding's id")
            need("no vault yet" in page.inner_text("#wiz-restore-note"), "the restore note")
            break
        page.click("#wiz-next")
    stored = calls_of(page, "store_credentials")
    need(len(stored) == 1 and stored[0].get("vault") == PARENT + "\\Knowlu" and stored[0].get("source") == "zybooks", "store_credentials: %s" % [{k: v for k, v in s.items() if k != "secret"} for s in stored])
    ctx.close()
    return fails + errors, shots


def check_subscribe(browser, base, size, theme, text):
    fails = []
    need = lambda ok, what: None if ok else fails.append(what)
    tag = "%dx%d-%s-%s" % (size[0], size[1], theme, text)
    ctx, page, errors = open_page(browser, base, "t21-subscribe", size, theme, text)
    stage(page, "checkout_offer", OFFER_FAILS[FAIL_SAYS[0][0]])
    page.click("#wiz-next")
    wait_panel(page, "account")
    sign_in(page)
    wait_panel(page, "subscribe")
    page.wait_for_function("() => !document.getElementById('wiz-sub-month').hidden", timeout=5000)
    # No offer: each failure leaves both buttons disabled beside Try again, with that failure's sentence.
    shot_wait = None
    for i, (name, says) in enumerate(FAIL_SAYS):
        if i:
            stage(page, "checkout_offer", OFFER_FAILS[name])
            page.click("#wiz-sub-retry")
        page.wait_for_function("(n) => window.__fixture.calls.filter(c => c.cmd === 'checkout_offer').length === n", arg=i + 1, timeout=5000)
        page.wait_for_function("(s) => document.getElementById('wiz-sub-why').textContent === s", arg=says, timeout=5000)
        need(page.is_disabled("#wiz-sub-year") and page.is_disabled("#wiz-sub-month"), "%s: the plan buttons wait" % name)
        need(page.is_visible("#wiz-sub-retry"), "%s: Try again is offered" % name)
        need(page.inner_text("#wiz-sub-why").strip() == says, "%s: the step says %r" % (name, page.inner_text("#wiz-sub-why")))
        need("$" not in page.evaluate("() => document.getElementById('wizard').textContent"), "%s: a price shows with no offer" % name)
        if i == 0:
            shot_wait = shot(page, "t21-subscribe-wait-%s.png" % tag)
        p = page.evaluate(PROBE)
        common(p, text, need)
    # The offer: the two plans, Yearly first, each with exactly the envelope's lines, and the note below both.
    stage(page, "checkout_offer", OFFER)
    page.click("#wiz-sub-retry")
    page.wait_for_function("() => !document.getElementById('wiz-sub-year').disabled && !document.getElementById('wiz-sub-month').disabled", timeout=5000)
    p = page.evaluate(PROBE)
    common(p, text, need)
    need(page.inner_text("#wiz-sub-year").strip() == "Yearly" and page.inner_text("#wiz-sub-month").strip() == "Every 4 weeks", "the plan buttons read %r / %r" % (page.inner_text("#wiz-sub-year"), page.inner_text("#wiz-sub-month")))
    first = page.evaluate("() => !!(document.getElementById('wiz-sub-year').compareDocumentPosition(document.getElementById('wiz-sub-month')) & Node.DOCUMENT_POSITION_FOLLOWING)")
    need(first, "Yearly comes before Every 4 weeks")
    need(page.inner_text("#wiz-sub-year-lines").strip() == lines_of(OFFER, "yearly"), "the Yearly lines read %r" % page.inner_text("#wiz-sub-year-lines"))
    need(page.inner_text("#wiz-sub-month-lines").strip() == lines_of(OFFER, "four_weekly"), "the Every 4 weeks lines read %r" % page.inner_text("#wiz-sub-month-lines"))
    need(page.inner_text("#wiz-sub-offer-note").strip() == OFFER["note"], "the note reads %r" % page.inner_text("#wiz-sub-offer-note"))
    need(not page.is_visible("#wiz-sub-retry") and not page.inner_text("#wiz-sub-why").strip(), "Try again and its sentence go once the offer is held")
    words = page.evaluate("() => document.getElementById('wizard').textContent")
    rest = words
    for line in [lines_of(OFFER, "yearly"), lines_of(OFFER, "four_weekly")]:
        rest = rest.replace(line, "")
    need("$" not in rest, "a price the offer did not send")
    for bad in ["trial", "7-day", "seven day", "14-day", "14 days", "per month", "/month"]:
        need(bad not in words.lower(), "the wizard shows %r" % bad)
    shot_path = shot(page, "t21-subscribe-%s.png" % tag)
    page.click("#wiz-sub-year")
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'open_checkout')", timeout=5000)
    page.click("#wiz-sub-month")
    page.wait_for_function("() => window.__fixture.calls.filter(c => c.cmd === 'open_checkout').length === 2", timeout=5000)
    cents = {p["plan"]: p["amount_cents"] for p in OFFER["plans"]}
    want = [{"plan": "yearly", "amountCents": cents["yearly"], "termsVersion": OFFER["terms_version"]},
            {"plan": "four_weekly", "amountCents": cents["four_weekly"], "termsVersion": OFFER["terms_version"]}]
    need(calls_of(page, "open_checkout") == want, "open_checkout sent %s" % calls_of(page, "open_checkout"))
    need("Waiting for the payment page" in page.inner_text("#wiz-sub-note"), "the waiting note")
    # The tester account: the poll's next tick reads the comped plan and moves on; Back shows the nothing-to-pay form.
    page.evaluate("""() => {
      const real = window.__TAURI__.core.invoke;
      window.__TAURI__.core.invoke = function (cmd, args) {
        if (cmd === 'entitlement_now') { window.__fixture.calls.push({ cmd: cmd, args: args || {} }); return Promise.resolve({ ok: true, error: null, status: 'active', plan: 'tester', current_period_end: null }); }
        return real(cmd, args);
      };
    }""")
    wait_panel(page, "mail", timeout=8000)
    page.click("#wiz-back")
    wait_panel(page, "subscribe")
    need(page.inner_text("#wiz-subscribe h2").strip() == "You're all set", "the tester heading %r" % page.inner_text("#wiz-subscribe h2"))
    need(page.inner_text("#wiz-sub-note").strip() == "Your account is already set up. Nothing to pay.", "the tester note %r" % page.inner_text("#wiz-sub-note"))
    need(not page.is_visible("#wiz-sub-month") and not page.is_visible("#wiz-sub-year") and not page.is_visible("#wiz-subscribe .lede"), "a tester sees the plan buttons or the lede")
    need("$" not in page.inner_text("#wiz-subscribe") and not page.is_visible("#wiz-sub-retry"), "a tester sees a price or Try again")
    p = page.evaluate(PROBE)
    common(p, text, need)
    ctx.close()
    return fails + errors, [x for x in (shot_wait, shot_path) if x]


def check_picker(browser, base, size, theme, text):
    fails = []
    need = lambda ok, what: None if ok else fails.append(what)
    tag = "%dx%d-%s-%s" % (size[0], size[1], theme, text)
    ctx, page, errors = open_page(browser, base, "t21-picker", size, theme, text)
    page.wait_for_selector("#pick-list [data-profile]", timeout=5000)
    p = page.evaluate(PROBE)
    common(p, text, need)
    rows = page.evaluate("() => Array.from(document.querySelectorAll('#pick-list [data-profile]')).map(r => [r.getAttribute('data-profile'), r.textContent])")
    need([r[0] for r in rows] == ["profile-one", "profile-two"], "the picker lists %s" % [r[0] for r in rows])
    need(all("Open" in r[1] for r in rows), "each row has Open")
    need(page.inner_text("#pick-lede").strip() == "Which vault?", "the lede %r" % page.inner_text("#pick-lede"))
    need(page.is_visible("#pick-restore-why") and "never from your account" in page.inner_text("#pick-restore-why"), "the restore door's sentence is not visible")
    need(page.evaluate("() => !document.querySelector('#picker [title]')"), "a title tooltip in the picker")
    described = page.evaluate("""() => Array.from(document.querySelectorAll('#pick-list [data-profile]')).map(r => {
      const b = r.querySelector('[data-open]'), d = document.getElementById(b.getAttribute('aria-describedby') || '');
      return !!d && r.contains(d) && d.classList.contains('pick-name') ? d.textContent : null;
    })""")
    need(described == ["Knowlu", "Second term"], "each Open is described by its row's name (M7): %s" % described)
    shot_path = shot(page, "t21-picker-%s.png" % tag)
    page.click("#pick-list [data-profile='profile-two'] [data-open]")
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'open_profile')", timeout=5000)
    need(calls_of(page, "open_profile") == [{"id": "profile-two"}], "Open sent %s" % calls_of(page, "open_profile"))
    ctx.close()
    return fails + errors, [shot_path]


def main():
    ap = argparse.ArgumentParser(description="T21's DOM check: the wizard and the picker.")
    ap.add_argument("--make", action="store_true", help="write envelopes/t21-*-answers.json and stop")
    args = ap.parse_args()
    if args.make:
        make()
        return
    from playwright.sync_api import sync_playwright
    server, base = serve.start()
    total = failed = 0
    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(channel="msedge", headless=True)
            for check in (check_wizard, check_subscribe, check_picker):
                for size in SIZES:
                    for theme in THEMES:
                        for text in TEXTS:
                            total += 1
                            try:
                                fails, shots = check(browser, base, size, theme, text)
                            except Exception as e:  # a timeout fails this combination, not the run
                                fails, shots = ["%s: %s" % (type(e).__name__, str(e).splitlines()[0])], []
                            label = "%s %dx%d %s %s" % (check.__name__[6:], size[0], size[1], theme, text)
                            if fails:
                                failed += 1
                                print("FAIL " + label, flush=True)
                                for f in fails:
                                    print("     " + f, flush=True)
                            else:
                                print("ok   %s  %s" % (label, "  ".join(s.relative_to(REPO).as_posix() for s in shots)), flush=True)
            browser.close()
    finally:
        server.shutdown()
    print("%d of %d passed" % (total - failed, total))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
