# Two desktops on one account, plan 2 of 3: the account holds the shared settings

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every computer on one account plans the student's day from the same settings: the six shared
`config/` files travel through the account whole, each computer's own keys live in a never-synced
`config/device.yaml`, no change to a shared file is ever lost in silence, a new computer's defaults never
override a working computer's settings, and a second computer's setup asks only for its own logins.

**Architecture:** A new engine module, `config.rs`, owns D17's split: the device-first readers
(`overlay`), `config/device.yaml` written from literals and grown by line insertion, the push guard's
test (`computer_only_value`) and a text-only migration (`move_device_keys`) that `sync` runs on every run
until done. `sync.rs` sends the six files as `sync_notes` rows carrying a `base`, takes them back by a
three-way check (a replaced local text is kept under `state/config-conflicts/`), re-inserts card-made
mappings after any replacing pull, and holds a new computer's files **provisional** (a cursor mark with
three endings). The cloud gains one migration (the widened path check and `save_config_row`, one
compare-and-set), `sync-push` routes settings rows to it and answers `config_refused`, and `sync-pull`
gains a `view=settings` read. The app births split vaults, reads its own device keys first, and its
wizard asks `account_settings_exist` after sign-in and skips every panel the account already answers.

**Tech Stack:** Rust 1.98 (`stable-x86_64-pc-windows-gnu`), one Cargo workspace (`engine/` =
`knowlu-engine`, `app/` = `knowlu`), `serde_yaml_ng`, `serde_json`, `jiff`, `ring` (for `sha256_hex`);
Deno 2 for `cloud/supabase/`; plpgsql for the one migration; the page's plain JavaScript
(`app/static/console.js`); Python + Playwright for `scripts/wizard-check.py` (a hand-off).

**Spec:** `docs/specs/2026-09-25-two-desktop-design.md`, **SIGNED by Quinn 2026-09-25**. This plan carries
D17–D19 — §4.8 whole: the split, the push guard, the migration, whole-file settings sync with the
three-way check, `save_config_row`'s compare-and-set, `state/config-conflicts/`,
`apply_approved_mappings`, provisional settings and their three endings, Q13, and D19's second-computer
wizard — with §6.1's shared-config bullets, §6.2's wizard walk and §6.3's `sync-push`/`sync_rows.ts`
bullets and function pin. Where this plan and the spec disagree, the spec wins and this plan is wrong;
every place the spec is silent, or cannot be carried as written, is a **Plan ruling** below
(R-TD2-*), repeated at the step it changes.

**Status: PLAN, written 2026-09-26 on branch `two-desktop` (Plan 1 committed as `4374142`, its fix round
as `4ebc7d6`, its execution under way: Tasks 1–2 at `3010cdc`). Not executed.** Plan 1 executes first;
every task here builds on its *Interfaces this plan produces* as `4ebc7d6` states them.

**Checked while writing:** every task's code and tests, with hand-offs H1–H4, were applied in order —
one task at a time, by a script that also rendered this text's code blocks, so the two cannot differ — to
a scratch copy of the tree with Plan 1 applied (before its fix round; never this worktree). Every
`file:line` below was read in that copy; earlier tasks, and Plan 1's own execution, shift the lines, so a
step finds its place by the code it quotes, and every edit is written as the exact text it replaces.
Each task ended green: `cargo test -p knowlu-engine` whole at 0 warnings for Tasks 1, 2, 4, 5 and 6 (one
test that needs a git checkout skipped), the whole `cloud/supabase` Deno suite with `deno check` and
`deno lint` for Task 3, and `cargo test --workspace` whole (engine and app, the one accepted `.rsrc`
linker line and nothing else) for Tasks 6, 7 and 8, plus `scripts/wizard-check.py` for Task 8. Every new
test was also run first against the tree without its task's code and seen failing (*RED and GREEN, as
run while writing*, at the end). Two edits anchor away from lines Plan 1's fix round also changes
(`RestoreState`, `materialise`'s write arm), so they apply to Plan 1 before and after `4ebc7d6` alike;
and Tasks 1, 2, 3, 7 and 8, applied as written to the real branch head at `3010cdc`, found every text
they quote exactly once.

## Global Constraints

Binding on every task. Where a line quotes `CLAUDE.md`, it is verbatim.

1. **Where.** Work only in `C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop`, on branch
   `two-desktop`. Absolute paths and `git -C <worktree>`; **never `cd`**. `.superpowers/` is git-ignored
   scratch.
2. **0 warnings.** "`cargo build --workspace` and `cargo test --workspace` from the root. **0 warnings is
   part of green.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple
   non-default manifests` linker message." The CI gate prints `warnings: N accepted (.rsrc), N tallies,
   N other`; the last number must be `0 other`.
3. **Frozen references.** "**Never regenerate a frozen reference.** Eight Python-written references live
   in `engine/tests/fixtures/`: `golden-today-s1.md`, `golden-today-full.md`, `calendar-snapshot-gcal.md`,
   `vault-full/state/events.md`, `zybooks-parsed-reference.json`, `vhl-parsed-reference.json`,
   `run-records-reference.json`, `pyyaml-safe-dump-reference.json`. If the engine disagrees with one, the
   engine is wrong. The three Rust-generated read-model references — `surface-today-{s1,s1-migrated,full}.json`
   (`engine/tests/surface_oracle.rs`) — have their own rule (console spec §4.6): regenerate only in a commit
   whose diff shows the change and whose message says why." **No task changes any of the eleven**:
   `oracle.rs` and `surface_oracle.rs` stay green and unedited. The fixture vaults carry no
   `config/device.yaml`, and every reader falls back to the shared file when it has none.
4. **Line endings.** "**Line endings: LF everywhere in this repo** (`.gitattributes`: `* text=auto
   eol=lf`; `*.ps1` are CRLF). `engine/tests/fixtures/**` is `-text`: those bytes are the contract —
   several are compared byte for byte and they are CRLF because vaults are — **never re-encode them**." No
   task writes under `engine/tests/fixtures/`. Vault files are read with `pystr::read_text` and written
   with `pystr::write_text`, so a settings text travels with `\n` and lands in the vault's own terminator.
5. **Never rewrite a vault file wholesale — and the second recorded exception.** "Never rewrite a vault
   file wholesale. Every note write goes through `write` ... **No note is ever parsed and re-dumped**."
   This plan adds **the second recorded exception** (spec D18): `sync::apply_settings` writes the
   account's text over one of the six shared settings files, verbatim, and a restore's `materialise`
   writes a settings row over the wizard's file — bounded to `sync::SHARED_CONFIG` paths and the
   account's own row. Every other config write is a line insertion or a line deletion
   (`coursework::insert_mapping`, `config::move_device_keys`, `config::insert_source_key` and its three
   siblings), or a brand-new file written from literals (`config/device.yaml`); no config file is ever
   parsed and re-dumped. No note write changes. Hand-off H5 names both exceptions in `CLAUDE.md`.
6. **Contracts.** "All JSON the crate writes goes through `ledger::dumps_value`." The two new `Cursor`
   fields go through `save_cursor`. `config/device.yaml`'s literal shape (spec §4.8) is a new contract
   with every vault the wizard makes: `config::device_yaml` is its one writer.
7. **`rank` never calls a model** (Knowlu spec decision 11). This plan does not touch `cli.rs`.
8. **Plan 1 comes first.** Its fix round (`4ebc7d6`) removed `sync::Totals::offline`, moved
   `sync::pull_event_verdicts` into `judge` (it takes `sync`'s run lock itself) and gave `RestoreState` an
   `undone` field. No task here reads `Totals::offline`, calls `pull_event_verdicts`, or anchors an edit on
   a line that round changed.
9. **Determinism.** Same input, same order: `SHARED_CONFIG` is sorted, every map is a `BTreeMap`, the
   migration walks the files and their keys in file order, and the conflict file name is a UTC stamp.
10. **`sync.rs`'s own text guard.** `engine/tests/dependency_boundary.rs::
    the_sync_module_holds_no_key_and_no_envelope` fails if `engine/src/sync.rs` contains any of `SyncKey`,
    `recovery`, `Recovery`, `seal(`, `fn open(`, `IndexKey`, `HKDF`, `hkdf`, `aead`, `CROCKFORD` — in code
    or comments. Nothing added to `sync.rs` uses those words.
11. **Credential Manager.** "**Tests that touch the real Credential Manager are serialised.**" Task 8's
    two `create_vault_in` tests hold `app/tests/onboarding.rs`'s `PendingSession` (its `CREDMAN_LOCK`) like
    every neighbour, and drop one before taking the next; `account_settings_exist_at`'s test touches none.
    Every cloud call is a `127.0.0.1:0` loopback. The lock is in-process only: `knowlu/pending/session` is
    one fixed target on the machine, so an `onboarding.rs` run in another worktree at the same time races
    it (`no session at knowlu/pending/session to move`; Plan 1's R-TD1-exec-8). That failure is
    environmental, never this plan's: check `Get-Process onboarding-*` is empty and re-run
    `--test onboarding` alone.
12. **`KNOWLU_DEVICE` is process-global.** No test sets it.
13. **Tests run in the foreground**, one cargo at a time, with `-j 2` (host memory is low). Never
    `cargo test --release`. Before any cargo or deno command in a fresh shell, refresh PATH:
    `$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"`
14. **TDD.** In every task the failing test is written first, run, and seen failing for the stated reason;
    then the code.
15. **Hand-off files are not edited by a task.** `engine/src/{lib,ingest}.rs`, `app/src/main.rs`,
    `scripts/wizard-check.py`, `CLAUDE.md`, `HANDOFF.md`, `Cargo.toml` and `Cargo.lock` are shared
    single-owner files (`HANDOFF.md` §2). Their changes are hand-offs **H1–H5** (*Controller hand-offs*),
    applied verbatim by the controller as their own commit at the task that names them. A task that
    silently edits one is a plan defect: stop and report it.
16. **No push, no `supabase` command, no `knowlu.exe`.** The controller deploys Task 3's migration and two
    functions to staging after Task 3, and **to production before any release that carries Task 4** (the
    rollout order in *Hand-off to the controller*, item 2: a server without the widened path check
    refuses a settings row with a 400, and a push is all or nothing).
17. **Commits.** `git -C <worktree> add <paths>` (never `-A`), the message written with the Write tool to
    `C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-N.txt` (Git Bash
    heredocs halve backslashes), then `git -C <worktree> commit -F <that file>`. Every message ends with the
    two trailers the controller's dispatch names; this plan's own commit used
    `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>` and
    `Claude-Session: https://claude.ai/code/session_0116SUa7sU6BMt66hiEXrjTU`.
18. **No personal data.** Invented names; accounts `acct-1`; hosts `127.0.0.1` and `*.invalid`.
19. **The whole suite at the end of every task**, three commands in this order, each with
    `--manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml`:
    `cargo test --workspace -j 2 --no-run`, then `cargo build -p knowlu-engine -j 2`, then
    `cargo test --workspace -j 2` — green at 0 warnings. Why the order: building the app's tests copies
    `app/binaries/`' zero-byte placeholder sidecar over `target\debug\knowlu-engine.exe`, and five test
    files spawn that exe (`oracle.rs`, `surface_oracle.rs`, `entitlement_gate.rs`, `rank_no_digest.rs`,
    and `onboarding.rs`'s discover test) — so the engine is rebuilt after the app, and the last command
    then builds nothing. Task 3 adds the whole Deno suite, Task 8 the wizard walk.

## Spec map

| Spec | Where it lands |
|---|---|
| §4.8 *Why*; D17's table — the shared files and `config/device.yaml`'s keys | Task 1 (`config::SHARED_CONFIG`, `DEVICE_FILE`, `PORTAL_SOURCES`, `SOURCE_DEVICE_KEYS`, `RUNNER_DEVICE_KEYS`) |
| D17 — `config/device.yaml`'s literal shape, written from literals, `{}` for an empty source | Task 1 (`config::device_yaml`, `write_new_device_yaml`); Task 7 (`scaffold::device_yaml` births it) |
| D17 — readers take a device key from `device.yaml` first; `calendars:` is the union | Task 1 (`config::overlay` in `coursework::load_coursework_config`, `calfeed`, `runs::local_runner_settings`; hand-off **H2** for `ingest`); Task 7 (`scheduler::mode`, `device_ok`, `ics_state`) |
| D17 — `config::move_device_keys`: gather, write `device.yaml` whole (temp + rename), then delete lines; runs at the start of every `sync` until done; a crash between is harmless | Task 2 (the function), Task 4 (`sync` runs it) |
| D17 — later additions insert lines under their known parent in `device.yaml` | Task 2 (`insert_source_key`, `insert_runner_key`, `set_device_ics_url`, `add_device_calendar`) |
| D17 — the invariant: a capability URL never enters a synced file; `restore_capability_url` writes `device.yaml` | Task 2 (the guard's test), Task 4 (`build_push` applies it), Task 7 (`restore_capability_url`) |
| D17 — the wedge guard and its line | Task 2 (`config::computer_only_value`), Task 4 (`build_push`) |
| D18 — `sync::SHARED_CONFIG`; one `sync_notes` row per path | Task 4 |
| D18 — server: the widened path check, `sync_rows.ts`, no tombstone for a settings path | Task 3 |
| D18 — the conditional push: `base`, the hash convention, `save_config_row`, `config_refused`, the device's line | Task 3 (server), Task 4 (device) |
| D18 — push: sends a shared file whose hash differs from the cursor's; `Cursor.notes` records last received and last sent; never tombstoned | Task 4 (send), Task 5 (received) |
| D18 — pull: the three-way check, no base yet (Q13), the conflict branch, `state/config-conflicts/` | Task 5 (`sync::apply_settings`) |
| D18 — `apply_approved_mappings` after every pull that replaced a settings file | Task 5 |
| D18 — rollout warnings on an old build | Nothing to build (an old build's own refusal); *Hand-off to the controller*, item 2 |
| D18 — the second recorded exception | Global Constraint 5; hand-off **H5** (`CLAUDE.md`) |
| D18/D19 — provisional settings: the mark in the cursor; never pushed while marked | Task 4 (the fields and the hold), Task 6 (setting it) |
| D18 (a) the account's settings arrive by pull | Task 5 (`apply_settings`' provisional branch) |
| D18 (b) the student edits a setting in the app | Task 6 (`sync::settings_edited`, the seam — R-TD2-11) |
| D18 (c) seven days | Task 6 (`sync::publish_if_due`) |
| D18 — automatic writes never end the mark | Task 6 (its test) |
| D19 — `onboarding::account_settings_exist`; the window's commands 29 → 30 | Task 8; hand-off **H3** (`app/src/main.rs`) |
| D19 — settings exist: skip the answered panels, logins only, mapping rows only for an unmapped source | Task 8 (`console.js`, `create_vault_in`) |
| D19 — notes but no settings: every panel, answers provisional | Task 6 (`restore_all_as`, `restore_into_as`), Task 8 (the plan's `account_holds_notes`) |
| D19 — an account with no vault: nine panels, real answers | Task 6 (no mark), Task 8 (the default walk is unchanged) |
| Q13 — the first to sync after the upgrade provides the settings | Task 5 (the no-base branch) |
| §6.1 — no capability URL ever travels; every settings row scanned | Tasks 2 and 4 |
| §6.1 — the migration: the literal shape, twice a no-op, a crash between its steps | Task 2 |
| §6.1 — the three-way pull; a refused push becomes the conflict | Task 5 |
| §6.1 — `apply_approved_mappings` | Task 5 |
| §6.1 — the provisional mark (R3-I1): automatic writes, a pull, an app edit, seven days with the clock injected, a refused insert | Task 6 |
| §6.2 — the wizard walk: settings exist; notes but no settings | Task 8; hand-off **H4** (`scripts/wizard-check.py`) |
| §6.3 — `sync-push`/`sync_rows.ts`: six paths in, every other `config/` out, no tombstone, `base` only on a shared path, two pushes from one base | Task 3 |
| §6.3 — `migrations_test.ts`: the function pin **28 → 29** (`save_config_row`; Plan 3's `fetch_turn` makes it 30), revoked, the widened check pinned to the six paths | Task 3 |
| §3.3 — the privacy words (the settings sentences) | **Plan 3** owns the one privacy change; *Hand-off to the controller*, item 4 carries the exact sentences |
| D9–D16, §3, §4.1–§4.7, §5, §6.2's turn tests, §6.3's `/turn` and RLS guard, §6.4 | **Plan 3.** |
| D1–D8, §2 | **Plan 1.** |

## Fidelity ledger

Every sentence of §4.8 (and D17–D19) that asks for behaviour, and where it is carried. "Refined" means the
spec is silent there and a Plan ruling decides it; "departs" marks the three places the spec cannot be
carried as written (spec defects S-1 to S-3), each decided by a ruling.

| Spec requirement | Task | Faithful? |
|---|---|---|
| Shared: `ingest.yaml` less its device keys (`timezone`, `course_map`, the coursework mappings, `cloud:` calendars); `runners.yaml` less two keys; `events`, `campus`, `planning`, `week_template` whole | 1, 2, 7 | yes |
| `device.yaml`: each source's `enabled`, `credential_target`, `base_url`; the `ics_url` fallback; every non-`cloud:` calendar; the `local` runner's `device` and `scheduler` | 1, 2, 7 | yes |
| `config/cloud.yaml` is this computer's | 4 | yes: not in `SHARED_CONFIG`, so never sent |
| A capability URL never enters a synced file; `restore_capability_url` writes `device.yaml` | 2, 4, 7 | yes; the guard also refuses `http://` and an unreadable file (R-TD2-4) |
| `device.yaml` written from literals, never from a parsed value; `{}` for a source with no keys | 1 | yes |
| Readers: device first, old place only when `device.yaml` lacks it; `calendars:` the union; an unmigrated vault works | 1, 7 | yes |
| The wizard births a split vault | 7 | yes |
| The migration runs at the start of every `sync` until done; writes `device.yaml` whole (temp + rename), then deletes lines; no re-dump; a crash between is harmless | 2, 4 | yes; where it runs and what wins a duplicate are R-TD2-2, R-TD2-3 |
| Later additions insert lines under their known parent | 2, 7 | yes |
| The guard: device keys at their D17 position, non-`cloud:` calendars, any `https://`/`webcal://` outside `events.yaml`'s feeds; the spec's line | 2, 4 | yes (R-TD2-4) |
| `sync::SHARED_CONFIG`, one text row per path | 4 | yes |
| Server: the path check widened to exactly six paths; `sync_rows.ts` the same; `is_note_path` gains the list | 3, 4 | **departs** on the last clause: `is_note_path` stays the note rule and a separate `config::is_shared_config` carries the list (S-1, R-TD2-1) |
| `sync_rows.ts` refuses `deleted: true` for a shared path | 3 | yes |
| The row is `{path, body, base}`; `base` only on a shared path; the hash convention | 3, 4 | yes |
| The handler writes records and note rows as today, then each settings row through `save_config_row` | 3 | yes |
| `save_config_row(p_account, p_path, p_body, p_device, p_base) returns boolean`: one compare-and-set; plpgsql, not SECURITY DEFINER, revoked from `public, anon, authenticated`; bytes count toward the ceiling | 3 | yes; the function also refuses a non-settings path (defence in depth) |
| The reply gains `config_refused`; the device keeps text and base and prints the line | 3, 4 | yes; the reply also carries `config` (R-TD2-6) |
| Push sends a shared file whose hash differs from the cursor's; `Cursor.notes` records last received as well as last sent; never tombstoned | 4, 5 | yes |
| Pull: equal to base or missing → write verbatim, record base; local changed and account equals base → nothing; both changed or last push refused → conflict: account wins, local copied, the line | 5 | yes (R-TD2-7, R-TD2-8) |
| No base yet counts as changed here: account wins, local kept unless identical (Q13) | 5 | yes |
| A missing shared file comes back with the next pull | 5 | **refined**: made true by a settings read when a file with a base is missing (R-TD2-9, spec defect S-2) |
| Provisional: a computer born into an account with notes marks its files; the mark is state (`provisional_config`, `provisional_since`); never pushed while marked | 4, 6 | yes (R-TD2-12) |
| (a) the account's settings arrive by pull: replace, copy first when it differs, mark ends for every path | 5 | yes |
| (b) the student edits a setting in the app: mark ends for that file, pushed with an empty base; a hand edit is not an event | 6 | **refined**: the seam `sync::settings_edited`; no console command writes a shared setting today (S-3, R-TD2-11) |
| (c) seven days on this computer's clock: publish with empty bases, the line | 6 | yes; only once a pull has read the account to its end (R-TD2-10) |
| Automatic writes never end the mark | 6 | yes (its test) |
| `apply_approved_mappings` after every replacing pull; inserts only a key the file lacks; a deleted line comes back | 5 | yes; a zyBooks key the student moved to `ignore:` is also left alone (R-TD2-16) |
| D19: `account_settings_exist` after sign-in; yes only with a `config/ingest.yaml` row; 29 → 30 | 8 | **departs** on "one `/sync-pull` call": a paged pull cannot promise the row is on its first page, so the call is `/sync-pull?view=settings` (spec defect S-4, R-TD2-13) |
| Settings exist: skip subscribe-when-active, school, LMS, calendars, Gmail, slots and time zone; logins only; mapping rows only for a source the account does not map | 8 | yes; the name panel is skipped too, as §6.2's walk says (R-TD2-14) |
| Finish births a split vault of provisional defaults, and the restore replaces them | 6, 7, 8 | yes (R-TD2-17); the wizard's own mapping rows then go back in by insertion (R-TD2-15) |
| Notes but no settings: every question, answers provisional | 6, 8 | yes |
| An account with no vault: nine panels, real answers | 6, 8 | yes |

## Order and file ownership

| Task | Unit | Files it edits | Hand-off applied with it | Depends on |
|---|---|---|---|---|
| 1 | `config.rs`: the split's vocabulary, `device.yaml`'s writer, the device-first readers | `engine/src/config.rs` (new), `engine/src/{coursework,calfeed,runs}.rs`, `engine/tests/device_config.rs` (new) | **H1** (`engine/src/lib.rs`), **H2** (`engine/src/ingest.rs`) | Plan 1 |
| 2 | The guard's test and the migration | `engine/src/config.rs`, `engine/src/coursework.rs` (five helpers' visibility), `engine/tests/device_config.rs` | — | 1 |
| 3 | The cloud: the path check, `save_config_row`, the conditional push, the settings view | `cloud/supabase/migrations/20260926000200_shared_settings.sql` (new), `migrations/migrations_test.ts`, `functions/_shared/{sync_rows,sync_rows_test,sync_db}.ts`, `functions/sync-push/{handler,handler_test,index}.ts`, `functions/sync-pull/{handler,handler_test,index}.ts` | — | — |
| 4 | The push: six rows with their base, the guard, the refusals; `sync` runs the migration | `engine/src/sync.rs`, `engine/tests/sync_contract.rs` (two struct literals), `engine/tests/sync_settings.rs` (new) | — | 2, 3 |
| 5 | The pull: the three-way check, the conflict folder, `apply_approved_mappings`, the settings read | `engine/src/sync.rs`, `engine/src/coursework.rs`, `engine/tests/{device_config,sync_settings}.rs` | — | 4 |
| 6 | Provisional settings: the mark, its endings (b) and (c), the restore | `engine/src/sync.rs`, `engine/tests/sync_settings.rs` | — | 5 |
| 7 | The app births split vaults and reads its own keys first | `app/src/{scaffold,scheduler}.rs`, `app/tests/{scaffold,onboarding}.rs` | — | 2, 5 |
| 8 | D19: the second computer's wizard | `app/src/onboarding.rs`, `app/static/console.js`, `app/tests/onboarding.rs` | **H3** (`app/src/main.rs`), **H4** (`scripts/wizard-check.py`), **H5** (`CLAUDE.md`) | 6, 7 |

Strictly sequential: `sync.rs` is edited by Tasks 4–6, `config.rs` by 1–2, `coursework.rs` by 1, 2 and 5,
`device_config.rs` by 1, 2 and 5, `sync_settings.rs` by 4–6, `app/tests/onboarding.rs` by 7–8. Task 3
touches no Rust and could run beside 1–2, but the controller keeps one implementer per worktree. Each task
ends with a green workspace and one commit (two where a hand-off lands beside it: the controller's
hand-off commit first, then the task's).

## Plan rulings

Each is repeated at the step it changes. None departs from a signed decision; each decides something the
spec leaves open or cannot carry as written (the four defects are under *Spec problems*).

- **R-TD2-1 — `is_note_path` stays the note rule** (spec defect S-1). §4.8 says "`is_note_path` gains the
  list". It is the one predicate `apply`, `materialise` and `build_push` use for **records**, moves and
  tombstones as well as texts; widened, a pulled `set` naming `config/ingest.yaml` would reach
  `write_literals` and edit a config file by frontmatter surgery. So the six paths are
  `config::is_shared_config`, `sync::SHARED_CONFIG` is `config::SHARED_CONFIG` re-exported, and only the
  settings paths of the page (`apply_settings`), the push (`build_push`'s settings loop) and a restore's
  live rows (`materialise`) accept them. `apply`'s step 6 skips a settings row silently; a record or a
  tombstone naming one is refused as today.
- **R-TD2-2 — in the migration, a value `device.yaml` already holds wins**, the readers' own rule, and the
  duplicate line in the shared file is deleted. A shared value `device.yaml` lacks is inserted first. So a
  crash between the two steps, or a device key hand-added to a shared file later, ends with one value,
  the one the readers were already reading.
- **R-TD2-3 — where the migration runs**: at the top of `sync::run_lines_with_client`, both directions,
  after `run_lines_with`'s lock, account and session checks. A vault with no account is never split by
  `sync` (it pushes nothing, so nothing is at stake), and the fixture vaults — no account — are untouched.
- **R-TD2-4 — the guard's reach.** A device key at its D17 position (`coursework.<source>.enabled`,
  `.credential_target`, `.base_url` for any source; the top-level `ics_url`, even empty; the `local`
  runner's `device` and `scheduler`), a `calendars:` entry whose `ics_url` is a non-empty value that is not
  a `cloud:` marker, and any string value starting `https://`, `http://` or `webcal://` except
  `config/events.yaml`'s `sources[<n>].url`. `http://` is added to the spec's two schemes (a capability URL
  can arrive in it), and a file that does not parse as YAML is refused too, because it cannot be proved
  clean. Lines: `sync: config/<file> holds a computer-only value (<key>); it stays on this computer until
  it is moved` (the spec's) and `sync: config/<file> is not readable YAML; it stays on this computer until
  it is fixed`.
- **R-TD2-5 — settings rows ride the wire's `notes` array** (the spec's "beside the notes") but are kept
  apart on the device as `PushBatch::settings`; the notes loop's `PAGE` check counts both, so the array
  never passes the server's 500. They are built before the notes, so a page that breaks on the notes never
  starves them. The server tells them apart by path.
- **R-TD2-6 — the reply** is `{records, notes, config, config_refused, bytes_used, bytes_ceiling}`:
  `notes` counts note rows only, `config` the settings rows stored. `sync::push` keeps its signature; the
  new `sync::push_reply` returns `PushReply { records, notes, config_refused }`.
- **R-TD2-7 — where the pull takes settings**: `apply_settings` runs right after `apply` in
  `run_lines_with_client`'s pull, on the same page, and records every base in `Cursor.notes`. Lines the
  spec does not word: `sync: config/<file> updated from your account`, `sync: your account's settings
  arrived; they replace the ones this computer was set up with` (the mark's end by (a)), and
  `sync: config/<file> — another computer's settings could not be taken (<cause>); yours stay` (a conflict
  copy that could not be written: nothing is replaced).
- **R-TD2-8 — the conflict file** is `state/config-conflicts/<stem>-<YYYYMMDDTHHMMSSZ>.yaml` on this
  computer's UTC clock (`planning-20260926T101500Z.yaml`), `-2`, `-3`… if the name is taken.
- **R-TD2-9 — a missing settings file comes back** (spec defect S-2). A pull is cursor-based, so "a missing
  one comes back with the next pull" is true only when another computer next changes it. After every pull,
  a `SHARED_CONFIG` file this computer has a base for but no longer finds is written back from
  `GET /sync-pull?view=settings` (`sync::restore_missing_settings`): `sync: config/<file> was missing; your
  account's copy is back`. A vault with every file present makes no extra call.
- **R-TD2-10 — (c) waits for the whole account**: it fires only once `Cursor::pulled_to_end` is set, since
  the account's settings may sit on a page this computer has not read, and an unreadable
  `provisional_since` is stamped `now` (the wait starts again — never "due at once", never "never").
- **R-TD2-11 — (b) is a seam** (spec defect S-3): `sync::settings_edited(vault, rel)` ends that file's mark
  and clears its base. No console command writes a shared setting today (`set_settings` writes only the
  profile's `settings.json`), so nothing calls it yet; the first command that edits a slot time, the time
  zone or any other shared setting calls it after its write (named in hand-off H5).
- **R-TD2-12 — who sets the mark**: `sync::restore_all_as` (the wizard's restore), when no settings row
  arrived and a note row or a record did, or the wizard's `account_settings_exist` saw notes
  (`account_holds_notes`); and `restore_into_as`'s two could-not-read arms when the wizard saw notes. It
  marks every `SHARED_CONFIG` file present, stamped with this computer's clock.
- **R-TD2-13 — `account_settings_exist` reads `GET /sync-pull?view=settings`** (spec defect S-4): the
  account's live settings rows and whether it holds a note, in one call, answered by a new
  `sync-pull` handler (`handleSettings`) beside the paged one. The command answers
  `{ok, error, settings, notes, mapped}`; `mapped` lists the sources whose mappings the account's
  `ingest.yaml` carries.
- **R-TD2-14 — the skipped panels** are the name (its default `Knowlu` stands, as §6.2's walk has it:
  sign-in → logins → Finish), the calendars panel (school, LMS capture, calendars), Gmail, slots and time
  zone, and the subscription when `entitlement_now` says active; the finish summary says the account's
  settings are used rather than naming slot times the restore will replace.
- **R-TD2-15 — a new portal's route survives the restore.** The wizard's confirmed rows (only for sources
  the account does not map) are written into `config/ingest.yaml` at birth; if the restore then replaces
  that file with the account's, `create_vault_in` puts each row back by `coursework::insert_mapping`, which
  creates a missing `coursework.<source>:` block with that source's shared defaults. The file then differs
  from its base and reaches the account at the first sync, compare-and-set as ever.
- **R-TD2-16 — `apply_approved_mappings` leaves a zyBooks key in `ignore:` alone**, since "to stop fetching
  a book, map it to `ignore`" is how a student ignores one.
- **R-TD2-17 — a restore replaces the wizard's settings files without a conflict copy**: they are the
  skipped panels' defaults, never a student's edit. The mark's own (a) keeps a copy.
- **R-TD2-18 — the migration's stamp is `20260926000200`**, and its folder group carries `commitments`:
  the commitment model's branches add `20260926000100_sync_note_path_check_commitments.sql`, which replaces
  the same constraint, and this file sorts after it — so the check must be the union whichever stream
  merges first (*Overlaps*).
- **R-TD2-19 — `restore_capability_url`'s calendar case** moves the address to `device.yaml` and takes the
  `cloud:personal` entry out of the shared file (the account never got the address, so the marker would
  only fail); an emptied list becomes `calendars: []`.
- **R-TD2-20 — `coursework::write_mapping` creates a missing source block** instead of refusing it: under
  D17 whether a source is set up is `device.yaml`'s `enabled`, never whether the account's shared file has
  its block yet. Every other rule of `write_mapping` stands.
- **R-TD2-21 — `account::feeds_in` is unchanged**: attaching an account reads a vault that has never had
  one, and `sync` never splits a vault with no account (R-TD2-3).

## Interfaces this plan produces

Plan 3 consumes these exact names. A task that changes one of them changes this table too.

| Name | Signature or shape | Task |
|---|---|---|
| `config::SHARED_CONFIG` (= `sync::SHARED_CONFIG`) | `pub const SHARED_CONFIG: [&str; 6] = ["config/campus.yaml", "config/events.yaml", "config/ingest.yaml", "config/planning.yaml", "config/runners.yaml", "config/week_template.yaml"]`; `sync.rs` re-exports it (`pub use crate::config::SHARED_CONFIG;`) | 1, 4 |
| `config::DEVICE_FILE` | `pub const DEVICE_FILE: &str = "config/device.yaml";` — never synced, never pushed | 1 |
| `config::PORTAL_SOURCES`, `SOURCE_DEVICE_KEYS`, `RUNNER_DEVICE_KEYS` | `[&str; 2] = ["zybooks", "vhl"]`; `[&str; 3] = ["enabled", "credential_target", "base_url"]`; `[&str; 2] = ["device", "scheduler"]` | 1 |
| `config::is_shared_config`, `config::is_cloud_marker` | `pub fn is_shared_config(rel: &str) -> bool`; `pub fn is_cloud_marker(url: &str) -> bool` | 1 |
| `config::load_device`, `overlay`, `overlay_device` | `pub fn load_device(vault: &Path) -> Mapping` (missing or unreadable: empty); `pub fn overlay(device: &Mapping, config: &mut Mapping)`; `pub fn overlay_device(vault: &Path, config: &mut Mapping)` — D17's reader rule over a parsed `ingest.yaml` | 1 |
| `config::PortalSource`, `config::portal_sources` | `pub struct PortalSource { pub name: String, pub enabled: bool, pub credential_target: String }`; `pub fn portal_sources(vault: &Path) -> Vec<PortalSource>` — every `PORTAL_SOURCES` entry in order, device keys first. **Plan 3's registry builds `devices.logins` from it**: a source `enabled` here whose `credential_target` holds a credential (`app/src/credentials.rs::exists`). | 1 |
| `config::mapped_sources` | `pub fn mapped_sources(ingest_text: &str) -> Vec<String>` — the sources whose `courses:`/`sections:` is a non-empty mapping | 1 |
| `config::quote`, `SourceKeys`, `DeviceKeys`, `device_yaml`, `write_new_device_yaml` | `pub fn quote(field: &str, value: &str) -> Result<String, String>`; `pub struct SourceKeys { pub enabled: Option<bool>, pub credential_target: Option<String>, pub base_url: Option<String> }`; `pub struct DeviceKeys { pub sources: BTreeMap<String, SourceKeys>, pub ics_url: String, pub calendars: Vec<(String, String)>, pub device: Option<String>, pub scheduler: Option<String> }`; `pub fn device_yaml(keys: &DeviceKeys) -> Result<String, String>`; `pub fn write_new_device_yaml(vault: &Path, keys: &DeviceKeys) -> Result<(), String>` (refuses an existing file) | 1 |
| `config::insert_source_key`, `insert_runner_key`, `set_device_ics_url`, `add_device_calendar` | `pub fn insert_source_key(vault: &Path, source: &str, key: &str, literal: &str) -> Result<bool, String>`; `pub fn insert_runner_key(vault: &Path, key: &str, literal: &str) -> Result<bool, String>` (both `Ok(false)` when `device.yaml` already holds the key); `pub fn set_device_ics_url(vault: &Path, url: &str, replace: bool) -> Result<bool, String>`; `pub fn add_device_calendar(vault: &Path, name: &str, url: &str) -> Result<bool, String>` — line insertions; a result that would not parse is refused. **Plan 3 or a later Settings command uses `insert_source_key` for a login saved after onboarding.** | 2 |
| `config::DeviceSpot`, `device_spots`, `computer_only_value` | `pub enum DeviceSpot { SourceKey { source, key }, IcsUrl, Calendar { index, name }, Runner { index, key } }` with `pub fn label(&self) -> String`; `pub fn device_spots(rel: &str, doc: &Mapping) -> Vec<(DeviceSpot, Value)>`; `pub fn computer_only_value(rel: &str, text: &str) -> Result<Option<String>, String>` — the guard | 2 |
| `config::move_device_keys` | `pub fn move_device_keys(vault: &Path) -> Vec<String>` — the migration; lines, never an error; idempotent | 2 |
| `runs::local_runner_settings` | `pub fn local_runner_settings(vault: &Path) -> RunnerSettings` — `device.yaml`'s `runner:` first, `runners.yaml`'s `local` entry otherwise. **Plan 3's scheduler reads it** (`scheduler::mode` and `device_ok` already do, Task 7). | 1 |
| `coursework::insert_mapping`, `ZYBOOKS_SHARED_DEFAULTS`, `VHL_SHARED_DEFAULTS`, `apply_approved_mappings` | `pub fn insert_mapping(vault: &Path, source: &str, key: &str, course: &str, label: &str) -> Result<bool, String>`; two `pub const &str` (unindented block lines); `pub fn apply_approved_mappings(vault: &Path) -> Vec<String>` | 5 |
| `coursework::{indent_of, is_filler, find_key, block_end, child_indent}` | visibility only: `pub(crate)` | 2 |
| `sync::Cursor::provisional_config`, `provisional_since` | `#[serde(default)] pub provisional_config: Vec<String>` (sorted `SHARED_CONFIG` paths), `#[serde(default)] pub provisional_since: String` (a journal `ts`, empty with no mark) — **beside Plan 1's `verdicts_after`**, as its interfaces said | 4 |
| `sync::PushBatch::settings` | `pub settings: Vec<Value>` — **an amendment to Plan 1's (and C3′'s) `PushBatch`**: a struct literal must now name it (`engine/tests/sync_contract.rs`'s two do, Task 4) | 4 |
| `sync::PushReply`, `sync::push_reply` | `pub struct PushReply { pub records: usize, pub notes: usize, pub config_refused: Vec<String> }`; `pub fn push_reply(client: &CloudClient, batch: &PushBatch) -> Result<PushReply, CloudError>`; `sync::push` unchanged | 4 |
| `sync::run_lines_with_client` | signature unchanged; its order after this plan: `config::move_device_keys` → pull → `apply` → `apply_settings` → cursors → `restore_missing_settings` → `publish_if_due` → save → push (`build_push` with the settings rows) → `config_refused` restores those bases → save | 4–6 |
| `sync::CONFLICTS_DIR`, `sync::apply_settings` | `pub const CONFLICTS_DIR: &str = "state/config-conflicts";`; `pub fn apply_settings(vault: &Path, page: &Pulled, cursor: &mut Cursor) -> Vec<String>` | 5 |
| `sync::AccountSettings`, `sync::account_settings`, `sync::restore_missing_settings` | `pub struct AccountSettings { pub texts: BTreeMap<String, String>, pub notes: bool }`; `pub fn account_settings(client: &CloudClient) -> Result<AccountSettings, SyncError>` (one `GET /sync-pull?view=settings`); `pub fn restore_missing_settings(vault: &Path, client: &CloudClient, cursor: &mut Cursor) -> Vec<String>` | 5 |
| `sync::PROVISIONAL_DAYS`, `publish_if_due`, `settings_edited`, `mark_provisional` | `pub const PROVISIONAL_DAYS: i64 = 7;`; `pub fn publish_if_due(cursor: &mut Cursor, now: jiff::Timestamp) -> Option<String>`; `pub fn settings_edited(vault: &Path, rel: &str) -> Result<bool, SyncError>` (**the seam every console command that writes a shared setting calls**); `pub fn mark_provisional(vault: &Path, cursor: &mut Cursor, now: jiff::Timestamp)` | 6 |
| `sync::restore_all_as`, `sync::restore_into_as` | `pub fn restore_all_as(dest: &Path, client: &CloudClient, tolerate: &[String], account_holds_notes: bool) -> Result<Restored, SyncError>`; `pub fn restore_into_as(dest: &Path, account_holds_notes: bool) -> Result<Restored, String>`; `restore_all` and `restore_into` keep their signatures and pass `false` | 6 |
| `POST /sync-push` | a settings row is `{"path", "body", "base"}` in `notes` (`base`: 64 hex or `""`; refused on a note path; no tombstone); reply `{records, notes, config, config_refused, bytes_used, bytes_ceiling}` | 3 |
| `GET /sync-pull?view=settings` | `{"settings": [{"path", "body"}], "notes": bool}` — live settings rows by path, and whether any live note row exists; 401/402 from the entitlement; `405` for another method | 3 |
| `save_config_row(p_account uuid, p_path text, p_body text, p_device text, p_base text) returns boolean` | `20260926000200_shared_settings.sql`; plpgsql, `security invoker`, revoked from `public, anon, authenticated`; the function pin is **29** after this plan | 3 |
| `sync_notes_path_check` | the note folders (with `commitments`) **or** exactly the six settings paths | 3 |
| `scaffold::device_yaml` | `pub fn device_yaml(p: &VaultPlan) -> Result<String, String>`; `ingest_yaml` and `runners_yaml` no longer write a device key | 7 |
| `onboarding::account_settings_exist`, `account_settings_exist_at` | `#[tauri::command(async)] pub fn account_settings_exist() -> Value`; `pub fn account_settings_exist_at(api_base: &str, token: &str) -> Value` → `{ok, error, settings, notes, mapped}` | 8 |
| `onboarding::WizardPlan::account_holds_notes` | `#[serde(default)] pub account_holds_notes: bool` | 8 |

What Plan 3 can rely on, beyond the names: `config/device.yaml` is where a computer's logins are named, so
the registry's `logins` come from `config::portal_sources` and never from a synced file; a vault born by
the wizard carries `runner: {device, scheduler: app}` in `device.yaml`, and `runs::local_runner_settings`
answers it for any vault, split or not; the slot's `sync` step is where the migration runs, so Plan 3's
`coursework --only` always reads a split config on a vault that has synced once; and the shared files a
holder fetches against are the account's once any computer has published them.

---

### Task 1: `config.rs` — the split's vocabulary, `device.yaml`'s writer, the device-first readers (spec §4.8 D17)

**Files:**
- Create: `engine/src/config.rs` (Part A: the constants, `load_device`, `overlay`, `overlay_device`,
  `PortalSource`, `portal_sources`, `mapped_sources`, `quote`, `SourceKeys`, `DeviceKeys`, `device_yaml`,
  `write_new_device_yaml`). Task 2 appends Part B.
- Modify: `engine/src/coursework.rs` — `load_coursework_config` (`:134-154`): every readable arm overlays
  `device.yaml`.
- Modify: `engine/src/calfeed.rs` — `calendar_entries` (`:599-604`) and `load_calendar_events`
  (`:637-642`): the parsed config overlays `device.yaml` before `calendars:` is read.
- Modify: `engine/src/runs.rs` — a new `local_runner_settings` after `runner_settings` (`:353-366`).
- Create: `engine/tests/device_config.rs` (five tests).
- Hand-offs: **H1** (`engine/src/lib.rs`: `pub mod config;`) and **H2** (`engine/src/ingest.rs`: the
  `ics_url` fallback reads `device.yaml` first), applied by the controller at Step 4.

**Interfaces:**
- Consumes: `yaml::{get, text, mapping_from_file, mapping_of}`, `pystr::{yaml_truthy, universal_newlines,
  read_text, write_text}`, `runs::{runner_settings, RunnerSettings}`, `schedule::SchedulerMode::parse`.
- Produces: `config::{SHARED_CONFIG, DEVICE_FILE, PORTAL_SOURCES, SOURCE_DEVICE_KEYS, RUNNER_DEVICE_KEYS,
  is_shared_config, is_cloud_marker, load_device, overlay, overlay_device, PortalSource, portal_sources,
  mapped_sources, quote, SourceKeys, DeviceKeys, device_yaml, write_new_device_yaml}`;
  `runs::local_runner_settings` (signatures in *Interfaces this plan produces*).

Every reader keeps its old behaviour on a vault with no `config/device.yaml` (the fixtures, every vault
before this plan): `overlay` over an empty mapping changes nothing. The five readers D17 names are
`load_coursework_config` (and through it `coursework`, `coursework-discover`, `collect_cloud`),
`ingest`'s `ics_url` (H2), `calfeed`'s `calendars:` (both functions, so `enrich`'s Google-calendar predicate
too), and the `local` runner's keys (`runs::local_runner_settings`; the app's two callers switch to it in
Task 7).

- [ ] **Step 1: Write the failing tests**

Create `engine/tests/device_config.rs`:

```rust
//! Two-desktop design D17 (§4.8, §6.1's shared-config bullets): `config/device.yaml`, the readers'
//! device-first rule, the push guard's test and the text-only migration. Every test works in its own
//! temp vault; nothing here touches a real vault, the network or Credential Manager.

use std::path::{Path, PathBuf};

use knowlu_engine::config;

fn vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-td2-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for folder in ["config", "state", "tasks", "approvals", "archive"] {
        std::fs::create_dir_all(dir.join(folder)).expect("mkdir");
    }
    dir
}

fn put(dir: &Path, rel: &str, text: &str) {
    knowlu_engine::pystr::write_text(&dir.join(rel), text).expect("write");
}

const OLD_RUNNERS: &str = "runners:\n  - name: local\n    times: ['12:00', '18:00']\n    tz: 'America/Chicago'\n    grace_minutes: 20\n    device: 'EXAMPLE-PC'\n    scheduler: app\n";

#[test]
fn device_yaml_is_written_from_literals_in_d17s_shape() {
    let mut keys = config::DeviceKeys::default();
    keys.sources.insert(
        "zybooks".to_string(),
        config::SourceKeys { enabled: Some(true), credential_target: Some("knowlu/profile_0123456789/zybooks".into()), base_url: None },
    );
    keys.device = Some("EXAMPLE-PC".into());
    keys.scheduler = Some("app".into());
    assert_eq!(
        config::device_yaml(&keys).unwrap(),
        "coursework:\n  zybooks:\n    enabled: true\n    credential_target: 'knowlu/profile_0123456789/zybooks'\n  vhl: {}\n\
         ics_url: ''\ncalendars: []\nrunner:\n  device: 'EXAMPLE-PC'\n  scheduler: app\n",
        "the spec's own example, byte for byte"
    );
    keys.calendars.push(("personal".into(), "https://calendar.example.invalid/private-abc/basic.ics".into()));
    let text = config::device_yaml(&keys).unwrap();
    assert!(text.contains("calendars:\n  - name: 'personal'\n    ics_url: 'https://calendar.example.invalid/private-abc/basic.ics'\n"), "{text}");
    keys.device = Some("DESK\u{1}TOP".into());
    assert!(config::device_yaml(&keys).is_err(), "a control character is refused by name, never written");
}

#[test]
fn a_device_key_is_read_from_device_yaml_first_and_from_its_old_place_otherwise() {
    let dir = vault("device-first");
    put(&dir, "config/ingest.yaml", "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: false\n    credential_target: 'old/zybooks'\n    courses: {}\n");
    put(&dir, config::DEVICE_FILE, "coursework:\n  zybooks:\n    enabled: true\n  vhl:\n    enabled: true\n    credential_target: 'knowlu/p/vhl'\nics_url: ''\ncalendars: []\nrunner: {}\n");
    let (cfg, warnings) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let zy = cfg["coursework"]["zybooks"].as_mapping().unwrap();
    assert_eq!(zy.get("enabled").and_then(|v| v.as_bool()), Some(true), "device.yaml wins");
    assert_eq!(zy.get("credential_target").and_then(|v| v.as_str()), Some("old/zybooks"), "device.yaml lacks it, so the old place answers");
    assert!(zy.get("courses").is_some(), "the shared keys are untouched");
    let vhl = cfg["coursework"]["vhl"].as_mapping().unwrap();
    assert_eq!(vhl.get("credential_target").and_then(|v| v.as_str()), Some("knowlu/p/vhl"), "a source only device.yaml names is read");
    let sources = config::portal_sources(&dir);
    assert_eq!(
        sources.iter().map(|s| (s.name.as_str(), s.enabled, s.credential_target.as_str())).collect::<Vec<_>>(),
        vec![("zybooks", true, "old/zybooks"), ("vhl", true, "knowlu/p/vhl")]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn calendars_are_the_union_of_both_files() {
    let dir = vault("calendars");
    put(&dir, "config/ingest.yaml", "timezone: America/Chicago\ncalendars:\n  - name: google\n    ics_url: 'cloud:google'\n");
    put(&dir, config::DEVICE_FILE, "coursework:\n  zybooks: {}\n  vhl: {}\nics_url: ''\ncalendars:\n  - name: 'personal'\n    ics_url: 'https://calendar.example.invalid/private-abc/basic.ics'\nrunner: {}\n");
    assert_eq!(
        knowlu_engine::calfeed::calendar_entries(&dir),
        vec![
            ("google".to_string(), "cloud:google".to_string()),
            ("personal".to_string(), "https://calendar.example.invalid/private-abc/basic.ics".to_string()),
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_lms_feed_fallback_comes_from_device_yaml() {
    let dir = vault("ics-url");
    put(&dir, "config/ingest.yaml", "timezone: America/Chicago\ncourse_map: {}\n");
    put(&dir, config::DEVICE_FILE, "coursework:\n  zybooks: {}\n  vhl: {}\nics_url: 'https://lms.example.invalid/feed.ics'\ncalendars: []\nrunner: {}\n");
    let asked = std::cell::RefCell::new(Vec::new());
    let fetch = |url: &str| -> Result<String, String> {
        asked.borrow_mut().push(url.to_string());
        Ok("BEGIN:VCALENDAR\nEND:VCALENDAR\n".to_string())
    };
    let (code, _) = knowlu_engine::ingest::run_lines(&dir, "cli", None, Some(&fetch));
    assert_eq!(code, 0);
    assert_eq!(asked.borrow().as_slice(), ["https://lms.example.invalid/feed.ics"], "the vault's fallback is this computer's");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_local_runners_device_keys_come_from_device_yaml_first() {
    let dir = vault("runner");
    put(&dir, "config/runners.yaml", OLD_RUNNERS);
    let before = knowlu_engine::runs::local_runner_settings(&dir);
    assert_eq!(before.device.as_deref(), Some("EXAMPLE-PC"), "a vault not yet split reads as it always did");
    put(&dir, config::DEVICE_FILE, "coursework:\n  zybooks: {}\n  vhl: {}\nics_url: ''\ncalendars: []\nrunner:\n  device: 'OTHER-PC'\n");
    let after = knowlu_engine::runs::local_runner_settings(&dir);
    assert_eq!(after.device.as_deref(), Some("OTHER-PC"), "device.yaml wins");
    assert_eq!(after.scheduler, knowlu_engine::schedule::SchedulerMode::App, "and the key it lacks comes from runners.yaml");
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run them to see them fail**

Run (PATH refreshed, Global Constraint 13):
`cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test device_config`
Expected: FAIL to compile — `unresolved import knowlu_engine::config`, and `cannot find function
local_runner_settings in module knowlu_engine::runs`.

- [ ] **Step 3: The module and the readers**

In `engine/src/coursework.rs`, replace

```rust
pub fn load_coursework_config(vault: &Path) -> Result<(Mapping, Vec<String>), SourceError> {
    let path = vault.join("config").join("ingest.yaml");
    if !path.exists() {
        return Ok((Mapping::new(), Vec::new()));
    }
    let bytes = match std::fs::read(&path) {
```

with

```rust
pub fn load_coursework_config(vault: &Path) -> Result<(Mapping, Vec<String>), SourceError> {
    let path = vault.join("config").join("ingest.yaml");
    // Two-desktop design D17: every readable config takes this computer's own keys from
    // `config/device.yaml` first (`config::overlay_device`) — `enabled`, `credential_target`,
    // `base_url`, the `ics_url` fallback and the raw calendars — so a vault split by the wizard or by
    // the migration reads exactly as it did whole.
    let device = |mut map: Mapping| {
        crate::config::overlay_device(vault, &mut map);
        map
    };
    if !path.exists() {
        return Ok((device(Mapping::new()), Vec::new()));
    }
    let bytes = match std::fs::read(&path) {
```

In `engine/src/coursework.rs`, replace

```rust
    })?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&pystr::universal_newlines(&text)) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => Ok((map, Vec::new())),
        // `yaml.safe_load(...) or {}` — a scalar or empty document is an empty config.
        Ok(_) => Ok((Mapping::new(), Vec::new())),
        Err(err) => Ok((Mapping::new(), vec![format!("config unreadable: {err}")])),
    }
```

with

```rust
    })?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&pystr::universal_newlines(&text)) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => Ok((device(map), Vec::new())),
        // `yaml.safe_load(...) or {}` — a scalar or empty document is an empty config.
        Ok(_) => Ok((device(Mapping::new()), Vec::new())),
        Err(err) => Ok((Mapping::new(), vec![format!("config unreadable: {err}")])),
    }
```

In `engine/src/calfeed.rs`, replace

```rust
    let config_path = vault.join("config").join("ingest.yaml");
    let Ok(raw) = pystr::read_text(&config_path) else { return Vec::new() };
    let Ok(config) = serde_yaml_ng::from_str::<Value>(&raw) else { return Vec::new() };
    let feeds = calendars_feeds(&config).ok().flatten().unwrap_or_default();
    feeds
```

with

```rust
    let config_path = vault.join("config").join("ingest.yaml");
    let Ok(raw) = pystr::read_text(&config_path) else { return Vec::new() };
    let Ok(mut config) = serde_yaml_ng::from_str::<Value>(&raw) else { return Vec::new() };
    // Two-desktop design D17: this computer's raw calendars live in `config/device.yaml`.
    if let Value::Mapping(map) = &mut config {
        crate::config::overlay_device(vault, map);
    }
    let feeds = calendars_feeds(&config).ok().flatten().unwrap_or_default();
    feeds
```

In `engine/src/calfeed.rs`, replace

```rust
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    let config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    use serde_yaml_ng::Value;
    let feeds: Vec<Value> = match calendars_feeds(&config) {
        Ok(Some(feeds)) => feeds,
```

with

```rust
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    let mut config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&raw) {
        Ok(value) => value,
        Err(err) => return (Vec::new(), vec![format!("config unreadable: {err}")]),
    };
    use serde_yaml_ng::Value;
    // Two-desktop design D17: the shared `calendars:` plus this computer's own raw ones from
    // `config/device.yaml` (`config::overlay`), before either is read.
    if let Value::Mapping(map) = &mut config {
        crate::config::overlay_device(vault, map);
    }
    let feeds: Vec<Value> = match calendars_feeds(&config) {
        Ok(Some(feeds)) => feeds,
```

In `engine/src/runs.rs`, replace

```rust
    }
    default
}
```

with

```rust
    }
    default
}

/// The `local` runner's `device` and `scheduler` for this vault — two-desktop design D17:
/// `config/device.yaml`'s `runner:` keys first, `config/runners.yaml`'s `local` entry only for a key
/// `device.yaml` lacks, so a vault not yet split reads exactly as it did.
pub fn local_runner_settings(vault: &Path) -> RunnerSettings {
    let shared = runner_settings(&vault.join("config").join("runners.yaml"), "local");
    let device = crate::config::load_device(vault);
    let Some(serde_yaml_ng::Value::Mapping(runner)) = crate::yaml::get(&device, "runner") else { return shared };
    RunnerSettings {
        device: match crate::yaml::get(runner, "device") {
            Some(v) => v.as_str().map(str::to_string),
            None => shared.device,
        },
        scheduler: match crate::yaml::get(runner, "scheduler") {
            Some(v) => crate::schedule::SchedulerMode::parse(v.as_str()),
            None => shared.scheduler,
        },
    }
}
```

Create `engine/src/config.rs`:

```rust
//! Two-desktop design D17 (§4.8): which `config/` values are the account's and which are this
//! computer's.
//!
//! **The split.** Six files are the student's shared settings and travel through `sync` whole
//! ([`SHARED_CONFIG`], D18). A handful of keys inside two of them belong to one computer only —
//! each coursework source's `enabled`, `credential_target` and `base_url`, the vault-held `ics_url`
//! fallback, every `calendars:` entry whose `ics_url` is not a `cloud:` marker (a raw capability
//! URL), and the `local` runner's `device` and `scheduler` — and live in [`DEVICE_FILE`], which never
//! syncs.
//!
//! **Readers take a device key from `config/device.yaml` first** and from its old place only when
//! `device.yaml` lacks it; `calendars:` is the union of both files ([`overlay`]). A vault not yet
//! migrated keeps working, and a migration interrupted between its two steps leaves a duplicate the
//! device-first rule makes harmless.
//!
//! **Nothing here parses a vault file and re-dumps it.** `device.yaml` is written whole from
//! literals when it is new ([`device_yaml`]) and grows by line insertion afterwards; the shared files
//! lose device lines by line deletion ([`move_device_keys`]), the kind of edit
//! `coursework::write_mapping` makes.

use std::path::Path;

use serde_yaml_ng::{Mapping, Value};

/// The six shared settings files (D17, D18), sorted. `sync::SHARED_CONFIG` is this list, and
/// `_shared/sync_rows.ts`'s `SHARED_CONFIG` and the `sync_notes` path check name the same six.
pub const SHARED_CONFIG: [&str; 6] = [
    "config/campus.yaml",
    "config/events.yaml",
    "config/ingest.yaml",
    "config/planning.yaml",
    "config/runners.yaml",
    "config/week_template.yaml",
];

/// This computer's own settings (D17). Generated from literals, never synced, never pushed.
pub const DEVICE_FILE: &str = "config/device.yaml";

/// The coursework sources a computer can hold a login for, in the order `device.yaml` lists them.
pub const PORTAL_SOURCES: [&str; 2] = ["zybooks", "vhl"];

/// A coursework source's device-owned keys, in the order `device.yaml` writes them.
pub const SOURCE_DEVICE_KEYS: [&str; 3] = ["enabled", "credential_target", "base_url"];

/// The `local` runner's device-owned keys, in the order `device.yaml` writes them.
pub const RUNNER_DEVICE_KEYS: [&str; 2] = ["device", "scheduler"];

/// Is `rel` one of the six shared settings files?
pub fn is_shared_config(rel: &str) -> bool {
    SHARED_CONFIG.contains(&rel)
}

/// `cloud:personal`, `cloud:google`: a routing marker the account resolves, never a URL.
pub fn is_cloud_marker(url: &str) -> bool {
    url.trim().starts_with("cloud:")
}

/// `config/device.yaml`, parsed. Missing or unreadable is an empty mapping: a vault not yet split.
pub fn load_device(vault: &Path) -> Mapping {
    crate::yaml::mapping_from_file(&vault.join(DEVICE_FILE))
}

fn get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    crate::yaml::get(map, key)
}

/// `config[key]` as a mapping, made one when it is absent or is not a mapping.
fn mapping_at<'a>(config: &'a mut Mapping, key: &str) -> &'a mut Mapping {
    let k = Value::from(key);
    if !matches!(config.get(&k), Some(Value::Mapping(_))) {
        config.insert(k.clone(), Value::Mapping(Mapping::new()));
    }
    match config.get_mut(&k) {
        Some(Value::Mapping(m)) => m,
        _ => unreachable!("inserted as a mapping one line up"),
    }
}

/// D17's reader rule over a parsed `config/ingest.yaml`: every device key `device` carries replaces
/// the shared file's, and `device`'s `calendars:` entries are appended to the shared ones. In
/// memory only — nothing is written.
pub fn overlay(device: &Mapping, config: &mut Mapping) {
    if let Some(Value::Mapping(cw)) = get(device, "coursework") {
        for source in PORTAL_SOURCES {
            let Some(Value::Mapping(keys)) = get(cw, source) else { continue };
            let present: Vec<(&str, Value)> =
                SOURCE_DEVICE_KEYS.iter().filter_map(|k| get(keys, k).map(|v| (*k, v.clone()))).collect();
            if present.is_empty() {
                continue;
            }
            let block = mapping_at(mapping_at(config, "coursework"), source);
            for (k, v) in present {
                block.insert(Value::from(k), v);
            }
        }
    }
    if let Some(url) = get(device, "ics_url") {
        config.insert(Value::from("ics_url"), url.clone());
    }
    if let Some(Value::Sequence(extra)) = get(device, "calendars") {
        if !extra.is_empty() {
            let mut all = match get(config, "calendars") {
                Some(Value::Sequence(shared)) => shared.clone(),
                _ => Vec::new(),
            };
            all.extend(extra.iter().cloned());
            config.insert(Value::from("calendars"), Value::Sequence(all));
        }
    }
}

/// [`overlay`] with this vault's own `config/device.yaml`.
pub fn overlay_device(vault: &Path, config: &mut Mapping) {
    overlay(&load_device(vault), config);
}

/// One coursework source as this computer sees it: device keys first (D17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalSource {
    pub name: String,
    pub enabled: bool,
    pub credential_target: String,
}

/// Every [`PORTAL_SOURCES`] entry, in order, as the merged view reads it. Plan 3's registry reads
/// this for `devices.logins` (a source enabled here whose credential is present).
pub fn portal_sources(vault: &Path) -> Vec<PortalSource> {
    let mut config = crate::yaml::mapping_from_file(&vault.join("config").join("ingest.yaml"));
    overlay_device(vault, &mut config);
    let cw = match get(&config, "coursework") {
        Some(Value::Mapping(m)) => m.clone(),
        _ => Mapping::new(),
    };
    PORTAL_SOURCES
        .iter()
        .map(|name| {
            let block = match get(&cw, name) {
                Some(Value::Mapping(m)) => m.clone(),
                _ => Mapping::new(),
            };
            PortalSource {
                name: name.to_string(),
                enabled: get(&block, "enabled").map(crate::pystr::yaml_truthy).unwrap_or(false),
                credential_target: get(&block, "credential_target").and_then(crate::yaml::text).unwrap_or_default(),
            }
        })
        .collect()
}

/// The sources whose shared `courses:` (zyBooks) or `sections:` (VHL) mapping in `ingest_text` is a
/// non-empty mapping — what D19's wizard reads to decide whether to show a source's mapping rows.
pub fn mapped_sources(ingest_text: &str) -> Vec<String> {
    let config = crate::yaml::mapping_of(&crate::pystr::universal_newlines(ingest_text));
    let cw = match get(&config, "coursework") {
        Some(Value::Mapping(m)) => m.clone(),
        _ => return Vec::new(),
    };
    PORTAL_SOURCES
        .iter()
        .filter(|name| {
            let field = if **name == "zybooks" { "courses" } else { "sections" };
            matches!(
                get(&cw, name).and_then(|b| b.as_mapping()).and_then(|b| get(b, field)),
                Some(Value::Mapping(m)) if !m.is_empty()
            )
        })
        .map(|name| name.to_string())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// config/device.yaml, written from literals
// ---------------------------------------------------------------------------------------------

/// One value as a single-quoted YAML scalar — the wizard's `scaffold::yaml_scalar` rule: inside
/// `'…'` YAML does no escape processing, `'` doubles, and a control character is refused by field
/// name rather than written out to break the file's shape.
pub fn quote(field: &str, value: &str) -> Result<String, String> {
    if let Some(c) = value.chars().find(|c| c.is_control()) {
        return Err(format!("{field}: control character U+{:04X} is not allowed", c as u32));
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

/// One source's device keys. `None` is "not set here", never `false` or `''`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceKeys {
    pub enabled: Option<bool>,
    pub credential_target: Option<String>,
    pub base_url: Option<String>,
}

/// Everything `config/device.yaml` holds (D17's table).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceKeys {
    /// Keyed by [`PORTAL_SOURCES`] name; a source with nothing set is written `{}`.
    pub sources: std::collections::BTreeMap<String, SourceKeys>,
    /// The vault-held LMS feed fallback; `''` when the account holds it.
    pub ics_url: String,
    /// `(name, ics_url)` of every calendar whose address is not a `cloud:` marker.
    pub calendars: Vec<(String, String)>,
    pub device: Option<String>,
    pub scheduler: Option<String>,
}

/// `scheduler: app` stays a bare word, as `scaffold::runners_yaml` wrote it; anything else is quoted.
fn scheduler_literal(value: &str) -> Result<String, String> {
    if !value.is_empty() && value.chars().all(|c| c.is_ascii_lowercase()) {
        Ok(value.to_string())
    } else {
        quote("scheduler", value)
    }
}

fn source_key_literal(source: &str, key: &str, keys: &SourceKeys) -> Result<Option<String>, String> {
    Ok(match key {
        "enabled" => keys.enabled.map(|b| b.to_string()),
        "credential_target" => match &keys.credential_target {
            Some(t) => Some(quote(&format!("{source} credential target"), t)?),
            None => None,
        },
        _ => match &keys.base_url {
            Some(u) => Some(quote(&format!("{source} base url"), u)?),
            None => None,
        },
    })
}

/// `config/device.yaml`'s text, in D17's literal shape: every portal source (`{}` when nothing is
/// set), `ics_url`, `calendars` (`[]` when none), and the `runner` keys.
pub fn device_yaml(keys: &DeviceKeys) -> Result<String, String> {
    let mut s = String::from("coursework:\n");
    for source in PORTAL_SOURCES {
        let k = keys.sources.get(source).cloned().unwrap_or_default();
        let mut lines = Vec::new();
        for key in SOURCE_DEVICE_KEYS {
            if let Some(lit) = source_key_literal(source, key, &k)? {
                lines.push(format!("    {key}: {lit}\n"));
            }
        }
        if lines.is_empty() {
            s.push_str(&format!("  {source}: {{}}\n"));
        } else {
            s.push_str(&format!("  {source}:\n"));
            lines.iter().for_each(|l| s.push_str(l));
        }
    }
    s.push_str(&format!("ics_url: {}\n", quote("LMS feed URL", &keys.ics_url)?));
    if keys.calendars.is_empty() {
        s.push_str("calendars: []\n");
    } else {
        s.push_str("calendars:\n");
        for (name, url) in &keys.calendars {
            s.push_str(&format!(
                "  - name: {}\n    ics_url: {}\n",
                quote("calendar name", name)?,
                quote("calendar address", url)?
            ));
        }
    }
    match (&keys.device, &keys.scheduler) {
        (None, None) => s.push_str("runner: {}\n"),
        (device, scheduler) => {
            s.push_str("runner:\n");
            if let Some(d) = device {
                s.push_str(&format!("  device: {}\n", quote("device name", d)?));
            }
            if let Some(m) = scheduler {
                s.push_str(&format!("  scheduler: {}\n", scheduler_literal(m)?));
            }
        }
    }
    Ok(s)
}

/// A new `config/device.yaml`, whole, through a temporary file and a rename. Refuses one that is
/// already there: after its birth the file only grows by line insertion.
pub fn write_new_device_yaml(vault: &Path, keys: &DeviceKeys) -> Result<(), String> {
    let path = vault.join(DEVICE_FILE);
    if path.exists() {
        return Err(format!("{DEVICE_FILE} is already here"));
    }
    let text = device_yaml(keys)?;
    let tmp = path.with_extension("yaml.tmp");
    crate::pystr::write_text(&tmp, &text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Hand-offs H1 and H2 are applied**

The controller applies **H1** (`engine/src/lib.rs`) and **H2** (`engine/src/ingest.rs`) verbatim to the
working tree. An implementer never edits either file. (H1 names the module this task created, so it can
only land now: a module line whose file does not exist yet does not compile.)

- [ ] **Step 5: Run the tests**

Run the command of Step 2. Expected: PASS, all five. Then
`cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- coursework:: calfeed:: runs:: ingest::`
— PASS: every existing reader test is unchanged (none of their vaults has a `device.yaml`).

- [ ] **Step 6: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings; `oracle.rs` and `surface_oracle.rs` untouched.

- [ ] **Step 7: Commit**

The controller commits H1 and H2 first, as their own commit; then the task. The two land back to back:
neither builds alone (the hand-off names a file the task adds, and the task's readers call a module the
hand-off declares), and the branch is pushed only after both.

Message file `.superpowers\sdd\msg-p2-task-1.txt`:

```
feat: config/device.yaml and the device-first readers (two desktops, D17)

A new engine module names the six shared settings files and this
computer's own keys, writes config/device.yaml whole from literals, and
lays device.yaml over the shared ingest.yaml wherever a device key is
read: coursework's config, the LMS feed fallback, the calendars list (the
union of both files) and the local runner's device and scheduler. A vault
with no device.yaml reads exactly as before.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/config.rs engine/src/coursework.rs engine/src/calfeed.rs engine/src/runs.rs engine/tests/device_config.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-1.txt`

---

### Task 2: The guard's test and the migration (spec §4.8 D17: the wedge guard, `move_device_keys`, later additions)

**Files:**
- Modify: `engine/src/config.rs` — the `use` block gains `crate::coursework`'s five line helpers, and
  Part B is appended: the text helpers, `insert_source_key`, `insert_runner_key`, `set_device_ics_url`,
  `add_device_calendar`, `DeviceSpot`, `device_spots`, `computer_only_value` and `move_device_keys`.
- Modify: `engine/src/coursework.rs` — `indent_of`, `is_filler`, `find_key`, `block_end`,
  `child_indent` (`:1280-1330`) become `pub(crate)` (visibility only; `write_mapping` is their author and
  the migration reads YAML lines exactly as it does).
- Modify: `engine/tests/device_config.rs` — six tests appended.

**Interfaces:**
- Consumes: Task 1's `config` Part A; `coursework::{indent_of, is_filler, find_key, block_end,
  child_indent}`.
- Produces: `config::{insert_source_key, insert_runner_key, set_device_ics_url, add_device_calendar,
  DeviceSpot, device_spots, computer_only_value, move_device_keys}`.

**How the migration edits text** (R-TD2-2): it parses each shared file once to learn *which* device values
it holds (`device_spots`, a read), then locates each one's lines in the text (`spot_span`: a key line and
its deeper-indented continuation; a whole list item for a calendar) and deletes exactly those, bottom-up.
A value that is not on lines of its own — a flow mapping, a key on a list item's dash line — is left in
place and named; `device.yaml` still gets it, so the readers still see it, and the guard keeps the shared
file local. After the deletions it re-parses the result and writes the file only if every value it meant
to move is gone and nothing else moved; otherwise the file stays as it was, named. An emptied
`calendars:` becomes `calendars: []`, its birth shape. Its lines:
`config: moved <n> computer-only value(s) into config/device.yaml`,
`config: config/<file> — <key> could not be moved (<why>); it stays on this computer`,
`config: config/<file> could not be edited cleanly; it stays on this computer as it was`,
`config: config/device.yaml could not be written (<cause>); nothing was moved`.

- [ ] **Step 1: Write the failing tests**

Append to `engine/tests/device_config.rs` (after one blank line):

```rust
fn read(dir: &Path, rel: &str) -> String {
    knowlu_engine::pystr::read_text(&dir.join(rel)).expect("read")
}

/// The whole-vault shape the wizard wrote before this stream: an account vault with both portals,
/// its personal calendar in the account, and the `local` runner's two device keys.
const OLD_INGEST: &str = "ics_url: ''\n\
timezone: 'America/Chicago'\n\
course_map:\n  'CS 100': 'cs-100'\n\
calendars:\n  - name: personal\n    ics_url: 'cloud:personal'\n\
\n# Passwords are NOT here. They live in Windows Credential Manager under the\n# credential_target names below.\n\
coursework:\n  zybooks:\n    enabled: true\n    credential_target: 'knowlu/profile_0123456789/zybooks'\n    ignore: []\n    courses: {}\n  \
vhl:\n    enabled: true\n    credential_target: 'knowlu/profile_0123456789/vhl'\n    importance: 3\n    sections: {}\n";

#[test]
fn the_guard_names_a_device_key_a_raw_calendar_or_an_address_and_passes_a_clean_file() {
    let guard = |rel: &str, text: &str| config::computer_only_value(rel, text).expect("parses");
    assert_eq!(guard("config/ingest.yaml", OLD_INGEST).as_deref(), Some("coursework.zybooks.enabled"));
    assert_eq!(guard("config/ingest.yaml", "timezone: X\nics_url: ''\n").as_deref(), Some("ics_url"), "the key itself, even empty");
    for raw in ["https://calendar.example.invalid/a.ics", "webcal://calendar.example.invalid/a.ics"] {
        let text = format!("timezone: X\ncalendars:\n  - name: personal\n    ics_url: '{raw}'\n");
        assert_eq!(guard("config/ingest.yaml", &text).as_deref(), Some("calendars.personal"), "{raw}");
    }
    assert_eq!(guard("config/runners.yaml", OLD_RUNNERS).as_deref(), Some("runners.local.device"));
    assert_eq!(
        guard("config/planning.yaml", "recurring:\n  - title: x\n    link: 'https://lms.example.invalid/secret'\n").as_deref(),
        Some("recurring[0].link"),
        "an address anywhere else in a shared file"
    );
    assert_eq!(
        guard("config/events.yaml", "sources:\n  - name: campus\n    type: ics\n    url: \"https://calendar.example.invalid/campus.ics\"\n    enabled: true\n"),
        None,
        "events.yaml's public campus feeds, and its per-feed `enabled:`, are shared values"
    );
    assert_eq!(
        guard("config/events.yaml", "sources: []\nnote: 'https://example.invalid/x'\n").as_deref(),
        Some("note"),
        "only the campus feeds' own `url` is exempt"
    );
    let split = "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: 'cloud:personal'\ncoursework:\n  zybooks:\n    courses: {}\n";
    assert_eq!(guard("config/ingest.yaml", split), None, "a split file with cloud markers travels");
    assert!(config::computer_only_value("config/ingest.yaml", "a: [unclosed\n").is_err(), "an unreadable file cannot be proved clean");
}

#[test]
fn the_migration_moves_exactly_the_device_keys_into_device_yamls_literal_shape() {
    let dir = vault("migrate");
    put(&dir, "config/ingest.yaml", OLD_INGEST);
    put(&dir, "config/runners.yaml", OLD_RUNNERS);
    let (before, _) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    let runner_before = knowlu_engine::runs::local_runner_settings(&dir);

    let lines = config::move_device_keys(&dir);
    assert_eq!(lines, vec!["config: moved 7 computer-only value(s) into config/device.yaml".to_string()]);
    assert_eq!(
        read(&dir, config::DEVICE_FILE),
        "coursework:\n  zybooks:\n    enabled: true\n    credential_target: 'knowlu/profile_0123456789/zybooks'\n  \
         vhl:\n    enabled: true\n    credential_target: 'knowlu/profile_0123456789/vhl'\n\
         ics_url: ''\ncalendars: []\nrunner:\n  device: 'EXAMPLE-PC'\n  scheduler: app\n"
    );
    assert_eq!(
        read(&dir, "config/ingest.yaml"),
        "timezone: 'America/Chicago'\ncourse_map:\n  'CS 100': 'cs-100'\ncalendars:\n  - name: personal\n    ics_url: 'cloud:personal'\n\
         \n# Passwords are NOT here. They live in Windows Credential Manager under the\n# credential_target names below.\n\
         coursework:\n  zybooks:\n    ignore: []\n    courses: {}\n  vhl:\n    importance: 3\n    sections: {}\n",
        "the moved lines are gone and every other byte stays"
    );
    assert_eq!(
        read(&dir, "config/runners.yaml"),
        "runners:\n  - name: local\n    times: ['12:00', '18:00']\n    tz: 'America/Chicago'\n    grace_minutes: 20\n"
    );
    let (after, _) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    assert_eq!(after, before, "the readers see the same settings after the split");
    assert_eq!(knowlu_engine::runs::local_runner_settings(&dir), runner_before);
    for rel in ["config/ingest.yaml", "config/runners.yaml"] {
        assert_eq!(config::computer_only_value(rel, &read(&dir, rel)).unwrap(), None, "{rel} may travel now");
    }

    let files = ["config/ingest.yaml", "config/runners.yaml", config::DEVICE_FILE];
    let snapshot: Vec<String> = files.iter().map(|r| read(&dir, r)).collect();
    assert!(config::move_device_keys(&dir).is_empty(), "twice is a no-op");
    assert_eq!(files.iter().map(|r| read(&dir, r)).collect::<Vec<String>>(), snapshot);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_raw_calendar_address_moves_to_device_yaml_and_a_marker_stays() {
    let dir = vault("migrate-calendars");
    put(
        &dir,
        "config/ingest.yaml",
        "timezone: America/Chicago\ncalendars:\n  - name: personal\n    ics_url: 'https://calendar.example.invalid/private-abc/basic.ics'\n  \
         - name: google\n    ics_url: 'cloud:google'\n  - name: club\n    ics_url: 'webcal://club.example.invalid/feed.ics'\n",
    );
    config::move_device_keys(&dir);
    assert_eq!(read(&dir, "config/ingest.yaml"), "timezone: America/Chicago\ncalendars:\n  - name: google\n    ics_url: 'cloud:google'\n");
    assert!(read(&dir, config::DEVICE_FILE).contains(
        "calendars:\n  - name: 'personal'\n    ics_url: 'https://calendar.example.invalid/private-abc/basic.ics'\n  \
         - name: 'club'\n    ics_url: 'webcal://club.example.invalid/feed.ics'\n"
    ));
    // …and with every entry raw, the shared list keeps its shape as an empty one.
    let dir2 = vault("migrate-calendars-all");
    put(&dir2, "config/ingest.yaml", "timezone: X\ncalendars:\n  - name: personal\n    ics_url: 'https://a.example.invalid/x.ics'\n");
    config::move_device_keys(&dir2);
    assert_eq!(read(&dir2, "config/ingest.yaml"), "timezone: X\ncalendars: []\n");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn a_crash_between_the_two_steps_leaves_a_vault_the_readers_still_read() {
    let dir = vault("migrate-crash");
    put(&dir, "config/ingest.yaml", OLD_INGEST);
    put(&dir, "config/runners.yaml", OLD_RUNNERS);
    let (before, _) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    // Step 1 happened (device.yaml holds every value) and the process died before step 2.
    let mut keys = config::DeviceKeys::default();
    for source in ["zybooks", "vhl"] {
        keys.sources.insert(
            source.to_string(),
            config::SourceKeys {
                enabled: Some(true),
                credential_target: Some(format!("knowlu/profile_0123456789/{source}")),
                base_url: None,
            },
        );
    }
    keys.device = Some("EXAMPLE-PC".into());
    keys.scheduler = Some("app".into());
    config::write_new_device_yaml(&dir, &keys).unwrap();
    let device_before = read(&dir, config::DEVICE_FILE);
    let (between, _) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    assert_eq!(between, before, "a duplicate is harmless: the device-first rule reads the same values");
    let ingest = read(&dir, "config/ingest.yaml");
    assert!(config::computer_only_value("config/ingest.yaml", &ingest).unwrap().is_some(), "and the guard keeps it local");
    // The next run finishes it: the shared lines go, device.yaml is not touched.
    let lines = config::move_device_keys(&dir);
    assert_eq!(lines, vec!["config: moved 7 computer-only value(s) into config/device.yaml".to_string()]);
    assert_eq!(read(&dir, config::DEVICE_FILE), device_before);
    assert_eq!(config::computer_only_value("config/ingest.yaml", &read(&dir, "config/ingest.yaml")).unwrap(), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_value_that_is_not_on_a_line_of_its_own_stays_and_is_named() {
    let dir = vault("migrate-flow");
    let text = "timezone: X\ncoursework:\n  zybooks: {enabled: true, courses: {}}\n";
    put(&dir, "config/ingest.yaml", text);
    let lines = config::move_device_keys(&dir);
    assert!(
        lines.iter().any(|l| l.contains("coursework.zybooks.enabled could not be moved (it is not on a line of its own)")),
        "{lines:?}"
    );
    assert_eq!(read(&dir, "config/ingest.yaml"), text, "the shared file is untouched");
    assert!(read(&dir, config::DEVICE_FILE).contains("  zybooks:\n    enabled: true\n"), "device.yaml holds it, so the readers still see it");
    assert!(config::computer_only_value("config/ingest.yaml", text).unwrap().is_some(), "the guard keeps the file local");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_later_addition_is_inserted_under_its_known_parent() {
    let dir = vault("insert");
    config::write_new_device_yaml(&dir, &config::DeviceKeys::default()).unwrap();
    assert!(config::insert_source_key(&dir, "vhl", "enabled", "true").unwrap());
    assert!(!config::insert_source_key(&dir, "vhl", "enabled", "false").unwrap(), "a value already there wins");
    assert!(config::set_device_ics_url(&dir, "https://lms.example.invalid/f.ics", true).unwrap());
    assert!(config::add_device_calendar(&dir, "personal", "https://c.example.invalid/p.ics").unwrap());
    assert!(config::insert_runner_key(&dir, "device", "'EXAMPLE-PC'").unwrap());
    assert_eq!(
        read(&dir, config::DEVICE_FILE),
        "coursework:\n  zybooks: {}\n  vhl:\n    enabled: true\nics_url: 'https://lms.example.invalid/f.ics'\n\
         calendars:\n  - name: 'personal'\n    ics_url: 'https://c.example.invalid/p.ics'\nrunner:\n  device: 'EXAMPLE-PC'\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test device_config`
Expected: FAIL to compile — `cannot find function computer_only_value in module config`, and the same for
`move_device_keys`, `insert_source_key`, `set_device_ics_url`, `add_device_calendar`, `insert_runner_key`.

- [ ] **Step 3: The helpers' visibility, the guard and the migration**

In `engine/src/coursework.rs`, replace

```rust
/// The indentation of a line, in spaces. Vault YAML is space-indented — a tab is invalid YAML
/// there and every reader in the crate would already have refused the file.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
```

with

```rust
/// The indentation of a line, in spaces. Vault YAML is space-indented — a tab is invalid YAML
/// there and every reader in the crate would already have refused the file.
pub(crate) fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
```

In `engine/src/coursework.rs`, replace

```rust
/// A line that carries no structure: blank, or a comment. Its column means nothing, so neither
/// helper below may let one end a block or set an indent.
fn is_filler(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#')
```

with

```rust
/// A line that carries no structure: blank, or a comment. Its column means nothing, so neither
/// helper below may let one end a block or set an indent.
pub(crate) fn is_filler(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#')
```

In `engine/src/coursework.rs`, replace

```rust
/// The first line in `[from, end)` at exactly `depth` whose key is `name`, or `None` if the block
/// ends first. A block ends at the first structural line indented *less* than `depth`.
fn find_key(lines: &[String], from: usize, end: usize, name: &str, depth: usize) -> Option<usize> {
    for i in from..end.min(lines.len()) {
        if is_filler(&lines[i]) {
```

with

```rust
/// The first line in `[from, end)` at exactly `depth` whose key is `name`, or `None` if the block
/// ends first. A block ends at the first structural line indented *less* than `depth`.
pub(crate) fn find_key(lines: &[String], from: usize, end: usize, name: &str, depth: usize) -> Option<usize> {
    for i in from..end.min(lines.len()) {
        if is_filler(&lines[i]) {
```

In `engine/src/coursework.rs`, replace

```rust
/// Where the block headed by `header` ends: the first structural line indented no deeper than the
/// header itself, or the end of the file.
fn block_end(lines: &[String], header: usize) -> usize {
    let depth = indent_of(&lines[header]);
    (header + 1..lines.len())
```

with

```rust
/// Where the block headed by `header` ends: the first structural line indented no deeper than the
/// header itself, or the end of the file.
pub(crate) fn block_end(lines: &[String], header: usize) -> usize {
    let depth = indent_of(&lines[header]);
    (header + 1..lines.len())
```

In `engine/src/coursework.rs`, replace

```rust
/// header's own plus 2 when it has none yet. Never a constant — a student who indents by four
/// keeps indenting by four.
fn child_indent(lines: &[String], header: usize, end: usize) -> usize {
    (header + 1..end)
        .find(|&i| !is_filler(&lines[i]))
```

with

```rust
/// header's own plus 2 when it has none yet. Never a constant — a student who indents by four
/// keeps indenting by four.
pub(crate) fn child_indent(lines: &[String], header: usize, end: usize) -> usize {
    (header + 1..end)
        .find(|&i| !is_filler(&lines[i]))
```

In `engine/src/config.rs`, replace

```rust
use serde_yaml_ng::{Mapping, Value};

/// The six shared settings files (D17, D18), sorted.
```

with

```rust
use serde_yaml_ng::{Mapping, Value};

use crate::coursework::{block_end, child_indent, find_key, indent_of, is_filler};

/// The six shared settings files (D17, D18), sorted.
```

Append to `engine/src/config.rs` (after one blank line):

```rust
/// A file's lines with its final newline set aside, and the inverse.
fn lines_of(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn text_of(lines: &[String]) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// The value after `key:` on a key line, trimmed (`{}`, `[]`, a scalar, or empty for a block).
fn inline_value(line: &str) -> String {
    line.trim_start().split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default()
}

/// The last non-filler line in `(after, end)`, if any.
fn last_structural(lines: &[String], after: usize, end: usize) -> Option<usize> {
    (after + 1..end.min(lines.len())).rev().find(|&i| !is_filler(&lines[i]))
}

/// Make `path` (keys from the top, e.g. `["coursework", "vhl"]`) a block in `lines`, creating each
/// missing header as the last child of its parent and opening an empty `{}`/`[]` into block form.
/// Returns the innermost header's line and the indent its children take.
fn ensure_block(lines: &mut Vec<String>, path: &[&str]) -> (usize, usize) {
    let (mut from, mut end, mut depth, mut parent) = (0usize, lines.len(), 0usize, None::<usize>);
    let mut header = 0usize;
    for key in path {
        header = match find_key(lines, from, end, key, depth) {
            Some(at) => at,
            None => {
                let at = match parent {
                    None => lines.len(),
                    Some(p) => last_structural(lines, p, end).map(|l| l + 1).unwrap_or(p + 1),
                };
                lines.insert(at, format!("{}{key}:", " ".repeat(depth)));
                at
            }
        };
        let value = inline_value(&lines[header]);
        if matches!(value.as_str(), "{}" | "[]" | "{ }" | "[ ]") {
            lines[header] = format!("{}{key}:", " ".repeat(indent_of(&lines[header])));
        }
        end = block_end(lines, header);
        depth = child_indent(lines, header, end);
        from = header + 1;
        parent = Some(header);
    }
    (header, depth)
}

/// Append `children` (already indented) as the last children of the block `path` names.
fn insert_children(lines: &mut Vec<String>, path: &[&str], children: &[String]) {
    let (header, _) = ensure_block(lines, path);
    let end = block_end(lines, header);
    let at = last_structural(lines, header, end).map(|l| l + 1).unwrap_or(header + 1);
    for (offset, line) in children.iter().enumerate() {
        lines.insert(at + offset, line.clone());
    }
}

/// Read, edit and write `config/device.yaml` in place; a missing file is born empty first. An edit
/// whose result would not parse is refused and the file is left as it was.
fn edit_device(vault: &Path, edit: impl FnOnce(&mut Vec<String>)) -> Result<(), String> {
    let path = vault.join(DEVICE_FILE);
    if !path.exists() {
        write_new_device_yaml(vault, &DeviceKeys::default())?;
    }
    let text = crate::pystr::read_text(&path).map_err(|e| e.to_string())?;
    let mut lines = lines_of(&text);
    edit(&mut lines);
    let out = text_of(&lines);
    if serde_yaml_ng::from_str::<Value>(&out).is_err() {
        return Err(format!("{DEVICE_FILE} would not parse after the edit; left as it was"));
    }
    crate::pystr::write_text(&path, &out).map_err(|e| e.to_string())
}

/// Set one source's device key when `device.yaml` does not already hold it. `Ok(false)`: it did.
pub fn insert_source_key(vault: &Path, source: &str, key: &str, literal: &str) -> Result<bool, String> {
    let held = load_device(vault);
    let has = get(&held, "coursework")
        .and_then(Value::as_mapping)
        .and_then(|cw| get(cw, source))
        .and_then(Value::as_mapping)
        .is_some_and(|b| get(b, key).is_some());
    if has {
        return Ok(false);
    }
    edit_device(vault, |lines| {
        let (_, indent) = ensure_block(lines, &["coursework", source]);
        insert_children(lines, &["coursework", source], &[format!("{}{key}: {literal}", " ".repeat(indent))]);
    })?;
    Ok(true)
}

/// Set the `local` runner's `device` or `scheduler` when `device.yaml` does not already hold it.
pub fn insert_runner_key(vault: &Path, key: &str, literal: &str) -> Result<bool, String> {
    let held = load_device(vault);
    if get(&held, "runner").and_then(Value::as_mapping).is_some_and(|r| get(r, key).is_some()) {
        return Ok(false);
    }
    edit_device(vault, |lines| {
        let (_, indent) = ensure_block(lines, &["runner"]);
        insert_children(lines, &["runner"], &[format!("{}{key}: {literal}", " ".repeat(indent))]);
    })?;
    Ok(true)
}

/// `ics_url` in `device.yaml`: written when `replace` is set or the key is absent; otherwise a value
/// already there wins (`Ok(false)`).
pub fn set_device_ics_url(vault: &Path, url: &str, replace: bool) -> Result<bool, String> {
    if !replace && get(&load_device(vault), "ics_url").is_some() {
        return Ok(false);
    }
    let literal = quote("LMS feed URL", url)?;
    edit_device(vault, |lines| match find_key(lines, 0, lines.len(), "ics_url", 0) {
        Some(at) => lines[at] = format!("ics_url: {literal}"),
        None => lines.push(format!("ics_url: {literal}")),
    })?;
    Ok(true)
}

/// Add one raw calendar to `device.yaml` unless an entry of that name is already there.
pub fn add_device_calendar(vault: &Path, name: &str, url: &str) -> Result<bool, String> {
    let taken = matches!(get(&load_device(vault), "calendars"), Some(Value::Sequence(items))
        if items.iter().any(|e| e.get("name").and_then(Value::as_str) == Some(name)));
    if taken {
        return Ok(false);
    }
    let (n, u) = (quote("calendar name", name)?, quote("calendar address", url)?);
    edit_device(vault, |lines| {
        let (_, indent) = ensure_block(lines, &["calendars"]);
        let pad = " ".repeat(indent);
        insert_children(lines, &["calendars"], &[format!("{pad}- name: {n}"), format!("{pad}  ics_url: {u}")]);
    })?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// Where a device value sits in a shared file: the guard and the migration read the same answer
// ---------------------------------------------------------------------------------------------

/// A device-owned value at its D17 position in a shared file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSpot {
    /// `coursework.<source>.<key>` in `config/ingest.yaml`.
    SourceKey { source: String, key: String },
    /// The top-level `ics_url` in `config/ingest.yaml`.
    IcsUrl,
    /// The `index`th `calendars:` entry of `config/ingest.yaml`, whose address is not a marker.
    Calendar { index: usize, name: String },
    /// `device` or `scheduler` of the `index`th `runners:` entry of `config/runners.yaml` (`local`).
    Runner { index: usize, key: String },
}

impl DeviceSpot {
    /// The name the guard's line prints.
    pub fn label(&self) -> String {
        match self {
            DeviceSpot::SourceKey { source, key } => format!("coursework.{source}.{key}"),
            DeviceSpot::IcsUrl => "ics_url".to_string(),
            DeviceSpot::Calendar { name, .. } => format!("calendars.{name}"),
            DeviceSpot::Runner { key, .. } => format!("runners.local.{key}"),
        }
    }
}

/// Every device-owned value the parsed shared file `rel` carries, in file order.
pub fn device_spots(rel: &str, doc: &Mapping) -> Vec<(DeviceSpot, Value)> {
    let mut out = Vec::new();
    if rel == "config/ingest.yaml" {
        if let Some(Value::Mapping(cw)) = get(doc, "coursework") {
            for (name, block) in cw {
                let (Some(source), Some(block)) = (name.as_str(), block.as_mapping()) else { continue };
                for key in SOURCE_DEVICE_KEYS {
                    if let Some(v) = get(block, key) {
                        out.push((DeviceSpot::SourceKey { source: source.to_string(), key: key.to_string() }, v.clone()));
                    }
                }
            }
        }
        if let Some(v) = get(doc, "ics_url") {
            out.push((DeviceSpot::IcsUrl, v.clone()));
        }
        if let Some(Value::Sequence(items)) = get(doc, "calendars") {
            for (index, item) in items.iter().enumerate() {
                let Some(entry) = item.as_mapping() else { continue };
                let url = get(entry, "ics_url").and_then(Value::as_str).unwrap_or("").trim();
                if url.is_empty() || is_cloud_marker(url) {
                    continue;
                }
                let name = get(entry, "name").and_then(crate::yaml::text).unwrap_or_else(|| "calendar".to_string());
                out.push((DeviceSpot::Calendar { index, name }, item.clone()));
            }
        }
    }
    if rel == "config/runners.yaml" {
        if let Some(Value::Sequence(items)) = get(doc, "runners") {
            for (index, item) in items.iter().enumerate() {
                let Some(entry) = item.as_mapping() else { continue };
                if get(entry, "name").and_then(Value::as_str) != Some("local") {
                    continue;
                }
                for key in RUNNER_DEVICE_KEYS {
                    if let Some(v) = get(entry, key) {
                        out.push((DeviceSpot::Runner { index, key: key.to_string() }, v.clone()));
                    }
                }
            }
        }
    }
    out
}

/// A value that is an address somebody could fetch: the three schemes a capability URL arrives in.
fn is_url(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    t.starts_with("https://") || t.starts_with("http://") || t.starts_with("webcal://")
}

/// Every string value in `doc` that is a fetchable address, as `dotted.path` labels — except
/// `config/events.yaml`'s own `sources[<n>].url`, the public campus feeds.
fn addresses(rel: &str, doc: &Value, path: &str, out: &mut Vec<String>) {
    match doc {
        Value::String(s) if is_url(s) => {
            let campus_feed = rel == "config/events.yaml"
                && path.starts_with("sources[")
                && path.ends_with("].url")
                && !path["sources[".len()..path.len() - "].url".len()].contains(['.', '[']);
            if !campus_feed {
                out.push(path.to_string());
            }
        }
        Value::Mapping(m) => {
            for (k, v) in m {
                let key = crate::pystr::yaml_str(k);
                let next = if path.is_empty() { key } else { format!("{path}.{key}") };
                addresses(rel, v, &next, out);
            }
        }
        Value::Sequence(items) => {
            for (i, v) in items.iter().enumerate() {
                addresses(rel, v, &format!("{path}[{i}]"), out);
            }
        }
        Value::Tagged(t) => addresses(rel, &t.value, path, out),
        _ => {}
    }
}

/// D17's wedge guard: the first reason the shared file `rel`, holding `text`, may not leave this
/// computer — a device key at its D17 position, a raw calendar address, or any other address
/// outside `config/events.yaml`'s campus feeds — or `Ok(None)` when it may. `Err` is a file that is
/// not readable YAML, which cannot be proved clean and so does not travel either.
pub fn computer_only_value(rel: &str, text: &str) -> Result<Option<String>, String> {
    let doc: Value = serde_yaml_ng::from_str(&crate::pystr::universal_newlines(text)).map_err(|e| e.to_string())?;
    if let Value::Mapping(map) = &doc {
        if let Some((spot, _)) = device_spots(rel, map).into_iter().next() {
            return Ok(Some(spot.label()));
        }
    }
    let mut found = Vec::new();
    addresses(rel, &doc, "", &mut found);
    Ok(found.into_iter().next())
}

// ---------------------------------------------------------------------------------------------
// The migration (D17): text only, device.yaml first, the shared lines second
// ---------------------------------------------------------------------------------------------

/// The key line at `at` and every line that belongs to its value: deeper-indented lines, and the
/// filler between them — never a trailing blank line or comment.
fn key_span(lines: &[String], at: usize) -> std::ops::Range<usize> {
    let depth = indent_of(&lines[at]);
    let mut end = at + 1;
    let mut i = at + 1;
    while i < lines.len() {
        if is_filler(&lines[i]) {
            i += 1;
            continue;
        }
        if indent_of(&lines[i]) <= depth {
            break;
        }
        i += 1;
        end = i;
    }
    at..end
}

/// The first line of each item of the block sequence under `header`, and where the sequence ends.
/// `None` when the value is not a block sequence (a flow `[...]`, a scalar).
fn sequence_items(lines: &[String], header: usize) -> Option<(Vec<usize>, usize)> {
    let value = inline_value(&lines[header]);
    if !value.is_empty() {
        return if value == "[]" { Some((Vec::new(), header + 1)) } else { None };
    }
    let Some(first) = (header + 1..lines.len()).find(|&i| !is_filler(&lines[i])) else {
        return Some((Vec::new(), header + 1));
    };
    let dash = |i: usize| lines[i].trim_start().starts_with("- ") || lines[i].trim() == "-";
    if !dash(first) || indent_of(&lines[first]) < indent_of(&lines[header]) {
        return Some((Vec::new(), header + 1));
    }
    let item_indent = indent_of(&lines[first]);
    let (mut starts, mut end) = (Vec::new(), lines.len());
    for i in first..lines.len() {
        if is_filler(&lines[i]) {
            continue;
        }
        let ind = indent_of(&lines[i]);
        if ind < item_indent || (ind == item_indent && !dash(i)) {
            end = i;
            break;
        }
        if ind == item_indent {
            starts.push(i);
        }
    }
    Some((starts, end))
}

/// The span of the `index`th item: its dash line to the next item, trailing filler excluded.
fn item_span(lines: &[String], starts: &[usize], end: usize, index: usize) -> Option<std::ops::Range<usize>> {
    let start = *starts.get(index)?;
    let stop = starts.get(index + 1).copied().unwrap_or(end);
    let last = last_structural(lines, start, stop).unwrap_or(start);
    Some(start..last + 1)
}

/// The lines `spot` occupies in `lines`, or `None` when it is not on lines of its own (a flow
/// mapping, a key on an item's dash line) and so cannot be moved by line deletion.
fn spot_span(lines: &[String], spot: &DeviceSpot) -> Option<std::ops::Range<usize>> {
    match spot {
        DeviceSpot::SourceKey { source, key } => {
            let cw = find_key(lines, 0, lines.len(), "coursework", 0)?;
            let cw_end = block_end(lines, cw);
            let src = find_key(lines, cw + 1, cw_end, source, child_indent(lines, cw, cw_end))?;
            let src_end = block_end(lines, src);
            let at = find_key(lines, src + 1, src_end, key, child_indent(lines, src, src_end))?;
            Some(key_span(lines, at))
        }
        DeviceSpot::IcsUrl => find_key(lines, 0, lines.len(), "ics_url", 0).map(|at| key_span(lines, at)),
        DeviceSpot::Calendar { index, .. } => {
            let header = find_key(lines, 0, lines.len(), "calendars", 0)?;
            let (starts, end) = sequence_items(lines, header)?;
            item_span(lines, &starts, end, *index)
        }
        DeviceSpot::Runner { index, key } => {
            let header = find_key(lines, 0, lines.len(), "runners", 0)?;
            let (starts, end) = sequence_items(lines, header)?;
            let item = item_span(lines, &starts, end, *index)?;
            let key_indent = (item.start + 1..item.end).find(|&i| !is_filler(&lines[i])).map(|i| indent_of(&lines[i]))?;
            let at = find_key(lines, item.start + 1, item.end, key, key_indent)?;
            Some(key_span(lines, at))
        }
    }
}

/// A spot `device.yaml` has a place for: every one but a source outside [`PORTAL_SOURCES`].
fn movable(spot: &DeviceSpot) -> bool {
    !matches!(spot, DeviceSpot::SourceKey { source, .. } if !PORTAL_SOURCES.contains(&source.as_str()))
}

/// The value a spot moves as, for `device.yaml`.
fn text_value(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => crate::yaml::text(other).unwrap_or_default(),
    }
}

/// D17's migration. Gathers every device-owned value from `config/ingest.yaml` and
/// `config/runners.yaml`; writes `config/device.yaml` whole from literals when it is new (a
/// temporary file and a rename), or inserts what it lacks when it exists — a value `device.yaml`
/// already holds wins, the readers' own rule; and only then deletes the moved lines from the shared
/// files, line by line. **No file is parsed and re-dumped.** Idempotent: a vault with nothing left
/// to move returns no line and writes nothing, so `sync` runs it on every run. A crash between the
/// two steps leaves a duplicate the readers' device-first rule makes harmless and the push guard
/// keeps local; the next run finishes it.
///
/// Lines, never an error: `config: moved <n> computer-only value(s) into config/device.yaml`, and
/// one `config: config/<file> — <key> could not be moved (<why>); it stays on this computer` for a
/// value it cannot move.
pub fn move_device_keys(vault: &Path) -> Vec<String> {
    let mut lines_out = Vec::new();
    let mut found: Vec<(&str, Vec<(DeviceSpot, Value)>)> = Vec::new();
    for rel in ["config/ingest.yaml", "config/runners.yaml"] {
        let Ok(text) = crate::pystr::read_text(&vault.join(rel)) else { continue };
        let Ok(Value::Mapping(doc)) = serde_yaml_ng::from_str::<Value>(&text) else { continue };
        let mut spots = device_spots(rel, &doc);
        // A source `device.yaml` has no place for stays where it is, named; the guard keeps the
        // file on this computer meanwhile.
        spots.retain(|(spot, _)| {
            if !movable(spot) {
                lines_out.push(format!(
                    "config: {rel} — {} could not be moved (not a coursework source this computer knows); it stays on this computer",
                    spot.label()
                ));
            }
            movable(spot)
        });
        if !spots.is_empty() {
            found.push((rel, spots));
        }
    }
    if found.is_empty() {
        return lines_out;
    }
    // Step 1: device.yaml holds every value before any shared line goes.
    let fresh = !vault.join(DEVICE_FILE).exists();
    let mut keys = DeviceKeys::default();
    let mut written: Result<(), String> = Ok(());
    for (spot, value) in found.iter().flat_map(|(_, s)| s.iter()) {
        let step = match spot {
            DeviceSpot::SourceKey { source, key } => {
                let entry = keys.sources.entry(source.clone()).or_default();
                let literal = match key.as_str() {
                    "enabled" => {
                        entry.enabled = Some(crate::pystr::yaml_truthy(value));
                        Ok(crate::pystr::yaml_truthy(value).to_string())
                    }
                    "credential_target" => {
                        entry.credential_target = Some(text_value(value));
                        quote(&format!("{source} credential target"), &text_value(value))
                    }
                    _ => {
                        entry.base_url = Some(text_value(value));
                        quote(&format!("{source} base url"), &text_value(value))
                    }
                };
                if fresh { literal.map(|_| ()) } else { literal.and_then(|l| insert_source_key(vault, source, key, &l)).map(|_| ()) }
            }
            DeviceSpot::IcsUrl => {
                keys.ics_url = text_value(value);
                if fresh { Ok(()) } else { set_device_ics_url(vault, &text_value(value), false).map(|_| ()) }
            }
            DeviceSpot::Calendar { name, .. } => {
                let url = value.get("ics_url").map(text_value).unwrap_or_default();
                keys.calendars.push((name.clone(), url.clone()));
                if fresh { Ok(()) } else { add_device_calendar(vault, name, &url).map(|_| ()) }
            }
            DeviceSpot::Runner { key, .. } => {
                let v = text_value(value);
                let literal = if key == "device" {
                    keys.device = Some(v.clone());
                    quote("device name", &v)
                } else {
                    keys.scheduler = Some(v.clone());
                    scheduler_literal(&v)
                };
                if fresh { literal.map(|_| ()) } else { literal.and_then(|l| insert_runner_key(vault, key, &l)).map(|_| ()) }
            }
        };
        if let Err(e) = step {
            written = Err(format!("{} ({e})", spot.label()));
            break;
        }
    }
    if written.is_ok() && fresh {
        written = write_new_device_yaml(vault, &keys);
    }
    if let Err(e) = written {
        lines_out.push(format!("config: {DEVICE_FILE} could not be written ({e}); nothing was moved"));
        return lines_out;
    }
    // Step 2: the moved lines leave the shared files, one file at a time.
    let mut moved = 0usize;
    for (rel, spots) in &found {
        let path = vault.join(rel);
        let Ok(text) = crate::pystr::read_text(&path) else { continue };
        let mut lines = lines_of(&text);
        let mut spans: Vec<std::ops::Range<usize>> = Vec::new();
        for (spot, _) in spots {
            match spot_span(&lines, spot) {
                Some(span) => spans.push(span),
                None => lines_out.push(format!(
                    "config: {rel} — {} could not be moved (it is not on a line of its own); it stays on this computer",
                    spot.label()
                )),
            }
        }
        spans.sort_by_key(|s| std::cmp::Reverse(s.start));
        let count = spans.len();
        for span in spans {
            lines.drain(span);
        }
        // A `calendars:` left with no entry keeps its shape as an empty list.
        if let Some(header) = find_key(&lines, 0, lines.len(), "calendars", 0) {
            if inline_value(&lines[header]).is_empty() && sequence_items(&lines, header).is_some_and(|(s, _)| s.is_empty()) {
                lines[header] = "calendars: []".to_string();
            }
        }
        let out = text_of(&lines);
        let still = match serde_yaml_ng::from_str::<Value>(&out) {
            Ok(Value::Mapping(doc)) => device_spots(rel, &doc).iter().filter(|(s, _)| movable(s)).count(),
            _ => usize::MAX,
        };
        if still == usize::MAX || still + count != spots.len() {
            lines_out.push(format!("config: {rel} could not be edited cleanly; it stays on this computer as it was"));
            continue;
        }
        match crate::pystr::write_text(&path, &out) {
            Ok(()) => moved += count,
            Err(e) => lines_out.push(format!("config: {rel} could not be written ({e}); it stays on this computer as it was")),
        }
    }
    if moved > 0 {
        lines_out.insert(0, format!("config: moved {moved} computer-only value(s) into {DEVICE_FILE}"));
    }
    lines_out
}
```

- [ ] **Step 4: Run the tests**

Run the command of Step 2. Expected: PASS, all eleven. Then
`cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- coursework::`
— PASS: `write_mapping`'s own tests are unchanged by the visibility change.

- [ ] **Step 5: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-2.txt`:

```
feat: the device-key migration and the push guard's test (two desktops, D17)

move_device_keys gathers every device-owned value from ingest.yaml and
runners.yaml, writes config/device.yaml whole from literals (or inserts
what it lacks), and only then deletes the moved lines - text only, never a
re-dump, idempotent, and a crash between the two steps is harmless.
computer_only_value names a device key at its D17 position, a raw calendar
address or any other capability URL outside events.yaml's campus feeds.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/config.rs engine/src/coursework.rs engine/tests/device_config.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-2.txt`

---

### Task 3: The cloud — the path check, `save_config_row`, the conditional push, the settings view (spec §4.8 D18, §6.3)

**Files:**
- Create: `cloud/supabase/migrations/20260926000200_shared_settings.sql` — the widened
  `sync_notes_path_check` and `save_config_row` (R-TD2-18 for the stamp and the `commitments` folder).
- Modify: `cloud/supabase/functions/_shared/sync_rows.ts` — `SHARED_CONFIG`, `isSharedConfig`, and
  `checkNote`'s settings branch (`{path, body, base}`, never a tombstone).
- Modify: `cloud/supabase/functions/_shared/sync_db.ts` — `saveConfigRow` (through `Db.rpc`, so
  `judge_db_test.ts`'s scan holds it to passing `p_account`), `readSettings`, `hasNotes`.
- Modify: `cloud/supabase/functions/sync-push/handler.ts` and `index.ts` — settings rows kept apart and
  stored one compare-and-set each; the reply's `config` and `config_refused`.
- Modify: `cloud/supabase/functions/sync-pull/handler.ts` and `index.ts` — `handleSettings`, routed on
  `?view=settings`.
- Tests: `functions/_shared/sync_rows_test.ts`, `functions/sync-push/handler_test.ts`,
  `functions/sync-pull/handler_test.ts`, `migrations/migrations_test.ts` (the function pin **28 → 29**,
  and the migration's own pins).

**Interfaces:**
- Consumes: `_shared/http.ts::{fail, json, methodNotAllowed}`, `_shared/db.ts::{Rest, restSelect}`,
  `_shared/judge_db.ts::{Db, serviceDb}`, `_shared/entitlement.ts::requireActiveEntitlement`.
- Produces: `POST /sync-push`'s settings rows and reply, `GET /sync-pull?view=settings`,
  `save_config_row`, `sync_rows.ts::{SHARED_CONFIG, isSharedConfig}`, `sync_db.ts::{saveConfigRow,
  readSettings, hasNotes}`, `sync-pull/handler.ts::{SettingsDeps, handleSettings}`.

**No `supabase` command.** The controller deploys the migration and both functions to staging after this
task, and to production before any release carrying Task 4 (*Hand-off to the controller*, item 2). Why the
handler never passes a settings row to `saveNotes`: its upsert is unconditional (`sync_db.ts:23-26`), so
of two pushes of one file the later would win and the earlier pusher would lose its change in silence
(re-review R2-I1). Settings rows are stored after the batch's records and notes, which stay all or
nothing as today; a refused one is named, never an error.

- [ ] **Step 1: Write the failing tests**

In `cloud/supabase/functions/_shared/sync_rows_test.ts`, replace

```ts
import { assert, assertEquals, assertThrows } from "@std/assert";
import { checkNote, checkRecord, isNotePath, MAX_NOTE_BYTES, MAX_RECORD_BYTES, NOTE_PATH_RE } from "./sync_rows.ts";

const DEVICE = "0123456789abcdef";
```

with

```ts
import { assert, assertEquals, assertThrows } from "@std/assert";
import {
  checkNote,
  checkRecord,
  isNotePath,
  isSharedConfig,
  MAX_NOTE_BYTES,
  MAX_RECORD_BYTES,
  NOTE_PATH_RE,
  SHARED_CONFIG,
} from "./sync_rows.ts";

const DEVICE = "0123456789abcdef";
```

In `cloud/supabase/functions/_shared/sync_rows_test.ts`, replace

```ts
Deno.test("every sync query is scoped to one account, and nothing bypasses C1's helpers", async () => {
  const src = await Deno.readTextFile(new URL("./sync_db.ts", import.meta.url));
  for (const table of ["sync_records", "sync_notes", "sync_usage"]) {
    for (const call of src.matchAll(new RegExp(`"${table}",\\s*\`([^\`]*)\``, "g"))) {
```

with

```ts
Deno.test("every sync query is scoped to one account, and nothing bypasses C1's helpers", async () => {
  const src = await Deno.readTextFile(new URL("./sync_db.ts", import.meta.url));
  assert(src.includes('db.rpc("save_config_row", { p_account: accountId,'), "the settings write names the account first");
  for (const table of ["sync_records", "sync_notes", "sync_usage"]) {
    for (const call of src.matchAll(new RegExp(`"${table}",\\s*\`([^\`]*)\``, "g"))) {
```

In `cloud/supabase/functions/_shared/sync_rows_test.ts`, replace

```ts
  assert(!src.includes("sync_generation"), "the key generation is gone");
});
```

with

```ts
  assert(!src.includes("sync_generation"), "the key generation is gone");
});

Deno.test("the six shared settings paths are the migration's, and a settings row keeps its base", async () => {
  // Two-desktop design D18. One list in three places — here, `engine/src/config.rs` and the column
  // check — so it is pinned from this side against the migration's own text.
  const sql = await Deno.readTextFile(new URL("../../migrations/20260926000200_shared_settings.sql", import.meta.url));
  const listed = SHARED_CONFIG.map((p) => `'${p}'`).join(", ");
  assert(sql.replace(/\s+/g, " ").includes(listed), "sync_notes' path check names exactly SHARED_CONFIG, in order");
  for (const p of SHARED_CONFIG) {
    assert(isSharedConfig(p), p);
    assert(!isNotePath(p), `${p} is a settings path, never a note path`);
  }
  const base = "b".repeat(64);
  const row = checkNote({ path: "config/ingest.yaml", body: "timezone: x\n", base }, DEVICE, ACCOUNT);
  assertEquals(row.base, base);
  assertEquals(row.deleted, false);
  const e = assertThrows(() => checkNote({ path: "config/ingest.yaml", body: "x", base, extra: 1 }, DEVICE, ACCOUNT)) as Response;
  assertEquals(e.status, 400);
});
```

In `cloud/supabase/functions/sync-push/handler_test.ts`, replace

```ts
    saveRecords: () => Promise.resolve(),
    saveNotes: () => Promise.resolve(),
    ...over,
  };
```

with

```ts
    saveRecords: () => Promise.resolve(),
    saveNotes: () => Promise.resolve(),
    saveConfigRow: () => Promise.resolve(true),
    ...over,
  };
```

In `cloud/supabase/functions/sync-push/handler_test.ts`, replace

```ts
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { records: 2, notes: 2, bytes_used: 0, bytes_ceiling: 1_000_000 });
  assertEquals(records.length, 2);
  assertEquals((records[0] as Record<string, unknown>).account_id, "acct-1");
```

with

```ts
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { records: 2, notes: 2, config: 0, config_refused: [], bytes_used: 0, bytes_ceiling: 1_000_000 });
  assertEquals(records.length, 2);
  assertEquals((records[0] as Record<string, unknown>).account_id, "acct-1");
```

In `cloud/supabase/functions/sync-push/handler_test.ts`, replace

```ts
  assertEquals(res.status, 405);
});
```

with

```ts
  assertEquals(res.status, 405);
});

// ---------------------------------------------------------------------------------------------
// Two-desktop design D18 (§4.8, §6.3): the six shared settings files, and the conditional push.
// ---------------------------------------------------------------------------------------------

const SETTINGS = "timezone: 'America/Chicago'\n";

/** An account's `sync_notes` settings rows in memory, stored exactly as `save_config_row` stores them:
 * an empty base inserts only where no row exists; a base updates only a row whose text hashes to it. */
function configStore() {
  const rows = new Map<string, string>();
  const saveConfigRow = async (_account: string, path: string, body: string, _device: string, base: string) => {
    const held = rows.get(path);
    if (base === "" ? held !== undefined : held === undefined || (await sha256Hex(held)) !== base) return false;
    rows.set(path, body);
    return true;
  };
  return { rows, saveConfigRow };
}

Deno.test("the six shared settings paths are accepted, and every other config/ path is refused", async () => {
  const store = configStore();
  let noteRows: unknown[] = [];
  const paths = [
    "config/campus.yaml", "config/events.yaml", "config/ingest.yaml",
    "config/planning.yaml", "config/runners.yaml", "config/week_template.yaml",
  ];
  const res = await handle(
    push({ device: DEVICE, notes: paths.map((path) => ({ path, body: SETTINGS, base: "" })) }),
    deps({ saveConfigRow: store.saveConfigRow, saveNotes: (n) => { noteRows = n; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals([body.notes, body.config, body.config_refused], [0, 6, []]);
  assertEquals([...store.rows.keys()].sort(), paths);
  assertEquals(noteRows, [], "a settings row never goes through saveNotes' upsert");
  for (const path of ["config/device.yaml", "config/cloud.yaml", "config/other.yaml", "config/ingest.yml"]) {
    const refused = await refusal(push({ device: DEVICE, notes: [{ path, body: SETTINGS, base: "" }] }), deps());
    assertEquals(refused.status, 400, `${path} was accepted`);
  }
});

Deno.test("a tombstone for a shared settings path is refused, and base is accepted only on one", async () => {
  const dead = await refusal(push({ device: DEVICE, notes: [{ path: "config/ingest.yaml", deleted: true }] }), deps());
  assertEquals(dead.status, 400);
  const onNote = await refusal(push({ device: DEVICE, notes: [{ path: "tasks/x.md", body: "x", base: "" }] }), deps());
  assertEquals(onNote.status, 400);
  assert((await onNote.json()).error.includes("base"), "the refusal names the field");
  const noBase = await refusal(push({ device: DEVICE, notes: [{ path: "config/ingest.yaml", body: SETTINGS }] }), deps());
  assertEquals(noBase.status, 400, "a settings row always says what it was based on");
  const badBase = await refusal(push({ device: DEVICE, notes: [{ path: "config/ingest.yaml", body: SETTINGS, base: "abc" }] }), deps());
  assertEquals(badBase.status, 400);
});

Deno.test("two pushes of one path from one base: the first is stored, the second is config_refused", async () => {
  const store = configStore();
  store.rows.set("config/planning.yaml", "daily_effort_budget: 4.0\n");
  const base = await sha256Hex("daily_effort_budget: 4.0\n");
  const first = await handle(
    push({ device: DEVICE, notes: [{ path: "config/planning.yaml", body: "daily_effort_budget: 5.0\n", base }] }),
    deps({ saveConfigRow: store.saveConfigRow }),
  );
  assertEquals((await first.json()).config_refused, []);
  const second = await handle(
    push({ device: "fedcba9876543210", notes: [{ path: "config/planning.yaml", body: "daily_effort_budget: 6.0\n", base }] }),
    deps({ saveConfigRow: store.saveConfigRow }),
  );
  const body = await second.json();
  assertEquals(second.status, 200);
  assertEquals([body.config, body.config_refused], [0, ["config/planning.yaml"]]);
  assertEquals(store.rows.get("config/planning.yaml"), "daily_effort_budget: 5.0\n", "the first change is never lost");
  // An empty base is an insert only where the account holds nothing.
  const late = await handle(
    push({ device: DEVICE, notes: [{ path: "config/planning.yaml", body: "x: 1\n", base: "" }] }),
    deps({ saveConfigRow: store.saveConfigRow }),
  );
  assertEquals((await late.json()).config_refused, ["config/planning.yaml"]);
});

Deno.test("settings bytes count toward the ceiling like any row", async () => {
  let asked = 0;
  const res = await refusal(
    push({ device: DEVICE, notes: [{ path: "config/ingest.yaml", body: SETTINGS, base: "" }] }),
    deps({
      bytesUsed: () => Promise.resolve(1_000_000),
      ceiling: () => Promise.resolve(1_000_000),
      saveConfigRow: () => { asked += 1; return Promise.resolve(true); },
    }),
  );
  assertEquals(res.status, 403);
  assertEquals(asked, 0);
});
```

In `cloud/supabase/functions/sync-pull/handler_test.ts`, replace

```ts
import { assertEquals } from "@std/assert";
import { handle, MAX_PAGE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
```

with

```ts
import { assertEquals } from "@std/assert";
import { handle, handleSettings, MAX_PAGE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
```

In `cloud/supabase/functions/sync-pull/handler_test.ts`, replace

```ts
  assertEquals(res.status, 405);
});
```

with

```ts
  assertEquals(res.status, 405);
});

Deno.test("the settings view answers the account's shared settings and whether it holds notes", async () => {
  // Two-desktop design D19: the second computer's wizard asks this after sign-in.
  const asked: string[] = [];
  const res = await handleSettings(pull("?view=settings"), {
    requireEntitled: OK,
    readSettings: (a) => { asked.push(a); return Promise.resolve([{ path: "config/ingest.yaml", body: "timezone: x\n" }]); },
    hasNotes: (a) => { asked.push(a); return Promise.resolve(true); },
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { settings: [{ path: "config/ingest.yaml", body: "timezone: x\n" }], notes: true });
  assertEquals(asked, ["acct-1", "acct-1"], "the account comes from the entitlement alone");
  const post = await handleSettings(new Request("http://127.0.0.1/sync-pull?view=settings", { method: "POST" }), {
    requireEntitled: OK, readSettings: () => Promise.resolve([]), hasNotes: () => Promise.resolve(false),
  });
  assertEquals(post.status, 405);
});
```

In `cloud/supabase/migrations/migrations_test.ts`, replace

```ts
  // trigger function, exempt by kind, one more definition parsed), plus C1c's R-C1c-7
  // `create or replace function charge_call` in 20260923000100 (still a writing, non-definer
  // function — revoked again in the same file, and one more definition parsed) —
  // counted by hand against today's corpus: C3′ adds six and C1c one, on top of the 21 both
  // streams inherited (the C3′ merge of main, PR #13).
  assertEquals(parsed, 28, "today's corpus should parse exactly 28 function creations");
});
```

with

```ts
  // trigger function, exempt by kind, one more definition parsed), plus C1c's R-C1c-7
  // `create or replace function charge_call` in 20260923000100 (still a writing, non-definer
  // function — revoked again in the same file, and one more definition parsed), plus the
  // two-desktop stream's `save_config_row` in 20260926000200 (a writing, non-definer function,
  // revoked from public, anon and authenticated in the same file; Plan 3's `fetch_turn` moves this
  // to 30) — counted by hand against today's corpus: C3′ adds six, C1c one and two-desktop one, on
  // top of the 21 the streams inherited (the C3′ merge of main, PR #13).
  assertEquals(parsed, 29, "today's corpus should parse exactly 29 function creations");
});
```

In `cloud/supabase/migrations/migrations_test.ts`, replace

```ts
    `${last!.name}: charge_call's single insert must still be the on-conflict upsert`,
  );
});
```

with

```ts
    `${last!.name}: charge_call's single insert must still be the on-conflict upsert`,
  );
});

Deno.test("the shared-settings migration widens the path check to exactly six settings paths and revokes save_config_row", async () => {
  // Two-desktop design D18 (§6.3). Read by name: this file is the one that defines both.
  const sql = await Deno.readTextFile(new URL("./20260926000200_shared_settings.sql", import.meta.url));
  const code = stripLineComments(sql);
  const six = "'config/campus.yaml', 'config/events.yaml', 'config/ingest.yaml', 'config/planning.yaml', " +
    "'config/runners.yaml', 'config/week_template.yaml'";
  const flat = code.replace(/\s+/g, " ");
  assert(flat.includes("drop constraint sync_notes_path_check, add constraint sync_notes_path_check check ("), "the check is replaced in place");
  assert(flat.includes(`or path in (${six})`), "exactly the six settings paths beside the note folders");
  assert(
    flat.includes("path ~ '^(tasks|approvals|archive|courses|issues|info|commitments)/[A-Za-z0-9._ /-]+\\.md$'"),
    "the note folders, with the commitment model's own (whichever stream merges first)",
  );
  assert(!/\{\d+,\d{3,}\}/.test(code), "no bound repetition over Postgres's DUPMAX");
  assert(/create\s+or\s+replace\s+function\s+public\.save_config_row\(/i.test(code), "the function exists");
  assert(!/security\s+definer/i.test(code), "not SECURITY DEFINER");
  assert(
    /revoke\s+execute\s+on\s+function\s+public\.save_config_row\(uuid, text, text, text, text\)\s+from\s+public,\s*anon,\s*authenticated;/i.test(code),
    "revoked from public, anon and authenticated",
  );
  assert(flat.includes("encode(sha256(convert_to(body, 'UTF8')), 'hex') = p_base"), "the compare is on the stored text's hash");
  assert(flat.includes("on conflict (account_id, path) do nothing"), "an empty base inserts only where no row exists");
  assert(!/create\s+table/i.test(code), "no table: settings rows are sync_notes rows");
});
```

- [ ] **Step 2: Run them to see them fail**

Run (PATH refreshed):
`deno test --allow-read --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\sync-push\ C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\sync-pull\ C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\_shared\sync_rows_test.ts`
Expected: FAIL type-checking — `sync_rows.ts` has no exported member `isSharedConfig` or `SHARED_CONFIG`,
`sync-pull/handler.ts` none named `handleSettings`, and `saveConfigRow` does not exist in `PushDeps`.
Then `deno test --allow-read --config …\deno.json …\cloud\supabase\migrations\migrations_test.ts`:
FAIL — "today's corpus should parse exactly 29 function creations" (28), and the new pin cannot read
`20260926000200_shared_settings.sql`.

- [ ] **Step 3: The migration, the rows, the handlers**

In `cloud/supabase/functions/_shared/sync_rows.ts`, replace

```ts
export const NOTE_PATH_RE = /^(tasks|approvals|archive|courses|issues|info)\/[A-Za-z0-9._ /-]{1,300}\.md$/;

export function isDeviceToken(x: unknown): boolean {
  return typeof x === "string" && DEVICE_RE.test(x);
```

with

```ts
export const NOTE_PATH_RE = /^(tasks|approvals|archive|courses|issues|info)\/[A-Za-z0-9._ /-]{1,300}\.md$/;

/** Two-desktop design D18: the six shared settings files that travel beside the notes, one row per
 * path — the same list as `engine/src/config.rs::SHARED_CONFIG` and `sync_notes`' path check
 * (`20260926000200_shared_settings.sql`). Any other `config/` path is refused. */
export const SHARED_CONFIG = [
  "config/campus.yaml",
  "config/events.yaml",
  "config/ingest.yaml",
  "config/planning.yaml",
  "config/runners.yaml",
  "config/week_template.yaml",
];

export function isSharedConfig(x: unknown): boolean {
  return typeof x === "string" && SHARED_CONFIG.includes(x);
}

export function isDeviceToken(x: unknown): boolean {
  return typeof x === "string" && DEVICE_RE.test(x);
```

In `cloud/supabase/functions/_shared/sync_rows.ts`, replace

```ts

export interface RecordIn { hash: string; body: string }
export interface NoteIn { path: string; deleted?: boolean; body?: string }

function noExtras(row: Record<string, unknown>, allowed: string[], what: string): void {
```

with

```ts

export interface RecordIn { hash: string; body: string }
export interface NoteIn { path: string; deleted?: boolean; body?: string; base?: string }

function noExtras(row: Record<string, unknown>, allowed: string[], what: string): void {
```

In `cloud/supabase/functions/_shared/sync_rows.ts`, replace

```ts
}

/** One pushed note → the row `sync_notes` takes. A tombstone carries no bytes; a live note carries them. */
export function checkNote(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a note is not an object");
  const row = raw as Record<string, unknown>;
  noExtras(row, ["path", "deleted", "body"], "note");
  if (!isNotePath(row.path)) throw fail(400, "a note has no usable path");
```

with

```ts
}

/** One pushed note → the row `sync_notes` takes. A tombstone carries no bytes; a live note carries them.
 *
 * Two-desktop design D18: a shared settings row is the same `{path, body}` plus `base`, the SHA-256
 * of the text the device last synced for that path (`""` for none) — accepted on a shared path
 * only, and never as a tombstone, so "a settings file is never tombstoned" holds against any client.
 * Its row keeps `base` for `save_config_row`; it never reaches `saveNotes`' upsert. */
export function checkNote(raw: unknown, device: string, accountId: string): Record<string, unknown> {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) throw fail(400, "a note is not an object");
  const row = raw as Record<string, unknown>;
  if (isSharedConfig(row.path)) {
    noExtras(row, ["path", "body", "base"], "settings row");
    if (typeof row.base !== "string" || (row.base !== "" && !isHash(row.base))) {
      throw fail(400, "a settings row carries its base: the sha256 of the text last synced, or empty");
    }
    if (typeof row.body !== "string" || row.body.length === 0) throw fail(400, "a settings row has no body");
    if (row.body.includes("\u0000")) throw fail(400, "a settings row's body contains a null byte");
    if (bytes(row.body) > MAX_NOTE_BYTES) throw fail(400, `a settings row's body is over ${MAX_NOTE_BYTES} bytes`);
    return { account_id: accountId, path: row.path as string, device, deleted: false, body: row.body, base: row.base };
  }
  noExtras(row, ["path", "deleted", "body"], "note");
  if (!isNotePath(row.path)) throw fail(400, "a note has no usable path");
```

In `cloud/supabase/functions/_shared/sync_db.ts`, replace

```ts
 */
import { Rest, restSelect, restUpsert } from "./db.ts";

/**
```

with

```ts
 */
import { Rest, restSelect, restUpsert } from "./db.ts";
import type { Db } from "./judge_db.ts";

/**
```

In `cloud/supabase/functions/_shared/sync_db.ts`, replace

```ts
  if (rows.length === 0) return;
  await restUpsert(rest, "sync_notes", rows, "account_id,path");
}
```

with

```ts
  if (rows.length === 0) return;
  await restUpsert(rest, "sync_notes", rows, "account_id,path");
}

/**
 * Two-desktop design D18: one settings row, stored only if the account's text for the path still
 * hashes to `base` (an empty base: only where the account holds none) — `save_config_row`, one
 * compare-and-set statement. `true` when it was stored. Through `Db.rpc`, so `judge_db_test.ts`'s
 * scan holds the call to passing `p_account`.
 */
export async function saveConfigRow(
  db: Db,
  accountId: string,
  path: string,
  body: string,
  device: string,
  base: string,
): Promise<boolean> {
  const stored = await db.rpc("save_config_row", { p_account: accountId, p_path: path, p_body: body, p_device: device, p_base: base });
  return stored === true;
}

/** Two-desktop design D19: the account's live shared settings rows, by path. */
export async function readSettings(rest: Rest, accountId: string): Promise<unknown[]> {
  return await restSelect(
    rest,
    "sync_notes",
    `select=path,body&account_id=eq.${accountId}&deleted=is.false&path=like.config/*&order=path.asc`,
  );
}

/** Two-desktop design D19: whether the account holds any live note (a row outside `config/`). */
export async function hasNotes(rest: Rest, accountId: string): Promise<boolean> {
  const rows = await restSelect(
    rest,
    "sync_notes",
    `select=path&account_id=eq.${accountId}&deleted=is.false&path=not.like.config/*&limit=1`,
  );
  return rows.length > 0;
}
```

In `cloud/supabase/functions/sync-push/handler.ts`, replace

```ts
  checkRecord,
  isDeviceToken,
  MAX_PUSH_BYTES,
  MAX_ROWS,
```

with

```ts
  checkRecord,
  isDeviceToken,
  isSharedConfig,
  MAX_PUSH_BYTES,
  MAX_ROWS,
```

In `cloud/supabase/functions/sync-push/handler.ts`, replace

```ts
  saveRecords: (rows: unknown[]) => Promise<void>;
  saveNotes: (rows: unknown[]) => Promise<void>;
}
```

with

```ts
  saveRecords: (rows: unknown[]) => Promise<void>;
  saveNotes: (rows: unknown[]) => Promise<void>;
  /** Two-desktop design D18: `save_config_row` — `true` when the row was stored. */
  saveConfigRow: (accountId: string, path: string, body: string, device: string, base: string) => Promise<boolean>;
}
```

In `cloud/supabase/functions/sync-push/handler.ts`, replace

```ts
  }
  const notes = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.notes, "notes")) {
    const row = checkNote(raw, device, account_id);
    notes.set(row.path as string, row);
  }

  const [used, cap] = await Promise.all([deps.bytesUsed(account_id), deps.ceiling()]);
  const adding = [...records.values(), ...notes.values()]
    .reduce((n, r) => n + (typeof r.body === "string" ? bytes(r.body) : 0), 0);
  // `adding > 0` (R-C3′-exec-9 m1): with no net bytes to add — an empty push, a tombstone-only one,
```

with

```ts
  }
  const notes = new Map<string, Record<string, unknown>>();
  // Two-desktop design D18: settings rows are kept apart from the note rows from here on — they are
  // stored by `save_config_row`'s compare-and-set, never by `saveNotes`' unconditional upsert.
  const settings = new Map<string, Record<string, unknown>>();
  for (const raw of listOf(body.notes, "notes")) {
    const row = checkNote(raw, device, account_id);
    (isSharedConfig(row.path) ? settings : notes).set(row.path as string, row);
  }

  const [used, cap] = await Promise.all([deps.bytesUsed(account_id), deps.ceiling()]);
  const adding = [...records.values(), ...notes.values(), ...settings.values()]
    .reduce((n, r) => n + (typeof r.body === "string" ? bytes(r.body) : 0), 0);
  // `adding > 0` (R-C3′-exec-9 m1): with no net bytes to add — an empty push, a tombstone-only one,
```

In `cloud/supabase/functions/sync-push/handler.ts`, replace

```ts
  await deps.saveRecords([...records.values()]);
  await deps.saveNotes([...notes.values()]);
  return json(200, { records: records.size, notes: notes.size, bytes_used: used, bytes_ceiling: cap });
}
```

with

```ts
  await deps.saveRecords([...records.values()]);
  await deps.saveNotes([...notes.values()]);
  // D18: each settings row after the batch's records and notes, one compare-and-set each. A refused
  // one is named in `config_refused`; the device keeps its text and its base, and its next pull takes
  // the conflict branch. No change is lost silently.
  const refused: string[] = [];
  for (const row of settings.values()) {
    const stored = await deps.saveConfigRow(account_id, row.path as string, row.body as string, device, row.base as string);
    if (!stored) refused.push(row.path as string);
  }
  return json(200, {
    records: records.size,
    notes: notes.size,
    config: settings.size - refused.length,
    config_refused: refused,
    bytes_used: used,
    bytes_ceiling: cap,
  });
}
```

In `cloud/supabase/functions/sync-push/index.ts`, replace

```ts
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { bytesUsed, ceiling, saveNotes, saveRecords } from "../_shared/sync_db.ts";
import { handle } from "./handler.ts";
```

with

```ts
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { serviceDb } from "../_shared/judge_db.ts";
import { bytesUsed, ceiling, saveConfigRow, saveNotes, saveRecords } from "../_shared/sync_db.ts";
import { handle } from "./handler.ts";
```

In `cloud/supabase/functions/sync-push/index.ts`, replace

```ts
      saveRecords: (rows) => saveRecords(rest, rows),
      saveNotes: (rows) => saveNotes(rest, rows),
    });
  } catch (e) {
```

with

```ts
      saveRecords: (rows) => saveRecords(rest, rows),
      saveNotes: (rows) => saveNotes(rest, rows),
      saveConfigRow: (accountId, path, body, device, base) => saveConfigRow(serviceDb(), accountId, path, body, device, base),
    });
  } catch (e) {
```

In `cloud/supabase/functions/sync-pull/handler.ts`, replace

```ts
  const n = Number(params.get(key));
  return Number.isSafeInteger(n) && n > 0 ? n : 0;
}
```

with

```ts
  const n = Number(params.get(key));
  return Number.isSafeInteger(n) && n > 0 ? n : 0;
}

/** Two-desktop design D19: what `GET /sync-pull?view=settings` reads. */
export interface SettingsDeps {
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  readSettings: (accountId: string) => Promise<unknown[]>;
  hasNotes: (accountId: string) => Promise<boolean>;
}

/**
 * `GET /sync-pull?view=settings` (two-desktop design D19): the account's shared settings rows, whole,
 * and whether it holds any note — the second computer's wizard asks this after sign-in, and a
 * device with a settings file missing reads it back. No cursor: six rows at most. The account comes
 * from the entitlement alone.
 */
export async function handleSettings(req: Request, deps: SettingsDeps): Promise<Response> {
  if (req.method !== "GET") return methodNotAllowed(["GET"]);
  const { account_id } = await deps.requireEntitled(req);
  const [settings, notes] = await Promise.all([deps.readSettings(account_id), deps.hasNotes(account_id)]);
  return json(200, { settings, notes });
}
```

In `cloud/supabase/functions/sync-pull/index.ts`, replace

```ts
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { readNotes, readRecords } from "../_shared/sync_db.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      requireEntitled: requireActiveEntitlement,
```

with

```ts
import { restFromEnv } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { hasNotes, readNotes, readRecords, readSettings } from "../_shared/sync_db.ts";
import { handle, handleSettings } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    // Two-desktop design D19: the settings view is its own read, with its own handler.
    if (new URL(req.url).searchParams.get("view") === "settings") {
      return await handleSettings(req, {
        requireEntitled: requireActiveEntitlement,
        readSettings: (accountId) => readSettings(rest, accountId),
        hasNotes: (accountId) => hasNotes(rest, accountId),
      });
    }
    return await handle(req, {
      requireEntitled: requireActiveEntitlement,
```

Create `cloud/supabase/migrations/20260926000200_shared_settings.sql`:

```sql
-- Two-desktop design D18 (§4.8): the account holds the student's shared settings.
--
-- Six `config/` files travel through C3′'s `sync_notes`, one text row per path, beside the notes:
-- `config/campus.yaml`, `config/events.yaml`, `config/ingest.yaml`, `config/planning.yaml`,
-- `config/runners.yaml`, `config/week_template.yaml` — the list `engine/src/config.rs::SHARED_CONFIG`
-- and `_shared/sync_rows.ts::SHARED_CONFIG` name. `config/device.yaml` and `config/cloud.yaml` are
-- this computer's and never travel. The rows are the account's data under C3′'s RLS, ceiling, purge
-- and export, like every other `sync_notes` row.
--
-- **The path check, widened in place** (20260912000400's own shape: drop and re-add the one
-- constraint; 20260912000300's two climb-out siblings stay). The six settings paths are accepted
-- beside the note folders, and no other `config/` path. **The folder group carries `commitments`**:
-- the commitment model's 20260926000100_sync_note_path_check_commitments.sql replaces this same
-- constraint to add that folder, and this file sorts after it, so this check has to be the union
-- whichever stream merges first — a check without `commitments` would refuse every push that
-- carries one. On a branch without the commitment model the column accepts a folder no device writes
-- yet, which refuses nothing a device sends.
alter table public.sync_notes
  drop constraint sync_notes_path_check,
  add constraint sync_notes_path_check check (
    (
      path ~ '^(tasks|approvals|archive|courses|issues|info|commitments)/[A-Za-z0-9._ /-]+\.md$'
      and char_length(regexp_replace(path, '^[a-z]+/', '')) between 4 and 303
    )
    or path in ('config/campus.yaml', 'config/events.yaml', 'config/ingest.yaml',
                'config/planning.yaml', 'config/runners.yaml', 'config/week_template.yaml')
  );

-- **The conditional push** (re-review R2-I1). `sync-push` upserts note rows unconditionally, so of two
-- pushes of one settings file the later would win and the earlier pusher's next pull would overwrite
-- its own change with no line. A settings row is therefore stored only if the account's text is still
-- the one the device last synced: `p_base` is the SHA-256 (hex) of that text, hashed as it travels,
-- and an empty base stores the row only where the account holds none for the path. One statement
-- either way; `true` when a row was written. Like every C2/C3′ writer it runs as the caller (the
-- service role), not SECURITY DEFINER, and the client roles cannot execute it.
create or replace function public.save_config_row(
  p_account uuid,
  p_path    text,
  p_body    text,
  p_device  text,
  p_base    text
)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  n integer;
begin
  if p_path not in ('config/campus.yaml', 'config/events.yaml', 'config/ingest.yaml',
                    'config/planning.yaml', 'config/runners.yaml', 'config/week_template.yaml') then
    raise exception 'save_config_row: % is not a shared settings path', p_path using errcode = '22023';
  end if;
  if coalesce(p_base, '') = '' then
    insert into public.sync_notes (account_id, path, device, deleted, body)
    values (p_account, p_path, p_device, false, p_body)
    on conflict (account_id, path) do nothing;
  else
    update public.sync_notes
       set body = p_body, device = p_device, deleted = false
     where account_id = p_account
       and path = p_path
       and not deleted
       and encode(sha256(convert_to(body, 'UTF8')), 'hex') = p_base;
  end if;
  get diagnostics n = row_count;
  return n = 1;
end;
$$;

revoke execute on function public.save_config_row(uuid, text, text, text, text) from public, anon, authenticated;
grant execute on function public.save_config_row(uuid, text, text, text, text) to service_role;
```

- [ ] **Step 4: Run the Deno gate**

Run, each with absolute paths (PATH refreshed):
`deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\`
Expected: PASS, the whole suite — the new tests, `judge_db_test.ts`'s account-scoping scan (it now finds
`save_config_row` among the functions taking `p_account` and sees `sync_db.ts`'s call pass one),
`migrations_sync_test.ts` unchanged (it reads only `20260912…` files), and every other migration guard
(the new function is revoked; it creates no table and no view).

`deno check --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json` and
`deno lint --config …\deno.json`, each over the ten files this task touches that end in `.ts`
(`functions\sync-push\{index,handler,handler_test}.ts`, `functions\sync-pull\{index,handler,handler_test}.ts`,
`functions\_shared\{sync_rows,sync_rows_test,sync_db}.ts`, `migrations\migrations_test.ts`).
Expected: both clean.

- [ ] **Step 5: The whole suite**

Global Constraint 19 — PASS at 0 warnings (no Rust changed; `engine/tests/sync_contract.rs::
is_note_path_and_the_servers_regex_agree` still finds the note folders in `sync_rows.ts`).

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-3.txt`:

```
feat: the account stores shared settings by compare-and-set (two desktops, D18)

A migration widens sync_notes' path check to the six shared settings
paths (keeping the commitment model's folder) and adds save_config_row:
one statement that stores a settings row only if the account's text still
hashes to the base the device last synced, or, with an empty base, only
where the account holds none. sync-push routes settings rows to it, never
through saveNotes' upsert, and answers config_refused; sync-pull gains a
settings view for the second computer's wizard. Function pin 28 -> 29.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add cloud/supabase/migrations/20260926000200_shared_settings.sql cloud/supabase/migrations/migrations_test.ts cloud/supabase/functions/_shared/sync_rows.ts cloud/supabase/functions/_shared/sync_rows_test.ts cloud/supabase/functions/_shared/sync_db.ts cloud/supabase/functions/sync-push/handler.ts cloud/supabase/functions/sync-push/handler_test.ts cloud/supabase/functions/sync-push/index.ts cloud/supabase/functions/sync-pull/handler.ts cloud/supabase/functions/sync-pull/handler_test.ts cloud/supabase/functions/sync-pull/index.ts`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-3.txt`

---

### Task 4: The push — six rows with their base, the guard, the refusals; `sync` runs the migration (spec §4.8 D17, D18)

**Files:**
- Modify: `engine/src/sync.rs` — the `SHARED_CONFIG` re-export (after the `use` block, `:36-37`);
  `Cursor` (`:213-237`) gains `provisional_config` and `provisional_since` beside Plan 1's
  `verdicts_after`; `PushBatch` (`:532-538`) gains `settings`; `build_push` (`:570-755`) gains the
  settings loop ahead of the notes, the notes loop's `PAGE` check counts both, and the tombstone pass
  skips a settings path; `push` (`:767-772`) becomes a wrapper over the new `push_reply`;
  `run_lines_with_client` runs the migration first and restores a refused path's base.
- Modify: `engine/tests/sync_contract.rs` — two `PushBatch { … }` literals (`:315`, `:336`) name the new
  field.
- Create: `engine/tests/sync_settings.rs` — the loopback harness and three tests.

**Interfaces:**
- Consumes: `config::{SHARED_CONFIG, is_shared_config, computer_only_value, move_device_keys}`;
  Task 3's reply shape.
- Produces: `sync::{SHARED_CONFIG, PushBatch::settings, PushReply, push_reply}`,
  `Cursor::{provisional_config, provisional_since}` (read here — a marked path is never sent — and set in
  Task 6).

**Amendment to Plan 1's interfaces:** `sync::PushBatch` gains a public field, so a struct literal of it
must name `settings` (only `sync_contract.rs`'s two do). `Cursor`'s two fields sit beside Plan 1's
`verdicts_after`, as its table said.

Rulings applied here: R-TD2-1 (a settings path is its own predicate; `is_note_path` is unchanged),
R-TD2-3 (the migration at the top of `run_lines_with_client`), R-TD2-4 (the guard and its two lines),
R-TD2-5 (the rows' place on the wire and in the page count), R-TD2-6 (the reply). The refused path's
line is the spec's: `sync: config/<file> — another computer changed it first; yours waits for the next
pull`.

- [ ] **Step 1: Write the failing tests**

In `engine/tests/sync_contract.rs`, replace

```rust
    let mut server = loopback(vec![(200, r#"{"records":2,"notes":1,"bytes_used":10,"bytes_ceiling":20}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    assert_eq!(sync::push(&client, &batch).expect("a 200"), (2, 1));
    let sent = server.requests();
```

with

```rust
    let mut server = loopback(vec![(200, r#"{"records":2,"notes":1,"bytes_used":10,"bytes_ceiling":20}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], settings: vec![], warnings: vec![] };
    assert_eq!(sync::push(&client, &batch).expect("a 200"), (2, 1));
    let sent = server.requests();
```

In `engine/tests/sync_contract.rs`, replace

```rust
    let mut server = loopback(vec![(403, r#"{"error":"this account's copy is at its size limit"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], warnings: vec![] };
    match sync::push(&client, &batch) {
        Err(knowlu_engine::cloudmodel::CloudError::Status { code, .. }) => assert_eq!(code, 403),
```

with

```rust
    let mut server = loopback(vec![(403, r#"{"error":"this account's copy is at its size limit"}"#.to_string())]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let batch = sync::PushBatch { device: "0123456789abcdef".into(), records: vec![], notes: vec![], settings: vec![], warnings: vec![] };
    match sync::push(&client, &batch) {
        Err(knowlu_engine::cloudmodel::CloudError::Status { code, .. }) => assert_eq!(code, 403),
```

Create `engine/tests/sync_settings.rs`:

```rust
//! Two-desktop design D18/D19 (§4.8, §6.1's shared-config bullets): the six shared settings files
//! through C3′'s sync — the push with its base and guard, the three-way pull, the conflict folder,
//! `apply_approved_mappings`, and the provisional mark's three endings. Loopback servers only
//! (`127.0.0.1:0`, served from a second thread and joined); no real vault, no network, no Credential
//! Manager. The harness is copied from `sync_contract.rs`, as that file copied `cloud_contract.rs`'s.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use knowlu_engine::cloudmodel::{CloudClient, CloudConfig};
use knowlu_engine::journal::Journal;
use knowlu_engine::sync::{self, Cursor};

struct Loopback {
    base: String,
    handle: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Loopback {
    fn requests(&mut self) -> Vec<String> {
        self.handle.take().expect("joined once").join().expect("the listener thread did not panic")
    }
}

fn read_request(stream: &std::net::TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
    let (mut head, mut length) = (String::new(), 0usize);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = rest.trim().parse().unwrap_or(0);
        }
        let blank = line == "\r\n" || line == "\n";
        head.push_str(&line);
        if blank {
            break;
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        let _ = reader.read_exact(&mut body);
    }
    format!("{head}{}", String::from_utf8_lossy(&body))
}

fn loopback(replies: Vec<(u16, String)>) -> Loopback {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
    let port = listener.local_addr().expect("an address").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (code, body) in replies {
            let Ok((mut stream, _)) = listener.accept() else { break };
            seen.push(read_request(&stream));
            let response = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    Loopback { base: format!("http://127.0.0.1:{port}/functions/v1"), handle: Some(handle) }
}

fn cfg(base: &str) -> CloudConfig {
    CloudConfig {
        api_base: base.to_string(),
        anon_key: "anon-not-a-secret".to_string(),
        session_credential_target: "knowlu/test-profile/session".to_string(),
        account_id: "acct-1".to_string(),
    }
}

/// A split vault: the six shared files as the wizard writes them, and `config/device.yaml`.
fn vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-td2s-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for folder in ["config", "state", "tasks", "approvals", "archive"] {
        std::fs::create_dir_all(dir.join(folder)).expect("mkdir");
    }
    put(&dir, "config/ingest.yaml", INGEST);
    put(&dir, "config/runners.yaml", "runners:\n  - name: local\n    times: ['12:00', '18:00']\n    tz: 'America/Chicago'\n    grace_minutes: 20\n");
    put(&dir, "config/planning.yaml", "daily_effort_budget: 4.0\n");
    put(&dir, "config/week_template.yaml", "monday: []\n");
    put(&dir, "config/events.yaml", "sources:\n  - name: campus\n    type: ics\n    url: \"https://calendar.example.invalid/campus.ics\"\n    enabled: true\n");
    put(&dir, "config/campus.yaml", "unitid: '100751'\nname: 'Example University'\n");
    put(&dir, "config/device.yaml", "coursework:\n  zybooks:\n    enabled: true\n    credential_target: 'knowlu/p/zybooks'\n  vhl: {}\nics_url: ''\ncalendars: []\nrunner:\n  device: 'EXAMPLE-PC'\n  scheduler: app\n");
    dir
}

const INGEST: &str = "timezone: 'America/Chicago'\ncourse_map: {}\ncalendars:\n  - name: personal\n    ics_url: 'cloud:personal'\ncoursework:\n  zybooks:\n    ignore: []\n    courses: {}\n";

fn put(dir: &Path, rel: &str, text: &str) {
    knowlu_engine::pystr::write_text(&dir.join(rel), text).expect("write");
}

fn read(dir: &Path, rel: &str) -> String {
    knowlu_engine::pystr::read_text(&dir.join(rel)).expect("read")
}

fn hash(text: &str) -> String {
    sync::sha256_hex(text.as_bytes())
}

fn pull_reply(settings: &[(&str, &str)], cursor: i64) -> (u16, String) {
    let notes: Vec<serde_json::Value> = settings
        .iter()
        .enumerate()
        .map(|(i, (path, body))| serde_json::json!({ "rev": i + 1, "device": "fedcba9876543210", "path": path, "deleted": false, "body": body }))
        .collect();
    let reply = serde_json::json!({ "records": [], "notes": notes, "record_cursor": cursor, "note_cursor": cursor, "more": false });
    (200, knowlu_engine::ledger::dumps_value(&reply))
}

fn push_reply(refused: &[&str]) -> (u16, String) {
    (200, knowlu_engine::ledger::dumps_value(&serde_json::json!({ "records": 0, "notes": 0, "config": 0, "config_refused": refused })))
}

/// The settings rows one `/sync-push` request carried, parsed back out of the request text.
fn pushed_settings(request: &str) -> Vec<serde_json::Value> {
    let body = request.split("\r\n\r\n").nth(1).expect("a request body");
    let sent: serde_json::Value = serde_json::from_str(body).expect("JSON");
    sent["notes"].as_array().expect("notes").iter().filter(|r| r["path"].as_str().is_some_and(|p| p.starts_with("config/"))).cloned().collect()
}

fn run(dir: &Path, replies: Vec<(u16, String)>) -> (Vec<String>, Vec<String>) {
    let mut server = loopback(replies);
    let cloud = cfg(&server.base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let (lines, _) = sync::run_lines_with_client(dir, sync::Direction::Both, "cli", None, &client, &cloud, Vec::new(), sync::Totals::default());
    (lines, server.requests())
}

#[test]
fn the_shared_settings_list_is_the_servers_and_never_a_note_path() {
    // One list in three places — here, `_shared/sync_rows.ts` and `sync_notes`' path check — pinned
    // from this side, the way `is_note_path_and_the_servers_regex_agree` pins the note rule.
    let ts = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("cloud").join("supabase").join("functions").join("_shared").join("sync_rows.ts"),
    )
    .expect("sync_rows.ts");
    let listed: String = sync::SHARED_CONFIG.iter().map(|p| format!("  \"{p}\",\n")).collect();
    assert!(ts.contains(&format!("export const SHARED_CONFIG = [\n{listed}];")), "the server's list is the engine's, in order");
    let vault = std::env::temp_dir();
    for rel in sync::SHARED_CONFIG {
        assert!(!sync::is_note_path(&vault, rel), "{rel} is never a note path: no record or tombstone may name it");
    }
    assert!(!knowlu_engine::config::is_shared_config(knowlu_engine::config::DEVICE_FILE));
    assert!(!knowlu_engine::config::is_shared_config("config/cloud.yaml"));
}

#[test]
fn the_six_shared_files_go_up_whole_with_their_base_and_device_yaml_never_does() {
    let dir = vault("push");
    let mut journal = Journal::new(&dir);
    let (batch, next) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(batch.warnings.is_empty(), "{:?}", batch.warnings);
    let paths: Vec<&str> = batch.settings.iter().map(|r| r["path"].as_str().unwrap()).collect();
    assert_eq!(paths, sync::SHARED_CONFIG.to_vec(), "every shared file, and nothing from device.yaml or cloud.yaml");
    for row in &batch.settings {
        assert_eq!(row["base"], "", "nothing synced yet: an empty base");
        assert_eq!(row["body"].as_str().unwrap(), read(&dir, row["path"].as_str().unwrap()), "the whole text, as read");
    }
    assert!(batch.notes.iter().all(|n| !n["path"].as_str().unwrap().starts_with("config/")), "settings are not notes");
    let (again, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert!(again.settings.is_empty(), "an unchanged file is not sent twice");
    put(&dir, "config/planning.yaml", "daily_effort_budget: 5.0\n");
    let (edited, _) = sync::build_push(&dir, &next, "acct-1", &mut journal);
    assert_eq!(edited.settings.len(), 1);
    assert_eq!(edited.settings[0]["base"], hash("daily_effort_budget: 4.0\n"), "the base is the text last synced");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_capability_url_ever_travels_and_the_migration_is_what_lets_the_file_go() {
    let dir = vault("guard");
    // As `restore_capability_url` wrote it before this stream, and a hand-added webcal feed.
    put(
        &dir,
        "config/ingest.yaml",
        "timezone: 'America/Chicago'\ncalendars:\n  - name: personal\n    ics_url: 'https://calendar.example.invalid/private-abc/basic.ics'\n  \
         - name: club\n    ics_url: 'webcal://club.example.invalid/feed.ics'\n",
    );
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &Cursor::default(), "acct-1", &mut journal);
    assert!(!batch.settings.iter().any(|r| r["path"] == "config/ingest.yaml"), "the file stays local");
    assert!(
        batch.warnings.contains(&"sync: config/ingest.yaml holds a computer-only value (calendars.personal); it stays on this computer until it is moved".to_string()),
        "{:?}", batch.warnings
    );
    // A sync runs the migration first, and then the file travels without either address.
    let (lines, sent) = run(&dir, vec![pull_reply(&[], 1), push_reply(&[])]);
    assert!(lines.iter().any(|l| l == "config: moved 2 computer-only value(s) into config/device.yaml"), "{lines:?}");
    let rows = pushed_settings(&sent[1]);
    assert!(rows.iter().any(|r| r["path"] == "config/ingest.yaml"), "{rows:?}");
    for row in &rows {
        let body = row["body"].as_str().unwrap();
        let path = row["path"].as_str().unwrap();
        let allowed = path == "config/events.yaml";
        for scheme in ["https://", "http://", "webcal://"] {
            assert!(allowed || !body.contains(scheme), "{path} carried an address: {body}");
        }
    }
    assert!(!sent[1].contains("private-abc") && !sent[1].contains("club.example.invalid"), "neither address left this computer");
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_settings --test sync_contract`
Expected: FAIL to compile — `cannot find value SHARED_CONFIG in module sync`, `no field settings on type
PushBatch` (in both files).

- [ ] **Step 3: The rows, the guard, the refusals**

In `engine/src/sync.rs`, replace

```rust
use crate::journal::Journal;
use crate::ledger::Record;

/// The largest journal record this device will send, in bytes, matching
```

with

```rust
use crate::journal::Journal;
use crate::ledger::Record;

/// Two-desktop design D18: the six shared settings files that travel whole, one `sync_notes` row per
/// path — `config::SHARED_CONFIG`, named here too because the transport is this module's.
pub use crate::config::SHARED_CONFIG;

/// The largest journal record this device will send, in bytes, matching
```

In `engine/src/sync.rs`, replace

```rust
    /// cursor pulls from the start again, which restores a line the student deleted: §2.8's named cost.
    #[serde(default)] pub verdicts_after: String,
}
```

with

```rust
    /// cursor pulls from the start again, which restores a line the student deleted: §2.8's named cost.
    #[serde(default)] pub verdicts_after: String,
    /// Two-desktop design D18/D19 (§4.8, re-review R2-I2, R3-I1): the shared settings files this
    /// computer holds **provisionally** — written on a computer born into an account that already
    /// held notes, before the account's settings reached it. `build_push` never sends one while it is
    /// listed. The mark is state, never inferred from bytes, and it ends only by the account's
    /// settings arriving by pull ([`apply_settings`]), a settings edit in the app
    /// ([`settings_edited`]), or seven days with none ([`publish_if_due`]). Sorted, `SHARED_CONFIG`
    /// paths only.
    #[serde(default)] pub provisional_config: Vec<String>,
    /// This computer's own clock (a journal `ts`) when the mark was set; empty with no mark.
    #[serde(default)] pub provisional_since: String,
}
```

In `engine/src/sync.rs`, replace

```rust
    pub records: Vec<Value>,
    pub notes: Vec<Value>,
    pub warnings: Vec<String>,
}
```

with

```rust
    pub records: Vec<Value>,
    pub notes: Vec<Value>,
    /// Two-desktop design D18: the shared settings rows, `{"path", "body", "base"}` — sent in the
    /// wire's `notes` array beside the notes, and routed by the server to `save_config_row`.
    pub settings: Vec<Value>,
    pub warnings: Vec<String>,
}
```

In `engine/src/sync.rs`, replace

```rust
    }

    // I1: tombstone against the COMPLETE listing, never against which paths this loop happened to
    // *visit* — a `PAGE` or budget `break` below must not read as "everything after this point in
```

with

```rust
    }

    // Two-desktop design D18 (§4.8): the six shared settings files, whole, one row each, ahead of the
    // notes so a page that breaks on the notes never starves them. Each row carries `base`, the
    // SHA-256 of the text this computer last synced for the path (empty for none) — the server stores
    // it only if the account's text still hashes to that (`save_config_row`), so no change is ever
    // overwritten in silence. Never a tombstone: a missing file is simply not sent.
    for rel in SHARED_CONFIG {
        if batch.notes.len() + batch.settings.len() >= PAGE { break; }
        let path = vault.join(rel);
        if !path.is_file() { continue; }
        // D19: a provisional file waits for the account's settings, an edit in the app, or seven days.
        if cursor.provisional_config.iter().any(|p| p == rel) { continue; }
        let Ok(text) = crate::pystr::read_text(&path) else {
            batch.warnings.push(format!("sync: {rel} could not be read; it stays on this machine"));
            continue;
        };
        if text.is_empty() || text.contains('\u{0}') || text.len() > MAX_NOTE_BYTES {
            batch.warnings.push(format!("sync: {rel} is empty, too large or holds a character the account cannot store; it stays on this machine"));
            continue;
        }
        let hash = sha256_hex(text.as_bytes());
        let base = cursor.notes.get(rel).cloned().unwrap_or_default();
        if hash == base { continue; }
        // D17's wedge guard: a device value or a capability URL never reaches the account.
        match crate::config::computer_only_value(rel, &text) {
            Err(_) => {
                batch.warnings.push(format!("sync: {rel} is not readable YAML; it stays on this computer until it is fixed"));
                continue;
            }
            Ok(Some(key)) => {
                batch.warnings.push(format!("sync: {rel} holds a computer-only value ({key}); it stays on this computer until it is moved"));
                continue;
            }
            Ok(None) => {}
        }
        let row = serde_json::json!({ "path": rel, "body": text, "base": base });
        let row_len = crate::ledger::dumps_value(&row).len();
        if budget_used + row_len > PUSH_BUDGET_BYTES { break; }
        budget_used += row_len;
        next.notes.insert(rel.to_string(), hash);
        batch.settings.push(row);
    }

    // I1: tombstone against the COMPLETE listing, never against which paths this loop happened to
    // *visit* — a `PAGE` or budget `break` below must not read as "everything after this point in
```

In `engine/src/sync.rs`, replace

```rust
    // `cursor.notes` against a full `BTreeSet` of it, after the loop, is what makes a break safe.
    for rel in &on_disk {
        if batch.notes.len() >= PAGE { break; }
        if !is_note_path(vault, rel) {
            batch.warnings.push(format!(
```

with

```rust
    // `cursor.notes` against a full `BTreeSet` of it, after the loop, is what makes a break safe.
    for rel in &on_disk {
        if batch.notes.len() + batch.settings.len() >= PAGE { break; }
        if !is_note_path(vault, rel) {
            batch.warnings.push(format!(
```

In `engine/src/sync.rs`, replace

```rust
    let on_disk_set: std::collections::BTreeSet<&String> = on_disk.iter().collect();
    for rel in cursor.notes.keys() {
        if batch.notes.len() >= PAGE { break; }
        // I1: still on disk (whether or not this build's first loop got as far as visiting it) — not
        // deleted, so never a tombstone.
```

with

```rust
    let on_disk_set: std::collections::BTreeSet<&String> = on_disk.iter().collect();
    for rel in cursor.notes.keys() {
        // D18: a shared settings file's base is not a note's hash, and a settings file is never
        // tombstoned — a missing one comes back from the account (`restore_missing_settings`).
        if crate::config::is_shared_config(rel) { continue; }
        if batch.notes.len() + batch.settings.len() >= PAGE { break; }
        // I1: still on disk (whether or not this build's first loop got as far as visiting it) — not
        // deleted, so never a tombstone.
```

In `engine/src/sync.rs`, replace

```rust
/// journal and telling the student their copy is full.
pub fn push(client: &crate::cloudmodel::CloudClient, batch: &PushBatch) -> Result<(usize, usize), crate::cloudmodel::CloudError> {
    let body = serde_json::json!({ "device": batch.device, "records": batch.records, "notes": batch.notes });
    let reply = client.post("/sync-push", &body)?;
    let count = |key: &str| reply.get(key).and_then(Value::as_u64).unwrap_or(0) as usize;
    Ok((count("records"), count("notes")))
}
```

with

```rust
/// journal and telling the student their copy is full.
pub fn push(client: &crate::cloudmodel::CloudClient, batch: &PushBatch) -> Result<(usize, usize), crate::cloudmodel::CloudError> {
    push_reply(client, batch).map(|r| (r.records, r.notes))
}

/// What `/sync-push` answered: the two counts, and (two-desktop design D18) the settings paths whose
/// conditional store the server refused because the account's text had moved past this device's base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushReply {
    pub records: usize,
    pub notes: usize,
    pub config_refused: Vec<String>,
}

/// [`push`], with the reply's `config_refused`. The settings rows travel in the wire's `notes` array
/// beside the notes; the server tells them apart by path.
pub fn push_reply(client: &crate::cloudmodel::CloudClient, batch: &PushBatch) -> Result<PushReply, crate::cloudmodel::CloudError> {
    let notes: Vec<Value> = batch.notes.iter().chain(batch.settings.iter()).cloned().collect();
    let body = serde_json::json!({ "device": batch.device, "records": batch.records, "notes": notes });
    let reply = client.post("/sync-push", &body)?;
    let count = |key: &str| reply.get(key).and_then(Value::as_u64).unwrap_or(0) as usize;
    let config_refused = reply
        .get("config_refused")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).filter(|p| crate::config::is_shared_config(p)).map(str::to_string).collect())
        .unwrap_or_default();
    Ok(PushReply { records: count("records"), notes: count("notes"), config_refused })
}
```

In `engine/src/sync.rs`, replace

```rust
    mut totals: Totals,
) -> (Vec<String>, Totals) {
    let mut cursor = load_cursor(vault);
    let mut journal = Journal::new(vault);
```

with

```rust
    mut totals: Totals,
) -> (Vec<String>, Totals) {
    // Two-desktop design D17: device keys leave the shared files before either half runs — every sync
    // until done, a no-op after.
    lines.extend(crate::config::move_device_keys(vault));
    let mut cursor = load_cursor(vault);
    let mut journal = Journal::new(vault);
```

In `engine/src/sync.rs`, replace

```rust
            lines.push("sync: the push waits for the network".to_string());
        } else {
            let (batch, next) = build_push(vault, &cursor, &cfg.account_id, &mut journal);
            lines.extend(batch.warnings.iter().cloned());
            match push(client, &batch) {
                Ok((records, notes)) => {
                    totals.pushed_records = records;
                    totals.pushed_notes = notes;
                    lines.push(format!("sync: {records} record(s) and {notes} note(s) up"));
                    if let Err(e) = save_cursor(vault, &next) {
                        totals.errors.push(e.label());
```

with

```rust
            lines.push("sync: the push waits for the network".to_string());
        } else {
            let (batch, mut next) = build_push(vault, &cursor, &cfg.account_id, &mut journal);
            lines.extend(batch.warnings.iter().cloned());
            match push_reply(client, &batch) {
                Ok(PushReply { records, notes, config_refused }) => {
                    totals.pushed_records = records;
                    totals.pushed_notes = notes;
                    lines.push(format!("sync: {records} record(s) and {notes} note(s) up"));
                    // D18: a refused settings row keeps its text and its base; the pull that brings
                    // the other computer's text takes the conflict branch.
                    for rel in config_refused {
                        match cursor.notes.get(&rel) {
                            Some(base) => next.notes.insert(rel.clone(), base.clone()),
                            None => next.notes.remove(&rel),
                        };
                        lines.push(format!("sync: {rel} — another computer changed it first; yours waits for the next pull"));
                    }
                    if let Err(e) = save_cursor(vault, &next) {
                        totals.errors.push(e.label());
```

- [ ] **Step 4: Run the tests**

Run the command of Step 2. Expected: PASS — the three new tests, and every `sync_contract.rs` test
unchanged: `vault-s1`'s three config files now travel as settings rows, which are not notes, so
`a_first_push_carries_every_record_and_every_note_once` still counts only notes, and a scripted reply
without `config_refused` refuses nothing. Then `--test sync_replay --test entitlement_gate`: PASS.

- [ ] **Step 5: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-4.txt`:

```
feat: sync pushes the six shared settings files with their base (two desktops, D18)

build_push sends each shared config file whose text changed since it was
last synced, whole, with the SHA-256 of that last text as its base; never
a provisional file, never config/device.yaml, and never a file that carries
a device key or a capability URL (named, and kept on this computer). A path
the server refuses keeps its base, and the next pull settles it. sync runs
the device-key migration first, on every run until done.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/tests/sync_contract.rs engine/tests/sync_settings.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-4.txt`

---

### Task 5: The pull — the three-way check, the conflict folder, `apply_approved_mappings`, the settings read (spec §4.8 D18, Q13)

**Files:**
- Modify: `engine/src/coursework.rs` — `write_mapping` (`:1359-1433`) becomes a wrapper over the new
  `insert_mapping` (an explicit label, and a missing `coursework:`/`coursework.<source>:` block created
  with that source's shared defaults, R-TD2-20); the two `*_SHARED_DEFAULTS` constants; a new
  `apply_approved_mappings` after it.
- Modify: `engine/src/sync.rs` — after `pull` (`:890-894`): `CONFLICTS_DIR`, `keep_conflict`,
  `apply_settings`, `AccountSettings`, `account_settings`, `restore_missing_settings`; `apply`'s step 6
  (`for note in ordered_notes`, `:2655`) skips a settings row; `run_lines_with_client`'s pull calls
  `apply_settings` after `apply` and `restore_missing_settings` after the cursors.
- Modify: `engine/tests/device_config.rs` (one test), `engine/tests/sync_settings.rs` (four tests).

**Interfaces:**
- Consumes: Task 4's `Cursor` fields, `PushBatch`; `approvals::sorted_md`, `models::split_frontmatter`,
  `write::to_literal`.
- Produces: `coursework::{insert_mapping, ZYBOOKS_SHARED_DEFAULTS, VHL_SHARED_DEFAULTS,
  apply_approved_mappings}`, `sync::{CONFLICTS_DIR, apply_settings, AccountSettings, account_settings,
  restore_missing_settings}`.

**The three-way check** (`apply_settings`, R-TD2-7), per settings row a page carries (the latest for a
path wins), with `ours` the hash of the local file, `theirs` the row's, `base` the cursor's:
`ours == theirs` → record the base, write nothing; a provisional mark, a missing file or no base → take
the account's text (keeping ours in the conflict folder when it exists and differs — a missing file has
nothing to keep); `ours == base` → take it; `theirs == base` → nothing (the push sends ours); otherwise →
take it and keep ours. "This computer's last push was refused" needs no state of its own: the refused
text is still ≠ its base, and the other computer's row is ≠ it too, so the pull that brings that row is
the both-changed branch (§6.1's refused-push test proves it end to end). A provisional mark ends for every
path once any settings row arrives (D18 (a)). After any replacement, `apply_approved_mappings`.

- [ ] **Step 1: Write the failing tests**

Append to `engine/tests/device_config.rs` (after one blank line):

```rust
#[test]
fn a_mapping_for_a_source_the_shared_file_lacks_creates_its_block_with_the_shared_defaults() {
    let dir = vault("insert-mapping");
    put(&dir, "config/ingest.yaml", "timezone: X\ncoursework:\n  zybooks:\n    courses: {}\n");
    assert_eq!(knowlu_engine::coursework::insert_mapping(&dir, "vhl", "2102121", "gn-103", "GN 103"), Ok(true));
    assert_eq!(
        read(&dir, "config/ingest.yaml"),
        "timezone: X\ncoursework:\n  zybooks:\n    courses: {}\n  vhl:\n    importance: 3\n    sections:\n      \
         \"2102121\":\n        course: \"gn-103\"\n        label: \"GN 103\"\n"
    );
    assert_eq!(knowlu_engine::coursework::insert_mapping(&dir, "vhl", "2102121", "gn-999", "X"), Ok(false), "a key the file holds is left alone");
    let dir2 = vault("insert-mapping-no-coursework");
    put(&dir2, "config/ingest.yaml", "timezone: X\n");
    assert_eq!(knowlu_engine::coursework::insert_mapping(&dir2, "zybooks", "UACS100Fall2026", "cs-100", "CS 100"), Ok(true));
    let (cfg, warnings) = knowlu_engine::coursework::load_coursework_config(&dir2).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let zy = cfg["coursework"]["zybooks"].as_mapping().unwrap();
    assert!(zy.get("categories").is_some() && zy.get("effort").is_some(), "the shared defaults came with the block");
    assert_eq!(cfg["coursework"]["zybooks"]["courses"]["UACS100Fall2026"]["label"].as_str(), Some("CS 100"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}
```

Append to `engine/tests/sync_settings.rs` (after one blank line):

```rust
/// One pulled page carrying `settings` as `(path, text)` rows from another computer.
fn page(settings: &[(&str, &str)]) -> sync::Pulled {
    sync::Pulled {
        notes: settings
            .iter()
            .map(|(path, text)| sync::PulledNote { device: "fedcba9876543210".into(), path: path.to_string(), text: Some(text.to_string()) })
            .collect(),
        record_cursor: 1,
        note_cursor: 1,
        ..Default::default()
    }
}

const THEIRS: &str = "daily_effort_budget: 6.0\n";

fn conflicts(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join(sync::CONFLICTS_DIR))
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn the_three_way_pull_takes_keeps_or_keeps_the_accounts_text_and_the_local_one_beside_it() {
    let rel = "config/planning.yaml";
    let mine = "daily_effort_budget: 4.0\n";
    // (1) Unchanged here since the last sync: the account's text is written and becomes the base.
    let dir = vault("pull-take");
    let mut cursor = Cursor::default();
    cursor.notes.insert(rel.into(), hash(mine));
    let lines = sync::apply_settings(&dir, &page(&[(rel, THEIRS)]), &mut cursor);
    assert_eq!(read(&dir, rel), THEIRS);
    assert_eq!(cursor.notes[rel], hash(THEIRS));
    assert_eq!(lines, vec!["sync: config/planning.yaml updated from your account".to_string()]);
    assert!(conflicts(&dir).is_empty());
    // (2) Changed here, and the account still holds the base: nothing to take — the push sends ours.
    let dir2 = vault("pull-keep");
    let mut cursor = Cursor::default();
    cursor.notes.insert(rel.into(), hash(THEIRS));
    assert!(sync::apply_settings(&dir2, &page(&[(rel, THEIRS)]), &mut cursor).is_empty());
    assert_eq!(read(&dir2, rel), mine, "our change stands");
    assert_eq!(cursor.notes[rel], hash(THEIRS), "and the base with it, so the push sends ours");
    // (3) Both changed: the account's text wins and ours goes to the conflict folder, named.
    let dir3 = vault("pull-both");
    let mut cursor = Cursor::default();
    cursor.notes.insert(rel.into(), hash("daily_effort_budget: 1.0\n"));
    let lines = sync::apply_settings(&dir3, &page(&[(rel, THEIRS)]), &mut cursor);
    assert_eq!(read(&dir3, rel), THEIRS);
    let kept = conflicts(&dir3);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0].starts_with("planning-") && kept[0].ends_with(".yaml"), "{kept:?}");
    assert_eq!(read(&dir3, &format!("{}/{}", sync::CONFLICTS_DIR, kept[0])), mine, "our text, whole");
    assert_eq!(lines[0], format!("sync: config/planning.yaml — another computer's settings won; yours are in state/config-conflicts/{}", kept[0]));
    // (4) No base yet (the first sync after the upgrade): the account's text wins (Q13) and ours is
    //     kept — unless the two are byte-identical, which needs no copy.
    let dir4 = vault("pull-nobase");
    let mut cursor = Cursor::default();
    sync::apply_settings(&dir4, &page(&[(rel, THEIRS), ("config/runners.yaml", &read(&dir4, "config/runners.yaml"))]), &mut cursor);
    assert_eq!(read(&dir4, rel), THEIRS);
    assert_eq!(conflicts(&dir4).len(), 1, "only the file that differed is kept");
    assert_eq!(cursor.notes["config/runners.yaml"], hash(&read(&dir4, "config/runners.yaml")), "an identical text just becomes the base");
    // (5) Missing here: written.
    let dir5 = vault("pull-missing");
    std::fs::remove_file(dir5.join(rel)).unwrap();
    sync::apply_settings(&dir5, &page(&[(rel, THEIRS)]), &mut Cursor::default());
    assert_eq!(read(&dir5, rel), THEIRS);
    assert!(conflicts(&dir5).is_empty());
    for d in [dir, dir2, dir3, dir4, dir5] {
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn a_refused_push_keeps_its_text_and_base_and_becomes_the_conflict_at_the_next_pull() {
    let dir = vault("refused");
    let rel = "config/planning.yaml";
    // Both computers synced the same text once.
    let mut cursor = Cursor::default();
    for p in sync::SHARED_CONFIG {
        cursor.notes.insert(p.to_string(), hash(&read(&dir, p)));
    }
    cursor.pulled_to_end = true;
    sync::save_cursor(&dir, &cursor).unwrap();
    let base = cursor.notes[rel].clone();
    put(&dir, rel, "daily_effort_budget: 5.0\n");
    let (lines, sent) = run(&dir, vec![pull_reply(&[], 1), push_reply(&[rel])]);
    assert_eq!(pushed_settings(&sent[1]).iter().map(|r| (r["path"].as_str().unwrap().to_string(), r["base"].as_str().unwrap().to_string())).collect::<Vec<_>>(),
        vec![(rel.to_string(), base.clone())], "only the changed file, from its base");
    assert!(lines.contains(&"sync: config/planning.yaml — another computer changed it first; yours waits for the next pull".to_string()), "{lines:?}");
    assert_eq!(sync::load_cursor(&dir).notes[rel], base, "text and base are both kept");
    assert_eq!(read(&dir, rel), "daily_effort_budget: 5.0\n");
    // The pull that brings the other computer's text takes the conflict branch: file and line.
    let (lines, _) = run(&dir, vec![pull_reply(&[(rel, THEIRS)], 2), push_reply(&[])]);
    assert_eq!(read(&dir, rel), THEIRS);
    let kept = conflicts(&dir);
    assert_eq!(kept.len(), 1);
    assert!(lines.iter().any(|l| l.contains("another computer's settings won; yours are in state/config-conflicts/")), "{lines:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A settled `coursework-map` card, as `apply_map_cards` leaves one in `archive/`.
fn map_card(dir: &Path, folder: &str, key: &str, course: &str, status: &str) {
    put(
        dir,
        &format!("{folder}/map-zybooks-{}.md", key.to_lowercase()),
        &format!(
            "---\ntype: approval\nkind: coursework-map\ntitle: \"Map {key}\"\nstatus: {status}\nproposed_at: 2026-09-01\n\
             first_proposed_at: 2026-09-01\nexpires: 2026-10-01\nsnooze_until: null\ncreated_by: coursework\nsource: \"zybooks\"\n\
             map_key: \"{key}\"\ncourse: \"{course}\"\n---\n\nbody\n"
        ),
    );
}

#[test]
fn apply_approved_mappings_puts_back_a_card_made_mapping_and_never_changes_a_key_the_file_holds() {
    let dir = vault("mappings");
    map_card(&dir, "archive", "UACS100Fall2026", "cs-100", "executed");
    map_card(&dir, "archive", "UAMATH120Fall2026", "math-120", "executed");
    map_card(&dir, "archive", "UAOLD2020", "old-100", "executed");
    map_card(&dir, "approvals", "UAGN103Fall2026", "gn-103", "approved");
    map_card(&dir, "archive", "UAREJ100", "rej-100", "rejected");
    // The account's text: the student moved MATH 120 to another course by hand, and ignores OLD.
    let theirs = "timezone: 'America/Chicago'\ncourse_map: {}\ncoursework:\n  zybooks:\n    ignore:\n      - 'UAOLD2020'\n    courses:\n      \
                  'UAMATH120Fall2026':\n        course: 'math-121'\n        label: 'MATH 121'\n";
    let mut cursor = Cursor::default();
    cursor.notes.insert("config/ingest.yaml".into(), hash(INGEST));
    let lines = sync::apply_settings(&dir, &page(&[("config/ingest.yaml", theirs)]), &mut cursor);
    let (cfg, _) = knowlu_engine::coursework::load_coursework_config(&dir).unwrap();
    let courses = cfg["coursework"]["zybooks"]["courses"].as_mapping().unwrap();
    let course_of = |k: &str| courses.get(k).and_then(|m| m.get("course")).and_then(|c| c.as_str()).map(str::to_string);
    assert_eq!(course_of("UACS100Fall2026").as_deref(), Some("cs-100"), "an executed card in archive/ is put back");
    assert_eq!(course_of("UAGN103Fall2026").as_deref(), Some("gn-103"), "an approved card in approvals/ too");
    assert_eq!(course_of("UAMATH120Fall2026").as_deref(), Some("math-121"), "a key the file holds is never changed");
    assert_eq!(course_of("UAOLD2020"), None, "a key the student moved to ignore stays ignored");
    assert_eq!(course_of("UAREJ100"), None, "a rejected card maps nothing");
    assert!(lines.iter().any(|l| l == "sync: config/ingest.yaml — the mapping of UACS100Fall2026 to cs-100, from an approved card, is back"), "{lines:?}");
    // Idempotent: a second pass changes nothing.
    let once = read(&dir, "config/ingest.yaml");
    assert!(knowlu_engine::coursework::apply_approved_mappings(&dir).is_empty());
    assert_eq!(read(&dir, "config/ingest.yaml"), once);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_settings_view_answers_the_wizard_and_brings_back_a_missing_file() {
    let body = serde_json::json!({
        "settings": [{ "path": "config/ingest.yaml", "body": INGEST }, { "path": "config/device.yaml", "body": "x" }],
        "notes": true,
    });
    let mut server = loopback(vec![(200, knowlu_engine::ledger::dumps_value(&body))]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let held = sync::account_settings(&client).expect("a reply");
    let sent = server.requests();
    assert!(sent[0].starts_with("GET /functions/v1/sync-pull?view=settings HTTP/1.1"), "{}", sent[0]);
    assert!(held.notes);
    assert_eq!(held.texts.keys().collect::<Vec<_>>(), vec!["config/ingest.yaml"], "only the six shared paths are read");
    // A file synced before and deleted by hand comes back from the account at the next sync.
    let dir = vault("missing");
    let mut cursor = Cursor::default();
    cursor.notes.insert("config/ingest.yaml".into(), hash(INGEST));
    std::fs::remove_file(dir.join("config/ingest.yaml")).unwrap();
    let mut server = loopback(vec![(200, knowlu_engine::ledger::dumps_value(&body))]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let lines = sync::restore_missing_settings(&dir, &client, &mut cursor);
    let _ = server.requests();
    assert_eq!(lines, vec!["sync: config/ingest.yaml was missing; your account's copy is back".to_string()]);
    assert_eq!(read(&dir, "config/ingest.yaml"), INGEST);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_settings --test device_config`
Expected: FAIL to compile — `cannot find function apply_settings in module sync`, and the same for
`account_settings`, `restore_missing_settings`, `CONFLICTS_DIR`, `coursework::apply_approved_mappings`
and `coursework::insert_mapping`.

- [ ] **Step 3: The pull, the conflict folder, the mappings**

In `engine/src/coursework.rs`, replace

```rust
/// does for every note this engine writes.
fn write_mapping(vault: &Path, source: &str, key: &str, course: &str) -> Result<bool, String> {
    let (block, field) = match source {
        "zybooks" => ("zybooks", "courses"),
        "vhl" => ("vhl", "sections"),
        other => return Err(format!("unknown coursework source {other:?}")),
    };
```

with

```rust
/// does for every note this engine writes.
fn write_mapping(vault: &Path, source: &str, key: &str, course: &str) -> Result<bool, String> {
    // The label prefixes every title the parser produces (`CS 100 HW 01`). Derived from the slug
    // rather than asked for: one field on the card is one decision, and this one is mechanical.
    let label = course.trim().to_uppercase().replace('-', " ");
    insert_mapping(vault, source, key, course, &label)
}

/// Two-desktop design D17: a source block's shared defaults — what `scaffold::ingest_yaml` writes at
/// birth — unindented, one per line. [`insert_mapping`] writes them into a `coursework.<source>:`
/// block it has to create, so a source first set up on a second computer parses as the first
/// computer's would.
pub const ZYBOOKS_SHARED_DEFAULTS: &str = "categories:\n  HW: hw\n  Lab: lab\n  Project: project\n\
effort:\n  minutes_per_section: 6\n  floors:\n    hw: 0.25\n    lab: 0.5\n    project: 1.0\n\
importance:\n  hw: 2\n  lab: 2\n  project: 2\n";
pub const VHL_SHARED_DEFAULTS: &str = "importance: 3\n";

/// [`write_mapping`] with the label given — the wizard's own rows carry one (two-desktop design D19,
/// re-review r6: a portal first set up on this computer gets its route). **Creates a missing
/// `coursework:` or `coursework.<source>:` block** (D17: whether a source is set up is
/// `config/device.yaml`'s `enabled`, never whether the account's shared file has its block yet), with
/// that source's shared defaults. Every other rule is `write_mapping`'s.
pub fn insert_mapping(vault: &Path, source: &str, key: &str, course: &str, label: &str) -> Result<bool, String> {
    let (block, field, defaults) = match source {
        "zybooks" => ("zybooks", "courses", ZYBOOKS_SHARED_DEFAULTS),
        "vhl" => ("vhl", "sections", VHL_SHARED_DEFAULTS),
        other => return Err(format!("unknown coursework source {other:?}")),
    };
```

In `engine/src/coursework.rs`, replace

```rust
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();

    let coursework = find_key(&lines, 0, lines.len(), "coursework", 0)
        .ok_or_else(|| "config/ingest.yaml has no coursework: block".to_string())?;
    let coursework_end = block_end(&lines, coursework);
    let source_depth = child_indent(&lines, coursework, coursework_end);
    let source_line = find_key(&lines, coursework + 1, coursework_end, block, source_depth)
        .ok_or_else(|| format!("config/ingest.yaml has no coursework.{block}: block"))?;
    let source_end = block_end(&lines, source_line);
    let field_depth = child_indent(&lines, source_line, source_end);
```

with

```rust
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    // Where a new top-level key goes: before the final newline's empty line.
    let tail = |lines: &[String]| if lines.last().is_some_and(|l| l.is_empty()) { lines.len() - 1 } else { lines.len() };

    let coursework = match find_key(&lines, 0, lines.len(), "coursework", 0) {
        Some(at) => at,
        None => {
            let at = tail(&lines);
            lines.insert(at, "coursework:".to_string());
            at
        }
    };
    let coursework_end = block_end(&lines, coursework);
    let source_depth = child_indent(&lines, coursework, coursework_end);
    let source_line = match find_key(&lines, coursework + 1, coursework_end, block, source_depth) {
        Some(at) => at,
        None => {
            let at = (coursework + 1..coursework_end).rev().find(|&i| !is_filler(&lines[i])).map(|i| i + 1).unwrap_or(coursework + 1);
            let pad = " ".repeat(source_depth + 2);
            let mut new_lines = vec![format!("{}{block}:", " ".repeat(source_depth))];
            new_lines.extend(defaults.lines().map(|l| format!("{pad}{l}")));
            new_lines.push(format!("{pad}{field}:"));
            lines.splice(at..at, new_lines);
            at
        }
    };
    let source_end = block_end(&lines, source_line);
    let field_depth = child_indent(&lines, source_line, source_end);
```

In `engine/src/coursework.rs`, replace

```rust
    // unquoted `2102121:` is an integer key and `yaml::get(sections, "2102121")` then misses.
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));
    // The label prefixes every title the parser produces (`CS 100 HW 01`). Derived from the slug
    // rather than asked for: one field on the card is one decision, and this one is mechanical.
    let label = course.trim().to_uppercase().replace('-', " ");

    let (insert_at, entry_indent) = match find_key(&lines, source_line + 1, source_end, field, field_depth) {
```

with

```rust
    // unquoted `2102121:` is an integer key and `yaml::get(sections, "2102121")` then misses.
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));

    let (insert_at, entry_indent) = match find_key(&lines, source_line + 1, source_end, field, field_depth) {
```

In `engine/src/coursework.rs`, replace

```rust
            format!("{pad}{}:", lit(key)),
            format!("{pad}  course: {}", lit(course.trim())),
            format!("{pad}  label: {}", lit(&label)),
        ],
    );
    pystr::write_text(&path, &lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(true)
}
```

with

```rust
            format!("{pad}{}:", lit(key)),
            format!("{pad}  course: {}", lit(course.trim())),
            format!("{pad}  label: {}", lit(label)),
        ],
    );
    pystr::write_text(&path, &lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Two-desktop design D18 (§4.8; re-review N2, R2-I1, r5, m6): every mapping a card made, put back
/// after a pull replaced `config/ingest.yaml`. Idempotent and cheap: it inserts the mapping of every
/// `coursework-map` card that is `approved` in `approvals/`, or `approved` or `executed` in `archive/`
/// (as [`apply_map_cards`] leaves it), whose key the file **lacks** — never touching a key the file
/// holds, whatever course it names now, nor a zyBooks key the student moved to `ignore:`. So a mapping
/// made from a card is never lost to a race, and a student's own change to one is kept; a line the
/// student deleted outright comes back, because the card is the source of truth for its key.
pub fn apply_approved_mappings(vault: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
            let field = |k: &str| crate::yaml::opt_text(crate::yaml::get(&meta, k)).unwrap_or_default();
            if field("kind") != "coursework-map" {
                continue;
            }
            let status = field("status");
            let settled = status == "approved" || (folder == "archive" && status == "executed");
            let (source, key, course) = (field("source"), field("map_key"), field("course"));
            if !settled || key.is_empty() || course.trim().is_empty() {
                continue;
            }
            let (config, _) = load_coursework_config(vault).unwrap_or_default();
            let ignored = crate::yaml::get(&config, "coursework")
                .and_then(Yaml::as_mapping)
                .and_then(|cw| crate::yaml::get(cw, &source))
                .and_then(Yaml::as_mapping)
                .and_then(|b| crate::yaml::get(b, "ignore"))
                .and_then(Yaml::as_sequence)
                .is_some_and(|items| items.iter().any(|i| yaml_str(i) == key));
            if ignored {
                continue;
            }
            match write_mapping(vault, &source, &key, &course) {
                Ok(true) => lines.push(format!(
                    "sync: config/ingest.yaml — the mapping of {key} to {course}, from an approved card, is back"
                )),
                Ok(false) => {}
                Err(e) => lines.push(format!(
                    "sync: config/ingest.yaml — the mapping of {key}, from an approved card, could not be put back ({e})"
                )),
            }
        }
    }
    lines
}
```

In `engine/src/sync.rs`, replace

```rust
    pulled_from_reply(&reply)
}

/// Every character a query value may not carry raw
```

with

```rust
    pulled_from_reply(&reply)
}

// ---------------------------------------------------------------------------
// Two-desktop design D18/D19 (§4.8): the shared settings files.
// ---------------------------------------------------------------------------

/// Where a replaced local settings text is kept: `state/config-conflicts/<file>-<ts>.yaml`.
pub const CONFLICTS_DIR: &str = "state/config-conflicts";

/// Copy `text` (this computer's version of shared file `rel`) into [`CONFLICTS_DIR`] and name it.
fn keep_conflict(vault: &Path, rel: &str, text: &str) -> Result<String, String> {
    let dir = vault.join(CONFLICTS_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = Path::new(rel).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ts = jiff::Timestamp::now().strftime("%Y%m%dT%H%M%SZ").to_string();
    let mut name = format!("{stem}-{ts}.yaml");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem}-{ts}-{n}.yaml");
        n += 1;
    }
    crate::pystr::write_text(&dir.join(&name), text).map_err(|e| e.to_string())?;
    Ok(name)
}

/// The account's shared settings rows in one pulled page, taken by D18's **three-way check** against
/// the base (`Cursor::notes`, the hash of the text this computer last synced for the path):
///
/// - the local file equals the base, or is missing: the account's text is written verbatim (a
///   whole-file copy of another computer's file, never a re-dump) and becomes the base;
/// - the local file changed and the account's text equals the base: nothing to take — the push
///   sends the local one;
/// - both changed, or no base yet: the account's text wins, and the local text is kept under
///   [`CONFLICTS_DIR`] unless the two are byte-identical. The first computer to sync after the
///   upgrade therefore provides the settings (Q13); a push the server refused (`config_refused`)
///   becomes this case at the pull that brings the other computer's text.
///
/// **Provisional files** (D19 (a)): while this computer holds a provisional mark, any settings row
/// ends it — the account's text replaces the file (copied to the conflict folder first when it
/// differs) and the mark ends for every path. After any replacement, `apply_approved_mappings`
/// re-inserts every card-made mapping the file lacks. **The second recorded exception** to "never
/// rewrite a vault file wholesale", bounded here: only [`SHARED_CONFIG`] paths, and only from the
/// account's own row.
pub fn apply_settings(vault: &Path, page: &Pulled, cursor: &mut Cursor) -> Vec<String> {
    let mut lines = Vec::new();
    // Rows arrive in `rev` order; the latest for a path is the account's text.
    let mut latest: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for note in &page.notes {
        if let (true, Some(text)) = (crate::config::is_shared_config(&note.path), note.text.as_deref()) {
            latest.insert(note.path.as_str(), text);
        }
    }
    if latest.is_empty() {
        return lines;
    }
    let provisional = !cursor.provisional_config.is_empty();
    let mut replaced = false;
    for (rel, text) in latest {
        let file = vault.join(rel);
        let theirs = sha256_hex(text.as_bytes());
        let local = crate::pystr::read_text(&file).ok();
        let ours = local.as_deref().map(|t| sha256_hex(t.as_bytes()));
        let base = cursor.notes.get(rel).cloned();
        // `None`: nothing to take. `Some(keep)`: take the account's text, keeping ours when `keep`.
        let take: Option<bool> = if ours.as_deref() == Some(theirs.as_str()) {
            cursor.notes.insert(rel.to_string(), theirs.clone());
            None
        } else if provisional || local.is_none() || base.is_none() {
            Some(local.is_some())
        } else if ours == base {
            Some(false)
        } else if base.as_deref() == Some(theirs.as_str()) {
            None
        } else {
            Some(true)
        };
        let Some(keep) = take else { continue };
        if keep {
            match keep_conflict(vault, rel, local.as_deref().unwrap_or_default()) {
                Ok(name) => lines.push(format!(
                    "sync: {rel} — another computer's settings won; yours are in {CONFLICTS_DIR}/{name}"
                )),
                Err(e) => {
                    // Never replace what could not be kept.
                    lines.push(format!("sync: {rel} — another computer's settings could not be taken ({e}); yours stay"));
                    continue;
                }
            }
        }
        match crate::pystr::write_text(&file, text) {
            Ok(()) => {
                cursor.notes.insert(rel.to_string(), theirs);
                replaced = true;
                if !keep {
                    lines.push(format!("sync: {rel} updated from your account"));
                }
            }
            Err(e) => lines.push(format!("sync: {rel} could not be written ({e})")),
        }
    }
    if provisional {
        cursor.provisional_config.clear();
        cursor.provisional_since.clear();
        lines.push("sync: your account's settings arrived; they replace the ones this computer was set up with".to_string());
    }
    if replaced {
        lines.extend(crate::coursework::apply_approved_mappings(vault));
    }
    lines
}

/// What the account holds for the six shared paths, and whether it holds any note (D19's wizard
/// question and a missing file's way back). One `GET /sync-pull?view=settings`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountSettings {
    pub texts: std::collections::BTreeMap<String, String>,
    pub notes: bool,
}

pub fn account_settings(client: &crate::cloudmodel::CloudClient) -> Result<AccountSettings, SyncError> {
    let reply = client.get("/sync-pull?view=settings").map_err(SyncError::service)?;
    let mut out = AccountSettings { notes: reply.get("notes").and_then(Value::as_bool).unwrap_or(false), ..Default::default() };
    for row in reply.get("settings").and_then(Value::as_array).cloned().unwrap_or_default() {
        let (Some(path), Some(body)) = (row.get("path").and_then(Value::as_str), row.get("body").and_then(Value::as_str)) else {
            continue;
        };
        if crate::config::is_shared_config(path) && !body.is_empty() {
            out.texts.insert(path.to_string(), body.to_string());
        }
    }
    Ok(out)
}

/// D18: "a missing one comes back". A shared file this computer has synced before (it has a base)
/// but no longer finds on disk is written back from the account's current text.
pub fn restore_missing_settings(vault: &Path, client: &crate::cloudmodel::CloudClient, cursor: &mut Cursor) -> Vec<String> {
    let missing: Vec<&str> =
        SHARED_CONFIG.iter().copied().filter(|rel| cursor.notes.contains_key(*rel) && !vault.join(rel).exists()).collect();
    if missing.is_empty() {
        return Vec::new();
    }
    let held = match account_settings(client) {
        Ok(held) => held,
        Err(e) => return vec![format!("sync: a missing settings file could not be fetched back ({})", e.label())],
    };
    let mut lines = Vec::new();
    for rel in missing {
        let Some(text) = held.texts.get(rel) else { continue };
        match crate::pystr::write_text(&vault.join(rel), text) {
            Ok(()) => {
                cursor.notes.insert(rel.to_string(), sha256_hex(text.as_bytes()));
                lines.push(format!("sync: {rel} was missing; your account's copy is back"));
            }
            Err(e) => lines.push(format!("sync: {rel} could not be written ({e})")),
        }
    }
    lines
}

/// Every character a query value may not carry raw
```

In `engine/src/sync.rs`, replace

```rust
        .collect();
    for note in ordered_notes {
        if !is_note_path(vault, &note.path) {
            report.refused += 1;
```

with

```rust
        .collect();
    for note in ordered_notes {
        // Two-desktop design D18: a shared settings row has no `id:` and never meets §2.5's rules;
        // `apply_settings` takes it by its three-way check.
        if crate::config::is_shared_config(&note.path) {
            continue;
        }
        if !is_note_path(vault, &note.path) {
            report.refused += 1;
```

In `engine/src/sync.rs`, replace

```rust
                    totals.pulled_records, totals.pulled_notes, report.applied, report.cards, report.refused
                ));
                cursor.record_cursor = page.record_cursor;
                cursor.note_cursor = page.note_cursor;
```

with

```rust
                    totals.pulled_records, totals.pulled_notes, report.applied, report.cards, report.refused
                ));
                // Two-desktop design D18/D19: the settings rows this page carried, by the three-way check.
                lines.extend(apply_settings(vault, &page, &mut cursor));
                cursor.record_cursor = page.record_cursor;
                cursor.note_cursor = page.note_cursor;
```

In `engine/src/sync.rs`, replace

```rust
                if !page.more {
                    cursor.pulled_to_end = true;
                }
                if let Err(e) = save_cursor(vault, &cursor) {
```

with

```rust
                if !page.more {
                    cursor.pulled_to_end = true;
                }
                // D18: a settings file synced before and missing now comes back from the account.
                lines.extend(restore_missing_settings(vault, client, &mut cursor));
                if let Err(e) = save_cursor(vault, &cursor) {
```

- [ ] **Step 4: Run the tests**

Run the command of Step 2. Expected: PASS — seven in `sync_settings.rs`, twelve in `device_config.rs`.
Then `--lib -- coursework::` (the `write_mapping` tests, byte-for-byte insertions unchanged) and
`--test sync_contract --test sync_replay`: PASS.

- [ ] **Step 5: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-5.txt`:

```
feat: sync takes the account's settings by a three-way check (two desktops, D18)

A pulled settings text is written verbatim when this computer's copy is
unchanged since the last sync (or missing), left alone when only this
computer changed it, and wins when both changed or nothing was synced yet
- the first computer to sync provides the settings (Q13) - with this
computer's text kept under state/config-conflicts/ and named. Every
card-made mapping the new text lacks is put back. A settings file synced
before and missing now comes back from the account.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/src/coursework.rs engine/tests/device_config.rs engine/tests/sync_settings.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-5.txt`

---

### Task 6: Provisional settings — the mark, its endings (b) and (c), and the restore (spec §4.8 D18, D19; R3-I1)

**Files:**
- Modify: `engine/src/sync.rs` — after Task 5's block: `PROVISIONAL_DAYS`, `publish_if_due`,
  `settings_edited`, `mark_provisional`; `materialise` (`:1034-1183`) takes a settings row (written over
  the wizard's file and recorded as the base, R-TD2-17) and never settles a settings tombstone;
  `RestoreState` (`:1417-1431`) gains `settings_arrived` and `notes_arrived` (at its top, clear of the
  `undone` field Plan 1's fix round adds after `written_ids`); `restore_all` becomes a wrapper over the new
  `restore_all_as`, which marks the files provisional (R-TD2-12); `restore_into` becomes a wrapper over
  the new `restore_into_as`, whose two could-not-read arms mark them when the wizard saw notes;
  `run_lines_with_client`'s pull calls `publish_if_due` after `restore_missing_settings`.
- Modify: `engine/tests/sync_settings.rs` — five tests.

**Interfaces:**
- Consumes: Tasks 4–5's `Cursor` fields, `apply_settings`' provisional branch (D18 (a)), `build_push`'s hold.
- Produces: `sync::{PROVISIONAL_DAYS, publish_if_due, settings_edited, mark_provisional, restore_all_as,
  restore_into_as}`.

**The three endings, and what never ends the mark.** (a) is Task 5's: any settings row a pull brings.
(b) is `settings_edited`, the seam a console command calls after it writes a shared setting (R-TD2-11:
none does today; the test drives the seam). (c) is `publish_if_due`, on this computer's own clock since
`provisional_since`, once a pull has read the account to its end (R-TD2-10); it clears the marked paths'
bases, so the push that follows sends each with `base: ""` and `save_config_row` inserts it only where the
account still holds none — a row another computer stored meanwhile comes back in `config_refused`, and the
next pull takes it by the no-base branch. `apply_map_cards`, `apply_approved_mappings`, the migration and
the wizard's own writes change a provisional file and leave the mark alone: none of them touches the
cursor.

- [ ] **Step 1: Write the failing tests**

Append to `engine/tests/sync_settings.rs` (after one blank line):

```rust
/// A computer born into an account with notes and no settings: every shared file marked.
fn provisional(tag: &str) -> PathBuf {
    let dir = vault(tag);
    let mut cursor = Cursor::default();
    sync::mark_provisional(&dir, &mut cursor, jiff::Timestamp::now());
    cursor.pulled_to_end = true;
    sync::save_cursor(&dir, &cursor).unwrap();
    dir
}

#[test]
fn automatic_writes_change_a_provisional_file_and_publish_nothing() {
    let dir = provisional("auto");
    assert_eq!(sync::load_cursor(&dir).provisional_config, sync::SHARED_CONFIG.to_vec(), "all six are marked");
    // A map card applied on a portal holder, `apply_approved_mappings` and the migration all write.
    map_card(&dir, "approvals", "UACS100Fall2026", "cs-100", "approved");
    let ctx = knowlu_engine::write::WriteContext::new("agent:coursework", "local-runner");
    let mut journal = Journal::new(&dir);
    let lines = knowlu_engine::coursework::apply_map_cards(&dir, &ctx, &mut journal, false);
    assert!(lines.iter().any(|l| l.contains("mapped UACS100Fall2026")), "{lines:?}");
    map_card(&dir, "archive", "UAGN103Fall2026", "gn-103", "executed");
    assert!(!knowlu_engine::coursework::apply_approved_mappings(&dir).is_empty());
    put(&dir, "config/runners.yaml", "runners:\n  - name: local\n    times: ['12:00']\n    tz: 'UTC'\n    grace_minutes: 20\n    device: 'X'\n");
    assert!(!knowlu_engine::config::move_device_keys(&dir).is_empty());
    let cursor = sync::load_cursor(&dir);
    assert_eq!(cursor.provisional_config, sync::SHARED_CONFIG.to_vec(), "no automatic write ends the mark");
    let (batch, _) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    assert!(batch.settings.is_empty(), "and nothing provisional is published: {:?}", batch.settings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_accounts_settings_arriving_by_pull_replace_the_files_and_end_the_mark() {
    let dir = provisional("arrive");
    let theirs_runners = "runners:\n  - name: local\n    times: ['07:00']\n    tz: 'America/New_York'\n    grace_minutes: 20\n";
    let (lines, sent) = run(&dir, vec![pull_reply(&[("config/runners.yaml", theirs_runners)], 1), push_reply(&[])]);
    assert_eq!(read(&dir, "config/runners.yaml"), theirs_runners, "the account's text replaces the file");
    assert_eq!(conflicts(&dir).len(), 1, "this computer's differing text is kept");
    assert!(lines.contains(&"sync: your account's settings arrived; they replace the ones this computer was set up with".to_string()), "{lines:?}");
    let cursor = sync::load_cursor(&dir);
    assert!(cursor.provisional_config.is_empty() && cursor.provisional_since.is_empty(), "the mark ends for every path");
    // The files the page did not carry go up with empty bases: stored only where the account has none.
    let rows = pushed_settings(&sent[1]);
    assert_eq!(rows.len(), 5, "{rows:?}");
    assert!(rows.iter().all(|r| r["base"] == "" && r["path"] != "config/runners.yaml"), "{rows:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_edit_in_the_app_publishes_that_file_and_only_that_file() {
    let dir = provisional("edited");
    put(&dir, "config/runners.yaml", "runners:\n  - name: local\n    times: ['08:00', '18:00']\n    tz: 'America/Chicago'\n    grace_minutes: 20\n");
    assert!(sync::settings_edited(&dir, "config/runners.yaml").unwrap(), "the mark on that file ends");
    assert!(!sync::settings_edited(&dir, "config/runners.yaml").unwrap(), "once");
    let cursor = sync::load_cursor(&dir);
    assert_eq!(cursor.provisional_config.len(), 5);
    let mut journal = Journal::new(&dir);
    let (batch, _) = sync::build_push(&dir, &cursor, "acct-1", &mut journal);
    assert_eq!(batch.settings.len(), 1);
    assert_eq!((batch.settings[0]["path"].as_str(), batch.settings[0]["base"].as_str()), (Some("config/runners.yaml"), Some("")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn seven_days_with_no_settings_in_the_account_publish_every_marked_file_with_empty_bases() {
    let now = jiff::Timestamp::now();
    let mut cursor = Cursor { pulled_to_end: true, ..Default::default() };
    cursor.provisional_config = vec!["config/planning.yaml".into()];
    cursor.provisional_since = knowlu_engine::journal::now_ts(Some(now - jiff::SignedDuration::from_hours(6 * 24 + 23)));
    assert_eq!(sync::publish_if_due(&mut cursor, now), None, "six days and 23 hours is not seven");
    cursor.pulled_to_end = false;
    assert_eq!(sync::publish_if_due(&mut cursor, now + jiff::SignedDuration::from_hours(2)), None, "never before a pull has read the account to its end");
    cursor.provisional_since = "not a time".into();
    assert_eq!(sync::publish_if_due(&mut Cursor { pulled_to_end: true, ..cursor.clone() }, now), None, "an unreadable stamp restarts the wait");

    // End to end, with the clock injected by the stamp: eight days ago, and one row another computer
    // stored meanwhile turns that file's insert into a refusal.
    let dir = provisional("seven-days");
    let mut cursor = sync::load_cursor(&dir);
    cursor.provisional_since = knowlu_engine::journal::now_ts(Some(now - jiff::SignedDuration::from_hours(8 * 24)));
    sync::save_cursor(&dir, &cursor).unwrap();
    let (lines, sent) = run(&dir, vec![pull_reply(&[], 1), push_reply(&["config/campus.yaml"])]);
    assert!(lines.contains(&"sync: no other computer has shared settings for 7 days; this computer's settings are now the account's".to_string()), "{lines:?}");
    let rows = pushed_settings(&sent[1]);
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| r["base"] == ""), "empty bases: an insert only where the account has none");
    assert!(lines.contains(&"sync: config/campus.yaml — another computer changed it first; yours waits for the next pull".to_string()), "{lines:?}");
    let after = sync::load_cursor(&dir);
    assert!(after.provisional_config.is_empty());
    assert!(!after.notes.contains_key("config/campus.yaml"), "the refused one keeps no base");
    assert_eq!(after.notes["config/planning.yaml"], hash(&read(&dir, "config/planning.yaml")), "a stored one is synced");
    let _ = std::fs::remove_dir_all(&dir);
}

/// One `/sync-pull` page for a restore: the given settings rows, and one note when `with_note`.
fn restore_page(settings: &[(&str, &str)], with_note: bool) -> (u16, String) {
    let mut notes: Vec<serde_json::Value> = settings
        .iter()
        .map(|(path, body)| serde_json::json!({ "rev": 1, "device": "fedcba9876543210", "path": path, "deleted": false, "body": body }))
        .collect();
    if with_note {
        notes.push(serde_json::json!({ "rev": 2, "device": "fedcba9876543210", "path": "tasks/from-a.md", "deleted": false, "body": "---\nid: task_0000000099\n---\nfrom the other computer\n" }));
    }
    let reply = serde_json::json!({ "records": [], "notes": notes, "record_cursor": 0, "note_cursor": 2, "more": false });
    (200, knowlu_engine::ledger::dumps_value(&reply))
}

#[test]
fn a_restore_writes_the_accounts_settings_or_marks_this_computers_provisional() {
    // Settings in the account: the wizard's defaults are replaced, recorded as the base, no mark.
    let dir = vault("restore-settings");
    let mut server = loopback(vec![restore_page(&[("config/planning.yaml", THEIRS)], true)]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let restored = sync::restore_all_as(&dir, &client, &[], false).expect("a restore");
    let _ = server.requests();
    assert_eq!(restored.notes, 2);
    assert_eq!(read(&dir, "config/planning.yaml"), THEIRS);
    let cursor = sync::load_cursor(&dir);
    assert_eq!(cursor.notes["config/planning.yaml"], hash(THEIRS));
    assert!(cursor.provisional_config.is_empty(), "the account's settings are here: nothing provisional");
    // Notes but no settings: every shared file is marked.
    let dir2 = vault("restore-notes");
    let mut server = loopback(vec![restore_page(&[], true)]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    sync::restore_all_as(&dir2, &client, &[], false).expect("a restore");
    let _ = server.requests();
    let cursor = sync::load_cursor(&dir2);
    assert_eq!(cursor.provisional_config, sync::SHARED_CONFIG.to_vec());
    assert!(cursor.provisional_since.parse::<jiff::Timestamp>().is_ok());
    // An empty account: this computer's answers are real from the start.
    let dir3 = vault("restore-empty");
    let mut server = loopback(vec![restore_page(&[], false)]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    sync::restore_all_as(&dir3, &client, &[], false).expect("a restore");
    let _ = server.requests();
    assert!(sync::load_cursor(&dir3).provisional_config.is_empty());
    // The account could not be read, but the wizard saw notes there: still marked.
    let dir4 = vault("restore-unread");
    let out = sync::restore_into_as(&dir4, true).expect("a network failure is not an error");
    assert!(!out.ok);
    assert_eq!(sync::load_cursor(&dir4).provisional_config, sync::SHARED_CONFIG.to_vec());
    for d in [dir, dir2, dir3, dir4] {
        let _ = std::fs::remove_dir_all(&d);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_settings`
Expected: FAIL to compile — `cannot find function mark_provisional in module sync`, and the same for
`publish_if_due`, `settings_edited`, `restore_all_as`, `restore_into_as`.

- [ ] **Step 3: The mark, (b), (c), and the restore**

In `engine/src/sync.rs`, replace

```rust
    lines
}

/// Every character a query value may not carry raw
```

with

```rust
    lines
}

/// D19 (c): how long a provisional mark waits for the account's settings.
pub const PROVISIONAL_DAYS: i64 = 7;

/// D19 (c): a provisional mark older than [`PROVISIONAL_DAYS`] on this computer's own clock, with the
/// account's copy read to the end (`pulled_to_end`) and no settings in it, ends: the marked files are
/// published as they stand, with empty bases, so `save_config_row` inserts each only where the
/// account still holds none. An unreadable `provisional_since` is stamped `now` and the wait starts
/// again — never "due at once", never "never". `Some(line)` when the mark ended.
pub fn publish_if_due(cursor: &mut Cursor, now: jiff::Timestamp) -> Option<String> {
    if cursor.provisional_config.is_empty() || !cursor.pulled_to_end {
        return None;
    }
    let Ok(since) = cursor.provisional_since.parse::<jiff::Timestamp>() else {
        cursor.provisional_since = crate::journal::now_ts(Some(now));
        return None;
    };
    if now.as_second() - since.as_second() < PROVISIONAL_DAYS * 86_400 {
        return None;
    }
    for rel in std::mem::take(&mut cursor.provisional_config) {
        cursor.notes.remove(&rel);
    }
    cursor.provisional_since.clear();
    Some("sync: no other computer has shared settings for 7 days; this computer's settings are now the account's".to_string())
}

/// D19 (b): the student changed shared file `rel` through the app. Its provisional mark ends and it
/// is pushed at the next sync with an empty base. `Ok(true)` when a mark ended. The engine seam every
/// console command that writes a shared setting calls after its write.
pub fn settings_edited(vault: &Path, rel: &str) -> Result<bool, SyncError> {
    let mut cursor = load_cursor(vault);
    let before = cursor.provisional_config.len();
    cursor.provisional_config.retain(|p| p != rel);
    if cursor.provisional_config.len() == before {
        return Ok(false);
    }
    if cursor.provisional_config.is_empty() {
        cursor.provisional_since.clear();
    }
    cursor.notes.remove(rel);
    save_cursor(vault, &cursor)?;
    Ok(true)
}

/// Mark every shared settings file present in `vault` provisional, stamped with `now` (D19).
pub fn mark_provisional(vault: &Path, cursor: &mut Cursor, now: jiff::Timestamp) {
    cursor.provisional_config =
        SHARED_CONFIG.iter().filter(|rel| vault.join(rel).is_file()).map(|rel| rel.to_string()).collect();
    cursor.provisional_since =
        if cursor.provisional_config.is_empty() { String::new() } else { crate::journal::now_ts(Some(now)) };
}

/// Every character a query value may not carry raw
```

In `engine/src/sync.rs`, replace

```rust
            continue;
        }
        if !is_note_path(dest, &note.path) {
            out.warnings.push("restore: a note named a path outside the vault's notes".to_string());
```

with

```rust
            continue;
        }
        // Two-desktop design D18: a shared settings file is never tombstoned (`sync_rows.ts` refuses
        // one), so a tombstone for one settles nothing.
        if crate::config::is_shared_config(&note.path) {
            continue;
        }
        if !is_note_path(dest, &note.path) {
            out.warnings.push("restore: a note named a path outside the vault's notes".to_string());
```

In `engine/src/sync.rs`, replace

```rust
    for note in &page.notes {
        let Some(text) = note.text.as_deref() else { continue };
        if !is_note_path(dest, &note.path) {
            out.warnings.push("restore: a note named a path outside the vault's notes".to_string());
            continue;
```

with

```rust
    for note in &page.notes {
        let Some(text) = note.text.as_deref() else { continue };
        // Two-desktop design D18/D19: the account's shared settings replace the files the wizard
        // wrote for the panels it skipped — whole, verbatim, and recorded as this computer's base.
        let settings = crate::config::is_shared_config(&note.path);
        if !settings && !is_note_path(dest, &note.path) {
            out.warnings.push("restore: a note named a path outside the vault's notes".to_string());
            continue;
```

In `engine/src/sync.rs`, replace

```rust
            Ok(()) => {
                out.notes += 1;
                // R4: hash what is ACTUALLY on disk now that the write is confirmed to have
                // succeeded — never assumed just because `page` carried a live row for this path.
```

with

```rust
            Ok(()) => {
                out.notes += 1;
                if settings {
                    state.settings_arrived = true;
                } else {
                    state.notes_arrived = true;
                }
                // R4: hash what is ACTUALLY on disk now that the write is confirmed to have
                // succeeded — never assumed just because `page` carried a live row for this path.
```

In `engine/src/sync.rs`, replace

```rust
struct RestoreState {
    touched: Touched,
    /// R2: every tombstone `materialise` has seen so far, collected but never executed. See
    /// [`settle_tombstones`].
```

with

```rust
struct RestoreState {
    touched: Touched,
    /// Two-desktop design D19: a shared settings row landed, so the account already holds settings.
    settings_arrived: bool,
    /// Two-desktop design D19: a note row landed, so this computer was born into an account with notes.
    notes_arrived: bool,
    /// R2: every tombstone `materialise` has seen so far, collected but never executed. See
    /// [`settle_tombstones`].
```

In `engine/src/sync.rs`, replace

```rust
/// left it.
pub fn restore_all(dest: &Path, client: &crate::cloudmodel::CloudClient, tolerate: &[String]) -> Result<Restored, SyncError> {
    if let Some(stray) = unexpected_notes(dest, tolerate).first() {
        return Err(SyncError::Io(format!("{stray} is already here and was not part of this new vault")));
```

with

```rust
/// left it.
pub fn restore_all(dest: &Path, client: &crate::cloudmodel::CloudClient, tolerate: &[String]) -> Result<Restored, SyncError> {
    restore_all_as(dest, client, tolerate, false)
}

/// [`restore_all`], told whether the wizard's `account_settings_exist` already saw notes in the
/// account (two-desktop design D19). When the restore brought no settings but the account holds notes
/// — this page's own, or the wizard's answer — every shared file this vault holds is marked
/// provisional in the cursor it saves.
pub fn restore_all_as(
    dest: &Path,
    client: &crate::cloudmodel::CloudClient,
    tolerate: &[String],
    account_holds_notes: bool,
) -> Result<Restored, SyncError> {
    if let Some(stray) = unexpected_notes(dest, tolerate).first() {
        return Err(SyncError::Io(format!("{stray} is already here and was not part of this new vault")));
```

In `engine/src/sync.rs`, replace

```rust
    // seeds the account never had (N1) instead of holding them back.
    cursor.pulled_to_end = reached_end;
    total.empty = total.notes == 0 && total.records == 0;
    // Saved only here, after every page has landed and every tombstone settled (I2): a cursor saved
```

with

```rust
    // seeds the account never had (N1) instead of holding them back.
    cursor.pulled_to_end = reached_end;
    // Two-desktop design D19 (R3-I1): born into an account that holds notes but no settings, this
    // computer's shared files are provisional — used here at once, never pushed until the account's
    // settings arrive, a settings edit in the app, or seven days pass.
    if !state.settings_arrived && (state.notes_arrived || total.records > 0 || account_holds_notes) {
        mark_provisional(dest, &mut cursor, jiff::Timestamp::now());
    }
    total.empty = total.notes == 0 && total.records == 0;
    // Saved only here, after every page has landed and every tombstone settled (I2): a cursor saved
```

In `engine/src/sync.rs`, replace

```rust
/// (`not_fresh`) is refused by name, and nothing is written.
pub fn restore_into(dest: &Path) -> Result<Restored, String> {
    if let Some(why) = not_fresh(dest) {
        return Err(format!("restore refused: {why}; a restore only ever fills a vault the wizard has just made"));
```

with

```rust
/// (`not_fresh`) is refused by name, and nothing is written.
pub fn restore_into(dest: &Path) -> Result<Restored, String> {
    restore_into_as(dest, false)
}

/// Two-desktop design D19: a restore that could not read the account at all still marks the shared
/// files provisional when the wizard already saw notes there (`account_holds_notes`), so a first push
/// cannot publish them over a working computer's settings.
fn mark_unread_restore(dest: &Path, account_holds_notes: bool, warnings: &mut Vec<String>) {
    if !account_holds_notes {
        return;
    }
    let mut cursor = Cursor::default();
    mark_provisional(dest, &mut cursor, jiff::Timestamp::now());
    if let Err(e) = save_cursor(dest, &cursor) {
        warnings.push(format!("restore: the cursor could not be saved ({e})"));
    }
}

/// [`restore_into`], with the wizard's `account_settings_exist` answer (two-desktop design D19).
pub fn restore_into_as(dest: &Path, account_holds_notes: bool) -> Result<Restored, String> {
    if let Some(why) = not_fresh(dest) {
        return Err(format!("restore refused: {why}; a restore only ever fills a vault the wizard has just made"));
```

In `engine/src/sync.rs`, replace

```rust
        Err(e) => {
            // N3: `ok: false` is the field the page keys its sentence on now — never `warnings`.
            return Ok(Restored { empty: true, ok: false, warnings: vec![format!("restore: {e}; the first slot will fill this vault")], ..Default::default() });
        }
    };
    match restore_all(dest, &client, &tolerate) {
        Ok(r) => Ok(r),
        // N4 carry-forward: Task 7 gave `SyncError::Service` its `{ cause, transport }` shape, so the
```

with

```rust
        Err(e) => {
            // N3: `ok: false` is the field the page keys its sentence on now — never `warnings`.
            let mut warnings = vec![format!("restore: {e}; the first slot will fill this vault")];
            mark_unread_restore(dest, account_holds_notes, &mut warnings);
            return Ok(Restored { empty: true, ok: false, warnings, ..Default::default() });
        }
    };
    match restore_all_as(dest, &client, &tolerate, account_holds_notes) {
        Ok(r) => Ok(r),
        // N4 carry-forward: Task 7 gave `SyncError::Service` its `{ cause, transport }` shape, so the
```

In `engine/src/sync.rs`, replace

```rust
        // either way the account could not be read just now, and the first slot will fill the vault.
        Err(SyncError::Service { cause, .. }) => {
            Ok(Restored { empty: true, ok: false, warnings: vec![format!("restore: {cause}; the first slot will fill this vault")], ..Default::default() })
        }
        Err(e) => Err(format!("{e}")),
```

with

```rust
        // either way the account could not be read just now, and the first slot will fill the vault.
        Err(SyncError::Service { cause, .. }) => {
            let mut warnings = vec![format!("restore: {cause}; the first slot will fill this vault")];
            mark_unread_restore(dest, account_holds_notes, &mut warnings);
            Ok(Restored { empty: true, ok: false, warnings, ..Default::default() })
        }
        Err(e) => Err(format!("{e}")),
```

In `engine/src/sync.rs`, replace

```rust
                lines.extend(restore_missing_settings(vault, client, &mut cursor));
                if let Err(e) = save_cursor(vault, &cursor) {
```

with

```rust
                lines.extend(restore_missing_settings(vault, client, &mut cursor));
                // D19 (c): seven days with no settings in the account end a provisional mark.
                if let Some(line) = publish_if_due(&mut cursor, jiff::Timestamp::now()) {
                    lines.push(line);
                }
                if let Err(e) = save_cursor(vault, &cursor) {
```

- [ ] **Step 4: Run the tests**

Run the command of Step 2. Expected: PASS, all twelve. Then `--test sync_replay --test sync_contract`:
PASS — every restore test is unchanged (`restore_all` and `restore_into` pass `false`, and a restore that
brings notes and no settings now also leaves a mark in the cursor it saves, which no existing test reads).

- [ ] **Step 5: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings — the app's `create_vault_in` tests included (they
call `restore_into`, unchanged for them).

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-6.txt`:

```
feat: a new computer's settings stay provisional until the account's arrive (two desktops, D18/D19)

A computer born into an account that holds notes but no settings marks its
shared files provisional in the cursor: used at once, never pushed. The
mark ends when the account's settings arrive by pull, when the student
edits a setting in the app (settings_edited), or after seven days on this
computer's clock with the account read to its end - then the files go up
with empty bases, stored only where the account holds none. A restore
writes the account's settings over the wizard's defaults.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/tests/sync_settings.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-6.txt`

---

### Task 7: The app births split vaults and reads its own keys first (spec §4.8 D17; the wizard's writes)

**Files:**
- Modify: `app/src/scaffold.rs` — `ingest_yaml` (`:430-549`) writes no `ics_url`, no raw calendar, no
  `enabled` and no `credential_target`, and takes its zyBooks/VHL defaults from the engine's constants; a
  new `device_yaml(p)` (through `config::device_yaml`); `restore_capability_url` (`:563-585`) writes into
  `config/device.yaml` (R-TD2-19); `runners_yaml` (`:590-603`) drops `device` and `scheduler`;
  `build_into` (`:667-700`) writes `config/device.yaml`.
- Modify: `app/src/scheduler.rs` — `ics_state` (`:239-247`) overlays `device.yaml`; `mode` and
  `device_ok` (`:447-461`) read `runs::local_runner_settings`.
- Modify: `app/tests/scaffold.rs` — six tests move their device-key assertions to `config/device.yaml`,
  and one test is new.
- Modify: `app/tests/onboarding.rs` — `read_device`, and six `create_vault_in` tests read the LMS feed
  or the personal calendar from `config/device.yaml`.

**Interfaces:**
- Consumes: `config::{device_yaml, DeviceKeys, SourceKeys, DEVICE_FILE, load_device, set_device_ics_url,
  add_device_calendar, overlay_device, computer_only_value, SHARED_CONFIG, move_device_keys}`,
  `coursework::{ZYBOOKS_SHARED_DEFAULTS, VHL_SHARED_DEFAULTS, load_coursework_config}`,
  `runs::local_runner_settings`.
- Produces: `scaffold::device_yaml`; a wizard-born vault that is split from birth (nothing for the
  migration to move, every shared file clean for the guard).

What changes for a student: nothing they can see. A new vault's `config/ingest.yaml` loses its
`ics_url: ''` line, each source's `enabled`/`credential_target`, and the two comment lines that pointed at
them (one line now says each computer names its own logins in `config/device.yaml`); `runners.yaml` loses
`device`/`scheduler`; `config/device.yaml` carries all of it in D17's literal shape. Every reader reads
the same values (Task 1). Which test assertions move, and why: the six existing tests asserted that a
device key or a capability URL sits in `ingest.yaml` or `runners.yaml`, which D17 now forbids; each now
asserts the same value in `device.yaml` and its absence from the shared file.

- [ ] **Step 1: Write the failing tests**

In `app/tests/scaffold.rs`, replace

```rust
use knowlu::commands::state_inner;
use knowlu::scaffold::{campus_yaml, create_vault, ingest_yaml, runners_yaml, VaultPlan, CAMPUSES};
use knowlu::state::ConsoleState;
use std::path::{Path, PathBuf};
```

with

```rust
use knowlu::commands::state_inner;
use knowlu::scaffold::{campus_yaml, create_vault, device_yaml, ingest_yaml, runners_yaml, VaultPlan, CAMPUSES};
use knowlu::state::ConsoleState;
use std::path::{Path, PathBuf};
```

In `app/tests/scaffold.rs`, replace

```rust

/// Decision 4: a vault born in the wizard is `scheduler: app` on the machine that made it, from
/// birth — and `device:` still gates a second install (F3).
#[test]
fn a_fresh_vault_is_scheduler_app_on_the_machine_that_made_it() {
```

with

```rust

/// Decision 4: a vault born in the wizard is `scheduler: app` on the machine that made it, from
/// birth — and `device:` still gates a second install (F3). Two-desktop design D17: both keys are
/// this computer's, in `config/device.yaml`; `config/runners.yaml` carries only the shared times.
#[test]
fn a_fresh_vault_is_scheduler_app_on_the_machine_that_made_it() {
```

In `app/tests/scaffold.rs`, replace

```rust
    create_vault(&v, &p).unwrap();
    let cfg = v.join("config").join("runners.yaml");
    let s = knowlu_engine::runs::runner_settings(&cfg, "local");
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App);
    assert_eq!(s.device.as_deref(), Some("TEST-MACHINE"));
    let runners = knowlu_engine::runs::load_runners_config(&cfg).unwrap();
    let local = runners.iter().find(|r| r.name == "local").unwrap();
```

with

```rust
    create_vault(&v, &p).unwrap();
    let cfg = v.join("config").join("runners.yaml");
    let s = knowlu_engine::runs::local_runner_settings(&v);
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App);
    assert_eq!(s.device.as_deref(), Some("TEST-MACHINE"));
    let shared = knowlu_engine::runs::runner_settings(&cfg, "local");
    assert_eq!((shared.device, shared.scheduler), (None, knowlu_engine::schedule::SchedulerMode::Script), "neither is in the shared file");
    let runners = knowlu_engine::runs::load_runners_config(&cfg).unwrap();
    let local = runners.iter().find(|r| r.name == "local").unwrap();
```

In `app/tests/scaffold.rs`, replace

```rust
    assert_eq!(local.times, p.slots, "each slot survives whole, commas and colons included");
    assert_eq!(local.grace_minutes, 20);
    let s = knowlu_engine::runs::runner_settings(&cfg, "local");
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App, "the runner is NOT silently switched off");
    assert_eq!(s.device.as_deref(), Some(p.device.as_str()));
```

with

```rust
    assert_eq!(local.times, p.slots, "each slot survives whole, commas and colons included");
    assert_eq!(local.grace_minutes, 20);
    let s = knowlu_engine::runs::local_runner_settings(&v);
    assert_eq!(s.scheduler, knowlu_engine::schedule::SchedulerMode::App, "the runner is NOT silently switched off");
    assert_eq!(s.device.as_deref(), Some(p.device.as_str()));
```

In `app/tests/scaffold.rs`, replace

```rust
    assert_eq!(ev.sources.len(), 6);

    // `ics_url` round-trips byte for byte — the engine fetches exactly what was pasted.
    let text = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("ingest.yaml still parses");
    assert_eq!(parsed.get("ics_url").and_then(|u| u.as_str()), Some(url));
    assert_eq!(parsed.get("timezone").and_then(|t| t.as_str()), Some(nasty));
    assert_eq!(
```

with

```rust
    assert_eq!(ev.sources.len(), 6);

    // `ics_url` round-trips byte for byte — the engine fetches exactly what was pasted — from this
    // computer's own `config/device.yaml` (two-desktop design D17), never the shared file.
    let text = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("ingest.yaml still parses");
    assert_eq!(parsed.get("ics_url"), None, "the fallback is not a shared value");
    let device: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(v.join("config").join("device.yaml")).unwrap()).expect("device.yaml parses");
    assert_eq!(device.get("ics_url").and_then(|u| u.as_str()), Some(url));
    assert_eq!(parsed.get("timezone").and_then(|t| t.as_str()), Some(nasty));
    assert_eq!(
```

In `app/tests/scaffold.rs`, replace

```rust
        "the slot still knows there is a feed"
    );
    let cw = parsed.get("coursework").expect("the coursework block survives");
    assert_eq!(cw.get("zybooks").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/zybooks"));
    assert_eq!(cw.get("vhl").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/vhl"));

    // m2: a student-typed label carrying YAML metacharacters round-trips through the parsed-back
```

with

```rust
        "the slot still knows there is a feed"
    );
    let (merged, _) = knowlu_engine::coursework::load_coursework_config(&v).unwrap();
    let merged_cw = merged.get("coursework").expect("the merged coursework block");
    assert_eq!(merged_cw.get("zybooks").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/zybooks"));
    assert_eq!(merged_cw.get("vhl").and_then(|z| z.get("credential_target")).and_then(|t| t.as_str()), Some("knowlu/profile_8888888888/vhl"));
    let cw = parsed.get("coursework").expect("the coursework block survives");
    assert_eq!(cw.get("zybooks").and_then(|z| z.get("credential_target")), None, "a credential target is this computer's");

    // m2: a student-typed label carrying YAML metacharacters round-trips through the parsed-back
```

In `app/tests/scaffold.rs`, replace

```rust
    // The two generators say the same thing on their own, so a future caller cannot route round it.
    assert!(ingest_yaml(&{ let mut b = plan_for(&root.join("V-ingest-err")); b.timezone = "a\rb".into(); b }).is_err());
    assert!(runners_yaml(&{ let mut b = plan_for(&root.join("V-runners-err")); b.device = "a\rb".into(); b }).is_err());
}
```

with

```rust
    // The two generators say the same thing on their own, so a future caller cannot route round it.
    assert!(ingest_yaml(&{ let mut b = plan_for(&root.join("V-ingest-err")); b.timezone = "a\rb".into(); b }).is_err());
    assert!(runners_yaml(&{ let mut b = plan_for(&root.join("V-runners-err")); b.timezone = "a\rb".into(); b }).is_err());
    assert!(device_yaml(&{ let mut b = plan_for(&root.join("V-device-err")); b.device = "a\rb".into(); b }).is_err());
}
```

In `app/tests/scaffold.rs`, replace

```rust
    assert_eq!(others[0]["actor"], "quinn");
    assert_eq!(others[0]["via"], "dashboard");
    // The credential target the engine will read is named, and holds no secret.
    let ingest = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    let target = knowlu::credentials::target_for(&knowlu::profiles::id_for(&v), "zybooks");
    assert!(ingest.contains(&format!("credential_target: '{target}'")), "{ingest}");
    assert!(!ingest.contains("vhl:"), "a friend with no VHL course gets no VHL block");
}
```

with

```rust
    assert_eq!(others[0]["actor"], "quinn");
    assert_eq!(others[0]["via"], "dashboard");
    // The credential target the engine will read is named — in this computer's own
    // `config/device.yaml` (two-desktop design D17) — and holds no secret.
    let device = knowlu_engine::pystr::read_text(&v.join("config").join("device.yaml")).unwrap();
    let target = knowlu::credentials::target_for(&knowlu::profiles::id_for(&v), "zybooks");
    assert!(device.contains(&format!("  zybooks:\n    enabled: true\n    credential_target: '{target}'\n")), "{device}");
    assert!(device.contains("  vhl: {}\n"), "{device}");
    let ingest = std::fs::read_to_string(v.join("config").join("ingest.yaml")).unwrap();
    assert!(!ingest.contains("credential_target") && !ingest.contains("enabled:"), "{ingest}");
    assert!(!ingest.contains("vhl:"), "a friend with no VHL course gets no VHL block");
}
```

In `app/tests/scaffold.rs`, replace

```rust
    // No calendar: the list the engine has always read, empty.
    assert!(ingest_yaml(&base).unwrap().contains("calendars: []\n"));
    // One: the shape `calfeed::load_calendar_events` parses — a list of {name, ics_url} mappings.
    let mut with = base.clone();
    with.personal_calendar = Some("https://calendar.google.com/calendar/ical/x/private-def/basic.ics".into());
    let text = ingest_yaml(&with).unwrap();
    assert!(
        text.contains("calendars:\n  - name: personal\n    ics_url: 'https://calendar.google.com/calendar/ical/x/private-def/basic.ics'\n"),
        "{text}"
    );
```

with

```rust
    // No calendar: the list the engine has always read, empty.
    assert!(ingest_yaml(&base).unwrap().contains("calendars: []\n"));
    // One: the shape `calfeed::load_calendar_events` parses — a list of {name, ics_url} mappings — in
    // this computer's `config/device.yaml`, since a raw address never enters a shared file (D17).
    let mut with = base.clone();
    with.personal_calendar = Some("https://calendar.google.com/calendar/ical/x/private-def/basic.ics".into());
    assert!(ingest_yaml(&with).unwrap().contains("calendars: []\n"), "the shared list keeps no address");
    let text = device_yaml(&with).unwrap();
    assert!(
        text.contains("calendars:\n  - name: 'personal'\n    ics_url: 'https://calendar.google.com/calendar/ical/x/private-def/basic.ics'\n"),
        "{text}"
    );
```

In `app/tests/scaffold.rs`, replace

```rust
    let mut bad = base.clone();
    bad.personal_calendar = Some("https://a\nb".into());
    assert!(ingest_yaml(&bad).unwrap_err().contains("personal calendar address"));
}
```

with

```rust
    let mut bad = base.clone();
    bad.personal_calendar = Some("https://a\nb".into());
    assert!(device_yaml(&bad).unwrap_err().contains("calendar address"));
}
```

In `app/tests/scaffold.rs`, replace

```rust
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(!yaml.contains("secret-capability"), "{yaml}");
    assert!(!yaml.contains("private-abc"), "{yaml}");
    assert!(yaml.contains("ics_url: ''"), "the key stays, empty, so `ingest` still parses it: {yaml}");
    assert!(yaml.contains("- name: personal\n    ics_url: 'cloud:personal'"), "{yaml}");
}
```

with

```rust
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    let device = device_yaml(&plan).expect("device.yaml");
    for text in [&yaml, &device] {
        assert!(!text.contains("secret-capability"), "{text}");
        assert!(!text.contains("private-abc"), "{text}");
    }
    assert!(device.contains("ics_url: ''"), "the key stays, empty, in this computer's file: {device}");
    assert!(!yaml.contains("ics_url: ''"), "and is not a shared value: {yaml}");
    assert!(yaml.contains("- name: personal\n    ics_url: 'cloud:personal'"), "{yaml}");
    assert_eq!(knowlu_engine::config::computer_only_value("config/ingest.yaml", &yaml), Ok(None), "the file may travel as born");
}
```

In `app/tests/scaffold.rs`, replace

```rust
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(yaml.contains("secret-capability"), "{yaml}");
    assert!(yaml.contains("private-abc"), "{yaml}");
}
```

with

```rust
    plan.ics_url = Some("https://lms.example.invalid/feed/secret-capability.ics".into());
    plan.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    // In this computer's own file (two-desktop design D17): never in a shared one.
    let device = device_yaml(&plan).expect("device.yaml");
    assert!(device.contains("secret-capability"), "{device}");
    assert!(device.contains("private-abc"), "{device}");
    let yaml = ingest_yaml(&plan).expect("ingest.yaml");
    assert!(!yaml.contains("secret-capability") && !yaml.contains("private-abc"), "{yaml}");
}

/// Two-desktop design D17: the wizard births a split vault — `config/device.yaml` in D17's literal
/// shape, and six shared files the push guard lets travel as they are.
#[test]
fn a_new_vault_is_born_split_and_every_shared_file_may_travel() {
    let v = temp("born-split").join("Vault");
    let mut p = plan_for(&v);
    p.device = "EXAMPLE-PC".into();
    p.zybooks = true;
    p.google_calendar = true;
    p.personal_calendar = Some("https://calendar.example.invalid/private-abc/basic.ics".into());
    create_vault(&v, &p).unwrap();
    let target = knowlu::credentials::target_for(&p.profile_id, "zybooks");
    assert_eq!(
        std::fs::read_to_string(v.join("config").join("device.yaml")).unwrap().replace("\r\n", "\n"),
        format!("coursework:\n  zybooks:\n    enabled: true\n    credential_target: '{target}'\n  vhl: {{}}\nics_url: ''\ncalendars: []\nrunner:\n  device: 'EXAMPLE-PC'\n  scheduler: app\n")
    );
    for rel in knowlu_engine::config::SHARED_CONFIG {
        let text = knowlu_engine::pystr::read_text(&v.join(rel)).unwrap();
        assert_eq!(knowlu_engine::config::computer_only_value(rel, &text), Ok(None), "{rel}: {text}");
    }
    assert!(knowlu_engine::config::move_device_keys(&v).is_empty(), "nothing left to migrate");
}
```

In `app/tests/onboarding.rs`, replace

```rust
}

/// Task 11 review, I1 (`R-C3'-exec-40`), narrowed by the re-review's N1 (`R-C3'-exec-41`): a feed's
/// save at paste or capture time is a `note`, never an error, so a save that failed there must not
```

with

```rust
}

/// The vault's `config/device.yaml` (two-desktop design D17): where a feed kept on this computer lives.
#[cfg(windows)]
fn read_device(home: &Path) -> String {
    knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("device.yaml")).unwrap()
}

/// Task 11 review, I1 (`R-C3'-exec-40`), narrowed by the re-review's N1 (`R-C3'-exec-41`): a feed's
/// save at paste or capture time is a `note`, never an error, so a save that failed there must not
```

In `app/tests/onboarding.rs`, replace

```rust
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: ''\n"), "the retry landed, so the link is in the account and the vault stays empty: {ingest}");
    let seen = handle.join().expect("the loopback thread did not panic");
    assert!(seen[0].starts_with("PUT /functions/v1/account/sources ") && seen[0].contains("\"kind\":\"lms_ics\""), "{}", seen[0]);
```

with

```rust
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let device = read_device(&home);
    assert!(device.contains("ics_url: ''\n"), "the retry landed, so the link is in the account and the vault stays empty: {device}");
    assert!(!read_ingest(&home).contains("ics_url"), "never a shared value");
    let seen = handle.join().expect("the loopback thread did not panic");
    assert!(seen[0].starts_with("PUT /functions/v1/account/sources ") && seen[0].contains("\"kind\":\"lms_ics\""), "{}", seen[0]);
```

In `app/tests/onboarding.rs`, replace

```rust
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: 'https://x.invalid/b.ics'\n"), "the feed is never lost, even when the account never answers: {ingest}");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
```

with

```rust
    let id = out["profile"]["id"].as_str().expect("a profile id").to_string();
    session.expect_move_to(&id);
    let device = read_device(&home);
    assert!(device.contains("ics_url: 'https://x.invalid/b.ics'\n"), "the feed is never lost, even when the account never answers: {device}");
    assert!(!read_ingest(&home).contains("x.invalid"), "and it stays on this computer (D17)");
    handle.join().expect("the loopback thread did not panic");
    let _ = std::fs::remove_dir_all(&root);
```

In `app/tests/onboarding.rs`, replace

```rust
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "the only call is the restore's pull, never a second PUT: {}", seen[0]);
    assert_eq!(out["restored"]["ok"], true, "{out}");
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: ''\n"), "a link the account holds never reaches the vault: {ingest}");
    let _ = std::fs::remove_dir_all(&root);
}
```

with

```rust
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "the only call is the restore's pull, never a second PUT: {}", seen[0]);
    assert_eq!(out["restored"]["ok"], true, "{out}");
    let device = read_device(&home);
    assert!(device.contains("ics_url: ''\n"), "a link the account holds never reaches the vault: {device}");
    let _ = std::fs::remove_dir_all(&root);
}
```

In `app/tests/onboarding.rs`, replace

```rust
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "a rejected link is never PUT: {}", seen[0]);
    let ingest = read_ingest(&home);
    assert!(ingest.contains("ics_url: 'https://x.invalid/rejected.ics'\n"), "{ingest}");
    let _ = std::fs::remove_dir_all(&root);
}
```

with

```rust
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(seen[0].starts_with("GET /functions/v1/sync-pull"), "a rejected link is never PUT: {}", seen[0]);
    let device = read_device(&home);
    assert!(device.contains("ics_url: 'https://x.invalid/rejected.ics'\n"), "{device}");
    let _ = std::fs::remove_dir_all(&root);
}
```

In `app/tests/onboarding.rs`, replace

```rust
    // into the vault — `create_vault_in` itself is what this test now exercises end to end, not
    // `https_from_webcal` called a second time beside it.
    let ingest = knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap();
    assert!(
        ingest.contains("calendars:\n  - name: personal\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{ingest}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
```

with

```rust
    // into the vault — `create_vault_in` itself is what this test now exercises end to end, not
    // `https_from_webcal` called a second time beside it.
    // Two-desktop design D17: into this computer's own `config/device.yaml`; the shared file keeps
    // no address and no marker the account cannot answer.
    let device = read_device(&home);
    assert!(
        device.contains("calendars:\n  - name: 'personal'\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{device}"
    );
    assert!(read_ingest(&home).contains("calendars: []\n"), "{}", read_ingest(&home));
    let _ = std::fs::remove_dir_all(&root);
}
```

In `app/tests/onboarding.rs`, replace

```rust
    // too, so the fallback writes the trimmed address into the vault and `create_vault_in` is what
    // this test exercises, end to end.
    let ingest = knowlu_engine::pystr::read_text(&home.join("Knowlu").join("Fall 2026").join("config").join("ingest.yaml")).unwrap();
    assert!(
        ingest.contains("calendars:\n  - name: personal\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{ingest}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
```

with

```rust
    // too, so the fallback writes the trimmed address into the vault and `create_vault_in` is what
    // this test exercises, end to end.
    // Two-desktop design D17: into this computer's own `config/device.yaml`; the shared file keeps
    // no address and no marker the account cannot answer.
    let device = read_device(&home);
    assert!(
        device.contains("calendars:\n  - name: 'personal'\n    ics_url: 'https://x.invalid/y.ics'\n"),
        "{device}"
    );
    assert!(read_ingest(&home).contains("calendars: []\n"), "{}", read_ingest(&home));
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu -j 2 --test scaffold --test onboarding`
Expected: FAIL — `scaffold.rs` does not compile (`unresolved import knowlu::scaffold::device_yaml`). Run
`--test onboarding` alone too: its six edited tests FAIL on their assertions (`config/device.yaml` does
not exist yet; the feed and the calendar are still in `config/ingest.yaml`).

- [ ] **Step 3: The split birth and the app's readers**

In `app/src/scaffold.rs`, replace

```rust
}

/// The vault's own settings. **No secret is ever written here** — only the `credential_target`
/// names the engine's `wincred::read_credential` looks up (decision 6).
///
/// `ics_url` is written verbatim (quoted, never rewritten): the engine fetches exactly the string
/// the friend pasted, so any mangling here would be a feed that 404s for a reason nothing explains.
pub fn ingest_yaml(p: &VaultPlan) -> Result<String, String> {
    // m8: a plan carrying confirmed mappings with the source's own flag off would drop them in
```

with

```rust
}

/// The vault's shared settings (two-desktop design D17): the time zone, the course map, the
/// coursework mappings and every calendar that is a `cloud:` marker. **Nothing here belongs to one
/// computer** — each source's `enabled` and `credential_target`, the `ics_url` fallback and a raw
/// calendar address are `device_yaml`'s — so the file can travel to the account as it stands.
pub fn ingest_yaml(p: &VaultPlan) -> Result<String, String> {
    // m8: a plan carrying confirmed mappings with the source's own flag off would drop them in
```

In `app/src/scaffold.rs`, replace

```rust
    }
    let mut s = String::new();
    // C3′, Task 11 (cloud design §9, Alabama SPII): a capability URL is a credential in all but
    // name, and on a vault that has an account the one place it belongs is the account, encrypted,
    // where `PUT /account/sources` already put it. The KEY stays, empty, so `ingest` still parses
    // the file and so a reader can see the feed is elsewhere rather than missing.
    if !p.account_id.is_empty() {
        s.push_str("ics_url: ''\n");
    } else if let Some(u) = &p.ics_url {
        s.push_str(&format!("ics_url: {}\n", yaml_scalar("LMS feed URL", u)?));
    }
    s.push_str(&format!("timezone: {}\n", yaml_scalar("timezone", &p.timezone)?));
    // R-OB-2 and R-C1-48: what the student confirmed, plus two fragments per enrolled course.
```

with

```rust
    }
    let mut s = String::new();
    // Two-desktop design D17: no `ics_url` here at all — the key, empty on an account vault and the
    // pasted address on one with no account, is this computer's own (`device_yaml`).
    s.push_str(&format!("timezone: {}\n", yaml_scalar("timezone", &p.timezone)?));
    // R-OB-2 and R-C1-48: what the student confirmed, plus two fragments per enrolled course.
```

In `app/src/scaffold.rs`, replace

```rust
    // marker for the account's grant, C2's — §11a).
    let mut entries: Vec<String> = Vec::new();
    if p.personal_calendar.is_some() {
        if p.account_id.is_empty() {
            let u = p.personal_calendar.as_deref().unwrap_or_default();
            entries.push(format!("  - name: personal\n    ics_url: {}\n", yaml_scalar("personal calendar address", u)?));
        } else {
            // The same removal and the same argument; `cloud:personal` is C2's own routing
            // (`cli.rs:241-250`) and resolves to `/ingest-calendar?name=personal`.
            entries.push("  - name: personal\n    ics_url: 'cloud:personal'\n".to_string());
        }
    }
    if p.google_calendar {
```

with

```rust
    // marker for the account's grant, C2's — §11a).
    let mut entries: Vec<String> = Vec::new();
    // `cloud:personal` is C2's own routing (`cli.rs:241-250`) and resolves to
    // `/ingest-calendar?name=personal`. A vault with no account keeps the raw address, and D17 puts
    // that in `device_yaml`: a capability URL never enters a shared file.
    if p.personal_calendar.is_some() && !p.account_id.is_empty() {
        entries.push("  - name: personal\n    ics_url: 'cloud:personal'\n".to_string());
    }
    if p.google_calendar {
```

In `app/src/scaffold.rs`, replace

```rust
    }
    if p.zybooks || p.vhl {
        s.push_str("\n# Passwords are NOT here. They live in Windows Credential Manager under the\n");
        s.push_str("# credential_target names below.\ncoursework:\n");
        if p.zybooks {
            let target = yaml_scalar("zybooks credential target", &crate::credentials::target_for(&p.profile_id, "zybooks"))?;
            s.push_str(&format!("  zybooks:\n    enabled: true\n    credential_target: {target}\n"));
            // Review round 1, I2: every code the student declined to map, plus zyBooks' own
            // `HowToUseZyBooks2` (zero assignments, never coursework) — `create_vault_in` is the one
```

with

```rust
    }
    if p.zybooks || p.vhl {
        s.push_str("\n# Passwords are NOT here. Each computer names its own logins in its config/device.yaml.\n");
        s.push_str("coursework:\n");
        if p.zybooks {
            s.push_str("  zybooks:\n");
            // Review round 1, I2: every code the student declined to map, plus zyBooks' own
            // `HowToUseZyBooks2` (zero assignments, never coursework) — `create_vault_in` is the one
```

In `app/src/scaffold.rs`, replace

```rust
            }
            // What `parse_assignments` reads. Without these three blocks every item is uncategorised
            // and takes the default effort, which is the second half of the first-slot failure.
            s.push_str("    categories:\n      HW: hw\n      Lab: lab\n      Project: project\n");
            s.push_str("    effort:\n      minutes_per_section: 6\n      floors:\n        hw: 0.25\n        lab: 0.5\n        project: 1.0\n");
            s.push_str("    importance:\n      hw: 2\n      lab: 2\n      project: 2\n");
            if p.zybooks_courses.is_empty() {
                s.push_str("    courses: {}\n");
```

with

```rust
            }
            // What `parse_assignments` reads. Without these three blocks every item is uncategorised
            // and takes the default effort, which is the second half of the first-slot failure. The
            // engine's own copy, so a block `insert_mapping` creates on a second computer matches.
            for line in knowlu_engine::coursework::ZYBOOKS_SHARED_DEFAULTS.lines() {
                s.push_str(&format!("    {line}\n"));
            }
            if p.zybooks_courses.is_empty() {
                s.push_str("    courses: {}\n");
```

In `app/src/scaffold.rs`, replace

```rust
        }
        if p.vhl {
            let target = yaml_scalar("vhl credential target", &crate::credentials::target_for(&p.profile_id, "vhl"))?;
            s.push_str(&format!("  vhl:\n    enabled: true\n    credential_target: {target}\n"));
            s.push_str("    importance: 3\n");
            if p.vhl_sections.is_empty() {
                s.push_str("    sections: {}\n");
```

with

```rust
        }
        if p.vhl {
            s.push_str("  vhl:\n");
            for line in knowlu_engine::coursework::VHL_SHARED_DEFAULTS.lines() {
                s.push_str(&format!("    {line}\n"));
            }
            if p.vhl_sections.is_empty() {
                s.push_str("    sections: {}\n");
```

In `app/src/scaffold.rs`, replace

```rust
}

/// Task 11 review, I1 (`R-C3'-exec-40`): the account save `lms_link::finish` attempts at paste or
/// capture time is a `note`, never an error, so a save that failed used to leave the feed in
```

with

```rust
}

/// `config/device.yaml` (two-desktop design D17): what belongs to this computer alone — which portal
/// logins it holds (`enabled: true` and the `credential_target` Credential Manager keeps them under),
/// the vault-held `ics_url` fallback (`''` on an account vault, whose feed is the account's), a raw
/// personal calendar address on a vault with no account, and the `local` runner's `device` and
/// `scheduler: app` (decision 4: from birth, on the machine that made the vault). Through the
/// engine's own writer, so the wizard and the migration write one shape.
pub fn device_yaml(p: &VaultPlan) -> Result<String, String> {
    use knowlu_engine::config::{DeviceKeys, SourceKeys};
    let mut keys = DeviceKeys::default();
    for (source, on) in [("zybooks", p.zybooks), ("vhl", p.vhl)] {
        if on {
            keys.sources.insert(
                source.to_string(),
                SourceKeys {
                    enabled: Some(true),
                    credential_target: Some(crate::credentials::target_for(&p.profile_id, source)),
                    base_url: None,
                },
            );
        }
    }
    if p.account_id.is_empty() {
        keys.ics_url = p.ics_url.clone().unwrap_or_default();
        if let Some(u) = &p.personal_calendar {
            keys.calendars.push(("personal".to_string(), u.clone()));
        }
    }
    keys.device = Some(p.device.clone());
    keys.scheduler = Some("app".to_string());
    knowlu_engine::config::device_yaml(&keys)
}

/// Task 11 review, I1 (`R-C3'-exec-40`): the account save `lms_link::finish` attempts at paste or
/// capture time is a `note`, never an error, so a save that failed used to leave the feed in
```

In `app/src/scaffold.rs`, replace

```rust
/// call exists to replace — both are a plan or a call-site bug, never a student's, so they fail
/// loudly rather than silently doing nothing.
pub fn restore_capability_url(vault: &Path, kind: &str, url: &str) -> Result<(), String> {
    let path = vault.join("config").join("ingest.yaml");
    let text = knowlu_engine::pystr::read_text(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let patched = match kind {
        "lms_ics" => {
            let scalar = yaml_scalar("LMS feed URL", url)?;
            if !text.contains("ics_url: ''\n") {
                return Err(format!("{}: no blanked LMS feed line to restore", path.display()));
            }
            text.replacen("ics_url: ''\n", &format!("ics_url: {scalar}\n"), 1)
        }
        "calendar_ics" => {
            let scalar = yaml_scalar("personal calendar address", url)?;
            let blanked = "- name: personal\n    ics_url: 'cloud:personal'\n";
            if !text.contains(blanked) {
                return Err(format!("{}: no blanked personal-calendar line to restore", path.display()));
            }
            text.replacen(blanked, &format!("- name: personal\n    ics_url: {scalar}\n"), 1)
        }
        other => return Err(format!("{other}: not a capability url this restores")),
    };
    knowlu_engine::pystr::write_text(&path, &patched).map_err(|e| format!("{}: {e}", path.display()))
}

/// Decision 4: `scheduler: app` and `device:` from birth. `grace_minutes: 20` is the local
/// runner's own number, unchanged; `cloud` is deliberately absent — a friend has no cloud runner,
/// and an expected-but-never-seen runner would paint every Runs view amber forever.
pub fn runners_yaml(p: &VaultPlan) -> Result<String, String> {
    let mut times = Vec::with_capacity(p.slots.len());
```

with

```rust
/// call exists to replace — both are a plan or a call-site bug, never a student's, so they fail
/// loudly rather than silently doing nothing.
///
/// **Two-desktop design D17: into `config/device.yaml`, never a shared file** — a capability URL never
/// enters a synced file. The LMS feed replaces `device.yaml`'s blank `ics_url`; the personal calendar
/// leaves `config/ingest.yaml`'s `cloud:personal` entry (the account never got the address, so the
/// marker would only fail) and joins `device.yaml`'s `calendars:`.
pub fn restore_capability_url(vault: &Path, kind: &str, url: &str) -> Result<(), String> {
    match kind {
        "lms_ics" => {
            yaml_scalar("LMS feed URL", url)?;
            let device = knowlu_engine::config::load_device(vault);
            if knowlu_engine::yaml::get(&device, "ics_url").and_then(|v| v.as_str()) != Some("") {
                return Err(format!("{}: no blanked LMS feed line to restore", vault.join(knowlu_engine::config::DEVICE_FILE).display()));
            }
            knowlu_engine::config::set_device_ics_url(vault, url, true).map(|_| ())
        }
        "calendar_ics" => {
            yaml_scalar("personal calendar address", url)?;
            let path = vault.join("config").join("ingest.yaml");
            let text = knowlu_engine::pystr::read_text(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let blanked = "  - name: personal\n    ics_url: 'cloud:personal'\n";
            if !text.contains(blanked) {
                return Err(format!("{}: no blanked personal-calendar line to restore", path.display()));
            }
            let mut patched = text.replacen(blanked, "", 1);
            // The last entry gone, the shared list keeps its shape as an empty one.
            if patched.contains("calendars:\n") && !patched.contains("calendars:\n  - ") {
                patched = patched.replacen("calendars:\n", "calendars: []\n", 1);
            }
            knowlu_engine::config::add_device_calendar(vault, "personal", url)?;
            knowlu_engine::pystr::write_text(&path, &patched).map_err(|e| format!("{}: {e}", path.display()))
        }
        other => Err(format!("{other}: not a capability url this restores")),
    }
}

/// The shared slot times, `tz` and `grace_minutes` (two-desktop design D17). `grace_minutes: 20` is
/// the local runner's own number, unchanged; `cloud` is deliberately absent — a friend has no cloud
/// runner, and an expected-but-never-seen runner would paint every Runs view amber forever. Decision
/// 4's `scheduler: app` and `device:` are this computer's, in `device_yaml`.
pub fn runners_yaml(p: &VaultPlan) -> Result<String, String> {
    let mut times = Vec::with_capacity(p.slots.len());
```

In `app/src/scaffold.rs`, replace

```rust
    }
    Ok(format!(
        "runners:\n  - name: local\n    times: [{}]\n    tz: {}\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n",
        times.join(", "),
        yaml_scalar("timezone", &p.timezone)?,
        yaml_scalar("device name", &p.device)?,
    ))
}
```

with

```rust
    }
    Ok(format!(
        "runners:\n  - name: local\n    times: [{}]\n    tz: {}\n    grace_minutes: 20\n",
        times.join(", "),
        yaml_scalar("timezone", &p.timezone)?,
    ))
}
```

In `app/src/scaffold.rs`, replace

```rust
    }
    write_file(root, "config/runners.yaml", &runners_yaml(plan)?)?;
    write_file(root, "config/cloud.yaml", &cloud_yaml(plan)?)?;
    Ok(())
```

with

```rust
    }
    write_file(root, "config/runners.yaml", &runners_yaml(plan)?)?;
    // Two-desktop design D17: this computer's own keys, born split — a new file, from literals.
    write_file(root, knowlu_engine::config::DEVICE_FILE, &device_yaml(plan)?)?;
    write_file(root, "config/cloud.yaml", &cloud_yaml(plan)?)?;
    Ok(())
```

In `app/src/scheduler.rs`, replace

```rust
    let path = vault.join("config").join("ingest.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else { return IcsState::NoUrl };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return IcsState::Unreadable };
    match v.get("ics_url").and_then(|u| u.as_str()) {
        Some(s) if !s.trim().is_empty() => IcsState::Feed,
```

with

```rust
    let path = vault.join("config").join("ingest.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else { return IcsState::NoUrl };
    let Ok(mut v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return IcsState::Unreadable };
    // Two-desktop design D17: the vault-held fallback is this computer's own, in `config/device.yaml`
    // first — the engine's own reader rule, so the slot and `ingest` agree on whether there is a feed.
    if let serde_yaml_ng::Value::Mapping(map) = &mut v {
        knowlu_engine::config::overlay_device(vault, map);
    }
    match v.get("ics_url").and_then(|u| u.as_str()) {
        Some(s) if !s.trim().is_empty() => IcsState::Feed,
```

In `app/src/scheduler.rs`, replace

```rust
}

/// `Script` unless `config/runners.yaml`'s `local` entry says `scheduler: app` — absent means
/// script, and the live vault has no such key throughout this plan.
pub fn mode(vault: &Path) -> SchedulerMode {
    knowlu_engine::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").scheduler
}
```

with

```rust
}

/// `Script` unless the `local` runner says `scheduler: app` — `config/device.yaml`'s `runner:` first,
/// `config/runners.yaml`'s `local` entry for a vault not yet split (two-desktop design D17). Absent
/// means script.
pub fn mode(vault: &Path) -> SchedulerMode {
    knowlu_engine::runs::local_runner_settings(vault).scheduler
}
```

In `app/src/scheduler.rs`, replace

```rust
/// against a shared vault never double-runs the same slots.
pub fn device_ok(vault: &Path) -> bool {
    match knowlu_engine::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").device {
        None => true,
        Some(d) => knowlu_engine::journal::device_name().eq_ignore_ascii_case(&d),
```

with

```rust
/// against a shared vault never double-runs the same slots.
pub fn device_ok(vault: &Path) -> bool {
    match knowlu_engine::runs::local_runner_settings(vault).device {
        None => true,
        Some(d) => knowlu_engine::journal::device_name().eq_ignore_ascii_case(&d),
```

- [ ] **Step 4: Run the tests**

Run the command of Step 2 (Global Constraint 19's order first — the engine exe must be rebuilt after the
app). Expected: PASS — `scaffold.rs` 27, `onboarding.rs` 36. Then `--test scheduler`: PASS, unchanged (its
vaults write `runners.yaml` with `device`/`scheduler`, which `local_runner_settings` still reads when
`device.yaml` is absent).

- [ ] **Step 5: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings (the accepted `.rsrc` line only).

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-p2-task-7.txt`:

```
feat: the wizard births a split vault; the app reads its own keys first (two desktops, D17)

A new vault's config/device.yaml carries this computer's logins, the LMS
feed fallback, any raw calendar address and the local runner's device and
scheduler; ingest.yaml and runners.yaml carry only shared settings, so
every shared file may travel as born. restore_capability_url writes the
address into device.yaml, never a shared file. The scheduler reads the
runner's keys and the feed fallback device-first.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add app/src/scaffold.rs app/src/scheduler.rs app/tests/scaffold.rs app/tests/onboarding.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-7.txt`

---

### Task 8: D19 — the second computer's wizard (spec §4.8 D19, §6.2)

**Files:**
- Modify: `app/src/onboarding.rs` — `WizardPlan` (`:431-487`) gains `account_holds_notes`;
  `create_vault_in` (`:577-760`) restores through `restore_into_as` and then puts the wizard's own
  mapping rows back by `coursework::insert_mapping` (R-TD2-15); between `create_vault_in` and
  `create_vault` (`:762`): `account_settings_exist_at` and the command `account_settings_exist`.
- Modify: `app/static/console.js` — `WIZ.held`; `afterSignIn` (called by both sign-in paths instead of
  `wizGo(2)`); `HELD_PANELS`, `skipped` and `wizGo`'s step-over loop; the discovery rows filtered by the
  account's `mapped`; the plan's `account_holds_notes`; the finish summary.
- Modify: `app/tests/onboarding.rs` — `base_plan` and the `WizardPlan` literal in
  `a_vault_that_cannot_be_finished_is_removed_and_nothing_is_registered` name the new field; three new
  tests.
- Hand-offs: **H3** (`app/src/main.rs`: the wizard window registers `onboarding::account_settings_exist`),
  **H4** (`scripts/wizard-check.py`: the command is a read, and the two D19 walks), **H5** (`CLAUDE.md`),
  applied by the controller at Step 4.

**Interfaces:**
- Consumes: `sync::{account_settings, restore_into_as, load_cursor, sha256_hex, SHARED_CONFIG}` (Tasks 5–6),
  `config::mapped_sources` (Task 1), `coursework::insert_mapping` (Task 5), `scaffold::SectionMapping`,
  `account::{api_base, auth_base, anon_key, check_api_base, valid_access_token_at, PENDING_TARGET}`.
- Produces: `onboarding::{account_settings_exist, account_settings_exist_at}`,
  `onboarding::WizardPlan::account_holds_notes`; the wizard window's command count **29 → 30** (the
  console's stays 42; distinct **61 → 62**, recounted by script over the two `generate_handler!` lists).

**The three walks** (D19). After either sign-in succeeds, the page asks `account_settings_exist` once.
*Settings exist* (a `config/ingest.yaml` row in the account): `wizGo` steps over the name, the calendars
panel (school, LMS capture, calendars), Gmail, slots and time zone, and the subscription when
`entitlement_now` says active (R-TD2-14) — sign-in → logins → Finish, Back from Finish to the logins —
and the discovery rows are only the sources the account's `ingest.yaml` does not map. Finish births a
split vault of defaults (Task 7), the restore writes the account's settings over them (Task 6,
R-TD2-17), and the confirmed rows go back in (R-TD2-15). *Notes but no settings*: every panel is asked,
and the plan's `account_holds_notes: true` makes Finish mark the answers provisional (Task 6), even when
the restore could not read the account. *No vault*: `account_settings_exist` answers no to both, the walk
is today's nine panels, and nothing is marked. A failed read (`ok: false`) is the third walk: the wizard
asks everything, and Finish's restore still brings whatever the account holds.

- [ ] **Step 1: Write the failing tests**

In `app/tests/onboarding.rs`, replace

```rust
        courses: Vec::new(),
        zybooks_ignore: Vec::new(),
    };
    // A vault, scaffolded directly through `scaffold::create_vault` — this test is about
```

with

```rust
        courses: Vec::new(),
        zybooks_ignore: Vec::new(),
        account_holds_notes: false,
    };
    // A vault, scaffolded directly through `scaffold::create_vault` — this test is about
```

In `app/tests/onboarding.rs`, replace

```rust
        autostart: true,
        offer_inference,
    }
}
```

with

```rust
        autostart: true,
        offer_inference,
        account_holds_notes: false,
    }
}
```

In `app/tests/onboarding.rs`, replace

```rust
    assert_eq!(out["ok"], true, "an ordinary vault must still be adoptable: {out}");
    let _ = std::fs::remove_dir_all(&root);
}
```

with

```rust
    assert_eq!(out["ok"], true, "an ordinary vault must still be adoptable: {out}");
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------------------------
// Two-desktop design D19: the second computer's wizard.
// ---------------------------------------------------------------------------------------------

/// The account's `config/ingest.yaml` in the D19 tests: zyBooks mapped by the first computer, VHL not.
#[cfg(windows)]
const ACCOUNT_INGEST: &str = "timezone: 'America/New_York'\ncourse_map: {}\ncalendars: []\ncoursework:\n  zybooks:\n    ignore: []\n    courses:\n      'UACS100Fall2026':\n        course: 'cs-100'\n        label: 'CS 100'\n";

/// One `/sync-pull` page carrying `settings` rows and, when `with_note`, one note.
#[cfg(windows)]
fn account_page(settings: &[(&str, &str)], with_note: bool) -> String {
    let mut notes: Vec<serde_json::Value> = settings
        .iter()
        .map(|(path, body)| serde_json::json!({ "rev": 1, "device": "fedcba9876543210", "path": path, "deleted": false, "body": body }))
        .collect();
    if with_note {
        notes.push(serde_json::json!({ "rev": 2, "device": "fedcba9876543210", "path": "tasks/from-the-laptop.md", "deleted": false,
            "body": "---\nid: task_0000000099\ntitle: From the laptop\n---\n" }));
    }
    knowlu_engine::ledger::dumps_value(&serde_json::json!({ "records": [], "notes": notes, "record_cursor": 0, "note_cursor": 2, "more": false }))
}

#[cfg(windows)]
fn vault_cursor(home: &Path) -> knowlu_engine::sync::Cursor {
    knowlu_engine::sync::load_cursor(&home.join("Knowlu").join("Fall 2026"))
}

/// `account_settings_exist`'s answer: settings only when the account holds `config/ingest.yaml`,
/// whether it holds notes, and which portal sources its settings already map.
#[cfg(windows)]
#[test]
fn account_settings_exist_answers_settings_notes_and_the_mapped_sources() {
    let body = knowlu_engine::ledger::dumps_value(&serde_json::json!({
        "settings": [{ "path": "config/ingest.yaml", "body": ACCOUNT_INGEST }], "notes": true,
    }));
    let (base, handle) = restore_loopback(&body);
    let out = knowlu::onboarding::account_settings_exist_at(&base, "jwt-not-a-secret");
    handle.join().expect("the loopback thread did not panic");
    assert_eq!(out, serde_json::json!({ "ok": true, "error": null, "settings": true, "notes": true, "mapped": ["zybooks"] }));
    let (base, handle) = restore_loopback(r#"{"settings":[],"notes":true}"#);
    let out = knowlu::onboarding::account_settings_exist_at(&base, "jwt-not-a-secret");
    handle.join().expect("the loopback thread did not panic");
    assert_eq!((out["settings"].clone(), out["notes"].clone()), (serde_json::json!(false), serde_json::json!(true)), "notes but no settings: {out}");
    let closed = knowlu::onboarding::account_settings_exist_at("http://127.0.0.1:9/functions/v1", "jwt-not-a-secret");
    assert_eq!((closed["ok"].clone(), closed["settings"].clone()), (serde_json::json!(false), serde_json::json!(false)), "{closed}");
}

/// The settings-exist walk's Finish: the account's settings replace the skipped panels' defaults, the
/// VHL row confirmed for a portal the account does not map goes back in (re-review r6), this
/// computer's own login is in `config/device.yaml`, and nothing is provisional.
#[cfg(windows)]
#[test]
fn a_second_computer_takes_the_accounts_settings_and_keeps_its_own_logins_route() {
    let root = tmp("d19-settings");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-d19-settings");
    let (base, handle) = restore_loopback(&account_page(&[("config/ingest.yaml", ACCOUNT_INGEST)], true));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let mut plan = base_plan(false);
    plan.vhl = true;
    plan.vhl_sections = vec![knowlu::scaffold::SectionMapping { section: "2102121".into(), course: "GN 103".into(), label: "GN 103".into() }];
    plan.account_holds_notes = true;
    let out = create_vault_in(&app_data, &home, "Fall 2026", &plan);
    handle.join().expect("the loopback thread did not panic");
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().unwrap());
    let ingest = read_ingest(&home);
    assert!(ingest.starts_with(ACCOUNT_INGEST), "the account's text, whole: {ingest}");
    assert!(ingest.contains("  vhl:\n    importance: 3\n    sections:\n      \"2102121\":\n        course: \"gn-103\"\n        label: \"GN 103\"\n"), "{ingest}");
    assert!(read_device(&home).contains("  vhl:\n    enabled: true\n"), "this computer's own login");
    let cursor = vault_cursor(&home);
    assert!(cursor.provisional_config.is_empty(), "the account's settings arrived: nothing is provisional");
    assert_eq!(cursor.notes["config/ingest.yaml"], knowlu_engine::sync::sha256_hex(ACCOUNT_INGEST.as_bytes()), "the base the first push starts from");
    let _ = std::fs::remove_dir_all(&root);
}

/// Notes in the account and no settings yet: the wizard asked every question, and Finish marks every
/// shared file provisional — also when the restore could not read the account at all.
#[cfg(windows)]
#[test]
fn notes_but_no_settings_mark_the_new_computers_answers_provisional() {
    let root = tmp("d19-provisional");
    let home = root.join("home");
    let app_data = root.join("appdata");
    let mut session = PendingSession::new("acc-d19-provisional");
    let (base, handle) = restore_loopback(&account_page(&[], true));
    unsafe { std::env::set_var("KNOWLU_API_BASE", &base) };
    let out = create_vault_in(&app_data, &home, "Fall 2026", &base_plan(false));
    handle.join().expect("the loopback thread did not panic");
    assert_eq!(out["ok"], true, "{out}");
    session.expect_move_to(out["profile"]["id"].as_str().unwrap());
    assert_eq!(vault_cursor(&home).provisional_config, knowlu_engine::sync::SHARED_CONFIG.to_vec());
    drop(session);

    let root2 = tmp("d19-provisional-unread");
    let (home2, app_data2) = (root2.join("home"), root2.join("appdata"));
    let mut session = PendingSession::new("acc-d19-unread");
    let mut plan = base_plan(false);
    plan.account_holds_notes = true;
    let out = create_vault_in(&app_data2, &home2, "Fall 2026", &plan);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["restored"]["ok"], false, "the closed loopback: the account could not be read");
    session.expect_move_to(out["profile"]["id"].as_str().unwrap());
    assert_eq!(vault_cursor(&home2).provisional_config, knowlu_engine::sync::SHARED_CONFIG.to_vec(), "the wizard's answer still marks it");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&root2);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu -j 2 --test onboarding`
Expected: FAIL to compile — `WizardPlan` has no field `account_holds_notes` (the two struct literals
and the two D19 tests that set it), and `cannot find function account_settings_exist_at in module
knowlu::onboarding`.

- [ ] **Step 3: The command, the restore's rows, the page**

In `app/src/onboarding.rs`, replace

```rust
    #[serde(default)]
    pub zybooks_ignore: Vec<String>,
}
```

with

```rust
    #[serde(default)]
    pub zybooks_ignore: Vec<String>,
    /// Two-desktop design D19: `account_settings_exist` saw notes in the account. With no settings
    /// arriving at Finish, this computer's shared files are then provisional (`sync::restore_into_as`)
    /// even when the restore itself could not read the account. `#[serde(default)]`: an older page
    /// that never asked is an account with nothing in it, as before.
    #[serde(default)]
    pub account_holds_notes: bool,
}
```

In `app/src/onboarding.rs`, replace

```rust
    // which is exactly why `restore_into` computes its allowlist from what is on disk right now
    // rather than demanding an empty folder.
    let restored = match knowlu_engine::sync::restore_into(&dest) {
        // **An empty copy keeps the vault.** A student signing in on their first desktop, or on a
        // second one before the first has ever pushed, has made a perfectly good vault; rolling it
```

with

```rust
    // which is exactly why `restore_into` computes its allowlist from what is on disk right now
    // rather than demanding an empty folder.
    let restored = match knowlu_engine::sync::restore_into_as(&dest, plan.account_holds_notes) {
        // **An empty copy keeps the vault.** A student signing in on their first desktop, or on a
        // second one before the first has ever pushed, has made a perfectly good vault; rolling it
```

In `app/src/onboarding.rs`, replace

```rust
        }
    };
    // `finish_or_roll_back`'s envelope gains one key here, rather than inside that function, because
    // its OTHER two callers — the backup-folder restore and the adopted-vault path — never call
```

with

```rust
        }
    };
    // Two-desktop design D19 (re-review r6): the rows this wizard confirmed are for a portal the
    // account's settings do not map yet (the page shows no others). The restore above may have
    // replaced `config/ingest.yaml` with the account's own; each row goes back in by line insertion,
    // a key the file already holds left alone, so a portal first set up on this computer still gets
    // its route — and reaches the account at the first sync, from the base the restore recorded.
    // Where no settings came down the rows are already in the file and nothing changes.
    let mut restored = restored;
    let rows = vp.zybooks_courses.iter().map(|b| ("zybooks", &b.code, &b.course, &b.label))
        .chain(vp.vhl_sections.iter().map(|v| ("vhl", &v.section, &v.course, &v.label)));
    for (source, key, course, label) in rows {
        if let Err(e) = knowlu_engine::coursework::insert_mapping(&dest, source, key, course, label) {
            restored.warnings.push(format!("setup: the {source} mapping of {key} could not be written ({e})"));
        }
    }
    // `finish_or_roll_back`'s envelope gains one key here, rather than inside that function, because
    // its OTHER two callers — the backup-folder restore and the adopted-vault path — never call
```

In `app/src/onboarding.rs`, replace

```rust
        });
    }
    out
}

#[tauri::command(async)]
```

with

```rust
        });
    }
    out
}

/// Two-desktop design D19, the handle-free core of [`account_settings_exist`]: one
/// `GET /sync-pull?view=settings` with `token`. `settings` is yes only when the account holds a
/// `config/ingest.yaml` row; `notes` is whether it holds any note; `mapped` names the portal sources
/// whose mappings the account's `ingest.yaml` already carries (`config::mapped_sources`), so the
/// logins panel shows mapping rows only for the others. A failure is `ok: false` and reads as "no
/// settings": the wizard then asks every question, and Finish's restore still brings whatever the
/// account holds.
pub fn account_settings_exist_at(api_base: &str, token: &str) -> Value {
    let refused = |e: String| json!({ "ok": false, "error": e, "settings": false, "notes": false, "mapped": [] });
    if let Err(e) = crate::account::check_api_base(api_base) {
        return refused(e);
    }
    let cfg = knowlu_engine::cloudmodel::CloudConfig {
        api_base: api_base.to_string(),
        anon_key: crate::account::anon_key(),
        session_credential_target: crate::account::PENDING_TARGET.to_string(),
        account_id: String::new(),
    };
    let client = knowlu_engine::cloudmodel::CloudClient::new(&cfg, token);
    match knowlu_engine::sync::account_settings(&client) {
        Ok(held) => {
            let ingest = held.texts.get("config/ingest.yaml");
            json!({
                "ok": true, "error": Value::Null,
                "settings": ingest.is_some(),
                "notes": held.notes,
                "mapped": ingest.map(|t| knowlu_engine::config::mapped_sources(t)).unwrap_or_default(),
            })
        }
        Err(e) => refused(e.label()),
    }
}

/// Two-desktop design D19: after sign-in, does the account already hold settings? The wizard window's
/// thirtieth command. Reads the PENDING session, as every wizard call before Finish does; writes
/// nothing, on this machine or the account.
#[tauri::command(async)]
pub fn account_settings_exist() -> Value {
    let base = crate::account::api_base();
    let token = crate::account::auth_base(&base).and_then(|auth| {
        crate::account::valid_access_token_at(&auth, &crate::account::anon_key(), crate::account::PENDING_TARGET, jiff::Timestamp::now().as_second())
    });
    match token {
        Ok(token) => account_settings_exist_at(&base, &token),
        Err(e) => json!({ "ok": false, "error": e, "settings": false, "notes": false, "mapped": [] }),
    }
}

#[tauri::command(async)]
```

In `app/static/console.js`, replace

```js
              // read here by `renderWizard` like every other wizard field. Blank until then: a
              // sentence claiming an outcome before Finish has even run is the bug this replaces.
              restoreNote: "" };

  // The trim and the trailing-separator strip are not cosmetic. `dest_for` in onboarding.rs trims
```

with

```js
              // read here by `renderWizard` like every other wizard field. Blank until then: a
              // sentence claiming an outcome before Finish has even run is the bug this replaces.
              restoreNote: "",
              // Two-desktop design D19: what `account_settings_exist` answered after sign-in —
              // `{ settings, notes, mapped }`, or null when it could not answer (every panel is asked).
              held: null };

  // The trim and the trailing-separator strip are not cosmetic. `dest_for` in onboarding.rs trims
```

In `app/static/console.js`, replace

```js
    EL("wiz-google-note").textContent = WIZ.googleNote;
    EL("wiz-google").disabled = WIZ.google || WIZ.googlePolling;
    EL("wiz-summary").textContent = dest() + ", looking at " + WIZ.slots.join(" and ") + " " + WIZ.tz + ".";
    // M1 (fix round 1): painted from WIZ, like every other wizard field — blank until `wizFinish`
    // has an actual answer from `restore_into`, never a claim made before Finish has even run.
```

with

```js
    EL("wiz-google-note").textContent = WIZ.googleNote;
    EL("wiz-google").disabled = WIZ.google || WIZ.googlePolling;
    EL("wiz-summary").textContent = (WIZ.held && WIZ.held.settings)
      ? dest() + ", with the settings your account already has."
      : dest() + ", looking at " + WIZ.slots.join(" and ") + " " + WIZ.tz + ".";
    // M1 (fix round 1): painted from WIZ, like every other wizard field — blank until `wizFinish`
    // has an actual answer from `restore_into`, never a claim made before Finish has even run.
```

In `app/static/console.js`, replace

```js
  }

  function wizGo(n) {
    // R-C1b-exec-9: the only thing that ever set WIZ.entitled true used to be the two-minute poll
    // below, and a student who came back to the wizard after that poll had already given up found
```

with

```js
  }

  // Two-desktop design D19: with settings in the account, every panel whose answer is shared or
  // account-held is stepped over — the name (its default stands), the school, the LMS capture and the
  // calendars, Gmail, the slots and the time zone, and the subscription when it is already active —
  // and this computer is asked only for its own logins.
  var HELD_PANELS = ["vault", "calendars", "gmail", "slots"];
  function skipped(i) {
    if (!WIZ.held || !WIZ.held.settings) { return false; }
    return (PANELS[i] === "subscribe" && WIZ.entitled) || HELD_PANELS.indexOf(PANELS[i]) !== -1;
  }

  // D19: one question after sign-in. The subscription is asked about only when the answer can skip it.
  function afterSignIn() {
    WIZ.busy = true; renderWizard();
    return invoke("account_settings_exist", {}).catch(function () { return null; }).then(function (held) {
      WIZ.held = (held && held.ok) ? { settings: !!held.settings, notes: !!held.notes, mapped: held.mapped || [] } : null;
      return (WIZ.held && WIZ.held.settings) ? checkEntitled() : false;
    }).then(function (yes) {
      if (yes) { WIZ.entitled = true; }
      WIZ.busy = false;
      return wizGo(2);
    });
  }

  function wizGo(n) {
    // D19: step over the panels the account already answers, whichever way this press goes.
    var dir = n >= WIZ.step ? 1 : -1;
    while (n > 0 && n < PANELS.length - 1 && skipped(n)) { n += dir; }
    // R-C1b-exec-9: the only thing that ever set WIZ.entitled true used to be the two-minute poll
    // below, and a student who came back to the wizard after that poll had already given up found
```

In `app/static/console.js`, replace

```js
        renderWizard();
        return invoke("discover_coursework", { vault: dest(), zybooks: WIZ.zy, vhl: WIZ.vhl }).then(function (d) {
          WIZ.map = ((d && d.rows) || []).map(function (r) {
            return { source: r.source, key: r.key, detail: r.detail, suggested: r.suggested, course: r.suggested || "", ignore: !!r.ignored };
          });
```

with

```js
        renderWizard();
        return invoke("discover_coursework", { vault: dest(), zybooks: WIZ.zy, vhl: WIZ.vhl }).then(function (d) {
          // D19 (re-review r6): a source the account's settings already map gets its books by card,
          // so only the others get rows here.
          WIZ.map = ((d && d.rows) || []).filter(function (r) {
            return !(WIZ.held && WIZ.held.settings && WIZ.held.mapped.indexOf(r.source) !== -1);
          }).map(function (r) {
            return { source: r.source, key: r.key, detail: r.detail, suggested: r.suggested, course: r.suggested || "", ignore: !!r.ignored };
          });
```

In `app/static/console.js`, replace

```js
                   zybooks_ignore: WIZ.map.filter(function (r) { return r.source === "zybooks" && r.ignore; })
                                          .map(function (r) { return r.key; }),
                   courses: WIZ.courses };
      // Before anything is created: move the credentials if the path has changed since they were
      // written, so Credential Manager and the vault's `credential_target:` lines agree the moment
```

with

```js
                   zybooks_ignore: WIZ.map.filter(function (r) { return r.source === "zybooks" && r.ignore; })
                                          .map(function (r) { return r.key; }),
                   courses: WIZ.courses,
                   // D19: notes in the account and no settings yet make this computer's answers
                   // provisional at Finish (`sync::restore_into_as`).
                   account_holds_notes: !!(WIZ.held && WIZ.held.notes) };
      // Before anything is created: move the credentials if the path has changed since they were
      // written, so Credential Manager and the vault's `credential_target:` lines agree the moment
```

In `app/static/console.js`, replace

```js
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        wizGo(2);
      }).catch(function () { WIZ.busy = false; WIZ.accountNote = ""; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
```

with

```js
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        afterSignIn();
      }).catch(function () { WIZ.busy = false; WIZ.accountNote = ""; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
```

In `app/static/console.js`, replace

```js
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        EL("wiz-code-row").hidden = true;
        wizGo(2);
      }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
```

with

```js
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        EL("wiz-code-row").hidden = true;
        afterSignIn();
      }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
```

- [ ] **Step 4: Hand-offs H3, H4 and H5 are applied**

The controller applies **H3** (`app/src/main.rs`), **H4** (`scripts/wizard-check.py`) and **H5**
(`CLAUDE.md`) verbatim to the working tree. An implementer never edits any of the three. (H3 registers
the command this task created, so it can only land now.)

- [ ] **Step 5: Run the tests**

Run the command of Step 2 (Global Constraint 19's order first — the engine exe must be rebuilt after the
app; Global Constraint 11 if another worktree's `onboarding` run is live). Expected: PASS — 39 in
`onboarding.rs`. Then `--test scaffold --test scheduler --test account`: PASS, unchanged.

- [ ] **Step 6: The wizard walk**

Run (the system Python with Playwright, as for every earlier wizard hand-off):
`python C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\scripts\wizard-check.py`
Expected: one line, `ok` — the nine-panel walk unchanged (the fake answers "nothing in the account"
when a walk sets no `window.__HELD`), `account_settings_exist` counted as a read before Finish, and the
two D19 walks: settings exist (sign-in → logins → Finish and back, only the unmapped VHL section offered,
no skipped panel's command, the plan carrying `account_holds_notes` and the VHL row), and notes but no
settings (every panel shown, the plan carrying `account_holds_notes`).

- [ ] **Step 7: The whole suite**

Global Constraint 19. Expected: PASS at 0 warnings. Recount the commands by script (the count H5 quotes):
the wizard's `generate_handler!` list 30, the console's 42, distinct 62.

- [ ] **Step 8: Commit**

The controller commits H3, H4 and H5 first, as their own commit; then the task, back to back (H3 names a
command this task adds).

Message file `.superpowers\sdd\msg-p2-task-8.txt`:

```
feat: a second computer's wizard asks only for its own logins (two desktops, D19)

After sign-in the wizard asks the account whether it already holds
settings (account_settings_exist, one GET /sync-pull?view=settings). If it
does, every shared panel is stepped over - name, school, LMS, calendars,
Gmail, slots, time zone, and the subscription when active - and mapping
rows are offered only for a portal the account does not map yet; Finish's
restore brings the account's settings and the new rows go back in. If the
account holds notes and no settings, every question is asked and the
answers stay provisional until the account's settings arrive.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add app/src/onboarding.rs app/static/console.js app/tests/onboarding.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-p2-task-8.txt`

---

## Controller hand-offs

Every change below is to a shared single-owner file (`HANDOFF.md` §2). **No task edits these files.**
Each lands on the branch at the task that first needs it, as its own commit, applied **verbatim** by the
controller and reviewed with that task — never at merge. Unlike Plan 1's, two of them are
compile-blocking the other way round: H1 declares the module Task 1 creates and H3 registers the command
Task 8 creates, so each goes into the working tree after its task's code (Step 4 of Tasks 1 and 8) and
is committed immediately before that task's commit; neither commit builds alone, and the branch is only
pushed after both.

| Task | Hand-off applied with it |
|---|---|
| 1 | **H1** — `engine/src/lib.rs`: `pub mod config;`. **H2** — `engine/src/ingest.rs`: the `ics_url` fallback reads `config/device.yaml` first |
| 8 | **H3** — `app/src/main.rs`: the wizard window registers `onboarding::account_settings_exist`. **H4** — `scripts/wizard-check.py`: the command is a read, and the two D19 walks. **H5** — `CLAUDE.md`: the split, both recorded exceptions, the settings sync, the scheduler sentence and the command counts |

`engine/src/main.rs` and `engine/src/cli.rs` need nothing (no new command-line flag), nor do `Cargo.toml`
and `Cargo.lock` (no new crate: `ring`, `serde_yaml_ng` and `jiff` are already the engine's).
`cloud/supabase/config.toml` needs nothing: `[functions.sync-push]` and `[functions.sync-pull]` exist,
and Task 3 adds no function. `HANDOFF.md` is *Hand-off to the controller*, item 3.

### H1 — `engine/src/lib.rs`: the `config` module (applied at Task 1)

In `engine/src/lib.rs`, replace

```rust
// exits 0.
pub mod sync;
// Knowlu C3′ — the entitlement gate (cloud design, amendment 2026-09-17, ruling 3). The app caches
// `GET /entitlement` with a 72-hour grace; past it, the four cloud slot steps refuse to run and say
```

with

```rust
// exits 0.
pub mod sync;
// Two-desktop design D17 (§4.8): which `config/` values are the account's and which are this
// computer's — `config/device.yaml`, its readers' device-first rule, the push guard's test and the
// text-only migration that moves device keys out of the shared files.
pub mod config;
// Knowlu C3′ — the entitlement gate (cloud design, amendment 2026-09-17, ruling 3). The app caches
// `GET /entitlement` with a 72-hour grace; past it, the four cloud slot steps refuse to run and say
```

### H2 — `engine/src/ingest.rs`: the LMS feed fallback reads `device.yaml` first (applied at Task 1)

The one reader of `ics_url`, in `ingest::run_lines`, overlays `device.yaml` on the parsed config before
it reads. `timezone` is not a device key, so the overlay leaves it as it was.

In `engine/src/ingest.rs`, replace

```rust
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let url = config.get("ics_url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let tz_name = config.get("timezone").and_then(|v| v.as_str()).unwrap_or("America/Chicago").to_string();
```

with

```rust
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    let mut config: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(e) => return (1, vec![format!("ingest: config unreadable: {e}")]),
    };
    // Two-desktop design D17: the vault-held `ics_url` fallback is this computer's own, in
    // `config/device.yaml`; the shared file's is read only when `device.yaml` has none.
    if let serde_yaml_ng::Value::Mapping(map) = &mut config {
        crate::config::overlay_device(vault, map);
    }
    let url = config.get("ics_url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let tz_name = config.get("timezone").and_then(|v| v.as_str()).unwrap_or("America/Chicago").to_string();
```

### H3 — `app/src/main.rs`: the wizard window's thirtieth command (applied at Task 8)

The vault-less window's `generate_handler!` list only; the console's list is unchanged (the wizard is the
command's one caller).

In `app/src/main.rs`, replace

```rust
            }
        })
        .invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::retarget_credentials, onboarding::finish_onboarding, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::google_connect_url, account::google_connected, account::open_external, lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::capture_courses, lms_link::paste_calendar_link, lms_link::close_lms_window, onboarding::discover_coursework, onboarding::timezone_for_state, onboarding::campus_search])
        .build(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime")
```

with

```rust
            }
        })
        .invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::retarget_credentials, onboarding::finish_onboarding, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::google_connect_url, account::google_connected, account::open_external, lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::capture_courses, lms_link::paste_calendar_link, lms_link::close_lms_window, onboarding::discover_coursework, onboarding::timezone_for_state, onboarding::campus_search, onboarding::account_settings_exist])
        .build(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime")
```

### H4 — `scripts/wizard-check.py`: the command is a read, and the two D19 walks (applied at Task 8)

Four edits: `account_settings_exist` joins `BEFORE_FINISH_OK` (it writes nothing, on this machine or the
account); the fake `__TAURI__` answers it from `window.__HELD`, and answers "nothing in the account" when a
walk sets none, so the existing nine-panel walk is unchanged; two new walks; and `main` runs each on a
fresh page, since a walk ends on the finished wizard.

In `scripts/wizard-check.py`, replace

```python
                    "paste_calendar_link", "close_lms_window", "discover_coursework",
                    "campus_search", "timezone_for_state",
                    "store_credentials", "retarget_credentials"}
# A raw string: the JavaScript below is the page's, backslashes and all.
FAKE = r"""
```

with

```python
                    "paste_calendar_link", "close_lms_window", "discover_coursework",
                    "campus_search", "timezone_for_state",
                    "store_credentials", "retarget_credentials",
                    # Two-desktop design D19: a read of the account, after sign-in.
                    "account_settings_exist"}
# A raw string: the JavaScript below is the page's, backslashes and all.
FAKE = r"""
```

In `scripts/wizard-check.py`, replace

```python
      }); }
  if (cmd === 'google_sign_in') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
  if (cmd === 'send_magic_link') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'verify_email_code') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
```

with

```python
      }); }
  if (cmd === 'google_sign_in') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
  // Two-desktop design D19: what the account holds. `window.__HELD` unset is an account with nothing
  // in it, which is what `check()`'s nine-panel walk is about.
  if (cmd === 'account_settings_exist') { return Promise.resolve(window.__HELD || { ok: true, error: null, settings: false, notes: false, mapped: [] }); }
  if (cmd === 'send_magic_link') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'verify_email_code') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
```

In `scripts/wizard-check.py`, replace

```python


def check_picker_restore(page) -> list:
    """M3 (fix round 1): the picker's *Restore from a backup folder…* asks for a name and an
```

with

```python


def check_second_computer(page) -> list:
    """Two-desktop design D19 (§6.2): with settings in the account, the walk goes from sign-in to the
    logins panel to Finish. The panels the account answers never open, and only a source the
    account's settings do not map yet gets mapping rows."""
    bad = []
    page.evaluate("window.__HELD = { ok: true, error: null, settings: true, notes: true, mapped: ['zybooks'] }; window.__ENTITLED = true")
    page.click("#wiz-next"); page.wait_for_timeout(120)
    page.check("#wiz-18"); page.check("#wiz-terms")
    page.click("#wiz-google-signin"); page.wait_for_timeout(500)
    if "account_settings_exist" not in names(page): bad.append("the wizard did not ask whether the account holds settings")
    if page.is_hidden("#wiz-logins"): bad.append(f"sign-in did not go straight to the logins panel ({page.inner_text('#wiz-step')!r})")
    secret = secrets.token_urlsafe(12)
    page.fill("#wiz-vhl-user", "a@example.invalid"); page.fill("#wiz-vhl-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(500)
    rows = page.inner_text("#wiz-map-rows")
    if "UACS100Fall2026" in rows: bad.append(f"a source the account already maps was offered for mapping: {rows!r}")
    if "2102121" not in rows: bad.append(f"a portal first set up here got no mapping row: {rows!r}")
    page.fill('[data-course-for="0"]', "GN 103"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-finish"): bad.append(f"the logins panel did not go straight to Finish ({page.inner_text('#wiz-step')!r})")
    if "settings your account already has" not in page.inner_text("#wiz-summary"):
        bad.append(f"the summary names slot times the account will replace: {page.inner_text('#wiz-summary')!r}")
    page.click("#wiz-back"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-logins"): bad.append("Back from Finish did not return to the logins panel")
    page.click("#wiz-next"); page.wait_for_timeout(200)
    page.click("#wiz-next"); page.wait_for_timeout(500)
    for cmd in ["open_checkout", "open_lms_window", "campus_search", "google_connect_url", "timezone_for_state"]:
        if cmd in names(page): bad.append(f"a skipped panel's command ran: {cmd}")
    plan = (first_args(page, "create_vault") or {}).get("plan") or {}
    if not plan: bad.append("Finish did not invoke create_vault")
    if plan.get("account_holds_notes") is not True: bad.append("the plan did not carry the account's notes")
    if not any(v.get("section") == "2102121" for v in (plan.get("vhl_sections") or [])):
        bad.append(f"the new portal's mapping did not reach the plan: {plan.get('vhl_sections')!r}")
    if plan.get("zybooks_courses"): bad.append(f"a mapped source's rows reached the plan: {plan.get('zybooks_courses')!r}")
    return bad


def check_notes_without_settings(page) -> list:
    """D19: notes in the account and no settings yet — every panel is asked, and the plan says the
    account holds notes, so Finish marks the answers provisional."""
    bad = []
    page.evaluate("window.__HELD = { ok: true, error: null, settings: false, notes: true, mapped: [] }; window.__ENTITLED = true")
    page.click("#wiz-next"); page.wait_for_timeout(120)
    page.check("#wiz-18"); page.check("#wiz-terms")
    page.click("#wiz-google-signin"); page.wait_for_timeout(400)
    for panel in ["subscribe", "vault", "calendars", "logins", "gmail", "slots", "finish"]:
        if page.is_hidden(f"#wiz-{panel}"): bad.append(f"the {panel} panel was skipped with no settings in the account")
        if panel != "finish":
            page.click("#wiz-next"); page.wait_for_timeout(250)
    page.click("#wiz-next"); page.wait_for_timeout(500)
    plan = (first_args(page, "create_vault") or {}).get("plan") or {}
    if plan.get("account_holds_notes") is not True: bad.append("the plan did not carry the account's notes")
    return bad


def check_picker_restore(page) -> list:
    """M3 (fix round 1): the picker's *Restore from a backup folder…* asks for a name and an
```

In `scripts/wizard-check.py`, replace

```python
                page2.goto(url); page2.wait_for_timeout(400)
                bad += check_picker_restore(page2)
                console = browser.new_context(viewport={"width": 1280, "height": 860}).new_page()
                errors = []
```

with

```python
                page2.goto(url); page2.wait_for_timeout(400)
                bad += check_picker_restore(page2)
                # Two-desktop design D19: the second computer's two walks, each on a fresh page.
                for walk in (check_second_computer, check_notes_without_settings):
                    fresh = browser.new_context(viewport={"width": 1280, "height": 900}).new_page()
                    fresh.add_init_script(FAKE)
                    fresh.goto(url); fresh.wait_for_timeout(400)
                    bad += walk(fresh)
                console = browser.new_context(viewport={"width": 1280, "height": 860}).new_page()
                errors = []
```

### H5 — `CLAUDE.md`: what D17–D19 change (applied at Task 8)

Seven edits. Each quotes the text it replaces as `CLAUDE.md` stands on `two-desktop` at `bdd33c6`, and
none touches a sentence Plan 1's H4 edits (H4 appends to the end of Edit 2's bullet and inserts inside
Edit 4's, after "another desktop's writes down and applied through `write`."). If H4's application moved
a line break inside one of these, find the text by its words. The spec asks for two of them (§4.8: name
both exceptions; the scheduler sentence); the rest are sentences D17–D19 make stale.

**Edit 1** — *Engine invariants*, the vault bullet's first sentence (`CLAUDE.md:34-36`): name the split. Replace

```markdown
- A vault is markdown + YAML frontmatter (`tasks/`, `approvals/`, `archive/`, `courses/`, `info/`,
  `issues/`, `config/`), the single source of truth. `state/` is generated; `today.md` is rewritten
  every run. The engine is **deterministic**: same input, same order.
```

with

```markdown
- A vault is markdown + YAML frontmatter (`tasks/`, `approvals/`, `archive/`, `courses/`, `info/`,
  `issues/`, `config/`), the single source of truth. `state/` is generated; `today.md` is rewritten
  every run. The engine is **deterministic**: same input, same order.
- `config/` is split (two-desktop D17, `engine/src/config.rs`): six files are the account's shared
  settings (`sync::SHARED_CONFIG` — `campus`, `events`, `ingest`, `planning`, `runners`,
  `week_template`) and travel whole through `sync`; **`config/device.yaml`** is this computer's own —
  each portal's `enabled`, `credential_target` and `base_url`, the `ics_url` fallback, every
  non-`cloud:` calendar, the `local` runner's `device` and `scheduler` — and is never synced, like
  `config/cloud.yaml`. Readers take a device key from `device.yaml` first and the shared file only when
  it has none (`config::overlay`; `calendars:` is the union), so an unmigrated vault still works; a
  capability URL never enters a synced file. `config::device_yaml` is `device.yaml`'s one writer of a
  new file, and later keys go in by line insertion.
```

**Edit 2** — the same section, the first line of the bullet "Never rewrite a vault file wholesale" (`CLAUDE.md:40`): name both recorded exceptions in it (spec §4.8, D18). The rest of the bullet — including what Plan 1's H4 appended after "`source_uid` is the external key." — is unchanged. Replace

```markdown
- Never rewrite a vault file wholesale. Every note write goes through `write`: journal record first
```

with

```markdown
- Never rewrite a vault file wholesale — with two recorded exceptions, each bounded to one kind of
  file: C3′'s seed pre-pass (`sync`'s `apply` replaces an untouched wizard seed note, byte for byte as
  the wizard wrote it, with the account's text at its path), and two-desktop D18's settings pull
  (`sync::apply_settings`, and a restore's `materialise`, write the account's text verbatim over one of
  the six `sync::SHARED_CONFIG` files; a changed local text is kept first under
  `state/config-conflicts/`). Every other `config/` write inserts or deletes lines
  (`coursework::insert_mapping`, `config::move_device_keys`, `config::insert_source_key` and its
  siblings) or writes a new file from literals (`config/device.yaml`); no config file is parsed and
  re-dumped. Every note write goes through `write`: journal record first
```

**Edit 3** — *The engine's commands*, the `coursework` bullet (`CLAUDE.md:58`): where the credential target is named. Replace

```markdown
  Credential Manager via the vault's `credential_target`; zyBooks 403s without a `User-Agent`; VHL is
```

with

```markdown
  Credential Manager via the `credential_target` this computer's `config/device.yaml` names (the
  shared `config/ingest.yaml`'s in a vault `sync` has not split yet); zyBooks 403s without a
  `User-Agent`; VHL is
```

**Edit 4** — the `sync` bullet's last sentence (`CLAUDE.md:79-80`, "... and deletes it with the account."): append the settings, inside the same bullet. Replace

```markdown
  and deletes it with the account.
```

with

```markdown
  and deletes it with the account. It also carries the six shared settings files whole (two-desktop
  D18): a push sends a changed one with the SHA-256 of the text last synced as its `base`, and the
  server stores it only by compare-and-set (`save_config_row`; a refused path keeps its base and is
  named); a pull takes the account's text by a three-way check, then puts back every card-approved
  mapping the new text lacks (`coursework::apply_approved_mappings`). `sync` runs
  `config::move_device_keys` first on every run, and a shared file that still holds a device key or a
  capability URL stays on this computer, named. A new computer's settings stay provisional
  (`Cursor::provisional_config`: used, never pushed) until the account's arrive, the student edits one
  in the app, or seven days pass — **a console command that writes a shared setting calls
  `sync::settings_edited` after its write** (none does yet).
```

**Edit 5** — *Knowlu (the app)*, the command-count sentence (`CLAUDE.md:97-103`). Recount by script at merge before quoting it; another branch may have moved it. Replace

```markdown
  with `console_ctx()` (`via: "dashboard"`). **Tauri commands, recounted 2026-09-24 (C3′ Task 12, by
  script, over the two `generate_handler!` lists in `app/src/main.rs`; C3′ added none)**: the console
  window registers **42**, the vault-less picker/wizard window **29** (+3 from C2's hand-off H9 phase
  (a) — `account::google_connect_url`, `account::google_connected`, `account::open_external`; C1b's
  H1 removed `account::sign_up` and `account::sign_in` with the password and added
  `account::google_sign_in` to both lists) — **61** distinct. Commands live beside the module they
```

with

```markdown
  with `console_ctx()` (`via: "dashboard"`). **Tauri commands, recounted 2026-09-26 (two-desktop Plan 2
  Task 8, by script, over the two `generate_handler!` lists in `app/src/main.rs`)**: the console
  window registers **42**, the vault-less picker/wizard window **30** (+3 from C2's hand-off H9 phase
  (a) — `account::google_connect_url`, `account::google_connected`, `account::open_external`; C1b's
  H1 removed `account::sign_up` and `account::sign_in` with the password and added
  `account::google_sign_in` to both lists; two-desktop D19 added `onboarding::account_settings_exist`,
  a read of the account after sign-in) — **62** distinct. Commands live beside the module they
```

**Edit 6** — the scheduler sentence (`CLAUDE.md:125-127`). Replace

```markdown
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless the vault's `config/runners.yaml` `local` entry says `scheduler: app` for this `device:`; a
  wizard-created vault carries both from birth.
```

with

```markdown
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless this computer's `config/device.yaml` `runner:` says `scheduler: app` for this `device:`
  (`runs::local_runner_settings`; `config/runners.yaml`'s `local` entry only when `device.yaml` has
  neither); a wizard-created vault carries both in `device.yaml` from birth.
```

**Edit 7** — the credentials bullet (`CLAUDE.md:128-129`). Replace

```markdown
  engine's `wincred.rs` reads whatever `credential_target` the vault names.
```

with

```markdown
  engine's `wincred.rs` reads whatever `credential_target` this computer's `config/device.yaml` names.
```

## Spec problems found while planning, and how the plan handles them

None changes a signed decision. S-1 to S-4 are places §4.8 cannot be carried exactly as written; each is
decided by a Plan ruling, named at its step. S-5 to S-8 are places the spec is silent.

- **S-1 — "`is_note_path` gains the list" would let a pulled record edit a config file.** `is_note_path`
  is the one predicate `apply`, `materialise` and `build_push` use for records, moves and tombstones as
  well as texts; widened, a `set` record naming `config/ingest.yaml` would reach `write_literals` and do
  frontmatter surgery on a settings file, and a tombstone could delete one. R-TD2-1 keeps `is_note_path`
  the note rule and gives the six paths their own predicate, accepted only by the settings code paths.
  The server's two checks (`sync_rows.ts`, the constraint) do widen, as the spec says.
- **S-2 — "a missing one comes back with the next pull" is not true of a cursor-based pull.** A pull
  brings only rows changed since the cursor, so a settings file deleted on this computer comes back only
  when another computer next changes it — possibly never. R-TD2-9 makes the sentence true: after every
  pull, a shared file with a base and no file is read back from `GET /sync-pull?view=settings`.
- **S-3 — ending (b), "the student edits a setting in the app", has no event to hang on.** No console
  command writes a shared setting today: `set_settings` writes the profile's `settings.json`, and every
  shared setting is written by the wizard, a card or a hand edit (which the spec rules out as an event).
  R-TD2-11 builds (b) as a seam, `sync::settings_edited`, tested directly, and H5 tells the first such
  command to call it.
- **S-4 — D19's "one `/sync-pull` call" cannot answer "does the account hold `config/ingest.yaml`?".**
  `/sync-pull` is paged by `rev`; the settings row may sit on any page, so one call can only say "not on
  page one". R-TD2-13 adds a `view=settings` read to the same function — still one call — that answers
  the live settings rows and whether any note exists. The same read serves S-2.
- **S-5 (minor) — `write_mapping` refuses a source with no `coursework.<source>:` block** (a pre-D17 rule:
  "the source is not set up"). Under D17 a source is set up by `device.yaml`'s `enabled`, and a second
  computer can bring a portal the account's shared file has never had. R-TD2-20 creates the block with
  the source's shared defaults; every other rule of `write_mapping` stands.
- **S-6 (minor) — the wizard's rows for a new portal would be lost to the restore** (re-review r6's
  case): D19 offers mapping rows only for a source the account does not map, the wizard writes them into
  its `ingest.yaml`, and the restore then replaces that file with the account's. R-TD2-15 puts them back
  by insertion.
- **S-7 (minor) — the guard's reach.** The spec names `https://` and `webcal://`; a capability URL can
  also be `http://`, and a file that does not parse cannot be scanned. R-TD2-4 refuses both, each with
  its own line.
- **S-8 (minor) — the conflict file's name, and the pull's own lines**, are unworded (R-TD2-7,
  R-TD2-8); so is what `restore_capability_url`'s calendar case leaves in the shared file (R-TD2-19).

Found outside the spec, while planning: **the settings leave the device before the privacy page says so**
unless the two ship together. Task 4 is the first code that sends a settings file to the account; the
privacy words that name them (§3.3) are Plan 3's. *Hand-off to the controller*, item 4 makes the words a
release gate for Task 4.

## Self-review (2026-09-26, done while writing)

- **Spec coverage.** Every §4.8 sentence that asks for behaviour has a row in the *Fidelity ledger* and a
  task; §6.1's shared-config bullets have named tests — no capability URL travels: Task 2's
  `the_guard_names_a_device_key_a_raw_calendar_or_an_address_and_passes_a_clean_file` and Task 4's
  `no_capability_url_ever_travels_and_the_migration_is_what_lets_the_file_go`; the migration's shape,
  twice a no-op and a crash between its steps: Task 2's `the_migration_moves_exactly_…` and
  `a_crash_between_the_two_steps_…`; the three-way pull and the refused push: Task 5's
  `the_three_way_pull_…` and `a_refused_push_…`; `apply_approved_mappings`: Task 5; the provisional mark
  (automatic writes, a pull, an app edit, seven days with the clock injected, a refused insert): Task 6's
  five tests. §6.2's two D19 walks are H4's `check_second_computer` and
  `check_notes_without_settings`, with Task 8's Rust tests beneath them; §6.3's `sync-push`/`sync_rows.ts`
  bullets and the function pin are Task 3's. D9–D16 and §3.3's words are Plan 3's and only referenced.
- **Placeholders.** None: every test and every code change is written out as the exact text it replaces
  and the text that replaces it, generated from the tree that passed (header, *Checked while writing*).
- **Type consistency.** `DeviceKeys`, `SourceKeys`, `PortalSource`, `DeviceSpot`, `PushReply`,
  `AccountSettings`, the two `Cursor` fields, `WizardPlan::account_holds_notes`, and every signature in
  *Interfaces this plan produces* are spelled the same in each task that uses them — they compiled
  together, task by task.
- **Warnings.** No `#[allow(dead_code)]` anywhere: every function has its first caller or test in the
  task that adds it (`settings_edited` and `mark_provisional` are `pub`, called by tests and by
  `restore_all_as`).
- **What the scratch runs found, and this text fixes:** an empty `calendars:` list read as a malformed
  one by the migration's sequence reader (it is now an empty list to insert into); `app/tests/scaffold.rs` reading a CRLF file with
  `fs::read_to_string` (it uses `pystr::read_text`); `insert_mapping` creating a source block without
  its `courses:`/`sections:` header; the migration's first stamp colliding with the commitment model's
  `20260926000100` (R-TD2-18); two edits anchored on lines Plan 1's fix round also changes (now anchored
  beside them); and Global Constraint 19's order, since building the app's tests blanks the engine exe
  the oracle tests spawn.
- **Line numbers** were read in the scratch tree with Plan 1 applied; Plan 1's execution and each earlier
  task here move them. Each step's edit is found by the text it quotes, never by its line number. Tasks
  1, 2, 3, 7 and 8 were also applied, as written, to the branch's real head at `3010cdc` (Plan 1's Tasks
  1–2 executed): every quoted text was found exactly once. Tasks 4–6 quote code Plan 1's Task 10 adds
  (`Cursor::verdicts_after`, `query_value`), so they wait for Plan 1 whole (Global Constraint 8).

## Hand-off to the controller

After Task 8's commit the branch carries Plan 2 whole. What is left is the controller's.

1. **The gate**, in the foreground, from the worktree: Global Constraint 19's three cargo commands — 0
   warnings, the gate line ending `0 other`; the whole Deno suite as CI runs it (`ci.yml:98`, from the
   repository root: `deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1
   --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS
   --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`); `pwsh -File scripts/ci/eol-check.ps1`;
   `python scripts/wizard-check.py` → `ok`; and `git diff --stat <Plan 2's base> -- engine/tests/fixtures`
   empty. An `onboarding.rs` failure reading `no session at knowlu/pending/session to move` is Global
   Constraint 11's race with another worktree: re-run that binary alone once nothing else runs it.
2. **Rollout — the order is the point.**
   - **Staging, after Task 3:** deploy the migration `20260926000200_shared_settings.sql`, then
     `sync-push` and `sync-pull`. The smoke, with a staging session minted by the standing OTP procedure
     (the controller holds it; no agent does): `POST /sync-push` with `{"path": "config/planning.yaml",
     "body": <text>, "base": ""}` in `notes` answers `config: 1`, `config_refused: []`; the same row again
     with `base: ""` answers `config_refused: ["config/planning.yaml"]`; a changed body with `base` = the
     SHA-256 of the stored text answers `config: 1`; `{"path": "config/planning.yaml", "deleted": true}`
     and `{"path": "config/device.yaml", ...}` each answer 400; `GET /sync-pull?view=settings` answers
     `{"settings": [{"path", "body"}], "notes": <bool>}`.
   - **Production, before any release that carries Task 4.** A server without the widened check refuses
     a settings row with a 400, and a push is all or nothing, so a new build against an old server would
     stop syncing notes too. The migration and both functions join `HANDOFF.md` §4's production-parity
     checklist. An old build against the new server is harmless: it sends no settings rows, and refuses
     each one it pulls with "a pulled note named a path outside the vault's notes", once per changed file
     (spec §4.8, *Rollout warnings*).
   - **With the commitment model's migration.** `20260926000100_sync_note_path_check_commitments.sql`
     (branches `p1-`/`p2-commitments`, `p3-registrar`) replaces the same constraint with the note folders
     only. This plan's `000200` carries `commitments` in its folder group, so it is the union either way —
     **but only if it is applied after `000100`.** In any environment where `000200` is deployed first,
     `000100` must never be applied after it (`db push --include-all` would, and settings pushes would
     then fail with 400): at that merge, drop `000100` from the commitment branch, since `000200`
     subsumes it, or re-assert the union in a migration stamped after both.
3. **`HANDOFF.md` §3**, batched with the next milestone push (CI minutes): Plan 2 is code-complete on
   `two-desktop` (D17–D19; S-1 to S-4 as ruled by R-TD2-1, -9, -11, -13); the staging deploy of item 2
   is done or owed; production parity gains one migration and two functions; the release gate of item 4.
4. **Plan 3's hand-offs** (write them into Plan 3's brief):
   - **The privacy words are a release gate for Task 4.** The page does not say the account keeps
     settings (`site/privacy.html:36` names only "the text of every task and note" and the journal).
     Plan 3 owns the one privacy change (§3.3); it must carry, verbatim, §3.3's settings sentences —
     *"Your account also keeps your settings, so that every computer plans your day the same way: your
     time zone, your school, which course each coursework book or section belongs to, your calendars,
     the campus event feeds you follow, when your day refreshes, and your weekly planning template. What
     belongs to one computer stays on it: which logins it holds, where Windows keeps them, and any
     calendar link saved on it."* — and the closing clause *"deleting your account deletes the list and
     your settings."*, pinned in `engine/tests/site.rs`, with §3.3's `PRIVACY_VERSION` rule. No release
     carries Task 4 without them.
   - **The migration stamp and the pin.** Plan 3's migration sorts after `20260926000200` (e.g.
     `20260926000300_fetch_turn.sql`), and its `fetch_turn` moves the function pin **29 → 30**
     (`migrations_test.ts`, "today's corpus should parse exactly 29 function creations").
   - **What it consumes** is *Interfaces this plan produces*, verbatim. In particular: the registry's
     `devices.logins` comes from `config::portal_sources` (a source `enabled` in `device.yaml` whose
     `credential_target` holds a credential), never from a synced file; the scheduler's runner keys come
     from `runs::local_runner_settings`; a slot's `sync` step is where `config::move_device_keys` runs;
     `sync::run_lines_with_client`'s order is the table's. A Plan 3 command that writes a shared setting
     calls `sync::settings_edited` after its write.
   - **Plan 1's `sync::pull_event_verdicts`** is still Plan 3's to call for a computer that runs no
     `judge` (Plan 1, R-TD1-4); nothing here changes it.
5. **The Credential Manager race** (an open finding for Quinn, not this plan's): `app/tests/onboarding.rs`
   uses the machine-global target `knowlu/pending/session`, so two worktrees running it at once fail each
   other. Seen again in this plan's scratch runs.

### Overlaps with the unmerged branches

Read on 2026-09-26 with `git diff --name-only $(git merge-base two-desktop <branch>) <branch>`, against
this plan's files. None of these branches is edited by this plan; whichever merges second resolves the
text, and must not lose the semantics below.

| Branch | Files it shares with this plan | What to keep in mind |
|---|---|---|
| `p1-commitments`, `p2-commitments`, `p3-registrar` (the commitment model, phases 1–3) | all three: `engine/src/{coursework,calfeed,sync,lib}.rs`, `engine/tests/sync_contract.rs`, `app/tests/scaffold.rs`, `CLAUDE.md`, `migrations/migrations_test.ts`, `_shared/sync_rows{,_test}.ts`; `p2`/`p3` also `app/src/main.rs`, `app/static/console.js`, `scripts/wizard-check.py`; `p3` also `app/src/scaffold.rs` | **The path-check migration** (item 2). **`sync_rows.ts`**: its note regex gains `commitments`; Task 3's settings branch sits beside it, and `sync_contract.rs::is_note_path_and_the_servers_regex_agree` must still find the folder list. **`calfeed.rs`**: `p3-registrar` moves `calendar_entries` and `load_calendar_events` onto a new `calendars_feeds(config)`; Task 1's `overlay` must stay ahead of it in both. **Every new reader of a device key** (`ics_url`, `calendars:`' addresses, `coursework.<source>.{enabled, credential_target, base_url}`, the runner's `device`/`scheduler`) reads through `config::overlay_device` or `runs::local_runner_settings`, or it reads nothing on a split vault. **`main.rs`**: the console gains `commands::answer_card`, `registrar::{open_registrar_window, capture_registrar, close_registrar_window}` and `week::{your_week, commitment_proposals, commitments_confirm, preview_window}` — H5's counts are recounted at merge. **`console.js`**: the registrar button is in the week view, not a wizard panel, so `HELD_PANELS` is unaffected. **`scaffold.rs`**: `Registrar`/`Call` and `Curated::registrar` sit beside Task 7's edits. **`CLAUDE.md`**: `p3-registrar` rewrites ~85 lines; H5 is found by its words. |
| `j-followups` | `engine/src/{coursework,sync,lib}.rs`, `engine/tests/sync_contract.rs`, `CLAUDE.md`, `migrations_test.ts` | Its one `sync.rs` hunk is `propose_amendment`'s new argument (Plan 1's overlap, not near Tasks 4–6). Its `coursework.rs` hunks are the fetchers and `main_with_fetchers`, away from `load_coursework_config` and `write_mapping`. Its `migrations_test.ts` additions pin its own migrations, which create no function: the pin stays 28, and this plan's 29 holds. |
| `j-judgment-quality`, `jf-cloud` | `engine/src/{coursework,lib}.rs`, `CLAUDE.md`, `migrations_test.ts` | Text only: their `coursework.rs` hunks (`route_zybook`, `fetch_*`, `main_with_fetchers`) are away from `load_coursework_config` and `write_mapping`. |
| `j-email`, `j-completion` | `engine/src/{coursework,lib}.rs` | Text only (`lib.rs` module lines beside H1's). |
| `j-events` | `migrations_test.ts` | Text only; pin unchanged. |
| `fix-literal-dashes` | `engine/src/coursework.rs` | Its `coursework.rs` hunks are in `sync_coursework` and the tests, away from Tasks 1, 2 and 5. |
| `c5-relay` | none yet (its plan is unexecuted; it stacks on `two-desktop`) | Its fetch moves portal credentials through the relay: it must read them from `config::portal_sources`, i.e. `device.yaml`. |

### RED and GREEN, as run while writing

In the scratch tree, every task's new tests were first run without its code (RED), then with it
(GREEN), one task at a time in order.

| Task | RED (without the task's code) | GREEN (with it) |
|---|---|---|
| 1 | `device_config.rs` does not compile: `unresolved import knowlu_engine::config`, no `runs::local_runner_settings` | 5 tests; the engine suite whole |
| 2 | `cannot find function computer_only_value`, `move_device_keys`, `insert_source_key` (…) in `config` | 11; the engine suite whole |
| 3 | Deno type-check: no `isSharedConfig`/`SHARED_CONFIG`/`handleSettings`/`saveConfigRow`; the pin test reads 28 and cannot read the migration | the whole `cloud/supabase` suite (443 passed, 0 failed), `deno check` and `deno lint` clean |
| 4 | `cannot find value SHARED_CONFIG in module sync`; no field `settings` on `PushBatch` | `sync_settings.rs` 3; `sync_contract.rs` unchanged; the engine suite whole |
| 5 | no `apply_settings`, `account_settings`, `restore_missing_settings`, `CONFLICTS_DIR`, `apply_approved_mappings`, `insert_mapping` | 7 and 12; the engine suite whole |
| 6 | no `mark_provisional`, `publish_if_due`, `settings_edited`, `restore_all_as`, `restore_into_as` | 12; the workspace whole |
| 7 | `scaffold.rs`: no `scaffold::device_yaml`; `onboarding.rs`: six assertions fail (no `device.yaml`) | `scaffold.rs` 27, `onboarding.rs` 36; the workspace whole |
| 8 | no field `account_holds_notes`, no `account_settings_exist_at`; the wizard walk FAILs both D19 walks | `onboarding.rs` 39; `wizard-check.py` → `ok`; the workspace whole |
