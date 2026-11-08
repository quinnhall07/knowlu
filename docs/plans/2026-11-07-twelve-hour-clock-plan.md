# A 12-hour clock on every surface that shows a time: plan

**Status: T1 to T6 BUILT on `feat/twelve-hour-clock` (not merged; CI and Checkpoint 2 pending). Was: READY TO DISPATCH. r5 PASSED review r5 (0 Critical, 0 Important, 2 Minor, applied); Q1, Q2
and Q3 are all ruled (`docs/notes/2026-11-07-onboarding-rulings.md` on `feat/o3-long-onboarding`). Reviews r1 (0 Critical, 5 Important, 4 Minor), r2 (0 Critical, 2 Important,
4 Minor), r3 (0 Critical, 1 Important, 3 Minor) and r4 (0 Critical, 1 Important, 3 Minor) failed. The
reports are `docs/reports/2026-11-07-twelve-hour-clock-plan-review-r{1,2,3,4}.md`; r5 passed by message (2026-11-07). r4 was filed
as a report. This revision closes every finding (see the ledger's last four rows). Nothing dispatched.** Q1 is
ruled: **today.md and the events digest stay on 24-hour time** (Quinn, 2026-11-07, relayed by the
coordinator; §6). Q2 and Q3 are ruled too (Quinn, 2026-11-07, "yes to all four"): one fixed 12-hour
clock for every student, and both UTC displays fixed in this lane.

**Date:** 2026-11-07. **Branch:** `docs/twelve-hour-clock` (from `feat/o3-long-onboarding`).
**Source:** Quinn, 2026-11-04, replan draft §9 (`docs/plans/2026-11-04-onboarding-replan-draft.md`):
"A 12-hour clock for class times ... across every surface that shows a time". On 2026-11-07 Quinn said
to start it. Wave B's C6 leaves the format alone, and this lane changes nothing wave B owns (§5, merge order).

**Authority.** The cloud design, then `CLAUDE.md`'s two overriding rules, then the commitment-model
spec (`meets` is `"HH:MM"` 24-hour in frontmatter, §2 of that spec's table). This plan changes
**display only**. No frontmatter, journal, ledger, run-record or series-file byte changes format.
One caution (review r1, Important 4): the engine's 12-hour strings are **already vault bytes**. A
proposal or event card's title becomes approval `title:` frontmatter, and through its slug it names
the approval file (`commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md`). `ends_label` becomes
a commitment note's `ends:` key. T1 moves that code without changing a byte and pins it (§4, T1).

---

## 1. The format

This follows `eventemit::clock` and `eventemit::when_label` (`engine/src/eventemit.rs:509`, `:527`;
the rule's doc is on `what_and_when`, `:555-561`), which are already live in card titles and
commitment labels:

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
  strings it composes, such as `when` labels, card titles and note bodies. Its module doc says that
  these strings are written into notes (the caution under Authority).
- **One wording rule, two clocks, for a due-date move (review r1, Important 1).** `render::due_move`
  feeds both today.md and the console's amendment card (`surface.rs:877` → `AmendCard.due_move`, drawn
  at `console.js:209`). T2 splits it. `render::due_move_with(before, after, clock)` holds the wording,
  which covers days, years, EARLIER/later and same day. `render::due_move` stays as a thin wrapper that
  passes `%H:%M`, so today.md's bytes cannot move. The proof is a whole-string pin committed before
  the split (T2), not the goldens, which hold only a three-day move. `surface.rs` calls `due_move_with` with
  `clockfmt::clock(_, true)`. The wording is written once and only the clock differs. A surface-only
  copy of the wording was rejected because two copies would drift.
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

**Two zones on one page (review r2, Minor 2).** The engine's naive stamps (`local_stamp`,
`surface.rs:414-417`) are in the **vault's** zone, `cli::vault_zone`. The page's conversion of a
`...Z` stamp uses the **machine's** zone. The read model does not expose the vault zone: no `tz` field
appears in any surface reference or in `surface.rs`'s output types. So the helpers can't use it
without a new read-model field, and that would mean regenerating all three references. The two zones
differ only when the computer's zone is not the vault's, for example a student travelling. Then a
converted stamp (sync line, drawer history, `checked`/`synced`) and an engine stamp (generated,
Closed, Runs) would disagree by the offset. This is named in Q3.

---

## 2. Inventory: every place a clock time reaches the student

Legend: **12h** means already in the §1 format, **24h** means `HH:MM`, and **amb** means ambiguous (a
raw value shown as stored). "Frozen" means a Python reference pins it (`CLAUDE.md` rule 2).

### 2.1 Engine (what it writes or emits)

| Site | Reaches the student as | Now | Plan |
|---|---|---|---|
| `eventemit.rs:509` `clock`, `:527` `when_label`, `:561` `what_and_when` | event card titles, **written as approval `title:` frontmatter** | 12h | moves to `clockfmt` (T1), bytes unchanged and pinned |
| `commitments.rs:2998` `range_label`, `:3008` `meets_label` (`:3043-3052` `proposal_title`, `:3109` proposal `when`, `:5295` week `when`) | commitment cards (**`title:` frontmatter**), proposals, Schedule, onboarding step 10 | 12h | calls `clockfmt::range` (T1), bytes unchanged and pinned |
| `eventaccept.rs:64` `ends_label` (`:231` → **`ends:` frontmatter**, asserted at `:398`), `:272` Register task body | the accepted event's commitment note, Register task | 12h | calls `clockfmt::clock` (T1), bytes unchanged and pinned |
| `render.rs:110` `due_move` via `surface.rs:877` → `AmendCard.due_move` | console amendment card (`console.js:209`): `9/25 09:00 → 9/26 09:00` when a move is a day or less | 24h | **changes** on the surface only, through `due_move_with` (T2, §1). No surface reference pins a time: all three carry `9/25 → 9/22 (3 days EARLIER)` |
| `surface.rs:1244` `coming_up[].when` (`Wed 9/2 16:00`) | console Coming up | 24h | **changes** to `Wed 9/2 4pm` (T2). Pinned in `surface-today-full.json` (T3) |
| `eventemit.rs:296-302` events-digest line (`- [ ] Wed 9/2 16:00 · …`) | `approvals/events-digest-*.md` body only (default: `event_cards` off). The console never draws it and shows only counts (`console.js:307`, `:331`, `:661`) | 24h | **stays, Q1 ruled (Quinn, 2026-11-07)**. It is a vault page like today.md, and changing it would split one folder between two formats with no console gain. T2 pins it with a test that needs no contract-list edit |
| `surface.rs:979` `hm` → `the_day.blocks[].start/end` | console The day | 24h data | stays; the console formats it (T5). Pinned in all three surface references |
| `surface.rs:414` `local_stamp` → `topline.generated_at`, `closed[].when`, `runs_panel.recent[].started/ended` | console status line, Closed, Runs | 24h data | stays; the console formats it (T5). Its doc ties it to today.md's footer |
| `surface.rs:693` `due`, `:1245` `coming_up[].start`, `:2177` `now` | data only | ISO | stays |
| `render.rs:432-435` Schedule `09:00–10:00`, `:240-245` Coming up `Wed 9/2 16:00`, `:336` footer `2026-09-01 20:55`, `:119-120` `due_move` | `state/today.md` | 24h | **stays, frozen**: `golden-today-full.md` lines 9-10, 36-38 and 40, and `golden-today-s1.md` line 29. today.md's `due_move` is not pinned. It keeps 24-hour through the `render::due_move` wrapper so that one page has one format; the surface's copy changes (row above). **Q1 ruled: stays.** today.md is also the first-run marker (`ingest::is_first_run`, `ingest.rs:516`; `scheduler::needs_first_run`, `scheduler.rs:274`) and the parity oracle against the goldens |
| `eventroster.rs:116`, `:216` | `state/events.md` roster | 24h | **stays, frozen** (`vault-full/state/events.md`), and `eventroster.rs:270` parses it back |
| `calfeed.rs:882-889` | `state/calendar.md` snapshot | 24h | **stays, frozen** (`calendar-snapshot-gcal.md`), and `calfeed.rs:916` parses it back |
| `commitments.rs:1951` `hm` (`meets` JSON, series file, registrar changes) | data the page reads | 24h data | stays; the commitment spec makes it `HH:MM` |
| `eventaccept.rs:207-211` | commitment note `meets` frontmatter | 24h | stays: frontmatter is a contract |
| `cli.rs:930` `proposal_line` | `commitments` without `--json`, read by people at a terminal | 24h | stays: no student reads the CLI |
| `surface.rs:1949` `describe` → delta and history `text` (`due "2026-09-25T09:00" → …`) | console delta (`console.js:108`) and drawer history (`:1659`) | amb | **stays (review r1, Minor 2)**. The text quotes journal values verbatim through `ledger::dumps_value`. Reformatting it would mean parsing values back out of a sentence, and the line is an audit of what was written |
| `approvals.rs:1674`, `:4948`, `coursework.rs:1349`, `enrich.rs:1108`, `runs.rs:550`, `journal.rs:65` | frontmatter, the run log and the journal | stamps | stay: contracts |

### 2.2 App (what the page and the shell draw)

| Site | What the student sees | Now | Plan (T5) |
|---|---|---|---|
| `console.js:272` The day blocks `b.start–b.end` | `08:00–12:00` | 24h | `clockRange` → `8am–12pm` |
| `console.js:369` Coming up `e.when` | engine string | 24h now | 12h by T2; no page change |
| `console.js:209` amendment card `a.due_move` | engine string, `due 9/25 09:00 → 9/26 09:00` | 24h now | 12h by T2 (§1); no page change |
| `console.js:92` `generated … by …` | `2026-09-01 20:55` | 24h | `stampOf` |
| `console.js:151` task due chip text | `2026-08-26 23:59` | 24h | `stampOf` on the label only. The chip's edit value stays `r.due` |
| `console.js:396` Closed `x.when.slice(11)` | `20:55` | 24h | `timeOf` |
| `console.js:455` `localHM`, used at `:422`, `:474`, `:588`, `:610` | `checked 14:05`, `synced 14:05` | 24h, local | replaced by `timeOf` |
| `console.js:622` sync line `last slot` | `last_slot.ended.slice(11, 16)` | 24h **and UTC**: `ended` is `journal::now_ts` (`app/src/scheduler.rs:1334`, `...Z`) | `timeOf` converts to local time (Q3) |
| `console.js:639` Runs view expected row, `h(e.due)` passed through raw, with no slice (review r2) | `2026-08-27 15:00` | 24h | `stampOf` |
| `console.js:643` Runs view recent row, `(r.ended \|\| r.started).slice(0, 16)` | `2026-08-27 15:00` | 24h | `stampOf` |
| `console.js:1286` runs panel expected: `e.due.slice(11, 16)` **and** the raw tail `" — expected " + h(e.due)` | `15:00 … expected 2026-08-27 15:00` | 24h | `timeOf` for the first, `stampOf` for the tail |
| `console.js:1287` runs panel recent, `.slice(11)` | `15:00` | 24h | `timeOf` |
| `console.js:643` `h(r.run_id)` and `<pre class="summary">`; `:1287` `x.summary` | `rank-2026-08-27T20:00:00Z`, run-log text | ids and verbatim logs | **stays**: an id (`runs.rs:140`) and the engine's own summary, not a clock display. These are allowed regions in T4's render scan |
| `console.js:1659` note drawer history `r.ts.slice(0, 16).replace("T", " ")` (review r1, Important 2) | `2026-09-01 19:55`. `r.ts` is the raw journal stamp (`surface.rs:1955`, from `journal::now_ts`) | 24h **and UTC** | `stampOf`, which converts it to local time. The `r.text` beside it stays (§2.1, `describe` row) |
| `console.js:1654-1657` drawer frontmatter list (`due`, `executed_at`, …) | the stored value | amb | **stays (review r1, Minor 2)**. It is the note as written, and `due` is editable in place (`EDITABLE`), so the shown value must be the value the edit sends back |
| `console.js:674-675` decision card `changes`, `JSON.stringify(v)` | `due → {"from":"2026-09-25T09:00",…}` | amb | a `due` change renders `stampOf(from) → stampOf(to)`. Other keys stay as today |
| `console.js:1057` `regMeets` (registrar changes) | `Mon/Wed 13:00–14:15` | 24h | `clockRange` |
| `console.js:2803` mail applied "changed" row: `x.old`/`x.new` when `x.field` is `due`; `:2806` `x.opened_at`; `:2808` `x.due` | raw vault value | amb | `stampOf` for those fields only. Other fields' `old`/`new` stay |
| `console.js:898`, `:904`, `:1042`, `:1126`, `:1170`; `onboarding.js:646`, `:704` | engine `when` | 12h | no change |
| `console.js:817-818`; `onboarding.js:677`, `:787` `<input type="time">` | native control (follows the Windows locale; value is `HH:MM`) | OS | no change |
| `onboarding.js:248-275` week strip | days and counts, no clock | n/a | no change |
| `app/src/tray.rs:42` tooltip `Knowlu` | no time | n/a | no change. The app sends no OS notifications (no notification code in `app/src`) |
| `app/src/tray.rs:184-198` diagnostics copy | raw stamps for support | 24h | out of scope: a support artefact, not a display |

Cloud judgment text, such as a model's `why`, is not formatted by us and is out of scope.

---

## 3. What may change, and what may not

- **May change:** `surface --view today`'s `coming_up[].when` and `must_do.amendments[].due_move`,
  and everything the console draws in the 24h and amb rows of §2.2 that say what to change.
- **May not change:** `state/today.md` and the events-digest body (Q1, ruled), `state/events.md`, `state/calendar.md`, `state/calendar-series.json`, any frontmatter or journal
  record, run records, the run log, `commitments` CLI text, the delta and history `text`, and the
  bytes of every 12h string in §2.1. Those last ones are vault bytes, and T1 is a pure move.
- **Pinned by:** `engine/tests/oracle.rs` (both goldens), T2's `today_md_due_move_is_pinned_whole_string`
  and digest pin, `commitments.rs:9315`, `eventaccept.rs:398`, `eventroster.rs`'s and `calfeed.rs`'s
  reference tests, `engine/tests/surface_oracle.rs`. All stay green with no edit, except
  `surface-today-full.json`, which T3 regenerates.

---

## 4. Tasks

Order: T1 ∥ T4, then T2 → T3. **T5 runs after both T3 and T4**, starting from a head that contains
both (review r4, Important 1). T4's Layer 1 reads `surface-today-full.json` and expects `Wed 9/2 4pm`,
which exists only once T3 has regenerated it. T6 runs last. The file sets are disjoint except T1 and
T2 (both `eventemit.rs`, run in sequence). Extract each brief by its heading.

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
- **Vault bytes (review r1, Important 4; r2, Minor 1).** `clockfmt`'s module doc says plainly that
  its output is written into the vault:
  - approval `title:` frontmatter, through `proposal_title` and `what_and_when`;
  - **approval file names**, because a card's title becomes its slug
    (`commitment-check-cs-100-mon-wed-fri-12-12-50pm-a-class.md`);
  - commitment `ends:` frontmatter, through `ends_label`;
  - the Register task's body.

  A change to any output therefore gives new notes a different format and different file names from
  old ones, and it needs a ruling and a plan of its own. **The pins already exist, and T1 adds no new
  test for this.** `commitments.rs:9315`
  (`a_class_proposal_files_one_card_with_the_exact_title_frontmatter_and_body`) pins the file name
  (`:9321-9322`) and the `title:` line (`:9334`) byte for byte, and `eventaccept.rs:398` pins `ends:`.
  T1 adds a one-line comment beside each pin that points at the module doc. Both tests pass unedited
  through the move.
- **Done when:** every existing assertion passes unedited, including `commitments.rs:9381-9383`,
  `:10829`, `:11117`, `eventaccept.rs:391`, `:398`, the `what_and_when` tests and every oracle.
  `cargo build --workspace` and `cargo test --workspace` pass with 0 warnings.

### T2. Engine: the surface's two 24-hour strings (`implementer`, after T1)

**Why this agent:** two specified call sites, one wrapper split and their tests, all off the contract
list. **Files:** `engine/src/surface.rs`; `engine/src/render.rs` (the whole-string pin first, then
the `due_move` split only); `engine/src/eventemit.rs` (one new test, no code change).

- **First commit, before any code moves: today.md's due-move pins (review r2, Important 2).** The
  goldens pin only a three-day move, which shows no time, and `render.rs`'s existing due-move tests
  check substrings (`:1016`, `:1027`, `:1033`, `:1049`). So neither proves the clock path survives
  the split. Add `today_md_due_move_is_pinned_whole_string` to `render.rs`'s tests. It calls
  `render::due_move` directly and uses `assert_eq!` on whole strings. These are today's outputs,
  cross-checked by reading `render.rs:110-133`: `days_between(before, after)` is `before − after`;
  times appear when the gap is at most one day; years appear when the years differ.

  | before → after | `due_move` today |
  |---|---|
  | 2026-10-09T00:30 → 2026-10-08T23:00 | `10/9 00:30 → 10/8 23:00 (1 day EARLIER)` |
  | 2026-12-31T23:00 → 2027-01-01T01:00 | `12/31/2026 23:00 → 1/1/2027 01:00 (1 day later)` |
  | 2026-10-09T23:59 → 2026-10-09T13:00 | `10/9 23:59 → 10/9 13:00 (EARLIER same day)` |
  | 2026-10-09T13:00 → 2026-10-09T23:59 | `10/9 13:00 → 10/9 23:59 (later same day)` |
  | 2026-10-09T13:00 → 2026-10-09T13:00 | `10/9 13:00 → 10/9 13:00 (no change)` |
  | 2026-09-25T09:00 → 2026-09-22T09:00 | `9/25 → 9/22 (3 days EARLIER)` |
  | 2027-01-05T09:00 → 2026-10-05T09:00 | `1/5/2027 → 10/5/2026 (92 days EARLIER)` |

  This test is committed green on the unchanged code, as its own commit. The rest of T2 must leave it
  unedited and green. **That, not the oracle, is the proof that today.md's clock did not move.**
- Then, tests first for the change.
  - Amendment card: `surface.rs` gets a test for the same seven inputs, run through the card's path
    (`due_move_with` with `clockfmt`). It expects `10/9 12:30am → 10/8 11pm (1 day EARLIER)`,
    `12/31/2026 11pm → 1/1/2027 1am (1 day later)`, `10/9 11:59pm → 10/9 1pm (EARLIER same day)`,
    `10/9 1pm → 10/9 11:59pm (later same day)`, `10/9 1pm → 10/9 1pm (no change)`, and the two
    multi-day strings unchanged.
  - Coming up: change `surface.rs:2775` to `["Wed 9/2 4pm", "Fri 9/4 12pm", "Thu 9/10 6pm"]`, and
    update the `when` literals at `:4016` and `:4357`.
  - **Digest pin (review r1, Important 3; Q1 ruled).** `the_digest_line_keeps_its_24_hour_stamp_and_its_parse_shape`
    in `eventemit.rs`'s tests emits a digest for an event at 16:00 on 2026-09-02. It asserts that the
    line starts `- [ ] Wed 9/2 16:00 · ` and ends with the backticked uid. It then matches that line
    against the parser's unchecked shape, restated in the test as `` ^- \[ \] .*`([^`]+)`\s*$ ``, and
    gets the uid back from the capture. The test calls nothing private and edits no contract-list file.
    Its comment (review r2, Minor 4) says: "a copy of `approvals.rs:1244` (`UNCHECKED`): change the
    two together, in one commit". It also cites Quinn's 2026-11-07 ruling that the digest stays
    24-hour.
- Code.
  - Split `render::due_move` into `due_move_with(before, after, clock: impl Fn(DateTime) -> String)`.
    `due_move` becomes a wrapper that passes `|t| t.strftime("%H:%M").to_string()`.
  - `surface.rs:877` calls `due_move_with` with `|t| clockfmt::clock(t.time(), true)`, and
    `surface::coming_up`'s `when` calls `clockfmt::clock(t.time(), true)` instead of `hm`.
  - `surface::hm` stays for the blocks. Nothing else in `render.rs` changes.
- **Expected red:** `surface_today_full_is_frozen` only, because the references' `due_move` carries no
  time. s1 and s1-migrated must stay green. `oracle.rs`, the existing due-move tests and
  `today_md_due_move_is_pinned_whole_string` must stay green and unedited. T3 makes the tree green,
  and T2 and T3 go in one push.

### T3. Regenerate `surface-today-full.json` (`contract-engineer`; `contract-reviewer` reads it)

**Why this agent:** `engine/tests/fixtures/**` is on the contract list, and console spec §4.6 governs
a read-model reference. **File:** `engine/tests/fixtures/surface-today-full.json` only.

- Regenerate it with the Git Bash command in `engine/tests/surface_oracle.rs`'s header: one JSON line
  plus CRLF, UTF-8, no BOM.
- **Its own commit**, whose diff shows exactly four substitutions, each occurring once: `Wed 9/2 16:00`
  → `Wed 9/2 4pm`, `Fri 9/4 12:00` → `Fri 9/4 12pm`, `Thu 9/10 18:00` → `Thu 9/10 6pm`, and the
  payload hash `"revision": "697b11c05ff9"` → `"revision": "f98981fd806a"`. The fourth is
  `surface::revision` (`surface.rs:2219`), the sha1 over the payload minus `now` and `revision`, so
  it moves with the three strings (corrected at T3; this brief first said three). Check it by
  applying those four substitutions to the old bytes and comparing the result with the new file,
  and by recomputing the revision over the new payload. The message says why:
  "surface: coming_up.when on a 12-hour clock (twelve-hour-clock plan T2); console spec §4.6".
- `git diff --stat -- engine/tests/fixtures` names one file. `surface-today-s1.json`,
  `surface-today-s1-migrated.json` and the eight Python references are byte-unchanged. Every
  reference's `due_move` stays `9/25 → 9/22 (3 days EARLIER)`: a three-day move shows no time, so T2's
  amendment change reaches no reference.

### T4. Console formatter: the failing tests (`test-writer`)

**Why this agent:** this is the failing test, written before the page changes. **File:**
`app/tests/console_clock.rs` (new).

- Lift `clock12`, `clockRange`, `timeOf` and `stampOf` out of `console.js` under node, with the
  harness pattern `main_handover_html` uses (`app/tests/onboarding_static.rs:634`).
- Tables.
  - `clock12` and `clockRange` match T1's cases string for string.
  - `stampOf` on naive local stamps, one per shape the page receives (review r1, Minor 4):
    `"2026-09-01 20:55"` → `2026-09-01 8:55pm` (run and journal-derived stamps);
    `"2026-08-26T23:59"` → `2026-08-26 11:59pm` (task `due`, amend `from`/`to`);
    `"2026-08-26T23:59:00"` → `2026-08-26 11:59pm` (with seconds, a mail `due`/`old`/`new`);
    `"2026-09-25 09:00"` → `2026-09-25 9am`. A date-only `"2026-08-26"` comes back unchanged. These are
    never shifted by `TZ`.
  - Zoned stamps convert to local time. Run node with `TZ=America/Chicago` and expect
    `stampOf("2026-09-01T14:05:00.000Z")` → `2026-09-01 9:05am` (the drawer history's
    `journal::now_ts` shape) and `timeOf` of the same → `9:05am` (the sync line's `last_slot`). An
    offset form, `2026-09-01T14:05:00-05:00`, gives `2:05pm`. If node on the runner ignores `TZ`,
    assert against the harness's own `new Date(iso)` hours instead.
  - Empty, `null` or unparseable input gives `""` from `timeOf` and the input unchanged from
    `stampOf`.

There are three layers. The render scan covers every row `paint` reaches. The two static layers cover
the rows it does not reach, and they name the site whenever something slips through.

- **Layer 1, the render scan over `paint` (review r2, Important 1; r3, Important 1).**
  - **A new recording `document`, written for this test.** It is modelled on the
    `onboarding_static.rs:634` harness's way of lifting a function out of the IIFE, but it is not that
    harness: that `document` is a Proxy whose `set` discards every write (`:638`).
    - **Each recorder is a `Proxy` (review r4, Minor 1).** It records `innerHTML`, `textContent` and
      `title` on `set`, and lets every other member through as a permissive stub, so that any other
      member access or call succeeds. `paint` reaches `querySelectorAll`, `closest`, `prepend` and
      `contains` through `preserveAcross` (`console.js:176`, `:188-198`). The page's startup code also
      runs under `vm.runInContext` and touches the DOM before any test call.
    - On a recorder, `querySelectorAll` returns `[]`, `querySelector` returns a fresh recorder, and
      `closest`, `prepend`, `contains` and `classList.*` are no-ops (`contains` returns `false`).
      `appendChild` records the child's `title` and `textContent`.
    - `document.getElementById(id)` returns one recorder per id, kept across calls. Its
      `querySelector`/`querySelectorAll` behave as on a recorder, and `createElement` returns a new
      recorder.
    - `__TAURI__.core.invoke` returns a promise that never settles. **No `invoke` is stubbed with
      data**, and nothing awaits one.
  - **What it runs.** Lift `paint` and `current`. Call `paint(state, true)` with `current.view` set to
    `today`, `runs` and `decisions` in turn. `paint` is the only renderer called.
  - **Its inputs.**
    - `engine/tests/fixtures/surface-today-full.json` gives the full reference's blocks, Coming up and
      task `due`s.
    - `surface-today-s1-migrated.json` gives the s1 blocks (review r3, Minor 3).
    - A supplement, merged in (its `decisions.cards` is appended to the reference's; its `closed_this_week`, `runs_panel.expected` and `runs_panel.recent` replace the reference's), supplies only rows that `paint` reaches. Each row must produce its own
      known string under `TZ=America/Chicago`:

    | Supplement row | Rendered by | Known string |
    |---|---|---|
    | `topline.generated_at: "2026-09-01 20:55"`, `generated_by: "manual"` | `renderTopline` `:92` | `generated 2026-09-01 8:55pm by manual` |
    | `topline.last_slot.ended: "2026-09-01T14:05:00.000Z"`, `ok: true` | `renderSyncLine` `:622` | `last slot 9:05am ok` |
    | `topline.sync.at: "2026-09-01T14:05:00.000Z"`, with no `last_error` and no `skipped` (`console.js:600`) | `renderSyncLine` `:610` | `synced 9:05am` |
    | `closed_this_week` (`console.js:394`): two rows whose `when` is `"2026-09-01 20:55"` and `"2026-08-31 09:30"` | `renderClosed` `:396` | `8:55pm`; `9:30am` |
    | an amendment, `due_move: "10/9 11:59pm → 10/9 1pm (EARLIER same day)"` (T2's output) | `renderMustDo` `:209` | `due 10/9 11:59pm → 10/9 1pm (EARLIER same day)` |
    | one decision card in the full card shape: `id: "appr_clock12_importance"`, `kind: "amend"`, `age_days: 0`, `title: "Clock test card"`, `why: "**Why proposed:** test."`, `changes: {importance: 4}` (review r4, Minor 3) | `renderDecisionsView` `:675` | `importance → 4` (a key other than `due` stays as today) |
    | `runs_panel.expected`: one `late` row, `due: "2026-08-27 15:00"` | `renderRunsView` `:639`; `renderRuns` `:1286` | `2026-08-27 3pm`; `3pm`; `expected 2026-08-27 3pm` |
    | `runs_panel.recent`: one row, `ended: "2026-08-27 15:02"` | `renderRunsView` `:643`; `renderRuns` `:1287` | `2026-08-27 3:02pm`; `3:02pm` |

    The references alone must yield:
    - `8–9am`, `10am–12pm`, `12–12:50pm` and `3–6pm` (the full reference's blocks);
    - `8am–12pm`, `1–1:45pm` and `1:45–6pm` (the s1 blocks);
    - `Wed 9/2 4pm` (Coming up, after T3);
    - `2026-08-26 11:59pm` (a task's due chip);
    - `2026-09-25 9am → 2026-09-22 9am`, from `decisions.cards[1]`'s `due` change. Both references
      already carry that card, so the supplement adds no due card (review r4, Minor 3).
  - **The scan.** Strip tags, decode entities, and remove the **allowed regions**: every delta `text`
    string, attribute values, `<pre class="summary">` contents and run ids. The test knows each of
    these strings. Then assert:
    - no 24-hour form: `\b(0\d|1[3-9]|2[0-3]):[0-5]\d\b`, and no ISO `T\d\d:\d\d`;
    - every remaining `\b(1[0-2]|[1-9]):[0-5]\d\b` is followed either by `am`/`pm`, or by `–` and a
      time ending in `am`/`pm`;
    - every known string above is present.
  - **Rows `paint` never reaches (review r3, Important 1).** They are named here so that nobody drops
    them quietly. Each is covered by its Layer 2 pin, by Layer 3's forbidden forms, and by **a direct
    node unit call of the helper it uses**, on that row's input shape:

    | Row | Why `paint` misses it | Helper call and expected output |
    |---|---|---|
    | update `:422`, `:474` | rendered from the update command's own answer | `timeOf("2026-09-01T14:05:00.000Z")` → `9:05am` |
    | sync toast `:588` | inside `runSync()` | the same `timeOf` call → `9:05am` |
    | registrar `regMeets` `:1057` | reached only through `your_week` and `registrar_label` | `clockRange("13:00", "14:15")` → `1–2:15pm` |
    | the drawer history, `openDrawer` `:1648` (`:1659`) | opened through `invoke("note")` | `stampOf("2026-09-01T14:05:00.000Z")` → `2026-09-01 9:05am` |
    | mail applied `:2803`, `:2806`, `:2808` | `renderMailApplied` runs from `invoke("mail_applied")` (`:2834`) | `stampOf("2026-08-26T23:59:00")` → `2026-08-26 11:59pm`; `stampOf("2026-08-26T23:59")` → the same |

- **Layer 2, positive pins (review r2, Important 1; r3, Minor 1).** Each pin is one row: an anchor and
  the helper call required on the same source line. **An anchor is the exact substring shown, matched
  without any surrounding quotes. Each one occurs once in `console.js` today**; the test asserts that
  count, so a duplicate anchor fails it. T5 must not reflow these lines.

  | §2.2 row | Anchor (exact substring) | Required |
  |---|---|---|
  | `:92` | `"generated "` | `stampOf(` |
  | `:151` | `"no due date"` | `stampOf(` |
  | `:272` | `<div class="blk ` | `clockRange(` |
  | `:396` | `EL("closed").innerHTML` | `timeOf(` |
  | `:422` | `", checked "` | `timeOf(` |
  | `:474` | `"up to date (checked "` | `timeOf(` |
  | `:588` | `showNote("synced "` | `timeOf(` |
  | `:610` | `"tried "` | `timeOf(` |
  | `:622` | `t.last_slot.ended` | `timeOf(` |
  | `:639` | `function renderRunsView` (its body) | `stampOf(e.due)` |
  | `:643` | `<pre class="summary">` | `stampOf(` |
  | `:674-675` | `class="changes"` (its block, through the `.join(" · ")`) | a `k === "due"` branch that calls `stampOf(` |
  | `:1057` | `function regMeets` (its body) | `clockRange(` |
  | `:1286` | `" — expected "` | `timeOf(` and `stampOf(` |
  | `:1287` | `r.recent.slice(0, 6)` | `timeOf(` |
  | `:1659` | `<div>no journal history</div>` | `stampOf(` |
  | `:2803` | `h(x.field) + ": "` | `stampOf(` |
  | `:2806` | `x.opened_at ?` | `stampOf(` |
  | `:2808` | `" · due "` | `stampOf(` |

  These raw passthroughs are forbidden anywhere: `h(e.due)`, `h(x.due)`, `h(x.opened_at)` and
  `h(t.generated_at)`. **`JSON.stringify(v)` is forbidden only on the `due` branch of the `:674-675`
  block (review r3, Minor 2).** Other keys keep `JSON.stringify` as §2.2 says. Layer 1's
  `importance → 4` and its `2026-09-25 9am → 2026-09-22 9am` check both halves.
- **Layer 3, forbidden forms (review r1, Important 2; r2, Important 1).** These apply outside the four
  helpers' bodies, which are cut out first. One array lists every form, so a new one is a one-line
  addition:
  - `.slice(` with a first argument from 10 to 19, any `.slice(-…)`, and `.slice(0,` with a second
    argument from 13 to 19 (`.slice(0, 10)` for a date stays legal);
  - `.substr(` and `.substring(`;
  - `.replace(` or `.split(` whose argument is `"T"`, `'T'`, `" "` or `":"`, or a regex literal
    whose source contains `T` or `:`. `:9`'s `h()` and `:1518`'s `/[ (]/` stay legal;
  - `.split(" ")[1]`;
  - `getHours(`, `getMinutes(`, `getUTCHours(`, `getUTCMinutes(`, `toTimeString(`,
    `toLocaleTimeString(`, `toLocaleString(`, `toISOString().slice` and `Intl.DateTimeFormat`.
- **How the static layers strip comments (review r2, Minor 3).** The scanner is a small state machine
  that walks the source once, with a state for each string type: `"`, `'` and template literals
  (with `\` escapes), and regex literals. A regex literal is recognised by a `/` after `(`, `,`, `=`,
  `:`, `[`, `!`, `&`, `|`, `?`, `{`, `;` or `return`, and runs to the unescaped `/` outside a
  character class. It drops `//` and `/* */` **only** in code state and keeps strings intact. Its
  self-checks must all hold:
  - `var u = "https://x.example"; x.slice(11, 16)` is a hit, because the `//` inside the string is
    not a comment;
  - `// x.slice(11, 16)` and `/* x.slice(11, 16) */` are not hits;
  - `/\/\//.test(s); x.slice(11, 16)` and `` `a//b`; x.slice(11, 16) `` are hits;
  - every form in Layer 3's array is a hit;
  - **lossless guard:** every Layer 2 anchor is still present in the stripped text, so a stripper that
    eats code fails loudly.
- Every test fails before T5. The helpers are missing, the render scan finds `08:00–09:00` and its
  kin, and today's `:151`, `:396`, `:622`, `:639`, `:643`, `:1286`, `:1287` and `:1659` each trip a
  layer.

### T5. Console: route every time through the formatter (`console-ui`, after T3 and T4)

**Why this agent:** this is `app/static` work, and T4's tests check it. **Files:**
`app/static/console.js`, and `app/tests/static_assets.rs` (review finding: four assertions there pinned
`localHM`, which T5 replaces with `timeOf`).

- Add the four helpers in place of `localHM` and implement §1's rule. A string with `Z` or an offset
  goes through `new Date` and is shown in local time. A naive `YYYY-MM-DD[T ]HH:MM[:SS]` is read as
  written and never shifted. The page does no other arithmetic.
- Change every **24h** and **amb** row of §2.2 whose plan names a helper, and nothing else. The
  "stays" rows stay as they are: the drawer's frontmatter list (`:1654-1657`) and the delta and
  history `text`. So do the due chip's edit value, the window editor's `<input type="time">` values
  and the `windowSequence` payload (`:838`), which stay `HH:MM` and ISO.
- Keep every Layer 2 anchor line as a single line (T4); do not reflow them.
- Q3 is ruled yes: `:622` and `:1659` are converted to local time, like every other zoned stamp. The
  helpers have no mode that keeps UTC.
- **Done when** T4 passes, `static_assets.rs` and `onboarding_static.rs` stay green, and fixture
  screenshots exist of:
  - The day, Coming up, and an amendment card with a same-day move;
  - a decision card with a `due` change;
  - the sync line, Closed, Runs, and the note drawer's history;
  - Schedule's registrar changes;
  - the mail-applied rows: a changed `due`, a notice's `opened_at`, and an added row's `due`.

  Take them by `?fixture=` and headless Edge, never with synthetic input.

### T6. Docs (`docs-keeper`, last)

**Why this agent:** reference upkeep. **Files:** `docs/surface/anatomy.md` (the format of Coming up's `when`
and the amendment card's `due_move`; `the_day.blocks` times are data that the console formats), `docs/reference/engine-commands.md` if it
quotes a surface time, this plan's ledger status, and one `HANDOFF.md` line at the milestone.

---

## 5. Merge order and review checkpoints

- Wave B's C6, C7 and C10 edit `app/static` and `onboarding_static.rs`. This lane edits `console.js`
  and a new test file, never `onboarding.js`. Dispatch T5 from a head that contains both T3 (the
  regenerated `surface-today-full.json`) and T4 (the tests). A head with T4 alone still holds
  `Wed 9/2 16:00`, `Fri 9/4 12:00` and `Thu 9/10 18:00`, so T5 could never pass T4 there.
  If wave B lands first, the `integrator` rebases by function name, never by line number.
- **Checkpoint 1** (after T1 to T3): `reviewer` reads T1 and T2, and `contract-reviewer` reads T3's
  diff. The report goes in `docs/reports/`.
- **Checkpoint 2** (after T5): `reviewer` reads T4 and T5. **Quinn** looks at the screenshots: the
  format on the real page is Quinn's call.
- Nothing is pushed before its checkpoint passes, and every PR waits for green CI and a reviewer PASS.

---

## 6. Questions for Quinn

1. **RULED (Quinn, 2026-11-07, relayed by the coordinator): today.md and the events digest stay on
   24-hour time.** today.md is still needed: it is the first-run marker (`ingest::is_first_run`,
   `ingest.rs:516`; `scheduler::needs_first_run`, `scheduler.rs:274`) and the parity oracle against
   the frozen goldens. What it means here: §2.1's today.md and digest rows stay; T2's digest pin and its
   `today_md_due_move_is_pinned_whole_string` hold that line; the roster and calendar snapshot stay as
   well. (The question as asked: `golden-today-full.md` pins `09:00–10:00`, `Wed 9/2 16:00` and the
   footer `20:55`, and `golden-today-s1.md` pins its footer. The recommendation was to keep both.)
2. **RULED (Quinn, 2026-11-07, "yes to all four"): one fixed 12-hour clock for every student, not
   the Windows setting.** The question as asked follows. **A fixed 12-hour clock, or a setting per
   student?** (Corrected per review r1, Important 5.)
   Determinism does not decide this. A format taken from vault config or a flag is ordinary input, and
   the same input still gives the same bytes. Only reading the OS setting at run time would break
   that. The real constraint is that the engine's 12-hour strings are written into notes (card
   `title:` and commitment `ends:` frontmatter, §1). A per-student format, or a later change of one,
   splits one vault between the old format in old notes and the new format in new ones, and synced
   vaults carry that split. The page could follow a setting freely, but then its strings and the
   engine's would differ unless both read it. **Recommendation: fixed, the same for every student.**
   That is not a single-user assumption, but it is a US-locale one. A setting waits until a non-US
   school signs up, and it would come with its own plan for the notes already written. Native time
   inputs already follow Windows.
3. **RULED (Quinn, 2026-11-07, "yes to all four"): both UTC displays are fixed in this lane.** The
   question as asked follows. **Fix the UTC times in the same lane?** Two places show UTC
   (`journal::now_ts`) instead of local time: the sync line's `last slot 14:05` (`console.js:622`) and
   the note drawer's history stamps (`:1659`, found by review r1). T5's helpers correct both as a side
   effect, so the shown times move by the zone offset. The recommendation was yes, because the lines
   are the same and so are the helpers.

   **The zone they convert to (review r2, Minor 2).** The helpers convert to the **machine's** zone,
   as `localHM` does today. The engine's own stamps (generated, Closed, Runs) are in the **vault's**
   zone (`local_stamp`, `surface.rs:414-417`). The read model doesn't expose the vault zone, so the
   page can't use it in this lane. The two differ only when the computer's zone is not the vault's,
   for example while a student travels. Then a converted stamp and an engine stamp on one page
   disagree by the offset.
   **Recommendation: accept the machine zone for this lane.** This plan proceeds on it, and the
   rulings record does not name a zone. It's what the student's clock says, and
   it's today's behaviour for `checked`/`synced`. The alternative is a follow-up: add the vault zone to
   the read model's topline, which means regenerating all three surface references under §4.6, and
   convert with it through `Intl` inside the helpers.

---

## 7. Fidelity ledger

| Source | Requirement | Task | Evidence |
|---|---|---|---|
| Quinn, 2026-11-04 (replan §9) | A 12-hour clock for class times, on every surface that shows a time | T1 to T5 | §2's inventory; T4's tables; Checkpoint 2's screenshots |
| Quinn, 2026-11-07 | Start it | this plan | — |
| Caller brief | Follow eventemit's rule: minutes only when non-zero, the end always carries am/pm | §1, T1 | `clockfmt` table; existing 12h assertions unedited |
| Caller brief | One formatter per layer | T1, T2, T5 | `clockfmt.rs`; `due_move_with` (one wording, two clocks); T4's three layers (render scan, positive pins, forbidden forms) and the stripper's self-checks |
| `CLAUDE.md` rule 2 | No frozen Python reference regenerated; today.md keeps matching | §3, T2, T3 | `oracle.rs`, roster and snapshot tests green with no edit; `today_md_due_move_is_pinned_whole_string`, committed before the split; T3's diff stat names one file |
| **Quinn, Q1 ruling (2026-11-07, relayed by the coordinator)** | today.md and the events digest stay on 24-hour time. today.md stays as the first-run marker and the parity oracle | §2.1, §3, T2, §6 Q1 | the §2.1 rows marked "stays, Q1 ruled"; T2's digest pin; the whole-string due-move pin; `oracle.rs` unedited |
| Console spec §4.6 | A surface reference is regenerated only in a commit that shows the diff and says why | T3 | the four-substitution check (three `when` strings plus the `revision` hash, which moves with the payload); the commit message |
| `CLAUDE.md`, engine invariants | Frontmatter, journal and ledger bytes never change format; display only | §2.1, T1, T2 | `clockfmt`'s module doc names the frontmatter and approval file names it feeds; `commitments.rs:9315` (file name `:9321-9322`, `title:` `:9334`) and `eventaccept.rs:398` pin the bytes; `meets` stays `HH:MM` |
| `CLAUDE.md` | Determinism | §1, T1 | `clockfmt` is pure and reads no clock, zone or locale |
| `CLAUDE.md` rule 1 | No single-user assumptions | §1, Q2 | one fixed format for every student |
| **Quinn, Q2 ruling (2026-11-07, "yes to all four"; `docs/notes/2026-11-07-onboarding-rulings.md`)** | One fixed 12-hour clock for every student, not the Windows setting | §1, T1, T5, §6 Q2 | `clockfmt` and the console helpers take no format input; native time inputs left as they are |
| Replan §9 | Wave B neither fixes nor changes the format | §5 | this lane edits no wave-B file but `console.js`; rebase by function name |
| `CLAUDE.md` gates | TDD, 0 warnings, tests never leave the machine | T1, T2, T4 | tests before code; node runs locally; no listener |
| `CLAUDE.md`, contract list | Fixtures go to `contract-engineer`; no cheaper agent edits `approvals.rs` | T2, T3 | T3's agent; `contract-reviewer` at Checkpoint 1; T2's digest pin restates the parser's shape and calls nothing private |
| **Quinn, Q3 ruling (2026-11-07, "yes to all four"; `docs/notes/2026-11-07-onboarding-rulings.md`)** | Both UTC displays are fixed in this lane: the sync line and the drawer history show local time (the machine's zone; §1's two-zones note), not UTC | T4, T5 | T4's zoned `timeOf` and `stampOf` cases under `TZ=America/Chicago`; Layer 1's converted-stamp check; the `:622` and `:1659` positive pins; Checkpoint 2's screenshots of both. T5's "Q3 no" path is removed |
| Review r1 (`docs/reports/2026-11-07-twelve-hour-clock-plan-review-r1.md`) | **I1** amendment `due_move`: §1, the §2.1 row, T2 (`due_move_with`). **I2** drawer UTC and every slicing form: the §2.2 rows, T4's forbidden-forms list. **I3** a digest test with no contract-list edit: T2's pin. **I4** clockfmt writes vault bytes: Authority, §1, the §2.1 rows, T1's pin. **I5** Q2's reason: §6 Q2. **M1** citations: §1. **M2** the raw `due` displays: the §2.1 `describe` row and the §2.2 drawer row, marked stays. **M3** the digest: dropped from the change, follows Q1. **M4** T4's cases and the Q3 row: T4, this ledger | §1 to §7 | r2 review |
| Review r2 (`docs/reports/2026-11-07-twelve-hour-clock-plan-review-r2.md`) | **I1** raw passthroughs and dodges: §2.2 rows split (`:639` is a raw `h(e.due)`, `:1286`'s tail, run ids and summaries allowed); T4's Layer 1 render scan over the real renderers, Layer 2's per-row anchors and raw-passthrough ban, Layer 3's widened forms. **I2** the clock path's proof: T2's whole-string `today_md_due_move_is_pinned_whole_string`, committed before the split and cross-checked against `render.rs:110-133`, plus its surface twin. **M1** the existing `commitments.rs:9315` pin replaces the new test; the module doc names approval file names. **M2** two zones: §1's note and Q3. **M3** the stripper's state machine and self-checks, including `https://` in a string, plus the lossless guard. **M4** the digest regex comment says to change it together with `approvals.rs:1244`. Mail rows added to Checkpoint 2's screenshots | §1 to §7 | r3 review |
| Review r3 (`docs/reports/2026-11-07-twelve-hour-clock-plan-review-r3.md`) | **I1** Layer 1 rebuilt on the simpler path: a new recording `document` (not the discarding `onboarding_static.rs:638` Proxy), `paint` only, no `invoke` stubbed with data; a supplement of rows `paint` reaches, one known string each. The rows `paint` never reaches are named: update `:422`/`:474`, sync `:588`, `regMeets` `:1057`, the drawer `:1648`, and also mail applied `:2803`/`:2806`/`:2808` (`:2834`). Each is covered by its Layer 2 pin, Layer 3 and a direct helper unit call. **M1** anchors are exact unquoted substrings, each asserted to occur once; `:396` re-anchored on `EL("closed").innerHTML` (`data-undo-close` also matched `:2148`), `:643` on `<pre class="summary">`, `:1659` on `<div>no journal history</div>`. **M2** the `JSON.stringify(v)` ban is limited to the `due` branch of `:674-675`; other keys keep it, checked by `importance → 4`. **M3** `surface-today-s1-migrated.json` is fed for its s1 blocks; the runs rows come from the supplement | T4, T5 | r4 review |
| Review r4 (`docs/reports/2026-11-07-twelve-hour-clock-plan-review-r4.md`) | **I1** T5 runs after both T3 and T4, from a head that holds both: the §4 Order line, T5's heading and §5's dispatch sentence changed, because without T3 `surface-today-full.json` still says `Wed 9/2 16:00`. **M1** each recorder is a `Proxy` that records `innerHTML`/`textContent`/`title` and lets every other member through; `querySelectorAll` returns `[]`; `closest`, `prepend` and `contains` are no-ops (`preserveAcross`, `console.js:176`, `:188-198`; startup under `vm.runInContext`). **M2** exact key paths: `closed_this_week` (`:394`) and `topline.sync.at` (`:600`). **M3** the due change is read from both references' `decisions.cards[1]`; the supplement keeps one full-shape card (id, kind, age_days, title, why) for `importance → 4` | T4, T5, §4, §5 | r5 review |
