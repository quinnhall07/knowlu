# M2 editing: task bodies and the student's profile, from the app — design

**Status: Signed by Quinn, 2026-09-29 (every recommendation accepted).** Q1–Q8 in §12 are decided
as recommended: Q1 (A), Q2 (A), Q3 (A), Q4 (A), Q5 (A), Q6 (A), Q7 per Appendix A ((a) cut, (b) cut
from the MVP, (c) delivered by the events spec's D3, (d) done), Q8 (A).
Written 2026-09-29 on branch `mvp-specs` at main `97dc27b`.
**Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` and its *Amendment 2026-09-29* (ruling 10:
the MVP holds "body and profile editing, each small parity item done or cut"; ruling 11: the human
actor token). **Implements:** parity rows P5 and P6 of `docs/notes/2026-09-29-vision-program.md`, and
row 28 of `docs/surface/inventory.md` ("Note body editing — tasks, courses, profile"). **Starts from:**
W1's research (`.superpowers/sdd/2026-09-29-ultracode/w1/research-m2-p6.md`), every file:line claim
re-read against `97dc27b`; §0 lists what changed.

## 0. What the code does today (read at `97dc27b`)

- **The body is append-only.** `write::append_body` (`engine/src/write.rs:570`) journals an
  `append_body` record whose `new` is the plain line (`:597-603`), then calls `append_body_line`
  (`:104`), which finds the body textually after `---\n…\n---\n` and appends one `> ` line. Nothing
  replaces a body. Its only caller outside tests is an approved amend card's `append:` list
  (`approvals.rs:778-780`).
- **The journal.** `journal::OPS` (`journal.rs:27`) is `set, create, delete, move, append_body,
  supersede`; `make_record` refuses any other op (`:156-159`). Judge-once reads only `set` and
  `create` records (`latest_by_field` `:190-214`, `human_set` `:271-282`, `human_edited` `:289-295`).
  `passes::verify_tail` re-applies only `set` records (`passes.rs:184-244`); `detect_external`
  journals frontmatter differences only, so a hand edit to a body is never journalled
  (`sync.rs:1893-1896` says so).
- **Reading a note.** `models::split_frontmatter` (`models.rs:70`) returns an empty mapping and the
  whole text for a file with no frontmatter (rule 3, `:67`), tolerates trailing spaces on the closing
  line, and strips every leading newline from the body (rule 4). The writer does not tolerate the
  trailing spaces: `ingest::apply_frontmatter_fields_to_text` (`ingest.rs:81-115`) needs a first line
  starting `---` and a closing line exactly `---`, and replaces **one line** per key.
  `write::load` (`write.rs:195`) therefore refuses only broken YAML, not a missing block.
- **Writing a file.** `pystr::write_text` (`pystr.rs:90`) is `std::fs::write` after translating `\n`
  to the platform newline. No write path is atomic; set_body is no worse.
- **The drawer.** `console.js:1082-1103` (`openDrawer`) renders `n.body` as a read-only `<pre>` at
  `:1094`, and only when the body is non-empty. `n.body` is `surface::note_detail`'s body
  (`surface.rs:1773`), which is `split_frontmatter`'s (leading newlines stripped). Fields edit in
  place through `data-field` and `commitEdit` (`:1202`) → `set_fields`.
- **The app.** `commands::set_fields_inner` (`commands.rs:207-227`) goes through
  `write::write_literals` with `console_ctx()` (`:156-159`, actor `quinn`, via `dashboard`) inside
  `mutate` (`:179-194`), which takes `vault_io` then `cs.lock` and always returns fresh state. New
  tasks are written as `---\n<yaml>---\n\n` (`:255`): the body starts after one blank line. The
  console's `generate_handler!` list is `app/src/main.rs:177` (47 commands at `97dc27b`); the wizard's
  is `:107`.
- **Sync.** Every journal record this device writes, and the whole text of every changed note under
  `ids::NOTE_FOLDERS`, goes to the account (`sync.rs:3-6`). A record over 16 KiB stays unsent
  (`MAX_RECORD_BYTES`, `:43`); a note over 128 KiB or containing NUL stays unsent (`:725-732`). The
  server checks only that `op` and `actor` are present (`_shared/sync_rows.ts:90`) and keeps a
  record past 400 days only when it is a human `set` or `create`
  (`migrations/20260912000300_sync_plaintext.sql:65-70`). A pulled record naming a path outside the
  note folders is refused by `apply` (`sync.rs:1790-1795`) and by restore (`:983-987`). A pulled note
  text never overwrites a note this device already has (`:2284-2296`).
- **The profile.** `profile/preferences.md` is free text; `judge` reads it whole and clips it to 600
  characters (`judge.rs:36-37`, `:414-415`), and it travels in every `/judge-task` request
  (`cloudmodel.rs:363`). `profile/interests.md` has four frontmatter lists (`strong`, `mild`, `never`,
  `clubs`) that `events::load_interests` reads for the prefilter (`events.rs:313-348`); its **whole
  text**, clipped to 600 characters, also travels to the event judge (`events.rs:431-433`). The one
  real example, `engine/tests/fixtures/vault-full/profile/interests.md`, writes those lists in block
  style (`strong:` then `  - …` lines). `profile/` is not in `NOTE_FOLDERS` (`ids.rs:20-21`), so its
  files have no id, never sync as notes and are reachable only by path (`resolve_target`,
  `ids.rs:265-278`). Nothing in `app/` reads or writes them; the wizard does not create them.
- **Course notes.** A course note's `## Grade weights` section is what grounds a task's importance in
  the judge (`judge.rs:409`). Editing a course body in the app is how a student states weights until
  syllabus upload (Beyond).

**Corrections to the research.** Line numbers had drifted (write.rs by about six lines; the drawer
`<pre>` is `console.js:1094`, not `:744`). `load()` accepts a file with no frontmatter. Editing
interests "as four list editors via `write_literals`" is **unsafe as stated**: single-line surgery
on a block-style list replaces the `strong:` line and orphans its `  - …` lines, which breaks the
YAML (D10). `write::create` would stamp `id: task_…` on a profile file, because `ids::kind_for`
defaults to `task` (`ids.rs:97`) (D11). P6(b) is no longer blocked on the commitment model:
`commitments::conflicts` and `fit` are on main (`commitments.rs:7839`, `:7903`). P6(c) does **not**
hold today. A digest can outlive 14 days when a registration deadline opens its horizon early
(Appendix A), and the events spec's D3 already replaces the digest. The interests text reaching the event judge is
not named on the privacy page, and because no app-made vault has `profile/`, M2 is the first path
by which a student's typed interests reach a model at all (§5, Q8).

## 1. The student-facing goal

A student opens a task and writes down what they know about it: what the assignment asks, the
page numbers, the professor's hint, the step they stopped at. They open a course and paste its
grade weights. They tell Knowlu, in their own words, how they like to work and which campus events
they care about. All of it happens in the app, with no text editor and no file path.

This serves **"what's next?"** in three ways. A task's body is where the student keeps the context
that makes the next step obvious once it reaches the top of the list. A course's grade weights
decide how much a task matters, which moves the order. Preferences and interests are the only
place the student's own voice enters the judge and the campus-event filter, so they decide which
work is sized how and which events are worth showing at all. Today every one of these needs a text
editor on the vault folder, which a pilot student will never open (VISION: onboarding asks for
sign-ins, "nothing else"; success 6: a second student needs nothing from anyone).

VISION commitment 5 applies: an edit is "the student asked", so it **acts at once, with undo**.

## 2. Stage

**MVP**, by ruling 10 of the Amendment 2026-09-29: the MVP holds "body and profile editing, each
small parity item done or cut". It is proven on the founder's scratch profile against staging, and
its exit needs the parity audit re-run with no open row, so P5 and every P6 item must be marked done
or cut by Quinn (Appendix A). Nothing here needs production, a release, a second person or a second
desktop. Two-desktop body merging is Launch's (§7.5).

**Disclosure before any non-founder.** M2 makes the interests editor the first way a student's
typed interests reach a model (§5). No non-founder may use it until privacy bump #1 has shipped
with the interests sentence drafted in §5. Ruling 10's Pilot gate already says that no non-founder
account exists before bump #1, so this costs the MVP nothing. What it adds is a condition on bump
#1 itself: its page must carry that sentence (Q8).

## 3. Decisions

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| **D1** | A new write primitive, `write::set_body`, replaces everything after the closing frontmatter fence and nothing before it. Journal record first, file second. No note is parsed and re-dumped: the frontmatter is carried over as text. | The one write path must stay complete by construction (`write.rs:1-11`); a body edit outside it would be a silent, unattributed change. | A wrong fence rule would overwrite frontmatter; D4's refusals and the byte tests (§11) guard it. |
| **D2** | A new journal op, `set_body`, added to `journal::OPS`. The record's `field` is `null`. | Reusing `op: set, field: body` would enter judge-once (`journal.rs:207`, `:277`), `verify_tail` (which would re-apply a body as one frontmatter line) and `reconcile::resolve`. A distinct op is ignored by all three. | Older builds refuse the op by name (`unknown op`); MVP has one build and one desktop. |
| **D3** | The record holds **hashes, not text**: `old` and `new` are each `{"sha256": <hex>, "bytes": <n>}` of the body. (Q1.) | Privacy, §5. The note's current text already syncs; a text record would be a second copy that outlives the edit. | No undo from the journal; D8 gives undo without it. |
| **D4** | set_body refuses: a file that opens with `---` and has no closing line exactly `---`; a new body containing NUL; a result longer than `sync::MAX_NOTE_BYTES` (128 KiB); and, on a file with no frontmatter, a new body whose first line starts with `---`. Each refusal is a named error, before any record. | Each is a case where the write would corrupt the file, wedge sync, or turn the body into frontmatter on the next read. | A student with a malformed note sees a named refusal and edits the file by hand. |
| **D5** | **Compare-and-swap.** set_body takes the body the caller last read. If the note's current body differs (an agent appended a line, another window saved, a text editor changed the file), it refuses with a named `Conflict`, writes no record and changes nothing. (Q2.) | A body replace is the first write in the product that can erase text it did not see. `detect_external` never journals a body edit, so the file is the only witness. | A student who reopens after a conflict loses nothing: the page keeps their draft (§9). |
| **D6** | **One definition of "the body"**: what `note_detail` shows, i.e. everything after the closing fence line with leading newlines stripped. The write keeps the file's own run of newlines after the fence (one blank line when the body was empty and there was none). The new body's line endings become `\n`, trailing newlines are trimmed, and one final `\n` is added when it is not empty. A save that equals the current body under that rule writes nothing (the F4 rule of `write_literals`, `write.rs:261-264`). | The drawer, the hash and the file must agree on one string, or the compare-and-swap refuses every save. | A body whose meaning lived in trailing blank lines loses them; none does. |
| **D7** | **Scope of body editing in the app:** notes in `tasks/` and `courses/`, reached by id; the two profile files through their own commands (D11-D12). Approvals, issues, info, commitments and archive are refused by name. The engine primitive itself takes any target `resolve_target` accepts. (Q3.) | Tasks carry the student's working notes; courses carry grade weights, which move the order. An approval's body is the card the deck parses, and a commitment or an issue has its own editor. | Editing another folder later is a one-line allow-list change. |
| **D8** | **Undo in the session, for every M2 editor.** After a save the page shows "Saved · Undo" for 10 seconds. For a body, undo sends a second set_body. Its expected body is **the body the page re-read after the save**, never the textarea's text, and its new body is the text the page held before. Preferences undo the same way through `set_preferences`. Interests undo by sending the four lists the page held before through `set_interests`. Each undo is journalled like any edit. There is no undo after the drawer or the settings panel closes, or after the app restarts. (Q5.) | VISION commitment 5 and Amendment ruling 3: "the student asked → act at once with undo". The page already holds the old text, so undo needs no text in the journal. Using the re-read body, together with §7.1's normalised compare, means D6's added final newline can never make an undo refuse itself. | A student who wants yesterday's text back cannot get it from Knowlu; the local backup snapshots still hold it. |
| **D9** | A body edit changes no frontmatter field and does not queue the task for judging again. | "Judge once" governs fields; a body edit is context, not a request for a new estimate. The student edits `effort_hours` directly when the estimate is wrong. | A task whose body now says "this is a 10-page paper" keeps its old estimate until the student changes it. |
| **D10** | **Interests are edited as four lists**, one item per line, and written as one-line flow lists (`strong: ["a", "b"]`) through a new contract-list primitive, `write::write_one_line_literals`. It refuses by name, before any record, when any key it would replace has a value that continues onto following lines (block style, as the fixture writes it), with the words "Knowlu can only edit a list written on one line". Otherwise it does exactly what `write_literals` does. The detector is `write::value_spans_lines`, the one function `profile::read` also uses for `interests_editable` (§7.2). (Q4.) | Single-line surgery replaces one line (`ingest.rs:98-113`); on a block list it would orphan the `  - …` lines and break the file, so `load_interests` would return empty lists and the event filter would silently change. `guard_block_style` (`provenance.rs:198`) guards `judgment:` only. The check is the only guard against that corruption, so it lives in `write.rs` with contract-engineer at xhigh, not in `profile.rs` (CLAUDE.md: "could a silent error corrupt vault bytes"). Files Knowlu creates are always flow style (D11), so the refusal only meets hand-written files. | A student with a hand-written block list must rewrite it on one line, or Quinn picks Q4 (B). T1 grows by one primitive and one test. |
| **D11** | **An absent profile file is created** by a new `write::create_profile_file`: only `profile/preferences.md` or `profile/interests.md`, only when absent, journal `create` first (id `null`, `new` the frontmatter mapping), file second, **no id stamped**. preferences.md is created with no frontmatter; interests.md with `strong: []`, `mild: []`, `never: []`, `clubs: []`. | `write::create` would mint `id: task_…` (`ids.rs:97`) and refuses a file with no frontmatter (`write.rs:412-416`). The wizard creates no profile, so every app-made vault needs this. | None known; the function refuses every other path. |
| **D12** | **Preferences are edited through set_body.** A preferences.md with no frontmatter is all body, so the whole text is replaced; one a student gave frontmatter keeps it. | The same primitive, the same guarantees, the same hash-only record. | None. |
| **D13** | **Records about `profile/` stay on this computer.** `sync::build_push` skips any journal record whose `path` fails `sync::is_note_path`. (Q6.) | Every other desktop and every restore refuses such a record (`sync.rs:1790-1795`, `:983-987`), so sending it gives the account data no computer can use and prints one refusal line per record on every restore. Data minimisation (VISION, standing rules). | The account's journal lacks a record no reader accepts. A future stream that syncs `profile/` removes the filter in the same change that adds the folder. |
| **D14** | **The app computes nothing** (CLAUDE.md). Reading the profile lives in a new `engine/src/profile.rs`, off the contract list. Hashing a body and checking a list's style live in `write.rs` (D10). `commands.rs` marshals. | The existing rule. | None. |
| **D15** | **Telemetry is unchanged.** The page reuses `edit_started`, `edit_committed` and `edit_cancelled` (`uievents.rs:11-14`) with `object_kind` `body`, `preferences` or `interests`, and never sends text. | No new action means no new data class. | None. |

## 4. Contract-list impact

Files on the contract list that M2 edits, all by `contract-engineer` at `xhigh`, reviewed by
`contract-reviewer` before any push:

- `engine/src/journal.rs`: `OPS` gains `"set_body"` (seven entries). `VIAS`, `make_record`, the
  record shape, `latest_by_field`, `human_set` and `human_edited` do not change.
- `engine/src/write.rs`: `set_body`, `body_sha256` (the one hash function, so the app and tests
  never hash on their own), `create_profile_file`, `value_spans_lines` and
  `write_one_line_literals` (D10), and three `WriteError` variants: `Conflict`, `Body(&'static str)`
  for D4's named refusals, and `MultiLine(key)` for D10's. `append_body`, `write_literals`,
  `create` and every other function stay byte-for-byte as they are.
- `engine/src/sync.rs`: one filter in `build_push`'s record loop (D13), placed with the other
  `continue` filters so it never advances the cursor over an unsent record. `record_is_well_formed`
  reads `journal::OPS` (`:837`) and needs no edit.
- `engine/tests/sync_contract.rs` (a sync test): two pins (§11).

Untouched: `yamlemit`, `yaml`, `pystr`, `ledger`, `ids`, `provenance`, `approvals`, `entitle`,
`wincred`, `reconcile`; `app/src/{credentials,account,updates}.rs`; `engine/tests/fixtures/**`.

**Frozen references.** None is regenerated. No fixture journal holds a `set_body` record, so the
eight Python-written references and the three `surface-today-*.json` references read the same.
`build_state` over the fixtures reads the same: §7.4's new history wording applies only to a
`set_body` record, which no fixture holds, and the P6(d) list, if Quinn keeps it, is its own
command rather than a new key in the state.

**Rule 1, and the ruling-11 lane.** M2 adds no new `"quinn"` literal. Every write uses
`console_ctx()`, and every test reads the actor from `console_ctx()` or, once it exists, from
`journal::HUMAN_ACTOR`, never from a literal. The profile files name no person.

Ruling 11's actor-token work is a separate contract-list lane: Integrate's "rule-1 token work",
`HUMAN_ACTOR` plus the new-vault token, run by contract-engineer at xhigh. It edits three files M2
also edits. It adds `journal::HUMAN_ACTOR` and changes `human_set` and `human_edited` in
`journal.rs` (T1). It changes `console_ctx()` and the scaffold's `"quinn"` fields in `commands.rs`
(T4). It changes the `"quinn"` comparisons at `surface.rs:1495` and `:1725` (T3's file). It also
adds a test that fails on the literal `"quinn"` in non-test code. **Order: the two never run
alongside each other.** M2's branch is cut from a `main` that already carries the ruling-11 lane,
so T1, T3 and T4 start after that lane merges (recommended, since the lane belongs to Integrate and
M2 to the MVP). If Quinn wants M2 first, the lane waits until M2 merges and rebases onto it; its
literal test then covers M2's code too. Either way the controller checks the order with
`git diff --name-only` before dispatching T1.

## 5. Privacy impact

**What reaches the account that did not before.** One `set_body` record per saved body edit, holding
two SHA-256 digests and two byte counts, the note's id and path, the actor, the via and the machine
name every record already carries. The new body itself reaches the account as the note's text, as
every note change does today; the account keeps only the current text. Nothing about `profile/`
reaches the account (D13). Telemetry is unchanged (D15).

**What reaches a model that did not before.** M2 changes no request, but it does change what
students send. A task's body (its first 1,200 characters) already travels when that task is
judged, and the page names it. `judge` and the event judge already read `profile/` when it exists
(`judge.rs:414-415`, `events.rs:431-433`). But no app-made vault has `profile/` (§0), so today no
app user's preferences or interests reach a model. With M2 they do:

- *Preferences* (up to 600 characters) go with each `/judge-task` call. The page already names
  "your stated preferences" (`site/privacy.html:56`).
- *Interests* (up to 600 characters of the file's whole text) go with each event judged. The page
  names neither the interests nor the event listing. **This is a new disclosure, and M2 is what
  makes it real** (the disclosure paragraph below, Q8).

**Why hashes and not text (Q1).**

1. *A second copy.* The note's current text already syncs; the record would duplicate it.
2. *Retention.* A `set_body` record is not a human `set` or `create`, so the server prunes it at 400
   days. A text record would keep, for 400 days, text the student deleted from the note, while the
   page promises the account holds "the current text of each" (`site/privacy.html:92`).
3. *Size.* A record over 16 KiB is never sent. Old plus new text passes that at about 8 KiB of body,
   so long edits would leave the account's journal with holes exactly where the notes are longest.
4. *Reach.* A record's `old` and `new` appear in the drawer's history, the "since I last looked"
   delta (`surface.rs:1692`), the Runs view and issue reports. Hashes keep prose out of all four.
5. *Use.* The hash is what compare-and-swap (D5) and a future two-desktop merge compare.

What a hash still reveals: an unsalted digest of a very short body ("done") can be confirmed by
guessing. What it confirms is text the account held at that time as the note's own text. Accepted;
a keyed hash would stop two desktops comparing bodies, which Launch will want.

**`PRIVACY_VERSION`.** `app/src/account.rs:21-23`: bump the constant and the page's date in the same
commit, and only when the page's text changes. Under the recommended answers M2 changes no page
text. The page already says the account holds "the text of every task and note, and the journal
that records each change" (`site/privacy.html:36`), and a hash-only record is a change in that
journal. So M2 does not move `PRIVACY_VERSION` (it stays `2026-09-24`) and asks for no re-consent.
M2's code may ride any release. It may not reach a non-founder before privacy bump #1 ships with
the interests sentence below. Ruling 10's Pilot gate already guarantees that (§2, Q8). Two answers
would move the page for M2's own sake:

- **Q1 (C), text in the record:** the page must say that the journal keeps the earlier text of an
  edited note for 400 days.
- **Q6 (C), syncing the profile:** the page must name the profile among what the account holds.

Either one moves the page, its Effective date and `PRIVACY_VERSION` together in one PR. Ruling 10
puts the next move in privacy bump #1 (Pilot: one lawyer read, the re-consent screen), so that
option's code could not reach a release before bump #1.

**The interests disclosure, which M2 makes necessary.** The page's list of what travels to a
model (`privacy.html:56`) names "the note's title and up to the first 1,200 characters of its
text, your stated preferences and grade weights". It does not name the interests text that goes to
the event judge (`events.rs:431-433`), or the event listing itself. The code gap predates M2, but
before M2 no app user's interests were ever sent. M2 creates `interests.md` (D11) and makes it
editable, so it opens the path. Drafted for bump #1's packet, never edited into `site/` here:
*"…your stated preferences and grade weights, and, when a campus event is judged, the event's
listing and up to the first 600 characters of the interests you have told Knowlu."* **Condition:**
bump #1's page carries this sentence (or one with the same facts) before any non-founder uses the
editor. When the spec is signed, the controller adds the sentence to bump #1's row in HANDOFF §4.
Until bump #1 the founder is the only user, and the editor's own copy (§9.2) is the disclosure.
That copy's facts are fixed.

## 6. Data model

### 6.1 The body

For a task made by the app (`commands.rs:255`):

```
---\n<frontmatter lines>---\n\n<body>
^ head: kept as text     ^ fence ^ separator: kept    ^ replaced
```

The body the drawer shows, the body the hash covers and the body set_body replaces are one string
(D6). The new body is normalised: `\r\n` and a lone `\r` become `\n`, leading and trailing newlines
are trimmed, and one final `\n` is added when the result is not empty. Written after the head and
the separator, it reads back through `split_frontmatter` as exactly that string, so a record's
`new.sha256` equals the hash of the body the next edit starts from.

A file that does not start with `---` has an empty head and no separator: the whole text is the
body. A file that starts with `---` must have a closing line exactly `---` (the writer's rule,
`ingest.rs:92`), or set_body refuses it.

### 6.2 The journal record

One line through `Journal::append`, so its bytes are `ledger::dumps_value`'s and its keys are
`make_record`'s order:

```json
{"ts":"2026-10-02T14:03:11.120Z","device":"LAPTOP","actor":"quinn","via":"dashboard","run_id":null,"op":"set_body","id":"task_0123456789","path":"tasks/stats-hw-4.md","field":null,"old":{"sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","bytes":0},"new":{"sha256":"<64 hex>","bytes":212},"evidence":null,"seq":1}
```

- `sha256` is lowercase hex over the body's UTF-8 bytes (`write::body_sha256`); `bytes` is that
  byte count. The example's `old` is an empty body.
- `id` is the note's `id:` when it is a valid id, else `null` (the profile files).
- `field` is `null`: no field-keyed reader sees the record (D2).
- The actor is whatever `console_ctx()` gives; ruling 11 decides the token, not M2. **The
  example's `"actor":"quinn"` is illustrative** (it is `console_ctx()` at `97dc27b`). A new vault
  after the ruling-11 lane writes `student`, and no test may assert either literal (§4, test 23).

### 6.3 The profile files

`profile/preferences.md`, as Knowlu creates it: no frontmatter, the student's text, one final
newline. `profile/interests.md`, as Knowlu creates it:

```
---
strong: []
mild: []
never: []
clubs: []
---
```

After an edit each key is one flow line written by the one emitter (`write::to_literal` →
`yamlemit::safe_dump_flow`), e.g. `strong: [undergraduate research, AI talks]`, which
`load_interests` reads as it reads the block form. Items are trimmed, empty items dropped and exact
duplicates dropped; an item containing a line break is refused. The body of interests.md, if a
student wrote one, is never touched.

## 7. The engine surface

### 7.1 `write::set_body`

`set_body(vault, target, expected: &str, new_body: &str, ctx, journal) -> Result<bool, WriteError>`;
`true` when it wrote, `false` for a no-op. In order:

1. `resolve_target(vault, target)` (id or path), then `load(path, false)` so broken YAML is refused
   as it is everywhere (`NoFrontmatter`).
2. Split textually (§6.1). A file that starts with `---` and has no closing `---` line is refused
   (`Body("the frontmatter has no closing line")`).
3. **Compare-and-swap, on normalised bodies:** normalise `current` and `expected` by D6's rule
   (line endings to `\n`, leading and trailing newlines trimmed, one final `\n` when not empty). If
   `body_sha256` differs between the two, refuse with `Conflict(path)`. Nothing is journalled or written. So a
   caller whose copy differs from the file only by a trailing newline, or by CRLF, is not refused
   (test 30). The record's `old` stays the hash of the current body exactly as `note_detail` shows
   it (§6.2, test 6).
4. Normalise the new body. If it equals the normalised current body, return `Ok(false)`.
5. D4's refusals: a NUL; a result over `sync::MAX_NOTE_BYTES`; on a file with no head, a first line
   starting `---`.
6. Build and append the record (§6.2). **Journal first.**
7. Write `head + separator + new body` through `pystr::write_text`. **File second.**

The head's text is untouched. After the universal-newline translation every write path applies, a
CRLF note stays CRLF, byte for byte, through the end of its closing fence line.

### 7.2 `write::body_sha256`, `create_profile_file`, `value_spans_lines`, `write_one_line_literals`

`body_sha256(body: &str) -> String` is SHA-256 through `ring`, already an engine dependency
(`sync.rs:33`). `create_profile_file(vault, name, text, ctx, journal)` accepts only `preferences` or
`interests`, refuses when the file exists (`Exists`), appends a `create` record with `id: null` and
`new` the frontmatter mapping (`{}` for preferences), then creates `profile/` and writes the file. It
never stamps an id (D11).

`value_spans_lines(text, key) -> bool` scans only the frontmatter, by the writer's fence rule
(`ingest.rs:86-92`). It is true when the key's line (the first line starting `key:`, as the
surgery finds it) is followed by any non-blank line before the next line that starts at column 0
with a character other than `-`, `#` or whitespace, or before the closing fence. It is also true
when the key appears twice. **It errs toward true.** A false true only refuses an edit by name,
while a false false corrupts the file. That covers the block list, a list at column 0 (`strong:`
then `- a`), a comment line after the key, trailing spaces after `strong:`, and a blank line
inside the block.

`write_one_line_literals(vault, target, literals, ctx, journal, opts)` reads the file once. It
returns `MultiLine(key)` before any record when `value_spans_lines` is true for any key it would
replace, and otherwise does exactly what `write_literals` does. It is a wrapper, so
`write_literals` itself does not change.

### 7.3 `sync::build_push`

One more `continue` in the record loop, beside the local-card filter (`sync.rs:652`): a record whose
`path` fails `is_note_path` is not sent (D13). It is a `continue`, like its neighbours, so it never
advances `next` past an unsent record.

### 7.4 `surface::describe`

A `set_body` record reads `"{path}: body edited ({who})"` in the drawer's history and the delta,
instead of the generic `"{path}: set_body ({who})"` (`surface.rs:1690`). The record's `old` and `new`
stay in `DeltaRecord`; the page never prints them.

### 7.5 Sync, now and at Launch

With one desktop (MVP and Pilot), a `set_body` record is pushed like any other and the new text
reaches the account as the note's text. Nothing is pulled back, because `/sync-pull` returns only
other desktops' records.

With two desktops (Launch), the record would arrive and be journalled, but the body would not
change: a pulled note text never overwrites an existing note (`sync.rs:2284-2296`), and `apply`
reconciles only `set` records. That is already true of `append_body` and of a hand edit. The
two-desktop stream's spec (`docs/specs/2026-09-25-two-desktop-design.md`) should add: apply a pulled
`set_body` when this desktop's body hash equals the record's `old` (the body did not move here), by
taking the body from the pulled note text; otherwise file a card. M2 builds none of this; the hashes
are what makes it possible.

A crash between the record and the file leaves a record whose `new` the file does not hold.
`verify_tail` does not heal it (it heals `set` only), as it does not heal `append_body` today. The
page still holds the student's text and shows the failure, so nothing is lost.

### 7.6 `engine/src/profile.rs` (new, off the contract list)

- `read(vault) -> Profile`: the preferences body, the four interest lists (through
  `events::load_interests`, so the app and the event filter read the same lists),
  `interests_editable` (false when `write::value_spans_lines` is true for any of the four keys;
  `profile.rs` has no detector of its own), and warnings (an unreadable file is a warning, never an
  error).
- `set_preferences(vault, expected, text, ctx, journal)`: `create_profile_file` when the file is
  absent, then `set_body`.
- `set_interests(vault, lists, ctx, journal)`: `create_profile_file` when absent; clean the items
  (§6.3); then one `write::write_one_line_literals` call with the four keys in the order `strong`,
  `mild`, `never`, `clubs`. That primitive makes D10's refusal, so the refusal holds even if
  `profile.rs` is wrong. Like every field edit today, this is last-writer-wins: only the student
  writes these lists.
- `pub mod profile;` in `engine/src/lib.rs` is a controller hand-off (HANDOFF §2: single owner).

## 8. The app's commands

Four commands, in `app/src/commands.rs` beside `set_fields`, each an `_inner` function that
`app/tests/commands.rs` calls directly and a thin `#[tauri::command]` wrapper, all four in the
console's `generate_handler!` list (`app/src/main.rs:177`) and none in the wizard's (`:107`). The
list holds 47 at `97dc27b`. Gmail connect merges first and adds three (its spec: 47 to 50). With
M2's four it then holds 54, or 55 with P6(d)'s command. **Recount at merge before quoting a
number.**

| Command | Arguments | Does | Returns |
|---|---|---|---|
| `set_body` | `view`, `id`, `expected`, `body` | Refuses unless the note is in `tasks/` or `courses/` (D7), then `write::set_body` with `console_ctx()` inside `mutate`. | `mutate`'s envelope `{ok, error, state}`, plus `conflict: true` when the refusal was a `Conflict`, so the page can keep the draft. |
| `profile` | — | `profile::read` under `cs.lock`, like `note`. | `{ok, error, profile}` |
| `set_preferences` | `view`, `expected`, `text` | `profile::set_preferences` inside `mutate`. | as `set_body`, with `conflict` |
| `set_interests` | `view`, `strong`, `mild`, `never`, `clubs` (arrays of strings) | `profile::set_interests` inside `mutate`. | `mutate`'s envelope |

Each wrapper attaches the scheduler like its neighbours (`commands.rs:440`). No command hashes,
parses or decides anything the engine could; the folder allow-list is a list, like `EDITABLE`.

## 9. The page

### 9.1 The drawer's body editor (`app/static/console.js`)

For a note in `tasks/` or `courses/`:

- **View.** The body as today's escaped `<pre>`, with an *Edit* button. An empty body shows
  *Add notes* instead of nothing.
- **Edit.** A plain textarea holding the loaded body, with *Save* and *Cancel*; Ctrl+Enter saves and
  Esc cancels. The loaded body is kept as `expected`.
- **Saving.** The controls are disabled until the envelope returns.
- **Saved.** The drawer re-reads the note and shows it; a toast says *Saved · Undo* for 10 seconds.
  Undo is D8's second `set_body`. Its `expected` is the body from that re-read (the `note`
  command's `body`), never the textarea's text, and its `body` is the text loaded before the edit.
  A refused undo is shown like any other refusal, and the pre-edit text stays copyable.
- **Conflict.** The textarea stays, with the student's text in it, under one line: *This note
  changed since you opened it. Your text is still here. Copy it, then reload to see the new
  version.* *Copy* uses the existing `copy_text` command; *Reload* re-reads the note and asks first
  if the draft would be lost.
- **Other refusals** use `showRefusal` (`console.js:1190`) and leave the editor open.
- **Polls.** A poll or any repaint never closes, rebuilds or refills an open editor.
- **Telemetry.** `edit_started`, `edit_committed` and `edit_cancelled` with `object_kind: body` and
  the note's id; never the text (D15).

No Markdown rendering: the body stays plain text, as the vault holds it.

### 9.2 Settings: "Your preferences"

A section in the settings panel (`index.html`, `console.js`, `console.css`), filled by `profile`:

- **How you like to work**: a textarea for preferences, with a counter that marks 600 characters.
  Copy: *Knowlu's judge reads the first 600 characters of this when it sizes up a task. They are
  sent for that one call, and neither we nor the model's host keep them. Otherwise this stays on
  this computer.*
- **Campus events**: four one-item-per-line lists, *Always show me* (`strong`), *Maybe* (`mild`),
  *Never show me* (`never`) and *Clubs I'm in* (`clubs`). Copy: *Knowlu uses these to choose which
  campus events to show you. The first 600 characters also go to its judge with each event it
  judges, for that one call.*
- When `interests_editable` is false, the lists are shown read-only with D10's reason.
- Each part saves on its own *Save* and behaves like the drawer on save, conflict and refusal. A
  save shows *Saved · Undo* for 10 seconds (D8). Preferences undo through `set_preferences`, whose
  `expected` is the text `profile` returns after the save. Interests undo through
  `set_interests` with the four lists held before the save.

The copy is a draft. `console-ui` may tune the words but not the facts: 600 characters, one call,
not kept, stays on this computer, and, for *Campus events*, that the text goes to the judge with
each event. Until bump #1 that copy is the only disclosure of interests reaching a model (§5, Q8).

## 10. Non-goals

- Version history, or undo after the drawer closes (D8).
- Rich text, Markdown preview or attachments.
- Body editing for approvals, issues, info, commitments or archive (D7).
- A `knowlu-engine write set-body` subcommand; `docs/reference/engine-commands.md` does not change.
- Re-judging a task because its body changed (D9).
- Carrying a body edit to a second desktop, or syncing `profile/` (§7.5, D13).
- A profile panel in the wizard, or converting a hand-written block list (Q4 decided (A)).
- Any change to `append_body` or to how agents write bodies.

## 11. Test plan (the test first, then the code)

Every task below writes its failing tests first, runs them red, then writes the code. Test names are
the behaviour; the implementer may adjust a name, not the assertion.

**Engine, contract list (`write.rs`, `journal.rs`, `sync_contract.rs`; contract-engineer):**

1. `set_body_replaces_only_the_body_and_keeps_the_head_bytes`: a CRLF note with a `judgment:` line
   and a body; after set_body the file's bytes through the closing fence's line ending are identical,
   and the body reads back as the normalised new body.
2. `set_body_journals_before_it_writes`: with the note made unwritable, set_body fails, the record is
   in the journal and the note is unchanged. It mirrors the ordering test the module doc names.
3. `set_body_record_holds_hashes_and_never_text`: the record has exactly §6.2's keys and shapes, and
   its `dumps_value` bytes contain neither a marker string from the old body nor one from the new.
   The actor it checks is the test's own `WriteContext`, not §6.2's example value.
4. `set_body_is_a_no_op_when_the_body_is_unchanged`: the same text with CRLF endings, extra trailing
   newlines or leading blank lines returns `false`, appends nothing and leaves the bytes alone.
5. `set_body_refuses_a_stale_expected_body`: `Conflict`, no record, bytes unchanged.
6. `set_body_hashes_chain`: `new.sha256` equals `body_sha256` of the body `note_detail` then shows,
   and a second edit's `old` equals the first's `new`.
7. `a_body_holding_a_dash_rule_keeps_the_frontmatter`: a body with `---` lines; the frontmatter
   mapping is unchanged after the write and the body reads back whole.
8. `set_body_on_a_file_without_frontmatter_replaces_the_whole_text`, and refuses a new body whose
   first line starts `---` there.
9. `set_body_refuses_an_unclosed_fence_and_a_space_trailed_one`.
10. `set_body_refuses_nul_and_a_note_over_the_sync_limit`.
11. `an_emptied_body_keeps_the_blank_line_after_the_fence` (the `create_task` shape).
12. `set_body_records_are_invisible_to_judge_once_and_verify_tail`: `human_set`, `human_edited` and
    `latest_by_field` find nothing for it, and `verify_tail` re-applies nothing.
13. `create_profile_file_makes_only_the_two_files_once_and_stamps_no_id`: other names and an
    existing file are refused; the `create` record comes first with `id: null`; the created
    interests.md reads back through `load_interests` as four empty lists.
14. `ops_include_set_body` (journal.rs): `make_record` accepts it, still refuses an unknown op.
15. `a_set_body_record_is_well_formed_and_small_whatever_the_body` (sync_contract.rs): with a 100 KiB
    body, `record_is_well_formed` accepts the record, it is under 1 KiB, and `build_push` sends it.
16. `build_push_sends_no_record_about_a_path_outside_the_note_folders` (sync_contract.rs): a
    `profile/` record is withheld, a `tasks/` record in the same journal is sent, and a second push
    neither re-sends nor wedges.

**Engine, off the list (`profile.rs`, `surface.rs`; implementer):**

17. `profile_read_of_an_absent_profile_is_empty_and_editable`.
18. `profile_read_of_the_fixture_reads_the_lists_and_marks_them_not_editable` (from a scratch copy of
    `vault-full`; the fixture itself is never written).
19. `set_preferences_creates_then_edits`, with a conflict on a stale `expected`.
20. `set_interests_writes_one_line_lists_that_load_interests_reads`: trimmed, empties and
    duplicates dropped; a line break in an item refused.
21. `set_interests_refuses_a_block_list_before_any_record`: the fixture's block form is refused
    with `MultiLine` from `write::write_one_line_literals`, and `profile.rs` makes no check of its
    own. The primitive's own cases are test 31.
22. `a_set_body_record_reads_body_edited` (the history line).

**App (`app/tests/commands.rs`; implementer):**

23. `set_body_edits_a_task_body_and_returns_fresh_state`, journalled with `console_ctx()`'s actor and
    via. The expected actor is read from `console_ctx().actor` (or `journal::HUMAN_ACTOR` once the
    ruling-11 lane has added it). The test never contains the literal `"quinn"` or `"student"`, so
    it passes on both sides of ruling 11.
24. `set_body_refuses_an_approval_and_an_issue_by_name`.
25. `set_body_conflict_carries_conflict_true_and_the_state`.
26. `profile_commands_round_trip` (`set_preferences`, `set_interests`, then `profile`).

**Page (`app/tests/static_assets.rs`; console-ui):**

27. The drawer invokes `set_body` with `expected`; the settings panel invokes `profile`,
    `set_preferences` and `set_interests`.
28. No `ui_event` call carries a body or profile text; body events use `object_kind` `body`.
29. The poll's repaint path leaves an element marked as an open editor alone.

**Added by the re-check (numbered on so no reference above moves):**

30. `set_body_undo_succeeds_after_a_save_without_a_trailing_newline` (`write.rs`, T1): save a text
    with no final newline. Then undo, with the body `note_detail` re-reads as `expected` and the
    original body as new: `Ok(true)`, the original body is back, and the two records chain
    (`old`/`new`). Passing the saved text without its final newline as `expected` is also not a
    `Conflict` (§7.1 step 3's normalised compare).
31. `write_one_line_literals_refuses_a_value_that_spans_lines` (`write.rs`, T1). Each case returns
    `MultiLine`, appends no record and leaves the bytes alone: a block list; a list at column 0;
    a comment line after the key; `strong:` with trailing spaces, then items; a blank line inside the
    block; a duplicated key. A one-line or absent key writes the same bytes and record that
    `write_literals` writes for the same input.
32. `app/tests/static_assets.rs` (T5): after a save the drawer offers *Undo*. Its `set_body` call
    passes the re-read note's body as `expected`, not the textarea's value. The settings panel offers
    *Undo* for preferences and interests.
33. `scripts/settings-check.py` (T5) walks "Your preferences" with a stubbed `invoke`: save, Undo,
    a conflict that keeps the draft, and the read-only block-list state.

**Gates.** `cargo build --workspace` and `cargo test --workspace` from the root, 0 warnings but the
one accepted `.rsrc` line; `git diff --stat main -- engine/tests/fixtures` empty; the four
`#[ignore]` tests still ignored with their reasons; the eol check. The controller runs the live
proof on a scratch profile against staging (dev build, the DOM driver, never OS input): edit a task
body without a trailing newline, undo it, force a conflict by appending through `knowlu-engine write`
meanwhile, edit preferences and undo that, edit interests, then check `sync` pushed the task's record
and note and no `profile/` record.

## 12. Open questions for Quinn (all decided, 2026-09-29)

Quinn signed this spec on 2026-09-29 and accepted every recommendation. Each question below is kept
with its options and reasoning; its **DECIDED** line records the answer.

Each changes what gets built. The research's four questions are merged into Q1, Q2, Q4 and Q7.

**Q1. What does the journal record of a body edit hold?**
(A) Two SHA-256 digests and two byte counts, no text. (B) The two digests only. (C) The old and new
text in full.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** §5 gives the five reasons text is worse. (C) would keep deleted prose on
the account for 400 days, split any edit of more than about 8 KiB of body into a record that never
sends, and move the privacy page and `PRIVACY_VERSION` into bump #1. The byte counts cost nothing
and let the history say how big an edit was; (B) is fine if Quinn wants the minimum.

**Q2. The body changed after the drawer opened it (an agent appended a line, another window saved).
What does Save do?**
(A) Refuse, say so, keep the student's draft in the editor. (B) The last writer wins.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** (B) erases a line the student never saw, and a hand edit to a body leaves
no journal record, so nothing would show that it happened.

**Q3. Which note bodies can the student edit in the app?**
(A) Tasks and courses. (B) Tasks only. (C) Every folder the drawer opens.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** A course's `## Grade weights` section grounds importance (`judge.rs:409`),
which moves the order, and quinn-ops edited course notes (inventory row 28). An approval's body is
the card the deck parses, so (C) could break a card.

**Q4. How are interests edited?**
(A) Four one-item-per-line lists, written as one-line lists; a hand-written multi-line list is
refused by name. (B) As (A), but the first save rewrites a multi-line list as a one-line list (a new
block-aware surgery in `write.rs`, on the contract list). (C) Preferences only; interests stay a
file edited by hand.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** Every vault the app made since cut day has no `profile/` at all, so Knowlu
creates the file in the one-line form and the refusal meets only hand-written files. (B) adds a
second surgery rule to the one write path for a case no app-made vault has. (C) leaves the campus
event filter out of reach of every student who never opens the vault folder, and editing the file
as raw text is how a stray colon empties every list.

**Q5. Does an M2 edit get an undo?**
(A) *Saved · Undo* for 10 seconds after each save, in the session, for the body, preferences and
interests editors alike (D8). (B) Body only; the two profile editors have none, and that gap goes
to the parity audit. (C) No undo in the MVP.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** VISION commitment 5 says an edit the student makes "acts at once, with
undo", and the page already holds the old text, so it costs a button and one more call per editor.
Undo sends the re-read body as `expected`, and the compare is on normalised bodies (§7.1), so D6's
added final newline cannot make an undo refuse itself (test 30). Field edits (`set_fields`) have
no undo today either. That gap is outside M2 and is named here so the parity audit can carry it.

**Q6. Do records about the profile go to the account?**
(A) No: `build_push` skips every record whose path is outside the note folders (D13). (B) Yes, as
today's code would send them. (C) Sync `profile/` itself now: the server's note-path rule, a
migration (`cloud-engineer`), the restore path, and the privacy page with `PRIVACY_VERSION` in
bump #1.
**DECIDED (Quinn, 2026-09-29): (A), as recommended.** Under (B) the account would keep the interest lists for as long as the
account exists (a student's `set` is a kept record), no computer could use them (apply and restore
both refuse the path), and every restore would print one refusal line per record. (C) is real work
for a second desktop, which is Launch's; the two-desktop stream can take it with its own privacy
words.

**Q7. P6: which small parity items are done in M2, and which are cut?**
**DECIDED (Quinn, 2026-09-29): Appendix A as recommended, item by item:**
- (a) cut;
- (b) **cut from the MVP**, an explicit cut that Quinn ruled on 2026-09-29, and not a hand-off (the
  events spec listed it as "P6, M2 or cut", so a hand-off would have left it with no owner);
- (c) cut from M2, because the events spec's D3 delivers it;
- (d) do.

Quinn accepted all four items as recommended. Both specs were signed in one sitting on 2026-09-29,
and the signing commit aligns events-design.md's non-goal line with them. As drafted: the
controller changes events-design.md's non-goal line ("a conflict line on the card … P6, M2 or
cut") so that it agrees. The conflict line becomes "cut (M2 Appendix A (b)), unless Quinn rules
it into events", and the audit list becomes "M2 (d)". If Quinn instead wants (b) built, the events
spec takes it, computed at read time from `commitments::conflicts`, and that line says so.

**Q8. Interests will now reach the event judge. How is that disclosed before a non-founder uses
the editor?**
(A) Rely on ruling 10's Pilot gate. M2 ships in any release. No non-founder account exists before
privacy bump #1, and bump #1's page must carry §5's interests sentence, which the controller adds
to bump #1's row in HANDOFF §4 when this spec is signed. (B) A mechanical gate, like ruling 12's
date-and-bump rule. The *Campus events* editor is hidden, and `set_interests` refuses by name,
while `PRIVACY_VERSION` is older than bump #1, and a test pins that.
**DECIDED (Quinn, 2026-09-29): (A), as recommended. It costs the MVP nothing.** The Pilot gate is already the hard stop for
every non-founder, and the founder is the MVP's only user. The in-app copy states the fact in the
meantime, and console-ui may not drop it (§9.2). (B) is safe as well, but it needs bump #1's
version date before one exists. It would also hide the editor from the founder's MVP proof on any
build before bump #1, which leaves P5 without its interests half at MVP exit. Pick (B) only if
Quinn wants the disclosure enforced by code rather than by the gate.

**Review findings rejected.** None. Both findings of the spec review were verified against the code
and the cloud design, and both are folded in. First, M2 is the first path by which an app user's
interests reach a model (`events.rs:431-433`, and `site/privacy.html:56` names no interests): see
§2, §5, §9.2 and Q8. Second, the ruling-11 lane shares `journal.rs`, `commands.rs` and `surface.rs`
with M2: see §4, §6.2, tests 3 and 23, and §13. One nuance on the second finding: test 23 already
read the actor from `console_ctx()`. The revision still forbids the literal outright and makes
§6.2's example explicitly illustrative.

**Re-check findings (second round): none rejected.** All five were verified against the code, the
two sibling specs and HANDOFF on `main`, and all five are folded in:

1. **P6(c) did not hold.** The cited lines (`eventemit.rs:660`, `:735-737`) belong to the
   `event-check` card. The digest becomes eligible at `horizon_start` (`eventemit.rs:59-70`), the
   earlier of the 14-day horizon and three days before a registration deadline, and it expires at
   its earliest event (`:316`). (c) is now "cut from M2; delivered by the events spec's D3", and
   T6 no longer touches `eventemit.rs` (Appendix A, §13).
2. **(b) had no owner.** It is now an explicit cut for Quinn, with the events spec's line to align
   at signature (Q7, Appendix A).
3. **Sibling lanes.** §13 now orders M2 after Gmail connect and before events (HANDOFF §3, MVP
   items 2 to 4), names `scripts/settings-check.py` in T5, and recounts the handler list (§8).
4. **Undo.** Undo now uses the re-read body, the compare is normalised, every M2 editor gets
   Saved · Undo, and tests 30, 32 and 33 cover it (D8, §7.1, §9, Q5).
5. **The block-style check** is now a contract-list primitive in `write.rs` (T1), with test 31
   (D10, §7.2, §7.6).

One nuance on finding 2: M2 takes the finding's second option, an explicit cut, rather than
editing the events spec from here, which another pass is revising. The events spec's own
lane-overlap table (its §9) does not list `m2-editing` either. The controller should add it when
aligning that non-goal line: shared files
`app/static/console.js` and `app/tests/static_assets.rs`, with M2 merging first.

## 13. Task sketch

One worktree, one branch (`m2-editing`), one plan (`docs/plans/`, with its fidelity ledger) written
from this spec once it is signed. **Ordering with ruling 11.** The branch is cut from a `main` that
carries the ruling-11 actor-token lane (Integrate's rule-1 token work: `journal::HUMAN_ACTOR`, the
new-vault `student` token, the literal test). That lane and M2 never run alongside each other (§4).
If Quinn orders M2 first instead, T1's "After" becomes "—", and the lane rebases onto M2 once M2
merges.

**Ordering with the sibling MVP lanes.** HANDOFF §3 orders the MVP lanes Gmail connect (2), then
M2 (3), then events (4).
- **Gmail connect** shares `app/static/{index.html,console.js,console.css}`,
  `app/tests/static_assets.rs` and `scripts/settings-check.py` with M2 (its T4 builds a row in the
  same settings panel where M2's T5 adds "Your preferences"). It also shares `app/src/main.rs:177`
  (its T6 adds three names). **M2's branch is cut from a `main` that already carries Gmail connect.**
  No M2 task runs beside a Gmail task.
- **Events** rewrites `eventemit.rs` (its T2a) and edits `console.js` and `static_assets.rs` (its
  T5). Since P6(c) moved to events (Appendix A), M2 no longer edits `eventemit.rs`. The remaining
  overlap is the page files, and events rebases onto M2 after M2 merges.
- **The check.** Before dispatching T1 and again before T5, the controller runs
  `git diff --name-only main...<branch>` for every open sibling branch and confirms that none
  shares a file with the task. This is the check §4 already makes for ruling 11.

| # | Agent | Files (exclusive to the task) | After | Why this agent |
|---|---|---|---|---|
| T1 | `contract-engineer` (Opus, xhigh), then `contract-reviewer` | `engine/src/journal.rs`, `engine/src/write.rs` (including D10's `value_spans_lines` and `write_one_line_literals`); tests 1-14, 30, 31 | the ruling-11 lane and Gmail connect merged (§4, above) | Both files are on the contract list; a silent error here corrupts vault bytes, and D10's detector is the only guard on a block list. `journal.rs` is also the ruling-11 lane's file. |
| T2 | `contract-engineer`, then `contract-reviewer` | `engine/src/sync.rs`, `engine/tests/sync_contract.rs`; tests 15-16 | T1 | Sync and its tests are on the list. It also greps every writer for a record whose path is outside the note folders and reports what D13 stops sending. |
| T3 | `implementer` (Sonnet, high) | `engine/src/profile.rs` (new), `engine/src/surface.rs` (`describe` only); tests 17-22 | T1 (so after the ruling-11 lane, which edits `surface.rs:1495`, `:1725`) | Fully specified, off the list, checked by tests. It holds no vault-byte guard of its own: the block-list refusal is T1's primitive, so a mistake here refuses or mis-cleans a list but cannot orphan list lines. |
| T4 | `implementer` | `app/src/commands.rs`, `app/tests/commands.rs`; tests 23-26, then P6(d)'s command | T3 (so after the ruling-11 lane, which changes `console_ctx()`) | Thin wrappers, off the list. |
| T5 | `console-ui` (Sonnet, medium) | `app/static/console.js`, `index.html`, `console.css`, `app/tests/static_assets.rs`, `scripts/settings-check.py`; tests 27-29, 32, 33 | §8's names, and Gmail connect merged (may run beside M2's own T4) | `app/static` and the walk scripts are its lane. |
| T6 | `implementer` | `engine/src/eventroster.rs` (the audit reader) | T1 | P6(d), engine half; disjoint from T3. It does not touch `eventemit.rs`, which is the events spec's T2a file. |
| T7 | `console-ui` | `app/static/console.js`, `console.css` (the *Not shown* list) | T5, T6, T4's command | P6(d), page half; same files as T5, so after it. |
| T8 | `docs-keeper` (Sonnet, medium) | `docs/reference/app.md`, `docs/surface/anatomy.md`, `docs/surface/inventory.md` row 28, `docs/notes/2026-09-29-vision-program.md` P5-P6 | T7 | Reference text only. |
| T9 | `reviewer` (Opus, high) | `docs/reports/<date>-m2-editing-whole-branch-review.md` | T8 | Everything a cheaper agent changed is reviewed before a push. |

**The main session's own work:** the two controller hand-offs (`pub mod profile;` in
`engine/src/lib.rs`; the four or five names in `app/src/main.rs:177`), the gate, the live proof
(§11), and the PR. It pushes nothing that carries code without Quinn's word.

**Size.** T1 M (the risk is here, and D10's primitive moved it up from S-M), T2 S, T3 S, T4 S,
T5 M, T6 S, T7 S, T8-T9 S. M2 as a whole is M.

## Appendix A. P6's small items

| Item | What exists at `97dc27b` | Do or cut | Size | Why |
|---|---|---|---|---|
| **(a)** A producer for *Good to know* items | The list, its read model and its expiry pass exist (`surface::good_to_know`, `surface.rs:1406`; `info::info_pass`, `info.rs:194-226`); `info::open_info` is called only by the `info` CLI (`info.rs:320`). Gmail's `information` tier is deliberately the noise tier and writes nothing (`enrich.rs:660-665`). | **Cut** | S-M, and an open design | A producer is a judgment design (which email is worth knowing, which is noise) plus a prompt change in the cloud. Nothing names what should feed it. Revisit with the assistant or a Gmail pass that has a use for it. |
| **(b)** Conflict flags | Read and shown (`models.rs:179`, `:262`; `render.rs:460`; `surface.rs:592`); nothing writes `conflicts_with`. `commitments::conflicts` and `fit` are on main (`commitments.rs:7839`, `:7903`). | **Cut from the MVP** (explicit; ruled by Quinn, 2026-09-29; not a hand-off) | S-M if later built | The commitment model no longer blocks it. What is missing is tasks with a clock time, which arrive when required events become tasks (P4). The events spec lists the conflict line as "P6, M2 or cut", so handing it to events from here would leave it with neither owner. It is recorded as a cut, so the parity audit can close the row. If Quinn rules it in, the events spec takes it, computed at read time from `commitments::conflicts` and never written into a note, and its non-goal line says so (Q7). |
| **(c)** Opportunity proposals expire after 14 days | Opportunities are proposed in the **digest**, not the `event-check` card (whose lines `eventemit.rs:660` and `:735-737` are). An event becomes eligible at `horizon_start` (`eventemit.rs:59-70`, used by `eligible_events`, `:118`, `:141`). That is the **earlier** of `start − propose_horizon_days` (default 14) and three days before a registration deadline. The digest's `expires` is its earliest event (`:316`). **Counterexample:** an event on Nov 30 with registration closing Nov 5 opens on Nov 2; its digest is filed Nov 2, expires Nov 30 and lives 28 days. | **Cut from M2; delivered by the events spec's D3** | none in M2 | It does not hold today. The events spec's D3 replaces the digest with one card per opportunity that "expires after 14 days or at the event, whichever is first", and its F1 fixes the digest's decline-on-expiry. A pin in M2 would either fail or pin a false claim, and it would sit in `eventemit.rs`, which the events spec's T2a rewrites. So M2 adds no test. The events spec's §4.4 and §11.1 test 7 already carry the cap: an opportunity card expires at "the earlier of that date and `first_proposed_at + 14`". When both specs are signed, the controller asks that test 7 include the registration-deadline case above. |
| **(d)** The dropped-event audit list | Every drop, the prefilter's included, is written to the audit section of `state/events.md` (`eventroster.rs:11-13`), which no student opens. | **Do** | S + S | VISION: "failures are visible" and "nothing missed that Knowlu had the information to catch". A read-only *Not shown (N)* list under *Coming up*, each line the event, its date and why it was dropped, from a new read command (T6, T4, T7). No engine write, no new key in the state, no oracle change. T6's test reads a scratch copy of the frozen `vault-full/state/events.md` and never writes the reference. |

**Before signing.** The spec review (`reviewer`, Opus high) lands in
`docs/reports/2026-09-29-m2-editing-spec-review.md`; Quinn's answers to §12 are folded into §3 and
recorded in a closing section, as the two-desktop spec did.

**Signed.** Quinn signed this spec on 2026-09-29 and accepted every recommendation in §12, so §3's
decisions stand as written and each question's **DECIDED** line is the record.
