"""Headless behaviour check for the wizard and the settings overlay (plan 4a, Tasks 6-7).

Serves a copy of app/static/ from a temp dir over loopback (never the repo — ruling R40) and
installs a fake __TAURI__ that records every invoke. No Tauri, no vault, no network.

Screenshots do not catch a Next button that skips a panel, a validation that never fires, a
password field that is still full after the write, or a summary that states a slot time the user
changed a moment ago — those are what this drives.

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
PARENT = "C:\\Docs\\Knowlu"
DEST_OLD = PARENT + "\\Fall 2026"
DEST_NEW = PARENT + "\\Spring 2027"
# Decision 3: until Finish is clicked, the only commands the wizard may have called are the two
# that read and the two that write a credential. Anything else here would mean something reached
# the disk before the user said go.
BEFORE_FINISH_OK = {"launch_state", "pick_folder", "store_credentials", "retarget_credentials"}
FAKE = """
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'wizard', profiles: [], machine: 'M',
      tz: 'America/Chicago', documents: 'C:\\\\Docs\\\\Knowlu', campuses: [{ key: 'none', label: 'None' }] }); }
  if (cmd === 'pick_folder') { return Promise.resolve({ ok: true, path: 'C:\\\\Docs\\\\Knowlu' }); }
  // A write that fails: the wizard must keep the user on panel 5 with what they typed.
  if (cmd === 'store_credentials') {
    return (args.user || '').indexOf('fail') === 0
      ? Promise.resolve({ ok: false, error: 'credential write failed for ' + args.source })
      : Promise.resolve({ ok: true, error: null, target: 'knowlu/p/' + args.source }); }
  if (cmd === 'retarget_credentials') { return Promise.resolve({ ok: true, error: null, moved: 1 }); }
  if (cmd === 'create_vault' || cmd === 'restore_vault' || cmd === 'adopt_vault') {
    return Promise.resolve({ ok: true, error: null, profile: { id: 'p1', name: 'Spring 2027', vault: 'C:\\\\Docs\\\\Knowlu\\\\Spring 2027' } }); }
  if (cmd === 'apply_profile_settings') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'get_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: null, autostart: true, quit_at: null } }); }
  if (cmd === 'settings_context') { return Promise.resolve({ ok: true, vault: 'C:\\\\v', version: '0.1.0', profile_name: 'Ada' }); }
  if (cmd === 'set_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: 'C:\\\\b', autostart: true, quit_at: null } }); }
  return Promise.resolve({ ok: true, error: null });
} } };
window.__CALLS = [];
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
    # 1. The wizard is what a zero-profile launch shows, on panel 1.
    if page.is_hidden("#wizard") or page.is_hidden("#wiz-welcome"): bad.append("wizard did not open on panel 1")
    # 2. Next walks the panels in order; Back returns.
    page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-vault"): bad.append("Next did not reach panel 2")
    page.click("#wiz-back"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-welcome"): bad.append("Back did not return to panel 1")
    page.click("#wiz-next"); page.click("#wiz-pick-parent"); page.wait_for_timeout(200)
    page.fill("#wiz-name", "Fall 2026"); page.wait_for_timeout(120)
    if DEST_OLD not in page.inner_text("#wiz-vault-path"): bad.append("the resulting path is not shown")
    # 3a. R-P4a-25: the backup folder may not be the vault or sit inside it, and a vault INSIDE the
    #     backup folder is allowed — the mirror writes under <backup>\<profile>\vault. The fake
    #     picker answers with PARENT, which is the vault's own parent and so exactly that case; it
    #     is also the wizard's default parent, so refusing it made the obvious choice unpickable.
    page.click("#wiz-next"); page.wait_for_timeout(150)
    page.click("#wiz-pick-bdir"); page.wait_for_timeout(250)
    if PARENT not in page.inner_text("#wiz-bdir"): bad.append("the chosen backup folder is not shown on panel 3")
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-lms"): bad.append(f"a backup folder holding the vault was refused: {page.inner_text('#wiz-error')!r}")
    # Back to panel 3 and Skip, so the rest of the run is the no-backup path it has always been.
    page.click("#wiz-back"); page.wait_for_timeout(120)
    # 3. The ICS shape is validated, and a bad one blocks Next.
    page.click("#wiz-skip-backup"); page.wait_for_timeout(150)
    page.fill("#wiz-ics", "not-a-url"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-lms"): bad.append("a malformed ICS url did not block Next")
    page.fill("#wiz-ics", "https://lms.example.invalid/x/learn.ics"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-logins"): bad.append("Next did not reach the credentials panel")
    # 4a. Half a login is refused by name, not skipped in silence (review round 1, minor).
    page.fill("#wiz-zy-user", "a@example.invalid"); page.wait_for_timeout(60)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-logins"): bad.append("a half-filled login pair did not block Next")
    if "both" not in page.inner_text("#wiz-error"): bad.append("a half-filled pair said nothing about needing both")
    if "store_credentials" in names(page): bad.append("a half-filled pair was written anyway")
    # 4b. A FAILED write keeps the user on the panel, with what they typed (IMPORTANT 2).
    secret = secrets.token_urlsafe(12)
    page.fill("#wiz-zy-user", "fail@example.invalid"); page.fill("#wiz-zy-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-logins"): bad.append("a failed credential write advanced anyway")
    if page.input_value("#wiz-zy-pass") == "": bad.append("a failed write cleared the fields the user must retype")
    if "credential write failed" not in page.inner_text("#wiz-error"): bad.append("a failed write said nothing")
    # 4c. …and a good one writes, clears the fields, and names the whole destination path.
    page.fill("#wiz-zy-user", "a@example.invalid"); page.wait_for_timeout(60)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-slots"): bad.append("a stored credential did not advance to panel 6")
    if "store_credentials" not in names(page): bad.append("store_credentials was not invoked")
    if page.input_value("#wiz-zy-pass") != "": bad.append("the password field was not cleared")
    # Only the `vault` field is read back out of the recorder — never the secret beside it.
    cred_vault = page.evaluate("(window.__CALLS.filter(c => c[0] === 'store_credentials').slice(-1)[0] || [null, {}])[1].vault || ''")
    if cred_vault != DEST_OLD: bad.append(f"store_credentials named {cred_vault!r}, not {DEST_OLD!r}")
    # 5. Panel 6 is live: a slot typed here is what panel 7 says and what the payload carries
    #    (IMPORTANT 3).
    page.fill("#wiz-slot1", "09:00"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not reach the finish panel")
    summary = page.inner_text("#wiz-summary")
    if "09:00" not in summary: bad.append(f"the summary did not pick up the edited slot: {summary!r}")
    if DEST_OLD not in summary: bad.append("the summary does not say where the vault will be")
    # 6. R-P4a-23: go Back and rename — which is exactly what a refused Finish asks for — and the
    #    credentials written under the old path must MOVE before anything is created.
    for _ in range(5):
        page.click("#wiz-back"); page.wait_for_timeout(90)
    if page.is_hidden("#wiz-vault"): bad.append("Back did not walk all the way to panel 2")
    page.fill("#wiz-name", "Spring 2027"); page.wait_for_timeout(120)
    for _ in range(5):
        page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not walk back to the finish panel")
    if DEST_NEW not in page.inner_text("#wiz-summary"): bad.append("the summary did not follow the rename")
    # 7. Decision 3, measured: nothing but the reads and the credential writes has happened yet.
    before = set(names(page))
    if not before <= BEFORE_FINISH_OK: bad.append(f"something reached disk before Finish: {sorted(before - BEFORE_FINISH_OK)}")
    page.click("#wiz-next"); page.wait_for_timeout(400)
    order = names(page)
    if "retarget_credentials" not in order:
        bad.append("a rename after panel 5 did not move the credentials")
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
        if not str(plan.get("ics_url") or "").endswith(".ics"): bad.append("the plan did not carry the LMS feed")
        if plan.get("zybooks") is not True: bad.append("the plan did not record that a zyBooks login was stored")
        if plan.get("timezone") != "America/Chicago": bad.append("the plan did not carry the timezone")
        if plan.get("slots") != ["09:00", "18:00"]: bad.append(f"the plan carried slots {plan.get('slots')!r}")
    if "finish_onboarding" not in order: bad.append("finish_onboarding was not invoked")
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
