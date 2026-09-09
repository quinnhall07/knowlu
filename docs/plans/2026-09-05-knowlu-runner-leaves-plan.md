# Knowlu — The Runner Leaves — Implementation Plan (Knowlu plan 2 of 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: PART A EXECUTED 2026-09-05 (Tasks 1–6 on `worktree-knowlu-plan-2`, subagent-driven: one fix round each for Tasks 4 and 5, a whole-branch final review, one fix wave, a scoped re-review; rulings R-P2-1…R-P2-12 and the deferred triage preserved at `docs/superpowers/reports/2026-09-05-knowlu-plan-2-part-a-sdd-ledger.md`). PART B NOT STARTED — gated as below; resume from that report's ▶ RESUME HERE block.** Written 2026-09-05. Execute on branch `worktree-knowlu-plan-2`. **Part A (Tasks 1–6) may start now.** **Part B (Tasks 7–13) starts only when the cutover plan's Task 9 step 2 is on `main`** — the harness gone from the runner script, `state/dual-run-log.md` deleted (~2026-09-18). The controller records that check in the SDD ledger before dispatching Task 7.

**Goal:** Knowlu becomes the local runner. The tray scheduler runs coursework and rank on the slots, the PowerShell script and the scheduled task are gone, the local Python venv is gone, the three remaining views (Decisions, Good to know, Issues) are real, the gauge has history and coursework has run records, and everything the running system depends on carries the Knowlu name (rename stage 2).

**Architecture:** Nothing new in shape. Part A finishes the page (three views over read-model data that already exists), hardens what plan 1 built (history, scheduler, quit) and writes the phase-2 checklist. Part B lands the two engine changes that were gated on the cutover (three counts on `rank`'s `tasks` step; coursework `start`/`step`/`end` records under a `coursework` runner), flips `scheduler: app` in one commit after the scheduled task is unregistered, retires the scripts to `scripts/cutover/`, and renames the engine crate, the app crate and the app-data root. The cloud routine, `engine/`, the journal, the run-record vocabulary and the eight frozen references are never touched.

**Tech Stack:** Rust 1.98 `stable-x86_64-pc-windows-gnu`; engine crate deps unchanged; Tauri 2 shell (`tauri`, `tray-icon`, `tauri-plugin-autostart`, `tauri-plugin-clipboard-manager`, `tauri-plugin-single-instance`, `tauri-plugin-window-state`, `windows 0.62`); the page is plain HTML/CSS/JS with no bundler; headless Playwright in the git-ignored `.wv` venv for page checks; PowerShell 5.1 for the two system-state steps (unregistering the task, the credential re-store which is Quinn's).

**Spec:** `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` (§2 phases 2, §3 the tray scheduler, §7 stage 2, §8 testing, §9 plan 2, §10 decisions 5, 6, 17, 19, 20) argued together with `docs/superpowers/specs/2026-09-02-console-on-rust-design.md` (§4.5 the two gated builders, §5 commands, §6 views, §10 sequencing, §16 hand-off 3) and `docs/superpowers/specs/2026-08-31-s2-surface-design.md` (§5 views, §6.2 parity rows for Decisions/Good to know/Issues/Runs, §7.7 gauge counts, §15 coursework run records). The carry-over list from plan 1 is `docs/superpowers/reports/2026-09-05-knowlu-plan-1-sdd-ledger.md` ("Carried to plan 2", and the final review's triage in its appendix).

## Global Constraints

Every task's requirements implicitly include these. Values are copied from the specs and from plan 1's ledger.

- **The gate.** Tasks 7–13 change what the local runner writes, remove the runner, or rename what it runs. None starts before the cutover plan's Task 9 step 2 is on `main`. Until then `scripts/diff-engines.ps1` (three fixtures) and `scripts/diff-engines-notes.ps1` still run clean after any `src/` change (Task 4 is the only Part A task that touches `src/`). After the gate those scripts are historical (Task 10) and the check is `cargo test` + `tests/oracle.rs` + `tests/surface_oracle.rs`.
- **Scratch copies for every test and every manual look** (`scripts/scratch-vault.ps1`). The live vault is written only by Knowlu itself, launched from the Start-menu shortcut after the go-live checklist — never by a test, a headless check, `--run-slot-once`, or a manual `rank`. **No task in this plan performs a live network fetch except Task 9's switch itself**, which is watched, not triggered (the slot fires on the clock). Google rate-limits the calendar feed.
- **The cloud routine is never touched** — prompt, config, environment. `QUINN_OPS_DEVICE` keeps its name (stage 3). `engine/` (Python) is untouched; its 687 tests are not run by this plan after the gate and no local venv is kept (Task 10).
- **Before any claim of green:** `cargo test` (dev profile; `--release` will not link) at **0 warnings**; `cd app; cargo test` at **zero new warnings** (the `.rsrc merge failure` linker line is pre-existing); `tests/oracle.rs` and `tests/surface_oracle.rs` green — `state/today.md` never moves by a byte in this plan.
- **Frozen references:** the eight Python-written references are never regenerated. The three `surface-today-*.json` regenerate only in a commit whose diff shows the change and whose message says why — this plan does so at most twice (Task 7's `empty.gauge` text; nothing else adds a payload key). `tests/fixtures/run-records-reference.json` is untouched: it pins `runs::{start_run, add_step, end_run}` at fixed inputs, not what `rank` chooses to count.
- **Every console write goes through `quinn_ops::write` / `approvals` / `issues` / `info`** with `console_ctx()` (`quinn`/`dashboard`); approvals execution with `executor_ctx()` (R-T9). `commands.rs` computes nothing; every mutating command returns `{ok, error, state}` with the freshly rebuilt state; a refused write returns `ok: false` and the current state. **The read model never writes** (`build_state_never_writes` stays green).
- **The scheduler stays inert unless the local runner's entry in `config/runners.yaml` says `scheduler: app`.** That key is written exactly once, by Task 9, after the scheduled task is unregistered. **Exactly one active local runner per vault**: the `device:` key stays and `device_ok` keeps gating the tick.
- **The app never builds anything and never runs the passes itself**; a slot runs the sibling engine exe as a child process (`slot_argv`), coursework then rank, and kills the whole process tree on timeout (Task 5).
- **All JSON the engine crate writes goes through `ledger::dumps_value`.** Run records through `runs::{start_run, add_step, end_run}` only.
- **Page rules:** no `http://` / `https://` under `app/static/`; no `import ` / `require(` in `console.js`; every empty-state phrase from the engine's `empty_text`/`texts`; every observed `[data-id]` element carries `data-kind` (R-T15b); `mark_seen` from exactly one place; interaction events carry ids only and only the eleven actions in `uievents::ACTIONS` — **this plan adds no action**.
- **Names, stage 2 (Task 11–12):** engine crate `knowlu-engine` (lib `knowlu_engine`, binary `knowlu-engine.exe`, clap name `knowlu-engine`); app crate `knowlu-desktop` (lib `knowlu_desktop`, binary `knowlu.exe` unchanged); app-data root `%LOCALAPPDATA%\knowlu` with a one-time move of `%LOCALAPPDATA%\quinn-ops`; user-agent `Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu`; credential targets `knowlu/zybooks` and `knowlu/vhl` (a `config/ingest.yaml` edit that lands **only after Quinn's re-store**, Task 12 step 6). **Never renamed:** journal actors and `via` values (`local-runner`, `dashboard`), run-record `runner` names (`local`, `cloud`, and the new `coursework`), ledger lines, the eight references, `QUINN_OPS_DEVICE`, the repository, the folder, `engine/`.
- **Credentials never in the repo, a log, a backup rule, a test fixture or a plan.** The zyBooks/VHL passwords are rotated and re-stored by Quinn from a non-Claude terminal; this plan only flips the `credential_target` strings after that.
- **No single-user assumption.** Vault from `--vault`; backup folder, scheduler mode, device name from config or the settings file; no path, machine name or user name in code or tests.
- **Desktop safety:** never synthetic keyboard or mouse input; screenshots by `PrintWindow` on a window handle from a **DPI-aware** process only (plan 1 Task 16's lesson: a DPI-unaware capture rendered a 2560-px viewport and clipped it), never a full-screen grab (R27).
- **Line endings are per file, never per directory** (plan 1, R-T16e): the index is LF everywhere; a fresh checkout is CRLF; a tool-rewritten file may be bare LF. Check the specific file with `od -c | head` or `tr -cd '\r' | wc -c` — never `Get-Content`, never `grep -c $'\r'` (it counts every line on this machine). `git diff --stat` must show no whole-file flip.
- **Commits:** `git add` specific paths, never `-A`; message via `-F <file>`; trailers `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the session's `Claude-Session:` line. Merges into `main` happen in the main checkout via the PowerShell tool after `git rev-list --left-right --count main...origin/main` reads `0 0`. **Obsidian Git is off after go-live**; until go-live it still sweeps the main checkout every ~5 minutes, so scaffolding is committed before long builds.
- **Build PATH (Bash):** `export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"`. Long cargo builds run as background calls with a bounded wait. Absolute paths containing `GitHub` trip the Bash worktree guard — use relative paths or the PowerShell tool.

---

## Fidelity ledger — rulings this plan must not lose

| # | Ruling | Source | Carried by |
|---|---|---|---|
| F1 | The app is the scheduler; engine steps as child processes; no build step in a slot | Knowlu spec §3, decision 5 | Tasks 5, 9 |
| F2 | `scheduler: script \| app`, flipped in one commit at phase 2, after the harness is out; the script and the task removed after | Knowlu spec §2, §3, decision 6 | Tasks 6, 9, 10 |
| F3 | One active local runner per vault — the `device:` key stays; a second install never double-runs | Knowlu spec §3; cutover C14 | Tasks 9, 10 (device key untouched) |
| F4 | No slot is ever skipped; inside grace on time, past grace late; wake/restart/crash catch up from `state/runs/` | Knowlu spec §3 | Task 5 (backoff never cancels a due slot; DST tests) |
| F5 | Failure visibility: topline shows the last run and the next slot; tray changes on FAIL or missed slot; Runs view is the rendering of the `end` record | Knowlu spec §3 | Tasks 5, 8, 13 |
| F6 | Gauge counts (`runway_days`, `deficit_hours_x10`, `must_count`) on `rank`'s `tasks` step, recorded from the first run after the gate; the gauge reads them, direction gaining/holding/losing | S2 §7.7; console spec §4.5; anatomy §3.5 | Task 7 |
| F7 | Coursework writes `start`/`step`/`end` run records with per-item counts so the one runner that reported nothing joins RUNS | S2 §15 (Quinn 2026-08-31); console spec §4.5 | Task 8 |
| F8 | Decisions view lists **every** pending approval regardless of urgency; neutral/decreasing amendments are shown there, counted (not shown) on the deck | S2 §5, §6.2 B4 | Task 1 |
| F9 | Good to know = open info items with the close action; Issues = open issue notes with categories and target, resolvable | S2 §5, §6.2 B10/B11; console spec §7.4 | Tasks 2, 3 |
| F10 | Push on close, synchronous, 10 s cap; backup on quit | console spec §8; Knowlu spec §4 triggers | Task 5 |
| F11 | Writes and sync serialised (`vault_io`); the backup is a vault reader and joins the lock | console spec §8/§9; plan 1 final review | Task 5 |
| F12 | A failed rebase is an error, never a pull; push only from `main`; no side branch is rebased | plan 1 fix wave A1/A3 + re-review residuals | Task 4 |
| F13 | Rename stage 2 renames what the running system depends on; history and references never | Knowlu spec §7, decisions 17, 18 | Tasks 11, 12 |
| F14 | `%LOCALAPPDATA%\quinn-ops` → `knowlu` is a move, not a fresh start — settings, seen stamp and logs survive. **Already done by Knowlu plan 4a Task 2 (2026-09-06)**, which also folded the flat files under `profiles\<id>\`; verify `%LOCALAPPDATA%\knowlu` exists with `profiles.json`, and that `tauri.conf.json` reads `com.knowlu.desktop` — change nothing | Knowlu spec §7 stage 2; plan 1 B3; plan 4a §2 | Task 12 (verify only) |
| F15 | Credential re-store under `knowlu/…` together with the owed rotation, by Quinn, from a non-Claude terminal | Knowlu spec §7, §11 | Task 12 step 6 (gated) |
| F16 | The cutover scripts are kept as historical checks, not deleted; the runner script itself goes | cutover plan Task 9 step 2; Knowlu spec §9 | Task 10 |
| F17 | `runner-log.md` is not written by the app; the engine's own `append_run_log` line stays until the log is retired (not this plan) | Knowlu spec §3 | Tasks 8, 9 (no app write) |
| F18 | Every empty-state phrase comes from the engine — the gauge's "no history yet" included | console spec §6; plan 1 ledger | Task 7 |
| F19 | Interaction events: ids only, the eleven actions; the three new views emit `object_seen` through `watchSeen` and nothing new | console spec §7.5; R-T15b/c | Tasks 1–3 |
| F20 | Refusals: `ok:false` + current state, reason inline; the Decisions view's rows behave exactly like the deck's card (busy guard, snap back) | console spec §7.2; R-T14 fixes | Task 1 |
| F21 | `engine newer than console` stays a reachable state until one bundle ships (plan 4); the engine and console still build separately | console spec §15 Q1; Knowlu spec §6 | Task 11 (both crates build from one commit) |
| F22 | The go-live checklist ran before this plan's Part B; the phase-2 checklist is written before the gate so Quinn reads it in advance | plan 1 Task 17 pattern | Task 6 |

---

## File structure

**Engine crate (`src/`)** — Part A touches only `history.rs` and `schedule.rs` tests; Part B touches `cli.rs` (three counts), `coursework.rs` (run records), `surface.rs` (`empty.gauge`), `zybooks.rs`/`vhl.rs` (user-agent), `main.rs` (clap name), `Cargo.toml` (names).

**App crate (`app/`)** — `src/scheduler.rs` (tree kill, backoff, log retention, poison tolerance), `src/state.rs` (`quit_flush`, `app_data_root` migration), `src/tray.rs` (quit), `src/commands.rs` (`backup_now` under `vault_io`), `Cargo.toml` (names), `tests/*`.

**Page (`app/static/`)** — `index.html` (three `main-*` sections and the gauge strip), `console.js` (`renderDecisionsView`, `renderGoodToKnowView`, `renderIssuesView`, `renderGauge`, `decideCard(host)`), `console.css` (the `.dec`, `.gtk-row`, `.iss` rows, the gauge strip), `tests/static_assets.rs`.

**Scripts** — `scripts/local-run.ps1` deleted; `scripts/cutover/` gains `dual-run.ps1`, `compare-vaults.ps1`, `rehearse-rollback.ps1`, `diff-engines.ps1`, `diff-engines-notes.ps1`, `lint-yaml-11.py`, `diagnose-local-runner.ps1`, `register-local-runner.ps1` (new, the registration block from cutover Task 1) and a `README.md`; `scripts/scratch-vault.ps1` path updated.

**Config** — `config/runners.yaml` (`scheduler: app` under `local`, Task 9); `config/ingest.yaml` (`credential_target` lines, Task 12 step 6, gated on Quinn).

**Docs** — `docs/runners/knowlu-phase-2.md` (new, Task 6); `docs/surface/anatomy.md`, `app/README.md`, `docs/HANDOFF.md`, `CLAUDE.md`, this plan's status line (Task 13).

---

## Part A — buildable now, on a worktree, against scratch copies

### Task 1: The Decisions view

**Files:**
- Modify: `app/static/index.html` (a `main-decisions` section after `main-list`), `app/static/console.js` (`renderDecisionsView`, `decideCard` takes a host, `VIEW_RENDERERS.decisions`, the digest pluralisation), `app/static/console.css` (`.row.dec`, `.dnote`, `.changes`)
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `state.decisions` (`cards: Vec<Card>` — every pending approval except `events-digest`, amendments of every urgency included — plus `pending`, `oldest_days`, `budget_used/total/remaining`, `deferred`, `awaiting_calendar`, `events_in_digest`, `hidden_amendments`, `empty_text`); `Card { id, slug, title, kind, source_uid, first_proposed_at, age_days, urgency, why, expires, snooze_until, target, changes }`; the `decide` command; `state.ahead.buckets[1].date` for the snooze default.
- Produces: `renderDecisionsView(state)`; `decideCard(id, verdict, snoozeUntil, host)` where `host` defaults to the deck; the `#dec-list` click binding.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_decisions_view_lists_every_pending_approval_and_acts_like_the_deck() {
    let js = read("console.js");
    assert!(js.contains("function renderDecisionsView("), "the view has its own renderer");
    assert!(js.contains("decisions: renderDecisionsView"), "and it is in VIEW_RENDERERS");
    assert!(js.contains("function decideCard(id, verdict, snoozeUntil, host)"), "decideCard is host-aware");
    assert!(js.contains("hidden_amendments"), "the header says how many amendments the deck hides");
    // F8: the view renders d.cards whole — no urgency filter anywhere in the renderer.
    let view = js.split("function renderDecisionsView(").nth(1).unwrap().split("\n  function ").next().unwrap();
    assert!(!view.contains("urgency !==") && !view.contains("urgency ==="), "no urgency filter in the view");
    assert!(view.contains("data-kind=\"approval\""), "rows are observed as approvals");
    assert!(view.contains("data-verdict=\"approved\"") && view.contains("data-verdict=\"rejected\"") && view.contains("data-verdict=\"snoozed\""));
    // The digest count reads honestly at one.
    assert!(!js.contains("\" events in today's digest\""), "no fixed plural");
    assert!(js.contains("=== 1 ? \" event\" : \" events\""), "singular at one");
    let html = read("index.html");
    assert!(html.contains("id=\"main-decisions\"") && html.contains("id=\"dec-list\""));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run (from `app/`): `cargo test --test static_assets the_decisions_view -- --nocapture`
Expected: FAIL at the first assertion (`renderDecisionsView` does not exist).

- [ ] **Step 3: The section in `index.html`**

Directly after the `main-list` div:

```html
    <section id="main-decisions" hidden>
      <div class="sec"><div class="sec-hd"><h2>Decisions</h2><span class="n" id="dec-n"></span></div>
      <div class="hint" id="dec-hint"></div>
      <div id="dec-list"></div></div>
    </section>
```

- [ ] **Step 4: `decideCard` takes a host**

Replace the first two lines of `decideCard` and every `.nb input` lookup inside it:

```js
  function decideCard(id, verdict, snoozeUntil, host) {
    host = host || EL("deck");
    var card = host.querySelector('[data-id="' + id + '"]');
    if (card && card.dataset.busy) { return; }
    var noteEl = card ? card.querySelector(".nb input, .dnote") : null;
    var note = noteEl ? (noteEl.value || "") : "";
```

and, in the same function, replace each `card.querySelector(".nb input")` with `card.querySelector(".nb input, .dnote")`. `bindDeck` is unchanged (it calls `decideCard(id, verdict, snooze)` and gets the deck by default).

- [ ] **Step 5: The renderer**

Add after `renderRunsView`:

```js
  // Plan 2 Task 1 (S2 §6.2 B4, F8): EVERY pending approval as a list — the deck shows one card
  // and hides nothing but the events digest; this view is the whole queue, amendments of every
  // urgency included, so the "N hidden" count on the deck's header has somewhere to point.
  function renderDecisionsView(state) {
    var d = state.decisions, host = EL("dec-list");
    var parts = [d.pending + " pending"];
    if (d.pending) { parts.push("oldest " + d.oldest_days + "d"); }
    parts.push(d.budget_used + " of " + d.budget_total + " today");
    if (d.deferred) { parts.push(d.deferred + " deferred"); }
    if (d.awaiting_calendar) { parts.push(d.awaiting_calendar + " awaiting calendar"); }
    if (d.events_in_digest) { parts.push(d.events_in_digest + (d.events_in_digest === 1 ? " event" : " events") + " in digest"); }
    if (d.hidden_amendments) { parts.push(d.hidden_amendments + " amendment" + (d.hidden_amendments === 1 ? "" : "s") + " the deck does not surface"); }
    EL("dec-n").textContent = parts.join(" · ");
    EL("dec-hint").textContent = d.cards.length ? "approve, reject, or snooze — one click commits" : "";
    // Typed notes survive a repaint, per id (the deck's own rule, R-T14 2b).
    var notes = {};
    host.querySelectorAll(".row.dec[data-id]").forEach(function (r) { var n = r.querySelector(".dnote"); if (n && n.value) { notes[r.getAttribute("data-id")] = n.value; } });
    var tomorrow = state.ahead.buckets[1].date;
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = d.cards.map(function (c) {
        var changes = "";
        if (c.changes && typeof c.changes === "object") {
          changes = '<div class="changes">' + Object.keys(c.changes).map(function (k) {
            var v = c.changes[k]; return '<span class="k lp">' + h(k) + "</span> → " + h(typeof v === "object" ? JSON.stringify(v) : String(v));
          }).join(" · ") + "</div>";
        }
        var meta = [c.kind, c.age_days + "d old"];
        if (c.urgency) { meta.push("urgency " + c.urgency); }
        if (c.target) { meta.push("→ " + c.target); }
        if (c.expires) { meta.push("expires " + c.expires); }
        if (c.snooze_until) { meta.push("snoozed to " + c.snooze_until); }
        return '<div class="row dec" data-id="' + h(c.id) + '" data-kind="approval">' +
          '<button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button>' +
          '<div class="ttl"><span class="a">' + h(c.title) + '</span><span class="meta">' + h(meta.join(" · ")) + "</span>" +
          '<div class="why">' + h(c.why) + "</div>" + changes + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Note (optional)" value="' + h(notes[c.id] || "") + '">' +
          '<button class="b pri y" data-verdict="approved">Approve</button><button class="b n" data-verdict="rejected">Reject</button>' +
          '<button class="b" data-verdict="snoozed" data-snooze="' + h(tomorrow) + '">Snooze</button></div></div>';
      }).join("") + (d.empty_text && !d.cards.length ? '<div class="empty">' + h(d.empty_text) + "</div>" : "");
    });
  }
```

- [ ] **Step 6: Wire the view and the clicks**

In the route/apply function: `VIEW_RENDERERS = { runs: renderRunsView, decisions: renderDecisionsView }`; add `EL("main-decisions").hidden = current.view !== "decisions";` beside the other `hidden` toggles. In the deck's renderer, the digest phrase becomes `(d.events_in_digest + (d.events_in_digest === 1 ? " event" : " events") + " in today's digest")`. Add one binding next to `bindDeck()`:

```js
  function bindDecisionsView() {
    EL("dec-list").addEventListener("click", function (e) {
      var b = e.target.closest("button[data-verdict]"); if (!b) { return; }
      var row = b.closest(".row.dec"); if (!row) { return; }
      decideCard(row.getAttribute("data-id"), b.getAttribute("data-verdict"), b.getAttribute("data-snooze") || null, EL("dec-list"));
    });
  }
```

called once where `bindDeck()` is called. `watchSeen()` already observes every `[data-id]` after a paint; `applyEnvelope` repaints the current view, so a decided row leaves the list with the fresh state.

- [ ] **Step 7: CSS**

```css
.row.dec { align-items: flex-start; }
.row.dec .meta { color: var(--mute); font: 500 11px/1.4 var(--mono); margin-left: 8px; }
.row.dec .why { color: var(--ink2); font-size: 13px; margin-top: 4px; max-width: 72ch; }
.row.dec .changes { margin-top: 4px; font: 500 11px/1.5 var(--mono); color: var(--mute); }
.row.dec .acts { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.row.dec .dnote { width: 16ch; }
```

(Token names are the ones `console.css` already defines at the top; use the existing `--mute`/`--ink2`/`--mono` spellings verbatim from that block.)

- [ ] **Step 8: Run the tests and the headless check**

Run: `cargo test --test static_assets` (all green, 15 tests). Then a headless check in `.wv` against `tests/fixtures/surface-today-full.json` served the way `scripts/console-shots.py` serves it (temp dir, loopback): navigate to `#decisions`, assert `#main-decisions` visible, the row count equals `state.decisions.cards.length`, a click on one row's Reject calls a fake `__TAURI__` `decide` with that id, and a typed note survives a forced `applyEnvelope` repaint. No desktop input.

- [ ] **Step 9: Commit**

`git add app/static/index.html app/static/console.js app/static/console.css app/tests/static_assets.rs` — `page: the Decisions view — every pending approval, acting like the deck (Knowlu plan 2, Task 1)`.

---

### Task 2: The Good to know view

**Files:**
- Modify: `app/static/index.html` (`main-gtk`), `app/static/console.js` (`renderGoodToKnowView`, `VIEW_RENDERERS["good-to-know"]`), `app/static/console.css`
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `state.good_to_know: Vec<InfoItem { id, title, kind, close_key, expires, opened_at, path }>`, `state.empty.info` (the engine's empty text; the rail already uses it), `closeInfoItem(id)` (exists; calls `close_info`).
- Produces: `renderGoodToKnowView(state)`.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn the_good_to_know_and_issues_views_are_real_and_every_observed_row_names_its_kind() {
    let js = read("console.js");
    for f in ["renderGoodToKnowView", "renderIssuesView"] { assert!(js.contains(&format!("function {f}(")), "{f}"); }
    assert!(js.contains("\"good-to-know\": renderGoodToKnowView") && js.contains("issues: renderIssuesView"));
    assert!(js.contains("data-kind=\"info\"") && js.contains("data-kind=\"issue\""), "R-T15b: the two kinds that had no template");
    // Every template that stamps data-id stamps data-kind in the same string (R-T15b, re-verified).
    assert_eq!(js.matches("data-id=\"").count(), js.matches("data-kind=\"").count(), "data-id without data-kind somewhere");
    assert!(!js.contains("is not built yet"), "no view is a placeholder any more");
    let html = read("index.html");
    assert!(html.contains("id=\"main-gtk\"") && html.contains("id=\"main-issues\""));
}
```

Run: `cargo test --test static_assets the_good_to_know` — FAIL (`renderGoodToKnowView` missing). This one test covers Tasks 2 and 3; it goes green at the end of Task 3 and Task 2's commit carries it red-but-committed with the `renderIssuesView` half still failing — **no**: keep Task 2 green on its own by committing the test with the Issues assertions present and the Task 2 commit message saying "issues half lands in Task 3", and run only `the_good_to_know` filtered assertions? A test is one unit. Resolution: Task 2 adds the test **without** the three Issues lines (`renderIssuesView`, `issues: renderIssuesView`, `data-kind="issue"`, `main-issues`); Task 3 adds them. The `data-id`/`data-kind` count equality holds at both points.

- [ ] **Step 2: Section, renderer, wiring**

`index.html`, after `main-decisions`:

```html
    <section id="main-gtk" hidden>
      <div class="sec"><div class="sec-hd"><h2>Good to know</h2><span class="n" id="gtkv-n"></span></div><div id="gtk-list"></div></div>
    </section>
```

`console.js`:

```js
  // Plan 2 Task 2 (S2 §6.2 B11, F9): open info items as a list with the close action the rail
  // already has. Rows are observed as "info" (R-T15b) — the kind that had no template.
  function renderGoodToKnowView(state) {
    var g = state.good_to_know, host = EL("gtk-list");
    EL("gtkv-n").textContent = g.length ? g.length + " open" : "";
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = g.map(function (i) {
        var meta = [i.kind];
        if (i.opened_at) { meta.push("since " + i.opened_at.slice(0, 10)); }
        if (i.expires) { meta.push("expires " + i.expires); }
        if (i.close_key) { meta.push("closes on " + i.close_key); }
        return '<div class="row gtk-row" data-id="' + h(i.id) + '" data-kind="info">' +
          '<button class="flag" data-flag="' + h(i.id) + '" title="agent-authored — flag it">&#9873;</button>' +
          '<div class="ttl"><span class="a">' + h(i.title) + '</span><span class="meta">' + h(meta.join(" · ")) + "</span></div>" +
          '<div class="acts"><button class="b" data-close-info="' + h(i.id) + '">Close</button></div></div>';
      }).join("") + (g.length ? "" : '<div class="empty">' + h(state.empty.info) + "</div>");
    });
  }
```

Wiring: `VIEW_RENDERERS["good-to-know"] = renderGoodToKnowView`; `EL("main-gtk").hidden = current.view !== "good-to-know";`; one delegated click on `#gtk-list` for `button[data-close-info]` → `closeInfoItem(id)` with the same busy guard the rail's close uses (disable the button until the envelope returns; `applyEnvelope` repaints). CSS: `.row.gtk-row .acts { justify-content: flex-end; }` and reuse `.row.dec .meta` by making the selector `.row .meta`.

- [ ] **Step 3: Tests, headless check, commit**

`cargo test --test static_assets` green (Task 2's half of the new test). Headless: navigate to `#good-to-know` on a fixture with at least one info item — `tests/fixtures/surface-today-full.json` has none, so the check plants one row by feeding a modified copy of the fixture JSON (`good_to_know: [{id:"info_test0000001", title:"Planted", kind:"notice", close_key:null, expires:null, opened_at:"2026-09-01T00:00:00Z", path:"info/planted.md"}]`) — assert one row, `data-kind="info"`, and that Close invokes `close_info` with that id.

Commit: `page: the Good to know view — open info items with close (Knowlu plan 2, Task 2)`.

---

### Task 3: The Issues view

**Files:**
- Modify: `app/static/index.html` (`main-issues`), `app/static/console.js` (`renderIssuesView`, `resolveIssue`), `app/static/console.css`
- Test: `app/tests/static_assets.rs` (the Issues lines of Task 2's test)

**Interfaces:**
- Consumes: `state.issues_panel: { open_count, rows: Vec<IssueRow { id, title, categories, target, opened_at, path }>, empty_text }`; the `resolve_issue(view, id, resolution)` command; `openDrawer(id)` (exists).
- Produces: `renderIssuesView(state)`, `resolveIssue(id, text)`.

- [ ] **Step 1: The test lines** — add to Task 2's test the four Issues assertions listed there. Run: FAIL on `renderIssuesView`.

- [ ] **Step 2: Section, renderer, resolve**

```html
    <section id="main-issues" hidden>
      <div class="sec"><div class="sec-hd"><h2>Issues raised</h2><span class="n" id="issv-n"></span></div><div id="iss-list"></div></div>
    </section>
```

```js
  // Plan 2 Task 3 (S2 §6.2 B10, F9): open issue notes — the ⚑ flags — with their categories, the
  // object they were raised on (opens the drawer), and a resolution. S3's duplicate side-by-side
  // resolution is not built (deferred with S3; console spec §13).
  function renderIssuesView(state) {
    var p = state.issues_panel, host = EL("iss-list");
    EL("issv-n").textContent = p.open_count ? p.open_count + " open" : "";
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = p.rows.map(function (r) {
        var chips = (r.categories || []).map(function (c) { return '<span class="chip on">' + h(c) + "</span>"; }).join("");
        var when = r.opened_at ? "raised " + r.opened_at.slice(0, 10) : "";
        return '<div class="row iss" data-id="' + h(r.id) + '" data-kind="issue">' +
          '<div class="ttl"><span class="a">' + h(r.title) + '</span><span class="meta">' + h(when) + "</span><div class=\"chips\">" + chips + "</div>" +
          (r.target ? '<button class="lnk" data-open-target="' + h(r.target) + '">open the object</button>' : "") + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Resolution"><button class="b" data-resolve="' + h(r.id) + '">Resolve</button></div></div>';
      }).join("") + (p.rows.length ? "" : '<div class="empty">' + h(p.empty_text || state.empty.issues) + "</div>");
    });
  }
  function resolveIssue(id, text, btn) {
    if (btn) { btn.disabled = true; }
    return invoke("resolve_issue", { view: current.view, id: id, resolution: text }).then(function (env) {
      if (!applyEnvelope(env, function (m) { if (btn) { btn.disabled = false; showRefusal(btn.closest(".row"), m); } })) { return; }
    }).catch(function () { if (btn) { btn.disabled = false; } });
  }
```

Wiring: `VIEW_RENDERERS.issues = renderIssuesView`; `EL("main-issues").hidden = current.view !== "issues";`; delegated click on `#iss-list`: `button[data-resolve]` → `resolveIssue(id, row.querySelector(".dnote").value || "", btn)`; `button[data-open-target]` → `openDrawer(target)`. A resolution may be empty — the engine decides (`issues::address_issue` accepts an empty resolution today; if it refuses, the refusal shows inline like every other). CSS: `.row.iss .chips { margin-top: 4px; display: flex; gap: 4px; flex-wrap: wrap; } .lnk { background: none; border: 0; color: var(--acc); cursor: pointer; padding: 0; font: inherit; }` (`--acc` spelled as `console.css` spells its accent token).

- [ ] **Step 3: Tests, headless, commit**

`cargo test --test static_assets` green (15 tests). Headless on a fixture copy with one planted issue row (`issues_panel.rows: [{id:"issue_test000001", title:"Planted", categories:["wrong course"], target:"task_0000000001", opened_at:"2026-09-01T00:00:00Z", path:"issues/planted.md"}]`): one row, `data-kind="issue"`, Resolve invokes `resolve_issue` with the id and the typed text, "open the object" invokes `note` with the target.

Commit: `page: the Issues view — flags with categories, target and resolution (Knowlu plan 2, Task 3)`.

---

### Task 4: History hardening — no rebase on a side branch, a stuck rebase is an error, git calls time out

**Files:**
- Modify: `src/history.rs`
- Test: `src/history.rs` (module tests; `repo_pair` builds a bare remote and two clones)

**Interfaces:**
- Consumes: `git(vault, args)`, `status(vault)`, `conflicts_pending(vault)`, `rebase_in_progress(vault)`, `resolve_conflicts(...)`, `SyncOutcome`.
- Produces: `pub const GIT_TIMEOUT: Duration = Duration::from_secs(60)`; `fn git_with(program: &str, vault, args, timeout) -> Result<String, String>` (the seam; `git()` calls it with `"git"` and `GIT_TIMEOUT`); `fn unmerged_paths_exist(vault) -> bool`.

- [ ] **Step 1: Failing tests**

```rust
    /// Plan 2 Task 4 (F12): a vault parked on a side branch is not rebased onto origin/main and
    /// not pushed — the local commit still happens (edits are never lost), the sync says why.
    #[test]
    fn a_side_branch_is_committed_locally_and_neither_rebased_nor_pushed() {
        let (remote, a, b) = repo_pair("side");
        push_a_commit_from(&b, "tasks/remote-edit.md", "remote moved on");
        git(&a, &["checkout", "-q", "-b", "side"]).unwrap();
        write_note(&a, "tasks/local.md", "local edit");
        let before = git(&a, &["rev-parse", "HEAD"]).unwrap();
        let out = sync(&a, &ctx(), &mut journal(&a), 1);
        assert!(out.committed && !out.pulled && !out.pushed);
        assert_eq!(out.status.last_error.as_deref(), Some("not on main (side) — sync refused"));
        assert_ne!(git(&a, &["rev-parse", "HEAD"]).unwrap(), before, "the local commit happened");
        assert_eq!(git(&a, &["rev-parse", "HEAD~1"]).unwrap(), before, "…and nothing was rebased under it");
        assert_eq!(remote_tip(&remote), tip_of(&b), "the remote never moved");
    }

    /// A rebase that stops with the working tree in `rebase-merge` but NO unmerged paths (an
    /// untracked-file collision, an empty commit needing --skip) is not a conflict to resolve:
    /// it is aborted and reported. `resolve_conflicts` runs only when `git status` lists a `U`.
    #[test]
    fn a_stopped_rebase_without_unmerged_paths_is_aborted_and_reported() {
        let (_remote, a, _b) = repo_pair("stuck");
        let dir = git(&a, &["rev-parse", "--git-path", "rebase-merge"]).unwrap();
        std::fs::create_dir_all(a.join(&dir)).unwrap();
        assert!(rebase_in_progress(&a) && !unmerged_paths_exist(&a));
        assert!(!conflicts_pending(&a), "conflicts_pending now means unmerged paths, nothing else");
        std::fs::remove_dir_all(a.join(&dir)).unwrap();
    }

    /// A git invocation that hangs (a credential prompt that GIT_TERMINAL_PROMPT could not stop,
    /// a remote that never answers) is killed at the cap and reported, never waited on forever.
    #[test]
    fn a_hanging_git_call_is_killed_at_the_cap() {
        let (_remote, a, _b) = repo_pair("hang");
        let shim = a.join("slow-git.cmd");
        std::fs::write(&shim, "@echo off\r\nping -n 6 127.0.0.1 >nul\r\n").unwrap();
        let t0 = std::time::Instant::now();
        let out = git_with(shim.to_str().unwrap(), &a, &["status"], std::time::Duration::from_secs(1));
        assert!(t0.elapsed() < std::time::Duration::from_secs(4), "returned at the cap, not at ping's end");
        assert!(out.unwrap_err().contains("timed out after 1 s"), "{out:?}");
    }
```

(`repo_pair`, `push_a_commit_from`, `write_note`, `ctx`, `journal`, `remote_tip`, `tip_of` are the module's existing helpers — read them first and use their real names; if one does not exist, add it beside the others rather than inlining git plumbing in the test.)

Run: `cargo test history::` — the three fail (`unmerged_paths_exist`/`git_with` undefined; the side-branch test finds `pulled == true`).

- [ ] **Step 2: `git_with` and the timeout**

```rust
pub const GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub fn git(vault: &Path, args: &[&str]) -> Result<String, String> { git_with("git", vault, args, GIT_TIMEOUT) }

/// The one process spawn in this module. `program` is a seam for the timeout test; production is
/// always `"git"`. Stdout and stderr are drained on their own threads (a chatty `fetch` must not
/// fill a pipe), the child is polled every 100 ms, and past `timeout` it is killed and reported.
pub fn git_with(program: &str, vault: &Path, args: &[&str], timeout: std::time::Duration) -> Result<String, String> {
    use std::io::Read;
    let mut child = Command::new(program).current_dir(vault).env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "commit.gpgsign=false", "-c", "core.editor=true"]).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("git: {e}"))?;
    let mut out_pipe = child.stdout.take().unwrap();
    let mut err_pipe = child.stderr.take().unwrap();
    let out_t = std::thread::spawn(move || { let mut s = Vec::new(); let _ = out_pipe.read_to_end(&mut s); s });
    let err_t = std::thread::spawn(move || { let mut s = Vec::new(); let _ = err_pipe.read_to_end(&mut s); s });
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| format!("git: {e}"))? { break Some(st); }
        if started.elapsed() >= timeout { let _ = child.kill(); let _ = child.wait(); break None; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let stdout = String::from_utf8_lossy(&out_t.join().unwrap_or_default()).trim().to_string();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).trim().to_string();
    match status {
        None => Err(format!("git {}: timed out after {} s", args.first().copied().unwrap_or(""), timeout.as_secs())),
        Some(st) if st.success() => Ok(stdout),
        Some(_) => Err(stderr),
    }
}
```

The `.cmd` shim receives `-c commit.gpgsign=false …` as arguments and ignores them — that is fine for the test. On a non-Windows host the shim test is `#[cfg(windows)]`.

- [ ] **Step 3: The side-branch check moves before the fetch; the conflict predicate narrows**

In `sync`, immediately after the `commit_by_name` match and before `if !out.status.has_remote`:

```rust
    let branch = git(vault, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| "unknown".into());
    if branch != "main" {
        out.status = status(vault);
        out.status.last_error = Some(format!("not on main ({branch}) — sync refused"));
        return out;
    }
```

and delete the later pre-push branch check (it is now unreachable). Rename the predicate:

```rust
/// `git status --porcelain` lists an unmerged path (`UU`, `AA`, `DU`, …). This — and only this —
/// is what `resolve_conflicts` can act on. A rebase directory with no unmerged path is a rebase
/// that stopped for another reason and is aborted, not resolved (plan 2 Task 4, F12).
fn unmerged_paths_exist(vault: &Path) -> bool { !status(vault).conflicted.is_empty() }
fn conflicts_pending(vault: &Path) -> bool { unmerged_paths_exist(vault) }
```

so the `Err(e) if !conflicts_pending(vault)` arm's `rebase --abort` is reachable. Update the A3 test from plan 1 (`push refused` on a branch) to the new message and the new point of refusal.

- [ ] **Step 4: Run, dual-run scripts, commit**

`cargo test` 0 warnings; `scripts\diff-engines.ps1` on the three fixtures and `scripts\diff-engines-notes.ps1` exit 0 (Part A is before the gate; `history.rs` is a `src/` change even though `rank`/`write` never call it). Commit `src/history.rs`: `engine: history refuses a side branch before touching it, aborts a stuck rebase, and caps every git call at 60 s (Knowlu plan 2, Task 4)`.

---

### Task 5: Scheduler and app hardening — tree kill, backoff, log retention, poison tolerance, backup under the lock, push and back up on quit, DST

**Files:**
- Modify: `app/src/scheduler.rs`, `app/src/state.rs`, `app/src/tray.rs`, `app/src/commands.rs`, `src/schedule.rs` (tests only)
- Test: `app/tests/scheduler.rs`, `app/tests/commands.rs`, `src/schedule.rs`

**Interfaces:**
- Consumes: `run_child(exe, args, log, timeout)`, `RunSummary`, `Scheduler { running, last, paused, mode_device }`, `ConsoleState { vault_io, settings, history, backup, pending_edits, auto_sync }`, `state::run_sync`, `state::run_backup`, `schedule::{due_slot, missed_slots}`.
- Produces: `fn kill_tree(child: &mut Child)`; `pub fn should_retry(attempts: u32, since: Duration) -> bool`; `Scheduler.attempts: Mutex<HashMap<String, (u32, Instant)>>`; `pub fn prune_logs(dir: &Path, keep: usize) -> usize`; `fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T>`; `pub struct QuitFlush { pub synced: bool, pub backed_up: bool, pub timed_out: bool }`; `pub fn quit_flush(cs: &ConsoleState, cap: Duration) -> QuitFlush`.

- [ ] **Step 1: Failing tests (app)**

```rust
/// Plan 2 Task 5: the cap kills the TREE. `cmd /c ping` is exactly the shape plan 1's fix wave
/// could not meet (the grandchild kept the pipe ends open past the kill) — now it returns at the cap.
#[test]
fn a_child_tree_that_outlives_its_cap_is_killed_whole_and_returns_at_the_cap() {
    let log = std::env::temp_dir().join(format!("knowlu-tree-{}.log", std::process::id()));
    let t0 = std::time::Instant::now();
    let code = run_child(Path::new("cmd"), &["/c".into(), "ping -n 6 127.0.0.1 >nul".into()], &log, std::time::Duration::from_secs(1));
    assert_eq!(code, -2);
    assert!(t0.elapsed() < std::time::Duration::from_secs(3), "returned at the cap: {:?}", t0.elapsed());
}

#[test]
fn retry_backoff_is_5_15_30_then_hourly_and_never_forever() {
    use std::time::Duration as D;
    assert!(should_retry(0, D::ZERO), "a slot never attempted is due now");
    assert!(!should_retry(1, D::from_secs(4 * 60)) && should_retry(1, D::from_secs(5 * 60)));
    assert!(!should_retry(2, D::from_secs(14 * 60)) && should_retry(2, D::from_secs(15 * 60)));
    assert!(!should_retry(3, D::from_secs(29 * 60)) && should_retry(3, D::from_secs(30 * 60)));
    assert!(!should_retry(9, D::from_secs(59 * 60)) && should_retry(9, D::from_secs(60 * 60)));
}

#[test]
fn log_retention_keeps_the_newest_thirty() {
    let dir = std::env::temp_dir().join(format!("knowlu-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
    for i in 0..35 { std::fs::write(dir.join(format!("slot-2026-08-{:02}-1200.log", i % 31 + 1 + (i / 31) * 0)), b"x").unwrap(); }
    // 35 distinct names: fall back to a counter suffix so names never collide
    let names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
    assert!(names.len() >= 30, "planted at least thirty files: {}", names.len());
    let removed = prune_logs(&dir, 30);
    let left = std::fs::read_dir(&dir).unwrap().count();
    assert_eq!(left, 30); assert_eq!(removed, names.len() - 30);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Quit flushes: no remote → nothing to push (synced=false, not an error); a backup dir set →
/// the mirror is written; all inside the cap.
#[test]
fn quit_flush_backs_up_and_reports_within_the_cap() {
    let v = scratch("quitflush");
    let cs = open(&v, "quitflush");
    let bdir = std::env::temp_dir().join(format!("knowlu-quit-backup-{}", std::process::id()));
    { let mut s = cs.settings.lock().unwrap(); s.backup_dir = Some(bdir.clone()); }
    let t0 = std::time::Instant::now();
    let q = quit_flush(&cs, std::time::Duration::from_secs(10));
    assert!(t0.elapsed() < std::time::Duration::from_secs(10));
    assert!(!q.timed_out && q.backed_up && !q.synced, "{q:?}");
    let profile = cs.settings.lock().unwrap().profile_id.clone();
    assert!(bdir.join(&profile).join("vault").join("tasks").is_dir(), "the mirror exists");
    let _ = std::fs::remove_dir_all(&bdir);
}
```

(Write the log-retention planting loop so the 35 names are distinct — `slot-{i:03}.log` is enough; the snippet above shows the intent, the implementer writes the simple form.) In `app/tests/commands.rs`, extend the `vault_io` test from plan 1: hold `vault_io`, call `backup_now_inner` on a thread, assert it blocks until release.

Run: `cd app; cargo test --test scheduler` — the four fail (`should_retry`, `prune_logs`, `quit_flush` undefined; the tree test returns at ~5 s).

- [ ] **Step 2: Failing tests (engine, `src/schedule.rs`)**

```rust
    /// DST: America/Chicago falls back on 2026-11-01 (01:00 happens twice) and springs forward on
    /// 2026-03-08 (02:00 does not happen). Slots at 12:00/18:00 are unaffected; a slot placed
    /// INSIDE the shifted hour must fire exactly once per day, whichever instant jiff picks.
    #[test]
    fn dst_days_fire_each_slot_exactly_once() {
        let cfg = RunnerConfig { name: "local".into(), times: vec!["01:30".into(), "12:00".into(), "18:00".into()], tz: "America/Chicago".into(), grace_minutes: 20, ..local_cfg() };
        for day in ["2026-11-01", "2026-03-08"] {
            let mut starts: Vec<Timestamp> = Vec::new();
            let mut fired = 0;
            let mut now: Zoned = format!("{day}T00:00[America/Chicago]").parse().unwrap();
            let end: Zoned = format!("{day}T23:59[America/Chicago]").parse().unwrap();
            while now < end {
                if let Some(d) = due_slot(&cfg, &now, &starts) { fired += 1; starts.push(d.due.timestamp()); }
                now = now.checked_add(Span::new().minutes(1)).unwrap();
            }
            assert_eq!(fired, 3, "{day}: every configured slot exactly once");
            let from: Zoned = format!("{day}T00:00[America/Chicago]").parse().unwrap();
            assert_eq!(missed_slots(&cfg, &from, &end, &[]), 3, "{day}: three missed with no starts");
        }
    }
```

(`local_cfg()` is whatever helper the module's tests already use to build a `RunnerConfig`; if `RunnerConfig` has more fields, fill them from that helper.) Run: `cargo test schedule::dst` — it may already pass (jiff resolves the gap and the fold deterministically). **If it passes red-free, keep it**: it is the pin the reverse week wanted before `scheduler: app` flips (plan 1 final review, T3 triage).

- [ ] **Step 3: Tree kill and backoff (`app/src/scheduler.rs`)**

```rust
/// Kill the child AND everything it spawned. `Child::kill` is a single TerminateProcess; a `cmd`
/// wrapper or any future engine step that forks would keep the pipe ends open past the kill and
/// `run_child`'s drain threads would wait for the grandchild (plan 1 final review, B5). Windows
/// has no process groups to signal, so this is `taskkill /T`; the fallback is the plain kill.
fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill").args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
```

used in `run_child` where the cap fires (replace the `kill()`/`wait()` pair). Backoff:

```rust
/// Retry cadence for a slot that failed before the engine wrote its `start` record — which is the
/// only case the tick would otherwise retry every 60 s until midnight (plan 1 Task 12 minor). The
/// slot stays due (F4: never skipped); this decides only how often it is tried again.
pub fn should_retry(attempts: u32, since: std::time::Duration) -> bool {
    let wait_secs = match attempts { 0 => 0, 1 => 5 * 60, 2 => 15 * 60, 3 => 30 * 60, _ => 60 * 60 };
    since.as_secs() >= wait_secs
}
```

`Scheduler` gains `pub attempts: Mutex<HashMap<String, (u32, std::time::Instant)>>` (key: the due slot's RFC-3339 string). The tick: after `due_slot` returns `Some(due)`, look up `attempts[due]`; if `should_retry(n, since)` is false, skip this tick; after `run_slot`, if the returned summary has `!engine_ok`, insert `(n + 1, Instant::now())`, else remove the key. `RunSummary` gains `pub attempts: u32` so the sync line can say `last slot 12:00 failed (attempt 3, retry in 30 min)` — the page's `last slot` branch appends ` (attempt N)` when `attempts > 1` (a two-line `console.js` change in this task, plus the static test literal `"attempt "`).

- [ ] **Step 4: Log retention and poison tolerance**

```rust
/// Keep the newest `keep` files in the slot-log directory; returns how many were removed. Sorted by
/// modified time, oldest first. Never touches anything but regular files directly in `dir`.
pub fn prune_logs(dir: &Path, keep: usize) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd.flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path()))).collect();
    if files.len() <= keep { return 0; }
    files.sort();
    let doomed = files.len() - keep;
    files.iter().take(doomed).filter(|(_, p)| std::fs::remove_file(p).is_ok()).count()
}
```

called from housekeeping on `n % 360 == 0` (once an hour) with `log_dir()` and `30`. Poison tolerance: one helper `fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }` at the top of `scheduler.rs`, and every `sch.running.lock().unwrap()`, `sch.last.lock().unwrap()`, `sch.mode_device.lock().unwrap()` in the file becomes `lock(&sch.…)` (the `RunGuard` and `refuse` already tolerate poison; they use the helper too).

- [ ] **Step 5: Backup joins the lock; quit flushes**

`backup_now_inner`: take `let _io = cs.vault_io.lock().unwrap_or_else(|e| e.into_inner());` for the duration of `run_backup` (before `cs.lock`, per the ordering doc). The scheduler's backup step likewise. Then:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct QuitFlush { pub synced: bool, pub backed_up: bool, pub timed_out: bool }

/// Push on close, synchronous, 10 s cap (console spec §8); backup on quit (Knowlu spec §4). Runs
/// the two engine calls on a helper thread and waits at most `cap` — a hung remote must not hold
/// the Quit click hostage. Whatever did not finish is simply not done: edits are on disk and in
/// the journal, the next launch's housekeeping picks them up (F10).
pub fn quit_flush(cs: &'static ConsoleState, cap: std::time::Duration) -> QuitFlush { … }
```

Implementation note: `quit_flush` needs the state to outlive the thread — take `cs: &ConsoleState` and do the work through an `Arc<…>`-free channel by running `run_sync`/`run_backup` inside `std::thread::scope`, with the receiving side using `recv_timeout(cap)`; on timeout return `timed_out: true` and let the scoped thread finish in the background **only if** the scope allows it — it does not (scope joins). So: the thread is a plain `std::thread::spawn` over a `'static` handle obtained via `tauri::State`/`AppHandle` (`app.state::<ConsoleState>()` is `'static` inside the tray callback) and the function signature is `quit_flush(app: &AppHandle, cap) -> QuitFlush` in `tray.rs`, with the testable core `quit_flush_inner(cs: &ConsoleState, cap) -> QuitFlush` in `state.rs` written with `std::thread::scope` and a `recv_timeout` — accepting that the scope joins after the timeout (the test never hangs because the test has no remote). The tray's `"quit"` arm becomes: stamp `quit_at`, save, `let q = quit_flush(app, Duration::from_secs(10));`, log it to `logs/quit-<ts>.txt`, `app.exit(0)`. `synced` is true only when `has_remote && auto_sync` and `run_sync` pushed or had nothing to push without error.

- [ ] **Step 6: Run everything, commit**

`cd app; cargo test` green, zero new warnings; `cargo test` (root) 0 warnings — `schedule.rs` tests only; no dual-run needed for a test-only `src/` change, but run `scripts\diff-engines.ps1 -Vault tests\fixtures\vault-s1` once anyway (seconds) since the crate rebuilt. Two commits: `app: the slot kills its process tree, retries with backoff, prunes its logs, tolerates poison, backs up under the vault lock, and pushes and backs up on quit (Knowlu plan 2, Task 5)` and `engine: DST-day slot arithmetic pinned (Knowlu plan 2, Task 5)`.

---

### Task 6: The phase-2 checklist and the registration script — written before the gate

**Files:**
- Create: `docs/runners/knowlu-phase-2.md`, `scripts/cutover/register-local-runner.ps1`, `scripts/cutover/README.md`
- Modify: `docs/HANDOFF.md` (one line in the NEXT SESSION block pointing at the checklist)

**Interfaces:**
- Consumes: the cutover plan's Task 1 registration block (its `Register-ScheduledTask` call with `-StartWhenAvailable`, `-AllowStartIfOnBatteries`, the 12:00/18:00 triggers); `docs/runners/knowlu-go-live.md`'s shape.
- Produces: the checklist Task 9 executes; the rollback script Task 9 names.

- [ ] **Step 1: `scripts/cutover/register-local-runner.ps1`** — the exact `Register-ScheduledTask` block from the cutover plan Task 1 step 5 (`$action`, `$triggers`, `$settings` with `-StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries`, the task name `quinn-ops-local-runner`), preceded by a header comment: *"ROLLBACK ONLY (Knowlu plan 2 Task 9). Re-registers the script runner on this laptop after `scheduler: app` is reverted. Never run while `scheduler: app` is in config/runners.yaml — two runners on one vault."* and a guard that reads the `local` entry's `scheduler:` key by the same regex idiom `local-run.ps1` used for `device:` and exits 1 if it says `app`.

- [ ] **Step 2: `scripts/cutover/README.md`** — five lines: what lives here (the cutover week's harness and its checks), that they need a Python venv the repo no longer keeps, that `compare-vaults.ps1` is pure PowerShell and still usable, that `register-local-runner.ps1` is rollback only, and the pointer to `docs/superpowers/plans/2026-09-02-rust-cutover-plan.md`.

- [ ] **Step 3: `docs/runners/knowlu-phase-2.md`**

```markdown
# Knowlu phase 2 — the runner leaves (after the harness is out)

Preconditions, all true before step 1:
- [ ] Cutover plan Task 9 step 2 is on main: no snapshot, no harness call, no `state/dual-run-log.md`; `scripts/local-run.ps1` runs pull → build → Rust coursework → Rust rank → push.
- [ ] Knowlu has been on the live vault since the go-live checklist (`docs/runners/knowlu-go-live.md`, step 7's line in `state/knowlu-log.md`) for at least three days with `synced` after every edit and no `refused:` line that was not intended.
- [ ] `cd app; cargo test` and root `cargo test` green at HEAD; the exe on the Start-menu shortcut is built from HEAD (`knowlu.exe` topline `build` == `git rev-parse --short HEAD`).
- [ ] The laptop is the machine named by `device:` in `config/runners.yaml` (it is; the key does not change).
- [ ] Nobody runs `--run-slot-once` on the live vault, before or after: the first app-run slot is the scheduled one.

Steps (Knowlu plan 2, Task 9 — the controller performs 2–3 with Quinn present; 4–6 are watching):
1. Confirm the next slot is at least 30 minutes away (`Get-Date`; slots 12:00 and 18:00 America/Chicago). Do not switch inside a slot's grace window.
2. `Unregister-ScheduledTask -TaskName quinn-ops-local-runner -Confirm:$false` on this laptop. `Get-ScheduledTask quinn-ops-local-runner` must now error. The script file stays until Task 10.
3. One commit on main: `config/runners.yaml` gains `scheduler: app` under the `local` entry (beside `device:`), plus a HANDOFF line `phase 2: scheduler: app since <date time> (commit <sha>)`. Push.
4. Within 60 s Knowlu's sync line reads `scheduler on` (the housekeeping pass re-reads the key). If it reads nothing, the app is not on HEAD's vault or the key is misspelled — fix before the slot.
5. Watch the slot fire on the clock: the tray shows running, then ok; the Runs view shows `local` and `coursework` runs with full summaries; `state/runs/<day>.jsonl` has both; `runner-log.md` gained the engine's own lines (the app writes none). **Do not act inside the 20-minute grace.** A `late` slot is on time for this purpose.
6. The next morning: two more slots seen, `synced` after each, backup `backed up` (the slot's backup tick). Then Task 10 removes the script.

Rollback (any step fails, or a slot is missed twice): revert the Task 9 commit (`scheduler: app` gone → the app is inert within 60 s), then `scripts\cutover\register-local-runner.ps1` on this laptop. Nothing else moves: the vault was written only by the engine exe either way.

What does NOT happen here: any rename (Tasks 11–12), removing the remote, touching the routine, `Disable`/`Enable` of the desktop's task (already stood down by the device guard; disable it when the desktop is next on).
```

- [ ] **Step 4: HANDOFF pointer, commit** — one line in the NEXT SESSION block: `- Phase 2 (the runner leaves) runs from docs/runners/knowlu-phase-2.md once cutover Task 9 step 2 is on main; plan 2 Part B follows it.` Commit: `docs: the phase-2 checklist and the rollback registration script, written before the gate (Knowlu plan 2, Task 6)`.

---

## Part B — after the gate: cutover plan Task 9 step 2 is on `main`

**Gate check (controller, ledgered before Task 7 dispatch):** `git log --oneline -20 main -- scripts/local-run.ps1` shows the harness-removal commit; `state/dual-run-log.md` does not exist; `scripts/local-run.ps1` contains no `dual-run`. Until then Part B waits; Part A's branch may merge to main on its own (Tasks 1–6 change nothing the runner reads).

### Task 7: Gauge counts on `rank`'s `tasks` step, and the gauge renders

**Files:**
- Modify: `src/cli.rs` (the `tasks` step), `src/surface.rs` (`EmptyText.gauge`, `EMPTY_TEXT.gauge`), `app/static/index.html` (the gauge strip in the topline), `app/static/console.js` (`renderGauge`), `app/static/console.css`
- Regenerate (in this commit, with the reason in the message): `tests/fixtures/surface-today-{s1,s1-migrated,full}.json` — the one new key `empty.gauge`
- Test: `src/cli.rs` (module test), `src/surface.rs` (`gauge` over planted records), `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `render::runway_days(&ranked, today, &cal)`, `render::capacity_breakdown(&cal, today).capacity`, `ranking::partition_must_do(&ranked, today, &cal)`, `Task::remaining_hours()`; `surface::gauge` (reads `runway_days`, `deficit_hours_x10`, `must_count` from the `tasks` step — already written); `state.gauge { points: [{date, runway_days, deficit_hours, must_count}], direction }`.
- Produces: the three counts on every `rank` run from the first run after this lands (F6); `state.empty.gauge`; `renderGauge(state)`.

- [ ] **Step 1: Failing tests**

In `src/cli.rs`'s existing run test (the one asserting `names == ["passes","tasks",…]` and `active == 5`):

```rust
        // Plan 2 Task 7 (S2 §7.7, F6): the three gauge counts, recorded from the first run.
        assert_eq!(count(&tasks.counts, "must_count"), out_must_count_from_fixture(), "must_count is the must-do partition's size");
        assert!(tasks.counts.iter().any(|(k, _)| *k == "runway_days"));
        assert!(count(&tasks.counts, "deficit_hours_x10") >= 0);
        // The count agrees with the topline's integer, or the gauge and the header disagree forever.
        let tl = crate::surface::topline(&vault, &crate::surface::load(&vault, today), today);
        assert_eq!(count(&tasks.counts, "runway_days"), tl.runway_days as i64);
```

(`out_must_count_from_fixture()` is not a helper — replace it with the literal the fixture produces: compute `partition_must_do` in the test and use `.0.len() as i64`.) In `src/surface.rs`, a `gauge` test that plants three `tasks` step records with the counts across three days in a scratch `state/runs/` and asserts three points and `direction == "losing"` when runway falls by 2 and deficit rises by 1. In `static_assets.rs`: `assert!(js.contains("function renderGauge(") && js.contains("no-history") && html.contains("id=\"gauge\""))` and that `console.js` contains `state.empty.gauge` (F18).

Run: FAIL (`must_count` absent; `renderGauge` missing).

- [ ] **Step 2: The counts**

In `src/cli.rs`, before the `steps` vector is built (after `ranked` and `cal` exist):

```rust
    // Gauge counts (S2 §7.7; plan 2 Task 7, F6). Three integers on the record `rank` already
    // writes every run — the trend history the gauge could never backfill. `runway_days` uses the
    // topline's own rounding so the header and the gauge never disagree by one.
    let gauge_runway = crate::render::runway_days(&ranked, today, &cal).round() as i64;
    let gauge_cap = crate::render::capacity_breakdown(&cal, today).capacity;
    let (gauge_must, _) = crate::ranking::partition_must_do(&ranked, today, &cal);
    let gauge_must_hours: f64 = gauge_must.iter().map(|t| t.remaining_hours()).sum();
    let gauge_deficit_x10 = ((gauge_must_hours - gauge_cap).max(0.0) * 10.0).round() as i64;
```

and the `tasks` step's counts become
`vec![("active", tasks.len() as i64), ("unreadable", skipped.len() as i64), ("runway_days", gauge_runway), ("deficit_hours_x10", gauge_deficit_x10), ("must_count", gauge_must.len() as i64)]`. `today.md` is untouched by construction (`tests/oracle.rs` stays green — assert it in the run).

- [ ] **Step 3: The empty text and the page**

`EmptyText` gains `pub gauge: &'static str` with `EMPTY_TEXT.gauge = "no history yet — the gauge fills from the next run"`. Regenerate the three surface references with `quinn-ops surface --vault tests/fixtures/<v> --view today` per `tests/surface_oracle.rs`'s documented procedure; the diff shows exactly the one added key per file. Page: in the topline template (`index.html`) add `<span id="gauge" class="gauge"></span>` after the runway figure; `console.js`:

```js
  // Plan 2 Task 7 (anatomy §3.5): fourteen runs of runway as a strip beside the runway figure,
  // and the engine's direction word. Nothing computed here — points and direction are the payload.
  function renderGauge(state) {
    var g = state.gauge, el = EL("gauge");
    if (!g || g.direction === "no-history" || !g.points.length) { el.textContent = state.empty.gauge; el.className = "gauge calm"; return; }
    var lo = Math.min.apply(null, g.points.map(function (p) { return p.runway_days; }));
    var hi = Math.max.apply(null, g.points.map(function (p) { return p.runway_days; }));
    var span = Math.max(hi - lo, 1);
    el.className = "gauge " + (g.direction === "losing" ? "amber" : "calm");
    el.innerHTML = g.points.map(function (p) {
      return '<i title="' + h(p.date) + " · runway " + p.runway_days + "d · deficit " + p.deficit_hours + "h" + '" style="height:' + Math.round(20 + 80 * (p.runway_days - lo) / span) + '%"></i>';
    }).join("") + '<span class="word">' + h(g.direction) + "</span>";
  }
```

called from the paint sequence after `renderTopline`. CSS: `.gauge { display: inline-flex; align-items: flex-end; gap: 1px; height: 12px; margin-left: 8px; } .gauge i { display: inline-block; width: 3px; background: currentColor; opacity: .7; } .gauge .word { margin-left: 6px; font: 500 11px var(--mono); }`.

- [ ] **Step 4: Run, commit**

`cargo test` 0 warnings (oracle green); `cd app; cargo test`; the surface oracle green against the regenerated references. One commit: `engine+page: gauge counts on rank's tasks step from the first run; the gauge renders; surface references regenerated for the one new key empty.gauge (Knowlu plan 2, Task 7)` — the message names the key and why (F18: the phrase belongs to the engine).

---

### Task 8: Coursework writes run records

**Files:**
- Modify: `src/coursework.rs` (`main_with_fetchers`)
- Test: `src/coursework.rs` (the fetcher-seam tests), `src/surface.rs` (`runs_panel` lists a `coursework` run and expects nothing of it)

**Interfaces:**
- Consumes: `runs::{start_run, add_step, end_run, git_sha, Record}`; `collect(...) -> Vec<Assignment>` (each `Assignment.uid` is prefixed `zybooks:` or `vhl:`); the `log` lines (`created …`, `updated …`, `skipped …`) and `warnings` `main_with_fetchers` already builds; `append_run_log` (kept, F17).
- Produces: one run per invocation under runner `coursework` with steps `zybooks`, `vhl`, `sync` and counts; `run_id` reused when the caller passed `--run-id` (the app passes none; the old script passed none).

- [ ] **Step 1: Failing test**

Beside `main_journals_under_the_runners_via_and_run_id`:

```rust
    /// Plan 2 Task 8 (S2 §15, F7): coursework is a runner like the others now — start, one step
    /// per source with its item count, a sync step with created/updated/skipped, and an end whose
    /// summary is the line `append_run_log` already wrote. A dry run writes no record at all.
    #[test]
    fn main_writes_a_coursework_run_record_with_per_source_counts() {
        let vault = scratch_vault("cw-runs");
        let fetchers: Vec<(&str, Fetcher)> = vec![("zybooks", Box::new(|_, _| Ok(two_zybooks_items()))), ("vhl", Box::new(|_, _| Err(SourceError::Dead("vhl".into()))))];
        assert_eq!(main_with_fetchers(&vault, false, "local-runner", None, Some(&fetchers)), 0);
        let day = crate::runs::Runs::new(&vault).read(None);
        let cw: Vec<_> = day.iter().filter(|r| r["runner"] == "coursework").collect();
        let phases: Vec<&str> = cw.iter().map(|r| r["phase"].as_str().unwrap()).collect();
        assert_eq!(phases, ["start", "step", "step", "step", "end"]);
        let names: Vec<&str> = cw.iter().filter(|r| r["phase"] == "step").map(|r| r["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["zybooks", "vhl", "sync"]);
        assert_eq!(cw[1]["counts"]["items"], 2);
        assert_eq!(cw[2]["result"], "WARN");
        assert_eq!(cw[3]["counts"]["created"], 2);
        assert_eq!(cw[4]["result"], "WARN");
        assert!(cw[4]["summary"].as_str().unwrap().starts_with("coursework (2 assignments; 2 created, 0 updated;"));
        // A dry run leaves state/runs/ exactly as it was.
        let before = crate::runs::Runs::new(&vault).read(None).len();
        assert_eq!(main_with_fetchers(&vault, true, "local-runner", None, Some(&fetchers)), 0);
        assert_eq!(crate::runs::Runs::new(&vault).read(None).len(), before);
    }
```

(`scratch_vault`, `two_zybooks_items`, `SourceError::Dead` — use the names the module's existing seam tests use; the dead-session test already constructs a failing fetcher.) Run: FAIL (no `coursework` runner records).

- [ ] **Step 2: The records**

In `main_with_fetchers`, after `ctx` is built and before the `outcome` closure:

```rust
    // Plan 2 Task 8 (S2 §15, F7): coursework joins RUNS. Its own runner name — not `local` — so
    // `expected_status` (which only knows the runners in config/runners.yaml) never mistakes a
    // coursework start for the slot's rank run. A dry run writes nothing.
    let run_id = if dry_run { None } else {
        Some(run_id.map(str::to_string).unwrap_or_else(|| crate::runs::start_run(vault, "coursework", None, None, crate::runs::git_sha(vault).as_deref())))
    };
```

`collect` needs to report per-source counts and per-source warnings: give it a small return change **only if** it does not already expose them — the cheapest seam is a `Vec<(String /*source*/, usize /*items*/, Vec<String> /*warnings*/)>` computed *after* `collect` from `assignments` (count by `uid` prefix) and from `warnings` (a warning names its source in its text: `zybooks: …` / `vhl: …`; count those prefixes). Then, after the `outcome` block and before printing:

```rust
    let mut steps: Vec<crate::runs::Record> = Vec::new();
    if let Some(rid) = run_id.as_deref() {
        for source in ["zybooks", "vhl"] {
            let items = assignments.iter().filter(|a| a.uid.starts_with(&format!("{source}:"))).count() as i64;
            let source_warns: Vec<&String> = warnings.iter().filter(|w| w.starts_with(&format!("{source}:")) || w.contains(&format!("({source})"))).collect();
            let result = if source_warns.is_empty() { "ok" } else { "WARN" };
            let msg = source_warns.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
            crate::runs::add_step(vault, rid, source, result, &[("items", items)], &msg, None);
            steps.push(step_record(source, result, &[("items", items)], &msg));
        }
        let created = log.iter().filter(|l| l.starts_with("created")).count() as i64;
        let updated = log.iter().filter(|l| l.starts_with("updated")).count() as i64;
        let skipped = log.iter().filter(|l| l.starts_with("skipped")).count() as i64;
        crate::runs::add_step(vault, rid, "sync", "ok", &[("created", created), ("updated", updated), ("skipped", skipped)], "", None);
        steps.push(step_record("sync", "ok", &[("created", created), ("updated", updated), ("skipped", skipped)], ""));
    }
```

where `step_record` builds the `Record` shape `end_run` folds (`name`, `result`, `counts`, `message` — the same shape `cli.rs` builds for `rank`; copy its constructor rather than inventing a second one: if `cli.rs` has a private helper, make it `pub(crate)` and call it). In the `if !dry_run` block, after `append_run_log`, add `if let Some(rid) = run_id.as_deref() { crate::runs::end_run(vault, rid, status, &summary, 0, None, crate::runs::git_sha(vault).as_deref(), &steps, Some("coursework")); }`. `journal_records` is `0` here because `sync_coursework` journals through `write` under its own actor; counting them would double-count the journal's own record of the run — say so in a comment.

- [ ] **Step 3: `runs_panel` proves the separation**

In `src/surface.rs` tests: plant a `coursework` start/end pair and a `local` pair in a scratch `state/runs/`; assert `runs_panel().recent` has both rows (runner names as written), `expected` rows name only `cloud`/`local` (from the fixture's `runners.yaml`), and a `WARN` coursework run raises `warn_count` by one. `run-records-reference.json` is untouched (`git status` shows it clean — assert in the commit's review, not in code).

- [ ] **Step 4: Run, commit**

`cargo test` 0 warnings; `tests/oracle.rs` green (coursework does not touch `today.md`). Commit: `engine: coursework writes start/step/end run records under its own runner name, per-source item counts and the sync counts (Knowlu plan 2, Task 8)`.

---

### Task 9: The switch — `scheduler: app`

**Files:**
- Modify: `config/runners.yaml` (one key), `docs/HANDOFF.md` (one line), `state/knowlu-log.md` (one line, `merge=union` is not needed — the app never writes it)
- Run: `docs/runners/knowlu-phase-2.md` steps 1–6

**Interfaces:**
- Consumes: the checklist (Task 6), `scripts/cutover/register-local-runner.ps1` (rollback), `schedule::SchedulerMode::parse`, `scheduler::mode` (re-read every 60 s by housekeeping).
- Produces: the key; the first app-run slot's records.

This task is a procedure with Quinn present (a system-state change and the first live run under the app). The controller performs it by the checklist and records each step's evidence in the SDD ledger.

- [ ] **Step 1: Preconditions** — every box in the checklist's preconditions, with the command output pasted into the ledger. If any is false, stop; the plan waits.
- [ ] **Step 2: Unregister the task** (checklist step 2), then prove it: `Get-ScheduledTask quinn-ops-local-runner` errors with "No MSFT_ScheduledTask objects found".
- [ ] **Step 3: The commit.** `config/runners.yaml`, under the `local` entry, directly after the `device:` line:

```yaml
    # Knowlu plan 2 Task 9 (spec §3, decision 6): the app's tray scheduler runs this runner's
    # slots. `script` (or absent) means the PowerShell task did; that task was unregistered
    # in the same step this key was written. Neither engine reads this key.
    scheduler: app
```

`docs/HANDOFF.md` NEXT SESSION block: `- Phase 2 given <date time CT>: scheduler: app (commit <sha>); the scheduled task is unregistered; the script leaves in plan 2 Task 10.` Commit both: `config: scheduler: app — Knowlu runs the local slots; the scheduled task is unregistered (Knowlu plan 2, Task 9)`. Push from the main checkout (this commit goes straight to `main`: the app reads the vault, and the live vault is the main checkout).
- [ ] **Step 4: Confirm pickup** — within 60 s the sync line shows `scheduler on` (`topline.scheduler.mode == "app"`, `paused == false`). Evidence: a DPI-aware `PrintWindow` capture, or the `state` payload from `quinn-ops surface --vault <live> --view today` is **not** sufficient (the scheduler status is app-side) — the capture it is.
- [ ] **Step 5: Watch the slot** (checklist step 5). Evidence into the ledger: the `local` and `coursework` run ids, the tray state, the sync line after the slot. Do not act inside grace.
- [ ] **Step 6: Next morning** (checklist step 6). Then Task 10.

Rollback is the checklist's; if it is used, the ledger records why and the plan pauses at Task 9.

---

### Task 10: The runner leaves — scripts retired, venv gone, docs rewritten

**Files:**
- Delete: `scripts/local-run.ps1`
- Move (git mv): `scripts/dual-run.ps1`, `scripts/compare-vaults.ps1`, `scripts/rehearse-rollback.ps1`, `scripts/diff-engines.ps1`, `scripts/diff-engines-notes.ps1`, `scripts/lint-yaml-11.py`, `scripts/diagnose-local-runner.ps1` → `scripts/cutover/`
- Modify: `CLAUDE.md` (the "Local runner", "Coursework ingest", "Tests" and "Dual runs" bullets), `docs/HANDOFF.md` (the NEXT SESSION block's runner lines), `.gitattributes` (remove the `state/dual-run-log.md` line if cutover Task 9 left it), `scripts/cutover/README.md` (list what arrived)
- Local, untracked: remove `.venv` from the main checkout and the `.venv` junction from the worktree

**Interfaces:**
- Consumes: Task 9 done (two clean app-run days); the cutover plan's Task 9 step 2 already removed the harness from the script.
- Produces: a repository where nothing runs the Python engine locally.

- [ ] **Step 1: Prove nothing references the script.** `grep -rn "local-run.ps1" --include=*.rs --include=*.ps1 --include=*.py . | grep -v "^./docs\|^./scripts/cutover"` returns nothing but `scripts/local-run.ps1` itself and `diagnose-local-runner.ps1` (moving with it). `Get-ScheduledTask quinn-ops-local-runner` still errors (Task 9).
- [ ] **Step 2: The moves and the delete.** `git rm scripts/local-run.ps1`; `git mv` each of the seven; append to `scripts/cutover/README.md` one line per file with what it was for. Nothing in `tests/` references any of them (`grep -rn "diff-engines\|dual-run\|compare-vaults" tests/ src/ app/` is empty).
- [ ] **Step 3: CLAUDE.md.** The "Local runner" bullet becomes: *"**Local runner: Knowlu** (`scheduler: app` since <date>, plan 2 Task 9). The tray scheduler runs `knowlu-engine coursework` then `knowlu-engine rank` as child processes on the slots in `config/runners.yaml`, commits by name, pushes while a remote exists, and ticks the backup. There is no scheduled task, no script and no venv on this machine; `scripts/cutover/` holds the cutover week's harness for the record."* The "Coursework ingest … LIVE in the local runner only" bullet: the runner is Knowlu; the credential targets sentence gains "(`knowlu/…` after plan 2 Task 12)". The "Tests" bullet: *"`cargo test` (engine) and `cd app; cargo test` (console) are the suites; `0 warnings` is part of green. The Python suite (687) still describes `engine/`, which the cloud routine runs until plan 3 — run it from a throwaway venv only when `engine/` changes, which this repo no longer plans."* The "Dual runs are scripts" bullet is rewritten in the past tense with the `scripts/cutover/` path. **Rule 1's banner** (the top of CLAUDE.md) is rewritten by the cutover plan's Task 10, not here; if that has not happened, add one line under it: *"Plan 2 phase 2 done <date>: the local runner is Knowlu."*
- [ ] **Step 4: The venv.** In the main checkout, `Remove-Item -Recurse -Force .venv` (untracked; nothing tracked changes). In the worktree, remove the junction (`cmd /c rmdir .venv`). `.gitignore` keeps `.venv/`.
- [ ] **Step 5: Run, commit.** `cargo test` 0 warnings; `cd app; cargo test`; `git diff --stat` shows renames, not rewrites (line endings untouched by `git mv`). Commit: `runner: the PowerShell runner leaves — script deleted, the cutover harness retired to scripts/cutover/, the venv gone, CLAUDE.md and HANDOFF say Knowlu is the local runner (Knowlu plan 2, Task 10)`.

---

### Task 11: Rename stage 2 — the engine crate, binary, clap name and user-agent

**Files:**
- Modify: `Cargo.toml` (`[package] name`, `[lib] name`, `[[bin]] name`), `Cargo.lock` (regenerated by cargo), `src/main.rs` (`#[command(name = …)]`), `src/lib.rs` (crate doc line), `src/zybooks.rs` and `src/vhl.rs` (`USER_AGENT`), `src/wincred.rs` (test literals), `app/Cargo.toml` (the path dependency's name), every `use quinn_ops::` / `quinn_ops::` in `app/src/*.rs`, `app/tests/*.rs`, `tests/*.rs`; `app/src/scheduler.rs` (`engine_exe` sibling name); `scripts/scratch-vault.ps1`, `docs/runners/knowlu-go-live.md`, `docs/runners/knowlu-phase-2.md`, `app/README.md` (the binary name where it is a command to type)
- Test: `tests/dependency_boundary.rs` (unchanged — it reads `Cargo.toml` by path), a new `tests/names.rs`

**Interfaces:**
- Consumes: decision 17 (rename stage 2); the plan's Global Constraints naming row.
- Produces: `knowlu-engine.exe`, crate `knowlu-engine`, lib `knowlu_engine`; `KNOWLU_ENGINE_EXE` unchanged.

**Ruling recorded here for Quinn (R-P2-1):** the spec's §3 sentence *"runs the sibling `knowlu.exe` (today `quinn-ops.exe`)"* cannot hold once both binaries share a folder in the installer (plan 4): the console is already `knowlu.exe`. The engine binary is therefore **`knowlu-engine.exe`**, the crate `knowlu-engine`. One name, said once, reversible in one commit if Quinn prefers another.

- [ ] **Step 1: The names test (fails first)**

```rust
//! Plan 2 Task 11 — rename stage 2 (Knowlu spec §7). What the running system depends on carries
//! the Knowlu name; history and references never do.
use std::fs;
use std::path::Path;

fn rust_sources() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for dir in ["src", "tests", "app/src", "app/tests"] {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("rs") { out.push(p); }
        }
    }
    out
}

#[test]
fn the_engine_crate_is_knowlu_engine_everywhere_code_names_it() {
    let manifest = fs::read_to_string("Cargo.toml").unwrap();
    assert!(manifest.contains("name = \"knowlu-engine\"") && manifest.contains("name = \"knowlu_engine\""), "crate and lib names");
    assert!(!manifest.contains("quinn-ops") && !manifest.contains("quinn_ops"), "no old name in the manifest");
    for p in rust_sources() {
        let s = fs::read_to_string(&p).unwrap();
        assert!(!s.contains("quinn_ops::"), "{}: still imports the old crate name", p.display());
    }
    let main = fs::read_to_string("src/main.rs").unwrap();
    assert!(main.contains("name = \"knowlu-engine\""), "clap name");
    for f in ["src/zybooks.rs", "src/vhl.rs"] {
        let s = fs::read_to_string(f).unwrap();
        assert!(s.contains("x64) Knowlu\""), "{f}: user-agent identifies Knowlu");
    }
    let sched = fs::read_to_string("app/src/scheduler.rs").unwrap();
    assert!(sched.contains("knowlu-engine.exe") && !sched.contains("quinn-ops.exe"), "the sibling the app runs");
}

/// What must NOT change: the vocabulary history is written in.
#[test]
fn history_vocabulary_is_untouched_by_the_rename() {
    for (f, needle) in [("src/journal.rs", "QUINN_OPS_DEVICE"), ("src/cli.rs", "\"local\""), ("src/coursework.rs", "local-runner")] {
        assert!(fs::read_to_string(f).unwrap().contains(needle), "{f} must still say {needle}");
    }
    for r in ["golden-today-s1.md", "golden-today-full.md", "run-records-reference.json", "pyyaml-safe-dump-reference.json", "zybooks-parsed-reference.json", "vhl-parsed-reference.json"] {
        assert!(Path::new("tests/fixtures").join(r).is_file(), "{r} still exists");
    }
}
```

Run: `cargo test --test names` — FAIL on the manifest.

- [ ] **Step 2: The mechanical rename** — a scratchpad script (PowerShell, `[IO.File]::ReadAllText`/`WriteAllText` with the file's own encoding, no `Get-Content`) that, over `app/src/*.rs`, `app/tests/*.rs`, `tests/*.rs`, replaces `quinn_ops::` → `knowlu_engine::` and `quinn_ops_console` → `knowlu_desktop` (Task 12's lib; done here so the app compiles once); `Cargo.toml`: `name = "quinn-ops"` → `"knowlu-engine"`, `[lib] name = "quinn_ops"` → `"knowlu_engine"`, `[[bin]] name = "quinn-ops"` → `"knowlu-engine"`; `app/Cargo.toml`: the dependency line `quinn-ops = { path = ".." }` → `knowlu-engine = { path = ".." }` (and `[package] name`/`[lib] name` per Task 12, done here); `src/main.rs` clap `name = "knowlu-engine"`; the two `USER_AGENT` consts → `"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu"`; `engine_exe()` sibling → `knowlu-engine.exe` and its two error strings; `src/wincred.rs` test literals `quinn-ops/zybooks` → `knowlu/zybooks` (test-only strings; production targets come from config). The zyBooks/VHL UA tests assert the constant, not a literal — verify by reading them; if one hard-codes `quinn-ops`, update the literal in the same commit.
- [ ] **Step 3: Build both crates, run everything.** `cargo build --release` at root produces `target\release\knowlu-engine.exe`; `cd app; cargo build --release` produces `knowlu.exe` linking `knowlu_engine`. `cargo test` 0 warnings; `cd app; cargo test`; `tests/oracle.rs`, `tests/surface_oracle.rs` green (no behaviour changed). `git diff --stat`: `Cargo.lock` changes (package name), no whole-file flips.
- [ ] **Step 4: The exe the app runs.** After Task 9 the live app runs the sibling engine exe from `app\target\release\` — **the shortcut's exe folder must contain `knowlu-engine.exe` before this merges to main**, or the next slot fails with `engine not found`. The merge step therefore builds both crates in the main checkout and copies `target\release\knowlu-engine.exe` beside `app\target\release\knowlu.exe` (the runner's build step used to do this — after Task 10 nothing builds on a schedule, so the go-live checklist's rule "both from one HEAD" is now a **release** rule: it goes into `docs/runners/knowlu-phase-2.md` as a post-merge step and into plan 4's release script). Until plan 4, `KNOWLU_ENGINE_EXE` in the shortcut's environment is the documented alternative.
- [ ] **Step 5: Commit** — `rename: stage 2 — the engine is knowlu-engine (crate, lib, binary, clap name), the user-agent says Knowlu, the app links knowlu_engine (Knowlu plan 2, Task 11)`.

---

### Task 12: Rename stage 2 — the app crate, the app-data root, the identifier, the credential targets

> **Amended 2026-09-06 by Knowlu plan 4a.** **Steps 2 and 3 are done and become checks**, and
> **step 1's test is superseded.** Plan 4a Task 2 moved the app-data root (`app/src/state.rs`'s
> `app_data_root`/`app_data_root_in` plus `app/src/profiles.rs`'s `migrate_flat_layout`) and Task 9
> set the identifier. What is left in this task is the crate rename, the script/doc paths (step 4),
> the build and commit (step 5), and the gated credential targets (step 6).

**Files:**
- Modify: `app/Cargo.toml` (`[package] name = "knowlu-desktop"`, `[lib] name = "knowlu_desktop"`; done in Task 11's script, verified here), `app/tauri.conf.json` (`identifier`), `app/src/state.rs` (`app_data_root` with the one-time move), `app/src/main.rs` (the `--run-slot-once` temp dir name is fine), `scripts/scratch-vault.ps1` (`%LOCALAPPDATA%\knowlu\scratch`), `docs/runners/knowlu-go-live.md` + `knowlu-phase-2.md` + `app/README.md` (the settings path), `config/ingest.yaml` (`credential_target` × 2 — **step 6, gated**)
- Test: `app/tests/commands.rs` or a new `app/tests/appdata.rs`

**Interfaces:**
- Consumes: `app_data_root()` (plan 1 B3); `Settings`; the credential targets read by `coursework` from `config/ingest.yaml`'s `coursework:` block (`credential_target:`).
- Produces: `pub fn app_data_root_in(local_app_data: &Path) -> PathBuf` (pure, tested) with `app_data_root()` calling it on `%LOCALAPPDATA%`; identifier `com.knowlu.desktop`.

- [ ] **Step 1: Failing tests — SUPERSEDED, do not write this one** (S12)

**The test below asserts a root-level `settings.json`, and there is no longer any such file**: since
plan 4a the flat files live under `profiles\<profile_id>\`. Writing it now would fail against
correct code, and the obvious next move — "fixing" the code to match the assertion — would undo the
per-profile layout. `app/tests/profiles.rs` already covers this ground, under
`the_flat_files_fold_into_the_profile_folder_and_nothing_is_lost` and
`a_half_moved_layout_is_folded_on_the_next_launch_and_never_stranded`; read those instead. The
`static_assets.rs` half below (the identifier and the scratch path) is still worth having, and the
identifier assertion is already green.

The superseded test, kept only so nobody re-derives it:

```rust
/// SUPERSEDED by plan 4a — see the note above; kept for the record, not to be added.
/// Plan 2 Task 12 (F14): `%LOCALAPPDATA%\knowlu` is the root; an existing `quinn-ops` root is
/// MOVED there once (settings, seen stamp, logs, scratch — everything), never copied and never
/// left behind as a second source of truth.
#[test]
fn the_app_data_root_is_knowlu_and_an_old_root_is_moved_once() {
    let base = std::env::temp_dir().join(format!("knowlu-appdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base); std::fs::create_dir_all(base.join("quinn-ops").join("logs")).unwrap();
    std::fs::write(base.join("quinn-ops").join("settings.json"), b"{}").unwrap();
    let root = app_data_root_in(&base);
    assert_eq!(root, base.join("knowlu"));
    assert!(root.join("settings.json").is_file() && root.join("logs").is_dir(), "moved whole");
    assert!(!base.join("quinn-ops").exists(), "nothing left behind");
    // Second call: idempotent, no old dir, no error.
    assert_eq!(app_data_root_in(&base), base.join("knowlu"));
    // Both present (a half-done move, or a manual restore): the new root wins, the old is left alone.
    std::fs::create_dir_all(base.join("quinn-ops")).unwrap();
    assert_eq!(app_data_root_in(&base), base.join("knowlu"));
    assert!(base.join("quinn-ops").is_dir());
    let _ = std::fs::remove_dir_all(&base);
}
```

and in `static_assets.rs` (or `names.rs`): `tauri.conf.json` contains `"identifier": "com.knowlu.desktop"` and no `app.knowlu.desktop`; `scripts/scratch-vault.ps1` contains `knowlu\scratch`. Run: FAIL.

- [ ] **Step 2: The root — ALREADY DONE by plan 4a Task 2; verify, change nothing.** Check that
`%LOCALAPPDATA%\knowlu` exists and holds `profiles.json`, and that `state::app_data_root()` is still
the one place the path is decided. The shipped shape is **not** the sketch below: the move lives in
`profiles::migrate_flat_layout`, it retries on every launch rather than being lost to one failed
rename, and it folds `settings.json` / `seen.txt` / `logs\` under `profiles\<profile_id>\` on the
way. Kept for the record:

```rust
/// SUPERSEDED by plan 4a Task 2 — the shipped move is `profiles::migrate_flat_layout`.
/// The app's data root under a given `%LOCALAPPDATA%` (plan 2 Task 12, F14). `knowlu`, created on
/// demand. If only the pre-rename `quinn-ops` root exists it is renamed — one `fs::rename`, same
/// volume, atomic on NTFS — so settings, the seen stamp, logs and scratch vaults all carry over.
/// If both exist nothing is merged: the new root is authoritative and the old one is left for a
/// human (a message in `diagnostics_text` names it).
pub fn app_data_root_in(local_app_data: &Path) -> PathBuf {
    let new = local_app_data.join("knowlu");
    let old = local_app_data.join("quinn-ops");
    if !new.exists() && old.is_dir() {
        if std::fs::rename(&old, &new).is_err() { let _ = std::fs::create_dir_all(&new); }
    } else {
        let _ = std::fs::create_dir_all(&new);
    }
    new
}
pub fn app_data_root() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty())?;
    Some(app_data_root_in(Path::new(&base)))
}
```

`diagnostics_text` (tray "Copy diagnostics") adds a line `old app-data root still present: <path>` when both exist. `scheduler::log_dir` already uses `app_data_root()`.

- [ ] **Step 3: The identifier — ALREADY DONE by plan 4a Task 9; verify, change nothing.** Check that `app/tauri.conf.json` reads `"identifier": "com.knowlu.desktop"` (a static test in `app/tests/static_assets.rs` already pins it) and move on. For the record: reverse-DNS of `knowlu.com`, which Quinn holds (2026-09-05). This is the identifier the installer, the updater manifest and Trusted Signing's publisher use; changing it later would orphan installed copies, so it is settled. The `tauri_plugin_autostart` entry and `window-state` file are keyed by it: the first launch after this re-registers autostart under the new name (the plugin's `enable()` on startup does that) and window position resets once — say so in README's "Known wrinkles".

- [ ] **Step 4: Paths in scripts and docs.** `scripts/scratch-vault.ps1`: `$dest = Join-Path $env:LOCALAPPDATA "knowlu\scratch\$stamp"` and its header comment. `app/README.md`, `docs/runners/knowlu-go-live.md` (a note: "since plan 2 Task 12 the root is `%LOCALAPPDATA%\knowlu`; the first launch moves the old one"), `knowlu-phase-2.md`: the same.

- [ ] **Step 5: Build, test, commit (steps 1–4).** `cd app; cargo test` (the new test, static assets); `cargo build --release` in `app/`. Commit: `rename: stage 2 — knowlu-desktop, %LOCALAPPDATA%\knowlu with the one-time move, identifier com.knowlu.desktop (Knowlu plan 2, Task 12)`.

- [ ] **Step 6: Credential targets — gated on Quinn.** Ask Quinn, once, in these words: *"Rotate the zyBooks and VHL passwords, then from a terminal that is not this session run `cmdkey /generic:knowlu/zybooks /user:<user> /pass` and `cmdkey /generic:knowlu/vhl /user:<user> /pass` (each prompts), and reply when `cmdkey /list` shows both `knowlu/` targets."* Only after that reply: edit `config/ingest.yaml`'s two `credential_target:` lines to `knowlu/zybooks` and `knowlu/vhl`, run `knowlu-engine coursework --vault <scratch copy of the live vault> --dry-run` **once** (it authenticates; it is the verification the rotation needs and counts as this plan's one credential check — report `would update` lines and no `no credential` error), commit `config: credential targets knowlu/zybooks, knowlu/vhl after the rotation (Knowlu plan 2, Task 12 step 6)`, and tell Quinn the old `quinn-ops/…` entries can be removed with `cmdkey /delete:quinn-ops/zybooks` and `/delete:quinn-ops/vhl`. If Quinn has not replied when Task 13 closes, the plan closes with this step open and the ledger says so; the old targets keep working meanwhile.

---

### Task 13: Close — anatomy, README, HANDOFF, CLAUDE.md, screenshots, status line

**Files:**
- Modify: `docs/surface/anatomy.md` (§2 the page set — three views built; §3.1 gauge strip; §3.5 gauge live; §3.9 the Decisions view; §3.12 GOOD TO KNOW view; §3.14 RUNS with coursework; §4.6 unchanged actions; §7.1 parity rows for the three views), `app/README.md` (the crate names, the data root, the identifier, the slot procedure, `KNOWLU_ENGINE_EXE`, the release rule from Task 11 step 4), `docs/HANDOFF.md` (`▶ KNOWLU PLAN 2 DONE` block above plan 1's), `CLAUDE.md` (the `## Knowlu (the console)` section: the runner sentence, `knowlu-engine`, the data root; the toolchain bullets that name `quinn-ops.exe`), this plan's status line
- Run: `scripts/console-shots.py` with a view argument (a five-line extension: an optional third argument `--views today,decisions,good-to-know,issues,runs` that navigates by hash before each shot), plus one DPI-aware `PrintWindow` capture of the live app at 1280×860 on the Decisions view

**Interfaces:**
- Consumes: everything above.
- Produces: docs that describe the code; `shots/*-decisions.png` etc. (ignored); the plan status line.

- [ ] **Step 1: Screenshots.** Extend `scripts/console-shots.py`: `VIEWS = sys.argv[3].split(",") if len(sys.argv) > 3 else ["today"]`; for each view, `page.goto(url + "#" + view)` before the shot and name the file `<width>-<view>.png`. Run it at the eight viewports over the five views against `tests/fixtures/surface-today-full.json` — every ≥820 viewport `ok`. Then one real capture, DPI-aware:

```python
import ctypes, subprocess, time, sys
ctypes.windll.shcore.SetProcessDpiAwareness(2)   # BEFORE any window call (plan 1 Task 16's lesson)
import win32gui, win32ui, win32process
exe, vault, out = sys.argv[1:4]
proc = subprocess.Popen([exe, "--vault", vault]); hwnd = None
for _ in range(60):
    def cb(h, _):
        global hwnd
        _, pid = win32process.GetWindowThreadProcessId(h)
        if pid == proc.pid and win32gui.IsWindowVisible(h) and win32gui.GetWindowText(h): hwnd = h
    win32gui.EnumWindows(cb, None)
    if hwnd: break
    time.sleep(0.5)
time.sleep(4)
l, t, r, b = win32gui.GetWindowRect(hwnd); w, hgt = r - l, b - t
dc = win32gui.GetWindowDC(hwnd); mdc = win32ui.CreateDCFromHandle(dc); sdc = mdc.CreateCompatibleDC()
bmp = win32ui.CreateBitmap(); bmp.CreateCompatibleBitmap(mdc, w, hgt); sdc.SelectObject(bmp)
ctypes.windll.user32.PrintWindow(hwnd, sdc.GetSafeHdc(), 2)
bmp.SaveBitmapFile(sdc, out)
proc.terminate()
```

(`pywin32` into `.wv`; the window opens at its default 1280×860 logical so no resize is needed; navigate to Decisions by launching with `#decisions`? The app has no URL argument — instead capture Today, and rely on the headless shots for the three views. Say so in the report.) Look at the PNGs and record what they show in the anatomy's §5 note, as plan 1 did.

- [ ] **Step 2: Anatomy.** §2: the page set table marks Decisions, Good to know, Issues, Runs as built, with the view's main-column content in one line each. §3.1: the gauge strip beside runway. §3.5: "renders since plan 2 Task 7; `no history yet` until fourteen runs exist" and the count keys. §3.9: a paragraph "The Decisions view" — every card, the header's counts, the hidden-amendments count, rows act like the deck (busy guard, note preserved per id). §3.12: the view and the close. §3.14: coursework runs under their own runner name; `expected` never lists it. §7.1: three rows (Decisions view → `decisions.cards` → `decide` → `decision_made`/`decision_deferred`; Good to know view → `good_to_know` → `close_info` → none; Issues view → `issues_panel` → `resolve_issue` → none — no new action, F19). Every `Computed by:` names the payload key.

- [ ] **Step 3: README, HANDOFF, CLAUDE.md, status line.** README: names (crate, lib, binaries), the data root and the one-time move, the identifier, the slot procedure (what a slot does, the backoff cadence, the tree kill, the logs and their retention), quit's flush, `KNOWLU_ENGINE_EXE`, the release rule (both binaries from one HEAD, beside each other). HANDOFF: `▶ KNOWLU PLAN 2 DONE (<date>)` — what shipped, the phase-2 date and commit, the rename, what stays open (credential targets if Quinn has not replied; the release rule until plan 4), plan 3 next. CLAUDE.md: the console section's three lines updated (`knowlu-engine`, the root, "the app is the local runner"); the toolchain bullets (`target\release\quinn-ops.exe` → `knowlu-engine.exe`); rule 1's banner gets its phase-2 line if the cutover plan's Task 10 has not rewritten it. This plan's status line: `**Status: EXECUTED <date> on branch worktree-knowlu-plan-2 (Tasks 1–13; the SDD ledger is preserved at docs/superpowers/reports/<date>-knowlu-plan-2-sdd-ledger.md).**`

- [ ] **Step 4: Commit.** `git add docs/surface/anatomy.md app/README.md docs/HANDOFF.md CLAUDE.md docs/superpowers/plans/2026-09-05-knowlu-runner-leaves-plan.md scripts/console-shots.py` — `docs: Knowlu plan 2 — the three views, the gauge, coursework in RUNS, the runner is Knowlu, stage-2 names (Knowlu plan 2, Task 13)`.

---

## Self-review (writing-plans checklist)

**Spec coverage.** Knowlu spec §9 item 2: `scheduler: app` (Task 9), script and scheduled task removed (Tasks 9–10), the venv gone (Task 10), rename stage 2 (Tasks 11–12), the remaining views Decisions/Good to know/Issues (Tasks 1–3), the gauge counts and coursework run records that waited for the cutover (Tasks 7–8). Decision 19 (Runs view) landed in plan 1. §3's failure visibility (Task 5's `attempts`, the sync line), §4's on-quit trigger (Task 5), §7 stage 2's list — engine crate and binary, clap name, app-data root, user-agent, credential targets, test and script references (Tasks 11–12) — all placed; stage 3 items (`QUINN_OPS_DEVICE`, repo, folder) explicitly not. Console spec §4.5/§10's two gated engine changes are Part B's first two tasks. Plan 1's carry-over list: process-tree kill, retry storm, log retention, DST arithmetic (Task 5); A1 residual, side-branch rebase, git timeout (Task 4); `backup_now` outside `vault_io`, poison tolerance, push/backup on quit (Task 5); the console spec's "N edits" lines (left: the spec is a dated record; anatomy is the living doc — Task 13 says so in one line); "1 events" (Task 1); the over-strict "all four keys" (Task 5's docs rider — **add**: Task 5 step 6's commit touches `app/README.md` and `docs/runners/knowlu-go-live.md` to say only `profile_id` and `autostart` are required, `backup_dir` and `quit_at` may be omitted). Plan 4's items (settings UI, drawer affordance, status enum) stay out.

**Placeholder scan.** No TBD/TODO. Two places point at existing code rather than restating it — Task 4 step 1's helper names and Task 8 step 2's `step_record` — each says exactly which file and what to do if the helper is not there. Task 5's `quit_flush` note resolves its own lifetime question (scoped thread + `recv_timeout`; the test never hangs because it has no remote).

**Type consistency.** `run_child(exe, args, log, timeout: Duration) -> i32` (plan 1 fix wave) is what Task 5 calls; `should_retry(u32, Duration) -> bool`; `prune_logs(&Path, usize) -> usize`; `quit_flush_inner(&ConsoleState, Duration) -> QuitFlush`; `git_with(&str, &Path, &[&str], Duration) -> Result<String, String>`; `app_data_root_in(&Path) -> PathBuf`; `renderDecisionsView/renderGoodToKnowView/renderIssuesView/renderGauge(state)`; `decideCard(id, verdict, snoozeUntil, host)`; `resolveIssue(id, text, btn)`. Counts keys `runway_days`, `deficit_hours_x10`, `must_count` match `surface::gauge`'s reader exactly. Runner name `coursework` in Task 8 matches Task 9's checklist and Task 13's anatomy text.

**Gate discipline.** Part A never changes what `rank` or `write` writes (Task 4 is history; Task 5's engine change is tests only). Part B's first line is the gate check. Task 9 is the only live run, watched on the clock.

**Open items this plan will surface to Quinn one at a time (his rule, 2026-09-05):** (1) the engine binary name `knowlu-engine.exe` — Task 11's ruling R-P2-1, at Task 11 dispatch; (2) the credential rotation and re-store — Task 12 step 6; (3) the desktop's task disable — mentioned at Task 9, not blocking; (4) the release rule (both binaries beside each other) until plan 4 — at Task 11 merge.
