# Email forwarding to Knowlu (implementation plan)

**Date:** 2026-09-30. **Base:** this lane's branch and worktree under `.claude/worktrees/`, cut on
2026-09-30 for the spec. Their current name carries the retired working title; PQ1 renames both to
`email-forwarding`. The branch is `main` at `4aa6560` plus the signed spec and its amendments. `main`
is now `30b6fb7`: Gmail connect (#26) and M2 editing (#27) are both merged. T0.0 merges `origin/main`'s
HEAD as it stands then, recorded by sha. §2 was read at `199cd1f`, and its citations were re-checked by
name at `30b6fb7` (§10, R1).
**Governs:** `docs/specs/2026-09-30-email-forwarding-design.md`, signed by Quinn on 2026-09-30: all of
§10 (A1–A13) and the signing sheet's decisions 1–8. §13 Q1–Q11 were answered the same day and bind.
Above it: the cloud design's Amendment 2026-09-30 and Amendment 2026-09-29 (rulings 3, 10, 11, 12).
**Names (Quinn, 2026-09-30).** The student-facing and design name of the private
`<token>@in.knowlu.com` address is **"your Knowlu address"**. The feature is **"email forwarding to
Knowlu"**, and what it brings is **"forwarded mail"**. Code names are `mail` and `forward`
(`mail-inbound`, `mail_addresses`, origin `forward`), with the spec's meanings unchanged. The retired
working title appears nowhere in this plan, the code, the tests or the copy.
**Stage:** MVP (spec D1; ruling 10's MVP list as amended by A5). **Size:** XL, the largest MVP lane.
**Status:** draft for review. Nothing here is built.

## 1. Goal, and what must not move

A student signs in to their school mail, and to personal Gmail if they want it, in a Knowlu window
that is only for signing in. Knowlu turns forwarding on to their Knowlu address, proves it with a
test message, and brings in mail since the term began, applied and listed under "From your email"
with Undo. Mail after setup is proposed as cards. A mailbox that goes quiet says so, and the saved
sign-in repairs it. VISION check: the sources line (A7), commitment 5's setup exception (A10),
"Knowlu reads" (A8), data minimisation (A9), success 2 (nothing missed) and success 5 (the first
session already knows what's next).

These must not change:

- **The engine chain.** `sync → coursework → ingest → grades → judge → rank` and `slot_argv` stay as
  they are. The `mail` step is the app's, after the grades capture and before the chain (spec F6,
  A12). `rank` and `surface` never see a claim; no model runs in the engine; `judge` exits 0 on every
  mail failure shape.
- **The contract list, except two files.** `write.rs`, `journal.rs` (`VIAS`, `OPS`), `yamlemit.rs`,
  `yaml.rs`, `pystr.rs`, `ledger.rs`, `ids.rs` (`NOTE_FOLDERS`), `provenance.rs`
  (`JUDGED_FIELDS_TASK` is not widened), `sync.rs`, `entitle.rs`, `wincred.rs`, `reconcile.rs`,
  `credentials.rs` and `updates.rs` are used, never edited. Only `approvals.rs` (T8) and `account.rs`
  (T12) change, by contract-engineer, reviewed by contract-reviewer. Every gate checks this with §10
  R12's three-dot diff over the whole list.
- **Frozen references and oracles.** None is regenerated. `git diff --stat origin/main...HEAD --
  engine/tests/fixtures` stays empty at every gate; no fixture vault holds a mail note.
- **The Gmail OAuth path.** Only D22 changes `gmail-read`. `gmail_queue`, `pull_gmail` and the
  device's Gmail code are untouched.
- **Privacy text.** No `site/` edit; `PRIVACY_VERSION` stays `2026-09-24`. No release is tagged from
  a `main` that carries this lane or the suspended grades gate before privacy bump #1 merges (D18).
- **Rule 1.** No student's mailbox, address, vault or machine is named in code, tests or copy. Every
  test message is synthetic. Founder-owned mailboxes appear only in T0's and T18's live steps, never
  in the repo. No new `"quinn"` or `"student"` literal; `engine/tests/human_actor_literal.rs` passes.
- **Desktop safety.** The product uses no synthetic OS input, no CDP and no debugging port (test 34).
  T0's spikes may use the spike harness, and that harness never enters the repo.
- **Tests never leave the machine.** Every server a test needs is a listener on `127.0.0.1:0`. No new
  Credential Manager target is written.
- **Telemetry.** No new row shape and no forwarded-mail content (VISION, as amended by A12).

## 2. What changed since the spec was written

Read at `199cd1f` in the root checkout; every name cited below was re-found at `30b6fb7` (§10, R1).
Briefs cite functions and headings, never line numbers.

- **The branch predates #26.** Its `enrich.rs` has no `PULL_ROUNDS`. At `199cd1f` every citation in
  spec §1 holds: `pull_gmail`, `GMAIL_ACTOR`, `PULL_ROUNDS`, `GMAIL_NOTE`, `write_gmail_card`, the
  kinds `labels_to_report` reports, `pull_gmail_queue`'s quiet reasons, `gmail-read`'s `WINDOW` and
  `READ_CAP`, `CONFIDENCE_FLOOR`, `DAILY_CAP`, `MONTHLY_CEILING_USD`, the kind and origin checks, the
  export assertion in `migrations_test.ts`, `write::find_pending_amendment`, `propose_amendment` and
  `write_literals`' judged check, `journal::human_set`, `human_edited` and `read_human_actor`,
  `approvals::AMENDABLE_FIELDS`, `validate_amendment` and `apply_amendment`, `ingest::sync_tasks`'
  update branch and `IMPORTED_PAST`, `info::open_info`, `close_info` and `KINDS`,
  `commitments::vault_day`, `completion::propose_done`, `grades::availability` and its four callers,
  `scheduler::{GradesSeam, grades_step, slot_argv, run_slot, run_slot_with}`, `account::{sign_out,
  delete_local_data, delete_my_data}`, `commands::{executor_ctx, set_fields, delete_note,
  close_info}`, `scaffold::Curated` and `onboarding::offer_marker`.
- **The signing commit is on this branch only.** VISION, the cloud design, the Gmail spec and the
  grades spec carry their 2026-09-30 markers here; `main` carries none of them. `CLAUDE.md` carries
  neither of A12's two lines on either branch (P2, H0).
- **`config.toml` opts every function out of the gateway's JWT check**, and
  `_shared/config_toml_test.ts` fails both when a function directory has no `[functions.<name>]`
  section with `verify_jwt = false` and when a section names no directory. The spec's "for
  `mail-inbound` only" is read as: `mail-inbound` is the one function with no bearer at all, and the
  SNS signature is its check. Every new function gets its own section in the commit that creates its
  directory (P5).
- **`Kind` is `"task" | "event" | "email"` in `judge_validate.ts`,** and `DAILY_CAP` is a
  `Record<Kind, number>` in `judge_caps.ts`. Widening `Kind` in one file breaks the other's type
  check (P6).
- **`models` rows are seeded and re-pinned by migrations.** `migrations_test.ts`'s
  `lastModelPinBlocks` reads the last pin per kind and is typed for the three existing kinds (P8).
- **`write::to_literal` emits any mapping through `yamlemit::safe_dump_flow`.** `mail_asof` needs no
  new write mode, so spec §8's stop condition does not fire (P17).
- **`write::delete` moves a note into `archive/` and never unlinks it.** A rejected card is an
  `archive/` file with `status: rejected`. T8b reads that for "already rejected" (P12).
- **`approvals.rs` already keys one amend arm on `created_by`** (sync's
  `reassert_rejected_sync_card`). T8's arm follows that precedent.
- **`labels_to_report` reports only the `task` and `event` judgment kinds.** A `mail_claim` card is
  already passed over, so test 28b pins this without changing anything.
- **`run_slot_with` takes a `GradesSeam`.** The mail step needs a seam of its own, and the three
  calls in `app/tests/scheduler.rs` (the file T19 edits) must pass it (P18).
- **`Curated` literals appear in `app/tests/grades.rs` and `app/tests/scheduler.rs`.** T14's three
  new fields therefore touch T19's two test files, so T19 lands first.
- **CI's `deno test` covers only `cloud/supabase/` and `cloud/eval/`.** The step scripts' tests under
  `app/assets/mail/` need their own step (T17). `cloud/supabase/deno.json` has `lock: false` and
  imports only `@std/assert`.
- **The grades kept session never needs a script's result back from a page.** It reads cookies and
  fetches from Rust. The mail scripts do need results, so S1's result channel is new ground (T0).
- **The host.** Tauri is pinned at `2` (`wry`, default features off). Grades' hidden window is
  `visible(false)` with `skip_taskbar(true)`. The one capability file grants `windows: ["main"]`, so a
  `mail` window has no grant unless one is added; test 33 pins that none is.
- **Sibling lanes.** M2 editing is merged (#27, `30b6fb7`). `write::set_body`,
  `write_one_line_literals` and the `set_body` op (`journal::OPS` has seven) are on `main`, and M2's
  commands are already in the console's `generate_handler!` list. M2 changed `write.rs`, `grades.rs`,
  `app/src/main.rs`, the page files and `settings-check.py`. The events lane (`j-events`), the registrar
  (`p3-registrar`, draft PR #19) and two desktops (`two-desktop`, draft PR #20) are not merged. Their
  shared files, and who rebases each, are in §10, R2.
- **HANDOFF on `main` has no email lane.** Spec §15 and T13 cite HANDOFF lines that will drift, so T13
  edits by heading.
- **The spec's file name.** The signed spec is `2026-09-30-email-forwarding-design.md`; it was renamed
  at signing.

## 3. Decisions this plan makes

- **P1. Small tasks, with the spec's ids kept.** Spec §14's rows are split into dispatches of about 80
  lines of edits each, tests included where possible. The spec's id is kept with a dot number (T2 is
  T2a–T2g; T5 is T5.1–T5.5), so the fidelity ledger maps back to spec §14. A dispatch that would
  clearly exceed the size is split before it starts, not after.
- **P2. The signed docs reach `main` now; CLAUDE.md's lines reach it with the code.** At T0.0 the main
  session lands the signing commit's documents on `main` as a docs-only commit (the spec, the cloud
  design's amendment and markers, VISION, the Gmail spec, the grades spec). Rulings bind from
  signing, and other lanes need them: the founder's grades proof waits on A13. CLAUDE.md's two A12
  lines describe code, so they land with the lane's merge (H0).
- **P3. Three sub-lanes, one implementer each.** HANDOFF allows one implementer at a time per
  worktree, so parallel work needs separate worktrees. After T0.0 the controller cuts three
  sub-branches from the lane branch, each in its own worktree: `email-cloud` (only `cloud/**`),
  `email-engine` (only `engine/**`) and `email-app` (`app/**` and `scripts/*.py`). Their files are
  disjoint by construction. `integrator` merges each into the lane branch at the checkpoints. T19
  runs on its own branch from `main` (P19).
- **P4. The gates are read as narrowly as the spec states them.**
  - S0 gates every cloud task.
  - S1–S3 gate every task that touches the window or a step script (T9.2, T10.2a onward, T11.3).
  - S6 gates merging the pace numbers, not writing the code (spec §12.5).
  - T5, T6.1, T7, T9.1, T10.1, T14 and T19 wait on no gate. T8 and T8b wait on PQ5, which is asked at
    T0.0 (§10, R9). Spec §14 placed T5 after T1, but T5 is pure engine code ("T5 needs no cloud"). If S0
    fails, D2's fallback changes only intake.
- **P5. Function config.** Each task that creates a function directory adds its
  `[functions.<name>] verify_jwt = false` section in the same commit. The `mail-inbound` section
  carries one comment line: it is authenticated by its SNS signature, as `stripe-webhook` is by
  Stripe's. `config.toml` is the one file several cloud tasks touch. They touch it in sequence, in one
  worktree, and only by appending.
- **P6. Mail kinds are their own type.** `judge_validate.ts` gains `MailKind = "mail_claim" |
  "mail_resolve" | "mail_digest"`. `Kind` is not widened, so no existing `Record<Kind, …>` changes.
  `judge_caps.ts` gains a `MailKind`-keyed daily-cap record beside `DAILY_CAP` (spec §5.1's
  "`DAILY_CAP` gains" in substance). A shared function typed on `Kind` that must take a mail kind is
  widened to `Kind | MailKind` in its own task. T3b lands before T3c.
- **P7. The nightly sweep is a mode of `mail-process`.** A cron call with `{mode: "sweep"}` runs the
  sweep, so its rules are tested in Deno against a fake store:
  - expire work and delete each expired item's `mail_seen` row;
  - close timed-out batches;
  - prune `mail_seen` at 180 days;
  - prune delivered claims at 7 days and undelivered ones at 30, counting them;
  - prune batches and canaries at 30 days.

  The two RPCs (`charge_allowance`, `delete_mail_address`) are SQL. Like every RPC here, they are
  pinned textually in `migrations_test.ts`, and their behaviour is smoke-tested on staging (T18).
- **P8. Provisional pins.** T1.2 inserts `models` rows for `mail_claim` and `mail_resolve`, pinned to
  `email`'s current model, route and ZDR provider. They are marked provisional in a SQL comment.
  `mail_digest` gets a row only if T16 is built. Test 17's live check (T18a) confirms each pin, or a
  one-statement follow-up migration re-pins it before the proof. The eval seed stays empty
  (R-C2-E12).
- **P9. "A Knowlu email that day" (test 6).** It means a canary to any of the account's mailboxes, or
  the annual reminder (`last_reminded_at`), on that UTC day. A sign-in code the student asked for does
  not count. The canary goes through Resend, as `billing-jobs`' mail does, from a new
  `_shared/canary_send.ts`. `billing-jobs` is not edited.
- **P10. Authoritative feeds by `created_by`.** `mailreconcile` holds one constant naming the
  creators whose notes a feed owns: `blackboard` (`ingest`) and the coursework writers' values. T5.5's
  first step greps every engine template's `created_by` and records the list in the ledger. A test
  asserts that the feed templates' values are in the constant and that `gmail`, `events`, `mail`,
  `agent` and a hand-made note are not.
- **P11. T8's human write keeps the executor's `via` and run id.** Only the actor changes, to
  `journal::read_human_actor(vault)`. The card's own status bookkeeping and an approved mail **task**
  card stay `agent:approvals`.
- **P12. The card `ingest` files (T8b).**
  - It carries `created_by: blackboard` (the feed's own name, as its notes carry) and no judgment
    fields.
  - "Already rejected" means an `archive/` card with `kind: amend` and `status: rejected`, the same
    `target`, and `changes.<field>.to` equal to the feed's current value, whoever filed it.
  - One card per field, never one card for both, so its field set is `{title}` or `{due}`. No ingest
    card is filed while `write::find_pending_amendment` finds one for that target and field, and the
    mail card writer's own check then finds the ingest card too (§10, R14).
  - A hand-set value of null, or a feed value of null, keeps the note's value and files no card,
    because `validate_amendment` refuses a null `from` or `to` (§10, R14).
  - It is a proposal like any other: `proposed_at` and `first_proposed_at` are the day it is filed,
    `first_proposed_at` set once, so `approvals::defer_over_budget` charges it (§10, R3).
- **P13. The mail writers get a contract read.** T6.2 and T6.3 are implementer work, as the spec
  routes them. Their bytes must pass `approvals::validate_amendment` and the frontmatter rules, so
  Checkpoint C has contract-reviewer read the card writer, the direct-set writer (with `mail_asof` and
  the body line) and the seen-ledger write (§10, R13).
- **P14. The DOM shim for the step scripts.**
  - `app/assets/mail/deno.json` imports one HTML DOM implementation for Deno, pinned to an exact
    version, with `lock: true`.
  - T10.2a's first step confirms the package resolves. `deno-dom` from JSR is first choice, `linkedom`
    second.
  - The lock file is `app/assets/mail/deno.lock`, committed with LF endings in T10.2a, so the shim's
    bytes are pinned by integrity hash.
  - The tests never ship: `include_str!` names only the scripts. The tests fetch nothing at run time
    beyond module resolution. That fetch follows the cloud tests' `@std/assert` precedent, and it is
    the one exception to "tests never leave the machine" this lane takes (§10, R8).
- **P15. The app's client for the `mail` routes** is a new `app/src/mail_client.rs`. It uses rustls
  `ureq`, as `grades.rs` does. It reads the session through `account`'s existing public reader and
  never edits `account.rs`.
- **P16. Test placement.** Engine unit tests go in each new file's `#[cfg(test)]` module, and the
  loopback tests go in a new `engine/tests/mail_pass.rs`. App tests go in a new `app/tests/mail.rs`,
  except for the scheduler (`scheduler.rs`), account (`account.rs`), grades (`grades.rs`) and page
  (`static_assets.rs`) tests, which go in their existing files. No test reads or writes
  `engine/tests/fixtures/**`.
- **P17. `mail_asof` is written by `write_literals` with `WriteOpts::default()`,** with its value
  from `write::to_literal` of a one-key-per-field mapping. Test 27 pins the line, and a second test pins
  that re-applying it from its journal record gives the same bytes. If Checkpoint C's contract-reviewer
  finds a multi-line or re-dumped result, the task stops and goes to contract-engineer (spec §8).
- **P18. The mail step's seam.** `run_slot_with` gains a `MailSeam` beside `GradesSeam`, carrying the
  hidden-window step and whether a mail window is open. The three existing test calls pass an empty
  seam. T11.5 runs after T19, which shares the test file.
- **P19. T19 goes first, as its own PR from `main`, with T13a's HANDOFF lines** (the release guard and
  A13's review trigger). The spec allows it ("may merge first, with T13's guard line"). It unblocks
  the founder's grades proof on a dev build of `main`.
- **P20. Two claims at one ordering date with different values (§4.9).** The card proposes the value
  of the claim that sorts last by uid and claim index, and its body names both values, each with its
  sender. This is a deterministic reading of "naming both".
- **P21. Two route details that §4.1 and §4.6 imply but §6.3's list omits.**
  - `POST /addresses/<id>/hint` sets `mailbox_hint` once, after sign-in, and only to an address that
    fits the provider's domains.
  - The state route accepts `{state: confirming}`, which opens Gmail's 15-minute confirm window at
    §4.3 step 3.

  Neither adds a column or data class.
- **P22. "An active forwarding" (D22)** means a `mail_addresses` row for that Gmail address that is
  neither `revoked` nor `off`. A `quiet` forwarding still counts: recover brings its gap in, and
  reading both paths would file every message twice.

## 4. Open questions (each with a recommendation)

- **PQ1. Rename the branch and worktree.** The lane's branch and worktree carry the retired working
  title, and Quinn asked for it to be removed everywhere.
  - *Recommend:* at T0.0, before any code, rename the branch to `email-forwarding`, move the worktree
    to `.claude/worktrees/email-forwarding`, push the new name and delete the old remote branch. The
    branch holds docs only today, so nothing is lost.
  - *Needs Quinn's word:* deleting a remote branch is destructive.
- **PQ2. DKIM and ARC verification: our own verifier or a library.** Binding a live forward
  (§4.6 step 5) needs the provider hop's ARC set verified. SES gives verdicts only for the outer
  message, so this verification is ours.
  - *Recommend:* our own small verifier in `_shared/dkim.ts` (T2d), scoped to what binding needs:
    rsa-sha256 with relaxed and simple canonicalisation, the highest ARC instance's seal, and its
    ARC-Message-Signature over the headers it lists. The key resolver is injected, so tests use
    in-test keys and no DNS. Before Checkpoint B, the main session runs the verifier on its own
    machine over S0's captured real forwards (never committed) and records pass or fail by case in
    the T0 report.
  - *The alternative:* a mature npm library (for example `mailauth`). It is better tested on real mail,
    but Deno's `node:dns` and `node:crypto` compatibility is unproven, and with `lock: false` its
    transitive dependencies would float inside the function that reads every student's mail.
  - *Blocks:* T2d only. Asked when S0 passes.
- **PQ3. The per-course digest (D13, T16).** It is optional, behind `mail_digest: false`, and "cut if
  time is short".
  - *Recommend:* decide at Checkpoint D, and cut it from this lane unless the lane is ahead of plan.
    Nothing in VISION's test needs it before a student has used notices. Cutting it removes one
    function, one pinned model and its `models` row.
- **PQ4. How a submission receipt travels.** Spec §4.7 step 3 and §4.9 route a templated "submission
  received" mail to `completion::propose_done`. §5.1's signed claim shape (`field: due | cancelled |
  null`) has no way to carry one.
  - *Recommend:* one more `field` value, `completed`, with `value: "true"`, emitted **only by a
    template** (`source: template`). T3b refuses it from the model. T5.5 maps it to propose-done in
    both phases, which always proposes and asks once per task, as §4.9 says. The four claim kinds are
    unchanged (Q5).
  - *The alternative:* a fifth claim kind, `receipt`. This is clearer on the wire, but it changes the
    kind set Q5 fixed.
  - *Blocks:* T3a, T3b and T5.5's receipt test. It is asked with PQ2.
- **PQ5. A feed card for `title` cannot be approved today (§10, R9).** D11 (5) has `ingest` file an
  amend card for a hand-set `title` or `due`. `approvals::AMENDABLE_FIELDS` has no `title`, and
  `validate_amendment` answers "field not amendable: title". That refusal is recoverable, so an approved
  title card goes back to pending at every `rank`, for good. Spec §8's `approvals.rs` row names only D11
  (4)'s change, so every answer needs Quinn's word.
  - *Recommend (c):* T8 makes `title` amendable only on a card whose `created_by` is `blackboard`, the
    one feed that files such a card (P12), beside the existing sync arm.
    `AMENDABLE_FIELDS` stays nine, and `amendable_fields_is_exactly_the_nine` is unchanged. Every other
    amend executes exactly as today. contract-engineer writes it, and Checkpoint A reads it.
  - *(a)* Add `title` to `AMENDABLE_FIELDS`. That changes the pinned test's assertion and every amend in
    every vault, including model-filed ones, against §8's "every other amend executes exactly as today".
  - *(b)* Narrow D11 (5) to `due` only: a hand-set title is kept and the feed's rename files nothing.
    This changes a signed answer (§13 Q7).
  - *Blocks:* T8 and T8b. It is asked at T0.0, after PQ1.

## 5. Tasks

**Rules for every task.**
- **Tests first.** Write the named tests, run them in the foreground and see each fail for the stated
  reason, then write the code. A test's name may change; its assertion may not.
- **Gates by area.**
  - A Rust task ends with `cargo build --workspace` and `cargo test --workspace` from the root, in the
    dev profile. Green means 0 warnings except the accepted `.rsrc` line, the four `#[ignore]` tests
    untouched, and §10 R12's contract-list diff (after `git fetch`, against `origin/main...HEAD`)
    showing no file but `approvals.rs` and `account.rs`, and nothing under `engine/tests/fixtures`.
  - A cloud task ends with `deno check`, `deno lint` and `deno test` exactly as `ci.yml`'s `cloud` job
    runs them.
  - A step-script task also runs `deno test --config app/assets/mail/deno.json app/assets/mail/`.
- **Files and data.**
  - New files are LF.
  - Every test message, address and name is synthetic.
  - No test touches `engine/tests/fixtures/**`.
  - No test contains the literal `"quinn"` or `"student"`.
  - Every mail-derived string written to the vault goes through `write::to_literal`.
  - No log line, error or panic carries a message body, header value, address or token (spec §6.3's
    house rule).
- **Running the work.**
  - One implementer at a time per worktree (P3).
  - Each dispatch commits on its own with the session's trailer and leaves its worktree green.
  - An agent that fails twice goes one level up, never straight to `max`.
- **Briefs.** The controller extracts each brief by its `###` or `####` heading below, never by line
  number. Each brief carries the spec sections it names.

**CL** marks a contract-list file. **Opus** marks a task whose first agent runs on Opus. **Quinn**
marks a task that needs Quinn's word, presence, a secret or a spend. **WT** is the worktree (P3):
`lane`, `cloud`, `engine`, `app`, or `main` for T19's own branch.

| Task | Agent (model, effort) | Opus | Quinn | Files | After | WT |
|---|---|---|---|---|---|---|
| T0.0 | main session | yes | PQ1 | none (git, signed docs to `main`, ledger) | — | lane |
| T0.S0 | main session | yes | AWS, DNS | T0 report | T0.0 | — |
| T0.S1–S3 | main session | yes | at the machine | T0 report (spike in scratchpad) | T0.0 | — |
| T0.S4 | main session | yes | §13 Q3 revisit | T0 report | 2026-10-08 | — |
| T0.S5 | main session (optional) | yes | — | T0 report | S1–S3 | — |
| T0.S6 | main session | yes | M365 test tenant | T0 report | S0 | — |
| T19 | `implementer` (Sonnet, high) | no | merge word | `app/src/grades.rs`, `app/tests/grades.rs`, `app/tests/scheduler.rs` (grades tests only) | T0.0 | main |
| T13a | `docs-keeper` (Sonnet, medium) | no | — | `HANDOFF.md` | T19 | main |
| R19 | `reviewer` (Opus, high) | yes | diff to Quinn | `docs/reports/…-grades-gate-suspended-review.md` | T13a | main |
| T5.1 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/mailreconcile/mod.rs` (new) | T0.0, H1a | engine |
| T5.2 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/mailreconcile/resolve.rs` (new) | T5.1 | engine |
| T5.3 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/mailreconcile/rules.rs` (new) | T5.2 | engine |
| T5.4 | `contract-engineer` (Opus, xhigh) | yes | — | `engine/src/mailreconcile/rules.rs` | T5.3 | engine |
| T5.5 | `contract-engineer` (Opus, xhigh) | yes | PQ4 | `engine/src/mailreconcile/rules.rs` | T5.4 | engine |
| T8 | `contract-engineer` (Opus, xhigh) | yes | PQ5 | `engine/src/approvals.rs` CL | T5.5, PQ5 | engine |
| T8b | `contract-engineer` (Opus, xhigh) | yes | PQ5 | `engine/src/ingest.rs` | T8, PQ5 | engine |
| A | `contract-reviewer` (Opus, xhigh) | yes | diffs to Quinn | `docs/reports/…-email-forwarding-contract-review-a.md` | T8b | — |
| T6.1 | `implementer` (Sonnet, high) | no | — | `engine/src/cloudmodel.rs`, `engine/tests/mail_pass.rs` (new) | T8b | engine |
| T6.2 | `implementer` (Sonnet, high) | no | — | `engine/src/mail.rs` (new), `engine/tests/mail_pass.rs` | A, T6.1, H1b | engine |
| T6.3 | `implementer` (Sonnet, high) | no | — | `engine/src/mail.rs`, `engine/tests/mail_pass.rs` | T6.2 | engine |
| T6.4 | `implementer` (Sonnet, high) | no | — | `engine/src/mail.rs`, `engine/src/enrich.rs` (one call), `engine/tests/mail_pass.rs` | T6.3 | engine |
| T7 | `implementer` (Sonnet, high) | no | — | `engine/src/mailapplied.rs` (new) | T6.4, H1c | engine |
| T1.1 | `cloud-engineer` (Opus, high) | yes | — | `cloud/supabase/migrations/<stamp>_mail.sql` (new), `migrations_test.ts` | S0 | cloud |
| T1.2 | `cloud-engineer` (Opus, high) | yes | — | the same two files | T1.1 | cloud |
| T1.3 | `cloud-engineer` (Opus, high) | yes | — | the same two files | T1.2 | cloud |
| T2a | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/sns.ts` (new, with its test) | T1.3 | cloud |
| T2b | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/s3.ts` (new) | T2a | cloud |
| T2c | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mime.ts` (new) | T2b | cloud |
| T2d | `cloud-engineer` (Opus, high) | yes | PQ2 | `functions/_shared/dkim.ts` (new) | T2c | cloud |
| T2e | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mail_bind.ts` (new) | T2d | cloud |
| T2f | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mail_bind.ts` | T2e | cloud |
| T2g.1 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-inbound/**` (new), `config.toml` | T2f | cloud |
| T2g.2 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-inbound/**` | T2g.1 | cloud |
| T3a | `cloud-engineer` (Opus, high) | yes | PQ4 | `functions/_shared/mail_templates.ts` (new) | T2g.2 | cloud |
| T3b | `cloud-engineer` (Opus, high) | yes | PQ4 | `functions/_shared/judge_validate.ts` (mail region) | T3a | cloud |
| T3c | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/judge_prompts.ts`, `judge_caps.ts` | T3b | cloud |
| T3d | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-process/**` (new), `config.toml` | T3c | cloud |
| T3e | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-process/**` | T3d | cloud |
| T4.1 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail/**` (new), `config.toml` | T3e | cloud |
| T4.2 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail/**`, `functions/_shared/canary_send.ts` (new) | T4.1 | cloud |
| T4.3 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-canary/**` (new), `config.toml` | T4.2 | cloud |
| T4.4 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-pull/**` (new), `config.toml` | T4.3 | cloud |
| T4.5 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-resolve/**` (new), `config.toml` | T4.4 | cloud |
| T4.6 | `cloud-engineer` (Opus, high) | yes | — | `functions/account/**` | T4.5 | cloud |
| T4b | `cloud-engineer` (Opus, high) | yes | — | `functions/gmail-read/handler.ts` and its test | T4.6 | cloud |
| B | `reviewer` (Opus, high) | yes | — | `docs/reports/…-email-forwarding-cloud-review.md` | T4b | — |
| T14 | `mechanical` (Sonnet, low) | no | UA's values | `app/src/scaffold.rs`, `app/src/onboarding.rs`, `app/tests/{scaffold,onboarding}.rs`, the `Curated` literals in `app/tests/{grades,scheduler}.rs` | T19 in the lane | app |
| T9.1 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/mail_session.rs` (new), `app/tests/mail.rs` (new) | T14, H3a | app |
| T10.1 | `implementer` (Sonnet, high) | no | — | `app/src/mail_steps.rs` (new), `app/tests/mail.rs` | T9.1, H3c | app |
| T9.2 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/mail.rs` (new), `app/tests/mail.rs` | S1–S3, T10.1, H3b | app |
| T10.2a | `implementer` (Sonnet, high) | no | — | `app/assets/mail/m365/` (forwarding), `app/assets/mail/{deno.json,deno.lock,test_dom.ts}` | T9.2 | app |
| T10.2b | `implementer` (Sonnet, high) | no | — | `app/assets/mail/m365/` (mailbox) | T10.2a | app |
| T10.3a | `implementer` (Sonnet, high) | no | — | `app/assets/mail/google/` (forwarding) | T10.2b | app |
| T10.3b | `implementer` (Sonnet, high) | no | — | `app/assets/mail/google/` (mailbox) | T10.3a | app |
| T11.1 | `implementer` (Sonnet, high) | no | — | `app/src/mail_client.rs` (new), `app/tests/mail.rs` | T10.3b, H3d | app |
| T11.2 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (recover, reconcile), `app/tests/mail.rs` | T11.1 | app |
| T11.3 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (the setup runner), `app/tests/mail.rs` | T11.2; then H4 | app |
| T12 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/account.rs` CL, `app/tests/account.rs` | T11.3 | app |
| C | `contract-reviewer` (Opus, xhigh) | yes | — | `docs/reports/…-email-forwarding-contract-review-c.md` | T12, T6.2 | — |
| T11.4 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (commands), `app/tests/mail.rs` | C, `email-engine` merged in; then H5 | app |
| T11.5 | `implementer` (Sonnet, high) | no | — | `app/src/scheduler.rs`, `app/tests/scheduler.rs` | T11.4 | app |
| T15.1–T15.5 | `console-ui` (Sonnet, medium) | no | — | `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs`, `scripts/{wizard,settings}-check.py` | T11.5, the M2 check | app |
| T16 | `cloud-engineer` (Opus, high) | yes | PQ3 | `functions/mail-digest/**`, its pin | B, PQ3 | cloud |
| T13b | `docs-keeper` (Sonnet, medium) | no | — | `HANDOFF.md`, `docs/reference/{app,engine-commands}.md`, `cloud/supabase/README.md`, the legal note, the vision-program note | T15.5, B | lane |
| T17 | `integrator` (Opus, high) | yes | — | the sub-branch merges, `.github/workflows/ci.yml` (one step), the hand-off check | T13b | lane |
| D | `reviewer` (Opus, high), and `contract-reviewer` for late contract diffs | yes | report to Quinn | `docs/reports/…-email-forwarding-whole-branch-review.md` | T17 | — |
| T18 | main session | yes | secrets, sign-ins, merge word | none in the repo | D | lane |

No task goes to `test-writer`: every task writes its own failing tests first. `mechanical` gets T14
only, because it is the one task whose content is fully fixed (two struct fields, one marker).

### T0. Gates (main session, with Quinn)

**Why the main session:** git, the docs push, AWS, DNS, live mailboxes, Quinn's sign-ins and phone
taps all belong to the controller (spec §14, T0). Nothing from T0 enters the repo except the report,
`docs/reports/<date>-email-forwarding-t0-gates.md`. **Rule 1 for the report:** it records header
*shapes* (domains, selectors, ARC instance numbers, `h=` lists, verdict words). Every local part and
display name is replaced by a placeholder. Real captures stay in the session's scratchpad and are
deleted after Checkpoint B's interop check (PQ2).

#### T0.0. Preflight

1. Ask Quinn PQ1, one question with its context. On a yes, rename the branch and move the worktree,
   push the new name, and delete the old remote name. Then ask PQ5 the same way.
2. `git fetch`, then merge `origin/main` at its HEAD (`30b6fb7` or later) into the lane branch, and
   record the sha. The delta carries code (#25, #26, #27), so run the full Rust and Deno gates before
   anything else. Then re-find every name §2 cites, by grep, in `write.rs`, `journal.rs`, `sync.rs`,
   `approvals.rs`, `grades.rs`, `scheduler.rs` and `app/src/main.rs`, and record any that moved.
3. Check that the signing commit carries every §10 edit: A1–A13's markers in the cloud design, VISION's
   seven edits, the Gmail spec's §5 line and the grades spec's markers. Then land those documents and
   the spec on `main` as one docs-only commit (P2). CLAUDE.md is not touched now (H0).
4. Record the base commit in `.superpowers/sdd/2026-09-30-email-forwarding/`.
5. Run `git diff --name-only origin/main...<branch>` for `j-events`, `p3-registrar`, `two-desktop`
   and `c5-relay`, and record every shared file against §10 R2's table, with who rebases each. M2 is
   merged and is no longer checked. Repeat the check before T1.2, T3b, T6.4, T8, T8b and T14.
6. Check whether `knowlu.com` is verified in Resend, since the canary needs Knowlu's DKIM (§4.10). If
   it is not, add the DNS records to Quinn's queue; they are needed before T4.2's live use and T18,
   not before S0.
7. Cut `email-cloud`, `email-engine` and `email-app` with their worktrees (P3), and run the gates in
   each.

**Done when:** the ledger records the base, PQ1's answer, the docs commit on `main`, the disjointness
list and Resend's status, and the three worktrees are green.

#### T0.S0. The receiver: the DMARC gate and the header evidence

**Quinn hand-offs, one at a time, each with its context:**
- **Q-A.** An AWS account, which is a spend. Quinn configures an AWS CLI profile in their own shell;
  the main session uses it by name and never reads a key.
- **Q-B.** The SES receiving region. The main session proposes one that offers SES receiving.
- **Q-C.** Cloudflare DNS: the MX record for `in.knowlu.com` pointing at the region's SES inbound
  host, and the SES domain identity's records. The apex's Cloudflare routing is untouched.
- **Q-D.** The DMARC-failing sender (spec S0's "founder-controlled sending domain"). *Recommended:* a
  throwaway subdomain of `knowlu.com` publishing `p=reject`, sending through a path that fails
  alignment. An example is an SES sending identity on that subdomain with DKIM signing off. Its
  records are removed after S0.

**The main session builds (§5.1's AWS list):**
- a receipt rule whose only action is the S3 action with its `TopicArn` set. Never the SNS action,
  never "stop", never "bounce";
- a bucket with public access blocked, encryption at rest, and a one-day expiry on `inbound/` and
  `work/`;
- an SNS topic. S0 reads it through a temporary SQS subscription, deleted after S0;
- an IAM user limited to get, put and delete on the bucket, with no list.

**The messages sent to a test address:**
- the DMARC-failing message;
- a forward from a mailbox on the M365 test tenant, or from a founder-owned school mailbox if the
  tenant does not exist yet;
- a forward from a founder-owned Gmail;
- one intra-school message (a school sender to a school student), forwarded;
- one forward-as-attachment from each provider.

**It records** everything spec §12.5 S0 lists: the SNS notification's fields, each ARC instance and
its sealing domain, the AMS `h=` lists, the envelope sender's shape and SES's SPF verdict on it,
Microsoft's `ForwardingLoop` tenant field, which `Authentication-Results` each provider stamps at
delivery and how to tell it apart, the intra-school message's alignment, and whether a
forward-as-attachment keeps the delivery headers.

**Pass:** the DMARC-failing message is in S3 with its verdicts recorded, and nothing was bounced or
dropped. **No cloud task starts until S0 passes.** If the intra-school message is not aligned, Quinn
hears the proportion before T2f fixes the `aligned` rule (spec §4.6 step 6).

#### T0.S1–S3. The Tauri 2 host checks

These run in a throwaway Tauri 2 app in the scratchpad, on the app's own Tauri pin and WebView2. The
app has no capability grant, no debugging port and no OS input. Quinn is at the machine for each
sign-in and phone tap. **Each check, and what passes it:**
- **S1a. A script's result comes back without IPC.** Try, in order:
  1. WebView2's `ExecuteScript` with its completion result, reached through `with_webview`;
  2. a title channel: the script writes a tagged result into `document.title`, and Rust reads
     `title()`;
  3. a sentinel navigation, cancelled in the navigation handler.

  Pass: a result of at least 4 KB, from Outlook's message list, reaches Rust, and `__TAURI__` is
  absent from the page. Record the channel and its size limit.
- **S1b. Google's "verify it's you" popup.** Check whether the pinned Tauri exposes a new-window
  handler on the webview builder that opens the popup as a child window in the same profile, with
  `window.opener` intact. If not, try WebView2's `NewWindowRequested` through `with_webview`. Pass:
  the popup opens, Quinn taps the phone, and Gmail's add-address flow continues in the opener. If
  neither route works, Gmail's step 3 is guided (spec risk 4).
- **S1c. The origin check.** The runner reads the top-level URL before it evaluates a script. A
  script queued before a cross-origin redirect does not run on the new origin.
- **S2. Hidden or off-screen.** Compare grades' `visible(false)` with `skip_taskbar(true)` against a
  visible window placed off-screen. Pass: for ten minutes, Outlook's settings page renders, the
  forwarding switch reads by `checked`, and a scripted click takes effect. Record which mode passes.
  Also record:
  - each provider's message-list form of the sender (a display name or an address) and of the
    received time. This fixes the fingerprint's sender form (§4.6 step 7);
  - how long after `close()` a profile folder can be deleted (the basis for `forget`'s retry);
  - that two `data_directory` profiles open one after the other in one process;
  - that the main window stays responsive throughout.
- **S3. Outlook's "open email links" prompt is suppressed.** Try an initialisation script that answers
  `navigator.registerProtocolHandler`, or a WebView2 setting. Pass: the prompt never shows across
  sign-in and settings.

**No task that touches the window or a step script starts until S1–S3 pass** (P4). The report names
the result channel, the hide mode and the popup route that T9.2 and T10.2a build on.

**Cleanup.** The spike's WebView2 profile folders hold real mailbox sessions. When S1–S3 are recorded,
the main session closes the spike app, deletes the spike app and every profile folder it made, and
records the deletion in the T0 report, per the live-proof cleanup rule (§10, R17).

#### T0.S4, T0.S5 and T0.S6

- **S4.** After 2026-10-08, the main session reads the session-lifetime spike's `sessions.log` and
  removes the spike's scheduled task and folder. Quinn then revisits §13 Q3: keep a daily reconcile
  if sessions survive a week, or reconcile only to repair if they die within two days. Either way
  the answer sets the value of one constant, `RECONCILE_CADENCE`, whose two modes T11.5 already
  tests, so no assertion changes (§10, R7).
- **S5 (optional).** Whether an Outlook inbox rule can forward to the Knowlu address beside an
  existing forwarding address. If it can, T10.1 adds the third choice in §4.2 step 3.
- **S6 (Quinn: an M365 developer or trial tenant, §13 Q8).** A few hundred forwards-as-attachment
  at 10 a minute from a tenant mailbox to S0's receiver, by the spike harness and never from a real
  school account. Record throttling, non-delivery reports and any restriction, then set the pace and
  the daily ceiling. T10.1 ships the provisional 10 a minute and 600 a day, and T17 refuses to merge
  numbers S6 has not confirmed.

**Checkpoint 0 (Quinn):** the T0 report with S0 and S1–S3; S6 and S4 are added when they land.

### T19. Ruling 12's gate suspended (D25, A13), its own PR

**Agent:** `implementer` (Sonnet, high). **Why:** `app/src/grades.rs` is off the contract list. The
change is one constant, one pure seam and one delegation, fully specified and pinned by tests. It
carries a signed promise, so `reviewer` reads it (R19) and Quinn sees the diff. **Branch:** its own,
cut from `main` (P19). **Files:** `app/src/grades.rs`, `app/tests/grades.rs`, and in
`app/tests/scheduler.rs` the grades tests only.

**Tests first** (spec tests 39–41, each by the name the spec gives):
- **39.** The six predicate tests:
  - `a_dated_blackboard_row_is_available_with_its_own_host`
  - `an_undated_blackboard_row_is_not_available_yet`
  - `an_uncurated_blackboard_school_is_not_available_yet`
  - `a_curated_canvas_row_is_not_a_blackboard_school`
  - `no_school_is_not_a_blackboard_school`
  - `no_real_campus_is_available_on_this_branch`

  Each keeps today's assertions against `availability_with(Gate::Enforced, ..)` and gains the
  `Suspended` arm's through `availability`. The undated and real-campus tests are renamed to say both
  arms. A source test fails if `cfg`, a feature or `std::env` reaches the gate.
- **40.** In `app/tests/grades.rs`, the undated cases of these three tests are rewritten to
  `Suspended`, and every other case is unchanged:
  - `grades_status_reports_the_gate_and_with_a_date_the_session`
  - `grades_connect_refuses_without_a_date_and_opens_nothing`
  - `grades_refresh_refuses_without_a_date_and_reads_no_session`

  Each is renamed where "with a date" or "without a date" is no longer true.
- **41.** In `app/tests/scheduler.rs`, the changes are exactly as spec test 41 states:
  - `the_grades_decision_asks_the_predicate_and_never_rederives_it`
  - `the_skip_order_is_school_then_availability_then_entitlement_then_session_then_window`
  - `the_slot_records_the_grades_skip_and_runs_and_cleans_up_the_grades_step`

  The date-and-bump tests and `no_curated_row_carries_a_policy_read_date` pass unchanged.

**Behaviour.** Spec §6.2's `grades.rs` item:
- `enum Gate { Enforced, Suspended }` and `POLICY_READ_GATE` set to `Suspended`, the same in every
  build.
- `availability_with(gate, row, lms)` holds the one branch the gate decides.
- `availability` keeps its signature and delegates. Its four callers are unchanged and take no gate.

**Stops:** if a kept assertion cannot pass with the callers unchanged, or if a change would reach an
entitlement test, the implementer stops and asks. No test is deleted.
**Done when:** tests 39–41 pass, the date-and-bump tests are untouched, and R19 has no open finding.

### T13a. HANDOFF for the suspension and the lane (`docs-keeper`, on T19's branch)

**Why docs-keeper:** this is HANDOFF text only. It ships in T19's PR because D18's guard must reach
`main` with the suspension (spec T19). **Edits, by heading, never by line:**
- **§3, Pilot, *Release gate*.** No release is cut from a `main` carrying the Gmail settings row,
  email forwarding to Knowlu, or the suspended grades gate until privacy bump #1 merges (D18).
- **§3, MVP.**
  - Lanes 1 and 6 and the MVP exit row: UA's read leaves the MVP, and the grades proof runs on a dev
    build of `main` (A13).
  - A new lane, "Email: email forwarding to Knowlu", after Gmail connect.
  - The Pilot's and Launch's read rows carry A13's suspended marker.
- **§4, Quinn's queue.** "Record UA's university-policy read" leaves the MVP. A13's review trigger
  joins it: the 20th paying account, checked by the controller's count-only query at each milestone.
  The lane's asks are added one at a time: AWS, DNS, staging secrets, UA's mail fields and term
  dates, and the M365 test tenant.
- **Privacy bump #1's row** gains spec §7's seven drafts and the lawyer-packet additions.

### R19. Review of T19 and T13a (`reviewer`, Opus high)

It reads the diff against D25, A13, tests 39–41 and spec T19's rules (no gate parameter, no deleted
test, the entitlement tests untouched). **Report:**
`docs/reports/<date>-grades-gate-suspended-review.md`. Quinn reads the diff. The PR merges on CI
green and Quinn's word, and the main session then merges `main` into the lane branch.

### T5. The reconciler (`engine/src/mailreconcile/`), in five dispatches

**Agent, all five:** `contract-engineer` (Opus, xhigh). **Why:** it is off the list, but it is the
one place that decides direct write or card (D11). A silent error there changes a student's plan
with no card, which spec risk 1 calls the worst harm in the lane. **Rules for all five:**
- The module is pure: no file, clock or network access. `today` and the vault day are parameters, so
  a grep for `std::fs`, `Instant` and `Zoned::now` in the directory finds nothing.
- Its output is an ordered `Vec<Action>`, and nothing else decides card or direct. The view's
  `human_set` is a snapshot. T6.2's direct-set writer re-checks it, from a fresh journal read, just
  before it writes, and files the amend card instead if a human set the field in between (§10, R10).
  That re-check never turns a card into a direct write.
- `recover` is treated exactly as `live`. A batch that finishes an interrupted backfill is opened as
  phase `backfill` (T11.3), so its items arrive as `backfill`.
- Each dispatch ends in its own commit, for Checkpoint A.

**Before T5.1:** hand-off H1a.

#### T5.1. The types, the order and determinism (`mod.rs`)

**Tests first** (spec test 18, first two halves):
- `the_same_vault_and_claims_plan_the_same_actions_twice`
- `a_shuffled_delivery_plans_the_same_actions`
- `a_claim_with_a_line_break_in_any_field_is_skipped_as_noise` (§10, R11): a title, sender, why or
  evidence that `write::single_line_problem` refuses (`\r`, `\n`, U+0085, U+2028, U+2029 and every
  other break `pystr::splitlines` makes) gives a skip and no other action.

**Behaviour.** Spec §6.1's input and output, and §4.8's order.
- The vault view: per note, its id, path, folder, course, title, due, status, `source_uid`,
  `created_by`, `mail_asof`, and the set of fields `Journal::human_set` finds.
- Messages carry their uid, ordering date, message date, phase, authenticity word, sender and claims
  (§5.1's claim shape).
- `Action` covers create, set (with the `mail_asof` stamp and the body line), task card, amend card
  (with `status`, `proposed_at`, `first_proposed_at` and `snooze_until`), notice (kind, `close_key`),
  propose-done, and skip with a reason. The reasons are superseded, past, already applied, equal,
  noise and unmatched.
- `plan(view, messages, today, vault_day)` sorts by ordering date, then uid, then claim index.

This first commit holds the types, the ordering and test 18 over a rule stub. It is what T6 builds
against (spec T5). Test 18 runs unchanged after every later dispatch.

#### T5.2. Resolving a claim's target (`resolve.rs`)

**Tests first** (spec tests 24 and 22e, the pure halves, plus plan tests for §4.8 step 2):
- `a_target_equal_to_a_source_uid_or_lms_id_resolves`
- `a_normalised_title_in_the_claims_course_resolves`
- `the_one_active_note_holding_every_token_resolves`
- `zero_or_two_matches_are_ambiguous`
- `a_stored_resolution_wins_and_a_stored_none_is_unmatched` (D12)
- `a_new_item_whose_title_equals_an_active_note_is_a_change_to_its_due` (24)
- `a_claim_whose_source_uid_is_already_on_a_note_or_card_writes_nothing` (22e)

**Behaviour.** §4.8 steps 2 and 4. Normalisation folds case, punctuation and whitespace. The result is
one of: a note, ambiguous (listed for `mail-resolve`, batched per course by T6.4), or unmatched.
A uid already on a note in `tasks/`, `archive/` or `approvals/` is skipped as already applied.

#### T5.3. §4.9's `new_item` rows, authenticity and the first day (`rules.rs`)

**Tests first** (spec tests 20's `new_item` rows, 23, 22b, and 22d's planning half):
- `a_backfill_task_ahead_or_undated_is_created_direct`
- `a_live_task_is_a_task_card`
- `a_past_task_backfill_is_created_archived_imported_past_and_live_is_a_card` (23)
- `a_future_event_is_a_card_in_both_phases`
- `a_past_event_backfill_is_archived_imported_past_and_live_writes_nothing_and_counts` (23, D24)
- `an_unaligned_backfill_original_makes_cards_where_an_aligned_one_writes_direct` (22b)
- `a_first_day_card_is_proposed_and_snoozed_to_day_two` (22d): on vault day 1 a card carries
  `status: snoozed`, `proposed_at` and `snooze_until` set to day 2, and `first_proposed_at` set to day
  1. `approvals::transition_note` wakes only `status: snoozed`, so a `pending` card would show on day 1
  (§10, R16). On any other day a card is `status: pending` with `proposed_at` and `first_proposed_at`
  today. Direct writes and notices are not held.

**Behaviour.** §4.9's first four rows, §4.6 step 6's rule that only an aligned backfill original may
write directly, D24, and §4.8 step 5's first-day rule. `exam` follows the task-shaped rows. A
created note carries the claim's `effort_hours` and `importance` (§5.2).

#### T5.4. §4.9's `change` rows for mail, the student, other notes and `cancelled` (`rules.rs`)

**Tests first** (spec tests 20's change rows, all using `due`; 21's planning half; 22c):
- `a_backfill_change_to_a_mail_note_field_not_set_by_the_student_is_direct`
- `a_live_change_to_a_mail_note_is_an_amend_card`
- `a_hand_created_task_gets_an_amend_card_for_due_in_both_phases` (21: a human `create` carrying `due`)
- `a_hand_set_due_gets_an_amend_card_in_both_phases` (21: a human `set`)
- `a_change_to_a_gmail_or_events_note_is_an_amend_card`
- `cancelled_is_an_archive_card_in_both_phases_and_never_direct` (22c: `status: active → archived`,
  and no new field)
- `two_claims_with_one_ordering_date_and_different_values_make_one_card_naming_both`

**Behaviour.** §4.9's rows 5, 6, 10, 11 and 12, and D11 (1) and (2): card or direct is decided from
the view's `human_set` set, never from `write`'s judged set. For the two-value row, the card proposes
the value of the claim that sorts last by uid and claim index, and its body names both values, each
with its sender. That is this plan's deterministic reading of "naming both".

#### T5.5. Feed items, templates, supersession and notices (`rules.rs`)

**First step:** grep every `created_by:` the engine's templates write, and record the list in the
ledger (P10).
**Tests first** (spec tests 22, 19, and 26's planning half, plus P10 and D24 pins):
- `a_model_change_to_an_lms_item_is_a_card`
- `an_lms_item_already_holding_the_value_writes_nothing`
- `a_backfill_template_change_to_a_fed_item_writes_nothing_and_counts_superseded`
- `a_backfill_template_change_to_an_unfed_note_is_direct`
- `a_live_template_change_is_a_card_unless_equal`
- `a_claim_not_later_than_mail_asof_is_superseded` (19, across pulls)
- `a_live_claim_older_than_an_applied_backfill_value_is_superseded` (19)
- `a_future_dated_message_orders_at_its_received_time` (19)
- `course_info_opens_a_notice_keyed_mail_course_topic` (26)
- `an_unmatched_claim_is_a_notice`
- `feed_creators_hold_the_feeds_and_nothing_else` (P10)
- `no_action_ever_targets_commitments` (D24)
- `a_template_receipt_is_propose_done_in_both_phases` (§4.9's receipt row, under PQ4)

**Behaviour.** §4.9's rows 7–9 and 13–15, D13 and D24. Values are compared in the vault's literal
form (`write::to_literal`), so a date and a date-time never compare equal by accident.

### T8. Approving a mail amend card writes as the student (`approvals.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** `approvals.rs` is on the contract list, and a
wrong actor here either breaks judge-once or stops every approval. **Before it:** PQ5 is answered,
and T0.0's disjointness check is repeated for `j-events` and `two-desktop`, which both edit
`approvals.rs`. If either merged first, this arm is rebased onto it (spec §9); otherwise they rebase
onto this lane (§10, R2).
**Tests first** (spec test 31, plus P11 pins):
- `an_approved_mail_amend_writes_its_fields_as_the_human_actor`: the record's actor is
  `journal::read_human_actor`, and its `via` is the executor's.
- `a_bad_actor_file_leaves_the_mail_card_pending_with_ruling_11s_error`: nothing is written, and no
  other actor is tried.
- `a_non_mail_amend_executes_exactly_as_today`
- `an_approved_mail_task_card_materialises_as_agent_approvals`
- `the_mail_cards_own_status_bookkeeping_stays_agent_approvals`
- `a_mail_card_carrying_append_is_refused_and_writes_nothing` (§10, R15): the refusal is recoverable,
  so the card stays pending where the student can reject it, and no `append_body` record is written
  under any actor.
- Under PQ5's answer (c): `title_is_amendable_only_on_a_blackboard_card`,
  `a_title_amend_on_any_other_card_is_refused_exactly_as_today`, and
  `amendable_fields_is_exactly_the_nine` passing unchanged. Under (a) or (b), the tests that answer
  names instead.

**Behaviour.** D11 (4), §5.3, P11, PQ5.
- In the executor's `amend` arm, a card with `created_by: mail` gets a context whose actor is the
  vault's human token, with the executor's `via` and run id. That context goes to `apply_amendment`.
- A failed read of the token leaves the card pending with the named error, as a recoverable refusal
  does today.
- Every other kind and every other amend is untouched, beside the existing `created_by` arm for sync.
- A mail card never carries `append` (T6.2 writes none). The arm refuses one that does, before
  `apply_amendment` runs, so `apply_amendment` itself is not changed and no append line is ever
  journalled as the human.

### T8b. `ingest` honours ruling 3 for `title` and `due` (`ingest.rs`)

**Agent:** `contract-engineer` (Opus, xhigh), reviewed at Checkpoint A. **Why:** it changes the feed's
behaviour in every existing vault, and it writes cards that `approvals::validate_amendment` must
accept (spec T8b).
**Before it:** PQ5 is answered, and the disjointness check is repeated for `two-desktop`, which edits
`ingest.rs` (§10, R2).
**First step, before any code:** check that no fixture vault, oracle or golden test runs `ingest` over
a hand-edited LMS `title` or `due` the feed contradicts. If one does, stop and ask: frozen references
are never regenerated.
**Tests first** (spec test 31b):
- `a_hand_set_due_gets_one_amend_card_from_the_feed_and_keeps_its_value`
- `a_second_ingest_files_no_second_card`
- `a_rejected_feed_value_is_not_proposed_again_until_the_feed_changes` (P12)
- `a_hand_set_title_behaves_the_same`
- `an_unedited_due_is_overwritten_exactly_as_today`
- `the_feed_card_passes_validate_amendment_and_never_says_rejudged`
- `approving_the_feed_card_moves_due_and_a_later_feed_move_is_a_card_again`
- `a_hand_set_title_and_due_both_contradicted_file_two_one_field_cards` (§10, R14)
- `a_pending_mail_due_card_blocks_the_feed_due_card_and_the_reverse` (§10, R14)
- `a_hand_cleared_due_or_a_feed_cleared_due_keeps_the_value_and_files_no_card` (§10, R14)
- `the_feed_card_is_charged_to_the_daily_budget` (§10, R3): sixteen contradicted notes in one run,
  then `defer_over_budget` at 15, leave one card `status: snoozed` and none deleted.

**Behaviour.** Spec §6.1's `ingest.rs` item, D11 (5), P12 and PQ5's answer.
- The change is in `sync_tasks`' update branch only. Each field `Journal::human_edited` finds, where
  the feed differs, is filed on its own `kind: amend` card through `write::create`. Fields no human
  edited are written exactly as today, in the same pass.
- The card is `status: pending`, with `proposed_at` and `first_proposed_at` the run's day.
- A null on either side files nothing and keeps the note's value.
- The card carries the ingest actor and this "Why proposed" line: "Blackboard now says <to>; you set
  <from> by hand, so this is a proposal". It is never filed through `write::propose_amendment`.
- The run line says `proposed <stem>: <field>`, one line per card.

### Checkpoint A. Contract review of T5.1–T5.5, T8 and T8b

**Agent:** `contract-reviewer` (Opus, xhigh). **Report:**
`docs/reports/<date>-email-forwarding-contract-review-a.md`. The review:
- reads one commit at a time, against D10–D13, D20, D24, §4.8 steps 2, 4 and 5, §4.9 row by row,
  §5.2–§5.3, D11 (4) and (5), ruling 3, P10–P12, and T5.4's two-value reading;
- confirms the module is pure, card or direct is decided only in `rules.rs`, `JUDGED_FIELDS_TASK` and
  `write.rs` are untouched, and the fixtures diff is empty;
- confirms the oracle, golden, sync and entitlement tests ran unchanged;
- reads PQ5's arm, T8's `append` refusal, and T8b's per-field cards, null rule and budget fields
  (§10, R3, R9, R14 and R15);
- confirms that a mail or feed `append_body` landing while M2's body editor is open makes the editor's
  `set_body` refuse as a conflict, as M2 intends, and never drops the line (§10, R1).

T6.1 runs while the reviewer reads, since the review only reads commits. The fix loop opens only after
T6.1 has committed, so one agent at a time writes in `email-engine` (§10, R6). Fixes go back to
`contract-engineer`, and the reviewer re-reads them. **Quinn** reads the report with
the diffs (spec §14's checkpoint). T6.2 starts when no Critical or Important finding is open.

### T6. The mail pass (`cloudmodel.rs`, `engine/src/mail.rs`, one call in `enrich.rs`), in four dispatches

**Agent, all four:** `implementer` (Sonnet, high). **Why:** it is off the list, specified by spec
§4.8, §5.2 and §6.1, and every behaviour has a test. Decisions are made only by T5's `plan`; this
code reads, calls and writes. T6.2's card writer is read by contract-reviewer at Checkpoint C (P13).
**Rules for all four:**
- The actor is `agent:knowlu.mail` (D20) for every write, with the slot's `via`.
- Every write goes through `write` (journal first). Every mail-derived string goes through
  `write::to_literal`.
- Every loopback server is on `127.0.0.1:0`.

#### T6.1. The two service calls (`cloudmodel.rs`)

**Tests first** (spec test 30's client half, beside `pull_gmail_queue`'s own tests):
- `pull_mail_claims_over_loopback_returns_messages_in_order_with_mailbox_states`
- `every_mail_failure_shape_is_a_named_outcome`: no session, 402, 429, 5xx and a timeout.
- `a_mail_pull_never_reads_as_gmail_quiet`
- `resolve_mail_over_loopback_returns_one_answer_per_claim`

**Behaviour.** `pull_mail_claims(client, ack)` and `resolve_mail(client, batch)` (spec §6.1). The pull
answer carries the messages (§4.7 step 6's row) and each mailbox's state, `reenabled_at` and counters
(§4.10's *Shown*). Mail has no `Quiet` arm: a mailbox's state is data, never an error.

#### T6.2. The writers (`mail.rs`)

**Before it:** hand-off H1b. **Tests first** (spec tests 27, 21's writer half, 22d's write half, 26,
the first half of 28c, plus P13 pins):
- `mail_asof_is_a_single_line_flow_mapping_and_no_other_byte_moves` (27, P17)
- `a_second_claim_while_a_mail_card_is_pending_files_no_second_card` (21)
- `a_first_day_card_is_written_snoozed_to_day_two` (22d)
- `a_later_notice_closes_the_earlier_by_close_key` (26)
- `a_mail_note_is_written_needs_enrichment_false_with_the_claims_effort_and_importance` (28c)
- `a_mail_amend_card_passes_validate_amendment_and_carries_from_mail_date_and_evidence` (D11 (3),
  D21)
- `a_colon_a_quote_and_an_img_onerror_in_evidence_survive_as_literals`, widened (§10, R11) to every
  mail-derived field (title, sender, why, evidence) and to `\r`, U+0085 and U+2028: a value T5 lets
  through lands as one literal line, and one T5 skips writes no byte.
- `a_direct_change_sets_the_field_and_mail_asof_and_appends_one_body_line` (§5.2)
- `a_human_set_landing_between_plan_and_apply_turns_the_direct_set_into_a_card` (§10, R10): the test
  plans, writes a human `set` through `write` with a second `Journal`, then applies, and finds a card
  and the human's value intact.
- `mail_asof_reapplied_from_its_journal_record_is_byte_identical` (§10, R13)
- `a_mail_body_line_under_an_open_editor_makes_set_body_refuse_as_a_conflict` (§10, R1)
- `a_first_day_card_is_written_snoozed_to_day_two` asserts `status: snoozed` as well (§10, R16).

**Behaviour.** One function per `Action` kind, each through `write`:
- **Create.** `write::create` with `GMAIL_NOTE`'s shape and §5.2's differences.
- **Direct set.** First `journal.invalidate()` and a fresh `Journal::human_set(id, field)`; if a human
  set the field since the view was built, the card writer runs instead. Otherwise `write_literals`
  with `WriteOpts::default()` for the field and `mail_asof`, then `append_body` for the line
  `<date> · <sender>: <why> — "<evidence>"` (§10, R10).
- **Cards.** The card writer uses `write::create` after `write::find_pending_amendment`. The task card
  has `write_gmail_card`'s shape; the amend card has D11 (3)'s keys, plus `created_by: mail`,
  `source_uid`, `judgment_id`, `judgment_kind: mail_claim`, `from`, `mail_date` and `evidence`, and
  never `append`. A card carries the `status`, `proposed_at`, `first_proposed_at` and `snooze_until`
  T5 planned, so `defer_over_budget` charges it (§10, R3, R15, R16).
- **Notices.** `info::open_info` and `close_info`, with `opened_by: agent:knowlu.mail`.
- **Receipts.** `completion::propose_done`.

#### T6.3. The pass: view, pull, apply, ack, line (`mail.rs`)

**Tests first** (spec tests 28, 18's third half, 22e's vault half, the second half of 28c, and the
engine half of 36), in `engine/tests/mail_pass.rs`:
- `the_mail_run_line_is_exact`: `mail: N new, M changed, K proposed, J notices, P past, S superseded, U
  unmatched`.
- `no_connected_mailbox_prints_nothing`
- `seen_records_mail_uids_and_the_ack_sends_them`
- `a_closed_batch_drains_past_three_rounds_until_the_budget_is_spent`
- `a_backfill_split_across_pulls_gives_the_same_vault_as_one_pull` (18; the service's withholding is
  faked at the loopback)
- `a_claim_already_on_a_synced_note_or_card_writes_nothing` (22e)
- `judge_makes_no_judge_task_call_for_a_mail_note` (28c, a spy loopback)
- `a_non_active_mailbox_keeps_one_heads_up_and_names_its_line` (§4.10)
- `a_reenabled_mailbox_opens_one_heads_up_with_the_exact_text` (36's engine half): "Email forwarding
  was off; Knowlu turned it back on. Turn off email in Settings to stop this."
- `twenty_mail_cards_are_charged_and_overflow_is_snoozed_by_defer_over_budget` (§10, R3): on a vault
  past its first day, a live pull of twenty card claims, then `approvals::defer_over_budget` at 15,
  leaves five `status: snoozed` with
  `proposed_at` tomorrow and `first_proposed_at` unchanged, and none deleted.
- `the_seen_ledger_records_mail_uids_as_email_and_a_hostile_title_cannot_add_a_line` (§10, R11): a
  claim titled `Quiz` + U+2028 + `- bb-uid-123 · x` leaves `ingest::load_seen` holding the mail uid and
  no `bb-uid-123`.

**Behaviour.** `pull_mail(vault, client, opts, budget)` (§4.8 steps 1, 4–7).
- It builds T5's view from `tasks/`, `archive/` and `approvals/`, with `Journal::human_set` for each
  field a claim may touch.
- Live mail takes up to `PULL_ROUNDS`. A closed batch drains until `budget`, by `pull_gmail`'s
  before-the-round check.
- It calls `mailreconcile::plan`, applies the actions through T6.2's writers, acks, and records each
  `mail:<h>` in `state/ingest-seen.md` through `ingest::record_seen` with the fixed title `(email)`,
  never a mail-derived title (§10, R11; the Gmail pass's own word for an item it did not write).
- Heads-up items use `close_key: mail-source:<address_id>` and `mail-reenabled:<address_id>`.

#### T6.4. Resolve batching, the one `enrich.rs` call, labels

**Before it:** the disjointness check is repeated for `two-desktop`, which edits `enrich.rs` (§10, R2).
**Tests first** (spec tests 25, 28b, and 30's `judge` half):
- `one_resolve_call_per_course` (25, a scripted fake)
- `a_none_answer_is_a_notice` (25)
- `a_stored_resolution_replays_with_zero_calls` (25)
- `a_rejected_mail_card_is_never_reported` (28b; `labels_to_report` is unchanged)
- `judge_runs_the_mail_pass_after_the_gmail_pull_and_exits_zero_on_every_mail_failure` (30)

**Behaviour.** Ambiguous claims from T5.2 are batched per course to `resolve_mail` before `plan`
(§4.8 step 3). `enrich::run_lines_with` gains one call to `mail::pull_mail` after `pull_gmail`, inside
the same entitlement gate. That hunk is the only edit to `enrich.rs` (spec §6.1).

### T7. `knowlu-engine mail-applied` (`engine/src/mailapplied.rs`)

**Agent:** `implementer` (Sonnet, high). **Why:** read-only and specified. **Before it:** H1c. **After
it:** H2 (the subcommand's dispatch).
**Tests first** (spec test 29):
- `mail_applied_lists_each_mail_write_joined_to_the_notes_current_value`
- `a_note_changed_since_or_gone_reads_changed_since`
- `mail_applied_leaves_the_vaults_bytes_and_journal_unchanged`

**Behaviour.** Spec §4.14 and §6.1. It reads every journal record by `agent:knowlu.mail` since
`--since` (by default, the first such record). It groups them by course: items created, fields
changed (old → new, sender, date), past items archived, and notices opened. It prints JSON through
`ledger::dumps_value`. It never writes and is never entitlement-gated, like `surface`.

### T1. The schema (`<stamp>_mail.sql`, `migrations_test.ts`), in three dispatches

**Agent, all three:** `cloud-engineer` (Opus, high). **Why:** `cloud/` is theirs, and a wrong grant
leaks every student's mail metadata (spec T1). **After:** S0. The stamp sorts after every migration on
`main` when T1.1 starts. T17 re-checks it still sorts last; nothing is applied anywhere until T18.
`migrations_test.ts` is textual, so SQL behaviour is pinned by text and smoke-tested on staging (P7).

#### T1.1. The seven tables

**Tests first** (spec test 14, the RLS half):
- `every_mail_table_has_rls_on_and_no_anon_or_authenticated_grant`
- `every_mail_account_id_cascades_from_accounts`
- `mail_seen_is_keyed_by_account_and_msg_hash_and_token_hmac_is_unique`

**Behaviour.** Spec §5.1's table, column for column: `mail_accounts`, `mail_addresses`,
`mail_batches`, `mail_canaries`, `mail_seen`, `mail_work` and `mail_claims`. `mail_batches` gains its
`phase` (`backfill` or `recover`) as the spec lists it.

#### T1.2. Kinds, origins, the training and rules paths, the provisional pins

**Before it:** the disjointness check is repeated for `j-events`, which edits `migrations_test.ts` and
adds a migration (§10, R2). The `mail` migration's stamp sorts after `j-events`' if that has merged.
**Tests first** (spec test 14, the constraint half, plus P8):
- `models_usage_daily_and_judgments_accept_the_three_mail_kinds`
- `judgments_origin_accepts_forward`
- `corrections_rules_and_both_eval_tables_still_refuse_mail_kinds`
- `promote_rules_and_the_corrections_backfill_pass_over_forward_judgments`
- The existing `the last definition of export_training_rows excludes …` test, retitled and checking
  `origin not in ('gmail_api', 'events', 'forward')`. This is stronger, never looser (spec §8).
- `mail_claim_and_mail_resolve_are_pinned_provisionally_to_the_email_models_route` (P8;
  `lastModelPinBlocks` widened to the new kinds)

**Behaviour.** Spec §5.1's "Changes to existing objects", constraint by constraint.
`telemetry/handler.ts`'s list is not touched.

#### T1.3. The RPCs, the budget and the cron

**Tests first** (spec test 14's RPC half, and 11's database half):
- `delete_mail_address_revokes_then_deletes_one_mailboxs_rows_delivered_claims_included`: it is
  `security definer` with a pinned `search_path`.
- `charge_allowance_is_one_statement_like_charge_call`
- `enforce_budget_raises_the_ceiling_by_the_allowance_only_while_it_is_open` (D23: 2,000 judgments and
  $1.00 per account per term, within 14 days of the first setup)
- `the_mail_cron_jobs_use_the_vault_token`: `mail-process` every minute, `mail-canary` hourly, and
  `mail-process` in sweep mode nightly (P7).

### T2. Intake (`mail-inbound` and its shared modules), in eight dispatches

**Agent, all eight:** `cloud-engineer` (Opus, high). **Why:** student mail, signatures and secrets
(spec T2). **Rules for all eight:**
- Every external party is faked on `127.0.0.1:0`: S3, SNS's certificate host, Google's confirmation
  page and DNS (an injected resolver).
- S3 is a spy that counts GETs.
- No log line, error or answer carries a body, header value, address or token (test 15).
- Every path that ends a message deletes its object (test 16).
- The provider facts (signing domains, return-path shapes, the `ForwardingLoop` field, the delivery
  stamp) come from the T0 report.

#### T2a. SNS verification (`_shared/sns.ts`)

**Tests first** (spec test 1):
- `a_valid_signature_from_the_local_cert_host_is_accepted`
- `a_wrong_topic_is_refused`
- `a_bad_signature_is_refused`
- `a_cert_url_outside_https_sns_region_amazonaws_com_is_refused`
- `a_subscription_handshake_is_confirmed_only_for_the_configured_topic`

**Behaviour.** D2 and §4.6 step 1. The certificate is fetched through an injected fetcher and only from
an allowed host. Every refusal answers 403 and reads nothing more.

#### T2b. S3 by SigV4 (`_shared/s3.ts`)

**Tests first** (spec test 16, the SigV4 half):
- `sigv4_get_put_and_delete_against_a_local_fake_bucket`
- `a_failed_put_surfaces_as_an_error_the_handler_answers_500`
- `the_client_offers_no_list_operation`

**Behaviour.** §5.1's IAM scope: get, put and delete on `MAIL_BUCKET`, and nothing else.

#### T2c. MIME (`_shared/mime.ts`)

**Tests first** (plan tests that tests 4 and 7 rest on):
- `parses_from_date_subject_message_id_list_headers_and_precedence`
- `takes_the_first_text_plain_part_else_reduces_html_to_text`
- `extracts_each_message_rfc822_attachment_and_opens_no_other_part`
- `reads_the_topmost_received_time`
- `a_message_without_a_message_id_keys_on_date_from_and_subject` (D20)

**Behaviour.** §4.7 step 1's parse, plus the parts intake needs: attachments for a batch send, and
the delivery time for §4.6 step 7.

#### T2d. Signature verification (`_shared/dkim.ts`; under PQ2)

**Tests first:**
- `verifies_relaxed_and_simple_rsa_sha256_signatures_made_with_an_in_test_key`
- `a_changed_signed_header_fails`
- `a_header_absent_from_h_is_not_covered`
- `the_highest_arc_instances_seal_verifies_and_reports_its_domain`
- `the_key_resolver_is_injected_and_no_test_queries_dns`

**Behaviour.** PQ2's recommended scope. In production the resolver uses `Deno.resolveDns` for TXT
records. **After it:** the main session runs the interop check over S0's captures (PQ2).

#### T2e. Binding, part 1: canary, Gmail confirm, batch send (`_shared/mail_bind.ts`)

**Tests first** (spec tests 4, 5 and 6, the recognition halves):
- `a_send_from_the_mailbox_with_aligned_outer_dkim_and_an_open_nonce_is_a_batch_send` (4)
- `a_wrong_outer_sender_is_not_a_batch_send` (4)
- `the_mailboxs_own_gmail_confirmation_is_recognised` (5)
- `notjane_is_not_jane` (5)
- `a_null_hint_is_refused` (5)
- `a_closed_confirm_window_is_refused` (5)
- `a_sender_other_than_googles_exact_sender_or_without_aligned_dkim_is_refused` (5)
- `a_link_off_the_allow_list_is_refused` (5)
- `a_canary_with_its_nonce_and_knowlus_dkim_is_recognised_with_its_forward_hint` (6)
- `a_canary_from_a_wrong_sender_domain_is_not_a_canary` (6)

**Behaviour.** The first three cases of §4.6 step 5.

#### T2f. Binding, part 2: live forwards and authenticity (`_shared/mail_bind.ts`)

**Tests first** (spec tests 2's forgeries, and 3):
- `a_gmail_forward_binds_by_googles_arc_seal_and_its_caf_return_path`
- `an_m365_forward_binds_by_microsofts_arc_forwardingloop_and_srs_return_path`
- `a_direct_send_with_a_forged_x_forwarded_for_is_unbound`
- `a_forward_sealed_by_google_from_an_attackers_own_gmail_is_unbound`
- `an_m365_forward_naming_another_tenant_than_forward_hint_is_unbound`
- `a_dmarc_failing_original_with_an_aligned_provider_hop_result_is_aligned` (3)
- `a_pass_only_in_a_sender_written_authentication_results_is_unaligned` (3)
- `a_backfilled_original_reads_only_the_providers_delivery_stamp_or_arc`
- `no_authentication_result_ever_produces_a_non_2xx_answer` (3)

**Behaviour.** §4.6 steps 5 (the live case and "anything else") and 6. The words recorded are
`aligned` or `unaligned`, never a header.

#### T2g.1. The handler: verify, find, entitle, fetch, dedup, hand off (`mail-inbound`)

**Tests first** (spec tests 1, 2, 7, 15 and 16, the intake halves):
- `a_bad_notification_answers_403_and_reads_nothing`
- `an_unknown_or_revoked_token_is_deleted_with_zero_s3_gets`
- `a_lapsed_accounts_message_is_deleted_unread_with_no_seen_row_and_lapsed_since_set`
- `a_retired_token_inside_its_rotate_grace_still_binds`
- `no_forgery_moves_last_received_at_or_charges_a_cap`
- `one_message_id_twice_makes_one_work_item` (7)
- `seen_hashes_are_hmacs_under_the_accounts_key` (7: the same Message-ID under two accounts hashes
  differently)
- `a_failed_put_answers_500_and_writes_neither_work_nor_seen` (7)
- `no_intake_log_line_carries_the_messages_marker` (15)
- `no_object_is_left_after_unbound_revoked_unauthenticated_and_duplicate` (16)

**Behaviour.** §4.6 steps 1–4 and 7–8 (D8). The `[functions.mail-inbound]` section goes in with its
comment (P5).

#### T2g.2. The handler: what each arrival does (`mail-inbound`)

**Tests first** (spec tests 4, 5 and 6, the effect halves):
- `each_rfc822_attachment_becomes_one_work_item_counted_toward_its_batch` (4)
- `a_pdf_attachment_is_ignored` (4)
- `an_unaligned_backfilled_original_is_kept_unaligned` (4)
- `the_confirmation_is_completed_by_a_cookieless_get_and_post_to_a_local_fake` (5)
- `a_confirmation_redirect_is_refused` (5)
- `a_form_action_off_the_allow_list_is_refused` (5)
- `a_canary_marks_arrived_and_records_forward_hint` (6)
- `a_replayed_canary_changes_nothing` (6)

**Behaviour.** The effect of each classification in §4.6 steps 5 and 6.

### T3. Claims (`mail-process` and the judge files), in five dispatches

**Agent, all five:** `cloud-engineer` (Opus, high). **Why:** the screen and the model path, and spend
(spec T3). Each shared judge file has one owning dispatch (spec §14).

#### T3a. Templates (`_shared/mail_templates.ts`)

**Tests first** (spec test 9):
- `an_aligned_lms_due_date_changed_mail_yields_a_template_claim_with_a_resolved_due`
- `the_same_mail_unaligned_goes_to_the_model`
- `a_template_matches_only_the_vendors_exact_address_subject_and_body`
- `a_submission_receipt_template_yields_field_completed` (PQ4)
- `the_code_names_no_vendor` (a source test: vendors are data rows)

**Behaviour.** §4.7 step 3. Dates go through `judge_due.ts`, and course codes are matched against the
known courses. `lms_receipts.ts` is not edited.

#### T3b. Validation (`_shared/judge_validate.ts`, the mail region)

**Before it:** the disjointness check is repeated for `j-events`, which edits `judge_validate.ts` and
`judge_prompts.ts`. `MailKind` is checked against `j-events`' `Kind` and validators, and T3b and T3c
rebase onto them if they merged first (§10, R2).
**Tests first** (spec test 10, and 13's validation half):
- `evidence_not_verbatim_after_whitespace_folding_is_refused_incomplete`
- `a_line_separator_or_control_character_in_title_from_or_evidence_is_refused` (§10, R11): `\r`, `\n`,
  U+0085, U+2028, U+2029 and C0 controls other than tab.
- `an_unknown_course_reads_null`
- `an_unresolvable_due_reads_null`
- `a_sixth_claim_is_dropped`
- `a_claim_below_point_six_is_refused`
- `a_new_item_without_effort_or_importance_in_range_is_refused`
- `a_topic_outside_the_fixed_set_is_refused`
- `the_model_can_never_emit_field_completed` (PQ4)
- `a_resolve_answer_naming_an_id_outside_its_batch_reads_none` (13)

**Behaviour.** §5.1's claim shape and D9, with `MailKind` (P6).

#### T3c. Prompts and caps (`judge_prompts.ts`, `judge_caps.ts`)

**Tests first** (spec test 11's cap half, plus plan pins):
- `the_mail_claim_prompt_holds_only_scrubbed_text_cut_to_2000_characters`
- `the_resolve_prompt_holds_only_ids_titles_and_dues`
- `mail_daily_caps_are_120_for_claims_and_20_for_resolve`
- `the_existing_kinds_caps_are_unchanged`

**Behaviour.** The prompt and grammar `mail-1` (§4.7 step 4), the resolve prompt (D12), and P6's caps.

#### T3d. The worker, part 1: parse, screen, templates (`mail-process`)

**Tests first** (spec tests 8, 15 and 16, the process halves):
- `bulk_mail_from_an_off_list_sender_is_noise_with_zero_model_calls` (8)
- `an_edu_sender_with_list_unsubscribe_passes_on` (8)
- `a_microsoft_bulk_score_of_seven_or_more_is_noise`
- `a_promoted_rule_answering_noise_is_noise`
- `the_students_own_mail_outside_a_batch_is_noise`
- `no_process_log_line_carries_the_messages_marker` (15)
- `no_object_is_left_after_noise_refused_template_only_or_unauthenticated` (16)

**Behaviour.** §4.7 steps 1–3, plus the step 6 deletes. The `[functions.mail-process]` section goes
in (P5).

#### T3e. The worker, part 2: charge, model, queue, expire, sweep (`mail-process`)

**Tests first** (spec tests 11 and 7's expiry half, plus P7's sweep):
- `a_backfill_item_charges_the_allowance_not_the_daily_cap`
- `the_2001st_backfill_item_waits_for_the_daily_cap`
- `charge_and_enforce_budget_run_before_the_spy_model`
- `a_capped_item_makes_no_model_call_and_waits_to_the_next_utc_day_at_most_three_times`
- `allowance_calls_add_tokens_to_usage_daily_but_no_calls`
- `the_raised_ceiling_ends_with_the_allowance`
- `disconnect_and_reconnect_do_not_renew_the_allowance`
- `an_expired_item_loses_its_seen_row_and_a_later_recover_is_accepted` (7)
- `a_queued_row_holds_the_claims_and_metadata_and_no_text` (§4.7 step 6)
- `the_sweep_closes_timed_out_batches_and_prunes_by_age` (P7)

**Behaviour.** §4.7 steps 4–6 (D8, D9, D23) and P7.

### T4. Endpoints (`mail`, `mail-canary`, `mail-pull`, `mail-resolve`, `account`), in six dispatches

**Agent, all six:** `cloud-engineer` (Opus, high). **Rules for all six:**
- Authentication follows spec §6.3: `requireUser` for `GET /addresses` and `DELETE /addresses/<id>`;
  the session plus `requireActiveEntitlement` for every other device route; the Vault token for the
  cron functions.
- Each new directory gets its `config.toml` section (P5).
- No token, key or hash ever leaves in an answer, except the token itself to its own account's
  Settings (D3).

#### T4.1. Addresses: list, mint, hint, rotate, delete (`functions/mail/`)

**Tests first** (spec tests 11's address half and 16b, plus D3, D17 and P21 pins):
- `a_fourth_live_address_is_refused_by_name`
- `a_seventh_mint_in_30_days_is_refused_by_name`
- `get_and_delete_addresses_succeed_for_a_canceled_account` (16b)
- `every_other_mail_route_answers_402_without_entitlement` (16b)
- `a_minted_token_is_26_lowercase_base32_characters_kept_as_hmac_and_ciphertext`
- `rotate_keeps_the_old_token_until_the_new_canary_or_seven_days`
- `delete_reads_work_keys_deletes_their_objects_then_calls_delete_mail_address`
- `a_hint_is_set_once_and_must_fit_the_providers_domains` (P21)

**Behaviour.** §4.1, D3, §4.12 step 2 and D17. Token encryption uses `_shared/crypto.ts`'s
`encryptString` with `MAIL_ENC_KEY`, and the HMAC uses `MAIL_ADDRESS_KEY`.

#### T4.2. Canary, batches, reconcile, state, provider (`functions/mail/`, `_shared/canary_send.ts`)

**Tests first** (spec test 6's route half, plus plan pins for §4.2, §4.11 and §6.3):
- `post_canary_refuses_a_hint_outside_the_providers_domains` (6)
- `a_fourth_canary_for_one_address_in_a_utc_day_is_refused` (6)
- `the_canary_goes_through_resend_with_its_nonce_in_a_header_and_the_subject` (P9, a fake Resend)
- `post_batches_opens_a_batch_of_its_phase_and_answers_its_nonce_once`
- `sent_closes_a_batch_with_the_devices_count`
- `reconcile_answers_only_held_fingerprints_in_the_window_and_the_key` (§4.11 step 3)
- `the_reconcile_request_carries_only_the_window` (nothing about unforwarded mail reaches the service)
- `state_reenabled_stamps_reenabled_at_and_elsewhere_sets_off`
- `state_confirming_opens_a_fifteen_minute_confirm_window` (P21)
- `provider_reads_mx_through_an_injected_resolver_as_m365_google_or_other` (§13 Q4)

**Behaviour.** The rest of spec §6.3's `mail` routes, with §4.10's limits.

#### T4.3. The periodic canary (`functions/mail-canary/`)

**Tests first** (spec test 6's cadence half, plus §4.10's states):
- `no_canary_to_a_mailbox_with_bound_mail_inside_24_hours_on_a_weekday_or_48_over_a_weekend`
- `no_canary_within_72_hours_of_that_mailboxs_last`
- `no_canary_on_a_day_the_student_had_a_knowlu_email` (P9)
- `a_mailbox_quiet_past_both_gets_one`
- `an_unarrived_setup_canary_sets_blocked_and_an_unarrived_periodic_one_sets_quiet`: a canary to a
  mailbox that was never `active` is a setup canary. Periodic canaries are judged after two hours.

**Behaviour.** §4.10 and D14. The run is hourly.

#### T4.4. The pull (`functions/mail-pull/`)

**Tests first** (spec test 12):
- `claims_come_in_ordering_date_then_uid_order`
- `a_message_dated_2099_orders_at_its_arrival`
- `a_batch_is_withheld_until_received_reaches_sent_or_a_timeout_then_delivered_whole`
- `live_claims_queued_during_an_open_batch_come_after_it`
- `ack_marks_delivered`
- `an_ack_outside_the_mail_uid_shape_is_dropped`
- `each_mailboxs_state_and_reenabled_at_ride_on_the_answer` (§4.10's *Shown*)

**Behaviour.** §4.8 step 1 and D10.

#### T4.5. Resolve (`functions/mail-resolve/`)

**Tests first** (spec test 13, plus the cap):
- `one_call_per_course_batch_stores_each_answer_on_its_claim`
- `a_stored_answer_is_replayed_on_a_re_pull_with_no_model_call`
- `an_id_outside_the_batch_reads_none`
- `a_resolve_call_is_charged_before_the_model`

**Behaviour.** D12, through the one pipeline, under the `mail_resolve` cap.

#### T4.6. Account deletion and export (`functions/account/`)

**Tests first** (spec test 14's purge and export half):
- `deletion_purges_each_mailbox_by_delete_mail_address_after_deleting_its_objects_by_key`
- `the_purge_list_names_all_seven_mail_tables`
- `the_export_carries_mailboxes_and_queued_claims_and_never_a_token_key_or_hash`

**Behaviour.** §4.13's server half and D17. The Google grant is still revoked first, as today.

### T4b. One mailbox, one path (`gmail-read`, D22)

**Agent:** `cloud-engineer` (Opus, high). **Tests first** (spec test 16c, plus P22):
- `a_gmail_address_with_a_forwarding_gets_a_non_quiet_answer_with_its_undelivered_items_and_no_gmail_call`
  (`read: 0`, `more: false`, `via_forward: true`)
- `a_gmail_address_without_a_forwarding_reads_exactly_as_today`

**Behaviour.** D22, read under P22: a forwarding counts while its `mail_addresses` row is neither
`revoked` nor `off`. It is never `quiet` (spec §1: the merged device reads any unknown quiet reason as
"re-connect from settings"). Calendar is untouched, and nothing on the device changes.

### Checkpoint B. Cloud review of T1–T4b

**Agent:** `reviewer` (Opus, high). **Report:** `docs/reports/<date>-email-forwarding-cloud-review.md`.
It reads the cloud diff against spec §4.6–§4.10, §5.1, §6.3, §7's retention rows, D2, D3, D8, D9,
D14, D17, D22, D23 and P5–P9, P21 and P22. It reads in particular:
- the SNS verification and the binding rules, with the PQ2 interop record;
- that no text lands in a table or a log;
- that every final path deletes its object;
- that charge and budget always run before the model;
- the per-route authentication and every `config.toml` section.

Fixes go back to `cloud-engineer`. The cloud sub-branch merges into the lane when no Critical or
Important finding is open.

### T14. Campus rows and the wizard's marker (`scaffold.rs`, `onboarding.rs`)

**Agent:** `mechanical` (Sonnet, low). **Why:** three struct fields and one marker, fully fixed here.
**After:** T19 is in the lane, because T14 touches T19's test files. **Files:** `app/src/scaffold.rs`,
`app/src/onboarding.rs`, `app/tests/scaffold.rs`, `app/tests/onboarding.rs`, and the four `Curated`
literals in `app/tests/{grades,scheduler}.rs`. Those literals gain the three fields and no assertion
changes. **Before it:** the disjointness check is repeated for `p3-registrar`, which adds fields to
`Curated` in `scaffold.rs`. T14's three fields go after every field `main`'s `Curated` has, and the
registrar (a draft Pilot PR, expected to merge later) rebases onto them (§10, R2). **UA's values:**
the main session supplies them from UA's public academic calendar and its
student mail domain, and Quinn confirms them (a queue item). Kentucky's row is `other`, with no
domain and no terms.
**Tests first:**
- `every_curated_row_names_a_known_mail_provider`
- `a_mail_domain_is_set_only_for_m365_or_google`
- `terms_are_ordered_non_overlapping_iso_date_pairs`
- `the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt`

**Behaviour.**
- `Curated` gains `mail_provider` (`m365`, `google` or `other`), `mail_domain` (an optional string)
  and `terms` (`&'static [(start, end)]`), per §4.1 and D7.
- `onboarding` gains `mail_offer_marker(profile_dir)`, set or cleared on the three finishing paths
  from the finish plan's `offer_mail` field. Like `offer_marker`, it is a file, never a `Settings`
  field, and `offer_mail` carries `#[serde(default)]` so existing callers are unchanged (D6, §13 Q9).

### T9. The mail window and its kept profiles (`app/src`), in two dispatches

**Agent, both:** `contract-engineer` (Opus, xhigh). **Why:** a signed-in mailbox, a kept profile and
its deletion by path. A path bug deletes the wrong folder, and a runner bug runs script on the wrong
origin (spec T9: no Opus-high implementer exists, and this is the case contract-engineer's
description names).

#### T9.1. The kept profiles and `mail.json` (`app/src/mail_session.rs`)

**Before it:** H3a. **Tests first** (spec test 32's session half, plus §5.4 pins):
- `is_session_dir_accepts_only_mail_session_ids_and_mail_setup_under_data_dir`
- `forget_deletes_only_that_mailboxs_profile`
- `a_failed_delete_is_recorded_and_retried_at_start_up`
- `the_start_up_sweep_removes_a_leftover_mail_setup`
- `forget_all_removes_every_mail_profile_and_nothing_else`
- `turned_off_only_grows`
- `mail_json_is_its_own_file_and_settings_json_is_untouched`

**Behaviour.** §5.4 and D15. It follows grades' `is_session_dir` rule, and the `grades.json` precedent
for `mail.json`.

#### T9.2. The window and the script runner (`app/src/mail.rs`)

**After:** S1–S3. **Before it:** H3b. **Tests first** (spec tests 33 and 34, plus the pure pieces):
- `the_mail_label_has_no_capability_grant` (33)
- `one_mail_window_at_a_time` (33)
- `the_runner_refuses_a_page_off_the_providers_origins` (33, a synthetic SSO origin)
- `the_runner_runs_only_compiled_in_scripts` (D19)
- `the_sign_in_detector_reads_m365_and_gmail_titles`
- `no_os_input_and_no_debugging_port_in_mail_sources` (34): `SendInput`, `keybd_event`, `mouse_event`,
  `SetCursorPos` and `remote-debugging` appear nowhere in `app/src/mail*.rs` or `app/assets/mail/**`.

**Behaviour.** D5, and §6.2's window pieces, on T0's recorded routes: the one label `mail`;
`open_visible`; hidden or off-screen as S2 decided; stepping aside; coming back centred and focused;
the popup per S1b; the protocol prompt per S3; the result channel per S1a, with a timeout.

### T10. Step machines and step scripts, in five dispatches

**Agent, all five:** `implementer` (Sonnet, high). **Why:** pure state machines, and DOM scripts tested
against synthetic pages (spec T10). **Rules for the scripts:**
- One readable file per provider and step under `app/assets/mail/<provider>/`, compiled in with
  `include_str!` (D19).
- Each script acts, reads its effect back, and returns a result through S1a's channel. A missing
  control returns the named `not found`, which triggers that step's guided fallback.
- Fixture pages are synthetic HTML modelled on the spikes' selectors (research note §6), with no real
  address or name in them.
- No script names a token, and no script logs.

#### T10.1. The step machines (`app/src/mail_steps.rs`)

**Before it:** H3c. **Tests first** (spec test 32's step half):
- `each_provider_has_its_step_list_in_order` (§4.2, §4.3)
- `a_failed_step_falls_back_to_guided_for_that_step_only_and_later_steps_continue` (§4.4)
- `an_existing_forwarding_address_is_a_needs_you_and_never_replaced`
- `the_term_start_default_is_the_latest_of_5_jan_15_may_and_15_aug_before_today` (D7)
- `a_curated_term_covering_today_wins_over_the_default` (D7)
- `the_pace_is_at_most_10_a_minute_and_600_a_mailbox_a_day` (provisional until S6)
- `the_first_send_failure_stops_the_routine_with_its_named_outcome`
- `nothing_is_retried_in_a_loop`
- `stop_after_the_current_step_says_what_was_done` (§4.5)

**Behaviour.** §4.2–§4.5's steps, states and outcomes, as pure data and transitions, with no window.
S5's third choice is added here only if S5 found it.

#### T10.2a. Microsoft 365: forwarding scripts

**Files:** `app/assets/mail/m365/` (sign-in probe, forwarding read, set and off), `deno.json` and
`test_dom.ts` (P14). **Tests first** (spec test 35, the M365 forwarding half):
- `m365_forwarding_set_turns_it_on_with_keep_copy_and_reads_it_back`
- `m365_the_switch_is_read_by_checked_not_aria_checked`
- `m365_forwarding_elsewhere_is_reported_not_replaced`
- `m365_a_missing_control_returns_not_found`
- `m365_forwarding_off_turns_it_off_and_reads_it_back`

#### T10.2b. Microsoft 365: mailbox scripts

**Tests first** (spec test 35, the M365 mailbox half, §4.2 step 5, §4.11):
- `m365_search_since_a_date_excludes_junk_and_deleted_and_counts_by_aria_setsize`
- `m365_the_list_reads_sender_subject_and_received_time_in_s2s_form`
- `m365_forward_as_attachment_sends_one_message_with_the_nonce_in_the_subject`
- `m365_a_non_delivery_report_or_send_failure_in_the_page_is_reported`
- `m365_sent_cleanup_deletes_only_copies_with_the_nonce_and_the_knowlu_recipient`
- `m365_sent_cleanup_keeps_a_nonce_copy_to_another_recipient_and_a_knowlu_copy_without_the_nonce`
  (§10, R4): both survive, and so does every other item on the synthetic Sent page.

#### T10.3a. Gmail: forwarding scripts

**Tests first** (spec test 35, the Gmail forwarding half, §4.3):
- `gmail_add_address_clicks_input_buttons_by_value`
- `gmail_a_section_that_did_not_draw_is_redrawn_by_a_hash_round_trip`
- `gmail_forwarding_on_keeps_gmails_copy_in_the_inbox_and_reads_it_back`
- `gmail_forwarding_elsewhere_is_reported_not_replaced`
- `gmail_a_missing_control_returns_not_found`
- `gmail_remove_address_removes_it` (§4.12 step 1)
- `gmail_remove_address_leaves_every_other_forwarding_address` (§10, R4)

#### T10.3b. Gmail: mailbox scripts

**Tests first** (spec test 35, the Gmail mailbox half, §4.3 step 7):
- `gmail_search_uses_after_date_minus_spam_minus_trash`
- `gmail_pages_are_selected_oldest_first`
- `gmail_forward_as_attachment_keeps_each_send_under_25_mb_with_the_nonce`
- `gmail_a_send_failure_is_reported`
- `gmail_sent_cleanup_deletes_only_its_own_copies`
- `gmail_sent_cleanup_keeps_a_nonce_copy_to_another_recipient_and_a_knowlu_copy_without_the_nonce`
  (§10, R4)
- `gmail_the_list_reads_sender_subject_and_received_time_in_s2s_form`

### T11. Client, recover, the setup runner, the commands, the slot, in five dispatches

**Agent, all five:** `implementer` (Sonnet, high). **Why:** specified orchestration off the list,
checked by tests through seams (fake window, fake scripts, a loopback service, a fake clock). The app
never writes the vault here: the vault changes only through the engine's `judge`.

#### T11.1. The service client (`app/src/mail_client.rs`)

**Before it:** H3d. **Tests first** (plan tests over loopback, P15):
- `every_route_sends_the_session_bearer_and_maps_failures_to_named_outcomes`
- `a_402_on_an_entitled_route_reads_lapsed_while_get_and_delete_still_work` (16b, device half)
- `no_token_or_address_reaches_an_error_string`
- `a_timeout_is_a_named_outcome`

#### T11.2. Reconcile and recover (`app/src/mail.rs`)

**Tests first** (spec §4.11 steps 3–4 and test 32's pace half, as plan tests):
- `a_listed_message_is_held_when_its_hmac_matches_within_fifteen_minutes`
- `nothing_about_an_unheld_message_is_sent_to_the_service`
- `recover_forwards_oldest_first_and_at_most_30_a_slot_when_no_backfill_is_open`
- `an_open_backfill_ignores_the_slot_cap_and_keeps_microsofts_pace`
- `the_window_is_three_days_or_back_to_lapsed_since_at_most_30_or_to_when_last_known_on`
- `the_batch_closes_with_the_count_sent_and_the_sent_copies_are_cleaned`

#### T11.3. The setup runner (`app/src/mail.rs`)

**Tests first** (§4.1–§4.5, D6, D7, D15 and D16, as plan tests):
- `one_setup_at_a_time`
- `a_closed_sign_in_window_is_setup_cancelled_and_nothing_changed`
- `stop_ends_after_the_current_step_and_names_what_was_done`
- `a_canary_not_back_in_three_minutes_leaves_blocked_with_retry_and_offers_what_remains` (D16)
- `an_uncurated_school_asks_once_for_the_address_and_the_service_decides_the_provider` (§4.1)
- `a_term_date_with_no_curated_term_is_asked_before_any_mail_is_read` (D7)
- `an_open_backfill_resumes_at_launch`, and resumes again as its pace allows
- `a_closed_backfill_batch_starts_a_slot_now` (§4.8 step 1, `scheduler::run_slot`)
- `with_the_saved_sign_in_off_setup_uses_mail_setup_and_wipes_it_on_close`

**Behaviour.** The runner is on a background thread. It drives T10.1's machine through T9.2's window,
T10's scripts and T11.1's client, writes the progress rows, and takes answers. A batch that finishes
an interrupted backfill is opened as phase `backfill`. **After it:** H4.

### T12. Deletion and sign-out (`account.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** `account.rs` is on the contract list (session
JWT). **Files:** `app/src/account.rs`, `app/tests/account.rs`, keeping that file's `CREDMAN_LOCK` and
`Drop`-guard conventions.
**Tests first** (spec test 38):
- `delete_my_data_runs_the_forwarding_off_pre_step_before_the_server_call`
- `a_failing_pre_step_never_blocks_the_deletion`
- `the_mail_windows_close_before_delete_local_data`
- `sign_out_deletes_this_profiles_mail_session_and_mail_setup_profiles_and_no_others`

**Behaviour.** §4.13. The pre-step is bounded to a minute per mailbox and calls into `mail.rs`.
`PRIVACY_VERSION` is untouched (D18). The confirm sentence is T15.2's.

### Checkpoint C. Contract review of T9.1, T9.2, T12 and the mail card writer

**Agent:** `contract-reviewer` (Opus, xhigh). **Report:**
`docs/reports/<date>-email-forwarding-contract-review-c.md`. It reads:
- the profile paths and their guard, `turned_off`'s only-grows rule, and the runner's origin check;
- that no OS input, CDP or debugging port appears anywhere;
- `account.rs` against §4.13;
- T6.2's card bytes against `approvals::validate_amendment` (P13);
- T6.2's direct-set writer: the human-set re-check, `mail_asof` built only by `write::to_literal`,
  and the body line; and T6.3's seen-ledger write (§10, R10, R11, R13);
- the scripts that delete or change something in a student's mailbox, with their synthetic pages:
  M365 and Gmail Sent cleanup, Gmail's remove-address and both forwarding-off scripts. Each must
  select by the nonce **and** the Knowlu recipient, or by the Knowlu address alone, and nothing wider
  (§10, R4).

T11.4 starts when no Critical or Important finding is open.

### T11.4. The ten commands (`app/src/mail.rs`)

**Tests first** (spec test 37, and 36's Disconnect half, plus plan pins):
- `the_ten_mail_commands_are_in_the_console_list_and_none_in_the_wizards` (37)
- `mail_disconnect_writes_turned_off_before_anything_else` (§4.12 step 0, §13 Q11)
- `mail_keep_session_off_deletes_every_mail_profile`
- `mail_applied_passes_the_engines_json_through_unchanged`
- `mail_status_reports_offer_pending_until_setup_starts_or_not_now` (D6, T14's marker)

**Behaviour.** Spec §6.2's table: thin wrappers over T11.1–T11.3 and T9.1. `mail_applied` runs
`knowlu-engine mail-applied` and computes nothing. **Commit and H5 (§10, R5).** The implementer
commits the commands and every test but 37, with the gate green, and leaves test 37 uncommitted. The
main session then applies H5, runs the gate, and commits test 37 with H5, so no commit is red.

### T11.5. The `mail` step in the slot (`scheduler.rs`)

**After:** T19, which shares `app/tests/scheduler.rs`. **Tests first** (spec test 36, in
`app/tests/scheduler.rs`):
- `the_mail_step_runs_after_the_grades_capture_and_before_sync`
- `slot_argv_is_unchanged`
- `the_mail_row_and_each_named_skip_exit_zero`: saved sign-in off, not due, and no entitlement.
- `reconcile_follows_reconcile_cadence_in_both_modes` (§13 Q3, §10 R7): with `Daily`, at most once a
  day per mailbox; with `RepairOnly`, only for a mailbox whose state is not `active`. The shipped
  value is `Daily` until S4's revisit, and the revisit changes only the value.
- `no_mailbox_records_no_mail_row_and_no_mail_step_name_reaches_the_runner_log` (§10, R17)
- `no_file_or_log_of_knowlus_own_holds_the_address_or_token` (§5.4, §10 R17): a setup and a slot run
  over a fake service whose address and token are unique markers. A byte search then finds neither in
  `mail.json`, any other file Knowlu writes in the data folder (the WebView2 profiles are the
  provider's and are excluded), the `RunSummary` or `state/runner-log.md`. The progress view may show
  the address (§4.1 step 2), but only from the in-memory answer.
- `a_forwarding_found_off_is_turned_back_on_and_reported_reenabled` (§13 Q11)
- `a_turned_off_mailbox_is_never_turned_back_on_even_after_a_failed_disconnect`
- `a_forwarding_pointing_elsewhere_is_left_unchanged_and_asked_about`
- `a_signed_out_profile_records_needs_sign_in_and_shows_no_window`
- `the_grades_capture_and_the_mail_step_never_run_at_once`

**Behaviour.** §4.11 and P18. A `MailSeam` sits beside `GradesSeam`, and `slot_argv` and the engine
chain are unchanged. With no mailbox connected the step adds no row. With one, its named skips go
into the `RunSummary` the Settings page reads, and never into the scheduler's skip filter for
`state/runner-log.md`. A mail step therefore writes nothing in the vault, as A12's sentence says
(§10, R17).

### T15. The page (`app/static`), in five dispatches

**Agent, all five:** `console-ui` (Sonnet, medium). **Why:** `app/static` and the walk scripts are
its lane. **Files:** `app/static/{index.html,console.js,console.css}`, `app/tests/static_assets.rs`,
and in T15.3 and T15.5 `scripts/{wizard,settings}-check.py`. Each dispatch is its own commit, and the
console's existing static tests stay green.
**Before T15.1:** M2 is in `main` (#27), and T15 builds on its page and its `settings-check.py`. The
main session merges `origin/main` into `email-app` first if it has moved, and re-runs the
disjointness check against `p3-registrar`, which edits `scripts/wizard-check.py` (T15.3; §10, R1 and
R2).
**Rules for all five:**
- Every mail-derived string reaches the DOM only through `h()`.
- No `ui_event` call passes a mail-derived value or a variable holding one (VISION as amended by A12).
- The copy keeps the spec's fixed sentences word for word.

#### T15.1. The progress view (§4.5)

**Tests first:**
- `the_progress_view_shows_one_row_per_step_with_its_state`
- `needs_you_names_its_reason_and_offers_its_answers`
- `the_term_date_needs_you_shows_the_date_to_confirm_before_any_mail_is_read` (D7)
- `leave_closes_the_panel_and_settings_carries_the_progress`
- `stop_invokes_mail_setup_stop_and_shows_what_was_done`
- `backfill_shows_n_of_m_and_a_time_left`
- `the_consoles_first_open_starts_school_mail_setup_when_offer_pending` (D6, §13 Q9)

#### T15.2. Settings → Email (§4.12, §4.13, D15)

**Tests first:**
- `settings_email_lists_each_mailbox_with_its_state_and_its_address_with_copy`
- `resume_rotate_and_the_saved_sign_in_switch_invoke_their_commands`
- `turn_off_email_is_two_steps_with_the_confirm_text`: §4.12's sentence, word for word.
- `delete_my_data_confirm_adds_the_forwarding_sentence`: §4.13's sentence, word for word.
- `a_failed_profile_delete_is_named_until_it_succeeds`
- `the_google_row_and_the_email_row_sit_side_by_side` (§9)

#### T15.3. The wizard's announcement (D6, risk 12, §9)

**Tests first:**
- `the_panel_after_sign_in_reads_next_connect_your_school_email`
- `not_now_carries_the_school_rules_sentence`: "If your school's rules don't allow forwarding your
  school email, skip this."
- `finish_sends_offer_mail`
- `the_gmail_panel_button_reads_connect_gmail_directly_google_testers_only`

`scripts/wizard-check.py` walks the panel with a stubbed `invoke`.

#### T15.4. "From your email" (§4.14, D10)

**Tests first:**
- `from_your_email_lists_by_course_items_created_fields_changed_past_archived_and_notices`
- `undo_invokes_delete_note_set_fields_with_the_old_value_or_close_info`
- `a_changed_since_row_shows_no_undo`
- `from_your_email_opens_from_the_last_progress_row_and_from_settings`

#### T15.5. The deck, untrusted text and telemetry (D21, §5.2, §12.4)

**Tests first:**
- `a_mail_card_shows_its_sender_date_and_quote`
- `mail_derived_text_renders_inert`: `<img src=x onerror=…>` in `from`, `why`, `evidence` and the
  title, on a card, a notice and a "From your email" row.
- `no_mail_field_reaches_the_dom_without_h` (a static check)
- `no_ui_event_carries_mail_text`

`scripts/settings-check.py` walks Settings → Email, the progress view and "From your email" with a
stubbed `invoke`. It checks that the stub recorded no `ui_event` argument carrying the walk's mail
text or the walk's Knowlu address (§5.4, §10 R17).

### T16. The per-course digest (optional; PQ3)

**Agent:** `cloud-engineer` (Opus, high), and only if Quinn keeps it at Checkpoint D. **Files:**
`functions/mail-digest/**`, its `config.toml` section, its `models` pin and its cap. Behind
`mail_digest: false` (D13). The device side of the digest is not designed in the spec; if T16 is
kept, that side is a follow-on brief, not part of this lane.

### T13b. Reference docs (`docs-keeper`)

**Files and claims**, each citing a function and never a line:
- **`docs/reference/app.md`:** the ten commands, `mail.json`, the `mail-session-<address_id>` and
  `mail-setup` profiles, the `mail` window label, and the `mail` step before the chain.
- **`docs/reference/engine-commands.md`:** `mail-applied`, `judge`'s mail pass and its run line, and
  `ingest`'s ruling-3 card.
- **`cloud/supabase/README.md`:** the AWS runbook (§5.1), with the secrets by name only.
- **The legal note:** its two stale Crimson lines.
- **`docs/notes/2026-09-29-vision-program.md`:** the email row as an MVP item.
- **HANDOFF:**
  - production parity's migration, functions, secrets and production's own receiving domain;
  - in the Launch row, the two-desktop hand-off: the lease covers the mail pass and the `mail` step
    (spec §9).

### T17. Integration (`integrator`)

1. Trial-merge each sub-branch into the lane (`git merge-tree --write-tree`) and list conflicts by
   file. Appended `config.toml` sections are resolved here. A contract-list or migration hunk goes to
   its engineer, with both sides quoted. So does every conflict in a file §10 R2's table lists, to the
   agent that table names: never a cheaper agent for a cloud judge file, `migrations_test.ts`,
   `approvals.rs`, `ingest.rs` or `enrich.rs`.
2. Merge `main` into the lane. Check the migration stamp still sorts last; if not, re-stamp it, since
   nothing is applied yet.
3. Recount both `generate_handler!` lists by script: ten new console names beside M2's, which are
   already there (#27), and none in the wizard's.
4. Confirm H0–H5 are in. H0, CLAUDE.md's two A12 lines, lands now (§7).
5. Add one step to `ci.yml`'s `cloud` job: `deno test --allow-read --config app/assets/mail/deno.json
   app/assets/mail/`, with `deno check` and `deno lint` over the same files. No new action is added,
   so `engine/tests/workflows.rs` stays green.
6. Refuse the merge if T10.1's pace constants differ from S6's record, or S6 has not run.
7. Run the full gate:
   - the workspace build and tests with 0 other warnings;
   - `scripts/ci/eol-check.ps1`;
   - Deno;
   - §10 R12's contract-list diff, after `git fetch`, showing only `approvals.rs` and `account.rs`,
     and the fixtures diff empty;
   - the four ignores in place;
   - `human_actor_literal.rs` green.

### Checkpoint D. Whole-branch review

**Agent:** `reviewer` (Opus, high), with `contract-reviewer` re-reading any contract-list file changed
after A or C. **Report:** `docs/reports/<date>-email-forwarding-whole-branch-review.md`. It uses §9's
ledger as the checklist, confirms that A, B and C are closed, and checks:
- `site/` and `PRIVACY_VERSION` are untouched;
- §10 R12's contract-list diff, `git diff --stat origin/main...HEAD` after `git fetch`, shows only
  `approvals.rs` and `account.rs`, and nothing in the oracle, sync and entitlement tests or the
  fixtures;
- no mail path writes `commitments/`;
- the Gmail path changed only by D22;
- no mail text reaches telemetry or a log;
- no OS input or debugging port appears;
- no new `"quinn"` or `"student"` literal was added;
- a grep finds the retired working title nowhere in the branch.

**Quinn** reads the report before T18 (spec §14) and answers PQ3.

### T18. Deploy, pin check, live proof, merge (main session, with Quinn)

**Why the main session:** staging pushes, secrets, the OTP session, AWS and the sign-ins are the
controller's (spec T18; HANDOFF §2's controller rules).
- **T18a. Staging.**
  - Quinn sets the IAM key pair as staging secrets from their own shell. The main session generates
    `MAIL_ADDRESS_KEY` and `MAIL_ENC_KEY` blind, and sets `MAIL_DOMAIN`, `MAIL_BUCKET`,
    `MAIL_SNS_TOPIC_ARN` and `AWS_REGION`.
  - `db push` the migration, with `--include-all` only if an older stamp is pending.
  - Deploy the functions with `--use-api`, and add the cron's Vault token.
  - Point SNS's HTTPS subscription at `mail-inbound` and confirm it.
  - Smoke by OTP session (at most two OTPs an hour): a canary round trip, one synthetic forward, and
    the controller's count-only query.
- **T18b. The pin check** (spec test 17). About twenty synthetic messages go through `mail_claim` and
  `mail_resolve` on staging, and their answers are recorded in
  `docs/reports/<date>-email-forwarding-pin-check.md`. Each pin is then confirmed, or re-pinned by a
  one-statement migration from `cloud-engineer` (P8).
- **T18c. The live proof**, spec §12.6 steps 1–8 as written. It runs on a dev build of the lane, with a
  scratch profile and founder-owned mailboxes, and Quinn at the machine for each sign-in and phone
  tap. Until S6 confirms the pace, the school backfill stays under 600. Afterwards, clean up per the
  standing rule and restore the school mailbox's forwarding.
- **T18d. The PR.** CI green and Quinn's word to merge; the merge carries H0. No release is tagged
  before privacy bump #1 (D18). The controller's HANDOFF milestone update follows.

## 6. Order, parallelism and checkpoints

**Per worktree.** One implementer at a time in each; the four run in parallel (P3).

| Worktree | Order |
|---|---|
| `main` (T19's branch) | T19 → T13a → R19 → PR merged on Quinn's word → `main` merged into the lane and into `email-app` |
| `email-engine` | H1a → T5.1 → T5.2 → T5.3 → T5.4 → T5.5 → (PQ5) T8 → T8b → Checkpoint A reads while T6.1 runs → T6.1 committed → A's fix loop → H1b → T6.2 → T6.3 → T6.4 → H1c → T7 → H2 |
| `email-cloud` | S0 → T1.1 → T1.2 → T1.3 → T2a → T2b → T2c → T2d (PQ2) → interop check → T2e → T2f → T2g.1 → T2g.2 → T3a (PQ4) → T3b → T3c → T3d → T3e → T4.1 → T4.2 → T4.3 → T4.4 → T4.5 → T4.6 → T4b → Checkpoint B → T16 (only if PQ3 keeps it) |
| `email-app` | T19 merged → T14 → H3a → T9.1 → H3c → T10.1 → S1–S3 → H3b → T9.2 → T10.2a → T10.2b → T10.3a → T10.3b → H3d → T11.1 → T11.2 → T11.3 → H4 → T12 → Checkpoint C → `email-engine` merged in (for `mail-applied`) → T11.4 → H5 with test 37 → T11.5 → `origin/main` merged in and the registrar check → T15.1 → … → T15.5 |
| lane | T0.0 → … → T13b (after T15.5 and B) → T17 → Checkpoint D → T18 |

**Cross-worktree dependencies.**
- T11.4's `mail_applied` test runs the built engine, so `email-engine`, through T7 and H2, is merged
  into `email-app` before T11.4.
- T6's tests fake the service at the loopback and wait on no cloud task.
- T18 needs every sub-branch merged (T17).

**Gates** (P4).
- S0 blocks all of `email-cloud`.
- S1–S3 block T9.2 onward in `email-app`.
- S6 blocks T17's merge of the pace numbers, not the code.
- S4 changes one constant whenever it lands.

**Waits on Quinn, one at a time, each with its context:**
- PQ1 (T0.0);
- PQ5 (T0.0, before T8 and T8b);
- the AWS account, region, DNS and DMARC sender (S0);
- the sign-ins at the machine (S1–S3);
- PQ2 (before T2d) and PQ4 (before T3a and T5.5);
- the M365 test tenant (S6);
- UA's values (T14);
- T19's merge word;
- the §13 Q3 revisit after S4;
- PQ3 at Checkpoint D;
- the staging secrets, the sign-ins and the merge word at T18.

**Opus starts:** T0, T5.1–T5.5, T8, T8b, A, T1–T4b, B, T9.1, T9.2, T12, C, T16, T17, D, T18 and
R19. Every other task starts on Sonnet, and a second failure moves it to Opus at high.

**Checkpoints for Quinn** (spec §14):
- **Checkpoint 0:** the T0 report, with S0's DMARC result and header shapes, and S1–S3; S6 and S4 are
  added when they land.
- **R19:** T19's diff and review, before its PR merges.
- **Checkpoint A:** the contract review of T5, T8 and T8b, with the diffs.
- **Checkpoint D:** the whole-branch review, before T18, with PQ3.
- **The merge word** at T18d.
- **Outside this lane:** A13's review trigger at the 20th paying account. T13a puts it in Quinn's
  queue.

Checkpoints B and C are internal: Quinn sees their reports through Checkpoint D.

## 7. Controller hand-offs (main session)

`engine/src/{lib,main,cli}.rs`, `app/src/{lib,main}.rs` and `CLAUDE.md` are single-owner, so the main
session edits them, each when the next task needs it (the M2 plan's P3 rule). `engine/src/ingest.rs`
is single-owner too, but the signed spec gives it to T8b, which this plan follows (spec §8, §14).
- **H0 (T17).** `CLAUDE.md`, A12's text word for word. The slot line keeps its chain and gains
  "Before the chain, the app runs its own window steps, the grades capture and then the
  saved-sign-in `mail` step; neither writes the vault." The actor list gains "forwarded mail as
  `agent:knowlu.mail`".
- **H1a (before T5.1).** `engine/src/lib.rs` gains `pub mod mailreconcile;`, with a
  `mailreconcile/mod.rs` that holds only its module doc, so the gate stays green with no unused code.
- **H1b (before T6.2).** The same for `pub mod mail;`.
- **H1c (before T7).** The same for `pub mod mailapplied;`.
- **H2 (after T7).** The engine's dispatch gains `mail-applied --vault <v> [--since <ts>]`. It sits
  beside `surface`, outside the entitlement gate's list. Check: on a scratch vault the command prints
  JSON and exits 0.
- **H3a–H3d.** `app/src/lib.rs` gains `pub mod mail_session;` (before T9.1), `pub mod mail_steps;`
  (before T10.1), `pub mod mail;` (before T9.2) and `pub mod mail_client;` (before T11.1), each with a
  doc-only file.
- **H4 (after T11.3).** `app/src/main.rs`'s `setup` calls `mail::on_start` with the app handle. It runs
  the start-up sweep, retries pending profile deletes, and resumes an open backfill.
- **H5 (after T11.4).** The ten command names go into the console's `generate_handler!` list in
  `app/src/main.rs`, which already holds M2's commands (#27), and none into the wizard's. The list is
  recounted by script; a number is quoted only after the count. The main session commits H5 together
  with T11.4's test 37 (§10, R5).
- **H6 (T0.0).** The signed documents land on `main` as a docs-only commit (P2).
- **H7 (T0.S0, T18a).** AWS, DNS, the SNS subscription, secrets set from Quinn's own shell, the
  migration push and the function deploys.
- **H8 (T13a, T13b).** HANDOFF pushes. They are batched at milestones, because every push to `main`
  runs CI.

## 8. Risks

- **The lane's size moves the MVP exit** (XL). Guarded by four worktrees in parallel, by T19 going
  first so the grades proof does not wait, and by PQ3's cut.
- **S0 fails.** If SES drops DMARC failures, or SNS proves awkward, D2's fallback (a small Lambda that
  POSTs with a shared secret) replaces only T2a's seam. T2b–T4b are unchanged.
- **S1–S3 fail.** A hidden window that stops rendering, a blocked popup, or no result channel makes
  that step guided (spec risk 4). T9.2 and T10 are re-briefed at Checkpoint 0, before they start.
- **The verifier is wrong in the unsafe direction (PQ2).** If binding fails, every live forward is
  dropped as unbound, while canaries still arrive, because they are recognised before binding. That
  is silent loss. Guarded by:
  - the interop check over S0's real captures;
  - the `unbound` counter in Settings and the run line;
  - Checkpoint B;
  - live-proof step 5, which needs a live mail to become a card.
- **A wrong silent change in the backfill** (spec risk 1). Guarded by T5 going to contract-engineer,
  by Checkpoint A, by tests 20–24 and 22b, by the evidence check (T3b), and by Undo (T15.4).
- **Microsoft restricts the student's school account** (spec risk 15). Guarded by S6 on a test tenant,
  T17's refusal of unconfirmed pace numbers, and a live-proof backfill under 600.
- **Shared files with events, the registrar and two desktops** (M2 is merged). Guarded by §10 R2's
  table, which names who rebases each file, by the disjointness check before T1.2, T3b, T6.4, T8, T8b,
  T14 and T15.1, and by T17's routing of each conflict to its engineer.
- **Ripples from a new type or field.** T14's `Curated` fields touch T19's tests, so T14 runs after
  T19. Widening `Kind` would touch every typed record, so it is not widened (P6). Every new function
  needs its own config section (P5).
- **A cheaper agent edits a contract-list file.** The rules forbid it. §10 R12's three-dot diff over
  the whole contract list, run at every gate and at Checkpoint D, catches it.
- **A step script deletes the wrong mail.** Guarded by the negative cleanup tests in T10.2b and T10.3b
  and by Checkpoint C's contract read of every destructive script (§10, R4).
- **Real mail or names in the repo** (rule 1). Guarded by T0's placeholder rule, synthetic fixtures,
  and Checkpoint D's grep.
- **A release cut too early.** T13a's guard reaches `main` with T19 (D18).
- **The live proof touches the founder's own school mailbox.** Only copies carrying the nonce are
  deleted, the forwarding is restored afterwards, and S6 runs on a test tenant (§13 Q8).
- **Staging spend.** Bounded by the caps, the allowance and the per-account ceiling. OpenRouter stays
  funded (Quinn's standing Pilot item).
- **The rename (PQ1) disturbing the controller's state.** It is done at T0.0, before any sub-branch
  exists.

## 9. Fidelity ledger

Each row is a requirement of the signed spec, the task that meets it, and what proves it. Test
numbers are spec §12's; named tests are this plan's (§5).

### 9.1 Decisions, amendments and Quinn's answers

| Spec requirement | Task | What proves it |
|---|---|---|
| D1: all of email in the MVP; Gmail OAuth changed only by D22 | every task; T4b | the proof (T18c); Checkpoint D's check of the Gmail path |
| D2: SES on `in.knowlu.com`; an S3 action with `TopicArn` only; one-day expiry; SNS verified; nothing rejected at the door | T0.S0, T2a, T2b, T2g.1, T13b, T18a | S0's DMARC result; tests 1, 3 and 16; the runbook |
| D3: one secret address per mailbox; HMAC lookup; encrypted token; Rotate's grace; three live addresses and six mints in 30 days | T1.1, T4.1, T2g.1 | `a_minted_token_is_26_…`, `rotate_keeps_the_old_token_…`, `a_retired_token_inside_its_rotate_grace_still_binds`; test 11's address half |
| D4: forward everything, and keep the copy | T10.2a, T10.3a | `m365_forwarding_set_turns_it_on_with_keep_copy_…`, `gmail_forwarding_on_keeps_gmails_copy_…` |
| D5: automatic, DOM-only setup; window visible only to sign in; an origin list; no CDP or OS input; guided fallback per step | T0.S1–S3, T9.2, T10.1–T10.3b, T11.3 | tests 33, 34 and 35; `a_failed_step_falls_back_to_guided_…`; Checkpoint C |
| D6, §13 Q9: announced in the wizard, run at the console's first open, offered in Settings any time | T14, T11.4, T15.1–T15.3 | `the_mail_offer_marker_is_set_…`, `mail_status_reports_offer_pending_…`, `the_consoles_first_open_starts_…`, `the_panel_after_sign_in_reads_…`; proof step 1 |
| D7: backfill to the term's start; curated terms, else a default the student confirms; no age cutoff | T14, T10.1, T11.3, T15.1, T5.3 | `terms_are_ordered_…`, `the_term_start_default_…`, `a_curated_term_covering_today_wins_…`, `a_term_date_with_no_curated_term_is_asked_…`; test 23 |
| D8: two-stage intake and a separate queue; `seen` written with work after the put; an expired item loses `seen` | T1.1, T2g.1, T3d, T3e, T4.4 | tests 7 and 16; `an_expired_item_loses_its_seen_row_…` |
| D9: claims cheapest first; templates; a small model with a fixed schema; verbatim evidence; the 0.6 floor | T3a–T3e | tests 8, 9 and 10 |
| D10: deterministic reconcile in `judge`; the ordering date; batches whole; backfill applies and lists, live proposes; Undo | T4.4, T5.1–T5.5, T6.3, T7, T15.4 | tests 12, 18, 19 and 29; `undo_invokes_…`; proof step 3 |
| D11 (1)–(3): card or direct decided from `human_set`; the mail card writer after `find_pending_amendment` | T5.4, T5.5, T6.2 | tests 20, 21 and 22; `a_mail_amend_card_passes_validate_amendment_…`; `a_human_set_landing_between_plan_and_apply_…` (§10, R10); Checkpoints A and C |
| D11 (4): an approved mail amend writes as the human actor; a bad actor file stops it | T8 | test 31; Checkpoint A |
| D11 (5), §13 Q7: `ingest` honours ruling 3 for `title` and `due` | T8b; T8 under PQ5 | test 31b; PQ5's tests; `a_hand_set_title_and_due_both_contradicted_file_two_one_field_cards`; Checkpoint A; proof step 5 |
| D12: ambiguity resolved per course; stored answers; "none" is a notice | T3b, T3c, T4.5, T5.2, T6.4 | tests 13 and 25; `a_stored_resolution_wins_…` |
| D13: course information as notices, superseding by key; the digest optional | T5.5, T6.2, T16 | test 26; PQ3's answer at Checkpoint D |
| D14: setup and periodic canaries; the seven states; named lines and heads-up items | T2e, T2g.2, T4.2, T4.3, T6.3, T11.3, T11.5, T15.2 | tests 6 and 36; `a_non_active_mailbox_keeps_one_heads_up_…`; proof step 6 |
| D15, §13 Q6: one saved session per mailbox, on by default; reconcile, recover, re-enable; every deletion path; a retried delete | T9.1, T11.2, T11.4, T11.5, T12 | tests 32, 36 and 38; `mail_keep_session_off_deletes_…`; proof steps 6–7 |
| D16, Q7: no contact with school IT; a blocking tenant is named | T11.3, T15.1 | `a_canary_not_back_in_three_minutes_leaves_blocked_…` |
| D17: Disconnect and deletion purge everything that serves the connection; the export | T1.3, T4.1, T4.6, T11.4, T12 | test 14's purge and export half; `delete_reads_work_keys_…`; proof step 7 |
| D18: no `PRIVACY_VERSION` or `site/` change; no release before bump #1 | T13a, D | the guard line on `main` with T19; Checkpoint D |
| D19: the step scripts compiled in | T9.2, T10.2a–T10.3b | `the_runner_runs_only_compiled_in_scripts` |
| D20: the actor, `created_by: mail`, `mail:` uids, `ingest-seen` | T2c, T5.1, T6.2, T6.3 | `seen_records_mail_uids_…`, `a_message_without_a_message_id_keys_…`; Checkpoint A |
| D21, §13 Q10: sender and date on mail cards, notices and applied changes | T3e, T6.2, T15.5 | `…_carries_from_mail_date_and_evidence`, `a_mail_card_shows_its_sender_date_and_quote` |
| D22: one mailbox, one path; never `quiet` | T4b | test 16c; P22 |
| D23, §13 Q1: an allowance of 2,000 judgments and $1.00 per account per term | T1.1, T1.3, T3e | test 11 |
| D24: events and schedules stay the student's; nothing written to `commitments/` | T5.3, T5.5 | test 23; `no_action_ever_targets_commitments` |
| D25, §13 Q6, A13: the policy-read gate suspended, as one constant | T19, T13a, R19 | tests 39–41; R19's report; proof step 1's grades half |
| A1–A4 and A6–A11: the signed text in the cloud design, the Gmail spec and VISION | T0.0 (H6) | T0.0's marker check; the docs commit on `main` |
| A5: ruling 10's MVP list and exit gain email and its proof; ruling 9 | T0.0, T13a, T18c | HANDOFF's MVP lane; the proof |
| A12: no forwarded-mail content in telemetry; CLAUDE.md's slot and actor lines | T15.5, T17 (H0) | `no_ui_event_carries_mail_text`; H0's diff |
| A13: ruling 12's five passages, the markers and the review trigger | T0.0, T19, T13a | tests 39–41; HANDOFF's queue line |
| §10's corrections that need no signature (the legal note's Crimson lines, HANDOFF) | T13a, T13b | the edited lines |
| §13 Q2: a canary only after quiet, at least 72 hours apart, never on a day with a Knowlu email | T4.3 | test 6's cadence half; P9 |
| §13 Q3: reconcile once a day, revisited after the 10-08 spike | T11.5, T0.S4 | `reconcile_follows_reconcile_cadence_in_both_modes`; S4's record |
| §13 Q4: the provider from the curated row, else MX | T14, T4.2, T11.3 | `provider_reads_mx_…`, `an_uncurated_school_asks_once_…` |
| §13 Q5: `in.knowlu.com` | T0.S0, T18a | S0's MX; `MAIL_DOMAIN` |
| §13 Q8: S6 on an M365 test tenant, never a real school account | T0.S6, T17 | S6's record; T17 step 6 |
| §13 Q11: re-enable automatically and say so; never a mailbox turned off in Knowlu's Settings | T9.1, T11.4, T11.5, T6.3 | `turned_off_only_grows`, `mail_disconnect_writes_turned_off_before_anything_else`; test 36; proof steps 6–7 |

### 9.2 Flows, data, surfaces, privacy, contracts and lanes

| Spec requirement | Task | What proves it |
|---|---|---|
| §4.1: which mailbox, the address, the binding hint | T14, T4.1, T4.2, T11.3 | P21's tests; `an_uncurated_school_asks_once_…` |
| §4.2: school Microsoft 365, step by step, with the pace and its stop | T10.1, T10.2a, T10.2b, T11.2, T11.3 | test 35 (M365); T10.1's tests; proof step 2 |
| §4.3: personal Gmail, step by step, with the server-side confirm | T10.3a, T10.3b, T2e, T2g.2, T4.2 | tests 5 and 35 (Gmail); S1b; proof step 4 |
| §4.4–§4.5: guided steps and the progress view | T10.1, T11.3, T15.1 | T10.1's and T15.1's tests |
| §4.6: receiver intake, steps 1–8 | T2a–T2g.2 | tests 1–7, 15 and 16 |
| §4.7: claims extraction, steps 1–6 | T3a–T3e | tests 8–11, 15 and 16 |
| §4.8: reconcile on the device, steps 1–7 | T4.4, T5.1–T5.5, T6.3, T6.4 | tests 12, 18, 19, 25 and 28 |
| §4.9: which writes are direct and which are cards, row by row | T5.3, T5.4, T5.5 | tests 20–24 and 22b–22d; `a_template_receipt_…` (PQ4); `…_naming_both` (P20) |
| §4.9: a card is charged to the 15-a-day budget, overflow snoozed, never deleted; on day 1 snoozed to day 2 | T5.3, T6.2, T6.3, T8b | `twenty_mail_cards_are_charged_and_overflow_is_snoozed_…`, `the_feed_card_is_charged_to_the_daily_budget`, `a_first_day_card_is_written_snoozed_to_day_two` (`status: snoozed`) |
| §4.10: the canary, its limits and *Shown* | T4.2, T4.3, T6.3 | test 6; `each_mailboxs_state_…`; `a_non_active_mailbox_…` |
| §4.11: the saved session: reconcile, recover, re-enable | T11.2, T11.5, H4 | test 36; T11.2's tests |
| §4.12: Disconnect, steps 0–3 | T4.1, T10.2a, T10.3a, T11.4, T15.2 | `mail_disconnect_writes_turned_off_…`, `gmail_remove_address_…`, `turn_off_email_is_two_steps_…`; proof step 7 |
| §4.13: account deletion and sign-out | T4.6, T12, T15.2 | tests 14 and 38; `delete_my_data_confirm_adds_…` |
| §4.14: "From your email" and Undo | T7, T15.4 | test 29; T15.4's tests; proof step 3 |
| §5.1: the seven tables, the constraint changes, the RPCs and the cron | T1.1, T1.2, T1.3, T3e | test 14; P7's sweep test |
| §5.1: AWS, and the secrets by name | T0.S0, T13b, T18a | the T0 report; the runbook |
| §5.2: vault shapes (`mail_asof`, body lines, card fields, notices, untrusted text) | T6.2, T15.5 | tests 27 and 28c; `a_colon_a_quote_and_an_img_onerror_…`; `mail_derived_text_renders_inert` |
| §5.3: the journal (mail actor; Undo as the student; the approval actors) | T6.2, T8, T15.4 | test 31; `undo_invokes_…` |
| §5.4: app data (profiles, `mail.json`, one `mail` label, no credential) | T9.1, T9.2 | T9.1's tests; test 33 |
| §5.4: Knowlu writes the address to no file and no log of its own | T11.5, T15.5 | `no_file_or_log_of_knowlus_own_holds_the_address_or_token`; `settings-check.py`'s `ui_event` check |
| §6.1: the engine (`mailreconcile`, `mail`, `cloudmodel`, `mail-applied`, `ingest`; the rest unchanged) | T5–T8b, H1, H2 | Checkpoints A and D |
| §6.2: the app (`mail.rs`, the commands, the scheduler, `account.rs`, the wizard, `grades.rs`, the page) | T9–T12, T14, T15, T19, H3–H5 | tests 32–41 |
| §6.3: the cloud functions and their authentication | T2–T4b, P5, P21 | tests 1–16c; Checkpoint B |
| §7: the privacy sentences go only to bump #1's list; no `site/` edit | T13a, D | the HANDOFF row; Checkpoint D |
| §8: `approvals.rs`, `ingest.rs`, `grades.rs` and `account.rs` changed as stated; every other contract file used, not changed | T8, T8b, T19, T12, A, C, D | each task's tests; Checkpoint D's diff check |
| §8: `migrations_test.ts`'s export assertion moved and stronger | T1.2 | the retitled test |
| §8: frozen references untouched; frontmatter additive only | every task | the fixtures diff at every gate; T6.2's tests |
| §9: Gmail (D22), events (`approvals.rs`), M2 (the page), M1 (T19), two desktops (the lease note), C5 and stream J | T0.0, T8, T15.1, T19, T13b, T3a | the disjointness checks; the HANDOFF note |
| §11: the ranked risks | §8 here | the mitigations named there |
| §12.4: page tests, the walk scripts and inert untrusted text | T15.1–T15.5 | T15's tests; `wizard-check.py` and `settings-check.py` |
| §15: what signing changes elsewhere | T0.0, T13a, T13b | the docs commit; HANDOFF; the vision-program row |

### 9.3 The test plan, test by test

| Spec test | Task | Named tests (§5) |
|---|---|---|
| 1: SNS signature, topic, certificate host | T2a, T2g.1 | T2a's five; `a_bad_notification_answers_403_…` |
| 2: binding, zero GETs, lapsed, Rotate's grace, forgeries | T2f, T2g.1 | T2f's five binding tests; T2g.1's first five |
| 3: never reject, verify after | T2f | `a_dmarc_failing_original_…`, `a_pass_only_in_a_sender_written_…`, `no_authentication_result_ever_produces_…` |
| 4: a backfill send and its attachments | T2e, T2g.2 | the batch-send tests; `each_rfc822_attachment_…`, `a_pdf_attachment_is_ignored`, `an_unaligned_backfilled_original_is_kept_unaligned` |
| 5: Gmail's confirm, completed and refused | T2e, T2g.2 | T2e's six confirm tests; T2g.2's three |
| 6: the canary, its route limits and its cadence | T2e, T2g.2, T4.2, T4.3 | the canary tests in each |
| 7: dedup, keyed hashes, put before rows, expiry | T2c, T2g.1, T3e | T2g.1's three dedup tests; `an_expired_item_loses_its_seen_row_…` |
| 8: the screen with zero model calls | T3d | T3d's first five |
| 9: templates and `aligned` | T3a | T3a's tests |
| 10: validation | T3b | T3b's first eight |
| 11: allowance, caps, order, address caps | T1.3, T3c, T3e, T4.1 | `enforce_budget_raises_…`; the caps test; T3e's first seven; T4.1's first two |
| 12: the pull's order, withholding and ack | T4.4 | T4.4's tests |
| 13: resolve replays and bounds | T3b, T4.5 | `a_resolve_answer_naming_an_id_outside_…`; T4.5's tests |
| 14: migrations, RLS, kinds, export, purge | T1.1, T1.2, T1.3, T4.6 | T1's tests; T4.6's tests |
| 15: no log line carries a message's marker | T2g.1, T3d | `no_intake_log_line_…`, `no_process_log_line_…` |
| 16: S3 by SigV4, and nothing left behind | T2b, T2g.1, T3d | T2b's tests; the two `no_object_is_left_…` tests |
| 16b: entitlement on the `mail` routes | T4.1, T11.1 | `get_and_delete_addresses_succeed_…`, `every_other_mail_route_answers_402_…`, `a_402_on_an_entitled_route_reads_lapsed_…` |
| 16c: `gmail-read`'s non-quiet answer | T4b | T4b's tests |
| 17: the pin check | T18b (P8) | `docs/reports/…-pin-check.md` |
| 18: determinism, and a split batch | T5.1, T6.3 | T5.1's two tests; `a_backfill_split_across_pulls_…` |
| 19: supersession and future dates | T5.5 | its three supersession tests |
| 20: §4.9 row by row, with `due` | T5.3, T5.4, T5.5 | the row tests in each |
| 21: judge-once with `due` | T5.4, T6.2 | `a_hand_created_task_…`, `a_hand_set_due_…`, `a_second_claim_while_a_mail_card_is_pending_…` |
| 22: LMS items | T5.5 | its first five |
| 22b: authenticity in the backfill | T5.3 | `an_unaligned_backfill_original_makes_cards_…` |
| 22c: `cancelled` | T5.4 | `cancelled_is_an_archive_card_…` |
| 22d: the first day | T5.3, T6.2 | `a_first_day_card_is_proposed_…`, `a_first_day_card_is_written_…` |
| 22e: two desktops | T5.2, T6.3 | `a_claim_whose_source_uid_is_already_…`, `a_claim_already_on_a_synced_note_…` |
| 23: past due and past events | T5.3 | its two past tests and the future-event test |
| 24: a `new_item` matching a note is a change | T5.2 | `a_new_item_whose_title_equals_…` |
| 25: ambiguity and stored answers | T6.4 | its three resolve tests |
| 26: notices supersede by key | T5.5, T6.2 | `course_info_opens_a_notice_…`, `a_later_notice_closes_the_earlier_…` |
| 27: `mail_asof` on one line | T6.2 | `mail_asof_is_a_single_line_flow_mapping_…` |
| 28: seen, ack, the line, the drain | T6.3 | its first four |
| 28b: no label for a mail card | T6.4 | `a_rejected_mail_card_is_never_reported` |
| 28c: no device judgment of mail text | T6.2, T6.3 | `a_mail_note_is_written_needs_enrichment_false_…`, `judge_makes_no_judge_task_call_…` |
| 29: `mail-applied` | T7 | T7's three tests |
| 30: loopback failure shapes; `judge` exits 0 | T6.1, T6.4 | T6.1's tests; `judge_runs_the_mail_pass_after_…` |
| 31: approving a mail amend | T8 | T8's five tests |
| 31b: `ingest` and ruling 3 | T8b | T8b's seven tests |
| 32: the app's pure pieces | T9.1, T10.1, T11.2 | T9.1's tests; T10.1's tests; T11.2's pace tests |
| 33: the label, one window, the origin list | T9.2 | its first three |
| 34: no OS input, no debugging port | T9.2 | `no_os_input_and_no_debugging_port_in_mail_sources` |
| 35: each step script against synthetic pages | T10.2a, T10.2b, T10.3a, T10.3b | their tests |
| 36: the slot's `mail` step, re-enable and `turned_off` | T11.5, T11.4, T6.3 | T11.5's tests; `mail_disconnect_writes_turned_off_…`; `a_reenabled_mailbox_opens_one_heads_up_…` |
| 37: the ten commands in the console list only | T11.4, H5, T17 | `the_ten_mail_commands_are_in_the_console_list_…`; T17's recount |
| 38: deletion and sign-out | T12 | T12's four tests |
| 39: the predicate, both arms | T19 | the six predicate tests and the source test |
| 40: the command seams, `Suspended` | T19 | the three named `grades.rs` tests |
| 41: the scheduler, `Suspended` | T19 | the three named `scheduler.rs` tests |
| `oracle.rs`, `surface_oracle.rs`, the sync and entitlement tests unchanged | every gate | the workspace gate; Checkpoints A, C and D |

### 9.4 Gates and the live proof

| Spec requirement | Task | What proves it |
|---|---|---|
| §12.5 S0: SES with a DMARC-failing message, and the header evidence | T0.S0 | the T0 report; no cloud task before it |
| §12.5 S1–S3: the result channel, hidden or off-screen, the popup, the protocol prompt | T0.S1–S3 | the T0 report; no window or script task before it |
| §12.5 S2's sender and received-time form | T0.S1–S3 | T2c and T11.2 use the recorded form |
| §12.5 S4: session lifetime and §13 Q3's revisit | T0.S4 | the report and Quinn's ruling |
| §12.5 S5 (optional): an inbox rule beside existing forwarding | T0.S5, T10.1 | the report; T10.1's step list |
| §12.5 S6: Microsoft's pace on a test tenant | T0.S6, T17 | the report; T17 step 6 |
| §12.6 step 1: onboarding, the announcement, the first open, grades offered | T18c | the proof record |
| §12.6 step 2: M365 setup, the canary, backfill, Sent copies, "From your email" | T18c | the proof record |
| §12.6 step 3: Undo holds across the next slot | T18c | the proof record |
| §12.6 step 4: Gmail, the popup, the confirm, the backfill | T18c | the proof record |
| §12.6 step 5: a live amend card; approval as the student; `ingest`'s card | T18c | the proof record |
| §12.6 step 6: quiet, off, re-enabled with the exact notice, recovered; a forwarding pointed elsewhere is asked about | T18c | the proof record |
| §12.6 step 7: Disconnect both; no rows left; nothing re-enabled | T18c | the count-only query |
| §12.6 step 8: cleanup and restore | T18c | the ledger |

**Plan-only tests.** Every named test that pins a plan decision is this plan's addition: P9, P10,
P12, P20, P21, P22, and PQ4's receipt tests. So are the module-level tests under T2a–T2d, the extra
T10.1 and T11.1–T11.3 tests, and the T15 tests beyond §12.4's list. Each pins a spec requirement that
had no test of its own, or only a combined one. None changes a spec test's assertion.

## 10. Review revisions (2026-09-30)

One review round raised eighteen findings. Two of them (the stale base) are the same finding and are
merged as R1, so there are seventeen entries here, R1–R17. Each was checked against `main` at `30b6fb7`
in the root checkout. The planner has no shell, so R2's sibling-lane diffs are taken as reported and
re-run at T0.0 step 5. No signed spec decision is changed here. One fix would need such a change, and
it goes to Quinn as PQ5 (R9). One part of R17 is rejected, with the reason given there. The body of
the plan has been edited to match, and each edit names its R number.

### R1. The plan was written against an old `main` (important; two findings)

- **Verified.** `30b6fb7` is "Merge pull request #27 from quinnhall07/m2-editing", and `m2-editing`
  (`d8653cf`) is in it. `write::set_body` and `write_one_line_literals` are in `write.rs`, and
  `journal::OPS` has seven entries ending in `set_body`. So §2's line "M2 editing is not merged" was
  false, and T0.0's merge of `199cd1f` would have left #27 out.
- **Re-check.** Every name §2 cites was searched for by grep at `30b6fb7`, in `write.rs`, `journal.rs`,
  `approvals.rs`, `enrich.rs`, `ingest.rs`, `info.rs`, `completion.rs`, `commitments.rs`,
  `grades.rs`, `scheduler.rs`, `account.rs`, `commands.rs`, `scaffold.rs` and `onboarding.rs`. All of
  them are present. Line numbers moved, and the plan cites none.
- **Changed.**
  - The header and §2 now name `30b6fb7` as the base.
  - T0.0 step 2 fetches, merges `origin/main` at its HEAD, records the sha, and re-greps §2's names.
  - T15's precondition now reads "M2 is in `main`".
  - H5 and T17 step 3 now note that M2's commands are already in the console list.
  - Checkpoint A now confirms that a mail or feed `append_body` makes an open editor's `set_body` refuse
    as a conflict. T6.2 gains `a_mail_body_line_under_an_open_editor_makes_set_body_refuse_as_a_conflict`.
- **Not changed.** `sync::build_push`'s filtering of non-note paths does not bear on this lane. Mail
  writes only notes in `tasks/`, `archive/` and `approvals/`, and the seen ledger stays local.

### R2. More shared files than §2 expected (important)

- **Verified in part.** The four branches exist (`packed-refs`), and HANDOFF shows `p3-registrar`
  (draft PR #19, Pilot) and `two-desktop` (draft PR #20, Launch) are open. The file lists below are the
  finding's. T0.0 step 5 re-runs them and records the result.
- **Changed.** The disjointness check now also runs before T1.2, T3b, T6.4, T8b and T14. Each file
  below names who rebases and who resolves a conflict. The rule is that a lane later in the program
  (Pilot, Launch) rebases onto this MVP lane, and between two MVP lanes the second to merge rebases.

| Shared file | Sibling | Email task | Who rebases | A conflict goes to |
|---|---|---|---|---|
| `engine/src/approvals.rs` CL | `j-events`, `two-desktop` | T8 | the second MVP lane to merge (events or email); `two-desktop` onto both | `contract-engineer` |
| `engine/src/ingest.rs` | `two-desktop` | T8b | `two-desktop` | `contract-engineer` |
| `engine/src/enrich.rs` | `two-desktop` | T6.4 (one call) | `two-desktop` | `contract-engineer` |
| `engine/src/write.rs` CL | `two-desktop` | none (used only) | `two-desktop`; if it merges first, Checkpoint D re-reads T6.2's use of `write` | `contract-engineer` |
| `_shared/judge_validate.ts` | `j-events` | T3b (`MailKind`) | the second to merge | `cloud-engineer` |
| `_shared/judge_prompts.ts` | `j-events` | T3c | the second to merge | `cloud-engineer` |
| `_shared/migrations_test.ts` | `j-events` | T1.1–T1.3 | the second to merge; the `mail` stamp sorts last | `cloud-engineer` |
| `migrations/20260922120300_event_decomposed.sql` (new on `j-events`) | `j-events` | none; T1.1's stamp sorts after it | — | `cloud-engineer` |
| `app/src/scaffold.rs` (`Curated`) | `p3-registrar` | T14 | `p3-registrar`; T14's fields go after `main`'s | `implementer` |
| `scripts/wizard-check.py` | `p3-registrar` | T15.3 | `p3-registrar` | `console-ui` |
| `engine/src/cli.rs` | `p3-registrar` | H2 | the second to merge | main session |
| `app/src/{lib,main}.rs` | `p3-registrar` | H3–H5 | the second to merge | main session |

- T3b checks `MailKind` against `j-events`' `Kind` and validators before it starts. T1.2's
  `lastModelPinBlocks` change and its retitled export test are checked against `j-events`' edits to
  the same file.

### R3. §4.9's daily budget had no task, test or ledger row (important)

- **Verified.** `approvals::defer_over_budget` (called from `cli.rs` in `rank`) considers only
  approvals that are `type: approval`, `status: pending` and `proposed_at` today, and are not
  `events-digest`. A card shaped any other way escapes the budget and fails no test.
- **Changed.**
  - T6.2's card writer carries T5's `status`, `proposed_at`, `first_proposed_at` and `snooze_until`.
  - T6.3 gains `twenty_mail_cards_are_charged_and_overflow_is_snoozed_by_defer_over_budget`.
  - P12 and T8b give the feed card `status: pending`, with `proposed_at` and `first_proposed_at` set to
    the run's day and `first_proposed_at` set once. T8b gains `the_feed_card_is_charged_to_the_daily_budget`.
  - §9.2 has a new row for §4.9's budget sentence.

### R4. Scripts that change a student's mailbox had no Opus read (important)

- **Verified.** Checkpoint C read only profiles, the origin check, `account.rs` and the card writer.
  Sent cleanup, Gmail's remove-address and forwarding-off were read only by Checkpoint D's
  whole-branch pass. A wrong selector in any of them deletes real mail that cannot be recovered.
- **Changed.**
  - Checkpoint C now reads those scripts and their synthetic pages. Each must select by the nonce
    **and** the Knowlu recipient, or by the Knowlu address alone, and nothing wider.
  - T10.2b and T10.3b gain `…_sent_cleanup_keeps_a_nonce_copy_to_another_recipient_and_a_knowlu_copy_without_the_nonce`.
  - T10.3a gains `gmail_remove_address_leaves_every_other_forwarding_address`.
  - Routing stays as spec §14 has it: the Sonnet `implementer` writes them.

### R5. T11.4 left a red commit until H5 (minor)

- **Verified.** H5 cannot land first, because `generate_handler!` must name functions that already
  exist. As written, then, test 37 was committed while it failed.
- **Changed.** T11.4 commits everything except test 37, with the gate green. The main session applies
  H5, runs the gate, and commits test 37 with H5.

### R6. T6.1 and Checkpoint A's fixes in one worktree (minor)

- **Verified.** P3 allows one writer per worktree. A's fixes and T6.1 both write in `email-engine`.
- **Changed.** T6.1 runs while the reviewer reads, which writes nothing in the worktree. A's fix loop
  opens only after T6.1 has committed. §6's row and Checkpoint A say so.

### R7. S4's revisit could change T11.5's assertion (minor)

- **Verified.** "Reconcile only to repair" is a behaviour, not a value, and §5 forbids changing an
  assertion.
- **Changed.** T11.5 tests a constant, `RECONCILE_CADENCE`, in both of its modes. `Daily` means at most
  once a day per mailbox. `RepairOnly` means only for a mailbox that is not `active`. The shipped value
  is `Daily` until S4. Quinn's revisit changes only the value, and the test is
  `reconcile_follows_reconcile_cadence_in_both_modes`.

### R8. The DOM shim's lock file and egress (minor; accepted in part)

- **Accepted.** `app/assets/mail/deno.lock` (LF) is now named in T10.2a's files and in P14.
- **Not accepted as a rule breach.** CLAUDE.md's rule is that a test needing a server serves itself
  on `127.0.0.1:0`. Module resolution is a toolchain fetch, and the cloud tests' `@std/assert` already
  does it in CI. P14 now names this as the one exception the lane takes, pinned by the lock's integrity
  hashes. The shim is not vendored: vendoring a DOM implementation would add a large third-party tree
  to the repo, for no gain the lock does not already give. If Quinn wants that stricter line, vendoring
  is a one-step change to T10.2a.

### R9. A feed card for `title` can never apply (important)

- **Verified.** `approvals::AMENDABLE_FIELDS` is the nine fields with no `title`, and
  `amendable_fields_is_exactly_the_nine` pins it. `validate_amendment` answers "field not amendable:
  title". That prefix is not in `UNRECOVERABLE_REFUSALS`, so `is_recoverable_refusal` keeps the card
  pending at every `rank`. `a_hand_set_title_behaves_the_same` and
  `the_feed_card_passes_validate_amendment_…` cannot both pass while T8b edits only `ingest.rs`. Spec
  §1's table has the same gap.
- **Changed.** Every fix needs a signed text read differently or changed, so this is Quinn's call.
  It is §4's PQ5, recommending (c): `title` amendable only on a `created_by: blackboard` card. T8 and
  T8b now wait on PQ5, asked at T0.0, and the ledger's D11 (5) row names PQ5's tests.

### R10. A student's edit can land between plan and direct set (important)

- **Verified.**
  - `Journal::read` caches the unbounded read until `append` or `invalidate`.
  - `human_set` reads through that cache.
  - `write_literals` with `WriteOpts::default()` does no human check.
  - The console writes from another process, and `vault_io` is never held across a child.

  So a console `set` made during `resolve_mail`'s network call is invisible to the plan and gets
  overwritten.
- **Changed.**
  - T6.2's direct-set writer calls `journal.invalidate()` and a fresh `human_set(id, field)` just before
    `write_literals`. If a human has set the field, it files the amend card instead.
  - T5's rules name the re-check, and say it can only turn a direct write into a card.
  - The test is `a_human_set_landing_between_plan_and_apply_turns_the_direct_set_into_a_card`.
  - What is left is the time between that re-check and the write itself. No lock spans two processes,
    so this is narrowed, not closed. A human set landing after the mail write is later in the journal
    and in the file, so the student's value stands.

### R11. Mail text could forge seen-ledger lines (important)

- **Verified.** `ingest::record_seen` writes `- {uid} · {title} · first seen {date}` unescaped.
  `load_seen` splits with `pystr::splitlines`, which breaks on `\r`, U+0085, U+2028 and more. The Gmail
  pass passes `item.title` for an item it wrote. A title carrying U+2028 then `- bb-uid-123 · x` would
  mark that Blackboard uid as seen, and `ingest` would then skip it as "deleted earlier".
- **Changed.**
  - T6.3 records every `mail:` uid with the fixed title `(email)`, and never a mail-derived title. The
    test is `the_seen_ledger_records_mail_uids_as_email_and_a_hostile_title_cannot_add_a_line`.
  - T5.1 skips, as `noise`, any claim with a title, sender, why or evidence that
    `write::single_line_problem` refuses. The test is
    `a_claim_with_a_line_break_in_any_field_is_skipped_as_noise`.
  - T3b refuses line separators and C0 controls in `title`, `from` and `evidence` at the service.
  - T6.2's literal test covers `\r`, U+0085 and U+2028 in every mail-derived field.

### R12. The contract-list check was too narrow and diffed a stale `main` (important)

- **Verified.** §1, the gates and Checkpoint D checked only `write.rs`, `journal.rs`, `sync.rs` and
  `provenance.rs`, plus the fixtures, against the local `main`. A Sonnet edit to `ledger.rs` or `ids.rs`
  would have passed. Once `main` moves, a two-dot diff also shows M2's lines as the lane's own.
- **Changed.** Every Rust gate, T17 and Checkpoint D run this after `git fetch`:
  `git diff --stat origin/main...HEAD -- engine/src/{write,journal,yamlemit,yaml,pystr,ledger,ids,provenance,approvals,sync,entitle,wincred,reconcile}.rs app/src/{credentials,account,updates}.rs engine/tests/{oracle,surface_oracle,sync_contract,sync_replay,entitlement_gate}.rs engine/tests/fixtures`.
  The only files it may show are `approvals.rs` (T8) and `account.rs` (T12). In a sub-branch it may
  show only that sub-branch's own one of them.

### R13. The mail writers' frontmatter bytes had no named reviewer (minor)

- **Verified.** P17's stop said "if review finds", and Checkpoint C read only the card bytes.
- **Changed.**
  - P13 and Checkpoint C now have contract-reviewer read T6.2's direct-set writer: `mail_asof` built
    only through `write::to_literal`, and the body line. They also read T6.3's seen-ledger write.
  - P17 names Checkpoint C as the review.
  - T6.2 gains `mail_asof_reapplied_from_its_journal_record_is_byte_identical`.

### R14. One open decision per field, and null values (minor)

- **Verified.**
  - `write::find_pending_amendment` matches an identical field set only. A pending mail `{due}` card
    and an ingest `{due, title}` card could therefore coexist, and after one was approved the other
    would stay stale and pending.
  - `validate_amendment` refuses a null `from` or `to`, and that refusal is recoverable, so a card for a
    hand-cleared `due` could never apply.
- **Changed.**
  - T8b files one card per field. Spec §6.1's wording ("for `title` and `due` … file one `kind: amend`
    card") and its run line (`proposed <stem>: due`) already read per field, so no signed text moves.
    Each card's dedup is then an exact match on `{field}`, and the mail card writer's own check finds
    it too.
  - A null on either side keeps the note's value and files no card.
  - T8b gains three tests: `a_hand_set_title_and_due_both_contradicted_file_two_one_field_cards`,
    `a_pending_mail_due_card_blocks_the_feed_due_card_and_the_reverse` and
    `a_hand_cleared_due_or_a_feed_cleared_due_keeps_the_value_and_files_no_card`.

### R15. A mail card's `append` lines would be journalled as the human (minor)

- **Verified.** `apply_amendment` runs `append_body` with the same `ctx` it uses for `changes`, so
  swapping the context for the whole call writes any append line as the student.
- **Changed.** T6.2 writes no `append` on a mail card. T8's arm refuses a `created_by: mail` card that
  carries one, before `apply_amendment` runs. The refusal is recoverable, so the card stays visible to
  reject, and `apply_amendment` is not edited. T8 gains
  `a_mail_card_carrying_append_is_refused_and_writes_nothing`.

### R16. A first-day card written `pending` shows on day 1 (minor)

- **Verified.** `approvals::transition_note` wakes only `status: snoozed`. A `pending` card shows
  whatever its `snooze_until` says.
- **Changed.** T5.1's amend-card action carries `status`. T5.3's first-day action is
  `status: snoozed`, with `proposed_at` and `snooze_until` set to day 2 and `first_proposed_at` set to
  day 1. T6.2's `a_first_day_card_is_written_snoozed_to_day_two` asserts `status: snoozed`. The spec's
  "snoozed to day 2" (§4.9) already says this, so no signed text moves.

### R17. Four gaps: the runner log, H0's sentence, the address and the spike profiles (minor)

- **(1) Mail skips and `state/runner-log.md`. Accepted.** Verified: `run_slot_with`'s skip filter sends
  `grades (skipped: …)` to `cli::append_run_log`. T11.5 now records no row when no mailbox is
  connected. When one is, the named skips go only to the `RunSummary` the Settings page reads, and
  never to the runner log. A "not due" skip would otherwise add a line every slot. The test is
  `no_mailbox_records_no_mail_row_and_no_mail_step_name_reaches_the_runner_log`.
- **(2) H0's "neither writes the vault". Rejected.**
  - The sentence is A12's signed text, which H0 lands word for word. The plan may not change it.
  - It is also accurate as written. The grades capture writes only app data (`grades-capture.json`).
    Its vault writes are the engine's `grades` step inside the chain, and the scheduler's skip line,
    neither of which is the capture.
  - With (1), the mail step writes nothing in the vault at all.
- **(3) §5.4's "the address in no file and no log". Accepted.** T11.5 gains
  `no_file_or_log_of_knowlus_own_holds_the_address_or_token`. It excludes the WebView2 profiles, which
  are the provider's. The progress view may still show the address from memory (§4.1 step 2). T15.5's
  `settings-check.py` also checks that no `ui_event` carries the address. §9.2 has a row for it.
- **(4) The spike's profiles. Accepted.** T0.S1–S3 now ends by deleting the spike app and every profile
  folder it made, recorded in the T0 report, per the live-proof cleanup rule.

### Open for Quinn after this round

- **PQ5 (blocks T8 and T8b).** How a feed card for `title` becomes approvable. The recommendation is
  (c): `title` is amendable only on a `created_by: blackboard` card, `AMENDABLE_FIELDS` stays nine, and
  every other amend is unchanged. (a) widens the list for every amend in every vault. (b) narrows the
  signed D11 (5) to `due` only.
- **No new ask from R8 or R17.** R8 follows the `@std/assert` precedent, and vendoring stays available
  if Quinn wants it. R17 keeps A12's sentence true by keeping mail skips out of the runner log.

**Plan-only tests added in this round:** every test named in R1, R3, R4, R7, R10, R11 and R13–R17.
Each pins a requirement the review found unpinned. None changes a spec test's assertion. The one
rename, T11.5's cadence test, keeps the "at most once a day" assertion as its `Daily` arm.
