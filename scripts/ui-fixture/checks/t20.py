"""T20 the account check, the upgrade sheet, privacy re-consent and the report: the DOM check (v0.4.0 plan §5 T20; note
§5 "Sign-in, upgrade", "Privacy re-consent", "Report a problem").

    python scripts/ui-fixture/checks/t20.py           # check against the staged t20 envelopes
    python scripts/ui-fixture/checks/t20.py --make    # rebuild envelopes/t20-*.json first

The state envelopes are the base set's, unchanged (make_base_courses.py builds them from the engine): t20-upgrade,
t20-report and t20-report-failed carry base-courses, t20-reconsent carries base-reconsent-owed. The answers are the app's commands as
app/src shapes them (account.rs: account_status, entitlement_now, google_sign_in, open_checkout, open_policy,
privacy_status, accept_privacy; report.rs: report_preview, report_send), plus T16's Settings reads. Invented values
only.

Checks, for every size (820x600, 1280x800) x theme x text size: no horizontal scroll, no console error.
  upgrade (Home, no account): the sheet "Knowlu now needs an account" is up and names #sheet (review M9), Google before
    the email field, two policy links, Not now, no price; the email field's fill differs from the sheet's in both
    themes (review M1); at 1280x800 light and 820x600 dark larger: Continue with Google with the boxes unticked
    says why and sends nothing; ticked, it sends google_sign_in {ageAttested: true}, asks entitlement_now and shows
    Subscribe; a policy link sends open_policy; Escape dismisses it and it stays down across a route change.
  upgrade-settings (Settings, no account): the sheet waits while Settings is the route; leaving Settings raises it.
  reconsent (Home, signed in, privacy owed): the panel is in #layer with line 1 from the reply's current, in a polite
    live region (review M4), the shipped sentence after the policy link (M3), Accept, Settings and the policy link, and no
    dismiss; the page stays usable (not inert, Home's rows behind it, the sidebar routes with it up); at the two full
    sizes: Escape leaves it up; Settings routes to Data and privacy with the panel down, Delete my data free and Sync
    now locked with the owed tip; Home brings it back; Accept sends accept_privacy and takes it down, and Sync now is
    free again.
  report (Settings > Data and privacy): the Report a problem row; at the two full sizes: the sheet shows the preview
    text, an edit is sent byte for byte by report_send, "Sent. Thank you." then the sheet closes; the tray's
    KNOWLU_OPEN_REPORT from Home routes to Data and privacy and opens the sheet.
  report-failed (Settings > Data and privacy, the two full sizes): a preview that could not be built leaves Send off and
    offers Try again, which asks for the preview again (review M6).
Shots go to target/ui-shots/t20-*.png (never committed); t20-reportsheet-* is the open report at the two full sizes. Exit 1 on any failure.
"""
import argparse
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import t16  # noqa: E402
from t16 import FULL, calls, dump, go, run, shown, text_of  # noqa: E402,F401

OUT = t16.OUT
NOT_ENTITLED = {"ok": True, "error": None, "status": None, "plan": None, "current_period_end": None}
SIGNED_IN = {"ok": True, "error": None, "account_id": "acct_f1x7ure01", "email": "student@example.edu"}
PREVIEW = ("Knowlu 0.4.0 on Windows\nview: today\nlast slot: ok\nwarnings: 0\nmail: [email]\n\nWhat went wrong:\n")
EDIT = "The Today list did not refresh after a sync."
PREVIEW_FAILED = "the report could not be read from this computer"
SETTINGS_READS = {"get_settings": t16.SETTINGS, "settings_context": t16.CONTEXT, "grades_status": t16.GRADES, "profile": t16.PROFILE,
                  "mail_status": t16.MAIL_NONE}


def load(name):
    return json.loads((OUT / name).read_text(encoding="utf-8"))


def make():
    t16.write_set("t20-upgrade", dict(SETTINGS_READS, account_status=t16.ACCOUNT_LOCAL, entitlement_now=NOT_ENTITLED, google_sign_in=SIGNED_IN,
                                      open_policy={"ok": True, "error": None}))
    t16.write_set("t20-report", dict(SETTINGS_READS, account_status=t16.ACCOUNT_MEMBER, report_preview={"ok": True, "error": None, "text": PREVIEW},
                                     report_send={"ok": True, "error": None}))
    t16.write_set("t20-report-failed", dict(SETTINGS_READS, account_status=t16.ACCOUNT_MEMBER, report_preview={"ok": False, "error": PREVIEW_FAILED}))
    owed = load("base-reconsent-owed-answers.json")
    for view in ["today", "all"]:
        dump(OUT / ("t20-reconsent-%s.json" % view), load("base-reconsent-owed-%s.json" % view))
    dump(OUT / "t20-reconsent-answers.json", dict(SETTINGS_READS, account_status=t16.ACCOUNT_MEMBER, privacy_status=owed["privacy_status"],
                                                  accept_privacy={"ok": True, "error": None, "recorded": owed["privacy_status"]["current"]},
                                                  open_policy={"ok": True, "error": None}))
    for p in ["t20-reconsent-today.json", "t20-reconsent-all.json", "t20-reconsent-answers.json"]:
        print((OUT / p).relative_to(t16.REPO).as_posix())


# ---------------------------------------------------------------- the checks


def sheet_up(page):
    return page.evaluate("() => !document.getElementById('sheet').hidden")


def layer_up(page):
    return page.evaluate("() => { const l = document.getElementById('layer'); return !l.hidden && !!l.querySelector('#rc-accept'); }")


def check_upgrade(page, full, need):
    page.wait_for_selector("#sheet #up-h", timeout=5000)
    need(text_of(page, "#up-h") == "Knowlu now needs an account", "the upgrade sheet: %r" % text_of(page, "#up-h"))
    order = page.evaluate("() => { const g = document.getElementById('up-google'), e = document.getElementById('up-email'); return !!(g.compareDocumentPosition(e) & Node.DOCUMENT_POSITION_FOLLOWING); }")
    need(order, "Continue with Google comes before the email field")
    need(page.evaluate("() => document.querySelectorAll('#sheet [data-policy]').length") == 2, "the two policy links")
    need(shown(page, "#up-later"), "Not now")
    body = text_of(page, "#sheet") or ""
    need("$" not in body and " trial" not in body.lower(), "no price in the sheet")
    need(page.evaluate("() => document.getElementById('up-subscribe').hidden"), "Subscribe waits for the entitlement answer")
    need(page.get_attribute("#sheet", "aria-labelledby") == "up-h", "#sheet is named by the sheet's heading: %r" % page.get_attribute("#sheet", "aria-labelledby"))
    fills = page.evaluate("() => [getComputedStyle(document.getElementById('up-email')).backgroundColor, getComputedStyle(document.querySelector('#sheet .sheet-in')).backgroundColor]")
    need(fills[0] != fills[1], "the email field's fill differs from the sheet's: %s" % fills)
    if not full:
        return
    page.click("#up-google")
    page.wait_for_timeout(150)
    need("Tick both boxes" in (text_of(page, "#up-error") or ""), "unticked boxes say why")
    need(not calls(page, "google_sign_in"), "nothing is sent unticked")
    page.click('#sheet [data-policy="terms"]')
    page.wait_for_timeout(150)
    pol = [c["args"] for c in calls(page, "open_policy")]
    need(pol == [{"which": "terms"}], "a policy link sends open_policy: %s" % pol)
    page.check("#up-18")
    page.check("#up-terms")
    page.click("#up-google")
    page.wait_for_timeout(300)
    gs = [c["args"] for c in calls(page, "google_sign_in")]
    need(gs == [{"ageAttested": True}], "google_sign_in {ageAttested: true}: %s" % gs)
    need(len(calls(page, "entitlement_now")) == 1, "the entitlement is asked after the sign-in")
    need(shown(page, "#up-subscribe"), "a not-entitled answer shows Subscribe")
    page.keyboard.press("Escape")
    page.wait_for_timeout(200)
    need(not sheet_up(page), "Escape dismisses the sheet")
    go(page, "#tasks", "tasks")
    go(page, "#home", "home")
    need(not sheet_up(page), "a dismissed sheet stays down")


def check_upgrade_settings(page, full, need):
    page.wait_for_selector(".st-page", timeout=5000)
    page.wait_for_timeout(250)
    need(not sheet_up(page), "the upgrade sheet waits while Settings is the route")
    need(len(calls(page, "account_status")) >= 1, "the account was asked")
    go(page, "#home", "home")
    page.wait_for_timeout(150)
    need(sheet_up(page) and shown(page, "#up-h"), "leaving Settings raises it")


def check_reconsent(page, full, need):
    page.wait_for_function("() => !!document.querySelector('#layer #rc-accept')", timeout=5000)
    page.wait_for_timeout(100)  # line 1 is written into its live region a frame after the panel is drawn
    need(layer_up(page), "the re-consent panel is in #layer")
    lines = page.evaluate("() => Array.from(document.querySelectorAll('#rc-changes p')).map(p => p.textContent)")
    need(lines and lines[0] == "Knowlu's privacy policy changed on 2026-10-20.", "line 1 from the reply's current: %s" % lines[:1])
    need(len(lines) == 4 and "Gmail" in lines[1], "the three change lines")
    live = page.evaluate("() => Array.from(document.querySelectorAll('#layer [aria-live=\"polite\"]')).map(e => e.textContent)")
    need(any(lines[0] in t for t in live), "line 1 is announced politely: %s" % live)
    need("Read the privacy policy. Today's page is underneath and still works." in (text_of(page, "#layer .rc-read") or ""), "the shipped sentence: %r" % text_of(page, "#layer .rc-read"))
    need(shown(page, "#rc-settings") and shown(page, '#layer [data-policy="privacy"]'), "Settings and the policy link")
    low = (text_of(page, "#layer") or "").lower()
    need(not any(w in low for w in ["not now", "close", "later", "dismiss", "skip"]), "no way to dismiss it")
    need(page.evaluate("() => !document.getElementById('center').hasAttribute('inert') && document.getElementById('sheet').hidden"), "Home is not inert")
    need(page.evaluate("() => document.querySelectorAll('#center [data-id]').length") >= 1, "Home shows its rows behind the panel")
    # The page stays usable: a route change through the sidebar with the panel up, and the panel covers a corner, never
    # the whole page.
    box = page.evaluate("() => { const r = document.querySelector('#layer .layer-in').getBoundingClientRect(); return [r.width, r.height, innerWidth, innerHeight]; }")
    need(box[0] < box[2] - 52 and not page.evaluate("() => document.getElementById('layer').hasAttribute('aria-modal')"), "the panel is not modal and leaves the page beside it: %s" % box)
    page.click('#side a[data-route="tasks"]')
    page.wait_for_function("() => document.documentElement.getAttribute('data-route') === 'tasks'", timeout=5000)
    need(layer_up(page), "the page is usable with the panel up, and the panel stays")
    go(page, "#home", "home")
    need(len(calls(page, "privacy_status")) == 1, "privacy_status once")
    if not full:
        return
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)
    need(layer_up(page), "Escape leaves the panel up")
    page.click("#rc-settings")
    page.wait_for_function("() => document.documentElement.getAttribute('data-route') === 'settings'", timeout=5000)
    page.wait_for_timeout(500)
    need(not layer_up(page), "the panel stands down over Settings")
    need(shown(page, "#set-delete-1") and not page.is_disabled("#set-delete-1"), "Delete my data is reachable and free")
    sync = page.evaluate("() => { const b = document.querySelector('[data-sync]'); return b ? [b.disabled, !!b.closest('.owed')] : null; }")
    need(sync == [True, True], "Sync now is locked with the owed tip: %s" % sync)
    go(page, "#home", "home")
    page.wait_for_timeout(150)
    need(layer_up(page), "leaving Settings brings the panel back")
    page.click("#rc-accept")
    page.wait_for_timeout(300)
    need(len(calls(page, "accept_privacy")) == 1, "Accept sends accept_privacy")
    need(not layer_up(page), "Accept takes the panel down")
    need(page.evaluate("() => !KnowluKit.isOwed()"), "the owed flag is clear")
    go(page, "#settings/status", "settings")
    page.wait_for_timeout(400)
    need(page.evaluate("() => { const b = document.querySelector('[data-sync]'); return !!b && !b.disabled && !b.closest('.owed'); }"), "Sync now is free again")
    go(page, "#home", "home")
    need(not layer_up(page), "an accepted update stays accepted")


def check_report(page, full, need):
    page.wait_for_selector("#set-report", timeout=5000)
    page.wait_for_timeout(200)
    need(shown(page, "#set-report-go"), "the Report a problem row")
    need("Report a problem" in (text_of(page, "#set-report") or ""), "its name")
    if not full:
        return
    page.click("#set-report-go")
    page.wait_for_selector("#sheet #report-text", timeout=5000)
    page.wait_for_timeout(200)
    need(page.input_value("#report-text") == PREVIEW, "the sheet shows the preview text")
    tag = page.evaluate("() => [innerWidth, innerHeight, document.documentElement.getAttribute('data-theme'), localStorage.getItem('knowlu.text')].join('-')")
    page.screenshot(path=str(t16.SHOTS / ("t20-reportsheet-%s.png" % tag)))
    need(len(calls(page, "report_preview")) == 1 and calls(page, "report_preview")[0]["args"].get("view") == "today", "report_preview with the page's view")
    page.focus("#report-text")
    page.keyboard.press("Control+End")
    page.keyboard.type(EDIT)
    page.click("#report-send")
    page.wait_for_timeout(250)
    sent = [c["args"] for c in calls(page, "report_send")]
    need(sent == [{"text": PREVIEW + EDIT}], "report_send sends exactly what is on screen: %s" % sent)
    need((text_of(page, "#report-note") or "") == "Sent. Thank you.", "a sent report says so")
    page.wait_for_timeout(1700)
    need(not sheet_up(page), "the sheet closes after a send")
    go(page, "#home", "home")
    page.evaluate("() => window.KNOWLU_OPEN_REPORT()")
    page.wait_for_selector("#sheet #report-text", timeout=5000)
    need(page.evaluate("() => location.hash") == "#settings/data", "the tray's report routes to Data and privacy")
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)
    need(not sheet_up(page), "Escape closes the report")


def check_report_failed(page, full, need):
    page.wait_for_selector("#set-report", timeout=5000)
    page.wait_for_timeout(200)
    page.click("#set-report-go")
    page.wait_for_selector("#sheet #report-text", timeout=5000)
    page.wait_for_timeout(250)
    need(PREVIEW_FAILED in (text_of(page, "#report-note") or ""), "the failure is said: %r" % text_of(page, "#report-note"))
    need(page.is_disabled("#report-send"), "Send stays off")
    need(shown(page, "#report-retry"), "Try again")
    page.click("#report-retry")
    page.wait_for_timeout(250)
    need(len(calls(page, "report_preview")) == 2, "Try again asks for the preview again")
    need(page.is_disabled("#report-send") and shown(page, "#report-retry"), "a second failure still leaves Send off, with Try again")
    page.keyboard.press("Escape")
    page.wait_for_timeout(150)


CASES = [
    {"name": "upgrade", "set": "t20-upgrade", "route": "home", "check": check_upgrade},
    {"name": "upgrade-settings", "set": "t20-upgrade", "route": "settings/account", "check": check_upgrade_settings},
    {"name": "reconsent", "set": "t20-reconsent", "route": "home", "check": check_reconsent},
    {"name": "report", "set": "t20-report", "route": "settings/data", "check": check_report},
    {"name": "report-failed", "set": "t20-report-failed", "route": "settings/data", "check": check_report_failed, "combos": FULL},
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t20-*.json first")
    args = ap.parse_args()
    if args.make:
        make()
    sys.exit(1 if run("t20", CASES) else 0)


if __name__ == "__main__":
    main()
