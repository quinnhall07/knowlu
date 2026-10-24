# Evening ranking plan review, round 2

**Plan:** `docs/plans/2026-10-24-evening-ranking-plan.md`. **Verdict: PASS**, 0 critical, 0 important,
4 minors, all closed in the plan's §9.2. No further round. Relayed by the controller on 2026-10-24;
recorded by the planner.

1. **E2's `nav_counts`** includes `runs_warn` from `runs::expected_status`, which reads the clock, so
   it differs at 21:00 for reasons that have nothing to do with ranking. E2 now compares
   `nav_counts` with that field removed.
2. **`--now` alone is a real run.** Without `--today`, `--now` sets the `today` that `info_pass`,
   `defer_over_budget`, `process_approvals` and `count_proposals_created` date their writes by
   (`cli.rs:410-440`). §1 and T5 now say "beyond the day, exactly as `--today`", so no one reads
   `rank --now` as a dry run.
3. **One clock in T3.** `run_with_at` reuses the existing `now` binding (`cli.rs:370`) and makes no
   second `local_now` call.
4. **Single digits in `--now`.** jiff's `%m`, `%d` and `%H` accept one digit. Controller's call:
   accept them and say so. T3 states it, and C1 pins `2026-8-28T9:00`.
