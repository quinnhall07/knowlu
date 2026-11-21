FAIL

# Review: site-check kit (feed-shape), commit 36e986c6 on origin/main

Size: Small. Diff: `scripts/site-check/{feed-shape.mjs, feed-shape.test.mjs, README.md}`, nothing else.
No contract-list file is touched.

Verdict: FAIL, on guarantee 2 ("no feed text in the default output, by any path"). Two realistic
inputs put feed text into the default report. Both fixes are small: once findings 1 and 2 are fixed
and tested, this is a PASS. Finding 3 (redirect downgrade) should go in the same round.

Tests: `node --test scripts/site-check/feed-shape.test.mjs` on Node v24.19.0 gives
`tests 27, pass 27, fail 0`. I also ran the tool with `--file` on made-up feeds kept in a temp folder
outside the repository, and deleted them afterwards.

## Important

### 1. A raw line break in a text value puts that text in the "Properties" section
`scripts/site-check/feed-shape.mjs:79-81` (`parseLine`), `:290`, `:383`. Breaks guarantee 2.

`parseLine` treats everything before the first `;` or `:` as the property name, and does not check
that it looks like a name. Some feed generators write a newline inside DESCRIPTION without escaping
it. The line after that break is then not folded, so it parses as a property of the item. Its "name"
is the student's text, uppercased, and it is counted and printed in the census.

Input (made up):
```
DESCRIPTION:Bring the following
Note from Prof Zqxname: ZQXLEAK see me
```
Default output, seen when I ran it:
```
== Properties on items (items carrying each) ==
...
NOTE FROM PROF ZQXNAME  1
```
Any text line with a colon in it leaks this way ("Due: Friday", "Reading: chapter 4 of ..."). A
line that starts with `X-` (for example "X-ray results: ...") also becomes a marker property, so its
name can show under a site's `fields:` list too. The JSON output carries the same name.

Fix: accept a property only when the name matches `/^[A-Za-z0-9-]+$/` (RFC 5545 iana-token or
x-name). Anything else counts toward a single "lines that are not properties: N" figure, and its
text is never kept. Test: plant a sentinel in an unfolded continuation line, as above, and assert it
appears in neither the text nor the JSON output.

### 2. UID shapes keep every word in the UID; the README says less than that
`scripts/site-check/feed-shape.mjs:184-190` (`uidShape`), `:293`, `:386`; `README.md:46-48`. Breaks
guarantee 2. The README is not accurate (point 7).

Exactly what survives the masking: every run of letters that is not an all-hex run of 8 or more
characters. That means every word with a letter from g to z in it ("photosynthesis", "lab",
"writeup"), hex-only words shorter than 8 letters ("facade", "cafe", "bead"), all punctuation, the
host, and usernames or email local parts with only their digits replaced (`jdoe42` becomes `jdoe#`).
What is masked: UUIDs, digit runs, and runs of 8 or more hex characters that include a letter.

Input: `UID:photosynthesis-lab-writeup-jdoe@lms.example`. Default output:
`1  photosynthesis-lab-writeup-jdoe@lms.example`. When a generator builds UIDs from titles or from
student logins, every UID has its own shape, so the top 12 shapes print 12 titles or usernames. The
README promises only that "a UID shape can name the school's site host". The planted-sentinel test
(`feed-shape.test.mjs:178-216`) never puts the sentinel in a UID, so nothing catches this.

Fix: keep the part after the last `@` (the host) as written. On the left of the `@`, replace each
letter run with `<w>` unless it is on a short fixed allowlist of structural words (`event`,
`assignment`, `calendar`, `quiz`, `discussion`, `todo`, `item`, `course`, and so on); digits become
`#` and hex runs `<hex>`, as now. Make the README sentence state exactly what can remain. Add a test
that plants the sentinel in a UID.

### 3. `redirect: "follow"` lets the server move the fetch to plain http
`scripts/site-check/feed-shape.mjs:454`. Breaks guarantee 1 ("only https is fetched").

`normaliseLink` checks only the first URL. Node's fetch (undici, following the Fetch standard) will
follow an https-to-http redirect. Scenario: the feed host answers with `302 Location:
http://<same host>/feed/<token>.ics`. The tool follows it. The token in the path, and the whole feed
with the student's titles, then cross the network in cleartext, and the report looks normal. No
link is printed, so "redirects cannot print the target" does hold.

Fix: use `redirect: "manual"` and follow up to 5 hops by hand. Resolve each `Location` against the
current URL and refuse any that is not `https:`, with an error that names no part of the URL
(`refused: the feed redirected to a non-https address`). Test with a fake fetch that returns a 302 to
an `http://` Location: it must exit 1, make no second call, and print no part of the link.

## Minor

### 4. An invisible U+FEFF sits inside the source, and no test covers a BOM
`scripts/site-check/feed-shape.mjs:127`. The regex is `/^<literal U+FEFF>/`. The file does not start
with a BOM, and LF holds. But the character is invisible, and an editor or formatter that strips
BOMs can delete it without anyone noticing. Fix: write `/^﻿/`. Add a test that passes
`"﻿BEGIN:VCALENDAR..."` to `parse` and expects `isCalendar: true`. (In the CLI, `TextDecoder`
already strips a leading BOM: I ran a BOM-prefixed file and it parsed.)

### 5. The run line at the top of the test file fails on current Node
`scripts/site-check/feed-shape.test.mjs:1` says `node --test scripts/site-check/`. On Node 24 that
fails with `MODULE_NOT_FOUND` (tests 1, fail 1), because the argument is treated as a module. The
README's form (`node --test scripts/site-check/feed-shape.test.mjs`) works. Fix: use the README's form
in the comment.

### 6. Gaps in the tests (point 5)
The tests open no socket (only in-process `new Response`) and use only made-up classes. These cases
are not tested:
- a quoted parameter value that contains `:` or `;` (`ATTENDEE;CN="Doe: J; Z":mailto:...`). It parses
  correctly when I run it, but no test holds that;
- a huge single line (a 15 MB DESCRIPTION ran in about 0.3 s, untested);
- the declared `content-length` refusal (`:461-462`); the size test only covers the streamed path;
- an empty file (`""`, which exits 2 with "not a calendar");
- a redirect (finding 3).

`the same input renders byte-identical twice` (`:226-231`) can fail only if randomness or the clock
enters the code. Feeding the same items in two orders and comparing the census and site sections
would test ordering for real.

### 7. Host fragments match as substrings
`scripts/site-check/feed-shape.mjs:213-214`. The output says a host match is a hint, but `aktiv.com`
hits `praktiv.com` and `101edu.co` hits `101edu.com.example`. Name matching is whole-word as asked:
`nortonville` does not match `norton`. JS `\b` is ASCII-only, though, so a name right after a
non-ASCII letter still matches (`émastering`). Fix: require a host to start the string or follow a
non-host character (`(^|[^a-z0-9-])`), and to end at a non-host character.

### 8. A link put on the command line is not refused as a link
`scripts/site-check/feed-shape.mjs:510-512`, `:523-525`. `--file https://host/feed/token.ics` is read
as a path: the tool prints only `cannot read the file (ENOENT)`, so nothing leaks. But the link is
already in the shell history and the process list, and neither this message nor the "unknown
argument" refusal tells the student so. Fix: when a `--file` value or a positional argument looks
like `scheme://`, refuse it with "that looks like a link: it is now in your shell history; reset the
feed link in your school's site and paste the new one when asked".

## Checked and holding

- Link: never in stdout or stderr. Every error names an HTTP status, `ENOTFOUND`-style codes (from a
  restricted pattern) or an error name. The internal-error handler prints only `e.name`. A positional
  argument is refused without being echoed. stdin is muted on a TTY. Only `webcal://` is rewritten;
  `http://` and other schemes are refused. There is a 20 s timeout signal, which also covers reading
  the body, and a 20 MB cap on both the declared and the streamed size. No environment variable is
  read.
- Never writes a file: it imports only `readFile`.
- Default output: dates and times are taken from digits only. Class names are replaced by
  `class N`. Title endings count only the fixed phrases, and the test checks that other endings stay
  hidden. Site sections list field names only. DESCRIPTION and LOCATION are never printed, even with
  `--show-titles`.
- Parsing: space and tab unfolding, CRLF, LF and lone CR, escapes, properties of a nested VALARM kept
  off the item, unbalanced END handled.
- Determinism: every list is sorted with a tiebreak, and site titles are in input order.
- Repository rules: three new files under `scripts/site-check/` only, all `i/lf w/lf`, no BOM at the
  start of any file, no `package.json` or dependency. No person, school, machine path or account is
  named; product names only. Nothing needs editing for a second user.
- README: accurate except the UID sentence (finding 2). `--show-titles` shows SUMMARY under each
  matched site, up to 40 unmatched titles and class names, as the README says.
