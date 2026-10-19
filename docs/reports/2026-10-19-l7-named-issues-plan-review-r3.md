# L7 plan review, round 3, delta (2026-10-19)

Plan: `docs/plans/2026-10-19-l7-named-issues-plan.md`, r3. It was reworked for Quinn's ruling on Q3: quiet means
no message of any kind for 7 days, not 14. Reviewer, plan mode, with the code checked against the email
branch. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL.** 0 critical, 2 important, 5 minor. The rule itself is correct against the handler. Both
important fixes are a paragraph of plan text each, so a delta check of those lines is enough.

## §3.3 against the handler: the rule holds

- **The raw list.** `api.list` returns every raw id from `pagedList` (index.ts:46-54, handler.ts:110-125).
  `.filter(unseen).slice(READ_CAP)` runs after that (:351-353), so reading the result before the filter is
  sound.
- **A failed list** reaches the outer catch before any check (a 500 and one class-and-status log line,
  :423-431). Test 5 pins this.
- **The other paths.** `no_gmail_scope` and `revoked` return first (:321-334). The forwarding path returns at
  :309-311.
- **`quiet` is never treated as revoked.**
  - `lookupFromRow` refuses only `revoked` (:154). `google-connect` empties scopes only for `revoked`.
  - No function filters `google_accounts` on `status=eq.active`.
  - `markRevoked` is unconditional (index.ts:121), and a reconnect resets the row to `active`.

## Important

1. **Nothing pins that a row already `quiet` keeps answering `went_quiet: true`** (:784-803).
   - Scenario: an implementation answers `true` only when its conditional write changed a row. On the next
     slot the row is already `quiet`, so it answers `false`.
   - The engine's clear then closes the heads-up, and the slot after reopens it. The heads-up flickers every
     slot.
   - Fix: add test 4b. Start from a `quiet` row, an empty list and a 30-day-old grant. Expect
     `went_quiet: true` and no successful status change. Cite it in §7 :1083 and T7 5a.
2. **No task commits the §5.3 marker** (:1323-1326, :1083).
   - The marker exists only as an uncommitted edit in the main checkout, beside unrelated uncommitted price
     amendments.
   - T2b's diff (:811), T6's list and T7 check 9 leave it out.
   - Fix: name who commits it (the controller, docs-only, alone, before T0). Add "the §5.3 marker is on
     `main`" to T7 check 5a.

## Minor

3. **The age comparison** (:781 against :297-298).
   - :781 says "in whole 24-hour days"; §3.3 compares milliseconds against 7 × 24 h.
   - Test 3, at exactly 6 days, does not catch `Math.round` or `Math.ceil`.
   - Fix: whole days describe only the fixture ages. Add a case at 7 × 24 h − 1 ms (not quiet).
4. **The preamble at :782-783 says every case asserts exactly one list call.** Tests 7 and 8 assert zero.
   Say "tests 1-6".
5. **The status write's order and failure are unspecified** (:290-305).
   - If the `quiet`→`active` write runs after the loop, a mid-loop throw leaves a row with mail marked quiet.
   - Fix: put the write straight after `list` returns, before the loop. A failed write is a 500, consistent
     with `markSeen` and `enqueue`; say so.
6. **The Testing-mode claim overstates the edge** (:306-311, :1335-1338).
   - `connected_at` is stamped after the code exchange (privileges migration :62). Testing-mode tokens
     therefore die before `connected_at` + 7 d, and the rule cannot fire in Testing mode.
   - The interplay is sane: at worst one slot of "gone quiet", then `revoked` clears it. C2's "yes" stands.
   - Fix: reword both passages, and drop Q4's `true` half.
7. **Citations and wording.**
   - `store_google_grant`'s live definition is `20260911000300_google_privileges.sql:62`.
   - Plan :28 should quote the marker in the spec's own lowercase form.
   - The marker should name the plan's path, not just "L7 Q3".
   - Note that the 14 days at `index.ts:125,131` belongs to the `gmail_seen` window, not this rule.

## Checked, nothing to report

- **Stale references:** none.
- **The migration exclusion is correct.** `migrations_test.ts:980-987` pins the file's SHA-256. C1's README
  note is right.
- **Tests 2, 4 and 5 each go red** against the plausible wrong implementation.
