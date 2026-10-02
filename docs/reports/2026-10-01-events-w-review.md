# Events branch: W whole-branch review (2026-10-01)

Branch `events`, reviewed at 794d961 against origin/main 5ef16c5 (the merge base). Source: the
workflow journal's four W agents (`contract-reviewer: W`, `reviewer: W`, `contract-engineer: W fixes`,
`contract-reviewer: W re-check`). The reviewers ran no cargo; the gate figure below is the fixer's.

## Counts

| Pass | Agent | Critical | Important | Minor |
|---|---|---|---|---|
| Review | contract-reviewer (approvals.rs and the contract-list files) | 0 | 0 | 8 |
| Review | reviewer | 0 | 2 | 5 |
| Fixes | contract-engineer, commit 3f662fe | | | |
| Re-check | contract-reviewer | 0 | 0 | 1 |

W's done-when is no open Critical or Important finding. It is met once the two Important findings
below are closed.

## Important findings

1. **Spec §5.1 rewritten in place, no dated marker, no ruling.** Commits 6bd177f and f7caa8f changed
   the signed card example from plain minutes to quoted seconds
   (`'2026-10-01T10:00:00'`). The fixer confirmed it and recommended acceptance: it is the span
   format the carry line already uses (`eventledger::SPAN_FORMAT`, `%Y-%m-%dT%H:%M:%S`), no shipped
   vault holds these cards, and a minute-only card still reads.
   **Ruled by Quinn, 2026-10-01: accepted.** The spec now carries the dated marker at §5.1 and one
   bullet in the 2026-10-01 note. No other spec text changed.
2. **The B1 report was missing from `events`.** `docs/reports/2026-09-30-events-b1-result.md`
   (0599753, "event-4 does not ship") lived only on `j-events`. Fixed in 3f662fe: the file is copied
   byte for byte (blob 558b1b5, LF) and the local progress ledger gained a T9 row and a W row. The
   report keeps its `j-events` file name, not the plan's `<date>-events-b1.md`.

Fixer's gate at 3f662fe: `cargo test --workspace` 2354 passed, 0 failed, 4 ignored (by design), 0
warnings apart from the accepted `.rsrc` line.

## Re-check (3f662fe)

Both Important findings are handled correctly and the fix adds nothing new. 3f662fe touches no spec,
plan, engine, fixture or contract-list bytes; its trial `git merge-tree` against origin/main is clean.
The one Minor finding is the stale test comment at `engine/src/eventaccept.rs:636`, which still
pointed at a "minutes" spec example. Closed in this commit: the comment now says cards are written to
the second and a minute-only card still reads. The `eventaccept` tests pass (17 passed, 0 failed).

## Minor findings still open at HEAD

None blocks the merge. Each needs a disposition (fix, or accepted by Quinn) before it is called
closed. The plan's W check "Checkpoint B's, R1's and B2's reports are closed" is not met until then.

- `eventcarry.rs:502` (new): `book` books a moved lane date without the card check `follow_moves`
  applies, so with two overlapping executed cards for one series a date can show twice. Fix: skip a
  uid whose lane-shaped carry names another card, with a test. Needs a hand edit or two desktops.
- `surface.rs:3952` (B2 pass 2): a test passes the literal `"quinn"` to `record_answer`. Fix: use
  `journal::read_human_actor`. Assertions unchanged.
- `eventcarry.rs:164` (Checkpoint B 3): `accepted_check_series` accepts `instances: []` that
  `settle_event_accept` treats as no payload. Fix: require a non-empty sequence, add the test case.
- `approvals.rs:1470` and `:1286` (Checkpoint B 4): a retry re-appends answer or `declined` lines,
  so the ledger is not byte-identical to a clean run. Fix: skip a uid that already holds the line, or
  document the duplicate case.
- `eventroster.rs:342` and `:363` (Checkpoint B 2): `read_dropped` lists a carried date whose machine
  verdict is `drop` as "judged not relevant" while the lane draws it. Fix: skip entries with a
  `carry`, with a test, or name the case in the trade-offs.
- `eventcarry.rs:184` (Checkpoint B 1, for Quinn): a booked note deleted outside the app is booked
  again on the next `rank` under a new `cmt_` id. Name it in the PQ3 trade-offs; optionally add each
  journal `create`'s `new.source_uid` to the set.
- `anatomy.md:278`, spec :788, plan :3083 (B2 pass 1 4): PQ5's "not covered" case should read
  "every lane date an executed `event-check` card lists".
- `engine-commands.md:12` and two preset comments: the `event_cards` switch turns opportunity cards
  on and the digest off. Obligation cards are filed either way (D6).
- Local progress ledger: rows for B2 pass 2, 794d961 and T0c are missing, and the last full code
  gate recorded is an estimate. T0c (the Alabama count) is unmeasured and the plan wants it before T10.

## Checked and clean

- `rank` calls no model; the P16 pass order holds (carry, inheritance, roster, obligation cards,
  digest, checks, opportunity cards); inheritance skips every date the carry answered.
- Determinism: only BTree collections, events ordered by (start, uid), the lane by (first, title, uid).
- Writes: notes through `create_confirmed` as `agent:commitments`, the register task as
  `agent:approvals`, journal first, the stamp, then the move; a failed write leaves the card in place.
  A card without `instances:` gives the same ledger and journal bytes as main.
- `remove_lane_date` admits only the vault's own human token, through `student_gate`, and writes one
  existing-shape `declined` line.
- Ledger readers are additive: lines written before PQ3 read as before. No new JSON writer. `VIAS`,
  `LOCAL_CARD_KINDS`, `is_note_path` and run-record keys are untouched.
- `surface` is read-only; the three surface-today references hold; no fixture, frozen reference or
  workflow file is touched; no `#[ignore]` added or removed; no CR byte in a changed file.
- App: `app/src` changed only in H2; the command computes nothing and uses `console_ctx`; the
  console's 10-second Undo matches PQ6 (b).
- Privacy: `site/` and `PRIVACY_VERSION` unchanged; the removal sends no telemetry.
