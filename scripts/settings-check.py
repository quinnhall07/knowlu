"""Headless behaviour check for the settings overlay (Knowlu plan 4a, Task 7).

The overlay is an overlay IN the console page (R-P4a-4), so a static-asset test can only prove the
markup is there. This drives the real page in Chromium behind a FAKE `__TAURI__` — the same shape
`withGlobalTauri` puts on the window — and asserts what the rows actually do: the gear opens the
panel, every row shows what the commands said (never a guess), each control invokes the command it
should with exactly the payload it should, Escape and Close put it away, and nothing inside it
carries a `data-id` (the observer's key; the panel is not an observed region).

Usage (the Playwright venv is a throwaway and must never enter requirements.txt):
    python -m venv .wv && .wv\\Scripts\\python -m pip install playwright && .wv\\Scripts\\python -m playwright install chromium
    .wv\\Scripts\\python scripts/settings-check.py [out_dir]

Serves a FRESH TEMP DIRECTORY over loopback, never the repo root (ruling R40 — `config/ingest.yaml`
holds a live token and a secret calendar URL): a copy of app/static/ and one frozen surface fixture.
Prints `ok` and exits 0, or prints every failure and exits 1. Read-only; no desktop input.
"""
import http.server
import json
import shutil
import socketserver
import sys
import tempfile
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[1]
WIDTH, HEIGHT = 1512, 945

# What the fake commands answer. Deliberately not values the page could invent for itself: the
# profile name is NOT the vault folder's name, so a panel that guessed would show "ada" here.
CTX = {"ok": True, "error": None, "vault": "C:\\v\\ada", "version": "0.1.0", "profile_name": "Ada"}
SETTINGS = {"profile_id": "profile_x", "backup_dir": "C:\\Backups\\Knowlu", "autostart": False, "quit_at": None}
PICKED = "C:\\Docs\\Knowlu"
# Plan 4a Task 8, and its key-gated step. Five states `check_for_updates` can answer with, and the
# row has to render every one of them as a FACT, not as an incident: no dialog, no retry, and an
# offer only when the engine says there is one.
#
# UNREACHABLE is the normal state until the release site exists — the plugin is configured, the
# endpoint simply does not answer. It is what the page boots into here, deliberately: the quiet
# failure is the case a regression would hide behind.
UNREACHABLE = {"ok": False, "error": "error sending request for url", "version": "0.1.0",
               "staged": None, "held": None,
               "last_check": "2026-09-05T12:00:00Z", "last_error": "error sending request for url"}
# CONFIGURED: the endpoint answered and there is nothing newer.
CONFIGURED = {"ok": True, "error": None, "version": "0.1.0", "staged": None, "held": None,
              "last_check": "2026-09-06T12:00:00Z", "last_error": None}
# BUSY: single flight (R-P4a-24). *Check now* pressed while the daily check is still downloading is
# refused, and told so — never queued, never a second download into the same bundle path. Since fix
# round 1's m2 the refusal is an ACTION error, not a check error: nothing was fetched, so
# `last_check` still carries the PREVIOUS check's stamp and `last_error` its verdict, and the row
# must go on reporting that check truthfully while naming the refusal separately.
BUSY = {"ok": False, "error": "a check is already running", "version": "0.1.0",
        "staged": None, "held": None,
        "last_check": "2026-09-06T12:00:00Z", "last_error": None,
        "action_error": "a check is already running"}
# The offer's own state: staged, and no slot running (the mid-run gate is Rust's, so a staged
# version reaching the page IS the offer — the page never re-decides it).
OFFERED = {"ok": True, "error": None, "version": "0.1.0", "staged": "0.2.0", "held": None,
           "last_check": "2026-09-06T12:00:00Z", "last_error": None}
# HELD: the same bundle, downloaded and verified, while a slot IS running. `update_offer` withholds
# the offer (R9) and `stage_note` fills `held` — so the row must say what is on disk rather than
# "up to date", and the topline must still carry no offer (R-P4a-24).
HELD = {"ok": True, "error": None, "version": "0.1.0", "staged": None, "held": "0.2.0",
        "last_check": "2026-09-06T12:10:00Z", "last_error": None}
# FAILED: a check that reached the endpoint and refused what it found. A bad signature is an error
# the reader is shown, never a silent retry (Knowlu spec §6).
FAILED = {"ok": False, "error": "signature verification failed", "version": "0.1.0",
          "staged": None, "held": None,
          "last_check": "2026-09-06T12:20:00Z", "last_error": "signature verification failed"}
# ACTION_ERROR (fix round 1, IMPORTANT 2): the check itself SUCCEEDED — `ok`, no `last_error`, an
# endpoint that answered "nothing newer" — while the last ACTION, a tray *Restart to update*,
# was refused. The row has to carry both: "up to date" is a true statement about the check and says
# nothing about the install that failed, so the action's outcome is its own suffix and outlives
# every check until the next explicit action.
ACTION_ERROR = {"ok": True, "error": None, "version": "0.1.0", "staged": None, "held": None,
                "last_check": "2026-09-06T12:30:00Z", "last_error": None,
                "action_error": "a slot is running — the update will be offered when it finishes"}

# Gmail connect T9c: the Google row's states. Copy is spec section 4.4's table, verbatim.
ACCOUNT = {"ok": True, "error": None, "needs_account": False, "email": "ada@example.test", "status": "active", "plan": "monthly"}
G_TESTING = "While Google reviews Knowlu, this works only for invited testers, and the connection needs renewing about once a week."
G_TIMEOUT = "Google did not finish connecting. If Google said Knowlu is not verified, this Google account is not on the tester list yet."
G_NONE = {"ok": True, "error": None, "state": "none", "calendar": False, "gmail": False, "email": None}
G_CAL = {"ok": True, "error": None, "state": "active", "calendar": True, "gmail": False, "email": "ada@example.test"}
G_GMAIL = {"ok": True, "error": None, "state": "active", "calendar": True, "gmail": True, "email": "ada@example.test"}
G_REVOKED = {"ok": True, "error": None, "state": "revoked", "calendar": True, "gmail": True, "email": "ada@example.test"}
G_ERROR = {"ok": False, "error": "Knowlu could not reach Google just now."}
G_CAL_SAYS = "Google Calendar is connected (as ada@example.test). Gmail is not."
G_BUTTONS = ["#set-google-connect", "#set-google-reconnect", "#set-google-disconnect-1", "#set-google-retry"]
G_STATES = [
    (G_NONE, "Gmail is not connected. Knowlu can read your inbox for things you have to do and propose each one for you to approve.",
     ["#set-google-connect"]),
    (G_CAL, G_CAL_SAYS, ["#set-google-connect", "#set-google-disconnect-1"]),
    (G_GMAIL, "Gmail is connected (as ada@example.test), read-only. Knowlu proposes what it finds; nothing is added without you.",
     ["#set-google-disconnect-1"]),
    (G_REVOKED, "Google stopped answering for Knowlu. While Google reviews Knowlu, connections expire after seven days.",
     ["#set-google-reconnect", "#set-google-disconnect-1"]),
    (G_ERROR, G_ERROR["error"], ["#set-google-retry"]),
]

FAKE = """
window.__CALLS = [];
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args || {}]);
  var S = window.__SETTINGS;
  switch (cmd) {
    case "launch_state":     return Promise.resolve({ ok: true, error: null, mode: "console" });
    case "state":            return fetch(window.__FIXTURE).then(function (r) { return r.json(); })
                                    .then(function (s) { return { ok: true, error: null, state: s }; });
    case "settings_context": return Promise.resolve(window.__CTX);
    case "get_settings":     return Promise.resolve({ ok: true, error: null, settings: S });
    case "set_settings":     Object.keys(args.patch).forEach(function (k) { S[k] = args.patch[k]; });
                             return Promise.resolve({ ok: true, error: null, settings: S });
    case "set_profile_name": return Promise.resolve({ ok: true, error: null,
                               profile: { id: "profile_x", name: args.name, vault: window.__CTX.vault } });
    case "pick_folder":      return Promise.resolve({ ok: true, error: null, path: window.__PICKED });
    case "backup_now":       return Promise.resolve({ ok: true, error: null, state: null });
    // Plan 4a Task 8. `window.__UPDATE` is what `check_for_updates` answered last; the check
    // swaps it to drive the three states the row and the offer have to render.
    case "check_for_updates": return Promise.resolve(window.__UPDATE);
    case "install_update":   window.__INSTALLED = true;
                             return Promise.resolve(window.__INSTALL_RESULT || { ok: true, error: null });
    // Gmail connect T9c: the Google row. `window.__ACCOUNT` unset keeps the vault-less shell's
    // behaviour (the command is not registered, the row stays hidden).
    case "account_status":   return window.__ACCOUNT ? Promise.resolve(window.__ACCOUNT)
                                                     : Promise.reject(new Error("no account_status"));
    case "google_status":    return Promise.resolve(window.__GSTATUS);
    case "google_connect":   return Promise.resolve({ ok: true, error: null });
    case "google_disconnect": return Promise.resolve(window.__GDISCONNECT || { ok: true, error: null });
    case "switch_profile":
    case "copy_text":
    case "copy_diagnostics":
    case "mark_seen":
    case "ui_event":         return Promise.resolve({ ok: true, error: null });
    default:                 return Promise.reject(new Error("unexpected command " + cmd));
  }
} } };
"""


def calls(page, cmd):
    return [c[1] for c in page.evaluate("window.__CALLS") if c[0] == cmd]


def boxes_overlap(a, b):
    if not a or not b:
        return False
    return not (a["x"] + a["width"] <= b["x"] or b["x"] + b["width"] <= a["x"]
                or a["y"] + a["height"] <= b["y"] or b["y"] + b["height"] <= a["y"])


def check(url, fixture_name, out, bad):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_context(viewport={"width": WIDTH, "height": HEIGHT},
                                   color_scheme="dark", device_scale_factor=1).new_page()
        page.add_init_script(
            f"window.__FIXTURE = '/fixtures/{fixture_name}';"
            f"window.__CTX = {json.dumps(CTX)};"
            f"window.__SETTINGS = {json.dumps(SETTINGS)};"
            f"window.__PICKED = {json.dumps(PICKED)};"
            f"window.__UPDATE = {json.dumps(UNREACHABLE)};" + FAKE)
        # A broken page must produce a LIST of failures, not a 30 s hang and a traceback from the
        # first click on something that is not there.
        page.set_default_timeout(5000)
        page.goto(url)
        page.wait_for_timeout(900)

        # 1. The panel starts closed, and the topline carries the gear (one of its two ways in).
        if not page.is_hidden("#settings"):
            bad.append("the settings overlay is open before anything asked for it")
        if page.locator("#topline [data-settings]").count() != 1:
            bad.append("the topline has no gear")

        # 2. The gear opens it, and every row shows what the COMMANDS said.
        page.click("#topline [data-settings]")
        page.wait_for_timeout(300)
        if page.is_hidden("#settings"):
            bad.append("the gear did not open the settings overlay")
            # Open it the other way so the rest of the run still reports on the rows, instead of
            # every later assertion timing out on a hidden element.
            page.evaluate("window.KNOWLU_OPEN_SETTINGS && window.KNOWLU_OPEN_SETTINGS()")
            page.wait_for_timeout(300)
        if page.input_value("#set-name-in") != "Ada":
            bad.append(f"S4: the profile name did not come from settings_context ({page.input_value('#set-name-in')!r})")
        if CTX["vault"] not in page.inner_text("#set-vault-path"):
            bad.append("the vault path is not shown")
        if page.inner_text("#set-bdir") != SETTINGS["backup_dir"]:
            bad.append(f"the backup folder is not get_settings' ({page.inner_text('#set-bdir')!r})")
        if page.is_checked("#set-autostart-in"):
            bad.append("autostart is shown on when get_settings said off")
        if page.locator("#set-diag-copy").count() != 1:
            bad.append("the diagnostics row has no Copy diagnostics button")
        # Plan 4a Task 8: R-P4a-5's placeholder is spent — the button is live, and the row shows
        # the LAST CHECK, which at boot here is an endpoint that did not answer. That is a fact
        # about the world, not an incident: no dialog, no offer, no retry.
        if page.is_disabled("#set-update-check"):
            bad.append("the Updates row's button must be live now that a command stands behind it")
        row = page.inner_text("#set-update-state")
        if UNREACHABLE["last_error"] not in row:
            bad.append(f"the Updates row does not carry the check's own words ({row!r})")
        if CTX["version"] not in row:
            bad.append("the Updates row does not show the version the check gave")
        if not page.is_hidden("#upd"):
            bad.append("nothing is staged, so the topline must carry no offer")

        # 3. Nothing inside the overlay is an observed object.
        if page.evaluate("document.querySelectorAll('#settings [data-id]').length") != 0:
            bad.append("an element inside the overlay carries data-id")

        # 4. The panel must not sit on top of the controls it shares the page with.
        panel = page.locator("#settings").bounding_box()
        for sel, what in [("#topline [data-settings]", "the gear"), ("[data-sync]", "sync now"),
                          ("[data-backup]", "back up now")]:
            if boxes_overlap(panel, page.locator(sel).first.bounding_box()):
                bad.append(f"the open panel covers {what}")
        sw, iw = page.evaluate("document.documentElement.scrollWidth"), page.evaluate("window.innerWidth")
        if sw > iw:
            bad.append(f"the open panel overflows at {WIDTH}px (scrollWidth={sw} innerWidth={iw})")
        page.screenshot(path=str(out / "settings-1512.png"), full_page=False)

        # 5. Each control sends exactly the command and payload it should — no extra keys, no
        #    guessed values.
        page.evaluate("window.__CALLS = []")
        page.click("#set-bdir-pick")
        page.wait_for_timeout(300)
        if calls(page, "set_settings") != [{"patch": {"backup_dir": PICKED}}]:
            bad.append(f"set_settings payload from the folder picker: {calls(page, 'set_settings')}")
        if page.inner_text("#set-bdir") != PICKED:
            bad.append("the row did not repaint from set_settings' own settings")

        page.evaluate("window.__CALLS = []")
        page.check("#set-autostart-in")
        page.wait_for_timeout(300)
        if calls(page, "set_settings") != [{"patch": {"autostart": True}}]:
            bad.append(f"set_settings payload from the autostart row: {calls(page, 'set_settings')}")

        page.evaluate("window.__CALLS = []")
        page.fill("#set-name-in", "Ada Two")
        page.click("#set-name-save")
        page.wait_for_timeout(300)
        if calls(page, "set_profile_name") != [{"name": "Ada Two"}]:
            bad.append(f"set_profile_name payload: {calls(page, 'set_profile_name')}")

        page.evaluate("window.__CALLS = []")
        page.click("#set-vault-copy")
        page.click("#set-diag-copy")
        page.wait_for_timeout(300)
        if calls(page, "copy_text") != [{"text": CTX["vault"]}]:
            bad.append(f"copy_text payload: {calls(page, 'copy_text')}")
        if calls(page, "copy_diagnostics") != [{}]:
            bad.append("Copy diagnostics does not go through the one Rust-built blob")

        # R-P4a-15: Switch profile… is a relaunch through --pick, never a second --vault launch
        # (which single-instance would swallow by focusing the window that is already running).
        page.evaluate("window.__CALLS = []")
        page.click("#set-switch")
        page.wait_for_timeout(300)
        if calls(page, "switch_profile") != [{}]:
            bad.append(f"Switch profile… did not invoke switch_profile: {page.evaluate('window.__CALLS')}")

        # 6. "Never silently": an unreadable profile registry means the name shown is the vault
        #    folder's, not this profile's, and settings_context says so instead of letting the
        #    fallback pass for the real thing (review round 1, IMPORTANT 2).
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)
        page.evaluate("window.__CTX = Object.assign({}, window.__CTX, "
                      "{ profile_name: 'ada', registry_error: 'profiles.json: is not a profile registry' });")
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(300)
        if "is not a profile registry" not in page.inner_text("#set-diag-note"):
            bad.append(f"a broken registry is not shown to the reader ({page.inner_text('#set-diag-note')!r})")
        page.evaluate("window.__CTX = Object.assign({}, window.__CTX, "
                      f"{{ profile_name: {json.dumps(CTX['profile_name'])}, registry_error: null }});")
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(300)
        if page.inner_text("#set-diag-note") != "":
            bad.append("the registry error outlived the registry problem")

        # 6a-bis. Plan 4a Task 8's key-gated step: the four row states that are NOT an offer, each
        #     rendered as a fact in the row's own words. `#upd` stays hidden through all four —
        #     an offer is made only when the engine puts a version in `staged` (R9), and the HELD
        #     case is the one R-P4a-24 is about: a verified bundle on disk that the running slot is
        #     holding back must not be reported as "up to date".
        for u, wants in [
            (CONFIGURED, ["up to date"]),
            # m2: a refused check keeps reporting the LAST REAL check ("up to date", 12:00) and
            # names the refusal as an action, rather than dating a check that never happened.
            (BUSY, ["up to date", "checked 2026-09-06 12:00", "last action: a check is already running"]),
            (HELD, [HELD["held"] + " staged; installs after the run"]),
            (FAILED, [FAILED["last_error"]]),
            # IMPORTANT 2: the check succeeded and the last action did not. BOTH are on the row.
            (ACTION_ERROR, ["up to date", "last action: " + ACTION_ERROR["action_error"]]),
        ]:
            page.evaluate(f"window.__UPDATE = {json.dumps(u)};")
            page.click("#set-update-check")
            page.wait_for_timeout(300)
            got = page.inner_text("#set-update-state")
            for want in wants:
                if want not in got:
                    bad.append(f"the Updates row does not say {want!r} for that check ({got!r})")
            if CTX["version"] not in got:
                bad.append(f"the Updates row dropped the version in state {wants[0]!r} ({got!r})")
            if not page.is_hidden("#upd"):
                bad.append(f"nothing is offered in state {wants[0]!r}, so the topline must stay empty")

        # 6b. Plan 4a Task 8: the offer. A staged version reaching the page IS the offer — the
        #     mid-run gate lives in `updates::update_offer` in Rust, so the page never re-decides
        #     it — and *Restart to update* goes to `install_update` and nowhere else.
        page.evaluate(f"window.__UPDATE = {json.dumps(OFFERED)};")
        page.evaluate("window.__CALLS = []; window.__INSTALLED = false;")
        page.click("#set-update-check")
        page.wait_for_timeout(300)
        if calls(page, "check_for_updates") != [{}]:
            bad.append(f"Check now payload: {calls(page, 'check_for_updates')}")
        if page.is_hidden("#upd"):
            bad.append("a staged update is not offered in the topline")
        if "0.2.0" not in page.inner_text("#upd"):
            bad.append(f"the offer does not name the version ({page.inner_text('#upd')!r})")
        if page.evaluate("document.querySelectorAll('#upd [data-id]').length") != 0:
            bad.append("the offer carries data-id — it is not an observed object")
        if "0.2.0 ready" not in page.inner_text("#set-update-state"):
            bad.append(f"the Updates row does not follow the check ({page.inner_text('#set-update-state')!r})")
        # The offer sits in the topline, so the panel comes off before it is clicked — a click
        # Playwright cannot deliver raises instead of adding to `bad`, and the point here is the
        # payload, not the z-order (which section 4 already checks).
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)
        page.click("#upd [data-install]")
        page.wait_for_timeout(300)
        if calls(page, "install_update") != [{}]:
            bad.append(f"Restart to update payload: {calls(page, 'install_update')}")
        # A refusal (a slot started between the offer and the click) is SHOWN and the button comes
        # back — never a silent retry, never a disabled button with no explanation. A fresh check
        # rewrites #upd, so the button that was disabled above is replaced rather than re-enabled.
        page.evaluate("window.__INSTALL_RESULT = { ok: false, error: 'a slot is running' };")
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(300)
        page.click("#set-update-check")
        page.wait_for_timeout(300)
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)
        page.click("#upd [data-install]")
        page.wait_for_timeout(300)
        if page.locator("#upd [data-install]").is_disabled():
            bad.append("a refused install left the button disabled with nothing said")
        if page.locator(".refusal").count() == 0:
            bad.append("a refused install said nothing to the reader")
        # The toast is `position: fixed; top; right; z-index: 50` and lives 6 s — long enough to
        # sit over the panel's Close button and make section 7's click un-deliverable. Take it away
        # now that it has been asserted on, rather than sleeping out its lifetime.
        page.evaluate("document.querySelectorAll('.refusal').forEach(function (n) { n.remove(); });")
        page.evaluate("window.__INSTALL_RESULT = null;")
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(300)
        # R-P4a-24: the offer is CLEARED before it is hidden. Going from an offer to a state with
        # none must empty `#upd`, not merely hide it — a stale `[data-install]` left in a hidden div
        # is a button that would install last week's version the moment anything unhid it, and the
        # click handler finds it by selector, not by visibility.
        page.evaluate(f"window.__UPDATE = {json.dumps(HELD)};")
        page.click("#set-update-check")
        page.wait_for_timeout(300)
        if page.evaluate("document.getElementById('upd').innerHTML") != "":
            bad.append("the offer was hidden without being cleared")
        if page.locator("#upd [data-install]").count() != 0:
            bad.append("a dead Restart to update button was left in the page")

        # 6c. Gmail connect T9c (spec section 4.4): the Google row's states with a stubbed invoke,
        #     the poll timeout, and the two-step Disconnect. Copy is asserted verbatim from the spec.
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)
        page.evaluate(f"window.__ACCOUNT = {json.dumps(ACCOUNT)};")

        def open_google(status):
            page.evaluate(f"window.__GSTATUS = {json.dumps(status)}; window.__CALLS = [];")
            page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
            page.wait_for_timeout(400)

        def shown(sel):
            return not page.is_hidden(sel)

        for status, says, buttons in G_STATES:
            open_google(status)
            got = page.inner_text("#set-google-state")
            if says not in got:
                bad.append(f"the Google row says {got!r}, wanted {says!r}")
            for sel in G_BUTTONS:
                if shown(sel) != (sel in buttons):
                    bad.append(f"the Google row ({says[:24]!r}) shows {sel} = {shown(sel)}")
            note = page.inner_text("#set-google-note")
            if G_TESTING not in note:
                bad.append(f"the Google row lacks the Testing sentence ({note!r})")
            page.keyboard.press("Escape")
            page.wait_for_timeout(100)

        # A poll that never finishes: timers are collapsed so twenty 3 s waits take no time.
        page.evaluate("window.__realST = window.__realST || window.setTimeout.bind(window); "
                      "window.setTimeout = function (f) { return window.__realST(f, 0); };")
        open_google(G_CAL)
        page.click("#set-google-connect")
        page.wait_for_timeout(1500)
        if calls(page, "google_connect") != [{"scope": "gmail"}]:
            bad.append(f"Connect Gmail payload: {calls(page, 'google_connect')}")
        if G_TIMEOUT not in page.inner_text("#set-google-note"):
            bad.append(f"the poll timeout sentence is missing ({page.inner_text('#set-google-note')!r})")
        if G_CAL_SAYS not in page.inner_text("#set-google-state"):
            bad.append("the row did not repaint from one fresh status after the timeout")
        page.keyboard.press("Escape")
        page.wait_for_timeout(100)

        # Two-step Disconnect: step 2 is hidden until step 1, step 1 names Calendar and calls nothing.
        open_google(G_GMAIL)
        if shown("#set-google-disconnect-2"):
            bad.append("Yes, disconnect is visible before step 1")
        page.click("#set-google-disconnect-1")
        page.wait_for_timeout(200)
        if "Google Calendar too" not in page.inner_text("#set-google-note") or not shown("#set-google-disconnect-2"):
            bad.append("step 1 did not reveal the Calendar confirm and Yes, disconnect")
        if calls(page, "google_disconnect"):
            bad.append("step 1 disconnected already")
        page.keyboard.press("Escape")
        page.wait_for_timeout(100)
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(400)
        if shown("#set-google-disconnect-2"):
            bad.append("closing Settings did not hide step 2")
        # ok: false keeps the row and shows the error under it.
        page.evaluate("window.__GDISCONNECT = { ok: false, error: 'Google could not be reached.' };")
        page.click("#set-google-disconnect-1")
        page.click("#set-google-disconnect-2")
        page.wait_for_timeout(400)
        if calls(page, "google_disconnect") != [{}]:
            bad.append(f"Yes, disconnect payload: {calls(page, 'google_disconnect')}")
        if "Google could not be reached." not in page.inner_text("#set-google-note"):
            bad.append("a refused disconnect showed no error")
        if "Gmail is connected" not in page.inner_text("#set-google-state"):
            bad.append("a refused disconnect changed the row")
        # ok: the row asks again and shows the new truth.
        page.evaluate(f"window.__GDISCONNECT = null; window.__GSTATUS = {json.dumps(G_NONE)};")
        page.click("#set-google-disconnect-1")
        page.click("#set-google-disconnect-2")
        page.wait_for_timeout(400)
        if "Gmail is not connected" not in page.inner_text("#set-google-state"):
            bad.append(f"the row did not repaint after Disconnect ({page.inner_text('#set-google-state')!r})")
        page.keyboard.press("Escape")
        page.wait_for_timeout(100)
        page.evaluate("window.__ACCOUNT = null;")

        # 7. Both ways out, and the tray's one way in.
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)
        if not page.is_hidden("#settings"):
            bad.append("Escape did not close the overlay")
        page.evaluate("window.KNOWLU_OPEN_SETTINGS()")
        page.wait_for_timeout(300)
        if page.is_hidden("#settings"):
            bad.append("KNOWLU_OPEN_SETTINGS (the tray's way in) did not open the overlay")
        page.click("#set-close")
        page.wait_for_timeout(200)
        if not page.is_hidden("#settings"):
            bad.append("Close did not close the overlay")
        browser.close()


def main(argv):
    out = Path(argv[1]) if len(argv) > 1 else REPO / "shots"
    out.mkdir(parents=True, exist_ok=True)
    fixture = REPO / "engine" / "tests" / "fixtures" / "surface-today-full.json"
    tmp = Path(tempfile.mkdtemp(prefix="knowlu-settings-check-"))
    bad = []
    try:
        shutil.copytree(REPO / "app" / "static", tmp / "app" / "static")
        (tmp / "fixtures").mkdir()
        shutil.copy2(fixture, tmp / "fixtures" / fixture.name)

        # Quiet, and stopped on the way out: the default handler logs every GET to stderr, and a
        # `serve_forever` thread whose socket is closed under it prints a traceback after the
        # verdict line. A check's output is its result and nothing else.
        class Quiet(http.server.SimpleHTTPRequestHandler):
            def __init__(self, *a, **k):
                super().__init__(*a, directory=str(tmp), **k)

            def log_message(self, *a):
                pass

        with socketserver.TCPServer(("127.0.0.1", 0), Quiet) as srv:
            port = srv.server_address[1]
            server = threading.Thread(target=srv.serve_forever, daemon=True)
            server.start()
            try:
                check(f"http://127.0.0.1:{port}/app/static/index.html", fixture.name, out, bad)
            finally:
                srv.shutdown()
                server.join(timeout=5)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    for b in bad:
        print(b)
    print("ok" if not bad else f"{len(bad)} failure(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
