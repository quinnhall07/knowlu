# Deadline-ordering lane review (T7)

The `reviewer` and `contract-reviewer` agents reviewed read-only, and the controller filed this report. The diff runs from `b1de78f` to `cf9f8e2`, with the controller's fixes after it.

## contract-reviewer: `deadline_order_oracle.rs` re-read, PASS
- The file is unchanged since `088c8f1`: its blob is identical.
- DO1 still checks both goldens at four clock times. DO2 checks the four ruled blocks at 09:00 and 21:00, a strict subset of E3's.
- `oracle.rs`, `surface_oracle.rs` and `evening_rank.rs` are unedited at every commit in the range.
- No fixture or contract-list file is in the diff.
- Minor: plan T1o cited E2 where E3 is meant. Fixed in `7ae6eec`.

## reviewer, round 1 (`b1de78f..f79a3c3`): FAIL on the T5 evidence
The code is clean against every T7 check:
- only T0's seven retire-listed tests are deleted, and only the two named tests are re-pointed;
- no vault write path changed;
- the sort keys are deterministic.

The findings:
- **Important 1.** T5's shots came from a hand-staged JSON, with strings the engine never produces, and the `spill_blocks == 0` carry was never shot. Fixed in `cf9f8e2`: the shots now come from real `surface` output on scratch vaults (§9's T7 demo), and the hand-staged files are removed.
- **Minor 1.** HANDOFF's D1 and E1 rows were stale. Fixed in `79cefbb`.
- **Minor 2.** anatomy.md's horizon sort was not qualified as applying without a moment. Fixed in `79cefbb`.
- **Minor 3.** The meter key said "0 blocks do not" and "nowhere to go" for items that only miss a deadline. Fixed in `cf9f8e2`.
- **Minor 4.** DC1's bare `"fit"` substring check would also match an ordinary word. Fixed in `79cefbb`.

## reviewer, round 2 (`f79a3c3..cf9f8e2`): PASS
Every round-1 finding is closed. Two wording minors remain:
- **The key line's grammar with spill hours and deadline items.** It read "1.0h across 1 item won't make its deadline", and the mixed form used a plural verb on 1. The controller fixed both after this round: it now reads "1.0h over; 1 item won't make its deadline", and the mixed form agrees in number. `node --check` passes and `static_assets` gives 164 passed. The reference pages have no deadline items, so their key line is unchanged.
- **HANDOFF's E1 row** still reads "review next". That predates this lane and is left for the next docs-keeper pass.

## Gate
The controller ran `cargo test --workspace -j 2 --no-fail-fast` at `cf9f8e2`:
- 3208 passed, 0 failed, 3 ignored;
- the only warning is the accepted `.rsrc` line.

CLAUDE.md says four tests are ignored. `runtime.rs`'s smoke test went with the runtime-removal lane, so three is correct, and CLAUDE.md's count is for Quinn to correct.
