# Wave B plan §14.57 review, spec/plan mode (r1, bc0a7ee): FAIL

Reviewer: `reviewer`. Filed by the controller.

## Important
- **I-1. The step-3 `runners.yaml` write.**
  - (a) The file holds the slot times, `grace_minutes`, `device` and `scheduler: app`, not only the zone. Build it with `scaffold::runners_yaml` over step 2's plan, changing only the zone. Test 44 checks the bytes.
  - (b) Create-new at the school answer keeps the first school's zone after `change_school`. Fix: create the file at `school.apply`, or record that step 3 made it so `change_school` can remove it. Add a test 44 case.
- **I-2. The raw registrar file can ship before its privacy line (S7).** Gate the wave-B release on S7 merged and deployed. Fix the draft line: confirmed commitments do sync to the account.

## Answers
- F7e's `meets` is a single-line write through `write_literals` with `console_ctx`, so contract-reviewer is the right level, not contract-engineer.
- §7 has no cycle, and every size is within the cap.
- Trim the leftover option-B and F7d branches in the F8h-runner and F8b rows.

## Minor
- m-1: reuse `commitments::answer_literal` for `meets`, not a second emitter.
- m-2: a double-pressed step-10 confirm can stamp twice. Guard inside `confirm_step10` under the lock.
- m-3: hold F7e and F5d on Quinn's D5 signature. Move step 10's hours Edit into a separate F11c follow-up.
- m-4: the F3z row's refusal needs its condition (no IANA zone). Add a test 44 case where an IANA zone accepts an unlisted school.

---

## r2 (b3a53bf): PASS
All r1 findings are closed, and §7 has no cycle; every size is within the cap. Four minors were folded in at §14.57.13:
- n-1: F11c2 is on §8's checklist and in the merge order.
- n-2: the schedule-file wait applies only while the file is absent.
- n-3: F3z's place in the onboarding_jobs.rs chain.
- n-4: F7e's fixed `meets` message.
