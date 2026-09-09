# The console on Rust — design

**Written 2026-09-02**, the day the Tauri spike ran (`app/README.md`) and the post-cutover order was
reordered to *Tauri spike → console on Rust → profiles + onboarding + installer* (rewrite spec §15,
Quinn: "I accept the reorder"). Brainstormed with Quinn the same afternoon; every decision below
was put to him in sections and approved.

**This is a translation, not a redesign.** `docs/superpowers/specs/2026-08-31-s2-surface-design.md`
(S2) is the design of record for *what* the console is: its views, its completeness rule, its read
model, its write semantics, its empty states, its sync loop. This document decides *how* that
design is built on Rust + Tauri, records the four questions S2 left open and Quinn has now answered,
and names every place it deviates from S2 and why. Where this document is silent, S2 stands.

**Read with:** S2 §2 (the mockup is a mockup), §5 (views), §6 (the parity audit — the part to
review hardest), §7 (the read model), §9 (the page), §10 (sync), §13 (failure modes);
`docs/surface/anatomy.md`; `docs/mockups/2026-08-31-s2-console-CHOSEN.html`; VISION.md;
`app/README.md`.

---

## 1. Purpose

One window Quinn opens in the morning and operates the system from all day — every number the
engine computes visible, every property changeable by clicking, every decision one click, every
edit journalled — built on the Rust engine, inside a Tauri shell, at a size a friend can install.

The first shippable slice is **Today + click-CRUD** (Quinn, 2026-09-02): the console is the way
the system is operated from day one, not a report.

### What S2 settled and this document carries unchanged

The completeness rule and its audit (S2 §2, §6, §14.1); the eight views inside one window (§5);
the read-model builders and what each computes (§7); the row fields and the region → payload map
(§9.1); click-CRUD's editable set and the new-task minimum fields (§9.2); the drawer (§9.3); issue
flags (§9.4); the deck's shape and ordering (§9.5); every section always renders (§9.6); ranking is
soft (§9.8); the sync loop (§10); customisability and vendored fonts (§11); the failure table
(§13); every decision in §15 except the two named in §14 below.

### What changes

| S2 | Here | Why |
|---|---|---|
| Python `engine/surface.py`, `serve.py`, `sync.py`; `app/window.py` + pywebview | `src/surface.rs`, `src/sync.rs` in the engine crate; `app/` a Tauri crate linking the engine | The rewrite (spec decision 14) |
| Loopback HTTP + JSON API, hardened (§3.4) | Tauri IPC, in-process; no port, token, Host or Origin check | Tauri removes the process boundary the server existed to cross |
| `render.py` imports shared helpers from `surface.py` | `surface.rs` calls `pub` helpers in `render.rs` | Touches `render.rs`'s visibility only; the oracle proves `today.md` unmoved |
| Window geometry in `state/.window.json` | `tauri-plugin-window-state`, app-data dir | Shell state stays out of a vault that, for a friend, is not a git repo |
| A decision is executed at the next runner pass (§8, implicit) | `decide` executes it in the same call | Criterion 3 — a write re-sorts in about a second — is defeated by a six-hour wait |
| Console launched as `pythonw -m app.window` | `quinn-ops-console.exe --vault <path>` | — |

---

## 2. Questions S2 left open, answered 2026-09-02

| S2 § | Question | Quinn's answer |
|---|---|---|
| 16.1 | "Everything on one page" — one screen, or one window with swapping views? | **One window, permanent rails, main column swaps.** S2 §5 as written. |
| — | First slice | **Today + click-CRUD.** Read-only was offered and declined. |
| — | Friends' platforms | **All Windows.** `wincred` and Task Scheduler stay; no keychain seam this milestone. |
| — | Sync in this milestone | **Yes, S2 §10 as designed.** Deferral was recommended and declined. |
| 16.2 | Delta window | "Since I last looked", per device (S2's reading; taken here). |
| 16.3 | Launch at login | No. A window that opens itself is a notification. Start-menu shortcut. |
| 16.4 | The PH 106 alert | A `.cf`-style line under the AHEAD takeaway when it fires, at most once a day. |
| 16.5 | Runway's threshold strip | Stays parked. |
| — | Engine boundary | **A with B folded in** (§3): in-process library, plus the read model as a CLI. |

---

## 3. Architecture — two crates, two binaries, one commit

```
quinn-ops/                        the engine crate (lib + bin); shape unchanged
  src/surface.rs                  NEW  the read model: pure builders; where the logic and the tests live
  src/sync.rs                     NEW  git status/sync + the reconcile driver (S2 §10)
  src/render.rs                   three helpers become pub (status word, capacity breakdown, due_move)
  src/scheduling.rs, ranking.rs   explained variants of start_by / designate_today (§4.3)
  src/main.rs                     `quinn-ops surface --vault <v> --view <view> [--today D] [--now T]`
  tests/fixtures/surface-*.json   NEW  frozen read-model references (§4.6)
app/                              the console crate: quinn-ops-console.exe
  Cargo.toml                      quinn-ops = { path = ".." }, tauri, serde, serde_json, two official plugins
  src/main.rs                     builder, window, plugins, --vault resolution
  src/commands.rs                 one #[tauri::command] per §5 route; marshal, call, envelope — nothing computes
  src/state.rs                    ConsoleState: vault path, seen_at, one Mutex serialising vault operations
  static/index.html console.css console.js fonts/     the page — no bundler; withGlobalTauri
  tests/                          command-level tests on scratch vault copies
```

### 3.1 The boundary the compiler enforces

- **The engine crate never depends on `tauri`** (rewrite spec, waves 0–3 global constraints). The
  console depends on the engine, never the reverse. S2 needed an AST-walking test for this; Cargo
  makes it structural.
- **`commands.rs` is thin by construction.** It may call `quinn_ops::surface`, `write`,
  `approvals`, `info`, `issues`, `sync`; it marshals arguments, calls, wraps the envelope. Arithmetic
  in a command handler is a review rejection, for the reason S2 gave: this project's test culture
  is on pure engine functions.

### 3.2 Process model

Tauri's main thread owns the window. Commands run on Tauri's worker pool and take the single
`Mutex` in `ConsoleState`, so console operations on the vault are serialised. Engine calls are
milliseconds of file I/O; nothing here needs a background thread.

### 3.3 Vault resolution — no single-user assumption

`--vault <path>`; else the working directory if it holds `config/planning.yaml`; else an error
dialog naming both, and exit. Quinn's shortcut passes `--vault`. The profiles milestone replaces
this with a picker and a per-user settings file. Nothing in the shell hardcodes a path, a machine
name or a user name.

### 3.4 Plugins

Two, both official: `tauri-plugin-single-instance` (a second launch focuses the first) and
`tauri-plugin-window-state` (geometry, in the app-data dir — §1's deviation). No others.

### 3.5 Build coupling on Quinn's laptop

Two binaries embed the engine. After a pull the runner's build step (`scripts/local-run.ps1`,
cutover plan Task 5) builds **both** crates; a console that is open holds its exe locked, which is
`BUILD-WARN` and next time, never a failed run. A friend's installer ships both binaries from one
commit, so skew cannot arise there.

---

## 4. The read model — `src/surface.rs`

```rust
pub fn build_state(vault: &Path, view: View, today: Date, now: DateTime, seen_at: Option<&str>) -> State
```

Composed of S2 §7's builders, each independently tested: `topline`, `delta`, `verdict`, `meter`,
`must_do`, `ranked_list(horizon)`, `the_day`, `decisions`, `ahead`, `good_to_know`,
`closed_this_week`, `runs_panel`, `issues_panel`, `coming_up`, `note_detail(id)`, `nav_counts`,
`gauge`, `is_judged`. What each computes is S2 §7.1–7.8, unchanged.

### 4.1 Explicit by construction

Each region is a hand-written `#[derive(Serialize)]` struct. No `Task`, `Approval` or note mapping
is ever serialised directly, so a field added to the model cannot appear on the page unnoticed —
S2's "never a blanket `asdict`" rule, in the type system.

### 4.2 Shared logic has one implementation

The status word, the capacity-breakdown string and `due_move`'s phrasing are computed inline in
`render.rs` today. They become `pub fn`s there and `surface` calls them. S2 pointed the dependency
the other way; this direction touches `render.rs`'s visibility only, and `tests/oracle.rs` proves
`today.md` did not move a byte. The page and `today.md` cannot disagree about the same number.

### 4.3 "Why" — derivation traceability (S2 §7.5)

`scheduling::start_by` and `ranking::designate_today` discard their intermediate values at return.
New *explained* variants return them (`start_by_explained` → the hours, the session cap, the working
days, the buffer; the allocation → which free block absorbed it and what was left). The originals
are untouched; a test asserts each explained variant agrees with its original on every fixture task.

### 4.4 Rules

- **Never writes.** A test byte-compares the vault before and after `build_state` on every fixture.
- **Reads only the engine's own inputs** — notes, `config/`, `state/calendar.md`, `state/events.md`,
  `state/runs/`, the journal, `profile/`. It never reads `today.md`.
- **Deterministic on its arguments.** `today`, `now` and `seen_at` are passed in; the shell reads
  the clock and the per-device seen file; the builders never do.
- **Unreadable notes are rendered, not dropped** — a row `unreadable: <path>`, critical-coloured,
  in every list that would have held the note (S2 §13).

### 4.5 Two builders wait for the cutover to close

`gauge` needs three counts on the run record (`runway_days`, `deficit_hours`, `must_count` — S2
§7.7) and RUNS needs coursework to write run records (S2 §15's folded-in fix). Both change what the
runner writes, so both land **after the reverse dual-run week closes (~2026-09-18)**. Until then
`gauge` renders its designed "no history yet" state and coursework appears in RUNS as it does today
— absent, and said so.

### 4.6 The `surface` subcommand and the frozen references

`quinn-ops surface --vault <v> --view <view> [--today D] [--now T]` prints the `State` as pretty
JSON. It is B's one good idea folded into A: the read model as a thing a terminal or a Claude
session can look at, and the read model's acceptance harness.

`surface --view today` on each of the three fixture vaults at the pinned date, frozen as
`tests/fixtures/surface-today-{s1,s1-migrated,full}.json` and byte-compared by a test. **There is
no Python oracle for this module** — S2 was never built — so these are the first correct output,
and the rule differs from the eight Python-written references: **regenerating is allowed, but only
in a commit whose diff shows the change and whose message says why.** A regenerated reference with
no reviewed diff is the mockup's `content: "8.7h"` all over again.

---

## 5. IPC — the commands

One `#[tauri::command]` per S2 §8 route, same names:

| Command | Calls | Returns |
|---|---|---|
| `state(view)` | `surface::build_state` | `State` |
| `note(id)` | `surface::note_detail` | detail |
| `create_task(fields)` | `write::create` | envelope |
| `set_fields(id, fields)` | `write::write` | envelope |
| `delete_note(id)` | `write::delete` → `archive/` | envelope |
| `decide(id, verdict, note, snooze_until?)` | `write::write_literals` then `approvals::process_approvals` (§7.3) | envelope |
| `close_info(id)` | `info::close_info` | envelope |
| `open_issue(target, categories, text)` | `issues::open_issue` | envelope |
| `resolve_issue(id)` | `issues::address_issue` | envelope |
| `sync()` | `sync::sync` | envelope + `sync_status` |
| `mark_seen()` | writes the per-device seen stamp | envelope |

Envelope `{ok: bool, error?: string, state: State}`. **Every mutating command returns the freshly
rebuilt state** — one call, the whole re-ranked view back, swapped in; no optimistic local state
to drift, and the visible cue is that the row moved. A refused write still returns current state,
so the page never shows a stale value beside an error.

`unbind` (S3 §3.5b) is not a command; it sits in the parity test's deferral set with its reason.

**Write context, always:** `WriteContext { actor: "quinn", via: "dashboard", run_id: None }` —
S1 reserved `dashboard`. Console writes are not a run and write no run record.

---

## 6. The page — `app/static/`

`index.html`, `console.css`, `console.js`, `fonts/`. The mockup's class names, tokens and layout
grammar verbatim (S2 §9, §11): mock data removed; `.kb` deleted, not hidden; the `p.foot`
annotation deleted; the CSS `content: "8.7h"` replaced by a real element fed from `ahead.peak`;
S2 §6's regions added — start-by, slack and conflicts on rows, the delta line, COMING UP, the
all-day lane, the overflow sentence with both figures, runway, active count and **runner name** in
the topline.

- **`console.js` is one file of plain functions**, one `render<Region>(state)` per region, calling
  `window.__TAURI__.core.invoke` — `withGlobalTauri: true` in `tauri.conf.json`, so there is no npm,
  no bundler, no generated code. The file you read is the file that ships.
- **Tokens at the top of `console.css`**, all thirty from the mockup, commented with what each is for.
- **Fonts vendored** — Instrument Sans 400/500/600/700 and JetBrains Mono 400/500/600, woff2, SIL
  OFL, in `static/fonts/`. Embedded in the exe by `frontendDist`, so offline is by construction.
- **CSP `default-src 'self'`**, and a test that no `http://` or `https://` appears in any static file.
- **Views:** the nav's counts come from `nav_counts` and its links are real. Overdue / This week /
  Later / All active are the same row component over `ranked_list(horizon)` and ship with the first
  plan. Decisions, Good to know, Issues and Runs are the third plan; until then their nav entries
  show the live count and a designed "not built yet" column — never a dead link, never a hidden
  entry.
- **`docs/surface/anatomy.md`** is updated per region as each lands (S2 §11 item 4).

---

## 7. Writes and the deck

### 7.1 Edits

Click a field → input; blur or Enter commits, Escape cancels. The editable set is S2 §9.2's:
`title`, `course`, `due`, `effort_hours`, `importance`, `importance_reason`, `status`, `progress`,
`slice_hours`, `domain`, `rank_override`, `effort_confidence`. `id`, `source_uid`, `also_uids` and
`judgment` are shown in the drawer and never editable. Progress drags in 5 % steps; reaching 100
sets `status: done` in the **same** write — one journal moment, one line in CLOSED THIS WEEK.
Delete is behind a confirm and goes to `archive/`; nothing is unlinked. New task: title (required),
course (datalist from `courses/`), due (defaults 23:59), effort (default 1.0); `importance` defaults
to 3 and is not asked for.

**Consequence, stated once:** every console edit is an `actor: quinn` record, so the console is the
primary input to judge-once. Editing an effort estimate freezes it against the routine; the
routine's future opinion arrives as an `amend` card in the deck. That loop is the design.

### 7.2 Refusals

A refused write — judge-once, an unamendable field, a bad value — returns `ok: false` and the
current state; the field snaps back and the reason renders inline. Nothing is retried silently.

### 7.3 The deck, and the one deviation from S2

A decision is `status: approved | rejected | snoozed` (plus `snooze_until`, plus the note field —
**`decision_note`**, a key no engine reader consults today, so the cloud routine's Python ignores it
and nothing in `today.md` changes) written onto the approval note through `write`. Today that is where it stops until a runner's
`process_approvals` pass executes it — materialises the task, expands the digest, applies the
amend. **`decide` runs `process_approvals` in-process in the same call**, with the console's
context, so an approved task is in the ranked list when the envelope comes back, not six hours
later. Calendar-event approvals are unaffected: their executor is the cloud routine, and they show
as "awaiting calendar pass" exactly as `today.md` does. The runner's own pass then finds nothing to
do — the status machine is idempotent by construction.

Deck shape and semantics are S2 §9.5: one card, two slivers, `N behind`, fixed height whatever
the queue length; note field always visible; one click commits, no confirm relabel; ordering by
`first_proposed_at`; header with pending count, oldest age, budget spent/remaining, deferred,
calendar backlog, digest count; snooze in the overflow; the empty state says so.

### 7.4 Issue flags and info

⚑ on every `is_judged` object → eight category chips **and** free text, both always →
`issues::open_issue`, which snapshots the object and its judgment. Nothing acts on it. GOOD TO
KNOW's close action → `info::close_info`.

### 7.5 Interaction events (S2 §7.6)

Through the same `ledger` seam as the journal, to `state/events-ui/YYYY-MM-DD.jsonl`, ids only,
never read by the engine, git-ignored unless `config/planning.yaml` opts in. Actions:
`view_opened`, `object_seen` (≥ 2 s in viewport), `edit_started`, `edit_committed`, `edit_cancelled`,
`decision_made`, `decision_deferred`, `issue_opened`, `sync_run`, `delta_expanded`, `why_expanded`.
`object_seen` is the one that cannot be backfilled, which is why this ships in the writes plan and
not later.

---

## 8. Sync — `src/sync.rs` (S2 §10, as designed)

```rust
pub fn status(vault: &Path) -> SyncStatus        // synced_at, unsynced, ahead, behind, dirty, last_error
pub fn sync(vault: &Path, ctx: &WriteContext, journal: &mut Journal) -> SyncStatus
```

Git through `std::process::Command`; the console is the only caller.

1. `git add` the note folders and `state/` **by name, never `-A`** — Obsidian Git shares this tree.
2. Commit if staged: `surface: <n> edits`.
3. `git rev-list --left-right --count main...origin/main` first. **Only ahead → push, never
   rebase**: rebase flattens merge commits and replays them as conflicts (CLAUDE.md).
4. Otherwise `pull --rebase`, then push.
5. On conflict, per note: upstream wins the file; this device's un-pushed records go through
   `reconcile::resolve` (ported in wave 3, zero callers until now); winners re-applied through
   `write`; `supersede` records appended; `git add`; continue. Ledger files never reach this path —
   `merge=union`.
6. Offline or rejected: the commit stands, `last_error` set, topline reads `N edits pending push`,
   the loop retries. **A failed sync is an amber topline.** A conflict `resolve` cannot settle leaves
   the note conflicted, names it in `last_error`, and **stops auto-sync until cleared**. Never
   auto-resolve by discarding.

Advisory lock `state/.sync.lock` (pid + ts, stale after 120 s), held for the duration of a console
write. **The runner deliberately does not take it** (S2 §8.1): a lock on a scheduled task that can
die mid-run trades a rare race for a permanent wedge.

Triggers: pull on open · push debounced 30 s after the last write · pull every 5 min while focused ·
push on close, synchronous, 10 s cap · manual `sync now`.

One fact new since S2: Obsidian Git may commit the console's edits before the console does. Harmless
— the console's commit then finds nothing staged, and the push proceeds.

---

## 9. Freshness and concurrency

- `state(view)` on load, on focus, every 60 s. `revision` is a hash over the payload minus
  `generated_at`; identical means no repaint. **A poll never reorders under the cursor**: if the
  ranking changed, the delta line says so and offers `refresh order`. The page is a plan, not a feed.
- The console never reads or writes `today.md` or a run record. A runner rewriting the page
  mid-session is irrelevant; `runner-log.md` remains a record of runs and nothing else.
- Runner vs console: single-line surgery makes concurrent different-field writes safe; the one real
  race is a console write during the runner's `pull --rebase`, which §8's lock covers on the
  console's side.
- No file watching. Polling suffices; `notify` would be a dependency for a nicety.

---

## 10. Engine changes and their sequencing against the cutover

The engine is under dual-run measurement until the reverse week closes (~2026-09-18). The rule
recorded in HANDOFF on 2026-09-02: **new read-only modules and visibility changes may land in
`src/` during the weeks; nothing that changes what `rank` or `write` writes.**

| Change | Kind | When |
|---|---|---|
| `src/surface.rs`, the `surface` subcommand, the frozen references | read-only | now |
| `pub` on three `render.rs` helpers; explained variants in `scheduling`/`ranking` | visibility / additive | now |
| `src/sync.rs` | new module; only the console calls it | now |
| Gauge counts on the run record (`cli::run`'s tasks step) | changes what `rank` writes | after ~09-18 |
| Coursework run records (`start_run`/`add_step`/`end_run` in `coursework`) | changes what the runner writes | after ~09-18 |

Throughout: `tests/oracle.rs` green (**S2 must not move `today.md` by a byte**); both dual-run
scripts clean after every engine change; `cargo test` at 0 warnings; **`engine/` untouched and
Python at 687** — the console is Rust-only, the read model has no Python twin, and the cloud routine
never needs one.

---

## 11. Testing

- **Builders on all three fixture vaults** (S2 §14): `ahead` puts overdue hours on today, reports
  `undated_hours`, breaks peak ties earliest; `meter` arithmetic including the 120-block cap;
  `closed_this_week` marks me/agent/lapsed from the actors that exist live (`quinn`,
  `agent:coursework.vhl`, `agent:approvals`, `system:*`); `delta` groups and renders a `supersede`
  and a `via: external`; `is_judged` is false for `{migrated: true}`; every explained variant agrees
  with its original on every fixture task.
- **Frozen `surface-today-*.json`** on the three fixtures at the pinned date, byte-compared (§4.6).
- **`build_state` writes nothing** — vault bytes identical before and after, every fixture.
- **The parity test** — S2 §14.1, the completeness rule enforced: the union of task ids across the
  horizon views equals the full ranked set; a table maps every S2 §6.1 row to the payload key that
  carries it and a missing key fails by name; every `engine` CLI operation has a command or sits in
  an explicit deferral set with its S2 §6.3 reason.
- **Commands, in `app/`**, on scratch copies of `vault-s1-migrated`: `set_fields` writes exactly one
  journal record and returns rebuilt state; a judge-once refusal returns `ok: false` plus state;
  `decide` on a task proposal leaves a materialised task note and an archived approval in the same
  call; `create_task` mints an id; a read-only vault makes every mutating command return
  `ok: false` with the reason.
- **Sync, with temporary git repos**: ahead-only pushes without rebasing; a same-field two-device
  conflict resolves later-wins and appends one `supersede`; ledger files never enter the conflict
  path; offline leaves the commit and sets `last_error`; a merge commit in local history is pushed,
  not rebased.
- **The page, statically**: no `http` in any static file; `.kb` absent; every region selector
  present; the `content:` peak label gone; every token from the mockup's `:root` present.
- **The page, visually** — S2 §11.12's rule: `scripts/mockup-shots.py` at eight viewports before any
  layout is called done, plus the spike's launch-screenshot-close smoke against the real exe.
- **Existing guards stay green throughout:** `tests/oracle.rs`, `diff-engines.ps1` on three
  fixtures, `diff-engines-notes.ps1`, Python 687, 0 warnings.

---

## 12. Failure modes

| Failure | Behaviour |
|---|---|
| WebView2 missing | Tauri's own dialog. The installer's bootstrapper (profiles milestone) prevents it on friends' machines |
| A note fails to parse | Rendered in place as `unreadable: <path>`, critical-coloured; the rest renders. **Never a silently shorter list** |
| `write` refuses | `ok: false` + current state; field snaps back; reason inline |
| Death between journal append and note write | `verify_tail` re-applies at the next run; the console shows the journal's value, which is the one that will win |
| Sync conflict `resolve` cannot settle | Note left conflicted, named in `last_error`, topline amber, **no further auto-sync until cleared** |
| Offline | Writes commit locally; `N edits pending push`; loop retries |
| No run in 24 h | RUNS says so **in words** from `expected_status` — today an absent line |
| Two consoles | The single-instance plugin focuses the first |
| The path is not a vault | Error dialog naming `--vault` and the working directory; no guessing |
| Clock skew | S1 WARNs; the delta line shows the `supersede`, so a wrong winner is visible |
| A runner rewrites `today.md` mid-session | Irrelevant — the console never reads it |

---

## 13. Deferred, with reasons

- **S3's unbind and duplicate resolution** — CLI-complete in S3; one click each here afterwards.
- **The learning loop on `rank_override`** — editable, nothing learns; probably unnecessary (S2 §9.8).
- **Grades** — no region in this layout; S5 decides its home.
- **The gauge's history and coursework run records** — §4.5; after ~09-18.
- **Profiles, onboarding, the installer, the WebView2 bootstrapper, an OS-keychain seam** — the next
  milestone; friends are all on Windows, and nothing here forecloses any of it.
- **File watching** — §9.
- **macOS** — later port; the shell is cross-platform by construction, the credential read is not.

---

## 14. Decisions this document makes

| # | Decision | Source |
|---|---|---|
| 1 | The console is a **translation of S2**; where this document is silent, S2 stands | this doc |
| 2 | **First slice is Today + click-CRUD** | Quinn 2026-09-02 |
| 3 | **Views in one window**, rails permanent, main column swaps (S2 §16.1 confirmed) | Quinn 2026-09-02 |
| 4 | **Windows only** this milestone; friends are all on Windows | Quinn 2026-09-02 |
| 5 | **Sync as S2 §10 designed it**, in this milestone | Quinn 2026-09-02 |
| 6 | **Engine boundary: in-process library (A), plus the read model as a CLI (B folded in)**; the loopback server and the one-binary option rejected | Quinn 2026-09-02 |
| 7 | Two crates, two binaries, one commit; the engine never depends on `tauri` | this doc, §3 |
| 8 | `commands.rs` computes nothing | this doc, §3.1 |
| 9 | Vault from `--vault` or a working directory that holds `config/planning.yaml`; no hardcoded path | this doc, §3.3 |
| 10 | Window state in the app-data dir via the official plugin — **deviates from S2** (`state/.window.json`) | this doc, §3.4 |
| 11 | `surface` calls `pub` helpers in `render` — **reverses S2's import direction**; same guarantee | this doc, §4.2 |
| 12 | Explained variants beside the originals, never in place of them | this doc, §4.3 |
| 13 | `surface-today-*.json` references are Rust-generated; regenerate only with a reviewed diff | this doc, §4.6 |
| 14 | **`decide` executes the decision in-process, in the same call** — **deviates from S2's silence** | Quinn 2026-09-02, §7.3 |
| 15 | Gauge counts and coursework run records wait for the cutover to close | this doc, §4.5 |
| 16 | Delta "since I last looked"; no launch at login; PH 106 line under AHEAD; runway strip parked | Quinn 2026-09-02, §2 |
| 17 | Interaction events ship in the writes plan, `object_seen` included | this doc, §7.5 |
| 18 | Three plans: foundation → writes → views; the first two are the first slice | Quinn 2026-09-02, §16 |
| 19 | The runner rebuilds both crates; the topline says `engine newer than console` on a SHA mismatch | this doc, §15 |
| 20 | The console does not run the passes | this doc, §15 |
| 21 | `State.schema = 1` | this doc, §15 |

---

## 15. Open questions — resolved 2026-09-02 (Quinn: "I trust your judgement")

1. **Who rebuilds the console after a pull?** **The runner**, in the same build step that rebuilds
   the engine (cutover plan Task 5), and the topline shows **`engine newer than console`** when the
   two binaries' embedded git SHAs differ — computed from a `--version`-style constant each binary
   carries, so a stale console is visible rather than discovered. The console never builds itself:
   its start stays instant, and a build inside a GUI process is a failure mode nobody wants to debug.
2. **Does the console run the passes** (`verify_tail`, `detect_external`, `ensure_ids`) on open?
   **No** — S2's answer stands; the runners own the passes. Revisit with profiles, where a friend's
   install may have no runner for its first day.
3. **`State` carries `schema: 1`** from the first commit. One line now; a future client that reads
   the same JSON (plan §6.2) gets a version to check instead of a guess.

---

## 16. Hand-offs — three plans, in order

1. **Console foundation** — `surface.rs`, the `surface` subcommand, the three frozen references,
   the `pub` helpers and explained variants, `app/` with `--vault`, both plugins, the page at real
   data: Today + the four horizon lists + the drawer, **read-only**. Starts now; touches the engine
   read-only.
   *Plan 1 done 2026-09-03 (branch worktree-console-plan-1; engine half merged as f8c36d2).*
2. **Writes** — every §5 command, the deck with in-process execution, `sync.rs` and the loop,
   interaction events. *The end of this plan is the first slice.*
3. **The other views** — Decisions, Good to know, Issues, Runs; the gauge and coursework run
   records once §4.5's gate has passed.

Then the profiles + onboarding + installer milestone, which is what puts this in a friend's hands.
