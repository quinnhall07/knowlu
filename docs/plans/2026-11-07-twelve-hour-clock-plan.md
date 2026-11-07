# A 12-hour clock on every surface that shows a time: plan

**Status: DRAFT r1, for review. Nothing dispatched.** Three questions for Quinn are in §6. Q1 decides
whether `today.md` is in scope at all.

**Date:** 2026-11-07. **Branch:** `docs/twelve-hour-clock` (from `feat/o3-long-onboarding`).
**Source:** Quinn, 2026-11-04, replan draft §9 (`docs/plans/2026-11-04-onboarding-replan-draft.md`):
"A 12-hour clock for class times ... across every surface that shows a time". On 2026-11-07 Quinn said
to start it. Wave B's C6 leaves the format alone, and this lane changes nothing wave B owns (§5, merge order).

**Authority.** The cloud design, then `CLAUDE.md`'s two overriding rules, then the commitment-model
spec (`meets` is `"HH:MM"` 24-hour in frontmatter, §2 of that spec's table). This plan changes
**display only**. No frontmatter, journal, ledger, run-record or series-file byte changes format.

---

## 1. The format

This follows `eventemit::clock` and `eventemit::when_label` (`engine/src/eventemit.rs:508`, `:557`),
which are already live in card titles and commitment labels:

- A 12-hour clock with minutes only when they are non-zero: `10am`, `10:30am`, `7pm`, `12pm` (noon),
  `12am` (midnight), `11:59pm`.
- In a range, the end always carries am/pm. The start carries it only when it falls on the other
  side of noon: `10am–3pm`, `7–9pm`, `10:30–11:15am`, `11am–12pm`, `12–1pm`. The dash is U+2013.
- A time on its own always carries am/pm.
- A date-and-time stamp keeps its date exactly as shown today and changes only the clock:
  `2026-09-01 20:55` becomes `2026-09-01 8:55pm`. A friendlier date is out of scope.
- The existing words stay: `eventaccept::ends_label`'s `midnight` and `1am the next day`.

**One formatter per layer.**
- **Engine:** a new pure module, `engine/src/clockfmt.rs`, with `clock(Time, meridiem: bool)` and
  `range(Time, Time)`. `eventemit::clock` moves there. `eventemit::when_label`,
  `commitments::range_label` and `eventaccept` call it, so the split-by-noon rule is written once,
  not twice as it is today (`eventemit.rs:547`, `commitments.rs:2999`). The engine formats only the
  strings it composes, such as `when` labels, card titles and note bodies.
- **Console:** one set of helpers in `app/static/console.js`, next to today's `localHM` (`:455`), which
  they replace: `clock12(hhmm)`, `clockRange(a, b)`, `timeOf(stamp)` and `stampOf(stamp)`. The page
  formats every raw time field the read model sends as data: block `start`/`end`, `due`, run and
  journal stamps and `meets`. `onboarding.js` shows only engine-made `when` strings and native
  `<input type="time">` controls, so it needs no formatter (§2.2).
- **Data stays data.** The read model's raw fields stay `"HH:MM"` and ISO (`blocks[].start`,
  `due`, `now`, `meets`). The page computes nothing from a display string, and all three Rust surface
  references pin those fields.

Determinism holds. `clockfmt` is pure and reads no clock, zone or locale. The page converts a zoned
stamp (`...Z`) to the machine's zone for display, as `localHM` does today. That happens only in the page.

---

## 2. Inventory: every place a clock time reaches the student

Legend: **12h** means already in the §1 format, **24h** means `HH:MM`, and **amb** means ambiguous (a
raw value shown as stored). "Frozen" means a Python reference pins it (`CLAUDE.md` rule 2).

### 2.1 Engine (what it writes or emits)

| Site | Reaches the student as | Now | Plan |
|---|---|---|---|
| `eventemit.rs:509` `clock`, `:527` `when_label`, `:561` `what_and_when` | event card titles | 12h | moves to `clockfmt` (T1), bytes unchanged |
| `commitments.rs:2998` `range_label`, `:3008` `meets_label` (`:3043` titles, `:3109` proposal `when`, `:5295` week `when`) | commitment cards, proposals, Schedule, onboarding step 10 | 12h | calls `clockfmt::range` (T1), bytes unchanged |
| `eventaccept.rs:64` `ends_label`, `:272` Register task body | accepted-event note body, Register task | 12h | calls `clockfmt::clock` (T1), bytes unchanged |
| `surface.rs:1244` `coming_up[].when` (`Wed 9/2 16:00`) | console Coming up | 24h | **changes** to `Wed 9/2 4pm` (T2). Pinned in `surface-today-full.json` (T3) |
| `eventemit.rs:296-302` events-digest line (`- [ ] Wed 9/2 16:00 · …`) | `approvals/events-digest-*.md` body (default: `event_cards` off) | 24h | **changes** for new digests only (T2). No frozen file pins it, and `approvals::digest_uids` reads only the last backticked token (`approvals.rs:4051`) |
| `surface.rs:979` `hm` → `the_day.blocks[].start/end` | console The day | 24h data | stays; the console formats it (T5). Pinned in all three surface references |
| `surface.rs:414` `local_stamp` → `topline.generated_at`, `closed[].when`, `runs_panel.recent[].started/ended` | console status line, Closed, Runs | 24h data | stays; the console formats it (T5). Its doc ties it to today.md's footer |
| `surface.rs:693` `due`, `:1245` `coming_up[].start`, `:2177` `now` | data only | ISO | stays |
| `render.rs:432-435` Schedule `09:00–10:00`, `:240-245` Coming up `Wed 9/2 16:00`, `:336` footer `2026-09-01 20:55`, `:119-120` `due_move` | `state/today.md` | 24h | **stays, frozen**: `golden-today-full.md` lines 9-10, 36-38 and 40, and `golden-today-s1.md` line 29. `due_move` is not pinned but stays so the page has one format. Q1 |
| `eventroster.rs:116`, `:216` | `state/events.md` roster | 24h | **stays, frozen** (`vault-full/state/events.md`), and `eventroster.rs:270` parses it back |
| `calfeed.rs:882-889` | `state/calendar.md` snapshot | 24h | **stays, frozen** (`calendar-snapshot-gcal.md`), and `calfeed.rs:916` parses it back |
| `commitments.rs:1951` `hm` (`meets` JSON, series file, registrar changes) | data the page reads | 24h data | stays; the commitment spec makes it `HH:MM` |
| `eventaccept.rs:207-211` | commitment note `meets` frontmatter | 24h | stays: frontmatter is a contract |
| `cli.rs:930` `proposal_line` | `commitments` without `--json`, read by people at a terminal | 24h | stays: no student reads the CLI |
| `approvals.rs:1674`, `:4948`, `coursework.rs:1349`, `enrich.rs:1108`, `runs.rs:550`, `journal.rs:65` | frontmatter, the run log and the journal | stamps | stay: contracts |

### 2.2 App (what the page and the shell draw)

| Site | What the student sees | Now | Plan (T5) |
|---|---|---|---|
| `console.js:272` The day blocks `b.start–b.end` | `08:00–12:00` | 24h | `clockRange` → `8am–12pm` |
| `console.js:369` Coming up `e.when` | engine string | 24h now | 12h by T2; no page change |
| `console.js:92` `generated … by …` | `2026-09-01 20:55` | 24h | `stampOf` |
| `console.js:151` task due chip text | `2026-08-26 23:59` | 24h | `stampOf` on the label only. The chip's edit value stays `r.due` |
| `console.js:396` Closed `x.when.slice(11)` | `20:55` | 24h | `timeOf` |
| `console.js:455` `localHM`, used at `:422`, `:474`, `:588`, `:610` | `checked 14:05`, `synced 14:05` | 24h, local | replaced by `timeOf` |
| `console.js:622` sync line `last slot` | `last_slot.ended.slice(11, 16)` | 24h **and UTC**: `ended` is `journal::now_ts` (`app/src/scheduler.rs:1334`, `...Z`) | `timeOf` converts to local time (Q3) |
| `console.js:639`, `:643` Runs view; `:1286-1287` runs list | `15:00`, `2026-08-27 15:00` | 24h | `timeOf` / `stampOf` |
| `console.js:674-675` amend card `changes` | `due → {"from":"2026-09-25T09:00",…}` | amb | a `due` change renders `stampOf(from) → stampOf(to)`. Other keys stay as today |
| `console.js:1057` `regMeets` (registrar changes) | `Mon/Wed 13:00–14:15` | 24h | `clockRange` |
| `console.js:2803-2808` mail applied `due`, `opened_at`, a `due` field's `old`/`new` | raw vault value | amb | `stampOf` for those fields only |
| `console.js:898`, `:904`, `:1042`, `:1126`, `:1170`; `onboarding.js:646`, `:704` | engine `when` | 12h | no change |
| `console.js:817-818`; `onboarding.js:677`, `:787` `<input type="time">` | native control (follows the Windows locale; value is `HH:MM`) | OS | no change |
| `onboarding.js:248-275` week strip | days and counts, no clock | n/a | no change |
| `app/src/tray.rs:42` tooltip `Knowlu` | no time | n/a | no change. The app sends no OS notifications (no notification code in `app/src`) |
| `app/src/tray.rs:184-198` diagnostics copy | raw stamps for support | 24h | out of scope: a support artefact, not a display |

Cloud judgment text, such as a model's `why`, is not formatted by us and is out of scope.

---

## 3. What may change, and what may not

- **May change:** `surface --view today`'s `coming_up[].when`; the body of a newly filed events
  digest; everything the console draws in §2.2.
- **May not change:** `state/today.md` (unless Quinn rules otherwise on Q1), `state/events.md`,
  `state/calendar.md`, `state/calendar-series.json`, any frontmatter or journal record, run records,
  the run log, `commitments` CLI text, and the bytes of every 12h string in §2.1 (T1 is a pure move).
- **Pinned by:** `engine/tests/oracle.rs` (both goldens), `eventroster.rs`'s and `calfeed.rs`'s
  reference tests, `engine/tests/surface_oracle.rs`. All stay green with no edit, except
  `surface-today-full.json`, which T3 regenerates.

---

## 4. Tasks

Order: T1 ∥ T4, then T2 → T3, T5 after T4, T6 last. The file sets are disjoint except T1/T2
(both `eventemit.rs`, run in sequence). Extract each brief by its heading.

### T1. Engine: one clock formatter (`implementer`)

**Why this agent:** fully specified, off the contract list, and the compiler plus existing tests
check it. **Files:** `engine/src/clockfmt.rs` (new), `engine/src/lib.rs` (one `pub mod`),
`engine/src/eventemit.rs`, `engine/src/commitments.rs`, `engine/src/eventaccept.rs`.

- Tests first, in `clockfmt.rs`: `clock` at 00:00 `12am`, 00:30 `12:30am`, 09:00 `9am`, 12:00 `12pm`,
  12:50 `12:50pm`, 13:05 `1:05pm`, 23:59 `11:59pm`; `clock(_, false)` drops the suffix; `range` on
  `10am–3pm`, `7–9pm`, `10:30–11:15am`, `11am–12pm`, `12–1pm`, `11:30am–12:15pm`, `8am–10pm`.
- Move `eventemit::clock` into `clockfmt::clock`. Add `clockfmt::range` with the split-by-noon rule.
  `eventemit::when_label` and `commitments::range_label` call `range`, and `eventaccept` calls `clock`.
  No `eventemit::clock` re-export is left behind.
- **Done when:** every existing assertion passes unedited, including `commitments.rs:9381-9383`,
  `:10829`, `:11117`, `eventaccept.rs:391`, `:398`, the `what_and_when` tests and every oracle.
  `cargo build --workspace` and `cargo test --workspace` pass with 0 warnings.

### T2. Engine: the two 24-hour display strings (`implementer`, after T1)

**Why this agent:** two specified call sites and their tests. **Files:** `engine/src/surface.rs`,
`engine/src/eventemit.rs`. Never `render.rs`.

- Tests first. Change `surface.rs:2775` to `["Wed 9/2 4pm", "Fri 9/4 12pm", "Thu 9/10 6pm"]`, and
  update the `when` literals at `:4016` and `:4357`. Add an `eventemit.rs` test: a digest emitted for
  events at 16:00 and 10:30 carries `Wed 9/2 4pm ·` and `… 10:30am ·`. In the same test, feed the
  emitted body to `approvals`' digest parser and get the uids back. It calls the parser and does not
  edit `approvals.rs`, which is on the contract list.
- `surface::coming_up`'s `when` uses `clockfmt::clock(t, true)` instead of `hm`. The digest line
  uses the same call instead of `strftime("%H:%M")`. `surface::hm` stays for the blocks.
- **Expected red:** `surface_today_full_is_frozen` only. s1 and s1-migrated must stay green, and
  `oracle.rs` must stay green. That proves today.md's own Coming up did not move. T3 makes the tree
  green, and T2 and T3 go in one push.

### T3. Regenerate `surface-today-full.json` (`contract-engineer`; `contract-reviewer` reads it)

**Why this agent:** `engine/tests/fixtures/**` is on the contract list, and console spec §4.6 governs
a read-model reference. **File:** `engine/tests/fixtures/surface-today-full.json` only.

- Regenerate it with the Git Bash command in `engine/tests/surface_oracle.rs`'s header: one JSON line
  plus CRLF, UTF-8, no BOM.
- **Its own commit**, whose diff shows exactly three substitutions: `Wed 9/2 16:00` → `Wed 9/2 4pm`,
  `Fri 9/4 12:00` → `Fri 9/4 12pm`, `Thu 9/10 18:00` → `Thu 9/10 6pm`. Check it by applying those three
  substitutions to the old bytes and comparing the result with the new file. The message says why:
  "surface: coming_up.when on a 12-hour clock (twelve-hour-clock plan T2); console spec §4.6".
- `git diff --stat -- engine/tests/fixtures` names one file. `surface-today-s1.json`,
  `surface-today-s1-migrated.json` and the eight Python references are byte-unchanged.

### T4. Console formatter: the failing tests (`test-writer`)

**Why this agent:** this is the failing test, written before the page changes. **File:**
`app/tests/console_clock.rs` (new).

- Lift `clock12`, `clockRange`, `timeOf` and `stampOf` out of `console.js` under node, with the
  harness pattern `main_handover_html` uses (`app/tests/onboarding_static.rs:634`).
- Tables: `clock12` and `clockRange` match T1's cases string for string. `stampOf` gives
  `"2026-09-01 20:55"` → `2026-09-01 8:55pm` and `"2026-08-26T23:59"` → `2026-08-26 11:59pm`.
  `timeOf("2026-09-01 20:55")` → `8:55pm`. A zoned stamp (`2026-09-01T14:05:00.000Z`) converts to
  local time: run node with `TZ=America/Chicago` and expect `9:05am`. If node on the runner ignores
  `TZ`, assert against the harness's own `new Date(iso)` hours instead. Empty or unparseable input
  gives `""` from `timeOf` and the input unchanged from `stampOf`.
- Static pins, on `console.js` with its strings and comments stripped as `static_assets.rs` does: no
  `.slice(11` remains, `getHours(` appears only inside the formatter, the day-block template calls
  `clockRange`, and `regMeets` calls `clockRange`.
- Every test fails before T5 because the functions are missing.

### T5. Console: route every time through the formatter (`console-ui`, after T4)

**Why this agent:** this is `app/static` work, and T4's tests check it. **File:**
`app/static/console.js` only.

- Add the four helpers in place of `localHM` and implement §1's rule. A string with `Z` or an offset
  goes through `new Date` and is shown in local time. A naive `YYYY-MM-DD[T ]HH:MM[:SS]` is read as
  written and never shifted. The page does no other arithmetic.
- Change every **24h** and **amb** row of §2.2, and nothing else. The due chip's edit value, the
  window editor's `<input type="time">` values and the `windowSequence` payload (`:838`) stay `HH:MM`
  and ISO.
- **Done when** T4 passes, `static_assets.rs` and `onboarding_static.rs` stay green, and fixture
  screenshots exist of The day, Coming up, the sync line, Closed, Runs and Schedule's registrar
  changes. Take them by `?fixture=` and headless Edge, never with synthetic input.

### T6. Docs (`docs-keeper`, last)

**Why this agent:** reference upkeep. **Files:** `docs/surface/anatomy.md` (Coming up's `when` format;
`the_day.blocks` times are data that the console formats), `docs/reference/engine-commands.md` if it
quotes a surface time, this plan's ledger status, and one `HANDOFF.md` line at the milestone.

---

## 5. Merge order and review checkpoints

- Wave B's C6, C7 and C10 edit `app/static` and `onboarding_static.rs`. This lane edits `console.js`
  and a new test file, never `onboarding.js`. Dispatch T5 on the head that T4 was written against.
  If wave B lands first, the `integrator` rebases by function name, never by line number.
- **Checkpoint 1** (after T1 to T3): `reviewer` reads T1 and T2, and `contract-reviewer` reads T3's
  diff. The report goes in `docs/reports/`.
- **Checkpoint 2** (after T5): `reviewer` reads T4 and T5. **Quinn** looks at the screenshots: the
  format on the real page is Quinn's call.
- Nothing is pushed before its checkpoint passes, and every PR waits for green CI and a reviewer PASS.

---

## 6. Questions for Quinn

1. **today.md stays on a 24-hour clock?** `golden-today-full.md` pins `09:00–10:00`, `Wed 9/2 16:00`
   and the footer `20:55`, and `golden-today-s1.md` pins its footer. The rule is that the engine
   follows them. **Recommendation: yes, keep it.** Students read the console, Obsidian is gone, and nothing
   in `app/static` shows or links today.md. Moving it would mean retiring a frozen Python reference, which
   only you can rule. The roster and calendar snapshot stay 24-hour in either case: they are frozen,
   and the engine parses them back.
2. **A fixed 12-hour clock, or follow each student's Windows setting?** The engine cannot read a
   locale and stay deterministic, so following the setting would mean the page and the engine's
   strings disagree on some machines. **Recommendation: fixed, the same for every student.** That is
   not a single-user assumption, but it is a US-locale one. A display setting comes only when a
   non-US school signs up. Native time inputs already follow Windows.
3. **Fix the sync line's UTC time in the same lane?** `last slot 14:05` today shows UTC
   (`journal::now_ts`), not local time. T5's `timeOf` corrects it as a side effect, so the shown time
   moves by the zone offset. **Recommendation: yes.** It is the same line and the same helper.

---

## 7. Fidelity ledger

| Source | Requirement | Task | Evidence |
|---|---|---|---|
| Quinn, 2026-11-04 (replan §9) | A 12-hour clock for class times, on every surface that shows a time | T1 to T5 | §2's inventory; T4's tables; Checkpoint 2's screenshots |
| Quinn, 2026-11-07 | Start it | this plan | — |
| Caller brief | Follow eventemit's rule: minutes only when non-zero, the end always carries am/pm | §1, T1 | `clockfmt` table; existing 12h assertions unedited |
| Caller brief | One formatter per layer | T1, T5 | `clockfmt.rs`; T4's static pins (no `.slice(11`, `getHours(` only in the helpers) |
| `CLAUDE.md` rule 2 | No frozen Python reference regenerated; today.md keeps matching | §3, T2, T3 | `oracle.rs`, roster and snapshot tests green with no edit; T3's diff stat names one file; Q1 |
| Console spec §4.6 | A surface reference is regenerated only in a commit that shows the diff and says why | T3 | the three-substitution check; the commit message |
| `CLAUDE.md`, engine invariants | Frontmatter, journal and ledger bytes never change format; display only | §2.1, T2 | no writer of frontmatter or the journal touched; `meets` stays `HH:MM` |
| `CLAUDE.md` | Determinism | §1, T1 | `clockfmt` is pure and reads no clock, zone or locale |
| `CLAUDE.md` rule 1 | No single-user assumptions | §1, Q2 | one fixed format for every student |
| Replan §9 | Wave B neither fixes nor changes the format | §5 | this lane edits no wave-B file but `console.js`; rebase by function name |
| `CLAUDE.md` gates | TDD, 0 warnings, tests never leave the machine | T1, T2, T4 | tests before code; node runs locally; no listener |
| `CLAUDE.md`, contract list | Fixtures go to `contract-engineer` | T3 | T3's agent; `contract-reviewer` at Checkpoint 1 |
