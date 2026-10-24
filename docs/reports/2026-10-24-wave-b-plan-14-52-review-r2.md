# Wave B plan §14.52, review r2

- Reviewer: `reviewer` (spec/plan mode). The controller filed this text, because the reviewer has no write tool.
- Target: O3 at 7f66ee0. Plan lines ~4564–5099, plus §4.2 step 5 (~648), §4.10's mail rows (~1199–1200) and §7.

**Verdict: FAIL.** Two Important findings, both introduced by the r1 revision.
- Closed in the text: I1, I2, I4, I5 and m1–m5.
- Partly closed: I3 and I6.

## Important

### R2-1. The `mail.connect` item clears at the first normal slot

Where: ruling 1 (~4644–4665), the §4.10 row (~1199) and test 40.

The item clears on `CanaryVerified` or `Disconnected`. F8s2 reads `Disconnected` as an `Ok` list with no row and no `pending_setups` entry. Two cases reach that state almost at once:

- **Setup never started.** `mail.setup` was pending at Open Knowlu, so no address was ever made. The first normal slot sees no row, reads `Disconnected`, and clears the item.
- **Setup interrupted.** At the next launch, `on_start`'s `clean_unfinished` (`mail/live.rs:195-199`) deletes the address and its record. The same thing happens.

In both cases the "Finish connecting your email" line lasts only minutes. That is the silent drop r1 I3 was meant to prevent.

**Fix:**
- `mail.connect` clears only on `CanaryVerified`, or when the student explicitly turns mail off or skips it in Settings (`turned_off`).
- It never clears just because no row exists.
- Test 40 covers both cases.

### R2-2. F8d's fix-round items have no open task

Where: the §7 F8d row and ruling 1 (~4596–4603, ~4669–4671).

The F8d row says these items land "in its fix round (built at `ad006ca`)". F8d is already merged into O3 (62f6c73, carrying ad006ca and cb2b2bf) without them. At HEAD:

- there is no `skip_mail`;
- there is no `MAIL_FIRST_PASS_LOOKS`;
- there is no `retry_at`;
- `mail_setup_start` has no onboarding-mode guard;
- `mail_batch_closed` only stores a flag (`onboarding_jobs.rs:650`).

Consequence: F2b2a's Skip calls `Queue::skip_mail()`, which does not exist and has no owner. F2b2a won't compile, and the Skip half of test 34 can't pass.

**Fix:** add a new task, F8d2 (`implementer`), that covers:
- the guard on `mail_setup_start` (`mail.rs`, about 6 lines);
- `skip_mail`;
- the first pass's one look, plus the batch wake;
- the 30-second `retry_at`.

Everything except the guard lives in `onboarding_jobs.rs` (about 30 lines). Place F8d2 in that file's task chain, add it to F2b2a's and F8w2's dependencies, and make the fix-round parts of test 34 its tests, written first.

## r1 findings, checked against the text

- **I1, closed.**
  - The worker records `JobContext { school, gen }` under the queue lock when it marks a school job running.
  - `SCHOOL_GEN` is bumped only while both locks are held.
  - Lock order is the queue lock, then `ANSWERS_LOCK`. Fenced writes take only `ANSWERS_LOCK`.
  - `inputs_ready` uses `load_answers`, which takes no lock (`onboarding_state.rs:406`).
  - One wording error: the plan says `on_sign_out_with` and `write_finish` take `ANSWERS_LOCK` alone "as today", but `on_sign_out_with` takes no lock today (`onboarding_state.rs:624-652`). Reword to "take at most `ANSWERS_LOCK`, never the queue lock".
- **I2, closed.** `write_capture_file_for_gen` covers the feed, the grades bundle and the option-A registrar file, and test 35 checks each one by name.
- **I3, partly closed.** The item's lifecycle is now named, and finish never waits on the mail jobs. The rule for clearing it is still wrong; see R2-1.
- **I4, closed.** F8b's registrar half and F8h's runner are held for Q7. Option A carries S7's privacy-page change and `PRIVACY_VERSION`.
- **I5, closed.** Rows with no file are `Failed("registrar_file_missing")`, with a fix sentence. Try again re-runs the capture.
- **I6, mostly closed.**
  - The task sizes agree with §7: F2b2a (~45), F2b2b (~75), F8b2 (45+35), F8h (~80), F8g (~78).
  - Remaining gap: F2b2a has no dependency on whoever builds `skip_mail`; see R2-2.
- **m1–m5, closed.** For m4: 623a5f8 is an ancestor of 7f66ee0. `tidied` is at `engine/src/onboarding.rs:499` and `live_by_source` at `:529`. Cite 623a5f8 as "F7a's merge, which carried F7c r4".

## Q7: can each option be executed as written?

- **Option A: yes.**
- **Option B: nearly.** Three gaps:
  - Under B, F8b still writes the raw response to `tmp\` for `--preview --registrar`, through the fenced helper, and deletes it after the preview. The plan doesn't say this.
  - F7d's "byte-identical notes and journal records" check needs either a fixed clock seam or the weaker claim "identical except `ts`".
  - B's §9 amendment needs a matching CLAUDE.md contract line, which is docs-keeper's.

## Minor

- **n1.** The `job_context` map's lifetime isn't defined.
  - Per job, school and generation are filled in at run time, and F8g sets the flags.
  - A Google job that is still pending when a slot ends misses that slot's flag, costing one slot.
  - Either say so, or set the flags on pending Google jobs too.
- **n2.** If the patient read moves to F4c, the read half of test 38 moves with it. The plan should say so.

## Holding

- No new command.
- No lock is held across a network call or a window.
- No vault write goes around `write`.
- Every test from 33 to 40 has an owner who writes it first.
- No dependency cycle in §7 once F8d2 is placed.
