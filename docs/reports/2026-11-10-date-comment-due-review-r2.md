PASS (Spec: PASS, Quality: PASS)

# Re-review: fix/date-comment-due, round 2 (`git diff 173ac640 fde921bf`)

The round-1 report is `2026-11-10-date-comment-due-review.md`. This round is a new file, because a
report is never overwritten. The overall verdict on the branch at `fde921bf` is on line 1.

Scope: one commit, two files, `judge_due.ts` (+24/-11) and `judge_due_test.ts` (+57/-10). Nothing
under `engine/` and nothing on the contract list changes. Both files are LF, and every added line is
ASCII; the rewrapped doc comment now uses `--` where the old line had an em dash. The worktree file
matches the commit byte for byte.

## Round-1 findings

1. **Important: the RFC 5322 pattern took quadratic time on a header with no zone. Closed.**
   - The tail is now `(?:\s*(zone))?\s*$`, and the `bare` line is capped at 200 characters.
   - My input from round 1, `"Tue, 10 Nov 2026 09:00:00" + " ".repeat(40000) + "?"`, now returns
     `null` in under 0.1 ms. It took 2224 ms before.
   - The engineer also found and fixed the same shape in two patterns that read the due phrase,
     `splitTime` and `LONG_FORM_RE`. I had not looked at those.
2. **Minor: the timing test's name claimed more than the test checked. Closed.** The test is renamed
   to "length: Date lines of 10k-120k characters ...", and the round-1 input is in it.
3. **Minor: a rewrapped doc comment left a stub line. Closed.**

## The checks the coordinator asked for

### 1. Each rewrite matches exactly the strings the old pattern did, with the same captures

**Method.** I tested the old and new functions themselves, not copies of the patterns:
- I made copies of `judge_due.ts` at `173ac640` and at `fde921bf` and added an export of `parseStamp`,
  `splitTime`, `parseLongForm` and `LONG_FORM_RE` to each.
- I compared their whole return values, not only the match:
  - `parseStamp`: 3.5M structured Date lines, about 138k of which parse. The whitespace included tab,
    U+00A0, LF, CRLF+tab, double zones, zone-like trailers and trailing comments.
  - `parseLongForm` and the raw `LONG_FORM_RE` match: 1.5M phrases, 180k of which match.
  - `splitTime`, `rest` included: 1.5M phrases, 27k of which match.
- **Result: 0 mismatches.**

**`splitTime`'s `\bat` anchor.**
- **"at" inside a word:** `flat 5`, `that 5pm`, `x_at 5` and `atat` stay null, as before, because
  `\b` was always there.
- **"at" after a non-word character:** `x-at 5` and `caf` + U+00E9 + `at 5` split the same way
  under both (`\b` is ASCII-only in both).
- **Several "at"s:** `the meeting at noon at 5` gives rest `the meeting at noon` under both. So does
  `friday at 5 at 6`.
- **Leading whitespace:** `  at 5` gives the same result.
- **Why trimming can't change:** the old match began at the whitespace before "at" and the new one
  begins at "at". The caller then does `slice(0, m.index).trim()`, so the only characters between the
  two starting points are whitespace that the trim removes anyway.
- **Why both pick the same "at":** only the last "at" can be followed by nothing but digits, am/pm
  and whitespace up to the end, so there is only one "at" either pattern can match.

**`LONG_FORM_RE`'s `|\s+` alternative.**
- **Same strings:** the old tail after the time was `ws* ampm? (ws+ zone)?`. The new one is
  `(ws* ampm)? (ws+ zone)? | ws+`. Every old split falls into one new branch:
  - am/pm present: the first group.
  - Zone only: `ws* ws+` becomes `ws+`.
  - Neither: empty, or the `\s+` branch.

  The new pattern adds nothing the old one rejected.
- **Same captures:**
  - The am/pm group is tried first in both, so `11:59 pm` captures am/pm and not zone `pm`.
  - `11:59 am pm` gives am `am` and zone `pm` in both.
  - `11:59 pmx` gives zone `pmx` and no am/pm in both.

**RFC tail.** `ws* zone? ws*` and `(ws* zone)? ws*` describe the same strings. A zone has to reach
the trailing `\s*$`, so there is only one way to read it, and the capture is the same.

### 2. Linear time on hostile input, for both arguments

At 100k characters, every shape I could build ran in 0.0-9 ms.

**Date line:**
- a valid header followed by 100k `(a)` comments (`bare` stays under 200, and it parses);
- ISO plus a comment run;
- ISO plus a whitespace run;
- tab and U+00A0 runs;
- `(` x N + `)`;
- `()` x N.

**Due phrase:**
- `at` + whitespace + `x`;
- `at ` x N, `at 5 ` x N, `at 5 pm ` x N;
- `at 5` + whitespace + `pm` + whitespace + `x`;
- a leading whitespace run;
- `friday ` x N;
- the long form with whitespace runs between the date and the time, around `at`, after the time,
  around `pm` and around the zone;
- `11:59` + (` pm` x N);
- `LONG_FORM_RE` on `a ` x N and on `a.,` x N;
- `this` + whitespace + `x`.

**Growth with length:** I timed the five slowest shapes at 100k, 200k and 400k characters. Time grows
in proportion to length (for example 0.7 / 1.2 / 3.2 ms for `at 5` + whitespace + `x`).

**Every pattern left in the module is linear:**
- the ISO pattern (anchored, and it runs on `s` only after the cap check);
- `THIS_WEEKDAY_RE` (anchored);
- `ABSOLUTE_DUE_RE` (anchored, no whitespace);
- the numeric-offset pattern;
- the `.` replace in `clockTime`.

**On `173ac640`,** this round's tests give 156 passed and 4 failed, as the engineer reported: the
three length timings plus the cap test. On `fde921bf` the test file gives 160 passed and 0 failed,
and the full CI `deno test` line gives 1493 passed (588 steps) and 0 failed. `deno check` and
`deno lint` are clean on both files.

### 3. The 200-character cap never nulls a real Date header

- **Where the cap applies:** it measures `bare`, the line after trailing comments are set aside, and
  the length is measured after `trim()`.
- **Real lengths:**
  - A real RFC 5322 Date with a numeric zone is at most 31 characters, and the pattern already accepts
    only a three-letter weekday.
  - `mime.ts` unfolds headers, so a folded line adds only a few whitespace characters.
  - A fold inside a trailing comment is set aside with the comment.
  - A comment that is not trailing was already null.
- **Boundary:** a well-formed line padded to 200 characters parses, the same line at 201 is null, and
  a 200-character line followed by a 5000-character trailing comment parses.
- **ISO path:** a trimmed ISO stamp cannot get near 200 characters.

### 4. The engineer's rulings

**No cap on the due phrase. Upheld.**
- Every pattern that reads the phrase is now linear, and I checked that up to 400k characters.
- The phrase is either model output (bounded by the model's token limit) or a template capture.
- A cap would be a second length rule to keep in step with the first, and it would protect nothing
  the linear patterns don't already protect.

**The RFC rewrite is backed by the audit, not a unit test. Upheld.**
- **What I confirmed:** if I put back the old RFC tail and keep the cap, all 160 tests still pass. The
  cap is what guards that path.
- **Why a unit test can't help:** below 200 characters, even the old quadratic tail costs
  microseconds, so a test cannot tell the two apart by time.
- **Why not reach the pattern directly:** it would mean exporting the pattern or lifting the cap
  inside a test, and that tests the implementation, not the behavior.
- **What already covers it:** the existing RFC rows pin the captures, and my 3.5M-line comparison
  found no difference between old and new.
- **What a regression would take:** removing both the cap and the rewrite. If both went, the
  10k-120k test would fail again.

### 5. Neutral wording

- No added comment, test name or the `fde921bf` message mentions ReDoS, backtracking, a sender
  triggering anything, or a measured time. The test names state bounds ("within one second"), which
  is neutral.
- The round-1 comment about the quadratic regex is gone. See finding R2-2 for where the wording still
  survives.

## New findings

### R2-1. Minor: the long-comment test doesn't check that the cap comes after comment stripping
- **Location:** `judge_due_test.ts`, the test "length: the longest real header shapes still parse,
  long comments included".
- **Why:** its commented header is 55 + 93 = 148 characters, under the cap whether or not the comment
  is stripped first.
- **Mutation check:** I changed `if (bare.length > MAX_DATE_LINE)` to `if (s.length > MAX_DATE_LINE)`.
  All 160 tests still pass.
- **What that change would break:** a real header followed by a 200-character comment would become
  null without any test noticing.
  - Example: `"Wed, 30 Sep 2026 23:59:59 -0700 (Pacific Daylight Time) (" + "c".repeat(200) + ")"`.
  - Expected: the 23:59 wall clock.
  - Actual under the mutation: `null`, with every test green.
- **Also:** the test's string says the comment is "a hundred and twenty characters", but it is 92
  characters between its parentheses.
- **Fix:** make the comment long enough that the whole line goes past 200, and drop the length claim
  from the string.

### R2-2. Minor (for the controller, not the branch): the old wording survives outside the new commit
- **The `173ac640` commit message** still says "that regex, unanchored, backtracks quadratically on a
  hostile line (40,000 "(a)" groups took ten seconds)".
- **My round-1 report,** uncommitted, describes the path a sender could reach and gives timings.
- **If neutral wording in public history is the goal:**
  - squash-merge with a neutral message;
  - keep the round-1 report out of the commit, or reword it before committing.

Neither affects the code.
