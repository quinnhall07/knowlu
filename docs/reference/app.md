# Knowlu, the app

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Recount before quoting a number.

- `app/src/commands.rs` computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx(vault)` (`via: "dashboard"`; the actor is the vault's own token, read from `config/actor.yaml` on every write, and a bad file is the command's named `error` with nothing written). **Tauri commands, recounted 2026-10-03 (email forwarding: ten `mail::` commands; the 2026-10-01 recount gave 60 / 29 / 79; the 2026-09-30 recount, M2 editing after Gmail connect and M1 grades merged in, gave 59 / 29 / 78; the
  merge-time recount gave 54 / 29 / 73, the 2026-09-29 one 47 / 29 / 66)** (by script,
  over the two `generate_handler!` lists in `app/src/main.rs`; C3′ added none):
  the console window registers **71** (recounted 2026-10-21 by the same script when L7, carrying L4, merged into p3-registrar: L4 added `account::privacy_status` and `account::accept_privacy` to the console list, where the L4 branch alone counted 68 / 29 / 87; on 2026-10-20 main's runtime removal took out the four `inference` commands and the UA registrar added three `registrar::` ones, `open_registrar_window`, `capture_registrar` and `close_registrar_window`; the earlier build-up: email forwarding added the ten `mail::` commands; phase 2 added `answer_card`, `commitment_proposals`,
  `commitments_confirm`, `your_week`, `preview_window`; M1 added `grades::grades_status`,
  `grades_connect`, `grades_refresh`, `grades_forget`; Gmail connect added `account::google_status`,
  `account::google_connect` and `account::google_disconnect`; M2 added `set_body`, `profile`, `set_preferences`, `set_interests` and
  `dropped_events`; events added `remove_lane_date`), the vault-less picker/wizard window **29**
  (C2's hand-off H9 phase (a) added `account::google_connect_url`, `account::google_connected`,
  `account::open_external`; C1b's H1 removed `account::sign_up` and `account::sign_in` with the
  password and added `account::google_sign_in` to both lists) — **90** distinct (2026-10-21). Commands live
  beside the module they serve (`commands.rs`, `week.rs`, `onboarding.rs`, `account.rs`,
  `report.rs`, `lms_link.rs`, `mail.rs`, `registrar.rs`), never all in one file. **Thirteen** mutate notes (`set_fields`, `set_body`, `set_preferences`, `set_interests`,
  `create_task`, `delete_note`, `decide`, `answer_card`, `commitments_confirm`, `close_info`,
  `open_issue`, `resolve_issue`, `sync` — the last applies another desktop's writes through `write`
  and can file a `kind: amend` card); `set_fields` edits a commitment's `kind`/`level` only as
  `commitments::check_console_edit` allows; `backup_now` is the one command that moves the vault
  without writing a note; `ui_event` writes the `state/events-ui/` ledger; everything else touches
  app data, `profiles.json`, the clipboard, the process or the updater — never a note. Recount
  before quoting a number.
- **The UA registrar (commitment model phase 3; spec `docs/specs/2026-09-26-commitment-model-phase3-design.md`).**
  Three console commands in `app/src/registrar.rs`, none in the wizard's list. `open_registrar_window`
  opens `lms_link`'s incognito, capability-less sign-in window on the school's registrar
  (`scaffold::Registrar`, chosen by `school_of` from `config/campus.yaml`'s `unitid`; UA only, any
  other school gets the fixed line "Knowlu can't read your school's class schedule yet").
  `capture_registrar(view)` reads that window's cookies for the registrar host only
  (`lms_link::cookie_url`; they stay on the call's stack and never reach a file, a log or an error),
  fetches the term the engine names (`fetch_term`; the app never computes a term), writes the bodies to
  one temp file under the profile's `tmp\`, runs `commitments --registrar <file> --school <key>`
  (`registrar_argv`: `--via dashboard`, no `--actor`, so the engine writes as the vault's human token)
  under `vault_io` and the sync run lock (it refuses by name while a sync holds it), deletes the file
  whatever happened, closes and wipes the window (once bytes reach the engine the session is thrown
  away, whatever the engine says, so a retry needs a fresh sign-in) and returns the rebuilt state; it
  keeps the window open only when Banner answers with its sign-in page. `close_registrar_window` closes
  and wipes. `sweep_stale_files_in` removes `registrar-*.json` a crash left behind. A failed request
  is reported in fixed words (`fetch_error`), never ureq's text, which can carry the URL. The engine's
  exit 2 becomes a fixed sentence on the page; the page never shows engine text verbatim. The console
  builder manages `LmsSession`, wipes on the `lms-signin` window's `Destroyed` and at exit, and never
  touches `grades::WINDOW` or a mail window. Contract: `docs/reference/engine-commands.md`.
- **M2 editing: bodies and the profile** (spec `docs/specs/2026-09-29-m2-editing-design.md`). Five
  console commands in `app/src/commands.rs`, each an `_inner` function plus a thin wrapper, none in the
  wizard's list. `set_body_inner(view, id, expected, body)` refuses a non-id before the engine is
  asked, then refuses any note whose folder is outside `BODY_EDITABLE_FOLDERS` (`tasks`, `courses`),
  then calls `write::set_body` with `console_ctx(vault)`; the envelope is `{ok, error, state}` plus
  `conflict: true` only when the refusal was a stale `expected` (`WriteError::Conflict`), so the page
  keeps the draft. `profile_inner` returns `profile::read` (preferences body, the four interest lists,
  `interests_editable`, warnings) and writes nothing. `set_preferences_inner(view, expected, text)`
  calls `profile::set_preferences`; `set_interests_inner(view, strong, mild, never, clubs)` calls
  `profile::set_interests`, which refuses a block-style list by name (`write::value_spans_lines`)
  before any record. `dropped_events_inner` returns `eventroster::read_dropped` and writes nothing.
  `commands.rs` hashes, parses and decides nothing.
- **`remove_lane_date`** (events P17; console list only, beside `delete_note`).
  `remove_lane_date_inner(view, uid)` calls `eventcarry::remove_lane_date` with `console_ctx(vault)`
  and today in the vault's zone, and returns the usual `{ok, error, state}` envelope. The engine
  gates it: only the vault's own human token may call it (an `agent:` or `system:` actor, another
  human token or an invalid `config/actor.yaml` is a named refusal), the uid must be an accepted
  lane date the day draws, and the one write is a `declined` line in `state/events-seen.md`
  (`eventledger::record_declined`), with no note and no journal record. It is not one of the thirteen
  note-mutating commands. The console sends it only after the 10-second "Removed · Undo" toast
  closes (`removeFromMyDay` in `app/static/console.js`); Undo or a closed window sends nothing.
- **`set_body`'s record holds no text.** `write::set_body` journals first, then replaces the body
  after the head and its separator. The record is `op: set_body`, `field: null`, with `old` and `new`
  each `{sha256, bytes}` (`write::body_sha256`, lowercase hex over the body's UTF-8 bytes); the body is
  normalised (`\r\n` to `\n`, outer newlines trimmed, one final `\n`) so the next edit's hash chains.
  An unchanged body is a no-op that writes no record. `surface::describe` reads it as `<path>: body
  edited (<who>)`. A stale `expected` is `WriteError::Conflict` and the note is untouched.
- **Two `profile/` files, local only.** `profile/preferences.md` (a free-text body, created empty) and
  `profile/interests.md` (four one-line flow lists `strong`, `mild`, `never`, `clubs`), made on the
  student's first save by `write::create_profile_file` and read by `profile::read`. They are not in
  `NOTE_FOLDERS`, so `ids` never stamps them and `sync::build_push` never pushes them: a journal record
  about a path that fails `sync::is_note_path` stays on this computer (D13, the `continue` in
  `build_push`). Only the task's or course's own `set_body` record and note go up. The settings page
  tells the student which text the judge reads (the first 600 characters, for one call, not kept).
- **Not shown (N)** under *Coming up* is read-only: `dropped_events` is called when *Coming up*
  renders, not on every poll. `eventroster::read_dropped` reads the audit section of
  `state/events.md` and lists only filtered, declined and judged-not-relevant events, each with its
  reason; unjudged events never appear (Quinn's PQ2 = no). Only a `drop` verdict reads "judged not
  relevant"; an `unsure` verdict is not listed, as it may still have an open card asking the student
  (ruled 2026-09-30, Quinn).
- **Google connection from Settings (Gmail connect).** Three console commands in `app/src/account.rs`,
  each reading the profile's session from `cfg.session_credential_target` (the wizard's
  `google_connect_url`, `google_connected` and `open_external` keep `PENDING_TARGET`, and call the same
  cores). `google_status` returns `{ok, state, calendar, gmail, email, error}` (`state` is `none`,
  `active`, `quiet` or `revoked`; an older server's reply without `status` reads `active` when connected,
  else `none`). `google_connect(scope)` takes `calendar`, `gmail` or `reconnect`, opens the consent URL
  from Rust only if it passes `external_url_allowed`, and returns `{ok, error}`. `google_disconnect`
  sends a bearer `DELETE` to `google-connect` and returns `{ok, error}`. Errors: 402 "your subscription
  is not active, so Google cannot be connected" (Connect and Reconnect only: the status read and
  Disconnect need a session, not a subscription, so a lapsed student can still disconnect), 502 "Google could not be reached to disconnect; try
  again", plus 401, 503 and the generic form. No token, URL or Google email is logged. The Settings row
  (`#set-google`, after `#set-account`) renders one state per `google_status` reply, polls every 3 s up
  to 20 times after Connect or Reconnect, and disconnects in two steps (a warning that Calendar goes
  too and that undelivered Gmail proposals are deleted, then **Yes, disconnect**). The wizard's Gmail
  step has a **Connect Gmail** button (`#wiz-gmail-connect`) and never blocks Next. Spec:
  `docs/specs/2026-09-29-gmail-connect-design.md` §4.2 and §4.4.
- **Grades** (spec `docs/specs/2026-09-29-grades-design.md`; ruling 12). `grades::availability(row,
  campus_lms)` is the one predicate: grades are available only for a curated campus row whose
  `lms_kind` is `blackboard` and which carries a `policy_read` date, and then only on that row's own
  `lms_host`. No `cfg`, feature or environment variable reaches it. Its four callers check it and never
  re-derive it: `grades_status`, `grades_connect`, `grades_refresh` and the scheduler's `grades_step`.
  A no answers with a named refusal: `not available at your school yet` (an uncurated Blackboard school,
  or a curated row without a date) or `not a Blackboard school`; the slot records the same as
  `grades (skipped: not available at your school yet)` / `(skipped: not a Blackboard school)`, at exit
  0. The four commands live in `app/src/grades.rs` (`grades_forget` is never gated: it closes the
  `lms-grades` window and deletes the kept session). *Hide grades* is a settings row like *Start with
  Windows*: `set_settings`'s `grades_hidden` key writes `grades::GradesPrefs` (`<profile>\grades.json`)
  and never `settings.json`, and `grades_status` reports it as `hidden`. The slot's other skips, in order: `no entitlement`,
  `not connected`, `no window on this run`, the sign-in window open, then the capture's own outcomes
  (signed out, Blackboard unreachable). A capture writes a bundle private to that run
  (`grades::slot_bundle_path`) for the engine's `grades` step, deleted afterwards. That step, the
  slot's or a Refresh's, holds `state/sync.lock` itself, so it never overlaps a sync or another
  `grades` run. It refuses a bundle older than the last one applied, so the slot's capture (taken
  before `sync`) never undoes a Refresh made while it waited. A Refresh whose step skipped answers
  the skip by name (`grades::step_answer`). No grade value is
  logged. No curated row carries a `policy_read` date on this branch; the date-and-bump test in
  `app/tests/grades.rs` keeps it so until privacy bump #1, which is now set (`PRIVACY_BUMP_1` is `2026-10-20`). Before any release that writes `grades/`,
  migration `20260929000100` must be applied to the server.
- **Email forwarding (the mail window)** (spec `docs/specs/2026-09-30-email-forwarding-design.md`;
  plan `docs/plans/2026-09-30-email-forwarding-plan.md`). Built on branch `email-forwarding`, not
  merged; no release is cut from a `main` carrying it before privacy bump #1 (D18), and `release.yml`'s
  first build step, `scripts/ci/privacy-guard.ps1`, fails one that tries. The app sets up
  forwarding from the student's school mailbox to a secret address the service mints, drives the
  mailbox's web page for it, and never reads a message. Recounted 2026-10-03 from the code.
  - **The ten commands** (`app/src/mail.rs`, thin wrappers over seams in `app/src/mail/commands.rs`;
    named in the console's `generate_handler!` list and not the wizard's): `mail_status`,
    `mail_setup_start(provider)`, `mail_setup_progress`, `mail_setup_answer(choice)`,
    `mail_setup_stop`, `mail_sign_in(address_id)`, `mail_disconnect(address_id)`,
    `mail_rotate(address_id)`, `mail_keep_session(on)` and `mail_applied(since)`. `mail_status`
    joins the service's mailbox rows to what this device keeps (`turned_off`, `open_backfill`,
    `saved_sign_in`) and adds `keep_session`, `offer_pending`, `setup_running` and
    `profile_delete_pending`; it still answers the device's own facts when the service cannot be
    read. `mail_applied` runs `knowlu-engine mail-applied` and hands its JSON through; a report that
    is not one JSON object is `{ok: false, error}`. None writes the vault: it changes only through the
    engine's `judge` (the mail pass).
  - **The `mail` window.** One label, `mail` (`mail::WINDOW`), and Google's "verify it's you" popup
    `mail-popup`, in the same profile. It has no Tauri capability grant, so every invoke from it is
    denied. It is visible only to sign in (`open_visible`), and every run on a saved sign-in uses
    `open_hidden` (out of the taskbar). `MailWindow::try_take` allows one holder at a time: setup, a
    repair sign-in, a Disconnect, the slot's mail step and the launch resume. A script runs only by
    name from `app/assets/mail/` (compiled in), only while the top-level page is on the provider's
    mail hosts (`mail::mail_hosts`), and its result returns through WebView2's completion value,
    never IPC. External-scheme launches are cancelled (`refuse_external_schemes`). Nothing
    simulates a keyboard or mouse.
  - **Kept profiles and `mail.json`** (`app/src/mail_session.rs`). One WebView2 profile per mailbox,
    `<profile data dir>\mail-session-<address_id>`, and with the saved sign-in off one throwaway
    `mail-setup`. `address_id` is the service's opaque id, never the address. `is_session_dir` is the
    one guard every delete passes: the exact child of an absolute data dir, named `mail-setup` or
    `mail-session-<id>`. `mail.json` sits beside `settings.json` (never a field in it) and holds
    `keep_session` (on by default), `last_reconcile`, `pending_deletes`, `turned_off` (only grows),
    and, only while any exists, `open_backfills` and `backfill_pace`. An unreadable file is an error,
    never the default: the slot's mail step then runs nothing, and every write refuses.
  - **The slot's `mail` step** runs after the grades capture and before the child-process chain
    (`scheduler::run_slot_with`, `mail::slot::run_step`). A mailbox is left out when it is turned off,
    guided, `revoked`, or not due (`RECONCILE_CADENCE` is `Daily`: due once a day when `active`, at
    once when it needs repair). The named skips: `mail (skipped: could not read your mailboxes)` (the
    service's list failed: no proof there is no mailbox), `(no entitlement)`, `(saved sign-in off)`,
    `(mail.json unreadable)`, `(not due)`, `(mail window open)`, `(no session)` (the pass could not
    have the account's client); no mailbox connected adds no row, and neither does a vault with no
    account. A kept profile that does not read as signed in is `needs sign-in`, never shown. The row is
    fixed words and counts, never an address, subject or sender, and stays out of
    `state/runner-log.md`. Pace constants `MAX_PER_MINUTE` (10) and `MAX_PER_DAY` (600) in
    `app/src/mail_steps.rs` are **provisional, pending S6**; the backfill window is `BACKFILL_DAYS`
    (120) before the setup day.
  - **Launch and deletion.** `mail::live::on_start` (called from `setup`) takes the mail window,
    sweeps a leftover `mail-setup`, retries `pending_deletes`, and resumes every open backfill whose
    profile is kept. `mail_keep_session(false)` records the switch first, closes the window and
    deletes every kept profile. `mail_disconnect` records the mailbox in `turned_off` before anything
    else, turns forwarding off, calls `DELETE /mail/addresses/<id>`, then deletes the profile (a failed
    delete is retried at launch and named in Settings). Sign-out and `delete_local_data` close the
    window and call `forget_all`, which deletes only those two folder shapes. **Sign-out's order** (L4 plan
    §4.7, `account::sign_out_with`): the mail-window hold, the unfinished-setup clean, every
    `pending_setups` record in `mail.json` cleared whatever the clean did, `forget_all`, the hold dropped,
    the session credential deleted, then the revoke. The reply carries `setups_cleared` beside `ok`,
    `had_session` and `mail_profiles_left`: `true` when `mail.json` holds no record afterwards (and when
    there was no profile folder to read).
- **App data is `%LOCALAPPDATA%\knowlu\`**: `profiles.json`, `profiles\<profile_id>\{settings.json,
  seen.txt, logs\}`, shared `updates\`. `state::app_data_root()` is the one
  place the path is decided. `profiles::migrate_flat_layout` still folds an old flat
  `%LOCALAPPDATA%\quinn-ops\` root in, file by file — that literal is the only `quinn-ops` left in
  `app/src`, and it stays.
- A slot is `sync → coursework → grades → ingest → judge → rank` (`scheduler::slot_argv`; grades before ingest since lane O1, so ingest can read the gradebook's submitted-work status), each the sibling
  `knowlu-engine.exe` as a child process (`KNOWLU_ENGINE_EXE` overrides). Steps are left out and
  named — `ingest (skipped: no ics_url)`, `judge (skipped: no account)` / `(skipped: no entitlement)`,
  `sync (skipped: no account)` / `(skipped: no entitlement)` / `(skipped: another sync is running)`,
  `grades (skipped: <why>)` (the Grades bullet below), `mail (skipped: <why>)` (the Email bullet) —
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless the vault's `config/runners.yaml` `local` entry says `scheduler: app` for this `device:`; a
  wizard-created vault carries both from birth.
- **The privacy update (L4; plan `docs/plans/2026-10-19-l4-honest-page-plan.md` §4).** `PRIVACY_VERSION`
  in `app/src/account.rs` is `2026-10-20` (privacy bump #1, commit `d4ee7ab`; it moves only in a commit
  that moves `site/privacy.html`'s date with it, and `app/tests/grades.rs`'s `PRIVACY_BUMP_1` is set to it).
  - **Two console commands** (in the console's `generate_handler!` list, not the wizard's):
    `privacy_status` and `accept_privacy`, thin wrappers over `privacy_status_in` and `accept_privacy_in`.
    `privacy_status` returns `{ok, error, current, recorded, needs_accept}`; it reads
    `GET /account/consent` with the vault's own session and caches the answer. A vault with no readable
    account answers `needs_accept: false`; a failed read answers `ok: false` and `needs_accept: null`
    (never a raised panel). `accept_privacy` re-sends the sign-in's consent for this build's version
    (`POST /account/consent`, which on an account that already consented moves `privacy_version` forward
    only), reads it back, and answers `ok: true` only if the read-back shows this version or a later one.
    The console calls `privacy_status` once per launch for a signed-in vault and raises the
    `#reconsent` panel only for `needs_accept: true`; the panel's Settings button opens Settings with the
    start controls disabled and Delete my data live.
  - **`privacy.json`** in the profile's app-data folder (beside `entitlement.json`) holds three keys,
    `account_id` (whose answer it is), `recorded` (the version the service said the account has, or
    `null`) and `checked_at` (RFC 3339, UTC, to the second). It is written only from a service reply,
    never from a request; it never syncs and enters neither the vault nor telemetry. None of the three is a
    new data class: the account id is already in `config/cloud.yaml` and `entitlement.json`, the version is
    a date the page publishes, and `checked_at` is the time of a read. The gate over it
    (`account::privacy_gate`) holds back only a definite `Older` (an earlier version, none, or a value that
    is not a `YYYY-MM-DD` day); no cache, a cache that does not read and another account's cache are
    `Unknown` and hold nothing.
  - **The scheduler's new skips.** For an entitled vault whose gate is not `Accepted`, the slot asks
    `account::refresh_privacy` once. A failed check is the row `privacy (check failed: <reason>)` at exit 0,
    recorded in the runner log as `privacy (check failed)`, and holds nothing. A definite `Older` turns the
    `sync`, `coursework`, `grades`, `ingest`, `judge` and `mail` steps into
    `<step> (skipped: privacy update not accepted)`; `rank`, the backup and telemetry run as always. The
    two naming rules: only a step that would have run gets the privacy name (`ingest` without a feed, a
    grades step that was already skipped and a mail pass with no mailbox keep their own names or no row),
    and the entitlement's names win (an unentitled account gets `(skipped: no entitlement)`, because the
    step would not have run whatever it accepts). *Sync now*, the quit push and the launch resume of open
    mail backfills are held the same way while the gate is `Older`; *Sync now* answers
    `sync (skipped: privacy update not accepted)` and the tray stays calm.
- Credentials the app writes are `knowlu/<profile_id>/<source>` (`app/src/credentials.rs`); the
  engine's `wincred.rs` reads whatever `credential_target` the vault names.
- The account's session JWT is Credential Manager's `knowlu/<profile_id>/session`
  (`app/src/account.rs`), moved there at onboarding from a pre-vault `knowlu/pending/session` entry;
  `config/cloud.yaml` names it alongside the project's `api_base`, its public `anon_key` and the
  `account_id`. Entitlement is cached at `profiles\<id>\entitlement.json` with a 72-hour grace, and
  past it every cloud step is a named skipped step, never a failure.
- There is no password on a Knowlu account: sign-in is `account::google_sign_in` (a loopback PKCE
  round trip on `127.0.0.1:0`, one listener per sign-in) or the emailed one-time code;
  `/auth/v1/signup` and `grant_type=password` are called by nothing.
- The identifier is **`com.knowlu.desktop`**, permanent: uninstall key, autostart entry and window state are keyed by it.
- The updater is configured: `tauri-plugin-updater`, `plugins.updater` (endpoint + minisign public
  key) and `bundle.createUpdaterArtifacts: true` are one decision — a static test pins flag ⇔ plugin.
  The private key exists **only** as the GitHub secret `TAURI_SIGNING_PRIVATE_KEY` (C0 Task 4
  regenerates it; the laptop's old Credential Manager copy is retired); `release.ps1` takes it from
  the environment and nowhere else. If it is ever lost, regenerate: one `pubkey` line and a release,
  and installed apps need one manual reinstall.
- Plain `cargo build` / `cargo test` work on a fresh checkout because `app/build.rs` drops a
  zero-byte placeholder sidecar at `app/binaries/knowlu-engine-<triple>.exe`. **Releases are CI-only**
  (`.github/workflows/release.yml`, on a `v*` tag): `scripts\release.ps1` is what CI runs; a human runs
  it only with `-DryRun`, which bundles unsigned and publishes nothing. It runs `cargo tauri build
  --target <triple>` (rustc's host triple, the one that names the staged sidecar) because CI's
  prebuilt MSVC tauri-cli would otherwise look for an MSVC-named sidecar, so the installer lands in
  `target\<triple>\release\bundle\nsis` and the app in `target\<triple>\release\knowlu.exe` (0.1.2,
  L10). A hand-run `cargo tauri build`
  is unsupported — it skips the clean-tree gate, the sidecar staging and the placeholder check, and
  can ship a zero-byte engine. `ci.yml` is the gate on every push and PR: `cargo test --workspace` at
  0 warnings (the gate prints `warnings: N accepted (.rsrc), N tallies, N other`), the eol contract
  (`scripts/ci/eol-check.ps1`), SHA-pinned actions (`engine/tests/workflows.rs`). A repository
  variable `CI_SELF_HOSTED` = `on` sends every `ci.yml` job to a self-hosted runner labelled
  `knowlu-ci`; unset, they run on GitHub's; `release.yml` never runs self-hosted. **Since
  2026-09-29 the repository is public, the variable is unset and no self-hosted runner is
  registered** — a self-hosted runner on a public repository runs any fork's pull request on the
  runner's machine, so the switch stays off for as long as the repository is public. Workflow runs
  from outside contributors need a maintainer's approval (`all_external_contributors`).
- Desktop safety: a live shared desktop — never synthetic keyboard/mouse input; screenshots by
  window handle (`PrintWindow`) only, never a full-screen grab. Develop and demo against scratch
  vaults (`scripts\scratch-vault.ps1 -Source <vault>`), never a live one.


## Direction, as it stood in CLAUDE.md (signed 2026-09-09)

- The authority is `docs/specs/2026-09-09-knowlu-cloud-design.md`: its §1 decisions D1–D12 are
  Quinn's and signed; §11's recommendations are ruled (R3: the academic-year price and the June–August
  pause both stay). Where an older spec or note disagrees with it, the cloud design wins.
- In one line: **accounts + $9.99/month, no free tier; every judgment runs in our cloud (Supabase +
  Cloudflare + Stripe, an open-weights model per kind through OpenRouter, pinned to a zero-retention
  host); CI builds and signs every release; the vault stays plain text on the student's machine and
  is created by the app; portal credentials never leave the device — fetch on device, think in the
  cloud.**
- The work is streams with disjoint files (`HANDOFF.md` §2): C0 CI release → C1 accounts, wizard,
  telemetry → C2 the judgment service → C3 sync (git leaves the product) → C4 removal of the local
  llama.cpp runtime. C4 is no longer a phase: the Pilot's runtime-removal lane (L6; Amendment
  2026-09-29, ruling 10) removed `app/src/inference.rs` and `engine/src/runtime.rs` (lane PR number
  and merge sha: `HANDOFF.md` §3). Older installs may still hold `runtime\` and `models\` folders
  under app data; nothing reads or removes them.
- Cut day (spec §7.2) is a procedure with Quinn at the machine: the old `quinn-ops` vault is archived,
  not migrated; Quinn re-onboards into `%USERPROFILE%\Knowlu\`.
