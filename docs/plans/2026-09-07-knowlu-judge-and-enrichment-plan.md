# Knowlu — the judge seam, the runtime, and enrichment — Implementation Plan (Knowlu plan 3a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: EXECUTED 2026-09-07 on branch `worktree-knowlu-plan-3a` (Tasks 1–12).** Written 2026-09-07, **revised 2026-09-07 (revision 1)** after an independent review — five blocking, nine should-fix and ten minor findings applied, with two controller rulings recorded as **R-P3a-1** (the deviation from `bundle.externalBin` stands, but on optionality, independent versioning and consistency with the models — never on installer size) and **R-P3a-2** (a compiled-in table of supported upstream runtime digests: there is no unverified path to executing a runtime binary). **Twelve tasks.** This is the **first of three plans** the Knowlu spec's plan 3 ("judgment comes home") splits into: **3a — the judge seam, the runtime, and enrichment** (this one); **3b — event verdicts and rule promotion**; **3c — Gmail, after which the routine is turned off**. Nothing here edits the cloud routine, its prompt, or `docs/runners/cloud-routine-prompt.md`; nothing here moves G2 or `$mode`. **Task 1's `Outcome:` letter — the one everything downstream was written against — came back B** (one process per call, `llama-cli.exe`, not a loopback server: the measured 2.15× did not clear the plan's pre-committed 3× bar), so Tasks 3 and 4 built to the Outcome B branch given in full in Task 1's own step 9 rather than the Outcome A default; every other task ran as written. Full measurement: `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`.

**Goal:** At its own slots the app enriches every task carrying `needs_enrichment: true` — locally, with no cloud call — so that the cloud routine's step 3 wakes up and finds nothing left to do; and it says so plainly, without failing a slot or painting the tray amber, when no model and no runtime are installed.

**Architecture:** Judgment becomes a first-class engine command. Four new engine modules — `src/judge.rs` (pure: the item, the verdict, the three tiers, the grammar, the prompt, the reply parser), `src/runtime.rs` (the llama.cpp process: locate, start, health-check, one call at a time, bounded, killed on drop), `src/judgelog.rs` (ids and field values only, into the profile's app data, never the vault), `src/enrich.rs` (select → judge → judged write → amend card → log) — plus `quinn-ops judge`, which the app's slot runs between `ingest` and `rank`. `src/write.rs` finally grows the judge-once re-proposal path the Python engine has always had and the port left as `WriteError::ProposeNotPorted`. On the app side one new module, `app/src/inference.rs`, locates the runtime and the model under the app-data root, installs either from a file the user points at or from a signed-by-hash manifest, and never installs anything automatically. The model sits behind a trait; every test drives a scripted fake.

**Tech Stack:** Rust 1.98 `stable-x86_64-pc-windows-gnu`; the engine crate's existing `ureq 3.4` (loopback HTTP to the runtime process) and `serde_json`; llama.cpp's prebuilt `llama-server.exe` as a consumed release artefact, GBNF grammar-constrained decoding; Tauri 2 in `app/`, with `sha2 0.10` and `zip 2` (both already in `app/Cargo.lock` through the graph) for verify-then-install; plain HTML/CSS/JS in `app/static/`; PowerShell 5.1 for `scripts/starvation.ps1`.

**Spec:** `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` — §5.1 (starve, never switch), §5.2 (one judge seam, three tiers), **§5.3 as amended 2026-09-07** (the runtime is a separate process, not a linked library), §5.4 (the judgment log and the rule-promotion loop), §5.5 (enrichment first), §5.6 (data minimisation), §8 (test seams), §10 decisions 4, 11, 12. The amendment's evidence is `docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md`.

## Global Constraints

Every task's requirements implicitly include this section. The first block is copied verbatim from the brief; the second is this plan's own gloss on it, added where the constraint needed a boundary drawn.

- Scratch copies only; never the live vault; never an interactive launch of `knowlu.exe`.
- No network in any test. No model download in any test.
- Every note write through `quinn_ops::write`; `rank` never calls a model; the read model never writes.
- `cargo test` at the repo root at **0 warnings**, `tests/oracle.rs` and `tests/surface_oracle.rs` green, and **both** `scripts\diff-engines.ps1` (all three fixtures) and `scripts\diff-engines-notes.ps1` exiting 0 — this plan touches `src/`, so that gate binds every task that does. `cd app; cargo test` at zero new warnings (the `.rsrc` linker line is pre-existing).
- The eight Python-written references and the three `surface-today-*.json` are never regenerated.
- No new `uievents::ACTIONS` action; no `http://`/`https://` under `app/static/`.
- No single-user assumption. No secret in any log, note, fixture or test name.
- Line endings per file; `git diff --stat` never shows a whole-file flip.
- Commits: specific `git add`, message via `-F <file>`, trailers `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4`.
- PowerShell 5.1 only in scripts. Build PATH note for mingw. `cargo test --release` will not link.

**This plan's gloss on those, so no task has to guess:**

- **"No network in any test" means no egress and no name resolution.** A `TcpListener` bound to `127.0.0.1:0` *inside one test*, answering that same test's own request from a second thread, is the seam `runtime::Server`'s HTTP client is exercised through, and is not egress: no DNS, no route off the machine, no listener on a routable interface. Every test that does this binds to `127.0.0.1`, never `0.0.0.0`, and joins its listener thread before returning. Nothing else in this plan opens a socket in a test.
- **The judge never fails a slot.** `quinn-ops judge` **always exits 0** — a missing runtime, a missing model, a runtime that will not start, a model that answers nonsense are all normal outcomes reported on stdout. A non-zero exit would set `RunSummary.engine_ok = false`, which paints the tray amber and puts the slot into retry backoff twice a day forever (spec §5.3: "the app runs with no model present"; brief decision 4). `coursework` already exits 0 for the same reason; `ingest` does not, and that difference is deliberate on both sides.
- **Agent actors start with `agent:`.** The three this programme uses are `agent:knowlu.enrich` (3a), `agent:knowlu.events` (3b) and `agent:knowlu.gmail` (3c), mirroring the routine's `agent:routine.enrich`. The spec §5.2's `knowlu/enrich` spelling is a defect — `provenance::is_agent` is `actor.starts_with("agent:")`, so a `knowlu/…` actor would silently skip judge-once and write no provenance block. Task 12 amends the spec.
- **`journal::VIAS` does not grow.** The judge step passes `--via local-runner`, exactly as `coursework` and `ingest` do from `scheduler::slot_argv`.
- **The model and the runtime never enter the vault, the backup or a journal record.** They live under the app-data root. The judgment log lives under the *profile's* app data. `backup::tick` mirrors the vault and nothing else, so none of it is ever copied.
- **The prompt may carry note content; the log may not.** §5.6: judgment logs hold ids, field values and confidences — never a body, never an event description. A test proves a distinctive body token never reaches a log line.
- **No path, machine or person is named in anything that ships.** Every script, test, module doc
  and page string derives its paths from an argument, an environment variable or the app-data
  root. The one exception is Task 1's **throwaway spike**, whose scratch directory under `%TEMP%`
  is spelled out because it is deleted in the task's last step and reaches no committed file.
- **Nothing in this plan edits the cloud routine, `config/runners.yaml`'s `scheduler:` key, `$mode`, `engine/`, or any name on the never-rename list.** Both producers running at once is the expected state for weeks: whichever runs second finds `needs_enrichment: false` and does nothing.

---

## Fidelity ledger — decisions and rulings this plan must not lose

| # | Decision / ruling | Source | Carried by |
|---|---|---|---|
| D1 | The routine is **starved, never switched**: the app does the work first, at its own slots, and the routine finds nothing. Its prompt and config are never edited during the transition | spec §5.1, decision 3 | Tasks 7, 8 (the app enriches at its slots); Task 11 (the observation script); no task touches `docs/runners/cloud-routine-prompt.md` |
| D2 | **One judge seam, three tiers**: heuristics, promoted rules, the model — and the model only when 1–2 report low confidence and only if one is installed | spec §5.2, decision 11 | Task 2 (`judge::judge_task`, `tier1`, `trait Rules`/`NoRules` as 3b's seam), Task 4 (tier 3) |
| D3 | Every judgment is a **judged write by an agent actor**, so judge-once holds and the app's later opinion arrives as a `kind: amend` card | spec §5.2, decision 11 | Task 6 (`write::propose_amendment`), Task 7 (`WriteOpts { judged: true, propose: true, inputs }`, actor `agent:knowlu.enrich`) |
| D4 | **`rank` never calls a model.** The engine stays deterministic in the sense that matters | spec §5.2, decision 11 | The judge is its own command and its own slot step; `src/cli.rs` is not touched by any task; Task 9's step order is coursework → ingest → judge → rank |
| D5 | The judgment work moves to **local models, never a cloud API** | spec decision 4 | Task 3 (a loopback process on `127.0.0.1`); no task adds an API key, a token or a remote inference endpoint |
| D6 | llama.cpp is consumed **as a separate process**, not linked; grammar-constrained decoding on every call without exception | spec §5.3 as amended 2026-09-07; the spike report | Tasks 1, 3, 4; `judge::GRAMMAR` is passed on every call and a test proves it |
| D6a | **There is no unverified path to executing a runtime binary** (ruling R-P3a-2): the app compiles in a table of supported upstream release digests (tag, asset, SHA-256), **both** install paths check against it, and an unrecognised digest is refused with the computed digest printed. A manifest is a convenience, never the root of trust. A model is data, not an executable, and is checked against the manifest's digest when it came from one | ruling R-P3a-2 | Task 1 step 1 (measures the digest), Task 8 (`SUPPORTED_RUNTIMES`, `check_runtime_supported`, `install_runtime_from_zip`) + `a_runtime_whose_digest_is_not_in_the_table_is_refused_by_the_production_path`, Task 10 (both commands go through `inference::install_from_*`), Task 12 step 8 (refuses to close on an empty table unless the spike said Outcome C) |
| D6b | The runtime is **not bundled** — because it is **optional**, **versions independently** and is **consistent with the models**, and **not** because of installer size; the repo's size figure is the product plan's **+20–50 MB** (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`) plus Task 1's measured byte size | ruling R-P3a-1 | Task 3's module doc, Task 8's module doc, Task 12 step 1's §5.3 amendment and step 8's measurement check |
| D7 | **The app runs with no model present.** "Model not installed" and "runtime not installed" are normal outcomes, reported once, never a failed slot and never an amber tray | spec §5.3, brief decision 4 | Task 2 (`Outcome::{ModelNotInstalled, RuntimeNotInstalled}`), Task 7 (exit 0 always), Task 9 (`JudgeState::{NoRuntime, NoModel}` recorded as a step with code 0) |
| D8 | The model download is **a settings action and an onboarding offer, never automatic** | spec §5.3 | Tasks 8, 10; no tick, no thread and no slot in this plan downloads anything |
| D9 | **Judgment logs hold ids and field values only**, in app data, never the vault — so neither the backup nor a future second device carries raw content | spec §5.4, §5.6, decision 13 | Task 5 (`judgelog`), and its `no_note_text_ever_reaches_a_log_line` test |
| D10 | Rule promotion (tier 2) is **3b's**, and this plan leaves it a seam rather than a stub with an opinion | spec §5.4 | Task 2 (`trait Rules` + `NoRules`), Task 7 (the `course_map` pin is deliberately not written; see the task's note) |
| D11 | **Enrichment first**, then event verdicts, then Gmail | spec §5.5 | This plan's scope; Tasks 2 and 4 name the two later actors without building them |
| D12 | Judged fields for a task are exactly `effort_hours, effort_confidence, importance, importance_reason, course, domain` | `provenance::JUDGED_FIELDS_TASK` | Task 7 writes five of the six plus `needs_enrichment`; a test asserts the set it writes is a subset of `JUDGED_FIELDS_TASK` ∪ `{needs_enrichment}` |
| R1 | No new single-user assumptions | CLAUDE.md rule 2, spec §12 | No task names a path, machine or person; every directory is derived from the app-data root or a `--vault`; every test builds its own temp vault |
| R2 | Every note write goes through `write`; journal first, single-line surgery second; **no note is ever parsed and re-dumped** | spec §12 | Tasks 6, 7 — the only new note either creates is an approval, minted through `write::create` from a `yamlemit::safe_dump_block` frontmatter, exactly as `info`/`issues` do |
| R3 | Judge once, re-propose freely; amend cards for re-judgements; `judgment:` is a single-line flow mapping and `guard_block_style` refuses anything else | CLAUDE.md, spec §12 | Task 6 (the proposal is built from `jsonable` scalars and validated by `approvals::validate_amendment`), Task 7 |
| R4 | Never regenerate the eight Python-written references; the three Rust ones only with a reviewed diff | spec §12 | No task touches `tests/fixtures/*`; Task 6 is the only `write`-path change and its gate is both dual-run scripts |
| R5 | `$mode` stays `python-live` until G2; Rust writes scratch only until then | spec §12 | Global constraints; Task 12's HANDOFF line says it again |
| R6 | Credentials in the OS keychain; never in the repo, a log, a backup rule, **or a model prompt** | spec §12 | Task 4's prompt is built from the note's own fields, the course note's grade weights and `profile/preferences.md`, and from nothing else; a test asserts `config/ingest.yaml` never reaches it |
| R7 | Failures visible; silence never ambiguous | spec §12 | Task 7 prints one line per item and a summary; Task 9's skip is a named step; Task 10's settings row has a state for every outcome |
| R8 | Desktop safety: no synthetic input; screenshots by window handle only | spec §12; CLAUDE.md | No task drives the UI; page checks are the existing headless ones |
| R9 | The read model never writes; `commands.rs` computes nothing | console spec §3.1, spec §12 | Task 8's `inference::*` does the computing; `commands.rs` marshals |
| R10 | All JSON the crate writes goes through `ledger::dumps_value` | CLAUDE.md | Tasks 3, 5, 9 |
| R11 | Every file the crate reads and writes is CRLF-translated on read and restored on write (`pystr::read_text`/`write_text`) | CLAUDE.md | Tasks 2, 7, 11 |
| R12 | The engine binary is found beside the app exe by `scheduler::engine_exe()`; a second binary is found the same way | brief | Task 3's `runtime::resolve` third branch is a sibling `llama-server.exe`, the same shape `engine_exe()` uses |
| R13 | `updates_dir` is one folder for the whole install, not per profile; judgment logs are per profile | brief, spec §5.4 | Task 8 (`runtime\` and `models\` beside `updates\`), Task 5 (`profiles\<id>\judgments\`) |

### Where the spec and the code disagree — and what this plan writes instead

Three, all found while reading the code this plan is written against. Task 12 amends the spec for each.

1. **Actor names.** Spec §5.2 says the agent actors are `knowlu/enrich`, `knowlu/events`, `knowlu/gmail`. `provenance::is_agent` is `actor.starts_with("agent:")` — nothing else. A `knowlu/enrich` actor is therefore **not an agent**: `write_literals` would skip judge-once entirely and write no `judgment:` block, so Quinn's hand-set effort estimates would be silently overwritten by the model. **This plan uses `agent:knowlu.enrich`**, mirroring `agent:routine.enrich`, and names `agent:knowlu.events` / `agent:knowlu.gmail` for 3b and 3c.

2. **The re-proposal path does not exist in Rust.** Spec §5.2 and CLAUDE.md both state that a judged write against a field Quinn set files a `kind: amend` card. In the Rust port that path returns `WriteError::ProposeNotPorted` unless an identical proposal already happens to be pending (`src/write.rs:356`). Enrichment is exactly where it fires. **Task 6 ports `engine/write.py:propose_amendment` and deletes the `ProposeNotPorted` variant**, and extends `scripts/diff-engines-notes.ps1` — whose header currently says `--propose` is "unported by decision" — with a `--propose` pair, so the new path is byte-compared against Python like every other write.

3. **The runtime is a separate process, but it is not bundled.** Spec §5.3 as amended says llama.cpp ships "as a second Tauri sidecar beside the engine (`bundle.externalBin`)". **This plan keeps the process boundary and drops the bundling**, for three reasons and **none of them about installer size** (ruling R-P3a-1): the runtime is **optional** — the free tier runs with no model at all and most installs will never enable local judgment, so bundling makes every friend pay for something most will not use; it **versions independently** — llama.cpp moves far faster than this app, and coupling its version to our installer means shipping an app release to pick up a runtime fix; and it is **consistent with the models**, which §5.3 already downloads after install for those same reasons. The repo's own figure for the runtime is the product plan's **+20–50 MB** (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`); Task 1 measures the actual asset and Task 12 refuses to close without that number. The spike report already anticipated the shape — *"if the runtime is downloaded rather than bundled, a downloaded executable that must be verified before it is run"* — and **ruling R-P3a-2 makes that verification structural**: the app compiles in a table of supported upstream release digests and there is **no unverified path to executing a runtime binary** (Task 8). `runtime::resolve` still falls back to a sibling `llama-server.exe` beside the exe, so a future bundled build needs no code change; `app/tauri.conf.json`'s `externalBin` list is **not** touched, and `scripts/release.ps1` keeps its one sidecar and its 1 MiB guard.

---

## File structure

**New in the engine crate (`src/`)**
- `judge.rs` — the seam. `Item`, `Verdict`, `Outcome`, `Missing`, `ModelError`, `trait Model`, `trait Rules` + `NoRules`, `Heuristics` (what tier 1 needs, loaded from the vault), `tier1`, `judge_task`, and — from Task 4 — `GRAMMAR`, `prompt_for`, `parse_reply`. **Pure**: no process, no socket, no clock. One responsibility: *given an item and what is available, what is the answer and how sure are we*.
- `runtime.rs` — the llama.cpp process. `RUNTIME_EXE`, `resolve`, `Server` (`start`, `port`, `complete`, `stop`, `Drop`), `impl judge::Model for Server` (Task 4). One responsibility: *there is a process, it is healthy, one call at a time, and it dies with us*.
- `judgelog.rs` — `Entry`, `record`, `read_day`. One responsibility: *what the model was asked and answered, by id, outside the vault*.
- `enrich.rs` — `ACTOR`, `Options`, `pending`, `enrich_with`, `run_lines`, `run_with`, `run`. One responsibility: *the pass*.

**Modified in `src/`**
- `write.rs` — `propose_amendment` ported; `WriteError::ProposeNotPorted` removed; `AMEND_BUTTONS` added.
- `ingest.rs` — `match_course_fields` split out of `match_course` (no behaviour change) so tier 1 uses the ingest's own course rule rather than a second one.
- `lib.rs` — four `pub mod` lines.
- `main.rs` — the `Judge` clap arm.

**New in `app/src/`**
- `inference.rs` — `runtime_dir`, `models_dir`, `runtime_exe`, `model_file`, `judgments_dir`, `Status`, `status`, `sha256_of`, `SupportedRuntime`/`SUPPORTED_RUNTIMES`/`runtime_release_for`/`check_runtime_supported`, `extract_runtime_zip`, `install_runtime_from_zip`, `install_model_from_file`, `remove_model`, `Half`, `install_from_file`, `install_from_manifest`, `download_to`, `Asset`, `Manifest`, `fetch_manifest`. One responsibility: *where the runtime and the model are, and how a **verified** one gets there* — the digest table is the root of trust (R-P3a-2), and the manifest composition lives here so `commands.rs` keeps computing nothing.

**Modified in `app/src/`** — `lib.rs` (one `pub mod`), `scheduler.rs` (`JudgeArgs`, `JudgeState`, `judge_state`, `slot_argv`'s third step, the skip step in `run_slot_inner`), `commands.rs` (`inference_root`, four commands), `onboarding.rs` (`WizardPlan.offer_inference`, the marker file, `pick_file`), `main.rs` (the new handlers), `Cargo.toml` (`sha2`, `zip`, `ureq` — all three already resolved in `app/Cargo.lock`).

**Page (`app/static/`)** — `index.html` (`#set-judge` row, the wizard's finish-panel offer), `console.js` (`renderInference`, the row's handlers, the first-launch offer), `console.css` (no new tokens; the existing `.set-row` rules cover it).

**Tests** — `src/judge.rs`, `src/runtime.rs`, `src/judgelog.rs`, `src/enrich.rs`, `src/write.rs` and `src/ingest.rs` each gain a `mod tests`; `app/tests/inference.rs` (new); `app/tests/scheduler.rs`, `app/tests/commands.rs`, `app/tests/static_assets.rs` extended.

**Scripts** — `scripts/diff-engines-notes.ps1` (a `--propose` pair), `scripts/starvation.ps1` (new, read-only).

**Docs** — `app/README.md`, `docs/surface/anatomy.md`, `docs/HANDOFF.md`, `CLAUDE.md`, the Knowlu spec's §5.2/§5.3/§10, this plan's status line (all Task 12).

---

### Task 1: Spike — the sidecar protocol, measured

**Throwaway allowed. Nothing but the report is committed**, and no source file is modified. The point is one decision Tasks 3–7 are written against: **a loopback server started once per slot, or one process per call?**

The two candidates:

- **A loopback server.** `llama-server.exe` bound to `127.0.0.1` on an ephemeral port, started once at the top of a slot's judge step and stopped at the end of it, so a ~2 GB model loads once for the whole batch. Risks to measure: does Windows Firewall prompt for a loopback-only listener; how long does the load take; what happens to the port when the process crashes.
- **One process per call.** `llama-cli.exe --grammar …`, no port at all, no lifecycle — but the model is re-loaded on every single judgment.

**The expected outcome is the server, amortised over a slot**, and Tasks 3–7 are written against it. This task decides it by measurement, and carries the alternative in full so that outcome B costs one replaced step and nothing else.

**Files:**
- Create: `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`
- Read first: `docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md` (why there is a process boundary at all)
- Touch nothing under `src/`, `app/`, `tests/` or `scripts/`.

**Interfaces:**
- Consumes: nothing. This runs before any code in this plan exists.
- Produces: **the report's `Outcome:` line — exactly one of A, B or C below.** Task 3 step 1 reads it.

- [ ] **Step 1: Try to obtain a runtime binary**

The upstream project publishes prebuilt Windows CPU binaries. Try, in a scratch directory outside the repo:

```bash
mkdir -p /c/Users/danie/AppData/Local/Temp/claude-spike-llama
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
curl -fsSL -o rt.zip "https://github.com/ggml-org/llama.cpp/releases/latest/download/llama-bin-win-cpu-x64.zip" || echo "NO RUNTIME"
```

The asset name changes between releases; if that exact URL 404s, list the release assets and take the `bin-win-cpu-x64` one.

**Outcome C, stated once and referred to from here on: if either this step or step 2 cannot produce a file — no network, a blocked host, an unavailable asset — the outcome is C**, the report records which of the two failed and its exact message, and the task skips to step 7's third question and then step 8.

Record the release tag, the asset name, the **byte size** (R-P3a-1: this is the number Task 12 checks for, and the only size figure this plan is allowed to state beyond the product plan's +20–50 MB) and the SHA-256 — **the SHA-256 is what seeds `inference::SUPPORTED_RUNTIMES` in Task 8** (R-P3a-2):

```bash
unzip -o rt.zip -d rt >/dev/null && ls -l rt && sha256sum rt.zip
```

- [ ] **Step 2: Try to obtain a small GGUF**

A 1–2 B parameter Q4 model is what §5.3 specifies for extraction. Any small instruct GGUF is enough to measure with:

```bash
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
curl -fsSL -o model.gguf "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf" || echo "NO MODEL"
ls -l model.gguf && sha256sum model.gguf
```

If this fails, the outcome is C (see step 1). Record the URL tried and the exact failure.

- [ ] **Step 3: Record the runtime's own flag spelling**

```bash
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
./rt/llama-server.exe --help 2>&1 | head -80
```

Copy into the report the exact spelling of these six, which Task 3 hard-codes: the model flag (`-m` / `--model`), `--host`, `--port`, the context-size flag (`-c` / `--ctx-size`), the GPU-layers flag (`-ngl` / `--n-gpu-layers`), and whether `/health` and `/completion` are the endpoint paths this build serves. **If any spelling differs from the list in Task 3 step 1, the report says so in a `Flags:` line and Task 3 uses the report's.**

- [ ] **Step 4: Build the batch**

There are **no tasks carrying `needs_enrichment: true` in `tests/fixtures/vault-full`** (verified 2026-09-07: `grep -rl "needs_enrichment: true" tests/fixtures/vault-full/tasks/` is empty; the fixture's six tasks are all enriched already), so the batch is **thirty synthesized items**, which is the brief's floor. Write them as thirty plain prompts, one per line, of the shape enrichment actually sends — a title, a due date and two lines of body:

```bash
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
python - <<'PY'
import json
kinds = ["homework set", "reading response", "lab report", "problem set", "quiz review",
         "discussion post", "essay draft", "presentation slides", "exam prep", "worksheet"]
courses = ["CS 100", "PH 106", "GN 103"]
out = []
for i in range(30):
    out.append("Title: %s %s %02d\nDue: 2026-10-%02d\nBody:\nSubmit through the course page. Covers the week's material.\nLate work loses 10%% per day.\n" % (courses[i%3], kinds[i%10], i+1, (i%28)+1))
open("batch.txt","w",encoding="utf-8").write("\x1e".join(out))
PY
wc -c batch.txt
```

- [ ] **Step 5: Measure the server**

Start it once, time the load, then time thirty grammar-constrained completions through one process.

```bash
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
cat > grammar.gbnf <<'G'
root ::= "{" ws "\"course\":" ws course "," ws "\"effort_hours\":" ws number "," ws "\"importance\":" ws importance "," ws "\"importance_reason\":" ws string "," ws "\"confidence\":" ws number ws "}"
course ::= "null" | string
importance ::= "1" | "2" | "3" | "4" | "5"
string ::= "\"" ([^"\\] | "\\" ["\\/bfnrt])* "\""
number ::= [0-9]+ ("." [0-9]+)?
ws ::= " "?
G
./rt/llama-server.exe -m model.gguf --host 127.0.0.1 --port 18099 -c 4096 -ngl 0 &
SRV=$!
python - <<'PY'
import json, time, urllib.request
t0 = time.time()
while time.time() - t0 < 300:
    try:
        urllib.request.urlopen("http://127.0.0.1:18099/health", timeout=2).read()
        break
    except Exception:
        time.sleep(0.25)
load = time.time() - t0
grammar = open("grammar.gbnf", encoding="utf-8").read()
items = open("batch.txt", encoding="utf-8").read().split("\x1e")
per = []
t1 = time.time()
for it in items:
    body = json.dumps({"prompt": it + "\nJSON:", "grammar": grammar, "n_predict": 256,
                       "temperature": 0.0, "top_k": 1, "seed": 0, "cache_prompt": True, "stream": False}).encode()
    s = time.time()
    req = urllib.request.Request("http://127.0.0.1:18099/completion", data=body,
                                 headers={"content-type": "application/json"})
    r = json.loads(urllib.request.urlopen(req, timeout=120).read())
    per.append(time.time() - s)
    assert "content" in r, r
total = time.time() - t1
per.sort()
print("SERVER load=%.2fs items=%d total=%.2fs median=%.2fs p90=%.2fs" % (load, len(per), total, per[len(per)//2], per[int(len(per)*0.9)]))
PY
kill $SRV 2>/dev/null
```

Record `load`, `total`, `median` and `p90` verbatim.

- [ ] **Step 6: Measure one process per call**

```bash
cd /c/Users/danie/AppData/Local/Temp/claude-spike-llama
python - <<'PY'
import subprocess, time
items = open("batch.txt", encoding="utf-8").read().split("\x1e")
per = []
t1 = time.time()
for it in items:
    s = time.time()
    subprocess.run(["./rt/llama-cli.exe", "-m", "model.gguf", "-ngl", "0", "-c", "4096",
                    "--grammar-file", "grammar.gbnf", "-n", "256", "--temp", "0",
                    "-no-cnv", "-p", it + "\nJSON:"],
                   capture_output=True, timeout=600)
    per.append(time.time() - s)
total = time.time() - t1
per.sort()
print("PERCALL items=%d total=%.2fs median=%.2fs p90=%.2fs" % (len(per), total, per[len(per)//2], per[int(len(per)*0.9)]))
PY
```

- [ ] **Step 7: Answer the three risk questions**

Write down, in the report, one line each:

1. **Did Windows Firewall prompt?** Note whether any dialog appeared while step 5 ran. (Expected: no — Windows raises the private/public-network prompt for listeners on a routable interface, not for `127.0.0.1`. Say what actually happened.)
2. **What happens to the port on a crash?** Kill the server mid-request (`taskkill /T /F /PID <pid>`) and immediately re-bind the same port from Python (`socket.socket().bind(("127.0.0.1", 18099))`). Record whether it binds first try. This is why Task 3 asks the OS for an ephemeral port rather than a fixed one.
3. **How long is the load?** The `load=` number from step 5, and the model's size in bytes.

- [ ] **Step 8: Decide, by the rule, and write the report**

The decision rule, pre-committed so the measurement decides and not the writer:

- **Outcome A — the server.** `PERCALL total` is at least **3×** `SERVER total + SERVER load`, **or** the per-call median exceeds 5 s. → Tasks 3–7 exactly as written.
- **Outcome B — one process per call.** The server prompted the firewall, or failed to bind, or came within 3× of per-call. → **`runtime::Server` is replaced by `runtime::PerCall`**, given in full in step 9 below. Everything downstream talks to `judge::Model`, so the blast radius is small — but it is not nil, and here is every place it reaches, so nobody has to discover them one compile error at a time:

  1. **Task 3.** `RUNTIME_EXE` becomes `"llama-cli.exe"`. `Server`, `for_port`, `free_port` and `wait_healthy` go, and with them the four loopback tests (`complete_posts_…`, `a_reply_that_is_not_…`, `only_one_call_…`, `a_call_that_never_answers_…`). `resolve`, `kill_tree` and their two tests stay exactly as written. `LOAD_TIMEOUT` goes; `CALL_TIMEOUT` stays and becomes `PerCall`'s.
  2. **Task 4 step 5.** `impl judge::Model for Server` becomes the `impl judge::Model for PerCall` the report carries, and the `#[ignore]`d smoke test constructs `PerCall::new(&rt, &gg, CALL_TIMEOUT)` instead of `Server::start(…)`.
  3. **Task 7.** `enrich::run_lines`'s `Ok((rt, gguf)) => match Server::start(…)` arm becomes `Ok((rt, gguf)) => enrich_with(vault, opts, Ok(&crate::runtime::PerCall::new(&rt, &gguf, crate::runtime::CALL_TIMEOUT)))` — no start, no failure branch, and the "started once per slot, dropped at the end of it" sentences in the module doc and the function doc become "one process per judgment, which is why this branch lost the spike".
  4. **Task 8.** Nothing, **provided** its tests are written against `quinn_ops::runtime::RUNTIME_EXE` rather than the literal `"llama-server.exe"` — which is what they are (see Task 8's `make_zip` calls and its `the_app_and_the_engine_agree_on_the_runtime_binarys_name` test). The `SUPPORTED_RUNTIMES` table is seeded from whichever asset this spike measured either way.

  Tasks 2, 5, 6, 9, 10, 11 and 12 are untouched by the outcome.
- **Outcome C — nothing could be obtained offline.** Neither a runtime nor a model could be downloaded. → Tasks 3–7 as written and tested **entirely against the scripted fake and the loopback test server**; the real-binary path ships unexercised, and Task 3's `#[ignore]`d smoke test is the only thing that will ever touch one. **This is a legitimate outcome, not a failure** (the brief: "the seam matters more than the measurement").

Write `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md` with, in order: **Outcome:** A/B/C on its own line; the release tag, asset name, size and SHA-256 of anything obtained; the `Flags:` line from step 3; the two measurement lines from steps 5 and 6 verbatim; the three risk answers from step 7; and the arithmetic that produced the outcome.

- [ ] **Step 9: If and only if the outcome is B, record the replacement here**

Paste this into the report under a `## Outcome B replacement` heading, so Task 3's implementer has it beside the decision. It is a complete `judge::Model` implementation with no process lifecycle at all:

```rust
/// Outcome B: one process per call. No port, no health check, no lifetime — `llama-cli` is
/// spawned per judgment, writes its completion to stdout, and exits. The model re-loads every
/// time, which is why this is the losing branch whenever the server is available.
pub struct PerCall {
    runtime: PathBuf,
    model: PathBuf,
    timeout: std::time::Duration,
}

impl PerCall {
    pub fn new(runtime: &Path, model: &Path, timeout: std::time::Duration) -> PerCall {
        PerCall { runtime: runtime.to_path_buf(), model: model.to_path_buf(), timeout }
    }

    pub fn complete(&self, prompt: &str, grammar: &str) -> Result<String, ModelError> {
        // The grammar goes through a file, never an argument: a GBNF rule set carries quotes,
        // backslashes and newlines, and PowerShell 5.1 has already cost this repo one debugging
        // session over a quoted native argument (CLAUDE.md).
        let dir = std::env::temp_dir().join(format!("knowlu-gbnf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| ModelError::Failed(e.to_string()))?;
        let gbnf = dir.join("judge.gbnf");
        std::fs::write(&gbnf, grammar).map_err(|e| ModelError::Failed(e.to_string()))?;
        let args: Vec<String> = vec![
            "-m".into(), self.model.to_string_lossy().into_owned(),
            "-ngl".into(), "0".into(),
            "-c".into(), "4096".into(),
            "--grammar-file".into(), gbnf.to_string_lossy().into_owned(),
            "-n".into(), "256".into(),
            "--temp".into(), "0".into(),
            "-no-cnv".into(),
            "-p".into(), prompt.to_string(),
        ];
        let mut child = std::process::Command::new(&self.runtime)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ModelError::Failed(format!("llama-cli: {} ({e})", self.runtime.display())))?;
        let deadline = std::time::Instant::now() + self.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() >= deadline => {
                    kill_tree(&mut child);
                    return Err(ModelError::Failed("llama-cli: timed out".into()));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(e) => return Err(ModelError::Failed(format!("llama-cli: {e}"))),
            }
        }
        let mut out = String::new();
        if let Some(mut s) = child.stdout.take() {
            use std::io::Read;
            let _ = s.read_to_string(&mut out);
        }
        let _ = std::fs::remove_file(&gbnf);
        Ok(out)
    }
}

impl crate::judge::Model for PerCall {
    fn judge(
        &self,
        item: &crate::judge::Item,
        h: &crate::judge::Heuristics,
        seed: &crate::judge::Verdict,
    ) -> Result<crate::judge::Verdict, ModelError> {
        let text = self.complete(&crate::judge::prompt_for(item, h, seed), crate::judge::GRAMMAR)?;
        crate::judge::parse_reply(&text)
    }
}
```

Under outcome B, Task 3 keeps `RUNTIME_EXE = "llama-cli.exe"`, `resolve`, `kill_tree` and the resolution tests, drops `Server`/`free_port`/`wait_healthy` and the loopback tests, and `enrich::run_lines` constructs `PerCall::new(&rt, &gg, CALL_TIMEOUT)` where it would have called `Server::start`.

- [ ] **Step 10: Commit the report and clean up**

```bash
rm -rf /c/Users/danie/AppData/Local/Temp/claude-spike-llama
git add docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md
git commit -F .git/COMMIT_MSG_3A1
```

with `.git/COMMIT_MSG_3A1` holding:

```
docs: the sidecar protocol spike — server vs one process per call (Knowlu plan 3a, Task 1)

Measured a thirty-item batch through both. Records the outcome line Task 3 branches on,
the runtime's own flag spellings, and the three risk answers (firewall, port on crash, load
time). Nothing was committed to either crate; the probe directory is deleted.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 2: `src/judge.rs` — the seam, the three tiers, and the heuristics

The pure half. No process, no socket, no network, no clock. Given one item, what the vault knows, an optional rule table and either a model or the reason there is none, it answers.

**Files:**
- Create: `src/judge.rs` (with its own `mod tests`)
- Modify: `src/lib.rs` (one `pub mod`), `src/ingest.rs` (`match_course_fields` split out of `match_course`)
- Read first: `src/provenance.rs:52–70` (`JUDGED_FIELDS_TASK`, `judged_fields_for`), `src/ingest.rs:265–285` (`match_course`) and `:236–255` (`contains_bounded`), `docs/runners/cloud-routine-prompt.md` line 10–14 (what step 3 actually decides, which this reproduces)

**Interfaces:**
- Consumes: `ingest::{contains_bounded (private, now called through the new split), match_course}`; `planning::load_planning`; `pystr::{read_text, strip}`; `yaml::get`.
- Produces:
  - `ingest::match_course_fields(uid: &str, haystack: &str, course_map: &[(String, String)]) -> Option<String>`
  - `judge::{CONFIDENCE_FLOOR, MAX_BODY_CHARS, MAX_WEIGHTS_CHARS, MAX_PREFS_CHARS, MAX_REASON_CHARS}`
  - `judge::Item { id, rel_path, title, body, source_uid, created_by, course, due, effort_hours, effort_source }`
  - `judge::Verdict { course, effort_hours, importance, importance_reason, confidence, tier }` with `Verdict::complete(&self) -> bool`
  - `judge::Outcome::{Answered(Verdict), LowConfidence { seed, why }, ModelNotInstalled(Verdict), RuntimeNotInstalled(Verdict)}` with `verdict(&self) -> &Verdict` and `label(&self) -> &'static str`
  - `judge::Missing::{Runtime, Model}`; `judge::ModelError::Failed(String)`
  - `judge::Model` (`fn judge(&self, item, h, seed) -> Result<Verdict, ModelError>`)
  - `judge::Rules` (`fn lookup(&self, item) -> Option<Verdict>`) and `judge::NoRules`
  - `judge::Heuristics { course_map, slice_hours, weights, preferences }` with `Heuristics::load(vault) -> Heuristics`
  - `judge::{tier1, merge, judge_task, clip, one_line}`

- [ ] **Step 1: Split the course rule out of `match_course`, unchanged**

Tier 1 must attribute a course by **exactly the rule the ICS ingest applies** — a case-sensitive uid pin first, then a bounded, case-insensitive course-code match — and not by a second rule that can drift from it. In `src/ingest.rs`, replace the body of `match_course` (currently lines 265–285) with a call to a new function, keeping every comment:

```rust
/// The two passes [`match_course`] runs, over haystacks the caller supplies.
///
/// Split out so `judge`'s tier-1 course heuristic is the same rule this module applies to a feed
/// and not a second one that can drift from it: an enrichment that attributed a course differently
/// from the ingest would make the same assignment land under two slugs depending on which producer
/// saw it first. `match_course` is this function plus the SUMMARY/CATEGORIES extraction; nothing
/// about the ordering, the case sensitivity or the boundary test changed when it moved here.
pub fn match_course_fields(
    uid: &str,
    haystack: &str,
    course_map: &[(String, String)],
) -> Option<String> {
    for (fragment, slug) in course_map {
        if uid.contains(fragment.as_str()) {
            return Some(slug.clone());
        }
    }
    for (fragment, slug) in course_map {
        if contains_bounded(haystack, fragment) {
            return Some(slug.clone());
        }
    }
    None
}

/// Attribute an event to a course.
///
/// Two passes, and the order is load-bearing:
/// **(a) uid pins** — a fragment that is a *case-sensitive* substring of the raw uid always wins.
/// This is how course-less gradebook items get pinned by their Blackboard item id.
/// **(b) course codes** — matched only against `SUMMARY` and `CATEGORIES` values, never
/// `DESCRIPTION`, which would attribute an event by an incidental mention.
pub fn match_course(event: &Event, course_map: &[(String, String)]) -> Option<String> {
    let mut fields: Vec<String> = Vec::new();
    for line in event.raw.split('\n') {
        if let Some((name, _params, value)) = parse_property(line) {
            if name == "SUMMARY" || name == "CATEGORIES" {
                fields.push(unescape(&value));
            }
        }
    }
    match_course_fields(&event.uid, &fields.join("\n"), course_map)
}
```

- [ ] **Step 2: Run the existing ingest tests to prove the split changed nothing**

Run (repo root): `cargo test ingest::`
Expected: PASS, same count as before the edit. The split is behaviour-preserving by construction; this is the proof.

- [ ] **Step 3: Write the failing tests** — create `src/judge.rs` with only its `mod tests` filled in and the module doc:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str) -> Item {
        Item {
            id: "task_0123456789".to_string(),
            rel_path: "tasks/x.md".to_string(),
            title: title.to_string(),
            body: "Submit through the course page.".to_string(),
            source_uid: String::new(),
            created_by: "blackboard".to_string(),
            course: None,
            due: Some("2026-10-01T23:59".to_string()),
            effort_hours: 1.0,
            effort_source: "inferred".to_string(),
        }
    }

    fn heur() -> Heuristics {
        Heuristics {
            course_map: vec![
                ("CS-100".to_string(), "cs-100".to_string()),
                ("_884411_1".to_string(), "ph-106".to_string()),
            ],
            slice_hours: 2.0,
            weights: std::collections::BTreeMap::new(),
            preferences: String::new(),
        }
    }

    /// A scripted model: one canned verdict, and a record of what it was asked. Nothing in this
    /// crate's tests ever starts a process (spec §8).
    struct Scripted {
        reply: std::sync::Mutex<Result<Verdict, ModelError>>,
        calls: std::sync::Mutex<usize>,
    }
    impl Scripted {
        fn ok(v: Verdict) -> Scripted {
            Scripted { reply: std::sync::Mutex::new(Ok(v)), calls: std::sync::Mutex::new(0) }
        }
        fn err(why: &str) -> Scripted {
            Scripted {
                reply: std::sync::Mutex::new(Err(ModelError::Failed(why.to_string()))),
                calls: std::sync::Mutex::new(0),
            }
        }
        fn calls(&self) -> usize { *self.calls.lock().unwrap() }
    }
    impl Model for Scripted {
        fn judge(&self, _i: &Item, _h: &Heuristics, _s: &Verdict) -> Result<Verdict, ModelError> {
            *self.calls.lock().unwrap() += 1;
            self.reply.lock().unwrap().clone()
        }
    }

    fn full(conf: f64) -> Verdict {
        Verdict {
            course: None,
            effort_hours: Some(2.5),
            importance: Some(4),
            importance_reason: Some("Worth 15% of the grade.".to_string()),
            confidence: conf,
            tier: 3,
        }
    }

    /// The uid pin wins over the course code, exactly as `ingest::match_course` orders them: a
    /// gradebook item carries no course code at all, and the pin is the only thing that attributes
    /// it.
    #[test]
    fn tier_one_attributes_a_course_by_uid_pin_then_by_code() {
        let h = heur();
        let mut i = item("Homework 3");
        i.source_uid = "blackboard:_884411_1".to_string();
        assert_eq!(tier1(&i, &h).course.as_deref(), Some("ph-106"));

        let mut i = item("CS-100 Homework 3");
        i.source_uid = "blackboard:nothing".to_string();
        assert_eq!(tier1(&i, &h).course.as_deref(), Some("cs-100"));

        // The boundary test survives the move: CS-100 must not match inside a longer token.
        let mut i = item("STATISTICS-1000 reading");
        i.source_uid = "blackboard:nothing".to_string();
        assert_eq!(tier1(&i, &h).course, None);
    }

    /// A note that already names a course is never re-attributed — enrichment fills gaps, it does
    /// not second-guess what is there.
    #[test]
    fn tier_one_keeps_a_course_the_note_already_carries() {
        let mut i = item("CS-100 Homework 3");
        i.course = Some("gn-103".to_string());
        assert_eq!(tier1(&i, &heur()).course.as_deref(), Some("gn-103"));
    }

    /// `effort_source: vendor` means zyBooks or VHL stated the effort. Tier 1 answers it and the
    /// model is never asked — the same rule `coursework::sync_coursework` applies when it declines
    /// to overwrite a non-vendor estimate.
    #[test]
    fn tier_one_answers_effort_only_when_the_vendor_stated_it() {
        let mut i = item("zyBooks 4.2");
        i.effort_source = "vendor".to_string();
        i.effort_hours = 0.75;
        assert_eq!(tier1(&i, &heur()).effort_hours, Some(0.75));
        assert_eq!(tier1(&item("Homework 3"), &heur()).effort_hours, None);
    }

    /// Importance is never a heuristic: it needs the course's grade weights, which is judgment.
    /// So tier 1 alone is never complete, and that is what sends an item to the model.
    #[test]
    fn tier_one_never_completes_on_its_own() {
        let mut i = item("zyBooks 4.2");
        i.effort_source = "vendor".to_string();
        i.course = Some("cs-100".to_string());
        let v = tier1(&i, &heur());
        assert_eq!(v.importance, None);
        assert!(!v.complete(), "importance is judgment, not a heuristic");
    }

    #[test]
    fn the_model_is_asked_only_when_the_tiers_above_it_left_a_gap() {
        let m = Scripted::ok(full(0.9));
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert!(matches!(out, Outcome::Answered(_)), "{out:?}");
        assert_eq!(m.calls(), 1);
        assert_eq!(out.verdict().tier, 3);
        assert_eq!(out.verdict().effort_hours, Some(2.5));
    }

    /// D7: not installed is a normal outcome, and it still carries whatever tier 1 managed — a
    /// course attributed from the map is worth writing even with no model on the machine.
    #[test]
    fn a_missing_runtime_or_model_is_an_outcome_that_still_carries_tier_one() {
        let mut i = item("CS-100 Homework 3");
        i.source_uid = "blackboard:nothing".to_string();
        let r = judge_task(&i, &heur(), &NoRules, Err(Missing::Runtime));
        assert!(matches!(r, Outcome::RuntimeNotInstalled(_)), "{r:?}");
        assert_eq!(r.verdict().course.as_deref(), Some("cs-100"));
        assert_eq!(r.label(), "runtime not installed");
        let m = judge_task(&i, &heur(), &NoRules, Err(Missing::Model));
        assert!(matches!(m, Outcome::ModelNotInstalled(_)), "{m:?}");
        assert_eq!(m.verdict().course.as_deref(), Some("cs-100"));
    }

    /// The floor is the whole point of tier 3 being conditional: an unsure answer is not written,
    /// and the reason says which number failed.
    #[test]
    fn an_answer_under_the_floor_is_refused_and_says_why() {
        let m = Scripted::ok(full(CONFIDENCE_FLOOR - 0.01));
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        match out {
            Outcome::LowConfidence { seed, why } => {
                assert_eq!(seed.tier, 1, "the model's rejected answer is never merged in");
                assert!(seed.effort_hours.is_none(), "nothing of the refused answer survives");
                assert!(why.contains("0.59") || why.contains("confidence"), "{why}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A transport failure is not a low-confidence answer, and the outcome must not pretend it is:
    /// same variant, different `why`, so "failures visible, silence never ambiguous" holds.
    #[test]
    fn a_model_error_is_reported_as_itself() {
        let m = Scripted::err("llama-server: connection refused");
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        match out {
            Outcome::LowConfidence { why, .. } => assert!(why.contains("connection refused"), "{why}"),
            other => panic!("{other:?}"),
        }
    }

    /// A course the model invented is dropped. A slug that is not in the vault's own course map
    /// would send the note into a group `rank` cannot render and no course note explains.
    #[test]
    fn a_course_slug_the_vault_does_not_know_is_dropped() {
        let mut v = full(0.9);
        v.course = Some("phys-999".to_string());
        let m = Scripted::ok(v);
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert_eq!(out.verdict().course, None, "an unknown slug never reaches a note");

        let mut v = full(0.9);
        v.course = Some("cs-100".to_string());
        let m = Scripted::ok(v);
        let out = judge_task(&item("Homework 3"), &heur(), &NoRules, Ok(&m));
        assert_eq!(out.verdict().course.as_deref(), Some("cs-100"));
    }

    /// D10: tier 2 is 3b's, and its seam is real — a `Rules` that answers stops the model being
    /// asked at all, which is the whole economic point of promotion.
    #[test]
    fn a_rule_that_answers_stops_the_model_being_asked() {
        struct Always;
        impl Rules for Always {
            fn lookup(&self, _i: &Item) -> Option<Verdict> {
                Some(Verdict {
                    course: Some("cs-100".to_string()),
                    effort_hours: Some(1.5),
                    importance: Some(3),
                    importance_reason: Some("Promoted rule: weekly homework.".to_string()),
                    confidence: 1.0,
                    tier: 2,
                })
            }
        }
        let m = Scripted::ok(full(0.9));
        let out = judge_task(&item("Homework 3"), &heur(), &Always, Ok(&m));
        assert!(matches!(out, Outcome::Answered(_)), "{out:?}");
        assert_eq!(m.calls(), 0, "a promoted rule means the model is not called");
        assert_eq!(out.verdict().tier, 2);
        assert!(NoRules.lookup(&item("x")).is_none(), "this plan promotes nothing");
    }

    /// `Heuristics::load` reads four things and never fails: a vault with none of them still
    /// judges (R1 — nothing here assumes a particular vault's shape).
    #[test]
    fn heuristics_load_reads_the_vault_and_survives_an_empty_one() {
        let v = std::env::temp_dir().join(format!("qo-judge-heur-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&v);
        std::fs::create_dir_all(v.join("config")).unwrap();
        std::fs::create_dir_all(v.join("courses")).unwrap();
        std::fs::create_dir_all(v.join("profile")).unwrap();
        let empty = Heuristics::load(&v);
        assert!(empty.course_map.is_empty());
        assert_eq!(empty.slice_hours, 2.0, "planning's own default");

        crate::pystr::write_text(
            &v.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100\n",
        ).unwrap();
        crate::pystr::write_text(&v.join("config").join("planning.yaml"), "slice_hours: 1.5\n").unwrap();
        crate::pystr::write_text(
            &v.join("courses").join("cs-100.md"),
            "---\ntitle: CS 100\n---\n\n## Grade weights\n- Homework: 20%\n\n## Policies\n- No late work.\n",
        ).unwrap();
        crate::pystr::write_text(&v.join("profile").join("preferences.md"), "- German is daily.\n").unwrap();
        let h = Heuristics::load(&v);
        assert_eq!(h.course_map, vec![("CS-100".to_string(), "cs-100".to_string())]);
        assert_eq!(h.slice_hours, 1.5);
        let w = h.weights.get("cs-100").expect("cs-100 weights");
        assert!(w.contains("Homework: 20%"), "{w}");
        assert!(!w.contains("No late work"), "the section stops at the next heading: {w}");
        assert!(h.preferences.contains("German is daily"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `clip` and `one_line` bound what reaches a prompt and a frontmatter line. `clip` must be
    /// char-safe: slicing a multi-byte character in half panics.
    #[test]
    fn clip_is_char_safe_and_one_line_is_single_line() {
        assert_eq!(clip("abcdef", 3), "abc");
        assert_eq!(clip("abc", 10), "abc");
        assert_eq!(clip("é".repeat(10).as_str(), 3), "ééé");
        let messy = "  two\nlines\tand   spaces  ";
        assert_eq!(one_line(messy, 100), "two lines and spaces");
        assert_eq!(one_line("abcdef", 4), "abcd");
        assert!(crate::write::single_line_problem(&one_line(messy, 100)).is_none());
    }
}
```

- [ ] **Step 4: Run them to verify they fail**

Run (repo root): `cargo test judge::`
Expected: FAIL to compile — `cannot find type Item in this scope`, and the rest.

- [ ] **Step 5: Write the module**

Put this above the `mod tests` block in `src/judge.rs`:

```rust
//! The judgment seam (Knowlu spec §5.2, decision 11): one module answers every judgment the same
//! way, in three tiers — deterministic heuristics in the engine's own terms, then a promoted rule
//! table, then a local model, and only when the tiers above it left a gap.
//!
//! **Pure.** Nothing here starts a process, opens a socket, reads the clock or writes a file. The
//! model is a trait (`Model`), the rule table is a trait (`Rules`), and what the vault knows is a
//! value (`Heuristics`) the caller loads once per run. That is what lets every test drive a
//! scripted fake and what keeps `rank` — which never calls a model (decision 11) — structurally
//! unable to reach one from here.
//!
//! **This module answers; it never writes.** `enrich` turns an `Outcome` into a judged write
//! through `write`, with an agent actor, so judge-once holds and a field Quinn set comes back as a
//! `kind: amend` card instead of being overwritten.
//!
//! The three agent actors this programme uses are `agent:knowlu.enrich` (plan 3a),
//! `agent:knowlu.events` (3b) and `agent:knowlu.gmail` (3c). They start with `agent:` because
//! `provenance::is_agent` is a plain `starts_with("agent:")` test and nothing else — an actor
//! spelled `knowlu/enrich` would skip judge-once silently and write no provenance block.

use std::collections::BTreeMap;
use std::path::Path;

/// Under this, the model's answer is refused: it is logged, and nothing is written.
///
/// 0.6 rather than a higher bar because the alternative to a merely-plausible estimate is **no
/// estimate at all** — `ingest`'s template leaves `effort_hours: 1.0, importance: 3` on every
/// Blackboard task, which is worse than a considered 2.5, and every field written this way is a
/// judged write Quinn can overrule in the console at any time.
pub const CONFIDENCE_FLOOR: f64 = 0.6;

/// How much of a note's body reaches the prompt. A bound, not a guess: an assignment description
/// that carries a whole syllabus would otherwise push the real question out of the context window.
pub const MAX_BODY_CHARS: usize = 1200;
/// How much of a course note's `## Grade weights` section reaches the prompt.
pub const MAX_WEIGHTS_CHARS: usize = 600;
/// How much of `profile/preferences.md` reaches the prompt.
pub const MAX_PREFS_CHARS: usize = 600;
/// The cap on a generated `importance_reason`. It becomes one frontmatter line, so it must survive
/// `write::single_line_problem`, and a paragraph in a note's frontmatter is unreadable anyway.
pub const MAX_REASON_CHARS: usize = 140;

/// One thing to judge, flattened out of a task note by `enrich::pending`.
///
/// `id` is the note's opaque `id:` — the key judge-once and the judgment log are both written
/// against. An item with no id never reaches here: `enrich` skips it and says so, because without
/// one `journal::human_set` cannot answer "did Quinn set this?" and the whole protection is void.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub rel_path: String,
    pub title: String,
    pub body: String,
    pub source_uid: String,
    pub created_by: String,
    pub course: Option<String>,
    pub due: Option<String>,
    pub effort_hours: f64,
    pub effort_source: String,
}

/// What a tier answered. **Every field is optional**, because a tier answers what it can: tier 1
/// attributes a course and accepts a vendor's effort and stops there, and that partial answer is
/// still worth writing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Verdict {
    pub course: Option<String>,
    pub effort_hours: Option<f64>,
    pub importance: Option<i64>,
    pub importance_reason: Option<String>,
    /// 0–1. Tier 1 and tier 2 are deterministic and answer 1.0; only tier 3's is a judgement.
    pub confidence: f64,
    /// 0 nothing, 1 heuristics, 2 a promoted rule, 3 the model.
    pub tier: u8,
}

impl Verdict {
    /// Enough to clear `needs_enrichment`.
    ///
    /// **`course` is deliberately not required.** The cloud routine writes `needs_enrichment=false`
    /// even when it could not attribute a course, saying so in `importance_reason`; an item held
    /// open forever because no course matched would be re-judged twice a day for the rest of the
    /// semester.
    pub fn complete(&self) -> bool {
        self.effort_hours.is_some() && self.importance.is_some() && self.importance_reason.is_some()
    }
}

/// The four ways a judgment ends. Three of them are normal (spec §5.3: "the app runs with no model
/// present"), and none of them is a failed slot.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Answered(Verdict),
    /// The model was reached and its answer was not used: under the floor, incomplete, or the call
    /// itself failed. `seed` is what the tiers ABOVE the model had — the refused answer is never
    /// merged in — and `why` says which of the three it was, verbatim.
    LowConfidence { seed: Verdict, why: String },
    ModelNotInstalled(Verdict),
    RuntimeNotInstalled(Verdict),
}

impl Outcome {
    /// What may be written, whatever the outcome. Always the tiers that succeeded, never a refused
    /// model answer.
    pub fn verdict(&self) -> &Verdict {
        match self {
            Outcome::Answered(v) => v,
            Outcome::LowConfidence { seed, .. } => seed,
            Outcome::ModelNotInstalled(v) => v,
            Outcome::RuntimeNotInstalled(v) => v,
        }
    }

    /// The word `enrich` prints and `judgelog` records. Stable, lower case, no punctuation.
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Answered(_) => "answered",
            Outcome::LowConfidence { .. } => "low confidence",
            Outcome::ModelNotInstalled(_) => "model not installed",
            Outcome::RuntimeNotInstalled(_) => "runtime not installed",
        }
    }
}

/// Why there is no model to call. Resolved once per run, before any item is judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    Runtime,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    Failed(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::Failed(m) => write!(f, "{m}"),
        }
    }
}

/// Tier 3. The implementation owns the transport, the grammar and the reply format; this module
/// owns only the tiers — which is what makes `runtime::Server` and a scripted fake substitutable.
pub trait Model {
    fn judge(&self, item: &Item, h: &Heuristics, seed: &Verdict) -> Result<Verdict, ModelError>;
}

/// Tier 2 — **the seam plan 3b fills** (spec §5.4). A rule that reproduces the model's output
/// across repeated cases is proposed for promotion as an approval card; once approved it enters
/// this tier and the model stops being called for that pattern. Nothing in plan 3a promotes
/// anything, so the only implementation here is [`NoRules`].
pub trait Rules {
    fn lookup(&self, item: &Item) -> Option<Verdict>;
}

/// The empty rule table plan 3a ships. Deliberately not a stub with an opinion: a heuristic
/// invented here would be one 3b's promotion loop then had to argue with.
pub struct NoRules;

impl Rules for NoRules {
    fn lookup(&self, _item: &Item) -> Option<Verdict> {
        None
    }
}

/// What tier 1 knows and what the prompt is grounded in — loaded once per run from the vault, so
/// judging a hundred items reads `config/` and `courses/` once rather than a hundred times.
#[derive(Debug, Clone, PartialEq)]
pub struct Heuristics {
    /// `config/ingest.yaml`'s `course_map`, **in file order** — `match_course_fields` walks it in
    /// order and the first match wins, exactly as the ICS ingest does.
    pub course_map: Vec<(String, String)>,
    /// `config/planning.yaml`'s `slice_hours`. Never an answer — an anchor in the prompt, so an
    /// estimate is expressed in the units this vault actually works in.
    pub slice_hours: f64,
    /// slug -> the course note's `## Grade weights` section, clipped. This is what makes an
    /// importance answer grounded rather than invented; the routine's step 3 reads the same thing.
    pub weights: BTreeMap<String, String>,
    /// `profile/preferences.md`, clipped. Absent on a friend's fresh vault, which is fine.
    pub preferences: String,
}

/// Python's `str[:n]` by **characters**, never bytes: `&s[..n]` panics in the middle of a
/// multi-byte character, and a note title with an em dash in it is not an edge case here.
pub fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Collapse every run of whitespace to one space, trim, and clip — so the result can be written as
/// one frontmatter line. `write::single_line_problem` is what would otherwise refuse it, and being
/// refused at the write is one round trip too late.
pub fn one_line(text: &str, max: usize) -> String {
    clip(&text.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

/// The `## <heading>` section of a markdown body, up to the next `## ` line, clipped.
fn section(text: &str, heading: &str, max: usize) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim() == heading {
            inside = true;
            continue;
        }
        if inside {
            if line.starts_with("## ") {
                break;
            }
            out.push(line);
        }
    }
    clip(out.join("\n").trim(), max)
}

impl Heuristics {
    /// Reads four things and **never fails**: a vault with no `courses/`, no `profile/` and no
    /// `course_map` still judges, on the model alone. Every path is derived from `vault` (R1).
    pub fn load(vault: &Path) -> Heuristics {
        let course_map: Vec<(String, String)> = crate::pystr::read_text(
            &vault.join("config").join("ingest.yaml"),
        )
        .ok()
        .and_then(|t| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&t).ok())
        .and_then(|v| v.get("course_map").and_then(|m| m.as_mapping()).cloned())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();

        let slice_hours =
            crate::planning::load_planning(&vault.join("config").join("planning.yaml")).slice_hours;

        let mut weights = BTreeMap::new();
        if let Ok(rd) = std::fs::read_dir(vault.join("courses")) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.extension().map(|x| x != "md").unwrap_or(true) {
                    continue;
                }
                let Ok(text) = crate::pystr::read_text(&path) else { continue };
                let Some(slug) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                    continue;
                };
                let w = section(&text, "## Grade weights", MAX_WEIGHTS_CHARS);
                if !w.is_empty() {
                    weights.insert(slug, w);
                }
            }
        }

        let preferences = crate::pystr::read_text(&vault.join("profile").join("preferences.md"))
            .map(|t| clip(t.trim(), MAX_PREFS_CHARS))
            .unwrap_or_default();

        Heuristics { course_map, slice_hours, weights, preferences }
    }

    /// Is this a slug the vault itself knows? The guard on a model that invents one.
    pub fn knows_course(&self, slug: &str) -> bool {
        self.weights.contains_key(slug) || self.course_map.iter().any(|(_, s)| s == slug)
    }
}

/// **Tier 1 — deterministic, in the engine's own terms.**
///
/// Two answers and no more:
///
/// - **course** — whatever the note already carries, else `ingest::match_course_fields` over the
///   `course_map`: a case-sensitive uid pin, then a bounded course code in the title. The same
///   rule the ICS ingest applies, reached through the same function.
/// - **effort_hours** — the note's own value when `effort_source: vendor`, because zyBooks and VHL
///   state it and nothing may re-state it (the rule `coursework::sync_coursework` already keeps).
///
/// **Importance is never a heuristic.** It is grounded in a course's grade weights, which is a
/// judgement, so tier 1 alone is never `complete()` — which is exactly what sends an item onward.
pub fn tier1(item: &Item, h: &Heuristics) -> Verdict {
    let course = match item.course.as_ref().map(|c| c.trim()).filter(|c| !c.is_empty()) {
        Some(c) => Some(c.to_string()),
        None => crate::ingest::match_course_fields(&item.source_uid, &item.title, &h.course_map),
    };
    let effort_hours = (item.effort_source == "vendor").then_some(item.effort_hours);
    Verdict { course, effort_hours, importance: None, importance_reason: None, confidence: 1.0, tier: 1 }
}

/// `lower` wins every field it answered; `upper` fills only the gaps. The tier and confidence come
/// from whichever actually contributed.
pub fn merge(lower: Verdict, upper: Verdict) -> Verdict {
    let contributed = (lower.course.is_none() && upper.course.is_some())
        || (lower.effort_hours.is_none() && upper.effort_hours.is_some())
        || (lower.importance.is_none() && upper.importance.is_some())
        || (lower.importance_reason.is_none() && upper.importance_reason.is_some());
    Verdict {
        course: lower.course.or(upper.course),
        effort_hours: lower.effort_hours.or(upper.effort_hours),
        importance: lower.importance.or(upper.importance),
        importance_reason: lower.importance_reason.or(upper.importance_reason),
        confidence: if contributed { upper.confidence } else { lower.confidence },
        tier: if contributed { upper.tier.max(lower.tier) } else { lower.tier },
    }
}

/// The seam. Tier 1, then tier 2, then — only if a gap is left, and only if there is one to reach —
/// tier 3.
///
/// `model` is `Ok(m)` or the reason there is none, resolved once per run rather than per item: a
/// hundred items must not each re-stat the disk for a model file that is not there.
pub fn judge_task(
    item: &Item,
    h: &Heuristics,
    rules: &dyn Rules,
    model: Result<&dyn Model, Missing>,
) -> Outcome {
    let mut seed = tier1(item, h);
    if seed.complete() {
        return Outcome::Answered(seed);
    }
    if let Some(rule) = rules.lookup(item) {
        seed = merge(seed, rule);
        if seed.complete() {
            return Outcome::Answered(seed);
        }
    }
    let model = match model {
        Err(Missing::Runtime) => return Outcome::RuntimeNotInstalled(seed),
        Err(Missing::Model) => return Outcome::ModelNotInstalled(seed),
        Ok(m) => m,
    };
    let mut answer = match model.judge(item, h, &seed) {
        Ok(a) => a,
        Err(e) => return Outcome::LowConfidence { seed, why: e.to_string() },
    };
    if answer.confidence < CONFIDENCE_FLOOR {
        let why = format!("confidence {:.2} below {:.2}", answer.confidence, CONFIDENCE_FLOOR);
        return Outcome::LowConfidence { seed, why };
    }
    // A slug the vault does not know would put the note in a group nothing renders and no course
    // note explains. Dropped rather than refused: the effort and importance answers are still good.
    if let Some(c) = &answer.course {
        if !h.knows_course(c) {
            answer.course = None;
        }
    }
    let merged = merge(seed.clone(), answer);
    if merged.complete() {
        Outcome::Answered(merged)
    } else {
        Outcome::LowConfidence { seed, why: "the model left a required field empty".to_string() }
    }
}
```

- [ ] **Step 6: Declare the module**

In `src/lib.rs`, after the `pub mod uievents;` block and before `pub mod backup;`, add:

```rust
// Knowlu plan 3a — the judgment seam (spec §5.2). Pure: three tiers, a model behind a trait, a
// rule table behind a trait. `rank` never reaches it; `enrich` is its only production caller.
pub mod judge;
```

- [ ] **Step 7: Run the tests**

Run (repo root): `cargo test judge:: ingest::`
Expected: PASS, 0 warnings.

- [ ] **Step 8: Run the full engine gate**

```bash
export PATH="$HOME/.cargo/bin:/c/Users/danie/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH"
cargo test 2>&1 | tail -20
```

Expected: all green, **0 warnings**, `tests/oracle.rs` and `tests/surface_oracle.rs` included. Then, from PowerShell:

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: both exit 0. `ingest.rs` changed, so this is not optional.

- [ ] **Step 9: Commit**

```bash
git add src/judge.rs src/lib.rs src/ingest.rs
git commit -F .git/COMMIT_MSG_3A2
```

```
feat(judge): the seam, the three tiers, and the tier-1 heuristics (Knowlu plan 3a, Task 2)

src/judge.rs answers a judgment in three tiers — deterministic heuristics, a promoted rule
table (3b's seam, empty here), then a local model behind a trait. Pure: no process, no socket,
no clock, so every test drives a scripted fake. Tier 1 attributes a course through
ingest::match_course_fields, split out of match_course unchanged so enrichment and the ICS
ingest cannot drift apart, and accepts a vendor's effort. Importance is never a heuristic.

Outcome carries what the tiers managed even when the model is absent or refused, so a course
attributed from the map is still written on a machine with no model — spec §5.3's "the app
runs with no model present", made concrete.

Actors are agent:knowlu.enrich / .events / .gmail, not the spec's knowlu/enrich:
provenance::is_agent is starts_with("agent:"), so the spec's spelling would skip judge-once.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 3: `src/runtime.rs` — locate it, start it, health-check it, one call at a time, kill it on drop

The process half, written against **Task 1's outcome**. Nothing in this task parses a model reply or builds a prompt — that is Task 4. This task's job is that there is a healthy process, that exactly one call is in flight at a time, that a call is bounded, and that the process dies with us.

**Files:**
- Create: `src/runtime.rs` (with its own `mod tests`)
- Modify: `src/lib.rs` (one `pub mod`)
- Read first: `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md` (**the `Outcome:` and `Flags:` lines**), `app/src/scheduler.rs:144–156` (`engine_exe`, the resolution order this mirrors) and `:237–249` (`kill_tree`), `src/calfeed.rs:541–554` (how this crate builds a `ureq::Agent`)

**Interfaces:**
- Consumes: `judge::{Missing, ModelError}`; `ledger::dumps_value`; `ureq` (already a dependency).
- Produces:
  - `runtime::RUNTIME_EXE: &str`
  - `runtime::{LOAD_TIMEOUT, CALL_TIMEOUT}: std::time::Duration`
  - `runtime::resolve(runtime_arg: Option<&Path>, model_arg: Option<&Path>) -> Result<(PathBuf, PathBuf), Missing>`
  - `runtime::Server` with `start(runtime: &Path, model: &Path, load: Duration, call: Duration) -> Result<Server, ModelError>`, `for_port(port: u16, call: Duration) -> Server` (the loopback test seam), `port(&self) -> u16`, `complete(&self, prompt: &str, grammar: &str) -> Result<String, ModelError>`, `stop(&self)`, `impl Drop`
  - `runtime::kill_tree(child: &mut std::process::Child)`

- [ ] **Step 1: Read the spike's two lines and act on them**

Open `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`.

- If **`Outcome: B`**, stop here and follow the report's `## Outcome B replacement` section: keep steps 2–4's `resolve`/`kill_tree` and their tests, drop `Server`, `free_port`, `wait_healthy` and the loopback tests, and paste the `PerCall` implementation the report carries. Everything downstream is unchanged.
- Otherwise (**A or C**) continue with this task as written. Under **C** the code is identical; only the `#[ignore]`d smoke test in step 7 has nothing to run against, which is what its attribute already says.
- The flags in step 5's `args` vector are `-m`, `--host`, `--port`, `-c`, `-ngl`. **If the spike's `Flags:` line records different spellings for this build, use the spike's**, and say so in a comment naming the release tag.

- [ ] **Step 2: Write the failing tests** — create `src/runtime.rs` with only the module doc and this `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-runtime-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The resolution order is the one `scheduler::engine_exe` already uses for the engine, with an
    /// explicit argument in front: the app knows where it put the runtime and says so, and a
    /// developer can drop one beside the exe instead.
    #[test]
    fn resolve_prefers_the_argument_then_the_env_and_names_what_is_missing() {
        let d = tmp("resolve");
        let rt = d.join("llama-server.exe");
        let gg = d.join("model.gguf");
        std::fs::write(&rt, b"x").unwrap();
        std::fs::write(&gg, b"x").unwrap();

        assert_eq!(resolve(Some(&rt), Some(&gg)).unwrap(), (rt.clone(), gg.clone()));
        // A path that is not a file is not a runtime: an app that passed a deleted one must get
        // "not installed", not a spawn failure twenty seconds later.
        assert_eq!(resolve(Some(&d.join("gone.exe")), Some(&gg)), Err(judge::Missing::Runtime));
        assert_eq!(resolve(Some(&rt), Some(&d.join("gone.gguf"))), Err(judge::Missing::Model));
        assert_eq!(resolve(Some(&rt), None), Err(judge::Missing::Model));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Two ports in a row are different and both are bindable — the ephemeral-port ask is what
    /// makes a crashed server's port a non-problem (spike risk 2).
    #[test]
    fn free_port_hands_out_a_bindable_loopback_port() {
        let a = free_port().unwrap();
        let b = free_port().unwrap();
        assert!(a > 0 && b > 0);
        let l = std::net::TcpListener::bind(("127.0.0.1", a)).expect("the port it named is bindable");
        drop(l);
    }

    /// A one-request loopback server, in this test's own process, on 127.0.0.1 — no DNS, no route
    /// off the machine, no listener on a routable interface (this plan's gloss on "no network in
    /// any test"). It is the only way `complete`'s request shape and reply parsing get exercised
    /// without a real llama-server.
    fn one_shot(reply_body: &'static str, status: &'static str) -> (u16, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut buf = [0u8; 4096];
            // Read until the body is in: the request carries Content-Length, so stop once the
            // bytes after the blank line reach it.
            loop {
                let n = sock.read(&mut buf).unwrap_or(0);
                if n == 0 { break; }
                seen.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&seen).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let want: usize = text
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0)))
                        .unwrap_or(0);
                    if seen.len() - (split + 4) >= want { break; }
                }
            }
            let out = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply_body}",
                reply_body.len()
            );
            let _ = sock.write_all(out.as_bytes());
            let _ = sock.flush();
            String::from_utf8_lossy(&seen).to_string()
        });
        (port, handle)
    }

    #[test]
    fn complete_posts_the_prompt_and_the_grammar_and_returns_the_content() {
        let (port, server) = one_shot(r#"{"content":"{\"importance\":4}"}"#, "200 OK");
        let s = Server::for_port(port, std::time::Duration::from_secs(10));
        let got = s.complete("PROMPT-TOKEN", "GRAMMAR-TOKEN").expect("a reply");
        assert_eq!(got, "{\"importance\":4}");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /completion "), "{request}");
        assert!(request.contains("PROMPT-TOKEN"), "the prompt goes in the body");
        assert!(request.contains("GRAMMAR-TOKEN"), "D6: the grammar goes on EVERY call");
        assert!(request.contains("\"temperature\": 0.0"), "a judgment is not a creative writing task");
        assert!(request.to_ascii_lowercase().contains("content-type: application/json"));
    }

    #[test]
    fn a_reply_that_is_not_the_expected_shape_is_an_error_naming_the_server() {
        let (port, server) = one_shot("not json at all", "200 OK");
        let s = Server::for_port(port, std::time::Duration::from_secs(10));
        let err = s.complete("p", "g").expect_err("garbage must not be returned as content");
        assert!(format!("{err}").contains("llama-server"), "{err}");
        let _ = server.join();

        let (port, server) = one_shot(r#"{"error":"no slot"}"#, "200 OK");
        let s = Server::for_port(port, std::time::Duration::from_secs(10));
        let err = s.complete("p", "g").expect_err("no content is an error");
        assert!(format!("{err}").contains("no content"), "{err}");
        let _ = server.join();
    }

    /// Single flight. Without the lock the second call races the first onto the same process and
    /// llama-server answers one of them from the wrong slot.
    ///
    /// **The server side must be genuinely concurrent or this test asserts nothing** (review S1): an
    /// acceptor that reads, sleeps and replies inline can never have two connections live at once
    /// whether or not the client holds a lock, so `peak` would be 1 either way. Each accepted
    /// connection therefore gets its own handler thread, and the sleep inside the handler is what
    /// gives a second unlocked call the window to overlap.
    #[test]
    fn only_one_call_is_in_flight_at_a_time() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let live = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (p2, l2) = (peak.clone(), live.clone());
        let acceptor = std::thread::spawn(move || {
            let mut handlers = Vec::new();
            for _ in 0..2 {
                let (mut sock, _) = listener.accept().unwrap();
                let (p3, l3) = (p2.clone(), l2.clone());
                handlers.push(std::thread::spawn(move || {
                    let n = l3.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    p3.fetch_max(n, std::sync::atomic::Ordering::SeqCst);
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf);
                    // Long enough that an unlocked second call would be inside its own handler
                    // while this one is still here.
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    let body = r#"{"content":"ok"}"#;
                    let _ = sock.write_all(format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes());
                    l3.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }));
            }
            for h in handlers { let _ = h.join(); }
        });
        let s = std::sync::Arc::new(Server::for_port(port, std::time::Duration::from_secs(10)));
        let s2 = s.clone();
        let t = std::thread::spawn(move || s2.complete("a", "g"));
        let _ = s.complete("b", "g");
        let _ = t.join();
        let _ = acceptor.join();
        assert_eq!(peak.load(std::sync::atomic::Ordering::SeqCst), 1, "two calls overlapped on the server");
    }

    /// A call that never comes back must not wedge a slot for twenty minutes. The listener accepts
    /// and says nothing.
    #[test]
    fn a_call_that_never_answers_is_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let acceptor = std::thread::spawn(move || {
            let (sock, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(900));
            drop(sock);
        });
        let s = Server::for_port(port, std::time::Duration::from_millis(200));
        let started = std::time::Instant::now();
        let err = s.complete("p", "g").expect_err("a hung server must time out");
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "took {:?}", started.elapsed());
        assert!(format!("{err}").contains("llama-server"), "{err}");
        let _ = acceptor.join();
    }

    /// Kill on drop, and it kills the TREE: a child that spawned its own children leaves the pipes
    /// open, which is the failure `scheduler::kill_tree` was written for.
    #[test]
    fn the_child_is_killed_when_the_server_is_dropped() {
        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 60"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("powershell is on every Windows 11 machine");
        assert!(child.try_wait().unwrap().is_none(), "it should still be running");
        kill_tree(&mut child);
        assert!(child.try_wait().unwrap().is_some(), "kill_tree must reap it");
    }

    /// A runtime that is not there is an error naming the path, not a panic, and not a hang.
    #[test]
    fn starting_a_runtime_that_is_not_there_says_which_path() {
        let d = tmp("nostart");
        let rt = d.join("does-not-exist.exe");
        let gg = d.join("model.gguf");
        std::fs::write(&gg, b"x").unwrap();
        let err = Server::start(&rt, &gg, std::time::Duration::from_secs(1), std::time::Duration::from_secs(1))
            .expect_err("no binary, no server");
        assert!(format!("{err}").contains("does-not-exist.exe"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // The one test that needs a real runtime and a real model lives in **Task 4**, not here.
    // `#[ignore]` suppresses RUNNING, never COMPILING: it calls `judge::GRAMMAR` and
    // `judge::parse_reply`, which Task 4 introduces, so writing it here would break the build at the
    // end of this task. It arrives beside the `impl judge::Model for Server` it actually exercises.
}
```

- [ ] **Step 3: Run them to verify they fail**

Run (repo root): `cargo test runtime::`
Expected: FAIL to compile — `cannot find function resolve in this scope`.

- [ ] **Step 4: `resolve`, `free_port` and `kill_tree`**

Put this above the tests in `src/runtime.rs`:

```rust
//! The local inference runtime as a **separate process** (Knowlu spec §5.3 as amended 2026-09-07,
//! decision 12): llama.cpp's own `llama-server`, bound to loopback, started once per judge run and
//! killed when this value is dropped.
//!
//! **Why a process and not a library.** `llama-cpp-2` cannot be linked on this product's
//! `stable-x86_64-pc-windows-gnu` toolchain without forking a dependency — llama.cpp's vendored
//! `cpp-httplib` calls `CreateFile2`, which this mingw-w64 does not declare, and it is built
//! whatever `LLAMA_BUILD_SERVER` says (`docs/superpowers/reports/2026-09-07-llama-cpp-on-gnu-spike.md`).
//! The deciding argument is ownership rather than that failure: linking makes us own llama.cpp's
//! build, a separate process lets us consume its releases. Grammar-constrained decoding — which the
//! product plan requires on every model call without exception — is available across the boundary.
//!
//! **Why it is not bundled** (ruling R-P3a-1). The amended §5.3 says "a second Tauri sidecar
//! (`bundle.externalBin`)". The process boundary is kept and the bundling is not, for three reasons
//! and none of them about installer size: the runtime is **optional** (the free tier runs with no
//! model at all, so bundling makes every install pay for something most will never enable), it
//! **versions independently** (llama.cpp moves far faster than this app; coupling its version to our
//! installer means an app release to pick up a runtime fix), and it is **consistent with the
//! models**, which §5.3 already downloads after install for the same reasons. The repo's own figure
//! for its size is the product plan's +20–50 MB
//! (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`).
//!
//! So the app downloads or installs the runtime into its app-data root, checks its SHA-256 against a
//! compiled-in table of supported upstream releases before it is ever run (R-P3a-2,
//! `app/src/inference.rs`), and passes the path in. [`resolve`]'s third branch is nevertheless a
//! sibling `llama-server.exe` — the same shape `scheduler::engine_exe()` uses — so a future bundled
//! build needs no change here.
//!
//! **Nothing in this module is reached by `rank`** (decision 11) and nothing in it is reached by a
//! test: `cargo test` never starts llama-server, never downloads a model, and never leaves the
//! loopback interface.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::judge::{self, ModelError};

/// The binary this crate looks for. Named once; `app/src/inference.rs` uses the same spelling and a
/// test there asserts the two agree.
pub const RUNTIME_EXE: &str = "llama-server.exe";

/// How long a ~2 GB model may take to load before the start is called a failure. Generous on
/// purpose: this is a cold read of gigabytes from a student's laptop disk, once per slot, and a
/// judge step that gives up early simply does no judging.
pub const LOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// How long one completion may take. A judge step runs inside a slot whose own child cap is twenty
/// minutes (`scheduler::CHILD_TIMEOUT`), so a per-call bound well under that is what keeps a wedged
/// server from eating the whole slot.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// Where the runtime and the model are — or which of the two is missing.
///
/// Order, for each half: **the argument** (the app passes what it installed), then the environment
/// variable (a developer's own copy, and what the `#[ignore]`d smoke test uses), then — for the
/// runtime only — **a sibling beside the running exe**, which is exactly how
/// `scheduler::engine_exe()` finds the engine.
///
/// A path that exists but is not a file is treated as absent: an app that passed a deleted runtime
/// must get "not installed" now, not a spawn failure later.
pub fn resolve(
    runtime_arg: Option<&Path>,
    model_arg: Option<&Path>,
) -> Result<(PathBuf, PathBuf), judge::Missing> {
    let file = |p: PathBuf| p.is_file().then_some(p);
    let env = |name: &str| {
        std::env::var(name).ok().filter(|s| !s.is_empty()).map(PathBuf::from).and_then(file)
    };
    let runtime = runtime_arg
        .map(Path::to_path_buf)
        .and_then(file)
        .or_else(|| env("KNOWLU_LLAMA_SERVER"))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|d| d.join(RUNTIME_EXE)))
                .and_then(file)
        })
        .ok_or(judge::Missing::Runtime)?;
    let model = model_arg
        .map(Path::to_path_buf)
        .and_then(file)
        .or_else(|| env("KNOWLU_MODEL"))
        .ok_or(judge::Missing::Model)?;
    Ok((runtime, model))
}

/// A loopback port the OS says is free.
///
/// Ask-and-release, so a crashed server never leaves a fixed port unusable (spike risk 2). There is
/// a race — another process could take it between the release and the spawn — and it is accepted:
/// the window is microseconds, the consequence is one failed judge step that says so, and the
/// alternative (a fixed port) turns every crash into a permanently broken feature.
fn free_port() -> Result<u16, ModelError> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| ModelError::Failed(format!("llama-server: no loopback port ({e})")))?;
    let port = listener
        .local_addr()
        .map_err(|e| ModelError::Failed(format!("llama-server: no loopback port ({e})")))?
        .port();
    drop(listener);
    Ok(port)
}

/// Kill the child **and everything it spawned**, then reap it.
///
/// `Child::kill` is one `TerminateProcess` against one pid; a child that forked hands its children
/// inherited duplicates of its handles, so killing the parent alone can leave the tree alive. Windows
/// has no process group to signal, so this is `taskkill /T /F`, with the plain kill after it as the
/// fallback for a machine where `taskkill` is missing or refuses. Deliberately the same shape as
/// `app/src/scheduler.rs`'s, which was written for exactly this failure.
pub fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
```

- [ ] **Step 5: `Server`**

Append to `src/runtime.rs`, still above the tests:

```rust
/// One `llama-server` process, and the one connection to it.
///
/// **One call at a time.** `call` is held for the whole of `complete`. llama-server will happily
/// accept two concurrent completions and serve them from different slots; the judge has no use for
/// that and every use for a bounded, ordered batch, and two in flight would make the per-item
/// latency in a run record meaningless.
///
/// **Killed on drop.** `enrich::run_lines` owns the `Server` for the length of the batch and lets
/// it fall out of scope at the end, so the ordinary path, an early return and a panic unwind all
/// stop the process. Nothing survives a judge step.
pub struct Server {
    child: Mutex<Option<std::process::Child>>,
    port: u16,
    agent: ureq::Agent,
    call: Mutex<()>,
}

impl Server {
    /// Spawn, then wait for `/health`. `load` bounds the model load; `call` bounds each completion.
    ///
    /// Flags, from the spike's `Flags:` line: `-m` the model, `--host 127.0.0.1` (loopback only —
    /// a routable bind is what raises the Windows Firewall prompt), `--port`, `-c 4096` context,
    /// `-ngl 0` because §5.3's sizes are CPU-sufficient and a GPU offload on an unknown laptop is a
    /// class of failure this does not need.
    ///
    /// stdio is `null`, not piped: nothing reads it, and a full pipe would block the server the
    /// first time it logged more than the buffer.
    pub fn start(runtime: &Path, model: &Path, load: Duration, call: Duration) -> Result<Server, ModelError> {
        let port = free_port()?;
        let args: Vec<String> = vec![
            "-m".into(),
            model.to_string_lossy().into_owned(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "-c".into(),
            "4096".into(),
            "-ngl".into(),
            "0".into(),
        ];
        let child = Command::new(runtime)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ModelError::Failed(format!("llama-server: {} ({e})", runtime.display())))?;
        let server = Server {
            child: Mutex::new(Some(child)),
            port,
            agent: Server::agent(call),
            call: Mutex::new(()),
        };
        server.wait_healthy(load)?;
        Ok(server)
    }

    /// A `Server` over an already-running listener: the seam the loopback tests drive, and nothing
    /// production ever calls. There is no child, so `stop` and `Drop` are no-ops on it.
    pub fn for_port(port: u16, call: Duration) -> Server {
        Server { child: Mutex::new(None), port, agent: Server::agent(call), call: Mutex::new(()) }
    }

    /// `proxy(None)` deliberately: a machine with `HTTP_PROXY` set — a campus network, a VPN client —
    /// would otherwise send a loopback request through a proxy that cannot reach 127.0.0.1, and the
    /// judge would fail on exactly the networks a student is most often on.
    fn agent(call: Duration) -> ureq::Agent {
        ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(call))
            .http_status_as_error(true)
            .build()
            .into()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Poll `/health` until it answers or `load` runs out — and give up at once if the child has
    /// already exited, which is what a bad model file or an incompatible build looks like. Without
    /// that check a wrong `.gguf` costs the full five minutes before it says anything.
    fn wait_healthy(&self, load: Duration) -> Result<(), ModelError> {
        let url = format!("http://127.0.0.1:{}/health", self.port);
        let deadline = Instant::now() + load;
        loop {
            if let Some(child) = self.child.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(ModelError::Failed(format!(
                        "llama-server exited before it was ready ({status}) — is the model file a supported .gguf?"
                    )));
                }
            }
            if self.agent.get(&url).call().is_ok() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(ModelError::Failed(format!(
                    "llama-server did not become healthy within {}s",
                    load.as_secs()
                )));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// One grammar-constrained completion. Returns the model's raw text; parsing it is `judge`'s.
    ///
    /// `temperature: 0.0`, `top_k: 1` and `seed: 0` because a judgment is not a creative writing
    /// task: the same note judged twice by the same model should answer the same way, which is as
    /// close to the engine's determinism rule as tier 3 can get. `cache_prompt` is on because every
    /// prompt in a batch shares a long identical preamble.
    pub fn complete(&self, prompt: &str, grammar: &str) -> Result<String, ModelError> {
        let _flight = self.call.lock().unwrap_or_else(|e| e.into_inner());
        // Through `dumps_value` like every other JSON this crate writes (CLAUDE.md).
        let body = crate::ledger::dumps_value(&serde_json::json!({
            "prompt": prompt,
            "grammar": grammar,
            "n_predict": 256,
            "temperature": 0.0,
            "top_k": 1,
            "seed": 0,
            "cache_prompt": true,
            "stream": false,
        }));
        let url = format!("http://127.0.0.1:{}/completion", self.port);
        let mut response = self
            .agent
            .post(&url)
            .header("content-type", "application/json")
            .send(body)
            .map_err(|e| ModelError::Failed(format!("llama-server: {e}")))?;
        let text = response
            .body_mut()
            .with_config()
            .limit(1 << 20)
            .read_to_string()
            .map_err(|e| ModelError::Failed(format!("llama-server: {e}")))?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| ModelError::Failed(format!("llama-server: reply is not JSON ({e})")))?;
        value
            .get("content")
            .and_then(|c| c.as_str())
            .map(str::to_string)
            .ok_or_else(|| ModelError::Failed("llama-server: no content in the reply".to_string()))
    }

    /// Idempotent: the child is taken out of the mutex, so a `stop` followed by `Drop` kills once.
    pub fn stop(&self) {
        let taken = self.child.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut child) = taken {
            kill_tree(&mut child);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}
```

- [ ] **Step 6: Declare the module**

In `src/lib.rs`, immediately after `pub mod judge;`:

```rust
// Knowlu plan 3a — the judge's process half: llama.cpp on loopback, one call at a time, killed on
// drop. Consumed as a release artefact, never linked (spec §5.3 as amended 2026-09-07).
pub mod runtime;
```

- [ ] **Step 7: Run the tests**

Run (repo root): `cargo test runtime::`
Expected: PASS, 0 warnings.

- [ ] **Step 8: The full engine gate and commit**

```bash
cargo test 2>&1 | tail -20
```

Expected: green, 0 warnings, **2 ignored** (traps 4 and 5 — the third, the real-runtime smoke test, arrives in Task 4).

**Both dual-run scripts, every time, no carve-out.** This task touches `src/`, and the plan's Global Constraints bind that gate to every task that does. They run in seconds; a per-task judgement about whether they are "really needed" is exactly the check that gets skipped under time pressure, and the one that would catch an accidental change to a shared module.

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: both exit 0.

```bash
git add src/runtime.rs src/lib.rs
git commit -F .git/COMMIT_MSG_3A3
```

```
feat(runtime): llama.cpp on loopback — start, health, one call at a time, killed on drop (plan 3a, Task 3)

src/runtime.rs owns the process half of the judge seam. resolve() finds the runtime by argument,
then KNOWLU_LLAMA_SERVER, then a sibling llama-server.exe — the resolution order
scheduler::engine_exe already uses — and says which of the two halves is missing. Server binds an
ephemeral loopback port, waits for /health (giving up at once if the child died, which is what a
bad .gguf looks like), holds one mutex for the whole of every completion, bounds each call, and
kills the whole process tree on drop.

Deviation recorded in the module doc (ruling R-P3a-1): the amended spec §5.3 says
bundle.externalBin. The process boundary is kept and the bundling is not, because the runtime is
optional, versions far faster than this app, and is the same kind of artefact as the models §5.3
already downloads after install. Not a size argument - the repo's own figure is the product plan's
+20-50 MB. resolve()'s sibling branch leaves the door open to bundling later.

No test starts llama-server or downloads anything. The HTTP client is exercised against a
TcpListener on 127.0.0.1 inside the test itself; the one test needing a real model is #[ignore]d
with the reason in its attribute, like traps 4 and 5.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 4: The grammar, the prompt, the reply — and `Server` becomes a `Model`

Tier 3's contents. The five-field schema the grammar constrains, the prompt that grounds it, the parser that clamps it, and the one `impl` that joins Task 2's trait to Task 3's process.

**Files:**
- Modify: `src/judge.rs` (`GRAMMAR`, `prompt_for`, `parse_reply`, and tests), `src/runtime.rs` (`impl judge::Model for Server`)
- Read first: `docs/runners/cloud-routine-prompt.md` line 10–14 — what step 3 is told to decide, and what it is told to write; this reproduces it locally

**Interfaces:**
- Consumes: `judge::{Item, Heuristics, Verdict, ModelError, clip, one_line, MAX_BODY_CHARS, MAX_REASON_CHARS}`; `runtime::Server::complete`.
- Produces:
  - `judge::GRAMMAR: &str`
  - `judge::prompt_for(item: &Item, h: &Heuristics, seed: &Verdict) -> String`
  - `judge::parse_reply(text: &str) -> Result<Verdict, ModelError>`
  - `impl judge::Model for runtime::Server`

- [ ] **Step 1: Write the failing tests** — append to `src/judge.rs`'s `mod tests`:

```rust
    /// D6: grammar-constrained decoding on every call without exception. The grammar has to name
    /// all five fields, in the order the emitter will produce them, and it must be plain GBNF —
    /// llama-server rejects a malformed rule set and every judgment then fails identically.
    #[test]
    fn the_grammar_names_the_five_fields_and_only_those() {
        for f in ["course", "effort_hours", "importance", "importance_reason", "confidence"] {
            assert!(GRAMMAR.contains(&format!("\\\"{f}\\\"")), "grammar must pin {f}: {GRAMMAR}");
        }
        assert!(GRAMMAR.starts_with("root ::= "), "GBNF starts at root");
        assert!(GRAMMAR.contains("importance ::= \"1\" | \"2\" | \"3\" | \"4\" | \"5\""), "1-5, in the grammar, not in a prompt");
        assert!(!GRAMMAR.contains("effort_source"), "effort_source is the engine's, never the model's");
        assert!(!GRAMMAR.contains("needs_enrichment"), "the flag is the pass's decision, not the model's");
    }

    /// R6: nothing but the note, the course's weights and the preferences reaches the model. In
    /// particular no config file, no path, no credential target.
    #[test]
    fn the_prompt_carries_the_note_and_its_grounding_and_nothing_else() {
        let mut h = heur();
        h.weights.insert("cs-100".to_string(), "- Homework: 20% of the grade.".to_string());
        h.preferences = "- German is near-daily small work.".to_string();
        let mut i = item("CS-100 Homework 3");
        i.body = "Chapters 4 and 5. Submit on the course page.".to_string();
        i.source_uid = "blackboard:nothing".to_string();
        let seed = tier1(&i, &h);
        let p = prompt_for(&i, &h, &seed);

        assert!(p.contains("CS-100 Homework 3"), "the title");
        assert!(p.contains("Chapters 4 and 5"), "the body");
        assert!(p.contains("2026-10-01T23:59"), "the due date");
        assert!(p.contains("cs-100"), "the resolved slug");
        assert!(p.contains("Homework: 20% of the grade."), "the grade weights ground importance");
        assert!(p.contains("German is near-daily"), "preferences are honoured, as the routine honours them");
        assert!(p.contains("2 hours"), "the vault's own session length anchors the estimate");
        assert!(!p.contains("ingest.yaml") && !p.contains("credential"), "R6");
        assert!(p.ends_with("JSON:"), "the model is asked for the object and nothing else");
    }

    /// A note whose body is a whole syllabus must not push the question out of the context window.
    #[test]
    fn the_prompt_is_bounded_however_long_the_note_is() {
        let mut i = item("Long one");
        i.body = "x".repeat(50_000);
        let mut h = heur();
        h.preferences = "p".repeat(50_000);
        h.weights.insert("cs-100".to_string(), "w".repeat(50_000));
        i.course = Some("cs-100".to_string());
        let p = prompt_for(&i, &h, &tier1(&i, &h));
        assert!(p.len() < 6_000, "prompt was {} chars", p.len());
        assert!(p.contains("Long one"), "the title survives the clipping");
    }

    /// A course with no note, and a note with no due date and no body: the prompt still stands up.
    #[test]
    fn the_prompt_survives_a_note_with_nothing_in_it() {
        let mut i = item("Untitled");
        i.due = None;
        i.body = String::new();
        i.source_uid = "blackboard:nothing".to_string();
        let h = Heuristics {
            course_map: Vec::new(),
            slice_hours: 2.0,
            weights: std::collections::BTreeMap::new(),
            preferences: String::new(),
        };
        let p = prompt_for(&i, &h, &tier1(&i, &h));
        assert!(p.contains("Course slug: null"));
        assert!(!p.contains("Due:"), "an absent due date is absent, never the word None");
        assert!(p.ends_with("JSON:"));
    }

    #[test]
    fn parse_reply_reads_the_object_clamps_it_and_makes_the_reason_one_line() {
        let v = parse_reply(
            r#" here you go {"course": "cs-100", "effort_hours": 2.5, "importance": 4, "importance_reason": "Worth  15%\nof the grade.", "confidence": 0.82} thanks"#,
        )
        .expect("an object anywhere in the reply");
        assert_eq!(v.course.as_deref(), Some("cs-100"));
        assert_eq!(v.effort_hours, Some(2.5));
        assert_eq!(v.importance, Some(4));
        assert_eq!(v.importance_reason.as_deref(), Some("Worth 15% of the grade."));
        assert_eq!(v.confidence, 0.82);
        assert_eq!(v.tier, 3);

        // Clamped, never refused: the grammar bounds importance, and effort and confidence are
        // bounded here so a runaway number cannot reach a note or a ranking.
        let v = parse_reply(r#"{"course": null, "effort_hours": 900, "importance": 9, "importance_reason": "x", "confidence": 5}"#).unwrap();
        assert_eq!(v.effort_hours, Some(40.0));
        assert_eq!(v.importance, Some(5));
        assert_eq!(v.confidence, 1.0);
        assert_eq!(v.course, None);

        let v = parse_reply(r#"{"course": "  ", "effort_hours": 0.01, "importance": 0, "importance_reason": "   ", "confidence": -1}"#).unwrap();
        assert_eq!(v.course, None, "a blank slug is no slug");
        assert_eq!(v.effort_hours, Some(0.25));
        assert_eq!(v.importance, Some(1));
        assert_eq!(v.importance_reason, None, "a blank reason is no reason, so the verdict is incomplete");
        assert_eq!(v.confidence, 0.0);
    }

    #[test]
    fn parse_reply_refuses_what_it_cannot_read() {
        assert!(parse_reply("no object here").is_err());
        assert!(parse_reply("{not json}").is_err());
        assert!(parse_reply(r#"{"importance": 3, "importance_reason": "x", "confidence": 1}"#).is_err(), "effort_hours is required");
        assert!(parse_reply(r#"{"effort_hours": 2, "importance_reason": "x", "confidence": 1}"#).is_err(), "importance is required");
    }

    /// The reason becomes a frontmatter line, so it has to survive the write path's own guard.
    #[test]
    fn a_generated_reason_is_always_writable_as_one_line() {
        let v = parse_reply(&format!(
            r#"{{"course": null, "effort_hours": 1, "importance": 3, "importance_reason": "{}", "confidence": 1}}"#,
            "long ".repeat(200)
        ))
        .unwrap();
        let reason = v.importance_reason.unwrap();
        assert!(reason.chars().count() <= MAX_REASON_CHARS);
        assert!(crate::write::single_line_problem(&reason).is_none(), "{reason}");
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run (repo root): `cargo test judge::tests::the_grammar judge::tests::the_prompt judge::tests::parse_reply judge::tests::a_generated`
Expected: FAIL to compile — `cannot find value GRAMMAR in this scope`.

- [ ] **Step 3: The grammar**

Append to `src/judge.rs`, above `mod tests`:

```rust
/// The GBNF grammar every model call is constrained by (D6, product plan §2.3: grammar-constrained
/// decoding on every call without exception).
///
/// **Five fields, and the grammar is what enforces four of the five rules.** `importance` is an
/// alternation of the five literals rather than a number the prompt asks to be in range, so a 7 is
/// not merely unlikely but unrepresentable. `course` is a string or the literal `null`, so "I could
/// not tell" has a spelling. `confidence` is the field the tiers key on and is generated last, after
/// the answer it is about.
///
/// Written as a raw string with real newlines: llama.cpp's parser wants one rule per line, and a
/// grammar that fails to parse fails every judgment identically and silently.
///
/// **Not in here:** `effort_source` (the engine sets it), `effort_confidence` (always `low`, as the
/// routine writes it) and `needs_enrichment` (the pass's decision about its own completeness, never
/// the model's opinion of it).
pub const GRAMMAR: &str = r#"root ::= "{" ws "\"course\":" ws course "," ws "\"effort_hours\":" ws number "," ws "\"importance\":" ws importance "," ws "\"importance_reason\":" ws string "," ws "\"confidence\":" ws number ws "}"
course ::= "null" | string
importance ::= "1" | "2" | "3" | "4" | "5"
string ::= "\"" ([^"\\] | "\\" ["\\/bfnrt])* "\""
number ::= [0-9]+ ("." [0-9]+)?
ws ::= " "?
"#;
```

- [ ] **Step 4: The prompt**

Append below `GRAMMAR`:

```rust
/// What tier 3 is asked. Built from the note, the course's grade weights and
/// `profile/preferences.md`, and from nothing else (R6).
///
/// It is the cloud routine's step 3, written down: attribute the course if you can, estimate a
/// realistic effort, ground importance in the grade weights, default to 3 and say so when there are
/// none, and write one line of reason. The differences from the routine are that the rules are
/// stated to a 1–4 B model rather than to Claude, and that the output shape is a grammar rather
/// than a command line.
///
/// **Every free-text input is clipped HERE, at the point of use** — `MAX_BODY_CHARS`,
/// `MAX_WEIGHTS_CHARS`, `MAX_PREFS_CHARS` — and not only inside `Heuristics::load`. Clipping in
/// `load` alone would make this function's bound a property of one caller rather than of the
/// function: a test, or any future caller that builds a `Heuristics` by hand, would get a prompt of
/// whatever size the vault happened to hold. Clipped here, a note carrying a whole syllabus can
/// never push the question out of a 4096-token context, whoever assembled the inputs.
pub fn prompt_for(item: &Item, h: &Heuristics, seed: &Verdict) -> String {
    let slug = seed.course.as_deref().unwrap_or("null");
    let weights = seed.course.as_deref().and_then(|c| h.weights.get(c)).map(String::as_str).unwrap_or("");
    let mut p = String::new();
    p.push_str("You estimate effort and importance for one university assignment.\n");
    p.push_str("Answer with the JSON object and nothing else.\n\n");
    p.push_str("Rules:\n");
    p.push_str("- effort_hours: the realistic time in hours to finish this one assignment, 0.25 to 40.\n");
    p.push_str(&format!("- a typical work session in this student's plan is {} hours.\n", h.slice_hours));
    p.push_str("- importance: 1 to 5, grounded in the grade weights below. With no weights given, answer 3 and say so in importance_reason.\n");
    p.push_str("- importance_reason: one line, under 140 characters, no line breaks.\n");
    p.push_str("- course: the slug given below, or null when the slug given is null.\n");
    p.push_str("- confidence: 0 to 1, how sure you are of effort_hours and importance.\n\n");
    if !h.preferences.is_empty() {
        p.push_str("The student's stated preferences:\n");
        p.push_str(&clip(&h.preferences, MAX_PREFS_CHARS));
        p.push_str("\n\n");
    }
    p.push_str(&format!("Course slug: {slug}\n"));
    if !weights.is_empty() {
        p.push_str("Grade weights:\n");
        p.push_str(&clip(weights, MAX_WEIGHTS_CHARS));
        p.push('\n');
    }
    p.push_str(&format!("Title: {}\n", one_line(&item.title, 200)));
    // An absent due date is absent, never the word `None`: a model shown "Due: None" reliably
    // treats it as a date it failed to read rather than as an assignment without one.
    if let Some(due) = item.due.as_deref().filter(|d| !d.is_empty()) {
        p.push_str(&format!("Due: {due}\n"));
    }
    let body = clip(item.body.trim(), MAX_BODY_CHARS);
    if !body.is_empty() {
        p.push_str("Body:\n");
        p.push_str(&body);
        p.push('\n');
    }
    p.push_str("\nJSON:");
    p
}

/// Read the model's text back into a `Verdict`.
///
/// **Tolerant about the envelope, strict about the contents.** The grammar guarantees the object,
/// but a build that ignores the grammar, or a future runtime that wraps it in prose, must not take
/// the whole judge step down — so the first `{` to the last `}` is what gets parsed. Inside that,
/// `effort_hours` and `importance` are required (a verdict without them answers nothing), the two
/// numbers are **clamped rather than refused** (a 900-hour estimate is a bad answer, not a broken
/// one, and clamping keeps the confidence floor as the single place a bad answer is rejected), and
/// the reason is collapsed to one clipped line so `write`'s single-line guard can never refuse it.
///
/// A blank `course` or a blank reason is `None`, not `Some("")`: an empty string would be written
/// into frontmatter and would then read as "attributed to nothing", which is a different claim.
pub fn parse_reply(text: &str) -> Result<Verdict, ModelError> {
    let bad = |m: &str| ModelError::Failed(format!("the model's reply {m}"));
    let start = text.find('{').ok_or_else(|| bad("carried no JSON object"))?;
    let end = text.rfind('}').ok_or_else(|| bad("carried no JSON object"))?;
    if end < start {
        return Err(bad("carried no JSON object"));
    }
    let value: serde_json::Value =
        serde_json::from_str(&text[start..=end]).map_err(|e| bad(&format!("is not JSON ({e})")))?;
    let effort = value
        .get("effort_hours")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| bad("has no effort_hours"))?;
    let importance = value
        .get("importance")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| bad("has no importance"))?;
    let course = value
        .get("course")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let reason = value.get("importance_reason").and_then(serde_json::Value::as_str).unwrap_or("");
    let reason = one_line(reason, MAX_REASON_CHARS);
    let confidence = value.get("confidence").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    Ok(Verdict {
        course,
        effort_hours: Some(effort.clamp(0.25, 40.0)),
        importance: Some(importance.clamp(1, 5)),
        importance_reason: (!reason.is_empty()).then_some(reason),
        confidence: confidence.clamp(0.0, 1.0),
        tier: 3,
    })
}
```

- [ ] **Step 5: Join the two halves, and the smoke test that needed both**

Append to `src/runtime.rs`, above its `mod tests`:

```rust
/// Tier 3, joined up: build the prompt, send it under the grammar, parse what comes back.
///
/// Three lines, and deliberately so — every decision about the schema, the wording and the bounds
/// lives in `judge`, where it is testable without a process, and every decision about the transport
/// lives here, where it is testable without a model.
impl judge::Model for Server {
    fn judge(
        &self,
        item: &judge::Item,
        h: &judge::Heuristics,
        seed: &judge::Verdict,
    ) -> Result<judge::Verdict, ModelError> {
        let text = self.complete(&judge::prompt_for(item, h, seed), judge::GRAMMAR)?;
        judge::parse_reply(&text)
    }
}
```

And append to `src/runtime.rs`'s `mod tests` — **here, not in Task 3.** `#[ignore]` suppresses running, never compiling, and this test names `judge::GRAMMAR` and `judge::parse_reply`, which did not exist until this task. Written earlier it would have broken the build at the end of Task 3, with `cargo test` reporting a compile error in a test nobody ever runs:

```rust
    /// The one test that needs a real runtime and a real model — the whole tier-3 path, end to end:
    /// a process starts, a model loads, the grammar constrains the output, and `parse_reply` reads
    /// it back. `#[ignore]`d with the reason in the attribute, the way traps 4 and 5 already are
    /// (spec §8), so `cargo test` never downloads anything and never starts llama-server.
    #[test]
    #[ignore = "needs a real llama-server.exe and a .gguf: set KNOWLU_LLAMA_SERVER and KNOWLU_MODEL, then cargo test -- --ignored real_runtime"]
    fn real_runtime_loads_a_model_and_answers_under_the_grammar() {
        let (rt, gg) = resolve(None, None).expect("set KNOWLU_LLAMA_SERVER and KNOWLU_MODEL");
        let s = Server::start(&rt, &gg, LOAD_TIMEOUT, CALL_TIMEOUT).expect("the server starts");
        let out = s
            .complete("Title: CS 100 Homework 3\nDue: 2026-10-01\nBody:\nSubmit online.\n\nJSON:", crate::judge::GRAMMAR)
            .expect("a completion");
        let v = crate::judge::parse_reply(&out).expect("the grammar guarantees a parseable object");
        assert!(v.importance.is_some() && v.effort_hours.is_some(), "{out}");
        assert!((0.0..=1.0).contains(&v.confidence), "{out}");
    }
```

- [ ] **Step 6: Run the tests**

Run (repo root): `cargo test judge:: runtime::`
Expected: PASS, 0 warnings.

- [ ] **Step 7: Full gate and commit**

```bash
cargo test 2>&1 | tail -20
```

Expected: green, 0 warnings, **3 ignored** — traps 4 and 5, plus `real_runtime_loads_a_model_and_answers_under_the_grammar`, which arrives with this task because it needs `judge::GRAMMAR` and `judge::parse_reply` to compile at all.

**Both dual-run scripts, no carve-out** (this task touches `src/`):

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: both exit 0.

```bash
git add src/judge.rs src/runtime.rs
git commit -F .git/COMMIT_MSG_3A4
```

```
feat(judge): the grammar, the prompt and the reply parser; Server becomes a Model (plan 3a, Task 4)

Five fields under GBNF — course, effort_hours, importance, importance_reason, confidence — with
importance an alternation of the five literals so an out-of-range answer is unrepresentable rather
than merely unlikely. The prompt is the cloud routine's step 3 written down: the course slug tier 1
resolved, the course note's grade weights, profile/preferences.md, the vault's own session length,
the title, the due date and a clipped body — and nothing else (R6). parse_reply is tolerant about
the envelope and strict about the contents: required fields, clamped numbers, a reason collapsed to
one clipped line that write's single-line guard can never refuse.

impl Model for Server is three lines: the schema is judge's and testable without a process, the
transport is runtime's and testable without a model.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 5: `src/judgelog.rs` — ids and field values, in app data, never in the vault

Spec §5.4: every model call is logged — inputs **by id**, the output, the confidence — to the profile's app-data directory, so neither the backup nor a future second device carries raw content. §5.6: ids, field values and confidences, never a message body, never an event description. This is the seed corpus 3b's rule promotion learns from.

**Files:**
- Create: `src/judgelog.rs` (with its own `mod tests`)
- Modify: `src/lib.rs` (one `pub mod`)
- Read first: `src/ledger.rs:69–110` (`JsonlLedger::new`/`append` — the day file, the `ts` requirement, the fsync), `src/uievents.rs` (the closest existing "ids only, never text" ledger)

**Interfaces:**
- Consumes: `ledger::{JsonlLedger, Record}`; `journal::{now_ts, device_name}`; `judge::{Outcome, Verdict}`.
- Produces:
  - `judgelog::Entry<'a> { id, tier, outcome, confidence, fields, run_id, ms }`
  - `judgelog::record(dir: &Path, entry: &Entry<'_>, now: Option<jiff::Timestamp>) -> Result<(), String>`
  - `judgelog::read_day(dir: &Path, day: &str) -> Vec<serde_json::Value>`
  - `judgelog::entry_for<'a>(id: &'a str, outcome: &Outcome, fields: &'a [(String, String)], run_id: Option<&'a str>, ms: i64) -> Entry<'a>`

- [ ] **Step 1: Write the failing tests** — create `src/judgelog.rs` with only the module doc and this `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qo-judgelog-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn fields() -> Vec<(String, String)> {
        vec![
            ("course".to_string(), "cs-100".to_string()),
            ("effort_hours".to_string(), "2.5".to_string()),
            ("importance".to_string(), "4".to_string()),
        ]
    }

    #[test]
    fn a_record_lands_in_the_utc_day_file_and_reads_back() {
        let d = dir("day");
        let now: jiff::Timestamp = "2026-09-07T23:30:00Z".parse().unwrap();
        let f = fields();
        let e = Entry {
            id: "task_0123456789",
            tier: 3,
            outcome: "answered",
            confidence: 0.82,
            fields: &f,
            run_id: Some("local-2026-09-07T18:00:00Z"),
            ms: 940,
        };
        record(&d, &e, Some(now)).unwrap();
        assert!(d.join("2026-09-07.jsonl").is_file(), "the UTC day of the record's own ts");
        let back = read_day(&d, "2026-09-07");
        assert_eq!(back.len(), 1);
        let r = &back[0];
        assert_eq!(r["id"], "task_0123456789");
        assert_eq!(r["tier"], 3);
        assert_eq!(r["outcome"], "answered");
        assert_eq!(r["confidence"], 0.82);
        assert_eq!(r["run_id"], "local-2026-09-07T18:00:00Z");
        assert_eq!(r["ms"], 940);
        assert_eq!(r["fields"]["course"], "cs-100");
        assert_eq!(r["fields"]["importance"], "4");
        assert!(r["device"].is_string(), "which machine judged it");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// D9 / spec §5.6 — the one property this module exists to have. A title and a body are note
    /// CONTENT: they may go to the model, they may never go to a log that a second device or a
    /// support request could carry.
    #[test]
    fn no_note_text_ever_reaches_a_log_line() {
        let d = dir("private");
        let f = vec![("importance_reason".to_string(), "Worth 15% of the grade.".to_string())];
        let e = Entry {
            id: "task_0123456789",
            tier: 3,
            outcome: "answered",
            confidence: 0.9,
            fields: &f,
            run_id: None,
            ms: 12,
        };
        record(&d, &e, Some("2026-09-07T12:00:00Z".parse().unwrap())).unwrap();
        let text = std::fs::read_to_string(d.join("2026-09-07.jsonl")).unwrap();
        // `importance_reason` IS a field value and is meant to be here; a title and a body are not,
        // and `Entry` has nowhere to put them — this asserts the shape as well as the content.
        assert!(text.contains("Worth 15% of the grade."), "a written field value is the record");
        for forbidden in ["Title:", "Body:", "prompt", "tasks/", "\\path"] {
            assert!(!text.contains(forbidden), "{forbidden} must never reach a log line: {text}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Every outcome is logged, not only the ones that wrote something: a week of "model not
    /// installed" is exactly what tells Quinn why nothing is being enriched.
    #[test]
    fn every_outcome_is_recorded_with_the_tier_that_reached_it() {
        let empty: Vec<(String, String)> = Vec::new();
        let v = crate::judge::Verdict { confidence: 1.0, tier: 1, ..Default::default() };
        let e = entry_for("task_0123456789", &crate::judge::Outcome::RuntimeNotInstalled(v.clone()), &empty, None, 3);
        assert_eq!(e.outcome, "runtime not installed");
        assert_eq!(e.tier, 1);
        assert_eq!(e.confidence, 1.0);

        let low = crate::judge::Outcome::LowConfidence { seed: v, why: "confidence 0.41 below 0.60".to_string() };
        let e = entry_for("task_0123456789", &low, &empty, None, 5);
        assert_eq!(e.outcome, "low confidence");
    }

    /// A directory that cannot be created is an error the caller reports, never a panic and never a
    /// reason to stop enriching: the judgment is the work, the log is the record of it.
    #[test]
    fn a_log_that_cannot_be_written_is_an_error_not_a_panic() {
        let d = dir("blocked");
        std::fs::create_dir_all(d.parent().unwrap()).unwrap();
        // A FILE where the directory should be: `create_dir_all` fails on it every time.
        std::fs::write(&d, b"not a directory").unwrap();
        let f = fields();
        let e = Entry { id: "task_0123456789", tier: 1, outcome: "answered", confidence: 1.0, fields: &f, run_id: None, ms: 1 };
        assert!(record(&d, &e, None).is_err());
        let _ = std::fs::remove_file(&d);
    }

    #[test]
    fn read_day_of_a_missing_directory_is_empty() {
        assert!(read_day(&dir("absent"), "2026-09-07").is_empty());
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (repo root): `cargo test judgelog::`
Expected: FAIL to compile — `cannot find type Entry in this scope`.

- [ ] **Step 3: Write the module**

```rust
//! The judgment log (Knowlu spec §5.4, §5.6, decision 13): what was judged, by which tier, how
//! sure, and what was written — **by id and by field value, never by content.**
//!
//! **Where it is not.** Not in the vault, so `backup::tick` (which mirrors the vault and nothing
//! else) never copies it, `history::sync` never commits it, and a second device replaying the
//! journal never receives it. It lives under the *profile's* app-data directory —
//! `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments\` — which is per profile because two vaults on one
//! machine are two different sets of judgments, unlike `updates\`, which is one bundle for the whole
//! install.
//!
//! **What it is for.** Two things. Today it is the only place that answers "why is nothing being
//! enriched?" — a week of `model not installed` lines is the answer. Tomorrow it is 3b's seed
//! corpus: a rule that reproduces the model's output across repeated cases is what gets proposed for
//! promotion into tier 2, and this file is the evidence for that.
//!
//! **The shape is a `JsonlLedger`**, the same day-file, `merge=union`, fsync-on-append primitive the
//! journal and the interaction events use — so the `ts`-decides-the-day rule, the sort order and
//! `ledger::dumps_value`'s Python separators all come for free and cannot drift from the other two.

use std::path::Path;

use serde_json::{json, Value};

use crate::journal::{device_name, now_ts};
use crate::ledger::{JsonlLedger, Record};

/// One judgment, ready to record.
///
/// **There is nowhere in this struct to put a title, a body or a prompt**, and that is the design:
/// §5.6 is enforced by the type, not by the discipline of whoever calls it. `fields` is what was (or
/// would have been) written — field name to the literal — which are values the note itself carries
/// in the open.
#[derive(Debug, Clone)]
pub struct Entry<'a> {
    /// The note's opaque `id:`.
    pub id: &'a str,
    /// 0 nothing, 1 heuristics, 2 a promoted rule, 3 the model.
    pub tier: u8,
    /// `Outcome::label()`.
    pub outcome: &'a str,
    pub confidence: f64,
    pub fields: &'a [(String, String)],
    pub run_id: Option<&'a str>,
    /// How long the judgment took, milliseconds — what tells a slow model from a slow disk.
    pub ms: i64,
}

/// Build an `Entry` from an `Outcome`, so the tier, the confidence and the label are read off the
/// outcome in one place rather than at each call site.
pub fn entry_for<'a>(
    id: &'a str,
    outcome: &crate::judge::Outcome,
    fields: &'a [(String, String)],
    run_id: Option<&'a str>,
    ms: i64,
) -> Entry<'a> {
    let verdict = outcome.verdict();
    Entry {
        id,
        tier: verdict.tier,
        outcome: outcome.label(),
        confidence: verdict.confidence,
        fields,
        run_id,
        ms,
    }
}

/// Append one line to `<dir>/<UTC day>.jsonl`.
///
/// `Err` when the directory cannot be made or the append fails. The caller **reports it and carries
/// on**: the judgment is the work and the log is the record of it, so a full disk must not stop a
/// slot from enriching.
pub fn record(dir: &Path, entry: &Entry<'_>, now: Option<jiff::Timestamp>) -> Result<(), String> {
    let mut fields = serde_json::Map::new();
    for (name, literal) in entry.fields {
        fields.insert(name.clone(), Value::String(literal.clone()));
    }
    let mut rec: Record = Record::new();
    rec.insert("ts".into(), json!(now_ts(now)));
    rec.insert("device".into(), json!(device_name()));
    rec.insert("id".into(), json!(entry.id));
    rec.insert("tier".into(), json!(entry.tier));
    rec.insert("outcome".into(), json!(entry.outcome));
    rec.insert("confidence".into(), json!(entry.confidence));
    rec.insert("fields".into(), Value::Object(fields));
    rec.insert("run_id".into(), entry.run_id.map(|r| json!(r)).unwrap_or(Value::Null));
    rec.insert("ms".into(), json!(entry.ms));
    JsonlLedger::new(dir).append(&rec).map_err(|e| format!("judgment log: {e}"))
}

/// One day's records, in file order. Used by the app's settings row and by 3b's promotion loop; a
/// missing directory or an unreadable line is empty or skipped, never an error — this is a report,
/// not a source of truth.
pub fn read_day(dir: &Path, day: &str) -> Vec<Value> {
    let Ok(text) = std::fs::read_to_string(dir.join(format!("{day}.jsonl"))) else {
        return Vec::new();
    };
    text.split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}
```

- [ ] **Step 4: Declare the module**

In `src/lib.rs`, after `pub mod runtime;`:

```rust
// Knowlu plan 3a — what was judged, by id and by field value, into the profile's app data and never
// into the vault (spec §5.4, §5.6). 3b's rule promotion reads it.
pub mod judgelog;
```

- [ ] **Step 5: Run the tests, the gate, and commit**

Run (repo root): `cargo test judgelog::` — expected PASS. Then `cargo test` — green, 0 warnings, 3 ignored.

**Both dual-run scripts, no carve-out** (this task touches `src/`):

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: both exit 0.

```bash
git add src/judgelog.rs src/lib.rs
git commit -F .git/COMMIT_MSG_3A5
```

```
feat(judgelog): the judgment log — ids and field values, in app data, never the vault (plan 3a, Task 5)

A JsonlLedger under the profile's app-data directory: id, tier, outcome, confidence, the fields
written, run_id, and how long it took. Entry has nowhere to put a title, a body or a prompt, so
spec §5.6 is enforced by the type rather than by discipline — a test asserts a distinctive body
token never reaches a line. Not in the vault, so backup::tick never mirrors it and history::sync
never commits it. Every outcome is logged, not only the ones that wrote something: a week of
"model not installed" is exactly what says why nothing is being enriched. 3b's rule promotion
reads it as its seed corpus.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 6: `src/write.rs` learns to re-propose — `propose_amendment`, ported

**This is the plan's one change to the write path, and it closes a real gap.** CLAUDE.md and the Knowlu spec both say a judged write against a field Quinn set files a `kind: amend` approval. In the Rust port that path is `WriteError::ProposeNotPorted` (`src/write.rs:356`) unless an identical proposal already happens to be pending. Enrichment is exactly where it fires — the routine has been re-judging effort and importance twice a day for weeks — so Task 7 cannot honour judge-once without this.

The Python is `engine/write.py:333–353` and it is thirteen lines. The gate is both dual-run scripts, extended: `scripts/diff-engines-notes.ps1`'s header currently says `--propose` is "unported by decision", and this task deletes that sentence and adds the pair.

**Files:**
- Modify: `src/write.rs` (`AMEND_BUTTONS`, `propose_amendment`, the `proposed` branch, `WriteError`), `scripts/diff-engines-notes.ps1`
- Read first: `engine/write.py:287–307` (`AMEND_BUTTONS`) and `:333–353` (`propose_amendment`); `src/approvals.rs:339–420` (`validate_amendment` — what the proposal must satisfy to be applicable); `src/info.rs:87–104` (how this crate mints a note from a `yamlemit::Node`)

**Interfaces:**
- Consumes: `write::{create, find_pending_amendment, WriteContext, WriteError}`; `ids::{new_id, rel}`; `yaml::to_json`; `yamlemit::{safe_dump_block, Node}`; `journal::Journal`.
- Produces:
  - `write::AMEND_BUTTONS: &str`
  - `write::propose_amendment(vault, target_path, meta, changes, ctx, journal, evidence, today) -> Result<PathBuf, WriteError>`
  - `WriteError::ProposeNotPorted` **removed** (a compile error at every remaining reference is the point)

- [ ] **Step 1: Write the failing tests** — append to `src/write.rs`'s `mod tests`:

```rust
    fn propose_vault(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-write-propose-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        std::fs::create_dir_all(d.join("approvals")).unwrap();
        pystr::write_text(
            &d.join("tasks").join("t.md"),
            "---\ntitle: \"CS 100 HW 01\"\ncourse: cs-100\neffort_hours: 2.5\nimportance: 2\nstatus: active\nprogress: 0\nid: task_0123456789\n---\n\nBody.\n",
        ).unwrap();
        d
    }

    /// The judge-once loop, end to end: Quinn sets a field, an agent re-judges it, and instead of
    /// overwriting it the write path mints a `kind: amend` card the deck can approve. This is the
    /// path that returned `ProposeNotPorted` before this task.
    #[test]
    fn a_judged_write_over_a_field_quinn_set_files_an_amendment() {
        let v = propose_vault("files");
        let mut journal = Journal::new(&v);
        // Quinn sets it by hand.
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        // The agent re-judges it.
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();

        let path = res.proposal.expect("an amendment was filed");
        assert!(path.starts_with(v.join("approvals")), "{path:?}");
        let text = pystr::read_text(&path).unwrap();
        let (meta, body) = crate::models::split_frontmatter(&text).unwrap();
        assert_eq!(get_str(&meta, "type").as_deref(), Some("approval"));
        assert_eq!(get_str(&meta, "kind").as_deref(), Some("amend"));
        assert_eq!(get_str(&meta, "status").as_deref(), Some("pending"));
        assert_eq!(get_str(&meta, "target").as_deref(), Some("tasks/t.md"));
        assert_eq!(get_str(&meta, "created_by").as_deref(), Some("agent:knowlu.enrich"));
        // S1: `first_proposed_at` is set once and is what `age_days` reads.
        assert_eq!(get_str(&meta, "first_proposed_at"), get_str(&meta, "proposed_at"));
        assert!(body.contains("re-judged effort_hours"), "{body}");
        assert!(body.contains("meta-bind-button"), "the deck's two buttons, as every proposal has");

        // The note itself is untouched — that is the whole point.
        let note = pystr::read_text(&v.join("tasks").join("t.md")).unwrap();
        assert!(note.contains("effort_hours: 4.0"), "judge-once held: {note}");

        // And the proposal is one the engine can actually apply.
        assert!(crate::approvals::validate_amendment(&v, &meta).is_ok(), "an unappliable proposal is worse than none");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Re-judging twice a day must not bury the queue. The second run finds the first proposal and
    /// points at it — matched on the FIELD SET, because approving the older one answers the same
    /// question.
    #[test]
    fn a_second_re_judgement_reuses_the_pending_proposal() {
        let v = propose_vault("dedup");
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "4.0".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        let ctx = WriteContext::new("agent:knowlu.enrich", "local-runner");
        let opts = WriteOpts { judged: true, propose: true, ..Default::default() };
        let first = write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "2.0".to_string())], &ctx, &mut journal, &opts).unwrap();
        let second = write_literals(&v, "tasks/t.md", &[("effort_hours".to_string(), "1.0".to_string())], &ctx, &mut journal, &opts).unwrap();
        assert_eq!(first.proposal, second.proposal, "one open decision, one card");
        assert!(second.skipped.get("effort_hours").unwrap().contains("already pending"), "{:?}", second.skipped);
        let n = std::fs::read_dir(v.join("approvals")).unwrap().flatten().count();
        assert_eq!(n, 1, "a second card would spend the day's approval budget on one decision");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The 2026-08-21 hardening rule: `validate_amendment` refuses a null `from`, so a proposal it
    /// could never apply is not minted at all — it is reported as a skip instead.
    #[test]
    fn a_field_quinn_set_to_nothing_is_skipped_rather_than_proposed_from_null() {
        let v = propose_vault("null");
        pystr::write_text(
            &v.join("tasks").join("t.md"),
            "---\ntitle: \"T\"\nstatus: active\nid: task_0123456789\n---\n\nBody.\n",
        ).unwrap();
        let mut journal = Journal::new(&v);
        write_literals(&v, "tasks/t.md", &[("importance_reason".to_string(), "null".to_string())],
            &WriteContext::new("quinn", "dashboard"), &mut journal, &WriteOpts::default()).unwrap();
        let res = write_literals(
            &v, "tasks/t.md",
            &[("importance_reason".to_string(), "\"Worth 15%.\"".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();
        assert!(res.proposal.is_none());
        assert!(res.skipped.get("importance_reason").unwrap().contains("cannot propose from null"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Several fields re-judged at once is ONE card naming all of them — one decision, one
    /// proposal, one charge against the day's budget.
    #[test]
    fn several_fields_re_judged_together_make_one_card() {
        let v = propose_vault("multi");
        let mut journal = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/t.md", &[
            ("effort_hours".to_string(), "4.0".to_string()),
            ("importance".to_string(), "5".to_string()),
        ], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let res = write_literals(
            &v, "tasks/t.md",
            &[("effort_hours".to_string(), "2.0".to_string()), ("importance".to_string(), "3".to_string())],
            &WriteContext::new("agent:knowlu.enrich", "local-runner"),
            &mut journal,
            &WriteOpts { judged: true, propose: true, ..Default::default() },
        ).unwrap();
        let path = res.proposal.expect("one card");
        let meta = crate::ids::read_meta(&path).unwrap();
        let Some(serde_yaml_ng::Value::Mapping(changes)) = crate::yaml::get(&meta, "changes") else {
            panic!("no changes block");
        };
        assert_eq!(changes.len(), 2);
        assert_eq!(crate::yaml::get(changes, "effort_hours").and_then(|c| crate::yaml::get(c.as_mapping().unwrap(), "from")).and_then(crate::yaml::f64_of), Some(4.0));
        assert_eq!(crate::yaml::get(changes, "importance").and_then(|c| crate::yaml::get(c.as_mapping().unwrap(), "to")).and_then(crate::yaml::i64_of), Some(3));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Python's `if evidence:` is truthiness, so an EMPTY mapping appends nothing. `!is_null()`
    /// alone would write `Evidence: {}` here and nothing there — a one-line byte difference on a
    /// note, which is the class of divergence the dual-run scripts exist to catch.
    #[test]
    fn empty_evidence_appends_nothing_and_a_real_one_appends_itself() {
        let v = propose_vault("evidence");
        let mut journal = Journal::new(&v);
        let quinn = WriteContext::new("quinn", "dashboard");
        write_literals(&v, "tasks/t.md", &[("importance".to_string(), "5".to_string())], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let ctx = WriteContext::new("agent:knowlu.enrich", "local-runner");

        let empty = serde_json::json!({});
        let res = write_literals(&v, "tasks/t.md", &[("importance".to_string(), "2".to_string())], &ctx, &mut journal,
            &WriteOpts { judged: true, propose: true, evidence: Some(&empty), inputs: None }).unwrap();
        let body = pystr::read_text(&res.proposal.clone().unwrap()).unwrap();
        assert!(!body.contains("Evidence:"), "an empty mapping is falsy in Python: {body}");

        // A different field set, so `find_pending_amendment` mints a second card rather than
        // pointing at the first.
        write_literals(&v, "tasks/t.md", &[("course".to_string(), "\"cs-100\"".to_string())], &quinn, &mut journal, &WriteOpts::default()).unwrap();
        let real = serde_json::json!({"why": "syllabus"});
        let res = write_literals(&v, "tasks/t.md", &[("course".to_string(), "\"gn-103\"".to_string())], &ctx, &mut journal,
            &WriteOpts { judged: true, propose: true, evidence: Some(&real), inputs: None }).unwrap();
        let body = pystr::read_text(&res.proposal.unwrap()).unwrap();
        assert!(body.contains("Evidence: {\"why\": \"syllabus\"}"), "{body}");
        let _ = std::fs::remove_dir_all(&v);
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run (repo root): `cargo test write::tests::a_judged_write_over write::tests::a_second_re_judgement write::tests::a_field_quinn_set write::tests::several_fields`
Expected: FAIL — the first panics with `judge-once re-proposal is not ported yet`.

- [ ] **Step 3: `AMEND_BUTTONS`**

Append to `src/write.rs`, near `find_pending_amendment`:

```rust
/// The two meta-bind buttons every approval note carries, byte-for-byte
/// `engine/write.py:AMEND_BUTTONS` (lines 287–307).
///
/// A literal, not something built from a template: it is Obsidian markup that has to be identical
/// to what Python writes for `scripts/diff-engines-notes.ps1` to compare the two engines' proposals
/// as bytes, and identical to every other approval in `approvals/` for the deck's own parser.
/// Leading newline included — Python concatenates it straight after the `**Why proposed:**` line.
pub const AMEND_BUTTONS: &str = "\n```meta-bind-button\nlabel: Approve\nstyle: primary\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: approved\n```\n\n```meta-bind-button\nlabel: Reject\nstyle: destructive\naction:\n  type: updateMetadata\n  bindTarget: status\n  evaluate: false\n  value: rejected\n```\n";
```

- [ ] **Step 4: `propose_amendment`**

Append below it:

```rust
/// Judge-once's other half: the agent may **re-propose, never silently overwrite** (Quinn,
/// 2026-08-29). Port of `engine/write.py:propose_amendment` (lines 333–353).
///
/// Called only from `write_literals`, and only when the journal shows Quinn set the field himself
/// and `propose` is on. It mints one `kind: amend` approval naming every re-judged field, through
/// `create` — so the proposal is journalled like any other note and gets its own `id`.
///
/// Four details that are load-bearing:
///
/// 1. **`from` is the note's current value and `to` is the agent's**, both put through
///    `yaml::to_json` first (Python's `jsonable`), because `approvals::validate_amendment` refuses a
///    collection on either side and compares `from` against the note before applying.
/// 2. **`expires: null`** — an amendment does not go stale on a date; it is answered or it is not.
/// 3. **`proposed_at` and `first_proposed_at` are both today.** The second is the S1 field
///    `age_days` and `oldest_pending_days` read, set once and never rewritten;
///    `approvals::defer_over_budget` may later move `proposed_at` and must not move that.
/// 4. **The stem carries six characters of a fresh id**, so two re-judgements of two different
///    fields on the same note never collide on a filename — and `find_pending_amendment` is what
///    stops the same field set minting a second card at all.
///
/// **The evidence branch, and its one documented divergence.** Python is `if evidence:` — plain
/// truthiness, so `None` **and an empty dict** both append nothing. `filter(|e| !e.is_null())` alone
/// would let `{}` through and write a bare `Evidence: {}` the other engine never writes, so the
/// filter tests for both. What is *not* reproducible is key ORDER in a multi-key mapping:
/// `json.dumps` keeps Python's insertion order, while `serde_json`'s `Map` is a `BTreeMap` (no
/// `preserve_order` feature) and `ledger::dumps_value` sorts as well, so `{"b": 1, "a": 2}` renders
/// as `{"b": 1, "a": 2}` there and `{"a": 2, "b": 1}` here. It is **not fixable in this task** —
/// `preserve_order` would change the shape of every journal record in the crate — and it is
/// unreachable from production, where `enrich` passes `None` and only a hand-typed
/// `write set --evidence` can supply one. `scripts/diff-engines-notes.ps1` therefore uses a
/// **one-key** evidence mapping on `w14`, exactly as `w2` already does, which compares the branch
/// byte for byte without tripping an ordering difference neither engine is wrong about.
pub fn propose_amendment(
    vault: &Path,
    target_path: &Path,
    meta: &Mapping,
    changes: &[(String, Value, Value)],
    ctx: &WriteContext,
    journal: &mut Journal,
    evidence: Option<&serde_json::Value>,
    today: jiff::civil::Date,
) -> Result<PathBuf, WriteError> {
    let target_rel = rel(vault, target_path);
    let stem_of_target = target_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut names: Vec<String> = changes.iter().map(|(n, _, _)| n.clone()).collect();
    names.sort();
    let fields = names.join(", ");
    let suffix = {
        let id = new_id("appr");
        id.chars().skip(id.chars().count().saturating_sub(6)).collect::<String>()
    };
    let stem = format!("amend-{stem_of_target}-{suffix}");

    let title = get_str(meta, "title").unwrap_or_else(|| stem_of_target.clone());
    let mut change_block: Vec<(crate::yamlemit::Node, crate::yamlemit::Node)> = Vec::new();
    for (name, old, new) in changes {
        change_block.push((
            crate::yamlemit::Node::text(name),
            crate::yamlemit::Node::map(vec![
                ("from", crate::yamlemit::Node::from_json(&yaml_to_json(old))),
                ("to", crate::yamlemit::Node::from_json(&yaml_to_json(new))),
            ]),
        ));
    }
    let front = crate::yamlemit::Node::map(vec![
        ("type", crate::yamlemit::Node::text("approval")),
        ("kind", crate::yamlemit::Node::text("amend")),
        ("title", crate::yamlemit::Node::text(&format!("Re-proposed {fields} for {title}"))),
        ("status", crate::yamlemit::Node::text("pending")),
        ("target", crate::yamlemit::Node::text(&target_rel)),
        ("proposed_at", crate::yamlemit::Node::Date(today)),
        ("first_proposed_at", crate::yamlemit::Node::Date(today)),
        ("expires", crate::yamlemit::Node::Null),
        ("snooze_until", crate::yamlemit::Node::Null),
        ("created_by", crate::yamlemit::Node::text(&ctx.actor)),
        ("changes", crate::yamlemit::Node::Map(change_block)),
    ]);
    let mut why = format!(
        "{} re-judged {fields}; Quinn had set them by hand, so this is a proposal (judge-once rule).",
        ctx.actor
    );
    // Python's `if evidence:` is TRUTHINESS, so an empty mapping appends nothing — not just `None`.
    // A `!e.is_null()` test alone would write a bare `Evidence: {}` the other engine never writes.
    let truthy = |e: &&serde_json::Value| match e {
        serde_json::Value::Null => false,
        serde_json::Value::Object(m) => !m.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
    };
    if let Some(ev) = evidence.filter(truthy) {
        why.push_str(" Evidence: ");
        why.push_str(&crate::ledger::dumps_value(ev));
    }
    let text = format!(
        "---\n{}---\n\n**Why proposed:** {why}\n{AMEND_BUTTONS}",
        crate::yamlemit::safe_dump_block(&front)
    );
    create(vault, &format!("approvals/{stem}.md"), &text, ctx, journal, evidence)
}
```

- [ ] **Step 5: Wire it into `write_literals` and delete the variant**

In `src/write.rs`, `write_literals` currently collects `proposed: Vec<String>`. It needs the old and new values too. Change the declaration:

```rust
    // `(name, old, new)` — `propose_amendment` writes both sides into the card's `changes` block,
    // and `validate_amendment` compares `from` against the note before it applies.
    let mut proposed: Vec<(String, Value, Value)> = Vec::new();
```

and the push inside the judge-once branch:

```rust
                if opts.propose && old != Value::Null {
                    proposed.push((name.clone(), old.clone(), new.clone()));
                } else if opts.propose {
```

and the tail block:

```rust
    if !proposed.is_empty() {
        // An identical pending proposal already IS the re-proposal; point at it and report the
        // skip, rather than minting a fresh `amend-*.md` on every run and burying the queue.
        let fields: std::collections::BTreeSet<String> =
            proposed.iter().map(|(n, _, _)| n.clone()).collect();
        match find_pending_amendment(vault, &rel_path, &fields) {
            Some(existing) => {
                let name = existing
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                for (field, _, _) in &proposed {
                    result.skipped.insert(
                        field.clone(),
                        format!("judge-once: proposal already pending ({name})"),
                    );
                }
                result.proposal = Some(existing);
            }
            None => {
                // `today` is the system date, as Python's `today or date.today()` resolves it for
                // every production caller. Nothing in the engine passes a pinned date to this path.
                let today = jiff::Zoned::now().date();
                result.proposal = Some(propose_amendment(
                    vault, &path, &meta, &proposed, ctx, journal, opts.evidence, today,
                )?);
            }
        }
    }
```

Then delete the `ProposeNotPorted` variant from `enum WriteError`, its `Display` arm and its doc comment. `cargo build` will name every remaining reference; there should be none outside `write.rs`.

- [ ] **Step 6: Run the tests**

Run (repo root): `cargo test write::`
Expected: PASS, 0 warnings.

- [ ] **Step 7: Add the `--propose` trio to the dual-run script — and the mask it needs**

Three edits, and the order does not matter, but **all three are required or step 8 cannot pass**.

**(a) The header.** Replace the last sentence of the `.DESCRIPTION` block —

```
    a `=`, a --via outside VIAS). tasks/ and approvals/ join the note comparison. The one `write`
    path NOT here is `--propose`: `write::propose_amendment` is unported by decision (S3c
    replaces it), so it would only ever report the difference everyone already knows about.
```

— with:

```
    a `=`, a --via outside VIAS). tasks/ and approvals/ join the note comparison.

    Knowlu plan 3a Task 6 added the `--propose` trio (w13-w15), which was excluded while
    `write::propose_amendment` was unported: a human set, then a judged agent set with --propose
    and --evidence that must mint a `kind: amend` card in approvals/, then the SAME agent set
    again, which must find the pending card and mint nothing.

    That trio needed a THIRD mask. `propose_amendment` names its card
    `amend-<target stem>-<6 hex>`, and the six hex characters are random on both sides. They reach
    two places this script compares and the id mask does not cover: the journal `create` record's
    `path`, and stdout (`proposed amend-...-<6hex>.md`, and on the second run the
    `judge-once: proposal already pending (...)` line). `NoteHashes` hashes file CONTENTS and never
    sees a filename, which is why the existing `(info|iss|task|appr)_[0-9a-f]{10}` mask does not
    reach it.
```

**(b) The mask.** In `function Mask`, after the id line:

```powershell
    # Knowlu plan 3a Task 6: `propose_amendment`'s card stem carries six random hex characters
    # (`new_id("appr")`'s last six), and they land in the journal record's `path` and in stdout.
    # Masked here rather than in `NoteHashes`, which only ever sees file contents.
    $t = $t -replace 'amend-([A-Za-z0-9_.-]+)-[0-9a-f]{6}', 'amend-$1-XXXXXX'
```

**(c) The commands.** `w5`, `w6` and `w7` are **already taken** — `w5` is a `set` and `w6`/`w7` are the `append-body` idempotence pair, and `$results` is a hashtable keyed by name, so reusing those names would silently overwrite three existing cases and drop their coverage without a word. Append **after `w12`**:

```powershell
# Knowlu plan 3a Task 6: the judge-once re-proposal path. `w13` is Quinn's own edit, so `w14`'s
# judged agent write finds a human-set field and files a card instead of overwriting; `w15` repeats
# `w14` and must find that card rather than mint a second one. Both card and both journals are
# compared by the note/journal passes above; the stem's six random characters are masked by `Mask`.
#
# `--evidence` is on `w14` deliberately: `propose_amendment` appends it to the card's
# `**Why proposed:**` line, and that branch would otherwise be reasoned about rather than compared.
# ONE key, like `w2`'s: `json.dumps` preserves insertion order while `serde_json`'s map is sorted on
# parse, so a two-key evidence mapping would surface an ordering difference this task cannot fix
# (it is `serde_json` without `preserve_order`, which every journal record in the crate depends on).
Run "w13" "write" @("--actor", "quinn", "--via", "dashboard", "set", "tasks/cs-100-hw-01.md", "effort_hours=4.0")
Run "w14" "write" @("--actor", "agent:routine.enrich", "--via", "cloud-routine", "set", "tasks/cs-100-hw-01.md", "effort_hours=2.0", "--judged", "--propose", "--evidence", '{"""why""": """syllabus"""}')
Run "w15" "write" @("--actor", "agent:routine.enrich", "--via", "cloud-routine", "set", "tasks/cs-100-hw-01.md", "effort_hours=1.0", "--judged", "--propose")
```

**(d) The success list.** The stdout comparison at the `foreach ($name in @("i1", …` line only checks the names it is given, so three new commands that nobody added would be run and never compared. Add them:

```powershell
foreach ($name in @("i1", "i2", "i3", "i4", "i5", "i6", "i7", "i8", "s1", "s2", "s3", "s4", "s5", "s7", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "w8", "w9", "w13", "w14", "w15")) {
```

- [ ] **Step 8: Run both dual-run scripts**

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: **both exit 0.** This is the acceptance test for the whole task: the Rust proposal note must be byte-identical to PyYAML's, the journal records must match, and the three new stdout comparisons must agree — on a real vault copy.

**If it fails, read the failure before changing anything.** Three things can break here and they need different fixes:

- **`approvals/ notes` DIFFERS** — the card's bytes. The script writes both sides' masked notes to `$work\masked-*`; diff those two `amend-*.md` files. The frontmatter comes from `yamlemit::safe_dump_block`, which the 188-case corpus and fourteen `info`/`issues` commands already cover, so the likely culprit is this task's own new code: the `changes` block's `Node::from_json(&yaml_to_json(…))` conversion, the `Date` on `proposed_at`/`first_proposed_at`, or the `**Why proposed:**` line's evidence rendering. **Fix the Rust, never the reference.**
- **`journal records` DIFFERS** with only the card's `path` differing — the mask from (b) is missing or its pattern does not match the stem your `new_id` produced.
- **`stdout w14`/`w15` DIFFERS** — same mask, reached through the printed `proposed …` and `already pending (…)` lines.

- [ ] **Step 9: Full gate and commit**

```bash
cargo test 2>&1 | tail -20
```

```bash
git add src/write.rs scripts/diff-engines-notes.ps1
git commit -F .git/COMMIT_MSG_3A6
```

```
feat(write): port propose_amendment — judge-once files an amend card again (plan 3a, Task 6)

engine/write.py:333-353, ported. A judged agent write over a field the journal shows Quinn set
now mints one `kind: amend` approval naming every re-judged field, with `from` the note's current
value and `to` the agent's, `expires: null`, `proposed_at` and `first_proposed_at` both today, and
the two meta-bind buttons every proposal carries. WriteError::ProposeNotPorted is deleted.

This is the path enrichment needs: without it the whole judge-once loop the spec and CLAUDE.md
describe ended in an error the moment the model disagreed with something Quinn had set by hand.
Deduped on the field SET through the existing find_pending_amendment, so re-judging twice a day
points at the open card rather than spending the day's approval budget on one decision.

scripts/diff-engines-notes.ps1 gains the --propose pair its header used to say was excluded by
decision, so both engines' cards are now compared as bytes on a real vault copy.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 7: `src/enrich.rs` and `quinn-ops judge` — enrichment, end to end

The pass. Select every task carrying `needs_enrichment: true`, judge it, write the judged fields through `write` as `agent:knowlu.enrich`, clear the flag only when the answer is complete, let judge-once file an amend card, log every judgment, and **always exit 0**.

**What this deliberately does not do.** The cloud routine's step 3 also *appends a `course_map` pin to `config/ingest.yaml`* when it attributes a gradebook item by hand, so the deterministic parser attributes it next time. This plan does not write that pin: learning a rule from a model's output and promoting it is exactly §5.4's loop, which is **plan 3b's** — and a config line written here would be one 3b's promotion loop then had to argue with (D10). Nothing is lost while both producers run: the routine keeps writing pins until it is turned off, and until then an unpinned item is simply re-attributed by the model each time it appears.

**Files:**
- Create: `src/enrich.rs` (with its own `mod tests`)
- Modify: `src/lib.rs` (one `pub mod`), `src/main.rs` (the `Judge` clap arm)
- Read first: `src/ingest.rs:681–748` (`run`/`run_with`/`run_lines` — the shape this copies, including the seam), `docs/runners/cloud-routine-prompt.md` line 13 (the exact `$W` command this reproduces), `src/write.rs`'s `WriteOpts`

**Interfaces:**
- Consumes: `judge::{Item, Heuristics, Outcome, Missing, Model, NoRules, judge_task, one_line}`; `runtime::{resolve, Server, LOAD_TIMEOUT, CALL_TIMEOUT}`; `judgelog::{entry_for, record}`; `write::{write_literals, to_literal, WriteContext, WriteOpts}`; `journal::Journal`; `approvals::sorted_md`; `models::split_frontmatter`; `ids::rel`; `yaml::{get, text, opt_text, opt_f64}`.
- Produces:
  - `enrich::ACTOR: &str` (`"agent:knowlu.enrich"`)
  - `enrich::Options<'a> { via, run_id, runtime, model, log_dir, limit }`
  - `enrich::pending(vault: &Path) -> (Vec<judge::Item>, Vec<String>)`
  - `enrich::literals_for(v: &judge::Verdict, complete: bool) -> Vec<(String, String)>`
  - `enrich::enrich_with(vault, opts, model: Result<&dyn judge::Model, judge::Missing>) -> (i32, Vec<String>)`
  - `enrich::{run_lines, run_with, run}`

- [ ] **Step 1: Write the failing tests** — create `src/enrich.rs` with only the module doc and this `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\ntitle: \"CS-100 Homework 3\"\ncourse: null\ndomain: school\ndue: 2026-10-01T23:59\neffort_hours: 1.0\neffort_confidence: low\neffort_source: inferred\nimportance: 3\nimportance_reason: \"pending enrichment\"\nstatus: active\nprogress: 0\ncreated_by: blackboard\nsource_uid: \"blackboard:_884411_1\"\nneeds_enrichment: true\nid: task_1111111111\n---\n\nChapters 4 and 5.\n";

    fn vault(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qo-enrich-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tasks")).unwrap();
        std::fs::create_dir_all(d.join("approvals")).unwrap();
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::create_dir_all(d.join("courses")).unwrap();
        crate::pystr::write_text(&d.join("tasks").join("hw3.md"), NOTE).unwrap();
        crate::pystr::write_text(
            &d.join("config").join("ingest.yaml"),
            "course_map:\n  CS-100: cs-100\n  _884411_1: cs-100\n",
        ).unwrap();
        crate::pystr::write_text(
            &d.join("courses").join("cs-100.md"),
            "---\ntitle: CS 100\n---\n\n## Grade weights\n- Homework: 20%\n",
        ).unwrap();
        d
    }

    struct Fixed(crate::judge::Verdict);
    impl crate::judge::Model for Fixed {
        fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
            -> Result<crate::judge::Verdict, crate::judge::ModelError> { Ok(self.0.clone()) }
    }
    fn answered() -> Fixed {
        Fixed(crate::judge::Verdict {
            course: Some("cs-100".to_string()),
            effort_hours: Some(2.5),
            importance: Some(4),
            importance_reason: Some("Homework is 20% of CS 100.".to_string()),
            confidence: 0.9,
            tier: 3,
        })
    }
    fn opts<'a>(log: &'a Path) -> Options<'a> {
        Options { via: "local-runner", run_id: Some("local-2026-09-07T18:00:00Z"), runtime: None, model: None, log_dir: Some(log), limit: 50 }
    }
    fn meta_of(v: &Path, name: &str) -> serde_yaml_ng::Mapping {
        crate::ids::read_meta(&v.join("tasks").join(name)).unwrap()
    }
    fn records(v: &Path) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(v.join("state").join("journal")).unwrap().flatten() {
            for line in crate::pystr::read_text(&e.path()).unwrap().split('\n').filter(|l| !l.trim().is_empty()) {
                out.push(serde_json::from_str(line).unwrap());
            }
        }
        out
    }

    /// Only flagged notes are selected, and nothing else in `tasks/` is even read for judgment —
    /// "never modify notes without the flag" is the routine's own rule and it is this pass's too.
    #[test]
    fn only_notes_carrying_the_flag_are_selected() {
        let v = vault("select");
        crate::pystr::write_text(&v.join("tasks").join("done.md"),
            &NOTE.replace("needs_enrichment: true", "needs_enrichment: false").replace("task_1111111111", "task_2222222222")).unwrap();
        crate::pystr::write_text(&v.join("tasks").join("never.md"),
            &NOTE.replace("needs_enrichment: true\n", "").replace("task_1111111111", "task_3333333333")).unwrap();
        let (items, skipped) = pending(&v);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "task_1111111111");
        assert_eq!(items[0].title, "CS-100 Homework 3");
        assert_eq!(items[0].body.trim(), "Chapters 4 and 5.");
        assert_eq!(items[0].source_uid, "blackboard:_884411_1");
        assert_eq!(items[0].due.as_deref(), Some("2026-10-01T23:59"));
        assert!(skipped.is_empty());
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Without an `id:` judge-once cannot answer "did Quinn set this?", so the protection is void
    /// and the note is left alone and named.
    #[test]
    fn a_note_with_no_id_is_skipped_and_said_so() {
        let v = vault("noid");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &NOTE.replace("id: task_1111111111\n", "")).unwrap();
        let (items, skipped) = pending(&v);
        assert!(items.is_empty());
        assert_eq!(skipped, vec!["hw3.md: no id, so judge-once cannot protect it".to_string()]);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The whole pass: five judged fields plus the flag, an agent actor, the run's via and run_id,
    /// and a `judgment:` block naming exactly the judged fields it wrote.
    #[test]
    fn enrichment_writes_the_five_fields_clears_the_flag_and_records_a_judgment() {
        let v = vault("happy");
        let log = v.join("_log");
        let m = answered();
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        assert!(lines.last().unwrap().starts_with("judge: 1 item"), "{lines:?}");

        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "course")).as_deref(), Some("cs-100"));
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 2.5);
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "effort_confidence")).as_deref(), Some("low"));
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 4);
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "importance_reason")).as_deref(), Some("Homework is 20% of CS 100."));
        assert_eq!(crate::yaml::get(&meta, "needs_enrichment"), Some(&serde_yaml_ng::Value::Bool(false)));

        // The judgment block: single-line flow mapping, agent actor, the run, and the inputs.
        let text = crate::pystr::read_text(&v.join("tasks").join("hw3.md")).unwrap();
        assert!(crate::provenance::guard_block_style(&text).is_ok(), "must stay a single-line flow mapping");
        let Some(serde_yaml_ng::Value::Mapping(j)) = crate::yaml::get(&meta, "judgment") else { panic!("no judgment") };
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(j, "actor")).as_deref(), Some("agent:knowlu.enrich"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(j, "run_id")).as_deref(), Some("local-2026-09-07T18:00:00Z"));
        let Some(serde_yaml_ng::Value::Sequence(fields)) = crate::yaml::get(j, "fields") else { panic!("no fields") };
        let names: Vec<String> = fields.iter().filter_map(crate::yaml::text).collect();
        // D12: only fields provenance protects appear here, and `needs_enrichment` is not one.
        assert_eq!(names, vec!["course", "effort_confidence", "effort_hours", "importance", "importance_reason"]);
        for n in &names {
            assert!(crate::provenance::JUDGED_FIELDS_TASK.contains(&n.as_str()), "{n} is not a judged field");
        }
        let Some(serde_yaml_ng::Value::Mapping(inputs)) = crate::yaml::get(j, "inputs") else { panic!("no inputs") };
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(inputs, "source_uid")).as_deref(), Some("blackboard:_884411_1"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(inputs, "title_seen")).as_deref(), Some("CS-100 Homework 3"));

        for r in records(&v) {
            assert_eq!(r["actor"], "agent:knowlu.enrich");
            assert_eq!(r["via"], "local-runner");
            assert_eq!(r["run_id"], "local-2026-09-07T18:00:00Z");
        }
        let logged = crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10]);
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0]["outcome"], "answered");
        assert_eq!(logged[0]["tier"], 3);
        let _ = std::fs::remove_dir_all(&v);
    }

    /// D7: no runtime and no model are normal. Whatever tier 1 answered is still written — a course
    /// attributed from the uid pin is worth having — and the flag STAYS SET so the routine, or a
    /// later run with a model, finishes the job.
    #[test]
    fn with_no_model_tier_one_still_writes_and_the_flag_stays_set() {
        let v = vault("nomodel");
        let log = v.join("_log");
        let (code, lines) = enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        assert_eq!(code, 0, "a missing model is never a failed slot");
        assert!(lines.iter().any(|l| l.contains("model not installed")), "{lines:?}");
        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&meta, "course")).as_deref(), Some("cs-100"), "the uid pin answered");
        assert_eq!(crate::yaml::get(&meta, "needs_enrichment"), Some(&serde_yaml_ng::Value::Bool(true)), "still owed");
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 1.0, "the template's value is untouched");
        assert_eq!(crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10])[0]["outcome"], "model not installed");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// Running twice must be a no-op the second time: `write`'s F4 rule means an identical value
    /// produces no record and no write, so nothing is re-journalled and no second judgment block
    /// appears.
    #[test]
    fn a_second_run_over_the_same_vault_writes_nothing() {
        let v = vault("idem");
        let log = v.join("_log");
        enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        let before = records(&v).len();
        let before_bytes = std::fs::read(v.join("tasks").join("hw3.md")).unwrap();
        let (code, _) = enrich_with(&v, &opts(&log), Err(crate::judge::Missing::Model));
        assert_eq!(code, 0);
        assert_eq!(records(&v).len(), before, "a re-run journalled something");
        assert_eq!(std::fs::read(v.join("tasks").join("hw3.md")).unwrap(), before_bytes, "the note changed on a re-run");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// D3: a field Quinn set comes back as a card, and the note is untouched — the loop the routine
    /// has followed for weeks, now run locally.
    #[test]
    fn a_field_quinn_set_becomes_an_amend_card_and_the_note_is_untouched() {
        let v = vault("amend");
        let log = v.join("_log");
        let mut journal = crate::journal::Journal::new(&v);
        crate::write::write_literals(&v, "tasks/hw3.md", &[("effort_hours".to_string(), "6.0".to_string())],
            &crate::write::WriteContext::new("quinn", "dashboard"), &mut journal, &crate::write::WriteOpts::default()).unwrap();
        let m = answered();
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0);
        // S2: the summary must COUNT the card and the item line must NAME it. `contains("proposed")`
        // alone is satisfied by the summary's own `0 proposed`, so it asserted nothing.
        assert!(lines.iter().any(|l| l.contains("amend-")), "the card is named: {lines:?}");
        assert!(lines.last().unwrap().contains("1 proposed"), "{lines:?}");
        let meta = meta_of(&v, "hw3.md");
        assert_eq!(crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 0.0), 6.0, "judge-once held");
        assert_eq!(crate::yaml::opt_i64(crate::yaml::get(&meta, "importance"), 0), 4, "the fields Quinn did not set are still written");
        let cards: Vec<_> = std::fs::read_dir(v.join("approvals")).unwrap().flatten().collect();
        assert_eq!(cards.len(), 1, "one card");
        let card = crate::ids::read_meta(&cards[0].path()).unwrap();
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "kind")).as_deref(), Some("amend"));
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&card, "created_by")).as_deref(), Some("agent:knowlu.enrich"));
        let _ = std::fs::remove_dir_all(&v);
    }

    /// A model that fails on one item must not stop the batch: the other items are still judged and
    /// the failure is one line and one log record.
    #[test]
    fn one_bad_item_does_not_stop_the_batch() {
        let v = vault("batch");
        let log = v.join("_log");
        crate::pystr::write_text(&v.join("tasks").join("hw4.md"),
            &NOTE.replace("task_1111111111", "task_4444444444").replace("Homework 3", "Homework 4")).unwrap();
        struct Flaky(std::sync::Mutex<usize>);
        impl crate::judge::Model for Flaky {
            fn judge(&self, _i: &crate::judge::Item, _h: &crate::judge::Heuristics, _s: &crate::judge::Verdict)
                -> Result<crate::judge::Verdict, crate::judge::ModelError> {
                let mut n = self.0.lock().unwrap();
                *n += 1;
                if *n == 1 { return Err(crate::judge::ModelError::Failed("llama-server: broken pipe".into())); }
                Ok(crate::judge::Verdict { course: None, effort_hours: Some(1.5), importance: Some(3),
                    importance_reason: Some("No weights given, so 3.".into()), confidence: 0.8, tier: 3 })
            }
        }
        let m = Flaky(std::sync::Mutex::new(0));
        let (code, lines) = enrich_with(&v, &opts(&log), Ok(&m));
        assert_eq!(code, 0, "a model failure is never a failed slot");
        assert!(lines.iter().any(|l| l.contains("broken pipe")), "the failure is visible: {lines:?}");
        assert_eq!(crate::judgelog::read_day(&log, &crate::journal::now_ts(None)[..10]).len(), 2, "both judgments logged");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// `limit` bounds one run. A vault with two hundred flagged notes on the first launch after a
    /// semester's ingest must not hold a slot for an hour; the rest are taken at the next slot.
    #[test]
    fn the_batch_is_bounded_and_says_how_many_are_left() {
        let v = vault("limit");
        let log = v.join("_log");
        for i in 0..4 {
            crate::pystr::write_text(&v.join("tasks").join(format!("more{i}.md")),
                &NOTE.replace("task_1111111111", &format!("task_555555555{i}"))).unwrap();
        }
        let mut o = opts(&log);
        o.limit = 2;
        let (code, lines) = enrich_with(&v, &o, Err(crate::judge::Missing::Runtime));
        assert_eq!(code, 0);
        let summary = lines.last().unwrap();
        assert!(summary.contains("2 item"), "{summary}");
        assert!(summary.contains("3 left"), "{summary}");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// S8: a whole-number estimate is written `2.0`, never `2`. Rust's `Display` for `2.0_f64` is
    /// `2`, and writing that would retype the field from float to int on a live vault — every other
    /// producer writes a float, and two spellings of one estimate is exactly the silent drift the
    /// single write path exists to stop.
    #[test]
    fn a_whole_number_effort_is_written_as_a_float() {
        let v = crate::judge::Verdict {
            effort_hours: Some(2.0),
            importance: Some(3),
            importance_reason: Some("No weights given, so 3.".to_string()),
            ..Default::default()
        };
        let lits = literals_for(&v, true);
        let find = |n: &str| lits.iter().find(|(k, _)| k == n).map(|(_, l)| l.clone());
        assert_eq!(find("effort_hours").as_deref(), Some("2.0"), "Display would have written 2");
        assert_eq!(find("importance").as_deref(), Some("3"), "importance IS an int");
        assert_eq!(find("effort_confidence").as_deref(), Some("\"low\""));
        assert_eq!(find("needs_enrichment").as_deref(), Some("false"));
        assert_eq!(find("course"), None, "an unresolved course writes no literal at all");
        assert_eq!(literals_for(&v, false).iter().find(|(k, _)| k == "needs_enrichment"), None);
    }

    /// A vault with nothing to do says so in one line and touches nothing.
    #[test]
    fn an_empty_queue_is_one_line() {
        let v = vault("empty");
        crate::pystr::write_text(&v.join("tasks").join("hw3.md"), &NOTE.replace("needs_enrichment: true", "needs_enrichment: false")).unwrap();
        let (code, lines) = enrich_with(&v, &opts(&v.join("_log")), Err(crate::judge::Missing::Runtime));
        assert_eq!(code, 0);
        assert_eq!(lines, vec!["judge: nothing to enrich".to_string()]);
        assert!(!v.join("state").join("journal").exists(), "nothing was journalled");
        let _ = std::fs::remove_dir_all(&v);
    }

    /// The CLI wrapper resolves nothing and starts nothing when neither path is given and no env
    /// var is set: it reports and exits 0.
    #[test]
    fn run_lines_with_no_runtime_anywhere_reports_and_exits_zero() {
        let v = vault("cli");
        // Explicit paths that cannot exist, rather than clearing the environment: `remove_var`
        // mutates process-global state every other test in this binary shares, and they run in
        // parallel. `resolve` prefers the ARGUMENT, so this takes the same branch on a machine that
        // does have KNOWLU_LLAMA_SERVER set — which is the machine the smoke test runs on.
        let gone = v.join("does-not-exist");
        let o = Options { via: "cli", run_id: None, runtime: Some(&gone), model: Some(&gone), log_dir: None, limit: 50 };
        let (code, lines) = run_lines(&v, &o);
        assert_eq!(code, 0);
        assert!(lines.iter().any(|l| l.contains("runtime not installed")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&v);
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run (repo root): `cargo test enrich::`
Expected: FAIL to compile — `cannot find function pending in this scope`.

- [ ] **Step 3: The module**

```rust
//! Enrichment (Knowlu spec §5.5 step 1, §5.1): the cloud routine's step 3, run locally at the app's
//! own slots, so the routine wakes up and finds nothing left to do.
//!
//! **Starve, never switch.** Nothing here turns the routine off or edits it. Both producers running
//! at once is the expected state for weeks: whichever runs second finds `needs_enrichment: false`
//! and writes nothing, because `write`'s F4 rule makes an identical value a no-op with no record.
//!
//! **This command always exits 0.** A missing runtime, a missing model, a model that will not start
//! and a model that answers nonsense are all normal outcomes reported on stdout. A non-zero exit
//! would set `RunSummary.engine_ok = false` in the app's scheduler, which paints the tray amber and
//! puts the slot into retry backoff twice a day forever — for a machine that has simply not
//! downloaded a 2 GB file. `coursework` exits 0 for the same reason; `ingest` does not, and that
//! difference is deliberate on both sides.
//!
//! **What it writes, and as whom.** Exactly the fields the routine's step 3 writes — `course`,
//! `effort_hours`, `effort_confidence: low`, `importance`, `importance_reason`, and
//! `needs_enrichment: false` — through `write` with `judged: true`, `propose: true` and
//! `inputs: {source_uid, title_seen}`, as **`agent:knowlu.enrich`**. Five of the six are in
//! `provenance::JUDGED_FIELDS_TASK`, so judge-once protects them: a field Quinn set in the console
//! is never overwritten and comes back as a `kind: amend` card instead.
//!
//! **What it deliberately does not write.** The routine also appends a `course_map` pin to
//! `config/ingest.yaml` when it attributes a gradebook item. Learning a rule from a model's output
//! and promoting it is spec §5.4's loop, which is plan **3b's**; a config line written here would be
//! one that loop then had to argue with. Nothing is lost meanwhile — the routine keeps writing pins
//! until it is turned off, and an unpinned item is simply re-attributed each time.

use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use crate::journal::Journal;
use crate::judge::{self, Heuristics, Item, NoRules, Outcome};
use crate::write::{self, WriteContext, WriteOpts};

/// The agent actor for plan 3a's writes. **`agent:` prefix, not `knowlu/`**: `provenance::is_agent`
/// is `starts_with("agent:")` and nothing else, so the spec §5.2 spelling would silently skip
/// judge-once and write no provenance block. 3b uses `agent:knowlu.events`, 3c `agent:knowlu.gmail`.
pub const ACTOR: &str = "agent:knowlu.enrich";

/// How many items one run judges by default. A semester's Blackboard ingest can flag a hundred at
/// once and each is a model call of a second or two; the rest are taken at the next slot, and the
/// summary line says how many are left, so nothing is lost and no slot is held for an hour.
pub const DEFAULT_LIMIT: usize = 50;

pub struct Options<'a> {
    pub via: &'a str,
    pub run_id: Option<&'a str>,
    /// Where the app installed the runtime. `None` falls back to `KNOWLU_LLAMA_SERVER` and then to
    /// a sibling of the exe (`runtime::resolve`).
    pub runtime: Option<&'a Path>,
    pub model: Option<&'a Path>,
    /// The profile's `judgments\` directory. `None` writes no log — which is what a hand-typed
    /// `quinn-ops judge` does, since only the app knows where a profile's app data is (§5.4).
    pub log_dir: Option<&'a Path>,
    pub limit: usize,
}

/// Every task note flagged `needs_enrichment: true`, plus one line per note that had to be skipped.
///
/// Path order (`approvals::sorted_md`), so a bounded run takes the same items in the same order
/// twice. **A note with no `id:` is skipped and named**: judge-once is `journal::human_set(id,
/// field)`, so without an id the protection is void and writing anyway would be exactly the silent
/// overwrite the whole rule exists to prevent.
///
/// Read with `std::fs::read_to_string`, not `pystr::read_text` — the same choice
/// `models::Task::from_file_with_meta` makes, since Python's task loader never ran notes through
/// universal-newline translation.
pub fn pending(vault: &Path) -> (Vec<Item>, Vec<String>) {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("tasks")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok((meta, body)) = crate::models::split_frontmatter(&text) else { continue };
        if !matches!(crate::yaml::get(&meta, "needs_enrichment"), Some(Value::Bool(true))) {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let id = crate::yaml::opt_text(crate::yaml::get(&meta, "id")).filter(|s| crate::ids::is_id(s));
        let Some(id) = id else {
            skipped.push(format!("{name}: no id, so judge-once cannot protect it"));
            continue;
        };
        let title = crate::yaml::opt_text(crate::yaml::get(&meta, "title")).unwrap_or_default();
        if title.trim().is_empty() {
            skipped.push(format!("{name}: no title, so there is nothing to judge from"));
            continue;
        }
        items.push(Item {
            id,
            rel_path: crate::ids::rel(vault, &path),
            title,
            body: crate::pystr::strip(&body).to_string(),
            source_uid: crate::yaml::opt_text(crate::yaml::get(&meta, "source_uid")).unwrap_or_default(),
            created_by: crate::yaml::opt_text(crate::yaml::get(&meta, "created_by")).unwrap_or_default(),
            course: crate::yaml::opt_text(crate::yaml::get(&meta, "course")),
            due: crate::yaml::get(&meta, "due").filter(|v| !matches!(v, Value::Null)).map(crate::approvals::plain),
            effort_hours: crate::yaml::opt_f64(crate::yaml::get(&meta, "effort_hours"), 1.0),
            effort_source: crate::yaml::opt_text(crate::yaml::get(&meta, "effort_source")).unwrap_or_default(),
        });
    }
    (items, skipped)
}

/// The frontmatter literals one verdict becomes.
///
/// Free text goes through `write::to_literal` so a colon or a quote in `importance_reason` survives
/// as YAML; the numbers are their own spelling. **`needs_enrichment=false` only when the verdict is
/// complete** — a partial answer leaves the flag set, so the routine (while it still runs) or a
/// later slot with a model finishes the job.
///
/// `effort_confidence` is always `low`, exactly as the routine writes it: this is an estimate from a
/// title and a description, and saying otherwise on the note would misrepresent it to `rank`.
pub fn literals_for(v: &judge::Verdict, complete: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(c) = &v.course {
        out.push(("course".to_string(), write::to_literal(&Value::String(c.clone()))));
    }
    if let Some(e) = v.effort_hours {
        // Through `to_literal` with a YAML `Number`, **never `format!("{e}")`**: Rust's `Display`
        // for `2.0_f64` is `2`, which retypes the field from float to int on a live vault — every
        // other producer (`ingest`'s template, `coursework`, the routine) writes `2.0`, and two
        // spellings of one estimate is exactly the silent drift the single write path exists to
        // stop. `serde_yaml_ng`'s `Number` renders a float through ryu, which always emits the
        // decimal point.
        out.push((
            "effort_hours".to_string(),
            write::to_literal(&Value::Number(serde_yaml_ng::Number::from(e))),
        ));
        out.push(("effort_confidence".to_string(), write::to_literal(&Value::String("low".into()))));
    }
    if let Some(i) = v.importance {
        out.push(("importance".to_string(), i.to_string()));
    }
    if let Some(r) = &v.importance_reason {
        out.push(("importance_reason".to_string(), write::to_literal(&Value::String(r.clone()))));
    }
    if complete {
        out.push(("needs_enrichment".to_string(), "false".to_string()));
    }
    out
}

/// The testable core: the model — or the reason there is none — is handed in, and **no process is
/// started here**. `run_lines` is this with `runtime::resolve` and `Server::start` in front.
pub fn enrich_with(
    vault: &Path,
    opts: &Options<'_>,
    model: Result<&dyn judge::Model, judge::Missing>,
) -> (i32, Vec<String>) {
    let (all, skipped) = pending(vault);
    let mut lines: Vec<String> = skipped.into_iter().map(|s| format!("skipped {s}")).collect();
    if all.is_empty() {
        lines.push("judge: nothing to enrich".to_string());
        return (0, lines);
    }
    let left = all.len().saturating_sub(opts.limit);
    let batch: Vec<Item> = all.into_iter().take(opts.limit).collect();

    let heuristics = Heuristics::load(vault);
    let ctx = WriteContext {
        actor: ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let mut answered = 0usize;
    let mut proposed = 0usize;

    for item in &batch {
        let started = std::time::Instant::now();
        let outcome = judge::judge_task(item, &heuristics, &NoRules, model);
        let complete = matches!(outcome, Outcome::Answered(_));
        let literals = literals_for(outcome.verdict(), complete);
        let ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;

        // Ids and field values only (§5.6) — and logged for EVERY outcome, because a week of
        // "model not installed" is exactly what answers "why is nothing being enriched?".
        if let Some(dir) = opts.log_dir {
            let entry = crate::judgelog::entry_for(&item.id, &outcome, &literals, opts.run_id, ms);
            if let Err(e) = crate::judgelog::record(dir, &entry, None) {
                lines.push(format!("judge: {e}"));
            }
        }

        if literals.is_empty() {
            lines.push(format!("{} nothing to write ({})", item.rel_path, outcome.label()));
            continue;
        }
        let mut inputs = serde_yaml_ng::Mapping::new();
        inputs.insert("source_uid".into(), Value::String(item.source_uid.clone()));
        inputs.insert("title_seen".into(), Value::String(item.title.clone()));
        let write_opts = WriteOpts { judged: true, evidence: None, propose: true, inputs: Some(&inputs) };
        match write::write_literals(vault, &item.rel_path, &literals, &ctx, &mut journal, &write_opts) {
            Ok(res) => {
                if complete {
                    answered += 1;
                }
                let wrote: Vec<&str> = res.written.iter().map(|(n, _)| n.as_str()).filter(|n| *n != "judgment").collect();
                let mut line = format!("{} {} [{}]", item.rel_path, outcome.label(), wrote.join(" "));
                // `res.proposal` is the CARD — the whole point of `propose: true` — and it is the
                // thing that has to be counted and named. `res.skipped` is the explanation beside
                // it, and it is also set for the "already pending" case, where nothing new was
                // minted; counting `skipped` alone reported "0 proposed" on the very run that filed
                // one, and named no card for the reader to go and look at (review S2).
                if let Some(card) = &res.proposal {
                    proposed += 1;
                    let name = card.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    line.push_str(&format!(" — proposed instead ({name})"));
                }
                if !res.skipped.is_empty() {
                    let mut why: Vec<String> = res.skipped.iter().map(|(n, w)| format!("{n}: {w}")).collect();
                    why.sort();
                    line.push_str(&format!(" [{}]", why.join("; ")));
                }
                if let Outcome::LowConfidence { why, .. } = &outcome {
                    line.push_str(&format!(" — {why}"));
                }
                lines.push(line);
            }
            // A single unwritable note must not end the batch: the rest are still worth judging.
            Err(e) => lines.push(format!("{} not written: {e}", item.rel_path)),
        }
    }

    let mut summary = format!("judge: {} item(s), {answered} enriched, {proposed} proposed", batch.len());
    if left > 0 {
        summary.push_str(&format!(", {left} left for the next slot"));
    }
    lines.push(summary);
    (0, lines)
}

/// Resolve the runtime and the model, start a server if both are there, and enrich.
///
/// The `Server` is owned for the length of the batch and dropped at the end of it, which is what
/// stops the process: a ~2 GB model loads once per slot and not once per item, and nothing survives
/// the step. A runtime that will not start is one line and exit 0 — never a failed slot.
pub fn run_lines(vault: &Path, opts: &Options<'_>) -> (i32, Vec<String>) {
    match crate::runtime::resolve(opts.runtime, opts.model) {
        Err(missing) => enrich_with(vault, opts, Err(missing)),
        Ok((rt, gguf)) => {
            match crate::runtime::Server::start(&rt, &gguf, crate::runtime::LOAD_TIMEOUT, crate::runtime::CALL_TIMEOUT) {
                Ok(server) => enrich_with(vault, opts, Ok(&server)),
                Err(e) => (0, vec![format!("judge: the runtime did not start ({e})")]),
            }
        }
    }
}

/// [`run_lines`] with the output printed, in the order it was produced.
pub fn run_with(vault: &Path, opts: &Options<'_>) -> i32 {
    let (code, lines) = run_lines(vault, opts);
    for line in &lines {
        println!("{line}");
    }
    code
}

/// What `main` calls.
pub fn run(
    vault: &Path,
    via: &str,
    run_id: Option<&str>,
    runtime: Option<&PathBuf>,
    model: Option<&PathBuf>,
    log_dir: Option<&PathBuf>,
    limit: usize,
) -> i32 {
    run_with(
        vault,
        &Options {
            via,
            run_id,
            runtime: runtime.map(PathBuf::as_path),
            model: model.map(PathBuf::as_path),
            log_dir: log_dir.map(PathBuf::as_path),
            limit,
        },
    )
}
```

**One borrow to watch:** `write_opts` holds `Some(&inputs)`, and `inputs` is rebuilt inside the loop, so both must be declared inside it — as written above. Hoisting `inputs` out of the loop to save an allocation would make the second item's judgment block record the first item's `source_uid`, which is a lie recorded permanently in the note and the journal.

- [ ] **Step 4: Declare the module**

In `src/lib.rs`, after `pub mod judgelog;`:

```rust
// Knowlu plan 3a — the enrichment pass (spec §5.5 step 1): select, judge, write as
// `agent:knowlu.enrich`, log. The cloud routine's step 3, run locally, so that step starves.
pub mod enrich;
```

- [ ] **Step 5: The clap arm**

In `src/main.rs`'s `enum Command`, after `Ingest { … }`:

```rust
    /// Enrich tasks flagged `needs_enrichment: true` using the local model, if one is installed.
    ///
    /// Always exits 0: no runtime and no model are normal outcomes (Knowlu spec §5.3), and a
    /// non-zero exit here would put the app's scheduler into retry backoff and paint the tray
    /// amber for a machine that has simply not downloaded a model.
    Judge {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Without these the run's writes journal as `via: cli, run_id: null` --
        /// indistinguishable from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
        /// The llama.cpp server binary. Without it: KNOWLU_LLAMA_SERVER, else a sibling of this exe.
        #[arg(long)]
        runtime: Option<PathBuf>,
        /// The .gguf model file. Without it: KNOWLU_MODEL, else "model not installed".
        #[arg(long)]
        model: Option<PathBuf>,
        /// Where judgments are logged. **Never inside the vault** (spec §5.6) -- the app passes its
        /// profile's app-data folder; a hand-typed run without it logs nothing.
        #[arg(long = "log-dir")]
        log_dir: Option<PathBuf>,
        /// How many items one run judges. The rest wait for the next slot.
        #[arg(long, default_value_t = quinn_ops::enrich::DEFAULT_LIMIT)]
        limit: usize,
    },
```

and in `fn main`'s match, after the `Command::Ingest` arm:

```rust
        Command::Judge { vault, via, run_id, runtime, model, log_dir, limit } => {
            // Always SUCCESS: `enrich::run` only ever returns 0, and this arm says so out loud
            // rather than mapping a code that cannot occur.
            let _ = quinn_ops::enrich::run(
                &vault, &via, run_id.as_deref(), runtime.as_ref(), model.as_ref(), log_dir.as_ref(), limit,
            );
            ExitCode::SUCCESS
        }
```

Add `enrich` to the `use quinn_ops::{cli, coursework, ingest, journal, runs};` line so it reads `use quinn_ops::{cli, coursework, enrich, ingest, journal, runs};`, and use the short `enrich::run(...)`/`enrich::DEFAULT_LIMIT` spellings consistently.

- [ ] **Step 6: A clap test, beside the two already in `main.rs`**

Append to `src/main.rs`'s `mod tests`:

```rust
    /// The judge step is a runner's write like coursework's and ingest's: `--via` is validated by
    /// clap against `VIAS`, and the vocabulary does not grow for it.
    #[test]
    fn judge_takes_the_runners_via_and_refuses_anything_outside_vias() {
        assert!(Cli::try_parse_from(["quinn-ops", "judge", "--vault", ".", "--via", "local-runner"]).is_ok());
        let err = Cli::try_parse_from(["quinn-ops", "judge", "--via", "knowlu"]).err().expect("clap must refuse");
        assert_eq!(err.exit_code(), 2);
        assert!(Cli::try_parse_from(["quinn-ops", "judge", "--runtime", "a.exe", "--model", "b.gguf", "--log-dir", "c", "--limit", "10"]).is_ok());
    }
```

- [ ] **Step 7: Run the tests**

Run (repo root): `cargo test enrich:: write:: judge::`
Expected: PASS, 0 warnings.

- [ ] **Step 8: Full gate, both dual runs, and commit**

```bash
cargo test 2>&1 | tail -20
```

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: green at 0 warnings; both scripts exit 0. Then a real look at the command against a scratch copy — never the live vault:

```powershell
.\scripts\scratch-vault.ps1
cargo run -- judge --vault <the path that printed> --via cli
```

Expected: it prints `judge: nothing to enrich` or a `runtime not installed` line per flagged note, and **exits 0** (`$LASTEXITCODE` is 0).

```bash
git add src/enrich.rs src/lib.rs src/main.rs
git commit -F .git/COMMIT_MSG_3A7
```

```
feat(enrich): quinn-ops judge — the routine's step 3, run locally (plan 3a, Task 7)

Selects every task flagged needs_enrichment: true, judges it through the three tiers, and writes
course / effort_hours / effort_confidence: low / importance / importance_reason as
agent:knowlu.enrich with judged: true, propose: true and inputs {source_uid, title_seen} — the
routine's own step-3 command, in Rust. needs_enrichment is cleared only when the answer is
complete, so a partial answer (a course from the uid pin, with no model on the machine) is still
written and the item stays owed.

Always exits 0. No runtime, no model, a runtime that will not start and a model that answers
nonsense are all normal outcomes on stdout: a non-zero exit would paint the tray amber and put the
slot into retry backoff for a machine that has simply not downloaded a 2 GB file.

Judge-once is honoured through the path Task 6 ported: a field Quinn set comes back as a
kind: amend card and the note is untouched. The batch is bounded and says how many are left.
The course_map pin the routine also writes is deliberately not written here — that is spec §5.4's
promotion loop, which is plan 3b's.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 8: `app/src/inference.rs` — where the runtime and the model are, and how a verified one gets there

Task 9's `judge_state` calls into it, so it lands first.

Three rules, each a function a test can hold: **never automatic** (nothing here is called by a tick, a thread or a slot), **no unverified path to executing a runtime binary** (ruling R-P3a-2: a compiled-in table of supported upstream release digests, checked on *both* install paths), and **temp-then-rename** (a download that dies half way never leaves a partial where a runtime is expected) — the same shape the updater already keeps.

**Why a table and not a manifest digest** (R-P3a-2). The path that works today is *Install from a file…* — `https://knowlu.com/inference/manifest.json` does not exist until Quinn's Cloudflare account does — and a file the user points at carries no digest with it. A design where that path passes `None` would ship the one binary this product did not build with **nothing** verifying it, while the ledger and the module doc claimed otherwise. So the digests live in the app: **release tag, asset name, SHA-256**, compiled in, seeded from what Task 1 actually measured. A manifest is then a convenience for finding the bytes, not the root of trust, and an unrecognised digest is refused with the computed digest printed so it can be reported and added deliberately.

**The model is not in the table, and that asymmetry is deliberate.** A `.gguf` is *data* — it is parsed by a runtime this table has already vouched for, never executed — and §5.3 explicitly allows any 1–4 B Q4 model, so pinning model digests would make the feature useless for anyone who wants a different one. Its digest is checked when the manifest supplied one, and a file the user chose is accepted on their own authority.

**Files:**
- Create: `app/src/inference.rs`, `app/tests/inference.rs`
- Modify: `app/src/lib.rs` (one `pub mod`), `app/Cargo.toml` (three dependencies, all already in `app/Cargo.lock`)
- Read first: `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md` (**the release tag, asset name and SHA-256 that seed `SUPPORTED_RUNTIMES`**), `app/src/updates.rs` (`stage_bytes`, `already_staged`, `record_check` — the pattern this mirrors), `app/src/profiles.rs:19–23` (`profile_dir`, `updates_dir`)

**Interfaces:**
- Consumes: `quinn_ops::runtime::RUNTIME_EXE`; `quinn_ops::ledger::dumps_value`; `sha2::{Digest, Sha256}`; `zip::ZipArchive`; `ureq`.
- Produces:
  - `inference::{runtime_dir, models_dir, judgments_dir, runtime_exe, model_file}`
  - `inference::Status { runtime: Option<PathBuf>, model: Option<PathBuf>, model_bytes: u64 }` and `inference::status(root: &Path) -> Status`
  - `inference::sha256_of(path: &Path) -> Result<String, String>`
  - `inference::SupportedRuntime { tag, asset, sha256 }`, `inference::SUPPORTED_RUNTIMES: &[SupportedRuntime]`, `inference::runtime_release_for(sha256: &str) -> Option<&'static SupportedRuntime>`, `inference::check_runtime_supported(src: &Path) -> Result<&'static SupportedRuntime, String>`
  - `inference::extract_runtime_zip(root, src) -> Result<PathBuf, String>` (the extraction half, no check — the test seam) and `inference::install_runtime_from_zip(root, src) -> Result<PathBuf, String>` (check **then** extract; the only thing production calls)
  - `inference::install_model_from_file(root, src, expect: Option<&str>) -> Result<PathBuf, String>`
  - `inference::remove_model(root) -> Result<(), String>`
  - `inference::Half::{Runtime, Model}` with `Half::parse(&str) -> Option<Half>`
  - `inference::install_from_file(root, half, src) -> Result<PathBuf, String>` and `inference::install_from_manifest(root, half, manifest_url) -> Result<PathBuf, String>` — **the composition lives here, not in `commands.rs`** (M7)
  - `inference::{Asset, Manifest, MANIFEST_URL, fetch_manifest, download_to}`

- [ ] **Step 1: The three dependencies**

In `app/Cargo.toml`, after the `rfd` line:

```toml
# Knowlu plan 3a Task 8. All three are ALREADY in app/Cargo.lock through the graph — `sha2` and
# `zip` transitively, `ureq` through `quinn-ops` — so naming them here adds no compilation and no
# new version to audit; cargo unifies them onto the versions already resolved.
#
# `sha2`: a downloaded executable is verified against a pinned digest before it is ever run. The
# updater's minisign discipline does not apply — upstream's llama.cpp build is not signed by us —
# so a hash pinned in a manifest we serve is what stands in for it.
sha2 = "0.10"
# `zip`: upstream ships the Windows runtime as a zip. Deflate only, no encryption — the same
# feature set the engine crate takes for backup snapshots.
zip = { version = "2", default-features = false, features = ["deflate"] }
# `ureq`: the manifest fetch and the asset download. Default features, matching the engine's line
# minus `cookies`; nothing here needs a jar.
ureq = "3.4.0"
```

- [ ] **Step 2: Write the failing tests** — create `app/tests/inference.rs`:

```rust
use quinn_ops_console::inference::{
    extract_runtime_zip, install_model_from_file, install_runtime_from_zip, judgments_dir,
    model_file, models_dir, remove_model, runtime_dir, runtime_exe, runtime_release_for, sha256_of,
    status, Half, Manifest, SUPPORTED_RUNTIMES,
};
use std::io::Write;
use std::path::{Path, PathBuf};

fn root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-inference-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The empty digest of the empty string, and one known vector — so a wrong hash function is caught
/// here rather than by an install that mysteriously always refuses.
#[test]
fn sha256_of_matches_the_known_vectors() {
    let d = root("sha");
    let f = d.join("x");
    std::fs::write(&f, b"").unwrap();
    assert_eq!(sha256_of(&f).unwrap(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    std::fs::write(&f, b"abc").unwrap();
    assert_eq!(sha256_of(&f).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert!(sha256_of(&d.join("nope")).is_err(), "a missing file is an error, never a hash of nothing");
    let _ = std::fs::remove_dir_all(&d);
}

/// The layout: `runtime\` and `models\` under the install-wide root, `judgments\` under the
/// PROFILE's folder. Two vaults share a model and do not share a judgment log (spec §5.4).
#[test]
fn the_layout_puts_artefacts_install_wide_and_the_log_per_profile() {
    let r = root("layout");
    assert_eq!(runtime_dir(&r), r.join("runtime"));
    assert_eq!(models_dir(&r), r.join("models"));
    let profile = r.join("profiles").join("profile_abc");
    assert_eq!(judgments_dir(&profile), profile.join("judgments"));
    assert!(!judgments_dir(&profile).starts_with(models_dir(&r)));
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn nothing_is_installed_on_a_fresh_root() {
    let r = root("fresh");
    let s = status(&r);
    assert!(s.runtime.is_none() && s.model.is_none());
    assert_eq!(s.model_bytes, 0);
    assert!(runtime_exe(&r).is_none() && model_file(&r).is_none());
    let _ = std::fs::remove_dir_all(&r);
}

/// A model is verified against the digest it was offered under, and a mismatch leaves NOTHING
/// behind — not the bad bytes, not a partial, not a half-installed directory.
#[test]
fn a_model_is_verified_before_it_is_installed_and_a_bad_hash_leaves_nothing() {
    let r = root("model");
    let src = r.join("downloaded.gguf");
    std::fs::write(&src, b"abc").unwrap();
    let good = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    let err = install_model_from_file(&r, &src, Some("0000000000000000000000000000000000000000000000000000000000000000")).unwrap_err();
    assert!(err.contains("does not match"), "{err}");
    assert!(model_file(&r).is_none(), "a refused model must not be installed");
    assert!(!models_dir(&r).join("downloaded.gguf").exists());
    assert_eq!(std::fs::read_dir(models_dir(&r)).map(|d| d.flatten().count()).unwrap_or(0), 0, "no partial left behind");

    let out = install_model_from_file(&r, &src, Some(good)).unwrap();
    assert_eq!(out, models_dir(&r).join("downloaded.gguf"));
    assert_eq!(model_file(&r), Some(out.clone()));
    assert_eq!(status(&r).model_bytes, 3);
    assert!(src.is_file(), "the source the user pointed at is never moved or deleted");

    remove_model(&r).unwrap();
    assert!(model_file(&r).is_none(), "removing the model frees the gigabytes and nothing else");
    let _ = std::fs::remove_dir_all(&r);
}

/// A file that is not a `.gguf` is refused by name: pointing at the wrong file in a picker is the
/// commonest mistake, and a two-gigabyte copy is an expensive way to find out.
#[test]
fn only_a_gguf_is_accepted_as_a_model() {
    let r = root("ext");
    let src = r.join("notes.txt");
    std::fs::write(&src, b"abc").unwrap();
    let err = install_model_from_file(&r, &src, None).unwrap_err();
    assert!(err.contains(".gguf"), "{err}");
    let _ = std::fs::remove_dir_all(&r);
}

/// S6: the binary's name comes from the engine's constant, never a literal — so Task 1's Outcome B,
/// which renames it to `llama-cli.exe`, costs this file nothing.
fn server_exe() -> &'static str { quinn_ops::runtime::RUNTIME_EXE }

fn make_zip(path: &Path, entries: &[(String, &[u8])]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let o: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    for (name, body) in entries {
        z.start_file(name.as_str(), o).unwrap();
        z.write_all(body).unwrap();
    }
    z.finish().unwrap();
}

/// The runtime arrives as a zip. Every entry is extracted, the server binary is found at any depth,
/// and a zip that does not contain one is refused with everything cleaned up.
///
/// Drives `extract_runtime_zip` — the half without the digest check — because a zip this test builds
/// can never be in `SUPPORTED_RUNTIMES`, and pinning a fake digest into the shipped table to make a
/// test pass would defeat the table's whole purpose. The check itself is asserted below.
#[test]
fn a_runtime_zip_is_extracted_and_the_server_is_found_at_any_depth() {
    let r = root("zip");
    let src = r.join("rt.zip");
    make_zip(&src, &[(format!("build/bin/{}", server_exe()), b"MZfake" as &[u8]), ("build/bin/ggml.dll".to_string(), b"dll")]);
    let out = extract_runtime_zip(&r, &src).unwrap();
    assert!(out.ends_with(server_exe()), "{out:?}");
    assert_eq!(runtime_exe(&r), Some(out));
    assert!(runtime_dir(&r).join("build").join("bin").join("ggml.dll").is_file(), "its DLLs come with it");

    let r2 = root("zipbad");
    let bad = r2.join("rt.zip");
    make_zip(&bad, &[("readme.txt".to_string(), b"nothing useful" as &[u8])]);
    let err = extract_runtime_zip(&r2, &bad).unwrap_err();
    assert!(err.contains(server_exe()), "{err}");
    assert!(runtime_exe(&r2).is_none());
    assert!(!runtime_dir(&r2).exists(), "a refused runtime leaves no half-extracted directory");
    let _ = std::fs::remove_dir_all(&r);
    let _ = std::fs::remove_dir_all(&r2);
}

/// **R-P3a-2, and this is the test that proves the rule is on the production path.** The zip above
/// extracts cleanly through `extract_runtime_zip`; through `install_runtime_from_zip` — the only
/// function `commands.rs` calls — the same zip is refused, because its digest is not in the table,
/// and the refusal prints the digest so a user can report it.
#[test]
fn a_runtime_whose_digest_is_not_in_the_table_is_refused_by_the_production_path() {
    let r = root("unsupported");
    let src = r.join("rt.zip");
    make_zip(&src, &[(server_exe().to_string(), b"MZfake" as &[u8])]);
    let computed = sha256_of(&src).unwrap();
    let err = install_runtime_from_zip(&r, &src).unwrap_err();
    assert!(err.contains(&computed), "the refusal must print the digest to report: {err}");
    assert!(err.to_lowercase().contains("not a runtime release knowlu knows"), "{err}");
    assert!(runtime_exe(&r).is_none(), "nothing was installed");
    assert!(!runtime_dir(&r).exists(), "and nothing was extracted on the way to finding out");
    assert!(runtime_release_for(&computed).is_none());
    let _ = std::fs::remove_dir_all(&r);
}

/// Whatever the table holds, it holds it in the shape the lookup expects: a 64-character lowercase
/// hex digest, a non-empty tag and a non-empty asset name. Vacuously true on an empty table, which
/// is Outcome C's state — Task 12 step 8 is what refuses to close on an empty one.
#[test]
fn every_pinned_runtime_digest_is_well_formed() {
    for r in SUPPORTED_RUNTIMES {
        assert_eq!(r.sha256.len(), 64, "{}: {}", r.asset, r.sha256);
        assert!(r.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{}", r.sha256);
        assert!(!r.tag.is_empty() && !r.asset.is_empty(), "{r:?}");
        assert_eq!(runtime_release_for(&r.sha256.to_ascii_uppercase()).map(|f| f.asset), Some(r.asset),
            "the lookup is case-insensitive: a digest pasted from a checksum file is usually upper case");
    }
}

/// Zip slip: an archive entry naming `..\` or an absolute path must never write outside the runtime
/// directory. This is the one place this app extracts an archive it did not create.
#[test]
fn an_entry_that_escapes_the_runtime_directory_is_refused() {
    let r = root("slip");
    let src = r.join("evil.zip");
    make_zip(&src, &[("../escaped.exe".to_string(), b"MZ" as &[u8]), (server_exe().to_string(), b"MZ")]);
    let err = extract_runtime_zip(&r, &src).unwrap_err();
    assert!(err.to_lowercase().contains("outside"), "{err}");
    assert!(!r.join("escaped.exe").exists(), "nothing was written outside the runtime directory");
    let _ = std::fs::remove_dir_all(&r);
}

/// The manifest is data, and a malformed one is an error rather than a panic — it comes off the
/// network and this app never trusts what it is handed.
#[test]
fn a_manifest_parses_or_says_why_not() {
    let m: Manifest = serde_json::from_str(
        r#"{"runtime":{"name":"llama-b1.zip","url":"https://example.invalid/a.zip","sha256":"aa","bytes":1},
            "model":{"name":"m.gguf","url":"https://example.invalid/m.gguf","sha256":"bb","bytes":2}}"#,
    ).unwrap();
    assert_eq!(m.runtime.name, "llama-b1.zip");
    assert_eq!(m.model.sha256, "bb");
    assert!(serde_json::from_str::<Manifest>(r#"{"runtime":{}}"#).is_err());
}

/// The engine and the app must look for the same file name, or an installed runtime is invisible.
///
/// Asserted through the constant on both sides rather than against a literal (S6): under Task 1's
/// Outcome B that constant is `llama-cli.exe`, and this test must keep meaning the same thing.
#[test]
fn the_app_and_the_engine_agree_on_the_runtime_binarys_name() {
    let r = root("name");
    std::fs::create_dir_all(runtime_dir(&r)).unwrap();
    std::fs::write(runtime_dir(&r).join(server_exe()), b"MZ").unwrap();
    assert!(runtime_exe(&r).is_some(), "the app must find what the engine will look for");
    assert!(server_exe().ends_with(".exe"), "it is an executable and the resolver matches on the name");
    let _ = std::fs::remove_dir_all(&r);
}

/// `Half` is what both commands parse their `kind` argument into, so a typo is refused by name in
/// one place rather than by an `else` arm in each of them (M7).
#[test]
fn half_parses_exactly_two_words() {
    assert_eq!(Half::parse("runtime"), Some(Half::Runtime));
    assert_eq!(Half::parse("model"), Some(Half::Model));
    assert_eq!(Half::parse("Runtime"), None, "exact, so a typo is a refusal and not a surprise");
    assert_eq!(Half::parse(""), None);
}
```

`zip` is needed by the test to build fixtures, so add it under `[dev-dependencies]` too if `cargo test` reports it unavailable to integration tests — it is a normal dependency of the crate, so `zip::ZipWriter` is reachable from `app/tests/*` only via the crate; write the helper to use `zip::write::ZipWriter` through the `zip` crate named in `[dependencies]`, which integration tests can also use because they link the same crate graph. If the compiler disagrees, add the identical line under `[dev-dependencies]`.

- [ ] **Step 3: Run them to verify they fail**

Run: `cd app; cargo test --test inference`
Expected: FAIL to compile — `unresolved import quinn_ops_console::inference`.

- [ ] **Step 4: Write the module**

Create `app/src/inference.rs`:

```rust
//! Where the local inference runtime and the model live, and how a verified one gets there
//! (Knowlu spec §5.3, decision 12).
//!
//! **Three rules, and every one of them is a function below.**
//!
//! 1. **Never automatic** (§5.3: "the model download is a settings action and an onboarding offer,
//!    never automatic"). Nothing in this module is called by a tick, a housekeeping thread or a
//!    slot. Every entry point is reached from a command the user clicked.
//! 2. **There is no unverified path to executing a runtime binary** (ruling R-P3a-2). A
//!    `llama-server.exe` is an executable this app will run with the user's own privileges, and the
//!    updater's minisign discipline does not reach it — upstream's build is not signed by us. So
//!    the digests live **in the app**: [`SUPPORTED_RUNTIMES`] is a compiled-in table of release
//!    tag, asset name and SHA-256, and **both** install paths check against it. A manifest is a
//!    convenience for finding the bytes, never the root of trust — which matters because the path
//!    that works today is *Install from a file…*, where there is no manifest at all and where a
//!    design that took the digest from the caller would be taking it from nobody.
//!
//!    An unrecognised digest is **refused**, and the refusal prints the computed digest so a user
//!    can report it and a supported release can be added deliberately.
//!
//!    **The model is not in the table, and the asymmetry is deliberate.** A `.gguf` is *data*,
//!    parsed by a runtime the table has already vouched for and never executed, and §5.3 allows any
//!    1–4 B Q4 model — pinning model digests would make the feature useless to anyone who wants a
//!    different one. A model is checked against the manifest's digest when it came from a manifest,
//!    and accepted on the user's own authority when they pointed at it.
//! 3. **Temp-then-rename**, like `updates::stage_bytes`: a download that dies half way leaves a
//!    `.part<pid>` beside the target and never a partial at the name the judge resolves.
//!
//! **Where things live.** `runtime\` and `models\` are under the **install-wide** app-data root,
//! beside `updates\` — a llama.cpp build and a `.gguf` are immutable artefacts identical for every
//! profile, and a friend with two vaults must not download two gigabytes twice. The judgment LOG is
//! per profile (`judgments_dir`), because two vaults are two different sets of judgments.
//!
//! **A deviation from spec §5.3, recorded** (ruling R-P3a-1). The amended §5.3 says the runtime
//! ships as a second Tauri sidecar (`bundle.externalBin`). The process boundary is kept and the
//! bundling is not, for three reasons and none of them about installer size: the runtime is
//! **optional** (most installs will never enable local judgment), it **versions independently** of
//! this app, and it is **consistent with the models**, which §5.3 already downloads after install.
//! The repo's own figure for its size is the product plan's +20–50 MB
//! (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`).
//! `app/tauri.conf.json` is untouched, and `quinn_ops::runtime::resolve`'s sibling branch leaves the
//! door open to bundling later without a code change.

use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The install-wide runtime directory. The zip is extracted here whole, structure and all.
pub fn runtime_dir(root: &Path) -> PathBuf {
    root.join("runtime")
}

/// The install-wide model directory. One `.gguf` at a time in practice; `model_file` picks the
/// newest if a user has left two.
pub fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

/// **Per profile**, not per install (spec §5.4): `profiles\<id>\judgments\`. Takes the profile's own
/// `data_dir`, which `ConsoleState` already carries, so nothing here has to re-derive an id.
pub fn judgments_dir(profile_data_dir: &Path) -> PathBuf {
    profile_data_dir.join("judgments")
}

/// Find `llama-server.exe` anywhere under `runtime\`, to a bounded depth.
///
/// Bounded because upstream has shipped the Windows build both flat and under `build\bin\`, and a
/// recursive walk of a directory the user can put anything in is not something to leave unbounded.
/// The name is `quinn_ops::runtime::RUNTIME_EXE`, so the app and the engine cannot look for
/// different files — a test asserts it.
pub fn runtime_exe(root: &Path) -> Option<PathBuf> {
    fn find(dir: &Path, depth: usize) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        let mut dirs = Vec::new();
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.file_name().map(|n| n.eq_ignore_ascii_case(quinn_ops::runtime::RUNTIME_EXE)) == Some(true) {
                    return Some(path);
                }
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
        dirs.sort();
        dirs.into_iter().find_map(|d| find(&d, depth - 1))
    }
    find(&runtime_dir(root), 4)
}

/// The newest `.gguf` directly in `models\`. Newest rather than first so that installing a second
/// model switches to it without the user having to delete the old one by hand.
pub fn model_file(root: &Path) -> Option<PathBuf> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(models_dir(root))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("gguf")) == Some(true))
        .filter_map(|p| {
            let t = p.metadata().ok().and_then(|m| m.modified().ok())?;
            Some((t, p))
        })
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// What the settings row renders. `Serialize` so `commands.rs` marshals it and computes nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    pub runtime: Option<PathBuf>,
    pub model: Option<PathBuf>,
    /// The model's size, so the row can say "2.1 GB" beside the name. `0` when there is none.
    pub model_bytes: u64,
}

pub fn status(root: &Path) -> Status {
    let model = model_file(root);
    let model_bytes = model.as_ref().and_then(|p| p.metadata().ok()).map(|m| m.len()).unwrap_or(0);
    Status { runtime: runtime_exe(root), model, model_bytes }
}

/// Streamed, in 1 MiB chunks: a `.gguf` is gigabytes and reading it whole to hash it would allocate
/// the file into memory on a laptop that is about to load it anyway.
pub fn sha256_of(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("{}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `Ok` when `expect` is `None` — used only for a **model** a user pointed at themselves, never for
/// a runtime — or when the digest matches, case-insensitively.
fn verify(path: &Path, expect: Option<&str>) -> Result<(), String> {
    let Some(want) = expect else { return Ok(()) };
    let got = sha256_of(path)?;
    if got.eq_ignore_ascii_case(want.trim()) {
        Ok(())
    } else {
        Err(format!("{} does not match the digest it was offered under (expected {want}, got {got})", path.display()))
    }
}

/// One upstream llama.cpp release this build is prepared to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SupportedRuntime {
    /// The upstream release tag, e.g. `b1234`. Shown in the settings row so a user knows what they
    /// have.
    pub tag: &'static str,
    /// The asset's own file name in that release.
    pub asset: &'static str,
    /// Lower-case hex SHA-256 of the **archive as published**, not of the extracted exe.
    pub sha256: &'static str,
}

/// **The root of trust for executing a runtime** (ruling R-P3a-2). Compiled in, so it cannot be
/// edited by whatever also edited the file being installed, and so *Install from a file…* — the only
/// path that works before the site exists — is as verified as *Download*.
///
/// **Seeded by Task 1's spike, never invented.** Paste the `Runtime:` line from
/// `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`: its release tag, its asset name,
/// and the SHA-256 the spike computed over the archive it actually downloaded and then measured. If
/// the spike's outcome was C — nothing could be obtained — this table is **empty**, every runtime
/// install is refused, and that is the honest state: a digest that was never seen cannot be vouched
/// for. Task 12 step 8 refuses to close the plan on an empty table without the spike report saying
/// Outcome C.
///
/// Adding a release later is a code change and a release of this app, which is the point: it is a
/// deliberate act with a diff, not a file someone dropped in a folder.
pub const SUPPORTED_RUNTIMES: &[SupportedRuntime] = &[
    // Task 1 fills this from the spike report, e.g.:
    // SupportedRuntime { tag: "b1234", asset: "llama-b1234-bin-win-cpu-x64.zip", sha256: "<64 hex>" },
];

/// The table entry for a digest, case-insensitively — a checksum pasted from an upstream `.sha256`
/// file is usually upper case.
pub fn runtime_release_for(sha256: &str) -> Option<&'static SupportedRuntime> {
    let want = sha256.trim();
    SUPPORTED_RUNTIMES.iter().find(|r| r.sha256.eq_ignore_ascii_case(want))
}

/// Hash `src` and look it up. **The one gate on executing a runtime**, and the reason
/// `install_runtime_from_zip` is the only function `commands.rs` may call.
pub fn check_runtime_supported(src: &Path) -> Result<&'static SupportedRuntime, String> {
    let got = sha256_of(src)?;
    runtime_release_for(&got).ok_or_else(|| {
        format!(
            "{} is not a runtime release Knowlu knows (sha256 {got}). \
             Knowlu only runs llama.cpp builds whose digest it ships; report this one and it can be \
             added in a release.",
            src.display()
        )
    })
}

/// Copy a `.gguf` into `models\`, verified first.
///
/// **The source is never moved or deleted** — it is a file the user pointed at, quite possibly the
/// only copy, and quite possibly in their Downloads folder where they will look for it again.
/// Copied to `.part<pid>` and renamed, so a copy that dies half way never leaves a truncated model
/// at a name `model_file` would then hand to llama-server.
pub fn install_model_from_file(root: &Path, src: &Path, expect: Option<&str>) -> Result<PathBuf, String> {
    if src.extension().map(|x| x.eq_ignore_ascii_case("gguf")) != Some(true) {
        return Err(format!("{} is not a .gguf model file", src.display()));
    }
    let dir = models_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    verify(src, expect)?;
    let name = src.file_name().ok_or_else(|| format!("{}: no file name", src.display()))?;
    let dest = dir.join(name);
    let tmp = dir.join(format!("{}.part{}", name.to_string_lossy(), std::process::id()));
    let copied = std::fs::copy(src, &tmp).map_err(|e| format!("{}: {e}", tmp.display()));
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", dest.display())
    })?;
    Ok(dest)
}

/// Free the gigabytes. Every `.gguf` in `models\` goes; the runtime stays, because it is small and
/// re-downloading it is the annoying half.
pub fn remove_model(root: &Path) -> Result<(), String> {
    let dir = models_dir(root);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(()) };
    for path in entries.flatten().map(|e| e.path()) {
        if path.extension().map(|x| x.eq_ignore_ascii_case("gguf")) == Some(true) {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// **Check the digest, then extract.** The only runtime install `commands.rs` may call, and the
/// reason the check cannot be forgotten at a call site: it is inside the function, before anything
/// is written (ruling R-P3a-2).
pub fn install_runtime_from_zip(root: &Path, src: &Path) -> Result<PathBuf, String> {
    check_runtime_supported(src)?;
    extract_runtime_zip(root, src)
}

/// Extract a llama.cpp Windows zip into `runtime\`. **No digest check** — that is
/// [`install_runtime_from_zip`]'s, and this half is separated out so the extraction rules
/// (structure, zip slip, the `.new` swap) are testable against an archive a test built, which by
/// construction can never be in [`SUPPORTED_RUNTIMES`]. Pinning a fake digest into the shipped table
/// to make a test pass would defeat the table.
///
/// **Into a fresh `.new` directory, then swapped**, so a half-extracted archive is never what
/// `runtime_exe` finds — and if the archive turns out not to contain a server binary at all,
/// nothing is left behind at the real name.
///
/// **Zip slip is refused by name.** This is the one place the app extracts an archive it did not
/// create: an entry naming `..` or an absolute path would otherwise write anywhere the user can
/// write, and one of the things this directory sits beside is `updates\`.
pub fn extract_runtime_zip(root: &Path, src: &Path) -> Result<PathBuf, String> {
    let dir = runtime_dir(root);
    let staging = root.join("runtime.new");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;

    let result = (|| -> Result<(), String> {
        let file = std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{}: not a zip ({e})", src.display()))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| format!("{}: {e}", src.display()))?;
            // `enclosed_name` is the zip crate's own zip-slip guard: `None` for an absolute path,
            // a `..` component or anything else that would escape. Refused, never skipped — an
            // archive carrying one is not an archive to install part of.
            let Some(rel) = entry.enclosed_name() else {
                return Err(format!("{}: an entry points outside the runtime directory", src.display()));
            };
            let out = staging.join(&rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            let mut sink = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
            std::io::copy(&mut entry, &mut sink).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    // The archive has to contain the thing the judge will look for, or it is the wrong archive.
    let found = {
        let probe_root = staging.clone();
        // `runtime_exe` looks under `<root>/runtime`, so probe the staging directory as if it were
        // one: a temporary parent whose `runtime\` IS the staging folder.
        let fake_parent = root.join("runtime.probe");
        let _ = std::fs::remove_dir_all(&fake_parent);
        std::fs::create_dir_all(&fake_parent).map_err(|e| format!("{}: {e}", fake_parent.display()))?;
        let link = fake_parent.join("runtime");
        std::fs::rename(&probe_root, &link).map_err(|e| format!("{}: {e}", link.display()))?;
        let hit = runtime_exe(&fake_parent).is_some();
        std::fs::rename(&link, &staging).map_err(|e| format!("{}: {e}", staging.display()))?;
        let _ = std::fs::remove_dir_all(&fake_parent);
        hit
    };
    if !found {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!("{} contains no {}", src.display(), quinn_ops::runtime::RUNTIME_EXE));
    }

    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&staging, &dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging);
        format!("{}: {e}", dir.display())
    })?;
    runtime_exe(root).ok_or_else(|| format!("{} contains no {}", src.display(), quinn_ops::runtime::RUNTIME_EXE))
}

/// One downloadable artefact, as the manifest describes it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}

/// What `https://knowlu.com/inference/manifest.json` serves: the runtime build and the extraction
/// model this release was tested against, each with the digest it is verified by.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub runtime: Asset,
    pub model: Asset,
}

/// Beside `latest.json` on the same static site (spec §6: Cloudflare Pages, models on R2). **The
/// site does not exist yet** — Quinn's Cloudflare account is open item §11 — so this endpoint is
/// unreachable today, which is the expected state and not an incident, exactly as the updater's is.
/// Until it answers, *Install from a file…* is the working path, and it is a better one for a
/// friend on metered wifi anyway.
pub const MANIFEST_URL: &str = "https://knowlu.com/inference/manifest.json";

/// The manifest round-trip's budget, and the download's. Different numbers for the same reason
/// `updates::CHECK_TIMEOUT` and `DOWNLOAD_TIMEOUT` are: one is a small JSON GET, the other is two
/// gigabytes on a student's hotel wifi.
const MANIFEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60 * 60);

fn agent(timeout: std::time::Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(true)
        .build()
        .into()
}

pub fn fetch_manifest(url: &str) -> Result<Manifest, String> {
    let mut response = agent(MANIFEST_TIMEOUT).get(url).call().map_err(|e| e.to_string())?;
    let text = response
        .body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{url}: not an inference manifest ({e})"))
}

/// Which half is being installed. Parsed in one place so a typo is refused by name once, rather
/// than by an `else` arm in each of the two commands (M7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Runtime,
    Model,
}

impl Half {
    pub fn parse(kind: &str) -> Option<Half> {
        match kind {
            "runtime" => Some(Half::Runtime),
            "model" => Some(Half::Model),
            _ => None,
        }
    }
}

/// *Install from a file…*, for either half. **The runtime path checks the digest table**; the model
/// path takes the user's own word for a file they chose (see the module doc's rule 2).
pub fn install_from_file(root: &Path, half: Half, src: &Path) -> Result<PathBuf, String> {
    match half {
        Half::Runtime => install_runtime_from_zip(root, src),
        Half::Model => install_model_from_file(root, src, None),
    }
}

/// *Download*: read the manifest, fetch the named asset, install it.
///
/// **The composition lives here, not in `commands.rs`** (M7): `commands.rs` computes nothing, and
/// three network-and-disk steps chained together with two caps and a cleanup is computation. The
/// caps are per half — a runtime archive is tens of megabytes and a model is gigabytes — and a wrong
/// URL serving an endless stream therefore fills no disk on either.
///
/// The staged archive is removed on success: the runtime is now unpacked where it belongs, and the
/// zip is a second copy of it on a disk that is about to hold a model too.
pub fn install_from_manifest(root: &Path, half: Half, manifest_url: &str) -> Result<PathBuf, String> {
    let manifest = fetch_manifest(manifest_url)?;
    let (asset, cap) = match half {
        Half::Runtime => (&manifest.runtime, 512u64 << 20),
        Half::Model => (&manifest.model, 8u64 << 30),
    };
    let dest = root.join("downloads").join(&asset.name);
    download_to(&asset.url, &dest, Some(&asset.sha256), cap)?;
    // The runtime is re-checked against the compiled-in table by `install_runtime_from_zip`: the
    // manifest's digest proved the transport, the table decides whether we will run it (R-P3a-2).
    let out = match half {
        Half::Runtime => install_runtime_from_zip(root, &dest),
        Half::Model => install_model_from_file(root, &dest, Some(&asset.sha256)),
    };
    let _ = std::fs::remove_file(&dest);
    out
}

/// Download to `<dest>.part<pid>`, verify, then rename — the updater's `stage_bytes` shape, with the
/// verification in the middle so a bad transfer never reaches the name the installer will read.
///
/// `expect` is the **manifest's** digest and proves only the transport; for a runtime, whether the
/// bytes may be *executed* is decided afterwards by [`check_runtime_supported`] against the
/// compiled-in table. `None` is accepted for the same reason `verify` accepts it — a model the user
/// chose — and is never passed for a runtime.
///
/// `cap` bounds the response, so a wrong URL serving an endless stream fills no disk.
pub fn download_to(url: &str, dest: &Path, expect: Option<&str>, cap: u64) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let tmp = {
        let mut s = dest.as_os_str().to_os_string();
        s.push(format!(".part{}", std::process::id()));
        PathBuf::from(s)
    };
    let out = (|| -> Result<(), String> {
        let mut response = agent(DOWNLOAD_TIMEOUT).get(url).call().map_err(|e| e.to_string())?;
        let mut reader = response.body_mut().with_config().limit(cap).reader();
        let mut file = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::io::copy(&mut reader, &mut file).map_err(|e| format!("{}: {e}", tmp.display()))?;
        drop(file);
        verify(&tmp, expect)
    })();
    if let Err(e) = out {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", dest.display())
    })
}
```

- [ ] **Step 5: Seed `SUPPORTED_RUNTIMES` from the spike, and declare the module**

Open `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md` and copy its release tag, asset name and SHA-256 into the table as one `SupportedRuntime` entry — **the measured values, never invented ones.** Under **Outcome C** the table stays empty, every runtime install is refused with the message `check_runtime_supported` prints, and that is the honest state: nothing was ever seen, so nothing can be vouched for. Say which of the two happened in the task's review note, because Task 12 step 8 checks it.

In `app/src/lib.rs`, keeping the list alphabetical:

```rust
pub mod inference;
```

- [ ] **Step 6: Run the tests**

Run: `cd app; cargo test --test inference`
Expected: PASS. If `body_mut().with_config().limit(n).reader()` does not resolve, use `read_to_vec()` and write the bytes — `ureq 3.4`'s `BodyWithConfig` exposes both; the streaming form is preferred so a two-gigabyte model is not held in memory.

- [ ] **Step 7: Full app gate and commit**

Run: `cd app; cargo test` — expected PASS at zero new warnings.

```bash
git add app/src/inference.rs app/src/lib.rs app/Cargo.toml app/Cargo.lock app/tests/inference.rs
git commit -F .git/COMMIT_MSG_3A8
```

```
feat(inference): where the runtime and the model live, and how a verified one gets there (plan 3a, Task 8)

runtime\ and models\ under the install-wide app-data root beside updates\, because a llama.cpp
build and a .gguf are immutable artefacts identical for every profile and a friend with two vaults
must not download two gigabytes twice; judgments\ stays per profile.

Three rules, three functions: never automatic (nothing here is reached from a tick or a slot), NO
UNVERIFIED PATH TO EXECUTING A RUNTIME (ruling R-P3a-2 — SUPPORTED_RUNTIMES is a compiled-in table
of release tag, asset name and SHA-256, seeded from what Task 1 measured, and install_runtime_from_zip
checks it before anything is written; an unrecognised digest is refused and the refusal prints the
digest to report), and temp-then-rename. A manifest is a convenience for finding bytes, never the
root of trust — which matters because the path that works before the site exists is Install from a
file..., where there is no manifest at all. Models are not in the table and that asymmetry is
documented: a .gguf is data parsed by a runtime the table already vouched for, and §5.3 allows any
1-4B Q4 model.

The runtime zip is extracted into runtime.new and swapped only after the server binary is found in
it, with the zip crate's enclosed_name refusing zip slip outright — this is the one archive the app
extracts that it did not create. extract_runtime_zip is the digest-free half, split out so the
extraction rules are testable against an archive a test built, which by construction can never be
in the shipped table.

Deviation from the amended spec §5.3 recorded in the module doc (R-P3a-1): the runtime is optional,
versions independently of this app, and is the same kind of artefact as the models §5.3 already
downloads after install. Not a size argument - the repo's figure is the product plan's +20-50 MB.

sha2, zip and ureq are named in app/Cargo.toml and were already resolved in app/Cargo.lock.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 9: The slot runs it — coursework → ingest → **judge** → rank

The app's scheduler gains a fourth step, with a named skip when there is no runtime or no model, exactly as `ingest` already has one when there is no `ics_url`.

**Judge goes before `rank`** so the day's ranking sees the fields the judge just wrote — an enrichment landing after the rank would be a day late every time.

**Files:**
- Modify: `app/src/scheduler.rs` (`JudgeArgs`, `JudgeState`, `judge_state`, `slot_argv`, the skip step in `run_slot_inner`), `app/src/commands.rs` (`inference_root`), `app/tests/scheduler.rs`
- Read first: `app/src/scheduler.rs:158–194` (`IcsState`, `ics_state`, `has_ics_url`, `slot_argv`) and `:405–429` (where the skip is recorded); `app/src/commands.rs:482–500` (`updates_dir`, the app-data resolver this mirrors)

**Interfaces:**
- Consumes: from Task 7, the `quinn-ops judge` command and its flags; from Task 8, `inference::{runtime_exe, model_file, judgments_dir}`.
- Produces:
  - `scheduler::JudgeArgs { runtime: PathBuf, model: PathBuf, log_dir: PathBuf }`
  - `scheduler::JudgeState::{Ready(JudgeArgs), NoRuntime, NoModel}`
  - `scheduler::judge_state_in(root: &Path, cs: &ConsoleState) -> JudgeState`, and its thin wrapper `scheduler::judge_state(cs: &ConsoleState) -> JudgeState`
  - `scheduler::slot_argv(vault: &Path, exe: &Path, judge: Option<&JudgeArgs>) -> Vec<(PathBuf, Vec<String>)>` — **the signature changes; `app/tests/scheduler.rs` is the only other caller**
  - `commands::inference_root() -> Result<PathBuf, String>`

- [ ] **Step 1: Write the failing tests** — rename the existing test and replace its body.

`the_slot_runs_coursework_then_ingest_then_rank_and_leaves_ingest_out_without_a_feed` now covers four steps and two conditional ones, so its name is a lie by omission. Rename it to
`the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured`, and replace its body (currently lines 33–60) with:

```rust
    let v = scratch("argv");
    let exe = Path::new(r"C:\bin\quinn-ops.exe");
    let names = |a: &Vec<(PathBuf, Vec<String>)>| a.iter().map(|(_, x)| x[0].clone()).collect::<Vec<_>>();
    assert!(!has_ics_url(&v));
    // No judge args: the step is left out entirely, exactly as `ingest` is on a vault with no feed.
    let argv = slot_argv(&v, exe, None);
    assert_eq!(names(&argv), vec!["coursework", "rank"]);
    assert_eq!(argv[0].1, vec!["coursework", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert_eq!(argv[1].1, vec!["rank", "--vault", v.to_string_lossy().as_ref(), "--runner", "local"]);

    let cfg = v.join("config").join("ingest.yaml");
    let old = std::fs::read_to_string(&cfg).unwrap();
    std::fs::write(&cfg, format!("ics_url: \"https://lms.example.invalid/learn.ics\"\n{old}")).unwrap();
    assert!(has_ics_url(&v));
    let argv = slot_argv(&v, exe, None);
    assert_eq!(names(&argv), vec!["coursework", "ingest", "rank"]);
    assert_eq!(argv[1].1, vec!["ingest", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner"]);
    assert!(argv.iter().all(|(e, _)| e == exe));

    // With judge args: FOUR steps, and judge sits BEFORE rank so the day's ranking sees what it
    // just wrote. `--via local-runner`, the same value coursework and ingest pass — journal::VIAS
    // does not grow for this.
    let ja = JudgeArgs {
        runtime: PathBuf::from(r"C:\rt\llama-server.exe"),
        model: PathBuf::from(r"C:\rt\model.gguf"),
        log_dir: PathBuf::from(r"C:\data\judgments"),
    };
    let argv = slot_argv(&v, exe, Some(&ja));
    assert_eq!(names(&argv), vec!["coursework", "ingest", "judge", "rank"]);
    assert_eq!(argv[2].1, vec![
        "judge".to_string(), "--vault".to_string(), v.to_string_lossy().to_string(),
        "--via".to_string(), "local-runner".to_string(),
        "--runtime".to_string(), r"C:\rt\llama-server.exe".to_string(),
        "--model".to_string(), r"C:\rt\model.gguf".to_string(),
        "--log-dir".to_string(), r"C:\data\judgments".to_string(),
    ]);
    assert!(quinn_ops::journal::VIAS.contains(&"local-runner"));

    std::fs::write(&cfg, "ics_url: \"   \"\n").unwrap();
    assert!(!has_ics_url(&v), "a blank url is no url — the engine would exit 1 on it");
    // R-P4a-17: the three states are distinguishable, and a missing file is "no url", not a fault.
    assert_eq!(ics_state(&v), IcsState::NoUrl);
    std::fs::write(&cfg, "ics_url: [unclosed\n").unwrap();
    assert_eq!(ics_state(&v), IcsState::Unreadable);
    std::fs::remove_file(&cfg).unwrap();
    assert_eq!(ics_state(&v), IcsState::NoUrl);
```

Then append two tests to the same file:

```rust
/// D7: no runtime and no model are NORMAL. The step is recorded with code 0 and a name that says
/// which half is missing — the shape `ingest (skipped: no ics_url)` already uses — so a friend with
/// no model sees an explanation on the Runs view rather than a slot that quietly does less.
#[test]
fn a_machine_with_no_model_records_the_judge_skip_and_stays_green() {
    let v = scratch("judgeskip");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", quinn_ops::journal::device_name()),
    ).unwrap();
    let cs = open(&v, "judgeskip");
    let sch = Scheduler::default();
    let summary = run_slot_inner(&cs, &sch, None, false);
    let names: Vec<String> = summary.steps.iter().map(|(n, _)| n.clone()).collect();
    let judge = names.iter().find(|n| n.starts_with("judge")).expect("a judge step, named");
    assert!(judge.contains("skipped: no runtime") || judge.contains("skipped: no model"), "{judge}");
    let code = summary.steps.iter().find(|(n, _)| n.starts_with("judge")).unwrap().1;
    assert_eq!(code, 0, "a skip is not a failure — it must never paint the tray amber");
    let _ = std::fs::remove_dir_all(&v);
}

/// `judge_state_in` names which half is missing — the runtime first, because it is the prerequisite
/// for the other.
///
/// **Against a temp root, never the real one** (review S3): `judge_state` reads
/// `%LOCALAPPDATA%\knowlu`, so a test written against it would pass today and start failing the
/// first time the settings row actually installed something on this machine.
#[test]
fn judge_state_names_the_missing_half_runtime_first() {
    let v = scratch("judgestate");
    let cs = open(&v, "judgestate");
    let root = std::env::temp_dir().join(format!("knowlu-judgestate-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    assert_eq!(judge_state_in(&root, &cs), JudgeState::NoRuntime, "neither half installed");

    let rt = quinn_ops_console::inference::runtime_dir(&root);
    std::fs::create_dir_all(&rt).unwrap();
    std::fs::write(rt.join(quinn_ops::runtime::RUNTIME_EXE), b"MZ").unwrap();
    assert_eq!(judge_state_in(&root, &cs), JudgeState::NoModel, "runtime but no model");

    let models = quinn_ops_console::inference::models_dir(&root);
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join("m.gguf"), b"gg").unwrap();
    match judge_state_in(&root, &cs) {
        JudgeState::Ready(a) => {
            assert!(a.runtime.ends_with(quinn_ops::runtime::RUNTIME_EXE));
            assert!(a.model.ends_with("m.gguf"));
            // The log is the PROFILE's, never the install-wide root's (spec §5.4).
            assert_eq!(a.log_dir, cs.data_dir.join("judgments"));
            assert!(!a.log_dir.starts_with(&root), "the log must not live beside the model");
        }
        other => panic!("expected Ready, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&v);
}
```

Add `JudgeArgs, JudgeState, judge_state_in` to the `use quinn_ops_console::scheduler::{…}` line at the top of the file. `JudgeState` needs `#[derive(Debug, Clone, PartialEq, Eq)]` for the `assert_eq!`s above, which is what Task 9 step 4 gives it.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app; cargo test --test scheduler`
Expected: FAIL to compile — `this function takes 2 arguments but 3 arguments were supplied`, and `cannot find type JudgeArgs`.

- [ ] **Step 3: `inference_root` in `commands.rs`**

Beside `updates_dir` in `app/src/commands.rs`:

```rust
/// The install-wide app-data root, where `runtime\` and `models\` live.
///
/// **One folder for the whole install, like `updates\` and unlike `judgments\`** — a llama.cpp build
/// and a `.gguf` are immutable artefacts identical for every profile, and a friend with two vaults
/// must not download two gigabytes twice. The judgment LOG is per profile, because two vaults are
/// two different sets of judgments (spec §5.4).
///
/// No temp fallback, for the same reason `updates_dir` has none (m4): a runtime is an executable
/// this app will run with the user's own privileges, and the system temp directory is writable by
/// every account on the machine. With nowhere private to put it, Knowlu declines and says why.
pub(crate) fn inference_root() -> Result<std::path::PathBuf, String> {
    crate::state::app_data_root()
        .ok_or_else(|| "LOCALAPPDATA is not set, so there is nowhere private to keep a model".to_string())
}
```

- [ ] **Step 4: `JudgeArgs`, `JudgeState`, `judge_state` and the new `slot_argv`**

In `app/src/scheduler.rs`, immediately after `has_ics_url`:

```rust
/// Everything `quinn-ops judge` needs that only the app knows: where the runtime was installed,
/// which model file was chosen, and this profile's own judgments directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeArgs {
    pub runtime: PathBuf,
    pub model: PathBuf,
    pub log_dir: PathBuf,
}

/// Why a slot will or will not run `judge` — the same three-state shape `IcsState` uses, and for
/// the same reason: the two "no" cases are different problems and get different words on the Runs
/// view. A friend who has installed neither is told about the runtime, because it is the
/// prerequisite; one who has the runtime and no model is told about the model.
///
/// **Neither is a fault.** Spec §5.3: "the app runs with no model present." The step is recorded
/// with exit code 0 and an explanatory name, exactly as `ingest (skipped: no ics_url)` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgeState {
    Ready(JudgeArgs),
    NoRuntime,
    NoModel,
}

/// Resolve the judge step under an explicit app-data root — the seam the tests use, mirroring
/// `state::app_data_root_in`.
///
/// **Not optional** (review S3): without it the test below reaches the developer's *real*
/// `%LOCALAPPDATA%\knowlu`, so it would pass today and start failing the first time Task 10's
/// settings row installed a runtime on this machine — a test that breaks when the feature starts
/// working is worse than no test.
pub fn judge_state_in(root: &Path, cs: &ConsoleState) -> JudgeState {
    let Some(runtime) = crate::inference::runtime_exe(root) else { return JudgeState::NoRuntime };
    let Some(model) = crate::inference::model_file(root) else { return JudgeState::NoModel };
    JudgeState::Ready(JudgeArgs { runtime, model, log_dir: crate::inference::judgments_dir(&cs.data_dir) })
}

/// Resolve the judge step for this profile. Reads the install-wide root for the runtime and the
/// model and the profile's own `data_dir` for the log directory (spec §5.4/§5.6: the log is per
/// profile and never in the vault). A root that cannot be resolved at all is `NoRuntime`: there is
/// nowhere for one to be.
pub fn judge_state(cs: &ConsoleState) -> JudgeState {
    let Ok(root) = crate::commands::inference_root() else { return JudgeState::NoRuntime };
    judge_state_in(&root, cs)
}

/// coursework → **ingest** → **judge** → rank, as child processes of the sibling engine exe.
/// `ingest` is included only when the vault has a feed; `judge` only when a runtime and a model are
/// both installed, which is what `judge_state` decides.
///
/// **`judge` sits before `rank`**: it writes `effort_hours`, `importance` and `course`, and a rank
/// that ran first would order the day from the values the judge was about to replace — every
/// enrichment would be a slot late, forever.
///
/// Never build; never write to the vault directly.
pub fn slot_argv(vault: &Path, exe: &Path, judge: Option<&JudgeArgs>) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    let mut steps = vec![(exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
    if has_ics_url(vault) {
        steps.push((exe.to_path_buf(), vec!["ingest".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
    }
    if let Some(j) = judge {
        steps.push((exe.to_path_buf(), vec![
            "judge".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
            "--runtime".into(), j.runtime.to_string_lossy().into_owned(),
            "--model".into(), j.model.to_string_lossy().into_owned(),
            "--log-dir".into(), j.log_dir.to_string_lossy().into_owned(),
        ]));
    }
    steps.push((exe.to_path_buf(), vec!["rank".into(), "--vault".into(), v, "--runner".into(), "local".into()]));
    steps
}
```

- [ ] **Step 5: Record the skip, and pass the args, in `run_slot_inner`**

In `app/src/scheduler.rs`'s `run_slot_inner`, replace the `match ics_state(&cs.vault) { … }` block and the `match engine_exe()` block with:

```rust
    // A skipped step is still a step: without this line the Runs view and the sync line would show
    // a slot with no ingest in it and no reason why. `0` because a skip is not a failure — the same
    // shape `sync_step` uses for a busy lock. Recorded before the engine is resolved, so a missing
    // exe does not also hide the explanation. The two reasons read differently (R-P4a-17).
    match ics_state(&cs.vault) {
        IcsState::Feed => {}
        IcsState::NoUrl => steps.push(("ingest (skipped: no ics_url)".to_string(), 0)),
        IcsState::Unreadable => steps.push(("ingest (skipped: config unreadable)".to_string(), 0)),
    }
    // The same rule for the judge (Knowlu spec §5.3, decision 12): a machine with no runtime and no
    // model runs everything else and says which half is missing. NEVER a non-zero code — that would
    // set `engine_ok = false`, paint the tray amber and put the slot into retry backoff twice a day
    // for a friend who has simply not downloaded a model.
    let judge = judge_state(cs);
    match &judge {
        JudgeState::Ready(_) => {}
        JudgeState::NoRuntime => steps.push(("judge (skipped: no runtime)".to_string(), 0)),
        JudgeState::NoModel => steps.push(("judge (skipped: no model)".to_string(), 0)),
    }
    let judge_args = match &judge {
        JudgeState::Ready(a) => Some(a),
        _ => None,
    };
    match engine_exe() {
        Ok(exe) => {
            for (i, (e, args)) in slot_argv(&cs.vault, &exe, judge_args).into_iter().enumerate() {
                let log = log_dir(cs).join(format!("slot-{}-{}-{}.txt", started.replace(':', ""), i, args[0]));
                let code = run_child(&e, &args, &log, CHILD_TIMEOUT);
                if code != 0 {
                    engine_ok = false;
                }
                steps.push((args[0].clone(), code));
            }
        }
        Err(e) => {
            engine_ok = false;
            steps.push((format!("engine: {e}"), -1));
        }
    }
```

- [ ] **Step 6: Update `LOGS_KEPT`'s doc**

The comment above `LOGS_KEPT` says "a vault with an LMS feed has three steps (coursework, ingest, rank), so two slots a day is six files a day: sixty is about twenty runs, or ten days." With the judge that is four steps and eight files. Replace the constant and its doc:

```rust
/// Slot log FILES kept in `log_dir()` — files, not runs. A slot writes one per step, and since
/// Knowlu plan 3a a vault with an LMS feed and a model installed has four steps (coursework,
/// ingest, judge, rank), so two slots a day is eight files a day: eighty is about twenty runs, or
/// ten days. Long enough that a bug report can quote the run that went wrong, short enough that the
/// directory stays bounded. `quit-*` records are neither counted nor pruned (see `prune_logs`).
const LOGS_KEPT: usize = 80;
```

- [ ] **Step 7: Run the app tests**

Run: `cd app; cargo test`
Expected: PASS, zero **new** warnings (the `.rsrc merge failure` linker line is pre-existing).

- [ ] **Step 8: Look at it once, against a scratch copy**

Never the live vault, never an interactive launch:

```powershell
.\scripts\scratch-vault.ps1
# use the path it printed, and give the copy an `app` scheduler entry for this device
$v = "<the printed path>"
"runners:`n  - name: local`n    times: [""12:00""]`n    tz: America/Chicago`n    grace_minutes: 20`n    device: $env:COMPUTERNAME`n    scheduler: app`n" | Set-Content -Encoding utf8 "$v\config\runners.yaml"
cargo build --release
.\app\target\release\knowlu.exe --vault $v --run-slot-once
```

Expected: the JSON summary carries a `judge (skipped: no runtime)` or `judge (skipped: no model)` step at code 0, `ok` may be false only because of `push`/`backup` on a scratch copy, and **`engine_ok` is true**.

- [ ] **Step 9: Commit**

```bash
git add app/src/scheduler.rs app/src/commands.rs app/tests/scheduler.rs
git commit -F .git/COMMIT_MSG_3A9
```

```
feat(scheduler): the slot runs judge, between ingest and rank (plan 3a, Task 9)

slot_argv takes Option<&JudgeArgs> and emits coursework -> ingest -> judge -> rank. Judge goes
before rank because it writes effort_hours, importance and course: a rank that ran first would
order the day from the values the judge was about to replace, so every enrichment would land a
slot late forever.

judge_state resolves the runtime and the model from the install-wide app-data root and the log
directory from this profile's own folder, and names which half is missing. Neither is a fault:
the step is recorded as `judge (skipped: no runtime)` at exit code 0, the shape
`ingest (skipped: no ics_url)` already uses, so a friend with no model never gets an amber tray
or a retry ladder. --via is local-runner, the value coursework and ingest already pass.

LOGS_KEPT 60 -> 80: four steps twice a day is eight files, and the constant's arithmetic was
written for three.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 10: The settings row, its states, and the onboarding offer

Spec §5.3: "the model download is a settings action and an onboarding offer, never automatic." This is both, plus a state for every outcome so the row never leaves the reader guessing (R7).

**Files:**
- Modify: `app/src/commands.rs` (five commands), `app/src/onboarding.rs` (`WizardPlan.offer_inference`, the marker file, `pick_file`), `app/src/main.rs` (the new handlers), `app/static/index.html` (`#set-judge`, the finish-panel offer), `app/static/console.js` (`renderInference`, the row's handlers, the first-launch offer), `app/tests/static_assets.rs`, `app/tests/commands.rs`
- Read first: `app/static/index.html:117–126` (the six existing settings rows), `app/static/console.js:1005–1055` (`openSettings`, `renderSettings`, the panel's one click handler), `app/tests/static_assets.rs:376–399` (what the panel test asserts)

**Interfaces:**
- Consumes: `inference::{status, Status, Half, install_from_file, install_from_manifest, install_model_from_file, remove_model, models_dir, runtime_dir, MANIFEST_URL}`; `commands::inference_root`. **The commands never call `fetch_manifest`, `download_to` or `install_runtime_from_zip` directly** — `install_from_manifest` composes those three, so `commands.rs` keeps computing nothing (M7) and the digest check cannot be skipped at a call site (R-P3a-2).
- Produces:
  - `commands::{inference_status, install_inference_file, install_inference_download, remove_inference_model}` (`#[tauri::command]`)
  - `onboarding::pick_file(app, title, extension) -> Value` (`#[tauri::command]`)
  - `onboarding::offer_marker(profile_dir: &Path) -> PathBuf` and `WizardPlan.offer_inference`
  - the page's `renderInference(s)` and the `#set-judge` row

- [ ] **Step 1: Write the failing page tests** — append to `app/tests/static_assets.rs`:

```rust
/// Knowlu plan 3a Task 10: the seventh settings row. Every state the reader can be in has words
/// (R7), the two install doors are both there, and nothing on this page reaches the network — the
/// manifest URL lives in `app/src/inference.rs`, never in the page.
#[test]
fn the_local_judgment_row_has_a_state_for_every_outcome() {
    let html = read("index.html");
    assert!(html.contains("id=\"set-judge\""), "the row");
    for id in ["set-judge-state", "set-judge-file", "set-judge-download", "set-judge-remove"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "{id}");
    }
    let js = read("console.js");
    assert!(js.contains("function renderInference("), "renderInference");
    for cmd in ["\"inference_status\"", "\"install_inference_file\"", "\"install_inference_download\"", "\"remove_inference_model\""] {
        assert!(js.contains(cmd), "{cmd}");
    }
    // The four states the row can be in, in the page's own words. **`"ready"` alone would be a dead
    // assertion** — `already` contains it, and this file is full of `already` (M3) — so the
    // rendered string is asserted with its separator.
    for words in ["runtime not installed", "model not installed", "\"ready — \"", "installing "] {
        assert!(js.contains(words), "the row must have words for: {words}");
    }
    // Global constraint: no http:// or https:// under app/static/ — the endpoint is Rust's.
    assert!(!js.contains("manifest.json"), "the manifest URL is inference.rs's, never the page's");
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// The wizard offers local judgment and never performs it: "Nothing is fetched now" is the wizard's
/// promise, and a two-gigabyte download during onboarding would break it (spec §5.3, D8).
#[test]
fn the_wizard_offers_local_judgment_without_doing_anything() {
    let html = read("index.html");
    assert!(html.contains("id=\"wiz-judge\""), "the finish panel's offer");
    assert!(html.contains("Nothing is fetched now."), "the wizard's promise still stands");
    assert!(!html.contains("Gmail proposals, event verdicts and enrichment are not here yet"),
        "enrichment ships in this plan — that sentence is now false");
    let js = read("console.js");
    assert!(js.contains("offer_inference"), "the plan carries the checkbox to apply_profile_settings");
    // D8, asserted where it can actually fail: neither install command may be reachable from the
    // wizard's own code. `startWizard`'s body is where a "helpful" download would be added, so that
    // is what is checked — not a hand-built negative string nothing would ever match (M3).
    let wizard = js
        .split("function startWizard(")
        .nth(1)
        .and_then(|s| s.split("\n  function ").next())
        .expect("startWizard");
    for cmd in ["install_inference_download", "install_inference_file"] {
        assert!(!wizard.contains(cmd), "the wizard offers and never installs: {cmd}");
    }
}
```

- [ ] **Step 2: Write the failing command test** — append to `app/tests/commands.rs`:

```rust
/// The settings row's four commands compute nothing (R9): `inference_status` marshals
/// `inference::status`, and the two installers refuse a bad path with the module's own words.
#[test]
fn the_inference_commands_marshal_and_refuse() {
    let root = std::env::temp_dir().join(format!("knowlu-cmd-inference-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let s = quinn_ops_console::inference::status(&root);
    let v = serde_json::to_value(&s).unwrap();
    assert!(v["runtime"].is_null() && v["model"].is_null() && v["model_bytes"] == 0);

    let notes = root.join("notes.txt");
    std::fs::write(&notes, b"x").unwrap();
    let err = quinn_ops_console::inference::install_model_from_file(&root, &notes, None).unwrap_err();
    assert!(err.contains(".gguf"), "{err}");

    // A model that installs, then is removed: the two ends of the row's lifecycle.
    let gguf = root.join("m.gguf");
    std::fs::write(&gguf, b"abc").unwrap();
    quinn_ops_console::inference::install_model_from_file(&root, &gguf, None).unwrap();
    assert!(quinn_ops_console::inference::status(&root).model.is_some());
    quinn_ops_console::inference::remove_model(&root).unwrap();
    assert!(quinn_ops_console::inference::status(&root).model.is_none());
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cd app; cargo test --test static_assets --test commands`
Expected: FAIL — `the row` and `renderInference`.

- [ ] **Step 4: The five commands**

Append to `app/src/commands.rs`:

```rust
// ---- Knowlu plan 3a Task 10: local judgment. Every one of these is reached from a click and from
// nowhere else (spec §5.3: "never automatic"); nothing here computes (spec §3.1) — `inference` does.

/// `{ok, error, runtime, model, model_bytes, root}` — what the settings row renders.
#[tauri::command]
pub fn inference_status() -> Value {
    match inference_root() {
        Err(e) => json!({ "ok": false, "error": e, "runtime": Value::Null, "model": Value::Null, "model_bytes": 0 }),
        Ok(root) => {
            let s = crate::inference::status(&root);
            json!({
                "ok": true, "error": Value::Null,
                "runtime": s.runtime.map(|p| p.to_string_lossy().to_string()),
                "model": s.model.map(|p| p.to_string_lossy().to_string()),
                "model_bytes": s.model_bytes,
                "root": root.to_string_lossy(),
            })
        }
    }
}

/// One place both install commands turn `kind` into a `Half` and a missing root into an envelope,
/// so neither of them grows an `else` arm of its own (M7).
fn half_and_root(kind: &str) -> Result<(crate::inference::Half, std::path::PathBuf), Value> {
    let half = crate::inference::Half::parse(kind)
        .ok_or_else(|| json!({ "ok": false, "error": format!("{kind} is not something Knowlu installs") }))?;
    let root = inference_root().map_err(|e| json!({ "ok": false, "error": e }))?;
    Ok((half, root))
}

fn installed(done: Result<std::path::PathBuf, String>) -> Value {
    match done {
        Ok(p) => json!({ "ok": true, "error": Value::Null, "path": p.to_string_lossy() }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// *Install from a file…*, for either half. **The working path until the site exists** (the
/// manifest endpoint is open item §11), and the better one for a friend on metered wifi or a campus
/// network that blocks the host: they download once, by hand, and point at it.
///
/// **It is not the unverified path** (R-P3a-2): `inference::install_from_file` sends a runtime
/// through `install_runtime_from_zip`, which checks the file's SHA-256 against the compiled-in
/// `SUPPORTED_RUNTIMES` before anything is written. A model is taken on the user's own authority,
/// for the reason the module doc gives.
///
/// `(async)` because a two-gigabyte copy and a SHA-256 over it must not run on the webview thread.
#[tauri::command(async)]
pub fn install_inference_file(kind: String, path: String) -> Value {
    let (half, root) = match half_and_root(&kind) { Ok(v) => v, Err(e) => return e };
    installed(crate::inference::install_from_file(&root, half, std::path::Path::new(&path)))
}

/// *Download*, for either half. Reached only from the row's button — no tick and no slot calls it
/// (D8) — and it marshals only: the manifest read, the capped download and the install are one
/// function in `inference`, because chaining three network-and-disk steps is computation and
/// `commands.rs` computes nothing (M7).
#[tauri::command(async)]
pub fn install_inference_download(kind: String) -> Value {
    let (half, root) = match half_and_root(&kind) { Ok(v) => v, Err(e) => return e };
    installed(crate::inference::install_from_manifest(&root, half, crate::inference::MANIFEST_URL))
}

/// Free the gigabytes. The runtime stays: it is small, and re-downloading it is the annoying half.
#[tauri::command(async)]
pub fn remove_inference_model() -> Value {
    let root = match inference_root() { Ok(r) => r, Err(e) => return json!({ "ok": false, "error": e }) };
    match crate::inference::remove_model(&root) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
```

- [ ] **Step 5: The file picker and the onboarding marker**

In `app/src/onboarding.rs`, beside `pick_folder`:

```rust
/// A single file, filtered by one extension. The folder picker's twin — `rfd` is already this
/// crate's dialog (`tauri-plugin-dialog` cannot be used on this toolchain; see `app/Cargo.toml`).
#[tauri::command]
pub fn pick_file(app: tauri::AppHandle, title: String, extension: String) -> Value {
    let mut d = rfd::FileDialog::new().set_title(&title).add_filter(&extension, &[extension.as_str()]);
    if let Some(w) = app.get_webview_window("main") { d = d.set_parent(&w); }
    let path = d.pick_file();
    json!({ "ok": true, "error": Value::Null, "path": path.map(|p| p.to_string_lossy().to_string()) })
}

/// The wizard's *show me how after setup* marker.
///
/// A **file in the profile folder, not a `Settings` field**, and deliberately: `state::Settings`
/// derives `Deserialize` with no `#[serde(default)]` on any field, so adding one would make every
/// existing `settings.json` fail to parse — and `Settings::load` falls back to defaults on a parse
/// failure, which would silently reset a friend's backup folder and autostart choice on the first
/// launch after an update. A zero-byte marker costs nothing and breaks nothing.
pub fn offer_marker(profile_dir: &Path) -> PathBuf {
    profile_dir.join("offer-inference")
}
```

In `struct WizardPlan`, add — **with `#[serde(default)]`**, so a page that does not send it is not a deserialization failure:

```rust
    /// The finish panel's *Set up local judgment after setup* checkbox. The wizard NEVER installs
    /// anything (spec §5.3, "never automatic", and the wizard's own "Nothing is fetched now"); this
    /// only drops a marker the console reads once on its first launch.
    #[serde(default)]
    pub offer_inference: bool,
```

and at the end of `apply_profile_settings`, after the `settings.save(...)` match arm's `Ok(())`, replace the body with:

```rust
    let dir = profiles::profile_dir(&root, &id);
    match settings.save(&dir.join("settings.json")) {
        Ok(()) => {
            // Best effort: an unwritable marker costs the reader one prompt, not their setup.
            if plan.offer_inference {
                let _ = std::fs::write(offer_marker(&dir), b"");
            } else {
                let _ = std::fs::remove_file(offer_marker(&dir));
            }
            json!({ "ok": true, "error": Value::Null })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
```

In `commands::settings_context`, add one key so the console can read and clear the marker in the same call it already makes:

```rust
    // Knowlu plan 3a Task 10: the wizard's offer, read ONCE. Cleared here rather than by a second
    // command, so the prompt cannot be shown twice by a page that polled twice.
    let offer = crate::onboarding::offer_marker(&cs.data_dir);
    let offer_inference = offer.is_file();
    if offer_inference { let _ = std::fs::remove_file(&offer); }
```

and `"offer_inference": offer_inference,` in the returned `json!`.

Register all five in `app/src/main.rs`: add `commands::inference_status, commands::install_inference_file, commands::install_inference_download, commands::remove_inference_model` to `run_console`'s `generate_handler!`, and `onboarding::pick_file` to **both** builders' handler lists (the wizard uses it too, for the local-file path after setup).

- [ ] **Step 6: The markup**

In `app/static/index.html`, after the `set-updates` row and before `set-diag`:

```html
  <div class="set-row" id="set-judge"><span class="k">Local judgment</span><span class="meta" id="set-judge-state"></span><button class="b" id="set-judge-download">Download</button><button class="b" id="set-judge-file">Install from a file&hellip;</button><button class="b" id="set-judge-remove">Remove model</button></div>
```

and replace the finish panel's two `<p class="meta">` lines with:

```html
    <p class="meta">Nothing is fetched now. Your first slot runs on the clock.</p>
    <p class="meta">Knowlu can estimate effort and importance for new assignments on this machine, with no account and nothing sent anywhere. It needs a one-off download of about 2 GB, which you start yourself in Settings.</p>
    <label><input type="checkbox" id="wiz-judge"> Show me how after setup</label>
    <p class="meta">Gmail proposals and event verdicts are not here yet — they arrive in a later release.</p>
```

- [ ] **Step 7: The page code**

In `app/static/console.js`, add to `renderSettings`'s neighbourhood:

```javascript
  // Knowlu plan 3a Task 10: the Local judgment row. Four states and every one of them has words —
  // "not installed" is a normal state (spec §5.3), not an error, so it is never rendered red.
  function renderInference(s) {
    var el = EL("set-judge-state");
    // The Remove button is set on EVERY branch, including this one: leaving it in whatever
    // visibility the last successful render gave it would offer "Remove model" over a status the
    // page could not read.
    if (!s || !s.ok) { el.textContent = (s && s.error) || "unavailable"; EL("set-judge-remove").hidden = true; return; }
    if (!s.runtime) { el.textContent = "runtime not installed — assignments arrive unestimated"; }
    else if (!s.model) { el.textContent = "model not installed — assignments arrive unestimated"; }
    else { el.textContent = "ready — " + (Math.round((s.model_bytes / 1073741824) * 10) / 10) + " GB model"; }
    EL("set-judge-remove").hidden = !s.model;
  }
  function loadInference() {
    return invoke("inference_status", {}).then(renderInference).catch(function () {});
  }
  function installInference(kind, viaFile) {
    EL("set-judge-state").textContent = "installing " + kind + "\u2026 this can take a while";
    var step = viaFile
      ? invoke("pick_file", { title: kind === "model" ? "Choose the model file" : "Choose the runtime archive", extension: kind === "model" ? "gguf" : "zip" })
          .then(function (p) { if (!p || !p.path) { return null; } return invoke("install_inference_file", { kind: kind, path: p.path }); })
      : invoke("install_inference_download", { kind: kind });
    return step.then(function (r) {
      if (r && !r.ok) { EL("set-judge-state").textContent = r.error; return; }
      return loadInference();
    }).catch(function () { EL("set-judge-state").textContent = "that did not finish"; });
  }
```

Call `loadInference()` at the end of `openSettings`'s chain, and add to the `#settings` click handler, before its closing brace:

```javascript
    // The runtime first, then the model: the runtime is the prerequisite, and the row's own text
    // says which one is missing, so one button does the next thing rather than two doing halves.
    if (e.target.closest("#set-judge-download")) { loadInference().then(function () {
      var t = EL("set-judge-state").textContent;
      installInference(t.indexOf("runtime") === 0 ? "runtime" : "model", false);
    }); return; }
    if (e.target.closest("#set-judge-file")) { loadInference().then(function () {
      var t = EL("set-judge-state").textContent;
      installInference(t.indexOf("runtime") === 0 ? "runtime" : "model", true);
    }); return; }
    if (e.target.closest("#set-judge-remove")) {
      invoke("remove_inference_model", {}).then(loadInference).catch(function () {});
      return;
    }
```

In the wizard's plan builder, add `offer_inference: EL("wiz-judge").checked` to the object handed to `apply_profile_settings`. And in `openSettings`'s `settings_context` handler, carry the flag through, then in `bootConsole` — after the first `poll()` — add:

```javascript
    // The wizard's offer, honoured once: settings_context clears the marker as it reads it, so a
    // page that polls twice cannot open the panel twice.
    invoke("settings_context", {}).then(function (c) { if (c && c.offer_inference) { openSettings(); } }).catch(function () {});
```

- [ ] **Step 8: Run the page and command tests**

Run: `cd app; cargo test`
Expected: PASS, zero new warnings. Then the existing headless page check, which must still pass with a seventh row:

```powershell
python scripts\settings-check.py
```

- [ ] **Step 9: Commit**

```bash
git add app/src/commands.rs app/src/onboarding.rs app/src/main.rs app/static/index.html app/static/console.js app/tests/static_assets.rs app/tests/commands.rs
git commit -F .git/COMMIT_MSG_3A10
```

```
feat(settings): the Local judgment row, and the wizard's offer (plan 3a, Task 10)

A seventh settings row with a state for every outcome — runtime not installed, model not
installed, ready with the model's size — and two doors to install by: Download, which reads the
manifest, and Install from a file…, which is the working path until the site exists and the better
one on metered wifi. Both doors lead to the same rule (R-P3a-2): a runtime is executed only if its
SHA-256 is in the compiled-in SUPPORTED_RUNTIMES table, and a refusal prints the digest to report.
The commands marshal only — the manifest read, the capped download and the install are one function
in inference.rs, because chaining three network-and-disk steps is computation (M7). Remove model
frees the gigabytes and leaves the runtime.

Never automatic (spec §5.3): every one of the four commands is reached from a click, and no tick,
thread or slot calls any of them. The wizard OFFERS and does not install — a checkbox on the
finish panel drops a zero-byte marker that settings_context reads once and clears, so "Nothing is
fetched now" still holds. The marker is a file rather than a Settings field on purpose: Settings
derives Deserialize with no serde defaults, so a new field would make every existing settings.json
fail to parse and silently reset a friend's backup folder.

The wizard's "enrichment is not here yet" sentence is now false and is gone.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 11: Observing the starve — telling, from the journal, that the routine's step 3 has nothing left

D1: the routine is **starved, never switched**. When it has done nothing for a week, it is turned off once, by hand, at claude.ai/code/routines. This task builds the instrument that says when that week has passed. **It edits nothing** — not the routine, not its prompt, not its config, not a note.

**Files:**
- Create: `scripts/starvation.ps1`, `tests/starvation.rs`
- Read first: `src/journal.rs:143–160` (the record shape this reads), `docs/runners/cloud-routine-prompt.md` line 13 (the actor the routine writes as)

**Interfaces:**
- Consumes: `state/journal/*.jsonl` and `tasks/*.md`, **read-only**.
- Produces: `scripts\starvation.ps1 -Vault <path> [-Days <n>]`, printing a per-day table and one verdict line; exit 0 always.

- [ ] **Step 1: Write the failing test** — create `tests/starvation.rs`:

```rust
//! The starvation report is a PowerShell script, so this test drives it the way a person does and
//! asserts on what it prints. Read-only by construction: the fixture it runs against is a temp
//! directory this test builds and deletes.
use std::path::{Path, PathBuf};

fn vault(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qo-starve-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("state").join("journal")).unwrap();
    std::fs::create_dir_all(d.join("tasks")).unwrap();
    d
}

fn line(ts: &str, actor: &str, field: &str) -> String {
    format!(
        "{{\"actor\": \"{actor}\", \"device\": \"d\", \"evidence\": null, \"field\": \"{field}\", \"id\": \"task_0123456789\", \"new\": 2, \"old\": 1, \"op\": \"set\", \"path\": \"tasks/t.md\", \"run_id\": null, \"seq\": 1, \"ts\": \"{ts}\", \"via\": \"cloud-routine\"}}"
    )
}

fn run(vault: &Path, days: &str) -> (i32, String) {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-File", "scripts/starvation.ps1",
            "-Vault", &vault.to_string_lossy(),
            "-Days", days,
        ])
        .output()
        .expect("powershell");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

/// A day on which BOTH producers wrote is the state during the overlap; a day on which only the app
/// wrote is what "starved" looks like; the verdict counts back from today.
#[test]
fn the_report_separates_the_two_producers_and_counts_the_starved_days() {
    let v = vault("both");
    let today = jiff::Zoned::now().date();
    let day = |n: i64| today.checked_sub(jiff::Span::new().days(n)).unwrap().to_string();
    std::fs::write(
        v.join("state").join("journal").join(format!("{}.jsonl", day(9))),
        format!("{}\n{}\n", line(&format!("{}T13:00:00Z", day(9)), "agent:routine.enrich", "importance"),
                             line(&format!("{}T18:00:00Z", day(9)), "agent:knowlu.enrich", "importance")),
    ).unwrap();
    std::fs::write(
        v.join("state").join("journal").join(format!("{}.jsonl", day(1))),
        format!("{}\n", line(&format!("{}T18:00:00Z", day(1)), "agent:knowlu.enrich", "effort_hours")),
    ).unwrap();

    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    // **Not `contains("routine")` / `contains("knowlu")`** — the table's own header line carries
    // both words whatever the journal holds, so those asserted nothing (M4). Assert the ROWS.
    let row = |day: String, r: &str, k: &str| {
        let want = format!("{day}   {r:>7}  {k:>6}");
        assert!(out.contains(&want), "expected row {want:?} in:\n{out}");
    };
    row(day(9), "1", "1");
    row(day(1), "0", "1");
    row(day(0), "0", "0");
    // The most recent routine write was nine days ago, so the verdict counts from there.
    assert!(out.contains("9 day(s) ago"), "the verdict must count from the last routine write: {out}");
    // `contains("starved")` is satisfied by NOT STARVED (M4). The verdict token carries its colon.
    assert!(out.contains("STARVED: seven or more days"), "{out}");
    assert!(!out.contains("NOT STARVED"), "{out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A routine write today is the opposite verdict, and it must be unambiguous — turning the routine
/// off while it is still doing work is the mistake this report exists to prevent.
#[test]
fn a_routine_write_today_says_do_not_turn_it_off_yet() {
    let v = vault("busy");
    let today = jiff::Zoned::now().date().to_string();
    std::fs::write(
        v.join("state").join("journal").join(format!("{today}.jsonl")),
        format!("{}\n", line(&format!("{today}T13:00:00Z"), "agent:routine.enrich", "course")),
    ).unwrap();
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("NOT STARVED"), "{out}");
    assert!(!out.contains("STARVED: seven"), "{out}");
    assert!(out.contains("0 day(s) ago"), "a write today is zero days ago: {out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// The queue itself is part of the picture: nothing flagged means neither producer has anything to
/// do, which is not the same as the routine having been beaten to it.
#[test]
fn the_report_counts_what_is_still_flagged() {
    let v = vault("queue");
    std::fs::write(
        v.join("tasks").join("a.md"),
        "---\ntitle: A\nstatus: active\nneeds_enrichment: true\nid: task_0000000001\n---\n\nB.\n",
    ).unwrap();
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 task(s) still flagged needs_enrichment: true"), "{out}");
    assert!(out.contains("the routine has never enriched anything"), "an empty journal is its own verdict: {out}");
    let _ = std::fs::remove_dir_all(&v);
}

/// A vault with no journal at all is a report, not a crash.
#[test]
fn an_empty_vault_reports_rather_than_failing() {
    let v = vault("empty");
    let (code, out) = run(&v, "14");
    assert_eq!(code, 0, "{out}");
    assert!(!out.trim().is_empty());
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run (repo root): `cargo test --test starvation`
Expected: FAIL — powershell reports the script does not exist.

- [ ] **Step 3: Write the script**

Create `scripts/starvation.ps1`:

```powershell
<#
.SYNOPSIS
    Is the cloud routine's enrichment step starved yet? Read-only.

.DESCRIPTION
    Knowlu spec §5.1: the app does the routine's work first, at its own slots, and the routine finds
    nothing left each time it wakes. When it has done nothing for a week, it is turned off ONCE, by
    hand, at claude.ai/code/routines. This script is the instrument that says when that week has
    passed - and, just as importantly, when it has NOT.

    It reads two things and writes nothing:

      - state/journal/*.jsonl, for `op: set` records on the enrichment fields, split by actor:
        `agent:routine.enrich` (the cloud routine's step 3) against `agent:knowlu.enrich` (this
        app's judge step). Both producers running is the expected state for weeks.
      - tasks/*.md, for how many notes still carry `needs_enrichment: true` - because "the routine
        wrote nothing" means one thing when the queue is empty and quite another when it is not.

    THIS SCRIPT EDITS NOTHING. Not the routine, not its prompt, not its config, not a note. Turning
    the routine off is a deliberate act by Quinn in a browser; this only says whether the evidence
    supports it. The one thing it can be wrong about is a routine that is failing rather than
    starved, which is why it also prints the last date the routine wrote ANYTHING.

    PowerShell 5.1: no &&, no ||, no ternary, no ??.

.PARAMETER Vault
    The vault to read. Required - there is no default, because the interesting answer is about a
    specific vault and a wrong guess would read the wrong one.

.PARAMETER Days
    How many days back to tabulate. Default 14.

.EXAMPLE
    .\scripts\starvation.ps1 -Vault .
    .\scripts\starvation.ps1 -Vault <path-to-a-vault> -Days 21
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Vault,
  [int]$Days = 14
)
$ErrorActionPreference = "Stop"

$ENRICHMENT_FIELDS = @("course", "effort_hours", "effort_confidence", "importance", "importance_reason", "needs_enrichment")
$ROUTINE = "agent:routine.enrich"
$KNOWLU  = "agent:knowlu.enrich"

if (-not (Test-Path $Vault)) { Write-Output "no vault at $Vault"; exit 0 }
$journal = Join-Path $Vault "state\journal"

# day -> @{ routine = n; knowlu = n }
$byDay = @{}
$lastRoutine = $null
$lastKnowlu = $null
if (Test-Path $journal) {
  foreach ($file in Get-ChildItem $journal -Filter "*.jsonl" -File) {
    foreach ($line in Get-Content $file.FullName) {
      if ($line.Trim() -eq "") { continue }
      $rec = $null
      # A torn or hand-edited line is skipped, never fatal: this is a report.
      try { $rec = $line | ConvertFrom-Json } catch { continue }
      if ($rec.op -ne "set") { continue }
      if ($ENRICHMENT_FIELDS -notcontains $rec.field) { continue }
      $actor = [string]$rec.actor
      if (($actor -ne $ROUTINE) -and ($actor -ne $KNOWLU)) { continue }
      $day = ([string]$rec.ts).Substring(0, 10)
      if (-not $byDay.ContainsKey($day)) { $byDay[$day] = @{ routine = 0; knowlu = 0 } }
      if ($actor -eq $ROUTINE) {
        $byDay[$day].routine += 1
        if (($null -eq $lastRoutine) -or ($day -gt $lastRoutine)) { $lastRoutine = $day }
      } else {
        $byDay[$day].knowlu += 1
        if (($null -eq $lastKnowlu) -or ($day -gt $lastKnowlu)) { $lastKnowlu = $day }
      }
    }
  }
}

$today = (Get-Date).Date
Write-Output "day          routine  knowlu"
for ($i = $Days - 1; $i -ge 0; $i--) {
  $day = $today.AddDays(-$i).ToString("yyyy-MM-dd")
  $r = 0; $k = 0
  if ($byDay.ContainsKey($day)) { $r = $byDay[$day].routine; $k = $byDay[$day].knowlu }
  Write-Output ("{0}   {1,7}  {2,6}" -f $day, $r, $k)
}

# Still owed. A queue of zero means neither producer had anything to do, which is NOT evidence that
# the app beat the routine to it.
$flagged = 0
$tasks = Join-Path $Vault "tasks"
if (Test-Path $tasks) {
  foreach ($note in Get-ChildItem $tasks -Filter "*.md" -File) {
    $head = Get-Content $note.FullName -TotalCount 40
    if ($head -match "^needs_enrichment:\s*true\s*$") { $flagged += 1 }
  }
}
Write-Output ""
Write-Output ("{0} task(s) still flagged needs_enrichment: true" -f $flagged)
if ($null -ne $lastKnowlu) { Write-Output ("knowlu last enriched on {0}" -f $lastKnowlu) } else { Write-Output "knowlu has never enriched anything here" }

if ($null -eq $lastRoutine) {
  Write-Output "the routine has never enriched anything in this journal - starved, or it was never running against this vault"
  exit 0
}
$gap = [int]($today - [datetime]::ParseExact($lastRoutine, "yyyy-MM-dd", $null)).TotalDays
Write-Output ("the routine last enriched on {0}, {1} day(s) ago" -f $lastRoutine, $gap)
if ($gap -ge 7) {
  Write-Output "STARVED: seven or more days with no routine enrichment. Spec §5.1 says the routine is now turned off ONCE, by hand, at claude.ai/code/routines - never by editing its prompt or its config."
  Write-Output "Before doing it, check state/runner-log.md: a routine that is FAILING looks exactly like one that is starved from here."
} else {
  Write-Output "NOT STARVED: the routine is still doing enrichment work. Leave it alone - turning it off now would drop judgments nobody else is making yet."
}
exit 0
```

- [ ] **Step 4: Run the tests**

Run (repo root): `cargo test --test starvation`
Expected: PASS.

- [ ] **Step 5: Run it once, read-only, against the live vault**

The global constraint forbids **writing** the live vault and forbids an interactive launch of `knowlu.exe`; a read-only report is neither, and this is the one place in the plan the live vault is read:

```powershell
.\scripts\starvation.ps1 -Vault .
```

Expected: the fourteen-day table, the flagged count, and a `NOT STARVED` verdict — the routine is still running and this plan changes nothing about that. Record the output in the task's review note; it is the baseline the weeks after this plan are measured against.

- [ ] **Step 6: Commit**

```bash
git add scripts/starvation.ps1 tests/starvation.rs
git commit -F .git/COMMIT_MSG_3A11
```

```
feat(scripts): starvation.ps1 — is the routine's step 3 finding anything? (plan 3a, Task 11)

Read-only. Tabulates `op: set` records on the six enrichment fields by day, split between
agent:routine.enrich and agent:knowlu.enrich, counts what is still flagged, and gives one verdict:
STARVED at seven or more days with no routine enrichment, NOT STARVED otherwise. Both producers
running is the expected state for weeks (spec §5.1), and this is the instrument that says when the
week has passed.

It edits nothing — not the routine, not its prompt, not its config, not a note; turning the
routine off stays a deliberate act by Quinn in a browser. The STARVED verdict says so, and warns
that a FAILING routine looks identical from here, pointing at state/runner-log.md.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

### Task 12: Close — the docs, the spec amendments, and this plan's status line

**Files:**
- Modify: `app/README.md`, `docs/surface/anatomy.md`, `docs/HANDOFF.md`, `CLAUDE.md`, `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md`, this plan
- **All six are CRLF.** Before editing any of them, check and preserve: `tr -cd '\r' < <f> | wc -c` must equal `wc -l < <f>` before and after. Never `sed -i` (it strips CR), never `grep -c $'\r'` (it counts every line on this machine).

**Interfaces:**
- Consumes: everything Tasks 1–11 produced.
- Produces: no code.

- [ ] **Step 1: The spec's three amendments**

In `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md`, append to **§5.2**, after the paragraph ending "…exactly the loop the routine's judgments follow today":

```
> **Amended 2026-09-07 (plan 3a, Task 2).** The actor names above — `knowlu/enrich`,
> `knowlu/events`, `knowlu/gmail` — are a defect and are replaced by **`agent:knowlu.enrich`,
> `agent:knowlu.events`, `agent:knowlu.gmail`**. `provenance::is_agent` is
> `actor.starts_with("agent:")` and nothing else, so an actor spelled `knowlu/enrich` is not an
> agent: `write_literals` would skip judge-once entirely and write no `judgment:` block, and a
> field Quinn had set by hand would be silently overwritten by the model. The new names mirror the
> routine's own `agent:routine.enrich`, which is what the journal already holds.
```

Append to **§5.3**, after the amendment block that ends "…to accommodate one dependency":

```
> **Amended again 2026-09-07 (plan 3a, Tasks 3 and 8).** The process boundary stands; the
> **bundling does not** — for three reasons, and **none of them is installer size**: the runtime is
> **optional** (the free tier runs with no model at all, so bundling makes every install pay for
> something most will never enable), it **versions independently** (llama.cpp moves far faster than
> this app, and coupling its version to our installer means an app release to pick up a runtime
> fix), and it is **consistent with the models**, which this section already downloads after install
> for the same reasons. This document's own figure for the runtime's size is the product plan's
> **+20–50 MB** (`docs/superpowers/notes/2026-09-01-product-and-business-plan.md:235`); the measured
> size of the asset plan 3a's spike obtained is recorded in
> `docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md`.
>
> So the runtime is **downloaded, or installed from a file the user points at, into
> `<app data root>\runtime\`** — the same shape as the models, and the shape the spike report
> already anticipated ("a downloaded executable that must be verified before it is run"). **There is
> no unverified path to executing it:** the app compiles in a table of supported upstream release
> digests (release tag, asset name, SHA-256), both install paths check the file's SHA-256 against
> that table, and an unrecognised digest is refused with the computed digest printed so it can be
> reported and added deliberately. The manifest URL is a convenience, not the root of trust.
> `app/tauri.conf.json`'s `externalBin` list is unchanged and still carries only the engine.
> `runtime::resolve`'s third branch is a sibling `llama-server.exe` beside the exe, so bundling later
> needs no code change.
>
> The **models** live beside it, in `<app data root>\models\` rather than per profile: a `.gguf` is
> an immutable artefact identical for every profile, and a friend with two vaults must not download
> two gigabytes twice. The **judgment logs stay per profile** (§5.4), because two vaults are two
> different sets of judgments.
```

And in **§10**, replace decision 12's cell with:

```
| 12 | llama.cpp as a **separate process**, downloaded and hash-verified into app data, not bundled and not linked (amended 2026-09-07 twice — see §5.3 and the spike report; was "via Rust binding", then "a second Tauri sidecar"); models in app data from R2; the app runs with no model | this doc §5.3 |
```

- [ ] **Step 2: `CLAUDE.md` — the determinism sentence needs its carve-out**

The **Architecture invariants** section says:

```
- The Python engine (`engine/`) is **deterministic** — no inference, same input → same order.
  Judgment (effort, importance, course attribution) happens in Claude — at note creation or
  in the cloud routine's enrichment step — never inside the engine.
```

Replace with:

```
- The Python engine (`engine/`) is **deterministic** — no inference, same input → same order.
  Judgment (effort, importance, course attribution) happens outside the ranking path — at note
  creation, in the cloud routine's enrichment step, or (since Knowlu plan 3a) in the Rust crate's
  **own `judge` module, reached only by `quinn-ops judge`**. **`rank` never calls a model**
  (Knowlu spec decision 11): the judge writes fields into notes, `rank` reads them, and the two are
  separate commands and separate slot steps. `src/judge.rs` is pure — the model is behind a trait
  and the process lives in `src/runtime.rs` — so nothing under `cli.rs` can reach one.
```

Add to **Knowlu (the console)**, after the paragraph about `engine.write` and `console_ctx()`:

```
- **Local judgment (plan 3a).** `quinn-ops judge` enriches tasks flagged `needs_enrichment: true`
  through `src/judge.rs`'s three tiers — heuristics, a promoted-rule seam (3b's, empty today), then
  llama.cpp on loopback via `src/runtime.rs`. It writes as **`agent:knowlu.enrich`** (the `agent:`
  prefix is load-bearing: `provenance::is_agent` is a `starts_with` test) with `judged: true` and
  `propose: true`, so judge-once holds and a field Quinn set comes back as a `kind: amend` card.
  **It always exits 0** — no runtime and no model are normal outcomes, never a failed slot and
  never an amber tray. The runtime and the model are downloaded or installed from a file into
  `%LOCALAPPDATA%\knowlu\{runtime,models}\` and **never automatically**; judgment logs are per
  profile, hold ids and field values only, and never enter the vault or the backup. The slot is
  coursework → ingest → **judge** → rank.
- **There is no unverified path to executing a runtime** (ruling R-P3a-2). `app/src/inference.rs`'s
  `SUPPORTED_RUNTIMES` is a compiled-in table of upstream release tag, asset name and SHA-256;
  **both** install paths check against it and an unrecognised digest is refused with the computed
  digest printed. **Adding a supported release is a code change and an app release**, deliberately —
  never a file dropped in a folder. A manifest is a convenience for finding bytes, not the root of
  trust. Model files are *not* pinned: a `.gguf` is data, parsed by a runtime the table has already
  vouched for. The runtime is **not bundled** because it is optional, versions far faster than this
  app, and is the same kind of artefact as the models — **not** because of installer size (R-P3a-1;
  the repo's figure is the product plan's +20–50 MB).
- **The cloud routine is being STARVED, not switched off** (spec §5.1). Both producers run at once
  and that is expected; `scripts\starvation.ps1` is the read-only instrument that says when a week
  has passed with the routine finding nothing. **Do not edit the routine's prompt or config.**
```

- [ ] **Step 3: `app/README.md`**

Add a section after the one describing the scheduler:

```markdown
## Local judgment

The slot runs `quinn-ops judge` between `ingest` and `rank`. It reads every task note flagged
`needs_enrichment: true` and answers effort, importance and course in three tiers:

1. **Heuristics** — the course map's uid pins and course codes (the same rule `ingest` applies), and
   a vendor's own effort estimate. Deterministic.
2. **Promoted rules** — the seam plan 3b fills. Empty today.
3. **The model** — llama.cpp's `llama-server`, on `127.0.0.1`, started once per slot and killed at
   the end of it, answering a five-field JSON object under a GBNF grammar.

**It runs with nothing installed.** "runtime not installed" and "model not installed" are normal
outcomes: the step is recorded at exit code 0 with a name that says which half is missing, whatever
tier 1 answered is still written, and the flag stays set so the work is still owed. Nothing is ever
downloaded automatically — Settings → *Local judgment* has a *Download* button and an *Install from
a file…* button, and the wizard offers a reminder and nothing more.

**Knowlu only runs llama.cpp builds it knows.** `inference::SUPPORTED_RUNTIMES` is a table of
upstream release tags, asset names and SHA-256s compiled into the app, and **both** install buttons
check the file against it before anything is written — so the path that works before the download
site exists is as verified as the one that comes later. An unrecognised digest is refused and the
message prints it, which is what to send us if you have a build we should add. Model files are not
pinned: a `.gguf` is data parsed by a runtime that table already vouched for, and any 1–4 B Q4 model
is meant to work.

Every write is `agent:knowlu.enrich` through `quinn_ops::write` with `judged: true` and
`propose: true`, so a field you set in the console is never overwritten: the app's later opinion
arrives as a `kind: amend` card in the deck.

Where things live: `%LOCALAPPDATA%\knowlu\runtime\` and `…\models\` (install-wide — one download
serves every profile), `…\profiles\<id>\judgments\YYYY-MM-DD.jsonl` (per profile, ids and field
values only, never in the vault and never in the backup).
```

- [ ] **Step 4: `docs/surface/anatomy.md`**

Add a row to the constraints table in §1:

```markdown
| **`rank` never calls a model** (Knowlu spec decision 11) | Judgment is `quinn-ops judge`, a separate command and a separate slot step, writing fields into notes before `rank` reads them. Nothing the read model or the renderer touches can reach a model, a socket or a subprocess. |
```

**The panel table in §2 says six rows and there are now seven.** Replace its Settings row:

```markdown
| **Settings** | seven rows over the live page — profile name, vault path, backup folder, start with Windows, updates, local judgment, diagnostics | the topline gear, or *Settings* in the tray (which `eval`s `window.KNOWLU_OPEN_SETTINGS`) |
```

and add a paragraph at the end of the settings-panel section:

```markdown
**Local judgment** is the seventh settings row (plan 3a). It has a state for every outcome —
`runtime not installed`, `model not installed`, `ready — 2.1 GB model` — and "not installed" is
rendered as an ordinary state, never as an error, because running with no model is the free tier's
zero-cloud default and not a fault. Its two install buttons are two doors to one rule: **a runtime
is executed only if its SHA-256 is in `inference::SUPPORTED_RUNTIMES`**, a table compiled into the
app, whichever door it came through (ruling R-P3a-2). A refusal names the digest, so the answer to
"why won't it take my file" is a string the reader can send us. *Left out:* a progress bar. A
two-gigabyte download would want one, and the row deliberately says `installing…` instead: the
command is `(async)`, the page has no channel to stream progress over, and inventing one for a
button pressed once per install is cost without a reader. *Also left out:* pinning model digests —
a `.gguf` is data parsed by a runtime the table already vouched for, and the spec allows any 1–4 B
Q4 model, so a pinned list would make the row useless to anyone who wants a different one.
```

- [ ] **Step 5: `docs/HANDOFF.md`**

Insert a block immediately above `▶ KNOWLU PLAN 4A DONE (2026-09-06)`:

```markdown
> **▶ KNOWLU PLAN 3A DONE (2026-09-07).** `docs/superpowers/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md`
> — **judgment comes home, first third**: twelve tasks, from
> `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md` §5. Plan 3 is split into
> **3a (this), 3b (event verdicts and rule promotion), 3c (Gmail, then the routine is turned off)**.
>
> **What shipped.** `src/judge.rs` — three tiers (heuristics, a promoted-rule seam for 3b, the
> model), pure, model behind a trait; `src/runtime.rs` — llama.cpp's `llama-server` on loopback,
> started once per slot, one call at a time, bounded, killed on drop; `src/judgelog.rs` — ids and
> field values only, into the profile's app data; `src/enrich.rs` + `quinn-ops judge` — the cloud
> routine's step 3, run locally; `app/src/inference.rs` + a settings row — download or install from
> a file, hash-verified, never automatic. The slot is now **coursework → ingest → judge → rank**.
>
> **`src/write.rs` grew the re-proposal path the port had left as `WriteError::ProposeNotPorted`**:
> `propose_amendment` is ported from `engine/write.py:333–353`, and
> `scripts/diff-engines-notes.ps1` gained the `--propose` pair its header used to say was excluded
> by decision. Both dual-run scripts are clean.
>
> **Three spec deviations, all amended into the spec by Task 12:** the actors are
> `agent:knowlu.enrich` and not `knowlu/enrich` (`provenance::is_agent` is a `starts_with("agent:")`
> test, so the spec's spelling would have skipped judge-once); the runtime is downloaded and
> digest-checked rather than bundled as an `externalBin` sidecar (ruling R-P3a-1 — because it is
> optional, versions independently, and is the same kind of artefact as the models, **not** because
> of installer size); and the models are install-wide rather than per profile, while the judgment
> logs stay per profile. **There is no unverified path to executing the runtime** (R-P3a-2): its
> SHA-256 must be in `inference::SUPPORTED_RUNTIMES`, which Task 1's measurement seeded.
>
> **The routine is untouched and is being STARVED** (spec §5.1). Both producers run at once, which
> is expected. `scripts\starvation.ps1 -Vault .` is the read-only instrument: when it says
> **STARVED** — seven or more days with no `agent:routine.enrich` write — the routine is turned off
> ONCE, by hand, at claude.ai/code/routines, and never by editing its prompt or its config. Check
> `state/runner-log.md` first: a FAILING routine looks identical from there.
>
> **Owed, and Quinn's:** the Cloudflare account (spec §11), without which
> `https://knowlu.com/inference/manifest.json` does not exist and *Download* has nothing to read —
> *Install from a file…* is the working path meanwhile, and it is verified by the same table.
> **If the spike's `Outcome:` was C**, add a sentence here saying that `inference::SUPPORTED_RUNTIMES`
> is **empty**, that no runtime can therefore be installed by either door until a measured digest is
> added, and that obtaining one is the first thing the next session does. **Next:** plan 3b (event
> verdicts, embeddings, rule promotion), then 3c (Gmail), then the routine goes off and the remote
> comes out.
>
> `$mode` is still `python-live`; nothing here moved G2 or touched `engine/`.
```

- [ ] **Step 6: This plan's status line**

Change the plan's own status line from `**Status: NOT STARTED.**` to:

```
**Status: EXECUTED 2026-09-07 on branch worktree-knowlu-plan-3a (Tasks 1–12).** Written 2026-09-07.
```

adjusting the branch name to the one actually used, and noting any task whose outcome differed from the plan (in particular Task 1's `Outcome:` letter, which everything downstream was written against).

- [ ] **Step 7: Verify the line endings survived**

```bash
for f in CLAUDE.md docs/HANDOFF.md app/README.md docs/surface/anatomy.md \
         docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md \
         docs/superpowers/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md; do
  echo "$f lines=$(wc -l < "$f") cr=$(tr -cd '\r' < "$f" | wc -c)"
done
git diff --stat
```

Expected: `lines` equals `cr` for every file, and `git diff --stat` shows a handful of changed lines per file — **never a whole-file flip**.

- [ ] **Step 8: The full gate, one last time**

```bash
cargo test 2>&1 | tail -20
cd app && cargo test 2>&1 | tail -20
```

```powershell
.\scripts\diff-engines.ps1
.\scripts\diff-engines-notes.ps1
```

Expected: root `cargo test` green at **0 warnings** with `tests/oracle.rs` and `tests/surface_oracle.rs` passing; `app` green at zero new warnings; both dual-run scripts exit 0. Confirm the eight Python-written references and the three `surface-today-*.json` are untouched:

```bash
git log --oneline --name-only -20 -- tests/fixtures/ | head -20
```

Expected: nothing under `tests/fixtures/` in any of this plan's commits.

**Two closing checks the rulings require. Both fail the task loudly rather than quietly.**

**(a) R-P3a-1 — the measured size exists.** The only size figures this plan is allowed to state are the product plan's `+20–50 MB` and the byte size Task 1 measured. Confirm the second is on record:

```bash
grep -n "bytes\|MiB\|MB" docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md | head
grep -rn "100–200\|100-200\|one to two hundred" docs/ CLAUDE.md app/README.md
```

Expected: the spike report states the asset's byte size, **or** its `Outcome:` line is C, in which case the only figure anywhere is the product plan's and Task 12 step 1's amendment says so. The second grep must return **nothing** — if any unsupported size claim survived into `docs/`, `CLAUDE.md` or `app/README.md`, strike it before committing.

**(b) R-P3a-2 — the digest table is not empty by accident.**

```bash
grep -n "SupportedRuntime {" app/src/inference.rs
grep -n "^Outcome:" docs/superpowers/reports/2026-09-07-sidecar-protocol-spike.md
```

Expected: **at least one `SupportedRuntime { … }` entry**, seeded from the spike, *unless* the spike's `Outcome:` line is C — in which case the empty table is the honest state and the HANDOFF block must say, in words, that **no runtime can be installed until a measured digest is added**, and name that as the first thing the next session does. An empty table with an Outcome A or B report is a defect: the spike measured a real archive and its digest was not carried across.

- [ ] **Step 9: Commit**

```bash
git add CLAUDE.md docs/HANDOFF.md app/README.md docs/surface/anatomy.md \
        docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md \
        docs/superpowers/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md
git commit -F .git/COMMIT_MSG_3A12
```

```
docs: close Knowlu plan 3a — the judge, the runtime, enrichment (plan 3a, Task 12)

CLAUDE.md's "the engine is deterministic — no inference" sentence gets its carve-out: judgment is
now also the Rust crate's own judge module, reached only by `quinn-ops judge`, and rank still never
calls a model. A Knowlu section records the actor, the exit-0 rule, where the runtime and the model
live, and that the routine is being starved rather than edited.

Three spec amendments, each with its reason: §5.2's actor names (agent:knowlu.enrich, because
provenance::is_agent is a starts_with test), §5.3's bundling (downloaded and digest-checked against
a compiled-in table of supported upstream releases, because the runtime is optional, versions
independently and is the same kind of artefact as the models - not because of size, R-P3a-1/2) and
the model location (install-wide; the judgment logs stay per profile). Decision 12 restated.

app/README.md, docs/surface/anatomy.md and docs/HANDOFF.md carry the same three facts for their own
readers, and HANDOFF names what is owed: the Cloudflare account, without which Download has no
manifest to read.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_014MQESyCz34TjYypAojCJh4
```

---

## Self-review

Run after Task 12, and recorded here because the reviewer should see the same three lists the writer did.

**1. Spec coverage — §5, §8 and decisions 4, 11, 12.**

| Spec | Task |
|---|---|
| §5.1 starve, never switch | Tasks 7, 9 (the app does the work first), 11 (the instrument), 12 (the HANDOFF instruction). No task edits the routine. |
| §5.2 one seam, three tiers | Task 2. Judged writes by an agent actor: Tasks 6, 7. `rank` never calls a model: no task touches `cli.rs`; Task 9's step order. |
| §5.3 the runtime, the models, "runs with no model" | Tasks 1, 3, 4 (the process), 8 (acquisition **and the digest table**, R-P3a-2), 9 (the skip), 10 (the settings action and the onboarding offer). |
| §5.4 the judgment log; rule promotion | Task 5 (the log, per profile, ids only). Promotion itself is **3b's** and is deliberately left as `trait Rules` + `NoRules` (Task 2) — D10. |
| §5.5 enrichment first | Task 7. Event verdicts and Gmail are named as 3b/3c and not built. |
| §5.6 data minimisation | Task 5's `Entry` (no field can hold content) and its `no_note_text_ever_reaches_a_log_line` test; Task 7's `inputs` is `{source_uid, title_seen}`. |
| §8 test seams | The model is a trait; every test drives `Scripted`/`Fixed`/`Flaky`. One `#[ignore]`d smoke test names its reason in the attribute (Task 3). No test downloads, starts a real server or leaves loopback. |
| Decision 4 (local models, never a cloud API) | Task 3 binds `127.0.0.1`; no task adds a key, a token or a remote endpoint. |
| Decision 11 | D2, D3, D4 in the ledger. |
| Decision 12 | D6, D7, D8, and Task 12's amendment where the code and the spec parted. |

**Gap, stated rather than papered over:** the routine's step 3 also appends a `course_map` pin to `config/ingest.yaml`. Plan 3a does not, because promotion is §5.4's loop and that is 3b's (D10). Nothing is lost while the routine still runs, and Task 12's HANDOFF block says so.

**2. Placeholder scan.** No "TBD", no "similar to Task N", no "add appropriate error handling", no step that describes without showing. Every code step carries the code. The three places the plan branches carry every branch in full: Task 1's outcome (**step 8 now enumerates the four tasks Outcome B reaches**, and step 9 carries the whole replacement); the spike's flag spellings, which Task 3 step 1 names one by one with the file the replacement comes from; and `SUPPORTED_RUNTIMES`, whose single entry is **measured by Task 1 and pasted in Task 8 step 5** — the one value in the plan that cannot be written before execution, with its empty-table case (Outcome C) given an explicit, checked behaviour in Task 8, Task 12 step 8(b) and the HANDOFF block rather than being left to discover.

**3. Type consistency.** Checked across tasks: `judge::Model::judge(&self, &Item, &Heuristics, &Verdict) -> Result<Verdict, ModelError>` is the signature in Task 2, implemented in Task 4, faked in Tasks 2 and 7. `Outcome::verdict()`/`label()` are defined in Task 2 and used in Tasks 5 and 7. `Missing::{Runtime, Model}` is produced by `runtime::resolve` (Task 3) and consumed by `judge_task` (Task 2) and `enrich_with` (Task 7). `judge::{GRAMMAR, prompt_for, parse_reply}` are Task 4's, and **the only test that names them now lives in Task 4** — `#[ignore]` suppresses running, never compiling, so a test written against a later task's symbols breaks the build at the end of its own task. `slot_argv(vault, exe, Option<&JudgeArgs>)` and `judge_state_in(root, cs)` are defined in Task 9, and its only other caller, `app/tests/scheduler.rs`, is updated in the same task. `inference::{runtime_exe, model_file, judgments_dir}` are produced in Task 8 and consumed in Task 9 — **which is why the two are in that order**; the brief's suggested order had them the other way round and would have made Task 9 uncompilable. `install_runtime_from_zip(root, src)` takes **two** arguments (the digest comes from the table, not the caller) and `download_to(url, dest, Option<&str>, cap)` takes an optional one; both are used at their new arities in Tasks 8 and 10. `RUNTIME_EXE` is defined once (Task 3) and reached through the constant on the app side (Task 8), so Outcome B costs those tests nothing. `write::propose_amendment`'s `changes: &[(String, Value, Value)]` matches the `proposed` vector Task 6 also re-types.

**Deviations from the brief's suggested shape, and why:** twelve tasks rather than eleven. Task 6 is new — `write::propose_amendment` is unported and enrichment cannot honour judge-once without it. Tasks 8 and 9 are the brief's items 8 and 6 swapped into dependency order.

**Revision 1 (2026-09-07)** applied an independent review's five blocking, nine should-fix and ten minor findings, with two controller rulings: **R-P3a-1** (the unsupported "100–200 MB" figure is struck everywhere; the deviation stands on optionality, independent versioning and consistency with the models, and the only figures allowed are the product plan's +20–50 MB and Task 1's measurement) and **R-P3a-2** (a compiled-in table of supported upstream release digests, checked on both install paths, so there is no unverified path to executing a runtime binary).






