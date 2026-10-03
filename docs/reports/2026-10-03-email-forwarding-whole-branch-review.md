# Email forwarding: Checkpoint D whole-branch report

- Date: 2026-10-03
- Branch: `email-forwarding`, HEAD 33be65c
- Reviewers: `reviewer` (Opus, high) for the non-contract half, `contract-reviewer` (Opus, xhigh) for the contract half
- Source of round-by-round facts: `.superpowers/sdd/2026-09-30-email-forwarding/progress.md` (rows dated 2026-10-03)
- Verdict: **Checkpoint D is NOT closed.** It closes when Quinn rules PQ13 (and its fix lands if the ruling is (b)).
  Everything else is fixed or recorded.

## Gates

- T17 integration, 156ee3e (merges 26dc629, a692e80, 0fb1dd7): all gates green. Workspace then 2658 pass / 0 fail /
  4 ignored (the four by design); cloud Deno 1028; mail scripts 72.
- Contract-list diff against main: `engine/src/approvals.rs` and `app/src/account.rs` only, as plan section 8 states.
  `engine/tests/fixtures/**` untouched; the oracle, sync and entitlement tests unchanged.
- Handler lists (counted by script in T13b, 65be1c5): 70 console, 29 wizard.
- Workspace now, after the Checkpoint D fixes: 2665 pass / 0 fail / 4 ignored.

## Contract half (`contract-reviewer`)

First read: 0 Critical, 2 Important, 2 Minor.

- Important: `account.rs` left mail windows open on account deletion and on sign-out. Fixed in cb44eee (the stop is
  set, the mail window is taken within `MAIL_STOP_BOUND` of 60 s, and the wipe's close step waits the same way).
- Important: PQ13, open (below).
- Minor: the refusal sentence, and the Gmail sender gate. Fixed in dc37313 and cb44eee.
- Re-check: 0 Critical, 0 Important, 3 Minor (the three are the ones fixed below in d9868b6, f4ddf41 and ca17912).

## Non-contract half (`reviewer`)

First read: 0 Critical, 1 Important, 4 Minor; approve with conditions. The important item is PQ13.

- Minors, all fixed: mail step skip naming, the no-session reason, the slot waiting for a stopped window to go,
  the refusal overcount (a refused deletion now counts only the forwarding it turned off), and a weakened static
  assertion (the Gmail label's no-sender check is back beside the `mailCardLine` count).
- Fix commits: d9868b6, f4ddf41, ca17912. Each code fix was seen red on behaviour first.
- Re-check: 0 Critical, 0 Important, 4 Minor (pass).
- The D18 release guard is a Quinn question (Q5, below).

## UI walks

- `settings-check.py` and `wizard-check.py` pass with Playwright.
- Settings check 6c failed in the check script, not the console: `Date.now` was not advanced. Fixed in 33be65c
  (no `console.js` change).

## Open for Quinn

1. **PQ13, the Checkpoint D blocker.** A live `new_item` carrying a new due date for a task that a pending mail card
   already proposes plans `Skip(AlreadyApplied)`, and the date is lost with no card and no notice. The test
   `a_live_new_item_matching_a_pending_mail_task_card_files_nothing` pins it. *Recommend (b), due-aware:* a new due
   files its own card. The fix is T5.6's two files (`mailreconcile/{resolve,rules}.rs`) and the five cases of that
   one test, by `contract-engineer` on `email-engine`, then `email-engine` merges into `email-app` again before
   T18. No live mail pass runs before T18, so nothing has been lost on a student's vault.
2. **PQ14.** A resolve claim that is never answered is held forever. *Recommend:* give up after 3 slots and show the
   unmatched notice.
3. **PQ10.** Templates built from real captured notifications; Checkpoint B's re-read follows.
4. **D18 release guard (Q5).** *Recommend* a `release.yml` check that blocks a release before bump #1.
5. **Q2, controller default.** *Recommend:* keep T15.0's optional surface fields (`from`, `mail_date`, `evidence`;
   keys absent on non-mail cards, the surface oracle references byte-identical), with a dated marker on spec 6.1.
6. **S6.** The pace constants are provisional.
7. **PQ3.** The digest (T16).

## Checklist from plan Checkpoint D

Confirmed by the facts above: the contract-list diff and the fixtures. The other grep items on the plan's list
(`site/`, `PRIVACY_VERSION`, `commitments/`, telemetry, OS input, literals, the retired working title) are not
restated here because no source for them was given to this report; the reviewers' reads own them.

## Next

Quinn rules PQ13. If (b), the fix lands on `email-engine`, merges into `email-app`, and Checkpoint D is re-run on
that diff only. If (a), the plan records the loss as accepted and Checkpoint D closes. T18 follows either way.
