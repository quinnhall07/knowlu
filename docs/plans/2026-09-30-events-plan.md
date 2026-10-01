# Events (MVP): required events enter the plan, with accept and decline (implementation plan)

**Date:** 2026-09-30. **Base:** branch `events`, worktree `.claude/worktrees/events`, cut from `main`
at `4aa6560`, now at `2d5298a` ("Merge origin/main (Gmail connect #26) into events"), which already
contains `main`'s `199cd1f` and so `app/tests/handler_lists.rs`. The base carries #23 (ruling 11's
`student` token: `journal::HUMAN_ACTOR`, `console_ctx(vault)`, `write`'s human gate,
`engine/tests/human_actor_literal.rs`), #25 (M1 grades) and #26 (Gmail connect), as well as #12 (the
`unsure` word and the `event-check` card) and #14/#16 (the commitment model's phases 1–2).
**Governs:** `docs/specs/2026-09-29-events-design.md`, signed by Quinn on 2026-09-29 with every
recommendation accepted: Q1 (c), Q1a (i), Q1b (i), Q2 (a), Q3 (a), Q4 (a), Q5 (c). Above it: the
cloud design's Amendment 2026-09-29, ruling 10 (the MVP holds events; its parenthetical was reworded
at signing: required events enter the plan as a one-off commitment).
**Stage:** MVP, HANDOFF §3 item 4. **Implements:** parity row P4 of
`docs/notes/2026-09-29-vision-program.md`, and P6 (c) (the 14-day expiry of opportunity proposals,
delivered by the spec's D3).
**Status:** draft, revised for the plan review's nine findings (§10), for Quinn's answers to
PQ1–PQ4 of 2026-09-30 (§11), for the second review's four findings (§10, second round), and for
Quinn's final PQ3 answer, (b-prime), which replaced (c) (§11, item 3). PQ1, PQ2 and PQ4 are applied
to the tasks. PQ3 (b-prime) cannot be built as approved: the all-day lane cannot draw a carried
all-day date from the carry's ledger line plus what is stored, and Quinn ruled that no further write
be invented. So no task builds it, and it waits on Quinn (§4, PQ3). Nothing here is built yet.

## 1. Goal, and what must not move

A student asks "what's next?" and a required campus event is part of the answer. Every judged event
the student should decide on becomes one Accept/Decline card. Accept puts the event on the day it
happens: a hard commitment for a required event, a soft one for an optional event, the all-day lane
for an event with no hours to take, and a "Register" task when the feed says registration is
required. Decline drops the event, and its series, for good. An unanswered card expires and says
nothing. VISION check: the day's answer is more complete (the event is in it) and more honest
(capacity stops counting a fair's hours as free). Commitments 4 and 5 are applied as they stand:
nothing a model inferred enters the student's hard time without a click.

These must not change:

- **Frozen references (rule 2).** None of the eight Python references is regenerated.
  `golden-today-full.md:5` keeps `**Events: 3 in today's digest**`: `vault-full` has no
  `event_cards` key and no `obligation` verdict, so its digest path stays byte-identical (spec F3,
  D6). The three `surface-today-*.json` references are not regenerated either (§3, P8).
  `git diff --stat main -- engine/tests/fixtures` stays empty at every gate.
- **Contracts used, not changed.** `write.rs`, `journal.rs` (`VIAS`, `OPS`), `yamlemit.rs`,
  `yaml.rs`, `pystr.rs`, `ledger.rs`, `ids.rs`, `provenance.rs`, `sync.rs`, `entitle.rs`,
  `wincred.rs`, `reconcile.rs`, `app/src/{credentials,account,updates}.rs`. The one contract-list
  file this lane edits is `engine/src/approvals.rs` (spec §8).
- **The ledger contract.** `state/events-seen.md` gains no line shape; `VALID_VERDICTS` and
  `ANSWER_VERDICTS` (`eventledger.rs`) are unchanged. Quinn's PQ3 answer, (b-prime), would add one
  optional `from:` field to a carry-written answer line. It is blocked (§4), so no task edits
  `eventledger.rs` until Quinn rules.
- **The existing settlement arms.** The digest, `calendar-event`, `task`, `amend`,
  `commitment-check`, `commitment-ask` and `event-check` (without `instances:`) arms behave
  byte-for-byte as today. The digest arms stay forever.
- **`rank` never calls a model,** and the same input gives the same bytes. No new JSON is written.
- **Rule 1.** No new `"quinn"` or `"student"` literal anywhere, tests included.
  `engine/tests/human_actor_literal.rs` stays green.
- **No new command, flag or handler.** Neither `generate_handler!` list in `app/src/main.rs`
  changes. `app/src/commands.rs` is not edited.
- **Privacy.** `site/` is not edited and `PRIVACY_VERSION` does not move (spec §7). No new
  telemetry row shape (D12).
- **The cloud.** No migration, no function. B1 (T9) runs a harness on `j-events` and changes nothing
  that ships from this branch.

## 2. What changed since the spec was written

The spec was verified on `97dc27b`. This plan re-read every file and function it names on the
`events` worktree (`4aa6560`, and again at `2d5298a` after `main`'s `199cd1f` was merged in) and on
`j-events` (`fcad6d4`). Briefs cite functions, never line numbers.

**Merged since the spec (its §9 rows are met):**
- `human-actor` (#23). `console_ctx(vault)` now returns `Result<WriteContext, String>`, and every
  public `write` entry point takes a private human gate: a human actor that is not the vault's own
  token (`journal::read_human_actor`) is refused. Consequence for every test in this lane: a test
  that writes as the student takes its actor from `journal::read_human_actor(&vault)` (or the app's
  `console_ctx`), never a literal; an `agent:` actor is not gated (§5, rules).
- `m1-grades` (#25). `ids::NOTE_FOLDERS` now has eight entries, `grades` among them. Test 25 checks
  membership only, so it is unaffected.

**Code facts that differ from the spec's text:**
- **Line drift, same functions.** `executor_ctx`, `decide_inner`/`decide_in` and the in-process
  `process_approvals` call in `app/src/commands.rs`; `create_task_inner`'s `domain`/`due` lines;
  `write::delete`; the console's `generate_handler!` list. Every function the spec names exists with
  the behaviour it describes; `eventemit.rs`, `events.rs`, `approvals.rs`, `eventledger.rs`,
  `eventroster.rs`, `enrich.rs`, `commitments.rs` and `surface.rs` match the spec's lines.
- **Two Approve/Reject sites in `console.js`.** The deck's card (beside the `commitment-ask`
  switch) and the Decisions view's row both render `Approve`/`Reject`. Spec §6.3 names one; T5
  covers both.
- **`DiscoveredEvent` has no all-day flag.** Shape is inferred from `start`/`end`. The ICS reader
  gives an all-day event a midnight-to-midnight span and a missing `DTEND` one hour;
  `eventemit::when_label` already treats an end at midnight as belonging to the day before. P4 makes
  one classifier for every caller.
- **§5.1's `instances:` has no `url`, but §4.2's register task body needs the URL.** P5 adds it.
- **`surface::ComingUp` has no field for "Accepted",** and adding one would change the serialised
  state of every Coming up entry, hence the `surface-today-*.json` references. P8 makes the field
  absent when false. The console has no renderer for it; T5 adds one (spec §6.3 did not list it).
- **The roster drops `series_uid` and the all-day shape.** `eventroster::read_roster` reads back
  `HH:MM–HH:MM` on one day and never a series key. So `surface` can learn of an accepted all-day
  event only from an archived card's `instances:`. A carried all-day instance (D4) is on no card.
  On a run where every feed fails, `rank` swaps its candidates for `read_roster`'s (the
  `feeds_failed` branch in `rank`'s events pass). `read_roster` builds each event through
  `DiscoveredEvent::normalized`, so its `series_uid` is its own uid, not a series key. A later
  instance (`localist:77:3`) then matches no answered series (`localist:77`), and an event whose
  series is its own uid is the uid its card already lists. So neither `inherit_series_answers` nor
  the carry acts on a later instance on that run (T2b.2 states this as intended). *Corrected
  2026-09-30:* an earlier draft said these events carry no `series_uid`.
- **The roster's read-back is lossy, and on a feed-failure run the card emitters read it.** In
  `rank`'s events pass, `candidates = read_roster(…)` replaces the candidates when every feed failed
  and nothing was pre-filtered in, and that same `candidates` is then passed to
  `inherit_series_answers`, `write_roster`, `emit_digest`, `emit_event_checks` and
  `relevant_events` (checked on `2d5298a`). `read_roster` rebuilds each event from one
  `HH:MM–HH:MM` line on one day with `source: "roster"`: an `end <= start` becomes `start + 1h`, so a
  zero-length event, a past-midnight event (10pm–1am reads back as 10pm–11pm) and an all-day one
  (00:00–00:00 reads back as 00:00–01:00) all come back as a one-hour timed event, and `url`,
  `registration` and `registration_deadline` come back empty. A card's `instances:` built from such
  an event would book an hour the feed never claimed, or the wrong hour, and never-ask-twice makes
  that card the only one the event gets. So no `event-accept` card and no `instances:` entry is ever
  built from a `source: "roster"` event (T2a.1b, T2a.2).
- **A carried instance of an `event-accept` series has no verdict, so it is not in Coming up.**
  Coming up lists only what `eventroster::relevant_events` keeps: a uid with an `obligation` or
  `opportunity` verdict and no `declined` line. A later instance of an answered `event-accept`
  series is never judged (T2b.3 stops `judge_roster` from judging it), and the carry writes ledger
  lines only for declines. So a carried instance, timed or all-day, is absent from Coming up, and
  no "Accepted" mark can show for it. A timed one still has its commitment's block on its day. An
  instance judged confidently before the answer is still listed. A later instance of an answered
  `event-check` series differs: `inherit_series_answers` gives it the series' answer line, so it is
  listed, and under PQ1 (a) the carry also books it. Quinn's final PQ3 answer, (b-prime), gives
  each carried date an answer line attributed to the carry, which would list the `event-accept`
  kind too; its all-day lane half is blocked (§4, PQ3). (`eventledger::record_answer` takes only
  `obligation` or `drop`; (b-prime) widens that for the carry's line.)
- **`commitments::create_confirmed_as` picks its body from a `&'static str`.** The `kind: event`
  arm needs a computed line for the past-midnight true end. P6 says how it is passed.
- **`emit_event_checks`' window is not `horizon_start`.** It asks about `unsure` events starting in
  `[today, today + propose_horizon_days]`; the digest's `eligible_events` uses `horizon_start` (the
  registration deadline can open it earlier). P10 keeps each kind's window.
- **`enrich::labels_to_report` already maps a rejected `event-accept` card** (it has
  `judgment_kind: event`) to a `decision` row and an executed one to nothing. Test 24 pins this, and
  may pass on its first run.
- **No test pins the `event-check` closing sentence,** so §5.2's new sentence breaks no assertion.

**The T0 count.** Spec §12 records that Quinn signed before T0's count existed and that the number
is still recorded when measured. Nothing in the ultracode run or the signing packet records it. T0c
measures it, off the critical path (§5).

**`j-events` (`fcad6d4`, pushed, unmerged).** Five commits on a pre-Integrate base: the `unsure` word
(since merged to `main` as #12), a merge, the `event-4` prompt with migration
`20260922120300_event_decomposed.sql`, the B1 harness in `scripts/experiments/e1-decomposition/`
(`run.ts`, `arms.ts`, `stats.ts`, `model_row.ts`, `credentials.ts`, `event3_frozen.ts`, tests), and
two fix rounds. `arms.ts`' `deviceRecorded` records a refused reply as `unsure`, as the spec says.
**No B1 result is recorded anywhere.** The migration's timestamp sorts before `main`'s
`20260926000100` and `20260929000100`, so shipping `event-4` would be another out-of-order push
(PQ4). This lane's engine work does not depend on the branch (spec §10).

**Sibling lanes in flight (unmerged), and the files they share with this one:**

| Lane | Shares with events | Events task | Resolution |
|---|---|---|---|
| `m2-editing` | `app/static/console.js`, `app/tests/static_assets.rs`, `app/tests/commands.rs` (its T4a, T4b), `engine/src/surface.rs` (`describe` only), `engine/src/lib.rs` (`pub mod profile;`), `engine/src/eventroster.rs` (its `read_dropped`; events does not edit this file) | T5, T7, T4b, H1 | Per M2's plan §6, events takes M2 in after M2 merges: T5 and T7 wait for it (P12). The engine tasks touch other functions, so they run now. |
| `gmail-connect` (merged, #26, `199cd1f`; already in `events` at `2d5298a`) | `app/static/console.js`, `app/tests/static_assets.rs`, `engine/src/enrich.rs` (D4, D7 and the transport stop; events adds only test 24 to its test module), `app/src/main.rs` (its three commands; events adds none) | T5, T4 (test 24) | Nothing left to take in. Every events task starts on a base that has Gmail's code, so test 24 is written against `enrich.rs` as Gmail left it. |
| `p3-registrar` (Pilot) | `engine/src/commitments.rs` (`create_confirmed_as`' body branch), `engine/src/cli.rs`, `console.js`, `static_assets.rs` | T1b, T4, T5 | Unchanged from spec §9: p3 rebases after events, keeping both body arms (events' `kind: event` arm tested first), both `cli.rs` passes and both label switches. |
| `two-desktop` (Launch) | its spec's producer table | none | Hand-off only (spec §9). |

`app/src/main.rs` and `app/src/lib.rs` need **no** hand-off from this lane. Gmail moved the console's
list from 47 to 50 and M2 adds five more; events adds none. Gmail's `app/tests/handler_lists.rs`
pins the count and is already in the `events` branch (`2d5298a`), so every gate runs it (§7).

## 3. Decisions this plan makes

- **P1. Smaller tasks, with the spec's ids kept.** Each spec task with two separable pieces is split
  with a suffix, so the fidelity ledger still maps to spec §13. Each dispatch should land about 80
  lines or fewer of non-test code; one that clearly exceeds it is split before it starts.
  - T1 becomes T1a (`eventaccept.rs`) and T1b (the switch and the body line).
  - T2a becomes T2a.1a (the shared selection as a refactor, `emit_event_checks`' bytes unchanged),
    T2a.1b (obligation cards, `instances:` on both kinds, never-ask-twice across both) and T2a.2
    (opportunity cards behind the switch, their order and expiry).
  - T2b becomes T2b.1 (`answered_series` alone, the spec's "first commit"), T2b.2 (the
    ever-written set, D9's rebuild and the carry over `event-accept` series), T2b.2b (PQ1 (a):
    `accepted_check_series` and the carry over the union, its own commit) and T2b.3 (the
    `judge_roster` call site).
  - T3 becomes T3.1a (the approved `event-accept` arm and the retry), T3.1b (the rejected arm and
    the expiry test) and T3.2 (the extended `event-check` arm and the shapes).
- **P2. T0's count is measured off the critical path.** Q1, Q1a and Q1b are signed, and Q1b's
  fallback holds whatever the number is. T0c (researcher) counts the shapes from the Alabama feeds
  at any time before T10. The number goes into the SDD ledger and, through docs-keeper, into spec
  §12's Q1 note. It blocks nothing.
- **P3. Hand-offs land when the next task needs them** (§7). `engine/src/lib.rs` is a single-owner
  file (HANDOFF §2): the controller adds `pub mod eventaccept;` with a one-line module file before
  T1a, and `pub mod eventcarry;` with one before T2b.1, each green on its own.
- **P4. One shape classifier.** `eventaccept::shape(start, end)` returns one of four shapes, and the
  builders, the settlement and `surface` all call it. The rules extend `when_label`'s:
  - `last_day` is the day before `end` when `end` is at 00:00 on a later day, else
    `max(end.date, start.date)`;
  - **all-day**: `start` and `end` both at 00:00 (one day or several);
  - **multi-day**: `last_day` after `start`'s day and not past-midnight (below): drawn in the
    all-day lane on each day, as spec §4.2 says;
  - **past-midnight**: `end` on the next day at or before `start`'s time of day, including an end at
    exactly 00:00 the next day (a 10pm–12am event): a commitment to 23:59;
  - **timed one-day**: `start < end` on one day;
  - **zero-length** (`end <= start` on one day): no commitment, drawn in the all-day lane on its
    day (PQ2, answered 2026-09-30: Knowlu invents no hours).
- **P5. `instances:` entries carry `url`.** Each mapping is `uid`, `title`, `start`, `end`,
  `location`, `url`, `registration`, `registration_deadline`, free text through `judge::one_line`
  (title 200, location 120, url 500). The register task's body needs the URL (spec §4.2), and the
  card's own payload is the only input the settlement has (D10).
- **P6. The past-midnight true end is passed in the commitment mapping.** The builder adds an
  `ends:` key holding the clock label (`1am the next day`, or `midnight`). Only the `kind: event`
  body arm reads it; `proposed()` ignores unknown keys, and the key never reaches the frontmatter.
  The body is "You accepted this from your campus events." plus, for that shape, " Ends at <ends>."
- **P7. An `event-check` Accept books `level: hard`.** The student's answer word is `obligation`,
  so D1's level for an obligation applies.
- **P8. "Accepted" in Coming up.** An event is accepted when its uid is listed on an archived
  `executed` card's `instances:` (`event-accept`, or `event-check` with `instances:`), or is the
  `source_uid` of a `kind: event` note in `commitments/`. The second test is for an instance that is
  in Coming up and was booked by the carry: one judged confidently before its series was answered,
  or (PQ1 (a)) a later timed instance of an accepted `event-check` series, which Coming up lists
  through its inherited answer line. A carried `event-accept` instance with no verdict is not in
  Coming up (§2), so the mark never reaches it. PQ3 (b-prime) would list it, and is blocked (§4).
  `ComingUp` gains `accepted: bool`, serialised only when true, so every existing reference and
  `today.md` are byte-identical. `render::coming_up` (the page) is not changed.
- **P9. The carry writes commitments only.** Spec §4.2: a series writes one register task, for its
  primary instance. So the accept carry never writes a task, and test 20a's register-task clause
  asserts that deleting the primary's task is never undone.
- **P10. Each kind keeps its window.** The selection is one function over a verdict-to-card table,
  and the table row carries the window: `unsure` keeps `emit_event_checks`' start window exactly;
  `obligation` and `opportunity` use the digest's `eligible_events` rule (`horizon_start(e) <= today`
  and a start not before today).
- **P11. `HANDOFF.md` is the controller's,** updated in one batch at the MVP milestone (the CI
  minutes rule). T8 edits the reference documents only.
- **P12. Events takes M2 in by merge, not rebase, and is not pushed before review.** A merge keeps
  the commit hashes that Checkpoint B's, R1's and B2's reports cite, and it is what M2's own T0 did.
  The M2 plan's "rebases" is read as "takes `main` in after M2 merges". The `events` branch is first
  pushed only after B2 closes (every Sonnet-written commit has had a `reviewer` pass by then), and
  only with Quinn's go, since it is a code push. Commits after B2 (M, T5–T7, fixes) are pushed only
  after W closes.
- **P13. Carry tests at two levels.** Tests 19, 20 and 20a (spec T2b) drive `eventcarry::run`
  directly in T2b.2, as unit tests. Spec tests 19 and 20a are stated against `rank`, and what the
  unit tests cannot prove is the order of the passes in `rank` (`inherit_series_answers`, the carry,
  the roster, the cards). So T4 repeats both through a full `rank`
  (`rank_rebuilds_declines_from_a_rejected_card`, `rank_twice_never_rebooks_a_deleted_carried_note`),
  with the spec's assertions unchanged.
- **P14. What "the same input writes the same bytes" compares.** `write::create` gives every note
  without an `id:` a fresh random one (`ids::new_id`, OS entropy), and every journal record carries
  its wall-clock time. Both are contracts (the opaque id; the journal), so no test makes them
  deterministic. A determinism check (spec test 4, and W's check of spec §6.1's invariant) compares:
  the same file names, in the same order; each card's bytes equal once its `id:` line is removed;
  and the same `proposed` ledger lines. Journal timestamps and `id:` values are left out of the
  comparison, and the test says so in a comment. This is how spec test 4's "identical bytes" is
  read; the assertion is not weakened anywhere else.

## 4. Open questions, and Quinn's answers (2026-09-30)

Asked one at a time, each with its context, and answered by Quinn on 2026-09-30. Each answer is
recorded under its question; the options are kept as they were asked. One item is open: PQ3's
final answer, (b-prime), cannot be built as approved, and its blocker goes back to Quinn.

- **PQ1. Does an accepted `event-check` series carry, too?** D11 makes an `event-check` Approve an
  Accept when the card carries `instances:`, so the listed instances get commitments. But D4's carry
  reads only answered `event-accept` series (spec §6.1, and T2b's edge test "an `event-check` card
  not counted"). A later instance of that series is never asked again (`settled_series` closes it),
  and `inherit_series_answers` gives it an `obligation` line, so it shows in Coming up, but nothing
  books it.
  - *Recommend (a):* carry it. `eventcarry` gains a second reader, `accepted_check_series`, for
    `executed` `event-check` cards that carry `instances:`. The accept carry runs over the union.
    `answered_series` keeps its edge rule unchanged, so the emitter and `judge_roster` read what the
    spec says.
  - *(b):* build as signed. A later instance of a yes-it-applies series shows in Coming up and is
    never booked.
  - *Cost of (a):* one more reader and two tests in T2b.2 (contract-engineer). It widens a signed
    section, so it needs Quinn's word. *Before:* T2b.2.
  - **Answered 2026-09-30: (a), carry it.** An executed `event-check` card that carries
    `instances:` carries its series as an accepted `event-accept` card does. `eventcarry` gains
    `accepted_check_series`, the accept carry runs over the union, and `answered_series` keeps its
    edge rule (an `event-check` card is not counted). Applied in T2b.2b (split out of T2b.2 by the
    second review); the widened spec sections (§4.5, §6.1's `eventcarry.rs` row) get a dated note
    in T8.
- **PQ2. A zero-length event (`end <= start` on one day).** The spec writes a commitment only when
  `start < end`, and names no fallback for this shape. The ICS reader already gives a missing
  `DTEND` one hour, so this comes from Localist, Engage or a scraped page.
  - *Recommend:* the all-day lane, with no commitment, as for an all-day event: Knowlu invents no
    hours. `rank`'s own roster read-back gives such an event one hour. *Corrected 2026-09-30
    (second review):* an earlier draft called that hour "display only". It is not: on a
    feed-failure run the read-back events are the candidates every card emitter files from (§2). So
    no `event-accept` card and no `instances:` entry is built from a `source: "roster"` event
    (T2a.1b, T2a.2), which keeps the read-back's invented hour off every card.
  - *Alternative:* a one-hour commitment from its start. It blocks an hour the feed never claimed.
  - *Before:* T1a. If unanswered, T1a builds the recommendation; the other answer changes one arm
    of `shape`.
  - **Answered 2026-09-30: the all-day lane, no commitment.** Knowlu invents no hours. Applied in
    P4, T1a's shape arm and test, T3.2's test 16 and T4b's test 22a.
- **PQ3. Carried instances, timed and all-day, are absent from Coming up.** A later instance of an
  accepted `event-accept` series is never judged (T2b.3), and nothing gives it a verdict:
  `inherit_series_answers` reads `event-check` cards only, and the carry writes ledger lines only
  for declines. Coming up keeps only uids with a confident verdict (§2). So the carry books a timed
  instance (its block shows on its day) but the event is not in Coming up, and an all-day instance
  shows nowhere: no commitment, no card, no lane. The exception is an instance judged confidently
  before the answer; it stays listed, and P8 marks it "Accepted" when the carry booked it.
  - *Recommend (a):* accept the gap for the MVP and name it in anatomy §3.7 and §3.9. The timed case
    is still on the student's day; recurring all-day campus events are rare; and (b) writes a
    human-answer line that no human gave, with a word that can misstate an opportunity.
  - *(b):* T2b.2's carry also records the series' answer for each carried instance that has no
    verdict, or an `unsure` one, and no answer, through the existing `eventledger::record_answer`.
    No new line shape. But `record_answer` takes only `obligation` or `drop`, so an accepted
    opportunity series' instances would be recorded as `obligation`. The `by` value follows
    `settled_series`' rule (the answer's own `answered_by`, else `unknown`). This lists carried
    instances in Coming up and lets P8 mark the timed ones "Accepted". A carried all-day instance
    would be listed but still not in the lane and not marked. It is a design change to a signed
    section, so it needs Quinn's word.
  - *Cost of (b):* about 15 lines and two tests in T2b.2 (contract-engineer), and one more case in
    T4's rank-level test.
  - *Before:* T2b.2 (the carry is where (b) lands); T4b and T8 read the answer.
  - **Answered 2026-09-30, final: (b-prime).** Quinn first answered (c): list a carried instance at
    read time, with its provenance and no new write. (c) was blocked as approved, because nothing
    stored links a carried instance to its series and a carried all-day instance is stored nowhere.
    Quinn then replaced it with (b-prime):
    - For each carried date, timed and all-day, the carry writes **one** event-ledger answer line
      attributed to the carry, not the student: `by:agent:knowlu.carry` (the `agent:` prefix, so
      judge-once treats it as non-user), `from:<the answering card's id>` (the card holds the
      answer, its date and the series) and the series' real verdict (an accepted opportunity stays
      `opportunity`, so `record_answer` is widened to accept it).
    - Additive: one new optional field on a ledger line; every existing line stays byte-identical.
    - Coming up lists such a date like any judged event, labelled with its provenance (for example
      "Accepted · from your answer to the series on <date>", the date read from the card).
    - If the all-day lane still cannot draw a carried all-day date from that line plus what is
      stored, no further write is invented: the planner stops and reports it to Quinn as a blocker.
  - **What (b-prime) needs, checked against the code** (the `events` worktree at `8698e8d`):
    1. *The line shape* lives in `eventledger.rs`. `record_answer` writes
       `- <uid> · <title> · verdict:<w> · by:<actor>[ · jid:<uuid>] · answered <date>`, and
       `load_ledger` reads `by:`, `jid:` and `answered` from the line with its quoted `why:` removed.
       The carry's line would add ` · from:<id>` before `answered`. A card id is `appr_` plus ten hex
       characters (`ids::new_id`), safe unquoted. A new `FROM` pattern and a new `LedgerEntry`
       field would read it; every `LedgerEntry` literal in the crate ends `..Default::default()`, so
       none breaks.
    2. *The widening.* `record_answer` refuses `opportunity`, and two existing tests pin that for a
       human `by` (`record_answer_refuses_a_bad_word_actor_or_jid`,
       `an_answer_shaped_line_with_a_non_answer_word_does_not_supersede`). So the widening would be
       one entry point sharing `record_answer`'s line builder that takes `obligation` or
       `opportunity` only with the carry's actor and a `from:` id. `record_answer`'s own checks,
       `ANSWER_VERDICTS` and both tests stay unchanged.
    3. *Judge-once needs a read rule.* `load_ledger` lets an answer line supersede only an `unsure`
       verdict, and only with `obligation` or `drop`; its `answered_by` is not prefix-aware. So a
       carry line read first would hold against a later human answer for that uid, and a carry
       `opportunity` line would not settle an `unsure` instance. "A later human answer wins" and an
       `opportunity` carry over `unsure` would each need one new case in `load_ledger`
       (contract-engineer, xhigh).
    4. *Journal first.* No ledger line has a journal record. `journal::OPS` are note operations, and
       `record_answer`, `record_declined` and `inherit_series_answers` append to
       `state/events-seen.md` directly (`state/` never syncs). The carry can write a timed date's
       commitment journal first and its line after it. The line itself has no journal record unless
       a new record shape is added, which would be a further write and a `journal.rs` contract change.
    5. *Who calls it, and who reads it.* `eventcarry::run`'s accept carry would write the line for a
       carried instance that has no verdict, or an unanswered `unsure` one (`inherit_series_answers`'
       rule), after its commitment. `rank` runs the carry before `write_roster` (T4, step 2), so the
       same run's roster and `today.md` list the date. `surface::coming_up` reads the roster and the
       ledger, and would follow `from:` through `ids::build_index` to the archived card for its
       `executed_at` date. All of this can be built without a further write.
    6. *Dates carried from an `event-check` series already have a line, credited to the student.*
       Under PQ1 (a), `rank` runs `inherit_series_answers` before the carry (T4, step 1). For each
       instance of an executed `event-check` series with no verdict, or an unanswered `unsure` one,
       it writes an `obligation` line through `record_answer`, with `by` taken by `settled_series`
       from the ledger entry of the card's own `source_uid` (the student's actor, else `unknown`) and
       no `from:` (`eventemit.rs`, `settled_series` and `inherit_series_answers`). Item 5's rule then
       finds a verdict, so the carry writes nothing for those dates. With §4.5's row standing
       ("`inherit_series_answers`, which is unchanged", §9), the `agent:knowlu.carry` line, its
       `from:` and the provenance label reach dates carried from `event-accept` series only. A date
       carried from an `event-check` series stays credited to the student, with no `from:`, which is
       not "each carried date ... attributed to the carry, not the student". A label for such a date
       cannot follow `from:`; reading it through `settled_series`' card needs a stored link from the
       instance to its series, and the (c) check found none (the timed commitment holds only the
       instance's `source_uid`, T1a). The plan does not settle this; it is in the open choice below.
  - **Blocked as approved (found 2026-09-30): the all-day lane cannot draw a carried all-day date.**
    The lane needs the date's span, its first and last day. From the carry's line plus what is
    stored:
    1. **The line holds no event date.** Its `answered` date is the carry's run day.
    2. **The answering card does not list it.** `from:` leads to a card whose `events:` and
       `instances:` hold only its own instances, and a carried instance is by definition one the
       card did not list.
    3. **No note holds it.** A lane shape (all-day, multi-day, zero-length) gets no commitment (P4,
       Q1b, PQ2), so nothing in `commitments/` carries its dates.
    4. **The roster holds part of it, and not reliably.** Once the line gives the uid a verdict,
       `write_roster` writes it in the relevant section as one `HH:MM–HH:MM` line under its start
       day (`00:00–00:00` for an all-day day). That is not enough:
       - the section keeps only events that start today or later, so a multi-day event's second
         and later days are in no file once its first day has passed;
       - the line holds no end date, so a multi-day event's last day is lost even on its first day,
         and an end at or before the start cannot tell a zero-length event (the lane) from a
         past-midnight one (a commitment) or a multi-day timed one (the lane on each day);
       - a feed-failure run rebuilds the roster from `read_roster`, which turns `00:00–00:00` into
         `00:00–01:00`; after that, even a one-day all-day date reads as a timed hour.
    So the most the lane could draw is a one-day all-day carried date, on its own day, from the
    roster's raw line, until the next feed-failure run. Drawing a carried lane-shaped date on each of
    its days needs its span persisted, which is a further write, and Quinn ruled that out. The
    planner stops here and designs nothing around it. The same holds for an all-day later instance
    of an accepted `event-check` series under PQ1 (a). That series also has the attribution overlap
    in item 6, which is separate from the lane.
  - **Open, for Quinn.** Items 1–5 above (the line, the widening, the read rules, the call and the
    Coming-up provenance) can be built as approved; the lane half cannot. Either:
    - *(i)* build items 1–5 and leave carried lane-shaped dates out of the lane, with the gap named
      in anatomy §3.7. *Recommended:* every carried date gets its Coming-up listing, and recurring
      all-day campus events are rare. As item 6 shows, (i) gives the carry's line and provenance to
      dates carried from `event-accept` series only; or
    - *(ii)* Quinn names a write that persists a carried date's span.
    - *And, separately (item 6), for dates carried from an `event-check` series:* either accept the
      split, where they keep `inherit_series_answers`' line, credited to the student with no `from:`
      (and with no provenance label unless a stored instance-to-series link is found), or let the
      carry's line replace the inherited one for those series, which changes
      `inherit_series_answers`, a signed behaviour (§4.5's row, §9). The plan does not choose.
      *Before:* T2b.2b if the carry's line replaces the inherited one, otherwise with the (i)/(ii)
      ruling's task.
    - *Before:* T2b.2 if the ruling changes what the carry writes, otherwise before T4b. If it is
      still open at T2b.2, T2b.2 runs as planned: it writes no answer line for a carried date, and
      no test in it pins either way (§5, T2b.2's scope). The ruling's task is then added after T4b and before
      B2. No task builds (b-prime) until Quinn rules.
- **PQ4. B1's base, and shipping `event-4` if it wins.** B1 (T9) is signed (Q5 (c)) and makes paid
  OpenRouter calls through the key in Credential Manager `knowlu/dev/openrouter`.
  - *Recommend:* run the harness on `j-events` as it stands. Its `event-3` arm is frozen in
    `event3_frozen.ts`, and the pinned row is read from the branch's migrations, which T9 checks
    against `main`'s current event row before it runs. Record the result in `docs/reports/`. If
    `event-4` wins, shipping it is its own cloud PR after staging's pending `db push`: `main` merged
    into `j-events`, the migration renamed to a fresh timestamp (it has never been applied
    anywhere), then the review and the deploy. If it loses, the branch is closed as its header
    plans.
  - *Before:* T9. The spend is small (the harness has 26 seed cases, two arms).
  - **Answered 2026-09-30: as recommended.** B1 runs as it stands on `j-events`, after checking
    that its frozen `event-3` arm (`event3_frozen.ts`) matches `main`'s live event row. The result
    goes in `docs/reports/`. If `event-4` wins, it ships only through its own cloud PR. Applied in
    T9.

## 5. Tasks

**Rules for every task.**
- Write the named tests first, run them in the foreground, and see each fail for the stated reason
  before writing the code. A test's name may change; its assertion may not. A test that pins
  existing behaviour may pass first: the agent records which ones did in the SDD ledger.
- Each task ends with `cargo build --workspace` and `cargo test --workspace` from the root, dev
  profile, through `bash .superpowers/sdd/2026-09-29-ultracode/slot.sh cargo …`. Green means 0
  warnings except the accepted `.rsrc` line, the four `#[ignore]` tests untouched, and
  `git diff --stat main -- engine/tests/fixtures` empty.
- Vaults are built in temporary directories, as `eventemit.rs`' and `approvals.rs`' tests do. A
  test that reads a fixture copies it first. No test touches the network, Credential Manager or
  `engine/tests/fixtures/**`.
- No test or code contains the literal `"quinn"` or `"student"`. A human write in a test takes its
  actor from `journal::read_human_actor(&vault)`; an agent write uses an `agent:` actor.
- Free text in a card or note goes through `judge::one_line`; frontmatter goes through
  `yamlemit` (`safe_dump_block` or `commitments::front_matter`); every note through `write::create`.
- New files are LF. Commits carry the session's trailer.
- One implementer at a time in the worktree (HANDOFF §2). A task whose agent fails twice goes one
  level up, never straight to `max`.

**CL** marks a contract-list file. **Quinn** marks a task that needs Quinn's word or presence.

| Task | Agent (model, effort) | Quinn | Files | After |
|---|---|---|---|---|
| T0 | main session | PQ1–PQ4 answered; PQ3 (b-prime)'s blocker asked | none (git, ledger) | — |
| T0c | `researcher` (Sonnet, medium) | — | none (returns counts) | any time before T10 |
| H1a | main session | — | `engine/src/lib.rs`, `engine/src/eventaccept.rs` (stub) | T0 |
| T1a | `implementer` (Sonnet, high) | PQ2 (answered) | `engine/src/eventaccept.rs` | H1a |
| T1b | `implementer` (Sonnet, high) | — | `engine/src/events.rs` (`EventsConfig`, loader), `engine/src/commitments.rs` (`create_confirmed_as`' body) | T1a |
| H1b | main session | — | `engine/src/lib.rs`, `engine/src/eventcarry.rs` (stub) | T1b |
| T2b.1 | `contract-engineer` (Opus, xhigh) | — | `engine/src/eventcarry.rs` | H1b |
| T2a.1a | `implementer` (Sonnet, high) | — | `engine/src/eventemit.rs` | T2b.1 |
| T2a.1b | `implementer` (Sonnet, high) | — | `engine/src/eventemit.rs` | T2a.1a |
| T2a.2 | `implementer` (Sonnet, high) | — | `engine/src/eventemit.rs` | T2a.1b |
| T2b.2 | `contract-engineer` (Opus, xhigh) | — | `engine/src/eventcarry.rs` | T2a.2 |
| T2b.2b | `contract-engineer` (Opus, xhigh) | PQ1 (a) | `engine/src/eventcarry.rs` | T2b.2 |
| T2b.3 | `contract-engineer` (Opus, xhigh) | — | `engine/src/events.rs` (`judge_roster` only) | T2b.2b |
| T3.1a | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T2b.3 |
| T3.1b | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T3.1a |
| T3.2 | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T3.1b |
| B | `contract-reviewer` (Opus, xhigh) | checkpoint | `docs/reports/…-events-contract-review.md` | T3.2 |
| R1 | `reviewer` (Opus, high), beside B | — | `docs/reports/…-events-sonnet-review-1.md` | T3.2 |
| T4 | `contract-engineer` (Opus, xhigh) | — | `engine/src/cli.rs`; test 24 in `engine/src/enrich.rs`' test module | B, R1 |
| T4b | `implementer` (Sonnet, high) | — (PQ3 (b-prime) not here) | `engine/src/surface.rs` (`Loaded`, `load_with`, `the_day`, `coming_up`, `ComingUp`, one new private reader) | T4 |
| B2 | `contract-reviewer` (Opus, xhigh) on T4 + `reviewer` (Opus, high) on T4b | — | `docs/reports/…-events-review-2.md` | T4b |
| M | main session | push go | `main` merged into `events` | B2, and M2 merged (Gmail is already in) |
| T5 | `console-ui` (Sonnet, medium) | — | `app/static/console.js`, `app/tests/static_assets.rs` | M |
| T6 | `mechanical` (Sonnet, low) | — | `app/assets/campus/{none,university-of-alabama}.yaml`, `app/tests/scaffold.rs` | M |
| T7 | `test-writer` (Sonnet, medium) | — | `app/tests/commands.rs` | M |
| T8 | `docs-keeper` (Sonnet, medium) | — | `docs/surface/anatomy.md`, `docs/reference/engine-commands.md`, `docs/notes/2026-09-29-vision-program.md`, spec §12 note | T7 |
| T9 | `cloud-engineer` (Opus, high), on `j-events` | PQ4 (answered) | `docs/reports/…-events-b1.md` (harness run, not edited) | any time before T10 |
| W | `reviewer` (Opus, high) + `contract-reviewer` (Opus, xhigh) | checkpoint | `docs/reports/…-events-whole-branch-review.md` | T8 |
| T10 | main session | merge word | none (scratch profile) | W, T9, staging |

T2a and T2b interleave as the spec orders (T2b's first commit, then T2a, then the rest of T2b),
but serially, because one implementer runs at a time in the worktree. T5, T6 and T7 touch disjoint
files and run one after another after M.

**PQ3 (b-prime) has no task.** As approved, it cannot be built (§4, PQ3). When Quinn rules on its
blocker, the planner adds its tasks where §4 says, tests first, and the ledger (§9) gains their
rows. The agents are fixed now: the ledger-line change in `eventledger.rs` (the carry's entry point,
the `from:` field and `load_ledger`'s new read cases) goes to `contract-engineer` at xhigh, as Quinn
directed; the carry's call in `eventcarry.rs` to `contract-engineer` (Opus, xhigh); the provenance
read in `surface.rs` to `implementer`. Until then, no task writes or reads anything for it, and no
test pins whether the accept carry writes an answer line (T2b.2, T2b.2b).

### T0. Preflight (main session)

**Why the main session:** git state, sibling-lane checks and Quinn's answers are the controller's.
- Confirm the worktree is at `2d5298a` (or a later commit of this branch) with a clean tree, that
  `git merge-base --is-ancestor 199cd1f HEAD` succeeds, and that `app/tests/handler_lists.rs` is in
  the tree. Record the base in a new SDD ledger,
  `.superpowers/sdd/2026-09-29-ultracode/events-progress.md`.
- Run `git merge-tree --write-tree` of `events` (at `2d5298a`) against `m2-editing`,
  `p3-registrar` and `j-events`, and `git diff --name-only main...<branch>` for each (Gmail is
  merged and already in `events`). Record that
  the engine tasks (T1a–T4b) share only the files §2's table names. Repeat before M and before the
  merge (spec §9).
- Record Quinn's answers of 2026-09-30 (§4): PQ1 (a), PQ2 (the all-day lane), PQ3 (b-prime), which
  replaced (c), and PQ4 (as recommended). Bring Quinn (b-prime)'s blocker and the open choice (§4,
  PQ3), with its context, before T2b.2.

**Done when:** the ledger records the base, the overlap check, the four answers and PQ3
(b-prime)'s blocker, and later Quinn's ruling on it.

### T0c. The Alabama shape count (researcher, off the critical path)

**Agent:** `researcher` (Sonnet, medium). **Why:** read-only research over public feeds (spec T0).
It fetches the six feeds `app/assets/campus/university-of-alabama.yaml` lists, read-only, and counts
the next 60 days' events by P4's shapes: timed one-day, past-midnight, all-day, multi-day,
zero-length. It writes nothing; it returns the counts, one line per feed and a total.
**After it:** docs-keeper adds the number to spec §12's Q1 note in T8 ("measured 2026-…: …").
**Done when:** the counts are in the ledger.

### H1a. `pub mod eventaccept;` (main session)

The controller adds `pub mod eventaccept;` to `engine/src/lib.rs` beside `eventemit`, and creates
`engine/src/eventaccept.rs` holding only its module doc line. The workspace builds green with no
warning. One commit.

### T1a. The pure builders (`eventaccept.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** pure functions, no I/O, off the list, checked by
the compiler and tests. It writes nothing to a vault.
**Files:** `engine/src/eventaccept.rs`. **PQ2:** answered 2026-09-30: a zero-length event goes in
the all-day lane with no commitment.

**Tests first** (spec test 8, split so each shape is its own test, plus P5's payload):
- `a_timed_one_day_event_gives_the_commitment_mapping`: `kind: event`, `level` from the verdict
  (`obligation` hard, `opportunity` soft), `title` cut to 200, one `meets` entry with the weekday's
  `DAY_KEYS` key and `HH:MM` start and end, `from` = `until` = the date, `where` only when the
  location is 1–80 characters, and no `ends` key.
- `a_past_midnight_event_ends_at_2359_and_carries_its_true_end`: 10pm–1am gives `end: "23:59"` on
  the start day and `ends: "1am the next day"`; 10pm–12am gives `ends: "midnight"`.
- `an_all_day_or_multi_day_event_gives_the_lane_marker_and_no_commitment`: one all-day day, three
  all-day days, and a Fri 5pm–Sun 2pm event each give the all-day shape with their first and last
  day (an end at 00:00 belongs to the day before).
- `a_zero_length_event_gives_the_lane_marker` (PQ2): a 3pm–3pm event, and a 3pm–2pm event on
  one day, each give the lane shape with first day = last day = their date, and `commitment_for`
  returns `None` for both, at either level. Neither is read as past-midnight.
- `the_register_task_is_due_at_the_deadline_at_2359`: `registration: true` with a deadline gives
  §4.2 step 2's fields, `due: "<deadline>T23:59"`, `source_uid: "register:<uid>"`.
- `the_register_task_without_a_deadline_is_undated_and_names_the_start_as_an_upper_bound`: `due` is
  null, no field holds the event's start, and the body is §4.2's sentence with the start and the URL.
- `no_register_task_unless_the_feed_says_registration`.
- `an_instance_payload_round_trips` (P5): the payload built from a `DiscoveredEvent` and read back
  from its YAML gives the same instance, `start`/`end` formatted from real datetimes.

**Behaviour.** Spec §4.2, §5.1 and Q1, Q1a, Q1b, with P4, P5 and P6.
- `shape(start, end) -> Shape`: P4's rules, the one classifier.
- `Instance`: P5's fields, with `from_event(&DiscoveredEvent)`, `to_node()` (a `yamlemit::Node`
  mapping) and `from_yaml(&Mapping) -> Option<Instance>`. A malformed entry reads as `None`.
- `commitment_for(&Instance, level) -> Option<Mapping>`: the mapping `commitments::create_confirmed`
  takes, or `None` for the lane shapes. Times are wall-clock in the vault's timezone, as the
  instance holds them.
- `register_task(&Instance) -> Option<(String, String)>`: the note's file stem and its full text
  (frontmatter through `yamlemit::safe_dump_block`, then the body). Fields exactly as spec §4.2
  step 2: `title`, `due`, `effort_hours: 0.25`, `importance: 3`, `domain: school`,
  `status: active`, `created_by: events`, `source_uid`.

**Done when:** the eight tests pass and nothing outside `eventaccept.rs` changed.

### T1b. The switch and the commitment's body line (`events.rs`, `commitments.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** two small, fully specified edits off the list.
**Files:** `engine/src/events.rs` (`EventsConfig`, `Default`, `load_events_config` only),
`engine/src/commitments.rs` (`create_confirmed_as`' body choice only).

**Tests first:**
- `event_cards_reads_true_or_1_and_anything_else_is_off` (events.rs): `true` and `1` are on;
  `false`, `"yes"`, a list and no key are off, each with no warning. Every existing
  `events.rs` test passes unchanged.
- `a_kind_event_commitment_says_it_was_accepted` (commitments.rs): `create_confirmed` with
  `kind: event` and `source_uid: "localist:77:1"` writes "You accepted this from your campus
  events." and not the series line; with P6's `ends` key it adds " Ends at 1am the next day."; the
  frontmatter has no `ends` field. The `gcal-series:` and `card:` arms are byte-unchanged (their
  existing tests pass).

**Behaviour.** Spec §5.5 and §4.2 ("The note's body"), with P6.
- `EventsConfig` gains `event_cards: bool` (default false). The loader reads it with no warning
  path: a YAML `true` or the integer `1` is on. It is not added to `DEFAULTS`, which holds integers.
- The body branch tests `kind == "event"` first, before the key-prefix arms (p3's rebase keeps this
  order, spec §9).

**Done when:** both tests pass, and every `events.rs` and `commitments.rs` test (both tripwires
included) passes unchanged.

### H1b. `pub mod eventcarry;` (main session)

As H1a, for `engine/src/eventcarry.rs`. One commit.

### T2b.1. `answered_series`, the one definition (`eventcarry.rs`, first commit)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** every series decision in this lane reads this
predicate. A wrong answer here declines or hard-books a whole series with no card to catch it
(spec §13). The roster has no Opus-high implementer, and contract-engineer's description names this
case.
**Files:** `engine/src/eventcarry.rs`.

**Tests first** (spec §6.1's edge rules, T2b's unit tests):
- `an_executed_card_answers_accept_and_a_rejected_one_decline`.
- `an_expired_or_other_status_answers_nothing` (ruling G1), including a `pending` card left in
  `archive/` by hand.
- `a_card_with_a_missing_or_empty_series_uid_answers_no_series`.
- `an_event_check_card_is_not_counted`. PQ1 (a) leaves this rule unchanged: accepted `event-check`
  series reach the carry through their own reader, `accepted_check_series` (T2b.2b), never through
  `answered_series`.
- `two_cards_for_one_series_the_lowest_file_name_wins`.
- `only_archive_is_read`: an `executed` card in `approvals/` answers nothing.

**Behaviour.** `answered_series(vault) -> BTreeMap<String, SeriesAccept>`, where the value holds
`accepted: bool` and the card's file name. It reads `archive/` only, through
`approvals::sorted_md`, `type: approval` and `kind: event-accept`. It mirrors
`eventemit::settled_series`' shape and reads no ledger.
**Done when:** the six tests pass. This commit is T2a.1a's base.

### T2a.1a. The shared selection, as a refactor (`eventemit.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** off the list, and a refactor that a pinning test
checks byte for byte. `reviewer` reads it in R1, before any push; a second failed attempt goes to
Opus at high.
**Files:** `engine/src/eventemit.rs`.

**Test first:** `the_selection_refactor_keeps_what_event_checks_select`, written and green on
today's code before the refactor: for a fixed vault with four `unsure` events (one a two-instance
series, one past the cap), the card file names in order; each card's `title`, `source_uid`,
`series_uid`, `events:`, `expires` and `first_proposed_at`; and the `proposed` ledger lines. It
pins the selection, not the full card bytes, so it stays true when T2a.1b adds `instances:` and
the new closing (§5.2), which are spec-mandated byte changes. In this task the full bytes are also
unchanged: after the refactor, the same vault's cards equal the pre-refactor run's cards with their
`id:` lines removed (P14), checked once by the implementer and recorded in the SDD ledger, and every
existing `eventemit.rs` test passes unchanged.

**Behaviour.** P10. `emit_event_checks`' selection (eligibility, window, never-ask-twice, cap,
order, series grouping) moves into one private function over a verdict-to-card table that has one
row, `unsure` → `event-check`, carrying its window, closing and cap. `emit_event_checks`' signature
and output do not change. No new card kind, no `instances:` yet.
**Done when:** the pinning test and every existing `eventemit.rs` test pass.

### T2a.1b. Obligation cards, `instances:` and never-ask-twice across both kinds (`eventemit.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** off the list. What it writes is a card and its
`proposed` line: a wrong card is a proposal the student sees, never a silent change to the plan.
`reviewer` reads it in R1, before any push; a second failed attempt goes to Opus at high.
**Files:** `engine/src/eventemit.rs`.

**Tests first** (spec tests 1, 3, 5, 6 and 25, obligation and check halves):
- `an_obligation_files_one_event_accept_card` (1): `kind: event-accept`, `verdict: obligation`, the
  `Required · ` title, `why` = "**Knowlu thinks this is required of you.** <the ledger's why>" as
  the first paragraph, then the facts line, the URL, the series' other dates and the closing
  sentence; `events:`, `instances:` (P5), `source_uid`, `series_uid`, the ledger's `judgment_id`
  and `judgment_kind: event`; no Obsidian buttons. The `proposed` line follows the card: a create
  that fails leaves no line.
- `obligation_cards_are_capped_at_three_a_day_beside_the_checks` (3): four obligations file three;
  three `event-check` cards first proposed today do not reduce the obligation allowance; the budget
  argument bounds both.
- `never_ask_twice_across_both_kinds` (5): a uid named as `source_uid` or in `events:` on any card
  of either event kind, in `approvals/` or `archive/`, is skipped, and so is a uid with a `proposed`
  or `declined` line, or a series with a live card of either kind. The answered-series case seeds an
  `executed` and a `rejected` `event-accept` card in `archive/` and asserts a skip through
  `eventcarry::answered_series`; the edge cases stay T2b.1's. An expired card closes only its own
  instances.
- `a_series_files_one_card_of_at_most_twenty` (6), for `event-accept`.
- `new_event_check_cards_carry_instances_and_the_new_closing` (§5.2): `instances:` present, the
  closing reads "Approve if it applies to you: it goes on your schedule for that day. Reject and
  it's dropped. Either way you won't be asked again."
- `event_accept_is_not_a_local_card_kind_and_its_notes_sync` (25): `event-accept` is not in
  `commitments::LOCAL_CARD_KINDS`; `ids::NOTE_FOLDERS` contains `commitments` and `tasks`.
  Membership only, never length or order.
- `a_roster_read_event_files_no_event_accept_card_and_no_instances` (second review; PQ2, §2): the
  events are built as `read_roster` builds them (`source: "roster"`), among them a zero-length
  obligation read back as 15:00–16:00. `emit_event_accepts` files no card and writes no `proposed`
  line for any of them, so the next fetched run can still ask. An `unsure` roster event still gets
  its `event-check` card exactly as today, with no `instances:` key, so its Approve records the
  answer only (D11's without-`instances:` path) and books nothing. The same events with any other
  `source` file their cards as tests 1 and 5 say.

**Behaviour.** Spec §4.1, §5.1, §5.2, D5, D7 and P10.
- **No card is built from the roster's read-back** (§2). `emit_event_accepts` skips every event
  whose `source` is `"roster"`, and `emit_event_checks` adds `instances:` only for events whose
  `source` is not `"roster"`. The skip is by source, not by `rank`'s `feeds_failed`, so the emitter
  needs no new argument. The trade-off: an obligation is asked on the next run that fetches it
  while it is still in its window, so one whose date passes during a whole outage is never asked.
  The digest (switch off) and `emit_event_checks`' selection keep reading roster events as today.
- T2a.1a's table gains its second row, `obligation` → `event-accept` (P10's window). T2a.2 adds
  the third. `event-check` cards gain `instances:` and §5.2's closing.
- Never-ask-twice reads both kinds. Its answered-series test is the union of `settled_series` and
  `eventcarry::answered_series`; this module defines no predicate for answered `event-accept`
  cards.
- Each kind counts its own `first_proposed_at == today` cards for its cap of 3: `event-check` by
  kind, `event-accept` by its `verdict:`.
- `pub fn emit_event_accepts(vault, events, ledger, config, today, budget, verdict, ctx, journal)
  -> (Vec<PathBuf>, usize)` beside `emit_event_checks`, whose signature does not change. File name
  `approvals/event-<slug≤40>-<primary date>.md`, `-2`, `-3` on a collision; whole-card create through
  `write::create`; frontmatter through `yamlemit::safe_dump_block`.
- `expires` for an obligation card is its primary's date.

**Done when:** the seven tests pass and every existing `eventemit.rs` test passes unchanged.

### T2a.2. Opportunity cards behind the switch (`eventemit.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** as T2a.1b. **Files:** `engine/src/eventemit.rs`.

**Tests first** (spec tests 2, 4 and 7, and the opportunity halves of 1 and 3):
- `an_opportunity_files_a_card_only_when_event_cards_is_on` (2): with the switch on, a
  `verdict: opportunity` card titled `Worth a look · …` and led by "**This may interest you.**";
  with it off, `emit_event_accepts` files nothing for opportunities, and the existing digest tests
  pass unchanged.
- `opportunity_cards_are_capped_at_three_a_day` (3): three already first proposed today means none.
- `cards_come_out_in_their_order_and_the_same_input_writes_the_same_bytes` (4): obligations and
  checks in `(start, uid)`, opportunities by the digest's `sort_key` (strength, registration
  deadline, start, uid); two runs over copies of one vault write the same output, compared as P14
  says: the same file names in the same order, each card's bytes equal once its `id:` line is
  removed, and the same `proposed` ledger lines. Journal timestamps and `id:` values are left out,
  and the test's comment says why (random ids and wall-clock journal times are contracts). No
  deterministic id is introduced to make the test pass.
- `expiry_is_the_event_date_or_fourteen_days_for_an_opportunity` (7): an obligation card expires on
  its primary's date; an opportunity card on the earlier of that date and
  `first_proposed_at + 14`.
- `a_roster_read_opportunity_files_no_card` (second review; §2): with the switch on, an opportunity
  built as `read_roster` builds it (`source: "roster"`, a past-midnight event read back as
  22:00–23:00) files no card and no `proposed` line; the same event with a feed `source` files one.

**Behaviour.** Spec §4.4, D3, D6, D7. The table's third row, `opportunity` → `event-accept`, is
live only when `config.event_cards` is true. `emit_event_accepts` takes the verdict to file, so
`rank` can size obligations and opportunities separately (T4).
**Done when:** the five tests pass, and T2a.1a's and T2a.1b's tests pass unchanged.

### T2b.2. The ever-written set, D9's rebuild and the series carry (`eventcarry.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** this is the one place that writes into a
student's plan with no card in front of it. A wrong set brings back a note the student deleted; a
wrong carry books or declines a whole series silently (spec §13).
**Files:** `engine/src/eventcarry.rs`. **Scope:** `event-accept` series only; PQ1 (a)'s
`event-check` series are T2b.2b, its own commit. PQ3 (b-prime) adds nothing here until Quinn rules
on its blocker (§4): this task writes no answer line for a carried date, and no test in it asserts
either way on the accept carry's ledger lines, because the ruling decides them and a test's
assertion may not change later.

**Tests first** (spec tests 19, 20, 20a at the carry's entry point, P13; plus the set's units):
- `the_ever_written_set_reads_commitments_tasks_and_archive` (§5.3): a uid on a live commitment, a
  live task (`register:<uid>`), an archived commitment and an archived task is in the set; a uid
  only on an archived `type: approval` card is not.
- `d9_rebuilds_declined_lines_from_a_rejected_card` (19): a `rejected` `event-accept` card in
  `archive/` and an empty ledger: one `declined` line per listed uid, and `relevant_events` then
  excludes them. A second call writes nothing. An `executed` card writes nothing.
- `a_declined_series_declines_its_later_instances` (20, decline half).
- `an_accepted_series_books_a_later_instance_once` (20, accept half): one commitment, `level` from
  the card's `verdict:`, `source_uid` the instance's uid; a second call writes nothing, no journal
  record included. An instance starting before today is not booked. No register task is written
  (P9). An all-day instance gets no commitment and no warning. (No clause here reads
  `state/events-seen.md` after an accept: PQ3's ruling decides it, §4.)
- `a_deleted_carried_note_stays_deleted` (20a): carry, delete the commitment through
  `write::delete` (it lands in `archive/`), call twice more: no note and no journal record for that
  uid. Delete the primary's register task the same way: never re-created.
- `roster_read_events_carry_nothing` (the feed-failure run, §2): events built as `read_roster`
  builds them, each with its `series_uid` equal to its uid (`DiscoveredEvent::new`). Archived is an
  `executed` `event-accept` card for series `lx:77` listing `lx:77:1`. The event `lx:77:3` writes
  nothing: its series is its own uid.

**Behaviour.** Spec §4.5, §5.3, D4, D9, with P9.
- `ever_written(vault) -> BTreeSet<String>`: `source_uid` from every note in `commitments/`,
  `tasks/` and `archive/`, leaving out `type: approval`. Unreadable notes are skipped.
- `pub fn run(vault, events, ledger, today, ctx, journal) -> (Lines, Vec<String>)`, the one entry
  point `rank` calls (T4). In order: D9's rebuild (a `rejected` card's uids that have no `declined`
  line get one, through `eventledger::record_declined`), then the carry over every instance in
  `events` that its series' card did not list, in `(start, uid)` order.
  - Declined series (`answered_series`, rejected; unchanged): `record_declined` for each instance
    not yet declined.
  - Accepted series (`answered_series`' executed entries): for each instance starting today or
    later, unless its uid is in the ever-written set, the commitment from
    `eventaccept::commitment_for` goes through `commitments::create_confirmed` (actor
    `agent:commitments`, journal first). The level comes from the card's `verdict:`. The accepted
    set is built in one private helper, so T2b.2b widens it in one place.
  - The ledger map is updated as `load_ledger` would read the new `declined` lines. The accept
    carry writes no ledger line in this task, as spec §5.4 says. PQ3 (b-prime) would add one answer
    line per carried date and is blocked (§4); if Quinn's ruling builds it, its own task adds the
    line and its tests here.
- **A run built from the roster carries nothing, deliberately.** When every feed fails, `rank`
  passes `read_roster`'s events, whose `series_uid` is their own uid (§2). The carry then reaches no
  later instance and writes nothing; the next run that fetches the feeds carries as usual. This is
  intended: the carry reads series only from fetched events, never from the roster, the ledger or
  any other store. The module doc says so, so that a later change does not "fix" it.
- A write failure is a warning, and the next `rank` retries. Nothing here panics.
- No edit to `surface::delta`. Every carried note is a journal `create` in the `rank` run, which is
  what lists it (spec §4.5).

**Done when:** the six tests pass, and T2b.1's and T2a's tests pass unchanged.

### T2b.2b. PQ1 (a): an accepted `event-check` series carries (`eventcarry.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** it widens the silent booking path to a
second card kind, and a wrong reader hard-books a series the student only said applies (§8).
Split from T2b.2 by the second review so its diff is small and Checkpoint B reads it alone.
**Files:** `engine/src/eventcarry.rs`. **Answer applied:** PQ1 (a), Quinn, 2026-09-30.

**Tests first:**
- `an_accepted_event_check_series_books_a_later_instance_once`: an `executed` `event-check` card
  in `archive/` carrying `instances:` (series `lx:9`, listing `lx:9:1`), and events holding
  `lx:9:1` and a later timed `lx:9:2`. One commitment, for `lx:9:2`, with `level: hard` (P7) and
  `source_uid: "lx:9:2"`; none for `lx:9:1`, which the card listed. A second call writes nothing,
  no journal record included, and no register task is written (P9). (No clause reads
  `state/events-seen.md`: PQ3's ruling decides the accept carry's ledger lines, §4.)
  `answered_series` still does not count the card. A hand edit can make an archived `rejected` `event-accept` card answer
  `lx:9` too; then nothing is booked for the series, because `answered_series`' answer wins.
  Built as `read_roster` builds events (series equal to uid), a one-instance `ics:fair` whose
  executed `event-check` card lists it writes nothing.
- `an_event_check_card_without_instances_carries_nothing`: an `executed` `event-check` card with
  no `instances:` key (as filed before this lane), and its series' later instance in the events: no
  commitment and no journal record. The same holds for a `rejected` and an `expired` `event-check`
  card that carries `instances:`.

**Behaviour.** PQ1 (a), P7.
- `accepted_check_series(vault) -> BTreeMap<String, CheckAccept>`: reads `archive/` only, through
  `approvals::sorted_md`, for `type: approval`, `kind: event-check`, `status: executed` cards with
  an `instances:` sequence. The value holds the card's file name and the uids its `events:` lists.
  A card with a missing or empty `series_uid` answers no series, and the lowest file name wins. Any
  other status answers nothing here: a `rejected` `event-check` series already has `drop` lines
  through `inherit_series_answers`. It reads no ledger.
- T2b.2's accepted-set helper becomes the union of `answered_series`' executed entries and
  `accepted_check_series`. Where one series is in both readers, which only a hand edit can make,
  `answered_series` wins, so a series declined on an `event-accept` card is never booked. The level
  is `hard` for an `event-check` series.
- Only the accept carry calls it: not the decline carry, the emitter or `judge_roster`, which keep
  the union of `settled_series` and `answered_series`.

**Done when:** both tests pass, and T2b.1's and T2b.2's tests pass unchanged.

### T2b.3. `judge_roster` skips a settled series of either kind (`events.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** the spec places this one call-site edit in
T2b; it decides what the model is paid to judge. **Files:** `engine/src/events.rs` (`judge_roster`
only).
**Test first** (spec test 21): `judge_roster_does_not_judge_a_new_instance_of_an_accepted_series`,
built like the existing `judge_roster_does_not_judge_a_new_instance_of_a_settled_series` (a
`NeverCalled` model), with an `executed` `event-accept` card in `archive/`; and the same for a
`rejected` one.
**Behaviour.** The `settled` filter skips a series in the union of `eventemit::settled_series` and
`eventcarry::answered_series`.
**Done when:** the test passes and every `events.rs` test passes unchanged.

### T3.1a. The approved `event-accept` arm and the retry (`approvals.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** `approvals.rs` is on the contract list. A
silent error here writes the wrong note into a student's vault, or none.
**Files:** `engine/src/approvals.rs` (CL).

**Tests first** (spec tests 9–12):
- `approving_an_obligation_card_writes_a_hard_commitment` (9): one `commitments/` note
  (`kind: event`, `level: hard`, the one-day `meets`, `from` = `until`, `source_uid`,
  `status: confirmed`); the card ends `executed` in `archive/`; the journal's `create` record
  precedes the file and the stamp.
- `approving_an_opportunity_card_writes_a_soft_commitment` (10).
- `registration_writes_the_register_task` (11): with a deadline, `due` is it at 23:59; with
  `registration_deadline: null`, `due` is null, the task is in `surface`'s undated list, and no
  field holds the event's start.
- `a_retried_settlement_writes_no_second_note` (12): a card left `approved` (stamp failed, notes
  written), settled again, writes nothing new; a uid already on a note in `commitments/`, `tasks/`
  or `archive/` is skipped; an archived approval card carrying the uid as its `source_uid` does not
  count, so the primary is still written.

**Behaviour.** Spec §4.2, D1, D10.
- Approved `event-accept`: for each `instances:` entry (`eventaccept::Instance::from_yaml`) whose uid
  is not in `eventcarry::ever_written`, the commitment through `commitments::create_confirmed` when
  `commitment_for` gives one; then, for the primary only, the register task through
  `write::create` under the pass's context when `register_task` gives one and `register:<uid>` is not
  in the set. Then the stamp and `delete`, exactly as the `event-check` arm. A failed write returns
  `Err` and leaves the card `approved`.
- `proposal_weight` stays 1.

**Done when:** the four tests pass and every existing `approvals.rs` test passes unchanged.

### T3.1b. The rejected `event-accept` arm and expiry (`approvals.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** as T3.1a. **Files:**
`engine/src/approvals.rs` (CL).

**Tests first** (spec tests 13–14):
- `rejecting_writes_one_declined_line_per_listed_uid` (13): `relevant_events` then excludes them;
  the card is `rejected` in `archive/`.
- `an_expired_event_accept_card_writes_nothing` (14): no ledger line, no note. The digest's existing
  expiry test still declines.

**Behaviour.** Spec §4.3, §4.4, D8.
- Rejected `event-accept`: `record_declined` for every `events:` uid, then the generic archive.
- Expiry needs no new code: the generic path archives it `expired`. The test pins it, and may pass
  on its first run (recorded in the SDD ledger).

**Done when:** both tests pass, and T3.1a's and every existing `approvals.rs` test pass unchanged.

### T3.2. The extended `event-check` arm, and the shapes (`approvals.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Files:** `engine/src/approvals.rs` (CL).

**Tests first** (spec tests 15–17):
- `an_event_check_with_instances_records_the_answer_then_books_it` (15): the `record_answer` line as
  today, then a `level: hard` commitment (P7). Without `instances:`, the settlement's journal and
  ledger bytes equal today's for the same card.
- `an_all_day_instance_stands_with_no_commitment_and_no_warning` (16), with a zero-length
  instance alongside that also stands with no commitment and no warning (PQ2), and
  `a_past_midnight_instance_books_to_2359_with_its_true_end_in_the_body` (16).
- `the_calendar_event_and_digest_arms_are_unchanged` (17): their existing tests pass unedited; this
  named test settles one approved digest and one rejected digest beside an `event-accept` card in
  the same pass and checks the digest's results are what they were.

**Behaviour.** Spec §5.2, D11, Q1b. The approved `event-check` arm calls `settle_event_check`, then,
when `instances:` is present, the same accept settlement as T3.1a at `level: hard`; then the stamp.
**Done when:** the three tests pass, and T3.1a's, T3.1b's and every existing test pass unchanged.

### Checkpoint B. Contract review of T2b and T3 (Quinn's checkpoint)

**Agent:** `contract-reviewer` (Opus, xhigh). **Report:**
`docs/reports/<date>-events-contract-review.md`. It reads, one commit at a time, T2b.1, T2b.2,
T2b.2b, T2b.3, T3.1a, T3.1b and T3.2 against spec §4, §5, §6.1, §8, D1–D11 and Quinn's PQ1 (a),
and checks in particular:
- that `answered_series` has one definition and the emitter has no copy;
- reading T2b.2b's commit alone: that `accepted_check_series` feeds only the accept carry, never
  the decline carry, the emitter or `judge_roster`; that a cross-reader overlap goes to
  `answered_series`; and that an `event-check` card with no `instances:` carries nothing;
- that the accept carry writes no ledger line (spec §5.4): nothing for PQ3 (b-prime) is built
  before Quinn rules on its blocker (§4), and `eventledger.rs` is unchanged;
- that the ever-written set leaves out approval cards and includes `archive/`;
- that every note is written journal first, through `write::create`, by an `agent:` actor;
- that a failure leaves the card `approved` and a retry duplicates nothing;
- that the existing arms and the frozen references ran unchanged.

Fixes go back to `contract-engineer`, and the reviewer re-reads them. The controller then brings
Quinn the carry's and the settlement's diffs with the report (spec §13's second checkpoint). T4
starts when this report and R1's have no open Critical or Important finding.

### R1. Review of the Sonnet-written engine commits (beside Checkpoint B)

**Agent:** `reviewer` (Opus, high). **Why:** CLAUDE.md and spec §13: anything a cheaper agent
changed goes through `reviewer` before a push. **Report:**
`docs/reports/<date>-events-sonnet-review-1.md`. It reads H1a, T1a, T1b, H1b, T2a.1a, T2a.1b and
T2a.2, one commit at a time, against spec §4.1, §4.2, §5.1, §5.2, §5.5, D5–D8 and P4–P6, P10,
P14, and checks in particular: one shape classifier; free text through `judge::one_line`; every
card through `write::create` with its `proposed` line after it; no `event-accept` card and no
`instances:` entry built from a `source: "roster"` event (§2); never-ask-twice reads both kinds
through `eventcarry::answered_series` with no copy; test 4 compares as P14 says and no
deterministic id was introduced; no `"quinn"` or `"student"` literal. It runs beside Checkpoint B
(both are read-only and neither builds on the other). Fixes go back to `implementer`, and the
reviewer re-reads them; they run serially with B's fixes (one agent in the worktree at a time).

### T4. `rank` wiring (`cli.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** it sets the order of the vault-writing passes
(the carry and D9 before any card) and each one's budget. A wrong order files or carries against
stale state without a word (spec §13).
**Files:** `engine/src/cli.rs` (the events pass); test 24 appended to `engine/src/enrich.rs`' test
module (no code change there).

**Tests first** (spec tests 18, 19, 20a, 20b, 20c, 22, 23, 24; 19 and 20a at `rank` level, P13):
- `rank_rebuilds_declines_from_a_rejected_card` (19): a vault whose `archive/` holds a `rejected`
  `event-accept` card and whose ledger is empty, with the card's events in the feed; one `rank`
  writes one `declined` line per listed uid, files no new card for them, and
  `surface::coming_up` then lists none of them.
- `rank_twice_never_rebooks_a_deleted_carried_note` (20a): a `rank` carries a later instance of an
  accepted series; the commitment is deleted through `write::delete`, and so is the primary's
  register task; two more `rank` runs write no note and no journal record for either uid, and file
  no card for them. A second series in the same vault is an accepted `event-check` series (PQ1
  (a)). Its later timed instance gets its inherited `obligation` line from step 1 and one
  commitment from step 2, and once deleted it stays deleted. The test makes no Coming up
  assertion and reads no ledger line for the `event-accept` series: PQ3 (b-prime) is blocked (§4),
  and the task that builds it brings its own tests.
- `a_rank_on_the_roster_alone_carries_nothing` (feed-failure run): every feed fails, the roster
  holds a later instance of an accepted series and an unasked obligation; `rank` writes no
  commitment and no ledger line for the instance and files no `event-accept` card for the
  obligation (§2). The next `rank` with a working feed carries the instance and files the card.
- `rank_files_the_right_event_cards_within_the_budget` (18): a vault with an obligation, an
  opportunity, an `unsure` event and a three-instance series, `event_cards: true`: the right cards,
  the pending count raised by their number, the budget respected (seed same-day proposals so it
  binds), and `defer_over_budget` snoozing only same-day overflow.
- `the_carry_is_listed_in_the_delta_with_a_seen_at` (20b): after a `rank` that carries an
  instance, `surface::delta` with a `seen_at` earlier than that run's start counts the `create` and
  expands it to a record naming the carried commitment and the run id.
- `the_delta_fallback_is_pinned_without_a_seen_at` (20c): the same vault, no `seen_at`, the carrying
  run the newest: `since` is that run's `end`, `since_kind` is `"run"`, and none of the carry's
  `create` records is listed. If this fails, `delta` changed: the controller takes it to Quinn, and
  nobody edits `delta` to pass 20b.
- `an_event_card_is_an_ordinary_deck_card_and_accept_draws_the_block` (22): `surface` shows the card
  with `why` from its first paragraph; after an Accept and a settlement, `the_day` for the event's
  date has the commitment's block.
- `the_oracles_pass_with_no_reference_regenerated` (23): this is the existing `oracle.rs` and
  `surface_oracle.rs`, run unchanged; the task records their result, and the fixtures diff is empty.
- `labels_report_a_rejected_event_accept_card_as_a_decision` (24, in `enrich.rs`): one `decision`
  row (`proposed` → `rejected`) for a rejected card with a `judgment_id`; none for an executed one;
  an `event-check` still gives its `verdict` row.

**Behaviour.** Spec §6.1's `cli.rs` row. Inside the existing `if events_config.sources…` block, in
this order:
1. `inherit_series_answers` (unchanged);
2. `eventcarry::run` (D9's rebuild, then the carry), its warnings into `ledger:` lines; it is
   passed the same `candidates` as step 1, so on a run built from the roster it writes nothing
   (T2b.2), and no step reads series from anywhere else;
3. `write_roster` (unchanged, now after the carry, so the roster reads the carried declines);
4. obligation cards, sized to `remaining_budget`;
5. the digest **only when `event_cards` is off**, sized to what is left;
6. the `unsure` checks, sized to what is left;
7. opportunity cards **only when `event_cards` is on**, sized to what is left;
8. `relevant_events` for Coming up (unchanged).

Every card filed adds to `approvals.pending`. With the switch off and no obligation verdict (the
`vault-full` case), steps 2, 4 and 7 write nothing and the digest is sized exactly as today, which
is what keeps `golden-today-full.md` byte-identical. `surface::delta` is not edited.
**Done when:** the nine tests pass, and every existing `cli.rs` test, `oracle.rs` and
`surface_oracle.rs` pass unchanged. T4's commit is reviewed by `contract-reviewer` in B2.

### T4b. The all-day lane and "Accepted" (`surface.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** read-only: `surface` never writes, so an error
misdraws a lane but never changes vault bytes. **Files:** `engine/src/surface.rs` (`Loaded`,
`load_with`, `the_day`, `ComingUp`, `coming_up`, and one new private reader). **PQ3:** (b-prime)
is not built here. It is blocked (§4): no reader in this task reads a `from:` field or a carry's
answer line, or looks for carried instances that have no verdict.

**Tests first** (spec test 22a, plus P8's):
- `an_accepted_two_day_event_is_in_the_all_day_lane_on_each_day` (22a): after Accept of an all-day
  two-day event, `the_day` for each day lists it in `all_day`, `open_hours` equals the value before
  the Accept, and the day after lists nothing. An accepted zero-length event (PQ2) is in `all_day`
  on its own day only, and `open_hours` does not move.
- `an_accepted_event_is_marked_in_coming_up` (P8): an accepted timed event and an accepted all-day
  event (each on an `executed` card's `instances:`, each with a confident verdict) carry
  `accepted: true`. So do an instance judged confidently and then booked by the carry, and a later
  instance of an accepted `event-check` series with its inherited `obligation` line and its carried
  commitment (PQ1 (a)), each a `kind: event` note in `commitments/`. An unanswered one carries none.
- `accepted_is_absent_from_the_json_when_false` (P8): the serialised `ComingUp` of an unaccepted
  event has no `accepted` key, so `surface_oracle.rs` stays byte-identical.

**Behaviour.** Spec §4.2's lane, Q1b (i), §6.1's `surface.rs` row, P4, P8.
- One new private reader, `accepted_events(vault) -> AcceptedEvents`, reads `archive/` for
  `executed` `event-accept` and `event-check` cards with `instances:`, and `commitments/` for
  `kind: event` source uids. It returns the accepted uid set and the lane-shape instances (all-day,
  multi-day and zero-length, by `eventaccept::shape`; title, first day, last day). An unreadable note
  is skipped. A carried instance is on no card, so a carried all-day one is not in the lane (§4,
  PQ3).
- `load_with` calls it once and keeps both in new `Loaded` fields, for `the_day`.
- `coming_up(vault, today, now)` keeps its signature (it never receives a `Loaded`) and calls
  `accepted_events(vault)` itself for the uid set. That is one more read of `archive/` and
  `commitments/` per call, accepted for simplicity. No public signature changes, so `app/src` (which
  calls neither `coming_up` nor `load_with` directly at `4aa6560`; the implementer re-checks with a
  search) and M2's edits to `describe` are unaffected.
- `the_day` appends each lane instance covering `today` to `all_day`, after the calendar's own, in
  `(first day, title)` order. Capacity is not touched (anatomy §3.7).
- `ComingUp` gains `accepted: bool` with `#[serde(skip_serializing_if = …)]` for false;
  `coming_up` sets it from the uid set.
- No write, and `render` is untouched.

**Done when:** the three tests pass, and `surface_oracle.rs` and every `surface.rs` test pass
unchanged.

### B2. Review of T4 and T4b (before the first push)

**Agents:** `contract-reviewer` (Opus, xhigh) over T4's commit, and `reviewer` (Opus, high) over
T4b's. **Why:** spec §13 puts T4's `cli.rs` change (the order of the vault-writing passes and each
pass's budget) under contract-reviewer, though `cli.rs` is not on the contract list; T4b is
Sonnet-written and must be reviewed before a push. **Report:**
`docs/reports/<date>-events-review-2.md`, one section per agent. Contract-reviewer checks the
pass order against T4's list (the carry after `inherit_series_answers` and before the roster and
every card), each emitter's budget, the switch-off path that keeps the golden, the feed-failure
no-op, and that tests 19 and 20a at `rank` level assert what spec §11 states. Reviewer checks T4b
against P8 and the `accepted_events` reader (no public signature changed, `accepted` absent when
false, no write). Fixes go to `contract-engineer` and `implementer` respectively, serially.
**Done when:** no open Critical or Important finding. Then the controller asks Quinn's go to push
`events` (P12).

### M. Take `main` in after M2 merges (main session)

**Why the main session:** it is a merge that keeps the reviewed commits' hashes (P12). Gmail is
already in the branch (`2d5298a`). When `m2-editing` has merged to `main`, re-run T0's overlap
check, merge `main` into
`events`, resolve any textual conflict in `surface.rs`, `enrich.rs`'s test module or `lib.rs` (both
sides' lines kept), and run the gate. T5, T6 and T7 start from that commit. M and later commits are
not pushed until W closes.

### T5. Deck labels and the "Accepted" tag (`console.js`)

**Agent:** `console-ui` (Sonnet, medium). **Why:** `app/static` is its lane.
**Files:** `app/static/console.js`, `app/tests/static_assets.rs`.

**Tests first** (spec test 26, plus P8's renderer):
- `event_cards_read_accept_and_decline_and_send_the_same_verdicts` (26): on `event-accept` and
  `event-check` cards, at both sites (the deck's card and the Decisions view's row), the primary
  button reads **Accept** and the secondary **Decline**, and they still send `approved` and
  `rejected`; other kinds still read Approve and Reject, and `commitment-ask` keeps Answer….
- `coming_up_shows_accepted_events` (plan addition, P8): an entry with `accepted: true` renders an
  "Accepted" tag as escaped text; one without renders none.

**Behaviour.** Spec §6.3. The per-kind label switch follows the existing `commitment-ask` switch.
The tag is plain text beside the event's time, styled with an existing class. No new invoke.
**Done when:** both tests pass, and every existing static test passes unchanged.

### T6. Presets (`app/assets/campus/`)

**Agent:** `mechanical` (Sonnet, low). **Why:** two one-line asset edits and one assertion.
**Files:** `app/assets/campus/none.yaml`, `app/assets/campus/university-of-alabama.yaml`,
`app/tests/scaffold.rs`.
**Test first** (spec test 27): `either_preset_scaffolds_event_cards_on`: a vault scaffolded from
`none` and one from `university-of-alabama` both load with `event_cards == true` and no warning.
**Behaviour.** Spec §5.5, D6, Q5 (c). One line, `event_cards: true`, in each file, with a comment
line saying what it switches. The engine never writes `events.yaml`.
**Done when:** the test passes, and `the_campus_preset_is_the_shape_the_engine_already_reads` and the
other scaffold tests pass unchanged.

### T7. `decide` end to end (`app/tests/commands.rs`)

**Agent:** `test-writer` (Sonnet, medium). **Why:** a test only, over a finished engine.
**Files:** `app/tests/commands.rs`.
**Test** (spec test 28): `decide_accepting_an_event_card_books_it_before_returning`. A scaffolded
vault with a hand-built `event-accept` card (spec §5.1's shape); `decide_inner(…, "approved", …)`
returns the card in `decision.executed`, and the `commitments/` note exists before the call
returns. The card's `status` record has an actor `provenance::is_agent` rejects; the commitment's
`create` record has one it accepts. The test names no human token.
**Done when:** the test passes (it may pass first: it pins T3.1a through the app), and the file's
existing tests pass unchanged.

### T8. Reference documents

**Agent:** `docs-keeper` (Sonnet, medium). **Why:** reference text only. **Files:**
- `docs/surface/anatomy.md`, by its own headings. (Corrected 2026-09-30: an earlier draft put the
  delta under §3.11, which is Coming up; the delta line is §3.2.)
  - §3.2, the delta line: carried notes are listed as the carrying run's `create` records, with the
    `seen_at` gap and the 200-record cap named (spec §4.5).
  - §3.7, the day: accepted all-day, multi-day and zero-length (PQ2) events are in the all-day lane,
    read from executed cards' `instances:`. A carried all-day instance is not drawn, whether its
    series was answered on an `event-accept` or an `event-check` card: nothing stored holds its
    span (PQ3 (b-prime)'s blocker, §4). If Quinn's ruling on that blocker lands a task before T8,
    this section describes what it built instead.
  - §3.9, the deck: event cards, Accept and Decline on both event kinds, and expiry.
  - §3.11, Coming up: the "Accepted" mark (P8). A later instance of an accepted `event-check`
    series is listed through its inherited answer and, timed, booked and marked (PQ1 (a)). A later
    instance of an accepted `event-accept` series that has no verdict of its own is not listed,
    though a timed one has its block on its day. Quinn's answer is to list it through one answer
    line the carry writes, attributed to the carry and pointing at the answering card, with its
    provenance shown (PQ3 (b-prime), 2026-09-30); that is blocked pending a ruling (§4). If a task
    for it lands before T8, this section describes what that task built instead, with the
    provenance label it shows.
- `docs/reference/engine-commands.md`: one paragraph in `rank`'s entry (the events pass's order,
  `event_cards`, the carry over accepted `event-accept` and `event-check` series);
- `docs/notes/2026-09-29-vision-program.md`: the P4 row points at the spec and this plan; P6 (c)
  marked delivered by D3;
- `docs/specs/2026-09-29-events-design.md` §12: T0c's count in the Q1 note, dated; and one dated
  note recording the plan-stage answers of 2026-09-30. PQ1 (a) widens §4.5 and §6.1's
  `eventcarry.rs` row: the accept carry also reads executed `event-check` cards with `instances:`
  through `accepted_check_series`. PQ2 puts a zero-length event in the all-day lane (§4.2). PQ3's
  final answer, (b-prime), is recorded with its blocker and with the (c) answer it replaced. If
  Quinn's ruling builds (b-prime), the note also records §5.4's widening: the carry's answer line,
  its `by:agent:knowlu.carry` and `from:` fields, and `opportunity` on it. The signed text above
  the note is not rewritten;
- `docs/reference/app.md`, only if it lists the campus presets' keys.

HANDOFF is not edited (P11). **Done when:** each claim cites a function, not a line, and matches the
branch.

### T9. B1 (cloud-engineer, on `j-events`)

**Agent:** `cloud-engineer` (Opus, high). **Why:** cloud-side, a paid model run, and it needs the
OpenRouter key from Credential Manager. Q5 (c) is Quinn's go for the run. PQ4, answered
2026-09-30, settles the base: the harness runs as it stands on `j-events`.
**Files:** none edited on `j-events`; a report, `docs/reports/<date>-events-b1.md`, committed on
`main` by docs-keeper (the controller pushes docs).
- **The precondition (PQ4).** Before any paid call, check that the harness's frozen `event-3` arm
  (`event3_frozen.ts`) and the event row `model_row.ts` reads from the branch's migrations match
  `main`'s live event row: the row `main`'s migrations pin, at `main`'s current head. A read of
  staging's deployed row, if one is wanted, is the controller's. Record the rows compared and the
  result. **If they differ, do not run.** Report the difference, and the controller brings it to
  Quinn. Nothing is fixed on the branch.
- When they match: run `deno test` in `scripts/experiments/e1-decomposition/`, then `run.ts`, in the
  foreground.
- Report: the precondition's result; both arms' credit and recorded credit, the paired CI, the sign
  test, B1's verdict, and each arm's refusal rate (which is the rate of `unsure` cards on the
  device).
- **If `event-4` wins,** nothing ships from this lane or this task. It ships only through its own
  cloud PR (PQ4): after staging's pending `db push`, `main` merged into `j-events`, the migration
  renamed to a fresh timestamp, then review and deploy. That PR is opened by the controller with
  Quinn, not here. If it loses, the branch is closed as its header plans.
**Done when:** the report is written and the controller has brought Quinn the verdict, before T10.

### W. Whole-branch review (Quinn's checkpoint)

**Agents:** `reviewer` (Opus, high) over the whole diff, and `contract-reviewer` (Opus, xhigh) over
contract-list files, `eventcarry.rs` and T4's `cli.rs` events pass: every change to them made after
Checkpoint B and B2 (M's conflict resolutions and any fix included). **Report:**
`docs/reports/<date>-events-whole-branch-review.md`. It uses §9's ledger as its checklist and checks
in particular:
- Checkpoint B's, R1's and B2's reports are closed;
- determinism (spec §6.1): two `rank` runs over copies of one vault compared as P14 says (file names
  and order, card bytes without `id:` lines, `proposed` ledger lines; journal timestamps and `id:`
  values left out), and no deterministic id introduced anywhere;
- no new `"quinn"` or `"student"` literal; `human_actor_literal.rs` green;
- `app/src/` unchanged; no handler list changed;
- `site/` and `PRIVACY_VERSION` unchanged; no new telemetry row;
- the fixtures diff empty and both oracles green;
- `surface::delta` unchanged (20c green);
- one `answered_series`, one shape classifier, and `accepted_check_series` read only by the accept
  carry (PQ1 (a));
- PQ3 (b-prime): built and tested as Quinn's ruling on its blocker says, or, if unruled, absent
  with its gap named in anatomy §3.7 and §3.11, `eventledger.rs` unchanged and nothing half-built.

Fixes go to the task's own agent, and the reviewer re-reads them. The controller brings Quinn the
report before T10 (spec §13's third checkpoint). **Done when:** no open Critical or Important
finding.

### T10. Gate, live proof, PR and merge (main session, with Quinn)

**Why the main session:** it needs a staging session by OTP and a desktop; neither is delegated.
**Waits on:** W, T9's verdict, and staging matching the repo (START-HERE §3: Quinn's `db push`, then
the J functions redeployed), since the proof's verdicts come from staging's `judge-event`.
- **Gate:** the workspace build and tests with 0 other warnings; `scripts/ci/eol-check.ps1`; the
  fixtures diff empty; the four ignores in place; `human_actor_literal.rs` green; the handler lists
  unchanged, by running `app/tests/handler_lists.rs` (in the branch since `2d5298a`) as part of the
  workspace tests.
- **Live proof** (spec §11.4), under the standing solo-proof approval (dev build, DOM driver, OTP,
  never OS input), on a scratch profile against staging, Alabama preset:
  1. one real slot against the Alabama feeds;
  2. accept one obligation card and one opportunity card; decline one;
  3. check the schedule (the commitment's block, or the all-day lane) and Coming up ("Accepted",
     the declined event gone). A carried later instance has its block on its day. A later
     instance of an accepted `event-check` series is listed and marked (PQ1 (a)). A carried
     `event-accept` instance with no verdict is listed only if PQ3 (b-prime)'s blocker was ruled
     and built; otherwise the proof records it as absent, as anatomy §3.11 says;
  4. run one more `rank` and confirm nothing is re-asked.

  Then remove the scratch profile (profile, vault, credentials, autostart).
- **Then:** the PR, CI green, and Quinn's word to merge. At the MVP's parity audit, Quinn marks P4.

## 6. Order, parallelism and checkpoints

- **Order:** T0 → H1a → T1a → T1b → H1b → T2b.1 → T2a.1a → T2a.1b → T2a.2 → T2b.2 → T2b.2b →
  T2b.3 → T3.1a → T3.1b → T3.2 → (Checkpoint B ∥ R1) → T4 → T4b → B2 → first push (Quinn's go) →
  (M2 merges; Gmail is already in at `2d5298a`) → M → T5 → T6 → T7 → T8 → W → push → T10. PQ3
  (b-prime)'s tasks, if Quinn's ruling adds them, go where §4 says (before T2b.2, or after T4b and
  before B2).
- **Off the line:** T0c any time before T10 (read-only, no worktree). T9 any time before T10, in the
  `j-events` worktree, so it may run beside the engine tasks.
- **One implementer at a time** in the `events` worktree. Reviews run beside the next task only when
  that task does not build on the reviewed code: T4 waits for Checkpoint B and R1; M waits for B2.
- **Pushes (P12):** nothing is pushed before B2 closes; every Sonnet-written commit has then had a
  `reviewer` pass (R1 or B2). Commits after B2 are pushed only after W closes. Each push is a code
  push and needs Quinn's go.
- **Waits on Quinn:** PQ1, PQ2 and PQ4 were answered on 2026-09-30 and wait on nothing. PQ3
  (b-prime)'s blocker (§4) is wanted before T2b.2, and T2b.2 does not wait for it. Also the two checkpoints
  (B and W), the push go after B2 and after W, and the merge word in T10.
- **Opus starts:** T0, H1a, H1b, T2b.1, T2b.2, T2b.2b, T2b.3, T3.1a, T3.1b, T3.2, B, R1, T4, B2, T9, W, M, T10.
  Every other task starts on Sonnet, and a second failed attempt moves it to Opus at high.
- **Quinn's checkpoints (spec §13):** at signing (done, 2026-09-29); after Checkpoint B, with the
  carry's and the settlement's diffs; before T10, with W's report.
- **Sibling lanes:** the overlap check runs at T0, at M, and before the merge.

## 7. Controller hand-offs (main session)

- **H1a**, before T1a: `pub mod eventaccept;` in `engine/src/lib.rs`, beside `eventemit`, and the
  module file with its doc line.
- **H1b**, before T2b.1: `pub mod eventcarry;`, likewise.
- **`app/src/main.rs` and `app/src/lib.rs`: none.** No command is added, so neither
  `generate_handler!` list changes. Gmail's `app/tests/handler_lists.rs` is in the branch since
  `2d5298a`, so every gate runs it and no hand count is needed. The one fallback is M: if M2's
  merge changes the lists and `handler_lists.rs` does not yet pin M2's count, the controller counts
  both `generate_handler!` lists in `app/src/main.rs` by hand after M, compares them with `main`'s,
  and records both counts in the SDD ledger.
- **`engine/src/lib.rs` at M:** M2's `pub mod profile;` and these two lines are kept side by side.

## 8. Risks

- **The carry hard-books or declines a whole series silently.** Guarded by one `answered_series`
  with its edge tests (T2b.1), the carry's tests 20 and 20a, Opus at xhigh for T2b, and Checkpoint B
  reading each commit alone.
- **PQ1 (a) books an `event-check` series as hard that the student did not mean.** The student
  answered "this applies to me", which D11 already treats as an Accept for the listed instances.
  Guarded by `accepted_check_series`' own edge rules (executed, with `instances:`, archive only),
  `answered_series` winning an overlap, the two PQ1 tests in T2b.2b (its own commit), the
  rank-level case in 20a, and Checkpoint B's check, on that commit alone, that the reader feeds
  only the accept carry.
- **A card books an hour the feed never claimed.** On a feed-failure run the emitters read
  `read_roster`'s lossy events (§2). Guarded by the source rule in T2a.1b and T2a.2, their two
  roster tests, the rank-level clause in `a_rank_on_the_roster_alone_carries_nothing`, and R1.
- **PQ3 (b-prime) is not delivered.** As approved it cannot be built (§4). If Quinn has not ruled
  by B2, the branch ships with the gap: a carried `event-accept` instance with no verdict is absent
  from Coming up, and a carried all-day instance from the lane. Anatomy §3.7 and §3.11 name the gap
  (T8), and W checks that nothing is half-built and `eventledger.rs` is unchanged.
- **A ledger line attributed to no one who answered.** If Quinn's ruling builds (b-prime), the carry
  writes answer lines no human gave. Guarded, in that ruling's tasks, by the `agent:` actor and the
  `from:` card, by `load_ledger`'s new rule that a human answer wins over a carry line, and by
  contract-engineer at xhigh on `eventledger.rs` (§4, PQ3, items 1–3).
- **A deleted note comes back.** Guarded by the ever-written set reading `archive/` (T2b.2), test
  20a at the carry and at `rank` level (T4), and test 12's retry case.
- **`rank` runs its passes in the wrong order.** Guarded by tests 19 and 20a at `rank` level (T4)
  and B2's contract-reviewer pass on T4.
- **Sonnet-written code is pushed unreviewed.** Guarded by P12: nothing is pushed before B2, and
  R1 and B2 cover every Sonnet commit up to then; W covers the rest before the second push.
- **The golden moves.** `vault-full` has no switch and no obligation; T4's order keeps the digest's
  budget exactly as today in that case. Guarded by test 23 (both oracles) at every gate and the
  empty fixtures diff.
- **The read-model references move** through the new `accepted` field. Guarded by P8's
  skip-when-false and T4b's JSON test; `surface_oracle.rs` is never regenerated by this lane.
- **The budget is overspent or a card kind starves another.** Each emitter is sized to what the
  previous left (T4), each kind has its own cap of 3 (T2a), and test 18 binds the budget.
- **A human-actor refusal in tests.** Since #23, a human write with the wrong token is refused.
  Guarded by the rule that tests read the actor from the vault (§5).
- **`event-4` ships through an out-of-order migration.** PQ4: a fresh timestamp and its own PR.
- **The proof runs against a staging that does not match the repo.** T10 waits on START-HERE §3.
- **Merge friction with M2.** Gmail is already in (`2d5298a`). The engine tasks avoid M2's
  functions; T5 and T7 wait for M; the overlap check runs three times.
- **An older engine** warns `unknown kind:` on an approved `event-accept` card. Accepted by the spec
  (single desktop, single engine in the MVP); two-desktop's hand-off is spec §9's.

## 9. Fidelity ledger

Each row is a requirement of the signed spec, the task that meets it, and what proves it. Test
numbers are spec §11's. Plan-only tests are named.

| Spec requirement | Task | What proves it |
|---|---|---|
| §0, §2: one Accept/Decline card per judged obligation and opportunity, series grouped | T2a.1a, T2a.1b, T2a.2 | tests 1, 2, 6; `the_selection_refactor_keeps_what_event_checks_select` |
| §1 F1, D3, Q3 (a): one card per opportunity replaces the digest under the switch; no decline on expiry | T2a.2, T3.1b, T4 | tests 2, 7, 14, 18 |
| §1 F2: the digest's executor gap is not reached by new vaults | T4, T6 | test 27; the digest only with the switch off (T4 step 5) |
| §1 F3, D6: the golden stays byte-identical | T4 | test 23; the fixtures diff at every gate |
| §1 F4, D9: the archived card is the record of an answer; D9's rebuild | T2b.2, T4 | test 19 (unit, and `rank_rebuilds_declines_from_a_rejected_card`) |
| D1, Q1 (c): Accept writes a one-off `kind: event` commitment, hard or soft | T1a, T3.1a | tests 8, 9, 10 |
| Q1a (i): the register task is undated without a deadline; body names the start as an upper bound | T1a, T3.1a | test 8 (`…undated_and_names_the_start…`), test 11 |
| Q1b (i): all-day and multi-day in the all-day lane; past midnight to 23:59 with its true end | T1a, T1b, T3.2, T4b | test 8's shape tests; tests 16, 22a; `a_kind_event_commitment_says_it_was_accepted` |
| D2, Q2 (a): obligations are asked, never auto-created | T2a.1b, T3.1b | test 1 (a card, no note); test 14 |
| D4, Q4 (a): an answer covers the series both ways; the carry never rewrites an ever-written uid; every carried note listed | T2b.2, T4, B2 | tests 20, 20a (unit, and `rank_twice_never_rebooks_a_deleted_carried_note`), 20b, 20c; `roster_read_events_carry_nothing`, `a_rank_on_the_roster_alone_carries_nothing` |
| D5: a new card kind `event-accept`, filed by the same path as `event-check` | T2a.1a, T2a.1b | test 1; the shared selection |
| D7: caps of 3 + 3 beside the checks' 3; order obligations, checks, opportunities; opportunity sort | T2a.1b, T2a.2, T4 | tests 3, 4, 18 |
| D8: expiry writes nothing; only the card's own instances close | T2a.1b, T3.1b | tests 5, 14 |
| D10: the card carries `instances:`; Accept settles inside `decide` | T2a.1b, T3.1a, T7 | tests 1, 9, 28 |
| D11: an `event-check` Approve with `instances:` is an Accept; without, unchanged | T2a.1b, T3.2 | test 15; `new_event_check_cards_carry_instances_and_the_new_closing` |
| D12: no new telemetry shape | T4 | test 24 |
| §4.1: eligibility (verdict, unanswered, horizon, uid rule, series union through `answered_series`) | T2a.1a, T2a.1b, T2a.2, T2b.1 | tests 1, 5; T2b.1's six tests |
| §4.1: title and why paragraph; no Obsidian buttons | T2a.1b, T2a.2 | tests 1, 2 |
| §4.2: commitment fields; actor `agent:commitments`; task under the pass's context; body line | T1a, T1b, T3.1a, T7 | tests 8, 9, 28; `a_kind_event_commitment_says_it_was_accepted` |
| §4.2: a failed write leaves the card `approved`; a retry duplicates nothing | T3.1a | test 12 |
| §4.3: Decline writes one `declined` line per uid and leaves Coming up | T3.1b | test 13 |
| §4.4: snooze unchanged; expiry dates | T2a.1b, T2a.2, T3.1b | tests 7, 14 |
| §4.5: the carry beside `inherit_series_answers`, which is unchanged; `delta` not edited | T2b.2, T4 | tests 20, 20b, 20c; W checks `delta`'s diff |
| §4.5: the display cap and the `seen_at` gap are named, not fixed | T8 | anatomy §3.2 (the delta line) |
| §5.1: the card's fields and file name; `events:` keeps its shape | T2a.1b | tests 1, 5 |
| §5.2: new `event-check` cards gain `instances:` and the new closing | T2a.1b | `new_event_check_cards_carry_instances_and_the_new_closing` |
| §5.3: the ever-written set (commitments, tasks, archive; approvals left out) | T2b.2, T3.1a | `the_ever_written_set_reads_commitments_tasks_and_archive`; test 12 |
| §5.4: no new ledger line shape or verdict word | T3.1b, T2b.2, B, W | Checkpoint B checks the accept carry writes no ledger line; W checks `eventledger.rs` is unchanged. PQ3 (b-prime) would widen §5.4 and is blocked (§4); if Quinn's ruling builds it, this row is rewritten with that task's tests |
| §5.5: `event_cards` read as a boolean; both presets carry it | T1b, T6 | `event_cards_reads_true_or_1_and_anything_else_is_off`; test 27 |
| §6.1 `eventemit.rs` row | T2a.1a, T2a.1b, T2a.2 | tests 1–7; the refactor's pinning test |
| §6.1 `eventcarry.rs` row, one `answered_series`, the `judge_roster` call site | T2b.1–T2b.3 | T2b.1's tests; tests 19, 20, 20a, 21 |
| §6.1 `eventaccept.rs` row | T1a | test 8's tests |
| §6.1 `surface.rs` row: all-day lane, "Accepted" | T4b, T5 | test 22a; P8's three tests |
| §6.1 `approvals.rs` row (CL) | T3.1a, T3.1b, T3.2, B | tests 9–17; Checkpoint B |
| §6.1 `commitments.rs` row | T1b | `a_kind_event_commitment_says_it_was_accepted` |
| §6.1 `events.rs` row | T1b, T2b.3 | the loader test; test 21 |
| §6.1 `cli.rs` row: the pass order and budgets | T4, B2 | test 18; tests 19 and 20a at `rank` level; B2's contract-reviewer section |
| §6.1 invariants: no model in `rank`; deterministic bytes; no new JSON | T2a.2, T4 | test 4 compared as P14 says; W's determinism check |
| §6.2: no new command or flag; `engine-commands.md`'s `rank` entry | T8 | T8's paragraph; W |
| §6.3: no new Tauri command; `decide` settles in-process; Accept/Decline labels | T5, T7 | tests 26, 28; the handler-list recount (§7) |
| §7: no new data class; `PRIVACY_VERSION` and `site/` unchanged | every task | W checks the diff |
| §8: `approvals.rs` by contract-engineer, reviewed by contract-reviewer; other contracts untouched | T3.1a, T3.1b, T3.2, B, W | Checkpoint B; W's diff check |
| §8: frozen references and oracles unchanged | every task | test 23; the fixtures diff at every gate |
| §9: lane collisions and merge order | T0, M, T10 | the overlap checks in the ledger |
| §9: two-desktop hand-off recorded at that spec's next revision | none (out of scope) | carried in HANDOFF by the controller (P11) |
| §10, Q5 (c): B1 runs before the founder's proof | T9 | the B1 report |
| §11.4: the live proof | T10 | T10's steps |
| §12: T0's count recorded when measured | T0c, T8 | the ledger; spec §12's Q1 note |
| §13: agents and checkpoints; `approvals.rs`, `eventcarry.rs` and T4's `cli.rs` change through contract-reviewer; Sonnet work through `reviewer` before a push | every task, B, R1, B2, W | §5's table; §6's push rule; the B, R1, B2 and W reports |
| §14: ruling 10 reworded at signing (done); P4 row updated | T8 | the vision-program row |
| Reviews land in `docs/reports/` | B, R1, B2, W, T9 | the five reports |

**Quinn's plan-stage answers (2026-09-30).** Each widens or settles a signed section, so each has
its own row.

| Answer | Task | What proves it |
|---|---|---|
| PQ1 (a): an executed `event-check` card with `instances:` carries its series; `accepted_check_series`; the accept carry over the union; `answered_series`' edge rule unchanged | T2b.1, T2b.2b, T4, B, T8 | `an_accepted_event_check_series_books_a_later_instance_once`, `an_event_check_card_without_instances_carries_nothing`; T2b.1's `an_event_check_card_is_not_counted`; 20a's `event-check` case at `rank` level; Checkpoint B; the spec §12 note |
| PQ2: a zero-length event is drawn in the all-day lane with no commitment | T1a, T3.2, T4b, T8 | `a_zero_length_event_gives_the_lane_marker`; test 16's zero-length clause; test 22a's zero-length clause; anatomy §3.7 |
| PQ2, held on a feed-failure run: no card or `instances:` entry is built from `read_roster`'s lossy events (second review) | T2a.1b, T2a.2, T4 | `a_roster_read_event_files_no_event_accept_card_and_no_instances`, `a_roster_read_opportunity_files_no_card`, the card clause of `a_rank_on_the_roster_alone_carries_nothing` |
| PQ3 (b-prime), final (replaced (c)): one answer line per carried date, timed and all-day, by `agent:knowlu.carry`, with `from:` the answering card and the series' real verdict; Coming up lists the date with its provenance; the all-day lane draws a carried all-day date; no further write | **none: blocked** (§4) | Not met. The Coming-up half can be built; the lane half cannot without a further write, which Quinn ruled out. The blocker and the open choice go to Quinn at T0; anatomy §3.7 and §3.11 name the gap (T8); W checks nothing is half-built and `eventledger.rs` is unchanged |
| PQ4: B1 runs as it stands on `j-events` after the frozen `event-3` check; report in `docs/reports/`; `event-4` ships only through its own cloud PR | T9 | the B1 report, its precondition's result first |

**Plan-only tests.** `event_cards_reads_true_or_1_and_anything_else_is_off`,
`a_kind_event_commitment_says_it_was_accepted`, `an_instance_payload_round_trips`,
`a_zero_length_event_gives_the_lane_marker`, `the_selection_refactor_keeps_what_event_checks_select`,
`new_event_check_cards_carry_instances_and_the_new_closing`,
`the_ever_written_set_reads_commitments_tasks_and_archive`, `roster_read_events_carry_nothing`,
`a_rank_on_the_roster_alone_carries_nothing`, `an_accepted_event_is_marked_in_coming_up`,
`accepted_is_absent_from_the_json_when_false`, `coming_up_shows_accepted_events`,
`an_accepted_event_check_series_books_a_later_instance_once`,
`an_event_check_card_without_instances_carries_nothing`,
`a_roster_read_event_files_no_event_accept_card_and_no_instances` and
`a_roster_read_opportunity_files_no_card`.
PQ3 (b-prime) has no test until its blocker is ruled; `the_accept_carry_writes_no_ledger_line` was
removed with (c) (§11, item 3). Each pins a spec requirement that had no test of
its own, a plan decision (§3) or one of Quinn's answers; none changes a spec test's assertion. Spec test 4's "identical bytes"
is read as P14 states; that is the one interpretation of a spec assertion this plan makes.

## 10. Review revisions (2026-09-30)

Nine findings from the plan review; all accepted, none refuted. Each was checked against the code
on the `events` worktree (`4aa6560`) before the plan changed.

1. **Test 4 cannot compare whole bytes (Important).** Confirmed: `write::create` mints a random
   `id:` through `ids::new_id` (OS entropy) and journal records carry wall-clock times. *Changed:*
   new P14 defines the comparison (file names and order, card bytes without the `id:` line, the
   `proposed` ledger lines; ids and journal timestamps left out, no deterministic id). T2a.2's
   test 4 and W's determinism check cite it; the plan-only note records it as the one reading of a
   spec assertion.
2. **T4's `cli.rs` change had no contract-reviewer pass (Important).** *Changed:* new checkpoint
   B2 after T4b puts T4's commit under `contract-reviewer` (and T4b under `reviewer`). W's
   contract-reviewer scope now names contract-list files, `eventcarry.rs` and T4's `cli.rs` events
   pass, for changes after B and B2.
3. **Sonnet work could be pushed before review (Important).** *Changed:* P12 rewritten: nothing is
   pushed before B2 closes; new R1 (`reviewer`, beside Checkpoint B) covers H1a–T2a.2, B2 covers
   T4b, W covers everything after. M no longer claims a pushed branch. §6 states the push rule; each
   push still needs Quinn's go.
4. **PQ3 and P8 rested on a false premise (Important).** Confirmed: `settled_series` reads
   `event-check` cards only, `inherit_series_answers` reads only it, T2b.3 stops judging carried
   instances, and `relevant_events` drops a uid with no verdict. Also found: `record_answer` takes
   only `obligation` or `drop`. *Changed:* a new §2 fact; PQ3 restated ("carried instances, timed
   and all-day, are absent from Coming up") with (a) accept and name it, recommended, and (b)
   `record_answer` in the carry, with its word and `by` costs; PQ3 now due before T2b.2; P8's
   justification rewritten; T2b.2, T4 (test 20a), T4b's test, T8 and T10's proof step follow the
   answer.
5. **Rank-level tests 19 and 20a were moved off `rank` (Important).** *Changed:* P13 rewritten; T4
   adds `rank_rebuilds_declines_from_a_rejected_card` and
   `rank_twice_never_rebooks_a_deleted_carried_note` with the spec's assertions; T2b.2 keeps its unit
   tests. The ledger and §8 cite both levels.
6. **T4b's mechanism did not fit `coming_up`'s signature (Minor).** Confirmed:
   `surface::coming_up(vault, today, now)` takes no `Loaded`, and nothing in `app/src` calls it or
   `load_with`. *Changed:* one private `accepted_events(vault)` reader, called by `load_with` and by
   `coming_up`; no public signature changes.
7. **T2a.1 and T3.1 were oversized (Minor).** *Changed:* split into T2a.1a (the selection refactor,
   pinned by `the_selection_refactor_keeps_what_event_checks_select`, which pins the selection
   rather than full bytes so T2a.1b's spec-mandated `instances:` and closing do not break it),
   T2a.1b, T3.1a (approved arm, retry: tests 9–12) and T3.1b (rejected arm, expiry: 13–14). Table,
   order, Opus starts, Checkpoint B and the ledger renamed.
8. **The feed-failure run's silent no-op (Minor).** Confirmed: `rank` swaps in `read_roster`'s
   events, which carry no `series_uid`. *Changed:* §2 notes it; T2b.2 states the no-op as intended
   (with a module-doc line) and adds `events_with_no_series_uid_carry_nothing`; T4's step 2 says so
   and adds `a_rank_on_the_roster_alone_carries_nothing`.
9. **`app/tests/handler_lists.rs` is not on `main` (Minor).** Confirmed: no such file at
   `4aa6560`. *Changed:* §2, §7 and T10's gate say the controller counts both `generate_handler!`
   lists in `app/src/main.rs` by hand and records the count in the SDD ledger if Gmail has not
   merged by T10. *Superseded by the second round's finding 1:* the branch has had the test since
   `2d5298a`.

### Second round (2026-09-30)

Four findings from the second review of the revised plan; all accepted, none refuted. Each was
checked against the `events` worktree at `2d5298a` (read from the worktree's files and git refs)
before the plan changed. Quinn's answers in §4 are unchanged; only the plan's own text around them
moved.

1. **The base was misstated (Important).** Confirmed: `refs/heads/events` is `2d5298a`, whose
   reflog entry is the merge of `origin/main`, made after the plan commit `6352411` on `4aa6560`;
   `app/tests/handler_lists.rs` is in the worktree. *Changed:* the header, §2's opening and its
   sibling table (Gmail merged and in), T0 (confirms `2d5298a` and the ancestry, drops the "take
   `main` in" step, drops Gmail from the overlap check), M (waits on M2 only), §6's order, §7 (the
   hand count kept only as a fallback at M), T10's gate (runs `handler_lists.rs` directly), §8, and
   finding 9 above marked superseded. §11's bullet "`main` moved to `199cd1f`" is removed: the
   correction it recorded was itself wrong.
2. **The roster read-back's invented hour is not display only (Important).** Confirmed on
   `2d5298a`: `rank`'s events pass sets `candidates = read_roster(…)` when every feed failed, then
   passes `candidates` to `emit_digest`, `emit_event_checks` and the rest; `read_roster` turns
   `end <= start` into `start + 1h`. Also found: the same read-back turns a past-midnight event and
   an all-day one into one-hour timed events and drops `url` and the registration fields, so the
   rule is by source, not by shape. *Changed:* a new §2 fact; PQ2's recommendation sentence
   corrected with a dated note (Quinn's answer untouched); T2a.1b gains the source rule and
   `a_roster_read_event_files_no_event_accept_card_and_no_instances`, T2a.2 gains
   `a_roster_read_opportunity_files_no_card`; T4's `a_rank_on_the_roster_alone_carries_nothing`
   gains the card clause; R1, §8 and the ledger follow. Choice made here, for Quinn at Checkpoint B:
   the skip is by `source: "roster"` in the emitter, not by a `feeds_failed` argument, and an
   `unsure` roster event still gets its `event-check` card, without `instances:`, so its Approve
   books nothing. The cost is that an event whose date passes during a whole outage is never asked.
3. **PQ3's roster option understated its cost (Important).** Confirmed: `read_roster` is the
   feed-failure candidate source, and `relevant_events` serves `write_roster`, `rank`'s Coming up
   and `surface::coming_up`. *Changed:* the roster option in §4 names both costs, recommends a
   separate reader so the feed-failure rule and its two tests stand, names the `relevant_events`
   change, and states that no frozen or read-model reference can move (`vault-full` has no carried
   instance; test 23 and the fixtures diff prove it). The alternative is marked as sharing the
   second cost only. Quinn's answer (c) is unchanged; the blocker still waits on Quinn.
   *Superseded 2026-09-30:* Quinn replaced (c) with (b-prime), and the roster option went with
   (c)'s blocker (§4, PQ3; §11, item 3).
4. **T2b.2 was oversized with PQ1 in it (Important).** *Changed:* PQ1 (a) moved to a new task,
   T2b.2b (contract-engineer, its own commit after T2b.2): `accepted_check_series`, the union, the
   overlap rule, and its two tests, which now also carry the `event-check` halves of the
   no-ledger-line and roster-run checks. T2b.2 keeps seven tests over `event-accept` series and
   builds the accepted set in one helper for T2b.2b to widen. P1, §5's table, T2b.1's note,
   Checkpoint B (reads T2b.2b alone), §6, §8 and the PQ1 ledger row follow. *Changed again
   2026-09-30 (PQ3 (b-prime)):* the no-ledger-line test and clause were removed from T2b.2 and
   T2b.2b, so T2b.2 now has six tests (§11, item 3).

### Third round (2026-09-30)

One finding on the (b-prime) section; accepted, not refuted. Checked against
`engine/src/eventemit.rs` in the `events` worktree (`settled_series`, `inherit_series_answers`).
Quinn's answers in §4 are unchanged.

1. **Dates carried from an `event-check` series keep a line credited to the student
   (Important).** Confirmed: `settled_series` copies `by` from the ledger entry of the card's own
   `source_uid` (else `unknown`), and `inherit_series_answers` writes an `obligation` line through
   `record_answer` with that `by` and no `from:`, before the carry runs (T4, step 1). Item 5's rule
   then skips those dates. *Changed:* PQ3 gains item 6 under "What (b-prime) needs"; the blocker's
   last sentence separates this overlap from the lane gap; the open choice states that (i) reaches
   `event-accept` series only and adds a separate choice for Quinn (accept the split, or let the
   carry's line replace the inherited one, which changes the signed `inherit_series_answers`). Also
   found: a provenance label for such a date read through `settled_series`' card needs a stored
   instance-to-series link, which the (c) check found absent; item 6 says so. The plan does not
   choose.

## 11. Quinn's answers (2026-09-30)

Quinn answered the four open questions on 2026-09-30. Each answer is recorded under its question in
§4. Before the plan changed, each was checked against the code on the `events` worktree and on
`main` (`199cd1f`): `surface::coming_up`, `the_day`, `load_with`, `eventroster::{write_roster,
read_roster, relevant_events}`, `eventledger::{LedgerEntry, load_ledger, record_answer}`,
`eventemit::{settled_series, inherit_series_answers}`, `commitments::create_confirmed_as`,
`eventfeed::parse_localist`, `DiscoveredEvent::normalized` and `rank`'s events pass.

1. **PQ2: the all-day lane, no commitment.** *Changed:* P4's zero-length arm. T1a's PQ2 test is
   now concrete: two shapes, `commitment_for` gives `None`, neither is read as past-midnight. Test
   16 (T3.2) and test 22a (T4b) gain zero-length clauses, T4b's reader names the lane shapes, T8's
   §3.7 bullet covers it, and a ledger row was added.
2. **PQ1: (a), carry it.** *Changed (moved to T2b.2b by the second review):* T2b.2 gains
   `accepted_check_series` (behaviour, edge rules,
   and the rule that `answered_series` wins an overlap), the accept carry runs over the union, and
   the level for an `event-check` series is `hard` (P7). Its two tests are named, with the
   contract-engineer as before. T2b.1's edge test is annotated as unchanged. T4's 20a gains an
   `event-check` case at `rank` level, which proves the pass order for PQ1 as P13 does for the rest.
   T4b's P8 test gains the carried `event-check` instance. Also changed: P8, §2, Checkpoint B, W,
   §8, T8 (the spec §12 note, anatomy §3.11) and a ledger row.
3. **PQ3: (b-prime), Quinn's final answer, which replaced (c).** Quinn first answered (c), listing
   carried instances at read time with no new write. It was blocked as approved: nothing stored
   links a carried instance to its series, and a carried all-day instance is stored nowhere. Quinn
   then answered (b-prime): one answer line per carried date, by `agent:knowlu.carry`, with `from:`
   the answering card and the series' real verdict, and no further write invented. Checked against
   the `events` worktree at `8698e8d`: `eventledger::{record_answer, load_ledger, LedgerEntry}`,
   `eventroster::{write_roster, read_roster, relevant_events}`,
   `eventemit::{settled_series, inherit_series_answers}`, `approvals::settle_event_check`,
   `surface::{coming_up, the_day, load_with}`, `ids::{new_id, build_index}`, `journal::OPS`, `sync`'s
   note-path rule and `rank`'s events pass. *Blocked as approved* in its all-day half: the lane
   cannot draw a carried all-day date from the line plus what is stored (§4, PQ3). The rest can be
   built, but needs a sibling of `record_answer`, two new read cases in `load_ledger`, and a line
   with no journal record of its own (§4, PQ3, items 1–5); and dates carried from an `event-check`
   series keep `inherit_series_answers`' student-credited line (item 6, added by §10's third
   round). *Changed:* §4's (c) answer, blocker and
   unblocking options are replaced by (b-prime), what it needs, its blocker and the open choice. No
   task builds it; §5's note names the agents its tasks will take. `the_accept_carry_writes_no_ledger_line`
   is removed from T2b.2 and the matching clause from T2b.2b, because (b-prime) would invert both
   and a test's assertion may not change later. T2b.2's all-day clause now asserts no commitment
   and no warning, without reading the ledger. Also changed: the status line, §1, §2, P8, T0, T4's
   20a, T4b, Checkpoint B, T8 (anatomy §3.7 and §3.11, the spec §12 note), W, T10, §6, §8 (a new
   risk) and the ledger (the §5.4 and PQ3 rows, the plan-only list).
4. **PQ4: as recommended.** *Changed:* T9 makes the frozen `event-3` check a precondition: no paid
   call when it fails. The report leads with it, and `event-4` ships only through its own cloud PR,
   opened by the controller with Quinn.

**Also found while re-reading, and corrected:**
- **`read_roster`'s events carry `series_uid` equal to their uid, not an empty one.**
  `DiscoveredEvent::normalized` sets it. Review finding 8 (§10) said otherwise. The feed-failure
  no-op still holds: a later instance's own uid matches no answered series, and a one-instance
  event is the uid its card lists. *Changed:* §2, T2b.2's behaviour, and its test, renamed
  `roster_read_events_carry_nothing` and built as `read_roster` builds events.
- **T8 placed the delta under anatomy §3.11, which is Coming up.** The delta line is §3.2.
  *Changed:* T8's anatomy bullet is split by section, and the §4.5 ledger row now cites §3.2.
