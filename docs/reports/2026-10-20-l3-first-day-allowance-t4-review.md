# L3 first-day allowance: T4 review

Reviewer, 2026-10-20, run inside the wave-1 workflow. Filed by the controller.

PASS (no critical or important findings; one minor). This is my T4 review of lane L3 (first-day event allowance) for the T1, T2a, T2b, T3 and T3b changes, before any push.

Scope: the worktree compared with origin/main. T3 is commit 71a4790 and T3b is commit 1ed2398. T1, T2a and T2b are not committed yet: eight modified files plus the new cloud/supabase/migrations/20261020000200_event_allowance.sql. The ledger says the controller commits them. My PASS covers exactly that working tree; if any byte changes before the commit, run this review again. No contract-list file is in the diff (cloud_contract.rs was ruled off the list under Q-B). cloud/, engine/ and app/ have not changed on main since the cut at ba7eb0b, so the stamp 20261020000200 still holds against main. The worktree diff shows some L8 report and spec files changing, but those changes come from main having moved ahead, not from this lane.

Plan T4 checks:
- Opening test is "no row": yes. The SQL has no delete and no truncate. The conflict arm only lowers calls_left, and only while `until > now()` and `calls_left > 0`. There is no 120-day clause. A pin scans every migration for any delete or truncate of the table.
- Order: yes. The pipeline in judge_pipeline.ts checks rules, then the breaker, then withinBudget, then chargeOutcome. Only a "refused" answer on an `event` reaches chargeAllowance; an "error" answers capped and is logged byte-for-byte like a refusal (the test compares the logged rows). `charge` is now `chargeOutcome(...) === "charged"`, so it gives the same true/false answers as before (a four-case test). A live-store test drives a real capStore over a scripted Db and confirms that a charge_call 503 never calls charge_judge_allowance.
- Every CapStore implements the new methods: yes. That is capStore itself, the pipeline's Caps fake, ALWAYS, the gmail-read fake and EVAL_CAPS in run_eval.ts. mail-resolve's `charges.charge` belongs to a separate store and is not a CapStore.
- Refund routing: yes. `chargedTo` sends a "model failed" refund to refundAllowance or to refund, never both. A refusal, a truncation or an invalid answer refunds nothing. refundAllowance increments `failures` exactly as `refund` does, and a test mixes the two and trips the breaker.
- Privileges: yes. RLS is on with no policy, all access is revoked from anon and authenticated, both functions revoke execute from public, anon and authenticated and grant it to service_role only, and the account foreign key has on delete cascade. The parsed count goes from 40 to 42, and the cap-store marker list now names both new functions.
- Static pins: the exact pin on the conflict arm's `set ... where` would catch a renewing allowance (an extra `or`, or a reset of until). The pipeline tests would catch an allowance-first order (`charged` before `allowanceCharged`, and the ceiling test leaving both empty). The 14-day and 2000 mirror (spec test [11]) reads the SQL text. Minor finding below.
- T3's `left`: verified in judge_roster. The capped item breaks before `processed` is counted, so left = left_for_cap + (batch - processed). Mid-batch 5/k=2 gives "2 judged, 3 left"; the first item gives "0 judged, 5 left". The budget break and the cap break are exclusive exits, and T18c's 862 + 70 = 932 holds. Failed items still get their own `not judged` line and leave `left` at 0.
- T3b: no wire change. A 401, 402 or 403 is still global (`fatal` Cell checked first in `call`, and the a_402_still_stops_every_path test covers both directions). The capped state is now kept per path. BATCH_PATHS is `/judge-task` only, which matches the finding that enrich_with's batch calls only Model::judge. fatal() names the cap only for that path. enrich.rs and judge.rs are not in the diff.
- Rules: no single-user assumption (acct-1, example.edu and example.invalid only). No fixture moved, and no Tauri command or app file is touched.
- Privacy: judge_allowances holds account_id, opened_at, until and two counters. That is metering, not a new data class. Neither the privacy page nor PRIVACY_VERSION is in the diff.
- Gates (from the ledger, not re-run by me): cargo test --workspace 2837/0/4 with only the .rsrc warning; deno cloud+eval 1121/0, check and lint clean. Files are LF, and the migration contains no CRLF and never uses the word "models", so it will not trip the eval gate.

Minor (does not block):
1. migrations_test.ts, charge_judge_allowance_is_one_statement_on_a_unique_key: the loop `new RegExp(`\b${word}\b`)` sits inside a template literal, so `\b` is a backspace character and the checks for no delete, select, if or found can never fail. Today the statement count, the exact conflict-arm text and the exact return line still reject every regression this loop was meant to catch, so the SQL is protected. Fix: doubled backslashes or String.raw, then confirm it goes red on a scratch copy of the SQL containing `select`.

The SQL is checked only by text pins (Q-C). It is first actually run by T5's dry run and staging push, which is where any SQL error would appear.

I can't tell you whether W8 is being worked on: this review covers only L3 T4. The controller should answer that question and pick up W8 if no one has it.

## Controller follow-up (2026-10-20)

- The minor is fixed: the controller doubled the backslashes, so the loop's word boundaries are real again, and the test stays green on the committed SQL.
- T1, T2a and T2b were then committed by the controller with no byte changed (562e82c, 4a07e3e).
- All five commits were rebased onto origin/main 4792629 (d12530b, b2e6999, ea954e6, 61e2b41).
- Deno 1121/0 after the rebase.
