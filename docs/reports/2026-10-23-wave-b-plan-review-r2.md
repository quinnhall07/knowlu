# Wave B plan review, round 2

Plan: `docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` (revised after r1).
Verdict: **FAIL** (0 critical, 6 important, 5 minor).

> Provenance: filed from the coordinator's message carrying the review content; the text below is
> that content, arranged under headings, with nothing added.

## Holds

- C1: write-once matches `journal::create_actor_file`; config never syncs (`ids::NOTE_FOLDERS`, 8
  note folders); the `none` preset is valid (`scaffold.rs` `campus_yaml`).
- I3: S3 test 7 and S4c present; `mail_session::forget` deletes only `mail-session-<id>`,
  `forget_all` only `mail-setup` and `mail-session-*`.
- I5, I6, I8 and the r1 minors are answered. I4 and I7 only partly (see M1, M2 and I6 below).

## Important

**I1. A failed `school.store` loses the LMS feed silently** (§4.2 step 4, F6b, F8c, Q-B2).
Today `create_vault_in` writes an unstorable feed into `ingest.yaml` (`onboarding.rs:744-773`;
R-C3'-exec-40, "never silently lost"). Under the plan the link lives only in memory and F6b pins
`ics_url ''`. Breaks: (a) the store fails on a blip after payment, `''` is written, the reveal has
no LMS work and nothing says why; (b) a quit after payment and before `school.store` resumes the
job pending with no link; re-capture needs step 3's window, and a school without `shared_sign_in`
prompts a sign-in later, uncovered. Fix: `config.ingest` writes the link when `school.store` isn't
done; Open Knowlu waits for `school.store` or that fallback; a test and a ledger row for each.

**I2. The answers `config.ingest` needs don't survive a quit** (§4.5, §4.9). `onboarding.json`
"answers" versus §4.5's field list (none; it forbids titles); `course_map` needs step 3's course
list; F6b needs step 8's confirmed mapping and the calendar answers. Quit at step 8 and resume →
an empty `course_map`, kept forever by write-once. Name where each answer is kept; F1 test: quit at
step 8, and F6b's file is byte-equal to the uninterrupted case.

**I3. Open Knowlu waits for the whole first slot, undoing the 3-minute cap** (§4.10, F1 test 9
bullet 2). `scheduler::CHILD_TIMEOUT` is 20 minutes per step (`scheduler.rs:28`); Finish later,
close and Sign out are held the same way. Fix: stop at an engine-step boundary and hand the rest of
the slot to the first-run slot (`is_first_run` is still true because `rank` hasn't run).

**I4. `config.ingest` can sit in the serial queue while step 10 asks for calendar proposals.**
`commitment_proposals` runs `commitments --json` synchronously against `config/ingest.yaml`
(`week.rs:54`, `cli.rs:977`); with step 9 absent, step 10 follows step 8; behind `mail.canary`
(up to 3 minutes), `mail.first_pass` or `coursework.discover`, step 10 shows no proposed meetings.
Fix: local config jobs run ahead of the serial queue; gate step 10 on `config.ingest` done; an F1
test.

**I5. An empty restore on a down network leaves duplicate course notes.** `restore_into` treats
network-down as empty (`onboarding.rs:790-797`); `school.apply` creates fresh notes, `slot.partial`'s
sync pulls the account's copies → two notes per course, with `course_map` on the new slugs. Fix:
`school.apply`'s course notes wait for the first successful sync or restore; a test, plus an F6b test
that `course_map` names the restored notes.

**I6. Still over the 80-line cap:** F2, F7, F8a and F11a (each M in one file). Split now.

## Minor

- **M1.** Q-B14's deny list lets the mail window (`INIT_SCRIPT`) open on other school hosts (Simple
  Syllabus, a second SIS) with their cookies: deny the school's own domain except the IdP host, or
  record it as an accepted risk for Quinn.
- **M2.** S3 test 3: "cookie host" must mean the URL passed to the jar query, not a cookie's
  `Domain`.
- **M3.** S3 test 2: don't set `KNOWLU_MAIL_DEBUG_PORT` in a test (process-wide, races, `set_var`
  is unsafe in edition 2024); pass it as a parameter.
- **M4.** §4.9/F6a: name the exclusive-create method (`hard_link`, or `MoveFileExW` without
  replace; `std::fs::rename` replaces on Windows); F6b gets F6a's partial-failure test.
- **M5.** §4.7 counts a `campus.yaml` `unitid` as step-3 proof, but a restored vault never has one;
  §4.10 clears `handover` at the first slot's end, which also clears a mail canary item the slot
  doesn't run.

## Passes

No single-user assumptions; only `account.rs` from the contract list (`contract-engineer`); the
`generate_handler!` recounts are covered.
