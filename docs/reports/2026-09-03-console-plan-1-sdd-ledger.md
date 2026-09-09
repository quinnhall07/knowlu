# Console plan 1 — SDD ledger (rulings R1–R40, reviews, deferred findings)

The controller ledger of the subagent-driven execution of `docs/superpowers/plans/2026-09-02-console-foundation-plan.md`, 2026-09-02/03. Preserved verbatim from the worktree workspace (`.superpowers/sdd/`, ignored by the VCS) so plan 2 starts from the deferred findings and the rulings. Line numbers cited by the final review refer to the original file; this header shifts them by 6.

---

# SDD ledger — plan: docs/superpowers/plans/2026-09-02-console-foundation-plan.md

Spec: docs/superpowers/specs/2026-09-02-console-on-rust-design.md (read in full 2026-09-02 22:10; binding authority).
Workspace: git worktree `.claude/worktrees/console-plan-1`, branch `worktree-console-plan-1`, base 4b76c00 (Quinn chose "worktree on a branch", 2026-09-02).
`.venv` is a junction to the main checkout's venv. Dev + release builds warmed (77 s / 126 s cold).
Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4

## Pre-flight scan (2026-09-02 22:20)

### Pairs sharing a file or an interface

| Tasks | Produces → consumes | Finding |
|---|---|---|
| T1 → T4 | `capacity_breakdown(&WeekCalendar, Date) -> Capacity{text,capacity,template,calendar}`, `status_word`, `py_float`, `round2`, `short_date` pub → `surface::{verdict,meter,topline}` | consistent |
| T1 → T5 | `one_line`, `due_move(DateTime, DateTime) -> String` pub → `amend_card` | consistent |
| T1 → T6 | `upcoming(&[ComingUpEvent], Date, Option<DateTime>) -> Vec<ComingUpEvent>`; `ComingUpEvent{uid,title,start,location,organizer}` already pub with pub fields (render.rs:47) → `surface::coming_up` | consistent (verified) |
| T2 → T4 | `load_task_notes(&Path, Option<&mut Vec<String>>) -> Vec<(Task, Mapping)>` → `surface::load` | consistent; existing `load_tasks` uses `Task::from_file` and filters `active` AFTER the skipped list — the twin must keep that order |
| T2 → T6 | `WeekCalendar::template_blocks` pub (weekcal.rs:145, pre-`min_filter`, so the class gaps 12:00–12:50 / 13:00–13:45 hold; `day_start`/`day_end` are already pub `Time`) | consistent (verified) |
| T2 → T7 | `approvals::proposal_weight(&Mapping) -> i64` pub (approvals.rs:854) | consistent |
| T2 → T9/T10 | `lib.rs BUILD_SHA: Option<&str>` via root `build.rs` → `topline.engine_build`, `--build-sha`, `commands::CONSOLE_BUILD` | consistent; see R6 |
| T3 → T4/T5/T6 | `Take{task,hours,block_index,block_before,block_after,budget_after,capped_by}`; `StartByWhy{start_by,needed_hours,days_walked,slice_cap,buffer_days}` | consistent |
| T4 → T7/T8 | `local_stamp(vault, ts)`, `newest_end_record`, `cli::vault_zone(&Path) -> TimeZone` (T4 extracts it from `local_now`) | consistent; see R5, R9 |
| T4 → T9/T10 | `View::parse/name`, `Topline` fields; T10 adds `console_build`/`engine_newer` on the JSON value, T12 reads them | consistent |
| T5 ↔ T7 | `PendingAmendment{target,title,urgency,proposal,changes: Mapping,age_days}` defined T5, filled T7 | consistent (six fields both sides) |
| T5 → T6/T7/T8/T13 | `EMPTY_TEXT` fields `active,must_do,recommended,list,the_day,decisions,info,issues,runs,coming_up,offline`; T13 adds `closed` + `Texts` | consistent |
| T5 → T8 | `is_judged(&Mapping, &[ledger::Record])`; `ledger::Record = Map<String, Value>` (ledger.rs:36) | consistent (verified) |
| T7 → T8 | `read_approvals -> ApprovalsRead{…, amendments}`; `Journal::read(Option<&str>, Option<&str>)`, `Runs::read(Option<&str>)`, `expected_status(&Path, Timestamp) -> Result<Vec<StatusRow{runner,due:String,status:&'static str}>,String>` | consistent (verified) |
| T8 → T9 | `state_json`, `revision` → `revision_of` pub alias; `build_state(vault, View, Date, &Zoned, Option<&str>)` | consistent; see R7 |
| T8 → T10 | `note_detail(vault, id, Date, &mut Journal)`; `journal::now_ts(Option<Timestamp>) -> String` for `mark_seen` | consistent (verified) |
| T9 → T13 | the three references; T13 regenerates them with a reviewed diff (adds `texts`) | consistent; see R1 |
| T10 → T12 | envelopes `{ok,error,state}` / `{ok,error,note}` / `{ok,error,seen_at}` | consistent |
| T11 → T12 | `static_assets.rs` reads `console.js`, which T12 creates | **conflict** → R3 |
| T12 ↔ T12 test | test asserts the literal `__TAURI__.core.invoke`; the draft JS calls `tauri.invoke` | **conflict** → R2 |
| T12 → T13 | T13 forbids six strings; the draft hardcodes three *other* strings | **defect** → R4 |
| T12 → T14 | `invoke` gains the `?fixture=` branch only when `__TAURI__` is absent | consistent |
| T15 → cutover Task 5 | `scripts/local-run.ps1` is the live runner (renamed 2026-09-02); T15 edits it on the branch | consistent |

### Per-task self-consistency

| Task | Finding |
|---|---|
| T1 | tests ↔ code agree; `WeekCalendar::from_file(path, events)` arg order verified |
| T2 | code sketch uses `NoteError::Io` + `split_frontmatter`; the note says copy `load_tasks`' real read path (`Task::from_file`) — fine |
| T3 | agrees; the `capped_by` precedence is spelled out |
| T4 | Files header omits `src/cli.rs`; commit line and the note at plan l.751 include it → R9. Plan says `vault_zone` defaults `America/Chicago`; `local_now` falls back to the SYSTEM zone → R5 |
| T5 | agrees; slug names to confirm with `ls` (plan says so) |
| T6 | `hours_between` uses `Span::get_seconds()` — a component, not a total → R8 |
| T7 | `list_info`/`list_issues` return `Vec<Mapping>` with a `path` key injected (info.rs:108) — the builders' `meta_text(m,"path")` works |
| T8 | `build_state_never_writes` byte-compares the fixture; `Journal::read` writes nothing (the index is `passes.rs`' alone) — verified |
| T9 | prose says the oracle passes `--build-sha pinned`; the `surface()` helper's arg list omits it → R1 |
| T10 | lib+bin split for the tests; `generate_handler![commands::state]` across the module boundary relies on tauri's `pub use __cmd__*` — standard |
| T11 | see R3 |
| T12 | see R2 |
| T13 | see R4; the empty-vault test needs `recurring: []` in vault-s1's planning.yaml — verified true |
| T14 | agrees |
| T15 | agrees |

### Rulings (pre-flight)

- Ruling R1: T9's `surface_oracle.rs` helper passes `--build-sha pinned` on every invocation — the prose and the references require it; the code block omitted it — costs nothing if wrong beyond a redundant flag.
- Ruling R2: T12's `invoke` calls `window.__TAURI__.core.invoke(cmd, args)` literally (presence check kept) — the test and spec §6 name that call — if wrong, one test asserts a string.
- Ruling R3: T11's `no_network_reference_in_the_shipped_page` iterates `["index.html", "console.css"]`; T12 adds `"console.js"` when the file exists — keeps T11 green without a stub file — if wrong, T11 is one file short of coverage for one task.
- Ruling R4: T13's forbidden list gains `"No events coming up"`, `"Nothing to know right now"`, `"Nothing closed this week"` (what T12's draft actually hardcodes) — makes the test red before the fix, as the plan claims — if wrong, an over-strict string test.
- Ruling R5: `cli::vault_zone` falls back to `jiff::tz::TimeZone::system()`-equivalent (what `local_now` does today), NOT `America/Chicago`; all three fixtures declare `timezone: America/Chicago`, so the references remain machine-independent — if wrong, a friend's vault without a timezone shows machine-local times, which is today's behaviour.
- Ruling R6: root `build.rs` derives its rerun triggers from `git rev-parse --git-path HEAD` (plus the symref's target file when HEAD is symbolic) instead of the literal `.git/HEAD` — correct in a worktree (`.git` is a file) and in the main checkout — if wrong, an extra rebuild, never a wrong SHA.
- Ruling R7: `build_state` takes `now: &jiff::Zoned` (plan) rather than spec §4's `DateTime` — the zone is needed for `runs_panel`/`gauge`; the plan's signature is used consistently by T9/T10 — if wrong, a signature the spec glossed.
- Ruling R8: T6's `hours_between` uses `DateTime::duration_since` → `SignedDuration::as_secs_f64() / 3600` (or `Block::hours()` where one exists) — `Span::get_seconds()` is a field of a balanced span — if wrong, the test pins the hours anyway.
- Ruling R9: T4 modifies `src/cli.rs` (the `vault_zone` extraction) and commits it — the Files header omission is clerical.
- Ruling R10: engine tasks 1–9 are NOT merged to main task by task; one merge is proposed to Quinn after Task 9 (subcommand + references frozen) — a push to the shared branch needs the go — if wrong, the dual-run harness proves the read-only changes a few days later than it could.

Watch (not rulings): T13's empty-vault test omits `archive/`, `courses/`, `profile/` — `count_proposals_created` skips missing folders (verified); the other readers are the implementer's to verify. vault-full's `state/runs/` holds only `.gitkeep` → `generated_*` None and `delta.since_kind == "none"` for that fixture.

## Progress

- Task 1: dispatched 2026-09-02 22:30, implementer sonnet, BASE 4b76c00
- Task 1: minor (deferred): the coming_up formatting hunk was cut off in the review package — unmodified context, rename only; re-check in the final review
- Task 1: minor (deferred, plan-mandated): Capacity.calendar is 0.0 in the collapsed branch (< 0.005h) — surface consumers (T4 verdict/meter) must know
- Task 1: complete (commits 4b76c00..7e1f9d0, review clean)
- Task 2: dispatched 2026-09-02 22:58, implementer sonnet, BASE 7e1f9d0
- Task 2: minor (deferred): temp-dir test lacks a Drop guard (leaks qo-ltn-<pid> on assertion failure)
- Task 2: minor (deferred): load_tasks/load_task_notes duplicate the match-over-scan_task_notes shape; a doc cross-reference on load_tasks
- Task 2: complete (commits 7e1f9d0..b137ec4, review clean)
- Task 3: dispatched 2026-09-02 23:35, implementer sonnet, BASE b137ec4
- Task 3: minor (deferred): designate_today wrapper has no doc comment (start_by's does)
- Task 3: complete (commits b137ec4..cae3da7, review clean)
- Task 4: dispatched 2026-09-02 23:58, implementer sonnet, BASE cae3da7
- Ruling R11 (found preparing T5): the amend proposal's change set is `changes: {due: {from, to}}` (approvals fixture and `render::amendment_block`), not the scalar `due: <date>` the brief's T5 tests construct — the tests build `mapping_of("due:\n  from: 2026-09-25T09:00\n  to: 2026-09-22T09:00")`, `amend_card` reads `from`/`to` exactly as `amendment_block` does (before = the target note's current due, else `from`; stale when both exist and differ), and `amend_sentence` renders a mapping value's `to` — if wrong, the golden text `9/25 → 9/22 (3 days EARLIER)` would not reproduce, and the test pins it.
- Ruling R12: `surface::PendingAmendment` is `pub use crate::approvals::AmendmentEntry as PendingAmendment` — the brief's struct is field-for-field the engine's (`Debug, Clone, Default, PartialEq`), so T7's read-only scan yields the same type `process_approvals` fills — if wrong, one alias to replace with a struct.
- Fact for T5: the GN slug is `gn-103-hausaufgaben-2026-08-31` (the brief said to confirm with ls).
- Ruling R13 (T4, implementer's resolution accepted): nav count hours are `round1(sum of raw remaining)`, not the brief's `round2` — the brief's own golden assertions (week 5.3, all 20.3; GN 103 has 2.79h left) hold only at one decimal, which is also how rows and today.md print hours — if wrong, a second decimal on a nav badge.
- Task 4: review round 1 → Needs fixes: 3 Important (round1 tie rule; generated_* assertion + local_stamp coverage; verdict lede/overflow precision) + spec item (verdict rounds rw before status_word)
- Task 4: minor (deferred): runway fold seeds NO_DUE_DATE_SLACK (clamps at 999 where render's None-seeded fold would print 1100) — F4 territory; final review triages
- Task 4: minor (deferred): newest_end_record compares ts as strings (mixed sub-second precision misorders)
- Task 4: minor (deferred): spill_hours (144.0) vs sum of spill_items (145.0) disagree — a partially fitting task is listed whole
- Task 4: minor (deferred): vault_zone doc claims TimeZone::UTC fallback; jiff's Zoned::now uses TimeZone::unknown() — same civil result, wrong claim
- Task 4: minor (deferred): Loaded.tasks duplicates Loaded.ranked (pre-rank order) — document or drop
- Task 4: minor (deferred): verdict calls partition_must_do twice
- Task 4: note for T9: local_stamp falls back to the machine zone for a vault without timezone: — fixtures all pin America/Chicago; keep it so
- Ruling R14: the never-writes grep (preservation check 2) applies to `src/surface.rs` ABOVE its `#[cfg(test)]` line; tests may build temp fixtures through the engine's own writers (`pystr::write_text`, `Runs::append` — the latter because `only_this_module_opens_ledger_files` forbids hand-written ledger files) — if wrong, a stricter grep that forces fixture-building into another module.
- Task 4: fix round 1/5 dispatched: commit 0ec98f7; scoped re-review pending
- Task 4: fix round 1/5 (4 addressed, 0 open; commits 5306184..0ec98f7)
- Task 4: complete (commits cae3da7..0ec98f7, review clean after 1 fix round)
- Task 5: dispatched 2026-09-02 23:36, implementer sonnet, BASE 0ec98f7
- Task 5: review round 1 → Approved on spec, 1 Important (sort_rows zip/drain can silently truncate) → fix round 1 dispatched
- Task 5: minor (deferred): the mandated amendment tests cannot distinguish amend_card from a vault-ignoring version (target due == from; title equal) — add a stale case
- Task 5: minor (deferred): amend_card title fallback diverges from amendment_block on empty title / empty proposal title
- Task 5: minor (deferred, plan-mandated): group hours sum rounded row hours while the section total rounds the raw sum (0.4 vs 0.5 possible)
- Task 5: minor (deferred): partition_must_do computed three times per page build; start_by_explained twice per row
- Task 5: minor (deferred, plan-mandated): badge matches a bare slug that resolve_amend_target would refuse; amend_sentence prefix is date-shaped for any field
- Task 5: minor (deferred): flag_text/merge_text rationale is a free comment, not field docs
- Task 5: minor (deferred, plan-mandated test): sort_is_start_by_then_due test uses None<Some while cmp_none_last puts None last — goes red the day an undated task enters 'all'
- Task 5: fix round 1/5 (1 addressed, 0 open; commits 5924ba1..17531cb)
- Task 5: minor (deferred, out of scope): must_do's labels/members are two Vecs grown together and zipped — safe by construction, same shape as the fixed one
- Task 5: complete (commits 0ec98f7..17531cb, review clean after 1 fix round)
- Task 6: dispatched 2026-09-03 00:05, implementer sonnet, BASE 17531cb
- Ruling R15 (T7, from approvals.rs:1247-1268): a `snoozed` note whose `snooze_until <= today` is counted as PENDING by the read model (the pass wakes it in place); `snooze_until > today` is deferred; absent/unparseable `snooze_until` is neither (the pass warns and leaves it) — the brief counted every `snoozed` as deferred — if wrong, one proposal shows in the deck a pass earlier than today.md would.
- Ruling R16 (T7, from approvals.rs:1270-1292): a `pending` note with `expires < today` is EXCLUDED from pending/cards/oldest (the pass expires and archives it); an unparseable non-absent `expires` stays pending — the brief counted it — if wrong, a card the next pass deletes is visible for up to six hours.
- Ruling R17 (T7, from approvals.rs:1333-1337): `oldest` is taken over EVERY pending note including `events-digest` ones — the brief skipped digests with `continue` before updating oldest — if wrong, `oldest Nd` can disagree with today.md's header.
- Trap for T7: `approvals::days_between(from, to)` (private, = to − from) has the OPPOSITE argument order from `scheduling::days_between(later, earlier)`; surface.rs imports scheduling's, so age = `days_between(today, first).max(0)` as the brief writes.
- Task 6: minor (deferred): free blocks carry raw Block::hours() beside round2'd busy/class hours in the same lane (R8-mandated) — a :40 event yields 2.3333333333333335 in the frozen JSON; the comment at :818 overstates 'same value'
- Task 6: minor (deferred, plan-mandated): open_hours is unclamped while spare_hours is clamped at 0 — T8/T12 must expect a negative
- Task 6: minor (deferred): commitments derivation duplicated verbatim in recommended and the_day
- Task 6: minor (deferred): Take.block_index is a positional join against free_blocks(today) — (l, today) must agree; module-wide convention
- Task 6: complete (commits 17531cb..637e959, review clean)
- Task 7: dispatched 2026-09-03 00:24, implementer sonnet, BASE 637e959
- Ruling R18 (T7): the fixture holds NO events-digest note — the golden's `3 in today's digest` is emitted by `emit_digest` during the rank run; a read-only scan of the pristine fixture reports `events_in_digest == 0`. The brief's `(2, 1, 3, 0, 0)` becomes `(2, 1, 0, 0, 0)` and a planted-digest test proves the weight path — if wrong, the assertion would be unreachable without a write.
- Ruling R19 (T7 review, plan-mandated clause): `Decisions.empty_text` fires only when there are no cards AND `events_in_digest == 0` — a pending digest is N decisions waiting (CLAUDE.md: one proposal = one decision; a digest counts its events) — if wrong, the deck says 'queue clear' beside '3 in digest'. Carry to T12: when cards are empty and empty_text is None, render the digest count line in the deck.
- Task 7: review round 1 → Approved on spec, 1 Important (plan-mandated, R19) → fix round 1 dispatched
- Task 7: minor (deferred): runs_panel has no test against real run records (grouping, sort, truncate, warn_count's recent half) — plant via start_run/add_step/end_run
- Task 7: minor (deferred, brief-verbatim): a crashed run reads 'running' and is not counted from recent; empty_text can accompany an in-flight row
- Task 7: minor (deferred): expected_status error swallowed by unwrap_or_default — a missing runners.yaml reads all-clear; RunsPanel has no warnings channel (T8/T9)
- Task 7: minor (deferred): sorted_md duplicated (approvals::sorted_md is pub(crate)); unreadable/not-an-approval notes skipped silently
- Task 7: minor (deferred): amend target via meta_text keeps whitespace / bool spelling where the pass uses truthy_str
- Task 7: minor (deferred): closed_this_week can emit two rows per note (progress=100 + status=done) and has no until bound
- Task 7: minor (deferred): gauge arithmetic untested (no writer exists; spec §4.5) — plant a step record
- Task 7: minor (deferred): R15/R16 test one arm each
- Task 7: fix round 1/5 (1 addressed, 0 open; commits 45ca4ef..0f70023)
- Task 7: complete (commits 637e959..0f70023, review clean after 1 fix round)
- Task 8: dispatched 2026-09-03 01:07, implementer sonnet, BASE 0f70023
- Ruling R20 (T9, found preparing): the brief's reference-generation recipe (`Out-File -Encoding utf8 -NoNewline` + BOM strip) drops the trailing newline the binary's `println!` emits, so the oracle's byte compare (`got.replace(CRLF, LF) == want`) would fail on the last byte. Generate each reference as the binary's exact stdout with CRLF line endings (fixtures convention): in Git Bash `quinn-ops.exe surface … --build-sha pinned | sed 's/$/\r/' > tests/fixtures/surface-today-<f>.json` — one line + CRLF, UTF-8, no BOM — if wrong, the oracle says so on first run.
- Ruling R21 (T8 review, plan-mandated line): `delta` passes `seen_at` whole to `Journal::read` — the ledger derives the day bound itself; the brief's `&s[..10]` panics on a short/non-ASCII stamp — if wrong, nothing: output-identical.
- Ruling R22 (T8 review, plan-mandated line): `note_detail` builds its row with `read_approvals(vault, today).amendments` so the drawer carries the same amend badge as the list — if wrong, one extra read-only scan per drawer open.
- Task 8: review round 1 → Needs fixes: 2 Important (R21, R22) → fix round 1 dispatched
- Task 8: minor (deferred): the run-fallback arm of delta (since_kind == run) is never exercised — all fixtures have empty runs/
- Task 8: minor (deferred): never-writes test does not detect a newly created empty directory or a write outside the vault root
- Task 8: minor (deferred): revision's 'changes when payload changes' half untested
- Task 8: minor (deferred): journal.warnings() dropped by delta/note_detail; State.unreadable covers tasks/ only
- Task 8: minor (deferred): unreadable list formatted twice (ranked_list 'all' and State) — ships twice in one payload
- Task 8: minor (deferred): delta.records unbounded for an old seen_at
- Task 8: minor (deferred, brief-inherited): NoteDetail.judgment duplicates a frontmatter sub-mapping; delta's now parameter is dead

## Cutover week (read daily)
- 2026-09-03 13:05 (the 12:00 slot, caught up 65 min late by -StartWhenAvailable — laptop asleep; not diagnosed inside grace, C12): CLEAN files=242 lint=0/0 exits 0/0 15s git=d97f976 bin=1c2832fd5972 (binary unchanged from 09-02). COUNTS: 1 of 14, day 1. runner-log: cloud 08:07 ok (147 active; 1 expired), local 13:05 ok ×2. Task 7 step 1 (rehearsal) still due today.

- Task 8: fix round 1/5 dispatched: commit 6bc420c; scoped re-review pending (session resumed 2026-09-03 13:07 after the previous process exited)
- Task 8: fix round 1/5 (2 addressed, 0 open; commits 9ed2d2f..6bc420c)
- Task 8: complete (commits 0f70023..6bc420c, review clean after 1 fix round)
- Task 9: dispatched 2026-09-03 13:20, implementer sonnet, BASE 6bc420c
- 2026-09-03 13:11 Task 7 step 1 (day-1 rollback rehearsal) on a fresh copy of the live vault: REHEARSAL-CLEAN stage1=CLEAN stage2=CLEAN external=+0/+0 git=0320c6e — logged to the live state/dual-run-log.md (the 18:00 run commits it)
- Task 9: review round 1 → Approved on code, 2 Important doc findings (HANDOFF NEVER-headline governs the may-regenerate table; oracle recipe does not reproduce the bytes) → fix round 1 dispatched
- Task 9: minor (deferred): exit-2 test asserts only the code (passes on any clap rejection) — assert stderr contains 'unknown view'
- Task 9: minor (deferred): no test for exit 2 on bad --today / --now; --seen-at unexercised by any artefact
- Task 9: minor (deferred): oracle failure message slices by byte (&got[..600]) — a multibyte char at the boundary would panic
- Task 9: minor (deferred): --view is a free String (no value list in --help) where Rank's runner uses value_parser
- Task 9: fix round 1/5 (2 addressed, 0 open; commits 6c1ef4f..6499626)
- Task 9: minor (deferred): surface_oracle.rs:7-9 prose attaches the LF clause to PowerShell's > — confusing wording, command correct
- Task 9: complete (commits 6bc420c..6499626, review clean after 1 fix round) — ENGINE TASKS 1–9 DONE; merge checkpoint (R10) put to Quinn 2026-09-03
- Task 10: dispatched 2026-09-03 13:33, implementer sonnet, BASE 6499626
- 2026-09-03 14:08 MERGED engine tasks 1–9 into main: f8c36d2 (merge --no-ff of 6499626; 17 files, +2789/−57; pushed, main level with origin). Quinn: 'Merge now'. The 18:00 slot rebuilds the engine with build.rs — expect a NEW bin= on that line; rank output must stay CLEAN. PS 5.1 trap hit again: an inline -m here-string was split into args ('not something we can merge'); -F <file> works.
- Ruling R23 (T10 review, plan-mandated CSP): the CSP gains `connect-src ipc: http://ipc.localhost` — Tauri 2's documented minimum; without it the first invoke on Windows is CSP-blocked and falls back to postMessage with a console warning — if wrong, one directive that names no network host.
- Ruling R24 (T10 review): the app crate is held to ZERO NEW warnings for the rest of plan 1; the spike's pre-existing `.rsrc merge failure: multiple non-default manifests` linker line (fix path in app/README.md: tauri-build WindowsAttributes::app_manifest) is a task of its own before the console ships — if wrong, a cosmetic manifest warning survives to the friend build.
- Task 10: review round 1 → Approved, 1 Important (plan-mandated, R23) → fix round 1 dispatched
- Task 10: minor (deferred): --vault with no following arg falls through to the cwd branch; --vault=path not accepted
- Task 10: minor (deferred): relative --vault stored raw — canonicalize() would make it cwd-independent
- Task 10: minor (deferred): build_state_value mutates the payload after revision was stamped and does not recompute it (deliberate? comment it)
- Task 10: minor (deferred): poisoned lock bricks the window; into_inner() would recover (unreachable under panic=abort)
- Task 10: minor (deferred, brief-mandated): mark_seen's error branch omits seen_at (two shapes)
- Task 10: minor (deferred): main.rs lost the spike's module doc explaining windows_subsystem
- Task 10: minor (deferred): test app-data scratch dir qo-console-appdata not pid-scoped
- Task 10: ⚠ for T12: the shell injects topline.console_build/engine_newer into the payload — keys the CLI's frozen references do not carry; confirm the page contract there
- Facts for T11 (from the CHOSEN mockup): `.kb` rules at :80-81 AND `nav .kb { display: none; }` at :226 inside a media query (the test forbids any `.kb`); `p.foot` :200-201; the peak label `content: "8.7h"` :178; the runs row selector in the mockup is `.run { grid-template-columns: 46px 62px … }` (:190) — the test asserts `.ln.run` + `46px 62px`, i.e. the carried rule must be spelled `.ln.run` (the grid-collision fix), not the mockup's bare `.run`.
- 2026-09-03 14:19 BLOCKER: Windows Smart App Control (state On) began blocking the rustup toolchain's cargo.exe — CodeIntegrity 3077 'did not meet the Enterprise signing level requirements' (Policy {0283ac0f-fff1-49ae-ada1-8a933130cad6}), os error 4551 on every cargo invocation; rustc runs. cargo built/tested fine until ~14:19. Task 10 fix 1 staged; Ruling R25: config-only fix is covered by a JSON parse now, `cd app; cargo test` owed before Task 11's first commit. Tasks 11–15 and the 18:00 runner's cargo build are blocked until Quinn lifts it (SAC has no per-app allowlist; turning it off is irreversible without a Windows reset).
- Task 10: fix round 1/5 committed as 3ae09b7 (CSP connect-src; R25: JSON-parse evidence, cargo test owed) — the original implementer stalled on a blocked cargo call; a fresh agent committed the staged fix
- Task 10: fix round 1/5 (1 addressed, 0 open; commits 83ffbc8..3ae09b7)
- Task 10: complete (commits 6499626..3ae09b7, review clean after 1 fix round) — DEBT: `cd app; cargo test` on 3ae09b7 owed before Task 11's first commit (R25; cargo blocked by Smart App Control since 14:19; Quinn chose to turn it off)
- Task 11: dispatched 2026-09-03 15:09, implementer sonnet, BASE 3ae09b7 — authoring proceeds without cargo; tests + the owed Task 10 app test run when cargo returns; BLOCKED if still blocked after 20 min of waiting
- 2026-09-03 19:27 cargo runs again (Smart App Control turned off by Quinn); Task 11 resumed to run the owed app test + static tests and commit
- 2026-09-03 18:00 slot: NO line. Laptop asleep (wake events 15:04 and 19:27); Task Scheduler shows LastRun 13:04, NumberOfMissedRuns=1, NextRun 09-04 12:00 — StartWhenAvailable had not caught up by 19:30. Class: missed (asleep) — counts no, resets no; per C12 not diagnosed inside 20 min of wake; re-check ~19:50. Count stays 1 of 14 (day 1). Cloud 15:10 ok.
- Task 11: DONE a2bacb5 (owed Task 10 app test green: 3 command + 3 static tests, zero new warnings); review dispatched. Note: the 4 Instrument Sans and 3 JetBrains Mono weight files are byte-identical per family (Google served the variable font) — plan-mandated seven files; the reviewer may weigh in
- Task 11: minor (deferred): two token comments name the wrong usage sites (--s1c/--s2c)
- Task 11: minor (deferred): per family the weight files are byte-identical variable fonts (~150 KB duplicate) — one variable file per family with a font-weight range is a follow-up
- Task 11: complete (commits 3ae09b7..a2bacb5, review clean)
- Ruling R26 (T12): `mark_seen` is invoked AFTER the first `state` resolves (chained on `route(...)`), not concurrently — the brief's prose says 'once after the first state' but its code fires both at once and the lock does not order them — if wrong, the first paint's delta window could start a poll early.
- Task 12: dispatched 2026-09-03 19:36, implementer sonnet, BASE a2bacb5
- CORRECTION: the 18:00 slot CAUGHT UP at 19:34 (7 min after the 19:27 wake; my 19:30 read was too early): `2026-09-03 19:34 dual python-live git=a65efc1 bin=bfc75341905c build=ok CLEAN files=244 lint=0/0 exits 0/0 11s`. build=ok → cargo was already unblocked, so the runner REBUILT the engine: NEW bin=bfc75341905c = the merged engine (tasks 1–9 + build.rs) — and the live dual run is CLEAN: the read-only additions changed nothing rank writes. COUNTS: 2 of 14, day 1 complete (both slots).
- Task 12: DONE_WITH_CONCERNS 1b3c30a (8/8 app tests). INCIDENT: during the 'look' step the implementer sent synthetic Ctrl+Minus/click input to zoom the webview; it landed on Quinn's foreground Chrome, and one full-desktop CopyFromScreen capture showed an unrelated private search. The three full-desktop PNGs (console-today-zoom*.png) were deleted from the scratchpad at 19:53; console-today.png (PrintWindow by HWND, console content only) kept. Ruling R27: no subagent sends synthetic input to the desktop, ever; screenshots are PrintWindow-by-window-handle only (never full-screen CopyFromScreen) — this supersedes T14 step 4's 'System.Windows.Forms.Screen + CopyFromScreen'. Added to machine-rules.md.
- Task 12: layout finding for T14: the mockup's fixed 1560px grid overflows a 1440-logical-px display at 100% zoom (console.css, pre-existing from the mockup)
- Ruling R28 (T12 review, plan-mandated paint draft): a route is never a reorder — `route` resets `current.state`; the brief's draft blanked Today when returning from a horizon list — if wrong, one extra full repaint per navigation.
- Ruling R29: the F19 hold freezes ONLY the ordered lists (must_do/recommended on Today, the list on horizon views); rails, nav, topline, day, deck, ahead, coming up, closed, runs always repaint — if wrong, a held order also holds the runner health, which is the failure the brief's draft had.
- Ruling R30: the hold applies to the four list views too (F19 says a poll never reorders under the cursor, not 'on Today only').
- Ruling R31: the deck header reads 'clear' only when pending == 0 AND events_in_digest == 0 (the header-level twin of R19).
- Task 12: review round 1 → Needs fixes: 4 Important (R28–R31) → fix round 1 dispatched
- Task 12: minor (deferred): openDrawer has no .catch; mark_seen fires even when the first state failed
- Task 12: minor (deferred → T14): console.css .day .allday / .day .commit are orphaned — #allday and #commitments are siblings of .day in the markup
- Task 12: minor (deferred): meter tooltips index spill blocks by item (one item spans many blocks)
- Task 12: minor (deferred): renderRecommended patches generated HTML by string replace — a takeHours arg on rowHtml removes the class
- Task 12: minor (deferred, F7 gap): State.unreadable never rendered; list.unreadable only on 'all' — an unreadable note is silently absent from Today
- Task 12: minor (deferred): drawer closes only by ×; deck cards' data-id inert until plan 2; .b double duty in the click handler
- Task 12: fix round 1/5 committed d201f00; scoped re-review dispatched
- Task 12: fix round 1/5 (4 addressed, 0 open; commits 1b3c30a..d201f00)
- Task 12: minor (deferred, out of scope): orderOf reads state.must_do.groups unguarded
- Task 12: complete (commits a2bacb5..d201f00, review clean after 1 fix round)
- Ruling R32 (T13): the reviewed regeneration diff of the three references is the added `texts` object PLUS the consequent `revision` value (the hash covers the payload) — nothing else may differ; the commit message says so.
- Task 13: dispatched 2026-09-03 20:23, implementer sonnet, BASE d201f00
- Task 13: DONE 4868a7e (references regenerated under R32; surface_oracle RED→GREEN; full engine gate clean); review dispatched. Env note: a stale proc-macro DLL hit a residual Application Control block after the implementer's cargo clean — deleted and rebuilt
- Task 13: minor (deferred): Texts doc comment says 'three sections' but has four fields (offline)
- Task 13: minor (deferred): verdict hardcodes 'Nothing active.' instead of EMPTY_TEXT.active (pinned by the new test)
- Task 13: minor (deferred): JS forbidden list omits 'Nothing here.' and 'No free block today.'; nothing asserts State.texts by name; test panic message points at state.*.empty_text
- Task 13: complete (commits d201f00..4868a7e, review clean)
- Ruling R33 (T14): the layout fixes the eight-viewport pass demands (the 1560px fixed grid overflowing 1440px; the orphaned .day .allday/.commit selectors) are committed in Task 14 — app/static/console.css and, if needed, index.html join its commit list; the plan's step 3 says 'Fix CSS until all eight are ok'.
- Ruling R34 (T14): the console is desktop-only with a window minWidth of 820 — console-shots.py's exit-1 gate applies to viewports ≥ 820 px wide; the 390×844 phone viewport is shot and reported, not gating — if wrong, a phone layout nobody ships blocks the plan.
- Task 14: dispatched 2026-09-03 20:42, implementer sonnet, BASE 4868a7e
- Task 14: DONE 46ce0e2 (8 viewports ok, 390 informational per R34; three visual defects fixed: coming-up .rt overlap, orphaned .day selectors, drawer over nav at 820; the 1440px overflow did not reproduce headless — DPI capture artefact suspected); review dispatched
- Ruling R35 (T14 review): R33(a) is closed only with real-exe evidence at the desktop breakpoint — the window is sized programmatically (SetWindowPos on our own HWND, not input) to 1280 and 1440 logical and captured by PrintWindow; overflow → fluid grid fix; no overflow → recorded in the anatomy — if wrong, one extra capture pass.
- Task 14: review round 1 → Approved with 1 Important open (R33(a) unverified on the device) → fix round 1 dispatched (R35)
- Task 14: minor (deferred): console-shots.py lacks argv/fixture guards and an explicit srv.shutdown()
- Task 14: fix round 1/5 committed 54ae7e4 (docs-only: no overflow at 1280/1440 logical on the real exe, DPI scale 2.0; R33(a) closed with PNG evidence); scoped re-review dispatched
- Task 14: fix round 1/5 (1 addressed, 0 open; commits 46ce0e2..54ae7e4)
- Task 14: complete (commits 4868a7e..54ae7e4, review clean after 1 fix round)
- Task 15: dispatched 2026-09-03 21:28, implementer sonnet, BASE 54ae7e4
- Task 15: DONE ad8bad5 (runner builds app/ too — dry-run line updated; HANDOFF/CLAUDE.md/plan status/spec §16); review dispatched
- Task 15: review round 1 → Needs fixes: 2 Important (CLAUDE.md says two commands — three; console build block unguarded on app\Cargo.toml — a missing dir masks as ok) → fix round 1 dispatched
- Task 15: minor (deferred): CLAUDE.md console section ~12 lines vs 6–10 asked; HANDOFF's cutover facts went into a new paragraph rather than the ▶ THE NEXT ACTION one
- Task 15: fix round 1/5 committed 5a1f161; scoped re-review dispatched
- Task 15: fix round 1/5 (2 addressed, 0 open; commits ad8bad5..5a1f161)
- Task 15: complete (commits 54ae7e4..5a1f161, review clean after 1 fix round) — ALL 15 TASKS COMPLETE; final whole-branch review dispatched (fable) over 4b76c00..5a1f161 (25 commits, 47 files)

## Final whole-branch review (fable, 4b76c00..5a1f161): Ready WITH FIXES — 0 Critical, 7 Important (2 JS fixes before merge; 3 documentation-honesty items; runner reorder; a UTC stamp for plan 2), minors incl. console-shots serving the repo root (secrets) — one fix wave dispatched
- Ruling R36 (final review #1): F22's `engine_newer` compares the shell's SHA with the in-process engine's — the same build, a placeholder; plan 2 compares CONSOLE_BUILD with `git rev-parse --short HEAD` in the vault (None for a non-git vault). HANDOFF says so now — if wrong, a warning that never fires until plan 2.
- Ruling R37 (final review #4): F18's window decays 60 s after launch (mark_seen at first paint); plan 2 stamps at the END of the look (blur/close) or holds the first-paint delta for the session. HANDOFF says so now.
- Ruling R38 (final review #6): §12's 'error dialog' on a missing vault is a stderr line + exit 2 under windows_subsystem=windows (silent from a double-click) until the profiles picker; recorded as a deviation in HANDOFF — a MessageBoxW is the cheap interim if Quinn wants it.
- Ruling R39 (final review #5): the runner's console build moves AFTER git push, off the run's critical path (a cold/LTO build could push the start record past the 20-min grace); its result goes to its own log line; Kill() is not tree-kill — noted. HANDOFF: build app/ by hand in the main checkout after the merge.
- Ruling R40 (final review minor): console-shots.py serves a TEMP dir holding app/static + the fixture, never the repo root (config/ingest.yaml holds a live token); /shots/ git-ignored.
- Final review: closed by verification L76 (coming_up hunk — rename only) and L204 (done by R33(b) in Task 14).
- PLAN 2 first engine touch (bundle, one reference regeneration): runs_panel.expected.due UTC→local_stamp (#7); -0.0 empty sums; runway extraction from render (L92); L217; plus L134, L131, L147, L149, L121 must land in plan 2.
- 2026-09-03 22:10 CORRECTION: Smart App Control state is STILL ON (Get-MpComputerStatus); Code Integrity 3077 blocks at 22:09 (cargo.exe loading a fresh file under the worktree) and 22:10 (bash.exe) — the 14:19 cargo block cleared by itself at ~19:27, mechanism unknown; fresh unsigned binaries are still blocked intermittently (the fix-wave agent hit it on app/target/debug; delete+rebuild clears). HANDOFF/CLAUDE.md said OFF — false; docs-only correction dispatched to the fix-wave agent. Quinn's toggle has not taken effect; ask in the closing message. Runner risk: a slot's release build may BUILD-WARN at random (run proceeds on the previous binary).
- Final fix 2 (docs): 95e919f — HANDOFF/CLAUDE.md now say Smart App Control is On (measured 22:10), the block cleared by itself, fresh binaries still intermittently blocked
- 2026-09-03 22:17 Quinn turned Smart App Control OFF (verified: Get-MpComputerStatus → Off). Docs correction #3 dispatched (HANDOFF/CLAUDE.md).
- Final fix 3 (docs): 2caa5bc — SAC turned off 22:17 recorded in HANDOFF/CLAUDE.md
- Final fix wave re-review: A–F all ADDRESSED, no new Critical/Important. Residual minors → plan 2: must_do.empty_text can show beside unreadable rows (engine: add unreadable.is_empty() to the condition; moves a reference); the state.unreadable test guard is satisfied by a comment; the runner comment 'incremental (seconds)' is optimistic (HEAD moves every run → rustc-env changes → LTO relink, minutes); Set-Content -Encoding utf8 BOMs the status file (nothing reads it yet); mark_seen is never retried after a failed first poll (safe direction; F18 redesign).

## PLAN 1 CLOSED 2026-09-03 22:30 — final review clean after one fix wave (f022ecd) + two docs corrections (95e919f, 2caa5bc). HEAD 2caa5bc. Second merge to main awaits Quinn's go; then prime app/ release build in the main checkout by hand.
