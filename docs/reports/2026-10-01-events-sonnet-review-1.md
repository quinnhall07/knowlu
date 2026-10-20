# Events: R1, review of the Sonnet commits

- Date: 2026-10-01
- Reviewer: `reviewer` (Opus, high), run beside Checkpoint B
- Scope: T1a 642ee94 with fix a509eab, T1b 6bb7abd, T2a.1a c5e1456, T2a.1b b2ad0f3 with fix 5405908, T2a.2 112f042
- Verdict: R1 passes. 0 Critical, 0 Important, 3 Minor. None blocks Checkpoint B or B2.
- Over 642ee94~1..112f042 only `commitments.rs`, `eventaccept.rs`, `eventcarry.rs` (T2b.1, out of scope),
  `eventemit.rs`, `events.rs` and `lib.rs` change. No contract-list file, no CR bytes, no fixture or reference edit.

## Plan §5 R1 checks

- One shape classifier: `eventaccept::shape` is the only one, `eventemit` adds no midnight or 23:59 logic. P4's
  order holds (all-day, past-midnight, multi-day, zero-length, timed). PQ2 holds: zero-length goes to the lane.
- Free text goes through `judge::one_line` (title 200, location 120, url 500, `where` 1 to 80, quoted `why` 400).
- Every card goes through `write::create` with its `proposed` line after; a blocked folder writes no line.
- Roster events: `needs_check` skips `source: "roster"` for `event-accept` rows, `write_check` writes `instances:`
  only for non-roster events, and a roster `event-check` card keeps the old closing (D11).
- Never-ask-twice reads both kinds in `approvals/` and `archive/`, the union of `settled_series` and
  `eventcarry::answered_series`, with no local copy.
- Caps are counted per kind, and per `verdict:` for `event-accept`, as `min(budget, 3 - today's)`.
- Test 4 compares file names in order, card bytes with `id:` lines removed, and identical `seen` ledgers.
- No `"quinn"` or `"student"` literal; production code builds no `WriteContext`.
- `event-accept` is not in `LOCAL_CARD_KINDS`. Spec text (titles, closings, file name, expiry, P10 windows,
  opportunity `sort_key`) matches spec 4.1, 4.4, 5.1, 5.2, D3, D6, D7.
- T1b: the `kind == "event"` arm is tested first and `ends` never reaches frontmatter.
- PQ1 and PQ3 land in T2b, outside R1, and nothing here conflicts with them.

## Findings

| # | Severity | Where | Problem | Disposition |
|---|---|---|---|---|
| 1 | minor | `eventaccept.rs:188` | `commitment_for` returned `None` for `Timed` and `PastMidnight` events whose start and end are equal at minute resolution, and `Shape::lane()` is `None` for both. An 11:59pm to 12:30am event, or a 10:00:00 to 10:00:40 event, showed only in Coming up. | Fixed in 6bd177f: `shape` classifies at minute resolution, so these are `ZeroLength` and reach the lane. Confirmed by B2's second pass. |
| 2 | minor | `eventaccept.rs:28` | `Instance::from_event` kept seconds but `to_node` wrote `%H:%M`, so a 10:00:30 start read back as 10:00 and the payload could disagree with P15's `:SS` carry line. | Fixed in 6bd177f and f7caa8f: instances are stored to the second, quoted. The format change rewrote spec 5.1's example without a ruling; see B2 pass 2, finding 2 (for Quinn). |
| 3 | minor | `eventemit.rs` (c5e1456) | The T2a.1a pinning test was committed with the refactor, so history cannot show it was green on the old code. No code change needed. | Rejected as a code finding. The controller confirms the SDD ledger records the pre-refactor green run and the id-stripped byte comparison. Later refactor tasks ask for the pin test as its own commit. |
