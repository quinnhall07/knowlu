# Two desktops on one account, plan 1 of 3: deterministic ids, apply by id, event verdicts

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two desktops that both run the machine steps converge on one note per imported item: every
imported note takes an id derived from the item, sync applies records, moves, deletes, tombstones and
texts by id, an item already doubled under two ids is joined by an alias pre-pass, and event proposals
and verdicts hold across desktops.

**Architecture:** The engine only, plus one read route on an existing cloud function. `ids.rs` gains
the derivation (`import_id`), the one function that says whether a note is imported (`import_key`), and
the alias map (`state/id-aliases.json`); `write.rs` gains `create_imported`, which the nine producers
call instead of `create`. `sync::apply` builds one id index per run and finds every note by id, runs
the alias pre-pass before its reconcile, and writes one file per id; `materialise` keeps one file per
id on a restore. `rank --no-digest` leaves the digest to the `feeds` holder, and every desktop's `sync`
pulls the account's event verdicts (the word only) through a new `GET /judge-event`.

**Tech Stack:** Rust 1.98 (`stable-x86_64-pc-windows-gnu`), one Cargo workspace (`engine/` =
`knowlu-engine`), `sha1` (already linked for `ids::derived_id`), `serde_json`, `serde_yaml_ng`, `jiff`;
Deno 2 for `cloud/supabase/functions/judge-event/`.

**Spec:** `docs/specs/2026-09-25-two-desktop-design.md`, **SIGNED by Quinn 2026-09-25**. This plan
carries D1–D8 (§2 whole), the §5.4 rows ruled **In** that fall to them (E5, M5), the §6.1 engine tests
for all of it, and §6.3's `judge-event` GET. D9–D19 are Plans 2 and 3 (see *Spec map*). Where this
plan and the spec disagree, the spec wins and this plan is wrong; every place the spec is silent is a
**Plan ruling** below, repeated at the step it changes.

**Status: PLAN, written 2026-09-25 at `3e6e13b` on branch `two-desktop`. Not executed.** Every
`file:line` below was read at `3e6e13b`. Earlier tasks shift the lines a later task names, so a later
task finds its place by the code it quotes; the line numbers say where that code stood at `3e6e13b`.
**Checked while writing:** every task's code and tests, with hand-offs H1–H3, were applied in order to a
scratch export of `3e6e13b` (never this worktree); `cargo test -p knowlu-engine` passed whole at 0
warnings (`oracle.rs` and `surface_oracle.rs` included; one test that needs a git checkout skipped), and
the whole `cloud/supabase` Deno suite with `judge-event`'s new files, `deno check` and `deno lint`
passed. The app crate was not rebuilt (this plan changes no app file and no engine type the app
constructs). Three plan defects found that way are fixed in this text.

## Global Constraints

Binding on every task. Where a line quotes `CLAUDE.md`, it is verbatim.

1. **Where.** Work only in `C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop`, on branch
   `two-desktop`. Absolute paths and `git -C <worktree>`; **never `cd`**. `.superpowers/` is
   git-ignored scratch.
2. **0 warnings.** "`cargo build --workspace` and `cargo test --workspace` from the root. **0 warnings
   is part of green.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple
   non-default manifests` linker message." The CI gate prints `warnings: N accepted (.rsrc), N tallies,
   N other`; the last number must be `0 other`.
3. **Frozen references.** "**Never regenerate a frozen reference.** Eight Python-written references
   live in `engine/tests/fixtures/`: `golden-today-s1.md`, `golden-today-full.md`,
   `calendar-snapshot-gcal.md`, `vault-full/state/events.md`, `zybooks-parsed-reference.json`,
   `vhl-parsed-reference.json`, `run-records-reference.json`, `pyyaml-safe-dump-reference.json`. If the
   engine disagrees with one, the engine is wrong. The three Rust-generated read-model references —
   `surface-today-{s1,s1-migrated,full}.json` (`engine/tests/surface_oracle.rs`) — have their own rule
   (console spec §4.6): regenerate only in a commit whose diff shows the change and whose message says
   why." **No task in this plan changes any of the eleven**, and none may: `engine/tests/oracle.rs` and
   `engine/tests/surface_oracle.rs` stay green and unedited at the end of every task.
4. **Line endings.** "**Line endings: LF everywhere in this repo** (`.gitattributes`: `* text=auto
   eol=lf`; `*.ps1` are CRLF). `engine/tests/fixtures/**` is `-text`: those bytes are the contract —
   several are compared byte for byte and they are CRLF because vaults are — **never re-encode them**."
   No task writes under `engine/tests/fixtures/`.
5. **Journal first; never re-dump a note.** "Never rewrite a vault file wholesale. Every note write
   goes through `write`: journal record first (`state/journal/YYYY-MM-DD.jsonl`, UTC days),
   single-line frontmatter surgery second. **No note is ever parsed and re-dumped**". This plan adds no
   exception: every new note write is `write::create_imported`, `write::write_literals`,
   `write::delete` or `write::move_note`; the one pulled-text write it touches (`apply`'s step 6, and
   `materialise` on a restore) is C3′'s existing recorded exception, bounded as before.
6. **Contracts.** "`journal::VIAS`, run records, ledgers and note frontmatter are contracts with
   existing vaults: byte-identical, never renamed." "All JSON the crate writes goes through
   `ledger::dumps_value`". The new `state/id-aliases.json` and the new `Cursor` field are written
   through it.
7. **`rank` never calls a model** (Knowlu spec decision 11). `engine/tests/cloud_contract.rs::
   rank_cannot_reach_a_judgment_endpoint` reads `engine/src/cli.rs` as text and fails if it contains
   `/judge-task`, `/judge-event`, `/judge-email`, `judge_task`, `CloudModel`, `EventModel` or
   `EmailModel` — **comments included**. Hand-off H2's text contains none of them.
8. **`rank --no-digest`'s default is unchanged**, so `oracle.rs` and the frozen references are
   untouched: `cli::run` and `cli::run_with` keep their signatures and pass the default options.
9. **Determinism.** Same input, same order: every new map is a `BTreeMap`/`BTreeSet`, every new scan is
   sorted, and the alias winner is the lowest id as a string.
10. **`sync.rs`'s own text guard.** `engine/tests/dependency_boundary.rs::
    the_sync_module_holds_no_key_and_no_envelope` fails if `engine/src/sync.rs` contains any of
    `SyncKey`, `recovery`, `Recovery`, `seal(`, `fn open(`, `IndexKey`, `HKDF`, `hkdf`, `aead`,
    `CROCKFORD` — in code or comments. Nothing added to `sync.rs` may use those words.
11. **Credential Manager.** "**Tests that touch the real Credential Manager are serialised.**" No test
    in this plan touches it: every cloud call is `CloudClient::new(&cfg, "jwt-not-a-secret")` against a
    `127.0.0.1:0` loopback.
12. **`KNOWLU_DEVICE` is process-global.** No test sets it. Two desktops in one process are told apart
    by relabelling records, as `engine/tests/sync_contract.rs::transfer` does. A unit test under
    `engine/src/` that compares `journal::device_name()` holds `crate::journal::DEVICE_ENV_MUTEX`.
13. **Tests run in the foreground**, one cargo at a time, with `-j 2` (host memory is low). Never
    `cargo test --release` (it will not link). Before any cargo command in a fresh shell, refresh PATH
    (it also brings Deno):
    `$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"`
14. **TDD.** In every task the failing test is written first, run, and seen failing for the stated
    reason; then the code.
15. **Hand-off files are not edited by a task.** `engine/src/{ingest,cli,main,lib}.rs`, `Cargo.toml`,
    `Cargo.lock`, `CLAUDE.md` and `HANDOFF.md` are shared single-owner files (`HANDOFF.md` §2). Their
    changes are hand-offs **H1–H4** (*Controller hand-offs*), applied verbatim by the controller as
    their own commit at the task that names them. A task that silently edits one is a plan defect:
    stop and report it.
16. **No push, no `supabase` command, no `knowlu.exe`.** The controller deploys `judge-event` to
    staging after Task 10.
17. **Commits.** `git -C <worktree> add <paths>` (never `-A`), the message written with the Write tool
    to `C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-N.txt`
    (Git Bash heredocs halve backslashes), then `git -C <worktree> commit -F <that file>`. Every
    message ends with the two trailers the controller's dispatch names; this plan's own commit used
    `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>` and
    `Claude-Session: https://claude.ai/code/session_0116SUa7sU6BMt66hiEXrjTU`.
18. **No personal data.** Invented titles and uids; accounts `acct-1`; hosts `127.0.0.1`.
19. **The whole suite at the end of every task:** `cargo build -p knowlu-engine -j 2`, then
    `cargo test --workspace -j 2`, green at 0 warnings. (The app's tests spawn the real engine exe, and
    `app/build.rs` drops a zero-byte placeholder over it, so the engine is built first.)

## Spec map

| Spec | Where it lands |
|---|---|
| §0, §1 D1, §2.1 — `ids::import_id` and its four reference values | Task 1 |
| D2, §2.2 — `ids::import_key`, `ids::IMPORT_VENDORS`, the nine producers | Task 1 (the key), Tasks 2–3 (the producers) |
| D3, §2.3 — `write::create_imported`, `WriteError::IdHeld`, the held set, dated keys | Task 1 (the write), Tasks 2–3 (each producer's line) |
| D4, §2.4 — paths are not identity | Task 5 (step 6's `-N` name; test (ii)) |
| D5 (a), §2.5 — reconcile by id; a foreign `create` for a held id writes no file | Task 5 |
| D5 (b) — moves and deletes by id (E5's move half) | Task 6 |
| D5 (c) — one id, one file, every id gets one; restore's highest `rev` | Task 5 (`apply`), Task 8 (`materialise`) |
| D5 (d) — a tombstone names a note, not a place (E5's tombstone half) | Task 6 |
| D6, §2.6 — the alias pre-pass, `state/id-aliases.json`, `records_for`/`human_set`/`resolve_target` follow the alias | Task 4 (the map and its readers), Task 7 (the pre-pass in `apply`) |
| D7, §2.7 — delete against edit (M5) | Task 6 (test (iii)) |
| D8, §2.8 — proposed means named in any digest; `rank --no-digest` | Task 9 (with hand-offs H2, H3) |
| D8, §2.8 — the event-verdict pull and its cursor | Task 10 |
| §5.4 row 30, E5 — **In** | Task 6 |
| §5.4 row 40, M5 — **In** | Task 6 |
| §5.4 row 11, N17 — **Out** | Nothing (the spec's ruling). |
| §5.4 row 41, M6 — **In, narrowly** | Plan 3 (it is D16's *Sync now* guard). |
| §5.3 — the first-day cap per account, unchanged | Nothing to build; Task 10's pull is what keeps a moved turn from re-judging. |
| §5.6 — the commitment model's `cmt` notes | Not on this branch (no `cmt` kind here): the merge note in *Hand-off to the controller*, and spec defect S-1. |
| §6.1 — `ids.rs` tests | Task 1 |
| §6.1 (i) the I2 scenario | Task 2 |
| §6.1 (ii) two paths | Task 5 |
| §6.1 (iii) delete against edit; (iv) E5 and I3 | Task 6 |
| §6.1 (v) a pre-existing pair, one file and both files each side | Task 7 |
| §6.1 (vi) a restore given two rows for one id | Task 8 |
| §6.1 — the verdict pull | Task 10 |
| §6.1 — `eventemit` and `rank --no-digest`; `oracle.rs`, `surface_oracle.rs` unchanged | Task 9; every task (Global Constraint 3) |
| §6.3 — `judge-event`'s new GET and its `handler_test.ts` | Task 10 |
| §6.3 — `migrations_test.ts` | **No change in this plan**: no migration and no SQL function (R-TD1-14). The function pin stays **28** and the view pin **5**; Plans 2 and 3 move the function pin to 30. |
| §1 D9–D16, §3, §4.1–§4.7, §6.2, §6.3's `/turn` and RLS-guard widening, §6.4 | **Plan 3.** |
| §1 D17–D19, §4.8, §6.1's shared-config bullets, §6.3's `sync_rows.ts` bullets | **Plan 2.** |
| §7, §8, §9 | Nothing to build. |

## Fidelity ledger

Every sentence of §2 that asks for behaviour, and where it is carried. "Refined" means the spec is
silent there and a Plan ruling decides it; nothing here departs from the spec.

| Spec requirement | Task | Faithful? |
|---|---|---|
| `import_id(kind, vendor, key)` = `kind_` + first 10 hex of SHA-1 over `knowlu/import-id/1\n<kind>\n<vendor>\n<key>` | 1 | yes; the four reference values are pinned (re-computed with Python's `hashlib` while writing this plan) |
| `kind` is what `ids::kind_for` answers; `vendor` is the note's `created_by:`; `key` is the producer's word, a colon, and `source_uid:` or the producer's own key | 1 | yes |
| Only kinds `task`, `appr`; only `created_by` in `zybooks, vhl, blackboard, gmail, events, coursework, rules` | 1 | yes |
| The nine producers and their keys (§2.2 table) | 1 (`import_key`), 2, 3 | yes: each producer calls `create_imported`, and `import_key` reads the key off the note it writes, so `approvals::materialize` keys a gmail payload exactly as `enrich::write_gmail_note` does |
| Map card and digest keys carry their date | 1, 2, 3 | yes: `first_proposed_at` and `proposed_at`; a second digest the same day is `IdHeld` (Task 3's test) |
| Not covered: `create_task`, issues, info, amend cards, C5's login notice, wizard seeds, commitments | 1 | yes: `import_key` is `None` for each (`created_by` outside the list, or a kind outside `task`/`appr`) |
| `create_imported(vault, rel, text, ctx, journal, held)` puts the id where `create` puts a minted one, journals the same record, refuses a held id as `IdHeld(<id>)`, logged `skipped (already held as <id>): <stem>` | 1, 2, 3 | yes; what each producer does after the line is **refined** by R-TD1-1 |
| `held` is `ids::build_index`'s ids, built once per producer run and extended as it creates | 1 (`ids::held_ids`), 2, 3 | yes |
| D4: paths keep today's naming; sync reconciles by id | 5 | yes |
| D5 (a): step 4 finds the note by the foreign records' id or its alias; never a different id at the record's path; the path only for a note with no `id:`; `write_literals` and `live_sync_cards` use the local path; a foreign `create` for a held id writes no file | 5, 7 | yes; a note with no `id:` line is identified by its path in every step (R-TD1-2) |
| D5 (b): a foreign `move` acts on the note holding the id, `Exists` rule kept; a foreign `delete` settles the note holding its id under `sync::ACTOR` unless it is in `archive/` | 6 | yes; a record with no id keeps today's path rule (R-TD1-15) |
| D5 (c): skip a text whose id is held at another path (the spec's line); skip one whose `import_key` names an item held under another id; write a new id at its path, or at the next free `-N` name; restore: one file per id, highest `rev` wins | 5, 7, 8 | yes; the lines the spec does not word are R-TD1-3's |
| D5 (d): a tombstone settles unless other desktops' records place a different id at `P` and none ever placed this note's id, ids compared as alias groups; the spec's line | 6 | yes; "place" and "other desktops" are defined by R-TD1-13 |
| D6: groups from every `create` record (this page's included) and from a note only when its id has no `create`; lowest id wins; `state/id-aliases.json` generated, device-local, rebuilt by every apply | 4, 7 | yes; the file is rewritten only when the map changes (R-TD1-11) |
| D6: step 4 groups foreign records under the winner; `records_for` returns the whole group; `human_set` reads through it; `resolve_target` answers an old id | 4, 7 | yes; records reach `reconcile::resolve` under the winner's id (R-TD1-10), and `detect_external` reads a group as one note (R-TD1-18) |
| D6: a group found for the first time is reconciled in full against every other desktop's records for all its ids; the pre-pass runs on every sync | 7 | yes |
| D6: both files here → the loser's fields reconciled into the winner, the loser settled through `write::delete`, never re-id'd; only the loser here → re-id'd by one `write_literals` of `id` under `sync::ACTOR` | 7 | yes; how the loser's fields are reconciled is **refined** by R-TD1-9 (spec defect S-2) |
| D7: delete beats a concurrent edit; archived on both, the edit on the archived copy, no card | 6 | yes (test (iii), both push orders) |
| D8: a uid named in any digest note, `approvals/` or `archive/`, any status, is proposed | 9 | yes, through a new `eventemit::proposed_digest_uids` (R-TD1-7) |
| D8: `rank --no-digest` (new; default unchanged) | 9 | yes (hand-offs H2, H3; R-TD1-8) |
| D8: every desktop's `sync`, after its pull, reads `GET /judge-event?after=<judged_at>,<id>` (`answered` event rows; `item_id`, `verdict`, `judged_at`, `id`; 500 a page; ordered `(judged_at, id)`); a uid with no ledger line gets one through `record_verdict` with empty `why` and `strength` and the title from its roster, else `(untitled)`; never replaces a line; the cursor is a new `Cursor` field | 10 | yes; placement, lines and the title source are **refined** by R-TD1-4, R-TD1-5, R-TD1-6 |
| §5.4 E5, M5 **In** | 6 | yes |

## Order and file ownership

| Task | Unit | Files it edits | Hand-off applied with it | Depends on |
|---|---|---|---|---|
| 1 | The import id and `create_imported` | `engine/src/ids.rs`, `engine/src/write.rs` | — | — |
| 2 | Coursework and the LMS feed mint import ids; the I2 scenario | `engine/src/coursework.rs`, `engine/tests/import_ids.rs` (new), `engine/tests/sync_contract.rs` | **H1** (`engine/src/ingest.rs`) | 1 |
| 3 | Gmail, rule cards, approvals and the digest mint import ids | `engine/src/enrich.rs`, `engine/src/approvals.rs`, `engine/src/eventemit.rs` | — | 1 |
| 4 | The alias map and its readers | `engine/src/ids.rs`, `engine/src/journal.rs` | — | 1 |
| 5 | `apply` by id: reconcile and pulled texts | `engine/src/sync.rs`, `engine/src/write.rs` (`free_slot` visibility), `engine/tests/sync_contract.rs` | — | 2, 4 |
| 6 | `apply` by id: moves, deletes and tombstones | `engine/src/sync.rs`, `engine/tests/sync_contract.rs` | — | 5 |
| 7 | The alias pre-pass | `engine/src/sync.rs`, `engine/src/passes.rs`, `engine/tests/sync_contract.rs` | — | 6 |
| 8 | Restore keeps one file per id | `engine/src/sync.rs`, `engine/tests/sync_contract.rs` | — | 7 |
| 9 | Event proposals across desktops: any digest proposes; `rank --no-digest` | `engine/src/eventemit.rs`, `engine/tests/rank_no_digest.rs` (new) | **H2** (`engine/src/cli.rs`), **H3** (`engine/src/main.rs`) | 3 |
| 10 | The event-verdict pull | `cloud/supabase/functions/judge-event/{index.ts,handler.ts (new),handler_test.ts (new)}`, `engine/src/sync.rs`, `engine/tests/sync_contract.rs` | **H4** (`CLAUDE.md`) | 8, 9 |

Strictly sequential: `sync.rs` is edited by Tasks 5–8 and 10, `sync_contract.rs` by Tasks 2, 5–8 and
10, `ids.rs` by Tasks 1 and 4, `eventemit.rs` by Tasks 3 and 9. No two tasks run in parallel. Each ends
with a green workspace and one commit (two where a hand-off lands beside it: the controller's hand-off
commit first, then the task's).

## Plan rulings

Each is repeated at the step it changes. None departs from the spec; each decides something the spec
leaves open.

- **R-TD1-1 — what `IdHeld` means to a producer.** `WriteError::IdHeld(id)` means this item's note is
  already in the vault under its import id. Every producer skips the write and logs
  `skipped (already held as <id>): <stem>` (`<stem>` is the file stem the producer would have written;
  for a calendar-event note, whose name `calendar_note` chooses, the event's uid). Then: `coursework` and
  `ingest` record the uid in the seen ledger, exactly as `ingest` backfills a note it found by
  `source_uid` (`ingest.rs:664-669`); Gmail acknowledges the item and records it seen, because the queue
  would otherwise deliver it again every slot; an approved task card whose task is held is stamped
  `executed` and archived like one that materialised, with the line as a warning; an approved digest
  counts the held calendar note as created; a map card, a rule card or a digest is simply not written
  (the digest silently, as today's same-day path check is).
- **R-TD1-2 — a note with no `id:` line is identified by its path**, D5 (a)'s own rule, in every step:
  step 4 reconciles foreign records against it when no note holds their id; step 6 never writes a
  pulled text over it or beside it (today's rule); a foreign `move` carrying an id moves it when no
  note holds that id; a tombstone settles it as today. This keeps every existing test on
  `engine/tests/fixtures/vault-s1`, whose notes carry no ids.
- **R-TD1-3 — lines the spec does not word.** Every new line, exactly:
  - step 6, a new id beside a different note: `sync: <path> holds a different note here; <id> was written to <slot>`;
  - step 6, the alias joins it: `sync: <path> is <id>, the same item as <held id> held here as <local path>; the two are joined`;
  - a move whose old path holds another note: `sync: <from> is <other id>, not <id>; the move to <dest> is not applied here`;
  - the pre-pass, both files here: `sync: <loser path> is <loser>, the same item as <winner> at <winner path>; merged into it and archived`;
  - the pre-pass, only the loser here: `sync: <path> re-identified from <loser> to <winner>, the same item your other computer holds`;
  - the alias file unwritable: `sync: the alias record could not be saved (<cause>); the next sync rebuilds it`;
  - a restore given one id at two paths: `restore: <earlier path> and <path> are both <id>; the later row is kept`;
  - the verdict pull: `event verdicts: <n> added from the account`, `event verdicts: more to come — the next sync continues`, `event verdicts: skipped (<cause>)`, `event verdicts: <uid> not recorded (<cause>)`, `event verdicts: the cursor could not be saved (<cause>)`.
  The spec's own two lines are used verbatim: `sync: <path> is <id>, already held here as <local path>`
  and `sync: <P> — the account settled a different note there; this one stays`.
- **R-TD1-4 — where the verdict pull runs.** In `sync::run_lines_with`, after
  `run_lines_with_client` returns, when the direction pulls and the pull was not offline — not inside
  `run_lines_with_client`, whose loopback tests script an exact sequence of two requests (pull, push)
  and would each meet a third. It is tested directly through the new `sync::pull_event_verdicts`.
  `sync::Totals` gains `pub offline: bool` so the wrapper can tell. A failed verdict pull is a named line,
  never an entry in `Totals::errors`: the sync itself succeeded, and the verdicts come at the next pull.
- **R-TD1-5 — one page per sync.** One `GET` per sync run, as §2.8's cost line says; a full page (the
  reply's `more`) prints the "more to come" line and the next sync continues from the cursor.
- **R-TD1-6 — the title of a pulled verdict** comes from the roster `rank` last wrote
  (`state/events.md`, read with `eventroster::read_roster`), else it is empty and `record_verdict`
  writes `(untitled)`. `sync` never fetches a feed.
- **R-TD1-7 — D8's "proposed" is a new function.** `eventemit::proposed_digest_uids` (every
  `events-digest-*.md` in `approvals/` and `archive/`, any status) is what `emit_digest` reads.
  `pending_digest_uids` and its tests are kept, because three unmerged branches carry both; one of its
  tests is renamed, because its old name would claim an eligibility D8 removes.
- **R-TD1-8 — `--no-digest` reaches `cli.rs` as `RankOptions { no_digest }`** through new
  `cli::run_opts` and `cli::run_with_opts`. `cli::run` and `cli::run_with` keep their signatures and pass
  `RankOptions::default()`, so every existing caller, `oracle.rs` and the frozen references are
  untouched. The roster and *Coming up* are still built; only `emit_digest` is skipped, silently.
- **R-TD1-9 — how the pre-pass merges a loser file into the winner file** (spec defect S-2). The
  winner's records (every device) are the upstream history of the winner file, and the loser's records
  (every device) are the other side, both passed to `reconcile::resolve` under the winner's id; the
  result is applied and carded exactly as step 4 applies and cards. A field set only on the losing copy
  (on any computer) therefore reaches the winner file, and one set on both becomes one card. The pre-pass
  runs this only for a loser held **outside** `archive/`: an archived loser is already settled.
- **R-TD1-10 — records reach `reconcile::resolve` under the group's winner.** `journal::latest_by_field`
  keys on `(id, field)` and `resolve` collapses that map by field in key order, so records under two ids
  would pick "latest" by id, not by time. The rewrite is in memory only; the journal is never changed.
- **R-TD1-11 — `state/id-aliases.json` is rewritten only when the rebuilt map differs from the file**
  (or the file is missing and the map is not empty), through a temp file and a rename like
  `save_seed_hashes`. A vault with no import doubles never gets the file.
- **R-TD1-12 — a restore's undone row never enters the cursor.** When a later row for an id replaces an
  earlier path, the earlier path leaves `RestoreState::written_notes`, so the first push after the
  restore sends no tombstone for it — a tombstone would archive the other desktop's live copy.
- **R-TD1-13 — "places" and "other desktops" for D5 (d).** A journal record places id `I` at path `P`
  when its `id` is `I` and its `path` is `P`, or it is a `move` whose `new` is `P`. A record with no id
  places nothing. "Other desktops" are records whose `device` is not `journal::device_name()` (§3.2's
  caveat applies).
- **R-TD1-14 — the verdict read is two PostgREST selects** through the existing `Db`, not a new SQL
  function: rows at the cursor's own `judged_at` with a greater `id`, then rows with a later `judged_at`.
  It needs no migration (the `judgments_account_day` index serves it), so `migrations_test.ts` does not
  move. `judgments` keeps RLS on with no policy (C2's rule); the service role reads it, and the select
  names the account, which `_shared/judge_db_test.ts`'s scan enforces.
- **R-TD1-15 — a record with no id keeps today's path rule**: a `move` with no id moves the note at its
  old path; a `delete` with no id has no effect of its own (its tombstone carries it).
- **R-TD1-16 — `enrich`'s equivalence test is rewritten over two vaults.** `an_approved_gmail_card_
  materialises_the_same_note_a_task_tier_would` wrote a message's direct note and its card's task into
  one vault; under D3 the second is `IdHeld`. It now writes each into its own vault and compares them
  byte for byte, id line included — the stronger claim D2 makes.
- **R-TD1-17 — `write::free_slot` becomes `pub(crate)`** so step 6 names the next free `-N` path by the
  same rule `write::delete` uses.
- **R-TD1-18 — `passes::detect_external` reads an alias group as one note.** D6 says records follow the
  alias wherever they are read by id; `detect_external`'s field index (`load_index`) is one such reader,
  and without this a re-identified note's first `rank` would journal a fabricated `quinn`/`external`
  edit for every field its old-id history explains — a record that travels, settles the other desktop's
  card unanswered and locks the field. While an alias group exists the index is rebuilt from the whole
  journal under each group's winner (the id-keyed cache cannot know the groups), and a loser id's note
  — an archived copy the pre-pass settled — is not compared. A vault with no import doubles is
  unchanged.

## Interfaces this plan produces

Plans 2 and 3 consume these exact names. A task that changes one of them changes this table too.

| Name | Signature or shape | Task |
|---|---|---|
| `ids::import_id` | `pub fn import_id(kind: &str, vendor: &str, key: &str) -> String` | 1 |
| `ids::IMPORT_VENDORS` | `pub const IMPORT_VENDORS: [&str; 7] = ["zybooks", "vhl", "blackboard", "gmail", "events", "coursework", "rules"]` | 1 |
| `ids::ImportKey`, `ids::import_key` | `pub type ImportKey = (String, String, String);` (kind, vendor, key) — `pub fn import_key(path: &Path, meta: &Mapping) -> Option<ImportKey>` | 1 |
| `ids::held_ids` | `pub fn held_ids(vault: &Path) -> BTreeSet<String>` | 1 |
| `write::WriteError::IdHeld` | `IdHeld(String)`; `Display` is `already held as <id>` | 1 |
| `write::create_imported` | `pub fn create_imported(vault: &Path, rel_path: &str, text: &str, ctx: &WriteContext, journal: &mut Journal, held: &mut BTreeSet<String>) -> Result<PathBuf, WriteError>` | 1 |
| `write::free_slot` | `pub(crate) fn free_slot(folder: &Path, name: &str) -> PathBuf` (visibility only) | 5 |
| `ids::ALIASES_FILE` | `pub const ALIASES_FILE: &str = "state/id-aliases.json";` — `{loser: winner}`, generated, device-local, never synced | 4 |
| `ids::load_aliases`, `ids::save_aliases` | `pub fn load_aliases(vault: &Path) -> BTreeMap<String, String>`; `pub fn save_aliases(vault: &Path, aliases: &BTreeMap<String, String>) -> Result<(), String>` | 4 |
| `ids::canonical`, `ids::alias_group` | `pub fn canonical<'a>(aliases: &'a BTreeMap<String, String>, id: &'a str) -> &'a str`; `pub fn alias_group(aliases: &BTreeMap<String, String>, id: &str) -> BTreeSet<String>` | 4 |
| `ids::import_ids_by_key`, `ids::aliases_from` | `pub fn import_ids_by_key(records: &[Record], notes: &[(PathBuf, Option<Mapping>)]) -> BTreeMap<ImportKey, BTreeSet<String>>`; `pub fn aliases_from(keys: &BTreeMap<ImportKey, BTreeSet<String>>) -> BTreeMap<String, String>` | 4 |
| `Journal::set_aliases` | `pub fn set_aliases(&mut self, aliases: BTreeMap<String, String>)`; `records_for` and `human_set` now answer the whole alias group | 4 |
| `sync::apply` | signature unchanged; step order after this plan: record pass → id index → alias map (Task 7) → moves and deletes by id → seed pre-pass → alias file settle → step 4 by group → step 6 by id | 5–7 |
| `sync::Cursor::verdicts_after` | `#[serde(default)] pub verdicts_after: String` — `<judged_at>,<id>` of the last verdict row taken, empty for none. **Plan 2 adds `provisional_config` and `provisional_since` beside it.** | 10 |
| `sync::Totals::offline` | `pub offline: bool` — the pull met a transport failure | 10 |
| `sync::pull_event_verdicts` | `pub fn pull_event_verdicts(vault: &Path, client: &crate::cloudmodel::CloudClient) -> Vec<String>` | 10 |
| `GET /judge-event` | `?after=<judged_at>,<id>` (absent: from the start) → `{"verdicts": [{"item_id", "verdict", "judged_at", "id"}], "more": bool}`, 500 a page, ordered `(judged_at, id)`; 400 `{"error": "after must be <judged_at>,<id>"}`; 401/402 from the entitlement; `POST` unchanged | 10 |
| `cli::RankOptions`, `cli::run_opts`, `cli::run_with_opts` | `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)] pub struct RankOptions { pub no_digest: bool }`; `run_opts(vault, today_iso, runner, run_id, opts)`, `run_with_opts(vault, today_iso, runner, run_id, fetchers, opts)` | 9 (H2) |
| `knowlu-engine rank --no-digest` | flag; default off. **Plan 3's `TurnPlan` passes it** on a computer that did not hold `feeds`. | 9 (H3) |
| `eventemit::proposed_digest_uids` | `pub fn proposed_digest_uids(vault: &Path) -> BTreeSet<String>` | 9 |

What Plans 2 and 3 can rely on, beyond the names: D13's fail-open is safe because two desktops running
every machine step converge (Tasks 2–8); a shared `config/` file has no `id:`, and step 6's id rules
apply only to `ids::NOTE_FOLDERS` paths, so Plan 2's three-way settings check branches before them; the
verdict pull runs in every `sync` that pulls, so Plan 3's catch-up (`sync → rank --no-digest`) gets the
holder's verdicts with no step of its own.

---

### Task 1: The import id and `create_imported` (spec §2.1, §2.3; D1–D3)

**Files:**
- Modify: `engine/src/ids.rs` — the `use` block (`:7-17`), after `new_id` (`:67-73`), after `kind_for`
  (`:79-94`), and `mod tests` (`:276-442`).
- Modify: `engine/src/write.rs` — `WriteError` and its `Display` (`:32-55`), `create` (`:377-429`), and
  `mod tests` (the create tests at `:1090-1121`).

**Interfaces:**
- Consumes: `ids::kind_for(&Path, Option<&Mapping>) -> String` (`ids.rs:79`), `ids::build_index`
  (`:124`), `yaml::get`/`yaml::text`, `write::create` (`write.rs:379`).
- Produces: `ids::import_id`, `ids::IMPORT_VENDORS`, `ids::ImportKey`, `ids::import_key`,
  `ids::held_ids`, `WriteError::IdHeld`, `write::create_imported` — exactly as the table above.

- [ ] **Step 1: Write the failing `ids.rs` tests**

In `engine/src/ids.rs`, inside `mod tests`, after `derived_id_reproduces_pythons_sha1_exactly` (`:303-309`):

```rust
    /// Two-desktop design §2.1: the four reference values, computed outside Rust (Python's `hashlib`
    /// and `sha1sum`). Frozen once shipped (D1): a different digest here would re-open doubles between
    /// two computers on different builds.
    #[test]
    fn import_id_reproduces_the_spec_reference_values() {
        assert_eq!(import_id("task", "zybooks", "coursework:zybooks:1839992"), "task_18734fe8b7");
        assert_eq!(
            import_id("task", "blackboard", "lms:_blackboard.platform.gradebook2.GradableItem-_4732722_1"),
            "task_d8891a504b"
        );
        assert_eq!(import_id("task", "gmail", "gmail:gmail:m1"), "task_3bc4bec4a9");
        assert_eq!(import_id("appr", "events", "events-digest:2026-09-25"), "appr_40ab7c3a10");
        assert!(is_id(&import_id("task", "vhl", "coursework:vhl:1")), "the shape of every id (ID_RE)");
    }

    fn key_of(rel: &str, frontmatter: &str) -> Option<ImportKey> {
        import_key(Path::new(rel), &crate::yaml::mapping_of(frontmatter))
    }

    fn want(kind: &str, vendor: &str, key: &str) -> Option<ImportKey> {
        Some((kind.to_string(), vendor.to_string(), key.to_string()))
    }

    /// §2.2: one row per producer, each key prefixed by its producer's word, so no two producers can
    /// share a key (review M14).
    #[test]
    fn import_key_names_each_producer_by_its_own_word() {
        assert_eq!(
            key_of("tasks/cs-100-hw-01.md", "created_by: zybooks\nsource_uid: \"zybooks:1839992\""),
            want("task", "zybooks", "coursework:zybooks:1839992")
        );
        assert_eq!(
            key_of("archive/gn-103-hw.md", "created_by: vhl\nsource_uid: \"vhl:1:2026-08-28\"\nstatus: archived"),
            want("task", "vhl", "coursework:vhl:1:2026-08-28")
        );
        assert_eq!(
            key_of("tasks/cs-100-quiz.md", "created_by: blackboard\nsource_uid: \"_blackboard.platform.gradebook2.GradableItem-_4732722_1\""),
            want("task", "blackboard", "lms:_blackboard.platform.gradebook2.GradableItem-_4732722_1")
        );
        assert_eq!(
            key_of("tasks/ps-4.md", "created_by: gmail\nsource_uid: \"gmail:m1\""),
            want("task", "gmail", "gmail:gmail:m1")
        );
        assert_eq!(
            key_of("approvals/task-ps-4.md", "type: approval\nkind: task\ncreated_by: gmail\nsource_uid: \"gmail:m1\""),
            want("appr", "gmail", "gmail:gmail:m1"),
            "a Gmail card and its task are two notes: the kind keeps them apart"
        );
        assert_eq!(
            key_of("approvals/calendar-event-fair.md", "type: approval\nkind: calendar-event\ncreated_by: events\nsource_uid: \"engage:1\""),
            want("appr", "events", "calendar-event:engage:1")
        );
        assert_eq!(
            key_of("approvals/events-digest-2026-09-25.md", "type: approval\nkind: events-digest\nproposed_at: 2026-09-25\ncreated_by: events"),
            want("appr", "events", "events-digest:2026-09-25")
        );
        assert_eq!(
            key_of(
                "approvals/map-zybooks-uahcs100fall2026.md",
                "type: approval\nkind: coursework-map\nfirst_proposed_at: 2026-09-10\ncreated_by: coursework\nsource: \"zybooks\"\nmap_key: \"UAHCS100Fall2026\""
            ),
            want("appr", "coursework", "map:zybooks:UAHCS100Fall2026:2026-09-10")
        );
        assert_eq!(
            key_of("approvals/rule-42.md", "type: approval\nkind: rule\ncreated_by: rules\nrule_id: 42"),
            want("appr", "rules", "rule:42")
        );
    }

    /// §2.2: never for a person, the console, an agent or any other word; never for a kind other
    /// than `task` and `appr`; never for a row whose own key field is missing.
    #[test]
    fn import_key_is_none_for_everything_a_person_or_a_judgment_makes() {
        for who in ["quinn", "dashboard", "agent:knowlu.enrich", "agent:coursework.zybooks", "claude", "commitments"] {
            assert_eq!(key_of("tasks/x.md", &format!("created_by: {who}\nsource_uid: \"zybooks:1\"")), None, "{who}");
        }
        assert_eq!(key_of("tasks/x.md", "source_uid: \"zybooks:1\""), None, "no created_by");
        assert_eq!(key_of("courses/cs-100.md", "created_by: zybooks\nsource_uid: \"zybooks:1\""), None, "kind course");
        assert_eq!(key_of("issues/x.md", "type: issue\ncreated_by: gmail\nsource_uid: \"gmail:m1\""), None, "kind iss");
        assert_eq!(key_of("info/x.md", "type: info\ncreated_by: events\nsource_uid: \"engage:1\""), None, "kind info");
        assert_eq!(key_of("tasks/x.md", "created_by: zybooks"), None, "a coursework task with no source_uid");
        assert_eq!(
            key_of("approvals/amend-x.md", "type: approval\nkind: amend\ncreated_by: events\nsource_uid: \"engage:1\""),
            None,
            "an events card that is no producer's"
        );
        assert_eq!(key_of("approvals/events-digest-x.md", "type: approval\nkind: events-digest\ncreated_by: events"), None, "a digest with no proposed_at");
        assert_eq!(key_of("tasks/x.md", "created_by: events\nsource_uid: \"engage:1\""), None, "a task is never an events note");
    }

    /// D3's `held`: every valid id in the six note folders, `build_index`'s keys.
    #[test]
    fn held_ids_is_every_valid_id_in_the_note_folders() {
        let v = vault();
        note(&v, "tasks/a.md", "---\nid: task_0123456789\n---\n\nb\n");
        note(&v, "archive/b.md", "---\nid: task_abcdef0123\n---\n\nb\n");
        note(&v, "tasks/c.md", "---\nid: not-an-id\n---\n\nb\n");
        assert_eq!(
            held_ids(&v).into_iter().collect::<Vec<_>>(),
            vec!["task_0123456789".to_string(), "task_abcdef0123".to_string()]
        );
    }
```

- [ ] **Step 2: Write the failing `write.rs` tests**

In `engine/src/write.rs`, inside `mod tests`, after `create_refuses_an_existing_path_and_a_note_without_frontmatter` (`:1107-1121`):

```rust
    // -- create_imported (two-desktop design D3) ------------------------------

    const IMPORTED: &str = "---\ntitle: \"CS 100 HW 01\"\nstatus: active\ncreated_by: zybooks\nsource_uid: \"zybooks:1839992\"\n---\n\nbody\n";

    /// D3: the import id goes exactly where `create` puts a minted one, and the journal record has the
    /// same shape — only the id's value differs.
    #[test]
    fn create_imported_puts_the_import_id_where_create_puts_a_minted_one() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::new();
        let imported = create_imported(&v, "tasks/imported.md", IMPORTED, &ctx, &mut j, &mut held).unwrap();
        let minted = create(&v, "tasks/minted.md", IMPORTED, &ctx, &mut j, None).unwrap();

        assert_eq!(get_str(&read_meta(&imported).unwrap(), "id").as_deref(), Some("task_18734fe8b7"));
        let lines = |p: &Path| pystr::read_text(p).unwrap().lines().map(str::to_string).collect::<Vec<_>>();
        let without_id = |p: &Path| lines(p).into_iter().filter(|l| !l.starts_with("id: ")).collect::<Vec<_>>();
        assert_eq!(without_id(&imported), without_id(&minted), "the same text but for the id's value");
        let at = |p: &Path| lines(p).iter().position(|l| l.starts_with("id: "));
        assert_eq!(at(&imported), at(&minted), "the id line sits where create puts it");

        let records = j.read(None, None);
        assert_eq!(records.len(), 2);
        let keys = |r: &crate::ledger::Record| r.keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys(&records[0]), keys(&records[1]), "the same record shape");
        assert_eq!(records[0].get("op").and_then(|x| x.as_str()), Some("create"));
        assert_eq!(records[0].get("id").and_then(|x| x.as_str()), Some("task_18734fe8b7"));
        assert_eq!(records[0]["new"]["id"], serde_json::json!("task_18734fe8b7"), "the record carries the note as minted");
        assert!(held.contains("task_18734fe8b7"), "held grows as the run creates");
    }

    /// D3: an id the vault already holds is refused before anything is journalled or written.
    #[test]
    fn create_imported_refuses_an_id_the_vault_already_holds() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::from(["task_18734fe8b7".to_string()]);
        let err = create_imported(&v, "tasks/again.md", IMPORTED, &ctx, &mut j, &mut held).unwrap_err();
        assert_eq!(err, WriteError::IdHeld("task_18734fe8b7".to_string()));
        assert_eq!(err.to_string(), "already held as task_18734fe8b7");
        assert!(!v.join("tasks").join("again.md").exists(), "no file");
        assert!(j.read(None, None).is_empty(), "no record");
    }

    /// D3: with no import key it is `create` — a random id — and a taken path is still `Exists`,
    /// checked first, exactly as `create` checks it.
    #[test]
    fn create_imported_without_a_key_mints_like_create_and_checks_the_path_first() {
        let v = vault();
        seed(&v);
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("quinn", "dashboard");
        let mut held = std::collections::BTreeSet::new();
        let mine = "---\ntitle: \"Mine\"\ncreated_by: quinn\n---\n\nb\n";
        let a = create_imported(&v, "tasks/a1.md", mine, &ctx, &mut j, &mut held).unwrap();
        let b = create_imported(&v, "tasks/a2.md", mine, &ctx, &mut j, &mut held).unwrap();
        let id = |p: &Path| get_str(&read_meta(p).unwrap(), "id").unwrap();
        assert!(is_id(&id(&a)) && is_id(&id(&b)));
        assert_ne!(id(&a), id(&b), "random, as new_id mints");
        assert!(matches!(
            create_imported(&v, "tasks/a.md", IMPORTED, &ctx, &mut j, &mut held),
            Err(WriteError::Exists(_))
        ));
    }

    /// An id the text already carries is kept, as `create` keeps it — and still refused when held.
    #[test]
    fn create_imported_keeps_an_id_the_text_carries_and_still_checks_it() {
        let v = vault();
        let mut j = Journal::new(&v);
        let ctx = WriteContext::new("agent:coursework.zybooks", "local-runner");
        let mut held = std::collections::BTreeSet::new();
        let text = IMPORTED.replace("status: active\n", "status: active\nid: task_0000000abc\n");
        let p = create_imported(&v, "tasks/carried.md", &text, &ctx, &mut j, &mut held).unwrap();
        assert_eq!(get_str(&read_meta(&p).unwrap(), "id").as_deref(), Some("task_0000000abc"));
        assert!(matches!(
            create_imported(&v, "tasks/carried-2.md", &text, &ctx, &mut j, &mut held),
            Err(WriteError::IdHeld(_))
        ));
    }
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- import_ held_ids create_imported`
Expected: FAIL to compile — `cannot find function import_id`, `import_key`, `held_ids`,
`create_imported`, type `ImportKey`, and no variant `IdHeld` on `WriteError`.

- [ ] **Step 4: Implement the three `ids.rs` items**

In `engine/src/ids.rs`, change the collections import at `:7` to
`use std::collections::{BTreeMap, BTreeSet};`, and add after `new_id` (`:67-73`):

```rust
/// Two-desktop design D1: the id of an imported note, derived from the item so that two computers
/// importing one item mint one id. The same shape as every id (`kind_` + 10 lowercase hex, `ID_RE`)
/// and `derived_id`'s SHA-1 and cut — an identity, not a security primitive. The first line of the
/// hashed text separates this hash from `derived_id`'s and versions it. **Frozen once shipped**: a
/// second derivation would re-open doubles between computers on different builds.
pub fn import_id(kind: &str, vendor: &str, key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("knowlu/import-id/1\n{kind}\n{vendor}\n{key}").as_bytes());
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    format!("{kind}_{}", &hex[..10])
}
```

And after `kind_for` (`:79-94`):

```rust
/// D2: the `created_by:` words whose notes an automatic step creates because an outside item exists —
/// the notes two desktops create independently. A closed list: a person, the console, an agent's
/// judgment and every other word keep `new_id`.
pub const IMPORT_VENDORS: [&str; 7] = ["zybooks", "vhl", "blackboard", "gmail", "events", "coursework", "rules"];

/// `(kind, vendor, key)` — the three inputs of [`import_id`].
pub type ImportKey = (String, String, String);

/// D2: whether a note is imported, and under which key — the one function that answers it (§2.2).
///
/// Only kinds `task` and `appr` ([`kind_for`], so the prefix is the one `create` would have minted),
/// only a `created_by:` in [`IMPORT_VENDORS`], and the key is the producer's word, a colon, and the
/// note's `source_uid:` or, where a producer has none, its own key. Every input is frontmatter the
/// producer writes from the item itself; none is per-desktop. The map card's and the digest's keys
/// carry their date, because each is re-proposed into a free path and must not take the old card's id.
pub fn import_key(path: &Path, meta: &Mapping) -> Option<ImportKey> {
    let kind = kind_for(path, Some(meta));
    if kind != "task" && kind != "appr" {
        return None;
    }
    let field = |name: &str| crate::yaml::get(meta, name).and_then(crate::yaml::text).filter(|v| !v.is_empty());
    let vendor = field("created_by")?;
    if !IMPORT_VENDORS.contains(&vendor.as_str()) {
        return None;
    }
    let card = field("kind");
    let (word, rest) = match (vendor.as_str(), kind.as_str(), card.as_deref()) {
        ("zybooks" | "vhl", "task", _) => ("coursework", field("source_uid")?),
        ("blackboard", "task", _) => ("lms", field("source_uid")?),
        ("gmail", "task" | "appr", _) => ("gmail", field("source_uid")?),
        ("events", "appr", Some("calendar-event")) => ("calendar-event", field("source_uid")?),
        ("events", "appr", Some("events-digest")) => ("events-digest", field("proposed_at")?),
        ("coursework", "appr", Some("coursework-map")) => {
            ("map", format!("{}:{}:{}", field("source")?, field("map_key")?, field("first_proposed_at")?))
        }
        ("rules", "appr", Some("rule")) => ("rule", field("rule_id")?),
        _ => return None,
    };
    Some((kind, vendor, format!("{word}:{rest}")))
}

/// D3's `held`: every id this vault already holds — [`build_index`]'s keys, read once per producer
/// run and extended by `write::create_imported` as the run creates.
pub fn held_ids(vault: &Path) -> BTreeSet<String> {
    build_index(vault).into_keys().collect()
}
```

- [ ] **Step 5: Implement `WriteError::IdHeld` and `create_imported`**

In `engine/src/write.rs`, add the variant after `Exists(String),` (`:36`):

```rust
    /// Two-desktop design D3: the import id this note would take is already held in the vault — the
    /// item's note is here already. Refused before anything is journalled or written.
    IdHeld(String),
```

and its `Display` arm after the `Exists` arm (`:48`):

```rust
            WriteError::IdHeld(id) => write!(f, "already held as {id}"),
```

Replace `create` (`:377-429`) with a private `create_minting` that both public functions call. The body
is today's `create` with two changes, marked `D1–D3`:

```rust
/// Mint a new note. Stamps an `id` if the text has none, and refuses to create one the write path
/// could never edit again.
pub fn create(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
) -> Result<PathBuf, WriteError> {
    create_minting(vault, rel_path, text, ctx, journal, evidence, None)
}

/// Two-desktop design D3: [`create`] for a note an automatic step makes because an outside item
/// exists. With an `ids::import_key` the note takes `ids::import_id` exactly where `create` would
/// have put a minted id, and the journal record keeps its shape — only the id's value differs.
/// Without one it is `create`. An id already in `held` (the vault's ids, `ids::held_ids`, read once
/// per producer run) is refused as [`WriteError::IdHeld`]; a created id joins `held`. Every producer
/// dedups by `source_uid` first, so this is the belt.
pub fn create_imported(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    held: &mut std::collections::BTreeSet<String>,
) -> Result<PathBuf, WriteError> {
    create_minting(vault, rel_path, text, ctx, journal, None, Some(held))
}

fn create_minting(
    vault: &Path,
    rel_path: &str,
    text: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
    mut held: Option<&mut std::collections::BTreeSet<String>>,
) -> Result<PathBuf, WriteError> {
    let path = crate::ids::inside_vault(vault, &vault.join(rel_path))?;
    if path.exists() {
        return Err(WriteError::Exists(rel_path.to_string()));
    }
    let (mut meta, _) = split_frontmatter(text)
        .map_err(|_| WriteError::NoFrontmatter(rel_path.to_string()))?;
    if meta.is_empty() {
        return Err(WriteError::NoFrontmatter(rel_path.to_string()));
    }

    let mut text = text.to_string();
    let existing = get_str(&meta, "id");
    let note_id = match existing {
        Some(id) if is_id(&id) => id,
        _ => {
            // D1-D3: an imported note's id is derived from the item, so two computers mint the same
            // one; every other note's is random, as ever.
            let imported = if held.is_some() { crate::ids::import_key(&path, &meta) } else { None };
            let minted = match imported {
                Some((kind, vendor, key)) => crate::ids::import_id(&kind, &vendor, &key),
                None => new_id(&kind_for(&path, Some(&meta))),
            };
            text = apply_frontmatter_fields_to_text(
                &text,
                &[("id".to_string(), minted.clone())],
                rel_path,
            )?;
            meta.insert(Value::String("id".into()), Value::String(minted.clone()));
            minted
        }
    };
    // D3: one id, one note. Checked before the journal, so a refusal leaves no trace.
    if held.as_deref().is_some_and(|held| held.contains(&note_id)) {
        return Err(WriteError::IdHeld(note_id));
    }
    // Never mint a note the write path cannot later edit.
    guard_block_style(&text)?;

    let rel_path = rel(vault, &path);
    let mut spec = NewRecord::new("create", &rel_path, &ctx.actor, &ctx.via);
    spec.id = Some(&note_id);
    spec.new = yaml_to_json(&Value::Mapping(meta));
    spec.run_id = ctx.run_id.as_deref();
    spec.evidence = evidence.cloned().unwrap_or(serde_json::Value::Null);
    let mut record = make_record(spec).map_err(|e| WriteError::Io(e.to_string()))?;
    journal.append(&mut record).map_err(|e| WriteError::Io(e.to_string()))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| WriteError::Io(e.to_string()))?;
    }
    pystr::write_text(&path, &text).map_err(|e| WriteError::Io(e.to_string()))?;
    if let Some(held) = held.as_deref_mut() {
        held.insert(note_id);
    }
    Ok(path)
}
```

(`note_id` is borrowed by `spec` only until `make_record` returns, so the final `insert` moves it.)

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- ids:: write::`
Expected: PASS — every `ids::` and `write::` test, the seven new ones included.

- [ ] **Step 7: Run the whole suite**

Run: `cargo build --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2`, then
`cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml --workspace -j 2`
Expected: PASS at 0 warnings (the one accepted `.rsrc` line aside). No producer calls
`create_imported` yet, so nothing else moves.

- [ ] **Step 8: Commit**

Message file `.superpowers\sdd\msg-task-1.txt`:

```
feat(engine): the import id and create_imported (two desktops, D1-D3)

ids::import_id derives an imported note's id from its kind, its created_by
word and its producer's key (SHA-1 over a versioned line, cut to the shape
of every id), pinned by the spec's four reference values. ids::import_key
is the one function that says whether a note is imported, over a closed
list of seven producer words. write::create_imported puts that id exactly
where create puts a minted one and refuses an id the vault already holds
as WriteError::IdHeld. No producer calls it yet.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/ids.rs engine/src/write.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-1.txt`

---

### Task 2: Coursework and the LMS feed mint import ids; the I2 scenario (spec §2.2, §6.1 (i); D1–D3)

**Files:**
- Modify: `engine/src/coursework.rs` — `sync_coursework` (`:296-598`: the two `write::create` calls at
  `:528` and `:573`), `propose_map_cards` (`:994-1045`), `write_map_card` (`:1071-1114`), and `mod tests`
  (the `write_map_card(` calls at `:4050`, `:4089`, `:4090`, `:4110`, `:4149`, `:4258`, `:4350`, `:4407`,
  and four new tests).
- Create: `engine/tests/import_ids.rs` — the LMS producer, from outside the crate.
- Modify: `engine/tests/sync_contract.rs` — a new section at the end (helpers and test (i)).
- Hand-off: **H1** (`engine/src/ingest.rs::sync_tasks`), applied by the controller at Step 7.

**Interfaces:**
- Consumes: Task 1's `write::create_imported`, `WriteError::IdHeld`, `ids::held_ids`, `ids::import_id`.
- Produces: `pub fn write_map_card(vault: &Path, proposal: &MapProposal, today: Date, ctx: &WriteContext, journal: &mut Journal, held: &mut std::collections::BTreeSet<String>) -> Result<PathBuf, crate::write::WriteError>`
  (one parameter added, last); `sync_coursework`'s signature is unchanged (it builds `held` itself).
  In `engine/tests/sync_contract.rs`, the helpers `desk`, `item`, `fetch`, `edit`, `meta_id`, `int_at`
  and `sync_cards`, which Tasks 5–8 and 10 reuse (each is used by test (i), so none is dead code; Task 7
  adds `float_at` beside its first use).

- [ ] **Step 1: Write the failing coursework tests**

In `engine/src/coursework.rs`, inside `mod tests`, after `fn sync_first` (`:1986-1988`):

```rust
    // --- two-desktop design D1–D3: the import id ------------------------------------------------

    /// Two computers fetching one book create the one item under one id (D1), whatever path each
    /// computer's own course label gives it (D4).
    #[test]
    fn two_vaults_fetching_one_item_mint_the_same_id() {
        let a = vault_with("importid-a");
        let b = vault_with("importid-b");
        sync(&[plain()], &a, false);
        let mut elsewhere = plain();
        elsewhere.slug = "comp-100-hw-01".to_string();
        sync(&[elsewhere], &b, false);
        let id_a = field(&meta_of(&a.join("tasks").join("cs-100-hw-01.md")), "id");
        let id_b = field(&meta_of(&b.join("tasks").join("comp-100-hw-01.md")), "id");
        assert_eq!(id_a, "task_0a4d052724", "import_id(task, zybooks, coursework:zybooks:1)");
        assert_eq!(id_a, id_b);
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    /// The archived twin (a new source's past item) is the same item, so it takes the same import id.
    #[test]
    fn an_item_archived_as_imported_past_carries_its_import_id() {
        let vault = vault_with("importid-past");
        let log = sync_first(&past_and_future(), &vault, false);
        assert!(log.iter().any(|l| l == "archived (imported-past) cs-100-hw-01"), "{log:?}");
        let id = field(&meta_of(&vault.join("archive").join("cs-100-hw-01.md")), "id");
        assert_eq!(id, crate::ids::import_id("task", "zybooks", "coursework:zybooks:p1"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// D3's belt (R-TD1-1): a note carrying the item's import id but not its `source_uid` (so dedup
    /// cannot find it) makes the create `IdHeld`. The line names it, nothing is written, and the uid
    /// is recorded as seen, as a note found by `source_uid` would be.
    #[test]
    fn an_import_id_the_vault_already_holds_is_skipped_and_named() {
        let vault = vault_with("importid-held");
        std::fs::write(
            vault.join("tasks").join("renamed-by-hand.md"),
            "---\ntitle: \"CS 100 HW 01\"\nid: task_0a4d052724\n---\n\nb\n",
        )
        .unwrap();
        let log = sync(&[plain()], &vault, false);
        assert_eq!(log, vec!["skipped (already held as task_0a4d052724): cs-100-hw-01".to_string()]);
        assert!(!vault.join("tasks").join("cs-100-hw-01.md").exists());
        assert!(load_seen(&vault).contains("zybooks:1"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// §2.2: the map card's key carries the day it was first proposed, so a card re-proposed after the
    /// old one expired (into a free `approvals/` path, the old card in `archive/`) is a new card with a
    /// new id — never `IdHeld` for ever.
    #[test]
    fn a_map_card_takes_its_import_id_and_a_re_proposal_takes_another() {
        let vault = scratch_vault("importid-map");
        let ctx = crate::write::WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let p = MapProposal {
            source: "zybooks".into(),
            key: "UAHCS100Fall2026".into(),
            label: "CS 100".into(),
            suggested_course: None,
        };
        let mut held = crate::ids::held_ids(&vault);
        let first = write_map_card(&vault, &p, date(2026, 9, 10), &ctx, &mut journal, &mut held).unwrap();
        assert_eq!(field(&meta_of(&first), "id"), "appr_56e758875a");
        std::fs::rename(&first, vault.join("archive").join(first.file_name().unwrap())).unwrap();
        let again = write_map_card(&vault, &p, date(2026, 10, 11), &ctx, &mut journal, &mut held).unwrap();
        assert_eq!(
            field(&meta_of(&again), "id"),
            crate::ids::import_id("appr", "coursework", "map:zybooks:UAHCS100Fall2026:2026-10-11")
        );
        let _ = std::fs::remove_dir_all(&vault);
    }
```

Then append `, &mut std::collections::BTreeSet::new()` as the last argument of each of the eight
existing `write_map_card(` calls in `mod tests` (`:4050`, `:4089`, `:4090`, `:4110`, `:4149`, `:4258`,
`:4350`, `:4407`). Each gets its own empty set: none of those tests is about ids, and `:4089-4090`'s
second call still fails on the taken path (`Exists` is checked before any id).

- [ ] **Step 2: Write the failing LMS test file**

Create `engine/tests/import_ids.rs`:

```rust
//! Two-desktop design D1–D3 from outside the crate: the LMS feed's producer, `ingest::sync_tasks`
//! (hand-off H1), mints the item's import id, and a vault that already holds it skips the item by name.

use std::path::{Path, PathBuf};

use knowlu_engine::ingest::{load_seen, sync_tasks, Due, Event};
use knowlu_engine::journal::Journal;

fn vault(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-importids-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for folder in ["tasks", "archive", "state"] {
        std::fs::create_dir_all(dir.join(folder)).expect("mkdir");
    }
    dir
}

/// One Blackboard item, carrying the uid behind §2.1's second reference value.
fn quiz() -> Event {
    Event {
        uid: "_blackboard.platform.gradebook2.GradableItem-_4732722_1".to_string(),
        title: "Quiz 3".to_string(),
        due: Some(Due::DateTime(jiff::civil::date(2026, 10, 2).at(23, 59, 0, 0))),
        description: "Chapter 3".to_string(),
        raw: String::new(),
    }
}

fn id_of(path: &Path) -> String {
    let meta = knowlu_engine::ids::read_meta(path).expect("the note");
    knowlu_engine::yaml::get(&meta, "id").and_then(knowlu_engine::yaml::text).expect("an id")
}

#[test]
fn the_lms_feed_mints_the_items_import_id() {
    let v = vault("lms");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 9, 25)), false);
    assert_eq!(log, vec!["created task-quiz-3".to_string()]);
    assert_eq!(id_of(&v.join("tasks").join("task-quiz-3.md")), "task_d8891a504b", "§2.1's reference value");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn an_lms_item_archived_on_a_first_run_carries_the_same_import_id() {
    let v = vault("lms-past");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 10, 5)), true);
    assert_eq!(log, vec!["archived (imported-past) task-quiz-3".to_string()]);
    assert_eq!(id_of(&v.join("archive").join("task-quiz-3.md")), "task_d8891a504b");
    let _ = std::fs::remove_dir_all(&v);
}

/// R-TD1-1: a note carrying the item's import id but not its `source_uid` (so dedup cannot find it)
/// makes the create `IdHeld` — named, not written, and the uid recorded as seen.
#[test]
fn an_lms_item_whose_id_is_already_held_is_skipped_by_name_and_recorded_seen() {
    let v = vault("lms-held");
    std::fs::write(v.join("tasks").join("renamed.md"), "---\ntitle: \"Quiz 3\"\nid: task_d8891a504b\n---\n\nb\n")
        .expect("seed");
    let mut journal = Journal::new(&v);
    let log = sync_tasks(&[quiz()], &v, &[], None, &mut journal, Some(jiff::civil::date(2026, 9, 25)), false);
    assert_eq!(log, vec!["skipped (already held as task_d8891a504b): task-quiz-3".to_string()]);
    assert!(!v.join("tasks").join("task-quiz-3.md").exists());
    assert!(load_seen(&v).contains("_blackboard.platform.gradebook2.GradableItem-_4732722_1"));
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 3: Write the failing I2 scenario and its helpers**

Append to `engine/tests/sync_contract.rs`:

```rust
// ---------------------------------------------------------------------------
// Two desktops on one account, plan 1 (two-desktop design §6.1): import ids, apply by id, aliases.
// ---------------------------------------------------------------------------

/// An empty vault on a temp path, for the tests whose notes a producer makes.
fn desk(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("knowlu-td1-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for folder in ["tasks", "archive", "approvals", "courses", "state"] {
        std::fs::create_dir_all(dir.join(folder)).expect("mkdir");
    }
    dir
}

/// One zyBooks item as the fetch hands it to `coursework::sync_coursework`. `slug` is the part of its
/// path each computer's own course label decides (§2.4); the item itself is `zybooks:77` on both.
fn item(slug: &str, title: &str) -> knowlu_engine::coursework::Assignment {
    knowlu_engine::coursework::Assignment {
        uid: "zybooks:77".to_string(),
        slug: slug.to_string(),
        title: title.to_string(),
        due: jiff::civil::date(2026, 10, 9).at(23, 59, 0, 0),
        course: Some("cs-100".to_string()),
        effort_hours: 2.0,
        effort_confidence: "low".to_string(),
        effort_source: "inferred".to_string(),
        importance: 3,
        importance_reason: "reason".to_string(),
        progress: 0,
        created_by: "zybooks".to_string(),
        body: "body".to_string(),
    }
}

/// One desktop's `coursework` step: the producer itself, dated before the item is due so it is
/// created rather than archived.
fn fetch(vault: &Path, journal: &mut Journal, items: &[knowlu_engine::coursework::Assignment]) -> Vec<String> {
    let ctx = knowlu_engine::write::WriteContext::new("agent:coursework", "local-runner");
    knowlu_engine::coursework::sync_coursework(items, vault, Some("2026-09-22".parse().unwrap()), false, Some(&ctx), Some(journal))
        .expect("the coursework sync")
}

/// The student's own edit, through the console's `write` path.
fn edit(vault: &Path, rel: &str, journal: &mut Journal, fields: &[(&str, &str)]) {
    let me = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    let literals: Vec<(String, String)> = fields.iter().map(|(f, v)| (f.to_string(), v.to_string())).collect();
    knowlu_engine::write::write_literals(vault, rel, &literals, &me, journal, &Default::default()).expect("a hand edit");
}

fn meta_id(vault: &Path, rel: &str) -> String {
    let meta = knowlu_engine::ids::read_meta(&vault.join(rel)).expect("the note");
    knowlu_engine::yaml::get(&meta, "id").and_then(knowlu_engine::yaml::text).unwrap_or_default()
}

fn int_at(vault: &Path, rel: &str, field: &str) -> Option<i64> {
    let meta = knowlu_engine::ids::read_meta(&vault.join(rel))?;
    knowlu_engine::yaml::get(&meta, field).and_then(knowlu_engine::yaml::i64_of)
}

/// Every live sync amend card in `approvals/`, frontmatter only, sorted by path.
fn sync_cards(vault: &Path) -> Vec<serde_yaml_ng::Mapping> {
    let Ok(entries) = std::fs::read_dir(vault.join("approvals")) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| knowlu_engine::ids::read_meta(&p))
        .filter(|m| knowlu_engine::yaml::get(m, "created_by").and_then(knowlu_engine::yaml::text).as_deref() == Some(sync::ACTOR))
        .collect()
}

/// §6.1 (i), the review's I2 scenario end to end. Two desktops fetch item x and exchange; the student
/// sets `importance` 5 on A and then 2 on B; A's next `coursework` run updates an unrelated field (its
/// title). Before this plan x lived under two ids, A's own `importance` record never reached `mine`, and
/// the file-mtime stand-in let A's older 5 win with no card: the desktops disagreed for good. With one
/// id the later write (B's 2) is offered on A as ONE card, B keeps its 2, and each side holds one note.
#[test]
fn td1_i_one_item_fetched_on_two_desktops_is_one_note_and_a_both_sides_edit_is_one_card() {
    let (a, b) = (desk("i-a"), desk("i-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let rel = "tasks/cs-100-hw-07.md";
    fetch(&a, &mut ja, &[item("cs-100-hw-07", "CS 100 HW 07")]);
    fetch(&b, &mut jb, &[item("cs-100-hw-07", "CS 100 HW 07")]);
    assert_eq!(meta_id(&a, rel), "task_d0fd865fb3", "import_id(task, zybooks, coursework:zybooks:77)");
    assert_eq!(meta_id(&b, rel), "task_d0fd865fb3", "D1: the same item, the same id, on both");

    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let r = deliver(&b, &page, &mut jb);
    assert_eq!(r.notes_written, 0, "a foreign create for an id held here writes no file: {r:?}");
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    deliver(&a, &page, &mut ja);

    edit(&a, rel, &mut ja, &[("importance", "5")]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    edit(&b, rel, &mut jb, &[("importance", "2")]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    fetch(&a, &mut ja, &[item("cs-100-hw-07", "CS 100 HW 07 (revised)")]);

    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let rb = deliver(&b, &page, &mut jb);
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let ra = deliver(&a, &page, &mut ja);

    assert_eq!((ra.cards, rb.cards), (1, 0), "one card, on the desktop whose value lost: {ra:?} {rb:?}");
    let cards = sync_cards(&a);
    assert_eq!(cards.len(), 1);
    let offer = knowlu_engine::yaml::to_json(knowlu_engine::yaml::get(&cards[0], "changes").expect("changes"));
    assert_eq!(offer["importance"], serde_json::json!({"from": 5, "to": 2}));
    assert_eq!(int_at(&b, rel, "importance"), Some(2), "B keeps its later value");
    assert_eq!(int_at(&a, rel, "importance"), Some(5), "A's is withheld until the student answers");
    for dir in [&a, &b] {
        assert_eq!(std::fs::read_dir(dir.join("tasks")).expect("tasks").count(), 1, "one note per desktop");
    }
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
```

- [ ] **Step 4: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- coursework::tests::two_vaults coursework::tests::an_item_archived coursework::tests::an_import_id coursework::tests::a_map_card_takes`
Expected: FAIL to compile — `write_map_card` takes five arguments, not six.

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test import_ids --test sync_contract -- td1_ the_lms an_lms`
Expected: FAIL — each asserts an import id and finds a random one (`task_d8891a504b` / `task_d0fd865fb3`
against 10 random hex characters); the held case finds `created task-quiz-3`.

- [ ] **Step 5: Coursework creates through `create_imported`**

In `engine/src/coursework.rs::sync_coursework`, after `let mut seen = load_seen(vault);` (`:314`):

```rust
    // Two-desktop design D3: every id this vault already holds, read once for this run and extended
    // by `write::create_imported` as it mints — an imported note never takes an id a note has here.
    let mut held = crate::ids::held_ids(vault);
```

Replace the archive-branch create (`:528-529`):

```rust
            match crate::write::create_imported(vault, &target, &text, &item_ctx, journal, &mut held) {
                Ok(_) => {}
                // R-TD1-1: the item's note is already here under its import id. Named, not written,
                // and the uid recorded as seen — as a note found by `source_uid` would be.
                Err(crate::write::WriteError::IdHeld(id)) => {
                    log.push(format!("skipped (already held as {id}): {stem}"));
                    record_seen(vault, &item.uid, &item.title, &stamp)
                        .map_err(|err| SourceError::Failed(format!("{err}")))?;
                    continue;
                }
                Err(err) => return Err(SourceError::Failed(format!("{err}"))),
            }
```

and the active-branch create (`:573-574`) with the same `match`, word for word.

In `write_map_card` (`:1071-1114`), add the parameter `held: &mut std::collections::BTreeSet<String>`
last, and replace its last line (`:1113`) with:

```rust
    crate::write::create_imported(vault, &rel, &text, ctx, journal, held)
```

and extend its doc comment with: `` /// Two-desktop design D2: the card takes `import_id(appr, coursework,
/// map:<source>:<map_key>:<first_proposed_at>)`; `held` is the run's `ids::held_ids`. ``

In `propose_map_cards` (`:994-1045`), after `let mut journal = Journal::new(vault);` (`:1022`) add
`let mut held = crate::ids::held_ids(vault);`, pass `&mut held` as `write_map_card`'s last argument
(`:1027`), and add this arm before `Err(e) => warnings.push(...)` (`:1042`):

```rust
            // R-TD1-1: this card is already held (its key carries the day, so only a card made today
            // on another computer can be).
            Err(crate::write::WriteError::IdHeld(id)) => warnings.push(format!(
                "{}: skipped (already held as {id}): map-{}-{}",
                proposal.source,
                proposal.source,
                crate::ingest::slugify(&proposal.key)
            )),
```

Run the coursework filter from Step 4 again. Expected: PASS.

- [ ] **Step 6: Run the whole `coursework` module**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib coursework::`
Expected: PASS — every existing test too (none of them pins a random id's value).

- [ ] **Step 7: Hand-off H1 is applied**

The controller applies **H1** (`engine/src/ingest.rs::sync_tasks`, *Controller hand-offs*) verbatim as
its own commit before the next run. An implementer never edits `engine/src/ingest.rs`.

- [ ] **Step 8: Run the integration tests, then the whole suite**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test import_ids --test sync_contract`
Expected: PASS — the three `import_ids` tests, `td1_i_…`, and every existing `sync_contract` test.

Then Global Constraint 19's suite. Expected: PASS at 0 warnings.

- [ ] **Step 9: Commit**

Message file `.superpowers\sdd\msg-task-2.txt`:

```
feat(engine): coursework and the LMS feed mint import ids (two desktops, D2-D3)

sync_coursework and write_map_card create through write::create_imported,
so two computers fetching one book mint one id whatever path each one's
course label gives it; a map card's key carries its first day, so a card
re-proposed after expiry is a new card. An id the vault already holds is
named and recorded as seen. The LMS feed does the same through hand-off H1.
The review's I2 scenario, end to end, now files one card instead of
diverging.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/coursework.rs engine/tests/import_ids.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-2.txt`

---

### Task 3: Gmail, rule cards, approvals and the digest mint import ids (spec §2.2; D2, D3)

**Files:**
- Modify: `engine/src/enrich.rs` — `pull_gmail` (`:535-707`, the outcome `match` at `:622-650`),
  `write_gmail_note` (`:729-749`), `write_gmail_card` (`:783-819`), `pull_rules` (`:917-933`),
  `write_rule_card` (`:998-1040`); `mod tests`: the equivalence test (`:1742-1790`, rewritten), the
  `write_gmail_card(` call at `:1807` and the `write_rule_card(` calls at `:2448`, `:2502`, `:2542`,
  `:2579`, `:2612`, `:2653`, and four new tests.
- Modify: `engine/src/approvals.rs` — the `use crate::write::{…}` list (`:24-27`), `calendar_note`
  (`:1014-1069`), `expand_digest` (`:1099-1148`), `process_approvals` (`:1161-1206`), `transition_note`
  (`:1256-1462`: the three `expand_digest` calls at `:1277`, `:1334`, `:1433` and the task branch at
  `:1391-1402`), `materialize` (`:1473-1505`); `mod tests`: the three `calendar_note(` calls at `:3386`,
  `:3413`, `:3417`, and two new tests.
- Modify: `engine/src/eventemit.rs` — the `use crate::write::{create, WriteContext};` line (`:35`),
  `emit_digest`'s create (`:324-327`), and one new test.

**Interfaces:**
- Consumes: Task 1's `write::create_imported`, `WriteError::IdHeld`, `ids::held_ids`.
- Produces (all private to their modules): `write_gmail_note(vault, item, ctx, journal, held: &mut BTreeSet<String>) -> Result<String, crate::write::WriteError>`,
  `write_gmail_card(vault, item, today, ctx, journal, held) -> Result<String, crate::write::WriteError>`,
  `write_rule_card(vault, proposal, today, ctx, journal, held) -> Result<String, crate::write::WriteError>`
  (each error type moves from `String` to `WriteError`, so a caller can tell `IdHeld` apart);
  `calendar_note(vault, entry, ctx, journal, held: &mut BTreeSet<String>)`, `materialize(vault, path, body, ctx, journal, held: &mut BTreeSet<String>)`,
  `expand_digest(…, journal, held: &mut Option<BTreeSet<String>>)`, `transition_note(…, oldest, held: &mut Option<BTreeSet<String>>)`
  — `Option` so the index is read only when a card actually needs a note written.

- [ ] **Step 1: Write the failing enrich tests**

In `engine/src/enrich.rs`, replace the whole of `an_approved_gmail_card_materialises_the_same_note_a_task_tier_would`
(`:1742-1790`, doc comment included) with (R-TD1-16):

```rust
    /// An approved Gmail card must produce the SAME note a `tier: task` message produces, or there
    /// are two note writers and one of them will drift. Two-desktop design D2 (R-TD1-16): the two are
    /// one item, so each is written into its own vault (in one vault the second would be `IdHeld`)
    /// and they are compared byte for byte — the id line included, because both take the item's
    /// import id.
    #[test]
    fn an_approved_gmail_card_materialises_the_same_note_a_task_tier_would() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let direct_vault = vault("gmail-card-direct");
        let card_vault = vault("gmail-card-approved");
        for v in [&direct_vault, &card_vault] {
            // `vault(tag)` seeds `hw3.md` for the enrichment tests above; this test reads `tasks/` whole.
            std::fs::remove_file(v.join("tasks").join("hw3.md")).unwrap();
        }
        let item = crate::cloudmodel::GmailItem {
            uid: "gmail:m1".into(), tier: "borderline".into(),
            title: "PH 106 problem set 4".into(), course: Some("ph-106".into()),
            due: Some("2026-09-11".into()), effort_hours: Some(2.5), importance: Some(4),
            why: "the email states a Friday deadline".into(), confidence: 0.86,
        };
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let today = jiff::civil::date(2026, 9, 9);

        let mut direct_journal = Journal::new(&direct_vault);
        let direct = write_gmail_note(&direct_vault, &item, &ctx, &mut direct_journal, &mut crate::ids::held_ids(&direct_vault))
            .expect("the note writes");
        let mut card_journal = Journal::new(&card_vault);
        let card = write_gmail_card(&card_vault, &item, today, &ctx, &mut card_journal, &mut crate::ids::held_ids(&card_vault))
            .expect("the card writes");
        // Approve it exactly as the deck would, then let `process_approvals` materialise it.
        let rel = format!("approvals/{card}.md");
        crate::write::write_literals(
            &card_vault, &rel, &[("status".to_string(), "approved".to_string())],
            &ctx, &mut card_journal, &WriteOpts::default(),
        )
        .expect("approve");
        let _ = crate::approvals::process_approvals(
            &card_vault, today, jiff::civil::date(2026, 9, 9).at(9, 0, 0, 0), &ctx, &mut card_journal,
        );

        let from_tier = std::fs::read_to_string(direct_vault.join("tasks").join(format!("{direct}.md"))).unwrap();
        let materialised = crate::approvals::sorted_md(&card_vault.join("tasks"));
        assert_eq!(materialised.len(), 1, "the card produced one note: {materialised:?}");
        let from_card = std::fs::read_to_string(&materialised[0]).unwrap();
        assert_eq!(from_tier, from_card, "the same bytes, the id line included");
        // `lines()`, not a `\n` search: `pystr::write_text` writes CRLF on Windows, as vaults are.
        assert!(from_tier.lines().any(|l| l == "id: task_3bc4bec4a9"), "the item's import id (spec §2.1): {from_tier}");
        let _ = std::fs::remove_dir_all(&direct_vault);
        let _ = std::fs::remove_dir_all(&card_vault);
    }

    /// D2: a message's note and its card are one item but two notes; the kind keeps their ids apart.
    #[test]
    fn a_gmail_note_and_a_gmail_card_take_the_items_two_import_ids() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-ids");
        let item = crate::cloudmodel::GmailItem {
            uid: "gmail:m1".into(), tier: "task".into(),
            title: "PH 106 problem set 4".into(), course: Some("ph-106".into()),
            due: Some("2026-09-11".into()), effort_hours: Some(2.5), importance: Some(4),
            why: "the email states a Friday deadline".into(), confidence: 0.86,
        };
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&v);
        let mut held = crate::ids::held_ids(&v);
        let note = write_gmail_note(&v, &item, &ctx, &mut journal, &mut held).expect("the note");
        let card = write_gmail_card(&v, &item, jiff::civil::date(2026, 9, 9), &ctx, &mut journal, &mut held).expect("the card");
        let id_of = |meta: &serde_yaml_ng::Mapping| crate::yaml::opt_text(crate::yaml::get(meta, "id"));
        assert_eq!(id_of(&meta_of(&v, &format!("{note}.md"))).as_deref(), Some("task_3bc4bec4a9"));
        assert_eq!(id_of(&meta_of_approval(&v, &format!("{card}.md"))).as_deref(), Some("appr_d6015090ac"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// D2: a rule card takes `import_id(appr, rules, rule:<rule_id>)`.
    #[test]
    fn a_rule_card_takes_its_import_id() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("rules-importid");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&v);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 42, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&v, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal, &mut crate::ids::held_ids(&v))
            .expect("the card writes");
        assert_eq!(stem, "rule-42");
        let meta = meta_of_approval(&v, "rule-42.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "id")).as_deref(), Some("appr_23a359564f"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// R-TD1-1: a message whose note is already here under its import id (the other computer wrote it
    /// and it synced) is acknowledged and recorded seen, never written twice — otherwise the queue
    /// would offer it again every slot.
    #[test]
    fn a_gmail_item_already_held_under_its_import_id_is_acknowledged_and_not_written() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap();
        let v = vault("gmail-held");
        std::fs::write(
            v.join("tasks").join("from-the-other-computer.md"),
            "---\ntitle: \"PH 106 problem set 4\"\nid: task_3bc4bec4a9\n---\n\nb\n",
        )
        .unwrap();
        let (base, handle) = gmail_loopback(vec![
            gmail_reply(
                r#"[{"uid":"gmail:m1","tier":"task","payload":{"title":"PH 106 problem set 4","course":"ph-106","due":"2026-09-11","effort_hours":2.5,"importance":4,"why":"the email states a Friday deadline","confidence":0.86}}]"#,
                false,
            ),
            gmail_reply("[]", false),
        ]);
        let client = client_for(base);
        let log = v.join("_log");
        let lines = pull_gmail(&v, &client, &opts(&log), BATCH_BUDGET);

        assert!(
            lines.contains(&"gmail gmail:m1: task (skipped (already held as task_3bc4bec4a9))".to_string()),
            "{lines:?}"
        );
        assert_eq!(lines.last().unwrap(), "gmail: 0 task(s), 0 proposed, 0 dropped as information", "{lines:?}");
        assert!(!v.join("tasks").join("ph-106-problem-set-4.md").exists(), "never written twice");
        assert!(crate::ingest::load_seen(&v).contains("gmail:m1"));
        let requests = handle.join().expect("the listener thread did not panic");
        assert_eq!(requests.len(), 2, "one pull, then one ack flush: {requests:?}");
        assert!(requests[1].contains("gmail:m1"), "the held item is acknowledged: {}", requests[1]);
        let _ = std::fs::remove_dir_all(&v);
    }
```

In the same `mod tests`, append `, &mut std::collections::BTreeSet::new()` as the last argument of the
`write_gmail_card(` call at `:1807` and of the six `write_rule_card(` calls at `:2448`, `:2502`,
`:2542`, `:2579`, `:2612`, `:2653` (the last on `&waiting`). Their `.expect(...)` calls stand: `WriteError` is `Debug`.

- [ ] **Step 2: Write the failing approvals tests**

In `engine/src/approvals.rs::a_calendar_note_reproduces_pythons_bytes_exactly` (`:3375-3418`), append
`, &mut std::collections::BTreeSet::new()` as the last argument of all three `calendar_note(` calls
(`:3386`, `:3413`, `:3417`), and after `assert!(crate::ids::is_id(&id));` (`:3390`) add:

```rust
        assert_eq!(id, "appr_c40ff1180e", "two-desktop D2: import_id(appr, events, calendar-event:engage:1)");
```

After `an_approved_task_is_materialised_verbatim_apart_from_the_minted_id` (`:1891-1911`) add:

```rust
    /// Two-desktop design D2/D3: an approved card's task is the item's own note, so it takes the
    /// item's import id — the id the same message's `tier: task` note would have carried.
    #[test]
    fn an_approved_task_card_materialises_under_the_items_import_id() {
        let v = vault();
        proposal(&v, "task-study-group.md", &PENDING.replace("status: pending", "status: approved"), &approved_task_body());
        run(&v);
        assert_eq!(field(&v.join("tasks").join("study-group.md"), "id"), "task_f224b42dd6");
    }

    /// R-TD1-1: when the item's task is already here under its import id (the other computer
    /// materialised it and it synced), no second task is written; the card is stamped `executed` and
    /// archived as if it had materialised, and a warning names the note that answers it.
    #[test]
    fn an_approved_task_card_whose_task_is_already_held_settles_without_a_second_note() {
        let v = vault();
        with_task(&v, "study-group-kickoff.md", "title: Study group kickoff\nid: task_f224b42dd6");
        proposal(&v, "task-study-group.md", &PENDING.replace("status: pending", "status: approved"), &approved_task_body());
        let result = run(&v);
        assert_eq!(result.warnings, vec!["skipped (already held as task_f224b42dd6): study-group".to_string()]);
        assert!(result.executed.is_empty(), "{:?}", result.executed);
        assert!(!exists(&v, "tasks/study-group.md"), "no second note");
        let archived = read(&v.join("archive").join("task-study-group.md"));
        assert!(archived.contains("status: executed"), "{archived}");
    }
```

- [ ] **Step 3: Write the failing digest test**

In `engine/src/eventemit.rs`, inside `mod tests`, after `a_second_emit_on_the_same_day_is_a_noop` (`:570-592`):

```rust
    /// Two-desktop design D2/D3: the day's digest takes `import_id(appr, events,
    /// events-digest:<proposed_at>)`, so two computers emitting on one day mint one id — and a second
    /// digest the same day, after the first settled and freed its path, is `IdHeld` and not written.
    /// "One digest a day" was only a path check; now it is true.
    #[test]
    fn the_days_digest_takes_its_import_id_and_a_second_one_that_day_is_refused() {
        let vault = tmp("importid");
        let events = [event_at("engage:1", 25, 8, Some("AI Club Kickoff"), "")];
        let (path, count) = emit(&vault, &events, &ledger_of(vec![opp("engage:1", "strong")]), None);
        assert_eq!(count, 1);
        let path = path.unwrap();
        let (meta, _) = split_frontmatter(&pystr::read_text(&path).unwrap()).unwrap();
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "id")).as_deref(), Some("appr_07def97c80"));
        // The student settles it the same day: it moves to archive/ and the path is free again.
        fs::create_dir_all(vault.join("archive")).unwrap();
        fs::rename(&path, vault.join("archive").join("events-digest-2026-08-20.md")).unwrap();
        let later = [event_at("engage:2", 25, 8, Some("Robotics Night"), "")];
        let (again, count) = emit(&vault, &later, &ledger_of(vec![opp("engage:2", "strong")]), None);
        assert_eq!((again, count), (None, 0), "one digest a day, now by id");
        assert!(!vault.join("approvals").join("events-digest-2026-08-20.md").exists());
        let seen = pystr::read_text(&vault.join("state").join("events-seen.md")).unwrap();
        assert!(!seen.contains("- engage:2 · proposed"), "nothing is marked proposed: {seen}");
    }
```

- [ ] **Step 4: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- enrich:: approvals:: eventemit::`
Expected: FAIL to compile — `write_gmail_note`, `write_gmail_card`, `write_rule_card` and
`calendar_note` take one argument fewer than the tests pass.

- [ ] **Step 5: Enrich's three writers create through `create_imported`**

In `engine/src/enrich.rs`:

- `write_gmail_note` (`:729-749`): add `held: &mut std::collections::BTreeSet<String>` last; return
  `Result<String, crate::write::WriteError>`; `create_dir_all(...)` maps its error with
  `.map_err(|e| crate::write::WriteError::Io(e.to_string()))?`; the create becomes
  `crate::write::create_imported(vault, &rel, &text, ctx, journal, held).map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())`
  (no `map_err`).
- `write_gmail_card` (`:783-819`) and `write_rule_card` (`:998-1040`): the same three changes.
- `pull_gmail`: after `let mut journal = Journal::new(vault);` (`:546`) add
  `let mut held: Option<std::collections::BTreeSet<String>> = None;` and replace the two writer arms of
  the outcome `match` (`:637-649`) with:

```rust
                    // Two-desktop design D2/D3. `held` is read on the first write this run makes.
                    "task" => match write_gmail_note(vault, item, &ctx, &mut journal, held.get_or_insert_with(|| crate::ids::held_ids(vault))) {
                        Ok(stem) => {
                            notes += 1;
                            Ok(format!("created {stem}"))
                        }
                        // R-TD1-1: already in the vault under its import id — acknowledged and
                        // recorded seen below, like a written one, so the queue stops offering it.
                        Err(crate::write::WriteError::IdHeld(id)) => Ok(format!("skipped (already held as {id})")),
                        Err(e) => {
                            failed += 1;
                            Err(e.to_string())
                        }
                    },
                    _ => match write_gmail_card(vault, item, today, &ctx, &mut journal, held.get_or_insert_with(|| crate::ids::held_ids(vault))) {
                        Ok(stem) => {
                            cards += 1;
                            Ok(format!("proposed {stem}"))
                        }
                        Err(crate::write::WriteError::IdHeld(id)) => Ok(format!("skipped (already held as {id})")),
                        Err(e) => {
                            failed += 1;
                            Err(e.to_string())
                        }
                    },
```

- `pull_rules`: after `let existing = existing_rule_ids(vault);` (`:917`) add
  `let mut held = crate::ids::held_ids(vault);`, pass `&mut held` last to `write_rule_card` (`:925`), and
  add before its `Err(e) =>` arm (`:930`):

```rust
            // R-TD1-1: this rule's card is already held (another computer filed it and it synced).
            Err(crate::write::WriteError::IdHeld(id)) => {
                lines.push(format!("rules {}: skipped (already held as {id}): rule-{}", proposal.id, proposal.id))
            }
```

- [ ] **Step 6: Approvals' two producers create through `create_imported`**

In `engine/src/approvals.rs`:

- The `use crate::write::{…}` list (`:24-27`): replace `create` with `create_imported` (both call sites
  change, so `create` would otherwise be an unused import).
- `calendar_note` (`:1014-1069`): add `held: &mut std::collections::BTreeSet<String>` last; `:1067`
  becomes `create_imported(vault, &rel, &text, ctx, journal, held)?;` — an `IdHeld` propagates.
- `materialize` (`:1473-1505`): add `held: &mut std::collections::BTreeSet<String>` last; `:1498`
  becomes `match create_imported(vault, &rel, &content, ctx, journal, held) {`, and the propagated arm
  (`:1502`) becomes `Err(err @ (WriteError::Io(_) | WriteError::Exists(_) | WriteError::IdHeld(_))) => Err(err),`
  — without it an `IdHeld` would fall into `Err(_) => Ok(None)` and read as "missing task payload".
- `expand_digest` (`:1099-1148`): add `held: &mut Option<std::collections::BTreeSet<String>>` last, and
  replace the `match calendar_note(vault, entry, ctx, journal)? { … }` block (`:1132-1141`) with:

```rust
            match calendar_note(vault, entry, ctx, journal, held.get_or_insert_with(|| crate::ids::held_ids(vault))) {
                Ok(None) => {
                    warnings.push(format!("{name}: bad payload entry for {uid}"));
                    continue;
                }
                Ok(Some(_)) => {
                    already.insert(uid);
                    created += 1;
                }
                // R-TD1-1: this event's calendar note is already here under its import id.
                Err(WriteError::IdHeld(id)) => {
                    warnings.push(format!("skipped (already held as {id}): {uid}"));
                    already.insert(uid);
                    created += 1;
                }
                Err(e) => return Err(e),
            }
```

- `process_approvals`: before the loop (`:1175`) add
  `let mut held: Option<std::collections::BTreeSet<String>> = None;` and pass `&mut held` as
  `transition_note`'s new last argument (`:1185-1196`).
- `transition_note`: add `held: &mut Option<std::collections::BTreeSet<String>>` last; pass `held` as
  the last argument of its three `expand_digest(` calls (`:1277`, `:1334`, `:1433`); and replace the
  task branch's first four lines (`:1392-1395`) and its last line (`:1402`) so the branch reads:

```rust
        } else if kind == "task" {
            let slug = match materialize(vault, path, body, ctx, journal, held.get_or_insert_with(|| crate::ids::held_ids(vault))) {
                Ok(Some(slug)) => Some(slug),
                Ok(None) => {
                    result.warnings.push(format!("missing task payload: {name}"));
                    return Ok(());
                }
                // R-TD1-1: the item's task is already here under its import id. The card is settled
                // as if it had materialised, and the warning names the note that answers it.
                Err(WriteError::IdHeld(id)) => {
                    result.warnings.push(format!(
                        "skipped (already held as {id}): {}",
                        stem.strip_prefix("task-").unwrap_or(&stem)
                    ));
                    None
                }
                Err(e) => return Err(e),
            };
            let literals = vec![
                ("status".to_string(), "executed".to_string()),
                ("executed_at".to_string(), stamped),
            ];
            write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default())?;
            delete(vault, &rel, ctx, journal)?;
            result.executed.extend(slug);
```

- [ ] **Step 7: The digest creates through `create_imported`**

In `engine/src/eventemit.rs`, `:35` becomes `use crate::write::WriteContext;`, and `:324-327` becomes:

```rust
    let rel = format!("approvals/events-digest-{}.md", today.strftime("%Y-%m-%d"));
    // Two-desktop design D2/D3: the digest takes `import_id(appr, events, events-digest:<today>)`, so a
    // second digest the same day — after the first settled and freed the path — is `IdHeld` and not
    // written, silently, as the path check above is (R-TD1-1).
    let mut held = crate::ids::held_ids(vault);
    if crate::write::create_imported(vault, &rel, &note, ctx, journal, &mut held).is_err() {
        return (None, 0);
    }
```

- [ ] **Step 8: Run the three modules**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- enrich:: approvals:: eventemit::`
Expected: PASS — the new tests and every existing one (the materialisation tests assert `is_id`, which
an import id satisfies; `the_materialised_slug_drops_a_task_prefix_and_never_overwrites`'s second vault
holds a frontmatter-less `study-group.md`, which carries no id).

- [ ] **Step 9: Run the whole suite**

Global Constraint 19. Expected: PASS at 0 warnings — `cli.rs`'s
`crash_after_executed_stamp_heals_then_archives_without_re_expansion` included (its calendar note now
takes an import id; the uid dedup still blocks a second one).

- [ ] **Step 10: Commit**

Message file `.superpowers\sdd\msg-task-3.txt`:

```
feat(engine): Gmail, rule cards, approvals and the digest mint import ids (two desktops, D2-D3)

The six remaining producers create through write::create_imported: a Gmail
note and card, a rule card, an approved card's task, an approved event's
calendar note, and the day's events digest. A held id is named and, where
the item came from a queue or a card, settled as written (R-TD1-1). A
second digest the same day is now refused by id. The Gmail equivalence
test compares its two notes across two vaults, id line included.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/enrich.rs engine/src/approvals.rs engine/src/eventemit.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-3.txt`

---

### Task 4: The alias map and its readers (spec §2.6; D6)

**Files:**
- Modify: `engine/src/ids.rs` — after `held_ids` (Task 1), `resolve_target` (`:261-274`), `mod tests`.
- Modify: `engine/src/journal.rs` — `Journal` (`:216-220`), `Journal::new` (`:223-227`),
  `records_for` (`:253-262`), `mod tests`.

**Interfaces:**
- Consumes: Task 1's `ids::import_key`, `ids::ImportKey`; `yaml::from_json` (`yaml.rs:118`);
  `ledger::dumps_value`; `pystr::read_text`/`write_text`.
- Produces: `ids::ALIASES_FILE`, `ids::load_aliases`, `ids::save_aliases`, `ids::canonical`,
  `ids::alias_group`, `ids::import_ids_by_key`, `ids::aliases_from`, `Journal::set_aliases` — exactly as
  *Interfaces this plan produces*; `Journal::records_for` and `Journal::human_set` answer a whole alias
  group; `ids::resolve_target` answers a loser id with its group's note.

- [ ] **Step 1: Write the failing `ids.rs` tests**

In `engine/src/ids.rs::mod tests`, after `held_ids_is_every_valid_id_in_the_note_folders` (Task 1):

```rust
    fn create_record(id: &str, path: &str, new: serde_json::Value) -> crate::ledger::Record {
        let mut spec = crate::journal::NewRecord::new("create", path, "agent:coursework.zybooks", "local-runner");
        spec.id = Some(id);
        spec.new = new;
        spec.ts = Some("2026-09-20T10:00:00.000Z".to_string());
        spec.device = Some("DeskA".to_string());
        crate::journal::make_record(spec).unwrap()
    }

    /// §2.6 (re-review m3): `create` records key first, and a note on disk only when its id has no
    /// `create` record — so a later hand edit of `source_uid` never moves a note between keys, and both
    /// desktops read the same immutable input.
    #[test]
    fn import_ids_by_key_reads_create_records_first_and_notes_only_without_one() {
        let v = vault();
        let minted = serde_json::json!({"created_by": "zybooks", "source_uid": "zybooks:1", "id": "task_00000000b2"});
        let records = vec![create_record("task_00000000b2", "tasks/x.md", minted)];
        // The same note on disk, its source_uid since edited by hand: its create record still keys it.
        note(&v, "tasks/x.md", "---\ncreated_by: zybooks\nsource_uid: \"zybooks:edited\"\nid: task_00000000b2\n---\n\nb\n");
        // A note with no create record here (made before the upgrade, or a text written by a pull) keys from disk.
        note(&v, "tasks/y.md", "---\ncreated_by: zybooks\nsource_uid: \"zybooks:1\"\nid: task_00000000a1\n---\n\nb\n");
        let keys = import_ids_by_key(&records, &scan_notes(&v));
        let item: ImportKey = ("task".into(), "zybooks".into(), "coursework:zybooks:1".into());
        assert_eq!(
            keys.get(&item).map(|ids| ids.iter().cloned().collect::<Vec<_>>()),
            Some(vec!["task_00000000a1".to_string(), "task_00000000b2".to_string()])
        );
        let edited: ImportKey = ("task".into(), "zybooks".into(), "coursework:zybooks:edited".into());
        assert!(!keys.contains_key(&edited), "a hand edit does not move a note between keys");
    }

    /// §2.6: two or more ids under one key are one alias group, and the lowest id (as a string; the
    /// shared `kind_` prefix makes this compare the hex) wins — the same on every computer.
    #[test]
    fn aliases_from_maps_every_loser_to_the_lowest_id() {
        let key = |k: &str| -> ImportKey { ("task".into(), "zybooks".into(), k.into()) };
        let mut keys: BTreeMap<ImportKey, BTreeSet<String>> = BTreeMap::new();
        keys.insert(
            key("coursework:zybooks:1"),
            BTreeSet::from(["task_00000000b2".to_string(), "task_00000000a1".to_string(), "task_00000000c3".to_string()]),
        );
        keys.insert(key("coursework:zybooks:2"), BTreeSet::from(["task_00000000d4".to_string()]));
        let aliases = aliases_from(&keys);
        assert_eq!(
            aliases,
            BTreeMap::from([
                ("task_00000000b2".to_string(), "task_00000000a1".to_string()),
                ("task_00000000c3".to_string(), "task_00000000a1".to_string()),
            ])
        );
        assert_eq!(canonical(&aliases, "task_00000000c3"), "task_00000000a1");
        assert_eq!(canonical(&aliases, "task_00000000d4"), "task_00000000d4");
        let group = |id: &str| alias_group(&aliases, id).into_iter().collect::<Vec<_>>();
        assert_eq!(group("task_00000000b2"), vec!["task_00000000a1", "task_00000000b2", "task_00000000c3"]);
        assert_eq!(group("task_00000000d4"), vec!["task_00000000d4"]);
    }

    /// `state/id-aliases.json`: written through `ledger::dumps_value`; missing or unreadable is `{}`.
    #[test]
    fn the_alias_file_round_trips_and_a_missing_or_broken_one_is_empty() {
        let v = vault();
        assert!(load_aliases(&v).is_empty());
        let map = BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())]);
        save_aliases(&v, &map).unwrap();
        assert_eq!(load_aliases(&v), map);
        assert_eq!(pystr::read_text(&v.join(ALIASES_FILE)).unwrap(), "{\"task_00000000b2\": \"task_00000000a1\"}");
        pystr::write_text(&v.join(ALIASES_FILE), "not json").unwrap();
        assert!(load_aliases(&v).is_empty());
    }

    /// Review M3: an issue's `target_id`, and anything else naming the old id, keeps working.
    #[test]
    fn resolve_target_answers_an_old_id_with_its_groups_note() {
        let v = vault();
        note(&v, "tasks/x.md", "---\nid: task_00000000a1\n---\n\nb\n");
        save_aliases(&v, &BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())])).unwrap();
        assert_eq!(resolve_target(&v, "task_00000000b2").unwrap(), v.join("tasks").join("x.md"));
        assert_eq!(resolve_target(&v, "task_00000000c3").unwrap_err(), IdError::UnknownId("task_00000000c3".into()));
    }
```

- [ ] **Step 2: Write the failing `journal.rs` tests**

In `engine/src/journal.rs::mod tests`, after `human_set_ignores_agent_writes` (`:439-446`):

```rust
    /// Two-desktop design D6: with an alias group on file, `records_for` answers every id of the
    /// group, so the loser's history is the winner's too — and judge-once reads through the same map.
    #[test]
    fn records_for_and_human_set_follow_the_alias_file_to_the_whole_group() {
        let v = vault();
        crate::ids::save_aliases(
            &v,
            &std::collections::BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())]),
        )
        .unwrap();
        let mut j = Journal::new(&v);
        let mut a = set_rec("task_00000000a1", "due", "agent:x", "2026-08-29T12:00:00.000Z");
        let mut b = set_rec("task_00000000b2", "importance", "quinn", "2026-08-29T12:00:01.000Z");
        let mut c = set_rec("task_00000000c3", "due", "quinn", "2026-08-29T12:00:02.000Z");
        j.append(&mut a).unwrap();
        j.append(&mut b).unwrap();
        j.append(&mut c).unwrap();
        let ids = |records: Vec<Record>| records.iter().map(|r| str_of(r, "id").unwrap()).collect::<Vec<_>>();
        assert_eq!(ids(j.records_for("task_00000000a1", None)), vec!["task_00000000a1", "task_00000000b2"]);
        assert_eq!(ids(j.records_for("task_00000000b2", None)), vec!["task_00000000a1", "task_00000000b2"]);
        assert_eq!(ids(j.records_for("task_00000000c3", None)), vec!["task_00000000c3"]);
        assert!(j.human_set("task_00000000a1", "importance").is_some(), "set by the student under the loser id");
    }

    /// `set_aliases` installs the map an `apply` has just rebuilt, with no re-read of the file.
    #[test]
    fn set_aliases_replaces_the_map_the_journal_reads_through() {
        let v = vault();
        let mut j = Journal::new(&v);
        let mut b = set_rec("task_00000000b2", "due", "quinn", "2026-08-29T12:00:01.000Z");
        j.append(&mut b).unwrap();
        assert!(j.records_for("task_00000000a1", None).is_empty());
        j.set_aliases(std::collections::BTreeMap::from([("task_00000000b2".to_string(), "task_00000000a1".to_string())]));
        assert_eq!(j.records_for("task_00000000a1", None).len(), 1);
    }
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- ids:: journal::`
Expected: FAIL to compile — `import_ids_by_key`, `aliases_from`, `canonical`, `alias_group`,
`load_aliases`, `save_aliases`, `ALIASES_FILE` and `Journal::set_aliases` do not exist.

- [ ] **Step 4: Implement the alias map in `ids.rs`**

In `engine/src/ids.rs`, after `held_ids`:

```rust
/// Two-desktop design D6: `{loser id: winner id}` for every alias group — one imported item under
/// two or more ids. Generated and device-local; never synced (`sync::build_push` reads only the note
/// folders); rebuilt by every `sync::apply`. A lost file costs one more full reconcile, never a wrong
/// write (re-review m2).
pub const ALIASES_FILE: &str = "state/id-aliases.json";

/// Missing or unreadable is `{}`, never an error: a vault with no doubles has no groups.
pub fn load_aliases(vault: &Path) -> BTreeMap<String, String> {
    pystr::read_text(&vault.join(ALIASES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Through `ledger::dumps_value`, and a temp file renamed into place, as `sync`'s other generated
/// files are written, so a crash never leaves half a file.
pub fn save_aliases(vault: &Path, aliases: &BTreeMap<String, String>) -> Result<(), String> {
    let path = vault.join(ALIASES_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(aliases).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    pystr::write_text(&tmp, &crate::ledger::dumps_value(&value)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// The winner of `id`'s group, or `id` itself when it is in none.
pub fn canonical<'a>(aliases: &'a BTreeMap<String, String>, id: &'a str) -> &'a str {
    aliases.get(id).map(String::as_str).unwrap_or(id)
}

/// Every id of `id`'s group, the winner included; `{id}` when it is in none.
pub fn alias_group(aliases: &BTreeMap<String, String>, id: &str) -> BTreeSet<String> {
    let winner = canonical(aliases, id).to_string();
    let mut group: BTreeSet<String> =
        aliases.iter().filter(|(_, w)| **w == winner).map(|(loser, _)| loser.clone()).collect();
    group.insert(winner);
    group
}

/// D6's input: every import key, and the ids that name it. From every `create` record first — a
/// `create` record's `new` is the whole frontmatter as minted (`write.rs:416-418`), immutable, so a
/// later hand edit of `source_uid` or `created_by` never moves a note between keys — and from a note
/// on disk only when its id has no `create` record (re-review m3).
pub fn import_ids_by_key(
    records: &[crate::ledger::Record],
    notes: &[(PathBuf, Option<Mapping>)],
) -> BTreeMap<ImportKey, BTreeSet<String>> {
    let mut keys: BTreeMap<ImportKey, BTreeSet<String>> = BTreeMap::new();
    let mut created: BTreeSet<String> = BTreeSet::new();
    for record in records {
        let field = |name: &str| record.get(name).and_then(serde_json::Value::as_str).unwrap_or_default();
        if field("op") != "create" || !is_id(field("id")) {
            continue;
        }
        created.insert(field("id").to_string());
        let minted = crate::yaml::from_json(record.get("new").unwrap_or(&serde_json::Value::Null));
        let serde_yaml_ng::Value::Mapping(meta) = minted else { continue };
        if let Some(key) = import_key(Path::new(field("path")), &meta) {
            keys.entry(key).or_default().insert(field("id").to_string());
        }
    }
    for (path, meta) in notes {
        let Some(meta) = meta else { continue };
        let Some(id) = crate::yaml::get(meta, "id").and_then(crate::yaml::text).filter(|id| is_id(id)) else {
            continue;
        };
        if created.contains(&id) {
            continue;
        }
        if let Some(key) = import_key(path, meta) {
            keys.entry(key).or_default().insert(id);
        }
    }
    keys
}

/// D6: a key with two or more ids is an alias group, and its lowest id as a string wins. Both desktops
/// compute the same group and winner with no coordination and no knowledge of which id is derived.
pub fn aliases_from(keys: &BTreeMap<ImportKey, BTreeSet<String>>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for ids in keys.values().filter(|ids| ids.len() > 1) {
        let mut ids = ids.iter();
        let Some(winner) = ids.next() else { continue };
        for loser in ids {
            out.insert(loser.clone(), winner.clone());
        }
    }
    out
}
```

In `resolve_target` (`:261-274`), replace the id branch (`:262-268`) so an id this vault does not hold
is answered by its group's note, the winner first (review M3). The alias file is read only on that
miss: `resolve_target` runs on every `write`, and a held id needs no map.

```rust
    if is_id(target) {
        let index = build_index(vault);
        let path = match index.get(target) {
            Some(path) => path.clone(),
            None => {
                let aliases = load_aliases(vault);
                index
                    .get(canonical(&aliases, target))
                    .or_else(|| alias_group(&aliases, target).iter().find_map(|id| index.get(id)))
                    .cloned()
                    .ok_or_else(|| IdError::UnknownId(target.to_string()))?
            }
        };
        return inside_vault(vault, &path);
    }
```

- [ ] **Step 5: `Journal` reads through the map**

In `engine/src/journal.rs`, add a private field to `Journal` (`:216-220`) and initialise it in `new`
(`:223-227`) as `aliases: None`:

```rust
    /// Two-desktop design D6: `{loser: winner}`, read from `ids::ALIASES_FILE` at first use, or
    /// installed by the `sync::apply` that has just rebuilt it (`set_aliases`). Not cleared by
    /// `invalidate`, which is about the ledger's cache only.
    aliases: Option<std::collections::BTreeMap<String, String>>,
```

and add, after `append` (`:234-239`):

```rust
    /// D6: install the alias map an `apply` has just rebuilt, with no re-read of the file.
    pub fn set_aliases(&mut self, aliases: std::collections::BTreeMap<String, String>) {
        self.aliases = Some(aliases);
    }

    /// D6: every id of `note_id`'s alias group (just `note_id` when it is in none).
    fn group_of(&mut self, note_id: &str) -> std::collections::BTreeSet<String> {
        let vault = self.vault.clone();
        let aliases = self.aliases.get_or_insert_with(|| crate::ids::load_aliases(&vault));
        crate::ids::alias_group(aliases, note_id)
    }
```

and make `records_for` (`:253-262`) filter on the group:

```rust
    /// Every record for `note_id` — and, with an alias group on file (two-desktop design D6), for every
    /// id of its group, so a loser's history is the winner's. `human_set` reads through this.
    pub fn records_for(&mut self, note_id: &str, field: Option<&str>) -> Vec<Record> {
        let group = self.group_of(note_id);
        self.read(None, None)
            .into_iter()
            .filter(|r| str_of(r, "id").is_some_and(|id| group.contains(&id)))
            .filter(|r| match field {
                None => true,
                Some(f) => str_of(r, "field").as_deref() == Some(f),
            })
            .collect()
    }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- ids:: journal::`
Expected: PASS.

- [ ] **Step 7: Run the whole suite**

Global Constraint 19. Expected: PASS at 0 warnings — no fixture vault carries `state/id-aliases.json`,
so every group is a single id and nothing that reads `records_for` (`surface.rs:1697`,
`sync.rs:1970`, `write.rs:253`) moves; `surface_oracle.rs` included.

- [ ] **Step 8: Commit**

Message file `.superpowers\sdd\msg-task-4.txt`:

```
feat(engine): the alias map and its readers (two desktops, D6)

ids.rs keys every imported note from its create record (or, with none,
from the note on disk) and maps each loser id to the lowest id of its
group, saved as state/id-aliases.json - generated, device-local, never
synced. Journal::records_for and human_set answer the whole group, and
resolve_target answers an old id with the group's note. Nothing builds
the map yet; sync::apply does in a later task.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/ids.rs engine/src/journal.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-4.txt`

---

### Task 5: `apply` by id — reconcile and pulled texts (spec §2.4, §2.5 (a) and (c); D4, D5)

**Files:**
- Modify: `engine/src/write.rs` — `fn free_slot` (`:431`) becomes `pub(crate) fn free_slot` (R-TD1-17).
- Modify: `engine/src/sync.rs` — a new private `IdIndex` and `note_has_no_id` before `pub fn apply`
  (after `changes_json`, `:1698-1707`); in `apply`: the index built before step 4 (`:1904`), step 4's
  head (`:1905-1924`), and step 6's live branch (`:2243-2318`).
- Modify: `engine/tests/sync_contract.rs` — two tests amended (`a_seed_the_student_has_edited_keeps_todays_never_overwrite_rule`,
  `:2026-2058`; `with_no_seed_hashes_file_apply_is_unchanged`, `:2099-2136`), and two new tests.

**Interfaces:**
- Consumes: Task 4's `ids::canonical`, `ids::alias_group`; `ids::build_index`; `write::free_slot`.
- Produces (private to `sync.rs`; Tasks 6–8 extend them):

```rust
#[derive(Debug, Default)]
struct IdIndex {
    by_id: std::collections::BTreeMap<String, String>,
    aliases: std::collections::BTreeMap<String, String>,
}
impl IdIndex {
    fn build(vault: &Path) -> IdIndex;
    fn exact(&self, id: &str) -> Option<&String>;   // the note holding exactly `id`
    fn holder(&self, id: &str) -> Option<&String>;  // `id` or any id of its alias group, the winner first
    fn place(&mut self, id: &str, rel: &str);
    fn forget(&mut self, id: &str);
}
fn note_has_no_id(file: &Path) -> bool;
```

- [ ] **Step 1: Amend the two tests that pinned the silent drop D5 (c) removes**

`today that second text is dropped with no line (sync.rs:2273-2275)` is what §2.5 (c) changes, and two
tests pinned it. In `engine/tests/sync_contract.rs`, replace `a_seed_the_student_has_edited_keeps_todays_never_overwrite_rule`
(`:2026-2058`, doc comment included) with:

```rust
/// N2's other half, with two-desktop design D5 (c): a seed the student HAS edited since — its BYTES no
/// longer match the hash `restore_into` recorded — is never overwritten, because a real change has been
/// made here. The account's copy is a different note (a different id), and every id gets one file: it
/// lands at the next free name beside the seed, named in one line. Before D5 it was dropped with no line.
#[test]
fn a_seed_the_student_has_edited_is_never_overwritten_and_the_accounts_copy_lands_beside_it() {
    let dir = fixture_with_id("edited-seed-keeps-rule");
    let mut journal = Journal::new(&dir);
    let seed_ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    let seed_text = "---\nid: course_0000000002\n---\nExams: TBD\n";
    knowlu_engine::write::create(&dir, "courses/cs-200.md", seed_text, &seed_ctx, &mut journal, None)
        .expect("the wizard's own seed");
    seed_hash(&dir, "courses/cs-200.md");
    knowlu_engine::pystr::write_text(
        &dir.join("courses").join("cs-200.md"),
        "---\nid: course_0000000002\n---\nExams: TBD, but I filled in the weights myself: 60/40\n",
    ).expect("the student's own hand edit");

    let apply_ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let note = sync::PulledNote {
        device: "fedcba9876543210".to_string(),
        path: "courses/cs-200.md".to_string(),
        text: Some("---\nid: course_00000000cd\n---\nExams 60%, labs 40%\n".to_string()),
    };
    let report = sync::apply(&dir, &pulled(vec![], vec![note]), &apply_ctx, &mut journal, "2026-09-22".parse().unwrap());
    let now = knowlu_engine::pystr::read_text(&dir.join("courses").join("cs-200.md")).expect("the note");
    assert!(now.contains("I filled in the weights myself"), "a hand-edited seed is not silently overwritten: {now:?}");
    assert_eq!(report.notes_written, 1, "{report:?}");
    assert_eq!(
        knowlu_engine::pystr::read_text(&dir.join("courses").join("cs-200-2.md")).expect("the account's copy"),
        "---\nid: course_00000000cd\n---\nExams 60%, labs 40%\n"
    );
    assert!(
        report.warnings.contains(&"sync: courses/cs-200.md holds a different note here; course_00000000cd was written to courses/cs-200-2.md".to_string()),
        "{:?}", report.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}
```

and replace `with_no_seed_hashes_file_apply_is_unchanged` (`:2099-2136`, doc comment included) with a
test of the same setup whose name and last assertions follow D5 (c):

```rust
/// R1's own probes with two-desktop design D5 (c): a vault with NO `state/seed-hashes.json` never has a
/// veteran desktop's hand-typed course, or a console-created task, overwritten by a foreign note at the
/// same path. Each foreign note is a different note (a different id), so it lands at the next free name.
#[test]
fn with_no_seed_hashes_file_a_foreign_copy_lands_beside_the_note_never_over_it() {
    let dir = fixture_with_id("no-seed-hashes-file");
    assert!(!dir.join(sync::SEED_HASHES_FILE).exists(), "this fixture never restored");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
    knowlu_engine::write::create(&dir, "courses/cs-300.md", "---\nid: course_0000000003\n---\nExams: TBD\n", &ctx, &mut journal, None)
        .expect("created once");
    knowlu_engine::pystr::write_text(&dir.join("courses").join("cs-300.md"), "---\nid: course_0000000003\n---\nExams 60%, labs 40%, hand-typed from the syllabus\n")
        .expect("typed in by hand, never journalled");
    knowlu_engine::write::create(&dir, "tasks/my-own-task.md", "---\nid: task_0000000009\n---\nMine\n", &ctx, &mut journal, None)
        .expect("created in the console");

    let apply_ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let notes = vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: "courses/cs-300.md".into(), text: Some("---\nid: course_00000000ee\n---\nForeign copy\n".into()) },
        sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/my-own-task.md".into(), text: Some("---\nid: task_00000000ff\n---\nForeign task\n".into()) },
    ];
    let report = sync::apply(&dir, &pulled(vec![], notes), &apply_ctx, &mut journal, "2026-09-22".parse().unwrap());
    let read = |rel: &str| knowlu_engine::pystr::read_text(&dir.join(rel)).expect(rel);
    assert!(read("courses/cs-300.md").contains("hand-typed"), "the veteran desktop's hand-typed course is not overwritten");
    assert!(read("tasks/my-own-task.md").contains("Mine"), "the console-created task is not overwritten");
    assert_eq!(report.notes_written, 2, "{report:?}");
    assert!(read("courses/cs-300-2.md").contains("Foreign copy"));
    assert!(read("tasks/my-own-task-2.md").contains("Foreign task"));
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Write the two new failing tests**

Append to `engine/tests/sync_contract.rs`:

```rust
/// §6.1 (ii), D4 and D5: each computer's own course label puts x at a different path (§2.4). Each
/// desktop keeps one file — the other's text is named and not written, its id being held here, and
/// the other's `create` is journalled with no file — and a later edit travels by id to the other
/// desktop's own path.
#[test]
fn td1_ii_one_item_at_two_paths_stays_one_file_each_side_and_an_edit_travels_by_id() {
    let (a, b) = (desk("ii-a"), desk("ii-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let (a_rel, b_rel) = ("tasks/cs-100-hw-07.md", "tasks/comp-100-hw-07.md");
    fetch(&a, &mut ja, &[item("cs-100-hw-07", "CS 100 HW 07")]);
    fetch(&b, &mut jb, &[item("comp-100-hw-07", "COMP 100 HW 07")]);

    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let r = deliver(&a, &page, &mut ja);
    assert_eq!(r.notes_written, 0, "{r:?}");
    assert!(
        r.warnings.contains(&format!("sync: {b_rel} is task_d0fd865fb3, already held here as {a_rel}")),
        "{:?}", r.warnings
    );
    ja.invalidate();
    assert!(
        ja.records_for("task_d0fd865fb3", None).iter().any(|rec| rec.get("op").and_then(|v| v.as_str()) == Some("create")
            && rec.get("device").and_then(|v| v.as_str()) == Some("DeskB")),
        "B's create is journalled here"
    );
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    deliver(&b, &page, &mut jb);
    assert!(!a.join(b_rel).exists() && !b.join(a_rel).exists(), "one file each side");

    edit(&a, a_rel, &mut ja, &[("importance", "5")]);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let r = deliver(&b, &page, &mut jb);
    assert_eq!((r.applied, r.cards), (1, 0), "{r:?}");
    assert_eq!(int_at(&b, b_rel, "importance"), Some(5), "the edit reached B's own path");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// D5 (a): a foreign record reconciles against the note holding its id — never against a different
/// note that happens to sit at the record's path here.
#[test]
fn td1_a_foreign_record_never_reconciles_against_a_different_note_at_its_path() {
    let dir = fixture_with_id("td1-different-note"); // tasks/cs-100-hw-01.md is task_0000000001, importance 2
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let rec = foreign_set("task_00000000ff", "tasks/cs-100-hw-01.md", "importance", serde_json::json!(2), serde_json::json!(5), "2026-09-17T10:00:00.000Z");
    let report = sync::apply(&dir, &pulled(vec![rec], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert_eq!((report.records, report.applied, report.cards, report.superseded), (1, 0, 0, 0), "{report:?}");
    assert_eq!(int_at(&dir, "tasks/cs-100-hw-01.md", "importance"), Some(2), "the other note is untouched");
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract -- td1_ii td1_a_ a_seed_the_student with_no_seed_hashes`
Expected: FAIL — (ii): `notes_written` is 1 (B's text written at its own free path, one id on two
files); the D5 (a) test: `applied` is 1 (reconciled by path onto task_0000000001); the two amended tests:
`notes_written` is 0 (the text dropped with no line).

- [ ] **Step 4: `free_slot` for `sync`**

In `engine/src/write.rs:431`, `fn free_slot` becomes `pub(crate) fn free_slot`, and gains the doc line
`/// Also \`sync::apply\`'s rule for a pulled note whose path holds a different note (two-desktop D5 (c)).`

- [ ] **Step 5: The id index**

In `engine/src/sync.rs`, after `changes_json` (`:1698-1707`):

```rust
/// Two-desktop design D5: which note holds each id here, vault-relative — built once per `apply` and
/// kept current as `apply` moves, settles, re-identifies and writes notes, so every step asks one map
/// rather than re-scanning the folders. The first path wins, as `ids::build_index` rules. `aliases` is
/// the alias map the pre-pass builds (empty until then), so `holder` finds a group by any of its ids.
#[derive(Debug, Default)]
struct IdIndex {
    by_id: std::collections::BTreeMap<String, String>,
    aliases: std::collections::BTreeMap<String, String>,
}

impl IdIndex {
    fn build(vault: &Path) -> IdIndex {
        let by_id = crate::ids::build_index(vault)
            .into_iter()
            .map(|(id, path)| (id, crate::ids::rel(vault, &path)))
            .collect();
        IdIndex { by_id, aliases: std::collections::BTreeMap::new() }
    }

    /// The note holding exactly `id`.
    fn exact(&self, id: &str) -> Option<&String> {
        self.by_id.get(id)
    }

    /// The note holding `id` or any id of its alias group, the group's winner first (D5 (a), D6).
    fn holder(&self, id: &str) -> Option<&String> {
        self.by_id.get(crate::ids::canonical(&self.aliases, id)).or_else(|| {
            crate::ids::alias_group(&self.aliases, id).iter().find_map(|g| self.by_id.get(g.as_str()))
        })
    }

    fn place(&mut self, id: &str, rel: &str) {
        self.by_id.insert(id.to_string(), rel.to_string());
    }

    fn forget(&mut self, id: &str) {
        self.by_id.remove(id);
    }
}

/// A readable note with no valid `id:` line: the one kind of note only its path identifies (D5 (a),
/// R-TD1-2). A missing or unreadable file is not one.
fn note_has_no_id(file: &Path) -> bool {
    crate::ids::read_meta(file).is_some_and(|meta| {
        !crate::yaml::get(&meta, "id").and_then(crate::yaml::text).is_some_and(|id| crate::ids::is_id(&id))
    })
}
```

(`exact` and `forget` have no caller until Task 7: give each `#[allow(dead_code)]` now, and Task 7
removes both attributes, so every task ends at 0 warnings.)

In `apply`, immediately before `// 4. Per note, with the roles reversed exactly as the table above says.` (`:1904`):

```rust
    // D5 (two-desktop design §2.5): which note holds each id here, read after the record pass, the
    // moves and the seed pre-pass, and kept current below as this apply writes notes.
    let mut index = IdIndex::build(vault);
```

- [ ] **Step 6: Step 4 finds the note by id**

In `apply`'s step 4, replace the loop's head (`:1905-1924` — from `for (id, foreign) in &touched {`
through the `let Some(meta) = crate::ids::read_meta(&file) else { … };` block, the N2 comment kept) with:

```rust
    for (id, foreign) in &touched {
        if foreign.is_empty() { continue; }
        let record_path = foreign
            .iter()
            .rev()
            .find_map(|r| r.get("path").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        // D5 (a) (two-desktop design §2.5): the note holding this id, wherever it is here — never a
        // different note that happens to sit at the record's path. The record's path is consulted only
        // for a note there with no `id:` line, which only its path identifies (R-TD1-2).
        let path = match index.holder(id) {
            Some(rel) => rel.clone(),
            None if note_has_no_id(&vault.join(&record_path)) => record_path,
            None => continue,
        };
        // N2: this path was just replaced wholesale by the pre-pass above — reconciling the
        // account's own records (a different id than the seed's, in any case) against a file that no
        // longer holds the seed at all would merge fields the replacement already delivered in full,
        // and journal a spurious echo doing it. Nothing left to reconcile here.
        if replaced_paths.contains(&path) {
            continue;
        }
        let file = vault.join(&path);
        let Some(meta) = crate::ids::read_meta(&file) else {
            // No local file: nothing to reconcile. The note's own text arrives below, if it came.
            continue;
        };
```

Everything after it in the loop is unchanged: it already uses `path` and `file`, which now name the
local note (so `write_literals`, `live_sync_cards` and the card's `target` use the local path, D5 (a)).

- [ ] **Step 7: Step 6 writes one file per id**

In step 6's live branch (`Some(text) => { … }`, `:2243-2318`), keep the foreign-sync-card refusal
(`:2250-2257`) and the comments above `exact_case_exists` as they stand, and replace everything from
`if exact_case_exists(&file) {` (`:2273`) to the end of the branch's `match result { … }` (`:2317`) with:

```rust
                let text_id = note_frontmatter_id(text).filter(|id| crate::ids::is_id(id));
                // D5 (c) (two-desktop design §2.5): one id, one file, and every id gets one.
                let mut target = note.path.clone();
                if exact_case_exists(&file) {
                    // The path is held. The same note — the same id, or a note here with no `id:` line,
                    // which its path identifies (R-TD1-2) — keeps today's rule: never overwritten.
                    let local_id = crate::ids::read_meta(&file)
                        .and_then(|m| crate::yaml::get(&m, "id").and_then(crate::yaml::text))
                        .filter(|id| crate::ids::is_id(id));
                    match (&text_id, &local_id) {
                        (Some(theirs), Some(ours)) if theirs != ours => {
                            // A different note holds this path: the text takes the next free name
                            // (`write::free_slot`'s rule), unless its id is already held here.
                            let folder = file.parent().unwrap_or(vault);
                            let name = file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            target = crate::ids::rel(vault, &crate::write::free_slot(folder, &name));
                        }
                        _ => continue,
                    }
                } else if let Some(existing) = case_insensitive_match(&file) {
                    // O1 / B3, as before: this device's own copy of the same note, cased differently.
                    let local_id = crate::ids::read_meta(&existing)
                        .and_then(|m| crate::yaml::get(&m, "id").and_then(crate::yaml::text));
                    let live_id = note_frontmatter_id(text);
                    if local_id.is_some() && local_id == live_id {
                        match rename_case_only(&existing, &file) {
                            Ok(()) => {
                                report.moved += 1;
                                if let Some(id) = &text_id {
                                    index.place(id, &note.path);
                                }
                            }
                            Err(e) => report.warnings.push(format!(
                                "sync: {} could not be renamed to match the account's spelling ({e}); keeping the old spelling",
                                note.path
                            )),
                        }
                        continue;
                    }
                }
                // D5 (a) and (c): a text whose id is already held here — a foreign `create` for a held
                // id, or one item at two paths (D4) — writes no second file.
                if let Some(id) = &text_id {
                    if let Some(local) = index.holder(id) {
                        report.warnings.push(format!("sync: {} is {id}, already held here as {local}", note.path));
                        continue;
                    }
                }
                let file = vault.join(&target);
                if let Some(parent) = file.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let translated = if crate::pystr::NEWLINE == "\n" {
                    text.clone()
                } else {
                    crate::pystr::universal_newlines(text).replace('\n', crate::pystr::NEWLINE)
                };
                let result = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&file)
                    .and_then(|mut f| f.write_all(translated.as_bytes()));
                match result {
                    Ok(()) => {
                        report.notes_written += 1;
                        if let Some(id) = &text_id {
                            index.place(id, &target);
                            if target != note.path {
                                report.warnings.push(format!(
                                    "sync: {} holds a different note here; {id} was written to {target}",
                                    note.path
                                ));
                            }
                        }
                    }
                    Err(e) => report.warnings.push(format!("sync: {target} could not be written ({e})")),
                }
```

Keep the doc comment block above the old `exact_case_exists` check (`:2258-2272`) and add one sentence to
its end: `Two-desktop design D5 (c): a path held by a DIFFERENT note (another id) no longer drops the
text; it takes the next free name, and a text whose id is held anywhere here is never written twice.`

- [ ] **Step 8: Run the sync suites**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract --test sync_replay`
Expected: PASS — the four tests of Steps 1–2, `td1_i_…` (Task 2), and every existing test:
`a_pulled_note_never_overwrites_a_note_this_device_already_has` (its local note has no `id:`, so it keeps
today's rule), the O1/B3 case tests (their branch is unchanged), and the seed tests (the index is built
after the seed pre-pass, and `replaced_paths` is checked against the holder's path).

- [ ] **Step 9: Run the whole suite**

Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 10: Commit**

Message file `.superpowers\sdd\msg-task-5.txt`:

```
feat(engine): sync applies records and pulled texts by id (two desktops, D4-D5)

apply builds one id index and finds the note holding each id wherever it
is, never a different note at the record's path; a note with no id: line
is still found by its path. A pulled text whose id is held here writes no
second file and says so; one whose path holds a different note takes the
next free -N name instead of being dropped with no line. Two tests that
pinned the old silent drop are amended to D5 (c).

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/src/write.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-5.txt`

---

### Task 6: `apply` by id — moves, deletes and tombstones (spec §2.5 (b) and (d), §2.7; D5, D7; §5.4 E5, M5)

**Files:**
- Modify: `engine/src/sync.rs` — `IdIndex` (a `vacate` method), a new `note_id_at`; in `apply`: the
  record pass (`:1745-1811`: `moves` and a new `deletes`), the index build (moved up to just after the
  record pass, `:1812`), step 3a (`:1814-1840`), the seed pre-pass (`:1890-1897`), and step 6's
  tombstone arm (`:2210-2242`).
- Modify: `engine/tests/sync_contract.rs` — four new tests.

**Interfaces:**
- Consumes: Task 5's `IdIndex`, `note_has_no_id`; `write::move_note`, `write::delete`.
- Produces: `IdIndex::vacate(&mut self, rel: &str)` (every id at `rel` forgotten) and
  `fn note_id_at(file: &Path) -> Option<String>` (a note's valid `id:`), both private to `sync.rs`.

- [ ] **Step 1: Write the failing tests**

Append to `engine/tests/sync_contract.rs`:

```rust
/// §6.1 (iii), D7 (review M5): the student deletes x on A while B edits it. x sits at a different path
/// on each desktop (D4), so only its id can find it. Whatever the push order, x ends archived on both
/// desktops with B's edit on the archived copy, and no card: on B, A's `delete` record archives B's
/// copy, edit and all (D5 (b)) — A's tombstone names A's path, which B does not have; on A, B's `set`
/// finds the archived copy by id (D5 (a)), and B's live text cannot bring x back, its id being held in
/// `archive/` (D5 (c)).
#[test]
fn td1_iii_a_delete_beats_a_concurrent_edit_in_either_order_and_nothing_is_lost() {
    for a_first in [true, false] {
        let tag = if a_first { "a-first" } else { "b-first" };
        let (a, b) = (desk(&format!("iii-{tag}-a")), desk(&format!("iii-{tag}-b")));
        let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
        let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
        let (a_rel, b_rel) = ("tasks/cs-100-hw-07.md", "tasks/comp-100-hw-07.md");
        fetch(&a, &mut ja, &[item("cs-100-hw-07", "CS 100 HW 07")]);
        fetch(&b, &mut jb, &[item("comp-100-hw-07", "COMP 100 HW 07")]);
        let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
        deliver(&b, &page, &mut jb);
        let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
        deliver(&a, &page, &mut ja);

        let student = knowlu_engine::write::WriteContext::new("quinn", "dashboard");
        knowlu_engine::write::delete(&a, a_rel, &student, &mut ja).expect("the student deletes x on A");
        edit(&b, b_rel, &mut jb, &[("importance", "5")]);
        let a_page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
        let b_page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
        let (ra, rb) = if a_first {
            let rb = deliver(&b, &a_page, &mut jb);
            (deliver(&a, &b_page, &mut ja), rb)
        } else {
            let ra = deliver(&a, &b_page, &mut ja);
            (ra, deliver(&b, &a_page, &mut jb))
        };
        assert_eq!((ra.cards, rb.cards), (0, 0), "{tag}: {ra:?} {rb:?}");
        for (dir, rel, archived, who) in [(&a, a_rel, "archive/cs-100-hw-07.md", "A"), (&b, b_rel, "archive/comp-100-hw-07.md", "B")] {
            assert!(!dir.join(rel).exists(), "{tag}: x is not live on {who}");
            assert_eq!(int_at(dir, archived, "importance"), Some(5), "{tag}: {who}'s archived copy carries B's edit");
        }
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }
}

/// §6.1 (iv), D5 (d) (review E5): a tombstone names a note, not a place. B settled ITS note at P — a
/// different note from A's: other desktops' records place only B's id at P, never A's — so A's note
/// at P stays, named in one line.
#[test]
fn td1_iv_a_tombstone_for_a_path_holding_a_different_id_leaves_the_note() {
    let dir = fixture_with_id("td1-iv-tombstone"); // A's note: tasks/cs-100-hw-01.md, task_0000000001
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let theirs = foreign_create("task_00000000bb", "tasks/cs-100-hw-01.md", "2026-09-17T09:00:00.000Z", "DeskB");
    let report = sync::apply(&dir, &pulled(vec![theirs], vec![
        sync::PulledNote { device: "fedcba9876543210".into(), path: "tasks/cs-100-hw-01.md".into(), text: None },
    ]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert!(dir.join("tasks").join("cs-100-hw-01.md").exists(), "A's note stays: {report:?}");
    assert_eq!(report.moved, 0, "{report:?}");
    assert!(
        report.warnings.contains(&"sync: tasks/cs-100-hw-01.md — the account settled a different note there; this one stays".to_string()),
        "{:?}", report.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// §6.1 (iv), D5 (d)'s other half (the I3 case): a note created on B and deleted by hand on A — in
/// Explorer, so no `delete` record exists — is still archived on B, because no other desktop's record
/// places a different id at its path.
#[test]
fn td1_iv_a_note_deleted_by_hand_on_a_is_still_archived_on_b() {
    let (a, b) = (desk("iv-hand-a"), desk("iv-hand-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let rel = "tasks/cs-100-hw-07.md";
    fetch(&b, &mut jb, &[item("cs-100-hw-07", "CS 100 HW 07")]);
    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    deliver(&a, &page, &mut ja);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    deliver(&b, &page, &mut jb);
    std::fs::remove_file(a.join(rel)).expect("deleted by hand in Explorer");
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let r = deliver(&b, &page, &mut jb);
    assert_eq!(r.moved, 1, "{r:?}");
    assert!(!b.join(rel).exists() && b.join("archive").join("cs-100-hw-07.md").exists(), "archived on B");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// D5 (b), E5's move half: a foreign `move` acts on the note holding its id, wherever that is here —
/// and never on a different note sitting at the record's old path.
#[test]
fn td1_iv_a_foreign_move_acts_on_the_id_not_on_the_old_path() {
    let dir = fixture_with_id("td1-iv-move"); // task_0000000001 at tasks/cs-100-hw-01.md
    knowlu_engine::pystr::write_text(&dir.join("tasks").join("mine-for-m.md"), "---\ntitle: \"M\"\nid: task_000000000e\n---\n\nm\n")
        .expect("this desktop's copy of m, at a path of its own");
    let mut journal = Journal::new(&dir);
    let ctx = knowlu_engine::write::WriteContext::new(sync::ACTOR, "local-runner");
    let moved = |id: &str, from: &str, to: &str| {
        let mut spec = knowlu_engine::journal::NewRecord::new("move", from, "quinn", "dashboard");
        spec.id = Some(id);
        spec.old = serde_json::json!(from);
        spec.new = serde_json::json!(to);
        spec.ts = Some("2026-09-17T10:00:00.000Z".to_string());
        spec.device = Some("OtherDesktop".to_string());
        knowlu_engine::journal::make_record(spec).expect("a record")
    };
    let report = sync::apply(&dir, &pulled(vec![
        moved("task_000000000e", "tasks/theirs-for-m.md", "tasks/m-renamed.md"),
        moved("task_00000000cc", "tasks/cs-100-hw-01.md", "tasks/elsewhere.md"),
    ], vec![]), &ctx, &mut journal, "2026-09-17".parse().unwrap());
    assert!(dir.join("tasks/m-renamed.md").exists() && !dir.join("tasks/mine-for-m.md").exists(), "m moved by id: {report:?}");
    assert!(dir.join("tasks/cs-100-hw-01.md").exists() && !dir.join("tasks/elsewhere.md").exists(), "the other note did not move");
    assert_eq!(report.moved, 1, "{report:?}");
    assert!(
        report.warnings.contains(&"sync: tasks/cs-100-hw-01.md is task_0000000001, not task_00000000cc; the move to tasks/elsewhere.md is not applied here".to_string()),
        "{:?}", report.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract -- td1_iii td1_iv`
Expected: FAIL — (iii): B's copy stays live (A's `delete` record has no effect of its own yet, and A's
tombstone names a path B does not have); the tombstone test: A's note is archived; the move test: `m`
is not moved (nothing at the record's old path) and A's different note is. `td1_iv_a_note_deleted_by_hand…`
passes already — it is the regression guard for the half of D5 (d) that must not change.

- [ ] **Step 3: Two small helpers**

In `engine/src/sync.rs`, add to `impl IdIndex`:

```rust
    /// Every id placed at `rel` is forgotten — the seed pre-pass replaced that file wholesale.
    fn vacate(&mut self, rel: &str) {
        self.by_id.retain(|_, held| held != rel);
    }
```

and after `note_has_no_id`:

```rust
/// A note's valid `id:`, read without writing anything; `None` for a missing, unreadable or
/// unidentified note.
fn note_id_at(file: &Path) -> Option<String> {
    crate::ids::read_meta(file)
        .and_then(|meta| crate::yaml::get(&meta, "id").and_then(crate::yaml::text))
        .filter(|id| crate::ids::is_id(id))
}
```

- [ ] **Step 4: The record pass keeps each move's id, and collects deletes**

In `apply`'s record pass:
- `let mut moves: Vec<(String, String)> = Vec::new();` (`:1745`) becomes
  `let mut moves: Vec<(Option<String>, String, String)> = Vec::new();` and, beside it,
  `let mut deletes: Vec<String> = Vec::new();`.
- `pending_move = Some((path.to_string(), dest.to_string()));` (`:1788`) becomes
  `pending_move = Some((record.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()).map(str::to_string), path.to_string(), dest.to_string()));`
  (and `pending_move`'s type follows).
- After `report.records += 1;` (`:1794`) add:

```rust
        // D5 (b): a foreign `delete` settles the note holding its id (below, after the moves).
        if record.get("op").and_then(Value::as_str) == Some("delete") {
            if let Some(id) = record.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
                deletes.push(id.to_string());
            }
        }
```

- [ ] **Step 5: The index is built before the moves**

Move Task 5's `let mut index = IdIndex::build(vault);` (and its comment, re-worded to "read after the
record pass and kept current below") from before step 4 to immediately after the record pass's
`journal.invalidate();` (`:1812`). In the seed pre-pass, inside `Ok(()) => { … }` after a replacement
(`:1891-1895`), add:

```rust
                    // D5: the account's copy replaced the seed wholesale, so the seed's id is gone.
                    index.vacate(&note.path);
                    if let Some(id) = note_frontmatter_id(text).filter(|id| crate::ids::is_id(id)) {
                        index.place(&id, &note.path);
                    }
```

- [ ] **Step 6: Moves and deletes by id**

Replace step 3a's loop (`:1820-1839`, from `for (from, dest) in moves {` to its closing brace; the comment
above it kept and extended by the sentence `Two-desktop design D5 (b): a foreign move acts on the note
holding its id, wherever it is here — never on a different note at the record's old path (E5).`) with:

```rust
    for (id, from, dest) in moves {
        let holder = match &id {
            Some(id) => match index.holder(id) {
                Some(rel) => Some(rel.clone()),
                // R-TD1-2: a note at the old path with no `id:` line is identified by its path.
                None if note_has_no_id(&vault.join(&from)) => Some(from.clone()),
                None => {
                    if let Some(other) = note_id_at(&vault.join(&from)) {
                        report.warnings.push(format!(
                            "sync: {from} is {other}, not {id}; the move to {dest} is not applied here"
                        ));
                    }
                    None
                }
            },
            // R-TD1-15: a move with no id keeps today's rule — the note at its old path.
            None => vault.join(&from).exists().then(|| from.clone()),
        };
        // Nothing here to move, or it is already where it should be (a re-pull, or this device made
        // the same move itself).
        let Some(holder) = holder else { continue };
        if holder == dest {
            continue;
        }
        match crate::write::move_note(vault, &holder, &dest, ctx, journal) {
            Ok(_) => {
                report.moved += 1;
                if let Some(held) = note_id_at(&vault.join(&dest)) {
                    index.place(&held, &dest);
                }
            }
            // The destination is taken. Not a failure of the sync, and **not retried**: `Cursor`
            // carries no retry queue, so if the destination frees up later this device does not
            // notice. Pilot-acceptable and recorded as such (round-2 re-review, R4(b)): the record
            // is journalled either way so nothing is lost, the note stays at its old path rather
            // than overwriting whatever is there, and the warning names both paths. A retry queue is
            // a feature, not a one-line fix, and it needs two desktops independently choosing one
            // destination filename — a case this plan says has never been exercised even once.
            Err(crate::write::WriteError::Exists(_)) => {
                report.warnings.push(format!("sync: {holder} could not be renamed to {dest} — a note is already there"));
            }
            Err(e) => report.warnings.push(format!("sync: {holder} could not be renamed ({e})")),
        }
    }
    // D5 (b): a foreign `delete` settles the note holding its id — new: until now it had no effect of
    // its own, and only its tombstone, by path, settled anything — unless that note is already in
    // `archive/`. A delete with no id has no effect of its own (R-TD1-15); its tombstone carries it.
    for id in deletes {
        let Some(holder) = index.holder(&id).cloned() else { continue };
        if holder.starts_with("archive/") {
            continue;
        }
        match crate::write::delete(vault, &holder, ctx, journal) {
            Ok(dest) => {
                report.moved += 1;
                if let Some(held) = note_id_at(&dest) {
                    index.place(&held, &crate::ids::rel(vault, &dest));
                }
            }
            Err(e) => report.warnings.push(format!("sync: {holder} could not be settled ({e})")),
        }
    }
```

- [ ] **Step 7: A tombstone names a note, not a place**

Just before step 6's `let live_by_lower` (`:2184`) add:

```rust
    // D5 (d) (two-desktop design §2.5, R-TD1-13): which ids other desktops' records have placed at each
    // path, as alias groups — read only when this page carries a tombstone.
    let placed_elsewhere: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        if page.notes.iter().any(|n| n.text.is_none()) {
            let mut placed: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
                std::collections::BTreeMap::new();
            for r in journal.read(None, None) {
                let s = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                if s("device") == this_device || s("id").is_empty() {
                    continue;
                }
                let group = crate::ids::canonical(&index.aliases, &s("id")).to_string();
                placed.entry(s("path")).or_default().insert(group.clone());
                if s("op") == "move" && !s("new").is_empty() {
                    placed.entry(s("new")).or_default().insert(group);
                }
            }
            placed
        } else {
            std::collections::BTreeMap::new()
        };
```

and replace the tombstone arm's settle (`:2236-2241`, `if exact_case_exists(&file) { match … }`) with:

```rust
                if exact_case_exists(&file) {
                    // D5 (d): a tombstone names a note, not a place. When other desktops' records
                    // place a different id at this path and none of them ever placed this note's
                    // (alias groups compared), the tombstone is about another note and this one stays.
                    let ours = note_id_at(&file);
                    if let (Some(ours), Some(theirs)) = (&ours, placed_elsewhere.get(&note.path)) {
                        if !theirs.contains(crate::ids::canonical(&index.aliases, ours)) {
                            report.warnings.push(format!(
                                "sync: {} — the account settled a different note there; this one stays",
                                note.path
                            ));
                            continue;
                        }
                    }
                    match crate::write::delete(vault, &note.path, ctx, journal) {
                        Ok(dest) => {
                            report.moved += 1;
                            if let Some(ours) = &ours {
                                index.place(ours, &crate::ids::rel(vault, &dest));
                            }
                        }
                        Err(e) => report.warnings.push(format!("sync: {} could not be settled ({e})", note.path)),
                    }
                }
```

- [ ] **Step 8: Run the sync suites, then the whole suite**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract --test sync_replay`
Expected: PASS — the four new tests and every existing one: `a_valid_pulled_move_relocates_the_note_and_counts_it`
(its local note has no `id:`, R-TD1-2), `a_pulled_tombstone_settles_the_note_rather_than_unlinking_it`
(its local note has no id, and the fixture's one `TestPC` record has none either), the O1 tests (a
case-only rename's move still meets `Exists` at `move_note`, as before).

Then Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 9: Commit**

Message file `.superpowers\sdd\msg-task-6.txt`:

```
feat(engine): sync applies moves, deletes and tombstones by id (two desktops, D5 b/d, D7)

A foreign move acts on the note holding its id, never on a different
note at the record's old path; a foreign delete now settles the note
holding its id; a tombstone settles the note at its path unless other
desktops placed only a different id there. So a delete beats a concurrent
edit on both desktops, whatever the push order, with the edit kept on the
archived copy (review E5 and M5, both ruled In).

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-6.txt`

---

### Task 7: The alias pre-pass (spec §2.6; D6)

**Files:**
- Modify: `engine/src/sync.rs` — step 4's body extracted into `settle_resolution`; three small helpers
  (`took_no_effect`, `mtime_ts`, `under_id`) and `merge_into_winner`; in `apply`: the alias map after the
  index, the file settle after the seed pre-pass, step 4 grouped by winner, step 6's import-key join;
  `IdIndex::exact`/`forget` lose their `#[allow(dead_code)]`.
- Modify: `engine/src/passes.rs` — `load_index` (`:98-146`), `detect_external` (`:259-350`), and one test
  (R-TD1-18).
- Modify: `engine/tests/sync_contract.rs` — three helpers (`float_at`, `old_note`, `pulled_before`) and
  two tests.

**Interfaces:**
- Consumes: Task 4's `ids::import_ids_by_key`, `ids::aliases_from`, `ids::load_aliases`,
  `ids::save_aliases`, `ids::canonical`, `ids::alias_group`, `Journal::set_aliases`; Task 5–6's `IdIndex`.
- Produces (private to `sync.rs`):

```rust
#[allow(clippy::too_many_arguments)]
fn settle_resolution(vault: &Path, file: &Path, path: &str, meta: &serde_yaml_ng::Mapping,
    resolution: &crate::reconcile::Resolution, own_created: &std::collections::BTreeSet<String>,
    ctx: &crate::write::WriteContext, journal: &mut Journal, ledger: &crate::ledger::JsonlLedger,
    today: jiff::civil::Date, report: &mut ApplyReport);
fn took_no_effect(record: &Record, meta: &serde_yaml_ng::Mapping, this_device: &str) -> bool;
fn mtime_ts(file: &Path) -> String;
fn under_id(records: &[Record], id: &str) -> Vec<Record>;
#[allow(clippy::too_many_arguments)]
fn merge_into_winner(vault: &Path, winner: &str, winner_rel: &str, loser: &str,
    own_created: &std::collections::BTreeSet<String>, ctx: &crate::write::WriteContext,
    journal: &mut Journal, ledger: &crate::ledger::JsonlLedger, today: jiff::civil::Date,
    this_device: &str, report: &mut ApplyReport);
```

- [ ] **Step 1: Write the failing tests**

Append to `engine/tests/sync_contract.rs`:

```rust
fn float_at(vault: &Path, rel: &str, field: &str) -> Option<f64> {
    let meta = knowlu_engine::ids::read_meta(&vault.join(rel))?;
    knowlu_engine::yaml::get(&meta, field).and_then(knowlu_engine::yaml::f64_of)
}

/// x as a build from before this plan made it: a random id of its own, which `write::create` keeps
/// because the text carries it.
fn old_note(vault: &Path, journal: &mut Journal, rel: &str, id: &str) {
    let text = format!(
        "---\ntitle: \"CS 100 HW 07\"\ncourse: \"cs-100\"\ndue: 2026-10-09T23:59\neffort_hours: 2.0\nimportance: 3\nstatus: active\ncreated_by: zybooks\nsource_uid: \"zybooks:77\"\nid: {id}\n---\n\nbody\n"
    );
    let ctx = knowlu_engine::write::WriteContext::new("agent:coursework.zybooks", "local-runner");
    knowlu_engine::write::create(vault, rel, &text, &ctx, journal, None).expect("the old build's note");
}

/// The other desktop's `create` for `id`, appended to `vault`'s journal verbatim — as a pull made before
/// this plan appended it — under the other desktop's name.
fn pulled_before(vault: &Path, from: &Path, id: &str, device: &str) {
    let mut record = Journal::new(from)
        .read(None, None)
        .into_iter()
        .find(|r| r.get("op").and_then(|v| v.as_str()) == Some("create") && r.get("id").and_then(|v| v.as_str()) == Some(id))
        .expect("the create record");
    record.insert("device".to_string(), serde_json::json!(device));
    knowlu_engine::ledger::JsonlLedger::new(vault.join("state").join("journal")).append(&record).expect("appended verbatim");
}

/// §6.1 (v), D6 (Q9): a pair made and pulled BEFORE the upgrade — x under two random ids at one path, each
/// journal holding the other's `create`, neither text written (the path was held) — is joined at the
/// first sync after it: the lower id on both (B's copy re-identified), a field set only on the losing
/// copy ends equal on both, and a field set on both becomes one card. A lost alias file then costs one
/// more full pass and no write (re-review m2).
#[test]
fn td1_v_a_pair_made_before_the_upgrade_is_joined_on_the_lower_id_at_the_first_sync() {
    let (a, b) = (desk("v-a"), desk("v-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let rel = "tasks/cs-100-hw-07.md";
    let (winner, loser) = ("task_00000000a1", "task_00000000b2");
    old_note(&a, &mut ja, rel, winner);
    old_note(&b, &mut jb, rel, loser);
    pulled_before(&a, &b, loser, "DeskB");
    pulled_before(&b, &a, winner, "DeskA");
    ja.invalidate();
    jb.invalidate();
    // Both cursors past everything so far: the first sync after the upgrade carries only what follows.
    let _ = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    edit(&b, rel, &mut jb, &[("effort_hours", "4.0")]);
    edit(&a, rel, &mut ja, &[("importance", "5")]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    edit(&b, rel, &mut jb, &[("importance", "2")]);

    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let ra = deliver(&a, &page, &mut ja);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let rb = deliver(&b, &page, &mut jb);

    assert_eq!(meta_id(&a, rel), winner);
    assert_eq!(meta_id(&b, rel), winner, "B's copy took the lower id");
    assert!(
        rb.warnings.contains(&format!("sync: {rel} re-identified from {loser} to {winner}, the same item your other computer holds")),
        "{:?}", rb.warnings
    );
    assert_eq!(float_at(&a, rel, "effort_hours"), Some(4.0), "the loser-only field reached the winner");
    assert_eq!(float_at(&b, rel, "effort_hours"), Some(4.0));
    assert_eq!((ra.cards, rb.cards), (1, 0), "one card, for the field set on both: {ra:?} {rb:?}");
    for dir in [&a, &b] {
        assert_eq!(knowlu_engine::ids::load_aliases(dir).get(loser).map(String::as_str), Some(winner));
    }

    std::fs::remove_file(a.join(knowlu_engine::ids::ALIASES_FILE)).expect("the alias file");
    let before = knowlu_engine::pystr::read_text(&a.join(rel)).expect("the note");
    let again = deliver(&a, &pulled(vec![], vec![]), &mut ja);
    assert_eq!((again.applied, again.cards), (0, 0), "a lost alias file writes nothing: {again:?}");
    assert_eq!(knowlu_engine::pystr::read_text(&a.join(rel)).expect("the note"), before);
    assert!(a.join(knowlu_engine::ids::ALIASES_FILE).exists(), "rebuilt");
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// §6.1 (v) with both files on each side (re-review N1): before this plan `apply` wrote the other
/// desktop's text wherever its path was free, so each desktop holds both copies of x at two paths.
/// The first sync after the upgrade leaves one live file per desktop: the loser's fields reconciled
/// into the winner file, the loser archived and never re-identified, and no id on two files.
#[test]
fn td1_v_n1_a_desktop_holding_both_files_merges_the_loser_into_the_winner_and_archives_it() {
    let (a, b) = (desk("v1-a"), desk("v1-b"));
    let (mut ja, mut jb) = (Journal::new(&a), Journal::new(&b));
    let (mut ca, mut cb) = (Cursor::default(), Cursor::default());
    let (a_rel, b_rel) = ("tasks/cs-100-hw-07.md", "tasks/comp-100-hw-07.md");
    let (winner, loser) = ("task_00000000a1", "task_00000000b2");
    old_note(&a, &mut ja, a_rel, winner);
    old_note(&b, &mut jb, b_rel, loser);
    pulled_before(&a, &b, loser, "DeskB");
    pulled_before(&b, &a, winner, "DeskA");
    std::fs::copy(b.join(b_rel), a.join(b_rel)).expect("B's text, written on A before the upgrade");
    std::fs::copy(a.join(a_rel), b.join(a_rel)).expect("A's text, written on B before the upgrade");
    ja.invalidate();
    jb.invalidate();
    let _ = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let _ = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    edit(&b, b_rel, &mut jb, &[("effort_hours", "4.0")]);

    let page = transfer(&b, &mut cb, &mut jb, "DeskB", "DeskA");
    let ra = deliver(&a, &page, &mut ja);
    let page = transfer(&a, &mut ca, &mut ja, "DeskA", "DeskB");
    let rb = deliver(&b, &page, &mut jb);

    assert!(
        ra.warnings.contains(&format!("sync: {b_rel} is {loser}, the same item as {winner} at {a_rel}; merged into it and archived")),
        "{:?}", ra.warnings
    );
    assert_eq!((ra.cards, rb.cards), (0, 0), "{ra:?} {rb:?}");
    for (dir, who) in [(&a, "A"), (&b, "B")] {
        assert!(dir.join(a_rel).exists(), "{who}: the winner file stays live");
        assert!(!dir.join(b_rel).exists(), "{who}: the loser file is no longer live");
        assert_eq!(meta_id(dir, "archive/comp-100-hw-07.md"), loser, "{who}: archived, never re-identified");
        assert_eq!(float_at(dir, a_rel, "effort_hours"), Some(4.0), "{who}: the loser-only field reached the winner file");
        let mut seen = std::collections::BTreeSet::new();
        for (path, meta) in knowlu_engine::ids::scan_notes(dir) {
            if let Some(id) = meta.and_then(|m| knowlu_engine::yaml::get(&m, "id").and_then(knowlu_engine::yaml::text)) {
                assert!(seen.insert(id.clone()), "{who}: {id} is on two files ({})", path.display());
            }
        }
    }
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract -- td1_v`
Expected: FAIL — no alias exists: A and B keep two ids, B's `effort_hours` never reaches A (B's records
name an id A does not hold), and in the N1 test both files stay live on both desktops.

- [ ] **Step 3: Extract step 4's apply-and-card half, and three helpers**

In `engine/src/sync.rs`, move step 4's body from `// 5. **A carded field is withheld from \`apply\`…`
(`:1988`) to the end of the `if !resolved.is_empty() { … }` block (`:2158`) — the rest of the loop body —
into a new private function, word for word, with `path` and `file` taken as parameters:

```rust
/// Steps 5 and 4's card half, for one note `resolution` was computed for: write what it applies
/// (withholding a carded conflict — the card IS the write), append its supersede records, and settle
/// or file the one sync card. `path` is the local note's vault-relative path and `file` its full path.
/// Called by step 4 for every note a pull touches, and by the alias pre-pass to merge a loser file into
/// its winner (two-desktop design D6, R-TD1-9).
#[allow(clippy::too_many_arguments)]
fn settle_resolution(
    vault: &Path,
    file: &Path,
    path: &str,
    meta: &serde_yaml_ng::Mapping,
    resolution: &crate::reconcile::Resolution,
    own_created: &std::collections::BTreeSet<String>,
    ctx: &crate::write::WriteContext,
    journal: &mut Journal,
    ledger: &crate::ledger::JsonlLedger,
    today: jiff::civil::Date,
    report: &mut ApplyReport,
) {
    // `:1988-2158`, moved here verbatim, with the two edits named below.
}
```

The body is lines `:1988-2158` of `apply` exactly as they stand after Tasks 5 and 6, cut from the loop and
pasted into this function, with two mechanical edits and no others: `crate::ids::read_meta(&file)`
becomes `crate::ids::read_meta(file)`, and `&own_created` becomes `own_created`. Every `&path`/`path`
stays as written (a `&str` now; `&path` coerces), as do `&file` in the `propose_amendment` call, `ledger`,
`today`, `ctx`, `journal` and every `report.` field. Step 4's loop then ends with
`settle_resolution(vault, &file, &path, &meta, &resolution, &own_created, ctx, journal, &ledger, today, &mut report);`.
(Checked while writing this plan: the extraction compiles and every `sync_contract` test stays green.)

Replace the `never_took_effect` closure (`:1961-1968`) and the `mtime_ts` computation (`:1982-1985`) with
calls to two new functions, and add a third, all after `note_id_at`:

```rust
/// R1 (re-review round 3): a `set` another desktop made whose `new` is not what the note holds for that
/// field never took effect here, so it never decides a conflict here (the closure step 4 used to carry).
fn took_no_effect(record: &Record, meta: &serde_yaml_ng::Mapping, this_device: &str) -> bool {
    let field = record.get("field").and_then(Value::as_str).unwrap_or_default();
    record.get("op").and_then(Value::as_str) == Some("set")
        && !field.is_empty()
        && record.get("device").and_then(Value::as_str) != Some(this_device)
        && record.get("new").cloned().unwrap_or(Value::Null)
            != crate::yaml::get(meta, field).map(crate::yaml::to_json).unwrap_or(Value::Null)
}

/// The file's mtime as a journal `ts`, the synthetic contender `reconcile::resolve` uses for a value
/// nobody journalled.
fn mtime_ts(file: &Path) -> String {
    std::fs::metadata(file)
        .and_then(|m| m.modified())
        .map(|t| crate::journal::now_ts(jiff::Timestamp::try_from(t).ok()))
        .unwrap_or_else(|_| crate::journal::now_ts(None))
}

/// R-TD1-10: records of one alias group, under the group's winner, for `reconcile::resolve` only —
/// `journal::latest_by_field` keys on `(id, field)`, so two ids would pick "latest" by id, not by time.
/// In memory; the journal is never rewritten.
fn under_id(records: &[Record], id: &str) -> Vec<Record> {
    records
        .iter()
        .cloned()
        .map(|mut r| {
            r.insert("id".to_string(), Value::String(id.to_string()));
            r
        })
        .collect()
}
```

Run the sync suites (`--test sync_contract --test sync_replay`): every test except the two new ones
passes — the extraction changes nothing.

- [ ] **Step 4: Merge a loser file into its winner**

After `under_id`:

```rust
/// D6 (re-review N1, R-TD1-9), both files of a group here: the loser file's history is reconciled into
/// the winner file as though the loser's records were the other side — the winner's records (every
/// device) are the winner file's own history, the loser's (every device) are the other side, both
/// under the winner's id. A field set only on the losing copy, on any computer, reaches the winner; one
/// set on both becomes one card, exactly as step 4 applies and cards. The caller then settles the loser.
#[allow(clippy::too_many_arguments)]
fn merge_into_winner(
    vault: &Path,
    winner: &str,
    winner_rel: &str,
    loser: &str,
    own_created: &std::collections::BTreeSet<String>,
    ctx: &crate::write::WriteContext,
    journal: &mut Journal,
    ledger: &crate::ledger::JsonlLedger,
    today: jiff::civil::Date,
    this_device: &str,
    report: &mut ApplyReport,
) {
    let file = vault.join(winner_rel);
    let Some(meta) = crate::ids::read_meta(&file) else { return };
    let all = journal.read(None, None);
    let under = |r: &Record, id: &str| r.get("id").and_then(Value::as_str) == Some(id);
    let not_an_echo = |r: &Record| r.get("actor").and_then(Value::as_str) != Some(ACTOR);
    let upstream: Vec<Record> = all
        .iter()
        .filter(|r| under(r, winner) && not_an_echo(r) && !took_no_effect(r, &meta, this_device))
        .cloned()
        .collect();
    let theirs: Vec<Record> = all.iter().filter(|r| under(r, loser) && not_an_echo(r)).cloned().collect();
    let resolution = crate::reconcile::resolve(
        &meta,
        &under_id(&upstream, winner),
        &under_id(&theirs, winner),
        &mtime_ts(&file),
        Some(winner),
        winner_rel,
        ctx.via.as_str(),
    );
    settle_resolution(vault, &file, winner_rel, &meta, &resolution, own_created, ctx, journal, ledger, today, report);
}
```

- [ ] **Step 5: Build the alias map in `apply`**

Immediately after `let mut index = IdIndex::build(vault);` (placed by Task 6 after the record pass):

```rust
    // D6 (two-desktop design §2.6): one imported item under two or more ids. The groups come from every
    // `create` record in the journal (this page's included) and from a note only when its id has none,
    // so both desktops compute the same groups and winners with no coordination. The map is saved as
    // `ids::ALIASES_FILE` (generated, device-local, never synced) when it changes (R-TD1-11). It runs on
    // every sync, pulled rows or not, so doubles made before this shipped are found at the first sync
    // after the upgrade (Q9).
    let previous = crate::ids::load_aliases(vault);
    let keys = crate::ids::import_ids_by_key(&journal.read(None, None), &crate::ids::scan_notes(vault));
    let aliases = crate::ids::aliases_from(&keys);
    if aliases != previous {
        if let Err(e) = crate::ids::save_aliases(vault, &aliases) {
            report.warnings.push(format!("sync: the alias record could not be saved ({e}); the next sync rebuilds it"));
        }
    }
    // A group is new when one of its losers is not in the saved map: reconciled in full in step 4.
    let new_groups: std::collections::BTreeSet<String> = aliases
        .iter()
        .filter(|(loser, _)| !previous.contains_key(*loser))
        .map(|(_, winner)| winner.clone())
        .collect();
    journal.set_aliases(aliases.clone());
    index.aliases = aliases.clone();
```

- [ ] **Step 6: Settle the files of every group**

Immediately after the seed pre-pass (after `if !replaced_paths.is_empty() { … }`'s enclosing block,
`:1902`), before step 4:

```rust
    // D6 (re-review N1): then the files. Before this plan `apply` wrote another desktop's text wherever
    // its path was free, so this desktop can hold more than one file of a group.
    let winners: std::collections::BTreeSet<String> = aliases.values().cloned().collect();
    for winner in &winners {
        let group = crate::ids::alias_group(&aliases, winner);
        let losers: Vec<String> = group.into_iter().filter(|id| id != winner).collect();
        // Only a loser here: the lowest one takes the winner's id — the repair `ensure_ids` makes for a
        // duplicate — by one `write_literals` of `id` under `sync::ACTOR` (journalled, never sent).
        if index.exact(winner).is_none() {
            let Some((loser, rel)) = losers.iter().find_map(|l| index.exact(l).map(|r| (l.clone(), r.clone()))) else {
                continue;
            };
            match crate::write::write_literals(vault, &rel, &[("id".to_string(), winner.clone())], ctx, journal, &Default::default()) {
                Ok(_) => {
                    index.forget(&loser);
                    index.place(winner, &rel);
                    report.warnings.push(format!(
                        "sync: {rel} re-identified from {loser} to {winner}, the same item your other computer holds"
                    ));
                }
                Err(e) => {
                    report.warnings.push(format!("sync: {rel} could not be re-identified ({e})"));
                    continue;
                }
            }
        }
        // Both files here: every loser file still live is reconciled into the winner file and settled
        // through `write::delete` — never re-identified, which would leave one id on two files for
        // `ensure_ids` to split again at random. An archived loser is already settled.
        let Some(winner_rel) = index.exact(winner).cloned() else { continue };
        for loser in &losers {
            let Some(loser_rel) = index.exact(loser).cloned() else { continue };
            if loser_rel.starts_with("archive/") {
                continue;
            }
            merge_into_winner(vault, winner, &winner_rel, loser, &own_created, ctx, journal, &ledger, today, &this_device, &mut report);
            match crate::write::delete(vault, &loser_rel, ctx, journal) {
                Ok(dest) => {
                    report.moved += 1;
                    index.place(loser, &crate::ids::rel(vault, &dest));
                    report.warnings.push(format!(
                        "sync: {loser_rel} is {loser}, the same item as {winner} at {winner_rel}; merged into it and archived"
                    ));
                }
                Err(e) => report.warnings.push(format!("sync: {loser_rel} could not be settled ({e})")),
            }
        }
    }
```

(`this_device` and `own_created` are computed at `:1849-1858`, after the moves and before the seed
pre-pass — both are in scope here.) Remove the `#[allow(dead_code)]` from `IdIndex::exact` and
`IdIndex::forget`: both have callers now.

- [ ] **Step 7: Step 4 reconciles by group**

Immediately before step 4's `for (id, foreign) in &touched {`:

```rust
    // D6: foreign records are grouped under their group's winner, so a pull reconciles one note however
    // many ids the item had. A group found for the first time is reconciled in full, against every
    // other desktop's records for all of its ids — not only this page's (the full pass is idempotent:
    // converged fields file no card, re-review m2).
    let mut grouped: std::collections::BTreeMap<String, Vec<Record>> = std::collections::BTreeMap::new();
    for (id, records) in touched {
        grouped.entry(crate::ids::canonical(&aliases, &id).to_string()).or_default().extend(records);
    }
    if !new_groups.is_empty() {
        let all = journal.read(None, None);
        for winner in &new_groups {
            let group = crate::ids::alias_group(&aliases, winner);
            let everything: Vec<Record> = all
                .iter()
                .filter(|r| {
                    r.get("id").and_then(Value::as_str).is_some_and(|id| group.contains(id))
                        && r.get("device").and_then(Value::as_str) != Some(this_device.as_str())
                        && r.get("actor").and_then(Value::as_str) != Some(ACTOR)
                })
                .cloned()
                .collect();
            grouped.insert(winner.clone(), everything);
        }
    }
    let touched = grouped;
```

In the loop, `mine` is already `journal.records_for(id, None)` filtered (the journal now answers the whole
group). Replace the closure-based filter's `!never_took_effect(r)` with `!took_no_effect(r, &meta,
&this_device)`, and the `resolve` call with (R-TD1-10):

```rust
        let resolution = crate::reconcile::resolve(
            &meta,
            &under_id(&mine, id),
            &under_id(foreign, id),
            &mtime_ts(&file),
            Some(id),
            &path,
            ctx.via.as_str(),
        );
```

- [ ] **Step 8: Step 6 lets the alias join an item held under another id**

In step 6's live branch, immediately after Task 5's "text whose id is already held here" check:

```rust
                // D5 (c), D6: a text for an item held here under another id is joined by the alias,
                // never written as a second note — even before its `create` record has arrived.
                if let Some(id) = &text_id {
                    let joined = crate::models::split_frontmatter(text)
                        .ok()
                        .and_then(|(meta, _)| crate::ids::import_key(Path::new(&note.path), &meta))
                        .and_then(|key| keys.get(&key).cloned())
                        .and_then(|ids| ids.into_iter().filter(|held| held != id).find_map(|held| index.exact(&held).map(|local| (held, local.clone()))));
                    if let Some((held, local)) = joined {
                        report.warnings.push(format!(
                            "sync: {} is {id}, the same item as {held} held here as {local}; the two are joined",
                            note.path
                        ));
                        continue;
                    }
                }
```

- [ ] **Step 9: `detect_external` reads a group as one note**

`passes::detect_external` (`passes.rs:259-350`) keys its field index by record id (`load_index`,
`:98-146`). After the pre-pass re-identifies a note, its history sits under the old id, so the next
`rank` would read every field that history explains as a `quinn`/`external` edit — a record that
travels, settles the other desktop's card unanswered, and locks the field under judge-once. D6's "records
follow the alias" reaches it too.

Write the failing test first, in `engine/src/passes.rs::mod tests` after
`the_index_is_built_from_the_journal_and_kept_up_to_date`:

```rust
    /// Two-desktop design D6: a note the alias pre-pass re-identified keeps its history under the old
    /// id. `detect_external` reads the group as one note, under the winner, so it journals no
    /// `quinn`/`external` edit for a value that history explains — and an archived loser, already
    /// settled by the pre-pass, is not compared at all.
    #[test]
    fn detect_external_reads_an_alias_group_as_one_note() {
        let (vault, mut journal) = make_vault("alias-group");
        let loser = note_id(&vault, "tasks/a.md");
        let winner = "task_0000000001";
        // The other desktop's create for the winner, with an older importance, as a pull delivers it.
        let mut spec = NewRecord::new("create", "tasks/theirs.md", "agent:coursework.zybooks", "local-runner");
        spec.id = Some(winner);
        spec.new = serde_json::json!({"title": "A", "progress": 0, "importance": 3, "id": winner});
        spec.ts = Some("2026-09-01T00:00:00.000Z".to_string()); // before everything this test does here
        spec.device = Some("DeskA".to_string());
        journal.append(&mut make_record(spec).unwrap()).unwrap();
        // This desktop's own later edit under the loser id, then the pre-pass's re-identification.
        write(&vault, "tasks/a.md", &[("importance".into(), Yaml::from(5))], &ctx(), &mut journal, &WriteOpts::default()).unwrap();
        crate::write::write_literals(&vault, "tasks/a.md", &[("id".to_string(), winner.to_string())],
            &WriteContext::new(crate::sync::ACTOR, "local-runner"), &mut journal, &WriteOpts::default()).unwrap();
        // A second copy the pre-pass merged and archived under a third id of the group.
        crate::pystr::write_text(&vault.join("archive").join("copy.md"), "---\ntitle: A\nimportance: 1\nid: task_00000000c3\n---\n").unwrap();
        crate::ids::save_aliases(&vault, &BTreeMap::from([
            (loser.clone(), winner.to_string()),
            ("task_00000000c3".to_string(), winner.to_string()),
        ])).unwrap();
        let log = detect_external(&vault, &mut journal, &ctx());
        assert!(log.iter().all(|l| !l.starts_with("external")), "{log:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

(`scratch` makes only `tasks/`; add `std::fs::create_dir_all(vault.join("archive")).unwrap();` before
the `write_text` of `archive/copy.md`.) Run
`cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- passes::tests::detect_external_reads`
and see it FAIL: `external edit importance on tasks/a.md` and `external create archive/copy.md`.

Then, in `load_index`: before the cache read (`:103`), add
`let aliases = crate::ids::load_aliases(vault);` and make the cache read conditional —
`if path.exists() && aliases.is_empty() {` — with this comment above it:
`// Two-desktop design D6: with an alias group on file, the records of every id in a group are one note's
// history. The cache is keyed by id and cannot know that, so while a group exists it is rebuilt from the
// whole journal, under each group's winner (a vault with no import doubles never pays for this).`
And replace `apply(&mut index, &records);` (`:141`) with:

```rust
    let records: Vec<Record> = if aliases.is_empty() {
        records
    } else {
        records
            .into_iter()
            .map(|mut r| {
                if let Some(winner) = r.get("id").and_then(Value::as_str).and_then(|id| aliases.get(id)).cloned() {
                    r.insert("id".to_string(), Value::String(winner));
                }
                r
            })
            .collect()
    };
    apply(&mut index, &records);
```

In `detect_external`, after `let mut index = load_index(vault, journal);` (`:276`) add
`let aliases = crate::ids::load_aliases(vault);`, and after the `if !is_id(&note_id) { continue; }` block
(`:283-285`):

```rust
        // Two-desktop design D6: a loser id's note is a copy the alias pre-pass settled; its history is
        // the winner's, compared on the winner's own note.
        if aliases.contains_key(&note_id) {
            continue;
        }
```

Run the `passes::` tests: PASS, the new one included.

- [ ] **Step 10: Run the sync suites, then the whole suite**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract --test sync_replay`
Expected: PASS — both `td1_v…` tests and every existing one (a vault with no import doubles builds an
empty map, writes no alias file, and takes every path it took before).

Then Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 11: Commit**

Message file `.superpowers\sdd\msg-task-7.txt`:

```
feat(engine): the alias pre-pass joins one item held under two ids (two desktops, D6)

Every sync builds the alias map from the journal's create records and
saves it when it changes. A desktop holding only a loser re-identifies it
to the lowest id; one holding both files merges the loser's history into
the winner file and archives it. Foreign records are reconciled under the
group's winner, a newly found group in full, so doubles made before this
shipped are joined at the first sync after it. detect_external reads a
group as one note, so a re-identification fabricates no external edit.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/src/passes.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-7.txt`

---

### Task 8: Restore keeps one file per id (spec §2.5 (c), §6.1 (vi); D5)

**Files:**
- Modify: `engine/src/sync.rs` — `Touched` (`:1209-1283`: a new `undo`), `RestoreState` (`:1291-1303`: a
  new `written_ids`), `materialise`'s live loop (`:1025-1071`).
- Modify: `engine/tests/sync_contract.rs` — one test.

**Interfaces:**
- Consumes: `note_frontmatter_id` (`sync.rs:1608`), `restore_all` (`:1350`).
- Produces (private): `Touched::undo(&mut self, path: &Path)`; `RestoreState::written_ids:
  std::collections::BTreeMap<String, String>` (id → the path this restore wrote it at).

- [ ] **Step 1: Write the failing test**

Append to `engine/tests/sync_contract.rs`:

```rust
/// §6.1 (vi), D5 (c) for a restore: the account holds two text rows for one id — the rare backstop case
/// D4 accepts (one item at two paths). The restore writes one file, from the highest `rev` (the later
/// row: the pull is `rev`-ordered, E2's rule), and the first push after it tombstones neither row: the
/// earlier path never enters the cursor (R-TD1-12), or it would archive the other desktop's live copy.
#[test]
fn td1_vi_a_restore_given_two_rows_for_one_id_writes_one_file_from_the_highest_rev() {
    let dest = desk("vi-restore");
    let body = |title: &str| {
        format!("---\ntitle: \"{title}\"\ncreated_by: zybooks\nsource_uid: \"zybooks:77\"\nid: task_d0fd865fb3\n---\n\nbody\n")
    };
    let reply = pull_reply(vec![], vec![
        serde_json::json!({"rev": 5, "device": "0123456789abcdef", "path": "tasks/cs-100-hw-07.md", "deleted": false, "body": body("CS 100 HW 07")}),
        serde_json::json!({"rev": 9, "device": "fedcba9876543210", "path": "tasks/comp-100-hw-07.md", "deleted": false, "body": body("COMP 100 HW 07")}),
    ], 9);
    let mut server = loopback(vec![(200, reply)]);
    let cloud = cfg(&server.base);
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let restored = sync::restore_all(&dest, &client, &[]).expect("the restore");
    assert!(!dest.join("tasks").join("cs-100-hw-07.md").exists(), "the earlier row is not kept");
    assert!(
        knowlu_engine::pystr::read_text(&dest.join("tasks").join("comp-100-hw-07.md")).expect("the later row").contains("COMP 100 HW 07")
    );
    assert!(
        restored.warnings.contains(&"restore: tasks/cs-100-hw-07.md and tasks/comp-100-hw-07.md are both task_d0fd865fb3; the later row is kept".to_string()),
        "{:?}", restored.warnings
    );
    let cursor = sync::load_cursor(&dest);
    assert!(!cursor.notes.contains_key("tasks/cs-100-hw-07.md"), "{:?}", cursor.notes);
    let (batch, _) = sync::build_push(&dest, &cursor, "acct-1", &mut Journal::new(&dest));
    assert!(batch.notes.is_empty(), "no tombstone for either row: {:?}", batch.notes);
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dest);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract -- td1_vi`
Expected: FAIL — both rows are written, one id on two files.

- [ ] **Step 3: `Touched` can undo one path**

In `impl Touched`, after `note_dir` (`:1250-1255`):

```rust
    /// Two-desktop design D5 (c): put one path this call wrote back to its state before the call —
    /// removed if it did not exist, its own bytes if it did (a seed) — and forget it, so a later
    /// rollback does not touch it again.
    fn undo(&mut self, path: &Path) {
        let Some(at) = self.files.iter().position(|(p, _)| p == path) else { return };
        let (path, prior) = self.files.remove(at);
        match prior {
            Some(bytes) => {
                let _ = std::fs::write(&path, bytes);
            }
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
```

Add to `RestoreState` (`:1291-1303`), after `confirmed_records`:

```rust
    /// Two-desktop design D5 (c): the id of every live note this restore wrote, and where — so a later
    /// row for the same id replaces it rather than becoming a second file.
    written_ids: std::collections::BTreeMap<String, String>,
```

- [ ] **Step 4: `materialise` keeps one file per id**

In `materialise`'s live loop, after the foreign-sync-card refusal (`:1031-1038`) and before
`let file = dest.join(&note.path);` (`:1039`):

```rust
        // D5 (c) for a restore (two-desktop design §2.5): one id, one file. The pull is `rev`-ordered,
        // so a later row for an id this restore already wrote at another path has the higher `rev` and
        // wins, E2's rule. The earlier path goes back to what it held before the restore and leaves the
        // cursor, so the first push never tombstones the other desktop's row for it (R-TD1-12).
        let text_id = note_frontmatter_id(text).filter(|id| crate::ids::is_id(id));
        if let Some(id) = &text_id {
            if let Some(earlier) = state.written_ids.get(id).cloned() {
                if earlier != note.path {
                    state.touched.undo(&dest.join(&earlier));
                    state.written_notes.remove(&earlier);
                    out.warnings.push(format!(
                        "restore: {earlier} and {} are both {id}; the later row is kept",
                        note.path
                    ));
                }
            }
        }
```

and in the write's `Ok(()) => { … }` arm (`:1055-1068`), after `state.pending_tombstones.retain(…)`:

```rust
                if let Some(id) = &text_id {
                    state.written_ids.insert(id.clone(), note.path.clone());
                }
```

(`Restored::notes` keeps counting writes performed; the undone one is not subtracted — nothing reads the
count for anything but a sentence, and `empty` is unaffected because the later row was written.)

- [ ] **Step 5: Run the sync suites, then the whole suite**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract --test sync_replay`
Expected: PASS — `td1_vi…` and every restore test in `sync_replay.rs` (their fixtures carry one path per
id, and a seed overwritten by the account's copy at its own path is unchanged).

Then Global Constraint 19. Expected: PASS at 0 warnings.

- [ ] **Step 6: Commit**

Message file `.superpowers\sdd\msg-task-8.txt`:

```
feat(engine): a restore keeps one file per id (two desktops, D5 c)

When the account holds two text rows for one id - one item at two paths,
the backstop case D4 accepts - a restore writes one file, from the later
row, and puts the earlier path back as it was before the restore. The
earlier path never enters the cursor, so the first push sends no
tombstone that would archive the other desktop's live copy.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/sync.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-8.txt`

---

### Task 9: Event proposals across desktops — any digest proposes; `rank --no-digest` (spec §2.8; D8)

**Files:**
- Modify: `engine/src/eventemit.rs` — the module doc (`:16`), a new `proposed_digest_uids` after
  `pending_digest_uids` (`:72-115`), `emit_digest`'s read (`:253`), and `mod tests` (one test renamed,
  one added).
- Create: `engine/tests/rank_no_digest.rs`.
- Hand-offs: **H2** (`engine/src/cli.rs`) and **H3** (`engine/src/main.rs`), applied by the controller at
  Step 6.

**Interfaces:**
- Consumes: Task 3's `emit_digest` (it creates through `create_imported`).
- Produces: `pub fn proposed_digest_uids(vault: &Path) -> BTreeSet<String>`; through H2,
  `cli::RankOptions`, `cli::run_opts`, `cli::run_with_opts`; through H3, `knowlu-engine rank --no-digest`.

- [ ] **Step 1: Write the failing `eventemit` test, and rename one**

In `engine/src/eventemit.rs::mod tests`, rename `a_uid_in_a_settled_digest_is_still_eligible` (`:623-630`)
to `a_uid_in_a_settled_digest_is_not_pending`, and replace its two comment lines with:

```rust
        // An executed/rejected/expired digest is not *pending*: `pending_digest_uids` names live digests
        // only. Whether its uids may be proposed again is `proposed_digest_uids`'s question (two-desktop
        // design D8), and the answer is no — see the next test.
```

(its body is unchanged). After it, add:

```rust
    /// Two-desktop design D8: proposed means named in ANY digest note, in `approvals/` or `archive/`,
    /// whatever its status. A settled digest keeps its `events:` payload in `archive/`, and that note
    /// syncs, so a desktop whose own ledger never saw the proposal — it never held `feeds` — still never
    /// proposes the event again.
    #[test]
    fn a_uid_in_any_digest_note_settled_or_archived_is_already_proposed() {
        let vault = tmp("proposed-anywhere");
        write_digest(&vault, "a", "events-digest-2026-08-18.md", "executed");
        fs::create_dir_all(vault.join("archive")).unwrap();
        fs::rename(
            vault.join("approvals").join("events-digest-2026-08-18.md"),
            vault.join("archive").join("events-digest-2026-08-18.md"),
        )
        .unwrap();
        write_digest(&vault, "b", "events-digest-2026-08-19.md", "rejected");
        assert_eq!(proposed_digest_uids(&vault), BTreeSet::from(["a".to_string(), "b".to_string()]));
        // This desktop's ledger carries both verdicts and no `proposed` line.
        let events = [event("a", 25), event("b", 25)];
        let ledger = ledger_of(vec![opp("a", "mild"), opp("b", "mild")]);
        let (path, count) = emit(&vault, &events, &ledger, None);
        assert_eq!((path, count), (None, 0), "neither is proposed a second time");
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --lib -- eventemit::`
Expected: FAIL to compile — `proposed_digest_uids` does not exist.

- [ ] **Step 3: `proposed_digest_uids`, and `emit_digest` reads it**

In `engine/src/eventemit.rs`, after `pending_digest_uids` (`:72-115`):

```rust
/// Two-desktop design D8: every uid named in any digest note — in `approvals/` or `archive/`, whatever
/// its status. A settled digest keeps its `events:` payload in `archive/`, and that note syncs, so a
/// desktop whose own ledger never recorded the proposal (it never held `feeds`) still never proposes the
/// event again. `emit_digest` reads this; [`pending_digest_uids`] stays the live-only reading, unchanged
/// (R-TD1-7). Defensive throughout, as it is: a malformed digest never fails the run.
pub fn proposed_digest_uids(vault: &Path) -> BTreeSet<String> {
    let mut uids = BTreeSet::new();
    for folder in ["approvals", "archive"] {
        let Ok(entries) = std::fs::read_dir(vault.join(folder)) else { continue };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("events-digest-") && n.ends_with(".md"))
            })
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = split_frontmatter(&text) else { continue };
            let Some(serde_yaml_ng::Value::Sequence(payload)) = crate::yaml::get(&meta, "events") else {
                continue;
            };
            for entry in payload {
                let Some(map) = entry.as_mapping() else { continue };
                let uid = crate::yaml::opt_text(crate::yaml::get(map, "uid")).unwrap_or_default();
                let uid = pystr::strip(&uid);
                if !uid.is_empty() {
                    uids.insert(uid.to_string());
                }
            }
        }
    }
    uids
}
```

In `emit_digest`, `let pending = pending_digest_uids(vault);` (`:253`) becomes:

```rust
    // Two-desktop design D8: named in any digest note, anywhere, is already proposed.
    let pending = proposed_digest_uids(vault);
```

and the module doc's line 16 gains, after "reads the payload back to close that window.":
`Since the two-desktop design's D8, [\`proposed_digest_uids\`] (every digest, any status, \`approvals/\`
and \`archive/\`) is what \`emit_digest\` reads.`

Run the `eventemit::` filter again. Expected: PASS — the new test, the renamed one, and
`a_uid_in_a_live_digest_is_not_eligible` (a live digest is in `approvals/`, so it is proposed too).

The `mod tests` doc line `//! Direct port of \`tests/test_event_emission.py\` — all 24 tests, same names.`
(`:337`) gains ` One is renamed by the two-desktop design's D8: \`a_uid_in_a_settled_digest_is_still_eligible\`
is now \`a_uid_in_a_settled_digest_is_not_pending\`.`

- [ ] **Step 4: Write the failing `rank --no-digest` tests**

Create `engine/tests/rank_no_digest.rs`:

```rust
//! Two-desktop design D8 from outside the crate: `rank --no-digest` (hand-offs H2 and H3) writes the
//! roster and *Coming up* but files no events digest — for a computer that did not hold the `feeds`
//! turn this slot. The default `rank` is unchanged, which is what keeps `oracle.rs` and every frozen
//! reference untouched.

use std::path::{Path, PathBuf};
use std::process::Command;

use knowlu_engine::childproc::NoConsole;
use knowlu_engine::cli::{self, Fetchers, RankOptions};

/// `cli.rs`'s own test scaffold: a flat week template and one enabled event source.
fn scaffold(name: &str) -> PathBuf {
    let vault = std::env::temp_dir().join(format!("knowlu-nodigest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&vault);
    std::fs::create_dir_all(vault.join("config")).expect("mkdir");
    std::fs::create_dir_all(vault.join("tasks")).expect("mkdir");
    knowlu_engine::pystr::write_text(
        &vault.join("config").join("week_template.yaml"),
        "day_start: '09:00'\nday_end: '17:00'\nclasses:\n  mon: []\n  tue: []\n  wed: []\n  thu: []\n  fri: []\n  sat: []\n  sun: []\n",
    )
    .expect("week template");
    knowlu_engine::pystr::write_text(
        &vault.join("config").join("events.yaml"),
        "sources:\n  - name: blount\n    type: ics\n    url: unreachable://x\n    enabled: true\n",
    )
    .expect("events config");
    vault
}

/// One event on 2026-08-27 at 18:00, judged an opportunity in this desktop's own ledger.
fn judged_feed(vault: &Path) -> String {
    knowlu_engine::eventledger::record_verdict(
        vault, "ics:ev-0", "Event 0", jiff::civil::date(2026, 8, 26), "opportunity", "", "", "",
    )
    .expect("a verdict line");
    "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;TZID=America/Chicago:20260827T180000\n\
     DTEND;TZID=America/Chicago:20260827T183000\nUID:ev-0\nSUMMARY:Event 0\nLOCATION:Union\n\
     END:VEVENT\nEND:VCALENDAR\n"
        .to_string()
}

fn digests(vault: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(vault.join("approvals"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| n.starts_with("events-digest-"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

#[test]
fn rank_with_no_digest_writes_the_roster_and_files_no_digest() {
    let vault = scaffold("off");
    let ics = judged_feed(&vault);
    let fetch = |_: &str| Ok(ics.clone());
    cli::run_with_opts(
        &vault, Some("2026-08-26"), "manual", None,
        Fetchers { calendar: None, events: Some(&fetch) },
        RankOptions { no_digest: true },
    )
    .expect("rank");
    assert!(digests(&vault).is_empty(), "no digest on a computer that did not hold feeds");
    let roster = knowlu_engine::pystr::read_text(&vault.join("state").join("events.md")).expect("the roster is written");
    assert!(roster.contains("ics:ev-0"), "{roster}");
    let ledger = knowlu_engine::pystr::read_text(&vault.join("state").join("events-seen.md")).expect("the ledger");
    assert!(!ledger.contains("· proposed"), "nothing is marked proposed: {ledger}");
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn rank_by_default_still_files_the_digest() {
    let vault = scaffold("on");
    let ics = judged_feed(&vault);
    let fetch = |_: &str| Ok(ics.clone());
    cli::run_with(&vault, Some("2026-08-26"), "manual", None, Fetchers { calendar: None, events: Some(&fetch) })
        .expect("rank");
    assert_eq!(digests(&vault), vec!["events-digest-2026-08-26.md".to_string()]);
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn the_binary_takes_no_digest_and_refuses_a_misspelling() {
    let vault = scaffold("flag");
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_knowlu-engine"));
    let path = vault.to_str().expect("a UTF-8 temp path");
    let ok = Command::new(&binary)
        .no_console()
        .args(["rank", "--vault", path, "--today", "2026-08-26", "--no-digest"])
        .output()
        .expect("run knowlu-engine");
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    let bad = Command::new(&binary)
        .no_console()
        .args(["rank", "--vault", path, "--today", "2026-08-26", "--no-dijest"])
        .output()
        .expect("run knowlu-engine");
    assert_eq!(bad.status.code(), Some(2), "clap refuses a flag it does not know");
    let _ = std::fs::remove_dir_all(&vault);
}
```

- [ ] **Step 5: Run them to see them fail**

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test rank_no_digest`
Expected: FAIL to compile — `cli::RankOptions` and `cli::run_with_opts` do not exist.

- [ ] **Step 6: Hand-offs H2 and H3 are applied**

The controller applies **H2** (`engine/src/cli.rs`) and **H3** (`engine/src/main.rs`) verbatim, as one
commit, before the next run. An implementer never edits either file.

- [ ] **Step 7: Run them to see them pass, then the whole suite**

Run the `--test rank_no_digest` command again. Expected: PASS, all three. Then
`cargo test … -p knowlu-engine -j 2 --test oracle --test surface_oracle --test cloud_contract`: PASS —
`rank`'s default is unchanged, and `cli.rs` still names no judgment endpoint. Then Global Constraint 19.
Expected: PASS at 0 warnings.

- [ ] **Step 8: Commit**

Message file `.superpowers\sdd\msg-task-9.txt`:

```
feat(engine): event proposals hold across desktops (two desktops, D8)

A uid named in any events digest, in approvals/ or archive/, whatever its
status, is already proposed: a settled digest keeps its payload and syncs,
so a desktop whose own ledger never saw the proposal never proposes the
event again. rank --no-digest (hand-offs H2, H3) writes the roster and
Coming up but files no digest, for a computer that did not hold feeds; the
default rank is unchanged.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add engine/src/eventemit.rs engine/tests/rank_no_digest.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-9.txt`

---

### Task 10: The event-verdict pull (spec §2.8; D8; §6.3's `judge-event` GET)

**Files:**
- Create: `cloud/supabase/functions/judge-event/handler.ts`, `cloud/supabase/functions/judge-event/handler_test.ts`.
- Modify: `cloud/supabase/functions/judge-event/index.ts` (the whole file, 10 lines).
- Modify: `engine/src/sync.rs` — `Cursor` (`:213-233`: a new field), `Totals` (`:2347-2361`: a new
  field), `run_lines_with` (`:2520-2521`), `run_lines_with_client` (`:2588-2592`), and a new
  `pull_event_verdicts` with a private `query_value`, after `pull` (`:886-890`).
- Modify: `engine/tests/sync_contract.rs` — five tests.
- Hand-off: **H4** (`CLAUDE.md`), applied by the controller at Step 9.

**Interfaces:**
- Consumes: `_shared/judge_handler.ts::judgeHandler` and `type Entitle`; `_shared/judge_db.ts::Db`;
  `_shared/judge_deps.ts::liveDeps`, `sharedDb`; `eventledger::{load_ledger, record_verdict,
  VALID_VERDICTS}`; `eventroster::read_roster`; `cli::local_now`.
- Produces: `GET /judge-event` (contract in *Interfaces this plan produces*), `export const VERDICT_PAGE = 500`,
  `export function parseAfter`, `export async function readVerdicts`, `export function judgeEventHandler(entitle, deps, db)`;
  `sync::Cursor::verdicts_after`, `sync::Totals::offline`, `sync::pull_event_verdicts`.

**No migration** (R-TD1-14): `judgments` already has RLS on with no policy (C2's rule: the service role
is the only reader, and every query names the account — `_shared/judge_db_test.ts`'s scan walks every
function directory and holds the new selects to it), nothing is created, so nothing is revoked, and the
`judgments_account_day` index `(account_id, kind, judged_at desc)` serves the read. `migrations_test.ts`
does not change: the function pin stays 28 and the view pin 5.

- [ ] **Step 1: Write the failing Deno tests**

Create `cloud/supabase/functions/judge-event/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import type { Db } from "../_shared/judge_db.ts";
import type { PipelineDeps } from "../_shared/judge_pipeline.ts";
import { judgeEventHandler, parseAfter, VERDICT_PAGE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const NO_PIPELINE = (): Promise<PipelineDeps> => Promise.reject(new Error("a GET never reaches the pipeline"));
const T = "2026-09-25T12:00:00.123456+00:00";
const U1 = "0f0e0d0c-0b0a-4908-8706-050403020101";
const U2 = "0f0e0d0c-0b0a-4908-8706-050403020102";
const U3 = "0f0e0d0c-0b0a-4908-8706-050403020103";
const COLUMNS = "select=item_id,fields,judged_at,id";

/** A `Db` that answers each `select` with the next scripted page and records every path it was asked. */
function fakeDb(pages: unknown[][]): { db: () => Db; paths: string[] } {
  const paths: string[] = [];
  const db: Db = {
    select(path) {
      paths.push(path);
      return Promise.resolve(pages.shift() ?? []);
    },
    insert() {
      return Promise.reject(new Error("a verdict read never inserts"));
    },
    update() {
      return Promise.reject(new Error("a verdict read never updates"));
    },
    rpc() {
      return Promise.reject(new Error("a verdict read never calls a function"));
    },
  };
  return { db: () => db, paths };
}

/** A `judgments` row as PostgREST returns the selected columns: `fields` holds the verdict word and the
 * promotion features, never a reason (`fieldsOf`, `_shared/judge_pipeline.ts`). */
const row = (item: string, verdict: string | null, id: string) => ({
  item_id: item,
  fields: verdict === null ? { source: "campus" } : { verdict, source: "campus", title_prefix: "Career Fair" },
  judged_at: T,
  id,
});

function get(query: string): Request {
  return new Request(`http://127.0.0.1/judge-event${query}`, { headers: { authorization: "Bearer t" } });
}

Deno.test("from the start: one select of this account's answered event rows, in (judged_at, id) order", async () => {
  const { db, paths } = fakeDb([[row("ics:fair", "opportunity", U1)]]);
  const res = await judgeEventHandler(OK, NO_PIPELINE, db)(get(""));
  assertEquals(res.status, 200);
  assertEquals(paths, [
    `judgments?account_id=eq.acct-1&kind=eq.event&outcome=eq.answered&${COLUMNS}&order=judged_at.asc,id.asc&limit=${VERDICT_PAGE}`,
  ]);
  assertEquals(await res.json(), { verdicts: [{ item_id: "ics:fair", verdict: "opportunity", judged_at: T, id: U1 }], more: false });
});

Deno.test("after a cursor: the rest of its judged_at first, then what follows, so a tie is never skipped", async () => {
  const { db, paths } = fakeDb([[row("ics:b", "drop", U2)], [row("ics:c", "obligation", U3)]]);
  const res = await judgeEventHandler(OK, NO_PIPELINE, db)(get(`?after=${encodeURIComponent(T)},${U1}`));
  const at = encodeURIComponent(T);
  assertEquals(paths, [
    `judgments?account_id=eq.acct-1&kind=eq.event&outcome=eq.answered&judged_at=eq.${at}&id=gt.${U1}&${COLUMNS}&order=id.asc&limit=${VERDICT_PAGE}`,
    `judgments?account_id=eq.acct-1&kind=eq.event&outcome=eq.answered&judged_at=gt.${at}&${COLUMNS}&order=judged_at.asc,id.asc&limit=${VERDICT_PAGE - 1}`,
  ]);
  const body = await res.json();
  assertEquals(body.verdicts.map((v: { item_id: string }) => v.item_id), ["ics:b", "ics:c"]);
});

Deno.test("a row carries the verdict word and three keys more, never a reason or a feature", async () => {
  const { db } = fakeDb([[row("ics:fair", "opportunity", U1), row("ics:odd", null, U2)]]);
  const body = await (await judgeEventHandler(OK, NO_PIPELINE, db)(get(""))).json();
  for (const v of body.verdicts) assertEquals(Object.keys(v).sort(), ["id", "item_id", "judged_at", "verdict"]);
  assertEquals(body.verdicts[1].verdict, null, "no word: the device skips it, and its cursor still passes it");
});

Deno.test("a full page says there is more", async () => {
  const { db } = fakeDb([Array.from({ length: VERDICT_PAGE }, (_, i) => row(`ics:${i}`, "drop", U1))]);
  assertEquals((await (await judgeEventHandler(OK, NO_PIPELINE, db)(get(""))).json()).more, true);
});

Deno.test("the account comes from the entitlement alone, never from the query", async () => {
  const { db, paths } = fakeDb([[]]);
  await judgeEventHandler(OK, NO_PIPELINE, db)(get("?account_id=someone-else"));
  assert(paths.every((p) => p.startsWith("judgments?account_id=eq.acct-1&") && !p.includes("someone-else")), paths.join("\n"));
});

Deno.test("a malformed cursor is a 400 and reads nothing", async () => {
  for (const q of ["?after=yesterday", `?after=${encodeURIComponent(T)}`, `?after=${encodeURIComponent(T)},not-a-uuid`, `?after=,${U1}`]) {
    const { db, paths } = fakeDb([]);
    const res = await judgeEventHandler(OK, NO_PIPELINE, db)(get(q));
    assertEquals(res.status, 400, q);
    assertEquals(await res.json(), { error: "after must be <judged_at>,<id>" });
    assertEquals(paths, [], q);
  }
});

Deno.test("an entitlement refusal passes through as its own status", async () => {
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const { db, paths } = fakeDb([]);
  assertEquals((await judgeEventHandler(refuse, NO_PIPELINE, db)(get(""))).status, 402);
  assertEquals(paths, []);
});

Deno.test("a POST is the judgment exactly as before; any other method is 405", async () => {
  const { db, paths } = fakeDb([]);
  const handler = judgeEventHandler(OK, NO_PIPELINE, db);
  const post = new Request("http://127.0.0.1/judge-event", {
    method: "POST",
    headers: { authorization: "Bearer t" },
    body: JSON.stringify({ kind: "task", item: {} }),
  });
  const res = await handler(post);
  assertEquals(res.status, 400);
  assertEquals((await res.json()).error, "expected kind 'event' and an item object", "judgeHandler(\"event\") answered");
  assertEquals((await handler(new Request("http://127.0.0.1/judge-event", { method: "PUT" }))).status, 405);
  assertEquals(paths, [], "neither reads a verdict");
});

Deno.test("parseAfter splits at the last comma", () => {
  assertEquals(parseAfter(null), null);
  assertEquals(parseAfter(""), null);
  assertEquals(parseAfter(`${T},${U1}`), { judged_at: T, id: U1 });
  assertEquals(parseAfter("x"), "bad");
});
```

- [ ] **Step 2: Run them to see them fail**

Run (PATH refreshed, Global Constraint 13):
`deno test --allow-read --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\`
Expected: FAIL — `Module not found "…/judge-event/handler.ts"`.

- [ ] **Step 3: The GET route**

Create `cloud/supabase/functions/judge-event/handler.ts`:

```ts
// cloud/supabase/functions/judge-event/handler.ts
//
// Two routes on one function. `POST` is the judgment, unchanged: the shared `judgeHandler`, origin
// "events". `GET` is new (two-desktop design D8, §2.8): the account's own `answered` event verdicts
// after a cursor — the verdict WORD only; `judgments.fields` keeps no reason by design (`fieldsOf`,
// `_shared/judge_pipeline.ts`) — so every desktop's `sync` can fill its own event ledger, and a desktop
// that newly holds the `feeds` turn judges only what no desktop has judged. `judgments` has RLS on and no
// policy: the service role is its one reader, and every select below names the account, which
// `_shared/judge_db_test.ts`'s scan enforces.
import { type Entitle, judgeHandler } from "../_shared/judge_handler.ts";
import type { Db } from "../_shared/judge_db.ts";
import type { Kind, PipelineDeps } from "../_shared/judge_pipeline.ts";

/** One page — the number `sync-pull` uses, under PostgREST's `max_rows` (1000). */
export const VERDICT_PAGE = 500;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Where a device's last page ended: the `judged_at` and `id` of the last row it took. */
export interface Cursor {
  judged_at: string;
  id: string;
}

/** `after=<judged_at>,<id>`, split at the last comma. Absent or empty is "from the start" (`null`);
 * anything else that does not read is `"bad"`. */
export function parseAfter(raw: string | null): Cursor | null | "bad" {
  if (raw === null || raw === "") return null;
  const cut = raw.lastIndexOf(",");
  if (cut < 1) return "bad";
  const judged_at = raw.slice(0, cut);
  const id = raw.slice(cut + 1);
  if (Number.isNaN(Date.parse(judged_at)) || !UUID.test(id)) return "bad";
  return { judged_at, id };
}

interface Row {
  item_id: unknown;
  fields: unknown;
  judged_at: unknown;
  id: unknown;
}

/**
 * The page after `after`, in `(judged_at, id)` order: first the rows at the cursor's own `judged_at` with
 * a greater `id` — a tie that straddled the last page is never skipped — then the rows after it. Two
 * plain selects rather than one `or=` filter, so a timestamp never meets PostgREST's logic-tree quoting.
 */
export async function readVerdicts(db: Db, accountId: string, after: Cursor | null): Promise<Row[]> {
  const rows: Row[] = [];
  if (after !== null) {
    const at = encodeURIComponent(after.judged_at);
    const tie = await db.select(
      `judgments?account_id=eq.${accountId}&kind=eq.event&outcome=eq.answered&judged_at=eq.${at}&id=gt.${after.id}&select=item_id,fields,judged_at,id&order=id.asc&limit=${VERDICT_PAGE}`,
    ) as Row[];
    rows.push(...tie);
  }
  const left = VERDICT_PAGE - rows.length;
  if (left > 0) {
    const since = after === null ? "" : `&judged_at=gt.${encodeURIComponent(after.judged_at)}`;
    const later = await db.select(
      `judgments?account_id=eq.${accountId}&kind=eq.event&outcome=eq.answered${since}&select=item_id,fields,judged_at,id&order=judged_at.asc,id.asc&limit=${left}`,
    ) as Row[];
    rows.push(...later);
  }
  return rows;
}

/** The verdict word `fields` carries, or `null`. */
function verdictOf(fields: unknown): string | null {
  if (typeof fields !== "object" || fields === null) return null;
  const verdict = (fields as Record<string, unknown>).verdict;
  return typeof verdict === "string" && verdict !== "" ? verdict : null;
}

export function judgeEventHandler(
  entitle: Entitle,
  deps: (kind: Kind) => Promise<PipelineDeps>,
  db: () => Db,
): (req: Request) => Promise<Response> {
  const judge = judgeHandler("event", entitle, deps);
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return await judge(req);
    try {
      const { account_id } = await entitle(req);
      const after = parseAfter(new URL(req.url).searchParams.get("after"));
      if (after === "bad") {
        return Response.json({ error: "after must be <judged_at>,<id>" }, { status: 400 });
      }
      const rows = await readVerdicts(db(), account_id, after);
      return Response.json({
        verdicts: rows.map((r) => ({ item_id: r.item_id, verdict: verdictOf(r.fields), judged_at: r.judged_at, id: r.id })),
        more: rows.length >= VERDICT_PAGE,
      });
    } catch (e) {
      // `requireActiveEntitlement` throws a Response (401 / 402); everything else is ours. The class,
      // never the message, reaches the log (§5.6 of the cloud design).
      if (e instanceof Response) return e;
      console.error(`judge-event verdicts: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "verdicts could not be read" }, { status: 500 });
    }
  };
}
```

Replace `cloud/supabase/functions/judge-event/index.ts` whole with:

```ts
// cloud/supabase/functions/judge-event/index.ts
//
// `origin: "events"`, so a judgment made for the events pass is distinguishable in `judgments`
// from one made for a task — which is what the eval's per-kind metrics and the correction-rate
// dashboards key on. `GET` reads the account's event verdicts back for every desktop's sync
// (two-desktop design D8; `handler.ts`).
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps, sharedDb } from "../_shared/judge_deps.ts";
import { judgeEventHandler } from "./handler.ts";

Deno.serve(judgeEventHandler(requireActiveEntitlement, (kind) => liveDeps(kind, "events"), sharedDb));
```

(`config.toml`'s `[functions.judge-event]` entry, `:130`, stands; nothing else deploys.)

- [ ] **Step 4: Run the Deno checks**

Run, each from any directory with absolute paths (PATH refreshed):
`deno test --allow-read --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\ C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\_shared\judge_db_test.ts C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\_shared\judge_handler_test.ts`
Expected: PASS — the nine new tests, the account-scoping scan (it now walks `judge-event/handler.ts` and
finds both selects naming the account), and the shared handler's own tests.

`deno check --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\index.ts C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\handler.ts C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\handler_test.ts`
and `deno lint --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\functions\judge-event\`
Expected: both clean.

- [ ] **Step 5: Write the failing device tests**

Append to `engine/tests/sync_contract.rs`:

```rust
const JUDGED_AT: &str = "2026-09-25T12:00:00.123456+00:00";
const ROW_1: &str = "0f0e0d0c-0b0a-4908-8706-050403020101";
const ROW_2: &str = "0f0e0d0c-0b0a-4908-8706-050403020102";
const ROW_3: &str = "0f0e0d0c-0b0a-4908-8706-050403020103";

fn verdicts_reply(rows: serde_json::Value, more: bool) -> (u16, String) {
    (200, knowlu_engine::ledger::dumps_value(&serde_json::json!({ "verdicts": rows, "more": more })))
}

fn ledger_text(dir: &Path) -> String {
    knowlu_engine::pystr::read_text(&dir.join("state").join("events-seen.md")).unwrap_or_default()
}

/// §6.1 and D8: a pulled verdict fills a missing ledger line with the word only — no `why`, no
/// `strength` — titled from the roster `rank` last wrote, else `(untitled)`. A word the ledger does not
/// know is skipped, and the cursor still passes it.
#[test]
fn td1_d8_a_pulled_verdict_fills_a_missing_line_with_the_word_only() {
    let dir = desk("d8-fill");
    std::fs::write(
        dir.join("state").join("events.md"),
        "### Thu 2026-10-01\n- 18:00–19:00 · **Career Fair** · Union · `ics:fair`\n",
    )
    .expect("a roster");
    let mut server = loopback(vec![verdicts_reply(
        serde_json::json!([
            {"item_id": "ics:fair", "verdict": "opportunity", "judged_at": JUDGED_AT, "id": ROW_1},
            {"item_id": "ics:quiet", "verdict": "drop", "judged_at": JUDGED_AT, "id": ROW_2},
            {"item_id": "ics:odd", "verdict": "maybe", "judged_at": JUDGED_AT, "id": ROW_3},
        ]),
        false,
    )]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    let lines = sync::pull_event_verdicts(&dir, &client);
    assert_eq!(lines, vec!["event verdicts: 2 added from the account".to_string()]);
    let ledger = knowlu_engine::eventledger::load_ledger(&dir, None);
    assert_eq!(ledger["ics:fair"].verdict.as_deref(), Some("opportunity"));
    assert_eq!((ledger["ics:fair"].why.as_str(), ledger["ics:fair"].strength.as_str()), ("", ""), "no reason comes down");
    assert!(!ledger.contains_key("ics:odd"), "an unknown word is not a verdict");
    let text = ledger_text(&dir);
    assert!(text.contains("- ics:fair · Career Fair · verdict:opportunity · first seen "), "{text}");
    assert!(text.contains("- ics:quiet · (untitled) · verdict:drop · first seen "), "{text}");
    assert!(!text.contains("why:") && !text.contains("strength:"), "{text}");
    assert_eq!(sync::load_cursor(&dir).verdicts_after, format!("{JUDGED_AT},{ROW_3}"));
    let sent = server.requests();
    assert!(sent[0].starts_with("GET /functions/v1/judge-event HTTP/1.1"), "from the start: {}", sent[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// D8: a pulled verdict never replaces a line the ledger has.
#[test]
fn td1_d8_a_pulled_verdict_never_replaces_a_line_the_ledger_has() {
    let dir = desk("d8-keep");
    knowlu_engine::eventledger::record_verdict(&dir, "ics:fair", "Career Fair", jiff::civil::date(2026, 9, 20), "drop", "", "not for me", "")
        .expect("this desktop's own verdict");
    let before = ledger_text(&dir);
    let mut server = loopback(vec![verdicts_reply(
        serde_json::json!([{"item_id": "ics:fair", "verdict": "opportunity", "judged_at": JUDGED_AT, "id": ROW_1}]),
        false,
    )]);
    let lines = sync::pull_event_verdicts(&dir, &CloudClient::new(&cfg(&server.base), "jwt-not-a-secret"));
    assert!(lines.is_empty(), "{lines:?}");
    assert_eq!(ledger_text(&dir), before, "nothing appended");
    assert_eq!(sync::load_cursor(&dir).verdicts_after, format!("{JUDGED_AT},{ROW_1}"), "the cursor still passes it");
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dir);
}

/// D8: the next page starts after the last row taken — `(judged_at, id)`, so a tie at a page boundary is
/// never skipped — and a line the student deleted to force a re-judge (`eventledger.rs:46-47`) is not
/// put back: the cursor has passed its verdict.
#[test]
fn td1_d8_the_cursor_pages_past_a_judged_at_tie_and_a_deleted_line_stays_deleted() {
    let dir = desk("d8-page");
    let mut server = loopback(vec![
        verdicts_reply(
            serde_json::json!([
                {"item_id": "ics:a", "verdict": "drop", "judged_at": JUDGED_AT, "id": ROW_1},
                {"item_id": "ics:b", "verdict": "drop", "judged_at": JUDGED_AT, "id": ROW_2},
            ]),
            true,
        ),
        verdicts_reply(serde_json::json!([]), false),
    ]);
    let client = CloudClient::new(&cfg(&server.base), "jwt-not-a-secret");
    assert_eq!(
        sync::pull_event_verdicts(&dir, &client),
        vec![
            "event verdicts: 2 added from the account".to_string(),
            "event verdicts: more to come — the next sync continues".to_string(),
        ]
    );
    let path = dir.join("state").join("events-seen.md");
    let kept: String = knowlu_engine::pystr::read_text(&path)
        .expect("the ledger")
        .lines()
        .filter(|l| !l.starts_with("- ics:a "))
        .map(|l| format!("{l}\n"))
        .collect();
    knowlu_engine::pystr::write_text(&path, &kept).expect("the student deletes a line");
    assert!(sync::pull_event_verdicts(&dir, &client).is_empty());
    assert!(!knowlu_engine::eventledger::load_ledger(&dir, None).contains_key("ics:a"), "the deleted line stays deleted");
    let sent = server.requests();
    assert!(
        sent[1].starts_with("GET /functions/v1/judge-event?after=2026-09-25T12%3A00%3A00.123456%2B00%3A00,0f0e0d0c-0b0a-4908-8706-050403020102 HTTP/1.1"),
        "the next page starts after the tie's last row: {}", sent[1]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// R-TD1-4: a service that cannot answer the verdict read is one named line, never an error.
#[test]
fn td1_d8_a_verdict_read_the_service_refuses_is_a_named_line() {
    let dir = desk("d8-refused");
    let mut server = loopback(vec![(405, knowlu_engine::ledger::dumps_value(&serde_json::json!({"error": "POST only"})))]);
    let lines = sync::pull_event_verdicts(&dir, &CloudClient::new(&cfg(&server.base), "jwt-not-a-secret"));
    assert_eq!(lines, vec!["event verdicts: skipped (the service refused (405: POST only))".to_string()]);
    assert!(sync::load_cursor(&dir).verdicts_after.is_empty(), "the cursor does not move");
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dir);
}

/// R-TD1-4: a pull that met a transport failure says so in `Totals`, so the wrapper skips the verdict
/// read as it skips the push.
#[test]
fn td1_d8_an_offline_pull_is_marked_offline() {
    let dir = fixture("td1-offline");
    let cloud = cfg("http://127.0.0.1:9/functions/v1");
    let client = CloudClient::new(&cloud, "jwt-not-a-secret");
    let (_, totals) = sync::run_lines_with_client(
        &dir, sync::Direction::Pull, "cli", None, &client, &cloud, Vec::new(), sync::Totals::default(),
    );
    assert!(totals.offline, "{totals:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
```

Run: `cargo test --manifest-path C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\Cargo.toml -p knowlu-engine -j 2 --test sync_contract -- td1_d8`
Expected: FAIL to compile — `sync::pull_event_verdicts`, `Cursor::verdicts_after` and `Totals::offline`
do not exist.

- [ ] **Step 6: The cursor field, the offline flag, and the pull**

In `engine/src/sync.rs`, add to `Cursor` (`:213-233`), after `withheld_ids`:

```rust
    /// Two-desktop design D8 (re-review m4): how far this computer has read the account's event
    /// verdicts — the `<judged_at>,<id>` of the last row it took, empty for "from the start". A lost
    /// cursor pulls from the start again, which restores a line the student deleted: §2.8's named cost.
    #[serde(default)] pub verdicts_after: String,
```

Add to `Totals` (`:2347-2361`), after `more`:

```rust
    /// The pull met a transport failure (R-TD1-4): the wrapper then skips the verdict read, as the push
    /// half already waits for the network.
    pub offline: bool,
```

In `run_lines_with_client`'s pull `Err(e)` arm (`:2588-2592`), after `pull_offline = e.is_transport();`
add `totals.offline = pull_offline;`.

After `pull` (`:886-890`) add:

```rust
/// Every character a query value may not carry raw, percent-encoded (RFC 3986's unreserved set is
/// kept): a `judged_at` carries `+` and `:`, and a raw `+` would reach the server as a space.
/// `cloudmodel::urlencode_component` does the same for a feed name but is private to that module.
fn query_value(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            other => other.encode_utf8(&mut [0u8; 4]).bytes().map(|b| format!("%{b:02X}")).collect(),
        })
        .collect()
}

/// Two-desktop design D8 (§2.8): the account's event verdicts, pulled past this computer's cursor into
/// its own ledger, `state/events-seen.md`, so a desktop that newly holds the `feeds` turn judges only
/// what no desktop has judged, and every desktop shows the same *Coming up*.
///
/// **The word only.** `GET /judge-event` returns `item_id`, `verdict`, `judged_at` and `id`, never a
/// reason; `eventledger::record_verdict` gets an empty `why` and `strength`, so the line omits both,
/// and the title is the one `rank` last wrote in the roster, else `(untitled)` (R-TD1-6). **Never
/// replaces a line**: a uid whose ledger already carries a verdict is passed over. **Never restores a
/// deleted line**: the cursor has passed that verdict. The cursor moves over every row the page
/// carried, taken or not. One page per sync (R-TD1-5). **Lines, never an error** (R-TD1-4).
pub fn pull_event_verdicts(vault: &Path, client: &crate::cloudmodel::CloudClient) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cursor = load_cursor(vault);
    let path = match cursor.verdicts_after.rsplit_once(',') {
        Some((judged_at, id)) => format!("/judge-event?after={},{}", query_value(judged_at), query_value(id)),
        None => "/judge-event".to_string(),
    };
    let reply = match client.get(&path) {
        Ok(reply) => reply,
        Err(e) => {
            lines.push(format!("event verdicts: skipped ({})", SyncError::service(e).label()));
            return lines;
        }
    };
    let rows = reply.get("verdicts").and_then(Value::as_array).cloned().unwrap_or_default();
    let ledger = crate::eventledger::load_ledger(vault, None);
    let titles: std::collections::BTreeMap<String, String> =
        crate::eventroster::read_roster(&vault.join("state").join("events.md"))
            .into_iter()
            .map(|event| (event.uid, event.title))
            .collect();
    let today = crate::cli::local_now(vault).date();
    let mut taken: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut added = 0usize;
    let mut last: Option<String> = None;
    for row in &rows {
        let s = |k: &str| row.get(k).and_then(Value::as_str).unwrap_or_default();
        let (uid, verdict, judged_at, id) = (s("item_id"), s("verdict"), s("judged_at"), s("id"));
        if judged_at.is_empty() || id.is_empty() {
            // No place in the order, so it cannot move the cursor.
            continue;
        }
        last = Some(format!("{judged_at},{id}"));
        // The ledger's own uid characters (`eventledger::sanitize_uid`), so the line reads back.
        let readable = !uid.is_empty() && uid.chars().all(|c| c.is_ascii_alphanumeric() || "_.:@+-".contains(c));
        if !readable || !crate::eventledger::VALID_VERDICTS.contains(&verdict) {
            continue;
        }
        if ledger.get(uid).is_some_and(|entry| entry.verdict.is_some()) || !taken.insert(uid.to_string()) {
            continue;
        }
        let title = titles.get(uid).map(String::as_str).unwrap_or("");
        match crate::eventledger::record_verdict(vault, uid, title, today, verdict, "", "", "") {
            Ok(()) => added += 1,
            Err(e) => lines.push(format!("event verdicts: {uid} not recorded ({e})")),
        }
    }
    if let Some(last) = last {
        cursor.verdicts_after = last;
        if let Err(e) = save_cursor(vault, &cursor) {
            lines.push(format!("event verdicts: the cursor could not be saved ({e})"));
        }
    }
    if added > 0 {
        lines.push(format!("event verdicts: {added} added from the account"));
    }
    if reply.get("more").and_then(Value::as_bool).unwrap_or(false) {
        lines.push("event verdicts: more to come — the next sync continues".to_string());
    }
    lines
}
```

(`VALID_VERDICTS` is read, never listed here, so the fourth word another branch adds — `unsure`,
`origin/j-judgment-quality` — is taken the day it merges.)

In `run_lines_with`, replace its last two lines (`:2520-2521`) with:

```rust
    let (mut lines, totals) = run_lines_with_client(vault, direction, via, run_id, &client, &cfg, lines, totals);
    // Two-desktop design D8 (R-TD1-4): after its pull, the account's event verdicts, the word only —
    // skipped when the pull could not reach the account, and never an error.
    if direction.pulls() && !totals.offline {
        lines.extend(pull_event_verdicts(vault, &client));
    }
    finish(vault, lines, totals)
```

(`run_lines_with_client` is untouched in its sequence: its tests script exactly a pull and a push.)

- [ ] **Step 7: Run the device tests**

Run the `td1_d8` filter from Step 5. Expected: PASS, all five. Then
`--test sync_contract --test sync_replay --test entitlement_gate`: PASS (the gate test's `sync` stops
before `run_lines_with`, and every `run_lines_with_client` test is unchanged).

- [ ] **Step 8: Run the whole suite and the Deno gate**

Global Constraint 19 — PASS at 0 warnings — and the three Deno commands of Step 4 again, then the whole
`cloud/supabase/` suite with absolute paths:
`deno test --allow-read --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\deno.json C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\cloud\supabase\`
Expected: PASS, `migrations_test.ts` included and unchanged. (`cloud/eval/`, which this plan does not
touch, is left to the controller's gate: three of its tests read `cloud/eval/thresholds.json` relative to
the repository root, so they run only as CI runs them, from the root — *Hand-off to the controller*,
item 1.)

- [ ] **Step 9: Hand-off H4 is applied**

The controller applies **H4** (`CLAUDE.md`) verbatim as its own commit. An implementer never edits
`CLAUDE.md`.

- [ ] **Step 10: Commit**

Message file `.superpowers\sdd\msg-task-10.txt`:

```
feat: every desktop's sync pulls the account's event verdicts (two desktops, D8)

GET /judge-event (a new route on the existing function; POST unchanged)
returns the account's answered event verdicts after a (judged_at, id)
cursor: the word only, 500 a page, the account taken from the JWT alone.
After its pull, sync appends a line for any uid its own ledger lacks - no
reason, the title from its roster - never replacing a line and never
restoring one the student deleted. A new holder of the feeds turn then
judges only what no desktop has judged. No migration.

(the two trailers of Global Constraint 17)
```

`git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop add cloud/supabase/functions/judge-event/index.ts cloud/supabase/functions/judge-event/handler.ts cloud/supabase/functions/judge-event/handler_test.ts engine/src/sync.rs engine/tests/sync_contract.rs`
then `git -C C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop commit -F C:\Users\danie\GitHub\knowlu\.claude\worktrees\two-desktop\.superpowers\sdd\msg-task-10.txt`

---

## Controller hand-offs

Every change below is to a shared single-owner file (`HANDOFF.md` §2). **No task edits these files.**
Each lands on the branch at the task that first needs it, as its own commit, applied **verbatim** by the
controller and reviewed with that task — never at merge, because a hand-off that waits makes every task
after it untestable. None is compile-blocking for an earlier task; each is needed before its task's
integration tests can pass.

| Task | Hand-off applied with it |
|---|---|
| 2 | **H1** — `engine/src/ingest.rs::sync_tasks` mints import ids |
| 9 | **H2** — `engine/src/cli.rs`: `RankOptions`, `run_opts`, `run_with_opts`, the digest skip; **H3** — `engine/src/main.rs`: `rank --no-digest` |
| 10 | **H4** — `CLAUDE.md`: the three sentences this plan makes stale |

`engine/src/lib.rs` needs nothing (no new module), nor do `Cargo.toml` and `Cargo.lock` (no new crate:
`sha1` is already linked). `cloud/supabase/config.toml` needs nothing: `[functions.judge-event]` exists.

### H1 — `engine/src/ingest.rs::sync_tasks`: the LMS feed mints import ids (applied at Task 2)

After `let mut seen = load_seen(vault);` (`:633`), insert:

```rust
    // Two-desktop design D3: every id this vault already holds, read once for this run and extended
    // by `write::create_imported` as it mints — an imported note never takes an id a note has here.
    let mut held = crate::ids::held_ids(vault);
```

Replace the archive branch's `match` (`:750-758`, from `match crate::write::create(` to its closing
brace) with:

```rust
            match crate::write::create_imported(vault, &rel_path, &body, ctx, journal, &mut held) {
                Ok(created) => {
                    let stem = created.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    log.push(format!("archived (imported-past) {stem}"));
                    known.insert(event.uid.clone(), created);
                    let _ = record_seen(vault, &event.uid, &event.title, &stamp);
                }
                // Two-desktop design R-TD1-1: the item's note is already here under its import id —
                // named, not written, and recorded as seen, as the backfill above records a note it
                // found by `source_uid`.
                Err(crate::write::WriteError::IdHeld(id)) => {
                    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    log.push(format!("skipped (already held as {id}): {stem}"));
                    let _ = record_seen(vault, &event.uid, &event.title, &stamp);
                }
                Err(err) => log.push(format!("skipped (unwritable): {err}")),
            }
```

and the active branch's `match` (`:781-789`) with:

```rust
        match crate::write::create_imported(vault, &rel_path, &body, ctx, journal, &mut held) {
            Ok(created) => {
                let stem = created.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                log.push(format!("created {stem}"));
                known.insert(event.uid.clone(), created);
                let _ = record_seen(vault, &event.uid, &event.title, &stamp);
            }
            // Two-desktop design R-TD1-1, as in the archive branch above.
            Err(crate::write::WriteError::IdHeld(id)) => {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                log.push(format!("skipped (already held as {id}): {stem}"));
                let _ = record_seen(vault, &event.uid, &event.title, &stamp);
            }
            Err(err) => log.push(format!("skipped (unwritable): {err}")),
        }
```

**Without it:** `engine/tests/import_ids.rs`'s three tests fail (a random id, and no `IdHeld` line), and
two desktops keep doubling every LMS item.

### H2 — `engine/src/cli.rs`: `rank --no-digest` in the library (applied at Task 9)

Constraint: `engine/tests/cloud_contract.rs::rank_cannot_reach_a_judgment_endpoint` reads this file as
text; nothing below names a judgment endpoint or model type, comments included.

**Edit 1.** After `pub struct Fetchers<'a> { … }` (`:213-216`), insert:

```rust

/// How one `rank` run behaves beyond its date, runner and run id. `Default` is `rank` exactly as it
/// always was, which is what [`run`] and [`run_with`] pass — so `oracle.rs` and every frozen reference
/// are untouched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RankOptions {
    /// Two-desktop design D8: file no events digest this run. For a computer that did not hold the
    /// `feeds` turn this slot: the digest is the holder's, and reaches this computer through sync. The
    /// roster and *Coming up* are still built from this computer's own feeds and ledger.
    pub no_digest: bool,
}
```

**Edit 2.** Replace `run`'s signature (`:222-227`, from `pub fn run(` through `) -> Result<RunOutcome, RunError> {`)
with (the doc comment above it stays):

```rust
pub fn run(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
) -> Result<RunOutcome, RunError> {
    run_opts(vault, today_iso, runner, run_id, RankOptions::default())
}

/// [`run`] with [`RankOptions`] — what `knowlu-engine rank` calls.
pub fn run_opts(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
    opts: RankOptions,
) -> Result<RunOutcome, RunError> {
```

**Edit 3.** At the end of that body (`:256-262`), replace

```rust
    run_with(
        vault,
        today_iso,
        runner,
        run_id,
        Fetchers { calendar: Some(&calendar), events: Some(&events) },
    )
```

with

```rust
    run_with_opts(
        vault,
        today_iso,
        runner,
        run_id,
        Fetchers { calendar: Some(&calendar), events: Some(&events) },
        opts,
    )
```

**Edit 4.** Replace `run_with`'s signature (`:268-274`, from `pub fn run_with(` through
`) -> Result<RunOutcome, RunError> {`) with (the doc comment above it stays):

```rust
pub fn run_with(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
    fetchers: Fetchers<'_>,
) -> Result<RunOutcome, RunError> {
    run_with_opts(vault, today_iso, runner, run_id, fetchers, RankOptions::default())
}

/// [`run_with`] with [`RankOptions`].
pub fn run_with_opts(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
    fetchers: Fetchers<'_>,
    opts: RankOptions,
) -> Result<RunOutcome, RunError> {
```

**Edit 5.** Replace the digest call and its count (`:391-401`, from `let (_, emitted) = emit_digest(`
through `approvals.events_in_digest += emitted as i64;`) with:

```rust
        // Two-desktop design D8: `--no-digest` leaves the day's digest to the computer that held
        // `feeds`; the roster above and *Coming up* below are still built from this computer's own
        // feeds and ledger.
        if !opts.no_digest {
            let (_, emitted) = emit_digest(
                vault,
                &candidates,
                &ledger,
                &events_config,
                today,
                Some(remaining_budget),
                Some(&ctx.with_actor("agent:events")),
                &mut journal,
            );
            approvals.events_in_digest += emitted as i64;
        }
```

**Without it:** `engine/tests/rank_no_digest.rs` does not compile (`cli::RankOptions`,
`cli::run_with_opts`).

### H3 — `engine/src/main.rs`: `rank --no-digest` on the command line (applied at Task 9)

**Edit 1.** In `Command::Rank` (`:27-37`), after the `run_id` field:

```rust
        /// Two-desktop design D8: file no events digest this run — for a computer that did not hold
        /// the `feeds` turn this slot. The roster and *Coming up* are still written.
        #[arg(long = "no-digest")]
        no_digest: bool,
```

**Edit 2.** The match arm (`:401-402`):

```rust
        Command::Rank { vault, today, runner, run_id } => {
            match cli::run(&vault, today.as_deref(), &runner, run_id.as_deref()) {
```

becomes

```rust
        Command::Rank { vault, today, runner, run_id, no_digest } => {
            match cli::run_opts(&vault, today.as_deref(), &runner, run_id.as_deref(), cli::RankOptions { no_digest }) {
```

(The entitlement gate's `gated_vault` and `name_of` match `Command::Rank` nowhere else: `rank` is
never gated.) **Without it:** `the_binary_takes_no_digest_and_refuses_a_misspelling` fails — clap refuses
`--no-digest` with exit 2.

### H4 — `CLAUDE.md`: the sentences this plan makes stale (applied at Task 10)

**Edit 1** — *Engine invariants*, the bullet ending "Every note has an opaque `id:`; `source_uid` is the
external key." (`CLAUDE.md:40-43`): append, inside the same bullet,

> An imported note's `id:` is derived, not minted (two-desktop design D1–D3): `ids::import_id(kind,
> vendor, key)` over `ids::import_key`'s closed producer list, written by `write::create_imported`, so
> two computers importing one item create it under one id; every other note keeps a random
> `ids::new_id`. The derivation is frozen once shipped. One item already under two ids is an alias group
> (`state/id-aliases.json`, generated, never synced): the lowest id wins, and `Journal::records_for`,
> `human_set`, `resolve_target` and `detect_external` read a group as one note.

**Edit 2** — *The engine's commands*, the `rank` line (`:55`):

> - `rank --vault <v> [--today YYYY-MM-DD] [--runner manual|local|cloud] [--run-id <id>] [--no-digest]`
>   — `--no-digest` (two-desktop D8) writes the roster and *Coming up* but files no events digest, for a
>   computer that did not hold the `feeds` turn; the default is unchanged. A uid named in any digest note,
>   in `approvals/` or `archive/`, is already proposed.

**Edit 3** — the `sync` bullet (`:75-80`): after "another desktop's writes down and applied through
`write`." insert

> **Applied by id** (two-desktop D5, D6): records, moves, deletes and tombstones find the local note by
> its `id:` (or its alias group), never by a path that holds a different id; a pulled text whose id is
> held here writes no second file, and one whose path holds a different note lands at the next free
> `-N` name. After its pull, `sync` reads the account's event verdicts (`GET /judge-event`, the word
> only, `Cursor::verdicts_after`) into `state/events-seen.md` for any uid with no line.

The command-count sentence under *Knowlu (the app)* does not move: this plan adds no Tauri command.

## Spec problems found while planning, and how the plan handles them

None changes a decision. Each is a place the signed spec is silent or under-specified; the plan decides
it by a ruling above and says so, never by departing silently.

- **S-1 — §5.6 names a `cmt` arm of `import_key` without its vendor or key string.** D1 makes the
  derivation frozen once shipped, so the strings must be fixed before the commitment model and this
  stream meet. Not implementable here (no `cmt` kind on this branch); *Hand-off to the controller*
  proposes exact values for a ruling, owed by whichever of the two merges second.
- **S-2 — §2.6's "both files here" merge points at "the full reconcile above", which is against *other
  desktops'* records.** A field the student set on this desktop's copy of the losing note (a local
  record under the loser id) is on neither side of that split and would never reach the winner file,
  breaking §6.1 (v)'s "a field set only on the losing copy ends equal on both" for the both-files case.
  R-TD1-9 reconciles by id instead (the winner's records against the loser's, every device); test (v)'s
  N1 case proves it.
- **S-3 — §2.5 is silent on notes and records with no id.** (c) does not say what a pulled text does at
  a path held by a note with no `id:` line, and (b)/(d) do not say what a record with no id does.
  R-TD1-2 and R-TD1-15 keep today's path rule, as (a)'s own sentence does for step 4; this is what keeps
  every existing test on `vault-s1` (which carries no ids) green.
- **S-4 — D6 lists `records_for`, `human_set` and `resolve_target` as the readers that follow the alias,
  but not `passes::detect_external`,** whose id-keyed index would turn a re-identification into
  fabricated `quinn`/`external` edits at the next `rank`. R-TD1-18 extends D6's own rule to it.
- **S-5 (minor) — §2.3 says an `IdHeld` is "logged", not what each producer does next** (seen ledger,
  Gmail acknowledgement, the card's settlement). R-TD1-1.
- **S-6 (minor) — §2.8's "the title from its own feed or roster":** `sync` never fetches a feed, so the
  roster `rank` last wrote is the source (R-TD1-6); nothing is lost, since the title in a verdict line
  is informational.
- **S-7 (minor) — §2.5 (c) gives no line for a text written at a `-N` name, nor for the alias joining a
  text.** R-TD1-3 words both, with the other new lines.

Found outside the spec: **F-1** (*Hand-off to the controller*, item 5) — a withheld sync-card field may
read as an external edit at the next `rank`, a suspected pre-existing C3′ defect this plan does not fix.

## Self-review (2026-09-25, done while writing)

- **Spec coverage.** Every §2 sentence that asks for behaviour has a row in the *Fidelity ledger* and a
  task; every §6.1 bullet this plan owns has a named test ((i) Task 2, (ii) Task 5, (iii)–(iv) Task 6,
  (v) Task 7, (vi) Task 8, the verdict pull Task 10, `eventemit` and `--no-digest` Task 9); §6.3's
  `judge-event` GET has its `handler_test.ts`; §5.4's E5 and M5 are Task 6; N17 and M6 are named where
  they go. D9–D19 are Plans 2 and 3 and are only referenced.
- **Placeholders.** None: every test is written out, every implementation step is exact code or names
  the exact function, lines and text to change (Task 7's one mechanical extraction names its lines and
  its only two edits). "TBD" appears only inside two existing tests' own fixture text.
- **Type consistency.** `create_imported`'s six parameters, `import_key`'s `Option<ImportKey>`,
  `IdIndex`'s five methods (plus `vacate`), `settle_resolution`'s eleven parameters, the producers' new
  `held` parameters, `RankOptions { no_digest }`, `Cursor::verdicts_after` and `Totals::offline` are
  spelled the same in every task that uses them and in *Interfaces this plan produces* — and compiled
  together (header, *Checked while writing*).
- **Warnings.** `IdIndex::exact` and `forget` carry `#[allow(dead_code)]` from Task 5 until Task 7 uses
  them; `float_at` is added with its first use (Task 7), not before.
- **Defects the scratch run found and this text fixes:** `float_at` defined a task before its first use
  (a dead-code warning at the end of Task 2); a sixth `write_rule_card(` call (`enrich.rs:2653`) missing
  from Task 3's list; a CRLF-blind assertion in Task 3's rewritten Gmail equivalence test.

## Hand-off to the controller

After Task 10's commit the branch carries Plan 1 whole. What is left is the controller's.

1. **The gate**, in the foreground, from the worktree: `cargo build -p knowlu-engine -j 2`, then
   `cargo test --workspace -j 2` — 0 warnings, the gate line ending `0 other`; the whole Deno suite as
   CI runs it (`ci.yml:98`, from the repository root, `cloud/supabase/` and `cloud/eval/` together);
   `pwsh -File scripts/ci/eol-check.ps1` — LF everywhere but `*.ps1`, nothing under
   `engine/tests/fixtures/**` touched; `git diff --stat 3e6e13b -- engine/tests/fixtures` empty.
2. **Staging.** Deploy `judge-event` (the one function this plan changes; no migration, no secret). The
   smoke, with a staging session minted per the standing OTP procedure: `GET /judge-event` answers 200
   `{"verdicts": [...], "more": false}` carrying only `item_id`, `verdict`, `judged_at`, `id`; a
   malformed `after` answers 400; a `POST` still judges. Production parity gains one line: redeploy
   `judge-event` with the rest of C2's functions (`HANDOFF.md` §4's checklist).
3. **`HANDOFF.md` §3**, batched with the next milestone push (CI minutes): the two-desktop stream's Plan 1
   is code-complete on `two-desktop` (D1–D8, E5, M5 in; N17 out; M6 with Plan 3's D16), Plans 2 and 3 to
   be written against *Interfaces this plan produces*.
4. **What Plans 2 and 3 consume** is the table *Interfaces this plan produces*, verbatim.
5. **Finding F-1, outside this plan and the spec (suspected, not verified).** `passes::detect_external`
   (`passes.rs:259-350`) builds each note's expected values from the latest journal record per field.
   When C3′'s `apply` withholds a foreign value behind a sync card, the foreign record is the latest one
   while the note keeps this desktop's value, so the next `rank` would journal a fabricated
   `quinn`/`external` edit restating the local value — a record that travels, applies cleanly on the
   other desktop ("upstream never moved"), flips it back, and leaves this desktop's card offering a
   value neither holds. Nothing in C3′'s suites runs `rank` between two syncs. Recommended: a probe test
   (two desktops, a card, then `passes::detect_external` on the carded side) before the two-PC proof,
   and a ruling on the fix (skip a field a live sync card withholds). R-TD1-18 covers only the alias half
   of this, which D6 itself would otherwise cause.

### Overlaps with the unmerged branches

Read with `git diff --name-only $(git merge-base HEAD <branch>) <branch>` on 2026-09-25; none of these
branches is edited by this plan. Whichever merges second resolves the textual conflicts; the semantic
notes below are what a resolver must not lose.

| Branch | Files it shares with this plan | What to keep in mind |
|---|---|---|
| `origin/p1-commitments`, `p2-commitments` (same file set) | `engine/src/{ids,write,approvals,coursework,enrich,eventemit,cli,main}.rs`, `CLAUDE.md` | **`ids.rs`**: adds kind `cmt`, `commitments/` to `NOTE_FOLDERS` and `ID_RE`, `kind_for`'s `commitment` arm — a textual merge beside Task 1's additions; then §5.6's `cmt` arm of `import_key` is owed (spec defect S-1, below). **`write.rs`**: `propose_amendment` gains a `judgment` parameter, which `sync.rs`'s own call (`:2151`, moved into `settle_resolution` by Task 7) must pass as `None`. **`eventemit.rs`**, **`approvals.rs`**: new code after `emit_digest` and in `transition_note`; Task 3's `create_imported` and `held` parameters must reach any new `write::create` of an imported note there. **`cli.rs`/`main.rs`**: H2/H3 against their `run`/`run_with`/`main` edits. **`enrich.rs`**: `write_gmail_card` and `pull_gmail` both change. |
| `origin/j-followups` | `engine/src/{write,approvals,coursework,enrich,eventemit,cli}.rs`, `CLAUDE.md` | Adds `kind: event-check` cards (`eventemit.rs`, `created_by: events`, a `source_uid`): **not a §2.2 producer**, so `import_key` answers `None` and they keep random ids — two desktops can each file one until Plan 3's turn makes one computer the `feeds` holder. A later spec row, not this plan, would add them. `pending_digest_uids` and its tests are unchanged on both sides (R-TD1-7 kept them for this). |
| `origin/j-judgment-quality` | `engine/src/{coursework,enrich,cli}.rs`, `CLAUDE.md`; also `_shared/judge_handler.ts` | Adds `unsure` to `eventledger::VALID_VERDICTS` and passes `accepts`/`timezone` through `judgeHandler`. Task 10's pull reads `VALID_VERDICTS`, so `unsure` verdicts are pulled the day it merges; `judge-event/handler.ts` wraps `judgeHandler` without touching it, so the POST keeps whatever `judge_handler.ts` becomes. |
| `jf-cloud` | `engine/src/{coursework,enrich}.rs`, `CLAUDE.md` | Its `coursework.rs` hunks are in `collect_cloud`/`fetch_*`/`main_with_fetchers`; its `enrich.rs` hunk is in `pull_gmail` — beside Task 3's outcome `match`. |
| `j-events` | none of this plan's files (`eventledger.rs`, `cloudmodel.rs`, `migrations_test.ts` only) | — |
| `fix-literal-dashes` (visible, not named in the brief) | `engine/src/{write,approvals,coursework}.rs` | Adds `WriteError::LineBreak` beside Task 1's `IdHeld` and a guard at the top of `write_literals`; both enums' `Display` arms merge side by side. |

### The commitment model's `cmt` notes (spec §5.6) — owed by whichever of the two lands second

§5.6 says `import_key` keys `commitments/` notes (kind `cmt`, no `created_by:`) "by that prefix", but
fixes neither the vendor word nor the key string, and D1 makes the derivation frozen once shipped. This
plan cannot implement it (no `cmt` kind on this branch) and does not decide it. **Proposed for the
controller's ruling**, in §2.2's own shape — the vendor is the `source_uid`'s prefix, the producer word
`commitment`:

```rust
        // in import_key, before the `created_by` requirement:
        if kind == "cmt" {
            let uid = field("source_uid")?;
            let vendor = uid.split(':').next().filter(|p| *p == "gcal-series" || *p == "registrar")?.to_string();
            return Some((kind, vendor, format!("commitment:{uid}")));
        }
```

so `import_id("cmt", "gcal-series", "commitment:gcal-series:4k2q9x7m1abc")`, with a pinned reference value
computed at merge. The commitment model's §2.5 already keeps the lowest id among duplicates, which is
D6's rule.
