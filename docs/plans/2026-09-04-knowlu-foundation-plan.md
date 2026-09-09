# Knowlu Foundation for Independence — Implementation Plan (Knowlu plan 1 of 4; console plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: EXECUTED 2026-09-05 on branch worktree-knowlu-plan-1 (Tasks 1–17; the SDD ledger with every ruling is preserved at docs/superpowers/reports/2026-09-05-knowlu-plan-1-sdd-ledger.md).** Execute in a fresh worktree on branch `worktree-knowlu-plan-1`. The console runs against **scratch copies of the vault** for the whole plan; the switch to the live vault is Task 17 and happens only after G2.

**Goal:** Knowlu can be used instead of Obsidian — every write the console spec names, the deck executing decisions in-process, history and backup, interaction events, the scheduler built but inert behind its config key, the Runs view, and the user-visible rename — with the one bundled read-model change from plan 1's final review landed first.

**Architecture:** Four new engine modules that only the console calls — `uievents` (interaction events through the ledger seam), `schedule` (pure slot arithmetic over run records), `backup` (mirror + dated snapshots), `history` (git as a transport, conflicts through `reconcile`) — and the surface bundle in `src/surface.rs`. The app crate gains eleven mutating commands in `commands.rs` that call, marshal and wrap; a tray, autostart, hide-to-tray and a scheduler thread in `app/src/`; and the page's click-to-edit, deck actions, issue flags, sync/backup topline and Runs view in `console.js`. Nothing here changes what `rank` or `write` writes; `tests/oracle.rs` and both dual-run scripts stay green throughout.

**Tech Stack:** Rust 1.98 `stable-x86_64-pc-windows-gnu`; engine crate deps already present (`jiff 0.2`, `serde`, `serde_json`, `serde_yaml_ng`, `sha1`, `regex`) plus **`zip = { version = "2", default-features = false, features = ["deflate"] }`** (Task 4); Tauri 2.11 with `tray-icon` feature, `tauri-plugin-single-instance`, `tauri-plugin-window-state`, **`tauri-plugin-autostart = "2"`**, `windows 0.62` (`Win32_UI_WindowsAndMessaging`) in the app crate only; plain HTML/CSS/JS, `withGlobalTauri`; Playwright in `.wv` for screenshots only.

**Spec:** `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` (§2 phase 0, §3, §4, §5.1's starving rule as a constraint, §7 stage 1, §8, §9 plan 1) argued together with `docs/superpowers/specs/2026-09-02-console-on-rust-design.md` §5, §7, §8, §9 (the commands, writes, deck, sync, freshness) and S2 `docs/superpowers/specs/2026-08-31-s2-surface-design.md` §7.6, §9.2–§9.5, §10. Plan 1's ledger, `docs/superpowers/reports/2026-09-03-console-plan-1-sdd-ledger.md`, holds the rulings R1–R40 this plan inherits and the bundle Task 1 lands.

## Global Constraints

Every task's requirements implicitly include these. Values are copied from the specs.

- **The engine is under dual-run measurement until ~2026-09-18 and the console must not write the live vault before G2 (~2026-09-11).** Every `src/` change here is a new module only the console calls, or a read-model change. **Nothing may change what `rank` or `write` writes.** `tests/oracle.rs` is green throughout — red is a stop. Never regenerate any of the eight frozen Python-written references. The three `surface-today-*.json` references are regenerated **once**, in Task 1, in a commit whose diff shows the change and whose message says why (spec §4.6); no other task regenerates them.
- **Scratch copies, never the live vault.** Every test copies `tests/fixtures/vault-full`; every manual look uses `scripts/scratch-vault.ps1` (Task 7). The Start-menu shortcut keeps pointing at the live vault with the **old** console until Task 17.
- **The cloud routine is never touched** — not its prompt, not its config, not its environment (spec §5.1, decision 13). `QUINN_OPS_DEVICE` keeps its name (rename stage 3).
- **Before any claim of green:** `cargo test` (dev profile; `--release` will not link) at **0 warnings**; `cd app; cargo test` with **zero new warnings** (the `.rsrc merge failure` linker line is pre-existing and is not this plan's); `scripts/diff-engines.ps1` on all three fixtures and `scripts/diff-engines-notes.ps1` clean after any `src/` change; `.venv\Scripts\python.exe -m pytest -q` still 687 — **`engine/` does not change by one line.**
- **Every console write goes through `quinn_ops::write` / `approvals` / `issues` / `info`** with `WriteContext { actor: "quinn", via: "dashboard", run_id: None }`. The app re-checks `via` against `journal::VIAS` itself (the engine validates it only in clap). Console writes are not a run and write no run record.
- **`commands.rs` computes nothing.** It parses arguments, calls the engine, and wraps the result. Every mutating command returns `{ok, error, state}` with the **freshly rebuilt** state for the requested view; a refused write returns `ok: false` **and** the current state.
- **The read model never writes.** `surface::build_state` byte-compare test (plan 1 Task 8) stays green.
- **The scheduler is inert unless the local runner's entry in `config/runners.yaml` says `scheduler: app`.** The key is absent from the live vault throughout this plan; absent means `script`. Both engines ignore unknown keys in a runner entry (tested in plan 1's predecessor); this plan adds a test that `load_runners_config` still does.
- **The app never builds anything and never runs the passes** (`verify_tail`, `detect_external`, `ensure_ids`) itself; a slot runs the sibling engine exe as a child process.
- **All JSON the engine crate writes goes through `ledger::dumps_value`**; interaction events included. Tauri serialises over IPC itself.
- **No `http://` or `https://` string anywhere under `app/static/`**, comments included (`app/tests/static_assets.rs`). No `import ` / `require(` in `console.js`. Every empty-state phrase comes from the engine's `empty_text`/`texts`.
- **Names:** the product is **Knowlu** in everything a user sees (window title, tray tooltip, `productName`, README, the new-task row's placeholder). The engine binary, crate, `%LOCALAPPDATA%\quinn-ops`, `QUINN_OPS_*`, credential targets and the repository keep their names (rename stages 2–3). The console exe becomes `knowlu.exe`.
- **Credentials never in the repo, a log, a backup rule or a test fixture.** The backup mirror copies the vault's own `config/`, which today holds Quinn's token — to storage that is Quinn's own; this is recorded, not fixed here (profiles milestone).
- **No single-user assumption.** Vault from `--vault`; backup folder, scheduler mode, device name from config or the app-data settings file; no path, machine name or user name in code or tests.
- **Desktop safety:** never send synthetic keyboard or mouse input to this desktop; screenshots by `PrintWindow` on a window handle only, never a full-screen grab (R27).
- **Line endings:** `src/*.rs`, `app/src/*.rs`, `app/tests/*.rs`, `tests/*.rs` are LF; everything under `docs/`, `scripts/`, `app/static/`, `app/*.json`, `config/`, `tests/fixtures/` is CRLF. Check `git diff --stat` — a whole-file flip is every line changed.
- **Obsidian Git sweeps uncommitted files into `vault backup` commits every ~5 minutes in the main checkout** (not the worktree). Commit app scaffolding *before* any long build anyway. `/app/target/`, `/app/gen/`, `/target/`, `/shots/` stay git-ignored.
- **Commits:** `git add` specific paths, never `-A`; message via `-F <file>` (PowerShell 5.1 splits an inline here-string); trailers `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the session's `Claude-Session:` line. Merges into `main` happen in the main checkout via the PowerShell tool; check `git rev-list --left-right --count main...origin/main` first.
- **Build PATH:** `$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"` first. In the Bash tool: `export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"`.
- **Manual `rank`s against live-vault copies stay rare** (Google 429). Tests never fetch: `rank` is never run by a test in this plan; the scheduler's slot execution is covered by an argv-builder test plus one `#[ignore]` integration test.

---

## Fidelity ledger — rulings this plan must not lose

| # | Ruling | Source | Carried by |
|---|---|---|---|
| F1 | Console writes the live vault only from G2; scratch copies until then | Knowlu spec §2, decision 10 | Tasks 7, 17; every test's `scratch()` |
| F2 | The routine is starved, never edited; nothing here touches it | Knowlu spec §5.1, decision 3 | all (no task names the routine) |
| F3 | The app is the scheduler, tray-resident, autostart on, engine steps as child processes | Knowlu spec §3, decision 5 | Tasks 3, 7, 12 |
| F4 | `scheduler: script \| app`; absent = script; flipped only at phase 2 | Knowlu spec §3, decision 6 | Task 3 (parser + default), Task 12 (inert on script) |
| F5 | A slot is due when a slot earlier than now has no run record today; catch-up on wake; inside grace on time, past grace late, never skipped; one run at a time | Knowlu spec §3 | Tasks 3, 12 |
| F6 | The journal is the history; git is a transport; commit by name never `-A`; only-ahead → push never rebase; conflicts through `reconcile`; a failed sync is amber; an unsettled conflict stops auto-sync | Knowlu spec §4; console spec §8; S2 §10 | Tasks 5, 6, 10, 15 |
| F7 | Backup = live mirror (temp-and-rename, never deletes, removed notes to the mirror's `archive/`) + 30 daily zip snapshots; amber never blocking; credentials never a rule | Knowlu spec §4, decision 9 | Tasks 4, 10, 15 |
| F8 | Every console edit is an `actor: quinn` record via `dashboard`; the console is the primary input to judge-once | console spec §7.1 | Tasks 8, 9 |
| F9 | Editable set is S2 §9.2's twelve fields; `id`, `source_uid`, `also_uids`, `judgment` never editable; progress 100 sets `status: done` in the same write; delete behind a confirm → `archive/` | S2 §9.2; console spec §7.1 | Tasks 8, 13 |
| F10 | New task: title required, course datalist, due default 23:59, effort default 1.0, importance 3 not asked | S2 §9.2 | Tasks 8, 13 |
| F11 | Refusals: `ok: false` + current state; field snaps back; reason inline; nothing retried silently | console spec §7.2 | Tasks 8, 13 |
| F12 | `decide` writes the decision (`status`, `snooze_until`, `decision_note`) and runs `process_approvals` in-process in the same call; calendar-event approvals stay "awaiting calendar pass" | console spec §7.3, decision 14 | Tasks 9, 14 |
| F13 | Deck: one card, two slivers, `N behind`, fixed height, note field always visible, one click commits, no confirm relabel, ordered by `first_proposed_at`, snooze in the overflow, header with all six figures, "clear" only when pending == 0 and events_in_digest == 0 (R31) | S2 §9.5; R31 | Task 14 |
| F14 | ⚑ on every `is_judged` object → eight category chips **and** free text → `open_issue` snapshots the object; nothing acts on it; GOOD TO KNOW close → `close_info` | console spec §7.4; S2 §9.4 | Tasks 9, 14 |
| F15 | Interaction events through the ledger seam to `state/events-ui/YYYY-MM-DD.jsonl`, ids only, never read by the engine, git-ignored unless `config/planning.yaml` opts in; `object_seen` ≥ 2 s | S2 §7.6; console spec §7.5, decision 17 | Tasks 2, 11, 15 |
| F16 | Delta window stamped at the END of the look (blur/hide), never at first paint (R37) | plan-1 R37 | Tasks 11, 15 |
| F17 | `engine_newer` compares the console's build SHA with `git rev-parse --short HEAD` in the vault; `None` for a non-git vault (R36); never spawned on the UI thread | plan-1 R36 | Tasks 11, 12 |
| F18 | A missing vault shows a `MessageBoxW` before exit 2 under `windows_subsystem = "windows"` (R38) | plan-1 R38 | Task 7 |
| F19 | The reorder hold protocol: a route is never a reorder (R28); the hold freezes only ordered lists (R29); it applies to the four list views (R30); a mutating command's returned state is painted with `force` | plan-1 R28–R30 | Tasks 13, 14 |
| F20 | Every region always renders; empty strings come from the engine | S2 §9.6; F20 of plan 1 | Tasks 13–15 |
| F21 | Runs view renders run records with full warning text and the expected rows; `runner-log.md` is not read by the app | Knowlu spec §3, decision 19 | Tasks 1, 15 |
| F22 | The bundle: `runs_panel.expected.due` in vault-local time; no `-0.0` sums; one runway implementation; `must_do.empty_text` absent beside unreadable rows; verdict uses `EMPTY_TEXT.active`; `closed_this_week` one row per note and bounded; RunsPanel warnings channel; `journal.warnings()` surfaced; `delta.records` bounded; `the_day.open_hours` clamped — one reference regeneration | plan-1 ledger line 251 and 256 | Task 1 |
| F23 | Knowlu in everything a user sees; engine names, app-data folder, env vars, credential targets, repo unchanged | Knowlu spec §7 stage 1 | Task 7 |
| F24 | Hide to tray on close; Quit stops the scheduler and the next launch says how many slots were missed; tray menu Open · Run now · Pause scheduling · Copy diagnostics · Quit | Knowlu spec §3 | Tasks 7, 12 |
| F25 | `docs/surface/anatomy.md` updated per region; screenshots at eight viewports with the 820 px gate before layout is called done | S2 §11 item 4, §11.12; R40 | Task 16 |
| F26 | `load_runners_config` ignores unknown keys in a runner entry (`device`, `scheduler`) | cutover plan Task 1 | Task 3 |

### Per-task preservation check (reviewer runs this on every task)

1. `cargo test` green, 0 warnings; `tests/oracle.rs` green; `tests/surface_oracle.rs` green (Task 1 regenerates once, later tasks never); both dual-run scripts clean after any `src/` change; Python 687; `git diff --stat` shows nothing under `engine/` and nothing under `tests/fixtures/` except Task 1's three JSON files.
2. `src/surface.rs` still opens no file for writing: grep for `write_text`, `fs::write`, `OpenOptions`, `append(` — zero.
3. No engine module other than `uievents.rs` references `events-ui` or `uievents::` (Task 2's test).
4. No line under `app/src/commands.rs` contains arithmetic on vault data or a decision about a note's content; it parses, calls, marshals, wraps.
5. Every test copies a fixture into a temp directory; no test opens the live vault or the worktree's own `config/`.
6. Every new engine `pub fn` that writes has a doc line naming its only caller (the console) and a test proving it never touches a path outside the vault (or the backup target).
7. Line endings per the Global Constraints; no whole-file flip in `git diff --stat`.
8. No `http://`/`https://` under `app/static/`; the 18 render-function names of plan 1 still exist; new ones are added to the list in `static_assets.rs`, never renamed.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `src/surface.rs`, `src/render.rs`, `tests/fixtures/surface-today-{s1,s1-migrated,full}.json` | the bundle; one regeneration | 1 |
| `src/uievents.rs`, `src/lib.rs` | interaction events through `JsonlLedger` | 2 |
| `src/schedule.rs`, `src/runs.rs` (one new `pub fn`) | slot arithmetic; `runner_settings` (`device`, `scheduler`) | 3 |
| `src/backup.rs`, `Cargo.toml` (`zip`) | mirror, snapshots, prune, status | 4 |
| `src/history.rs` | git status, commit by name, sync without conflicts, the lock | 5 |
| `src/history.rs` (conflicts) | rebase conflicts → `reconcile::resolve` → `write` → `supersede` | 6 |
| `app/Cargo.toml`, `app/tauri.conf.json`, `app/capabilities/default.json`, `app/icons/icon-warn.ico`, `app/src/main.rs`, `app/src/tray.rs`, `app/src/state.rs`, `app/README.md`, `scripts/scratch-vault.ps1`, `scripts/make-tray-icons.py` | rename stage 1, tray, autostart, hide-to-tray, MessageBoxW, settings file | 7 |
| `app/src/commands.rs`, `app/tests/commands.rs` | `set_fields`, `create_task`, `delete_note` | 8 |
| `app/src/commands.rs`, `app/tests/commands.rs` | `decide`, `close_info`, `open_issue`, `resolve_issue` | 9 |
| `app/src/commands.rs`, `app/src/state.rs`, `app/tests/commands.rs` | `sync`, `backup_now`, `settings`, status in the envelope | 10 |
| `app/src/commands.rs`, `app/src/state.rs`, `app/tests/commands.rs` | `ui_event`, `mark_seen` semantics, real `engine_newer` | 11 |
| `app/src/scheduler.rs`, `app/src/main.rs`, `app/tests/scheduler.rs` | the tick thread, slot execution, run now, pause, missed count, tray state | 12 |
| `app/static/console.js`, `app/static/console.css`, `app/static/index.html` | click-to-edit, progress drag, new-task row, delete | 13 |
| `app/static/console.js`, `app/static/console.css` | deck actions, snooze, issue flag popover, info close | 14 |
| `app/static/console.js`, `app/static/console.css`, `app/static/index.html`, `app/tests/static_assets.rs` | sync/backup/run topline, Runs view, end-of-look `mark_seen`, event emission | 15 |
| `scripts/console-shots.py` (unchanged), `docs/surface/anatomy.md`, `app/README.md`, `docs/HANDOFF.md`, `CLAUDE.md` | screenshots, anatomy, handoff | 16 |
| `docs/runners/knowlu-go-live.md`, `docs/HANDOFF.md`, the Start-menu shortcut | the switch to the live vault after G2 (checklist, Quinn-gated) | 17 |

---

### Task 1: The surface bundle — one reference regeneration

**Files:**
- Modify: `src/surface.rs` (`runs_panel` ~1429–1470, `must_do` ~677, `verdict` ~286–340, `closed_this_week` ~1379–1406, `delta` ~1581–1626, `the_day` ~855–906, `State` ~1685, `build_state` ~1717, `RunsPanel` ~1422)
- Modify: `src/render.rs` (extract the runway fold into `pub fn runway_days`)
- Modify: `tests/fixtures/surface-today-s1.json`, `surface-today-s1-migrated.json`, `surface-today-full.json` (regenerated once, last step)
- Test: `src/surface.rs` `#[cfg(test)]`, `src/render.rs` `#[cfg(test)]`, `tests/oracle.rs` (unchanged, green), `tests/surface_oracle.rs` (unchanged, green after regeneration)

**Interfaces:**
- Consumes: `surface::local_stamp(vault, ts) -> String`; `runs::expected_status(vault, now) -> Result<Vec<StatusRow>, String>`; `Journal::warnings(&self) -> &[String]`; `EMPTY_TEXT.active`.
- Produces:
  - `pub fn render::runway_days(tasks: &[Task], today: Date) -> f64` — the one runway implementation; `render`'s own caller uses it; `surface::runway` deleted.
  - `RunsPanel.warnings: Vec<String>` (new field) — `expected_status`'s error, if any, verbatim.
  - `State.warnings: Vec<String>` (new field) — `journal.warnings()` collected from every `Journal` the builders open.
  - `Delta.truncated: bool` (new field); `delta.records` capped at `DELTA_RECORD_CAP = 200` newest.
  - `pub const DELTA_RECORD_CAP: usize = 200;`

- [ ] **Step 1: Write the failing tests** (append to `src/surface.rs`'s `mod tests`; the fixture helpers `fixture_full()`, `pinned_today()`, `pinned_now()` exist there since plan 1 — reuse them)

```rust
#[test]
fn expected_due_is_a_vault_local_stamp_not_utc() {
    // vault-full's config/ingest.yaml says America/Chicago; config/runners.yaml says local 12:00.
    let v = fixture_full();
    let now_ts: jiff::Timestamp = "2026-08-28T20:00:00Z".parse().unwrap();
    let p = runs_panel(&v, now_ts);
    let local = p.expected.iter().find(|e| e.runner == "local").expect("a local row");
    assert!(local.due.starts_with("2026-08-28 12:00"), "due was {:?}", local.due);
    assert!(!local.due.ends_with('Z'));
}

#[test]
fn a_missing_runners_config_is_a_warning_not_all_clear() {
    let v = fixture_full();
    std::fs::remove_file(v.join("config/runners.yaml")).unwrap();
    let p = runs_panel(&v, pinned_now().timestamp());
    assert!(p.expected.is_empty());
    assert!(p.warnings.is_empty(), "a missing file is Ok(vec![]) upstream — no warning");
    std::fs::write(v.join("config/runners.yaml"), "runners:\n  - name: 3\n").unwrap();
    let p = runs_panel(&v, pinned_now().timestamp());
    assert_eq!(p.warnings.len(), 1, "a malformed entry surfaces as one warning: {:?}", p.warnings);
}

#[test]
fn sums_never_print_negative_zero() {
    let v = fixture_full();
    for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
    let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
    let json = state_json(&s);
    assert!(!json.contains("-0.0"), "{json}");
}

#[test]
fn must_do_has_no_empty_text_beside_an_unreadable_row() {
    let v = fixture_full();
    for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
    std::fs::write(v.join("tasks/broken.md"), "---\ntitle: [\n---\n").unwrap();
    let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
    assert_eq!(s.unreadable, vec!["tasks/broken.md".to_string()]);
    assert!(s.must_do.empty_text.is_none(), "unreadable rows are the content; no empty text beside them");
}

#[test]
fn verdict_uses_the_shared_empty_string() {
    let v = fixture_full();
    for f in std::fs::read_dir(v.join("tasks")).unwrap().flatten() { std::fs::remove_file(f.path()).unwrap(); }
    let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
    assert_eq!(s.verdict.headline, EMPTY_TEXT.active);
}

#[test]
fn closed_this_week_is_one_row_per_note_and_bounded_by_now() {
    let v = fixture_full();
    let mut j = crate::journal::Journal::new(&v);
    let ctx = crate::write::WriteContext::new("quinn", "dashboard");
    let id = first_task_id(&v);
    // progress 100 + status done in ONE write → one journal moment → one row.
    crate::write::write_literals(&v, &id, &[("progress".into(), "100".into()), ("status".into(), "done".into())], &ctx, &mut j, &Default::default()).unwrap();
    let now = crate::journal::now_ts(None).parse::<jiff::Timestamp>().unwrap().to_zoned(jiff::tz::TimeZone::UTC).datetime();
    let rows = closed_this_week(&v, now, &mut j);
    assert_eq!(rows.iter().filter(|r| r.id.as_deref() == Some(id.as_str())).count(), 1);
    let past = now.checked_sub(jiff::Span::new().days(1)).unwrap();
    assert!(closed_this_week(&v, past, &mut j).iter().all(|r| r.id.as_deref() != Some(id.as_str())), "a record after `now` is not closed yet");
}

#[test]
fn delta_records_are_capped_and_say_so() {
    let v = fixture_full();
    let mut j = crate::journal::Journal::new(&v);
    let ctx = crate::write::WriteContext::new("quinn", "dashboard");
    let id = first_task_id(&v);
    for i in 0..(DELTA_RECORD_CAP + 5) {
        crate::write::write_literals(&v, &id, &[("importance".into(), ((i % 5) + 1).to_string())], &ctx, &mut j, &Default::default()).unwrap();
    }
    let d = delta(&v, pinned_now().datetime(), Some("2000-01-01T00:00:00.000Z"), &mut j);
    assert_eq!(d.records.len(), DELTA_RECORD_CAP);
    assert!(d.truncated);
}

#[test]
fn journal_warnings_reach_the_state() {
    let v = fixture_full();
    std::fs::write(v.join("state/journal/2026-08-27.jsonl"), "not json\r\n").unwrap();
    let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
    assert!(!s.warnings.is_empty(), "the ledger's parse warning must surface");
}

#[test]
fn open_hours_is_clamped_like_spare_hours() {
    let v = fixture_full();
    let s = build_state(&v, View::Today, pinned_today(), &pinned_now(), None);
    assert!(s.the_day.open_hours >= 0.0);
    assert!(s.recommended.spare_hours >= 0.0);
}

fn first_task_id(v: &std::path::Path) -> String {
    let s = build_state(v, View::Today, pinned_today(), &pinned_now(), None);
    s.must_do.groups[0].rows[0].id.clone()
}
```

And in `src/render.rs`'s tests:

```rust
#[test]
fn runway_days_is_the_single_implementation_used_by_the_page() {
    let (tasks, today) = fixture_tasks_and_date(); // existing helper in render tests
    let r = runway_days(&tasks, today);
    assert!(r.is_finite());
    assert_eq!(runway_days(&[], today), NO_DUE_DATE_SLACK as f64, "an empty vault seeds the fold the way render always did");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib surface::tests 2>&1 | tail -30` — expected: compile errors (`RunsPanel.warnings`, `State.warnings`, `Delta.truncated`, `DELTA_RECORD_CAP`, `render::runway_days` do not exist).

- [ ] **Step 3: Implement, item by item, keeping `tests/oracle.rs` green after each**

`src/render.rs` — extract the fold (visibility + extraction; the caller inside `render` keeps its exact output):

```rust
/// The runway in days: the minimum slack over active tasks, seeded with `NO_DUE_DATE_SLACK`
/// exactly as the page's status line has always computed it. ONE implementation — `surface`
/// calls this; it no longer has a twin that clamped at 999 (plan-1 ledger L92).
pub fn runway_days(tasks: &[crate::models::Task], today: jiff::civil::Date) -> f64 {
    tasks.iter()
        .map(|t| crate::scheduling::slack_days(t, today))
        .fold(crate::scheduling::NO_DUE_DATE_SLACK as f64, f64::min)
}
```
(Replace the inline fold at the one place `render` computes it with a call; `cargo test --test oracle` must still pass — if it does not, the extraction changed behaviour: fix the extraction.) In `surface.rs`, delete `fn runway` and call `crate::render::runway_days(&l.tasks, today)` from `topline` and `verdict`.

`src/surface.rs`:

```rust
pub const DELTA_RECORD_CAP: usize = 200;

/// `-0.0` prints as `-0.0` through `py_float`; an empty sum is `0.0`. Apply at every summed
/// hours field before it is stored (total_hours, hours per group, spare_hours, open_hours,
/// commitments_hours, undated_hours, bucket hours).
fn nz(x: f64) -> f64 { if x == 0.0 { 0.0 } else { x } }
```

- `runs_panel`: `ExpectedRow.due = local_stamp(vault, &row.due)`; replace `unwrap_or_default()` with a match that pushes the `Err(e)` string into `warnings` and uses an empty vec; add `pub warnings: Vec<String>` to `RunsPanel` (serialised; the page shows it in the Runs view, Task 15).
- `must_do` line ~677: `let empty_text = (must.is_empty() && cards.is_empty() && l.unreadable.is_empty()).then(...)`.
- `verdict`: the hardcoded `"Nothing active."` becomes `EMPTY_TEXT.active.to_string()`.
- `closed_this_week`: build a `BTreeMap<String, Closed>` keyed by the note path, keep the latest record per note, skip records with `ts > now` (compare the RFC-3339 string against `now` formatted `%Y-%m-%dT%H:%M:%S` — the records are UTC ISO strings, so convert `now` to UTC first as the existing code does for the week bound).
- `delta`: after collecting `records` sorted ascending by `ts`, `let truncated = records.len() > DELTA_RECORD_CAP; if truncated { records.drain(..records.len() - DELTA_RECORD_CAP); }`; add `pub truncated: bool` to `Delta`.
- `the_day`: `open_hours: nz(open_hours.max(0.0))`.
- `State`: add `pub warnings: Vec<String>`; in `build_state`, every place that constructs a `Journal` (`closed_this_week`, `delta`) returns its warnings — simplest: build one `Journal` in `build_state`, pass `&mut journal` to both (they already take one), then `warnings: journal.warnings().to_vec()` before computing `revision`. `revision` covers `warnings` (it is payload).
- Apply `nz` to every summed field named above.

- [ ] **Step 4: Run the tests**

Run: `cargo test 2>&1 | tail -5` — expected: the new tests pass; `tests/oracle.rs` passes; **`tests/surface_oracle.rs` FAILS three times** with the "regenerate IN A COMMIT THAT SHOWS THE DIFF" message. That failure is the point of the next step. 0 warnings.

- [ ] **Step 5: Regenerate the three references — from Git Bash, dev binary, the exact recipe**

```bash
cargo build 2>&1 | tail -1
for f in s1 s1-migrated full; do
  ./target/debug/quinn-ops.exe surface --vault tests/fixtures/vault-$f --view today --today 2026-08-28 --now 2026-08-28T09:00 --build-sha pinned | sed 's/$/\r/' > tests/fixtures/surface-today-$f.json
done
cargo test --test surface_oracle 2>&1 | tail -3
git diff --stat tests/fixtures/
```
Expected: three files changed, one line each (they are one-line files). Then read the diff of `surface-today-full.json` with `git diff tests/fixtures/surface-today-full.json | tr ',' '\n' | grep '^[-+]' | head -60` and confirm every changed key is one of: `expected[].due`, `warnings`, `truncated`, a `-0.0` → `0.0`, `open_hours`, `must_do.empty_text`, `verdict.headline`, `runway_days`/`status_word` (if the runway fold moved), `revision`. **Anything else is a bug in this task, not a reference to accept.**

- [ ] **Step 6: Dual-run scripts, pytest**

```powershell
.\scripts\diff-engines.ps1; .\scripts\diff-engines-notes.ps1; .venv\Scripts\python.exe -m pytest -q | Select-Object -Last 1
```
Expected: both scripts exit 0; `687 passed`.

- [ ] **Step 7: Commit — one commit, diff shows the change, message says why**

```bash
git add src/surface.rs src/render.rs tests/fixtures/surface-today-s1.json tests/fixtures/surface-today-s1-migrated.json tests/fixtures/surface-today-full.json
git commit -F <msgfile>   # "surface: the plan-1 bundle — expected.due in vault-local time, no -0.0, one runway fold, no empty_text beside unreadable rows, EMPTY_TEXT.active, closed one row per note, RunsPanel/State warnings, delta capped at 200, open_hours clamped; the three references regenerated because every one of those changes the payload (Knowlu plan 1, Task 1)"
```

---

### Task 2: `src/uievents.rs` — interaction events through the ledger seam

**Files:**
- Create: `src/uievents.rs`
- Modify: `src/lib.rs` (`pub mod uievents;`), `.gitignore` (`/state/events-ui/`)
- Test: `src/uievents.rs` `#[cfg(test)]`, `tests/uievents_isolation.rs`

**Interfaces:**
- Consumes: `ledger::JsonlLedger::new(root)`, `.append(&Record)`; `journal::now_ts(None)`, `journal::device_name()`.
- Produces:
  - `pub const ACTIONS: [&str; 11] = ["view_opened","object_seen","edit_started","edit_committed","edit_cancelled","decision_made","decision_deferred","issue_opened","sync_run","delta_expanded","why_expanded"];`
  - `pub struct UiEvent<'a> { pub session: &'a str, pub view: &'a str, pub action: &'a str, pub object_id: Option<&'a str>, pub object_kind: Option<&'a str>, pub ms: Option<i64> }`
  - `pub fn record(vault: &Path, ev: &UiEvent<'_>, now: Option<jiff::Timestamp>) -> Result<(), UiEventError>`
  - `pub enum UiEventError { UnknownAction(String), FreeText(&'static str), Ledger(String) }`
  - `pub fn commit_opt_in(vault: &Path) -> bool` — `config/planning.yaml`'s `commit_ui_events: true`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/uievents.rs  (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-uievents-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::write(d.join("config/planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
        d
    }
    #[test]
    fn an_event_is_one_sorted_python_json_line_in_the_utc_day_file() {
        let v = scratch();
        let ev = UiEvent { session: "s1", view: "today", action: "object_seen", object_id: Some("task_0123456789"), object_kind: Some("task"), ms: Some(2400) };
        record(&v, &ev, Some("2026-09-04T03:15:00Z".parse().unwrap())).unwrap();
        let text = std::fs::read_to_string(v.join("state/events-ui/2026-09-04.jsonl")).unwrap();
        assert_eq!(text.lines().count(), 1);
        let line = text.lines().next().unwrap();
        assert!(line.starts_with("{\"action\": \"object_seen\", \"device\": "), "{line}");
        assert!(line.contains("\"ms\": 2400, \"object_id\": \"task_0123456789\", \"object_kind\": \"task\", \"session\": \"s1\", \"ts\": \"2026-09-04T03:15:00.000Z\", \"view\": \"today\"}"), "{line}");
    }
    #[test]
    fn unknown_actions_and_free_text_are_refused() {
        let v = scratch();
        let bad = UiEvent { session: "s", view: "today", action: "typed_a_title", object_id: None, object_kind: None, ms: None };
        assert!(matches!(record(&v, &bad, None), Err(UiEventError::UnknownAction(_))));
        let text = UiEvent { session: "s", view: "today", action: "edit_started", object_id: Some("Read chapter 3"), object_kind: None, ms: None };
        assert!(matches!(record(&v, &text, None), Err(UiEventError::FreeText(_))), "an object_id that is not an id is free text");
        assert!(!v.join("state/events-ui").exists());
    }
    #[test]
    fn commit_opt_in_defaults_to_false() {
        let v = scratch();
        assert!(!commit_opt_in(&v));
        std::fs::write(v.join("config/planning.yaml"), "daily_effort_budget: 4.0\ncommit_ui_events: true\n").unwrap();
        assert!(commit_opt_in(&v));
    }
}
```

```rust
// tests/uievents_isolation.rs
//! Interaction events are NEVER read by the engine (S2 §7.6): no module other than the writer
//! mentions the folder or the module. Determinism of `rank` is untouched by construction.
#[test]
fn no_engine_module_but_the_writer_references_ui_events() {
    for entry in std::fs::read_dir("src").unwrap().flatten() {
        let p = entry.path();
        if p.extension().map(|e| e == "rs").unwrap_or(false) && p.file_name().unwrap() != "uievents.rs" && p.file_name().unwrap() != "lib.rs" {
            let text = std::fs::read_to_string(&p).unwrap();
            assert!(!text.contains("events-ui") && !text.contains("uievents::"), "{} references interaction events", p.display());
        }
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test uievents 2>&1 | tail -5` → `unresolved module uievents`.

- [ ] **Step 3: Implement**

```rust
//! Interaction events (S2 §7.6, console spec §7.5): what the page saw and what was clicked,
//! **ids only, never a title or a body**, through the same `JsonlLedger` the journal uses, to
//! `state/events-ui/YYYY-MM-DD.jsonl` (UTC day from the record's own `ts`, `merge=union`).
//! Git-ignored by default; `config/planning.yaml`'s `commit_ui_events: true` lets `history`
//! stage the folder. **Never read by the engine** — `tests/uievents_isolation.rs` proves it.
//! The console is the only writer.
use std::path::Path;
use serde_json::{json, Value};
use crate::ledger::{JsonlLedger, Record};

pub const ACTIONS: [&str; 11] = [
    "view_opened", "object_seen", "edit_started", "edit_committed", "edit_cancelled",
    "decision_made", "decision_deferred", "issue_opened", "sync_run", "delta_expanded", "why_expanded",
];

#[derive(Debug, Clone)]
pub struct UiEvent<'a> {
    pub session: &'a str,
    pub view: &'a str,
    pub action: &'a str,
    pub object_id: Option<&'a str>,
    pub object_kind: Option<&'a str>,
    pub ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEventError { UnknownAction(String), FreeText(&'static str), Ledger(String) }

impl std::fmt::Display for UiEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiEventError::UnknownAction(a) => write!(f, "unknown ui action {a:?}"),
            UiEventError::FreeText(which) => write!(f, "{which} must be an id, not text"),
            UiEventError::Ledger(e) => write!(f, "events-ui ledger: {e}"),
        }
    }
}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
}

pub fn record(vault: &Path, ev: &UiEvent<'_>, now: Option<jiff::Timestamp>) -> Result<(), UiEventError> {
    if !ACTIONS.contains(&ev.action) { return Err(UiEventError::UnknownAction(ev.action.to_string())); }
    if let Some(id) = ev.object_id { if !crate::ids::is_id(id) { return Err(UiEventError::FreeText("object_id")); } }
    if let Some(k) = ev.object_kind { if !is_token(k) { return Err(UiEventError::FreeText("object_kind")); } }
    if !is_token(ev.session) { return Err(UiEventError::FreeText("session")); }
    if !is_token(ev.view) { return Err(UiEventError::FreeText("view")); }
    let mut rec: Record = Record::new();
    rec.insert("ts".into(), json!(crate::journal::now_ts(now)));
    rec.insert("device".into(), json!(crate::journal::device_name()));
    rec.insert("session".into(), json!(ev.session));
    rec.insert("view".into(), json!(ev.view));
    rec.insert("action".into(), json!(ev.action));
    rec.insert("object_id".into(), ev.object_id.map(|s| json!(s)).unwrap_or(Value::Null));
    rec.insert("object_kind".into(), ev.object_kind.map(|s| json!(s)).unwrap_or(Value::Null));
    rec.insert("ms".into(), ev.ms.map(|m| json!(m)).unwrap_or(Value::Null));
    JsonlLedger::new(vault.join("state").join("events-ui")).append(&rec).map_err(|e| UiEventError::Ledger(format!("{e:?}")))
}

/// `config/planning.yaml` → `commit_ui_events: true`. Absent, unreadable or anything else → false.
pub fn commit_opt_in(vault: &Path) -> bool {
    crate::pystr::read_text(&vault.join("config").join("planning.yaml")).ok()
        .and_then(|t| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&t).ok())
        .and_then(|v| v.get("commit_ui_events").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}
```
Add `pub mod uievents;` to `src/lib.rs` and `/state/events-ui/` to `.gitignore` (CRLF file — append with the same line ending; check `git diff --stat .gitignore` shows one line).

- [ ] **Step 4: Run** — `cargo test uievents 2>&1 | tail -5` → 4 passed, 0 warnings. `cargo test --test oracle` green.
- [ ] **Step 5: Commit** — `git add src/uievents.rs src/lib.rs tests/uievents_isolation.rs .gitignore` — "engine: uievents — interaction events through the ledger seam, ids only, never read by the engine (Knowlu plan 1, Task 2)".

---

### Task 3: `src/schedule.rs` — slot arithmetic over run records, and the runner settings

**Files:**
- Create: `src/schedule.rs`
- Modify: `src/lib.rs`, `src/runs.rs` (one new `pub fn runner_settings`)
- Test: `src/schedule.rs` `#[cfg(test)]`, `src/runs.rs` tests

**Interfaces:**
- Consumes: `runs::RunnerConfig { name, times, tz, grace_minutes }`, `runs::load_runners_config(path)`, `runs::Runs::new(vault).read(since)` (records with `phase`, `runner`, `ts`).
- Produces:
  - `pub enum SchedulerMode { Script, App }` with `pub fn parse(s: Option<&str>) -> SchedulerMode` (absent/`"script"`/anything else → `Script`; exactly `"app"` → `App`).
  - `pub struct RunnerSettings { pub device: Option<String>, pub scheduler: SchedulerMode }`
  - `pub fn runs::runner_settings(path: &Path, name: &str) -> RunnerSettings` — the named entry's `device` and `scheduler` keys; both engines already ignore them.
  - `pub struct SlotDue { pub due: jiff::Zoned, pub late: bool }`
  - `pub fn schedule::due_slot(cfg: &RunnerConfig, now: &jiff::Zoned, starts: &[jiff::Timestamp]) -> Option<SlotDue>` — the earliest slot of `now`'s day (in `cfg.tz`) that is `<= now` and has no `start` in `[due − 5 min, next slot or end of day)`; `late = now > due + grace`.
  - `pub fn schedule::missed_slots(cfg: &RunnerConfig, from: &jiff::Zoned, to: &jiff::Zoned, starts: &[jiff::Timestamp]) -> usize` — slots between `from` and `to` with no start.
  - `pub fn schedule::run_starts(vault: &Path, runner: &str, since_date: jiff::civil::Date) -> Vec<jiff::Timestamp>` — from `state/runs/`, `phase == "start"`, `runner` matches.

- [ ] **Step 1: Write the failing tests**

```rust
// src/schedule.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::RunnerConfig;
    fn cfg() -> RunnerConfig { RunnerConfig { name: "local".into(), times: vec!["12:00".into(), "18:00".into()], tz: "America/Chicago".into(), grace_minutes: 20 } }
    fn at(s: &str) -> jiff::Zoned { s.parse::<jiff::civil::DateTime>().unwrap().to_zoned(jiff::tz::TimeZone::get("America/Chicago").unwrap()).unwrap() }
    fn ts(s: &str) -> jiff::Timestamp { at(s).timestamp() }

    #[test]
    fn nothing_is_due_before_the_first_slot() { assert!(due_slot(&cfg(), &at("2026-09-04T11:59"), &[]).is_none()); }

    #[test]
    fn the_first_slot_is_due_on_time_inside_grace_and_late_after() {
        let d = due_slot(&cfg(), &at("2026-09-04T12:05"), &[]).unwrap();
        assert_eq!(d.due, at("2026-09-04T12:00")); assert!(!d.late);
        let d = due_slot(&cfg(), &at("2026-09-04T12:21"), &[]).unwrap();
        assert!(d.late, "past grace is late, never skipped");
    }

    #[test]
    fn a_start_record_in_the_window_settles_the_slot() {
        let started = [ts("2026-09-04T12:03")];
        assert!(due_slot(&cfg(), &at("2026-09-04T13:00"), &started).is_none());
        let d = due_slot(&cfg(), &at("2026-09-04T19:34"), &started).unwrap();
        assert_eq!(d.due, at("2026-09-04T18:00"), "the 18:00 slot is still owed after wake");
    }

    #[test]
    fn a_start_five_minutes_early_counts_and_one_from_yesterday_does_not() {
        assert!(due_slot(&cfg(), &at("2026-09-04T12:30"), &[ts("2026-09-04T11:56")]).is_none());
        assert!(due_slot(&cfg(), &at("2026-09-04T12:30"), &[ts("2026-09-03T12:01")]).is_some());
    }

    #[test]
    fn the_earliest_owed_slot_wins_when_two_are_owed() {
        let d = due_slot(&cfg(), &at("2026-09-04T19:00"), &[]).unwrap();
        assert_eq!(d.due, at("2026-09-04T12:00"));
    }

    #[test]
    fn missed_slots_counts_the_quit_gap() {
        assert_eq!(missed_slots(&cfg(), &at("2026-09-03T13:00"), &at("2026-09-04T19:00"), &[ts("2026-09-04T12:10")]), 2, "09-03 18:00 and 09-04 18:00");
    }

    #[test]
    fn scheduler_mode_defaults_to_script() {
        assert_eq!(SchedulerMode::parse(None), SchedulerMode::Script);
        assert_eq!(SchedulerMode::parse(Some("script")), SchedulerMode::Script);
        assert_eq!(SchedulerMode::parse(Some("App")), SchedulerMode::Script, "exactly `app`, nothing else, turns the app scheduler on");
        assert_eq!(SchedulerMode::parse(Some("app")), SchedulerMode::App);
    }
}
```

```rust
// src/runs.rs tests
#[test]
fn runner_settings_reads_device_and_scheduler_and_load_runners_config_still_ignores_them() {
    let d = std::env::temp_dir().join(format!("qo-runners-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join("runners.yaml");
    std::fs::write(&p, "runners:\n  - name: cloud\n    times: [\"13:00\"]\n  - name: local\n    times: [\"12:00\", \"18:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: BOX-1\n    scheduler: app\n").unwrap();
    let s = runner_settings(&p, "local");
    assert_eq!(s.device.as_deref(), Some("BOX-1"));
    assert_eq!(s.scheduler, crate::schedule::SchedulerMode::App);
    assert_eq!(runner_settings(&p, "cloud").scheduler, crate::schedule::SchedulerMode::Script);
    assert_eq!(load_runners_config(&p).unwrap().len(), 2, "unknown keys in an entry are ignored (cutover plan Task 1)");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test schedule 2>&1 | tail -3` → unresolved module.

- [ ] **Step 3: Implement**

```rust
//! Slot arithmetic for the app's scheduler (Knowlu spec §3). Pure: the app's tick thread asks
//! "is a slot owed?" and this module answers from the runner config and the run records it is
//! handed. It never reads the clock and never writes. The console is the only caller.
use std::path::Path;
use jiff::{civil::Date, tz::TimeZone, Span, Timestamp, Zoned};
use crate::runs::RunnerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerMode { Script, App }

impl SchedulerMode {
    /// Absent or anything but the exact word `app` is `Script` (spec §3: the key is inert until
    /// phase 2 flips it, and a typo must never start a second runner).
    pub fn parse(s: Option<&str>) -> SchedulerMode { if s == Some("app") { SchedulerMode::App } else { SchedulerMode::Script } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotDue { pub due: Zoned, pub late: bool }

const EARLY_MINUTES: i64 = 5;

fn tz_of(cfg: &RunnerConfig) -> TimeZone { TimeZone::get(&cfg.tz).unwrap_or(TimeZone::UTC) }

fn slots_on(cfg: &RunnerConfig, day: Date) -> Vec<Zoned> {
    let tz = tz_of(cfg);
    let mut out: Vec<Zoned> = cfg.times.iter()
        .filter_map(|t| t.parse::<jiff::civil::Time>().ok())
        .filter_map(|t| day.to_datetime(t).to_zoned(tz.clone()).ok())
        .collect();
    out.sort();
    out
}

fn settled(due: &Zoned, window_end: &Zoned, starts: &[Timestamp]) -> bool {
    let lo = due.checked_sub(Span::new().minutes(EARLY_MINUTES)).map(|z| z.timestamp()).unwrap_or_else(|_| due.timestamp());
    let hi = window_end.timestamp();
    starts.iter().any(|s| *s >= lo && *s < hi)
}

pub fn due_slot(cfg: &RunnerConfig, now: &Zoned, starts: &[Timestamp]) -> Option<SlotDue> {
    let now = now.with_time_zone(tz_of(cfg));
    let slots = slots_on(cfg, now.date());
    let end_of_day = now.date().tomorrow().ok()?.to_datetime(jiff::civil::Time::midnight()).to_zoned(tz_of(cfg)).ok()?;
    for (i, due) in slots.iter().enumerate() {
        if *due > now { break; }
        let window_end = slots.get(i + 1).cloned().unwrap_or_else(|| end_of_day.clone());
        if settled(due, &window_end, starts) { continue; }
        let grace_end = due.checked_add(Span::new().minutes(cfg.grace_minutes)).unwrap_or_else(|_| due.clone());
        return Some(SlotDue { due: due.clone(), late: now > grace_end });
    }
    None
}

pub fn missed_slots(cfg: &RunnerConfig, from: &Zoned, to: &Zoned, starts: &[Timestamp]) -> usize {
    let tz = tz_of(cfg);
    let (from, to) = (from.with_time_zone(tz.clone()), to.with_time_zone(tz.clone()));
    let mut day = from.date();
    let mut missed = 0;
    while day <= to.date() {
        let slots = slots_on(cfg, day);
        let end_of_day = day.tomorrow().ok().and_then(|d| d.to_datetime(jiff::civil::Time::midnight()).to_zoned(tz.clone()).ok());
        for (i, due) in slots.iter().enumerate() {
            if *due <= from || *due > to { continue; }
            let window_end = slots.get(i + 1).cloned().or_else(|| end_of_day.clone()).unwrap_or_else(|| due.clone());
            if !settled(due, &window_end, starts) { missed += 1; }
        }
        day = match day.tomorrow() { Ok(d) => d, Err(_) => break };
    }
    missed
}

/// `start` records for `runner` since `since_date`, as timestamps. Reads `state/runs/` through
/// the runs ledger; never writes.
pub fn run_starts(vault: &Path, runner: &str, since_date: Date) -> Vec<Timestamp> {
    let mut runs = crate::runs::Runs::new(vault);
    let since = since_date.to_string();
    runs.read(Some(&since)).iter()
        .filter(|r| r.get("phase").and_then(|v| v.as_str()) == Some("start") && r.get("runner").and_then(|v| v.as_str()) == Some(runner))
        .filter_map(|r| r.get("ts").and_then(|v| v.as_str()).and_then(|s| s.parse::<Timestamp>().ok()))
        .collect()
}
```

`src/runs.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RunnerSettings { pub device: Option<String>, pub scheduler: crate::schedule::SchedulerMode }

/// The named runner entry's `device` and `scheduler` keys (cutover plan Task 1; Knowlu spec §3).
/// Neither engine's run reads them; `load_runners_config` ignores them (tested). A missing file,
/// entry or key yields the defaults: no device, `Script`.
pub fn runner_settings(path: &Path, name: &str) -> RunnerSettings {
    let default = RunnerSettings { device: None, scheduler: crate::schedule::SchedulerMode::Script };
    let Ok(text) = crate::pystr::read_text(path) else { return default };
    let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return default };
    let Some(entries) = doc.get("runners").and_then(|r| r.as_sequence()) else { return default };
    for e in entries {
        if e.get("name").and_then(|n| n.as_str()) == Some(name) {
            return RunnerSettings {
                device: e.get("device").and_then(|d| d.as_str()).map(|s| s.to_string()),
                scheduler: crate::schedule::SchedulerMode::parse(e.get("scheduler").and_then(|s| s.as_str())),
            };
        }
    }
    default
}
```
Add `pub mod schedule;` to `src/lib.rs`.

- [ ] **Step 4: Run** — `cargo test schedule runs 2>&1 | tail -3` → all pass, 0 warnings; oracle green.
- [ ] **Step 5: Commit** — `git add src/schedule.rs src/runs.rs src/lib.rs` — "engine: schedule — slot arithmetic over run records; runner_settings reads device and scheduler (absent = script) (Knowlu plan 1, Task 3)".

---

### Task 4: `src/backup.rs` — mirror, snapshots, prune, status

**Files:**
- Create: `src/backup.rs`
- Modify: `Cargo.toml` (`zip = { version = "2", default-features = false, features = ["deflate"] }`), `src/lib.rs`
- Test: `src/backup.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `ids::NOTE_FOLDERS`, `pystr`, `ledger::dumps_value` (for the status file).
- Produces:
  - `pub const BACKUP_FOLDERS: [&str; 9] = ["tasks","approvals","archive","courses","issues","info","state","config","profile"];`
  - `pub const SNAPSHOTS_KEPT: usize = 30;`
  - `pub struct MirrorReport { pub copied: usize, pub unchanged: usize, pub archived: usize, pub errors: Vec<String> }`
  - `pub fn mirror(vault: &Path, target: &Path, profile_id: &str) -> Result<MirrorReport, String>` — into `<target>/<profile_id>/vault/`; temp-and-rename; never deletes — a mirrored file whose source is gone moves to `<target>/<profile_id>/vault/archive/<rel>` (suffix `-<n>` on collision).
  - `pub fn snapshot(vault: &Path, target: &Path, profile_id: &str, today: jiff::civil::Date) -> Result<PathBuf, String>` — `<target>/<profile_id>/snapshots/YYYY-MM-DD.zip`, written to `.tmp` then renamed; idempotent per day (an existing file is left alone and returned).
  - `pub fn prune_snapshots(target: &Path, profile_id: &str, keep: usize) -> Result<Vec<PathBuf>, String>` — removes the oldest beyond `keep`; returns what it removed.
  - `pub struct BackupStatus { pub last_ok: Option<String>, pub behind_days: Option<i64>, pub last_error: Option<String>, pub target_reachable: bool }`
  - `pub fn status(target: &Path, profile_id: &str, now: jiff::Timestamp) -> BackupStatus` — from `<target>/<profile_id>/status.json` written by `tick`.
  - `pub fn tick(vault: &Path, target: &Path, profile_id: &str, now: jiff::Timestamp, today: jiff::civil::Date) -> BackupStatus` — mirror → snapshot → prune → status file; errors become `last_error`, never a panic.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-backup-src-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        copy_tree(std::path::Path::new("tests/fixtures/vault-full"), &d);
        d
    }
    fn target() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-backup-dst-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy_tree(&p, &t) } else { std::fs::copy(&p, &t).unwrap(); } }
    }
    fn tree_bytes(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() { let p = e.path(); if p.is_dir() { walk(root, &p, out) } else { out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(&p).unwrap()); } }
        }
        walk(root, root, &mut out); out
    }

    #[test]
    fn the_mirror_is_byte_identical_for_every_backup_folder_and_a_second_pass_copies_nothing() {
        let (v, t) = (fixture(), target());
        let r = mirror(&v, &t, "p1").unwrap();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let m = t.join("p1").join("vault");
        for f in BACKUP_FOLDERS { if v.join(f).is_dir() { assert_eq!(tree_bytes(&v.join(f)), tree_bytes(&m.join(f)), "{f}"); } }
        let again = mirror(&v, &t, "p1").unwrap();
        assert_eq!(again.copied, 0); assert!(again.unchanged > 0);
    }

    #[test]
    fn a_removed_note_moves_to_the_mirrors_archive_and_nothing_is_ever_deleted() {
        let (v, t) = (fixture(), target());
        mirror(&v, &t, "p1").unwrap();
        let victim = std::fs::read_dir(v.join("tasks")).unwrap().flatten().next().unwrap().path();
        let name = victim.file_name().unwrap().to_owned();
        std::fs::remove_file(&victim).unwrap();
        let r = mirror(&v, &t, "p1").unwrap();
        assert_eq!(r.archived, 1);
        assert!(!t.join("p1/vault/tasks").join(&name).exists());
        assert!(t.join("p1/vault/archive/tasks").join(&name).exists(), "the mirror keeps its own archive");
    }

    #[test]
    fn a_crash_between_temp_and_rename_never_leaves_a_half_note_at_the_final_path() {
        let (v, t) = (fixture(), target());
        mirror(&v, &t, "p1").unwrap();
        let m = t.join("p1/vault/tasks");
        std::fs::write(m.join("stray.md.tmp-999"), "half").unwrap();   // a leftover temp from a dead process
        let r = mirror(&v, &t, "p1").unwrap();
        assert!(r.errors.is_empty());
        assert!(!m.join("stray.md.tmp-999").exists(), "leftover temps are swept");
        assert!(!m.join("stray.md").exists());
    }

    #[test]
    fn snapshots_are_one_per_day_and_pruned_to_the_newest_thirty() {
        let (v, t) = (fixture(), target());
        let today: jiff::civil::Date = "2026-09-04".parse().unwrap();
        let z = snapshot(&v, &t, "p1", today).unwrap();
        assert!(z.ends_with("snapshots/2026-09-04.zip") || z.ends_with("snapshots\\2026-09-04.zip"));
        let size = std::fs::metadata(&z).unwrap().len();
        assert_eq!(snapshot(&v, &t, "p1", today).unwrap(), z, "idempotent per day");
        assert_eq!(std::fs::metadata(&z).unwrap().len(), size);
        for i in 1..=35u32 { std::fs::write(t.join("p1/snapshots").join(format!("2026-07-{:02}.zip", i.min(31))), b"x").unwrap(); }
        let removed = prune_snapshots(&t, "p1", SNAPSHOTS_KEPT).unwrap();
        let left = std::fs::read_dir(t.join("p1/snapshots")).unwrap().count();
        assert_eq!(left, SNAPSHOTS_KEPT);
        assert!(removed.iter().all(|p| p.file_name().unwrap().to_string_lossy() < "2026-07-03".to_string()));
    }

    #[test]
    fn an_unreachable_target_is_amber_in_status_never_an_error_out_of_tick() {
        let v = fixture();
        let gone = std::path::Path::new(r"Q:\no-such-drive\knowlu-backup");
        let now: jiff::Timestamp = "2026-09-04T18:00:00Z".parse().unwrap();
        let s = tick(&v, gone, "p1", now, "2026-09-04".parse().unwrap());
        assert!(!s.target_reachable); assert!(s.last_error.is_some()); assert!(s.last_ok.is_none());
    }

    #[test]
    fn status_reports_days_behind_from_the_status_file() {
        let (v, t) = (fixture(), target());
        let then: jiff::Timestamp = "2026-09-01T18:00:00Z".parse().unwrap();
        tick(&v, &t, "p1", then, "2026-09-01".parse().unwrap());
        let s = status(&t, "p1", "2026-09-04T18:00:00Z".parse().unwrap());
        assert_eq!(s.behind_days, Some(3));
        assert!(s.target_reachable);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test backup 2>&1 | tail -3` → unresolved module.

- [ ] **Step 3: Implement**

```rust
//! Backup (Knowlu spec §4): a live **mirror** of the vault's folders under a folder the user
//! owns, copied on content change with temp-and-rename, **never deleting** — a source that
//! disappears moves to the mirror's own `archive/` — plus one dated zip **snapshot** a day,
//! `SNAPSHOTS_KEPT` retained. Failure is a status, never a panic; the console paints it amber.
//! The console is the only caller. Credentials are never a rule here: the vault does not hold
//! them (the keychain does); `config/` is copied as the user's own data to the user's own storage.
use std::io::Write;
use std::path::{Path, PathBuf};
use serde_json::json;

pub const BACKUP_FOLDERS: [&str; 9] = ["tasks", "approvals", "archive", "courses", "issues", "info", "state", "config", "profile"];
pub const SNAPSHOTS_KEPT: usize = 30;
const TMP_MARK: &str = ".tmp-";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MirrorReport { pub copied: usize, pub unchanged: usize, pub archived: usize, pub errors: Vec<String> }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BackupStatus { pub last_ok: Option<String>, pub behind_days: Option<i64>, pub last_error: Option<String>, pub target_reachable: bool }

fn profile_root(target: &Path, profile_id: &str) -> PathBuf { target.join(profile_id) }

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() { let p = e.path(); if p.is_dir() { walk(&p, out) } else { out.push(p) } }
    }
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.len() == mb.len() => std::fs::read(a).ok() == std::fs::read(b).ok(),
        _ => false,
    }
}

fn place(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?; }
    let tmp = dst.with_file_name(format!("{}{TMP_MARK}{}", dst.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
    std::fs::copy(src, &tmp).map_err(|e| format!("copy {}: {e}", src.display()))?;
    std::fs::rename(&tmp, dst).map_err(|e| { let _ = std::fs::remove_file(&tmp); format!("rename {}: {e}", dst.display()) })
}

fn free_name(dir: &Path, name: &str) -> PathBuf {
    let mut p = dir.join(name);
    let (stem, ext) = match name.rsplit_once('.') { Some((s, e)) => (s.to_string(), format!(".{e}")), None => (name.to_string(), String::new()) };
    let mut n = 2;
    while p.exists() { p = dir.join(format!("{stem}-{n}{ext}")); n += 1; }
    p
}

pub fn mirror(vault: &Path, target: &Path, profile_id: &str) -> Result<MirrorReport, String> {
    let root = profile_root(target, profile_id).join("vault");
    std::fs::create_dir_all(&root).map_err(|e| format!("backup target {}: {e}", root.display()))?;
    let mut rep = MirrorReport::default();
    // 1. sweep leftover temps from a dead process
    let mut existing = Vec::new(); walk(&root, &mut existing);
    for p in &existing { if p.file_name().map(|n| n.to_string_lossy().contains(TMP_MARK)).unwrap_or(false) { let _ = std::fs::remove_file(p); } }
    // 2. copy on content change
    let mut wanted = std::collections::BTreeSet::new();
    for folder in BACKUP_FOLDERS {
        let src_dir = vault.join(folder);
        if !src_dir.is_dir() { continue; }
        let mut files = Vec::new(); walk(&src_dir, &mut files);
        for src in files {
            let rel = src.strip_prefix(vault).map_err(|e| e.to_string())?.to_path_buf();
            let dst = root.join(&rel);
            wanted.insert(rel.clone());
            if same_bytes(&src, &dst) { rep.unchanged += 1; continue; }
            match place(&src, &dst) { Ok(()) => rep.copied += 1, Err(e) => rep.errors.push(e) }
        }
    }
    // 3. archive what the vault no longer has (never under the mirror's own archive/)
    let mut mirrored = Vec::new(); walk(&root, &mut mirrored);
    for m in mirrored {
        let rel = m.strip_prefix(&root).map_err(|e| e.to_string())?.to_path_buf();
        if rel.starts_with("archive") || wanted.contains(&rel) { continue; }
        let dst_dir = root.join("archive").join(rel.parent().unwrap_or(Path::new("")));
        if std::fs::create_dir_all(&dst_dir).is_ok() {
            let dst = free_name(&dst_dir, &rel.file_name().unwrap_or_default().to_string_lossy());
            match std::fs::rename(&m, &dst) { Ok(()) => rep.archived += 1, Err(e) => rep.errors.push(format!("archive {}: {e}", m.display())) }
        }
    }
    Ok(rep)
}

pub fn snapshot(vault: &Path, target: &Path, profile_id: &str, today: jiff::civil::Date) -> Result<PathBuf, String> {
    let dir = profile_root(target, profile_id).join("snapshots");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let out = dir.join(format!("{today}.zip"));
    if out.exists() { return Ok(out); }
    let tmp = dir.join(format!("{today}.zip{TMP_MARK}{}", std::process::id()));
    {
        let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for folder in BACKUP_FOLDERS {
            let src_dir = vault.join(folder);
            if !src_dir.is_dir() { continue; }
            let mut files = Vec::new(); walk(&src_dir, &mut files);
            for src in files {
                let rel = src.strip_prefix(vault).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                zip.start_file(rel, opts).map_err(|e| e.to_string())?;
                zip.write_all(&std::fs::read(&src).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, &out).map_err(|e| { let _ = std::fs::remove_file(&tmp); e.to_string() })?;
    Ok(out)
}

pub fn prune_snapshots(target: &Path, profile_id: &str, keep: usize) -> Result<Vec<PathBuf>, String> {
    let dir = profile_root(target, profile_id).join("snapshots");
    let mut zips: Vec<PathBuf> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "zip").unwrap_or(false)).collect();
    zips.sort();
    let mut removed = Vec::new();
    while zips.len() > keep { let p = zips.remove(0); std::fs::remove_file(&p).map_err(|e| e.to_string())?; removed.push(p); }
    Ok(removed)
}

fn status_path(target: &Path, profile_id: &str) -> PathBuf { profile_root(target, profile_id).join("status.json") }

pub fn status(target: &Path, profile_id: &str, now: jiff::Timestamp) -> BackupStatus {
    let reachable = target.is_dir();
    let doc = std::fs::read_to_string(status_path(target, profile_id)).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let last_ok = doc.as_ref().and_then(|d| d.get("last_ok")).and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_error = doc.as_ref().and_then(|d| d.get("last_error")).and_then(|v| v.as_str()).map(|s| s.to_string());
    let behind_days = last_ok.as_deref().and_then(|s| s.parse::<jiff::Timestamp>().ok()).map(|t| (now.as_second() - t.as_second()) / 86_400);
    BackupStatus { last_ok, behind_days, last_error, target_reachable: reachable }
}

pub fn tick(vault: &Path, target: &Path, profile_id: &str, now: jiff::Timestamp, today: jiff::civil::Date) -> BackupStatus {
    let stamp = crate::journal::now_ts(Some(now));
    let result = mirror(vault, target, profile_id)
        .and_then(|r| if r.errors.is_empty() { Ok(()) } else { Err(r.errors.join("; ")) })
        .and_then(|_| snapshot(vault, target, profile_id, today).map(|_| ()))
        .and_then(|_| prune_snapshots(target, profile_id, SNAPSHOTS_KEPT).map(|_| ()));
    let prior = status(target, profile_id, now);
    let (last_ok, last_error) = match result { Ok(()) => (Some(stamp), None), Err(e) => (prior.last_ok, Some(e)) };
    let doc = json!({ "last_ok": last_ok, "last_error": last_error });
    if target.is_dir() {
        if let Some(p) = status_path(target, profile_id).parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(status_path(target, profile_id), crate::ledger::dumps_value(&doc));
    }
    status(target, profile_id, now)
}
```
`Cargo.toml`: add the `zip` line under `[dependencies]`. `src/lib.rs`: `pub mod backup;`. The first `cargo build` fetches `zip` and `flate2`; check `cargo build --release` afterwards and note the binary size in the commit message (budget line).

- [ ] **Step 4: Run** — `cargo test backup 2>&1 | tail -3` → 6 passed, 0 warnings; oracle green; `cargo build --release` and `(Get-Item target\release\quinn-ops.exe).Length`.
- [ ] **Step 5: Commit** — `git add src/backup.rs src/lib.rs Cargo.toml Cargo.lock` — "engine: backup — mirror with temp-and-rename that never deletes, dated zip snapshots kept 30, amber status (Knowlu plan 1, Task 4; release binary N MiB)".

---
### Task 5: `src/history.rs` — git as a transport: status, commit by name, sync without conflicts, the lock

**Files:**
- Create: `src/history.rs`
- Modify: `src/lib.rs`
- Test: `src/history.rs` `#[cfg(test)]` (temporary bare repository as the remote)

**Interfaces:**
- Consumes: `std::process::Command` (`git`); `write::WriteContext`; `journal::Journal`; `uievents::commit_opt_in`.
- Produces:
  - `pub const STAGED_PATHS: [&str; 7] = ["tasks","approvals","archive","courses","issues","info","state"];`
  - `pub struct HistoryStatus { pub is_repo: bool, pub has_remote: bool, pub ahead: i64, pub behind: i64, pub dirty: bool, pub last_error: Option<String>, pub conflicted: Vec<String> }` (`Serialize`, `Default`)
  - `pub fn git(vault: &Path, args: &[&str]) -> Result<String, String>` — stdout trimmed on success; `Err(stderr)` on a non-zero exit; never inherits the console's stdio.
  - `pub fn status(vault: &Path) -> HistoryStatus`
  - `pub fn commit_by_name(vault: &Path, message: &str) -> Result<bool, String>` — `git add` each of `STAGED_PATHS` that exists (plus `-f state/events-ui` when `commit_opt_in`), commit if staged; `Ok(true)` if a commit was made.
  - `pub struct SyncOutcome { pub status: HistoryStatus, pub committed: bool, pub pulled: bool, pub pushed: bool }`
  - `pub fn sync(vault: &Path, ctx: &WriteContext, journal: &mut Journal, edits: usize) -> SyncOutcome` — the console spec §8 loop; conflicts delegate to `resolve_conflicts` (Task 6; in this task a conflict aborts the rebase and reports `conflicted`).
  - `pub struct SyncLock { path: PathBuf }` with `pub fn acquire(vault: &Path) -> Result<SyncLock, String>` (`state/.sync.lock`, `pid ts`, stale after 120 s; `Drop` removes it).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn git_env() { std::env::set_var("GIT_AUTHOR_NAME", "test"); std::env::set_var("GIT_AUTHOR_EMAIL", "t@localhost"); std::env::set_var("GIT_COMMITTER_NAME", "test"); std::env::set_var("GIT_COMMITTER_EMAIL", "t@localhost"); }
    fn copy_tree(from: &Path, to: &Path) { std::fs::create_dir_all(to).unwrap(); for e in std::fs::read_dir(from).unwrap().flatten() { let p = e.path(); let t = to.join(e.file_name()); if p.is_dir() { copy_tree(&p, &t) } else { std::fs::copy(&p, &t).unwrap(); } } }
    /// A bare remote + a working vault cloned from the fixture, on branch `main`.
    pub(crate) fn repo_pair(tag: &str) -> (PathBuf, PathBuf) {
        git_env();
        let base = std::env::temp_dir().join(format!("qo-history-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let bare = base.join("remote.git"); let work = base.join("vault");
        std::fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "--bare", "--initial-branch=main"]).unwrap();
        copy_tree(Path::new("tests/fixtures/vault-full"), &work);
        git(&work, &["init", "--initial-branch=main"]).unwrap();
        git(&work, &["add", "."]).unwrap();                      // test setup only — the module never uses -A or `.`
        git(&work, &["commit", "-q", "-m", "fixture"]).unwrap();
        git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]).unwrap();
        git(&work, &["push", "-q", "-u", "origin", "main"]).unwrap();
        (bare, work)
    }
    fn edit(work: &Path, field: &str, value: &str) {
        let mut j = crate::journal::Journal::new(work);
        let ctx = crate::write::WriteContext::new("quinn", "dashboard");
        crate::write::write_literals(work, "tasks/ph-106-exam-1-prep.md", &[(field.into(), value.into())], &ctx, &mut j, &Default::default()).unwrap();
    }

    #[test]
    fn a_plain_folder_is_not_a_repo_and_that_is_not_an_error() {
        let d = std::env::temp_dir().join(format!("qo-history-plain-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let s = status(&d);
        assert!(!s.is_repo && !s.has_remote && s.last_error.is_none(), "{s:?}");
    }

    #[test]
    fn commit_by_name_stages_only_the_named_paths() {
        let (_bare, work) = repo_pair("names");
        edit(&work, "importance", "4");
        std::fs::write(work.join("scratch.txt"), "not staged").unwrap();
        assert!(commit_by_name(&work, "surface: 1 edit").unwrap());
        let shown = git(&work, &["show", "--stat", "--format=", "HEAD"]).unwrap();
        assert!(shown.contains("tasks/ph-106-exam-1-prep.md") && shown.contains("state/journal/"), "{shown}");
        assert!(!shown.contains("scratch.txt"));
        assert!(!commit_by_name(&work, "surface: 0 edits").unwrap(), "nothing staged, no commit");
    }

    #[test]
    fn only_ahead_pushes_and_never_rebases() {
        let (bare, work) = repo_pair("ahead");
        edit(&work, "importance", "2");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && out.pushed && !out.pulled, "{out:?}");
        assert_eq!(git(&bare, &["rev-parse", "main"]).unwrap(), git(&work, &["rev-parse", "HEAD"]).unwrap());
        assert_eq!(out.status.ahead, 0);
    }

    #[test]
    fn behind_without_overlap_pulls_with_rebase_then_pushes() {
        let (bare, work) = repo_pair("behind");
        let other = work.parent().unwrap().join("other");
        git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
        std::fs::write(other.join("info").join("from-other.md"), "---\ntype: info\nid: info_0000000001\nstatus: open\ntitle: x\n---\n").unwrap();
        git(&other, &["add", "info"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
        edit(&work, "importance", "1");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.pulled && out.pushed, "{out:?}");
        assert!(work.join("info/from-other.md").exists());
        assert!(out.status.conflicted.is_empty());
    }

    #[test]
    fn a_dead_remote_is_amber_and_the_commit_stands() {
        let (bare, work) = repo_pair("dead");
        std::fs::remove_dir_all(&bare).unwrap();
        edit(&work, "importance", "3");
        let mut j = crate::journal::Journal::new(&work);
        let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
        assert!(out.committed && !out.pushed);
        assert!(out.status.last_error.is_some());
        assert_eq!(out.status.ahead, 1, "the edit is safe locally and waits");
    }

    #[test]
    fn the_lock_is_exclusive_and_stale_after_two_minutes() {
        let (_b, work) = repo_pair("lock");
        let l = SyncLock::acquire(&work).unwrap();
        assert!(SyncLock::acquire(&work).is_err());
        drop(l);
        std::fs::write(work.join("state/.sync.lock"), "99999 2000-01-01T00:00:00.000Z").unwrap();
        assert!(SyncLock::acquire(&work).is_ok(), "a stale lock is taken over");
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test history 2>&1 | tail -3` → unresolved module.

- [ ] **Step 3: Implement**

```rust
//! History (Knowlu spec §4; console spec §8; S2 §10): git as a **transport**, driven only by the
//! console. Commit by name, never `-A`; only-ahead means push, never rebase; behind means
//! `pull --rebase` then push; a conflict goes through `reconcile` (Task 6); offline is amber and
//! the commit stands. The journal is the history — when the remote goes, so does this module's
//! reason to run, and a vault that is not a repository is a normal state, not an error.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::journal::Journal;
use crate::write::WriteContext;

pub const STAGED_PATHS: [&str; 7] = ["tasks", "approvals", "archive", "courses", "issues", "info", "state"];
const LOCK_STALE_SECS: i64 = 120;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct HistoryStatus {
    pub is_repo: bool, pub has_remote: bool, pub ahead: i64, pub behind: i64, pub dirty: bool,
    pub last_error: Option<String>, pub conflicted: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOutcome { pub status: HistoryStatus, pub committed: bool, pub pulled: bool, pub pushed: bool }

pub fn git(vault: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").current_dir(vault).args(["-c", "commit.gpgsign=false", "-c", "core.editor=true"]).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output().map_err(|e| format!("git: {e}"))?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()) }
    else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

fn counts(vault: &Path) -> Result<(i64, i64), String> {
    let s = git(vault, &["rev-list", "--left-right", "--count", "HEAD...origin/main"])?;
    let mut it = s.split_whitespace().map(|n| n.parse::<i64>().unwrap_or(0));
    Ok((it.next().unwrap_or(0), it.next().unwrap_or(0)))
}

pub fn status(vault: &Path) -> HistoryStatus {
    let mut st = HistoryStatus::default();
    if git(vault, &["rev-parse", "--is-inside-work-tree"]).map(|s| s == "true").unwrap_or(false) { st.is_repo = true; } else { return st; }
    st.has_remote = git(vault, &["remote", "get-url", "origin"]).is_ok();
    st.dirty = !git(vault, &["status", "--porcelain", "--", "tasks", "approvals", "archive", "courses", "issues", "info", "state"]).unwrap_or_default().is_empty();
    if st.has_remote { if let Ok((a, b)) = counts(vault) { st.ahead = a; st.behind = b; } }
    st.conflicted = git(vault, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default().lines().map(|s| s.to_string()).collect();
    st
}

pub fn commit_by_name(vault: &Path, message: &str) -> Result<bool, String> {
    for p in STAGED_PATHS { if vault.join(p).exists() { git(vault, &["add", "--", p])?; } }
    if crate::uievents::commit_opt_in(vault) && vault.join("state/events-ui").exists() { git(vault, &["add", "-f", "--", "state/events-ui"])?; }
    if git(vault, &["diff", "--cached", "--quiet"]).is_ok() { return Ok(false); }
    git(vault, &["commit", "-q", "-m", message])?;
    Ok(true)
}

pub fn sync(vault: &Path, ctx: &WriteContext, journal: &mut Journal, edits: usize) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    let _lock = match SyncLock::acquire(vault) { Ok(l) => l, Err(e) => { out.status = status(vault); out.status.last_error = Some(e); return out; } };
    out.status = status(vault);
    if !out.status.is_repo { return out; }
    match commit_by_name(vault, &format!("surface: {edits} edit{}", if edits == 1 { "" } else { "s" })) {
        Ok(c) => out.committed = c,
        Err(e) => { out.status.last_error = Some(e); return out; }
    }
    if !out.status.has_remote { out.status = status(vault); return out; }
    if let Err(e) = git(vault, &["fetch", "-q", "origin"]) { out.status = status(vault); out.status.last_error = Some(e); return out; }
    let (ahead, behind) = match counts(vault) { Ok(c) => c, Err(e) => { out.status.last_error = Some(e); return out; } };
    if behind > 0 {
        match git(vault, &["rebase", "-q", "origin/main"]) {
            Ok(_) => out.pulled = true,
            Err(e) => {
                let settled = super::history::resolve_conflicts(vault, ctx, journal); // Task 6; until then: Err
                match settled {
                    Ok(_) => out.pulled = true,
                    Err(conflict) => {
                        let names = status(vault).conflicted;
                        let _ = git(vault, &["rebase", "--abort"]);
                        out.status = status(vault);
                        out.status.conflicted = names;
                        out.status.last_error = Some(format!("conflict: {conflict}; auto-sync stopped ({e})"));
                        return out;
                    }
                }
            }
        }
    }
    if ahead > 0 || out.committed || out.pulled {
        match git(vault, &["push", "-q", "origin", "HEAD:main"]) { Ok(_) => out.pushed = true, Err(e) => { out.status = status(vault); out.status.last_error = Some(e); return out; } }
    }
    out.status = status(vault);
    out
}

/// Task 6 replaces this body. Until then every conflict is reported, never resolved.
pub fn resolve_conflicts(_vault: &Path, _ctx: &WriteContext, _journal: &mut Journal) -> Result<Vec<String>, String> {
    Err("conflict resolution not built yet".into())
}

pub struct SyncLock { path: PathBuf }

impl SyncLock {
    pub fn acquire(vault: &Path) -> Result<SyncLock, String> {
        let path = vault.join("state").join(".sync.lock");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let ts = text.split_whitespace().nth(1).and_then(|t| t.parse::<jiff::Timestamp>().ok());
            let fresh = ts.map(|t| jiff::Timestamp::now().as_second() - t.as_second() < LOCK_STALE_SECS).unwrap_or(false);
            if fresh { return Err(format!("another sync holds {}", path.display())); }
        }
        if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        std::fs::write(&path, format!("{} {}", std::process::id(), crate::journal::now_ts(None))).map_err(|e| e.to_string())?;
        Ok(SyncLock { path })
    }
}
impl Drop for SyncLock { fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); } }
```
Add `pub mod history;` to `src/lib.rs`. (`state/.sync.lock` must never be committed: confirm `.gitignore` gets `/state/.sync.lock`, CRLF.)

- [ ] **Step 4: Run** — `cargo test history 2>&1 | tail -3` → 6 passed, 0 warnings; oracle green.
- [ ] **Step 5: Commit** — `git add src/history.rs src/lib.rs .gitignore` — "engine: history — git as a transport: status, commit by name, only-ahead push, rebase when behind, amber offline, the sync lock (Knowlu plan 1, Task 5)".

---

### Task 6: `history::resolve_conflicts` — rebase conflicts through `reconcile`, first caller since wave 3

**Files:**
- Modify: `src/history.rs`
- Test: `src/history.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `reconcile::resolve(upstream_meta, upstream_records, local_records, upstream_mtime_ts, note_id, path, via) -> Resolution { apply, supersede, notes }`; `write::write(vault, target, changes, ctx, journal, opts)`; `journal.records_for(note_id, None)`; `journal.append(&mut rec)`; `ids::read_meta(path)`; `journal::device_name()`.
- Produces: `pub fn resolve_conflicts(vault, ctx, journal) -> Result<Vec<String>, String>` — for every conflicted `.md` under a note folder: upstream wins the file, this device's replayed records go through `reconcile`, winners re-applied through `write`, `supersede` records appended, `git add`, `rebase --continue`; ledgers (`state/**`) are `merge=union` and never reach this path — a conflicted ledger is an `Err`. Returns the settled paths.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_same_field_conflict_is_settled_by_reconcile_and_leaves_a_supersede_record() {
    let (bare, work) = repo_pair("conflict");
    let other = work.parent().unwrap().join("other");
    git(&bare, &["clone", "-q", bare.to_str().unwrap(), other.to_str().unwrap()]).unwrap();
    // The other device edits `due` first and pushes; this device edits `due` later, then syncs.
    std::env::set_var("QUINN_OPS_DEVICE", "other-box");
    edit(&other, "due", "2026-10-01T09:00");
    git(&other, &["add", "tasks", "state"]).unwrap(); git(&other, &["commit", "-q", "-m", "other"]).unwrap(); git(&other, &["push", "-q"]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::env::set_var("QUINN_OPS_DEVICE", "this-box");
    edit(&work, "due", "2026-10-02T09:00");
    let mut j = crate::journal::Journal::new(&work);
    let out = sync(&work, &crate::write::WriteContext::new("quinn", "dashboard"), &mut j, 1);
    assert!(out.status.conflicted.is_empty(), "{out:?}");
    assert!(out.pushed);
    let meta = crate::ids::read_meta(&work.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    assert_eq!(crate::approvals::plain(meta.get("due").unwrap()), "2026-10-02T09:00", "the later record wins");
    let mut j = crate::journal::Journal::new(&work);
    let id = crate::approvals::plain(meta.get("id").unwrap());
    assert!(j.records_for(&id, Some("due")).iter().any(|r| r.get("op").and_then(|o| o.as_str()) == Some("supersede")), "the loser is superseded, never silently dropped");
    std::env::remove_var("QUINN_OPS_DEVICE");
}
```
(The `sleep` makes the two `ts` values distinct to the millisecond; `reconcile::wins` orders by `ts`. If `reconcile` ranks otherwise, the assertion on the value flips — read `reconcile::wins` before adjusting the expected value; **never adjust `reconcile`**.)

- [ ] **Step 2: Run to verify failure** — the sync reports `conflicted` and the test fails on the first assertion.

- [ ] **Step 3: Implement** (replace the stub)

```rust
fn is_note_path(p: &str) -> bool {
    p.ends_with(".md") && crate::ids::NOTE_FOLDERS.iter().any(|f| p.starts_with(&format!("{f}/")))
}

/// Journal `ts` strings compare lexically (UTC, fixed width). `%cI` from git is `+00:00`-suffixed
/// local time; normalise to the journal's `YYYY-MM-DDTHH:MM:SS.000Z`.
fn git_time_to_ts(iso: &str) -> String {
    iso.parse::<jiff::Timestamp>().map(|t| crate::journal::now_ts(Some(t))).unwrap_or_else(|_| iso.to_string())
}

pub fn resolve_conflicts(vault: &Path, ctx: &WriteContext, journal: &mut Journal) -> Result<Vec<String>, String> {
    let mut settled = Vec::new();
    loop {
        let conflicted: Vec<String> = git(vault, &["diff", "--name-only", "--diff-filter=U"])?.lines().map(|s| s.to_string()).collect();
        if conflicted.is_empty() { break; }
        if let Some(bad) = conflicted.iter().find(|p| !is_note_path(p)) {
            return Err(format!("{bad} conflicted and is not a note (ledgers are merge=union)"));
        }
        let this_device = crate::journal::device_name();
        let base_ts = git_time_to_ts(&git(vault, &["log", "-1", "--format=%cI", "origin/main"])?);
        for path in &conflicted {
            // During a rebase, `--ours` is the branch being rebased onto: upstream wins the file.
            git(vault, &["checkout", "--ours", "--", path])?;
            let upstream_meta = crate::ids::read_meta(&vault.join(path)).ok_or_else(|| format!("{path}: unreadable after taking upstream"))?;
            let note_id = upstream_meta.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let upstream_mtime = git_time_to_ts(&git(vault, &["log", "-1", "--format=%cI", "origin/main", "--", path])?);
            journal.invalidate();
            let all = note_id.as_deref().map(|id| journal.records_for(id, None)).unwrap_or_default();
            let ts_of = |r: &crate::ledger::Record| r.get("ts").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let dev_of = |r: &crate::ledger::Record| r.get("device").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let local: Vec<_> = all.iter().filter(|r| dev_of(r) == this_device && ts_of(r) > base_ts).cloned().collect();
            let upstream: Vec<_> = all.iter().filter(|r| dev_of(r) != this_device && ts_of(r) > base_ts).cloned().collect();
            let res = crate::reconcile::resolve(&upstream_meta, &upstream, &local, &upstream_mtime, note_id.as_deref(), path, &ctx.via);
            if !res.apply.is_empty() {
                let changes: Vec<(String, serde_yaml_ng::Value)> = res.apply.iter().map(|(k, v)| (k.clone(), crate::yaml::from_json(v))).collect();
                crate::write::write(vault, path, &changes, ctx, journal, &Default::default()).map_err(|e| format!("{path}: re-apply: {e}"))?;
            }
            for mut rec in res.supersede { journal.append(&mut rec).map_err(|e| format!("{path}: supersede: {e:?}"))?; }
            git(vault, &["add", "--", path])?;
            for p in STAGED_PATHS { if p == "state" { git(vault, &["add", "--", "state"])?; } }
            settled.push(path.clone());
        }
        match git(vault, &["-c", "core.editor=true", "rebase", "--continue"]) {
            Ok(_) => {}
            Err(e) if e.contains("CONFLICT") || status(vault).conflicted.len() > 0 => continue,
            Err(e) => return Err(format!("rebase --continue: {e}")),
        }
    }
    Ok(settled)
}
```
(`crate::yaml::from_json(&serde_json::Value) -> serde_yaml_ng::Value` exists at `src/yaml.rs:118`; `yaml::to_json` is its twin. Do not add a second converter.) Update the `sync` call site to use the real function and drop the `super::history::` prefix.

- [ ] **Step 4: Run** — `cargo test history 2>&1 | tail -3` → 7 passed; oracle green; both dual-run scripts clean (no engine output path changed, but run them: this task touched `write`'s callers).
- [ ] **Step 5: Commit** — `git add src/history.rs` — "engine: history resolves rebase conflicts through reconcile — upstream wins the file, this device's records re-applied through write, supersede records appended; first caller of reconcile since wave 3 (Knowlu plan 1, Task 6)".

---

### Task 7: The app becomes Knowlu — rename stage 1, tray, autostart, hide-to-tray, MessageBoxW, settings, scratch vault

**Files:**
- Modify: `app/Cargo.toml`, `app/tauri.conf.json`, `app/capabilities/default.json`, `app/src/main.rs`, `app/src/state.rs`, `app/src/lib.rs`, `app/README.md`, `app/static/index.html` (the `<b>quinn-ops</b>` brand → `Knowlu`; `<title>`)
- Create: `app/src/tray.rs`, `scripts/scratch-vault.ps1`
- Test: `app/tests/commands.rs` (settings), `app/tests/static_assets.rs` (brand)

**Interfaces:**
- Produces:
  - `app/Cargo.toml`: `[[bin]] name = "knowlu" path = "src/main.rs"`; `tauri = { version = "2", features = ["tray-icon"] }`; `tauri-plugin-autostart = "2"`; `tauri-plugin-clipboard-manager = "2"`; `[target.'cfg(windows)'.dependencies] windows = { version = "0.62", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }`.
  - `tauri.conf.json`: `productName: "Knowlu"`, `identifier: "app.knowlu.desktop"` (placeholder until Quinn settles the domain — say so in a JSON-adjacent comment in README, JSON has none), window `title: "Knowlu"`.
  - `state.rs`: `pub struct Settings { pub profile_id: String, pub backup_dir: Option<PathBuf>, pub autostart: bool, pub quit_at: Option<String> }` with `load(path) -> Settings` / `save(&self, path) -> Result<(), String>`; `ConsoleState` gains `settings_path: PathBuf`, `settings: Mutex<Settings>`, `session: String`, `head_sha: Mutex<Option<String>>`, `history: Mutex<HistoryStatus>`, `backup: Mutex<BackupStatus>`, `last_write: Mutex<Option<std::time::Instant>>`, `pending_edits: AtomicUsize`, `auto_sync: AtomicBool`, `startup_missed: AtomicUsize`. `profile_id` defaults to `quinn_ops::ids::derived_id("profile", &vault.to_string_lossy())`.
  - `tray.rs`: `pub enum TrayState { Ok, Warn }`; `pub fn build(app: &tauri::AppHandle) -> tauri::Result<()>` (menu ids `open`, `run-now`, `pause`, `diag`, `quit`; `run-now`/`pause` emit `knowlu://scheduler` events that Task 12 consumes; until then they are no-ops); `pub fn set_state(app: &tauri::AppHandle, s: TrayState)`; `pub fn diagnostics_text(cs: &ConsoleState) -> String` (version, build, last three run lines from `state/runs/` via `surface::runs_panel`, last error, slot config — **no note content**).
  - `main.rs`: `fn fatal(msg: &str) -> !` — on Windows a `MessageBoxW` titled `Knowlu` then `exit(2)`; hide-to-tray on `CloseRequested`; autostart enabled per settings with `--vault <path>`.

- [ ] **Step 1: Write the failing tests**

```rust
// app/tests/commands.rs (append)
#[test]
fn settings_round_trip_and_the_profile_id_is_derived_not_named() {
    use quinn_ops_console::state::Settings;
    let d = std::env::temp_dir().join(format!("qo-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d); std::fs::create_dir_all(&d).unwrap();
    let p = d.join("settings.json");
    let s = Settings::load(&p, std::path::Path::new(r"C:\anywhere\vault"));
    assert!(s.profile_id.starts_with("profile_") && s.profile_id.len() == "profile_".len() + 10, "{}", s.profile_id);
    assert!(s.backup_dir.is_none() && s.autostart && s.quit_at.is_none());
    let mut s2 = s.clone(); s2.backup_dir = Some(d.join("bk")); s2.save(&p).unwrap();
    assert_eq!(Settings::load(&p, std::path::Path::new(r"C:\anywhere\vault")), s2);
}

#[test]
fn diagnostics_text_carries_no_note_title() {
    use quinn_ops_console::{state::ConsoleState, tray::diagnostics_text};
    let v = scratch("diag");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-diag-data-{}", std::process::id())));
    let t = diagnostics_text(&cs);
    assert!(t.contains("Knowlu") && t.contains("build "), "{t}");
    assert!(!t.contains("PH 106"), "a task title leaked into diagnostics: {t}");
}
```
```rust
// app/tests/static_assets.rs (append)
#[test]
fn the_brand_is_knowlu_everywhere_a_user_reads_it() {
    let html = read("index.html");
    assert!(html.contains("<title>Knowlu</title>") && html.contains("<b>Knowlu</b>"), "index.html");
    assert!(!html.contains("quinn-ops"), "the old name must not be visible in the page");
    let conf = fs::read_to_string("tauri.conf.json").unwrap();
    assert!(conf.contains("\"productName\": \"Knowlu\"") && conf.contains("\"title\": \"Knowlu\""));
}
```

- [ ] **Step 2: Run to verify failure** — `cd app; cargo test 2>&1 | tail -5` → `Settings` not found; brand test fails.

- [ ] **Step 3: Implement**

`app/src/state.rs` additions:

```rust
use std::sync::atomic::{AtomicBool, AtomicUsize};
use quinn_ops::{backup::BackupStatus, history::HistoryStatus};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Settings { pub profile_id: String, pub backup_dir: Option<PathBuf>, pub autostart: bool, pub quit_at: Option<String> }

impl Settings {
    pub fn load(path: &Path, vault: &Path) -> Settings {
        let default = Settings { profile_id: quinn_ops::ids::derived_id("profile", &vault.to_string_lossy()), backup_dir: None, autostart: true, quit_at: None };
        std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<Settings>(&t).ok()).unwrap_or(default)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        let v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        std::fs::write(path, quinn_ops::ledger::dumps_value(&v)).map_err(|e| e.to_string())
    }
}

pub struct ConsoleState {
    pub vault: PathBuf, pub seen_path: PathBuf, pub settings_path: PathBuf, pub lock: Mutex<()>,
    pub settings: Mutex<Settings>, pub session: String,
    pub head_sha: Mutex<Option<String>>, pub history: Mutex<HistoryStatus>, pub backup: Mutex<BackupStatus>,
    pub last_write: Mutex<Option<std::time::Instant>>, pub pending_edits: AtomicUsize,
    pub auto_sync: AtomicBool, pub startup_missed: AtomicUsize,
}

impl ConsoleState {
    pub fn open(vault: PathBuf, app_data_dir: PathBuf) -> ConsoleState {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let settings_path = app_data_dir.join("settings.json");
        let settings = Settings::load(&settings_path, &vault);
        ConsoleState {
            seen_path: app_data_dir.join("seen.txt"), settings_path, lock: Mutex::new(()),
            settings: Mutex::new(settings), session: quinn_ops::ids::new_id("sess"),
            head_sha: Mutex::new(None), history: Mutex::new(HistoryStatus::default()),
            backup: Mutex::new(BackupStatus { last_ok: None, behind_days: None, last_error: None, target_reachable: false }),
            last_write: Mutex::new(None), pending_edits: AtomicUsize::new(0),
            auto_sync: AtomicBool::new(true), startup_missed: AtomicUsize::new(0), vault,
        }
    }
    pub fn seen_at(&self) -> Option<String> { /* unchanged */ }
    pub fn note_write(&self) { *self.last_write.lock().unwrap() = Some(std::time::Instant::now()); self.pending_edits.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
}
```

`app/src/tray.rs`:

```rust
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, AppHandle, Emitter, Manager};
use crate::state::ConsoleState;

pub enum TrayState { Ok, Warn }

fn square(rgb: [u8; 3]) -> tauri::image::Image<'static> {
    let mut px = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 { for x in 0..32 { let edge = x < 2 || y < 2 || x > 29 || y > 29; if edge { px.extend_from_slice(&[0, 0, 0, 0]) } else { px.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]) } } }
    tauri::image::Image::new_owned(px, 32, 32)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let run = MenuItem::with_id(app, "run-now", "Run now", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause scheduling", true, None::<&str>)?;
    let diag = MenuItem::with_id(app, "diag", "Copy diagnostics", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &run, &pause, &diag, &quit])?;
    TrayIconBuilder::with_id("main").icon(square([0x3F, 0xB6, 0x8B])).tooltip("Knowlu").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
            "run-now" | "pause" => { let _ = app.emit("knowlu://scheduler", ev.id().as_ref().to_string()); }
            "diag" => { let cs = app.state::<ConsoleState>(); let text = diagnostics_text(&cs); use tauri_plugin_clipboard_manager::ClipboardExt; let _ = app.clipboard().write_text(text); }
            "quit" => { let cs = app.state::<ConsoleState>(); let mut s = cs.settings.lock().unwrap(); s.quit_at = Some(quinn_ops::journal::now_ts(None)); let _ = s.save(&cs.settings_path); app.exit(0); }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| { if let tauri::tray::TrayIconEvent::DoubleClick { .. } = ev { if let Some(w) = tray.app_handle().get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } } })
        .build(app)?;
    Ok(())
}

pub fn set_state(app: &AppHandle, s: TrayState) {
    if let Some(t) = app.tray_by_id("main") {
        let _ = t.set_icon(Some(match s { TrayState::Ok => square([0x3F, 0xB6, 0x8B]), TrayState::Warn => square([0xD9, 0xA4, 0x41]) }));
    }
}

/// Version, build, the last three run lines, the last sync/backup error, the slot config — and
/// **no note content**: a friend pastes this into a message.
pub fn diagnostics_text(cs: &ConsoleState) -> String {
    let now = jiff::Timestamp::now();
    let runs = quinn_ops::surface::runs_panel(&cs.vault, now);
    let mut out = format!("Knowlu {} build {}\n", env!("CARGO_PKG_VERSION"), crate::commands::CONSOLE_BUILD.unwrap_or("unknown"));
    for r in runs.recent.iter().take(3) { out.push_str(&format!("run {} {} {} {}\n", r.runner, r.started.clone().unwrap_or_default(), r.result, r.summary)); }
    for e in &runs.expected { out.push_str(&format!("expected {} {} {}\n", e.runner, e.due, e.status)); }
    if let Some(e) = &cs.history.lock().unwrap().last_error { out.push_str(&format!("sync error: {e}\n")); }
    if let Some(e) = &cs.backup.lock().unwrap().last_error { out.push_str(&format!("backup error: {e}\n")); }
    out
}
```
(`RunRow.summary` is the engine's run summary — counts and warning text, never a title; `diagnostics_text_carries_no_note_title` guards it.)

`app/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Manager;
use quinn_ops_console::{commands, state::{resolve_vault, ConsoleState}, tray};

fn fatal(msg: &str) -> ! {
    eprintln!("Knowlu: {msg}");
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        use windows::core::PCWSTR;
        let text: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "Knowlu".encode_utf16().chain(std::iter::once(0)).collect();
        MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
    }
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let vault_arg = args.iter().position(|a| a == "--vault").and_then(|i| args.get(i + 1)).cloned();
    let cwd = std::env::current_dir().unwrap_or_default();
    let vault = match resolve_vault(vault_arg.as_deref(), &cwd) { Ok(v) => v, Err(msg) => fatal(&msg) };
    let vault_str: &'static str = Box::leak(vault.to_string_lossy().into_owned().into_boxed_str());
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--vault", vault_str])))
        .setup(move |app| {
            let data = app.path().app_data_dir()?;
            let cs = ConsoleState::open(vault.clone(), data);
            let autostart = cs.settings.lock().unwrap().autostart;
            app.manage(cs);
            tray::build(app.handle())?;
            use tauri_plugin_autostart::ManagerExt;
            let al = app.autolaunch();
            if autostart { let _ = al.enable(); } else { let _ = al.disable(); }
            Ok(())
        })
        .on_window_event(|w, e| { if let tauri::WindowEvent::CloseRequested { api, .. } = e { api.prevent_close(); let _ = w.hide(); } })
        .invoke_handler(tauri::generate_handler![commands::state, commands::note, commands::mark_seen])
        .run(tauri::generate_context!())
        .expect("Knowlu: failed to start the Tauri runtime");
}
```
`app/src/lib.rs`: `pub mod tray;`. `capabilities/default.json` `permissions`: add `"autostart:default"`, `"clipboard-manager:allow-write-text"` (harmless for Rust-side calls, required if the page ever calls them). `index.html`: `<title>Knowlu</title>`, `<b>Knowlu</b>`.

`scripts/scratch-vault.ps1` (CRLF):

```powershell
# Copies the vault's data folders from the main checkout into a fresh scratch folder under
# %LOCALAPPDATA%\quinn-ops\scratch\<stamp>, makes it a local-only git repository (no remote), and
# prints the launch line. The console is developed and looked at against THIS, never the live
# vault, until G2 (Knowlu spec §2). Read-only against the source.
param([string]$Source = (Split-Path $PSScriptRoot -Parent))
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dest = Join-Path $env:LOCALAPPDATA "quinn-ops\scratch\$stamp"
New-Item -ItemType Directory -Force $dest | Out-Null
foreach ($f in @('tasks','approvals','archive','courses','issues','info','state','config','profile')) {
    $src = Join-Path $Source $f
    if (Test-Path $src) { Copy-Item -Recurse -Force $src (Join-Path $dest $f) }
}
Push-Location $dest
git init -q --initial-branch=main; git add .; git -c user.name=scratch -c user.email=scratch@localhost commit -q -m "scratch copy $stamp"
Pop-Location
Write-Output "scratch vault: $dest"
Write-Output "launch: app\target\release\knowlu.exe --vault `"$dest`""
```

`app/README.md`: rewrite the header — Knowlu, what the crate is now (engine linked, commands, tray, scheduler behind a key), build and run, the scratch-vault rule, the identifier placeholder note, the known `.rsrc` warning.

- [ ] **Step 4: Commit scaffolding BEFORE the long build**, then build

```bash
git add app/Cargo.toml app/Cargo.lock app/tauri.conf.json app/capabilities/default.json app/src app/static/index.html app/README.md scripts/scratch-vault.ps1
git commit -F <msgfile>   # "app: Knowlu — rename stage 1, tray, autostart, hide-to-tray, MessageBoxW on a missing vault, settings file, scratch-vault script (Knowlu plan 1, Task 7)"
cd app && cargo test 2>&1 | tail -5     # expect all green, no NEW warnings
cargo build --release 2>&1 | tail -3
```
Then `scripts/scratch-vault.ps1` and launch `app\target\release\knowlu.exe --vault <scratch>`: the window title reads Knowlu; closing hides to the tray; the tray menu shows five items; Quit exits. **Look at it yourself; do not send input to the desktop from a script.** Screenshot by window handle only.

- [ ] **Step 5: Amend nothing — a second commit if the build forced a fix**, message "app: Task 7 build fix — <what>".

---

### Task 8: `set_fields`, `create_task`, `delete_note`

**Files:**
- Modify: `app/src/commands.rs`, `app/src/main.rs` (handler list)
- Test: `app/tests/commands.rs`

**Interfaces:**
- Consumes: `write::write_literals(vault, target, &[(String,String)], ctx, journal, &WriteOpts)`, `write::create`, `write::delete`, `write::to_literal`, `yamlemit::{Node, safe_dump_block}`, `journal::VIAS`.
- Produces (all `pub fn *_inner(cs, …) -> Result<Value, String>` plus the `#[tauri::command]` twins):
  - `pub const EDITABLE: [&str; 12] = ["title","course","due","effort_hours","importance","importance_reason","status","progress","slice_hours","domain","rank_override","effort_confidence"];`
  - `pub const QUOTED: [&str; 5] = ["title","course","importance_reason","domain","effort_confidence"];` — emitted through `write::to_literal(&Value::String)` so a colon in a title survives; every other field is passed as the literal the page sent.
  - `pub fn console_ctx() -> WriteContext` — `WriteContext::new("quinn", "dashboard")`, asserted against `journal::VIAS` in a test.
  - `set_fields_inner(cs, view, id, fields: Map<String, Value>)`, `create_task_inner(cs, view, fields)`, `delete_note_inner(cs, view, id)`.
  - Envelope: `{ok, error, state}`; helper `fn mutate(cs, view, f: impl FnOnce(&mut Journal) -> Result<(), String>) -> Value` — takes the lock, runs `f`, calls `cs.note_write()` on success, rebuilds state for `view`, returns the envelope with `ok` from `f`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn set_fields_journals_a_quinn_dashboard_record_and_returns_fresh_state() {
    let v = scratch("set");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-set-data-{}", std::process::id())));
    let id = first_id(&cs);
    let mut f = serde_json::Map::new(); f.insert("importance".into(), json!(1)); f.insert("title".into(), json!("Exam 1: prep, part 2"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    let text = std::fs::read_to_string(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    assert!(text.contains("importance: 1") && text.contains("title: 'Exam 1: prep, part 2'") || text.contains("title: \"Exam 1: prep, part 2\""), "{text}");
    let journal = std::fs::read_dir(v.join("state/journal")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect::<String>();
    assert!(journal.contains("\"actor\": \"quinn\"") && journal.contains("\"via\": \"dashboard\""));
    assert_eq!(env["state"]["schema"], 1);
    assert!(quinn_ops::journal::VIAS.contains(&"dashboard"));
}

#[test]
fn a_non_editable_field_is_refused_with_the_current_state_and_nothing_written() {
    let v = scratch("refuse");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-refuse-data-{}", std::process::id())));
    let id = first_id(&cs);
    let before = std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap();
    let mut f = serde_json::Map::new(); f.insert("source_uid".into(), json!("x"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], false); assert!(env["error"].as_str().unwrap().contains("source_uid"));
    assert_eq!(env["state"]["schema"], 1, "a refusal still carries the current state");
    assert_eq!(before, std::fs::read(v.join("tasks/ph-106-exam-1-prep.md")).unwrap());
}

#[test]
fn progress_100_and_status_done_land_in_one_write() {
    let v = scratch("done");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-done-data-{}", std::process::id())));
    let id = first_id(&cs);
    let mut f = serde_json::Map::new(); f.insert("progress".into(), json!(100)); f.insert("status".into(), json!("done"));
    let env = set_fields_inner(&cs, "today", &id, f).unwrap();
    assert_eq!(env["ok"], true);
    assert!(env["state"]["closed_this_week"].as_array().unwrap().iter().any(|c| c["id"] == json!(id)), "one line in CLOSED THIS WEEK");
}

#[test]
fn create_task_defaults_and_slug_and_delete_archives() {
    let v = scratch("create");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-create-data-{}", std::process::id())));
    let mut f = serde_json::Map::new(); f.insert("title".into(), json!("Read chapter 3: waves")); f.insert("course".into(), json!("ph-106")); f.insert("due".into(), json!("2026-09-10"));
    let env = create_task_inner(&cs, "today", f).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    let path = v.join("tasks/read-chapter-3-waves.md");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("due: 2026-09-10T23:59") && text.contains("effort_hours: 1.0") && text.contains("importance: 3") && text.contains("status: active") && text.contains("created_by: quinn") && text.contains("id: task_"), "{text}");
    let id = text.lines().find_map(|l| l.strip_prefix("id: ")).unwrap().trim().to_string();
    let env = delete_note_inner(&cs, "today", &id).unwrap();
    assert_eq!(env["ok"], true);
    assert!(!path.exists() && v.join("archive/read-chapter-3-waves.md").exists(), "nothing is unlinked");
    let mut f = serde_json::Map::new(); f.insert("title".into(), json!("   "));
    assert_eq!(create_task_inner(&cs, "today", f).unwrap()["ok"], false, "title is required");
}

fn first_id(cs: &ConsoleState) -> String { state_inner(cs, "today").unwrap()["state"]["must_do"]["groups"][0]["rows"][0]["id"].as_str().unwrap().to_string() }
```

- [ ] **Step 2: Run to verify failure** — `cd app; cargo test 2>&1 | tail -5` → functions not found.

- [ ] **Step 3: Implement** (`app/src/commands.rs`)

```rust
use quinn_ops::{journal::Journal, write::{self, WriteContext, WriteOpts}};
use quinn_ops::yamlemit::{safe_dump_block, Node};

pub const EDITABLE: [&str; 12] = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence"];
pub const QUOTED: [&str; 5] = ["title", "course", "importance_reason", "domain", "effort_confidence"];

pub fn console_ctx() -> WriteContext {
    debug_assert!(quinn_ops::journal::VIAS.contains(&"dashboard"));
    WriteContext::new("quinn", "dashboard")
}

fn mutate(cs: &ConsoleState, view: &str, f: impl FnOnce(&mut Journal) -> Result<(), String>) -> Result<Value, String> {
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let mut journal = Journal::new(&cs.vault);
    let result = f(&mut journal);
    if result.is_ok() { cs.note_write(); }
    let state = build_state_value(cs, view)?;
    Ok(match result { Ok(()) => json!({ "ok": true, "error": Value::Null, "state": state }), Err(e) => json!({ "ok": false, "error": e, "state": state }) })
}

fn literal_for(field: &str, v: &Value) -> Result<String, String> {
    if QUOTED.contains(&field) {
        let s = v.as_str().ok_or_else(|| format!("{field} must be a string"))?;
        return Ok(write::to_literal(&serde_yaml_ng::Value::String(s.to_string())));
    }
    Ok(match v { Value::String(s) => s.clone(), Value::Null => "null".into(), other => other.to_string() })
}

pub fn set_fields_inner(cs: &ConsoleState, view: &str, id: &str, fields: serde_json::Map<String, Value>) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        let mut literals = Vec::new();
        for (k, v) in &fields {
            if !EDITABLE.contains(&k.as_str()) { return Err(format!("{k} is not editable")); }
            literals.push((k.clone(), literal_for(k, v)?));
        }
        let res = write::write_literals(&cs.vault, id, &literals, &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
        if let Some((name, why)) = res.skipped.iter().next() { return Err(format!("{name}: {why}")); }
        Ok(())
    })
}

fn slugify(title: &str) -> String {
    let mut s: String = title.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    while s.contains("--") { s = s.replace("--", "-"); }
    s.trim_matches('-').chars().take(60).collect::<String>().trim_matches('-').to_string()
}

pub fn create_task_inner(cs: &ConsoleState, view: &str, fields: serde_json::Map<String, Value>) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        let title = fields.get("title").and_then(|v| v.as_str()).map(str::trim).filter(|t| !t.is_empty()).ok_or("title is required")?;
        let course = fields.get("course").and_then(|v| v.as_str()).map(str::trim).filter(|c| !c.is_empty());
        let due = fields.get("due").and_then(|v| v.as_str()).map(|d| if d.len() == 10 { format!("{d}T23:59") } else { d.to_string() });
        let effort = fields.get("effort_hours").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let front = Node::map(vec![
            ("title", Node::text(title)),
            ("course", Node::opt_text(course)),
            ("domain", Node::text("school")),
            ("due", due.as_deref().map(Node::text).unwrap_or(Node::Null)),
            ("effort_hours", Node::Float(effort)),
            ("effort_source", Node::text("quinn")),
            ("importance", Node::Int(3)),
            ("status", Node::text("active")),
            ("progress", Node::Int(0)),
            ("created_by", Node::text("quinn")),
        ]);
        let text = format!("---\n{}---\n\n", safe_dump_block(&front));
        let mut slug = slugify(title); if slug.is_empty() { slug = "task".into(); }
        let mut rel = format!("tasks/{slug}.md"); let mut n = 2;
        while cs.vault.join(&rel).exists() { rel = format!("tasks/{slug}-{n}.md"); n += 1; }
        write::create(&cs.vault, &rel, &text, &console_ctx(), journal, None).map(|_| ()).map_err(|e| e.to_string())
    })
}

pub fn delete_note_inner(cs: &ConsoleState, view: &str, id: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| write::delete(&cs.vault, id, &console_ctx(), journal).map(|_| ()).map_err(|e| e.to_string()))
}

#[tauri::command] pub fn set_fields(cs: State<'_, ConsoleState>, view: String, id: String, fields: serde_json::Map<String, Value>) -> Value { set_fields_inner(&cs, &view, &id, fields).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
#[tauri::command] pub fn create_task(cs: State<'_, ConsoleState>, view: String, fields: serde_json::Map<String, Value>) -> Value { create_task_inner(&cs, &view, fields).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
#[tauri::command] pub fn delete_note(cs: State<'_, ConsoleState>, view: String, id: String) -> Value { delete_note_inner(&cs, &view, &id).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
```
Add the three to `generate_handler!`. (`yamlemit` is already `pub mod` in the engine's `src/lib.rs:35`; `Node::{text, opt_text, map, Int, Float, Null}` and `safe_dump_block` are the API `info::open_info` uses for the same job.)

- [ ] **Step 4: Run** — `cd app; cargo test 2>&1 | tail -5` → green, no new warnings. Root `cargo test` unchanged.
- [ ] **Step 5: Commit** — `git add app/src/commands.rs app/src/main.rs app/tests/commands.rs` (+ `src/lib.rs` if visibility changed) — "app: set_fields, create_task, delete_note — quinn/dashboard writes, the editable twelve, refusals carry the current state (Knowlu plan 1, Task 8)".

---

### Task 9: `decide` with in-process execution; `close_info`, `open_issue`, `resolve_issue`

**Files:**
- Modify: `app/src/commands.rs`, `app/src/main.rs`
- Test: `app/tests/commands.rs`

**Interfaces:**
- Consumes: `approvals::process_approvals(vault, today, now: civil::DateTime, ctx, journal) -> ApprovalsResult`; `info::close_info(vault, key, info_id, closed_by, ctx, Option<&mut Journal>, now)`; `issues::open_issue(vault, target, categories: &[String], text: Option<&str>, ctx, journal, now, target_record)`; `issues::address_issue(vault, issue_id, resolution, commit, ctx, journal, now)`; `issues::CATEGORIES`.
- Produces: `decide_inner(cs, view, id, verdict, note, snooze_until: Option<String>)` — verdict ∈ `approved|rejected|snoozed`; writes `status`, `decision_note` (quoted), and `snooze_until` (`null` unless snoozed) in one `write_literals`, then `process_approvals` with the console context; the envelope adds `"decision": {executed, expired, woken, rejected, warnings}` marshalled from `ApprovalsResult` (strings only). `close_info_inner(cs, view, id)`, `open_issue_inner(cs, view, target, categories, text)`, `resolve_issue_inner(cs, view, id, resolution)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn approving_a_task_proposal_puts_the_task_in_the_returned_state() {
    let v = scratch("decide");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-decide-data-{}", std::process::id())));
    let s = state_inner(&cs, "decisions").unwrap();
    let card = s["state"]["decisions"]["cards"].as_array().unwrap().iter().find(|c| c["kind"] == json!("task")).expect("vault-full holds a pending task proposal");
    let before = s["state"]["verdict"]["active"].as_i64().unwrap();
    let env = decide_inner(&cs, "today", card["id"].as_str().unwrap(), "approved", "looks right", None).unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["decision"]["executed"].as_array().unwrap().len(), 1);
    assert_eq!(env["state"]["verdict"]["active"].as_i64().unwrap(), before + 1, "the approved task is in the ranked list NOW, not six hours later");
    assert!(!env["state"]["decisions"]["cards"].as_array().unwrap().iter().any(|c| c["id"] == card["id"]));
}

#[test]
fn rejecting_and_snoozing_write_the_decision_fields() {
    let v = scratch("snooze");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-snooze-data-{}", std::process::id())));
    let s = state_inner(&cs, "decisions").unwrap();
    let cards = s["state"]["decisions"]["cards"].as_array().unwrap().clone();
    let a = cards[0]["id"].as_str().unwrap(); let b = cards[1]["id"].as_str().unwrap();
    assert_eq!(decide_inner(&cs, "today", a, "rejected", "no", None).unwrap()["ok"], true);
    let env = decide_inner(&cs, "today", b, "snoozed", "later", Some("2099-01-01".into())).unwrap();
    assert_eq!(env["ok"], true);
    let text = std::fs::read_to_string(v.join("approvals").join(format!("{}.md", cards[1]["slug"].as_str().unwrap()))).unwrap();
    assert!(text.contains("status: snoozed") && text.contains("snooze_until: 2099-01-01") && text.contains("decision_note:"), "{text}");
    assert_eq!(decide_inner(&cs, "today", a, "maybe", "", None).unwrap()["ok"], false, "verdict is one of three words");
}

#[test]
fn issue_flag_needs_a_category_and_snapshots_the_object_and_info_closes() {
    let v = scratch("issue");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-issue-data-{}", std::process::id())));
    let id = first_id(&cs);
    assert_eq!(open_issue_inner(&cs, "today", &id, vec![], "x").unwrap()["ok"], false);
    let env = open_issue_inner(&cs, "today", &id, vec!["wrong-effort".into()], "12 h is too much").unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["state"]["issues_panel"]["open_count"], 1);
    let issue_id = env["state"]["issues_panel"]["rows"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(resolve_issue_inner(&cs, "today", &issue_id, "effort set to 6").unwrap()["state"]["issues_panel"]["open_count"], 0);
    let s = state_inner(&cs, "today").unwrap();
    if let Some(info) = s["state"]["good_to_know"].as_array().unwrap().first() {
        let env = close_info_inner(&cs, "today", info["id"].as_str().unwrap()).unwrap();
        assert_eq!(env["ok"], true);
        assert!(env["state"]["good_to_know"].as_array().unwrap().iter().all(|i| i["id"] != info["id"]));
    }
}
```

- [ ] **Step 2: Run to verify failure** — functions not found.

- [ ] **Step 3: Implement**

```rust
pub fn decide_inner(cs: &ConsoleState, view: &str, id: &str, verdict: &str, note: &str, snooze_until: Option<String>) -> Result<Value, String> {
    let mut decision = Value::Null;
    let env = mutate(cs, view, |journal| {
        if !["approved", "rejected", "snoozed"].contains(&verdict) { return Err(format!("verdict must be approved, rejected or snoozed, not {verdict:?}")); }
        let snooze = match (verdict, snooze_until.as_deref()) { ("snoozed", Some(d)) if d.len() == 10 => d.to_string(), ("snoozed", _) => return Err("snoozed needs snooze_until as YYYY-MM-DD".into()), _ => "null".into() };
        let literals = vec![
            ("status".to_string(), verdict.to_string()),
            ("decision_note".to_string(), write::to_literal(&serde_yaml_ng::Value::String(note.to_string()))),
            ("snooze_until".to_string(), snooze),
        ];
        write::write_literals(&cs.vault, id, &literals, &console_ctx(), journal, &WriteOpts::default()).map_err(|e| e.to_string())?;
        let now = now_in(&cs.vault);
        let r = quinn_ops::approvals::process_approvals(&cs.vault, now.date(), now.datetime(), &console_ctx(), journal);
        decision = json!({ "executed": r.executed, "expired": r.expired, "woken": r.woken, "rejected": r.rejected, "warnings": r.warnings });
        Ok(())
    })?;
    let mut env = env; env["decision"] = decision; Ok(env)
}

pub fn close_info_inner(cs: &ConsoleState, view: &str, id: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| quinn_ops::info::close_info(&cs.vault, None, Some(id), "quinn", &console_ctx(), Some(journal), None).map(|_| ()).map_err(|e| e.to_string()))
}

pub fn open_issue_inner(cs: &ConsoleState, view: &str, target: &str, categories: Vec<String>, text: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| {
        if categories.is_empty() { return Err("pick at least one category".into()); }
        let text = text.trim();
        quinn_ops::issues::open_issue(&cs.vault, target, &categories, (!text.is_empty()).then_some(text), &console_ctx(), Some(journal), None, None).map(|_| ()).map_err(|e| e.to_string())
    })
}

pub fn resolve_issue_inner(cs: &ConsoleState, view: &str, id: &str, resolution: &str) -> Result<Value, String> {
    mutate(cs, view, |journal| quinn_ops::issues::address_issue(&cs.vault, id, resolution, None, &console_ctx(), Some(journal), None).map(|_| ()).map_err(|e| e.to_string()))
}

#[tauri::command] pub fn decide(cs: State<'_, ConsoleState>, view: String, id: String, verdict: String, note: String, snooze_until: Option<String>) -> Value { decide_inner(&cs, &view, &id, &verdict, &note, snooze_until).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
#[tauri::command] pub fn close_info(cs: State<'_, ConsoleState>, view: String, id: String) -> Value { close_info_inner(&cs, &view, &id).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
#[tauri::command] pub fn open_issue(cs: State<'_, ConsoleState>, view: String, target: String, categories: Vec<String>, text: String) -> Value { open_issue_inner(&cs, &view, &target, categories, &text).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
#[tauri::command] pub fn resolve_issue(cs: State<'_, ConsoleState>, view: String, id: String, resolution: String) -> Value { resolve_issue_inner(&cs, &view, &id, &resolution).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })) }
```
(`IssueError`/`InfoError` need `Display`; if they lack it, `format!("{e:?}")` — do not add `Display` impls to the engine in this task.) Add the four to `generate_handler!`.

- [ ] **Step 4: Run** — `cd app; cargo test 2>&1 | tail -5` → green. Note in the report which approval the first test executed (kind `task`) and that the calendar-event kind stays pending — assert it if vault-full has one.
- [ ] **Step 5: Commit** — "app: decide executes the decision in-process; close_info, open_issue, resolve_issue (Knowlu plan 1, Task 9)".

---

### Task 10: `sync`, `backup_now`, settings commands, and status in the envelope

**Files:**
- Modify: `app/src/commands.rs`, `app/src/state.rs`, `app/src/main.rs`
- Test: `app/tests/commands.rs`

**Interfaces:**
- Consumes: `history::{status, sync, HistoryStatus}`, `backup::{tick, status, BackupStatus}`.
- Produces:
  - `state.rs`: `pub fn refresh_history(cs)` (→ `cs.history`), `pub fn run_backup(cs, now) -> BackupStatus` (no-op with `last_error: "no backup folder set"` when `settings.backup_dir` is `None`), `pub fn run_sync(cs) -> SyncOutcome` (uses `pending_edits`, resets it, sets `auto_sync = false` when `conflicted` is non-empty).
  - `commands.rs`: `sync_inner(cs, view)`, `backup_now_inner(cs, view)`, `get_settings_inner(cs)`, `set_settings_inner(cs, patch: Map)` (`backup_dir`, `autostart` only), and **`build_state_value` adds `topline.sync`, `topline.backup`, `topline.startup_missed`, `topline.auto_sync`** copied from the caches — no computation.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn sync_on_a_non_repo_vault_is_calm_and_backup_needs_a_folder() {
    let v = scratch("sync");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-sync-data-{}", std::process::id())));
    let env = sync_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true);
    assert_eq!(env["state"]["topline"]["sync"]["is_repo"], false);
    let env = backup_now_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], false); assert!(env["error"].as_str().unwrap().contains("backup folder"));
    let bk = std::env::temp_dir().join(format!("qo-sync-bk-{}", std::process::id()));
    let mut patch = serde_json::Map::new(); patch.insert("backup_dir".into(), json!(bk.to_string_lossy()));
    assert_eq!(set_settings_inner(&cs, patch).unwrap()["ok"], true);
    let env = backup_now_inner(&cs, "today").unwrap();
    assert_eq!(env["ok"], true, "{env}");
    assert_eq!(env["state"]["topline"]["backup"]["behind_days"], 0);
    assert!(bk.join(&cs.settings.lock().unwrap().profile_id).join("vault/tasks").is_dir());
    let mut bad = serde_json::Map::new(); bad.insert("profile_id".into(), json!("me"));
    assert_eq!(set_settings_inner(&cs, bad).unwrap()["ok"], false, "only backup_dir and autostart are settable");
}
```

- [ ] **Step 2: Run to verify failure.**

- [ ] **Step 3: Implement**

```rust
// state.rs
pub fn refresh_history(cs: &ConsoleState) { let s = quinn_ops::history::status(&cs.vault); *cs.history.lock().unwrap() = s; }

pub fn run_sync(cs: &ConsoleState) -> quinn_ops::history::SyncOutcome {
    let edits = cs.pending_edits.swap(0, std::sync::atomic::Ordering::SeqCst);
    let mut journal = quinn_ops::journal::Journal::new(&cs.vault);
    let out = quinn_ops::history::sync(&cs.vault, &crate::commands::console_ctx(), &mut journal, edits);
    if !out.status.conflicted.is_empty() { cs.auto_sync.store(false, std::sync::atomic::Ordering::SeqCst); }
    *cs.history.lock().unwrap() = out.status.clone();
    out
}

pub fn run_backup(cs: &ConsoleState, now: jiff::Timestamp) -> Result<quinn_ops::backup::BackupStatus, String> {
    let (dir, profile) = { let s = cs.settings.lock().unwrap(); (s.backup_dir.clone(), s.profile_id.clone()) };
    let dir = dir.ok_or("no backup folder set — pick one in settings")?;
    let today = now.to_zoned(quinn_ops::cli::vault_zone(&cs.vault)).date();
    let st = quinn_ops::backup::tick(&cs.vault, &dir, &profile, now, today);
    *cs.backup.lock().unwrap() = st.clone();
    Ok(st)
}
```
```rust
// commands.rs — in build_state_value, after engine_newer:
v["topline"]["sync"] = serde_json::to_value(&*cs.history.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
v["topline"]["backup"] = serde_json::to_value(&*cs.backup.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())?;
v["topline"]["auto_sync"] = json!(cs.auto_sync.load(std::sync::atomic::Ordering::SeqCst));
v["topline"]["startup_missed"] = json!(cs.startup_missed.load(std::sync::atomic::Ordering::SeqCst));

pub fn sync_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let out = crate::state::run_sync(cs);
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let state = build_state_value(cs, view)?;
    Ok(json!({ "ok": out.status.last_error.is_none(), "error": out.status.last_error, "state": state }))
}
pub fn backup_now_inner(cs: &ConsoleState, view: &str) -> Result<Value, String> {
    let r = crate::state::run_backup(cs, jiff::Timestamp::now());
    let _g = cs.lock.lock().map_err(|_| "console lock poisoned".to_string())?;
    let state = build_state_value(cs, view)?;
    Ok(match r { Ok(st) if st.last_error.is_none() => json!({ "ok": true, "error": Value::Null, "state": state }), Ok(st) => json!({ "ok": false, "error": st.last_error, "state": state }), Err(e) => json!({ "ok": false, "error": e, "state": state }) })
}
pub fn get_settings_inner(cs: &ConsoleState) -> Result<Value, String> { Ok(json!({ "ok": true, "error": Value::Null, "settings": serde_json::to_value(&*cs.settings.lock().map_err(|_| "lock")?).map_err(|e| e.to_string())? })) }
pub fn set_settings_inner(cs: &ConsoleState, patch: serde_json::Map<String, Value>) -> Result<Value, String> {
    let mut s = cs.settings.lock().map_err(|_| "lock")?;
    for (k, v) in &patch {
        match k.as_str() {
            "backup_dir" => s.backup_dir = v.as_str().filter(|p| !p.is_empty()).map(std::path::PathBuf::from),
            "autostart" => s.autostart = v.as_bool().ok_or("autostart must be true or false")?,
            other => return Ok(json!({ "ok": false, "error": format!("{other} is not a setting you can change here"), "settings": Value::Null })),
        }
    }
    s.save(&cs.settings_path)?;
    Ok(json!({ "ok": true, "error": Value::Null, "settings": serde_json::to_value(&*s).map_err(|e| e.to_string())? }))
}
```
Plus the four `#[tauri::command]` twins and handler entries. Autostart toggling at runtime: `set_settings` with `autostart` also calls the plugin — that needs the `AppHandle`; do it in the command twin (`app: tauri::AppHandle` parameter → `app.autolaunch().enable()/disable()`), not in `*_inner`.

- [ ] **Step 4: Run** — green, no new warnings.
- [ ] **Step 5: Commit** — "app: sync, backup_now, settings; sync/backup/auto_sync/startup_missed on the topline (Knowlu plan 1, Task 10)".

---

### Task 11: `ui_event`, `mark_seen` at the end of the look, real `engine_newer`

**Files:**
- Modify: `app/src/commands.rs`, `app/src/state.rs`, `app/src/main.rs`
- Test: `app/tests/commands.rs`

**Interfaces:**
- Consumes: `uievents::{record, UiEvent, ACTIONS}`; `runs::git_sha(vault)`.
- Produces: `ui_event_inner(cs, action, view, object_id: Option<String>, object_kind: Option<String>, ms: Option<i64>) -> {ok, error}` (never returns state; never takes the console lock — it is fire-and-forget from the page); `state.rs`: `pub fn refresh_head(cs)` (spawns nothing itself; the caller runs it off the UI thread), and `build_state_value` now sets `topline.engine_newer = CONSOLE_BUILD.is_some() && head.is_some() && CONSOLE_BUILD != head` and `topline.vault_head = head` from `cs.head_sha`. `mark_seen` is unchanged in Rust; **its call site moves in Task 15** (blur/hide), and `topline.seen_at` echoes the stamp so the page can show "since HH:MM".

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ui_events_land_ids_only_and_refuse_text() {
    let v = scratch("uiev");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-uiev-data-{}", std::process::id())));
    let id = first_id(&cs);
    assert_eq!(ui_event_inner(&cs, "object_seen", "today", Some(id.clone()), Some("task".into()), Some(2100))["ok"], true);
    assert_eq!(ui_event_inner(&cs, "object_seen", "today", Some("Exam prep".into()), None, None)["ok"], false);
    assert_eq!(ui_event_inner(&cs, "typed", "today", None, None, None)["ok"], false);
    let files: Vec<_> = std::fs::read_dir(v.join("state/events-ui")).unwrap().flatten().collect();
    assert_eq!(files.len(), 1);
    let text = std::fs::read_to_string(files[0].path()).unwrap();
    assert_eq!(text.lines().count(), 1); assert!(text.contains(&cs.session) && !text.contains("Exam"));
}

#[test]
fn engine_newer_compares_the_console_build_with_the_vaults_head_and_is_false_without_a_repo() {
    let v = scratch("head");
    let cs = ConsoleState::open(v.clone(), std::env::temp_dir().join(format!("qo-head-data-{}", std::process::id())));
    quinn_ops_console::state::refresh_head(&cs);
    let s = state_inner(&cs, "today").unwrap();
    assert_eq!(s["state"]["topline"]["engine_newer"], false, "a non-git scratch vault has no head: never a false alarm");
    assert!(s["state"]["topline"]["vault_head"].is_null());
}
```

- [ ] **Step 2: Run to verify failure.**

- [ ] **Step 3: Implement**

```rust
// state.rs
pub fn refresh_head(cs: &ConsoleState) { let h = quinn_ops::runs::git_sha(&cs.vault); *cs.head_sha.lock().unwrap() = h; }

// commands.rs
pub fn ui_event_inner(cs: &ConsoleState, action: &str, view: &str, object_id: Option<String>, object_kind: Option<String>, ms: Option<i64>) -> Value {
    let ev = quinn_ops::uievents::UiEvent { session: &cs.session, view, action, object_id: object_id.as_deref(), object_kind: object_kind.as_deref(), ms };
    match quinn_ops::uievents::record(&cs.vault, &ev, None) { Ok(()) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e.to_string() }) }
}
#[tauri::command] pub fn ui_event(cs: State<'_, ConsoleState>, action: String, view: String, object_id: Option<String>, object_kind: Option<String>, ms: Option<i64>) -> Value { ui_event_inner(&cs, &action, &view, object_id, object_kind, ms) }

// in build_state_value, replacing the placeholder comparison:
let head = cs.head_sha.lock().map_err(|_| "lock")?.clone();
v["topline"]["console_build"] = json!(CONSOLE_BUILD);
v["topline"]["vault_head"] = json!(head);
v["topline"]["engine_newer"] = json!(matches!((CONSOLE_BUILD, head.as_deref()), (Some(c), Some(h)) if c != h));
v["topline"]["seen_at"] = json!(cs.seen_at());
```
In `main.rs` `setup`, after `app.manage(cs)`: `let h = app.handle().clone(); std::thread::spawn(move || { let cs = h.state::<ConsoleState>(); quinn_ops_console::state::refresh_head(&cs); quinn_ops_console::state::refresh_history(&cs); });` — off the UI thread (F17). Task 12's housekeeping thread refreshes both every 60 s.

- [ ] **Step 4: Run** — green.
- [ ] **Step 5: Commit** — "app: ui_event (ids only), real engine_newer against the vault's HEAD off the UI thread, seen_at echoed (Knowlu plan 1, Task 11)".

---
### Task 12: The scheduler runtime — tick thread, slot execution, run now, pause, missed count, tray state

**Files:**
- Create: `app/src/scheduler.rs`
- Modify: `app/src/main.rs`, `app/src/lib.rs`, `app/src/tray.rs` (menu wiring)
- Test: `app/tests/scheduler.rs`

**Interfaces:**
- Consumes: `schedule::{due_slot, missed_slots, run_starts, SchedulerMode}`, `runs::{load_runners_config, runner_settings, expected_status}`, `history`, `backup`, `state::{run_sync, run_backup, refresh_head, refresh_history}`, `tray::set_state`.
- Produces:
  - `pub struct Scheduler { pub paused: AtomicBool, pub running: Mutex<bool>, pub last: Mutex<Option<RunSummary>> }` managed alongside `ConsoleState`.
  - `pub struct RunSummary { pub started: String, pub ended: String, pub steps: Vec<(String, i32)>, pub ok: bool, pub late: bool }` (`Serialize`; surfaced as `topline.last_slot`).
  - `pub fn engine_exe() -> Result<PathBuf, String>` — `KNOWLU_ENGINE_EXE` if set, else `quinn-ops.exe` beside the running exe, else `Err` naming both.
  - `pub fn slot_argv(vault: &Path, exe: &Path) -> Vec<(PathBuf, Vec<String>)>` — exactly `[(exe, ["coursework","--vault",V,"--via","local-runner"]), (exe, ["rank","--vault",V,"--runner","local"])]`.
  - `pub fn local_runner(vault: &Path) -> Option<RunnerConfig>` — the entry named `local` from `config/runners.yaml`.
  - `pub fn mode(vault: &Path) -> SchedulerMode` and `pub fn device_ok(vault: &Path) -> bool` — `runner_settings(...).device` is `None` or equals `COMPUTERNAME`/`HOSTNAME`.
  - `pub fn run_slot(app: &AppHandle, late: bool) -> RunSummary` — pull (if remote) → each argv as a child with stdio captured to `%LOCALAPPDATA%\quinn-ops\logs\slot-<stamp>-<step>.txt` → `run_sync` (commit by name + push) → `run_backup` → refresh caches → tray state. Never two at once (`running`).
  - `pub fn spawn(app: AppHandle)` — two threads: **tick** every 60 s (mode `App` && `device_ok` && `!paused` → `due_slot` → `run_slot`); **housekeeping** every 10 s (`pending_edits > 0 && last_write ≥ 30 s ago && auto_sync` → `run_sync` + `run_backup`; every 300 s → `run_sync` when `has_remote`; every 60 s → `refresh_head`, `refresh_history`, tray state from `expected_status`).
  - On startup: `startup_missed = missed_slots(cfg, quit_at, now, starts)` when `settings.quit_at` is set, then `quit_at` cleared.
  - Menu: `run-now` → `run_slot(late=false)` on a thread; `pause` toggles `paused` and relabels itself `Resume scheduling`.

- [ ] **Step 1: Write the failing tests**

```rust
// app/tests/scheduler.rs
use quinn_ops_console::scheduler::{engine_exe, slot_argv, mode, device_ok};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf { /* same helper as commands.rs — copy it; tests are separate crates */ }

#[test]
fn the_slot_runs_exactly_the_two_engine_commands_the_script_ran() {
    let v = Path::new(r"C:\v"); let exe = Path::new(r"C:\bin\quinn-ops.exe");
    let argv = slot_argv(v, exe);
    assert_eq!(argv.len(), 2);
    assert_eq!(argv[0].1, vec!["coursework", "--vault", r"C:\v", "--via", "local-runner"]);
    assert_eq!(argv[1].1, vec!["rank", "--vault", r"C:\v", "--runner", "local"]);
    assert!(argv.iter().all(|(e, _)| e == exe));
}

#[test]
fn engine_exe_prefers_the_env_override_and_names_both_places_when_missing() {
    std::env::set_var("KNOWLU_ENGINE_EXE", r"C:\somewhere\quinn-ops.exe");
    assert_eq!(engine_exe().unwrap(), PathBuf::from(r"C:\somewhere\quinn-ops.exe"));
    std::env::remove_var("KNOWLU_ENGINE_EXE");
    match engine_exe() { Ok(p) => assert!(p.ends_with("quinn-ops.exe")), Err(e) => assert!(e.contains("KNOWLU_ENGINE_EXE") && e.contains("quinn-ops.exe"), "{e}") }
}

#[test]
fn the_scheduler_is_inert_on_script_and_only_on_the_designated_device() {
    let v = scratch("mode");
    assert_eq!(mode(&v), quinn_ops::schedule::SchedulerMode::Script, "vault-full's runners.yaml has no scheduler key");
    let yaml = std::fs::read_to_string(v.join("config/runners.yaml")).unwrap();
    std::fs::write(v.join("config/runners.yaml"), yaml.replace("grace_minutes: 20", "grace_minutes: 20\n    scheduler: app\n    device: NOT-THIS-MACHINE")).unwrap();
    assert_eq!(mode(&v), quinn_ops::schedule::SchedulerMode::App);
    assert!(!device_ok(&v), "a second install never double-runs a shared vault");
}

#[test]
#[ignore = "runs the real engine exe against a scratch vault; needs a built target/release/quinn-ops.exe and network for rank's feeds — run by hand before Task 17"]
fn run_slot_end_to_end() { /* documented in the task; not part of `cargo test` */ }
```

- [ ] **Step 2: Run to verify failure** — module not found.

- [ ] **Step 3: Implement** (`app/src/scheduler.rs`)

```rust
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use quinn_ops::runs::RunnerConfig;
use quinn_ops::schedule::{self, SchedulerMode};
use crate::state::{self, ConsoleState};
use crate::tray::{self, TrayState};

pub struct Scheduler { pub paused: AtomicBool, pub running: Mutex<bool>, pub last: Mutex<Option<RunSummary>> }
impl Default for Scheduler { fn default() -> Self { Scheduler { paused: AtomicBool::new(false), running: Mutex::new(false), last: Mutex::new(None) } } }

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunSummary { pub started: String, pub ended: String, pub steps: Vec<(String, i32)>, pub ok: bool, pub late: bool }

pub fn engine_exe() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("KNOWLU_ENGINE_EXE") { if !p.is_empty() { return Ok(PathBuf::from(p)); } }
    let sibling = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("quinn-ops.exe")));
    match sibling { Some(p) if p.is_file() => Ok(p), Some(p) => Err(format!("engine not found: {} (or set KNOWLU_ENGINE_EXE)", p.display())), None => Err("engine not found: no sibling quinn-ops.exe and KNOWLU_ENGINE_EXE unset".into()) }
}

pub fn slot_argv(vault: &Path, exe: &Path) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    vec![
        (exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]),
        (exe.to_path_buf(), vec!["rank".into(), "--vault".into(), v, "--runner".into(), "local".into()]),
    ]
}

pub fn local_runner(vault: &Path) -> Option<RunnerConfig> {
    quinn_ops::runs::load_runners_config(&vault.join("config").join("runners.yaml")).ok()?.into_iter().find(|r| r.name == "local")
}
pub fn mode(vault: &Path) -> SchedulerMode { quinn_ops::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").scheduler }
pub fn device_ok(vault: &Path) -> bool {
    match quinn_ops::runs::runner_settings(&vault.join("config").join("runners.yaml"), "local").device {
        None => true,
        Some(d) => ["COMPUTERNAME", "HOSTNAME"].iter().filter_map(|k| std::env::var(k).ok()).any(|h| h.eq_ignore_ascii_case(&d)),
    }
}

fn log_dir() -> PathBuf { let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir()); let d = base.join("quinn-ops").join("logs"); let _ = std::fs::create_dir_all(&d); d }

fn run_child(exe: &Path, args: &[String], log: &Path) -> i32 {
    let out = Command::new(exe).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output();
    match out {
        Ok(o) => { let _ = std::fs::write(log, [o.stdout.as_slice(), b"\n--- stderr ---\n", o.stderr.as_slice()].concat()); o.status.code().unwrap_or(-1) }
        Err(e) => { let _ = std::fs::write(log, format!("spawn failed: {e}")); -1 }
    }
}

pub fn run_slot(app: &AppHandle, late: bool) -> RunSummary {
    let cs = app.state::<ConsoleState>();
    let sch = app.state::<Scheduler>();
    { let mut r = sch.running.lock().unwrap(); if *r { return sch.last.lock().unwrap().clone().unwrap_or(RunSummary { started: String::new(), ended: String::new(), steps: vec![], ok: false, late }); } *r = true; }
    let started = quinn_ops::journal::now_ts(None);
    let mut steps = Vec::new();
    if cs.history.lock().unwrap().has_remote { let out = state::run_sync(&cs); steps.push(("pull".to_string(), if out.status.last_error.is_none() { 0 } else { 1 })); }
    match engine_exe() {
        Ok(exe) => { for (i, (e, args)) in slot_argv(&cs.vault, &exe).into_iter().enumerate() { let log = log_dir().join(format!("slot-{}-{}-{}.txt", started.replace(':', ""), i, args[0])); let code = run_child(&e, &args, &log); steps.push((args[0].clone(), code)); } }
        Err(e) => { steps.push((format!("engine: {e}"), -1)); }
    }
    let out = state::run_sync(&cs); steps.push(("push".to_string(), if out.status.last_error.is_none() { 0 } else { 1 }));
    if let Ok(st) = state::run_backup(&cs, jiff::Timestamp::now()) { steps.push(("backup".to_string(), if st.last_error.is_none() { 0 } else { 1 })); }
    state::refresh_head(&cs); state::refresh_history(&cs);
    let ok = steps.iter().all(|(_, c)| *c == 0);
    let summary = RunSummary { started, ended: quinn_ops::journal::now_ts(None), steps, ok, late };
    *sch.last.lock().unwrap() = Some(summary.clone());
    tray::set_state(app, if ok { TrayState::Ok } else { TrayState::Warn });
    *sch.running.lock().unwrap() = false;
    summary
}

fn now_in(vault: &Path) -> jiff::Zoned { jiff::Zoned::now().with_time_zone(quinn_ops::cli::vault_zone(vault)) }

pub fn spawn(app: AppHandle) {
    // startup: missed slots while quit
    {
        let cs = app.state::<ConsoleState>();
        let quit_at = { let mut s = cs.settings.lock().unwrap(); let q = s.quit_at.take(); let _ = s.save(&cs.settings_path); q };
        if let (Some(q), Some(cfg)) = (quit_at, local_runner(&cs.vault)) {
            if let Ok(t) = q.parse::<jiff::Timestamp>() {
                let tz = quinn_ops::cli::vault_zone(&cs.vault); let now = now_in(&cs.vault);
                let starts = schedule::run_starts(&cs.vault, "local", t.to_zoned(tz.clone()).date());
                cs.startup_missed.store(schedule::missed_slots(&cfg, &t.to_zoned(tz), &now, &starts), Ordering::SeqCst);
            }
        }
    }
    let tick = app.clone();
    std::thread::spawn(move || loop {
        {
            let cs = tick.state::<ConsoleState>(); let sch = tick.state::<Scheduler>();
            if mode(&cs.vault) == SchedulerMode::App && device_ok(&cs.vault) && !sch.paused.load(Ordering::SeqCst) {
                if let Some(cfg) = local_runner(&cs.vault) {
                    let now = now_in(&cs.vault);
                    let starts = schedule::run_starts(&cs.vault, "local", now.date().yesterday().unwrap_or(now.date()));
                    if let Some(due) = schedule::due_slot(&cfg, &now, &starts) { let _ = run_slot(&tick, due.late); }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(60));
    });
    let house = app.clone();
    std::thread::spawn(move || { let mut n: u64 = 0; loop {
        std::thread::sleep(std::time::Duration::from_secs(10)); n += 1;
        let cs = house.state::<ConsoleState>();
        let due_write = cs.last_write.lock().unwrap().map(|t| t.elapsed().as_secs() >= 30).unwrap_or(false) && cs.pending_edits.load(Ordering::SeqCst) > 0;
        if due_write && cs.auto_sync.load(Ordering::SeqCst) { let _ = state::run_sync(&cs); let _ = state::run_backup(&cs, jiff::Timestamp::now()); *cs.last_write.lock().unwrap() = None; }
        if n % 30 == 0 && cs.history.lock().unwrap().has_remote && cs.auto_sync.load(Ordering::SeqCst) { let _ = state::run_sync(&cs); }
        if n % 6 == 0 {
            state::refresh_head(&cs); state::refresh_history(&cs);
            let warn = quinn_ops::runs::expected_status(&cs.vault, jiff::Timestamp::now()).map(|rows| rows.iter().any(|r| r.status == "missing" || r.status == "crashed")).unwrap_or(true);
            let last_bad = house.state::<Scheduler>().last.lock().unwrap().as_ref().map(|l| !l.ok).unwrap_or(false);
            tray::set_state(&house, if warn || last_bad { TrayState::Warn } else { TrayState::Ok });
        }
    }});
}
```
`main.rs`: `app.manage(Scheduler::default()); scheduler::spawn(app.handle().clone());` in `setup`; `tray.rs` menu: `"run-now"` → `let h = app.clone(); std::thread::spawn(move || { let _ = crate::scheduler::run_slot(&h, false); });`; `"pause"` → toggle `paused` and `item.set_text(...)` (keep a handle to the `MenuItem` in a `Mutex<Option<MenuItem<_>>>` inside `Scheduler` or rebuild the menu). `build_state_value` adds `topline.last_slot` from `Scheduler.last` and `topline.scheduler = {mode, paused, device_ok}` (strings/bools only). The `knowlu://scheduler` event from Task 7 is no longer needed — remove it.

- [ ] **Step 4: Run** — `cd app; cargo test 2>&1 | tail -5` (3 pass, 1 ignored); `cargo build --release`; launch against a scratch vault whose `runners.yaml` has `scheduler: app` and no `device:` line (edit the **scratch** copy only), set `KNOWLU_ENGINE_EXE` to `target\release\quinn-ops.exe`, pick *Run now*, confirm `state/runs/` in the scratch vault gained a `local` run and the tray icon reflects the result. That is the `#[ignore]` test done by hand; record the outcome in the task report. **The scratch vault's `rank` fetches the calendar feed once — that is the one live fetch this plan allows, counted against the 429 budget.**
- [ ] **Step 5: Commit** — "app: scheduler — tick and housekeeping threads, slot execution as child processes, run now, pause, missed count on launch, tray state; inert on scheduler: script (Knowlu plan 1, Task 12)".

---

### Task 13: The page writes — click-to-edit, progress drag, new-task row, delete

**Files:**
- Modify: `app/static/console.js`, `app/static/console.css`, `app/static/index.html`
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `set_fields`, `create_task`, `delete_note` envelopes.
- Produces (new `console.js` functions, added to the name list in `static_assets.rs`): `editField(rowEl, field)`, `commitEdit(id, fields)`, `cancelEdit()`, `bindProgressDrag(trackEl, id)`, `renderNewTaskRow()`, `submitNewTask()`, `confirmDelete(id, title)`, `applyEnvelope(env)` (paints `env.state` with `force = true` — a write's return is never a held reorder, F19), `showRefusal(el, message)`.
- Rows carry `data-field` spans for `title`, `course`, `due`, `effort_hours`, `importance`, `status`; the drawer's `<dl>` carries `data-field` on every editable key; `id`, `source_uid`, `also_uids`, `judgment` render with class `ro`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_page_can_write_and_never_relabels_to_confirm() {
    let js = read("console.js");
    for f in ["editField", "commitEdit", "cancelEdit", "bindProgressDrag", "renderNewTaskRow", "submitNewTask", "confirmDelete", "applyEnvelope", "showRefusal"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    for cmd in ["\"set_fields\"", "\"create_task\"", "\"delete_note\""] { assert!(js.contains(cmd), "{cmd}"); }
    assert!(js.contains("paint(env.state, true)"), "a write's returned state is painted with force (F19)");
    assert!(js.contains("Escape") && js.contains("Enter"), "blur/Enter commits, Escape cancels");
    assert!(js.contains("progress\": 100") || js.contains("progress: 100") || js.contains("pct === 100"), "reaching 100 sets status done in the same write");
    assert!(!js.contains("Confirm?") && !js.contains("Are you sure"), "one click commits; delete uses the confirm() dialog, not a relabel");
    assert!(js.contains("readonly") || js.contains("class=\"ro\""), "id/source_uid/also_uids/judgment are shown read-only");
    let html = read("index.html");
    assert!(html.contains("id=\"newtask\""), "the new-task button lives in MUST DO's header");
}
```

- [ ] **Step 2: Run to verify failure.**

- [ ] **Step 3: Implement** (representative code; the rest follows the same shape)

```js
  // ----- writes (Knowlu plan 1, Task 13). Every mutating call returns the fresh state; paint it with force.
  function applyEnvelope(env, onRefusal) {
    if (env.state) { current.pendingOrder = null; paint(env.state, true); }
    if (!env.ok) { if (onRefusal) { onRefusal(env.error || "refused"); } else { EL("delta").textContent = "refused: " + env.error; } }
    return env.ok;
  }
  function showRefusal(el, message) { var n = document.createElement("span"); n.className = "refusal"; n.textContent = message; el.appendChild(n); setTimeout(function () { n.remove(); }, 6000); }
  function commitEdit(id, fields, el) {
    return invoke("set_fields", { view: current.view, id: id, fields: fields }).then(function (env) { return applyEnvelope(env, function (msg) { showRefusal(el, msg); }); });
  }
  var editing = null;
  function cancelEdit() { if (editing) { editing.el.textContent = editing.old; editing = null; ev("edit_cancelled", null, null, null); } }
  function editField(el, id, field) {
    if (editing) { cancelEdit(); }
    var old = el.textContent, input = document.createElement("input");
    input.type = field === "due" ? "datetime-local" : (field === "effort_hours" || field === "importance" || field === "slice_hours" || field === "rank_override" ? "number" : "text");
    if (field === "importance") { input.min = 1; input.max = 5; }
    input.value = field === "due" ? old.replace(" ", "T") : old;
    editing = { el: el, old: old, id: id, field: field };
    el.textContent = ""; el.appendChild(input); input.focus(); input.select();
    ev("edit_started", id, "task", null);
    function done(commit) {
      if (!editing) { return; }
      var v = input.value.trim(); editing = null;
      if (!commit || v === old) { el.textContent = old; return; }
      var fields = {}; fields[field] = (input.type === "number" && v !== "") ? Number(v) : v;
      el.textContent = v;
      commitEdit(id, fields, el).then(function (ok) { ev(ok ? "edit_committed" : "edit_cancelled", id, "task", null); });
    }
    input.addEventListener("keydown", function (e) { if (e.key === "Enter") { e.preventDefault(); done(true); } else if (e.key === "Escape") { e.preventDefault(); done(false); } });
    input.addEventListener("blur", function () { done(true); });
  }
  function bindProgressDrag(track, id) {
    var dragging = false, pct = 0;
    function at(e) { var r = track.getBoundingClientRect(); var raw = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)); return Math.round(raw * 20) * 5; }
    track.addEventListener("pointerdown", function (e) { dragging = true; track.setPointerCapture(e.pointerId); pct = at(e); track.style.setProperty("--pct", pct + "%"); });
    track.addEventListener("pointermove", function (e) { if (dragging) { pct = at(e); track.style.setProperty("--pct", pct + "%"); } });
    track.addEventListener("pointerup", function () {
      if (!dragging) { return; } dragging = false;
      var fields = { progress: pct }; if (pct === 100) { fields.status = "done"; }   // one journal moment, one CLOSED line
      commitEdit(id, fields, track);
    });
  }
  function renderNewTaskRow() {
    var host = EL("mustdo"); if (host.querySelector(".newrow")) { return; }
    var courses = (current.state && current.state.courses) || [];
    var row = document.createElement("form"); row.className = "row newrow";
    row.innerHTML = '<span class="pip"></span><div class="ttl"><input name="title" type="text" placeholder="What needs doing? (Knowlu)" required autofocus>'
      + '<input name="course" list="courses" placeholder="course"><datalist id="courses">' + courses.map(function (c) { return "<option value=\"" + h(c) + "\">"; }).join("") + "</datalist>"
      + '<input name="due" type="date"><input name="effort_hours" type="number" step="0.5" min="0" value="1.0"></div>'
      + '<button class="b pri" type="submit">Add</button><button class="b" type="button" data-cancel>Cancel</button>';
    host.prepend(row);
    row.addEventListener("submit", function (e) { e.preventDefault(); submitNewTask(row); });
    row.querySelector("[data-cancel]").addEventListener("click", function () { row.remove(); });
    row.querySelector("[name=title]").focus();
  }
  function submitNewTask(row) {
    var f = new FormData(row), fields = { title: f.get("title"), course: f.get("course") || null, due: f.get("due") || null, effort_hours: Number(f.get("effort_hours") || 1) };
    invoke("create_task", { view: current.view, fields: fields }).then(function (env) { if (applyEnvelope(env, function (m) { showRefusal(row, m); })) { row.remove(); } });
  }
  function confirmDelete(id, title) {
    if (!window.confirm("Archive \"" + title + "\"? Nothing is deleted — it moves to archive/ and shows in CLOSED THIS WEEK.")) { return; }
    invoke("delete_note", { view: current.view, id: id }).then(function (env) { if (applyEnvelope(env)) { EL("drawer").hidden = true; } });
  }
```
`rowHtml` gains `data-field` spans: `<span class="a" data-field="title">…</span>`, the chips `data-field="course"`, `data-field="due"`, the hours cell `data-field="effort_hours"`; the delegated click handler: a click on `[data-field]` inside a row → `editField(target, rowId, field)` and **does not open the drawer**; the `.track` gets `bindProgressDrag` in `paint` after each list render (or delegated pointer events). The drawer's `<dl>` adds `data-field` on every `EDITABLE` key and a `Delete…` button calling `confirmDelete`. `index.html`: `<button class="b" id="newtask">+ task</button>` inside `#mustdo`'s `.sec-hd`. **`state.courses`** does not exist yet — add a read-only `courses: Vec<String>` (slugs from `courses/*.md` filenames) to `State` in `surface.rs`? **No: that would move the references.** Use the course slugs already present on rows (`current.state` rows' `course` values, de-duplicated) for the datalist in this plan; the proper `courses` field is a plan-2-of-4 item. CSS: `.refusal` (crit colour, small), `.newrow input`, `.track` with `--pct` fill, `.ro` muted.

- [ ] **Step 4: Run** — `cd app; cargo test 2>&1 | tail -5` green; `cargo build --release`; look at it against a scratch vault: click a title, change it, Enter → the row updates and the journal has a `quinn`/`dashboard` record; drag a track to 100 → the row leaves MUST DO and CLOSED THIS WEEK gains a line; `+ task` → a row → Add → it appears where the ranking puts it; drawer → Delete → confirm → archived.
- [ ] **Step 5: Commit** — "page: click-to-edit, progress drag with done-at-100, the new-task row, delete behind confirm — writes painted with force (Knowlu plan 1, Task 13)".

---

### Task 14: The deck acts — approve/reject/snooze, the issue flag popover, info close

**Files:**
- Modify: `app/static/console.js`, `app/static/console.css`
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `decide`, `open_issue`, `close_info` envelopes; `state.decisions` (cards, pending, oldest_days, budget_*, deferred, awaiting_calendar, events_in_digest); `Row.judged`.
- Produces: `bindDeck()`, `decideCard(id, verdict, snoozeUntil)`, `openFlag(anchorEl, targetId)`, `submitFlag(popEl, targetId)`, `closeInfoItem(id)`. Deck buttons enabled; note field always visible; snooze in the `…` overflow with a date input defaulting to tomorrow; the 260 ms slide disabled under `prefers-reduced-motion`; the header "clear" rule stays R31's. The ⚑ appears on every row/card whose `judged` is true and opens a popover with the eight `issues::CATEGORIES` as toggle chips **and** a textarea, submit disabled until one chip is on.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_deck_acts_and_the_flag_has_eight_chips_and_free_text() {
    let js = read("console.js");
    for f in ["bindDeck", "decideCard", "openFlag", "submitFlag", "closeInfoItem"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    for cmd in ["\"decide\"", "\"open_issue\"", "\"close_info\""] { assert!(js.contains(cmd), "{cmd}"); }
    for cat in ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"] { assert!(js.contains(cat), "{cat}"); }
    assert!(js.contains("<textarea"), "free text, always, beside the chips");
    assert!(!js.contains("disabled>Approve") && !js.contains("disabled>Reject"), "the deck's buttons are live now");
    assert!(js.contains("decision_made") && js.contains("decision_deferred") && js.contains("issue_opened"));
    let css = read("console.css");
    assert!(css.contains("prefers-reduced-motion") && css.contains(".deck .card.gone"));
}
```

- [ ] **Step 2: Run to verify failure.**

- [ ] **Step 3: Implement**

```js
  function decideCard(id, verdict, snoozeUntil) {
    var card = EL("deck").querySelector('.card[data-id="' + id + '"]');
    var note = card ? (card.querySelector(".nb input").value || "") : "";
    if (card) { card.classList.add("gone"); }
    return invoke("decide", { view: current.view, id: id, verdict: verdict, note: note, snoozeUntil: snoozeUntil || null }).then(function (env) {
      ev(verdict === "snoozed" ? "decision_deferred" : "decision_made", id, "approval", null);
      if (!applyEnvelope(env, function (m) { if (card) { card.classList.remove("gone"); showRefusal(card, m); } })) { return; }
      if (env.decision && env.decision.warnings && env.decision.warnings.length) { EL("delta").textContent = "decision: " + env.decision.warnings.join("; "); }
    });
  }
  function bindDeck() {
    var deck = EL("deck");
    deck.addEventListener("click", function (e) {
      var b = e.target.closest("button[data-verdict]"); if (!b) { return; }
      var id = b.closest(".card").getAttribute("data-id");
      if (b.getAttribute("data-verdict") === "snoozed") {
        var d = b.closest(".card").querySelector("input[type=date]"); if (d.hidden) { d.hidden = false; d.focus(); return; }
        decideCard(id, "snoozed", d.value);
      } else { decideCard(id, b.getAttribute("data-verdict"), null); }
    });
  }
  function openFlag(anchor, targetId) {
    var cats = ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"];
    var pop = document.createElement("div"); pop.className = "flagpop";
    pop.innerHTML = "<div class=\"chips\">" + cats.map(function (c) { return '<button type="button" class="chip" data-cat="' + c + '">' + c + "</button>"; }).join("") + "</div>"
      + '<textarea placeholder="what is wrong, in your words"></textarea><div class="acts"><button class="b pri" data-send disabled>Flag</button><button class="b" data-cancel>Cancel</button></div>';
    anchor.closest(".row, .card, .ln").appendChild(pop);
    pop.addEventListener("click", function (e) {
      var chip = e.target.closest(".chip"); if (chip) { chip.classList.toggle("on"); pop.querySelector("[data-send]").disabled = !pop.querySelector(".chip.on"); return; }
      if (e.target.closest("[data-cancel]")) { pop.remove(); }
      if (e.target.closest("[data-send]")) { submitFlag(pop, targetId); }
    });
  }
  function submitFlag(pop, targetId) {
    var cats = Array.prototype.map.call(pop.querySelectorAll(".chip.on"), function (c) { return c.getAttribute("data-cat"); });
    invoke("open_issue", { view: current.view, target: targetId, categories: cats, text: pop.querySelector("textarea").value }).then(function (env) {
      if (applyEnvelope(env, function (m) { showRefusal(pop, m); })) { ev("issue_opened", targetId, "task", null); pop.remove(); }
    });
  }
  function closeInfoItem(id) { invoke("close_info", { view: current.view, id: id }).then(applyEnvelope); }
```
`renderDeck`: remove `disabled` from the buttons, add `data-verdict="approved|rejected"` and an overflow `…` button `data-verdict="snoozed"` with a hidden `<input type=date>` beside it; `rowHtml`/cards/`.ln` info rows add `<button class="flag" data-flag="<id>">⚑</button>` when `judged`; GOOD TO KNOW rows add `<button class="b" data-close-info="<id>">close</button>`; the delegated handler routes `[data-flag]` → `openFlag`, `[data-close-info]` → `closeInfoItem`. CSS: `.flagpop`, `.chip`, `.chip.on`, `.deck .card.gone { transform: translateX(110%); opacity: 0; transition: 260ms }`, `@media (prefers-reduced-motion: reduce) { .deck .card.gone { transition: none } }` (the reduced-motion block exists; extend it).

- [ ] **Step 4: Run** — tests green; build; against a scratch vault approve one task proposal and watch it land in MUST DO in the same paint; snooze one; flag a row with two chips and text; close an info item.
- [ ] **Step 5: Commit** — "page: the deck acts (approve/reject/snooze, one click, fixed height), the ⚑ popover with eight chips and free text, info close (Knowlu plan 1, Task 14)".

---

### Task 15: Topline sync/backup/scheduler, the Runs view, end-of-look `mark_seen`, event emission

**Files:**
- Modify: `app/static/console.js`, `app/static/console.css`, `app/static/index.html`
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `topline.sync`, `topline.backup`, `topline.auto_sync`, `topline.startup_missed`, `topline.last_slot`, `topline.scheduler`, `topline.engine_newer`, `topline.seen_at`; `runs_panel` (+ `warnings`); `state.warnings`; `sync`, `backup_now`, `mark_seen`, `ui_event`.
- Produces: `renderSyncLine(state)` (in the topline: `synced` / `N edits pending push` amber / `conflict — auto-sync stopped` crit / `no remote — local history` calm; `backup 3 days behind` amber; `N slots missed while quit`; `engine newer than console`; a `sync now` button and `back up now` button); `renderRunsView(state)` — the Runs nav entry becomes real: expected rows, every recent run with its full summary, `runs_panel.warnings` and `state.warnings` in a `.warnings` block; `endOfLook()` — calls `mark_seen` on `window` `blur` and on `visibilitychange` → hidden, **never at first paint**; `ev(action, id, kind, ms)` — the `ui_event` wrapper, fire-and-forget; `watchSeen()` — an `IntersectionObserver` over `[data-id]` elements that emits `object_seen` after 2 000 ms in view, once per id per session; `view_opened` on every `route`, `sync_run` on manual sync, `delta_expanded`/`why_expanded` on those toggles.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn seen_is_stamped_at_the_end_of_the_look_and_events_are_emitted() {
    let js = read("console.js");
    for f in ["renderSyncLine", "renderRunsView", "endOfLook", "ev", "watchSeen"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    assert_eq!(js.matches("\"mark_seen\"").count(), 1, "mark_seen is invoked from exactly one place");
    assert!(js.contains("\"blur\", endOfLook") && js.contains("visibilitychange"), "stamped on blur/hide (R37)");
    assert!(!js.contains("mark_seen\", {}).then") || !js.contains("route(location.hash.slice(1)).then(function () { if (current.state) { invoke(\"mark_seen\""), "not at first paint");
    for a in ["view_opened", "object_seen", "sync_run", "delta_expanded", "why_expanded"] { assert!(js.contains(a), "{a}"); }
    assert!(js.contains("IntersectionObserver") && js.contains("2000"));
    assert!(js.contains("\"sync\"") && js.contains("\"backup_now\""));
    assert!(!js.contains("arrives with the third slice") || js.contains("runs: renderRunsView"), "Runs is built now; the not-built column no longer names it");
    let html = read("index.html");
    assert!(html.contains("id=\"main-runs\"") && html.contains("id=\"syncline\""));
}
```

- [ ] **Step 2: Run to verify failure.**

- [ ] **Step 3: Implement**

```js
  function ev(action, objectId, objectKind, ms) {
    invoke("ui_event", { action: action, view: current.view, objectId: objectId || null, objectKind: objectKind || null, ms: ms == null ? null : ms }).catch(function () {});
  }
  var seenOnce = {};
  var seenObserver = ("IntersectionObserver" in window) ? new IntersectionObserver(function (entries) {
    entries.forEach(function (en) {
      var id = en.target.getAttribute("data-id"); if (!id) { return; }
      if (en.isIntersecting) { en.target._seenAt = Date.now(); if (!seenOnce[id]) { setTimeout(function () { if (en.target._seenAt && Date.now() - en.target._seenAt >= 2000 && !seenOnce[id]) { seenOnce[id] = true; ev("object_seen", id, en.target.getAttribute("data-kind") || "task", 2000); } }, 2000); } }
      else { en.target._seenAt = null; }
    });
  }, { threshold: 0.6 }) : null;
  function watchSeen() { if (!seenObserver) { return; } document.querySelectorAll("[data-id]").forEach(function (el) { seenObserver.observe(el); }); }
  function endOfLook() { invoke("mark_seen", {}).catch(function () {}); }
  function renderSyncLine(state) {
    var t = state.topline, s = t.sync || {}, b = t.backup || {}, bits = [];
    if (!s.is_repo) { bits.push('<span class="calm">local history</span>'); }
    else if (s.conflicted && s.conflicted.length) { bits.push('<span class="crit">conflict in ' + s.conflicted.length + " note" + (s.conflicted.length === 1 ? "" : "s") + " — auto-sync stopped</span>"); }
    else if (s.last_error) { bits.push('<span class="amber">' + h(s.last_error.split("\n")[0]) + "</span>"); }
    else if (!s.has_remote) { bits.push('<span class="calm">no remote — local history</span>'); }
    else if (s.ahead > 0) { bits.push('<span class="amber">' + s.ahead + " commit" + (s.ahead === 1 ? "" : "s") + " pending push</span>"); }
    else { bits.push('<span class="calm">synced</span>'); }
    if (b.last_error) { bits.push('<span class="amber">backup: ' + h(b.last_error.split("\n")[0]) + "</span>"); }
    else if (b.behind_days != null && b.behind_days >= 1) { bits.push('<span class="amber">backup ' + b.behind_days + " day" + (b.behind_days === 1 ? "" : "s") + " behind</span>"); }
    else if (b.last_ok) { bits.push('<span class="calm">backed up</span>'); }
    if (t.startup_missed > 0) { bits.push('<span class="amber">' + t.startup_missed + " slot" + (t.startup_missed === 1 ? "" : "s") + " missed while quit</span>"); }
    if (t.engine_newer) { bits.push('<span class="amber">engine newer than console</span>'); }
    if (t.scheduler && t.scheduler.mode === "app") { bits.push('<span class="' + (t.scheduler.paused ? "amber" : "calm") + '">scheduler ' + (t.scheduler.paused ? "paused" : "on") + "</span>"); }
    EL("syncline").innerHTML = bits.join(" · ") + ' <button class="b" data-sync>sync now</button> <button class="b" data-backup>back up now</button>';
  }
  function renderRunsView(state) {
    var p = state.runs_panel, html = "";
    p.expected.forEach(function (e) { html += '<div class="ln run ' + (e.status === "seen" ? "" : "wn") + '"><span class="k">' + h(e.status) + '</span><span>' + h(e.runner) + '</span><span class="rt">' + h(e.due) + "</span></div>"; });
    p.recent.forEach(function (r) { html += '<div class="ln run ' + (r.result === "ok" ? "" : "wn") + '"><span class="k">' + h(r.result) + '</span><span>' + h(r.runner) + " " + h((r.started || "").slice(0, 16)) + '</span><span class="rt">' + h(r.run_id) + '</span><pre class="summary">' + h(r.summary) + "</pre></div>"; });
    var warns = (p.warnings || []).concat(state.warnings || []);
    if (warns.length) { html += '<div class="warnings">' + warns.map(function (w) { return "<div>" + h(w) + "</div>"; }).join("") + "</div>"; }
    if (p.empty_text) { html += '<div class="empty">' + h(p.empty_text) + "</div>"; }
    EL("runs-view").innerHTML = html;
  }
```
Wiring: `window.addEventListener("blur", endOfLook); document.addEventListener("visibilitychange", function () { if (document.hidden) { endOfLook(); } });` and **delete** the `mark_seen` call from the boot line. `paint` calls `renderSyncLine` always, `watchSeen` after lists render, and toggles a fourth main body `#main-runs` for `view === "runs"` (remove `runs` from the not-built set; the other three stay not-built until plan 2 of 4). `route` emits `view_opened`. The delegated handler routes `[data-sync]` → `invoke("sync", {view})` then `applyEnvelope` + `ev("sync_run")`, `[data-backup]` → `invoke("backup_now", …)`. `index.html`: `<div id="syncline" class="syncline"></div>` under the topline; `<section id="main-runs" hidden><h1>Runs</h1><div id="runs-view"></div></section>`. CSS: `.syncline`, `.calm`, `.amber`, `.crit`, `.ln.run .summary`, `.warnings`.

- [ ] **Step 4: Run** — tests green; build; against a scratch vault: blur the window and refocus — the delta line now reads "since HH:MM" with the blur time; open Runs — expected rows and recent runs with full summaries; `sync now` on the scratch (no remote) reads "no remote — local history"; set a backup folder, `back up now`, the line reads "backed up" and the folder holds `vault/` and `snapshots/`.
- [ ] **Step 5: Commit** — "page: sync/backup/scheduler topline with amber states, the Runs view, mark_seen at the end of the look, interaction events with object_seen ≥ 2 s (Knowlu plan 1, Task 15)".

---

### Task 16: Look at it — screenshots, anatomy, README, HANDOFF, CLAUDE.md

**Files:**
- Modify: `docs/surface/anatomy.md`, `app/README.md`, `docs/HANDOFF.md`, `CLAUDE.md` (`## The console` → `## Knowlu (the console)`), `docs/superpowers/plans/2026-09-04-knowlu-foundation-plan.md` (status line)
- Run: `scripts/console-shots.py` (unchanged) against `tests/fixtures/surface-today-full.json` — the fixture now carries the Task 1 shape; the page must render it.

- [ ] **Step 1: Screenshots at eight viewports**

```
python -m venv .wv; .wv\Scripts\python -m pip install playwright; .wv\Scripts\python -m playwright install chromium
.wv\Scripts\python scripts\console-shots.py tests\fixtures\surface-today-full.json shots\
```
Expected: every viewport ≥ 820 px `ok`; exit 0. Look at `shots\1280.png` and `shots\820.png` yourself: the new-task button, the sync line, the ⚑ marks, the deck's live buttons, the drawer's Delete. Then one real-exe capture at 1280×860 against a scratch vault, **by window handle** (`PrintWindow`), and one at 1440.

- [ ] **Step 2: Anatomy** — add or update sections: 3.1 Topline (sync line, engine newer, missed slots), 3.9 DECISIONS (acts now; `decide` path; R31), 3.14 RUNS (a view now; warnings block), 4.1 ⚑ (popover shape), 4.3 Click-to-edit (the twelve fields; `data-field`; progress 100 rule), 4.6 Interaction events (what is emitted, when, ids only, `object_seen` 2 s), a new 4.7 "Sync, backup, the scheduler" with the amber/crit vocabulary, and the §6 parity table rows for the deck and the flags. Every `Computed by:` bullet names the payload key.

- [ ] **Step 3: README, HANDOFF, CLAUDE.md** — README: what the crate is now, the eleven commands, the tray menu, `scheduler: app` and when it flips, `KNOWLU_ENGINE_EXE`, the scratch-vault rule, the identifier placeholder, the `.rsrc` warning still open. HANDOFF: a **▶ KNOWLU PLAN 1 DONE** block above the console block: what shipped, that the console is still on scratch, the go-live checklist's path (Task 17), the 429 fetch spent in Task 12 if it was. CLAUDE.md `## The console`: rename the heading, add three lines — the app writes through `write` with `quinn`/`dashboard`; the scheduler is inert until `scheduler: app`; the console is on scratch until Task 17's checklist is run after G2.

- [ ] **Step 4: Commit** — `git add docs/surface/anatomy.md app/README.md docs/HANDOFF.md CLAUDE.md docs/superpowers/plans/2026-09-04-knowlu-foundation-plan.md` — "docs: Knowlu plan 1 — anatomy for writes/deck/flags/sync/runs, README, handoff (Knowlu plan 1, Task 16)". `shots/` stays ignored.

---

### Task 17: Go-live after G2 — the checklist (Quinn-gated; nothing here runs before G2)

**Files:**
- Create: `docs/runners/knowlu-go-live.md`
- Modify: `docs/HANDOFF.md` (point at it); the Start-menu shortcut (by hand, Quinn's machine)

This task produces a document and is executed only when Quinn has given **G2** (cutover plan Task 8) and the switch commit is in. Until then the console keeps running against scratch copies. The checklist, verbatim into the file:

```markdown
# Knowlu go-live (after G2) — the switch to the live vault

Preconditions, all true before step 1:
- [ ] G2 given and cutover Task 8's commit is on main (`$mode = "rust-live"` in scripts/local-run.ps1).
- [ ] `cd app; cargo test` green; root `cargo test` green; both dual-run scripts clean at HEAD.
- [ ] The last three scratch-vault sessions of Knowlu show no `refused:` lines that were not intended and the journal shows every edit as quinn/dashboard.
- [ ] A backup folder exists on the laptop (OneDrive or an external drive) and `back up now` against a scratch vault filled it.

Steps:
1. Build: `cd app; cargo build --release` in the MAIN checkout (not a worktree). Confirm `app\target\release\knowlu.exe` and `target\release\quinn-ops.exe` are from the same HEAD (`knowlu.exe` topline `build` == `git rev-parse --short HEAD`).
2. Update the Start-menu shortcut `quinn-ops.lnk` → rename to `Knowlu.lnk`; target `app\target\release\knowlu.exe`, args `--vault "C:\Users\danie\GitHub\quinn-ops"`, icon `app\icons\icon.ico`.
3. Launch from the shortcut. Settings → backup folder → pick the folder. `back up now`. Confirm `<folder>\<profile>\vault\tasks` exists.
4. Make one edit (an importance) and wait 30 s: the sync line goes `1 commit pending push` → `synced`. Confirm on GitHub that a `surface: 1 edit` commit landed on main.
5. Close the window (it hides). Confirm the tray icon. Quit from the tray. Relaunch — no missed-slot line (the script runner still owns slots).
6. Stop Obsidian Git: in Obsidian, disable the plugin's auto-commit and auto-push. Do NOT uninstall Obsidian yet — it is the rollback viewer for the reverse week.
7. Record the switch in docs/HANDOFF.md and state/dual-run-log.md's neighbour, `state/knowlu-log.md`: `- <date> knowlu live vault=<path> build=<sha>`.

Rollback (any step fails): point the shortcut back at the previous exe, re-enable Obsidian Git. The vault has only ever been written through `write`; nothing needs undoing.

What does NOT happen here: `scheduler: app` (phase 2, after the harness comes out ~09-18); renaming anything the engine, the runner or the routine reads; removing the remote.
```

- [ ] **Step 1: Write the file** (CRLF), link it from HANDOFF's Knowlu block, commit: "docs: Knowlu go-live checklist — the switch to the live vault after G2 (Knowlu plan 1, Task 17)".
- [ ] **Step 2: Stop.** The switch itself is Quinn's, on Quinn's timeline.

---

## Self-review (run before handing this plan over)

1. **Spec coverage.** Knowlu spec §2 phase 0 → Tasks 1–16; §3 scheduler → 3, 7, 12; §4 storage/backup → 4, 5, 6, 10; §5.1 starving → constraint (no task edits the routine); §7 stage 1 → 7; §8 testing → every task's seam (fake clock: 3; bare repo: 5, 6; mirror byte-compare: 4; scripted fake model: not in this plan — no model here); §9 plan 1 list → Tasks 8–9 (writes), 9 (deck), 11 (events), 5–6/10 (history+backup), 3/12 (scheduler behind key), 15 (Runs view), 7 (rename). Console spec §5 commands → `state`, `note`, `mark_seen` (exist), `create_task`, `set_fields`, `delete_note`, `decide`, `close_info`, `open_issue`, `resolve_issue`, `sync` → Tasks 8–10; **`unbind` stays out (S3)**. §7.5 events → 2, 11, 15. §8 sync steps 1–6 → 5, 6, 10, 15. §9 freshness → unchanged from plan 1. Plan-1 bundle line 251/256 → Task 1, item by item.
2. **Placeholder scan.** No TBD/TODO. Two deliberate deferrals are named with their reason: `state.courses` (would move the references; the datalist uses row courses) and the three not-built views (plan 2 of 4).
3. **Type consistency.** `mutate` returns `Result<Value, String>` everywhere; `console_ctx()` is the only `WriteContext` constructor in `app/`; `HistoryStatus`/`BackupStatus` derive `Serialize` and are copied into `topline.sync`/`topline.backup`; `SchedulerMode::parse(Option<&str>)`; `runner_settings(path, name)`; `schedule::run_starts(vault, runner, since_date)`; `backup::tick(vault, target, profile_id, now, today)`; `history::sync(vault, ctx, journal, edits)`; `RunSummary.steps: Vec<(String, i32)>`. `quinn_ops::yaml::from_json` (Task 6) and `quinn_ops::yamlemit::{Node, safe_dump_block}` (Task 8) were verified against `src/yaml.rs:118` and `src/lib.rs:35` on 2026-09-04.
4. **Constraint check.** No task touches `engine/`, the routine, `config/runners.yaml` in the live vault, or `$mode`. The one live fetch (Task 12's hand-run slot) is called out. No reference regeneration after Task 1.
