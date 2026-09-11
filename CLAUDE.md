# Knowlu — Claude working rules

Knowlu is a desktop app for students that answers one question every morning: **what should I work
on today, and in what order?** Rust + Tauri, Windows, desktop-only. This repository is the product:
`engine/` (`knowlu-engine`) and `app/` (`knowlu`) in one Cargo workspace. Where the code came from
and what was left behind: `PROVENANCE.md`. Where the work stands: `HANDOFF.md`.

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
- `docs/specs/2026-08-11-personal-ops-system-design.md` — the parent design (vault schema, ranking,
  rendering; language-neutral, authoritative); `2026-09-01-rust-rewrite-design.md` (crate budget, the
  oracle); `2026-09-02-console-on-rust-design.md` (the read model); `2026-09-04-knowlu-independent-app-design.md`
  and `2026-09-05-knowlu-friends-shell-design.md` (the app, onboarding, profiles, releases).
- `docs/notes/2026-09-01-market-pricing-and-distribution.md` (§0 first), then
  `2026-09-01-product-and-business-plan.md` (§12's rulings win over its body); the most recent ruling wins.
- `docs/surface/anatomy.md` — the read model behind the window; `app/README.md` — the app itself.

## Engine invariants

- A vault is markdown + YAML frontmatter (`tasks/`, `approvals/`, `archive/`, `courses/`, `info/`,
  `issues/`, `config/`), the single source of truth. `state/` is generated; `today.md` is rewritten
  every run. The engine is **deterministic**: same input, same order.
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
- All JSON the crate writes goes through `ledger::dumps_value` (Python `json.dumps` separators), so
  a new line and an old line carrying the same data are the same bytes.
- `journal::VIAS`, run records, ledgers and note frontmatter are contracts with existing vaults:
  byte-identical, never renamed.

## The engine's commands (`knowlu-engine`, `engine/src/main.rs`)

- `rank --vault <v> [--today YYYY-MM-DD] [--runner manual|local|cloud] [--run-id <id>]`
- `coursework --vault <v> [--dry-run] [--via <via>] [--run-id <id>]` — zyBooks + VHL into `tasks/`.
  Always exits 0. An empty parse is a failure, never an empty semester. Passwords come from Windows
  Credential Manager via the vault's `credential_target`; zyBooks 403s without a `User-Agent`; VHL is
  CAS with a one-time `lt` ticket and a dashboard on `m3a.vhlcentral.com`.
- `coursework-discover [--vault <v>] [--zybooks-target <t>] [--vhl-target <t>]` — read-only: the
  zyBooks books and VHL sections the stored logins can see, as JSON (`errors`, `vhl`, `zybooks`, each
  row marked `mapped` against the vault's `course_map`). Always exits 0; the wizard's mapping rows
  come from it, and it writes nothing.
- `ingest --vault <v> [--via <via>] [--run-id <id>]` — the LMS `.ics` feed into `tasks/`. Exits 1 on
  an empty `ics_url`, which is why the app leaves the step out rather than run it.
- `judge --vault <v> [--via <via>] [--run-id <id>] [--runtime <llama-cli.exe>] [--model <.gguf>]
  [--log-dir <dir>] [--limit N]` — enriches tasks flagged `needs_enrichment: true` in three tiers
  (heuristics, promoted rules, the model — one process per judgment). **Always exits 0**: no runtime
  and no model are normal outcomes. Writes as `agent:knowlu.enrich` (`provenance::is_agent` is a
  `starts_with` test) with `judged: true`, `propose: true`. Judgment logs never enter the vault.
- `surface --vault <v> --view today|overdue|week|later|all|decisions|good-to-know|issues|runs
  [--today] [--now] [--seen-at] [--build-sha]` — the read model as JSON. Never writes.
- `runs`, `info`, `issues`, `write` (`--actor`, `--via` from `journal::VIAS`) — run records, info
  items, issue notes, journaled note edits.

## Knowlu (the app)

- `app/src/commands.rs` computes nothing itself; every vault write goes through the engine's `write`
  with `console_ctx()` (`via: "dashboard"`). **Tauri commands, recounted 2026-09-10** from the two
  `generate_handler!` lists in `app/src/main.rs`: the console window registers **43**, the vault-less
  picker/wizard window **27** — 59 distinct. Commands live beside the module they serve
  (`commands.rs`, `onboarding.rs`, `account.rs`, `report.rs`, `lms_link.rs`), never all in one
  file. Seven mutate notes
  (`set_fields`, `create_task`, `delete_note`, `decide`, `close_info`, `open_issue`,
  `resolve_issue`); `sync`/`backup_now` move the vault without writing a note; `ui_event` writes the
  `state/events-ui/` ledger; everything else touches app data, `profiles.json`, the clipboard, the
  process or the updater — never a note. Recount before quoting a number.
- **App data is `%LOCALAPPDATA%\knowlu\`**: `profiles.json`, `profiles\<profile_id>\{settings.json,
  seen.txt, logs\}`, shared `updates\`, `runtime\`, `models\`. `state::app_data_root()` is the one
  place the path is decided. `profiles::migrate_flat_layout` still folds an old flat
  `%LOCALAPPDATA%\quinn-ops\` root in, file by file — that literal is the only `quinn-ops` left in
  `app/src`, and it stays.
- A slot is `coursework → ingest → judge → rank` (`scheduler::slot_argv`), each the sibling
  `knowlu-engine.exe` as a child process (`KNOWLU_ENGINE_EXE` overrides). Steps are left out and
  named — `ingest (skipped: no ics_url)`, `judge (skipped: no runtime)` / `(skipped: no model)` —
  never run-and-failed: a non-zero step means retry backoff and an amber tray. The scheduler is inert
  unless the vault's `config/runners.yaml` `local` entry says `scheduler: app` for this `device:`; a
  wizard-created vault carries both from birth.
- Credentials the app writes are `knowlu/<profile_id>/<source>` (`app/src/credentials.rs`); the
  engine's `wincred.rs` reads whatever `credential_target` the vault names.
- The account's session JWT is Credential Manager's `knowlu/<profile_id>/session`
  (`app/src/account.rs`), moved there at onboarding from a pre-vault `knowlu/pending/session` entry;
  `config/cloud.yaml` names it alongside the project's `api_base`, its public `anon_key` and the
  `account_id`. Entitlement is cached at `profiles\<id>\entitlement.json` with a 72-hour grace, and
  past it every cloud step is a named skipped step, never a failure.
- The identifier is **`com.knowlu.desktop`**, permanent: uninstall key, autostart entry and window state are keyed by it.
- The updater is configured: `tauri-plugin-updater`, `plugins.updater` (endpoint + minisign public
  key) and `bundle.createUpdaterArtifacts: true` are one decision — a static test pins flag ⇔ plugin.
  The private key exists **only** as the GitHub secret `TAURI_SIGNING_PRIVATE_KEY` (C0 Task 4
  regenerates it; the laptop's old Credential Manager copy is retired); `release.ps1` takes it from
  the environment and nowhere else. If it is ever lost, regenerate: one `pubkey` line and a release,
  and installed apps need one manual reinstall.
- `app/src/inference.rs`: `SUPPORTED_RUNTIMES` is a compiled-in table of release tag, asset name and
  SHA-256; both install paths verify against it, so there is no unverified path to executing a
  runtime. Adding a release is a code change. Model files are data, not pinned. Not bundled.
- Plain `cargo build` / `cargo test` work on a fresh checkout because `app/build.rs` drops a
  zero-byte placeholder sidecar at `app/binaries/knowlu-engine-<triple>.exe`. **Releases are CI-only**
  (`.github/workflows/release.yml`, on a `v*` tag): `scripts\release.ps1` is what CI runs; a human runs
  it only with `-DryRun`, which bundles unsigned and publishes nothing. A hand-run `cargo tauri build`
  is unsupported — it skips the clean-tree gate, the sidecar staging and the placeholder check, and
  can ship a zero-byte engine. `ci.yml` is the gate on every push and PR: `cargo test --workspace` at
  0 warnings (the gate prints `warnings: N accepted (.rsrc), N tallies, N other`), the eol contract
  (`scripts/ci/eol-check.ps1`), SHA-pinned actions (`engine/tests/workflows.rs`).
- Desktop safety: a live shared desktop — never synthetic keyboard/mouse input; screenshots by
  window handle (`PrintWindow`) only, never a full-screen grab. Develop and demo against scratch
  vaults (`scripts\scratch-vault.ps1 -Source <vault>`), never a live one.

## Toolchain and conventions

- `cargo build --workspace` and `cargo test --workspace` from the root. **0 warnings is part of
  green.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple non-default
  manifests` linker message. Four tests are `#[ignore]` by design, each with its reason in the
  attribute (traps 4 and 5 in `engine/src/events.rs`; `runtime.rs`'s real-runtime smoke test, run by
  hand with `KNOWLU_RUNTIME` and `KNOWLU_MODEL` set; `app/tests/scheduler.rs::run_slot_end_to_end`).
  None may be un-ignored by changing the assertion. TDD: the test first, then the code.
- Toolchain `stable-x86_64-pc-windows-gnu` (1.98); TLS via `rustls`/`ring`, **never OpenSSL**
  (`engine/tests/dependency_boundary.rs` pins it, and that `tauri` never enters the engine). **The
  GNU host needs mingw-w64 binutils** — rustup's `self-contained/` lacks `as`, so `windows-*` crates
  fail with `dlltool ... CreateProcess` without WinLibs POSIX **MSVCRT** (`winget install
  BrechtSanders.WinLibs.POSIX.MSVCRT`; MSVCRT because the GNU target links msvcrt). If a build fails
  with `failed to find tool "gcc.exe"`, the shell's PATH is stale — refresh it:
  `$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"`
- The **one** release profile is the workspace root's (`opt-level = "z"`, `lto`, `codegen-units = 1`,
  `panic = "abort"`, `strip`); Cargo ignores `[profile.*]` in members. **`cargo test --release` will
  not link** — test in the dev profile.
- `ureq` is built with its non-default **`cookies`** feature and VHL does not work without it: CAS
  login on `www.vhlcentral.com`, dashboard on `m3a.vhlcentral.com`, one jar scoped to `.vhlcentral.com`
  — `vhl::default_opener` owns one agent; a fresh agent per request reads as a dead session.
- **`tauri-plugin-dialog` is unusable on this toolchain**: its `rfd` hard-codes `common-controls-v6`,
  which imports `TaskDialogIndirect` from `comctl32.dll` by name and fails to load with
  `STATUS_ENTRYPOINT_NOT_FOUND` before `main`. The folder picker is **`rfd 0.16` with
  `default-features = false`**, called from an app command; message boxes are `MessageBoxW`.
- `cargo-bloat` needs a non-LTO audit build (`CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_LTO=false
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo bloat --release --crates`) or everything lands in
  `[Unknown]` — and it replaces `target/release`; `cargo build --release` again before quoting a size.
- **Tests that touch the real Credential Manager are serialised.** Windows races parallel
  `CredWriteW`/`CredReadW` calls (spurious `ERROR_NOT_FOUND`), so `app/tests/account.rs` holds a
  file-scoped `CREDMAN_LOCK` mutex and every test that writes, reads or deletes a real credential
  takes it, under a generated test id with a `Drop` guard that deletes what it wrote. A new test file
  that touches the store carries its own lock.
- **Line endings: LF everywhere in this repo** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` are
  CRLF). `engine/tests/fixtures/**` is `-text`: those bytes are the contract — several are compared
  byte for byte and they are CRLF because vaults are — **never re-encode them**. The engine still
  translates CRLF on every vault read and write (`pystr`): users' vaults are whatever they are.
  `str::lines()` strips a trailing `\r`; `split('\n')` does not.
- Workflow: brainstorm → spec (`docs/specs/`) → plan (`docs/plans/`) → execute with review
  checkpoints; every plan carries a fidelity ledger and every review lands in `docs/reports/`. Quinn
  reviews at checkpoints — surface trade-offs, ask before assuming.

## Direction (signed 2026-09-09)

- The authority is `docs/specs/2026-09-09-knowlu-cloud-design.md`: its §1 decisions D1–D12 are
  Quinn's and signed; §11's recommendations are ruled (R3: the academic-year price and the June–August
  pause both stay). Where an older spec or note disagrees with it, the cloud design wins.
- In one line: **accounts + $9.99/month, no free tier; every judgment runs in our cloud (Supabase +
  Cloudflare + Stripe, Anthropic API); CI builds and signs every release; the vault stays plain
  text on the student's machine and is created by the app; portal credentials never leave the
  device — fetch on device, think in the cloud.**
- The work is streams with disjoint files (`HANDOFF.md` §2): C0 CI release → C1 accounts, wizard,
  telemetry → C2 the judgment service → C3 sync (git leaves the product) → C4 removal of the local
  llama.cpp runtime (`app/src/inference.rs`, `engine/src/runtime.rs`). Until C4 lands, that runtime
  code stays and is not extended.
- Cut day (spec §7.2) is a procedure with Quinn at the machine: the old `quinn-ops` vault is archived,
  not migrated; Quinn re-onboards into `%USERPROFILE%\Knowlu\`.
