# site-check: feed-shape

A read-only look at what a school calendar feed (the iCalendar link from Blackboard, Canvas or
Brightspace) carries. Before building a reader for another homework site, it shows whether that
site's assignments already arrive in a student's feed, and in what shape. It does this without
printing the student's item titles or keeping the link.

Node 20 or later. No install and no dependencies.

## At a sitting

1. The student finds their calendar feed link in their school's site. In Canvas it is under Calendar,
   then Calendar Feed. In Blackboard and Brightspace it is under the calendar's share or subscribe
   option.
2. Run the tool from the repository root:

   ```
   node scripts/site-check/feed-shape.mjs
   ```

3. When it asks `Paste the calendar link, then Enter:`, the student pastes the link themselves and
   presses Enter. On a terminal the pasted text is not echoed.
4. Read the report together. Its sections are totals, the property names on items, UID shapes,
   class counts, outside-site markers per site, and title endings.

Options:

- `--file <path>` reads a saved `.ics` file instead of fetching.
- `--now YYYY-MM-DD` sets the day the before/after counts use. The default is today.
- `--json` prints the same summary as one JSON object.
- `--show-titles` also shows item titles and real class names. **It is for eyes at the sitting
  only.** Do not save, paste or share its output.

Exit codes: 0 means a report was printed. 2 means bad input or a refused link, including a link put
on the command line. 1 means the fetch failed, including a redirect the tool refused.

## What is and is not kept

- **Nothing is kept.** The tool never writes a file. The report goes only to the screen.
- **The link is a secret.** It is read from one line of stdin, never from the command line, so it
  stays out of shell history and the process list. It is never printed, logged or written. An
  error names the HTTP status or the kind of failure, never the link or any part of it. Only
  `https://` links are fetched (`webcal://` is read as `https://`). Anything else is refused. A
  fetch follows at most 5 redirects, only to `https://` addresses. It gives up after 20 seconds
  or past 20 MB.
- **If a link is typed on the command line**, the tool refuses it without repeating it. That
  covers any argument holding `://` and any `http:`, `https:` or `webcal:` at an argument's start
  or after `=`. The link is then in the shell history, so reset the feed link in the school's
  site and paste the new one when asked.
- **By default no text from the student's items is shown.** That covers titles, descriptions,
  locations, class names and `X-` values. The default report holds counts, dates, times of day,
  property names and UID shapes.
- **A UID shape can still show** the host after the UID's last `@`, as written. That happens only
  when it is a plain host name and the UIDs of at least 2 items end with it (usually the school's
  site host); otherwise it shows as `<host>`. It also shows punctuation and these structural
  words: assignment, blackboard, brightspace, calendar, canvas, course, d2l, discussion, due,
  event, gradableitem, gradebook, ics, instructure, item, lms, moodle, platform, quiz, todo, uid.
  Every other word becomes `<w>`, digit runs become `#`, and UUIDs and runs of 8 or more hex
  characters become `<hex>`. A UID holding whitespace shows only as `(UID with spaces)`.
- **Property names** come from a fixed rule. A line counts as a property only if its name is
  registered in RFC 5545 or RFC 7986 (in any case), or starts with `X-` and is written entirely
  in uppercase letters, digits and `-` (as generated extension properties are). `BEGIN` and `END`
  count only when their value is one word. Any other line inside an item counts as
  `(not a property line)`, and its text is not kept. That line is usually text broken across
  lines without escaping, such as `Reading: chapter 4`, `X-ray: results` or
  `Begin: chapter 4 tonight`. The site markers read only the `X-` properties this rule accepts.
  One case cannot be caught: a broken line written all in uppercase that starts with `X-` (for
  example `X-ZQX: room 4`) looks exactly like a real extension property, so its name is printed.
- **For UID, DTSTART, DTEND and DUE the first well-formed value wins.** For UID that means no
  whitespace; for the dates, a date or a date-time. A broken line such as `Due: Friday` never
  replaces an item's real date or UID.
- A name match (for example the word "mastering") is a hint, not proof. A count of 0 does not prove
  a site is absent: its items may carry no marker.

## Tests

```
node --test scripts/site-check/feed-shape.test.mjs
```

Every feed in the tests is made up. No test touches the network.
