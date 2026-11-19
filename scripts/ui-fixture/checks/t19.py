"""T19 Status and the update line: the DOM check (v0.4.0 plan §5 T19; note §4 Settings "Status", §5 "Update offer").

    python scripts/ui-fixture/checks/t19.py           # check against the staged t19 envelopes
    python scripts/ui-fixture/checks/t19.py --make    # rebuild envelopes/t19-*.json first

The state envelopes are derived from the base set (make_base_courses.py builds it from the engine), T11's precedent:
  t19-status   base-unreadable (one note the engine could not read) with the app's topline keys changed as the app
               writes them (app/src/commands.rs, scheduler.rs: a sync skipped for "no entitlement", a backup two days
               behind, a retried slot that failed, one slot missed while quit), and, shaped as the engine's surface
               writes them, one journal warning, one source that needs the student, one flagged issue and one
               recent run. The updater answers with a staged version; installing meets a running slot.
  t19-quiet    base-courses unchanged: a healthy app; the updater says up to date; Sync now syncs.
  t19-failed   base-courses unchanged: the launch's update check fails (no network); the reply still carries the
               updater's state, as every check_for_updates reply does (review I3).
The other answers are the app's commands as app/src shapes them (check_for_updates, install_update, sync), plus T16's
Settings reads. Invented values only.

Checks, for every size (820x600, 1280x800) x theme x text size: no horizontal scroll, no console error.
  t19-status: Status sits before This computer; the sync line says the engine's skip in SYNC_SAYS' words and "tried";
    the backup is behind; the last slot failed on its second attempt; Sync now is a start control; the source that
    needs a look is drawn once, under Sources, never in Status (review M8); the flagged issue with Resolve, the warning and the unreadable note, the runs; the top band offers
    the staged version. At 1280x800 light and 820x600 dark larger: Sync now sends sync, emits sync_run with no text
    and toasts the skip; Resolve sends the typed resolution; Check now says the version is available; Restart to
    update waits for the running slot.
  t19-quiet: the line says in step; the top band is empty; Check now says up to date; Sync now toasts the time.
  t19-failed: the Updates row names the failed check, never "not checked yet"; the top band is empty.
Shots go to target/ui-shots/t19-*.png (never committed); t19-statuspart-* and t19-quietpart-* show the section itself. Exit 1 on any failure.
"""
import argparse
import copy
import json
import pathlib
import sys

sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import t16  # noqa: E402
from t16 import FULL, answers_of, calls, dump, events, run, shown, text_of  # noqa: E402,F401

OUT = t16.OUT
STAGED = "0.4.1"
FAILED = "the release host could not be reached"
WARNING = "state/journal/2026-08-27.jsonl line 4: not JSON, skipped"
SOURCE = {"id": "info_f1x7ure301", "title": "Coursework site", "text": "Knowlu could not sign in to the coursework site; sign in again in Settings.",
          "since": "2026-08-27T18:00", "close_key": "source:coursework", "path": "info/coursework-site-heads-up.md"}
ISSUE = {"id": "iss_f1x7ure301", "title": "ABC 101 Reading quiz", "categories": ["wrong-effort", "wrong-date"], "target": "task_a1b2c3d402",
         "opened_at": "2026-08-27T20:15", "path": "issues/abc-101-reading-quiz.md"}
RUN = {"run_id": "run_f1x7ure301", "runner": "local", "started": "2026-08-28T07:00", "ended": "2026-08-28T07:01", "result": "WARN",
       "summary": "coursework: 1 site could not sign in\nrank: 6 tasks ranked"}


def load(name):
    return json.loads((OUT / name).read_text(encoding="utf-8"))


def status_state(view):
    env = copy.deepcopy(load("base-unreadable-%s.json" % view))
    st = env["state"]
    top = st["topline"]
    top["sync"] = {"ok": False, "at": "2026-08-28T13:00:00Z", "lines": [], "last_error": None, "skipped": "no entitlement"}
    top["backup"] = {"last_ok": "2026-08-26", "behind_days": 2, "last_error": None, "target_reachable": True}
    top["startup_missed"] = 1
    top["last_slot"] = {"started": "2026-08-28T07:00:00-05:00", "ended": "2026-08-28T07:01:12-05:00",
                        "steps": [["sync", 0], ["coursework", 1], ["ingest", 0], ["grades", 0], ["judge", 0], ["rank", 0]],
                        "ok": False, "engine_ok": False, "late": False, "reason": None, "attempts": 2}
    st["warnings"] = [WARNING]
    st["issues_panel"] = {"open_count": 1, "sources": [SOURCE], "rows": [ISSUE], "empty_text": "No open issues."}
    st["runs_panel"] = dict(st["runs_panel"], recent=[RUN], empty_text=None)
    return env


def offer(staged):
    return {"ok": True, "error": None, "version": "0.4.0", "staged": staged, "held": None, "last_check": "2026-08-28T13:55:00Z",
            "last_error": None, "action_error": None}


def make():
    common = {"account_status": t16.ACCOUNT_MEMBER, "get_settings": t16.SETTINGS, "settings_context": t16.CONTEXT, "grades_status": t16.GRADES,
              "profile": t16.PROFILE, "mail_status": t16.MAIL_NONE}
    today = status_state("today")
    for view in ["today", "all"]:
        dump(OUT / ("t19-status-%s.json" % view), status_state(view))
    dump(OUT / "t19-status-answers.json", dict(common, check_for_updates=offer(STAGED),
                                               install_update={"ok": False, "error": "a slot is running — the update will be offered when it finishes"},
                                               sync={"ok": True, "error": None, "state": today["state"]}))
    t16.write_set("t19-quiet", dict(common, check_for_updates=offer(None)))
    t16.write_set("t19-failed", dict(common, check_for_updates=dict(offer(None), ok=False, error=FAILED, last_error=FAILED)))
    for p in ["t19-status-today.json", "t19-status-all.json", "t19-status-answers.json"]:
        print((OUT / p).relative_to(t16.REPO).as_posix())


# ---------------------------------------------------------------- the checks


def wait_status(page):
    page.wait_for_selector('.st-page [data-section="status"]', timeout=5000)
    page.wait_for_timeout(300)


def toasts(page):
    return page.evaluate("() => Array.from(document.querySelectorAll('#toasts .toast')).map(t => t.textContent)")


def section_shot(page, name):
    """Status itself, scrolled to its heading (the driver's own shot shows the top of Settings)."""
    tag = page.evaluate("() => [innerWidth, innerHeight, document.documentElement.getAttribute('data-theme'), localStorage.getItem('knowlu.text')].join('-')")
    w, hgt, theme, text = tag.split("-")
    page.evaluate("() => document.querySelector('.st-page [data-section=\"status\"]').scrollIntoView({ block: 'start' })")
    page.wait_for_timeout(120)
    page.screenshot(path=str(t16.SHOTS / ("t19-%s-%sx%s-%s-%s.png" % (name, w, hgt, theme, text))))
    page.evaluate("() => { document.getElementById('center').scrollTop = 0; }")


def check_status(page, full, need):
    wait_status(page)
    section_shot(page, "statuspart")
    hs = page.evaluate("() => Array.from(document.querySelectorAll('.st-page .st-sec > .st-h')).map(h => h.textContent.trim())")
    need("Status" in hs and "This computer" in hs and hs.index("Status") < hs.index("This computer"), "Status before This computer: %s" % hs)
    sync = (text_of(page, "#sx-sync") or "").lower()
    need("can't confirm your subscription — changes stay on this computer" in sync, "the skip in SYNC_SAYS' words: %r" % sync)
    need("tried " in sync and "in step" not in sync, "a skip says tried, never in step")
    need((text_of(page, "#sx-backup") or "") == "Backup 2 days behind", "the backup is behind, its first word capitalised")
    slot = (text_of(page, "#sx-slot") or "").lower()
    need("failed" in slot and "(attempt 2)" in slot and "1 slot missed while quit" in slot and "scheduler on" in slot, "the last slot: %r" % slot)
    need(page.get_attribute("[data-sync]", "data-start") is not None and not page.is_disabled("[data-sync]"), "Sync now is a free start control")
    need(SOURCE["text"] in (text_of(page, "#set-src-issues") or ""), "the source that needs a look, under Sources")
    need(SOURCE["text"] not in (text_of(page, '[data-section="status"]') or ""), "drawn once: Status does not repeat it")
    need(page.evaluate("(id) => document.querySelectorAll('[data-id=\"' + id + '\"]').length", SOURCE["id"]) == 1, "one row for the source on the page")
    iss = text_of(page, "#sx-issues") or ""
    need(ISSUE["title"] in iss and "wrong-effort, wrong-date" in iss, "the flagged issue, its categories as words")
    need(shown(page, '#sx-issues [data-resolve="%s"]' % ISSUE["id"]), "Resolve")
    warns = text_of(page, "#sx-warns") or ""
    need(WARNING in warns and "tasks/broken-note.md" in warns, "the warning and the unreadable note: %r" % warns)
    runs = text_of(page, "#sx-runs") or ""
    need("Local, " in runs and "6 tasks ranked" in runs and "Missing" in runs, "the expected slots, the recent run and its summary: %r" % runs[:200])
    note = text_of(page, "#top-note") or ""
    need(("Knowlu %s is ready." % STAGED) in note and shown(page, "#top-note [data-install]"), "the top band offers the staged version: %r" % note)
    need(len(calls(page, "check_for_updates")) == 1, "one check at launch")
    if not full:
        return
    page.click("[data-sync]")
    page.wait_for_timeout(300)
    need(len(calls(page, "sync")) == 1 and calls(page, "sync")[0]["args"].get("view") == "today", "Sync now sends sync with the state view")
    ev = events(page, "sync_run")
    need(len(ev) == 1 and ev[0].get("objectId") is None and ev[0].get("objectKind") is None, "sync_run carries nothing: %s" % ev)
    need(any("can't confirm your subscription" in t.lower() for t in toasts(page)), "the skip is toasted in the line's words: %s" % toasts(page))
    page.fill('#sx-issues [data-res="%s"]' % ISSUE["id"], "The quiz is due Friday.")
    page.click('#sx-issues [data-resolve="%s"]' % ISSUE["id"])
    page.wait_for_timeout(250)
    rs = [c["args"] for c in calls(page, "resolve_issue")]
    need(rs and rs[0].get("id") == ISSUE["id"] and rs[0].get("resolution") == "The quiz is due Friday.", "Resolve sends the resolution: %s" % rs)
    need(not any("Friday" in json.dumps(e) for e in events(page)), "no event carries the resolution")
    page.evaluate("() => document.querySelector('#set-update-check').scrollIntoView()")
    page.click("#set-update-check")
    page.wait_for_timeout(250)
    need(len(calls(page, "check_for_updates")) == 2, "Check now checks")
    need((text_of(page, "#set-update-state") or "") == STAGED + " is available", "the row says what is available: %r" % text_of(page, "#set-update-state"))
    page.click("#top-note [data-install]")
    page.wait_for_timeout(250)
    need(len(calls(page, "install_update")) == 1, "Restart to update installs")
    need("waiting for the current sync to finish" in (text_of(page, "#top-note") or ""), "a running slot is a wait: %r" % text_of(page, "#top-note"))


def check_quiet(page, full, need):
    wait_status(page)
    section_shot(page, "quietpart")
    need((text_of(page, "#sx-sync") or "").startswith("In step with your account"), "a healthy sync says in step")
    need("synced " in (text_of(page, "#sx-sync") or ""), "…and when")
    need(not (text_of(page, "#top-note") or ""), "nothing staged, nothing in the band")
    need("No open issues." in (text_of(page, "#sx-issues") or ""), "the engine's empty text")
    if not full:
        return
    page.evaluate("() => document.querySelector('#set-update-check').scrollIntoView()")
    page.click("#set-update-check")
    page.wait_for_timeout(250)
    need((text_of(page, "#set-update-state") or "").startswith("up to date (checked "), "up to date: %r" % text_of(page, "#set-update-state"))
    page.click("[data-sync]")
    page.wait_for_timeout(300)
    need(any(t.startswith("Synced ") for t in toasts(page)), "a clean sync toasts the time: %s" % toasts(page))


def check_failed(page, full, need):
    wait_status(page)
    page.wait_for_timeout(200)
    row = text_of(page, "#set-update-state") or ""
    need(FAILED in row and "not checked yet" not in row, "a failed check is named in the Updates row: %r" % row)
    need(not (text_of(page, "#top-note") or ""), "nothing offered, nothing in the band")


CASES = [
    {"name": "status", "set": "t19-status", "route": "settings/status", "check": check_status},
    {"name": "quiet", "set": "t19-quiet", "route": "settings/status", "check": check_quiet},
    {"name": "failed", "set": "t19-failed", "route": "settings/status", "check": check_failed},
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--make", action="store_true", help="rebuild envelopes/t19-*.json first")
    args = ap.parse_args()
    if args.make:
        make()
    sys.exit(1 if run("t19", CASES) else 0)


if __name__ == "__main__":
    main()
