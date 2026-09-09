# Provenance

Cut from `quinn-ops` at commit `283d71032a04351db5c2c1de21f528774a7d00f9` (its `main`) on
2026-09-09. **History intentionally not carried:** the old repository holds secrets (a live
Blackboard token and a secret Google Calendar capability URL in its `config/ingest.yaml`, in history
as well as the working tree) and a personal vault in its history. This repository starts at one
commit; the old one remains the record of how the code got here (the reports under `docs/reports/`
and the plans under `docs/plans/` cite its commits).

Every file below was copied from that commit's **working tree** byte for byte, then renormalised to
LF by `.gitattributes` — except `engine/tests/fixtures/`, which is `-text` and holds exactly the
source bytes (CRLF, as the golden files were frozen).

## What moved

| Here | From `quinn-ops` | Notes |
|---|---|---|
| `Cargo.toml` (workspace root) | — | authored: `members = ["engine", "app"]`, resolver 2, the **one** release profile (the engine's size-tuned settings) |
| `Cargo.lock` | `app/Cargo.lock` | it already resolved the engine's graph; cargo settled the two renamed packages |
| `engine/src/`, `engine/build.rs`, `engine/Cargo.toml` | `src/`, `build.rs`, `Cargo.toml` (the crate at the repo root) | package `knowlu-engine`, lib `knowlu_engine`, bin `knowlu-engine` |
| `engine/tests/*.rs` | `tests/*.rs` (7 files) | `site.rs`, `starvation.rs`, `dependency_boundary.rs` re-pointed at `../site`, `../scripts`, `../../Cargo.toml` |
| `engine/tests/fixtures/` | `tests/fixtures/` (64 files) | bytes preserved; the eight frozen references and the three surface references live here |
| `app/` | `app/` minus `target/`, `binaries/`, `gen/`, `Cargo.lock` | package `knowlu`, lib `knowlu`, bin `knowlu`; depends on `knowlu-engine = { path = "../engine" }` |
| `site/` | `site/` | unchanged |
| `scripts/` | 9 of the 24: `release.ps1`, `sign.ps1`, `find-signtool.ps1`, `scratch-vault.ps1`, `console-shots.py`, `mockup-shots.py`, `wizard-check.py`, `settings-check.py`, `starvation.ps1` | paths updated for one `target/` and `engine/tests/fixtures/` |
| `docs/specs/` | `docs/superpowers/specs/`: `2026-08-11-personal-ops-system-design.md`, `2026-09-01-rust-rewrite-design.md`, `2026-09-02-console-on-rust-design.md`, `2026-09-04-knowlu-independent-app-design.md`, `2026-09-05-knowlu-friends-shell-design.md` | historical, unchanged |
| `docs/plans/` | `docs/superpowers/plans/`: `2026-09-04-knowlu-foundation-plan.md`, `2026-09-05-knowlu-runner-leaves-plan.md`, `2026-09-05-knowlu-friends-shell-plan.md`, `2026-09-07-knowlu-judge-and-enrichment-plan.md` | historical, unchanged |
| `docs/notes/` | `docs/superpowers/notes/`: `2026-09-01-product-and-business-plan.md`, `2026-09-01-market-pricing-and-distribution.md`, `2026-08-28-redesign-program.md`, `2026-09-09-knowlu-cloud-legal-landscape.md` | historical, unchanged |
| `docs/reports/` | `docs/superpowers/reports/`: `2026-09-03-console-plan-1-sdd-ledger.md`, `2026-09-05-knowlu-plan-1-sdd-ledger.md`, `2026-09-05-knowlu-plan-2-part-a-sdd-ledger.md`, `2026-09-06-knowlu-plan-4a-sdd-ledger.md`, `2026-09-07-knowlu-plan-3a-sdd-ledger.md`, `2026-09-01-preserved-python-defects.md`, `2026-09-05-tauri-bundle-spike.md`, `2026-09-07-llama-cpp-on-gnu-spike.md`, `2026-09-07-sidecar-protocol-spike.md` | historical, unchanged |
| `docs/surface/` | `docs/surface/{README,anatomy,inventory}.md` | the living reference for the read model; only the names changed (`knowlu-engine judge`, `knowlu_engine::write`, `knowlu.exe`) |
| `docs/procedures/` | `docs/runners/knowlu-go-live.md`, `docs/runners/knowlu-phase-2.md` | unchanged (they still say `quinn-ops` where they describe the old runner) |
| `VISION.md` | `VISION.md` | unchanged |
| `README.md`, `CLAUDE.md`, `HANDOFF.md`, `PROVENANCE.md`, `.gitattributes`, `.editorconfig`, `.gitignore` | — | authored here |

## What stayed behind, deliberately

- The Python engine (`engine/*.py`), `tests/test_*.py`, `conftest.py`, `requirements.txt` — the
  reference implementation the Rust engine was measured against. It is not part of the product.
- The vault: `tasks/ approvals/ archive/ courses/ info/ issues/ profile/ state/ config/ intake/
  views/ .obsidian/ Untitled.base`. A user's vault is data the app is pointed at, never in a repo.
- The cutover and dual-run apparatus: `scripts/cutover/`, `diff-engines.ps1`,
  `diff-engines-notes.ps1`, `dual-run.ps1`, `compare-vaults.ps1`, `local-run.ps1`,
  `rehearse-rollback.ps1`, `rediscover-parity.ps1`, `simulate-days.ps1`, `reverse-proxy.ps1`,
  `diagnose-local-runner.ps1`, `build-vault-full.py`, `lint-yaml-11.py`, `migrate_s1.py`.
- The cloud routine's `docs/runners/cloud-routine-prompt.md` and `proposal-templates.md`, and every
  other spec, plan, report and note.

## Renames made on the way in

Following `docs/plans/2026-09-05-knowlu-runner-leaves-plan.md` Tasks 11 and 12, adapted to the
workspace layout:

- crate `quinn-ops` → `knowlu-engine`, lib `quinn_ops` → `knowlu_engine`, binary `quinn-ops.exe` →
  `knowlu-engine.exe`, clap `name = "knowlu-engine"`, stderr prefix `knowlu-engine:`;
- app crate `quinn-ops-console` → `knowlu`, lib `quinn_ops_console` → `knowlu` (the bin was
  already `knowlu`); path dependency `knowlu-engine = { path = "../engine" }`;
- `QUINN_OPS_BUILD_SHA` → `KNOWLU_BUILD_SHA` (both `build.rs` and both readers);
- `QUINN_OPS_DEVICE` → `KNOWLU_DEVICE` (the env-var override for `journal::device_name`; the plan
  kept the old spelling because the retired cloud sandbox set it — nothing in this product does);
- user-agents: `… x64) Knowlu` (zyBooks, VHL) and `… x64) Knowlu/1.0` (event feeds);
- sidecar `binaries/knowlu-engine` in `tauri.conf.json`, `app/build.rs`'s placeholder,
  `app/tests/static_assets.rs`, `scripts/release.ps1`; `scheduler::engine_exe()` looks for a sibling
  `knowlu-engine.exe`;
- `scripts/scratch-vault.ps1` writes under `%LOCALAPPDATA%\knowlu\scratch\` and requires `-Source`;
- test-only literals (`knowlu/zybooks`, temp-dir prefixes, argv `[0]`) and comments.

**What did not change, because it is a contract with existing vaults or installs:** every on-disk
format (`journal::VIAS`, journal and run records, note frontmatter, the ledgers), `KNOWLU_ENGINE_EXE`,
the app-data root `%LOCALAPPDATA%\knowlu` **and** the fold from the old `%LOCALAPPDATA%\quinn-ops`
root (`profiles::migrate_flat_layout` still names it — that string is the only `quinn-ops` left in
`app/src`, and it must stay), the identifier `com.knowlu.desktop`, and the credential targets
`knowlu/<profile_id>/<source>`.
