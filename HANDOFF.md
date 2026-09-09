# HANDOFF

**NEXT SESSION STARTS HERE** — the cloud design spec is being written in quinn-ops docs and will be
moved here; until then see `PROVENANCE.md` for what this repository is, where it came from and what
was renamed on the way in.

## State at the initial import (2026-09-09)

Cut from `quinn-ops` `283d71032a04351db5c2c1de21f528774a7d00f9`. One commit on `main`, no remote.

Measured on the import machine (`stable-x86_64-pc-windows-gnu` 1.98, WinLibs POSIX MSVCRT mingw):

- `cargo build --workspace`: **0 rustc warnings**. The one accepted line is the app's pre-existing
  `.rsrc merge failure: multiple non-default manifests` linker message.
- `cargo test --workspace` (dev profile): **engine 888 passed, 0 failed, 3 ignored** (lib 867, bin 4,
  seven integration suites 17); **app 129 passed, 0 failed, 1 ignored**. The ignored four are named
  in their own `#[ignore]` attributes (traps 4 and 5, the real-runtime smoke test, `run_slot_end_to_end`).
- `cargo build --release --workspace`: `target\release\knowlu-engine.exe` **4,517,376 bytes**
  (4,512,256 before the move — the longer crate name in a handful of strings);
  `target\release\knowlu.exe` **7,666,176 bytes** (unchanged). One release profile, the engine's,
  at the workspace root; the app builds under it as it always did.
- A fresh clone (`git clone` → `cargo test --workspace`) is green with the same counts.

Beyond the move and the renames, three engine tests that had reached outside the crate for the old
repo root's vault files (`config/week_template.yaml` and its `.gitattributes`) now use the fixture
copy and an inline constant; the fixture template is byte-identical to the live one they used, so
nothing they assert changed. `runs::tests::git_sha_reads_this_repo_and_never_propagates` needs a
commit to exist — on a checkout with no `HEAD` it fails, and that is the test working.

## Direction

Accounts, cloud AI and CI releases are the next design
(`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` is the legal groundwork). The local
llama.cpp runtime under `app/src/inference.rs` is scheduled for removal once the cloud design lands.
Plan 2 Part B (the runner leaves) and plan 4b stay in `quinn-ops`'s history as the road here.
