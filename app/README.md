# `app/` — the Knowlu desktop shell

**Status: Knowlu plan 4a, Tasks 1–11 (2026-09-06) — the shell now installs, onboards, configures
and updates itself.** It stopped being a window that needs a `--vault` on the command line: a first
launch runs a wizard, a machine with several vaults gets a picker, and settings are a panel rather
than a file to hand-edit. (Plan 1 Tasks 7–15 made it the way the system is *operated* rather than
viewed; that half is unchanged.) The product is called **Knowlu** wherever a user sees it (window title, tray tooltip,
message boxes, the page); the engine crate under `engine/` is `knowlu-engine` (library
`knowlu_engine`, binary `knowlu-engine.exe`). The exe is `knowlu.exe` (`[[bin]] name = "knowlu"`) and
the library target is `knowlu` too, so `app/tests/` and `main.rs` say `use knowlu::…`.

**The `identifier` is `com.knowlu.desktop`, and it is permanent** (Knowlu spec decision 8 — the
reverse-DNS of `knowlu.com`, which Quinn holds). The installer bakes it in as the uninstall key, and
`tauri_plugin_autostart`'s registry entry and the window-state file are keyed by it too, so moving it
after a friend has installed orphans all three. It changed from `app.knowlu.desktop` in plan 4a Task
9, before the first installer existed; the one visible cost was that the next launch re-registered
autostart and reset the window position once. `tauri.conf.json` has no comment syntax, hence this note.

The engine is linked as a path dependency (`knowlu-engine = { path = "../engine" }`). **Sixty-six**
(recounted 2026-09-24, commitment model phase 2, by script)
`#[tauri::command]`s exist — **twenty-seven in `src/commands.rs`, four in `src/week.rs`, fourteen in
`src/onboarding.rs`, fourteen in `src/account.rs`, five in `src/lms_link.rs` and two in
`src/report.rs`**. Commands live beside the module they serve, never all in one file. Count them in
`src/main.rs`'s two `generate_handler!` lists if this drifts, and note that the two lists are
different windows, not one:

- **The console window** registers 47 — all 27 of `commands.rs`; all four of `week.rs`;
  `launch_state`, `pick_folder` and `pick_file` from `onboarding.rs`; eleven of `account.rs`'s
  fourteen (the seven sign-in commands included, because an install that predates the account is
  upgraded in place, inside this window, over its own vault; the three it lacks are the picker-only
  Google ones below); and both of `report.rs`.
- **The vault-less shell** (picker or wizard) registers 29 — the 14 of `onboarding.rs`, ten of
  `account.rs` (`google_sign_in`, `send_magic_link`, `verify_email_code`, `sign_out`,
  `entitlement_now`, `open_checkout`, `open_policy`, plus `google_connect_url`, `google_connected`
  and `open_external` — C2's hand-off H9 phase (a), the wizard's Google button) and all five of
  `lms_link.rs`: there is no `ConsoleState` yet, so no command that needs one can be called.

In `commands.rs`: `state`, `note`, `mark_seen`, `ui_event` (read-only with respect to the vault's
notes); `set_fields`, `create_task`, `delete_note`, `decide`, `answer_card`, `close_info`, `open_issue`,
`resolve_issue` (mutating, each returning the fresh state so the page never holds a write as a
pending reorder); `sync`, `backup_now` (the vault's transport and mirror); `get_settings`,
`set_settings` (the settings file, not the vault); and plan 4a's seven — `set_profile_name`
(`profiles.json`), `settings_context`, `copy_diagnostics`, `copy_text`, `switch_profile`,
`check_for_updates`, `install_update`. In `onboarding.rs`: `launch_state`, `pick_folder`,
`adopt_vault`, `open_profile`, `create_vault`, `restore_vault`, `apply_profile_settings`,
`store_credentials`, `retarget_credentials`, `finish_onboarding`. Every mutating command marshals, calls
`knowlu_engine::write` (or `sync`/`backup` directly) and returns an envelope — **nothing in
`commands.rs` computes** (spec §3.1). Writes go through `write::WriteContext` with `actor:
"quinn"`, `via: "dashboard"` (`console_ctx()`). The **one** exception is `decide`'s in-process
approvals *processing* — the `process_approvals` pass that materializes a task, applies an
amendment or expands a digest right after the verdict is written — which uses `executor_ctx()`
(`agent:approvals`) so the new note's fields are not frozen as human-set by the console's own write
path (ruling R-T9). The verdict itself, and both issue commands (`open_issue`, `resolve_issue`),
are Quinn's own judgement and stay on `console_ctx()`.

In `week.rs`: `your_week` and `preview_window` (in-process reads), `commitment_proposals` and
`commitments_confirm` (the sibling engine, the latter with a temp file under the profile's `tmp\`
and under `vault_io`). `answer_card` (in `commands.rs`) writes a `commitment-ask` card's
`answer_meets` and approves it, the engine validating both (`commitments::check_answerable`,
`commitments::answer_literal`).

The shell is also a **tray application**: closing the window hides it rather than quitting, a
tray icon with seven menu items (`Open`, `Run now`, `Pause scheduling`, `Settings`, `Restart to
update`, `Copy diagnostics`, `Quit`) sits in the notification area, and autostart is wired through
`tauri-plugin-autostart` per the profile's settings file. `Run now` and `Pause scheduling` are wired
directly to `src/scheduler.rs` — no event round-trip through the page; `Settings` opens the same
overlay the topline gear does, by `eval`ing `window.KNOWLU_OPEN_SETTINGS()`. `Restart to update`
ships **disabled** and is enabled only once a bundle is staged and no slot is running. A tray click
has no window to answer in, so a refusal ("a slot is running…") or a failed install is written to
`Updates.last_action_error` and shown in the settings row rather than swallowed — the tray never
opens a dialog. That field is separate from `last_error` on purpose: `record_check` rewrites
`last_error` on every check, and the boot runs one, so a tray failure written there reached nobody.
**Every explicitly initiated action** (*Check now*, *Restart to update* from either door) clears the
previous action's outcome and records its own; the daily housekeeping check never touches it.

## The scheduler — inert until `scheduler: app`

`src/scheduler.rs` is a tick thread that can fire the local runner's slots plus a housekeeping
thread for debounced sync/backup and the tray's health colour.

**A slot is `coursework` → `ingest` → `judge` → `rank`**, each a child process of the sibling
engine exe (`slot_argv`). `ingest` is plan 4a's: a wizard that collects an LMS calendar URL and then
never fetches it is not a working install. It is **left out when the vault has no `ics_url`**, and
named as a skipped step in the run summary rather than silently dropped — `ics_state` distinguishes
the two "no" cases, so the Runs view says `ingest (skipped: no ics_url)` for a friend who has not
connected a feed and `ingest (skipped: config unreadable)` for a `config/ingest.yaml` that does not
parse. The app decides this, not the engine: `knowlu-engine ingest` exits 1 on an empty `ics_url`
(Python does, and the port keeps it), and a step exiting non-zero would put every friend without a
feed into permanent retry backoff and an amber tray, twice a day, forever. `judge` is plan 3a's,
skipped the same way for the same reason — see **Local judgment** below.

It stays **inert — never ticks, never runs a slot** — unless `config/runners.yaml`'s
`local` entry says `scheduler: app` **and** this device is the one named there (or none is named);
today's live vault carries neither key, so every install is inert by default. `Run now` and
`--run-slot-once` (a hidden CLI path used once to prove the scheduler live, Task 12) refuse to run
a slot under `scheduler: script` or on the wrong device rather than silently doing it anyway. A
hung child (a stalled vendor login, a wedged network read) is killed after 20 minutes rather than
left to hang the scheduler forever.

The child engine exe is resolved as **`KNOWLU_ENGINE_EXE`** if set and non-empty, else a
`knowlu-engine.exe` sitting beside the running `knowlu.exe`, else an error naming both places so a
friend's bug report says exactly what to check.

## Local judgment

The slot runs `knowlu-engine judge` between `ingest` and `rank`. It reads every task note flagged
`needs_enrichment: true` and answers effort, importance and course in three tiers:

1. **Heuristics** — the course map's uid pins and course codes (the same rule `ingest` applies), and
   a vendor's own effort estimate. Deterministic.
2. **Promoted rules** — the seam plan 3b fills. Empty today.
3. **The model** — **one `llama-cli.exe` process per judgment**, no port, no health check, no
   process kept alive between calls, answering a five-field JSON object under a GBNF grammar. A
   spike measured a loopback server 2.15× faster than one process per call — faster on every
   measure — but short of the plan's pre-committed 3× bar for the added lifecycle, so the simpler
   shape shipped: `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`.

**It runs with nothing installed.** "runtime not installed" and "model not installed" are normal
outcomes: the step is recorded at exit code 0 with a name that says which half is missing, whatever
tier 1 answered is still written, and the flag stays set so the work is still owed. Nothing is ever
downloaded automatically — Settings → *Local judgment* has a *Download* button and an *Install from
a file…* button, and the wizard offers a reminder and nothing more.

**Knowlu only runs llama.cpp builds it knows.** `inference::SUPPORTED_RUNTIMES` is a table of
upstream release tags, asset names and SHA-256s compiled into the app, and **both** install buttons
check the file against it before anything is written — so the path that works before the download
site exists is as verified as the one that comes later. An unrecognised digest is refused and the
message prints it, which is what to send us if you have a build we should add. Model files are not
pinned: a `.gguf` is data parsed by a runtime that table already vouched for, and any 1–4 B Q4 model
is meant to work.

Every write is `agent:knowlu.enrich` through `knowlu_engine::write` with `judged: true` and
`propose: true`, so a field you set in the console is never overwritten: the app's later opinion
arrives as a `kind: amend` card in the deck.

Where things live: `%LOCALAPPDATA%\knowlu\runtime\` and `…\models\` (install-wide — one download
serves every profile), `…\profiles\<id>\judgments\YYYY-MM-DD.jsonl` (per profile, ids and field
values only, never in the vault and never in the backup).

## Build and run

```powershell
# from app/ — its own crate, its own Cargo.lock, its own target/ (git-ignored as /app/target/)
$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"
cargo build --release
.\target\release\knowlu.exe --vault <path-to-a-vault>
```

Plain `cargo` — no `tauri-cli`. `tauri-build` in `build.rs` embeds `static/` at compile time via
`tauri::generate_context!()`, so there is no dev server and no bundler step. `tauri-cli` becomes
necessary only for the installer, which is `scripts\release.ps1`'s job — see below.
**A fresh checkout builds with plain `cargo`, and that is not an accident.** `bundle.externalBin`
makes `tauri-build` copy `binaries/knowlu-engine-<target triple>.exe` next to the built exe, and a
missing file is a hard `resource path ... doesn't exist` error raised before a single test compiles.
`app/binaries/` is git-ignored build output, so without help a fresh clone, a fresh worktree,
`cargo test` and the runner's `cargo build --release` would all fail on any checkout that has never
released. So `build.rs` drops a **zero-byte placeholder** there when the sidecar is absent (ruling
R-P4a-26). It is silent by design: `cargo:warning=` would add a line to every `cargo test`, and zero
new warnings is part of green here.

That placeholder is enough to compile and test, and it can never ship: plain `cargo` does not bundle
at all, and `scripts\release.ps1` refuses to bundle a sidecar smaller than 1 MiB. **The real engine
sits beside `knowlu.exe` only after `scripts\release.ps1`** — or `scripts\release.ps1 -StageOnly`,
which builds the engine and stages the sidecar and then stops, for a developer who wants a working
engine beside a plain `cargo build`.

## Releasing

`scripts\release.ps1` (repo root) is the **one** release command. **A hand-run `cargo tauri build` is not a supported release path** — it skips the clean-tree gate, the sidecar staging and the placeholder check, so it can bundle a stale or zero-byte engine. Releases go through `release.ps1` only. It refuses a working tree with
uncommitted changes so the installer always matches a commit, builds the engine at the root
(`cargo build --release -p knowlu-engine`), stages it as the sidecar, and runs `cargo tauri build`
from `app/`. The output is `target/release/bundle/nsis/Knowlu_<version>_x64-setup.exe` (one
`target/` for the workspace), copied into
`site/releases/` both under its versioned name and as the stable `Knowlu-setup.exe` the download
page links.

**The engine travels inside the installer as a Tauri sidecar** (`bundle.externalBin`). Tauri requires
the target triple in the staged file name — `app/binaries/knowlu-engine-x86_64-pc-windows-gnu.exe`,
which `release.ps1` copies there — and strips it at install time, so what lands next to
`knowlu.exe` is plain `knowlu-engine.exe`: exactly the sibling `scheduler::engine_exe()` resolves.
`app/binaries/` is git-ignored build output. The sidecar's stem is spelled in exactly two places,
`bundle.externalBin` and one variable at the top of `release.ps1`, and the script refuses to build
if they disagree.

**Signing happens inside the bundler, not after it.** `bundle.windows.signCommand` runs
`scripts\sign.ps1` once per binary, with the binary's path as the only argument, *before* the NSIS
package is written — so `knowlu.exe`, the sidecar and the installer are each signed as they are
produced. Signing only the finished installer would leave the two binaries inside it unsigned, which
is the half SmartScreen actually inspects. `sign.ps1` prints one loud `UNSIGNED:` line and **exits 0**
when there is no Trusted Signing profile, no `signtool.exe` or no Azure dlib, because a non-zero exit
there fails the whole `cargo tauri build` and a dev build still has to bundle.

**No secret reaches disk, and the laptop holds none (C0 Task 3).** Authenticode needs no secret —
Trusted Signing authenticates through the ambient Azure login (`azure/login` with OIDC on the runner)
and `sign.ps1` reads the profile from the path `KNOWLU_SIGNING_PROFILE` names (default
`%USERPROFILE%\.knowlu\trusted-signing.json`) and the dlib from `KNOWLU_SIGN_DLIB`. The Tauri
**updater** key exists only as the GitHub secret `TAURI_SIGNING_PRIVATE_KEY`: `release.yml` sets it in
the environment for one step, `release.ps1` checks it is there and lets `cargo tauri build` inherit
it, and nothing reads Credential Manager any more. It is never written to a file and never printed.

`bundle.createUpdaterArtifacts` and `plugins.updater` are **one** decision. `tauri-cli` reads
`plugins > updater` whenever the flag is anything but `false`, and fails the build outright with
*"failed to get updater configuration: plugins > updater doesn't exist"* if the plugin is absent — so
the two must be turned on in the same edit. Plan 4a Task 8's key-gated step turned **both on**, so
`release.ps1` no longer prints `UPDATER OFF` — and **every release now requires the updater signing
key**: with `createUpdaterArtifacts: true` the script refuses to build unless
`TAURI_SIGNING_PRIVATE_KEY` is set (*"TAURI_SIGNING_PRIVATE_KEY is not set - releases are built by CI
(.github/workflows/release.yml); run with -DryRun locally"*). `-DryRun` turns the updater artefacts
off for that one build through a temporary `--config` file, never by editing `tauri.conf.json`.

What it needs installed: `tauri-cli` (`cargo install tauri-cli`), mingw-w64 on `PATH` (see above),
`signtool.exe` from the Windows SDK and a Trusted Signing profile for a signed build, and network
access the first time — the bundler downloads NSIS, `nsis_tauri_utils.dll` and the WebView2
bootstrapper into `%LOCALAPPDATA%\tauri\` and caches them. **`release.ps1` never uploads**;
`.github/workflows/release.yml` deploys `site/` to Cloudflare Pages and creates the GitHub Release
on a `v*` tag. A human runs `release.ps1` only with `-DryRun`.
## Where the app's data lives, and how a launch resolves

```
%LOCALAPPDATA%\knowlu\
  profiles.json                      the registry: [{id, name, vault, created_at, last_opened_at}]
  profiles\<profile_id>\settings.json
  profiles\<profile_id>\seen.txt
  profiles\<profile_id>\logs\        slot-*.txt, quit-*.txt
  updates\                           the updater's downloads — one folder per install, not per profile
```

`state::app_data_root()` is still the one place that path is decided. **The root moved from
`quinn-ops` to `knowlu` here, and it is a move of OUR FILES, never of the directory** (F14 +
**R-P4a-28**): on the first launch of a build with profiles, `profiles::migrate_flat_layout` folds
the flat `settings.json` and `seen.txt` under `profiles\<id>\`, moves the app's own `slot-*` and
`quit-*` logs out of `logs\` one file at a time, and writes a one-entry registry. **Nothing else in
`%LOCALAPPDATA%\quinn-ops\` is touched** — `dual\`, `rehearsal\`, `scratch\`, `shots\` and the logs
the runner scripts write there belong to those scripts, and on the machine this was written they are
a live cutover week's evidence; the old root is removed only if the fold left it completely empty.
The fold reads both roots, is idempotent from either side, and whatever it could not do is printed
as a `Knowlu: …` line rather than swallowed. `id` is
`ids::derived_id("profile", <vault path>)`, so a vault opened by path and the same vault registered
by the wizard are one profile.

**Launch resolution** (`profiles::resolve_launch`, in `main.rs` before any window exists):

| Arguments | What happens |
|---|---|
| `--vault <path>` | that vault, exactly as before — this is what the Start-menu shortcut and autostart pass |
| none, zero profiles registered | the **wizard** |
| none, one profile | that profile, straight in |
| none, more than one | the **picker** |
| `--pick` | the picker, whatever the registry holds — how *Switch profile…* gets back to the chooser |
| a registry that exists and cannot be read | a `MessageBoxW` and exit 2; nothing is written, because rewriting an unreadable registry would lose every profile on the machine |
| a profile whose vault folder is gone | the **picker** (`profiles::fallback_shell`) — a moved, renamed or unplugged vault leaves the other profiles reachable and this one repairable, rather than ending the launch at an error box. With no profile registered it is still the `MessageBoxW` and exit 2 |

**One Knowlu at a time on a machine.** `tauri-plugin-single-instance` is keyed on the app
identifier, and that is kept — so *switching profiles is a relaunch*: `switch_profile` spawns
`knowlu.exe --pick --after-pid <our pid>` (and the picker's *Open* spawns `--vault <path>
--after-pid <pid>`), then takes the full quit path — stamp `quit_at`, push, back up, capped at 10 s.
The child waits, bounded at 10 s, for the parent process to go before registering; past the cap it
launches anyway and the plugin closes whichever loses.

## Why a separate crate

The rewrite spec forbids `tauri` in the engine crate. The two are separate crates in one Cargo
workspace (one `Cargo.lock`, one `target/`, one release profile, all at the repo root). When the
shell needs the engine it takes it as a path dependency — `knowlu-engine = { path = "../engine" }`
— never the other way round.

## What is in here

| File | What |
|---|---|
| `Cargo.toml` | `tauri = "2"` with `tray-icon`; `tauri-plugin-autostart`, `tauri-plugin-clipboard-manager`, `tauri-plugin-single-instance`, `tauri-plugin-window-state`; a Windows-only dependency on `windows` (`MessageBoxW`); `[[bin]] name = "knowlu"` (the release profile is the workspace root's) |
| `build.rs` | `tauri_build::build()`, the engine's `KNOWLU_BUILD_SHA` logic verbatim, and the **zero-byte sidecar placeholder** so a fresh checkout builds (see above) |
| `src/main.rs` | the flat-layout move, launch resolution, and the three entry points it produces (`run_console`, the wizard shell, the picker shell); `--after-pid` and `--pick`; `--run-slot-once`; a `MessageBoxW` titled "Knowlu" and exit 2 when a vault or a registry is unusable |
| `src/tray.rs` | tray icon (two runtime-generated 32×32 RGBA squares, no icon files), the seven menu items, `diagnostics_text` (version/build/last three runs/last errors — **never a note title**) |
| `src/profiles.rs` | the registry (`load`/`save`/`register`, atomic through a temp file), `profile_dir`/`updates_dir`, `id_for`/`default_name`, `migrate_flat_layout` (F14), `resolve_launch` → `Launch::{Open,Onboard,Pick,Broken}`, `wait_for_pid_gone` |
| `src/credentials.rs` | Windows-only: `target_for`/`write`/`exists`/`delete` over `CredWriteW`/`CredReadW`/`CredDeleteW`. Targets are `knowlu/<profile_id>/<source>`; the engine's read-only `src/wincred.rs` is unchanged and keeps reading whatever `credential_target` the vault names |
| `src/scaffold.rs` | the embedded vault skeleton — `VaultPlan`, `CAMPUSES`, `campus_yaml`/`ingest_yaml`/`runners_yaml`, `create_vault` (built and seeded in a staging folder, then **one rename** into place, so a crash never leaves a half-vault) and the two seed writes |
| `src/onboarding.rs` | the wizard's and picker's commands: `launch_state`, `pick_folder` (`rfd`), `adopt_vault`, `restore_vault`, `create_vault`, `dest_for`, `check_backup_dir`, `apply_profile_settings`, `store_credentials`, `retarget_credentials`, `finish_onboarding`, `open_profile`, and `relaunch`/`relaunch_args` — the one place this exe respawns itself |
| `src/updates.rs` | `update_offer` (**never mid-run**) and `stage_note` (its other half: what a run is holding back is not "up to date"), `record_check` (a failed check is quiet) and `note_error`/`clear_action_error` (the last ACTION's outcome, in its own field, which `record_check` never touches), `hold_for_install`/`InstallHold`, `begin_check`/`CheckGuard` (**single flight** — a second check is refused, not queued), `already_staged` (a version already on disk is not downloaded again), `stage_bytes` (temp-then-rename), `check_and_stage`/`install_staged` and their `_blocking` twins. The plugin verifies the minisign signature **inside `Update::download`**, so only verified bytes are ever staged |
| `assets/` | files compiled into the exe with `include_str!`: `scaffold/{planning.yaml,week_template.yaml,gitignore.txt}` and `campus/{none.yaml,university-of-alabama.yaml}`. Adding a campus is adding a file |
| `binaries/` | **git-ignored build output**: the staged engine sidecar, `knowlu-engine-<target triple>.exe`. `build.rs` drops a zero-byte placeholder here when it is absent; `scripts\release.ps1` puts the real engine here and refuses to bundle anything under 1 MiB |
| `src/state.rs` | `ConsoleState` — vault, settings, data dir, session id, and the live `sync`/`backup`/`last_write`/`pending_edits` fields the topline reads — plus `Settings` (profile id, backup dir, autostart, quit timestamp), `quit_flush`, and `app_data_root()`/`app_data_root_in()`: `%LOCALAPPDATA%\knowlu`, with `settings.json`, `seen.txt` and `logs\` under `profiles\<profile_id>\`. There is no `src/sync.rs` in this crate — the sync logic is `engine/src/sync.rs`'s; this file only holds the `Mutex<knowlu_engine::sync::SyncStatus>` the topline reads and the `run_sync` wrapper that calls into it under `vault_io`. |
| `src/scheduler.rs` | the tick/housekeeping threads, `RunGuard`, `engine_exe()` (`KNOWLU_ENGINE_EXE` resolution), `mode()`/`device_ok()` (`scheduler: app` gating), `ics_state`/`has_ics_url`, `slot_argv` (coursework → ingest → judge → rank), `run_slot`/`run_slot_inner`, and `lock` — every mutex in the crate is taken through it |
| `src/week.rs` | the commitment commands: `your_week` (`commitments::overview`) and `preview_window` (`surface::build_state_preview`) in-process; `commitment_proposals` (`commitments --json`) and `commitments_confirm` (`commitments --confirm <file>`, the file under the profile's `tmp\`, run under `vault_io`) on the sibling engine; `proposals_argv`/`confirm_argv` |
| `tauri.conf.json` | one 1280×860 window titled `Knowlu` (`minWidth: 820`), `productName: "Knowlu"`, `identifier: "com.knowlu.desktop"` (permanent, see above), `frontendDist: static`; and the bundle — `targets: ["nsis"]`, `externalBin: ["binaries/knowlu-engine"]`, `webviewInstallMode: embedBootstrapper`, `windows.nsis.installMode: "currentUser"` (**load-bearing**: `updates::install_staged`'s reasoning about the staged bundle depends on the exe living in the user's own profile), `windows.signCommand` → `scripts\sign.ps1`, `createUpdaterArtifacts: true` (one decision with `plugins.updater`, see *Releasing*); and `plugins.updater` — the release endpoint and the minisign **public** key (id `C2EC981122E1D2DF`). The private half is never in this repo: `release.ps1` reads it from Credential Manager for one build |
| `capabilities/default.json` | `core:default`, `window-state:default`, `autostart:default`, `clipboard-manager:allow-write-text` — and **no updater permission** (R-P4a-30): the app drives the updater from Rust, where no capability applies, and `updater:default` would hand the page `allow-install` / `allow-download-and-install`, letting a page script bypass the mid-run gate and the install hold. The folder picker is `rfd` called from an app command, not a plugin, so it needs no permission here either |
| `static/index.html`, `console.js`, `console.css` | the console page — writes, the deck and flag popover, sync/backup/scheduler/Runs/interaction events — plus plan 4a's three non-view panels in the same document and the same IIFE: the **picker** (`renderPicker`), the **wizard** (`startWizard`, seven panels) and the **settings overlay** (`openSettings`, also reachable as `window.KNOWLU_OPEN_SETTINGS` for the tray). The console also holds the **Schedule** view, the **confirm screen** (`#week-setup`) and the `commitment-ask` **ask form** in the Decisions view. `window.KNOWLU_SHOTS` is the one seam the headless scripts use |

## Never point this at the live vault

`scripts/scratch-vault.ps1` (repo root) copies a vault's data folders (`tasks`, `approvals`,
`archive`, `courses`, `issues`, `info`, `state`, `config`, `profile`) from the main checkout into
`%LOCALAPPDATA%\knowlu\scratch\<stamp>`, turns that copy into its own local-only git repo (no
remote — a dev convenience for diffing what one run changed, unrelated to the account sync C3′ added;
nothing in the product itself touches git any more), and prints the launch line. `-Source` is
required — this repository holds no vault of its own:

```powershell
scripts\scratch-vault.ps1 -Source <path to a vault>
target\release\knowlu.exe --vault "<the printed scratch path>"
```

**Develop and demo against a scratch copy, never the live vault, until Task 17's go-live
checklist (`docs/procedures/knowlu-go-live.md`) is run after cutover's G2** (Knowlu spec §2) — the
console writes notes, syncs them to the account, and runs scheduled slots inside the vault it's
pointed at.

**Every `--vault` launch registers a profile, and nothing removes one yet.** So a few scratch
sessions leave a few profiles in `profiles.json`, and from then on a plain launch (no `--vault`)
shows the **picker** rather than opening one straight away — that is the launch rules working, not
a fault. Pass `--vault` to go straight to the one you want. A *Forget this profile* control is a
plan-4b item; until it exists, editing `%LOCALAPPDATA%\knowlu\profiles.json` by hand is the way to
prune the list (Knowlu is not running when you do it).

## Known wrinkles, none blocking

- **`ld: .rsrc merge failure: multiple non-default manifests`** at link — **still open.** One
  cause was removed in Task 7 (`common-controls-v6` is now off — `muda`'s `TaskDialogIndirect`
  import needs it, but a GNU binary linking that symbol by name fails to load with
  `STATUS_ENTRYPOINT_NOT_FOUND` on any real Windows `comctl32.dll`, which only ever resolves it
  through an isolation-aware stub MSVC compiles in, never GNU), but the merge failure itself
  persists — Tauri's remaining manifest and mingw's default still collide. Windows uses one;
  controls render and DPI is correct anyway because `tao` calls
  `SetProcessDpiAwarenessContext` at startup. Fix before shipping — `tauri-build`'s
  `WindowsAttributes::app_manifest`, or stop mingw embedding its default — and verify with a
  manifest count of 1.
- **The CSP names the IPC origin** (`connect-src ipc: http://ipc.localhost`): Tauri's `invoke`
  fetches `http://ipc.localhost/<cmd>` on Windows, a different origin from `'self'`
  (`http://tauri.localhost`), so without it the first `invoke` is CSP-blocked; this is Tauri's own
  documented minimum CSP, not network egress — the static-asset tests still forbid `http://` in
  `app/static/`.
- **`settings.json` is still a file, and it is still all-or-nothing.** All four keys
  (`profile_id`, `backup_dir`, `autostart`, `quit_at`) must be present or the file does not parse
  and defaults are used, with the reason printed and shown in `Copy diagnostics`. What changed in
  plan 4a is that nobody has to edit it by hand: the **settings panel** (the topline gear, or
  *Settings* in the tray) writes it through `set_settings`, and it lives per profile under
  `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\`.
  **An existing `%APPDATA%\app.knowlu.desktop\settings.json` is not migrated** — the only one that
  ever existed is Quinn's own; set the backup folder again from the panel.
- **Autostart is on by default** (`autostart: true` when no settings file exists), so a first launch
  registers Knowlu to start with Windows. The wizard's *Slots and campus* panel offers the choice,
  and the settings panel's *Start with Windows* row changes it afterwards — the toggle re-registers
  the OS entry with this profile's `--vault`.
- **Three accepted costs of the commitment screens** (phase 2, review finding 9).
  `commitment_proposals` has no timeout of its own, so a hung calendar fetch leaves the confirm
  screen on "Reading your calendar…" while Finish still works. A repaint on a new `revision` closes
  an open kind `<select>` on the *Schedule* view, and it drops half-entered day toggles and times
  in an ask form.
