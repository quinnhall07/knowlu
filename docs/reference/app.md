# Knowlu, the app

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Recount before quoting a number.

- `app/src/commands.rs` computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx()` (`via: "dashboard"`). **Tauri commands, recounted 2026-09-29 (the merge of
  the commitment model into this file's branch; the 2026-09-26 recount of C3′ into phase 2 gave the
  same)** (by script, over the two `generate_handler!` lists in `app/src/main.rs`; C3′ added none):
  the console window registers **47** (phase 2 added `answer_card`, `commitment_proposals`,
  `commitments_confirm`, `your_week`, `preview_window`), the vault-less picker/wizard window **29**
  (C2's hand-off H9 phase (a) added `account::google_connect_url`, `account::google_connected`,
  `account::open_external`; C1b's H1 removed `account::sign_up` and `account::sign_in` with the
  password and added `account::google_sign_in` to both lists) — **66** distinct. Commands live
  beside the module they serve (`commands.rs`, `week.rs`, `onboarding.rs`, `account.rs`,
  `report.rs`, `lms_link.rs`), never all in one file. **Ten** mutate notes (`set_fields`,
  `create_task`, `delete_note`, `decide`, `answer_card`, `commitments_confirm`, `close_info`,
  `open_issue`, `resolve_issue`, `sync` — the last applies another desktop's writes through `write`
  and can file a `kind: amend` card); `set_fields` edits a commitment's `kind`/`level` only as
  `commitments::check_console_edit` allows; `backup_now` is the one command that moves the vault
  without writing a note; `ui_event` writes the `state/events-ui/` ledger; everything else touches
  app data, `profiles.json`, the clipboard, the process or the updater — never a note. Recount
  before quoting a number.
- **App data is `%LOCALAPPDATA%\knowlu\`**: `profiles.json`, `profiles\<profile_id>\{settings.json,
  seen.txt, logs\}`, shared `updates\`, `runtime\`, `models\`. `state::app_data_root()` is the one
  place the path is decided. `profiles::migrate_flat_layout` still folds an old flat
  `%LOCALAPPDATA%\quinn-ops\` root in, file by file — that literal is the only `quinn-ops` left in
  `app/src`, and it stays.
- A slot is `sync → coursework → ingest → judge → rank` (`scheduler::slot_argv`), each the sibling
  `knowlu-engine.exe` as a child process (`KNOWLU_ENGINE_EXE` overrides). Steps are left out and
  named — `ingest (skipped: no ics_url)`, `judge (skipped: no runtime)` / `(skipped: no model)`,
  `sync (skipped: no account)` / `(skipped: no entitlement)` / `(skipped: another sync is running)` —
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
