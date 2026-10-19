# First-day event allowance (lane L3): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: REVIEWED; the stamp rule's fourth re-review (2026-10-19) PASSED, and its minors are closed
in the rule's final pass, r8 (§9.00 item 11).** The review record is this line and §9; no review
report is on disk, and none is needed to trace a verdict:
- **Round 3** (plan-mode `reviewer`, 2026-10-18): **PASS**. 0 critical, 0 important, 4 minors: T1
  Step 4's `deno test` lacked CI's flags; Q-F's citations into L8's and L5's specs were stale; §4's
  `cloudmodel.rs` row missed L7; spec test 12's "exit unaffected" had no stated proof. All four are
  answered in §9.00 items 1-4 (2026-10-19), with no change to any task's scope.
- **Convergence review** of this plan with L8's spec (plan-mode `reviewer`, 2026-10-19): **PASS**.
  0 critical, 1 important (the C3 plan's §5 departed from the shared stamp rule in R6, R13 and its
  `--include-all` fallback), 3 minors (M2's "same stamp" claim; this status line resting on a report
  not on disk; the cloud design's amendments uncommitted). Answered in §9.00 item 6.
- **Stamp-rules review** of this plan, L8's spec and the C3 plan's §5 (plan-mode `reviewer`,
  2026-10-19): **not a pass**. 0 critical, 1 important (M7 cleared a lane ahead on its merge alone,
  and the documents disagreed on S7a's wait and on the remedy for a merged, unpushed migration), 1
  minor (the amendments and these documents uncommitted). Answered in §9.00 item 7; the minor is the
  controller's commit.
- **Re-review of the stamp-rules answers** (plan-mode `reviewer`, 2026-10-19): **not a pass**. 0
  critical, 1 important (a merged, unpushed lane recorded as slipped still rode into staging inside
  the next lane's rebased `db push`), 1 minor (still uncommitted). Answered in §9.00 item 8 on the
  controller's direction.
- **Second re-review of the stamp rule** (plan-mode `reviewer`, 2026-10-19): **not a pass**. 1
  important (a lane that pushed from its lane branch but had not merged counted as cleared and could
  slip, though every dry run behind it then refuses). Answered in §9.00 item 9.
- **Third re-review of the stamp rule** (plan-mode `reviewer`, 2026-10-19): **not a pass**, though
  r6's earlier fix passed. 1 important (a lane that pushed from its branch and then stops can never
  be cleared), 3 minors (a re-stamp under a broken rule can deadlock behind a pushed, unmerged lane;
  the dry run assumed every orphan version is a lane's; two wording points). Answered in §9.00 item
  10 (the rule's r7).
- **Fourth re-review of the stamp rule** (plan-mode `reviewer`, of the rule's r7, 2026-10-19):
  **PASS**. 0 critical, 0 important, 5 minors and one M5 note (M5 ignored a re-push after a
  withdrawal; a withdrawal dropped objects while branch-built functions still used them; no check
  that the drop finished; no path if Quinn declines M8's revert; T5 withdrew on any failed proof; a
  follow-up migration had no place in the order). All closed in §9.00 item 11 (the rule's r8), with
  Quinn's withdrawal ruling of 2026-10-19 written in.
- **Round 2**'s five findings are answered in §9.0, and **round 1**'s six in §9.1. §9.2 is an earlier
  **pre-review self-check** (seven points), kept so its fixes stay traceable; it is not a review round.

The 2026-10-19 convergence also wrote one migration-stamp rule shared by the email, L3, L8, L5 and L1
migrations into §2.1, after Quinn's C3 Q8 ruling; the C3 plan's §5 now cites it. Nothing runs until
the controller has an answer to §3's open questions, or has deferred them. The spec header's signature
is Quinn's (§7, T0).

**Date:** 2026-10-18 (session 5, the friends-MVP push). **Lane:** L3 of
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5.

**Spec:** `docs/specs/2026-10-06-first-day-event-allowance-design.md`. It is untracked in the main
checkout and its header still reads DRAFT. **Every decision in it is ruled:** Quinn's row of 2026-10-06
in `.superpowers/sdd/2026-09-30-email-forwarding/progress.md` reads "build as drafted, all 10 decisions
at their recommendations (2,000 event judgments once per account, opened at the first event judgment
the daily cap refuses, 14 days, never renewed, events only, keep the first-two-days doubling, $2 ceiling
unchanged, per-run 150 unchanged, nothing new shown, include the capped-run fix in events.rs, one per
account)". **Quinn signs the spec header in their block (handoff §4 item 5). This plan does not edit
it.** Task T0 commits the spec file onto the lane branch as Quinn left it.

**Goal.** A new student's first event backlog (about 1,000 events at the T18c school) is judged in
three to four days instead of about two weeks. This also covers the account the first-two-days
doubling misses (a new profile or vault on an old account). Two pieces:
- The server spends a one-time 2,000-judgment allowance only on event calls the daily cap refuses.
- The engine's events pass stops at the first `capped` reply with one honest line.

**Architecture.**
- **Cloud.** One migration (a table and two RPCs). One change to the pipeline's charge order, for
  `event` only and only on overflow, with its refund routing. `charge_call`, `refund_call` and
  `enforce_budget` are not re-issued.
- **Engine.** One change to `events.rs`'s `judge_roster`. No change to what the engine asks the server,
  so no device needs an update for the allowance to work; the engine fix is only about honesty.

**Code read for this plan:** the worktree `.claude/worktrees/email-forwarding` at 68301ec, which is what
`main` becomes at the email merge. Every `file:line` here is from that tree, so the implementer
re-confirms each one on the lane branch before editing.

## 1. VISION and the two overriding rules

- **VISION.md:154 and :192** (success criterion 5: "a new student reaches 'it already knows what's
  next' in their first session"). These are the 68301ec worktree's lines, which the lane branch
  inherits. The main checkout's uncommitted `VISION.md` has the same text two lines lower (:156,
  :194); re-confirm on the lane branch. The spec's :155-159 and :192 are within a line of these. The
  allowance serves this criterion directly.
- **VISION.md:155** ("Minimize the AI surface"). Holds: no new judgment kind, no new prompt, no new
  model call type. The allowance only stops the cap from cutting a run short. `rank` still never calls
  a model.
- **Rule 1 (no single-user assumptions).** Holds: the allowance is keyed on `account_id`. Nothing
  names a school, profile, vault, device or mailbox. Tests use generated uuids and `acct-1`-style ids.
- **Rule 2 (never regenerate a frozen reference).** Holds: no fixture, frozen reference or
  `surface-today-*.json` moves (spec Q8 = nothing new shown).
- **Contract list.** No task edits a file on it (spec header; re-checked here). `judge_caps.ts`,
  `judge_pipeline.ts`, the migrations, `migrations_test.ts`, `cloud/eval/run_eval.ts`,
  `engine/src/events.rs` and `engine/src/cloudmodel.rs` are all off it. The one borderline file is `engine/tests/cloud_contract.rs`,
  which only the conditional task T3b touches (§3 Q-B).
- **Privacy: no new data class.** `judge_allowances` is usage metering of the same kind as
  `usage_daily`: per account, an opening time, an end time and two counters. It holds no grades, no
  coursework, no event or note content and nothing from a mailbox. It is account data, which
  `site/privacy.html` already covers ("Account data — until you delete it"), and it is purged with the
  account by the cascade. `usage_daily` has no line of its own on the page either. So the privacy page
  and `PRIVACY_VERSION` (`app/src/account.rs`, L4's file) do not move. Quinn hears this at the
  checkpoint (§7); if Quinn wants a line on the page, that is a privacy bump, and it goes to L4, not
  this lane.
- **Money: this lane touches it.** It adds a spend path on the judge route: up to 2,000 extra model
  calls per account, and at most 4,000 model requests counting refunds (spec §6, about $0.22 at the
  pinned price), under the unchanged $2 ceiling. The 2026-10-06 ruling settled the design; under the
  merge-authority ruling of 2026-10-18 (handoff §1 item 7, "anything touching money"), the **merge**
  still waits for Quinn's word (§5, T4, Q-G).

## 2. Global constraints

- **Branch and worktree.** Branch `l3-first-day-allowance`, worktree
  `C:\Users\danie\GitHub\knowlu\.claude\worktrees\l3-first-day-allowance`. Both are created by the
  controller from `main` **after the email-forwarding merge**, never before: the migration must stamp
  after the email branch's migrations, and `judge_caps.ts` and `judge_pipeline.ts` are read as the email
  branch left them. Use absolute paths and never `cd`.
- **Agents never commit, push or branch.** Each task ends by handing the controller a file list and a
  commit message. The controller commits: `git add <paths>` (never `-A`), `git commit -F <file>`, merge
  commits for the lane PR.
- **Gates.**
  - Engine: `cargo build --workspace` and `cargo test --workspace` for the worktree, dev profile, in the
    foreground, and only under the build-slot guard (two cargo runs machine-wide; handoff §5). The
    guard lives in the **main checkout**, not the worktree (`.superpowers/` is not on the branch):
    `C:\Users\danie\GitHub\knowlu\.superpowers\sdd\2026-09-29-ultracode\slot.sh`. It runs its
    arguments as given and blocks until a slot is free, so pass the worktree's manifest rather than
    `cd`:
    `bash C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test --workspace --manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l3-first-day-allowance/Cargo.toml`
    (and the same with `cargo build`). **0 warnings is part of green.** The one accepted line is the
    app's `.rsrc merge failure: multiple non-default manifests`.
  - Cloud: the three commands `ci.yml`'s `cloud` job runs (`ci.yml:148-167`), exactly as CI writes
    them. Prefix each path with the worktree's absolute path rather than running them after a `cd`.
    CI checks more than `cloud/supabase/`:
    - `deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts cloud/eval/*.ts`
    - `deno lint --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
    - `deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1
      --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS
      --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/` (one line in `ci.yml:167`). The
      flags are always needed, not only "if a test needs them". Tests under `cloud/supabase/` bind
      loopback listeners (`judge_anthropic_test.ts`, `auth_test.ts`, `http_test.ts`), and
      `cloud/eval/`'s loader tests write under `cloud/eval`. With `--allow-read` alone those tests fail
      with `PermissionDenied`, and that red is not the task's.
    - Re-read `ci.yml:167` on the lane branch before the first run. If the line has changed, use the
      branch's line, not this copy.

    `cloud/eval/run_eval.ts:93` implements `CapStore`, so it is in T2a's diff (§4).
  - No test leaves the machine.
- **CI on main is red until the email merge** (the ingest time bomb; handoff §8). The lane branches
  after that merge, so its PR's CI is expected green. A red check is never merged over without Quinn's
  word.
- **LF everywhere.** `*.ps1` stay CRLF. `engine/tests/fixtures/**` is untouched.
- **No secret, account id, email, school or person's name** in code, tests, comments or test names.
- **TDD.** Every behaviour below is a failing test first. The test is run and seen red for the stated
  reason, then the code follows, then green.
- **Write-path rule.** Nothing in this lane writes a vault file except through the existing
  `eventledger::record_judged_verdict`. T3 removes per-item lines; it adds no writes.

### 2.1 The migration's stamp: the shared rule, and what it means for L3

**The migration-stamp rule (shared by the email, L3, L8, L5 and L1 migrations; written 2026-10-19;
M5, M7 and M8 revised the same day after the stamp-rules review and its re-review, and M7 and M8
again after the second and third re-reviews; M1, M5, M7 and M8 given their final pass, r8, after the
fourth re-review passed, with Quinn's withdrawal ruling of 2026-10-19 in M7).**
The same eight points appear in this section and in L8's spec (`2026-10-18-friends-testers-design.md`
§8.4). If the two copies ever differ, the controller re-aligns them to the newer dated copy before
either lane dispatches a migration task. The rule rests on handoff §5's merge order and on Quinn's C3
Q8 ruling of 2026-10-19: "C1 waits until L3, L8 and L5 have merged, or until the controller records
that one slipped past C3; the prod deploy stays a plain push."

- **M1. Order.** Stamps sort in the merge order email → L3 → L8 → L5 → L1 (handoff §5; L5a, PR #19,
  adds no migration). The email branch's two migrations are fixed: `20261002000200_mail.sql` and
  `20261005000100_mail_claim_max_tokens.sql`. Any migration the email branch adds before its merge
  follows M3 and sorts after them. A migration a lane adds after its own merge (a follow-up fix to
  merged SQL, whose file M6 froze) joins this order as a new lane at the back, behind every lane then
  on record, and follows M3, M5 and M7 like any lane.
- **M2. Format and the next free stamp.** A stamp is fourteen digits (the root `migrations_test.ts:35`
  check). It is the date the stamp is fixed, followed by a step counter in the time field: `000100`,
  `000200`, `000300` and so on, never a wall-clock time. **The next free stamp after X** is today's
  date with `000100` if that sorts after X. Otherwise it is X plus `000100`. This definition replaces
  the C3 plan's earlier "one minute after the newest on `main`" (its r6 §5), which gives a different
  stamp whenever the newest is from an earlier day; the C3 plan's §5 now cites M2.
- **M3. Picking.** A lane fixes its stamp when it creates its file: L3 at T0, L8 at S2a, L5 at T6a,
  and L1 at R6 (provisional, per the C3 plan §5). The stamp is the next free stamp after the greatest
  of three sets:
  - every migration on `main`;
  - every stamp that a lane ahead of it (M1) has recorded;
  - every version staging has applied.

  The lane records the stamp in its own ledger before it creates the file, and the controller passes
  the stamp to every lane behind it. No two lanes ever record the same stamp. A stamp chosen before
  every lane ahead has recorded its own is **provisional**: L8 waits for L3's (its S2a), and L5 and L1
  may go ahead and rely on M5's re-check.
- **M4. Tests never name a stamp.** Each lane's tests find its file by its suffix, as
  `mail-process/worker_test.ts:959-967` finds `_mail.sql`: `^\d{14}_event_allowance\.sql$`,
  `^\d{14}_testers\.sql$`, `^\d{14}_mail_upload\.sql$`, and L5's own suffix. So a rename changes no test
  line. The only lines it moves are the shared parse-count pins in `migrations_test.ts`, which the lane
  recounts on its rebased tree (the sum of everything there, never either side's literal). The last
  lane to land owns the final numbers.
- **M5. The re-check before the first push.** Immediately before a lane's migration first reaches
  staging, or reaches it again after a withdrawal (M7), its stamp is checked again against M3's three
  sets, on the lane branch rebased on `main`. Under M7 that is before the lane's merge (whose push
  follows at once) or before a push from its lane branch, whichever comes first. If anything sorts at
  or after its stamp, the file is renamed (`git mv`) to the next free stamp, the pins are recounted,
  and a dated row goes in the lane ledger. That is the last free rename unless the controller
  withdraws the lane (M7). Only a migration merged against M7 (its push did not follow) reaches its
  first push with no lane branch: then the check runs on `main`, and a needed rename is M8's re-stamp
  commit, never M6's repair.
- **M6. Frozen from the first push.** Once staging's history holds the version, the name is frozen,
  because `supabase db push` refuses to run while a remote version has no local file. Any later rename
  is the controller's alone, done as one step with both stamps in the ledger:
  - on staging, `supabase migration repair --status reverted <old stamp>`;
  - the `git mv`;
  - then `supabase migration repair --status applied <new stamp>`. The SQL already ran under the old
    name, so it is not run again. The C3 plan §5's `db push` variant re-runs the file, so it suits only
    an idempotent file such as L1's (its P11).

  No agent renames a frozen file.
- **M7. Merge with push, and staging push order.**
  - **A migration merges only with its push.** A lane's migration merges to `main` only once its
    staging push is allowed, and the controller pushes it to staging from `main` right after the merge
    (unless the lane already pushed from its lane branch). "Merged but not pushed" is a transient
    state, never a waiting one. A lane whose push waits on a Quinn word does not merge before that
    word (L3's T5 waits on Q-E, so L3's merge waits for Q-E as well as Q-G).
  - **Clearing.** A lane pushes its migration to staging only after every lane ahead of it is
    cleared. A lane ahead is cleared once its migration is **on staging and on `main`**, in either
    order. Either half alone holds every lane behind it:
    - **Merged, not on staging:** its file is on `main`, so every branch rebased behind it carries the
      file and a plain `db push` would apply it.
    - **On staging, not merged** (pushed from its lane branch): staging holds a version `main` lacks,
      so a dry run on `main`, or on any branch rebased on it, refuses. The lane holds every lane behind
      it until it merges or the controller withdraws it (below).

    A lane that has **neither merged nor pushed** is also cleared once the controller has recorded it
    in the session ledger as having slipped behind. A recorded slip clears no other lane.
  - **Withdrawal.** A lane that pushed from its lane branch and will not merge as pushed leaves
    staging only by withdrawal, and only the controller withdraws a lane. When it may (Quinn's
    ruling, 2026-10-19):
    - **The lane's staging proof fails**, meaning it shows a defect in the lane's SQL: the controller
      withdraws the lane on its own, records the withdrawal in the session ledger and tells Quinn. A
      defect in the lane's functions alone is no withdrawal: they are fixed on the lane branch and
      redeployed from it, and the migration stays as pushed.
    - **Quinn declined the lane's merge:** that counts as Quinn's word, and the controller withdraws.
    - **Any other case** (for example, Quinn's word on the merge still unanswered after a pre-merge
      push, a failure the controller cannot place in the SQL or the functions, or a lane that stopped
      for another reason): the controller asks Quinn first.

    Five steps, in order, each recorded in the session ledger and the lane ledger:
    - Redeploy from `main` every function the lane deployed to staging from its branch (none, if it
      deployed none). `main`'s code ignores what the lane added, provided every column the lane added
      to an existing table is nullable or has a default. Where the lane changed an existing object
      that `main`'s functions call, the lane's own withdrawal step names the window between this
      redeploy and the next step.
    - On staging, in a single `begin … commit`, drop every object the lane's migration created, as
      its file names them (tables, views, functions, triggers, `pg_cron` jobs, schemas, and columns it
      added to existing tables), and put back `main`'s definition of any object it changed.
    - Check the catalogue for every object the file names (tables, views, functions, triggers,
      columns, `cron.job`, schemas), and record that nothing is left and that each changed object
      reads as `main` defines it. If anything is left, stop: the version row stays.
    - Then remove its version row: `supabase migration repair --status reverted <stamp>`. Once the
      objects are gone this is safe, because no later push can meet them.
    - Run `supabase db push --dry-run` on `main` and see that it no longer names the lane's version.

    The lane then counts as neither merged nor pushed: its name is no longer frozen (M6), it may be
    recorded as slipped, and M5 runs again before any new push.
  - **A dry run before every staging push and every merge.** Every staging push, from a lane branch
    or from `main`, first runs `supabase db push --dry-run`. If the dry run refuses, or its list holds
    any version other than the pushing lane's own, the controller stops, records the version, and does
    not push. So no migration ever reaches staging inside another lane's push. A refusal that names a
    remote version with no local file means a lane ahead pushed before merging; the controller
    records every such push in the session ledger when it runs, so the ledger names the lane. The
    lane behind waits for that lane's merge or its withdrawal, rebases on `main` and runs the dry run
    again. A remote version that no lane's push recorded is not waited on: the controller stops and
    asks Quinn. `supabase migration repair` is never run to get past a refusal; it runs only for M6's
    rename of a frozen file and in a withdrawal. A lane does not merge its migration until a dry run
    on its lane branch, rebased on `main`, lists its own version alone (or nothing, once it has
    pushed from that branch).

  That keeps staging's history in stamp order, so no planned push uses `--include-all`. Two waits,
  spelled out:
  - L1's C1 waits until each of L3, L8 and L5 is cleared: **merged** (C3 Q8, ruled 2026-10-19) with
    its migration on staging, or, having neither merged nor pushed, recorded as slipped. That is
    stricter than Q8's words in two ways, and Quinn hears both at the next checkpoint: Q8 names merges
    only, and Q8 lets the controller record any lane's slip, where M7 allows it only for a lane that
    has neither merged nor pushed (a withdrawn lane counts as one). Under the merge-with-push rule the
    first adds a wait only between a merge and its push; the second holds C1 behind a lane that pushed
    before merging until it merges or is withdrawn. L8 always pushes before merging (its S7a), so C1
    also waits for Quinn's go on L8's merge (its S6).
  - L8's S7a pushes from its lane branch only once L3 is cleared (L8 spec §8.10). L3 having merged is
    not enough, though under the merge-with-push rule a merged L3 is on staging moments later. L3
    having pushed from its lane branch (L3's Q-G proof-first) is not enough either: S7a then waits for
    L3's merge or L3's withdrawal.

  L3 is first after the email branch, and its T0 branches only after the email merge, so it waits
  only for the email branch's migrations to be on staging, which M3's read of staging at T0 shows. At
  T0 the controller also checks that every version staging has applied has a file on `main`. If one
  has none, it belongs to no lane on record, and the controller stops and asks Quinn before L3 fixes
  its stamp. After that check, every version on staging that `main` lacks is a lane's pre-merge push,
  recorded in the session ledger when it ran.
- **M8. A slip, and prod.** A lane recorded as slipped (only a lane that has neither merged nor
  pushed can be, M7) drops behind the lanes that overtake it.
  - M5 on its lane branch, before its merge, gives it a stamp after theirs.
  - A lane that pushed from its lane branch cannot be overtaken while its version is on staging: until
    it merges it holds every lane behind it (M7), so it is never recorded as slipped, and its frozen
    stamp keeps its place. Once the controller withdraws it (M7), it has neither merged nor pushed, and
    the bullet above applies.
  - **If M7's merge-with-push rule is ever broken** (a migration merged and its push did not follow),
    the controller's re-stamp commit on `main` is the remedy, before that migration's first staging
    push. One commit: the `git mv` to the next free stamp after M3's three sets (same suffix), the
    pins recounted, both stamps in the session ledger and the lane's ledger. It lands as a one-commit
    PR off `main` on green CI, and the push is then a plain `db push` from `main`. Staging never
    applied the old stamp, so the file is not frozen and no `migration repair` runs (M6 is only for a
    version staging holds). The same remedy answers a staging push that asks for `--include-all`
    because a merged, unpushed migration sorts too early. A re-stamp fixes the order only: the file is
    still on `main`, so M7's dry run stops every push from a branch that carries it until its own push
    is allowed and done, and the controller tells Quinn. If prod already holds the old stamp, the file
    is frozen there: the controller asks Quinn instead.
  - **If the broken merge sits behind a lane that pushed from its lane branch and has not merged**,
    the re-stamp cannot help. The lane ahead's merge dry run lists the stray file, and the stray file's
    push from `main` refuses on the lane ahead's version, so each waits on the other. The controller
    pushes nothing from `main` and offers Quinn two remedies; the choice is Quinn's:
    - **Revert the broken merge** in one commit on `main`. The reverted lane is then neither merged
      nor pushed again (M5 runs on its lane branch before its next push). It lands again by reverting
      that revert in its next merge, or with its commits re-applied on a branch cut from `main`. A
      plain rebase of the old lane branch onto `main` drops them silently, because git counts them as
      merged.
    - **Withdraw the lane ahead** (M7), which rewrites nothing on `main`. The controller records the
      withdrawn lane's slip and pushes the stray migration from `main` once its own push is allowed
      (its dry run then lists it alone). The withdrawn lane then re-stamps by M5 on its lane branch
      and pushes again, re-running its staging proof.

  Prod takes migrations only from `main`: L9's parity pass, then one push per release, each on Quinn's
  go. Each is a **plain** `supabase db push` (C3 Q8). Before each prod push, the controller checks that
  every migration on `main` not yet applied on prod sorts after prod's newest applied version. If one
  does not, the controller re-stamps it by M6 before the push, or asks Quinn. The controller never adds
  `--include-all` to a prod push on its own.

**What the rule means for L3.**
- **The file is `cloud/supabase/migrations/<stamp>_event_allowance.sql`.** T0 fixes `<stamp>` by M3.
  Only the email branch is ahead of L3, so it is today's `YYYYMMDD000100` unless something on `main`
  or staging sorts at or after that. Before fixing it, the controller reads L8's, L5's and L1's ledgers
  and worktrees. If one of them already holds that exact stamp, L3 takes the next free one and the
  other lane re-checks by M5. T0 records the stamp in the lane ledger. T1 uses it as recorded, and the
  controller passes it to L8, L5 and L1 at dispatch (Q-F).
- **The email branch's last stamp is `20261005000100`.** The 33 files on the email branch are main's 31
  plus `20261002000200_mail.sql` and that one.
- **In the expected order, nothing is renamed.**
- **L3's first staging push is T5's push step.** By M7, L3 merges only once Q-E allows that push, and
  the controller runs the push from `main` right after the merge. If Quinn asks to see the proof first
  (Q-G), T5 pushes from the lane branch before the merge instead, so the freeze (M6) comes before the
  merge. Either way M5's re-check runs on the lane branch, rebased on `main`, immediately before the
  merge or the pre-merge push. Only if the merge went ahead without its push (M7 broken) does the
  check run on `main`, with M8's re-stamp commit as the rename.
- **Until T5 has pushed and L3 has merged, L3 holds L8's S7a, L5's push and L1's C1 (M7).** While
  L3 has neither merged nor pushed, the controller may record its slip instead. Once L3 has merged,
  only its push clears it, and M7 makes that push follow the merge at once. Once T5 has pushed from
  the lane branch (Q-G proof-first), only L3's merge clears it, unless the controller withdraws it
  (M7): until then every dry run on `main` refuses on L3's version. So Q-E gates the lanes behind L3,
  not only L3's proof, and so does Q-G if T5 pushes first; L3's merge waits for Q-E as well as Q-G
  (§7 item 2).
- **If L3 slips** (Q-G or Q-E unanswered, so L3 is unmerged, and T5 has not pushed or the controller
  has withdrawn it; M7) and L8, L5 or L1 overtakes it, M8 applies: L3 re-stamps at M5 on its lane
  branch before its merge and recounts `parsed` (`migrations_test.ts:333`).
- **If T5 pushed from the lane branch before the merge**, L3 cannot slip: until Quinn's Q-G word
  merges it, it holds L8's S7a, L5's push and L1's C1 (M7), so nothing overtakes it and the name
  stays. If the proof shows a defect in L3's SQL, the controller withdraws L3 by M7 on its own and
  tells Quinn; if Quinn declines the merge, that is Quinn's word, and the controller withdraws L3 too
  (T5's withdrawal step). Anything else goes to Quinn first, and a defect in the functions alone is
  fixed on the lane branch and redeployed from it, with no withdrawal (M7, Quinn's ruling of
  2026-10-19). After a withdrawal L3 is a lane that has neither merged nor pushed, and the bullet
  above applies.
- **If L3 merged without T5's push** (M7 broken), the controller re-stamps it on `main` by M8 before
  T5's push, and M7's dry run stops every push behind it until T5 has pushed. If a lane that pushed
  before merging sorts ahead of L3 by then (L3 having slipped), the re-stamp cannot help, and M8
  offers Quinn two remedies: a revert of L3's merge, or a withdrawal of that lane, after which L3
  pushes from `main` and the withdrawn lane re-stamps and pushes again.

## 3. Open questions (for Quinn at a checkpoint, or the controller where marked)

These came out of reading the code. None reopens a ruled decision. Each has a recommendation, and
the plan states what happens if it is not answered.

**Q-A (Quinn). The task cap can stop the event pass before the allowance is ever reached.**
- **What the code does.**
  - One `CloudModel` serves both the task batch and the events pass (`engine/src/enrich.rs:360`,
    `:412`, `:432-439`).
  - A `capped` reply sets the model's single `fatal` flag (`cloudmodel.rs:425-427`).
  - After that, every later call on any path answers `ModelError::Capped` without a request
    (`cloudmodel.rs:438-444`).
- **The consequence.** On a slot where the **task** cap (60, or 120 on the first two days) is spent
  first, `judge_roster` sends no `/judge-event` request at all.
  - That slot's events never reach the server's event cap (a separate 80 a day), so they never reach
    the allowance either.
  - This hits hardest on the account the allowance exists for: an old account with a new profile has
    no doubling, and a semester's task backlog over 60 is plausible.
  - It is a cross-kind short-circuit that exists today, and it predates this spec.
- **Recommendation: include T3b.**
  - A `capped` reply short-circuits later calls on **the same judge path only**.
  - 401, 402 and 403 stay global.
  - `fatal()` keeps naming the cap only when the task batch was capped, so `enrich.rs`'s closing line is
    unchanged and `enrich.rs` is not edited (it is L6's ground).
  - Size S, in `engine/src/cloudmodel.rs` plus one assertion in `engine/tests/cloud_contract.rs`.
- **If not ruled by the time T3 is green:** T3b is left out and filed as a named follow-up in the lane
  ledger. The rest of the lane does not wait on it.

**Q-B (controller). Is `engine/tests/cloud_contract.rs` on the contract list?**
- CLAUDE.md lists "the oracle/sync/entitlement tests" and does not name this file. T3b changes one of
  its assertions: `a_capped_event_reply_still_stops_the_batch_rather_than_becoming_unsure`,
  `cloud_contract.rs:370-381`, asserts `model.fatal() == Some(CAPPED_LABEL)` after an **event** cap.
- **Recommendation:** treat it as **off** the list. It pins the device-to-service wire shapes, not vault
  bytes, and T3b keeps the shape (an event cap is still `ModelError::Capped`); only which later calls
  short-circuit changes. If the controller rules it **on**, T3b goes to `contract-engineer` and
  `contract-reviewer`.

**Q-C (Quinn; recommendation already taken). Spec §9's SQL tests 1-3 cannot run "against the local
database".**
- The repo has none. `migrations_test.ts:1-3` reads "There is no Docker in this plan, so nothing here
  applies SQL", and nothing under `cloud/` opens a database.
- **Plan as written:** three kinds of proof, each covering a different part. **Only (i) touches the
  SQL's rules.**
  - (i) Static pins on the statement's exact text in `migrations_test.ts`, as the mail allowance is
    pinned (`migrations_test.ts:1480-1533`). These are the only proof of: the 14-day expiry, never
    reopening, the `calls_left < 2000` / `refunds < 2000` refund bounds, and concurrency (spec test 3).
  - (ii) Behaviour of the **pipeline** against a TS fake (T2a/T2b). The fake's allowance is one boolean
    answer (`allowanceAllow`). It proves the charge order and the refund routing. It does **not** model
    expiry, reopening or the refund bounds. No task builds a TS model of the SQL. A model like that
    would test itself, not Postgres.
  - (iii) The live staging proof (T5). It reads the real row after real slots, so it checks the opening
    row and `calls_left` falling. It never reaches day 15, the refund bounds or two concurrent first
    charges. So it does not exercise expiry, the bounds or concurrency either.
- **What Quinn should know.** Expiry, no reopening, the refund bounds and concurrency are proved by
  text alone. A pin passes on text. If the SQL means something else (for example, an `or` slipped into
  the `where`, changing precedence), nothing behavioural catches it before prod. Concurrency is proved
  by construction: one `insert … on conflict … do update … where` statement on the primary key. A local
  Postgres harness would cost RAM the session does not have (handoff §3), so it is left out.
- **Option for Quinn (Q-C2, recommended): a rolled-back staging probe in T5.** The controller runs
  one SQL session against staging, inside `begin; … rollback;`, on an account id with no
  `judge_allowances` row. Use a generated id inserted in the same transaction if `public.accounts`
  accepts one there; otherwise use Profile A's account before its first overflow. The controller
  checks which before running.
  - charge once and see the row open;
  - set `until` into the past and see `charge_judge_allowance` and `refund_judge_allowance` both
    answer `false`, with no new row;
  - put `calls_left` at 2000 and see a refund answer `false`;
  - `rollback`, then confirm by a count that nothing stayed.

  This runs the real Postgres on the rules (i) only pins. It costs about ten minutes and leaves no
  row. It writes to the staging database inside a transaction, so it needs Quinn's word with Q-E.
  It does not prove concurrency. **If not ruled,** T5 skips it and the lane ledger records the rules
  as "pinned, not exercised".
- **Quinn may overrule** (ask for a harness); that would be a new task, not a change to these.

**Q-D (controller ruling, recorded here). The column `left` is renamed `calls_left`.**
- The spec names the table's column `left` (§2). `LEFT` is a reserved word in Postgres (as in
  `left join`), so the column would need double quotes in every statement and every static pin, and an
  unquoted slip is a syntax error.
- The plan uses `calls_left`. `opened_at`, `until`, `refunds` and `kind` keep the spec's names
  (`until` is non-reserved).
- This is a name only, and the fidelity ledger records it as a deviation.

**Q-E (Quinn, at T5). The staging deploy.** T5 needs Quinn's word to deploy the migration and the
functions to staging. The handoff's merge-authority ruling (§1.7) keeps prod deploys for Quinn. Staging
pushes are the controller's, and the spec's T5 asks for Quinn's word, so the plan asks. Profile A
(spec §9 "a **new** account") also needs a sign-in address Quinn approves for a fresh staging account.
Prod reaches this migration only through L9, on Quinn's go. By §2.1's M7 a migration merges only with
its staging push, so the lane PR's merge (Q-G) also waits for this word, unless T5 has already pushed
from the lane branch.

**Q-F (controller). Two other lanes' specs overlap this lane's files.** Citations re-read on
2026-10-19 (plan review round 3, minor 2). They are given by section, because both specs are still
being revised.
- **L8's spec** (`docs/specs/2026-10-18-friends-testers-design.md`) is already aligned on two points,
  so neither needs a message:
  - §7 Q2 says L3 neither edits nor re-issues `enforce_budget`. Its revision-3 note records the fix.
  - §8.4 adopts L3's stamp, and it now carries §2.1's shared rule word for word.

  §7 Q4's alternative (`record_tokens` cost columns) names `judge_caps.ts` as L3's file, which is
  right.
- **L5's spec** (`docs/specs/2026-10-05-syllabus-class-times-design.md`):
  - T6a (§13's task table) edits `judge_caps.ts` (`Kind` gains `syllabus`, and §7 adds a `DAILY_CAP`
    row at `judge_caps.ts:45`), edits the `judge_caps_test.ts:11` pin, and widens the kind checks in
    its own migration.
  - §16.8 and §17 Q41 already rule out a `judge_allowances` row for `syllabus`.
  - T5 (§13; §4.3 names the client) edits `engine/src/cloudmodel.rs` and `engine/src/enrich.rs` to add
    a `/judge-syllabus` path beside the three existing ones, as its own pass in `enrich.rs`. T3b (if
    Q-A is ruled yes) changes how `cloudmodel.rs`'s `call` and `verdict_of` treat a `capped` reply, and
    what `fatal()` answers (T3b Behaviour).
  - §13 stamps T6a's migration "after L8's and before L1's", which matches M1.
- **Recommendation:** the controller tells L8's, L5's and L1's planners at dispatch:
  - L3 merges first in the expected order, and the other lanes rebase on it (§4).
  - **L3's migration stamp, exactly as T0 recorded it in the lane ledger**, and the shared rule of
    §2.1 (M1-M8) by name. The same-day `<date>000100` default is not safe on its own.
  - L5's kind-widening migration must not widen `judge_allowances.kind`. Spec Q4 rules the allowance
    events-only, T1's pin [4] fails if the kind is widened, and L5's Q41 already agrees.
  - **`CapStore` gains three methods** (T2a: `chargeOutcome`, `chargeAllowance`, `refundAllowance`),
    and the pipeline calls `chargeOutcome`, not `charge`. Any `CapStore` a later lane adds (a fake or
    a store) implements all three. `charge` keeps its boolean, fail-closed contract (`false` on a
    refusal **or** an error), so a caller that needs to tell the two apart calls `chargeOutcome`.
  - **To L5 only, if T3b lands:** a `capped` reply then stops only its own judge path, and `fatal()`
    names the cap only for the paths on the enrichment batch's list (`/judge-task`, `/judge-email`;
    T3b). `/judge-syllabus` is not on that list, so a capped syllabus call stops later syllabus calls
    and leaves `enrich.rs`'s closing "the rest of the batch was not sent" line alone. L5 decides
    whether its pass reports its own cap (as T3 does for events) and adds its path to the list only if
    the syllabus pass is part of that batch. L5's edits to `call` rebase onto T3b's.
- No task in this plan edits either spec. L8's §8.4 took the shared rule in the same 2026-10-19
  docs pass that wrote §2.1, outside this plan's tasks.

**Q-G (Quinn). The lane PR's merge.**
- The lane touches money (§1): it opens a new spend path on the judge route. Handoff §1 item 7 and
  progress.md's 2026-10-18 row both keep "anything touching money" with Quinn, so T4 PASS plus green
  CI is **not** enough to merge. The 2026-10-06 ruling settled the design, not the merge.
- **Recommendation:** the controller asks Quinn for the merge in one message with Q-E (and Q-C2),
  carrying T4's report path, the CI result and the spend bound (2,000 calls, at most 4,000 requests
  with refunds, about $0.22 an account once, $2 ceiling unchanged). It also says that the PR runs
  `ci.yml`'s `eval-gate`, because T2b touches `judge_pipeline.ts`: an existing, bounded gate that
  spends nothing while staging's `eval_cases` is empty (T2b CI note). Quinn may also ask to see T5's
  staging proof first; T5 can run from the lane branch before the merge, because no device behaviour
  depends on the server change.
- **If not answered** (Q-G, or Q-E while T5 has not pushed, because by §2.1's M7 the merge waits for
  both): the PR stays open and green, and the controller does not merge it. If T5 has not pushed,
  then rather than hold L8 and L5 behind it, the merge order may move. The controller records L3's
  slip in the session ledger (L3 has neither merged nor pushed, so M7 allows it), the other lanes land
  first, and L3 follows M8 of §2.1, at the cost of one rename on the lane branch and a `parsed`
  recount. If T5 pushed from the lane branch, L3 cannot be recorded as slipped (M7): it holds L8's
  S7a, L5's push and L1's C1 until Q-G's merge, so the controller tells Quinn that before the
  pre-merge push. If the proof then shows a defect in L3's SQL, the controller withdraws L3 on its
  own, records it in the session ledger and tells Quinn; if Quinn declines the merge, that is Quinn's
  word, and the controller withdraws L3 (§2.1 M7; T5's withdrawal step). Either way L3 may then be
  recorded as slipped. A defect in the functions alone is no withdrawal: it is fixed on the lane
  branch and redeployed from it. While Q-G stays unanswered after a pre-merge push, the controller
  asks Quinn whether to withdraw L3 before it does (Quinn's ruling, 2026-10-19).

## 4. Conflict map (what this lane owns, and what it must not touch)

| File | L3 task | Other lanes | Rule |
|---|---|---|---|
| `cloud/supabase/migrations/<stamp>_event_allowance.sql` | T1 (new) | L8, L5, L1 add their own | the shared stamp rule of §2.1 (M1-M8) |
| `cloud/supabase/migrations/migrations_test.ts` | T1 | L8, L5, L1 also append | append-only blocks at the file end; on rebase keep both, recount `parsed` |
| `cloud/supabase/functions/_shared/judge_caps.ts` | T2a | **L5** T6a (`Kind` gains `syllabus`, a `DAILY_CAP` row at `:45`); **L8** only if Quinn takes its Q4 alternative (`record_tokens` cost columns) | L5 and L8 rebase after L3 (merge order email → L3 → L8 → L5). L3 adds three `CapStore` methods, a `ChargeOutcome` type and two constants; the only existing lines it changes are `charge`'s body (now `chargeOutcome(…) === "charged"`, same answers). None of those is a line L5 or L8 edits, so a rebase keeps both sides. L3 does not touch `enforce_budget` (tell L8, Q-F) |
| `cloud/eval/run_eval.ts` | T2a (`EVAL_CAPS` at `:93` gains the three methods) | none found | three added lines in the literal; nothing else. `deno check` in CI covers `cloud/eval/*.ts`, so leaving it out fails CI |
| `_shared/judge_caps_test.ts` | T2a (appends tests) | **L5** T6a edits the `DAILY_CAP` pin at `:11` | L5 rebases after L3. L3 edits no existing pin; on conflict keep both |
| `_shared/judge_pipeline.ts`, `judge_pipeline_test.ts` | T2b (T2a extends the `Caps` fake) | none found in L5's or L8's spec | L3 owns. L5's new kind reaches the pipeline only through `Kind`, and T2b's `req.kind === "event"` test is unaffected |
| `_shared/judge_handler_test.ts` | T2a (one fake gains two methods) | none found | L3 owns |
| `cloud/supabase/functions/gmail-read/handler_test.ts` | T2a (one fake gains two methods) | none (L0 merged first) | a five-line edit at :90-96 only |
| `engine/src/events.rs` | T3 | **L6** indirectly: `events.rs:483-497` imports `crate::judge::{EventModel, EventItem, one_line, clip, MAX_BODY_CHARS}`, T3 adds `crate::judge::CAPPED_LABEL`, and L6 owns `judge.rs` (handoff §5). L7 and L5 do not list `events.rs` | Whichever of L3 and L6 lands second rebases. If L6 lands first and moves or renames an item `events.rs` imports, T3 imports it from its new home on rebase and says so in the hand-off. If L3 lands first, L6's import edit in `events.rs` rebases over T3's diff. T3 changes no import line except to add `CAPPED_LABEL` |
| `engine/src/cloudmodel.rs`, `engine/tests/cloud_contract.rs` | T3b only | **L5** T5 adds a `/judge-syllabus` path to `cloudmodel.rs` (Q-F). **L6** removes `runtime.rs` and edits `judge.rs` and `enrich.rs` (handoff §5); it may touch `cloudmodel.rs` where it names the runtime. **L7** (handoff §5, "OpenRouter 402" as a named issue) is likely to touch how `cloudmodel.rs` maps a 402 and its `fatal` label, which is the same `call` and `fatal()` code T3b rewrites | T3b edits neither `judge.rs` nor `enrich.rs`. Whichever of L3, L5, L6 and L7 lands later rebases on the earlier. L5 rebases onto T3b's capped rule (Q-F). Between L3 and L7, whichever lands second keeps **402 global** (T3b's `a_402_still_stops_every_path` and L7's named issue both hold), and keeps the capped state per path |
| `HANDOFF.md`, `cloud/supabase/README.md`, `docs/reference/engine-commands.md` | T6 | Docs lane (docs-keeper) refreshes HANDOFF | T6 lands after the Docs lane's refresh, or hands its lines to it |
| `engine/src/judge.rs` | read only (`ModelError`, `CAPPED_LABEL`, and the items `events.rs` imports) | **L6 owns and edits it** | no L3 edit; on rebase, follow L6's names (the `events.rs` row) |
| `engine/src/enrich.rs` | read only | L6's runtime-arm removal; L5's T5 syllabus pass | no L3 edit |

`main.rs`, `commands.rs`, `account.rs`, `approvals.rs` and `console.js` are untouched. The lane adds no
Tauri command and no window copy.

## 5. Tasks at a glance

T1, T2a and T2b are the cloud lane and run in that order: T2a's mirror test [11] reads T1's migration,
and T2b's tests use the fake T2a extends. T3 and T3b are the engine lane. The two lanes' files are
disjoint, so they run in parallel. Only the engine lane compiles, and T3 and T3b run back to back,
never at once.

T1 stays one task, though it is at the size limit. Under Q-C its pins are the only proof of the
SQL's rules, so the SQL and its pins land in one diff and are reviewed together.

| # | Task | Agent | Why that agent | After |
|---|---|---|---|---|
| T0 | Branch, worktree, commit the spec as Quinn left it | main session | branching and commits are the controller's | the email merge |
| T1 | Migration: `judge_allowances`, `charge_judge_allowance`, `refund_judge_allowance`; spec tests 1-7 | `cloud-engineer` | SQL on the money path, under row locks; `cloud/` is its lane | T0 |
| T2a | Store: `chargeOutcome` (charged / refused / error), `chargeAllowance`/`refundAllowance`, the two constants and the cost comment, the four `CapStore` implementations outside the store; spec test 11 and the store tests | `cloud-engineer` | the money path's store, under the breaker; `cloud/` is its lane | T1 (test [11] reads its file) |
| T2b | Pipeline: overflow order for `event` and refund routing; spec tests 8-10 | `cloud-engineer` (a fresh dispatch) | the charge order and refund routing, reviewed as their own diff | T2a |
| T3 | `judge_roster` ends at the first `Capped` with one line and an exact `left` (Q9); spec tests 12-13 | `implementer` | fully specified, off the contract list, checked by tests | T0 |
| T3b | *(only on Q-A yes)* per-path cap short-circuit in `CloudModel` | `implementer` (or `contract-engineer` if Q-B rules the test file on the list) | small, specified, compiler- and test-checked | T3 |
| T4 | Review T1, T2a, T2b, T3 and T3b (if built) before any push; then the merge ask (Q-G) | `reviewer` (+ `contract-reviewer` only if Q-B = on); the ask is the main session's | cheaper-agent and cloud changes pass review; money path | T1-T3(b) |
| T5 | Deploy to staging; live proof (spec §9 "Live") | main session | staging pushes, sessions and live reads are the controller's; Quinn's word (Q-E) | T4 PASS |
| T6 | Docs: README RPC list, HANDOFF line, engine-commands sentence | `docs-keeper` | docs only | T5 |

Lane PR: T4 PASS and green CI make the PR **ready**, not merged. **The merge waits for Quinn's word**
(Q-G), because the lane touches money, which handoff §1 item 7 keeps with Quinn, as it does the email
merge and prod deploys. T5 and T6 can land in a docs-only follow-up commit if the PR merges first. The
staging proof does not gate the merge unless Quinn asks to see it first (Q-G); no device behaviour
depends on the server change. **The prod deploy is L9's, on Quinn's go.**

---

## T0. Branch and spec (main session)

- [ ] After the email-forwarding merge lands on `main`, create branch `l3-first-day-allowance` and its
      worktree (§2).
- [ ] Copy `docs/specs/2026-10-06-first-day-event-allowance-design.md` from the main checkout into the
      worktree byte for byte, header included. Commit it alone ("docs: first-day event allowance spec,
      ruled 2026-10-06"). If Quinn has signed the header by then, commit the signed copy.
- [ ] Read staging's applied versions (`supabase migration list` on staging) and check that each has
      a file on `main` (§2.1 M7). If one has none, stop and ask Quinn before fixing the stamp: no lane
      on record owns it, so no dry run can be read against it.
- [ ] Fix the migration stamp by §2.1 (check the other lanes first), then open the lane ledger
      `.superpowers/sdd/2026-10-18-l3-first-day-allowance/progress.md` with this plan's path and that
      stamp. T1 uses it unchanged, and the controller passes it to L8, L5 and L1 at dispatch (Q-F).

## T1. The migration (cloud-engineer)

**Files.** Create `cloud/supabase/migrations/<stamp>_event_allowance.sql` (§2.1). Modify
`cloud/supabase/migrations/migrations_test.ts` by appending one block at the end of the file, plus the
`parsed` count at :333 and its comment.

**What the SQL does** (spec §2, §3 step 3, §5; Q1, Q2, Q3, Q4, Q10):

- **Table `public.judge_allowances`.**
  - Columns:
    - `account_id uuid not null references public.accounts (id) on delete cascade`
    - `kind text not null check (kind in ('event'))`
    - `opened_at timestamptz not null`
    - `until timestamptz not null`
    - `calls_left integer not null check (calls_left >= 0)`
    - `refunds integer not null default 0 check (refunds >= 0)`
  - `primary key (account_id, kind)`.
  - `alter table … enable row level security; revoke all on public.judge_allowances from anon,
    authenticated;` with no policy.
  - A header comment says the row is **never deleted while the account lives**, which is what makes it
    one-time (the opening test is "no row", never "row expired"). No migration and no function deletes
    from it; only the cascade does.
- **`public.charge_judge_allowance(p_account uuid, p_kind text) returns boolean`.**
  - Declared `language plpgsql`, `security invoker`, `set search_path = public, extensions`.
  - **One write and no read before it.** It inserts the opening row (`opened_at = now()`,
    `until = now() + interval '14 days'`, `calls_left = 2000 - 1`, `refunds = 0`). On conflict on
    `(account_id, kind)` it does `update set calls_left = judge_allowances.calls_left - 1 where
    judge_allowances.until > now() and judge_allowances.calls_left > 0`, then
    `returning true into charged; return coalesce(charged, false);`.
  - **The conflict arm's column references stay qualified** (`judge_allowances.calls_left`,
    `judge_allowances.until`). Inside `do update`, both the existing row and `excluded` are in scope,
    so an unqualified `calls_left` on the right of `set` or in the `where` risks an "ambiguous column"
    error. In plpgsql it could also collide with a variable. The pins follow the qualified SQL; the
    SQL is never unqualified to suit a pin. Only the `set` target (`set calls_left = …`) is unqualified,
    because Postgres does not accept a qualified target there.
  - **It is one statement and stays one.** Never write it as an `update` followed by an `insert` when
    not found. Two concurrent first charges would then race. One would hit the primary key and throw,
    and the rpc error would make `chargeAllowance` answer `false`. That event would be answered
    `capped` while the allowance had room.
  - Write the size as `2000 - 1`, so the size appears as `2000`. The T1 pin and T2a's size-mirror test
    both use that form.
- **`public.refund_judge_allowance(p_account uuid, p_kind text) returns boolean`.**
  - Same header shape. One `update public.judge_allowances set calls_left = calls_left + 1,
    refunds = refunds + 1 where account_id = p_account and kind = p_kind and until > now() and
    calls_left < 2000 and refunds < 2000 returning true into done; return coalesce(done, false);`.
  - It never inserts, and never sets `opened_at` or `until`.
- **Privileges.** For both functions: `revoke execute … from public, anon, authenticated; grant execute
  … to service_role;`.
- **What it does not do.** No `create or replace` of `charge_call`, `refund_call` or `enforce_budget`
  (Q5, Q6). Its text, comments included, never uses the word `models`: `ci.yml:246-248` runs the
  paid eval gate for any migration that matches `\bmodels\b`, and this one changes no pin (say "model
  calls", never the plural noun on its own).

**Tests.** These are static pins in the house style (Q-C). A helper finds the file by
`^\d{14}_event_allowance\.sql$` and normalises whitespace. Each test name says what it pins.

- [ ] **Step 1: write the tests, run them, see them fail** (the file does not exist yet). Spec test
      numbers are in brackets.
  - `event_allowance_opens_on_the_first_overflow_with_2000_minus_one_and_14_days` [1]
    - The one `insert` sets `opened_at = now()`, `until = now() + interval '14 days'`,
      `calls_left = 2000 - 1` and `refunds = 0`.
    - The function answers `coalesce(charged, false)` from `returning true`.
  - `event_allowance_is_one_time_and_never_reopens` [2]
    - The `on conflict (account_id, kind) do update` sets **only** `calls_left`, as
      `set calls_left = judge_allowances.calls_left - 1` (no `opened_at`, `until` or `refunds` in the
      update's `set`).
    - Its `where`, after whitespace normalisation, is exactly
      `where judge_allowances.until > now() and judge_allowances.calls_left > 0` followed by
      `returning`. The pin takes everything between `do update` and `returning` and compares it to the
      expected `set … where …` text. That way an extra `or` clause, or a reordered precedence, fails
      the pin.
    - There is no `interval '120 days'` anywhere in the file (the contrast with mail's
      `charge_allowance`).
    - No `delete from public.judge_allowances` exists in any migration.
  - `charge_judge_allowance_is_one_statement_on_a_unique_key` [3]
    - **Do not reuse the house count unchanged.** `migrations_test.ts:1488` counts
      `\b(insert|update|delete)\b`. That count is 2 for `insert … on conflict … do update`, because
      both words are in the one statement. Loosening it to "at most 2" would no longer guard one
      statement. Pin the shape instead, in the function body (between the `$$` markers, with `--`
      comments stripped first):
      - `\binsert\b` occurs exactly once, as `insert into public.judge_allowances`;
      - `\bupdate\b` occurs exactly once, and that occurrence is `do update` (the conflict arm). No
        standalone `update public.judge_allowances` statement exists;
      - no `\bdelete\b`, no `\bselect\b`, no `\bif\b` and no `found` (no branch on a prior result);
      - between `begin` and `end` there are exactly two statements ending in `;`: the insert and the
        `return`.
    - The table's `primary key (account_id, kind)` is the conflict target.
    - `security invoker` and the pinned `search_path`. A comment in the test says why one statement is
      the concurrency proof: the second of two first charges waits on the key, then takes the update
      path; Postgres re-checks the `where` on the locked row.
  - `judge_allowances_kind_is_checked_to_event` [4]: the column carries `check (kind in ('event'))`
    and no other word.
  - `refund_judge_allowance_gives_back_only_while_open_and_within_both_bounds` [5]
    - One `update`, no `insert`, no `select`.
    - The `where` carries `until > now()`, `calls_left < 2000` and `refunds < 2000`.
    - The `set` is `calls_left = calls_left + 1, refunds = refunds + 1` and nothing else.
  - `judge_allowances_is_service_role_only_and_purged_with_the_account` [6]
    - RLS enabled.
    - `revoke all … from anon, authenticated`.
    - No `grant` to either role.
    - `references public.accounts (id) on delete cascade`.
    - Each function has the revoke-and-grant pair: reuse `assertServiceRoleOnly`
      (`migrations_test.ts:1440`), with args `uuid, text`.
  - `the_event_allowance_migration_reissues_no_cap_function` [7]
    - The file defines no `charge_call`, `refund_call` or `enforce_budget`.
    - The existing "the last charge_call doubles the cap…" test (`:864`) and the F1 `refund_call` test
      (`:1722`) still pass **with no edit**. Re-running them is part of this step's green.
  - Also extend the cap-store list at `:483` with the two new names, so "the cap-store functions are
    revoked, not marked" covers them.
- [ ] **Step 2:** write the migration. Run the cloud suite and see the new pins green, with nothing
      else changed. **One existing test is expected red here:** the `parsed` guard at
      `migrations_test.ts:333` (`assertEquals(parsed, 40, …)`) now counts 42 and fails. That is not a
      defect; Step 3 moves the number. Every other existing test is green at this step.
- [ ] **Step 3:** the guard at `:284-334` now parses **42** function creations. Update the pinned
      number and append one clause to its comment naming the two new writing, non-definer,
      revoked functions. If another lane's migration landed first, recount on the rebased tree (§2.1).
- [ ] **Step 4:** §2's three cloud commands, CI's exact lines with every `--allow-*` flag `ci.yml:167`
      passes, are green, and `deno lint` is clean on `migrations_test.ts`. A narrower
      `deno test --allow-read` run is not this step: loopback tests fail under it for reasons that are
      not T1's. Hand the controller the file list and the message "cloud: first-day event allowance
      table and RPCs (spec 2026-10-06 §2, §3, §5)".

**Acceptance.** Every spec-§9 SQL test (1-7) has a named pin. No existing migration test is edited
except the count at :333 and the list at :483. The SQL has no account, school or person in it.

## T2a. The store (cloud-engineer)

**Runs after T1:** test [11] reads T1's migration file.

**Files.**
- Modify `cloud/supabase/functions/_shared/judge_caps.ts` and `_shared/judge_caps_test.ts`.
- Modify the four other `CapStore` implementations (a grep for `CapStore` and `charge:` on the lane
  branch confirms the list before editing; a missed one fails `deno check`):
  - `_shared/judge_pipeline_test.ts:36-63` (the `Caps` fake, extended as below; no test added here);
  - `_shared/judge_handler_test.ts:27-33` and `gmail-read/handler_test.ts:90-96`, three lines each;
  - `cloud/eval/run_eval.ts:93-99` (`EVAL_CAPS`, the eval suite's unmetered store), three lines.
- No assertion changes anywhere. `judge_pipeline.ts` is **not** in this diff.

**Behaviour** (spec §3 step 3, §5, §6; Q1, Q2, Q3, Q6):

- **Why a third answer.** Today `charge` (`judge_caps.ts:93-102`) catches every rpc error and answers
  `false`, the same answer as a real refusal ("a cap store that cannot be reached REFUSES"). That is
  right for the cap. But Q2 and spec §3 step 3 open the allowance only "when step 2 refused", so the
  pipeline must be able to tell a refusal from an error. Otherwise a transient 503 on `charge_call`
  alone, or a deploy that breaks its signature, would open the one-time allowance, or spend it, while
  the cap still had room. No later charge could undo that.
- **`CapStore` gains three methods and one type.**
  - `export type ChargeOutcome = "charged" | "refused" | "error"`.
  - `chargeOutcome(account, kind): Promise<ChargeOutcome>` makes the same one `charge_call` rpc
    `charge` makes today, with the same args. It answers `"charged"` for `true`, `"refused"` for
    `false`, and `"error"` for a thrown or rejected rpc **and** for any answer that is not a boolean
    (a real `false` is the only thing that reads as a refusal).
  - `charge` keeps its signature and its answers: it becomes `(await chargeOutcome(…)) === "charged"`,
    so it still fails closed. Its doc comment says it folds `"refused"` and `"error"` into `false`, and
    that the pipeline calls `chargeOutcome`.
  - An error is not a model failure: `chargeOutcome` never counts toward `tripped()`, as `charge`
    never did.
  - `chargeAllowance(account, kind): Promise<boolean>` calls
    `db.rpc("charge_judge_allowance", { p_account, p_kind })`. A database it cannot reach **refuses**
    (`false`); here a refusal and an error mean the same thing (`capped`), so it needs no third answer.
  - `refundAllowance(account, kind): Promise<void>` calls `refund_judge_allowance`. It never throws,
    and it counts one failure toward `tripped()` exactly as `refund` does. The breaker counts every
    `model failed`, whichever store charged it.
- **`judge_caps.ts` constants.** Add `EVENT_ALLOWANCE_CALLS = 2000` and `EVENT_ALLOWANCE_DAYS = 14`,
  exported and documented as **mirrors of the migration, never sent to it**: the SQL alone decides.
  - Add the cost paragraph spec §6 asks for: ~$0.00011 a call on the pinned Granite row, ~$0.22 for
    the whole allowance once, the $2 ceiling not raised and checked first.
  - **The comment states the arithmetic, not a test tally**, in the style of the existing CORRECTION
    paragraph (`judge_caps.ts:32-43`).
- **What does not change.** `DAILY_CAP`, `MONTHLY_CEILING_USD`, `MAIL_DAILY_CAP` and `BREAKER_FAILURES`
  keep their values, so the pins at `judge_caps_test.ts:6-26` and `:79-81` stay green with no edit.
- **The fakes.**
  - The `Caps` fake in `judge_pipeline_test.ts`:
    - gains `chargeOutcome`, which pushes to the same `charged` array `charge` pushes to and answers
      `"charged"` when `allow` is true, otherwise `"error"` when a new optional flag `capError`
      (default `false`) is set, otherwise `"refused"`;
    - gains a third answer `allowanceAllow`, defaulting to `false`, and two arrays,
      `allowanceCharged` and `allowanceRefunded`, that its allowance methods push to.
    - The defaults keep every existing pipeline test's meaning, before T2b (the pipeline still calls
      `charge`) and after it (the pipeline calls `chargeOutcome`, which fills `charged` the same way).
      T2b's tests use these.
  - The other three (`ALWAYS`, the `gmail-read` fake and `EVAL_CAPS`) always charge today, so
    `chargeOutcome` answers `"charged"`, `chargeAllowance` answers `false` and `refundAllowance`
    resolves. `EVAL_CAPS` keeps its header's rule ("always true, always a no-op"); written as
    `() => Promise.resolve(…)` like its siblings, for `require-await`.
  - `run_eval.ts` is run by `ci.yml`'s `eval-gate` with the staging service-role key; the edit is the
    three added lines of the literal and nothing else.

**Tests** (in `judge_caps_test.ts`, with the `FakeDb` at `:32-48`, left unedited, plus one new fake).

- [ ] **Step 1: write the tests, run them, see them fail** (the methods and constants do not exist
      yet). The spec test number is in brackets.
  - **A new fake, appended:** `ScriptedDb implements Db`, whose `rpc` answers per function name from
    a map: a value to resolve, or a rejection. It records its calls as `FakeDb` does. `FakeDb` is not
    edited.
  - `chargeOutcome tells a charge, a refusal and an error apart`
    - `charge_call` resolving `true`, `false`, a rejection (`PostgREST 503`) and `null` answer
      `"charged"`, `"refused"`, `"error"` and `"error"`.
    - Each makes exactly one rpc, `charge_call`, with the args `charge` sends today
      (`{ p_account, p_kind, p_cap: DAILY_CAP[kind] }`).
  - `charge still folds a refusal and an error into false`: the same four cases answer `true`,
    `false`, `false`, `false`.
  - `a charge_call error never counts toward the breaker`: `BREAKER_FAILURES` erroring
    `chargeOutcome`s leave `tripped()` false.
  - `chargeAllowance calls charge_judge_allowance with the account and the kind`: the rpc name and
    args are exactly `{ p_account, p_kind }`.
  - `a chargeAllowance that cannot reach the database refuses`: `FakeDb(true)` answers `false`.
  - `refundAllowance calls refund_judge_allowance and never throws` (both `FakeDb` modes).
  - `the breaker counts refundAllowance failures too`: mix `refund` and `refundAllowance` up to
    `BREAKER_FAILURES`, and `tripped()` is true; a `recordTokens` closes it.
  - `EVENT_ALLOWANCE_CALLS and _DAYS are the numbers the migration opens with` [11]
    - Read the migration by the `^\d{14}_event_allowance\.sql$` pattern, as
      `mail-process/worker_test.ts:959-967` does.
    - Assert `2000 - 1` and `interval '14 days'` match the two constants.
    - Assert `calls_left < 2000` and `refunds < 2000` in the refund function match
      `EVENT_ALLOWANCE_CALLS`.
- [ ] **Step 2:** implement in `judge_caps.ts`. Then extend the four implementations as above. With
      that, `deno check` passes and every existing test keeps today's meaning.
- [ ] **Step 3:** run §2's three cloud commands (they cover `cloud/eval/`). All green, with no edit to
      any existing assertion. Hand the controller the file list and the message "cloud: CapStore tells
      a refusal from an error and can charge and refund the first-day event allowance (spec 2026-10-06
      §3, §5, §6)".

**Acceptance.**
- `chargeOutcome` answers `"refused"` only for a real `false` from `charge_call`; an error or a
  non-boolean is `"error"`. `charge`'s answers are unchanged.
- The two allowance methods call exactly the two RPC names with `{ p_account, p_kind }`; an
  unreachable database refuses an allowance charge and never throws on a refund.
- Every `CapStore` in the tree, `cloud/eval/run_eval.ts` included, implements the three methods.
- The breaker counts allowance refund failures.
- The constants are mirrors, pinned to the migration's text by [11], and nothing sends them.
- No existing test changed its assertions, and `judge_pipeline.ts` is untouched.

## T2b. The pipeline (cloud-engineer, a fresh dispatch)

**Runs after T2a** (its tests use the extended `Caps` fake).

**Files.** Modify `cloud/supabase/functions/_shared/judge_pipeline.ts` and add tests to
`_shared/judge_pipeline_test.ts`. Nothing else.

**Behaviour** (spec §3, §5; Q2, Q4, Q5, Q6):

- **The order for one request** (`judgeUngated`, today `judge_pipeline.ts:183-208`) becomes:
  - rules, then breaker, then `withinBudget`, then `chargeOutcome` (T2a), in place of today's
    `charge` at `:197`. `"charged"` goes on exactly as a `true` from `charge` does today.
  - **Only if `chargeOutcome` answered `"refused"` and `req.kind === "event"`:** `chargeAllowance`.
  - **`"error"` never reaches the allowance.** It answers `capped`, logged exactly as today's refused
    charge is, whatever the kind. That keeps today's fail-closed answer and keeps Q2's "opened at the
    first event judgment the daily cap refuses": a cap store that could not answer has not refused.
  - If the allowance also refused, or the kind is not `event`: `capped`, logged exactly as today.
- **The pipeline remembers which store charged** (`"cap"` or `"allowance"`).
  - On `cause === "model failed"`, it calls `refundAllowance` for an allowance charge and `refund` for a
    cap charge, never both.
  - A refusal, a truncation or a failed validation calls neither (unchanged).
- **Header comment.** The order line at `judge_pipeline.ts:3-5` names the new step: "daily cap ->
  (event only, on overflow) first-day allowance".
- `gmail-read` (email) and `judge-task` never reach the new step.

**Tests** (in `judge_pipeline_test.ts`, using T2a's `Caps` fake).

- [ ] **Step 1: write the tests, run them, see them fail** (the pipeline never calls the allowance yet).
      Spec test numbers are in brackets.
    - `an_event_the_cap_refuses_is_charged_to_the_allowance_and_judged` [8]
      - The cap refuses and the allowance charges, so the model is called once.
      - The reply is `answered`, tier 3.
      - `charged` is `[[acct, "event"]]` and `allowanceCharged` is `[[acct, "event"]]`.
    - `a_task_or_email_the_cap_refuses_is_capped_and_never_asks_the_allowance` [8]
      - For both kinds: `capped`, tier 0.
      - `allowanceCharged` is empty, and the model is never called.
    - `an_event_both_refuse_is_capped_as_today` [8]
      - The cap and the allowance both refuse: `capped`, one logged row with `outcome: "capped"`.
    - `an_event_whose_cap_charge_errors_is_capped_and_never_asks_the_allowance` [8]
      - The `Caps` fake with `allow = false`, `capError = true` and `allowanceAllow = true` (the
        allowance would charge if asked).
      - `capped`, tier 0, one logged row exactly as a refused charge logs, the model never called.
      - `allowanceCharged` is empty.
      - **Green before Step 2**, because today's pipeline never asks the allowance. It is the guard
        that Step 2 does not spend on an error (an implementation that tests `!charged` turns it red),
        so it is written with the others and must stay green through Step 2.
    - `a_charge_call_error_on_the_live_store_never_opens_the_allowance` [8] (Q2, end to end)
      - The real `capStore` over a small `RpcDb` appended to this file: `charge_call` rejects
        (`PostgREST 503`), `enforce_budget` answers `true`, and `charge_judge_allowance` would answer
        `true`. Every other rpc (`record_tokens`, `refund_call`) resolves `null`.
      - An `event` request answers `capped`, and `RpcDb`'s calls hold `charge_call` and **no**
        `charge_judge_allowance`. (Green before Step 2, as the guard above.)
      - A second case where `charge_call` answers a real `false`: the allowance **is** asked and the
        event is judged. **Red before Step 2.** The pair proves the pipeline reads the store's third
        answer, not only the fake's.
    - `over_the_monthly_ceiling_neither_charge_is_asked` [9]
      - `withinBudget` is false: `capped`.
      - `charged` and `allowanceCharged` are both empty.
    - `a_failed_call_on_the_allowance_is_refunded_to_the_allowance` [10]
      - The cap refuses, the allowance charges and the model throws a transport error.
      - `allowanceRefunded` has one entry and `refunded` is empty.
    - `a_failed_call_on_the_cap_is_refunded_to_the_cap` [10]
      - The cap charges and the model throws: `refunded` has one entry and `allowanceRefunded` is
        empty.
    - `a_refusal_truncation_or_invalid_answer_on_the_allowance_refunds_nothing` [10]
      - Three cases: `ModelRefused`, a `max_tokens` error and an answer that fails `validate`.
      - Neither refund array grows.
    - `the_breaker_skips_both_charges` (regression): `tripped()` true means no `chargeOutcome`
      (the `charged` array stays empty) and no `chargeAllowance`.
- [ ] **Step 2:** implement in `judge_pipeline.ts`.
- [ ] **Step 3:** run §2's three cloud commands. All green, with no edit to any existing assertion.
      Hand the controller the file list and the message "cloud: judge-event spends the first-day
      allowance only when the daily cap really refused, refunds to whichever charged (spec 2026-10-06
      §3, §5)".
- **CI note for the controller.** Touching `judge_pipeline.ts` makes `ci.yml`'s `eval-gate` run on
  the lane PR (`ci.yml:242`). With `eval_cases` empty it scores nothing and spends nothing; if staging
  holds cases by then, it makes real model calls under its own `MAX_CASES` bound (well under a dollar,
  `ci.yml:205-208`). This is the gate working as designed, not a new spend; it is named in the Q-G
  merge ask so Quinn is not surprised by it. The migration's comments avoid the word `models`, so the
  migration does not trip the gate's second test (`ci.yml:246-248`) on its own.

**Acceptance.**
- The new step is `event`-only and overflow-only, and sits after `withinBudget`.
- **Only a real refusal opens or spends the allowance.** A `charge_call` error answers `capped` and
  never asks it (the two error tests).
- Refunds route to whichever store charged.
- No existing test changed its assertions.
- Nothing in the pipeline reads `EVENT_ALLOWANCE_*`: the SQL alone decides the size and the window.

## T3. The capped pass ends with one line (implementer)

**File.** Modify `engine/src/events.rs` only (`judge_roster` at `:406-535` and its test module).

**Behaviour** (spec §8 "one honesty fix", Q9; tests 12-13):

- **The first `Capped` ends the loop.** In the item loop, `Err(ModelError::Capped)` from
  `model.judge_event` sets a `capped` flag and **breaks before that item counts as processed**.
  - So the capped item and everything after it fall into `batch.len() - processed`.
  - Today's `processed += 1` sits above the call (`:481`). Move the count, or undo it on this arm; both
    keep the budget check unchanged.
- **No per-item `not judged` line for a capped item.** Every other `Err` keeps today's per-item line
  (`:500-503`), and still counts as processed.
- **The summary.** `left` stays `left_for_cap + (batch.len() - processed)`, which on a capped stop
  equals pending minus the items processed before the cap.
  - **What `left` counts:** every pending event **not reached** this run (past the per-run cap, past
    the budget, or at and after the capped item). It is not "every event not judged". An item that was
    reached and answered `Failed` (or was refused, or not recorded) counts as processed and is named
    by its own `not judged` line, so it is not in `left`, although it is retried next slot.
  - Example: five pending; item 1 `Failed`, item 2 judged, item 3 `Capped`. The lines are one
    `not judged` line for item 1 and `events: 1 judged, 3 left for the next slot (the daily judgment
    cap)`. Four events are unjudged, and the log accounts for all four across the two lines.
  - The formula is kept on purpose. Folding failed items into `left` would also change today's
    uncapped summary after a failure, and the regression test below pins `left` 0 there.
  - When `capped`, the summary line ends `" ({CAPPED_LABEL})"`, using `crate::judge::CAPPED_LABEL`
    (`judge.rs:190`, read only; L6 owns that file).
  - So the T18c case reads `events: 80 judged, 932 left for the next slot (the daily judgment cap)`.
  - When capped with nothing left (the last pending item was the capped one), the line still names it:
    `events: K judged, 1 left for the next slot (the daily judgment cap)`. `left` is at least 1 by
    construction.
- **Unchanged.** Exit codes (`judge` always exits 0), the budget check before each item, the per-run
  cap, the soonest-first order, the ledger writes and the `fatal()` line `enrich.rs` prints after the
  passes.
  - That closing `judge: the service answered the daily judgment cap, so the rest of the batch was not
    sent` line still prints when the event pass was the one capped. That is a known residual of T3
    alone; T3b removes it for the event-only case.

**Tests** (in `events.rs`'s `mod tests`, beside `the_per_run_cap_bounds_the_batch_and_says_what_is_left`
at `:1042`).

- [ ] **Step 1: a scratch vault with several pending events.**
  - Add a helper beside `scratch_vault_with_feed` (`:850`): `scratch_vault_with_n_events(name, n)`.
  - It writes one enabled ICS source whose feed holds `n` events with distinct UIDs, **listed out of
    chronological order**, with no verdict seeded. Every event falls inside the 90-day roster window of
    the test's pinned `today` and passes `prefilter_events`.
  - Copy the existing helper's date shape (`today = 2026-08-28`, events from `20260829`), which is
    green today, rather than computing from the wall clock.
  - Check that `load_discovered_events_at`'s `Timestamp::now()` drops nothing for these dates. The
    existing helper's events pass, which is the evidence. If it ever stops being true, that is a time
    bomb like 4123749's, and the fix is to pin, never to widen the dates.
  - Temp dir named with `std::process::id()`, removed at the end. It never points at the worktree.
- [ ] **Step 2: write the tests, run them, see them fail for the stated reason.** Spec test numbers are
      in brackets.
  - `a_capped_reply_ends_the_pass_with_one_line_and_an_exact_left` [12]
    - Setup: `n = 5`, `cap = 150`. A model answers `Ok(verdict("drop", …))` for its first `k = 2`
      calls and `ModelError::Capped` from call 3 on. It counts its calls in a `RefCell`, as `Counting`
      does at `:960`.
    - Exactly 2 ledger verdicts.
    - The model was called exactly 3 times.
    - No line contains `not judged`.
    - Exactly one line starts `events: `, and it equals
      `events: 2 judged, 3 left for the next slot (the daily judgment cap)`.
    - **Red today:** today the fake model is called 5 times. Items 3, 4 and 5 each get `Capped` and
      `continue` (`events.rs:498-503`), so there are **three** `not judged` lines. `left` is 0, so the
      summary is `events: 2 judged`.
  - `a_capped_reply_on_the_first_item_judges_nothing_and_leaves_everything` [12]
    - `k = 0`.
    - One call, zero verdicts, and the line `events: 0 judged, 5 left for the next slot (the daily
      judgment cap)`.
  - `the_events_judged_before_a_cap_are_the_soonest_pending` [13]
    - The fixture from the first test.
    - The two uids with verdicts are the two earliest `DTSTART`s, although the feed lists them in
      another order.
    - **Before writing this test, search `eventfeed.rs`'s tests for an existing pin of
      soonest-first** (`eventfeed.rs:133-134`, `:846`). If one exists and covers the merged list
      `judge_roster` reads, cite it in a comment and keep this test anyway: it is the one that pins the
      order **under a cap**.
  - `a_failed_call_still_names_its_item_and_does_not_end_the_pass` (regression)
    - `ModelError::Failed` on call 1, then `Ok` on the rest.
    - One `not judged` line for item 1, the remaining items judged.
    - No cap suffix on the summary, and `left` is 0.
  - The existing `the_per_run_cap_bounds_the_batch_and_says_what_is_left` and
    `the_events_pass_stops_at_its_wall_clock_budget_and_reports_the_remainder` stay green **unedited**.
- [ ] **Step 3:** implement in `judge_roster`. Update the doc comment above it (`:396-405`) with one
      paragraph: a capped reply ends the pass, and its remainder folds into the same line.
- [ ] **Step 4:** run `cargo build --workspace` and then `cargo test --workspace` in the foreground,
      each under `C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh` with
      the worktree's `--manifest-path`, exactly as §2 Gates writes it. Green, 0 warnings other than the accepted `.rsrc` line, the four
      by-design ignores unchanged. Hand the controller the file list and the message "engine: a capped
      event pass ends at the cap with one line and an exact left (spec 2026-10-06 Q9)".

**Acceptance.**
- The run log for a capped event pass is one summary line naming the cap, and `left` counts every
  pending event not reached this run. Items reached and failed keep their own `not judged` lines.
- No vault write changed, no frozen reference moved, and `events.rs` is the only file in the diff.

## T3b (conditional on Q-A). A cap on one kind stops only that kind (implementer)

**Runs only if Quinn says yes to Q-A.** Otherwise it is filed in the lane ledger as follow-up
"cross-kind capped short-circuit (cloudmodel.rs:425-444)" and this section is skipped.

**Files.** Modify `engine/src/cloudmodel.rs` and its tests, plus `engine/tests/cloud_contract.rs`
(one assertion and one new test). It does not touch `judge.rs` or `enrich.rs` (§4).

**Behaviour.**
- **`capped` is remembered per judge path.**
  - `verdict_of`'s `capped` arm records the path the reply came from (`/judge-task`, `/judge-event`,
    `/judge-email`).
  - `call` short-circuits to `ModelError::Capped` only for **that same path**.
  - A capped `/judge-task` therefore leaves `/judge-event` free to reach the server, and the server's
    event cap and allowance.
- **401, 402, 403 and every other `fatal` reason stay global**, exactly as today (`cloudmodel.rs:447-449`).
- **`fatal()` returns `Some(CAPPED_LABEL)` only when a path on the enrichment batch's list was
  capped.** The list is named once in `cloudmodel.rs` (expected `/judge-task` and `/judge-email`), an
  allowlist, not "any path other than `/judge-event`". So `enrich.rs:473-477`'s closing line ("the
  rest of the batch was not sent") keeps describing that batch, an event-only cap is named once, by
  T3's summary, and a path a later lane adds (L5's `/judge-syllabus`, Q-F) never makes `fatal()` name
  the cap unless that lane puts it on the list.
  - Before coding, the implementer confirms which paths `enrich_with`'s batch calls through `model`,
    fixes the list to exactly those, and states the finding in the hand-off.
- **Wire shapes are unchanged:** the request body, the reply parsing and the `ModelError` variants.

**Tests.** All on loopback servers bound to `127.0.0.1:0`, as `cloud_contract.rs` already does.

- [ ] **Step 1: write the tests, see them fail.**
  - `a_capped_task_reply_does_not_stop_event_requests`
    - Script: `/judge-task` answers `capped`, then `/judge-event` answers a verdict.
    - `judge_event` returns `Ok`, and the server saw both requests.
    - **Red today:** the event call short-circuits with no request.
  - `a_capped_event_reply_stops_later_event_requests_only`
    - A capped event, then a second `judge_event`: `Capped` with no second request.
    - A following task call does reach the server.
  - `a_402_still_stops_every_path` (regression): a 402 on any path makes every later call fail without
    a request.
  - `only_a_capped_batch_path_makes_fatal_name_the_cap`: a capped `/judge-task` makes `fatal()`
    `Some(CAPPED_LABEL)`; a capped `/judge-event` alone leaves it `None`.
  - Change one assertion in `a_capped_event_reply_still_stops_the_batch_rather_than_becoming_unsure`
    (`cloud_contract.rs:379`): after an **event** cap, `model.fatal()` is now `None`.
    - Its two other assertions stay: the error is `ModelError::Capped`, and nothing is recorded as
      `unsure`.
    - Rename the test only if its name stops being true; record the rename in the hand-off.
  - `enrich.rs`'s `a_capped_reply_stops_the_batch_after_one_request_and_the_log_says_capped` (`:2926`)
    must stay green **unedited**. It is the proof that a task cap still names itself once.
- [ ] **Step 2:** implement in `cloudmodel.rs`.
- [ ] **Step 3:** run the workspace build and tests in the foreground, each under
      `C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh` with the worktree's
      `--manifest-path`, as §2 Gates writes it. Green and 0 other warnings. Hand off "engine: a capped reply stops only its own judge path; a task cap no
      longer silences the event pass (L3 Q-A)".

**Acceptance.** No wire change. One edited assertion, with the ruling (Q-A) named in the commit message.
`judge.rs` and `enrich.rs` are not in the diff.

## T4. Review (reviewer)

- [ ] Review the lane's diff against `main`, using this plan and the spec. Report to
      `docs/reports/2026-10-18-first-day-allowance-review.md`. Check in particular:
  - **The opening test is "no row".** Nothing deletes or re-opens a row, and there is no 120-day
    clause.
  - **The order.** The ceiling comes before both charges, the allowance is `event`-only, and it is
    spent only after the cap **really** refused: a `charge_call` error (`chargeOutcome` = `"error"`)
    answers `capped` and never reaches `chargeAllowance`. `charge`'s boolean answers are unchanged.
  - **Every `CapStore`** implements the new methods, `cloud/eval/run_eval.ts` included, and
    `deno check` covers `cloud/eval/*.ts`.
  - **Refund routing.** Refunds go to the store that charged, and the breaker counts both.
  - **Privileges.** Both RPCs are service-role only. RLS is on. The `accounts` cascade is in place.
  - **Static pins.** They would catch a regression to allowance-first, or to a renewing allowance.
  - **T3's `left`.** The arithmetic holds when the cap hits mid-batch, on the first item, and with the
    budget also cut.
  - **T3b, if present.** No wire change, and 402 is still global.
  - **The rules.** No single-user assumption, no fixture moved, no contract-list file in the diff.
  - **Privacy.** `judge_allowances` holds only an opening time, an end time and two counters per
    account, so it is usage metering like `usage_daily` and no new data class (§1). The privacy page
    and `PRIVACY_VERSION` are not in the diff, as §1 rules; a column that holds content would break
    that and is blocking.
- [ ] A finding the reviewer marks blocking goes back to the task's own agent once. A second failed
      attempt goes one level up (CLAUDE.md).
- [ ] With PASS and green CI, the controller opens the lane PR and **asks Quinn for the merge** (Q-G):
      it touches money, so it is not merged on PASS and green alone. By §2.1's M7 the merge also waits
      for Q-E unless T5 has already pushed from the lane branch, because the migration merges only
      with its staging push. On both words, the controller runs M5's re-check on the rebased lane
      branch, then `supabase db push --dry-run` there, which must list L3's version alone (or nothing,
      if T5 already pushed from that branch; M7). Then it merges (merge commit) and runs T5's push
      step from `main` straight after. If the check is red or the dry run refuses, the PR also waits
      for Quinn. If Quinn declines the merge after T5 pushed from the lane branch, T5's withdrawal
      step runs: the decline is Quinn's word for it.

## T5. Staging deploy and live proof (main session)

**Needs Quinn's word to deploy** (Q-E). Scratch vaults and scratch profiles only. Structure-only reads.
The controller holds every token: a staging session is minted by OTP (memory `staging-session-otp`).

- [ ] **Re-check the stamp, then push the migration** to staging with a plain `supabase db push`.
      By §2.1's M7 the push runs either from the lane branch before the merge (if Quinn asked to see
      the proof first, Q-G) or from `main` straight after the merge, which itself waited for this
      word; L3 never waits merged and unpushed. Before either, run M5's re-check on the lane branch,
      rebased on `main`, against `main`, the other lanes' ledgers and staging's applied versions, and
      rename if needed. Then run `supabase db push --dry-run`: if it refuses, or its list holds any
      version other than L3's own, stop, record the version, and do not push (M7). A refusal naming a
      remote version with no local file means a lane ahead pushed before merging: wait for that
      lane's merge or its withdrawal, rebase and run the dry run again, never `migration repair` to get
      past it (M6's rename and a withdrawal only); a version no lane recorded goes to Quinn (M7). If T5
      pushes from the lane branch, record the push in the session ledger. Only if L3 merged
      without this push (M7 broken) does the re-check run on `main`, and a needed rename is M8's
      re-stamp commit, merged before the push. From this push on the name is frozen (M6). If
      `db push` still asks for `--include-all`, stop: the stamp rule has been broken somewhere. The
      controller records which version sorts out of order. That version is not yet on staging, so it
      is not frozen and M6 does not apply: on a lane branch it is re-stamped by M5, on `main` by M8's
      re-stamp commit, and then the push resumes, never past it. The hook asks before the push; that
      is expected.
- [ ] **Redeploy every function that bundles the changed `_shared` modules: eleven**, then confirm
      the deployed list matches the bundle graph. Only value imports count; `import type` is erased
      (so the many `import type { Entitle } from "../_shared/judge_handler.ts"` handlers do not bundle
      it). Re-derive the set on the lane branch with a grep for `judge_deps.ts`, `judge_caps.ts`,
      `judge_pipeline.ts` and `judge_handler.ts` before deploying.
  - **Six call the caps**, so the allowance depends on them:
    - `judge-event`, `judge-task` and `judge-email` (`judge_handler.ts` imports `judge_pipeline.ts`;
      `judge_deps.ts` imports `capStore`);
    - `gmail-read` (it imports `judge_pipeline.ts` and `judge_deps.ts` directly);
    - `mail-process` and `mail-resolve` (they import `judge_caps.ts`'s constants and `judge_deps.ts`).
  - **Five bundle `judge_caps.ts` only through `judge_deps.ts`'s `sharedDb`** (`judge_deps.ts:4`
    imports `capStore` as a value) and never call a cap: `google-callback`, `google-connect`,
    `ingest-calendar`, `ingest-ics` and `judge-rules`. **Redeploy them too**, so the deployed bundles
    match the source and the check above passes as written. Leaving them stale would change nothing
    at runtime; if the controller does so anyway, it records "five left stale on purpose: they never
    call the caps" in the lane ledger instead.

  The migration goes first. A function that calls the new RPC before it exists gets `false` and
  answers `capped`, today's behaviour, so the order is safe either way but stated anyway.
- [ ] **Profile A: a new account** at the T18c school, on a scratch profile.
  - The first slot logs `events: 150 judged, M left for the next slot` with **no** cap suffix. The
    doubled cap gives 160, so the allowance may not open yet.
  - Read `judge_allowances` (a count and the `calls_left` / `until` columns only) after each slot.
    It stays absent until the day's event calls pass 160.
  - The same Q-A branch applies if this account's task batch passes its doubled cap of 120 before the
    events pass: read the task line first, as for Profile B.
- [ ] **Rolled-back SQL probe** (only if Quinn rules Q-C2 yes, with Q-E). Run the four checks of Q-C2
      in one `begin; … rollback;` session, and record each answer and the final zero count in the
      ledger. If any check fails, stop and treat it as a T1 defect, which goes back to T1's agent.
- [ ] **Profile B: an old account** (the proof account) with a new scratch profile and vault.
  - **First read slot 1's task line** and note whether the task batch was capped (60 a day on an old
    account). Which expectation holds depends on that and on whether T3b landed (Q-A):
  - **T3b landed, or the task batch was not capped:**
    - Slot 1 judges 150 with no cap line.
    - The `judge_allowances` row exists, with `calls_left` near `2000 - (150 - 80)`.
    - Later slots lower `calls_left` until the run log shows `0 left`.
  - **T3b left out and the task batch was capped:** this is the known cross-kind short-circuit, not a
    broken allowance.
    - The events pass sends no request. Its line is `events: 0 judged, N left for the next slot (the
      daily judgment cap)`, and no `judge_allowances` row is created on that slot.
    - Record it in the ledger as the Q-A follow-up showing live. Then prove the allowance on a later
      slot or day whose task backlog is under the cap: the "T3b landed" expectations apply there.
  - Record each slot's task line, events line and `calls_left` in the lane ledger.
- [ ] **Refund spot-check** (optional, only if a provider outage happens naturally). Never induce one
      against a live key.
- [ ] **Clean up** per memory `live-proof-profile-cleanup`: remove both scratch profiles, their vaults,
      credentials and autostart entries. The `judge_allowances` rows stay: they are what makes the
      allowance one-time for those accounts.
- [ ] Record the proof as dated rows in the lane ledger. Note that it was staging; prod gets the
      migration with L9.
- [ ] **Withdrawal: the proof shows a defect in L3's SQL, or Quinn declines the merge, after T5
      pushed from the lane branch** (Q-G proof-first). L3 will not merge as pushed, so the controller
      withdraws it by §2.1 M7. On a defect in the SQL (a failed Q-C2 probe check, or a Profile A or B
      line traced to T1's file) it withdraws on its own and tells Quinn; Quinn's no is Quinn's word;
      any other reason (Q-G still unanswered, or a failure it cannot place) goes to Quinn first
      (Quinn's ruling, 2026-10-19). **A defect in the functions alone is no withdrawal:** it goes back
      to T2a's or T2b's agent, is fixed on the lane branch, and the eleven functions are redeployed
      from it, the migration staying as pushed. Each withdrawal step is recorded, with the failure or
      Quinn's word, in the lane and session ledgers:
  - Redeploy the eleven functions above **from `main`** and confirm the deployed list again. L3's
    migration only adds objects and redefines none that `main`'s functions call (`charge_call`,
    `refund_call` and `enforce_budget` are not re-issued, T1), so `main`'s bundles run unchanged
    beside L3's table and RPCs, and the withdrawal opens no window.
  - On staging, in one `begin … commit`, drop `charge_judge_allowance`, `refund_judge_allowance` and
    `judge_allowances`. The table's rows go with it, the proof accounts' included, so a later proof
    starts them afresh.
  - Check the catalogue: no `judge_allowances` table and no function of either name is left. Record
    it; if one is left, stop, and the version row stays.
  - Then `supabase migration repair --status reverted <stamp>`.
  - Confirm that `supabase db push --dry-run` on `main` no longer names L3's version.
  - L3 is then neither merged nor pushed: the controller may record its slip (§2.1's L3 bullets), and
    M5 runs again before any new push. The SQL defect goes back to T1's agent first; a decline waits
    for Quinn's next word.
- [ ] **The proof fails after the merge** (the default path, T5's push from `main`). L3 is already
      cleared and holds no lane, so nothing is withdrawn. The failure goes back to its task's agent
      and lands as a follow-up PR on Quinn's word (the lane touches money). A fix to the functions
      alone is redeployed from `main` after that PR. A fix to the SQL is a new migration, because the
      merged file is frozen (M6): it joins §2.1's M1 order as a new lane at the back, behind every
      lane then on record, and follows M3, M5 and M7 like any lane.

## T6. Docs (docs-keeper)

**Files.** Modify `cloud/supabase/README.md` (the RPC list near `:80`, and the cost note), `HANDOFF.md`
(one line in the lane or state section) and `docs/reference/engine-commands.md:110` (one sentence).

- [ ] **README:** name `judge_allowances`, `charge_judge_allowance` and `refund_judge_allowance`, and
      say what each does in one line. Say the allowance is events-only, one per account, 2,000, opened
      on the first refused event call, 14 days, never renewed, and that the $2 ceiling is unchanged.
- [ ] **engine-commands.md:** the events pass ends at the first capped reply. Its summary then names
      the cap, and `left` counts every pending event not reached this run; an event that was sent and
      failed has its own `not judged` line instead (T3's wording, not "every event not judged").
- [ ] **HANDOFF.md:** one line recording the lane merged and the staging proof's result. If the Docs
      lane's HANDOFF refresh is open, hand the line to it instead of editing in parallel (§4).
- [ ] LF only, and recount anything quoted. Docs-only, so this batches with the next docs push (memory
      `ci-minutes-docs-pushes`).

## 6. Fidelity ledger

Every ruled decision and every spec requirement, the task that carries it, and what proves it. **Ruled**
means Quinn's row of 2026-10-06 (all ten at their recommendations).

| Spec | Requirement | Task | Proof |
|---|---|---|---|
| Q1, §2 | Size: 2,000 event judgments | T1, T2a | T1 pin [1] (`2000 - 1`), refund bounds [5]; T2a mirror test [11] |
| Q2, §2, §3 step 3 | Opens at the first event call the daily cap refuses, never earlier; a cap store that errors has not refused | T1, T2a, T2b | T2a `chargeOutcome tells a charge, a refusal and an error apart` (only a real `false` is `"refused"`); T2b [8] (the allowance is asked only after `chargeOutcome` answered `"refused"`), the two error tests (`…cap_charge_errors_is_capped_and_never_asks_the_allowance` on the fake, `a_charge_call_error_on_the_live_store_never_opens_the_allowance` on the real store); T1 [1] (the row is created by the first charge) |
| Q3, §2 | 14 days from opening; never renewed; unspent lapses | T1 | T1 [1] (`interval '14 days'`), [2] (no 120-day clause, the update sets only `calls_left`, the exact `where`, no delete). Text pins only (Q-C); exercised only by the optional Q-C2 probe |
| Q4, §2 | Events only; tasks and email keep the doubling and D23 | T1, T2b | T1 [4] (`check (kind in ('event'))`); T2b [8] (task and email never ask the allowance); Q-F tells L5 not to widen the check |
| Q5, §4 | First-two-days doubling kept for every kind; `charge_call` and `refund_call` unchanged | T1 | T1 [7] (not re-issued; the existing tests at `:864`, `:1722` green unedited) |
| Q6, §6 | $2 ceiling not raised; checked before any charge | T1, T2b | T1 [7] (no `enforce_budget` re-issue); T2b [9] |
| Q7, §3 | `judge_per_run_cap` 150 and the 15-minute budget unchanged | T3 | T3 touches neither; the existing cap and budget tests stay green unedited |
| Q8, §8 | Nothing new shown to the student; no read-model field; no reference moves | all | no `surface.rs`, `console.js` or fixture in the diff (T4 checks) |
| Q9, §8 | The capped pass ends with one line, and `left` counts every pending event not reached this run (failed items keep their own lines); spec test 12's "exit unaffected (judge exits 0)" | T3 | T3 [12] (two tests), regression test for `Failed`. **Exit code:** structurally untouched, so no test is added. `judge_roster` returns its lines (`Vec<String>`) and has no exit path, and T3's diff is `events.rs` alone (its Acceptance), so `judge`'s always-0 exit cannot change. T4 confirms that T3's diff holds no other file |
| Q10, §2 | One per account id, across vaults, reinstalls and schools | T1 | T1 [6] (keyed `(account_id, kind)`, cascades from `accounts`); no profile, vault or mailbox column |
| §2 state | Table, RLS on, no anon or authenticated grants, cascade from `accounts`, never deleted while the account lives | T1 | T1 [2], [6] |
| §2 state | Column `left` | T1 | **Deviation (Q-D):** named `calls_left`, because `LEFT` is reserved in Postgres; same meaning |
| §3 | The server decides; the engine asks nothing new | T2b | no change to `cloudmodel.rs`'s request bodies (T3b also keeps them); T4 checks |
| §3 | Allowance-first rejected | T2b | T2b [8]: `chargeAllowance` is never called when `chargeOutcome` answers `"charged"` |
| §3 | Concurrent requests serialise on the row | T1 | T1 [3] (the one-statement shape pin on the primary key, not the house DML count; a static proof, Q-C) |
| §3 step 4 | Both refuse: `capped`, exactly as today | T2b | T2b `an_event_both_refuse_is_capped_as_today` |
| §5 | `model failed` refunds whichever store charged; refusal, truncation and invalid answers stay charged | T2b | T2b [10] (three tests) |
| §5 | `refund_judge_allowance` gives back only while open, `calls_left < size`, `refunds < size`; never opens or extends | T1 | T1 [5]. Text pins only (Q-C); exercised only by the optional Q-C2 probe |
| §5 | A refused `charge_call` stays counted in `usage_daily.calls` | T1, T2b | unchanged by construction: `charge_call` not re-issued (T1 [7]), and T2b never calls `refund_call` for a refused cap |
| §5 | The lifetime refund bound replaces the per-invocation breaker across requests | T1, T2a | T1 [5]; T2a breaker test counts `refundAllowance` |
| §6 | Cost comment (~$0.00011 a call, ~$0.22 once) | T2a | T2a `judge_caps.ts` comment; T4 checks the arithmetic |
| §6 | Abuse bounds: one per account, entitlement before opening, `2 x size` lifetime model requests, $2 ceiling | T1, T2b | T1 [2], [5]; T2b [9]; §7's gate is unchanged (next row) |
| §7 | No new entitlement gate; lapsed accounts cannot open or spend; the 14 days keep running | none | unchanged code (`requireActiveEntitlement`, `entitle.rs`); T4 confirms neither is in the diff |
| §9 tests 1-7 | SQL tests | T1 (T5) | static pins; the live proof (T5) covers opening and spending only; the Q-C2 probe, if ruled, covers expiry and the refund bounds; concurrency stays by construction (Q-C) |
| §9 tests 8-11 | Function tests | T2a, T2b | 11 and the store tests in T2a; 8-10 in T2b |
| §9 tests 12-13 | Engine tests | T3 | as named in T3 |
| §9 Live | New and old account proofs, `calls_left` falling slot by slot, cleanup | T5 | the lane ledger's dated rows |
| §10 T4, T6 | Review; docs | T4, T6 | `docs/reports/2026-10-18-first-day-allowance-review.md`; the T6 diffs |
| §11 | Q8's alternative (two extra tasks) | none | not built: Q8 ruled "nothing new" |
| §12 | Out of scope: judging horizon, cap raise, concurrent judging, task and email backlogs, D23 changes | none | not built; T4 checks none appears |
| (plan) | Q-A, the cross-kind capped short-circuit | T3b | conditional on Quinn; else a ledger follow-up |
| (plan) | The lane touches money, so its merge waits for Quinn (handoff §1 item 7) | T4 | Q-G; the controller's merge ask and Quinn's answer as a dated ledger row |
| (plan) | No new data class; the privacy page and `PRIVACY_VERSION` do not move | T1, T4 | §1's privacy line; T4's privacy check (the table holds times and counters only) |
| (plan) | One migration version per lane, in merge order, and prod a plain push (C3 Q8, ruled 2026-10-19) | T0, T5 | §2.1's shared rule M1-M8 (the same text as L8's spec §8.4): the stamp fixed at T0 and recorded, passed to L8, L5 and L1 (Q-F); M5's re-check on the rebased lane branch before the merge or the pre-merge push; M7's merge-with-push (L3's merge waits for Q-E as well as Q-G; T5's push straight after it), "a lane ahead cleared only once on staging and on `main`, a slip only for a lane that has neither merged nor pushed", and the `db push --dry-run` stop before every staging push and every merge (a refusal naming a remote version with no local file waits for that lane's merge or withdrawal, never `migration repair` to get past it; a version no lane recorded goes to Quinn); T0's check that staging's versions all have files on `main`; M7's withdrawal if T5 pushed first and the proof shows a defect in L3's SQL (the controller's own call, Quinn told) or Quinn declines, anything else asked of Quinn first and a defect in the functions alone redeployed from the lane branch (T5's withdrawal step: the functions from `main` first, then the drop in one transaction, a catalogue check, the repair and the dry run); M8's re-stamp commit on `main` only if L3 merged without its push, or, if a pre-merge-pushed lane sorts ahead, Quinn's choice of a revert of that merge or a withdrawal of that lane; a follow-up SQL fix after the merge as a new lane at the back of M1's order (T5); the file found by pattern (M4) |
| (plan) | `charge`'s fail-closed boolean is unchanged for every existing caller | T2a | T2a `charge still folds a refusal and an error into false`; every existing test green unedited |
| (plan) | Every `CapStore` implements the new methods, so CI's `deno check` (which covers `cloud/eval/`) stays green | T2a | §2's three cloud commands; T4 checks `run_eval.ts`'s edit is the three lines only |
| (plan) | Every function bundling a changed module is redeployed, or a stale one is recorded | T5 | T5's eleven-function list, re-derived on the lane branch; the ledger row |

## 7. Checkpoints

1. **Plan review** (`reviewer`, plan mode), before T1 starts. Findings are answered in a fidelity-notes
   section added to this file, and the status line moves to REVIEWED with each round's verdict and
   finding counts recorded in it (and a report's path, if the controller files one).
2. **Quinn**, at the next checkpoint:
   - Q-A (T3b in or out). The recommendation is in.
   - Q-C (static SQL proof acceptable). The recommendation is already taken. Quinn hears that the
     pipeline fake and the live proof never exercise expiry, reopening or the refund bounds, and that
     those rules rest on text pins alone.
   - Q-C2 (the rolled-back staging SQL probe, recommended), asked together with Q-E.
   - Q-E (the staging deploy, and an address for Profile A's new staging account). Under §2.1's M7,
     L8's S7a, L5's push and L1's C1 wait until L3 is cleared (T5's push and L3's merge, or, while L3
     has neither merged nor pushed, a recorded L3 slip), so Q-E gates them too. A migration merges
     only with its staging push (M7), so L3's merge waits for Q-E as well.
   - For information (§2.1 M7, 2026-10-19): C1 waits for L3, L8 and L5 to merge, as Q8 rules, **and**
     for each merged one's migration to be on staging, and a lane that pushed before merging cannot
     be recorded as slipped unless the controller first withdraws it. That is stricter than Q8's words
     in two ways: Q8 names merges only, and Q8 leaves the slip record to the controller. Under M7's
     merge-with-push rule the first adds a wait only between a merge and its push. The second always
     applies to L8, which pushes before it merges (its S7a): from S7a until L8's merge, L5's push and
     C1 wait, so **Quinn's go on L8's merge (its S6) also releases C1**, and so does L8's withdrawal if
     L8 stops.
   - Withdrawal and revert (§2.1 M7, M8, 2026-10-19). A withdrawal drops a lane's objects and version
     row on staging; only the controller runs one, as it runs staging pushes. Quinn ruled when
     (2026-10-19, now in M7): on its own only when a lane's staging proof shows a defect in its SQL,
     recorded and told to Quinn; on a merge Quinn declined, which is Quinn's word; anything else
     asks Quinn first. A defect in the functions alone is redeployed from the lane branch, with no
     withdrawal. A merge that broke M7 behind a pushed, unmerged lane (M8's last bullet) gets one of
     two remedies, both Quinn's call: a revert on `main`, or a withdrawal of the lane ahead that
     rewrites nothing on `main` but re-runs that lane's staging proof.
   - Q-G (the lane PR's merge, because it touches money), asked with Q-E once T4 has passed; by M7 the
     merge needs both words (unless T5 already pushed from the lane branch). If Quinn asks to see the
     proof first, T5's pre-merge push holds L8's S7a, L5's push and L1's C1 until Q-G's merge (M7),
     and L3 can no longer be recorded as slipped unless the controller withdraws it (a defect in L3's
     SQL, or Quinn's no; T5's withdrawal step).
   - For information, no question unless Quinn wants one: §1's privacy reading (the allowance row is
     usage metering, so the page and `PRIVACY_VERSION` do not move).
   - Signing the spec header, which is Quinn's block, item 5. This plan does not edit it.
3. **Controller**, at dispatch: Q-B (the contract-list status of `cloud_contract.rs`) and Q-F's
   recommendation to L5, L8 and L1:
   - L3's recorded migration stamp, and the shared rule of §2.1 (M1-M8);
   - that they rebase after L3;
   - that `judge_allowances.kind` stays events-only;
   - `CapStore`'s three new methods;
   - for L5, T3b's capped rule if it lands.
4. **T4**, before the lane PR; then the merge on Quinn's word (Q-G) and, by §2.1's M7, Q-E (unless T5
   already pushed from the lane branch).
5. **T5**, on Quinn's word (Q-E): its push step straight after the merge, or before the merge from the
   lane branch if Quinn asks to see the proof first (Q-G). After a pre-merge push, a defect in L3's
   SQL or Quinn's no runs T5's withdrawal step; a defect in the functions alone is redeployed from
   the lane branch.

## 8. Out of scope for this lane

- The prod deploy (L9, Quinn's go).
- Any task or email allowance.
- Any change to `judge_per_run_cap`, `DAILY_CAP`, `MONTHLY_CEILING_USD`, D23 or the mail allowance.
- Any window copy, read-model field or console change.
- Concurrent event judging within a slot.
- A local Postgres test harness (Q-C).

## 9. Fidelity notes

### 9.00 Plan review, round 3 (PASS, four minors), the shared stamp rule, and the convergence review

Round 3 passed with no critical or important finding. Its four minors were answered on 2026-10-19, in
the reviewer's order. None changes a task's scope or a ruled decision.

1. **(minor) T1 Step 4's `deno test` lacked CI's flags.** Fixed. §2 Gates now gives CI's exact three
   lines, the `deno test` line with `--allow-write=cloud/eval`, `--allow-net=127.0.0.1` and the four
   `--allow-env` names (`ci.yml:167`). It also says the flags are always needed, and says to re-read
   the line on the lane branch. T1 Step 4 points to those lines and says a narrower run is not the
   step. T2a and T2b already pointed to §2's commands.
2. **(minor) Q-F's citations into L8's and L5's specs were stale.** Fixed. Q-F now cites both specs by
   section. It drops the `enforce_budget` correction, which L8's spec made itself (its §7 Q2 and
   revision-3 note), and records that L5's §16.8 and Q41 already rule out a `syllabus` allowance row.
   What the controller tells the planners is now the stamp and its rule, the rebase order, the
   events-only kind, `CapStore`'s new methods and, for L5, T3b's capped rule (§7 item 3).
3. **(minor) The `cloudmodel.rs` row of §4 missed L7.** Fixed. The row names L7's OpenRouter 402 named
   issue, which is likely to touch `call` and `fatal()`. The rule is the same as for L5 and L6
   (whichever lands later rebases), with two things both sides keep: 402 stays global, and the capped
   state stays per path.
4. **(minor) Spec test 12's "exit unaffected" had no stated proof.** Fixed in the ledger's Q9 row. The
   exit code is structurally untouched: `judge_roster` returns lines and has no exit path, and T3's
   diff is `events.rs` alone. No test is added.

Added in the same pass, outside the review:

5. **One migration-stamp rule for five lanes.** Quinn ruled C3 Q8 on 2026-10-19: C1 waits until L3, L8
   and L5 have merged, or until the controller records that one slipped past C3, and the prod deploy
   stays a plain push. §2.1 now states one rule, M1-M8, and L8's spec §8.4 carries the same text.
   - **What it unifies:** L3's "fixed at T0, strictly later", L8's "strictly after L3's recorded
     stamp", L5's "after L8's and before L1's" and L1's "one re-stamp before C1, frozen after".
   - **What is new:**
     - M2 defines the next free stamp. It replaces L1's "one minute after the newest", which does not
       give the same stamp when the newest is from an earlier day (item 6).
     - M6's frozen-file repair marks the new version applied rather than re-running the SQL. L3's and
       L8's migrations are not idempotent the way L1's is (its P11).
     - M7 orders staging pushes: a lane pushes only after the lanes ahead have merged, pushed or been
       recorded as slipped. So no planned push uses `--include-all`. (Item 7 removes "merged": a
       merge alone no longer clears a lane. Item 9 removes "pushed" alone too: a lane clears once on
       staging and on `main`.)
     - M8 keeps prod a plain push, as Q8 rules, by a check before each prod push.
   - **What changed in this plan as a result:**
     - §2.1's L3 bullets;
     - T5's push step (a re-check, no `--include-all`);
     - Q-G's slip path;
     - the conflict map's migration row;
     - the ledger's stamp row.
   - L5's §13 already agrees with the rule. The C3 plan's §5 did not, in more places than one
     fallback line; item 6 lists them and how they were aligned.

6. **Convergence review of this plan and L8's spec (2026-10-19): PASS, one important, three minors.**
   - **(important) The C3 plan's §5 departed from M2, M3, M5, M7 and M8.** The finding holds; item 5
     had understated it. In the C3 plan's r6 (email worktree,
     `docs/plans/2026-10-18-c3-m365-backfill-plan.md`): R6 took "one later than the newest file on
     `main`" and R13 "one minute after the newest on `main`", so neither looked at the stamps the
     lanes ahead had recorded or at staging's applied versions (M3, M5). The fallback for "`main`
     moved past the frozen name" pushed staging with `--include-all` and noted the same flag for prod
     (against M7, M8 and Q8). The reviewer's case: L5 pushes to staging, is then recorded as slipped,
     and C1 goes ahead; a `main`-only re-check gives L1 a stamp below L5's applied one. Fixed in the
     C3 plan (its r7): §5 now states L1's picking and re-check by M2, M3 and M5's three sets (the
     controller reads staging's applied versions for R13), drops every `--include-all`, sends any
     out-of-order version to M5 (not yet pushed) or M6 and M8's prod check (pushed), and names M6's
     repair for a forced rename. P11, R13, risk 4 and its ledger row follow. Q8's ruling is unchanged.
   - **(minor) M2's "gives the same stamp" was false across days.** Fixed in both copies (§2.1 M2 here,
     L8 spec §8.4) and in item 5: M2's definition replaces the C3 phrase, and the C3 plan cites M2.
   - **(minor) The status line rested on a report not on disk.** Fixed: the status line now records
     each round's verdict and finding counts itself, and §7 item 1 asks for that rather than a path.
     Filing the text in `docs/reports/` stays optional and the controller's.
   - **(minor) The cloud design's amendments of 2026-10-16, 10-16b and 10-19, which L8's §8.2 and this
     plan's Q8 citations rely on, are an uncommitted edit in the main checkout.** Not settled here: it
     needs a commit, which is the controller's (a docs-only push). Until then both documents cite text
     that is in no commit.

7. **Stamp-rules review of the three documents (2026-10-19): not a pass; one important, one minor.**
   - **(important) M7 cleared a lane ahead on its merge, though a merge does not put its migration on
     staging.** The finding holds. L3's T5 runs after the merge by default (§7 item 5), so L8's S7a
     (which §8.10 and its S7a row let go on "L3 has merged") or L5 could push past L3's merged,
     unpushed stamp, and L3's plain `db push` would then refuse. The two remedies on record did not
     fit: T5 sent it to M6, which repairs a version staging holds, and the C3 plan's §5 sent it to M5,
     which needs a lane branch the merge has removed. The same gap let C1 pass on merges alone. Fixed
     in both copies of the rule (§2.1 here, L8 spec §8.4) and in the C3 plan's §5:
     - M7: a lane ahead is cleared only once its migration is on staging, or once it is recorded as
       slipped; a merge alone clears nothing. L8's S7a waits for L3's migration on staging or L3's
       recorded slip, in M7, the L8 spec's §8.4 item 4, §8.10 and its S7a row alike.
     - C1 waits for L3, L8 and L5 to merge (Q8) and for each merged migration to be on staging. **This
       is stricter than Q8's words; Quinn hears it at the next checkpoint (§7 item 2).** It adds a
       wait only while a merged lane has not pushed. Where it bites (L3 merged, T5 waiting on Q-E),
       the controller either gets Q-E or records L3's slip. (Superseded by item 8: a merged lane can no
       longer be recorded as slipped, and L3 does not merge before Q-E.)
     - One remedy for a merged, unpushed migration (M8, with M5 pointing to it): the controller's
       re-stamp commit on `main` before its first push, landed as a one-commit PR on green CI. No
       `migration repair` runs, because staging never applied the old stamp. T5's push step, §2.1's L3
       bullets and the C3 plan's §5 use it. The C3 plan's r8 also adds M7's "on staging" to C1, R13,
       risk 4 and its ledger row. L8's spec carries the same M7 and M8 in §8.4, and its item 4,
       §8.10, S7a row, order paragraph and ledger row all say S7a waits for L3's migration on staging
       or L3's recorded slip.
   - **(minor) The cloud design's amendments are still uncommitted, and this plan, L8's spec and the
     C3 plan are untracked.** Not settled here: the commit is the controller's (docs-only), before
     any lane dispatches a migration task. Two follow-ups outside these documents stay with the
     controller too: the C3 plan's plan-mode re-review (now of its r8) before R6 is dispatched, and
     the stale L3 row in `.superpowers/sdd/2026-10-18-session-5/progress.md`.

8. **Re-review of item 7's answers (2026-10-19): not a pass; one important, one minor.**
   - **(important) A merged, unpushed lane recorded as slipped still reached staging.** The finding
     holds. Every push behind L3 runs from a branch rebased on `main`, and a plain `db push` applies
     every local version staging lacks. So with L3 merged (stamp X) and T5 waiting on Q-E, a recorded
     L3 slip let L8's S7a, L5's push or C1 apply X ahead of their own, without Quinn's Q-E word, and
     M8's merged-but-unpushed remedy could never come into play. Fixed on the controller's direction,
     in both copies of the rule (§2.1 here, L8 spec §8.4) and in the C3 plan's §5 (its r9):
     - **Merge with push (M7).** A migration merges to `main` only once its staging push is allowed,
       and the controller pushes it right after the merge, so "merged but not pushed" is transient,
       never a waiting state. L3's merge waits for Q-E as well as Q-G (Q-E, Q-G, T4, T5, §7 items 2,
       4 and 5), unless T5 already pushed from the lane branch.
     - **A slip clears only an unmerged lane (M7, M8).** A merged lane clears only by being on
       staging. §2.1's L3 bullets, Q-G's "if not answered", the L8 spec's item 4, §8.10, S7a row,
       order paragraph and ledger row, and the C3 plan's §5, R13, C1, risk 4 and ledger row say so.
       (Superseded by item 9: a lane ahead clears only once on staging **and** on `main`, and a slip
       clears only a lane that has neither merged nor pushed.)
     - **A dry run before every staging push (M7).** `supabase db push --dry-run` first; if it
       refuses, or lists any version other than the pushing lane's own, the controller stops and does
       not push. T5's push step, S7a and C1 run it.
     - **M8's re-stamp commit stays** as the remedy only if the merge-with-push rule is ever broken.
       It fixes the order, not the ride-along: the dry run still holds every push behind the stray
       migration until its own push is allowed and done.
     - M5's re-check now runs on the rebased lane branch before the merge (or the pre-merge push); the
       `main` branch of M5 is only for a migration merged against M7.
     - C1's on-staging wait stays stricter than Q8's words (Q8's text unchanged); under the
       merge-with-push rule it adds a wait only between a merge and its push. Quinn hears it (§7 item
       2).
   - **(minor) Still uncommitted.** As in item 7: the controller's docs-only commit. The C3 plan's
     plan-mode re-review is now of its r9, and the stale `progress.md` L3 row still stands.

9. **Second re-review of the stamp rule (2026-10-19): not a pass; one important.** The finding holds.
   M7 counted a lane that had pushed from its lane branch but not merged as cleared, and let it slip,
   yet under M7's own dry run nothing behind it can push. The reviewer's case, on the planned path:
   L8's S7a pushes stamp Y from its lane branch, then waits for Quinn's go at S6. M7 counted L8 as
   cleared, so L5's push was allowed and L5 merged under merge-with-push. The controller's dry run
   from `main` then refuses ("remote migration versions not found in local migrations directory"),
   because `main` has no file for Y. L5 is merged and unpushed, and C1 is held behind it. The CLI's
   own hint is `supabase migration repair --status reverted Y`; if run, L5 pushes, and L8's later
   push from `main` re-runs non-idempotent SQL and fails on staging. L8's S7a meets the same whenever
   L3's T5 pushes before L3 merges (Q-G proof-first). Fixed by the reviewer's remedy, in both copies of
   the rule (§2.1 here, L8 spec §8.4) and in the C3 plan's §5 (its r10). Q8's text and Quinn's guess
   G4 (merge with push, a slip only for an unmerged lane, a dry run before every staging push, M8's
   re-stamp as the remedy) are unchanged; this narrows the slip further and adds to the dry run:
   - **Clearing (M7).** A lane ahead is cleared only once its migration is on staging **and** on
     `main`. A lane that pushed from its lane branch holds every lane behind it until it merges. A
     recorded slip clears only a lane that has neither merged nor pushed.
   - **The dry run (M7).** A refusal naming a remote version with no local file means a lane ahead
     pushed before merging: wait for that merge, rebase, run it again; `migration repair` runs only
     for M6's frozen-file rename. A lane does not merge until a dry run on its lane branch, rebased on
     `main`, lists its own version alone (or nothing, once it has pushed from that branch).
   - **M8's second bullet** is restated: a lane that pushed from its lane branch cannot be overtaken,
     so it is never recorded as slipped.
   - **C1** (M7's two waits): stricter than Q8's words in a second way, because Q8 leaves the slip
     record to the controller and M7 now forbids it for a lane that has pushed. Quinn hears both (§7
     item 2).
   - **In this plan:** §2.1's L3 bullets (a pushed L3 cannot slip, and Q-G then gates the lanes
     behind), Q-G's "if not answered", T4's merge step (the dry run before the merge), T5's push step
     (the refusal's meaning), the ledger's stamp row and §7 item 2. **In L8's spec:** the revision
     note, §8.4 item 4, §8.10 (S7a's push now holds L5 and C1 until S6's merge), the S7a row, the
     order paragraph and the ledger row. **In the C3 plan (r10):** §5's C1 wait, its slip wording, its
     dry run and its "a lane that lands later" bullet, R13, C1, risk 4 and the ledger row.
   - A further re-review of M7 and M8 is due before T0. The commit stays the controller's.

10. **Third re-review of the stamp rule (2026-10-19, the rule's r7): not a pass; one important, three
    minors.** r6's fix passed. Q8's text, Quinn's guess G4, the order email → L3 → L8 → L5 → L1,
    L8's S7a before its merge, r6's clearing (on staging **and** on `main`) and "a pushed, unmerged
    lane holds the lanes behind it" are unchanged.
   - **(important) A lane that pushed from its branch and then stops could never be cleared.** The
     finding holds. Under r6, a lane on staging but not on `main` held every lane behind it until it
     merged, could not be recorded as slipped, and `migration repair` was barred outside M6, so
     L8's spec §8.10's hand teardown (whose version-row delete is that repair) was no clearing. The
     reviewer's case: S7a pushes Y, the staging proof fails at P1, and L8 stops with nothing on
     `main`, holding L5 and C1 for good. The same held for a re-push under the frozen name and for
     L3's proof-first T5 when the proof fails or Quinn declines. Fixed in both copies of the rule
     (§2.1 here, L8 spec §8.4) and in the C3 plan's §5 (its r11):
     - **M7 gains a withdrawal clause.** Only the controller withdraws a lane that pushed from its
       branch and will not merge as pushed. It drops the lane's objects on staging, then removes its
       version row (`supabase migration repair --status reverted <stamp>`, safe once the objects are
       gone), redeploys from `main` any function the lane deployed from its branch, and confirms by a
       dry run on `main`; each step goes in the session and lane ledgers. The lane then counts as
       neither merged nor pushed: its name is unfrozen, it may be recorded as slipped, and M5 runs
       again before any new push.
     - "Never run outside M6" now reads "only for M6's rename of a frozen file and in a withdrawal",
       and a refusal on a pre-merge-pushed lane waits for its merge **or its withdrawal**. M7's
       "holds … until it merges", its C1 and S7a waits, and M8's second bullet say the same.
     - **In this plan:** §2.1's L3 bullets, Q-G's "if not answered" (a failure or Quinn's no
       withdraws L3; an unanswered Q-G after a pre-merge push goes to Quinn rather than being
       withdrawn unasked), T4's merge step, T5's push step and two new T5 steps (the withdrawal,
       which drops L3's table and two RPCs and redeploys the eleven functions from `main`; and a
       failure after the merge, which withdraws nothing), the ledger's stamp row and §7 items 2
       and 5. **In L8's spec:** §8.4 item 4, §8.10 (its teardown is now M7's withdrawal), the S7a
       row, the order paragraph, the ledger row and a revision note.
   - **(minor 1) Under a broken rule, M8's re-stamp could leave two lanes waiting on each other.**
     The finding holds: L8 pushed Y and has not merged; L5 merges against M7, so Z is on `main`
     unpushed; L8's merge dry run lists Z and L5's push refuses on Y. M8 gains a last bullet: behind
     a pushed, unmerged lane, the remedy is a one-commit revert of the broken merge on `main`, after
     which that lane is neither merged nor pushed. Reverting a merge is Quinn's call: the controller
     asks for it. The bullet also warns that the reverted lane re-lands by reverting the revert or
     re-applying its commits, never by a plain rebase of its old branch, which git would treat as
     already merged. §2.1's last L3 bullet and the C3 plan's §5 point to it.
   - **(minor 2) The dry run assumed every orphan version on staging is a lane's.** Fixed: at T0 the
     controller checks that every version staging has applied has a file on `main`, and stops and
     asks Quinn if not (M7's closing paragraph; a new T0 step). M7's dry run now says the controller
     records each pre-merge push in the session ledger, and a refusal naming a version no lane
     recorded goes to Quinn instead of being waited on.
   - **(minor 3) Wording.** §2.1's "If L3 slips" bullet no longer holds a "T5 pushed, L3 cannot slip"
     sub-bullet: that case, and "L3 merged without T5's push", are bullets of their own. §7 item 2
     names L8, which always pushes before merging, so Quinn hears that the S6 go also releases C1;
     M7's C1 wait says the same.
   - A further re-review of M7 and M8 is due before T0. The commit stays the controller's.

11. **Fourth re-review of the stamp rule (2026-10-19, of the rule's r7): PASS; five minors and one M5
    note, all closed here in the rule's final pass, r8.** Both copies of the rule (§2.1 here, L8 spec
    §8.4) changed identically, and the C3 plan's §5 and C1 follow (its r12). Q8's text, the order
    email → L3 → L8 → L5 → L1, r6's clearing and r7's withdrawal and dry-run rules are unchanged
    except as below.
   - **Quinn's ruling (2026-10-19), written into M7's Withdrawal bullet.** The controller withdraws a
     lane on its own when its staging proof fails, records it in the session ledger and tells Quinn.
     A merge Quinn declined counts as Quinn's word. Any other case (an unanswered Q-G after a
     pre-merge push, a failure the controller cannot place, a lane stopped for another reason) asks
     Quinn first. "Fails" means a defect in the SQL; a defect in the functions alone is fixed on the
     lane branch and redeployed from it, with no withdrawal. Here: §2.1's "T5 pushed" bullet, Q-G's
     "if not answered", T4's merge step, T5's withdrawal step, the ledger row and §7 items 2 and 5.
     In L8's spec: §8.10 and the S7a row. In the C3 plan: C1's withdrawal.
   - **(M5 note) M5 ignored a re-push after a withdrawal.** It now runs before a migration "first
     reaches staging, or reaches it again after a withdrawal (M7)", and its rename is "the last free
     rename unless the controller withdraws the lane (M7)".
   - **(minor 1) A withdrawal dropped objects while branch-built functions still used them.** M7's
     withdrawal now redeploys from `main` first and drops second. `main`'s code ignores what a lane
     added if every added column is nullable or has a default; a lane that changed an existing object
     `main`'s functions call names the window in its own withdrawal step. L3 changes none (T5 now
     says so, and its old drop-to-redeploy window is gone); L8 deploys no function from its branch;
     L1's `via` is `not null default 'send'` and it redefines no function `main` calls (C3 C1).
   - **(minor 2) Nothing checked that the drop finished.** The drop runs as one `begin … commit`,
     then a catalogue check (tables, views, functions, triggers, columns, `cron.job`, schemas) is
     recorded before the repair; anything left stops the withdrawal with the version row in place.
     M7 now has five steps. T5, L8's §8.10 and C3's C1 follow.
   - **(minor 3) No path if Quinn declined M8's revert.** M8's last bullet offers Quinn two remedies:
     the revert, or a withdrawal of the pushed lane ahead, after which the stray migration pushes from
     `main` and the withdrawn lane re-stamps by M5 and re-pushes. §2.1's last L3 bullet, §7 item 2,
     the ledger row and the C3 plan's §5 follow.
   - **(minor 4) T5 withdrew on any failed proof.** T5's trigger, Q-G and §7 now name a defect in
     L3's SQL or Quinn's no; a fix to the functions alone goes to T2a's or T2b's agent and is
     redeployed from the lane branch, as C3's C1 already said.
   - **(minor 5) A follow-up migration after the merge had no place in the order.** M1 now says it
     joins as a new lane at the back and follows M3, M5 and M7; T5's post-merge step says the same.
   - The rule header, this plan's status line and review record, L8's revision note and the C3
     plan's r12 note record the pass. Whether r8's edits get a confirming read before T0 is the
     controller's call; the commit stays the controller's.

### 9.0 Plan review, round 2

How each round-2 finding was answered, in the reviewer's order, then what the revision found on its
own.

1. **(important) A `charge_call` error read as a refusal and would open or spend the allowance.**
   Fixed inside L3's files, without changing `charge`'s answers. T2a adds `ChargeOutcome`
   (`"charged" | "refused" | "error"`) and `chargeOutcome`, which makes the same one `charge_call`
   rpc. Only a real `false` is `"refused"`; a rejection or a non-boolean answer is `"error"`. `charge`
   becomes `chargeOutcome(…) === "charged"`, so it still fails closed for any caller. T2b calls
   `chargeOutcome` and asks the allowance only on `"refused"` for an `event`; `"error"` answers
   `capped`, logged as today. New tests: T2a's three-state and fold tests, and the breaker test (an
   error is not a model failure); T2b's fake-level error test and an end-to-end test over the real
   `capStore` with a per-function `RpcDb` (`charge_call` rejects, `charge_judge_allowance` would
   succeed: `capped`, the allowance never asked; a real `false`: asked and judged). The Q2 ledger row
   names them. **Why a new method rather than a three-state `charge`:** `!"refused"` is `false` in
   TypeScript and type-checks, so a leftover or rebased `if (!await caps.charge(…))` (L5, L8) would
   read every refusal as a charge and run uncapped. A boolean `charge` cannot fail that way.
2. **(minor) The conflict map missed L5 on `cloudmodel.rs`.** Fixed. The row names L5's T5
   `/judge-syllabus`, and Q-F gains an L5 bullet on T3b's capped rule. T3b's `fatal()` rule is now an
   allowlist of the enrichment batch's paths (expected `/judge-task`, `/judge-email`), not "any path
   other than `/judge-event`", so a new path never makes `enrich.rs` say "the rest of the batch was
   not sent" unless its lane puts it on the list. T3b gains a test for that.
3. **(minor) No rebase rule against L6 for `events.rs`, and :262 said L6 removes `judge.rs`.** Fixed.
   The `events.rs` row names L6 (through `events.rs`'s `crate::judge` imports and T3's
   `CAPPED_LABEL`), and whichever lands second rebases; T3 follows L6's names if L6 moved them. The
   `cloudmodel.rs` and `judge.rs` rows now say L6 removes `runtime.rs` and **edits** `judge.rs`.
4. **(minor) T5's redeploy list was not the bundle graph.** Fixed. T5 names all eleven functions: the
   six that call the caps, and the five that bundle `judge_caps.ts` through `judge_deps.ts`'s
   `sharedDb` and never call it. It redeploys all eleven, says type-only imports do not count, has
   the set re-derived on the lane branch, and says what to record if the five are left stale.
5. **(minor) `slot.sh` had no path.** Fixed. §2 Gates names
   `C:\Users\danie\GitHub\knowlu\.superpowers\sdd\2026-09-29-ultracode\slot.sh` (in the main
   checkout, not the worktree), with the full command using `--manifest-path` so no one has to `cd`.
   T3 Step 4 and T3b Step 3 point to it.

Found while answering finding 1 (not in the round-2 report):

6. **`cloud/eval/run_eval.ts:93` is a fourth `CapStore` implementation** that the plan did not list.
   CI's `deno check` covers `cloud/eval/*.ts` (`ci.yml:150`), so adding interface methods without it
   would fail CI. T2a now edits it (three lines in `EVAL_CAPS`), §4 gains its row, and §2's cloud
   gates are now CI's own three commands (they include `cloud/eval/`), not the narrower
   `cloud/supabase/` run.
7. **T2b's edit to `judge_pipeline.ts` makes the lane PR run `eval-gate`** (`ci.yml:242`). That is
   bounded and spends nothing while staging's `eval_cases` is empty. T2b carries a CI note, Q-G's
   merge ask names it, and T1's migration avoids the word `models` so it does not trip the gate's
   migration test by itself.

### 9.1 Plan review, round 1

How each round-1 finding was answered, in the reviewer's order.

1. **(important) The plan merged the lane PR without asking, against handoff §1 item 7.** Fixed. The
   lane touches money (a new spend path on the judge route), so §1 now says so, §5 and T4 make T4 PASS
   plus green CI the point where the PR is ready, and the merge waits for Quinn's word. New Q-G asks
   it together with Q-E, and offers T5 from the lane branch first if Quinn wants the proof before the
   merge. If unanswered, the PR stays open, and L8 and L5 may land first with L3 renaming its stamp.
2. **(minor) `left` was described as "every pending event not judged".** Fixed by rewording, not by
   changing the formula. T3, its acceptance, T6's engine-commands sentence and the Q9 ledger row now
   say "not reached this run", with failed items named by their own lines. T3 gives the
   Failed-judged-Capped example and the reason the formula stays (folding failures in would change
   today's uncapped line, which the regression test pins at `left` 0).
3. **(minor) T1 Step 2 said "green with nothing else changed", but the `parsed` guard goes red.**
   Fixed. Step 2 now says the guard at `:333` (40, now 42) is the one expected red until Step 3.
4. **(minor) Two lanes could pick the same `<date>000100` stamp.** Fixed. §2.1 fixes the stamp once at
   T0, after checking the other lanes' ledgers and worktrees, takes the next free number on a clash,
   and records it. Q-F and §7 have the controller pass that exact stamp to L8, L5 and L1, which stamp
   strictly later.
5. **(minor) The header called this round 2 and asked the reviewer to write both rounds.** Fixed. The
   header labels the dispatch's findings round 1 (this section) and the earlier seven points a
   pre-review self-check (§9.2). The controller files round 1's text at the report path.
6. **(minor) No privacy ruling for the new per-account table.** Fixed. §1 says `judge_allowances` is
   usage metering like `usage_daily` (times and counters, no content), already covered as account data,
   so the page and `PRIVACY_VERSION` do not move. T4 checks it, and Quinn hears it at the checkpoint;
   a page line, if wanted, is a privacy bump for L4.

### 9.2 Pre-review self-check (before round 1; no report on disk)

Seven points found and fixed before the round-1 review. They are kept for traceability only.

1. **(important) Conflict map missed L5 and L8 on `judge_caps.ts` and `judge_caps_test.ts`.** Fixed.
   §4 now names L5's T6a (`Kind`, `DAILY_CAP` and the `:11` pin) and L8's Q4 alternative
   (`record_tokens`), with the rule that both rebase after L3. New Q-F has the controller tell L8 that
   L3 does not re-issue `enforce_budget`, and tell L5 not to widen `judge_allowances.kind`. The other
   lanes' specs are not edited here.
2. **(important) T1's own SQL failed pins [2] and [3].** Fixed by changing the pins, not by
   loosening them or splitting the statement.
   - [3] no longer uses the house `insert|update|delete` count. It pins the shape: one `insert`, one
     `update` that is the `do update` arm, no `delete`, `select`, `if` or `found`, and two statements
     in the body.
   - [2] pins the qualified `where` (`judge_allowances.until > now() and judge_allowances.calls_left >
     0`), compared over the whole span from `do update` to `returning`.
   - T1's SQL notes now say why the conflict arm stays qualified (both the row and `excluded` are in
     scope there) and why it must never become update-then-insert (the unique-violation race that
     would answer `capped` with room left).
3. **(important) T2 was too large.** Split into T2a (store methods, constants, cost comment, the three
   fakes, [11] and the store tests) and T2b (charge order, refund routing, [8]-[10]), run in that
   order as separate dispatches. T2a now runs after T1, because [11] reads T1's file; the old "parallel
   with T1" was wrong for that test. T1 stays whole, with the reason stated in §5: its pins are the
   only proof of its SQL.
4. **(minor) Profile B ignored Q-A.** Fixed. T5 reads the task line first and branches on whether T3b
   landed and whether the task batch was capped. Profile A has the same note for its cap of 120.
5. **(minor) T3's red reason said two `not judged` lines.** Fixed to three (items 3, 4 and 5).
6. **(minor) Q-C promised a TS fake of the SQL's rules.** Corrected. Q-C now says which proof covers
   what, and that expiry, reopening and the refund bounds rest on text pins alone. Quinn hears that at
   the checkpoint (§7). New Q-C2 offers a rolled-back staging probe that exercises those rules against
   the real Postgres without a harness.
7. **(minor) VISION citations came from the main checkout.** Fixed to the 68301ec worktree's :154,
   :155 and :192, noting the main checkout's uncommitted copy is two lines lower.
