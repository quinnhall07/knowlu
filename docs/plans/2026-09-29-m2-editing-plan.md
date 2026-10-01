# M2 editing: task bodies and the student's profile (implementation plan)

**Date:** 2026-09-29. **Base:** branch `m2-editing`, worktree `.claude/worktrees/m2-editing`. The
base already carries ruling 11's actor-token lane (`journal::HUMAN_ACTOR`, `console_ctx(vault)`,
`write::human_gate`, `engine/tests/human_actor_literal.rs`). It does not carry Gmail connect (PQ1).
**Governs:** `docs/specs/2026-09-29-m2-editing-design.md`, signed by Quinn on 2026-09-29 with every
recommendation accepted: Q1–Q6 and Q8 are (A), and Q7 follows Appendix A ((a) cut, (b) cut from the
MVP, (c) delivered by the events spec's D3, (d) done). Above it: the cloud design's Amendment
2026-09-29, ruling 10 (what the MVP holds) and ruling 11 (the human token).
**Stage:** MVP, HANDOFF §3 item 3. **Implements:** parity rows P5 and P6 of
`docs/notes/2026-09-29-vision-program.md`, and row 28 of `docs/surface/inventory.md`.
**Status:** draft for review. Nothing here is built yet.

## 1. Goal, and what must not move

A student edits a task's or a course's notes in the drawer, and edits "how you like to work" and
the four campus-event lists in Settings. Each edit takes effect at once and can be undone for 10
seconds. There is no text editor and no file path. VISION check: commitment 5 says an edit the
student makes "acts at once, with undo". Success 6 says a second student needs nothing from
anyone, and today these edits need a text editor on the vault folder.

These must not change:

- **Every existing write function.** `append_body`, `write_literals`, `create`, `delete`,
  `move_note` and the rest of `write.rs` stay byte-for-byte as they are (spec §4). The new
  functions sit beside them.
- **The journal contract.** `VIAS`, `make_record`, the record shape and key order,
  `latest_by_field`, `human_set` and `human_edited` do not change. `OPS` gains one entry.
- **Frozen references.** None of the eight Python references is regenerated. The three
  `surface-today-*.json` references are not regenerated either: no fixture journal holds a
  `set_body` record. `git diff --stat main -- engine/tests/fixtures` stays empty.
- **No text in the journal** (Q1 (A)). A `set_body` record holds two SHA-256 digests and two byte
  counts, never prose.
- **Nothing about `profile/` reaches the account** (D13, Q6 (A)).
- **The privacy page.** `site/` is not edited, and `PRIVACY_VERSION` stays `2026-09-24` (spec §5).
  The interests sentence goes to privacy bump #1's packet, never into `site/` here.
- **The cloud.** The server checks only that `op` and `actor` are present, and it prunes a
  non-`set`/`create` record at 400 days. No migration, no function and no `cloud-engineer` task.
  This was checked by grep of `cloud/supabase`: nothing there lists ops.
- **Rule 1.** No new `"quinn"` or `"student"` literal. Every write uses `console_ctx(vault)`, and
  `engine/tests/human_actor_literal.rs` still passes.
- **Telemetry.** No new action and no text (D15).

## 2. What changed since the spec was written

These were read against the worktree. Briefs cite functions, never line numbers, because the
spec's line numbers have drifted.

- **Ruling 11 has merged into the base.** The spec's "After: the ruling-11 lane" for T1, T3 and T4
  (spec §4, §13) is met. Three consequences:
  - `console_ctx(vault)` now returns `Result<WriteContext, String>`. T4a passes its error through
    `mutate` like its neighbours do.
  - `write::human_gate` is the one gate at the top of every public write entry point. The three
    new public write functions (`set_body`, `create_profile_file`, `write_one_line_literals`) must
    take it before any record. The spec predates the gate, so this plan adds two tests, G1 and G2,
    that pin it.
  - `WriteError` already has `LineBreak` and `Actor`. The spec's three new variants (`Conflict`,
    `Body`, `MultiLine`) join them.
- **Gmail connect is not merged.** `main` has no Gmail command in `app/src/main.rs`. Spec §13 cut
  M2's branch "from a `main` that already carries Gmail connect". PQ1 asks how to proceed.
- **The worktree's `HANDOFF.md` predates `main` at `4cf3661`.** `main` records the interests
  sentence in bump #1's row (Q8) and the Gmail release gate. T0 merges `main` in.

## 3. Decisions this plan makes

- **P1. Smaller tasks, with the spec's ids kept.** Each spec task that holds two separable pieces
  is split with a letter, so the fidelity ledger still maps to spec §13:
  - T1 becomes T1a (the op, `set_body` and the hash) and T1b (the three profile-file primitives).
    Both edit `write.rs`, so they run in order. T1a is split again into three dispatches, so that
    no single xhigh dispatch can half-land on `write.rs` and Checkpoint A reads three small diffs:
    T1a.1 (the op, the hash and the new error variants), T1a.2 (`set_body`'s order, refusals and
    journal-first write) and T1a.3 (the no-op and the pinning tests).
  - T4 becomes T4a (the four commands) and T4b (P6(d)'s read command).
  - T5 becomes T5a (the drawer) and T5b (Settings). T5a is split into T5a.1 (view, edit, save)
    and T5a.2 (conflict, undo, the poll exemption). T5b is split into T5b.1 (preferences),
    T5b.2 (interests) and T5b.3 (`settings-check.py`, last).
  - Sizing: each dispatch should land an edit of about 80 lines or fewer, tests included where
    that is possible. A dispatch that would clearly exceed it is split before it starts, not after.
- **P2. P6(d)'s read.** The engine half is `eventroster::read_dropped(vault)`, which returns the
  dropped events from the audit section of `state/events.md`. The app half is the command
  `dropped_events`. Each item carries the event's title, its date and time, and one reason:
  - "filtered by your interests" for a line marked `· filtered`;
  - "you declined it" when the event ledger (`eventledger::load_ledger`, read-only) marks the uid
    declined;
  - "judged not relevant" when the ledger holds a verdict outside `RELEVANT_VERDICTS`, followed by
    the ledger's `why` when there is one.

  An unjudged line is not a drop and is left out (PQ2). The read never writes, adds no key to the
  state and changes no oracle. An absent file reads as an empty list.
- **P3. Hand-offs land when the next task needs them**, not all at merge. `pub mod profile;` goes
  into `engine/src/lib.rs` before T3, because T3's tests cannot compile without it. The four names
  go into the console's `generate_handler!` list after T4a, and `dropped_events` after T4b, so the
  gate never carries a command that is not registered. The count is taken again at merge.
- **P4. `scripts/settings-check.py`.** HANDOFF §2 lists `scripts/*.py` as single-owner files. The
  signed spec, which is newer, gives this script to T5b (here T5b.3), as the Gmail spec gives it
  to its own T4. The plan follows the spec. If Gmail's version is on `main` by then, T5b.3 builds
  on it.
- **P5. Two review points.** Checkpoint A: `contract-reviewer` reviews T1a–T2 before any
  off-list task builds on the API. T9: `reviewer` reviews the whole branch.

## 4. Open questions (each with a recommendation)

- **PQ1. Gmail connect is not on `main`. Spec §13 assumed it was, and says no M2 task runs beside
  a Gmail task.**
  - *Recommend:* run the engine and command tasks now (T1a.1–T1a.3, T1b, T2, T3, T4a, T6, T4b). They share
    no file with Gmail connect's tasks, which touch `app/src/account.rs`, `engine/src/enrich.rs`,
    `cloud/` and the page. T0 confirms this with `git diff --name-only` against any Gmail branch.
  - Hold the page tasks (T5a.1–T5b.3, T7) until Gmail connect merges and `main` is merged into
    `m2-editing`, as the spec orders. If Gmail connect has not started by the time T4b is done,
    Quinn picks: wait, or let M2's page go first and have Gmail's T4 rebase onto it.
  - *Cost:* none if the files stay disjoint. The one shared file, `app/src/main.rs`, is changed
    only by the main session at hand-off time.
  - *Needs Quinn's word:* it departs from the letter of a signed section.
  - Answered 2026-09-30: resolved. Gmail connect merged to main (#26) before the page tasks, as the spec ordered.
- **PQ2. Do unjudged events appear under *Not shown*?**
  - *Recommend:* no. An unjudged event has not been dropped: the next slot judges it. Listing it
    as "not shown" would claim a decision nobody made. Asked before T6; not blocking before then.
  - Answered 2026-09-30: NO. Unjudged events never appear under Not shown (only filtered, declined, judged-irrelevant, each with its reason).

## 5. Tasks

**Rules for every task.**
- Write the named tests first, run them in the foreground and see each fail for the stated reason,
  then write the code. A test's name may change; its assertion may not.
- Each task ends with `cargo build --workspace` and `cargo test --workspace` from the root in the
  dev profile. That means green, 0 warnings except the accepted `.rsrc` line, the four `#[ignore]`
  tests untouched, and `git diff --stat main -- engine/tests/fixtures` empty.
- New files are LF.
- A test copies any fixture it reads into a temporary directory first; it never writes into
  `engine/tests/fixtures/`.
- No test contains the literal `"quinn"` or `"student"`. The actor comes from the test's own
  `WriteContext`, from `console_ctx(&vault)` or from `journal::HUMAN_ACTOR`.
- One implementer at a time in the worktree (HANDOFF §2).
- Commits carry the session's trailer.
- A task whose agent fails twice goes one level up, never straight to `max`.

**CL** marks a file on the contract list. **Starts on Opus** marks tasks whose first agent runs on
Opus. **Quinn** marks tasks that need Quinn's word or presence.

| Task | Agent (model, effort) | Starts on Opus | Quinn | Files | After |
|---|---|---|---|---|---|
| T0 | main session | yes | PQ1 answer | none (git, ledger) | — |
| T1a.1 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/journal.rs` CL, `engine/src/write.rs` CL | T0 |
| T1a.2 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/write.rs` CL | T1a.1 |
| T1a.3 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/write.rs` CL | T1a.2 |
| T1b | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/write.rs` CL | T1a.3 |
| T2 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/sync.rs` CL, `engine/tests/sync_contract.rs` CL | T1b |
| A | `contract-reviewer` (Opus, xhigh) | yes | — | `docs/reports/…-m2-editing-contract-review.md` | T2 |
| T3 | `implementer` (Sonnet, high) | no | — | `engine/src/profile.rs` (new), `engine/src/surface.rs` (`describe` only) | A, H1 |
| T4a | `implementer` (Sonnet, high) | no | — | `app/src/commands.rs`, `app/tests/commands.rs` | T3 |
| T6 | `implementer` (Sonnet, high) | no | PQ2 answer | `engine/src/eventroster.rs` | T4a |
| T4b | `implementer` (Sonnet, high) | no | — | `app/src/commands.rs`, `app/tests/commands.rs` | T6 |
| T5a.1 | `console-ui` (Sonnet, medium) | no | — | `app/static/console.js`, `app/static/console.css`, `app/tests/static_assets.rs` | T4a, H2, PQ1 |
| T5a.2 | `console-ui` (Sonnet, medium) | no | — | `app/static/console.js`, `app/static/console.css`, `app/tests/static_assets.rs` | T5a.1 |
| T5b.1 | `console-ui` (Sonnet, medium) | no | — | `app/static/index.html`, `console.js`, `console.css`, `app/tests/static_assets.rs` | T5a.2 |
| T5b.2 | `console-ui` (Sonnet, medium) | no | — | `app/static/index.html`, `console.js`, `console.css`, `app/tests/static_assets.rs` | T5b.1 |
| T5b.3 | `console-ui` (Sonnet, medium) | no | — | `scripts/settings-check.py` | T5b.2 |
| T7 | `console-ui` (Sonnet, medium) | no | — | `app/static/console.js`, `console.css`, `app/tests/static_assets.rs` | T5b.3, T4b, H3 |
| T8 | `docs-keeper` (Sonnet, medium) | no | — | `docs/reference/app.md`, `docs/surface/anatomy.md`, `docs/surface/inventory.md`, `docs/notes/2026-09-29-vision-program.md` | T7 |
| T9 | `reviewer` (Opus, high) | yes | — | `docs/reports/…-m2-editing-whole-branch-review.md` | T8 |
| T10 | main session | yes | merge word | `engine/src/lib.rs`, `app/src/main.rs` (hand-offs, already in) | T9 |

No task goes to `cloud-engineer`, because M2 changes nothing under `cloud/` (§1). No task goes to
`mechanical`. Every piece is either on the contract list or needs judgment about the page.

### T0. Preflight (main session)

**Why the main session:** git state, sibling-lane checks and Quinn's answers belong to the
controller.
- Merge `main` into `m2-editing`. The delta should be docs only. If it carries code, run the gate
  before T1a.1.
- Confirm that ruling 11 is in the base (`journal::HUMAN_ACTOR`, `write::human_gate`,
  `console_ctx(vault)`). Record the base commit in the SDD ledger
  (`.superpowers/sdd/2026-09-29-m2-editing/`).
- Run `git diff --name-only main...<branch>` for every open sibling branch: `m1-grades`, any Gmail
  connect branch, `j-events`, `p3-registrar`, `two-desktop`. Record in the ledger that none shares
  a file with T1a.1–T4b or T6. Repeat this before T5a.1 (spec §13).
- Confirm on `main` that the events spec's non-goal line was aligned at signing (Q7: "cut (M2
  Appendix A (b))…", the audit list "M2 (d)") and that bump #1's row carries §5's interests
  sentence (Q8). If either is missing, it is a docs-only fix on `main`.
- Ask Quinn PQ1, one question with its context. Ask PQ2 before T6.

**Done when:** the ledger records the base, the disjointness check and Quinn's PQ1 answer.

### T1a. The `set_body` op and the body write (`journal.rs`, `write.rs`), in three dispatches

**Agent, all three:** `contract-engineer` at xhigh. **Why:** both files are on the contract list,
and a wrong fence rule here overwrites frontmatter in a student's vault. This is where M2's risk
sits. **Why three dispatches:** one dispatch holding all of it would be several hundred lines on the
riskiest code in M2. A stall or a half-landed dispatch would leave `write.rs` in an intermediate
state, and Checkpoint A would have to read one very large diff. Each dispatch below leaves the
workspace green, with 0 warnings, and ends in its own commit.

**Rules for all three.**
- Tests go in each file's `#[cfg(test)]` module.
- Every existing `journal.rs` and `write.rs` test passes unchanged.
- The diff touches no line inside an existing function body.
- The sync, oracle and golden tests run unchanged.

#### T1a.1. The op, the hash and the new error variants

**Files:** `engine/src/journal.rs` (CL), `engine/src/write.rs` (CL).
**Tests first** (spec test 14, plus two plan tests that pin what test 6 and D4 build on):
- `ops_include_set_body` (journal.rs, test 14): `make_record` accepts it and still refuses an
  unknown op.
- `body_sha256_is_lowercase_hex_of_the_utf8_bytes` (the hash half of test 6). It checks the empty
  body's well-known digest and a non-ASCII body's, both in lowercase hex.
- `the_new_write_errors_name_the_file_or_the_reason`: `Conflict`'s line names the path,
  `Body`'s its reason, and `MultiLine`'s reads "Knowlu can only edit a list written on one line"
  plus the key.

**Behaviour.** Spec §6.2 and §7.2's hash paragraph.
- `journal::OPS` gains `"set_body"`, making seven entries. Nothing else in `journal.rs` changes.
- `write::body_sha256(body)` is SHA-256 through `ring`, in lowercase hex. It is the one hash
  function: no caller, app or test hashes on its own.
- `WriteError` gains `Conflict(String)`, `Body(&'static str)` and `MultiLine(String)`, beside
  `LineBreak` and `Actor`, each with its `Display` line. Nothing raises them yet.

**Done when:** the three tests pass and the rules above hold. Expected size: well under 80 lines.

#### T1a.2. `set_body`: gate, split, compare, refusals, journal first, file second

**Files:** `engine/src/write.rs` (CL).
**Tests first** (spec tests 1, 2, 5, 7, 8, 9 and 10, plus G1):
- `set_body_replaces_only_the_body_and_keeps_the_head_bytes` (test 1). It uses a CRLF note with a
  `judgment:` line.
- `set_body_journals_before_it_writes` (test 2). It mirrors the existing ordering test named in
  the module doc.
- `set_body_refuses_a_stale_expected_body` (test 5): `Conflict`, no record, bytes unchanged.
- `a_body_holding_a_dash_rule_keeps_the_frontmatter` (test 7).
- `set_body_on_a_file_without_frontmatter_replaces_the_whole_text` (test 8), including the refusal
  of a first line starting `---`.
- `set_body_refuses_an_unclosed_fence_and_a_space_trailed_one` (test 9).
- `set_body_refuses_nul_and_a_note_over_the_sync_limit` (test 10). The limit is
  `sync::MAX_NOTE_BYTES`, read from the constant.
- `set_body_normalises_by_d6` (plan addition): `\r\n` and a lone `\r` become `\n`, and leading and
  trailing newlines follow D6's rule. It tests the one private normaliser.
- **G1 (plan addition)** `set_body_takes_the_human_gate`. With an invalid `config/actor.yaml`, and
  again with a human actor that is not the vault's token, the call returns `WriteError::Actor`.
  No record is appended and the note's bytes do not change. An `agent:` actor is not gated.

**Behaviour.** Spec §6.1, §6.2 and §7.1, with D1, D3–D6 and D9.
- Normalisation is D6's rule, written once as a private function, and used by both the compare
  and the write. It lands here, not in T1a.1, because this is its first caller; landing it earlier
  would leave an unused private function and break the 0-warning gate.
- `write::set_body(vault, target, expected, new_body, ctx, journal) -> Result<bool, WriteError>`
  works in this order, and a refusal at any step leaves the journal and the file untouched:
  1. `human_gate`.
  2. `resolve_target`, then `load(path, false)`.
  3. The textual split, using the writer's fence rule: the closing line is exactly `---`.
  4. Compare-and-swap on the *normalised* current and expected bodies (`Conflict(path)`).
  5. (T1a.3 inserts the no-op here.)
  6. D4's refusals, each a `Body(&'static str)` with a named reason.
  7. The record through `make_record` and `dumps_value`. Its `old` is the hash and byte count of
     the current body exactly as `note_detail` shows it, and its `new` is the hash and byte count
     of the normalised new body. Its `field` and `evidence` are `null`, and `id` is the note's id
     when valid, else `null`. No body text enters it.
  8. The file: head, then separator, then the new body, through `pystr::write_text`.

**Done when:** the nine tests pass and the rules above hold. If the edit runs well past 80 lines,
the agent stops after steps 1–4 with their tests green, commits, and reports; the controller
dispatches steps 6–8 as a follow-on under the same brief.

#### T1a.3. The no-op, and the pinning tests

**Files:** `engine/src/write.rs` (CL).
**Tests first** (spec tests 3, 4, 6, 11, 12 and 30):
- `set_body_is_a_no_op_when_the_body_is_unchanged` (test 4). It fails first, because T1a.2 has no
  step 5.
- `set_body_record_holds_hashes_and_never_text` (test 3): exactly §6.2's keys and shapes. Marker
  strings from the old and new bodies are absent from the `dumps_value` bytes.
- `set_body_hashes_chain` (test 6). It uses `surface::note_detail`'s body.
- `an_emptied_body_keeps_the_blank_line_after_the_fence` (test 11), the `create_task` shape.
- `set_body_records_are_invisible_to_judge_once_and_verify_tail` (test 12).
- `set_body_undo_succeeds_after_a_save_without_a_trailing_newline` (test 30).

**Behaviour.** Step 5 of §7.1: when the normalised new body equals the normalised current body,
return `Ok(false)` with no record and no write.

**Pinning tests.** Tests 3, 6, 11, 12 and 30 pin behaviour T1a.2 already built, so they may pass on
their first run. The agent records in the SDD ledger which of the six failed first and which
passed first. A pinning test that fails is a T1a.2 defect: it is fixed here, and the report names
it for Checkpoint A. The assertion is never weakened to make it pass.

**Done when:** the six tests pass, the rules above hold, and the ledger lists each test's first
result.

### T1b. The profile-file primitives (`write.rs`)

**Agent:** `contract-engineer` at xhigh. **Why:** `value_spans_lines` is the only guard against
single-line surgery orphaning a block list's `  - …` lines (D10). A false "no" corrupts the file
silently, and the event filter would then read empty lists. CLAUDE.md puts that at xhigh.
**Files:** `engine/src/write.rs` (CL).

**Tests first.** These are spec tests 13 and 31, plus G2:
- `create_profile_file_makes_only_the_two_files_once_and_stamps_no_id` (test 13). Other names and
  an existing file are refused. The `create` record comes first, with `id: null`. The created
  interests file reads back through `events::load_interests` as four empty lists.
- `write_one_line_literals_refuses_a_value_that_spans_lines` (test 31). The six refusal cases are:
  a block list, a list at column 0, a comment line after the key, `strong:` with trailing spaces
  and then items, a blank line inside the block, and a duplicated key. The pass case is a one-line
  or absent key: its bytes and record equal what `write_literals` writes for the same input on a
  copy of the same file.
- **G2 (plan addition)** `profile_primitives_take_the_human_gate`: G1's two refusal cases for
  `create_profile_file` and `write_one_line_literals`.

**Behaviour.** Spec §6.3, §7.2, D10 and D11.
- `create_profile_file(vault, name, text, ctx, journal)`:
  - accepts only `preferences` or `interests`. Any other name is a named `Body` refusal;
  - refuses with `Exists` when the file is present;
  - takes `human_gate`;
  - appends a `create` record first, with `id: null`, path `profile/<name>.md`, and `new` set to
    the frontmatter mapping `split_frontmatter` reads from `text` (`{}` when there is none);
  - then creates `profile/` and writes `text`. It never stamps an id. The caller supplies `text`
    (T3 holds the two canonical texts).
- `value_spans_lines(text, key) -> bool` is spec §7.2's rule, scanning the frontmatter only by the
  writer's fence rule. It errs toward `true`.
- `write_one_line_literals(vault, target, literals, ctx, journal, opts)`:
  - takes the gate;
  - reads the file once;
  - returns `MultiLine(key)` before any record when `value_spans_lines` is true for any key it
    would replace;
  - otherwise calls `write_literals` unchanged.

**Done when:** the three tests pass, T1a's tests still pass, and `write_literals` has no changed
line.

### T2. Profile records stay on this computer (`sync.rs`)

**Agent:** `contract-engineer` at xhigh. **Why:** `sync.rs` and its contract test are on the
list. A filter that advanced the push cursor past an unsent record would lose records for good.
**Files:** `engine/src/sync.rs` (CL), `engine/tests/sync_contract.rs` (CL).

**Tests first.** These are spec tests 15 and 16:
- `a_set_body_record_is_well_formed_and_small_whatever_the_body`. With a 100 KiB body,
  `record_is_well_formed` accepts the record, the record's `dumps_value` bytes are under 1 KiB, and
  `build_push` includes it.
- `build_push_sends_no_record_about_a_path_outside_the_note_folders`:
  - a `profile/` record is withheld;
  - a `tasks/` record in the same journal is sent;
  - a second push neither sends the profile record nor stalls on it.

**Behaviour.** Spec §7.3 and D13. `build_push` gets one more `continue` in its record loop, beside
the local-card filter, for any record whose `path` fails `is_note_path`. It is placed so that it
never moves the cursor over an unsent record. `record_is_well_formed` is not edited.

**Report.** The agent greps every journal writer for a record whose path can fall outside the
note folders. It lists in the SDD ledger what D13 now withholds, and names any writer other than
the two profile paths. Checkpoint A reads that list.

**Done when:** both tests pass, and every existing sync, restore and P21 local-card test passes
unchanged.

### Checkpoint A. Contract review of T1a–T2

**Agent:** `contract-reviewer` (Opus, xhigh). **Report:**
`docs/reports/<date>-m2-editing-contract-review.md`, where the date is the day it lands. The
review:
- reads the diff of `journal.rs`, `write.rs`, `sync.rs` and `sync_contract.rs` against spec §4,
  §6, §7.1–§7.3 and D1–D13, one commit at a time (T1a.1, T1a.2, T1a.3, T1b, T2);
- reads T1a.3's first-run list, and any T1a.2 defect it names;
- checks that no existing function body moved;
- checks that the fence rule and `value_spans_lines` err in the safe direction;
- checks that the gate precedes every record;
- checks that no text enters a record;
- checks that the frozen references and oracles ran unchanged.

Fixes go back to `contract-engineer`, and the reviewer re-reads them. T3 starts when the report has
no open Critical or Important finding.

### T3. Reading and writing the profile (`profile.rs`), and the history line (`surface.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** the work is off the list, fully specified and
checked by tests. It holds no vault-byte guard of its own: the block-list refusal lives in T1b's
primitive, so a mistake here refuses an edit or mis-cleans a list but cannot orphan list lines.
**Files:** `engine/src/profile.rs` (new), and `engine/src/surface.rs` (`describe` and its tests
only). **Before it:** hand-off H1 (`pub mod profile;`), applied by the main session.

**Tests first.** These are spec tests 17–22:
- `profile_read_of_an_absent_profile_is_empty_and_editable` (17).
- `profile_read_of_the_fixture_reads_the_lists_and_marks_them_not_editable` (18). It runs on a
  scratch copy of `vault-full`.
- `set_preferences_creates_then_edits` (19). It also covers a `Conflict` on a stale `expected`.
- `set_interests_writes_one_line_lists_that_load_interests_reads` (20):
  - items are trimmed, and empty items and exact duplicates are dropped (the first one is kept);
  - an item that `write::single_line_problem` rejects is refused by name before any record.
- `set_interests_refuses_a_block_list_before_any_record` (21). On a scratch copy of the fixture's
  block-style interests file, the call returns `MultiLine` from `write::write_one_line_literals`.
  The journal and the file are unchanged.
- `a_set_body_record_reads_body_edited` (22, in `surface.rs`).

**Behaviour.** Spec §6.3, §7.4, §7.6, D11 and D12.
- `read(vault) -> Profile` returns:
  - the preferences body;
  - the four lists, through `events::load_interests`, so the app and the event filter read the
    same lists;
  - `interests_editable`, which is false when `write::value_spans_lines` is true for any of the
    four keys. `profile.rs` has no detector of its own, and the reviewer checks this;
  - warnings. An unreadable file is a warning, never an error.
- `set_preferences(vault, expected, text, ctx, journal)`:
  - calls `create_profile_file("preferences", "")` when the file is absent;
  - then calls `write::set_body` on `profile/preferences.md`.
- `set_interests(vault, lists, ctx, journal)`:
  - calls `create_profile_file("interests", <the four empty flow lists>)` when the file is absent;
  - cleans the items;
  - makes one `write::write_one_line_literals` call with `strong`, `mild`, `never` and `clubs` in
    that order. Each literal comes from `write::to_literal`, so the one emitter writes it.
- The two canonical texts are constants in `profile.rs`, byte-equal to spec §6.3.
- `describe` renders a `set_body` record as `"{path}: body edited ({who})"`. The record's `old`
  and `new` stay in `DeltaRecord` unprinted.

**Done when:** the six tests pass, and the surface oracle and every existing `surface.rs` test
pass unchanged.

### T4a. The four commands (`commands.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** these are thin wrappers off the list, and
`commands.rs` computes nothing.
**Files:** `app/src/commands.rs`, `app/tests/commands.rs`.

**Tests first.** These are spec tests 23–26:
- `set_body_edits_a_task_body_and_returns_fresh_state` (23). The record's actor and via are read
  from `console_ctx(&vault)`, never from a literal.
- `set_body_refuses_an_approval_and_an_issue_by_name` (24). The error names the folder, and nothing
  is journalled.
- `set_body_conflict_carries_conflict_true_and_the_state` (25).
- `profile_commands_round_trip` (26): `set_preferences`, then `set_interests`, then `profile`.

**Behaviour.** Spec §8, D7 and D14.
- `set_body_inner(cs, view, id, expected, body)` refuses unless the resolved note lies in `tasks/`
  or `courses/`. The folder allow-list is a `const`, like `EDITABLE`. It then calls
  `write::set_body` with `console_ctx(&vault)` inside `mutate`.
- `profile_inner(cs)` runs under `cs.lock`, as `note_inner` does.
- `set_preferences_inner` and `set_interests_inner` run inside `mutate`.
- The envelope's `conflict: true` is set from the `WriteError::Conflict` variant, never by matching
  the error's text.
- Each gets a thin `#[tauri::command]` wrapper that attaches the scheduler, as its neighbours do.
- No hashing, parsing or deciding happens here.

**After it:** hand-off H2 (the four names into the console's `generate_handler!` list, none into
the wizard's), applied by the main session. The count is taken again.
**Done when:** the four tests pass, and `app/tests/commands.rs`'s existing tests pass unchanged.

### T6. The dropped-event list, engine half (`eventroster.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** it is a read-only parser, fully specified by P2
and checked by tests. The file is disjoint from T3 and from the events lane's `eventemit.rs`.
**Files:** `engine/src/eventroster.rs` (the function and its tests). **Before it:** Quinn's answer
to PQ2.

**Tests first:**
- `read_dropped_reads_the_fixtures_audit_section`. It runs on a scratch copy of
  `vault-full/state/events.md` (a frozen reference, read only through the copy). Every `· filtered`
  line in the audit section is returned with its title and date. Nothing from *Coming up* is
  returned. The copy's bytes are unchanged afterwards.
- `read_dropped_names_why`. A roster written by `write_roster` into a temporary vault, with a
  ledger holding one filtered, one declined and one judged-irrelevant event (with a `why`), yields
  P2's three reasons.
- `read_dropped_skips_unjudged_and_continuation_lines`. This holds under PQ2's recommendation; if
  Quinn says yes to PQ2, unjudged lines instead get the reason "not judged yet".
- `read_dropped_of_an_absent_roster_is_empty`.

**Behaviour:** P2. The function reads `state/events.md` and the event ledger and writes nothing. It
parses the audit line shape that `audit_lines` writes. The line's date and time are kept as text
the page can show; no new date arithmetic is done.
**Done when:** the four tests pass, and the frozen `events.md` reference test and the event tests
pass unchanged.

### T4b. The `dropped_events` command (`commands.rs`)

**Agent:** `implementer` (Sonnet, high). **Files:** `app/src/commands.rs`, `app/tests/commands.rs`.

**Test first:** `dropped_events_lists_the_audit_drops_and_writes_nothing`. A scratch vault holding a
copied roster gets `{ok: true, dropped: [...]}`, and the vault's bytes and journal are unchanged.

**Behaviour:** `dropped_events_inner(cs)` calls `eventroster::read_dropped` under `cs.lock`, as
`note_inner` does. The Tauri wrapper follows its neighbours. **After it:** hand-off H3 (one name).
**Done when:** the test passes.

### T5a. The drawer's body editor (`console.js`), in two dispatches

**Agent, both:** `console-ui` (Sonnet, medium). **Why:** `app/static` is its lane, and the static
tests pin the behaviour that matters. **Why two:** the whole editor is well over 80 lines; the
split puts the happy path first and the harder states on a working base. **Before T5a.1:** PQ1's
answer is applied (Gmail connect merged, or Quinn's word to go first), and the controller re-runs
T0's disjointness check. **Files, both:** `app/static/console.js`, `app/static/console.css`,
`app/tests/static_assets.rs`. Each ends with the console's existing static tests unchanged and
green, in its own commit.

#### T5a.1. View, edit and save

**Tests first** (spec tests 27 and 28, the drawer halves, in `static_assets.rs`):
- `the_drawer_saves_a_body_with_expected` (27): `openDrawer`'s editor invokes `set_body` with `id`,
  `expected` and `body`.
- `no_ui_event_carries_body_text` (28): body events use `object_kind: "body"` (it fails first
  because no such event exists yet), and no `ui_event` call passes the textarea's value.

**Behaviour.** Spec §9.1, for a note in `tasks/` or `courses/`:
- **View:** the escaped `<pre>` with *Edit*. An empty body shows *Add notes*.
- **Edit:** a textarea, with *Save* and *Cancel*, Ctrl+Enter to save and Esc to cancel. The
  loaded body is kept as `expected`.
- **Saving:** the controls stay disabled until the envelope returns.
- **Saved:** the drawer re-reads the note. (The toast is T5a.2's.)
- **Refusals:** every refusal goes to `showRefusal` for now; T5a.2 gives `conflict` its own state.
- No Markdown rendering. The text reaches the DOM only as escaped text.

**Done when:** both tests pass.

#### T5a.2. Conflict, undo and the poll exemption

**Tests first** (spec tests 29 and 32, the drawer halves):
- `a_poll_leaves_an_open_editor_alone` (29): `paint` and `poll` skip an element marked as an open
  editor.
- `body_undo_sends_the_reread_body_as_expected` (32): after a save, *Undo* invokes `set_body` with
  the re-read note's `body` as `expected`, never the textarea's value.
- `a_body_conflict_keeps_the_draft` (plan addition, spec §9.1's conflict state): an envelope with
  `conflict: true` leaves the textarea's draft in place and offers *Copy* (`copy_text`) and
  *Reload*.

**Behaviour.** Spec §9.1:
- **Saved:** a 10-second *Saved · Undo* toast.
- **Conflict:** the draft stays, under the spec's line, with *Copy* and *Reload*. *Reload* asks
  before it discards the draft.
- **Open editor:** marked so that a poll's repaint leaves it alone.
- Test 28 from T5a.1 stays green: *Undo* and *Reload* events carry no text either.

**Done when:** the three tests pass, and T5a.1's tests pass unchanged.

### T5b. Settings: "Your preferences" (`index.html`, `console.js`, `settings-check.py`), in three dispatches

**Agent, all three:** `console-ui` (Sonnet, medium). **Why three:** a new Settings section, the
interests lists with their read-only state, two undo paths and a Playwright walk are each a
separate piece, and together well over 80 lines. **Files:** T5b.1 and T5b.2 edit
`app/static/index.html`, `console.js`, `console.css` and `app/tests/static_assets.rs`; T5b.3 writes
only `scripts/settings-check.py` (P4). Each ends in its own commit, with the existing static tests
unchanged.

**Telemetry rule, both parts (D15).** Profile events reuse `edit_started`, `edit_committed` and
`edit_cancelled` with `object_kind` `preferences` or `interests`; no new action. No `ui_event` call
passes the preferences textarea's value, the interests lists' values, or any variable derived from
them (the draft, the cleaned items, the lists held for undo).

**The copy's facts are fixed** (Q8). The words may be tuned, but each line must keep them:
- 600 characters;
- sent for one call;
- not kept by us or the model's host;
- otherwise stays on this computer;
- for *Campus events*, the text goes to the judge with each event it judges.

Until bump #1 this copy is the only disclosure that interests reach a model.

#### T5b.1. *How you like to work* (preferences)

**Tests first** (the preferences halves of spec tests 27, 28 and 32):
- `settings_reads_and_saves_preferences` (27): the panel invokes `profile` when it opens and
  `set_preferences` with `expected` on *Save*.
- `no_settings_ui_event_carries_preferences_text` (28, Settings half). It asserts that a
  preferences event with `object_kind: "preferences"` exists (so it fails first), and that no
  `ui_event` call passes the preferences textarea's value or a variable holding it.
- `settings_offers_undo_for_preferences` (32): preferences undo passes the text `profile` returned
  after the save as `expected`.

**Behaviour.** Spec §9.2's first part: a new "Your preferences" section in Settings, and in it
*How you like to work* as a textarea with a counter that marks 600 characters, its own *Save*, and
the drawer's save, conflict, refusal and *Saved · Undo* behaviour. The copy carries the fixed
facts above.

**Done when:** the three tests pass.

#### T5b.2. *Campus events* (interests)

**Tests first** (the interests halves of spec tests 27, 28 and 32):
- `settings_reads_and_saves_interests` (27): the panel invokes `set_interests` with the four lists.
- `no_settings_ui_event_carries_interests_text` (28, Settings half). It asserts that an interests
  event with `object_kind: "interests"` exists (so it fails first), and that no `ui_event` call
  passes any of the four lists' values or a variable holding them.
- `settings_offers_undo_for_interests` (32): interests undo sends the four lists held before the
  save.
- `interests_are_read_only_when_not_editable` (plan addition, D10): when `profile` returns
  `interests_editable: false`, the four lists are read-only, *Save* is absent or disabled, and
  D10's reason is shown.

**Behaviour.** Spec §9.2's second part: four one-item-per-line lists, labelled *Always show me*,
*Maybe*, *Never show me* and *Clubs I'm in*, with their own *Save* and the same save, conflict,
refusal and undo behaviour. The *Campus events* copy line carries its fixed fact.

**Done when:** the four tests pass, and T5b.1's tests pass unchanged.

#### T5b.3. `scripts/settings-check.py` (spec test 33)

**What it walks:** "Your preferences", with a stubbed `invoke`:
- a preferences save, then *Undo*;
- an interests save, then *Undo*;
- a conflict that keeps the draft;
- the read-only block-list state, showing D10's reason;
- a check that the stub recorded no `ui_event` argument containing the text typed into either
  part. This is test 28's Settings half run against the live page, not only the source.

If Gmail's version of the script is on `main` by then, this task builds on it (P4).
**Done when:** `settings-check.py` prints `ok` with exit 0. The main session runs the script if the
agent has no Playwright venv.

### T7. *Not shown (N)* under *Coming up* (`console.js`)

**Agent:** `console-ui` (Sonnet, medium). **Files:** `app/static/console.js`, `console.css`,
`app/tests/static_assets.rs`. **After:** T5b.3 (T5b.2 shares its files), T4b and H3.

**Test first:** `coming_up_offers_a_not_shown_list_from_dropped_events`. The page invokes
`dropped_events`, renders a collapsed *Not shown (N)* list under *Coming up* with each item's
title, date and reason as escaped text, and offers no write action.
**Behaviour:** Appendix A (d). It is read-only and fetched when *Coming up* renders, not on every
poll. An empty list renders nothing. A refusal is one quiet line, never a blank.
**Done when:** the test passes.

### T8. Reference docs

**Agent:** `docs-keeper` (Sonnet, medium). **Why:** reference text only.
**Files:**
- `docs/reference/app.md`: the five commands, the two `profile/` files, the `set_body` op and its
  hash-only record, and D13's local-only rule.
- `docs/surface/anatomy.md`: the body editor, the "body edited" history line and *Not shown*.
- `docs/surface/inventory.md`: row 28.
- `docs/notes/2026-09-29-vision-program.md`: the P5 row, and the P6 row item by item: (a) cut,
  (b) cut from the MVP, (c) delivered by the events spec's D3, (d) done.

`docs/reference/engine-commands.md` does not change (spec §10). HANDOFF is not edited here: the
controller updates §3 item 3 at the MVP milestone, in one batch.
**Done when:** each claim cites a function, not a line, and matches the branch's code.

### T9. Whole-branch review

**Agent:** `reviewer` (Opus, high). **Why:** everything a cheaper agent changed is reviewed before
a push. **Report:** `docs/reports/<date>-m2-editing-whole-branch-review.md`.
- It reviews the whole diff against the spec, section by section, using §9's ledger as the
  checklist.
- It confirms Checkpoint A's report is closed. If a contract-list file changed after Checkpoint A,
  `contract-reviewer` re-reads that diff.
- It checks the following in particular:
  - `profile.rs` has no block-style detector;
  - `commands.rs` computes nothing;
  - no page path puts body or profile text into telemetry;
  - no `"quinn"` or `"student"` literal was added;
  - a rank run never stamps an id on `profile/` files (`ids` touches only `NOTE_FOLDERS`);
  - the copy keeps §9.2's facts.

Fixes go to the task's own agent, and the reviewer re-reads them. **Done when:** the report reads
ready, with no open Critical or Important finding.

### T10. Gate, live proof, PR and merge (main session, with Quinn)

**Why the main session:** hand-off counts, the staging session and the push are the controller's.
**Quinn:** the merge is a code push, so it needs Quinn's word. The proof runs under the standing
solo-proof approval: a dev build, the DOM driver, an OTP session and never OS input. No deploy is
needed, because M2 changes nothing on the server.

- **Gate:**
  - the workspace build and tests with 0 other warnings;
  - `scripts/ci/eol-check.ps1`;
  - the fixtures diff empty;
  - the four ignores in place;
  - `human_actor_literal.rs` green;
  - the console handler list recounted against H2 and H3.
- **Live proof, spec §11,** on a scratch profile against staging:
  1. Edit a task body without a trailing newline, and undo it.
  2. Force a conflict by appending through `knowlu-engine write` while the editor is open.
  3. Edit preferences and undo that.
  4. Edit interests.
  5. Check that `sync` pushed the task's record and note, and no `profile/` record.
  6. Open *Not shown* on a vault that has an events roster.

  Afterwards, remove the scratch profile.
- **Then:** the PR, CI green, and Quinn's word to merge. At the MVP's parity audit, Quinn marks P5
  and P6's four items done or cut, as Appendix A records.

## 6. Order, parallelism and checkpoints

- **Order:** T0 → T1a.1 → T1a.2 → T1a.3 → T1b → T2 → Checkpoint A → H1 → T3 → T4a → H2 → T6 →
  T4b → H3 → T5a.1 → T5a.2 → T5b.1 → T5b.2 → T5b.3 → T7 → T8 → T9 → T10.
- One implementer runs at a time. T6 is placed after T4a only for that reason: its files are
  disjoint, so it may run earlier if the controller has a free slot.
- **Waits on Quinn:** PQ1 before T5a.1 (T0 asks it), PQ2 before T6, and the merge word in T10.
- **Opus starts:** T0, T1a.1–T1a.3, T1b, T2, Checkpoint A, T9 and T10. Every other task starts on
  Sonnet, and a second failed attempt moves it to Opus at high.
- **Sibling lanes:** re-run the disjointness check before T1a.1 and before T5a.1. Events rebases onto
  M2 after M2 merges (shared: `console.js`, `static_assets.rs`).

## 7. Controller hand-offs (main session)

- **H1**, before T3: `pub mod profile;` in `engine/src/lib.rs`, beside the other modules.
- **H2**, after T4a: `set_body`, `profile`, `set_preferences` and `set_interests` go into the
  console's `generate_handler!` list in `app/src/main.rs`. None goes into the wizard's list.
- **H3**, after T4b: `dropped_events` goes into the console list.
- **At merge:** recount the console list and quote the number only after the count. The spec
  counts 47 at its base, plus Gmail's 3, plus M2's 5.

## 8. Risks

- **A fence rule that is wrong in the unsafe direction** would overwrite frontmatter. This is
  guarded by tests 1, 7, 8 and 9, by T1a.2's order (refusals before the record), and by
  Checkpoint A reading each T1a commit on its own.
- **A half-landed contract dispatch** would leave `write.rs` in an intermediate state. This is
  guarded by T1a's three dispatches, each green and committed on its own, and by T1a.2's stop
  point after step 4.
- **Student text reaching telemetry from Settings** would send profile text off the device
  (D15). This is guarded by T5b.1's and T5b.2's named telemetry tests and by T5b.3's check of the
  stubbed `ui_event` arguments on the rendered page, not only by T9's read.
- **A `value_spans_lines` false negative** would orphan list lines and silently empty the event
  filter. This is guarded by test 31's six shapes, by the err-toward-true rule, and by keeping the
  check in the primitive rather than in `profile.rs` (test 21).
- **The push cursor advancing past a withheld record** would lose records. This is guarded by
  test 16's second push and by placing the filter beside the local-card filter.
- **A crash between the record and the file** leaves a record whose `new` the file does not hold.
  This is accepted (spec §7.5): the page keeps the student's text, and `verify_tail` does not heal
  `append_body` today either.
- **Gmail connect landing mid-lane.** PQ1 and the disjointness check at T0 and before T5a.1 cover it.

## 9. Fidelity ledger

Each row is a requirement of the signed spec, the task that meets it, and what proves it. Test
numbers are spec §11's. G1 and G2 are this plan's additions (§2).

| Spec requirement | Task | What proves it |
|---|---|---|
| D1: `set_body` replaces only the body; the frontmatter is carried as text; journal first, file second | T1a.2 | tests 1, 2, 7; Checkpoint A |
| D2: a new op `set_body` with `field: null`, which judge-once, `verify_tail` and `reconcile` ignore | T1a.1, T1a.3 | tests 14, 12 |
| D3, Q1 (A): the record holds hashes and byte counts, never text | T1a.1, T1a.2, T1a.3 | `body_sha256_is_lowercase_hex_of_the_utf8_bytes`; test 3; Checkpoint A |
| D4: named refusals (unclosed fence, NUL, over 128 KiB, a `---` first line on a head-less file) before any record | T1a.1, T1a.2 | `the_new_write_errors_name_the_file_or_the_reason`; tests 8, 9, 10 |
| D5, Q2 (A): compare-and-swap, `Conflict`, nothing written | T1a.2, T4a, T5a.2 | tests 5, 25; `a_body_conflict_keeps_the_draft`; live proof step 2 |
| D6: one definition of the body; normalisation; a no-op writes nothing | T1a.2, T1a.3 | `set_body_normalises_by_d6`; tests 4, 6, 11 |
| §7.1 step 3: the compare is on normalised bodies | T1a.2, T1a.3 | test 30 |
| D7, Q3 (A): tasks and courses only in the app; other folders refused by name | T4a | test 24 |
| D8, Q5 (A): *Saved · Undo* for 10 seconds on all three editors; undo's `expected` is the re-read body | T1a.3, T5a.2, T5b.1, T5b.2 | test 30; test 32's three halves (`body_undo_sends_the_reread_body_as_expected`, `settings_offers_undo_for_preferences`, `settings_offers_undo_for_interests`); test 33; live proof steps 1 and 3 |
| D9: a body edit sets no field and does not queue judging | T1a.2, T1a.3 | test 12 (no field record); T9 checks it |
| D10, Q4 (A): interests as four one-line lists; the block-style refusal lives in `write.rs` | T1b, T3, T5b.2, T5b.3 | tests 31, 21, 18; `interests_are_read_only_when_not_editable`; `settings-check.py`'s read-only state |
| D11: profile files created only when absent, with no id, `create` record first | T1b, T3 | tests 13, 17, 19 |
| D12: preferences edited through `set_body` | T3 | test 19 |
| D13, Q6 (A): no `profile/` record leaves this computer | T2 | test 16; T2's grep report; live proof step 5 |
| D14: the app computes nothing; reading lives in `profile.rs` | T3, T4a | T9's check of `commands.rs` |
| D15: telemetry unchanged, no text (spec test 28: no `ui_event` carries a body or profile text) | T5a.1, T5b.1, T5b.2, T5b.3 | `no_ui_event_carries_body_text`, `no_settings_ui_event_carries_preferences_text`, `no_settings_ui_event_carries_interests_text`; T5b.3's stubbed-`ui_event` check; T9 |
| §4: contract-list edits by `contract-engineer` at xhigh, reviewed before push | T1a.1–T1a.3, T1b, T2, A | Checkpoint A's report, one commit at a time |
| §4: `append_body`, `write_literals` and `create` unchanged | T1a.1–T1a.3, T1b | the unchanged existing tests; test 31's byte equality; Checkpoint A |
| §4: no frozen reference regenerated; oracles unchanged | every task | the empty fixtures diff at each gate; T10 |
| §4, ruling 11: no new `"quinn"` literal; the actor comes from `console_ctx` | T4a | test 23; `human_actor_literal.rs` |
| Ruling 11's gate on every human write (after the spec was signed) | T1a.2, T1b | G1, G2 |
| §5: `PRIVACY_VERSION` and `site/` unchanged | every task | T9 checks the diff for `site/` and `account.rs` |
| §5, Q8 (A): the interests sentence is in bump #1's row; the in-app copy carries the facts | T0, T5b.1, T5b.2 | T0's check on `main`; T5b's copy; T9 |
| §6.2: record key order and bytes through `make_record` and `dumps_value` | T1a.2, T1a.3 | test 3 |
| §6.3: the canonical profile texts; `load_interests` reads the one-line form | T3 | tests 13, 20 |
| §7.3: the filter never advances the cursor over an unsent record | T2 | test 16 (second push) |
| §7.4: "body edited" in history and in the delta | T3 | test 22 |
| §7.5: nothing pulled back; no two-desktop merge built | none | out of scope (spec §10); T9 confirms nothing was added |
| §7.6: `profile::read`, `set_preferences`, `set_interests`; no detector in `profile.rs` | T3 | tests 17–21; T9 |
| §8: four commands in the console list only, with the `conflict` flag | T4a, H2 | tests 23–26; the recount in T10 |
| §9.1: the drawer editor (view, edit, conflict, polls, refusals) | T5a.1, T5a.2 | tests 27, 28, 29, 32 (drawer halves); `a_body_conflict_keeps_the_draft` |
| §9.2: "Your preferences" with its fixed facts | T5b.1, T5b.2, T5b.3 | tests 27, 28, 32 (Settings halves, split by part); `interests_are_read_only_when_not_editable`; test 33; T9's copy check |
| §10: no engine CLI subcommand; `engine-commands.md` unchanged | T8 | T8's file list; T9 |
| §11 gates and live proof | T10 | T10's gate list and proof steps |
| §13: ordered after ruling 11 (met) and after Gmail connect (PQ1) | T0 | the ledger's base record; PQ1's answer |
| Appendix A (a): cut | T8 | the vision-program row |
| Appendix A (b): cut from the MVP (Quinn's explicit cut) | T0, T8 | the events spec's aligned non-goal line; the vision-program row |
| Appendix A (c): delivered by the events spec's D3; M2 adds no test and no `eventemit.rs` edit | T8 | the vision-program row; T9 confirms `eventemit.rs` is untouched |
| Appendix A (d): the *Not shown* list, read-only | T6, T4b, T7 | T6's four tests, T4b's test, T7's test; live proof step 6 |
| Inventory row 28, P5 | T8 | the updated rows |
| Reviews land in `docs/reports/` | A, T9 | the two reports |

**Plan-only tests.** G1, G2, `body_sha256_is_lowercase_hex_of_the_utf8_bytes`,
`the_new_write_errors_name_the_file_or_the_reason`, `set_body_normalises_by_d6`,
`a_body_conflict_keeps_the_draft`, `interests_are_read_only_when_not_editable` and the Settings
telemetry tests are this plan's additions. Each pins a spec requirement that had no test of its own
or only a combined one; none changes a spec test's assertion.

### Review revisions (plan review, 2026-09-29)

Both findings were checked against the plan and spec §11 before the plan was changed.

- **Finding 1 (important), accepted.** Spec test 28 covers body *and* profile text, but only the
  drawer half had a named test, and T5a ran before Settings existed. T5b.1 and T5b.2 now each carry
  a named, fail-first telemetry test (`no_settings_ui_event_carries_preferences_text`,
  `no_settings_ui_event_carries_interests_text`), and T5b.3 checks the stubbed `ui_event` arguments
  on the rendered page. The D15 row cites them in place of "its Settings counterpart".
  - *Changed from the finding:* one test per part instead of the single suggested
    `no_settings_ui_event_carries_profile_text`. With T5b split, a single test written in T5b.1
    would either pass vacuously for the interests half or need its assertion widened in T5b.2,
    which the task rules forbid.
- **Finding 2 (important), accepted with two changes to the proposed split.** T1a, T5a and T5b are
  split as proposed, with their letters kept so every ledger row still maps to spec §13.
  - *Rejected in part: normalisation in T1a.1.* The finding put D6's normalisation in the first
    dispatch. It is a private function whose first caller is `set_body`. Landing it in T1a.1 would
    leave dead code and a warning, which breaks the 0-warning gate at T1a.1's end. It lands in
    T1a.2, with its own test.
  - *Noted: T1a.3's tests pin rather than drive.* Of T1a.3's six tests only test 4 (the no-op, which
    T1a.3 now owns) is guaranteed to fail first. Tests 3, 6, 11, 12 and 30 pin behaviour T1a.2
    builds. The alternative, holding back parts of T1a.2 so they fail, would leave `set_body`
    writing an incomplete record or separator between two commits on a contract-list file. That is
    the intermediate state the split exists to avoid. The ledger records each test's first result
    for Checkpoint A instead.
  - *Added:* T1a.2 has a named stop point after step 4, in case it still runs past about 80 lines.
