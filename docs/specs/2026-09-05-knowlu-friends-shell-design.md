# Knowlu — Friends, the shell half (plan 4a) — design

**Status: EXECUTED 2026-09-06** (Tasks 1–10; the updater's key-gated step is owed) — see
docs/superpowers/plans/2026-09-05-knowlu-friends-shell-plan.md

Argued from
`docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` (the Knowlu spec: §4 storage,
§6 distribution, §9 plan 4, §10 decisions 7, 9, 14–16, §11 open items) and the product plan's
§12 rulings 2 and 6 (local profiles + onboarding are near-term; "account" means a local profile,
not a hosted backend). It changes one thing in the Knowlu spec — the order of §9 — and records
why.

---

## 0.0 Amended by execution (2026-09-06) — read this before the body

Twelve things this document says were changed by what the code met. Each is one line with the
ruling that made it; where a line and the body disagree, **the line wins**.

- **§2 (one instance):** the sentence *"the single-instance plugin's key includes the profile id, so
  two profiles can be open in two tray icons"* is **withdrawn** — `tauri-plugin-single-instance` is
  keyed on the app identifier and stays that way, so **one Knowlu runs on a machine at a time**;
  switching profiles relaunches the exe with `--vault … --after-pid <old pid>`, and the child waits
  (bounded, 10 s) for the parent to go (**R-P4a-1**).
- **§2 (the picker):** its two buttons are *Use an existing vault…* (adopts a folder) and *Create a
  new vault…* (the wizard); the *Add another* wording is superseded (**R-P4a-22**). Both that
  button and the settings overlay's *Switch profile…* are in scope, and both are relaunches — the
  latter through `main`'s `--pick` (**R-P4a-15**).
- **§3.1 and decision 5 (the seed record):** its context is `("system:migration", "cli")`, not
  `"onboarding"` — the S1 migration's own shape. The unmigrated-vault guard keys on the **actor**,
  so the journal's `via` vocabulary does not grow (**R-P4a-6**).
- **§3 panel 2 (where a new vault goes):** the default path is restored as this document wrote it —
  the native picker chooses the **parent** (it can only return a folder that exists), a name field
  makes `<parent>\<name>`, defaulting to `%USERPROFILE%\Documents\Knowlu\<name>`, and the panel
  shows the resulting path before *Finish* (**R-P4a-11**).
- **§3 panel 5 and §5 (credentials):** a credential is keyed to the vault path **at the moment it is
  stored**. If the vault is renamed before *Finish*, Finish calls `retarget_credentials(from, to)`
  — read → write the new target → delete the old, and the secret never reaches the page — or is
  refused back to panel 5. `dest_for` refuses names Windows would rewrite: `* ? " < > |`, the
  reserved device names, a trailing dot or space (**R-P4a-23**).
- **§3 panel 3 (the backup folder):** it may not equal the vault or sit inside it; **a vault inside
  the backup folder is allowed**, because the mirror writes under `<backup>\<profile>\vault`
  (**R-P4a-25**). Done in the plan's final fix wave — `onboarding::check_backup_dir` and the
  wizard's own check refuse the one direction, and the default parent `%USERPROFILE%\Documents\Knowlu`
  is a valid backup folder for a vault at `Documents\Knowlu\<name>`. (Panel 4 is the LMS calendar.)
- **§4 and §3 (the folder picker):** it is **`rfd 0.16` called directly** with `default-features =
  false`, not `tauri-plugin-dialog`. The plugin 2.7.3 forces `rfd/common-controls-v6`, whose by-name
  `TaskDialogIndirect` import from `comctl32.dll` makes the GNU binary fail to load
  (`STATUS_ENTRYPOINT_NOT_FOUND`); there is consequently no `dialog:default` capability. Message
  boxes are `MessageBoxW`, never rfd's (**R-P4a-20**).
- **§4 (the table) vs decision 9 ("five rows"):** reconciled as **five plus Updates** (**R-P4a-5**).
  The Updates row shows the version, the last check, staged-or-not and the last error, and **names
  no "channel"** — there is one (**M8**). Its markup landed with the settings panel and was wired by
  the updater task; before that task it could only report what the stub returned.
- **§6 (bundle and release):** `bundle.createUpdaterArtifacts` is `false` until the key-gated
  updater step lands — tauri-cli refuses the flag without the plugin — and a static test pins
  flag ⇔ plugin. `release.ps1` sets `TAURI_SKIP_SIDECAR_SIGNATURE_CHECK` only when `signtool` is
  absent, which also skips *signing* the sidecar, so the Windows SDK is a prerequisite for the first
  signed release. `app/build.rs` drops a zero-byte placeholder sidecar so plain `cargo` builds on a
  fresh checkout, and `release.ps1` refuses to bundle a sidecar under 1 MiB (**R-P4a-26**).
- **§6 (the updater) and decision 15:** the key-gated step is **still owed at close** — no
  `tauri-plugin-updater`, no `plugins.updater`, and `check_for_updates`/`install_update` answer
  `updater not configured`. **The shipped build never offers an update until Quinn's public key
  arrives**, and the three tray/timeout/single-flight items land with it (**R-P4a-13**,
  **R-P4a-24**).
- **§1 and §9 (scope):** the shell half also lands **`quinn-ops ingest`** and the **three-step slot**
  (`coursework` → `ingest` → `rank`), because a wizard that collects an LMS feed nothing fetches is
  not a working install. The step is left out — and named `ingest (skipped: no ics_url)` — when the
  vault has no feed, since the engine exits 1 on an empty one (**R-P4a-7**, **R-P4a-9**).
- **§8 (testing):** the headless behaviour checks are `scripts/settings-check.py` and
  `scripts/wizard-check.py`. `window.KNOWLU_SHOTS` exports **three** — `startWizard`,
  `renderPicker` and `openSettings` (the last added in the final fix wave) — and
  `scripts/console-shots.py` puts all three panels on screen through it, writing
  `<w>-wizard.png`, `<w>-picker.png` and `<w>-settings.png` at every viewport. With no backend the
  settings rows keep their static markup, so that shot is the layout; `settings-check.py` is the
  one that drives the rows with answers and shoots them filled in (`settings-1512.png`).
- **§2 (the move):** `migrate_flat_layout` **never renames `%LOCALAPPDATA%\quinn-ops`**. It folds
  the flat `settings.json`/`seen.txt` and the app's own `slot-*`/`quit-*` logs into
  `profiles\<id>\`, file by file, and leaves everything else in that directory where it is —
  `dual\`, `logs\`'s other writers, `rehearsal\`, `scratch\`, `shots\`. The old root goes only if
  the fold emptied it (**R-P4a-28**, the final review's CRITICAL; §2 amended in the fix wave).

---

## 0. Why now, and what this reorders

Plan 2 Part A closed 2026-09-05; Part B waits for the cutover calendar (the harness comes out
~2026-09-13 with the reverse week at three days, see §10 decision 1). Quinn (2026-09-05): *"I don't
want to wait to keep building."* Everything in plan 4 that lives in the desktop shell is
independent of the runner hand-off, so it runs now, in parallel with the cutover clock.

**The Knowlu spec's §9 ordered plan 3 (judgment comes home) before plan 4 (friends).** This
document splits plan 4 and moves its shell half ahead of plan 3:

- **Plan 4a — the shell half (this document):** profiles, onboarding, the settings panel,
  credentials into Credential Manager, the installer with the WebView2 bootstrapper, the updater,
  the release script, the download page. None of it changes what the runner writes, what the
  engine computes, or what the routine reads.
- **Plan 4b — the rest of friends (later):** whatever the first friend installs reveal, plus the
  pieces that need plan 3 (a friend's vault has no cloud routine, so judgment must be local before
  Gmail or enrichment can be offered to them) and Quinn's lead-time accounts once they exist.

Plan 3 keeps its place otherwise; its spec is written while 4a is built.

**What a friend gets after 4a, honestly:** a signed installer; a wizard that makes a vault from
nothing, connects the LMS calendar feed, stores coursework logins in Credential Manager, picks a
backup folder and the two daily slots; the tray app running coursework and rank from those slots
on their own machine; the console over their vault; automatic updates. **What they do not get
yet:** anything the cloud routine does for Quinn today — Gmail proposals, event verdicts,
enrichment. Those arrive with plan 3. The wizard says so in one sentence rather than pretending.

---

## 1. Scope

**In (runner-independent, all in `app/` plus a release script and a static site):**

1. Profiles — a registry, a per-profile app-data layout, and the migration of today's flat layout.
2. Onboarding — the first-run wizard that produces a working vault or adopts an existing one.
3. The settings panel — backup folder, autostart, profile name, updates, diagnostics.
4. Credentials — the app writes Windows Credential Manager entries the engine already reads.
5. The installer, the updater and the release script.
6. The download page and the release manifest.

**Out (gated, or another plan):** `scheduler: app` on Quinn's live vault, the runner script's
removal, rename stage 2 of crates and binaries, coursework run records and gauge counts (plan 2
Part B); local models, Gmail, event verdicts, the routine's retirement, the remote's removal,
rename stage 3 (plan 3); telemetry (needs a backend and a policy — not before plan 4b); billing;
a second device.

**Never here:** a hosted account, a cloud API, a write to the live vault before G2, a change to
`engine/` or the cloud routine, any name on the never-rename list (Knowlu spec §7).

---

## 2. Profiles

**A profile is a vault plus its app data.** The vault stays where the user put it (Knowlu spec §4:
the vault stays files, one folder per profile). App data moves from one flat folder to one folder
per profile:

```
%LOCALAPPDATA%\knowlu\
  profiles.json                      the registry
  profiles\<profile_id>\settings.json
  profiles\<profile_id>\seen.txt
  profiles\<profile_id>\logs\        slot-*.log, quit-*.txt
  updates\                           the updater's downloads
```

`profiles.json` is a list of `{ "id", "name", "vault", "created_at", "last_opened_at" }`; `id` stays
`ids::derived_id("profile", <vault path>)` as today, so a vault opened by path and the same vault
registered by the wizard are the same profile. `name` is what the picker shows; it defaults to the
vault folder's name and is editable in settings. Written through `ledger::dumps_value` like every
JSON the app writes.

**The root moves to `knowlu` here, not in plan 2 Task 12.** The Knowlu spec's F14 rule (a move, not
a fresh start) is honoured by this plan's migration — and **what moves is our own files, never the
directory** (**R-P4a-28**, the final review's CRITICAL). On first launch of a build with profiles,
the flat `settings.json` and `seen.txt` are folded under `profiles\<profile_id>\` (the id is in the
old settings file), the app's own `slot-*` and `quit-*` logs are moved out of `logs\` one file at a
time, and `profiles.json` is written with that one profile. Everything else in
`%LOCALAPPDATA%\quinn-ops\` is left alone — `dual\`, `rehearsal\`, `scratch\`, `shots\` and the logs
the runner scripts write there belong to those scripts, and on Quinn's laptop they are a live
cutover week's evidence trail; the old root is removed only if the fold left it completely empty.
Plan 2 Task 12's step becomes
"verify the move happened; do nothing" — recorded in the plan-4a plan's fidelity ledger and in
Task 12's brief when it is dispatched. `app_data_root()` stays the one place the path is decided.

**Launch resolution.** `--vault <path>` keeps working exactly as today and is how the Start-menu
shortcut and autostart launch Quinn's profile (autostart re-registers with the chosen profile's
`--vault`). Without `--vault`: zero registered profiles → onboarding; one → open it; more than one →
the picker (a small window listing names and vault paths, with *Use an existing vault…* and
*Create a new vault…* below them — §0.0, R-P4a-22). ~~One instance per profile: the single-instance
plugin's key includes the profile id, so two profiles can be open in two tray icons and the same
profile never twice.~~ **Withdrawn (§0.0, R-P4a-1): one Knowlu per machine; a profile switch is a
relaunch with `--after-pid`.**

**No single-user assumption is re-introduced:** nothing in the registry, the layout or the tests
names a path, a machine or a person; the fixtures are scratch copies.

---

## 3. Onboarding

**The wizard runs inside the main window before the console is shown**, as a sequence of panels in
the same page (no second webview, no bundler — the page rules of the console spec §7 hold: no
network references, no `import`, every string in the page or from the engine). It ends by opening
the console over the new profile. Back is always allowed; nothing is written to disk until the last
panel's *Finish*, except Credential Manager entries, which are written when the user leaves the
credentials panel so a crash never leaves a password in page memory longer than needed.

The panels, in order:

1. **Welcome.** One paragraph: what Knowlu does, and the privacy sentence — *everything stays on
   this machine; Knowlu has no account and sends nothing anywhere; the only network calls are to
   the sources you connect and to check for updates.* A link to the same text on the site. No
   telemetry toggle: there is no telemetry.
2. **Your vault.** Three choices: **Create a new vault** in a folder the user picks (default
   `%USERPROFILE%\Documents\Knowlu\<name>`); **Use an existing vault** (a folder with `config/` and
   `tasks/`; this is how Quinn's own vault becomes a profile); **Restore from a backup** (a folder
   with `<profile>\vault\`; the mirror is copied to the chosen vault location, never used in place —
   Knowlu spec §4 "point at a backup folder and its mirror becomes the vault"). Adopting or restoring
   skips panels 4–6 when the vault already has the config.
3. **Backup folder.** A native folder picker; the amber-not-blocking rule is stated (*if the
   drive is unplugged Knowlu says so and carries on*). Skippable, with the consequence shown.
4. **Your LMS calendar.** The Blackboard / Canvas ICS feed URL, with the two three-step how-tos
   from the market doc §4.4 (Blackboard: Calendar → Settings → Share Calendar; Canvas: Calendar →
   Calendar Feed). Pasted URL is validated by shape (`https://`, ends in `.ics` or contains
   `/calendar/`), never fetched here (no live fetch in onboarding; the first slot fetches). This is
   connector #1 by the market doc's ruling; no Canvas token, no Blackboard REST.
5. **Coursework logins (optional).** zyBooks and VHL email + password fields. On *Next* the app
   writes `knowlu/<profile_id>/zybooks` and `knowlu/<profile_id>/vhl` in Credential Manager
   (§5) and clears the fields. The vault's `config/ingest.yaml` gets `credential_target:` lines
   naming those targets, which is all the engine reads (`wincred::read_credential`). A friend with
   neither course skips.
6. **Slots and campus.** Timezone (default: the machine's), two slot times (default 12:00 and
   18:00), autostart on/off (default on), and a **campus preset**: *University of Alabama* (the
   six event sources from Quinn's `config/events.yaml`, hosts only, no secrets) or *none*. The
   preset is a file embedded in the app; adding a campus is adding a file.
7. **Finish.** Writes everything (§3.1), registers the profile, launches the console.

### 3.1 What *Finish* writes for a new vault

A scaffold, embedded in the app as plain files, materialised into a temp folder beside the target
and renamed into place (a crash never leaves a half-vault):

```
<vault>\config\ingest.yaml         ics_url (panel 4), timezone (panel 6), course_map: {},
                                   calendars: [] , coursework: { zybooks/vhl blocks with the
                                   credential_target lines, enabled only if panel 5 filled them }
<vault>\config\planning.yaml       daily_effort_budget: 4.0, slice_hours: 2.0,
                                   daily_approval_budget: 15, recurring: []
<vault>\config\week_template.yaml  day_start/day_end/min_block_minutes defaults, classes: {}
<vault>\config\runners.yaml        runners: [{ name: local, times: [panel 6], tz: panel 6,
                                   grace_minutes: 20, device: <this machine>, scheduler: app }]
<vault>\config\events.yaml         the campus preset, or sources: []
<vault>\tasks\  approvals\  archive\  courses\  issues\  info\  state\  state\journal\
<vault>\.gitignore                 state/.sync.lock, state/events-ui/ (as plan 1's history expects)
```

Then two engine calls through the library, in this order:

- `write::create` of `archive/_migrated.md` (`status: archived`) with
  `WriteContext::new("system:migration", "cli")` (**R-P4a-6** — `via: "onboarding"` as this
  document first wrote it is struck: the journal's `via` vocabulary does not grow for this) — the
  same record shape
  `scripts/migrate_s1.py` left in Quinn's journal, so `detect_external`'s unmigrated guard
  (`passes.rs`: any `create` record whose actor starts with `system:migration`) is satisfied from the
  vault's first day and the friend never sees `journal has no migration records`. This is the one
  app write that does not carry `console_ctx()`, and it is written exactly once per new vault.
- `write::create` of a first task, *Get to know Knowlu*, with `console_ctx()` — so Today is not
  empty and the first `rank` has something to order. Adopted and restored vaults get neither.

**`scheduler: app` on a fresh vault is not a violation of plan 2's rule.** That rule ("written
exactly once, by Task 9") protects Quinn's live vault, which has a script runner to stand down. A
vault born in the wizard never had one; the app is its only runner, and `device:` names the machine
that made it, so a second install of the same vault elsewhere still stands down (Knowlu spec F3).
Recorded as decision 4 below.

**No live fetch in onboarding.** Nothing is fetched until the first slot fires (Global constraint
from plan 2, and Google rate-limits the calendar feed). The wizard's last panel says when the
first slot is.

---

## 4. The settings panel

Reached from the console's topline gear and the tray menu. One panel in the page, backed by the
existing `get_settings` / `set_settings` commands (plan 1) plus two new ones:

| Row | Backed by | Notes |
|---|---|---|
| Profile name | `profiles.json` (`set_profile_name`) | picker label only |
| Vault path | read-only | copy button |
| Backup folder | `set_settings { backup_dir }` + a native folder picker (`pick_folder`) | *Back up now* beside it, the existing command |
| Autostart | `set_settings { autostart }` | re-registers with `--vault` |
| Updates | `check_for_updates` (§6) | shows version, last check, staged-or-not and the last error, plus *Check now*; *Restart to update* when one is staged. **No channel** — there is one (§0.0, M8) |
| Diagnostics | the existing Copy diagnostics | unchanged |

**Not in the panel (YAGNI, revisit in 4b):** slots and timezone, sources, campus preset, course map.
Those are the wizard's, and a *Reconfigure…* button that re-enters the relevant wizard panel for an
existing profile is the 4b shape — cheaper than a second editor for the same fields. Settings that
live in the vault's `config/*.yaml` are edited by line surgery (`apply_frontmatter_fields_to_text`'s
sibling for plain YAML), never by re-dumping a file — the crate's one-emitter invariant.

The native folder picker is **`rfd 0.16` called directly**, not `tauri-plugin-dialog` — the plugin
is unusable on this toolchain (§0.0, R-P4a-20). It is the same `IFileOpenDialog`, parented to the
window, and it needs no capability entry because it is not a plugin.

---

## 5. Credentials

`app/src/credentials.rs`: `write(target, user, secret)`, `delete(target)`, `exists(target)` over
`CredWriteW` / `CredDeleteW` / `CredReadW` from the `windows` crate the app already depends on
(`Win32_Security_Credentials` feature added). The engine keeps its read-only `wincred.rs`
unchanged and keeps reading whatever `credential_target` the vault names. Targets are
`knowlu/<profile_id>/<source>` so two profiles on one machine never share a login.

Rules, unchanged from every earlier document: a secret is never logged, never in a run record, a
backup, a fixture, a test name or a plan; the page clears its fields the moment the write returns;
tests use a throwaway target under `knowlu/test/…` and delete it. Quinn's own `quinn-ops/zybooks`
and `quinn-ops/vhl` entries are untouched by this plan — their re-store under `knowlu/…` with the
owed rotation is plan 2 Task 12 step 6 and is Quinn's, from a non-Claude terminal; adopting Quinn's
vault as a profile leaves its `credential_target` lines as they are.

---

## 6. Installer, updater, release

Everything the Knowlu spec §6 decided, made concrete:

**Bundle.** `bundle.active: true`, target `nsis`, `windows.webviewInstallMode: embedBootstrapper`,
`bundle.externalBin: ["binaries/quinn-ops"]` — the engine exe travels as a Tauri sidecar and lands
beside `knowlu.exe`, which is where `scheduler::engine_exe()` already looks. The sidecar's name
follows plan 2 Task 11's rename (`knowlu-engine`) when that lands; the release script names it in
one place. Identifier `com.knowlu.desktop` is set here (the installer bakes it in; changing it after
the first friend build would orphan their install) — plan 2 Task 12's identifier step becomes a
check. Version `0.1.0` in one place, the app config; the engine's build SHA stays in run records.

**Build.** `cargo install tauri-cli --locked` once on the laptop (it is not installed today); `cargo
tauri build` from `app/` after `cargo build --release` at the root has produced the sidecar. The
GNU toolchain question is answered by a spike task before anything depends on it: NSIS is
downloaded by the CLI and needs no MSVC; if the bundler refuses the GNU target the plan says what
to do (a plain `makensis` script over the same inputs).

**Updater.** `tauri-plugin-updater` with endpoint `https://knowlu.com/releases/latest.json`, the
public key in the app config, the **private key generated by Quinn** with `cargo tauri signer
generate` and stored in Credential Manager as `knowlu/updater-key` (never a file, never the repo —
Knowlu spec decision 15). The app checks on launch and once a day while resident, downloads in the
background, and offers *Restart to update* in the topline and the tray; **never mid-run** — the
offer is deferred while `Scheduler.running` is true. A failed signature is refused and shown, not
retried silently. Until the site exists the endpoint is unreachable and the check fails quietly
once a day; that is the correct behaviour and a test pins it.

**Signing.** `scripts/release.ps1` (PowerShell 5.1): bump check, root release build, sidecar copy,
`cargo tauri build`, then `signtool sign` against Quinn's Azure Trusted Signing profile when the
profile file exists and skips with one loud line when it does not, then writes `latest.json` and
puts the three artefacts in `site/releases/`. Signing secrets never enter the repo; the script runs
under Quinn's own Azure login. The Windows SDK's `signtool` is not on this laptop today — the
script names where to get it.

**Release location.** Uploaded from the laptop to Cloudflare Pages by hand until the account exists
and the code is split out (Knowlu spec §6, §7 stage 3). This repository never becomes public as
part of this plan.

---

## 7. The site

`site/` at the repo root: `index.html` (what Knowlu is, the download button, the privacy paragraph
from the wizard, *Copy diagnostics* as the feedback path, the version), `releases/latest.json` and
the artefacts the release script drops there, a `privacy.html` carrying the same paragraph and the
data list (what is stored, where, what leaves the machine: nothing but source fetches and the update
check). Plain HTML and CSS, the console's tokens, no scripts. Deployed to Pages when Quinn's
account exists; until then it is a folder that opens in a browser.

The product plan §8's legal items (scraping ToS, minors, FERPA) are not resolved by a privacy page;
they stay Quinn's external dependency and the page makes no claim about them.

---

## 8. Testing

Everything against scratch copies and temp folders; no live vault; no network.

- **Profiles:** registry round-trip; the flat→per-profile migration from a planted
  `%LOCALAPPDATA%`-shaped temp root (settings, seen stamp and logs survive; F14); launch resolution
  for zero/one/many profiles; `--vault` unchanged; the single-instance key per profile.
- **Onboarding:** the scaffold materialises atomically (a killed write leaves no partial vault); a
  scaffolded vault passes `quinn_ops::cli::run_with(rank, no fetchers)` with only the two expected
  warnings (`no ics_url` when panel 4 was skipped; nothing when it was not) and no `journal has no
  migration records`; adopting `tests/fixtures/vault-s1` as a profile writes nothing into it;
  restore copies a planted mirror and never writes into the backup; the campus preset produces the
  same `events.yaml` shape `load_events_config` reads today.
- **Credentials:** write / exists / read-through-the-engine / delete on a `knowlu/test/…` target;
  the secret never appears in any log the test can read.
- **Settings panel and wizard, headless:** Playwright in `.wv` over a fake `__TAURI__`: each panel
  renders, Back/Next order, validation of the ICS URL shape, fields cleared after the credential
  write, the panel's commands invoked with the right payloads; `console-shots.py` gains the wizard
  panels and the settings panel.
- **Updater:** the mid-run deferral (a fake staged update while `running` is true is not offered);
  an unreachable endpoint yields one quiet log line and no dialog; a bad signature is refused.
- **Static assets:** the existing rules (no `http://`/`https://` under `app/static/`; the updater
  endpoint lives in the app config, not the page).
- **Installer:** the spike's output (a real `.exe` built on this laptop, installed into a scratch
  user folder is out of scope — the check is that the installer builds, is the right size, and its
  contents list both binaries and the bootstrapper).
- Root `cargo test`, both oracles, `cd app; cargo test` at zero new warnings, as always. No `src/`
  change is expected in this plan; if one is needed the dual-run scripts run before the claim.

---

## 9. Plan 4a tasks (outline; the plan document carries the steps)

1. Spike: `tauri-cli` on the GNU toolchain — `cargo tauri build` with `bundle.active: true` and
   the sidecar; the answer decides Task 9's shape. Half a day, throwaway allowed.
2. Profiles: registry, per-profile layout, the flat-layout migration, launch resolution, the
   picker window.
3. Credentials module and its tests.
4. Scaffold and campus preset (embedded files), the seed and first-task writes, the atomic
   materialisation, the rank-passes test.
5. The wizard panels and their commands (`create_vault`, `adopt_vault`, `restore_vault`,
   `store_credentials`, `pick_folder`, `finish_onboarding`).
6. The settings panel (`set_profile_name`, `pick_folder`, the rows), the topline gear, the tray
   item.
7. Updater plugin, the mid-run deferral, `check_for_updates`, the topline/tray offer.
8. Bundle config, identifier, sidecar, bootstrapper; `scripts/release.ps1` with the signing step
   that skips loudly.
9. The site.
10. Close: README, anatomy, HANDOFF, the Knowlu spec's §9 note, this document's status line.

Quinn's moments, asked one at a time when reached: the updater keypair (Task 7); the Cloudflare
account (Task 9's deploy step, not blocking the folder); Trusted Signing progress (Task 8's
signing step, not blocking the build); a first friend's name and machine for the install rehearsal
(after Task 8).

---

## 10. Decisions this document makes

| # | Decision | Source |
|---|---|---|
| 1 | Plan 4 splits; **its shell half runs now, before plan 3** — the Knowlu spec's §9 order is amended for plan 4a only. In the same conversation the cutover's reverse week was cut to **three days / six executed runs** (recorded in the cutover plan, not here) | Quinn 2026-09-05 |
| 2 | A profile is a vault plus per-profile app data under `%LOCALAPPDATA%\knowlu\profiles\<id>\`; the root moves here (F14 honoured), plan 2 Task 12's move becomes a check | this doc §2 |
| 3 | The wizard writes nothing until Finish, except credentials at the moment they are entered | this doc §3 |
| 4 | A vault born in the wizard gets `scheduler: app` and `device:` from birth; plan 2's "written once by Task 9" rule is about Quinn's live vault only | this doc §3.1 |
| 5 | The unmigrated guard is satisfied by one `system:migration`/`cli` create record of an archived seed note — the S1 migration's own shape, no engine change (amended from `via: "onboarding"` by **R-P4a-6**) | this doc §3.1 |
| 6 | Credential targets are `knowlu/<profile_id>/<source>`; the app writes them, the engine keeps reading by name; Quinn's existing targets are untouched | this doc §5 |
| 7 | The engine ships as a Tauri sidecar (`externalBin`), landing beside `knowlu.exe` where the scheduler already looks | this doc §6 |
| 8 | Identifier `com.knowlu.desktop` is set by this plan (the installer bakes it in) | this doc §6; Quinn 2026-09-05 (domain) |
| 9 | Settings panel holds five rows; slots/timezone/sources stay the wizard's, re-entered via *Reconfigure…* in 4b | this doc §4 |
| 10 | No telemetry, no toggle, until there is a policy and a backend (4b at the earliest) | product plan §7; this doc §3 |
| 11 | The site is plain HTML/CSS, no scripts; deployed by hand until the Cloudflare account exists | this doc §7 |

## 11. Open items — Quinn's, asked individually when reached

| Item | Needed by |
|---|---|
| Updater keypair (`cargo tauri signer generate`, private half into Credential Manager `knowlu/updater-key`) | Task 7 |
| Cloudflare account (Pages for the site and manifest; R2 later for models) | Task 9's deploy — the folder is built regardless |
| Azure Trusted Signing certificate profile (started 2026-09-04) | Task 8's signing step — the build runs unsigned until then |
| Windows SDK `signtool` on the laptop | Task 8 |
| The first friend: name, machine, which LMS | after Task 8 |
| Legal read on scraping ToS and minors (product plan §8) | before the pilot widens beyond friends |

## 12. Rulings preserved

No single-user assumption (CLAUDE.md rule 2); every console write through `quinn_ops::write` with
`console_ctx()` (the one exception is decision 5, a system record); the read model never writes;
the scheduler inert unless `scheduler: app` (decision 4 applies it to fresh vaults by the same key);
the app never builds and never runs the passes itself; credentials never in the repo, a log, a
backup rule, a fixture or a plan; desktop safety (no synthetic input; `PrintWindow` from a DPI-aware
process only; headless Playwright for page checks); the never-rename list; the eight Python-written
references and the three surface references untouched; the live vault written only by Knowlu
launched from the shortcut after the go-live checklist; the console stays on scratch copies until
G2; nothing here moves G2, `$mode`, or the routine.
