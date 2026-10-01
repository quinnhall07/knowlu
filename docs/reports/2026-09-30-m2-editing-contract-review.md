# Review: M2 editing, Checkpoint A (T1a.1–T2, the contract list)

Branch `m2-editing`, base `3fb4ac1` (the plan), head `b7fa42d`: six commits, four files, all on the
contract list (`engine/src/journal.rs`, `engine/src/write.rs`, `engine/src/sync.rs`,
`engine/tests/sync_contract.rs`). Reviewer: `contract-reviewer` (Opus, xhigh). Each commit was read
on its own against spec §4, §6, §7.1–§7.3 and D1–D13, then the range as a whole. This review ran
cargo itself (below). It also ran a throwaway probe test through the public API, which was deleted
and never committed, to confirm findings M1 and CF2 rather than infer them.

## Verdict

- **Pass.** 0 Critical, 0 Important, 3 Minor. **T3 may start** once H1 lands.
- The four items the SDD ledger left open for this checkpoint are ruled below (R1–R4).
- Three carry-forward points go into later briefs (CF1–CF3). None is a defect in this diff.

## Gate (run by this review, dev profile, `-j 2`, through the slot guard)

- Targeted: the 23 M2 tests in the engine library pass (T1a.1: 3, T1a.2: 9 + 1 pin, T1a.3: 6,
  T1b: 3 + the ruling test), and `sync_contract` passes 91/91.
- `cargo test --workspace --no-fail-fast`: **2057 passed, 0 failed, 4 ignored**. The four ignored
  are the known four (traps 4 and 5, the real-runtime smoke test, `run_slot_end_to_end`), each with
  its reason. `oracle.rs`, `surface_oracle.rs`, `human_actor_literal.rs`, `dependency_boundary.rs`
  and `workflows.rs` all ran green.
- Warnings: 1 accepted (`.rsrc`), 2 tallies, 0 other.
- Frozen references: `git diff --stat origin/main...HEAD -- engine/tests/fixtures` and the same
  against `3fb4ac1` are both empty. No `surface-today-*.json` was regenerated.
- Removed lines across the four files: exactly one, the old `OPS` constant. No existing line in any
  function body changed. The additions inside existing bodies are the three `Display` arms, which
  T1a.1 requires, and T2's filter in `build_push`, which spec §4 sanctions.
- No `"quinn"` or `"student"` literal was added. Tests take the actor from `HUMAN_ACTOR`,
  `LEGACY_HUMAN_ACTOR`, `read_human_actor` or their own `WriteContext`.

## What held, commit by commit

### T1a.1 `343ee6e`: the op, the hash and the error variants

- `OPS` gains `"set_body"` at the end, and the six ops before it are unmoved. `make_record` still
  refuses a near miss (`set_bodies`). `VIAS`, the record shape, `latest_by_field`, `human_set` and
  `human_edited` are untouched.
- `body_sha256` hashes through `ring` and returns lowercase hex. Both test vectors were checked here
  against Python's `hashlib`: the empty body and `café — 日本\n` match.
- `Conflict`, `Body` and `MultiLine` each have a `Display` line, and nothing in this commit raises
  them.

### T1a.2 `8b5bfed`: `set_body`

- **Order.** Gate → `resolve_target` → `load(path, false)` → split → compare-and-swap → (T1a.3's
  no-op) → D4 (NUL, a `---` first line when there is no head, size) → record → file. Every refusal
  comes before the record, and the unwritable-file test pins journal first.
- **Fence rule: errs safe.** `split_body` runs the loop `models::split_frontmatter` runs: the same
  opening test, the same closing test, and the same `rest.trim_start_matches('\n')`. On top of that
  it refuses wherever the writer (`ingest.rs:86-92`) disagrees: no newline, no closing line, or a
  closing line with trailing whitespace. Where the reader and the writer part ways, `set_body`
  refuses. Where it goes ahead, its `current` is byte-equal to `note_detail`'s body. A text that is
  not frontmatter by the reader (for example `\n---…`) is all body. Replacing it with a text that
  starts `---` is refused by D4.
- **Carried as text.** The head comes from `load`'s single read. `load` parses the frontmatter only
  to get the id, and nothing is re-dumped. Test 1 pins the CRLF head's bytes through the fence's
  line ending.
- **Size.** `out` holds only `\n`, and sync measures a note after `read_text` (`sync.rs:736`), so
  both count on one scale. The test pins both sides of the boundary.
- **Record.** It goes through `make_record` and `Journal::append`, so its bytes are `dumps_value`'s.
  `old` is the body as `note_detail` shows it, `new` is the normalised body, and `field` and
  `evidence` are null. `id` is the note's id when valid, else null. It holds no text.
- **G1.** An invalid file, a human token that is not the vault's, and an absent file are each
  refused with `Actor`, with nothing journalled. An `agent:` actor is not gated.
- **Process note, not a finding.** The dispatch landed +345 lines in one commit (about 105 of code)
  and did not take the plan's stop point after step 4. One pin (D6's blank line) was written after
  the code, and a stalled attempt's tests were adopted after review. No defect came of it, and this
  checkpoint read the diff whole.

### T1a.3 `5e85930`: the no-op and the pins

- The no-op sits after the compare-and-swap, so a stale copy is still a `Conflict`, and before D4
  (spec §7.1 step 4). It leaves a file that is not in D6's form exactly as it is.
- First runs, from the ledger: test 4 failed first. Tests 3, 6, 11, 12 and 30 passed first as pins,
  so they name no T1a.2 defect. Their assertions are real:
  - test 3 compares the on-disk line with `dumps_value` and looks for markers, including non-ASCII;
  - test 6 chains through `surface::note_detail`;
  - test 11 checks bytes;
  - test 12 runs `verify_tail` and checks the fingerprint.
- `latest_by_field` skips a record with no id (`journal.rs:318`) and reads only `set` and `create`,
  so a `set_body` record is invisible to judge-once whatever its id.

### T1b `a85f281` and its fix `e3f7f2e`: the profile primitives

- **`create_profile_file`:**
  - It checks, in order: the gate, the two-name allow-list, `inside_vault` (a `profile` link that
    points outside the vault is refused), and `symlink_metadata` for `Exists`.
  - It journals the `create` record first, with a null id and `new` set to the frontmatter mapping.
    It creates the folder and the file second, and stamps no id.
- **`value_spans_lines` checks the line the surgery replaces.** It finds the key with the surgery's
  own prefix test and the closing line with its own exact-`---` test (`ingest.rs:92`, `:102-103`).
  Every rule it adds beyond §7.2 makes it true in more cases, never fewer (R1). Knowlu's own
  `interests.md` stays editable in both LF and CRLF: the emitter runs at `width=10**6` and never
  wraps a flow list.
- **`write_one_line_literals`** runs, in order: the gate, one read, `NoFrontmatter`, `MultiLine`,
  then `write_literals`, which is unchanged. Test 31's pass case is byte- and record-equal to
  `write_literals` on a copy.
- **`e3f7f2e` closes a real hole.** `write_literals` journals a record per key before the surgery
  refuses a file with no frontmatter by the writer's rule. The wrapper now refuses first, and the
  four new cases retry twice with an unchanged fingerprint.
- **`verify_tail` cannot re-apply a profile `set` record.** It finds notes by id through
  `build_index` (`passes.rs:202-205`), and profile records carry a null id. So a hand edit to
  `interests.md` is never overwritten by a stale record through the unguarded surgery.

### T2 `b7fa42d`: profile records stay on this computer

- The filter is one `continue`. It sits after `sync_cards.covers` and before the `op`/`actor` check.
  It comes before any `next`, `PAGE` or budget accounting: `PAGE` counts `batch.records`, which a
  withheld record never joins. Test 16's second, third and fourth pushes pin the "no wedge" and "no
  re-send" behaviour.
- `is_note_path` → `inside_vault` → `resolve_lenient` works on missing paths (`ids.rs:208-213`).
  So `delete` and `move` records about notes that are gone still go up.
- `apply` (`sync.rs:1798`) and restore (`:991`) refuse by the same function, and so does a record
  with no path (`unwrap_or_default()` → `""`). Every class the implementer's grep lists as now
  withheld was already unusable on every other computer: the profile paths, CLI writes outside the
  note folders, records about a hand-named note whose name fails the server's rule, and a legacy
  line with no path. The account loses nothing any computer reads.
- `record_is_well_formed` is not edited. Test 15 shows a 100 KiB body gives a well-formed record
  under 1 KiB that holds no body text.

## Rulings on the ledger's open items

- **R1. `value_spans_lines` goes beyond §7.2's rule (T1b): accepted.**
  - *What.* It is also true where the reader and the surgery disagree: they close on different
    lines, the reader cannot parse the frontmatter, or the key's line read alone does not hold the
    value the reader reads.
  - *Why.* Each added case is a false false under §7.2's letter. A flow list continued at column 0
    would be orphaned. A quoted key, or a key on the opening line, would be duplicated. §7.2's own
    principle ("errs toward true") calls for the widening, and it only ever narrows what gets
    written.
  - *Cost if wrong.* A hand-written file the two sides disagree about is shown read-only. No file
    Knowlu writes is affected, and the ruling's test pins that.
- **R2. For a file with only a spaced closing line, `NoFrontmatter` comes before `MultiLine`
  (`e3f7f2e`): accepted.**
  - *Why.* Both refuse before any record, and `NoFrontmatter` is the accurate name. The page never
    offers that save anyway: `value_spans_lines` is true for such a file, so `interests_editable`
    is false.
  - *Cost if wrong.* Only the wording a caller that bypasses the page sees.
- **R3. `interests_editable` for an `interests.md` with no frontmatter the writer can edit: no
  `write.rs` change now; T3 follows §7.6 to the letter.**
  - *What happens.* Such a file reads as four empty lists and is shown as editable. Save is then
    refused by name ("profile/interests.md has no readable frontmatter"), and nothing is written.
  - *Why.* Only a hand-made or truncated file can be in this state, because D11 always creates the
    file with frontmatter.
  - *Cost if wrong.* That student needs a text editor, the cost Q4 (A) already accepts for block
    lists.
  - *If Quinn wants it shown read-only.* Add one public predicate to `write.rs`,
    `one_line_refusal(text, keys) -> Option<WriteError>`, called by both
    `write_one_line_literals` and `profile::read`, so the page and the primitive can never
    disagree. It is contract-engineer work, about 20 lines.
- **R4. A `move` whose `path` is a note but whose `new` is outside the note folders still goes up:
  accepted as is.**
  - *Why.* §7.3 filters on `path` only. `apply` refuses such a move (`sync.rs:1812`). No app
    command calls `move_note`: only the CLI (`write.rs:1256`) and `apply` do, and `apply` checks
    the destination first.
  - *Cost if wrong.* The account keeps one path string no computer applies. The two-desktop stream
    can close it when it touches `apply`.

## Findings

- **M1 (Minor, T1b): a YAML anchor on the key's line is a false false.** Probe:
  `strong: &x [a]` with `extra: *x` makes `value_spans_lines(text, "strong")` false. The write
  succeeds, and `load_interests` then returns empty lists with the warning "interests unreadable:
  unknown anchor". The line read alone holds the value the reader reads, but removing the line
  breaks another key.
  - *Why only Minor.* It is a hazard the single-line surgery already has on every `write_literals`
    call, not one M2 introduces. Knowlu never writes anchors (`yamlemit` has none). `set_interests`
    sends all four keys, so an alias on one of them is caught; only an alias on a fifth,
    hand-written key slips through. And the reader warns rather than failing silently.
  - *Fix, if taken.* In `value_spans_lines`, also return true when the frontmatter with the key's
    line removed no longer parses, or reads any other key differently. That is one more read-only
    parse, done by contract-engineer.
- **M2 (Minor, T1b): a read-then-read window.** `write_one_line_literals` checks its own read, and
  then `write_literals` reads again. An external editor that saves a block list inside that window,
  which is microseconds long, would get past the check. The app's own writes and the slot's run
  under the vault lock. Closing the window would mean changing `write_literals` (forbidden) or
  duplicating it. Accepted; noted for the two-desktop stream.
- **M3 (Minor, docs): spec §6.2's example line is not `dumps_value`'s bytes.**
  - *What the journal holds.* `dumps_value` sorts keys and uses Python's `", "` and `": "`
    separators. The probe's real line is `{"actor": …, "device": …, … "new": {"bytes": 4,
    "sha256": "…"}, …}`. The example shows compact separators, `make_record`'s order and `sha256`
    before `bytes`.
  - *The code is right.* The binding sentence ("its bytes are `ledger::dumps_value`'s") holds, and
    test 3 pins it. The example should be marked illustrative, as its actor already is. T8 or a
    spec erratum can do that, with no code change.

## Carry-forward (not defects in this diff)

- **CF1 → T3.** `set_preferences` must create the file with `create_profile_file("preferences",
  "")` and put the student's words through `set_body`, as the plan says. Passing the student's
  text to `create_profile_file` would skip D4's NUL, size and `---` refusals. It would also journal
  any frontmatter the text starts with as `new`.
- **CF2 → T4a.** The primitive takes any target `resolve_target` accepts (D7), and the app's `id`
  may be a path. Probe: a human `set_body` on `config/actor.yaml` returns `Ok(true)`, and every
  human write is refused from then on. The plan already puts the allow-list on the *resolved*
  path. Recommend that test 24 also refuse a path-shaped id (`config/actor.yaml`) by name.
- **CF3 → T5b and T9.** A profile `set` record carries the interest items as `old`/`new`. That is
  the ordinary field-record shape, and it stays local under D13. `issues::open_issue` copies a
  target's frontmatter, or a flagged record, into `issues/`, which syncs. T9 should confirm that no
  page path opens an issue on a profile file or a profile record.

## Where the implementation departs from the signed spec or the plan

- §7.1 has no gate, because the spec predates ruling 11. The plan adds it (G1, G2), and the code
  follows the plan.
- §7.2's `value_spans_lines` rule is widened toward true (R1).
- §7.2 says `write_one_line_literals` "otherwise does exactly what `write_literals` does". It now
  also refuses a file with no writer frontmatter before any record (`e3f7f2e`, R2). The departure
  is on the side of journal first and D10's "before any record".
- §6.2's example bytes (M3).
- The plan's size guidance for T1a.2 and T1b (about 80 lines, with a stop point) was not followed.
  This is a process point, not a code point.

## Not verified

- The CLI path (`knowlu-engine write …`) against a profile target. No M2 code reaches it, and T2's
  grep names it.
- Behaviour on a network share or with a file held open by another process. `write_text` is the
  same non-atomic write every path uses (spec §0).
- Two-desktop body merging (§7.5), which is out of scope.
