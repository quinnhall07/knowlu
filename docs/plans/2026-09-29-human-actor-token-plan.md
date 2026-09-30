# Ruling 11: the human token `student` (implementation plan)

**Date:** 2026-09-29. **Base:** main at `86a3431`, worktree `.claude/worktrees/human-actor`.
**Governs:** cloud design, Amendment 2026-09-29, ruling 11 (signed by Quinn 2026-09-29); signing
packet item 4; focus report D5. **Stage:** Integrate (it is part of the Pilot gate, ruling 10).
**Status:** draft for review. Nothing here is built yet.

## 1. Goal, and what must not move

A vault the app creates records its student as `student`, not `quinn`. Every comparison against the
human actor treats `student` and `quinn` as the same human, forever. The token is stated once per
vault in `config/actor.yaml`.

Five things must not change:

- **Existing vaults.** A vault without `config/actor.yaml` reads as `quinn` and keeps writing `quinn`,
  so its journal lines, frontmatter and today page come out byte-identical.
- **Frozen references.** None of the eight Python references is regenerated. The three surface-oracle
  references are not regenerated either: their fixture vaults have no `actor.yaml`.
- **The journal byte contract.** The record shape, key order and separators stay the same. A
  `student` record differs from a `quinn` record only in the actor's value.
- **The cloud.** The keep rule (`not like 'agent:%'`) and `provenance::is_agent` already read both
  tokens as human. `cloud/` has no `quinn` literal. Nothing there changes.
- **`profiles::migrate_flat_layout`.** Its `quinn-ops` literal is a different string and stays.

VISION check: this is the Pilot gate's neutral-token clause (VISION "The pilot"). It also applies
CLAUDE.md rule 1 and does not touch rule 2.

## 2. Decisions this plan makes, with the reason for each

- **D1. Where the token and its reader live: `engine/src/journal.rs`.** It holds
  `HUMAN_ACTOR = "student"`, `LEGACY_HUMAN_ACTOR = "quinn"`, `is_human(actor)` (an exact match on
  either token), the `config/actor.yaml` reader and its create-once writer, and a pure function that
  picks the token from a list of records. The ruling names `journal::HUMAN_ACTOR`, and `journal.rs`
  is already on the contract list, so the list does not grow. The alternative is a new
  `engine/src/actor.rs`, which would have to be added to the contract list in CLAUDE.md.
- **D2. The CLI defaults become the vault's own token.** These flags lose their static
  `default_value = "quinn"` and become optional: `commitments --actor`, `info --actor`,
  `issues --actor`, `write --actor`, `info open --opened-by` and `info close --closed-by`. When a flag
  is absent, the vault's token is used, read through D1's reader. If `actor.yaml` is invalid, the
  command exits 2 with the reader's named line and writes nothing.
  Two simpler choices were rejected:
  - A static `"student"` default would write `student` into every legacy vault. That breaks "one
    account writes one token".
  - Keeping `"quinn"` would break rule 1.

  The ruling says the defaults change "for that vault", and only the vault can say which token that
  is. An explicit value is passed through unchanged, subject to D3.
- **D3. `write` is the gate.** Every note write goes through `write`, so it is the one place that can
  make "no human write proceeds" true for every caller. Before any journal record, a write whose
  actor is human checks the vault's token. "Human" means neither an agent (`is_agent`) nor
  `system:`-prefixed. An invalid `actor.yaml` refuses the write by name. Agent and `system:` writes
  are never gated, so `judge`, `sync` and `coursework` still exit 0.
  `passes::detect_external` writes records without going through `write`, so it runs the same check
  itself.
  Whether a human actor that does not match the vault's token is also refused is Q2.
- **D4. `reconcile.rs`'s synthetic contender uses `journal::HUMAN_ACTOR`.** That record is never
  written: `record_key` keeps only ts, device, seq and field. Its actor only feeds `actor_rank`, where
  both tokens rank 0, so the outcome and the bytes are unchanged and `resolve` stays pure.
- **D5. The judge-once skip reason names the recorded actor.** `write.rs`'s
  `judge-once: {name} set by quinn at {ts}` takes the actor from the human record it found. On a
  legacy vault that is still `quinn`, so the text is byte-identical. The guard test would not catch
  this line, because it is not the exact literal, but it prints a person's name. Rule 1 covers it.
- **D6. The restore probe is built from the engine's pure picker plus `sync::pull`, and runs in the
  app.** The wizard seeds human records (the first task and the course notes) inside
  `scaffold::create_vault`, and that happens before `sync::restore_into` runs. So the token has to be
  decided before `create_vault`. `onboarding::create_vault_in` pages `knowlu_engine::sync::pull`
  under the pending session and passes the records to D1's picker:
  - any human record by `quinn` gives `quinn`;
  - otherwise the answer is `student`, including when there is no human record.

  The probe stops at the first `quinn`. `sync.rs` is not edited for the probe.
  Reordering scaffold to run after restore was rejected. It would reopen C3′'s seed-hash and
  `not_fresh` logic, which is reviewed and merged.
- **D7. A new vault always gets `actor.yaml`, written once.** `scaffold::build_into` writes it from
  the token the wizard decided, before `seed_writes`, so the seeds are written with that token. If the
  probe found `quinn`, the file says `human_actor: quinn`. An explicit file reads more clearly than an
  absent one, and the reader accepts both values. The adopted-vault path, the backup-folder restore
  (`restore_vault_in`) and `account` adopt all copy or keep a vault and never write this file.
- **D8. `console_ctx()` becomes `console_ctx(vault) -> Result<WriteContext, String>`.** It reads the
  token for each write, so a hand-edit to `actor.yaml` takes effect on the next write without a
  restart. `executor_ctx()` stays `agent:approvals`. CLAUDE.md and `docs/reference/app.md` name
  `console_ctx()`, so their wording changes (Q3).

## 3. Open questions for Quinn (each with a recommendation)

- **Q1. What does the wizard do when the restore probe cannot reach the account?**
  - *Recommend:* refuse Finish with a named, retryable line ("Knowlu could not reach your account to
    check for an existing vault; check the connection and press Finish again"), keeping the plan.
  - *Why:* the wizard signed in a few panels earlier, so being offline at Finish is rare. A guessed
    `student` on an account whose journal says `quinn` breaks "one account writes one token", and a
    later restore on another desktop would then choose differently.
  - *Cost:* this reverses C3′'s "never refuse a vault over a dead connection" for this one call.
  - *The alternative:* proceed as `student` and accept a mixed account. This is functionally
    harmless, because every comparison treats the two tokens as equal. Before the Pilot the only
    `quinn` accounts are the founder's.
- **Q2. Does `write` also refuse a human actor that differs from the vault's token?** Examples are an
  explicit `--actor quinn` on a `student` vault, or `--actor alice`.
  - *Recommend:* yes. This is the only thing that makes "one account writes one token" mechanical.
    Without it, one stray CLI call puts a `quinn` record into a `student` account, and every later
    restore flips to `quinn`.
  - *Cost:* three existing tests write as `student` on a vault with no `actor.yaml`
    (`sync.rs`'s `build_push_sends_no_local_card_nor_any_record_about_one`, and `surface.rs`'s
    `note` and `edit` helpers). Each gains an `actor.yaml` in its setup, and no assertion changes.
  - Without Q2, D3 refuses only on an invalid file.
- **Q3. CLAUDE.md wording.** "every vault write goes through the engine's `write` with
  `console_ctx()`" becomes `console_ctx(vault)`. A line on `config/actor.yaml` belongs in the engine
  invariants. This plan writes only under `docs/`, so the main session makes both edits at merge.

**Quinn's answers, 2026-09-29:**

- **Q1:** refuse Finish with the named, retryable line, as built.
- **Q2:** yes. `write` refuses a human actor that differs from the vault's token, as built in ef28570.
- **Q3:** applied to CLAUDE.md (`console_ctx(vault)` and one engine-invariant line on
  `config/actor.yaml`) in the commit that records these answers.

## 4. Site inventory (grep of `quinn` on `86a3431`, test code excluded)

Line numbers are for orientation only. Briefs cite the function, never the line. **CL** marks a
file on the contract list.

| Site | File (CL?) | What it does today | Becomes | Task |
|---|---|---|---|---|
| `actor_rank` | `engine/src/journal.rs` (CL) :113 | `"quinn" => 0` | both tokens rank 0 | T1 |
| `Journal::human_set` | `journal.rs` (CL) :274 | actor `== "quinn"` | `is_human` | T1 |
| `Journal::human_edited` (from p2) | `journal.rs` (CL) :292 | actor `== "quinn"` | `is_human` | T1 |
| judge-once skip reason | `engine/src/write.rs` (CL) :285 | text `set by quinn` | the recorded actor (D5) | T2 |
| human-write gate | `write.rs` (CL) | none | D3 | T2 |
| synthetic contender | `engine/src/reconcile.rs` (CL) :133 | actor `"quinn"` | `HUMAN_ACTOR` (D4) | T2 |
| `detect_external` create and set | `engine/src/passes.rs` :293, :332 | writes `quinn`/`external` records | the vault's token; a bad file is a named pass-log line | T2 |
| action log "me" | `engine/src/surface.rs` :1495 | `actor == "quinn"` | `is_human` | T2 |
| provenance "you" | `surface.rs` :1725 | `actor == "quinn"` | `is_human` | T2 |
| CLI defaults | `engine/src/main.rs` :81 (`commitments --actor`), :197 (`info`), :208 (`issues`), :219 (`write`), :284 (`--opened-by`), :299 (`--closed-by`) | `default_value = "quinn"` | optional, resolved from the vault (D2) | T3 |
| `console_ctx` | `app/src/commands.rs` :158 | `WriteContext::new("quinn", …)` | reads the vault's token (D8) | T4 |
| console task create | `commands.rs` :249, :253 | `effort_source` and `created_by` set to `quinn` | the context's actor | T4 |
| console info close | `commands.rs` :329 | `closed_by` set to `quinn` | the context's actor | T4 |
| first-task seed | `app/src/scaffold.rs` :736, :740 | `effort_source` and `created_by` set to `quinn` | the decided token | T4 |
| `actor.yaml` written | `scaffold.rs` `build_into` | none | D7 | T4 |
| commitments confirm doc | `app/src/week.rs` :17-18 | "the engine's default, `quinn`" | "the vault's token" | T4 |
| restore decides the token | `app/src/onboarding.rs` `create_vault_in` | none | D6 | T5 |
| the literal guard | new `engine/tests/human_actor_literal.rs` | none | §5 T6 | T6 |

Doc comments that describe legacy behaviour stay unchanged. These are in `commitments.rs`,
`passes.rs`, `reconcile.rs` and `surface.rs`, and the guard ignores comments.

These need no change:
- `provenance::is_agent`;
- `approvals.rs`, `eventledger.rs` and `completion.rs`, which take their actor from the caller's
  context;
- `app/static`, which has no `quinn`;
- `cloud/`.

The spec's site list also names p3-registrar's `main.rs:91`, which is not on main. When p3 resumes,
its `--actor` default follows D2. The guard test (T6) fails if it does not.

## 5. Tasks

The tasks run in order: T1, T2, T3, T4, T5, T6. Each task writes its named tests first and sees them
fail for the stated reason, then writes the code. Each task ends with `cargo build --workspace` and
`cargo test --workspace`: green, 0 warnings, and the four `#[ignore]`s untouched. Ruling 11 assigns
this work to `contract-engineer` at xhigh, with review before merge. T1-T5 follow that. T6 is a
self-contained scan test.

### T1. The token, the reader and the picker (`journal.rs`)

**Agent:** `contract-engineer`. **Why:** `journal.rs` is on the contract list, and everything later
depends on this API.
**Files:** `engine/src/journal.rs` (CL) only. The tests go in its `#[cfg(test)]` module.

**Tests first:**
- `the_two_tokens_are_the_constants_and_nothing_else`: `HUMAN_ACTOR == "student"` and
  `LEGACY_HUMAN_ACTOR == "quinn"`. `is_human` is true for exactly these two, and false for
  `agent:x`, `system:migration`, `Quinn` (case matters), `student ` and the empty string.
- `actor_rank_ranks_student_with_quinn`: `student` ranks 0. The existing `quinn`, `agent`,
  `system` and other ranks are unchanged.
- `human_set_and_human_edited_count_a_student_record`: a `student` `set` counts for both.
  A `student` `create` counts only for `human_set`. A mixed journal (one `quinn` set, then a
  `student` set) returns the latest record under either token.
- `an_absent_actor_file_reads_as_quinn`.
- `the_actor_file_accepts_only_student_or_quinn`:
  - Accepted: `human_actor: student`, `human_actor: quinn`, the CRLF forms, and a quoted value.
  - Refused with the named error: `alice`, `agent:knowlu.enrich`, `system:x`, an empty value, a
    missing key, a non-mapping, and an unreadable file. The error text names the file, the value it
    found and the two accepted values. It never includes more of the file than the value.
- `the_actor_file_is_written_once_and_never_rewritten`: the writer creates the file with one line,
  `human_actor: <token>` and a trailing LF. A second call returns an error and leaves the bytes as
  they were. The writer refuses any token that is not one of the two.
- `the_token_is_picked_from_the_accounts_records`:
  - any human record by `quinn` gives `quinn`, even when `student` records come first;
  - only `student` records give `student`;
  - no records, or only agent and `system:` records, gives `student`.
- `a_student_record_is_a_quinn_record_with_one_value_changed`: the same `NewRecord` made under each
  token serialises through `make_record` to lines that are equal after swapping the token. This pins
  the byte contract.

**Behaviour:** see D1.
- The reader reads through the crate's CRLF-aware path (`pystr`) and parses with the crate's YAML
  reader.
- The writer uses create-new semantics, so it can never overwrite.
- The picker takes records in the shape `sync::pull` returns. It only reads them.

**Done when:** the eight tests pass. The existing `journal.rs` tests pass unchanged.

### T2. The engine's write paths and readers

**Agent:** `contract-engineer`. **Why:** three of these files are on the contract list, and
`passes.rs` writes journal records itself, so a silent error here would corrupt vault bytes.

**Files:**
- `engine/src/write.rs` (CL);
- `engine/src/reconcile.rs` (CL);
- `engine/src/sync.rs` (CL), only the setup of its existing test (Q2);
- `engine/src/passes.rs`;
- `engine/src/surface.rs`.

**Tests first:**
- `write.rs`:
  - `a_human_write_on_a_vault_with_a_bad_actor_file_writes_nothing`: covers `write_literals`,
    `create`, `delete`, `move` and `append_body` as a human. Each one returns the named error. The
    journal directory and the note bytes are unchanged, compared byte for byte.
  - `an_agent_or_system_write_ignores_the_actor_file`: the same bad file, and `agent:knowlu.enrich`
    and `system:idfix` writes still succeed.
  - `a_student_vault_takes_student_writes`: a vault with `human_actor: student` accepts a `student`
    write, and the record carries `student`.
  - `a_human_actor_other_than_the_vaults_is_refused`: `quinn` on a `student` vault, `student` on a
    legacy vault and `alice` anywhere are each refused by name, and nothing is written. This test
    exists only if Quinn says yes to Q2.
  - `judge_once_skip_names_the_recorded_human`: on a legacy vault the reason is byte-identical to
    today's (`judge-once: importance set by quinn at <ts>`). On a `student` vault it reads
    `set by student`.
  - `an_agent_cannot_overwrite_a_field_student_set`: a copy of
    `an_agent_cannot_overwrite_a_field_quinn_set` on a `student` vault, which proves judge-once holds
    for the new token.
- `reconcile.rs`: `an_external_write_still_wins_a_tie_as_the_human`. The synthetic contender beats
  an agent record with the same `ts`, exactly as before. The existing reconcile tests pass unchanged.
- `passes.rs`:
  - `an_external_edit_on_a_student_vault_is_journalled_as_student`.
  - The existing `an_obsidian_style_edit_is_journalled_as_quinn_external` stays as the legacy proof.
  - `a_bad_actor_file_skips_detect_external_by_name`: the pass log has one line naming the file.
    No record is written, and the external-edit index does not advance, so the edit is picked up once
    the file is fixed.
- `surface.rs`: `a_student_action_reads_as_me_and_you`. A `student` record renders as "me" in the
  action log and "you" in provenance. `quinn` still does, and an `agent:` record still does not.

**Behaviour:**
- D3's gate sits at the top of each public entry point in `write.rs`, before the journal is touched.
  The gate uses one helper, so the entry points cannot drift apart.
- D4 and D5 as written.
- `detect_external` reads the token once per call.
- `surface.rs` swaps its two `== "quinn"` tests for `journal::is_human`.

**Fixture rule:** the golden, oracle and sync tests run unchanged and must pass. They are the proof
that legacy vaults are byte-identical. If one fails, the change is wrong, not the reference.
**Done when:** the named tests pass. The three Q2 tests (if Q2 is yes) gain an `actor.yaml` in their
setup with no assertion changed.

### T3. The CLI's defaults (`main.rs`)

**Agent:** `contract-engineer`. **Why:** the ruling names these defaults. The file is off the list,
but every human write from a script or the app's `commitments --confirm` call goes through them.
**Files:** `engine/src/main.rs`, and a new `engine/tests/cli_human_actor.rs`. That test drives the
built binary against scratch vaults under a temporary directory, the way the existing CLI tests do.

**Tests first:**
- `write_set_without_actor_journals_the_vaults_token`: a legacy vault gets `quinn` and a
  `student` vault gets `student`.
- `info_open_and_close_default_to_the_vaults_token`: `opened_by` and `closed_by` in the note, and
  the journal actor, equal the token.
- `commitments_confirm_without_actor_writes_as_the_vaults_token`.
- `issues_open_without_actor_writes_as_the_vaults_token`.
- `a_bad_actor_file_exits_2_and_writes_nothing`: this runs `write`, `info`, `issues` and
  `commitments --confirm`. Each exits 2 with the reader's line on stderr, and the vault's bytes are
  unchanged.
- `an_explicit_agent_actor_is_passed_through`: `--actor agent:x` still writes as `agent:x`.

**Behaviour:** see D2.
- `commitments` without `--confirm`, and every command that only reads, never looks at the file.
- `surface` and `rank` keep their "never gated" contract.
- The `--help` text for each flag reads "default: the vault's human token (config/actor.yaml)".

**Done when:** the named tests pass. `docs/reference/engine-commands.md`'s flag lines are listed as
a docs follow-up (§7).

### T4. The app writes the token (`commands.rs`, `scaffold.rs`)

**Agent:** `contract-engineer`. **Why:** this is the cross-crate pin between `console_ctx` and
`human_set` that the 09-23 plan deferred (its fidelity ledger's M-2 row). A wrong token here is
written into every new student's first notes.

**Files:**
- `app/src/commands.rs`;
- `app/src/scaffold.rs`;
- `app/src/week.rs` (doc comment only);
- `app/src/onboarding.rs`, one line: `VaultPlan`'s new field is set to `journal::HUMAN_ACTOR`, and
  T5 replaces it;
- `app/tests/commands.rs`, `app/tests/scaffold.rs`, `app/tests/week.rs`.

**Tests first:**
- `app/tests/scaffold.rs`:
  - `a_new_vault_has_actor_yaml_and_seeds_as_student`: `config/actor.yaml` is exactly
    `human_actor: student\n`. The first task carries `created_by: student` and
    `effort_source: student`. Every non-migration seed record's actor is `student`. The existing
    assertion `others[0]["actor"] == "quinn"` becomes `"student"`, which is the ruling's intended
    change.
  - `a_vault_planned_as_quinn_writes_quinn_throughout`: the restore case, with the token passed in.
- `app/tests/commands.rs`:
  - `console_writes_on_a_student_vault_are_student_and_frozen_against_agents`: a console `set` on a
    `student` vault is journalled as `student`, and `journal::human_set` then finds it. This is the
    M-2 pin.
  - `console_create_task_stamps_the_vaults_token`.
  - `a_console_write_with_a_bad_actor_file_is_refused_by_name_and_writes_nothing`: `ok: false`, the
    named error, fresh state, and unchanged bytes.
  - The existing `quinn` assertions (for example
    `set_fields_journals_a_quinn_dashboard_record_and_returns_fresh_state`) stay as they are when
    their vault has no `actor.yaml`. They become the legacy proof.
- `app/tests/week.rs`: on a scaffolded vault, the confirm's journal actor is `student`. The current
  assertion at :123 changes with it.

**Behaviour:** see D7 and D8.
- `console_ctx(vault)` returns the reader's error as the command's `error` string. Every caller
  propagates it through `mutate`.
- `create_task_inner` takes `created_by` and `effort_source` from the context's actor.
- `close_info_inner` passes the context's actor as `closed_by`.

**Done when:** the named tests pass. `app/src` has no `"quinn"` left.

### T5. A restore decides the token first (`onboarding.rs`)

**Agent:** `contract-engineer`. **Why:** this is one account's token for life. It also handles a
session bearer and reads account data, so a mistake would either leak or flip the token.
**Files:** `app/src/onboarding.rs` and `app/tests/onboarding.rs`. `scaffold.rs` is only read here;
its `VaultPlan` field came in T4.

**Tests first.** Each test serves its own stub `/sync-pull` on `127.0.0.1:0`, the way
`app/tests/onboarding.rs` already does. No egress.
- `a_restore_into_an_account_with_quinn_records_writes_quinn`: the stub's journal has one human
  `quinn` record (as at :648). `actor.yaml` then says `quinn`, the seeds are `quinn`, and after
  `restore_into` the vault contains no `student` record.
- `a_new_account_gets_student`: the stub returns no records. `actor.yaml` says `student`.
- `an_account_with_only_agent_records_gets_student`.
- `quinn_anywhere_wins_over_earlier_student_records`: the stub returns two pages, with `quinn` only
  on the second.
- `the_token_is_decided_before_anything_is_written`: the stub is made to fail the probe. No vault
  folder is created, and neither `actor.yaml` nor a journal record exists. What the wizard returns
  depends on Q1: a named, retryable refusal, or a `student` vault.

**Behaviour:** see D6.
- The probe runs after `account_id` is read and before `scaffold::create_vault`. It uses the pending
  session target, `account::api_base()` and `account::anon_key()`.
- It pages `sync::pull` until it finds `quinn` or runs out of pages, using the same stall guard as
  `restore_all`. The records go to T1's picker.
- The probe's records are not written anywhere. `restore_into` still does the real pull.
- The session token is never logged or put in an error string.

**Cost:** an account with only `student` records is pulled twice at Finish, once here and once by
`restore_into`. An account with `quinn` records stops at the first one.
**Done when:** the named tests pass. The existing onboarding and restore tests pass unchanged.

### T6. The literal guard

**Agent:** `test-writer`. **Why:** it is one self-contained test with no product code, and its
specification is complete. It lands last, because it asserts the state T1-T5 leave behind.
**Files:** a new `engine/tests/human_actor_literal.rs`. Like `workflows.rs`, it reads the workspace
from `CARGO_MANIFEST_DIR/..`.

**Tests:**
- `no_quinn_literal_outside_the_legacy_constant`. It scans `engine/src/**/*.rs` and
  `app/src/**/*.rs`. From each file it first removes every item under a `#[cfg(test)]` attribute,
  skipped by brace depth, and every line whose trimmed start is `//`. It then counts `"quinn"` as an
  exact double-quoted literal. It also scans `app/static/**/*.js`, and every file under
  `cloud/supabase/` that is not `*_test.ts`, for `"quinn"` or `'quinn'`. The only occurrence allowed
  is `LEGACY_HUMAN_ACTOR`'s definition in `engine/src/journal.rs`. A failure lists `file:line` for
  each hit.
- `the_scanner_is_not_vacuous`: on inline samples, a `#[cfg(test)] mod tests { … "quinn" … }` counts
  0, a plain `fn` holding `"quinn"` counts 1, and `"quinn-ops"` counts 0. The last case is why
  `migrate_flat_layout` stays legal.
- `the_legacy_constant_is_there_exactly_once`: this keeps the guard from passing if the constant is
  renamed away.

**Done when:** all three pass on the tip of T5. Reverting any single T2-T4 site by hand makes the
first one fail. The reviewer checks this by reading the scanner, not by a commit.

## 6. Order, parallelism and checkpoints

- **One branch, `human-actor`, run strictly in order.** Each task depends on the one before it:
  T2-T5 use T1's API; T4's `week.rs` test needs T3's default; T5 needs T4's field; T6 needs all of
  them.
- The tasks mostly touch disjoint files. There are two overlaps, each by one line and in sequence:
  `onboarding.rs` (T4, then T5) and `scaffold.rs` (T4 edits it, T5 only reads it).
- One cargo at a time (`-j 2`), and tests run in the foreground.
- **Checkpoint A, after T2:** `contract-reviewer` reviews the contract-list diff (`journal.rs`,
  `write.rs`, `reconcile.rs`, `sync.rs`) and confirms that the frozen references and oracles ran
  unchanged. Quinn answers Q2 before T2 starts, because it decides one test and the gate's shape.
- **Checkpoint B, after T5:** Quinn answers Q1 before T5 starts. `reviewer` covers the app diff, and
  `contract-reviewer` rules on whether a session could leak from the probe.
- **Checkpoint C, after T6:** the whole-branch review lands in
  `docs/reports/2026-09-29-human-actor-review.md`. CI must be green before the push, which is a code
  push and needs Quinn's go.
- **Live proof** (the controller's job, after merge, on a scratch profile):
  - A new profile from a dev build shows `config/actor.yaml` as `student`, with `student` in the first
    task and the journal.
  - The founder's existing vault opens and writes `quinn`.
  - The profile is removed afterwards.

## 7. After the tasks (docs, by `docs-keeper`)

- `docs/reference/engine-commands.md`: the six flags now default to the vault's token.
  `commitments --confirm`'s `[--actor quinn]` becomes `[--actor <token>]`.
- `docs/reference/app.md`: `console_ctx(vault)`; `config/actor.yaml` in the app-data and vault
  description; the wizard's probe order.
- `docs/surface/anatomy.md`: "me" and "you" mean either token.
- HANDOFF gets its §1 fact at the Integrate milestone, batched with the other Integrate updates
  rather than as a separate docs push.
- CLAUDE.md (Q3) is the main session's edit.

## 8. Risks

- **A missed comparison site** would silently drop a student's hand-edit protection. Three things
  guard against it: T6 fails on the literal; T2 and T4 test judge-once under `student` end to end;
  and the inventory was built by grep rather than taken from the spec's list. The grep found two
  sites the spec did not name (`write.rs`'s skip reason and `commands.rs`'s `close_info_inner`) and
  corrected `surface.rs`'s line numbers.
- **An older app on another desktop** (0.1.x) reads a `student` record as not human. The Pilot is one
  desktop per student, `student` records exist only in vaults this version creates, and the updater
  moves installs forward. Two-desktop (Launch) should confirm every desktop runs a post-ruling-11
  build.
- **A hand-edited `actor.yaml`** stops the student's own writes until it is fixed. The ruling asks for
  exactly that. The console and the CLI both name the file and the two accepted values.

## 9. Fidelity ledger

| Ruling 11 requirement | Task | What proves it |
|---|---|---|
| New vaults write `student` wherever they wrote `quinn` | T4, T3, T2 | `a_new_vault_has_actor_yaml_and_seeds_as_student`, `console_create_task_stamps_the_vaults_token`, T3's four default tests (next row), `an_external_edit_on_a_student_vault_is_journalled_as_student` |
| The CLI's `--actor`, `--opened-by` and `--closed-by` defaults, for that vault | T3 | `write_set_without_actor_journals_the_vaults_token`, `info_open_and_close_default_to_the_vaults_token`, `commitments_confirm_without_actor_writes_as_the_vaults_token`, `issues_open_without_actor_writes_as_the_vaults_token` |
| Every human-actor comparison treats `student` and `quinn` as equal, forever | T1, T2, T4 | `actor_rank_ranks_student_with_quinn`, `human_set_and_human_edited_count_a_student_record`, `an_agent_cannot_overwrite_a_field_student_set`, `a_student_action_reads_as_me_and_you`, `an_external_write_still_wins_a_tie_as_the_human`, the M-2 pin `console_writes_on_a_student_vault_are_student_and_frozen_against_agents` |
| `actor.yaml` is one line, written once at creation, never rewritten | T1, T4 | `the_actor_file_is_written_once_and_never_rewritten`, `a_new_vault_has_actor_yaml_and_seeds_as_student` (exact bytes) |
| An absent `actor.yaml` means `quinn` | T1, T2, T3, T4 | `an_absent_actor_file_reads_as_quinn`; the unchanged legacy tests (`an_obsidian_style_edit_is_journalled_as_quinn_external`, `set_fields_journals_a_quinn_dashboard_record_…`, the CLI tests' legacy half) |
| The reader accepts only `student` or `quinn`; anything else is a named error | T1 | `the_actor_file_accepts_only_student_or_quinn` (both the accepted values and the refusals, as the ruling asks) |
| No human write proceeds while the file is invalid | T2, T3, T4 | `a_human_write_on_a_vault_with_a_bad_actor_file_writes_nothing`, `a_bad_actor_file_skips_detect_external_by_name`, `a_bad_actor_file_exits_2_and_writes_nothing`, `a_console_write_with_a_bad_actor_file_is_refused_by_name_and_writes_nothing` |
| Agent work is not stopped by a bad file (the exit-0 contracts) | T2 | `an_agent_or_system_write_ignores_the_actor_file` |
| A restore decides the token from the account's journal before `actor.yaml` or any human record | T5 | `the_token_is_decided_before_anything_is_written`, `a_restore_into_an_account_with_quinn_records_writes_quinn`, `quinn_anywhere_wins_over_earlier_student_records` |
| With no human record, `student` | T1, T5 | `the_token_is_picked_from_the_accounts_records`, `a_new_account_gets_student`, `an_account_with_only_agent_records_gets_student` |
| One account writes one token | T5, T2 (Q2) | the restore tests; `a_human_actor_other_than_the_vaults_is_refused` (Q2 answered yes, 2026-09-29; built in ef28570). Q1 answered: Finish refuses with the named, retryable line |
| One constant `journal::HUMAN_ACTOR` plus one legacy constant | T1, T6 | `the_two_tokens_are_the_constants_and_nothing_else`, `the_legacy_constant_is_there_exactly_once` |
| A guard fails on the exact literal `"quinn"` in non-test code outside the constants; `quinn-ops` stays | T6 | `no_quinn_literal_outside_the_legacy_constant`, `the_scanner_is_not_vacuous` |
| Existing vaults are unchanged | T2 (gate) | the frozen-reference, surface-oracle, sync and entitlement tests, run unchanged; Checkpoint A confirms nothing was regenerated |
| The journal byte contract holds | T1 | `a_student_record_is_a_quinn_record_with_one_value_changed`; the unchanged sync contract and replay tests |
| No existing journal line or note is rewritten | T1, T4 | no migration exists; the writer is create-only; the adopt and backup-restore paths never write `actor.yaml` (D7), checked at review |
| The cloud needs no change | none | §1; grep of `cloud/` at `86a3431`; T6 scans `cloud/supabase` |
| Contract-list work by `contract-engineer` at xhigh, reviewed before merge | T1-T5 | Checkpoints A-C and their reports in `docs/reports/` |
