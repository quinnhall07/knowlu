# Wave B F7c review, round 1 (2026-10-24)

Reviewer, on `git diff 47e7888..6a64eea` (`feat/o3-f7a`) against the wave B plan at cfc89ec. Filed by the controller; the reviewer has no write tool.

**Verdict: FAIL.** Four Important findings, no Critical. Nothing on the contract list is touched, and no frozen reference or `surface-today-*.json` changed.

## What holds
- **It never writes.** `--tidied` only reads: `read_dir`, `read_to_string`, `JsonlLedger::new` and `read`. Only `append` creates a directory, so no `state/` file is made.
- **Key order and determinism.** The output uses BTreeMaps, sorted paths and `ledger::dumps_value`.
- **CRLF.** `course_texts` splits `e.raw` after `unfold` has normalised it, and `split_frontmatter` trims `---\r`. Files are LF and `diff --check` is clean.
- **Live means `status: active`, with a missing status read as active.** This matches `ingest.rs:822` and `models.rs:311`.
- **The edited key-order test** is not a frozen reference and still checks all six top-level keys.
- No `#[ignore]` changes and no single-user assumptions.

## Important
1. **`by_course` has no `unmatched` bucket** (`onboarding.rs:244`). The plan (F7a, line 2209) says an unmatched item counts in the totals only, under `unmatched`, so the buckets sum to the totals.
   - Failure: of 4 upcoming items, 1 has no course text and no gradebook column. Kept-row totals then show 3, while ingest writes 4 (`ingest.rs:883` keeps course-less items).
2. **Buckets keyed by slug can collide** (`onboarding.rs:80`). A past-term shell and the current shell of one course share a slug.
   - Failure: removing the old shell also drops the live course's counts. If both are kept, the counts are doubled.
   - Fix: key by the LMS course key (`code`). Fall back to slug, then label, only when `code` is empty.
3. **A second course-attribution rule that can drift from ingest's** (`onboarding.rs:94`, `:283`). `ingest.rs:258-264` requires one shared rule (`match_course_fields`). This diff adds prefix-only matching, a bundle fallback and course-order precedence.
   - Failure A: "Quiz covering SYN 120" is unmatched in the proof, but is `course: syn-120` in the vault.
   - Failure B: with courses [SYN 120, SYN 110], "SYN 120 review [SYN-110-003-FALL2026]" goes to SYN 120, because an earlier course's prefix beats a later course's exact bracket.
   - Fix: build (fragment, key) pairs the way F8b will write `course_map`, call `ingest::match_course`, and pin gradebook ids the way ingest's uid pass does. That pin is the bundle fallback's legitimate role.
4. **`effort_estimated` misses Gmail tasks** (`onboarding.rs:368`). Gmail tasks are a `create` by `agent:knowlu.gmail` carrying the cloud's judged `effort_hours` with `effort_source: inferred` (`enrich.rs:547-558`, `:919`). Only `op: set` counts.
   - Failure: 6 judged Gmail tasks show as 0 estimated.
   - Fix: also count an agent `create` whose note has `effort_source: inferred`, and add a test for it.

## The implementer's calls
- **Bundle fallback:** only as ingest's uid pin (Important 3).
- **Journal join by path or id:** acceptable, but join by path only when the note has no `id` (Minor 2).
- **Live definition:** accepted.
- **Counting only `set` toward effort:** wrong for Gmail (Important 4).
- **Key-order test change:** accepted.

## Minor
1. **A real-looking course code.** The comment at `onboarding.rs:844` names a code that looks like a real course. Replace it with a synthetic one. Codes from before this diff at `ingest.rs:1872` need a separate sweep.
2. **Path-join false positive** (`:405`). A task archived and then re-created at the same path inherits the old agent `set`.
3. **CATEGORIES splitting** (`:115`) runs after `unescape`, so an escaped `\,` splits one category in two.
4. **Missing tests:** a CRLF or folded-line ICS for `by_course`, and a CRLF note for `--tidied`.
5. **`live_by_source` vs `load_tasks`.** It counts notes that `load_tasks` would reject, so the rail can show more live tasks than the console.
6. **Docs.** `engine-commands.md` has no `onboarding` entry (docs-keeper, in a later task).
7. **Sizing.** F7a's `by_course`, about 120 lines, shipped inside F7c's commit.

## Outside this diff (for the planner)
R-C1-51 keeps a removed course out of `course_map`, but ingest still imports that course's feed items as course-less tasks. A proof total over kept rows will under-count the vault unless F8b or ingest also drops those items.
