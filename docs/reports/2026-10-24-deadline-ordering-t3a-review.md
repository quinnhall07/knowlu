# T3a review: deadline ordering, `ranking.rs` (diff 7e9208d..a57b667)

Reviewer: `reviewer` (read-only; the controller filed this report). **Verdict: PASS.** Nothing
Critical or Important; three Minor notes. Test counts are the implementer's (3172 passed, 0 failed,
3 ignored; only the accepted `.rsrc` warning).

## The four rulings
1. **`plan_at` copies `rank_at`'s sort and override insertion: accepted.** It is an exact copy: the
   same decorated key, `total_cmp` then `-importance`, a stable sort and the same insertion
   arithmetic. The plan forbids touching `rank_at` before T3c; T3c's DP7 guards against drift.
2. **Timeline capped at 365 dates: accepted** (spec §2.3). See m-2.
3. **Moment truncated to the minute: correct** (§2.1, §5; DP6).
4. **Must do is `slack_days <= 0.0` over the non-override list: correct.** It is the same predicate
   as `partition_must_do`, and overrides are excluded (§2.1, §4/Q8).

## Other checks
- **Determinism:** no HashMap, stable sorts, and position-unique keys.
- **Midnight and timezone:** blocks end at the next date's 00:00, and minutes are civil (§2.3).
- **30-minute margin:** `total <= before(d) - 30`, so exactly 30 minutes spare counts as safe
  (§2.4; the DP9 edge).
- **Placeholders:** out of scope for T3a (T3b/T4c).
- **Spec fidelity:** steps 0 and 1 match §2.1, §2.4 and §2.5. `rank_at`, `EVENING_HOUR` and
  `closes_tonight` are untouched.
- **Ignored count:** 3 is expected. 3c46fd6 (L6) deleted `runtime.rs` and its smoke test, so
  CLAUDE.md's "four" is out of date (a docs-keeper fix, not this lane's).
- **Gates:** no CR bytes, `git diff --check` clean, no single-user assumptions.

## Minor
- **m-1:** `deadline()` (`ranking.rs:185`) and `closes_tonight()` (`ranking.rs:66`) both claim to
  be "the one place" O6's late-window close goes. The claim is temporary, because T3c deletes
  `closes_tonight`. Note it in T3c's brief.
- **m-2:** the 365-date cap can sit one or two days short of `start_by`'s bound, for absurd
  estimates only. The result is still deterministic. No action.
- **m-3:** `progress` over 100 gives negative `work_minutes`. This follows the spec literally and
  predates T3a. T3b's walk should decide whether to clamp at 0.
