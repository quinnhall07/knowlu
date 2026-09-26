# Pre-execution review: the commitment model, phase 3 plan (the UA registrar)

**Reviewed:** `docs/plans/2026-09-26-commitment-model-phase3-plan.md` against
`docs/specs/2026-09-26-commitment-model-phase3-design.md` (the spec), the parent
`2026-09-23-commitment-model-design.md`, phase 2's `2026-09-24-commitment-model-phase2-design.md`,
`CLAUDE.md`, and the code on branch `p3-registrar` (plan commit `14b2aff`, code as at `eb10c1b`).
**Date:** 2026-09-26. **Reviewer:** pre-execution reviewer (no cargo run; every name was grepped).

## Verdict

**Ready to execute, with the fixes below applied.** Two conditions stay open and are the
controller's:

1. The p2 judge-once fix must be merged into `p3-registrar` before R1. R3's end-to-end test is the
   pin: without the fix it files neither card.
2. R4 (the app) is blocked on R0. R1–R3 are not; see I1.

One Critical finding (the plan contradicted the controller's problem-4 ruling), three Important
and ten Minor. Every Critical and Important finding is fixed in the plan. The spec now has an
"Amendments (2026-09-26, plan review)" section covering problems 1–4 and the task order.

## Controller rulings, checked

| Ruling | Plan before review | Now |
|---|---|---|
| (1) One calendar key per term | R2-a, sound: a spring fetch in November would otherwise end every fall class | Unchanged; spec amendment 1 |
| (2) A dropped course dated the day before the fetch | R2-c, sound: with no instances, `detect_changes`' ended path took `until` = the note's `until`, and `note.until <= end` filed nothing | Unchanged; spec amendment 2 |
| (3) Registrar series out of the instance map | R2-d, sound: `instances_map` makes instances the only busy time inside the 28-day horizon | Unchanged; spec amendment 3 |
| (4) R24 stays `quinn`; the p2 fix changes change detection | R3-c wrote as `agent:commitments` | Reverted to `quinn` via `dashboard` everywhere (C1); spec amendment 4 |
| R0 not done; "only R4 needs `UA_ROWS_PATH`" | R1 depended on R0 (the fixture) | Partly true; fixed (I1); spec amendment 5 |

**On the R0 claim.** It is true that `UA_ROWS_PATH`, the term-selection call and R4-h's headers are
R4's only. But as written, R1 `include_str!`s a fixture that R0 step 8 wrote, so R1 could not
compile before R0. And R0 can still reach R1–R3 in three ways:
- the same facts in other key names or another envelope: `parse_banner`, the fixture, and four test
  bodies that build or edit Banner rows by key (three in R1, the end-to-end test in R3);
- the spring suffix (R0 question 3), which is `SCHOOLS`' `(1, "10")`;
- a rows call with no per-meeting `startDate`/`endDate`, such as shape (a). Then `first`/`until`
  are unknown, and R2-b (the term in play), R2-h (the ask gate) and R3-d (the refresh flag) have
  nothing to read. That is a re-plan, not a `parse_banner` edit.

So R1–R3 can proceed first on the documented shape (b), as a provisional contract. The plan now
says so, moves the fixture into R1, and names the re-plan trigger.

## Findings

### Critical

**C1. R3-c contradicted the controller's problem-4 ruling. Fixed.**
The plan wrote the R24 notes as `agent:commitments` (`ctx.with_actor(CARD_ACTOR)`) and its tests
asserted that. The ruling keeps spec §3 (`quinn` via `dashboard`); the p2 fix makes change detection
count only a later `set` by `quinn`. Changed:
- Global constraint 10, the collected R3-c, and R3-c in R3.
- `run`'s call is now `confirm(vault, &input, today, ctx, journal)`, and its doc comment.
- R3's interfaces: `--actor quinn` is used, `CARD_ACTOR`/`with_actor` are no longer consumed.
- The test is renamed `matched_rows_are_confirmed_by_the_student_and_the_rest_proposed`. It now
  asserts `"actor": "quinn"` with `"via": "dashboard"` and no `agent:commitments` record.
- The end-to-end test's doc names the p2 fix as the reason the cards come.
- R3's commit message, R4's `registrar_argv` doc and test doc, R6's CLAUDE.md sentence, and the
  closing checklist's actor.
- Spec problems item 4, and the fidelity ledger's §6 item 3.
- A new **Preconditions** block at the top: the p2 fix is merged before R1. R3 step 4 stops and
  reports if `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank` files
  neither card, instead of changing the actor or the assertion.

Checked against the code: `journal::human_set` (`journal.rs:271–282`) counts a `create` by `quinn`.
`detect_changes` guards `meets`, `where` and `until` with `is_human_set` (`commitments.rs:3549`,
`3556`, `3566`, `3615`). So without the fix, both of that test's assertions fail, which makes it a
real pin. `write::write_literals`' judge-once guard applies only under `opts.judged`, so a card
settlement's write on a `quinn`-created note is not refused.

### Important

**I1. R1 could not start before R0, and R1-a overstated the containment. Fixed.**
Changed:
- R1 now writes the fixture (R1 Files, Step 1 and Step 5's `git add`).
- R0's Files, Output, step 8 ("check, rewrite only if the shape differs") and step 10.
- Constraint 4 and the ownership table: R1 depends on the p2 fix, and R0 runs any time before R4.
- A new "R0 and R1–R3" paragraph lists the four Banner-keyed test bodies and the re-plan trigger.
- R1-a, in both places.
- R6 step 6.4's `git log` expectation: R1's commit, and R0's if it rewrote the fixture.

**I2. `Overview.to_json` gaining `registrar`/`registrar_proposals` was untested in Rust. Fixed.**
R5's JavaScript reads `week.registrar` and `week.registrar_proposals`, and `wizard-check.py` fakes
`your_week`. A forgotten `to_json` line would have passed every test. R3's
`the_overview_names_the_term_and_the_registrar_proposals` now asserts both keys in `o.to_json()`.

**I3. The p2 fix was not a stated precondition. Fixed** (with C1): the Preconditions block, R1's
precondition line, and the stop rule in R3 step 4.

### Minor

- **M1. Fixed.** `capture`'s no-registrar path returned `closed: true` without closing the window.
  It now calls `lms_link::close_and_wipe(app)` first.
- **M2. Fixed.** The lab test was a substring test (`contains("lab")`), so "Collaborative
  Seminar" became a lab. It is now a whole-word test (`lab`/`laboratory`), with an assertion added
  to `a_laboratory_is_titled_lab_and_a_repeated_pattern_is_one_meet`.
- **M3. Fixed (as an R0 question).** `ureq` drops the `cookie` header on **every** redirect, same
  host included (`ureq-proto-0.6.4/src/client/redirect.rs`). That means no cookie leaks to Okta,
  but a Banner call that redirects inside `bannerssb.ua.edu` would read as "not signed in". R0
  question 5 now records redirects.
- **M4. Fixed.** `your_week_names_the_registrar_only_at_a_school_that_has_one` now removes its
  scratch vault.
- **M5. Fixed.** R2(b) now also updates `proposals`' doc comment, which described the old
  google-first order.
- **M6. Not fixed; for Quinn.** D8's refresh rule approximates the spec. D8 says "Refresh" when
  today is on or after the start of a term the file does not hold. R3-d uses `term_for`, which
  switches on 1 January and 1 June, so the button takes the primary style before spring starts and
  all summer. Before the term list is known, the start of an unfetched term cannot be computed.
  R5-c also shows the "Refresh" label whenever any term is held, which reads D6 literally. Both
  are reasonable, but neither is in the spec's words.
- **M7. Not fixed.** `app/tests/registrar.rs` leaves its `qo-registrar-data-*` profile folders in
  `%TEMP%`, as `week.rs`'s tests already do.
- **M8. Not fixed.** R4 is the largest task: 8 files and about 250 lines of Rust. Suggest a
  controller look after edit (e), before `main.rs`.
- **M9. Note.** The byte checks diff against `eb10c1b`. They stay valid after the p2 merge only if
  the fix touches no file under `engine/tests/fixtures/`. Check at the merge.
- **M10. Note, accepted.** `commitments --registrar` runs under `vault_io`, but a slot's `rank`
  does not take it around `run_child`. This is the same trade as phase 2's `--confirm` (Q9-a).

## Correctness against the code

Grepped and read, at `eb10c1b`:
- `Series`, `Rule`, `Meet`, `Instance`, `Ended`, `SeriesFile`, `Codes { table, names }`,
  `Commitment` (11 fields, as R2's test builds it) and `Commitments` (`Default`).
- `classify`, `eligible`, `class_course`, `precedence` (3 callers plus `proposals`' `rank`),
  `instances_map`, `refresh_series` (fn at 2079, doc from 2061, `configured` at 2100),
  `days_since`, `add_days`, `ENDED_DAYS`, `detect_changes` (7 arguments) and
  `SERIES_KEY_PREFIXES` (3364).
- `create_confirmed_as`' body chain and the slug file names (`cs-100.md`, `cs-100-lab.md`).
- `stored_proposals`, `ConfirmInput`, `confirm` → `ConfirmReport { created, warnings }`,
  `proposal_value`, `Overview`/`overview`.
- `cli::commitment_passes`: `fresh_keys` at 834, the ask guard at 860.
- `cli` test helpers: `p16_vault` (journal seeded 09-06), `rank_p16`, `p16_day`, `checks`,
  `md_names` and `P16_MONDAY`. `main.rs`'s `Commitments` arm.
- `lms_link`: `WINDOW`, `LmsSession`, `session_dir`, `open_window_at`, `cookie_url`,
  `session_agent` and `sweep_stale_sessions` (both private today, and R4 makes them `pub`),
  `close_and_wipe`, `wipe_session`, `wipe_session_on_exit`.
- `scaffold::{Curated, CAMPUSES, curated}`, `commands::{now_in (pub(crate)), state_inner,
  attach_scheduler}`, `scheduler::engine_exe`, `NoConsole::no_console(&mut self) -> &mut Self`,
  `journal::now_ts`, and `ConsoleState`'s `vault`, `data_dir`, `vault_io`, `note_write` and
  `set_test_today`.
- `console.js`'s `h` (escapes `& < > " '`), `EL`, `stateView`, `paint`, `current.pendingOrder`,
  `confirmWeek`, `showRefusal`, and the four phase-2 functions R5 edits.
- The `index.html` ids and `WEEK_FAKE`'s `gcal-series:chess` row.

The test code traces through the real functions to the stated values:
- R1: the fixture gives 4 kept rows, 1 without a time and 1 across midnight, and `where` is cut to
  80 characters.
- R2: the configured, ended, two-term and twin cases. The Google twin classifies as `class
  cs-100`, and tier 0 beats tier 1.
- R2: the end card lands at 09-09, and the ask gate's four dates hold.
- R3: `confirmed` is 2 and then 0 on a repeat. In the end-to-end test, 09-07 is day 2, so the lab
  gets an end card at 09-06 and the lecture a change card for `where`.
- The binary tests: clap's `requires`/`conflicts_with` exit 2 before anything is written.

They should compile as written:
- Two-phase borrows cover `rows.push(rows[1].clone())`.
- `super::tests::FIXTURE` is reachable from the sibling test modules.
- `Option::is_none_or` is stable.
- ureq 3's `send(String)` and `call()` return the same type.
- The command counts check out by script: **47 / 29 / 66 today → 50 / 29 / 69**.
- Per-module `#[tauri::command` counts: commands 32 and account 15 include the non-registered
  helpers. R6's per-module expectations (27, 4, 3, 14, 14, 5, 2) must therefore be re-derived by
  the script, as R6 step 1 already says.

## Invariants

| Invariant | Holds? | Where |
|---|---|---|
| Journal-first writes | Yes | R24 goes through `confirm` → `create_confirmed_as` → `write::create` |
| No parse-and-redump | Yes | No note is re-emitted; the series file is generated state |
| `rank` reaches no network or model | Yes | R2's `commitment_passes` edits are pure; `cloud_contract.rs` runs in R3 step 4 |
| `--registrar` makes no network call | Yes | `registrar_makes_no_network_call` (loopback plus `cloud.yaml`) |
| `commands.rs` and `registrar.rs` compute nothing | Yes | Term, signed-out test and parse are engine functions; the app only looks up the curated row and substitutes `{term}` |
| No stored password or cookie | Yes | The jar lives on `capture`'s stack; no error formats it; the bytes go to a temp file deleted after the child (source-scan test) |
| Registrar window: no capability, no IPC | Yes | It is `lms-signin`; `capabilities/default.json` names only `main`; `no_capability_names_the_sign_in_window` |
| Frozen fixtures untouched | Yes | R1, R2 and R6 diff checks; the registrar fixture is new and not frozen |
| Command counts: console 47→50, wizard 29 | Yes | R4's source test and R6's script |
| XSS in `console.js` | Yes | Every interpolation goes through `h()`; messages use `textContent`; the label is a curated constant |
| The wizard keeps nine panels | Yes | Constraint 17; R4's test pins no `registrar::` in `run_shell` |

## Cross-task table (shared files and interfaces)

| Producer → consumer | Shared file or interface | What the producer makes | What the consumer relies on |
|---|---|---|---|
| R1 → R2, R3 | `engine/tests/fixtures/registrar/banner-ua-registration.json` | Six invented rows, shape (b) | Four kept (40001, 40002, 40003, 40006), one without a time, one across midnight; term 202640; 08-19 → 12-04 |
| R1 → R2, R3 | `registrar::tests::FIXTURE` (`pub(crate)`) | The fixture text | `d5_d7_tests` and `run_tests` read it via `super::tests::FIXTURE` |
| R1 → R2 | `registrar::{CALENDAR_PREFIX, parse_banner, calendar_key}` | `registrar:` keys, `Series` with `calendar = registrar:ua:<term>`, no instances, `last_seen: None` | `classify`'s arm, `precedence`, `instances_map`, `refresh_series`, `SERIES_KEY_PREFIXES`, `create_confirmed_as` and the ask gate all test the prefix |
| R1 → R2 | Title `"<SUBJ> <NUM>"` plus `" Lab"` | Set from the schedule type | `registrar_class` reads kind from the `" Lab"` suffix; `registrar_course` reads the leading code |
| R1 → R3 | `registrar::{school, term_for}` | The school table and the term rule | `run`'s unknown-school check and `status`' `current`/`refresh` |
| R1 → R4 | `knowlu_engine::registrar::{term_for, looks_signed_out}` | Engine functions | The app's `capture` (the term before the first call; "not signed in yet") |
| R2 → R3 | `commitments::registrar_course`, `refresh_series` under D5, R2-c's `ended`, R2-g's fresh keys, R2(f)'s body sentence | Course matching, merge and end dating | `run`'s `mine`/`proposed`; the end-to-end end card (09-06) and change card; the note body `run_tests` asserts |
| R2 → R3 | `engine/src/commitments.rs`, `engine/src/cli.rs` | Edits to `classify`/`precedence`/`refresh_series`/`commitment_passes` | R3 adds `Overview` fields and `commitments_registrar`: disjoint regions of the same files, strictly sequential |
| p2 fix → R2, R3 | `detect_changes`' judge-once helper | Only a later `set` by `quinn` counts | R3's end-to-end test (notes created by `quinn`); R2's end-card test (empty journal, unaffected) |
| R3 → R4 | `knowlu-engine commitments --registrar <file> --school <s> [--today] [--via]` | JSON report; exit 2 plus a stderr message; nothing written on exit 2 | `run_file`: `result` from stdout, the `error` text from stderr (`"no class with meeting times"`) |
| R3 → R4, R5 | `Overview.{registrar, registrar_proposals}` in `to_json` | `{school, held, current, refresh}` or `null`; `proposal_value` rows | `your_week` passes `week` through unchanged; R5's `renderRegistrar` reads both |
| R4 → R5 | `open_registrar_window` → `{ok, error, opened}`; `capture_registrar({view})` → `{ok, error, closed, result, state}`; `close_registrar_window` → `{ok, error}`; `your_week.registrar_label` | Envelopes | `bindRegistrar`'s branches on `ok`/`closed`; `result.rows/confirmed/proposed`; the label gates both buttons |
| R4 ↔ `lms_link.rs` | `WINDOW`, `LmsSession`, `session_dir`, `open_window_at`, `cookie_url`, `close_and_wipe`, `sweep_stale_sessions` and `session_agent` (both made `pub`) | Unchanged behaviour | One sign-in window for either purpose; its existing pins cover the registrar |
| R4 → R6 | `app/src/main.rs`'s console `generate_handler!` | +3 commands | R6's recount: 50 / 29 / 69 |
| R5 → R6 | `index.html` ids, `console.js` functions | The two buttons and the marker | anatomy §2 and §3.15, and the README text |
| R0 → R4 (and R1–R3 on a shape change) | The D3 amendment | The call list, headers, redirects and spring suffix | `UA_ROWS_PATH` and the calls, R4-h; possibly the fixture, `parse_banner`, four test bodies and `SCHOOLS` |

## Placeholders, task size, edits to existing assertions

- **Placeholders:** one, deliberate: `const UA_ROWS_PATH: &str = "…";` in R4, filled from R0's
  amendment. R4 is blocked on R0, so it is a scheduled data dependency, not a gap.
- **Task size:** R2 has eight code edits across `commitments.rs` and `cli.rs`, each under 60
  lines. R4 is the largest (M8). Both are acceptable under constraint 16.
- **Existing test assertions:** none is edited.
  - R6 adds one assertion to `site.rs`'s existing test.
  - R2 changes `precedence`'s type but keeps every non-registrar order, so the P8–P16 tests (for
    example `by_key_takes_one_record_per_key_google_first_then_calendar_key` and
    `twins_under_two_keys_are_one_proposal_by_google_first_precedence`) should not move. R2 step 4
    says to stop, not edit, if one does.
  - The phase-2 overview test checks keys by presence, so the new keys do not disturb it.
  - The wizard's nine-panel test is untouched.

## Spec edits made

`docs/specs/2026-09-26-commitment-model-phase3-design.md` gains **"Amendments (2026-09-26, plan
review)"**, with five items:
1. one key per term;
2. a dropped course dated the day before the fetch;
3. registrar series out of the instance map;
4. R24 as written, with change detection counting only a later `set` by `quinn` (answers §6
   item 3);
5. task order, with R0 before the app task only and the re-plan trigger.

No other line of the spec was changed.
