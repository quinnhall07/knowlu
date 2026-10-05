# Email forwarding to Knowlu (implementation plan)

**Date:** 2026-09-30, amended 2026-10-02 for Quinn's rulings (§4, §9.5). **Base:** this lane's branch
and worktree under `.claude/worktrees/`, cut on 2026-09-30 for the spec, renamed `email-forwarding` at
T0.0 (PQ1). The three lane branches were cut from `main` at `25592d4` (§9.5). The branch was `main` at `4aa6560` plus the signed spec and its amendments. `main`
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
**Status:** in integration (T17, 2026-10-03): `email-engine` (through H2) and `email-app` (through T15.5) are merged into the lane by way of `email-app`, and `email-cloud` (through T4b and Checkpoint B's rulings, PQ9, PQ11, PQ12 and the unbound display) beside them. Before that, in build: T5.1–T5.4, T8, T8b, T8c, T5.5 and both Checkpoint A fix rounds are on
`email-engine` (HEAD `4194817`). Checkpoint A passed on 2026-10-02 (Quinn). PQ6, PQ7 and PQ8 were
answered the same day (§4). T5.6 is built (`d0abe3e`); its addendum read opened PQ13 (§4) and
amended T6.3. T6.2 through H2 were built past PQ13's block (§9.5). PQ13 was answered (b), due-aware, on
2026-10-03 (Quinn), and T5.6's fix is built on `email-forwarding` (`79c1742`, with its fix round
`c99785f`, 2026-10-04; review and contract re-check PASS). PQ14 (`2adf0c9`) and D18's release guard
(`5470cd2`) are built and reviewed the same day. The ledger is
`.superpowers/sdd/2026-09-30-email-forwarding/progress.md`.

## 1. Goal, and what must not move

A student signs in to their school mail, and to personal Gmail if they want it, in a Knowlu window
that is only for signing in. Knowlu turns forwarding on to their Knowlu address, proves it with a
test message, and brings in the last 120 days of mail (Quinn, 2026-10-02), applied and listed under "From your email"
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
  `credentials.rs` and `updates.rs` are used, never edited. Only `approvals.rs` (T8, T8c) and
  `account.rs` (T12) change, by contract-engineer, reviewed by contract-reviewer. Every gate checks this with §10
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
  T2a–T2g; T5 is T5.1–T5.6), so the fidelity ledger maps back to spec §14. A dispatch that would
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

  The three RPCs (`charge_allowance`, `refund_allowance` (§9.5), `delete_mail_address`) are SQL.
  Like every RPC here, they are pinned textually in `migrations_test.ts`, and their behaviour is smoke-tested on staging (T18).
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
- **P22. "An active forwarding" (D22)** means a `mail_addresses` row for that Gmail address whose
  status is `confirmed`, `active` or `quiet` (an allow-list; amended 2026-10-02 by Quinn's PQ11
  answer, which replaced "neither `revoked` nor `off`"). A `quiet` forwarding still counts: recover
  brings its gap in, and reading both paths would file every message twice. `pending` and `blocked`
  do not count, so a stalled or abandoned setup keeps the Gmail OAuth read.

## 4. Open questions (each with a recommendation)

- **PQ1. Rename the branch and worktree.** The lane's branch and worktree carry the retired working
  title, and Quinn asked for it to be removed everywhere.
  - *Recommend:* at T0.0, before any code, rename the branch to `email-forwarding`, move the worktree
    to `.claude/worktrees/email-forwarding`, push the new name and delete the old remote branch. The
    branch holds docs only today, so nothing is lost.
  - *Needs Quinn's word:* deleting a remote branch is destructive.
  - **Answered at T0.0, 2026-10-01 (Quinn): done.** The lane branch is `email-forwarding`. The three
    lane branches `email-engine`, `email-app` and `email-cloud` were cut from `main` (`25592d4`), not
    from the lane branch as P3 says (§9.5).
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
  - **Answered 2026-10-02 (Quinn): our own verifier**, as recommended. `_shared/dkim.ts` does
    rsa-sha256 with relaxed and simple canonicalisation, and verifies the **topmost** ARC instance's
    seal (the highest `i=`, sealed by the provider that handed the message to SES) and that instance's
    ARC-Message-Signature. The key resolver is injected, and no test queries DNS. S0's captures
    (§9.5) are the interop corpus. The raw copies are never committed. The progress ledger says they
    are kept for the check; `s0-captures.md` says local copies were deleted. The controller confirms
    which is true before T2d. If they are gone, the check re-captures cases 1, 3, 4 and 6.
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
  - **Answered 2026-10-02 (Quinn): the alternative, a new fifth claim kind `receipt`.** This amends
    Q5's four kinds and spec §5.1's claim shape, which now carries a dated marker.
    - The shape is `{kind: receipt, course, target, field: null, value: null, item: null, topic: null,
      evidence, why, confidence, source: template, resolution}`. No `completed` field value exists.
    - Only a deterministic template parser emits it (T3a), never the model. T3b refuses a `receipt`
      whose `source` is `model`, and `mail-1`'s grammar offers no such kind (T3c).
    - Its `target` is resolved as a `change`'s is (T5.2, D12). T5.5 maps it to
      `completion::propose_done` in both phases. That always proposes, asking once per task, so a
      receipt is always a card and never a direct write. An unmatched receipt takes §4.9's unmatched
      row, a notice.
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
  - **Answered at T0.0, 2026-10-01 (Quinn): (c).** `title` is amendable only on a card whose
    `created_by` is `blackboard`. `AMENDABLE_FIELDS` stays nine, and `amendable_fields_is_exactly_the_nine`
    is unchanged. Built in T8 (`968d86d`).

### Quinn's rulings of 2026-10-02 that no PQ asked

- **F3, the `NotAmendable` rows of §9.5. Answered 2026-10-02 (Quinn): (b).** An amend card may fill
  an **empty** `due`, such as a mail-created "needs a date" note. Approving a card that gives an archived
  `imported-past` note a due on or after the approval day **un-archives** the note. This is a change to
  `approvals.rs`, built by contract-engineer as the new task T8c and read at Checkpoint A. T5.5 then
  plans those cards where T5.4 planned `NotAmendable`.
- **BW, the backfill window. Answered 2026-10-02 (Quinn): a fixed 120 days for every student.** This
  amends signed decision 4's "mail since the term began" and D7. The spec carries a dated marker at
  decision 4.
  - The window starts 120 days before the setup day. It is fixed when the mailbox's first backfill
    batch opens, and every resume or recover of that backfill reads the same start (§9.5).
  - There is no term-start logic. No school is named in code, and there is no default to confirm.
  - Every task that read a term start now reads this window: T1.1, T4.2, T10.1, T11.3, T14, T15.1,
    the ledger and the tests (§9.5). The engine never sees the window, so T6 does not change.
  - VISION check: a fixed window needs no per-school table that would need hand-editing for a second
    school (rule 1). D7's "no age cutoff inside the window" still holds.
- **PS, per-school data. Answered 2026-10-02 (Quinn).** The mail provider comes from the curated
  campus row, else from an MX lookup of the school address's domain (§13 Q4, T4.2's
  `provider_reads_mx_…`). Campus rows are data, never code. No code path branches on a school, and
  T14 adds only the rows' mail fields (`mail_provider`, `mail_domain`).

### Asked after the rulings of 2026-10-02 (all answered the same day)

- **PQ6. The backfill allowance's key under BW.** D23 holds one allowance "per account per term,
  keyed by the term's start" (`mail_accounts.allowance_term_start`), and signed decision 8 says "per
  term". BW leaves no term to key on.
  - *Recommend (a):* one allowance per account per 120-day period. It opens at the account's first
    backfill, and a backfill that opens 120 days or more after the last opening opens a new one.
    Disconnecting and reconnecting inside the period renews nothing. The column becomes
    `allowance_opened_at`. The 14 days, 2,000 judgments and $1.00 are unchanged.
  - *(b)* One allowance per account, ever. This is simpler, but a returning student's setup in a
    later semester trickles at the daily cap.
  - *Blocks:* the one `mail_accounts` column in T1.1, and T1.3's and T3e's allowance tests. T1.1's
    other six tables do not wait. If PQ6 is open when T1.1 starts, `mail_accounts` lands last, in its
    own commit. A ruling other than the recommendation needs a dated marker at decision 8.
  - **Answered 2026-10-02 (Quinn): (a).** One allowance per account per 120-day period. It opens at
    the account's first backfill; a backfill opening 120 days or more after the last opening opens a
    new one; disconnecting and reconnecting inside the period renews nothing. The 14 days, 2,000
    judgments and $1.00 are unchanged. Signed decision 8 carries a dated marker, and spec D23 and
    §5.1 are amended to match.
    - `mail_accounts.allowance_term_start` becomes `allowance_opened_at` (a `timestamptz`).
    - `mail_addresses.term_start` is **kept, renamed `backfill_since`**, as BW's T1.1 note already
      said. It is not dropped: T4.2's bound test and T10.1's resume test read it, and T15.1's
      backfill row shows it. The first backfill batch's `mail_batches.since` could stand in only
      until that row is pruned 30 days after closing, so a later recover would lose its start.
    - T1.1 was built with the spec's old names (`20261002000200_mail.sql`, not applied anywhere).
      T1.3 renames both columns in that same file and updates
      `the_mail_tables_have_section_5_1s_columns_and_words` (T1.1, T1.3).
- **PQ7. The count for a residual `NotAmendable`.** After T8c, a few card rows still cannot be filed:
  a note the student archived, a `cancelled` claim for a note with no `status:` line, and a collection
  `from`. T6.3 counts each on the run line.
  - *Recommend:* count it in `U` (unmatched: "no live item to change"), which keeps §4.8 step 7's
    signed line.
  - *The alternative:* a new slot, which changes the signed line and needs a marker.
  - *Blocks:* T6.3's line test only.
  - **Answered 2026-10-02 (Quinn): `U`.** A residual `NotAmendable` counts in `U`. §4.8 step 7's
    signed line is unchanged, and T6.3's line test says so.
- **PQ8. The same mail by two routes (S0 capture 5).** A rule forward ("FW:") sent from one of the
  student's mailboxes is a new message with a new Message-ID. When it reaches Knowlu through another
  connected mailbox, intake's Message-ID dedup (D8, test 7) cannot see it.
  - *Recommend (i), the screen:* T3d reads §4.7 step 2's "the student's own mail coming back (From
    is the mailbox itself)" as From equal to **any** `mailbox_hint` on the account, outside a batch.
    The test is `a_rule_forward_from_another_of_the_accounts_mailboxes_is_noise`. The cost: a note
    the student mails themselves from one connected mailbox to another is dropped too.
  - *Recommend (ii), the device:* a live `new_item` whose normalised title and course match a
    pending mail `kind: task` card files nothing (`AlreadyApplied`). Today two such claims file two
    cards (progress ledger, T5.3). If answered yes, this is a T5.6 dispatch, one rule and its test,
    before T6.2. T5.5 does not wait for it.
  - *Blocks:* T3d (i) and T5.6 (ii) only.
  - **Answered 2026-10-02 (Quinn): (ii) only.** T5.6 is a task: a live `new_item` whose normalised
    title and course match a pending mail `kind: task` card files nothing (`AlreadyApplied`). It is
    one rule and its tests, built before T6.1 and T6.2. **(i) is declined:** T3d keeps the existing
    rule (From is the mailbox itself) and never reads From equal to any `mailbox_hint` on the account
    as the student's own mail, so a note the student mails themselves between two connected mailboxes
    still reaches the model.
- **PQ9. The Gmail live binding (T2f's review, 2026-10-02; for Checkpoint B).** Spec §4.6 step 5 and
  test 2 need both checks: (i) the provider's AMS `h=` covers a binding header naming this mailbox, and
  (ii) SES's SPF pass on Gmail's `<local>+caf_=…@gmail.com` return path. PQ2's interop check found that
  no AMS covers `X-Forwarded-*`, so (i) as written cannot be met, and Gmail live mail would never bind.
  - *Built (pending this answer):* on Gmail, check (i) is Google's verifying seal and AMS with no
    seeded forwarding field, and does not name the mailbox; check (ii) alone names it. So risk 10's
    "both must hold" rests on (ii) alone on Gmail. The envelope is `@gmail.com` only, as the spec says.
  - *Recommend:* sign an amendment that accepts (i) without the mailbox naming on Gmail, since S0 shows
    no provider-covered field names it, and keep (ii) at `@gmail.com`.
  - *The open half:* a Workspace mailbox's `+caf_=` envelope is on its own domain. Accepting it would
    let a sender who can set that MAIL FROM on Google's infrastructure (a Workspace SMTP-relay tenant
    allowed "any addresses", or a school whose SPF includes Google's shared ranges) bind live mail. If
    Workspace is wanted, accept a non-`gmail.com` domain only when the provider came from a curated
    campus row. Until the answer, Workspace live mail is unbound.
  - *Blocks:* nothing before Checkpoint B. The ruling is recorded in the spec's amendments, then the
    code is kept or narrowed.
  - **Answered 2026-10-02 (Quinn): amend, and accept all of Workspace.** Gmail live mail binds on
    Google's verifying seal plus SES's SPF pass on the `+caf_=` envelope naming the mailbox at
    `gmail.com`. A `googlemail.com` hint expects that `gmail.com` envelope. A `google` mailbox on its
    own domain binds on Google's seal plus SES's SPF pass on the `+caf_=` envelope at that domain naming
    the hint, only when the domain's MX reads Google (`mail/provider.ts`'s `providerOf`, injected as
    `BindDeps.resolveMx`; a DNS failure is unbound, like a DKIM key lookup failure). The MX clause is
    the controller's record of Quinn's answer (progress ledger, 2026-10-02); this question never raised
    it. **PQ12 (answered 2026-10-03) widened it:** the domain's MX reads Google or another host (a
    gateway), and only Microsoft's MX, no MX or a DNS failure refuse. No curated-row
    condition: the server never sees the row. The residual risk (a leaked address plus a sender able to
    set that MAIL FROM on Google's relay) is accepted; Rotate mitigates it, and T18a probes it. The
    spec's §4.6 step 5 carries the amendment. Tests: `a_workspace_forward_binds_by_googles_seal_its_own_domain_caf_return_path_and_a_google_or_gateway_mx`,
    `a_workspace_caf_envelope_on_a_domain_whose_mx_is_microsoft_missing_or_unanswered_is_unbound`
    (both renamed by PQ12), `a_googlemail_hint_expects_a_gmail_com_caf_return_path`.
  - **M365, answered 2026-10-02 (Quinn): find a sealed signal, else accept the risk.** The question was
    whether the topmost `microsoft.com` ARC-Authentication-Results, which the seal covers, records the
    original delivery or recipient domain (`smtp.rcpttodomain=`, or a recipient field). `s0-captures.md`
    records each ARC chain's instances, seal domains and `cv`, and the AMS `h=` lists, but not the AAR's
    fields, and the raw captures were deleted after PQ2's check. So no sealed recipient field is
    established, and the M365 binding stays as built (the SRS envelope with SES's SPF pass, Microsoft's
    verifying seal, and the topmost ForwardingLoop naming the mailbox and `forward_hint`, though that
    field is unsealed). The residual risk, recorded in the spec: a sender on another Microsoft tenant who
    can write the school domain's SRS envelope and an unsealed ForwardingLoop naming the mailbox and
    tenant. T18a's second-tenant probe records the AAR's fields. If the topmost AAR names the hint's
    domain on the real forward and not on the probe, a `cloud-engineer` task adds it as a third check.
    (The test fixture `MS_AAR` has carried a synthetic `smtp.rcpttodomain=school.test` since T2f. It is
    not an S0 record, and nothing reads it.)
- **PQ10. D9's notification rows deferred until real captures exist (T3a's review, 2026-10-02; for
  Checkpoint B).** D9 names "Blackboard, Canvas and zyBooks notifications, as data rows beside
  `RECEIPT_TEMPLATES`", and §4.7 step 3 lists new assignment, due soon, due date changed and
  announcement rows. T3a shipped `MAIL_TEMPLATES` empty: the lane holds no captured notification
  mail, and a row is written from a real message's exact sender, subject and body, never guessed. The
  implementer ruled this alone; departing from a signed decision is Quinn's call.
  - *What the empty list changes:* every such notification goes to the `mail_claim` model. Each one
    spends against the 120-a-day cap and the per-account allowance that §11 risk 9 bounds with "the
    screen and the templates first". Its claim is `source: model`, so it takes §4.9's model rows: in
    backfill, a change to a note no feed supplies becomes a `kind: amend` card where a template claim
    would have written directly. So the gap is a behaviour change as well as a cost.
  - *Recommend:* T3a.2 writes the Blackboard, Canvas and zyBooks rows from real captured mail before
    Checkpoint B. If no capture can be had by then, Quinn accepts the model path until the pilot, and
    T3a.2 lands before the pilot.
  - *Blocks:* Checkpoint B's D9 reading. Nothing built before it waits.
  - **Ruled 2026-10-04 (Quinn).** Blackboard's rows were built from real captures (T3a.2: `bac3159`,
    `eeb49ef`; follow-up `0aea395`, review PASS). For the rest, the model path is accepted for Canvas,
    zyBooks and due-date-changed notifications until the pilot. T3a.3 (�9.5) writes their template rows
    from pilot students' real mail, forwarded as attachments, never guessed. PQ10 is closed.
- **PQ11. Which forwarding statuses stop the Gmail OAuth read (T4b's review, 2026-10-02; for
  Checkpoint B).** P22 counts a forwarding under every status except `revoked` and `off`, and T4b
  built that literally (`FORWARDING_ENDED` in `functions/gmail-read/handler.ts`). That set includes
  `pending` (setup started, never finished), `confirmed` (Gmail's confirm done, the setup canary not
  yet back) and `blocked` (D14: the setup canary never arrived, so mail is not reaching Knowlu).
  - *The failing case:* a student whose Gmail is connected by OAuth starts forwarding setup. P21's
    route sets `mailbox_hint`, and then the canary never arrives (`blocked`) or the student abandons
    setup (`pending`). From the next slot, `gmail-read` answers `read: 0`, `quiet: false`, with no
    reason, so the tray stays green while neither path reads that Gmail. The 120-day backfill fills
    the gap only if forwarding later becomes active, and an abandoned `pending` row never does.
  - *Recommend:* amend P22 so a forwarding counts only while mail arrives by it: `confirmed`,
    `active` or `quiet`. `quiet` stays in, since recover fills its gap and reading both paths would
    file every message twice. `confirmed` stays in because it lasts only the canary's three-minute
    wait before it becomes `active` or `blocked`. `pending` and `blocked` drop out. `paused_send` and
    `lapsed` do not matter here: `paused_send` is Microsoft's (a Gmail hint never sits on a Microsoft
    row, P21), and `gmail-read` is itself entitlement-gated. If amended, `FORWARDING_ENDED` becomes an
    allow-list and the P22 status test moves `pending` and `blocked` to the not-counting side.
  - *If Quinn keeps P22 as written:* the §9.5 row stays as the record of the trade-off (a stalled or
    abandoned setup silently stops the Gmail read), so Checkpoint D does not raise it again.
  - *Blocks:* nothing before Checkpoint B. The code is unchanged until the answer.
  - **Answered 2026-10-02 (Quinn): the allow-list.** A forwarding counts, and the Gmail OAuth read
    steps aside, only while the row is `confirmed`, `active` or `quiet`. `pending` and `blocked` keep the
    OAuth read, and so does any other status, known or not. `FORWARDING_ENDED` became
    `FORWARDING_COUNTS` in `functions/gmail-read/handler.ts`. P22 and the spec's D22 carry the amendment.
    The P22 status test now reads `P22: a forwarding counts only while confirmed, active or quiet`, and
    `a_gmail_address_without_a_forwarding_reads_exactly_as_today` gains a pending case and a blocked case.
- **PQ12. PQ9's MX clause and Workspace schools behind a gateway (B-rulings review, 2026-10-02; for
  Checkpoint B).** The controller's record of Quinn's PQ9 answer binds a Workspace mailbox only when its
  domain's MX reads Google (`googleHosted` in `_shared/mail_bind.ts`). PQ9 as asked never raised MX.
  `hintFits` (`mail/provider.ts`) lets an `other` MX fit a `google` mailbox, since refusing a Proofpoint,
  Mimecast or campus-relay gateway in front of Google would need a per-school exception (rule 1).
  - *The failing case:* a student at a Workspace school behind a Proofpoint MX sets up a `google`
    mailbox. The hint fits, and the setup canary binds by Knowlu's DKIM and nonce, which never reads
    MX, so the row goes `active`. Every real forward is then unbound: deleted, counted as unbound, and
    Settings suggests a leaked address and offers Rotate. `last_received_at` never moves, the row goes
    `quiet`, and the periodic canary marks it healthy again. If the same address is also connected by
    Gmail OAuth, PQ11 counts the row and `gmail-read` steps aside, so neither path reads that mailbox.
    VISION does not allow that silence.
  - *What MX adds:* little. SES's SPF pass on the school's own domain already needs that domain's SPF to
    authorise the relay, and the residual risk (a Workspace SMTP-relay tenant on Google's shared ranges)
    is the same whether or not a gateway receives the school's mail.
  - *Recommend:* (a) match `hintFits`. Bind when `providerOf` reads `google` or `other`, and refuse
    `m365`, no MX or a DNS failure. The `gateway.test` case in
    `a_workspace_caf_envelope_on_a_domain_whose_mx_is_not_google_is_unbound` moves to the binding side,
    and T18a's Workspace-relay probe also runs on a gateway domain. (b) Keep the clause and make the
    loss visible: the canary refuses a `google` mailbox whose domain's MX is not Google's, with a named
    reason, so setup stops instead of passing.
  - *Blocks:* nothing before Checkpoint B. The code keeps the MX clause until the answer (§9.5).
  - **Answered 2026-10-03 (Quinn): (a).** A Workspace own-domain `+caf_=` envelope binds when
    `providerOf` reads `google` or `other` (a gateway such as Proofpoint or Mimecast); only `m365`, no MX
    or a DNS failure refuse. `googleHosted` in `_shared/mail_bind.ts` reads that; the gateway case moved
    to the binding test, now `a_workspace_forward_binds_by_googles_seal_its_own_domain_caf_return_path_and_a_google_or_gateway_mx`,
    and the refusing test is `a_workspace_caf_envelope_on_a_domain_whose_mx_is_microsoft_missing_or_unanswered_is_unbound`.
    T18a's Workspace-relay probe also runs on a gateway domain. Spec §4.6 step 5 carries the amendment.
  - **The unbound display, ruled the same day (Quinn, 2026-10-03; no PQ asked it).** When live forwards
    keep arriving unbound while the canary binds, the mailbox state the app reads (`GET /addresses` in
    `functions/mail`) carries a named reason with the count since the last bound live forward, and Rotate
    is suggested only for mail from outside the student's own school or provider.
    - *Where it came from:* intake classifies each unbound arrival with `unboundOrigin`
      (`_shared/mail_bind.ts`): **own** when its sole From domain matches the hint's domain (equal, a name
      under it, or a name it sits under) or its topmost ARC seal is the mailbox's own provider's and
      verifies; else **foreign**. It counts `unbound_own` or `unbound_foreign` through
      `count_mail_arrival`, which also adds to the cumulative `unbound`. A bound live forward (`live`, or
      `unauthenticated`: bound, then failed step 6) zeroes both runs. A refused Gmail confirmation still
      counts `unbound` alone.
    - *The columns:* `mail_addresses.unbound_own_since_live` and `unbound_foreign_since_live`, added in
      the still-unapplied `20261002000200_mail.sql` (T1.3's precedent). Both are in D17's export.
    - *The row:* each `GET /addresses` row gains `unbound_reason` (`skipped`, `leaked` or null),
      `unbound_note` (the sentence), `unbound_since_live` and `suggest_rotate`. The threshold is
      `UNBOUND_STREAK = 2`: that many unbound live forwards with no bound one between. **leaked**
      (`UNBOUND_LEAKED`, `suggest_rotate: true`) when that many were foreign, whatever the status.
      Otherwise **skipped** (`UNBOUND_SKIPPED`, "some forwarded mail could not be verified and was
      skipped") when that many arrived from anywhere while the status is `active` or `quiet`.
    - *Tests:* `an_unbound_message_is_own_when_from_the_hints_domain_or_sealed_by_the_mailboxs_provider_else_foreign`,
      `unbound_live_forwards_count_own_or_foreign_since_the_last_bound_live_forward`,
      `the_mailbox_state_names_skipped_mail_after_two_unbound_live_forwards_and_offers_rotate_only_for_foreign_mail`,
      and the updated `count_mail_arrival_adds_one_to_one_mailboxs_unbound_or_unauthenticated` and
      `the_mail_tables_have_section_5_1s_columns_and_words`.
    - *T15.2 renders it* (Settings amber with the sentence and count; Rotate offered only when
      `suggest_rotate`). Spec §4.6 step 5 and §5.1 carry the amendment.
    - *After a Rotate (built 2026-10-03, open for Quinn, §9.5's Rotate row):* Rotate zeroes both counts
      in the update that retires the old token, and an unbound arrival through a retired token counts on
      `unbound` alone. Tests: `a_rotate_clears_the_leaked_reason`,
      `an_unbound_arrival_through_a_retired_token_runs_no_since_live_count`.

### Asked after T5.6's addendum read (2026-10-02; answered 2026-10-03)

- **PQ13 (numbered PQ9 on `email-engine` and `email-app` until T17 renumbered it, 2026-10-03). A new due for a task a pending mail card proposes.** As built (`d0abe3e`), T5.6 matches an
  open mail `kind: task` card by normalised title and course only. A live `new_item` for that task
  with another due is skipped as already applied: no card and no notice, and once the message is
  acked the date is gone. Example: a pending card for "Lab 5" in CS 101 due 2026-10-12, then a live
  `new_item` "Lab 5" in CS 101 due 2026-10-14T23:59, plans `[Skip(AlreadyApplied)]`, and
  `a_live_new_item_matching_a_pending_mail_task_card_files_nothing` pins it. Before T5.6 this filed a
  second card. Had the task been a note, test 24 would read the same mail as a due change and file
  an amend card, so a pending card is the one place a moved date is lost without a trace. A weekly item
  that reuses its title is dropped too while last week's card is pending. PQ8's answer named title
  and course and never addressed the due; the code follows its words.
  - *Recommend (b), due-aware:* skip only when the claim has no due, or its due falls on the card's due
    day (the card's `due` read from its fenced payload, T6.3). Any other due takes `New`'s rows: a
    second card, as before T5.6. S0 capture 5's duplicate still files one card, since both copies
    carry the same date. The costs: if the model reads two dates from the two copies, the student sees one
    card per date; and a time moved within the same day is not caught while the card is pending.
  - *The alternative (a):* confirm title and course only. The finding closes, and nothing changes.
  - *If (b):* a contract-engineer fix to T5.6's two files. Five cases of
    `a_live_new_item_matching_a_pending_mail_task_card_files_nothing` move from the skip to `New`'s
    rows, every claim whose due day is not 2026-10-12 (`2026-10-14T23:59`, `2026-09-20`, the exam and
    both events). That moves a built assertion, which §5 allows only on a ruling. A named test,
    `a_live_new_item_with_another_due_files_its_own_card`, pins the 2026-10-14T23:59 case. `walk`'s
    argument that "resolution reads no `due`" is re-checked, since `carded` would read a task card's
    due.
  - *Blocks:* T6.2, by T5.6's rule that no Important finding from its read is open. T6.3 fills a
    card's `due` either way. *(2026-10-03: T6.2 through H2 were built with PQ13 open and merged into
    `email-app`. The block now falls on Checkpoint D and T18, before any live mail pass; §9.5.)*
  - **Answered 2026-10-03 (Quinn): (b), due-aware. Built 2026-10-04 (`79c1742`, `email-forwarding`).**
    `Working::carded` takes the claim's known due: an open mail task card silences a live copy only
    when the copy knows no due or its due falls on the card's due day; another day resolves `New`
    and files its own card. A due day is the calendar day `rules::due_date` reads from the floating
    wire-shaped due (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`, no zone applied), on both sides; a card with
    no readable due matches only a copy with no due. The five cases moved as written above (four
    cards; the past event is counted past, D24), and the named test also pins a weekly "Quiz" whole
    and split. `walk`'s argument holds: a `due` is read only on a task card, which no set or rewrite
    touches. The two costs above are accepted, not fixed. (b)'s "no due" clause brings a third,
    unnamed when it was asked (PQ13 fix round, 2026-10-04): a weekly item reusing an open card's
    title that arrives with no due (this week's "Quiz", announced with no date while last week's card
    is open) is read as that card's copy, skipped as already applied and acked, with no card and no
    notice, so it surfaces only if later mail names it again. It follows the ruling's words, so it
    stays unless Quinn rules otherwise. The fix round also pins that the card comes before D24: a
    past copy on the card's own due day is already applied, never counted past or carded again.

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
| T8 | `contract-engineer` (Opus, xhigh) | yes | PQ5 (answered) | `engine/src/approvals.rs` CL | T5.4, PQ5 (built first, §9.5) | engine |
| T8b | `contract-engineer` (Opus, xhigh) | yes | PQ5 (answered) | `engine/src/ingest.rs` | T8, PQ5 | engine |
| T8c | `contract-engineer` (Opus, xhigh) | yes | F3 (answered) | `engine/src/approvals.rs` CL | T8b | engine |
| T5.5 | `contract-engineer` (Opus, xhigh) | yes | PQ4, PQ7 (answered) | `engine/src/mailreconcile/{mod,resolve,rules}.rs` | T8c | engine |
| A | `contract-reviewer` (Opus, xhigh) | yes | diffs to Quinn | `docs/reports/…-email-forwarding-contract-review-a.md` | T5.5 and both fix rounds; **passed 2026-10-02** | — |
| T5.6 | `contract-engineer` (Opus, xhigh) | yes | PQ8 (answered: (ii)); PQ13 (answered (b) 2026-10-03; fix built `79c1742`, §9.5) for its read's fix | `engine/src/mailreconcile/{resolve,rules}.rs` | A | engine |
| T6.1 | `implementer` (Sonnet, high) | no | — | `engine/src/cloudmodel.rs`, `engine/tests/mail_pass.rs` (new) | T5.6 | engine |
| T6.2 | `implementer` (Sonnet, high) | no | — | `engine/src/mail.rs` (new), `engine/tests/mail_pass.rs` | A, T5.6's read, T6.1, H1b | engine |
| T6.3 | `implementer` (Sonnet, high) | no | PQ7 (answered) | `engine/src/mail.rs`, `engine/tests/mail_pass.rs` | T6.2 | engine |
| T6.4 | `implementer` (Sonnet, high) | no | — | `engine/src/mail.rs`, `engine/src/enrich.rs` (one call), `engine/tests/mail_pass.rs` | T6.3 | engine |
| T7 | `implementer` (Sonnet, high) | no | — | `engine/src/mailapplied.rs` (new) | T6.4, H1c | engine |
| T1.1 | `cloud-engineer` (Opus, high) | yes | — | `cloud/supabase/migrations/<stamp>_mail.sql` (new), `migrations_test.ts` | S0 | cloud |
| T1.2 | `cloud-engineer` (Opus, high) | yes | — | the same two files | T1.1 | cloud |
| T1.3 | `cloud-engineer` (Opus, high) | yes | PQ6 (answered) | the same two files | T1.2 | cloud |
| T2a | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/sns.ts` (new, with its test) | T1.3 | cloud |
| T2b | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/s3.ts` (new) | T2a | cloud |
| T2c | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mime.ts` (new) | T2b | cloud |
| T2d | `cloud-engineer` (Opus, high) | yes | PQ2 (answered) | `functions/_shared/dkim.ts` (new) | T2c | cloud |
| T2e | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mail_bind.ts` (new) | T2d | cloud |
| T2f | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/mail_bind.ts` | T2e | cloud |
| T2g.1 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-inbound/**` (new), `config.toml` | T2f | cloud |
| T2g.2 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-inbound/**` | T2g.1 | cloud |
| T3a | `cloud-engineer` (Opus, high) | yes | PQ4 (answered) | `functions/_shared/mail_templates.ts` (new) | T2g.2 | cloud |
| T3a.2 | `cloud-engineer` (Opus, high) | yes | PQ10 (ruled 2026-10-04: capture now) | `functions/_shared/mail_templates.ts` (the vendor rows region) and its test | T3a, real captures | cloud; **built 2026-10-04**: Blackboard new gradable item and due soon; Canvas, zyBooks and due-date-changed not captured |
| T3a.3 | `cloud-engineer` (Opus, high) | yes | PQ10 (ruled 2026-10-04: model path until the pilot) | `functions/_shared/mail_templates.ts` (the vendor rows region) and its test | T3a.2, pilot students' real mail (forwarded as attachments) | cloud; **pilot task**: Canvas, zyBooks and due-date-changed rows, written from real captures only, never guessed |
| T3b | `cloud-engineer` (Opus, high) | yes | PQ4 (answered) | `functions/_shared/judge_validate.ts` (mail region) | T3a | cloud |
| T3c | `cloud-engineer` (Opus, high) | yes | — | `functions/_shared/judge_prompts.ts`, `judge_caps.ts` | T3b | cloud |
| T3d | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-process/**` (new), `functions/mail-inbound/handler.ts` (D8's kick), `config.toml` | T3c | cloud |
| T3e | `cloud-engineer` (Opus, high) | yes | PQ6 (answered) | `functions/mail-process/**` | T3d | cloud |
| T4.1 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail/**` (new), `config.toml` | T3e | cloud |
| T4.2 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail/**`, `functions/_shared/canary_send.ts` (new) | T4.1 | cloud |
| T4.3 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-canary/**` (new), `config.toml` | T4.2 | cloud |
| T4.4 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-pull/**` (new), `config.toml` | T4.3 | cloud |
| T4.5 | `cloud-engineer` (Opus, high) | yes | — | `functions/mail-resolve/**` (new), `config.toml` | T4.4 | cloud |
| T4.6 | `cloud-engineer` (Opus, high) | yes | — | `functions/account/**` | T4.5 | cloud |
| T4b | `cloud-engineer` (Opus, high) | yes | — | `functions/gmail-read/handler.ts` and its test | T4.6 | cloud |
| B | `reviewer` (Opus, high) | yes | — | `docs/reports/…-email-forwarding-cloud-review.md` | T4b | — |
| T14 | `mechanical` (Sonnet, low) | no | UA's mail fields | `app/src/scaffold.rs`, `app/src/onboarding.rs`, `app/tests/{scaffold,onboarding}.rs`, the `Curated` literals in `app/tests/{grades,scheduler}.rs` | T19 in the lane | app |
| T9.1 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/mail_session.rs` (new), `app/tests/mail.rs` (new) | T14, H3a | app |
| T10.1 | `implementer` (Sonnet, high) | no | — | `app/src/mail_steps.rs` (new), `app/tests/mail.rs` | T9.1, H3c | app |
| T9.2 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/mail.rs` (new), `app/tests/mail.rs` | S1–S3, T10.1, H3b | app |
| T10.2a | `implementer` (Sonnet, high) | no | — | `app/assets/mail/m365/` (forwarding), `app/assets/mail/{deno.json,deno.lock,test_dom.ts}` | T9.2 | app |
| T10.2b | `implementer` (Sonnet, high) | no | — | `app/assets/mail/m365/` (mailbox) | T10.2a | app |
| T10.3a | `implementer` (Sonnet, high) | no | — | `app/assets/mail/gmail/` (forwarding) | T10.2b | app |
| T10.3b | `implementer` (Sonnet, high) | no | — | `app/assets/mail/gmail/` (mailbox) | T10.3a | app |
| T11.1 | `implementer` (Sonnet, high) | no | — | `app/src/mail_client.rs` (new), `app/tests/mail.rs` | T10.3b, H3d | app |
| T11.2 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (recover, reconcile), `app/tests/mail.rs` | T11.1 | app |
| T11.3 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (the setup runner), `app/tests/mail.rs` | T11.2; then H4 | app |
| T12 | `contract-engineer` (Opus, xhigh) | yes | — | `app/src/account.rs` CL, `app/tests/account.rs` | T11.3 | app |
| C | `contract-reviewer` (Opus, xhigh) | yes | — | `docs/reports/…-email-forwarding-contract-review-c.md` | T12, T6.2 | — |
| T11.4 | `implementer` (Sonnet, high) | no | — | `app/src/mail.rs` (commands), `app/tests/mail.rs` | C, `email-engine` merged in; then H5 | app |
| T11.5 | `implementer` (Sonnet, high) | no | — | `app/src/scheduler.rs`, `app/tests/scheduler.rs` | T11.4 | app |
| T15.0 (added 2026-10-03 by the controller) | controller's dispatch (Opus) | yes | **closed** (§9.5): G2 kept, signed by Quinn 2026-10-04, for D21, §5.2, §6.2, against §6.1's "Unchanged: `surface`" | `engine/src/surface.rs`, `docs/surface/anatomy.md`, this plan | T11.5 | app |
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

**Done (recorded 2026-10-02).**
- Q-B was answered `us-east-2`, not `us-east-1`. AWS's new sign-up fixes the region, and SES
  receiving is offered there. The staging receiver exists: `in.knowlu.com` is verified, and the
  bucket, topic and rule set are built. T18a sets `AWS_REGION=us-east-2`.
- The DMARC gate **passed 2026-10-01**. A `p=reject`, unaligned message was stored, with DMARC FAIL
  recorded. Every cloud task may start.
- The real forwards were captured on 2026-10-02 in `.superpowers/sdd/2026-09-30-email-forwarding/s0-captures.md`,
  as header facts only. The intra-school message is DKIM-signed by Microsoft when forwarded, so it is
  aligned and there is no proportion to report. T2f's facts are listed under T2f.
- The temporary SQS queue is deleted. Still Quinn's: remove the S0 Gmail filter and the S0 forwarding
  address in that Gmail.

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

Done 2026-10-02: see docs/reports/2026-10-02-email-forwarding-s1-s3.md

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
**Done:** merged as #29 (`77fa9bf`). It is in `main`'s `25592d4` and so in every lane worktree. T14
and T11.5 no longer wait on it.

### T13a. HANDOFF for the suspension and the lane (`docs-keeper`, on T19's branch)

**Why docs-keeper:** this is HANDOFF text only. It ships in T19's PR because D18's guard must reach
`main` with the suspension (spec T19). **Edits, by heading, never by line:**
- **§3, Pilot, *Release gate*.** No release is cut from a `main` carrying the Gmail settings row,
  email forwarding to Knowlu, or the suspended grades gate until privacy bump #1 merges (D18).
  **2026-10-04 (Quinn's D18 ruling):** the email forwarding half is mechanical. `release.yml`'s first
  build step runs `scripts/ci/privacy-guard.ps1`, which fails the release while any mail marker
  (`mail-inbound`, `app/src/mail_steps.rs`, `engine/src/mailreconcile`) is in the tree and
  `PRIVACY_VERSION` is on or before `2026-09-24`; `engine/tests/workflows.rs` pins the step and runs
  the script. The lane may merge to `main` before bump #1; a release may not ship it.
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

### T5. The reconciler (`engine/src/mailreconcile/`), in six dispatches

**Agent, all six:** `contract-engineer` (Opus, xhigh). **Why:** it is off the list, but it is the
one place that decides direct write or card (D11). A silent error there changes a student's plan
with no card, which spec risk 1 calls the worst harm in the lane. T5.6 was added by PQ8 (ii),
answered 2026-10-02. **Rules for all six:**
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

#### T5.5. Feed items, templates, receipts, supersession, notices and the F3 cards (`mod.rs`, `resolve.rs`, `rules.rs`)

**Before it:** T8c is committed (F3). PQ4 is answered. The P10 grep below is already done and
recorded (progress ledger, 2026-10-01).
**First step:** grep every `created_by:` the engine's templates write, and record the list in the
ledger (P10).
**Tests first** (spec tests 22, 19, and 26's planning half, plus P10 and D24 pins):
- `a_model_change_to_an_lms_item_is_a_card`
- `an_lms_item_already_holding_the_value_writes_nothing`
- `a_backfill_template_change_to_a_fed_item_writes_nothing_and_counts_superseded`
- `a_backfill_template_change_to_an_unfed_note_is_direct`
- `a_live_template_change_is_a_card_unless_equal`
- `a_claim_earlier_than_mail_asof_is_superseded` (19, across pulls; renamed from
  `a_claim_not_later_than_…`, §9.5)
- `a_live_claim_older_than_an_applied_backfill_value_is_superseded` (19)
- `a_future_dated_message_orders_at_its_received_time` (19)
- `course_info_opens_a_notice_keyed_mail_course_topic` (26)
- `an_unmatched_claim_is_a_notice`
- `feed_creators_hold_the_feeds_and_nothing_else` (P10)
- `no_action_ever_targets_commitments` (D24)
- PQ4, the receipt kind:
  - `a_template_receipt_is_propose_done_in_both_phases` (§4.9's receipt row): always a
    `ProposeDone`, never a direct write.
  - `a_receipt_from_the_model_is_noise`: the device re-checks the shape, and a `receipt` whose
    `source` is `model` skips as noise.
  - `a_receipt_on_a_note_this_plan_files_names_it_by_its_creating_claim` (`Place::Planned`, §9.5)
  - `an_unmatched_receipt_is_an_unmatched_notice`
- F3 (b), once T8c is in:
  - `a_live_date_for_a_needs_a_date_mail_note_is_an_amend_card_from_null`
  - `a_date_for_a_hand_made_note_with_no_due_is_an_amend_card_from_null` (row 6; the plan's
    reading, §9.5)
  - `a_due_ahead_for_an_archived_imported_past_mail_note_is_an_amend_card`, in both phases
  - `a_note_the_student_archived_is_still_not_amendable`: an `archive/` note without
    `archived_reason: imported-past` (`done`, or deleted by hand) still skips as `NotAmendable`
  - `a_cancelled_claim_for_a_note_with_no_status_line_is_still_not_amendable`

**The sanctioned changes to built assertions (F3, PQ4).** Quinn's F3 ruling changes what two T5.4
tests assert: `a_card_the_approvals_executor_would_refuse_is_never_planned` and
`a_backfill_never_sets_an_archived_mail_note_to_a_due_today_or_later`. Their undated, hand-made and
`imported-past` cases change from `NotAmendable` to the cards above. The second test's
whole-or-split property must still hold: the whole plan's card names the note by `Place::Planned`
and the split plan's by path, as the test's still-past case already does for its `Set`. Their other cases are unchanged: the `done` archive, the
missing `status:` line, and a due still past set directly. PQ4 changes one T5.1 assertion
(Checkpoint A, finding 3): `a_claim_in_the_wire_shape_deserializes_and_recover_reads_as_live` used
`receipt` as its unknown kind, which PQ4 makes a kind. Its probe becomes `completed`, a word no
ruling adds (the field value PQ4 declined), and the test gains an assertion that a `receipt`
deserializes. The commit message names the rulings. Checkpoint A reads these three diffs line by
line. No other built assertion moves. **Answered 2026-10-02 (Quinn), at Checkpoint A:** the PQ4
probe is approved: the unknown-kind test probes `completed` and asserts that `receipt` deserializes.

**Behaviour.** §4.9's rows 7–9 and 13–15, D13 and D24. Values are compared in the vault's literal
form (`write::to_literal`), so a date and a date-time never compare equal by accident.
- **The receipt kind (PQ4).**
  - `ClaimKind` gains `Receipt` in `mod.rs`, with the shape §5.1's marker gives. A `receipt`
    carrying a `field`, `value`, `item` or `topic` is noise.
  - `resolve.rs` resolves its `target` as it resolves a `change`'s, stored answers included.
  - `rules.rs` plans `ProposeDone` for it in both phases. Supersession and `mail_asof` do not apply
    to it.
- **The F3 cards (T8c).**
  - `amendable` mirrors T8c exactly. It accepts a null `from` for `due`. It also accepts a `due` card
    on an `archive/` note whose `archived_reason` is `imported-past` when `to` is on or after
    `today`.
  - The view gains `archived_reason`. T6.3 fills it.
  - Every other case keeps T5.4's rule. A due still before `today` on an `imported-past` note is
    still set directly when the row writes directly.
- **The residual `NotAmendable`** counts in `U` (PQ7, answered 2026-10-02).

#### T5.6. A live task already pending as a mail card files nothing (`resolve.rs`, `rules.rs`)

**Why this agent:** `contract-engineer` (Opus, xhigh), as for all of T5. The rule decides whether a
card is filed at all, so a match that is too loose drops a real task with no card (spec risk 1).
**Before it:** Checkpoint A has passed, and PQ8 is answered (ii) only (§4). **After it:**
contract-reviewer reads its one commit as an addendum to Checkpoint A's report while T6.1 runs; T6.2
starts when no Critical or Important finding from that read is open.

**The plan's reading of "pending".** A card counts while it is an open decision: `status: pending`
or `snoozed`, the rule §9.5's open-decision row already applies to amend cards. A first-day mail
task card is written snoozed (R16), so "pending" alone would let a rule forward on day 1 file a
second card. If Quinn meant `pending` alone, the snoozed test below flips.

**Tests first** (PQ8 (ii); S0 capture 5's shape, synthetic):
- `a_live_new_item_matching_a_pending_mail_task_card_files_nothing`: the skip is `AlreadyApplied`,
  and no other action is planned for the claim.
- `a_snoozed_mail_task_card_blocks_it_too` (the reading above, on vault day 1 and day 2).
- `another_course_or_another_normalised_title_still_files_its_card`
- `only_an_open_mail_task_card_blocks`: a rejected mail task card (`archive/`), a Gmail task card
  and a mail amend card with the same title and course block nothing.
- `two_matching_live_items_file_one_card_whole_or_split`: in one plan, the second claim matches the
  card the first plans; delivered over two pulls, it matches the card in the vault. Test 18's
  determinism holds.
- `a_backfill_new_item_keeps_its_rows` (T5.3's rows are unchanged; the rule is live only, and
  `recover` reads as live).
- PQ13 (b) (answered 2026-10-03, built `79c1742`): the first test keeps only its same-day and no-due
  cases as the skip; its five other-day cases take their own rows. New:
  `a_live_new_item_with_another_due_files_its_own_card` (the 2026-10-14T23:59 case, both day
  boundaries, an undated card, and a weekly "Quiz" with next week's due, whole and split). The fix
  round adds a past event and task on a past card's own due day to the first test: already applied,
  so the card comes before D24 (a mutation that let a past copy reach `new_item` passed every test
  before them).

**Behaviour.** One rule, beside T5.2's new-item-to-change match and before §4.9's live task row.
- Normalisation is T5.2's (case, punctuation and whitespace folded); the course must be equal.
- PQ13 (b): the claim matches only with no known due, or a due on the card's due day (the calendar
  day of the floating wire-shaped due, both sides; a card with no readable due has no day).
- A card is matched by `kind: task`, `created_by: mail` and an open status, from the view's
  `approvals/` entries. If `NoteView` does not carry a card's `kind`, it gains it in `mod.rs` and
  T6.3 fills it; that is the one edit outside the two named files.
- The skip is counted where T5.2's already-applied skips are, so the run line does not move.
- Nothing is written, so no Undo and no notice is needed; the student decides the one card.

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

### T8c. A mail amend may fill an empty due, or un-archive an imported-past note (`approvals.rs`)

**Agent:** `contract-engineer` (Opus, xhigh). **Why:** `approvals.rs` is on the contract list, and
this is Quinn's F3 ruling (b). A wrong arm moves a note out of `archive/` that the student put there,
or opens every amend to a null `from`. **Before it:** T8b is committed. Repeat the disjointness check
for `two-desktop`, which edits `approvals.rs` (§10, R2). **Reviewed at:** Checkpoint A.

**The plan's reading of the ruling. Answered 2026-10-02 (Quinn), at Checkpoint A: confirmed.**
- Both arms are keyed on the **card's** `created_by: mail`, as T8's actor arm is.
- They change only `due`, so spec §8's "every other amend executes exactly as today" stays true.
- Arm (1) is keyed on the card, so it fills **any** dateless task, a hand-made one too (row 6 makes
  that a card), and never checks the target's `created_by`. The hand-made test stands as built.

**Tests first**, in `approvals.rs`'s test module beside T8's:
- `a_mail_card_fills_an_empty_due_as_the_human_actor`: the target has `due: null` or no `due:`
  line, and the card's `from` is null. The due is written, journalled as T8 writes.
- `a_null_from_on_any_other_card_or_field_is_refused_exactly_as_today`: a blackboard card, an
  `agent:knowlu.enrich` card, and a mail card for `status`. All are refused with today's string, and
  all are recoverable.
- `a_mail_card_with_a_null_from_against_a_note_that_now_has_a_due_is_refused`: the note gained a due
  after the card was filed. The stale card is handled as any card whose `from` no longer matches is
  today, and never overwrites that due.
- `approving_a_future_due_for_an_imported_past_mail_note_unarchives_it`: the note moves from
  `archive/` to `tasks/` through `write::move_note`. It gets `status: active`, the new `due`, and
  `archived_reason` set to null.
- `a_due_that_is_past_by_approval_day_is_set_in_place_and_the_note_stays_archived`: this is T5.4's
  "a due still past is set directly". There is no refusal, so the card never sits pending for good.
- `an_archive_target_on_any_other_card_or_note_is_refused_exactly_as_today`: a non-mail card, or a
  note without `archived_reason: imported-past` (`done`, or deleted by hand). The refusal is
  unrecoverable, `target outside tasks/courses`.
- `an_unarchive_interrupted_between_its_writes_completes_or_stays_pending`: a failure injected
  between the steps either leaves the card pending with the note unchanged, or a re-run finishes
  it, at the suffixed path too when the plain one was taken. A card never settles while its note is
  half moved.
- `an_unarchived_task_can_still_be_proposed_done` (Checkpoint A): after an un-archive,
  `Journal::human_set(id, "status")` is empty and `completion::propose_done` on the moved task
  proposes, never `Skipped("status set by hand")`.
- `a_repeated_title_unarchives_to_the_first_free_suffixed_path` (Checkpoint A): with
  `tasks/lab-5.md` (and `-2`) taken, the note moves to the first free suffix, the notes already
  there are untouched, and the card settles.
- `amendable_fields_is_exactly_the_nine` and the `AMENDABLE_FOLDERS` pin (`["tasks", "courses"]`)
  pass unchanged.

**Behaviour.**
- Arm (1) relaxes one check, `validate_amendment`'s "from/to must both be present and non-null". It
  does so only for `due`, only on a `created_by: mail` card, and only when the target's `due` is null
  or absent. `to` must still be non-null and a single line.
- Arm (2) widens target resolution to `archive/` for a mail card whose `changes` hold only `due`,
  when the note's `status` is `archived` and its `archived_reason` is `imported-past`.
  `resolve_amend_target` and `AMENDABLE_FOLDERS` are unchanged; the arm sits beside them, as PQ5's
  title arm sits beside `AMENDABLE_FIELDS`.
- "Future" is the rule T5.3's `imported-past` uses: a due on or after the approval day. The note's
  move and its `due` are the student's, under T8's actor (`journal::read_human_actor`, with the
  executor's `via` and run id).
- **Ruled 2026-10-02 (Quinn): status/archived_reason as agent:approvals.** An un-archive writes
  `status: active` and `archived_reason: null` with the executor's own ctx (`agent:approvals`), not
  the student's, so `Journal::human_set(id, "status")` stays empty and `completion::propose_done`
  can still propose the task done. The due is written first, then those two, so a failure between
  them leaves the note `archived` (it never ranks) and the next run finishes it.
- **Free suffixed path (Checkpoint A).** Ingest names `archive/` and `tasks/` apart, so a repeated
  title can sit in both. The note moves to the first free `tasks/<stem>[-N].md`, ingest's own suffix
  loop, chosen before the move; a taken path is never a refusal. An interrupted run is finished at
  the move record's new path.
- The card's own bookkeeping stays `agent:approvals`. `write.rs` is used, never edited.

### Checkpoint A. Contract review of T5.1–T5.5, T8, T8b and T8c

**Passed 2026-10-02 (Quinn).** Quinn confirmed T8c's reading (arm (1) keyed on the card) and
approved T5.5's PQ4 probe. T5.6 (PQ8 (ii)) came after it and is read as an addendum (T5.6).

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
  `set_body` refuse as a conflict, as M2 intends, and never drops the line (§10, R1);
- re-reads the first fix round (`b697cf8`, `4e4c873`, `968d86d`), then T8c and T5.5 (added
  2026-10-02), then `fa980a1` and `4194817` (the second fix round, through `email-engine`'s HEAD at
  the pass):
  - T8c's two arms against F3 (b) and the plan's reading under T8c, which goes to Quinn with the
    report;
  - that no other amend changes, and the `AMENDABLE_FIELDS` and `AMENDABLE_FOLDERS` pins;
  - the un-archive's order of writes and its interrupted case;
  - that T5.5's `amendable` mirrors T8c exactly;
  - the two T5.4 assertions F3 changes, line by line;
  - the receipt kind (PQ4): template-only, and always a card.

After the pass, T5.6 is built, and T6.1 runs while the reviewer reads T5.6's commit, since the
review only reads commits. A fix loop for that read opens only after T6.1 has committed, so one agent
at a time writes in `email-engine` (§10, R6). Fixes go back to `contract-engineer`, and the reviewer
re-reads them. **Quinn** read the report with the diffs (spec §14's checkpoint) and passed it. T6.2
starts when no Critical or Important finding from T5.6's read is open.

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
(§4.10's *Shown*). Mail has no `Quiet` arm: a mailbox's state is data, never an error. The JSON the
two calls send and read is written under T4.4 and T4.5 (the wire shapes), which the cloud lane builds
to. A pull reply with no `messages` array is "an unreadable reply", unlike `pull_gmail_queue`, so key
drift is loud (test `a_mail_pull_reply_without_a_messages_array_is_an_unreadable_reply_not_an_empty_pull`).

#### T6.2. The writers (`mail.rs`)

**Before it:** hand-off H1b. **Tests first** (spec tests 27, 21's writer half, 22d's write half, 26,
the first half of 28c, plus P13 pins):
- `mail_asof_is_a_single_line_flow_mapping_and_no_other_byte_moves` (27, P17)
- `a_second_claim_while_a_mail_card_is_pending_files_no_second_card` (21), run on vault day 2 and
  on vault day 1, where the first card is written snoozed (R16) and still blocks the second
  (Checkpoint A, §9.5)
- `a_snoozed_card_for_the_field_files_no_second_card` (§10, R3 and R14): a card
  `defer_over_budget` or the student snoozed, whoever filed it, blocks a mail card for the same note
  and field
- `a_first_day_card_is_written_snoozed_to_day_two` (22d)
- `a_later_notice_closes_the_earlier_by_close_key` (26)
- `a_mail_note_is_written_needs_enrichment_false_with_the_claims_effort_and_importance` (28c)
- `a_mail_amend_card_passes_validate_amendment_and_carries_from_mail_date_and_evidence` (D11 (3),
  D21)
- `a_fill_due_card_and_an_unarchive_card_pass_validate_amendment` (F3, T8c): `from: null` on an
  undated note, and `target: archive/<name>` on an `imported-past` note.
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
- **Cards.** The card writer uses `write::create` after `ingest::open_amendment` finds no open card:
  a `kind: amend` card for the note and exactly the same field, `pending` or `snoozed`, whoever filed
  it (§9.5; `write::find_pending_amendment` alone misses a snoozed card). The task card
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
  unmatched`. A residual `NotAmendable` counts in `U` (PQ7, answered 2026-10-02), so the line is
  §4.8 step 7's signed line, unchanged. A receipt counts in `K` when
  `propose_done` files a card, and nowhere when it has already asked about that task.
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
- `a_rule_forward_in_a_later_pull_files_no_second_card` (T5.6 against real card files; S0 capture 5's
  shape, synthetic): pull one delivers a live `new_item` "Lab 5" in CS 101, and T6.2's writer files
  `approvals/task-lab-5.md` (`created_by: mail`; `status: snoozed` on vault day 1, `pending` on day
  2). Pull two delivers the rule forward (a new Message-ID, the same item) and "Lab 5" in MATH 220.
  On both days, `approvals/` then holds exactly two mail task cards: the first, unchanged byte for
  byte, and one for MATH 220; pull two's line counts `K` = 1. It fails if the view leaves a card's
  `kind` or `course` as `None`, which every T5.6 unit test would miss.

**Behaviour.** `pull_mail(vault, client, opts, budget)` (§4.8 steps 1, 4–7).
- It builds T5's view from `tasks/`, `archive/` and `approvals/`, with `Journal::human_set` for each
  field a claim may touch, each note's `archived_reason` (T5.5), and every note's `kind` as its
  frontmatter holds it (T5.6).
- A `kind: task` card in `approvals/` is viewed as the task it proposes (T5.6's addendum). Its own
  frontmatter gives `status`, `created_by` and `source_uid`. Its fenced `task` payload's frontmatter
  gives `title`, `course` and `due`, since `write_gmail_card`'s shape puts no `course:` or `due:` on
  the card. The fence is found by `approvals::materialize`'s pattern (`PAYLOAD_FENCE`, mirrored in
  `mail.rs`; `approvals.rs` is not edited), and the payload is read only, never re-dumped. A card
  whose payload has no frontmatter or no `course:` keeps `course: None`, so it matches no claim: a
  second card, never a dropped task. `due` is read whichever way PQ13 is answered; since PQ13 (b)
  (`79c1742`), T5.6 compares its day.
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
`phase` (`backfill` or `recover`) as the spec lists it. Two changes come from BW (§4, §9.5):
- `mail_addresses.term_start` is named `backfill_since`. It is the mailbox's window start, set once
  when its first backfill batch opens.
- `mail_accounts.allowance_term_start` is `allowance_opened_at` (PQ6 (a), answered 2026-10-02).

**As built.** T1.1 landed before PQ6 was answered, with the spec's old names (`term_start`,
`allowance_term_start`) in `20261002000200_mail.sql`, which is applied nowhere yet. T1.3 renames both
columns in that same file, never in a new migration, and updates
`the_mail_tables_have_section_5_1s_columns_and_words` to the new names. That moves a built assertion
under PQ6 and BW (§9.5).

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

**Before it:** PQ6 is answered (a) (2026-10-02); nothing here waits.
**First step:** the two renames T1.1 left (T1.1, *As built*): `allowance_term_start date` becomes
`allowance_opened_at timestamptz`, and `term_start` becomes `backfill_since`, both in
`20261002000200_mail.sql`. `the_mail_tables_have_section_5_1s_columns_and_words` names the new
columns, and a grep of `cloud/` finds neither old name.
**Tests first** (spec test 14's RPC half, and 11's database half):
- `delete_mail_address_revokes_then_deletes_one_mailboxs_rows_delivered_claims_included`: it is
  `security definer` with a pinned `search_path`.
- `charge_allowance_is_one_statement_like_charge_call`, and in that one statement it opens the
  allowance (PQ6 (a)): when `allowance_opened_at` is null or 120 days or more before now, it sets
  `allowance_opened_at` to now, 2,000 judgments, $1.00 and `allowance_until` 14 days on, then
  charges; otherwise it only charges what is left, and after `allowance_until` it charges nothing.
- `refund_allowance_gives_one_judgment_back_only_while_the_allowance_is_open` (T1.3's review, after
  F1's `refund_call`; §9.5): one statement, one judgment back, only before `allowance_until` and
  never above 2,000; it never opens, extends or re-prices an allowance.
- `the_allowance_period_is_keyed_on_allowance_opened_at_and_never_on_a_mailbox`: no
  `mail_addresses` column or row enters the opening test, so a Disconnect deletes nothing it reads.
- `enforce_budget_raises_the_ceiling_by_the_allowance_only_while_it_is_open` (D23 as amended: 2,000
  judgments and $1.00 per account per 120-day period, within 14 days of its opening)
- `the_mail_cron_jobs_use_the_vault_token`: `mail-process` every minute, `mail-canary` hourly, and
  `mail-process` in sweep mode nightly (P7).

**Behaviour.** D23 and decision 8 as amended 2026-10-02 (PQ6 (a)). The allowance opens at the first
backfill charge of a period, which is the account's first backfill in practice: the opening rule
lives in the one SQL statement, so T4.2's batch route does not change.

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
an allowed host. Every refusal answers 403 and reads nothing more. Only `SignatureVersion` 2
(SHA-256) is accepted (§9.5), so the topic's `SignatureVersion` attribute must be 2 before anything
subscribes (T18a, T13b's runbook).

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
- `the_topmost_arc_instances_seal_and_its_ams_verify_and_report_the_sealing_domain`
- `a_lower_arc_instance_is_never_read_as_the_topmost`
- `the_key_resolver_is_injected_and_no_test_queries_dns`

**Behaviour.** PQ2 as answered on 2026-10-02: rsa-sha256, relaxed and simple, and the topmost ARC
instance's seal and AMS. In production the resolver uses `Deno.resolveDns` for TXT records. **After
it:** the main session runs the interop check over S0's captures (PQ2's answer; cases 1, 3, 4 and 6)
and records pass or fail by case in the T0 report.

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
- `a_sender_seeded_x_forwarded_for_below_googles_own_is_unbound` (the sender writes an X-Forwarded-For
  naming a victim; Google adds its own above it and signs the bottom one, so the AMS passes and covers
  the forged line; the ARC pass's `unsignedExtra` names it, and binding refuses even with check (ii)
  met)
- `an_m365_forward_naming_another_tenant_than_forward_hint_is_unbound`
- `a_dmarc_failing_original_with_an_aligned_provider_hop_result_is_aligned` (3)
- `a_pass_only_in_a_sender_written_authentication_results_is_unaligned` (3)
- `a_backfilled_original_reads_only_the_providers_delivery_stamp_or_arc`
- `no_authentication_result_ever_produces_a_non_2xx_answer` (3)

**Behaviour.** §4.6 steps 5 (the live case and "anything else") and 6. The words recorded are
`aligned` or `unaligned`, never a header. A binding header is read only from the covered instance of a
passing ARC (`dkim.ts`'s `covered`), and only when the pass's `unsignedExtra` does not name it. A signer
picks fields from the bottom up (RFC 6376 §5.4.2), so a header the sender wrote below the provider's own
is the one signed, and an unsigned instance of the same name means the binding is refused (T2d review,
2026-10-02). Check (i) and check (ii) must each stop the seeded forgery alone (risk 10).

**S0's facts (2026-10-02, `s0-captures.md`).**
- **Gmail binding.** The envelope sender is `<user>+caf_=<knowlu-address-with-=>@gmail.com`: the
  Knowlu address with its `@` written as `=`. It encodes this mailbox's own destination, and the
  topmost seal is `google.com`.
  `X-Forwarded-To` and `X-Forwarded-For` are present but forgeable, and never bind alone.
- **Microsoft binding.** `X-MS-Exchange-ForwardingLoop: <mailbox>;<tenant GUID>` names the tenant,
  and the topmost ARC seal is `microsoft.com`. Microsoft rewrites the envelope by SRS on the school
  domain. S0 records Microsoft's AMS `h=` only in part. The interop check records whether
  `X-MS-Exchange-ForwardingLoop` is in that `h=`. If it is not, the tenant binding also needs the
  SRS envelope with SES's SPF pass, and Checkpoint B reads the case.
- **Intra-school mail** is DKIM-signed by Microsoft when forwarded, so it is `aligned`.
- **Every forwarding hop adds its own ARC set**, and the topmost seal is the provider that handed the
  message to SES. The original's DKIM survives Gmail's forward.
- **The same mail can arrive twice, by two routes, with different Message-IDs.** Intake's dedup is
  by Message-ID (D8) and does not change here. PQ8 covers the duplicate: answered (ii) only, so the
  device's T5.6 files one card, and T3d's screen is unchanged.
- **A forward-as-attachment from Gmail or Outlook keeps the original whole** (its Received lines,
  Authentication-Results, DKIM and Message-ID), so T2g.2 reads the original's own delivery stamp.

**The branch built (2026-10-02, after PQ2's interop check; signed by Quinn's PQ9 answer of
2026-10-02, §4).** No provider's topmost AMS covers `X-Forwarded-For`, `X-Forwarded-To` or
`X-MS-Exchange-ForwardingLoop`, so `mail_bind.ts` binds this way:
- **Check (ii) carries the binding on both providers.** The envelope is the verified SNS
  notification's (`mail.source`, `receipt.spfVerdict` `PASS`, and the recipient whose token found the
  mailbox), never the message's `Return-Path`. Gmail: `<hint local>+caf_=<recipient, @ as =>@gmail.com`
  for a `gmail.com` or `googlemail.com` hint. Workspace (PQ9): the same local part at the hint's own
  domain, and only after the seal passes, that domain's MX must read Google or another host, a gateway
  (`BindDeps.resolveMx`; PQ12 (a), answered 2026-10-03); Microsoft's MX, no MX or a DNS failure refuse.
  M365: `<hint local>+SRS=…@<hint domain>`.
- **Check (i)** is Google's or Microsoft's topmost seal and AMS, verifying. A signed forwarding field
  with an unsigned twin refuses the message. For M365, the topmost ForwardingLoop (or the covered one, if
  the AMS ever covers it) must name the hint and the canary's `forward_hint`. A null `forward_hint`
  binds nothing.
- **X-Forwarded-\* never bind.** For Gmail, check (i) cannot name the mailbox, which departs from spec
  §4.6 step 5 (PQ9). The attacker's-own-Gmail forgery is stopped by (ii) alone, and the seeded forgery
  by each check alone. Risk 10 holds for M365 in full.
- **Step 6.** A live forward reads the verified hop's AAR. An original reads the provider's verifying
  topmost ARC set, else the topmost `Authentication-Results`, counted only with `mx.google.com` (Google)
  or no authserv-id (EOP's form). One From field only. `other` reads nothing and is `unaligned`.
- **Arrival kinds.** T2e's `other` is now `unbound`, and `live` carries the word.
- **Step 6's alignment** is from the safe side: a dkim or spf pass aligns when its domain is the From
  domain or a parent of it; a domain under the From aligns only through the provider's own `dmarc=pass
  header.from=`. A From on a bare label never aligns (T2f's review, 2026-10-02).
- **PQ9's M365 outcome (2026-10-02, §4):** no sealed recipient field is on record, so the M365 binding
  above is kept, with its residual risk and T18a's second-tenant probe.
- **Checkpoint B reads:** PQ9 (answered 2026-10-02, §4), and whether EOP
  SRS-rewrites an **intra-school** forward. S0 did not record case
  4's envelope. If it does not, live intra-school mail is unbound.

**S0's facts (2026-10-02, `s0-captures.md`).**
- **Gmail binding.** The envelope sender is `<user>+caf_=<knowlu-address-with-=>@gmail.com`: the
  Knowlu address with its `@` written as `=`. It encodes this mailbox's own destination, and the
  topmost seal is `google.com`.
  `X-Forwarded-To` and `X-Forwarded-For` are present but forgeable, and never bind alone.
- **Microsoft binding.** `X-MS-Exchange-ForwardingLoop: <mailbox>;<tenant GUID>` names the tenant,
  and the topmost ARC seal is `microsoft.com`. Microsoft rewrites the envelope by SRS on the school
  domain. S0 records Microsoft's AMS `h=` only in part. The interop check records whether
  `X-MS-Exchange-ForwardingLoop` is in that `h=`. If it is not, the tenant binding also needs the
  SRS envelope with SES's SPF pass, and Checkpoint B reads the case.
- **Intra-school mail** is DKIM-signed by Microsoft when forwarded, so it is `aligned`.
- **Every forwarding hop adds its own ARC set**, and the topmost seal is the provider that handed the
  message to SES. The original's DKIM survives Gmail's forward.
- **The same mail can arrive twice, by two routes, with different Message-IDs.** Intake's dedup is
  by Message-ID (D8) and does not change here. PQ8 covers the duplicate: answered (ii) only, so the
  device's T5.6 files one card, and T3d's screen is unchanged.
- **A forward-as-attachment from Gmail or Outlook keeps the original whole** (its Received lines,
  Authentication-Results, DKIM and Message-ID), so T2g.2 reads the original's own delivery stamp.

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
- `a_redirect_after_the_post_is_refused_and_named` (5): a 3xx answer to the form's POST is not followed,
  is refused, and logs `post_redirect`, never the page's `redirect` (T2g.2 review, ruling 4)
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
- `a_submission_receipt_template_yields_a_receipt_claim` (PQ4): `kind: receipt`, `source: template`,
  a `target` and a course, and `field`, `value`, `item` and `topic` null.
- `an_unaligned_receipt_mail_yields_no_receipt` (only an `aligned` vendor sender matches a template)
- `the_code_names_no_vendor` (a source test: vendors are data rows)

**Behaviour.** §4.7 step 3, and PQ4's `receipt` kind as §5.1's marker gives it. Dates go through
`judge_due.ts`, and course codes are matched against the known courses. `lms_receipts.ts` is not
edited.

**Built (2026-10-02, `5ce22b4`):** `MAIL_TEMPLATES` ships empty, since no notification mail has been
captured and rows are never guessed. D9's rows are deferred to T3a.2 under PQ10.

#### T3a.2. D9's notification rows (`_shared/mail_templates.ts`, under PQ10)

**Needs:** real Blackboard, Canvas and zyBooks notification mail (new assignment, due soon, due date
changed, announcement), captured with its exact sending address, subject and body. The controller
supplies the captures. The agent never reads a student's or quinn-ops's mailbox or vault. A capture
is cut to the lines a row matches before it enters the repo, and it carries no student content.

**Tests first:** for each vendor and notification kind captured, one test that the aligned capture
yields its template claim (with `judge_due.ts`'s resolved due where the mail carries a date), one that
the same mail unaligned yields nothing, and `the_code_names_no_vendor` still green.

**Behaviour.** Rows only, inside `// vendor rows: begin` … `end`. No matcher change. A kind with no
capture gets no row, and its mail stays on the model path.

**When:** before Checkpoint B, or before the pilot if Quinn accepts the model path until then (PQ10).

**Built (2026-10-04, PQ10 ruled "capture now"):** two Blackboard rows from 23 real notifications Quinn
forwarded (the captures stayed outside the repo; the tests use invented messages in the vendor's layout).
`New gradable item added to <course>` and `<item> is due soon in <course>` (marker `Assignment due soon`)
each yield a `new_item` task with the item's title, its due through `judge_due.ts` (date and clock time
as the mail shows them, zone left out: `mail-process` passes no timezone, and a zoned due would land on
the UTC clock), the course by scan, and a fixed effort of 1 hour and importance 3. Not rows: Blackboard
announcements, course-subject or professor-sent (free text, and one sample itself moves a due: the model
reads it); `New content`, `New grade and feedback` and `Gradable item is overdue` (not D9 kinds; noise
rows since the follow-up below); `Submission received` stays `lms_receipts.ts`'s row. **Not covered (no
capture):** a due-date-changed notification from any vendor, and every Canvas and zyBooks notification;
those stay on the model path.

**Follow-up (2026-10-04, Quinn's rulings before T18):** (1) **A template's claims are checked against the
whole text.** `mail-process` validates a `source: template` claim against the text scrubbed but never cut
(the scrub check on evidence stands, so an address in a title is still refused); only the model's claims
are held to the 2,000-character cut it was shown. Fix round eeb49ef's `cutOff` (a template claim the cut
left out went to the model) and its two cut tests are gone; in their place a Blackboard fixture whose
style block is 400+ characters longer than the captures', its evidence wholly past the cut, still yields
its claim and passes the handler's check, and `handler_test.ts`
(`a_template_claim_is_checked_against_the_whole_scrubbed_text_and_a_model_claim_against_the_cut`, with
the real `model.ts` path) pins both: a past-the-cut template claim queued with no model call, an address
past the cut refused, a model claim past the cut refused, one inside it queued. (2) **Vendor noise rows.**
`NOISE_TEMPLATES` (data, between the vendor markers): Blackboard's `New content in <course>`, `New grade
and feedback for <item> in <course>` and `Gradable item is overdue in <course>`, from
`do-not-reply@blackboard.com`, matched on the exact address and the whole subject shape only (anchored,
case-sensitive, no body pattern), `aligned` mail only. Checked as the screen's rule 5 (`screen.ts`,
`vendor noise`), not as a `TemplateClaim` kind: the screen's noise path is already the no-model, no-charge,
no-write ack (the row and the object deleted, counted `noise`), where a noise claim would have needed an
evidence span, a validation bypass and a special case for the all-noise result. A subject any claim row
reads (the D9 rows, the receipt) is never noise, so an item titled like a noise subject keeps its row;
announcements (`New announcement in <course>`, a professor's `<course>: <title>`) match no shape and stay
on the model path.

#### T3b. Validation (`_shared/judge_validate.ts`, the mail region)

**Before it:** the disjointness check is repeated for `j-events`, which edits `judge_validate.ts` and
`judge_prompts.ts`. `MailKind` is checked against `j-events`' `Kind` and validators, and T3b and T3c
rebase onto them if they merged first (§10, R2).
**Tests first** (spec test 10, and 13's validation half):
- `evidence_not_verbatim_after_whitespace_folding_is_refused_incomplete`
- `a_line_separator_or_control_character_in_title_from_or_evidence_is_refused` (§10, R11): `\r`, `\n`,
  U+0085, U+2028, U+2029 and C0 controls other than tab. Evidence is folded first (D9), so the breaks
  JS `\s` folds (`\r`, `\n`, VT, FF, U+2028, U+2029) match as one space; the rest stay refused.
- `an_unknown_course_reads_null`
- `an_unresolvable_due_reads_null`
- `a_sixth_claim_is_dropped`
- `a_claim_below_point_six_is_refused`
- `a_new_item_without_effort_or_importance_in_range_is_refused`
- `a_topic_outside_the_fixed_set_is_refused`
- `a_receipt_claim_from_the_model_is_refused` (PQ4)
- `a_receipt_carrying_a_field_value_item_or_topic_is_refused` (PQ4)
- `a_resolve_answer_naming_an_id_outside_its_batch_reads_none` (13)

**Behaviour.** §5.1's claim shape as amended on 2026-10-02 (five kinds, `receipt` from templates
only), and D9, with `MailKind` (P6). The claim kind `receipt` is not a judgment kind, so `MailKind`
is unchanged.

#### T3c. Prompts and caps (`judge_prompts.ts`, `judge_caps.ts`)

**Tests first** (spec test 11's cap half, plus plan pins):
- `the_mail_claim_prompt_holds_only_scrubbed_text_cut_to_2000_characters`
- `the_resolve_prompt_holds_only_ids_titles_and_dues`
- `the_mail_1_grammar_offers_no_receipt_kind` (PQ4)
- `mail_daily_caps_are_120_for_claims_and_20_for_resolve`
- `the_existing_kinds_caps_are_unchanged`

**Behaviour.** The prompt and grammar `mail-1` (§4.7 step 4), the resolve prompt (D12), and P6's caps.

#### T3d. The worker, part 1: parse, screen, templates (`mail-process`)

**Tests first** (spec tests 8, 15 and 16, the process halves):
- `bulk_mail_from_an_off_list_sender_is_noise_with_zero_model_calls` (8)
- `an_edu_sender_with_list_unsubscribe_passes_on` (8)
- `a_microsoft_bulk_score_of_seven_or_more_is_noise`
- `a_promoted_rule_answering_noise_is_noise`
- `the_students_own_mail_outside_a_batch_is_noise`: the existing rule only, From is the mailbox
  itself.
- `mail_from_another_of_the_accounts_mailboxes_is_not_screened_as_own_mail` (PQ8 (i) declined,
  2026-10-02): From equal to a different `mailbox_hint` on the account passes the screen. S0 capture
  5's duplicate is caught on the device by T5.6 instead.
- `no_process_log_line_carries_the_messages_marker` (15)
- `no_object_is_left_after_noise_refused_template_only_or_unauthenticated` (16)
- `a_hand_off_kicks_mail_process_once_with_the_job_token` (D8, "kicked by intake"): in
  `mail-inbound`, a hand-off that made at least one new work item sends one POST to `mail-process`
  carrying `x-knowlu-job-token`; a duplicate or a non-work arrival sends none.
- `a_failed_or_slow_kick_still_answers_200_and_keeps_the_work` (D8): the kick is fire-and-forget,
  bounded in time; its failure is a fixed log line, never a 500, and the work row stays for the cron.

**Behaviour.** §4.7 steps 1–3, plus the step 6 deletes. The `[functions.mail-process]` section goes
in (P5). D8's kick: after a successful hand-off `mail-inbound` POSTs to `mail-process` (the
`MAIL_JOBS_TOKEN` header, the URL from `SUPABASE_URL`) without awaiting its work; the every-minute
cron stays the backstop. T2g.1 built intake without it (review, 2026-10-02), so the kick is T3d's.
`20261002000200_mail.sql`'s cron comment says so: T3d corrects that comment if the file is still
unapplied, and otherwise leaves it (an applied migration is never edited) and names the kick here.

#### T3e. The worker, part 2: charge, model, queue, expire, sweep (`mail-process`)

**Tests first** (spec tests 11 and 7's expiry half, plus P7's sweep):
- `a_backfill_item_charges_the_allowance_not_the_daily_cap`
- `the_2001st_backfill_item_waits_for_the_daily_cap`
- `a_model_failure_on_a_backfill_item_refunds_its_allowance_judgment_and_waits_to_retry` (§9.5):
  the pipeline's `model failed` calls `refund_allowance` and leaves the item to wait-and-retry, never
  an immediate re-charge; a refusal, a truncation or a failed validation stays charged.
- `charge_and_enforce_budget_run_before_the_spy_model`
- `a_capped_item_makes_no_model_call_and_waits_to_the_next_utc_day_at_most_three_times`
- `allowance_calls_add_tokens_to_usage_daily_but_no_calls`
- `the_raised_ceiling_ends_with_the_allowance`
- `disconnect_and_reconnect_do_not_renew_the_allowance`: inside the 120-day period, a reconnected
  mailbox's backfill gets only what is left of the open allowance, or none after its 14 days, and
  never a new one (PQ6 (a)).
- `a_second_backfill_inside_120_days_of_the_opening_opens_nothing`
- `a_backfill_120_days_or_more_after_the_last_opening_opens_a_new_allowance`: 2,000 judgments,
  $1.00 and 14 days again, with `allowance_opened_at` moved to that backfill's day.
- `an_expired_item_loses_its_seen_row_and_a_later_recover_is_accepted` (7)
- `a_queued_row_holds_the_claims_and_metadata_and_no_text` (§4.7 step 6)
- `the_sweep_closes_timed_out_batches_and_prunes_by_age` (P7)

**Behaviour.** §4.7 steps 4–6 (D8, D9, D23 as amended by PQ6 (a)) and P7. The fake store models
T1.3's `charge_allowance`, opening rule included; the period's key is `allowance_opened_at`.

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
- `a_backfill_batch_reaching_more_than_120_days_before_its_opening_is_refused` (BW): the service
  bounds the window, so a device bug cannot open a wider backfill. The first backfill batch sets
  `backfill_since`, and every later backfill or recover batch that finishes it reuses that value.
- `sent_closes_a_batch_with_the_devices_count`
- `reconcile_answers_only_held_fingerprints_in_the_window_and_the_key` (§4.11 step 3)
- `the_reconcile_request_carries_only_the_window` (nothing about unforwarded mail reaches the service)
- `state_reenabled_stamps_reenabled_at_and_elsewhere_sets_off`
- `state_confirming_opens_a_fifteen_minute_confirm_window` (P21)
- `provider_reads_mx_through_an_injected_resolver_as_m365_google_or_other` (§13 Q4)

**Behaviour.** The rest of spec §6.3's `mail` routes, with §4.10's limits. The provider route is the
MX fallback that Quinn's per-school ruling (PS) names: a curated row's `mail_provider` wins, else the
service reads the school address domain's MX.

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

**Wire shape (T6.1's addendum; the device in `cloudmodel.rs::pull_mail_claims` is built to it, and
`engine/tests/cloud_contract.rs` pins it).** The request is `{ack: [uid]}`. The reply is one JSON object:
`{messages: [...], mailboxes: [...], more: bool}`.
- `messages` is **required** and an array (empty is a real, empty pull). A reply without it, or with
  another key in its place (`claims`, `rows`), reads on the device as "an unreadable reply" and no mail
  lands, so the key name is a contract. Each row carries the `mail_claims` column names: `uid`
  (`mail:<h>`), `message_date` (nullable), `ordering_date`, `sender_name` (nullable), `sender_addr`,
  `phase`, `authenticity`, `claims` (the §5.1 jsonb array, one object per claim, in index order) and
  `judgment_id` (nullable). Rows are in delivery order (D10); the device never re-sorts.
- `phase` is one of `backfill`, `live` or `recover`; the device reads `recover` as `live`. Any other
  word makes the row unreadable: counted, not acked, so it returns.
- Every date is an RFC 3339 timestamp with a `Z` or an offset (`2026-10-02T09:30:00Z`), never a date
  alone. A row with no parsable `ordering_date`, no `sender_addr` or no `mail:` uid is unreadable and
  is not acked. `message_date` and `reenabled_at` may be null or absent.
- `authenticity` is `aligned` or `unaligned`; anything else reads `unaligned` (only `aligned` backfill
  may write directly, §4.9).
- `mailboxes` is an array of `{address_id, status, reenabled_at, unbound, unauthenticated, expired}`
  (§4.10's *Shown*): `status` is a `mail_addresses.status` word (an unknown word reads `Unknown`), the
  counters are non-negative integers. No address and no token ever ride here.
- `more` is `true` when the service stopped at its wall-clock budget; it defaults to `false`.

**Test, added to the list above:** `the_pull_reply_keys_and_phase_and_date_formats_are_the_devices`:
assert the answer's top-level keys are exactly `messages`, `mailboxes` and `more`, each row's key set
is the one above, `phase` is in `{backfill, live, recover}` and every date parses as RFC 3339.

#### T4.5. Resolve (`functions/mail-resolve/`)

**Tests first** (spec test 13, plus the cap):
- `one_call_per_course_batch_stores_each_answer_on_its_claim`
- `a_stored_answer_is_replayed_on_a_re_pull_with_no_model_call`
- `an_id_outside_the_batch_reads_none`
- `a_resolve_call_is_charged_before_the_model`

**Behaviour.** D12, through the one pipeline, under the `mail_resolve` cap.

**Wire shape (T6.1's addendum; `cloudmodel.rs::resolve_mail` is built to it).** The request is
`{course: string, claims: [{uid, index}], candidates: [{id, title, due}]}`: a claim goes as a reference
(`uid` is the message's `mail:<h>`, `index` its place in that message's `claims`, zero-based), since
the service holds the claim text; `due` is a string or null. The reply is `{answers: [{uid, index,
target_id}]}`: one entry per claim the service answered, `target_id` the id of one of the request's
candidates or `null` for "none". The device drops an id outside the batch (reads `None`) and an answer
for a claim it did not ask. A claim with no entry in the reply is asked again at the next slot, so
the service must answer "none" with an explicit `null`, never by leaving the claim out.

**Test, added to the list above:** `the_resolve_reply_is_answers_with_uid_index_and_target_id`: the
reply's only top-level key is `answers`, and each entry's keys are `uid`, `index` and `target_id`.

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

**Behaviour.** D22, read under P22 as amended by PQ11 (2026-10-02): a forwarding counts only while its
`mail_addresses` row is `confirmed`, `active` or `quiet`. The answer is never `quiet` (spec §1: the merged device reads any unknown quiet reason as
"re-connect from settings"). Calendar is untouched, and nothing on the device changes.

### Checkpoint B. Cloud review of T1–T4b

**Agent:** `reviewer` (Opus, high). **Report:** `docs/reports/<date>-email-forwarding-cloud-review.md`.
It reads the cloud diff against spec §4.6–§4.10, §5.1, §6.3, §7's retention rows, D2, D3, D8, D9,
D14, D17, D22, D23 and P5–P9, P21 and P22. It reads in particular:
- the SNS verification and the binding rules, with the PQ2 interop record, and PQ9 put to Quinn as a
  spec amendment (the Gmail binding and the Workspace envelope) before the sub-branch merges;
- that no text lands in a table or a log;
- that every final path deletes its object;
- that charge and budget always run before the model;
- the per-route authentication and every `config.toml` section;
- PQ10 put to Quinn: D9's notification rows (T3a.2) landed from real captures, or the model path
  accepted until the pilot;
- PQ11 put to Quinn as a P22 amendment: which forwarding statuses stop the Gmail OAuth read, with
  `pending` and `blocked` the open cases.

Fixes go back to `cloud-engineer`. The cloud sub-branch merges into the lane when no Critical or
Important finding is open.

**Status (2026-10-04): closed at `0aea395`.** Re-read dated 2026-10-04: the template path is sound and
deterministic, and nothing un-sampled is guessed (Blackboard's two rows come from 23 real captures; Canvas,
zyBooks and due-date-changed stay on the model path). PQ10 is ruled (�4). No cloud-review report file
exists for this checkpoint; its rulings and minors are recorded in �4 and the decisions log (�12).

### T14. Campus rows and the wizard's marker (`scaffold.rs`, `onboarding.rs`)

**Agent:** `mechanical` (Sonnet, low). **Why:** two struct fields and one marker, fully fixed here.
**Shrunk 2026-10-02 (BW, PS).** The rows carry only their mail fields, the provider and the domain.
D7's `terms` field is dropped, because the backfill window is a fixed 120 days (§4).
**After:** T19, which is in every lane worktree (§9.5). **Files:** `app/src/scaffold.rs`,
`app/src/onboarding.rs`, `app/tests/scaffold.rs`, `app/tests/onboarding.rs`, and the four `Curated`
literals in `app/tests/{grades,scheduler}.rs`. Those literals gain the two fields and no assertion
changes. **Before it:** the disjointness check is repeated for `p3-registrar`, which adds fields to
`Curated` in `scaffold.rs`. T14's two fields go after every field `main`'s `Curated` has, and the
registrar (a draft Pilot PR, expected to merge later) rebases onto them (§10, R2). **UA's values:**
the main session supplies UA's provider and student mail domain from the school's public pages, and
Quinn confirms them (a queue item). No calendar is read. Kentucky's row is `other`, with no domain.
**Tests first:**
- `every_curated_row_names_a_known_mail_provider`
- `a_mail_domain_is_set_only_for_m365_or_google`
- `the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt`

**Behaviour.**
- `Curated` gains `mail_provider` (`m365`, `google` or `other`) and `mail_domain` (an optional
  string), per §4.1. They are data in the curated table, never code: no code path branches on a
  school (T11.3's source test).
- **The fallback** for a school with no curated row: the setup asks once for the school address,
  and the service reads its domain's MX (T4.2's
  `provider_reads_mx_…`, T11.3's `an_uncurated_school_asks_once_…`, §13 Q4).
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
- `no_capability_names_a_mail_window_or_a_remote_url` (33; Quinn's IPC ruling 2026-10-02: fails if any
  `app/capabilities/*.json` names a mail window label or remote URL)
- `one_mail_window_at_a_time` (33)
- `the_popup_is_filtered_opens_nothing_and_closes_its_window` (S1b; T9.2 review): reads `src/mail.rs`,
  since no window opens in a test: the popup's builder filters every navigation through `popup_allowed`
  and denies its own popups, and its `WindowCloseRequested` destroys the Tauri window
- `the_runner_refuses_a_page_off_the_providers_origins` (33, a synthetic SSO origin)
- `the_runner_runs_only_compiled_in_scripts` (D19)
- `the_sign_in_detector_reads_m365_and_gmail_titles`
- `no_os_input_and_no_debugging_port_in_mail_sources` (34): `SendInput`, `keybd_event`, `mouse_event`,
  `SetCursorPos` and `remote-debugging` appear nowhere in `app/src/mail*.rs` or `app/assets/mail/**`.

**Behaviour.** D5, and §6.2's window pieces, on T0's recorded routes: the one label `mail`;
`open_visible`; hidden or off-screen as S2 decided; stepping aside; coming back centred and focused;
the popup per S1b; the protocol prompt per S3; the result channel per S1a, with a timeout.

**Outlook's origins** include `outlook.cloud.microsoft` (`outlook.office.com` redirects there, S1c), besides
`outlook.office.com` and the sign-in origins.

**Live checks for T11 and T18c** (the window half has not run live; T9.2 review): Gmail's popup
closes itself after the phone tap and `popup_open` turns false, so a later popup opens; a navigation
in the popup off the provider's hosts is cancelled (it stays on the page it was on).

**Live checks for T11 and T18c, Microsoft 365 mailbox scripts** (T10.2b; the synthetic page models the
spikes' selectors, it is not Outlook): (1) `search.js` types `received>=<day> NOT folder:"Junk Email" NOT
folder:"Deleted Items" NOT folder:"Sent Items" NOT folder:"Drafts"`; the real search box must read that
syntax and leave out all four folders, and `aria-setsize` must be the result count. The last two terms are
T10.3b's review fix, to match the Gmail query's `-in:sent -in:drafts` (T10.3b's ruling: Sent and Drafts are
not mail received): Exchange gives a Sent Items row a received time, so without them recover would forward
the student's own replies, and any `Knowlu <nonce>` copy a failed cleanup left in Sent Items, to Knowlu as
"missing". Spec §4.2 step 5's "excluding Junk Email and Deleted Items" names the two folders a
received-mail search must leave out of the mailbox's own folders; Sent Items and Drafts are the student's
own mail and follow the same rule (test `m365_search_leaves_out_sent_items_and_drafts_which_are_not_mail_received`). (2) `list.js` reads only the rows drawn: the routine (T11.2) must
scroll or narrow the virtualised list to reach the rest, and `forward.js` finds a row only among the rows
drawn. (3) `forward.js` finds Forward's dropdown (a button named Forward with `aria-haspopup`), the menu
item "Forward as attachment", the To, subject and Send controls by their names, and sends the subject
exactly `Knowlu <nonce>`; `sent_cleanup.js` deletes only a Sent Items row whose subject is exactly that
and whose recipients are Knowlu's address alone, and only while Sent Items is the selected folder.
(4) The result `{result: "refused", reason}` is the "first send failure" the step machine stops on.
`received_at` is the page's own reading of the time element's title (its locale, the system's zone).

**Live checks for T11 and T18c, Gmail forwarding scripts** (T10.3a; the synthetic page models the spikes'
selectors, it is not Gmail; the folder is `gmail/`, the name `runs_for` and the D19 test already enforce, not
the `google/` this plan first wrote): (1) the section is found by its `input[type=button]` named "Add a
forwarding address", the radios by their label text ("Disable forwarding", "Forward a copy of incoming mail
to"), the address list and the "keep Gmail's copy in the Inbox" list as the two `select`s beside the second
radio, and Save by its text or value "Save Changes"; none of that is verified live. (2) The dialog's email
field is `input[type=email]` inside `[role=dialog]`; `forwarding_add.js` ends after Next (and Proceed if
asked) and the host runs `forwarding_add_done.js` once the popup has closed; the "confirmation link sent"
dialog is found by the words confirmation and sent, its OK by value. (3) "Remove forwarding address" is a
guess at the control's name and that it acts on the list's selected address, with a confirm dialog naming a
removal: `forwarding_remove_address.js` selects ours, checks it is selected, and reads the list back after
a round trip, but the real button needs one live run on a scratch mailbox before Disconnect uses it.
Until that run has passed, `forwarding_remove_address.js` clicks nothing while forwarding is saved to an
address other than ours and returns `{result: "needs you", reason: "forwarding goes to another address"}`
(test `gmail_remove_address_does_nothing_while_forwarding_goes_to_another_address`). The live run is a
gate, not a note: T11.4's `mail_disconnect_gmail_removal_waits_for_the_live_remove_check` and T15.2's
`turn_off_email_names_a_forwarding_address_it_left_for_the_student` pin it (the live run itself is a T18c
step, and the guard is lifted only in a commit that records its result).
(4) A script that must reload returns `{result: "reloading"}` after `location.reload()`; whether its
result still reaches the host before the navigation must be seen live, and a host that gets no result
after a navigation must treat it as "reloading": T11.3 re-runs the step once with `reloaded: true`, and a
second miss is `not found` and the step's guided fallback.

**Live checks for T11 and T18c, Gmail mailbox scripts** (T10.3b; the synthetic page models the spikes'
selectors, it is not Gmail): (1) `search.js` types `after:<yyyy/mm/dd> -in:spam -in:trash -in:sent
-in:drafts` and presses Enter; the last two terms are this task's addition to §4.3 step 7's query (Gmail's
search covers Sent and Drafts, which are the student's own mail and are never received, so recover would
forward them to Knowlu as "missing"); the real box must read all four and the date form, and the count is
the pager's "1–50 of 123" total (a "many" or an estimate is `not found` `result count`). (2) `list.js`
reads a page of `tr[role=row]` in `table[role=grid]`: sender from the last `span[email]`'s attribute,
subject from `.bog`, time from the `[title]` that holds a clock time, and each row's checkbox is
`[role=checkbox]`; the Newest/Newer/Older/Oldest buttons are found by `aria-label`, and "Oldest" is a guess
at the real pager's name (the routine, T11.2, reads the oldest page with `go: "oldest"`, then `"newer"`; a
missing "Oldest" on a multi-page result is `not found` `pager` and the step's guided fallback). Gmail lists
conversations, not messages: a row with several senders is read as its newest and `thread: true`. This is
a named gate (T18c, "Gmail conversation rows"; T11.2's `a_gmail_thread_row_is_opened_and_every_message_in_it_is_fingerprinted`),
not a note. **Superseded in part (Quinn, 2026-10-02, T11.2):** `list.js` now reads the count and the
routine opens a thread row and fingerprints each message, so (a) and (b) below describe what T10.3b
built, not what the routine does. Three consequences: (a) `list.js` reads a conversation's newest sender and time only, so
reconcile never fingerprints its older messages and a missing older message is never recovered; and
`thread` is true only when the row shows two or more different senders, so a conversation of several
messages from one sender reads `thread: false` (the live run must show whether the row carries a count to
read, "Name (3)", and `list.js` must then read it); (b) if "Forward as attachment" attaches every
message of a conversation row, `forward.js` refuses the page ("attachments do not match the rows"), and
Gmail backfill falls to guided on a page that holds a thread; (c) `search.js` returns the pager's
total, which on Gmail is a **conversation** count, as the batch's `expected`, so on Gmail `expected` is
a conversation count and the progress view's "N of M" compares conversations handled against it, never
messages sent. (3) `forward.js` ticks the wanted rows, then More →
"Forward as attachment" (the message-list toolbar's More), and reads the attachment chips (`aria-label`
starting "Attachment:", the size in the text beside the name, "12K" or "1.2M"); it refuses to send over
`max_bytes` (default 25,000,000, the sum of the sizes shown) or when Gmail offers Google Drive links, and the
caller halves the rows on `over 25 MB`. Whether Gmail counts the base64 overhead against 25 MB (so that the
real ceiling on the shown sizes is nearer 18 MB) is unknown; the routine may pass a lower `max_bytes`. The
To field, Subject field and Send are found by `aria-label` ("To recipients", "Subject", "Send …"); the
subject is exactly `Knowlu <nonce>`. (4) A non-delivery report is looked for among the rows the page lists
(a row with the nonce and a delivery-failure phrase); Gmail's bounce lands in the Inbox as "Delivery Status
Notification (Failure)" and the page left open after a send may not list the Inbox, so the routine (T11.2)
must also look at the Inbox between sends. `result: "refused"` is the "first send failure" the step machine
stops on. (5) `sent_cleanup.js` runs only at `#sent`, takes one row at a time (Gmail draws the list again
after every Delete) and deletes a row only when its subject is exactly `Knowlu <nonce>` and its one
recipient (the `span[email]` list of a Sent row) is Knowlu's address; Gmail's Delete moves the copy to Trash
(30 days), which the privacy copy must say.

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
- `the_backfill_window_starts_120_days_before_the_setup_day` (BW; replaces D7's term-start default):
  `BACKFILL_DAYS = 120` is one constant, the same for every school, curated or not.
- `a_resumed_backfill_keeps_its_first_window_start` (BW): a resume reads `backfill_since` and never
  recomputes it from a later day.
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

#### T11.2. Reconcile and recover (`app/src/mail.rs`, in `app/src/mail/recover.rs`)

**Built** (2026-10-02). **Tests first** (spec §4.11 steps 3–4 and test 32's pace half, as plan tests, in
`app/tests/mail.rs`):
- `a_listed_message_is_held_when_its_hmac_matches_within_fifteen_minutes`
- `nothing_about_an_unheld_message_is_sent_to_the_service`
- `recover_forwards_oldest_first_and_at_most_30_a_slot_when_no_backfill_is_open`
- `an_open_backfill_ignores_the_slot_cap_and_keeps_microsofts_pace`
- `the_window_is_three_days_or_back_to_lapsed_since_at_most_30_or_to_when_last_known_on`
- `the_batch_closes_with_the_count_sent_and_the_sent_copies_are_cleaned`
- `a_gmail_thread_row_is_opened_and_every_message_in_it_is_fingerprinted` (replaces
  `a_gmail_thread_row_is_left_for_guided_never_sent`; Quinn's ruling below)
- `on_gmail_expected_and_the_progress_count_are_conversations`
- `a_microsoft_window_too_large_to_draw_is_narrowed_and_one_day_too_large_is_left` (T10.2b's live check
  (2): the routine narrows the virtualised list), `a_guided_mailbox_runs_nothing`

**Gmail conversation rows: open each thread** (Quinn, 2026-10-02; replaces the T10.3b review's "left for
guided" behaviour, which this section carried until now). For a list row that shows a message count
(`list.js` returns `thread: true` and `messages`: the count beside the sender names, or two or more
senders), the hidden window **opens the conversation** (`gmail/thread.js`, new) and fingerprints **every
message in it**: sender is the address from `span[email]`, received time the time element's `title`
(full date and time), subject the conversation's (a later message is also tried as `Re: <subject>`). The
rows are opened only inside the window. A conversation whose messages are all held is done. One with a
missing message is forwarded **alone** through `forward.js`, which now takes a per-row `messages` count and
expects that many attachments; the batch's `sent` counts each message in it. If Gmail attaches fewer
(T18c's gate (2)) `forward.js` refuses ("attachments do not match the rows"), the draft is discarded and
the conversation is **left** (counted, never named, never retried); the same for a conversation that will
not draw whole ("not every message drawn"). The batch's `expected` is still the pager's conversation
total and progress compares conversations handled against it, while `sent` is messages.

**Microsoft 365's list is virtualised** (T10.2b's live check (2)): `search.js` gains an optional `until`
(`received<until`, a day), and the routine searches the whole window first, then halves a span the list
does not draw whole until it does, down to one day; a day still too large is sent as far as it draws and
the rest counted as left. Rows go oldest first within a span, and spans run oldest first.

**Rulings in the build.** (1) `until` is 15 minutes before the pass: mail that has just arrived is still on
the live path and would read as missing. (2) The saved session's window reaches back to when forwarding
was last known on at most as far as the 120-day backfill window, which is as far as the service lets
reconcile and recover go. (3) A listed row whose time the page would not read cannot be fingerprinted and
is left, never forwarded. (4) The Gmail Inbox look for a non-delivery report between sends (T10.3b live
check (4)) is not done: `forward.js` reads only the page it is given and checks its rows after its own
guards, so it cannot probe an Inbox; a bounce is seen on the next pass's pages and by the canary. **Review (2026-10-02): this drop conflicts with
spec §4.2 step 5 and T10.3b's note (4), and no ruling covers it; it is Quinn's to rule, recorded as the T18c
gate "Gmail non-delivery report between sends" with options (a) accept and (b) build the Inbox look.** (4a) A
message the mailbox itself sent (a reply) is never judged missing: `thread.js` flags it `own: true` from the
account chip, the routine skips it, and a conversation whose only unheld messages are the student's own is
held; a conversation with a missing message is still forwarded whole, replies included (Gmail attaches every
message), which puts the student's own outgoing mail on the wire: **QUINN: does the privacy page need a line
for that, or should the forward be limited to received messages (T18c gate (2) decides what is possible)?** (5)
`expected` is the search count in both phases, and a recover batch opens at its first send, a backfill
batch before the list.

#### T11.3. The setup runner (`app/src/mail.rs`)

**Built** (2026-10-02), in `app/src/mail/{setup,runner,backfill,host,live}.rs`. **Rulings in the build.** (1) The
open-backfill marker is a mailbox id in `mail.json`'s `open_backfills`, written only while one is open (the
field is skipped when empty, so a file with none is byte-for-byte spec §5.4's shape); the spec names no other
store and the service's address view says nothing of a batch being whole. Cost if wrong: one line moves to a
sibling file. (2) A closed sign-in window revokes the address minted for it and deletes its profile ("setup
cancelled, nothing changed"); a Stop mid-backfill clears the marker, a quit leaves it. (3) A guided step's
"I've done this" runs a canary check for forwarding steps, the server's confirm for the Gmail address, and
passes the guided backfill unchecked (no route counts a hand-forwarded backfill). (4) Retry after a blocked
canary that arrives marks the backfill open for the next launch rather than reopening the window. (5) The
pace's day count restarts at 0 on a resume after a restart on the same UTC day (nothing stores it). (6)
`Job` gained `watch` (counts, and a stop) and `Stop::Stopped`; `Place` gained `Forwarding`. **Live checks:** the
M365 forwarding page is opened at `outlook.office.com/mail/options/mail/forwarding` (the path is from memory,
not from a spike); the Gmail window title's address for a Workspace mailbox; the popup wait (`popup_open` turns
false after the tap).

**Tests first** (§4.1–§4.5, D6, D7 as BW amends it, D15 and D16, as plan tests):
- `one_setup_at_a_time`
- `a_closed_sign_in_window_is_setup_cancelled_and_nothing_changed`
- `stop_ends_after_the_current_step_and_names_what_was_done`
- `a_canary_not_back_in_three_minutes_leaves_blocked_with_retry_and_offers_what_remains` (D16)
- `an_uncurated_school_asks_once_for_the_address_and_the_service_decides_the_provider` (§4.1)
- `no_date_is_asked_and_every_school_reads_the_same_120_days` (BW; replaces D7's ask)
- `the_mail_code_names_no_school` (PS): a source test. No curated campus id and no mail domain
  literal appears in `app/src/mail*.rs` or `app/assets/mail/**`.
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
- `mail_disconnect_gmail_removal_waits_for_the_live_remove_check` (T10.3a review, R4): a Gmail Disconnect
  that gets `{result: "needs you"}` from `forwarding_remove_address.js` leaves the address in Gmail, says
  so, and does not retry or fall back to a click; the step ends as a needs-you, never as done.

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

#### T15.0. The read model carries a mail card's sender, date and quote (added 2026-10-03 by the controller)

An engine task, not one of the five `console-ui` dispatches. The controller logged it as a guess while
Quinn was away, because D21, §5.2 and §6.2 put the card's `from`, `mail_date` and `evidence` on the
deck and no task carried them to the read model. **Files:** `engine/src/surface.rs`,
`docs/surface/anatomy.md`. `surface::Card` gains three optional strings. Each is read as a literal
from the card note's key of the same name (T6.2's `source_lines`) and left out of the JSON when
absent. `surface_oracle.rs` runs unchanged, and its three references stay byte-identical.
This changes `surface`'s output, against §6.1's last bullet ("**Unchanged:** … `surface`"). The
departure was signed by Quinn on 2026-10-04 (G2/Q2); §9.5's T15.0 row is closed.
**Tests first:** `a_mail_card_carries_its_from_mail_date_and_evidence_as_literals` and
`a_card_without_mail_fields_serializes_exactly_as_before`.

#### T15.1. The progress view (§4.5)

**Tests first:**
- `the_progress_view_shows_one_row_per_step_with_its_state`
- `needs_you_names_its_reason_and_offers_its_answers`
- `the_backfill_row_names_its_start_date_and_asks_nothing` (BW; replaces D7's confirm). The copy is
  this plan's ("mail from the last 120 days, since <date>"), not signed text.
- `leave_closes_the_panel_and_settings_carries_the_progress`
- `stop_invokes_mail_setup_stop_and_shows_what_was_done`
- `backfill_shows_n_of_m_and_a_time_left`
- `the_consoles_first_open_starts_school_mail_setup_when_offer_pending` (D6, §13 Q9)

#### T15.2. Settings → Email (§4.12, §4.13, D15)

**Tests first:**
- `settings_email_lists_each_mailbox_with_its_state_and_its_address_with_copy`
- `resume_rotate_and_the_saved_sign_in_switch_invoke_their_commands`
- `turn_off_email_is_two_steps_with_the_confirm_text`: §4.12's sentence, word for word.
- `turn_off_email_names_a_forwarding_address_it_left_for_the_student`: when Gmail's address removal
  returned needs-you, Settings says Knowlu's forwarding address is still in the student's Gmail list and
  names where to remove it by hand (T10.3a review, R4).
- `delete_my_data_confirm_adds_the_forwarding_sentence`: §4.13's sentence, word for word.
- `a_failed_profile_delete_is_named_until_it_succeeds`
- `the_google_row_and_the_email_row_sit_side_by_side` (§9)
- `an_unbound_reason_shows_amber_with_its_count_and_rotate_only_when_suggested` (the unbound display,
  Quinn 2026-10-03, §4 PQ12): a row whose `unbound_reason` is set shows `unbound_note` and
  `unbound_since_live` in amber; Rotate is offered from that line only when `suggest_rotate` is true.

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
- **`cloud/supabase/README.md`:** the AWS runbook (§5.1), with the secrets by name only. Before the
  subscribe step it sets the topic's `SignatureVersion` attribute to 2 and reads it back, as T18a
  does (§9.5).
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
   *(2026-10-03, at T17: S6 has not run. Per §6 ("S6 blocks T17's merge of the pace numbers, not the
   code"), the code was merged and `MAX_PER_MINUTE` and `MAX_PER_DAY` in `app/src/mail_steps.rs` say
   "provisional, pending S6" in their doc comments; §9.5 records it. T18c keeps the school backfill
   under 600 until S6 confirms them.)*
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
  - Set the topic to sign with SHA-256 before subscribing, since a topic signs with version 1 by
    default and `verifySns` refuses it (§9.5):
    `aws sns set-topic-attributes --topic-arn <MAIL_SNS_TOPIC_ARN> --attribute-name SignatureVersion --attribute-value 2`,
    then read it back with `aws sns get-topic-attributes`. If this step is skipped, the
    SubscriptionConfirmation gets a bare 403 and the subscription stays PendingConfirmation, with no log.
  - Point SNS's HTTPS subscription at `mail-inbound` and confirm it.
  - Smoke by OTP session (at most two OTPs an hour): a canary round trip, one synthetic forward, and
    the controller's count-only query.
  - **PQ9's two probes** (2026-10-02). Each uses a founder-owned test mailbox and records header facts
    only, as S0 did.
    - *Workspace relay:* a message sent through a Workspace SMTP relay tenant the founder controls, with
      a `+caf_=` MAIL FROM at a Google-hosted test domain naming a scratch mailbox. Record whether SES
      passes SPF and whether it binds. Binding is the accepted residual risk; the record sizes it.
    - *Workspace relay on a gateway domain* (PQ12, 2026-10-03): the same probe with the `+caf_=` MAIL
      FROM at a test domain whose MX is a filtering gateway in front of Google (`providerOf` reads
      `other`), since PQ12 (a) binds such a domain too. Record SES's SPF verdict and whether it binds.
    - *Intra-school live and the unbound reason* (the unbound display, 2026-10-03): with a canary
      bound, two forwards that do not bind (for M365, case 4's intra-school message if EOP does not
      SRS-rewrite it) give `GET /addresses` the `skipped` reason and no Rotate suggestion; a bound live
      forward clears it. Count-only, as every staging read.
    - *Blackboard DKIM alignment* (PQ10, 2026-10-04): all 23 captures read `dmarc=pass` with
      `header.from=blackboard.com`, but the DKIM `d=` was not preserved by forward-as-attachment. On a
      real Blackboard notification, check live that the signing domain aligns with the From domain, so
      the Blackboard rows' `aligned` gate holds on live mail.
    - *M365 second tenant:* a message from a second Microsoft tenant (S6's test tenant), with the school
      domain's SRS-shaped MAIL FROM and a hand-written ForwardingLoop naming a scratch mailbox and
      tenant. Record SES's SPF verdict, whether it binds, and, on the probe and on a real forward, the
      topmost `microsoft.com` ARC-Authentication-Results' fields (`smtp.rcpttodomain=` above all). If
      the real forward's names the hint's domain and the probe's does not, PQ9's third check becomes a
      `cloud-engineer` task.
- **T18b. The pin check** (spec test 17). About twenty synthetic messages go through `mail_claim` and
  `mail_resolve` on staging, and their answers are recorded in
  `docs/reports/<date>-email-forwarding-pin-check.md`. Each pin is then confirmed, or re-pinned by a
  one-statement migration from `cloud-engineer` (P8).
  *2026-10-05 (Quinn):* `mail_claim`'s `max_tokens` re-pinned 640 → 1024 before the pin check
  (`20261005000100_mail_claim_max_tokens.sql`, P8's one statement): five claims at the prompt's limits
  need ~690+ tokens, and a `max_tokens` stop drops every claim. `mail_resolve` keeps 640.
- **T18c. The live proof**, spec §12.6 steps 1–8 as written. It runs on a dev build of the lane, with a
  scratch profile and founder-owned mailboxes, and Quinn at the machine for each sign-in and phone
  tap. Until S6 confirms the pace, the school backfill stays under 600. Afterwards, clean up per the
  standing rule and restore the school mailbox's forwarding.
  **Gate, Gmail conversation rows** (T10.3b review; a result recorded in the proof record, and T11.2's
  `a_gmail_thread_row_is_opened_and_every_message_in_it_is_fingerprinted` stays until a commit records it): on a scratch Gmail
  mailbox holding a conversation of several messages from one sender and one of several senders, see
  live (1) what the list row shows (a count beside the name, several `span[email]`), and that `thread`
  is true for both; (2) what "Forward as attachment" attaches for a conversation row (every message or
  the newest), and whether `forward.js` then refuses; (3) that the pager's total is a conversation count.
  Backfill on Gmail is not called proven until this gate has a recorded result.
  **Gate, Gmail own replies** (T11.2 review): `thread.js` flags a message the signed-in account sent
  (`own: true`) by the address in the account chip's label ("Google Account: <name> (<address>)"); the
  routine never judges such a message missing. On the scratch Gmail mailbox, with a conversation the
  student has replied to, see live (1) that the chip's `aria-label` has that shape (else every conversation
  row is left, `control: "account"`, never forwarded); (2) that the student's reply reads `own: true`,
  also from a send-as alias (an alias is not recognised and would read as received: if it matters,
  Quinn's call); (3) that "Forward as attachment" on such a conversation attaches the reply too, which
  puts the student's own mail on the wire (see QUINN below).
  **Gate, Gmail non-delivery report between sends** (T11.2 review; **QUINN's ruling pending**): spec §4.2
  step 5 and risk 15 stop at the first non-delivery report, and T10.3b's note (4) said the routine would
  look at the Inbox between sends. T11.2 does not (`forward.js` cannot probe an Inbox: it reads the page
  it is given), so a bounce is seen only on the next pass's pages and by the canary, and a backfill could
  send up to 600 bounced forwards in a day. Quinn rules: (a) accept that, with this gate recording a
  live bounce (rotate the address, send one forward, see where the report lands and whether a
  nonce-bearing row exists), or (b) the routine opens `Place::Inbox` every N sends and stops on a row
  carrying the batch nonce and a delivery-failure phrase (a new `gmail/bounce.js`, then `refusal()`).
  - Step 4's checklist names **"POST answered 3xx"**: if `mail-inbound` logs `gmail confirmation
    refused (post_redirect)`, Google answered the confirm POST with a Post/Redirect/Get. The server-side
    confirm refuses that today, since §4.6 step 5 rules only on the page's redirect. Quinn decides from
    the proof whether a 3xx after the POST counts as completion; changing it is a `cloud-engineer` task.
- **T18d. The PR.** CI green and Quinn's word to merge; the merge carries H0. No release is tagged
  before privacy bump #1 (D18). The controller's HANDOFF milestone update follows.

## 6. Order, parallelism and checkpoints

**Per worktree.** One implementer at a time in each; the four run in parallel (P3).

| Worktree | Order |
|---|---|
| `main` (T19's branch) | T19 → T13a → R19 → PR merged on Quinn's word → `main` merged into the lane and into `email-app` |
| `email-engine` | H1a → T5.1 → T5.2 → T5.3 → T5.4 → (PQ5) T8 → T8b → A's first fix round (`968d86d`) → (F3) T8c → (PQ4) T5.5 → A's second fix round (all built, HEAD `4194817`) → Checkpoint A passed (2026-10-02) → (PQ8 ii) T5.6 → T6.1, while contract-reviewer reads T5.6 → that read's fix loop (the T6.3 amendment; PQ13, then its fix if (b)) → H1b → T6.2 → T6.3 (PQ7 answered) → T6.4 → H1c → T7 → H2 |
| `email-cloud` | S0 (passed) → T1.1 → T1.2 (both built, with the old column names) → T1.3 (PQ6 answered; unblocked; renames the two columns first) → T2a → T2b → T2c → T2d (PQ2 answered) → interop check → T2e → T2f → T2g.1 → T2g.2 → T3a (PQ4 answered) → T3b → T3c → T3d (PQ8 (i) declined) → T3e → T4.1 → T4.2 → T4.3 → T4.4 → T4.5 → T4.6 → T4b → T3a.2 (PQ10; only with real captures, else before the pilot) → Checkpoint B → T16 (only if PQ3 keeps it) |
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

**Waits on Quinn, one at a time, each with its context.** Answered: PQ1 and PQ5 (T0.0); the AWS
account, the region (`us-east-2`), DNS and the DMARC sender (S0); PQ2, PQ4, F3, BW and PS
(2026-10-02); Checkpoint A, with T8c's reading and the PQ4 probe, and PQ6 (a), PQ7 (`U`) and PQ8
((ii) only) (2026-10-02, later the same day). T19 merged as #29 (§9.5).
Still open:
- the plan's reading of "pending" in T5.6 (open or snoozed), read with T5.6's addendum;
- PQ13, a new due for a task a pending mail card proposes (T5.6's addendum). It blocked T6.2, but T6.2
  through H2 were built past it, so it then blocked Checkpoint D and T18 (§9.5). Answered (b)
  2026-10-03; the fix is built (`79c1742`, fix round `c99785f`) and contract-reviewer's re-check PASSED
  (2026-10-04). Closed;
- PQ14 (numbered PQ10 on `email-engine` and `email-app` until T17, 2026-10-03), whether and when a claim `mail-resolve` never answers is given up and acked. Answered 2026-10-04: held at most 3 slots, then its message is acked and an unmatched notice opened; built and reviewed (`220cf8d`, fix round `2adf0c9`; `mail/held.rs`, `mail/pass.rs`, T6.4 row in §9.5; review and re-check PASS). Closed;
- the sign-ins at the machine (S1–S3);
- the M365 test tenant (S6); **G3 (Quinn, 2026-10-04): T10.1's pace constants stay provisional and S6 runs
  in the pilot** (§9.5);
- **G4 (Quinn, 2026-10-04): the deletion-refused copy stays as written.** Closed;
- D18's release guard: ruled 2026-10-04 as a release-time check, built (`58340a1`, fix round `5470cd2`;
  `scripts/ci/privacy-guard.ps1`; review and re-check PASS). Closed;
- PQ10 is no longer open: ruled 2026-10-04 (Blackboard rows built; the model path until the pilot for
  the rest; T3a.3 at the pilot), and Checkpoint B is closed (�4);
- Q2, T15.0's optional `surface` fields: **signed by Quinn 2026-10-04 (G2)**; spec §6.1 carries the
  signed marker, §9.5's T15.0 row is closed;
- UA's mail fields (T14);
- the §13 Q3 revisit after S4;
- PQ3 at Checkpoint D;
- the staging secrets, the sign-ins and the merge word at T18.

**Opus starts:** T0, T5.1–T5.6, T8, T8b, T8c, A, T1–T4b, B, T9.1, T9.2, T12, C, T16, T17, D, T18 and
R19. Every other task starts on Sonnet, and a second failure moves it to Opus at high.

**Checkpoints for Quinn** (spec §14):
- **Checkpoint 0:** the T0 report, with S0's DMARC result and header shapes, and S1–S3; S6 and S4 are
  added when they land.
- **R19:** T19's diff and review, before its PR merges.
- **Checkpoint A:** the contract review of T5, T8, T8b and T8c, with the diffs and the plan's reading
  of F3. Passed 2026-10-02; T5.6's addendum goes to Quinn only if it opens a finding or the
  "pending" reading is questioned. It opened one: PQ13.
- **Checkpoint D:** the whole-branch review, before T18, with PQ3 (§9.5). Closed on PQ13 (b) at `c99785f`
  (2026-10-04); PQ3 and T18 remain.
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
- **A release cut too early.** T13a's guard reaches `main` with T19 (D18), and since 2026-10-04
  `scripts/ci/privacy-guard.ps1` fails any release carrying the mail code before bump #1.
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
| D7 as amended 2026-10-02 (BW; the marker at decision 4): the backfill reads a fixed 120 days before setup for every student; no term logic, no date asked; no age cutoff inside the window | T1.1, T4.2, T10.1, T11.3, T15.1, T5.3 | `the_backfill_window_starts_120_days_…`, `a_resumed_backfill_keeps_its_first_window_start`, `a_backfill_batch_reaching_more_than_120_days_…`, `no_date_is_asked_…`, `the_backfill_row_names_its_start_date_…`; test 23 |
| PS (2026-10-02): the provider from a curated row (data, never code), else MX | T14, T4.2, T11.3 | `every_curated_row_names_a_known_mail_provider`, `provider_reads_mx_…`, `the_mail_code_names_no_school` |
| F3 (b) (2026-10-02): a mail amend fills an empty `due`; a future due un-archives an `imported-past` note | T8c, T5.5, T6.2 | T8c's tests; T5.5's F3 tests and the two changed T5.4 assertions; `a_fill_due_card_and_an_unarchive_card_pass_validate_amendment`; Checkpoint A |
| T8c's reading, confirmed at Checkpoint A (2026-10-02): arm (1) keyed on the card, so any dateless task, hand-made too | T8c, T5.5 | `a_mail_card_fills_an_empty_due_…`, `a_date_for_a_hand_made_note_with_no_due_is_an_amend_card_from_null` |
| PQ7 (2026-10-02): a residual `NotAmendable` counts in `U`; §4.8 step 7's line unchanged | T5.5, T6.3 | `the_mail_run_line_is_exact` |
| PQ8 (2026-10-02): (ii) a live `new_item` matching an open mail task card files nothing; (i) declined, the screen keeps "From is the mailbox itself" | T5.6, T6.3, T3d | T5.6's tests; `a_rule_forward_in_a_later_pull_files_no_second_card` (T6.3's view of a card); `mail_from_another_of_the_accounts_mailboxes_is_not_screened_as_own_mail` |
| PQ13 (2026-10-03): (b) due-aware; PQ8 (ii)'s skip holds only for a copy with no known due or a due on the card's due day, another day files its own card | T5.6 | `a_live_new_item_matching_a_pending_mail_task_card_files_nothing` (same day and no due), `a_live_new_item_with_another_due_files_its_own_card` (built `79c1742`) |
| D8: two-stage intake and a separate queue; `seen` written with work after the put; an expired item loses `seen`; `mail-process` on a cron every minute and kicked by intake (the kick is T3d's: T2g.1 shipped without it) | T1.1, T2g.1, T3d, T3e, T4.4 | tests 7 and 16; `an_expired_item_loses_its_seen_row_…`; `a_hand_off_kicks_mail_process_once_with_the_job_token`, `a_failed_or_slow_kick_still_answers_200_and_keeps_the_work` |
| D9: claims cheapest first; templates; a small model with a fixed schema; verbatim evidence; the 0.6 floor | T3a–T3e, T3a.2 | tests 8, 9 and 10; T3a.2's per-row tests. **PQ10 (ruled 2026-10-04, capture now):** `MAIL_TEMPLATES` was empty at `5ce22b4`; T3a.2 (2026-10-04) adds Blackboard's new-gradable-item and due-soon rows. Canvas, zyBooks and any due-date-changed row wait for a real capture |
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
| D18: no `PRIVACY_VERSION` or `site/` change; no release before bump #1 | T13a, D | the guard line on `main` with T19; `release_refuses_mail_code_until_privacy_bump_1`, `the_privacy_guard_blocks_mail_code_at_or_before_the_pre_bump_version` (mechanical, 2026-10-04); Checkpoint D |
| D19: the step scripts compiled in | T9.2, T10.2a–T10.3b | `the_runner_runs_only_compiled_in_scripts` |
| D20: the actor, `created_by: mail`, `mail:` uids, `ingest-seen` | T2c, T5.1, T6.2, T6.3 | `seen_records_mail_uids_…`, `a_message_without_a_message_id_keys_…`; Checkpoint A |
| D21, §13 Q10: sender and date on mail cards, notices and applied changes | T3e, T6.2, T15.0, T15.5 | `…_carries_from_mail_date_and_evidence`, `a_mail_card_carries_its_from_mail_date_and_evidence_as_literals`, `a_mail_card_shows_its_sender_date_and_quote` |
| D22: one mailbox, one path; never `quiet` | T4b | test 16c; P22 |
| D23 and decision 8 as amended 2026-10-02 (PQ6 (a)), §13 Q1: 2,000 judgments and $1.00 per account per 120-day period, opened at the first backfill, keyed on `allowance_opened_at`; reconnecting renews nothing | T1.1, T1.3, T3e | test 11; `charge_allowance_is_one_statement_…`, `the_allowance_period_is_keyed_on_allowance_opened_at_…`, `a_second_backfill_inside_120_days_…`, `a_backfill_120_days_or_more_after_…`; the renamed column test |
| D24: events and schedules stay the student's; nothing written to `commitments/` | T5.3, T5.5 | test 23; `no_action_ever_targets_commitments` |
| D25, §13 Q6, A13: the policy-read gate suspended, as one constant | T19, T13a, R19 | tests 39–41; R19's report; proof step 1's grades half |
| A1–A4 and A6–A11: the signed text in the cloud design, the Gmail spec and VISION | T0.0 (H6) | T0.0's marker check; the docs commit on `main` |
| A5: ruling 10's MVP list and exit gain email and its proof; ruling 9 | T0.0, T13a, T18c | HANDOFF's MVP lane; the proof |
| A12: no forwarded-mail content in telemetry; CLAUDE.md's slot and actor lines | T15.5, T17 (H0) | `no_ui_event_carries_mail_text`; H0's diff |
| A13: ruling 12's five passages, the markers and the review trigger | T0.0, T19, T13a | tests 39–41; HANDOFF's queue line |
| §10's corrections that need no signature (the legal note's Crimson lines, HANDOFF) | T13a, T13b | the edited lines |
| §13 Q2: a canary only after quiet, at least 72 hours apart, never on a day with a Knowlu email | T4.3 | test 6's cadence half; P9 |
| §13 Q3: reconcile once a day, revisited after the 10-08 spike | T11.5, T0.S4 | `reconcile_follows_reconcile_cadence_in_both_modes`; S4's record |
| §13 Q4: the provider from the curated row, else MX (confirmed by PS, 2026-10-02) | T14, T4.2, T11.3 | `provider_reads_mx_…`, `an_uncurated_school_asks_once_…` |
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
| §4.9: which writes are direct and which are cards, row by row | T5.3, T5.4, T5.5, T5.6 | tests 20–24 and 22b–22d; `a_template_receipt_…` and `a_receipt_from_the_model_is_noise` (PQ4); `…_naming_both` (P20) |
| §5.1's claim shape as amended 2026-10-02: a fifth kind `receipt`, template-only (PQ4) | T3a, T3b, T3c, T5.5 | `a_submission_receipt_template_yields_a_receipt_claim`, `a_receipt_claim_from_the_model_is_refused`, `the_mail_1_grammar_offers_no_receipt_kind`, T5.5's receipt tests |
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
| §6.1: the engine (`mailreconcile`, `mail`, `cloudmodel`, `mail-applied`, `ingest`; the rest unchanged **except `surface`**: T15.0 adds `from`, `mail_date` and `evidence` to `surface::Card`, against "Unchanged: `surface`", §9.5, signed by Quinn 2026-10-04) | T5–T8b, H1, H2; T15.0 the exception | Checkpoints A and D; for T15.0, its two tests and `surface_oracle.rs` unchanged |
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
| 5: Gmail's confirm, completed and refused | T2e, T2g.2 | T2e's six confirm tests; T2g.2's four |
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
had no test of its own, or only a combined one. None changes a spec test's assertion. The tests
added on 2026-10-02 pin Quinn's rulings: BW, PS, F3 (T8c and T5.5), PQ4's `receipt` kind, PQ6's
allowance period (T1.3, T3e), PQ7's count (T6.3) and PQ8 ((ii) in T5.6, (i)'s decline in T3d).
Each replaces a D7 test that was never built, or adds a new one. The only built assertions that
move are F3's two T5.4 cases, T5.1's unknown-kind probe (PQ4) and T1.1's column-name test (PQ6, BW)
(§9.5).

### 9.5 Build deviations and open rulings (T5.4's review, 2026-10-01)

Each row departs from T5.1–T5.4 as written, or is an item a later task must not build past. No signed
spec text moves. A row marked **open** waits on Quinn or the planner.

| What | Why | Task, proof | Status |
|---|---|---|---|
| `Action::AmendCard` gains `also`: the other claims a two-value card names, each with its value and sender. T5.1's action list had no such field. | P20's body "names both values, each with its sender"; T6.2 writes them from the action. | T5.4; `two_claims_…_naming_both` | recorded |
| A seventh skip reason, `NotAmendable`, beyond T5.1's six: a card row whose card `approvals::validate_amendment` refuses (the note outside `tasks/` and `courses/`, or a null `from`). The note keeps its value and nothing is filed. | D11 (3); §10 R14's rule for the feed's card. | T5.4; `a_card_the_approvals_executor_would_refuse_…` | **Answered 2026-10-02 (Quinn): (b)**, built as T8c before T5.5, which then plans the card (§4, F3). The residual count is PQ7, **answered 2026-10-02 (Quinn): `U`**, so the signed line is unchanged. T8c's reading (keyed on the card) was confirmed at Checkpoint A. It was open because it drops the feature's main follow-up without a trace: a live date for a mail note that needs one, a hand-made note with no due, or an archived note. Quinn rules before T5.5 and T6: (a) a notice row, or (b) a contract-engineer change to `approvals` that accepts a null `from` for `due`, through Checkpoint A. T6.3 counts every `NotAmendable` on the run line. Which count it lands in (U, or a new slot in §4.8 step 7's signed line) is part of the ruling. |
| P20 is read as: the card proposes the value of the last claim, by uid and claim index, **whose value the note did not hold before the run**, and names every other claim. A group with a member already applied before the run files nothing more. | Read literally, P20 proposes the note's own value when one message names the old date and the new. `apply_amendment` then reads `to` as applied and writes nothing, so the student could never take the other date. The new reading stays deterministic and still names both. Without the applied rule, a run delivered again after its card was approved would propose the other value a second time (§4.8 step 4). | T5.4; `a_two_value_card_proposes_the_last_value_the_note_does_not_hold`, `a_claim_delivered_twice_in_a_two_value_run_…`, `a_message_that_creates_and_moves_an_item_…` | recorded, for the planner. If the planner declines, Quinn rules. |
| A direct set never gives a note outside `tasks/` a due today or later. An aligned backfill's change to an archived `imported-past` mail note takes the card row. No card can amend outside `tasks/`, so it skips as `NotAmendable`. A due still past is set directly. | Spec risk 1, and T5.3's `imported-past` (a due before today). The note would otherwise stay archived with a future due that rank never shows. A batch split over two slots would also write what the same batch planned whole does not (D10). | T5.4; `a_backfill_never_sets_an_archived_mail_note_…` | **Answered 2026-10-02 (Quinn): un-archive** (F3 (b)). On approval, T8c moves the note back to `tasks/`. T5.5 plans the card where T5.4 skipped. |
| The planned-note amendment. A change to a note this plan files (`Target::Planned`) takes the rows of the mail note it will be (`created_by: mail`, no field a human set). `Set`, `AmendCard` and `ProposeDone` name their note by `Place`: `Path` for a vault note, `Planned(by)` for the note the `Create` of claim `by` files earlier in the same plan, which `mail.rs` writes at the path that `Create` gave. The move is its own `Set` keyed by `by`, never folded into the `Create`. `settle` groups a planned note as a vault note, from the due it was filed with. An applied `new_item` whose target is the note it created itself says nothing in a group, as when it was first planned (`New`). | D10 and spec test 18's third case: a batch applied whole reaches the item it creates. A separate `Set` keeps the plan the same whole or split, an archived `imported-past` note included (a fold would un-archive it whole and not split), and keeps §5.2's body line and one action per claim. Without the said rule, a re-plan after a pass cut short between the `Create` and the `Set` would group the creating claim's due with the move and drop the move. | Checkpoint A fix; `a_batch_applied_whole_reaches_the_item_it_creates`, `a_message_that_creates_and_moves_an_item_moves_it_once_on_every_desktop` (replaces T5.4's `a_run_another_desktop_planned_…`, which pinned the dropped move), `a_two_value_run_on_a_note_this_plan_files_makes_one_card`, `a_backfill_never_sets_an_archived_mail_note_…` (now whole = split, action for action) | landed; Checkpoint A re-reads it. **For T5.5:** a `Create` stamps `mail_asof` at its own ordering date, so §4.8 step 5's "not later than `mail_asof`" read per claim would supersede a move in the same message, on the first plan and on a re-plan; the creates-and-moves test pins that it must not. **For T6.2:** an action naming a `Place::Planned` whose `Create` wrote nothing writes nothing and is counted, never retargeted. |
| T8 and T8b are built before T5.5 (§6 orders T5.5 first). | T5.5 waits on PQ4 and the `NotAmendable` ruling; T8 and T8b wait only on PQ5, answered (c), and read no T5.5 code. T8 closes Checkpoint A's judge-once gap: an approved mail card journalled as `agent:approvals` is invisible to `Journal::human_set`, so a later aligned backfill would set over the student's approved value. T6.2 wires no direct set before T8 is in. | Checkpoint A fix round | recorded |
| T8b's card, where P12 is silent. Its file is `approvals/amend-<note stem>-<field>.md` (a `-2` suffix if taken), not `propose_amendment`'s random suffix. Its title is "Blackboard changed <field> for <note title>". It carries no `source_uid`. A value no card can hold on one line (`write::single_line_problem`, on either side) files no card and keeps the note's value, as R14's null rule does. "Already rejected" compares a `due` as a time (`due_changed`) and a `title` as text. | A deterministic name keeps two runs' cards comparable. `existing_by_uid` indexes `archive/`, so an archived card holding the event's uid could shadow the note. `validate_amendment` refuses a line break on either side, and that refusal is recoverable, so such a card would stay pending for good (R9's failure). | T8b; `the_feed_card_passes_validate_amendment_…` (the card's bytes), `a_hand_cleared_due_or_a_feed_cleared_due_…` | recorded |
| One open decision per field counts a `snoozed` card. `ingest::open_amendment` finds a `kind: amend` card for the note and exactly `{field}`, `pending` or `snoozed`, whoever filed it. T8b files nothing beside it, and T6.2's card writer takes the same rule. | `write::find_pending_amendment` reads `pending` alone. A card `defer_over_budget` snoozed (R3), one the student snoozed, or a first-day mail card (R16) was invisible to it, so each slot filed another card for the same field (`amend-<stem>-<field>-2.md`) and rank deferred that one too. `write.rs` is not edited. | Checkpoint A fix; `a_card_the_budget_snoozed_is_still_the_open_decision`, `a_card_the_student_snoozed_is_still_the_open_decision`; T6.2's test 21 on day 1 | recorded |
| T8 writes an approved `title` quoted, as `status` and `importance_reason` are (`QUOTED_FIELDS` gains `title`). | PQ5 (c) makes `title` amendable on a `created_by: blackboard` card. Written bare, a title holding `: ` fails `write_problem`'s re-read, so the card would go back to pending at every `rank`: R9's failure by another door. Only a blackboard card reaches the literal, so every other amend writes exactly as before. | T8; `title_is_amendable_only_on_a_blackboard_card` | recorded |
| The three lane branches were cut from `main` at `25592d4`, not from the lane branch (P3). That base holds T19 (#29, `77fa9bf`) and the events lane (#31). | T0.0 found nothing to merge (progress ledger, 2026-10-01). `j-events` closed unmerged and the events work merged as #31, so R2's four `j-events` rows are already in the base. | T0.0 | recorded |
| PQ4's `receipt` kind. T5.5's files widen to `mod.rs` and `resolve.rs` (`ClaimKind::Receipt`, target resolution). T3a's and T3b's `field: completed` tests are renamed to the kind, and T3c gains a grammar test. | Quinn, 2026-10-02. It amends Q5's four kinds; §5.1 carries the marker. | T3a–T3c, T5.5 | **Answered 2026-10-02 (Quinn)** |
| BW, the fixed 120-day window. `mail_addresses.term_start` is named `backfill_since` and is set once by the first backfill batch. T14 drops `terms`. T10.1's two D7 tests, T11.3's ask test, T15.1's confirm test and T14's terms test are replaced by the BW tests. None of them was built, so no built assertion moves. T15.1 keeps D7's "the student sees the date" as a row that only shows it. | Quinn, 2026-10-02, amending signed decision 4 and D7; the marker is at decision 4. | T1.1, T4.2, T10.1, T11.3, T14, T15.1 | **Answered 2026-10-02 (Quinn)**; the allowance's key is PQ6, answered (a) the same day. T1.1 was built with `term_start`; T1.3 renames it (the PQ6 row below) |
| The only built assertions that move are two T5.4 tests' undated, hand-made and `imported-past` cases (T5.5), and one T5.1 test's unknown-kind probe: `a_claim_in_the_wire_shape_deserializes_and_recover_reads_as_live` probed with `receipt`, now probes with `completed` and asserts that a `receipt` deserializes. | F3 (b) makes those cards fileable, and PQ4 makes `receipt` a kind, so the old probe would fail on the ruling alone. §5's rule "a test's name may change; its assertion may not" yields only to a ruling. The probe's intent (an unknown kind is refused) is kept. | T5.5; Checkpoint A (finding 3 added the third) | **Answered 2026-10-02 (Quinn)** for all three: the two F3 tests, and the PQ4 probe, approved at Checkpoint A (it probes `completed` and asserts that `receipt` deserializes) |
| T5.5 reads §4.8 step 5's "not later than the note's `mail_asof`" as **earlier than**: a stamp at the claim's own ordering date is its own run's (the same message's `Create` or set, or the same run in an earlier pass) and never supersedes it; claims at one ordering date are the two-value row's. The test is `a_claim_earlier_than_mail_asof_is_superseded`. A run ordered before the stamp is superseded whole, and a claim planned superseded is no two-value member. | Read literally, a re-plan after a pass cut short between a `Create` and its move would supersede the move, which the built creates-and-moves test pins must not happen, and a run split across two drains of one batch would supersede its own second half. | T5.5; `a_claim_earlier_than_…`, `a_message_that_creates_and_moves_…` (unchanged) | recorded, for the planner; if declined, Quinn rules |
| T5.5's rows 7–9. P10's `FEEDS` is `blackboard`, `zybooks`, `vhl`. Row 8 (a backfill template's change to a fed item: nothing, counted superseded) is for `due` and wins over row 6, since the feed's T8b card covers a due the student set there. Row 9's direct write is row 5's: only a mail note. A template's change to a Gmail, events or hand-made note keeps rows 10 and 6, a card. | §4.9 rows 9 and 10 overlap for a template change to a Gmail note. A card is always safe; a direct write over another source's note is spec risk 1's harm. T5.4's ruling already kept direct writes to mail notes. | T5.5; `a_backfill_template_change_to_an_unfed_note_is_direct`, `…_to_a_fed_item_writes_nothing_…`, `feed_creators_…` | recorded, for the planner |
| T5.5's notices. `Action::Notice` gains `about` (`CourseInfo` or `Unmatched`), so T6.3 counts J and U without re-reading claims. A `course_info` notice is titled `<course>: <topic words>` and keyed `mail:<course>:<topic wire word>`. An unmatched notice is titled with D12's text, "Knowlu could not match this to an item". It and a course-less `course_info` notice are keyed by the claim's own uid (`mail:<h>:<n>`), so a re-delivered message keeps one open. A `course_info` with no topic, and a claim whose `course` would break a line, are noise. A claim still ambiguous at plan time is unmatched when it has no course (it can only be answered "none"); with a course it writes nothing (`Skip(Unmatched)`) until `mail-resolve` answers. | §5.2 names only the `mail:<course>:<topic>` and `mail-source:` keys, and §4.8 step 7 counts notices and unmatched apart. Without a key, a pass cut short before its ack would open a second unmatched notice. | T5.5; `course_info_opens_a_notice_…`, `an_unmatched_claim_is_a_notice`, `an_unmatched_receipt_…` | recorded. **T6.4 built (`mail/asks.rs`): resolve, and hold when a claim has no answer or the call fails; PQ14 (answered 2026-10-04) now bounds the hold at 3 slots, built below.** One `mail-resolve` call per course; a claim the service leaves unanswered, or a call that fails, holds its message unacked and it is asked again next slot. A batch-specific refusal (a 4xx other than 401, 402, 403, 408 and 429) skips that course and the next course is still asked (`a_refused_course_is_skipped_and_the_next_is_asked`); 401, 402, 403, 408, 429, 5xx, no network and no time end the asking for the pass. The line names either cause (`mail: could not ask which item (<closed word>); left for the next slot`). **PQ14 built (Quinn, 2026-10-04; `mail/held.rs`, `mail/pass.rs`):** a claim `mail-resolve` never answers (no answer, or its call fails, a 5xx, no network, 401, 402, 403, 408, 429 or a refused course alike) is held at most 3 slots; at the 3rd its message is acked and "Knowlu could not match this to an item" is opened, keyed by the claim's own uid (`mail:<h>:<n>`), by storing "none" on the claim so the plan's own unmatched row writes it. Never silent: the run line adds `mail: gave up asking which item for N claim(s) after 3 slots; each is an unmatched notice`, and U counts the notice. A slot is one pass that asked or tried to ask (a call was made about the claim), counted once however many rounds the pass makes; a claim the asking never reached (an earlier failed call, no time) is held without a count. The count is generated state, `state/mail-resolve-held.json` (claim uid to slots, `ledger::dumps_value`, written beside and renamed over, never a note), so it survives a restart; an answered or no-longer-asked claim drops its entry, and a given-up claim keeps its entry at 3 until its message is acked (or found already recorded), so the give-up survives a failed ack or a crash and a returning message is given up on again unasked (fix round, `a_give_up_stays_marked_until_the_message_is_acked`). Tests: `an_unanswered_claim_is_held_twice_and_given_up_on_the_third_slot`, `an_answer_on_slot_two_resolves_and_leaves_the_other_claims_count`, `the_count_survives_a_restart_and_a_failed_call_counts`, `a_claim_never_asked_this_pass_is_not_counted`, `the_drain_continues_past_a_claim_given_up_on`. **T6.3 built "hold, unacked":** the server returns an undelivered claim on every pull (§4.8, only an ack delivers), so a held message comes back each round and each slot. Within one pass the run keeps the held uids and skips a returning one (no replan, write or count), and a page of only held messages ends the pass; `a_held_message_delivered_in_two_rounds_is_planned_once`, `a_page_of_only_held_messages_ends_the_drain`. **Head-of-line effect, bounded by PQ14 to 3 slots:** a held message stays at the head of the undelivered queue until `mail-resolve` answers (T6.4) or it is acked, so in a closed-batch drain it can block the pages behind it each slot, and its notice (a `course_info` in the same message) is closed and reopened once per slot, a dismissed one returning. PQ14 (built) lifts the head-of-line effect after 3 slots: the claim is given up on and acked. |
| T5.5's F3 cards. `amendable` mirrors T8c: a null `from` for `due` only, and for `due` alone an `archive/` note that is `status: archived` with `archived_reason: imported-past` (`NoteView` gains `archived_reason`, T6.3 fills it). On that note, only a `to` on or after today is planned; an earlier one in a card row is the residual `NotAmendable`, since approving it would only re-date an item that stays archived. | T8c accepts any due there and sets an earlier one in place, so the plan files a subset of what the executor accepts and never a card it refuses. | T5.5; `a_due_ahead_for_an_archived_imported_past_…` | recorded |
| A receipt's card on the vault's first day. `completion::propose_done` files `status: pending` today with no hold, so a receipt card on day 1 shows on day 1, unlike §4.8 step 5's mail cards. | `ProposeDone` carries no `Proposal`, and the plan's T6.2 writes receipts through `propose_done` unchanged. | T6.2 | **open (planner):** hold it (a snooze after the write, or a `propose_done` parameter) or accept it |
| Spec text that still says "term" and is read through the two markers: §1's goal lines, D7, D9's kind list, §4.2 step 5, §4.3 step 7, §4.7 step 4, §4.9's backfill column header, D23 and decision 8. Quinn asked for two markers only. | No other spec edit was made. A reader of D7 or D9 meets the old text and must follow the markers. | T13b (reference docs cite the markers) | recorded. **2026-10-02:** D7, D23, §5.1's two columns and decision 8 now carry dated markers too (PQ6, BW). Still read through the markers: §1's goal lines, D9's kind list, §4.2 step 5, §4.3 step 7, §4.7 step 4, §4.9's column header, §11 risk 9, test 11 and §13 Q1 |
| S0: `us-east-2`; the DMARC gate passed 2026-10-01; real forwards captured 2026-10-02 (`s0-captures.md`). | Header facts for T2d and T2f, listed under T2f. | T0.S0, T2d, T2f, T18a | recorded |
| PQ2's interop corpus. The progress ledger says the raw S0 copies are kept for the check, but `s0-captures.md` says local copies were deleted. | The check needs the raw copies; the header facts alone verify nothing. | T2d; Checkpoint B | **open (controller):** confirm before T2d, and re-capture cases 1, 3, 4 and 6 if the copies are gone |
| Checkpoint A passed. T8c's two arms are keyed on the card's `created_by: mail`, so arm (1) fills any dateless task, hand-made too, and never checks the target's `created_by`. | Quinn confirmed the plan's reading at Checkpoint A. The re-read covered both fix rounds through `4194817` (`fa980a1` and `4194817` added to the list). | T8c, T5.5; Checkpoint A's report | **Answered 2026-10-02 (Quinn)** |
| PQ6 (a). The allowance is per account per 120-day period on `mail_accounts.allowance_opened_at`. T1.1 was built before the answer with `allowance_term_start` and `term_start`; T1.3 renames them to `allowance_opened_at` (`timestamptz`) and `backfill_since` in the still-unapplied `20261002000200_mail.sql`, and updates `the_mail_tables_have_section_5_1s_columns_and_words`. `term_start` is renamed, not dropped: T4.2 and T10.1 read the window start, and T15.1 shows it. The opening rule sits in `charge_allowance`'s one statement, so the period opens at the first backfill charge, not at the batch route. | Quinn, 2026-10-02, amending decision 8 and D23 (markers at decision 8, D23 and §5.1). Renaming in the unapplied file keeps one mail migration; nothing is applied until T18. Opening in the charge keeps T4.2 unchanged; the two moments differ by minutes. | T1.3, T3e; the renamed column test, T1.3's and T3e's allowance tests | **Answered 2026-10-02 (Quinn)**; the column and opening details are the plan's |
| The allowance refund (T1.3's review, 2026-10-02). `refund_allowance(p_account)` is added to the still-unapplied `20261002000200_mail.sql`: one `update` of `mail_accounts` that gives one judgment back only while `allowance_until > now()` and never above 2,000, service-role only, and never opens, extends or re-prices an allowance. T3e calls it on `model failed` alone (F1's rule: a refusal, a truncation or a failed validation reached the model and stays charged) and routes the item through wait-and-retry. Chosen over "a failed backfill item is never counted" because charge-before-call is the hot-loop guard and stays, as F1 kept it for `charge_call`. Unlike `refund_call`, refunds are not counted: the caller is the server's cron worker, so request volume is bounded by the item's retry schedule, and dollars by `enforce_budget`'s raised ceiling. Spec §5.1's RPC list names two; this third is the plan's. | `charge_allowance` charges before the call, as `charge_call` did before F1 (main's `20261002000100_refund_failed_call.sql`, PR #33). A first backfill during a provider outage would spend 2,000 judgments in seconds on calls the model never answered, then trickle at the daily cap for the 120-day period: D23's failure. | T1.3, T3e; `refund_allowance_gives_one_judgment_back_only_while_the_allowance_is_open`, `a_model_failure_on_a_backfill_item_refunds_…`. The function-count pin is 36 here; merging main (PR #33's `refund_call`) moves it to 37, not 36. | recorded, for the planner; T3e must not build past it |
| SNS signature version (T2a's review, 2026-10-02). `verifySns` accepts only `SignatureVersion` 2 (RSA with SHA-256) and refuses version 1 (SHA-1) before any fetch. An SNS topic signs with version 1 by default, and S0's topic was built with the default. So T18a now sets the topic's `SignatureVersion` attribute to 2 and reads it back before the subscribe step, and T13b's runbook carries the same step. `sns.ts` is unchanged. | Without the step, the SubscriptionConfirmation is refused with the same bare 403 as every other refusal and leaves no log line. The subscription stays PendingConfirmation, no mail arrives, and nothing points at the cause. That costs a live deploy, but it fails closed. | T18a, T13b; T2a's version test in `sns_test.ts` | **carried (controller):** T18a's attribute step; T13b's runbook line |
| PQ8 (ii) only. T5.6 is added before T6.1 and T6.2: a live `new_item` matching an open mail task card by normalised title and course files nothing (`AlreadyApplied`). "Open" is read as `pending` or `snoozed`, since a first-day card is snoozed (R16). (i) is declined: T3d keeps "From is the mailbox itself". | Quinn, 2026-10-02. The device rule catches S0 capture 5's rule-forward duplicate without dropping a note the student mails themselves. | T5.6, T3d; T5.6's tests, `mail_from_another_of_the_accounts_mailboxes_is_not_screened_as_own_mail` | **Answered 2026-10-02 (Quinn)**; the "snoozed" reading is the plan's, for Quinn's read with T5.6's addendum. Built `d0abe3e`. The addendum read amended T6.3: the view fills `kind`, and a task card's `title`, `course` and `due` come from its fenced payload. PQ13 (a different due, §4): **answered (b) 2026-10-03 (Quinn), built `79c1742`**; the match now also needs no known due or the card's due day |
| PQ9 (T2f's review, 2026-10-02). On Gmail, check (i) does not name the mailbox (no AMS covers `X-Forwarded-*`), and the `+caf_=` envelope is `@gmail.com` only, so Workspace live mail is unbound. Step 6 aligns a child of the From domain only through the provider's DMARC pass. | Spec §4.6 step 5 cannot be met as written on Gmail; widening to the hint's domain was not signed. | T2f; `a_gmail_forward_binds_by_googles_arc_seal_and_its_caf_return_path`, `a_from_on_a_bare_label_or_public_suffix_never_aligns_with_a_signer_under_it`; Checkpoint B | **Answered 2026-10-02 (Quinn), PQ9 (§4):** the amendment is signed in spec §4.6 step 5; Workspace binds on its own-domain `+caf_=` envelope when the domain's MX is Google's (three new tests); M365 kept as built, no sealed recipient field on record; T18a probes both residual risks. |
| D9's notification rows deferred until real captures exist (T3a's review, 2026-10-02). T3a shipped `MAIL_TEMPLATES` empty: no Blackboard, Canvas or zyBooks notification has been captured, and rows are never guessed. Until T3a.2, every such mail goes to the `mail_claim` model, spends the daily cap and the allowance, and takes §4.9's model rows (in backfill, a card where a template change to a note no feed supplies would write directly). | Departing from D9 as signed is Quinn's call; the implementer ruled alone. | T3a.2; PQ10; Checkpoint B | **ruled 2026-10-04 (Quinn), PQ10 (�4):** Blackboard rows built (T3a.2); the model path is accepted for Canvas, zyBooks and due-date-changed notifications until the pilot; T3a.3 writes their rows from pilot students' real mail. |
| T3e's schema (T3e's review, 2026-10-02). `mail_work.msg_hash`, written by `mail_hand_off`, and a fourth mail RPC, `expire_mail_work` (the work row, its `mail_seen` row and one `expired`, in one transaction; service-role only), in the still-unapplied `20261002000200_mail.sql`. Spec §5.1 lists neither, and T3e's files were `functions/mail-process/**`; `the_mail_tables_have_section_5_1s_columns_and_words` now lists the column. | D8's "an expired item loses its seen row" needs a link from a work row to its seen row once the object is gone; the HMAC is already in `mail_seen`. Nothing is applied until T18. | T3e; the migration tests and `an_expired_item_loses_its_seen_row_…` | **recorded in spec §5.1 (B-minors, 2026-10-02):** a dated marker on the `mail_work` row and the RPC list names `msg_hash` and `expire_mail_work` as built, with `mail_claims_uid` (row below). It records the build, not a Quinn ruling; Checkpoint D reads it |
| An allowance judgment charged over budget is given back (T3e's review). When `charge_allowance` passes and `enforce_budget` then refuses, no call is made, so T3e calls `refund_allowance`. This widens the refund row above ("on `model failed` alone") to "no call was made". A charge that opened the allowance while over budget still opens its 120-day period and its 14 days. A day's-cap charge taken over budget (a live or recover item, or a backfill item past the allowance) is not given back. | D23 and `judge_pipeline.ts`'s rule: an account over budget must not spend its allowance. Without it, about 667 over-budget items times three attempts empty the 2,000 judgments with no call. | T3e; `an_over_budget_backfill_item_on_an_open_allowance_gives_its_judgment_back`. **B-minors (2026-10-02):** the opening cost is closed. A backfill item first reads `enforce_budget` at `MONTHLY_CEILING_USD + ALLOWANCE_USD` (`MailCharges.allowanceCouldServe`); over even that, it is over budget with nothing charged and no allowance opened (`an_account_over_budget_by_more_than_the_allowance_never_opens_it`, `an_account_within_the_allowances_dollars_opens_it_and_is_judged`; `ALLOWANCE_USD` pinned to the SQL's `1.00`). Cost: one more `enforce_budget` read per backfill item, and its refusal upserts the month's `budget_alerts` row as the second read would | **open (planner):** confirm the widening; rule whether an over-budget day's-cap charge is refunded too (by `refund_call`, once main is merged) or the order changes for live items (budget before `charge_call`, as the pipeline does) |
| The attempt is counted before the charge (T3e's review). The model path writes `attempts + 1` and the next UTC day as its wait before any charge, and expires, uncharged, an item that arrives with its attempts spent. | Charge-before-call is the hot-loop guard (D23, F1). Counted only after the call, a store write refused every time after the call (a `mail_claims` check constraint) re-charged and re-called the item at every five-minute lease, up to about 500 calls before the object expired. | T3e; `a_store_failure_after_the_call_charges_and_calls_at_most_three_times` | recorded, for the planner |
| A live or recover item's `model failed` gives its day's-cap charge back by main's F1 `refund_call` (kind `mail_claim`, cap `MAIL_DAILY_CAP.mail_claim`), through `MailCharges.refundDaily`. The call is wired on this base, where the RPC does not exist yet: it fails, refuses and leaves the charge, as before F1. | Main's F1 rule (PR #33): a call the model never answered is given back. Without it, an hour-long outage during forwarding spends one cap charge per arriving item with no answers. | T3e; `a_model_failure_on_a_live_item_refunds_its_daily_cap_charge_and_waits_to_retry` (the fake models main's SQL); **T17:** after merging main, confirm `refund_call(uuid, text, integer)` is applied before T18 and re-run the test against it | recorded; T17 carries the check |
| Recover and reconcile windows start no earlier than the backfill's (T4.2's review, 2026-10-02). `POST /batches` with `phase: recover` and `POST /reconcile` refuse (422) a `since` before `backfill_since`'s UTC midnight, or, while it is unset, before the UTC day less 121 days (the first backfill's own bound). The 180-day recover cap (`mail_seen`'s horizon) and the 181-day reconcile span stay as outer caps beneath that floor. | D7 as amended 2026-10-02: every resume or recover of the backfill reads the same start. Without the floor a recover since 180 days back opened a batch whose mail from before the window was live, charged to the daily cap and the model budget. Every §4.11 window (3 days, `lapsed_since` at most 30 days, last known on) is after setup, so the floor refuses nothing the routine needs. | T4.2; `a_backfill_batch_reaching_more_than_120_days_before_its_opening_is_refused`, `reconcile_answers_only_held_fingerprints_in_the_window_and_the_key` | done. **Open (Quinn):** whether the 180/181-day outer caps should go (the floor makes them bind only on a mailbox set up more than about 60 days ago), and whether a device east of UTC must send the backfill's date, not its local midnight, for a recover that finishes it (the floor is UTC midnight) |
| A `mail-resolve` call's `model failed` gives its day's-cap charge back by main's F1 `refund_call` (kind `mail_resolve`, cap `MAIL_DAILY_CAP.mail_resolve`), through `ResolveCharges.refundDaily` (T4.5's review, 2026-10-02). A refusal or a truncation reached the model and stays charged. As for T3e, the call is wired on this base, where the RPC does not exist yet: it fails, refuses and leaves the charge. | Main's F1 rule (PR #33). Without it, an hour-long provider outage spends the 20-call `mail_resolve` cap on calls the model never answered, and every ambiguous claim that day waits behind T5.5's `Skip(Unmatched)` with nothing shown. | T4.5; `a_model_failure_gives_its_daily_cap_charge_back_and_a_refusal_or_truncation_keeps_it`, `a_daily_cap_refund_is_main_f1s_refund_call_for_mail_resolve_…` (the fake models main's SQL); **T17:** after merging main, confirm `refund_call(uuid, text, integer)` is applied before T18 and accepts kind `mail_resolve`, and re-run both tests against it | recorded; T17 carries the check |
| Account deletion goes on past a failed `work/` object delete (T4.6, 2026-10-02). `purgeMailboxes` (`functions/account/mail.ts`) deletes each mailbox's objects by key, logs a failure by its error class and count only (never a key, an id or a message), then calls `delete_mail_address` regardless. Disconnect's `purgeAddress` (`functions/mail/handler.ts`) does the opposite: it answers 502 before the RPC, so the revoked row stays and a retry finishes. D17 says deletion "calls the same purge" and does not say which way a failure goes. | The account's right to delete wins, as it does for the Google grant (`google_delete.ts`): a deletion held up by S3 would keep every row, not just the object. A straggler is mail text with no row pointing at it, and D2's one-day expiry on `work/` removes it. | T4.6; `a failed object delete never blocks the account's deletion, and logs no key`, `deletion_purges_each_mailbox_by_delete_mail_address_after_deleting_its_objects_by_key` | recorded. **Cost if wrong:** after a deletion whose object deletes failed, the student's forwarded mail text can stay in `work/` for up to about 2 days (S3 runs a one-day expiry rule once a day), with no row left to find it. Checkpoint B confirms the ruling; **T17 and the privacy page:** the deletion paragraph must not say mail text is deleted at once. It says "removed within about 2 days" unless the ruling is reversed |
| The export carries two tables, on fixed column lists (T4.6, 2026-10-02): `mail_addresses` as `MAILBOX_EXPORT_COLUMNS` (id, provider, `mailbox_hint`, status, its dates and counters; not `forward_hint`, `retired_until` or any token column) and every `mail_claims` row held for the account, delivered or not, as `CLAIM_EXPORT_COLUMNS` (all but `account_id`). Left out: `mail_accounts` (the fingerprint key ciphertext, the allowance, the mint times and the `digest` setting), `mail_batches`, `mail_canaries`, `mail_seen` and `mail_work`. | D17 names "its mailboxes … and its queued claims; never the token, its HMAC, a key or a hash". Batches and canaries hold nonce HMACs and service counts, `mail_seen` holds HMACs and `mail_work` holds object keys. A fixed column list (not `select *`) keeps a column added later out of the export until someone names it. | T4.6; `the_export_carries_mailboxes_and_queued_claims_and_never_a_token_key_or_hash` | recorded. **Cost if wrong:** the student's export omits the digest setting, the allowance state, the mailbox's `forward_hint` and the batch and canary history. If a portability reading counts these as the student's data, a later task adds a projected `mail_accounts` row (digest and allowance only) and the batch counts without their nonce HMACs. **Open (Checkpoint D; B-minors, 2026-10-02):** whether `mail_claims` exports delivered rows. D17 and §4.13 say "its queued claims", which does not clearly cover a delivered row (held 7 days); built: every held row, delivered or not, on the same columns. Left as built, since narrowing an export is the change that needs the ruling |
| T4b reads "an active forwarding" (D22) as P22 states it: every status but `revoked` and `off`, so `pending`, `confirmed`, `blocked`, `paused_send` and `lapsed` stop the Gmail OAuth read (T4b's review, 2026-10-02). A Gmail student whose forwarding setup stalls (`blocked`) or is abandoned (`pending`) after P21 set the hint gets `read: 0`, `quiet: false`, no reason, from the next slot: neither path reads that Gmail and the tray stays green. Backfill recovers the gap only if forwarding later becomes active. | P22 is literal; D14 defines `blocked` as mail not reaching Knowlu, so the reading loses items silently (VISION: silence is never ambiguous). | T4b; `P22: a forwarding counts while its row is neither revoked nor off, quiet included` | **Answered 2026-10-02 (Quinn), PQ11 (§4):** the allow-list `confirmed`, `active`, `quiet` (`FORWARDING_COUNTS`); the P22 test moves `pending` and `blocked`; P22 and spec D22 amended. |
| PQ9's Workspace MX clause (B-rulings review, 2026-10-02). `googleHosted` binds a `google` mailbox on its own domain only when the domain's MX reads Google, which is stricter than `hintFits` (an `other` MX fits). A Workspace school behind a Proofpoint, Mimecast or campus-relay gateway passes setup (the canary never reads MX), then every live forward is unbound and counted as a leak; the row goes `quiet` and the periodic canary calls it healthy. With Gmail OAuth on the same address, PQ11 steps the OAuth read aside, so neither path reads it. | The clause is in the controller's record of Quinn's PQ9 answer, but PQ9 never raised MX or gateways, so the consequence was never put to Quinn. Widening without an answer would override that record. | `_shared/mail_bind.ts` `googleHosted`; `a_workspace_caf_envelope_on_a_domain_whose_mx_is_not_google_is_unbound` (`gateway.test`); spec §4.6 step 5 carries the unsigned marker | **Answered 2026-10-03 (Quinn), PQ12 (a) (§4):** `googleHosted` binds on a `google` or `other` MX; `gateway.test` moved to `a_workspace_forward_binds_by_googles_seal_its_own_domain_caf_return_path_and_a_google_or_gateway_mx`, and the refusing test is `a_workspace_caf_envelope_on_a_domain_whose_mx_is_microsoft_missing_or_unanswered_is_unbound`; T18a's gateway-domain probe; spec §4.6 step 5 amended. |
| `mail_hand_off` writes only for a live mailbox (B-minors, 2026-10-02). Its first statement is `perform 1 from mail_addresses where id = p_address and account_id = p_account and status <> 'revoked' for share`; not found answers false, and the caller deletes its put (logged `duplicate or revoked, deleted`). | Checkpoint B (minor): Disconnect revokes, reads the mailbox's `work/` keys, deletes them, then calls `delete_mail_address`. A hand-off landing between the key read and the delete left an object with no row until the bucket's expiry. The share lock makes the revoke wait for a hand-off already past the check, so the key read sees its row. | B-minors; `mail_hand_off_writes_the_seen_row_then_the_work_row_in_one_transaction` (the live check first) | recorded |
| `mail_claims` is unique on `(address_id, uid)` (`mail_claims_uid`, a whole-table index), and `mail-process` queues with `on_conflict=address_id,uid` and `resolution=ignore-duplicates` (B-minors, 2026-10-02). | Checkpoint B (minor): the queue runs before the work row's delete, so a failed delete ran the item again and queued a second row for the same uid (up to three), which the device pulled as duplicates. The re-run's charge is still spent (at most `MAX_ATTEMPTS`, as before); only the duplicate claim is gone. Chosen over one RPC for queue and finish as the smaller change. | B-minors; `mail_claims_holds_one_row_per_mailbox_and_uid`, `a_second_queue_of_one_message_is_a_no_op_on_mail_claims_unique_key` | recorded |
| `l=` must name the whole canonical body (B-minors, 2026-10-02). `verifySignature` answers `permerror body_length` when `l=` differs from the body's canonical length, for a DKIM signature and an AMS alike. | Checkpoint B (minor): with `l=` honoured, text appended after the signed length verified, and would be judged as part of a live `aligned` message. Neither Google nor Microsoft is known to sign with `l=`, so nothing that binds today is lost. | B-minors; `an_l_tag_shorter_than_the_body_is_refused` | recorded. **Cost if wrong:** a sender who signs with `l=` reads unaligned (a live item is then deleted and counted `unauthenticated`) |
| SNS replay bound (B-minors, 2026-10-02). `verifySns` refuses a notification or handshake whose signed `Timestamp` is unreadable or more than `MAX_SNS_AGE_MS` (one hour) before `now`, before the certificate is fetched. `SnsDeps` gains a required `now`; `mail-inbound` passes its own clock. | Checkpoint B (minor): a replayed notification made intake GET a deleted object and answer 500. SNS keeps a message's `Timestamp` across its retries, and an HTTPS delivery policy's retries end within 3,600 seconds, so the hour holds every genuine retry. No MessageId store is added. | B-minors; `a_notification_older_than_an_hour_or_with_no_readable_timestamp_is_refused` | recorded. **Cost if wrong:** a custom delivery policy longer than an hour (AWS's maximum is 3,600 seconds) would lose its last retries |
| A multi-mailbox delivery that fails partway is retried whole (B-minors, 2026-10-02; accepted, not changed). An earlier mailbox's `batchReceived` or its `unbound` or `unauthenticated` count then runs a second time on SNS's retry; `handOff` stays deduped, so no item is queued twice. | Checkpoint B (minor), whose fix offered "accept and record". Idempotent counters would need a per-key record of each count. A doubled `received` can only close a batch early by the device's own count, and a doubled `unbound` overstates a leak by one. | B-minors; no test (accepted) | recorded. **Cost if wrong:** a batch closed early is delivered without its stragglers, which the next reconcile lists as missing and recover re-sends |
| Intra-school M365 forwards and SRS (Checkpoint B's open item, B-minors, 2026-10-02). If EOP does not SRS-rewrite a forward whose sender is on the school's own domain, the envelope is the colleague's own address, check (ii) cannot name the mailbox, and the message is unbound: deleted, counted on the mailbox's `unbound` (the count the Settings row shows), never live, never moving `last_received_at`. | Plan T2f: "If it does not, live intra-school mail is unbound." Binding on that envelope would let any Microsoft tenant's sender bind (risk 10). Pinned so it can never become a silent live item. What it does not do: the canary (sent from Knowlu's own domain, so SRS-rewritten) still binds and keeps the mailbox `active`, so Settings shows green with a rising `unbound` and offers Rotate, which reads as a leak. With saved sign-in on, the daily reconcile lists the missing messages and recover re-sends them (§4.11); with it off, they are lost. | B-minors; `an_intra_school_m365_forward_without_an_srs_envelope_is_unbound`, `an_intra_school_forward_microsoft_did_not_srs_rewrite_is_counted_unbound_never_live` | **Answered 2026-10-03 (Quinn), the unbound display (§4, under PQ12):** an unbound arrival from the hint's domain or under the provider's seal runs `unbound_own_since_live`, so after two with no bound live forward between, an `active` or `quiet` row reads `skipped` ("some forwarded mail could not be verified and was skipped", with the count) and Rotate is not suggested; only foreign mail reads `leaked` and suggests Rotate. T18a still records case 4's envelope and checks the reason live. |
| Rotate clears the unbound reason (PQ12+amber review, 2026-10-03). As first built, Rotate kept both since-live counts and intake ran them for unbound mail through the retired token. A `leaked` row whose student pressed Rotate went on saying "Rotate gives it a new one" with `suggest_rotate` true: spam to the old address kept adding foreign counts in the grace (up to seven days), and after it the counts held until the next bound live forward, days for a quiet mailbox. Each extra Rotate spends one of the six mints in 30 days (`MINT_CAP`) and redoes the forwarding step at the provider. | VISION's honesty commitment: the screen must not keep showing a wrong instruction. The counts describe the old address; once it is retired they are no longer an instruction. The cumulative `unbound` stays as history. No migration: `count_mail_arrival`'s plain `unbound` already counts without running either count. | `functions/mail` `rotate` (`RotateNext` carries both counts as 0; the store writes them in the same PATCH as `retired_token_hmac`); `mail-inbound` `arrive` (a non-current `Found` counts `unbound`); `a_rotate_clears_the_leaked_reason`, `an_unbound_arrival_through_a_retired_token_runs_no_since_live_count`; spec §4.6 step 5 carries the unsigned marker | **open** (built; put to Quinn). Cost if wrong: in the grace, unbound mail through the old address shows no reason, so a second leak of the *new* address is still named but skipped mail still reaching the old one is counted only in the cumulative `unbound`; reverting is two lines and the two tests. |
| T6.2, T6.3, T6.4, H1b, H1c, T7 and H2 were built on `email-engine` (`de36f56` through `5cd68ff`, 2026-10-02) with PQ13 open, past PQ13's "Blocks: T6.2" (§4). No answer or waiver was recorded; the T6.2 review fix's ledger line (`157ab49`) named the gate and the work went on. The branch was then merged into `email-app` (`3e39742`, 2026-10-03). Until PQ13 is answered, a live `new_item` with another due for a task a pending or snoozed mail card proposes plans `[Skip(AlreadyApplied)]`, and T6.3 acks it: the date is lost with no card and no notice (`a_live_new_item_matching_a_pending_mail_task_card_files_nothing` pins it). Numbered PQ9 on `email-engine` and `email-app` until T17 (2026-10-03) renumbered it PQ13, since `email-cloud`'s PQ9 is the Gmail live binding (answered 2026-10-02): a ledger record of "Quinn's PQ9 answer" answers the binding, never this. | Recorded 2026-10-03 (Checkpoint C's engine-merge review). The work built past the gate is unaffected by either answer: T6.3's view reads a task card's `due` from its payload either way (`mail/view.rs`), and T6.2–H2 only carry out the plan's actions. So (b) stays a fix to T5.6's two files (`mailreconcile/{resolve,rules}.rs`) and the five cases of that one test. The loss happens only in a live mail pass on a student's vault, and none runs before T18. | T5.6, T6.3; PQ13's (b) test `a_live_new_item_with_another_due_files_its_own_card` | **Answered 2026-10-03 (Quinn): (b), due-aware. Built 2026-10-04** (`79c1742`). The lane was merged by then (T17), so the fix landed on `email-forwarding` directly, not on `email-engine` and back through `email-app`. The five cases moved; the loss described here no longer happens for a due on another day. Contract-reviewer's re-check of `79c1742` and its fix round `c99785f` PASSED (2026-10-04), so Checkpoint D is closed on PQ13 (b). T11.4 is not blocked, since mail-applied's shape does not change. |
| T15.0 adds three optional output fields, `from`, `mail_date` and `evidence`, to `surface::Card`, against §6.1's last bullet, "**Unchanged:** `rank` …, `surface`, `write`, …" (spec line 798). The controller added it on 2026-10-03 as a logged guess while Quinn was away. No spec text was edited. | §6.2 puts the card's `from`, `mail_date` and `evidence` on the deck (spec line 846; D21, §5.2). `commands.rs` computes nothing, so the read model is the one place the deck can take them from. The cost on a vault without mail is zero. A key the card lacks is left out, never `null`, so every other card serializes byte for byte as before. `surface_oracle.rs` runs unchanged against its three references, `schema` stays 1, and `surface` still never writes. The alternative leaves `surface` as it was: the deck fetches each mail card's frontmatter through the existing `note_detail` command, one call per card, and each call re-indexes the vault. | T15.0 (`34a53c8`); `a_mail_card_carries_its_from_mail_date_and_evidence_as_literals`, `a_card_without_mail_fields_serializes_exactly_as_before`; `surface_oracle.rs` unchanged | **Closed: G2 kept, signed by Quinn 2026-10-04 (Q2).** T15.0 is kept, and spec §6.1's "Unchanged" bullet carries the signed marker: "Signed: Quinn, 2026-10-04 (G2/Q2): surface carries a mail card's from, mail_date and evidence, on created_by: mail cards only, omitted when absent." |
| T10.1's pace, `MAX_PER_MINUTE` 10 and `MAX_PER_DAY` 600 (`app/src/mail_steps.rs`), merged at T17 (2026-10-03) before S6 ran, against T17 step 6's "refuse … if S6 has not run". | §6: "S6 blocks T17's merge of the pace numbers, not the code"; the dispatch for T17 took that reading. Both doc comments say "provisional, pending S6". | T10.1, T17; S6's record when it lands | **open (Quinn, S6):** an M365 test tenant run sets both numbers; a change is one constant each, and T10.1's pace tests read the constants. Until then T18c's school backfill stays under 600. |
| The sign-in detector read the native window title (found live in T18c, 2026-10-05). Setup sat on "Sign in" forever after the student signed in to Microsoft 365: `window_signed_in` fed `signed_in` `WebviewWindow::title`, which in tauri 2.11.5 is the NATIVE window's title, the one `open` itself set ("Sign in to your email", hidden "Knowlu email setup"), never the page's. So X2's title rule could never hold on a real window, for Microsoft 365 and Gmail alike, and every hidden saved-sign-in run (backfill, `wait_signed_in`, the slot's mail step, a deletion's forwarding-off) waited out its limit; Gmail's address hint (`live.rs` `title_address`) read the same wrong title. Unit tests passed because the `Host` seam is stubbed. | Root cause, not a flake: the wrong title source. Fix: the mail window's builder registers `on_document_title_changed` (WebView2's `DocumentTitleChanged`, no IPC, no capability, no new invoke), which writes the page's title to `mail::PAGE_TITLE` (`PageTitle`) under the generation `open` began, so a late title from a closed window is dropped; `close` clears it. `window_signed_in` and `title_address` read it through `page_title`. `signed_in`'s pure title and URL rule and its tests are unchanged. The title is held in memory only while the window is open, never logged, written or sent. Grades' capture decides sign-in by URL and `users/me`, never a title, so it was not affected. | `mail_window.rs`: `the_page_title_store_keeps_the_current_windows_latest_title_only`, `the_sign_in_detector_is_fed_the_page_title_never_the_native_window_title` (a source tripwire: no mail source calls the native title getter) | **fixed** on `email-forwarding`; re-prove in T18c's live run. |
| The Microsoft 365 sign-in step ended on the page's first read, and its failure read as nothing (found live in T18c, 2026-10-05, build `e5c7a2b`). The page title turned signed in at 20:48:40 and the window closed at 20:48:41: `runner.rs` `sign_in` ran `m365/signin.js` once, on an Outlook page that had drawn its mail list but not yet its folder pane, so the address read `not found` and the step ended at once. The step machine then set a non-done sign-in to `skipped` with no reason (`mail_steps.rs`), so the progress read `failed`, `sign-in`, and every row `skipped` with a null reason. An earlier run on the same tenant and build passed, so it hung on timing. | Root cause: a single read, made before the page had drawn what it reads. Fix: once the title reads signed in, a page with no address is read again each second for `SIGN_IN_READ` (45 s, the order of `WindowMailbox::open`'s 30 s wait for `ready.js`, which says only that the document loaded, not that the folder pane is drawn), and the clock restarts if the title stops reading signed in. Gmail's title address takes the same wait. A sign-in that does not take keeps `failed` with its reason on its own row (`not_found` or `script_failed`); the later steps are `skipped`, and a closed window or the 15-minute wait is still `cancelled` and `skipped`. The console gives sign-in its own words for both reasons (`MP_STEP_REASON`). No assertion moved: no test pinned a failed sign-in's row. | `mail.rs`: `a_microsoft_folder_pane_drawn_after_the_title_still_signs_in` (20 misses, then the address), `a_microsoft_address_never_drawn_fails_sign_in_naming_why_within_a_bound`; `mail_steps.rs`: `a_sign_in_that_fails_names_its_reason_and_ends_the_setup`; `static_assets.rs`: `a_failed_sign_in_row_says_why_in_its_own_words` | **fixed** on `email-forwarding`; re-prove in T18c's live run. If the tenant's folder pane never names the address, the row now reads `failed` with `not_found` after 45 s, rather than a silent skip. Review fix round (2026-10-05): a read already under way finishes even past the 15-minute sign-in limit (`max(SIGN_IN_WAIT, signed-in + SIGN_IN_READ)`), so a sign-in that takes in its last seconds names why it failed and is not reported as cancelled (`a_sign_in_that_takes_in_the_last_seconds_still_gets_its_read_and_names_why`). |
| Microsoft 365's forwarding scripts required a Save button that live Outlook does not draw (found live in T18c, 2026-10-05, read-only CDP on a university tenant). The Forwarding dialog showed its heading and the `input type=checkbox role=switch` labelled "Enable forwarding" (checked false), and **no Save while the switch was off**: its only buttons were the settings tabs, "clear" and "Close settings". `forwarding_set.js` answered `not found` `save` before flipping the switch, so it could never succeed on a real page; `forwarding_off.js` made the same check before its change. Neither script, nor `forwarding_read.js`, waited for the dialog, which Outlook draws after the document is complete. The synthetic page had a Save button from the start, so the tests passed. | Root cause: the scripts were written against a page shape that live Outlook does not have. Fix: only the switch is required up front, waited for up to `draw_ms` (default 15 s). The change is then made, and the address field, keep-a-copy and an enabled Save are each waited for up to `wait_ms` (default raised from 2 s to 5 s). A Save that settles may disappear or go disabled. A control that never comes is answered with every change put back (keep-a-copy, the address, then the switch), so no unsaved change is left. Before, only the switch was put back. A field drawn empty while forwarding is on is given `wait_ms` to fill in before set, read or off concludes. Off's R4 rule is unchanged after the wait. Worst cases stay inside each caller's bound (off about 30 s against `FORWARDING_OFF_BOUND`'s 60 s; the runner's 90 s step). No Rust caller changed. | `m365_forwarding_test.ts`: the page is now the live shape (dialog drawn late, the field and keep-a-copy only while on, no Save until a change); `m365_the_live_page_has_no_save_until_the_switch_is_turned_on`, `m365_a_dialog_drawn_late_is_waited_for`, `m365_forwarding_off_waits_for_save_after_the_change_and_puts_the_switch_back_without_one`, `m365_forwarding_set_puts_back_every_change_when_save_never_comes`. No assertion moved. Tests whose page never draws a control pass a short `draw_ms` or `wait_ms`, and snapshots wait for the page to draw. | **fixed** on `email-forwarding`; re-prove in T18c's live run. Whether a saved Save disappears or goes disabled on the live page is unread; the script accepts both. Review fix round (2026-10-05): set treats a field still blank after the wait while the switch is on as `not found` `address` unless the student said replace, so it never overwrites an address not yet drawn. Every wait is a deadline on the clock (`Date.now()`), not a count of 50 ms turns, because a hidden WebView2 may throttle its timers. Off reads the switch again when it clicks, and undoes only its own click. A test covers set from off with no Save: every change is put back, the switch last. |

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
| `engine/src/enrich.rs` tests module (added 2026-10-02, T6.4) | `two-desktop` | T6.4 (`EMPTY_MAIL_PULL`, seven arm tests edited) | `two-desktop` | `contract-engineer` |
| `engine/tests/cloud_contract.rs` (added 2026-10-02, T6.4) | `two-desktop` | T6.1 and T6.4 (one arm test edited) | `two-desktop` | `contract-engineer` |
| `engine/src/write.rs` CL | `two-desktop` | none (used only) | `two-desktop`; if it merges first, Checkpoint D re-reads T6.2's use of `write` | `contract-engineer` |
| `_shared/judge_validate.ts` | `j-events` | T3b (`MailKind`) | the second to merge | `cloud-engineer` |
| `_shared/judge_prompts.ts` | `j-events` | T3c | the second to merge | `cloud-engineer` |
| `_shared/migrations_test.ts` | `j-events` | T1.1–T1.3 | the second to merge; the `mail` stamp sorts last | `cloud-engineer` |
| `migrations/20260922120300_event_decomposed.sql` (new on `j-events`) | `j-events` | none; T1.1's stamp sorts after it | — | `cloud-engineer` |
| `app/src/scaffold.rs` (`Curated`) | `p3-registrar` | T14 | `p3-registrar`; T14's fields go after `main`'s | `implementer` |
| `scripts/wizard-check.py` | `p3-registrar` | T15.3 | `p3-registrar` | `console-ui` |
| `engine/src/cli.rs` | `p3-registrar` | H2 | the second to merge | main session |
| `app/src/{lib,main}.rs` | `p3-registrar` | H3–H5 | the second to merge | main session |
| `engine/src/lib.rs` (added 2026-10-02, T0.0 step 5) | `p3-registrar` | H1a–H1c (`pub mod` lines; H1a is built) | `p3-registrar` | main session |
| `app/static/index.html` (added 2026-10-02) | `p3-registrar` | T15.1–T15.5 | `p3-registrar` | `console-ui` |
| `app/static/console.js` (added 2026-10-02) | `p3-registrar` | T15.1–T15.5 | `p3-registrar` | `console-ui` |
| `app/static/console.css` (added 2026-10-02) | `p3-registrar` | T15.1–T15.5 | `p3-registrar` | `console-ui` |
| `app/tests/static_assets.rs` (added 2026-10-02) | `p3-registrar` | T15.1–T15.5 | `p3-registrar` | `console-ui` |
| `CLAUDE.md` (added 2026-10-02) | `p3-registrar` | H0 (at T17) | `p3-registrar` | main session |
| `engine/src/journal.rs` CL (added 2026-10-02) | `two-desktop` | none: read-only here | `two-desktop` | `contract-engineer` (no conflict expected; this lane does not edit it) |
| `engine/src/sync.rs` CL (added 2026-10-02) | `two-desktop` | none: read-only here | `two-desktop` | `contract-engineer` (no conflict expected) |
| `engine/src/ids.rs` CL (added 2026-10-02) | `two-desktop` | none: read-only here | `two-desktop` | `contract-engineer` (no conflict expected) |

- T3b checks `MailKind` against `j-events`' `Kind` and validators before it starts. T1.2's
  `lastModelPinBlocks` change and its retitled export test are checked against `j-events`' edits to
  the same file. *(2026-10-02: the events lane merged as #31 and is in the base, so these checks
  read `main`'s files, and the `j-events` rows above need no rebase; §9.5.)*
- **T6.4's arm tests (integrator).** The cloud arm of `judge` now makes one more request, `mail-pull`,
  after `gmail-read` and before `judge-rules`. Any test that scripts the arm by arrival order, in
  `enrich.rs`'s tests module or `cloud_contract.rs`, needs a mail-pull reply (`EMPTY_MAIL_PULL`) between
  the gmail-read and judge-rules replies; `two-desktop` edits that module (hunks near its lines
  1740-2750), so the rebase re-checks each such test it adds or changes. Disjointness against
  `origin/two-desktop`, run 2026-10-02 with `git merge-tree`: `enrich.rs` conflicts with or without T6.4
  (the same three files, `approvals.rs`, `enrich.rs`, `write.rs`, conflict at `HEAD~1`, so the cause is
  `main` moving since `two-desktop` was cut, not this lane); `cloud_contract.rs` merges clean. T6.4's
  own commit recorded "disjoint" without that run being shown.
- The ten rows added on 2026-10-02 come from T0.0 step 5's record. `p3-registrar` (Pilot) and
  `two-desktop` (Launch) are later in the program, so each rebases onto this MVP lane. R12's
  contract-list diff shows any edit this lane makes to `journal.rs`, `sync.rs` or `ids.rs`.

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
  - T3b refuses line separators and C0 controls in `title`, `from`, `why` and `target` at the service.
    `evidence` is whitespace-folded first (D9: a quote of hard-wrapped mail is verbatim), then held to
    the same check, so no line break reaches the device (T3b review).
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

*(2026-10-02: PQ5 was answered (c) at T0.0 and built in T8. PQ6, PQ7 and PQ8 were answered later
the same day, and Checkpoint A passed with the plan's reading of F3 under T8c confirmed (§4's "Asked
after the rulings of 2026-10-02"). What is still open is §6's list.)* *(2026-10-03: PQ12 was answered
(a), and the unbound display was ruled the same day; both are built, §4 PQ12.)*

- **PQ5 (blocks T8 and T8b).** How a feed card for `title` becomes approvable. The recommendation is
  (c): `title` is amendable only on a `created_by: blackboard` card, `AMENDABLE_FIELDS` stays nine, and
  every other amend is unchanged. (a) widens the list for every amend in every vault. (b) narrows the
  signed D11 (5) to `due` only.
- **No new ask from R8 or R17.** R8 follows the `@std/assert` precedent, and vendoring stays available
  if Quinn wants it. R17 keeps A12's sentence true by keeping mail skips out of the runner log.

**Plan-only tests added in this round:** every test named in R1, R3, R4, R7, R10, R11 and R13–R17.
Each pins a requirement the review found unpinned. None changes a spec test's assertion. The one
rename, T11.5's cadence test, keeps the "at most once a day" assertion as its `Daily` arm.
