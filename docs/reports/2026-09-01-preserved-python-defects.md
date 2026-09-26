# Python defects found during the Rust port — preserved, not fixed

**Started:** 2026-09-01, during waves 0–3 of the Rust port.
**Extended:** 2026-09-01 (approvals re-read, defects 14–17; wave 5, cross-cutting trap 4),
2026-09-02 (wave 6 — `coursework`, `zybooks`, `vhl`; defects 18–19, four latent hazards, and the
wave-6 port deviations) and 2026-09-02 again (the Task 17 sweep of `calfeed`, the five `event*`
modules, `runs`, `info`, `issues`, `cli`; defects 20–22, a second site for 19, eight latent
hazards, and cross-cutting trap 5). **22 numbered defects and 5 cross-cutting traps.**

The port's governing rule is behaviour parity: the Rust engine is correct when it emits the same
bytes as Python from the same vault. That rule forbids fixing anything mid-port, **including
genuine bugs** — a golden-file diff must have exactly one possible cause.

So defects found while reading the Python go here instead. This file is the **post-cutover fix
list**. Nothing in it may be actioned until the oracle is green and the cutover is done (rewrite
spec §8), at which point each entry becomes a normal change with a normal test.

Each entry records how it was verified. "Verified by running Python" means the behaviour was
reproduced against the real interpreter, not inferred from reading.

---

## `engine/eventledger.py`

Found 2026-09-01 while porting to `src/eventledger.rs`. All four verified by running the Python.

### 1. A U+2028 in an event title silently destroys the verdict — with no warning at all

`record_verdict` sanitises `·`, `\n` and `\r` out of a title, but **not** U+2028, U+2029, `\v`,
`\f`, or `\x1c`–`\x1e` — every one of which `str.splitlines()` treats as a line boundary. On the
next read the record splits in two: the head has no `verdict:` and the tail does not start with
`- `, so **neither half trips the warning condition**. `load_ledger` returns `{}` with an empty
warnings list.

This is precisely the failure mode `sanitize_uid`'s docstring exists to prevent, arriving through
the *title* instead of the uid. Scraped event titles are a plausible source of U+2028.

**Severity: high.** It is a silent data loss in a ledger whose entire job is to stop an event being
re-judged, and the system's standing rule is that failures are visible.

### 2. A uid containing the substring `verdict:` breaks its own marker lines, forever

`:` is inside the uid character class and `sanitize_uid` deliberately keeps it. So a uid like
`x:verdict:1` fails the `"verdict:" not in line` guard used to tell a marker line from a verdict
line. The uid is never recorded as proposed, and it warns `unreadable verdict line` on **every run
from then on**.

**Severity: medium.** Self-inflicted only by a source minting such a uid, but it is unbounded once
it happens.

### 3. `strength` is unvalidated on write but must match `[a-z]+` on read

`strength="Strong"` reads back as `""`. `strength="very strong"` reads back as `"very"`. No warning
in either case.

**Severity: low**, but it silently degrades the relevance signal the OPPORTUNITY tier depends on.

### 4. `uid`, `strength` and `task` are written unexamined

Only `title` (for `·`/`\n`/`\r`) and `why` (via `why_problem`) are checked. A `task` value
containing ` · why:"…"` injects a `why` field; a uid containing a space writes a permanently
unreadable verdict line. This is why `eventfeed.py` must call `sanitize_uid` at mint time —
`record_verdict` does not enforce it.

**Severity: medium.** The invariant exists but is enforced in the wrong place, so a new producer
that forgets `sanitize_uid` corrupts the ledger silently.

---

## `engine/ledger.py`

Found 2026-09-01 while porting to `src/ledger.rs`. Verified against the interpreter.

### 5. `append` never validates that `ts` is date-shaped before using it as a filename

`ts[:10]` goes straight into a path join to pick the day file. A record whose `ts` begins
`"../../"` writes **outside `state/journal/`**, and a `ts` containing `:` or `/` in its first ten
characters fails with a confusing `OSError` on Windows rather than a clear validation error.

`ts` is supplied by record producers — including the cloud routine, which is a model writing
records through `engine.runs step`. This is not a live exploit and there is no evidence of a
malformed `ts` ever occurring; it is an unvalidated field on a path construction, which is worth
closing on principle in a system that will later run on other people's machines.

**Severity: moderate.** Fix is one `ts` shape check in `append`, and it costs nothing.

### 6. `fsync` covers the file but not the parent directory

`append` does `write → flush → fsync` on the file handle. On POSIX, a crash immediately after a
**new** day file is created can still lose the directory entry even though the record's bytes were
synced. The durability intent is one step short of complete. Irrelevant on Windows today; relevant
the moment anything runs on Linux.

**Severity: low.**

### 7. A float `seq` passes validation and then ties silently

The guard is `int(seq)`, which accepts `1.9` and truncates to `1`. That record then sorts as a tie
with a genuine `seq: 1` instead of being reported as a bad seq. **Severity: low.**

### 8. `until` is normalised only when `len(until) == 10` exactly

A near-miss bare date such as `"2026-8-1"` (9 characters, no zero padding) skips normalisation and
is compared whole against full timestamps, so it filters out **everything**, silently.
**Severity: low**, but it is a silent empty result rather than an error.

---

## `engine/ingest.py`

### 13. `sync_tasks` escapes non-ASCII titles on UPDATE but not on CREATE

`create` writes `json.dumps(event.title, ensure_ascii=False)` — with an explanatory comment saying
coursework does the same "so notes from both ingests agree on what a note looks like". The update
branch three lines earlier writes plain `json.dumps(event.title)`, which defaults to
`ensure_ascii=True`.

So the same title yields different bytes depending on which path wrote it:

```
create:  title: "GN 103 Hausaufgaben — due Wed 08-26 (10 activities)"
update:  title: "GN 103 Hausaufgaben — due Wed 08-26 (10 activities)"
```

**This is reachable and live.** Vault titles already contain em dashes and German umlauts
(`courses/gn-103`, every `Email Prof. …` task). The first time Blackboard changes such a title, the
note's title line flips to the escaped form — an unnecessary byte change that Obsidian Git commits
and pushes.

It does **not** loop: YAML decodes `—` back to the em dash, so the next comparison matches and
it flips only once. Verified against the interpreter.

**Severity: low**, but it directly contradicts the stated intent of the comment two lines below it,
and it is a one-word fix (`, ensure_ascii=False`).

---

## `engine/provenance.py`

Found 2026-09-01 while porting to `src/provenance.rs`. The emitter half was verified by
differentially fuzzing 13,000 cases against the installed PyYAML 6.0.3 — 0 mismatches — which also
caught a transcription of PyYAML **5.x**'s float regex rather than 6.0.3's, the difference between
emitting `.__` and `'.__'`.

### 9. `_flatten` does not flatten mapping KEYS

`{k: _flatten(v) for k, v in value.items()}` — the key is passed through untouched. So the
"defensive flatten" in `judgment_literal` does not defend against a newline in a *key*: instead of
producing a clean single-line literal it raises `ValueError`. **This is reachable** — `inputs` keys
come from agent-supplied dictionaries.

**Severity: moderate.** A judgment write fails rather than corrupting anything, so it is loud, but
it is loud in the wrong place and blames the wrong thing.

### 10. The guard's open and close tests are asymmetric

`guard_block_style` opens on a line that merely *starts with* `---` (so `--- x` opens frontmatter)
but closes only on a line *exactly* equal to `---`. **Severity: low.**

### 11. `judgment: {a: 1}  # comment` is rejected

The guard requires the line to *end* with `}`, so a valid single-line flow mapping with a trailing
YAML comment fails. **Severity: low.**

### 12. Two dead branches

`if not lines` can never fire (`"".split("\n")` is `[""]`, never empty), and in
`if not after_colon or not after_colon.startswith("{")` the first test is subsumed by the second.
**Severity: none** — noted only so a future reader does not mistake them for live paths.

### NOT a defect: `guard_block_style` and CRLF — but read this before touching the read path

The porting agent reported that the guard is blind to CRLF notes: with `\r\n` endings no line ever
equals `"---"`, `frontmatter_end` stays `None`, and the function returns **without checking
anything**. The mechanism is real; the conclusion that it is a live bug is not.

Both call sites (`write.py:138` via `_load`, and `write.py:223` in `create`) receive text from
`path.read_text(encoding="utf-8")` or from a Python-constructed string, so universal-newline
translation has already turned CRLF into LF before the guard sees it. **It works correctly today.**

It matters enormously for the port, though, and it is the same fact as the CRLF trap below seen
from the other side: **if the Rust port reads a note raw, `guard_block_style` does not merely
differ from Python — it silently stops guarding at all**, and the write path starts accepting
block-style `judgment:` blocks it can never edit again. Translate-on-read is not only about byte
parity on write; it is what keeps this guard functional.

---

## `engine/approvals.py`

Found 2026-09-01 by the **deliberate second sweep** of this module. The first sweep, inside the
subagent that ported it, lost its report to an API error mid-verification, so this was the only
module in the port carrying an incomplete defect sweep — the largest module in the engine (958
lines) with zero entries in this file, against four from 7 KB of `eventledger.py`. That asymmetry
was the reason to look again, and it was the right call: two of the four below are reachable, and
one of those is silent.

All four verified by running the Python. **All four are faithfully preserved in `src/approvals.rs`**
— checked against the Rust line by line, which is why the oracle stays green.

### 14. A ticked digest event with ANY trailing text on its line is silently DECLINED — permanently

```python
CHECKED   = re.compile(r"^- \[[xX]\] .*`(?P<uid>[^`]+)`\s*$")
UNCHECKED = re.compile(r"^- \[ \] .*`(?P<uid>[^`]+)`\s*$")
```

Both patterns require the backticked uid to be **the last thing on the line**. `emit_digest` writes
it last, so a pristine digest is fine. But the digest is a **checkbox UI Quinn edits by hand in
Obsidian** — that is its entire purpose — and a line edited to

```
- [x] Wed 9/3 18:00 · Career Fair · Ferg · `bb-ev-123` ask about parking
```

matches **neither** pattern. `_digest_uids` returns it in neither set, so `_expand_digest`'s
`if approve and uid in checked` is false and control falls through to the `else`:

```python
else:
    record_declined(vault, uid, today)
    declined += 1
```

**The event Quinn explicitly ticked YES is recorded as a decline** — in the event ledger, which
`eligible_events` filters on, so it is suppressed from every future digest too. There is **no
warning**: `declined` is an ordinary count on an ordinary path, and the run still reports `ok`.

Verified against the interpreter:

```
- [x] Meet `Dr. Smith` about `ev-123`     -> checked    (greedy .* takes the LAST span: correct)
- [x] Plain `ev-456`                      -> checked
- [ ] Unticked `ev-789`                   -> unchecked
- [X] Capital X `ev-abc`                  -> checked
- [x] trailing text after `ev-ghi` here   -> NEITHER    <-- silently declined
```

**Severity: high.** Silent, effectively irreversible, and triggered by ordinary use of the very
surface it guards. It is the only defect in this file where the system does the *opposite* of what
the user explicitly asked for, leaving no trace.

Not triggered by any plugin currently installed — `homepage`, `obsidian-git`,
`obsidian-meta-bind-plugin`, `templater-obsidian`, checked this session. A task plugin that appends
a completion stamp to a ticked line would fire it on every tick; none is installed today.

**Fix, post-cutover:** capture the last backtick span without requiring end-of-line, or better,
WARN on a `- [x]` / `- [ ]` line carrying no recognisable uid instead of silently treating it as an
unticked box.

### 15. A hostile `start`/`end` in a digest payload strands the digest forever, half-expanded

`_calendar_note` quotes and escapes `summary`, `location` and `uid` — with an explicit comment on
why even the uid is escaped — and then writes the two remaining fields raw:

```python
f"  start: {_stamp(entry.get('start'))}\n"
f"  end: {_stamp(entry.get('end'))}\n"
```

`_stamp` ISO-formats a `datetime`/`date`; for anything else it returns `str(value).strip()`. So a
payload entry whose `start` is the string `TBD: ask advisor` — perfectly valid YAML *inside the
digest* — is written back unquoted as `  start: TBD: ask advisor`, which is not valid YAML. `create`
re-reads before writing, so **nothing is corrupted on disk**; instead it raises `ScannerError`,
which unwinds out of `_expand_digest` into `process_approvals`' outer
`except (OSError, ValueError, yaml.YAMLError)` and becomes the generic line
`transition failed: <name>`.

The end state, verified by running `process_approvals` three times over the same vault:

```
--- run 1 --- warnings : ['transition failed: events-digest.md']
              approvals/: ['calendar-event-career-fair.md', 'events-digest.md']
              digest status on disk: 'approved'
--- run 2 --- identical
--- run 3 --- identical
```

So, permanently:

- payload entries **before** the bad one are written (`ev-good-1` → `calendar-event-career-fair.md`);
- the bad entry and **every entry after it** are neither created **nor declined** — so they never
  reach the event ledger, stay eligible, and are re-proposed in future digests, spending approval
  budget again;
- the digest keeps `status: approved`, is re-expanded every run and re-fails every run. It never
  settles and never archives. `existing_source_uids` stops the already-written notes duplicating —
  which is exactly what it is for — and that is also what makes the loop make no progress;
- the only signal is `transition failed: events-digest.md`, twice a day, forever, naming neither
  the entry nor the reason.

**Reachability:** the engine's own producer (`eventemit.emit_digest`) formats `start`/`end` from
real `datetime`s, so it cannot emit this. It is reachable by hand-editing the payload, and by any
future producer that does not format them. Same class as defects 3 and 4 above (`eventledger`
writes `strength` and `uid` unexamined), which is why it is recorded to the same standard.

**Severity: medium.** Not reachable from today's producers, but the failure is total, silent about
its cause, and self-perpetuating.

### 16. `_unusable_reason` judges 5 of the 9 amendable fields

The docstring states the principle plainly: *"Storing a value faithfully is not the same as storing
a usable one."* It then applies it to `due`, `effort_hours`, `slice_hours`, `importance` and
`status`, and exempts `importance_reason` deliberately and correctly.

The three fields **S1 added** to `AMENDABLE_FIELDS` on 2026-08-29 — `course`, `domain`,
`effort_confidence` — fall through every branch and return `None`:

```
domain            to='not-a-domain'       -> None
course            to='cs-999-nonexistent' -> None
effort_confidence to='banana'             -> None
status            to='Active'             -> 'to must be one of active, done, archived'
```

**Severity: low today.** `models.Task` reads `domain` with a default and nothing filters on it, so a
bogus value does not drop a task the way `status: Active` silently does. It becomes a real defect
the moment anything filters or groups by `domain` — which S2's surface work is likely to do.

The underlying fault is process, not logic: the S1 change widened `AMENDABLE_FIELDS` without
widening the validator sitting directly beside it.

### 17. A malformed digest is invisible to the approvals budget

`_proposal_weight` returns `len(payload)` only when `events` is a list, and `0` otherwise:

```
events=[{...}, {...}, {...}] -> 3
events={'a': 1}              -> 0
events=None                  -> 0
events='three'               -> 0
```

A digest whose `events:` is not a sequence therefore weighs nothing in `count_proposals_created`,
so it neither spends nor is charged against `daily_approval_budget`, and `result.events_in_digest`
gains nothing — the digest is invisible in the counts while still sitting in `approvals/`.

`_expand_digest` independently warns `events-digest with no payload` on the same condition, so the
malformation is not silent overall; only its budget treatment is. **Severity: low.** Recorded
because the daily cap is spec §3.3's guarantee and this is the one hole in it.

### What this sweep did NOT find

Recorded because a clean result is only informative if its scope is stated. The amendment path —
`validate_amendment`, `_write_problem`, `_reread_problem`, `apply_amendment` — was read line by line
against the interpreter and **no defect was found**. It is the most hardened code in the repo, and
three specific traps were probed and are all correctly handled:

- an inline `---` inside frontmatter, where `split_frontmatter`'s un-anchored `text.split("---", 2)`
  disagrees with the writer's line-anchored `lines.index("---", 1)` — caught by the
  `after_body != before_body` check (the Rust reader's un-anchored split was fixed on branch
  `fix-literal-dashes`, first in efd10ea: it now closes only on a whole `---` line);
- a leading blank line before the frontmatter — `apply_frontmatter_fields_to_text` raises
  `ValueError`, which `_UNREADABLE` catches;
- `_as_date` on hostile input (`0`, `False`, `[1, 2]`, `{'a': 1}`, `2026-13-45`, `3.7`) — returns
  `None` for every one and raises on nothing, which matters because it runs on every rank.

---

## `engine/coursework.py`, `engine/zybooks.py`, `engine/vhl.py`

Read in full during wave 6 of the Rust port (2026-09-02). Two defects, both confirmed against the
interpreter rather than inferred, plus four latent hazards that are working as designed today and
would fail silently if the vendor's page shape changed.

### 18. The same uid twice in one batch writes TWO lines to `state/ingest-seen.md`

`sync_coursework`'s **update** branch adds the uid to the in-memory `seen` set; its **create**
branch does not:

```python
create(vault, ..., ctx=item_ctx, journal=journal)
log.append(f"created {path.stem}")
known[item.uid] = path
record_seen(vault, item.uid, item.title, stamp)   # <- no seen.add(item.uid)
```

So the second occurrence of a uid in one batch finds `known[uid]` (just set), takes the update
path, evaluates `if item.uid not in seen` as **true**, and appends a second ledger line.
`record_seen` appends unconditionally — it has no dedup of its own.

Verified:

```
log:    ['created cs-100-hw-01', 'updated cs-100-hw-01: title']
ledger: - zybooks:1 · CS 100 HW 01 · first seen 2026-08-25
        - zybooks:1 · CS 100 HW 01 renamed · first seen 2026-08-25
tasks:  ['cs-100-hw-01.md']          (one note, correctly)
```

**Why it is worth recording rather than shrugging at.** `state/ingest-seen.md` carries
`merge=union` in `.gitattributes`. The two lines differ in their title text, so a two-device sync
keeps **both**, permanently, and no later run removes either. Reading still behaves — `load_seen`
splits on `" · "` and keys on the uid — so the outcome is a ledger that accumulates duplicate
entries with a misleading "first seen" title, not a resurrected or duplicated task.

**Reachable?** Only if one batch carries a uid twice: zyBooks listing the same `assignment_id`
under two mapped books, or VHL emitting the same section-and-date twice. Neither has happened.

**Not fixed.** One `seen.add` is exactly the kind of quiet improvement that makes a behaviour diff
impossible to trust during a port. Pinned by
`coursework::tests::preserved_defect_18_a_uid_twice_in_one_batch_writes_two_ledger_lines`.

### 19. `enabled: "false"` — quoted — turns a coursework source ON

`collect` tests truthiness, not equality:

```python
if not source_cfg.get("enabled"):
    continue
```

PyYAML gives `False` for a bare `enabled: false` and the **string** `'false'` for the quoted form,
and a non-empty string is truthy. Verified: with `enabled: "false"` and `enabled: "no"` both
fetchers are called; with the bare forms neither is.

The symptom is the opposite of alarming. There is no warning, no log line, nothing on the page —
a source the config appears to disable authenticates against a third party twice a day, and the
only trace is a `0 assignments parsed` warning if it happens to return nothing.

**Not fixed**, and pinned by
`coursework::tests::preserved_defect_19_a_quoted_false_enables_the_source`. Post-cutover the right
repair is not `== True` but a config validator that rejects a string where a boolean belongs, since
the same shape reaches `events.yaml` and `runners.yaml`.

### Four latent hazards — working as designed, and silent if the page shape moves

None of these is a bug today. They are recorded because each one fails **quietly**, and three of
them fail in the direction spec §7.2 was written to prevent.

1. **`engine/vhl.py:_ATTR` only matches quoted attribute values.** The pattern is
   `([\w-]+)\s*=\s*(["'])(.*?)\2`. An unquoted `name=lt` in the login form is invisible to it, and
   the field then vanishes from the POST body — which is precisely the silent HTTP-200 login
   failure the generic form scrape exists to prevent. Rails quotes its attributes today.
2. **`engine/vhl.py:_INPUT_TAG` cannot cross a `>`.** `<input\b([^>]*)/?>` truncates at the first
   `>` inside an attribute value, so `value="a>b"` loses everything after it. No current field
   contains one.
3. **`_SUMMARIES` / `_SCHOOLS_PAYLOAD` stop at the next same-kind quote.** A single-quoted
   attribute whose JSON contains a raw apostrophe truncates, the JSON no longer parses, and
   `parse_dashboard` raises `NotLoggedIn`. That is the **safe** direction — a session failure, not
   an empty semester — and it is worth knowing that is why, because "VHL session invalid" would be
   a misleading diagnosis for an apostrophe in a course title.
4. **`engine/zybooks.py:parse_assignments` truncates fractional points.**
   `int(s.get("total_points") or 0)` makes a 7.5-point section count 7. The number reaches only the
   note body and `importance_reason`; nothing computes with it.

### What this sweep did NOT find

- **No path where an empty parse becomes an empty semester.** `_require_success`, `collect`'s
  `0 assignments parsed` guard, and `parse_dashboard`'s four `NotLoggedIn` raises were each traced
  to a caller. The rule holds on every branch.
- **No path where `progress` is written after creation.** Checked in both the vendor-refresh and
  title-only branches, and pinned by two tests on each side.
- **No secret reaching a warning line.** `_scrub` covers the raw, percent-encoded and
  JSON-escaped forms; `_open_json` raises `from None` so no chained traceback re-prints the
  original message; the VHL password never enters a URL. Two ported tests assert the absence.
- **No unbounded read.** Both engines read whole responses, which is correct for these payload
  sizes (7 KB and 12 KB captured) and is not worth a streaming rewrite.

### Port deviations recorded in wave 6 (not Python defects)

- **`ureq` renders a malformed URL as `"http: invalid format"` and does not quote the URI**, where
  urllib's `InvalidURL`/`ValueError` embed the whole token-bearing URL. Measured. So the Rust scrub
  has nothing to redact on that path today — and runs unconditionally anyway, because the next
  transport release is not this port's to audit.
- **`vhl` scrubs its transport errors; the Python does not.** The password only ever reaches the
  POST body, which no urllib message quotes, so Python is safe without it. Deliberate,
  one-directional, and taken because this tree auto-pushes within minutes of a warning being
  written.
- **`date.fromisoformat` accepts more than `YYYY-MM-DD`** since Python 3.11 — the basic form
  `20260901` and ISO week dates. The port reads only the extended form, so a VHL bucket dated in
  the basic form would warn and skip where Python parses. VHL has only ever emitted the extended
  form.
- **`html_escape::decode_html_entities` does not implement legacy semicolonless references** (a
  bare `&amp` with no `;`) or the Windows-1252 C1 replacements that Python's `html.unescape`
  applies to `&#128;`–`&#159;`. Rails emits well-formed entities; recorded because the failure would
  be a JSON parse error read as a dead session.
- **Warning *text* differs where Python interpolates an exception's own message** — an unreadable
  due date, a non-mapping config value, a missing `session.auth_token`. The control flow is
  identical (same item skipped, same source failed); only the parenthetical differs. None of these
  strings reaches `today.md`; they reach `runner-log.md`.

## `engine/calfeed.py`, `engine/events.py`, `engine/eventfeed.py`, `engine/eventfilter.py`, `engine/eventroster.py`, `engine/eventemit.py`, `engine/runs.py`, `engine/info.py`, `engine/issues.py`, `engine/cli.py`

The Task 17 sweep (2026-09-02): the waves 4–7 modules that had no deliberate re-read on record,
2,023 lines, each read whole, every suspect probed against the interpreter before it was written
down. Three numbered defects, a second site for #19, one new cross-cutting trap, and — the part
that mattered most — four places where the **port** had drifted from the Python in the silent
direction, all closed the same day with tests pinned to Python's measured output.

### 20. A scalar `calendars:` value crashes every run; a string warns once per CHARACTER

`load_calendar_events` does `feeds = config.get("calendars") or []` and then `for feed in feeds`,
with the per-feed `try` *inside* the loop. So a string iterates one character at a time and each
character fails `.get` inside the try — `calendars: "https://x/y.ics"` is **15 warnings**, all
`calendar: bad feed entry ('str' object has no attribute 'get')`, and the snapshot is rewritten
from the previous one (verified). A mapping iterates its keys, one warning per key naming the
key's type (`'int' object …` for `{5: x}`; verified).

A scalar is worse: `calendars: 5` (or `true`, or `2.5`) raises `TypeError: 'int' object is not
iterable` from the `for` itself, which is **outside** every try in the function — and
`cli.run` calls `load_calendar_events` outside the events pass's `try`, so the exception reaches
the crash handler: a FAIL run record, a FAIL log line, **no `today.md`**. Verified for all three
scalars. A one-character config typo kills every rank until someone reads the log.

**Not fixed.** Pinned by `calfeed::tests::preserved_defect_20_a_string_calendars_value_warns_once_per_character`
for the warning half. The crash half is a recorded port divergence: the Rust returns **one**
warning, `calendar: bad calendars value ('int' object is not iterable)`, and leaves the previous
snapshot untouched — the same class of choice as `load_runners_config` returning `Result` — pinned
by `a_scalar_calendars_value_is_one_warning_here_where_python_crashes_the_run`. Until this sweep
the port returned **silently** for every non-list shape: no warning, no snapshot rewrite, the
calendar simply gone from the page. That was the finding.

### 21. A multi-line calendar title is truncated by the snapshot round trip

`unescape` turns an ICS `SUMMARY:Line one\nLine two` into a title with a real newline (verified:
`'Line one\nLine two'`). `write_snapshot` writes the title verbatim, so the event occupies two
physical lines of `state/calendar.md`; `read_snapshot` matches lines one at a time, so the second
line matches nothing and the event reads back as `Line one` (verified). Busy time survives the
fallback path intact; the title's tail does not. Google Calendar rarely emits a newline in a
summary, which is why nobody has seen it.

**Not fixed**, identical in both engines, pinned by
`calfeed::tests::preserved_defect_21_a_multi_line_title_is_truncated_by_the_snapshot_round_trip`.

### 22. `issues address` accepts an open INFO item

`address_issue` resolves its target by id or path, then checks `meta.get("status") != "open"` —
and never `type == "issue"`. An open info item is `status: open` too. So `issues address
info_… --resolution x` stamps `status: addressed`, `resolution`, `addressed_at` and
`addressed_in` onto an **info** note and archives it: exit 0, `addressed -> …`, note in
`archive/`. Verified through both CLIs on scratch copies of the migrated fixture; both agree.

**Not fixed**, pinned by `issues::tests::preserved_defect_22_address_accepts_an_open_info_item`.
Nothing calls `issues address` with an info id today.

### 19, second site: `events.yaml`'s `enabled:` is truthiness too

`load_events_config` builds `EventsSource(enabled=bool(entry.get("enabled")))`. Verified:
`"false"`, `1` and `[1]` are ON; `0`, `""`, `null` and `[]` are off. Same defect, same post-cutover
repair (a validator that refuses a string where a boolean belongs). Pinned by
`events::tests::preserved_defect_19_applies_to_events_yaml_too`.

**This one was a port drift, and the dangerous kind.** `events.rs` accepted only a bare `true`
until the sweep — so a source spelled `enabled: 1` would have been ON in Python and OFF in Rust,
and event discovery from it would have died at cutover with nothing on the page to say so.
`config/events.yaml` spells all six bare, so no live impact; the point is that the oracle could
not have caught it.

### Latent hazards — working as designed, and silent

- **`never:` terms are substring matches** over title + organizer + categories, lowercased. A
  short term (`art`) drops `Startup Week` and every `Party`. There is no word-boundary version;
  `_audience_excluded` has one and `never` does not.
- **`%a` is locale-dependent in Python** — the roster's `### Wed 2026-09-02`, the digest's `Wed
  9/2 13:00` stamp, the snapshot. The port hard-codes English. Identical on this en-US machine;
  a non-English locale would put a different word on the page from each engine.
- **ICS event sources get no RRULE expansion.** `eventfeed.parse_ics_events` reads `DTSTART`
  only; a weekly club meeting is discovered once, on its first date, and once that date is past
  the pre-filter drops it forever. `calfeed` expands RRULE; `eventfeed` does not.
- **Localist and Engage events carry no `end <= start` guard** where the ICS branch has one, so a
  feed emitting an end before its start passes through with a nonsense span (and can be dropped
  as "past" by the pre-filter while it has not started).
- **An all-day ICS event with no `DTEND`** becomes a one-hour midnight event in `eventfeed`
  (`start + 1h`), not a day.
- **Run ids have second resolution** (`<runner>-<ts[:19]>Z`). Two runs of one runner inside one
  second share an id; the dual-run scripts mint the same id from both engines routinely.
- **`runners.yaml` typos crash `runs status` in Python** — `times` as a string iterates its
  characters into `int("1")`, a non-numeric `grace_minutes` is a `ValueError` — where the port
  returns `Result` (already recorded). `rank` is unaffected either way.
- **An interests list item that is `null`** becomes the term `None` (Python's `str(None)`), and a
  bare `true` the term `True`. Verified; the port now spells them the same way.

### What this sweep did NOT find

Recorded because a clean result is only informative if its scope is stated.

- **`calfeed`'s recurrence generator** — `COUNT` and `EXDATE` interplay (RFC-correct: exdates
  count against `COUNT`), `BYDAY`/`WKST` handling, `INTERVAL=0` and negative intervals (harmless:
  the set dedups or nothing is generated), `_rule_expired`, and the DTSTART-not-matching-BYDAY
  case (RFC says DTSTART is always an occurrence; Python omits it; Google never emits it). All
  probed, none observable in live data, nothing beyond what the wave-4 port already recorded.
- **`eventemit`** — the one-digest-per-day guard, the `pending_digest_uids` window, the
  `expires` = earliest event day rule, `select_for_digest`'s target/ceiling reconciliation. Read
  line by line; consistent with spec §5 and with each other.
- **`eventroster`** — the lossy read-back contract is exactly as its docstring says; the
  audit-window bounds are asymmetric on purpose and the comment explains why.
- **`runs`** — `git_sha`'s operator precedence (correct as intended), `_due_times` across a DST
  transition (ZoneInfo handles the fold), `end_run`'s two-day fold window, the `crashed` vs
  `late` rule against `CLAUDE.md`'s definition. Clean.
- **`info` and `issues`** — `kind_for` returns a closed set of five, so `KIND_NAMES[kind]`
  cannot raise; `info_pass`'s expiry is exclusive of today like approvals; the fenced-body
  splitting in `list_issues` is anchored by the frontmatter opener. Clean beyond #22.
- **`cli`** — the problems/summary assembly order, the passes counters, the crash handler's
  ordering (`now`, `today`, `start_run` all before the `try`, so a bad `--today` writes nothing).
  Clean beyond the exit-code note below.

### Port deviations recorded in Task 17 (not Python defects)

- **Exit code for a refused `--run-id` or `--today`.** Python's `main` catches `ValueError` and
  hands it to `parser.error`: exit **2**. The port exited 1 for every `RunError`; now
  `RunError::exit_code` is 2 for the two validation refusals and 1 for everything else. A
  `ValueError` raised *deeper* inside `run` would still be 2 in Python and is 1 here.
  `scripts/local-run.ps1` tests only for non-zero. Measured.
- **A scalar `calendars:` is a warning, not a crash** — see #20.
- **`cargo-bloat`-adjacent:** none. The audit is in the build report.

## Cross-cutting traps (not defects — Python working as designed)

### `json.dumps` uses non-minimal separators

Python's `json.dumps` defaults to `", "` and `": "` — **with spaces**. `serde_json` emits the
minimal form. Every journal and run record is written with `json.dumps(..., ensure_ascii=False,
sort_keys=True)`, so a default serde_json writer produces byte-different lines for identical data.
On `merge=union` ledgers that means the same logical record can exist twice after a sync.

`src/ledger.rs` handles it with a custom `Formatter` and applies `sort_keys` recursively rather
than relying on `serde_json::Map` being a `BTreeMap` (which flips if anything ever enables the
`preserve_order` feature). **Any future module that writes JSON must use `ledger`'s writer, not
`serde_json::to_string`.**

### `serde_yaml_ng` accepts unknown YAML tags that PyYAML REJECTS — and it can change the page

Found 2026-09-01 during wave 5, by a ported test that would not go green. **This is the first
cross-cutting trap that is demonstrably capable of putting a task on `today.md` that Python leaves
off**, which makes it the most consequential entry in this file after CRLF and the timestamp gap.

PyYAML's `SafeLoader` constructs only the standard tags (`!!str`, `!!int`, `!!float`, `!!bool`,
`!!null`, `!!seq`, `!!map`, `!!timestamp`, `!!binary`, `!!set`, `!!omap`, `!!pairs`). Anything else
— an unknown suffix in the `tag:yaml.org,2002:` namespace, or any local `!tag` — raises
`ConstructorError`, which is a `yaml.YAMLError`, which every reader in `engine/` catches as
*unreadable*.

`serde_yaml_ng` does not. Verified against both:

```
                            serde_yaml_ng                    PyYAML
"strong: !!invalid"      => Ok({"strong": String("")})       ConstructorError
"b: !!nope x"            => Ok({"b": String("x")})           ConstructorError
"strong: !!str 5"        => Ok({"strong": String("5")})      {"strong": "5"}      (agree)
"strong: !custom v"      => Ok({"strong": TaggedValue{..}})  ConstructorError
```

An unknown `!!`-namespace tag is **silently resolved away** — the tag vanishes and the value is
coerced to a string. A local `!tag` at least survives as a `TaggedValue` and is detectable after
the fact; the `!!` case is not, because nothing about the tag remains in the parsed value.

**Why this is not merely cosmetic.** Run the same note through `load_tasks` in both engines:

```
tasks/t.md:  ---
             id: abc
             title: Tagged
             status: active
             due: !!weird 2026-09-01
             effort_hours: 2.0
             ---

PYTHON   tasks: []          skipped: ['t.md']        -> "1 unreadable: t.md" in the run summary
RUST     tasks: ["Tagged"]  skipped: []              -> due = 2026-09-01T23:59, ON THE PAGE
```

So the two engines disagree about **which tasks exist**, and about the unreadable count in the run
summary. That is a `today.md` difference, i.e. exactly what the oracle exists to catch — and the
oracle is green only because no fixture, and almost certainly no note in the live vault, contains
a YAML tag.

**Why it was NOT fixed here.** A faithful fix has to reject unknown tags before the value is built,
and neither available route is safe inside a behaviour-preserving port:

- *Post-parse detection* catches only the `!tag` half. The `!!` half is already gone by then, so
  the fix would be half a fix, which is worse than none — it would look handled.
- *Textual scanning of the frontmatter* for tag tokens is fragile in the direction that does real
  damage. `title: Hello !!world` is a plain scalar, not a tag, and a scanner that got that wrong
  would mark a **valid** note unreadable and silently drop a real task. Trading a divergence
  nobody can reach today for a false positive on ordinary notes is a bad trade.

**Status: ACCEPTED by decision, 2026-09-02 (Quinn; cutover plan ruling C8).** Not fixed in code.
`scripts/lint-yaml-11.py` — PyYAML's scanner over every note frontmatter, fenced `task` payload
and config file, read-only — reports any explicit tag and runs on the live vault every dual run
of the cutover weeks and as a standing check afterwards. **Reachability at acceptance: 0 tags in
238 documents.** The `#[ignore]`d test stays, its attribute names this decision, its assertion is
still Python's. **At the cutover close (2026-09-09; the plan is DONE):** the lint reported
`0 scalars, 0 tags` on every dual run of both weeks — `lint=0/0` on every `python-live` line (the
forced first run and all ten executed), both `rust-live` lines and all six `reverse-proxy.ps1`
cycles — so the trap was never reached on live data; the lint stays in `scripts/` as a standing
check, and `events::tests::malformed_interests_warns_and_stays_empty` stays `#[ignore]`d by name.
The paragraph below is the reasoning as it stood before the decision.

*Was:* open, and it belongs to cutover, not to wave 5. The decision to take before the dual
run is one of: accept it (documented, with a note that tags never appear in this vault), swap the
YAML crate for one that surfaces tags, or pre-scan with a parser rather than a regex. The dual run
against the live vault will also settle the reachability question empirically — if no note in the
real vault carries a tag, the risk is theoretical and accepting it is defensible.

**Visible marker in the code:** `events::tests::malformed_interests_warns_and_stays_empty` is
`#[ignore]`d with this trap named in the attribute, rather than rewritten to assert the Rust
behaviour. Encoding a divergence as correct is how it gets forgotten. `cargo test -- --ignored`
lists it.

---

### PyYAML resolves YAML **1.1** timestamps; serde_yaml_ng follows YAML **1.2**, which has none

Found 2026-09-01 while porting `render.py` and `passes.py`. Verified against the interpreter and
against the Python-written journal in `tests/fixtures/vault-s1-migrated/`.

This is the second-most dangerous port hazard after CRLF, and unlike CRLF it is **silent in both
directions**: nothing errors, a value simply arrives as a different type.

PyYAML resolves `2026-10-09T13:00:00` to a `datetime` and `2026-09-01` to a `date`. serde_yaml_ng
hands both over as `Value::String`, because YAML 1.2's core schema dropped the `!!timestamp` tag.
So **every Python branch of the form `isinstance(value, (datetime, date))` is dead in the port
unless the distinction is re-derived from the text.**

Two exact facts, both verified rather than reasoned:

1. **PyYAML's timestamp pattern requires seconds.** `2026-10-09T13:00` is a *string* in Python too.
   That single detail is why the port is safe today: the live vault and both fixtures spell every
   `due` as `YYYY-MM-DDTHH:MM` or a bare `YYYY-MM-DD`, and for those two shapes the two engines
   produce identical text.
2. **A timezone offset is kept, not applied.** `yaml.safe_load` returns
   `datetime(2026, 10, 9, 13, 0, tzinfo=-05:00)` — the wall clock is untouched — so `_plain`
   formats it `2026-10-09T13:00`. A port that shifted to UTC would move every offset-bearing
   proposal's deadline by hours, in the direction that makes a deadline look later than it is.

Where it lands in the port:

- `approvals::plain` transcribes PyYAML's resolver regex (`PYYAML_TIMESTAMP`) and formats the match
  through `format_due`'s spelling. Without it `as_note_datetime` returns `None` for an
  offset-bearing value where Python returns a naive datetime, and the `⚠ PENDING AMENDMENT` block
  silently loses its `due …` line — which is the one line that says how much earlier the deadline
  moved. `tests/test_render.py::test_same_day_mixed_timezone_amendment_does_not_crash_render`
  exercises exactly this path.
- `yaml::to_json` (the port of `journal.jsonable`) deliberately does **not** normalise, and the
  distinction matters: it feeds `detect_external`, which compares its output against Python-written
  journal records. A unilateral normalisation there would invent a `quinn`/`external` edit on every
  note whose `due` it touched, and judge-once would then lock the field.

**Not a Python defect, and not fixable by choosing a side.** serde_yaml_ng also cannot report
whether a scalar was *quoted*, so `"2026-10-09T13:00:00"` (a string to PyYAML) and
`2026-10-09T13:00:00` (a datetime) are indistinguishable in the port. `plain` normalises both;
Python normalises only the second. Unreachable in the vault, which quotes no timestamps.

**After cutover this becomes a one-line decision to make once:** either the vault standardises on
minute-precision naive timestamps and the resolver transcription can be deleted, or a proposal
producer is allowed to emit offsets and the transcription becomes load-bearing forever. Do not
leave it undecided — the whole hazard is that both engines look right until a source changes its
spelling.

**At the cutover close (2026-09-09): still open, deliberately.** Cutover plan ruling C9 named this
the first post-cutover decision and did not take it. The plan's Task 10 docket that was to carry it
was superseded by the Knowlu plans — see `docs/HANDOFF.md`'s **✅ CUTOVER CLOSED** block — which
picked up its first two items (profiles and onboarding, the local scheduler) and not this one. Until
it is taken, `approvals::PYYAML_TIMESTAMP` stays load-bearing in the port.

### PyYAML resolves YAML **1.1** scalars beyond timestamps — `yes`/`no`, `1_000`, `0x10`, `1:30`

Found 2026-09-02 by the Task 17 sweep, when a test asserting `enabled: no` is OFF went red: PyYAML
reads `no` as `False`; `serde_yaml_ng` (YAML 1.2) reads it as the string `"no"`, and a non-empty
string is truthy — so `enabled: no` is OFF in Python and **ON** in the port. Measured against the
interpreter, the full 1.1 set that 1.2 dropped: `yes`/`no`/`on`/`off` in any capitalisation are
booleans; `1_000` is 1000; `0x10` is 16; **`1:30` is 90** (sexagesimal); `.inf` is a float;
`Null`/`NULL` are null. `y`/`n` and `0o17` are strings in both. Same family as the timestamp trap
above, and it reaches every YAML read — a task titled `yes` renders as `True` in Python.

**Reachability, measured 2026-09-02:** a read-only scan of the live vault and configs with
PyYAML's own composer (so each scalar's raw spelling sits next to the tag it was given) —
`tasks/`, `approvals/`, `archive/`, `courses/`, `issues/`, `info/`, `config/`, `profile/`,
fenced `task` payloads included, **236 YAML documents** — found **zero** 1.1-only scalars outside
the **111 timestamps** the trap above already covers. Nobody has written `yes` or `1_000` into
a note or a config.

**Status: ACCEPTED by decision, 2026-09-02 (Quinn; cutover plan ruling C8), with the same lint as
trap 4** — `scripts/lint-yaml-11.py` composes every document with PyYAML and reports any plain
scalar the resolver typed as bool/int/float from a spelling YAML 1.2 would read as a string
(`yes`/`no`/`on`/`off`, `1_000`, `010`, `0x10`, `1:30`, a float with `_` or `:`), timestamps
counted separately. **Reachability at acceptance: 0 scalars, 117 timestamps, 238 documents.**
Shown failing on a planted vault before it was trusted (8 scalars, 2 tags, each named by line).
**At the cutover close (2026-09-09; the plan is DONE):** `lint=0/0` on every dual run of both weeks
— the same lines as trap 4's, above — so no 1.1-only scalar ever reached either engine from live
data; `events::tests::trap_5_a_yaml_1_1_boolean_spelling_is_read_as_python_reads_it` stays
`#[ignore]`d by name, its assertion untouched.

*Was:* deliberately unfixed, same treatment as trap 4: settle it before cutover, not in code. The
suite's second `#[ignore]`d test, `events::tests::trap_5_a_yaml_1_1_boolean_spelling_is_read_as_python_reads_it`,
says what "settled" means for the one spelling a hand-written config is likely to use. The
cutover-week dual run will confirm reachability stays at zero; the post-cutover options are the
same three as trap 4's (accept, swap the YAML crate, or pre-scan), and a config validator that
refuses `yes`/`no` closes the likeliest door at no cost.

### CRLF

Recorded here because it is the port's most dangerous non-obvious behaviour, though it is Python
working as designed rather than a bug.

Every file in the repo is CRLF. `Path.read_text()` applies universal-newline translation and
`write_text()` / `open(mode="a")` translate back to `os.linesep`, so Python round-trips CRLF
invisibly. Rust must do it explicitly. Full detail in `CLAUDE.md` and in the waves 0–3 plan's
Global Constraints. Verified across `tasks/*.md`, `state/today.md`, `state/journal/*.jsonl`,
`state/events-seen.md` and both fixture vaults.
