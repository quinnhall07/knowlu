# Events: Checkpoint B contract review

- Date: 2026-10-01
- Reviewer: `contract-reviewer` (Opus, xhigh), read-only
- Scope: T2b.1 to T2b.5 and T3.1a to T3.2, the `events` branch at 10071f6 against origin/main 5ef16c5
- Files read: `engine/src/eventcarry.rs`, `eventledger.rs`, `approvals.rs`, `events.rs`, `eventemit.rs`, `commitments.rs`
- Verdict: 0 Critical, 0 Important, 5 Minor. The gate output was not in the dispatch, so
  `cargo test --workspace` and 0 warnings were not verified by this review.

## What was checked

- Carry: `answered_series` has one definition, and the emitter and `judge_roster` call it. `accepted_check_series`
  feeds only the accept carry. A series is claimed the way `settled_series` claims one. A card with no
  `instances:` carries nothing. The ever-written set reads `commitments/`, `tasks/` and `archive/`, not approval
  cards. Events read from the roster write nothing.
- Carry line: one line per carried date, written after its booking step whatever the outcome. None goes over a
  human answer, a `declined` line or an earlier carry line. A line the ledger could not read back is never
  written. Carried dates stay out of the digest.
- Ledger reader: the change only adds. `record_answer`, `VALID_VERDICTS` and `ANSWER_VERDICTS` are unchanged.
  `opportunity` is accepted only by `record_carried_answer`. No existing vault line has an `agent:` `by`, so every
  existing line reads as before.
- Settlement: notes first (commitments as `agent:commitments`, the register task under `agent:approvals`), then
  the `executed` stamp, then the move. A failed write leaves the card `approved`, and a retry writes no second
  note. An `event-check` card without `instances:` gives the same ledger and journal bytes as before.
- Hygiene: each commit touches only its declared files. No assertion removed or weakened, no `#[ignore]` changed,
  no fixture touched, no `quinn` or `student` literal added. `merge-tree` against origin/main is clean.
- Not wired yet: `cli.rs` unchanged, so the carry and P16 do not run in `rank` until T4. P16 and the lane-date
  trade-off were Quinn's calls (answered at Checkpoint B, plan cfd50c8), not defects.

## Findings

Dispositions are as of branch head 02ae77a. "Open" means no commit on the branch addresses it.

| # | Severity | Where | Problem | Disposition |
|---|---|---|---|---|
| 1 | minor | `eventcarry.rs:288` (`book`) with `:177` (`ever_written`) | A note the student deletes outside the app (Explorer, or an emptied `archive/`) is booked again by the next `rank`, with a new `cmt_` id, every time. Spec 5.3 assumes deletes go through `delete_note`, so the signed spec accepts this, but PQ3's trade-off list does not name it. | Open. For Quinn with the PQ3 trade-offs. Optional code fix: add each journal `create`'s `new.source_uid` to the ever-written set, with a test that deletes the file directly. |
| 2 | minor | `eventcarry.rs:500` (`turned_down`) with `eventroster.rs:366` (`read_dropped`) | A machine `drop` is no answer, so the carry books that date and writes a line keeping verdict `drop`. With M2 merged, the console's Not shown list would say "judged not relevant" for a date on the schedule. | Open. Fix at step M or T4c: `read_dropped` skips entries whose `carry` is set, with a test. Otherwise add to the trade-offs Quinn sees. |
| 3 | minor | `eventcarry.rs:157` against `approvals.rs:1793` | The two readers disagree on `instances: []`. `accepted_check_series` accepts any sequence, `settle_event_accept` treats an empty one as no payload. Only a hand edit produces it. | Open at 02ae77a (`accepted_check_series` still tests for a sequence only). Fix: require non-empty and add the case to `an_event_check_card_without_instances_carries_nothing`. |
| 4 | minor | `approvals.rs:1470-1477` and `:1287` | A failed approved `event-check` settlement, or a failed rejected `event-accept` move, re-appends its answer or `declined` lines on each retry. Reads are unchanged; the ledger is not byte-identical to a clean run. | Open. Optional: skip uids whose entry already holds the answer, or note it in the doc comment next to "answering twice is harmless". |
| 5 | minor | `eventcarry.rs:264-268` (`run`) | Each `rank` re-parses the archive several times (`rebuild_declines`, `answered_series`, `accepted_check_series`, `ever_written`, `shown_on_a_card`, plus the emitter's scans). Spec D9 priced one extra read. | Open. Can wait until after T4: build the views in one archive pass behind a private struct. |
