# Wave B F11c review r1 (3652508 vs 44caf46): FAIL

Reviewer: `reviewer`. Filed by the controller (the reviewer has no write tool). No file on the contract list is in the diff.

I-1 is F11c's own and decides the verdict. I-2 to I-4 block steps 8 and 10 end to end, but they belong to other tasks; the planner names their owners.

## Important
- **I-1 (F11c): class-time rows on step 10 can't be edited.** `loadSet10` renders registrar rows as a plain list. The plan says they "show as set, each with Edit" (plan:703), and the fidelity row "editable on step 10" names F11c (plan:3292).
  - Scenario: the registrar gives CS 100 at the wrong hour, so the student can't fix it, and "Confirm my week" keeps it.
  - Fix: an Edit control per row, through the console's existing week commands.
- **I-2 (cross-task): `vault` and `courses` aren't in `onboarding_status`.** `status_inner`/`status_for` (onboarding_state.rs ~1110-1141) return neither.
  - Scenario: every Sign in stops at "Knowlu can't find your setup folder. Close setup and open it again." The sentence is also untrue, because reopening changes nothing. The course choice is empty.
  - Recommended fix: add `courses: [{key, code, label}]` from `Answers::kept_courses()`. Then either add `vault`, or add console-side wrappers that read the vault from `ConsoleState`, so the page never carries a path.
- **I-3 (cross-task): nothing calls `Queue::confirm_step10` outside tests** (onboarding_jobs.rs:600). `onboarding_answer` refuses any step-10 shape.
  - Scenario: Confirm succeeds, but `step10_confirmed_at` is never stamped and `slot.first_full` never runs. So `finish_ready` never turns true.
  - Recommended fix: add a `(10, "confirmed")` arm in onboarding_state.rs that calls `confirm_step10(now)`. The page then sends that answer after `commitments_confirm` succeeds and before `OB.show(11)`.
- **I-4 (cross-task): the row's course key is used as a slug.** onboarding_jobs.rs:996 passes `r.course_slug` into `books` without looking it up in `answers.slugs`.
  - Scenario: `ingest.yaml` maps a book to an LMS key such as `_12345_1`, so its assignments land under a course with no note.
  - Fix: translate through `answers.slugs.get(key)` in the config.ingest handoff.

## Minor
- **M1:** `loadSet10` runs once. Reload it in `onStatus10` when `registrar.confirm` turns done.
- **M2:** Confirm is locked until proposals are asked for or ingest fails. Plan:705-707 says class times "can be confirmed meanwhile".
- **M3:** while one site's discovery runs, the other site's Sign in does nothing and says nothing. Disable it in `paintNext`.
- **M4:** an empty-field message redraws the form and wipes the typed username. Write the message without redrawing.
- **M5:** a proposal with no `level` shows Hard selected, but `confirm10` falls back to soft. Default the select to soft.
- **M6:** the onStatus pin checks only the `onStatus` bodies, not the functions they call (it holds today).

## Checked, nothing found
- Credentials:
  - Read into locals in `storeLogins` only and passed straight to `invoke`.
  - Never logged, kept in `S`/`w8` or rendered.
  - Fields are cleared in every arm and on leave.
- Next and Skip are disabled during discovery. Only "Not a class" builds `zybooks_ignore`, and no slug is derived on the page.
- The panel seam holds, and `OB.clearError()` is in all five answering handlers.
- Step 10's sync-running refusal comes through `env.error` with Try again.
- No single-user assumptions. The only vault write is `commitments_confirm`, through the engine's `--confirm`.
- Tests are additions only, and no F11t assertion was edited. LF endings, and `git diff --check` is clean.

---

## r2 (1cb8eb4 vs 3652508): FAIL

M1 to M6 are closed, and the no-folder sentence no longer gives false advice. Hours are still not editable; the planner is ruling on that.

- **I-5 (new):** the kind menu in `editRowHtml` offers six of the engine's nine kinds. It leaves out `event`, `exam` and `task-block`.
  - Scenario: for an `exam` row, the menu shows "Class". The student saves a change to the level only, and `set_fields` also writes `kind: class`.
  - Fix: show the row's own kind as the selected option, or send only the fields the student changed. The console's `KIND_NAMES` may share this bug; it is to be checked.
- **I-6:** `set_fields` was added to the page's allowed list without a plan ruling. It is not an assertion edit. The planner needs to name `set_fields` (kind and level only) for step 10 at plan:703.
- **Still open from r1:** I-2, I-3 and I-4, all cross-task, with the planner.

## r3 (184783e vs 1cb8eb4): PASS

- I-5 is fixed: a kind with no label stays selected, and Save sends only what changed.
- The console was checked and does not have the bug (console.js:893-894).
- N-1 (an empty kind fell back to "Class") was fixed by the controller in 2154c0d.
- I-6 is ruled in plan §14.57, which names `set_fields` for step 10.
- I-2 to I-4 are assigned to F2b3, F5c and F8c2.
- Controller run: onboarding_static 42 pass / 13 fail (all later tasks); static_assets 149/149.
