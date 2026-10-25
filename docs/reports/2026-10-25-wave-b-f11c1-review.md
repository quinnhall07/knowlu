# Wave B F11c1 review: step 8's console wrappers and step 10's confirmed answer

Reviewer: `contract-reviewer` (read-only; the controller filed this report). Round 1 covered
`579826a..7618ec2` and failed. Round 2 covered the fix, `7618ec2..e985f87`, and passed. No file on the
contract list is touched.

## Round 1: FAIL
1. **Important:** `storeLogins` had a `!vault` guard reading `w8.status.vault`. `onboarding_status`
   never carries a vault (§14.57.1), so every step-8 Sign in stopped at "can't find your setup
   folder", and no pin ran `storeLogins`. *Fixed in e985f87:* the guard is removed, and the pin
   asserts that neither `storeLogins` nor `discoverSite` mentions `vault`.
2. **Minor:** if the step-10 `onboarding_answer` call threw, the page said "couldn't save your
   week", though the week was saved. *Fixed:* the call now has its own handler, "Your week is saved,
   but Knowlu couldn't move on. Try again."
3. **Minor:** the old path-taking names were still allowed. *Fixed:* they moved to `BANNED`.

What held in round 1:
- no path or vault is sent, and the secret stays inside the store handler;
- 7a9e1e9 is rename-only;
- the seam keeps the exact allowlist, the envelope and the refusal-first order;
- test 43 is stronger and never reaches Credential Manager;
- step 10's answer is sent only after a successful confirm.

## Round 2: PASS, with no new findings
Counts: onboarding_static 57 passed / 13 failed (the same placeholders), static_assets 151,
onboarding 83, onboarding_jobs 145 passed / 8 failed (placeholders).
