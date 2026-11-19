"""T16 Settings frame: the DOM check, and the driver t17.py to t20.py share (v0.4.0 plan §5 T16, M12).

    python scripts/ui-fixture/checks/t16.py           # check Settings against the staged t16 envelopes
    python scripts/ui-fixture/checks/t16.py --make    # rebuild envelopes/t16-*.json first

The state envelopes are derived from base-courses-{today,all}.json (which make_base_courses.py builds from the
engine), copied unchanged: Settings reads no state of its own. The other answers are the app's commands, staged as
each command answers (app/src: account_status, get_settings, settings_context, grades_status, mail_status,
delete_my_data) and as the engine's profile read answers (engine/src/profile.rs's Profile): invented values only, no
name, no real path, no account.

  t16-settings   a signed-in member: a mailbox connected, a saved sign-in still to delete, interests editable
  t16-local      an install with no account yet: no mailbox, interests read only (a list written over lines)
  t16-none       a signed-in account with no subscription (the cloud's status "none", review I1)

Checks, for every size (820x600, 1280x800) x theme x text size, on #settings: no horizontal scroll and no console
error; the mount reads get_settings, settings_context, account_status, grades_status, mail_status and profile, and
google_status exactly once (the Sources section, T17, on Settings mount; the frame itself never); every T16 section is drawn, in order; Membership names
the entitlement and no price, and never a renewal (review I2); an account with no subscription reads No
subscription, never Active, and has no Manage subscription (review I1); the rows show the staged answers. Then, at 1280x800 light and 820x600 dark larger:
Appearance's three preferences take at once; Hide grades saves through set_settings; preferences save with expected,
then Undo; Delete my data's two steps and the forwarding sentence; #settings/data shows Data and privacy. Shots go to
target/ui-shots/t16-*.png (never committed). Exit 1 on any failure.
"""
import argparse
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIX = HERE.parent
sys.path.insert(0, str(FIX))
import serve  # noqa: E402

REPO = FIX.parent.parent
OUT = FIX / "envelopes"
SHOTS = REPO / "target" / "ui-shots"
SIZES = [(820, 600), (1280, 800)]
THEMES = ["light", "dark"]
TEXTS = ["default", "larger"]
FULL = [((1280, 800), "light", "default"), ((820, 600), "dark", "larger")]


# ---------------------------------------------------------------- envelopes, derived from the base set


def base(view):
    return json.loads((OUT / ("base-courses-%s.json" % view)).read_text(encoding="utf-8"))


def dump(path, value):
    path.write_bytes((json.dumps(value, ensure_ascii=False, indent=1, sort_keys=True) + "\n").encode("utf-8"))


def write_set(name, answers):
    """Write envelopes/<name>-{today,all}.json (the base state, unchanged) and <name>-answers.json."""
    for view in ["today", "all"]:
        dump(OUT / ("%s-%s.json" % (name, view)), base(view))
    dump(OUT / ("%s-answers.json" % name), answers)
    for view in ["today", "all", "answers"]:
        print((OUT / ("%s-%s.json" % (name, view))).relative_to(REPO).as_posix())


ACCOUNT_MEMBER = {"ok": True, "error": None, "needs_account": False, "account_id": "acct_f1x7ure01", "email": "student@example.edu",
                  "status": "active", "plan": "monthly", "current_period_end": "2026-09-24T00:00:00Z", "checked_at": "2026-08-28T08:00:00Z"}
ACCOUNT_LOCAL = {"ok": True, "error": None, "needs_account": True, "account_id": None, "email": "", "status": None, "plan": None,
                 "current_period_end": None, "checked_at": None}
SETTINGS = {"ok": True, "error": None, "settings": {"profile_id": "profile_1111111111", "backup_dir": "D:\\Knowlu backups", "autostart": True, "quit_at": None}}
SETTINGS_BARE = {"ok": True, "error": None, "settings": {"profile_id": "profile_1111111111", "backup_dir": None, "autostart": False, "quit_at": None}}
CONTEXT = {"ok": True, "error": None, "vault": "D:\\Knowlu vault", "version": "0.4.0", "profile_name": "School", "registry_error": None}
GRADES = {"ok": True, "available": True, "reason": None, "connected": True, "signed_out": False, "host": "lms.example.edu",
          "fetched_at": "2026-08-28T07:30", "hidden": False}
PROFILE = {"ok": True, "error": None, "profile": {"preferences": "Short sessions in the morning; long reading after dinner.",
                                                  "strong": ["Career fairs", "Volunteer days"], "mild": ["Concerts"], "never": ["Sports"],
                                                  "clubs": ["Chess club"], "interests_editable": True, "warnings": []}}
PROFILE_LOCKED = {"ok": True, "error": None, "profile": {"preferences": "", "strong": ["Career fairs"], "mild": [], "never": [], "clubs": [],
                                                         "interests_editable": False, "warnings": []}}
MAIL_ONE = {"ok": True, "error": None, "keep_session": True, "profile_delete_pending": True,
            "mailboxes": [{"address_id": "adr_f1x7ure01", "provider": "m365", "state": "active", "address": "fwd-f1x7ure@in.example.net",
                           "turned_off": False, "open_backfill": False, "unbound_reason": None, "suggest_rotate": False}]}
MAIL_NONE = {"ok": True, "error": None, "keep_session": True, "profile_delete_pending": False, "mailboxes": []}
ACCOUNT_NONE = dict(ACCOUNT_MEMBER, status="none", plan=None, current_period_end=None)
DELETE_REFUSED = {"ok": False, "error": "Knowlu could not reach its service, so nothing was deleted."}

SETS = {
    "t16-settings": {"account_status": ACCOUNT_MEMBER, "get_settings": SETTINGS, "settings_context": CONTEXT, "grades_status": GRADES,
                     "profile": PROFILE, "mail_status": MAIL_ONE, "delete_my_data": DELETE_REFUSED},
    "t16-local": {"account_status": ACCOUNT_LOCAL, "get_settings": SETTINGS_BARE, "settings_context": CONTEXT,
                  "profile": PROFILE_LOCKED, "mail_status": MAIL_NONE},
    "t16-none": {"account_status": ACCOUNT_NONE, "get_settings": SETTINGS, "settings_context": CONTEXT, "grades_status": GRADES,
                 "profile": PROFILE, "mail_status": MAIL_NONE},
}


def make():
    for name, answers in SETS.items():
        write_set(name, answers)


# ---------------------------------------------------------------- the driver (t17.py to t20.py import it)


READY = "() => window.__fixture && window.__fixture.events.indexOf('ui-ready') >= 0"
LAYOUT = """() => {
  const se = document.scrollingElement, c = document.getElementById('center');
  return { hscroll: se.scrollWidth > window.innerWidth || document.body.scrollWidth > window.innerWidth,
           vscroll: se.scrollHeight > window.innerHeight, centerX: c.scrollWidth > c.clientWidth };
}"""


def calls(page, cmd=None):
    got = page.evaluate("() => window.__fixture.calls")
    return [c for c in got if cmd is None or c["cmd"] == cmd]


def events(page, action=None):
    return [c["args"] for c in calls(page, "ui_event") if action is None or c["args"].get("action") == action]


def answers_of(set_name):
    p = OUT / ("%s-answers.json" % set_name)
    return json.loads(p.read_text(encoding="utf-8")) if p.is_file() else {}


def open_page(browser, base_url, set_name, route, size, theme, text, init=""):
    ctx = browser.new_context(viewport={"width": size[0], "height": size[1]}, color_scheme=theme, service_workers="block")
    ctx.add_init_script("localStorage.setItem('knowlu.theme', %s); localStorage.setItem('knowlu.text', %s);%s" % (json.dumps(theme), json.dumps(text), init))
    page = ctx.new_page()
    errors = []
    page.on("console", lambda m: errors.append("console: %s" % m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append("pageerror: %s" % e))
    page.goto("%sindex.html?set=%s#%s" % (base_url, set_name, route))
    page.wait_for_function(READY, timeout=10000)
    page.wait_for_function("() => window.__fixture.calls.some(c => c.cmd === 'state')", timeout=10000)
    page.wait_for_timeout(400)
    return ctx, page, errors


def go(page, hash_, route):
    page.evaluate("(h) => { location.hash = h; }", hash_)
    page.wait_for_function("(r) => document.documentElement.getAttribute('data-route') === r && !document.documentElement.classList.contains('vt')", arg=route, timeout=5000)
    page.wait_for_timeout(200)


def run(tag, cases):
    """cases: dicts {name, set, route, check(page, full, need), combos (optional), init (optional)}. For each combination
    the check runs, then the layout is measured and a shot is taken. Exits 1 on any failure."""
    from playwright.sync_api import sync_playwright
    server, base_url = serve.start()
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
                        ctx, page, errors = open_page(browser, base_url, case["set"], case["route"], size, theme, text, case.get("init", ""))

                        def need(ok, what):
                            if not ok:
                                fails.append(what)
                        case["check"](page, full, need)
                        lay = page.evaluate(LAYOUT)
                        need(not lay["hscroll"], "horizontal scroll")
                        need(not lay["vscroll"], "the document scrolls (only #center may)")
                        need(not lay["centerX"], "#center overflows sideways")
                        page.evaluate("() => { document.getElementById('toasts').textContent = ''; document.getElementById('center').scrollTop = 0; }")
                        page.wait_for_timeout(100)
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
    return failed


def text_of(page, sel):
    return page.evaluate("(s) => { const e = document.querySelector(s); return e ? e.textContent.replace(/\\s+/g, ' ').trim() : null; }", sel)


def shown(page, sel):
    return page.evaluate("(s) => { const e = document.querySelector(s); if (!e) return false; const r = e.getBoundingClientRect(); return !e.closest('[hidden]') && r.width > 0 && r.height > 0; }", sel)


def in_view(page, sel):
    """True when the element's top sits inside #center's visible box."""
    return page.evaluate("""(s) => { const e = document.querySelector(s), c = document.getElementById('center'); if (!e) return false;
      const r = e.getBoundingClientRect(), b = c.getBoundingClientRect(); return r.top >= b.top - 2 && r.top < b.bottom - 20; }""", sel)


# ---------------------------------------------------------------- the checks


T16_SECTIONS = ["Account", "Membership", "Grades", "Appearance", "About you", "This computer", "Data and privacy"]
SEC_PROBE = "() => Array.from(document.querySelectorAll('.st-page .st-sec > .st-h')).map(h => h.textContent.trim())"
SENTENCE = "If Knowlu cannot turn forwarding off"


def common(page, need, answers):
    page.wait_for_selector(".st-page", timeout=5000)
    page.wait_for_timeout(250)
    need(text_of(page, ".st-page h1") == "Settings", "the page's title")
    got = [c["cmd"] for c in calls(page)]
    for cmd in ["get_settings", "settings_context", "account_status", "grades_status", "mail_status", "profile"]:
        need(cmd in got, "mount reads %s" % cmd)
    heads = page.evaluate(SEC_PROBE)
    order = [h for h in heads if h in T16_SECTIONS]
    need(order == T16_SECTIONS, "T16's sections in order: %s" % heads)
    member = text_of(page, "#set-member") or ""
    need("$" not in member and "trial" not in member.lower(), "Membership names no price: %r" % member)
    st = answers["settings_context"]
    need((text_of(page, "#set-vault-path") or "") == st["vault"], "the vault path")
    need(page.get_attribute("#set-autostart-in", "aria-checked") == ("true" if answers["get_settings"]["settings"]["autostart"] else "false"), "Start with Windows")
    prefs = answers["profile"]["profile"]
    need(page.input_value("#set-prefs-ta") == prefs["preferences"], "the preferences text")
    need(page.input_value("#set-int-strong") == "\n".join(prefs["strong"]), "the interests lists")


def check_settings(page, full, need):
    a = answers_of("t16-settings")
    common(page, need, a)
    n = [c["cmd"] for c in calls(page)].count("google_status")
    need(n == 1, "google_status read once, by Sources on mount; the frame asks Google nothing: %d" % n)
    need(a["account_status"]["email"] in (text_of(page, "#set-account") or ""), "the account's email")
    need(shown(page, "#set-portal") and text_of(page, "#set-portal") == "Manage subscription", "Manage subscription")
    member = text_of(page, "#set-member") or ""
    need("Active" in member and "Through " in member and "Sep 24" in member, "the entitlement and its period's end: %r" % member)
    need("renew" not in member.lower(), "no date is called a renewal: %r" % member)
    need((text_of(page, "#set-bdir") or "") == a["get_settings"]["settings"]["backup_dir"], "the backup folder")
    need(page.get_attribute("#set-grades-hide-in", "aria-checked") == "false", "Hide grades shows grades_status's hidden")
    need(shown(page, "#set-profile-pending"), "the saved sign-in still to delete is named")
    need((text_of(page, "#set-prefs-count") or "").endswith("/ 600"), "the counter")
    if not full:
        return
    # Appearance: the three preferences take at once and are kept.
    root = "() => ({ theme: document.documentElement.getAttribute('data-theme'), font: document.documentElement.getAttribute('data-font'), text: document.documentElement.getAttribute('data-text'), keep: [localStorage.getItem('knowlu.theme'), localStorage.getItem('knowlu.font'), localStorage.getItem('knowlu.text')] })"
    was = page.evaluate(root)
    other = "light" if was["theme"] == "dark" else "dark"
    page.check('input[name="st-theme"][value="%s"]' % other)
    r = page.evaluate(root)
    need(r["theme"] == other and r["keep"][0] == other, "Theme takes at once: %s" % r)
    page.check('input[name="st-theme"][value="%s"]' % was["theme"])
    page.check('input[name="st-font"][value="dyslexic"]')
    r = page.evaluate(root)
    need(r["font"] == "dyslexic" and r["keep"][1] == "dyslexic", "OpenDyslexic takes at once: %s" % r)
    page.check('input[name="st-font"][value="default"]')
    flip = "default" if was["text"] == "larger" else "larger"
    page.check('input[name="st-text"][value="%s"]' % flip)
    r = page.evaluate(root)
    need((r["text"] == "larger") == (flip == "larger") and r["keep"][2] == flip, "Text size takes at once: %s" % r)
    page.check('input[name="st-text"][value="%s"]' % ("larger" if was["text"] == "larger" else "default"))
    # Hide grades: set_settings {grades_hidden: true}.
    page.click("#set-grades-hide-in")
    page.wait_for_timeout(150)
    sent = [c["args"] for c in calls(page, "set_settings")]
    need({"patch": {"grades_hidden": True}} in sent, "Hide grades saves through set_settings: %s" % sent)
    # Preferences: edit, Save with expected, then Undo with the re-read text as expected.
    before = page.input_value("#set-prefs-ta")
    page.fill("#set-prefs-ta", before + " Quiet afternoons.")
    need(any(e.get("objectKind") == "preferences" for e in events(page, "edit_started")), "edit_started, kind preferences")
    page.click("#set-prefs-save")
    page.wait_for_timeout(250)
    saves = [c["args"] for c in calls(page, "set_preferences")]
    need(saves and saves[0].get("expected") == before and saves[0].get("text") == before + " Quiet afternoons.", "Save sends expected and the text: %s" % saves)
    need(any("Saved" in t for t in page.evaluate("() => Array.from(document.querySelectorAll('#toasts .toast')).map(t => t.textContent)")), "a Saved toast")
    page.click("#toasts .toast button")
    page.wait_for_timeout(250)
    saves = [c["args"] for c in calls(page, "set_preferences")]
    need(len(saves) == 2 and saves[1].get("text") == before and saves[1].get("expected") == before, "Undo sends the text held before, against the re-read text: %s" % saves)
    need(not any(before in json.dumps(e) for e in events(page)), "no usage event carries the preferences text")
    # Delete my data: two steps, the forwarding sentence while a mailbox is connected; a refusal is said.
    need(not shown(page, "#set-delete-2"), "the second step waits for the first")
    page.click("#set-delete-1")
    need(shown(page, "#set-delete-2"), "the first press shows the second")
    need(SENTENCE in (text_of(page, "#set-delete-note") or ""), "the forwarding sentence")
    page.click("#set-delete-2")
    page.wait_for_timeout(200)
    need(len(calls(page, "delete_my_data")) == 1, "the second press deletes")
    need(a["delete_my_data"]["error"] in (text_of(page, "#set-delete-note") or ""), "a refusal is said in its own words")
    # #settings/data: Data and privacy in view.
    go(page, "#home", "home")
    go(page, "#settings/data", "settings")
    page.wait_for_timeout(300)
    need(in_view(page, '[data-section="data"] .st-h'), "#settings/data shows Data and privacy")
    need(shown(page, "#set-delete-1"), "Delete my data is reachable")


def check_local(page, full, need):
    a = answers_of("t16-local")
    common(page, need, a)
    need("Not attached to an account yet" in (text_of(page, "#set-account") or ""), "an install with no account says so")
    need(not shown(page, "#set-portal"), "no subscription to manage")
    need(page.evaluate("() => document.getElementById('set-int-strong').readOnly") and page.is_disabled("#set-int-save"), "the lists are read only")
    need("Knowlu can only edit a list written on one line" in (text_of(page, '[data-section="about"]') or ""), "the reason is said")
    need(not shown(page, "#set-profile-pending"), "nothing to delete is named")
    need((text_of(page, "#set-bdir") or "") == "Not set", "no backup folder")
    if not full:
        return
    page.click("#set-delete-1")
    need(SENTENCE not in (text_of(page, "#set-delete-note") or ""), "no forwarding sentence with no mailbox")


def check_none(page, full, need):
    a = answers_of("t16-none")
    common(page, need, a)
    member = text_of(page, "#set-member") or ""
    need(member.startswith("No subscription"), "no subscription says so: %r" % member)
    need("Active" not in member and "renew" not in member.lower(), "never Active, never a renewal: %r" % member)
    need(not shown(page, "#set-portal"), "no subscription to manage")


CASES = [
    {"name": "settings", "set": "t16-settings", "route": "settings", "check": check_settings},
    {"name": "local", "set": "t16-local", "route": "settings", "check": check_local},
    {"name": "none", "set": "t16-none", "route": "settings", "check": check_none},
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t16-*.json first")
    args = ap.parse_args()
    if args.make:
        make()
    sys.exit(1 if run("t16", CASES) else 0)


if __name__ == "__main__":
    main()
