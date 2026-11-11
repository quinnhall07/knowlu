PASS (Spec: PASS, Quality: PASS)

# Review: fix/date-comment-due (`git diff 3c48bde6 173ac640`)

Scope: one commit (173ac640), two files, `cloud/supabase/functions/_shared/judge_due.ts` (+20/-2) and
`judge_due_test.ts` (+140). No contract-list file, no engine or app file. This is Small-lane work.

The fix is correct and does not regress anything. I found no blocking defect. There is one Important
problem in the same function, but it is older than this diff and should be a follow-up. It is listed
first because the new test's name implies it is already covered.

## Findings

### 1. Important (exists on base, not introduced here; follow-up): the RFC 5322 pattern takes quadratic time on a header with no zone
- **Location:** `judge_due.ts:139-140` (the `rfc` pattern).
- **Cause:** after the time, `\s*([+-]\d{4}|[A-Za-z]{1,5})?\s*$` puts two `\s*` next to each other
  around an optional group. Give it a header with no zone, a long run of whitespace and a tail that
  doesn't match, and every way of splitting the whitespace between the two `\s*` gets tried.
- **Failing input:** `"Tue, 10 Nov 2026 09:00:00" + " ".repeat(40000) + "?"`, or with `"(x"` as the tail.
  - Expected: `parseDateLine` returns `null` in about a millisecond.
  - Actual: it returns `null` after 2224 ms (1853 ms with the `"(x"` tail).
  - The `rfc` pattern alone takes 106 / 334 / 1332 ms at 10k / 20k / 40k spaces, so the time grows
    with the square of the length.
- **Same on base:** base runs the same `rfc.exec(s)` on the same `s`, so this diff neither adds the
  problem nor makes it worse. A header whose trailing comment comes after the whitespace now parses
  quickly instead of going quadratic.
- **Reachable in production:** `mail-process` reads a Date header the sender controls and passes it to
  `resolveDue` through `validateMailClaims` → `mailDue` whenever a claim carries a due. The sender
  also writes the body that triggers such a claim.
- **Why it matters for this review:** the new test's second hostile line puts a `-0500` zone in front
  of the whitespace, and that zone sidesteps this path.
- **Suggested fix (separate Small change):**
  - refuse a Date line longer than about 256 characters before any pattern runs (a real header is
    under 100); or
  - rewrite the tail as `(?:\s*([+-]\d{4}|[A-Za-z]{1,5}))?\s*$`, which only backtracks linearly.
  - Then add this input to the time-bound test.

### 2. Minor: the time-bound test's name claims more than the test checks
- **Location:** `judge_due_test.ts:396`, `"a long hostile Date line is refused in linear time"`.
- **Problem:** finding 1 shows a long hostile Date line that is not refused in linear time. The test
  really guards the comment-stripping pass.
- **What happens:** the input from finding 1 takes more than 2 seconds, yet the test's name says that
  case is covered.
- **Fix:** rename it to "setting trailing comments aside stays linear", or fix finding 1 and add its
  input to the test.

### 3. Minor: a rewrapped doc comment leaves a stub line
- **Location:** `judge_due.ts:185`.
- **Problem:** `* real calendar day` now sits alone between two full-width lines. This is cosmetic.

## The engineer's rulings

### Ruling 1: a hand-written loop instead of `/(?:\s*\([^()]*\))+$/`. Upheld.
- **Same output as the regex:** I compared `loop(s)` with `s.replace(regex, "")` and got 0 mismatches
  across:
  - 2,000,000 random strings of up to 11 characters over `( ) space \t \r \n a - 0     ﻿ \`;
  - every string of length 0 to 9 over `( ) space a \t`.
- **Linear time:** I timed the loop on ten hostile shapes at 200k characters, including `")"*N`,
  `"("+")"*N`, `"("*N+")"`, `"()"*N`, `"(a)"*N` and `" ("*N+")"`. The slowest took 22 ms, and the rest
  stayed under 1 ms.
- **Why the loop is linear:** each `lastIndexOf` and `indexOf` scan either covers characters that are
  then removed, or ends the loop.
- **Probes by hand** (outputs from `parseDateLine` and `resolveDue`):
  - **These resolve exactly like the same header without its comment:** an empty `()`, a trailing
    space after the comment, CRLF around the header, a no-break space (` `) before the comment,
    `PST(UTC)`, a NUL inside the comment.
  - **These stay null:** a nested comment, an escaped `\(` or `\)` inside a comment (it reads as
    nested), `(UTC)(`, `)(UTC)`, `(UTC))`, a comment between the time and the zone, a comment after
    the weekday, `-08(00)`, `-(0800)`.
- **One divergence from RFC 5322, accepted:** RFC 5322 allows nested comments and escaped characters
  in comments, and those headers stay null. The code comment and the tests document this. Null is the
  safe direction, and I know of no mail server that writes them in a Date header.

### Ruling 2: no comment stripping on the ISO path. Upheld.
- An ISO 8601 stamp is not an RFC 5322 header, and no source writes one with a comment.
- `"2026-11-10T14:00:00Z (UTC)"` stays null, as it was on base.
- The ISO pattern still runs on the original `s`, untouched.

## The checks

- **Headers without a comment:** behavior is unchanged. When the line does not end in `)`,
  `withoutTrailingComments` returns the trimmed input as it is, and that is the string base matched.
  The diff changes nothing else in `judge_due.ts` besides the helper, the one call site and the doc
  sentence.
- **Callers:** the only entry is `resolveDue` (`parseStamp` is private). `parseDateLine` is exported,
  but I found no caller outside the tests.
  - `judge_pipeline.ts:299` (judge-email): newly resolved dues still go through `validate()` and
    `realDue`.
  - `judge_validate.ts:299` `mailDue` (mail-process, gmail-read): passes through `realDue`.
  - `mail_templates.ts:265`: a `change` row on `due` that used to be dropped as null now emits a claim
    with the correct due. That is the point of the fix.
  - The pair tests show a commented header gives the same answer as the same header without its
    comment, across 5 timezones and 5 phrases. So a due that used to be null resolves to the date the
    uncommented header already gave, and no new kind of misfire appears.
  - The output shape (`YYYY-MM-DD[THH:MM]`, naive) is unchanged, which is what `engine/src/mailreconcile/resolve.rs:356` expects.
  - `mime.ts` has its own `parseDate`, used for `dateAt` and ordering, and this diff does not touch it.
- **The new tests:**
  - On base (the branch's tests run against base `judge_due.ts`) the result is 124 passed and 32
    failed. The 9 new tests that pass there are the null guards and the time bound, as the engineer
    reported.
  - On the branch all 156 pass.
  - I recomputed the weekday expectations:
    - 2026-11-01 is a Sunday, so 11-10 is a Tuesday and Friday is 11-13.
    - Wed 04:30 +0000 is Tue 22:30 CST.
    - Wed 02:00 +0530 is Tue 20:30 UTC, which is 14:30 CST.
    - Tue 09:00 -0800 is 11:00 CST.

    All are correct.
  - Every assertion goes through the public `resolveDue` or `parseDateLine`, so no test pins the
    implementation.
  - The 1000 ms bound is about 3000 times what the branch needs (0.3 ms) and about a tenth of what the
    quadratic regex took, so it is not flaky on a slow runner and would catch a regression to the
    regex.
- **CLAUDE.md:**
  - Nothing under `engine/` changes, and nothing on the contract list.
  - Both files are LF (`i/lf w/lf`), and every added line is ASCII. The non-ASCII em dashes in the
    diff are on unchanged context lines.
  - The tests contain no real person, school or address; `America/Chicago` and the other timezones
    are generic.
  - The tests are pure functions with no network.
  - `deno check` and `deno lint` are clean on both files. `deno fmt --check` flags `judge_due.ts`, but
    it flags the base file too, and fmt is not a CI gate.
