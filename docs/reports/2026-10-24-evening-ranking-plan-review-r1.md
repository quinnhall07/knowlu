# Evening ranking plan review, round 1

**Plan:** `docs/plans/2026-10-24-evening-ranking-plan.md`. **Spec:**
`docs/specs/2026-10-23-evening-ranking-rule-draft.md` (ruled by Quinn, 2026-10-24).
**Verdict: FAIL**: 0 critical, 5 important, 7 minor. The rule itself and the frozen-reference
reasoning hold up; the failures are places where the plan contradicts itself or leaves a decision
open. Relayed by the controller on 2026-10-24 and recorded here by the planner, in the planner's
words. Controller's calls are marked.

## Important

1. **T1's `Rank` stub would break the oracle.** Wiring the `rank` arm to a `todo!()` `run_at` makes
   `oracle.rs`'s golden tests panic from T1 until T3, which invites a test-writer to touch a
   contract-listed test. The stub must keep calling `run_with` and ignore `--now`.
2. **`--now` parsing was too loose.** jiff's `DateTime` `FromStr` accepts a bare date as midnight,
   so C1's "a bare date is refused" could not hold. Use the strict `%Y-%m-%dT%H:%M` parse that
   `models::coerce_datetime`'s `parse_exact` uses, and say whether seconds are accepted.
3. **The `surface.rs` turn was open-ended.** The `surface.rs` stub and tests sat in T1, ahead of a
   wait on L7 and L5. Controller's call: L7 has merged and L5 is not built, so T4 runs when ready,
   and L5 rebases over about ten lines if it starts later. The `surface.rs` red tests move into
   T4's own red commit.
4. **Split T1** into T1a (`ranking.rs`, R1-R15), T1b (`cli.rs` and the engine's `main.rs`, the C
   tests, E1 and E3) and T1c (`surface.rs`, the S tests and E2, in T4's turn).
5. **The meter's spill list changes after the hour, and the plan did not say so.** `surface::meter`
   fills capacity greedily in rank order, so from 20:00 tonight's task fits and the overdue one
   spills. Controller's call: the swap is intended. The spec records it, the ledger has a row, and
   an S test pins it (3h of capacity, a 3h overdue lab, a 1h task due tonight).

## Minor

6. `load_at` is `pub`.
7. C4 must not compare against the wall clock's date (UTC against local time can flake near
   midnight); assert the stamps are neither 2026-08-28 nor 21:00.
8. R4's importance-quirk parenthetical is wrong for a test whose ties differ in pressure: drop it,
   or use a pair of equal pressure.
9. The conflict map misses wave B's F7a (a new `Command` variant and arm in the engine's
   `main.rs`, a separate hunk) and L5's `cli.rs` edit. O1 has merged (#64).
10. Controller's call: `--now` without `--today` takes the day from `--now`. Add a test.
11. T7 should state the expected result: at 19:59 the first group is `Overdue`; at 20:00 it is
    `Due today`.
12. Record Q-2 and Q-3 as the controller's rulings in §9 and update the status line. The base is
    `cb3b661` (its engine matches `1d3c467`). Q-1 stays open for Quinn.
