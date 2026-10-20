# Syllabus class times (lane L5): build plan, re-planned around the registrar

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: r10 (2026-10-20), revised after the r9 delta review (FAIL: 0 critical, 1 important, 3
minor; every r8 finding closed, the eleventh place-10 change and the 8 by `first_proposed_at`
held); delta review pending.** r10 measures the horizon from the charge day: on the vault's first
day, "never once past" and the 14/21 days count from day 2, so an item due on day 1 plans no card
and one due on day 2 plans a held card that is still live when it wakes (I-1, with a new T4dt (A)
case). Q-B's reason for item cards' `expires` is corrected, and "as Q41 orders the 8" is replaced by
`defer_over_budget`'s real order. Expired item history still blocks the key by default; the
alternative (it unblocks under the twin's day rule, a thirteenth place-10 change since r10b) goes to Quinn in §7
with the cost of each (M-1). `PlannedCard` names `expires` and `first_proposed_at` (M-2). "Reaches its
day with no card only if every run is full" is bounded for items that enter the horizon late, in the
plan and in place 10's spec text, and the charge-day sizing is cited as new, modelled on
`emit_checks` (M-3). §10's "Round 10" table says where. The next report goes beside the others as
`-r10`.
r10a (2026-10-20) closes the r10 delta review's two minors (PASS: 0 critical, 0 important, 2 minor).
The horizon is inclusive by controller ruling (charge day + 14, an exam + 21, is inside; + 15 / + 22
is not), with one T4dt (A) case per kind (M-1). The course listing is T4d's and uses the charge-day
horizon: `Plan` gains `listing`, T4dt (A) pins its `later` boundary, and `rank`'s hook (T4d'a)
writes it into the state file through T2c's one writer (M-2, on the controller's go). §10's
"Round 11" table says where.
r10b (2026-10-20) answers the r10a delta review (FAIL: 0 critical, 2 important, 4 minor). `rank`'s
write re-reads the file and skips one `syllabus-add` replaced mid-run (I-1); `past` joins version
1's codes as place 10's twelfth change (I-2); `Filed` follows G9, the writer's two forms and
`item_outcomes` are pinned, same-day items have an order, and `judge` empties stale outcomes
(M-1 to M-4). §10's "Round 12" table says where. The next report is `-r10b`.
r10c (2026-10-20) closes the r10b delta review's five minors (PASS: 0 critical, 0 important, 5
minor): `judge` re-reads a state whose SHA is not its copy's; `ITEM_OUTCOME_CODES` freezes the six
codes; an item due today is worded as due today; one code per item by a stated precedence; the
leftovers. §10's "Round 13" table says where.
r9 (2026-10-20) answered the r8 delta review (FAIL: 0 critical, 1 important, 2 minor). It held the
filer to the 15 as well as the 8: `plan` files at most the smaller of what is
left of the day's 8 and `max(0, budget - count_proposals_created(vault, charge_day))`, where the
charge day is today, or day 2 on the vault's first day (new, modelled on `emit_checks`' sizing at
`cli.rs:873`, which counts today only). So no syllabus card takes a day over the 15. T4d'a's and Round 8's claim that sizing which already existed
did this is corrected, and T4dt's sum-of-15 case is restated: (ii) gives 10 + 5 = 15, and its held
variant shows overflow from held mail cards, as today (I-1). Item cards' `expires` is stated (a task
or exam card expires on its item's day; amend cards stay `null`), and the held variant names the
three cards `defer_over_budget` snoozes (M-1). The hook's cost is written out concretely and goes on
Quinn's list in both T4d'a and §7 (M-2).
r8 (2026-10-20) answered the r7 delta review (FAIL: 0 critical, 2 important, 4 minor). It named the
filer's pure plan (`syllabus_cards::read_inputs` and `syllabus_cards::plan`), so T4d's
Done is the plan cases only and the filer's `rank`-level cases move to T4d'a (I-1). It reads G11's
"no longer gives its value" one way for both halves: an absent value (a dropped or undated item, a
grading part that fails validation) is not *unsupported*, by Q15's reasoning; only a present, valid,
different value is (I-2, a controller reading reported to Quinn in Q-B's costs and §7). It pins the
hook between `process_approvals` and `remaining_budget` (`cli.rs:430`-`:439`), drops r7's "latent
double approval" claim, scopes P2 (f) and (i), and moves `Withdrawal`'s doc comment and the
caller-built reason into T4e and place 10 (M-1 to M-4).
r7 (2026-10-20) answered the r6 delta review (FAIL: 0 critical, 4 important, 4 minor) and Quinn's two
rulings of 2026-10-20: G11 (a card the current syllabus no longer supports is withdrawn quietly on any
desktop) and G10's one-time card accepted. It fixed `withdraw_syllabus`'s signature in T4e, took
withdrawn cards out of Must do, made G8 wait for room and gave G2 a file-name tie-break.
History: r6 rebuilt the grading card on Quinn's simplification of 2026-10-19, which retired device
ownership (G1), the human/agent split (G3), the re-base (G4) and Q-L. r4 reworked the plan for Quinn's
Q-B ruling and r5 reworked its *value moved* mechanism; r4 to r9 each failed delta review.
r3 passed review (0 critical, 0 important, 5 minor; minors applied); round 1 failed (0 critical, 6
important, 4 minor) and r2 answered it; round 2 failed (0 critical, 2 important, 6 minor) and r3
answered it.
Nothing runs until the controller has answers to §3's open questions (or has deferred them) and P0's
preconditions hold (PR #19 merged to `main`; the email branch merged). The reports are
`docs/reports/2026-10-19-l5-plan-review-r1.md`, `-r2.md`, `-r3.md`, `-r4.md`, `-r5.md`, `-r6.md`
and `docs/reports/2026-10-20-l5-plan-review-r7.md`, `-r8.md`, `-r9.md`, `-r10.md`, `-r10a.md` and `-r10b.md` (the last nine are delta reviews). Every
finding's answer is in §10, one table per round.

**Date:** 2026-10-19 (session 5, the friends-MVP push). **Lane:** L5 of
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5 ("engine commitments.rs/ingest.rs/
cli.rs, **approvals.rs (contract)**, cloud judge kind, lms_link.rs, console.js"; agents planner,
contract-engineer, implementer, cloud-engineer, console-ui).

**Authority, most recent first.**
- The signed spec `docs/specs/2026-10-05-syllabus-class-times-design.md`, revision 2 with the
  amendment of 2026-10-18 (Quinn signed 2026-10-19: Q4-Q28 as recommended, Q19 restated, Q15, Q49,
  Q50 (a); §17 Q29-Q47 as recommended; Q14 ruled; Q48 conditional). Cited by section heading.
- Quinn's rulings in `.superpowers/sdd/2026-10-18-session-5/progress.md`: row 31 (PR #19 merges
  **before** L5; L5 is re-planned around the registrar), row 39 item (4) (a per-source table in
  `commitments.rs` from #19's B3; L5 adds a `syllabus:` row), rows 117-122 (Q50, Q15, Q49, the
  signature, §17, Q14).
- `docs/plans/2026-10-18-p3-registrar-close-out-plan.md`: §4 B3f (the table), §5.3-§5.4 (the
  `main.rs` and `console.js` orders), §6 (the L5 delta, now folded into the spec).
- The session-5 handoff §5's conflict map; `docs/plans/2026-10-19-l6-runtime-removal-plan.md` §5
  (L6 owns `app/src/main.rs` and `app/src/commands.rs` until it merges).
- The cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`), which wins over any older text.

**Goal.** For every course, Knowlu finds or is given the syllabus and turns what it says into ordinary
proposals on existing paths: class and lab rows (then `commitment-check` cards), exam, quiz and
assignment cards, one grading amend card per course (weights, scale, late rule), and office hours and
policy quotes shown on the course. Nothing is written as the student without a tap; `rank` never calls
a model; the registrar outranks a syllabus, which outranks a calendar.

**Architecture (the spec's, unchanged).** Fetch on device (step 1 in the wizard's LMS window; Offer A
in its own Simple Syllabus window; Offer B a file or paste). `syllabus-add` (engine, ungated, offline)
keeps the copy and screens it into bounded parts. `judge` (gated, exit 0) sends only the unsettled
parts to a new pinned `syllabus` kind and writes only `state/syllabi/<slug>.json`. `commitments` reads
its meetings as a new series source (one `syllabus:` row of #19's per-source table). `rank` files task,
exam and grading cards from the extraction, deterministically. `surface` shows the rest.

**Code read for this plan (2026-10-19), read-only:** the main checkout (`main` after #39 and #40,
before the email merge), `.claude/worktrees/p3-registrar` (#19, phase B not started: no
`.superpowers/sdd/2026-10-18-p3-close-out/` ledger exists, so B1-B9a are all ahead) and
`.claude/worktrees/email-forwarding`. Facts the briefs rely on:
1. **#19's seam is not built yet.** On #19's head, `precedence` is a private function returning
   `(u8, &str)` with tiers registrar 0, `google:` 1, else 2; its four readers are `by_key`, the
   collapse in `proposals`, the aged-out sort in `refresh_series` and the swap pick in
   `detect_changes`. `SERIES_KEY_PREFIXES` is `["gcal-series:", "ics-series:", "registrar:"]`.
   The per-source table is B3f's; P0 reads what B3f actually left before T4a starts.
2. **No #19 test calls `precedence` directly** (grep on the branch). The two tests the spec names
   assert orders. Q48's case is unlikely but P0 re-checks after B3f.
3. **`AMENDABLE_FIELDS` has two readers besides `approvals.rs`.** `sync.rs`'s conflict step treats a
   field as *cardable* when its folder is in `AMENDABLE_FOLDERS` and the field is in
   `AMENDABLE_FIELDS` (the "a carded field is withheld from `apply`" step), and
   `engine/tests/sync_contract.rs` reads the list. `approvals.rs`'s own test
   `amendable_fields_is_exactly_the_nine` pins it. On its own, T4e's widening would turn a two-desktop
   conflict on a course note's `grade_weights`, `grade_scale` or `late_policy` into a sync card.
   Quinn ruled against that (Q-B, 2026-10-19). The one closure that decides it is `cardable` in
   `sync::apply`'s step 5 ("A carded field is withheld from `apply`", `sync.rs:2036` today):
   `card_folder && crate::approvals::AMENDABLE_FIELDS.contains(&field)`. All three of its uses
   (withholding at :2040, the card's change set at :2100 and the carry at :2139) read that one
   closure. A field it calls uncardable takes the branch at :2043-2046: reconcile's rule (the later
   `ts` wins), the supersede record kept, and the warning "sync: {path} — both machines set
   `{field}`; the later write won and the other is in the journal". `sync_contract.rs`'s
   `a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so` and
   `a_conflict_this_device_won_writes_nothing_and_files_no_card` pin that branch today, on fields
   outside `AMENDABLE_FIELDS`. T4e therefore edits `sync.rs` (Q-B; T4e's brief).
   **What the syllabus cards' rules rest on (r6, r7; email branch, re-read at P0 item 6).**
   - *No reason reaches a vault.* `withdraw_stale` and `withdraw_asks` ask their reason function only
     whether to withdraw. They stamp `status: superseded` and archive through `write::delete`
     (`approvals.rs:1880-1886`, `:1924-1930`). The records carry the run's `via`, from the closed
     `journal::VIAS`, and the existing ops (`journal.rs:30-34`).
   - *History counts every status.* `approvals::existing_source_uids` collects every `source_uid` in
     `approvals/` and `archive/`, whatever the card's status (`:1341-1357`).
   - *Ids are random and travel.* A card's `id:` is five random bytes (`ids::new_id`, `ids.rs:70-76`).
     `write::create` always stamps one (`write.rs:483-496`), and sync carries the note's text as it is.
   - *A pulled note never overwrites a file at its path* (`sync.rs:2300-2313`). So two desktops'
     cards must never share a file name, or each keeps its own and the other's records reach the wrong
     card.
   - *The pending count.* An approved amend card that the stale check refuses goes back to `pending`
     but is not counted pending (`approvals.rs:1695-1705`). `ApprovalsResult::amendments_pending`
     lists the amend cards that were counted.
   - *What a withdrawal returns (r7).* `Withdrawal` holds the withdrawn stems, an on-disk `pending`
     count and warnings (`approvals.rs:1944-1952`), and `withdraw_asks` calls its reason function in
     place (`:1924`). So T4e fixes `withdraw_syllabus`'s signature, and no later task edits the file.
   - *Must do reads the amend list (r7).* `render.rs:379-382` builds Must do from
     `amendments_pending` (urgency `increases`), so a withdrawn amend card must leave that list.
   - *The day's count includes pulled cards (r7).* `count_proposals_created` counts every approval
     card in `approvals/` and `archive/` whose `proposed_at` is today (`approvals.rs:1144-1162`),
     whichever desktop filed it.
   - *Who writes the three fields.* Nothing in the engine writes `grade_weights`, `grade_scale` or
     `late_policy` today (a search finds no use outside `docs/`). After L5, their writers are the
     student (a console write, or a hand edit that `passes::detect_external` journals), an approved
     grading card (`agent:approvals`), and sync's echo of either. That is the premise of Quinn's
     simplification (Q-B).
   - *`comparable`'s callers.* It has three callers in the file: `apply_amendment`'s two checks
     (`:803-827`) and `reassert_rejected_sync_card` (`:1507`). The last sees only sync cards, and
     under Q-B no sync card names the three fields, so G6's change cannot reach it.
4. **The email branch rewrites much of `approvals.rs`** (`withdraw_stale` and `withdraw_asks` move by
   about 260 lines; mail cards and the first-day hold). L5 is cut after it merges, so T4e builds on it.
5. **`onboarding::pick_file` takes one `extension: String`** today; T8b widens it as the spec says,
   and L6 keeps it in both handler lists.
6. **Judge routes are one function directory each** (`judge-task`, `judge-event`, `judge-email`), so
   `/judge-syllabus` is a new `cloud/supabase/functions/judge-syllabus/`.
7. **The `kind in ('task', 'event', 'email')` checks** sit in `20260911000100_judgment_service.sql`
   (five) **and** `20260911000700_eval.sql` (two, the eval tables). The spec names only the first file.
8. **L4's page draft** (`docs/notes/2026-10-19-l4-privacy-page-draft.md`, "Q2. The syllabus lines
   before L5's code exists") carries §6.1 items 2-5 with "(Knowlu does not read syllabi in this
   version …)" and recommends that removing it be material: a `PRIVACY_VERSION` move in `account.rs`
   and a re-prompt, as L5's go-live (§3 Q-A).
9. **The mail window's runner and its `Page` seam** exist only on the email branch
   (`app/src/mail*.rs`); T10b follows what is on `main` after that merge.
10. **The migration-stamp rule** is §2.1 of `docs/plans/2026-10-18-first-day-allowance-plan.md`
    (heading "The migration's stamp: the shared rule, and what it means for L3", items M1-M8). It is at
    r8 with a final minor pass in progress (session ledger, the STAMP rows). This plan cites it and
    does not copy it. Five of its points shape L5's order (M7, M8):
    - a lane ahead is **cleared** only once its migration is on staging **and** on `main`;
    - a lane that pushed from its branch and has not merged holds every lane behind it until it merges
      or is withdrawn;
    - the controller withdraws a lane on its own only when the lane's SQL fails the staging proof (any
      other withdrawal asks Quinn), by M7's withdrawal steps (five at r8);
    - a lane merges its migration only after a dry run on its lane branch, rebased on `main`, lists its
      own version alone, or nothing once it has pushed from that branch;
    - "every lane ahead" is M1's order (email, L3, L8) **plus any lane that overtook L5 by a recorded
      slip** (M8). If L5 is ever recorded as slipped, L1 can be ahead of it.
11. **The `judgments` row keeps the model's output.** `judgments.fields` is `jsonb`
    (`20260911000100_judgment_service.sql`, the `judgments` table). For the `syllabus` kind that would
    be extracted syllabus text (meetings, items, office hours, policy quotes) in a hosted database,
    which Q-E rules out for real seeds and §6's minimization rules out for students. The writer is
    `fieldsOf` in `cloud/supabase/functions/_shared/judge_pipeline.ts` (about :81-92 today). It
    stringifies every verdict field except `confidence`, `why`, `importance_reason` and `title`, then
    merges `featureMap(kind, item)` last. `featureMap` lives in `_shared/judge_rules.ts`, not in
    `judge_pipeline.ts`. For any kind but `task`, `featureMap` falls through to `organizer`, `source`,
    `series_uid` and the title prefix. A `syllabus` verdict's meetings, items, office hours and quotes
    would land in `judgments.fields` unless `fieldsOf` has a `syllabus` arm. T6b adds that arm. L3 and
    L7 also edit `judge_pipeline.ts` (§4.2). The model for T6b's test is
    `judge_pipeline_test.ts`'s "a body token reaches no judgment row, and neither does a title beyond
    its prefix" (the C2 plan's `a_body_token_reaches_no_judgment_row`).
12. **The eval runs the pipeline in-process.** `cloud/eval/run_eval.ts` calls `judge()` from
    `judge_pipeline.ts` with stubbed caps, rules and log, reading the provider key from the
    environment; it needs no deployed route. T7b uses that shape on the controller's machine, so no
    staging push happens before P8.
13. **L4's scheduler gate** (L4 plan §4.5, §5 Q-GATE): `Older` skips the whole `judge` step; `Accepted`
    and `Unknown` leave the slot as today. `Unknown` is "no cache, an unreadable cache, or another
    account's cache", which a failed first read leaves in place. A syllabus pass inside a plain `judge`
    would therefore run under `Unknown` (finding 3's gap; §3 Q-K and P3 close it). L4's gate block
    runs only for an `Entitled` vault, and there it calls `account::refresh_privacy` once when the
    cache is not `Accepted`, before deciding (L4 plan §4.5, "The check"). So a slot's gate value is
    the one read **after** that refresh. Under Q-GATE's fail-closed alternative, `Unknown` would hold
    back what `Older` does.
14. **L4's change rule names syllabi.** L4's draft §3.11 (applied by L4's T10) ends the "Changes to
    this policy" section, under its Q3 (ii), with: "Reading a kind of material this page says Knowlu
    does not read, such as syllabi, is never a smaller change." L4's T9 pins that sentence in
    `engine/tests/site.rs` (draft §4.3 "L4.1", "the Q3 (ii) or (i) sentence"). Once P1 takes the
    "does not read syllabi" parenthesis out, the example points at material the page now says Knowlu
    reads, and the page contradicts itself. P1 therefore rewords that sentence and its pin
    (r2 finding 1). Under Q3 (i), the sentence is the broad form with no example, and P1 leaves it
    and its pin alone.

Every `file:line` in the spec is pre-merge; every brief here names functions, tests and headings, and
the agent re-confirms locations on the lane branch.

## 1. VISION and the two overriding rules

- **"What's next?"** A midterm five weeks out, a lab on a day the calendar never showed, a late window
  still open: each is an answer the feed alone cannot give (spec §1). Every surface this lane adds
  names its outcome (spec §8), so "silence is never ambiguous" holds.
- **Commitment 1 (`rank` never calls a model).** Extraction is a `judge` pass that writes only
  `state/syllabi/`. `rank` files cards from that file and reads nothing else new; T4d's tests run
  `rank` with no network.
- **Commitment 4 (typed time).** Syllabus classes and labs are proposed and confirmed `hard` through
  the existing confirm path; an exam card writes a one-off hard `exam` only on Approve. Syllabus series
  stay out of `instances_map` so a confirmed note blocks time by `meets` (spec E5).
- **Commitment 5 (who started a change).** A syllabus is "Knowlu noticed": it proposes and waits. The
  registrar applies and lists (#19); the LMS feed and `coursework` are authorities a syllabus date never
  overrides (spec §4.6, §16.5). Office hours and policy quotes are shown, never proposed.
- **"Knowlu never invents a due date."** A week number resolves only from a date the syllabus or its
  Simple Syllabus term gives (Q37); otherwise "needs a date".
- **Fetch on device, think in the cloud; minimization.** Only the screened, bounded excerpt
  (13,500 characters with the policies part, Q38) and the course note's opaque `id:` travel.
- **CLAUDE.md invariants this lane must keep.**
  - `commitments/` holds only confirmed notes: syllabus meetings live in `state/calendar-series.json`
    and local cards until the student confirms; an exam note is written only by an approved card.
  - `LOCAL_CARD_KINDS` stays `["commitment-ask", "commitment-check"]`. The tripwires
    (`sync_keeps_every_local_card_kind_local`, `the_servers_note_path_rules_name_every_note_folder`)
    and every existing sync test are not edited, and none is affected. `sync.rs` changes in one place
    only, under Q-B's ruling (T4e): the three grading fields stay out of `cardable`, which names no
    card kind and no folder. `syllabi/` and `state/syllabi/` never sync (P2 pins `sync::is_note_path`).
    `ids::NOTE_FOLDERS` (`ids.rs`, contract list) is not edited: `syllabi/` is not a note folder.
  - Judge once, re-propose freely: every agent write is `agent:knowlu.syllabus` (filer) or the
    existing ingest actor (adoption); a field the journal shows the student set is never re-set, and a
    syllabus change to it is an amend card (spec §4.5, §4.6, §16.3). The grading card is stricter
    (Quinn, 2026-10-19). Once a grading field no longer holds the card's `from`, the card is withdrawn
    as answered, and that set is never proposed again. Only a new grading hash files a new card (G7,
    G9). This is spec §16.3's own rule: "never proposed back over unless the syllabus itself changes
    (a new hash)". A card the current syllabus no longer supports is withdrawn and not re-filed
    (Quinn, 2026-10-20; G11), so a withdrawal never becomes a re-proposal.
  - Every note write goes through `write`, journal first; the extraction file goes through
    `ledger::dumps_value`; no note is parsed and re-dumped.
- **Rule 1 (no single-user assumptions).** The Simple Syllabus host is a curated data field, `None`
  on every row until Q39 clears UA's (T10a, T10d). No school, course, room, person or account appears
  in code, tests or test names; examples use invented rooms. Real syllabi never enter the repo (§2).
  No rule reads a machine's identity. The grading card's rules read synced data (cards, ids, the
  course note) and the local extraction, never a hostname (r6 retires G1 and Q-L).
- **Rule 2 (frozen references).** No task opens `engine/tests/fixtures/**` or a `surface-today-*.json`
  for writing. A vault with no `state/syllabi/` gives byte-identical `rank` and `surface` output; T4dt,
  T16t and T17t compare, never regenerate. Each task touching `commitments`, `rank`, `ingest` or
  `surface` ends with `git diff --exit-code main -- engine/tests/fixtures` and the oracle tests green.
- **Contract list.** Two source files change, `engine/src/approvals.rs` and `engine/src/sync.rs`, both
  in T4e and in one commit, by `contract-engineer` at xhigh, reviewed by `contract-reviewer` (Q-B's
  ruling). One new sync-contract test file, P2's `engine/tests/syllabus_sync.rs`, holds the
  `sync::is_note_path` pin and Q-B's last-write-wins cases. Q-A's go-live edits
  `app/src/account.rs` (`PRIVACY_VERSION`, P1). All of these go to `contract-engineer` and
  `contract-reviewer`. A needed `journal::VIAS` value, `write.rs` change or
  edited sync test moves that piece to them too (spec §13's closing paragraph). `engine/src/backup.rs`
  is not on the list; T2c's `BACKUP_FOLDERS` change goes to `implementer` with its two pins (§10, r1
  finding 8).

## 2. Constraints

- **Branch and worktree.** Branch `l5-syllabus`, worktree `C:\Users\danie\GitHub\knowlu\.claude\worktrees\l5-syllabus`,
  cut by the controller from `main` **after PR #19's merge (close-out B8)**, which itself follows the
  email merge (Quinn's ruling, row 31). The work Q-C lists may run before the cut because it touches no
  file #19 or the email branch edits. Absolute paths only; never `cd`.
- **Agents never commit, push or branch.** Each task ends with a file list and a commit message; the
  controller commits with `git add <paths>` (never `-A`) and `git commit -F <file>`. Lane PRs merge
  with a merge commit.
- **Gates.** Under the build-slot guard (four builds at once since ledger row 136):
  `bash C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo build -p knowlu-engine --manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l5-syllabus/Cargo.toml`
  first (the zero-byte sidecar placeholder), then the same with `cargo build --workspace` and
  `cargo test --workspace`, dev profile, in the foreground. **0 warnings is part of green**; the one
  accepted line is the app's `.rsrc merge failure`. Cloud tasks run the `deno` lines `ci.yml` runs,
  copied from the workflow at dispatch, never retyped (L3's review found a hand-typed line lacked
  CI's flags). Every task also runs `scripts/ci/eol-check.ps1`; console tasks run
  `scripts/wizard-check.py`.
- **TDD.** Each build task has a failing-test task before it (`T<n>t`, spec §13). The red run is seen
  and recorded in the lane ledger, one reason per test, before the build task is dispatched. A test is
  never edited to pass; an edit the brief does not name is a stop-and-report.
- **Synthetic data only, in the repo.** Every syllabus, Simple Syllabus document, course, section,
  room and date in a test or under `engine/tests/syllabus_data/` is invented. Rooms read like
  "Example Hall 101". Never `engine/tests/fixtures/**` (spec T1). A committed binary (a DOCX or PDF the
  test cannot build in memory) gets a `-text` line in `.gitattributes` and passes the eol check.
- **Real syllabi (Quinn's Q14 ruling, row 122). The privacy rule, written into S1, S3, T6b, T7a and
  T7b:**
  > Real syllabi (eval seeds, the intake documents, Quinn's own, and any file a spike downloads) are
  > **never committed, never in a fixture, test, spec, plan, note or report**, and are stored only
  > outside every working tree: in Q-E's folder if Quinn keeps one as a seed, otherwise nowhere
  > (deleted when the step that fetched it ends). They never pass through a hosted Knowlu service:
  > not staging's `eval_cases`, not a deployed `/judge-syllabus`, not a function log or a
  > `judgments` row. Two third parties see a real seed during a local eval run (T7b): **OpenRouter**,
  > which every call routes through (`judge_openrouter.ts`), and the one upstream model host that the
  > candidate row's zero-retention pin names (`assertPinnedRoute`: an order of one, no fallbacks,
  > `zdr`). No one else sees one. An agent never prints, quotes or summarises their contents into a
  > transcript, a log, a ledger row or a commit message; it reports counts and pass rates per part
  > only. Synthetic seeds may sit in `cloud/eval/seed/`.
  >
  > **Consent is recorded per seed, off the repo** (Q14: "consented and not public"). Q-E's folder
  > holds a local manifest beside the seeds. For each seed, Quinn records that its owner consented to
  > its use as an eval seed, the date, and that the syllabus is not public. The manifest is as private
  > as the seeds: it is never committed, copied into a worktree, quoted or summarised, and its shape
  > (field names only) is fixed in T7a. T7a's reader reads only seeds the manifest lists with both
  > marks, and refuses the rest by count. A seed without both marks is not a seed.
- **Live sessions are the controller's.** S1, S3 and the live proof run in the main session with
  Quinn (or under the autonomous-proof permission on a scratch profile); shapes, counts and header
  names only, never a course, section, CRN, document code, name or cookie on disk, in the transcript
  or a commit (S2's incident). No subagent holds a session or a token.
- **Tests never leave the machine.** Engine tests are offline; app and engine HTTP tests bind
  `127.0.0.1:0` and serve themselves; the Simple Syllabus window is tested against a fake `Page`.
- **Single-owner files and orders** (handoff §5; §4 here):
  - `app/src/main.rs` and `app/src/commands.rs`: L6's while L6 runs. L5 adds handler lines only after
    L6 merges (T8a, T9, T10c), and adds nothing to `commands.rs` at all (its commands live in
    `app/src/syllabus.rs` and `app/src/syllabus_link.rs`).
  - `app/static/console.js` and its siblings: the one `console-ui` queue, **L6 → L4 → L7 → #19 (B5)
    → L1 R11 → L5 → the redesign** (L6's plan puts L6 at the head). T11a starts only when L1 R11 has
    merged or the controller records that it took a later slot (never after L5).
  - `engine/src/surface.rs`: after L7 merges (T17, then T16s), and so in PR-2 only. PR-1 never
    touches `surface.rs`, so PR-1 never waits on L7 (§10, r1 finding 1).
  - `app/src/scheduler.rs`: P3 only, after L4 and L6 have both merged (L6 rewrites `slot_argv` and
    `judge_plan_for`; L4 adds the privacy gate in `run_slot_with`).
  - `engine/src/approvals.rs`: L5's alone; #19 builds its settlement arms in `commitments.rs`.
  - `engine/src/{main,cli,ingest,lib}.rs`, root `Cargo.toml`/`Cargo.lock`: shared single-owner files;
    within the lane the order of §5 holds, across lanes whoever lands second rebases and the
    controller records which went first.
- **The migration.** T6a's one migration follows the shared stamp rule M1-M8 (order email → L3 → L8
  → L5 → L1), cited by heading in fact 10 above: T6a applies the version on `main` at its dispatch
  (r8 or later), read whole by its heading, never this plan's paraphrase. L5 fixes its stamp at T6a
  (M3), names its file by suffix only in tests (M4: `^\d{14}_judge_syllabus\.sql$`), recounts the shared
  pins on its rebased tree, and the controller runs every `supabase db push --dry-run` and push.
  **L5's migration reaches staging only at P8**, after every lane ahead is cleared (on staging and on
  `main`; fact 10's list: L3, L8 and any lane that overtook L5 by a recorded slip), and then
  immediately before PR-1's merge (Q-I). Until P8, L5 has neither merged nor pushed,
  so if PR-1 stalls the controller may record L5 as slipped (M7, M8) and L1's C1 is not held. L5
  does not widen `judge_allowances.kind` (L3's Q-F; spec Q41).
- **Q42's merge gate (ruled).** No task that gives a student a way to add or find a syllabus (T8a,
  T8b's hand-over, T9, T10c, T11a-T11f, T10d) merges to `main` until L4's page carrying §6.1 items 2-5
  is on `main`. Engine and cloud tasks may merge first. §3 Q-A adds the go-live condition.
- **No secret, account id, email, school or person's name** in code, tests, comments or test names.
- **LF everywhere**; `*.ps1` stay CRLF; `engine/tests/fixtures/**` untouched.
- **Write in pieces.** Any document a task writes over 150 lines is written in pieces of at most 150
  lines (memory: SDD dispatch lessons).

## 3. Open questions

Each has a recommendation and the plan is written to it, so a "yes" changes nothing. **(Quinn)** marks
Quinn's; the rest are the controller's at dispatch. Asked one at a time, recommendation first.

- **Q-A (Quinn). The go-live under L4's "not in this version" parenthesis.** L4's draft carries §6.1
  items 2-5 with "(Knowlu does not read syllabi in this version …)" and recommends (its Q2 (a)) that
  removing it is material: a `PRIVACY_VERSION` move and a re-prompt before syllabus reading starts.
  With that line on the page, Q42's gate alone would let L5's surfaces reach `main` under a page that
  says Knowlu does not read syllabi. *Recommend:* if L4's Q2 is ruled (a), L5's student-facing work
  merges as **one PR (PR-2)** whose last commit is the go-live task **P1**, privacy bump #2. P1 moves
  the same tokens as L4's bump (L4 plan §4.2), in one commit: the page's content change (the
  parenthesis out, and fact 14's change-rule sentence without "such as syllabi"), its Effective,
  version and "Last changed" lines, the page pins in `engine/tests/site.rs`, `PRIVACY_VERSION`, and the
  panel's "what changed" entry in `console.js`'s `PRIVACY_CHANGES`, all to `V2`. `PRIVACY_BUMP_1`
  (`app/tests/grades.rs`) is checked and stays at L4's `V`. Every syllabus read on the device waits on
  an accepted `V2`: the slot's pass (P3, under Q-K) and the scoped chain (T8a). If L4's Q2 is ruled
  otherwise, P1 shrinks to what that ruling says. *Alternative:* merge surfaces one by one and bump at
  the end, which leaves a window where `main` contradicts the page.
  - **The second bump needs Quinn's extension of L4's Q-AGENT ruling** (L4 plan §5, Q-AGENT):
    `.claude/agents/reviewer.md` and `cloud-engineer.md` say "`PRIVACY_VERSION` moves only in a release
    PR", and L4 asks Quinn to let its lane PR move it once. **Quinn: does that ruling also cover L5's
    PR-2 moving `PRIVACY_VERSION` a second time (bump #2), on the same terms?** *Recommend:* yes, on
    L4's terms: the reviewer reads the ruling as covering this bump, and `privacy-guard.ps1` plus the
    release tag stay the real gate. P1 does not start without the answer. If Quinn says no, P1's
    commit moves to the next release PR and PR-2 waits for it, since nothing may read a syllabus before
    the bump. No agent edits `.claude/`.
- **Q-B (Quinn, RULED 2026-10-19: the alternative). The course's grading fields stay last-write-wins
  across desktops.** Quinn's reason, verbatim: "making the student check their grades to confirm which
  is correct defeats the purpose of us showing them their grades". So a conflict between desktops on
  a course note's `grade_weights`, `grade_scale` or `late_policy` never becomes a card. It lands as it
  does today: the later write wins, the losing write stays in the journal as a supersede record, and
  sync names it in its warning line (fact 3).
  - *What builds it (T4e, `contract-engineer` at xhigh, `contract-reviewer`).* T4e still widens
    `AMENDABLE_FIELDS` by the three fields, because the syllabus grading amend card on one desktop
    goes through `apply_amendment`, which refuses any field outside that list. In the same commit,
    `sync.rs` gains a second list, `pub const LAST_WRITE_WINS_FIELDS: [&str; 3] = ["grade_weights",
    "grade_scale", "late_policy"]`, beside `ACTOR`, with Quinn's sentence as its doc comment. The
    `cardable` closure becomes `card_folder && crate::approvals::AMENDABLE_FIELDS.contains(&field) &&
    !LAST_WRITE_WINS_FIELDS.contains(&field)`.
  - *Why the list is separate, and lives in `sync.rs`.* `AMENDABLE_FIELDS` answers "may a card change
    this field?", and the grading card needs a yes. The new list answers "does a cross-desktop
    conflict on it become a card?", which is sync's own rule, and it has exactly one reader (this
    closure). Shrinking `AMENDABLE_FIELDS` would break the grading card. Moving the list into
    `approvals.rs` would put a sync rule in the approvals contract.
  - *Pins (P2).* The opposite of r3's: last-write-wins with its warning, in parity with
    `a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so`, and never a card. Also
    pinned: what happens to a syllabus grading card when the field moves under it, on one desktop or
    two (P2's syllabus-card cases; G2, G7-G11). Sync's existing tests and both tripwires stay untouched
    and green.
  - **Quinn's simplification (2026-10-19), as relayed by the controller. It supersedes the second
    ruling of the same day.**
    > A pending (or snoozed) syllabus grading card whose field no longer holds the card's `from` is
    > withdrawn as *answered* by **any** desktop's `rank`. Nothing is re-filed for that syllabus. A new
    > grading card for the course appears only when the syllabus itself changes (a new extraction or
    > grading hash), never because the note's value moved.

    Quinn's reasoning: the only writers of these three fields are the student and an approval on
    another desktop, so a moved value already is the student's answer. Fact 3 confirms it in code.
    This narrows the earlier I5 ruling ("only a foreign agent write re-bases"), which is superseded:
    the actor of the write is no longer read, and nothing is re-based. Q-B's last-write-wins ruling
    stands, and so do T4e's widening, `sync::LAST_WRITE_WINS_FIELDS`, the `cardable` exclusion and
    G5.
  - **Quinn's two rulings of 2026-10-20, as relayed by the controller** (the r6 review's I-3 and the
    G10 edge):
    > **I-3.** A pending card the current syllabus no longer supports is withdrawn quietly on any
    > desktop, and nothing is re-filed. "No longer supports" means either the syllabus no longer gives
    > its value, or the note already holds it.
    >
    > **G10.** The one-time card is accepted, as the planner described it.

    The first is G11 below. It covers grading cards (h2's card when the syllabus is now h3 and h3
    matches the note) and item cards (d1 → d2 → d1; a due amend card whose new day is already the
    task's `due`). The second closes the G10 edge as an accepted cost ("Costs", below, with the
    review's two notes).
  - **The syllabus cards' rules ("G" for grading card; R1 is T4a's review).** G9 and G2's filing half
    are T4d's. G5, G6 and `withdraw_syllabus`'s signature are T4e's. G11's item half, the item
    survivor and the survivor order are T4d'c's. G7, G8, G10, G11's grading half and G2's survivor are
    T4d'b's.
    - **G2, one pending grading card per course (r7: decided where the syllabus is; r6 M-2).** The
      filer files a grading card only when no pending or snoozed grading card for the course is left
      after this run's withdrawals. Where several are left on a desktop holding the course's usable
      extraction, it keeps the first by the *survivor order* and withdraws the rest (*duplicate*).
      The survivor order: a card with a readable `id:` before one without, then by `id:`, then by file
      name. Ids are random, file names carry a random part, and both travel with the card (fact 3), so
      every such desktop picks the same card, with no clock and no machine name. A desktop without the
      extraction withdraws no duplicate: it cannot tell which card a syllabus supports (G11), and the
      holders' withdrawals reach it by sync. If it could, it might archive the one card a holder keeps.
    - **G5, the approval (kept; r4 I1).** See "How an approved grading card is written", below.
    - **G6, one comparison (kept).** `approvals::amendment_is_stale` is the from-loop only, and the
      three fields compare key-sorted (T4e).
    - **G7, answered (the ruling).** This runs on any desktop, with or without the course's
      extraction. A pending or snoozed grading card is withdrawn when any of its fields no longer
      holds the card's `from` (`amendment_is_stale`). Who made the write is never read. The ruling
      says "whose field", so a card with one field moved is withdrawn whole.
    - **G8, *set changed* replaces, and never only removes (r7: it waits for room; r6 M-1).** This
      needs a desktop whose extraction has a valid grading set. It reaches only a card G11 leaves, so
      one the current syllabus still supports, under another key: one whose carried fields agree with
      the set, while the set differs from the note in a field the card does not carry. *Set changed*
      withdraws it only when the current key has never been filed in the vault, at least one carried
      field differs from the note (G10), and this run's filing plan files the current key within the
      day's 8 (Q41's order). Both happen in the same run. When the day has no room, the card stays and
      is replaced on the first run that has room. "Agree with the set" means the set gives every
      carried field a valid value equal to the card's. A card one of whose fields the set leaves
      absent (a part that failed §4.7's check) is not replaced, and it still blocks the narrower key
      under G2's filing half (r8, G11's one reading; T4d'b step 3).
      - *Why this choice over a stated gap (M-1).* The card G8 replaces is still one the syllabus
        supports. Taking it off the deck before its replacement can be filed only loses a correct
        proposal. It would also leave the course with no pending card, so G2's filing half would let
        another desktop file a different key into the gap. Unsupported cards are a different case:
        Quinn's ruling withdraws them at once, and their gap is stated under G11.
      - Without the "never filed" condition, a desktop holding a different extraction would withdraw
        every card another desktop files (r4 I2).
    - **G9, a grading key files once.** Any card holding a grading key blocks it, in `approvals/` or
      `archive/`, whatever its status (`existing_source_uids`, unchanged). "Superseded is not a
      decision" stays for item keys only, and only for a day never filed under the key (the twin). So
      an answered set is never proposed again, and a new hash is a new key that files.
    - **G10, a card carries only what differs (r5 I3).** At filing, a field the note already holds is
      left off the card (T4e's comparison). If none is left, nothing is filed. The edge this leaves is
      accepted by Quinn (2026-10-20; "Costs", below).
    - **G11, unsupported (Quinn's I-3 ruling, 2026-10-20).** A pending or snoozed syllabus card that
      the current syllabus no longer supports is withdrawn. It is *withdraw-only*: the rule files
      nothing, and a replacement comes only from the filer's own rules, for a key or day never filed.
      It is *quiet*: no warning and no notice on the course. `rank`'s run line counts it under its
      word, as it counts every withdrawal. "No longer supports" has two halves.
      - *The syllabus no longer gives its value.* This is read only on a desktop holding the course's
        usable extraction. A missing, unreadable or unusable extraction gives nothing to read, so it
        never "no longer gives" a value.
        - *One reading for both halves (r8, the r7 review's I-2; a controller reading reported to
          Quinn, "Costs" below).* The syllabus "no longer gives" a value only when it gives a
          **present, valid, different** one. An **absent** value is not evidence, by Q15's reasoning
          (spec §4.6 "When the syllabus changes": "a syllabus edit is not evidence that work was
          cancelled"; :766-767, "An item a re-extraction drops is not a reason"). Absent means an
          item the re-extraction drops, an item whose day no longer resolves (it now *needs a date*),
          a grading field whose part fails §4.7's all-or-nothing check (weights that do not sum to
          100 ±1, a scale out of order, a `late_policy` outside its closed keys), or a grading part
          the extraction did not read. A card whose value is absent stays as it is under this half,
          and G8 never replaces it; G7, G11's note half and G2 still apply to it.
        - A grading card: some carried field's `to` does not compare equal to the valid value the set
          gives that field (T4e's predicate). Word: *unsupported*.
        - A task, exam or due amend card: the item is still in the extraction with a resolved day,
          and that day is no longer the card's. This is T4d's *date moved*, which under the ruling
          withdraws whatever the filer then does.
        - *Why not the other reading.* Counting an invalid grading part as *unsupported* would let one
          model failure on one desktop withdraw *c*; its tombstone archives *c* everywhere, and G9
          then blocks *c*'s key when a later read gives the same set back, so the course loses its
          card to a read failure, not a syllabus change (the r7 review's consequence).
      - *The note already holds it.* This is read on any desktop, from synced notes only.
        - A grading card: G7 already covers it. A card's `to` never compares equal to its `from`
          (G10), so a note holding `to` no longer holds `from`.
        - A due amend card: the target task's `due` already compares equal to the card's `to`.
          Word: *unsupported*.
        - A task or exam card: a note that is not an approval card already carries the card's key as
          its `source_uid`: a task `materialize` wrote, or an exam's commitment, in `tasks/`,
          `commitments/` or `archive/`. That happens when another desktop approved a twin card before
          a sync. Word: *unsupported*. It withdraws a twin still pending once the approval has
          synced. It does not stop a twin the student approves on the second desktop before that
          desktop's `rank` runs: `process_approvals` (`cli.rs:430`) materializes it before the hook,
          so a second task or `exam` commitment can still appear in that window, and the student
          deletes one. r8 makes no claim beyond that and does not widen T4e (the r7 review's M-2).
      - *The gap (M-1).* An unsupported card is withdrawn whether or not its replacement fits the
        day's 8, since the ruling withdraws it at once. So a moved item can be without a card until a
        run has room (the day's 8 and the 15, T4d's "The 15"), usually the next day; Q41 puts items
        due within 7 days first.
      - *Convergence.* Every desktop that can read a half reaches the same answer from the same
        inputs. Two desktops whose syllabi disagree each withdraw the card their own syllabus does not
        support, and neither files again (G9, the twin's filing rule). So they end with no card, which
        P2 (h) and (i) pin.
    - **The item-card twin (T4d'c; r5 I1's second point, rewritten for G11).**
      - *Filing.* The filer files an item's day only when no card in `approvals/` or `archive/` has
        held the item's key with that day. For a due amend card, no card may hold
        `<item key>:due:<day>`, and the task's `due` must not already compare equal to the card's
        `to` (T4e's predicate, as G11's note half reads it). So d1 → d2 → d1
        ends with no card (Quinn's example). Two extractions that disagree on an item's day each
        withdraw the other's card once and file nothing more.
      - *Duplicates.* Two pending or snoozed cards for one item key and one day keep the first by the
        survivor order (G2), on a desktop holding the extraction (*duplicate*). Cards for one key on
        different days are G11's to decide.
      - *Authoritative match* reads only synced notes, so every desktop already agrees on it.
    - *Retired:* G1 (device ownership), G3 (the human/agent split), G4 (the per-field re-base), Q-L,
      and r6's "*date moved* withdraws only to make room" (replaced by G11, as Quinn ruled).
    - *What the vault sees.* A withdrawal stamps the existing `status: superseded` and archives the
      card, as `withdraw_asks` does. *Answered*, *unsupported* and *duplicate* are names that live
      only inside `syllabus_cards.rs` and in `rank`'s run-line words. None is ever written to a note or
      a journal record. No status, `journal::VIAS` value, op or frontmatter key is added (fact 3), so
      nothing joins the contract list beyond T4e.
    - *`expires`.* The grading card keeps `expires: null`, like every amend card the engine files
      (`write.rs:1079`, `commitments.rs:2576`). G7 and G11's note half run on every desktop, and G8
      replaces, so no card can block a course for good. An expiry would end an unanswered set
      permanently, since G9 keeps its key.
      - *Item cards (r9, the r8 review's M-1; the spec sets none).* A task card's `expires` is its
        payload's `due` day, and a `syllabus-exam` card's is the exam's date, as an `event-accept`
        card's is its event's date (`eventemit.rs:780-793`). A due amend card keeps `expires: null`,
        like every amend card (`write.rs:988`: "an amendment does not go stale on a date"). *Why:* a
        task or exam card answers nothing once its day has passed, and the horizon never files an
        item whose day is before the day its card charges (today, or day 2 on the vault's first
        day; T4d's "The horizon", r10, the r9 review's I-1), so no card is filed already stale:
        `process_approvals` wakes a held card and expires it in the same call when `expires` is
        before the day (`approvals.rs:1596-1627`), and the day-2 floor keeps that from happening to
        a day-1 card. `process_approvals` archives a card as `expired` the day after its item's
        day. In `defer_over_budget`'s order, `(expires, stem)` with `null` last
        (`approvals.rs:1209-1223`), a near item card is kept ahead of later item cards and of every
        amend card. That is not Q41's order, which puts amend cards second: a trim can snooze a
        grading card while keeping a later task card (T4dt (B)'s held variant; r10, the r9 review's
        M-3). *Its edge:* the expired card is history that is not `superseded`, so if the syllabus
        later moves that item to a new day, the key is blocked and nothing files (spec §4.6's
        `existing_source_uids` rule, whose one exception is `superseded` history), as an expired
        event card blocks its event. This stays the plan's default (r10, the r9 review's M-1); the
        alternative, that `expired` item history unblocks the key under the twin's day rule, is on
        Quinn's list with the cost of each (§7 item 2).
  - **How an approved grading card is written (r4 corrected).** Like every non-mail amend card, it
    applies under the executor's context, actor `agent:approvals` (`approvals.rs:30-36`, `:1693`).
    Only `MAIL_CARD_CREATOR` cards take the human actor (`apply_mail_amendment`, `:905-918`). No new
    arm is added. r4's "written as the student's write" was wrong and is withdrawn.
  - *VISION.* "Conflicts surface as proposals, never silent merges" holds only in its weaker sense
    here: the merge is not silent, since it is named and the losing write is in the journal, but it
    is not a proposal. That is Quinn's call, recorded here; `docs-keeper` names it beside VISION's line
    if Quinn wants it written down.
  - *The signed spec* never says these conflicts become cards, so Q-B's ruling changes no spec text.
    The grading card's rules do change one place, spec §4.6 "How a syllabus card is withdrawn". It
    joins Q-K's amendment block as place 10, rewritten for r7.
  - *Costs.* The first two follow from the I-3 ruling's words and are reported to Quinn as decisions
    taken. Quinn accepted the third (2026-10-20). The fourth is a controller reading of the ruling,
    taken in r8 and reported to Quinn as a decision taken.
    - **A reverted set ends with no card.** If a syllabus goes from set A to set B and back to A,
      A's key is blocked (G9), and B's pending card is unsupported (G11). The course holds no grading
      card until a new hash. Items do the same: d1 → d2 → d1 ends with no card (Quinn's example).
    - **Two desktops whose syllabi disagree end with no card.** Each withdraws the card its own
      syllabus does not support (G11), and both keys are blocked (G9). The lower id stands (G2) only
      between cards that both syllabi support. This follows from "the current syllabus" in the
      ruling. It replaces r6's "the lower id stands, whichever syllabus it came from".
    - **The one-time card (G10's edge; accepted by Quinn, 2026-10-20).** Suppose a new key's carried
      values all equal the note's when the key is first seen. G10 files nothing, and nothing remembers
      the key, so a later move of a value files it once, against the simplification's letter. The
      record holds the review's two notes (r6 M-4):
      - When that card files, G8 may withdraw a pending card under another key in the same run, as at
        any filing of a never-filed key.
      - Two fixes were not taken. One is a file `rank` writes. The other is a card filed and
        archived at once, so the key has history. That card would be counted by
        `count_proposals_created`, charging one unit of the day's 15, unless `proposal_weight`
        (`approvals.rs:1116-1124`) changed, and that function is on the contract list.
    - **An absent value is not "no longer supports" (r8; the r7 review's I-2; controller reading,
      reported to Quinn).** I-3's "the syllabus no longer gives its value" is read as "gives a present,
      valid, different value", for grading and item cards alike (G11). A dropped or undated item, and a
      grading part that fails validation or was not read, leave a pending card as it is.
      - *Why.* Q15's reasoning (a syllabus edit is not evidence), which the spec already applies to
        dropped items. The other reading lets one desktop's failed re-extraction withdraw the course's
        grading card everywhere, after which G9 blocks its key for good.
      - *Cost.* A card can stay pending when the syllabus no longer mentions it, until the student
        answers it, its value moves (G7), the item is dated again or a valid set replaces it (G8). The
        student can still reject it. No card is lost to a model failure.
      - *If Quinn reads the ruling the other way,* only G11's syllabus half changes (T4d'b and T4d'c),
        and T4dt case 6's re-extraction variant flips to a withdrawal.
- **Q-L (retired in r6).** Quinn's simplification removes device ownership, so there is nothing left
  to decide. Its flag on *date moved* is kept as the item-card twin in Q-B's rules (T4d'c).
- **Q-C (controller). What may start before the branch is cut.** *Recommend:* T0 (docs, after the
  controller's pending cloud-design amendment commit lands, so the two do not mix), T18 (research
  note), S1 and S3 (Quinn's block), and T7b's off-repo seed folder and its consent manifest (Quinn's,
  no code). Every code task waits for the cut, T7a included: it needs T6b's schema, and T6b waits for
  T6a and L3's merge (r3 minor 2).
- **Q-D (Quinn). S1 and S3 in one sitting.** Both need Quinn signed in at UA. *Recommend:* one block
  right after #19's B9 live proof, on the same scratch profile, then the profile's cleanup. T9 waits
  for S1, T10b for S3; Offer B's path waits for neither.
- **Q-E (Quinn). Where the real eval seeds live, and what touches them.** *Recommend:* a folder Quinn
  creates outside every working tree and outside any synced folder (not under the repo, `.claude/`,
  OneDrive or `%LOCALAPPDATA%\knowlu\`), read by the eval runner from one environment variable; real
  seeds are **never inserted into staging's `eval_cases`**, which CI's eval gate reads, and their runs
  report per-part counts and pass rates only. Quinn names the folder and keeps its consent manifest
  (§2): one entry per seed recording that its owner consented and that the syllabus is not public,
  including any file S1 or S3 moves there. T7a's reader reads nothing else. *Alternative:* staging
  `eval_cases` rows marked private, which puts real syllabus text in a third-party database.
- **Q-F (controller). The syllabus pass and L3's capped rule.** If L3's T3b lands, *recommend* the
  syllabus pass reports its own cap line ("Not read: the daily limit was reached; tomorrow", spec §8)
  and `/judge-syllabus` is **not** added to `fatal()`'s batch list (L3's plan, Q-F).
- **Q-G (controller). Two PRs, with fixed contents.** *Recommend:* **PR-1** carries the engine and
  cloud work with no `surface.rs` change and no app surface: T1-T5, T15, T16 (engine part), P2 and
  T6a-T7b. No app writes `state/syllabi/`, and under Q-K a plain `judge` never reads one, so Q42 allows
  it. It merges at P8 together with its staging push (M7). **PR-2** carries T17t, T17, T16s (the
  surface parts, after L7), every `app/src` and `app/static` task, P3, the curated host and P1, after
  L4, L6, L7 and the queue slot. No task is "1 or 2". *Alternative:* one PR, which holds the engine
  behind the console queue and L7.
- **Q-H (controller). B9 and B9a before the cut?** The close-out ends "B9a, then L5's planner
  re-plans". *Recommend:* the cut needs B8 only; B9 is a live proof and B9a edits only
  `registrar::SCHOOLS`, neither of which L5's files read. If either changes `commitments.rs`, P0
  re-reads it.
- **Q-I (controller). The pin before the eval, with no early staging push.** T6a's migration
  carries the `models` row the route needs, but T7b chooses the pin (Q9). *Recommend (review r1's
  fix):* T6a pins the spec's expected candidate (a model and host the page already names; spec Q9
  expects Qwen at DeepInfra). T7b evaluates **on the controller's machine**, running the pipeline
  in-process as `run_eval.ts` does (fact 12), with each candidate's model row given locally, so the
  eval needs no deployed route and no migration on staging. If a different candidate wins, the
  `models` row in T6a's own file is edited before P8. The file has never reached staging, so it is not
  frozen (M6) and no second migration is needed. The migration reaches staging only at P8, once every
  lane ahead is cleared (fact 10), from the lane branch immediately before the merge (P8's order). The window in
  which L5 is pushed and not merged is the minutes between P8's push and its merge. If that push's SQL
  fails, the controller withdraws L5 on its own (M7). Any other withdrawal asks Quinn. Until P8, a
  stall in PR-1 (L6 or L3 late, a red review) holds nobody: L5 has neither merged nor pushed, so the
  controller may record its slip and L1 goes ahead (M8), with L5 re-stamping by M5 before P8.
  *Alternative:* push from the lane branch for a staging eval, which holds L1's C1 for as long as
  PR-1 is open (the review's scenario).
- **Q-J (Quinn). PR-1 opens a spend path.** The `syllabus` kind charges the judge route (20 calls a
  day, 40 on the first two judging days, inside the $2 monthly ceiling), and T7b spends provider
  credit through the controller's local run (about $0.0025 a call at the expected pin, spec §7). Money stays with Quinn (handoff §1 item
  7), so green CI and a PASS are not enough. *Recommend:* the controller asks for PR-1's merge in one
  message with the eval's spend so far, the per-call bound and the review path, as L3's Q-G does.
- **Q-K (Quinn, new in r2). The syllabus pass runs only when asked.** Spec §4.3 says "a plain
  `judge` run in the slot runs the syllabus pass". Under L4's gate, a plain `judge` still runs when
  the privacy cache reads `Unknown` (fact 13), so a syllabus could be sent before anyone has seen
  that the page now covers syllabus reading. *Recommend:* fail closed. The pass runs only in
  `judge --syllabus` (a full run plus the pass) or `judge --only syllabus` (the spec's scoped run). A
  plain `judge` neither reads `state/syllabi/` nor sends a syllabus, and its output is unchanged byte
  for byte (T5, T5t). The scheduler adds `--syllabus` only when L4's gate is `Accepted` for the
  current `PRIVACY_VERSION`. Under `Unknown` it names the skip in its own row, and under `Older`, L4's
  whole-`judge` skip already holds (P3). T8a's scoped chain checks the same gate. One side effect:
  PR-1 on `main` is inert, since no build passes `--syllabus` before PR-2. Cost: `judge` gains a second
  flag (T13 documents it), and the signed spec changes in **nine places**, not one sentence (r2
  finding 6; r3 minor 1 added place 9). Located by heading and today's line, to be re-found by quoted text when edited:
  1. §4.3 "Where it runs: inside `judge`, as a scoped pass (Q5)" (:443-446): "A plain `judge` run in
     the slot runs the syllabus pass …" becomes: the pass runs under `judge --syllabus` (the full run
     plus the pass) or `judge --only syllabus`; a plain `judge` never reads `state/syllabi/`.
  2. The same paragraph (:449-450): "This changes the CLI contract … gains the flag (T13)" names
     both flags.
  3. §4.3 "Running it next to the scheduler (Q17)" (:459): "The slot's own `judge` step picks the
     course up" holds only when the slot passes `--syllabus`, which it does only under an `Accepted`
     gate (P3).
  4. §10 "Choices, with a recommendation each", row 5, "Where extraction runs" (:1150): option (a)
     gains `--syllabus` and "a plain `judge` never runs the pass".
  5. §11 "Decisions for Quinn", item 5, "Where" (:1199): the same.
  6. §13 row T5t (:1365): "the syllabus pass in a slot `judge`" becomes "in `judge --syllabus`", plus
     the plain-`judge` cases (T5t here).
  7. §13 rows T5 and T13 (:1366, :1392): "the flag" becomes the two flags; T13's list gains
     `judge --syllabus` and "a plain `judge` reads no syllabus".
  8. §16.7 "Which judgment runs, and where", the bullet "`judge` is where it runs" (:1748): "in the
     slot after enrichment" becomes
     "in the slot's `judge --syllabus` after enrichment, under an accepted privacy version".
  9. §10 "Choices, with a recommendation each", row 17, "Locking the post-add run against the slot"
     (:1162): "if a slot holds it, the slot reads the course" holds only when the gate is `Accepted`;
     otherwise the course waits for the privacy check (T8a).
  10. **From Q-B's ruling, Quinn's simplification (both 2026-10-19) and Quinn's I-3 and G10 rulings
      (2026-10-20), not Q-K:** §4.6 "How a syllabus card is withdrawn" (:757-790), with its
      neighbouring passages and §13's T4e row (:1361). Twelve changes (the tenth added in r8, the r7
      review's M-4; the eleventh in r9, the r8 review's I-1; the twelfth in r10b, the r10a review's
      I-2; the M-1 option in §7 item 2 would be a thirteenth):
      - The filer-side reasons gain *answered* (G7). It covers a pending or snoozed grading amend card
        any of whose fields the course note no longer holds at its `from`, by
        `approvals::amendment_is_stale`. It runs on every desktop, with or without an extraction, and
        nothing is filed again for that set.
      - The reasons gain *unsupported* (G11): a pending or snoozed card the current syllabus no longer
        supports, because the syllabus no longer gives its value or the note already holds it. It is
        withdraw-only and quiet. *Date moved* (:762-763) becomes its syllabus half for item cards. The
        syllabus half runs only where the extraction is; the note half runs on every desktop. "No
        longer gives its value" means the syllabus gives a present, valid, different value. An absent
        value (a dropped or undated item, a grading part that fails §4.7's check or was not read) is
        not a reason, as :766-767 already says of a dropped item (Q15's reasoning; r8).
      - *Set changed* (:764-766) replaces and never only removes (G8). It withdraws only in the run
        that files the current grading key within the day's 8. That key must never have been filed and
        must differ from the note in at least one field. It never replaces a card one of whose fields
        the current set leaves absent (r8).
      - "One course never holds two pending grading cards" is enforced at filing. Where two desktops
        filed before syncing, a desktop holding the extraction keeps the first by id, then file name
        (G2). Item cards for one key and one day get the same rule. An item's day files only if never
        filed under its key (the item-card twin).
      - "Superseded is not a decision" (:779-784), and the sentence on it in "The duplicate check"
        (:713-716), apply to item keys only, and only to a day never filed under the key. A grading key
        is blocked by any card holding it (G9). This states §16.3's own rule: "never proposed back over
        unless the syllabus itself changes (a new hash)".
      - A grading card carries only the fields the note does not already hold. If none is left, there
        is no card (G10). §4.7's "changes carrying whichever of the two is valid" (:802-803) gains "and
        differs from the course note". The one-time card this leaves is accepted (Quinn, 2026-10-20).
      - An approved grading card applies as `agent:approvals` (G5).
      - §4.6's "When the syllabus changes" (:741-745) gains: an unsupported card's replacement waits for
        room in the day's 8, so a moved item can be without a card for a day.
      - §5 "A second desktop (Q18)" (:891-896). A second desktop still files no syllabus card, but it
        withdraws a card the student answered (G7) and one the note already holds (G11's note half).
      - §4.6's "The move is `approvals.rs`'s" (:768-776) and §13's T4e row (:1361), which take the
        reason "from `syllabus_cards::withdrawal_reason`" and count every withdrawn card pending on
        disk. `withdraw_syllabus` takes a **caller-built** reason: `rank`'s hook looks each card's file
        stem up in the filer's decision for the run (`syllabus_cards::plan`), whose per-card reasons
        include `withdrawal_reason`'s. Its `pending` count is the **counted** rule: a withdrawn card
        counts only if `process_approvals` counted it, so pending on disk and, for an amend card, a
        stem in the run's `amendments_pending` (T4e; r6 I-1).
      - §4.6 "Which items are filed, and when" (:690-697) and §16.8's "The deck" (:1756-1760), whose
        "anything over is snoozed by `defer_over_budget`" assumed the filer could file past the 15.
        The filer files at most the smaller of what is left of the day's 8 and what is left of the
        15 on the day the cards charge (today, or day 2 on the vault's first day), so a syllabus card
        is never the overflow. An item that does not fit is not removed from the plan: it is planned
        again on each run while it is in the horizon and ahead, so it gets a card if any of those
        runs has room. It can still reach its day with no card when every such run is full, and an
        item that enters the horizon late (a syllabus added mid-term with a quiz two days out) has
        only the runs left before its day (r10, the r9 review's M-3). `defer_over_budget` and the 15
        are unchanged, and still snooze overflow that other cards make (T4d's "The 15"; r9). This
        states the spec's own "8 a day, inside the 15" (:1242, :1267) and keeps §16.8's "nothing
        changes the 15 or `defer_over_budget`'s order". The same change makes the horizon's "of
        today, and never once past" (:690-691) read from the day the card charges: on the vault's
        first day, day 2, so no held card is filed for an item due on day 1 (T4d's "The horizon";
        r10, the r9 review's I-1).
      - §5's byte-contract list (:880), "per item (`filed`, `duplicate`, `needs-date`, `later`,
        `feed-differs`)", gains `past`, so a past item with no card has a code in version 1, as
        §4.8's `items` state (:842) already has it; the per-item outcomes are stored as
        `item_outcomes` (T2c; r10b, the r10a review's I-2).
      Q-B's ruling changes no spec text, since the spec never says these conflicts become cards. This
      place is amended whatever Quinn answers on Q-K (T4d, T4d'b, T4d'c, T4e, P2).
  §13 has no row for the slot's flag decision; P3 is a plan-only task, and items 3 and 9 are where the
  spec learns of it. *How it is amended:* after Quinn's yes (for place 10, Q-B's ruling, the
  simplification and the two 2026-10-20 rulings are already given), `docs-keeper` edits the places in place
  and adds one dated block at the top of the spec, in the form of its "Amendment 2026-10-18 (signed by
  Quinn, 2026-10-19)" block, listing them. The block lists place 10 as "the syllabus cards under Quinn's
  simplification and the unsupported-card ruling (G2, G5, G7-G11) and the item-card twin". It cites
  the four rulings by quote: Q-B's reason, the simplification, I-3 and G10. It names none of the
  retired rules (G1, G3, G4, r6's twin condition), since none of them ever reached the spec. This lands in T0, or in a one-commit docs follow-up if T0
  has already landed. *Alternative:* keep the spec's default and add `judge --no-syllabus`, which the
  scheduler passes unless the gate is `Accepted`. That is fail-open: a caller that forgets the flag
  sends syllabi, and it changes only places 2 and 7.
- **Q48 (Quinn, conditional, unchanged).** Asked only if T4a finds a #19 test asserting a tier's
  number (spec §17 Q48; recommend rewriting that one assertion to the order it protects).

## 4. Conflict map

### 4.1 What PR #19 changes in the files L5 edits

#19 lands first, so these are facts L5 builds on, never edits to make. P0 re-reads each on `main`.

| File | #19's change (close-out plan task) | What L5 does there, and the rule |
|---|---|---|
| `engine/src/commitments.rs` | `precedence` tiers (registrar 0, `google:` 1, else 2); the registrar's `refresh_series` arms (in-play keep-alive, ended at once); registrar series out of `instances_map`; the per-kind hold in `proposals`, `withdrawal_reason` and `settle_approved` (B4a, one predicate); `SERIES_KEY_PREFIXES` += `registrar:`; `asks_wait_for_registrar`; `SERIES_LOCK_FILE`; apply-and-list, the `registrar` block in the series file, kept entries filed through `emit_checks` (B3a-B3e); **the per-source table** with two key columns and the tier, keep-alive, instances, per-kind-hold and landing columns (B3f) | T4a adds the `syllabus:` row and its two arms and renumbers the tiers (Google 2, default 3), editing no other column of another row; #19's tests stay green and unedited. T4b adds `contested` and the section match; T4c the ask gate inside `emit_asks` (#19's gate stays in `cli.rs`). Everything strictly after P0's re-read |
| `engine/src/cli.rs` | `commitment_passes`: the registrar's fresh keys, its ask gate, and the `asked` drop under the lock (B3e); the `--registrar` dispatch | T2c and T15 add `syllabus-add`'s dispatch; T4a reads the syllabus source beside the Google and ICS normalisation; T4d'a adds the `rank` hook. Keep #19's lines; L5's hook runs after `process_approvals`, `withdraw_syllabus` before the filer, outside the series lock |
| `engine/src/main.rs` | the `Commitments` arm gains `--registrar`/`--school`, actor through `human_or_vaults` (B1, B1a) | T2c, T15 (`SyllabusAdd` and its held flags) and T5 (`judge --syllabus` and `--only syllabus`, Q-K) add their own arms. L6 edits the `Judge` arm first (its T2); T5 rebases over it |
| `engine/src/lib.rs` | `pub mod registrar;` | `pub mod syllabus;` (T2a), `pub mod syllabus_cards;` (T4d). Keep every line |
| `engine/src/registrar.rs` | new | **Read only.** T15's course match reads the confirmed note's `registrar:` key (term and CRN) by #19's format and `CALENDAR_PREFIX`; no edit |
| `app/src/scaffold.rs` | `Curated` gains `registrar: Option<Registrar>`; the UA row's entry and `calls` (B4) | T10a adds `syllabus_host` and `syllabus_kind`, `None` on every row; T8b makes vault creation create `syllabi/` (spec §5). No other row's data changes |
| `app/src/lms_link.rs` | helpers made public for the registrar's reuse | T9 adds `capture_syllabi` beside them; the discard-everything window is unchanged |
| `app/src/week.rs` | `your_week`'s `registrar_label` | T4b's `contested` must reach the console through `commitment_proposals`; T11d's typed-row command if one is needed. Keep both keys |
| `app/src/main.rs` | the console builder manages `LmsSession`, wipes on `lms-signin`'s `Destroyed` and at exit; three `registrar::` handlers | After L6: console list `syllabus_add`, `syllabus_status`, `syllabus_settle_held` (T8a), `syllabus_connect`, `syllabus_connect_status` (T10c), and the `syllabus-signin` window's wipe; wizard list `capture_syllabi` (T9). Recount both lists by script |
| `app/static/console.js`, `index.html`, `console.css`, `app/tests/static_assets.rs`, `scripts/wizard-check.py` | the "From myBama" list (changed, ended, kept, status rows), `finishMayRun`'s busy gating (B5) | T11a-T11f, then P1's `PRIVACY_CHANGES` entry, in the queue slot after #19 and L1 R11. `finishMayRun` must also wait for T11c's `capture_syllabi`; a string both lanes pin carries both |
| `docs/reference/engine-commands.md`, `app.md`, `docs/surface/anatomy.md` | `commitments --registrar`, the three registrar commands (B6) | T13 adds L5's sections after B6's |

`engine/src/approvals.rs` is **not** in #19's diff (B3e and B4a build in `commitments.rs`).

### 4.2 Every file L5 edits, and who else does

| File | L5 task(s), in order | Other lanes | Rule |
|---|---|---|---|
| `engine/src/syllabus.rs` (new) | T2a → T2b → T2c → T15 → T16 (PR-1); T17 (reader, PR-2) | none | lane-internal order (spec §13) |
| `engine/src/syllabus_cards.rs` (new) | T4d → T4d'c → T4d'b → T16 | none | lane-internal; one task at a time in the file |
| `engine/src/backup.rs` | T2c (`BACKUP_FOLDERS` gains `syllabi`; its two pins) | none found | not contract; pins change in the same commit with the reason |
| `engine/src/commitments.rs` | T4a → T4b → T4c | #19 (merged before) | §4.1 |
| `engine/src/cli.rs` | T2c → T4a → T4d'a → T15 | #19, L6 (none expected: L6's plan found no runtime use in `cli.rs`) | whoever is second rebases |
| `engine/src/main.rs` | T2c → T5 → T15 | #19, L6 (`Judge` arm), email | disjoint arms; second rebases |
| `engine/src/approvals.rs` (**contract**) | T4e only, `withdraw_syllabus`'s final signature included, so no later task edits it (r6 I-1) | email (merged before); none while L5 runs | L5 only (handoff §5) |
| `engine/src/sync.rs` (**contract**) | T4e (Q-B: `LAST_WRITE_WINS_FIELDS` and the `cardable` closure; one comment line at step 5 naming the list) | email (merged before); no wave-1 plan edits it (L7 only calls it) | `contract-engineer` at xhigh, `contract-reviewer`; same commit as the `AMENDABLE_FIELDS` widening, so no tree ever makes the three fields cardable; P0 item 6 re-reads the closure on `main` |
| `engine/src/eventaccept.rs` | T4e | none found | — |
| `engine/src/ingest.rs` | T4f | none in wave 1 after the email merge | single owner; `integrator` |
| `engine/src/judge.rs` | T4g (`Heuristics::load`) | **L6** T5b deletes the local prompt and outcomes; **L7** maybe a `Missing` variant | L6 lands first; T4g touches `Heuristics::load` only |
| grades read model (P0 locates it; `engine/src/grades.rs` or its surface reader) | T4g (`letter` from `grade_scale`) | L2 (merged, PR #40) | read L2's merged code first |
| `engine/src/enrich.rs` | T5 | **L6** T2 (`Options`, `run`, the no-account arm), **L3** none, **L7** likely | T5 adds a pass after enrichment and before events; it rebases on L6's signatures |
| `engine/src/cloudmodel.rs` | T5 | **L3** T3b (`call`, `verdict_of`, `fatal()`), **L6** T5b (one doc), **L7** (402) | T5 rebases on L3's capped rule (Q-F); keeps 402 global |
| `engine/src/surface.rs` | T17 → T16s (PR-2 only) | **L7** first | after L7 merges (handoff §5) |
| `engine/src/lib.rs` | T2a, T4d | #19, email | keep every line |
| `engine/Cargo.toml`, root `Cargo.lock` | T3 | any lane adding a dependency | the lock is regenerated by `cargo build`, never by hand |
| `engine/tests/syllabus*.rs`, `surface_syllabi.rs`, `judge_syllabus.rs`, `engine/tests/syllabus_data/` (new) | T1, T4t, T4dt, T4ft, T5t, T15t, T16t, T17t | none | new files |
| `engine/tests/syllabus_sync.rs` (new) | P2 (`is_note_path`; Q-B's last-write-wins cases; the syllabus cards across two desktops, G2 and G7-G11) | none | contract-engineer, contract-reviewer |
| `.gitattributes` | only under T3's fallback (a committed invented PDF) | none found | one `-text` line per binary |
| `cloud/supabase/functions/_shared/judge_caps.ts`, `judge_caps_test.ts`, `judge_validate.ts` | T6a | **L3** T2a (merged before), **L8** only under its Q4 alternative | T6a starts only after L3 merges (r3 minor 2; T6a also edits `judge_pipeline_test.ts`, which L3 edits); L3 changes no line T6a edits in these three |
| `cloud/supabase/functions/_shared/judge_prompts*.ts`, `cloud/supabase/functions/judge-syllabus/` (new) | T6b | L6's §9 follow-up (one comment in `judge_prompts.ts`) | keep both |
| `cloud/supabase/functions/_shared/judge_pipeline.ts`, `judge_pipeline_test.ts` | T6a (the test's `DAILY_CAP` asserts) → T6b (the `syllabus` arm in `fieldsOf`; the marker test) | **L3, L7 ahead** | rebase; T6b re-reads `fieldsOf` on `main` and keeps both lanes' lines (r2 finding 2). L3 merges before T6a, and so before T6b (r3 minor 2). PR-1 still never waits on L7: if L7 has not merged by P8, whoever lands second rebases (§2) and keeps the `syllabus` arm and its test |
| `cloud/supabase/functions/_shared/judge_rules.ts`, `judge_rules_test.ts` | none planned; T6b stops and reports if `fieldsOf`'s arm alone cannot keep `featureMap` off a `syllabus` row | — | the SQL twin's tests (`judge_rules_test.ts`) stay unedited |
| `cloud/supabase/migrations/<stamp>_judge_syllabus.sql` (new), `migrations_test.ts` | T6a | email, L3, L8 ahead; L1 behind | M1-M8; append-only test blocks, recount `parsed` |
| `cloud/eval/**` (synthetic seeds; the off-repo real-seed reader, which holds no seed) | T7a | L3 T2a (`run_eval.ts`'s `EVAL_CAPS`) | keep both |
| `app/src/syllabus.rs`, `app/src/syllabus_link.rs`, `app/assets/syllabus/*.js`, `app/tests/syllabus*.rs` (new) | T8a; T10t → T10a → T10b | none | new files; T10b's scripts follow the mail window's rules |
| `app/src/onboarding.rs` | T8b | **L6** T3b (`WizardPlan.offer_inference`), email (mail marker) | after L6; the `WizardPlan` field list is the likely hunk |
| `app/src/scaffold.rs` | T8b → T10a → T10d | #19 (merged before) | §4.1 |
| `app/src/lms_link.rs` | T9 | #19 (merged before) | §4.1 |
| `app/src/main.rs` | T8a, T9, T10c (handler lines, the window wipe) | **L6 owns until it merges**; #19, L1, L4, L7 | after L6 merges; recount by script |
| `app/src/week.rs` | T4b (only if the envelope filters fields), T11d (only if a command is needed) | #19 | keep both |
| `app/static/*`, `app/tests/static_assets.rs`, `scripts/wizard-check.py` | T11a → … → T11f → P1 (`console.js`'s `PRIVACY_CHANGES` entry; one static pin on that entry) | L6, L4, L7, #19, L1 R11 ahead | the queue (§2); L5's slot is held until P1 commits |
| `engine/tests/site.rs` | P1 (the page pins: one existing pin reworded, one added) | **L4** (T9's pins, committed in its T13; merged before), C3 R12b (deletes the Microsoft 365 "does not yet" pin) | after L4 merges; P1 edits only the change-rule pin (fact 14) and adds its own; the reason in P1's commit message |
| `app/src/scheduler.rs`, `app/tests/syllabus_slot.rs` (new) | P3 | **L6** (`slot_argv`, `judge_plan_for`), **L4** (the gate in `run_slot_with`) | after both merge; P3 re-reads both on `main` |
| `app/src/account.rs` (**contract**), `site/privacy.html` | P1 (if Q-A; the page's parenthesis, the change-rule sentence of fact 14 and the three date lines) | **L4 owns both until it merges** | after L4 merges; contract-engineer |
| `app/tests/grades.rs` | P1 reads `PRIVACY_BUMP_1`, never edits it | L4 (T13 sets it), L2 | read only |
| `HANDOFF.md`, `VISION.md`, cloud design, `docs/reference/*`, `docs/surface/anatomy.md`, `docs/notes/` | T0, T12, T13, T18 | the Docs lane, #19's B6, L6's T6 | docs land after those lanes' refresh or hand them the lines |

Untouched by L5: every existing sync test and both tripwires (`sync.rs` changes only as T4e's row
says), `ids.rs` (`NOTE_FOLDERS` included), `entitle.rs`,
`write.rs`, `journal.rs`, `ledger.rs`, `yamlemit.rs`, `app/src/commands.rs`, and
`engine/tests/fixtures/**`. `app/src/scheduler.rs` changes in P3 only. T8a reads `Scheduler.running`
and L4's gate and adds nothing to `scheduler.rs`; if it must, it stops.

## 5. Tasks at a glance

Spec task ids are kept (T0-T18, S1, S3) so the fidelity ledger traces each to spec §13; plan-only tasks
are P0-P10. Spec §13's sizes hold: about 80 source lines a task, tests excluded; past about 120 the
agent stops and the controller splits the task.

| # | Task | Agent | Why that agent | After | PR |
|---|---|---|---|---|---|
| T0 | Apply spec §0's amendment: cloud ruling 5, `VISION.md` (by quoted text), `HANDOFF.md` §3 | `docs-keeper` | docs only, after Quinn's signature | the controller's pending cloud-design commit | docs push |
| T12 | Check L4's draft carries §6.1's six statements and the 13,500 bound; the release-guard line in `HANDOFF.md`; Q42's gate as a ledger row | `docs-keeper` | docs; L4 owns the page | — | docs push |
| T18 | First-pass terms read (spec §15.6's twelve clauses), flags only | `researcher` | read-only research | — | docs push |
| S1 | Spike: Blackboard contents by a student cookie session | main session + Quinn | a live session is the controller's | Q-D | none |
| S3 | Spike: the in-page read (spec §15.7) | main session + Quinn | same | Q-D | none |
| P0 | Cut the branch; ledger; baseline; re-read #19's seam and spec §14's open facts on `main` | main session | branching, baseline and facts are the controller's | #19 merged (B8) | — |
| T1 | Failing tests: text, screen, `syllabus-add` | `test-writer` | TDD first; off the contract list | P0 | 1 |
| T2a | Text extraction (DOCX, HTML, TXT; table rows) | `implementer` | specified, checked by T1 | T1 | 1 |
| T2b | The five-part screen, bounds, clip, rules per part | `implementer` | same | T2a | 1 |
| T2c | `syllabus-add` (file, outcome forms), the state file and its one writer (`write_state`, `write_outcomes`, write beside and rename; `item_outcomes` in version 1; r10b) | `implementer` | same | T2b | 1 |
| T3 | PDF crate: choose, prove toolchain, size, row order | `implementer`, then `reviewer` | a dependency with known traps | T2a | 1 |
| T4t | Failing tests: the `syllabus:` series source | `test-writer` | TDD first | P0 | 1 |
| T4a | The `syllabus:` row, tiers, the two arms, the usable read, the ended stamp, `instances_map`, the body line | `integrator` (Opus, high) | change detection on confirmed notes beside the registrar's arms; a silent error ends a class still meeting | T4t, T2c | 1 |
| R1 | Review of T4a | `reviewer` | the riskiest engine change | T4a | — |
| T4b | Section match; `contested` through `--json` and the envelope | `implementer` | specified; changes no tier or row | R1 | 1 |
| T4c | The bounded ask gate (`SYLLABUS_WAIT_DAYS`) | `implementer` | specified | T4b | 1 |
| T4dt | Failing tests: the filer's plan cases through the named `syllabus_cards::read_inputs` and `plan` (r8, the r7 review's I-1), its `rank`-level cases, the duplicate rule, withdrawal | `test-writer` | TDD first | P0 | 1 |
| T4d | `syllabus_cards.rs`: the filer as a pure plan, `read_inputs` and `plan` (G9, G2's filing half, the day's 8 in Q41's order, held inside what is left of the 15 (r9 I-1)), and then its writes; `withdrawal_reason`'s *date moved* and *authoritative match*; card file names with a random part. *Done:* the plan cases only | `implementer`, then `reviewer` | writes only new card notes through `write` | T4dt, T2c | 1 |
| T4e | `approvals.rs`: `syllabus-exam` arm, three amendable fields, `withdraw_syllabus` with its final signature (a caller-built reason; the counted stems; r6 I-1) and `Withdrawal`'s doc comment (r8 M-4), the finishing arm; `eventaccept.rs`; **`sync.rs`: `LAST_WRITE_WINS_FIELDS` and `cardable` (Q-B)**, in one commit | `contract-engineer` (xhigh), then `contract-reviewer` | two contract-list files; the widening and the sync exclusion must land together | T4d | 1 |
| T4d'a | The `rank` hook in `cli.rs`, between `process_approvals` (`:430`) and `remaining_budget` (`:439`) (r8 M-1): the plan, `withdraw_syllabus` with the caller-built reason, then the filer's writes; the context; the pending count from the counted stems; withdrawn stems leave `amendments_pending` (r6 I-2); the run line; the listing into the state file (r10a) (about 50 lines). *Done:* the byte-identical case and the filer's `rank`-level cases (r8 I-1) | `implementer` | one hook, specified and checked by T4dt; `cli.rs` after T4a | T4e | 1 |
| T4d'c | The item rules in `syllabus_cards.rs`: G11's item half (*date moved* unconditional, only for an item still present with a resolved day (r8 I-2); a `due` already at `to`; a key a task or commitment already holds), the twin's filing rule, and the survivor order with the file-name tie-break (about 45 lines) | `implementer` | pure conditions over synced notes and the extraction, checked by T4dt | T4d'a | 1 |
| T4d'b | The grading rules in `syllabus_cards.rs`: G7 answered, G11's grading half (a present, valid, different value only; r8 I-2), G8 set changed (only with room today), G2's survivor where the extraction is, G10's field drop (about 85 lines) | `implementer`, then `reviewer` | pure rules over synced inputs and the local extraction, checked by T4dt and P2; reviewed because this mechanism failed three rounds | T4d'c | 1 |
| P2 | The sync pins: `is_note_path` refuses `syllabi/` paths; the three fields stay last-write-wins with the warning and no card (Q-B); the syllabus cards across two desktops: three slots, a same-day variant, unsupported cards ending with none, approve against withdraw (G2, G7-G11) | `contract-engineer`, then `contract-reviewer` | a sync-contract test file (contract list) | T4e, T4d'b | 1 |
| T4ft | Failing tests: `ingest`'s adoption | `test-writer` | TDD first | P0 | 1 |
| T4f | `ingest`'s adoption branch | `integrator` (Opus, high) | a misfire re-keys the wrong task on the frozen references' path | T4ft | 1 |
| T4g | Weights into `Heuristics::load`; `letter` from `grade_scale` | `implementer` | both slots exist | T4e, L6 merged | 1 |
| T5t | Failing tests: the pass under `--syllabus` and `--only syllabus`; a plain `judge` sends none (Q-K) | `test-writer` | TDD first | P0 | 1 |
| T5 | The pass and the two flags (`enrich.rs`, `cloudmodel.rs`, `main.rs`) | `implementer` | follows the existing kinds | T5t, T2c, L6 and L3 merged | 1 |
| T15t | Failing tests: the Simple Syllabus source and held renderings | `test-writer` | TDD first | P0 | 1 |
| T15 | The `simple-syllabus` source, match, held form, replacement rule | `implementer` | specified | T15t, T4d'a, T5 | 1 |
| T6a | `Kind` + `DAILY_CAP` 20 + the kind-widening migration + models row | `cloud-engineer` | `cloud/`; the staging push is the controller's, at P8 only | P0; L3 merged (r3 minor 2); L8's stamp recorded or T6a's provisional | 1 |
| T6b | Prompt, schema, validator, bounds, `/judge-syllabus`; the `syllabus` arm in `fieldsOf` (`judge_pipeline.ts`), so the route keeps no syllabus text | `cloud-engineer` | same; L3 and L7 edit `judge_pipeline.ts` ahead of it | T6a, L3 merged | 1 |
| T7a | Eval harness, synthetic seeds, pass bar per part; the off-repo real-seed reader with its consent-manifest check | `cloud-engineer` | same; the reader is code, so it is reviewed in PR-1 | T6b | 1 |
| T7b | Real-seed run on the controller's machine; the pin chosen (Q9) | main session | it holds the provider key and the only copy of the real seeds; no subagent does | T7a, Q-E | 1 |
| T16t | Failing tests: policies, `late_policy`, no quote on a card (engine cases) | `test-writer` | TDD first | P0 | 1 |
| T16 | The policies part, `late_policy`, quotes (engine: `syllabus.rs`, `syllabus_cards.rs`) | `implementer`, then `reviewer` | specified | T16t, T4e, T15 | 1 |
| T14a | PR-1 whole-branch review | `reviewer` (+ `contract-reviewer` over `approvals.rs` and P2) | cheaper agents' changes pass review | all PR-1 tasks | 1 |
| P8 | PR-1: gate, every lane ahead cleared, M5 re-check, CI on the rebased head, dry run, staging push, pre-merge dry run, merge at once, on Quinn's word (Q-J) | main session | pushes and merges are the controller's | T14a PASS, T7b, Q-J, every lane ahead cleared (fact 10) | 1 |
| T17t | Failing tests: the `syllabi` read model, with T16's surface cases | `test-writer` | TDD first | L7 merged, P8 | 2 |
| T17 | The `syllabi` read model in `surface` | `implementer`, then `reviewer` | specified | T17t | 2 |
| T16s | T16's surface part: policies and quotes in `syllabi`; the late note on a past-due row | `implementer` | specified; `surface.rs` after T17 | T17 | 2 |
| T8a | `app/src/syllabus.rs`: add, status, settle-held, the scoped chain | `implementer`; handler lines after L6 | thin wiring | P8 (or rebased), L6 merged | 2 |
| T8b | `pick_file`'s extension list; Finish hands staged files over; the sweep; `syllabi/` at creation | `implementer` | specified; after L6's T3b | P8, L6 merged | 2 |
| T9 | Step 1: `capture_syllabi` | `implementer` | specified by S1's note | S1, T8b | 2 |
| T10t | Failing tests: the Simple Syllabus window | `test-writer` | TDD first | S3 | 2 |
| T10a | Curated `syllabus_host`/`syllabus_kind` (all `None`); host checks; script list | `implementer` | rule 1: the host is data | T10t | 2 |
| T10b | The window and the in-page read | `integrator` (Opus, high), then `reviewer` | script evaluation in a signed-in third-party page | T10a, S3 | 2 |
| T10c | Connect commands in the console list; the window's wipe | `implementer` | after L6 merges | T10b | 2 |
| T11a-f | The page, in the queue slot | `console-ui` | `app/static` is its lane | each its row (§6) | 2 |
| P3 | The slot's `judge` gets `--syllabus` only under an accepted version (Q-K) | `implementer` | wiring checked by tests; a fail-closed default leaves no silent path | L4 and L6 merged, T5 | 2 |
| T10d | UA's `syllabus_host` set (Q39) | `mechanical`, then `reviewer` | a one-line data edit | S3, T18, L4's page, P1 | 2 or later |
| P1 | (if Q-A) go-live, bump #2: the page (with fact 14's sentence), `PRIVACY_VERSION`, the panel's entry, the page pins in `site.rs`, one commit of five files, in L5's `console.js` slot | `contract-engineer`, then `contract-reviewer`; `reviewer` reads the page | the contract list; Quinn reads the page | L4 merged, T11f, P3, Quinn's Q-AGENT extension (Q-A) | 2 |
| T13 | Reference docs | `docs-keeper` | docs | P9 | docs push |
| T14b | PR-2 whole-branch review | `reviewer` (+ `contract-reviewer` over P1) | as T14a | all PR-2 tasks | 2 |
| P9 | PR-2: gate, merge | main session | as P8 | T14b PASS | 2 |
| P10 | Live proof on a scratch profile; cleanup | main session + Quinn | a live session | P9 | none |

**The order, in one line.** (T0 ∥ T12 ∥ T18 ∥ S1/S3 in Quinn's block) → #19's B8 → P0 → (T1 ∥ T4t ∥
T4dt ∥ T4ft ∥ T5t ∥ T15t ∥ T16t ∥ T6a once L3 has merged) → T2a → (T2b ∥ T3) → T2c → T4a → R1 → T4b → T4c ∥ T4d → T4e
→ T4d'a → T4d'c → T4d'b → P2 → T4f ∥ T4g ∥ T5 → T15 → T16 ∥ (T6b → T7a → T7b) → T14a → P8 (every lane ahead cleared) →
(T17t → T17 → T16s, after L7) ∥ (T8a ∥ T8b) → T9 → T10t → T10a → T10b → T10c → P3 → T11a … T11f
(queue slot) → P1 (same slot) → T10d (when Q39 holds) → T14b → P9 → T13 ∥ P10. PR-1 waits on L3 (T5,
T6a and T6b) and L6 (T5, T4g) but never on L7. Two test-writers or two implementers never share a file; `test-writer`
tasks on separate new files may run in parallel within the build-slot limit.

## 6. Task briefs

Each brief names the spec section that holds the behaviour, **extracted by its heading, never by line
number**, and the spec §13 row by its task id. A brief never restates code; where the spec gives a
case list, the test task takes every case in that row, including the cases the review and convergence
rounds added inside it.

### Before the cut

**T0 (docs-keeper).** *Files:* `docs/specs/2026-09-09-knowlu-cloud-design.md`, `VISION.md`,
`HANDOFF.md`. *Brief:* spec §0, "The amendment", applied by quoted text: ruling 5's new text; the
`VISION.md` "Syllabi" bullet (with the 2026-10-18 addition of the late-work, makeup and attendance
policies, Q34 and Q35 now ruled); the onboarding sentence; Beyond loses "syllabus upload with grade
weights,"; the open-decisions row "How a syllabus is read" closes; `HANDOFF.md` §3 gains the lane as a
wave-1 MVP lane (Q19). The signing date is 2026-10-19. *Stop rule:* a quoted "before" text that does
not match the file is reported, never guessed (the onboarding sentence has moved since the spec was
drafted). *After:* the controller's pending cloud-design commit (the 2026-10-19 bump-#1 amendment that
lands with `HANDOFF.md:530`), so the two do not mix. *Done:* a docs commit; `git diff` shows only those
edits.

**T12 (docs-keeper).** *Files:* `HANDOFF.md`, a dated row in the session ledger. *Brief:* spec §6.1
and §13 row T12. Check, statement by statement, that L4's draft (`docs/notes/2026-10-19-l4-privacy-page-draft.md`)
carries §6.1 items 1-6 as applicable and states 13,500 characters (Q38 ruled). List any gap for L4's
owner; never edit L4's draft or `site/privacy.html`. Add the release-guard line to `HANDOFF.md`'s
release gates: no release from a `main` carrying L5's app surfaces, or a curated syllabus host, before
the page carries the statements that apply (and, under Q-A, before P1). Write Q42's merge gate as a
ledger row for the controller. *Done:* the gap list (or "none") in the hand-off; the `HANDOFF.md` line.

**T18 (researcher).** *Files:* a new note `docs/notes/2026-10-2x-simple-syllabus-terms-first-read.md`.
*Brief:* spec §15.6's twelve clauses, each quoted with its section number and a one-line reading, from
the vendor's public terms and privacy policy, and whether the school publishes its contract terms with
the vendor. Flags, not a legal opinion. No student data is involved; the note names the vendor and the
public URLs only. *Done:* the note; a one-line answer to Q39's second condition ("no clause plainly
bars a student's own in-session read", or the clause that does).

**S1 (main session with Quinn, or the autonomous-proof harness on a scratch profile).** Spec §13 row
S1 and §3's step-1 row: can a student cookie session list a UA Blackboard course's contents
(`GET /learn/api/public/v1/courses/{id}/contents`, depth-limited) and download a file whose title
matches `syllabus`; which handlers appear; any Simple Syllabus LTI link. Shapes, counts and handler
names only. **§2's privacy rule applies in full.** A downloaded file is written only to one scratch
folder the controller creates for the spike, outside every working tree, `.claude/`, OneDrive and
`%LOCALAPPDATA%\knowlu\`. The spike records its size, content type, extension and whether text
extracts, never a line of its text, its file name or its course. *Deletion step:* when the spike ends,
the controller deletes that folder with the scratch profile's cleanup, unless Quinn names a file an
eval seed, in which case it moves to Q-E's folder and nowhere else. The ledger row records the count
deleted (or moved) and that `git status` in every worktree shows nothing new. *Done:* a note in
`docs/notes/` (shapes and counts only) and that ledger row; the scratch profile cleaned up.

**S3 (main session with Quinn).** Spec §15.7's seven questions, on a dev build in Quinn's block. A
diagnostic port is the spike's tool only; Google's refusal under a debugging port (ledger row 41) does
not apply to the vendor's SSO but is checked. Shapes, counts and header **names** only. **§2's
privacy rule applies in full** to every document the spike reads (each is a real syllabus). The
in-page read's results stay in the diagnostic session's memory or in S1's kind of scratch folder,
never in the transcript, a screenshot kept on disk, a note or the repo. The note records component
names, counts, sizes and which questions passed. *Deletion step:* at the spike's end the controller
deletes the scratch folder, any saved page or capture and the dev build's WebView2 data folder for
the vendor window, unless Quinn names a document an eval seed (then Q-E's folder only). It records
the count deleted in the ledger with a clean `git status` in every worktree. *Done:* a note in
`docs/notes/`, that ledger row, S2's cleanup list. If the script's same-origin call is refused, the
fallback order of Q29 decides T10b's read; the controller records which.

### P0. Cut, ledger, baseline, re-read (main session)

- [ ] After #19's B8, create `l5-syllabus` and its worktree from `main` (§2). Record the cut sha.
- [ ] Open `.superpowers/sdd/2026-10-19-l5-syllabus/progress.md` with this plan's path, §3's answers
      as dated rows, and Q42's gate (from T12).
- [ ] Baseline: engine build, workspace build and test under the slot guard; record passed, failed,
      ignored (3 if L6 has merged, else 4) and the warning tally.
- [ ] **Re-read on `main` and record as ledger rows** (each answer a task depends on; a surprise
      becomes a plan amendment before that task, never a mid-task guess):
  1. B3f's per-source table: its name, its columns, whether tiers are a number column or row order,
     where `registrar:` landed (still in `SERIES_KEY_PREFIXES`, or its own landing column), and the
     hold column (T4a).
  2. Any test that asserts a tier's number rather than an order between sources (Q48; T4a's stop rule).
  3. How #19's parser treats `EXAM` meetings (ruled: skipped and counted, close-out §8 Q10 (a)), so
     T4dt's "a syllabus final yields to a registrar exam" case is **not** built; a syllabus final is
     proposed as spec §4.6 says.
  4. `commitments --confirm`'s input: can it take a typed row (Q27; T11d chooses its branch).
  5. `onboarding::create_vault_in`: can it run an engine command at Finish (T8b; else the hand-over
     moves to the console's first open, spec §14).
  6. `approvals::apply_amendment` and `process_approvals`, read on `main` (the email branch's lines
     are cited):
     - whether `apply_amendment` can set a field the note lacks;
     - its `already_applied` gate (`:803-808`) and its from-loop (`:810-827`). T4e factors out the
       loop alone (G6);
     - what `comparable`/`plain` does with a mapping: it uses `{:?}`, so the result depends on key
       order (`:120-124`, `:519-527`);
     - what happens to an approved card the check calls stale. It gets a `stale amendment: … is …,
       expected …` warning, then its status goes back to `pending`, not archived and no decision. It
       is **not** counted pending in that run (`:1695-1705`; G5; T4d'a's count; P2 case (d));
     - the actor an approved non-mail amend is written under: the executor's context, `agent:approvals`
       (`:30-36`, `:1693`; G5);
     - **what the syllabus cards' rules rest on (fact 3, r6, r7).** Re-confirm each item:
       - `withdraw_stale` and `withdraw_asks` write only `status: superseded` and the archive, so no
         reason reaches a vault (*answered*, *unsupported*, *duplicate*);
       - `existing_source_uids` reads every status (G9);
       - card ids come from `ids::new_id` and travel unchanged, as file names do (G2's survivor order);
       - a pulled note never overwrites a file at its path (T4d's file names);
       - `amendments_pending` lists exactly the amend cards counted pending, and `render.rs`'s Must do
         reads it (T4d'a);
       - `Withdrawal`'s three fields, and `withdraw_asks` calling its reason in place (T4e's
         signature);
       - `count_proposals_created` reads `approvals/` and `archive/` by `proposed_at` (P2 (f)'s
         same-day variant);
       - `defer_over_budget` rewrites `proposed_at` and never `first_proposed_at`, and the mail
         hold's day-1 card carries `first_proposed_at` day 1 with `proposed_at` and `snooze_until`
         day 2 (T4d's day's 8 and its day-1 exception; r8);
       - `run_with` still calls `process_approvals` and then sets `remaining_budget` with nothing
         between them (`cli.rs:430`, `:439-440` on the email branch), and `commitment_passes` still
         re-counts at `:873` and `:881` (T4d'a's line; r8 M-1);
       - `approvals::count_proposals_created`, `planning::load_planning` (`planning.rs:65`) and
         `commitments::vault_day` are still public, so `read_inputs` can size the filer to the 15;
         `defer_over_budget` still sorts by `(expires, stem)` with a missing `expires` last and
         snoozes the tail past its allowance (`approvals.rs:1209-1223`); and the mail task card's
         `expires` is still `proposed_at` plus 14 days (`mail/text.rs:135`) (T4d's "The 15" and
         T4dt (B)'s held variant; r9);
       - `materialize` writes a task card's key into the task's `source_uid`, and `create_confirmed`
         writes an exam's key into its commitment's `source_uid` (G11's note half for items);
       - nothing on `main` writes the three fields yet (Quinn's premise).
     Also record: that no feature unification turns on `serde_json`'s `preserve_order`
     (`cargo tree -e features -i serde_json`; `engine/Cargo.toml:44` enables none), so a mapping sync
     carries comes back key-sorted (G6); and where `cardable` sits in `sync::apply`, with its three
     uses still reading the one closure (fact 3; T4e). Under Q-B no sync card is filed for the three
     fields, so the conflict card's mapping shape is not needed. No rule reads a journal record's
     actor or `device` (r6 retires G1, G3 and Q-L), so neither needs re-reading.
  7. `ingest`'s new-item path in `sync_tasks`, its actor and `via` (T4f).
  8. Where the grades read model's `letter` lives after L2 (T4g).
  9. The mail window's runner and `Page` seam on `main` (T10b: shared or copied).
  10. The kind checks (`grep -n "kind in ('task', 'event', 'email')" cloud/supabase/migrations`) and the
      newest stamps on `main`, L3's and L8's recorded stamps, and staging's applied list (T6a, M3).
  11. Whether L4 has merged, and L4's ruling on its Q2 (Q-A); whether L6 and L7 have merged.
  12. Whether every course has a course note with an `id:` when the pass runs (T5; spec §14).
  13. Whether `is_human_set` (`commitments.rs`) and #19's ended path's hand-set rule on `until` read
      the same journal test for every source, so a parity case works (T4t, finding 6).
  14. Once L4 and L6 have merged (re-read before P3 and T8a): L4's gate type and its refresh call, the
      place the slot builds `judge`'s argv after L6, L4's `PRIVACY_CHANGES` shape (P1), how Quinn
      ruled L4's Q-GATE (P3 rule 3) and L4's draft Q3 (P1 item 1), and the exact name of L4's T9
      change-rule pin in `engine/tests/site.rs` (P1 item 4).
- [ ] Hand T1, T4t, T4dt, T4ft, T5t, T15t, T16t and T6a their briefs with the answers they need.

### Engine: reading a file (PR-1)

**T1 (test-writer).** *Files:* `engine/tests/syllabus.rs`, `engine/tests/syllabus_data/` (new, text
files only). *Cases:* spec §13 row T1, every case, with one change
Q38's ruling forces: the total `MAX_SYLLABUS_CHARS` is pinned at **13,500** from the start (four parts
at 400, 1,600, 7,000 and 3,000, and 1,500 reserved for the policies part T16t pins), so no later task
edits a pin. Behaviour sources:
§4.1 "Fetch (device), and where fetched bytes wait" (the file form, the outcome form, one copy per
course, who may replace it by `source`) and §4.2 "Text and screen (device, engine)". The state file's
keys are §5's list under "The extraction file is a byte contract from its first release", in
`ledger::dumps_value` bytes, `"version": 1`, with `source` and the per-part states including `empty`.
One case (r10b, the r10a review's M-2) pins version 1's empty outcomes: a fresh `syllabus-add`,
`pending` or `read`, writes the key `item_outcomes` as an empty list (`[]`), with the file's keys
sorted, as `ledger::dumps_value` writes them (r10c, the r10b review's M-5).
Exit 2 on bad input writes nothing (an unknown course, an unreadable path); `syllabus-add` writes no
note and no journal record (assert the journal folder is unchanged). **Binary inputs are built in
the test, with no new dependency** (T3 alone owns `engine/Cargo.toml`). DOCX goes through the engine's
existing `zip` dependency: a `[Content_Types].xml`, a `word/document.xml` with an invented table, and
nothing else. PDF is **hand-built minimal PDF bytes**: one helper that writes a one-page PDF 1.4 with
one uncompressed content stream, a standard-14 font (`/Helvetica`, no embedded font), one `Tj` line
per invented table row at falling `y`, and an `xref` table whose offsets the helper computes from the
bytes it has written. That gives about 40 lines, which is the test's own fixture, never a file. A
case that needs a PDF is `#[ignore]`d **only until T3**, with that reason in the attribute, and T3
removes the attribute (it is not one of the four by-design ignored tests and must not ship ignored).
If T3's chosen crate cannot read the hand-built bytes, T3 stops and reports. The controller then
amends the plan for a committed **invented** PDF under `engine/tests/syllabus_data/`, with its `-text`
line in `.gitattributes`, made by the controller from invented text. A real syllabus is never used. *Done:* red for "no `syllabus` module / no `syllabus-add` command"; the hand-off lists each
test with its reason. Commit: "test(l5): syllabus text, screen and syllabus-add, red".

**T2a (implementer).** *Files:* `engine/src/syllabus.rs` (new), `engine/src/lib.rs`. DOCX through the
existing `zip` dependency (`word/document.xml`, one line per table row, cells joined by `" | "`), HTML
with tags stripped and rows joined the same way, TXT as is; CRLF handled as every vault read is. No
new dependency. *Done:* T1's text cases green.

**T2b (implementer).** *Files:* `engine/src/syllabus.rs`. §4.2's four parts with their bounds, the
dropped instructor lines, office-hours contacts replaced by `[removed]`, the line-boundary clip with
the earliest schedule lines kept and the clip recorded, unused bound never lent, and §4.2's rules per
part ("Rules settle what they can, per part"). "No part left: no call" is a value the pass reads, not
a call. *Done:* T1's screen and rule cases green.

**T2c (implementer).** *Files:* `engine/src/syllabus.rs`, `engine/src/cli.rs`, `engine/src/main.rs`,
`engine/src/backup.rs` (one array entry and its in-module pins). `syllabus-add`'s file and outcome forms (§4.1), the copy to `<vault>/syllabi/<slug>.<ext>` with the
replacement rule, the state file through `ledger::dumps_value`, `pending` or `read` by whether any part
is left for the model. **The one writer (r10a, the r10 review's M-2; pinned in r10b, the r10a
review's I-1 and M-2):** the state file is written by one module, `syllabus.rs`, built here, in two
forms, whose names T4dt and T4d'a use and the implementer never renames:
- `syllabus::write_state(vault: &Path, slug: &str, state: &State) -> io::Result<()>` writes the
  whole file (`syllabus-add`; `judge`'s pass, T5).
- `syllabus::write_outcomes(vault: &Path, slug: &str, seen_sha: &str, seen_added: Date, outcomes:
  &[(String, &str)]) -> io::Result<Written>` (`rank`'s hook, T4d'a), each pair a `source_uid` and
  an outcome code. It re-reads the file and returns `Written::Skipped`, writing nothing, when the
  file is gone, its `version` is unknown, or its SHA or `added` differs from what `read_inputs`
  saw; otherwise it replaces only `item_outcomes` and returns `Written::Changed`, or
  `Written::Unchanged` when the bytes would be the same.
Both write beside and rename over, as `grades.rs:583-586` does, so no reader sees a half file. The
window left is the re-read-to-rename instant, not step 1 to step 5. Version 1 carries
`item_outcomes`, a list of `{"source_uid", "outcome"}` in the listing's order, from the first
release; `syllabus-add` writes it `[]`, and the codes are `filed`, `later`, `past`, `duplicate`,
`needs-date` and `feed-differs`. `syllabus.rs` holds them as `ITEM_OUTCOME_CODES` (exactly these
six), and `write_outcomes` refuses a pair whose code is not listed: it returns an `InvalidInput`
error and writes nothing (r10c, the r10b review's M-2). Ungated (no `entitle` call), offline (no client constructed). The `--source`
values are a closed list; `lms` is accepted only as the wizard hands it over (T8b).
**The backup keeps the copies (r1 finding 8, decided):** `engine/src/backup.rs`'s `BACKUP_FOLDERS`
gains `"syllabi"` after `"profile"`. Otherwise the student's own backup, a folder they own, drops every
syllabus copy, and a restore leaves `state/syllabi/` naming files that are gone. This is the student's
own storage, as `config/` already is, and it is not sync (spec Q8 holds). The two pins that state the
array's tail and length (`backup_folders_is_every_note_folder_in_order_then_the_three_it_also_owns`
and `backup_folders_has_eleven_entries_with_commitments_then_grades_before_state`) change in the same
commit, with the reason in its message; a new in-module case mirrors a vault's `syllabi/` file.
`ids::NOTE_FOLDERS` is not touched (contract list; `syllabi/` holds no notes), so
`note_folders_is_a_subset_of_backup_folders` stays as is. *Done:* all of T1 green except the PDF
case; `git grep -n "entitle::" engine/src/syllabus.rs` empty; the backup tests green. Commit:
"engine(l5): syllabus-add keeps the copy and screens it; the backup keeps syllabi/".

**T3 (implementer, then reviewer).** *Files:* `engine/Cargo.toml`, `Cargo.lock` (by the build), the
PDF branch in `engine/src/syllabus.rs`, a note in `docs/reference/toolchain.md`. Spec Q7: one pure-Rust
PDF text crate, `rustls`/`ring` only, no OpenSSL, no `tauri`; `engine/tests/dependency_boundary.rs`
green and unedited; the release bundle's size change measured and recorded (the base app stays under
50 MB); table rows keep their order on T1's in-memory PDF. If no crate passes, the fallback is Q7 (c):
PDF is a named outcome ("Knowlu reads PDF, Word, web pages and text files" stays true only if PDF
works; otherwise the controller words the outcome with Quinn) and the ignored case is deleted with the
reason in the commit. *Done:* T1 fully green; `reviewer` PASS on the dependency.

### Engine: meetings into commitments (PR-1)

**T4t (test-writer).** *Files:* `engine/tests/syllabus_commitments.rs` (new). *Cases:* spec §13 row
T4t, **every case in it**, including the convergence-round cases: the dates the ended arm stamps (the
Friday lab dropped on a Wednesday run, the null-`until` case, a series past its own `until`, a series
not yet begun, Q49 ruled (a)), the swap-versus-two-cards cases, the usable-meetings-read cases for each
of the four conditions and for `pending` and `failed`, Q50 (a)'s second cost pinned as written, the
precedence **orders** (never tier numbers), the agreeing-pair collapses, `instances_map`, the registrar
hold, `contested`, the section match, and the ask gate's cases. **The hand-set case (spec T4t, "a
hand-set field follows the change path's existing rule"; r1 finding 6), pinned as parity, never as a
restated rule:** a confirmed `syllabus:` note where the journal shows the student set one field by
hand (`room`, say), then a re-added syllabus that changes that field and one other. The test builds
the same vault with a `google:` twin of the same note and the same change, runs `commitments` over
both, and asserts that the two card sets agree on kinds, the fields each card proposes, and what is
written to the note (nothing). A second parity case does the same for a hand-set `until` reaching the
ended arm. **Moved out (r1 finding 7):** the `sync::is_note_path` case is P2's, under
`contract-engineer`, because sync tests are on the contract list. Two corrections from P0's facts:
the cases that name `SERIES_KEY_PREFIXES` assert behaviour (a changed confirmed `syllabus:` series
files a change card and applies nothing), not the array; and nothing pins registrar behaviour (#19's
tests own that row). Synthetic vaults built in temp folders; the series file written through the
engine, never by hand. *Done:* red for "no syllabus source"; every pre-existing test green.

**T4a (integrator, Opus high).** *Files:* `engine/src/commitments.rs`, `engine/src/cli.rs` (the
syllabus source read beside the Google and ICS normalisation). *Behaviour:* spec §4.4, "Meetings into
the engine (device, deterministic)", all of it, and §9's first bullet: the key; the `syllabus:` row of
#19's table (tier 1, the two arms, no instances, no per-kind hold, change lands as a card through
`detect_changes`); the renumbering (Google 2, default 3), or nothing if B3f orders by row; the usable
meetings read from §5's per-part state; the deletion arm with its second trigger (Q15, Q50 (a)); the
ended arm's stamp ("What the arm stamps"); syllabus series out of `instances_map`;
`create_confirmed_as`'s "Found in your syllabus." body line. **A hand-set field on a syllabus series**
lands by the change path's existing rule, unedited (spec §4.4: changes land "only as a card through
`detect_changes`, the §5.4 change path, under that path's existing hand-set rule (§4.5)"; §4.5: a
field the journal shows the student set by hand, read by `journal::human_edited` and
`commitments::is_human_set`, "is never proposed back over"; and on the ended arm, "the hand-set rule
on `until` still holds"). T4a adds no hand-set test of its own and no branch for it: the `syllabus:`
row reaches `detect_changes` and the ended path the way a `google:` series does, and T4t's parity
cases prove it. *Never:* an edit to another row's columns, to #19's tests, to `detect_changes`'s
ended path or its hand-set check, or to `LOCAL_CARD_KINDS`. *Stop rule:* Q48; and if the parity
cases cannot pass without a syllabus-specific branch, stop and report. *Done:* T4t's T4a cases green; every #19 test green and unedited (`git diff main -- <#19's test files>` empty);
`rank` over `vault-full` byte-identical; fixtures diff empty. Commit: "engine(l5): syllabus: joins the
per-source table".

**R1 (reviewer).** Report `docs/reports/2026-10-2x-l5-t4a-review.md`: the row and its arms against
§4.4; the four `precedence` readers keep every existing pair's order; the usable-read predicate reads
all four conditions; the stamp's clamps in order; no registrar behaviour moved; a hand-set field
reaches the existing check with no syllabus-specific branch.

**T4b (implementer).** *Files:* `engine/src/commitments.rs`; `app/src/week.rs` only if
`commitment_proposals` filters fields. §4.4 "Section match" and "Contested rows": `contested: bool` on
each proposal, in `--json`, through to the envelope; no tier, no row. *Done:* T4t's T4b cases green.

**T4c (implementer).** *Files:* `engine/src/commitments.rs`. §8 "The ask card waits for a syllabus
being read, and only for a bounded time (Q16)": `SYLLABUS_WAIT_DAYS` = 2 inside `emit_asks` after its
`ASK_FROM_DAY` check; #19's `asks_wait_for_registrar` stays in `cli.rs` and both must allow. Each
course's outcome is carried for the read model. *Done:* T4t fully green.

### Engine: cards, the contract arm, adoption, weights (PR-1)

**T4dt (test-writer).** *Files:* `engine/tests/syllabus_cards.rs` (new). *Cases:* spec §13 row T4dt,
every case, in three groups so each task's Done names cases that task can turn green (r8, the r7
review's I-1). **Dropped by P0's fact 3:** "a syllabus final yields to a registrar `exam`"
(registrar finals are skipped, close-out Q10 (a)).
**The pure plan's interface, named here so the plan cases can be written before T4d (r8 I-1).**
`syllabus_cards::read_inputs(vault: &Path, today: Date) -> Inputs` reads, once, everything T4d'a's
step 1 lists and writes nothing. `syllabus_cards::plan(inputs: &Inputs, today: Date) -> Plan` is
pure. `Plan` has three public fields: `withdrawals`, a `BTreeMap<String, Reason>` from a card's file
stem to its reason; `filings`, a `Vec<PlannedCard>` in filing order (Q41's order within the
day's 8); and `listing` (r10a, the r10 review's M-2), a `BTreeMap<String, Vec<ListedItem>>` from a
course slug to its extraction's dated and undated items in date order (undated last), items on one
day in extraction order, then by `source_uid` (r10b, the r10a review's M-3), and undated items
among themselves the same way (r10c, the r10b review's M-5). Each item gets one code, the first
that holds in this order: `Filed`, `duplicate`, `feed-differs`, `needs-date`, `Past`, `Later`
(r10c, the r10b review's M-4). A `ListedItem` exposes
`source_uid` (the item key) and `outcome: Outcome`; `Outcome::code()` gives spec §5's per-item code.
`Filed` is an item this run plans a card for, or whose key is held by a card that blocks it under
G9 (so a key whose only history is a `superseded` card is not `Filed`; r10b, the r10a review's
M-1). `Later` is a dated item, not before the charge day, that is not `Filed` and is outside the
horizon measured from the charge day, or inside it but not fitted by "The 15" this run (it is
planned again next run, so "proposed nearer the time" holds). `Past` (r10b, the r10a review's I-2)
is a dated item before the charge day that is not `Filed`; on day 1 that includes an item due on
day 1. The other codes (`duplicate`, `needs-date`, `feed-differs`) are spec §5's, set by the rules
that already produce them. The six codes are version 1's (place 10's twelfth change). `PlannedCard` exposes at least `kind`, `source_uid`, `proposed_at`, `snooze_until`,
`first_proposed_at` and `expires` (`Option<Date>`, `None` for `null`; r10, the r9 review's M-2).
`Reason::word()` gives the run line's word. `Inputs` is opaque to the tests. `Date` is the engine's
own. The implementer may add fields and private helpers, never rename these.
- **(A) The filer's plan cases (T4d's Done).** Each builds a scratch vault, calls
  `plan(&read_inputs(&vault, today), today)`, and asserts on `Plan`. The vault's bytes are unchanged
  afterwards. The cases: date and week resolution (no `week1_start` and over 9 months each need a
  date); the in-class time from a confirmed class note; the key and its `@date` suffix; the 14/21-day
  horizon, never past, inclusive at its far edge (r10a, the r10 review's M-1, a controller ruling),
  one case per kind: an assignment due on the charge day + 14 plans a card and one due + 15 does
  not; an exam due on the charge day + 21 plans a card and one due + 22 does not; the 8 a day in Q41's order, counted by `first_proposed_at` (a card snoozed
  to tomorrow still counts today, and not tomorrow); the 15's room (r9, the r8 review's I-1): with
  10 cards charged to today, `filings` holds 5, the first 5 in Q41's order; with 15 or more, none;
  on day 1, with 10 held cards charged to day 2 and none to day 1, 5 held cards; each planned card's
  `expires` (an item's day for a task or exam card, `null` for an amend card); the day-1 hold (`proposed_at` and `snooze_until` day 2,
  `first_proposed_at` day 1), with day 1 planning against day 2's 8 and day 2 planning none while 8
  held cards stand; the day-1 horizon (r10, the r9 review's I-1): on day 1, an item due on day 1
  plans no card, an item due on day 2 plans a held card (`expires` day 2, so it is still pending
  when `process_approvals` wakes it on day 2), and the 14 days count from day 2 (an assignment due
  on day 16 plans a held card, one due on day 17 does not; r10a); each §4.6 duplicate case (equal title, token, kind-and-number; same day; undated note; a
  feed item on another day gives no card; a mail card on another day files); a `coursework` item on
  another day plans no card (Q45); a key in `existing_source_uids` is never planned; G2's filing
  half; grading validity and the grading `source_uid`; a dropped item plans nothing; T4d's two
  reasons as entries in `withdrawals` (*date moved* for an item still present on a new day,
  *authoritative match*); and the listing's `later` boundary (r10a, the r10 review's M-2): on day 1,
  with room in day 2's 8 and 15, an assignment due on day 16 is in `filings` as a held card and is
  `Filed` in `listing`, and one due on day 17 is in no filing and is `Later`; and `Past` (r10b, the
  r10a review's I-2): on a day after day 1, an item due the day before with no card is `Past` and
  is in no filing; the precedence (r10c, the r10b review's M-4): a past item that matches a task
  note is `duplicate`, not `Past`; and the code set (r10c, the r10b review's M-2): every
  `Outcome` variant's `code()` is in `syllabus::ITEM_OUTCOME_CODES`, which has exactly six entries.
- **(B) The filer's `rank`-level cases (T4d'a's Done).** Each runs `rank` end to end.
  - **The listing written (r10a).** After `rank`, each course's `state/syllabi/<slug>.json` carries
    `plan`'s `listing` as its `item_outcomes`, and every other key's value is unchanged from the
    file before the run (r10b, the r10a review's M-2). A second `rank` the same day leaves the file's bytes
    unchanged. A course whose file has an unknown `version` is not written.
  - **A file changed under the hook is not overwritten (r10b, the r10a review's I-1).** The one
    case here that does not run `rank` whole, since no test can step between its reads and writes:
    it drives step 5's pieces in order: `read_inputs` over a vault whose `bio-101` file is `read`; the
    test then replaces that file as `syllabus-add` would (a new SHA and `added`, `pending`); then
    `syllabus::write_outcomes` with the SHA and `added` the original file held returns `Skipped`,
    and the file holds the replacement's bytes. Run again with only `added` differing: `Skipped`
    too. With neither differing it returns `Changed` and only the outcomes key differs.
  - Every card the run files is read back for its synced fields (spec §4.6, "What each card carries,
    since every one syncs"), including Q46's `where` capped at 80 characters.
  - Each card's `create` record precedes its file, under `agent:knowlu.syllabus` with the run's
    `via`. Two scratch vaults filing one key choose different file names.
  - The `coursework` item on another day shows the notice (Q45).
  - `rank`'s output over `vault-full` with no `state/syllabi/` is byte-identical.
  - **Day 2's sum of 15 (spec §16.8; pinned by T4d'a's line and T4d's "The 15"; r8 M-1, r9 I-1
    and M-1).** Day 2 of a vault, the budget 15, an extraction that arrives on day 2 with at least 8
    items in the horizon, more event obligations than could fit, and *k* mail cards charged to day 2
    before `rank`. Each mail card is written in `task_card_text`'s shape (`mail/text.rs:134-146`;
    `expires` is its `proposed_at` plus 14 days).
    (i) With *k* = 4, one `rank` charges exactly 15 (`count_proposals_created`): 4 mail, 8 syllabus,
    3 `event-accept` cards. No card is snoozed, and the other obligations are not filed.
    (ii) With *k* = 10, one `rank` charges exactly 15: 10 mail and 5 syllabus cards, the first 5 in
    Q41's order, and no event card. No card is snoozed. A second `rank` the same day files nothing
    and snoozes nothing: 3 of the day's 8 are left, but none of the 15. The case runs twice, once
    with the 10 cards filed `pending` on day 2 (so `defer_over_budget` sees them) and once held
    from day 1 (`snoozed`, `proposed_at` and `snooze_until` day 2, woken by `process_approvals`
    after `defer_over_budget` has run). Both give 10 + 5 = 15.
    *(ii), held variant: overflow that held mail cards make, as today.* `defer_over_budget` reads
    only `pending` cards, and `process_approvals` wakes held cards after it (`approvals.rs:1203`,
    `:1581-1602`). The mail step does not size itself. So held mail cards can take day 2 over the
    15, and only a later run trims the day. This is existing mail behaviour, and the case pins that
    the syllabus cards add nothing to it.
    1. Day 1, slot 1: 4 held mail task cards (`expires` day 16), then `rank`. Unlike (i) and (ii),
       the extraction is in place on day 1. It gives seven assignments due on days 3 to 9 and a
       valid grading set, and no obligation event is in the feed. The hook plans
       min(8, max(0, 15 - 4)) = 8 held cards: the seven task cards (`expires` days 3 to 9) and the
       grading card (`expires: null`).
    2. Day 1, slot 2: 6 more held mail task cards (`expires` day 16), then `rank`. The hook files
       nothing, since day 2's 8 is used and 15 - 18 < 0.
    3. Day 2, first `rank`: `defer_over_budget` finds no `pending` card and snoozes nothing.
       `process_approvals` wakes all 18, the hook files nothing, and no event card is filed. Day 2 is
       charged 18.
    4. Day 2, second `rank`: `defer_over_budget` has 18 candidates and an allowance of 15. It sorts
       by `(expires, stem)`, with `null` as the latest date, keeps the first 15 (the seven syllabus
       task cards and the eight mail cards with the lowest stems) and snoozes the rest to day 3: the
       **grading card and the two mail cards with the highest stems**. The test names the three
       stems. Day 2 now charges exactly 15, and the hook files nothing.
    The day's 8 still counts the snoozed grading card toward day 2 (`first_proposed_at` day 1,
    the day-1 exception), so day 3's 8 is not charged for it; `defer_over_budget` never rewrites
    `first_proposed_at` (`approvals.rs:3724`'s test).
    (i) fails at any later hook line, where the events pass would be sized before the syllabus cards
    exist. (ii) fails without T4d's "The 15" (that run would charge 18). The held variant fails if an
    item card's `expires` is `null`, since `defer_over_budget` would then snooze syllabus task cards
    ahead of the mail cards.
- **(C) The withdrawal cases (T4d'c's and T4d'b's Done).** Through `approvals::withdraw_syllabus`
  end to end in `rank`: a moved date archives as `superseded` and refiles under the same key; an
  archived `rejected` card blocks for good; snoozed; `syllabus-exam`; the grading hash; the due
  amend card's key; the half-moved card; untouched commitment cards. Then the rules cases 1-10 below.
**The syllabus cards' rules, on one desktop (Q-B, Quinn's simplification and the two 2026-10-20
rulings; spec §4.6 place 10; G2, G5-G11).** Each case runs `rank` end to end. It asserts what
`approvals/` and `archive/` hold afterwards, and how many grading cards were ever filed:
1. **Answered by a hand edit in the file (G7; r5 M2).** A pending grading card, then the student
   edits `grade_weights` in the course note in a text editor: the file changes, with no journal
   record. The next `rank` archives the card as `superseded` and files nothing. `detect_external`
   journals the edit first, and G7 reads only the value. Two more runs file nothing. A new extraction
   with a new hash then files that key.
2. **Answered by an agent write (G7).** The same, but the field moves through `write` under
   `agent:approvals`, as another desktop's approval lands. The outcome matches case 1, since the actor
   is never read.
3. **One field of two (G7).** A two-field card (`grade_weights`, `grade_scale`). One field moves to a
   third value and the other is untouched: the card is withdrawn whole and nothing is filed. In a
   variant, one field moves to the card's `to`: withdrawn, nothing filed.
4. **Only what differs (G10; r5 I3).**
   - A new key whose carried values the note already holds files nothing.
   - A new key that differs in one of two fields files a card carrying that field alone, with
     `from` the note's value.
5. **Decided cards (G9).** An approved and a rejected grading card whose fields later move. Nothing
   is filed again under either key, on this run or the next two.
6. **Unsupported and set changed (G11, G8; Quinn's h2 → h3 example; r6 M-1).**
   - *Unsupported.* A pending card under h1 carries `grade_weights` W1, and a new extraction h2 gives
     W2. h1's card is withdrawn as *unsupported*, and h2's card is filed in the same run, carrying W2
     alone.
   - *Set changed.* A pending card under h1 carries only `grade_weights` W1 (the scale was held at
     filing), and h2 gives W1 and a new scale S2. h1's card is withdrawn as *set changed*, and h2's
     card is filed in the same run, carrying both fields.
   - *No room.* The set-changed case on a day whose 8 is already used: nothing is withdrawn and
     nothing is filed. The next day's run withdraws h1's card and files h2's in the same run.
   - *Quinn's example.* Then an extraction h3 whose values the note already holds: h2's card is
     withdrawn as *unsupported*, nothing is filed (G10), and the next two runs file nothing.
   - Then the extraction back at h2: nothing is filed (G9), and no grading card is pending.
   - *A failed re-extraction keeps the card (r8, the r7 review's I-2).* A pending card under h1
     carries `grade_weights` W1 and `grade_scale` S1. A new extraction reads weights that sum to 110
     and a scale whose minimums do not fall strictly, so §4.7's check gives no valid set. The next
     `rank` leaves the card pending, byte for byte, withdraws nothing and files nothing, and so do two
     more runs. In a variant only the weights fail and the scale is still S1. The read's key now
     hashes the scale alone, but the card stays and nothing is filed: G8 replaces no card with a
     field the set leaves absent, and the card blocks the narrower key (G2's filing half). In a
     second variant the grading part is not read at all (`none`): the card stays. A valid extraction
     giving W1 and S1 back then leaves the card pending under h1 and files nothing (G9 and G2). The
     course note is set up so that W1 and S1 both differ from it.
7. **The duplicate rule (G2; r5 M4, r6 M-2).** A second pending grading card under the same key,
   written as sync writes a pulled note: its file in `approvals/` and its `create` record in the
   journal, with its own random id.
   - `rank` keeps the lower `id:`, archives the other as `superseded` (*duplicate*) and files
     nothing. The same holds with the second card snoozed.
   - *Tie-break.* With one card lacking a readable `id:`, the card with one survives. With both
     lacking one, the lower file name survives.
   - *Unsupported first.* With the pulled card carrying a value that differs from the valid one the
     extraction gives, it is withdrawn as *unsupported*, whatever its id.
8. **No extraction here (G7, G11's note half).** A vault with no `state/syllabi/`, holding pending
   grading cards as a second desktop holds synced ones. Two pending cards for the course both stay,
   since G2 decides only where the extraction is. Each is withdrawn when its field moves, never for
   any other reason, and nothing is ever filed.
9. **The pending line (T4d'a's count; G7, so T4d'b's Done).** An approved grading card whose field
   moved before the run.
   - The run's warnings carry today's `stale amendment: grade_weights is …, expected …`.
   - The same run archives the card as `superseded`.
   - The `Approvals: N pending` line equals the number of pending cards left in `approvals/`.
10. **The item rules (T4d'c; G11's item half, the twin; r6 I-2).**
   - *Duplicate.* Two pending task cards for one item key at day d1, the second written as a pulled
     note. The first by the survivor order stays (*duplicate* for the other), and nothing is filed.
   - *Moved.* The extraction moves the item to d2. The survivor is withdrawn as *date moved*, and a d2
     card is filed in the same run.
   - *Moved back (Quinn's example).* The extraction returns to d1. d2's card is withdrawn as *date
     moved*, and nothing is filed, since d1 was filed before. No card is left for the item.
   - *The gap (M-1).* On a day whose 8 is used, a move to a new day d3 withdraws the card at once.
     The d3 card is filed on the next run that has room.
   - *A due already there.* An approved syllabus task is due d3, with a pending due amend card whose
     `to` is d2 (urgency `increases`), and today.md's Must do lists that card. The student sets `due`
     to d2. The next `rank`, on a vault with no `state/syllabi/` too, archives the card as
     *unsupported* and files nothing. Must do no longer lists it, and the run's `amendments_pending`
     lacks its stem (r6 I-2).
   - *A key already held.* A pending task card whose key a task note already carries as
     `source_uid`, because another desktop approved its twin: withdrawn as *unsupported*, on a vault
     with no extraction too.
   - *Absent, not moved (r8 I-2).* A pending task card at d1, then a re-extraction that drops the
     item: the card stays pending and nothing is filed. In a variant the item is still read but its
     day no longer resolves (no `week1_start`, so it needs a date): the card stays, and the course
     lists the item as needing a date.
No test sets `KNOWLU_DEVICE`, since no rule reads a device or an actor.
*Done:* red for "no filer".

**T4d (implementer, then reviewer).** *Files:* `engine/src/syllabus_cards.rs` (new), `engine/src/lib.rs`.
*Behaviour:* spec §4.6 "Dated items into proposals (device, deterministic, in `rank`)" (date and time
resolution, the key, the horizons and the 8 a day in Q41's order, the day-1 hold by
`commitments::vault_day` as mail cards are held, the duplicate rule with Q45's widening, "Superseded is
not a decision"), §4.7 (whole-or-nothing grading, the amend card and its hash), the pure
`withdrawal_reason` with *date moved* and *authoritative match*, and the course listing for items not
filed (spec §13's T4d row): `plan`'s `listing` field (T4dt's interface), which T4d'a writes into
the state file and T17 carries in `syllabi`'s `items` and `counts`; T11b words the "N more dates"
line. `Later` uses "The horizon" below, measured from the charge day, so the listing never counts a
held card: on day 1 an assignment due on day 16 (an exam on day 23) is a held card, `Filed`, and
one due on day 17 (an exam on day 24) is `Later` (r10a, the r10 review's M-2). Actor `agent:knowlu.syllabus`, via the run's existing `VIAS` value. It writes only new card
notes through `write`, and never stamps, moves or deletes an existing card. Eight rules (Q-B's
rules; the fourth new in r8, the fifth and sixth in r9, the seventh in r10):
- **G9.** "Superseded is not a decision" unblocks item keys only (T4d'c narrows it to a day never
  filed). A grading key is blocked by any card in `existing_source_uids`, whatever its status.
- **G2's filing half.** A grading card is filed only when no pending or snoozed grading card for the
  course is left after this run's withdrawals, whoever filed it. A card that G8 would replace counts
  as withdrawn here. G8 then withdraws it only if the plan in fact files the replacement (T4d'b,
  steps 2 and 3).
- **The filer is a plan, then writes (r7, for G8 and r6 M-1; named in r8, I-1).**
  `syllabus_cards::read_inputs` reads the run's inputs once, and `syllabus_cards::plan` is the one
  pure step (the interface is T4dt's, above). It returns this run's withdrawals, keyed by file stem,
  and the cards this run files, in Q41's order within the day's 8. Its inputs are the extraction, the
  cards in `approvals/` and `archive/`, the course and task notes, and `existing_source_uids`. It
  decides the withdrawals first, so the filing half sees the queue after them. `withdrawal_reason`
  stays the per-card pure function the spec names, and `plan` calls it. A second step writes the
  planned cards through `write`.
- **The day's 8 (r8, found while pinning M-1).** It counts the syllabus cards in `approvals/` and
  `archive/` by the day each was first proposed, as spec §16.8's "first proposed a day" words it,
  since `rank` keeps no state. That day is the card's `first_proposed_at`, which
  `defer_over_budget` never rewrites (`approvals.rs:3724`'s test), so each card takes one slot of one
  day's 8. Counting `proposed_at` instead would move a snoozed card into the 8 of the day it wakes
  on, taking a slot from a new item there. (r8 also said it let a same-day rerun refill the 8 after
  a snooze. Under "The 15" that cannot happen either way, since a day `defer_over_budget` trims has
  no room left in the 15; r9 corrects the reason.) One exception keeps the hold whole: a card
  filed on the vault's day 1 counts toward day 2, the day it is held to. The mail hold writes
  `first_proposed_at` day 1 with `proposed_at` and `snooze_until` day 2 (`mail/tests/cards.rs:77`),
  and the syllabus hold writes the same fields, so its age is the mail card's. So day 1 plans
  against day 2's 8, and day 2 adds nothing beyond it. A pulled card counts toward its own day (P2
  (f)'s same-day variant).
- **The 15 (r9, the r8 review's I-1).** `plan` files at most the smaller of what is left of the
  day's 8 and `max(0, budget - count_proposals_created(vault, charge_day))`. `budget` is
  `planning.daily_approval_budget`. The charge day is today, or day 2 when
  `commitments::vault_day` is 1, the day the held cards charge. This is modelled on `emit_checks`'
  sizing (`cli.rs:873`), which counts today only; the charge-day form is new here (r10, the r9
  review's M-3). It is read at the hook's line, so the count already holds every card charged before it
  (mail, held cards `process_approvals` woke, earlier slots, pulled cards). `read_inputs` reads the
  budget through `planning::load_planning` and the count once; `plan` stays pure. Q41's order picks
  which cards fit. *What it gives:* no syllabus card ever takes a day over the 15, and the events
  and commitment passes, sized after the hook, keep what is left. *What it does not change:*
  `defer_over_budget` and its order. A card charged after the hook ran (a later slot's mail step, or
  a held mail card filed on day 1 after day 1's hook) can still take a day over, as today, and the
  next run trims it in `defer_over_budget`'s order, which can snooze a syllabus card (T4dt (B), (ii)'s
  held variant). An item that does not fit is not removed from the plan: it is planned again on
  each run while it is in the horizon and ahead, and gets a card only if one of those runs has room
  (bounded as T4d'a's "The other side" states; r10). Wherever this plan says a card files "within
  the day's 8" (G8, G11's gap, T4d'b step 3, place 10), it means within this allowance.
- **The horizon (r10, the r9 review's I-1).** The spec's "within 14 days (an assignment or quiz) or
  21 days (an exam) of today, and never once past" is measured from the charge day, the same day
  "The 15" uses: today, or day 2 when `commitments::vault_day` is 1. On day 1 an item due on day 1
  plans no card, and an item due on day 2 plans a held card whose `expires` (day 2) is not before
  the day it wakes, so `process_approvals` keeps it `pending` (`approvals.rs:1616-1617` expires only
  `expires < today`). *Why:* measured from day 1, an item due on day 1 would plan a card held to day
  2 that `process_approvals` wakes and expires in the same call (`:1596-1627`). Q41 puts such
  items first, so they would take the first of day 2's 8 and 15 for cards nobody sees, push later
  items back a day, and leave `expired` history on their keys. Measured from day 2, day 1 plans
  exactly what day 2's horizon holds, which matches the 8's day-1 exception. Every other day the
  charge day is today, so nothing else changes. Place 10's eleventh change carries it.
- **`expires` (r9, the r8 review's M-1).** A task card's is its payload's `due` day, a
  `syllabus-exam` card's is the exam's date, and an amend card's (due or grading) is `null` (Q-B's
  "*`expires`*" bullet says why). Under "The horizon" no planned task or exam card has an
  `expires` before its charge day.
- **File names.** Each card's file name carries a random part, as `write::propose_amendment` names
  `amend-<stem>-<6 hex>` (`write.rs:1036-1040`). Two desktops filing one key then never choose the
  same path (fact 3: a pulled note never overwrites a file at its path).
T4d's *date moved* withdraws whatever the filer then does, as the spec has it; under Quinn's I-3
ruling it is G11's syllabus half for items. *Answered*, the grading *unsupported*, *set changed* and
G2's survivor are T4d'b's. The item *unsupported*, the twin's day rule and the survivor order are
T4d'c's.
*Done (r8, the r7 review's I-1):* T4dt's group (A), the filer's plan cases through `read_inputs` and
`plan`, green, with the interface exactly as T4dt names it. Nothing more is claimed: group (B) needs
T4d'a's hook and group (C) needs T4e, T4d'a, T4d'c and T4d'b. `reviewer` PASS (report in
`docs/reports/`).

**T4e (contract-engineer at xhigh, then contract-reviewer).** *Files:* `engine/src/approvals.rs`,
`engine/src/eventaccept.rs`, their in-module tests. *Behaviour:* spec §13 row T4e's four pieces: the
`syllabus-exam` arm sharing `settle_event_accept` (a hard one-off `exam` through `create_confirmed`
and a "Study for <title>" task due at its start; no time writes only the task); `AMENDABLE_FIELDS`
gains `grade_weights`, `grade_scale` and `late_policy`, each validated as a single-line flow mapping
in `apply_amendment` (`late_policy`'s closed keys and ranges, §16.3); `withdraw_syllabus` with the
signature below; and `process_approvals`'s finishing arm for the three shapes. Tests first, in-module beside
`withdraw_asks`'s; `amendable_fields_is_exactly_the_nine` becomes the twelve in the same commit, its
message saying why. P0's fact 6 decides how a field the course note lacks is set. Only one card
shape reaches the three fields: the syllabus grading card T4d files. Under Q-B's ruling `sync` files
none for them, so r1 finding 4's sync-card shape is no longer built or tested.
**G5, the approval (I1).** An approved syllabus grading card applies like every non-mail amend
card, through `apply_amendment` under the executor's context, actor `agent:approvals` (`:1693`). No
new arm is added, and no human actor is used. A stale one gets today's outcome (P0 item 6): the
`stale amendment` warning, then back to `pending`, not archived.
**`withdraw_syllabus`'s signature, fixed here (r6 I-1).** T4d'a, T4d'c and T4d'b then never edit
`approvals.rs`. It behaves as `withdraw_asks` does, except that the caller supplies the reason and
the count is the counted one:
- *Inputs.* The vault; `reason`, a caller-built `impl Fn(&str, &Mapping) -> Option<R>`; `counted`,
  the stems in this run's `amendments_pending`; the context and the journal. `reason` receives the
  card's file stem and its frontmatter. The report's form, `Fn(&Mapping)`, gains the stem for G2's
  file-name tie-break (r6 M-2): two cards whose frontmatter matches byte for byte, in two files, must
  get different answers. `contract-engineer` may name the parameters.
- *The walk.* `approvals/` in `sorted_md` order. Only pending or snoozed cards of the three syllabus
  shapes (spec §4.6) are offered to `reason`, once each. On `Some`, the card is stamped `status:
  superseded` and archived through `delete`, journal first. A card it cannot move gives one
  `transition failed:` warning, and the walk goes on. It never raises into the run.
- *The result.* The existing `Withdrawal`, unchanged in shape (`approvals.rs:1944-1952`).
  `withdrawn` lists the stems. `pending` counts only the withdrawn cards `process_approvals` counted:
  pending on disk, and for an amend card, a stem in `counted`. So a snoozed card is not counted, and
  neither is an approved card the stale check put back (fact 3). `R` is never read beyond `Some` and
  never written; the caller keeps its own reasons for the run line.
- *`Withdrawal`'s doc comment (r8, the r7 review's M-4).* Today it names only `withdraw_stale` and
  says `pending` counts the withdrawn cards "pending on disk" (`approvals.rs:1944-1946`). T4e
  rewrites it, in the same commit, to name its three producers (`withdraw_stale`, `withdraw_asks`,
  `withdraw_syllabus`) and to state the rule they share: `pending` counts the withdrawn cards that
  `process_approvals` counted into this run's line. For the first two that is pending on disk. For
  `withdraw_syllabus` an amend card also needs its stem in `counted`. The struct's fields and every
  existing caller stay byte-identical; only comment lines change.
- *In-module cases.* `reason` sees only the three shapes. A pending amend card outside `counted` is
  withdrawn but not counted; a snoozed card is not counted; a pending task card is counted. `None`
  leaves a card untouched. A failed move gives one warning, and the next card is still withdrawn.
**G6, `approvals::amendment_is_stale` and the comparison (r4's I3 and I4; kept in r6 for two
readers).** After r6, `amendment_is_stale` is two things: G7's *answered* test (T4d'b), and
`apply_amendment`'s from-check. Its comparison is also G10's and G11's. Its r5 re-base role (G4) is
gone.
- *The function is the from-loop only.* `apply_amendment`'s from-loop (`:810-827` on the email
  branch) becomes one `pub fn` taking the target note's frontmatter and a card's `changes`, and
  returning the first stale field's reason or none. The `already_applied` gate (`:803-808`) stays in
  `apply_amendment`, which calls the new function after it. The outcome and the `stale amendment: …`
  text stay byte-identical, so *answered* and the stale check never disagree.
- *One predicate for G10 and G11.* A second `pub fn` on the same `comparable` answers one question:
  do two values compare equal as a given field compares them? The filer uses it to leave a held
  field off a card (G10). G11 uses it to compare a card's `to` with the extraction's value, or a
  task's `due` with a due card's `to`. The from-loop is written on it. `contract-engineer` names it.
- *The three fields compare key-sorted.* `comparable` today formats a mapping with `{:?}` through
  `plain` (`:120-124`, `:519-527`), so key order counts. A mapping sync carried comes back key-sorted
  (`serde_json::Map` without `preserve_order`; P0 item 6). For `grade_weights`, `grade_scale` and
  `late_policy`, `comparable` therefore compares `ledger::dumps_value(&yaml::to_json(value))`, whose
  keys sort, at every depth. Every other field compares as today.
- *In-module cases:* a `from` with keys out of alphabetical order against the same mapping sorted is
  not stale; a different value is stale; the `already_applied` retry still applies nothing and
  reports nothing; a `due` card's outcome is byte-identical to today's; the G10 predicate agrees with
  the from-loop on every one of these.
**`engine/src/sync.rs` (Q-B, same commit; contract list).** Add `pub const LAST_WRITE_WINS_FIELDS:
[&str; 3] = ["grade_weights", "grade_scale", "late_policy"]` beside `ACTOR`, with Quinn's sentence
quoted in its doc comment. In `sync::apply`'s step 5, the `cardable` closure becomes
`card_folder && crate::approvals::AMENDABLE_FIELDS.contains(&field) &&
!LAST_WRITE_WINS_FIELDS.contains(&field)`. The step-5 comment (`sync.rs:2023-2028`) is amended (M3):
`grade_weights`, `grade_scale` and `late_policy` are never carded, so the later write wins with the
warning. A syllabus grading card left stale by it is withdrawn as answered by `rank` on any desktop
(G7, T4d'b) and never filed again. Sync never touches it. The warning text, the uncarded branch and every other line stay byte-identical. Why the list is
separate, and why here: Q-B. One commit carries the widening and the exclusion, so no tree on the
branch ever makes the three fields cardable. No `journal::VIAS` value, `write.rs` line, card kind or
note folder changes. *Done:* `contract-reviewer` PASS (report in `docs/reports/`). Every existing
approvals and sync test is green and unedited except `amendable_fields_is_exactly_the_nine`;
`sync_keeps_every_local_card_kind_local` and `the_servers_note_path_rules_name_every_note_folder`
are among them. P2's Q-B cases are green, and red on a tree with the widening but without the
`sync.rs` lines (shown once, recorded).

**P2 (contract-engineer, then contract-reviewer).** *Files:* `engine/tests/syllabus_sync.rs` (new).
Sync tests are on the contract list, so every sync-facing pin of this lane lives here (r1 finding 7).
- **Always:** `sync::is_note_path` refuses `syllabi/<slug>.pdf`, `syllabi/<slug>.docx` and
  `state/syllabi/<slug>.json` (moved from T4t).
- **Q-B as ruled: last-write-wins, never a card.** These run on a course note, through `sync::apply`
  as `sync_contract.rs` drives it, for each of `grade_weights`, `grade_scale` and `late_policy`:
  1. **A later foreign write wins as today.** In parity with
     `a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so`: `(cards, applied,
     superseded) == (0, 1, 1)`. The note holds the foreign value as a single-line flow mapping (assert
     the exact line), and no file appears in `approvals/`. For `grade_weights`, the test asserts the
     exact warning built at `sync.rs:2046` (r4 M2; r5 M5: the string as `format!` builds it, with
     real backticks around the field name and an em dash, and no escapes):
     ``sync: courses/<slug>.md — both machines set `grade_weights`; the later write won and the other is in the journal``.
     The other two fields assert that the warning names the path and the field.
  2. **An earlier foreign write loses as today.** In parity with
     `a_conflict_this_device_won_writes_nothing_and_files_no_card`: `(applied, cards) == (0, 0)`,
     `superseded == 1`, and the note keeps this device's value.
  3. **The list itself.** `LAST_WRITE_WINS_FIELDS` is exactly the three, and each is in
     `approvals::AMENDABLE_FIELDS`, so the exclusion is never dead. A field in `AMENDABLE_FIELDS` and
     outside the list (`due` on a task note, say) still becomes a card, so the closure did not lose
     its other half.
  4. **Key order survives the round trip (I3, G6).** Desktop B writes `grade_weights` with its keys
     out of alphabetical order, and sync carries it to A.
     - A's note holds the key-sorted single-line mapping (assert the exact line).
     - A pending card on A whose `to` is that value in B's order no longer holds its `from`. It is
       withdrawn as answered and nothing is filed (G7, G10).
     - A card whose `from` is the note's value in another key order is not stale and is not
       withdrawn, and it approves cleanly.
- **The syllabus cards across desktops** (Quinn's simplification and the I-3 ruling; spec §4.6 place
  10; G2, G5, G7-G11). Two vaults, A and B, are joined as `sync_contract.rs`'s `transfer` and
  `deliver` join them (`sync_contract.rs:1903-1942`). One desktop's `build_push` becomes the other's
  pulled page, with devices relabelled in the page and `KNOWLU_DEVICE` never set. `rank` runs on each
  vault in turn. Unless a case says otherwise, A holds the course's extraction and a pending grading
  card *c* it filed, with `from` = the note's value, and B holds no extraction.
  - *Why it converges, with each card charged once (r7).*
    1. *Filing is bounded.* A grading key files only while it has no history (G9). An item's day
       files only while no card has held it under its key (the twin). History syncs, so after a sync,
       a key or day filed on one desktop is never filed on another. Before a sync, each desktop files
       each at most once.
    2. *Every other rule only withdraws.* G7, G11, G8, G2, *date moved* and *authoritative match*
       each move a pending or snoozed card to `archive/`, and the engine never moves one back. So each
       card changes state at most once, and by point 1 finitely many cards are ever filed. The
       process ends, no card comes and goes twice, and each card is charged once, on its own
       `proposed_at` day.
    3. *Where it ends.* G7, G11's note half and *authoritative match* read only synced data (cards,
       ids, file names, notes), so after a sync each way every desktop decides them alike. G11's
       syllabus half, G8 and G2's survivor run only where the course's extraction is.
       - Two desktops holding the same extraction decide alike on the same cards.
       - Two holding different ones each withdraw the card their own syllabus does not support (G11),
         so the course ends with no card, or with one card that both support (G2).
       - A desktop without the extraction ends with what the holders keep, through their tombstones.
    4. *Two desktops archiving one card* give no conflict and no warning. Sync reconciles only where
       the local file still exists (`sync.rs:1955-1959`), and a tombstone for a path already archived
       does nothing (`:2274`).
       - *One approving while another withdraws* (r6 M-3) leaves two archived copies with different
         statuses, `executed` and `superseded`. Each side skips the other's records for the same two
         reasons.
       - That is benign. The key is blocked on both (G9 reads any status), the note converges by
         last-write-wins, and each desktop's day count sees one copy.
       - (j) pins it. (f), which has no approval, asserts equal archives.
  - *Cases:*
    - **(a) answered where it was filed.** A later foreign write from B lands on A by case 1, once
      with the record's actor `student` and once `agent:approvals`. A's next `rank` archives *c* as
      `superseded`, and nothing is filed on this run or the next two. Both actors leave the same
      bytes in `approvals/` and `archive/`, since G7 never reads the actor.
    - **(b) approved:** the note holds the syllabus value, its journal record's actor is
      `agent:approvals`, written journal first, and no sync card is filed (G5; r4 I1).
    - **(c) converged:** the foreign value equals *c*'s `to`. *c* is withdrawn and nothing is filed
      (G7, G10).
    - **(d) answered on a desktop without the extraction (r5 I2).**
      - B edits the field. B's own next `rank` archives B's copy of *c*, since G7 needs no extraction.
        After a sync each way, A holds *c* in `archive/` and files nothing.
      - The approval race, on a fresh pair: A's moved value reaches B, and the student approves *c*
        on B before B's `rank`. B's `rank` warns `stale amendment: grade_weights is …, expected …`,
        puts *c* back to `pending`, and archives it as `superseded` in the same run. The pending line
        equals the pending cards left in `approvals/` (T4d'a).
    - **(e) approved earlier:** an approved grading card whose value a later foreign write
      overwrites is not filed again (G9; the field stays last-write-wins).
    - **(f) two extraction-holding desktops over three slots (r5 M4; scoped by r6 M-3).** A and B
      each hold an extraction for the course, and neither has a grading card yet. No card is
      approved in (f); (j) covers an approval. All slots fall after the vault's first day, so no
      day-1 hold applies. The sequence:
      1. Slot 1: `rank` runs on both before any sync, and each files one card.
      2. A sync each way, then slot 2 on both.
      3. A sync each way, then slot 3 on both.
      Four variants:
      - *Same hash, successive days.*
        - After slot 1, one card on each desktop.
        - After slot 2, exactly one pending grading card on both desktops, the one with the lower
          `id:`, and the other archived as `superseded`.
        - After the second sync and slot 3, the same pending card under the same id on both, and the
          same `archive/` on both.
        - Two grading cards ever filed, both in slot 1. `count_proposals_created` on slot 3's day
          counts no grading card.
      - *Same hash, one day (r6 M-3).* The three slots run on one day.
        - After slot 2, each desktop's `count_proposals_created` for the day counts both grading
          cards, its own and the pulled one. The withdrawn duplicate still counts from `archive/`
          (`approvals.rs:1144-1162`).
        - The day's 8 counts both too.
        - After slot 3, both counts are unchanged, so nothing more is charged.
      - *Different hashes, differing in a carried field.* The two extractions give that field two
        different valid values, and the course note holds neither (r8, the r7 review's M-3), so G10
        leaves the field on both cards. (Were the note to hold one desktop's value, only the other
        card would carry the field, and which card stands would turn on the random ids through G2.
        Point 3 allows either end, no card or one card both support, so that variant is not pinned.)
        - In slot 2, each desktop withdraws the other's card as *unsupported* (G11) and keeps its own.
        - After the second sync, both cards are archived on both desktops, and slot 3 files nothing,
          since both keys have history.
        - No grading card is pending, and two were ever filed.
      - *Hand edit.* Same hash, and the student edits the field by hand on B after slot 2. After the
        next sync and slot, the survivor is archived on both, and nothing is filed again.
    - **(g) no orphan (r5 I2).** *c* came from a desktop that never runs again. On B, which holds no
      extraction, *c* is withdrawn when the field moves (G7). In a second variant, B gains its own
      extraction with a new hash that differs in one field. B withdraws *c*, as *unsupported* where
      the field is one *c* carries and as *set changed* where it is not. B files its own card in the
      same run (G11, G8). So no card can hold a course for good.
    - **(h) unsupported grading ends with no card (Quinn's I-3).** Both desktops hold h2's pending
      card *c*, and A's extraction is now h3, whose values the note already holds.
      - If B holds h3 too, each `rank` withdraws *c* as *unsupported*. After a sync each way, neither
        desktop has a grading card, and three slots file nothing.
      - If B still holds h2, A withdraws *c*. B keeps it until A's tombstone arrives, then holds it
        in `archive/`, and files nothing, since h2 has history. No card is left on either.
      - If B holds no extraction, A's withdrawal reaches B by sync. No card is left on either.
    - **(i) unsupported items end with no card (I-3).**
      - *Disagreeing days.* A's extraction gives an item d1, B's gives d2, and each files its card
        before any sync. In slot 2, each withdraws the other's as *date moved* and keeps its own.
        After the next sync both are archived on both, and slot 3 files nothing, since both days were
        filed under the key.
      - *A due already there.* A due amend card is synced to B, which holds no extraction. The
        student sets the task's `due` to the card's `to` on A through `write` (a console write, so
        the journal carries it with no `rank` needed). The order is fixed (r8, the r7
        review's M-3): the edit on A, then a sync each way, so the new `due` reaches B; then B's
        `rank`, then A's. A's `rank` runs only after that sync, or A's tombstone would archive B's
        copy and the case would not show B's own rule. B's `rank`
        withdraws its copy as *unsupported*, and A's withdraws A's the same way. After a further sync
        each way, the two archives agree and neither desktop files a card.
    - **(j) approve on one desktop, answer on the other (r6 M-3).** Both desktops hold *c*. Before
      any sync, the student approves *c* on A, and A's `rank` applies it. Later, the student edits the
      field by hand on B, and B's `rank` withdraws *c* as *answered*. After a sync each way:
      - both notes hold B's value, the later write, with case 1's warning on A;
      - A's `archive/` holds *c* as `executed` and B's holds it as `superseded`;
      - no grading card is pending on either, and three slots file nothing.
Sync's existing tests and both tripwires are untouched and green; P2 adds no case to them. *Done:*
green on the tree after T4e, T4d'a, T4d'c and T4d'b.
- **Red runs (r4 M1), each shown once and recorded:**
  - cases 1 and 3 are red on a tree with T4e's widening and without its `sync.rs` lines;
  - case 4's two card assertions are red without T4e's G6 and T4d'b's G7, since its first assertion
    already holds today;
  - cases (a), (c), (d), (f), (g), (h) and (j) are red without T4d'b, on the tree with T4e, T4d'a
    and T4d'c;
  - case (i) is red without T4d'c, on the tree with T4e and T4d'a: there, each desktop refiles its
    own day once the other's withdrawal syncs, because a key whose history is only superseded files
    again;
  - (b) and (e) are regression pins. They are red only before T4e's widening (the field is refused
    as not amendable), shown once on the tree after T4d, and not claimed against T4d'b;
  - case 2 passes on either tree (a conflict this device won files no card even when cardable), so
    it is not claimed red.

**T4d' is three tasks (r5 I5; resized in r7).** The hook (T4d'a, `cli.rs`, about 50 lines since r10a), the
item rules (T4d'c, about 45) and the grading rules (T4d'b, about 85) each fit under the 120-line stop
rule. T4d'c and T4d'b share `syllabus_cards.rs`, so they run one after the other. None of the three
edits `approvals.rs` (r6 I-1).

**T4d'a (implementer; about 50 lines, r10a).** *Files:* `engine/src/cli.rs`. The `rank` hook.
- **Where (pinned in r8, the r7 review's M-1).** In `run_with`, immediately after
  `let mut approvals = process_approvals(vault, today, now, &ctx, &mut journal);` (`cli.rs:430` on
  the email branch) and before the events pass's comment block and
  `let remaining_budget = … count_proposals_created(vault, today) …` (`:431`-`:440`). After
  `process_approvals`, so an approved card the stale check puts back is seen as `pending`. Outside
  #19's series lock, not inside `commitment_passes`. The hook runs whether or not `state/syllabi/`
  exists, since G7 and G11's note half need no extraction. *Why this line (r9, the r8 review's
  I-1):* the events pass (`:439-440`, `:497`) and `commitment_passes` (`:873`, `:881`) size their
  cards from `count_proposals_created(vault, today)`, and only here are this run's syllabus cards
  inside that count. The filer sizes itself the same way (T4d's "The 15"). Together, no syllabus
  card takes a day over the 15 in any run. `defer_over_budget` (`:423`) still runs first, unchanged,
  and a later run trims overflow that other cards make (T4dt (B), (ii)'s held variant).
  *Corrected in r9:* r8 said "one run charges no more than the 15 through sizing that already
  exists" and "no existing card is snoozed for them". Both were false. The filer was held to the
  day's 8 only, so r8's own case (ii) charged 18 in one run. And when a card charged after the hook
  takes a day over, `defer_over_budget` can snooze a mail card that would have kept its place had
  the syllabus cards not taken room earlier (T4dt (B), (ii)'s held variant).
  - *What it costs (on Quinn's list, §7 item 2; the r8 review's M-2).* On a full day the syllabus
    cards take up to 8 of the 15 ahead of `event-accept` cards, `unsure` checks and commitment
    cards, which are sized to what is left. They take nothing from cards charged before the hook.
    Concretely: at the start of a term, about 30 items fall inside the horizon at once. They take
    8 a day for about four days (30 / 8). In those days the events pass gets at most 7 of the 15,
    less whatever mail and held cards took, so an obligation event inside those days can get no card
    before it happens: the events pass files an obligation only while its start is today or later
    (`eventemit.rs:436-440`). Under T4d's "The 15" the same cost holds, capped at 8 a day, and the
    syllabus cards themselves never take the day past the 15.
  - *The other side of "The 15".* Syllabus cards wait behind every card charged before the hook. On
    a run of days whose 15 is used before the hook, no syllabus card files. An item is planned
    again on each run while it is in the horizon and ahead, so it reaches its day with no card only
    if every run from the day it entered the horizon to its day is full. For an item in the
    extraction from the start, that is every run in its 14 or 21 days. An item that enters late has
    fewer: a syllabus added mid-term with a quiz two days out has only two days of runs, and on the
    vault's first day the runs start on day 2 (r10, the r9 review's M-3). Also on Quinn's list.
  - *The alternative, not taken.* With "The 15" in place, a hook after the events block would also
    keep every run inside the 15 (under r8's sizing it let one run charge 15 plus 8). So the choice
    between the two lines is now only one of priority: after the events block, event cards come
    first, and the start-of-term burst waits behind them instead of crowding them. The line here is
    kept, as decided, and T4dt's sum-of-15 case (i) pins it. §7 item 2 reports the priority to
    Quinn as a decision taken, which Quinn may reverse by moving the line.
  - P0 item 6 re-reads the two lines on `main`. If anything now sits between them, the hook goes
    directly before `remaining_budget`, and P0 records why.
- **The order** (spec §4.6, "`rank`'s order", extended for G8 and M-1):
  1. The run's inputs are read once, by `syllabus_cards::read_inputs`: the cards in `approvals/` and
     `archive/`, the course and task notes, `existing_source_uids`, and each course's extraction
     where there is one.
  2. `syllabus_cards::plan` decides the withdrawals and the filing plan together, purely (T4d,
     T4d'c, T4d'b).
  3. `withdraw_syllabus` runs (T4e). Its reason closure looks the card's stem up in that decision,
     and `counted` is the stems of `approvals.amendments_pending`.
  4. The filer writes the planned cards (T4d).
  5. *(r10a, the r10 review's M-2; r10b, the r10a review's I-1)* For each course with an
     extraction of a known `version`, the hook calls `syllabus::write_outcomes` with `plan`'s
     `listing` and the SHA and `added` `read_inputs` saw; `Skipped` (a `syllabus-add` replaced the
     file during the run) writes nothing, and the next `rank` lists the new copy. `judge` (T5)
     carries the outcomes through unless the SHA changed. `state/` is generated, so this is not a
     note write and takes no journal record. About 10 lines, so T4d'a is about 50, under the
     120-line stop rule.
- **The context (r4 M6).** `withdraw_syllabus` runs under the filer's own `WriteContext`: actor
  `agent:knowlu.syllabus`, with the run's `via` and `run_id`. It never takes the human actor or
  `agent:approvals`. Every stamp and archive is journalled under it, journal first.
- **The pending count and Must do (fact 3; r6 I-1, I-2).**
  - `approvals.pending` gains the cards filed and loses `withdrawal.pending`, the counted ones, as
    the commitment passes do (`cli.rs:562`).
  - Each withdrawn stem is removed from `approvals.amendments_pending`, which Must do reads
    (`render.rs:379-382`).
  - When `withdrawal.pending` is above zero, the queue is re-aged as `cli.rs:563-565` does.
- **The run line.** It names each withdrawal with its reason's word (*answered*, *unsupported*,
  *set changed*, *duplicate*, *date moved*, *authoritative match*) and each filing. An *unsupported*
  withdrawal is quiet in the ruling's sense: no warning and no notice on the course. When nothing is
  withdrawn or filed, the hook writes nothing and adds no line. So `rank` over `vault-full`,
  `golden-today-*.md` and `run-records-reference.json` stay byte-identical.
*Done (r6 I-4; r8, the r7 review's I-1):* T4dt's group (B) is green: the filer's `rank`-level cases
(each filed card read back for its synced fields, Q46's `where` included; journal first under
`agent:knowlu.syllabus`; distinct file names across two vaults; the Q45 notice; day 2's sum of 15,
(i), (ii) both ways and (ii)'s held variant, r9) and the byte-identical case. The fixtures diff is empty. Nothing more is claimed.
Group (C) needs T4d'c's or T4d'b's rules, so cases 4, 9 and 10 and the end-to-end withdrawal cases
are in their Done.

**T4d'c (implementer; about 45 lines).** *Files:* `engine/src/syllabus_cards.rs`. The item rules: G11's
item half and the twin (Q-B's rules; Quinn's I-3 ruling; spec §4.6 place 10).
- **G11's item half.** *Date moved* (T4d's) stays unconditional: it needs no room in the day's 8.
  It fires only for an item still in the extraction with a resolved day other than the card's. A
  dropped item, or one that now needs a date, is absent and leaves its card (r8, the r7 review's
  I-2; Q-B G11's one reading; spec :766-767). Two *unsupported* reasons are added, read from synced
  notes on any desktop:
  - a due amend card whose target task's `due` already compares equal to the card's `to` (T4e's
    predicate);
  - a task or exam card whose key is already the `source_uid` of a note that is not an approval card
    (in `tasks/`, `commitments/` or `archive/`).
- **The twin's filing rule.** The filer files an item's day only when no card in `approvals/` or
  `archive/` has held the key with that day: the payload's `due` day for a task, its date for an exam.
  For a due amend card, no card may hold `<item key>:due:<day>`, and the task's `due` must not already
  compare equal to the card's `to`. Otherwise nothing is filed, which is the ruling's "nothing is
  re-filed".
- **The survivor order** (T4d'b reuses it). A card with a readable `id:` comes first, then by `id:`,
  then by file name. Two pending or snoozed cards for one item key and one day keep the first, on a
  desktop holding the extraction, and the rest are *duplicate*.
- Every condition is pure over the inputs T4d'a reads once per run.
*Done:* T4dt's case 10 green, its *absent, not moved* variants included; T4dt's existing *date moved*
and due-amend cases still green.

**T4d'b (implementer, then reviewer; about 85 lines).** *Files:* `engine/src/syllabus_cards.rs`. The
grading card's rules (Q-B; Quinn's simplification and I-3 ruling; spec §4.6 place 10).
- **Inputs**, all read once per run by T4d'a:
  - the course note's frontmatter;
  - the keys with history (`approvals::existing_source_uids`);
  - the course's pending and snoozed grading cards (ids, file names, keys, `changes`);
  - the course's usable extraction and its grading set, where this desktop has one.
  It never reads the journal, an actor or a device.
- **The order, per course.** Every desktop holding the same inputs computes the same result.
  1. G7 and G11 withdraw whatever the filer does.
  2. The current key is a *candidate* when it has no history, at least one of its fields differs
     from the note, and G10 leaves something to carry. Every pending or snoozed card left after
     step 1 is then under another key, since a card under the current key would give it history.
  3. If the filer's plan (T4d) files a candidate within the day's 8, G8 withdraws every card left
     under another key as *set changed*, in the same run. Otherwise G8 withdraws nothing, and the key
     waits for a run with room. A card left after step 1 is *replaceable* only if the set gives
     every field it carries, validly and equal to its `to`. A card one of whose carried fields the
     set leaves absent (that part failed §4.7's check) is never replaced, and under G2's filing half
     it still blocks the candidate. So a partly failed read, whose hash covers fewer fields, files
     nothing and withdraws nothing (r8, the r7 review's I-2; T4dt case 6).
  4. On a desktop holding the extraction, G2 keeps the first card left by the survivor order
     (T4d'c), and the rest are *duplicate*. A desktop without the extraction keeps them all.
- **G7, answered.** A card is answered when `approvals::amendment_is_stale(note, card's changes)`
  finds a field (T4e; never a comparison of its own). If the course note is missing or unreadable,
  the card is never called answered.
- **G11, unsupported (one reading, r8, the r7 review's I-2).** This needs the course's usable
  extraction. A card is unsupported when, for some field it carries, the extraction's grading set
  gives a valid value that does not compare equal to the card's `to` (T4e's predicate). A field the
  set gives no valid value for (the part failed §4.7's check, or was not read) is absent, and an
  absent field never makes a card unsupported (Q-B G11, "One reading for both halves").
- **G8, set changed.** This needs a grading set with at least one valid part, which gives the
  current key. Its rule is steps 2 and 3 above, with step 3's *replaceable* condition.
- **G10, at filing.** Each carried field the note already holds is left off the card. If none is
  left, nothing is filed, and the key stays unfiled. The one-time card this leaves is accepted (Quinn,
  2026-10-20).
*Stop rule:* past about 120 source lines, the agent stops and the controller splits G8 out.
*Done (r6 I-4):* T4dt fully green, cases 4 and 9 included; P2's syllabus-card cases green; fixtures
diff empty; `reviewer` PASS. The review report goes at
`docs/reports/2026-10-2x-l5-t4db-review.md`. It checks each rule against Q-B's text, the two
2026-10-20 rulings and P2's convergence argument.

**T4ft (test-writer).** *Files:* `engine/tests/syllabus_ingest.rs` (new). Spec §13 row T4ft, every
case, and the frozen references byte-identical with no `syllabus:` task. *Done:* red for "no adoption".

**T4f (integrator, Opus high).** *Files:* `engine/src/ingest.rs`. Spec §4.6 "When the feed brings the
item later (Q23)": only the new-item branch, only when an active `syllabus:` task matches by the title
rule in the same course; re-key through `write`, journal first, as an agent; a hand-set due gets an
amend card. *Done:* T4ft green; every ingest and oracle test green and unedited; fixtures diff empty.

**T4g (implementer).** *Files:* `engine/src/judge.rs` (`Heuristics::load` only), the grades read model
P0 located. Spec §4.7 "What reads them". Tests first in the same task, in-module; the surface
references untouched. *After:* L6's `judge.rs` deletions are on `main`. *Done:* green; no other
`judge.rs` item changed.

### Engine: the judge pass, the Simple Syllabus source, policies (PR-1)

**T5t (test-writer).** *Files:* `engine/tests/judge_syllabus.rs` (new). Spec §13 row T5t, every case,
against a `127.0.0.1:0` server, with the slot's case run as `judge --syllabus` (Q-K): the pass after
enrichment (at most 8), `--only syllabus` sends nothing else, the request's `item.id` is the course
note's opaque id with the date and the unsettled parts only, per-part status, `pending` to `read` or
`failed:<code>`, and no network, no entitlement, the cap, a 404 or an unknown-kind answer each leave
`pending` with its own named line and exit 0; the pass writes no note. **Q-K's fail-closed cases (r1
finding 3):** a plain `judge` over a vault holding a `pending` `state/syllabi/` file sends **no**
request to `/judge-syllabus` (the server records none), leaves the file's bytes unchanged, and prints
output byte-identical to the same run over the same vault without the file. **The case P3 relies on
(r2 finding 5):** `judge --syllabus` over a vault with **no** `state/syllabi/` folder behaves exactly
like a plain `judge` over the same vault: the server records the same requests (none to
`/judge-syllabus`), the vault's bytes after the run are the same, stdout is byte-identical, and both
exit 0. The slot passes the flag without looking at the vault (P3), so this is the slot's normal case
for a student with no syllabus. **The outcomes case (r10b, the r10a review's M-4):** a `pending`
state file holding non-empty `item_outcomes` whose recorded SHA is not the copy's in `syllabi/`
comes out of the pass with `item_outcomes` `[]`; with the SHA unchanged they are kept byte for byte.
**The mismatch case (r10c, the r10b review's M-1):** a `read` state file whose recorded SHA is not
its copy's: `judge --syllabus` sends that course to `/judge-syllabus` (the server records the
request) and writes the copy's SHA; with the SHAs equal, a `read` file sends nothing. *Done:* red for "no syllabus pass".

**T5 (implementer).** *Files:* `engine/src/enrich.rs`, `engine/src/cloudmodel.rs`, `engine/src/main.rs`.
Spec §4.3 "Where it runs: inside `judge`, as a scoped pass (Q5)", with Q-K's amendment: two flags
join `judge`'s parse, `--syllabus` (the full run plus the pass) and `--only syllabus` (the pass
alone). Without either flag, the pass is not constructed and `state/syllabi/` is not read. The pass
sits after enrichment and before events; Q-F's cap line; `/judge-syllabus` beside the other paths.
It writes the state file through T2c's `write_state` and carries the `item_outcomes` it read through
unchanged (r10a; `rank`'s hook owns them, T4d'a), except that it writes them `[]` when the SHA it
writes differs from the SHA in the file it read, so no outcome outlives its extraction (r10b, the
r10a review's M-4; T5t's outcomes case). In a `--syllabus` or `--only syllabus` run, a state file
whose recorded SHA differs from the SHA of its copy in `syllabi/` is treated as `pending`, whatever
its `status`, so a state left behind by a `syllabus-add` that renamed inside `rank`'s gap or was
killed between the copy and the state is re-read (r10c, the r10b review's M-1; T5t's mismatch
case).
*After:* L6's T2 (`Options`, `run`) and L3's T3b, so this rebases once. *Done:* T5t green.

**T15t (test-writer).** *Files:* `engine/tests/syllabus_simple.rs` (new), invented documents in S2's
shape under `engine/tests/syllabus_data/`. Spec §13 row T15t, every case (rendering drops attributes,
`material` and non-office-hours `instructor` lines; the course match none, one, two; the term check;
Q37's week anchor; a linked PDF is an outcome; an unknown shape writes nothing; the CRN never in an
excerpt; held renderings and the held form; the replacement rule for every pair of sources). The
registrar-key match case builds a confirmed `registrar:` note in #19's key format. *Done:* red.

**T15 (implementer).** *Files:* `engine/src/syllabus.rs`, `engine/src/cli.rs`, `engine/src/main.rs`.
Spec §15.3-§15.5 and §4.1's third and fourth forms and "Held renderings". *Done:* T15t green.

**T16t (test-writer).** *Files:* `engine/tests/syllabus_policies.rs` (new). Spec §13 row T16t, every
case that needs no `surface` call (the screen's policies part, `late_policy` into the grading card,
no quote in any card's bytes); the policies bound is 1,500 and the total stays T1's 13,500. The
row's read-model cases (policies and quotes in `syllabi`, the past-due row's late note) go to T17t.
*Done:* red.

**T16 (implementer, then reviewer).** *Files:* `engine/src/syllabus.rs`, `engine/src/syllabus_cards.rs`.
Spec §16.2-§16.5, engine side only; no `surface.rs` line. *Done:* T16t green; no card's bytes hold a
policy quote; fixtures diff empty.

### Engine: the read model (PR-2, after L7)

PR-1 never touches `engine/src/surface.rs` (r1 finding 1), so these tasks run on the lane branch
after P8 and L7's merge, and ship in PR-2.

**T17t (test-writer).** *Files:* `engine/tests/surface_syllabi.rs` (new). Spec §13 row T17t, every
case, plus the read-model cases of row T16t (above). *Done:* red.

**T17 (implementer, then reviewer).** *Files:* `engine/src/surface.rs`, a state-file reader in
`engine/src/syllabus.rs`. Spec §4.8 "The `syllabi` read model". An item's `state` is the file's
code as written, with its `date`, so a `past` item dated today reaches T11b, which words it as due
today (r10c, the r10b review's M-3). *After:* L7 merged. *Done:* T17t's
T17 cases green; `surface-today-{s1,s1-migrated,full}.json` unchanged (`engine/tests/surface_oracle.rs`
green).

**T16s (implementer).** *Files:* `engine/src/surface.rs`. Spec §16.4-§16.5's read-model part:
policies and quotes in `syllabi`, the late note on a past-due row. *After:* T17. *Done:* T17t fully
green; surface references compared, never regenerated.

### Cloud (PR-1)

**T6a (cloud-engineer).** *Files:* `cloud/supabase/functions/_shared/judge_validate.ts` (`Kind`),
`judge_caps.ts` (`DAILY_CAP.syllabus` 20), `judge_caps_test.ts` (the pin) and `judge_pipeline_test.ts`
(its `DAILY_CAP` asserts gain the row), a new migration `<stamp>_judge_syllabus.sql`,
`migrations_test.ts`. *After:* P0 and L3's merge (r3 minor 2): L3 edits `judge_pipeline_test.ts` and
the shared `_shared` files, so T6a starts on a tree that has L3. *Behaviour:* every
`kind in ('task', 'event', 'email')` check found by P0's grep
gains `'syllabus'` (the five in `judgment_service.sql` **and** the two in `eval.sql`, fact 7);
`judge_allowances.kind` is **not** widened (L3's Q-F); one `models` row for `syllabus` per Q-I; the
rule-promotion functions leave `syllabus` out (no rule is promoted from it). *Stamp:* §2's migration
bullet, M3 at this task, recorded in the lane ledger before the file exists, provisional until L8 has
recorded its own (L3's is on `main` by then); tests find the file by suffix only (M4). *Done:* the `ci.yml` deno lines
green; the stamp row in the ledger. **No staging push in this task.** The file first reaches staging
at P8 (Q-I). If T7b picks a different pin, the controller has `cloud-engineer` edit this file's
`models` row in place before P8 (the file is not frozen until its first push, M6).

**T6b (cloud-engineer).** *Files:* `cloud/supabase/functions/_shared/judge_prompts.ts` and its test,
`judge_validate.ts` and its test, `_shared/judge_pipeline.ts` and `_shared/judge_pipeline_test.ts`
(r2 finding 2), `cloud/supabase/functions/judge-syllabus/` (new, the route's handler and test,
beside `judge-event`'s shape). *After:* T6a, and L3 merged; re-read `fieldsOf` on `main` first (L3
and L7 edit `judge_pipeline.ts`; §4.2). *Behaviour:* spec §4.3's schema with §16.2's
`policies` block; the validator (closed kind lists, per-part confidence, the quote-is-a-substring
check after whitespace folding, quotes at most 300 characters, the late rule dropped with a dropped
quote); `MAX_SYLLABUS_CHARS` 13,500 and the part bounds pinned like `MAX_BODY_CHARS`; the request
carries `item.id`, the date and the parts only. An unknown kind or a missing route answers as the
engine expects (T5t's 404 case). **The route keeps no syllabus text (r1 finding 5, fact 11; the
privacy rule, §2).** The reply goes back to the device and nowhere else. The `judgments` row for kind
`syllabus` carries ids, kind, model, versions, hash, tokens, `ms` and an outcome, and its `fields` holds
only per-part status codes and counts: no excerpt line, no extracted meeting, item, office-hours
line, quote or late rule. Nothing the route logs (`console.*`, the error path, a validator rejection)
carries request or reply text.
**Where that is enforced: a `syllabus` arm in `fieldsOf` (fact 11).** Every judgment's row is built
by `fieldsOf` in `judge_pipeline.ts`, which today stringifies every verdict field but four and then
merges `featureMap`. For kind `syllabus`, the arm returns only a closed list of keys (per-part status
codes and per-part counts, the names fixed in this task) and nothing derived from the excerpt or the
verdict's text. It never stringifies a verdict field outside that list, and it does not merge
`featureMap`. Every syllabus-derived value stays out of `judgments.fields`, on every tier and outcome
path that calls `fieldsOf`. The arm adds no line to `judge_rules.ts`; if it cannot work without one,
T6b stops and reports (§4.2). The `task`, `event` and `email` paths are unchanged, and their
existing `fieldsOf` and `featureMap` tests stay green and unedited.
**The row's `confidence` (r3 minor 4).** `judgments.confidence` is `real not null` with a 0..1
check, and `judge_pipeline.ts` fills it at the tier-2 and tier-3 writes from
`Number(verdict.confidence ?? 0)`. A `syllabus` verdict carries per-part confidences, so that
expression would give `NaN` and the audit row would be lost. T6b sets a `syllabus` row's
`confidence` explicitly, at both writes, through one small function beside `fieldsOf`: the
**minimum per-part confidence, clamped to 0..1, with a non-finite value read as 0, or 0 when the
verdict has no parts**. The minimum fits the per-part floor: the row is only as sure as its least
sure part. A mean would hide one weak part behind strong ones; the first part's value is arbitrary.
Other kinds keep today's expression unchanged. The request item stays closed to `id`, `date` and
`parts`, which is what keeps tier 2's rule lookup empty for this kind (`features` returns no pair).
*Tests.*
1. **In `judge_pipeline_test.ts`, modelled on "a body token reaches no judgment row, and neither does
   a title beyond its prefix"** (`a_body_token_reaches_no_judgment_row`). Plant one marker in the
   request's parts and others in a scripted `syllabus` verdict: in a meeting's room, an item's title,
   an office-hours line, a quote and a late-rule value. Run `judge()` with a capturing log. Assert
   that no marker appears anywhere in the written row's JSON. Call `fieldsOf` directly on that
   verdict too, and assert its keys are within the closed list and its values are status codes or
   digit strings. Two more assertions (r3 minor 4): the written row's `confidence` is a finite
   number in 0..1 that equals the scripted verdict's lowest part confidence (and is 0 for a verdict
   with no parts); and `features("syllabus", item)`, imported from `judge_rules.ts` and never edited,
   is `[]` for the request item's shape (`id`, `date`, `parts`).
2. **In the route's test file,** drive the handler with a synthetic excerpt holding a unique marker
   string and a scripted model reply holding a second one. Capture the log writer's row and every
   console call on the success path, on a validator rejection and on a model error. Assert that
   neither marker, and no value from the reply, appears in any of them.
*Done:* deno green; both tests green; no real syllabus text anywhere in the diff.

**T7a (cloud-engineer).** *Files:* `cloud/eval/**` (synthetic seeds in `cloud/eval/seed/`, the pass
bar per part: meetings, items with dates and kinds, weights, scale, office hours, the late rule and the
three quotes; seeds in Simple Syllabus's component shape as well as files). **The privacy rule (§2)
applies in full: synthetic seeds only in this task; no real syllabus is opened, copied, quoted or
described.** Every invented course, room and instructor is fictional.
**The off-repo real-seed reader is T7a's code (r1 finding 5),** so it is written, tested and reviewed
in PR-1 like any other code, and T7b runs it without writing code. It is a separate mode of the
harness, reached only by a flag (say `--local-real`), and it:
- reads seeds only from the folder one environment variable names (the name fixed in this task), and
  refuses with a named line, reading nothing, when the variable is unset, the folder sits inside a git
  working tree (a `.git` in it or above it), or the path lies under OneDrive or `%LOCALAPPDATA%\knowlu`;
- reads the consent manifest in that folder (§2; the file name and field names fixed in this task,
  never a value), refuses with a named line, reading nothing, when the manifest is missing or
  unreadable, and reads **only** the seeds the manifest marks both "owner consented" and "not
  public". Any other file in the folder is skipped and counted ("N files skipped: no consent
  record"), never opened;
- runs `judge()` in-process as `run_eval.ts` does (fact 12), with the candidate's model row given on
  the command line, never read from staging, and with stubbed caps, rules and log, so no
  `judgments`, `eval_cases` or `eval_runs` row is written anywhere;
- never uses `serviceDb()`, never writes a file, and prints only per-part counts and pass rates, the
  clipped-schedule count, the skipped count and the summed tokens. A provider error is printed as
  its code or class and a count, never its message or body, because an upstream's error text can
  echo the request.
*Tests* (synthetic only, temp folders, a synthetic manifest): the refusals above, the manifest's
included; a file missing either mark is skipped, counted and never opened. **The marker test runs
on three paths:** a success, a validator rejection, and a **provider error**, where the fake
provider's error body and message both echo the seed's marker. On each path, the seed's marker never
appears in stdout, stderr or a thrown message. No database client is constructed in this mode
(r2 finding 7). *Done:* the harness runs the
synthetic seeds against a local fake and reports per-part results; the reader's tests green; CI's
eval gate is not widened to a spend it did not have (staging's `eval_cases` stays empty of syllabus
rows unless synthetic).

**T7b (main session; no subagent).** *Files:* none in the repo. If the winner differs from T6a's pin,
the in-place edit of T6a's `models` row is a separate `cloud-engineer` dispatch before P8 (Q-I). The
controller runs T7a's `--local-real` mode on its own machine, with the provider key put into the
environment from Credential Manager for that command only. No key goes in a file, and no subagent
holds the key or reads the folder. Candidates are only models at hosts the page already names (spec
Q9). **The privacy rule (§2) applies in full: real seeds stay in Q-E's folder and are read only
through that one variable, and only those Quinn has marked in the folder's consent manifest (owner
consented, not public). Quinn writes the manifest; the controller never reads its values into the
transcript, and records only the count of seeds marked. OpenRouter and the one pinned upstream see
each seed sent; nobody else does. They never reach staging (no deployed route, no `eval_cases`, no
`eval_runs`, no function log), are never copied into the repo or a worktree, and are never printed,
quoted or summarised in the transcript, a log, a ledger row, a report or a commit. Results are
per-part counts and pass rates only.** Check that 7,000 characters holds a real term schedule (a count
of clipped schedules, nothing more). Choose the pin by the pass bar (Q9). Spend is recorded for Q-J.
*Done:* a ledger row with counts (seeds marked, read and skipped), spend and the chosen pin; Q-E's
folder unchanged in location; `git status` in every worktree shows no seed file; staging's migration
list unchanged by T7b.

### PR-1's review and merge

**T14a (reviewer; contract-reviewer over `approvals.rs`, `sync.rs` and P2).** `git diff main...l5-syllabus`
against this plan and the spec. Checks beyond the usual: §1's invariants one by one; rule 2's fixture
diff empty and the surface oracle green; nothing on the contract list but `approvals.rs`, `sync.rs`
(the list, the `cardable` closure and one comment line, Q-B) and P2's new test; no existing sync test
or tripwire edited; the three grading fields never filed as a sync card (Q-B); no app surface and no `engine/src/surface.rs` line in the diff (Q42, Q-G); no real syllabus
text, course, CRN or room anywhere; a plain `judge` cannot read `state/syllabi/` (Q-K); T6b's
`fieldsOf` arm and both its marker tests present, and the other kinds' `fieldsOf` tests unedited;
T7a's reader refusals, its consent-manifest check and its three-path marker test present; each test task's red run is in the ledger; the
ignored count equals P0's baseline (T1's interim PDF `#[ignore]` is gone). Report
`docs/reports/2026-10-2x-l5-pr1-review.md`.

**P8 (main session). L5's only staging push, then the merge, back to back (Q-I, M5, M7).** The
controller reads M1-M8 whole on `main` by their heading first (fact 10); where this list and M7
differ, M7 wins and the controller records the difference. In order, each step a ledger row; a step
that fails stops the sequence:
1. Final gate (engine and workspace build and test, eol check); the PR ("L5 part 1: the syllabus
   engine and judge kind") open, CI green, T14a PASS, and Quinn's word on the spend (Q-J) recorded.
   Nothing below runs before all four hold.
2. **Every lane ahead cleared** (r2 finding 3). That is email (cleared before the cut), L3, L8, and
   any lane recorded in the session ledger as having overtaken L5 by a slip (M8; L1, if L5 was ever
   recorded as slipped). Each one's migration is on staging **and** on `main` (session-ledger rows),
   or, having neither merged nor pushed, it is recorded as slipped. A lane pushed from its branch and
   not yet merged (L8's S7a until its S6 merge) is not cleared, and P8 waits.
3. M5's re-check on the lane branch rebased on `main`: a needed rename is `git mv` plus the pins
   recounted, with a ledger row, then step 1's local gate again.
4. **CI green on the rebased head.** The rebased branch is pushed to the PR, and CI runs green on
   that exact head; its sha goes in the ledger. Nothing reaches staging from a head CI has not passed.
5. `supabase db push --dry-run` from that head lists L5's version alone. A refusal, or any other
   version in the list, stops P8 by M7's dry-run rule; `migration repair` is never run to get past it.
6. The push from the lane branch (the staging proof); then the kind checks are read back on staging
   and accept `'syllabus'`. **If the SQL fails, the controller withdraws L5 on its own** by M7's
   withdrawal steps (five at r8, followed as M7 lists them), tells Quinn, and the lane returns to T6a.
   Any other reason to withdraw asks Quinn.
7. **The pre-merge dry run** (M7: "a lane does not merge its migration until a dry run on its lane
   branch, rebased on `main`, lists its own version alone (or nothing, once it has pushed from that
   branch)"). On the same head it lists **nothing**. If `main` moved after step 4, the controller
   rebases again (the file is frozen now, so it is never renamed), waits for CI green on the new head,
   runs this dry run on it, and tells Quinn of the delay.
8. The merge at once (merge commit), with nothing but step 7 between steps 6 and 8. If the merge
   cannot happen right away (CI turns red, a conflict), the controller tells Quinn and asks before any
   withdrawal. L5 is pushed and not merged, and it holds L1 until then.
9. `/judge-syllabus` deployed to staging from `main`; `supabase db push --dry-run` on `main` lists
   nothing; tell L1's controller that C1's wait on L5 is over (L5 cleared).

### App (PR-2)

**T8a (implementer).** *Files:* `app/src/syllabus.rs` (new), `app/src/lib.rs`, `app/tests/syllabus.rs`
(new); the console handler lines in `app/src/main.rs` after L6 merges. *Behaviour:* spec §13 row T8a
and §4.3 "Running it next to the scheduler (Q17)": `syllabus_add` (file and paste; paste written to the
profile's staging folder and deleted), `syllabus_status`, `syllabus_settle_held`, and the scoped chain
(`judge --only syllabus`, then `commitments`, then `rank`) under `Scheduler.running`, or "Reading with
this run" when a slot holds it **and the gate is `Accepted`** (r3 minor 1: a slot under any other
gate runs `judge` without `--syllabus`, P3, so the line would be untrue). When a slot holds the flag
and the gate is not `Accepted`, the status reads the waiting outcome below, never "Reading with this
run". **The consent check (r1 finding 3; Q-A, Q-K):** before the chain's
`judge --only syllabus`, T8a reads L4's gate for the profile (refreshing it once when it is not
`Accepted`, as the slot does). It runs the chain only on `Accepted`. Under `Older`, `syllabus_add`
refuses with a named error and keeps nothing, since L4's panel disables the controls anyway (T11). Under
`Unknown`, the copy is kept (`syllabus-add` is local and sends nothing), the chain stops before
`judge`, and the status reads a named "waiting for the privacy check" outcome (Quinn words it). The
next slot under `Accepted` reads it (P3). Every vault effect goes through the engine; `commands.rs`
gains nothing. *Tests first*, against a fake engine binary as the scheduler tests do, with one case
per gate value, asserting the argv the fake engine received (no `judge` under `Older` or `Unknown`).
Two slot-held cases (r3 minor 1): with `Scheduler.running` held by a slot, `Accepted` gives "Reading
with this run" and starts nothing; `Unknown` (the refresh failing) gives the waiting outcome, never
"Reading with this run", and starts nothing. *Done:* green; both handler lists recounted by script.

**T8b (implementer).** *Files:* `app/src/onboarding.rs`, `app/src/scaffold.rs` (the `syllabi/` folder
at creation), `app/tests/onboarding.rs`. `pick_file` gains an optional extension list with its callers
unchanged; at Finish, staged step-1 files go to `syllabus-add --source lms` for courses still listed,
the rest are deleted unread, and the staging folder goes (or, by P0's fact 5, at the console's first
open); the one-hour sweep like `sweep_stale_sessions`. The `syllabi/` folder is created at vault
creation by `scaffold.rs` alone. `ids::NOTE_FOLDERS` (`engine/src/ids.rs`, contract list) stays
untouched, since `syllabi/` holds no notes, and the backup already carries it (T2c). Files handed over
at Finish only become `pending`. They are read by the first slot whose gate is `Accepted` (P3), and a
new account records the current version at sign-in. *After:* L6's T3b. *Done:* tests green.

**T9 (implementer).** *Files:* `app/src/lms_link.rs`, `app/tests/lms_link.rs` or its existing test
file; the wizard handler line after L6. Spec §3.1 and §4.1's bounds (3 files, 10 MB, per-course
timeout): `capture_syllabi(unitid, courses)` writes to staging only, per course one outcome, a
gated link recorded and never opened (Q44). Tests against a `127.0.0.1:0` server shaped by S1's note.
*Done:* green.

**T10t (test-writer).** *Files:* `app/tests/syllabus_link.rs` (new). Spec §13 row T10t, every case,
with a fake `Page` (the mail runner's seam as P0 found it) and a `127.0.0.1:0` server.

**T10a (implementer).** *Files:* `app/src/scaffold.rs`, `app/src/syllabus_link.rs` (new, the pure
half). `Curated` gains `syllabus_host` and `syllabus_kind`, `None` on **every** row; the host checks and
the script list as pure functions. *Done:* T10t's pure cases green.

**T10b (integrator, Opus high, then reviewer).** *Files:* `app/src/syllabus_link.rs` (the window),
`app/assets/syllabus/*.js` (new). Spec §15.2 "The window and the in-page read", all of it, with S3's
answer for the read path (Q29's order). Logs carry counts only. *Done:* T10t green; `reviewer` PASS.

**T10c (implementer).** *Files:* `app/src/main.rs`: `syllabus_connect` and `syllabus_connect_status` in
the console list, and the `syllabus-signin` window's data-folder wipe on `Destroyed` and at exit,
beside #19's `lms-signin` wipe (matching its own label only). *After:* L6 merged. Recount by script.

**P3 (implementer). The slot reads syllabi only under an accepted version (Q-K; r1 finding 3).**
*Files:* `app/src/scheduler.rs` (one decision where the slot builds `judge`'s argv, read on `main` by
P0 item 14), `app/tests/syllabus_slot.rs` (new). *Owner of the guarantee:* this task for the slot,
T8a for the scoped chain, and T5 for the engine's default. *Behaviour:* the slot's `judge` step gets
`--syllabus` only when L4's gate is `Accepted` for the build's `PRIVACY_VERSION`. Three rules fix
when and how that is read (r2 finding 4):
1. **The gate is read after L4's refresh, in the same slot.** P3's decision comes after L4's block
   in `run_slot_with` has run for this slot, including its one `refresh_privacy` call when the cache
   was not `Accepted` (fact 13). It reads the gate as that block left it, so a refresh that finds
   `Accepted` passes the flag this slot, not the next. When L4's block does not run (the vault is not
   `Entitled`), P3 reads the cache as it stands and still passes the flag only on `Accepted`. P3
   never calls `refresh_privacy` itself.
2. **The syllabus row exists only when `judge` runs.** The named row `syllabi (skipped: privacy update
   not confirmed)` (Quinn words it), at exit 0 and following L4's row rules for the runner log, is
   added only when judge's plan, after L4's filter, is `JudgePlan::Run` and the gate is not
   `Accepted`. When judge's plan is a `Skip` for any reason (no entitlement, no account, L6's own
   skips, L4's `Older` skip), no syllabus row is added: the judge row already names why nothing ran.
3. **Under Q-GATE's fail-closed alternative, `Unknown` acts as `Older`.** If Quinn rules L4's Q-GATE
   that way, L4 skips the whole `judge` under `Unknown`, and P3 adds no syllabus row (rule 2). P0
   item 14 records which Q-GATE ruling is on `main`, and the `Unknown` test asserts that ruling.
Under `Older`, L4's whole-`judge` skip is unchanged. `slot_argv`'s other entries, L4's filter and L6's
judge plan are otherwise untouched. *Tests first*, against a fake engine binary recording argv and
L4's account seam as L4's T6 tests drive it:
- `Accepted` gives `judge … --syllabus`;
- `Unknown` with a refresh that answers accepted gives `judge … --syllabus` in the same slot;
- `Unknown` with a failed refresh gives `judge` without the flag plus the named row (under Q-GATE
  as recommended), or no `judge` and no syllabus row (under the fail-closed alternative);
- `Older` gives no `judge` and no syllabus row;
- a judge `Skip` from L6's plan under `Unknown` adds no syllabus row;
- a vault with no `state/syllabi/` under `Accepted` still passes the flag, which the engine treats
  as a plain `judge` (T5t's no-folder case), so the slot never reads the vault to decide.
*After:* L4 and L6 merged; T5 on `main` (PR-1). *Done:* green; `app/tests/scheduler.rs` green and
unedited.

**T10d (mechanical, then reviewer).** *Files:* `app/src/scaffold.rs` (UA's `syllabus_host` only) and
the one test that pins it. Only once Q39's three conditions hold (S3 passed, T18 finds no plain bar,
L4's page carries §6.1 and, under Q-A, P1 has landed); the controller records each in the ledger first.
Otherwise it waits for a later 0.1.x, and PR-2 ships with every host `None`.

### The page (PR-2, the `console-ui` queue slot)

Every task: `app/static/console.js` (and `index.html`, `console.css` where named),
`app/tests/static_assets.rs`, `scripts/wizard-check.py` run by the controller; **Quinn words every new
string** (spec §8's table is draft wording; the console words each read-model code). Screenshots by
fixture, by window handle only. Each task starts when the previous one is merged into the branch.
**L5's slot in the queue runs T11a-T11f and then P1's `console.js` entry, and it is released only
once P1 has committed.** No lane behind L5 takes `console.js` in between. **While the privacy update is
owed** (L4 §4.6), every control a task here adds that starts syllabus reading (each "Add a syllabus",
paste, Offer A's connect, and the confirm screen's offer) joins L4's disabled-with-one-line set
("Accept the privacy update to use this"). Viewing what is already there stays live. Each task's
static test names its new ids in that set.

- **T11a.** Spec §3.1 "On the first-day confirm screen" and §8's two changed strings: the "Your
  syllabi" section with each course's step-1 outcome and the offer; uncovered rows point at it; no
  preset on a `contested` row; `static_assets.rs`'s pinned "no class times found" string changes in the
  same commit, with the reason. *Starts:* after L1 R11 (or its recorded later slot) and #19's B5.
- **T11b.** The ask card's offer and pending note; the Schedule view's course entry rendered from
  T17's `syllabi` fields (office hours, items not proposed with their states, notices, the offer).
  A `past` item whose `date` is today is worded as due today, never as past: on the vault's first
  day an item due that day is `past` to the filer (r10, charge day 2) but not to the student (r10c,
  the r10b review's M-3; a fixture case on T17's shape pins the wording). *Needs:* T17.
- **T11c.** The wizard calls `capture_syllabi` after `capture_courses`, with its own outcome line;
  `finishMayRun` waits for it as it waits for #19's buttons. *Needs:* T9.
- **T11d.** Offer C: the inline typed row if P0's fact 4 says `--confirm` takes one (with a thin
  `app/src/week.rs` command by `implementer` if needed), else the day-3 note (Q27).
- **T11e.** The deck renders `syllabus-exam` cards and a grading amend card's mapping change
  legibly. *Needs:* T4e.
- **T11f.** Offer A on the page where a host is curated (none until T10d), its outcomes, the match pick
  and "Use Simple Syllabus's instead" from T17's `held`; the course's policies and quotes; the quote
  beside a grading amend card from the local file; the past-due row's late note. *Needs:* T8a, T10c,
  T16, T17, T16s.

### Go-live, review, merge, docs, proof

**P1 (if Q-A; contract-engineer at xhigh, then contract-reviewer; `reviewer` reads the page text;
Quinn reads the page). Privacy bump #2, one commit, mirroring L4's bump (L4 plan §4.2, its T13).**
*Before starting:* L4 merged (it owns the page and the constant until then); T11f and P3 on the
branch; Quinn's answer to Q-A's Q-AGENT extension recorded as "yes" (otherwise P1 does not start; see
Q-A). P1 runs inside L5's `console.js` queue slot (above).
*Files and tokens, five files, all to `V2` together and nowhere else (r2 findings 1 and 8):*
1. `site/privacy.html`, three changes:
   - the content change: the "(Knowlu does not read syllabi in this version …)" parenthesis out, and
     the §6.1 lines as L4's ruling on its Q2 leaves them;
   - **the change-rule sentence (fact 14):** under L4's Q3 (ii), "Reading a kind of material this page
     says Knowlu does not read, such as syllabi, is never a smaller change." loses its example and
     reads "Reading a kind of material this page says Knowlu does not read is never a smaller
     change." That is the recommended wording; Quinn reads it with the page. Under Q3 (i), the
     sentence has no example and is left alone. L4's plan is not edited: its sentence is right until
     P1 makes Knowlu read syllabi;
   - then its **Effective**, **version** and **"Last changed"** lines.
2. `app/src/account.rs`: `PRIVACY_VERSION = "V2"` (one line; the `:21-23` rule that page and constant
   move in one commit).
3. `app/static/console.js`: the panel's **"what changed" entry**, `PRIVACY_CHANGES` keyed `"V2"`, with
   Quinn's three or four lines on syllabus reading. Whether L4's `V` entry stays follows L4's merged
   design (P0 item 14).
4. `engine/tests/site.rs`, where L4's page pins live (L4's T9; §4.2):
   - **reworded:** L4's T9 pin of the Q3 (ii) change-rule sentence (from the draft's §4.3 "L4.1"
     list, "the Q3 (ii) or (i) sentence") now pins the sentence without "such as syllabi". This is
     the only existing pin P1 edits, and only under Q3 (ii). The commit message says why: the page
     now reads syllabi, so the example would contradict it;
   - **new:** the page no longer carries "does not read syllabi in this version", and no longer
     carries "such as syllabi" (moved here from `static_assets.rs`; page pins belong in `site.rs`).
5. `app/tests/static_assets.rs`: one new pin on the panel, worded against `PRIVACY_VERSION` (r3
   minor 3). The test reads `PRIVACY_VERSION` as L4's version-key pin does; **while it is `V2`**,
   `PRIVACY_CHANGES`'s entry for that key must mention syllabi (a case-insensitive "syllab"; Quinn
   words the lines). L4's T7 version-key pin already checks the key; this pin checks that the lines
   changed with it. The `V2` date is a literal in the pin's condition, so once the constant moves
   the condition is false and the pin cannot fail the next bump, even if L4's "one key per bump"
   drops `V2`'s entry. **The next privacy bump retires this pin** (deletes it in its own bump commit,
   with the reason in the message), since it no longer checks anything.
*Checked, not moved:* `PRIVACY_BUMP_1` (`app/tests/grades.rs`) stays at L4's `V`. It names bump #1,
and the date-and-bump test needs only `PRIVACY_VERSION` at or after it, which `V2 > V` satisfies.
*`V2`:* the date P1's commit is made, never re-stamped (L4's Q-V). It must be **strictly later than
L4's `V`**; if both would fall on one day, P1 waits a day.
*Green after, with no other edit:* `the_privacy_version_constant_is_the_published_pages_date`, L4's
`PRIVACY_CHANGES` version-key pin (its T7), the date-and-bump test,
`the_privacy_guard_blocks_mail_code_at_or_before_the_pre_bump_version`, and every T9 pin of L4's
(the change-rule pin as item 4 rewords it; the "Last changed" pin reading `V2`). The controller runs
`scripts/ci/privacy-guard.ps1 -Root <worktree>` and records its line. The red run for item 4's and
item 5's new pins is recorded first, as every test task's is (§2, TDD).
*Effect:* every device whose recorded version is `V` reads `Older` against `V2` from its cache, so
L4's panel shows and the slot's `judge` is skipped until Accept. A new account records `V2` at sign-in.
The page reaches `knowlu.com` only at a release (L4's Q-LIVE), and T12's release guard holds until it
does. *Done:* `git show --stat` lists exactly the five files (`site/privacy.html`,
`app/src/account.rs`, `app/static/console.js`, `engine/tests/site.rs`, `app/tests/static_assets.rs`);
`contract-reviewer` PASS; Quinn's word on the page and on the `PRIVACY_CHANGES` lines in the ledger.
Commit: "privacy(l5): privacy bump #2, the
page says Knowlu reads syllabi (version V2)".

**T14b (reviewer; contract-reviewer over P1).** As T14a over PR-2's diff, plus: every new handler is in
the right list and the counts are recounted by script; the `syllabus-signin` window is never a kept
profile; no page text in an error or log; every host `None` unless T10d's ledger rows exist; Q42's and
Q-A's conditions recorded (the Q-AGENT extension among them); P1's commit moves exactly the five
files, and the page carries no "such as syllabi" or "does not read syllabi"; no path runs
`judge --syllabus` or `--only syllabus` except under `Accepted` (P3, T8a). Report
`docs/reports/2026-10-2x-l5-pr2-review.md`.

**P9 (main session).** Gate, PR ("L5 part 2: syllabi in the app"), CI, merge on green CI and T14b
PASS with Q42's gate and Q-A's conditions in the ledger. PR-2 carries no migration, so M7 does not
apply. L1 was told at P8.

**T13 (docs-keeper).** Spec §13 row T13: `docs/reference/engine-commands.md` (`syllabus-add`'s four
forms, the held renderings, the replacement rule, `judge --syllabus` and `--only syllabus` (a plain
`judge` reads no syllabus, Q-K), `contested`, the new card
kind, the three amendable fields), `docs/reference/app.md` (the commands, counts by script, the
staging folder, `syllabi/`, the Simple Syllabus window beside the mail window's rules),
`docs/surface/anatomy.md` (the `syllabi` read model), `HANDOFF.md` (the lane done, the release guard
satisfied or still standing). From the merged code, not this plan's words.

**P10 (main session with Quinn).** On a dev build of `main` and a scratch profile: Offer B with an
**invented** syllabus file (made for the proof, never a real one), the confirm screen's rows, a card
after day 1 by the slot, the grading amend card; step 1 at UA if T9 merged; Offer A only if T10d
landed. A proof note in `docs/reports/` with counts and pass/fail lines only; the scratch profile
removed after (memory: live-proof profile cleanup).

## 7. Checkpoints

1. **Plan review** (`reviewer`, plan mode) before P0; answers in §10, status to REVIEWED.
2. **Quinn**, one at a time with the recommendation: Q-A (with L4's Q2, and the Q-AGENT extension to
   a second bump), Q-K (the pass runs only when asked), Q-D (the block), Q-E (the folder), Q-J (at
   P8), Q48 only if it arises. Q-B is ruled (2026-10-19, last-write-wins), and so is Quinn's
   simplification (2026-10-19). Any desktop withdraws a grading card whose field no longer holds its
   `from` as answered, nothing is re-filed, and only a syllabus change files a new card. That
   supersedes the earlier follow-up ruling, under which an agent write re-based the card. Quinn's two
   rulings of 2026-10-20, as relayed by the controller, are quoted in Q-B and here:
   > **I-3.** A pending card the current syllabus no longer supports is withdrawn quietly on any
   > desktop, and nothing is re-filed. "No longer supports" means either the syllabus no longer gives
   > its value, or the note already holds it.
   >
   > **G10.** The one-time card is accepted, as the planner described it.

   The four rulings go in the lane ledger at P0, quoted. Reported to Quinn as decisions taken, not
   asked:
   - an approved grading card applies as `agent:approvals`, like every amend card (G5);
   - one field moved withdraws the whole card (G7);
   - *set changed* replaces and never only removes, and waits for room in the day's 8 rather than
     leaving a gap (G8; r6 M-1);
   - an unsupported card's replacement can wait a day for room, as the ruling withdraws it at once
     (G11);
   - duplicates keep the first by id, then file name, decided only where the extraction is (G2;
     r6 M-2);
   - the item-card twin (T4d'c), and G11's note half for a task or exam card whose key a task or
     commitment already carries;
   - Q-B's two costs that follow from I-3's words: a reverted set ends with no card, and two
     desktops whose syllabi disagree end with no card;
   - *(r8, a controller reading of I-3)* "the syllabus no longer gives its value" means a present,
     valid, different value. An absent one (a dropped or undated item, a grading part that fails
     validation or was not read) leaves the pending card, by Q15's reasoning, so a failed read never
     costs the course its card (Q-B's fourth cost; the r7 review's I-2);
   - *(r8, made concrete in r9; the r8 review's M-2)* on a full day the syllabus cards take up to 8
     of the 15 ahead of event and commitment cards, since the hook runs before the events pass is
     sized (T4d'a). At the start of a term about 30 items fall inside the horizon at once and take 8
     a day for about four days. An obligation event inside those days can get no card before it
     happens, because the events pass files an obligation only while it is still ahead. Since r9 the
     filer is also held to what is left of the 15, so the cost is capped at 8 a day and the syllabus
     cards never take a day past the 15. *Noted with it:* the hook after the events block would now
     also stay inside the 15, so the line is only a priority choice, syllabus items ahead of event
     cards. The plan keeps that priority unless Quinn prefers events first;
   - *(r9, the r8 review's I-1; bounded in r10, the r9 review's M-3)* the syllabus cards wait
     behind every card charged before the hook (mail, held cards, earlier slots). An item is planned
     again on each run while it is in the horizon and ahead, so it reaches its day with no card only
     if every run from the day it entered the horizon is full. An item that enters late (a syllabus
     added mid-term with a quiz two days out) has only the runs left before its day;
   - *(r10, the r9 review's I-1; a decision taken)* on the vault's first day the horizon and "never
     past" count from day 2, the day held cards charge, so day 1 files no card for an item due that
     day (it would expire unseen on day 2 and take a slot from a later item);
   - *(r10b, the r10a review's I-2; a decision taken)* `past` joins version 1's per-item codes
     (spec §5 :880, place 10's twelfth change), so the course can say "12 past"; on day 1 an item
     due that day is `past` to the filer, since day 1 files nothing for it, and the console words
     it as due today, never as past (T17, T11b; r10c, the r10b review's M-3);
   - *(r9, the r8 review's M-1; an option for Quinn in r10, the r9 review's M-1)* a task or exam card
     expires on its item's day (amend cards stay `null`), so an unanswered card whose day has passed
     is archived as `expired`. **One-line option: should `expired` item history keep blocking the
     key (the plan's default), or unblock it under the twin's day rule, as `superseded` does?**
     *Default's cost:* an item whose card expired unanswered and which the syllabus later moves to
     a new day (an exam rescheduled after its original date) gets no card, as with an expired event
     card; nothing else is lost (G8 keeps grading cards `null`, every desktop agrees by date, and the
     expired card is archived the next day). *Alternative's cost:* a thirteenth place-10 change (renumbered in r10b, the twelfth being `past`; the
     spec's `existing_source_uids` rule, whose one exception is `superseded` history, gains `expired`
     item history, the filed day staying blocked so nothing loops); one more predicate and one T4dt
     (C) case in `syllabus_cards.rs` (T4d'c, no contract file); each re-filed card takes a slot of
     that day's 8 and 15; and item cards then differ from event cards, whose expired history still
     blocks.
   Accepted by Quinn (2026-10-20): the G10 edge, with the review's two notes in Q-B's record.
   Then: the copy for T11a-T11f, P3's and T8a's named rows and P1's
   `PRIVACY_CHANGES` lines as each lands; the page under P1, with fact 14's reworded change-rule
   sentence; and, before T7b, the consent manifest filled for each seed (§2; Quinn's, no question).
3. **Controller at dispatch:** Q-C, Q-F, Q-G, Q-H, Q-I (Q-L was retired in r6); P0's fourteen facts;
   L3's and L8's clearing before P8.
4. **After each test task:** the red run in the ledger, one reason per test.
5. **R1** after T4a; **T4d's**, **T4d'b's**, **T16's** and **T17's** reviews; **T4e's** (`approvals.rs`
   and `sync.rs`, Q-B) and **P2's** `contract-reviewer`; **T10b's** review.
6. **T14a** before PR-1; **T14b** before PR-2; then each merge on green CI and PASS (and Q-J for PR-1;
   P8's push and merge back to back).
7. **P10**'s proof note, before the lane is called done.

## 8. Out of scope, and what this lane hands on

- **Registrar finals.** Skipped and counted by #19 (close-out Q10 (a)); the `HANDOFF.md` §4 deferral
  row stays. A syllabus final is proposed as spec §4.6 says.
- **The spec's non-goals** (§2, §16.6): GPA, the uncertainty band, exam prep, grade-weighted priority,
  the weighted ring, readings, topics and cancellations, materials, credit hours, extra credit, OCR,
  syllabi by email, a cross-student cache, and syncing syllabus files (Q18, revisit with the relay).
- **`rank` reading `late_policy`** (Q34), adoption from `coursework` (Q45), the instructional-method
  mapping (Q36), and an LMS session outside the wizard (Q11 (b)).
- **A second Simple Syllabus school**: one curated row, a later data change.
- **`CLAUDE.md` (Quinn's; no agent edits it).** Proposed after PR-2 merges, in one docs commit if Quinn
  agrees:
  1. "Engine invariants", the vault sentence: add that `syllabi/` holds the student's own syllabus
     files (PDF, DOCX, HTML, TXT), the one non-markdown folder, never synced (spec Q8).
  2. The agent-actor list gains `agent:knowlu.syllabus` (the syllabus filer).
  3. The commitment-proposals paragraph: a `syllabus-exam` card, like `event-accept`, proposes a
     one-off from coursework and syncs like any deck card (spec Q20, Q46); only the kinds in
     `LOCAL_CARD_KINDS` are local.
  4. "Command and app contracts": `syllabus-add` never writes a note or a journal record and exits 0
     or 2, writing nothing on 2; `judge --syllabus` and `judge --only syllabus` write only
     `state/syllabi/`, and a plain `judge` reads no syllabus (Q-K).
- **Spec gaps this plan fills, for the reviewer to confirm:** no §13 task created `syllabi/` at vault
  creation (now T8b); §13's T6a named five kind checks and missed `eval.sql`'s two (now T6a); the
  `rank` hook was "a controller hand-off" with no task (now T4d'a); the PDF case's interim `#[ignore]`
  is removed by T3; nothing gated the slot's pass on the accepted version (now Q-K, T5, P3, T8a); the
  route's `judgments` row could keep extracted text (now T6b's `fieldsOf` arm in `judge_pipeline.ts`
  and its two marker tests); §13 has no task for the slot's flag decision (now P3; Q-K's amendment,
  place 3).
- **Moved, not filled (r1 finding 10):** T10c is the spec's own task (spec §13 row T10c, the two
  console commands). Only the `syllabus-signin` window's data-folder wipe moves, from spec T10b to
  T10c, because it is a line in `app/src/main.rs`, which L6 owns until it merges. Likewise T16's surface
  part moves to T16s (PR-2) so PR-1 never waits on L7 (r1 finding 1).

## 9. Fidelity ledger

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Caller's brief | plan at `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md`, written in pieces | this file | first Write and Edit-appends, each under 150 lines |
| Spec, signed 2026-10-19 | every §13 task carried, ids kept | §5, §6 | the §5 table names T0-T18, S1, S3 |
| Row 31 | #19 merges before L5; L5 re-planned around the registrar | P0 (cut after B8); §4.1 | P0's first step; §4.1's table |
| Row 39 (4); close-out B3f | L5 adds one `syllabus:` row to #19's per-source table | T4a | T4t; R1; #19's tests green and unedited |
| Spec §4.4 (E4), Q6 | registrar > syllabus > Google/ICS; `syllabus:` its own tier; renumbering keeps every pair's order | T4a | T4t's order cases (never numbers); R1 |
| Spec Q48 | stop if a #19 test pins a tier number | T4a, P0 item 2 | the stop rule; ledger row |
| Spec E5 | syllabus series out of `instances_map` | T4a | T4t's "blocks time by `meets`" case |
| Spec Q15, Q50 (a), Q49 (a) | deletion and no-usable-read drop the series with no `ended` entry; the ended stamp's dates | T4a | T4t's deletion, usable-read and stamp cases |
| Handoff §5 | L5 touches `commitments.rs`, `ingest.rs`, `cli.rs`, `approvals.rs`, a cloud judge kind, `lms_link.rs`, `console.js` | T4a-c, T4f, T2c/T4d'a/T15, T4e, T6a-b, T9, T11a-f | §4.2 |
| Handoff §5 | agents planner, contract-engineer, implementer, cloud-engineer, console-ui | §5 | the agent column (plus `integrator`, `test-writer`, `reviewer`, `contract-reviewer`, `docs-keeper`, `researcher`, `mechanical` per `CLAUDE.md`'s table) |
| Handoff §5 | `approvals.rs` is L5's only, by `contract-engineer` and `contract-reviewer` | T4e | §4.2's row; T14a's contract check |
| Handoff §5 | `surface.rs` after L7 | T17, T16s (PR-2 only) | §5's "After: L7 merged"; T14a finds no `surface.rs` line in PR-1 |
| Handoff §5, L6 plan Q7, close-out §5.4 | `console.js` queue L6 → L4 → L7 → #19 → L1 R11 → L5 | T11a-f | §2's queue bullet; T11a's start condition |
| Handoff §5, L6 plan §5.1 | new Tauri commands only after L6 merges | T8a, T9, T10c | each brief's "after L6"; T14b's recount |
| First-day allowance plan §2.1 (M1-M8), r8 | L5's migration stamp by the shared rule, cited not copied | T6a | §2's migration bullet; the ledger's stamp row; M5 and M7's dry run at P8 |
| M7 (r8): cleared means on staging and on `main`; a pushed, unmerged lane holds every lane behind it | L5 pushes only at P8, after every lane ahead is cleared (L3, L8 and any lane that overtook L5 by a recorded slip, M8), and merges at once; no early push for the eval | Q-I, T6a, T7b, P8 | P8's steps 2-9 as ledger rows; T7b's "staging's migration list unchanged" |
| M7 (r8): the controller withdraws alone only on failed SQL, by M7's five withdrawal steps | P8 step 6 withdraws on failed SQL by M7's steps as listed; step 8 asks Quinn before any other withdrawal | P8 | P8's text; ledger rows |
| M7 (r8): a dry run before every push and every merge; the pre-merge dry run lists nothing once pushed | P8 step 5 (before the push: L5's version alone) and step 7 (before the merge: nothing), each on the rebased head | P8 | the two dry-run ledger rows |
| r2 finding 3 | CI green on the rebased head before the staging push | P8 step 4 | the head's sha and CI result in the ledger |
| L3 plan Q-F | `judge_allowances.kind` not widened; L5 rebases on L3's capped rule | T6a, T5 | T6a's brief; Q-F |
| Q14 (row 122) | real syllabi never committed, never in a fixture, test, spec or report, stored only outside the working tree | §2, S1, S3, T6b, T7a, T7b | the rule's text in S1, S3, T7a and T7b; S1's and S3's deletion steps and ledger counts; T6b's `fieldsOf` arm and its two marker tests; T7a's reader refusals; T7b's `git status` check; Q-E |
| Q14 (row 122), "consented and not public" | each seed's owner consented and the syllabus is not public, recorded per seed in a local manifest beside the seeds, never in the repo; OpenRouter and the one pinned upstream are the only third parties that see a seed | §2, Q-E, T7a, T7b | T7a's manifest refusal and skip-count tests; T7a's marker test on the success, rejection and provider-error paths; T7b's ledger count of seeds marked, read and skipped |
| Fact 11; r2 finding 2 | `fieldsOf` keeps every syllabus-derived value out of `judgments.fields` | T6b | the `judge_pipeline_test.ts` marker test (modelled on `a_body_token_reaches_no_judgment_row`); the other kinds' tests unedited; §4.2's `judge_pipeline.ts` row |
| CLAUDE.md | `commitments/` holds only confirmed notes | T4a, T4e | T4t (proposals live in the series file and local cards); T4e (an exam note only on Approve) |
| CLAUDE.md | `LOCAL_CARD_KINDS` never sync; the tripwires untouched | all | T14a's contract check; `sync_keeps_every_local_card_kind_local` green |
| CLAUDE.md | judge once, re-propose freely | T4d, T4f, T16 | T4dt's amend-card cases; T4ft's hand-set due; spec §16.3's hash |
| CLAUDE.md | `rank` never calls a model | T4d, T4d'a, T4d'b, T4d'c | T4dt runs `rank` offline; the filer reads only the state file |
| CLAUDE.md | every note write through `write`, journal first; JSON through `ledger::dumps_value` | T2c, T4d, T4e, T4f | T1's byte case; T4dt's journal reads |
| CLAUDE.md rule 1 | no single-user assumption; the host is data | T10a, T10d | every row `None`; T10d gated by Q39; T14b |
| CLAUDE.md rule 2 | no frozen reference regenerated | every engine task | the fixtures diff and the surface oracle in each Done |
| CLAUDE.md | 0 warnings; tests offline; LF; TDD | every task | §2's gates; each test task's recorded red run |
| Spec Q42 (ruled) | no student-facing task merges before L4's page carries §6.1 items 2-5 | Q-G, P9 | PR-2's conditions in the ledger; T12's gate row |
| L4 draft Q2 | removing the "not in this version" line is material | P1 (if Q-A) | `contract-reviewer` PASS; Quinn's word |
| L4 plan §4.2 (its bump's token list) | bump #2 moves the page's content and Effective, version and "Last changed" lines, `PRIVACY_VERSION` and the panel's `PRIVACY_CHANGES` entry in one commit; `PRIVACY_BUMP_1` checked, not moved | P1 | P1's numbered file list; `git show --stat` (five files); the named pins green |
| L4 draft §3.11 and L4's T9 change-rule pin (fact 14); r2 findings 1 and 8 | the page never says Knowlu does not read syllabi once it does; L4's plan is left alone | P1 | P1 items 1 and 4; the reworded and new `site.rs` pins; T14b's page check |
| L4 plan §4.5, Q-GATE (fact 13); r2 finding 4 | the slot reads the gate after L4's refresh; a syllabus row only when `judge` runs; `Unknown` as `Older` under the fail-closed alternative | P3 | P3's six test cases; P0 item 14's Q-GATE row |
| r2 finding 5 | `judge --syllabus` with no `state/syllabi/` is a plain `judge` | T5t, T5 | T5t's no-folder case |
| r2 finding 6; r3 minor 1 | every spec place Q-K changes is listed for Quinn | Q-K, T0 | Q-K's nine places plus Q-B's place 10; the dated amendment block |
| r3 minor 1 (spec §10 row 17) | "Reading with this run" only when a slot will really read the course | T8a | T8a's two slot-held cases |
| r3 minor 4 | a `syllabus` row's `confidence` is a valid 0..1 number; tier 2 finds no feature for the request shape | T6b | test 1's `confidence` and `features` assertions |
| L4 plan Q-AGENT | a lane PR moving `PRIVACY_VERSION` needs Quinn's ruling; a second bump needs it extended | Q-A, P1 | Quinn's answer in the ledger before P1 starts |
| L4 plan §4.5 (the gate) | nothing sends a syllabus before the current version is accepted | Q-K, T5, P3, T8a, T11 | T5t's plain-`judge` and no-folder cases; P3's six cases; T8a's gate cases; the disabled controls' static pins |
| Spec Q39 (ruled) | UA's host only after S3, T18 and the page | T10d | three ledger rows before T10d |
| Spec Q20, Q46 | `syllabus-exam` syncs and carries `where` (80 characters) | T4e, T4dt | T4dt's synced-field reads |
| Spec Q25, Q34 | one amend card with weights, scale and late rule; three amendable fields | T4d, T4e, T16 | T4dt's grading cases; T16t |
| Fact 3; Quinn's Q-B ruling (2026-10-19) | `grade_weights`, `grade_scale` and `late_policy` stay last-write-wins across desktops with sync's warning and never become a card ("making the student check their grades to confirm which is correct defeats the purpose of us showing them their grades"); `AMENDABLE_FIELDS` still widened for the one-desktop grading card | T4e (`sync.rs`'s `LAST_WRITE_WINS_FIELDS` and `cardable`, same commit as the widening), P2 | P2 cases 1-3 in parity with the two existing `sync_contract.rs` tests; the red run without `sync.rs`'s lines; `contract-reviewer` PASS |
| Q-B's consequence (spec §4.6 place 10); Quinn's simplification (2026-10-19), which supersedes the follow-up ruling | any desktop withdraws a pending or snoozed grading card whose field no longer holds its `from` as answered, whatever the actor (G7); nothing is re-filed for that set, and a grading key files once (G9); a new card comes only from a new hash, replacing the pending one when the day has room (G8); one pending grading card per course, the first by id then file name, decided where the extraction is (G2); a card carries only what differs (G10); an approved card applies as `agent:approvals` (G5); the three fields compare key-sorted (G6); the item-card twin; filing is bounded and every other rule only withdraws, so desktops converge with each card charged once | T4d (G9, G2's filing half, the plan, file names), T4e (G5, G6, `withdraw_syllabus`'s signature), T4d'a (hook, count, `amendments_pending`), T4d'c (twin, survivor order), T4d'b (G7, G8, G2's survivor, G10), T4dt, P2 | T4dt's ten cases; T4e's in-module G6 and `withdraw_syllabus` cases; P2's case 4 and cases (a)-(j), (f) over three slots with its same-day variant; T4d'b's review |
| Quinn's I-3 ruling (2026-10-20) | a pending card the current syllabus no longer supports is withdrawn quietly on any desktop, and nothing is re-filed: the syllabus half where the extraction is, the note half everywhere; two desktops end with no card (G11) | T4d'c (item half), T4d'b (grading half), T4d'a (run line) | T4dt cases 6, 7, 8 and 10; P2 (f)'s different-hash variant, (h) and (i) |
| Quinn's G10 ruling (2026-10-20) | the one-time card G10 leaves is accepted, with the review's two notes on record | Q-B's costs | no test; recorded in Q-B and §7 item 2 |
| r6 review I-1, I-2 | no task after T4e edits `approvals.rs`; a withdrawn amend card leaves today.md's Must do | T4e, T4d'a | T4e's `withdraw_syllabus` cases; T4dt case 10's Must do assertion |
| r7 review I-1 | every task's Done names cases that task alone can turn green: the pure plan is named before T4d (`syllabus_cards::read_inputs`, `plan`), T4d's Done is the plan cases, the filer's `rank`-level cases are T4d'a's | T4dt, T4d, T4d'a | T4dt's groups (A), (B), (C); T4d's and T4d'a's Done lines; T4d's review |
| r7 review I-2 (controller reading, reported to Quinn) | G11's syllabus half reads one way for grading and items: only a present, valid, different value is *unsupported*; an absent value leaves the card (Q15's reasoning), and G8 never replaces a card with an absent field | Q-B (G8, G11, fourth cost), T4d'b, T4d'c, place 10 | T4dt case 6's failed re-extraction variants; case 10's *absent, not moved* variants; §7 item 2's bullet |
| r7 review M-1; spec §16.8 ("nothing changes the 15 or `defer_over_budget`'s order, and a test pins the sum") | the hook sits between `process_approvals` and `remaining_budget` (`cli.rs:430`, `:439-440`), so the events and commitment passes size from a count that includes the syllabus cards; `defer_over_budget` untouched and first; the day's 8 counts `first_proposed_at`, held day-1 cards toward day 2 | T4d'a, T4d, P0 item 6 | T4dt group (B)'s sum-of-15 case, variants (i) and (ii); P0 item 6's two re-read lines |
| r7 review M-2 | no claim that G11's note half prevents a double approval; T4e not widened | Q-B G11 | the note-half bullet's stated limit; Round 7's row amended |
| r7 review M-3 | P2 (f)'s different-hash variant has the note hold neither value; P2 (i)'s due case runs A's `rank` only after the sync | P2 | the two cases' text |
| r7 review M-4 | `Withdrawal`'s doc comment names all three producers and the counted rule; the spec learns of the caller-built reason and the counted rule | T4e, place 10 | T4e's doc-comment bullet; place 10's tenth change; `contract-reviewer` PASS |
| r8 review I-1; spec "8 a day, inside the 15" (:1242, :1267) and §16.8 | no syllabus card takes a day over the 15: `plan` files at most min(what is left of the day's 8, `max(0, budget - count_proposals_created(vault, charge_day))`), charge day today or day 2 on the vault's first day (modelled on `emit_checks`' sizing, `cli.rs:873`, which counts today only; the charge-day form is new, r10); `defer_over_budget` and its order unchanged; the false claims in T4d'a and Round 8 corrected | T4d ("The 15"), T4d'a, place 10 (eleventh change), P0 item 6 | T4dt (A)'s 15's-room cases; T4dt (B)'s sum of 15, (ii) both ways (10 + 5 = 15) and its held variant |
| r8 review M-1 | item cards' `expires` stated (a task card's `due`, an exam's date, amend cards `null`); the held variant names the three cards `defer_over_budget` snoozes by its real order (keeps the soonest `expires`, snoozes the tail) | Q-B's `expires`, T4d, T4dt | T4dt (A)'s `expires` case; (ii)'s held variant step 4 (the grading card and the two highest-stem mail cards) |
| r8 review M-2 | the hook's cost is on Quinn's list in T4d'a and §7 alike, written concretely (about 30 items, 8 a day for about four days, an obligation event in those days can go uncarded), capped at 8 a day and inside the 15 | T4d'a, §7 item 2 | T4d'a's "What it costs"; §7 item 2's r8/r9 bullets |
| r9 review I-1; r10 review M-1 (controller ruling, r10a) | on the vault's first day the horizon and "never once past" count from the charge day (day 2), so no held card is filed for an item due on day 1 and none expires unseen on day 2; the horizon is inclusive: due on the charge day + 14 (an exam + 21) is inside, + 15 (+ 22) is not, as the spec's "within 14 days" and `eventemit`'s `day <= last_day` read | T4d "The horizon", place 10's eleventh change, Q-B's `expires` | T4dt (A)'s far-edge cases, one per kind (+ 14 / + 15; + 21 / + 22); its day-1 horizon case (due day 1: no card; due day 2: a held card, `expires` day 2, pending when woken; due day 16: a held card; due day 17: none) |
| r10 review M-2 (controller go, r10a) | the course listing is `Plan`'s `listing` (per item `Filed`, `Later` and spec §5's other codes), whose `Later` uses the charge-day horizon, so it never counts a held card; spec §5's "one writer" is T2c's one function in `syllabus.rs`, which `syllabus-add`, `judge` (outcomes carried through) and `rank`'s hook (outcomes written) call | T4dt's interface, T4d (the listing), T2c (the writer), T4d'a (step 5), T5, T17, T11b | T4dt (A)'s `later` boundary case (day 1: due day 16 `Filed` and held, due day 17 `Later`); T4dt (B)'s "The listing written" (outcomes as `listing`, every other key unchanged, a same-day rerun byte-unchanged, an unknown `version` not written) |
| r10a review I-1, M-2 | `rank` never writes back a file `syllabus-add` replaced during the run: `write_outcomes` re-reads, replaces only `item_outcomes`, and skips when the SHA or `added` differs from what `read_inputs` saw; both writer forms write beside and rename (`grades.rs:583-586`); the two forms' signatures and the `item_outcomes` key are pinned in T2c | T2c, T4d'a step 5, §5 T2c row | T4dt (B)'s changed-file case (`Skipped` on SHA, on `added`; `Changed` with only `item_outcomes` differing); T1's empty-outcomes case |
| r10a review I-2 | `past` is a version 1 code (spec §5 :880 gains it; place 10's twelfth change; the M-1 option becomes a thirteenth), reported to Quinn as a decision taken | T4dt's interface, place 10, §7 item 2 | T4dt (A)'s `Past` case |
| r10a review M-1, M-3, M-4 | `Filed` is planned this run or held by a card that blocks the key under G9; same-day items order by extraction order, then `source_uid`; `judge` empties `item_outcomes` when the SHA changes | T4dt's interface, T5 | T5t's outcomes case; the interface text (M-1, M-3) |
| r10b review M-1 | a state whose SHA is not its copy's is re-read in a `--syllabus` run, so no stale `read` state keeps filing from an old syllabus | T5 | T5t's mismatch case |
| r10b review M-2 | no seventh code reaches version 1: `ITEM_OUTCOME_CODES` (six) in `syllabus.rs`; `write_outcomes` refuses any other | T2c, T4dt | T4dt (A)'s code-set assertion |
| r10b review M-3 | an item dated today is worded as due today, never past | T17, T11b, §7 item 2 | T11b's wording case on T17's shape |
| r10b review M-4, M-5 | one code per item: `Filed` > `duplicate` > `feed-differs` > `needs-date` > `Past` > `Later`; undated items in extraction order, then `source_uid`; T1's file keys sorted | T4dt's interface, T1 | T4dt (A)'s past-duplicate case; T1's empty-outcomes case |
| r9 review M-1 | expired item history blocks the key by default; the alternative (unblocks under the twin's day rule, a thirteenth place-10 change since r10b) is put to Quinn with the cost of each | Q-B's `expires`, §7 item 2 | no new test under the default (T4dt (C)'s existing blocked-key cases); a T4dt (C) case if Quinn takes the alternative |
| r9 review M-2 | `PlannedCard` names every field (A) asserts | T4dt's interface | T4dt (A)'s `expires` and day-1 hold cases compile against the named fields |
| r9 review M-3 | the "no card only if every run is full" claim is bounded for late-entering items, in the plan and in place 10's spec text; no claim that `defer_over_budget` orders as Q41 does; the charge-day sizing cited as new | T4d "The 15", T4d'a, place 10, Q-B's `expires`, §7 item 2 | no test (wording); T4dt (B)'s held variant already shows the grading card snoozed ahead of a later task card |
| CLAUDE.md rule 1; r5 Q-L | no rule reads a machine's identity | T4d'b, T4d'c | T4dt and P2 never set `KNOWLU_DEVICE`; T4d'b's inputs name no journal record, actor or device |
| CLAUDE.md (contract list); r4 | the existing sync tests and both tripwires untouched; `sync.rs` changes only as T4e names | T4e, T14a | T14a's contract check; both tripwires green |
| Spec §4.4, §4.5 (T4t: "a hand-set field follows the change path's existing rule") | a hand-set field on a syllabus series is never proposed back over, by the existing check | T4a, T4t | T4t's two parity cases against a `google:` twin; R1 |
| Spec Q8 (never synced); backup design | `syllabi/` is backed up to the student's own folder and never synced; `ids::NOTE_FOLDERS` untouched | T2c, T8b, P2 | the backup pins; P2's `is_note_path` case |
| Spec Q41 | 8 a day in order; 20 calls a day; no allowance | T4d, T6a | T4dt's order case; `judge_caps_test.ts` |
| Spec Q17 | the post-add chain takes `Scheduler.running` | T8a | T8a's tests |
| Spec §6, §6.1 | the page's statements handed to L4, never written by L5 | T12 | T12's gap list |
| Spec Q44, Q47, Q33 | no host learned from a page; held renderings; replacement by `source` | T9, T15 | T15t; T9's gated-link case |
| Spec §14 | every unverified item settled before the task that needs it | P0 items 1-14 | ledger rows |
| Money (handoff §1 item 7) | a new spend path needs Quinn's word | Q-J, P8 | the ledger row with Quinn's answer |
| Caller's brief | open questions with recommendations, Quinn's marked; conflict map with #19's changes; checkpoints; out of scope | §3, §4, §7, §8 | this file |

## 10. Fidelity notes

Review findings and their answers land here, one table per round, as in the L6 plan.

### Round 1 (2026-10-19): `docs/reports/2026-10-19-l5-plan-review-r1.md`, FAIL (0 critical, 6 important, 4 minor)

Every finding is answered in the tasks themselves; this table says where. No spec change, except
the one sentence Q-K asks Quinn to amend (§4.3's "a plain `judge` run in the slot runs the syllabus
pass").

| # | Finding | Answer | Where |
|---|---|---|---|
| 1 | The early staging push can block L1 indefinitely | Took the review's fix. T7b runs the pipeline in-process on the controller's machine (fact 12), so the eval needs no route or migration on staging. A different winning pin edits T6a's unfrozen file in place before P8. The migration's only push is P8's, after L3 and L8 are cleared (on staging and on `main`), from the lane branch and immediately followed by the merge. Failed SQL is the controller's own withdrawal; any other withdrawal asks Quinn. Until P8, L5 has neither merged nor pushed, so a stalled PR-1 can be recorded as slipped and L1 is never held. T16/T17 are pinned: T16's engine part is in PR-1, while T17, T17t and the new T16s (the surface part) are in PR-2, so PR-1 never waits on L7. The false "which follows at once" is gone | fact 10; §2 (the `surface.rs` and migration bullets); Q-G; Q-I; §5 (T6a, T7b, T16, T16s, T17, P8 rows; the order line); T6a; T7b; T14a; P8 steps 1-7; P9; §9's two M7 rows |
| 2 | The version bump is incomplete | P1 now lists every token of L4's bump: the page's content and its Effective, version and "Last changed" lines; `PRIVACY_VERSION`; and the panel's `PRIVACY_CHANGES` entry in `console.js`. `PRIVACY_BUMP_1` is named and deliberately not moved (it names bump #1; `V2 > V` satisfies its test). Also: `V2` must be strictly later than L4's `V`; a static pin that the parenthesis is gone; the exact four-file `git show --stat`; the named pins. P1 runs inside L5's `console.js` queue slot, which is held until P1 commits. Q-A gains the Quinn line extending L4's Q-AGENT ruling to a second bump, and P1 does not start without it | Q-A (its sub-bullet); §4.1 static row; §4.2 static, `grades.rs` rows; §5 P1 row; the page section's slot sentence; P1; T14b; §9's two L4 rows |
| 3 | Nothing stops a scheduled `judge` from sending syllabi before re-acceptance | Owners named: T5 (engine default), P3 (the slot, new), T8a (the scoped chain), with T11's controls disabled while the update is owed. Mechanism (Q-K, new Quinn question): fail closed. A plain `judge` never reads `state/syllabi/`; the slot passes `--syllabus` only when L4's gate is `Accepted`; `Unknown` gets a named skip row; `Older` keeps L4's whole-`judge` skip. Tests: T5t's plain-`judge` cases (no request, file bytes and output unchanged), P3's three gate cases on the argv, and T8a's gate cases | fact 13; Q-A; Q-K; §2 (the `scheduler.rs` bullet); §4.2 `scheduler.rs` row; §5 T5t, T5, P3 rows; T5t; T5; T8a; T8b; P3; the page section; T14a; T14b; §9's gate row |
| 4 | P2's test can pass while the conflict card cannot be applied | P2 now approves the conflict card and asserts the foreign value lands as a single-line flow mapping through `write`. It adds the pending-syllabus-card case on the same field, run in both orders: the approved card wins and the other fails the existing stale check, never applied. T4e must accept `sync`'s exact `from`/`to` mapping shape, with an in-module test built in that shape. P0 item 6 records the shape and the stale outcome first | Q-B; P0 item 6; T4e; P2; §9's conflict-card row |
| 5 | The Q14 rule is missing from S1, S3 and the staging route | §2's rule now covers any file a spike downloads and forbids every hosted Knowlu path. S1 and S3 carry the rule, a scratch folder outside every tree and synced folder, and a deletion step with a ledger count (or a move to Q-E's folder if Quinn names a seed). T6b gains the nothing-kept rule (fact 11: `judgments.fields`) and a marker test over the log row and console output. The env-variable seed reader is T7a's code, with refusals and tests, so T7b truly writes nothing in the repo | fact 11; §2; S1; S3; T6b; T7a; T7b; §5 T6b, T7a, T7b rows; T14a; §9's Q14 row |
| 6 | T4a's brief does not say how a hand-set field on a syllabus series lands | T4a cites the rule by quote (spec §4.4's "under that path's existing hand-set rule (§4.5)"; §4.5's `is_human_set` "never proposed back over"; the ended arm's "hand-set rule on `until` still holds"), adds no branch, and stops if one would be needed. T4t pins it as parity with a `google:` twin, for a field and for `until`. P0 item 13 confirms both paths read the same check | P0 item 13; T4t; T4a; R1; §9's hand-set row |
| 7 | T4t's `sync::is_note_path` case is a sync test written by `test-writer` | Moved into P2's file, `engine/tests/syllabus_sync.rs` (renamed from `syllabus_sync_conflict.rs`), under `contract-engineer` and `contract-reviewer`. That part of P2 is unconditional | §1 (two bullets); §4.2 row; §5 P2 row; T4t; P2 |
| 8 | T8b: `ids::NOTE_FOLDERS`; `BACKUP_FOLDERS` | `ids::NOTE_FOLDERS` is stated untouched in §1, §4.2 and T8b. Decided: `BACKUP_FOLDERS` gains `syllabi` (T2c, `implementer`, not contract), with its two tail and length pins changed in the same commit and the reason given, so a restore keeps every copy. The backup is the student's own folder, as `config/` is, and is not sync. Flagged to Quinn in the hand-off as a decision taken, not a question | §1; §4.2 `backup.rs` row; T2c; T8b; §9's backup row |
| 9 | T1's "PDF built in memory" needs a writer | T1 hand-builds minimal PDF 1.4 bytes (standard-14 font, one uncompressed stream, computed `xref`) in a test helper of about 40 lines, with no dependency. DOCX goes through the existing `zip`. If T3's crate cannot read them, T3 stops and the controller amends for a committed invented PDF with its `-text` line | T1; §4.2 `.gitattributes` row |
| 10 | §8's "spec gaps" wrongly counts T10c | Corrected: T10c is spec §13's own task, and only the window wipe moves (from spec T10b to T10c, because `main.rs` is L6's). That sits under "Moved, not filled", beside T16s. The gap list gains the two real gaps this round found (the consent gate and the `judgments` row) | §8 |
| — | Line 9 named a different report path | Line 9 now names this report and the `-r2` naming for later rounds; the status reads DRAFT r2 | status block |

### Round 2 (2026-10-19): `docs/reports/2026-10-19-l5-plan-review-r2.md`, FAIL (0 critical, 2 important, 6 minor)

Round 1's ten answers stand, as the report confirms. One part of round 1's preamble is superseded:
Q-K changes eight places in the signed spec, not "the one sentence" (finding 6). The controller ruled
findings 1, 2 and 7; findings 3 to 6 are applied as the report says. No spec change is made by this
plan; Q-K's amendment still waits for Quinn.

| # | Finding | Answer | Where |
|---|---|---|---|
| 1 | P1 cannot keep L4's page pins green: L4's T9 pins "Reading a kind of material this page says Knowlu does not read, such as syllabi, is never a smaller change", which contradicts the page once P1 removes the parenthesis; and `site.rs` is not among P1's files | The controller's ruling: the report's second fix, and L4's plan is left alone (its sentence is true until P1 makes Knowlu read syllabi). P1 now covers **five files**. Under L4's Q3 (ii), it rewords the sentence to drop "such as syllabi" (recommended wording given; Quinn reads it with the page) and rewords **L4's T9 change-rule pin** in `engine/tests/site.rs` to match; that is the only existing pin P1 edits. P1's new page pin moves into `site.rs` and also asserts that "such as syllabi" is gone. Under Q3 (i), the sentence has no example and stays as it is. `static_assets.rs` stays in P1 with one pin that `PRIVACY_CHANGES`'s `"V2"` entry mentions syllabi, which L4's key pin does not check. That makes the five: `privacy.html`, `account.rs`, `console.js`, `site.rs`, `static_assets.rs`. P0 item 14 records L4's Q3 ruling and the pin's exact name | fact 14 (:115-122); Q-A (:267-269); §4.2 `app/static` row (:439), new `engine/tests/site.rs` row (:440), `account.rs` row (:442); §5 P1 row (:508); P0 item 14 (:615-618); P1 items 1, 4 and 5 (:1154-1180), the green list (:1184-1188), Done (:1193-1195); T14b (:1202-1203); §7 item 2 (:1231); §9 rows by Source: "L4 plan §4.2 (its bump's token list)" and "L4 draft §3.11 and L4's T9 change-rule pin (fact 14); r2 findings 1 and 8" |
| 2 | T6b's nothing-kept rule needs `judge_pipeline.ts`, which no task lists: `fieldsOf` stringifies every verdict field into `judgments.fields`; L3 and L7 edit the file | The controller's ruling: `judge_pipeline.ts` and `judge_pipeline_test.ts` join T6b, with a §4.2 row "L3, L7 ahead; rebase". T6b adds a `syllabus` arm to `fieldsOf` that returns only a closed list of per-part status codes and counts, never stringifies a verdict field outside it, and does not merge `featureMap`. Every syllabus-derived value stays out of `judgments.fields` on every path that calls `fieldsOf`. The marker test in `judge_pipeline_test.ts` is modelled on `a_body_token_reaches_no_judgment_row`: markers in the parts and in a scripted verdict's room, item title, office hours, quote and late rule, then `judge()` and a direct `fieldsOf` call. One correction: `featureMap` lives in `judge_rules.ts`, not `judge_pipeline.ts`. The arm keeps it off the row, so `judge_rules.ts` is not planned; if it must change, T6b stops and reports (a §4.2 row says so). T6b now waits for L3's merge. PR-1 still never waits on L7: whoever lands second rebases | fact 11 (:90-102); §4.2 rows (:429-430); §5 T6b row (:488); T6b (:894-930); T14a (:985-993); §8 gap list (:1263-1269); §9 row by Source: "Fact 11; r2 finding 2" |
| 3 | P8 does not follow M1-M8 exactly | P8 now reads M1-M8 whole first, and M7 wins on any difference. Step 2 clears **every lane ahead**: email, L3, L8 and any lane that overtook L5 by a recorded slip (M8). New step 4 requires **CI green on the rebased head**, with its sha in the ledger, before anything reaches staging. Step 6 cites M7's withdrawal steps (**five** at r8), not "four". New step 7 is the **pre-merge dry run**, which lists nothing once L5 has pushed, and it is re-run after a fresh rebase if `main` moved. Fact 10 states the five points, and §2, Q-I, §5 and §9 say "every lane ahead" | fact 10 (:77-89); §2 migration bullet (:243-245); Q-I (:329-330); §5 P8 row (:494) and the order line (:516); P8 (:995-1025); §9 rows by Source: the three "M7 (r8)" rows and "r2 finding 3" |
| 4 | P3 needs three clarifications | 1: the gate is read **after L4's refresh in the same slot**, so a refresh that finds `Accepted` passes the flag in that slot; P3 never refreshes itself, and it reads the cache as it stands when L4's block does not run. 2: the `syllabi (skipped: …)` row is added **only when judge's plan is `Run`**; any judge `Skip` adds none. 3: under Q-GATE's **fail-closed alternative, `Unknown` acts as `Older`**: no `judge` and no syllabus row. P0 item 14 records the Q-GATE ruling. P3's tests grow from three cases to six | fact 13 (:107-114); P0 item 14 (:615-618); P3 (:1077-1109); §9 rows by Source: "L4 plan §4.5, Q-GATE (fact 13); r2 finding 4" and "L4 plan §4.5 (the gate)" |
| 5 | T5t lacks the case P3 relies on | T5t gains: `judge --syllabus` over a vault with no `state/syllabi/` behaves exactly like a plain `judge`, with the same server requests, the same vault bytes after the run, byte-identical stdout, and exit 0 for both | T5t (:828-833); P3's last test case (:1106-1107); §9 row by Source: "r2 finding 5" |
| 6 | Q-K changes more of the signed spec than "one sentence" | Q-K now lists all **eight places**, each by heading and today's line, to be re-found by quoted text: §4.3 "Where it runs" (:443-446 and :449-450); §4.3 "Running it next to the scheduler (Q17)" (:459); §10 row 5 (:1150); §11 item 5 (:1199); §13 rows T5t (:1365), T5 (:1366) and T13 (:1392); §16.7's "`judge` is where it runs" (:1748). The amendment is made by `docs-keeper` in place, with one dated block at the top of the spec in the form of the 2026-10-18 amendment's block. The fail-open alternative is noted as touching only places 2 and 7 | Q-K (:342-378); §9 row by Source: "r2 finding 6; r3 minor 1" |
| 7 | The Q14 rule in §2 is narrower than the ruling | The controller's ruling, the per-seed consent record. §2: Q-E's folder holds a **local consent manifest** beside the seeds, which Quinn fills. For each seed it records that the owner consented, the date, and that the syllabus is not public. It is never committed, copied into a worktree, quoted or summarised. §2 also says that **OpenRouter**, as well as the one pinned upstream, sees each seed. T7a's reader refuses with no manifest and reads only seeds marked both ways; any other file is skipped by count and never opened. A provider error prints its code or class, never its text. **The marker test runs on the success, validator-rejection and provider-error paths**, with the fake provider's error echoing the marker. T7b reads only marked seeds and records the counts marked, read and skipped | §2 (:197-216); Q-E (:300-307); T7a (:932-963); T7b (:965-982); T14a (:985-993); §7 item 2 (:1231); §9 rows by Source: "Q14 (row 122)" and "Q14 (row 122), "consented and not public"" |
| 8 | The P1 pin location (the same issue as finding 1) | Answered by finding 1: P1's new page pin is in `engine/tests/site.rs` beside L4's page pins | P1 item 4 (:1170-1176) |
| — | The status block named round 1 only | The status reads DRAFT r3 and names both reports and the `-r3` path for the next round | status block (:6-11) |

### Round 3 (2026-10-19): `docs/reports/2026-10-19-l5-plan-review-r3.md`, PASS (0 critical, 0 important, 5 minor)

The review confirms all eight round-2 findings closed and the planner's three decisions sound
(skipping `featureMap`, the `V2` pin, T6b waiting for L3). The controller ruled all five minors, and
each is applied in place below; no further round follows. Line numbers in the Round 2 table are as
of r3 before these minors, and its §9 references now cite §9's Source column (minor 5). Line numbers
here are for this text.

| # | Minor | Answer | Where |
|---|---|---|---|
| 1 | Q-K misses spec §10 row 17, and T8a's "Reading with this run" can be untrue under `Unknown` | Row 17 ("if a slot holds it, the slot reads the course", spec :1162) is Q-K's place 9: true only under `Accepted`, otherwise the course waits for the privacy check. Q-K now says nine places. T8a shows "Reading with this run" only when a slot holds the flag **and** the gate is `Accepted`; otherwise it shows the waiting outcome. Two slot-held tests: `Accepted` gives the line, and `Unknown` with a failed refresh gives the waiting outcome; neither starts anything | Q-K (:344-383, place 9 :374-376); T8a (:1050-1069); §9 rows by Source "r2 finding 6; r3 minor 1" and "r3 minor 1 (spec §10 row 17)" |
| 2 | Stale dependency lines: the order line, Q-C's pre-cut T7a, and T6a's L3 rule | One rule: **T6a requires L3 merged.** It costs nothing, since T6b already waits for L3, and T6a also edits `judge_pipeline_test.ts`, which L3 edits. The §5 T6a row, §4.2's `judge_caps.ts` and `judge_pipeline.ts` rows, the T6a brief's *After*, and the stamp note (provisional on L8 only) all say so. The order line puts T6a after L3's merge and says PR-1 waits on L3 for T5, T6a and T6b. Q-C drops T7a from the pre-cut list; only T7b's seed folder and consent manifest, which are Quinn's and not code, stay | Q-C (:294-298); §4.2 (:432, :434); §5 T6a row (:492); order line (:519-525); T6a (:886-899) |
| 3 | The `V2` pin may break at the next privacy bump | The pin reads `PRIVACY_VERSION` and checks the `V2` entry only while the constant equals the `V2` date, written as a literal. Once the constant moves, the pin cannot fail, even if L4's "one key per bump" drops `V2`'s entry. P1 notes that the next bump retires the pin in its own bump commit, with the reason in the message | P1 item 5 (:1202-1209) |
| 4 | A syllabus row's `confidence` is set outside `fieldsOf` and could be `NaN` | T6b sets a `syllabus` row's `confidence` explicitly at both writes, through one small function beside `fieldsOf`: the minimum per-part confidence, clamped to 0..1, with a non-finite value read as 0, or 0 with no parts. Other kinds keep today's expression. The minimum suits the per-part floor; a mean hides a weak part, and the first part's value is arbitrary. No better option was found. Test 1 gains two assertions: the row's `confidence` is finite, in 0..1 and equal to the lowest part's value (0 with no parts); and `features("syllabus", item)` is `[]` for the closed request shape (`id`, `date`, `parts`), with `judge_rules.ts` unedited | T6b (:925-934, test 1 :936-945); §9 row by Source "r3 minor 4" |
| 5 | §10's Round 2 references into §9 are one row low | Every §9 reference in the Round 2 table now names the §9 row by its Source column instead of a line number. Round 3 does the same | §10 Round 2 table (:1399-1405) |
| — | Status | "r3, review PASS (minors applied)", naming all three reports | status block (:6-12) |

### Round 4 (Quinn's Q-B), 2026-10-19: a ruling, not a review

Quinn ruled Q-B the alternative way: `grade_weights`, `grade_scale` and `late_policy` stay
last-write-wins across desktops and never become a conflict card. Quinn's reason, verbatim: "making
the student check their grades to confirm which is correct defeats the purpose of us showing them
their grades". r4 changes only what this touches. A delta review follows (`-r4`). Line numbers are
for this text.

| Item | Change | Where |
|---|---|---|
| The ruling | Recorded in Q-B with the verbatim reason. A cross-desktop conflict on the three fields lands as today: the later `ts` wins, the supersede record is kept, and sync's warning names it. Q-B now notes that VISION's "conflicts surface as proposals, never silent merges" holds only in its weaker sense: the conflict is named, not silent, but it is not a proposal | Q-B (:298-327) |
| Fact 3 | Rewritten: the widening alone would make the fields cardable. `cardable` at `sync.rs:2036` is the one closure, and its three uses (:2040, :2100, :2139) read it. The uncarded branch (:2043-2046) and the two existing tests that pin it are named | fact 3 (:59-74) |
| The `sync.rs` edit and the second list | In T4e, same commit as the `AMENDABLE_FIELDS` widening, which is still needed because the one-desktop grading card goes through `apply_amendment`. `pub const LAST_WRITE_WINS_FIELDS: [&str; 3]` lives in `sync.rs` beside `ACTOR`, with Quinn's sentence as its doc comment. `cardable` gains `&& !LAST_WRITE_WINS_FIELDS.contains(&field)`, and one comment line at step 5 names the list; the warning and every other line stay byte-identical. It is separate because `AMENDABLE_FIELDS` answers "may a card change it" (yes, for the grading card), while the new list answers sync's own question and has one reader. It lives in `sync.rs` so no sync rule enters the approvals contract | Q-B (:304-315); T4e (:844-856); §4.2 `sync.rs` row (:462) |
| Agent | T4e is `contract-engineer` at xhigh, reviewed by `contract-reviewer`, and now covers two contract files. T14a and checkpoint 5 name `sync.rs` | §1 contract-list bullet (:178-186); §5 T4e row (:526); T14a (:1111-1115); §7 item 5 (:1375-1376) |
| P2 | Now pins the opposite of r3: a later foreign write wins with its warning and no card, and an earlier one loses with no card. Both are in parity with `a_conflict_on_an_unamendable_field_takes_reconciles_rule_and_says_so` and `a_conflict_this_device_won_writes_nothing_and_files_no_card`. The list is pinned exactly and as a subset of `AMENDABLE_FIELDS`, and an amendable field outside it still cards. Red runs: without `sync.rs`'s lines, and without the *value moved* arm | P2 (:858-903); §4.2 row (:473); §5 P2 row (:527) |
| The pending grading card | Decided as the coordinator recommended. The card stays as a proposal: the next `rank` re-bases it on the foreign value (*value moved*, superseded, filed again under the same key), and approving it writes the syllabus value as the student's write. It cannot stay unchanged, because `apply_amendment` answers a stale `from` with `stale amendment` and puts the card back to `pending` every run (`approvals.rs` ~721-746). Judge once does not object: the agent only re-proposes, and the write comes from the student's approval. The alternative (treat the foreign write as the answer) is not taken. A second desktop with no extraction never withdraws the card. P2 stops if approved amends turn out to be journalled under an agent actor | P2 pending-card bullet (:875-899); T4dt (:805-814); T4d' (:905-915); P0 item 6 (:651-658) |
| Mechanism | T4e factors `apply_amendment`'s from-check into `pub fn approvals::amendment_is_stale`, so both callers share one comparison and the stale text is byte-identical. T4d' (not T4d, which runs before T4e) adds the *value moved* arm to `withdrawal_reason`. P2 now runs after T4d' | T4e (:839-843); T4d' (:905-915); §5 T4d, P2, T4d' rows (:525-528); order line (:565) |
| Sync's tests and tripwires | Untouched and unaffected. `sync_keeps_every_local_card_kind_local` and `the_servers_note_path_rules_name_every_note_folder` read card kinds and note folders, and the edit changes neither. `sync_contract.rs`'s premise in `a_conflict_on_an_unamendable_field_…` (`notes_link` not amendable) still holds. No other sync test reads `AMENDABLE_FIELDS` or `cardable` | §1 (:160-164); §4.2 "Untouched" (:494-495); T4e Done (:852-856); §9 row (:1465) |
| Removed | T4e's acceptance of `sync`'s conflict-card shape (r1 finding 4) and P0 item 6's shape fact are gone. §9's conflict-card row is replaced by the ruling rows | T4e; P0 item 6; §9 (:1463-1465) |
| The spec | The signed spec never says these conflicts become cards (checked by search), so the ruling itself changes no spec text. The *value moved* reason does: spec §4.6 "How a syllabus card is withdrawn" (:761-767) becomes place 10 in the dated amendment block beside Q-K's nine, and it is amended whatever Quinn answers on Q-K | Q-K place 10 (:411-417); Q-B (:325-327); §9 row (:1455) |
| Status and Quinn | Status r4. Q-B leaves §7's question list as ruled; the re-filing is reported to Quinn as a decision taken | status (:6-15); §7 item 2 (:1365-1369) |

### Round 5 (2026-10-19): `docs/reports/2026-10-19-l5-plan-review-r4.md`, FAIL (0 critical, 5 important, 6 minor)

The r4 delta review found the `sync.rs` edit sound: `:2036` is its only reader and no bytes change.
The *value moved* mechanism is reworked, against the email branch's `approvals.rs`, `sync.rs`,
`sync_contract.rs`, `journal.rs` and `engine/Cargo.toml`. The rules are now named G1-G6 ("G" for
grading card), because R1 is already T4a's review. Line numbers in the Round 4 table are as of r4;
the ones here are for this text.

| # | Finding | Answer | Where |
|---|---|---|---|
| I1 | Approved non-mail amends apply as `agent:approvals`, not as the student | The controller's ruling, applied. The "student's write" claim is withdrawn. G5: a syllabus grading card applies like every non-mail amend card, under the executor's context, `agent:approvals` (`approvals.rs:30-36`, `:1693`), with no new arm. Only `MAIL_CARD_CREATOR` cards use the human actor (`:905-918`). P2 case (b) asserts the `agent:approvals` record. P0 item 6 records the actor and the stale outcome | Q-B (:330-333); P0 item 6 (:692-711); T4e G5 (:899-902); P2 (b) |
| I2 | Two desktops that both hold an extraction both withdraw and file | G1: *value moved* and *set changed* withdraw only the desktop's own cards. A card is its own when its journal `create` record carries `device == journal::device_name()`; every record carries `device` (`journal.rs:288`), and pulled records keep their origin's. G2: the filer files a grading card only when no pending or snoozed grading card for the course remains, whoever filed it, which stops a second card from a different extraction. **Flagged as Q-L (controller):** the device is a hostname. Sync dropped hostname matching for its own cards after a rename (`sync.rs:1877-1883`, N19). A rename here degrades without harm: no withdrawal, the card goes stale back to `pending`, no duplicate. The smallest reliable alternative is offered: `state/syllabi/filed-cards.json`, local and never synced. Also flagged: *date moved* and *authoritative match* have the same two-desktop shape; recommend applying G1 to them too. P2 case (f) covers two extraction-holding desktops, including different grading hashes and no cap charge over three slots | Q-L (:341-362); T4d' G1, G2 (:1013-1017); T4dt case 6 (:869-872); P2 (f) (:986-991) |
| I3 | A mapping's `{:?}` comparison depends on key order, while sync sorts keys | G6: for the three fields, `comparable` compares `ledger::dumps_value(&yaml::to_json(v))`, key-sorted at every depth; other fields are unchanged. P0 item 6 confirms `preserve_order` is off after feature unification (`engine/Cargo.toml:44`). Out-of-order cases are in T4e's in-module tests, and P2 case 4 round-trips one through sync | T4e G6 (:903-918); P0 item 6; P2 case 4 (:953-959) |
| I4 | The function's scope, and per-field re-filing | G6: `amendment_is_stale` is the from-loop only (`:810-827`), and the `already_applied` gate (`:803-808`) stays in `apply_amendment`. G4: a re-based card carries only the fields not yet at `to`, and nothing is filed if none remain. T4dt case 3 is the two-field case | T4e G6; T4d' G4 (:1025-1029); T4dt case 3 (:864-866) |
| I5 | Quinn's ruling: a human write after filing is the answer | Quoted in Q-B. G3: the foreign record's actor decides, never sync's `agent:knowlu.sync` echo. A human actor means *answered*: withdrawn, and the key is blocked (the one exception to "Superseded is not a decision"). An agent actor means *re-based*. T4dt case 1 is reworked as the answered case, case 2 is the agent re-base, and P2 gains (g). §7 records both rulings | Q-B (:323-329); T4d' G3 (:1017-1024); T4dt cases 1-2 (:857-863); P2 (g) (:992-995); §7 item 2 (:1487-1491); Q-K place 10 (:446-458) |
| M1 | P2's red claim is too broad | Narrowed: cases 1 and 3 are red without the `sync.rs` lines, case 2 is not claimed, case 4's card assertions need G6, and (a)-(g) need T4d''s arm | P2 red runs (:996-1003) |
| M2 | Case 1's warning | The exact `sync.rs:2046` line is asserted for `grade_weights` | P2 case 1 (:939-944) |
| M3 | The step-5 comment | T4e amends `sync.rs:2023-2028` to name the three fields and `rank`'s re-base | T4e (:919-924) |
| M4 | B's stale copy | Case (d) pins it: `stale amendment` warning, back to `pending`, not archived (`approvals.rs:1695-1704`), and gone once A's re-base syncs | P2 (d) |
| M5 | `first_proposed_at` | Restarts on the re-filed card, as for a moved date | T4d' G4; T4dt case 2; P2 (a) |
| M6 | `withdraw_syllabus`'s context | The filer's own: `agent:knowlu.syllabus`, with the run's `via` and `run_id`; never the human actor or `agent:approvals` | T4d' (:1007-1011) |
| — | Status | r5, naming `-r4.md` | status (:6-16) |

### Round 6 (Quinn's simplification), 2026-10-19: `docs/reports/2026-10-19-l5-plan-review-r5.md`, FAIL (0 critical, 5 important, 5 minor), and a ruling

Quinn's ruling of 2026-10-19, quoted in Q-B, simplifies the cross-desktop grading card. A moved value
is the student's answer on any desktop, and only a syllabus change files a new card. r6 rebuilds the
mechanism on that ruling. Each claim was checked against the email branch's `approvals.rs`,
`sync.rs`, `journal.rs`, `ids.rs`, `write.rs`, `cli.rs`, `reconcile.rs` and `sync_contract.rs`. The
Round 4 and Round 5 tables stand as history. Their rows on the re-base and on G1, G3 and G4 are
superseded here: Round 4's "The pending grading card" and "Mechanism", and Round 5's I2, I4, I5, M3,
M4 and M5. Line numbers are for this text.

| # | Item or finding | Answer | Where |
|---|---|---|---|
| Ruling | Quinn's simplification | Quoted, with Quinn's reasoning. It supersedes the I5 follow-up ruling. Q-B's last-write-wins, T4e's widening, `sync::LAST_WRITE_WINS_FIELDS`, the `cardable` exclusion and G5 all stand. Fact 3 confirms the premise: nothing writes the three fields today | Q-B (:354-366); fact 3 (:79-101); §7 item 2 (:1684-1700) |
| Retired | G1, G3, G4, Q-L | Gone from Q-B, P0 item 6, T4dt, the old T4d', P2, §7 and §9. No rule reads a device or an actor, and no test sets `KNOWLU_DEVICE` | Q-B (:405-406); Q-L (:435-436); P0 item 6 (:783-808); §1 rule 1 (:205-206); §9 rule-1 row (:1794) |
| 1 | What remains of `amendment_is_stale` | It is G7's *answered* test and `apply_amendment`'s from-check, sharing one comparison, so the two never disagree. One more predicate on the same `comparable` serves G10. The re-base role is gone | T4e G6 (:1039-1058) |
| 1 | The reason's name, and the contract | Named *answered*, Quinn's word. **Not a contract change.** Existing withdrawals write only `status: superseded` and the archive (`approvals.rs:1880-1886`, `:1924-1930`), and a reason function is only a predicate. `journal::VIAS` (`journal.rs:34`) and the ops (`:30`) are untouched, and the existing status is reused, so no reader of statuses changes. *Answered* and *duplicate* live in `syllabus_cards.rs` (not contract) and in the run line's words. Nothing beyond T4e goes to `contract-engineer` | fact 3 (:79-101); Q-B "What the vault sees" (:407-411); T4d'a's run line (:1191-1194) |
| 2 | What blocks the key | **G9.** Any card holding a grading key blocks it, whatever its status (`existing_source_uids`, unchanged, `approvals.rs:1341-1357`). "Superseded is not a decision" stays for item keys only. The same hash is blocked, while a new hash is a new key (spec §4.7) and files. This is spec §16.3's own rule. A key needs no status of its own: under G8 a grading key never needs to file twice, and blocking every withdrawn key is what stops two desktops refiling | Q-B G9 (:387-390); T4d (:1003-1021); place 10 (:520-544) |
| I1 | Q-L is not in the plan | Q-L is retired, and no device test remains. Its second point (*date moved* across desktops) is kept as the item-card twin, with tests | Q-B twin (:393-404); T4d'c (:1198-1209); T4dt case 10 (:993-999) |
| I2 | G1 plus G2 block a course for good; `expires` | G7 runs on every desktop, one without an extraction included, and G8 lets a new syllabus replace any pending card, so nothing is orphaned. `expires` stays `null`: an expiry plus G9 would end a set for good | Q-B G7, G8 and `expires` (:377-386, :412-415); P2 (d), (g) (:1130-1160) |
| I3 | A converged card is filed again | G9 blocks the re-file. G10 drops every held field at every filing, and files nothing if none is left | Q-B G10 (:391-392); T4dt case 4; P2 (c) |
| I4 | P2 (f) cannot pass | Rewritten for three slots on successive days. Slot 1 files one card per desktop, before any sync. After slot 2, every desktop keeps the lower id. Slots 2 and 3 file nothing, so no cap unit is charged after slot 1. Variants: same hash, different hashes, and a hand edit. The four-point convergence argument is written into P2 | P2 (f) (:1139-1156); "Why it converges" (:1109-1120) |
| I5 | T4d' is about 150 lines | Split three ways: T4d'a (`cli.rs`, about 35 lines), T4d'c (the twin, about 30) and T4d'b (the grading rules, about 80, reviewed) | §5 rows (:657-660); briefs (:1173-1233); §4.2 (:586, :589); order line (:696-697) |
| M1 | `journal::is_human` | Dropped: no human test remains, since G7 reads values only | — |
| M2 | T4dt case 1 should edit the file | Applied: case 1 is a text-editor edit with no journal record | T4dt case 1 (:958-962) |
| M3 | A re-based snoozed card loses its snooze | Dropped: nothing is re-based. A snoozed card is withdrawn exactly as a pending one is (G7) | — |
| M4 | Two desktops file before either syncs | G2's survivor: the lowest `id:`, chosen by every desktop. Ids are random and travel (`ids.rs:70-76`) | Q-B G2 (:369-373); T4dt case 7; P2 (f) |
| M5 | The warning's escapes | P2 case 1 quotes the string in a code span, with real backticks and the em dash | P2 case 1 (:1081-1088) |
| 4 | Determinism and no ping-pong | Four points. A key files once per desktop at most (G9). G7 and G2 read only synced data. G8 fires only in a run that files a never-filed key. Two desktops archiving one card reconcile as nothing (`sync.rs:1955-1959`, `:2274`). G8's condition is new, because the spec's *set changed* run on any desktop lets one desktop's different extraction veto every card the other files | P2 (:1109-1120); Q-B G8 (:381-386) |
| New | Card file names | Names derived from the key let two desktops filing one key pick one path. Sync never overwrites (`sync.rs:2300-2313`), so one desktop's records would reach the other's card. T4d's names carry a random part, as `write::propose_amendment`'s do (`write.rs:1036-1040`) | fact 3; T4d (:1016-1018) |
| New | The pending line | An approved card the stale check puts back is not counted pending (`approvals.rs:1695-1705`), so withdrawing it in the same run would undercount. T4d'a subtracts only counted cards (`amendments_pending`) | fact 3; T4d'a (:1186-1190); T4dt case 9 (:989-992); P2 (d) |
| 5 | The spec | Place 10 is rewritten with seven changes, and the amendment block's listing is set. The retired rules never reached the spec | Q-K place 10 (:520-544); its amendment-block listing (:545-552) |
| Seen, not L5's | A twin miscount outside L5 | `commitment-ask`'s `AskSettled::Returned` puts an approved card back to `pending` uncounted, and `withdraw_asks` in the same run can take it out of the line (`approvals.rs:1801-1806`, `cli.rs:867-868`). This is reported to the controller, and L5 does not touch it | — |
| — | Status | r6, naming `-r5.md`; delta review pending | status (:6-18) |

### Round 7, 2026-10-20: `docs/reports/2026-10-19-l5-plan-review-r6.md`, FAIL (0 critical, 4 important, 4 minor), and two rulings

The review found the cross-desktop design sound. What remained was the task interfaces and one cost
nobody had listed. Quinn ruled on that cost (I-3) and on G10's edge. Each claim below was checked
against the email branch's `approvals.rs`, `cli.rs`, `render.rs`, `commitments.rs` and `sync.rs`.
Round 6's line numbers were for the r6 text. These are for this text.

| # | Item or finding | Answer | Where |
|---|---|---|---|
| Rulings | Quinn's I-3 and G10 rulings (2026-10-20) | Quoted, as relayed, in Q-B and in §7 item 2. The four rulings go in the lane ledger at P0 | Q-B (:380-391); §7 item 2 (:1909-1936); §9 (:2033-2034) |
| I-1 | `withdraw_syllabus` cannot carry what T4d'a and T4d'b need | T4e fixes the signature. A caller-built `reason` and the run's `amendments_pending` stems go in. The counted-pending number comes back in the existing `Withdrawal` (`approvals.rs:1944-1952`). Only pending cards are counted, and an amend card only if its stem is in the set. **One deviation from the report's form:** `reason` is `Fn(&str, &Mapping)`, taking the file stem as well, because M-2's file-name tie-break must tell apart two cards whose frontmatter is identical. No task after T4e edits `approvals.rs` | T4e (:1164-1183); fact 3 (:98-100); §4.2 (:677); §5 (:742) |
| I-2 | A withdrawn amend card stays in Must do | T4d'a removes each withdrawn stem from `approvals.amendments_pending`, which `render.rs:379-382` reads. T4dt case 10 asserts it with an `increases` due card, Must do before and after | T4d'a (:1392-1397); T4dt case 10 (:1098-1114); fact 3 (:101-102) |
| I-3 | Cards the current syllabus contradicts stay pending | **Quinn ruled: G11.** It is withdraw-only and quiet. The syllabus half runs where the extraction is: a grading card's value is not given, or an item's day has moved. The note half runs everywhere: G7 covers it for grading, it covers a due card whose `to` is already the task's `due`, and a task or exam card whose key a task or commitment already carries. *Date moved* is now unconditional, and r6's "only to make room" condition is retired. The twin's filing rule keeps the "nothing is re-filed" half. Two desktops end with no card | Q-B G11 (:434-461), twin (:462-472); T4d'c (:1406-1423); T4d'b (:1446-1448); T4dt cases 6-8, 10 (:1068-1114); P2 (f), (h), (i) (:1300-1348) |
| I-4 | T4d'a's Done cannot be met | T4d'a's Done is the byte-identical case only. Cases 4 and 9 are in T4d'b's Done, and case 10 is in T4d'c's | T4d'a (:1403-1404); T4d'b (:1455-1458); T4d'c (:1423) |
| M-1 | The day's 8 can push G8's replacement later | **Picked: withdraw only when the replacement fits.** The card G8 replaces is still supported, so withdrawing it early loses a correct proposal and leaves a gap another desktop could file into. Withdraw-and-file stays one run. Unsupported cards are withdrawn at once, as the ruling says, and their one-day gap is stated. T4d's filer is a pure plan, then writes, so G8 can read the plan | Q-B G8 (:412-426), G11's gap (:455-457); T4d (:1129-1138); T4d'b (:1433-1442); T4dt case 6, case 10 |
| M-2 | G2 needs a tie-break | The survivor order: a readable `id:` first, then `id:`, then file name. File names travel and carry a random part | Q-B G2 (:396-405); T4d'c (:1419-1421); T4dt case 7 (:1080-1088) |
| M-3 | Approve against withdraw; the same-day count | P2 point 4 states that the archived statuses differ and why that is benign, and (j) pins it. (f) is scoped to no approval. (f) gains a same-day variant in which `count_proposals_created` and the day's 8 count the pulled card and nothing more is charged | P2 (:1273-1281); (f) (:1300-1328); (j) (:1349-1356) |
| M-4 | The G10 record | Both notes are in Q-B's costs: G8 may fire under another key, and an archived empty card costs a unit unless `proposal_weight` (contract) changes | Q-B costs (:504-513) |
| New | G2 decided only where the extraction is | Without the extraction, a desktop cannot tell which card a syllabus supports. Under G11, keeping the lower id there could archive the one card a holder keeps. The holders' tombstones reach it instead | Q-B G2 (:396-405); T4dt case 8 |
| New | G11's note half for task and exam cards | This reads Quinn's "the note already holds it" for a task or exam card: a task or commitment already carries the key. (r7 also claimed this closes a latent double approval; r8 withdraws the claim, the r7 review's M-2.) P0 item 6 re-confirms that `materialize` and `create_confirmed` carry the key. **Flagged for the reviewer as an interpretation** | Q-B G11 (:451-454); T4d'c (:1412-1413); P0 item 6 (:869-901) |
| New | Convergence, restated | Filing is bounded (G9, the twin), and every other rule only withdraws. So the process ends and each card is charged once. Where it ends: the same extraction gives one survivor, different ones give no card or one both support | P2 (:1256-1281) |
| New | Two consequences for Quinn's record | A reverted set ends with no card. Two desktops whose syllabi disagree end with no card, which replaces r6's "the lower id stands". Both follow from the ruling's words and are reported as decisions taken | Q-B costs (:495-503); §7 item 2 (:1934-1935) |
| Spec | Place 10 | Now nine changes. *Unsupported* is added, *date moved* is its item half, G8 waits for room, G2 takes the tie-break and is decided where the extraction is, and §4.6's "When the syllabus changes" states the gap. The block cites four rulings | Q-K place 10 (:599-630); block (:631-638) |
| — | Status | r7, naming `-r6.md`; delta review pending | status (:6-22) |

### Round 8, 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r7.md`, FAIL (0 critical, 2 important, 4 minor)

The review confirmed every r6 finding closed, G11's convergence on two desktops, G2 deciding only
where the extraction is, and the task order. The controller took the decisions below before r8 was
written: I-1, M-3 and M-4 as the review proposed; I-2 by the review's recommended reading; M-1 by
reading `cli.rs` and pinning the line; M-2 by dropping the claim. Each claim was checked against the
email branch's `approvals.rs`, `cli.rs`, `render.rs` and `mail/` (at `1a495d0`). Round 7's line
numbers were for the r7 text. These are for this text.

| # | Finding | Answer | Where |
|---|---|---|---|
| I-1 | T4d's Done cannot be met: its filer cases run through `rank`, and the hook is T4d'a's | Took the review's fix. T4dt names the pure plan's interface: `syllabus_cards::read_inputs(vault, today) -> Inputs` and `syllabus_cards::plan(&Inputs, today) -> Plan`, with `Plan.withdrawals` (stem to `Reason`) and `Plan.filings` (`PlannedCard`s in filing order). T4dt's cases fall in three groups. (A) is the plan cases, called directly, and is T4d's whole Done. (B) is the filer's `rank`-level cases: read-back of synced fields, journal first, distinct file names, the Q45 notice, day 2's sum of 15 and the byte-identical case. It moves to T4d'a's Done. (C) is the withdrawal cases, with T4d'c and T4d'b as before | T4dt (:1100-1145); T4d (:1245-1263, Done :1271-1274); T4d'a (Done :1562-1567); §5 rows (:796-801) |
| I-2 | G11's "no longer gives its value" was read two ways: grading counted an invalid part, items did not | **The recommended reading, as a controller decision reported to Quinn.** Only a present, valid, different value makes a pending card *unsupported*. An absent value (a dropped item, an item that now needs a date, a grading part that fails §4.7's check or was not read) leaves the card, by Q15's reasoning. G8 follows: a card with a field the set leaves absent is never *replaceable*, and it still blocks the narrower key under G2's filing half. Without that, a partly failed read would replace the card with a narrower one. Recorded as Q-B's fourth cost and in §7 item 2. New cases: T4dt case 6, a failed re-extraction (whole, weights only, part not read) leaves the card pending through three runs and files nothing; case 10, a dropped or undated item leaves its card | Q-B G8 (:428-431), G11 (:454-472), cost (:546-557); place 10 (:651-661); T4d'b (:1605-1611, :1617-1623); T4d'c (:1571-1575); T4dt cases 6 and 10 (:1178-1187, :1223-1226); §7 item 2 (:2109-2112); §9 row "r7 review I-2" |
| M-1 | The hook's line against the events pass was not pinned | **Pinned:** directly after `process_approvals` (`cli.rs:430`) and before `remaining_budget` (`:439-440`). Only there are this run's syllabus cards in the count that the events pass (`:440`) and `commitment_passes` (`:873`, `:881`) size from, while `defer_over_budget` (`:423`) runs first with its own order. The cost is stated: on a full day syllabus cards take room ahead of event and commitment cards. The sum-of-15 case is written out with two variants: four held mail cards give exactly 15 in one run; ten give 18, then 15 after a same-day rerun snoozes three by `defer_over_budget`'s order. (r8 also claimed that one run charged no more than the 15 through sizing that already existed. That was false, since the filer was held to the 8 only, and variant (ii) itself charged 18; r9 withdraws the claim and sizes the filer to the 15, the r8 review's I-1. Variant (ii) is restated in r9.) P0 item 6 re-reads the two lines | T4d'a "Where" (:1520-1538); T4dt group (B) (:1132-1141); P0 item 6 (:952-954); §7 item 2 (:2113-2114) |
| M-1, found on the way | The day's 8 under a same-day rerun | Had the day's 8 counted `proposed_at`, each snooze would free a slot and every same-day slot would refill the 8. T4d now counts `first_proposed_at`, which `defer_over_budget` never rewrites (`approvals.rs:3724`). The mail hold writes `first_proposed_at` day 1 with `proposed_at` and `snooze_until` day 2 (`mail/tests/cards.rs:77`), so a card filed on day 1 counts toward day 2's 8. **Flagged for the reviewer:** it is new in r8. (r9: once the filer is held to the 15, a same-day refill cannot happen under either count, since a day `defer_over_budget` trims has no room left. The reason that stands is that counting `proposed_at` would charge a woken card to its new day's 8 as well.) | T4d "The day's 8" (:1253-1263); T4dt group (A) (:1116-1119); P0 item 6 (:949-951) |
| M-2 | "Closes a latent double approval" was overstated | Dropped. Q-B G11's note half now states its limit: `process_approvals` (`cli.rs:430`) materializes a twin approved before that desktop's `rank`, so a second task or `exam` commitment can still appear in that window. T4e is not widened. Round 7's row records the withdrawal | Q-B G11 (:481-485); Round 7's "G11's note half" row |
| M-3 | P2 (f)'s different-hash variant and (i)'s due case were underspecified | (f): the two extractions give the field two valid values and the note holds neither, so both cards carry it. The variant where the note holds one value turns on the random ids and is not pinned; point 3 allows either end. (i): the student's `due` goes through `write`, then a sync each way, then B's `rank`, then A's. A's `rank` never runs before that sync | P2 (f) (:1456-1461); (i) (:1484-1491) |
| M-4 | `Withdrawal`'s doc comment and the spec's T4e row describe the old count and reason | T4e rewrites the doc comment (`approvals.rs:1944-1946`) to name `withdraw_stale`, `withdraw_asks` and `withdraw_syllabus` and the counted rule; only comment lines change. Place 10 gains a tenth change: §4.6's "The move is `approvals.rs`'s" and §13's T4e row take a caller-built reason and the counted rule | T4e (:1308-1314); place 10 (:645-646, :678-684); §5 T4e row (:798) |
| — | Status | r8, naming `-r7.md`; delta review pending; the next report is `-r8` | status (:6-31) |

### Round 9, 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r8.md`, FAIL (0 critical, 1 important, 2 minor)

The review confirmed every r7 finding closed, the day's 8 counted by `first_proposed_at`, G8's
convergence and Done's reachability. The controller took the decisions below before r9 was written:
I-1 exactly as the review proposed, M-1 by stating item cards' `expires` and naming the snoozed
cards, M-2 by putting the cost on Quinn's list in both places. Each claim was checked against the
email branch's `cli.rs`, `approvals.rs`, `eventemit.rs`, `planning.rs` and `mail/`. Round 8's line
numbers were for the r8 text. These are for this text.

| # | Finding | Answer | Where |
|---|---|---|---|
| I-1 | The filer was held to the day's 8 only, so one run could charge 15 plus 8, and the hook line's stated reason was false | **The review's fix.** `plan` files at most min(what is left of the day's 8, `max(0, budget - count_proposals_created(vault, charge_day))`). The charge day is today, or day 2 when `commitments::vault_day` is 1. This is `emit_checks`' form (`cli.rs:873`), read at the hook's line (r10 corrects this: `emit_checks`' sizing counts today only, so the charge-day form is new, modelled on it; the r9 review's M-3). `read_inputs` reads the budget (`planning::load_planning`) and the count; `plan` stays pure. No syllabus card takes a day over the 15. `defer_over_budget` and its order are unchanged. Case (ii) is restated: 10 mail cards and an extraction on day 2 give 10 + 5 = 15 in one run, run once with the mail cards `pending` and once held from day 1, and a same-day rerun files and snoozes nothing. The held variant is kept as existing mail behaviour: held cards wake after `defer_over_budget` (`approvals.rs:1203`, `:1581-1602`) and the mail step does not size itself, so 18 held cards charge day 2 18 until a second run trims it, and the hook adds none. The two false claims are corrected in T4d'a and in Round 8's M-1 row, and the reason in T4d's day's 8 (a same-day refill) is corrected too. The spec's "anything over is snoozed" gains place 10's eleventh change. Every "within the day's 8" now means within this allowance | T4d "The 15" (:1336-1350), "The day's 8" (:1323-1335); T4dt (A) (:1154-1157), (B) (:1172-1211); T4d'a "Why this line" (:1616-1626), Done (:1673-1674); place 10 (:667-668, :707-716); G11's gap (:496-499); §5 T4d row (:828); P0 item 6 (:986-991); Round 8 rows M-1 and "found on the way" (:2536-2537); §9 (:2344) |
| M-1 | Case (ii) could not be written exactly: item cards had no stated `expires`, and the order was read backwards | **Stated.** The spec sets none, so r9 chooses: a task card expires on its `due` day and a `syllabus-exam` card on the exam's date, as an `event-accept` card expires on its event's date (`eventemit.rs:780-793`). Amend cards (due and grading) keep `null` (`write.rs:988`). The order is `defer_over_budget`'s real one: it sorts by `(expires, stem)` with `null` last, keeps the first `allowance` and snoozes the tail (`approvals.rs:1209-1223`). Under I-1's fix (ii) snoozes nothing, so the named snoozes are in (ii)'s held variant: the grading card and the two mail cards with the highest stems. The variant fails if item cards' `expires` were `null`. The edge is stated and reported to Quinn: an expired item card is history, so a later move of that item files nothing | Q-B `expires` (:522-536); T4d "`expires`" (:1351-1353); T4dt (A) (:1157), (B) held variant steps 1-4 (:1185-1211); §7 item 2 (:2222-2237); §9 (:2345) |
| M-2 | T4d'a said "reported to the controller", §7 put the cost on Quinn's list, and the cost was vague | Both now say Quinn's list. The cost is concrete: at the start of a term about 30 items take 8 a day for about four days, and an obligation event inside those days can get no card before it happens, because the events pass files one only while its start is today or later (`eventemit.rs:436-440`). Under I-1 the cost is capped at 8 a day and inside the 15. **New, for Quinn:** with the filer sized to the 15, a hook after the events block would also stay inside the 15, so the line is now only a priority choice (syllabus cards ahead of event cards). It is kept as decided and reported as a decision Quinn may reverse. The other side is also listed: syllabus cards wait behind every card charged before the hook | T4d'a "What it costs" and the two bullets after it (:1627-1645); §7 item 2 (:2222-2237); §9 (:2346) |
| — | Status | r9, naming `-r8.md`; delta review pending; the next report is `-r9` | status (:6-17, :18-26, :40) |

### Round 10, 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r9.md`, FAIL (0 critical, 1 important, 3 minor)

The review confirmed every r8 finding closed, the eleventh place-10 change, the 8 counted by
`first_proposed_at`, the held variant written exactly and Done's reachability. The controller took
the decisions below before r10 was written: I-1 as the review proposed, M-1 by keeping r9's default
and putting the alternative to Quinn, M-2 and M-3 as the review proposed. Each claim was checked
against the email branch's `cli.rs`, `approvals.rs`, `eventemit.rs`, `commitments.rs` and `mail/`.
Round 9's line numbers were for the r9 text. These are for this text.

| # | Finding | Answer | Where |
|---|---|---|---|
| I-1 | On the vault's first day the filer planned items due that day; held to day 2, they expired unseen there and took the first of day 2's 8 and 15 | **The review's fix.** The horizon and "never once past" count from the charge day, the one "The 15" uses: today, or day 2 when `commitments::vault_day` is 1. An item due on day 1 plans no card; one due on day 2 plans a held card with `expires` day 2, which `process_approvals` keeps `pending` when it wakes it (`approvals.rs:1616-1617` expires only `expires < today`). Day 1 then plans exactly what day 2's horizon holds. Q-B's reason ("the horizon never files a past item") is corrected to say what the day-2 floor guarantees. The spec's "of today" is read from the charge day by place 10's eleventh change, which already amends §4.6 :690-697, so no new place is added | T4d "The horizon" (:1384-1394); T4d "`expires`" (:1395-1398); T4dt (A) (:1186-1189); Q-B `expires` (:537-556); place 10 (:738-741); §7 item 2 (:2284-2286); §9 (:2409) |
| M-1 | Expired item history blocks the key; a cheaper alternative exists | **Kept as the default, alternative to Quinn.** §7 item 2 asks it in one line with the cost of each: the default loses a card for an item moved after its card expired unanswered; the alternative is a twelfth place-10 change, one predicate and one T4dt (C) case in `syllabus_cards.rs` (no contract file), a slot of the day's 8 and 15 per re-file, and item cards that differ from event cards | Q-B `expires` (:551-556); §7 item 2 (:2287-2299); §9 (:2410) |
| M-2 | `PlannedCard` did not name every field (A) asserts | `expires` (`Option<Date>`) and `first_proposed_at` are named | T4dt's interface (:1172-1173); §9 (:2411) |
| M-3 | Three claims overreached | (1) "No card only if every run in its 14 or 21 days is full" is bounded: only the runs from the day the item entered the horizon count, so a late-entering item (a syllabus added mid-term, a quiz two days out) has fewer, in T4d'a, T4d "The 15", §7 and place 10's spec text. (2) "As Q41 orders the 8" is replaced by `defer_over_budget`'s real order, with the held variant's snoozed grading card as the example. (3) The charge-day sizing is cited as new, modelled on `emit_checks`' (which counts today only), in the status, T4d "The 15", §9's r8 I-1 row and Round 9's I-1 row | T4d'a "The other side" (:1681-1687); T4d "The 15" (:1367-1383); place 10 (:731-736); Q-B `expires` (:547-551); §7 item 2 (:2279-2283); §9 (:2406, :2412); Round 9 I-1 row |
| — | Status | r10, naming `-r9.md`; delta review pending; the next report is `-r10` | status (:6-18, :19-29, :49-52) |

### Round 11 (r10a), 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r10.md`, PASS (0 critical, 0 important, 2 minor)

Both minors closed in place; no spec change (the spec's "within 14 days" already reads inclusive).

| # | Finding | Answer | Where |
|---|---|---|---|
| M-1 | Nothing pinned the horizon's far edge | **Controller ruling: inclusive.** Due on the charge day + 14 (an exam + 21) is inside; + 15 (+ 22) is not. T4dt (A) gains one far-edge case per kind; the day-1 case reads "day 16 plans a held card, day 17 does not" (charge day 2, + 14 = day 16) | T4dt (A) (:1196-1198, :1206-1207); §9's r9 I-1 row (:2443) |
| M-2 | No task named for the "N more dates from your syllabus" listing | **Already T4d's** (spec §13's T4d row; plan T4d *Behaviour*), carried by T17's `items`/`counts` and worded by T11b. Stated that `later` uses the charge-day horizon, so on day 1 an item due day 16 (exam 23) is a held card, not `later` | T4d *Behaviour* (:1359-1364); §9's r10 M-2 row (:2444) |
| M-2 follow-up | The listing had no `Plan` field, no case, and no named writer into the state file | **Controller go.** `Plan` gains `listing` (`ListedItem`: `source_uid`, `outcome`; `Outcome::code()` is spec §5's code); `PlannedCard`'s names unchanged. `Later` is a dated, not past item with no card, outside the charge-day horizon or inside it but not fitted by "The 15" this run. The writer: no task owned spec §5's per-item outcomes. T2c already owns the state file's writer, so T2c makes it one public function (outcomes in version 1, empty until a `rank`); the consumer of `listing` is the smallest task that runs where `Plan` exists, T4d'a, as its step 5 (about 10 lines, so about 50, under the 120-line stop rule), writing only when the bytes differ; T5 carries the outcomes through. Reported to Quinn as a reading: "one writer" is one function with three callers, not one command | T4dt's interface (:1179-1188); T2c (:1088-1091); T4dt (A) (:1212-1214), (B) (:1216-1219); T4d'a (:862, :1676, :1681, :1732-1736); T5 (:1857-1858); §9 (:2444) |
| — | Status | r10a note under r10; the reports list names `-r10.md` | status (:19-24, :57) |

### Round 12 (r10b), 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r10a.md`, FAIL (0 critical, 2 important, 4 minor)

The review found r10a sound on determinism, `surface`, `rank` and the references. The controller
ruled each finding below before r10b was written. Round 11's line numbers were for the r10a text;
the "twelfth" for the M-1 option in Round 10 and r10's status now reads "thirteenth".

| # | Finding | Answer | Where |
|---|---|---|---|
| I-1 | A slot's `rank` could write an old extraction back over one `syllabus-add` had just replaced | **The review's fix.** `syllabus::write_outcomes` re-reads the file, replaces only `item_outcomes`, and returns `Skipped` when the SHA or `added` differs from what `read_inputs` saw (or the file is gone or its `version` unknown); both writer forms write beside and rename over (`grades.rs:583-586`). One T4dt (B) case changes the file between `read_inputs` and the write | T2c "The one writer" (:1101-1116); T4d'a step 5 (:1770-1776); T4dt (B)'s changed-file case (:1251-1257); §9 (:2492) |
| I-2 | Version 1's codes had no `past` | `Outcome` gains `Past` (a dated item before the charge day, not `Filed`) with one T4dt (A) case. Spec §5's list (:880) gains `past` as place 10's twelfth change; the M-1 option is renumbered the thirteenth. §7 item 2 reports it to Quinn as a decision taken | T4dt's interface (:1214-1217) and (A) (:1243-1245); place 10's count (:698-700) and last bullet (:754-757); §7 item 2 (:2365-2367, :2375); §9 (:2493) |
| M-1 | `Filed` disagreed with G9 | `Filed` = planned this run, or held by a card that blocks the key under G9 | T4dt's interface (:1210-1212) |
| M-2 | The writer had no pinned name | `write_state` and `write_outcomes` are pinned in T2c with their signatures and `Written`'s three values; T4dt (B) asserts only `item_outcomes` changed; T1 pins `item_outcomes: []` on a fresh add; §5's T2c row updated | T2c (:1101-1116); T1 (:1069-1071); T4dt (B) (:1247-1250); §5 T2c row (:862) |
| M-3 | Same-day items had no order | Extraction order, then `source_uid` | T4dt's interface (:1207-1208) |
| M-4 | Outcomes could outlive their extraction | T5 writes `item_outcomes` `[]` when the SHA changes; one T5t case | T5 (:1899-1902); T5t (:1890-1892); §9 (:2494) |
| — | Status | r10b note; the reports list names `-r10a.md`; the next report is `-r10b` | status (:25-29, :62) |

### Round 13 (r10c), 2026-10-20: `docs/reports/2026-10-20-l5-plan-review-r10b.md`, PASS (0 critical, 0 important, 5 minor)

Each minor closed with the report's own fix, on the controller's word. Round 12's line numbers were for
the r10b text.

| # | Finding | Answer | Where |
|---|---|---|---|
| M-1 | Nothing repaired a state file that disagreed with its copy | In a `--syllabus` or `--only syllabus` run T5 treats a state whose SHA differs from its copy's as `pending`; one T5t case | T5 (:1916-1920); T5t's mismatch case (:1904-1906); §9 (:2518) |
| M-2 | Nothing stopped a seventh code reaching version 1 | `syllabus.rs` holds `ITEM_OUTCOME_CODES` (the six); `write_outcomes` refuses any other (`InvalidInput`, nothing written); T4dt (A) asserts every `Outcome::code()` is listed and the list has six | T2c "The one writer" (:1120-1122); T4dt (A) (:1254-1255); §9 (:2519) |
| M-3 | `past` was false on day 1 | T17 carries the code and date as written; T11b words a `past` item dated today as due today, never past; §7 item 2 says so | T17 (:1951-1954); T11b (:2237-2240); §7 item 2 (:2388-2390); §9 (:2520) |
| M-4 | An item could hold two codes | One code, the first that holds: `Filed`, `duplicate`, `feed-differs`, `needs-date`, `Past`, `Later`; one (A) case (a past item matching a task note is `duplicate`) | T4dt's interface (:1215-1217) and (A) (:1252-1254); §9 (:2521) |
| M-5 | Leftovers | r10's status line now says "thirteenth" for the M-1 option; undated items order by extraction order, then `source_uid`; T1 says the file's keys are sorted | status (:13); T4dt's interface (:1213-1215); T1 (:1073-1075); §9 (:2521) |
| — | Status | r10c note; the reports list names `-r10b.md` | status (:30-33, :66) |
