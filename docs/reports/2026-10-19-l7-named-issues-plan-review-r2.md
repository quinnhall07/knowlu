# L7 plan review, round 2 (2026-10-19)

Plan: `docs/plans/2026-10-19-l7-named-issues-plan.md`. Reviewer, plan mode, checked against the email
branch at `163b11a`. Filed by the controller from the reviewer's returned text. Round 1's report is
`2026-10-19-l7-named-issues-plan-review.md`.

**Verdict: PASS.** 0 critical, 0 important, 4 minor. All ten round-1 findings are closed. The minors are
wording edits, made in place without a further round; see the plan's §11 round-2 entry.

## Round-1 findings, all closed

- **The eval gate:** §1, Q10, T8 and the ledger. The `ci.yml` lines match.
- **Day 21:** the second list call fires the rule on day 14.
  - `connected_at` resets on every reconnect (`google.sql:126`).
  - A `quiet` grant keeps its token: `read_google_grant` filters only `revoked` (:165), and
    `lookupFromRow` treats only `revoked` as dead (`handler.ts:154`).
  - The app already maps `quiet` (`account.rs:1670`).
- **Test 4:** moved to the green list.
- **The splits:** all within about 80 lines. T3a is about 80, at the bound; T3b about 40; T2a under 30;
  T2b about 50.
- **`events.rs`:** T3c matches L3's T3. The `Err(e)` catch-all (`events.rs:500`) means the build still
  passes if L7 lands first.
- **Every `Quiet(_)` clears the notice.** `CloudError` has exactly four variants (`cloudmodel.rs:206-214`).
- **Test 1's request order, the Deno line, Q8 and the ledger path:** fixed.

## The three points asked about

- **The second list call.** It leaves the read cap alone: `pagedList` stops after one full page, since
  100 is at least `READ_CAP` 60, and `MAX_LIST_PAGES` (5) bounds it. The cost is acceptable on the raw
  7-day trigger. A failed second call reporting nothing is safe.
- **The T3c handover** is sound in both orders, with two small gaps (minor 2).
- **The split sizes** are all within the bound.

## Minor (closed in place)

1. Decide on the raw `deps.api.list` result, before `.filter(unseen)`, and add a test with only
   already-seen ids.
2. The handover's two gaps: re-check T0(f) just before the merge, and write the hand-off row into L3's
   own ledger.
3. Log a failed second call as `gmail-read: GmailApiError <status>`, with no message text.
4. Re-run Q8's baseline after the rebase onto L6, and correct T8's wording on the `eval_cases` count.
