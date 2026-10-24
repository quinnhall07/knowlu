# Wave B F2b2a review (bc12139 vs 4c6a8f2): FAIL

Reviewer: `reviewer`. The controller filed this report. No contract-list file is touched. Line endings are LF.

## Important

### 1. A repeated answer or proceed re-runs jobs that already finished

**What breaks:** §14.52.3 requires that a repeat of any proceed changes nothing. Today it re-runs the job.

**How:** `Core::finish` removes a finished job from `order`. `push_job` checks only `order`, so a repeat writes a fresh pending record over the finished one.

**Failure scenarios:**
- **Step 5 (email):** after setup finished, the student goes Back and answers again. A second full setup starts: the provider window opens, `replace` is asked again, and the 180 s canary runs again.
- **Steps 3 and 8:** these re-run `school.courses`, `registrar.confirm` (not shown to be idempotent), `config.ingest` and `slot.partial`.
- **Open Knowlu:** `finish_ready` flickers false while the re-run is pending.
- **Skip, then answer:** the mail jobs come back to pending while `skips` still holds `(5, "mail")`.

**Test gap:** test 33 covers repeats only while a job is queued or running.

**Controller ruling:**
- Add `Queue::enqueue_once`, which leaves a job alone if it is done, failed, skipped, queued or running.
- Failed stays failed. Only Try again re-runs it.
- After a step-5 skip, an answer saves but starts nothing.

## Minor

### 2. TDD evidence

The new tests could fail only on compilation, never on their assertions. The fix round shows each one failing on behaviour first.

## Checked and holding

- `school_apply_may_run` is exactly §14.52.3's confirmed-school fact.
- No TOCTOU that matters: `school.apply` re-checks under the queue lock, and F8b2's `set_school` is one lock.
- No production caller uses the wrappers that pass `false`.
- Lock order holds: the snapshot drops the queue lock first, and each enqueue comes after `ANSWERS_LOCK` is released.
- The step-5 shape is exact, and its save goes through `update_answers_unless`.
- A proceed is exactly `{"proceed": true}`.
- `skip_and_stop_mail` is called only on a successful step-5 skip, and calling it again is safe.
- No single-user assumptions, and no frozen-reference or privacy issues.
- Until F8b2 lands, step 3's proceed is always refused. This follows from the dependency order and is not a defect.

---

# Review r2 (e1da439 vs bc12139): FAIL

**r1 Important 1 is closed for the step-5 answer and the three proceeds.** `enqueue_once` leaves alone any job that is done, failed or skipped, under the queue lock.

## New Important findings

**A. A re-capture no longer refreshes the preview.**
- `(3, "capture")` now asks for `school.preview` through `enqueue_once`, so a done preview is never rebuilt for the new list.
- The proof totals and pairs stay those of the first list.
- **Ruling:** `school.preview` stays on `enqueue`.

**B. A skip made before any mail job existed doesn't hold.**
- `skip_mail` turns only existing jobs into skipped ones, so a later answer starts `mail.setup` anyway.
- **Ruling:** `skip_mail` writes `Skipped` records for mail jobs that have none.

**C. `slot.partial`'s exemption needs a guard.**
- A repeat while a slot is queued is harmless: `no_rank_owed` is one flag.
- After the slot is done, every repeat re-runs coursework (vendor logins) and judge (cloud spend) on no new input.
- **Ruling:** `enqueue_once` returns a bool, and a proceed asks for the slot only if it newly queued another job.

## TDD
By reading the code, each of the four new tests would fail on its assertion against a stub that calls `enqueue`. No red run was recorded, so the fix report must state one.

---

# Review r3 (88f675f vs e1da439): FAIL

**A is closed.** `answer_enqueuer` is rebuilt fresh on every call, and the preview goes back to plain `enqueue`.

**B and C are closed while the app keeps running.** Both reopen after a sign-out rewind followed by a restart:
- `on_sign_out_with` resets every record to `pending` and keeps `skips`.
- `Queue::start` then queues every pending record.

What that does to each:
- **B:** skipped mail jobs come back as queued, so `mail.setup` starts with no step-5 answer.
- **C:** the redone proceeds find their jobs already queued and ask for no slot. The restored `slot.partial` then runs early as `BeforeConfig`, and no `NoRank` slot follows `config.ingest`.

The rewind guard test resets records inside a live queue. Production never reaches that state, so the test doesn't model the real path.

**Recommended fix:** the rewind clears `s.jobs` ("a step-2 file is a fresh install's"). This changes F2e's rule, so it needs a ruling, and the ruling must also say whether `skips` outlives a sign-out.

**For F2b2b:** the not-connected item must exclude `skipped` as well as `done`, with or without job history.

**Controller ruling:**
- The rewind clears both `s.jobs` and `skips`. A step-2 file is a fresh install's, and the student answers steps 3 to 10 again.
- Test 17's "every job pending" assertion becomes "no job records". The plan records why.
- This round is escalated to Opus, per CLAUDE.md, after three failed reviews.

---

# Review r4 (895bcc5 vs 88f675f): PASS

**B and C are closed on the real sign-out-and-relaunch path.** The rewound file now matches a fresh install. The new helpers model the production sequence: `onboarding_sign_out_inner`, dropping the queue, `resume` and a new `Queue::start`.

**The changed test-17 assertions follow from the ruling** and are stricter, with guards that make them bite. The no-answers test now checks the real wake path and also covers `registrar.confirm`.

**Nothing else relied on the rewound job records.** Checked: `finish_refusal` (fails closed), the gate, `handover_for`, `launch_mode`, `resume`, `Queue::start`, the rail and `retry`.

**Final pass over F2b2a:** every earlier finding is closed. The red runs are recorded in the commit messages, the lock order is unchanged and `git diff --check` is clean.

**Plan gap (not this diff's; for the planner).**
- **The gap:** once `advance_after_school` has a caller, an all-proven student skips steps 7 and 8. Their jobs are then never queued, and `finish_refusal` reads "still saving" for ever.
- **Fix:** the advance asks for the passed steps' jobs.
- **Test:** add an all-proven end-to-end test that reaches `finish_ready`.
