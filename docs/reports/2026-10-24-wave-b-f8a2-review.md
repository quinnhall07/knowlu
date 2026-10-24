# Wave B F8a2 reviews (2026-10-24)

The crash-loop guard. The reviewer read each round; the controller filed this report.

## r1 (`94b2905..341e9a8`): FAIL
**Critical.** At `Queue::start`, a paused `slot.partial` was re-pushed with `JobRecord::default()`, which reset its count, whenever `school.courses` was waiting on an unsettled account. A process that aborts on every launch during the BeforeConfig chain would therefore never trip the guard.

**Minors.** A stale `REASON_JOB_CRASHED` comment, and a bytes test whose "old" file was written by the new code.

**Rulings.**
- Retry keys on the reason, not on the count.
- `Queue::start` uses `save_state` directly.
- A clean drain does not reset the pending counts (the plan counts "in a row").

## r2 (`341e9a8..b4c2ab0`): FAIL
The Critical is closed. `push_job` refuses to write over a `job_crashed` record. The minors are closed; the bytes test now uses a hand-written literal with sorted keys.

**Important.** That refusal also swallowed the student's Try again on `first.sync`. It did the same on `school.courses` while the account was unsettled. The indicator was left pending with no reason and no slot.

**Ruling.** The student's press is the one allowed writer.

## r3 (`b4c2ab0..496f3af`): PASS
- `Core::press` is the only path that can lift a pause: the record is removed, pushed again, and the count is set to `CRASH_LIMIT - 1`.
- Its three call sites are all inside `Queue::retry`, and the only caller of that is the `onboarding_retry` command.
- No other path can lift a pause: start, the chain pushes, `pay_no_rank`, `confirm_step10` and `enqueue` all go through `push_job`, which refuses.
- **Ruling (b) confirmed.** A slot paused again stays in the gate as Failed with `job_crashed` and "Try again", and in `handover_for` and the rail. Nothing is hidden.

## Owed elsewhere
- The gate's fix for a failed item is plain "Try again". F8g or F11 maps `job_crashed` to `JOB_CRASHED_FIX`.
- Jobs outside `SLOT_ITEMS` and `MAIL_ITEMS` that are paused appear on the rail only, not in the gate. This matches §4.6.
