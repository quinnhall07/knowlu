# Knowlu — Claude working rules

Knowlu is a desktop app for students that answers one question: **what's next?** (`VISION.md`) Rust + Tauri, Windows, desktop-only. This repository is the product:
`engine/` (`knowlu-engine`) and `app/` (`knowlu`) in one Cargo workspace, plus `cloud/` (Supabase).
Where the work stands: `HANDOFF.md`. Where the code came from: `PROVENANCE.md`.

> ## Two rules that override the rest
>
> 1. **Add no single-user assumptions.** Anything that would need hand-editing for a second user is
>    a bug. Nothing here names a person's vault, machine, account or credential.
> 2. **Never regenerate a frozen reference.** Eight Python-written references live in
>    `engine/tests/fixtures/`: `golden-today-s1.md`, `golden-today-full.md`,
>    `calendar-snapshot-gcal.md`, `vault-full/state/events.md`, `zybooks-parsed-reference.json`,
>    `vhl-parsed-reference.json`, `run-records-reference.json`, `pyyaml-safe-dump-reference.json`.
>    If the engine disagrees with one, the engine is wrong. The three Rust-generated read-model
>    references — `surface-today-{s1,s1-migrated,full}.json` (`engine/tests/surface_oracle.rs`) —
>    have their own rule (console spec §4.6): regenerate only in a commit whose diff shows the change
>    and whose message says why.

## Read first

- `HANDOFF.md`, then `VISION.md` — check every decision against it.
- `docs/specs/2026-09-09-knowlu-cloud-design.md` is the authority: its §1 decisions and amendments are
  Quinn's and signed; where an older spec or note disagrees, it wins. Older design authority:
  `2026-08-11-personal-ops-system-design.md` (vault schema, ranking, rendering),
  `2026-09-01-rust-rewrite-design.md`, `2026-09-02-console-on-rust-design.md`,
  `2026-09-04-knowlu-independent-app-design.md`, `2026-09-05-knowlu-friends-shell-design.md`.
- Reference, read when the task touches it: `docs/reference/engine-commands.md` (every command's
  flags and contract), `docs/reference/app.md` (commands, app data, scheduler, sign-in, updater,
  releases, CI), `docs/reference/toolchain.md` (build setup and known traps), `docs/surface/anatomy.md`
  (the read model), `app/README.md`, `cloud/supabase/README.md`.
- Business context: `docs/notes/2026-09-01-market-pricing-and-distribution.md` (§0 first), then
  `2026-09-01-product-and-business-plan.md` (§12's rulings win over its body); the most recent ruling wins.

## Engine invariants

- A vault is markdown + YAML frontmatter (`tasks/`, `approvals/`, `archive/`, `courses/`, `info/`,
  `issues/`, `config/`, `commitments/`), the single source of truth. `state/` is generated;
  `today.md` is rewritten every run. The engine is **deterministic**: same input, same order.
  `commitments/` holds confirmed clock-time commitments (a class, a shift, a club), decline
  markers and the one `planning-day` note (the day's wake-to-bed window) — a student's `propose`d
  and undecided candidates never live here, only what they confirmed or declined.
- **`rank` never calls a model** (Knowlu spec decision 11). Judgment is the separate `judge`
  command, which writes fields into notes before `rank` reads them; `src/judge.rs` is pure and the
  model process lives behind a trait in `src/runtime.rs`, so nothing under `cli.rs` can reach one.
- Never rewrite a vault file wholesale. Every note write goes through `write`: journal record first
  (`state/journal/YYYY-MM-DD.jsonl`, UTC days), single-line frontmatter surgery second. **No note is
  ever parsed and re-dumped**; `src/yamlemit.rs` is the one YAML emitter and deliberately has no
  anchor or alias support. Every note has an opaque `id:`; `source_uid` is the external key.
- Judge once, re-propose freely: an agent never re-sets a field the journal shows the user set; with
  `propose` it files a `kind: amend` approval instead. `judgment:` is a single-line flow mapping.
- Approvals are capped at 15 new proposals a day; overflow is snoozed, never deleted. `proposed_at`
  is the day a proposal charges; `first_proposed_at` is set once and drives every age.
- Commitment proposals and `state/calendar-series.json` never leave the device: a card of a kind in
  `commitments::LOCAL_CARD_KINDS` (`commitment-ask`, `commitment-check`; both are filed, and
  no proposal card is filed on the vault's first day — the vault-local date of its earliest journal
  record, `commitments::vault_day`) is local by kind. `sync` (P21) keeps those cards, every record
  about one and the series file off the wire — only a **confirmed** note in `commitments/` or a
  decline marker syncs — and `commitments.rs`'s tripwires (`sync_keeps_every_local_card_kind_local`,
  `the_servers_note_path_rules_name_every_note_folder`) fail if that ever stops holding.
- All JSON the crate writes goes through `ledger::dumps_value` (Python `json.dumps` separators), so
  a new line and an old line carrying the same data are the same bytes.
- `journal::VIAS`, run records, ledgers and note frontmatter are contracts with existing vaults:
  byte-identical, never renamed.
- Agent actors start `agent:` (`judge` writes as `agent:knowlu.enrich`, completion detection as
  `agent:knowlu.completion`, commitment cards as `agent:commitments`); `provenance::is_agent` is a
  `starts_with` test, so an actor without the prefix reads as the user and "judge once" breaks.

## Command and app contracts

Full detail in `docs/reference/`; these are the parts a change must not break.

- `coursework`, `coursework-discover`, `judge` and `sync` **always exit 0**: no runtime, model,
  account, session, entitlement or network is a named outcome, never a failure. An empty coursework
  parse is a failure, never an empty semester. `surface` (`--window` included) and
  `coursework-discover` never write. `commitments` without `--confirm` always exits 0 and writes only
  the generated `state/calendar-series.json`; with `--confirm` it writes as the student through
  `write`, journal first, and exits 2 having written nothing on bad input. Judgment logs never enter
  the vault.
- **The engine gates itself:** `coursework`, `ingest`, `judge` and `sync` do not run past the 72-hour
  entitlement grace (`engine/src/entitle.rs`); the refusal is a named line at exit 0. `rank`,
  `surface` and `write` are never gated.
- Cloud calls from `ingest`, `coursework` and `rank` are **transport only**, never judgment. Judgment
  is `judge`'s tier 3 through `CloudModel` (`engine/src/cloudmodel.rs`); the prompt, schema and pinned
  model live server-side.
- A slot is `sync → coursework → ingest → judge → rank`. A step that cannot run is left out and named
  (`judge (skipped: no entitlement)`), never run-and-failed; a non-zero step means backoff and an
  amber tray.
- `app/src/commands.rs` computes nothing; every vault write goes through the engine's `write` with
  `console_ctx()`. A new Tauri command goes in the right `generate_handler!` list in `app/src/main.rs`
  and beside the module it serves; recount before quoting a number.
- Credentials the app writes are `knowlu/<profile_id>/<source>` in Credential Manager, the session JWT
  `knowlu/<profile_id>/session`; the engine's `wincred.rs` reads whatever `credential_target` the
  vault names and never assumes that shape. There is no password: sign-in is Google (loopback PKCE on
  `127.0.0.1:0`) or an emailed code. App data lives under `%LOCALAPPDATA%\knowlu\`, and
  `state::app_data_root()` is the one place that path is decided. `profiles::migrate_flat_layout`
  keeps the one `quinn-ops` literal in `app/src` (it folds an old flat install in); it stays.
- The identifier `com.knowlu.desktop` is permanent. The updater's private key exists only as the
  GitHub secret `TAURI_SIGNING_PRIVATE_KEY`; `release.ps1` reads it from the environment and nowhere
  else, and `release.yml` never runs self-hosted.
- **Releases are CI-only** (`release.yml` on a `v*` tag). A human runs `scripts\release.ps1` only with
  `-DryRun`; a hand-run `cargo tauri build` is unsupported and can ship a zero-byte engine.
- `app/src/inference.rs` and `engine/src/runtime.rs` (the local llama.cpp runtime) stay until the
  Pilot's runtime-removal lane (Amendment 2026-09-29, ruling 10) removes them and are not extended.
- Desktop safety: a live shared desktop — never synthetic keyboard or mouse input; screenshots by
  window handle (`PrintWindow`) only. Develop and demo against scratch vaults
  (`scripts\scratch-vault.ps1 -Source <vault>`), never a live one.

## Model and effort

The session default is Opus 5.5 at `medium`, capped at `xhigh` (`.claude/settings.json`). The main
session does everyday work and delegates a separable piece to a subagent in `.claude/agents/` when that
is cheaper or safer; each agent's `description:` says when. Rationale: `docs/notes/2026-09-29-model-and-effort-hierarchy.md`.

| Agent | Model, effort | Use for |
|---|---|---|
| `explorer` | Haiku, low | read-only search, before editing |
| `mechanical` / `test-writer` / `console-ui` | Sonnet, low / medium / medium | specified edits / the failing test / `app/static`, `site/` |
| `implementer` / `integrator` | Sonnet, high / Opus, high | specified feature tasks off the contract list / the merge train |
| `planner` / `debugger` / `cloud-engineer` / `reviewer` | Opus, high | specs and plans / root cause / `cloud/` / pre-push review, and spec/plan review |
| `contract-engineer` / `contract-reviewer` | Opus, xhigh | the contract list / contract-list diffs |
| `researcher` / `docs-keeper` | Sonnet, medium / medium | read-only gap research / HANDOFF, docs/reference, ledgers |

**The contract list** — cheaper agents never edit it:
`engine/src/{write,journal,yamlemit,yaml,pystr,ledger,ids,provenance,approvals,sync,entitle,wincred,reconcile}.rs`,
`app/src/{credentials,account,updates}.rs`, the oracle/sync/entitlement tests, `engine/tests/fixtures/**`.

**No agent fits? Ask in order:** could a silent error corrupt vault bytes, leak a credential or
student data, move money or break a release (Opus; `xhigh` on the contract list, `high` elsewhere)?
Is a design choice open, or more than about three files touched (Opus `high`, plan first)? Fully
specified and checked by the compiler or a test (Sonnet, `low` to `high`)? Read-only (Haiku, `low`)?
The main session orchestrates; it edits only what fits on one screen. Use the closest agent rather than
inventing one. Subagents don't see the conversation: give them the goal, the files and the
constraints. Anything a cheaper agent changed goes through `reviewer` before a push. Two failed
attempts means one level up, never straight to `max`; Fable and `max` are Quinn's call.

## Gates and conventions

- `cargo build --workspace` and `cargo test --workspace` from the root, dev profile
  (`cargo test --release` will not link). **0 warnings is part of green**; the one accepted line is
  the app's `.rsrc merge failure: multiple non-default manifests`. `ci.yml` gates every push and PR:
  the workspace tests (its line reads `warnings: N accepted (.rsrc), N tallies, N other`), the eol
  contract (`scripts/ci/eol-check.ps1`) and SHA-pinned actions (`engine/tests/workflows.rs`).
- Four tests are `#[ignore]` by design, each with its reason in the attribute: traps 4 and 5 in
  `engine/src/events.rs`, `runtime.rs`'s real-runtime smoke test, and
  `app/tests/scheduler.rs::run_slot_end_to_end`. None may be un-ignored by changing the assertion. TDD: the test first, then the code.
- TLS via `rustls`/`ring`, **never OpenSSL**; `tauri` never enters the engine
  (`engine/tests/dependency_boundary.rs`). Build trouble: `docs/reference/toolchain.md`.
- **Tests never leave the machine**: a test that needs a server binds a listener to `127.0.0.1:0` and
  serves itself — no egress, no name resolution, no listener on a routable interface.
- **Tests that touch the real Credential Manager are serialised**: each test file that writes, reads
  or deletes a real credential holds a file-scoped `CREDMAN_LOCK` mutex, uses a generated test id and
  cleans up with a `Drop` guard (`app/tests/account.rs`).
- **Line endings: LF everywhere** (`*.ps1` are CRLF). `engine/tests/fixtures/**` is `-text`: those
  bytes are the contract, CRLF because vaults are — **never re-encode them**. The engine translates
  CRLF on every vault read and write (`pystr`). `str::lines()` strips a trailing `\r`;
  `split('\n')` does not.
- Workflow: brainstorm → spec (`docs/specs/`) → plan (`docs/plans/`, with a fidelity ledger) →
  execute with review checkpoints; reviews land in `docs/reports/`. Quinn reviews at checkpoints —
  surface trade-offs, ask before assuming.

## Direction

`VISION.md` states what Knowlu is and must stay; the cloud design is the authority on how. In one line: **accounts
and $9.99 a month, no free tier; every judgment runs in our cloud; CI builds and signs every release;
the vault stays plain text on the student's machine and is created by the app; portal credentials
never leave the device — fetch on device, think in the cloud.** Streams and their order are in
`HANDOFF.md` §2.
