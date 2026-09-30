# Review: ruling 11, the human-actor token (checkpoints A-C)

Branch `human-actor`, base `86a3431`, head `b707c9e`, 10 commits (`4719ff3`..`b707c9e`), 23 files.
Two reviews: the contract-reviewer (contract-list files, whole branch) and the reviewer (app and docs).
Neither ran cargo; the gate counts come from the implementer. `git merge-tree` against `origin/main`
(`97dc27b`) is textually clean. Fix commit: none needed.

## Verdicts

- Checkpoint A (`journal.rs`, `write.rs`, `reconcile.rs`, `sync.rs`): pass.
- Checkpoint B (app and probe): pass.
- Checkpoint C (guard and whole branch): pass, minor findings only.
- Reviewer (non-contract parts): approve, 3 minor fixes, nothing blocks.
- No Critical or Important findings.

## What held

### Contract list (contract-reviewer)

- A vault with no `actor.yaml` reads as `quinn`, so every legacy path writes what it wrote before: the
  console, the CLI defaults, `detect_external`, the judge-once skip reason ("set by quinn") and the
  amend-card Why line (`613ada2`, gated on `LEGACY_HUMAN_ACTOR`, pinned by
  `an_amend_card_without_one_is_byte_identical`).
- `actor_rank`, `human_set` and `human_edited` only widened to `student`, which no production code
  wrote before this branch. The one legacy behaviour change is the ruled one (Q2): a human actor other
  than the vault's token is refused.
- The journal record shape is unchanged, pinned by
  `a_student_record_is_a_quinn_record_with_one_value_changed`. `VIAS` is untouched.
- Reconcile: the synthetic contender's actor feeds only `actor_rank` (both tokens rank 0) and never
  reaches the supersede bytes; a new test asserts this.
- `provenance.rs` is unchanged; the reader refuses `agent:` and `system:` values.
- `human_gate` covers `write_literals`, `create`, `reassert`, `delete`, `move_note` and `append_body`,
  before any record. Agent and `system:` actors are never gated, so `sync`, `judge`, `coursework` and
  approval execution keep their exit-0 contracts.
- `detect_external` checks the file before `load_index`; a bad file does not advance the index.
- Frozen references: 0 bytes changed under `engine/tests/fixtures`; `surface_oracle.rs` untouched; no
  `#[ignore]` touched. Every removed assertion was replaced one for one.

### App, probe and guard

- `account_human_actor` runs after the pending session is read and before `scaffold::create_vault`. A
  `quinn` record anywhere wins and the probe stops at the first; paging uses `restore_all`'s stall guard.
- On failure Finish is refused (Q1): no vault folder, pending session kept. The error never contains the
  bearer (`the_token_is_decided_before_anything_is_written` covers a closed port and a 503).
- `build_into` writes `actor.yaml` create-new before `seed_writes`; seeds take their actor from the file.
  Adopt and backup-folder restore never write it. `config/` never syncs. Credential targets unchanged.
  Test stubs bind `127.0.0.1:0`.
- Every console write uses `console_ctx(vault)`; `executor_ctx` stays `agent:approvals`;
  `week::confirm_argv` passes no `--actor`.
- CLI: `commitments --confirm`, `info`, `issues` and `write` default `--actor`, `--opened-by` and
  `--closed-by` to the vault's token. A bad file prints a named line and exits 2 before any write; the
  `list` commands never read it.
- `human_actor_literal.rs` (lexer for strings, raw and byte strings, chars, lifetimes, nested comments;
  skips `#[cfg(test)]`; exact `"quinn"` only; one allowance) scans `engine/src`, `app/src`,
  `app/static` and `cloud/supabase`. No `"quinn"` literal remains outside `journal::LEGACY_HUMAN_ACTOR`;
  the only other matches are `quinn-ops` (`profiles.rs:167`, legal) and the legacy prose at `write.rs:818`.
- `git diff --check` clean; no `.ps1` or workflow touched.

## Findings and dispositions

| # | Where | Finding | Source | Disposition |
|---|---|---|---|---|
| 1 | `engine/src/write.rs:818` | "Quinn had set them by hand" is legacy-gated and correct, but the guard is exact and case-sensitive, so it cannot catch an unconditional edit (the class `613ada2` fixed). Only the two Why-line tests would. Fix: case-insensitive word scan of non-test literals, this line allowlisted. | contract-reviewer | Minor, open. Follow-up on the guard. |
| 2 | `app/src/onboarding.rs:644` | Every probe error says "check the connection and press Finish again". A revoked refresh token or a 401/402 from `/sync-pull` fails the same way on every retry. Fix: keep the refusal; for a non-transport cause say "sign in again" (the sentence at :633). Also, the doc comment at :577-579 calls `Err` a "closed-set word", but `provider_error` passes server text and `auth_base` includes the URL. | both | Minor, open. Fix the wording and the comment. |
| 3 | `engine/src/journal.rs:192` | A mapping with extra keys is accepted although the text and ruling say one line. `human_actor: student\r\nnote: x\r\n` returns `Ok("student")`. Fix: refuse `map.len() != 1` or change the message. | contract-reviewer | Minor, open. Contract-list file, so contract-engineer. |
| 4 | `docs/reference/engine-commands.md:43` | Synopsis still reads `[--actor quinn]`, contradicting :47, :67 and the code (plan section 7). | both | Minor, open. Docs fix. |
| 5 | `docs/plans/2026-09-29-human-actor-token-plan.md` | Line 6 still says "draft... nothing built". T1 (:185-186) says the file ends in LF, but the build writes `pystr::NEWLINE` (CRLF on Windows, pinned by tests); the plan should say so. The section 7 update to `docs/surface/anatomy.md` ("me" and "you") is not done. | contract-reviewer | Minor, open. Docs fix. |
| 6 | `CLAUDE.md:68-70` | The new invariant omits Q2 (a mismatched human actor is refused, so `write --actor quinn` on a `student` vault exits 2) and "absent means `quinn`". | both | Minor, open. Keep CLAUDE.md near 150 lines. |
| 7 | `engine/src/journal.rs:148` | The bad-`actor.yaml` error reads "... human_actor: student or human_actor: quinn". `console_ctx` passes it to the console, so a student who hand-edits the file sees the founder's name in the UI. Raised by the reviewer for the contract-reviewer. | reviewer | Minor, open. Contract-list file, so contract-engineer; reword without the name. |
| 8 | Process | The checkpoint A and B reports were not on the branch (this report now records A-C). PR #21 (`97dc27b`) rewrote `CREDMAN_LOCK` in `app/tests/onboarding.rs`, which this branch also edits. | contract-reviewer | Rerun `cargo test --workspace` on the merge result before the push; the push needs Quinn's go. |

## Not verified

Neither reviewer ran cargo. The 0-warnings gate and the workspace test counts are the implementer's to
quote; the reviewer found no unused imports or dead bindings by eye.
