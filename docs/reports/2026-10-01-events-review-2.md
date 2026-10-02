# Events: B2, second review (two passes)

- Date: 2026-10-01
- Reviewer: `contract-reviewer` (Opus, xhigh), read-only, both passes
- Base: 10071f6
- Pass 1 scope: R1 fixes 6bd177f and f7caa8f, T2b.6 7550b0d, T4 68bf88a, T4b 1c309cc (T4c and T4d not yet built)
- Pass 2 scope: the above plus T2b.6b 9b71e3d, T4c 9e2fce2 and T4d aeaaffe
- Verdicts: pass 1 partial, 0 Critical, 2 Important, 4 Minor; pass 2 0 Critical,
  0 Important, 3 Minor. After pass 2, B2's Done-when (no open Critical or Important) is met on code alone.

## Pass 1 (partial)

B2 was dispatched before its prerequisite. Plan §5 says B2 runs after T4d, but the branch had no T4c or T4d commit
(10071f6..1c309cc held only R1's two fixes, T2b.6, T4 and T4b). The SDD ledger records both tasks blocked on PQ7.
The pass covered T4, T4b, T2b.6 and R1's fixes, found no Critical finding, and was not a basis for the push go.

Read and found sound: T4's pass order is P16's (carry, inherit_series_answers, write_roster, then cards in D7
order), each emitter is sized to what the earlier steps left, and with the switch off the digest and checks get the
old budgets, so the golden holds. T4 tests 19 and 20a assert what spec §11 states. T4b reads only. T2b.6 moves only
the span, only for a line from the same card; no test line edited; no contract-list file, fixture or `#[ignore]`
changed.

| # | Severity | Where | Problem | Disposition |
|---|---|---|---|---|
| 1 | important | plan §5 B2 (`plan:675`) | B2's scope and Done-when could not be met: T4c and T4d absent, so `accepted_events` read no ledger and dropped no declined uid, and no `remove_lane_date` existed. | Fixed by sequence. Recorded in ef5dc8a (plan: first pass was partial). T4c built in 9e2fce2, T4d in aeaaffe, B2 re-run as pass 2. |
| 2 | important | `eventcarry.rs:436` | T2b.6's `lane_date` guard in `book` changed T2b.5's reviewed booking (PQ7): a carried lane date later moved to clock hours was never booked. | Fixed in 9b71e3d after Quinn's PQ7 (c) ruling (plan 50247d9): guard dropped, `follow_moves` writes the timed span. Pass 2 re-read it and found it sound. |
| 3 | minor | `eventemit.rs:2294` | R1 fix 6bd177f changed a pinned T2a.2 assertion to the seconds form, and f7caa8f rewrote spec 5.1's example in place before a ruling. | Open, carried to pass 2 finding 2 (for Quinn). |
| 4 | minor | `eventcarry.rs:513` | PQ5's "Not covered" case is wider than the plan says: `settle_event_check` writes a student answer for every uid in `events:`, so every lane date an event-check card lists is answered. | No code change. T8 (02ae77a) and plan §13 should say "every lane date an event-check card lists"; not verified here. |
| 5 | minor | `eventaccept.rs:212` | The `debug_assert!` in `commitment_for` replaced a silent `return None` on a path that must not panic. | Open. Pass 2 judged the assertion always true; the guard restore is optional. |
| 6 | minor | `.superpowers/sdd/.../events-progress.md:27` | T2b.6 landed before T4 and T4b, against plan §5's order. No pinned assertion moved. | Open. One line in plan §5/§6 recording the order. |

## Pass 2

Verdict: 0 Critical, 0 Important, 3 Minor. On code alone, B2's Done-when is met. Not checked: the gate output
(supplied by the implementer, not run by the reviewer) and the first-check entries for T2b.6 and T2b.6b in the SDD
ledger, which is the controller's run.md and is not in the repo. 9b71e3d's message records T2b.6b's first check;
7550b0d's message only says the T2b.1 to T2b.5 tests are unchanged.

Read, against the plan's B2 list, base 10071f6:

- R1 fixes 6bd177f and f7caa8f: `shape()` classifies at minute resolution. The new `debug_assert` in
  `commitment_for` always holds, because Timed and PastMidnight always have start < end at the minute. Spans are
  stored to the second, and a minute card still reads.
- T2b.6 (7550b0d) and T2b.6b (9b71e3d): the read rule takes a later line only from the same card and changes only
  `carry`. The `lane_date` guard is gone from `book`, which is back to exactly the T2b.5 condition. `follow_moves`
  accepts a timed fetched span only when the uid has `carry` and its card does not list it; a card-listed date or a
  carried timed date never gets one. A timed move gives one line, one soft commitment and one journal create; a
  second run writes nothing. No T2b.4 or T2b.5 assertion edited; the only clause that moved is T2b.6's own timed
  case, as the plan sanctions.
- T4 (68bf88a): order is carry, inheritance, roster, obligations, digest (switch off only), unsure checks,
  opportunities (switch on only). Each emitter gets what the earlier ones left. A run built from the roster writes
  nothing. Tests 19 and 20a assert ledger and journal bytes; the PQ3 test asserts exactly one line per date.
- T4b and T4c (1c309cc, 9e2fce2): read-only. `accepted`, `provenance` and `all_day_uids` are left out of the JSON
  when empty, and `all_day_uids` stays parallel to `all_day`. A `declined` uid leaves the lane whatever its source.
  A carry span wins over the card's, and a timed carry span (PQ7) leaves the lane. A card-listed moved date gets no
  provenance.
- T4d (aeaaffe): the only write is one `record_declined` line. `eventledger.rs`, `journal.rs` and `write.rs` are
  unchanged. `student_gate` matches `write::human_gate` and also refuses `agent:` and `system:` actors. Removal
  checks the same `accepted_events` the lane draws from.
- Invariants: no fixture bytes touched, no `#[ignore]` added or removed, no new JSON writer, VIAS untouched, no sync
  or credential change.

| # | Severity | Where | Problem | Disposition |
|---|---|---|---|---|
| 1 | minor | `engine/src/surface.rs:3952` | T4c's test `a_declined_or_human_answered_date_is_neither_listed_nor_drawn` calls `record_answer(.., "drop", "quinn", None)`, a new human-actor literal. The plan says the actor comes from `journal::read_human_actor`. It passes only because vault-full has no `actor.yaml`; on a vault whose `human_actor` is `student` it writes a token the vault does not own. | Open at 02ae77a (literal still at line 3952). Fix: `let human = crate::journal::read_human_actor(&v).unwrap();` and pass `human`, assertions unchanged. Goes to `implementer` (T4c). |
| 2 | minor | `docs/specs/2026-09-29-events-design.md:309` | f7caa8f rewrote the signed spec's §5.1 card example in place: `start: 2026-10-01T10:00` became `start: '2026-10-01T10:00:00'`, the bytes every event-accept and event-check card now writes. The commit message says Quinn's ruling is still open, the spec has no dated marker, and plan §13 does not list the ruling. No shipped vault holds these cards, so no existing bytes break. | For Quinn. Add a dated marker at §5.1 (for example "Amended 2026-10-01 (R1 minor 2), pending Quinn's ruling") and list the format ruling with B2's report so the push go covers it. Docs change, `docs-keeper`. If Quinn declines, revert both commits' format changes together. |
| 3 | minor | `engine/src/eventcarry.rs:502` | Since PQ7 (c), `book` books a carried date whenever the local ledger shows it not turned down, and the P17 `declined` line never syncs. Desktops A and B on one account both carry lane date X; the student removes X on A; the feed moves X to 10:00 to 15:00; B books a `kind: event` commitment, sync delivers it to A, and A draws a block for a date the student removed. The MVP is one computer per student until Launch, so this extends P17's named device-local trade-off, not a pinned rule. | For Quinn, for information. No code change in this branch. Add one sentence to P17's device-local trade-off and to §8's two-desktop risk: since PQ7 (c), another desktop can book a removed lane date that moves to clock hours, and that booking syncs back. |
