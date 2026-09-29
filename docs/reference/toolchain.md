# Toolchain and conventions

Moved verbatim out of `CLAUDE.md` on 2026-09-29 so the file every session loads stays short.
`CLAUDE.md` keeps the rules; this file keeps the reference. Read it when a build fails or before touching the toolchain.

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

