# Wave B F1 review, round 3 (2026-10-24)

Reviewer, pre-build, on `git diff 16b02bd..1a97866`, against r2, the plan at ae504be and the engine's `by_course` (904a92c's encoding, feat/o3-f7a). Filed by the controller.

**Verdict: FAIL.** The r2 items are closed. Three new pins disagree with the plan or the engine. No contract-list file is in the diff.

## Important
1. **The course key is the shared human code, not the LMS key** (`app/src/onboarding_state.rs:150`, `app/tests/onboarding_state.rs:898`, `app/tests/onboarding_jobs.rs:1629-1639`). `key()` goes code, then slug, then label, and `CourseAnswer.code` holds the human code ("KNT 220"). F7a (plan :2277-2279) and §14.45.2 key on the LMS course code, so two shells of one course are two keys. The engine reads `code` as that LMS key.
   - Failure: "KNT 220" exists from spring (`_0990_1`) and from fall (`_1002_1`). `{"remove": "KNT 220"}` matches both shells.
   - Failure: `preview_courses` sends no `slug`. For a row with no code, the engine falls back to the label ("Lab") but the app's key is "lab-section", so the bucket drops out of the totals. The test at :1101 hides this by building the bucket by hand.
   - Fix: make the key `lms_key`, then slug, then label. `preview_courses` sends that key as the engine's `code` and also sends `slug`. Add a test with two rows that share a human code but have different `lms_key`s.
2. **`Bucket.counts` can't hold the engine's nulls** (`onboarding_state.rs:170`, `step3_totals` :224). The engine writes `null` when an input is missing: `turned_in` without grades, `class_times` without a registrar.
   - Failure: on the paste fallback, parsing into `BTreeMap<String, i64>` fails, so the buckets are never saved. Even if it parsed, the proof would show 0 where it should show `null`.
   - Fix: make the counts `Option<i64>`. A total is `null` when every bucket's value is null. Add a test that feeds an engine-shaped `by_course` with nulls through serde into `step3_totals`.
3. **`{"buckets"}` and `{"pairs"}` are page answers** (`onboarding_state.rs:230-231`; tests :979, :1115; `onboarding_jobs.rs:1606`, :1734). The plan sources both from F8b: counts from `school.preview` (§4.5 :913), and pairs built once at capture (§14.46.3). As page answers, they create a second source of pairs.
   - Failure: a screen posts re-derived pairs, `course_exclude` carries a fragment the capture never built, and F6c drops items that should have been imported.
   - Fix: save both through Rust seams that F8b calls, such as `save_preview(dir, by_course)` and `save_pairs(dir, ..)`. Assert that `onboarding_answer_inner` refuses both shapes with `ok: false`.

## Minor (ride along with the fix)
- **Re-capture keeps old fragments** (`onboarding_state.rs:936-945`). Pin that a re-captured row's non-empty fragments win, so a new gradebook pin isn't lost.
- **Untested triggers.** No answer is pinned to enqueue `school.capture` (step 3, "I'm signed in") or step 6's `google.gmail_first_read`/`mail.canary`. Name both triggers, or say they aren't answers.
- **A stale comment** (`onboarding_state.rs:862`): it shows `|`, but bucket names are joined with `,`.
- **The wrong id as a pin.** The fixtures use a course id (`_1001_1`) as a pin. Pins are gradebook column ids; use a column id in the shared-fragment test.
- **For the planner:** F1 test 12 (:1771-1774) should read "byte-equal less the shared fragments".

## The test-writer's rulings
- **Pairs as plain fragment lists:** acceptable.
- **The enqueue map:** consistent with §4.2 and §4.9 for the paste path (see the Minor above for the window path).
- **The answer shapes:** `capture`, `remove` and `label` are acceptable. `buckets` and `pairs` are not (Important 3).

## Safety: pass
The new `todo!`s have no caller in `app/src` and are in no `generate_handler!`. All data is synthetic. Nothing new touches the Credential Manager or the network.
