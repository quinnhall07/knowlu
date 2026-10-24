# Wave B F1 review, round 5 (2026-10-24)

Reviewer, pre-build, on `git diff a22d07b..a2383f9`, against the plan at a22d07b and the engine contract `feat/o3-f7a` at adfad36. Filed by the controller.

**Verdict: FAIL.** All r4 items are closed, nothing from r1-r4 has regressed, and safety passes. One new Important finding. No contract-list file is in the diff.

## r4 items: closed
- **I1:** `captured_label` is set by `recapture`, and the value the page sends is overwritten. `key()` is the LMS key, then the slug, then `captured_label`, each trimmed. This matches the engine.
- **I2:** test 14 creates one note per course, and the case where the first shell was removed is still covered.
- **Minors:** `save_preview` replaces rather than merges; no buckets gives null counts; a non-object is an `Err` with the file untouched; the pairs check reads what the fake received; `key()` trims; `[]` pairs are allowed only for a bare row.

## The new stubs
`captured_label` round-trips. `run_school_preview` is reachable only from its test, so it adds no handler and no warning.

## Important
1. **The rule for "same course" is unstated, and a row with no code is unpinned** (`app/src/onboarding_jobs.rs:184-187`). Test 14 needs a second shell to reuse the first shell's note, but the doc doesn't say how the two are recognised as the same course.
   - Failure: two typed rows, "Lab" and "Seminar", both have `code: ""`. Seminar reuses Lab's slug and gets no note of its own.
   - Fix: write the rule into the doc. A kept row whose trimmed, non-empty human code equals an earlier kept row's code reuses that row's slug, with no create. An empty code never matches. Test it with two empty-code rows: two creates, two slugs.

## Minor (fix before F2)
1. **`run_school_preview`'s failure outcomes** are unstated (`onboarding_jobs.rs:220-224`). An engine `Err`, or a `save_preview` `Err` (including `by_course: null`), should be `Failed` and leave the buckets unchanged.
2. **The order inside `recapture`** (`onboarding_state.rs:240-243`) is unpinned. `captured_label` must be overwritten before the key match that carries fragments over. Say so in the doc.

## Safety: pass
