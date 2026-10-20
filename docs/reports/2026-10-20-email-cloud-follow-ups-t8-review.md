# Email cloud follow-ups: whole-branch review (T8, r2)

**Verdict: PASS.** 0 critical, 0 important, 3 minor. r1's important finding is closed. None of the minor findings has to be fixed before the PR.

Scope: the worktree `C:/Users/danie/GitHub/knowlu/.claude/worktrees/email-cloud-follow-ups`, `origin/main...HEAD` (7c46b23, d9115b2, 9dda565, 91734be, 67ab89f), checked against the plan's T8 and the email-forwarding spec. T6 is held under C3 and is not in this branch.

## Confirmed

- **r1's important is closed.** `git status --porcelain` is empty at HEAD 67ab89f. CI's Deno line, run from the worktree root, gives `ok | 1129 passed | 0 failed`.
- **Money: no unpaid account is charged.** `processItem` runs the gate right after the lease. A held item gets no S3 read, no parse, no rules read, no charge, no model call and no queue entry. `deps.model` is reached only through `processItem`, after the gate. A failed entitlement read throws, so the item waits out its lease with nothing charged and no attempt spent. The test `an_unentitled_accounts_item_makes_no_model_call_and_no_charge` checks this for the right reason: the fake database pushes every RPC name into `events`, and the test asserts `events` is empty.
- **The hold and the mark.**
  - The hold writes `attempts` back unchanged and keeps the `mail_seen` row; the sweep and D8 still bound it.
  - `mail_mark_lapsed_from` never touches `off` or `revoked`, refuses a future time, and only ever moves `lapsed_since` earlier. Repeat runs write nothing, so `last_lapsed_at` cannot keep voiding a `covered` report.
  - Test 3c would fail without the `finally`.
- **Retention.** A held object lives only as long as the bucket's existing one-day lifecycle, which is within the page's "otherwise up to about two days in S3". If the account renews after the object is gone, the existing 404 path expires the item.
- **Receipt count.** One guarded update, atomic under the row lock. The count runs after every hand-off, so a retry after a partial failure counts once. The fake S3 is faithful, and each test says why the object is still there. Only a SHA-256 of the object key is stored; neither the hash nor the key is logged or answered.
- **Rotate and D22.**
  - `rotated_from` is cleared wherever the retired pair is; `endRotateGrace` is the only place that clears it.
  - D22 counts a `pending` row only while `retired_until` is in the future, and reads the clock only for such a row.
  - A second Rotate inside a grace records `pending`, which correctly turns the OAuth read back on, because the original forward no longer binds.
- **Migration rights.**
  - Both functions are `security invoker` with `search_path = public, extensions`.
  - Each has `revoke … from public, anon, authenticated` and `grant … to service_role`.
  - `returning true into` targets a primary key, so at most one row comes back.
  - The raw file has no `models`.
  - The stamp `20261020000400` sorts after main's greatest, `20261020000100`.
- **Applied files.** No applied migration changed. Both new pins match my own hashes of the `origin/main` blobs (`62336f3a…129f`, `a1eb2848…de8d`), and each file has only one commit.
- **Spec markers.** T7 wrote markers for Q1 (§4.7), Q2 and Q3 (§4.6 step 3), Q4 (D22), and (d) (recorded, not amended). There is no Q5 marker. The marker text matches §4's table, and Q3's third residual came before Quinn's rulings (§11).
- **Gates and privacy.**
  - Every changed file is LF.
  - No workflow, engine or app change.
  - New log lines are fixed phrases.
  - `counted_receipts` is on the secret-column list.
  - No new data class needs the privacy page.
  - Nothing names a person, vault, machine or credential.
- **Not run by me:** `deno check` and `deno lint`. They are T9's gate. `deno test` already type-checks every changed module.

## Critical

None.

## Important

None.

## Minor

1. **`cloud/supabase/functions/mail-process/handler.ts:415-428`: the marks run one at a time, after the run's 50-second budget (r1's minor, still open).**
   - Scenario: one run holds 500 items across 500 lapsed mailboxes, and then makes 500 mark calls in sequence. At about 100 ms each, that adds about 50 seconds.
   - Why it is not urgent: that is still inside the platform's 150-second limit. If the worker is killed, the holds stand and the next hourly re-check marks again. It matters only at a scale the MVP (testers) will not reach.
   - **Does not need fixing before the PR.** Fix later: run the marks with `Promise.allSettled` in chunks of about 10, or stop marking past a cap and count the rest `failed`. Add a §9 line for it.
2. **`docs/specs/2026-09-30-email-forwarding-design.md:359`: a reference that leads nowhere.**
   - The marker ends "and §9's rotated-Gmail one". In the spec, §9 is "The Gmail OAuth lane, and other lanes", which says nothing about this. A reader of the spec cannot find the residual it names.
   - T7 copied the plan's table faithfully; the problem is in the table's wording.
   - Fix: write "the email cloud follow-ups plan's §9 (a rotated Gmail can keep its OAuth read off past the grace)". It points at a residual Quinn already ruled on, so it adds no new ruling.
3. **`HANDOFF.md:142`: a mail-loss follow-up is tracked only inside the plan.**
   - The plan's §9 rotated-Gmail path is pre-existing, but this lane's Q3 mark makes it reachable with no arrival during the lapse:
     - A student rotates an `active` Gmail mailbox, then their account lapses.
     - A held item marks the mailbox `lapsed`.
     - On renewal, the unlapse writes `quiet`, so D22 keeps the OAuth read off.
     - The new address's confirmation cannot land, because it is guarded on `pending`.
     - After `retired_until`, the old forward's mail is deleted as unbound.
   - The L11 row names only T6 as owed.
   - Fix: add "owed: unlapse writes `pending` inside an open Rotate grace (plan §9)" to the L11 row, so it does not drop out when the worktree is removed.

## Controller follow-up (2026-10-20)

Minors 2 and 3 applied in the docs commit that files this report. Minor 1 (the sequential marks) stays open as a later fix; it is recorded in the lane ledger.
