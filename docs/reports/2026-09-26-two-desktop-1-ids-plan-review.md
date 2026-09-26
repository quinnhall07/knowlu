# Review: two desktops, plan 1 of 3 (deterministic ids, apply by id, event verdicts)

Plan: `docs/plans/2026-09-25-two-desktop-1-ids-plan.md` at `4374142` on `two-desktop`. Spec:
`docs/specs/2026-09-25-two-desktop-design.md` (signed). Read against the code at `4374142`: `sync.rs`
(`apply`, `materialise`, `restore_all`, `fold_confirmed`, `live_sync_cards`), `reconcile.rs`, `journal.rs`
(`latest_by_field`), `ledger.rs` (`read` sorts by `ts`), `passes.rs` (`load_index`, `detect_external`),
`write.rs`, `ids.rs`, `approvals.rs`, `coursework.rs`, `ingest.rs`, `enrich.rs`, `events.rs`, `eventledger.rs`,
`cli.rs`, `main.rs`, the `judgments` migration and `judge_pipeline.ts`. No build or test was run; the four
spec reference values and `task_d0fd865fb3` were re-computed with `sha1sum` and match.

## Plan Review

**Status:** Issues Found

**Issues:**

**I1. [Tasks 2, 5 and 7; D1 with D5 (a); R-TD1-18 / S-4] Another desktop's `create` for an id held here
poisons every `latest_by_field` read.** D5 (a) journals the foreign `create`. `journal::latest_by_field`
expands a `create` into one entry per field, and `JsonlLedger::read` orders by `ts`, so on the desktop that
created first, the other desktop's later `create` becomes the latest value of every field it minted. That
has two effects:
- (a) `passes::detect_external` runs on every `rank`. It journals a made-up `quinn`/`external` set for every
  field whose first value differed between the two desktops: the per-desktop title or course label (test
  (ii)'s own data), an LMS item's template `importance: 3` or `needs_enrichment: true` that the first
  desktop's judge has since changed, or `status: archived` against `active` for an imported-past twin. The
  record travels and applies cleanly on the other desktop, and `human_set` then locks every judged field
  under judge-once on both desktops.
- (b) In `apply`'s step 4, `mine` keeps the other desktop's `create`, because R1's `never_took_effect`
  (Task 7's `took_no_effect`) filters `set` records only. `up_latest` for such a field is then a value the
  note never held, and `reconcile::resolve` falls back to the file-mtime stand-in. That re-opens R1's
  silent loss (`a_later_foreign_write_never_loses_to_an_unrelated_local_mtime_bump`) for every item two
  desktops both imported. Example: A judged 4, and B's `create` (3) arrived on an earlier page. B's later
  edit 2→5 loses to A's mtime after any unrelated write on A. The only trace is a supersede record: no
  card, and A keeps 4.
- `merge_into_winner`'s `upstream` has the same gap. R-TD1-18's rebuild under the winner has it too,
  inside a group: its test covers only the favourable order, where the other desktop's `create` is dated
  earlier.

*Why it matters:* D13's fail-open rests on "D1–D6 make a double run safe". Before Plan 3, and after it
under fail-open, every item two desktops both import goes through this. No test in the plan runs
`detect_external`, or a later edit, after two `create`s whose fields differ.

*Fix:* one rule in both readers. Another device's `create` never supplies an expected or upstream value
for an id (or alias group) that this device minted itself. It still counts for a note that arrived here
as a pulled text, with no local `create`. RED tests:
- td1_ii's end state, then `passes::detect_external` on both desktops: no `external` line;
- the mtime-bump test with the foreign `create` on an earlier page and a later foreign `set`: one card or
  a clean apply, never a keep that leaves only a supersede record;
- R-TD1-18's test with the other desktop's `create` dated after this desktop's.

**I2. [Task 7, Steps 6–7; R-TD1-9] With both files here and a field set on both sides, one apply files
two sync cards.**
- `merge_into_winner` → `settle_resolution` files the card through `propose_amendment`. But `own_created`
  is computed before the file settle, and `live_sync_cards` only returns cards whose id is in `own`.
- Step 4's full reconcile of the same new group then reaches the same conflict, because the other
  device's conflicting `set` is in `everything`. It cannot see the merge's card, so it files an identical
  second one.
- Traced: A holds its own a (5 at t1) and B's copy b (B set 2 at t2 > t1). The merge files {5→2}. Step 4
  then weighs A's record at t1 against B's at t2 and files {5→2} again.
- The result costs two units of the daily cap and breaks §6.1 (v): "a field set on both becomes one card;
  the same with both files on each side". It is untested: the N1 test sets a field on B only.

*Fix:* make `own_created` mutable and add the id of any card `settle_resolution` files, read back from
the path `propose_amendment` returns. Or skip step 4's full pass for a winner the settle has just merged.
Then set `importance` on both sides in `td1_v_n1…` and assert `ra.cards + rb.cards == 1`.

**I3. [Task 8, Step 4; R-TD1-12] On a restore that spans pages, the path the restore undid is still in the
cursor.**
- `restore_all` calls `fold_confirmed` after every page, and that function only ever inserts
  `state.written_notes` into `cursor.notes`.
- When the earlier row for an id lands on page 1 and the later row on page 2,
  `state.written_notes.remove(&earlier)` comes too late. The cursor already holds the earlier path and
  the file is gone.
- The first push after the restore then sends a tombstone for that path, which archives the other
  desktop's live copy. That is exactly the harm R-TD1-12 exists to prevent.
- At 500 rows a page, the two rows' revs are usually far apart, so the cross-page case is the likely one.
  The test uses a single page.

*Fix:* keep the undone paths in `RestoreState`, and remove them from `cursor.notes` after the loop,
beside the existing `pending_tombstones` removal. Add a two-page variant of `td1_vi…` whose first reply
carries `more: true`.

**I4. [Task 10; spec §2.8, a spec defect the plan should name as S-8] Deleting a line to force a re-judge
fails for this desktop's own verdicts, at the usual timing.**
- The pull runs in `sync`, the slot's first step. The events pass runs in `judge`, after it. Plan 3's
  trailing `sync (push)` does not pull.
- So the cursor has not yet passed the rows this desktop's judge has just written.
- Example: the student deletes the morning slot's verdict line at noon. The evening sync pulls that
  verdict back from the account, and the re-judge never happens.
- The plan's test deletes a line that came from a pull, which is the one case that works.

*Fix (needs a ruling):* pass this desktop's own rows while their lines still exist. Either
`enrich::run_lines_with` calls `pull_event_verdicts` after the events pass, taking `sync`'s run lock
around the cursor file, or Plan 3's trailing sync also pulls verdicts. Add a test: a verdict judged here
after the last pull, its line deleted, then the next pull does not restore it.

**I5. [Task 7, Step 6; the spec is silent, S-9] When one file of an alias group is in `archive/`, id order
decides whether the item stays live.** Three cases:

| This desktop holds | What the plan does |
|---|---|
| winner archived, loser live | merges the live loser into the archived winner and archives it |
| lowest held loser archived, another loser live | re-identifies the archived one, then merges the live one into it and archives it |
| loser archived, winner live | leaves the loser; the winner stays live |

Before the upgrade, the natural thing for a student to do is delete one of the two copies they can see.
About half the time, the first sync after the upgrade then archives the whole item on both desktops.
D5 (b)'s delete-by-alias does the same when that pre-upgrade `delete` record is pulled later. Nothing is
lost, since the item is in Archive, but the outcome is silent and decided by the order of two random ids.

*Fix:* bring it to Quinn as a ruling. Recommended rule: an archived file never absorbs a live one. Among
a group's files on this desktop, the live one is kept, archived ones are left as settled, and
`IdIndex::holder` prefers a live member. Add a test for each row of the table.

**I6. [Task 10, Step 6; minor] Within one page, the pull records the oldest verdict for a uid.** Rows come
ascending and `taken` keeps the first one. After a deleted line has been re-judged, a fresh or lagging
desktop takes the superseded verdict, and keeps it for good, because a pulled verdict never replaces a
line.

*Fix:* the last row for each uid in the page wins (collect the page first, then write). Add one
assertion to `td1_d8_a_pulled_verdict_fills…` with two rows for one uid.

Buildability: nothing blocking. Every task names its files, lines, interfaces and exact code. The hand-off
anchors in `ingest.rs`, `cli.rs` and `main.rs` match the code at `4374142`, and so do the helpers Tasks 5–8
reuse from `sync_contract.rs` and the `Touched`/`RestoreState` shapes. Two wording nits are listed under
the advisory recommendations.

**Rulings assessment:**

| Ruling | Assessment |
|---|---|
| R-TD1-1 | Sound. |
| R-TD1-2 | Sound: D5 (a)'s own sentence, carried to every step. |
| R-TD1-3 | Sound. Consider wording the merge line "its fields merged into it", since bodies are not merged (spec §7). |
| R-TD1-4 | Sound for the slot, but a pull in `sync` alone cannot pass this desktop's own verdicts in time (I4). |
| R-TD1-5 | Sound. |
| R-TD1-6 | Sound: the roster is one of the spec's two sources. |
| R-TD1-7 | Sound. |
| R-TD1-8 | Sound: `oracle.rs` and the frozen references are untouched by construction. |
| R-TD1-9 | Change it. The loser-against-winner merge is the right reading of §2.6, but add the merge's card to `own_created` and test a field set on both sides (I2). |
| R-TD1-10 | Sound. The same `mine` also carries other desktops' `create`s (I1). |
| R-TD1-11 | Sound. |
| R-TD1-12 | Change it: remove undone paths from the cursor, not only from `written_notes` (I3). |
| R-TD1-13 | Sound, and faithful to §2.5 (d) and to §3.2's caveat. |
| R-TD1-14 | Sound: no migration, every select scoped to the account, and `judge_db_test.ts` holds them to it. |
| R-TD1-15 | Sound. |
| R-TD1-16 | Sound: comparing across two vaults is the stronger claim. |
| R-TD1-17 | Sound. |
| R-TD1-18 | Change it: widen it from alias groups to "another device's `create` for a note minted here", and make it independent of record order (I1). |
| S-1 | Sound to defer. The proposed `cmt` arm leaves out `ics-series:`: under the commitment spec's R1 and R6, a direct-ICS series is imported on each desktop independently, exactly like `gcal-series:`. It should also rule on `card:`. |
| S-2 | Sound. |
| S-3 | Sound. |
| S-4 | Sound but understated: the hazard comes from D1, not only D6 (I1). |
| S-5, S-6, S-7 | Sound. |
| Missing | S-8 (I4, deleting a line) and S-9 (I5, an archived file in a group) belong in the list for Quinn. |

**F-1:** Real.
- *Where:* `passes::load_index` and `detect_external` (`passes.rs:98-146`, `:259-350`) take each field's
  expected value from `latest_by_field` over the whole journal, ordered by `ts`.
- *How it fires:* a withheld foreign `set` has no local echo, so it is the latest record while the note
  keeps this desktop's value. `detect_external` then journals a `quinn`/`external` set restating the local
  value, stamped with the file's mtime.
- *The damage:* on the other desktop, `resolve` sees "upstream never moved", since its note holds the
  record's `old`, and applies it. The student's later value is replaced there with no card, and this
  desktop's card now offers a value neither desktop holds.
- *Why it matters for Plan 1:* its own test (i) ends in exactly this state. A withholds B's 2 behind a
  card; A's next `rank` makes up a 5; the next exchange turns B to 5. So §6.1 (i)'s "one card, never
  silent divergence" holds only until the next slot, and §6.4 step 5 will run into it.

*Recommendation:* carry it in Plan 1, as one task together with I1. It is the same function and the same
class of bug: a record that never took effect here supplies an expected value. RED first: td1_i's end
state, then `passes::detect_external` on A gives no `external` line, and after one more exchange B still
holds 2. Proposed fix for the ruling: a field named by a live (pending or snoozed) sync card that this
desktop filed on the note is expected at the note's own value.

**Recommendations (advisory):**
- **Task 6, moves of archived notes.** A foreign `move` whose holder is in `archive/` brings the note back
  to a live path. D5 (b) says "from wherever it is", but D7 says the archived copy wins. Today only
  `knowlu-engine write move` produces a `move` that travels, so the impact is low. Mirror the delete rule:
  skip an archived holder unless `new` is in `archive/` too.
- **Overlaps with `origin/j-followups` and `p1-commitments`.** Both add `jid:` to machine verdict lines,
  and `record_answer`, a human answer that supersedes `unsure`. After the merge, the pull should write
  through `record_judged_verdict` with the row's `id` as `jid` (an id, not free text). The overlap table
  should also say that a student's answer to an `unsure` stays on one desktop, because the pull brings
  only the machine's word.
- **Task 7, Steps 3 and 7.** Step 3 already replaces `never_took_effect` and `mtime_ts`, yet Step 7 tells
  the implementer to replace `!never_took_effect(r)` again. Say once which step makes the change.
- **Task 7, Step 9.** Fold the `create_dir_all(archive)` line into the test's code block instead of
  leaving it in a parenthetical.
- **D4 in steady state.** Every pull of the other desktop's changed copy prints `sync: <path> is <id>,
  already held here as <local path>`, a recurring line in the Runs view during normal use. Body edits
  also never cross for an item at two paths, which spec §7 accepts. Name both in HANDOFF, or print the
  line only once per id.
- **Imported-past twins.** A twin archived on one desktop and created active on the other stays split for
  good, because the holder check blocks both texts. It is rare (a source new to only one desktop), but
  worth naming.
- ***Sync now* before Plan 3.** Until Plan 3's busy guard lands, *Sync now* runs the verdict pull
  in-process beside a slot's `judge` child. `load_ledger`'s first-verdict-wins rule makes a doubled line
  harmless.
