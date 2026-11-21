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

Exit codes: 0 means a report was printed, 2 means bad input or a refused link, and 1 means the
fetch failed.

## What is and is not kept

- **Nothing is kept.** The tool never writes a file. The report goes only to the screen.
- **The link is a secret.** It is read from one line of stdin, never from the command line, so it
  stays out of shell history and the process list. It is never printed, logged or written. An
  error names the HTTP status or the kind of failure, never the link. Only `https://` links are
  fetched (`webcal://` is read as `https://`). Anything else is refused. A fetch gives up after 20
  seconds or past 20 MB.
- **By default no text from the student's items is shown.** That covers titles, descriptions,
  locations, class names and `X-` values. The default report holds counts, dates, times of day,
  property names and UID shapes (digits and hex replaced). A UID shape can name the school's
  site host.
- A name match (for example the word "mastering") is a hint, not proof. A count of 0 does not prove
  a site is absent: its items may carry no marker.

## Tests

```
node --test scripts/site-check/feed-shape.test.mjs
```

Every feed in the tests is made up. No test touches the network.
