# p3-registrar: contract review of the registrar fixture rewrite (2026-10-20)

**Reviewer:** `contract-reviewer` (Opus, xhigh). **Diff:** `engine/tests/fixtures/registrar/banner-ua-registration.json`,
blob `016f91a` -> `b419ca9`, committed alone as `871ed13`. **Verdict: PASS** (0 critical, 0 important, 3 minor).

## Checked

1. **No real student data (rule 1).** Every new value is invented or a placeholder: CRNs 40001-40006, titles and
   subjects starting "Invented", buildings "Invented Hall", "Pretend Library", "Make-Believe Studio", faculty
   `INV000001`-`INV000007` at `@example.invalid`, status "IR". Row and meeting counts differ from the R0 capture
   (6 rows and 8 meetings against 8 and 10). Course codes, term and dates were already in the old blob (plan review R1).
2. **Byte contract.** `-text`, `i/lf w/lf` on both blobs, no CR, no BOM, trailing newline kept, `git diff --check` clean.
3. **Shape and consumers.** `data` holds the six recorded keys and follows the phase 3 spec's key tree
   (`docs/specs/2026-09-26-commitment-model-phase3-design.md:125-170`). Traced through `parse_banner_with` and
   `read_meetings`: kept 4, no time 1, midnight 1, exam 1, status 0, matching `registrar.rs`'s tests,
   `engine/tests/commitments_registrar.rs`, `apply_support.rs`, `cli.rs` (40006) and `app/tests/registrar.rs`.
   No code still reads the old shape.
4. **Frozen set unchanged.** `git diff --exit-code origin/main -- engine/tests/fixtures ':!engine/tests/fixtures/registrar'`
   exits 0 (origin/main `cc8b162`).

## Minor findings (carried into the lane)

- **M1. P7's rule has no test.** Instructor names, emails and `properties` must never be read, logged or put in a
  warning or error; nothing asserts it. *Failing input:* a later `place()` fallback to `instructorNames[0]` carries
  "Invented Instructor Five" into `where` with every test green. *Fix:* `test-writer` adds a test that runs the
  fixture through `parse_banner` and `commitments --registrar` and asserts no series field, warning, error or stdout
  contains "Invented Instructor", "example.invalid" or "INV00000" (B4 follow-up or B7).
- **M2. `statusIndicator: "IR"` on every row.** When B9a sets UA's registered codes in `registrar.rs:29`, a set without
  "IR" makes `parse_banner(&fixture(), "ua")` return Err and turns most consumers red. *Fix:* an explicit B9a step,
  either the fixture's code moves to a recorded one (contract-engineer, then contract-reviewer) or those tests call
  `parse_banner_with(…, &[])`.
- **M3. Two values not marked invented:** the final's Tue 12/08/2026 08:00-10:30 and `maxHours: "19"`. School-wide data
  on an invented CRN, not student data, so nothing leaks. *Fix:* contract-engineer confirms neither came from the R0
  capture, and moves them if either did. No test asserts either value.

## Note

Commits `4bb6f94` through `fb48bb5` stay red in history (no rebase on a pushed lane branch); `871ed13` is the first
green candidate, pending the workspace gate on a clean checkout.
