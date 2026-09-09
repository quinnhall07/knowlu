# Knowlu — Friends, the shell half — Implementation Plan (Knowlu plan 4a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: EXECUTED 2026-09-06 on branch worktree-knowlu-plan-4a (Tasks 1–10 and 11; Task 8's key-gated updater step is owed until Quinn's public key arrives; the SDD ledger is preserved at docs/superpowers/reports/2026-09-06-knowlu-plan-4a-sdd-ledger.md).** Written 2026-09-05. Nothing here was gated on the cutover clock, on G2, or on plan 2 Part B, and the one ordering rule inside the plan held: **Task 9 read Task 1's spike outcome** and took the branch that outcome named.

**Goal:** A friend can install Knowlu from a signed installer, be walked from nothing to a working vault by a first-run wizard, keep their coursework logins in Windows Credential Manager, run their own slots from the tray, and get updates — with profiles, a settings panel, a release script and a download page behind it.

**Architecture:** Almost all of it lives in `app/` plus one PowerShell release script and a static `site/` folder. The app grows four modules — `profiles.rs` (registry, per-profile app data, the flat-layout move, launch resolution), `credentials.rs` (the write half of Credential Manager the engine already reads), `scaffold.rs` (an embedded vault skeleton materialised atomically), `onboarding.rs` (the wizard's commands) — and the page grows three panels (picker, wizard, settings) inside the same document, same IIFE, no bundler. The engine crate (`src/`) is touched **once**: Task 5 gives `engine/ingest.py:main` its clap home as `quinn-ops ingest`, without which the LMS feed the wizard collects would never be fetched on a friend's machine. Everything else the wizard writes goes through `quinn_ops::write`, the scaffolded vault is read by the engine exactly as it reads Quinn's, and the engine ships beside `knowlu.exe` as a Tauri sidecar.

**Tech Stack:** Rust 1.98 `stable-x86_64-pc-windows-gnu`; Tauri 2 (`tauri`, `tray-icon`, `tauri-plugin-autostart`, `tauri-plugin-clipboard-manager`, `tauri-plugin-single-instance`, `tauri-plugin-window-state`, **new:** `tauri-plugin-dialog`, `tauri-plugin-updater`); `windows 0.62` with `Win32_Security_Credentials` and `Win32_System_Threading`; `tauri-cli` (`cargo tauri build`) for the NSIS bundle; plain HTML/CSS/JS in `app/static/`; headless Playwright in the git-ignored `.wv` venv for page checks; PowerShell 5.1 for `scripts/release.ps1`.

**Spec:** `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md` (§1 scope, §2 profiles, §3 onboarding, §4 settings, §5 credentials, §6 installer/updater/release, §7 the site, §8 testing, §9 the ten tasks, §10 decisions 1–11, §11 Quinn's open items, §12 rulings preserved) — argued from `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` (§4 storage, §6 distribution, §7 naming, §9 plan 4, §10 decisions 7, 9, 14–16) and `docs/superpowers/notes/2026-09-01-market-pricing-and-distribution.md` §4.4 (the ICS feed is connector #1; no Canvas token).

## Global Constraints

Every task's requirements implicitly include these. Values are copied verbatim from the spec and from CLAUDE.md.

- **Scratch copies and temp folders only.** No live vault, ever, in this plan: not a test, not a headless check, not a manual look. `scripts\scratch-vault.ps1` makes the copy. **No network in any test, and no live fetch in onboarding** — "Nothing is fetched until the first slot fires" (spec §3.1); the ICS URL is validated by shape in the wizard and never requested there. `cli::run_with(…, Fetchers)` keeps `rank` off the network in Task 4's test, and `ingest::run_lines(…, fetch)` does the same for Task 5's.
- **The wizard's ICS URL is fetched by the slot's `ingest` step and nowhere else** (Task 5): the wizard writes it into `config/ingest.yaml`, and `scheduler::slot_argv` runs `coursework → ingest → rank` — with `ingest` included only when that key is non-empty.
- **Credentials.** "A secret is never logged, never in a run record, a backup, a fixture, a test name or a plan; the page clears its fields the moment the write returns; tests use a throwaway target under `knowlu/test/…` and delete it" (spec §5). Every test secret and every test target is generated at runtime (`quinn_ops::ids::new_id`), never a literal. **Quinn's own `quinn-ops/zybooks` and `quinn-ops/vhl` entries are untouched by this plan.**
- **Credential targets are `knowlu/<profile_id>/<source>`** (decision 6); the app writes them, the engine keeps its read-only `src/wincred.rs` and keeps reading whatever `credential_target` the vault names.
- **Every app write goes through `quinn_ops::write` with `console_ctx()`** (`actor: "quinn"`, `via: "dashboard"`). **The one exception is decision 5:** the onboarding seed record, **`WriteContext::new("system:migration", "cli")`** — the exact shape the S1 migration left in Quinn's journal — creating `archive/_migrated.md`, written **exactly once per new vault** and never for an adopted or restored one. The journal's `via` vocabulary does not grow (R-P4a-6).
- **The read model never writes** (`build_state_never_writes` stays green); `commands.rs` computes nothing; every mutating command returns `{ok, error, state}` with the freshly rebuilt state.
- **The scheduler stays inert unless `config/runners.yaml`'s `local` entry says `scheduler: app`.** A vault born in the wizard carries `scheduler: app` **and** `device: <this machine>` from birth (decision 4) — that is not the plan-2 rule's subject, which is Quinn's live vault. `device:` keeps gating the tick, so a second install against the same vault still stands down.
- **Exactly one `src/` change: Task 5 (the ingest command).** Its gate is root `cargo test` at 0 warnings, `tests/oracle.rs` and `tests/surface_oracle.rs` green, **and** `scripts\diff-engines.ps1` on all three fixtures plus `scripts\diff-engines-notes.ps1` exiting 0 — those scripts do not cover `ingest`; they are what proves `rank` and `write` did not move under the edit. No other task touches `src/`.
- **Frozen references:** the eight Python-written references are never regenerated; the three `surface-today-*.json` are not touched by this plan (no payload key changes).
- **Page rules (console spec §7, unchanged):** no `http://` / `https://` anywhere under `app/static/` — the updater endpoint lives in `app/tauri.conf.json`, and an ICS URL is matched with an escaped regex (`/^https:\/\//`), never a literal; no `import ` / `require(` in `console.js`; every `[data-id]` element carries `data-kind` and the static test's `data-id` == `data-kind` count equality must still hold — **picker, wizard and settings markup use `data-profile`, `data-panel`, `data-set`, never `data-id`**; `mark_seen` from exactly one place; **this plan adds no `uievents::ACTIONS` action.**
- **No single-user assumption.** Nothing in the registry, the layout, the scaffold or a test names a path, a machine or a person. `device:` in a scaffolded vault is `quinn_ops::journal::device_name()` read at wizard time.
- **No telemetry, no toggle** (decision 10). The wizard's privacy paragraph is exact and is the same text as the site's: *"Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates."*
- **Desktop safety:** never synthetic keyboard or mouse input; screenshots by `PrintWindow` on a window handle from a DPI-aware process only, never a full-screen grab; headless Playwright for page checks.
- **Line endings are per file, never per directory.** Check the file you are about to edit with `od -c <f> | head -2` or `tr -cd '\r' < <f> | wc -c` — **never `grep -c $'\r'`** (it counts every line on this machine) and **never `Get-Content`** (it hides CR). Docs under `docs/` are CRLF; `app/*.rs`, `app/static/*` and `app/tauri.conf.json` may be LF or CRLF — preserve whatever each file already has. `git diff --stat` must never show a whole-file flip.
- **Before any claim of green:** `cd app; cargo test` at **zero new warnings** (the `.rsrc merge failure` linker line is pre-existing), and root `cargo test` at **0 warnings** with `tests/oracle.rs` and `tests/surface_oracle.rs` green.
- **Commits:** `git add` specific paths, never `-A`; message via `-F <file>`; both trailers, `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4`. Obsidian Git still sweeps the main checkout every ~5 minutes, so commit scaffolding before long builds.
- **Build PATH (Bash):** `export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"`. Long cargo builds run as background calls with a bounded wait. **Absolute paths containing `GitHub` trip the Bash worktree guard — use relative paths or the PowerShell tool.**
- **PowerShell 5.1 only** for `scripts/release.ps1`: no `&&`, no `||`, no ternary, no `??`. `A; if ($?) { B }` is the chain. Native-exe arguments with spaces and embedded quotes split — spell an inner quote as `"""`.
- **Never here:** a hosted account, a cloud API, a write to the live vault, a change to `engine/` or the cloud routine, any name on the never-rename list (journal actors and `via` values, run-record runner names, ledger lines, the eight references, `QUINN_OPS_DEVICE`, the repository, the folder, `engine/`).

---

## Fidelity ledger — decisions and rulings this plan must not lose

| # | Decision / ruling | Source | Carried by |
|---|---|---|---|
| D1 | Plan 4 splits; the shell half runs now, before plan 3 | spec §10.1 | This plan's existence; Task 11 records the amended §9 order in the Knowlu spec |
| D2 | A profile is a vault plus per-profile app data under `%LOCALAPPDATA%\knowlu\profiles\<id>\`; the root moves here (F14 honoured); plan 2 Task 12's move becomes a check | spec §2, §10.2 | Task 2 (`profiles::migrate_flat_layout`, `state::app_data_root_in`); Task 11 (the note in plan 2's Task 12) |
| D3 | The wizard writes nothing until Finish, except credentials at the moment they are entered | spec §3, §10.3 | Task 4 (`scaffold::create_vault` called only by `create_vault`), Task 6 (`store_credentials` on leaving panel 5; `create_vault` on Finish) |
| D4 | A vault born in the wizard gets `scheduler: app` and `device:` from birth | spec §3.1, §10.4 | Task 4 (`runners.yaml` in the scaffold) + its test |
| D5 | The unmigrated guard is satisfied by one `system:migration` create record of an archived seed note — **`via: "cli"`**, the S1 migration's own shape, so the journal's `via` vocabulary does not grow (R-P4a-6; Task 11 amends the spec's wording) | spec §3.1, §10.5 | Task 4 (`scaffold::seed_writes`) + `one_migration_record_and_the_rest_are_dashboard_writes` |
| D6 | Credential targets are `knowlu/<profile_id>/<source>`; the engine keeps reading by name; Quinn's targets untouched | spec §5, §10.6 | Task 3 (`credentials::target_for`), Task 4 (`credential_target:` lines), Task 6 (`store_credentials`) |
| D7 | The engine ships as a Tauri sidecar (`externalBin`), landing beside `knowlu.exe` where `scheduler::engine_exe()` already looks | spec §6, §10.7 | Task 1 (spike), Task 9 (`bundle.externalBin`, `scripts/release.ps1`) |
| D8 | Identifier `com.knowlu.desktop` is set by this plan | spec §6, §10.8 | Task 9 step 2; Task 11 turns plan 2 Task 12's identifier step into a check |
| D9 | The settings panel holds five rows; slots/timezone/sources stay the wizard's, re-entered via *Reconfigure…* in 4b | spec §4, §10.9 | Task 7 (exactly five rows) |
| D10 | No telemetry, no toggle | spec §10.10 | Task 6 (no toggle in the wizard), Task 10 (privacy page says so) |
| D11 | The site is plain HTML/CSS, no scripts; deployed by hand until the Cloudflare account exists | spec §7, §10.11 | Task 10 (`tests/site.rs` forbids `<script`) |
| R1 | No single-user assumption (CLAUDE.md rule 2) | spec §12 | Tasks 2, 4 (`device_name()`, derived ids, temp roots in tests) |
| R2 | Every console write through `quinn_ops::write` with `console_ctx()`; the one exception is D5 | spec §12 | Tasks 4, 6 + `one_migration_record_and_the_rest_are_dashboard_writes` |
| R3 | The read model never writes; the app never builds and never runs the passes itself | spec §12 | No task calls `cli::run` outside a test; Task 4's test uses `run_with` with stub fetchers |
| R4 | The scheduler is inert unless `scheduler: app`; D4 applies the same key to fresh vaults | spec §12 | Task 4's scaffold; `scheduler.rs` untouched apart from `log_dir` |
| R5 | Credentials never in the repo, a log, a backup rule, a fixture or a plan | spec §5, §12 | Task 3's test target and secret are minted at runtime and deleted |
| R6 | Desktop safety — no synthetic input; `PrintWindow` from a DPI-aware process only; headless Playwright for page checks | spec §12 | Tasks 5, 6, 10 |
| R7 | The never-rename list; the eight Python references and the three surface references untouched | spec §12 | No task touches `tests/fixtures/*`; Task 9 renames nothing |
| R8 | The live vault is written only by Knowlu launched from the shortcut after the go-live checklist; the console stays on scratch copies until G2; nothing here moves G2, `$mode`, or the routine | spec §12 | Global constraints; Task 11's HANDOFF line says so again |
| R9 | Never update mid-run; one bundle, one version; a failed signature is refused and reported | Knowlu spec §6, decision 16 | Task 8 (`update_offer`, `install_update`) |
| R10 | Updater private key in Credential Manager (`knowlu/updater-key`); no signing secret in the repo | Knowlu spec decision 15 | Task 8 step 6; Task 9's signing step |
| R11 | **One Knowlu per machine**, `tauri-plugin-single-instance` kept exactly as it is; profile switching is a relaunch that waits for the old process to go (`--after-pid`), so the plugin's lock is free when the child registers. The spec §2 sentence about two profiles in two tray icons is withdrawn | ruling R-P4a-1 | Task 2 (`wait_for_pid_gone`, `open_profile`); Task 11 (the spec amendment) |
| R12 | **The LMS feed is actually fetched**: `quinn-ops ingest` ports `engine/ingest.py:main`, and the slot runs coursework → ingest → rank, with `ingest` left out when the vault has no `ics_url` (its engine exit code would otherwise read as a failed slot). Nothing changes on Quinn's live vault until plan 2 Task 9 flips `scheduler: app`; the overlap with the cloud routine is safe through `state/ingest-seen.md` | rulings R-P4a-7, R-P4a-9 | Task 5; Task 11's HANDOFF line |
| R13 | **Signing happens before the bundler packs**: `bundle.windows.signCommand` points at `scripts/sign.ps1`, which wraps `signtool` + the Azure Trusted Signing dlib and exits 0 with one loud `UNSIGNED:` line when no profile is configured, so an unsigned dev build still bundles. `release.ps1` never signs after the fact | ruling R-P4a-16 | Task 9 |
| R14 | **No commit registers `tauri-plugin-updater` without its config**: `plugins.updater.pubkey` is a required field (`tauri-plugin-updater` 2.11.0, `config.rs:137`), so registration without it is a startup panic. The plugin, the config block and the real check/install land in one gated step | ruling R-P4a-13 | Task 8 step 9; Tasks 9 and 10 (neither asserts the endpoint) |
| R15 | **The updater key never touches disk**: `release.ps1` reads `knowlu/updater-key` from Credential Manager into `TAURI_SIGNING_PRIVATE_KEY` for the length of one `cargo tauri build` and clears it in a `finally` | ruling R-P4a-14 | Task 9 |

---

## File structure

**Engine crate (`src/`) — the one change**
- `ingest.rs` gains `run_lines`/`run_with`/`run` (the port of `engine/ingest.py:main`, with a fetch seam) and five tests; `main.rs` gains the `Ingest` clap arm. Nothing else under `src/` is touched by any task.

**New in `app/src/`**
- `profiles.rs` — `Profile`, the registry (`load`/`save`/`register`), `profile_dir`, `updates_dir`, `migrate_flat_layout` (the one-time `quinn-ops` → `knowlu` move), `resolve_launch`, `wait_for_pid_gone`. One responsibility: *which vault, and where does its app data live*.
- `credentials.rs` — `target_for`, `write`, `exists`, `delete`. The write half of Credential Manager; nothing else in the app touches `CredWriteW`.
- `scaffold.rs` — the embedded vault skeleton, the campus presets, `create_vault` (atomic), `seed_writes` (the two engine calls). One responsibility: *a vault that did not exist a second ago ranks cleanly*.
- `onboarding.rs` — `Onboarding` state and the wizard/picker `#[tauri::command]`s. Marshal, call, envelope — like `commands.rs`, it computes nothing.

**Modified in `app/src/`** — `lib.rs` (four `pub mod` lines), `main.rs` (launch resolution, two builders, the new plugins), `state.rs` (`app_data_root_in`, `ConsoleState.data_dir`), `scheduler.rs` (`log_dir` takes the state), `tray.rs` (the quit log path, a `Settings` item, an update item), `commands.rs` (`set_profile_name`, `check_for_updates`, `install_update`).

**Page (`app/static/`)** — `index.html` (`#picker`, `#wizard`, `#settings` panels), `console.js` (`bootConsole`, `renderPicker`, the wizard's `WIZ`/`PANELS`, `openSettings`, `renderUpdateOffer`), `console.css` (`.picker`, `.wiz*`, `.set*` rules on the existing tokens).

**App config** — `app/Cargo.toml` (two plugins in, none out; two `windows` features), `app/tauri.conf.json` (identifier, bundle, sidecar, updater), `app/capabilities/default.json` (`dialog:default`, `updater:default`), `app/assets/scaffold/*` and `app/assets/campus/*` (embedded via `include_str!`), `app/binaries/` (git-ignored, the sidecar drop).

**Tests** — `app/tests/profiles.rs`, `app/tests/credentials.rs`, `app/tests/scaffold.rs`, `app/tests/updates.rs` (new); `app/tests/static_assets.rs`, `app/tests/commands.rs`, `app/tests/scheduler.rs` (extended); `src/ingest.rs`'s own `mod tests` (five added); `tests/site.rs` (new, root crate).

**Scripts and site** — `scripts/release.ps1` (new), `scripts/console-shots.py` (two more views), `site/index.html`, `site/privacy.html`, `site/releases/.gitkeep`.

**Docs** — `app/README.md`, `docs/surface/anatomy.md`, `docs/HANDOFF.md`, `CLAUDE.md`, the Knowlu spec's §9 note, this plan's status line (all Task 11).

---

### Task 1: Spike — `tauri-cli` on the GNU toolchain, `bundle.active: true`, and the sidecar

**Throwaway allowed.** Nothing but the report is committed. The point is a single fact: *does `cargo tauri build` produce an NSIS installer with the WebView2 bootstrapper and the engine sidecar, on `stable-x86_64-pc-windows-gnu`?* Task 9 branches on the answer.

**Files:**
- Create: `docs/superpowers/reports/2026-09-05-tauri-bundle-spike.md`
- Touch and revert: `app/tauri.conf.json` (bundle keys), `app/binaries/` (the sidecar copy)

**Interfaces:**
- Consumes: `scheduler::engine_exe()`'s resolution order (`KNOWLU_ENGINE_EXE`, else a sibling `quinn-ops.exe`).
- Produces: **the spike report's `Outcome:` line — one of A/B/C/D below.** Task 9 step 1 reads it.

- [ ] **Step 1: Install the CLI**

Background call, bounded wait (~5–10 min on this laptop):

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
cargo install tauri-cli --locked --version "^2"
cargo tauri --version
```

Expected: a version line (`tauri-cli 2.x.y`). Record it verbatim in the report. If `cargo install` fails to link, that is Outcome D and the report says the exact linker line.

- [ ] **Step 2: Build the engine and stage it as the sidecar**

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
cargo build --release
mkdir -p app/binaries
cp target/release/quinn-ops.exe app/binaries/quinn-ops-x86_64-pc-windows-gnu.exe
ls -l app/binaries/
```

Tauri requires the target-triple suffix on an `externalBin` file and strips it on install, so the installed name is `quinn-ops.exe` — exactly what `engine_exe()`'s sibling branch looks for. Record the byte size.

- [ ] **Step 3: Turn the bundle on, temporarily**

In `app/tauri.conf.json`, replace the `"bundle"` object with:

```json
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"],
    "externalBin": ["binaries/quinn-ops"],
    "windows": { "webviewInstallMode": { "type": "embedBootstrapper" } }
  }
```

- [ ] **Step 4: Build the bundle**

Background call, bounded wait (~10–20 min cold):

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
cd app && cargo tauri build 2>&1 | tail -40
```

Expected on success: a line naming `app/target/release/bundle/nsis/Knowlu_0.1.0_x64-setup.exe`.

- [ ] **Step 5: Record the outcome against this table**

| Outcome | What happened | What Task 9 does |
|---|---|---|
| **A** | The NSIS installer built; its size is recorded; `7z l` (or `Expand-Archive`-equivalent listing) shows `knowlu.exe`, `quinn-ops.exe` and a WebView2 bootstrapper entry | Task 9 keeps `cargo tauri build` as the one build command; `scripts/release.ps1` calls it |
| **B** | The bundler ran but could not fetch/execute NSIS (download blocked, `makensis` missing) | Task 9 sets `NSIS_PATH`/vendors NSIS per the CLI's own message, keeps `cargo tauri build`, and `release.ps1` fails loudly with that message if NSIS is absent |
| **C** | `cargo tauri build` refuses the GNU target, or the bundler is MSVC-only | Task 9 writes `scripts/nsis/knowlu.nsi` and `release.ps1` calls `makensis` over `knowlu.exe` + `quinn-ops.exe` + the bootstrapper; the app config keeps `bundle.active: false` and `externalBin` is copied by the script instead |
| **D** | `tauri-cli` will not install or the built exe does not launch (the known `.rsrc` multiple-manifest wrinkle) | Task 9's first step is the manifest fix (`tauri-build`'s `WindowsAttributes::app_manifest`, verified by a manifest count of 1) and the table is re-run |

- [ ] **Step 6: Sanity-check the installed layout without installing**

Do **not** run the installer on this desktop. Extract it instead and list what is inside:

```powershell
$exe = "app\target\release\bundle\nsis\Knowlu_0.1.0_x64-setup.exe"
if (Test-Path $exe) { (Get-Item $exe).Length; & 7z l $exe | Select-String -Pattern "knowlu.exe|quinn-ops.exe|MicrosoftEdgeWebview2Setup" }
```

Expected: both binaries listed and a WebView2 bootstrapper entry. If `7z` is absent, record that and note the size only.

- [ ] **Step 7: Revert the throwaway config**

```bash
git checkout -- app/tauri.conf.json
git status --short app/
```

Expected: `app/tauri.conf.json` unmodified; `app/binaries/` untracked (Task 9 git-ignores it).

- [ ] **Step 8: Write the report and commit only it**

`docs/superpowers/reports/2026-09-05-tauri-bundle-spike.md` (CRLF — it is under `docs/`), with, in this order: the `tauri-cli` version; the engine exe size; the exact bundle config used; the last 40 lines of the build output; **a bold `Outcome: A|B|C|D` line**; the installer size and content listing (or why not); and the one sentence Task 9 needs — what to do next.

```bash
git add docs/superpowers/reports/2026-09-05-tauri-bundle-spike.md
git commit -F <(printf '%s\n' "spike: cargo tauri build on the GNU toolchain — NSIS, the bootstrapper and the engine sidecar (Knowlu plan 4a, Task 1)" "" "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" "Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4")
```

(If `<(…)` process substitution is unavailable, write the message to a scratch file and pass `-F` that path — never `-m` with a multi-line string.)

---

### Task 2: Profiles — the registry, the per-profile layout, the move, launch resolution, the picker

**Files:**
- Create: `app/src/profiles.rs`, `app/src/onboarding.rs`, `app/tests/profiles.rs`
- Modify: `app/src/lib.rs`, `app/src/main.rs`, `app/src/state.rs`, `app/src/scheduler.rs` (`log_dir`), `app/src/tray.rs` (the quit-log path), `app/Cargo.toml`, `app/capabilities/default.json`, `app/static/index.html`, `app/static/console.js`, `app/static/console.css`, `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `quinn_ops::ids::derived_id("profile", &vault.to_string_lossy())` (the same call `Settings::load` already makes, so an opened vault and a registered one are the same profile); `quinn_ops::ledger::dumps_value`; `quinn_ops::journal::now_ts(None)`; `ConsoleState::open(vault, app_data_dir)`.
- Produces:
  - `profiles::Profile { id: String, name: String, vault: PathBuf, created_at: String, last_opened_at: Option<String> }`
  - `profiles::{registry_path, profile_dir, updates_dir}(root: &Path[, id: &str]) -> PathBuf`
  - `profiles::load(root: &Path) -> Vec<Profile>`; `profiles::save(root: &Path, &[Profile]) -> Result<(), String>`
  - `profiles::id_for(vault: &Path) -> String`; `profiles::default_name(vault: &Path) -> String`
  - `profiles::register(root: &Path, name: &str, vault: &Path) -> Result<Profile, String>`
  - `profiles::migrate_flat_layout(base: &Path, vault_hint: Option<&Path>) -> Option<String>`
  - `profiles::Launch { Onboard, Open(Profile), Pick(Vec<Profile>) }`; `profiles::resolve_launch(root: &Path, vault_arg: Option<&Path>) -> Launch`
  - `profiles::wait_for_pid_gone(pid: u32, cap: std::time::Duration) -> bool`
  - two new argv flags in `main`: `--after-pid <pid>` (wait, bounded, before registering with the single-instance plugin) and `--pick` (force the picker; Task 7's *Switch profile…* passes it)
  - `state::app_data_root_in(base: &Path) -> PathBuf`; `ConsoleState.data_dir: PathBuf`
  - `onboarding::Onboarding { root: PathBuf, mode: &'static str, profiles: Mutex<Vec<Profile>> }`
  - commands `launch_state`, `pick_folder`, `adopt_vault`, `open_profile` (Task 6 adds `create_vault`, `restore_vault`, `store_credentials`, `finish_onboarding` to the same module)
  - page: `bootConsole()`, `renderPicker(l)`

> **Scope note (recorded for the reviewer):** the spec's §9 lists `pick_folder` and `adopt_vault` under *its own* item 5, the wizard — this plan's Task 6. They land here instead, because the picker is useless without them — *Use an existing vault* is how Quinn's own vault becomes a profile, and it is the only way a zero-profile launch can reach a console before Task 6 exists. Task 6 builds the remaining six panels on top. No behaviour is added or dropped; only the task boundary moves.

- [ ] **Step 1: Write the failing tests** (`app/tests/profiles.rs`)

```rust
use quinn_ops_console::profiles::{
    default_name, id_for, load, migrate_flat_layout, profile_dir, register, resolve_launch, save,
    wait_for_pid_gone, Launch, Profile,
};
use std::path::{Path, PathBuf};

fn temp_root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-profiles-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_registry_round_trips_and_a_vault_registered_twice_stays_one_profile() {
    let root = temp_root("registry");
    let vault = root.join("vaults").join("Ada");
    std::fs::create_dir_all(&vault).unwrap();
    let p = register(&root, &default_name(&vault), &vault).unwrap();
    assert_eq!(p.id, id_for(&vault));
    assert_eq!(p.name, "Ada", "the folder name is the default label");
    assert!(profile_dir(&root, &p.id).is_dir(), "the per-profile folder exists");
    let again = register(&root, "Renamed", &vault).unwrap();
    assert_eq!(again.id, p.id);
    let all = load(&root);
    assert_eq!(all.len(), 1, "registering the same vault twice is one profile");
    assert_eq!(all[0].name, "Renamed");
    assert_eq!(all[0].created_at, p.created_at, "created_at is never rewritten");
    // Written through ledger::dumps_value: Python's separators, not serde's minimal ones.
    let text = std::fs::read_to_string(root.join("profiles.json")).unwrap();
    assert!(text.contains("\", \"") || text.contains("\": \""), "dumps_value separators: {text}");
    // A round trip through save(), and a missing registry that is empty rather than an error.
    let manual = Profile { id: "profile_0123456789".into(), name: "X".into(), vault: PathBuf::from("C:\\x"), created_at: "2026-09-05T00:00:00Z".into(), last_opened_at: None };
    save(&root, std::slice::from_ref(&manual)).unwrap();
    assert_eq!(load(&root), vec![manual]);
    assert!(load(Path::new("C:\\nowhere-at-all")).is_empty());
}

#[test]
fn the_flat_layout_moves_whole_and_nothing_is_lost() {
    let base = temp_root("f14");
    let old = base.join("quinn-ops");
    std::fs::create_dir_all(old.join("logs")).unwrap();
    let vault = base.join("v");
    std::fs::create_dir_all(&vault).unwrap();
    let id = id_for(&vault);
    std::fs::write(old.join("settings.json"), format!("{{\"profile_id\": \"{id}\", \"backup_dir\": null, \"autostart\": true, \"quit_at\": null}}")).unwrap();
    std::fs::write(old.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(old.join("logs").join("slot-x.txt"), "kept").unwrap();

    let moved = migrate_flat_layout(&base, Some(&vault)).expect("the move happened");
    assert_eq!(moved, id);
    let new = base.join("knowlu");
    assert!(!old.exists(), "the old root is gone, not copied");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert!(profile_dir(&new, &id).join("settings.json").is_file());
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("logs").join("slot-x.txt")).unwrap(), "kept");
    assert_eq!(load(&new).len(), 1, "the registry gained the one profile");
    assert!(migrate_flat_layout(&base, Some(&vault)).is_none(), "a second launch moves nothing");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z", "…and nothing was disturbed by the second run");
}

/// S5: the two halves are separately survivable. A crash between the rename and the fold — or an
/// `app_data_root()` call that created `knowlu` first — must not strand the flat files at the new
/// root, and a run with nothing to derive an id from must still move them.
#[test]
fn a_half_moved_layout_is_folded_on_the_next_launch_and_never_stranded() {
    let base = temp_root("halfmoved");
    let new = base.join("knowlu");
    std::fs::create_dir_all(new.join("logs")).unwrap();
    std::fs::write(new.join("seen.txt"), "2026-09-04T12:00:00Z").unwrap();
    std::fs::write(new.join("logs").join("slot-x.txt"), "kept").unwrap();
    // No settings.json and no vault hint: nothing names an id, and the files still move.
    let id = migrate_flat_layout(&base, None).expect("the fold ran on a renamed-but-not-folded root");
    assert_eq!(id, "profile_legacy");
    assert!(!new.join("seen.txt").exists(), "nothing is left at the root");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("seen.txt")).unwrap(), "2026-09-04T12:00:00Z");
    assert_eq!(std::fs::read_to_string(profile_dir(&new, &id).join("logs").join("slot-x.txt")).unwrap(), "kept");
    assert!(migrate_flat_layout(&base, None).is_none(), "and it is done");
}

#[test]
fn launch_resolution_is_none_one_many_and_the_flag_always_wins() {
    let root = temp_root("launch");
    assert!(matches!(resolve_launch(&root, None), Launch::Onboard));
    let a = root.join("A"); let b = root.join("B");
    std::fs::create_dir_all(&a).unwrap(); std::fs::create_dir_all(&b).unwrap();
    register(&root, "A", &a).unwrap();
    match resolve_launch(&root, None) { Launch::Open(p) => assert_eq!(p.vault, a), other => panic!("{other:?}") }
    register(&root, "B", &b).unwrap();
    match resolve_launch(&root, None) { Launch::Pick(ps) => assert_eq!(ps.len(), 2), other => panic!("{other:?}") }
    // --vault wins over everything, registered or not (the Start-menu shortcut and autostart).
    let c = root.join("C"); std::fs::create_dir_all(&c).unwrap();
    match resolve_launch(&root, Some(&c)) { Launch::Open(p) => { assert_eq!(p.vault, c); assert_eq!(p.id, id_for(&c)); } other => panic!("{other:?}") }
    match resolve_launch(&root, Some(&b)) { Launch::Open(p) => assert_eq!(p.name, "B"), other => panic!("{other:?}") }
}

/// R-P4a-1: one Knowlu per machine. Switching profiles is a relaunch, and the child must not
/// register with `tauri-plugin-single-instance` until the parent has actually exited — otherwise
/// the plugin kills the new window and the user is left with nothing.
#[test]
fn the_relaunched_child_waits_for_the_old_process_to_go_and_gives_up_at_the_cap() {
    // A real short-lived process, so a real pid that really disappears. `cmd /c exit` is the
    // cheapest one on this machine and touches nothing.
    let mut child = std::process::Command::new("cmd").args(["/c", "exit"]).spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    let t0 = std::time::Instant::now();
    assert!(wait_for_pid_gone(pid, std::time::Duration::from_secs(10)), "an exited pid is gone");
    assert!(t0.elapsed() < std::time::Duration::from_secs(2), "and it returns at once, not at the cap");

    // Our own pid never goes: the wait is bounded and gives up rather than hanging the launch.
    let t0 = std::time::Instant::now();
    assert!(!wait_for_pid_gone(std::process::id(), std::time::Duration::from_millis(400)));
    assert!(t0.elapsed() >= std::time::Duration::from_millis(400) && t0.elapsed() < std::time::Duration::from_secs(3));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (from `app/`): `cargo test --test profiles`
Expected: FAIL to compile — `unresolved import quinn_ops_console::profiles`.

- [ ] **Step 3: `app/src/profiles.rs`**

```rust
//! Profiles: the registry, the per-profile app-data layout, the one-time move of the flat layout,
//! launch resolution and the per-profile instance guard (plan 4a, Task 2; spec §2, decision 2).
//!
//! A profile is **a vault plus its app data**. `id` is `ids::derived_id("profile", <vault path>)` —
//! the same call `Settings::load` has always made — so a vault opened by `--vault` and the same
//! vault registered by the wizard are one profile.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub vault: PathBuf,
    pub created_at: String,
    pub last_opened_at: Option<String>,
}

pub fn registry_path(root: &Path) -> PathBuf { root.join("profiles.json") }
pub fn profile_dir(root: &Path, id: &str) -> PathBuf { root.join("profiles").join(id) }
/// One folder for the whole install, not per profile: an update is the same bundle whichever
/// profile is open (Knowlu spec §6, "one bundle, one version").
pub fn updates_dir(root: &Path) -> PathBuf { root.join("updates") }

pub fn id_for(vault: &Path) -> String { quinn_ops::ids::derived_id("profile", &vault.to_string_lossy()) }
/// The picker's default label: the vault folder's own name, never a person's name (R1).
pub fn default_name(vault: &Path) -> String {
    vault.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Knowlu".to_string())
}

/// A missing or unparsable registry is an EMPTY registry, never an error: the first launch has
/// none, and a truncated one must not wedge the app out of onboarding.
pub fn load(root: &Path) -> Vec<Profile> {
    std::fs::read_to_string(registry_path(root)).ok()
        .and_then(|t| serde_json::from_str::<Vec<Profile>>(&t).ok())
        .unwrap_or_default()
}

/// Through `ledger::dumps_value`, like every JSON the app writes — Python's `", "`/`": "`
/// separators, so a file this app wrote and a file the engine wrote never differ by whitespace.
pub fn save(root: &Path, profiles: &[Profile]) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let v = serde_json::to_value(profiles).map_err(|e| e.to_string())?;
    std::fs::write(registry_path(root), quinn_ops::ledger::dumps_value(&v)).map_err(|e| e.to_string())
}

/// Idempotent: registering a vault already in the registry updates its name and `last_opened_at`
/// and leaves `created_at` alone. Creates the per-profile folder so `ConsoleState::open` has one.
pub fn register(root: &Path, name: &str, vault: &Path) -> Result<Profile, String> {
    let id = id_for(vault);
    let now = quinn_ops::journal::now_ts(None);
    let mut all = load(root);
    let p = match all.iter_mut().find(|p| p.id == id) {
        Some(e) => { e.name = name.to_string(); e.last_opened_at = Some(now); e.clone() }
        None => {
            let p = Profile { id, name: name.to_string(), vault: vault.to_path_buf(), created_at: now.clone(), last_opened_at: Some(now) };
            all.push(p.clone());
            p
        }
    };
    std::fs::create_dir_all(profile_dir(root, &p.id)).map_err(|e| e.to_string())?;
    save(root, &all)?;
    Ok(p)
}

/// **F14: a move, not a fresh start.** One `rename` of `%LOCALAPPDATA%\quinn-ops` to `knowlu`, then
/// the flat files fold into `profiles\<id>\` and a one-entry registry is written. Runs at most once
/// per machine, and **must run before anything creates the new root** — which is why
/// `state::app_data_root()` calls it first. `vault_hint` is the resolved `--vault`: the old
/// `settings.json` carries the profile id but not the vault path.
pub fn migrate_flat_layout(base: &Path, vault_hint: Option<&Path>) -> Option<String> {
    let old = base.join("quinn-ops");
    let new = base.join("knowlu");
    // The rename half is skipped once `knowlu` exists — but the FOLD half still runs, so a crash
    // between the two (or an `app_data_root()` call that created the root first) never strands the
    // flat files at the new root. Both halves are idempotent (S5).
    if old.is_dir() && !new.exists() {
        std::fs::rename(&old, &new).ok()?;
    }
    if !new.is_dir() { return None; }
    let flat = ["settings.json", "seen.txt", "logs"];
    if !flat.iter().any(|n| new.join(n).exists()) { return None; }
    // The id comes from the old settings file, else from the launch's own vault. If neither is
    // there the files are still MOVED, under a fallback id: leaving them at the root would hide a
    // user's backup folder and seen stamp behind a layout nothing reads any more (S5).
    let id = std::fs::read_to_string(new.join("settings.json")).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("profile_id").and_then(|p| p.as_str()).map(str::to_string))
        .or_else(|| vault_hint.map(id_for))
        .unwrap_or_else(|| "profile_legacy".to_string());
    let dir = profile_dir(&new, &id);
    let _ = std::fs::create_dir_all(&dir);
    for name in flat {
        let src = new.join(name);
        // Never overwrite a per-profile file that is already there: a half-moved state re-run must
        // keep the newer, folded copy and drop the stale root one.
        if src.exists() {
            if dir.join(name).exists() { let _ = std::fs::remove_dir_all(&src).or_else(|_| std::fs::remove_file(&src)); }
            else { let _ = std::fs::rename(&src, dir.join(name)); }
        }
    }
    if let Some(v) = vault_hint { let _ = register(&new, &default_name(v), v); }
    Some(id)
}

#[derive(Debug)]
pub enum Launch { Onboard, Open(Profile), Pick(Vec<Profile>) }

/// `--vault` keeps working exactly as today and always wins — it is how the Start-menu shortcut
/// and autostart launch a profile, registered or not. Without it: none → onboarding, one → open
/// it, more → the picker (spec §2).
pub fn resolve_launch(root: &Path, vault_arg: Option<&Path>) -> Launch {
    if let Some(v) = vault_arg {
        let id = id_for(v);
        if let Some(p) = load(root).into_iter().find(|p| p.id == id) { return Launch::Open(p); }
        return Launch::Open(Profile { id, name: default_name(v), vault: v.to_path_buf(), created_at: quinn_ops::journal::now_ts(None), last_opened_at: None });
    }
    let all = load(root);
    match all.len() {
        0 => Launch::Onboard,
        1 => Launch::Open(all.into_iter().next().expect("len 1")),
        _ => Launch::Pick(all),
    }
}

/// **One Knowlu per machine** (R-P4a-1): `tauri-plugin-single-instance` stays exactly as it is,
/// keyed on the app identifier, and a second launch of any profile is killed by it. Switching
/// profiles is therefore a relaunch, and the child is handed `--after-pid <parent>` so it can wait
/// for the plugin's lock to be free before it registers. Polls every 100 ms up to `cap`; returns
/// `false` if the pid is still there at the cap — the launch goes ahead anyway, and the plugin
/// simply closes the loser, which is a worse outcome than waiting but better than hanging.
///
/// `OpenProcess` failing means the pid is gone (or is not ours to look at, which for our own
/// former parent is the same thing). A live handle is checked with a zero-timeout wait rather than
/// an exit code, so a pid reused by an unrelated process cannot read as "still running forever".
#[cfg(windows)]
pub fn wait_for_pid_gone(pid: u32, cap: std::time::Duration) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };
    let started = std::time::Instant::now();
    loop {
        // SAFETY: a plain handle open/close pair; every branch closes what it opened.
        let gone = unsafe {
            match OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Err(_) => true,
                Ok(h) => {
                    let signalled = WaitForSingleObject(h, 0) == WAIT_OBJECT_0;
                    let _ = CloseHandle(h);
                    signalled
                }
            }
        };
        if gone { return true; }
        if started.elapsed() >= cap { return false; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
#[cfg(not(windows))]
pub fn wait_for_pid_gone(_pid: u32, _cap: std::time::Duration) -> bool { true }
```

- [ ] **Step 4: `app/Cargo.toml` — one plugin in, one Windows feature**

```toml
tauri-plugin-dialog = "2"
```

added under `[dependencies]`. **`tauri-plugin-single-instance` stays** — one Knowlu per machine is the 4a model (R-P4a-1). And:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_System_Threading"] }
```

(`Win32_Security_Credentials` joins this list in Task 3.)

- [ ] **Step 5: `app/capabilities/default.json` — the dialog permission**

Tauri 2 gates every plugin command behind a capability. `app/capabilities/default.json` already exists; add `"dialog:default"` to its `permissions` array, leaving the rest as they are:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "The console window: core APIs, window-state, autostart, clipboard, folder picker.",
  "windows": ["main"],
  "permissions": ["core:default", "window-state:default", "autostart:default", "clipboard-manager:allow-write-text", "dialog:default"]
}
```

- [ ] **Step 6: `state.rs` — the root moves, and every profile gets its own folder**

Replace `app_data_root` with the pair, and add `data_dir` to `ConsoleState`:

```rust
/// The app-data root under an explicit base — the seam the profile tests use. Creating the root
/// here is safe only because `migrate_flat_layout` has already run (it refuses once `knowlu`
/// exists), which is why `app_data_root` calls the migration first and nothing else creates this
/// directory.
pub fn app_data_root_in(base: &Path) -> PathBuf {
    let d = base.join("knowlu");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// `%LOCALAPPDATA%\knowlu` (plan 4a decision 2 — the move plan 2 Task 12 was going to make).
/// `None` only when `LOCALAPPDATA` is unset or empty; callers fall back to Tauri's `app_data_dir()`
/// or the temp dir, as before.
pub fn app_data_root() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty())?;
    let base = PathBuf::from(base);
    crate::profiles::migrate_flat_layout(&base, None);
    Some(app_data_root_in(&base))
}
```

In `ConsoleState`: add `pub data_dir: PathBuf,` next to `seen_path`, and in `open` set `data_dir: app_data_dir.clone(),` before the existing `seen_path` line (which keeps using `app_data_dir.join("seen.txt")`).

- [ ] **Step 7: `scheduler.rs` and `tray.rs` — the log directory is the profile's**

`scheduler::log_dir` loses its global and takes the state:

```rust
/// `<profile app data>\logs` — per profile since plan 4a Task 2, so two profiles on one machine
/// never interleave their slot logs. The temp dir is no longer a fallback here: `ConsoleState`
/// always has a data dir, because `open` was given one.
fn log_dir(cs: &ConsoleState) -> PathBuf {
    let d = cs.data_dir.join("logs");
    let _ = std::fs::create_dir_all(&d);
    d
}
```

Call sites: in `run_slot_inner`, `log_dir().join(...)` becomes `log_dir(cs).join(...)`; in the housekeeping thread, `prune_logs(&log_dir(), LOGS_KEPT)` becomes `prune_logs(&log_dir(&cs), LOGS_KEPT)`. In `tray.rs`'s `"quit"` arm, `let logs = state::app_data_root()…join("logs");` becomes `let logs = cs.data_dir.join("logs");`.

- [ ] **Step 8: `app/src/onboarding.rs` — the state and the first four commands**

```rust
//! The wizard's and the picker's command surface (plan 4a, Tasks 2 and 5). Like `commands.rs`,
//! nothing here computes. These commands run in a shell that has **no `ConsoleState`** — there is
//! no vault yet — which is why `main.rs` builds two Tauri apps: a console over a resolved profile,
//! and this vault-less shell. `launch_state` is registered in both and is how the page tells them
//! apart.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde_json::{json, Value};
use tauri::Manager;
use crate::profiles::{self, Profile};

pub struct Onboarding {
    pub root: PathBuf,
    /// `"wizard"` (no profiles) or `"picker"` (more than one).
    pub mode: &'static str,
    pub profiles: Mutex<Vec<Profile>>,
}

/// `{ mode }` plus, for the shell, the registered profiles and this machine's name (which the
/// wizard writes into a new vault's `runners.yaml` as `device:` — R1: read, never hard-coded).
#[tauri::command]
pub fn launch_state(app: tauri::AppHandle) -> Value {
    match app.try_state::<Onboarding>() {
        Some(o) => json!({
            "ok": true, "error": Value::Null, "mode": o.mode,
            "profiles": serde_json::to_value(&*o.profiles.lock().unwrap_or_else(|e| e.into_inner())).unwrap_or(Value::Null),
            "machine": quinn_ops::journal::device_name(),
            "tz": jiff::tz::TimeZone::system().iana_name().unwrap_or("America/Chicago"),
            // The wizard's default PARENT folder for a new vault (spec §3 panel 2,
            // `%USERPROFILE%\Documents\Knowlu\<name>`): the page never builds a path itself.
            "documents": std::env::var("USERPROFILE").ok().map(|h| std::path::Path::new(&h).join("Documents").join("Knowlu").to_string_lossy().to_string()),
        }),
        None => json!({ "ok": true, "error": Value::Null, "mode": "console" }),
    }
}

/// A native folder picker (`tauri-plugin-dialog`). `(async)` because the blocking dialog must not
/// run on the webview thread. A cancelled dialog is `{ok: true, path: null}` — not an error.
///
/// It can only return a folder that **already exists** — which is why the wizard's *Create a new
/// vault* panel picks a PARENT here and takes the vault's own name from a text field (R-P4a-11);
/// `create_vault` then refuses a `<parent>\<name>` that exists.
#[tauri::command(async)]
pub fn pick_folder(app: tauri::AppHandle, title: String) -> Value {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().set_title(&title).blocking_pick_folder();
    let path = picked.and_then(|p| p.into_path().ok());
    json!({ "ok": true, "error": Value::Null, "path": path.map(|p| p.to_string_lossy().to_string()) })
}

/// *Use an existing vault* (spec §3 panel 2): a folder holding `config/` and `tasks/` becomes a
/// profile. **Nothing is written into the vault** — no scaffold, no seed note, no first task.
///
/// Split into a handle-free core so `app/tests/onboarding.rs` can hash the vault tree either side
/// of the call and prove that (spec §8; S7).
pub fn adopt_vault_in(root: &Path, path: &str, name: Option<String>) -> Value {
    let vault = PathBuf::from(path);
    if !vault.join("config").is_dir() || !vault.join("tasks").is_dir() {
        return json!({ "ok": false, "error": format!("{path}: not a vault — a vault has config/ and tasks/"), "profile": Value::Null });
    }
    let label = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| profiles::default_name(&vault));
    match profiles::register(root, &label, &vault) {
        Ok(p) => json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).unwrap_or(Value::Null) }),
        Err(e) => json!({ "ok": false, "error": e, "profile": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn adopt_vault(app: tauri::AppHandle, path: String, name: Option<String>) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let out = adopt_vault_in(&root, &path, name);
    if out["ok"] == true { refresh(&app, &root); }
    out
}

/// Open a profile: **relaunch this exe with `--vault`, then exit.** The console's whole world —
/// `ConsoleState`, the tray, the scheduler, autostart's arguments — is fixed at startup from one
/// vault, so switching profiles is a new process, not a re-`manage()`.
#[tauri::command(async)]
pub fn open_profile(app: tauri::AppHandle, id: String) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding" }) };
    let Some(p) = profiles::load(&root).into_iter().find(|p| p.id == id) else {
        return json!({ "ok": false, "error": format!("no profile {id}") });
    };
    match relaunch_with(&p.vault) {
        Ok(()) => { app.exit(0); json!({ "ok": true, "error": Value::Null }) }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// `--after-pid <us>` so the child waits for this process to go before it registers with
/// `tauri-plugin-single-instance` (R-P4a-1); without it the plugin kills the new window, since
/// its lock is keyed on the app identifier and is still held here.
pub fn relaunch_with(vault: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .arg("--vault").arg(vault)
        .arg("--after-pid").arg(std::process::id().to_string())
        .spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn refresh(app: &tauri::AppHandle, root: &Path) {
    if let Some(o) = app.try_state::<Onboarding>() {
        *o.profiles.lock().unwrap_or_else(|e| e.into_inner()) = profiles::load(root);
    }
}
```

Add `pub mod profiles;` and `pub mod onboarding;` to `app/src/lib.rs`.

- [ ] **Step 9: `main.rs` — resolve the launch, then build one of two apps**

`main.rs`'s `use` line gains the two new modules — `use quinn_ops_console::{commands, onboarding, profiles, scheduler, state::{app_data_root, resolve_vault, ConsoleState}, tray};` — and **`fn main` is replaced whole** (R-P4a-10). The order matters and is the reason for writing it out: today's `main` calls `resolve_vault` **before** anything else and `fatal()`s on failure, which on a first launch with no `--vault` would kill the wizard in a message box before it ever existed. `resolve_vault` therefore moves inside the two paths that actually need a vault — `--run-slot-once` (where a fatal is exactly right: it is a scripted, non-interactive path) and `run_console`.

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let vault_arg = args.iter().position(|a| a == "--vault").and_then(|i| args.get(i + 1)).cloned();

    // R-P4a-1: a relaunch for a profile switch waits, bounded, for the process that spawned it to
    // exit — `tauri-plugin-single-instance` is keyed on the app identifier, so registering before
    // the parent is gone gets THIS window killed instead of the old one. Bounded at 10 s; past
    // that the launch goes ahead anyway and the plugin closes whichever loses.
    if let Some(pid) = args.iter().position(|a| a == "--after-pid").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<u32>().ok()) {
        profiles::wait_for_pid_gone(pid, std::time::Duration::from_secs(10));
    }

    // Hidden manual-test path (Knowlu plan 1, Task 12): one scheduler slot, synchronously, no
    // window and no tray, summary as JSON on stdout. It is never reached by a normal launch, it
    // always carries `--vault`, and a bad one there IS fatal — nobody is watching a window.
    if args.iter().any(|a| a == "--run-slot-once") {
        let cwd = std::env::current_dir().unwrap_or_default();
        let vault = match resolve_vault(vault_arg.as_deref(), &cwd) { Ok(v) => v, Err(msg) => fatal(&msg) };
        let data = std::env::temp_dir().join("knowlu-run-slot-once-appdata");
        let cs = ConsoleState::open(vault, data);
        let sch = scheduler::Scheduler::default();
        let summary = scheduler::run_slot_inner(&cs, &sch, None, false);
        println!("{}", serde_json::to_string(&summary).unwrap_or_default());
        let code = if summary.reason.is_some() { 2 } else if summary.ok { 0 } else { 1 };
        std::process::exit(code);
    }

    // F14, and it must run before anything creates the new root: the move refuses once `knowlu`
    // exists, and it needs the resolved vault to write the one-entry registry.
    if let Some(base) = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty()) {
        profiles::migrate_flat_layout(std::path::Path::new(&base), vault_arg.as_deref().map(std::path::Path::new));
    }
    let root = match app_data_root() { Some(r) => r, None => std::env::temp_dir().join("knowlu") };

    // `--pick` forces the picker whatever the registry holds: it is how *Switch profile…* in the
    // settings overlay gets back to the chooser with one profile registered, or none (R-P4a-15).
    let launch = if args.iter().any(|a| a == "--pick") {
        profiles::Launch::Pick(profiles::load(&root))
    } else {
        profiles::resolve_launch(&root, vault_arg.as_deref().map(std::path::Path::new))
    };
    match launch {
        // S2: the profile is registered only AFTER the vault has been validated — a `--vault` typo
        // must not leave a broken entry in `profiles.json` that the picker then offers forever.
        profiles::Launch::Open(p) => run_console(p, root),
        profiles::Launch::Onboard => run_shell(root, "wizard", Vec::new()),
        profiles::Launch::Pick(ps) => run_shell(root, "picker", ps),
    }
}
```

and two functions are added. The first is new; the second is today's builder, moved into a function, with three edits — **the `tauri_plugin_single_instance` line stays** (R-P4a-1); `.plugin(tauri_plugin_dialog::init())` is added; `ConsoleState::open` takes the per-profile `data_dir` instead of `app_data_root()`; and `onboarding::launch_state, onboarding::pick_folder` join `generate_handler!`.

```rust
/// The vault-less shell: the same window and the same page, no `ConsoleState`, no tray, no
/// scheduler, no autostart registration — none of those has a vault to point at yet. It ends by
/// relaunching into `run_console` (`open_profile` / `finish_onboarding`), never by managing a
/// `ConsoleState` after the fact.
fn run_shell(root: std::path::PathBuf, mode: &'static str, ps: Vec<profiles::Profile>) -> ! {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            app.manage(onboarding::Onboarding { root, mode, profiles: std::sync::Mutex::new(ps) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::adopt_vault, onboarding::open_profile])
        .run(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime");
    std::process::exit(0)
}

/// The console over one profile. `resolve_vault` runs HERE, so a registered profile whose folder
/// has since been deleted reaches the existing `MessageBoxW` instead of panicking — and the
/// registry is only touched once that has passed (S2).
fn run_console(p: profiles::Profile, root: std::path::PathBuf) -> ! {
    let cwd = std::env::current_dir().unwrap_or_default();
    let vault = match resolve_vault(p.vault.to_str(), &cwd) { Ok(v) => v, Err(msg) => fatal(&msg) };
    let _ = profiles::register(&root, &p.name, &vault);
    let data_dir = profiles::profile_dir(&root, &p.id);
    // `Box::leak` because `tauri_plugin_autostart` wants `&'static str` for the arguments it
    // registers — unchanged from today, and the reason autostart relaunches THIS profile.
    let vault_str: &'static str = Box::leak(vault.to_string_lossy().into_owned().into_boxed_str());
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--vault", vault_str])))
        .setup(move |app| {
            let cs = ConsoleState::open(vault.clone(), data_dir.clone());
            let autostart = cs.settings.lock().unwrap().autostart;
            app.manage(cs);
            app.manage(scheduler::Scheduler::default());
            tray::build(app.handle())?;
            use tauri_plugin_autostart::ManagerExt;
            let al = app.autolaunch();
            if autostart { let _ = al.enable(); } else { let _ = al.disable(); }
            let h = app.handle().clone();
            std::thread::spawn(move || {
                let cs = h.state::<ConsoleState>();
                quinn_ops_console::state::refresh_head(&cs);
                quinn_ops_console::state::refresh_history(&cs);
            });
            scheduler::spawn(app.handle().clone());
            Ok(())
        })
        .on_window_event(|w, e| { if let tauri::WindowEvent::CloseRequested { api, .. } = e { api.prevent_close(); let _ = w.hide(); } })
        .invoke_handler(tauri::generate_handler![commands::state, commands::note, commands::mark_seen, commands::ui_event, commands::set_fields, commands::create_task, commands::delete_note, commands::decide, commands::close_info, commands::open_issue, commands::resolve_issue, commands::sync, commands::backup_now, commands::get_settings, commands::set_settings, onboarding::launch_state, onboarding::pick_folder])
        .run(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime");
    std::process::exit(0)
}
```

(M4: `tauri::generate_context!()` is expanded twice, once per builder. It re-embeds `static/` in the binary — measure the release exe before and after in step 13 and record the delta in the task report; if it is more than a megabyte, plan 4b factors the two builders onto one context. It is not worth a shared-context refactor before the number is known.)

- [ ] **Step 10: The picker in the page**

`index.html`, directly after `<div class="app">`'s closing `</div>` and before the drawer:

```html
<section class="picker" id="picker" hidden>
  <h1>Knowlu</h1>
  <p class="lede" id="pick-lede"></p>
  <div id="pick-list"></div>
  <div class="wiz-nav"><button class="b" id="pick-adopt">Use an existing vault…</button></div>
</section>
```

`console.js` — the boot becomes two functions, and the final line changes:

```js
  // Plan 4a Task 2: the console is one of three things this window can be. `launch_state` says
  // which: a console over a resolved profile, the picker (more than one profile), or the wizard
  // (none). The console's own listeners are bound only on the console path — a picker window has
  // no deck, no nav and no poll.
  function bootConsole() {
    window.addEventListener("hashchange", function () { route(location.hash.slice(1)); });
    window.addEventListener("focus", poll);
    setInterval(poll, 60000);
    window.addEventListener("blur", endOfLook);
    document.addEventListener("visibilitychange", function () { if (document.hidden) { endOfLook(); } });
    route(location.hash.slice(1));
  }

  function renderPicker(l) {
    var ps = l.profiles || [];
    EL("picker").hidden = false;
    document.querySelector(".app").hidden = true;
    EL("pick-lede").textContent = ps.length ? "Which vault?" : "No profile on this machine yet.";
    EL("pick-list").innerHTML = ps.map(function (p) {
      return '<div class="row pick" data-profile="' + h(p.id) + '"><div class="ttl"><span class="a">' + h(p.name) +
        '</span><span class="meta">' + h(p.vault) + "</span></div>" +
        '<div class="acts"><button class="b pri y" data-open="' + h(p.id) + '">Open</button></div></div>';
    }).join("");
  }

  EL("pick-list").addEventListener("click", function (e) {
    var b = e.target.closest("button[data-open]"); if (!b) { return; }
    b.disabled = true;
    invoke("open_profile", { id: b.getAttribute("data-open") }).catch(function () { b.disabled = false; });
  });
  EL("pick-adopt").addEventListener("click", function () {
    invoke("pick_folder", { title: "Choose your Knowlu vault folder" }).then(function (r) {
      if (!r || !r.path) { return; }
      return invoke("adopt_vault", { path: r.path, name: null }).then(function (a) {
        if (!a.ok) { EL("pick-lede").textContent = a.error; return; }
        return invoke("open_profile", { id: a.profile.id });
      });
    }).catch(function () {});
  });

  invoke("launch_state", {}).then(function (l) {
    if (l && l.mode === "console") { bootConsole(); return; }
    renderPicker(l || { profiles: [] });
  }).catch(function () { bootConsole(); });
```

The old trailing `bindDeck(); … route(location.hash.slice(1));` block keeps `bindDeck()`, `bindDecisionsView()`, `bindGoodToKnowView()`, `bindIssuesView()` where they are (binding a listener on a hidden node is harmless) and **loses only its five boot lines**, which are now `bootConsole`'s.

- [ ] **Step 11: CSS**

```css
.picker { max-width: 560px; margin: var(--s8) auto; padding: 0 var(--s5); }
.picker h1 { font-size: 22px; margin: 0 0 var(--s2); }
.row.pick { border-bottom: 1px solid var(--hair); padding: var(--s3) 0; display: flex; align-items: center; }
.row.pick .meta { color: var(--t4); font: 500 11px/1.4 var(--mono); display: block; }
.wiz-nav { display: flex; gap: var(--s2); justify-content: flex-end; margin-top: var(--s5); }
```

- [ ] **Step 12: The static-asset test gains the picker**

Add to `app/tests/static_assets.rs`:

```rust
#[test]
fn the_picker_is_in_the_page_and_carries_no_data_id() {
    let html = read("index.html");
    assert!(html.contains("id=\"picker\"") && html.contains("id=\"pick-list\"") && html.contains("id=\"pick-adopt\""));
    let js = read("console.js");
    assert!(js.contains("function bootConsole(") && js.contains("function renderPicker("));
    assert!(js.contains("invoke(\"launch_state\""), "the boot asks which window this is");
    // R-T15b's count equality still holds: picker rows are keyed by data-profile, never data-id.
    // **The leading space is the repo's spelling** (`app/tests/static_assets.rs:272`) — it excludes
    // CSS selectors like `[data-id="` and prose in comments. Copy it exactly; the unspaced form
    // counts things that are not row templates and the assertion stops meaning anything.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
    assert!(js.contains("data-profile=\""));
}
```

- [ ] **Step 13: Run everything**

Run (from `app/`): `cargo test` — expected: all green, zero new warnings; `profiles` (5 tests) and `static_assets` pass. Then root `cargo test` (unchanged, 0 warnings). Then `cargo build --release` in `app/` and record `(Get-Item app\target\release\knowlu.exe).Length` against the size before this task — that is M4's number for the second `generate_context!()`.

- [ ] **Step 14: Look at it, headless**

```bash
.wv/Scripts/python scripts/console-shots.py tests/fixtures/surface-today-full.json shots/
```

Expected: unchanged output — the picker is `hidden` on a console launch, so no viewport gains overflow. (`console-shots.py` has no `__TAURI__`, so `invoke("launch_state")` rejects and the boot falls through to `bootConsole()` — which is exactly the `.catch` branch above, and is why it is there.)

- [ ] **Step 15: Commit**

```bash
git add app/src/profiles.rs app/src/onboarding.rs app/src/lib.rs app/src/main.rs app/src/state.rs app/src/scheduler.rs app/src/tray.rs app/Cargo.toml app/Cargo.lock app/capabilities/default.json app/static/index.html app/static/console.js app/static/console.css app/tests/profiles.rs app/tests/static_assets.rs
```

Message: `app: profiles — a registry, per-profile app data, the %LOCALAPPDATA%\knowlu move, launch resolution and the picker (Knowlu plan 4a, Task 2)`.

---

### Task 3: Credentials — the write half of Credential Manager

**Files:**
- Create: `app/src/credentials.rs`, `app/tests/credentials.rs`
- Modify: `app/src/lib.rs`, `app/Cargo.toml`

**Interfaces:**
- Consumes: `quinn_ops::wincred::read_credential(target) -> Result<Credential, CredError>` (read-only, unchanged); `quinn_ops::ids::new_id`.
- Produces: `credentials::target_for(profile_id: &str, source: &str) -> String`; `credentials::write(target: &str, user: &str, secret: &str) -> Result<(), String>`; `credentials::exists(target: &str) -> bool`; `credentials::delete(target: &str) -> Result<(), String>`.

- [ ] **Step 1: Write the failing test** (`app/tests/credentials.rs`)

```rust
#![cfg(windows)]
use quinn_ops_console::credentials::{delete, exists, target_for, write};

/// Round-trips one credential through the app's writer and the ENGINE'S reader — the whole point
/// of the module is that `src/wincred.rs` keeps reading, unchanged, what the app now writes.
///
/// The target and the secret are minted at runtime and the target is deleted at the end: no secret
/// and no fixed target ever appears in this repo (spec §5).
#[test]
fn a_credential_written_by_the_app_is_read_by_the_engine_and_then_deleted() {
    let target = format!("knowlu/test/{}", quinn_ops::ids::new_id("t"));
    let user = "knowlu-test@example.invalid";
    let secret = quinn_ops::ids::new_id("s");

    assert!(!exists(&target), "the throwaway target must not already exist");
    write(&target, user, &secret).expect("write");
    assert!(exists(&target));

    let got = quinn_ops::wincred::read_credential(&target).expect("the engine reads it");
    assert_eq!(got.username, user);
    assert_eq!(got.password.expose(), secret, "byte-for-byte — the UTF-16 blob-size trap");

    write(&target, user, "second-value-then-overwritten").expect("rewrite is an update, not a duplicate");
    assert_eq!(quinn_ops::wincred::read_credential(&target).unwrap().password.expose(), "second-value-then-overwritten");

    delete(&target).expect("delete");
    assert!(!exists(&target));
    assert!(quinn_ops::wincred::read_credential(&target).is_err(), "gone for the engine too");
    assert!(delete(&target).is_err(), "deleting twice is an error, not a silent success");
}

#[test]
fn targets_are_namespaced_per_profile_so_two_profiles_never_share_a_login() {
    assert_eq!(target_for("profile_0123456789", "zybooks"), "knowlu/profile_0123456789/zybooks");
    assert_ne!(target_for("profile_aaaaaaaaaa", "vhl"), target_for("profile_bbbbbbbbbb", "vhl"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run (from `app/`): `cargo test --test credentials`
Expected: FAIL to compile — `unresolved import quinn_ops_console::credentials`.

- [ ] **Step 3: `app/Cargo.toml` — the credentials feature**

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_System_Threading", "Win32_Security_Credentials"] }
```

- [ ] **Step 4: `app/src/credentials.rs`**

```rust
//! Writing Windows Credential Manager entries the engine already reads (plan 4a, Task 3; spec §5).
//! The engine's `src/wincred.rs` is the reader and does not change.
//!
//! **Rules, unchanged from every earlier document:** a secret is never logged, never in a run
//! record, a backup, a fixture, a test name or a plan. Nothing here ever formats a secret — no
//! `Debug`, no `Display`, no error message that carries one; `secret` is only ever read as bytes.
#![cfg(windows)]

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};

/// `knowlu/<profile_id>/<source>` (decision 6) — two profiles on one machine never share a login,
/// and Quinn's existing `quinn-ops/zybooks` and `quinn-ops/vhl` are a different namespace entirely.
pub fn target_for(profile_id: &str, source: &str) -> String { format!("knowlu/{profile_id}/{source}") }

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

/// Creates or replaces the generic credential at `target`. The blob is UTF-16LE and
/// `CredentialBlobSize` is a count of **bytes** — the trap `wincred.rs`'s
/// `blob_size_is_bytes_not_code_units` pins from the reading side; getting it wrong here fails
/// silently on the vendor login three layers away.
pub fn write(target: &str, user: &str, secret: &str) -> Result<(), String> {
    let mut t = wide(target);
    let mut u = wide(user);
    let blob: Vec<u16> = secret.encode_utf16().collect();
    let mut bytes: Vec<u8> = blob.iter().flat_map(|c| c.to_le_bytes()).collect();
    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(t.as_mut_ptr()),
        CredentialBlobSize: bytes.len() as u32,
        CredentialBlob: bytes.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(u.as_mut_ptr()),
        ..Default::default()
    };
    // SAFETY: every pointer in `cred` is to a local that outlives the call; the API copies.
    unsafe { CredWriteW(&cred, 0) }.map_err(|e| format!("credential write failed for {target}: {}", e.code().0))
}

/// True when a credential with this target exists. Never returns or logs its contents.
pub fn exists(target: &str) -> bool {
    let t = wide(target);
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: `t` is NUL-terminated and outlives the call; `ptr` is a valid out-parameter and is
    // freed immediately, without ever being read.
    unsafe {
        if CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, Some(0), &mut ptr).is_err() || ptr.is_null() {
            return false;
        }
        CredFree(ptr as *const std::ffi::c_void);
    }
    true
}

/// Removes the credential. A target that is not there is an error, not a silent success — a
/// "cleared" login the user can still authenticate with is the worst possible outcome.
pub fn delete(target: &str) -> Result<(), String> {
    let t = wide(target);
    // SAFETY: `t` is NUL-terminated and outlives the call.
    unsafe { CredDeleteW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None) }
        .map_err(|e| format!("credential delete failed for {target}: {}", e.code().0))
}
```

Add `#[cfg(windows)] pub mod credentials;` to `app/src/lib.rs`.

- [ ] **Step 5: Run the tests**

Run (from `app/`): `cargo test --test credentials -- --nocapture`
Expected: PASS, 2 tests. Then `cargo test` — all green, zero new warnings.

- [ ] **Step 6: Prove nothing was left behind**

```powershell
cmdkey /list | Select-String -Pattern "knowlu/test"
```

Expected: no output. (If a target survives a killed test run, delete it with `cmdkey /delete:<target>` and say so in the task report.)

- [ ] **Step 7: Commit**

```bash
git add app/src/credentials.rs app/src/lib.rs app/Cargo.toml app/Cargo.lock app/tests/credentials.rs
```

Message: `app: credentials — the write half of Credential Manager, namespaced per profile (Knowlu plan 4a, Task 3)`.

---
### Task 4: The scaffold, the campus preset, the seed writes, and the atomic materialisation

**Files:**
- Create: `app/src/scaffold.rs`, `app/assets/scaffold/planning.yaml`, `app/assets/scaffold/week_template.yaml`, `app/assets/scaffold/gitignore.txt`, `app/assets/campus/none.yaml`, `app/assets/campus/university-of-alabama.yaml`, `app/tests/scaffold.rs`
- Modify: `app/src/lib.rs`

**Interfaces:**
- Consumes: `quinn_ops::write::{create, WriteContext}`; `quinn_ops::journal::Journal::new`; `quinn_ops::yamlemit::{safe_dump_block, Node}`; `crate::commands::console_ctx()`; `crate::credentials::target_for`; `quinn_ops::cli::{run_with, Fetchers}` (test only); `quinn_ops::events::load_events_config` (test only).
- Produces:
  - `scaffold::VaultPlan { profile_id: String, ics_url: Option<String>, timezone: String, slots: Vec<String>, device: String, campus: String, zybooks: bool, vhl: bool }`
  - `scaffold::CAMPUSES: [(&'static str, &'static str); 2]` — `(key, label)` pairs for the wizard's radio list
  - `scaffold::campus_yaml(key: &str) -> Option<&'static str>`
  - `scaffold::ingest_yaml(&VaultPlan) -> String`; `scaffold::runners_yaml(&VaultPlan) -> String`
  - `scaffold::create_vault(dest: &Path, plan: &VaultPlan) -> Result<(), String>` — atomic
  - `scaffold::seed_writes(vault: &Path, plan: &VaultPlan) -> Result<(), String>` — the two engine calls

- [ ] **Step 1: Write the failing tests** (`app/tests/scaffold.rs`)

```rust
use quinn_ops_console::scaffold::{campus_yaml, create_vault, seed_writes, VaultPlan, CAMPUSES};
use std::path::{Path, PathBuf};

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-scaffold-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn plan(id: &str) -> VaultPlan {
    VaultPlan {
        profile_id: id.to_string(),
        ics_url: Some("https://lms.example.invalid/feed/learn.ics".to_string()),
        timezone: "America/Chicago".to_string(),
        slots: vec!["12:00".to_string(), "18:00".to_string()],
        device: "TEST-MACHINE".to_string(),
        campus: "university-of-alabama".to_string(),
        zybooks: true,
        vhl: false,
    }
}

/// The whole point of the scaffold: a vault that did not exist a second ago RANKS — no crash, and
/// **no `journal has no migration records`**, which is what a friend would otherwise meet on their
/// first slot (decision 5). No network: `run_with`'s two fetcher seams both refuse.
#[test]
fn a_scaffolded_vault_ranks_without_the_unmigrated_warning() {
    let root = temp("ranks");
    let v = root.join("Vault");
    let p = plan("profile_1111111111");
    create_vault(&v, &p).unwrap();
    seed_writes(&v, &p).unwrap();

    let no_net = |_: &str| -> Result<String, String> { Err("no network in tests".to_string()) };
    let fetchers = quinn_ops::cli::Fetchers {
        calendar: Some(&no_net as &dyn Fn(&str) -> Result<String, String>),
        events: Some(&no_net as &dyn Fn(&str) -> Result<String, String>),
    };
    let out = quinn_ops::cli::run_with(&v, Some("2026-09-07"), "local", None, fetchers).expect("rank runs");
    assert!(out.output.is_file(), "state/today.md was written");
    let all = format!("{} {}", out.summary, out.steps.iter().map(|s| s.message.clone()).collect::<Vec<_>>().join(" "));
    assert!(!all.contains("journal has no migration records"), "the unmigrated guard is satisfied from day one: {all}");
    let tasks = out.steps.iter().find(|s| s.name == "tasks").unwrap();
    assert!(tasks.counts.iter().any(|(_, n)| *n >= 1), "the first task is there to order: {:?}", tasks.counts);
}

/// Decision 3 + spec §3.1: the scaffold is materialised beside the target and renamed in, so a
/// crash never leaves a half-vault. A refused create leaves NOTHING — not the destination, not a
/// staging folder.
#[test]
fn the_scaffold_is_all_or_nothing() {
    let root = temp("atomic");
    let v = root.join("Vault");
    create_vault(&v, &plan("profile_2222222222")).unwrap();
    assert!(v.join("config").join("planning.yaml").is_file());
    let strays: Vec<_> = std::fs::read_dir(&root).unwrap().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".knowlu-new-")).collect();
    assert!(strays.is_empty(), "the staging folder is renamed, never left behind");

    let err = create_vault(&v, &plan("profile_2222222222")).unwrap_err();
    assert!(err.contains("already"), "{err}");
    let strays: Vec<_> = std::fs::read_dir(&root).unwrap().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".knowlu-new-")).collect();
    assert!(strays.is_empty(), "a refused create cleans its staging folder up");
}

/// Decision 4: a vault born in the wizard is `scheduler: app` on the machine that made it, from
/// birth — and `device:` still gates a second install (F3).
#[test]
fn a_fresh_vault_is_scheduler_app_on_the_machine_that_made_it() {
    let v = temp("runners").join("Vault");
    create_vault(&v, &plan("profile_3333333333")).unwrap();
    let cfg = v.join("config").join("runners.yaml");
    let s = quinn_ops::runs::runner_settings(&cfg, "local");
    assert_eq!(s.scheduler, quinn_ops::schedule::SchedulerMode::App);
    assert_eq!(s.device.as_deref(), Some("TEST-MACHINE"));
    let runners = quinn_ops::runs::load_runners_config(&cfg).unwrap();
    let local = runners.iter().find(|r| r.name == "local").unwrap();
    assert_eq!(local.times, vec!["12:00".to_string(), "18:00".to_string()]);
    assert_eq!(local.tz, "America/Chicago");
    assert_eq!(local.grace_minutes, 20);
}

/// The campus preset is a file, and the file is the shape `load_events_config` already reads —
/// so adding a campus is adding a file, and never a code change (spec §3, panel 6).
#[test]
fn the_campus_preset_is_the_shape_the_engine_already_reads() {
    let v = temp("campus").join("Vault");
    create_vault(&v, &plan("profile_4444444444")).unwrap();
    let (cfg, warnings) = quinn_ops::events::load_events_config(&v.join("config").join("events.yaml"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(cfg.sources.len(), 6, "the six UA sources");
    assert_eq!(CAMPUSES.len(), 2);
    assert!(campus_yaml("none").unwrap().contains("sources: []"));
    assert!(campus_yaml("not-a-campus").is_none());

    let none = temp("campus-none").join("Vault");
    let mut p = plan("profile_5555555555"); p.campus = "none".into();
    create_vault(&none, &p).unwrap();
    let (cfg, warnings) = quinn_ops::events::load_events_config(&none.join("config").join("events.yaml"));
    assert!(warnings.is_empty() && cfg.sources.is_empty());
}

/// R2 + decision 5: exactly ONE `system:migration` record, and every other write the wizard makes
/// is an ordinary `quinn`/`dashboard` one. An adopted vault gets neither.
#[test]
fn one_migration_record_and_the_rest_are_dashboard_writes() {
    let v = temp("journal").join("Vault");
    let p = plan("profile_6666666666");
    create_vault(&v, &p).unwrap();
    seed_writes(&v, &p).unwrap();
    let mut records = Vec::new();
    for e in std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap().replace("\r\n", "\n");
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            records.push(serde_json::from_str::<serde_json::Value>(line).unwrap());
        }
    }
    let migration: Vec<_> = records.iter().filter(|r| r["actor"] == "system:migration").collect();
    assert_eq!(migration.len(), 1, "exactly one, and it is a create");
    assert_eq!(migration[0]["op"], "create");
    assert_eq!(migration[0]["via"], "cli", "the S1 migration's own shape — the via vocabulary does not grow");
    assert!(migration[0]["path"].as_str().unwrap().contains("archive/_migrated.md"));
    let others: Vec<_> = records.iter().filter(|r| r["actor"] != "system:migration").collect();
    assert_eq!(others.len(), 1, "the first task, and nothing else");
    assert_eq!(others[0]["actor"], "quinn");
    assert_eq!(others[0]["via"], "dashboard");
    // The credential target the engine will read is named, and holds no secret.
    let ingest = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    assert!(ingest.contains("credential_target: \"knowlu/profile_6666666666/zybooks\""), "{ingest}");
    assert!(!ingest.contains("vhl:"), "a friend with no VHL course gets no VHL block");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (from `app/`): `cargo test --test scaffold`
Expected: FAIL to compile — `unresolved import quinn_ops_console::scaffold`.

- [ ] **Step 3: The embedded asset files**

`app/assets/scaffold/planning.yaml`:

```yaml
daily_effort_budget: 4.0
slice_hours: 2.0
daily_approval_budget: 15
recurring: []
```

`app/assets/scaffold/week_template.yaml`:

```yaml
day_start: "08:00"
day_end: "18:00"
min_block_minutes: 45
classes:
  mon: []
  tue: []
  wed: []
  thu: []
  fri: []
  sat: []
  sun: []
```

`app/assets/scaffold/gitignore.txt` (named with an extension so the folder is not itself affected; written into the vault as `.gitignore`):

```
state/.sync.lock
state/events-ui/
```

`app/assets/campus/none.yaml`:

```yaml
# No campus preset. Event discovery is off; add sources here to turn it on.
sources: []
```

`app/assets/campus/university-of-alabama.yaml` — the six sources by host, public URLs only, no secrets:

```yaml
# University of Alabama. Six public event feeds; each host must be reachable from this machine.
roster_window_days: 90
audit_window_days: 14
judge_per_run_cap: 150
propose_horizon_days: 14
urgency_window_days: 7
daily_proposal_target: 5
daily_proposal_ceiling: 15

sources:
  - name: blount
    type: ics
    url: "https://blount.as.ua.edu/events/?ical=1"
    enabled: true
  - name: career-center
    type: ics
    url: "https://calendar.ua.edu/department/career_center/calendar.ics"
    enabled: true
  - name: campus
    type: localist
    url: "https://calendar.ua.edu/api/2/events"
    enabled: true
  - name: clubs
    type: engage
    url: "https://ua.campuslabs.com/engage/api/discovery/event/search"
    enabled: true
  - name: engineering
    type: html
    url: "https://students.eng.ua.edu/events/"
    enabled: true
  - name: edge
    type: html
    url: "https://edge.culverhouse.ua.edu/events/"
    enabled: true
```

- [ ] **Step 4: `app/src/scaffold.rs`**

```rust
//! The vault a wizard makes from nothing (plan 4a, Task 4; spec §3.1).
//!
//! Two halves, deliberately separate: `create_vault` writes **files only**, atomically, and
//! `seed_writes` makes the **two engine calls** that follow — which is what keeps the one
//! `system:migration` record (decision 5) in a function a reviewer can read at once. Adopting or
//! restoring a vault calls NEITHER: writing a seed note into an existing vault would be the app
//! inventing history.
use std::path::{Path, PathBuf};
use quinn_ops::journal::Journal;
use quinn_ops::write::{self, WriteContext};
use quinn_ops::yamlemit::{safe_dump_block, Node};

/// `(key, label)` — the wizard's radio list. **Adding a campus is adding a file** and one line
/// here; nothing else in the app knows a campus exists.
pub const CAMPUSES: [(&str, &str); 2] = [("none", "None"), ("university-of-alabama", "University of Alabama")];

const CAMPUS_NONE: &str = include_str!("../assets/campus/none.yaml");
const CAMPUS_UA: &str = include_str!("../assets/campus/university-of-alabama.yaml");
const PLANNING: &str = include_str!("../assets/scaffold/planning.yaml");
const WEEK_TEMPLATE: &str = include_str!("../assets/scaffold/week_template.yaml");
const GITIGNORE: &str = include_str!("../assets/scaffold/gitignore.txt");

pub fn campus_yaml(key: &str) -> Option<&'static str> {
    match key { "none" => Some(CAMPUS_NONE), "university-of-alabama" => Some(CAMPUS_UA), _ => None }
}

#[derive(Debug, Clone)]
pub struct VaultPlan {
    pub profile_id: String,
    pub ics_url: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub device: String,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
}

/// The vault's own settings. **No secret is ever written here** — only the `credential_target`
/// names the engine's `wincred::read_credential` looks up (decision 6).
pub fn ingest_yaml(p: &VaultPlan) -> String {
    let mut s = String::new();
    if let Some(u) = &p.ics_url { s.push_str(&format!("ics_url: \"{}\"\n", u.replace('"', "'"))); }
    s.push_str(&format!("timezone: {}\n", p.timezone));
    s.push_str("course_map: {}\n");
    s.push_str("calendars: []\n");
    if p.zybooks || p.vhl {
        s.push_str("\n# Passwords are NOT here. They live in Windows Credential Manager under the\n");
        s.push_str("# credential_target names below.\ncoursework:\n");
        if p.zybooks {
            s.push_str(&format!("  zybooks:\n    enabled: true\n    credential_target: \"{}\"\n    courses: {{}}\n", crate::credentials::target_for(&p.profile_id, "zybooks")));
        }
        if p.vhl {
            s.push_str(&format!("  vhl:\n    enabled: true\n    credential_target: \"{}\"\n    sections: {{}}\n", crate::credentials::target_for(&p.profile_id, "vhl")));
        }
    }
    s
}

/// Decision 4: `scheduler: app` and `device:` from birth. `grace_minutes: 20` is the local
/// runner's own number, unchanged; `cloud` is deliberately absent — a friend has no cloud runner,
/// and an expected-but-never-seen runner would paint every Runs view amber forever.
pub fn runners_yaml(p: &VaultPlan) -> String {
    let times = p.slots.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(", ");
    format!(
        "runners:\n  - name: local\n    times: [{times}]\n    tz: {}\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n",
        p.timezone, p.device
    )
}

/// Materialise into a sibling staging folder, then **one rename** into place: a crash, a full disk
/// or a refused write leaves the destination untouched and the staging folder removed — never a
/// half-vault (spec §3.1).
pub fn create_vault(dest: &Path, plan: &VaultPlan) -> Result<(), String> {
    if dest.exists() { return Err(format!("{}: already exists", dest.display())); }
    let parent = dest.parent().ok_or_else(|| format!("{}: no parent folder", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging: PathBuf = parent.join(format!(".knowlu-new-{}", quinn_ops::ids::new_id("stage")));
    let built = build_into(&staging, plan);
    match built.and_then(|()| std::fs::rename(&staging, dest).map_err(|e| e.to_string())) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = std::fs::remove_dir_all(&staging); Err(e) }
    }
}

fn build_into(root: &Path, plan: &VaultPlan) -> Result<(), String> {
    for folder in ["tasks", "approvals", "archive", "courses", "issues", "info", "state", "state/journal", "config"] {
        std::fs::create_dir_all(root.join(folder)).map_err(|e| e.to_string())?;
    }
    let campus = campus_yaml(&plan.campus).ok_or_else(|| format!("unknown campus preset {:?}", plan.campus))?;
    // `pystr::write_text` for every one of these: the repo's files are CRLF and the engine reads
    // them back through the same translation (CLAUDE.md's line-ending rule).
    let files: [(&str, String); 6] = [
        ("config/planning.yaml", PLANNING.to_string()),
        ("config/week_template.yaml", WEEK_TEMPLATE.to_string()),
        ("config/ingest.yaml", ingest_yaml(plan)),
        ("config/runners.yaml", runners_yaml(plan)),
        ("config/events.yaml", campus.to_string()),
        (".gitignore", GITIGNORE.to_string()),
    ];
    for (rel, text) in files {
        quinn_ops::pystr::write_text(&root.join(rel), &text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

const SEED_BODY: &str = "This vault was created by Knowlu's onboarding. The note exists so the \
journal has a migration record from the vault's first day; nothing reads its body.";

const FIRST_TASK_BODY: &str = "Open the drawer on this row to see every field Knowlu keeps. Tick \
it off when you have had a look — Today will fill up as your courses do.";

/// The two engine calls, in this order (spec §3.1). Both go through `quinn_ops::write`, and only
/// the first uses a non-`console_ctx()` context — **the one exception in the whole app** (R2).
pub fn seed_writes(vault: &Path, _plan: &VaultPlan) -> Result<(), String> {
    let mut journal = Journal::new(vault);

    // Decision 5: `passes::detect_external` refuses to attribute edits unless the journal holds a
    // `create` record whose actor starts with `system:migration` — the shape
    // `scripts/migrate_s1.py` left in Quinn's journal. One archived note satisfies it from the
    // vault's first day, so a friend never meets "journal has no migration records".
    // `via: "cli"` — the exact context `scripts/migrate_s1.py` and `seed_migrated` used. The
    // journal's `via` vocabulary does not grow for this (R-P4a-6): the record is recognised by its
    // ACTOR, which is what `passes.rs` keys on.
    let seed = format!("---\ntitle: Vault created by Knowlu\nstatus: archived\n---\n\n{SEED_BODY}\n");
    write::create(vault, "archive/_migrated.md", &seed, &WriteContext::new("system:migration", "cli"), &mut journal, None)
        .map_err(|e| e.to_string())?;

    // …and one real task, so Today is not empty and the first `rank` has something to order.
    let front = Node::map(vec![
        ("title", Node::text("Get to know Knowlu")),
        ("course", Node::Null),
        ("domain", Node::text("school")),
        ("due", Node::Null),
        ("effort_hours", Node::Float(0.5)),
        ("effort_source", Node::text("quinn")),
        ("importance", Node::Int(2)),
        ("status", Node::text("active")),
        ("progress", Node::Int(0)),
        ("created_by", Node::text("quinn")),
    ]);
    let text = format!("---\n{}---\n\n{FIRST_TASK_BODY}\n", safe_dump_block(&front));
    write::create(vault, "tasks/get-to-know-knowlu.md", &text, &crate::commands::console_ctx(), &mut journal, None)
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

Add `pub mod scaffold;` to `app/src/lib.rs`.

> The seed's context is `("system:migration", "cli")` — the shape the S1 migration left in Quinn's
> journal (R-P4a-6). `passes::detect_external` keys on the **actor** prefix, not the `via`, so this
> satisfies the guard while leaving `journal::VIAS` exactly as it is (R7). Task 11 amends the
> spec's decision-5 wording, which said `onboarding`.

- [ ] **Step 5: Run the tests**

Run (from `app/`): `cargo test --test scaffold -- --nocapture`
Expected: PASS, 5 tests. Then `cargo test` — all green, zero new warnings.

- [ ] **Step 6: Look at the vault a wizard would make**

```powershell
Get-ChildItem -Recurse (Join-Path $env:TEMP "knowlu-scaffold-ranks-*") | Select-Object -First 40 FullName
```

Expected (the test above left it there): the nine folders, five config files, `.gitignore`, `state/today.md`, `state/journal/*.jsonl`, `archive/_migrated.md`, `tasks/get-to-know-knowlu.md`. Read `state/today.md` and confirm the first task appears; delete the folders afterwards.

- [ ] **Step 7: Commit**

```bash
git add app/src/scaffold.rs app/src/lib.rs app/assets app/tests/scaffold.rs
```

Message: `app: the scaffold — a vault made from nothing that ranks, with the migration seed and scheduler: app from birth (Knowlu plan 4a, Task 4)`.

---

### Task 5: engine — `quinn-ops ingest` gets its clap home, and the slot runs it

**The one `src/` change in this plan** (ruling R-P4a-7). Without it the wizard's LMS feed is collected and never fetched: `rank`'s steps are `passes, tasks, calendar, events, approvals` — Blackboard ingest is not among them — and today it runs only in the cloud routine (`python3 -m engine.ingest --vault . --via cloud-routine --run-id $RUN_ID`), which a friend does not have. The rewrite spec's own command table (line 166) promised `quinn-ops ingest`; `src/main.rs`'s `Command` enum has Rank/Surface/Coursework/Runs/Info/Issues/Write and no `Ingest`, while `src/ingest.rs` already holds the whole ported core.

**On Quinn's live vault this changes nothing until plan 2 Task 9 flips `scheduler: app`** — the app's scheduler is inert before that, and the cloud routine keeps running the Python ingest until plan 3. When both do run, the overlap is safe: `state/ingest-seen.md` records every uid ever ingested, so the second pass creates nothing. Say this in Task 11's HANDOFF line too.

**Files:**
- Read first: `engine/ingest.py:388–431` (`main`), `tests/test_ingest.py:302–359` (its four tests)
- Modify: `src/ingest.rs` (the orchestrator and its tests), `src/main.rs` (the `Ingest` arm), `app/src/scheduler.rs` (`slot_argv`, `has_ics_url`, the skip step), `app/tests/scheduler.rs`

**Interfaces:**
- Consumes: `ingest::{parse_ics, sync_tasks, Event}` (already ported); `calfeed::fetch_ics(&str) -> Result<String, String>`; `journal::Journal::new`; `write::WriteContext`; `pystr::read_text`; `tests/fixtures/blackboard.ics`.
- Produces:
  - `ingest::run_lines(vault: &Path, via: &str, run_id: Option<&str>, fetch: Option<&dyn Fn(&str) -> Result<String, String>>) -> (i32, Vec<String>)`
  - `ingest::run_with(vault, via, run_id, fetch) -> i32`; `ingest::run(vault, via, run_id) -> i32`
  - the `Ingest { vault, via, run_id }` clap command
  - `scheduler::{IcsState, ics_state(vault) -> IcsState, has_ics_url(vault) -> bool}`; `scheduler::slot_argv` now returns two or three steps

- [ ] **Step 1: Read the Python and write down what it does**

`engine/ingest.py:main` is small, and every branch is behaviour this port must keep:

| Python | Result |
|---|---|
| `config/ingest.yaml` missing or bad YAML | prints `ingest: config unreadable: <err>`, returns **1** |
| `ics_url` absent or blank after `.strip()` | prints `ingest: no ics_url configured`, returns **1** |
| `fetch_ics(url)` raises `OSError` | prints `ingest: fetch failed: <err>`, returns **1** |
| otherwise | `parse_ics(text, tz)` → `sync_tasks(events, vault, course_map, ctx=WriteContext("agent:ingest.blackboard", via, run_id))`, prints each log line, then `ingest: N events, M action(s)`, returns **0** |

`tz` is `ZoneInfo(config.get("timezone", "America/Chicago"))`. Flags: `--vault` (default `.`), `--via` (default `cli`, choices `VIAS`), `--run-id`. **There is no run record, no `runner-log.md` line and no `--dry-run`** — so the port writes none of the three either.

- [ ] **Step 2: Write the failing tests** (in `src/ingest.rs`'s existing `mod tests`, beside the ones that already read `tests/fixtures/blackboard.ics`)

```rust
    fn ingest_vault(tag: &str, config: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-ingest-main-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        if !config.is_empty() { pystr::write_text(&d.join("config").join("ingest.yaml"), config).unwrap(); }
        d
    }
    fn feed() -> impl Fn(&str) -> Result<String, String> {
        |_: &str| pystr::read_text(Path::new("tests/fixtures/blackboard.ics")).map_err(|e| e.to_string())
    }
    const ONE_COURSE: &str = "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\ncourse_map:\n  PH-106: ph-106\n  GN-103: gn-103\n  CS-100: cs-100\n";

    fn records_of(vault: &Path) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(vault.join("state").join("journal")).unwrap().flatten() {
            let text = pystr::read_text(&e.path()).unwrap();
            for line in text.split('\n').filter(|l| !l.trim().is_empty()) {
                out.push(serde_json::from_str(line).unwrap());
            }
        }
        out
    }

    /// Port of `tests/test_ingest.py:302` — bad YAML is "config unreadable", not "no ics_url", and
    /// the two branches must stay distinguishable by message as well as by exit code.
    #[test]
    fn ingest_main_returns_1_and_says_so_when_the_config_is_unreadable() {
        let v = ingest_vault("badyaml", "ics_url: [unclosed\n");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert!(lines[0].starts_with("ingest: config unreadable:"), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:359` — a missing file takes the same branch as a broken one.
    #[test]
    fn ingest_main_returns_1_when_the_config_file_is_missing() {
        let v = ingest_vault("nofile", "");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert!(lines[0].starts_with("ingest: config unreadable:"), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A vault with a config but no feed. Not one of Python's four, but it is the branch the app's
    /// `has_ics_url` skip exists for, so it is pinned on both sides of that decision.
    #[test]
    fn ingest_main_returns_1_when_no_ics_url_is_configured() {
        let v = ingest_vault("nourl", "timezone: America/Chicago\nics_url: \"   \"\n");
        let (code, lines) = run_lines(&v, "cli", None, None);
        assert_eq!(code, 1);
        assert_eq!(lines, vec!["ingest: no ics_url configured".to_string()]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:312`: under a runner these writes are a RUN's writes. Default
    /// `via: cli, run_id: null` would make every ingested assignment unattributable to the run that
    /// fetched it, and indistinguishable from a hand invocation.
    #[test]
    fn ingest_main_journals_under_the_runners_via_and_run_id() {
        let v = ingest_vault("via", ONE_COURSE);
        let f = feed();
        let (code, lines) = run_lines(&v, "local-runner", Some("local-2026-08-29T17:00:00Z"), Some(&f));
        assert_eq!(code, 0);
        assert!(lines.last().unwrap().starts_with("ingest: "), "{lines:?}");
        let records = records_of(&v);
        assert!(!records.is_empty(), "ingest wrote notes but journalled nothing");
        for r in &records {
            assert_eq!(r["via"], "local-runner");
            assert_eq!(r["run_id"], "local-2026-08-29T17:00:00Z");
            assert_eq!(r["actor"], "agent:ingest.blackboard");
        }
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Port of `tests/test_ingest.py:341`, plus the happy path Python covers through its other 35
    /// tests: the fixture feed creates real task notes and the summary line counts them.
    #[test]
    fn ingest_main_defaults_to_cli_via_and_creates_tasks_from_the_feed() {
        let v = ingest_vault("happy", ONE_COURSE);
        let f = feed();
        let (code, lines) = run_lines(&v, "cli", None, Some(&f));
        assert_eq!(code, 0);
        let records = records_of(&v);
        for r in &records { assert_eq!(r["via"], "cli"); assert!(r["run_id"].is_null()); }
        let created = std::fs::read_dir(v.join("tasks")).unwrap().flatten().count();
        assert!(created > 0, "the fixture feed created notes");
        assert!(lines.iter().any(|l| l.starts_with("created ")), "{lines:?}");
        assert!(v.join("state").join("ingest-seen.md").is_file(), "the seen ledger is written");
        // A second run over the same feed is idempotent: the ledger stops every uid.
        let (code2, lines2) = run_lines(&v, "cli", None, Some(&f));
        assert_eq!(code2, 0);
        assert!(!lines2.iter().any(|l| l.starts_with("created ")), "{lines2:?}");
        let _ = std::fs::remove_dir_all(&v);
    }
```

- [ ] **Step 3: Run them to verify they fail**

Run (repo root): `cargo test ingest::tests::ingest_main`
Expected: FAIL to compile — `cannot find function run_lines in this scope`.

- [ ] **Step 4: The orchestrator** (append to `src/ingest.rs`)

```rust
/// Ports `engine/ingest.py:main` (lines 388–431). Exit codes and printed lines are Python's: `1`
/// for an unreadable or missing config, `1` for an empty `ics_url`, `1` for a failed fetch, `0`
/// otherwise. **Python writes no run record and no `runner-log.md` line, and has no `--dry-run`** —
/// neither does this.
///
/// Two documented deviations, both from Python raising where Rust returns: an unknown `timezone`
/// is a `ZoneInfoNotFoundError` traceback in Python and a `1` with a message here, and an empty
/// config file is an `AttributeError` traceback there and `no ics_url configured` here. Both are
/// non-zero on both sides. The text after `config unreadable:` is each language's own parser
/// message — the same class of difference `scripts/diff-engines.ps1` masks for the transport
/// library, and this command is not under the oracle.
pub fn run(vault: &Path, via: &str, run_id: Option<&str>) -> i32 { run_with(vault, via, run_id, None) }

/// [`run`] with the network seam exposed, exactly as `cli::run_with` exposes rank's two. Production
/// passes `None`; tests pass a closure over `tests/fixtures/blackboard.ics`.
pub fn run_with(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> i32 {
    let (code, lines) = run_lines(vault, via, run_id, fetch);
    for line in &lines { println!("{line}"); }
    code
}

/// The testable core: everything `run_with` does, with the output returned instead of printed, in
/// the same order Python prints it. Python's tests assert on `capsys`; Rust has no such capture, so
/// the lines are a value.
pub fn run_lines(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
) -> (i32, Vec<String>) {
    let ctx = crate::write::WriteContext {
        actor: "agent:ingest.blackboard".to_string(),
        via: via.to_string(),
        run_id: run_id.map(str::to_string),
    };
    let path = vault.join("config").join("ingest.yaml");
    // NOT `yaml::mapping_from_file`: it folds a missing file and a broken one into an empty
    // mapping, and Python distinguishes both from "no ics_url" by message. Read and parse here so
    // every Python branch survives.
    let text = match crate::pystr::read_text(&path) {
        Ok(t) => t,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let url = config.get("ics_url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if url.is_empty() { return (1, vec!["ingest: no ics_url configured".to_string()]); }
    let tz_name = config.get("timezone").and_then(|v| v.as_str()).unwrap_or("America/Chicago").to_string();
    let tz = match jiff::tz::TimeZone::get(&tz_name) {
        Ok(tz) => tz,
        Err(e) => return (1, vec![format!("ingest: config unreadable: unknown timezone {tz_name}: {e}")]),
    };
    let fetched = match fetch { Some(f) => f(&url), None => crate::calfeed::fetch_ics(&url) };
    let feed = match fetched {
        Ok(t) => t,
        Err(e) => return (1, vec![format!("ingest: fetch failed: {e}")]),
    };
    let (events, malformed) = parse_ics(&feed, &tz);
    // `course_map` is a mapping of fragment -> slug, and `match_course` walks it IN ORDER —
    // `serde_yaml_ng::Mapping` preserves the file's order, as Python's dict preserves insertion.
    let course_map: Vec<(String, String)> = config
        .get("course_map")
        .and_then(|v| v.as_mapping())
        .map(|m| m.iter().filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    let mut journal = crate::journal::Journal::new(vault);
    let mut log = sync_tasks(&events, vault, &course_map, Some(&ctx), &mut journal, None);
    if malformed > 0 { log.push(format!("skipped {malformed} malformed event(s)")); }
    let summary = format!("ingest: {} events, {} action(s)", events.len(), log.len());
    log.push(summary);
    (0, log)
}
```

(`sync_tasks` already writes every note through `write` and every ledger line through `ingest::record_seen`; nothing new here writes a file directly, and the only JSON is the journal's, which goes through `ledger::dumps_value` inside `Journal::append`. `pystr::read_text`/`write_text` keep the CRLF rule on both ends.)

- [ ] **Step 5: The clap arm** (`src/main.rs`)

In `enum Command`, after `Coursework`:

```rust
    /// Sync the LMS .ics feed into tasks/. Ports `python -m engine.ingest`.
    Ingest {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Without these the runner's writes journal as `via: cli, run_id: null` --
        /// indistinguishable from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
    },
```

and in the dispatch, beside `Command::Coursework`:

```rust
        Command::Ingest { vault, via, run_id } => match ingest::run(&vault, &via, run_id.as_deref()) {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        },
```

(with `ingest` added to `main.rs`'s `use quinn_ops::{…}` list.)

- [ ] **Step 6: Run the engine tests and the parity gate**

Run (repo root, dev profile — `--release` will not link):

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
cargo test 2>&1 | tail -20
```

Expected: 5 new tests pass, **0 warnings**, `tests/oracle.rs` and `tests/surface_oracle.rs` green — `state/today.md` does not move by a byte, because nothing `rank` runs was touched.

Then the parity gate. These scripts do not cover `ingest`; they are here to prove `rank` and `write` did not move under a `src/` edit:

```powershell
scripts\diff-engines.ps1 -Vault tests\fixtures\vault-s1
scripts\diff-engines.ps1 -Vault tests\fixtures\vault-s1-migrated
scripts\diff-engines.ps1 -Vault tests\fixtures\vault-full
scripts\diff-engines-notes.ps1
```

Expected: each exits 0. **Do not proceed to step 7 on a non-zero exit** — a difference here means the edit reached something it should not have.

- [ ] **Step 7: The slot runs it** (`app/src/scheduler.rs`)

```rust
/// Why a slot will or will not run `ingest` (R-P4a-17). The two "no" cases are different problems
/// and get different words on the Runs view: a config that does not parse is something to fix, a
/// config with no feed is a friend who has not connected one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcsState { Feed, NoUrl, Unreadable }

/// The engine's `ingest` exits **1** on an empty `ics_url` (Python does, and the port keeps it),
/// and a slot step that exits non-zero sets `engine_ok = false` — which would put every friend with
/// no LMS feed into permanent retry backoff and an amber tray, twice a day, forever. So the APP
/// decides not to run the step; the engine's own behaviour is unchanged (ruling R-P4a-9).
///
/// A **missing** file is `NoUrl` (a vault without one has no feed, which is not a fault); a file
/// that does not parse is `Unreadable`.
pub fn ics_state(vault: &Path) -> IcsState {
    let path = vault.join("config").join("ingest.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else { return IcsState::NoUrl };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return IcsState::Unreadable };
    match v.get("ics_url").and_then(|u| u.as_str()) {
        Some(s) if !s.trim().is_empty() => IcsState::Feed,
        _ => IcsState::NoUrl,
    }
}

pub fn has_ics_url(vault: &Path) -> bool { ics_state(vault) == IcsState::Feed }

/// coursework → **ingest** → rank, as child processes of the sibling engine exe. `ingest` is
/// included only when the vault has a feed; the wizard's LMS URL (plan 4a Task 6, panel 4) is
/// fetched here and nowhere else.
pub fn slot_argv(vault: &Path, exe: &Path) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    let mut steps = vec![(exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
    if has_ics_url(vault) {
        steps.push((exe.to_path_buf(), vec!["ingest".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
    }
    steps.push((exe.to_path_buf(), vec!["rank".into(), "--vault".into(), v, "--runner".into(), "local".into()]));
    steps
}
```

and in `run_slot_inner`, immediately before `match engine_exe() {`:

```rust
    // A skipped step is still a step: without this line the Runs view and the sync line would show
    // a slot with no ingest in it and no reason why. `0` because a skip is not a failure — the same
    // shape `sync_step` uses for a busy lock. Recorded before the engine is resolved, so a missing
    // exe does not also hide the explanation. The two reasons read differently (R-P4a-17).
    match ics_state(&cs.vault) {
        IcsState::Feed => {}
        IcsState::NoUrl => steps.push(("ingest (skipped: no ics_url)".to_string(), 0)),
        IcsState::Unreadable => steps.push(("ingest (skipped: config unreadable)".to_string(), 0)),
    }
```

- [ ] **Step 8: The app tests** (`app/tests/scheduler.rs`; add `has_ics_url`, `ics_state`, `IcsState` to the `use` list)

Replace `the_slot_runs_exactly_the_two_engine_commands_the_script_ran` with:

```rust
#[test]
fn the_slot_runs_coursework_then_ingest_then_rank_and_leaves_ingest_out_without_a_feed() {
    // `tests/fixtures/vault-full`'s ingest.yaml deliberately has no `ics_url` (it is offline by
    // construction), which is exactly the friend-with-no-LMS case.
    let v = scratch("argv");
    let exe = Path::new(r"C:\bin\quinn-ops.exe");
    let names = |a: &Vec<(PathBuf, Vec<String>)>| a.iter().map(|(_, x)| x[0].clone()).collect::<Vec<_>>();
    assert!(!has_ics_url(&v));
    let argv = slot_argv(&v, exe);
    assert_eq!(names(&argv), vec!["coursework", "rank"]);
    assert_eq!(argv[0].1, vec!["coursework", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[1].1, vec!["rank", "--vault", v.to_string_lossy().as_ref(), "--runner", "local"]);

    let cfg = v.join("config").join("ingest.yaml");
    let old = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("ics_url: \"https://lms.example.invalid/learn.ics\"\n{old}")).unwrap();
    assert!(has_ics_url(&v));
    let argv = slot_argv(&v, exe);
    assert_eq!(names(&argv), vec!["coursework", "ingest", "rank"]);
    assert_eq!(argv[1].1, vec!["ingest", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert!(argv.iter().all(|(e, _)| e == exe));

    std::fs::write(&cfg, "ics_url: \"   \"\n").unwrap();
    assert!(!has_ics_url(&v), "a blank url is no url — the engine would exit 1 on it");
    // R-P4a-17: the three states are distinguishable, and a missing file is "no url", not a fault.
    assert_eq!(ics_state(&v), IcsState::NoUrl);
    std::fs::write(&cfg, "ics_url: [unclosed\n").unwrap();
    assert_eq!(ics_state(&v), IcsState::Unreadable);
    std::fs::remove_file(&cfg).unwrap();
    assert_eq!(ics_state(&v), IcsState::NoUrl);
}

/// R-P4a-9: the skip is visible. No engine exe sits beside the test binary, so the run also
/// records `engine: …` — which is the point: the explanation is recorded before the exe is
/// resolved, so a missing engine never hides it.
#[test]
fn a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list() {
    let v = scratch("skipstep");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", quinn_ops::journal::device_name()),
    ).unwrap();
    let cs = open(&v, "skipstep");
    let sch = Scheduler::default();
    let s = run_slot_inner(&cs, &sch, None, false);
    assert!(s.reason.is_none(), "not refused: {:?}", s.reason);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"ingest (skipped: no ics_url)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "ingest"), "the step itself never ran");
    assert_eq!(s.steps.iter().find(|(n, _)| n.starts_with("ingest (skipped")).unwrap().1, 0, "a skip is not a failure");
}
```

- [ ] **Step 9: Run the app tests**

Run (from `app/`): `cargo test` — green, zero new warnings.

- [ ] **Step 10: Commit, twice**

```bash
git add src/ingest.rs src/main.rs
```

Message: `engine: quinn-ops ingest — engine/ingest.py:main ported with a fetch seam, its four tests and a fixture-fed happy path (Knowlu plan 4a, Task 5)`.

```bash
git add app/src/scheduler.rs app/tests/scheduler.rs
```

Message: `app: the slot runs coursework, ingest and rank — ingest only when the vault has a feed, and the skip is a named step (Knowlu plan 4a, Task 5)`.

---

### Task 6: The wizard — seven panels and their commands

**Files:**
- Modify: `app/src/onboarding.rs` (four commands), `app/src/main.rs` (the shell's handler list), `app/static/index.html` (`#wizard`), `app/static/console.js` (the wizard), `app/static/console.css`, `app/tests/static_assets.rs`, `scripts/console-shots.py`

**Interfaces:**
- Consumes: `scaffold::{VaultPlan, create_vault, seed_writes, CAMPUSES}`; `credentials::{target_for, write}`; `profiles::{register, default_name, id_for}`; `onboarding::{relaunch_with, refresh}`; the page's `invoke`, `h`, `EL`; `renderPicker` and the `launch_state` boot branch from Task 2.
- Produces:
  - commands `create_vault(parent, name, plan)`, `restore_vault(backup, parent, name, plan)`, `apply_profile_settings(id, plan)`, `store_credentials(vault, source, user, secret)`, `finish_onboarding(id)`; the shared `dest_for(parent, name) -> Result<PathBuf, String>`
  - page: `startWizard(l)`, `renderWizard()`, `wizGo(n)`, `wizFinish()`, `dest()`, `window.KNOWLU_SHOTS`; the panel ids `wiz-welcome … wiz-finish`; the picker's `#pick-add`
  - `scripts/wizard-check.py` — the headless behaviour check Task 7 extends

- [ ] **Step 1: Write the failing test** (append to `app/tests/static_assets.rs`)

```rust
#[test]
fn the_wizard_has_seven_panels_the_privacy_words_and_no_live_fetch() {
    let html = read("index.html");
    assert!(html.contains("id=\"wizard\""));
    for p in ["wiz-welcome", "wiz-vault", "wiz-backup", "wiz-lms", "wiz-logins", "wiz-slots", "wiz-finish"] {
        assert!(html.contains(&format!("id=\"{p}\"")), "panel {p}");
    }
    let js = read("console.js");
    for f in ["startWizard", "renderWizard", "wizGo", "wizFinish"] {
        assert!(js.contains(&format!("function {f}(")), "missing {f}");
    }
    // Decision 10: no telemetry, and no toggle to argue about.
    assert!(!js.to_lowercase().contains("telemetry"));
    // The privacy paragraph, exactly, and the same words the site carries (Task 10 pins the pair).
    assert!(js.contains("Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates."));
    // No live fetch in onboarding: the ICS URL is matched by shape, never requested. The regex is
    // escaped so the page still carries no `https://` literal (the network-reference rule).
    assert!(js.contains("/^https:\\/\\/"), "the ICS check is an escaped regex");
    assert!(!js.contains("fetch(\"http"), "onboarding never fetches");
    // Credentials leave page memory the moment the write returns (spec §5).
    assert!(js.contains("clearCredentialFields("), "the fields are cleared by name");
    // S13: the one seam the headless checks drive the panels through.
    assert!(js.contains("window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker }"), "the shots seam, spelled as scripts/console-shots.py calls it");
    // R-P4a-11: a new vault is <parent>\<name>, and the page says where before Finish.
    assert!(js.contains("function dest()") && js.contains("wiz-name") && js.contains("wiz-pick-parent"));
    // R-T15b's equality still holds: wizard markup is keyed by data-panel, never data-id.
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run (from `app/`): `cargo test --test static_assets the_wizard_has_seven_panels`
Expected: FAIL at `id="wizard"`.

- [ ] **Step 3: The four commands** (append to `app/src/onboarding.rs`)

```rust
/// Everything the panels collected. A serde struct rather than a loose map, so a missing field is
/// a refusal at the boundary and not a default nobody chose.
#[derive(Debug, serde::Deserialize)]
pub struct WizardPlan {
    pub ics_url: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
    pub backup_dir: Option<String>,
    pub autostart: bool,
}

/// `<parent>\<name>`, validated once for both `create_vault` and `restore_vault` (R-P4a-11). The
/// folder picker can only return a folder that exists, so the vault's own folder is named here and
/// created by the caller — and a name that would escape its parent, or land on something already
/// there, is refused with a sentence the panel shows.
fn dest_for(parent: &str, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() { return Err("give the vault a name".to_string()); }
    if name.contains(['\\', '/', ':']) || name == "." || name == ".." {
        return Err(format!("{name:?} is not a folder name — no slashes or colons"));
    }
    let parent = PathBuf::from(parent.trim());
    if !parent.is_dir() { return Err(format!("{}: pick a folder that exists", parent.display())); }
    let dest = parent.join(name);
    if dest.exists() { return Err(format!("{} already exists — pick another name", dest.display())); }
    Ok(dest)
}

/// *Finish* for a new vault (decision 3 — this is the first moment anything reaches the vault's
/// folder). Scaffold → seed → register. The settings file is written for the profile too, so the
/// backup folder and the autostart choice survive the relaunch.
#[tauri::command(async)]
pub fn create_vault(app: tauri::AppHandle, parent: String, name: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let dest = match dest_for(&parent, &name) { Ok(d) => d, Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }) };
    let vp = crate::scaffold::VaultPlan {
        profile_id: profiles::id_for(&dest),
        ics_url: plan.ics_url.clone().filter(|u| !u.trim().is_empty()),
        timezone: plan.timezone.clone(),
        slots: plan.slots.clone(),
        device: quinn_ops::journal::device_name(),
        campus: plan.campus.clone(),
        zybooks: plan.zybooks,
        vhl: plan.vhl,
    };
    if let Err(e) = crate::scaffold::create_vault(&dest, &vp) { return json!({ "ok": false, "error": e, "profile": Value::Null }); }
    if let Err(e) = crate::scaffold::seed_writes(&dest, &vp) { return json!({ "ok": false, "error": e, "profile": Value::Null }); }
    finish_profile(&app, &root, &dest, Some(name), &plan)
}

/// *Restore from a backup* (spec §3 panel 2): `<backup>\<profile>\vault\` is **copied** to
/// `<parent>\<name>` and used there — never used in place, so the backup folder is never written
/// to and never becomes the live vault by accident.
/// The handle-free core, so `app/tests/onboarding.rs` can hash the BACKUP folder either side of a
/// restore and prove it is only ever read (spec §8; S7). Returns the created vault path on success.
pub fn restore_vault_in(backup: &str, parent: &str, name: &str) -> Result<PathBuf, String> {
    let src = PathBuf::from(backup);
    let mirror = find_mirror(&src).ok_or_else(|| format!("{backup}: no <profile>\\vault\\ mirror in there"))?;
    let dest = dest_for(parent, name)?;
    let staging = dest.parent().unwrap_or(Path::new(".")).join(format!(".knowlu-restore-{}", quinn_ops::ids::new_id("stage")));
    match copy_tree(&mirror, &staging).and_then(|()| std::fs::rename(&staging, &dest).map_err(|e| e.to_string())) {
        Ok(()) => Ok(dest),
        Err(e) => { let _ = std::fs::remove_dir_all(&staging); Err(e) }
    }
}

#[tauri::command(async)]
pub fn restore_vault(app: tauri::AppHandle, backup: String, parent: String, name: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    match restore_vault_in(&backup, &parent, &name) {
        Ok(dest) => finish_profile(&app, &root, &dest, Some(name), &plan),
        Err(e) => json!({ "ok": false, "error": e, "profile": Value::Null }),
    }
}

/// S3: an ADOPTED vault is registered by `adopt_vault` (Task 2, which the picker also uses and
/// which has no wizard answers to apply). This is the wizard's second half of that path — panel
/// 3's backup folder and panel 6's autostart choice, written into the profile's settings file so
/// they survive the relaunch exactly as a created vault's do.
#[tauri::command(async)]
pub fn apply_profile_settings(app: tauri::AppHandle, id: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding" }) };
    let settings = crate::state::Settings {
        profile_id: id.clone(),
        backup_dir: plan.backup_dir.clone().filter(|b| !b.is_empty()).map(PathBuf::from),
        autostart: plan.autostart,
        quit_at: None,
    };
    match settings.save(&profiles::profile_dir(&root, &id).join("settings.json")) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// `<backup>` is either the profile folder itself or the folder holding profile folders.
fn find_mirror(root: &Path) -> Option<PathBuf> {
    if root.join("vault").join("tasks").is_dir() { return Some(root.join("vault")); }
    std::fs::read_dir(root).ok()?.flatten()
        .map(|e| e.path().join("vault"))
        .find(|p| p.join("tasks").is_dir())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let (p, t) = (e.path(), to.join(e.file_name()));
        if p.is_dir() { copy_tree(&p, &t)?; } else { std::fs::copy(&p, &t).map(|_| ()).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

/// Register the profile and write its settings file, so the backup folder and the autostart choice
/// are already in place when the relaunched console reads them.
fn finish_profile(app: &tauri::AppHandle, root: &Path, vault: &Path, name: Option<String>, plan: &WizardPlan) -> Value {
    let label = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| profiles::default_name(vault));
    let p = match profiles::register(root, &label, vault) { Ok(p) => p, Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }) };
    let settings = crate::state::Settings {
        profile_id: p.id.clone(),
        backup_dir: plan.backup_dir.clone().filter(|b| !b.is_empty()).map(PathBuf::from),
        autostart: plan.autostart,
        quit_at: None,
    };
    let _ = settings.save(&profiles::profile_dir(root, &p.id).join("settings.json"));
    refresh(app, root);
    json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).unwrap_or(Value::Null) })
}

/// Panel 5's write, made the moment the user leaves the panel (decision 3) — the vault folder need
/// not exist yet: the target is derived from the path the user chose, not from anything on disk.
/// **The secret is never returned, never logged and never echoed back to the page.**
#[tauri::command(async)]
pub fn store_credentials(vault: String, source: String, user: String, secret: String) -> Value {
    if !["zybooks", "vhl"].contains(&source.as_str()) { return json!({ "ok": false, "error": format!("unknown source {source}") }); }
    let target = crate::credentials::target_for(&profiles::id_for(Path::new(&vault)), &source);
    match crate::credentials::write(&target, &user, &secret) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "target": target }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The last click: relaunch into the console over the new profile (see `open_profile`).
#[tauri::command(async)]
pub fn finish_onboarding(app: tauri::AppHandle, id: String) -> Value { open_profile(app, id) }
```

`main.rs`'s `run_shell` handler list becomes:

```rust
        .invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::finish_onboarding])
```

- [ ] **Step 4: The panels in `index.html`**, directly after `#picker`:

```html
<section class="wiz" id="wizard" hidden>
  <div class="wiz-hd"><h1>Knowlu</h1><span class="n" id="wiz-step"></span></div>
  <div class="wiz-panel" id="wiz-welcome" hidden><h2>Welcome</h2><p id="wiz-privacy"></p><p class="lede">Knowlu answers one question every morning: what should I work on today, and in what order?</p></div>
  <div class="wiz-panel" id="wiz-vault" hidden><h2>Your vault</h2>
    <label><input type="radio" name="vmode" value="create" checked> Create a new vault</label>
    <label><input type="radio" name="vmode" value="adopt"> Use an existing vault</label>
    <label><input type="radio" name="vmode" value="restore"> Restore from a backup</label>
    <div class="wiz-row" id="wiz-new-row"><button class="b" id="wiz-pick-parent">Choose where…</button><input type="text" id="wiz-name" placeholder="Vault name"></div>
    <div class="wiz-row" id="wiz-adopt-row" hidden><button class="b" id="wiz-pick-vault">Choose the vault folder…</button></div>
    <div class="wiz-row" id="wiz-restore-row" hidden><button class="b" id="wiz-pick-backup">Choose the backup folder…</button><span class="meta" id="wiz-backup-src"></span></div>
    <p class="meta" id="wiz-vault-path"></p>
  </div>
  <div class="wiz-panel" id="wiz-backup" hidden><h2>Backup folder</h2>
    <p class="lede">A folder you own — OneDrive, an external drive, a share. If the drive is unplugged Knowlu says so and carries on.</p>
    <div class="wiz-row"><button class="b" id="wiz-pick-bdir">Choose folder…</button><span class="meta" id="wiz-bdir"></span><button class="b" id="wiz-skip-backup">Skip</button></div>
    <p class="meta" id="wiz-backup-note"></p>
  </div>
  <div class="wiz-panel" id="wiz-lms" hidden><h2>Your LMS calendar</h2>
    <p class="lede">Blackboard: Calendar &rarr; Calendar Settings &rarr; Share Calendar &rarr; copy the link. Canvas: Calendar &rarr; Calendar Feed &rarr; copy the link.</p>
    <input type="text" id="wiz-ics" placeholder="Paste the feed link (it ends in .ics)">
    <p class="meta" id="wiz-ics-note"></p>
  </div>
  <div class="wiz-panel" id="wiz-logins" hidden><h2>Coursework logins</h2>
    <p class="lede">Optional. Stored in Windows Credential Manager on this machine — never in the vault, never in a backup.</p>
    <div class="wiz-row"><input type="text" id="wiz-zy-user" placeholder="zyBooks email"><input type="password" id="wiz-zy-pass" placeholder="zyBooks password"></div>
    <div class="wiz-row"><input type="text" id="wiz-vhl-user" placeholder="VHL email"><input type="password" id="wiz-vhl-pass" placeholder="VHL password"></div>
  </div>
  <div class="wiz-panel" id="wiz-slots" hidden><h2>Slots and campus</h2>
    <div class="wiz-row"><input type="text" id="wiz-tz" placeholder="Timezone"><input type="text" id="wiz-slot1" placeholder="12:00"><input type="text" id="wiz-slot2" placeholder="18:00"></div>
    <label><input type="checkbox" id="wiz-autostart" checked> Start Knowlu with Windows</label>
    <div class="wiz-row" id="wiz-campus"></div>
  </div>
  <div class="wiz-panel" id="wiz-finish" hidden><h2>Ready</h2><p class="lede" id="wiz-summary"></p>
    <p class="meta">Nothing is fetched now. Your first slot runs on the clock.</p>
    <p class="meta">Gmail proposals, event verdicts and enrichment are not here yet — they arrive in a later release.</p></div>
  <div class="wiz-nav"><span class="crit" id="wiz-error"></span><button class="b" id="wiz-back">Back</button><button class="b pri y" id="wiz-next">Next</button></div>
</section>
```

- [ ] **Step 5: The wizard in `console.js`** (after `renderPicker`)

```js
  // ---- Plan 4a Task 6: onboarding (spec §3). Seven panels in this same document — no second
  // webview, no bundler. NOTHING reaches disk until Finish, except credentials, written the
  // moment the user leaves panel 5 (decision 3).
  var PANELS = ["welcome", "vault", "backup", "lms", "logins", "slots", "finish"];
  var PRIVACY = "Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates.";
  // Escaped on purpose: the shipped page carries no bare network literal (console spec §7).
  var ICS_OK = /^https:\/\/\S+(\.ics($|\?)|\/calendar\/)/i;
  // `parent` + `name` make the new vault's path (R-P4a-11); `vault` is the folder an ADOPT picks
  // directly. `dest()` is the one place the two shapes become a path, and it is what the panel
  // shows before Finish so nobody discovers where their vault went afterwards.
  var WIZ = { step: 0, mode: "create", parent: "", name: "Vault", vault: "", backupSrc: "", bdir: "", ics: "", tz: "", slots: ["12:00", "18:00"], autostart: true, campus: "none", zy: false, vhl: false, campuses: [], error: "" };
  function dest() { return WIZ.mode === "adopt" ? WIZ.vault : (WIZ.parent && WIZ.name ? WIZ.parent + "\\" + WIZ.name : ""); }

  function startWizard(l) {
    EL("wizard").hidden = false;
    EL("picker").hidden = true;
    document.querySelector(".app").hidden = true;
    WIZ.tz = l.tz || "";
    WIZ.campuses = l.campuses || [];
    // The default parent is the engine's, never built in the page (spec §3 panel 2). The name is a
    // plain default the user overwrites; it becomes the vault folder's name AND the profile label,
    // which the settings panel can rename later.
    WIZ.parent = l.documents || "";
    EL("wiz-name").value = WIZ.name;
    EL("wiz-privacy").textContent = PRIVACY;
    EL("wiz-tz").value = WIZ.tz;
    EL("wiz-slot1").value = WIZ.slots[0];
    EL("wiz-slot2").value = WIZ.slots[1];
    EL("wiz-campus").innerHTML = WIZ.campuses.map(function (c, i) {
      return '<label><input type="radio" name="campus" value="' + h(c.key) + '"' + (i === 0 ? " checked" : "") + "> " + h(c.label) + "</label>";
    }).join("");
    renderWizard();
  }

  function renderWizard() {
    PANELS.forEach(function (p, i) { EL("wiz-" + p).hidden = i !== WIZ.step; });
    EL("wiz-step").textContent = "step " + (WIZ.step + 1) + " of " + PANELS.length;
    EL("wiz-back").disabled = WIZ.step === 0;
    EL("wiz-next").textContent = WIZ.step === PANELS.length - 1 ? "Finish" : "Next";
    EL("wiz-error").textContent = WIZ.error;
    EL("wiz-vault-path").textContent = dest() ? (WIZ.mode === "adopt" ? "Using " : "Your vault will be at ") + dest() : "no folder chosen yet";
    EL("wiz-backup-src").textContent = WIZ.backupSrc || "no backup chosen";
    EL("wiz-new-row").hidden = WIZ.mode === "adopt";
    EL("wiz-adopt-row").hidden = WIZ.mode !== "adopt";
    EL("wiz-restore-row").hidden = WIZ.mode !== "restore";
    EL("wiz-bdir").textContent = WIZ.bdir || "not set";
    EL("wiz-backup-note").textContent = WIZ.bdir ? "" : "Without a backup folder there is no mirror and no snapshot to restore from.";
    EL("wiz-ics-note").textContent = WIZ.ics && !ICS_OK.test(WIZ.ics) ? "That does not look like a calendar feed link." : "";
    EL("wiz-summary").textContent = (WIZ.mode === "create" ? "A new vault at " : WIZ.mode === "adopt" ? "Your existing vault at " : "A vault restored to ") +
      dest() + ", slots at " + WIZ.slots.join(" and ") + " " + WIZ.tz + (WIZ.bdir ? ", backed up to " + WIZ.bdir : ", no backup folder") + ".";
  }

  // Panel 5 leaves: write whatever was typed straight into Credential Manager, then clear the
  // inputs — a crash must never leave a password in page memory longer than it has to be.
  function clearCredentialFields() {
    ["wiz-zy-pass", "wiz-vhl-pass", "wiz-zy-user", "wiz-vhl-user"].forEach(function (id) { EL(id).value = ""; });
  }
  function storeCredentials() {
    var jobs = [];
    [["zybooks", "wiz-zy-user", "wiz-zy-pass"], ["vhl", "wiz-vhl-user", "wiz-vhl-pass"]].forEach(function (t) {
      var u = EL(t[1]).value.trim(), s = EL(t[2]).value;
      if (!u || !s) { return; }
      if (t[0] === "zybooks") { WIZ.zy = true; } else { WIZ.vhl = true; }
      jobs.push(invoke("store_credentials", { vault: WIZ.vault, source: t[0], user: u, secret: s }));
    });
    return Promise.all(jobs).then(function (rs) {
      clearCredentialFields();
      var bad = rs.filter(function (r) { return r && !r.ok; });
      if (bad.length) { WIZ.error = bad[0].error; return false; }
      return true;
    }).catch(function () { clearCredentialFields(); WIZ.error = "the credential could not be stored"; return false; });
  }

  function wizValid() {
    WIZ.error = "";
    if (WIZ.step === 1) {
      if (WIZ.mode === "adopt" && !WIZ.vault) { WIZ.error = "Choose the vault folder first."; }
      if (WIZ.mode !== "adopt" && !WIZ.parent) { WIZ.error = "Choose where the vault should go."; }
      if (WIZ.mode !== "adopt" && !WIZ.name.trim()) { WIZ.error = "Give the vault a name."; }
      if (WIZ.mode !== "adopt" && /[\\/:]/.test(WIZ.name)) { WIZ.error = "A vault name has no slashes or colons."; }
      if (WIZ.mode === "restore" && !WIZ.backupSrc) { WIZ.error = "Choose the backup folder too."; }
    }
    if (WIZ.step === 3 && WIZ.ics && !ICS_OK.test(WIZ.ics)) { WIZ.error = "That does not look like a calendar feed link."; }
    return !WIZ.error;
  }

  function wizGo(n) {
    if (n > WIZ.step && !wizValid()) { renderWizard(); return Promise.resolve(); }
    var leaving = WIZ.step;
    WIZ.step = Math.max(0, Math.min(PANELS.length - 1, n));
    // Adopting or restoring skips panels 4-6: the vault already carries its own config (spec §3).
    if (WIZ.mode !== "create" && WIZ.step >= 3 && WIZ.step <= 5 && n > leaving) { WIZ.step = 6; }
    if (WIZ.mode !== "create" && WIZ.step >= 3 && WIZ.step <= 5 && n < leaving) { WIZ.step = 2; }
    if (leaving === 4 && n > leaving) { return storeCredentials().then(function () { renderWizard(); }); }
    renderWizard();
    return Promise.resolve();
  }

  function wizFinish() {
    WIZ.ics = EL("wiz-ics").value.trim();
    WIZ.tz = EL("wiz-tz").value.trim() || WIZ.tz;
    WIZ.slots = [EL("wiz-slot1").value.trim() || "12:00", EL("wiz-slot2").value.trim() || "18:00"];
    WIZ.autostart = EL("wiz-autostart").checked;
    var picked = document.querySelector('input[name="campus"]:checked');
    WIZ.campus = picked ? picked.value : "none";
    var plan = { ics_url: WIZ.ics || null, timezone: WIZ.tz, slots: WIZ.slots, campus: WIZ.campus, zybooks: WIZ.zy, vhl: WIZ.vhl, backup_dir: WIZ.bdir || null, autostart: WIZ.autostart };
    EL("wiz-next").disabled = true;
    var call = WIZ.mode === "create" ? invoke("create_vault", { parent: WIZ.parent, name: WIZ.name, plan: plan })
      : WIZ.mode === "restore" ? invoke("restore_vault", { backup: WIZ.backupSrc, parent: WIZ.parent, name: WIZ.name, plan: plan })
      // S3: adopting registers the profile (Task 2's command, which the picker shares), then the
      // wizard's own answers — backup folder, autostart — are written for that profile.
      : invoke("adopt_vault", { path: WIZ.vault, name: null }).then(function (r) {
          if (!r.ok) { return r; }
          return invoke("apply_profile_settings", { id: r.profile.id, plan: plan }).then(function () { return r; });
        });
    return call.then(function (r) {
      if (!r.ok) { WIZ.error = r.error; EL("wiz-next").disabled = false; renderWizard(); return; }
      return invoke("finish_onboarding", { id: r.profile.id });
    }).catch(function (e) { WIZ.error = String(e.message || e); EL("wiz-next").disabled = false; renderWizard(); });
  }

  EL("wizard").addEventListener("click", function (e) {
    var r = e.target.closest('input[name="vmode"]'); if (r) { WIZ.mode = r.value; renderWizard(); return; }
    if (e.target.closest("#wiz-back")) { wizGo(WIZ.step - 1); return; }
    if (e.target.closest("#wiz-next")) { if (WIZ.step === PANELS.length - 1) { wizFinish(); } else { wizGo(WIZ.step + 1); } return; }
    if (e.target.closest("#wiz-skip-backup")) { WIZ.bdir = ""; wizGo(WIZ.step + 1); return; }
    var titles = { vault: "Choose the vault folder", parent: "Choose where the vault should go", backupSrc: "Choose the backup to restore from", bdir: "Choose a backup folder" };
    var which = e.target.closest("#wiz-pick-vault") ? "vault"
      : e.target.closest("#wiz-pick-parent") ? "parent"
      : e.target.closest("#wiz-pick-backup") ? "backupSrc"
      : e.target.closest("#wiz-pick-bdir") ? "bdir" : null;
    if (which) {
      invoke("pick_folder", { title: titles[which] })
        .then(function (p) { if (p && p.path) { WIZ[which] = p.path; renderWizard(); } }).catch(function () {});
    }
  });
  EL("wiz-name").addEventListener("input", function () { WIZ.name = EL("wiz-name").value; renderWizard(); });
  EL("wiz-ics").addEventListener("input", function () { WIZ.ics = EL("wiz-ics").value.trim(); renderWizard(); });
```

and the boot branch from Task 2 gains one line, before `renderPicker`:

```js
    if (l && l.mode === "wizard") { startWizard(l); return; }
```

**R-P4a-15, the picker's other door.** `index.html`'s `#picker` gains one button beside *Use an existing vault…* — it exists only now, because only now is there a wizard to open:

```html
      <button class="b pri y" id="pick-add">Add another…</button>
```

and one handler beside the picker's others (the shell already carries every wizard command, so nothing is relaunched):

```js
  EL("pick-add").addEventListener("click", function () {
    invoke("launch_state", {}).then(function (l) { startWizard(l || {}); }).catch(function () {});
  });
```

`startWizard` hides `#picker` on the way in (see its first lines), so *Add another…* from a picker with two profiles lands on panel 1 and Finish relaunches into the new one.

`launch_state` gains the campus list so the page never hard-codes one:

```rust
            "campuses": crate::scaffold::CAMPUSES.iter().map(|(k, l)| json!({ "key": k, "label": l })).collect::<Vec<_>>(),
```

- [ ] **Step 6: CSS**

```css
.wiz { max-width: 640px; margin: var(--s7) auto; padding: 0 var(--s5); }
.wiz-hd { display: flex; align-items: baseline; justify-content: space-between; }
.wiz-panel h2 { font-size: 17px; margin: var(--s5) 0 var(--s2); }
.wiz-panel label { display: block; padding: var(--s1) 0; color: var(--t2); }
.wiz-row { display: flex; gap: var(--s2); align-items: center; flex-wrap: wrap; margin: var(--s3) 0; }
.wiz-panel input[type="text"], .wiz-panel input[type="password"] { background: var(--s2c); border: 1px solid var(--hair-2); border-radius: var(--r1); color: var(--t1); padding: 6px 8px; font: inherit; min-width: 22ch; }
.wiz-panel .meta { color: var(--t4); font: 500 11px/1.5 var(--mono); }
.wiz .crit { color: var(--crit); margin-right: auto; }
```

- [ ] **Step 7: Headless check**

The panels live inside the IIFE and are not hash routes, so they need one seam. At the end of `console.js`'s IIFE, beside the other window assignments:

```js
  // The one seam the headless checks use (S13). Nothing in the app calls these; they exist so a
  // Playwright page can put the wizard or the picker on screen without a Tauri backend.
  window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker };
```

and in `scripts/console-shots.py`, after the existing per-view loop:

```python
                    for panel, js in [("wizard", "KNOWLU_SHOTS.startWizard({tz:'America/Chicago',documents:'C:/Users/x/Documents/Knowlu',campuses:[{key:'none',label:'None'}]})"),
                                      ("picker", "KNOWLU_SHOTS.renderPicker({profiles:[{id:'p1',name:'Ada',vault:'C:/v'}]})")]:
                        page.evaluate(js)
                        page.wait_for_timeout(200)
                        page.screenshot(path=str(out / f"{w}-{panel}.png"), full_page=True)
                        page.reload(); page.wait_for_timeout(600)
```

Run: `.wv/Scripts/python scripts/console-shots.py tests/fixtures/surface-today-full.json shots/`
Expected: every viewport ≥ 820 `ok`; look at `1280-wizard.png` and confirm nothing overflows.

- [ ] **Step 7b: Headless *behaviour*, not only pictures** (S7)

Screenshots do not catch a Next button that skips a panel. Write `scripts/wizard-check.py` beside the shots script — the same temp-dir/loopback serving, a **fake `window.__TAURI__.core.invoke`** installed before the page's script runs, and five assertions:

```python
"""Headless behaviour check for the wizard and the settings overlay (plan 4a, Tasks 6-7).

Serves a copy of app/static/ from a temp dir over loopback (never the repo — ruling R40) and
installs a fake __TAURI__ that records every invoke. No Tauri, no vault, no network.
"""
import http.server, json, shutil, socketserver, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[1]
FAKE = """
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'wizard', profiles: [], machine: 'M',
      tz: 'America/Chicago', documents: 'C:\\\\Docs\\\\Knowlu', campuses: [{ key: 'none', label: 'None' }] }); }
  if (cmd === 'pick_folder') { return Promise.resolve({ ok: true, path: 'C:\\\\Docs\\\\Knowlu' }); }
  if (cmd === 'store_credentials') { return Promise.resolve({ ok: true, error: null, target: 'knowlu/p/zybooks' }); }
  if (cmd === 'get_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: null, autostart: true, quit_at: null } }); }
  if (cmd === 'settings_context') { return Promise.resolve({ ok: true, vault: 'C:\\\\v', version: '0.1.0', profile_name: 'Ada' }); }
  if (cmd === 'set_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: 'C:\\\\b', autostart: true, quit_at: null } }); }
  return Promise.resolve({ ok: true, error: null });
} } };
window.__CALLS = [];
"""

def main() -> int:
    tmp = Path(tempfile.mkdtemp(prefix="knowlu-wizard-check-"))
    try:
        shutil.copytree(REPO / "app" / "static", tmp / "app" / "static")
        handler = lambda *a, **k: http.server.SimpleHTTPRequestHandler(*a, directory=str(tmp), **k)
        with socketserver.TCPServer(("127.0.0.1", 0), handler) as srv:
            threading.Thread(target=srv.serve_forever, daemon=True).start()
            url = f"http://127.0.0.1:{srv.server_address[1]}/app/static/index.html"
            with sync_playwright() as p:
                page = p.chromium.launch().new_context(viewport={"width": 1280, "height": 900}).new_page()
                page.add_init_script(FAKE)
                page.goto(url); page.wait_for_timeout(400)
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
                if "Fall 2026" not in page.inner_text("#wiz-vault-path"): bad.append("the resulting path is not shown")
                # 3. The ICS shape is validated, and a bad one blocks Next.
                page.click("#wiz-next"); page.click("#wiz-skip-backup"); page.wait_for_timeout(150)
                page.fill("#wiz-ics", "not-a-url"); page.wait_for_timeout(120)
                page.click("#wiz-next"); page.wait_for_timeout(120)
                if page.is_hidden("#wiz-lms"): bad.append("a malformed ICS url did not block Next")
                page.fill("#wiz-ics", "https://lms.example.invalid/x/learn.ics"); page.wait_for_timeout(120)
                # 4. Leaving the credentials panel writes them and CLEARS the fields.
                page.click("#wiz-next"); page.wait_for_timeout(150)
                page.fill("#wiz-zy-user", "a@example.invalid"); page.fill("#wiz-zy-pass", "not-a-real-password")
                page.click("#wiz-next"); page.wait_for_timeout(300)
                calls = page.evaluate("window.__CALLS.map(c => c[0])")
                if "store_credentials" not in calls: bad.append("store_credentials was not invoked")
                if page.input_value("#wiz-zy-pass") != "": bad.append("the password field was not cleared")
                for line in bad: print("FAIL:", line)
                print("ok" if not bad else f"{len(bad)} failure(s)")
                return 1 if bad else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

if __name__ == "__main__":
    sys.exit(main())
```

Run: `.wv/Scripts/python scripts/wizard-check.py`
Expected: `ok`, exit 0. (Task 7 extends the same script with the settings overlay's two assertions.)

- [ ] **Step 7c: The two "never wrote there" tests** (`app/tests/onboarding.rs`, new — spec §8, S7)

```rust
use quinn_ops_console::onboarding::{adopt_vault_in, restore_vault_in};
use std::path::{Path, PathBuf};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-onb-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (p, t) = (e.path(), to.join(e.file_name()));
        if p.is_dir() { copy(&p, &t); } else { std::fs::copy(&p, &t).unwrap(); }
    }
}
/// Every file under `root`, as (vault-relative path, bytes), sorted — a whole-tree fingerprint.
/// Content, not mtimes: a copy that rewrote a file byte-identically is not a write worth failing.
fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); }
            else { out.push((p.strip_prefix(root).unwrap().to_string_lossy().to_string(), std::fs::read(&p).unwrap())); }
        }
    }
    out.sort();
    out
}

/// Spec §8: adopting `tests/fixtures/vault-s1` as a profile **writes nothing into it** — no
/// scaffold, no seed note, no first task, not even a journal record.
#[test]
fn adopting_a_vault_writes_nothing_into_it() {
    let root = tmp("adopt");
    let vault = root.join("vault-s1");
    copy(Path::new("../tests/fixtures/vault-s1"), &vault);
    let before = fingerprint(&vault);
    let out = adopt_vault_in(&root.join("appdata"), vault.to_str().unwrap(), None);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(fingerprint(&vault), before, "the adopted vault was modified");
    // …and a folder that is not a vault is refused by name, before anything is registered.
    let bad = adopt_vault_in(&root.join("appdata"), root.to_str().unwrap(), None);
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("config/ and tasks/"), "{bad}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Spec §8: restore COPIES the mirror out and never writes into the backup folder — the one place
/// a user's only other copy of their vault lives.
#[test]
fn restoring_copies_the_mirror_and_never_writes_into_the_backup() {
    let root = tmp("restore");
    let backup = root.join("backup").join("profile_1111111111").join("vault");
    copy(Path::new("../tests/fixtures/vault-s1"), &backup);
    let before = fingerprint(&root.join("backup"));
    let parent = root.join("restored-into");
    std::fs::create_dir_all(&parent).unwrap();
    let dest = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026").expect("restore");
    assert_eq!(dest, parent.join("Fall 2026"));
    assert_eq!(fingerprint(&root.join("backup")), before, "the backup folder was written to");
    assert_eq!(fingerprint(&dest), before.iter().map(|(p, b)| (p.trim_start_matches("profile_1111111111\\").trim_start_matches("vault\\").to_string(), b.clone())).collect::<Vec<_>>(), "the mirror arrived whole");
    // A second restore to the same name is refused, and still writes nothing to the backup.
    let again = restore_vault_in(root.join("backup").to_str().unwrap(), parent.to_str().unwrap(), "Fall 2026");
    assert!(again.unwrap_err().contains("already exists"));
    assert_eq!(fingerprint(&root.join("backup")), before);
    let strays: Vec<_> = std::fs::read_dir(&parent).unwrap().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".knowlu-restore-")).collect();
    assert!(strays.is_empty(), "a refused restore leaves no staging folder");
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 8: Run the tests**

Run (from `app/`): `cargo test` — all green, zero new warnings; `onboarding` (2 tests) and `static_assets` pass. Root `cargo test` unchanged.

- [ ] **Step 9: Commit**

```bash
git add app/src/onboarding.rs app/src/main.rs app/static/index.html app/static/console.js app/static/console.css app/tests/onboarding.rs app/tests/static_assets.rs scripts/console-shots.py scripts/wizard-check.py
```

Message: `app: onboarding — seven panels from nothing to a working vault, credentials at panel five, nothing else written until Finish (Knowlu plan 4a, Task 6)`.

---

### Task 7: The settings panel, the topline gear, the tray item

**Files:**
- Modify: `app/src/commands.rs` (`set_profile_name`, `copy_diagnostics`), `app/src/main.rs` (handler list), `app/src/tray.rs` (a `Settings` item), `app/static/index.html` (`#settings`), `app/static/console.js`, `app/static/console.css`, `app/tests/commands.rs`, `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `get_settings` / `set_settings` (plan 1, unchanged); `onboarding::pick_folder` and `onboarding::launch_state` (registered in the console app by Task 2 — **not re-registered here**, S1); `backup_now`; `profiles::{register, load, default_name}`; `tray::diagnostics_text`; `main.rs`'s `--pick` flag (Task 2).
- Produces: commands `set_profile_name(name)`, `set_profile_name_in(root, vault, name)`, `copy_diagnostics()`, `copy_text(text)`, `settings_context()` (returns `{vault, version, profile_name}`), `switch_profile()`; page: `openSettings()`, `renderSettings(s)`, `window.KNOWLU_OPEN_SETTINGS`; the row ids `set-name`, `set-vault`, `set-backup`, `set-autostart`, `set-updates` (disabled here), `set-diag`.

> Spec §4's table lists six rows while decision 9 says five. Both are honoured by building the five
> the spec's table backs with a command **plus** Diagnostics here, and adding the **Updates** row in
> Task 8 where the command it needs is written. The panel ends the plan with the table's six rows.

- [ ] **Step 1: Write the failing tests**

`app/tests/commands.rs`:

```rust
#[test]
fn set_profile_name_renames_the_registry_entry_and_refuses_an_empty_name() {
    let v = scratch("profname");
    let root = std::env::temp_dir().join(format!("knowlu-setname-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    quinn_ops_console::profiles::register(&root, "Before", &v).unwrap();
    let out = quinn_ops_console::commands::set_profile_name_in(&root, &v, "After").unwrap();
    assert_eq!(out["ok"], true);
    assert_eq!(quinn_ops_console::profiles::load(&root)[0].name, "After");
    let bad = quinn_ops_console::commands::set_profile_name_in(&root, &v, "   ").unwrap();
    assert_eq!(bad["ok"], false);
    assert_eq!(quinn_ops_console::profiles::load(&root)[0].name, "After", "a refusal changes nothing");
    let _ = std::fs::remove_dir_all(&root);
}
```

`app/tests/static_assets.rs`:

```rust
#[test]
fn the_settings_panel_has_its_rows_and_one_way_in() {
    let html = read("index.html");
    assert!(html.contains("id=\"settings\""));
    for row in ["set-name", "set-vault", "set-backup", "set-autostart", "set-diag"] {
        assert!(html.contains(&format!("id=\"{row}\"")), "row {row}");
    }
    let js = read("console.js");
    assert!(js.contains("function openSettings(") && js.contains("function renderSettings("));
    assert!(js.contains("window.KNOWLU_OPEN_SETTINGS"), "the tray's one way in");
    assert!(js.contains("data-settings"), "the topline gear");
    assert!(js.contains("invoke(\"switch_profile\""), "R-P4a-15: back to the picker");
    assert!(js.contains("c.profile_name"), "S4: the profile label comes from settings_context, not a guess");
    assert!(!js.to_lowercase().contains("channel"), "M8: there is one channel, so the row does not name one");
    // Decision 9: slots, timezone, sources and the course map stay the wizard's.
    for absent in ["set-slots", "set-timezone", "set-sources", "set-coursemap"] {
        assert!(!html.contains(absent), "{absent} is not a settings row in 4a");
    }
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (from `app/`): `cargo test --test commands set_profile_name` then `cargo test --test static_assets the_settings_panel`
Expected: FAIL — `set_profile_name_in` undefined; `id="settings"` missing.

- [ ] **Step 3: The two commands** (append to `app/src/commands.rs`)

```rust
/// The testable core: which registry, which vault, which name. The `#[tauri::command]` twin only
/// supplies the root and the vault from the running state.
pub fn set_profile_name_in(root: &std::path::Path, vault: &std::path::Path, name: &str) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() { return Ok(json!({ "ok": false, "error": "a profile needs a name", "profile": Value::Null })); }
    match crate::profiles::register(root, name, vault) {
        Ok(p) => Ok(json!({ "ok": true, "error": Value::Null, "profile": serde_json::to_value(p).map_err(|e| e.to_string())? })),
        Err(e) => Ok(json!({ "ok": false, "error": e, "profile": Value::Null })),
    }
}

#[tauri::command(async)]
pub fn set_profile_name(cs: State<'_, ConsoleState>, name: String) -> Value {
    let Some(root) = crate::state::app_data_root() else { return json!({ "ok": false, "error": "no app data root", "profile": Value::Null }) };
    set_profile_name_in(&root, &cs.vault, &name).unwrap_or_else(|e| json!({ "ok": false, "error": e, "profile": Value::Null }))
}

/// The tray's *Copy diagnostics*, reachable from the settings panel too — the same text, through
/// the same clipboard call, so there is one diagnostics blob and not two (spec §4). The text is
/// built in Rust and never passes through the page.
#[tauri::command(async)]
pub fn copy_diagnostics(app: tauri::AppHandle, cs: State<'_, ConsoleState>) -> Value {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let text = crate::tray::diagnostics_text(&cs);
    match app.clipboard().write_text(text) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// The vault-path row's Copy button. Separate from `copy_diagnostics` on purpose: this one copies
/// what the page already shows, that one copies text the page never sees.
#[tauri::command(async)]
pub fn copy_text(app: tauri::AppHandle, text: String) -> Value {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    match app.clipboard().write_text(text) { Ok(()) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e.to_string() }) }
}

/// What the panel needs that `Settings` deliberately does not carry: the vault path is `--vault`,
/// not a setting; the version is the binary's; and the profile NAME lives in `profiles.json`, not
/// in `settings.json` (S4) — so the panel reads all three from here and never guesses.
#[tauri::command]
pub fn settings_context(cs: State<'_, ConsoleState>) -> Value {
    let id = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    let name = crate::state::app_data_root()
        .map(|r| crate::profiles::load(&r))
        .and_then(|ps| ps.into_iter().find(|p| p.id == id).map(|p| p.name))
        .unwrap_or_else(|| crate::profiles::default_name(&cs.vault));
    json!({ "ok": true, "error": Value::Null, "vault": cs.vault.to_string_lossy(), "version": env!("CARGO_PKG_VERSION"), "profile_name": name })
}

/// *Switch profile…* (R-P4a-15): relaunch into the picker and quit. `--pick` forces it even with
/// one profile registered, and `--after-pid` keeps `tauri-plugin-single-instance` from killing the
/// window that is arriving instead of the one that is leaving.
#[tauri::command(async)]
pub fn switch_profile(app: tauri::AppHandle) -> Value {
    let exe = match std::env::current_exe() { Ok(e) => e, Err(e) => return json!({ "ok": false, "error": e.to_string() }) };
    match std::process::Command::new(exe).arg("--pick").arg("--after-pid").arg(std::process::id().to_string()).spawn() {
        Ok(_) => { app.exit(0); json!({ "ok": true, "error": Value::Null }) }
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}
```

Append **only** `commands::set_profile_name, commands::copy_diagnostics, commands::copy_text, commands::settings_context, commands::switch_profile` to the console app's `generate_handler!` (S1: `onboarding::launch_state` and `onboarding::pick_folder` are already in that list from Task 2 — a duplicate arm is a `warning: unused` at best and breaks the zero-warnings gate).

- [ ] **Step 4: The tray item**

In `tray::build`, add the item and its arm:

```rust
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
```

…included in `Menu::with_items(app, &[&open, &run, &pause, &settings, &diag, &quit])?`, and:

```rust
            "settings" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    // One way into the panel, from both entry points: the page owns the panel and
                    // the tray only asks. `eval` rather than an event keeps the capability set as
                    // it is (no `core:event` grant needed for a menu click).
                    let _ = w.eval("window.KNOWLU_OPEN_SETTINGS && window.KNOWLU_OPEN_SETTINGS()");
                }
            }
```

- [ ] **Step 5: The panel in `index.html`**, after `#wizard`:

```html
<aside class="setpanel" id="settings" hidden>
  <div class="set-hd"><h2>Settings</h2><button class="b" id="set-close">Close</button></div>
  <div class="set-row" id="set-name"><span class="k">Profile name</span><input type="text" id="set-name-in"><button class="b" id="set-name-save">Save</button></div>
  <div class="set-row" id="set-vault"><span class="k">Vault</span><span class="meta" id="set-vault-path"></span><button class="b" id="set-vault-copy">Copy</button><button class="b" id="set-switch">Switch profile…</button></div>
  <div class="set-row" id="set-backup"><span class="k">Backup folder</span><span class="meta" id="set-bdir"></span><button class="b" id="set-bdir-pick">Choose…</button><button class="b" id="set-backup-now">Back up now</button></div>
  <div class="set-row" id="set-autostart"><span class="k">Start with Windows</span><input type="checkbox" id="set-autostart-in"></div>
  <div class="set-row" id="set-updates"><span class="k">Updates</span><span class="meta" id="set-update-state"></span><button class="b" id="set-update-check" disabled>Check now</button></div>
  <div class="set-row" id="set-diag"><span class="k">Diagnostics</span><button class="b" id="set-diag-copy">Copy diagnostics</button><span class="meta" id="set-diag-note"></span></div>
</aside>
```

(R-P4a-5: the `set-updates` row's markup lands here **disabled**, reading `updates arrive with the next build` — there is no `check_for_updates` command until Task 8, and a live-looking button with nothing behind it is worse than an honest one. Task 8 enables it and fills the text. M8: the row shows the version, the last check time, `ready`/`up to date`, and the last error — **no "channel"**: there is one channel and the spec amendment at Task 11 says so.)

- [ ] **Step 6: The panel in `console.js`**

```js
  // ---- Plan 4a Task 7: the settings panel (spec §4). An overlay, not a view: a view name would
  // reach `surface::View::parse` on every poll and be refused. Two ways in, one function —
  // the topline gear and the tray's Settings item, which calls KNOWLU_OPEN_SETTINGS.
  function renderSettings(s) {
    EL("set-name-in").value = (current.profileName || "");
    EL("set-vault-path").textContent = current.vaultPath || "";
    EL("set-bdir").textContent = s.backup_dir || "not set";
    EL("set-autostart-in").checked = !!s.autostart;
    // Task 8 fills current.updateText; until then the row says so and its button stays disabled.
    EL("set-update-state").textContent = current.updateText || ("version " + (current.version || "") + " — updates arrive with the next build");
    EL("set-diag-note").textContent = "";
  }
  function openSettings() {
    EL("settings").hidden = false;
    invoke("settings_context", {}).then(function (c) {
      current.vaultPath = c.vault; current.version = c.version; current.profileName = c.profile_name;
      return invoke("get_settings", {});
    }).then(function (r) { if (r.ok) { renderSettings(r.settings); } }).catch(function () {});
  }
  window.KNOWLU_OPEN_SETTINGS = openSettings;

  EL("settings").addEventListener("click", function (e) {
    if (e.target.closest("#set-close")) { EL("settings").hidden = true; return; }
    if (e.target.closest("#set-name-save")) {
      invoke("set_profile_name", { name: EL("set-name-in").value }).then(function (r) {
        EL("set-diag-note").textContent = r.ok ? "saved" : r.error;
        if (r.ok) { current.profileName = r.profile.name; }
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#set-vault-copy")) { invoke("copy_text", { text: current.vaultPath || "" }).catch(function () {}); return; }
    // R-P4a-15: back to the picker. The window goes with it — this process is the one relaunching.
    if (e.target.closest("#set-switch")) { invoke("switch_profile", {}).catch(function () {}); return; }
    if (e.target.closest("#set-bdir-pick")) {
      invoke("pick_folder", { title: "Choose a backup folder" }).then(function (p) {
        if (!p || !p.path) { return; }
        return invoke("set_settings", { patch: { backup_dir: p.path } }).then(function (r) { if (r.ok) { renderSettings(r.settings); } });
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#set-backup-now")) { invoke("backup_now", { view: current.view }).then(applyEnvelope).catch(function () {}); return; }
    if (e.target.closest("#set-diag-copy")) {
      invoke("copy_diagnostics", {}).then(function (r) { EL("set-diag-note").textContent = r.ok ? "copied" : r.error; }).catch(function () {});
      return;
    }
  });
  EL("set-autostart-in").addEventListener("change", function () {
    invoke("set_settings", { patch: { autostart: EL("set-autostart-in").checked } })
      .then(function (r) { if (r.ok) { renderSettings(r.settings); } else { EL("set-diag-note").textContent = r.error; } }).catch(function () {});
  });
```

`renderTopline` gains one part, last:

```js
    parts.push('<button class="b" type="button" data-settings title="Settings">&#9881;</button>');
```

and the document-level click handler gains one line, before the `[data-field]` branch:

```js
    var gear = e.target.closest("[data-settings]"); if (gear) { openSettings(); return; }
```

- [ ] **Step 7: CSS**

```css
.setpanel { position: fixed; right: var(--s4); top: var(--s4); width: 420px; max-width: calc(100vw - 32px); background: var(--s2c); border: 1px solid var(--hair-2); border-radius: var(--r3); padding: var(--s4); z-index: 20; }
.set-hd { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: var(--s3); }
.set-row { display: flex; gap: var(--s2); align-items: center; flex-wrap: wrap; padding: var(--s2) 0; border-top: 1px solid var(--hair); }
.set-row .k { min-width: 13ch; color: var(--t3); }
.set-row .meta { color: var(--t4); font: 500 11px/1.4 var(--mono); flex: 1 1 12ch; overflow-wrap: anywhere; }
.set-row input[type="text"] { background: var(--s3c); border: 1px solid var(--hair-2); border-radius: var(--r1); color: var(--t1); padding: 4px 6px; font: inherit; flex: 1 1 14ch; }
```

- [ ] **Step 8: Run the tests and look at it**

Run (from `app/`): `cargo test` — green, zero new warnings.

Add `openSettings: openSettings` to `window.KNOWLU_SHOTS` (the seam Task 6 created) and one more entry to `console-shots.py`'s panel loop, `("settings", "KNOWLU_SHOTS.openSettings()")`. Run the shots: no viewport ≥ 820 overflows, and the panel does not cover the topline's own buttons.

Then extend `scripts/wizard-check.py` (Task 6) with the settings overlay's own behaviour, after its wizard block — the fake `__TAURI__` already answers `settings_context`, `get_settings` and `set_settings` (S7):

```python
                # 5. The settings overlay: it opens, it shows what the engine said, and Save sends
                #    exactly the patch the row edited — no extra keys, no guessed values.
                page.evaluate("KNOWLU_SHOTS.openSettings()")
                page.wait_for_timeout(250)
                if page.is_hidden("#settings"): bad.append("the settings overlay did not open")
                if page.input_value("#set-name-in") != "Ada": bad.append("the profile name did not come from settings_context")
                if "C:\\v" not in page.inner_text("#set-vault-path"): bad.append("the vault path is not shown")
                page.evaluate("window.__CALLS = []")
                page.click("#set-bdir-pick"); page.wait_for_timeout(250)
                patches = [c[1] for c in page.evaluate("window.__CALLS") if c[0] == "set_settings"]
                if patches != [{"patch": {"backup_dir": "C:\\Docs\\Knowlu"}}]: bad.append(f"set_settings payload: {patches}")
                if not page.is_disabled("#set-update-check"): bad.append("the Updates row must stay disabled until Task 8")
```

Run: `.wv/Scripts/python scripts/wizard-check.py` — `ok`, exit 0.

- [ ] **Step 9: Commit**

```bash
git add app/src/commands.rs app/src/main.rs app/src/tray.rs app/static/index.html app/static/console.js app/static/console.css app/tests/commands.rs app/tests/static_assets.rs scripts/console-shots.py scripts/wizard-check.py
```

Message: `app: the settings panel — name, vault, backup folder, autostart, diagnostics, from the gear and the tray (Knowlu plan 4a, Task 7)`.

---
### Task 8: The updater — check, stage, never mid-run, offer

**Two halves inside one task, and the order is a correctness rule, not a preference (R-P4a-13).**
`plugins.updater.pubkey` is a **required** field — `tauri-plugin-updater` 2.11.0 deserializes its
config with no `#[serde(default)]` on it (`config.rs:137`) and the plugin's `setup` runs at
startup — so a commit that registers the plugin without the config block is a commit in which
**Knowlu does not start**. Steps 1–7 therefore land and commit everything that needs no plugin (the model, the
mid-run gate, the install hold, the commands answering `updater not configured`, the offer's
rendering, every test), and **step 9 is the only step that adds the dependency, the registration,
the config block and the real check/install — committed together or not at all**, gated on Quinn's
public key from step 8.

**Files:**
- Create: `app/src/updates.rs`, `app/tests/updates.rs`
- Modify: `app/src/lib.rs`, `app/src/main.rs`, `app/src/scheduler.rs` (the daily check in housekeeping), `app/src/tray.rs`, `app/src/commands.rs`, `app/static/console.js`, `app/static/index.html`, `app/tests/static_assets.rs`; **step 10 only:** `app/Cargo.toml`, `app/capabilities/default.json`, `app/tauri.conf.json`

**Interfaces:**
- Consumes: `scheduler::{Scheduler, lock, RunGuard-equivalent}` (`running` is both the mid-run gate and, during an install, the thing an install holds); `profiles::updates_dir(root)`; `state::app_data_root()`.
- Produces:
  - `updates::Staged { version: String, path: PathBuf }`
  - `updates::Updates { staged: Mutex<Option<Staged>>, last_check: Mutex<Option<String>>, last_error: Mutex<Option<String>>, item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>> }`
  - `updates::update_offer(running: bool, staged: Option<&Staged>) -> Option<String>`
  - `updates::record_check(u: &Updates, result: Result<Option<Staged>, String>, now: &str)`
  - `updates::hold_for_install(sch: &Scheduler) -> Option<InstallHold>` and `updates::InstallHold` (clears `Scheduler.running` on drop)
  - **step 10 only:** `updates::check_and_stage(app, updates_dir).await`, `updates::install_staged(app, staged).await`, and their `_blocking` twins for the two std threads
  - commands `check_for_updates()`, `install_update()` (both `async fn`); page: `renderUpdateOffer(u)`

- [ ] **Step 1: Write the failing tests** (`app/tests/updates.rs`)

```rust
use quinn_ops_console::updates::{record_check, update_offer, Staged, Updates};
use std::path::PathBuf;

fn staged(v: &str) -> Staged { Staged { version: v.to_string(), path: PathBuf::from("C:\\stage.bundle") } }

/// R9 (Knowlu spec decision 16): **never mid-run.** A staged update while a slot is running is not
/// offered — not dimmed, not queued behind a dialog: not offered at all, until the slot ends.
#[test]
fn a_staged_update_is_never_offered_while_a_slot_is_running() {
    let s = staged("0.2.0");
    assert_eq!(update_offer(false, Some(&s)).as_deref(), Some("0.2.0"));
    assert_eq!(update_offer(true, Some(&s)), None, "a slot is running — no offer");
    assert_eq!(update_offer(false, None), None, "nothing staged, nothing offered");
    assert_eq!(update_offer(true, None), None);
}

/// An unreachable endpoint is the NORMAL state until the site exists: one quiet record, no dialog,
/// no offer, and the next check is not blocked by it.
#[test]
fn an_unreachable_endpoint_is_recorded_quietly_and_offers_nothing() {
    let u = Updates::default();
    record_check(&u, Err("error sending request for url".to_string()), "2026-09-05T12:00:00Z");
    assert!(u.staged.lock().unwrap().is_none());
    assert_eq!(u.last_check.lock().unwrap().as_deref(), Some("2026-09-05T12:00:00Z"));
    assert!(u.last_error.lock().unwrap().as_deref().unwrap().contains("error sending request"));
    assert_eq!(update_offer(false, u.staged.lock().unwrap().as_ref()), None);

    // A later success clears the error and stages the version.
    record_check(&u, Ok(Some(staged("0.2.0"))), "2026-09-06T12:00:00Z");
    assert!(u.last_error.lock().unwrap().is_none(), "a good check clears the last error");
    assert_eq!(update_offer(false, u.staged.lock().unwrap().as_ref()).as_deref(), Some("0.2.0"));

    // "No update available" is a success that stages nothing and must not un-stage silently…
    record_check(&u, Ok(None), "2026-09-07T12:00:00Z");
    assert!(u.staged.lock().unwrap().is_none(), "…except that a manifest without it means it is gone");
}

/// S11: an install is exactly as exclusive as a slot. `hold_for_install` takes `Scheduler.running`
/// for the whole install, so a tick that comes due mid-install finds the flag set and stands down —
/// and the hold is released on drop, including on the error path.
#[test]
fn an_install_holds_the_slot_flag_and_gives_it_back() {
    use quinn_ops_console::scheduler::{lock, Scheduler};
    use quinn_ops_console::updates::hold_for_install;
    let sch = Scheduler::default();
    assert!(!*lock(&sch.running));
    {
        let hold = hold_for_install(&sch).expect("a free scheduler grants the hold");
        assert!(*lock(&sch.running), "a slot cannot start while an install is running");
        assert!(hold_for_install(&sch).is_none(), "and neither can a second install");
        assert_eq!(update_offer(*lock(&sch.running), Some(&staged("0.2.0"))), None, "nor is a second offer made");
        drop(hold);
    }
    assert!(!*lock(&sch.running), "the flag is given back");
}
```

And in `app/tests/static_assets.rs`:

```rust
#[test]
fn the_update_offer_is_in_the_page_and_the_endpoint_is_not() {
    let js = read("console.js");
    assert!(js.contains("function renderUpdateOffer("));
    assert!(js.contains("Restart to update"), "the exact offer text");
    assert!(js.contains("invoke(\"check_for_updates\"") && js.contains("invoke(\"install_update\""));
    // The endpoint lives in app/tauri.conf.json, never in the page (spec §8, the page rules).
    for f in ["index.html", "console.css", "console.js"] {
        assert!(!read(f).contains("knowlu.com"), "{f} must not name the release host");
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (from `app/`): `cargo test --test updates`
Expected: FAIL to compile — `unresolved import quinn_ops_console::updates`.

- [ ] **Step 3: `app/src/updates.rs`**

```rust
//! The updater (plan 4a, Task 8; spec §6, Knowlu spec decisions 14–16). Three rules, each a
//! function a test can hold: **never mid-run** (`update_offer`), **a failed check is quiet**
//! (`record_check` — the endpoint is unreachable until the site exists, which is correct, not a
//! bug), and **a failed signature is refused and shown** (`install_staged`).
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::AppHandle;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Staged { pub version: String, pub path: PathBuf }

#[derive(Default)]
pub struct Updates {
    pub staged: Mutex<Option<Staged>>,
    pub last_check: Mutex<Option<String>>,
    pub last_error: Mutex<Option<String>>,
    /// The tray's *Restart to update* item, stashed by `tray::build` so the menu can be enabled in
    /// place rather than rebuilt (the same pattern as `Scheduler.pause_item`).
    pub item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

/// **Never mid-run** (Knowlu spec decision 16). `running` is `Scheduler.running`: a slot that is
/// part-way through coursework or rank must not have its engine replaced under it.
pub fn update_offer(running: bool, staged: Option<&Staged>) -> Option<String> {
    if running { return None; }
    staged.map(|s| s.version.clone())
}

/// Fold one check's outcome into the state. A failure keeps the previous staged update (a network
/// blip does not un-stage a bundle already on disk); a success that offers nothing means the
/// manifest no longer carries it, so the stage is cleared.
pub fn record_check(u: &Updates, result: Result<Option<Staged>, String>, now: &str) {
    *u.last_check.lock().unwrap_or_else(|e| e.into_inner()) = Some(now.to_string());
    match result {
        Ok(s) => {
            *u.last_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
            *u.staged.lock().unwrap_or_else(|e| e.into_inner()) = s;
        }
        Err(e) => { *u.last_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(e); }
    }
}

/// S11: an install is as exclusive as a slot. Taking `Scheduler.running` for the duration means a
/// tick that comes due mid-install stands down on its own existing "already running" branch —
/// there is no second flag and no new race. `None` when a slot (or another install) already holds
/// it, which the caller reports rather than queues.
pub struct InstallHold<'a>(&'a crate::scheduler::Scheduler);
impl Drop for InstallHold<'_> {
    fn drop(&mut self) { *crate::scheduler::lock(&self.0.running) = false; }
}
pub fn hold_for_install(sch: &crate::scheduler::Scheduler) -> Option<InstallHold<'_>> {
    let mut r = crate::scheduler::lock(&sch.running);
    if *r { return None; }
    *r = true;
    drop(r);
    Some(InstallHold(sch))
}

/// Check the manifest and, if there is one, download the bundle into `updates_dir`.
///
/// **`async`, and the two `_blocking` twins below exist for the two std threads** (B4): a
/// `#[tauri::command]` on an `async fn` already runs on the tokio runtime, and
/// `tauri::async_runtime::block_on` inside it panics — "cannot start a runtime from within a
/// runtime". The commands `.await` this; the tray arm and the housekeeping tick, which are plain
/// `std::thread::spawn`s with no runtime under them, call the blocking twin.
///
/// **Until step 10 this is the not-configured stub.** The plugin is not a dependency yet, and a
/// registration without `plugins.updater` would panic at startup (R-P4a-13) — so every caller gets
/// the same quiet `Err` the tests already pin for an unreachable endpoint, and no call site
/// changes when the real body arrives.
pub async fn check_and_stage(_app: &AppHandle, _updates_dir: &Path) -> Result<Option<Staged>, String> {
    Err("updater not configured".to_string())
}

/// Install what was staged. **A bad signature returns `Err` here and is shown, never retried
/// silently** (Knowlu spec §6). Stubbed until step 10, exactly like `check_and_stage`.
pub async fn install_staged(_app: &AppHandle, _staged: &Staged) -> Result<(), String> {
    Err("updater not configured".to_string())
}

/// For callers with no async runtime under them — the tray menu arm and the housekeeping thread.
/// Never call these from inside a command: that is the panic B4 is about.
pub fn check_and_stage_blocking(app: &AppHandle, updates_dir: &Path) -> Result<Option<Staged>, String> {
    tauri::async_runtime::block_on(check_and_stage(app, updates_dir))
}
pub fn install_staged_blocking(app: &AppHandle, staged: &Staged) -> Result<(), String> {
    tauri::async_runtime::block_on(install_staged(app, staged))
}
```

Add `pub mod updates;` to `app/src/lib.rs`.

- [ ] **Step 4: The two commands, the tray item, the daily check**

`app/src/commands.rs`:

```rust
/// `{ok, error, version, staged, last_check, last_error}` — the page renders it; nothing here
/// decides anything but the mid-run gate, which lives in `updates::update_offer`.
///
/// **`async fn`, deliberately** (B4): the body must `.await` the plugin's own async check, and
/// `tauri::async_runtime::block_on` inside a command panics, because a command is already running
/// on that runtime. `#[tauri::command]` on an `async fn` IS the async form — `(async)` is only for
/// a synchronous body that needs moving off the webview thread.
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle, sch: State<'_, Scheduler>, up: State<'_, crate::updates::Updates>) -> Result<Value, ()> {
    let dir = crate::state::app_data_root().map(|r| crate::profiles::updates_dir(&r)).unwrap_or_else(|| std::env::temp_dir().join("knowlu-updates"));
    let result = crate::updates::check_and_stage(&app, &dir).await;
    let err = result.as_ref().err().cloned();
    crate::updates::record_check(&up, result, &quinn_ops::journal::now_ts(None));
    let running = *crate::scheduler::lock(&sch.running);
    let offer = crate::updates::update_offer(running, crate::scheduler::lock(&up.staged).as_ref());
    if let Some(item) = crate::scheduler::lock(&up.item).as_ref() { let _ = item.set_enabled(offer.is_some()); }
    Ok(json!({
        "ok": err.is_none(), "error": err,
        "version": env!("CARGO_PKG_VERSION"),
        "staged": offer,
        "last_check": crate::scheduler::lock(&up.last_check).clone(),
        "last_error": crate::scheduler::lock(&up.last_error).clone(),
    }))
}

/// *Restart to update.* Refused outright while a slot is running (R9), and it **takes** the slot
/// flag for the whole install so a slot cannot start under it (S11) — the caller is told why in
/// both cases.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, sch: State<'_, Scheduler>, up: State<'_, crate::updates::Updates>) -> Result<Value, ()> {
    let staged = crate::scheduler::lock(&up.staged).clone();
    let Some(s) = staged else { return Ok(json!({ "ok": false, "error": "nothing staged" })) };
    let Some(_hold) = crate::updates::hold_for_install(&sch) else {
        return Ok(json!({ "ok": false, "error": "a slot is running — the update will be offered when it finishes" }));
    };
    Ok(match crate::updates::install_staged(&app, &s).await {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    })
}
```

(A command that borrows `State<'_, _>` across an `.await` must return `Result<_, _>`; the error half is never used, so it is `()`, and the page's `invoke` resolves with the `Ok` value exactly as for every other command.)

Both names go into the console app's `generate_handler!`.

`app/src/tray.rs` — one more item, disabled until something is staged:

```rust
    let update = MenuItem::with_id(app, "update", "Restart to update", false, None::<&str>)?;
```

in `Menu::with_items(app, &[&open, &run, &pause, &settings, &update, &diag, &quit])?`, stashed beside the pause item:

```rust
    if let Some(up) = app.try_state::<crate::updates::Updates>() {
        *scheduler::lock(&up.item) = Some(update.clone());
    }
```

and the arm:

```rust
            "update" => {
                // A plain std thread with no runtime under it, so the BLOCKING twin (B4). It also
                // takes the same install hold the command does (S11).
                let h = app.clone();
                std::thread::spawn(move || {
                    let sch = h.state::<Scheduler>();
                    let up = h.state::<crate::updates::Updates>();
                    let Some(s) = scheduler::lock(&up.staged).clone() else { return };
                    let Some(_hold) = crate::updates::hold_for_install(&sch) else { return };
                    let _ = crate::updates::install_staged_blocking(&h, &s);
                });
            }
```

`app/src/scheduler.rs`'s housekeeping loop, beside the other cadences (10 s per tick, so a day is 8640):

```rust
            // Once a day while resident, and once at launch (n == 1) — Knowlu spec §6. A failure
            // is one recorded line and nothing else: until the site exists the endpoint is
            // unreachable, and that is the expected state, not an incident.
            if n == 1 || n % 8640 == 0 {
                if let Some(up) = house.try_state::<crate::updates::Updates>() {
                    let dir = state::app_data_root().map(|r| crate::profiles::updates_dir(&r)).unwrap_or_else(|| std::env::temp_dir().join("knowlu-updates"));
                    // The blocking twin: this is a std thread, not a command (B4).
                    let r = crate::updates::check_and_stage_blocking(&house, &dir);
                    crate::updates::record_check(&up, r, &quinn_ops::journal::now_ts(None));
                    let offer = crate::updates::update_offer(*lock(&sch.running), lock(&up.staged).as_ref());
                    if let Some(item) = lock(&up.item).as_ref() { let _ = item.set_enabled(offer.is_some()); }
                }
            }
```

`app/src/main.rs`: `app.manage(updates::Updates::default());` in the console builder's `setup`, before `tray::build`. **No plugin registration here** — that is step 10, with its config (R-P4a-13).

- [ ] **Step 5: The offer in the page**

`index.html` — one span in the topline row, before `#syncline`:

```html
    <div class="upd" id="upd" hidden></div>
```

`console.js`:

```js
  // Plan 4a Task 8: the offer. Rendered only when the engine says there is one — the mid-run
  // gate is `updates::update_offer` in Rust, never a check here (R9).
  function renderUpdateOffer(u) {
    var el = EL("upd");
    current.version = u.version;
    // M8: version, last check, staged-or-not, error. There is one channel, so the row names none.
    var when = u.last_check ? ", checked " + u.last_check.slice(0, 16).replace("T", " ") : "";
    current.updateText = "version " + u.version + (u.staged ? " — " + u.staged + " ready" : u.last_error ? " — " + u.last_error.split("\n")[0] : " — up to date") + when;
    el.hidden = !u.staged;
    if (u.staged) {
      el.innerHTML = '<span class="amber">Knowlu ' + h(u.staged) + ' is ready.</span> <button class="b pri y" type="button" data-install>Restart to update</button>';
    }
  }
  function checkForUpdates() {
    return invoke("check_for_updates", {}).then(function (u) { renderUpdateOffer(u); if (!EL("settings").hidden) { EL("set-update-state").textContent = current.updateText; } }).catch(function () {});
  }
```

The document click handler gains one branch, beside the gear:

```js
    var ins = e.target.closest("[data-install]");
    if (ins) { ins.disabled = true; invoke("install_update", {}).then(function (r) { if (!r.ok) { ins.disabled = false; showRefusal(null, r.error); } }).catch(function () { ins.disabled = false; }); return; }
```

`bootConsole()` calls `checkForUpdates()` once after `route(...)`; the settings panel's `#set-update-check` button calls it too, and Task 7 left that button `disabled` — remove the attribute here, in the same commit that gives it something to call:

```js
    if (e.target.closest("#set-update-check")) { checkForUpdates(); return; }
```

- [ ] **Step 6: Run the first half**

Run (from `app/`): `cargo test` — `updates` (3 tests) and `static_assets` green, zero new warnings. Then `cargo build --release` in `app/` and launch against a scratch vault:

```powershell
scripts\scratch-vault.ps1 -Source <path to the main checkout>
app\target\release\knowlu.exe --vault "<the printed scratch path>"
```

Expected: the window opens (this is the half that must still start), no update banner, and the settings Updates row reads `version 0.1.0 — updater not configured, checked <now>`. Quit from the tray.

- [ ] **Step 7: Commit the first half**

```bash
git add app/src/updates.rs app/src/lib.rs app/src/main.rs app/src/scheduler.rs app/src/tray.rs app/src/commands.rs app/static/index.html app/static/console.js app/tests/updates.rs app/tests/static_assets.rs
```

Message: `app: the update model — the mid-run gate, the install hold, a quiet failed check, and the offer's rendering (Knowlu plan 4a, Task 8)`.

- [ ] **Step 8: Quinn's moment — the keypair (spec §11)**

Ask Quinn, once, in these words:

> *"Run `cargo tauri signer generate -w %USERPROFILE%\\.knowlu-updater.key` in a terminal that is not this session. Paste me the PUBLIC key it prints (the `pubkey` line, not the file). Then put the private half into Credential Manager — `cmdkey /generic:knowlu/updater-key /user:knowlu /pass` (it prompts) — and delete `%USERPROFILE%\\.knowlu-updater.key` and its `.pub`. Reply when `cmdkey /list` shows `knowlu/updater-key`."*

**Step 9 does not start without the public key**, and the task ends here if it does not come — the first half is committed, shipped and green, and the gated half waits. That is the whole point of the split.

- [ ] **Step 9: The gated step — the plugin, its config and the real bodies, in ONE commit** (R-P4a-13)

`plugins.updater.pubkey` is a required field (`tauri-plugin-updater` 2.11.0, `config.rs:137`) and the plugin's `setup` runs at startup, so **registering the plugin in one commit and configuring it in another leaves a commit where Knowlu does not launch.** These five edits land together:

1. `app/Cargo.toml`: `tauri-plugin-updater = "2"` under `[dependencies]`.
2. `app/capabilities/default.json`: `"updater:default"` appended to `permissions` —
   `["core:default", "window-state:default", "autostart:default", "clipboard-manager:allow-write-text", "dialog:default", "updater:default"]`.
3. `app/tauri.conf.json`, at the top level:

```json
  "plugins": {
    "updater": {
      "endpoints": ["https://knowlu.com/releases/latest.json"],
      "pubkey": "<the public key from step 8, verbatim, one line>"
    }
  }
```

4. `app/src/main.rs`: `.plugin(tauri_plugin_updater::Builder::new().build())` on the **console** builder only — the vault-less shell has no tray, no slot to defer to and no window that should offer an update.
5. `app/src/updates.rs`: the two stub bodies become the real ones. Nothing else changes — every call site already spells these names:

```rust
pub async fn check_and_stage(app: &AppHandle, updates_dir: &Path) -> Result<Option<Staged>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else { return Ok(None) };
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    std::fs::create_dir_all(updates_dir).map_err(|e| e.to_string())?;
    let path = updates_dir.join(format!("knowlu-{}.bundle", update.version));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(Some(Staged { version: update.version.clone(), path }))
}

/// The manifest is re-checked so the plugin hands back a fresh `Update` to verify THESE bytes
/// against — a bad signature is an `Err` the page shows, never a silent retry (Knowlu spec §6).
pub async fn install_staged(app: &AppHandle, staged: &Staged) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let bytes = std::fs::read(&staged.path).map_err(|e| e.to_string())?;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?
        .ok_or_else(|| "the update is no longer offered".to_string())?;
    update.install(bytes).map_err(|e| e.to_string())
}
```

And one assertion joins `app/tests/static_assets.rs` in this same commit — it is here, not in Task 10, because until now there is no `plugins.updater` to assert on (S6):

```rust
#[test]
fn the_updater_endpoint_lives_in_the_app_config() {
    let conf: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    let ep = conf["plugins"]["updater"]["endpoints"][0].as_str().unwrap_or("");
    assert!(ep.ends_with("/releases/latest.json"), "the endpoint: {ep}");
    assert!(!conf["plugins"]["updater"]["pubkey"].as_str().unwrap_or("").is_empty(), "pubkey is required by the plugin — an empty one panics at startup");
}
```

- [ ] **Step 10: Run everything, then commit the gated half**

Run (from `app/`): `cargo test` — green, zero new warnings. Then `cargo build --release` in `app/` and launch against a scratch vault:

```powershell
scripts\scratch-vault.ps1 -Source <path to the main checkout>
app\target\release\knowlu.exe --vault "<the printed scratch path>"
```

Expected: the window opens — **that is the assertion this step exists for**; a plugin registered without its config would have panicked here. The topline shows no update banner, the settings Updates row reads `version 0.1.0 — up to date, checked <now>` or the endpoint's error, never a dialog. Quit from the tray.

Then commit all five edits **together**:

```bash
git add app/src/updates.rs app/src/main.rs app/Cargo.toml app/Cargo.lock app/capabilities/default.json app/tauri.conf.json app/tests/static_assets.rs
```

Message: `app: the updater goes live — the plugin, its endpoint and public key, and the real check and install (Knowlu plan 4a, Task 8)`.

---

### Task 9: The bundle, the identifier, the sidecar, and `scripts/release.ps1`

**Depends on Task 1.** Step 1 reads the spike report's `Outcome:` line and takes that branch. Nothing else in this task is conditional.

**Files:**
- Create: `scripts/release.ps1`, `scripts/sign.ps1`
- Modify: `app/tauri.conf.json`, `.gitignore`, `app/README.md`, `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: Task 1's outcome; `scheduler::engine_exe()`'s sibling rule (the installed sidecar is `quinn-ops.exe` beside `knowlu.exe`); `credentials::target_for`'s namespace, for the `knowlu/updater-key` this script reads. **It does not depend on Task 8's gated step:** the bundle builds unsigned-by-updater and `release.ps1` says so if the key is absent.
- Produces: `app/target/release/bundle/nsis/Knowlu_<version>_x64-setup.exe`, its `.nsis.zip` + `.sig` update artefacts, and `site/releases/latest.json`.

- [ ] **Step 1: Read the spike and record the branch**

Open `docs/superpowers/reports/2026-09-05-tauri-bundle-spike.md`, find the `Outcome:` line, and write one line into this task's notes: *"Task 1 outcome X — this task takes branch X."* Branch A is the body below as written. Branch B adds the NSIS location the spike named to step 4's environment. Branch C replaces step 4's `cargo tauri build` with `makensis scripts\nsis\knowlu.nsi` and keeps `bundle.active: false` (the `.nsi` script lists `knowlu.exe`, `quinn-ops.exe` and the bootstrapper, and is written in this step from the spike's own file listing). Branch D does the manifest fix first and then re-runs Task 1's steps 3–6.

- [ ] **Step 2: `app/tauri.conf.json` — the identifier and the bundle**

```json
  "identifier": "com.knowlu.desktop",
```

(from `app.knowlu.desktop` — decision 8; reverse-DNS of `knowlu.com`, which Quinn holds. The installer bakes it in, so it is settled here and never moves again: `tauri_plugin_autostart`'s registry entry and the window-state file are keyed by it, so the first launch after this re-registers autostart and resets the window position once.)

```json
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"],
    "createUpdaterArtifacts": true,
    "externalBin": ["binaries/quinn-ops"],
    "windows": {
      "webviewInstallMode": { "type": "embedBootstrapper" },
      "signCommand": { "cmd": "powershell", "args": ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "../scripts/sign.ps1", "%1"] }
    }
  }
```

**`signCommand` is why signing works at all here (R-P4a-16).** The bundler runs it **per binary, before packing** (`tauri-utils` 2.9.3, `WindowsConfig::sign_command`: *"Specify a custom command to sign the binaries… `%1` … a placeholder for the binary path"*), so `knowlu.exe`, the sidecar and the installer are each signed at the moment they are produced. Signing the installer afterwards — what the previous draft did — leaves the two binaries *inside* it unsigned, which is the half SmartScreen actually looks at. `%1` is substituted by the bundler; the path in `args` is relative to `app/`, which is `cargo tauri build`'s working directory.

- [ ] **Step 3: `.gitignore`**

Append (the sidecar is a 4 MB build product; the release artefacts are tens of MB):

```
/app/binaries/
/site/releases/*
!/site/releases/.gitkeep
```

- [ ] **Step 3b: `scripts/sign.ps1`** — the one thing `signCommand` runs

```powershell
# Signs ONE binary, called by the Tauri bundler through bundle.windows.signCommand with the
# binary's path as the only argument (plan 4a Task 9, R-P4a-16).
#
# It must EXIT 0 when there is no signing profile: an unsigned dev build still has to bundle, and a
# non-zero exit here fails the whole `cargo tauri build`. It says UNSIGNED: once, loudly, per file.
#
# No signing secret is in this repo or in this script: Trusted Signing authenticates through
# Quinn's own Azure login, and the profile file lives outside the checkout.
# PowerShell 5.1: no &&, no ||, no ternary, no ??.
param(
  [Parameter(Mandatory = $true)][string]$Path,
  [string]$SigningProfile = (Join-Path $env:USERPROFILE ".knowlu\trusted-signing.json"),
  [string]$SignDlib = "C:\Program Files\Microsoft\Azure Code Signing\bin\x64\Azure.CodeSigning.Dlib.dll",
  [string]$SignTool = ""
)
$ErrorActionPreference = "Stop"
if ($SignTool -eq "") {
  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($cmd) { $SignTool = $cmd.Source }
}
if (-not (Test-Path $SigningProfile)) { Write-Output "UNSIGNED: no Trusted Signing profile at $SigningProfile - $Path"; exit 0 }
if ($SignTool -eq "") { Write-Output "UNSIGNED: signtool.exe not on PATH (winget install Microsoft.WindowsSDK.10.0.26100) - $Path"; exit 0 }
if (-not (Test-Path $SignDlib)) { Write-Output "UNSIGNED: no Azure Code Signing dlib at $SignDlib - $Path"; exit 0 }
& $SignTool sign /v /fd SHA256 /tr "http://timestamp.acs.microsoft.com" /td SHA256 /dlib $SignDlib /dmdf $SigningProfile $Path
if (-not $?) { throw "signtool failed on $Path" }
Write-Output "signed $Path"
exit 0
```

(M6: `-SignDlib` carries the Azure dlib path as a parameter with that default, so a different SDK layout is one argument rather than an edit.)

- [ ] **Step 4: `scripts/release.ps1`**

```powershell
# Knowlu release (plan 4a, Task 9): both binaries from ONE commit, signed by the bundler as it
# packs (bundle.windows.signCommand -> scripts\sign.ps1), manifest written, artefacts dropped in
# site\releases\.
#
# PowerShell 5.1: no &&, no ||, no ternary, no ?? — chains are `A; if ($?) { B }`.
#
# NO SECRET IS EVER WRITTEN TO DISK. The Tauri updater's private key is read from Windows
# Credential Manager into an environment variable for the length of one `cargo tauri build` and
# cleared in a finally (R-P4a-14). Authenticode signing needs no secret at all: Trusted Signing
# authenticates through Quinn's own Azure login, and sign.ps1 reads the profile from outside the
# repo.
param(
  [switch]$SkipBuild,
  [string]$UpdaterCredential = "knowlu/updater-key",
  [string]$UpdaterPasswordCredential = "knowlu/updater-key-password"
)
$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$conf = Get-Content (Join-Path $repo "app\tauri.conf.json") -Raw | ConvertFrom-Json
$version = $conf.version
Write-Output "Knowlu $version"

# --- Credential Manager, read-only, in-memory ------------------------------------------------
# The engine reads credentials in Rust and the app writes them in Rust; PowerShell has no cmdlet
# for generic credentials, so this is the same CredReadW through Add-Type. The blob is UTF-16LE
# and CredentialBlobSize counts BYTES, not code units — the one trap wincred.rs documents.
Add-Type -Namespace Knowlu -Name Cred -MemberDefinition @'
[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct CREDENTIAL {
  public uint Flags; public uint Type; public string TargetName; public string Comment;
  public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
  public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist;
  public uint AttributeCount; public IntPtr Attributes; public string TargetAlias; public string UserName;
}
[DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
public static extern bool CredReadW(string target, uint type, uint flags, out IntPtr credential);
[DllImport("advapi32.dll")] public static extern void CredFree(IntPtr buffer);
public static string Read(string target) {
  IntPtr p;
  if (!CredReadW(target, 1, 0, out p)) { return null; }
  try {
    CREDENTIAL c = (CREDENTIAL)Marshal.PtrToStructure(p, typeof(CREDENTIAL));
    return Marshal.PtrToStringUni(c.CredentialBlob, (int)(c.CredentialBlobSize / 2));
  } finally { CredFree(p); }
}
'@ -UsingNamespace System.Runtime.InteropServices

# --- one commit, both binaries ----------------------------------------------------------------
Push-Location $repo
$dirty = git status --porcelain
Pop-Location
if ($dirty) { throw "working tree is dirty - commit or stash before releasing" }

if (-not $SkipBuild) {
  Push-Location $repo
  cargo build --release
  if (-not $?) { Pop-Location; throw "engine build failed" }
  Pop-Location
}

# The sidecar: Tauri wants the target triple in the name and strips it on install, so the installed
# file is `quinn-ops.exe` beside `knowlu.exe` — exactly where scheduler::engine_exe() looks.
$triple = "x86_64-pc-windows-gnu"
$bin = Join-Path $repo "app\binaries"
if (-not (Test-Path $bin)) { New-Item -ItemType Directory -Force $bin | Out-Null }
Copy-Item (Join-Path $repo "target\release\quinn-ops.exe") (Join-Path $bin "quinn-ops-$triple.exe") -Force

# --- the updater key, for exactly one build ----------------------------------------------------
$needsKey = $conf.bundle.createUpdaterArtifacts -eq $true
$key = [Knowlu.Cred]::Read($UpdaterCredential)
if ($needsKey -and (-not $key)) {
  throw "createUpdaterArtifacts is true but $UpdaterCredential is not in Credential Manager. Create it with: cmdkey /generic:$UpdaterCredential /user:knowlu /pass"
}
try {
  if ($key) {
    $env:TAURI_SIGNING_PRIVATE_KEY = $key
    $pw = [Knowlu.Cred]::Read($UpdaterPasswordCredential)
    if ($pw) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $pw }
  }
  Push-Location (Join-Path $repo "app")
  cargo tauri build
  $built = $?
  Pop-Location
  if (-not $built) { throw "cargo tauri build failed" }
} finally {
  # Cleared whatever happened, including on the throw above.
  Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
  Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  $key = $null; $pw = $null
}

$bundle = Join-Path $repo "app\target\release\bundle\nsis"
$setup = Get-ChildItem $bundle -Filter "*-setup.exe" | Sort-Object LastWriteTime | Select-Object -Last 1
if (-not $setup) { throw "no installer in $bundle" }
Write-Output ("installer: " + $setup.FullName + " (" + [math]::Round($setup.Length / 1MB, 2) + " MB)")
Write-Output "authenticode: whatever scripts\sign.ps1 printed above - an UNSIGNED: line means Windows will warn on install"

# --- artefacts and the manifest ----------------------------------------------------------------
$rel = Join-Path $repo "site\releases"
if (-not (Test-Path $rel)) { New-Item -ItemType Directory -Force $rel | Out-Null }
Copy-Item $setup.FullName $rel -Force
# M5: a stable name the download page can link forever, beside the versioned file.
Copy-Item $setup.FullName (Join-Path $rel "Knowlu-setup.exe") -Force

$zip = Get-ChildItem $bundle -Filter "*.nsis.zip" | Sort-Object LastWriteTime | Select-Object -Last 1
$sig = Get-ChildItem $bundle -Filter "*.nsis.zip.sig" | Sort-Object LastWriteTime | Select-Object -Last 1
if ($needsKey) {
  if ((-not $zip) -or (-not $sig)) { throw "no update artefact beside the installer - the updater key produced nothing" }
  Copy-Item $zip.FullName $rel -Force
  $manifest = [ordered]@{
    version   = $version
    notes     = "Knowlu $version"
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
      "windows-x86_64" = [ordered]@{
        signature = (Get-Content $sig.FullName -Raw).Trim()
        url       = "https://knowlu.com/releases/" + $zip.Name
      }
    }
  }
  # S10: UTF-8 WITHOUT a BOM. Out-File -Encoding utf8 writes one in 5.1, and a BOM in front of `{`
  # makes the manifest fail to parse in the updater, in a browser, and in jq.
  $json = $manifest | ConvertTo-Json -Depth 6
  [System.IO.File]::WriteAllText((Join-Path $rel "latest.json"), $json, (New-Object System.Text.UTF8Encoding($false)))
  Write-Output ("manifest: " + (Join-Path $rel "latest.json"))
} else {
  Write-Output "no updater artefacts (createUpdaterArtifacts is false) - installer only"
}
Write-Output "upload site\ to Cloudflare Pages by hand (plan 4a Task 10; no account yet)"
```

- [ ] **Step 5: Pin the bundle facts in a test** (`app/tests/static_assets.rs`)

```rust
#[test]
fn the_bundle_config_is_the_one_the_installer_and_the_updater_need() {
    let conf: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    assert_eq!(conf["identifier"], "com.knowlu.desktop", "decision 8 — baked into every install");
    assert_eq!(conf["bundle"]["active"], true);
    assert_eq!(conf["bundle"]["targets"][0], "nsis");
    assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
    assert_eq!(conf["bundle"]["externalBin"][0], "binaries/quinn-ops", "the engine travels as a sidecar (decision 7)");
    assert_eq!(conf["bundle"]["windows"]["webviewInstallMode"]["type"], "embedBootstrapper");
    assert_eq!(conf["version"], "0.1.0", "one version, in one place (Knowlu spec §6)");
    // R-P4a-16: the bundler signs each binary BEFORE it packs, through one wrapper script.
    let sign = &conf["bundle"]["windows"]["signCommand"];
    assert_eq!(sign["cmd"], "powershell");
    let args: Vec<&str> = sign["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert!(args.contains(&"../scripts/sign.ps1") && args.contains(&"%1"), "{args:?}");
    assert!(std::path::Path::new("../scripts/sign.ps1").is_file(), "the wrapper the bundler will run");
    // The updater endpoint is NOT asserted here (S6): `plugins.updater` only exists once Task 8's
    // gated step has landed, and this task must not depend on Quinn's keypair.
}
```

(`serde_json` is already an `app/` dependency; the test runs with `app/` as its working directory, which is why the path is bare.)

- [ ] **Step 6: Build the release and check what is inside it**

Background call, bounded wait:

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
pwsh -NoProfile -File scripts/release.ps1 2>&1 | tail -30
```

(or `powershell.exe -NoProfile -File scripts\release.ps1` — 5.1 is the target). Expected: the installer path and size, either `signed against …` or the two-line unsigned banner, and the manifest path. **Do not install it on this desktop.** Extract and confirm both binaries and the bootstrapper are inside, as in Task 1 step 6. Record the installer size against the 50 MB target from the Knowlu spec §6.

- [ ] **Step 7: `app/README.md`**

Replace the "identifier is a placeholder" paragraph with: the identifier is `com.knowlu.desktop` and is permanent; the engine ships as a sidecar (`binaries/quinn-ops-<triple>.exe`, installed as `quinn-ops.exe` beside `knowlu.exe`, which is where `engine_exe()` looks); `cargo tauri build` from `app/` after `cargo build --release` at the root; `scripts\release.ps1` is the one release command and what it needs (`tauri-cli`, `signtool` from the Windows SDK, a Trusted Signing profile outside the repo, and `knowlu/updater-key` in Credential Manager); **signing happens inside the bundler** through `bundle.windows.signCommand` → `scripts\sign.ps1`, which prints `UNSIGNED:` and exits 0 when there is no profile, so a dev build still bundles; and **no secret reaches disk** — the updater key is read from Credential Manager into an environment variable for one build and cleared in a `finally`.

- [ ] **Step 8: Commit**

```bash
git add app/tauri.conf.json .gitignore scripts/release.ps1 scripts/sign.ps1 app/README.md app/tests/static_assets.rs
```

Message: `release: com.knowlu.desktop, an NSIS bundle with the WebView2 bootstrapper and the engine sidecar, signed as it packs, keyed from Credential Manager (Knowlu plan 4a, Task 9)`.

---

### Task 10: The site

**Files:**
- Create: `site/index.html`, `site/privacy.html`, `site/site.css`, `site/releases/.gitkeep`, `tests/site.rs`

**Interfaces:**
- Consumes: the privacy sentence from `app/static/console.js` (`PRIVACY`, Task 6) — the same words, pinned by the test; `scripts/release.ps1`'s drop location (`site/releases/`).
- Produces: a folder that opens in a browser today and is a Cloudflare Pages deploy the day Quinn's account exists.

- [ ] **Step 1: Write the failing test** (`tests/site.rs`, repo root — the engine crate's test dir)

```rust
//! The download page (plan 4a, Task 10; spec §7). Plain HTML and CSS, **no scripts**, and the
//! privacy paragraph is the SAME sentence the wizard shows — a page that promised something
//! different from the app would be worse than no page.
use std::fs;

const PRIVACY: &str = "Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates.";

#[test]
fn the_site_is_plain_html_and_says_exactly_what_the_wizard_says() {
    let index = fs::read_to_string("site/index.html").expect("site/index.html");
    let privacy = fs::read_to_string("site/privacy.html").expect("site/privacy.html");
    let wizard = fs::read_to_string("app/static/console.js").expect("console.js");
    for (name, text) in [("index.html", &index), ("privacy.html", &privacy)] {
        assert!(!text.contains("<script"), "{name} must carry no script (decision 11)");
        assert!(text.contains("site.css"), "{name} uses the one stylesheet");
    }
    assert!(privacy.contains(PRIVACY), "the privacy page carries the exact sentence");
    assert!(index.contains(PRIVACY), "so does the download page");
    assert!(wizard.contains(PRIVACY), "and so does the wizard — one sentence, three places");
    // M5: the stable name `release.ps1` copies beside the versioned installer, so the page's link
    // never has to change with a version. The versioned file stays there too.
    assert!(index.contains("releases/Knowlu-setup.exe"), "the download button points at the stable name");
    // No claim the product cannot back (product plan §8: scraping ToS, minors and FERPA are
    // Quinn's external dependency and the page makes no claim about them).
    for banned in ["FERPA", "compliant", "certified", "guarantee"] {
        assert!(!index.contains(banned) && !privacy.contains(banned), "the site must not claim {banned}");
    }
    assert!(std::path::Path::new("site/releases/.gitkeep").is_file(), "the drop folder exists in git");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run (repo root): `cargo test --test site`
Expected: FAIL — `site/index.html` missing.

- [ ] **Step 3: `site/site.css`** — the console's own tokens, reduced to what a two-page site needs

```css
:root {
  --canvas: #0A0B0D; --s2c: #15181B; --hair: #22262B; --hair-2: #30353B;
  --t1: #F1F3F4; --t2: #C3C9CE; --t3: #878E95; --acc: #3FB68B;
  --s2: 8px; --s3: 12px; --s4: 16px; --s5: 24px; --s7: 48px; --r2: 6px;
  color-scheme: dark;
}
* { box-sizing: border-box; }
body { background: var(--canvas); color: var(--t1); margin: 0; font: 15px/1.55 ui-sans-serif, system-ui, -apple-system, sans-serif; }
main { max-width: 680px; margin: 0 auto; padding: var(--s7) var(--s4); }
h1 { font-size: 30px; margin: 0 0 var(--s2); }
h2 { font-size: 18px; margin: var(--s5) 0 var(--s2); }
p { color: var(--t2); }
.mark { width: 16px; height: 16px; border-radius: 4px; background: var(--acc); display: inline-block; vertical-align: -2px; margin-right: var(--s2); }
.dl { display: inline-block; background: var(--acc); color: #08130F; font-weight: 700; text-decoration: none; padding: 10px var(--s5); border-radius: var(--r2); margin: var(--s3) 0; }
.meta, footer { color: var(--t3); font-size: 13px; }
footer { border-top: 1px solid var(--hair); margin-top: var(--s7); padding-top: var(--s4); }
ul { color: var(--t2); }
```

- [ ] **Step 4: `site/index.html`**

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Knowlu — what should I work on today?</title>
<link rel="stylesheet" href="site.css">
</head>
<body>
<main>
  <h1><span class="mark"></span>Knowlu</h1>
  <p>Knowlu answers one question every morning: <strong>what should I work on today, and in what order?</strong> It reads your course calendar, keeps every assignment as a plain file on your own machine, and puts the day in front of you when you open it.</p>
  <a class="dl" href="releases/Knowlu-setup.exe">Download for Windows</a>
  <p class="meta">Windows 10 or 11, 64-bit. Version 0.1.0. The installer brings everything it needs, including the WebView2 runtime.</p>
  <h2>Your data</h2>
  <p>Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates.</p>
  <p class="meta"><a href="privacy.html">What is stored, and where</a></p>
  <h2>What it does today</h2>
  <ul>
    <li>Reads your LMS calendar feed — the link Blackboard and Canvas already give you.</li>
    <li>Keeps coursework logins in Windows Credential Manager, never in a file.</li>
    <li>Runs twice a day on its own, from the tray, on the schedule you pick.</li>
    <li>Mirrors your vault to a folder you own, with a dated snapshot a day.</li>
  </ul>
  <p class="meta">Right-click the tray icon and choose <em>Copy diagnostics</em> to send us a problem. It carries the version, the last three runs and the last error — and no note content.</p>
  <footer>Knowlu 0.1.0 &middot; <a href="privacy.html">Privacy</a></footer>
</main>
</body>
</html>
```

- [ ] **Step 5: `site/privacy.html`**

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Knowlu — what is stored, and where</title>
<link rel="stylesheet" href="site.css">
</head>
<body>
<main>
  <h1>What is stored, and where</h1>
  <p>Everything stays on this machine. Knowlu has no account and sends nothing anywhere; the only network calls are to the sources you connect and to check for updates.</p>
  <h2>On your machine</h2>
  <ul>
    <li><strong>Your vault</strong> — the folder you chose. Plain Markdown files with a short header, one per task, plus a record of every change under <code>state\journal\</code>. You can read all of it in any text editor.</li>
    <li><strong>App data</strong> — <code>%LOCALAPPDATA%\knowlu\</code>: which profiles exist, your settings, when you last looked, and the logs of the last sixty runs.</li>
    <li><strong>Coursework logins</strong> — Windows Credential Manager, under <code>knowlu/&lt;profile&gt;/&lt;source&gt;</code>. Never in the vault, never in a backup, never in a log.</li>
    <li><strong>Your backup folder</strong>, if you set one — a mirror of the vault and one dated snapshot a day, in a folder you chose and control.</li>
  </ul>
  <h2>What leaves the machine</h2>
  <ul>
    <li>Requests to the sources <em>you</em> connected: your LMS calendar feed, your coursework sites, and any campus event feeds you turned on.</li>
    <li>One request a day to check whether a new version of Knowlu exists. It carries the version you are running and nothing else.</li>
  </ul>
  <p>That is the whole list. There is no analytics, no crash reporting and no account — and so there is no switch to turn any of it off.</p>
  <p><em>Copy diagnostics</em> in the tray menu puts the version, the last three runs, the last error and your slot times on the clipboard. It never includes the text of a note, and nothing is sent until you paste it somewhere yourself.</p>
  <footer><a href="index.html">Back to Knowlu</a></footer>
</main>
</body>
</html>
```

- [ ] **Step 6: `site/releases/.gitkeep`** — an empty file, so the drop folder exists in a fresh clone while `.gitignore` keeps the artefacts out (Task 9 step 3).

- [ ] **Step 7: Run the test and look at the pages**

Run (repo root): `cargo test --test site` — PASS. Then open `site/index.html` and `site/privacy.html` in a browser and read them: the download link resolves once `release.ps1` has run, the two pages agree, nothing is broken at a narrow window.

- [ ] **Step 8: Quinn's moment — Cloudflare (spec §11), not blocking**

Ask, once: *"Is there a Cloudflare account for knowlu.com yet? If so I will write the Pages deploy line into `scripts/release.ps1`; if not, the folder is built and uploaded by hand and nothing is blocked."* If the answer is no, the task closes with the folder as it is and the line in `release.ps1`'s output stands.

- [ ] **Step 9: Commit**

```bash
git add site tests/site.rs
```

Message: `site: the download page and the privacy page — the wizard's own words, no scripts (Knowlu plan 4a, Task 10)`.

---

### Task 11: Close — README, anatomy, HANDOFF, CLAUDE.md, the specs, the status line

**Files:**
- Modify: `app/README.md`, `docs/surface/anatomy.md`, `docs/HANDOFF.md`, `CLAUDE.md`, `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` (§9's order note), `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md` (status line), `docs/superpowers/plans/2026-09-05-knowlu-runner-leaves-plan.md` (Task 12's two steps become checks), this plan's status line

**Interfaces:**
- Consumes: everything above.
- Produces: docs that describe the code, and two steps of plan 2 turned from work into verification.

> Every file in this task is under `docs/` or is `CLAUDE.md`/`README.md` — **CRLF**. Check each with
> `tr -cd '\r' < <file> | wc -c` against `wc -l` before and after editing; if a script rewrote one
> whole, restore its endings before committing, and confirm `git diff --stat` shows no whole-file flip.

- [ ] **Step 1: Screenshots**

```bash
.wv/Scripts/python scripts/console-shots.py tests/fixtures/surface-today-full.json shots/
```

Expected: every viewport ≥ 820 `ok` across Today, the three plan-2 views, the wizard, the picker and the settings panel. **Look at** `1280-wizard.png`, `1280-picker.png` and `1280-settings.png` and write one sentence each about what they actually show into the anatomy's §5 note — the plan-1 lesson was that layout is looked at, not reasoned about.

- [ ] **Step 2: `app/README.md`**

Rewrite four things and add one section:
- The "No settings UI; settings are a file" wrinkle → **there is a settings panel** (gear on the topline, *Settings* in the tray); the file is still `settings.json` and still needs all four keys, now under `%LOCALAPPDATA%\knowlu\profiles\<id>\`.
- "No profiles UI, no picker" → **the picker and the wizard exist**; `--vault` still wins; zero profiles → the wizard, one → straight in, more → the picker; **switching profiles relaunches the exe with `--vault --after-pid <old>`**, which waits (bounded 10 s) for the old process to go so `tauri-plugin-single-instance` lets the new one in. **One Knowlu at a time on a machine** — that is the plugin's rule and it is kept.
- The data-root line → `%LOCALAPPDATA%\knowlu\` with the one-time move from `quinn-ops`, `profiles.json`, `profiles\<id>\{settings.json,seen.txt,logs\}`, `updates\`.
- The scheduler section → a slot is **coursework → ingest → rank**, with `ingest` left out (and named as a skipped step) when the vault has no `ics_url`.
- New section **"Releasing"**: `scripts\release.ps1`, what it needs, the sidecar, the unsigned banner, `site/` uploaded by hand.

The "What is in here" table gains rows for `src/profiles.rs`, `src/credentials.rs`, `src/scaffold.rs`, `src/onboarding.rs`, `src/updates.rs`, `assets/`, `binaries/`.

- [ ] **Step 3: `docs/surface/anatomy.md`**

§2 (the page set) gains three non-view panels — picker, wizard, settings — each with one line saying what it holds and how it is reached. §7.1's parity table gains three rows: *wizard → `create_vault`/`adopt_vault`/`restore_vault`/`store_credentials`/`finish_onboarding` → no ui_event* (this plan adds no action, F19/R7); *settings → `get_settings`/`set_settings`/`set_profile_name`/`copy_diagnostics`*; *update offer → `check_for_updates`/`install_update`*. §5 gains the three screenshot sentences from step 1.

- [ ] **Step 4: `docs/HANDOFF.md`**

A `▶ KNOWLU PLAN 4A DONE (<date>)` block above the previous one: what shipped (profiles and the move, onboarding, **`quinn-ops ingest` and the three-step slot**, settings, credentials, updater, bundle + release script, the site); what is open (**Quinn's**: the updater keypair if Task 8 step 8 went unanswered — in which case Task 8's gated step 9 is still owed and the shipped build simply never offers an update, the Cloudflare account, the Trusted Signing profile, `signtool`, the first friend's name and machine); what did **not** move (G2, `$mode`, the cloud routine, `engine/`, plan 2 Part B's gate); and what is next (plan 3, then plan 4b).

Add one paragraph on the ingest change, because it is the plan's only `src/` edit and it has a live-system consequence to state plainly: **the slot is now coursework → ingest → rank**, but Knowlu's scheduler is inert on Quinn's vault until plan 2 Task 9 flips `scheduler: app`, so nothing about today's runs changes. When it is flipped, the cloud routine will still be running `python3 -m engine.ingest` twice a day until plan 3 retires it — **the overlap is safe**: `state/ingest-seen.md` records every uid ever ingested, so whichever pass runs second creates nothing. The `rank` step is unaffected either way, and both oracles and both dual-run scripts were green on the commit that landed it.

- [ ] **Step 5: `CLAUDE.md`**

The `## Knowlu (the console)` section: the app-data root is `%LOCALAPPDATA%\knowlu` with per-profile folders; the "fifteen commands" figure is stale — **recount `generate_handler!` in `main.rs` and say the new number once**; a vault made by the wizard carries `scheduler: app` and `device:` from birth and that is not the plan-2 rule's subject; the credential targets the app writes are `knowlu/<profile_id>/<source>` and the engine's reader is unchanged; **a slot is coursework → ingest → rank**, `quinn-ops ingest` exists, and the cloud routine's Python ingest overlaps safely through `state/ingest-seen.md`; the installer identifier is `com.knowlu.desktop`.

- [ ] **Step 6: The two specs and plan 2**

- Knowlu spec §9: one sentence recording that plan 4's shell half ran before plan 3, and pointing at the friends-shell design. Its §7 stage-2 list loses the app-data-root and identifier items (done here).
- The friends-shell design, four amendments, each one line with the ruling that made it:
  - **§2**: the sentence *"two profiles can be open in two tray icons"* is **withdrawn** — one Knowlu per machine, `tauri-plugin-single-instance` unchanged; switching profiles relaunches with `--after-pid` (R-P4a-1).
  - **§3.1 and decision 5**: the seed record's context is `("system:migration", "cli")`, not `"onboarding"` — the S1 migration's own shape; the guard keys on the actor (R-P4a-6).
  - **§3 panel 2**: the default path is restored as the spec wrote it — the picker chooses the **parent**, a name field makes `<parent>\<name>`, defaulting to `%USERPROFILE%\Documents\Knowlu\<name>`, and the panel shows the resulting path before Finish (R-P4a-11).
  - **§2**: the picker's *Add another…* and the settings overlay's *Switch profile…* are both in scope, both relaunches (R-P4a-15).
  - **§4**: the table's six rows and decision 9's "five rows" are reconciled — five plus Updates, whose markup lands **disabled** with Task 7 (labelled *updates arrive with the next build*) and is wired by Task 8 (R-P4a-5). The row shows version, last check, staged-or-not and the last error; **it names no "channel"** — there is one (M8).
  - **§1 and §9**: the shell half also lands `quinn-ops ingest` and the three-step slot, because a wizard that collects an LMS feed nothing fetches is not a working install (R-P4a-7, R-P4a-9).
  - The status line: `**Status: EXECUTED <date>** — see docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md`.
- Plan 2's Task 12: **step 2 (the app-data move) and step 3 (the identifier) become checks** — "already done by plan 4a Task 2 / Task 9; verify `%LOCALAPPDATA%\knowlu` exists with `profiles.json`, and that `tauri.conf.json` reads `com.knowlu.desktop`; change nothing." Add the same sentence to plan 2's fidelity ledger row F14. **And its step 1's test is superseded** (S12): `the_app_data_root_is_knowlu_and_an_old_root_is_moved_once` as that plan spells it asserts a root-level `settings.json`, which no longer exists — the files live under `profiles\<id>\`. `app/tests/profiles.rs`'s `the_flat_layout_moves_whole_and_nothing_is_lost` and `a_half_moved_layout_is_folded_on_the_next_launch_and_never_stranded` cover it; say so in Task 12 step 1 so nobody writes the old assertion and then "fixes" the code to match it.

- [ ] **Step 7: This plan's status line**

`**Status: EXECUTED <date> on branch worktree-knowlu-plan-4a (Tasks 1–10; the SDD ledger is preserved at docs/superpowers/reports/<date>-knowlu-plan-4a-sdd-ledger.md).**`

- [ ] **Step 8: Final green, then commit**

Run: `cd app; cargo test` (zero new warnings) and root `cargo test` (0 warnings, `tests/oracle.rs`, `tests/surface_oracle.rs`, `tests/site.rs` green). Confirm `git diff --stat` shows no whole-file line-ending flip on any doc.

```bash
git add app/README.md docs/surface/anatomy.md docs/HANDOFF.md CLAUDE.md docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md docs/superpowers/plans/2026-09-05-knowlu-runner-leaves-plan.md docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md
```

Message: `docs: Knowlu plan 4a — profiles, onboarding, settings, credentials, the updater, the installer and the site (Knowlu plan 4a, Task 11)`.

---

## Self-review (writing-plans checklist)

**1. Spec coverage.** §1's six in-scope items map to tasks: profiles (2), onboarding (4, 6), settings (7), credentials (3), installer/updater/release (1, 8, 9), site and manifest (9, 10) — plus **Task 5**, which the spec did not have and ruling R-P4a-7 added. §2's layout, registry shape, `ledger::dumps_value`, the F14 move and launch resolution are Task 2; §2's "two profiles can be open in two tray icons" sentence is **withdrawn** by R-P4a-1 and amended in Task 11. §3's seven panels, the back-always rule, the credentials-on-leave exception, the skip for adopt/restore, and the "no live fetch" rule are Task 6; §3.1's scaffold, the two engine calls (the seed now `("system:migration", "cli")`, R-P4a-6), `scheduler: app` at birth and the atomic materialisation are Task 4. §4's table is Task 7 (five rows) plus Task 8 (the Updates row), reconciling the table's six rows with decision 9's "five". §5 is Task 3. §6's bundle, sidecar, identifier, updater, signing and release location are Tasks 1, 8, 9. §7 is Task 10. §8's test list is covered: profiles (Task 2, five tests including the half-moved layout and the `--after-pid` wait), **ingest (Task 5, five engine tests and two app tests)**, onboarding (Task 4's five, Task 6's static test, **Task 6's two "never wrote there" tests** — adopting hashes the vault either side, restoring hashes the backup either side — and the headless behaviour check), credentials (Task 3), settings and wizard behaviour (`scripts/wizard-check.py`, Tasks 6 and 7 — panel order, Back/Next, ICS shape, cleared credential fields, the settings payload), updater (Task 8's three), static assets (every task that touches the page), installer (Tasks 1, 9). §9's ten tasks are Tasks 1–4 and 6–11 in order, with Task 5 inserted after the scaffold — the port has to exist before the slot can run it, and after the scaffold that writes the `ics_url` it reads. §10's eleven decisions and §12's rulings are the fidelity ledger, each with a task; rows R11 and R12 carry the two rulings the spec did not have. §11's five Quinn items are asked one at a time, where the spec says: the keypair at Task 8 step 7 (blocking that step alone), Cloudflare at Task 10 step 8 (not blocking), Trusted Signing and `signtool` at Task 9 step 4 (the build runs unsigned and says so), the first friend after Task 9 (Task 11's HANDOFF line).

**Gaps found, and what was done with each.** (a) **Nothing fetched the LMS feed** — `rank`'s steps are `passes, tasks, calendar, events, approvals`, `slot_argv` was `coursework` then `rank`, `src/main.rs` had no `Ingest`, and Blackboard ingest lived only in the cloud routine. **Closed by Task 5**, the plan's one `src/` change, gated on root `cargo test` at 0 warnings, both oracles, and both dual-run scripts at exit 0. (b) **The engine exits 1 on an empty `ics_url`** (Python does, and the port keeps it), which a slot would read as a failed run and answer with backoff and an amber tray — so the app leaves the step out when the vault has no feed and records `ingest (skipped: no ics_url)` instead (R-P4a-9); the engine's own behaviour is untouched. (c) **Settings cannot be a view** — `surface::View::parse` would refuse the name on every poll, so it is an overlay panel with two entry points. (d) **Switching profiles relaunches the exe** with `--vault … --after-pid <old>`: `ConsoleState`, the tray, the scheduler and autostart's arguments are all fixed at startup, and `tauri-plugin-single-instance`'s lock — keyed on the app identifier, `platform_impl/windows.rs`'s `format!("{id}-sim")` — is still held by the process doing the switching, so the child waits for it to go.

**2. Placeholder scan.** No "TBD", no "implement later", no "add validation", no "write tests" without the test, **and no ellipsis standing in for a body** — `fn main` and `run_console` are written out (R-P4a-10). Every code step carries the code. Three steps are deliberately conditional and each says exactly what decides them: Task 9 step 1 (Task 1's outcome, with a four-row table), Task 8 step 8 (Quinn's public key — the plugin, its config and the real bodies land in one commit or none), Task 10 step 8 (Cloudflare — not blocking). Task 1 is a spike and says so, with its decision table and a revert step.

**3. Type consistency.** Names carried across tasks, checked one by one:
`profiles::{Profile, load, save, register, id_for, default_name, profile_dir, updates_dir, migrate_flat_layout, resolve_launch, Launch, wait_for_pid_gone}` — defined Task 2, used by Tasks 6 (`register`, `id_for`, `default_name`), 7 (`register`, `load`), 8 (`updates_dir`).
`state::{app_data_root, app_data_root_in, Settings}` and `ConsoleState.data_dir` — defined Task 2, used by Tasks 6 (`Settings::save`), 7, 8, and by `scheduler::log_dir(&ConsoleState)`.
`credentials::{target_for, write, exists, delete}` — Task 3; `target_for` used by Task 4 (`ingest_yaml`) and Task 6 (`store_credentials`).
`scaffold::{VaultPlan, CAMPUSES, campus_yaml, ingest_yaml, runners_yaml, create_vault, seed_writes}` — Task 4; `VaultPlan`'s fields (`profile_id, ics_url, timezone, slots, device, campus, zybooks, vhl`) are exactly what Task 6's `create_vault` command fills, and `CAMPUSES` is what `launch_state` serialises. `ingest_yaml`'s `ics_url:` line is the key Task 5's `has_ics_url` and `ingest::run_lines` both read.
`ingest::{run_lines, run_with, run}` and `scheduler::{IcsState, ics_state, has_ics_url, slot_argv}` — Task 5; `run_lines(&Path, &str, Option<&str>, Option<&dyn Fn(&str) -> Result<String, String>>) -> (i32, Vec<String>)` is the signature its five tests call, and `slot_argv` keeps returning `Vec<(PathBuf, Vec<String>)>`, so `run_slot_inner`'s loop is unchanged apart from the skip line, whose two spellings come from `ics_state` (R-P4a-17).
`profiles::wait_for_pid_gone(u32, Duration) -> bool` — Task 2, called from `main` on `--after-pid` and from `onboarding::relaunch_with`'s counterpart in the child; `--pick` is read in `main` and written by `commands::switch_profile` (R-P4a-15).
`onboarding::{Onboarding, WizardPlan, launch_state, pick_folder, adopt_vault, adopt_vault_in, open_profile, relaunch_with, refresh}` — Task 2; `onboarding::{dest_for, create_vault, restore_vault, restore_vault_in, apply_profile_settings, store_credentials, finish_onboarding, finish_profile, find_mirror, copy_tree}` — Task 6. `pick_folder` is called by Tasks 6 and 7 by that name; `create_vault(parent, name, plan)` and `restore_vault(backup, parent, name, plan)` are the shapes the page invokes (R-P4a-11), and both go through `dest_for`.
`updates::{Staged, Updates, InstallHold, update_offer, record_check, hold_for_install, check_and_stage, install_staged, check_and_stage_blocking, install_staged_blocking}` — Task 8. `update_offer(bool, Option<&Staged>) -> Option<String>` is what the tray arm, the command and the housekeeping tick all use; the `async` pair is what commands `.await` and the `_blocking` pair is what the two std threads call (B4). Steps 1–7 ship them as not-configured stubs; step 9 replaces the two bodies and nothing else.
`commands::{set_profile_name_in, set_profile_name, copy_diagnostics, copy_text, settings_context, switch_profile, check_for_updates, install_update}` — Tasks 7 and 8; the page invokes them by exactly these names, and `current.vaultPath` / `current.version` / `current.profileName` are filled by `settings_context` alone (S4).
Page: `bootConsole`, `renderPicker` (Task 2) are called by Task 6's boot branch; `startWizard`, `renderWizard`, `wizGo`, `wizFinish`, `clearCredentialFields`, `storeCredentials`, `PANELS`, `PRIVACY`, `ICS_OK`, `WIZ` (Task 6); `openSettings`, `renderSettings`, `window.KNOWLU_OPEN_SETTINGS` (Task 7) are what `tray.rs`'s `eval` calls by name; `renderUpdateOffer`, `checkForUpdates`, `current.updateText`, `current.version` (Task 8) are what Task 7's `#set-update-state` row reads. `window.KNOWLU_SHOTS` (Task 6) gains `openSettings` in Task 7 and is the only seam `console-shots.py` uses.
Test names are unique across files; `scratch`, `open`, `read`, `temp`, `temp_root`, `ingest_vault` follow the helper names already in `app/tests/` and `src/ingest.rs`.

**4. Rulings.** No task writes the live vault, performs a network fetch in a test, adds a `uievents` action, regenerates a reference, touches `engine/`, the cloud routine, `$mode` or G2. Exactly one task touches `src/` (Task 5) and it carries the full parity gate. The one write outside `console_ctx()` is decision 5's, in one three-line function, pinned by its own test. Rulings R-P4a-1 through R-P4a-17 are each carried by a named task and, where they change a written document, by a named amendment in Task 11.

**5. What the first review round changed, and where it landed.** B1 → Task 2 step 9 (`fn main` and `run_console` written out; `resolve_vault` no longer runs before the wizard exists). B2 → R-P4a-11, Task 6 (parent + name, `dest_for`, the path shown before Finish). B3 → the three count assertions now use the repo's space-prefixed spelling. B4 → R-P4a-12, Task 8 (`async fn` commands, `_blocking` twins for the two std threads, a comment at each). B5 → R-P4a-13, Task 8 split at step 8 (nothing registers the plugin before its config; the endpoint assertion moved out of Tasks 9 and 10 into the gated step). B6 → R-P4a-14, Task 9 (`CredReadW` via `Add-Type`, `TAURI_SIGNING_PRIVATE_KEY` for one build, cleared in a `finally`, refused when `createUpdaterArtifacts` is true and the credential is missing). S1–S13 and M1–M8 are applied where the review named them; S9 became R-P4a-16 (`bundle.windows.signCommand` → `scripts/sign.ps1`, verified against `tauri-utils` 2.9.3's `WindowsConfig::sign_command`), M7 became R-P4a-17 (`IcsState`'s two skip texts), and R-P4a-15 added the picker's *Add another…* (Task 6) and the settings overlay's *Switch profile…* with `main`'s `--pick` (Tasks 2 and 7).
