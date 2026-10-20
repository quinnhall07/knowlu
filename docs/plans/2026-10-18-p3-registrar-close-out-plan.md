# PR #19 close-out: the UA registrar merges before L5 (plan)

**Status:** PLAN, written 2026-10-18 by `planner`, not executed. For review at a checkpoint, then
Quinn's rulings in §8, asked one at a time. **Amended 2026-10-18:** A1's review findings folded into
phase B as B1a (A1-F2, ruling 11's actor) and B4a (A1-F1, the hold at withdrawal and settlement),
with §8 Q12; B4 gains its concrete fixture and call scope from the spec's D3 amendment.
**Ruling it serves (Quinn, 2026-10-18; row 30 of `.superpowers/sdd/2026-10-18-session-5/progress.md`):**
PR #19 (`p3-registrar`, the commitment model's phase 3, the UA registrar) **merges to `main` before
lane L5** (syllabus class times). It finishes its open items, is brought onto `main` after the email
branch merges, and L5 is re-planned around it. This reverses the order in the syllabus spec's §9,
which had L5 land a shared seam first and #19 rebase onto it.
**Governs:** the order of the remaining #19 work, who runs each piece, how #19 meets `main`, and the
text L5's spec needs. It designs no new feature; the one behaviour change it specifies (apply and
list, §3 O1) is owed by the signed cloud design.
**Authority, most recent first:** Quinn's ruling above; `docs/specs/2026-09-09-knowlu-cloud-design.md`
(Amendment 2026-09-29, ruling 3's consequence for the registrar at :563-572, ruling 11 at :634-663,
ruling 12's "the registrar never shares this profile" at :672-673); `VISION.md`; the phase-3 spec and
plan on the branch (`.claude/worktrees/p3-registrar/docs/specs/2026-09-26-commitment-model-phase3-design.md`,
`…/docs/plans/2026-09-26-commitment-model-phase3-plan.md`); the parent
`docs/specs/2026-09-23-commitment-model-design.md`; `HANDOFF.md` §3 "Pilot" and §4 "Open";
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5 (lanes and the conflict map).
Where the phase-3 spec disagrees with the cloud design's ruling 3, the cloud design wins.

## 1. Where #19 stands (facts, 2026-10-18)

- **Branch and head.** `p3-registrar`, worktree `.claude/worktrees/p3-registrar`, head `2ad1e3d`, 20
  commits over the p2 tip it was cut from (`eb10c1b` spec, `14b2aff` plan, `7d4dc51` plan review,
  `81c1e0f` merge of p2, then R1-R6 and the final review's fix wave `50530c0`, `b4788d0`, `204e4e7`,
  `e394c82`, `8de2c92`, `2ad1e3d`; from the worktree's reflog). Draft PR, opened stacked on
  `p2-commitments` (#16), which has since merged into `main`.
- **What it builds.** `engine/src/registrar.rs` (Banner parse, term rule, signed-out test,
  `commitments --registrar`); D5/D7 in `engine/src/commitments.rs` (per-term calendar key, the
  in-play keep-alive arm in `refresh_series`, `precedence` as tiers registrar 0 / `google:` 1 / else
  2, registrar series out of `instances_map`, the per-kind hold in `proposals`,
  `SERIES_KEY_PREFIXES` += `registrar:`, `asks_wait_for_registrar`, the series-file lock); `cli.rs`
  and `main.rs` wiring; `app/src/registrar.rs` (three console commands reusing `lms_link`'s
  throwaway window), `app/src/scaffold.rs`'s curated `Registrar` row for UA (label `myBama`, host,
  prefix, start URL, calls), `week.rs`'s `registrar_label`, `app/src/main.rs`'s console builder
  (manages `LmsSession`, wipes on the sign-in window's `Destroyed` and at exit, three handler
  lines); `console.js`, `console.css`, `index.html`, `static_assets.rs`, `wizard-check.py`;
  `site/privacy.html`, `CLAUDE.md`, `app/README.md`, `docs/surface/anatomy.md`. No cloud change, no
  migration, no slot step, no change to `app/src/commands.rs` (the integrator confirms each with
  `git diff --name-only main...p3-registrar`).
- **Last gate recorded:** full suite 2027 passed / 0 failed / 4 ignored at `2ad1e3d` (the
  commitment-model session's hand-off, 2026-09-29). Not re-run since; `main` has moved a long way.
- **The SDD ledger is gone.** The hand-off names
  `.claude/worktrees/p3-registrar/.superpowers/sdd/2026-09-26-commitment-model-phase3-plan/progress.md`;
  it is not on disk now, and no p3 review report is in `docs/reports/`. The hand-off says the fix
  wave through `2ad1e3d` was re-reviewed and passed; `HANDOFF.md` lists that review as open. This
  plan treats it as open (task A1).
- **Provisional by design:** the rows call (`UA_ROWS_PATH`), the call list, the headers and the
  fixture `engine/tests/fixtures/registrar/banner-ua-registration.json` (hand-written, invented, not
  frozen) wait on R0.

## 2. Checked against VISION and the two overriding rules

- **Commitment 5 (who started a change).** The registrar is an authoritative source: it applies and
  lists. The branch still files §5.4 change and end cards for registrar notes, which the cloud
  design's ruling 3 replaced. O1 fixes this before merge. "Judge once" still holds through
  `journal::human_edited`.
- **Commitment 4 (typed time).** Matched rows become `hard` class and lab notes (R24); registrar
  series stay out of the instance map so those notes block time by `meets` (R2-d).
- **Standing rules.** No password and no kept session: the school's own window, cookies read for the
  registrar host only, the window wiped after the run (§11a's "keep nothing", unchanged by ruling 12,
  whose kept Blackboard profile the registrar never uses). The fetched JSON lives in one temp file
  for one engine run. The series file stays local; only confirmed notes sync (P21). Failures are
  named, never silent; an empty parse is a failure.
- **Rule 1 (no single-user assumptions).** Two defects to fix at integration, one design point to
  keep: (a) the branch's engine CLI default `--actor quinn` (`engine/src/main.rs:91` on the branch)
  predates ruling 11; R24's notes must carry the vault's human token (`student` for a new vault,
  `quinn` for a legacy one). (b) The branch's `CLAUDE.md` hunk names `quinn` as the R24 actor. (c)
  UA stays a curated data row selected by the vault's `config/campus.yaml` `unitid`; nothing names a
  person's account. §7 R3 has the full check.
- **Rule 2 (frozen references).** None of the eight Python references or the three surface
  references holds a `registrar:` calendar; every task ends with the fixture byte check.
- **Fetch on device, think in the cloud.** The registrar makes no model call and no cloud call; the
  engine command makes no network call (`registrar_makes_no_network_call`).

## 3. Everything still open on #19

Each row says what it is, who does it and when it is done. Agents are from `CLAUDE.md`'s table.

| # | Item | Who, and why | Done when |
|---|---|---|---|
| O1 | **Spec revision to apply-and-list** (cloud ruling 3's consequence; `HANDOFF.md` §3 Pilot). The spec's §3 "Write (R24)" rows at :146-147, D5's end card, Plan ruling R2-g and Amendment 2 become: a fetch applies a changed `meets`, `where` or `until` to a confirmed registrar note and lists it; a dropped course gets `until` = the day before the fetch, applied and listed; a field the journal shows the student set by hand (`journal::human_edited`) is never overwritten, and that change is filed as a card instead (§8 Q2 picks which card). Behaviour in §4 task A2 | `planner` drafts it as a dated amendment section on the branch's spec (a design choice is open: the card, the listing surface, the actor); `reviewer` reviews; **Quinn signs** | The amendment is committed on `p3-registrar`, reviewed PASS, signed by Quinn, and B3's tests implement it |
| O2 | **R24.** Registrar rows whose course matches a vault course are confirmed `hard` without asking, written as the student (the vault's human token) via `dashboard`. The plan review answered the spec's §6 item 3 as "R24 as written"; that was the controller's ruling, not Quinn's | **Quinn**, asked by the controller with the context | Quinn's answer is a dated row in the close-out ledger and, if it changes R24, a line in O1's amendment |
| O3 | **G2-G5**, the session's guesses (away log `eb0a8cff…`): G2, plan problems 1-3 (one calendar key per term; a dropped course dated the day before the fetch; registrar series out of the instance map); G3, *Refresh from myBama* flips on 1 January and 1 June; G4, app code built before the spike on a provisional rows path; G5, myBama holds a course's class and lab per kind, and a Google class confirmed before the fetch stays beside it | **Quinn**, one at a time, each with its recommendation (§8 Q3) | Each is confirmed or undone in a dated ledger row; an undo becomes a task in this plan before B7 |
| O4 | **The review of `2ad1e3d`** (and, because the ledger is gone, the whole fix-wave tail `50530c0..2ad1e3d`) | `reviewer` (Opus, high; pre-push review) | A report `docs/reports/2026-10-18-p3-registrar-tail-review.md` with a verdict; any Critical or Important becomes a fix task. Its two findings so far: A1-F1 (the hold missing from withdrawal and settlement) is B4a; A1-F2 (ruling 11's actor) is B1a. Both run in phase B, since each needs `main`'s code (`human_or_vaults`, `journal::read_human_actor`) or sits on files B1 resolves |
| O5 | **The R0 spike**, shapes only (spec D3; plan §R0 steps 1-10) | **The controller with Quinn at the machine**, in Quinn's block; never a subagent (no subagent holds a session) | The D3 amendment is committed on the branch with the call list, the row shape and every answer in task A4; the fixture is confirmed or the shape recorded for B4; cleanup done |
| O6 | **The R0 follow-up**: `UA_ROWS_PATH`, the curated `calls`, any extra header (a Banner synchronizer token, a User-Agent), `parse_banner` and its four shape-bearing test bodies, the term table, and the fixture if its shape changed | `implementer` for code; **`contract-engineer`** writes the fixture and **`contract-reviewer`** reviews it (`engine/tests/fixtures/**` is a contract-list path; the rewrite is more than one screen) | One commit after B1; the registrar tests pass on the recorded shape; the re-plan trigger did not fire, or Quinn ruled on it |
| O7 | **Deferred minors**: m3 (the engine's exit-2 text reaches the page verbatim), m4 (a failed engine run wipes the window, so a retry needs a fresh Okta sign-in), m7 (change and end cards wait for the next slot's `rank`; dissolved by O1, which applies at the fetch), R2 m3 (a course dropped in fall and retaken in spring could get a swap card; dissolved by O1 if registrar keys take no swap card) | m3, m4: `implementer` (app) and `console-ui` (page); m7, R2 m3: closed by B3's tests | Each has a test or a written close in the ledger |
| O8 | **The privacy line** ("…your calendar link, your course list and, at schools Knowlu supports, your class schedule…") | The text goes to lane L4's honest page (§8 Q5); `docs-keeper` hands it over | #19's diff no longer touches `site/privacy.html`; L4's page carries the sentence, or the release guard in §5.5 holds |
| O9 | **Docs**: `docs/reference/engine-commands.md` (`commitments --registrar`), `docs/reference/app.md` (the three commands, recount by script), `docs/surface/anatomy.md`, `app/README.md`, `HANDOFF.md` | `docs-keeper` | Each names the shipped behaviour; the branch's `CLAUDE.md` hunk is dropped, and any `CLAUDE.md` sentence is proposed to Quinn (§8 Q7) |
| O10 | **The live proof** (plan's closing checklist) | **The controller with Quinn** signing in; dev build of `main` after the merge; scratch profile | A proof note in `docs/reports/` with counts and pass/fail lines only; the scratch profile cleaned up |

## 4. Tasks, in order

One ledger: `.superpowers/sdd/2026-10-18-p3-close-out/progress.md`, a dated row per event. Phase A
needs neither the email merge nor a green `main`; phase B starts only when the email branch has
merged and `main`'s CI is green (it is red until then on the ingest time bomb). No two tasks edit
one file at once; where two share a file the order below is the order they run.

### Phase A: on the branch, now

**A1. Review the fix-wave tail (O4).** `reviewer`. Read-only over `50530c0..2ad1e3d` against the
phase-3 spec and its two amendment sections. Focus: the per-kind hold (`8de2c92`) and its in-play
filter (`2ad1e3d`), the series-file lock (`b4788d0`), the confirm screen's added list (`204e4e7`).
*Done:* `docs/reports/2026-10-18-p3-registrar-tail-review.md` with a verdict; any Critical or
Important is fixed on the branch and re-reviewed before B1, except A1-F1 and A1-F2, which are
phase-B tasks B4a and B1a (they need `main`'s code), re-reviewed in B7.

**A2. Draft the apply-and-list amendment (O1).** `planner`, docs only, on the branch's spec, as
"Amendments (2026-10-18, close-out)". The behaviour it must state:
- **Where it applies.** In `commitments --registrar`, at the fetch, for each confirmed note keyed
  `registrar:<school>:<term>-<crn>` of the fetched term. Never in `rank`: `rank` files no §5.4
  change, end or swap card for a `registrar:` key, and Plan ruling R2-g's "every registrar calendar
  counts as fresh" goes.
- **What it writes.** Only `meets`, `where` and `until`, through `write`, journal first, one
  single-line frontmatter edit per field, as `agent:knowlu.registrar` via `dashboard` (the student
  pressed the button; the change is the registrar's). Never `level`, `kind`, `course` or the title.
  An agent actor matters: a write as the human would make every applied field read as hand-set
  under `human_edited`, and no later registrar change would apply.
- **A dropped course.** A row the fetch of its own term no longer returns sets the note's `until` to
  the day before the fetch (R2-c's date), applied and listed as ended.
- **Judge once.** A field `journal::human_edited` shows the student set after the note existed is
  not written; that change is filed as the card §8 Q2 picks, at most once per `(note, field, value)`.
- **The list.** The command's JSON gains `applied` (per note: field, from, to) and `ended`; the
  confirm screen's and the Schedule view's "From myBama" list shows added, changed and ended rows.
  The list a restart shows comes from the journal (the `agent:knowlu.registrar` records, the way
  `mailapplied.rs` on the email branch reads `agent:knowlu.mail`), inside `commitments --json`'s
  overview, so no new command and no new Tauri command.
- **Unchanged.** R24's create (§8 Q1), exit 2 writing nothing (applied writes included), the
  per-term key, the in-play rule, the per-kind hold, idempotence (a second identical fetch applies
  nothing and lists nothing new).
- **R0 additions** (A4's questions 6-7) and the human-token sentence (§2 rule 1 (a)).
*Done:* committed on the branch; `reviewer` (spec mode) PASS; Quinn signs (§8 Q2 with it).

**A3. Quinn's rulings (O2, O3).** The controller asks §8 Q1-Q3 one at a time, recommendation first.
*Done:* a dated ledger row per answer; an undo becomes a fix task scheduled before B7.

**A4. R0, the spike (O5).** The controller, with Quinn at the machine, in Quinn's block. The plan's
§R0 steps 1-10 stand, with three changes:
- **The build.** A dev copy of the branch as it is: it already has `open_registrar_window`, so
  step 3 may use it instead of `open_lms_window` plus `Page.navigate`. Either is fine; record which.
- **Two more questions** beside D3's five: (6) whether Banner's term list carries each term's start
  and end dates (settles G3); (7) whether the rows call pages (`totalCount`, `pageOffset`,
  `pageMaxSize` or similar) and whether a final-exam schedule is reachable (shape only; L5 §9's
  exams bullet). Paging matters under O1: a truncated answer would read as dropped courses.
- **The re-plan trigger stands:** a rows call with no per-meeting start and end dates stops the
  close-out for Quinn.
Shapes only: no CRN, course, room, name, cookie or token on disk, in the transcript or in a commit
(the term code is Banner's public identifier). *Done:* the D3 amendment committed on the branch;
the fixture confirmed byte for byte or its new shape recorded for B4; cleanup per plan step 9 (no
`knowlu-lms-session-*` folder left, scratch profile removed, the debugging variable unset).

### Phase B: after the email merge, `main` green

**B1. Bring #19 onto `main` (§5).** `integrator`. In `engine/src/main.rs` the merge takes `main`'s
`Commitments` arm (`actor: Option<String>`, resolved by `human_or_vaults`, at `main`'s :81-83 and
:494) and adds the branch's `registrar` and `school` fields to it; the `--registrar` branch of the
arm resolves its actor the same way, after reading the file and before any write (A1-F2, B1a). The
branch's `actor: String` with `default_value = "quinn"` (the branch's :92) is not kept, and no
`.unwrap_or("quinn")` or similar stands in for it. The doc comment at the branch's
`app/src/registrar.rs:48` ("`--actor` keeps the engine's default, `quinn`") becomes "the engine
writes R24's notes as the vault's human token" in the same commit. *Done:* the branch carries
`main`; the workspace gate is green with 0 other warnings; the fixture byte check passes; the PR's
base is `main`.

**B1a. The registrar writes as the vault's human token (A1-F2, ruling 11).** Right after B1, before
B4. *Finding:* on the branch `commitments --registrar` writes R24's notes as the CLI default
`quinn` (the branch's `engine/src/main.rs:92`, `actor: String`), and its new tests hardcode
`WriteContext::new("quinn", "dashboard")` (the branch's `engine/src/registrar.rs` test module, the
`run` helper at :657 and the calls at :682 and :716). In a new vault, whose token is `student`, that
puts a second human token into the journal (rule 1; cloud design ruling 11). *Ruled decisions kept:*
R24 still confirms matched rows `hard` as the student via `dashboard` (§8 Q1, ruled); applied
changes stay `agent:knowlu.registrar` (A2's amendment). *Who:* `test-writer` (Sonnet, medium) for
the tests; any source line the tests show wrong goes back to `integrator`, since `main.rs` is B1's
file. Files: the registrar's test module in `engine/src/registrar.rs` (test code only) and the engine
binary test file that already drives `commitments` through the built executable (the test-writer
names it in the ledger; a new `engine/tests/commitments_registrar_actor.rs` if none does).
*Tests* (all invented data; each proves itself by failing once against a local, uncommitted edit
that passes a fixed `"quinn"` to the registrar arm, reverted at once, recorded in the ledger row):
- a vault whose `config/actor.yaml` says `student`: `knowlu-engine commitments --registrar <fixture>
  --school ua` with no `--actor` writes every R24 record as `student`, and no journal record of the
  run carries `quinn`;
- a legacy vault with no `config/actor.yaml`: the same run writes as `quinn` (the legacy token);
- a vault whose `config/actor.yaml` holds a bad value: exit 2, the reader's named line on stderr,
  the vault's bytes and journal unchanged (exit 2 writes nothing, as the spec requires);
- an explicit `--actor` passes through unchanged and `write`'s gate still judges it;
- the registrar test module's helpers take the actor from `journal::read_human_actor(&vault)`, and
  at least one of them runs on a vault seeded with `config/actor.yaml: student`, so a library-level
  R24 test sees the new-vault path too. No `WriteContext::new("quinn", …)` is left in the
  registrar's tests.
*Done:* the tests are green on B1's tree and the mutation check is recorded; the bare-literal test
passes; `git grep -n '"quinn"' -- engine/src/registrar.rs app/src/registrar.rs` shows no line
outside a test that names the legacy token on purpose; the workspace gate is green.

**B2. Failing tests for apply-and-list.** `test-writer`, in `engine/tests/commitments_registrar.rs`
and the registrar's test modules, all invented data. At least:
- a refetch with a moved time applies `meets`, lists it, files no card, and writes as
  `agent:knowlu.registrar`;
- a refetch without a row sets `until` to the day before the fetch and lists it as ended;
- a field the student set by hand after the create is not written and gets one card; a second
  identical fetch files no second card;
- `level`, `kind`, `course` and the title are never written;
- an identical refetch applies nothing;
- exit 2 writes nothing, applied writes included;
- `rank` over a vault with a changed registrar series files no change, end or swap card for a
  `registrar:` key, and still files them for `gcal-series:` and `ics-series:` keys;
- a fall course retaken in spring gets a new R24 note and no swap card (R2 m3);
- (A1-F1, end to end, from B4a) `rank` files a card for a Google class one minute off the
  registrar's times; `commitments --registrar` runs; the card is approved before the next `rank`:
  no second class note for the course, the card stamped `superseded`; and without the approval,
  the next `rank` archives it;
- R24's create carries `student` in a new vault and `quinn` in a vault without `config/actor.yaml`;
- every test in every group under the spec's A2 item 20. There are five; extract each from the
  spec by its heading under item 20, never by line number: "The tests review round 1 adds to B2",
  "Round 2 adds", "Convergence round 1 adds", "Convergence round 3 adds" and "Convergence round 5
  adds". A case a later round added inside an earlier round's test is part of that test (round 3's
  second case of "Agreement clears it" and its addition to "The same with a hand-set `until`";
  round 4's replaced-value case inside "A `rank` does not withdraw a fresh card"). If the spec gains
  a sixth group before B2 starts, B2 takes it too and the controller adds it here:
  - round 1: a held card is not lost, the student's override is not proposed back, no prior row,
    agreement clears it (both cases), change cards on day 1, the list survives a restart from the
    registrar block, the block survives `rank`, idempotence read with 12, the dropped course with a
    hand-set `until`, the status rule, "over" is not "in play";
  - round 2: ended once across days, ended once with the block lost, the CRN comes back, seen is
    not missing (P9), `was` is read at filing, expired reopens, one filer (`SERIES_LOCK_FILE`), the
    block's bytes (and no `registrar` key in a vault that never fetched), not watched stays
    unwatched, the next term;
  - convergence round 1: dropped, back, dropped again (and the same with a hand-set `until`, with
    round 3's withdrawn day-1 and day-20 cards), a come-back that is not whole, the merge keeps the
    last whole row (P9's carry, with its `unread` and `status` variants), a partial first read
    creates nothing;
  - convergence round 3: a replaced value withdraws its card, a block that is gone withdraws
    nothing, a `rank` does not withdraw a fresh card (with round 4's replaced-value case), a dropped
    elective is not carried (with its `partial` and `unread` variants), a confirmed note is still
    carried;
  - convergence round 5: a `rank` does not refile a dropped elective's card (its `partial` and
    `unread` variants and its two controls), approving a dropped section's card writes nothing
    (with its "no longer holds X's term" and later-`rank` cases, and its three never-stamped
    controls).
- Each test names, in a comment, the B3 sub-task (B3a-B3f, as each sub-task's *Tests* line
  assigns it) whose behaviour it pins, so each sub-task's *Done* can name its tests.
(The paging bullet is dropped: R0 found the rows call is not paged; spec A2 item 19.) B2 runs on
the recorded-shape fixture B4 leaves, and builds its `partial`, `unread` and `status` rows with
B4's `seen`, never by deleting a series. A finals "is listed" test joins only if Quinn picks §8 Q10
(b).
The existing `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank` is replaced
by its apply-and-list counterpart in this task, by name, with the reason in the commit message.
*Done:* the new tests fail for the stated reason; nothing else fails.

**B3. Build apply-and-list, and the per-source seam: six sub-tasks, B3a to B3f** (split at spec
convergence round 1; one task was far past a task's size). Every sub-task is `integrator` (Opus,
high). *Why that agent:* each writes as an agent into confirmed hard notes or into the series file's
bytes, so a silent error moves or ends a class that is still meeting, or breaks `vault-full`'s
bytes; that is Opus work, and `commitments.rs` is not on the contract list. *Shared rules:*
- **Order and files.** They share `engine/src/registrar.rs` (its `run` and report part),
  `engine/src/commitments.rs` and `engine/src/cli.rs`, so they run **strictly in order**, one commit
  each, never in parallel.
- **Size.** About 80 lines of source edit each, tests excluded. Past about 120, the integrator
  stops and the controller splits the sub-task again.
- **Behaviour and tests.** Behaviour is the spec section named, extracted by heading. Each makes
  green the B2 tests that name it, and only those; the other B2 tests stay red for their stated
  reason.
- **Contract files.** A needed change to `write.rs`, `journal.rs` (a new `VIAS` value included) or
  `approvals.rs` moves that piece to `contract-engineer` (xhigh) and `contract-reviewer`, and
  `approvals.rs` waits for L5's owner (§5.3).
- **Done, for each.** Its B2 tests green, the workspace gate green, the frozen references and the
  three surface references byte-identical, and `rank`'s output over `vault-full` unchanged.
- **Review checkpoints.** `reviewer` reviews after B3c (the block's bytes) and after B3e (the card
  filing), each into `docs/reports/`, before the next sub-task starts.

**B3a. "Missing" from `seen`, and the merge carry.** Spec: D3 amendment P9 (with "The merge keeps
what it could not read"); A2 items 10 ("A row read only in part is not created") and 11 (the first
paragraph and "A row the parser saw but could not read").
- **Missing.** `run` reads B4's `seen`: a confirmed note is missing only when its CRN is absent
  from `seen`, never because its row formed no series. This sub-task lists the missing note only;
  B3d writes the end.
- **The merge carry.** The input `run` passes to `refresh_series` is the `whole` rows plus, for
  each `partial`, `unread` or `status` CRN **whose key a confirmed note holds**, its previous row,
  carried unchanged; with no previous row, nothing. A CRN no confirmed note holds is not carried,
  so `refresh_series` moves it to `ended` (narrowed at spec convergence round 3, "With no confirmed
  note, nothing is carried"); withdrawing and refusing its proposal card is B3e's.
- **No create from a part read.** R24 creates only from `whole` rows.
- **Unread.** A `partial` or `unread` note is left unchanged and named for the report's `unread`
  key (B3c stores it).
- **G3's two functions** (A2 item 15, "When the button flips" and "Which term it fetches then"):
  the `term_over` test beside `registrar::status` and the next-term function, in
  `engine/src/registrar.rs`. They are small and in this sub-task's file. If B3a passes its size,
  they are the piece that moves out into a B3a' of their own.
- *Files:* `engine/src/registrar.rs`; `commitments.rs` only if `refresh_series` needs a reader for
  the previous rows.
- *Tests:* seen is not missing; the merge keeps the last whole row (all three variants); a
  confirmed note is still carried; a partial first read creates nothing; the status rule (P6); over
  is not in play; the next term.

**B3b. Apply and list, and `rank` stops carding the registrar.** Spec: A2 items 8 and 9.
- **Apply.** A changed `meets`, `where` or `until` on a `whole` row is written field by field through
  `write`, journal first, as `agent:knowlu.registrar` via `dashboard`. Nothing is written on a
  hand-set field yet: B3e records those.
- **`rank`.** It files no §5.4 change, end or swap card for a `registrar:` key from
  `detect_changes`; Plan ruling R2-g goes. `gcal-series:` and `ics-series:` cards are unchanged.
- **The replaced test.** `a_registrar_refetch_files_an_end_card_and_a_change_card_at_the_next_rank`
  goes by name in this commit (B2 wrote its successor).
- *Files:* `engine/src/registrar.rs`, `engine/src/commitments.rs`, `engine/src/cli.rs`.
- *Tests:* B2's moved-time, never-written-fields, identical-refetch, exit-2 and `rank`-files-no-
  registrar-card bullets, and the retaken-course bullet.

**B3c. The registrar block, its bytes and the report keys.** Spec: A2 item 13 (all of it,
"The registrar block", "How it changes", "Its bytes", "When it is written", "Every writer keeps
it", "After a restart").
- **The block.** The top-level `registrar` object, per term, written in the merge's own series-file
  write under the lock. It goes into the one value `file_bytes` builds, so `ledger::dumps_value`
  writes it: object keys in that function's sorted order at every level (top level `calendars`,
  `ended`, `registrar`, `series`), list entries in the spec's fixed order, sorted by `file_bytes`
  before the write. Nothing serialises the block on its own path, and `ledger.rs` (contract list)
  does not change. It is omitted when no term has a block.
- **Every writer keeps it.** `refresh_series` carries the block unchanged and drops an aged-out
  term's entry.
- **The report and the overview.** The command's JSON gains `applied`, `ended`, `kept`, `status`,
  `unread` and the new `dropped` counts. `commitments --json`'s overview reads the block.
- **What this sub-task stores.** `applied`, `status` and `unread` entries. `ended` and `kept`
  entries arrive with B3d and B3e; their keys exist now as `[]`.
- *Files:* `engine/src/commitments.rs`, `engine/src/registrar.rs`.
- *Tests:* the block's bytes; the block is kept; the list survives a restart (its `applied`,
  `status` and `unread` part); idempotence (its `applied` part).
- *Extra Done:* `state/calendar-series.json` over `vault-full` is byte-identical to `main`'s.

**B3d. The end, once, and the come-back.** Spec: A2 item 11 ("Ended once", "When the CRN comes
back", "When the student set `until` by hand").
- **The end.** A missing note gets `until` = the day before the first fetch that found it missing
  since it last came back, written as in B3b. An open `ended` entry is recorded.
- **Already ended.** The test is the note's **open** `ended` entry, or the journal fallback when
  the block is absent; a closed entry never counts.
- **The come-back.** A `whole` come-back closes the open entry with `returned` (written or not)
  and is compared as in B3b. A come-back that is not whole closes nothing.
- **A hand-set `until`.** It is listed "ended by myBama, kept yours"; its kept entry and card are
  B3e's.
- *Files:* `engine/src/registrar.rs`, `engine/src/commitments.rs`.
- *Tests:* ended once across days; ended once with the block lost; the CRN comes back; dropped,
  back, dropped again (the hand-set half stays red until B3e); a come-back that is not whole; the
  list survives a restart (its `ended` part).

**B3e. Kept fields, the card, the cap and one filer.** Spec: A2 item 12 (all of it).
- **Detection.** At the fetch, `prior` comes from the series file before the merge (B3a's carry
  makes it the last whole reading). For `until` there are three cases: missing, already ended and
  come-back. They read the open `ended` entry.
- **Kept entries.** They are recorded and removed by 12's rules. A kept entry with no prior row is
  listed only.
- **Filing.** Each kept entry with a card is offered to `emit_checks` as a change, at the fetch and
  at every `rank`, with `was` read from the note at filing.
- **The cap and asked-once.** Both are as `main`'s `emit_checks` does after B1; cite it in the
  commit.
- **One filer.** `SERIES_LOCK_FILE` is held from the asked-once read to the last card write, and
  `rank` takes it after `refresh_series` releases it. *Check first:* the spec says a held lock
  makes the process wait, but the branch's `refresh_series` doc says a lock it cannot take is one
  warning and the run goes on unguarded. The integrator reads `lock_series_file` and reports which
  it does before building. If it does not wait, the controller takes that to `reviewer` (spec
  mode), because an unguarded filer breaks "one card".
- **Withdrawn with its entry** (12, "Withdrawn with its entry", convergence rounds 3-4). A pending
  or snoozed kept-field change card on a `registrar:` note is stamped `superseded` when its term's
  block holds no standing kept entry with its target and canonical change; a term with no block
  withdraws nothing. The test sits in `withdrawal_reason`'s change arm after `stale_change`. It
  runs at the fetch's withdrawal pass (after the series-file write, under the lock, over the
  re-read) and at `rank`'s second call under the lock; the I5 call stays where it is with the
  block emptied. The commit names the three call sites. `settle_approved`'s change arm makes the
  same test on the file on disk and returns `Settled::Superseded` for a card whose entry is gone.
- **The `asked` drop** (12, "A second call, under the lock", convergence round 5). In `cli.rs`'s
  `commitment_passes`, the proposals `rank` passes to `emit_checks` under the lock lose every one
  whose key starts `registrar:` and is not under `series` in the re-read. Nothing else in the
  proposals changes; `withdraw_asks` and `emit_asks` read them as today. On a malformed or
  unreadable re-read, the second call is skipped and no `registrar:` proposal and no kept entry is
  filed.
- **The proposal arm at settlement** (12, "And a `registrar:` proposal card", convergence round
  5). `settle_approved`'s proposal arm, after its confirmed key-or-signature test and before
  `create_confirmed` (beside B4a's hold), reads the series file on disk; a card keyed `registrar:`
  whose key is in that file's `ended`, or under no calendar of a file that knows one, gets
  `Settled::Superseded` (the reason, then "; nothing written"). A file that reads as empty says
  nothing; a card keyed outside `registrar:` is not tested. The stamp is fixed by the spec and does
  not wait on §8 Q12. Built in `commitments.rs`; `approvals.rs` does not change.
- *Size.* With the three bullets above, B3e is the sub-task most likely to pass about 120 lines. If
  it does, the split line is here: B3e keeps detection, kept entries, filing and one filer; a B3e'
  takes the withdrawal, the `asked` drop and both settlement tests, and runs before B3f. The
  reviewer checkpoint after B3e then moves to after B3e', and the round-3 and round-5 tests in the
  *Tests* line below, which pin the withdrawal and settlement, move with it.
- *Files:* `engine/src/commitments.rs`, `engine/src/cli.rs`, `engine/src/registrar.rs`.
- *Tests:* a held card is not lost; the override is not proposed back; no prior row; agreement
  clears it (both cases); change cards on day 1; the dropped course with a hand-set `until`; `was`
  is read at filing; expired reopens; one filer; dropped, back, dropped again (its hand-set half,
  with round 3's withdrawn cards); the rest of the restart and idempotence tests; from convergence
  round 3, a replaced value withdraws its card, a block that is gone withdraws nothing, a `rank`
  does not withdraw a fresh card (with round 4's replaced-value case), a dropped elective is not
  carried; from convergence round 5, a `rank` does not refile a dropped elective's card, approving
  a dropped section's card writes nothing.

**B3f. The per-source table (§8 Q4, recommended).** Spec: A2 item 16 (its table and "No output
change").
- **The table.** The prefix tests now in closures in `refresh_series`, `proposals`, `precedence`,
  `instances_map` and `SERIES_KEY_PREFIXES` fold into one table in `commitments.rs` with **two key
  columns**: the calendar prefix and the `source_uid` prefix. Each lookup names its column.
- **Rows.** As the spec's table, the "not watched" row included. The integrator checks the
  keep-alive and instances cells against the code and corrects the spec's table in its commit if a
  cell differs, never the code to match the text.
- **B4a's hold** moves into the table as its "holds per kind" column.
- **What L5 adds then.** One `syllabus:` row and its two arms.
- *Files:* `engine/src/commitments.rs`.
- *Tests:* not watched stays unwatched; every B2 and B4a test still green, with no output change.

**B4. The R0 follow-up (O6). Runs right after B1, before B2** (moved at spec review round 1, so
every apply-and-list test runs on the recorded shape; B4 and B3 both edit `engine/src/registrar.rs`,
and this order keeps them sequential). Three steps, per the spec's A2 item 19: (1)
`contract-engineer` (Opus, xhigh) rewrites the fixture to the recorded shape, and
`contract-reviewer` reviews that diff before step 2 starts. The fixture sits under
`engine/tests/fixtures/**`, a contract-list path, and the rewrite is several screens, more than the
main session edits; never `test-writer` or `implementer`. (2) `test-writer` writes the parse rules'
failing tests, P1-P6, P9's `seen` and P3's finals rule (a final adds no weekday, moves no date, is
counted in `dropped.exam`); (3) `implementer` (Sonnet, high) makes them pass. B4 owns what
`parse_banner` returns, including the `dropped` counts, P6's registered set (empty until B9a) and
**P9's `seen`**: every `courseReferenceNumber` in `data.registrations` with its reading. The
readings are `whole`, `partial` (a `CLAS` meeting skipped under P4, or a meeting skipped under P3's
unknown-type rule), `unread` (no series) and `status` (P6). B4's `seen` tests, red first:
- a row whose only `CLAS` meeting has null times is in `seen` as `unread` and forms no series;
- a row with only unknown-type meetings is `unread`;
- a row with one usable and one timeless `CLAS` meeting is `partial`, with a series from the usable
  one;
- a row under a non-registered status (with an invented non-empty set) is `status`;
- an `EXAM` meeting beside a `CLAS` one leaves the row `whole`;
- every CRN of the answer is in `seen` exactly once.
B3 owns the run, P9's merge carry, the report keys and the registrar block, and reads "missing"
only from `seen` (B3a), never from the absence of a series. Files: `engine/src/registrar.rs`
(`parse_banner`, `looks_signed_out`, `SCHOOLS`' term suffixes), the four shape-bearing test bodies
(plan "Order and file ownership"), `app/src/scaffold.rs` (the UA row's `calls` and any header),
`app/src/registrar.rs` (the headers sent; the redirect rule, since `ureq` drops `cookie` on a
redirect; m3's app half, an engine exit 2 returned as a fixed sentence, never its text),
`app/tests/registrar.rs`. A4 found the flat `meetingTimes[]` shape, so the fixture is rewritten
(spec D3 amendment, P1-P9). Every expected value in the four shape-bearing test bodies stays. B4
also keeps the assertions that pin UA's real registrar entry in as few app tests as it can and
names them in its ledger row (spec A2 item 18), and writes the key-set test as "every curated
`registrar.school` is a key of `SCHOOLS`". *Done:* B4's commits, registrar tests green on the
recorded shape, the UA-pinning tests named in the ledger.

*B4's concrete scope: what differs between the branch and the recorded shape* (spec D3 amendment,
"The rows call's shape" and P1-P9; every value below that is not a key name or a seen enum is
invented, as the fixture's are now):

| Where | The branch now | The recorded shape, which B4 moves to |
|---|---|---|
| Fixture, top level | `success: true`, `totalCount: 6`, `data` an array of rows | No `success`, no `totalCount`. `data` is an **object**: `maxHours`, `minHours`, `totalBill`, `totalCeu`, `totalCredit` (strings) and `registrations[]`, which holds the rows |
| Fixture, each row | `termDesc` | `termDescription` |
| Fixture, each row | `scheduleTypeDescription` (Lecture, Laboratory, Online, Studio) | **No `scheduleTypeDescription` and no `scheduleType`.** The row carries `scheduleDescription`; the lab row keeps its expected `lab` kind through P5's rule (`scheduleDescription` naming "Laboratory" as a whole word), the rest stay `class` |
| Fixture, each row's meetings | `meetingsFaculty[]`, each wrapping a `meetingTime` object | **Flat `meetingTimes[]`**, no wrapper; each meeting gains `meetingType` (`CLAS`), `meetingTypeDescription` and `meetingScheduleType` (from the seen set: `LEC`, `LLB`, `RCT`, `SEM`); times, dates, days, `building`, `buildingDescription` and `room` keep their names and formats (P8) |
| Fixture, each row | no status | `statusIndicator` and `statusDescription`, invented codes (the registered set is empty until B9a, so every row counts: P6) |
| Fixture, never-read keys | none | Invented `faculty[]`, `instructorNames[]` and `properties` on at least one row, so P7's test can assert none of them reaches a series, a warning, an error string or stdout |
| Fixture, finals | none | One `EXAM` meeting beside one row's `CLAS` meeting, on a weekday that row does not meet and dated outside its class dates, so a folded final would change `meets`, `first` or `until`; the four shape-bearing bodies' expected values staying the same is part of P3's proof |
| `app/src/scaffold.rs` | `UA_ROWS_PATH = "classRegistration/getRegistrationEvents?termFilter="` (the branch's :70); `calls` = the `term/search?mode=registration` POST then that GET | `UA_ROWS_PATH = "registrationHistory/reset?term={term}"`; `calls` holds that one GET only (the POST goes: Q2, no selection call). The term-list call is not added (the D3 amendment recommends it out; adding it is a design change for Quinn) |
| `app/src/registrar.rs` headers | as built | Unchanged on the recorded shape (cookies for the registrar host, `accept: application/json`, `x-requested-with: XMLHttpRequest`; no synchronizer token, no User-Agent); a redirect still reads as signed out |
| `parse_banner` | reads `data[]`, `termDesc`, `scheduleTypeDescription`/`scheduleType`, the `meetingsFaculty`/`meetingTime` wrapper | Reads `data.registrations` (absent: exit 2 "not a list of registration rows"; present but empty: the empty parse, exit 2, P1), `termDescription`, the flat `meetingTimes[]` (P2), `CLAS` meetings only (P3), P4's no-time rule, P5's kind rule, P6's status rule, and returns P9's `seen` |

The edge cases that would change the main fixture's expected values (an unknown `meetingType`, a
`CLAS` meeting without times beside a timed one, a row whose meetings are all `EXAM`, a row outside
a non-empty registered set, an empty `registrations`, a `data` with no `registrations`) are
invented JSON inline in step 2's tests, or a second small file under
`engine/tests/fixtures/registrar/`, never added to the main fixture. *Who writes the fixture:*
`contract-engineer`, with `contract-reviewer` reviewing the diff (convergence round 2); a second
small file under `engine/tests/fixtures/registrar/` is written and reviewed the same way. Never
`test-writer` or `implementer`. The file keeps its current line endings byte for byte
apart from the content change (`engine/tests/fixtures/**` is `-text`), and no other file under
`engine/tests/fixtures/` is opened for writing (the byte check in §5.2 rule 3). The review checks
those line endings, that every row follows the recorded shape and not the new parser's output, and
that every value is invented; its report lands in `docs/reports/`. *Extra tests for
the call list:* an app test that the UA row's only call is a `GET` whose URL ends
`registrationHistory/reset?term=<the chosen term>` with `{term}` filled in, and that no `POST` is
sent.

**B4a. The registrar hold withdraws a filed card and refuses its approval (A1-F1).** Runs beside
B4 (disjoint files) and before B2 and B3, so B3's per-source table folds in the fixed hold. If any
of B4's four shape-bearing test bodies sits in `engine/src/commitments.rs`, B4a waits for B4.
*Finding:* the final review's I1 fix (the per-kind hold) lives only in `proposals()` (the branch's
`engine/src/commitments.rs`, the `registrar_held` set at about :2414 and its check at about
:2458). `withdrawal_reason`'s `Shape::Proposal` arm (about :4443-4472) tests only "left the
calendar", "ended", "has finished", "was answered" and "already confirmed" (key or signature), and
`settle_approved`'s `Shape::Proposal` arm (about :4341-4369) refuses only on a confirmed note with
the card's key or signature. So a `commitment-check` card already filed for a Google class whose
times are one minute off the registrar's (a different signature, so neither test fires) stays
pending after a registrar fetch, and approving it writes the second hard class note I1 exists to
prevent. Approval can land before the next `rank`'s `withdraw_stale`, and in `rank` itself
`process_approvals` settles before `withdraw_stale` runs (the branch's `cli.rs` about :796-826), so
the withdrawal alone is not enough: both arms change.
*Behaviour:*
- **Withdrawal.** `withdrawal_reason` returns a reason for every condition that stops `proposals()`
  proposing the card's key, the registrar hold included: a card whose key is not a `registrar:`
  key, whose `commitment:` is `class` or `lab` with a course, is withdrawn when the registrar holds
  that `(kind, course)`, by a confirmed `registrar:` note of that kind in play (no `until`, or not
  before today) or by a `registrar:` series of that kind in the stored series file not past its
  `until`. Same course key (`course_key`) and same per-kind rule as `proposals()`, so a registrar
  lecture never withdraws a Google lab of the course. `withdraw_stale` is unchanged (it already
  stamps `superseded` and archives whatever `withdrawal_reason` names).
- **Settlement.** `settle_approved`'s proposal arm checks the same hold before `create_confirmed`
  and returns `Settled::Superseded("<course>'s <kind> is held by the registrar; nothing written")`
  (§8 Q12; recommended over `Refused` so the question can be asked again if the registrar's note
  later ends; *ruled `superseded` 2026-10-19, Quinn*). Nothing is written but the card's stamp.
- **One predicate.** The hold is computed in one place that `proposals()`, `withdrawal_reason` and
  `settle_approved` all call, so the three cannot drift again; B3's per-source table then hosts it.
- **Parity audit.** The integrator lists every condition under which `proposals()` skips a key (a
  past `until`, an answered key, a key in P12's `held`, a signature closed by a confirmed note or by
  an answered twin, `classify` returning nothing, the registrar hold) beside whether
  `withdrawal_reason` covers it. Each gap is either closed here with its own test or written in the
  ledger row with the reason it stays (P12's successor keys, say, have their own rule). Any gap
  closed beyond the hold is listed for B7.
*Who:* `test-writer` writes the failing tests in `engine/src/commitments.rs`'s test module;
`integrator` (Opus, high) makes them pass in `engine/src/commitments.rs` only. *Why that agent:* a
wrong hold either writes a second hard class note or withdraws a question the student still owes
an answer, silently, in confirmed time; and B3, which folds it into the table, is the integrator's.
`approvals.rs` is not edited (contract list, and L5's while L5 runs); if the fix turns out to need
it, that piece moves to `contract-engineer` and `contract-reviewer`.
*Tests, red first* (invented data; each fails on the branch for the stated reason):
- a pending Google `class` card for course C, then a registrar `class` series for C in the series
  file meeting one minute later: `withdrawal_reason` names the hold, and `withdraw_stale` archives
  the card as `superseded` (red: the card stays pending);
- the same with the hold coming from a confirmed in-play `registrar:` class note and no registrar
  series: withdrawn;
- a registrar `class` hold leaves a pending Google `lab` card of C pending;
- a confirmed `registrar:` note whose `until` is before today holds nothing: the card stays;
- a card keyed `registrar:` is never withdrawn by the hold;
- approving the Google `class` card while the registrar holds C: `Settled::Superseded`, no note
  created, no `create` record in the journal, one confirmed class note for C (red: two);
- after that, the next `rank` files no new card for that key and charges nothing against the cap
  (`withdraw_stale`'s "no cap-charge loop" holds for the new condition);
- the parity audit's closed gaps, one test each.
The end-to-end version (`rank` files the Google card, `commitments --registrar` runs on the
recorded-shape fixture, the card is approved before the next `rank`, then `rank`) joins B2's list
in B2's file, after B4.
*Done:* the tests failed red first, then green; the audit is in the ledger row; the workspace gate
is green; the frozen references and the three surface references byte-identical; `rank`'s output
over `vault-full` unchanged; `git diff --name-only` for the task shows `engine/src/commitments.rs`
only.

**B5. The page.** `console-ui`, in its queue slot (§5.4). Files: `app/static/console.js`,
`console.css`, `index.html`, `app/tests/static_assets.rs`, `scripts/wizard-check.py`. Behaviour:
the "From myBama" list shows changed, ended, kept ("kept yours", card filed or waiting) and status
rows beside added ones, from the overview's registrar block (spec A2 item 13); m3's fixed
words (the page never shows engine text verbatim; the app maps exit 2 to a fixed sentence);
m4 is closed by keeping Plan ruling R4-d (once bytes reach the engine the session is thrown away,
whatever the engine says) and saying on the page that a retry needs a fresh sign-in; keeping the
window open longer would keep a school session alive for a convenience. Quinn words any new copy. *Done:* static assertions and `wizard-check.py`
green; screenshots by fixture (memory "console fixture screenshots") attached to the ledger.

**B6. Docs (O8, O9).** `docs-keeper`. `docs/reference/engine-commands.md` (the `--registrar`
contract: no network, exit 0 or 2, nothing written on 2, the applied writes and their actor);
`docs/reference/app.md` (the three commands; counts by the plan's R6 Step 1 script, never by
hand); `docs/surface/anatomy.md` (the confirm screen and Schedule rows); `app/README.md`;
`HANDOFF.md` §3 Pilot (the registrar row closed) and §4 (the R0 row closed). **If Quinn picked §8
Q10 (a)**, `HANDOFF.md` §4 also gains the named-deferral row the spec's D3 amendment gives
("registrar `EXAM` meetings: skipped and counted; a later lane decides (b) or (c)"), since no other
lane owns registrar finals (L5's amended §9 designs no finals rule of its own). Under (b) or (c)
that row is not written. Drop the branch's
`CLAUDE.md` and `site/privacy.html` hunks; hand the privacy sentence to L4 (§8 Q5); propose any
`CLAUDE.md` sentence to Quinn (§8 Q7). *Done:* the docs match the code; #19's diff touches
`site/privacy.html` not at all and `CLAUDE.md` only in the two ruled edits below (2026-10-19:
Decision 3 lands them in B6, so the earlier "touches neither" gives way); under Q10 (a) the
deferral row is in `HANDOFF.md` §4.
*Ruled 2026-10-19 (Quinn): Q10 (a)*, so the deferral row is written. **The `CLAUDE.md` wording is
ruled (spec A2 Decision 3, 2026-10-19, Quinn) and lands with B6**: (i) the
existing line becomes "`commitments` without `--confirm` or `--registrar` always exits 0"; (ii) ruling
7's sentence gains "ends dropped courses as `agent:knowlu.registrar`… files a `commitment-check` card
under the day's cap for a hand-set field" (the ledger's short form of the spec's recommended (ii):
"…applies later changes and ends dropped courses as `agent:knowlu.registrar`, journal first, and
files a `commitment-check` card, under the day's cap, for a field the student set by hand.").
Quinn ruled the recommended option, so the full sentence is the one ruled.

**B7. Whole-branch review.** `reviewer` at high over `main...p3-registrar`, the spec with both
amendment sections and this plan; `contract-reviewer` too over every contract-list file in the
diff. Expected: B4's fixture files under `engine/tests/fixtures/registrar/` (already reviewed in
B4 step 1; B7 confirms the merged bytes are the reviewed ones), plus any piece B3 moved to
`contract-engineer`. Checks beside correctness: §7
R3's single-user list, the kept-profile pin (§7 R4), no registrar title, CRN or room in telemetry
or a log, the fixture byte check. *Done:* `docs/reports/2026-10-18-p3-registrar-whole-branch-review.md`
says PASS; its fix wave, if any, is re-reviewed.

**B8. Merge.** The controller. Mark #19 ready, CI green on the PR, then merge with a merge commit
(the ledgers cite shas). Under ruling 7 of 2026-10-18 a lane PR merges on green CI and a reviewer
PASS without asking; #19 was not in wave 1's list, so the controller says it is merging in one line
first. *Done:* merged; `main`'s CI green.

**B9. The live proof (O10).** The controller with Quinn at the machine, on a dev build of `main`,
scratch profile at UA. The plan's closing checklist, plus: a second press after a hand edit to one
confirmed note's room shows that field untouched and its card filed. A refetch with a real change
cannot be staged and is covered by B2's tests. **Plus the status set (spec D3 amendment, P6):**
the controller records the `statusIndicator` and `statusDescription` of every row the account
returns, from Chrome DevTools Protocol network shapes as R0 did (enum codes only; never from
Knowlu's warnings, which name courses), and P5's `meetingScheduleType` /
`meetingTypeDescription` / `scheduleDescription` values. *Done:* the proof note in `docs/reports/`
(counts, enum codes and pass/fail lines only), cleanup per memory "live-proof profile cleanup".

**B9a. The registered set (spec D3 amendment, P6).** `mechanical` (a specified one-line data edit
plus its test): the registered `statusIndicator` codes B9 recorded go into `registrar::SCHOOLS`' UA
entry, with a test that a row under each recorded code counts and a row under an invented other code
does not. If B9 saw a drop or withdraw code, it is recorded in a dated line under the spec's D3
amendment and nothing more: turning it into an end is a later change Quinn signs. `reviewer`
reviews. *Done:* merged before L10's tag, or §8 Q11's answer says the release ships without it.
*Q11 ruled 2026-10-19 (Quinn): yes*, so B9a merged is the tag check's third condition (§5.5).
Then L5's planner re-plans from §6's signed text.

### The order, in one line

A1 ∥ A2 → A3 → A4 (Quinn's block; A1-A3 may run before or beside it) → *email merge, `main`
green* → B1 → B1a → (B4 ∥ B4a) → B2 → B3a → B3b → B3c (reviewer) → B3d → B3e (reviewer; or
B3e → B3e' (reviewer) if B3e splits) → B3f → B5 (queue slot) ∥ B6 → B7 → B8 → B9 → B9a → L5
re-plan; §8 Q10 and Q11 are answered before B2, Q12 before B4a's settle arm is built (B3e's
`registrar:` proposal-arm stamp is the spec's and does not wait on Q12).

## 5. Bringing #19 onto `main` (task B1)

### 5.1 When, and how

- **After the email branch merges** (L0; about 180 commits over `main` `43d4b1b`), with `main`'s CI
  green. Not before: `main` is red on the ingest time bomb until then, and a second integration
  after the email merge would redo most of the work.
- **A merge of `main` into the branch, not a literal rebase (§8 Q6).** The branch's commits are cited
  by sha in `HANDOFF.md`, the cloud design's ruling 11 (`p3-registrar's :91`), the commitment-model
  hand-off and the phase-3 plan; a rebase rewrites all twenty. A merge resolves each conflict once,
  keeps the cited shas, and matches how M1 and the commitment stack came onto `main`. If Quinn wants
  a rebase anyway, the ledger records the old-to-new sha map and B6 updates every citation.
- **First, preview.** `git merge-tree --write-tree main p3-registrar` lists the conflicted paths;
  the integrator records them in the ledger before resolving anything.
- **The PR.** #19 was opened with base `p2-commitments`; the controller retargets it to `main`.

### 5.2 Expected conflicts and the resolution rules

Files both sides edit (the email branch's list from the session-5 hand-off, plus what `main` gained
since the branch was cut: M1 grades, Gmail connect, M2 editing, events, the human token):

| File | #19's part | `main`'s / email's part | Rule |
|---|---|---|---|
| `app/src/main.rs` | the console builder manages `LmsSession`, wipes on the `lms-signin` window's `Destroyed` and at exit; three `registrar::` handler lines; `registrar` in the `use` line | the mail window and its saved-sign-in handling, grades' window, the Gmail, M2 and events commands | Keep both. Each window label keeps its own handler; #19's wipe matches `lms_link::WINDOW` only and never `grades::WINDOW` (`lms-grades`) or a mail window. Recount both lists with the plan's R6 Step 1 script. L6's ownership: §5.3 |
| `app/src/lib.rs` | `pub mod registrar;` | email's and grades' modules | Keep every line |
| `app/src/scaffold.rs` | `Curated` gains `registrar: Option<Registrar>`; `Registrar` and `Call` types; the UA row's entry | `policy_read` and grades fields (M1), the mail fields (email) | The struct takes every field; every row gets `registrar: None` but UA; no other row's data changes |
| `app/src/week.rs` | `your_week`'s `registrar_label` | events' and phase-1 follow-up changes, if any | Keep both keys in the envelope |
| `app/src/lms_link.rs` | helpers made public for reuse | grades and email edits, if any | Keep both; the registrar still uses the discard-everything window |
| `app/static/console.js`, `console.css`, `index.html` | the two buttons, the busy gating of Finish and Not now (`finishMayRun`), the "From myBama" list | email's settings, setup and resume views; grades' strip; M2's editors; events' cards | Keep both. Re-check that `finishMayRun()` also waits for anything email added to the confirm screen. `console-ui` reviews the resolved file if the conflict is over about 40 lines |
| `app/tests/static_assets.rs` | the registrar assertions | email's and others' assertions | Keep every test; a string both sides pin carries both |
| `scripts/wizard-check.py` | `check_registrar` | email's checks | Keep both; run it |
| `engine/src/lib.rs` | `pub mod registrar;` | email's modules | Keep every line |
| `engine/src/main.rs` | `commitments --registrar --school` | email's commands; ruling 11's actor defaults | **Take `main`'s actor mechanism** (the vault's token, no bare `"quinn"`); the branch's `default_value = "quinn"` lines go, and the `--registrar` branch resolves its actor through `human_or_vaults` like `--confirm` (A1-F2; B1, B1a). The bare-literal test and B1a's tests are the check |
| `engine/src/cli.rs` | `commitment_passes` (the registrar fresh keys, the ask gate), the dispatch | events' and email's hooks in `rank` | Keep both; the fresh-key line is rewritten by B3 anyway |
| `engine/src/commitments.rs` | §1's D5/D7 changes, the lock | events' one-off commitments, human-token sites, any later fix | `main`'s structure wins; #19's arms are re-applied on it. A changed signature on `main` (`detect_changes`, `confirm`) is followed, never reverted |
| `docs/surface/anatomy.md`, `app/README.md` | the registrar rows | email's rows | Keep both; B6 re-reads them |
| `CLAUDE.md` | the `commitments` entry and Tauri counts | rewritten since | **Take `main`'s**; B6 proposes any sentence to Quinn |
| `site/privacy.html`, `engine/tests/site.rs` | the class-schedule sentence and its assertion | L4 rewrites the page | **Take `main`'s**; the sentence goes to L4 (§8 Q5) |

Rules that hold for every file:
1. Never resolve a conflict by dropping a test from either side, and never edit an assertion to make
   a merge green; a test that fails after the merge is a finding, fixed or ruled in the ledger.
2. No bare `"quinn"` in non-test code outside the two constants; R24 writes the vault's token.
3. `engine/tests/fixtures/**`: `git diff --exit-code main -- engine/tests/fixtures ':!engine/tests/fixtures/registrar'`
   exits 0 after the merge; the frozen references are never opened for writing.
4. Semantic checks no conflict marker shows: `journal::human_edited` on `main` is the one B3 uses;
   `LOCAL_CARD_KINDS` and `sync`'s tripwires are untouched; events' `exam` and other one-off kinds
   are not `class` or `lab`, so the per-kind hold leaves them alone; email's first-day hold and the
   doubled first-day cap do not change what the confirm screen counts.
5. A new pin: the registrar's window is `lms_link::WINDOW` on a fresh throwaway directory, never
   grades' `session_dir` or a mail profile (`HANDOFF.md` §3 Pilot: the registrar never shares the
   kept Blackboard profile). One test in `app/tests/registrar.rs`.
6. The gate: `cargo build -p knowlu-engine -j 2` first (the zero-byte placeholder), then
   `cargo test --workspace` (dev profile), 0 other warnings, `scripts/ci/eol-check.ps1`,
   `wizard-check.py` ok, and `git diff --name-only main...p3-registrar -- cloud app/src/scheduler.rs app/src/commands.rs`
   prints nothing.

### 5.3 `app/src/main.rs` and `commands.rs`: lane L6's files

L6 (runtime removal) owns both while it runs; other lanes hand it their lines or wait for its
merge. #19 never touches `commands.rs`. Its `main.rs` part is more than three handler lines (the
console builder's `LmsSession`, two wipes and the `use` line), and handing that to L6's owner would
make L6 carry code it does not test for a module not yet on `main`. So, in order of preference:
1. **#19 merges before L6 makes its `main.rs` commit.** L6's plan puts its `main.rs` edit last
   (deleting `inference::` lines from the two lists and the wizard offer), and rebases it over #19.
   The conflict is a few adjacent list lines.
2. **If L6 is ready first, L6 merges first**, and B1 (or a small B1b) re-applies #19's `main.rs`
   part on top, recounting by script. #19 does not ask L6 to wait.
L6 and #19 also meet in `engine/src/main.rs` and `cli.rs` (the runtime's commands); the same
whoever-is-second rule applies.

### 5.4 `console.js`: one `console-ui` queue

The queue is L4 re-consent → L7 → L1 R11 → L5 → the redesign. **Recommended slot for #19: after
L7, before L1 R11** (L4 → L7 → #19 → L1 R11 → L5). Why: #19's page work is already written and
arrives with B1's merge; its new piece (B5) is small; L1 R11 is C3's last task and lands late; L5
must come after #19 by Quinn's ruling. If #19 is not ready when L1 R11 is, L1 goes first and #19
takes the next slot, never after L5. Whichever lane merges into `console.js` second rebases its
page work over the first and re-runs `static_assets` and `wizard-check.py`.

### 5.5 The release guard

No release is tagged from a `main` carrying the registrar button until the privacy page names the
class-schedule fetch (spec §4). This joins `HANDOFF.md` §3's release-gate list. §8 Q5 says how the
sentence reaches the page; §8 Q8 says whether the friends release carries the button at all.

**The check before L10's tag (spec A2 item 18; added at spec review round 1).** The controller
checks ruling 8's two conditions (B9's live proof passed; L4's page carries the sentence) and, if
§8 Q11 is yes, a third (B9a merged), against the ledgers, and records one dated row. If any fails:
- `mechanical` makes the fallback commit: the UA row's `registrar` in `app/src/scaffold.rs` becomes
  `None` (no other row or field), and the UA-pinning assertions B4 named change to the release's
  behaviour, with the reason in the message;
- `reviewer` reviews it; it lands on `main` before the tag.

L10's integrator does not tag without the controller's row. If §8 Q11 is no, the row also records
that the release notes and `HANDOFF.md` state the residual (a dropped course Banner still returns
keeps its block).

## 6. The L5 delta: what the syllabus spec must say now

`docs/specs/2026-10-05-syllabus-class-times-design.md` (DRAFT revision 2, untracked on the main
checkout) assumed L5 lands first and builds the seam. With #19 first, the registrar lands the
seam and L5 adds a source to it. What changes for L5:
- **Precedence.** #19 already makes `precedence` `(tier, calendar)`: registrar 0, `google:` 1,
  everything else 2, and the collapse's `rank` uses it (`(precedence, source_uid)`). L5 does not
  need a separate `source_rank`: it inserts `syllabus:` as tier 1 and moves `google:` to 2 and the
  rest to 3. Every existing pair keeps its order, so `by_key` and every existing test are
  unchanged. Q6's order (registrar > syllabus > Google/ICS) stays.
- **The hold.** #19's per-kind hold (its Amendment 6) already suppresses any non-registrar class or
  lab proposal for a `(kind, course)` the registrar holds, so syllabus rows of that course withdraw
  without new code; a held row is never `contested`.
- **Keep-alive and ending.** #19 adds a registrar arm to `refresh_series` (in play by `until`; a
  missing row ends at once). L5 adds its two syllabus arms beside it, or as one more row of the
  per-source table if B3 builds it (§8 Q4).
- **Change detection.** After O1 the registrar applies and lists and files no change card; a
  syllabus still proposes. So the registrar leaves the card path while `syllabus:` joins it; L5's
  "`SERIES_KEY_PREFIXES` gains `syllabus:`" holds against whatever B3 leaves (the array, or the
  table's landing column).
- **The instance map, a gap the spec does not name.** A syllabus series has no instances, as a
  registrar series has none; under `instances_map` its confirmed note would block nothing inside
  the 28-day horizon (#19's plan problem 3, R2-d). L5 must leave syllabus series out of the map the
  way #19 leaves registrar series out.
- **The ask gate.** #19's `asks_wait_for_registrar` runs in `cli::commitment_passes`, outside
  `emit_asks`; L5's `SYLLABUS_WAIT_DAYS` runs inside it. An ask fires only when both allow.

**Proposed edits, for Quinn to sign with the spec (not applied here; L5's T0 applies them).** Line
references are to the untracked draft as of 2026-10-18.

**E1, header (:19).** "(`docs/specs/2026-09-26-commitment-model-phase3-design.md`, on the parked
branch `p3-registrar`, draft PR #19)" becomes "(`docs/specs/2026-09-26-commitment-model-phase3-design.md`,
merged to `main` with PR #19 before this lane, by Quinn's ruling of 2026-10-18)".

**E2, §2 non-goals (:144).** "The registrar. It stays on `p3-registrar`, parked until the Pilot
(§9)." becomes "The registrar. It reaches `main` with PR #19 before this lane is built; this spec
changes nothing in it (§9)."

**E3, §4.4, the prefix paragraph (:374).** Add after its first sentence: "PR #19 leaves the watched
prefixes as `gcal-series:` and `ics-series:` (the registrar applies its changes instead of carding
them, cloud ruling 3); `syllabus:` joins them, as an array entry or as a row of the per-source table
PR #19 leaves in `commitments.rs`, whichever is on `main` when T4a starts."

**E4, §4.4, the precedence paragraph (:401-408), replaced in full.** "**Precedence uses the tiers
PR #19 lands.** `precedence` is `(tier, calendar)`: tier 0 for a `registrar:` calendar, 1 for
`google:`, 2 for anything else, and the collapse's `rank` (one series per meeting signature) orders
by `(precedence, source_uid)`. This lane inserts `syllabus:` as tier 1 and moves `google:` to 2 and
the rest to 3 (Q6). Every pair of existing sources keeps its order, so `by_key`, which orders one
`source_uid` held under two calendars, is unchanged. A syllabus row and a calendar row that agree
(same kind, course and meet set) collapse to the syllabus row. A course the registrar holds, by a
confirmed in-play `registrar:` class or lab note or a current registrar series of that kind, gets
no syllabus row of that kind at all (PR #19's per-kind hold, its spec's Amendment 6)."

**E5, §4.4, a new paragraph after E4.** "**Syllabus series stay out of the instance map.** A syllabus
series carries no instances. `SeriesFile::instances_map` treats a series' instances as the only busy
time inside its 28-day horizon, so a confirmed syllabus class would block nothing there. Syllabus
series are left out of the map, as PR #19 leaves registrar series out (its plan ruling R2-d), and
their notes block time by `meets`."

**E6, §9, replaced in full.** "## 9. The registrar (PR #19), which lands first

The registrar (commitment phase 3) reaches `main` before this lane, by Quinn's ruling of
2026-10-18. It is the other planned class-time source, and the two share one shape: a series source
with its own calendar key (`registrar:<school>:<term>`; `syllabus:<slug>` here), a keep-alive rule,
a button on the confirm screen and the Schedule view, and a per-kind hold over Google and ICS.

How they fit:
- **The seam is the registrar's.** PR #19 lands tiered `precedence`, the registrar arms in
  `refresh_series`, the instance-map exclusion and the per-kind hold (as one per-source table if its
  close-out builds one). This lane adds `syllabus:` to each: tier 1, its two `refresh_series` arms
  (§4.4), the instance-map exclusion (E5), and the watched prefix.
- **Precedence: registrar above syllabus above Google and ICS** for the same course (Q6). The
  registrar is authoritative; a syllabus can be last term's copy; a calendar event is the student's
  own entry and may be stale.
- **Different landing rules, on purpose.** The registrar confirms matched courses as `hard` (R24)
  and applies later changes and lists them (cloud ruling 3); a field the student set by hand is
  never overwritten. A syllabus never does either: it proposes and waits (§4.5). When the registrar
  covers a course, that course's syllabus rows of the same kind are not proposed.
- **The ask gate.** The registrar's gate (no ask before the day after its term starts) runs in
  `cli::commitment_passes`; this lane's bounded wait (§8, Q16) runs inside `emit_asks`. An ask fires
  only when both allow.
- **The sign-in.** The registrar uses the discard-everything sign-in window and never the kept
  Blackboard profile. Offer A follows the same rule: its own window, cookies not kept.
- **Exams.** If the registrar's R0 spike found a final-exam schedule, a registrar final would be the
  authority for the final, and a syllabus final would yield to it as it yields to the feed (§4.6).
  Not designed here."

**E7, §10 row 6 and §11 Q6.** Q6 becomes "**Precedence:** registrar above syllabus above calendar,
built on the tiers PR #19 lands (syllabus inserted at tier 1). *Recommend yes.*" §10 row 6's option
(a) gains "(on PR #19's tiers)".

**E8, §12, the last risk row.** "Rebase cost on `p3-registrar` | Conflicts in `commitments.rs` | The
shared seam (§9) is small and lands with a note on PR #19 naming it" becomes "This lane builds on
the registrar's seam | A syllabus arm placed wrongly beside a registrar arm ends a class or loses a
card | T4t pins each source's arms separately; T4a runs as `integrator` after PR #19 is on `main`".

**E9, §13, T4a and T4b.** T4a's task cell becomes "The key; `syllabus:` added to the watched
prefixes (or the per-source table); the syllabus series fed to `refresh_series` as fresh each run;
the two syllabus arms beside the registrar's; syllabus series out of `instances_map`;
`create_confirmed_as`'s body line". T4b's becomes "`syllabus:` at tier 1 of `precedence` (google 2,
the rest 3); section match; `contested` on `Proposal` and in `--json`". T4t gains "syllabus series
block time by `meets` inside the horizon; a registrar-held course gets no syllabus row of that kind;
an ask waits while either gate holds".

**E10, §14, one more unverified item.** "Which shape PR #19 left in `commitments.rs` (the per-source
table or the prefix closures), and whether its R0 found a term list with dates or an exam schedule.
T4a and §9's exams bullet follow what is on `main`."

## 7. Risks

| # | Risk | Effect | Mitigation |
|---|---|---|---|
| R1 | **Un-spiked.** Every Banner call, header and row shape on the branch is provisional; the code was built before R0 (G4) | R0 finds another flow, a synchronizer token, a redirect (`ureq` drops `cookie` on every redirect, same host included), or no per-meeting dates | A4 before any B task that reads the shape; R1-a keeps the shape in `parse_banner`, its fixture, four test bodies and the curated call list; the re-plan trigger stops for Quinn; B9 proves the real path |
| R2 | **Applied writes into hard notes (O1)** | A wrong parse, or a truncated answer, silently moves or ends a class still meeting | Exit 2 writes nothing; an empty parse is a failure; A4 found the rows call unpaged, and an empty `registrations` is exit 2 (spec P1); a row under an unrecorded status is listed, never ended (spec P6, B9a); one term per key; `human_edited` respected; the list shows every applied write; B3 is Opus |
| R3 | **Single-user assumptions (rule 1)** | Something only the founder's vault or account hits | (a) R24's actor is the vault's token, never the literal `quinn` (the branch's `engine/src/main.rs:91` default goes; the bare-literal test fails otherwise; B2 pins `student` and legacy `quinn`). (b) UA is data: the curated `Registrar` row (label, host, prefix, start URL, calls) selected by `config/campus.yaml`'s `unitid`; the page reads `registrar_label`, so "myBama" is never in `console.js` logic. (c) The engine's `registrar::SCHOOLS` names `ua` and its term suffixes in code (the parent's §10: adding a school is a code change); B4 adds a test that the app's curated `registrar.school` keys and the engine's `SCHOOLS` keys are the same set, so a second school cannot half-exist. (d) A second UA student needs nothing hand-edited; a student elsewhere sees no button and nothing breaks; B7 checks both. (e) No test, fixture, doc, ledger or proof note carries a real CRN, course, room, schedule or a myBama username; R0 and B9 record shapes and counts only; the scratch vault holding Quinn's real schedule during B9 is removed after |
| R4 | **The kept profiles** (ruling 12's Blackboard session; email's saved mail sign-in) | The registrar reads cookies from a kept profile, or wipes one | The registrar uses `lms_link`'s `lms-signin` window on a throwaway directory; §5.2 rule 5's pin; #19's console wipe matches that label only |
| R5 | **Integration size** | The email branch alone is about 180 commits; `console.js` conflicts are large; a semantic break shows no marker | Preview with `merge-tree`; §5.2's rules; the full gate and `wizard-check.py`; B7 reads the merged diff, not only #19's commits |
| R6 | **The lost ledger** | Earlier review findings and rulings exist only in the hand-off and the away log | A1 re-reviews the tail; B7 reviews the whole branch; this plan's ledger restarts the record |
| R7 | **Collision with L6 and the `console.js` queue** | Two lanes edit `main.rs` or `console.js` at once | §5.3 and §5.4's order; whoever is second rebases |
| R8 | **Release before the page names the fetch** | The registrar reads a schedule under a page that does not say so | §5.5's guard; §8 Q5 and Q8 |
| R9 | **Telemetry and logs** | A course code or room reaches telemetry or a log line | B7 checks that no registrar title, CRN, room or fetched byte enters telemetry (a) or (b), a log, an envelope or an error string (plan constraint 9) |
| R10 | **Okta and Banner drift** | UA changes its sign-in or Banner version; the button breaks | A failure is a named message, never a partial write; the fix is a 0.1.x release through CI (ruling 10's stated cost); the curated row can drop the entry to hide the button |
| R11 | **Pilot scope inside a friends MVP** | The registrar was parked as Pilot work; merging it now puts it in front of testers | §8 Q8: the curated UA row carries the entry only once R0 and B9 pass |

## 8. Decisions for Quinn (asked one at a time, recommendation first)

1. **R24 (O2).** Registrar rows that match a vault course are confirmed `hard` without asking,
   written as the student via `dashboard`. *Recommend yes:* the student pressed the button, and the
   G1 fix keeps later registrar changes applying. (Alternative: create them as
   `agent:knowlu.registrar` too; one actor for every registrar write, but against the parent's §2.5,
   which has confirmations written as the human.)
2. **The card for a hand-set field (O1).** When the registrar changes a field the student set by
   hand: (a) the existing commitment change card (`commitment-check`, local, already built), or
   (b) a `kind: amend` card, which needs `meets`, `where` and `until` in `approvals::AMENDABLE_FIELDS`
   (contract list, and `approvals.rs` is L5's while L5 runs). *Recommend (a):* it is the commitment
   model's amend card already, and (b) collides with L5. Signed together with A2's amendment.
3. **G2-G5 (O3).** G2: confirm (part 2 now sets the applied `until`, not an end card). G3: keep
   1 January / 1 June unless R0 finds term dates, then use them. G4: closed by R0 and B4. G5:
   confirm; L5 inherits it for syllabus rows.
4. **The seam.** #19's close-out folds its prefix tests into one per-source table that L5 extends by
   one row (B3), or leaves the closures and L5 adds its arms beside them. *Recommend the table:* B3
   already rewrites the landing rule, so the extra cost is small and the second source is cheaper.
5. **The privacy sentence (O8).** It goes into L4's honest page and rides L4's `PRIVACY_VERSION`
   move; #19 does not edit `site/privacy.html`. *Recommend yes.* If L4's text is final before #19
   merges, the sentence still goes in, worded "at schools Knowlu supports", true either way.
6. **Merge, not rebase (§5.1).** *Recommend a merge of `main` into the branch*, to keep the cited
   shas. Quinn's ruling said "rebase"; this asks whether a merge serves.
7. **`CLAUDE.md`.** One sentence under "Command and app contracts": "`commitments --registrar` makes
   no network call and exits 0 or 2; on 2 it writes nothing; it confirms matched rows as the student
   and applies later changes as `agent:knowlu.registrar`, journal first." *Recommend yes;* rule
   text is Quinn's. *(Its wording is amended by spec A2 Decision 3, ruled 2026-10-19; see B6.)*
8. **The friends release.** Does the friends MVP carry the button? *Recommend:* only if A4 and B9
   pass before L10's tag and L4's page carries the sentence; otherwise the UA curated row ships
   without its `registrar` entry (data, no build flag), and the entry returns in the next 0.1.x.
9. **L5's spec edits E1-E10 (§6)**, signed with the syllabus spec.
10. **What a registrar final becomes** (added at spec review round 1; the spec's "Decisions for
    Quinn (A2)" 1; brought in line with it at convergence round 2). R0 found `EXAM` meetings
    beside `CLAS` ones, and no ruling covers them. **L5's amended spec (§9, "Exams") designs no
    registrar finals rule.** It follows whatever this answer puts on `main`: an arm for "dropped"
    and an arm for "a one-off `exam` commitment". So no lane owns registrar finals unless this
    answer names one. Before asking, the controller re-reads that §9 bullet on the main checkout
    and adjusts this item if L5's running amendment has changed it. P3's rule (a final is never a
    weekly class) is built either way.
    - (a) **Skipped and counted, named deferred.** *Recommended:* the smallest build before the
      friends release; L5's §9 handles it as written; nothing here is reworked later. The cost,
      stated plainly: the friends MVP shows no registrar finals, and they have no owner until the
      `HANDOFF.md` §4 deferral row B6 writes is picked up by a later lane.
    - (b) **Listed, never written, now.** Not recommended: L5's §9 has no arm for a listed final,
      so L5's planner would have to add one.
    - (c) **Written now** as a hard dated one-off `exam` note under R24: the strongest answer to
      "what's next?", and L5's "yield" arm is ready for it. The cost: a second create path,
      moved-final and dropped-final cases in B2 and B3, and a B9 recording that an `EXAM`
      meeting's `startDate` equals its `endDate`, all before L10's tag.
    - **Ruled 2026-10-19 (Quinn): (a)** (spec A2 Decision 1). Registrar `EXAM` meetings are skipped
      and counted, with a deferral row in `HANDOFF.md` §4 (B6); no lane owns registrar finals until a
      later lane takes the row.
11. **Does B9a's registered set join ruling 8's conditions?** (The spec's Decision 2.) Without it a
    dropped course Banner still returns keeps its hard block all term, unlisted. *Recommend yes:* B9
    always sees registered rows, so it can always record their codes, and B9a is a one-line data
    edit; with the set, the worst case is a dropped course listed and kept, which the tester sees.
    **Ruled 2026-10-19 (Quinn): yes** (spec A2 Decision 2). B9a's registered set joins ruling 8's
    conditions (a one-line data edit plus a test).
12. **What an approval of a registrar-held card is stamped (B4a, added 2026-10-18 with A1's
    findings).** A student approves a Google class card after the registrar came to hold that
    course's class. *Recommend `superseded`* (nothing written, one warning), the stamp
    `withdraw_stale` already gives a withdrawn card, so the two paths agree and the question is
    still open if the registrar's note later ends (a dropped course) and the Google class is
    proposed again. The alternative, `refused`, closes the question for good under the asked-once
    index, so a student who drops the registrar course is never asked about the calendar class.
    Ruled decisions are untouched either way: R24 (Q1) and the change card for a hand-set field
    (Q2).
    **Ruled 2026-10-19 (Quinn): `superseded`.** Approving a Google class card for a course the
    registrar already holds is stamped `superseded` (nothing written, one warning, as
    `withdraw_stale`); it can be asked again if the registrar course is dropped.

## 9. Fidelity ledger

| Requirement (the caller's brief and the ruling) | Where |
|---|---|
| Everything open on #19: the apply-and-list revision, R0, R24, the review of `2ad1e3d`, G2-G5 | §3 O1-O5, §4 A1-A4; the follow-ups O6-O10, B4-B9 |
| For each: what, who (agent per `CLAUDE.md`; R0 controller + Quinn), done criteria | §3's table, §4 per task |
| The integration onto `main` after the email merge: conflicts, the integrator's rules | §5.1, §5.2 |
| `main.rs`/`commands.rs` owned by L6: how #19's lines are sequenced | §5.3 |
| `console.js` queue placement | §5.4 |
| The L5 delta: which seam, §9 and §11 text, precedence registrar > syllabus > Google/ICS, as proposed text | §6 E1-E10; §8 Q4, Q9 |
| Risks: UA-only and un-spiked; rule 1; the curated UA entry as data | §7 R1, R3, R10, R11 |
| VISION and the two overriding rules | §2; §5.2 rules 2-3; §7 R3 |
| Trade-offs and open questions to Quinn, not assumed | §8 |
| No code pre-written; behaviour and tests specified | §4 A2, B2-B5 |
| Spec review round 1 (A2/D3): finals open for Quinn; B4 before B2 with named owners; the status set recorded and B9a; the fallback's owner and the tag check | §8 Q10-Q11; §4 B4, B2, B9, B9a, the order line; §5.5 |
| A1-F1: the per-kind hold is in `proposals()` only; `withdraw_stale` must withdraw on every condition that stops `proposals()`, and an approval must not write the second hard class note | §4 B4a (who, files, red-first tests, done), B2's end-to-end bullet; §8 Q12 |
| A1-F2: ruling 11, the registrar writes as the vault's human token; B1 takes `main`'s `actor: Option<String>` arm and `human_or_vaults`; tests read `journal::read_human_actor` | §4 B1, B1a; §5.2's `engine/src/main.rs` row |
| The D3 amendment's fixture and call differences as B4's concrete scope; the fixture written by `contract-engineer` and reviewed by `contract-reviewer` (corrected at convergence round 2: it is a contract-list path) | §4 B4, "B4's concrete scope" table and the paragraph under it |
| Ruled decisions unchanged (R24 as the student via `dashboard`; the hand-set field's change card; merge before L5) | B1a's "ruled decisions kept"; §8 Q12's last sentence |
| Spec review round 2: P9's `seen` (B4 owns it, B3 reads "missing" only from it); `SERIES_LOCK_FILE` as the one filer; the block's bytes and no `registrar` key in a vault that never fetched; G3's next-term function; ended once; the two-column per-source table | §4 B4 (`seen` and its tests), B2's round-2 list, B3a (seen, next term), B3c (bytes), B3d (ended once), B3e (lock, with the lock-blocking check), B3f (two key columns) |
| Spec convergence round 1: the open and closed `ended` entry (a drop after a come-back is a new drop); P9's merge carries a `partial`, `unread` or `status` CRN's previous row; no create from a partial read; B3 split into tasks of about 80 lines | §4 B2's convergence list, B3a (carry, no create), B3d (open entry, come-back), B3e (prior from the carried row), B3's shared rules and the order line |
| Spec convergence round 2: the block's object keys follow `ledger::dumps_value`'s sorted order (the spec fixes only list-entry order; `ledger.rs` unchanged); the fixture is `contract-engineer`'s with `contract-reviewer`'s review; Q10 matches spec Decision 1 (L5's §9 designs no finals rule) and B6 writes the deferral row under (a) | §4 B3c ("The block"), B4 (step 1 and "Who writes the fixture"), B6, B7; §8 Q10 |
| Spec convergence rounds 3-5: A2 item 20 has five groups ("Convergence round 3 adds" and "Convergence round 5 adds" beside the first three, and round-3/round-4 cases inside earlier tests); P9's carry narrowed to a confirmed note's CRN; a kept-field card withdrawn with its entry, at the fetch and at `rank`'s second call under the lock; the `asked` drop of `registrar:` proposals not under `series` in the re-read; `settle_approved`'s change and proposal arms return `Superseded` (the proposal stamp fixed by the spec, not §8 Q12) | §4 B2 (all five headings and the round-3 and round-5 lists), B3a (narrowed carry; "a confirmed note is still carried"), B3e (withdrawal, the `asked` drop, the proposal-arm settlement, the B3e' split line, and the round-3 and round-5 tests) |
