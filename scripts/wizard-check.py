"""Headless behaviour check for the wizard and the settings overlay (plan 4a, Tasks 6-7; the
nine-panel C1 wizard from hand-off H6 of `docs/plans/2026-09-09-c1-accounts-plan.md`).

Serves a copy of app/static/ from a temp dir over loopback (never the repo — ruling R40) and
installs a fake __TAURI__ that records every invoke. No Tauri, no vault, no network.

Screenshots do not catch a Next button that skips a panel, a validation that never fires, a
password field that is still full after the write, or a summary that states a slot time the user
changed a moment ago — those are what this drives.

A second page boots the same files as a CONSOLE over a vault `rank` has not reached yet (C1c Task
5, D7, re-ruled by R-C1c-8): while `state` carries `first_run` the first-run view replaces the day
and names each step the live slot has published, it says so when the first slot ends without a
day, and on the next poll after the block stops coming the ranked day takes its place whole.

Run:
    .wv\\Scripts\\python scripts/wizard-check.py
Prints one line: `ok`, or one `FAIL: …` per broken behaviour and a count. Exit 0 only when clean,
and nothing else on stdout — the request log is silenced and the server is shut down before the
serving thread's socket goes, so a green run is one word.

The zyBooks password typed below is minted at runtime and never leaves the fake page's memory:
the recorder keeps it in `window.__CALLS`, and nothing here reads it back out (spec §5 — no secret
in a repo, a test literal or a log line).
"""
import http.server, secrets, shutil, socketserver, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[1]
DEST_OLD = "C:\\Users\\Ada\\Knowlu\\Fall 2026"
DEST_NEW = "C:\\Users\\Ada\\Knowlu\\Spring 2027"
# Decision 3, restated for C1: until Finish is clicked, the only commands the wizard may have called
# are the ones that READ, the ones that make the ACCOUNT (which is not this machine's disk), the two
# that write a credential, and the sign-in window's own three. Anything else here would mean
# something reached this machine's disk before the user said go.
BEFORE_FINISH_OK = {"launch_state", "pick_folder", "google_sign_in", "send_magic_link",
                    "verify_email_code", "entitlement_now", "open_checkout", "open_policy",
                    "open_lms_window", "capture_calendar_link", "capture_courses",
                    "paste_calendar_link", "close_lms_window", "discover_coursework",
                    "campus_search", "timezone_for_state",
                    "store_credentials", "retarget_credentials"}
# A raw string: the JavaScript below is the page's, backslashes and all.
FAKE = r"""
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'wizard', profiles: [], machine: 'M',
      tz: 'America/Chicago', default_parent: 'C:\\Users\\Ada\\Knowlu',
      default_backup: 'C:\\Users\\Ada\\Knowlu\\Backups',
      }); }
  if (cmd === 'google_sign_in') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
  if (cmd === 'send_magic_link') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'verify_email_code') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
  if (cmd === 'open_checkout') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'entitlement_now') { return Promise.resolve({ ok: true, error: null, status: (window.__ENTITLED ? 'trialing' : 'none'), plan: 'monthly', current_period_end: null }); }
  if (cmd === 'open_policy') { return Promise.resolve({ ok: true, error: null }); }
  // R-C1-40 I2: the command answers with no session directory — the page never learns where the
  // sign-in window keeps its data.
  if (cmd === 'open_lms_window') { return Promise.resolve({ ok: true, error: null, opened: true }); }
  if (cmd === 'timezone_for_state') { return Promise.resolve({ ok: true, error: null, timezone: 'America/Chicago' }); }
  if (cmd === 'campus_search') {
    return Promise.resolve({ ok: true, error: null,
      hits: [[100751, 'The University of Alabama', 'Tuscaloosa', 'AL']] }); }
  // R-C1-41 M4: every capture envelope carries `kind` and `note`; the page reads `.note` on both paths.
  if (cmd === 'capture_calendar_link') {
    return Promise.resolve({ ok: true, error: null, kind: 'lms_ics', note: null,
      link: { url: 'https://lms.example.invalid/feed/a.ics', events: 12, courses: 4 } }); }
  if (cmd === 'paste_calendar_link') {
    return Promise.resolve({ ok: true, error: null, kind: args.kind, note: null,
      link: { url: args.url, events: 12, courses: 4 } }); }
  if (cmd === 'close_lms_window') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'capture_courses') {
    return Promise.resolve({ ok: true, error: null, typed: false,
      courses: [{ code: 'UACS100Fall2026', name: 'CS 100 Intro', slug: 'cs-100', label: 'CS 100' }] }); }
  if (cmd === 'discover_coursework') {
    return window.__DISCOVER_EMPTY
      ? Promise.resolve({ ok: true, error: null, note: 'We could not reach your coursework sites', rows: [] })
      : Promise.resolve({ ok: true, error: null, note: null, rows: [
        { source: 'zybooks', key: 'UACS100Fall2026', detail: null, suggested: 'CS 100', mapped: false, ignored: false },
        { source: 'vhl', key: '2102121', detail: 'course 1623220', suggested: null, mapped: false, ignored: false },
        // Final review, I2: one row left blank (never touched) and one row the student ticks as
        // ignored — proves `wizFinish` puts only the ticked one in `zybooks_ignore`.
        { source: 'zybooks', key: 'HowToUseZyBooks2', detail: null, suggested: null, mapped: false, ignored: false },
        { source: 'zybooks', key: 'AnotherOldBook2020', detail: null, suggested: null, mapped: false, ignored: false }] }); }
  if (cmd === 'store_credentials') {
    return (args.user || '').indexOf('fail') === 0
      ? Promise.resolve({ ok: false, error: 'credential write failed for ' + args.source })
      : Promise.resolve({ ok: true, error: null, target: 'knowlu/p/' + args.source }); }
  if (cmd === 'retarget_credentials') { return Promise.resolve({ ok: true, error: null, moved: 1 }); }
  if (cmd === 'create_vault') {
    return Promise.resolve({ ok: true, error: null, profile: { id: 'p1', name: args.name, vault: 'C:\\Users\\Ada\\Knowlu\\' + args.name } }); }
  if (cmd === 'account_status') { return Promise.resolve({ ok: true, needs_account: false }); }
  if (cmd === 'get_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: null, autostart: true, quit_at: null } }); }
  if (cmd === 'settings_context') { return Promise.resolve({ ok: true, vault: 'C:\\v', version: '0.1.0', profile_name: 'Ada' }); }
  if (cmd === 'set_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: 'C:\\b', autostart: true, quit_at: null } }); }
  return Promise.resolve({ ok: true, error: null });
} } };
window.__ENTITLED = false;
window.__DISCOVER_EMPTY = false;
window.__CALLS = [];
"""

# The console's first-run view (C1c Task 5, D7; R-C1c-8). The 2026-09-23 live proof saw no first-run
# line and the view's only check was a string search of the source, so this boots the real page as a
# console. `state` answers what `commands::state_envelope` answers for a never-ranked vault with a
# slot in flight (`app/tests/commands.rs` pins that half): `ok`, a day to paint, and `first_run` with
# the live slot's steps and the one in progress — until the check clears `window.__FIRST_RUN`, which
# is `rank` having written `state/today.md`. The day is the s1 read-model reference, read here and
# never written.
#
# R-C1c-exec-8a (I1): the window opens on the vault as it was before the slot (nothing in Must do
# yet), and the check swaps in the ranked s1 day while the view still stands, as the real first slot
# does once coursework has imported work. `window.__REJECT` makes that many `state` calls reject.
STATE_FIXTURE = REPO / "engine" / "tests" / "fixtures" / "surface-today-s1.json"
D7 = "Knowlu is doing its first run. Your day appears here in about a minute."
CONSOLE_FAKE = r"""
window.__FIRST_RUN = { running: true, current: "judge",
                       steps: [["pull", 0], ["ingest (skipped: no ics_url)", 0],
                               ["entitlement (refresh failed: the account service did not answer)", 0],
                               ["coursework", 0]] };
window.__RANKED = window.__STATE;
window.__STATE = JSON.parse(JSON.stringify(window.__RANKED));
window.__STATE.must_do.groups = [];
window.__STATE.revision = "pre-slot";
window.__REJECT = 0;
window.__CALLS = [];
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'console', profiles: [] }); }
  if (cmd === 'state' && window.__REJECT > 0) {
    window.__REJECT -= 1;
    return Promise.reject(new Error('the state call was rejected'));
  }
  if (cmd === 'state') {
    var env = { ok: true, error: null, state: JSON.parse(JSON.stringify(window.__STATE)) };
    if (window.__FIRST_RUN) { env.first_run = window.__FIRST_RUN; }
    return Promise.resolve(env);
  }
  if (cmd === 'account_status') { return Promise.resolve({ ok: true, needs_account: false }); }
  return Promise.resolve({ ok: true, error: null });
} } };
"""


class Quiet(http.server.SimpleHTTPRequestHandler):
    """The default handler logs every GET to stderr; a check's only output is its verdict."""

    def log_message(self, *args):
        pass


def names(page):
    return page.evaluate("window.__CALLS.map(c => c[0])")


def first_args(page, cmd):
    """The arguments of the first `cmd` invoke, or None. Never the secret: callers ask by field."""
    return page.evaluate(f"(window.__CALLS.filter(c => c[0] === '{cmd}')[0] || [null, null])[1]")


def check(page) -> list:
    bad = []
    # 1. The wizard is what a zero-profile launch shows, on panel 1 of nine.
    if page.is_hidden("#wizard") or page.is_hidden("#wiz-welcome"): bad.append("wizard did not open on panel 1")
    if "of 9" not in page.inner_text("#wiz-step"): bad.append(f"step counter says {page.inner_text('#wiz-step')!r}")

    # 2. Panel 2 is the account. Continue with Google leads it, there is no password field anywhere,
    #    and neither door opens until both boxes are ticked (spec §9's minors row).
    page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-account"): bad.append("Next did not reach the account panel")
    if page.query_selector("#wiz-pw"): bad.append("the account panel still has a password field")
    if not page.query_selector("#wiz-google-signin"): bad.append("there is no Continue with Google button")
    page.click("#wiz-google-signin"); page.wait_for_timeout(200)
    if "google_sign_in" in names(page): bad.append("a Google sign-in ran with the boxes unticked")
    if "Tick both" not in page.inner_text("#wiz-error"): bad.append("the refusal said nothing about the boxes")
    page.check("#wiz-18"); page.check("#wiz-terms")
    page.click("#wiz-google-signin"); page.wait_for_timeout(300)
    if "google_sign_in" not in names(page): bad.append("google_sign_in was not invoked")
    ga = first_args(page, "google_sign_in") or {}
    # Tauri v2 lower-camel-cases argument keys; `age_attested` here would pass the fake and fail the
    # real command with a missing argument (R-C1b-exec-6: the Rust-side gate is back on the Google path).
    if ga.get("ageAttested") is not True: bad.append("google_sign_in did not carry the attestation")
    if page.is_hidden("#wiz-subscribe"): bad.append("a signed-in account did not advance to the subscribe panel")

    # 3. Subscribe opens Checkout in the system browser and polls until the account is entitled
    #    (R-C1b-exec-9). A Next off the subscribe panel re-asks the service rather than trusting a
    #    stale WIZ.entitled — and once the account really is entitled, a second Next press
    #    never opens a second Checkout page.
    page.click("#wiz-sub-month"); page.wait_for_timeout(3600)
    if names(page).count("open_checkout") != 1: bad.append("open_checkout was not invoked exactly once")
    if (first_args(page, "open_checkout") or {}).get("plan") != "monthly": bad.append("open_checkout named the wrong plan")
    if "entitlement_now" not in names(page): bad.append("the wizard did not poll for the subscription")
    if page.is_hidden("#wiz-subscribe"): bad.append("an unentitled account left the subscribe panel")
    page.click("#wiz-next"); page.wait_for_timeout(400)
    if page.is_hidden("#wiz-subscribe"): bad.append("Next advanced an account the service still calls unentitled")
    if "Finish the payment page" not in page.inner_text("#wiz-error"): bad.append("Next's re-ask did not say what was still missing")
    page.evaluate("window.__ENTITLED = true")
    page.click("#wiz-next"); page.wait_for_timeout(400)
    if page.is_hidden("#wiz-vault"): bad.append("an entitled account did not advance to the name panel")
    if names(page).count("open_checkout") != 1: bad.append("a second Next press opened a second Checkout page")

    # 4. Panel 4 names the setup. NO folder is picked, and the path is shown before Finish.
    page.fill("#wiz-name", "Fall 2026"); page.wait_for_timeout(120)
    if DEST_OLD not in page.inner_text("#wiz-vault-path"): bad.append("the resulting path is not shown")
    if page.query_selector("#wiz-pick-parent") or page.query_selector("#wiz-pick-bdir"):
        bad.append("the wizard still offers a folder picker")

    # 5. The calendars panel: BOTH calendars, before logins and Gmail (spec §11a).
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-calendars"): bad.append("Next did not reach the calendars panel")
    if not page.query_selector("#wiz-calendars #wiz-school"): bad.append("the school search is not on the calendars panel")
    # R-OB-4: the school is typed, not picked off two radios. Three letters, one hit, one click.
    page.fill("#wiz-school", "alabama"); page.wait_for_timeout(250)
    hits = page.query_selector_all("#wiz-school-hits [data-school]")
    if not hits: bad.append("typing a school name found nothing in campuses.json")
    else: hits[0].click()
    page.wait_for_timeout(200)
    if "Alabama" not in page.input_value("#wiz-school"): bad.append("picking a school did not fill the field")
    page.click("#wiz-lms-open"); page.wait_for_timeout(250)
    if (first_args(page, "open_lms_window") or {}).get("unitid") != "100751":
        bad.append("the sign-in window was opened for the wrong school")
    # R-C1-42: the capture is the student's SECOND press. Chained onto the open it would read the
    # identity provider's page, where nobody has signed in yet.
    if page.is_hidden("#wiz-lms-capture"): bad.append("the capture button did not appear once the window opened")
    page.click("#wiz-lms-capture"); page.wait_for_timeout(400)
    if "12 events" not in page.inner_text("#wiz-lms-state"): bad.append("the captured feed was not summarised")
    # …and the personal one, by its secret address, under its own kind.
    page.fill("#wiz-cal-ics", "https://calendar.google.com/calendar/ical/x/private-def/basic.ics")
    page.dispatch_event("#wiz-cal-ics", "change"); page.wait_for_timeout(250)
    paste = [c[1] for c in page.evaluate("window.__CALLS") if c[0] == "paste_calendar_link"]
    if not any((a or {}).get("kind") == "calendar_ics" for a in paste):
        bad.append("the personal calendar was not validated under kind calendar_ics")
    if "already on your calendar" not in page.inner_text("#wiz-cal-note"):
        bad.append("the personal calendar was not summarised")
    if not page.query_selector("#wiz-google:not([disabled])"):
        bad.append("the Google sign-in must be live from C2 on")
    # R-OB-2: the captured class list is on this panel, with a typed fallback beside it.
    if page.is_hidden("#wiz-courses"): bad.append("the class list did not appear after the sign-in")
    if "CS 100" not in page.inner_text("#wiz-course-rows"): bad.append("the captured course is not listed")
    page.fill("#wiz-course-add", "GN 103"); page.click("#wiz-course-add-go"); page.wait_for_timeout(120)
    if "GN 103" not in page.inner_text("#wiz-course-rows"): bad.append("a typed course was not added")

    # 6. Coursework logins: half a pair is a typo, not a choice; a failed write keeps the user here.
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-logins"): bad.append("Next did not reach the credentials panel")
    if "close_lms_window" not in names(page): bad.append("the sign-in window was not closed on leaving")
    page.fill("#wiz-zy-user", "a@example.invalid"); page.wait_for_timeout(60)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-logins"): bad.append("a half-filled login pair did not block Next")
    if "both" not in page.inner_text("#wiz-error"): bad.append("a half-filled pair said nothing about needing both")
    secret = secrets.token_urlsafe(12)
    page.fill("#wiz-zy-user", "fail@example.invalid"); page.fill("#wiz-zy-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-logins"): bad.append("a failed credential write advanced anyway")
    if page.input_value("#wiz-zy-pass") == "": bad.append("a failed write cleared the fields the user must retype")

    # R-C1b-exec-10: an empty discovery — no rows, for any reason — must not trap the student on
    # this panel, and must say something rather than show an empty div.
    page.evaluate("window.__DISCOVER_EMPTY = true")
    page.fill("#wiz-zy-user", "a@example.invalid"); page.fill("#wiz-zy-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(400)
    if page.is_hidden("#wiz-logins"): bad.append("an empty discovery left the credentials panel on its own")
    if names(page).count("discover_coursework") != 1: bad.append("an empty discovery did not run exactly once")
    if page.is_hidden("#wiz-map"): bad.append("an empty discovery hid the panel — the student saw nothing")
    if page.inner_text("#wiz-map-rows").strip() != "": bad.append("an empty discovery still listed rows")
    if "You can go on" not in page.inner_text("#wiz-map-note"): bad.append("an empty discovery did not say the way forward")
    if not page.is_hidden("#wiz-map-heading"): bad.append("an empty discovery still showed the mapping heading")
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-gmail"): bad.append("an empty discovery trapped the student on the logins panel")
    if names(page).count("discover_coursework") != 1: bad.append("Next re-ran discovery on an already-finished empty result")

    page.click("#wiz-back"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-logins"): bad.append("Back did not return to the credentials panel")
    page.evaluate("window.__DISCOVER_EMPTY = false")
    # R-OB-1: the first Next after a successful store runs discovery and STAYS on the panel with the
    # rows; the second one moves on. A wizard that took the password and skipped the mapping is the
    # run this exists because of. The fields were cleared by the earlier successful store, so they
    # are re-typed here — a re-typed login is a new answer and runs discovery again.
    page.fill("#wiz-zy-user", "a@example.invalid"); page.fill("#wiz-zy-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(400)
    if page.is_hidden("#wiz-logins"): bad.append("the mapping step was skipped after the credentials were stored")
    if "discover_coursework" not in names(page): bad.append("discovery did not run after the credentials were stored")
    if page.is_hidden("#wiz-map"): bad.append("the mapping rows did not appear")
    rows = page.inner_text("#wiz-map-rows")
    if "UACS100Fall2026" not in rows or "2102121" not in rows: bad.append(f"the discovered sources are not listed: {rows!r}")
    if page.input_value('[data-course-for="0"]') != "CS 100": bad.append("the suggestion was not pre-filled")
    if names(page).count("discover_coursework") != 2: bad.append("re-typed logins did not run discovery again")
    # D6 / R-C1c-plan-3: the first Next after a discovery with blank rows STAYS on the panel and
    # says what they cost — the engine files a card for each (R-OB-1) and the app asks there; the
    # second Next goes on. `is_visible` is the assertion that matters: `inner_text` falls back to
    # `textContent` and passes on a hidden panel, which is the bug this check exists for.
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-logins"): bad.append("the blank-row sentence did not keep the student on the panel")
    if not page.is_visible("#wiz-map-note"): bad.append("the count sentence was written to a hidden panel")
    if "asked about in the app" not in page.inner_text("#wiz-map-note"):
        bad.append("a blank mapping row did not say it would be asked about in the app")
    if names(page).count("discover_coursework") != 2: bad.append("staying to warn re-ran discovery")
    # D5: the row offers the classes the wizard already captured, by the code the vault will use.
    if page.get_attribute('[data-course-for="1"]', "list") != "wiz-course-codes":
        bad.append("the mapping row does not offer the captured classes")
    opts = page.eval_on_selector_all("#wiz-course-codes option", "os => os.map(o => o.value)")
    if "CS 100" not in opts: bad.append(f"the datalist does not carry the captured class: {opts!r}")
    page.fill('[data-course-for="1"]', "GN 103"); page.wait_for_timeout(120)
    # Final review, I2: row 2 (HowToUseZyBooks2) is left untouched — blank, un-ticked — and row 3
    # (AnotherOldBook2020) is ticked as ignored. `zybooks_ignore` must carry the ticked one and
    # never the blank one, so the blank one stays UNMAPPED and gets a coursework-map card.
    page.check('[data-ignore-for="3"]'); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-gmail"): bad.append("a confirmed mapping did not advance to the Gmail panel")
    if page.input_value("#wiz-zy-pass") != "": bad.append("the password field was not cleared")
    cred_vault = page.evaluate("(window.__CALLS.filter(c => c[0] === 'store_credentials').slice(-1)[0] || [null, {}])[1].vault || ''")
    if cred_vault != DEST_OLD: bad.append(f"store_credentials named {cred_vault!r}, not {DEST_OLD!r}")

    # 7. Gmail is honest and does nothing; slots are live and the summary follows them.
    if "test user" not in page.inner_text("#wiz-gmail"): bad.append("the Gmail panel does not say it is testing-mode only")
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-slots"): bad.append("Next did not reach the slots panel")
    page.fill("#wiz-slot1", "09:00"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not reach the finish panel")
    summary = page.inner_text("#wiz-summary")
    if "09:00" not in summary: bad.append(f"the summary did not pick up the edited slot: {summary!r}")
    if DEST_OLD not in summary: bad.append("the summary does not say where the vault will be")

    # 8. R-P4a-23 still holds: Back, rename, forward — and the credentials MOVE before anything exists.
    for _ in range(5):
        page.click("#wiz-back"); page.wait_for_timeout(90)
    if page.is_hidden("#wiz-vault"): bad.append("Back did not walk all the way to the name panel")
    page.fill("#wiz-name", "Spring 2027"); page.wait_for_timeout(120)
    for _ in range(5):
        page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not walk back to the finish panel")
    if DEST_NEW not in page.inner_text("#wiz-summary"): bad.append("the summary did not follow the rename")

    # 9. Decision 3, measured: nothing but reads, the account and the credentials has happened yet.
    before = set(names(page))
    if not before <= BEFORE_FINISH_OK: bad.append(f"something reached disk before Finish: {sorted(before - BEFORE_FINISH_OK)}")
    page.click("#wiz-next"); page.wait_for_timeout(500)
    order = names(page)
    if "retarget_credentials" not in order:
        bad.append("a rename after the logins panel did not move the credentials")
    elif "create_vault" not in order or order.index("retarget_credentials") > order.index("create_vault"):
        bad.append("the credentials were moved after the vault was created, not before")
    rt = first_args(page, "retarget_credentials") or {}
    if rt.get("from_vault") != DEST_OLD or rt.get("to_vault") != DEST_NEW:
        bad.append(f"retarget_credentials was called with {rt.get('from_vault')!r} -> {rt.get('to_vault')!r}")
    created = first_args(page, "create_vault")
    if not created:
        bad.append("Finish did not invoke create_vault")
    else:
        plan = created.get("plan") or {}
        if created.get("name") != "Spring 2027": bad.append(f"create_vault got name {created.get('name')!r}")
        if "parent" in created: bad.append("create_vault still takes a parent folder")
        if "backup_dir" in plan: bad.append("the plan still carries a backup folder")
        if not str(plan.get("ics_url") or "").endswith(".ics"): bad.append("the plan did not carry the captured feed")
        if not str(plan.get("personal_calendar") or "").endswith(".ics"): bad.append("the plan did not carry the personal calendar")
        if ((plan.get("campus_choice") or {}).get("unitid")) != "100751": bad.append("the plan did not carry the school")
        zy = plan.get("zybooks_courses") or []
        if not any(b.get("code") == "UACS100Fall2026" and b.get("label") == "CS 100" for b in zy):
            bad.append(f"the plan did not carry the zyBooks mapping: {zy!r}")
        # H6-4: the field `create_vault_in` actually slugs; a row whose `course` is empty is dropped
        # in Rust without a sound, so `label` alone would pass a mapping that never lands.
        if not any(b.get("course") == "CS 100" for b in zy):
            bad.append("the zyBooks mapping carried no course code — create_vault_in drops a row whose `course` is empty")
        # Final review, I2: the ticked row is ignored; the blank, un-ticked row is not — it must
        # stay unmapped so the engine's coursework-map card path (R-OB-1) can still reach it.
        zi = plan.get("zybooks_ignore") or []
        if "AnotherOldBook2020" not in zi:
            bad.append(f"a ticked zyBooks row did not reach zybooks_ignore: {zi!r}")
        if "HowToUseZyBooks2" in zi:
            bad.append(f"a blank, un-ticked zyBooks row was sent as ignored: {zi!r}")
        vh = plan.get("vhl_sections") or []
        if not any(v.get("section") == "2102121" and v.get("label") == "GN 103" for v in vh):
            bad.append(f"the plan did not carry the VHL mapping: {vh!r}")
        if not any(v.get("course") == "GN 103" for v in vh):
            bad.append("the VHL mapping carried no course code")
        if not any(c[0] == "CS 100" for c in (plan.get("course_map") or [])):
            bad.append("the plan did not carry the course map")
        # Final review, I4: the CAPTURED course 'UACS100Fall2026' already carries its own slug
        # ('cs-100', from `capture_courses` above) and is covered by the engine's own
        # `course_fragments` — the page must never send its LMS id into `course_map` a second time
        # with a phantom empty slug, which `create_vault_in` would fill and which would then win
        # over the real one (first-wins by key).
        if any(c[0] == "UACS100Fall2026" for c in (plan.get("course_map") or [])):
            bad.append("a captured course's LMS id was sent in course_map with a phantom slug")
        if not any(c.get("code") == "UACS100Fall2026" for c in (plan.get("courses") or [])):
            bad.append("the plan did not carry the enrolled courses")
        if plan.get("zybooks") is not True: bad.append("the plan did not record that a zyBooks login was stored")
        if plan.get("timezone") != "America/Chicago": bad.append("the plan did not carry the timezone")
        if plan.get("slots") != ["09:00", "18:00"]: bad.append(f"the plan carried slots {plan.get('slots')!r}")
    if "finish_onboarding" not in order: bad.append("finish_onboarding was not invoked")
    return bad


def state_polls(page) -> int:
    return page.evaluate("window.__CALLS.filter(c => c[0] === 'state').length")


def first_run_block(page) -> dict:
    """What the view actually is on screen: its computed display, not its `hidden` attribute —
    `console.css` has taught three times that a class `display` beats the UA's `[hidden]` — each
    listed step's state and words, and the day's headline as painted (hidden or not)."""
    return page.evaluate("""() => {
      const el = document.getElementById('first-run');
      return { display: getComputedStyle(el).display, text: el.innerText,
               rows: Array.from(document.querySelectorAll('#first-run-steps [data-state]'))
                 .map(r => ({ state: r.getAttribute('data-state'), text: r.textContent })),
               headline: document.getElementById('headline').textContent };
    }""")


def listed(fr, say) -> list:
    """The states of every listed row that says `say` — one, when the view is right."""
    return [r["state"] for r in fr["rows"] if say in r["text"]]


# `nav`, the right-hand rail and the day's own column: R-C1c-8 hides all three while the view stands.
THE_DAY = [("nav", "the nav"), (".app > aside", "the rail"), ("#main-today", "the day")]


def check_first_run(page, errors) -> list:
    bad = []
    # 1. A slot in flight: the view REPLACES the day (R-C1c-8) — judged by what is on screen, never
    #    by an attribute — and names each published step in plain words: the account and coursework
    #    as done, the calendar as skipped (saying so), the live `current` as in progress, and nothing
    #    for the pull, which is the Runs view's. The day still paints, hidden.
    fr = first_run_block(page)
    if fr["display"] == "none" or not page.is_visible("#first-run"):
        bad.append(f"the first-run view is not on screen while state carries first_run (display {fr['display']!r})")
    for sel, what in THE_DAY:
        if page.is_visible(sel): bad.append(f"{what} is on screen beside the first-run view")
    if D7 not in fr["text"]: bad.append(f"the first-run view does not say D7's sentence: {fr['text']!r}")
    for say, state in [("Checking your account", "failed"), ("Fetching your coursework", "done"),
                       ("Reading your school calendar", "skipped"), ("Working out what each task needs", "now")]:
        if listed(fr, say) != [state]: bad.append(f"{say!r} is not listed once as {state!r}: {fr['rows']!r}")
    if not any("Reading your school calendar" in r["text"] and "skipped" in r["text"] for r in fr["rows"]):
        bad.append(f"the skipped step does not say it was skipped: {fr['rows']!r}")
    if any("pull" in r["text"].lower() for r in fr["rows"]): bad.append(f"the pull, which is not listed, has a row: {fr['rows']!r}")
    # R-C1c-final2 M1: the account check that could not reach the service lands at code 0, and the
    # view shows it as failed with a short note — never as a check mark.
    if not any("Checking your account" in r["text"] and "couldn't check" in r["text"] for r in fr["rows"]):
        bad.append(f"the failed account check does not say it could not check: {fr['rows']!r}")
    if len(fr["rows"]) != 4: bad.append(f"the view does not list exactly the four named steps: {fr['rows']!r}")
    if page.is_visible("#first-run-end"): bad.append("a slot still running says the first run did not finish")
    if not fr["headline"].strip(): bad.append("the day did not paint behind the first-run view")
    # The topline gear is hidden with the day, so the view carries its own way to Settings, routed by
    # the document's `[data-settings]` delegation.
    if not page.is_visible("#first-run [data-settings]"):
        bad.append("there is no Settings button on screen in the first-run view")
    else:
        page.click("#first-run [data-settings]"); page.wait_for_timeout(150)
        if not page.is_visible("#settings"): bad.append("the first-run view's Settings button did not open Settings")
        page.click("#set-close"); page.wait_for_timeout(100)
    # 2. M1: a rejected `state` call does not end the three-second cadence — the error line is hidden
    #    in this mode, so a stalled view would say nothing. A focus poll afterwards restarts a chain
    #    that did end, so the steps below still test what they are about.
    page.evaluate("window.__REJECT = 1")
    page.wait_for_timeout(3600)
    if page.evaluate("window.__REJECT") != 0: bad.append("the rejected state call never happened")
    after_reject = state_polls(page)
    page.wait_for_timeout(3600)
    if state_polls(page) <= after_reject: bad.append("a rejected state call ended the first-run view's three-second cadence")
    page.evaluate("window.dispatchEvent(new Event('focus'))"); page.wait_for_timeout(300)
    # Mid-slot, coursework has imported work and the day under the view changes (I1).
    page.evaluate("window.__STATE = window.__RANKED")
    # 3. The slot ends without a day: the list stays, the failed step is marked, nothing is in
    #    progress, and one more line says Knowlu will try again. The cadence carries on (M6).
    before = state_polls(page)
    page.evaluate("""window.__FIRST_RUN = { running: false, current: null, steps: [
      ["entitlement (refreshed)", 0], ["coursework", 0], ["judge", 0], ["rank", 1]] }""")
    page.wait_for_timeout(3600)
    if state_polls(page) <= before: bad.append("the first-run view did not poll again within its three-second cadence")
    fr = first_run_block(page)
    if listed(fr, "Putting your day in order") != ["failed"]: bad.append(f"the failed rank step is not marked failed: {fr['rows']!r}")
    if any(r["state"] == "now" for r in fr["rows"]): bad.append(f"an ended slot still shows a step in progress: {fr['rows']!r}")
    if not page.is_visible("#first-run-end") or "didn't finish" not in page.inner_text("#first-run-end"):
        bad.append("a first slot that ended without a day did not say so")
    # 4. M2 (R-C1c-exec-8a): the failure can be a step this view does not list — the engine missing,
    #    at -1. The line still says the run did not finish, and the unlisted step still has no row.
    page.evaluate("""window.__FIRST_RUN = { running: false, current: null, steps: [
      ["judge (skipped: no entitlement)", 0], ["engine: not found", -1], ["push", 0]] }""")
    page.wait_for_timeout(3600)
    fr = first_run_block(page)
    if not page.is_visible("#first-run-end"): bad.append(f"a first slot that failed on an unlisted step did not say it did not finish: {fr['rows']!r}")
    if [r["state"] for r in fr["rows"]] != ["skipped"]: bad.append(f"the unlisted steps were listed: {fr['rows']!r}")
    # …while the line itself keys on codes alone: an ended slot whose only trouble is that account
    #    check (code 0) marks the row failed and does not say the run did not finish.
    page.evaluate("""window.__FIRST_RUN = { running: false, current: null, steps: [
      ["entitlement (refresh failed: the account service did not answer)", 0], ["coursework", 0]] }""")
    page.wait_for_timeout(3600)
    fr = first_run_block(page)
    if listed(fr, "Checking your account") != ["failed"]: bad.append(f"an ended slot's failed account check is not marked failed: {fr['rows']!r}")
    if page.is_visible("#first-run-end"): bad.append("a code-0 account check made the view say the first run did not finish")
    # 5. More than the two-second dwell has passed in first-run mode: a row nobody could see must
    #    never have been reported as seen.
    seen = page.evaluate("window.__CALLS.filter(c => c[0] === 'ui_event' && c[1] && c[1].action === 'object_seen').length")
    if seen: bad.append(f"{seen} object_seen event(s) were sent for rows hidden behind the first-run view")
    # 6. `rank` writes the day: the three-second poll drops the view and the ranked day takes its
    #    place whole — Must do as ranked, never the pre-slot rows held behind a refresh-order button
    #    (I1). This state's revision is the one the view last saw, the harder case: a poll that
    #    matched it would return before painting at all.
    before = state_polls(page)
    page.evaluate("window.__FIRST_RUN = null")
    page.wait_for_timeout(3600)
    if state_polls(page) <= before: bad.append("the first-run view did not poll again within its three-second cadence")
    fr = first_run_block(page)
    if fr["display"] != "none": bad.append(f"the first-run view stayed on screen after the day arrived (display {fr['display']!r})")
    for sel, what in THE_DAY:
        if not page.is_visible(sel): bad.append(f"{what} did not come back once the day arrived")
    if not page.inner_text("#headline").strip(): bad.append("the day came back without its headline")
    shown = page.evaluate("Array.from(document.querySelectorAll('#mustdo .row[data-id]')).map(r => r.getAttribute('data-id'))")
    ranked = page.evaluate("window.__RANKED.must_do.groups.flatMap(g => g.rows.map(r => r.id))")
    if not ranked or shown != ranked:
        bad.append(f"the hand-over showed Must do as {shown!r}, not the ranked day's {ranked!r}")
    if page.is_visible("#refresh-order"): bad.append("the hand-over offered a refresh-order button instead of the ranked day")
    # 7. …and the cadence ends with it: nothing but the minute-long interval polls after that.
    settled = state_polls(page)
    page.wait_for_timeout(3600)
    if state_polls(page) != settled: bad.append("the three-second first-run poll kept running after the day arrived")
    # 8. The reset was the hand-over's alone: once the day is up, R28's hold works as it always did.
    #    A new order arrives, Must do keeps the order on screen, and "refresh order" offers the new one.
    page.evaluate("""(() => { const s = JSON.parse(JSON.stringify(window.__RANKED));
      s.must_do.groups.forEach(g => g.rows.reverse()); s.revision = 'reordered'; window.__STATE = s; })()""")
    page.evaluate("window.dispatchEvent(new Event('focus'))"); page.wait_for_timeout(400)
    held = page.evaluate("Array.from(document.querySelectorAll('#mustdo .row[data-id]')).map(r => r.getAttribute('data-id'))")
    if held != ranked or not page.is_visible("#refresh-order"):
        bad.append(f"after the hand-over a reorder no longer holds behind refresh order (Must do {held!r})")
    for e in errors: bad.append(f"console page error: {e}")
    return bad


def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="knowlu-wizard-check-"))
    try:
        shutil.copytree(REPO / "app" / "static", tmp / "app" / "static")
        handler = lambda *a, **k: Quiet(*a, directory=str(tmp), **k)
        srv = socketserver.TCPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        try:
            url = f"http://127.0.0.1:{srv.server_address[1]}/app/static/index.html"
            with sync_playwright() as p:
                browser = p.chromium.launch()
                page = browser.new_context(viewport={"width": 1280, "height": 900}).new_page()
                page.add_init_script(FAKE)
                page.goto(url); page.wait_for_timeout(400)
                bad = check(page)
                console = browser.new_context(viewport={"width": 1280, "height": 860}).new_page()
                errors = []
                console.on("pageerror", lambda e: errors.append(str(e)))
                console.add_init_script("window.__STATE = " + STATE_FIXTURE.read_text(encoding="utf-8") + ";\n" + CONSOLE_FAKE)
                console.goto(url); console.wait_for_timeout(400)
                bad += check_first_run(console, errors)
                browser.close()
            for line in bad: print("FAIL:", line)
            print("ok" if not bad else f"{len(bad)} failure(s)")
            return 1 if bad else 0
        finally:
            # Shut the loop down before the socket closes under it — otherwise the daemon thread
            # dies in select() and prints a traceback over the verdict.
            srv.shutdown()
            srv.server_close()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
