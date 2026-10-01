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
Quinn's final PQ3 answer: (b-prime), which replaced (c), with the carried date's span on the carry's
line (§11, item 3). All four answers are applied to the tasks. PQ3 is built by T2b.4, T2b.5 and T4c
under P15 and P16 (§3). Revised again for Quinn's two answers at Checkpoint B (2026-10-01, §12):
- P16 is **accepted**: in `rank` the carry runs before series inheritance, reversing spec §6.1's
  signed order. It is recorded in a dated spec note (T8), and T4 is built as planned.
- The lane trade-off is **not** accepted: the student must be able to remove a stale accepted lane
  date in the MVP. P17 is the removal path. It writes the existing `declined` line through one
  engine function (T4d), called by one new Tauri command (H2). T4c reads the line, T5 adds the
  button, and T7 tests the command end to end.

Revised again for the fifth review (2026-10-01, §10 fifth round). **Two questions are open (§13):**
- **PQ5, a moved event.** Quinn's answer names a date a later fetch drops because it was cancelled
  *or moved*. P17 covers a cancellation, but only half covers a move. PQ5 is asked before T4c is
  dispatched, and it gates T4c and T4d.
- **PQ6, Undo or confirm.** VISION commitment 5 says a change the student asked for acts at once,
  with undo. PQ6 is asked after PQ5, and at the latest with B2's report. It gates T5, and it gates
  T4d only if Quinn asks for a real undo before T4d runs.

Revised again for Quinn's answers to both (2026-10-01, §13):
- **PQ5: (b2).** The carry re-records a moved lane span. New T2b.6 builds it before T4c, and T4c
  draws the latest span.
- **PQ6: (b).** A console-only Undo toast replaces the confirm in T5. No engine change.

T0 through T3.2 are built and were read at Checkpoint B. Nothing from T4 on is built. T4 and T4b
do not wait on either answer.

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
- **The ledger contract.** `VALID_VERDICTS` and `ANSWER_VERDICTS` (`eventledger.rs`) are unchanged.
  `record_answer` keeps its signature, refusals and bytes, and every existing line reads as it does
  today. Quinn's PQ3 answer adds one shape, the carry's answer line (P15): an answer line by
  `agent:knowlu.carry` with three new fields, `from:`, `start:` and `end:`. It is additive and
  appended like every ledger line. No existing line is rewritten (T2b.4). The removal path (P17)
  adds no shape: it appends the existing `declined` line through `record_declined`, and
  `eventledger.rs` is not edited for it. Quinn answered PQ5 (b2) and PQ6 (b) on 2026-10-01 (§13),
  so no further shape is added. (b2) lets the carry append a second line of its existing shape for
  a moved lane date, from the same card (T2b.6).
- **The existing settlement arms.** The digest, `calendar-event`, `task`, `amend`,
  `commitment-check`, `commitment-ask` and `event-check` (without `instances:`) arms behave
  byte-for-byte as today. The digest arms stay forever.
- **`rank` never calls a model,** and the same input gives the same bytes. No new JSON is written.
- **Rule 1.** No new `"quinn"` or `"student"` literal anywhere, tests included.
  `engine/tests/human_actor_literal.rs` stays green.
- **No new engine command or flag, and one new Tauri command (P17).** Quinn's removal path at
  Checkpoint B widens spec §6.3's "No new Tauri command" by exactly one: `remove_lane_date`. It is
  added to the console window's `generate_handler!` list; the wizard's list does not change. Both
  files are single-owner (HANDOFF §2), so the controller applies them as hand-off H2 after M, with
  the code given in §7. The command computes nothing: it calls one engine function with
  `console_ctx(vault)`, and the engine gates, checks and writes.
- **Privacy.** `site/` is not edited and `PRIVACY_VERSION` does not move (spec §7). No new
  telemetry row shape (D12).
- **The cloud.** No migration, no function. B1 (T9) runs a harness on `j-events` and changes nothing
  that ships from this branch.

**One push so far, recorded (2026-10-01).** The controller pushed the `events` branch once, at
Checkpoint B, so that Quinn's review could open the diff by link. P12 asks for Quinn's go before
any push of this branch, and for no push before B2 closes. This push came before B2, and is
recorded here as a departure from P12, not a change to it. Two things follow:
- The pushed commits (what the branch held at Checkpoint B, T0 through T3.2; the SDD ledger
  records the pushed head) are now on the remote. They are never rewritten: no rebase, no force push. M stays a
  merge (P12), so every later push is a fast-forward.
- Every later push keeps P12's rule: after B2 closes and after W closes, each with Quinn's go.

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
  event only from an archived card's `instances:`. A carried all-day instance (D4) is on no card,
  so the carry's ledger line holds its span (PQ3, P15) and `surface` reads it there (T4c). On a run
  where every feed fails, `rank` swaps its candidates for `read_roster`'s (the
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
- **As signed, a carried instance of an `event-accept` series has no verdict, so it is not in
  Coming up.** Coming up (`surface::coming_up`) reads the roster back through `read_roster` and
  keeps what `eventroster::relevant_events` keeps: a uid with an `obligation` or `opportunity`
  verdict and no `declined` line. A later instance of an answered `event-accept` series is never
  judged (T2b.3), and the signed carry writes ledger lines only for declines.
  `eventledger::record_answer` takes only `obligation` or `drop`. Nothing stores a carried all-day
  instance's span. A later instance of an answered `event-check` series gets
  `inherit_series_answers`' line, credited to the student, with no span. Quinn's final PQ3 answer
  closes all of this: the carry writes one answer line per carried date, with the series' verdict,
  the answering card's id and the date's span (P15). P16 runs the carry before inheritance, so the
  `event-check` kind gets that line too. Coming up then lists the date, and the lane draws it
  (T4c). An instance a model judged confidently before the answer keeps its verdict (F1's rule) and
  gains the line's span and provenance.
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
| `m2-editing` | `app/static/console.js`, `app/tests/static_assets.rs`, `app/tests/commands.rs` (its T4a, T4b), `engine/src/surface.rs` (`describe` only), `engine/src/lib.rs` (`pub mod profile;`), `engine/src/eventroster.rs` (its `read_dropped`; events does not edit this file) | T5, T7, T4b, T4c, H1 | Per M2's plan §6, events takes M2 in after M2 merges: T5 and T7 wait for it (P12). The engine tasks touch other functions, so they run now. |
| `gmail-connect` (merged, #26, `199cd1f`; already in `events` at `2d5298a`) | `app/static/console.js`, `app/tests/static_assets.rs`, `engine/src/enrich.rs` (D4, D7 and the transport stop; events adds only test 24 to its test module), `app/src/main.rs` (its three commands; events adds one, `remove_lane_date`, through H2 after M) | T5, T4 (test 24), H2 | Nothing left to take in. Every events task starts on a base that has Gmail's code, so test 24 is written against `enrich.rs` as Gmail left it. |
| `p3-registrar` (Pilot) | `engine/src/commitments.rs` (`create_confirmed_as`' body branch), `engine/src/cli.rs`, `console.js`, `static_assets.rs` | T1b, T4, T5 | Unchanged from spec §9: p3 rebases after events, keeping both body arms (events' `kind: event` arm tested first), both `cli.rs` passes and both label switches. |
| `two-desktop` (Launch) | its spec's producer table | none | Hand-off only (spec §9). |
| `email-forwarding` (MVP; sub-branches `email-engine`, `email-app`, `email-cloud`; unmerged) | `app/src/main.rs` (the console's `generate_handler!` list: email's H5 adds ten names, events' H2 adds one); `app/static/console.js` and `app/tests/static_assets.rs` (email's T15.1–T15.5); `app/src/commands.rs` (email puts its commands in `app/src/mail.rs`, so no shared function is expected; T0's diff confirms it); `app/tests/handler_lists.rs` (if email's test 37 lands there); `engine/src/approvals.rs` CL (email's T8); `engine/src/cli.rs` (email's H2 dispatch line) | H2, T5, T7, T3.1a–T3.2, T4 | Neither lane is later in the program, so the rule in email's §10 R2 applies: **the second of the two to merge takes the other in** (events by merge, P12; email as its plan says). The handler list and `lib.rs`/`main.rs` conflicts go to the main session: both sides' names kept, the wizard's list unchanged, and both lists recounted by script. `console.js` and `static_assets.rs` conflicts go to `console-ui`, with both sides kept and every static test re-run. `approvals.rs` conflicts go to `contract-engineer`. A `cli.rs` conflict goes to the main session in the dispatch, or to `contract-engineer` if it touches T4's events pass. |

`app/src/lib.rs` needs **no** hand-off from this lane. `app/src/main.rs` and `app/src/commands.rs`
need one, H2 (P17, §7): one command in `commands.rs` and its name in the console's list. Gmail moved
the console's list from 47 to 50, M2 added five more, and email-forwarding's H5 adds ten. Events
adds one, after M. Which of these lines `main` holds when H2 lands depends on merge order. So no
count is quoted here, and H2 states its result as names (§7). Gmail's
`app/tests/handler_lists.rs` is already in the `events` branch (`2d5298a`), so every gate runs it.
It pins names, not a count (checked on `main`: it asserts two lists and the three Google commands'
places). T7 adds a test in it for `remove_lane_date`'s place (§7).

**The email-forwarding plan's §10 R2 table does not list events yet** for `app/src/{lib,main}.rs`.
It was written when events added no Tauri command. T0 has the main session tell the email lane's
controller, so its R2 table gains the matching rows (`app/src/main.rs`, `console.js`,
`static_assets.rs`, and `handler_lists.rs` if shared), with the same rule and resolvers. This plan
does not edit the email plan.

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
    `accepted_check_series` and the carry over the union, its own commit), T2b.3 (the
    `judge_roster` call site), T2b.4 (PQ3: the carry's ledger line and its read rules, in
    `eventledger.rs`) and T2b.5 (PQ3: the carry writes that line).
  - PQ3 also adds T4c (`surface.rs`: carried dates in the lane, and Coming up's provenance) after
    T4b.
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
  (title 200, location 120, url 500). `start` and `end` are stored to the second and quoted by the
  emitter (`start: '2026-10-01T10:00:00'`), the carry line's span format; a minute card still
  reads (R1 minor 2; spec §5.1). The register task's body needs the URL (spec §4.2), and the
  card's own payload is the only input the settlement has (D10).
- **P6. The past-midnight true end is passed in the commitment mapping.** The builder adds an
  `ends:` key holding the clock label (`1am the next day`, or `midnight`). Only the `kind: event`
  body arm reads it; `proposed()` ignores unknown keys, and the key never reaches the frontmatter.
  The body is "You accepted this from your campus events." plus, for that shape, " Ends at <ends>."
- **P7. An `event-check` Accept books `level: hard`.** The student's answer word is `obligation`,
  so D1's level for an obligation applies.
- **P8. "Accepted" in Coming up.** An event is accepted when any of these holds:
  - its uid is listed on an archived `executed` card's `instances:` (`event-accept`, or
    `event-check` with `instances:`);
  - it is the `source_uid` of a `kind: event` note in `commitments/`;
  - its ledger entry holds a carry line (P15).

  T4b reads the first two. T4c reads the third and adds its provenance label. A carried date has a
  carry line unless a human answered it first. The second source still covers a carried timed date
  whose line a later human answer replaced, since its commitment stands. `ComingUp` gains
  `accepted: bool` and `provenance: Option<String>`, each serialised only when set, so every
  existing reference and `today.md` are byte-identical. `render::coming_up` (the page) is not
  changed: `today.md` lists a carried date like any judged event, without the label.
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
  and the same `proposed` ledger lines and carry lines (P15; a carry line's `from:` is a card's id,
  copied with the vault, so it compares as is). Journal timestamps and `id:` values are left out
  of the comparison, and the test says so in a comment. This is how spec test 4's "identical
  bytes" is read; the assertion is not weakened anywhere else.
- **P15. The carry's ledger line (PQ3 (b-prime), with its span).** One line per carried date. A
  carried date is an instance of an accepted series (an executed `event-accept` card, or under PQ1
  (a) an executed `event-check` card with `instances:`) that the card did not list and that starts
  today or later.
  - *Shape:* `- <uid> · <title> · verdict:<w> · by:agent:knowlu.carry · from:<card id> ·
    start:<YYYY-MM-DDTHH:MM:SS> · end:<YYYY-MM-DDTHH:MM:SS> · answered <run day>`.
    - `<w>` is the series' real verdict: the `event-accept` card's `verdict:` (`obligation` or
      `opportunity`), or `obligation` for an `event-check` series (P7).
    - `start` and `end` are the fetched event's own, as the feed gave them. They never come from
      `read_roster`'s read-back, because a roster-built run carries nothing (T2b.2).
    - `<card id>` is the answering card's `id:`.
    - A new `eventledger::record_carried_answer` writes the line (T2b.4).
  - *When:* the accept carry writes the line right after that date's booking step. A timed date's
    commitment goes through `create_confirmed`, journal first; a lane-shaped date has none.
    - The line does not wait on the booking's outcome. The booking retries on its own through the
      ever-written set. If a failed booking held the line back, the same run's
      `inherit_series_answers` (P16) would give an `event-check` series' date the student's line,
      and the carry would then never write its own.
    - The carry writes no line for a uid that already has a carry line, a human answer or a
      `declined` line. One exception, from PQ5 (b2) (Quinn, 2026-10-01; T2b.6): when a fetch
      moves an accepted lane date (both spans lane-shaped), the carry appends one more line of
      this shape, from the same card, with the new span. A card-listed lane date gets its first
      carry line the same way.
    - The line has no journal record of its own. No ledger line has one, and a record shape would
      be a further write and a `journal.rs` contract change, which Quinn ruled out.
  - *Read:* `load_ledger` reads the line into a new `LedgerEntry::carry` (the card id and the
    parsed span). The read rules (T2b.4):
    - the line sets the verdict where there is none, or where an `unsure` is unanswered;
    - it never flips a confident machine verdict (F1's rule), but its span and card id are still
      recorded;
    - it never overrides a human answer;
    - a later human answer replaces a verdict a carry line set, and clears `carry` (judge-once);
    - (PQ5 (b2), T2b.6) a later carry line whose `from:` names the first one's card replaces only
      `carry`'s span. Any other later carry line is ignored, as before.

    The `agent:` prefix (`provenance::is_agent`) marks the line as non-human.
  - *Used by:*
    - `write_roster` and Coming up, which list the date like any judged event;
    - `surface`'s lane, which draws a lane-shaped date from the line's span on each day it covers,
      whatever the roster holds (T4c), including after a later successful fetch drops it (§4,
      PQ3's trade-offs), until the student removes it with "Remove from my day", which writes a
      `declined` line (P17);
    - Coming up's provenance label, whose date is read from the card `from:` names (T4c).

    `state/` never syncs (`sync`'s note scan skips it), so the line is device-local like every
    ledger line.
- **P16. The carry runs before series inheritance in `rank`.**
  - *Why:* `inherit_series_answers` answers a later instance of an executed `event-check` series
    with a line credited to the student (the `by` that `settled_series` reads), with no `from:` and
    no span. If it ran first, the carry would find a human answer and skip the date (P15), and a
    carried all-day date of that series could never be drawn in the lane.
  - *Effect:* run second, with its code unchanged, `inherit_series_answers` skips every date the
    carry answered, by its own rule: it skips a uid with a confident verdict or an `answered_by`,
    and the carry's line gives one or both. It still answers what the carry does not reach:
    instances that started before today and are still running (`prefilter_events` keeps them),
    series whose card has no `instances:`, and rejected `event-check` series.
  - *Accepted by Quinn at Checkpoint B (2026-10-01).* This reverses the order spec §6.1's `cli.rs`
    row signed ("series inheritance, then … the carry"). It follows from PQ1 (a) and PQ3 together
    and needs no write. T4 builds it as planned, and T8 records it in a dated spec note.
- **P17. The removal path: "Remove from my day" writes the existing `declined` line (Quinn,
  Checkpoint B, 2026-10-01).** Quinn did not accept the lane trade-off (§4, PQ3). When a student
  accepted an all-day or multi-day event that a later fetch drops (cancelled or moved), they must be
  able to remove the stale lane date in the MVP.
  - *What is removable.* Every accepted lane date, from either source: a carried date (a carry
    line whose span is lane-shaped, T4c) or a date on an executed card's `instances:` (T4b). The
    console offers it whether or not the feed still has the event. It cannot tell reliably: the
    roster gate's three faults are in §10's fourth round. A student who changes their mind gets
    the same control.
  - *The write.* One `declined` line for the uid, `- <uid> · declined <today>`, through the
    existing `eventledger::record_declined`. This is the line a card's Decline writes (spec §4.3).
    No new line shape, no new verdict word, and `eventledger.rs` is not edited.
  - *The entry point.* `eventcarry::remove_lane_date(vault, uid, today, ctx)` (T4d,
    contract-engineer). In order:
    1. It refuses any actor but the vault's own human token: an `agent:` or `system:` actor, and a
       human actor that `journal::read_human_actor` does not name, each by name. It writes nothing.
       This is `write`'s gate made stricter, because an agent may never remove a date. `write`'s
       `human_gate` is private and on the contract list, so this gate is restated here and is not
       shared.
    2. It refuses a uid the ledger cannot read back (`ledger_reads`).
    3. It writes nothing for a uid already `declined`, and returns `Ok(false)`.
    4. It refuses a uid that is not one of the accepted lane dates. That list comes from T4c's
       `surface::accepted_events`, now `pub(crate)`, so the lane and the removal read one
       definition.
    5. It appends the line and returns `Ok(true)`.
  - *The command.* `remove_lane_date(view, uid)` (H2) runs inside `mutate`, which takes the
    console's locks and returns the new state. It passes `now_in(cs).date()` and
    `console_ctx(&cs.vault)?`. `commands.rs` computes nothing.
  - *Why the existing readers suffice.* `load_ledger` sets `declined` whatever the verdict.
    `relevant_events` drops a declined uid, so Coming up and `today.md` drop it. Never-ask-twice
    skips it. The carry's `carry_line` and `turned_down` skip it, so `rank` never writes a second
    line or rebooks it. M2's *Not shown* lists it as "you declined it" while the roster still holds
    it. T4c's reader already leaves out a carried date with a `declined` line. P17 adds one read
    rule there: it also leaves out a card-listed lane date with one.
  - *Why not the student's `drop` answer.* `record_answer(… "drop", <human actor>)` would credit the
    student on the line. But under `load_ledger`'s rules (F1, T2b.4) it settles only a uid whose
    verdict is `unsure`, unset, or set by a carry line. Two kinds of date fall outside that:
    - a card-listed date, whose verdict is confident or already the student's `obligation`;
    - a carried date a model judged confidently before the answer.

    For those the line is ignored and the lane keeps drawing the date. Covering them needs a human
    answer to replace a confident verdict. `a_human_answer_never_replaces_a_confident_verdict` pins
    the opposite (F1's invariant), and a pinned assertion may not change.
  - *No journal record.* A ledger line is not a note, `journal::OPS` has no op for one, and P15
    records that no ledger line has a journal record: a record shape would be a `journal.rs`
    contract change. So "journal first" has nothing to apply to here. The human gate (step 1) is
    what makes the write the student's.
  - *Two open questions for Quinn (§13), not plan decisions.* The fifth review (§10) found that
    each one either contradicts a design commitment or only half answers Quinn:
    - **PQ5: a moved event.** If the campus moves an accepted lane date and keeps its uid, the lane
      still draws the old day. A carried date's span is frozen at its first carry line (P15), and a
      card-listed date's span is frozen on the card. Removing the stale day declines the uid, so the
      real new date also leaves Coming up for good. Quinn's "moved" case is therefore only half
      answered. T4c and T4d wait on PQ5.
    - **PQ6: undo, or a confirm.** As drafted, the button asks once ("Remove <title> from your day?
      Knowlu won't show it again.") and offers no Undo. That follows Delete…'s precedent
      (`console.js`' `confirmDelete`). But VISION commitment 5 says a change the student asked for
      acts at once, with undo, and M2 keeps that commitment for edits (`offerBodyUndo`,
      `offerPrefsUndo`). A planner cannot depart from that alone. T5 waits on PQ6.

    *Both answered 2026-10-01 (§13).* PQ5 (b2): the carry re-records a moved lane span (T2b.6),
    so a move fixes itself on the next successful fetch and removal is needed for a cancellation.
    PQ6 (b): "Remove from my day" hides the date at once behind a 10-second Undo toast, and the
    console calls `remove_lane_date` only when the toast closes (T5). Neither adds a line shape.
  - *Two trade-offs named for Quinn with B2, for information:*
    - **Device-local, and not credited to anyone on the line.** The `declined` line has no `by`
      and no journal record, like a card's Decline line. Like every ledger line, it never syncs. A
      card's Decline survives a restore because D9 rebuilds it from the synced card. A removal has
      no card, so a restored vault draws a card-listed or still-fetched date again until the
      student removes it again. Keeping a removal across a restore needs a note, which is a new
      vault file; that is not built.
    - **On its own day only.** The console draws today's lane alone (`the_day` is built once, for
      today), so a cancelled future date is removable on the day or days it is drawn, which are
      the only days it shows.

## 4. Open questions, and Quinn's answers (2026-09-30)

Asked one at a time, each with its context, and answered by Quinn on 2026-09-30. Each answer is
recorded under its question; the options are kept as they were asked. None is open. PQ3 took three
rounds: (c), then (b-prime), then (b-prime) with the span on the carry's line, which is final.

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
  - **Answered 2026-09-30, final: (b-prime), with the span.**
    - *How it was reached.* Quinn first answered (c): list a carried instance at read time, with no
      write. (c) was blocked, because nothing stored links a carried instance to its series and a
      carried all-day instance is stored nowhere. Quinn replaced it with (b-prime). The planner then
      showed that the lane still could not draw a carried all-day date from that line plus what is
      stored, and Quinn added the span. The final answer:
    - For each carried date, timed and all-day, the carry writes **one** event-ledger answer line
      attributed to the carry, not the student:
      - `by:agent:knowlu.carry`: the `agent:` prefix, so judge-once treats it as non-user;
      - `from:<the answering card's id>`: the card holds who answered, when, and the series;
      - the series' real verdict: an accepted opportunity stays `opportunity`, so `record_answer`
        is widened to accept it.
    - The same line also carries the instance's span, `start:` and `end:`, exactly as the feed gave
      them. The all-day lane can then draw carried all-day, multi-day and zero-length dates on each
      day they cover, independent of the roster, and through a run where every feed fails.
    - Additive: new optional fields on a ledger line; every existing line stays byte-identical.
      Still exactly one line per carried date, and no other new write.
    - Coming up lists such a date like any judged event, labelled with its provenance ("Accepted ·
      from your answer to the series on <date>", the date read from the card).
  - **Applied, and buildable as approved.** The design is in P15 and P16. The tasks are T2b.4,
    T2b.5, T4, T4c, T5 and T8. It was checked against the `events` worktree at `854d727`:
    - `eventledger::{record_answer, load_ledger, LedgerEntry, clean_title}`;
    - `eventroster::{write_roster, read_roster, relevant_events}`;
    - `eventemit::{settled_series, inherit_series_answers, needs_check}`;
    - `approvals::settle_event_check` and its `executed_at` stamp;
    - `surface::{coming_up, the_day, load_with, ComingUp, TheDay}`;
    - `ids::{new_id, is_id}` and `provenance::is_agent`;
    - `sync`'s note scan, `DiscoveredEvent::{start, end}` and `rank`'s events pass.

    The lane half needs nothing beyond the line. Point by point:
    1. *The line* lives in `eventledger.rs`, beside `record_answer`. "Widened to accept
       `opportunity`" is built as a sibling entry point, `record_carried_answer`. It shares
       `record_answer`'s title cleaning and append, and takes `obligation` or `opportunity` with
       the carry's actor, a card id and a span. `record_answer` itself keeps its refusals, because
       two existing tests pin them for a human `by` (`record_answer_refuses_a_bad_word_actor_or_jid`,
       `an_answer_shaped_line_with_a_non_answer_word_does_not_supersede`), and a test's assertion
       may not change.
    2. *The lane.* With `start:` and `end:` on the line, `surface` classifies the date with
       `eventaccept::shape` and draws it on each day from its first to its last. It reads only the
       ledger, which is append-only and never rebuilt from the roster.
    3. *Coming up* lists the date through the roster, like any judged event. `rank` runs the carry
       before `write_roster`, so the same run lists it. The label's date is the `executed_at` of
       the archived card whose `id:` is the line's `from:`.
    4. *Judge-once* needs new read rules in `load_ledger` (P15, T2b.4). Today an answer line
       supersedes only `unsure`, and only with `obligation` or `drop`, and the rule does not look
       at the actor's prefix.
    5. *Journal first.* A timed date's commitment is journaled first, and the line follows its
       booking step. The line itself has no journal record, like every ledger line (P15). It does
       not wait on a booking that failed, which retries on its own.
    6. *Dates carried from an `event-check` series* (§10, third round) get the carry's line too.
       Under the signed order, `inherit_series_answers` would answer them first, credited to the
       student and with no span, and the carry would then skip them. P16 runs the carry first
       instead. `inherit_series_answers`' code is unchanged, and it skips those dates by its own
       rule.
    7. *What the line is not.* It is not a journal record, and it is not a note: it never syncs.
       Surface follows `from:` by scanning `archive/`, which it already reads; it does not need
       `ids::build_index`, which scans every note folder.
  - **Why the span unblocked the lane.** Without it, the lane could not get a carried date's span:
    - the line's `answered` date is the run day, not the event's;
    - the answering card lists only its own instances;
    - a lane shape gets no commitment (P4, Q1b, PQ2);
    - the roster keeps one `HH:MM–HH:MM` line under the start day, only from today on, and a
      feed-failure run rewrites `00:00–00:00` as `00:00–01:00`.

    `start:` and `end:` on the line supply the span, and the ledger is never rebuilt from the
    roster. So the lane draws a carried date on each day it covers, through a feed-failure run, with
    no other write.
  - **Trade-offs the plan decides, named for Quinn with Checkpoint B:**
    - P16's order (above). *Accepted at Checkpoint B (2026-10-01).*
    - An instance a model judged confidently before the answer keeps that verdict (F1's rule).
      Its line still records the span and provenance, so a confident `drop` is booked or drawn but
      not listed in Coming up. The alternative is to let a carry line override any machine verdict.
      That widens F1's invariant, so the plan does not take it.
    - A carried date whose commitment the student deleted stays listed as accepted, as a
      card-listed one does today.
    - A lane-shaped accepted date that a later successful fetch no longer has (the campus
      cancelled it, or the feed dropped it) stays drawn in the all-day lane on its own day or days,
      while Coming up, which lists through the roster, drops it. The two disagree for that date.
      This holds for a carried date (its line is never withdrawn) and equally for a card-listed one
      (T4b draws an executed card's `instances:`). The student cannot remove such a date in the
      MVP: it has no commitment and no card, and `needs_check` files no `event-check` for a uid
      with an answer. A human `drop` or a `declined` line would remove it (T4c), but no MVP surface
      writes one for such a uid. It is drawn only on the day or days it covers, so it passes with
      them. A timed date is unaffected in kind: its commitment stays too, and the student can
      delete it. *Not taken:* drawing a carried date only while its uid is in the roster. It would
      break Quinn's "independent of the roster", and `rank` swaps in `read_roster` only when every
      feed fails (`cli.rs`'s events pass), so one failed source among working ones would blank all
      of that source's carried dates; and the roster's relevant section keeps only dates whose start
      is today or later (`write_roster`'s window), so a multi-day date would leave the lane on its
      second day. *For Quinn at Checkpoint B:* accept
      this for the MVP, or ask for a removal path (for example, a Dismiss on a lane entry that
      writes a `declined` line). That path is a further write and a new console action, so the
      plan does not build it; it is the one open item this trade-off leaves (§10, fourth round).
      **Answered at Checkpoint B (2026-10-01): a removal path.** Quinn did not accept the
      trade-off. A dropped date is still drawn after the fetch, but the student can now remove it
      with "Remove from my day". That writes the existing `declined` line, which takes the date
      out of the lane and Coming up for good (P17; T4c, T4d, H2, T5, T7). The rejected roster gate
      stays rejected. Two questions this answer raises are open: PQ5 (a moved event) and PQ6
      (undo), §13.
    - A timed date whose booking failed is listed as accepted, without its block, until the next
      run books it (P15's *When*).
    - A lane-shaped instance that has already started when its series is first carried is not
      carried, as for commitments (D4's rule: an instance starting before today is not booked).
    - `today.md` lists a carried date without the provenance label (P8).
    - A series card with no valid `id:` gets its dates booked, but no lines, and a warning. `rank`
      runs `ids::ensure_ids` before its events pass, so this happens only when that repair
      failed; the next run that repairs the card writes the lines.
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
| T0 | main session | PQ1–PQ4 answered (PQ3 final, with the span) | none (git, ledger) | — |
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
| T2b.4 | `contract-engineer` (Opus, xhigh) | PQ3 (answered) | `engine/src/eventledger.rs` | T2b.3 |
| T2b.5 | `contract-engineer` (Opus, xhigh) | PQ3 (answered) | `engine/src/eventcarry.rs` | T2b.4 |
| T3.1a | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T2b.5 |
| T3.1b | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T3.1a |
| T3.2 | `contract-engineer` (Opus, xhigh) | — | `engine/src/approvals.rs` CL | T3.1b |
| B | `contract-reviewer` (Opus, xhigh) | checkpoint | `docs/reports/…-events-contract-review.md` | T3.2 |
| R1 | `reviewer` (Opus, high), beside B | — | `docs/reports/…-events-sonnet-review-1.md` | T3.2 |
| T4 | `contract-engineer` (Opus, xhigh) | P16 (accepted at B, 2026-10-01) | `engine/src/cli.rs`; test 24 in `engine/src/enrich.rs`' test module | B, R1 |
| T4b | `implementer` (Sonnet, high) | — | `engine/src/surface.rs` (`Loaded`, `load_with`, `the_day`, `coming_up`, `ComingUp`, one new private reader) | T4 |
| T2b.6 | `contract-engineer` (Opus, xhigh) | PQ5 (b2), answered 2026-10-01 | `engine/src/eventledger.rs` (the span rule), `engine/src/eventcarry.rs` (the moved-span line) | T4b |
| T4c | `implementer` (Sonnet, high) | PQ3 (answered), P17 (B), PQ5 (b2) | `engine/src/surface.rs` (T4b's reader, `the_day`, `TheDay`, `coming_up`, `ComingUp`) | T2b.6 |
| T4d | `contract-engineer` (Opus, xhigh) | P17 (B), PQ5 (b2) | `engine/src/eventcarry.rs` (`remove_lane_date`); one rank-level test in `engine/src/cli.rs`' test module | T4c |
| B2 | `contract-reviewer` (Opus, xhigh) on T4, T2b.6 and T4d + `reviewer` (Opus, high) on T4b and T4c | — | `docs/reports/…-events-review-2.md` | T4d |
| M | main session | push go | `main` merged into `events` | B2, and M2 merged (Gmail is already in) |
| H2 | main session | P17 (B) | `app/src/commands.rs`, `app/src/main.rs` (console list) | M, and T4d landed (the command calls `eventcarry::remove_lane_date`) |
| T5 | `console-ui` (Sonnet, medium) | PQ6 (b), answered 2026-10-01 | `app/static/console.js`, `app/tests/static_assets.rs` | H2 |
| T6 | `mechanical` (Sonnet, low) | — | `app/assets/campus/{none,university-of-alabama}.yaml`, `app/tests/scaffold.rs` | M |
| T7 | `test-writer` (Sonnet, medium) | — | `app/tests/commands.rs`, `app/tests/handler_lists.rs` | H2 |
| T8 | `docs-keeper` (Sonnet, medium) | — | `docs/surface/anatomy.md`, `docs/reference/engine-commands.md`, `docs/reference/app.md`, `docs/notes/2026-09-29-vision-program.md`, spec §12 note | T7 |
| T9 | `cloud-engineer` (Opus, high), on `j-events` | PQ4 (answered) | `docs/reports/…-events-b1.md` (harness run, not edited) | any time before T10 |
| W | `reviewer` (Opus, high) + `contract-reviewer` (Opus, xhigh) | checkpoint | `docs/reports/…-events-whole-branch-review.md` | T8 |
| T10 | main session | merge word | none (scratch profile) | W, T9, staging |

T2a and T2b interleave as the spec orders (T2b's first commit, then T2a, then the rest of T2b),
but serially, because one implementer runs at a time in the worktree. After M, the controller
applies H2. T5, T6 and T7 then touch disjoint files and run one after another.

**PQ3 (b-prime), with its span, is built in three tasks plus clauses in three others.**
- T2b.4: the line's shape and read rules in `eventledger.rs`. It goes to `contract-engineer` at
  xhigh, as Quinn directed.
- T2b.5: the carry writes the line, in `eventcarry.rs` (`contract-engineer`, xhigh).
- T4c: `surface` draws carried dates in the lane and labels them in Coming up (`implementer`).
- Clauses elsewhere: T4 runs the carry before inheritance (P16) and adds the rank-level test, T5
  renders the label, and T8 documents it.

T2b.2 and T2b.2b write no answer line, and no test in them asserts on the accept carry's ledger
lines: T2b.5 adds the line, and a test's assertion may not change later.

**P17 (Quinn's removal path, Checkpoint B) is built in five steps, each on its own file set:**
- T4c: the read side. The lane and the reader leave out a `declined` uid from either source, and
  `the_day` names each accepted lane date by uid (`implementer`).
- T4d: the write. `eventcarry::remove_lane_date`, gated, checked, one `declined` line
  (`contract-engineer`, xhigh).
- H2: the Tauri command and its place in the console's list (the controller, single-owner files).
- T5: the button and its Undo toast, PQ6 (b) (`console-ui`).
- T7: the command end to end and its handler-list place (`test-writer`).

T8 documents it.

**PQ5 (b2) (Quinn, 2026-10-01) is built in one task plus a clause in another.**
- T2b.6: the carry appends a moved lane span, and `load_ledger` reads it (`contract-engineer`,
  xhigh). It touches both PQ3 files, so it runs after T4b and before T4c, alone in the worktree.
- T4c: the lane draws a carry line's span over the card's (`implementer`).

B2's contract-reviewer reads T2b.6's commit alone, and W checks it.

### T0. Preflight (main session)

**Why the main session:** git state, sibling-lane checks and Quinn's answers are the controller's.
- Confirm the worktree is at `2d5298a` (or a later commit of this branch) with a clean tree, that
  `git merge-base --is-ancestor 199cd1f HEAD` succeeds, and that `app/tests/handler_lists.rs` is in
  the tree. Record the base in a new SDD ledger,
  `.superpowers/sdd/2026-09-29-ultracode/events-progress.md`.
- Run `git merge-tree --write-tree` of `events` (at `2d5298a`) against `m2-editing`,
  `p3-registrar`, `j-events` and each `email-forwarding` branch (`email-engine`, `email-app`, and
  the lane branch once it exists). Also run `git diff --name-only main...<branch>` for each. Gmail
  is merged and already in `events`. Record that the engine tasks (T1a–T4d) share only the files
  §2's table names, `eventledger.rs` included (T2b.4). Record whether email's diff touches
  `app/src/commands.rs` and `app/tests/handler_lists.rs`, and correct §2's email row if it does.
  Repeat before M and before the merge (spec §9).
- Tell the email-forwarding lane's controller about §2's email row: events' H2 adds
  `remove_lane_date` to the console list in `app/src/main.rs`, and T5 edits `console.js` and
  `static_assets.rs`. The second of the two lanes to merge takes the other in, and the main
  session resolves the handler list. Ask them to add the matching rows to their §10 R2 table.
  Record in the ledger that they were told.
- Record Quinn's answers of 2026-09-30 (§4):
  - PQ1 (a);
  - PQ2 (the all-day lane);
  - PQ3, final: (b-prime) with the span on the carry's line, which replaced (c);
  - PQ4 (as recommended).

  Also record P16 (the carry before inheritance) as the plan's consequence of PQ1 and PQ3, to be
  brought to Quinn with Checkpoint B.
- Record Quinn's Checkpoint B answers (§12), and that PQ5 and PQ6 are open (§13). Bring PQ5 to
  Quinn before T4c is dispatched. Bring PQ6 next, one at a time, and before T5 at the latest.

**Done when:** the ledger records the base, the overlap check (email-forwarding included), the
note to the email controller, the four answers, and the two open questions.

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
- `an_executed_card_answers_accept_and_a_rejected_one_decline`: the value also holds the card's
  file name and its `id:`, and an empty id when the card has none or an invalid one (P15's `from:`).
- `an_expired_or_other_status_answers_nothing` (ruling G1), including a `pending` card left in
  `archive/` by hand.
- `a_card_with_a_missing_or_empty_series_uid_answers_no_series`.
- `an_event_check_card_is_not_counted`. PQ1 (a) leaves this rule unchanged: accepted `event-check`
  series reach the carry through their own reader, `accepted_check_series` (T2b.2b), never through
  `answered_series`.
- `two_cards_for_one_series_the_lowest_file_name_wins`.
- `only_archive_is_read`: an `executed` card in `approvals/` answers nothing.

**Behaviour.** `answered_series(vault) -> BTreeMap<String, SeriesAccept>`, where the value holds
`accepted: bool`, the card's file name and its `id:` (empty unless `ids::is_id` accepts it; T2b.5
puts it on the carry's line). It reads `archive/` only, through
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
`event-check` series are T2b.2b, its own commit. PQ3's carry line is T2b.5's, also its own commit.
This task writes no answer line for a carried date, and no test in it asserts on the accept
carry's ledger lines: T2b.5 adds them, and a test's assertion may not change later.

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
  `state/events-seen.md` after an accept: T2b.5's tests do.)
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
    set is built in one private helper, so T2b.2b widens it in one place. The accept carry handles
    one carried date at a time, so T2b.5 adds the line in one place, after each date's booking step.
  - The ledger map is updated as `load_ledger` would read the new `declined` lines. The accept
    carry writes no ledger line in this task, as spec §5.4 says; T2b.5 adds PQ3's line.
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
  `state/events-seen.md`: T2b.5 adds the carry's line and its tests.)
  `answered_series` still does not count the card. A hand edit can make an archived `rejected` `event-accept` card answer
  `lx:9` too; then nothing is booked for the series, because `answered_series`' answer wins.
  Built as `read_roster` builds events (series equal to uid), a one-instance `ics:fair` whose
  executed `event-check` card lists it writes nothing.
- `an_event_check_card_without_instances_carries_nothing`: an `executed` `event-check` card with
  no `instances:` key (as filed before this lane), and its series' later instance in the events: no
  commitment and no journal record. The same holds for a `rejected` and an `expired` `event-check`
  card that carries `instances:`.
- `a_rejected_or_pre_lane_check_card_blocks_its_series` (added by the review of T2b.2b): a
  `rejected` card followed by an `executed` card with `instances:` for `lx:9`, and an `executed`
  card without `instances:` followed by one with them for `lx:8`. A later instance of either gets
  no commitment and no journal record. `settled_series` and `accepted_check_series` agree on every
  series. A control series (`executed` with `instances:`, then `rejected`) is booked.

**Behaviour.** PQ1 (a), P7.
- `accepted_check_series(vault) -> BTreeMap<String, CheckAccept>`: reads `archive/` only, through
  `approvals::sorted_md`, for `type: approval`, `kind: event-check` cards. The value holds the
  card's file name, its `id:` (as T2b.1's) and the uids its `events:` lists.
  A series is claimed as `settled_series` claims it: the lowest-named `executed` or `rejected` card
  with a non-empty `series_uid` claims it, and an `expired` card or any other status claims
  nothing. The series carries only if its claiming card is `executed` with an `instances:`
  sequence. A claiming `rejected` card (its series already has `drop` lines through
  `inherit_series_answers`), or a claiming card with no `instances:`, blocks the carry for its
  series, whatever a later card says. *Changed by the review of T2b.2b. Checkpoint B confirms
  this reading.* It reads no ledger.
- T2b.2's accepted-set helper becomes the union of `answered_series`' executed entries and
  `accepted_check_series`. Where one series is in both readers, which only a hand edit can make,
  `answered_series` wins, so a series declined on an `event-accept` card is never booked. The level
  is `hard` for an `event-check` series.
- Only the accept carry calls it: not the decline carry, the emitter or `judge_roster`, which keep
  the union of `settled_series` and `answered_series`.

**Done when:** the three tests pass, and T2b.1's and T2b.2's tests pass unchanged.

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

### T2b.4. PQ3: the carry's ledger line and its read rules (`eventledger.rs`)

**Agent:** `contract-engineer` (Opus, xhigh), as Quinn directed. **Why:** the ledger records every
answer a student gave about an event, and every emitter, `judge_roster` and Coming up read it. A
wrong read rule silently loses a human answer, or lets an agent line stand over one (judge-once).
The file is off the contract list, but its lines are a contract with existing vaults.
**Files:** `engine/src/eventledger.rs`. **Answer applied:** PQ3, (b-prime) with the span (P15).

**Tests first** (in the module's tests; a human `by` is `journal::HUMAN_ACTOR`, never a literal):
- `the_carry_line_is_byte_exact`. The call `record_carried_answer(vault, "lx:77:3", "Career fair",
  2026-10-01, "opportunity", "appr_0123456789", 2026-10-08T10:00:00, 2026-10-08T15:00:00)` writes,
  after the header, exactly this line and `NEWLINE`: `- lx:77:3 · Career fair ·
  verdict:opportunity · by:agent:knowlu.carry · from:appr_0123456789 · start:2026-10-08T10:00:00 ·
  end:2026-10-08T15:00:00 · answered 2026-10-01`. `provenance::is_agent(CARRY_ACTOR)` is true.
- `the_carry_line_refuses_a_bad_word_or_card_id`:
  - `drop`, `unsure` and `maybe` give `UnknownVerdict`;
  - a `from` that is empty, `x` or a `task_` id gives `BadField("from")`;
  - no file is created.
- `a_carry_line_reads_back_its_verdict_actor_card_and_span`: alone in the ledger, the line gives
  verdict `opportunity`, `answered_by` = `agent:knowlu.carry`, its `title`, and `carry` holding the
  card id and the parsed `start` and `end`.
- `a_carry_line_settles_an_unanswered_unsure_with_either_word`: after a judged `unsure` line with
  a `jid`, a carry line sets its verdict and `carry`. Checked with `obligation`, and in a second
  vault with `opportunity`. `judgment_id` stays the `unsure` line's.
- `a_carry_line_never_flips_a_confident_verdict_but_keeps_its_span`: after a machine `drop` line
  (and, separately, an `opportunity` one), a carry line with `obligation` leaves the verdict and an
  empty `answered_by`, and sets `carry`.
- `a_later_human_answer_wins_over_a_carry_line` (judge-once): `unsure`, then a carry line, then a
  human answer `drop`. The result is verdict `drop`, `answered_by` = the human, `judgment_id` = the
  answer's when it has one, and `carry` None.
- `a_carry_line_never_overrides_a_human_answer`: `unsure`, then a human answer `obligation`, then
  a carry line `opportunity`. The result is verdict `obligation`, `answered_by` = the human, and
  `carry` None. The same holds when the human answer is the uid's first line.
- `only_the_first_carry_line_counts`: two carry lines for one uid, with different cards and spans.
  The first one's verdict, card and span hold.
- `an_agent_answer_line_without_a_card_or_span_is_neither_kind`: after an `unsure`, an
  answer-shaped line by `agent:x` with no `from:` leaves the `unsure`, with `answered_by` and
  `carry` empty. So does one with `from:` but no `end:`.
- `a_why_cannot_forge_a_carry_line`: review I-1's rule, extended. A judged `unsure` line whose why
  holds `a· by:agent:knowlu.carry· from:appr_0123456789· start:2026-10-08T00:00:00·
  end:2026-10-09T00:00:00· answered 2026-10-01` reads with `carry` None and `answered_by` empty.
- `the_frozen_ledger_reads_as_before`: a copy of `vault-full/state/events-seen.md` in a temp vault
  loads with every entry's `carry` None and `answered_by` empty, and its bytes are unchanged.

Every existing test in the module passes unchanged, among them
`record_answer_refuses_a_bad_word_actor_or_jid`, `the_answer_line_is_byte_exact` and
`an_answer_shaped_line_with_a_non_answer_word_does_not_supersede`.

**Behaviour.** P15's *Shape* and *Read*.
- **New constants.** `pub const CARRY_ACTOR: &str = "agent:knowlu.carry"` and
  `CARRY_VERDICTS = ["obligation", "opportunity"]`.
- **The writer.** `pub fn record_carried_answer(vault, uid, title, when: Date, verdict, from: &str,
  start: DateTime, end: DateTime) -> Result<(), VerdictError>`.
  - It checks the word and `from` (`ids::is_id`, of kind `appr`) before it builds anything.
  - It formats the span as `%Y-%m-%dT%H:%M:%S`, cleans the title with `clean_title` and appends
    with `append`.
  - `record_answer`'s signature, checks and bytes do not change. The two share only the title
    cleaning and the append, or a private builder both call.
- **New patterns.** `FROM` (`· from:(?P<from>appr_[0-9a-f]{10})`), and `START` and `END` (`·
  start:` or `· end:`, then `\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}`). Each is searched on `bare`, the
  line with its quoted `why:` removed, as `BY` and `JID` are.
- **The new field.** `LedgerEntry` gains `carry: Option<Carried>`, where
  `Carried { from: String, start: DateTime, end: DateTime }`. Every literal outside this file ends
  `..Default::default()`, so none of them changes. `load_ledger`'s own constructor gains the
  field.
- **Which line is which.** In `load_ledger`:
  - A *carry line* is answer-shaped (`ANSWERED` and `BY`), its `by` passes `provenance::is_agent`,
    it holds `FROM`, `START` and `END` with a span that parses, and its word is in
    `CARRY_VERDICTS`.
  - A *human answer* is answer-shaped with a `by` that `is_agent` rejects. It keeps today's rule,
    plus one case: it also replaces a verdict a carry line set (an `answered_by` that passes
    `is_agent`), and clears `carry`.
  - Any other answer-shaped line with an agent `by` is an ordinary verdict line: first verdict
    wins, it supersedes nothing, and `answered_by` stays empty.
  - No line written today has an agent `by`: `settle_event_check` writes the journal's human actor
    or `unknown`, and `inherit_series_answers` copies one of those. So no existing line reads
    differently.
- **The module doc** gains a section on the carry line, beside "The one exception to 'first
  verdict wins'".

**Done when:** the eleven tests pass, and every existing `eventledger.rs`, `eventemit.rs`,
`eventroster.rs` and `approvals.rs` test passes unchanged.

### T2b.5. PQ3: the carry writes its line (`eventcarry.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** it writes a line that no human wrote into
the record of the student's answers, once per carried date, with no card in front of it. A wrong
rule writes over a human answer, or writes twice. **Files:** `engine/src/eventcarry.rs`.
**Answer applied:** PQ3, (b-prime) with the span (P15).

**Tests first** (at `eventcarry::run`, as T2b.2's are):
- `the_carry_writes_one_line_per_carried_date`. The vault:
  - an executed `event-accept` card for series `lx:77`, with `verdict: opportunity` and an `id:`,
    listing `lx:77:1`;
  - events `lx:77:1`; a later timed `lx:77:2`; a later two-day all-day `lx:77:3`; a later
    zero-length `lx:77:4`; and `lx:77:0`, which starts before today.

  The result:
  - exactly one carry line each for `:2`, `:3` and `:4`, in `(start, uid)` order, each with
    `by:agent:knowlu.carry`, `from:` the card's id, `verdict:opportunity`, and `start:` and `end:`
    equal to the event's own;
  - no line for `:1` (listed on the card) or `:0` (before today);
  - `:2` also has its commitment and that commitment's journal `create` record.
- `a_failed_booking_still_gets_its_line_and_is_rebooked_next_run` (journal first). The test
  forces `:2`'s commitment create to fail, as T2a.1b's failure test does.
  - That run gives a warning and no commitment and no journal record for `:2`. `:2` still gets its
    one line, and so do `:3` and `:4`.
  - The next call, with the fault gone, writes `:2`'s commitment, journal record first, and no
    second line.
  - In the vault of `an_accepted_event_check_series_gets_the_carrys_line`, the same fault followed
    by `inherit_series_answers` (P16's order) leaves `lx:9:2` with the carry's line only.
- `a_second_run_writes_nothing`. A second call leaves the ledger's bytes and the journal unchanged.
  So does a third call made after `:2`'s commitment is deleted through `write::delete`: nothing is
  rebooked and no second line is written.
- `an_accepted_event_check_series_gets_the_carrys_line`: in T2b.2b's vault (series `lx:9`),
  `lx:9:2` gets one line, `verdict:obligation`, with `from:` the `event-check` card's id, beside its
  commitment.
- `the_carry_never_writes_over_a_human_answer_or_a_decline`:
  - a carried uid with a human answer line (actor from `journal::read_human_actor`) gets no carry
    line;
  - a carried uid with a `declined` line gets none;
  - a rejected series' later instances get their `declined` lines and no answer line.
- `a_confidently_judged_carried_date_gets_its_line_and_keeps_its_verdict`: `lx:77:2`, judged
  `obligation` before the answer, gets one line, and `load_ledger` then gives `obligation` with
  its `carry`.
- `an_existing_vaults_ledger_bytes_are_unchanged`. Two vaults: a copy of `vault-full` (copied
  first, never used in place), and a vault with an accepted series whose every instance is on its
  card. In each, `state/events-seen.md` is byte-identical after `run`, or still absent where it was
  absent.
- `a_card_without_an_id_books_but_writes_no_line`: the dates are booked, no line is written, and
  one warning names the card.
- `the_carrys_lines_are_deterministic`: two `run`s over copies of one vault give byte-identical
  ledgers (P14).

T2b.2's `roster_read_events_carry_nothing` also passes unchanged, so a run built from the roster
writes no line.

**Behaviour.** P15's *When*.
- **The line.** After each carried date's booking step (T2b.2's per-date hook), whatever the
  booking's outcome (P15's *When*), the carry calls `record_carried_answer` when all of these hold:
  the uid's entry has no `carry`, it has no human answer (its `answered_by` is empty or passes
  `is_agent`), and it has no `declined` line. The call takes:
  - the series' verdict: the card's `verdict:`, as T2b.2's level reads it, or `obligation` for a
    `CheckAccept`;
  - the card's id;
  - the event's `start()` and `end()`.
- **A card with an empty id.** The carry skips the line and warns: `carry: <file> has no id; its
  dates are booked but not listed`.
- **The ledger map** is updated as `load_ledger` would read the new line (P15's *Read*), so the
  same run's `inherit_series_answers` (P16), roster and emitters see it.
- **Failures.** A write failure is a warning, and the next `rank` retries. Nothing here panics.
  The line has no journal record, and the carry writes no other file.

**Done when:** the nine tests pass, and T2b.1's, T2b.2's, T2b.2b's and T2b.4's tests pass
unchanged.

**Review fix (one commit; it also touches `eventledger.rs` and `eventemit.rs`).**
- The carry writes a date's line only when `eventledger::carried_answer_reads_back` reads it back
  exactly as written. The check builds the line as `record_carried_answer` does and reads it with
  `load_ledger`'s own per-line rules (`read_lines`, split out of `load_ledger` unchanged).
- A date gets no line and the same warning, and is booked all the same, when the line would not
  read back: a feed title that starts with a field head (`verdict:tbd`, `by:appointment`, `from:`,
  …) or holds a line boundary, a uid outside the head's class, or a span year that is no `\d{4}`.
- `eventemit::eligible_events` skips an entry with a `carry`, so T4's digest (step 5,
  `event_cards` off) never proposes a carried date again.
- Tests: `a_title_the_ledger_cannot_read_back_gets_no_carry_line` and
  `a_carried_opportunity_date_is_never_in_the_digest` (`eventcarry.rs`), and
  `the_carry_line_reads_back_only_as_written` (`eventledger.rs`).
- *Checkpoint B reads this fix with T2b.5.*

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
T2b.2b, T2b.3, T2b.4, T2b.5, T3.1a, T3.1b and T3.2. It reads them against spec §4, §5, §6.1, §8,
D1–D11, Quinn's PQ1 (a) and PQ3 (P15), and checks in particular:
- that `answered_series` has one definition and the emitter has no copy;
- reading T2b.2b's commit alone: that `accepted_check_series` feeds only the accept carry, never
  the decline carry, the emitter or `judge_roster`; that a cross-reader overlap goes to
  `answered_series`; that an `event-check` card with no `instances:` carries nothing; and that a
  series is claimed as `settled_series` claims it, so a claiming `rejected` card or a claiming
  card with no `instances:` blocks a later card's carry (the reading the review of T2b.2b chose);
- reading T2b.4's commit alone, that the carry's line is additive:
  - `record_answer`'s signature, refusals and bytes, `VALID_VERDICTS` and `ANSWER_VERDICTS` are
    unchanged, and `opportunity` is accepted only on the carry's entry point;
  - every existing line reads as before;
  - a carry line never stands over a human answer, and a later human answer replaces one;
  - a carry line never flips a confident machine verdict;
- reading T2b.5's commit alone:
  - exactly one line per carried date, written after its booking step whatever the outcome;
  - none over a human answer, a `declined` line or an earlier carry line;
  - a roster-built run writes none;
  - no write besides the line: no journal record shape and no other file;
  - with its review fix, no line the reader would misread (the feed's title included), and no
    carried date in the digest;
- that the ever-written set leaves out approval cards and includes `archive/`;
- that every note is written journal first, through `write::create`, by an `agent:` actor;
- that a failure leaves the card `approved` and a retry duplicates nothing;
- that the existing arms and the frozen references ran unchanged.

Fixes go back to `contract-engineer`, and the reviewer re-reads them. The controller then brings
Quinn the report with two diffs: the carry's (with its ledger line) and the settlement's. This is
spec §13's second checkpoint. With them, the controller brings §4's PQ3 trade-offs and P16, the
carry ahead of inheritance in `rank`, which reverses spec §6.1's signed order and is built in T4.
Among the trade-offs it names one by itself, with its one question: an accepted lane date that a
later fetch drops stays drawn while Coming up drops it, and the student cannot remove it in the
MVP. If Quinn accepts it, T4c's `a_dropped_lane_date_stays_drawn_and_unlisted` pins it as
written. If Quinn asks for a removal path, that is a further write: it goes back to the planner,
which revises T4c, T5 and that test before T4c is written.
If Quinn keeps the signed order, T4 keeps it, and the planner rewrites T4's `event-check` clauses
before T4 is written. T2b.5's unit tests call `run` alone, so they hold either way. T4 starts when
this report and R1's have no open Critical or Important finding.

**Quinn's answers at Checkpoint B (2026-10-01).**
- P16 is accepted, so T4 is built as written.
- The lane trade-off is not accepted, and Quinn asked for a removal path. The planner revised the
  plan before T4c was written:
  - new P17;
  - T4c's tests and reader: `a_dropped_lane_date_stays_drawn_and_unlisted` is replaced by
    `a_dropped_lane_date_is_drawn_with_its_uid_until_removed`, and the lane names each accepted
    date by uid;
  - new tasks T4d and H2;
  - T5's button, T7's end-to-end tests, and T8, B2, W, T10, §6–§9.

The record is §12.

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
  register task; two more `rank` runs write no note, no journal record and no second ledger line
  for either uid, and file no card for them. A second series in the same vault is an accepted
  `event-check` series (PQ1 (a)). Step 1 gives its later timed instance one commitment and one
  line, the carry's. Once deleted, the commitment stays deleted.
- `rank_gives_each_carried_date_the_carrys_line_and_lists_it` (PQ3, P16). The vault holds an
  accepted `event-accept` series (`opportunity`) and an accepted `event-check` series with
  `instances:`. The feed has a later timed and a later two-day all-day instance of each. A third
  series was answered on an executed `event-check` card filed before this lane (no `instances:`),
  and the feed has one later instance of it. After one `rank`:
  - each of the four later uids has exactly one line in `state/events-seen.md`, the carry's, with
    `from:` its card's id and its feed span, and no inherited line;
  - the third series' instance has `inherit_series_answers`' line, as today, and no carry line;
  - the roster's relevant section lists all five.

  A second `rank` adds no line. A third, with every feed failing, leaves the ledger's bytes
  unchanged.
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

**Behaviour.** Spec §6.1's `cli.rs` row, with P16's order (accepted by Quinn at Checkpoint B,
2026-10-01). Inside the existing
`if events_config.sources…` block, in this order:
1. `eventcarry::run`: D9's rebuild, then the carry and its lines (T2b.5), its warnings into
   `ledger:` lines. It is passed the same `candidates` as every later step, so on a run built from
   the roster it writes nothing (T2b.2), and no step reads series from anywhere else;
2. `inherit_series_answers` (code unchanged), now after the carry (P16). It works over the ledger
   map the carry updated, so it skips every date the carry answered;
3. `write_roster` (unchanged, now after both, so the roster reads the carried declines and lines);
4. obligation cards, sized to `remaining_budget`;
5. the digest **only when `event_cards` is off**, sized to what is left;
6. the `unsure` checks, sized to what is left;
7. opportunity cards **only when `event_cards` is on**, sized to what is left;
8. `relevant_events` for Coming up (unchanged).

Every card filed adds to `approvals.pending`. With the switch off and no obligation verdict (the
`vault-full` case), steps 1, 4 and 7 write nothing and the digest is sized exactly as today, which
is what keeps `golden-today-full.md` byte-identical. `vault-full` has no event card, so step 2
writes nothing there either, and the swap of steps 1 and 2 cannot move the golden.
`surface::delta` is not edited.
**Done when:** the ten tests pass, and every existing `cli.rs` test, `oracle.rs` and
`surface_oracle.rs` pass unchanged. T4's commit is reviewed by `contract-reviewer` in B2.

### T4b. The all-day lane and "Accepted" (`surface.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** read-only: `surface` never writes, so an error
misdraws a lane but never changes vault bytes. **Files:** `engine/src/surface.rs` (`Loaded`,
`load_with`, `the_day`, `ComingUp`, `coming_up`, and one new private reader). **PQ3:** T4c, next,
adds the carry's lines. This task's reader reads cards and `commitments/` only, and no test in it
reads a carry line.

**Tests first** (spec test 22a, plus P8's):
- `an_accepted_two_day_event_is_in_the_all_day_lane_on_each_day` (22a): after Accept of an all-day
  two-day event, `the_day` for each day lists it in `all_day`, `open_hours` equals the value before
  the Accept, and the day after lists nothing. An accepted zero-length event (PQ2) is in `all_day`
  on its own day only, and `open_hours` does not move.
- `an_accepted_event_is_marked_in_coming_up` (P8): an accepted timed event and an accepted all-day
  event (each on an `executed` card's `instances:`, each with a confident verdict) carry
  `accepted: true`. So does a confidently judged timed instance with a `kind: event` commitment the
  carry booked, for a series of each kind (PQ1 (a)), through its commitment alone. An unanswered
  one carries none.
- `accepted_is_absent_from_the_json_when_false` (P8): the serialised `ComingUp` of an unaccepted
  event has no `accepted` key, so `surface_oracle.rs` stays byte-identical.

**Behaviour.** Spec §4.2's lane, Q1b (i), §6.1's `surface.rs` row, P4, P8.
- One new private reader, `accepted_events(vault) -> AcceptedEvents`, reads `archive/` for
  `executed` `event-accept` and `event-check` cards with `instances:`, and `commitments/` for
  `kind: event` source uids. It returns the accepted uid set and the lane-shape instances (all-day,
  multi-day and zero-length, by `eventaccept::shape`; uid, title, first day, last day). The uid is
  kept for T4c, which names each lane date by it for P17's removal. An unreadable note is skipped.
  A carried instance is on no card; T4c adds carried dates from the ledger.
- `load_with` calls it once and keeps both in new `Loaded` fields, for `the_day`.
- `coming_up(vault, today, now)` keeps its signature (it never receives a `Loaded`) and calls
  `accepted_events(vault)` itself for the uid set. That is one more read of `archive/` and
  `commitments/` per call, accepted for simplicity. No public signature changes, so `app/src` (which
  calls neither `coming_up` nor `load_with` directly at `4aa6560`; the implementer re-checks with a
  search) and M2's edits to `describe` are unaffected.
- `the_day` appends each lane instance covering `today` to `all_day`, after the calendar's own, in
  `(first day, title, uid)` order. The uid breaks a tie between two dates with one title, so the
  order is total. Capacity is not touched (anatomy §3.7).
- `ComingUp` gains `accepted: bool` with `#[serde(skip_serializing_if = …)]` for false;
  `coming_up` sets it from the uid set.
- No write, and `render` is untouched.

**Done when:** the three tests pass, and `surface_oracle.rs` and every `surface.rs` test pass
unchanged.

### T2b.6. PQ5 (b2): the carry re-records a moved lane span (`eventledger.rs`, `eventcarry.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** it reopens two reviewed contract tasks,
T2b.4's read rules and T2b.5's writer. It appends a line no human wrote to the record of the
student's answers. A wrong rule moves a date the student accepted, writes on every run, or stands
over a human answer or a removal. **Files:** `engine/src/eventledger.rs` (one read rule, one
sentence of module doc) and `engine/src/eventcarry.rs` (the writer and its tests). **After:** T4b;
before T4c, whose reader reads this rule. **Answer applied:** PQ5 (b2) (Quinn, 2026-10-01, §13).

**First check, before any test.** The result goes in the SDD ledger. Read every pinned assertion
over carry lines and confirm that none forbids a second carry line for a changed span:
- T2b.4's eleven, among them `only_the_first_carry_line_counts` (two cards: the same-card rule
  below leaves it passing unchanged) and `a_carry_line_never_overrides_a_human_answer`;
- T2b.5's nine and its review fix's two. `a_second_run_writes_nothing`,
  `an_existing_vaults_ledger_bytes_are_unchanged` and `the_carrys_lines_are_deterministic` each run
  over an unchanged feed;
- T4's `rank_gives_each_carried_date_the_carrys_line_and_lists_it` and
  `rank_twice_never_rebooks_a_deleted_carried_note`.

Also confirm that no existing test vault lists a lane-shaped date on a card with a span its feed
gives differently. Such a vault would gain a line under this task's rule and fail its own byte
assertion. If an assertion forbids a changed span, or a vault would gain a line, stop. No assertion
and no test vault is edited, and the controller brings PQ5 back to Quinn, with (a) recommended over
(c) (§13).

**Tests first** (at `eventcarry::run`, as T2b.5's are; a human actor comes from
`journal::read_human_actor`):
- `the_carry_records_a_moved_lane_span_once`. The vault holds an executed `event-accept` card for
  series `lx:77`, with `verdict: opportunity` and an `id:`, listing `lx:77:1`. It also holds the
  carry line T2b.5 wrote for `lx:77:3`, a two-day all-day date Fri–Sat. The feed now gives
  `lx:77:3` as all-day Sat–Sun.
  - One `run` appends exactly one line, in the carry's shape: `from:` the card's id,
    `verdict:opportunity`, and Sat–Sun's `start:` and `end:`. `load_ledger` then gives `lx:77:3`
    the Sat–Sun span in `carry`, with its `from:`, verdict and `answered_by` as before.
  - A second `run` with the same feed leaves the ledger's bytes unchanged.
  - With the feed moved back to Fri–Sat, a third `run` appends one line with Fri–Sat's span.
  - After it, a human `drop` answer clears `carry`, as T2b.4's rule says.
  - Each of these, in its own vault, writes no line and leaves the ledger's bytes unchanged:
    - the same move after a `declined` line for `lx:77:3` (P17's removal);
    - the same move on a roster-built run (`read_roster`'s events);
    - a move to a timed span (Sat 10:00–15:00), which also books no commitment and writes no
      journal record;
    - a move to a span that starts before today.
  - Two `run`s over copies of the first vault give byte-identical ledgers (P14).
- `a_card_listed_date_moved_by_the_feed_gets_a_carry_line`. The vault holds an executed
  `event-accept` card, with `verdict: obligation` and an `id:`, whose `instances:` lists `lx:88:1`,
  one-day all-day on Fri. `lx:88:1` has a confident `obligation` verdict and no carry line. The feed
  gives `lx:88:1` all-day on Sat.
  - One `run` appends one carry line: `from:` the card's id, `verdict:obligation`, and Sat's span.
    `load_ledger` keeps the verdict `obligation` and sets `carry`.
  - A second `run` writes nothing.
  - With the feed as the card has it (Fri), no line is written.
  - A card with an empty `id:` writes no line and gives T2b.5's warning.
  - In a second vault, an executed `event-check` card with `instances:` whose own uid is
    lane-shaped and moved, and holds the student's answer line from `settle_event_check`, gets no
    line (§13's *Not covered*).

**Behaviour.**
- **The read (`eventledger.rs`).** One rule in `load_ledger`'s per-line rules (`read_lines`):
  - a carry line for a uid whose entry already has `carry` replaces `carry`'s `start` and `end`
    when its `from:` equals `carry`'s;
  - the verdict, `answered_by`, `judgment_id` and `declined` are untouched;
  - a carry line from another card, or after a human answer, is ignored, as today.

  The module doc's carry section gains one sentence saying so.
- **The write (`eventcarry.rs`).** After T2b.5's lines, in the same run and over the same
  `candidates` (so a roster-built run writes nothing). For each fetched event, in `(start, uid)`
  order, the carry calls `record_carried_answer` when all of these hold:
  - the uid is an accepted lane date: its entry has `carry`, or it is on the `instances:` of an
    executed `event-accept` card, or of an `event-check` card that T2b.2b's reader accepts;
  - the span it is drawn with (the entry's `carry` span if it has one, else its `instances:`
    entry) is lane-shaped by `eventaccept::shape`;
  - the fetched span differs from that span, is lane-shaped too, and starts today or later;
  - the uid has no `declined` line and no human answer (`answered_by` empty or passing
    `is_agent`);
  - `carried_answer_reads_back` holds for the new line (T2b.5's review fix).

  The line takes the series' verdict as T2b.5 does (the card's `verdict:`, or `obligation` for an
  `event-check` card), the card's id, and the fetched `start()` and `end()`. The in-run ledger map
  is updated as `load_ledger` would read the line.
- **Nothing else.** No commitment, no journal record, no other file. A write failure is a warning,
  and the next `rank` retries. Nothing panics.

**Done when:** the two tests pass, and T2b.1's to T2b.5's tests (the review fix's included), T4's
and T4b's pass unchanged. `contract-reviewer` reads this commit alone in B2.

### T4c. PQ3: carried dates in the lane, and Coming up's provenance (`surface.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** read-only, like T4b: an error misdraws a lane or
a label but never changes vault bytes. `reviewer` reads it in B2. **Files:** `engine/src/surface.rs`
(T4b's `accepted_events`, `load_with`, `the_day`, `TheDay`, `ComingUp`, `coming_up`). **Answers
applied:** PQ3, (b-prime) with the span (P15); and the read side of Quinn's removal path (P17,
Checkpoint B, 2026-10-01). T4d writes the removal's line, and this task only reads it.
**PQ5 answered (b2), 2026-10-01 (§13).** The tests below include its one test,
`a_moved_lane_date_is_drawn_on_its_new_day_only`, and the behaviour includes its span rule. It runs
after T2b.6, whose read rule it relies on.

**Tests first.** Vaults are built by hand: carry lines through
`eventledger::record_carried_answer`, a removal's line through `eventledger::record_declined` (the
line T4d's `remove_lane_date` writes), and the roster through `eventroster::write_roster`, as
`rank` writes it.
- `a_carried_lane_date_is_drawn_on_each_day_it_covers`. Carry lines for three dates: a two-day
  all-day date, a zero-length date, and a Fri 5pm–Sun 2pm date. No card lists them, and there is
  no roster file.
  - `the_day` lists the first in `all_day` on both its days, the second on its day, and the third
    on Fri, Sat and Sun.
  - The day after each lists nothing.
  - `open_hours` is what it was without the lines.
  - A carried timed one-day date is not in `all_day`.
- `the_lane_holds_whatever_the_roster_says`: the same vault, twice. Once with the roster as a
  feed-failure run leaves it (each date read back as `00:00–01:00`), and once with no roster.
  `the_day` gives the same `all_day` for each covered day.
- `a_carried_date_is_listed_with_its_provenance`: a carried timed date and a carried all-day date,
  each in the roster's relevant section and each with its carry line. Their `from:` names an
  archived executed card with `executed_at: "2026-09-24 10:15"`. `coming_up` lists both with
  `accepted: true` and `provenance: "Accepted · from your answer to the series on Thu 9/24"`.
- `a_provenance_without_its_card_has_no_date`: when the card is gone, `provenance` is
  `"Accepted · from your answer to the series"`.
- `a_declined_or_human_answered_date_is_neither_listed_nor_drawn`. Four cases:
  - a declined series' instances (`declined` lines, no carry line);
  - a carry line followed by a `declined` line;
  - a carry line followed by a human `drop` answer (actor from `journal::read_human_actor`);
  - (P17) a one-day all-day date on an executed `event-accept` card's `instances:`, with a
    confident `obligation` verdict and in the roster's relevant section, followed by a `declined`
    line.

  None is in `coming_up`, and none of the lane-shaped ones is in `all_day` or among
  `accepted_events`' lane instances.
- `a_confidently_judged_carried_date_keeps_its_listing_and_gains_the_label`:
  - a machine `opportunity`, then a carry line: listed, with the provenance;
  - a machine `drop`, then a carry line, lane-shaped: in `all_day`, not in `coming_up` (§4, PQ3's
    trade-offs).
- `a_dropped_lane_date_is_drawn_with_its_uid_until_removed` (P17; replaces
  `a_dropped_lane_date_stays_drawn_and_unlisted`, whose trade-off Quinn did not accept at
  Checkpoint B). The vault holds:
  - a carry line for a two-day all-day date and one for a timed date;
  - an executed `event-accept` card whose `instances:` lists a one-day all-day date.

  The roster is written by `write_roster` as a successful fetch leaves it, holding another event
  but none of the three.
  - Before the removal, `the_day` draws the two all-day dates on each day they cover, as before the
    fetch, each named by its uid in `all_day_uids`, so the console can offer "Remove from my day".
    `coming_up` lists none of the three.
  - A `declined` line for the carried all-day uid takes it out of `all_day` and `all_day_uids` on
    both its days. A `declined` line for the card-listed uid takes it out of both on its day. A
    vault-wide check confirms each removed uid is gone from `accepted_events`' lane instances and
    accepted set.
  - The timed date is untouched by either line.
- `a_moved_lane_date_is_drawn_on_its_new_day_only` (PQ5 (b2)). The vault holds:
  - two carry lines from one card for a carried date: Fri–Sat, then Sat–Sun;
  - an executed `event-accept` card whose `instances:` lists a one-day all-day date on Fri, and one
    carry line from that card giving the date Sat.

  Then:
  - `the_day` draws the carried date on Sat and Sun, not on Fri;
  - it draws the card-listed date on Sat only, once, with its uid in `all_day_uids`, not on Fri;
  - `coming_up` gives the card-listed date `accepted: true` and no `provenance`, because its own
    card lists it;
  - a second carry line from another card does not move either date (T2b.4's first-line rule).
- `the_lane_names_each_accepted_date_by_uid` (P17). One day carries a calendar all-day event, a
  carried lane date and a card-listed lane date.
  - `all_day_uids` has `all_day`'s length.
  - It holds `None` at the calendar entry's index, and each accepted date's uid at that date's
    index, in `all_day`'s order.
  - On a day with only calendar all-day events, `all_day_uids` is empty.
  - On a day with no all-day entry at all, `all_day_uids` is empty.
- `provenance_is_absent_from_the_json_when_none`. The serialised `ComingUp` of an event with no
  carry line has no `provenance` key (nor `accepted`, P8). The serialised `TheDay` of a day with no
  accepted lane date has no `all_day_uids` key (P17). So `surface_oracle.rs`' references stay
  byte-identical.
- `surface_writes_nothing_and_reads_the_same_twice`: every file's bytes in the vault are unchanged
  after `load`, `the_day` and `coming_up`. Two calls give equal results, with `all_day` and
  `all_day_uids` in `(first day, title, uid)` order.

**Behaviour.** P8's third source, P15's *Used by*, and P17's read side.
- **The reader.** `accepted_events(vault, &ledger)` is T4b's reader, now given the ledger, and
  `pub(crate)` together with its result type. T4d's `eventcarry::remove_lane_date` checks a uid
  against its lane instances, so the lane and the removal read one definition. Its doc says so. It
  adds:
  - every uid whose entry has `carry` and no `declined` line, to the accepted set;
  - each such entry whose span `eventaccept::shape` classifies as all-day, multi-day or
    zero-length, to the lane instances, whatever its verdict. The uid and title come from the
    entry, and the first and last day from the shape. Lane instances are de-duplicated by uid
    against the cards' instances;
  - from its own `archive/` scan, the `executed_at` date of every executed card whose `id:` passes
    `ids::is_id`, keyed by id.

  It also applies one rule to T4b's card-listed instances (P17): a uid whose ledger entry is
  `declined` is neither accepted nor a lane instance, whichever source named it. A carried date
  already follows that rule (above), so after this a `declined` line removes a date from the lane
  whatever its source.

  And one span rule (PQ5 (b2)): a lane instance is drawn with its entry's `carry` span whenever
  the entry has one, which `load_ledger` reads from the last same-card carry line (T2b.6).
  Otherwise it is drawn with its card's `instances:` entry. So when a uid is both card-listed and
  carried, the carry line's span wins, and the uid is drawn once.
- **The lane's uids (P17).** `TheDay` gains `all_day_uids: Vec<Option<String>>`, with
  `#[serde(skip_serializing_if = "Vec::is_empty")]`.
  - It is empty when no accepted lane date covers the day.
  - Otherwise it has `all_day`'s length: `None` for each calendar entry, and the uid for each
    accepted lane date `the_day` appended.
  - It is a parallel array, not a change to `all_day`. Spec test 22a asserts accepted dates *in*
    `all_day`, which stays a `Vec<String>`, and the references' `all_day` stays byte-identical.
  - `the_day` is the only constructor of `TheDay` in the engine; the implementer re-checks
    `app/src` with a search.
- **The ledger read.** `load_with` loads the ledger once (`eventledger::load_ledger(vault, None)`)
  and passes it in. `coming_up` passes the one it already loads.
- **The label.** `ComingUp` gains `provenance: Option<String>`, with
  `#[serde(skip_serializing_if = "Option::is_none")]`. For a listed uid whose entry has `carry`, it
  is "Accepted · from your answer to the series on <Ddd> <M/D>", the same day format as `when`. It
  drops " on …" when the card or its `executed_at` cannot be read. `accepted` is true for that uid
  too. A uid listed on its `from:` card's own `instances:` gets no `provenance` (PQ5 (b2)): it was
  answered on its card, not through its series, and a move gave it the carry line.
- **What reads what.** Coming up still lists only what `relevant_events` keeps (P8), which already
  drops a `declined` uid. The lane reads the ledger; the listing reads the roster. This task writes
  nothing, and `render` and `today.md` are untouched.
- **The read-model references.** No fixture holds a carry line or an event card (checked: no
  `instances:`, `kind: event-check`, `kind: event-accept` or `agent:knowlu.carry` under
  `engine/tests/fixtures/`). So `surface_oracle.rs`' three references stay byte-identical. If a
  fixture ever held one, console spec §4.6's rule would apply: regenerate only in a commit whose
  diff shows the change and whose message says why.

**Done when:** the eleven tests pass, and T4b's and T2b.6's tests, `surface_oracle.rs` and every
`surface.rs` test pass unchanged.

### T4d. P17: the student removes an accepted lane date (`eventcarry.rs`)

**Agent:** `contract-engineer` (Opus, xhigh).

**Why:** it writes into the record of the student's answers on the student's click, through a
human gate of its own outside `write.rs`. A wrong gate lets an agent remove a date the student
accepted. A wrong check declines an event the student never saw in their lane. And this plan gives
`eventcarry.rs` to contract-engineer (T2b).

**Files:**
- `engine/src/eventcarry.rs`: the function, its unit tests, and one sentence in the module doc;
- one rank-level test appended to `engine/src/cli.rs`' test module, with no code change there (as
  T4's test 24 in `enrich.rs`).

**Answer applied:** P17 (Quinn, Checkpoint B, 2026-10-01). **PQ5 (b2) and PQ6 (b), answered
2026-10-01 (§13):** neither changes the line shape, so the tests below stand as written. Under
(b2), `accepted_events` keeps a moved date among its lane instances with its latest span, so the
check in step 4 still accepts the uid, and the console offers the button on the new day.

**Tests first.** They go in `eventcarry.rs`' test module unless named otherwise. The student's
context is `WriteContext::new(journal::read_human_actor(&vault)?, "dashboard")`, never a literal.
- `removing_a_carried_lane_date_appends_one_declined_line`. The vault holds an executed
  `event-accept` card with an `id:`, and a carry line for a later two-day all-day `lx:77:3`.
  - `remove_lane_date(vault, "lx:77:3", today, &student)` returns `Ok(true)`.
  - The ledger's bytes are the old bytes, then exactly `- lx:77:3 · declined <today>` and
    `NEWLINE`.
  - `load_ledger` gives `lx:77:3` `declined: true`, with its verdict, `answered_by` and `carry` as
    before.
  - Every other file in the vault is byte-identical, and no journal file is created or changed.
- `removing_a_card_listed_lane_date_appends_one_declined_line`: the same, for a one-day all-day
  instance on an executed `event-accept` card's `instances:`. A second vault does it for one on an
  executed `event-check` card's `instances:` (D11).
- `a_second_removal_writes_nothing`: a second call returns `Ok(false)` and leaves the ledger's bytes
  unchanged.
- `only_the_student_removes_a_date`. Four calls are each refused, with an error naming why and no
  file changed in any vault:
  - an `agent:` actor;
  - a `system:` actor;
  - a human actor that is not the vault's own token, built as `write.rs`' own gate tests build
    one, with no literal;
  - any actor, in a vault whose `config/actor.yaml` is invalid (the reader's named line).
- `only_an_accepted_lane_date_is_removed`. Each of these is refused by name, with no file changed:
  - a carried timed date;
  - a uid with only a machine verdict;
  - a uid no source names;
  - a lane date on a `rejected` card, and one on an `expired` card;
  - a uid the ledger cannot read back: `"a b"`, and one holding `verdict:`.
- `rank_never_brings_a_removed_date_back`, in `cli.rs`' test module, through full `rank` runs as
  T4's rank-level tests are. The vault holds an accepted `event-accept` series whose later two-day
  all-day instance is in the feed.
  - One `rank` writes the instance's carry line, and `remove_lane_date` then removes it.
  - Two more `rank`s run with the feed unchanged, and a third with every feed failing. The vault
    holds nothing else for `rank` to write, so none of the three changes the ledger's bytes from
    what they were after the removals: no second carry line, no answer line and no second
    `declined` line for either uid.
  - No card names the uid. `the_day` for each of its days does not draw it, and `coming_up` does
    not list it.
  - The same holds for a card-listed lane date of a second series in the same vault.
- `removal_is_deterministic_and_leaves_existing_vaults_alone`.
  - The same removal over two copies of one vault gives byte-identical ledgers (P14: the line holds
    only the uid and the day).
  - Take a copy of `vault-full`, made first and never used in place. It has no accepted lane date,
    so a call with any uid from its ledger is refused, and no byte of the copy changes.

**Behaviour.** P17's *The entry point*.
- `pub fn remove_lane_date(vault: &Path, uid: &str, today: Date, ctx: &WriteContext) ->
  Result<bool, String>` runs in P17's order:
  1. the gate;
  2. `ledger_reads`;
  3. already `declined`: `Ok(false)`;
  4. the uid must be among the lane instances of
     `crate::surface::accepted_events(vault, &load_ledger(vault, None))`;
  5. `record_declined(vault, uid, today)`, then `Ok(true)`.

  Each refusal is one line naming the uid and the reason. The console shows it as the command's
  `error`.
- The gate is stricter than `write`'s private `human_gate`: it also refuses an `agent:` or
  `system:` actor, because only the student removes a date. Its doc names `write::human_gate` as
  the rule it restates, so a change to one is seen beside the other.
- It writes nothing else: no note, no journal record (P17), no other ledger line. `rank` never
  calls it.
- The module doc gains one sentence: the student's removal of an accepted lane date is the one
  write in this module made on the student's click.

**Done when:** the seven tests pass, and every `eventcarry.rs`, `eventledger.rs`, `surface.rs` and
`cli.rs` test passes unchanged.

### B2. Review of T4, T4b, T2b.6, T4c and T4d (before the first push)

**Agents:** `contract-reviewer` (Opus, xhigh) over T4's, T2b.6's and T4d's commits, and `reviewer`
(Opus, high) over T4b's and T4c's.

**Why:** spec §13 puts T4's `cli.rs` change (the order of the vault-writing passes and each pass's
budget) under contract-reviewer, though `cli.rs` is not on the contract list. T4d writes into the
event ledger on the student's click, through a gate of its own (P17). T4b and T4c are
Sonnet-written and must be reviewed before a push.

**Report:** `docs/reports/<date>-events-review-2.md`, one section per agent.
- *Contract-reviewer* checks:
  - the pass order against T4's list: the carry first, then `inherit_series_answers` (P16, as
    Quinn ruled at Checkpoint B), both before the roster and every card;
  - each emitter's budget, and the switch-off path that keeps the golden;
  - the feed-failure no-op, ledger included;
  - that tests 19 and 20a at `rank` level assert what spec §11 states;
  - that `rank_gives_each_carried_date_the_carrys_line_and_lists_it` asserts one line per date;
  - reading T2b.6's commit alone (PQ5 (b2)):
    - its first check is recorded in the SDD ledger, and no pinned assertion or test vault was
      edited to pass;
    - a second line is written only for a moved lane-to-lane span, from the uid's own card, once
      per move, and never over a `declined` line, a human answer or on a roster-built run;
    - `load_ledger` takes a later line's span only from the same card, and changes nothing else in
      the entry; T2b.4's and T2b.5's tests pass unchanged;
    - the line is the carry's existing shape, and no other file or journal record is written;
  - reading T4d's commit alone (P17):
    - the only write is one existing-shape `declined` line through `record_declined`;
      `eventledger.rs`, `journal.rs` and `write.rs` are unchanged;
    - the gate refuses every actor but the vault's own human token, an agent's included;
    - a uid is removed only when `accepted_events` draws it in the lane, and a second call writes
      nothing;
    - `rank_never_brings_a_removed_date_back` asserts the ledger's bytes, not only the read model,
      over a vault that holds nothing else for `rank` to write.
- *Reviewer* checks:
  - T4b against P8 and the `accepted_events` reader: no public signature changed, `accepted`
    absent when false, no write;
  - T4c against P15's *Used by* and P17's read side:
    - the lane is drawn from the ledger alone;
    - `provenance` is absent when `None`, and `all_day_uids` is absent when empty;
    - `all_day_uids` stays parallel to `all_day`;
    - a `declined` uid leaves the lane whatever its source;
    - no write, and the read-model references untouched;
    - `a_dropped_lane_date_is_drawn_with_its_uid_until_removed` pins what Quinn ruled at
      Checkpoint B;
    - (PQ5 (b2)) a lane instance is drawn with its `carry` span over its card's, and a card-listed
      moved date gets no `provenance`, as `a_moved_lane_date_is_drawn_on_its_new_day_only` pins.

Fixes go to `contract-engineer` (T4, T2b.6, T4d) and `implementer` (T4b, T4c), serially.
**Done when:** no open Critical or Important finding. Then the controller asks Quinn's go to push
`events` (P12; §1 records the one earlier push, at Checkpoint B). With this report the controller
brings Quinn, for information, P17's two remaining trade-offs and (b2)'s two rules and two
uncovered cases (§13).

*B2's first pass (2026-10-01) was partial.* It read T4 (`68bf88a`), T4b (`1c309cc`) and T2b.6
(`7550b0d`) only. T4c and T4d are not built: PQ7 gates T4c (§13), and T4d builds on T4c. At
`1c309cc`, `surface.rs`'s `accepted_events(vault)` reads no ledger and drops no declined uid, and
there is no `eventcarry::remove_lane_date`, so P17 and PQ5's read side are unbuilt. Example: after
the first rank in `rank_gives_each_carried_date_the_carrys_line_and_lists_it`, `localist:77:3`
(all-day, Oct 20–21) has its carry line but is in neither day's `the_day(..).all_day`. And
`lx:88:1`, which T2b.6 moves from Fri to Sat, is still drawn on Fri. That pass cannot meet this
Done-when. It closes nothing, and the controller does not ask the push go on it. Its two Important
findings are this scope and PQ7. Once Quinn answers PQ7 and any T2b.6 fix has landed, T4c is built,
then T4d, and B2 is dispatched again over those two commits and that fix. Only that pass can close
B2.

### M. Take `main` in after M2 merges (main session)

**Why the main session:** it is a merge that keeps the reviewed commits' hashes (P12). Gmail is
already in the branch (`2d5298a`). When `m2-editing` has merged to `main`, re-run T0's overlap
check, `email-forwarding` included, and merge `main` into `events`. Resolve any textual conflict in
`surface.rs`, `enrich.rs`'s test module or `lib.rs`, keeping both sides' lines, and run the gate. If
any email-forwarding branch has merged to `main` first, M also takes in its H5 handler names, its
`console.js` and `static_assets.rs` changes and its `approvals.rs` and `cli.rs` changes. Each
conflict goes where §2's email row says. H2 then lands on a list that already holds email's names.
H2 is applied on M's commit, and T5, T6 and T7 start from H2's. M and later commits are not pushed
until W closes.

*M was taken in early (2026-10-01, `b04641d`), before B2 closed.* §5 makes M wait for B2, and B2's
first pass closed nothing (it needs T4c and T4d). Treat `b04641d` as taken in early and keep it
unpushed, as every M commit is until W closes. It does not make H2 ready: H2 also waits for T4d (see
below). If T4c or T4d change `surface.rs` or `engine/src/lib.rs` after the merge, re-run M's gate
(the workspace suite, then the engine build after the app build) on the commit that holds them.

### H2. The `remove_lane_date` command (main session, after M and T4d)

**Why the main session:** `app/src/commands.rs` and `app/src/main.rs` are single-owner files
(HANDOFF §2). The controller applies a lane's hand-off with the exact code its plan gives (§7).
**Answer applied:** P17 (Quinn, Checkpoint B, 2026-10-01). **After** M, so it lands on M2's
`commands.rs` and its console list, **and after T4d**, because §7's code calls
`eventcarry::remove_lane_date`, which T4d builds (T4d waits for T4c, which waits for PQ7). Before
T4d lands the call does not compile, and a stub would ship a "Remove from my day" button that
removes nothing. Dispatch H2 as a child of T4d's commit (M is already an ancestor, `b04641d`), never
before. T5 and T7 stay behind H2.
- Add §7's H2 code to `app/src/commands.rs`, beside `delete_note_inner` and `delete_note`.
- Add `commands::remove_lane_date` to the console window's `generate_handler!` list in
  `app/src/main.rs` (the list that names `commands::dropped_events`), right after
  `commands::delete_note`. The wizard's list does not change.
- Run the gate. `app/tests/handler_lists.rs`' existing tests pass unchanged; T7 adds the test for
  this command's place.
- One commit. `reviewer` reads it in W, as code in `app/src` that no reviewer has read yet.

### T5. Deck labels, the "Accepted" tag and "Remove from my day" (`console.js`)

**Agent:** `console-ui` (Sonnet, medium). **Why:** `app/static` is its lane.
**Files:** `app/static/console.js`, `app/tests/static_assets.rs`.
**PQ6 answered (b), 2026-10-01 (§13):** a console-only Undo toast, no confirm. The last two tests
below are §13's, and they replace the drafted confirm test. **Shared with
email-forwarding** (its T15.1–T15.5): if email has merged first, T5 starts from M's merged
`console.js`. If email merges after events, it takes T5's lines in (§2).

**Tests first** (spec test 26, plus P8's renderer):
- `event_cards_read_accept_and_decline_and_send_the_same_verdicts` (26): on `event-accept` and
  `event-check` cards, at both sites (the deck's card and the Decisions view's row), the primary
  button reads **Accept** and the secondary **Decline**, and they still send `approved` and
  `rejected`; other kinds still read Approve and Reject, and `commitment-ask` keeps Answer….
- `coming_up_shows_accepted_events` (plan addition, P8): an entry with `accepted: true` renders an
  "Accepted" tag as escaped text; one without renders none.
- `coming_up_shows_the_carrys_provenance` (PQ3, T4c): an entry with a `provenance` string renders
  that string as escaped text, in place of the plain "Accepted" tag. An entry without one is
  unchanged.
- `the_lane_offers_remove_on_accepted_dates_only` (P17, T4c).
  - `renderTheDay` reads `the_day.all_day_uids`.
  - An `all_day` entry whose uid is set renders a "Remove from my day" button. The button carries
    the uid and the title as escaped `data-` attributes.
  - An entry whose uid is `null` renders as today, `(all day) <title>`, with no button. So does
    every entry of a state with no `all_day_uids` key.
- `remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes` (P17, H2, PQ6 (b)).
  - The click hides the entry at once and shows a "Removed · Undo" toast for 10 seconds, after the
    pattern of M2's `offerBodyUndo`. There is no `window.confirm`.
  - While the toast shows, the entry stays hidden across a `state` poll's re-render: the console
    keeps the pending uid and `renderTheDay` skips it.
  - When the toast closes without Undo, the console invokes `remove_lane_date` once, with
    `{ view: stateView(), uid: <uid> }`, and hands the envelope to `applyEnvelope`, which shows a
    refusal's `error` and the state it carries.
- `undo_within_the_toast_never_calls_remove_lane_date` (PQ6 (b)). Undo inside the 10 seconds
  clears the pending uid, the entry shows again on the next render, and no `remove_lane_date`
  invoke is made, then or when the toast's time runs out.

**Behaviour.** Spec §6.3, and P17 for the lane's button.
- The per-kind label switch follows the existing `commitment-ask` switch.
- The tag is plain text beside the event's time, styled with an existing class. The provenance is
  shown exactly as `surface` gives it: the console composes no label text of its own.
- The button uses an existing class. Its click goes through the existing delegated handler, as
  `data-del` does.
- One new invoke, `remove_lane_date` (H2). It sends no `ui_event` of its own (D12: no new
  telemetry row).
- The Undo toast follows M2's `offerBodyUndo` and uses an existing class; `console-ui` reads that
  function first and reuses what it can. A window closed inside the 10 seconds sends nothing, and the date shows again (§13).

**Done when:** the six tests pass, and every existing static test passes unchanged.

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

### T7. `decide` and `remove_lane_date` end to end (`app/tests/`)

**Agent:** `test-writer` (Sonnet, medium). **Why:** tests only, over a finished engine and H2's
command. **Files:** `app/tests/commands.rs`, `app/tests/handler_lists.rs`. **After:** H2.
**Tests:**
- (spec test 28) `decide_accepting_an_event_card_books_it_before_returning`. A scaffolded vault
  with a hand-built `event-accept` card (spec §5.1's shape); `decide_inner(…, "approved", …)`
  returns the card in `decision.executed`, and the `commitments/` note exists before the call
  returns. The card's `status` record has an actor `provenance::is_agent` rejects; the commitment's
  `create` record has one it accepts.
- (P17) `remove_lane_date_takes_the_date_off_the_day_before_returning`, in `commands.rs`. The
  vault is scaffolded, and the console's day is pinned through `ConsoleState::test_today`, as the
  file's other tests pin it. It holds an archived executed `event-accept` card with an `id:`, and a
  carry line, written through `eventledger::record_carried_answer`, for a one-day all-day date on
  the pinned day.
  - Before the call, `state.the_day` holds the title in `all_day` and its uid in `all_day_uids`.
  - `remove_lane_date_inner(&cs, view, uid)` returns `ok: true`. Its `state.the_day.all_day` no
    longer holds the title, and it has no `all_day_uids` key.
  - The ledger holds exactly one `declined` line for the uid, after the carry line.
  - A second call returns `ok: true` and changes no byte of the ledger.
  - A uid that is not an accepted lane date returns `ok: false`, with the engine's error and the
    current state, and writes nothing.
- (P17) `the_console_list_names_remove_lane_date_and_the_wizard_list_does_not`, in
  `handler_lists.rs`, through the file's own parser.

No test names a human token. Under PQ6 (b), `remove_lane_date_inner` is the call the console sends
only once the Undo toast has closed (T5). These tests drive that call directly and have no toast,
so their assertions are unchanged. The toast's timing is T5's two tests and T10's step 5.
**Done when:** the three tests pass, and both files' existing tests pass unchanged. The tests may
pass on their first run, because they pin T3.1a, T4d and H2 through the app.

### T8. Reference documents

**Agent:** `docs-keeper` (Sonnet, medium). **Why:** reference text only. **Files:**
- `docs/surface/anatomy.md`, by its own headings. (Corrected 2026-09-30: an earlier draft put the
  delta under §3.11, which is Coming up; the delta line is §3.2.)
  - §3.2, the delta line: carried notes are listed as the carrying run's `create` records, with the
    `seen_at` gap and the 200-record cap named (spec §4.5).
  - §3.7, the day: accepted all-day, multi-day and zero-length (PQ2) events are in the all-day
    lane. Two sources:
    - executed cards' `instances:` (T4b);
    - carried dates of either series kind, read from the span on the carry's ledger line (P15,
      T4c). These are drawn on each day they cover, whatever the roster holds, through a run
      where every feed fails.

    Capacity is not touched. A lane-shaped instance that had already started when its series was
    first carried is not drawn (§4, PQ3's trade-offs). An accepted lane date from either source
    stays drawn on its day or days after a later fetch drops it (cancelled, or gone from the feed),
    while Coming up no longer lists it.

    Name the removal path Quinn ruled at Checkpoint B (P17):
    - Every accepted lane date carries its uid in `the_day.all_day_uids`, a parallel array that is
      absent when no such date covers the day. The console offers "Remove from my day" on it.
    - The button hides the date at once behind a 10-second "Removed · Undo" toast (PQ6 (b)). When
      the toast closes without Undo, the console calls `remove_lane_date`. That writes one
      existing-shape `declined` line through `eventcarry::remove_lane_date`, which only the
      vault's own human token may call. Undo sends nothing, and a window closed inside the 10
      seconds sends nothing.
    - A moved lane date fixes itself (PQ5 (b2)): when a successful fetch gives an accepted lane
      date a new lane-shaped span, the carry appends one more carry line from the same card, and
      the lane draws the new span. A card-listed date gets its first carry line this way. Two
      cases stay on the old day until the student removes them: a lane date moved to clock hours,
      and an `event-check` card's own uid, which holds the student's answer line.
    - The date then leaves the lane and Coming up for good, from either source. `rank` never
      brings it back: the carry, never-ask-twice and `relevant_events` all read `declined`.
    - Name P17's two remaining trade-offs:
      - the line is device-local and carries no `by`, so a restored vault draws a card-listed or
        still-fetched date again;
      - the console draws today's lane only, so a date is removable on the day or days it shows.
  - §3.9, the deck: event cards, Accept and Decline on both event kinds, and expiry. Also, beside
    the existing note on `event-check`'s human answer line: the carry writes one answer line per
    carried date, by `agent:knowlu.carry`, with `from:`, `start:` and `end:`. A later human answer
    replaces it, and it never replaces one (P15).
  - §3.11, Coming up: the "Accepted" mark (P8), and the carry's provenance label ("Accepted · from
    your answer to the series on <date>", the date from the answering card's `executed_at`).
    - A carried date of either series kind is listed through its carry line, like any judged
      event.
    - An instance a model judged confidently keeps its verdict and gains the label, so a confident
      `drop` stays unlisted.
    - `today.md` lists carried dates without the label.
    - A date the student removed (P17) is not listed. M2's *Not shown* lists it as "you declined
      it" while the roster still holds it.
- `docs/reference/engine-commands.md`: one paragraph in `rank`'s entry. It covers the events
  pass's order (the carry before series inheritance, P16), `event_cards`, the carry over accepted
  `event-accept` and `event-check` series, and its one ledger line per carried date (P15);
- `docs/notes/2026-09-29-vision-program.md`: the P4 row points at the spec and this plan; P6 (c)
  marked delivered by D3;
- `docs/specs/2026-09-29-events-design.md` §12: T0c's count in the Q1 note, dated; and one dated
  note recording the plan-stage answers of 2026-09-30. PQ1 (a) widens §4.5 and §6.1's
  `eventcarry.rs` row: the accept carry also reads executed `event-check` cards with `instances:`
  through `accepted_check_series`. PQ2 puts a zero-length event in the all-day lane (§4.2). PQ3's
  final answer, (b-prime) with the span, is recorded with the two answers it replaced, (c) and
  (b-prime) without the span. The note also records what that answer widens:
  - §5.4: the carry's answer line, its `by:agent:knowlu.carry`, `from:`, `start:` and `end:`
    fields, `opportunity` on it, and `load_ledger`'s read rules (P15);
  - §6.1's `surface.rs` row: carried lane dates and the provenance label.

  A second dated note records Quinn's two answers at Checkpoint B (2026-10-01):
  - P16 accepted. §6.1's `cli.rs` row runs the carry before series inheritance, reversing the
    signed order.
  - The removal path (P17). It widens four places:
    - §6.3, by one Tauri command, `remove_lane_date`, in the console's list, computing nothing;
    - §6.1's `eventcarry.rs` row, by `remove_lane_date`, the student's removal of an accepted lane
      date: gated to the vault's own human token, one `declined` line;
    - §6.1's `surface.rs` row, by `the_day`'s `all_day_uids`, and by the rule that a `declined`
      uid leaves the lane whatever its source;
    - §4.3, which gains the removal as a second writer of `declined`.

    §5.4's "no new line shape" still holds: Quinn answered PQ5 (b2) and PQ6 (b) on 2026-10-01.
    The note records both answers, (b2)'s second carry line of the existing shape for a moved
    lane date (with its same-card and lane-to-lane rules and its two uncovered cases), and P17's
    two remaining trade-offs.

  The signed text above both notes is not rewritten;
- `docs/reference/app.md`: `remove_lane_date` in the console's command list, beside `delete_note`
  (what it takes, what it writes, that the engine gates it). Also the campus presets' keys, if the
  file lists them.

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
- `app/src/` changed only by H2 (P17):
  - one command, `remove_lane_date`, which computes nothing, calls `eventcarry::remove_lane_date`
    with `console_ctx(vault)` and runs inside `mutate`;
  - one name added to the console's `generate_handler!` list, and the wizard's list unchanged
    (`handler_lists.rs` green, T7's test included);
- `site/` and `PRIVACY_VERSION` unchanged; no new telemetry row;
- the fixtures diff empty and both oracles green;
- `surface::delta` unchanged (20c green);
- one `answered_series`, one shape classifier, and `accepted_check_series` read only by the accept
  carry (PQ1 (a));
- PQ3, (b-prime) with the span:
  - exactly one carry line per carried date, and no other new write (no journal record shape, no
    new file);
  - `record_answer` and every existing ledger line byte-identical;
  - a human answer never overridden by a carry line;
  - the lane drawn from the ledger alone;
  - P16's order, as Quinn accepted at Checkpoint B.
- P17, the removal path Quinn ruled at Checkpoint B:
  - a dropped lane date is drawn with its uid until the student removes it;
  - the removal writes one existing-shape `declined` line and nothing else, only for the vault's
    own human token, and only for a date the lane draws;
  - no `rank` brings a removed date back;
- PQ5 (b2) and PQ6 (b), Quinn's answers of 2026-10-01 (§13):
  - T2b.6's first check is recorded, and B2's reading of its commit is closed;
  - a moved lane date gets one carry line per move, from its own card, lane to lane only, never
    over a `declined` line or a human answer, and the lane draws the latest span;
  - "Remove from my day" hides the date at once and calls `remove_lane_date` only when the Undo
    toast closes; Undo never calls it; no `window.confirm` remains;
- the overlap with email-forwarding (§2): if it merged first, both lanes' handler names, both
  lanes' `console.js` lines and their static tests are all present after M.

Fixes go to the task's own agent, and the reviewer re-reads them. The controller brings Quinn the
report before T10 (spec §13's third checkpoint). **Done when:** no open Critical or Important
finding.

*W's first pass (2026-10-01) ran before T8, so it is not W.* Its three findings, verified:
1. *Scope:* T4c, T4d, H2, T5, T7 and T8 are not built, and PQ7 is open. This is B2's finding,
   still blocked on Quinn's PQ7 answer (§13). W runs again over the whole branch after T8.
2. *Two desktops:* recorded as §8's risk and pinned by `a883710`. Its recommended fix, a
   deterministic id per uid, is not taken: spec §9 hands it to the two-desktop stream, and W's own
   determinism check asks for no deterministic id on this branch. It goes to Quinn at W.
3. *The routing gate on `approvals.rs`:* its premise does not hold. Checkpoint B's
   `contract-reviewer` read T3.1a, T3.1b and T3.2 one commit at a time (0/0/5; `run.md` and the
   SDD ledger at `10071f6`), and `approvals.rs` is unchanged since `10071f6`. No report file was
   committed, though, so W's `contract-reviewer` pass reads `approvals.rs` again rather than closing
   a report that is not on disk.

### T10. Gate, live proof, PR and merge (main session, with Quinn)

**Why the main session:** it needs a staging session by OTP and a desktop; neither is delegated.
**Waits on:** W, T9's verdict, and staging matching the repo (START-HERE §3: Quinn's `db push`, then
the J functions redeployed), since the proof's verdicts come from staging's `judge-event`.
- **Gate:** the workspace build and tests with 0 other warnings; `scripts/ci/eol-check.ps1`; the
  fixtures diff empty; the four ignores in place; `human_actor_literal.rs` green; the handler lists
  as H2 left them (the console's grown by `remove_lane_date`, the wizard's unchanged), by running
  `app/tests/handler_lists.rs` (in the branch since `2d5298a`, with T7's test) as part of the
  workspace tests.
- **Live proof** (spec §11.4), under the standing solo-proof approval (dev build, DOM driver, OTP,
  never OS input), on a scratch profile against staging, Alabama preset:
  1. one real slot against the Alabama feeds;
  2. accept one obligation card and one opportunity card; decline one;
  3. check the schedule (the commitment's block, or the all-day lane) and Coming up ("Accepted",
     the declined event gone). Where the live feed has a series with a later instance, check the
     carried date: its block on its day if it is timed, its lane entry if it is all-day, and its
     Coming up entry with the provenance label, for either series kind (PQ1 (a), PQ3). Check its
     one `agent:knowlu.carry` line in `state/events-seen.md`. If the feed has no such series, the
     proof records that, and T4's and T4c's tests stand for it;
  4. run one more `rank` and confirm nothing is re-asked and no second carry line is written;
  5. (P17, PQ6 (b)) where the day shows an accepted all-day date, click "Remove from my day" (DOM
     only):
     - the date leaves the lane at once and the "Removed · Undo" toast shows;
     - first click Undo: the date returns and `state/events-seen.md` is unchanged after the toast's
       time has passed;
     - then remove it again and wait for the toast to close: the date stays gone from the lane and
       Coming up, and `state/events-seen.md` gains one `declined` line for its uid;
     - one more `rank` brings nothing back.

     If no accepted all-day date falls on the proof's day, the proof records that, and T4d's and
     T7's tests stand for it.

  Then remove the scratch profile (profile, vault, credentials, autostart).
- **Before the PR: the pre-merge overlap check.** Re-run T0's check against every sibling,
  `email-forwarding` included. If an email branch merged to `main` after M, take `main` in again
  under M's rules before the PR. Conflicts go where §2's email row says, the gate and this task's
  handler-list check are re-run, and `reviewer` reads the resolution.
- **Then:** the PR, CI green, and Quinn's word to merge. At the MVP's parity audit, Quinn marks P4.

## 6. Order, parallelism and checkpoints

- **Order:** T0 → H1a → T1a → T1b → H1b → T2b.1 → T2a.1a → T2a.1b → T2a.2 → T2b.2 → T2b.2b →
  T2b.3 → T2b.4 → T2b.5 → T3.1a → T3.1b → T3.2 → (Checkpoint B ∥ R1) → T4 → T4b → T2b.6 → T4c →
  T4d → B2 → push (Quinn's go; §1 records the one push at Checkpoint B) → (M2 merges; Gmail is already in at `2d5298a`) → M → H2 → T5 → T6
  → T7 → T8 → W → push → T10.
- **Off the line:** T0c any time before T10 (read-only, no worktree). T9 any time before T10, in the
  `j-events` worktree, so it may run beside the engine tasks.
- **One implementer at a time** in the `events` worktree. Reviews run beside the next task only when
  that task does not build on the reviewed code: T4 waits for Checkpoint B and R1; M waits for B2.
  (M was taken in early at `b04641d`, unpushed; H2 waits for T4d regardless, as a child of its commit.)
- **Pushes (P12):** nothing is pushed before B2 closes; every Sonnet-written commit has then had a
  `reviewer` pass (R1 or B2). Commits after B2 are pushed only after W closes. Each push is a code
  push and needs Quinn's go. One push already departed from this rule: the controller's push at
  Checkpoint B, for the diff link, recorded in §1. The pushed commits are never rewritten.
- **Waits on Quinn:** PQ1–PQ4 were answered on 2026-09-30, and P16 and the removal path at
  Checkpoint B on 2026-10-01. Two questions are open (§13), asked one at a time:
  - **PQ5** (a moved event), before T4c is dispatched. It gates T4c and T4d.
  - **PQ6** (undo or confirm), after PQ5, and at the latest with B2's report. It gates T5. If
    Quinn answers (c) before T4d runs, T4d builds the un-remove with the removal.

  *Both answered 2026-10-01:* PQ5 (b2), built by the new T2b.6 before T4c; PQ6 (b), built in T5.
  Neither gates a task any longer, unless T2b.6's first check blocks (b2) and PQ5 returns.

  *Open, raised by T2b.6's review on 2026-10-01:* **PQ7** (a carried lane date moved to clock
  hours, §13). It was to be asked before T4. T4 and T4b were built while it was still open, and
  B2's partial pass found that no T4 test pins the guarded clause. It is now asked before T4c is
  dispatched, and it gates T4c. Under (b) or (c), a T2b.6 fix lands first. B2's first pass was
  partial (§5, B2) and carries no push go.

  Also still waiting on Quinn: W's checkpoint, the push go after B2 and after W, and the merge
  word in T10. P17's two remaining trade-offs (device-local; today's lane only) go to Quinn with
  B2's report, for information.
- **Opus starts:** T0, H1a, H1b, T2b.1, T2b.2, T2b.2b, T2b.3, T2b.4, T2b.5, T3.1a, T3.1b, T3.2, B,
  R1, T4, T2b.6, T4d, B2, T9, W, M, H2, T10.
  Every other task starts on Sonnet, and a second failed attempt moves it to Opus at high.
- **Quinn's checkpoints (spec §13):** at signing (done, 2026-09-29); after Checkpoint B (done,
  2026-10-01: P16 accepted, a removal path asked for, §12); PQ5 and PQ6 (done, 2026-10-01: (b2)
  and (b), §13); with B2's report, P17's two remaining trade-offs and (b2)'s rules and uncovered
  cases, for information; and before T10, with W's report.
- **Sibling lanes:** the overlap check runs at T0, at M, and before the merge (T10). Each time it
  covers `m2-editing` (until merged), `p3-registrar`, `j-events` and `email-forwarding`.

## 7. Controller hand-offs (main session)

- **H1a**, before T1a: `pub mod eventaccept;` in `engine/src/lib.rs`, beside `eventemit`, and the
  module file with its doc line.
- **H1b**, before T2b.1: `pub mod eventcarry;`, likewise.
- **H2**, after M and before T5 (P17, Quinn at Checkpoint B). `app/src/lib.rs` needs nothing.
  - In `app/src/commands.rs`, beside `delete_note_inner`:

    ```rust
    /// P17 (Quinn, Checkpoint B, 2026-10-01): "Remove from my day" on an accepted all-day date.
    /// The engine gates (only this vault's own human token), checks (a date the lane draws) and
    /// writes (one `declined` line); this computes nothing. A refusal is the engine's line.
    pub fn remove_lane_date_inner(cs: &ConsoleState, view: &str, uid: &str) -> Result<Value, String> {
        mutate(cs, view, |_journal| {
            knowlu_engine::eventcarry::remove_lane_date(&cs.vault, uid, now_in(cs).date(), &console_ctx(&cs.vault)?)
                .map(|_| ())
        })
    }
    ```

  - In the same file, beside the `delete_note` command (one line, as its neighbours):

    ```rust
    #[tauri::command(async)] pub fn remove_lane_date(cs: State<'_, ConsoleState>, sch: State<'_, Scheduler>, view: String, uid: String) -> Value { let mut env = remove_lane_date_inner(&cs, &view, &uid).unwrap_or_else(|e| json!({ "ok": false, "error": e, "state": Value::Null })); let _ = attach_scheduler(&mut env, &sch); env }
    ```

  - In `app/src/main.rs`, the console window's `generate_handler!` list (the one that names
    `commands::dropped_events`): `commands::remove_lane_date,` right after
    `commands::delete_note,`. The wizard's list is unchanged.
  - `handler_lists.rs` pins names, not a count, so no number is quoted. If the gate needs one,
    the controller counts both lists by script after H2 and records them in the SDD ledger. The
    console's list holds every name that was in it at M (M2's, and email's H5 names if email
    merged first), plus `remove_lane_date`. The wizard's list is unchanged. If email merges after
    events, its controller adds its names to a list that already holds `remove_lane_date` (§2).
- **`engine/src/lib.rs` at M:** M2's `pub mod profile;` and these two lines are kept side by side.

## 8. Risks

- **The carry hard-books or declines a whole series silently.** Guarded by one `answered_series`
  with its edge tests (T2b.1), the carry's tests 20 and 20a, Opus at xhigh for T2b, and Checkpoint B
  reading each commit alone.
- **PQ1 (a) books an `event-check` series as hard that the student did not mean.** The student
  answered "this applies to me", which D11 already treats as an Accept for the listed instances.
  Guarded by `accepted_check_series`' own edge rules (the series claimed as `settled_series`
  claims it, then executed, with `instances:`, archive only), `answered_series` winning an overlap,
  the three PQ1 tests in T2b.2b (its own commit), the
  rank-level case in 20a, and Checkpoint B's check, on that commit alone, that the reader feeds
  only the accept carry.
- **A card books an hour the feed never claimed.** On a feed-failure run the emitters read
  `read_roster`'s lossy events (§2). Guarded by the source rule in T2a.1b and T2a.2, their two
  roster tests, the rank-level clause in `a_rank_on_the_roster_alone_carries_nothing`, and R1.
- **A carry line stands over a student's answer, or a student's answer is lost.** The carry
  writes answer lines that no human gave. Guarded by:
  - the `agent:` actor and the `from:` card on every such line;
  - `load_ledger`'s rules that a later human answer replaces a carry line and a carry line never
    replaces a human answer (T2b.4's tests);
  - the carry's own skip rule (T2b.5);
  - contract-engineer at xhigh on both files, and Checkpoint B reading each commit alone.
- **An existing vault's ledger reads or writes differently.** Guarded by:
  - `record_answer` left unchanged, with its tests;
  - a read rule that needs an agent `by` together with a card and a span, which no line written
    today has;
  - `the_frozen_ledger_reads_as_before` and `an_existing_vaults_ledger_bytes_are_unchanged`;
  - the fixtures diff.
- **A carried date is listed or drawn wrongly.** Three cases:
  - a date shown as accepted after the student deleted its commitment;
  - a confident `drop` that is drawn but not listed;
  - a lane date a later fetch dropped, still drawn and unlisted.

  The first two are named trade-offs (§4, PQ3). The third went to Quinn at Checkpoint B, who asked
  for a removal path (P17): the date is drawn with its uid until the student removes it. All three
  are documented in anatomy §3.7 and §3.11 (T8), and T4c's tests pin each case.
- **The removal declines the wrong date, or is made by someone other than the student (P17).**
  Guarded by:
  - `remove_lane_date`'s gate, which refuses every actor but the vault's own human token;
  - its check against the lane's own reader (`accepted_events`), so only a date the lane draws can
    be removed;
  - the console's Undo toast, which sends the call only when it closes (PQ6 (b));
  - T4d's seven tests and T7's end-to-end test;
  - contract-engineer at xhigh, and B2's contract-reviewer reading T4d's commit alone.

  The write is the existing `declined` line, so no reader changes meaning, and `rank` never brings
  the date back (`rank_never_brings_a_removed_date_back`).
- **P16 is not what Quinn wants.** *Closed:* Quinn accepted it at Checkpoint B (2026-10-01). T4
  builds it as written, and T8 records it in a dated spec note.
- **An older engine reads a carry line.** It reads it as a human answer: as a first line it
  settles the uid, and over an `unsure` it takes effect only with `obligation`. It ignores `from:`,
  `start:` and `end:`. This is harmless, and the MVP is single-engine (spec §6.1's tolerance).
- **A deleted note comes back.** Guarded by the ever-written set reading `archive/` (T2b.2), test
  20a at the carry and at `rank` level (T4), and test 12's retry case.
- **`rank` runs its passes in the wrong order.** Guarded by tests 19 and 20a at `rank` level (T4)
  and B2's contract-reviewer pass on T4.
- **Sonnet-written code is pushed unreviewed.** Guarded by P12: nothing is pushed before B2, and
  R1 and B2 cover every Sonnet commit up to then; W covers the rest before the second push. The
  one push at Checkpoint B (§1) put H1a to T2a.2's Sonnet commits on the remote before B2. R1
  covers those commits, and no PR or merge is opened before W, so nothing unreviewed reaches
  `main`.
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
  functions; H2, T5 and T7 wait for M, so H2's command lands on M2's `commands.rs` and console
  list; the overlap check runs three times.
- **Merge friction with email-forwarding.** Both MVP lanes add to the console's
  `generate_handler!` list and edit `console.js`, `static_assets.rs`, `approvals.rs` and `cli.rs`.
  Guarded by §2's email row: the second lane to merge takes the other in, and each conflict goes to
  a named resolver (the main session for the handler list). Also guarded by the overlap check at
  T0, M and T10, and by the note to email's controller so its §10 R2 table matches.
- **The removal path ships a half-answer to Quinn's "moved" case, or departs from VISION's undo
  commitment, unasked.** *Closed by Quinn's answers of 2026-10-01 (§13):* PQ5 (b2), built by T2b.6
  and T4c; PQ6 (b), built by T5. Two risks remain:
  - *The moved-span line writes on every run, or over an answer.* Guarded by T2b.6's first check,
    its once-per-move, same-card and lane-to-lane rules, its tests' unchanged-bytes clauses, and
    B2's contract-reviewer reading its commit alone.
  - *A removal is lost when the window closes inside the Undo toast.* Accepted with (b): the date
    shows again, the safe direction.
- **An older engine** warns `unknown kind:` on an approved `event-accept` card. Accepted by the spec
  (single desktop, single engine in the MVP); two-desktop's hand-off is spec §9's.
- **Two desktops on one account book a carried date twice** (W's first pass, finding 2,
  2026-10-01). `eventcarry::book` runs in each desktop's `rank` over its own fetch, and
  `create_confirmed` mints a random `cmt_` id, so two desktops in one slot each book the date before
  either syncs. The ever-written set stops a third only once a sync has passed.
  - *One file name on both:* a pull keeps each desktop's own copy (`sync.rs`, `exact_case_exists`).
    Two ids sit at one path: the two-desktop spec's I2, as the events digest already has on `main`.
  - *Two names* (another note took the name first on one desktop): both copies land on both
    desktops. `commitments::load` counts the lowest id and warns once (commitment-model §2.5), no
    `rank` books a third, and the student deletes the date once per copy.
  - *Pinned by* `eventcarry::tests::a_date_two_desktops_carry_is_counted_once` (`a883710`).
  - *Accepted by* the cloud design's stages (the Pilot is one computer each; more than one desktop
    is Launch, with the two-desktop stream) and by spec §9's signed hand-off: deterministic ids for
    D4's notes come with that stream's producer table. This branch adds no deterministic id (W's
    determinism check). *For Quinn at W:* the reviewer asked that this be brought before merge.

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
| §4.3: Decline writes one `declined` line per uid and leaves Coming up; widened by P17 for a second writer of the same line, the student's removal of an accepted lane date | T3.1b, T4d | test 13; T4d's seven tests |
| §4.4: snooze unchanged; expiry dates | T2a.1b, T2a.2, T3.1b | tests 7, 14 |
| §4.5: the carry beside `inherit_series_answers`, whose code is unchanged (P16 moves its call after the carry); `delta` not edited | T2b.2, T2b.5, T4 | tests 20, 20b, 20c; `rank_gives_each_carried_date_the_carrys_line_and_lists_it`; W checks `delta`'s diff |
| §4.5: the display cap and the `seen_at` gap are named, not fixed | T8 | anatomy §3.2 (the delta line) |
| §5.1: the card's fields and file name; `events:` keeps its shape | T2a.1b | tests 1, 5 |
| §5.2: new `event-check` cards gain `instances:` and the new closing | T2a.1b | `new_event_check_cards_carry_instances_and_the_new_closing` |
| §5.3: the ever-written set (commitments, tasks, archive; approvals left out) | T2b.2, T3.1a | `the_ever_written_set_reads_commitments_tasks_and_archive`; test 12 |
| §5.4: no new verdict word; Decline writes `declined` lines; widened by PQ3 for one additive line shape, the carry's (P15); P17 adds no shape (the removal writes the existing `declined` line) | T3.1b, T2b.2, T2b.4, T2b.5, T4d, B, B2, W | test 13; `removing_a_carried_lane_date_appends_one_declined_line` (the line's exact bytes); `VALID_VERDICTS` and `ANSWER_VERDICTS` unchanged; T2b.4's eleven tests (`the_carry_line_is_byte_exact`, `the_frozen_ledger_reads_as_before` and the read rules); `an_existing_vaults_ledger_bytes_are_unchanged`; Checkpoint B and W check `record_answer` is unchanged |
| §5.5: `event_cards` read as a boolean; both presets carry it | T1b, T6 | `event_cards_reads_true_or_1_and_anything_else_is_off`; test 27 |
| §6.1 `eventemit.rs` row | T2a.1a, T2a.1b, T2a.2 | tests 1–7; the refactor's pinning test |
| §6.1 `eventcarry.rs` row, one `answered_series`, the `judge_roster` call site; widened by P17 for `remove_lane_date` | T2b.1–T2b.3, T4d | T2b.1's tests; tests 19, 20, 20a, 21; T4d's seven tests |
| §6.1 `eventaccept.rs` row | T1a | test 8's tests |
| §6.1 `surface.rs` row: all-day lane, "Accepted"; widened by PQ3 for carried dates and the provenance label, and by P17 for `all_day_uids` and a `declined` uid leaving the lane from either source | T4b, T4c, T5 | test 22a; P8's three tests; T4c's eleven tests (PQ5 (b2)'s included); `coming_up_shows_the_carrys_provenance`; `the_lane_offers_remove_on_accepted_dates_only` |
| §6.1 `approvals.rs` row (CL) | T3.1a, T3.1b, T3.2, B | tests 9–17; Checkpoint B |
| §6.1 `commitments.rs` row | T1b | `a_kind_event_commitment_says_it_was_accepted` |
| §6.1 `events.rs` row | T1b, T2b.3 | the loader test; test 21 |
| §6.1 `cli.rs` row: the pass order and budgets; the carry before series inheritance (P16, accepted by Quinn at B, 2026-10-01) | T4, B2 | test 18; tests 19 and 20a at `rank` level; `rank_gives_each_carried_date_the_carrys_line_and_lists_it`; B2's contract-reviewer section |
| §6.1 invariants: no model in `rank`; deterministic bytes; no new JSON | T2a.2, T4 | test 4 compared as P14 says; W's determinism check |
| §6.2: no new command or flag; `engine-commands.md`'s `rank` entry | T8 | T8's paragraph; W |
| §6.3: `decide` settles in-process; Accept/Decline labels; "no new Tauri command" widened by P17 for exactly one, `remove_lane_date`, in the console's list, computing nothing | T5, H2, T7, W | tests 26, 28; `remove_lane_date_takes_the_date_off_the_day_before_returning`; `the_console_list_names_remove_lane_date_and_the_wizard_list_does_not`; W's `app/src` check |
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

**Quinn's plan-stage answers (2026-09-30), Checkpoint B answers (2026-10-01) and PQ5 and PQ6
answers (2026-10-01).** Each widens or
settles a signed section, so each has its own row.

| Answer | Task | What proves it |
|---|---|---|
| PQ1 (a): an executed `event-check` card with `instances:` carries its series; `accepted_check_series`; the accept carry over the union; `answered_series`' edge rule unchanged | T2b.1, T2b.2b, T4, B, T8 | `an_accepted_event_check_series_books_a_later_instance_once`, `an_event_check_card_without_instances_carries_nothing`, `a_rejected_or_pre_lane_check_card_blocks_its_series`; T2b.1's `an_event_check_card_is_not_counted`; 20a's `event-check` case at `rank` level; Checkpoint B; the spec §12 note |
| PQ2: a zero-length event is drawn in the all-day lane with no commitment | T1a, T3.2, T4b, T8 | `a_zero_length_event_gives_the_lane_marker`; test 16's zero-length clause; test 22a's zero-length clause; anatomy §3.7 |
| PQ2, held on a feed-failure run: no card or `instances:` entry is built from `read_roster`'s lossy events (second review) | T2a.1b, T2a.2, T4 | `a_roster_read_event_files_no_event_accept_card_and_no_instances`, `a_roster_read_opportunity_files_no_card`, the card clause of `a_rank_on_the_roster_alone_carries_nothing` |
| PQ3, final: (b-prime) with the span (replaced (c), then (b-prime) without it). One answer line per carried date, timed and all-day, by `agent:knowlu.carry`, with `from:` the answering card, the series' real verdict (`opportunity` accepted) and the feed's `start:`/`end:`. Judge-once: a later human answer wins, and the carry never overwrites one. Coming up lists the date with its provenance. The all-day lane draws carried all-day, multi-day and zero-length dates on each day they cover, independent of the roster. No other write | T2b.1, T2b.2b (the card id), T2b.4, T2b.5, T4 (P16), T4c, T5, T8, B, B2, W | T2b.4's eleven tests; T2b.5's nine; `rank_gives_each_carried_date_the_carrys_line_and_lists_it`; T4c's ten that predate PQ5; `coming_up_shows_the_carrys_provenance`; anatomy §3.7, §3.9 and §3.11; the spec §12 note; Checkpoint B's and W's checks |
| P16: the carry before series inheritance in `rank` (follows from PQ1 (a) and PQ3; reverses spec §6.1's signed order); accepted by Quinn at Checkpoint B, 2026-10-01 | T4, B, T8 | the `event-check` clauses of 20a and `rank_gives_each_carried_date_the_carrys_line_and_lists_it`; Quinn's word at Checkpoint B (§12); the spec §12 note |
| P17 (Quinn at Checkpoint B, 2026-10-01): the student can remove a stale accepted lane date. "Remove from my day" on every accepted lane date writes one existing-shape `declined` line through `eventcarry::remove_lane_date`, gated to the vault's own human token and checked against the lane's own reader; one new Tauri command; the date leaves the lane and Coming up for good, and no `rank` brings it back | T4b (the uid), T4c, T4d, H2, T5, T7, T8, B2, W, T10 | T4c's `a_dropped_lane_date_is_drawn_with_its_uid_until_removed`, `the_lane_names_each_accepted_date_by_uid` and the fourth case of `a_declined_or_human_answered_date_is_neither_listed_nor_drawn`; T4d's seven (`rank_never_brings_a_removed_date_back` among them); T5's three (the lane's button, and PQ6 (b)'s two); T7's two; anatomy §3.7 and §3.11; the spec §12 note; T10's step 5 |
| PQ4: B1 runs as it stands on `j-events` after the frozen `event-3` check; report in `docs/reports/`; `event-4` ships only through its own cloud PR | T9 | the B1 report, its precondition's result first |
| PQ5 (b2) (Quinn, 2026-10-01): the carry re-records a moved lane span. When a fetch gives an accepted lane date a new lane-shaped span, the carry appends one line of its existing shape from the same card, once per move, never over a `declined` line or a human answer; `load_ledger` takes the later span from the same card only; the lane draws it. No new shape. Not covered: a lane date moved to clock hours, and an `event-check` card's own uid | T2b.6, T4c, T8, B2, W | T2b.6's first check (SDD ledger); `the_carry_records_a_moved_lane_span_once`, `a_card_listed_date_moved_by_the_feed_gets_a_carry_line`; T4c's `a_moved_lane_date_is_drawn_on_its_new_day_only`; T2b.4's and T2b.5's tests unchanged; anatomy §3.7; the spec §12 note; B2's and W's checks |
| PQ6 (b) (Quinn, 2026-10-01): "Remove from my day" acts at once with a 10-second console-only Undo; `remove_lane_date` is called only when the toast closes; no engine change | T5, T7, T8, W, T10 | `remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes`, `undo_within_the_toast_never_calls_remove_lane_date`; T7's end-to-end test unchanged; anatomy §3.7; W's check; T10's step 5 |

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

PQ3's tests are plan-only too:
- T2b.4's eleven;
- T2b.5's nine;
- `rank_gives_each_carried_date_the_carrys_line_and_lists_it`;
- T4c's ten that predate PQ5;
- `coming_up_shows_the_carrys_provenance`.

P17's tests (Checkpoint B) are plan-only too:
- T4c's `a_dropped_lane_date_is_drawn_with_its_uid_until_removed`,
  `the_lane_names_each_accepted_date_by_uid`, and the fourth case of
  `a_declined_or_human_answered_date_is_neither_listed_nor_drawn` (all among T4c's ten);
- T4d's seven;
- T5's `the_lane_offers_remove_on_accepted_dates_only`;
- T7's `remove_lane_date_takes_the_date_off_the_day_before_returning` and
  `the_console_list_names_remove_lane_date_and_the_wizard_list_does_not`.

PQ5 (b2)'s and PQ6 (b)'s tests (2026-10-01) are plan-only too:
- T2b.6's `the_carry_records_a_moved_lane_span_once` and
  `a_card_listed_date_moved_by_the_feed_gets_a_carry_line`;
- T4c's `a_moved_lane_date_is_drawn_on_its_new_day_only` (its eleventh);
- T5's `remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes` and
  `undo_within_the_toast_never_calls_remove_lane_date`.

`remove_from_my_day_confirms_then_invokes_remove_lane_date` was replaced under PQ6 (b) before it
was written.

`a_dropped_lane_date_stays_drawn_and_unlisted` was removed at Checkpoint B (§12). Quinn did not
accept what it pinned, and it had not been written.

`the_accept_carry_writes_no_ledger_line` was removed with (c) (§11, item 3), because PQ3's line
inverts it. Each plan-only test pins a spec requirement that had no test of its own, a plan
decision (§3) or one of Quinn's answers; none changes a spec test's assertion. Spec test 4's
"identical bytes" is read as P14 states; that is the one interpretation of a spec assertion this
plan makes.

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
   choose. *Resolved 2026-09-30 by Quinn's final PQ3 answer:* "each carried date" gets the
   carry's line. P16 runs the carry before `inherit_series_answers`, whose code is unchanged, so
   those dates get the carry's line, its `from:` and its span, and need no instance-to-series
   link (§4, PQ3, item 6).

### Fourth round (2026-09-30)

One finding on T4c; accepted, with its scale corrected and its second fix refuted. Checked against
the `events` worktree: `cli.rs`'s events pass (`read_roster` only when `candidates.is_empty() &&
feeds_failed`), `eventroster::write_roster` (the relevant section keeps `window_start <= start`)
and `surface::coming_up` (lists through `read_roster` and `relevant_events`). Quinn's answers in §4
are unchanged.

1. **A lane date a later fetch drops stays drawn, and the student cannot remove it
   (Important).** Confirmed: the lane reads the ledger, which is append-only, while Coming up
   lists through the roster, which a successful fetch rewrites. No commitment, card or
   `event-check` exists for the uid. Also found: the same holds for a lane date on an executed
   card's `instances:` (T4b), so the trade-off names both sources. *Corrected:* the finding's
   "every week … for the rest of the semester" overstates it. A dropped date is drawn only on the
   day or days it covers, so one cancelled date shows once; it is still wrong on that day.
   *Refuted, the alternative fix* (draw a carried date only while its uid is in the latest roster):
   it contradicts Quinn's "independent of the roster"; `rank` falls back to `read_roster` only when
   every feed fails, so one failed source among working ones would blank every carried date from
   that source; and the relevant section drops a multi-day date on its second day. *Changed:*
   - §4's PQ3 trade-offs gain the case, with the rejected gate and one question for Quinn: accept
     it for the MVP, or ask for a removal path. A removal path is a further write, so the plan does
     not build it; per Quinn's PQ3 instruction it would come back as a blocker, not be assumed;
   - P15's *Used by*, Checkpoint B (named by itself, with what happens on either answer), B2's and
     W's checks, and §8's risk follow;
   - T4c gains `a_dropped_lane_date_stays_drawn_and_unlisted` (nine tests), pinning the behaviour
     for a carried and a card-listed lane date after a successful fetch without them;
   - T8's anatomy §3.7 bullet says the student cannot remove such a date in the MVP and that it
     passes with its days, or names the path Quinn rules;
   - the ledger's T4c counts.

   *Superseded 2026-10-01 (§12):* Quinn asked for a removal path at Checkpoint B. P17 builds it.
   `a_dropped_lane_date_stays_drawn_and_unlisted` is replaced by
   `a_dropped_lane_date_is_drawn_with_its_uid_until_removed`, and T8 names the path.

### Fifth round (2026-10-01)

Three findings, all accepted. Quinn's Checkpoint B answers (§12) are unchanged.

1. **Email-forwarding shares the app files that H2 and T5 now edit (Important).** Confirmed
   against `docs/plans/2026-09-30-email-forwarding-plan.md`:
   - its H5 adds ten names to the console's `generate_handler!` list in `app/src/main.rs`;
   - its T15.1–T15.5 edit `console.js` and `static_assets.rs`;
   - its §10 R2 table lists `app/src/{lib,main}.rs` as shared with `p3-registrar` only.

   Not confirmed: the finding's `commands.rs` overlap. Email's T11.4 puts its commands in
   `app/src/mail.rs`. The row still names `commands.rs`, so T0's diff can settle it. Also found:
   `approvals.rs` (email's T8 against T3.x) and `cli.rs` (email's H2 against T4), which email's R2
   already lists against `j-events`. *Changed:*
   - §2 gains an email-forwarding row and a note to email's controller, and drops the "plus one"
     count;
   - T0 adds email to the overlap check and sends the note;
   - M and T10 gain the pre-merge check and email's conflict routes;
   - T5's header names the shared file, H2 (§7) states the list by names, and W, §6 and §8 follow.
2. **"Removal by uid" only half answers Quinn's "moved" (Important).** Confirmed. A move that keeps
   the uid leaves the lane drawing the old day, because the span is frozen at the first carry line
   (P15) or on the card. Removing that stale day also drops the real new date from Coming up.
   *Changed:* it is no longer a trade-off for information. It is PQ5 (§13), asked before T4c, and
   it gates T4c and T4d. *Also found:* the finding's option (b), the roster's current span, cannot
   give a lane span. §13 records why, and offers a variant.
3. **No Undo contradicts VISION commitment 5 (Important).** Confirmed: VISION's table gives "the
   student asked" the row "It acts at once, with undo", and M2 keeps it for edits. *Changed:* it is
   PQ6 (§13), and it gates T5. If Quinn answers it before T4d runs, a real undo is built with the
   removal, not after. The claim that it "blocks no task" is removed from P17, §6 and §12.

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
3. **PQ3: (b-prime) with the span, Quinn's final answer.** It took three rounds:
   - Quinn first answered (c), listing carried instances at read time with no new write. That was
     blocked: nothing stored links a carried instance to its series, and a carried all-day
     instance is stored nowhere.
   - Quinn then answered (b-prime): one answer line per carried date, by `agent:knowlu.carry`, with
     `from:` the answering card and the series' real verdict, and no further write invented. Its
     lane half was blocked in turn: the line held no event date, and nothing else stored a lane
     date's span.
   - Quinn's final answer puts the span on that same line: `start:` and `end:` as the feed gave
     them. It is still one line per carried date, with no other write.

   Checked against the `events` worktree at `854d727` (§4, PQ3, *Applied*). It can be built as
   approved, including the lane. *Changed:*
   - §4's PQ3 now records the final answer, how it is applied, and the trade-offs. The two blocker
     texts and the open choice are removed.
   - New P15 (the line: its shape, when it is written, how it is read, who uses it) and P16 (the
     carry before inheritance, for Quinn at Checkpoint B). P8 gains the third source and the
     provenance label. P14 compares carry lines.
   - New tasks T2b.4 (`eventledger.rs`, contract-engineer, xhigh, as Quinn directed), T2b.5
     (`eventcarry.rs`, contract-engineer, xhigh) and T4c (`surface.rs`, implementer), each with its
     tests named first.
   - T2b.1 and T2b.2b read the card's `id:`; T2b.2 gains its per-date hook; T4 swaps steps 1 and 2
     and adds `rank_gives_each_carried_date_the_carrys_line_and_lists_it`; T4b leaves carry lines
     to T4c; T5 renders the label.
   - Also changed: the status line, §1, §2, P1, the §5 table and its PQ3 note, T0, Checkpoint B,
     B2, T8 (anatomy §3.7, §3.9 and §3.11, `engine-commands.md`, the spec §12 note), W, T10, §6,
     §8 and the ledger (the §4.5, §5.4, §6.1 `surface.rs` and `cli.rs`, PQ3 and P16 rows, and the
     plan-only list).
   - §10's third-round finding is marked resolved.
   - `the_accept_carry_writes_no_ledger_line` stays removed, because PQ3's line inverts it.
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

## 12. Quinn's Checkpoint B answers (2026-10-01)

Quinn read Checkpoint B on 2026-10-01: the carry's and the settlement's diffs, PQ3's trade-offs and
P16. Quinn answered its two questions. Before the plan changed, each answer was checked against
code.

On the `events` worktree:
- `eventledger::{load_ledger, read_lines, record_answer, record_declined, LedgerEntry}`, and the
  module's pinned tests, `a_human_answer_never_replaces_a_confident_verdict` among them;
- `eventcarry::{run, carry_line, book, turned_down, ledger_reads}` and
  `eventroster::relevant_events`;
- `surface::{the_day, TheDay, coming_up}`, `write::human_gate`, `journal::{OPS, read_human_actor}`
  and `eventaccept::Shape`.

On `main`, where M2 is merged:
- `app/src/commands.rs`' `mutate`, `console_ctx`, `delete_note_inner` and `dropped_events_inner`;
- `eventroster::read_dropped`;
- the console's `generate_handler!` list in `app/src/main.rs`, and `app/tests/handler_lists.rs`;
- `console.js`' `renderTheDay` and its `confirmDelete`.

1. **P16: accepted.** The carry runs before series inheritance in `rank`, reversing spec §6.1's
   signed order.
   - *Changed:* P16's last bullet, §4's trade-off line, T4's table row and behaviour line,
     Checkpoint B's record, §6, §8 (the risk is closed), the P16 and `cli.rs` ledger rows, and
     T8's second spec note.
   - T4 is built as written.
2. **The lane trade-off: not accepted. Quinn asked for a removal path.** When a student accepted an
   all-day or multi-day event that a later fetch drops (cancelled or moved), they must be able to
   remove the stale lane date in the MVP.
   - *What the plan builds (P17).* The smallest write that fits the existing contracts: the
     existing `declined` line, through `eventledger::record_declined`. One new engine function
     writes it, `eventcarry::remove_lane_date` (T4d, contract-engineer), gated to the vault's own
     human token and checked against the lane's own reader. One new Tauri command calls it,
     `remove_lane_date` (H2, the controller). The button is T5's, and T7 tests the command end to
     end.
   - *What was weighed first, and not taken: the student's `drop` answer through
     `record_answer`.* It would put the student's actor on the line. But `load_ledger` lets a human
     answer settle only an `unsure` verdict, a verdict the carry set, or no verdict. So the answer
     is ignored for a card-listed lane date and for a carried date a model judged confidently. For
     those, the lane would keep drawing the date. Covering them needs a human answer to replace a
     confident verdict, which `a_human_answer_never_replaces_a_confident_verdict` pins against. The
     `declined` line covers every source with no change to `eventledger.rs`: `load_ledger` sets it
     whatever the verdict, and every reader the removal needs already honours it.
   - *What it does not need.* No new ledger line shape or verdict word. No journal record: no
     ledger line has one, and `journal::OPS` has no op for one. No vault file or folder, and no
     sync change. No edit to `eventledger.rs`, `journal.rs`, `write.rs` or any contract-list file.
     It needs one Tauri command, which widens spec §6.3's "No new Tauri command" by exactly one;
     T8's spec note records it.
   - *Changed:*
     - the status line, §1 (the ledger and command bullets), §2 (the sibling table and its
       paragraph), new P17, and §4's lane trade-off;
     - the §5 table (T4c, new T4d, B2, new H2, T5, T7, T8) and its notes;
     - T4b: each lane instance keeps its uid, and the lane is ordered `(first day, title, uid)`;
     - T4c: its header and the fourth declined case; the replaced test, and
       `the_lane_names_each_accepted_date_by_uid`; `all_day_uids` and its JSON rule; the reader,
       now `pub(crate)`, with its `declined` rule for card-listed dates. It has ten tests;
     - new T4d, with seven tests;
     - Checkpoint B's record, B2 (T4d under contract-reviewer, and T4c's new checks) and M;
     - new H2;
     - T5 (two tests), T7 (two tests, and `handler_lists.rs`), and T8 (anatomy §3.7 and §3.11,
       `app.md`, the spec note);
     - W, T10 (the gate, and proof step 5), §6, §7 (H2's code), §8 and §9.
   - *Trade-offs, to Quinn with B2 (P17):*
     - no Undo, a confirm instead;
     - the removal is by uid, so a moved event that kept its uid leaves Coming up too;
     - the line is device-local and carries no `by`, so a restored vault draws a card-listed or
       still-fetched date again;
     - the console draws today's lane only.

     *Superseded 2026-10-01 by the fifth review (§10, §13).* The first two are not trade-offs the
     plan may decide. They are now open questions PQ5 and PQ6, and they gate T4c, T4d and T5. The
     last two stay named for information with B2.

**Also found while re-reading, and corrected:**
- **`app/tests/handler_lists.rs` pins names, not a count.** It asserts two lists and the three
  Google commands' places. §2 and §7 said it pinned the count. *Changed:* §2 and §7 say what it
  pins, and T7 adds `remove_lane_date`'s place to it.
- **T4b's lane order had no tiebreak.** Two accepted lane dates with one first day and one title
  had no defined order, and `all_day_uids` makes the order visible. *Changed:* T4b orders by
  `(first day, title, uid)`, and T4c's determinism test checks that order.

## 13. Open questions for Quinn (fifth review, 2026-10-01)

Two questions, asked one at a time, each with its context. Neither changes Quinn's Checkpoint B
answers: P16 stands, and the student must be able to remove a stale lane date in the MVP. These
questions ask *how* that removal behaves in two cases Quinn named or VISION governs. Until they are
answered, `needs_quinn` is these two, not none.

**Both were answered on 2026-10-01: PQ5 (b2) and PQ6 (b).** Each answer is recorded under its
question, and the tasks below apply it. `needs_quinn` from this section is now none, unless
T2b.6's first check blocks (b2), which brings PQ5 back.

**PQ7 is open (raised by T2b.6's review, 2026-10-01).** It asks whether a carried lane date that
the campus moves to clock hours is booked. Until Quinn answers it, `needs_quinn` from this section
is PQ7.

### PQ5. An accepted all-day event that the campus moves (asked before T4c)

*Context.* Quinn's answer covers a date a later fetch drops because it was "cancelled or moved". A
campus feed usually keeps an event's uid when it reschedules it. Example: an accepted all-day fair
moves from Fri to Sat, with the same uid. As P17 stands:
- the lane draws Fri, because a carried date's span is its first carry line's (P15, and the carry
  writes no second line for a uid), and a card-listed date's span is the card's `instances:` entry;
- Sat's lane draws nothing, though Coming up lists Sat through the roster;
- "Remove from my day" on Fri writes `declined` for the uid, so Sat also leaves Coming up for good.

So the student must choose between a stale Fri and losing the real event. A cancellation works as
Quinn asked; a move does not.

*Options.*
- **(a) Accept it as named.** No change. T8 documents it. Cost: Quinn's "moved" case stays half
  answered.
- **(b) The lane draws the roster's current span.** As the review proposed it, this cannot be
  built. The roster (`state/events.md`) stores one `HH:MM–HH:MM` line on one day. `read_roster`
  reads an all-day event back as a one-hour `00:00–01:00` event and cannot hold a multi-day span
  (§2). The line format is pinned by the frozen reference `vault-full/state/events.md` (rule 2).
  So the roster cannot give a lane span.
- **(b2) The carry re-records a moved span.** This is (b)'s intent, built from the fetch instead of
  the roster. When a fetch holds an accepted lane-shaped uid whose span differs from the span the
  lane draws, the carry appends a carry line in the existing shape with the new `start:` and
  `end:`. This covers a card-listed date too: its line's `from:` is the card's id. `load_ledger`
  then reads the last carry line's span, and T4c's reader prefers a carry line's span to the
  card's. There is no new line shape, and no write when the span is unchanged. A move then fixes
  itself on the next successful fetch, and removal is needed only for a cancellation.
  - *Cost:* it reopens two reviewed contract tasks. A new **T2b.6** (`contract-engineer`, xhigh;
    `eventledger.rs` and `eventcarry.rs`) runs before T4c. B2's contract-reviewer reads it alone.
  - *First check:* `contract-engineer` confirms that no pinned assertion says "a uid never gets a
    second carry line" for an unchanged feed. If one forbids a changed span too, (b2) is blocked
    and comes back to Quinn.
  - *Tests:*
    - `the_carry_records_a_moved_lane_span_once` (T2b.6): one moved fetch gives one new line, and a
      second identical fetch gives none;
    - `a_card_listed_date_moved_by_the_feed_gets_a_carry_line` (T2b.6);
    - `a_moved_lane_date_is_drawn_on_its_new_day_only` (T4c).
  - *Not covered:* a moved *timed* date. Its commitment stays at the old time, and the student can
    delete it, as today.
- **(c) Remove by uid and span.** "Remove from my day" writes a new line shape, for example
  `- <uid> · removed <today> · start:<…> · end:<…>`. The lane leaves out only that span, and Coming
  up keeps the uid.
  - *Cost:* a new shape in `eventledger.rs` (`contract-engineer`, xhigh), a T4d rewrite, and a
    check of how an older engine reads the line.
  - *What it does not fix:* the lane still draws nothing on Sat. It only keeps Sat in Coming up.

*Recommendation:* **(b2).** It is the only option that puts the moved date in the lane. It adds no
line shape. Its cost is one contract-engineer task and one re-read in B2. If its first check blocks
it, the plan recommends (a) over (c), because (c) adds a shape and still leaves Sat out of the lane.

*What each answer changes:*
- (a): T4c and T4d as written; T8 names the moved case.
- (b2): new T2b.6 before T4c; T4c gains its test and the span rule; B2 and W check it; §9 gains a
  row.
- (c): T4d writes the new shape; T4c reads it; T7's removal test asserts the new line; B2 adds the
  shape to the contract-reviewer's list.

*Answered 2026-10-01:* **(b2)**, as recommended. The carry re-records a moved lane span. Applied:
- New **T2b.6** (`contract-engineer`, xhigh; `eventledger.rs` and `eventcarry.rs`), after T4b and
  before T4c, with its first check and its two tests, `the_carry_records_a_moved_lane_span_once` and
  `a_card_listed_date_moved_by_the_feed_gets_a_carry_line`. If the first check finds a pinned
  assertion that forbids a changed span, T2b.6 stops and PQ5 comes back to Quinn, with (a)
  recommended over (c).
- T4c gains `a_moved_lane_date_is_drawn_on_its_new_day_only` and the span rule (a carry line's
  span wins over the card's). It has eleven tests.
- B2's contract-reviewer reads T2b.6's commit alone, and W checks it. §9 gains a row, and T8
  documents it.
- T4d's tests stand as written. Its check reads `accepted_events`, which now draws the moved span,
  so a moved date is removable on its new day.
- §1's ledger bullet holds: (b2) writes a second line of the carry's existing shape, and no new
  shape.

*Two rules the plan makes so (b2) moves no pinned assertion* (for Quinn with B2, for information):
- **Same card only.** A later carry line changes only the entry's span, and only when its `from:`
  names the first carry line's card. A line from another card is still ignored. So T2b.4's
  `only_the_first_carry_line_counts` (two cards, two spans, the first holds) passes unchanged.
- **Lane to lane only.** The carry writes the new line only when both the span the lane draws and
  the fetched span are lane-shaped (all-day, multi-day or zero-length). A lane date moved to clock
  hours would otherwise leave the lane with no commitment booked for it.

*Not covered by (b2)*, besides the moved timed date above. In each case the date stays drawn on
its old day and P17's removal stays the answer. T8 names both.
- A lane date the campus moves to clock hours (the second rule).
- A lane date whose uid holds a human answer: an executed `event-check` card's own uid, which
  `settle_event_check` answered as the student. T2b.4's `a_carry_line_never_overrides_a_human_answer`
  pins that a carry line after a human answer is ignored, `carry` included. Reaching this case would
  change that assertion, so it is a further question for Quinn, not part of (b2).

### PQ6. Undo, or a confirm, for "Remove from my day" (asked after PQ5; before T5 at the latest)

*Context.* VISION's design commitment 5 says that when the student asked, Knowlu "acts at once,
with undo". M2 keeps that for edits (`console.js`' `offerBodyUndo`, `offerPrefsUndo`). P17 as
drafted asks once and offers no Undo. That follows Delete…'s precedent (`console.js`'
`confirmDelete`), because a `declined` line is terminal: `load_ledger` never clears it.

*Options.*
- **(a) A confirm, no Undo, as drafted.** Delete…'s precedent. Cost: none to build, but it departs
  from commitment 5.
- **(b) A console-only Undo.** The date leaves the lane at once, and a "Removed · Undo" toast shows
  for 10 seconds. The console calls `remove_lane_date` only when the toast closes, and Undo cancels
  the call.
  - *What it needs:* no line shape and no engine change; only T5 changes.
  - *Costs:* the console must keep the date hidden across `state` polls while it waits. A window
    closed inside the 10 seconds loses the removal, which is the safe direction: the date shows
    again and the student removes it again.
- **(c) A real undo.** The removal writes at once, and Undo writes an un-remove line.
  - *What it needs:* a new line shape in `eventledger.rs` (`contract-engineer`, xhigh), read by
    `load_ledger` and by every reader that honours `declined`. It also needs a second engine
    function in T4d and a second Tauri command in H2 (two names in the console list, and T7 tests
    both).
  - *Risk:* a card's Decline also writes `declined`. If the un-remove line could reverse it, a
    student's Decline could be undone through the lane. So the removal's own line must be told
    apart, which means a removal shape too. This overlaps PQ5 (c).
  - *Timing:* if Quinn answers (c) after T4d is built, T4d is reopened.

*Recommendation:* **(b).** It keeps commitment 5's "at once, with undo" on screen, needs no
contract change, and its one failure mode puts the date back rather than losing an event. (c) is
the faithful undo, at the cost of two shapes and a second command.

*What each answer changes:*
- (a): T5 as written.
- (b): T5's last test becomes `remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes`
  and `undo_within_the_toast_never_calls_remove_lane_date`. Also, the hidden date survives a
  `state` poll. W's and T10's step 5 wait for the toast.
- (c): T4d gains the un-remove function and its tests, and H2 gains a second command. T5's Undo
  calls it, T7 tests it end to end, and B2's contract-reviewer reads both shapes.

*Answered 2026-10-01:* **(b)**, as recommended. A console-only Undo. Applied:
- **T5.** The click hides the date at once and shows a "Removed · Undo" toast for 10 seconds. The
  console invokes `remove_lane_date` only when the toast closes without Undo. Undo cancels the call
  and the date shows again. No `window.confirm`. The confirm test is replaced by §13's two tests,
  `remove_from_my_day_hides_at_once_and_calls_when_the_undo_toast_closes` (with the clause that the
  hidden date survives a `state` poll) and `undo_within_the_toast_never_calls_remove_lane_date`.
  T5 has six tests.
- **T7.** Its end-to-end test stands as the call the console sends once the toast has closed. A
  Rust test drives the command directly and has no toast, so its assertions do not change. The
  toast's timing is pinned by T5's two tests and walked live in T10's step 5.
- **W and T10's step 5** wait for the toast to close before they check the ledger.
- No engine change, no line shape and no second command: T4d and H2 stand. §8's risk and T8's
  anatomy text follow.
- The named cost stands: a window closed inside the 10 seconds loses the removal, and the date
  shows again until the student removes it again.

### PQ7. A carried lane date the campus moves to clock hours (raised by T2b.6's review, 2026-10-01)

*Context.* T2b.6 (`7550b0d`) added a guard to `eventcarry::book`: a date whose `carry` span is
lane-shaped is never booked. T2b.6's test asks for it ("a move to a timed span … also books no
commitment"). But T2b.6's Behaviour says "Nothing else", and T2b.5's reviewed booking did book such
a move. PQ5's "lane to lane only" rule assumes that a lane date moved to clock hours "would
otherwise leave the lane with no commitment booked for it". That holds for a card-listed date, which
the carry never books, because its card's settlement does. It did not hold for a carried date: at
T2b.5, the carry's step 2 booked the move.

Example: the carried date `lx:77:3` is first fetched as zero-length, 2026-10-09 10:00–10:00 (PQ2:
the lane), and gets its carry line. The next fetch gives 10:00–11:00: the feed fixed a placeholder
end.
- At T2b.5 (`f7caa8f`): a soft commitment is booked for 10:00–11:00. The carry span is unchanged,
  so T4c's lane would also draw the 10:00 marker.
- As built (`7550b0d`): nothing is ever booked. The lane draws the stale marker. "Remove from my
  day" on it writes `declined`, so the real event also leaves Coming up for good.

The same happens to:
- a sub-minute event (10:00:00–10:00:30, zero-length at minute resolution);
- a 23:59 event that a later fetch makes past-midnight;
- an all-day date moved to clock hours (T2b.6's own case, Sat 10:00–15:00).

*Options.*
- **(a) Keep it, as built.** The date is never booked and stays drawn on its old day. P17's removal
  drops it. T8 and PQ5's *Not covered* say "never booked", not only "stays drawn on its old day".
  Cost: the accepted event's real time never reaches the plan.
- **(b) Revert the guard.** T2b.5's booking stands: the moved date is booked at its new time, at
  the series' level. The carry span is unchanged, so the lane also draws the old span. T4c then
  decides whether the lane hides a uid that has a booked commitment.
  - *Cost:* a `contract-engineer` fix to T2b.6 and to its pinned clause (`booked(&timed)` empty).
  - *Without T4c's rule:* the student sees the date twice. Removing the stale copy hides the event
    from Coming up, while its commitment stays.
- **(c) Revert the guard and follow the move, for a carried date.** As (b). In addition,
  `follow_moves` appends a carry line with the timed span when the entry already has `carry`. A
  card-listed date gets no such line, because it would then leave the lane unbooked.
  - *Result:* the date becomes what a carried timed date is today. It is booked and has a timed
    carry span, so the lane does not draw it (P17: "a carry line whose span is lane-shaped"). T4c
    needs no new rule.
  - *Cost:* (b)'s fix plus one condition in `follow_moves`. The pinned clause becomes "one line, one
    commitment, one journal `create`". No new line shape.

Under every option, a card-listed lane date moved to clock hours stays as PQ5's *Not covered* says,
because the carry never books a card-listed date.

*Recommendation:* **(c).** It puts the moved date in the plan at its real time without drawing a
stale marker beside it, and it adds neither a shape nor a T4c rule. (b) is the smaller code change,
but it leaves a duplicate for T4c to hide. (a) loses the event.

*What each answer changes:*
- (a): no code change. T8's anatomy §3.7 line and PQ5's *Not covered* say "never booked". B2 reads
  the guard as intended.
- (b): a T2b.6 fix (`contract-engineer`, xhigh). It drops the `lane_date` guard in `book`, and the
  test's timed case asserts the soft commitment. T4c gains a choice for Quinn: hide a uid that has a
  booked commitment, or draw both.
- (c): (b)'s fix, and `follow_moves` writes the timed span for an entry with `carry`. The test's
  timed case asserts one line, the commitment and its journal record. PQ5's "lane to lane only" rule
  narrows to card-listed dates, and T8 names that.

*When.* Asked now, before T4 is dispatched, as the review asks: a later change would move a pinned
clause. It gates T4c. Under (b) or (c), the fix lands before T4c, and B2's contract-reviewer reads it
with T2b.6.

*Still open at B2's partial pass (2026-10-01).* T4 (`68bf88a`) and T4b (`1c309cc`) were built with
PQ7 open. B2's contract-reviewer found that the `lane_date` guard in `eventcarry::book` goes beyond
T2b.6's "Nothing else" and is this question, not a defect to fix before Quinn answers. No T4 test
pins the clause, so any answer still moves only T2b.6's `booked(&timed)` assertion. PQ7 is now asked
before T4c is dispatched. The recommendation stays (c). Under (b) or (c), `contract-engineer` drops
the guard and updates the timed-move clause, and B2's contract-reviewer re-reads that fix together
with T2b.6. Under (a), T8 and PQ5's *Not covered* say "never booked".
