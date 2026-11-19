"""T17 Sources and mail: the DOM check (v0.4.0 plan §5 T17; note §4 "Settings" Sources row, §5 mail setup, §11 item 4).

    python scripts/ui-fixture/checks/t17.py                     # check against the staged t17 envelopes
    python scripts/ui-fixture/checks/t17.py --make              # rebuild envelopes/t17-*.json first (-j 2 engine build)
    python scripts/ui-fixture/checks/t17.py --make --no-build   # rebuild with the engine already in target/debug

The state envelopes come from the engine, never by hand: --make builds invented vaults with make_base_courses.py's own
functions (its base vault plus the changes below), runs `knowlu-engine surface` with the oracle's pins and wraps each
answer as the app's `state` does. The `mail_applied` answer is the engine's own `mail-applied` output over the same
vault after invented mail writes made through `knowlu-engine write` and `info open` as `agent:knowlu.mail` (its `ts`,
`since` and new ids come from the clock and the id generator, so a rebuild changes those bytes and nothing else). The
other answers are the app's commands, staged as each command answers (app/src: account_status, google_status,
grades_status, mail_status, mail_setup_progress, mail_setup_start, ...): invented values only, no name, no real path,
no account.

| Set | The vault and the answers |
|---|---|
| t17-sources | the base, invented grade notes and a source heads-up (`gmail-source:quiet`); a signed-in member, Gmail connected, Blackboard connected, two mailboxes (one asks for a rotate), What came from email |
| t17-local | the base unchanged; an install with no account, Blackboard not offered at the school, no mailbox |
| t17-setup | the base unchanged; the wizard's mail offer taken and no setup running, a guided Gmail setup waiting on the student |

Checks, for every size (820x600, 1280x800) x theme x text size: no console or page error, no horizontal scroll, only
#center scrolls, then each case's own assertions (CASES); the full interaction runs at 1280x800 light and 820x600 dark
larger. The setup and owed cases hold the launch's account_status answer (HOLD_ACCOUNT) until the check releases it, so
the first open is seen to wait for the account check (M10; review M2: KnowluAccount.after, however long it takes). The
mail sheet names #sheet and nests no dialog of its own (review M9). Shots go to target/ui-shots/t17-<case>-<w>x<h>-<theme>-<text>.png (never committed). Exit 1 on any failure.
"""
import argparse
import json
import pathlib
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
FIX = HERE.parent
sys.path.insert(0, str(FIX))
sys.path.insert(0, str(HERE))
import make_base_courses as mb  # noqa: E402
import t16  # noqa: E402  (the shared driver)
from t16 import answers_of, calls, events, go, shown, text_of  # noqa: E402

REPO = FIX.parent.parent
OUT = FIX / "envelopes"
TAG = "t17"
FULL = t16.FULL

# ---------------------------------------------------------------- the vaults

# Invented grade notes: (file stem, course, kind, title, category, score, possible, status, due)
GRADES = [
    ("abc-101-overall", "abc-101", "overall", "Overall Grade", None, 95.2, 100, "graded", None),
    ("abc-101-quiz-1", "abc-101", "item", "Quiz 1", "Quizzes", 9, 10, "graded", "2026-08-21T23:59"),
    ("jkl-120-outline", "jkl-120", "item", "Essay outline", "Essays", 41, 50, "graded", "2026-08-24T23:59"),
]
FETCHED_AT = "2026-08-28T13:00:00Z"
HEADS_UP = "\n".join([
    "---", "type: info", "id: info_f1x7ure171", "status: open", "kind: heads-up", "title: " + mb.yaml_text("Gmail has gone quiet"),
    "opened_at: " + mb.yaml_text("2026-08-27T10:00:00.000Z"), "opened_by: " + mb.yaml_text("agent:knowlu.gmail"),
    "close_key: " + mb.yaml_text("gmail-source:quiet"), "expires: null", "closed_at: null", "closed_by: null", "---", "",
    "Knowlu has found no mail in your Gmail for a week.", "",
])
MAIL_ACTOR = "agent:knowlu.mail"
SENDER = "Course Staff <staff@example.edu>"


def grade_note(stem, course, kind, title, category, score, possible, status, due):
    lines = ["---", "type: grade", "course: " + course, "kind: " + kind, "title: " + mb.yaml_text(title)]
    if category:
        lines.append("category: " + mb.yaml_text(category))
    if score is not None:
        lines.append("score: " + str(score))
    lines += ["possible: " + str(possible), "status: " + status]
    if due:
        lines.append("due: " + due)
    lines += ["created_by: fixture", "id: grd_" + stem.replace("-", "")[:20], "---", "", "Invented for the fixture harness.", ""]
    return "\n".join(lines)


def sources_vault(root):
    for g in GRADES:
        mb.write(root / "grades" / (g[0] + ".md"), grade_note(*g))
    mb.write(root / "state" / "grades.json", json.dumps({"fetched_at": FETCHED_AT}) + "\n")
    mb.write(root / "info" / "info_f1x7ure171.md", HEADS_UP)


def engine(*args):
    out = subprocess.run([str(mb.ENGINE)] + list(args), capture_output=True)
    if out.returncode != 0:
        sys.exit("knowlu-engine %s failed: %s" % (args[0], out.stderr.decode("utf-8", "replace")))
    return out.stdout.decode("utf-8")


def mail_writes(root, tmp):
    """What a mail pass would have written, through the engine's own `write` and `info`, as the mail actor: a new task,
    a past item straight into archive/, a moved due date with its evidence line, a moved due date the student then
    changed again, and a notice."""
    v = str(root)
    mb.write(root / "config" / "actor.yaml", "human_actor: student\n")
    w = ["write", "--vault", v, "--actor", MAIL_ACTOR, "--via", "local-runner"]
    new = pathlib.Path(tmp) / "new-task.md"
    mb.write(new, mb.task_note("task_ma11f1x701", "ABC 101 Quiz 2", "abc-101", "2026-09-04T23:59", 1.0, 3, "active", 0).replace("created_by: fixture", "created_by: mail"))
    engine(*(w + ["create", "tasks/abc-101-quiz-2.md", "--from-file", str(new)]))
    past = pathlib.Path(tmp) / "past-task.md"
    mb.write(past, mb.task_note("task_ma11f1x702", "DEF 210 Lab 0 sign-off", "def-210", "2026-08-20T17:00", 0.5, 2, "done", 100).replace("created_by: fixture", "created_by: mail"))
    engine(*(w + ["create", "archive/def-210-lab-0-sign-off.md", "--from-file", str(past)]))
    engine(*(w + ["set", "task_a1b2c3d403", "due=2026-08-31T17:00"]))
    engine(*(w + ["append-body", "task_a1b2c3d403", "--line", "2026-08-27 · " + SENDER + ": The lab moved — \"The lab write-up is now due Monday\""]))
    engine(*(w + ["set", "task_a1b2c3d404", "due=2026-09-03T23:59"]))
    engine(*(w + ["append-body", "task_a1b2c3d404", "--line", "2026-08-26 · " + SENDER + ": The draft moved — \"Drafts are due Thursday\""]))
    engine("write", "--vault", v, "--via", "dashboard", "set", "task_a1b2c3d404", "due=2026-09-04T23:59")
    engine("info", "--vault", v, "--actor", MAIL_ACTOR, "--via", "local-runner", "open", "--title", "JKL 120 meets in Hall B this week", "--kind", "notice", "--opened-by", MAIL_ACTOR, "--body", "Room change for this week only.")
    return json.loads(engine("mail-applied", "--vault", v))


# ---------------------------------------------------------------- the app's answers (invented)

GOOGLE = {"ok": True, "error": None, "state": "active", "calendar": True, "gmail": True, "email": "student@example.edu"}
GRADES_SHOWN = {"ok": True, "available": True, "reason": None, "connected": True, "signed_out": False, "host": "lms.example.edu",
                "fetched_at": "2026-08-28T07:30", "hidden": False}
GRADES_NONE = {"ok": True, "available": False, "reason": "not offered", "connected": False, "signed_out": False, "host": None,
               "fetched_at": None, "hidden": False}
BOX_ACTIVE = {"address_id": "adr_f1x7ure01", "provider": "m365", "state": "active", "address": "fwd-f1x7ure01@in.example.net",
              "turned_off": False, "open_backfill": False, "saved_sign_in": True, "unbound_reason": None, "unbound_note": None,
              "unbound_since_live": None, "suggest_rotate": False}
BOX_UNBOUND = {"address_id": "adr_f1x7ure03", "provider": "google", "state": "quiet", "address": "fwd-f1x7ure03@in.example.net",
               "turned_off": False, "open_backfill": False, "saved_sign_in": False, "unbound_reason": "unknown_sender",
               "unbound_note": "Mail is arriving from an address this mailbox never forwarded from", "unbound_since_live": 3,
               "suggest_rotate": True}
MAIL_TWO = {"ok": True, "error": None, "keep_session": True, "profile_delete_pending": False, "offer_pending": False, "setup_running": False,
            "mailboxes": [BOX_ACTIVE, BOX_UNBOUND]}
MAIL_NONE = {"ok": True, "error": None, "keep_session": True, "profile_delete_pending": False, "offer_pending": False, "setup_running": False, "mailboxes": []}
BOX_SETUP = {"address_id": "adr_f1x7ure02", "provider": "google", "state": "pending", "address": "fwd-f1x7ure02@in.example.net",
             "turned_off": False, "open_backfill": False, "saved_sign_in": True, "unbound_reason": None, "unbound_note": None,
             "unbound_since_live": None, "suggest_rotate": False}
MAIL_OFFER = {"ok": True, "error": None, "keep_session": True, "profile_delete_pending": False, "offer_pending": True, "setup_running": False,
              "mailboxes": [BOX_SETUP]}
NO_SETUP = {"ok": True, "active": False, "progress": None}


def row(step, state, reason=None, guided=False):
    return {"step": step, "state": state, "reason": reason, "guided": guided}


STEPS = ["sign_in", "step_aside", "forwarding", "add_address", "confirm_address", "turn_on", "canary", "backfill", "finish"]
GUIDED = {"ok": True, "active": True, "progress": {
    "status": "running", "ended": None, "provider": "google", "address_id": "adr_f1x7ure02",
    "rows": [row("sign_in", "done"), row("step_aside", "done"), row("forwarding", "done"), row("add_address", "needs_you", "guided", True),
             row("confirm_address", "waiting"), row("turn_on", "waiting"), row("canary", "waiting"), row("backfill", "waiting"), row("finish", "waiting")],
    "sentence": "the site needs you", "ask": None, "blocked": False, "offers": [], "backfill": None, "done": [], "left": [],
    "not_brought_in_before": None, "line": None, "kept": False}}
HELD = {"ok": True, "active": True, "progress": dict(GUIDED["progress"], provider="m365", address_id="adr_f1x7ure01", sentence="bringing in your mail",
    rows=[row(s, "done") for s in STEPS[:7]] + [row("backfill", "needs_you", "guided", True), row("finish", "waiting")],
    backfill={"since": "2026-04-30", "expected": 1200, "handled": 340, "sent": 340, "left": 860, "eta_secs": 360, "resumes_at": None, "paused": None})}
ENDED = {"ok": True, "active": False, "progress": dict(GUIDED["progress"], status="ended", ended="stopped", sentence="checking",
    rows=[row(s, "done") for s in STEPS[:4]] + [row(s, "skipped") for s in STEPS[4:]],
    done=STEPS[:4], left=STEPS[4:], not_brought_in_before="2026-04-30", line="setup stopped")}
OK = {"ok": True, "error": None}
WROTE = {"ok": True, "error": None, "state": None}


def answers_sources(applied):
    a = {k: v for k, v in answers_of("t16-settings").items() if k in ("get_settings", "settings_context", "profile", "account_status")}
    a.update({"google_status": GOOGLE, "grades_status": GRADES_SHOWN, "mail_status": MAIL_TWO, "mail_setup_progress": NO_SETUP,
              "mail_applied": applied, "copy_text": OK, "grades_refresh": OK, "mail_keep_session": dict(OK, profiles_left=False),
              "delete_note": WROTE, "set_fields": WROTE, "close_info": WROTE})
    return a


def answers_local():
    a = {k: v for k, v in answers_of("t16-local").items() if k in ("get_settings", "settings_context", "profile", "account_status")}
    a.update({"grades_status": GRADES_NONE, "mail_status": MAIL_NONE, "mail_setup_progress": NO_SETUP})
    return a


def answers_setup():
    a = answers_sources({"since": None, "courses": []})
    a.update({"mail_status": MAIL_OFFER, "mail_setup_start": OK, "mail_setup_progress": GUIDED, "mail_setup_answer": OK,
              "privacy_status": {"ok": True, "error": None, "current": "2026-09-09", "recorded": "2026-09-09", "needs_accept": False}})
    return a


def make(build):
    if build:
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not mb.ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % mb.ENGINE)
    banned = mb.fixture_slugs()
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-fixture-") as tmp:
        built = {}
        for name, change in [("sources", sources_vault), ("base", None)]:
            vault = pathlib.Path(tmp) / name
            mb.base_vault(vault)
            if change:
                change(vault)
            built[name] = {view: mb.envelope(mb.surface(vault, view)) for view in ["today", "all"]}
            if name == "sources":
                built["applied"] = mail_writes(vault, tmp)
        sets = {
            "t17-sources": (built["sources"], answers_sources(built["applied"])),
            "t17-local": (built["base"], answers_local()),
            "t17-setup": (built["base"], answers_setup()),
        }
        for name, (envs, answers) in sets.items():
            for kind, value in [("today", envs["today"]), ("all", envs["all"]), ("answers", answers)]:
                text = json.dumps(value, ensure_ascii=False)
                for needle in [tmp, tmp.replace("\\", "/"), tmp.replace("\\", "\\\\")]:
                    if needle in text:
                        sys.exit("%s-%s carries the temporary path" % (name, kind))
                for slug in banned:
                    if '"%s"' % slug in text:
                        sys.exit("%s-%s names the fixture slug %s" % (name, kind, slug))
                path = OUT / ("%s-%s.json" % (name, kind))
                t16.dump(path, value)
                print(path.relative_to(REPO).as_posix())


# ---------------------------------------------------------------- the checks

SEC_ORDER = "() => Array.from(document.querySelectorAll('.st-page [data-section]')).map(s => s.getAttribute('data-section'))"
STARTS = "() => Array.from(document.querySelectorAll('[data-section=\"sources\"] [data-start]')).map(b => ({ id: b.id || b.getAttribute('data-em-act') || (b.hasAttribute('data-grades-refresh') ? 'refresh' : 'connect'), off: b.disabled }))"


def shot(page, what):
    """A shot of the page as it is now: target/ui-shots/t17-<what>-<w>x<h>-<theme>-<text>.png."""
    vp = page.viewport_size
    look = page.evaluate("() => [document.documentElement.getAttribute('data-theme'), localStorage.getItem('knowlu.text') || 'default']")
    t16.SHOTS.mkdir(parents=True, exist_ok=True)
    page.screenshot(path=str(t16.SHOTS / ("%s-%s-%dx%d-%s-%s.png" % (TAG, what, vp["width"], vp["height"], look[0], look[1]))))


def settled(page, ms=300):
    page.wait_for_selector('[data-section="sources"]', timeout=5000)
    page.wait_for_timeout(ms)


def cmds(page):
    return [c["cmd"] for c in calls(page)]


def sources_order(page, need):
    order = page.evaluate(SEC_ORDER)
    need("sources" in order and order.index("sources") == order.index("membership") + 1, "Sources sits right after Membership: %s" % order)
    rows = page.evaluate("() => Array.from(document.querySelectorAll('[data-section=\"sources\"] > ul > li[id]')).map(l => l.id)")
    need(rows[:3] == ["set-grades", "set-google", "set-email"], "Blackboard, Google, School email in order: %s" % rows)


def no_mail_in_events(page, need, words):
    sent = json.dumps(events(page))
    for w in words:
        need(w not in sent, "no usage event carries %r" % w)


def check_sources(page, full, need):
    a = answers_of("t17-sources")
    settled(page)
    sources_order(page, need)
    need(cmds(page).count("google_status") == 1, "google_status is read once, on mount: %s" % cmds(page).count("google_status"))
    need(shown(page, "#set-google"), "a signed-in student sees the Google row")
    need("Gmail is connected (as student@example.edu)" in (text_of(page, "#set-google-state") or ""), "the Google sentence: %r" % text_of(page, "#set-google-state"))
    need(shown(page, "#set-google-disconnect-1") and not shown(page, "#set-google-connect"), "connected: Disconnect, no Connect")
    need(text_of(page, "#set-grades-say") == "Read 7:30 AM", "Blackboard says when it was read: %r" % text_of(page, "#set-grades-say"))
    need(shown(page, "[data-grades-refresh]") and shown(page, "#set-grades-forget"), "Refresh and Forget")
    need(text_of(page, "#set-email-state") == "Not connected here.", "no setup running: %r" % text_of(page, "#set-email-state"))
    need(shown(page, "#set-email-start") and shown(page, "#set-email-applied") and not shown(page, "#set-email-open"), "Connect school email and From your email; no progress to show")
    boxes = page.evaluate("() => Array.from(document.querySelectorAll('#set-email-boxes .em-box')).map(b => b.textContent.replace(/\\s+/g, ' ').trim())")
    need(len(boxes) == 2, "one row per mailbox: %s" % boxes)
    need(any(BOX_ACTIVE["address"] in b and "Forwarding is working." in b for b in boxes), "the working mailbox and its address")
    need(any(BOX_UNBOUND["unbound_note"] + " (3)" in b for b in boxes), "the amber line with its count")
    need(page.evaluate("() => !!document.querySelector('#set-email-boxes .em-amber [data-em-act=\"rotate\"]')"), "Rotate sits in the amber line when suggested")
    need(page.evaluate("() => document.querySelectorAll('#set-email-boxes [data-em-act=\"rotate\"]').length") == 2, "one Rotate per mailbox, in the line or the controls")
    need(shown(page, "#set-email-keep") and page.get_attribute("#set-email-keep", "aria-checked") == "true", "the saved sign-in switch is on")
    issue = text_of(page, "#set-src-issues") or ""
    need("Gmail has gone quiet" in issue and "Knowlu has found no mail in your Gmail for a week." in issue, "the source heads-up: %r" % issue)
    need(not page.evaluate("() => Array.from(document.querySelectorAll('[data-section=\"sources\"] [data-start]')).some(b => b.disabled)"), "nothing is owed: every start control is free")
    if not full:
        return
    page.evaluate("() => document.querySelector('[data-section=\"sources\"]').scrollIntoView({ block: 'start' })")
    page.wait_for_timeout(100)
    shot(page, "sources-section")
    # Disconnect is two steps and names Calendar.
    page.click("#set-google-disconnect-1")
    need("This disconnects Google Calendar too" in (text_of(page, "#set-google-note") or ""), "step 1 names Calendar")
    need(shown(page, "#set-google-disconnect-2"), "step 2 shows")
    need("google_disconnect" not in cmds(page), "the first press disconnects nothing")
    # Turn off email: two steps, then Cancel.
    page.click('#set-email-boxes [data-em-act="off1"]')
    need("Knowlu turns off forwarding in this mailbox" in (text_of(page, "#set-email-boxes") or ""), "the turn-off confirm")
    page.click('#set-email-boxes [data-em-act="cancel"]')
    need("mail_disconnect" not in cmds(page), "Cancel turns nothing off")
    # Copy an address through the console's copy command.
    page.click('#set-email-boxes [data-em-copy="m:%s"]' % BOX_ACTIVE["address_id"])
    page.wait_for_timeout(150)
    need({"text": BOX_ACTIVE["address"]} in [c["args"] for c in calls(page, "copy_text")], "Copy sends the address to copy_text")
    # Refresh re-reads Blackboard.
    page.click("[data-grades-refresh]")
    page.wait_for_timeout(250)
    need(len(calls(page, "grades_refresh")) == 1, "Refresh calls grades_refresh")
    # From your email, from Settings: one sheet, the applied view.
    page.click("#set-email-applied")
    page.wait_for_selector("#mail-applied:not([hidden])", timeout=3000)
    page.wait_for_timeout(250)
    need(shown(page, "#mail-applied") and not shown(page, "#mail-progress"), "From your email shows alone")
    heads = page.evaluate("() => Array.from(document.querySelectorAll('.ma-ch')).map(e => e.textContent)")
    need(heads == ["ABC 101", "DEF 210", "JKL 120", "Notices"] or heads[-1] == "Notices", "by course, notices last: %s" % heads)
    body = text_of(page, "#ma-body") or ""
    for part in ["Items created", "Fields changed", "Notices opened", "Past items archived (1)", "ABC 101 Quiz 2", "JKL 120 meets in Hall B this week"]:
        need(part in body, "From your email shows %r" % part)
    need(SENDER in body and page.evaluate("() => !document.querySelector('#ma-body staff, #ma-body [href]')"), "the sender reads as text, never markup")
    need("changed since" in body, "the row the student changed since shows no Undo")
    undos = page.evaluate("() => document.querySelectorAll('#ma-body [data-ma-undo]').length")
    need(undos >= 3, "Undo on every row nobody changed since: %d" % undos)
    page.click('#ma-body .ma-course:first-child [data-ma-undo]')
    page.wait_for_timeout(300)
    created = [x for g in a["mail_applied"]["courses"] for x in g["created"]]
    need({"id": created[0]["id"], "view": "today"} in [c["args"] for c in calls(page, "delete_note")], "Undo on a created item deletes it: %s" % calls(page, "delete_note"))
    need(text_of(page, "#ma-close") == "Close", "opened from Settings: Close")
    shot(page, "sources-applied")
    page.click("#ma-close")
    page.wait_for_timeout(150)
    need(not shown(page, "#sheet"), "Close closes the sheet")
    no_mail_in_events(page, need, ["staff@example.edu", BOX_ACTIVE["address"], "ABC 101 Quiz 2"])
    # The update owed: every start control locks with the tip; Disconnect, Forget and Turn off email stay free.
    page.evaluate("() => { KnowluKit.setOwed(true); KnowluSettings.redraw(); }")
    page.wait_for_timeout(150)
    starts = page.evaluate(STARTS)
    need(starts and all(s["off"] for s in starts), "owed: every start control is disabled: %s" % starts)
    need(page.evaluate("() => document.querySelectorAll('[data-section=\"sources\"] .owed[data-tip]').length") == len(starts), "each locked control carries the tip")
    for sel in ["#set-grades-forget", '#set-email-boxes [data-em-act="off1"]']:
        need(not page.is_disabled(sel), "%s stays free while owed" % sel)
    page.evaluate("() => { KnowluKit.setOwed(false); KnowluSettings.redraw(); }")
    page.wait_for_timeout(150)
    need(not any(s["off"] for s in page.evaluate(STARTS)), "the lock lifts when the update is accepted")
    # Leaving Settings drops the Google row's token; coming back asks again.
    go(page, "#home", "home")
    go(page, "#settings", "settings")
    page.wait_for_timeout(300)
    need(cmds(page).count("google_status") == 2, "each Settings visit reads google_status once")


def check_local(page, full, need):
    settled(page)
    sources_order(page, need)
    need("google_status" not in cmds(page), "no account: Google is never asked")
    need(not shown(page, "#set-google"), "no account: no Google row")
    need(text_of(page, "#set-grades-say") == "Grades from Blackboard are not available at your school yet", "the not-available line: %r" % text_of(page, "#set-grades-say"))
    need(page.evaluate("() => document.querySelectorAll('#set-grades button').length") == 0, "no button on the not-available line")
    need(text_of(page, "#set-email-state") == "Not connected here.", "no setup")
    need(shown(page, "#set-email-start") and not shown(page, "#set-email-applied"), "Connect school email; nothing came from email")
    need(page.evaluate("() => document.querySelectorAll('#set-email-boxes li').length") == 0 and not shown(page, "#set-email-keep"), "no mailbox, no saved sign-in switch")
    need((text_of(page, "#set-src-issues") or "") == "", "no source heads-up")
    if not full:
        return
    page.click("#set-email-start")
    page.wait_for_timeout(200)
    need({"provider": "school"} in [c["args"] for c in calls(page, "mail_setup_start")], "Connect school email starts the school setup")


# Holds every account_status answer until window.__releaseAccount() (then answers pass straight through): the stub sets
# window.__TAURI__ after this init script, so its invoke is wrapped as it is assigned.
HOLD_ACCOUNT = """(function () {
  var held = [], released = false, tauri;
  window.__releaseAccount = function () { released = true; held.splice(0).forEach(function (f) { f(); }); };
  Object.defineProperty(window, "__TAURI__", { configurable: true, get: function () { return tauri; }, set: function (v) {
    var inv = v.core.invoke;
    v.core.invoke = function (cmd, args) {
      var p = inv(cmd, args);
      if (cmd !== "account_status" || released) { return p; }
      return new Promise(function (res) { held.push(function () { p.then(res); }); });
    };
    tauri = v;
  } });
})();"""


def swap_progress(page, answer):
    page.evaluate("""(ans) => { const inv = window.__TAURI__.core.invoke;
      if (!window.__orig) { window.__orig = inv; }
      window.__TAURI__.core.invoke = (cmd, args) => cmd === 'mail_setup_progress' ? (window.__fixture.calls.push({ cmd: cmd, args: args || {} }), Promise.resolve(JSON.parse(JSON.stringify(ans)))) : window.__orig(cmd, args); }""", answer)


def check_setup(page, full, need):
    page.wait_for_timeout(900)
    need("account_status" in cmds(page) and "mail_setup_start" not in cmds(page), "no setup starts before the account check answers (M10)")
    page.evaluate("() => window.__releaseAccount()")
    page.wait_for_selector("#mail-progress:not([hidden])", timeout=4000)
    page.wait_for_timeout(400)
    need([c["args"] for c in calls(page, "mail_setup_start")] == [{"provider": "school"}], "the first open starts the school setup once")
    need(shown(page, "#sheet") and shown(page, "#mail-progress") and not shown(page, "#mail-applied"), "the progress view, in the one sheet")
    need(page.get_attribute("#sheet", "aria-labelledby") == "mp-h", "#sheet is named by the progress heading: %r" % page.get_attribute("#sheet", "aria-labelledby"))
    need(page.evaluate("() => document.querySelectorAll('#sheet [role=\"dialog\"], #sheet [aria-modal]').length") == 0, "no dialog inside the dialog")
    rows = page.evaluate("() => Array.from(document.querySelectorAll('#mp-rows .mp-row')).map(r => r.className)")
    need(len(rows) == 9 and "mp-needs_you" in rows[3], "one row per step, its state a class: %s" % rows)
    need(text_of(page, "#mp-sentence") == "the site needs you", "the engine's sentence as text")
    need(BOX_SETUP["address"] in (text_of(page, "#mp-ask") or "") and shown(page, "[data-mp-copy]"), "the guided step's address with Copy")
    need(page.evaluate("() => document.querySelectorAll('#mp-ask .mp-path li').length") == 4, "Gmail's path, four lines")
    need(shown(page, '[data-mp-choice="done"]'), "I've done this")
    if full:
        shot(page, "setup-sheet")
    if full:
        page.click('[data-mp-choice="done"]')
        page.wait_for_timeout(200)
        need({"choice": "done"} in [c["args"] for c in calls(page, "mail_setup_answer")], "I've done this answers done")
        swap_progress(page, HELD)
        page.wait_for_timeout(1700)
        ask = text_of(page, "#mp-ask") or ""
        need("Knowlu can’t bring in Outlook’s older mail yet." in ask and not shown(page, "[data-mp-copy]"), "the held backfill: its sentence, no address: %r" % ask)
        need("since 30 April: 340 of 1200 · about 6 min left" in (text_of(page, "#mp-backfill") or ""), "backfill N of M and time left: %r" % text_of(page, "#mp-backfill"))
        swap_progress(page, ENDED)
        page.wait_for_timeout(1700)
        end = text_of(page, "#mp-end") or ""
        need(end.startswith("setup stopped Done: Sign in, Step aside, Open forwarding, Add Knowlu’s address.") and "Mail before 30 April was not brought in." in end, "what an ended setup did: %r" % end)
        need(not shown(page, "#mp-stop"), "an ended setup has no Stop")
        page.click("#mp-applied")
        page.wait_for_selector("#mail-applied:not([hidden])", timeout=3000)
        need(shown(page, "#mail-applied") and not shown(page, "#mail-progress") and page.evaluate("() => document.querySelectorAll('.sheet-in').length") == 1, "From your email switches the one sheet in place")
        need(text_of(page, "#ma-close") == "Back" and "Nothing has come in from your email yet." in (text_of(page, "#ma-body") or ""), "Back, and the empty sentence")
        need(page.get_attribute("#sheet", "aria-labelledby") == "ma-h", "the view's heading names #sheet")
        page.click("#ma-close")
        page.wait_for_timeout(200)
        need(shown(page, "#mail-progress"), "Back returns to the progress view")
    page.click("#mp-leave")
    page.wait_for_timeout(200)
    need(not shown(page, "#sheet"), "Leave closes the sheet")
    need("mail_setup_stop" not in cmds(page), "Leave never stops the setup")
    need(page.evaluate("() => KnowluBoot.mode()") == "console", "the console path")


def check_owed(page, full, need):
    page.evaluate("() => KnowluKit.setOwed(true)")
    page.evaluate("() => window.__releaseAccount()")
    page.wait_for_timeout(1200)
    need("privacy_status" in cmds(page), "the account check answered")
    need("mail_setup_start" not in cmds(page), "no email setup starts while the update is owed")
    need(not shown(page, "#sheet"), "no mail sheet")
    page.evaluate("() => KnowluKit.setOwed(false)")


CASES = [
    {"name": "sources", "set": "t17-sources", "route": "settings", "check": check_sources},
    {"name": "local", "set": "t17-local", "route": "settings", "check": check_local},
    {"name": "setup", "set": "t17-setup", "route": "home", "check": check_setup, "init": HOLD_ACCOUNT},
    {"name": "owed", "set": "t17-setup", "route": "home", "check": check_owed, "combos": FULL, "init": HOLD_ACCOUNT},
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t17-*.json from the engine first")
    ap.add_argument("--no-build", action="store_true", help="with --make: use the engine already in target/debug")
    args = ap.parse_args()
    if args.make:
        make(not args.no_build)
    sys.exit(1 if t16.run(TAG, CASES) else 0)


if __name__ == "__main__":
    main()
