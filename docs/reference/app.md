# Knowlu, the app

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Recount before quoting a number.

- `app/src/commands.rs` computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx(vault)` (`via: "dashboard"`; the actor is the vault's own token, read from `config/actor.yaml` on every write, and a bad file is the command's named `error` with nothing written). **Tauri commands, recounted 2026-09-30 (M2 editing, after Gmail connect and M1 grades merged in; the
  merge-time recount gave 54 / 29 / 73, the 2026-09-29 one 47 / 29 / 66)** (by script,
  over the two `generate_handler!` lists in `app/src/main.rs`; C3′ added none):
  the console window registers **59** (phase 2 added `answer_card`, `commitment_proposals`,
  `commitments_confirm`, `your_week`, `preview_window`; M1 added `grades::grades_status`,
  `grades_connect`, `grades_refresh`, `grades_forget`; Gmail connect added `account::google_status`,
  `account::google_connect` and `account::google_disconnect`; M2 added `set_body`, `profile`, `set_preferences`, `set_interests` and
  `dropped_events`), the vault-less picker/wizard window **29**
  (C2's hand-off H9 phase (a) added `account::google_connect_url`, `account::google_connected`,
  `account::open_external`; C1b's H1 removed `account::sign_up` and `account::sign_in` with the
  password and added `account::google_sign_in` to both lists) — **78** distinct. Commands live
  beside the module they serve (`commands.rs`, `week.rs`, `onboarding.rs`, `account.rs`,
  `report.rs`, `lms_link.rs`), never all in one file. **Thirteen** mutate notes (`set_fields`, `set_body`, `set_preferences`, `set_interests`,
  `create_task`, `delete_note`, `decide`, `answer_card`, `commitments_confirm`, `close_info`,
  `open_issue`, `resolve_issue`, `sync` — the last applies another desktop's writes through `write`
  and can file a `kind: amend` card); `set_fields` edits a commitment's `kind`/`level` only as
  `commitments::check_console_edit` allows; `backup_now` is the one command that moves the vault
  without writing a note; `ui_event` writes the `state/events-ui/` ledger; everything else touches
  app data, `profiles.json`, the clipboard, the process or the updater — never a note. Recount
  before quoting a number.
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
  `app/tests/grades.rs` keeps it so until privacy bump #1. Before any release that writes `grades/`,
  migration `20260929000100` must be applied to the server.
- **App data is `%LOCALAPPDATA%\knowlu\`**: `profiles.json`, `profiles\<profile_id>\{settings.json,
  seen.txt, logs\}`, shared `updates\`, `runtime\`, `models\`. `state::app_data_root()` is the one
  place the path is decided. `profiles::migrate_flat_layout` still folds an old flat
  `%LOCALAPPDATA%\quinn-ops\` root in, file by file — that literal is the only `quinn-ops` left in
  `app/src`, and it stays.
- A slot is `sync → coursework → ingest → grades → judge → rank` (`scheduler::slot_argv`), each the sibling
  `knowlu-engine.exe` as a child process (`KNOWLU_ENGINE_EXE` overrides). Steps are left out and
  named — `ingest (skipped: no ics_url)`, `judge (skipped: no runtime)` / `(skipped: no model)`,
  `sync (skipped: no account)` / `(skipped: no entitlement)` / `(skipped: another sync is running)`,
  `grades (skipped: <why>)` (the Grades bullet below) —
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless the vault's `config/runners.yaml` `local` entry says `scheduler: app` for this `device:`; a
  wizard-created vault carries both from birth.
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
- `app/src/inference.rs`: `SUPPORTED_RUNTIMES` is a compiled-in table of release tag, asset name and
  SHA-256; both install paths verify against it, so there is no unverified path to executing a
  runtime. Adding a release is a code change. Model files are data, not pinned. Not bundled.
- Plain `cargo build` / `cargo test` work on a fresh checkout because `app/build.rs` drops a
  zero-byte placeholder sidecar at `app/binaries/knowlu-engine-<triple>.exe`. **Releases are CI-only**
  (`.github/workflows/release.yml`, on a `v*` tag): `scripts\release.ps1` is what CI runs; a human runs
  it only with `-DryRun`, which bundles unsigned and publishes nothing. A hand-run `cargo tauri build`
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
  llama.cpp runtime (`app/src/inference.rs`, `engine/src/runtime.rs`). C4 is no longer a phase:
  that runtime code stays until the Pilot's runtime-removal lane (Amendment 2026-09-29, ruling 10)
  removes it, and is not extended.
- Cut day (spec §7.2) is a procedure with Quinn at the machine: the old `quinn-ops` vault is archived,
  not migrated; Quinn re-onboards into `%USERPROFILE%\Knowlu\`.
