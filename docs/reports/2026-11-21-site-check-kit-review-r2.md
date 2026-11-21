PASS WITH FIXES

# Review round 2: site-check kit (feed-shape), branch tools/site-check-kit

Diff checked: `git diff origin/main...HEAD` (36e986c6, 0cd45117, dd2f70fa). It touches only
`scripts/site-check/{feed-shape.mjs, feed-shape.test.mjs, README.md}`. No contract-list file.

Verdict: PASS WITH FIXES. All eight round-1 findings are closed, and each has a test that would fail
if it reopened. One narrow leak remains (finding 1): the host kept after a UID's "@" can carry text
from a broken DESCRIPTION line. The fix is a few lines and one test. Findings 2 to 4 are Minor.

Tests: `node --test scripts/site-check/feed-shape.test.mjs` on Node v24.19.0 gives `tests 45, pass 45,
fail 0`. I also ran the module and the CLI on made-up feeds kept in a scratch folder outside the
repository (deleted afterwards). I checked the manual redirect with Node's real `fetch` against a
listener on 127.0.0.1 (a 302 to an http path: refused after one request).

## Round-1 findings

1. Text after a raw line break printed as a property name: **closed**. Tests: "a line inside an item
   that is not a property line..." and "only registered names and uppercase X- names count as
   properties".
2. UID shapes kept every word: **closed** for words. Tests: "UID shapes mask words except the tool's
   structural words..." (it plants a sentinel in a UID). The README now lists exactly what remains.
   The host rule this fix added has its own gap (finding 1 below).
3. https-to-http redirect: **closed**. Tests: "an https to http redirect is refused..." and the
   6/5/2-hop and missing-Location tests. Confirmed with a real `fetch` on loopback.
4. Invisible U+FEFF in the source: **closed**. It is now written `﻿`. Tests: "a feed that
   starts with a BOM parses", plus a test that no source file holds invisible or control characters.
5. Wrong run line in the test file: **closed** (comment only, so no test applies).
6. Test gaps: **closed**. There are now tests for a quoted parameter with `:` and `;`, a 5 MB line,
   a declared content-length, an empty file, redirects, and the same items in reversed order.
7. Host fragments matching as substrings: **closed**. Tests: "host fragments match only at a host
   boundary" and "names match as whole words, any Unicode letter...".
8. A link on the command line: **closed** for `--file <link>` and for a positional link. Test: "a
   link on the command line is refused...". Two other forms are in finding 4 below.

## Important

### 1. The host kept after "@" can carry a broken line's text, or any dotted text
`scripts/site-check/feed-shape.mjs:226` (`PLAIN_HOST`) and `:243-248` (`uidShape`), together with
`:101-104` (`isPropertyName`, any case). This breaks guarantee 2 for malformed lines.

A prose line that starts with a registered name, in any case, is still parsed as that property. For
`UID`, part of its value then prints. Suppose a generator leaves a line break in DESCRIPTION
unescaped, and the next line starts "Uid:". If that line comes before the real UID, it becomes the
item's UID (`first(item, "UID")`). Its text after the last "@" is then kept as written, because
`PLAIN_HOST` accepts any dotted run of letters and digits.

Input (made up):
```
BEGIN:VEVENT
DESCRIPTION:Questions?
Uid: write to zqxprof@Zqxname.Zqxleak
UID:123@lms.example
DTSTART:20261101T080000Z
END:VEVENT
```
Default output, seen when I ran it (the text report and the JSON both carry it):
`1  <w> <w> <w>@Zqxname.Zqxleak`.

A real UID gives the same result when its generator puts dotted text after the "@":
`UID:lab-12@Ch.4.Zqxreading` becomes `<w>-#@Ch.4.Zqxreading`.

Fix:
- A UID value that contains whitespace has the shape `(UID with spaces)`. Generated UIDs never
  contain spaces, and this rule alone closes the broken-line case.
- Keep the host as written only when it ends the UIDs of at least 2 items. A school's site host
  repeats on every item; text after "@" on one item does not. Mask it otherwise. Pre-count the
  hosts first, so the output stays deterministic.

Test: plant the input above, and a single `UID:lab-12@Ch.4.Zqxsentinel`. Assert the sentinel is in
neither the text output nor the JSON. Keep the existing `@lms.example` cases that are shared by
2 or more items passing.

## Minor

### 2. "Begin:" or "Due:" at the start of a broken line still bends the parse
`scripts/site-check/feed-shape.mjs:96` and `:175-178`. Nothing leaks, but the report is wrong.
`BEGIN` and `END` are accepted in any case, and so is every registered name. Input:
`DESCRIPTION:Steps` / `Begin: chapter 4 tonight` / `UID:b-1` / `DTSTART:20261101T080000Z` /
`SUMMARY:Lab 1 - Due`. The second line opens a phantom component named "CHAPTER 4 TONIGHT", which
takes the item's later lines. I saw the item reported as `no date: 1`, with the shape `(no UID)` and
the "due" ending not counted. In a VTODO, `Due: Friday after class` placed before the real `DUE`
gives `no date: 1` the same way. This behaviour predates round 2, but dd2f70fa now states it as the
rule.

Fix: accept a `BEGIN` or `END` line only when its value is one token
(`/^[A-Za-z0-9-]+$/` after trimming). Count anything else as `(not a property line)`. Add a test
with the input above.

### 3. An all-uppercase broken line that starts "X-" still prints its name
`scripts/site-check/feed-shape.mjs:97`. Input: `DESCRIPTION:Bring` / `X-ZQXNAME: room 4`. The
census shows `X-ZQXNAME  1`. Such a line cannot be told apart from a real extension property, so
this is a residual risk of the chosen rule, not a bug. But the README (`:58-63`) reads as if broken
lines are always caught. Fix: add one sentence to the README saying that an all-uppercase line
starting `X-` is read as a property name and printed.

### 4. Two command-line forms of a link get the generic refusal
`scripts/site-check/feed-shape.mjs:626`. The regex is anchored, so it catches neither
`--file=https://cal.example/feed/tok.ics` nor an argument with a leading space
(`" https://..."`). Both exit 2 with "unknown argument (not shown...)". `--file " webcal://..."`
gives `cannot read the file (ENOENT)`. Nothing leaks, but the student is not told to reset the
link. Fix: test `/(https?|webcal):\/\//i` anywhere in each argument, and add these three cases to
the existing test.

## Checked and holding

- **The link** is in no output. Errors give a status, a code from a restricted pattern, or a fixed
  phrase. The redirect refusals name no URL (tested, including an unreadable Location). The argv
  refusal does not echo the argument. The `--file` error prints only `e.code`. stdin is muted on
  a TTY.
- **Default output without `--show-titles`.** No SUMMARY, DESCRIPTION, LOCATION, CATEGORIES or
  X- value appears, and no malformed line appears, except for findings 1 and 3. I checked each
  section:
  - The census prints only registered names, accepted X- names and the fixed label.
    `ſUMMARY` uppercases to `SUMMARY`, which is still a registered name.
  - Site sections list field names only.
  - Title endings count fixed phrases only.
  - Classes print as `class N`.
  - Parameters are never printed. I planted sentinels in a parameter name and a parameter value:
    nothing appeared.
  - Allowlisted UID words print only themselves, so they can carry nothing else. Non-letter
    symbols survive the masking (an emoji, combining marks such as U+0301 or the Devanagari
    vowel signs), but they cannot spell a word.
- **Nothing is written to disk.** The only import is `readFile`, there is no `process.env`, and
  there is no `console.*`.
- **Redirects** never leave https: `redirect: "manual"`, at most 5 hops, each resolved against the
  current URL. Real Node `fetch` exposes the 3xx and its Location in manual mode.
- **Determinism.** Two CLI runs on the same file are byte-identical, and reordered items give the
  same report (tested).
- **Parsing ordinary feeds** still works:
  - lowercase `begin:vtodo`, `uid:`, `due;value=date:`, `summary:`;
  - folding with a space or a tab, CRLF, quoted parameters, `X-ALT-DESC;FMTTYPE=...`;
  - a nested VALARM stays off the item.
- **One regression, accepted.** A mixed-case or lowercase X- name is valid in RFC 5545 (names are
  case-insensitive), but it now counts as `(not a property line)`, and a site marker in it is
  missed: `x-lms-link:https://www.webassign.net/x` gives WebAssign 0. dd2f70fa chose this
  deliberately, and the README and the tests state it. I recommend keeping it.
- **Repository rules.**
  - Only the three files under `scripts/site-check/` change.
  - All three are `i/lf w/lf`, with no BOM and no CR.
  - The invisible-character test passes.
  - There is no dependency and no `package.json`.
  - No person, school, machine path or account is named. Classes are made up, and real names
    appear only as product names.

## For Quinn

- Finding 3: accept, as residual, that an all-uppercase prose line starting `X-` prints its name,
  and document it. Recommendation: accept. The only other choice is to drop every X- name from
  the census, which loses the signal the tool exists to collect.
