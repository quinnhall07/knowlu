# Local model runtime removal (lane L6): build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status: DRAFT, review rounds 1 and 2 answered (§11); round 3 passed with three minors, closed in place (§11).** Nothing runs until a plan-mode `reviewer` round has passed and the
controller has answers to §4's open questions, or has deferred them. Review findings go to
`docs/reports/2026-10-19-l6-runtime-removal-plan-review.md`, and their answers go to a fidelity-notes
section at the end of this file.

**Date:** 2026-10-19 (session 5, the friends-MVP push). **Lane:** L6 of
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` §5 ("ruling 10: remove
engine/src/runtime.rs, app/src/inference.rs, wizard model download, their tests (incl. the ignored
real-runtime smoke test, by deletion with the module, not by un-ignoring)"; owns `main.rs` and
`commands.rs` while it runs).

**Authority.** The cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`), Amendment 2026-09-29,
ruling 10, Pilot: "removal of the local model runtime (`runtime.rs`, `inference.rs`,
`SUPPORTED_RUNTIMES`, the wizard offer)". The §12 C4 row lists the same pieces plus "the settings row".
Ruling 10 spreads C4's rows over the stages and puts runtime removal in the Pilot; the 2026-10-18
handoff moves it into wave 1 of the friends MVP. There is no separate spec: the ruling names what goes,
and this plan names how. `HANDOFF.md` §3 (Pilot) repeats the ruling: "Until then they stay and are not
extended."

**Goal.** Nothing model-shaped is left on the device. After this lane the app cannot download, install,
verify or run a llama.cpp runtime or a `.gguf` model, the engine cannot spawn one, and every judgment
goes through `judge::Model`'s one production implementation, `CloudModel`. A vault with no account
gets a named skip (`judge (skipped: no account)`), never a failure.

**Architecture.** Deletion, plus three small behaviour changes:
- **App scheduler.** `JudgePlan::Local`, `JudgeArgs`, `JudgeState` and `judge_state(_in)` go. A vault
  with no `config/cloud.yaml` gets `JudgePlan::Skip("judge (skipped: no account)")`, the wording `sync`
  already uses. `judgments_dir` moves out of `inference.rs` into `scheduler.rs`. The folder it names,
  `profiles\<id>\judgments`, does not change.
- **Engine `judge`.** The `--runtime` and `--model` flags go. A vault with no account takes the arm a
  missing session already takes: tier 1 is still written, one lead line names the reason, and the
  run exits 0.
- **Settings page.** The *Local judgment* row, its three handlers and its four Tauri commands go.

Nothing else moves. No vault byte, no journal record, no run-record shape, no cloud function and no
migration changes.

**Code read for this plan:** the main checkout at `C:\Users\danie\GitHub\knowlu`, which is `main`
before the email merge, and (round 2, read-only) the email-forwarding and p3-registrar worktrees under
`.claude\worktrees\`. The round-2 sweep grepped every tripwire token, plus `runtime`, `model`,
`inference`, `offer_marker`, `JudgeArgs`, `JudgePlan::`, `Options {` and `not installed`, across
`engine/`, `app/`, `cloud/`, `scripts/`, `.github/` and the docs in all three trees. Results:
- **The email branch** edits `app/src/main.rs`, `app/src/onboarding.rs`, `app/src/scheduler.rs`
  (a shift of about 88 lines below the judge region), `app/tests/scheduler.rs`, `app/tests/onboarding.rs`,
  `engine/src/enrich.rs` (a shift of about 6 lines from its tests on) and `engine/src/main.rs` (a shift
  of 9 to 13 lines). It adds `engine/tests/mail_pass.rs`, which builds an `enrich::Options` literal
  (§3.4), and `app/src/mail/**`, which names no removed item (only `mail_offer_marker`).
- **p3-registrar (#19)** names no runtime, model or local-judge item beyond what `main` already has.
  Its base is older than `main`, so its hits are a subset.
- `cloud/**` has one hit in both trees, `judge_prompts.ts:4` (§9).

**Every `file:line` below is pre-merge `main` unless marked "EF" (email branch), so the implementer
re-confirms each one on the lane branch before editing.** Items are named by function, test name or
heading so they survive the line shift.

## 1. VISION and the two overriding rules

- **VISION, "The product":** "Judgment is a service … **Nothing model-shaped ships in the app.** Fetch
  on device, think in the cloud." This lane is what makes that sentence true of the code. Today the
  app ships a GBNF grammar, a local prompt builder, a llama.cpp process driver and a runtime
  installer.
- **VISION design commitment 1** ("`rank` never calls a model"): unchanged. `rank` never reached
  `runtime.rs`, and nothing in `rank`'s path is edited.
- **VISION standing rule "Failures are visible; silence is never ambiguous":** a vault with no account
  still gets a named judge step at exit 0, as it does today (`judge (skipped: no runtime)` becomes
  `judge (skipped: no account)`). A hand-run `knowlu-engine judge` on such a vault prints one lead line
  that says why.
- **VISION "Minimize the AI surface":** the lane only removes surface.
- **Rule 1 (no single-user assumptions).** Holds. The lane adds no path, name, school, account or
  credential. It removes an install-wide `runtime\` and `models\` layout that served every profile, and
  replaces no part of it.
- **Rule 2 (never regenerate a frozen reference).** Holds. No file under `engine/tests/fixtures/**` and
  no `surface-today-*.json` is opened for writing. `rank`, `surface` and the read model are untouched,
  so no reference can move. T7 checks with
  `git diff --exit-code main -- engine/tests/fixtures`.
- **Contract list.** No task edits a file on it. `engine/src/{judge,enrich,judgelog,cloudmodel,lib,main,runtime}.rs`,
  `app/src/{inference,scheduler,onboarding,commands,main,lib}.rs` and `app/Cargo.toml` are all off
  the list. `app/src/account.rs` is on it and is **not** touched, although one of its comments still
  names "the local runtime/model gate" (`account.rs:781`, EF `:818`). That comment goes to L4, which
  owns the file in this wave (§9). `engine/tests/mail_pass.rs` (EF), `engine/src/childproc.rs` and
  `app/src/grades.rs`, which round 2 added, are off the list too. One borderline file remains:
  `engine/tests/cloud_contract.rs`. T2 deletes two field lines from its `label_opts()` struct literal. L3's plan already asks the controller for this
  file's status (L3 Q-B), and §4 Q9 asks the same question here.
- **Privacy.** Nothing new is collected, stored or sent. The privacy page and `PRIVACY_VERSION` do not
  move. Removing a path that downloads and executes a third-party binary shrinks the attack surface
  (the threat notes in `inference.rs`'s module doc go with the file).
- **Money, releases, rules.** The lane touches none of them. Its PR merges on green CI and a `reviewer`
  PASS (handoff §1 item 7). The four `CLAUDE.md` sentences that name the runtime are rule text, so
  they are proposals for Quinn (§10) and are never edited by an agent.

## 2. Global constraints

- **Branch and worktree.** Branch `l6-runtime-removal`, worktree
  `C:\Users\danie\GitHub\knowlu\.claude\worktrees\l6-runtime-removal`. The controller creates both
  from `main` **after the email-forwarding merge**, never before (handoff §5, L6 "after merge"). Use
  absolute paths and never `cd`.
- **Agents never commit, push or branch.** Each task ends by handing the controller a file list and a
  commit message. The controller commits with `git add <paths>` (never `-A`) and `git commit -F <file>`,
  and uses a merge commit for the lane PR.
- **Gates.** `cargo build --workspace` and `cargo test --workspace` against the worktree, dev profile,
  in the foreground, and only under the build-slot guard, which allows two cargo runs machine-wide
  (handoff §5). The guard lives in the main checkout:
  `bash C:/Users/danie/GitHub/knowlu/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test --workspace --manifest-path C:/Users/danie/GitHub/knowlu/.claude/worktrees/l6-runtime-removal/Cargo.toml`
  (and the same with `cargo build`). Build the engine first (`cargo build -p knowlu-engine`), because of
  the zero-byte sidecar placeholder (`docs/reference/toolchain.md`). **0 warnings is part of green.**
  The one accepted line is the app's `.rsrc merge failure: multiple non-default manifests`. A deletion
  lane is where `dead_code` and `unused_imports` warnings appear, so every task's gate reads the warning
  tally, not only the pass count. Also run `scripts/ci/eol-check.ps1` and, after T4,
  `scripts/wizard-check.py` (a regression walk of the wizard; the settings panel is not in it).
- **The ignored-test count moves from 4 to 3.** `runtime.rs`'s
  `real_runtime_loads_a_model_and_answers_under_the_grammar` is **deleted with its module and never
  un-ignored** (handoff L6 row; `CLAUDE.md` "Gates": "None may be un-ignored by changing the
  assertion"). The other three (`events.rs` traps 4 and 5, `app/tests/scheduler.rs::run_slot_end_to_end`)
  are untouched. No CI line asserts the count (checked: `ci.yml`, `scripts/**`, `workflows.rs`), so only
  docs and `CLAUDE.md` carry the number.
- **Tests never leave the machine.** No test added here binds a socket. The CLI tests in T1 run the
  engine binary against a scratch vault with no `config/cloud.yaml`, so nothing reaches a network.
- **Real Credential Manager.** T3b deletes six `app/tests/onboarding.rs` marker tests. The two on the
  create path hold `CREDMAN_LOCK` through `PendingSession`. Deleting a test needs no sign-off. No task
  adds a test that touches the real store. T3b's `base_plan()` change touches callers that do hold the
  lock, but it does not change what they do with it.
- **LF everywhere**; `*.ps1` stay CRLF (relevant only if Q5 rewords `scripts/ci/tauri-cli.ps1`).
  `engine/tests/fixtures/**` is untouched.
- **No secret, account id, email, school or person's name** appears in code, tests, comments or test
  names.
- **TDD.** Every new behaviour is a failing test first, run and seen red for the stated reason. For a
  deletion, the "red" test is the tripwire of T1, which fails while any listed name or file still
  exists and goes green only when T5b lands. Tests that pinned removed behaviour are deleted in the
  same commit as their code, never edited to pass. A surviving test is edited only where §3.4
  "Rewritten" names the edit: a removed type, label or assertion about removed behaviour is taken out
  or respelled, and the test's own property stays (for example L0's mail-marker test, and
  `no_console.rs`'s spawn floor). An edit §3.4 does not name is a stop-and-report.
- **`commands.rs` computes nothing.** The lane only removes commands from it. It adds none and moves no
  logic into it.
- **Handler lists.** The console's `generate_handler!` loses exactly four names. The wizard's list
  loses none: `onboarding::pick_file` stays in **both** lists, because L5's syllabus Offer B uses it
  (`docs/specs/2026-10-05-syllabus-class-times-design.md`, the "Offer B" paragraph and task T8b,
  "`pick_file`'s optional extension list"). Recount both lists by script at T5a; never quote a count
  from this plan.

## 3. Inventory: what goes, what changes, what stays

Pre-merge `file:line` values; re-confirm on the lane branch.

### 3.1 Engine (`engine/`)

| Item | Where | Fate | Task |
|---|---|---|---|
| The whole module: `RUNTIME_EXE`, `CALL_TIMEOUT`, `resolve`, `kill_tree`, `read_capped`, `stderr_tail`, `wait_and_read`, `PerCall` and its `judge::Model` impl | `engine/src/runtime.rs` | deleted | T5b |
| `pub mod runtime;` and its comment; the `childproc` comment that names `runtime.rs`; the plan-3a comments over `judge`, `judgelog`, `enrich` | `engine/src/lib.rs:107-109`, `:140` | deleted / reworded | T5b |
| `Missing::Runtime`, `Missing::Model`; `Outcome::ModelNotInstalled`, `Outcome::RuntimeNotInstalled` with their `verdict()` and `label()` arms ("model not installed", "runtime not installed"); `judge_task`'s two `Err` arms | `engine/src/judge.rs:134-135`, `:149-150`, `:160-161`, `:169-171`, `:500-501` | deleted. `Missing::Service` and `Outcome::ServiceUnavailable` stay | T5b |
| `GRAMMAR` (the GBNF rule set) and `prompt_for` (the local prompt) | `judge.rs:559`, `:582` | deleted (Q3). The prompt and schema live server-side (CLAUDE.md, "Command and app contracts") | T5b |
| Doc text that describes the local path: the module doc's "then a local model" (`:3`), `Model`'s "`runtime::PerCall` and a scripted fake" (`:202`), `parse_reply`'s "Under outcome B … `llama-cli` echoes the prompt" (`:624`), `parse_reply`'s "which `GRAMMAR`'s `string` rule explicitly permits" (`:628`), and the test doc over the last-`{` anchor case, "`GRAMMAR`'s `string` rule …" (`:1208`). **Round 2 adds** the other docs and comments that describe the runtime: `:94` and `:131` ("the runtime's stderr" on `ModelFailed`), `:167` (`Missing`'s doc), `:201` ("the transport, the grammar and the reply format"), `:622` ("whatever the runtime wrote"), `:633` ("anything the runtime prints afterwards"), `:661` ("the runtime's 1 MiB stdout cap"), `:1142` ("the grammar bounds importance") and `:1219` ("Whatever the runtime prints after the completion") | `judge.rs` | reworded: the two `GRAMMAR` citations state the property itself (a string value may contain a `{` or `}`) without naming the deleted constant. The round-2 lines say "the model's reply" or "the service" where they said "the runtime", and "the schema" or "the clamps" where they said "the grammar"; no code line changes. `parse_reply` itself stays: `CloudModel` calls it (`cloudmodel.rs:459`) | T5b |
| `Options.runtime`, `Options.model`; `run`'s `runtime`/`model` parameters; `run_lines_with`'s `None` arm (`runtime::resolve` and `PerCall`) | `engine/src/enrich.rs:80-83`, `:487-507`, `:350-358` | fields and params deleted; the `None` arm becomes the named no-account skip (T2) | T2 |
| `run_lines`'s `Unavailable::NoConfig` arm, which hands `None` to the runtime arm | `enrich.rs:321-332` | joins the existing `Err(why)` arm: `judge: skipped (<why>)` as the lead line, tier 1 still written | T2 |
| Module and item docs about `llama-cli`, "no server, no load phase", `runtime::CALL_TIMEOUT` | `enrich.rs:8-13`, `:36-41`, `:63-64`, `:188-190`, `:340-344` | reworded to the cloud arm; `BATCH_BUDGET`'s doc cites `cloudmodel::CALL_TIMEOUT` | T2 |
| **Round 2:** the other runtime prose in `enrich.rs`'s non-test code: the `Options.runtime`/`model` field docs (`:80-81`, deleted with the fields); `enrich_with`'s comments "\"model not installed\" is exactly what answers …" (`:231`) and "runtime is broken … the runtime's stderr tail" (`:243-245`); `run_lines`'s doc "or, on a vault with no account, plan 3a's local runtime … the local branch below is plan 3a's and is removed by C4" (`:314-319`) and its comment "Not a fallback to the local runtime" (`:325-326`). **Round 3 adds** the R-C2-E41 comment over the `NoGmailScope` arm, "a step left out, exactly like `judge (skipped: no model)`" (`:583`, EF `:589`), which names a skip label T2 removes and which T2's whole-word grep does not catch | `enrich.rs` | reworded: tier 3 is the service, a vault with no account is the named skip, and a failed model call's text still reaches stdout. The `:583` comment cites `judge (skipped: no account)` instead; the rest of it stays. The property each comment explains stays | T2 |
| `Judge`'s `--runtime` and `--model` flags, their help, the command's "using the local model" help text, the dispatch, and the comment example `judge (skipped: no runtime)` | `engine/src/main.rs:150-169`, `:426`, `:558-563` | deleted / reworded; the clap parse test at `:726` is rewritten (T2) | T2 |
| `CALL_TIMEOUT`'s doc ("the same 120 seconds `runtime::CALL_TIMEOUT` gave … C4 removes `runtime.rs`") | `engine/src/cloudmodel.rs:68-70` | reworded; the constant and its value stay | T5b |
| A test's stderr literal `"llama-cli: panicked at …"`; the module doc's "a week of `model not installed` lines"; the fixture `Outcome::RuntimeNotInstalled` in a test. **Round 2 adds** the module doc's "carries the runtime's own stderr" (`:22`) and "a runtime that installs but does not run" (`:25`), and the test comment "the runtime's own stderr" (`:230`) | `engine/src/judgelog.rs:234`, `:12`, `:256-257`, `:22`, `:25`, `:230` | neutral literal; docs reworded ("the model call's own error text", "a service that answers but fails"); fixture becomes `ServiceUnavailable` / "service unavailable" | T5b |
| **Round 2:** the module doc's list of console children, "`git`, `taskkill`, the engine exe, a model runtime" | `engine/src/childproc.rs:4` | "a model runtime" dropped; doc only | T5b |
| `label_opts()`'s `runtime: None, model: None` | `engine/tests/cloud_contract.rs:951-952` | two lines deleted (Q9) | T2 |
| **Round 2, added by the email branch:** `opts()`'s `Options { …, runtime: None, model: None, … }` one-line literal | `engine/tests/mail_pass.rs:157-159` (EF) | the two fields go; nothing else in the file changes. Without this the engine's test crates stop compiling at T2 | T2 |
| The `dependency_boundary.rs` doc comment on `sha2` ("the APP's own direct edge, `app/Cargo.toml:69`, for `inference.rs`'s runtime digest check") | `engine/tests/dependency_boundary.rs:66-68` | reworded: the app's `sha2` edge now serves `account.rs`'s PKCE (`account.rs:247`). Every assertion in the file stays as it is | T5a |

`engine/src/cli.rs` does not name the runtime (checked), so L6 does not touch it. Its meeting with #19
there, which the #19 close-out plan's §5.3 anticipated, does not happen.

### 3.2 App, Rust (`app/`)

| Item | Where | Fate | Task |
|---|---|---|---|
| The whole module: `runtime_dir`, `models_dir`, `runtime_exe`, `find_exe`, `model_file`, `Status`/`status`, the hashing and `verify`, `SupportedRuntime`/`SUPPORTED_RUNTIMES`, `runtime_release_for`, `check_runtime_supported`, the model and runtime installers, `extract_runtime_zip`, `Asset`/`Manifest`/`MANIFEST_URL`, `fetch_manifest`, `Half`, `install_from_file`, `validate_asset`, `plain_asset_name`, `install_from_manifest`, `download_to` | `app/src/inference.rs` | deleted | T5a |
| `judgments_dir` | `inference.rs:76` | **moved** to `app/src/scheduler.rs` as `pub fn judgments_dir(profile_data_dir: &Path) -> PathBuf`, same body | T3a |
| `pub mod inference;` | `app/src/lib.rs:8` | deleted | T5a |
| `JudgeArgs`, `JudgeState`, `judge_state_in`, `judge_state`, `JudgePlan::Local`, `slot_argv`'s `Local` arm (`--runtime`/`--model`), `judge_plan_for`'s `NoAccount` arm | `app/src/scheduler.rs:273-316`, `:357-358`, `:381-385`, `:504-511` | deleted; `NoAccount` → `JudgePlan::Skip("judge (skipped: no account)")` | T3a |
| Docs that describe the local arm ("leaves in C4", "the local runtime/model gate", "C4 removes the runtime, not this folder") | `scheduler.rs:338-361`, `:375-376` | reworded | T3a |
| **Round 2:** three more scheduler docs: `LOGS_KEPT`'s "since Knowlu plan 3a a vault with an LMS feed and a model installed has four steps" (`:31-32`); `entitlement_state`'s "`NoAccount` (falls through to the local runtime/model gate, unchanged)" (`:325-326`, EF `:326`); `slot_argv`'s "`judge` only when a runtime and a model are both installed, which is what `judge_state` decides" (`:466-467`, EF `:554-555`) | `app/src/scheduler.rs` | reworded: a slot's step count is stated without "a model installed"; `NoAccount` is the named `no account` skip; `judge` runs when `judge_plan` returns `Cloud`. The file's two "a std thread with no runtime under it" comments (`:1157`, EF `:1276`) mean the async runtime and stay | T3a |
| `inference_root`, `inference_status`, `half_and_root`, `installed`, `install_inference_file`, `install_inference_download`, `remove_inference_model`, and the "plan 3a Task 10" banner | `app/src/commands.rs:706-727`, `:806-877` | deleted | T5a |
| `settings_context`'s marker read and its `"offer_inference"` key | `commands.rs:644-648`, `:654` | deleted; the envelope keeps every other key | T3b |
| `WizardPlan.offer_inference` (`serde(default)`); `offer_marker`; `set_offer_marker` and its three call sites; the doc comments that name it | `app/src/onboarding.rs:279-304`, `:434-435`, `:471-475`, `:879-881`, `:898`, `:971`, `:984-987` | deleted. Removing a `Deserialize` field is safe: `WizardPlan` has no `deny_unknown_fields`, so a page that still sends the key is not refused | T3b |
| **Added by the email branch (L0):** the docs of `mail_offer_marker` and `set_mail_offer_marker`, and one more doc, that link to `offer_marker`, `set_offer_marker` and `offer_inference` as their model | `app/src/onboarding.rs:290`, `:312`, `:491` (email-branch lines; re-confirm after the merge) | reworded to stand alone: each says what the mail marker is and when it is set and cleared, with no link to the removed items. `mail_offer_marker`, `set_mail_offer_marker` and their behaviour are untouched | T3b |
| **Round 2:** `GradesPrefs`'s doc cites "the precedent of `account::cache_path` and `onboarding::offer_marker`" (a file beside `settings.json`, never a field in it) | `app/src/grades.rs:87` (same line on EF and on L2's branch) | reworded to cite `onboarding::mail_offer_marker`, which after the email merge is the same precedent (a marker file in the profile folder). Doc only; no code line in `grades.rs` changes | T3b |
| `onboarding::pick_file` | `onboarding.rs:271-277` | **stays**, in both handler lists (L5's Offer B) | — |
| The four `commands::…inference…` names in the console's `generate_handler!` | `app/src/main.rs:179` | deleted | T5a |
| `zip` (direct dependency, used only by `inference.rs` and `app/tests/inference.rs`) and the "plan 3a Task 8" comment block | `app/Cargo.toml:62-75` | `zip` deleted. `sha2` stays (PKCE, `account.rs:247`) and `ureq` stays (account, grades, lms_link, report, telemetry); their comments are reworded to say so | T5a |
| `Cargo.lock` | workspace root (single owner) | regenerated by the build: the `knowlu` package loses its `zip` edge. `zip` stays in the lock through the engine (backups) and `tauri-plugin-updater` | T5a |

### 3.3 The page (`app/static/`)

| Item | Where | Fate | Task |
|---|---|---|---|
| The *Local judgment* settings row `#set-judge` (state, *Download*, *Install from a file…*, *Remove model*) | `app/static/index.html:223` | deleted | T4 |
| `renderInference`, `loadInference`, `installInference` | `console.js:1818-1854` | deleted | T4 |
| `openSettings`'s `.then(loadInference)` | `console.js:2177` | deleted; the chain ends at `renderSettings` | T4 |
| `renderAccountRow`'s `EL("set-judge").hidden = …` and its R-C1-59 comment | `console.js:1859-1864` | deleted | T4 |
| The three click handlers `#set-judge-download`, `#set-judge-file`, `#set-judge-remove` and their comment | `console.js:2214-2229` | deleted | T4 |
| The wizard's offer (`#wiz-judge`, its checkbox, `offer_inference` on the page) | — | **already gone since C1** (`static_assets.rs::the_wizard_never_offers_a_local_model`). Only the back end's marker half remains (§3.2, T3b) | — |

No `console.css` rule names `set-judge` (checked).

### 3.4 Tests

**Deleted with their code** (each pinned only removed behaviour; none is edited to pass):

| Test | File | Task |
|---|---|---|
| `resolve_prefers_the_argument_then_the_env_and_names_what_is_missing`, `kill_tree_ends_a_process_and_its_children`, `starting_a_runtime_that_is_not_there_says_which_path`, `a_child_that_outwrites_the_pipe_buffer_is_read_whole`, `a_call_that_never_answers_is_bounded` | `engine/src/runtime.rs` (in-module) | T5b |
| **`real_runtime_loads_a_model_and_answers_under_the_grammar`**, `#[ignore]`d. Deleted with the module, **never un-ignored**. The ignored count goes from 4 to 3 | `engine/src/runtime.rs` | T5b |
| `a_missing_runtime_or_model_is_an_outcome_that_still_carries_tier_one` (its property is re-pinned for `Missing::Service` in T1); `the_grammar_names_the_five_fields_and_only_those`, `the_prompt_carries_the_note_and_its_grounding_and_nothing_else`, `the_prompt_is_bounded_however_long_the_note_is`, `the_prompt_survives_a_note_with_nothing_in_it` (Q3; the request-body equivalents are `cloud_contract.rs::the_task_request_carries_the_note_and_its_grounding_and_nothing_else` and T1's new bound test) | `engine/src/judge.rs` | T5b |
| `run_lines_with_no_runtime_anywhere_reports_and_exits_zero` (replaced by T2's no-account test) | `engine/src/enrich.rs` | T2 |
| All 19 tests of the file: `sha256_of_matches_the_known_vectors` … `a_manifest_asset_url_that_is_not_https_is_refused` | `app/tests/inference.rs` (whole file) | T5a |
| `the_inference_commands_marshal_and_refuse` | `app/tests/commands.rs` | T5a |
| `judge_state_names_the_missing_half_runtime_first` | `app/tests/scheduler.rs` | T3a |
| `create_vault_drops_the_offer_marker_when_the_checkbox_was_checked`, `create_vault_leaves_no_offer_marker_when_the_checkbox_was_not_checked`, `restore_vault_drops_…`, `restore_vault_leaves_no_…`, `adopt_vault_drops_…`, `adopt_vault_leaves_no_…` and the helper `marker_for` (its one surviving caller, the email branch's mail-marker test, is rewritten below) | `app/tests/onboarding.rs` | T3b |
| `the_local_judgment_row_has_a_state_for_every_outcome` | `app/tests/static_assets.rs` | T4 |

**Rewritten, same property, new spelling** (the assertion's purpose is unchanged; only the removed
type or label is replaced):

| Test | File | Change | Task |
|---|---|---|---|
| `with_no_model_tier_one_still_writes_and_the_flag_stays_set` | `enrich.rs` | `Err(Missing::Model)` → `Err(Missing::Service("no account"))`; the line and the logged outcome read `service unavailable`; rename to `with_no_service_tier_one_still_writes_and_the_flag_stays_set` | T2 |
| `a_second_run_over_the_same_vault_writes_nothing`, `the_batch_is_bounded_and_says_how_many_are_left`, `an_empty_queue_is_one_line` | `enrich.rs` | the same seam change; the assertions on counts, bytes and lines stay | T2 |
| the `Options { … runtime: None, model: None … }` literals in `opts()` and two tests | `enrich.rs:1365`, `:1891-1892` | the two fields go | T2 |
| **Round 2:** `one_bad_item_does_not_stop_the_batch` | `enrich.rs:1651` (EF `:1657`) | the scripted error `"llama-cli: broken pipe"` becomes `"model: broken pipe"`. Assertions unchanged. Required: the tripwire scans in-module tests under `engine/src` and would report `llama` | T2 |
| **Round 2:** `a_broken_runtimes_stderr_survives_even_when_the_seed_answers_nothing` | `enrich.rs:1665-1697` (EF about `:1671-1703`) | renamed `a_failed_models_detail_survives_even_when_the_seed_answers_nothing`. The `Failed` text becomes `"judgment service: 502 — ERROR: model load failed"`. The doc and the two assertion messages say "the model call's failure text" and "a failed model call" instead of "the runtime's stderr" and "a broken runtime". The assertions themselves (exit 0, a line containing `ERROR: model load failed`) are unchanged. The property, failure text reaching stdout on the empty-seed path, holds for `CloudModel`'s `ModelError::Failed` as it did for `PerCall`'s | T2 |
| **Round 2:** `a_run_stops_at_its_wall_clock_budget_and_reports_the_remainder` | `enrich.rs:1721` (its doc) | "distinct from `runtime::CALL_TIMEOUT`" becomes "distinct from `cloudmodel::CALL_TIMEOUT`". Doc only | T2 |
| **Round 2, email branch:** `engine/tests/mail_pass.rs`'s `opts()` helper (every test in the file calls it) | `mail_pass.rs:157-159` (EF) | `runtime: None, model: None` go (§3.1). No test in the file is otherwise edited | T2 |
| the `judge` parse test (`--runtime a.exe --model b.gguf … is_ok()`) | `engine/src/main.rs:726` | parses `judge --log-dir c --limit 10`; `--runtime` and `--model` are now refused (T1's CLI test is the behavioural pin) | T2 |
| `the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured` | `app/tests/scheduler.rs` | the five-step half builds `JudgePlan::Cloud { log_dir }` instead of `Local(JudgeArgs)`, and asserts `judge --vault <v> --via local-runner --log-dir <d>`; the `Skip("… no runtime")` values become `Skip("judge (skipped: no account)")`. Its two comments "No judge args: the step is left out" (`:103`) and "With judge args: FIVE steps" (`:121-123`) say "a skip plan" and "a cloud plan" | T3a |
| **Round 2, added by the email branch:** `slot_argv_is_unchanged` | `app/tests/scheduler.rs:1938-1954` (EF) | the `JudgeArgs { runtime, model, log_dir }` line goes, and `JudgePlan::Local(ja)` becomes `JudgePlan::Cloud { log_dir: PathBuf::from("logs") }`. Every assertion stays byte for byte: none of them inspects the judge step's argv, and `Cloud` still yields a `judge` step in the same place | T3a |
| **Round 2:** `an_unreadable_cloud_yaml_is_named_and_never_falls_into_the_local_arm` | `app/tests/scheduler.rs:1191-1196` (same on EF) | doc only, plus a rename. The doc's "fall through to the local runtime/model gate and run `judge` with no account at all … distinct from … \"no runtime\"/\"no model\"" becomes: an unreadable file must not collapse into `NoAccount`, whose `judge (skipped: no account)` would hide a broken file; it is its own skip, distinct from "no entitlement" and "no account". Renamed `an_unreadable_cloud_yaml_is_named_and_never_reads_as_no_account`. Assertions unchanged | T3a |
| **Round 2:** the doc over `a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list` | `app/tests/scheduler.rs:319-324` | doc only. "`run_slot_inner` calls `judge_state(cs)` unconditionally" is replaced by the reason the `LOCALAPPDATA` seam stays after T3a. **Settled in round 3: no call remains.** On EF the only route from `run_slot_inner` to `state::app_data_root()` is `judge_state` → `commands::inference_root` (`app/src/scheduler.rs:314` → `app/src/commands.rs:724-725`), and T3a deletes `judge_state`. The other callers are not on the slot path with no tray handle: `commands::updates_dir` (`commands.rs:708-709`) is reached only from the console's update command (`:744`) and the resident loop's update tick (`scheduler.rs:1272`, which needs the tray's `AppHandle`); account deletion and sign-out (`account.rs:656`, `:1510`); `main.rs:58`; and the console commands at `commands.rs:601` and `:639`. So the seam stays, in both tests, **as a guard** that keeps any future call off the developer's real app data, and the doc says exactly that rather than naming a call. The sibling test is cited by its new name (`a_vault_with_no_account_records_the_judge_skip_and_stays_green`). Seams are never removed in this lane | T3a |
| `a_cloud_vault_runs_ingest_with_no_ics_url` | `app/tests/scheduler.rs` | the `Skip` label only | T3a |
| `a_machine_with_no_model_records_the_judge_skip_and_stays_green` | `app/tests/scheduler.rs` | renamed `a_vault_with_no_account_records_the_judge_skip_and_stays_green`; asserts the step reads `judge (skipped: no account)`, code 0, **and `engine_ok`** (the property that matters). The `LOCALAPPDATA` seam stays as a guard: after T3a nothing on `run_slot_inner`'s path reaches `state::app_data_root()` (evidence in the `a_vault_without_a_feed_…` row above). Its doc (`:213-232`) is rewritten to match: what it pins (no account is a named skip at code 0 with `engine_ok`) and that the seam is a guard against a future call, not a shield for a current one. The history of `judge_state`, `inference_root` and the settings row goes | T3a |
| `slot_argv_places_grades_after_ingest_and_before_judge_only_with_a_bundle` | `app/tests/scheduler.rs` | `JudgePlan::Cloud` instead of `Local(JudgeArgs)` | T3a |
| `an_entitled_vault_runs_judge_with_no_runtime_and_no_account_on_the_command_line` | `app/tests/scheduler.rs` | keeps its `--runtime`/`--model` absence check as a guard; its doc's "after C4, is every machine" is reworded. T1 tightens its `log_dir` pin to equality | T1 |
| `base_plan(offer_inference: bool)` and its roughly 25 callers, **plus the callers the email branch adds** (its mail-offer tests); the `WizardPlan { … offer_inference: false … }` literal; `base_plan`'s doc (`:448` EF, "the finish panel's checkbox is `WizardPlan.offer_inference`"); `offer_marker` in the file's `use knowlu::onboarding::{…}` line (`:7` EF) | `app/tests/onboarding.rs:162`, `:449-471` | `base_plan()` takes no argument; the field goes; every caller, main's and the email branch's, drops the argument; the doc loses the `offer_inference` sentence; the import loses `offer_marker` and keeps `mail_offer_marker` | T3b |
| `the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt` (added by the email branch) | `app/tests/onboarding.rs:1691-1705` (email-branch lines) | **only** the assertion at `:1705` that the inference marker is independent of the mail marker (with the `marker_for` call it needs) and the doc clause at `:1691` that says so are removed. Every assertion about the mail marker being set and cleared on create, restore and adopt stays byte for byte. This is a rewrite under §2's rule: the removed assertion pinned the inference marker, which is removed behaviour; the test's own property (the mail marker's life cycle) is unchanged | T3b |
| `every_non_test_spawn_suppresses_its_console_window` | `engine/tests/no_console.rs:82`, with the module doc `:8-9` and the comment `:80-81` | after T5b only `runs::git_sha` (`engine/src/runs.rs:63`) spawns a child outside test code (`childproc.rs:55` sits inside its `#[cfg(test)] mod`, which the scan skips). The floor becomes `with_spawns >= 1` (message "expected at least one file to spawn children"), which keeps the non-vacuous property: the guard still fails if every spawn site vanishes. The doc and comment name `runs::git_sha` as the one remaining spawn and drop `runtime.rs`. The per-file `guarded >= spawns` assertion is unchanged. If T0's re-grep finds the email merge added an engine spawn outside test code, the floor stays `>= 1` and the comment names every remaining spawn | T5b |
| the comment over the `pub mod childproc` assertion ("`runs::git_sha` and `runtime.rs` both spawn children") | `app/tests/no_git.rs:84` | comment only: "`runs::git_sha` spawns a child"; the assertion is unchanged | T5b |
| `the_wizard_never_offers_a_local_model` | `app/tests/static_assets.rs` | replaced by T1's `nothing_on_the_page_reaches_a_local_model` (T4 deletes the old one) | T4 |
| a scripted error `"llama-cli: connection refused"` and two doc lines | `judge.rs:893`, `:1060`, `:1179` | neutral wording; the tests' assertions are unchanged | T5b |

**Added** (T1, test-writer; each named with why it is red or green before the change):

1. `engine/tests/no_local_runtime.rs` (new): **the tripwire.** It reads the workspace from
   `CARGO_MANIFEST_DIR/..`, as `human_actor_literal.rs` does.
   - `the_runtime_files_are_gone`: `engine/src/runtime.rs`, `app/src/inference.rs` and
     `app/tests/inference.rs` do not exist. Red until T5a/T5b.
   - `nothing_names_the_local_runtime`: no file under `engine/src/**/*.rs`, `app/src/**/*.rs` or
     `app/static/**`, and neither `app/Cargo.toml` nor `engine/Cargo.toml`, contains any of: `llama`,
     `gguf`, `KNOWLU_RUNTIME`, `KNOWLU_MODEL`, `RUNTIME_EXE`, `SUPPORTED_RUNTIMES`, `PerCall`,
     `inference_status`, `install_inference`, `remove_inference_model`, `inference_root`,
     `offer_inference`, `offer-inference`, `set-judge`, `RuntimeNotInstalled`, `ModelNotInstalled`,
     `mod inference`, `mod runtime`, `runtime.rs`, `inference.rs`, `crate::runtime`,
     `crate::inference`, `knowlu_engine::runtime`. (Not bare `runtime::`: it would match
     `tauri::async_runtime::`.) No allowlist: on the pre-merge tree every hit is in a file this lane
     edits (checked; `account.rs` has none). Each hit is reported as `file:line` (the token). Red until
     T5b. It
     scans comments as well as code, on purpose: a doc that still describes a removed path is a bug
     report waiting to be filed. Test directories are not scanned (this file names every token).
   - `the_app_takes_no_zip_dependency`: no line of `app/Cargo.toml` starts with `zip`. Red until T5a.
2. `engine/tests/judge_without_runtime.rs` (new):
   - `judge_on_a_vault_with_no_account_names_the_reason_and_exits_zero`: runs
     `env!("CARGO_BIN_EXE_knowlu-engine") judge --vault <scratch>` on a scratch vault with one
     `needs_enrichment: true` task and no `config/cloud.yaml`. It asserts exit 0, a lead line
     starting `judge: skipped (no account`, no `runtime not installed` anywhere, and tier 1 written
     (a uid-pinned course, as `enrich.rs`'s own fixture does). Red today: the current run prints
     `runtime not installed`. An account-less vault is not entitlement-gated (`entitle::gate_in`), so
     the run reaches `enrich`. The test sets no `KNOWLU_RUNTIME`. A developer machine that exports one
     would take the old branch today and still fail, so the red holds either way.
   - `judge_refuses_the_runtime_and_model_flags`: `judge --vault <scratch> --runtime x.exe` and
     `--model y.gguf` each exit 2 (a clap usage error) and write nothing. Red today (both parse).
   - `a_missing_service_still_carries_tier_one` (green pin): `judge::judge_task(…, Err(Missing::Service("no account")))`
     is `ServiceUnavailable`, carries the tier-1 course and labels `service unavailable`. It replaces
     the deleted `a_missing_runtime_or_model_…` property.
   - `the_task_request_is_bounded_however_long_the_note_is` (green pin): `cloudmodel::task_request` on a
     note body of 10,000 characters puts at most `judge::MAX_BODY_CHARS` characters in `body`. It
     replaces the deleted prompt-bound test at the boundary where the bound now lives.
3. `app/tests/handler_lists.rs`, one test:
   `the_console_list_names_no_inference_command_and_both_lists_keep_pick_file`. No name in either list
   contains `inference`, and `onboarding::pick_file` is in both. Red today (four console names). It
   reuses the file's own `handler_lists` parser.
4. `app/tests/scheduler.rs`, one new test and one tightened pin:
   - `a_vault_with_no_account_skips_judge_by_name`: `judge_plan(&cs)` for a scratch vault with no
     `config/cloud.yaml` equals `JudgePlan::Skip("judge (skipped: no account)")`. It runs under
     `ENGINE_ENV_LOCK` with an `EnvSeam` pointing `LOCALAPPDATA` at a fresh temp folder, so the red run
     never reads the developer's real app data (today's code does, through `judge_state`). Red today:
     it returns `Skip("judge (skipped: no runtime)")`.
   - In `an_entitled_vault_runs_judge_with_no_runtime_and_no_account_on_the_command_line`, the
     `log_dir.starts_with(&cs.data_dir)` check becomes `assert_eq!(log_dir, cs.data_dir.join("judgments"))`.
     Green before and after. It pins the folder that `judgments_dir`'s move must keep byte for byte.
5. `app/tests/static_assets.rs`, one test: `nothing_on_the_page_reaches_a_local_model`.
   `index.html` has no `id="set-judge`. `console.js` contains none of `inference_status`,
   `install_inference`, `remove_inference_model`, `renderInference`, `loadInference`,
   `offer_inference` or `manifest.json` (the model-manifest URL; the absence check is carried over
   from the row test T4 deletes, `static_assets.rs:1249` pre-merge). Red today on the first group;
   the `manifest.json` check is green before and after.

### 3.5 Docs, comments and rule text

| File | What names the runtime | Fate | Task |
|---|---|---|---|
| `docs/reference/engine-commands.md` | the `judge` synopsis (`[--runtime <llama-cli.exe>] [--model <.gguf>]`, "one process per judgment", "no runtime and no model are normal outcomes"); "the same `judge::Model` the local runtime implemented" | synopsis without the two flags; outcomes "no account, session, entitlement or network"; the C2 paragraph's sentence reworded | T6 |
| `docs/reference/app.md` | app data's `runtime\`, `models\`; the skip names `judge (skipped: no runtime)` / `(skipped: no model)`; the `inference.rs` / `SUPPORTED_RUNTIMES` bullet; the "Direction" paragraph's "C4 … that runtime code stays until …" | folders and the bullet removed; skip name `judge (skipped: no account)`; the Direction paragraph says the removal landed (the lane PR's number) | T6 |
| `docs/reference/toolchain.md` | the ignored tests: "`runtime.rs`'s real-runtime smoke test, run by hand with `KNOWLU_RUNTIME` and `KNOWLU_MODEL` set" | three ignored tests, that one gone | T6 |
| `docs/surface/anatomy.md` | "**Local judgment** is the seventh settings row …" (about 12 lines) | paragraph removed; any later ordinal ("eighth row") recounted | T6 |
| `app/README.md` | the slot paragraph's "see **Local judgment** below"; the whole **Local judgment** section; the command list and counts near the top | the section becomes a short **Judgment** section: tier 1 and promoted rules on the device, tier 3 in the judgment service, judgments logged per profile in `profiles\<id>\judgments\`, never in the vault; command list and counts recounted by script | T6 |
| `HANDOFF.md` | §2's single-owner list (`engine/src/{main,cli,ingest,runtime,inference}.rs`); §3 Pilot's bullet "The local model runtime removed: …" and the C4 history line | `runtime` and `inference` leave the list; the Pilot bullet is marked done with the PR, the merge sha and the date | T6 |
| `engine/tests/workflows.rs:85`, `.github/workflows/release.yml:111`, `scripts/ci/tauri-cli.ps1:17` | a comment citing "the contract `app/src/inference.rs` holds for runtimes" as the precedent for pinning tauri-cli by SHA-256 | reworded to state the rule without the deleted file, comments only (Q5) | T6b |
| `CLAUDE.md` | four sentences (§10) | **proposals for Quinn**; no agent edits `CLAUDE.md` | §10 |
| `docs/specs/**`, `docs/plans/**`, `docs/reports/**`, `PROVENANCE.md`, `VISION.md` | history, rulings, ledgers | **not edited**: they are the record. The cloud design's C4 row and ruling 10 are signed text and keep their wording | — |

### 3.6 What stays, and why

- **`judgments_dir` and the folder it names.** Judgment logs never enter the vault (CLAUDE.md), and
  `JudgePlan::Cloud` passes `--log-dir` to the engine. The folder is still `profiles\<id>\judgments`.
  The function moves; its value does not.
- **`onboarding::pick_file`**, in both handler lists: L5 needs it.
- **`judge::parse_reply`**, with its clamps and its brace-scanning: `CloudModel` reads every reply
  through it (`cloud_contract.rs::the_reply_goes_through_parse_reply_so_the_clamps_still_apply`). The
  echo-skipping it does is harmless for a JSON reply, and removing it is not this lane's call.
- **`judge::{MAX_BODY_CHARS, MAX_WEIGHTS_CHARS, MAX_PREFS_CHARS, clip, one_line}`, `EventModel`,
  `EventItem`**: `cloudmodel.rs` and `events.rs` import them (L3's plan §4 lists `events.rs`'s imports).
  L6 renames and moves none of them.
- **`judge::Missing::Service` and `Outcome::ServiceUnavailable`**: the cloud arm's outcome, now also
  the no-account outcome. `Missing` keeps its one variant rather than being folded away, because L7
  (the OpenRouter 402 as a named issue) may want a second one.
- **`cloudmodel::CALL_TIMEOUT`** (120 s): already its own constant, so only its doc changes.
- **The app scheduler's own `kill_tree`** (`scheduler.rs:559`): it kills hung engine children and
  was never the runtime's. **`childproc::NoConsole`** still serves `runs::git_sha` and every engine
  child.
- **`sha2` and `ureq`** in `app/Cargo.toml`: other modules use them (§3.2).
- **Old judgment-log lines** on a device that read `model not installed` or `runtime not installed`
  stay as they are. Nothing reads the outcome strings back (`judgelog::read_day` has test callers only),
  and the logs never leave the device.
- **Folders already on disk** (`%LOCALAPPDATA%\knowlu\runtime\`, `models\`, `downloads\`,
  `runtime.new\`, and a profile's zero-byte `offer-inference` marker): no code removes them (Q2).

## 4. Open questions

Quinn's are marked **(Quinn)**; the rest are the controller's at dispatch. Each has a recommendation,
and the plan is written to it, so a "yes" changes nothing.

- **Q1 (Quinn). The no-account step's name.** **Ruled 2026-10-19 (Quinn): `judge (skipped: no account)`, as recommended.** Recommended: `judge (skipped: no account)`, the same
  words `sync` uses. It shows only on a vault with no `config/cloud.yaml`, which the wizard never
  creates (every wizard vault carries one from birth). So in practice it is a scratch vault or a
  hand-made folder, never a friend's. The alternative, leaving `judge` out silently, breaks "silence is
  never ambiguous".
- **Q2 (Quinn). Leftover runtime and model folders on existing installs.** **Answered 2026-10-19: the controller's read-only check found none of the four folders on Quinn's machine, so no clean-up code and nothing to delete.** Recommended: **no clean-up
  code.** No friend can have them. The wizard has offered nothing since C1, and the settings row is
  hidden whenever the vault has an account (`renderAccountRow`, R-C1-59), which every wizard vault
  does. Only a developer machine that installed a runtime during plan 3a's testing can hold them.
  Quinn deletes `%LOCALAPPDATA%\knowlu\{runtime,models,downloads,runtime.new}` by hand if they exist.
  The alternative is a one-time sweep in the housekeeping tick: a few lines, a test, and a code path
  that deletes files in app data for a case no student has. Quinn: do those folders exist on your
  machine, and should the controller check and delete them (read-only check first)?
- **Q3 (Quinn). Delete the local prompt and grammar (`judge::prompt_for`, `judge::GRAMMAR`) too.** **Ruled 2026-10-19 (Quinn): yes, delete them, as recommended.**
  Recommended: **yes.** They are the local model's prompt and its GBNF grammar, and only `runtime.rs`
  calls them. CLAUDE.md says the prompt, schema and pinned model live server-side, and VISION says
  nothing model-shaped ships in the app. Keeping them would leave a second, unused prompt that drifts
  from the server's. Their four tests go with them. The body bound is re-pinned on `task_request` (T1),
  and `cloud_contract.rs` already pins the request's content. `parse_reply` stays (§3.6).
- **Q4 (Quinn). The `CLAUDE.md` sentences** (§10). Four proposed edits, all of which follow from the
  removal. Recommended: approve them at the merge checkpoint. The lane merges whether or not they are
  answered: the code merge changes no rule, and the proposals wait in the ledger.
- **Q5 (controller). Three comments outside `docs/`** cite `inference.rs` as the precedent for pinning
  tauri-cli by SHA-256: `engine/tests/workflows.rs:85`, `.github/workflows/release.yml:111` and
  `scripts/ci/tauri-cli.ps1:17` (a CRLF file). Recommended: reword all three in one comment-only commit
  (T6b, `mechanical`). The reviewer confirms `git diff` shows comment lines only, and the eol check
  passes. The alternative is to leave them as history. That is harmless, but each one points a reader
  at a file that no longer exists, and one of them sits in the release workflow.
- **Q6 (controller and Quinn). Merge order on the single-owner files** (§5). Recommended: L6 is the
  first wave-1 lane to merge into `app/src/main.rs` and `app/src/commands.rs` after L0, and #19 goes
  before it only if #19 is already green and reviewed when L6's T5a is ready. L6 is mostly deletion and
  should take about a day, and every lane that needs a Tauri command waits on it (handoff §5).
- **Q7 (controller). L6's place in the `console.js` queue.** The queue (handoff §5; #19's close-out
  plan §5.4) is L4 → L7 → #19 → L1 R11 → L5, and L6 is not in it. T4 deletes about 45 lines in two
  regions of the settings panel and one `index.html` line, and adds nothing. Recommended: **at the
  head of the queue**, before L4. If L4's page work has already started, L6 goes immediately after L4.
  A deletion that adds nothing rebases trivially over L4's additions.
- **Q8 (controller). The no-account arm's shape in `enrich.rs`.** Recommended: keep `run_lines_with`'s
  `cloud: Option<&CloudClient>` signature and turn its `None` arm into the named skip, rather than
  making the parameter non-optional. Every cloud test calls `run_lines_with(…, Some(&client))`, so
  the narrower edit leaves them alone and gives L7, which is likely to edit the same function, less to
  rebase over.
- **Q9 (controller). `engine/tests/cloud_contract.rs`'s contract-list status.** This is the same
  question as L3's Q-B. T2 deletes two field lines from `label_opts()`'s struct literal, and the
  compiler forces the edit once `Options` loses the fields. Recommended: off the list (CLAUDE.md names
  "the oracle/sync/entitlement tests", and this is none of them). The edit stays with `implementer`, and
  T7 checks the file's diff is those two lines. If the controller rules the file on the list, T2's edit
  to it goes to `contract-engineer` and `contract-reviewer`.

## 5. Conflict map and the single-owner order

### 5.1 The order on `app/src/main.rs` and `app/src/commands.rs`

The handoff makes L6 the single owner of both files while it runs. Other lanes add Tauri commands only
by handing the line to L6's owner, or after L6 merges. The #19 close-out plan
(`docs/plans/2026-10-18-p3-registrar-close-out-plan.md` §5.3) already proposes how #19 and L6 meet.
This plan accepts that and extends it to the email branch and L7:

1. **L0, the email-forwarding branch, merges first.** L6's branch is cut from the `main` that
   carries it. The email branch edits `app/src/main.rs` (mail windows, handlers) and
   `app/tests/scheduler.rs`, so cutting L6 before the merge would put L6 on the wrong side of the
   largest diff in the wave. L6 starts nothing before then.
2. **PR #19 (`p3-registrar`).** #19 never touches `commands.rs`. Its `main.rs` part (the console
   builder manages `LmsSession`, two wipes, three `registrar::` handlers, the `use` line) is more than a
   few lines, so L6 does **not** carry it. L6's only `main.rs` edit, four deleted handler names, is
   T5a, the lane's last app commit.
   - If #19 has merged by T5a, T5a is cut from (or rebased onto) that `main`. The conflict is a few
     adjacent names in one list.
   - If L6 is ready first, L6 merges first, and #19's B1 (or a B1b) re-applies its `main.rs` part on
     top, recounting by script. Neither waits for the other.
   - L6's engine edits are in `engine/src/main.rs` (the `Judge` arm) and not in `cli.rs`. #19 edits
     `engine/src/main.rs`'s `commitments` arm and actor defaults. The two arms are disjoint, and
     whichever lands second rebases.
3. **L7 (OpenRouter 402 and "source went quiet" as named issues)** has no plan yet. If it needs a new
   Tauri command, it adds the handler after L6 merges. Handing a line to L6 is allowed by the handoff,
   but it would put untested code in a deletion PR, so it is not recommended.
4. **Everyone else who needs a handler** (L1 C3's upload commands, L4's re-consent, L5's syllabus
   commands) adds it after L6 merges, or merges before T5a so that T5a rebases. L6 accepts no foreign
   lines, which keeps its PR a pure removal that a reviewer can read as one.

Recommended overall (Q6): L0 → (#19 if ready) → **L6** → the lanes that add commands.

### 5.2 Every file L6 edits, and who else does

| File | L6 task | Other lanes | Rule |
|---|---|---|---|
| `app/src/main.rs` | T5a (four names) | L0 (merged before), #19, L1, L4, L5, L7 if they add commands | §5.1; recount both lists by script after any rebase |
| `app/src/commands.rs` | T3b (`settings_context`), T5a (the commands) | none may edit it while L6 runs (handoff) | §5.1 |
| `app/src/lib.rs` | T5a (`pub mod inference;`) | #19 (`pub mod registrar;`), email's modules | keep every other line |
| `app/src/scheduler.rs` | T3a | L2 grades may touch `grades_step`; L1 C3 may touch the mail step | L6 edits only the judge region (`JudgeArgs` … `judge_plan_for`, `slot_argv`'s judge arm) and adds `judgments_dir`; whoever is second rebases |
| `app/src/onboarding.rs` | T3b | L0 (merged before: `mail_offer_marker`, `set_mail_offer_marker`), **L5** T8b (`pick_file`'s optional extension list; `create_vault_in` staging hand-off) | L6 rewords the three L0 docs that link to the removed marker (§3.2) and leaves the mail marker's code alone. L6 removes `offer_inference` from `WizardPlan` and three `set_offer_marker` calls, and leaves `pick_file` alone. L5 rebases after L6, and the `WizardPlan` field list is the likely hunk |
| `app/Cargo.toml`, `Cargo.lock` | T5a (`zip` out) | any lane adding a dependency | regenerate the lock with `cargo build` on rebase, never by hand |
| `app/static/console.js`, `index.html` | T4 | L4 → L7 → #19 → L1 R11 → L5 queue | Q7: L6 at the head, or right after L4 |
| `app/tests/scheduler.rs` | T1, T3a | L0 (merged before: its `slot_argv_is_unchanged` uses `JudgeArgs` and `JudgePlan::Local`, rewritten per §3.4), L2, L1 | keep every other test; re-confirm test names after the email merge |
| `app/src/grades.rs` | T3b (one doc line, `:87`) | none found. L2's `l2/grades-score` edits `engine/src/grades.rs` (the score parse), a different file, and leaves `app/src/grades.rs:87` as it is on `main` | doc only |
| `engine/tests/mail_pass.rs` | T2 (two fields in `opts()`) | L0 (merged before; it adds the file) | nothing else in the file changes |
| `engine/src/childproc.rs` | T5b (one doc line, `:4`) | none found | doc only |
| `app/tests/onboarding.rs` | T3b | L0 (merged before: the mail-offer tests and their `base_plan(..)` callers), L5 (wizard staging tests) | `base_plan()` loses its argument, for L0's callers too. In L0's mail-marker test only the inference-marker assertion and its doc clause go (§3.4). On rebase, L5's new callers drop the argument |
| `app/tests/static_assets.rs` | T1, T4 | L4, L7, #19, L5 add assertions | keep every other test; a pin both sides need carries both |
| `app/tests/handler_lists.rs` | T1 | #19 or L4 may pin their handlers | keep both |
| `app/tests/commands.rs` | T5a (one test out) | none found | — |
| `engine/src/enrich.rs` | T2 | **L7** (likely, for 402), **L5** T5 (the syllabus pass) | L6 edits `Options`, `run`, `run_lines`, `run_lines_with`'s `None` arm, the module doc and four tests. Whoever is second rebases; Q8 keeps the signature so the cloud arm's code does not move |
| `engine/src/judge.rs` | T5b | **L3** (reads `CAPPED_LABEL`, imports for `events.rs`), **L7** (maybe a `Missing` variant) | L6 deletes only `Missing::{Runtime, Model}`, the two `Outcome` variants, `GRAMMAR`, `prompt_for` and their tests, and renames or moves nothing that `events.rs` or `cloudmodel.rs` imports (L3's plan §4 row) |
| `engine/src/main.rs` | T2 (`Judge` arm) | #19 (`commitments` arm), email's commands | disjoint arms; second rebases |
| `engine/src/lib.rs` | T5b (`pub mod runtime;`, comments) | #19 (`pub mod registrar;`), email's modules | keep every other line |
| `engine/src/cloudmodel.rs` | T5b (one doc comment) | **L3** T3b, **L5** T5, **L7** | L6's edit is the `CALL_TIMEOUT` doc only |
| `engine/src/judgelog.rs` | T5b (as §3.1 lists: the module doc at `:12`, `:22` and `:25`; the test comment at `:230`; the test literal at `:234`; the test fixture at `:256-257`) | none found | docs, one comment and two test lines; no non-test code line changes |
| `engine/tests/cloud_contract.rs` | T2 (two lines) | L3 T3b | Q9 |
| `engine/tests/dependency_boundary.rs` | T5a (one doc comment) | none found | assertions untouched |
| `engine/tests/no_console.rs` | T5b (the floor `>= 2` → `>= 1`, the module doc and one comment) | none found | the per-file `guarded >= spawns` assertion and the `#[cfg(test)]` boundary test are untouched |
| `app/tests/no_git.rs` | T5b (one comment, `:84`) | none found | comment only; every assertion untouched |
| `engine/tests/no_local_runtime.rs`, `judge_without_runtime.rs` | T1 (new) | none | — |
| docs (§3.5) | T6 | the Docs lane (HANDOFF refresh) | T6 lands after the Docs lane's refresh or hands it its lines |

`account.rs`, `approvals.rs`, every file on the contract list, `engine/src/cli.rs`, `surface.rs`,
`events.rs`, `cloud/**` and `engine/tests/fixtures/**` are untouched.

## 6. Tasks at a glance

The order is forced by one compile dependency: the app imports `knowlu_engine::runtime::RUNTIME_EXE`
(`inference.rs`, two app tests), so the engine's `runtime.rs` can be deleted only after the app stops
naming it. Behaviour changes come first, while everything still compiles; the deletions come last,
app before engine. Every commit leaves the workspace building and its tests green, apart from T1's
red tests, which stay red until the task that answers each one.

| # | Task | Agent | Why that agent | After |
|---|---|---|---|---|
| T0 | Branch, worktree, ledger, baseline run | main session | branching, commits and the baseline are the controller's | L0 merged |
| T1 | The red tests (§3.4 "Added", items 1-5) | `test-writer` | the failing tests first; five files, no source | T0 |
| T2 | Engine behaviour: the no-account arm, `Options`/`run` without runtime and model, the `judge` flags gone, `cloud_contract.rs`'s two lines, `mail_pass.rs`'s two fields (EF), `enrich.rs`'s docs and seven tests | `implementer` | specified, off the contract list, compiler- and test-checked | T1 |
| T3a | App scheduler: `JudgePlan::Local` and friends gone, the no-account skip, `judgments_dir` moved; its tests | `implementer` | same | T1 (parallel with T2: disjoint files, but mind the two-build limit) |
| T3b | The wizard offer's back end: `WizardPlan.offer_inference`, the marker, `settings_context`'s read, `grades.rs:87`'s doc; its tests | `implementer` | same | T3a (same crate, one implementer at a time) |
| T4 | The settings row out of the page | `console-ui` | `app/static` is its lane | T3b; its slot in the `console.js` queue (Q7) |
| T5a | App deletion: `inference.rs` and its tests, the four commands, `inference_root`, the four handler names, `pub mod inference;`, `zip`; the `dependency_boundary.rs` comment | `implementer` | pure deletion checked by the compiler; holds the `main.rs` edit last (§5.1) | T3a, T3b, T4; rebased over #19 if it merged |
| T5b | Engine deletion: `runtime.rs`, `pub mod runtime;`, the `Missing`/`Outcome` variants, `GRAMMAR`/`prompt_for` (Q3), comments (`childproc.rs`'s included); `no_console.rs`'s spawn floor and `no_git.rs`'s comment; the tripwire goes green | `implementer` | same | T2, T5a |
| T6 | Docs (§3.5 rows for `docs/` and `app/README.md` and `HANDOFF.md`) | `docs-keeper` | docs and HANDOFF are its lane | T5b |
| T6b | *(on Q5 yes)* The three tauri-cli comments | `mechanical` | three specified comment edits | T5b |
| T7 | Whole-branch review | `reviewer` | everything a cheaper agent changed passes review before a push | T6 (T6b) |
| T8 | Gate, PR, CI, merge on green + PASS; ledger; worktree removal | main session | pushes and merges are the controller's | T7 PASS |

No task needs `contract-engineer` unless Q9 rules `cloud_contract.rs` onto the list. No task touches
`cloud/`, so `cloud-engineer` has no part. The lane has no live proof: nothing in it reaches staging,
and the behaviour that changes (a named skip on a vault with no account) is pinned by tests. Quinn sees
the result at the next dev-build look (§8).

---

## T0. Branch, ledger, baseline (main session)

- [ ] After the email-forwarding merge lands on `main`, create branch `l6-runtime-removal` and its
      worktree (§2). If #19 has merged too, cut from that `main`.
- [ ] Open the ledger `.superpowers/sdd/2026-10-18-l6-runtime-removal/progress.md` with this plan's
      path, the branch point's sha, and §4's answers as dated rows.
- [ ] Baseline: `cargo build -p knowlu-engine`, then `cargo build --workspace` and
      `cargo test --workspace` under the slot guard. Record passed / failed / ignored (expected
      ignored: 4) and the warning tally. Later tasks compare against it.
- [ ] Re-run §3's greps on the branch (`runtime::`, `inference::`, `RUNTIME_EXE`, `JudgeArgs`,
      `offer_inference`, `offer_marker`, `marker_for`, `base_plan(`, `set-judge`, `--runtime`, `llama`,
      `GRAMMAR`, `prompt_for`, and `runtime.rs` / `inference.rs` under `engine/tests` and `app/tests`;
      round 2 adds `runtime: None` and `Options {` under `engine/tests`, `JudgePlan::` under
      `app/tests`, `-wi runtime` over `engine/src/{judge,enrich,judgelog,cloudmodel,childproc,lib}.rs`,
      and `offer_marker` over `app/src`)
      and confirm the email-branch items §3 already names (the mail-marker docs and test, the new
      `base_plan(..)` callers, `slot_argv_is_unchanged`, `mail_pass.rs`'s `opts()`). Record any other hit the email merge added. Each one joins the task that owns its file, as a ledger row.

## T1. The red tests (test-writer)

**Files:** `engine/tests/no_local_runtime.rs` (new), `engine/tests/judge_without_runtime.rs` (new),
`app/tests/handler_lists.rs`, `app/tests/scheduler.rs`, `app/tests/static_assets.rs`. **No source
file.**

**Brief.** §3.4 "Added", items 1-5, by name. Also:
- Each new test's doc comment says what it guards and which ruling asks for it (ruling 10; this plan's
  §3.4).
- The tripwire reports every hit, not the first. It collects `file:line (token)` strings and asserts
  the list is empty, printing the list. It follows `human_actor_literal.rs`'s `root()`, `files()` and
  `rel()` shape. Copying those helpers is fine; importing across test crates is not possible.
- The CLI tests build their scratch vault in a temp folder named for the test and the process id. The
  task note is a minimal `tasks/*.md` with an `id:`, a `title:`, `needs_enrichment: true` and a
  `source_uid` pinned in `config/ingest.yaml`'s `course_map`, copied in shape from `enrich.rs`'s test
  fixture (`NOTE`, `vault()`). The folder is removed at the end. No real course or school name.
- The engine-binary tests read stdout and exit status only, and never set `KNOWLU_RUNTIME` or
  `KNOWLU_MODEL`.
- The two `app/tests/scheduler.rs` additions name no item T3a removes (`judge_state`, `JudgeArgs`,
  "the local arm"), in code or in docs, so T3a's acceptance grep stays clean. Their docs describe the
  behaviour they pin, not the code they replace.

**Acceptance.**
- `cargo test --workspace` (slot guard): the new tests fail for **these** reasons and no others:
  - the tripwire: files exist, and tokens are named with their locations;
  - the no-account CLI test: the output has `runtime not installed` and no `judge: skipped (no account`;
  - the flags test: clap accepted the flags;
  - the handler test: four names contain `inference`;
  - the scheduler test: `Skip("judge (skipped: no runtime)")`;
  - the static test: `set-judge` and `inference_status` are found.
- The two green pins (`a_missing_service_still_carries_tier_one`,
  `the_task_request_is_bounded_however_long_the_note_is`) and the tightened `log_dir` pin pass.
- Every pre-existing test is still green, and there are 0 new warnings.
- Hand-off: the file list, the red output summarised per test, and a commit message
  ("test(l6): the runtime-removal tripwire and the no-account judge, red").

## T2. Engine behaviour: the no-account arm and the flags (implementer)

**Files:** `engine/src/enrich.rs`, `engine/src/main.rs`, `engine/tests/cloud_contract.rs` (two lines,
Q9), `engine/tests/mail_pass.rs` (two fields in `opts()`, added by the email branch). **Not** `runtime.rs`, `judge.rs` or `lib.rs`: the app still imports `runtime::RUNTIME_EXE`, so
the module survives until T5b, unused by the engine.

**Behaviour.**
- `run_lines`: `Unavailable::NoConfig` is no longer special. Every `Err(why)` takes the existing skip
  arm: `enrich_with(…, Err(Missing::Service(why.label())))`, with `judge: skipped ({why})` inserted as
  the first line. For `NoConfig` the line reads `judge: skipped (no account on this vault (no
  config/cloud.yaml))`, from `Unavailable`'s existing `Display`; no new string is invented. Tier 1 is
  still written, the flag stays set, and the exit is 0.
- `run_lines_with(…, cloud: None)` (Q8): the same result as above for `NoConfig` (lead line and
  `Missing::Service("no account")`), so a caller passing `None` gets the identical answer. The
  `runtime::resolve` / `PerCall` arm is gone.
- `Options` loses `runtime` and `model`. `run(vault, via, run_id, log_dir, limit)` loses the two
  parameters.
- `knowlu-engine judge` loses `--runtime` and `--model` (clap now refuses them with exit 2). Its help
  text says it enriches flagged tasks through the judgment service and always exits 0, with no
  account, session, entitlement or network as named outcomes. The dispatch passes the shorter
  argument list. The comment in `main()` that cites `judge (skipped: no runtime)` as an example cites
  `judge (skipped: no entitlement)` instead.
- `enrich.rs`'s module doc (the "starve, never switch" history can stay) loses the `llama-cli`, "no
  server, no load phase" and "a missing runtime, a missing model" passages. In their place: tier 3 is
  the judgment service, and a vault with no account is a named skip. `BATCH_BUDGET`'s doc cites
  `cloudmodel::CALL_TIMEOUT`. `enrich_with`'s and `run_lines_with`'s docs drop "`runtime::resolve`" and
  "`PerCall`". The round-2 rows of §3.1 (`:231`, `:243-245`, `:314-319`, `:325-326`) are reworded the
  same way, and the round-3 comment at `:583` (EF `:589`) cites `judge (skipped: no account)` in
  place of `judge (skipped: no model)`.

**Tests.** T1's `judge_on_a_vault_with_no_account_names_the_reason_and_exits_zero` and
`judge_refuses_the_runtime_and_model_flags` turn green. In-module: the §3.4 "Rewritten" rows for
`enrich.rs` and `main.rs`, and `run_lines_with_no_runtime_anywhere_reports_and_exits_zero` deleted.
In its place, `run_lines_on_a_vault_with_no_account_names_it_first_and_exits_zero` (in-module) asserts
the lead line's exact text and the logged outcome `service unavailable`. Round 2 adds three in-module
rewrites (`one_bad_item_does_not_stop_the_batch`'s literal, the renamed
`a_failed_models_detail_survives_even_when_the_seed_answers_nothing`, and
`a_run_stops_at_its_wall_clock_budget_and_reports_the_remainder`'s doc) and `mail_pass.rs`'s `opts()`,
each exactly as §3.4 states.

**Acceptance.** Workspace build and test green except T1's still-red tests (the tripwire, the handler
test, the scheduler test, the static test). 0 new warnings: `runtime.rs` is `pub` in a library crate,
so its now-unused items do not warn; if any do, report rather than `#[allow]`. `git grep -n
"crate::runtime" engine/src` is empty, and so is `git grep -niw "runtime\|llama-cli\|llama\.cpp"
engine/src/enrich.rs`, and so is `git grep -n "skipped: no model\|skipped: no runtime"
engine/src/enrich.rs`. `git grep -nE "(runtime|model): (None|Some)" engine` is empty (it catches every `Options`
literal, `mail_pass.rs`'s included). Hand-off with the file list and a commit message ("engine(l6):
judge on a vault with no account is a named skip; --runtime and --model are gone").

## T3a. App scheduler (implementer)

**Files:** `app/src/scheduler.rs`, `app/tests/scheduler.rs`.

**Behaviour.**
- `pub fn judgments_dir(profile_data_dir: &Path) -> PathBuf` moves here from `inference.rs`, with the
  same body (`profile_data_dir.join("judgments")`) and a doc that says why it is per profile and never
  in the vault. `judge_plan_for`'s `Entitled` arm calls it. `inference.rs`'s copy stays until T5a, now
  unused by `scheduler.rs`.
- `JudgeArgs`, `JudgeState`, `judge_state_in`, `judge_state` and `JudgePlan::Local` are deleted.
  `judge_plan_for`'s `NoAccount` arm is `JudgePlan::Skip("judge (skipped: no account)")` (Q1).
  `slot_argv`'s `Local` arm is deleted. The `Cloud` and `Skip` arms are unchanged byte for byte.
- `JudgePlan`'s doc now has three states: entitlement first, then an account runs `judge`, then no
  account is a named skip. The text about the local arm, "plan 3a" and "leaves in C4" goes.
- `scheduler.rs` no longer names `crate::inference` or `crate::commands::inference_root`.

**Tests.** T1's `a_vault_with_no_account_skips_judge_by_name` turns green. The §3.4 rows for
`app/tests/scheduler.rs` (rewritten and deleted) are applied, and the import line loses `JudgeArgs`,
`JudgeState` and `judge_state_in`. That includes the email branch's `slot_argv_is_unchanged`
(`JudgePlan::Cloud { log_dir: PathBuf::from("logs") }`, every assertion byte for byte), the rename of
`an_unreadable_cloud_yaml_is_named_and_never_falls_into_the_local_arm`, and the three doc rewrites
(`a_vault_with_no_account_…`, `a_vault_without_a_feed_…`, the unreadable-file test). The round-2 rows
of §3.2 for `scheduler.rs`'s own docs (`:31-32`, `:325-326`, `:466-467`) are applied too. The tightened
`log_dir` pin stays green, which proves the move kept the folder.

**Acceptance.** Workspace green except T1's tripwire, handler and static tests, and, if T2 has not yet
merged into the lane branch, T1's two engine CLI tests (`judge_on_a_vault_with_no_account_names_the_reason_and_exits_zero`
and `judge_refuses_the_runtime_and_model_flags`), red for T1's stated reasons. Any other red is a
regression to report. 0 new warnings.
`git grep -n "JudgeArgs\|JudgeState\|JudgePlan::Local\|judge_state" app` is empty, and so is
`git grep -n "skipped: no runtime\|skipped: no model\|runtime/model\|local arm\|local_arm\|runtime and a model\|model installed" app/src/scheduler.rs app/tests/scheduler.rs`.
(The bare words "no runtime" stay legal in `scheduler.rs`: its "std thread with no runtime under it"
comments mean the async runtime.) Hand-off and commit message
("app(l6): a vault with no account skips judge by name; the local judge plan is gone").

## T3b. The wizard offer's back end (implementer)

**Files:** `app/src/onboarding.rs`, `app/src/commands.rs` (**`settings_context` only**),
`app/tests/onboarding.rs`, `app/src/grades.rs` (**the doc line at `:87` only**).

**Behaviour.**
- `WizardPlan.offer_inference` is deleted, along with its doc and the struct doc's sentence about it. A
  page that still sends the key is not refused, because the struct has no `deny_unknown_fields`. This
  task's test proves it (below).
- `offer_marker`, `set_offer_marker` and the three calls (`apply_profile_settings_in`,
  `finish_profile_in`; the restore path reaches the latter through `finish_or_roll_back`) are deleted.
  The two functions' docs drop "drive … `offer_inference` marker directly" but keep why they are `pub`:
  the tests still drive them.
- The email branch's `mail_offer_marker` / `set_mail_offer_marker` docs and the third doc that links to
  the removed items (§3.2's L0 row; `onboarding.rs:290`, `:312`, `:491` on the email branch) are
  reworded to describe the mail marker on its own terms. No code line of the mail marker changes.
- `settings_context` no longer reads or clears the marker, and its envelope loses `"offer_inference"`.
  Every other key is unchanged. (`console.js` never read the key; `static_assets.rs` already pins its
  absence from the page.)
- `pick_file` is untouched.
- `grades.rs:87`: `GradesPrefs`'s doc cites `onboarding::mail_offer_marker` in place of
  `onboarding::offer_marker` as the precedent for a file beside `settings.json` (§3.2). No other line
  of `grades.rs` changes.

**Tests.** Delete the six marker tests and `marker_for` (§3.4). `base_plan()` loses its parameter,
every caller is updated (the email branch's mail-offer tests included), and the `offer_inference: false`
literal goes. In `the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt`, remove only
the inference-marker assertion (email-branch `:1705`, with its `marker_for` call) and the doc clause
that describes it (`:1691`); every mail-marker assertion stays as it is (§3.4 "Rewritten"). Add
`a_wizard_plan_that_still_sends_offer_inference_is_accepted`: deserialising a `WizardPlan` JSON that
carries `"offer_inference": true` succeeds. It is green before this task (the field exists) and must
stay green after (the field is gone and the key is ignored). It pins the upgrade path of a page cached
from an older build.

**Acceptance.** Workspace green except T1's tripwire, handler and static tests (and T1's two engine CLI
tests if T2 has not yet merged into the branch, as in T3a). 0 new warnings.
`git grep -nE "offer_inference|(^|[^_])(set_)?offer_marker|offer-inference" app/src app/tests` is
empty except the new test's JSON literal and `static_assets.rs`'s existing absence check. The pattern
is anchored on the left so `mail_offer_marker` and `set_mail_offer_marker` (which contain
`offer_marker`) do not match. `git grep -nw "marker_for" app/tests` is empty. One stale reference stays, and it is the grep's only
other expected hit: `account.rs:730` (EF `:767`) names `onboarding::offer_marker` as a precedent.
`account.rs` is a contract-list file owned by L4 in this wave, so the line is not edited here and goes
to §9's follow-up list. `grades.rs:87`, the other `::offer_marker` hit the anchored pattern finds, is
reworded in this task, so it is not an exception. Any further hit is a stop-and-report. Hand-off
and commit message ("app(l6): the wizard's local-judgment marker is gone").

## T4. The settings row out of the page (console-ui)

**Files:** `app/static/index.html`, `app/static/console.js`, `app/tests/static_assets.rs`.

**Behaviour.** §3.3's rows, exactly. The settings panel opens as before. `openSettings`'s chain is
`settings_context` → `get_settings` → `renderSettings`, with nothing after it. `renderAccountRow` keeps
every line except the `set-judge` one and its comment. No other row, id or handler changes, no copy is
added, and no CSS changes.

**Tests.** T1's `nothing_on_the_page_reaches_a_local_model` turns green. Delete
`the_local_judgment_row_has_a_state_for_every_outcome` and `the_wizard_never_offers_a_local_model`. T1's
test covers every assertion they made that still means anything: no `#wiz-judge` is subsumed by no
`set-judge`, no inference command, and no `offer_inference`. Its doc says so. Keep the old tests'
`#wiz-judge`, `2 GB` and `manifest.json` absence checks (the last from the row test,
`static_assets.rs:1249` pre-merge; T1 already carries it) in T1's test, so nothing that was pinned
becomes unpinned. Before deleting either old test, list each of its assertions in the hand-off with
where T1's test now holds it.

**Acceptance.** Workspace green except the tripwire and the handler test. `scripts/wizard-check.py`
reports ok, run by the controller (it needs the `.wv/` venv). A by-handle screenshot of the settings
panel on a fixture (memory: console fixture screenshots) is optional, at the controller's choice.
Hand-off with the diff's line counts per region and a commit message ("console(l6): the Local judgment
settings row is gone"). This task takes its slot in the `console.js` queue (Q7). If another lane's page
work merged first, rebase and re-run `static_assets`.

## T5a. App deletion, `main.rs` last (implementer)

**Files:** delete `app/src/inference.rs` and `app/tests/inference.rs`. Edit `app/src/commands.rs`,
`app/src/main.rs`, `app/src/lib.rs`, `app/Cargo.toml`, `Cargo.lock` (by the build),
`app/tests/commands.rs` and `engine/tests/dependency_boundary.rs` (the doc comment only).

**Before starting:** rebase onto `main` if #19 (or any lane that edits `main.rs`) merged since T0 (§5.1).

**Behaviour.**
- `commands.rs`: delete the plan-3a banner, `inference_root` with its doc, `inference_status`,
  `half_and_root`, `installed`, `install_inference_file`, `install_inference_download` and
  `remove_inference_model`. Nothing else in the file changes.
- `main.rs`: delete exactly the four `commands::…inference…` names from the console's
  `generate_handler!`. The wizard list is untouched, and `onboarding::pick_file` stays in both.
- `lib.rs`: delete `pub mod inference;`.
- `app/Cargo.toml`: delete the `zip` line. Reword the "plan 3a Task 8" block: `sha2` serves
  `account.rs`'s PKCE challenge, `ureq` serves the account, grades, LMS-link, report and telemetry
  calls. No version changes. The `Cargo.lock` change is whatever `cargo build` writes. Check it is the
  `knowlu` package's `zip` edge only.
- `dependency_boundary.rs`: the `sha2` doc sentence says the app's direct `sha2` edge is for
  `account.rs`'s PKCE. Every `assert!` and every forbidden-crate list is unchanged.

**Tests.** Delete `app/tests/inference.rs` (19 tests) and `commands.rs`'s
`the_inference_commands_marshal_and_refuse`. T1's handler test and the tripwire's
`the_app_takes_no_zip_dependency` turn green. The tripwire's file check is still red on
`engine/src/runtime.rs`, until T5b.

**Acceptance.** Workspace green except the tripwire's two engine-side checks. 0 new warnings. Recount
both handler lists by script, and record the counts in the hand-off for T6. `git grep -n
"knowlu_engine::runtime" app` is empty. Commit message: "app(l6): the local runtime installer, its
commands and its settings commands are gone".

## T5b. Engine deletion (implementer)

**Files:** delete `engine/src/runtime.rs`. Edit `engine/src/lib.rs`, `engine/src/judge.rs`,
`engine/src/judgelog.rs`, `engine/src/cloudmodel.rs` (one doc comment), `engine/src/childproc.rs`
(one doc line), `engine/tests/no_console.rs` (the spawn floor, its doc and comment) and
`app/tests/no_git.rs` (one comment).

**Behaviour.** §3.1's T5b rows:
- `pub mod runtime;` and its comment go. The `childproc` comment names `runs::git_sha` and the app's
  engine children, and the plan-3a comments over `judge`, `judgelog` and `enrich` describe today's
  roles.
- In `judge.rs`, `Missing::{Runtime, Model}`, `Outcome::{ModelNotInstalled, RuntimeNotInstalled}` and
  their match arms go. So do `GRAMMAR` and `prompt_for` with their docs (Q3), and the docs that describe
  the local path are reworded, including the two that cite `GRAMMAR`'s `string` rule (`parse_reply`'s
  doc and the last-`{` anchor test's doc, §3.1), and the round-2 doc lines §3.1 lists. After the task,
  `git grep -nw "GRAMMAR\|prompt_for" engine app` is empty, and so is
  `git grep -niw "runtime\|llama-cli\|llama\.cpp" engine/src/judge.rs engine/src/judgelog.rs engine/src/cloudmodel.rs engine/src/childproc.rs engine/src/lib.rs`.
- `childproc.rs`'s module doc drops "a model runtime" from its list of console children.
- `no_console.rs`: `with_spawns >= 2` becomes `with_spawns >= 1`, and the module doc and the comment
  above the assertion name `runs::git_sha` as the one remaining non-test spawn (§3.4 "Rewritten").
  Without this the guard reports "found 1" and the workspace cannot go green. `no_git.rs:84`'s comment
  drops `runtime.rs`. Neither file's other assertions change. `Outcome`'s doc now counts three ways a judgment ends (answered, low
  confidence, service unavailable) and drops the spec §5.3 quotation about running "with no model
  present".
- `judgelog.rs`: the test literal becomes neutral (for example `"model: panicked at prompt.cpp:88: token
  buffer overrun"`, which keeps the property that stderr text never reaches the log). The doc's "a week
  of `model not installed` lines" becomes "a week of `service unavailable` lines". The fixture outcome
  becomes `ServiceUnavailable`, with its label assertion `service unavailable`. The round-2 lines
  (`:22`, `:25`, `:230`) say "the model call's own error text" where they said "the runtime's own
  stderr", and "a service that answers but fails" where they said "a runtime that installs but does
  not run".
- `cloudmodel.rs`: `CALL_TIMEOUT`'s doc states the bound and why (a judge step runs inside a slot
  whose child cap is twenty minutes), without naming the deleted module. The value is unchanged.

**Tests.** The §3.4 deletions for `runtime.rs` (six tests, **including the ignored smoke test, by
deletion**) and `judge.rs` (five tests), the rewritten literals, and `no_console.rs`'s floor. Every
other test in those files is unchanged. `every_non_test_spawn_suppresses_its_console_window` passes
with `with_spawns == 1` (or more, if the email merge added a guarded engine spawn). The tripwire turns fully green, and every T1 test is now green.

**Acceptance.** Full workspace green: passed = baseline − deleted + added; **ignored = 3**; 0 failed; the
warning line reads `warnings: 1 accepted (.rsrc), …, 0 other` or its local equivalent. `git grep -n
"#\[ignore = " engine app` shows only the three that remain (`commitments.rs` also carries a bare
`#[ignore]` inside a string literal that builds a test source; it is not an attribute). Commit message:
"engine(l6): runtime.rs, the local prompt and grammar, and the not-installed outcomes are gone".

## T6. Docs (docs-keeper)

**Files:** `docs/reference/engine-commands.md`, `docs/reference/app.md`, `docs/reference/toolchain.md`,
`docs/surface/anatomy.md`, `app/README.md`, `HANDOFF.md`.

**Brief.** §3.5's rows for those files, from the merged branch's code, not from this plan's wording.
Handler counts come from T5a's recount. The ignored count is 3. In `HANDOFF.md`, mark §3 Pilot's
runtime bullet done (PR, merge sha, date) and take `runtime` and `inference` out of §2's single-owner
list. If the Docs lane is rewriting `HANDOFF.md` at the time, hand it these lines instead. Do not edit
`CLAUDE.md`, `VISION.md`, any spec, plan or report.

**Acceptance.** `git grep -n "inference_status\|SUPPORTED_RUNTIMES\|--runtime\|skipped: no runtime\|Local judgment" docs/reference docs/surface app/README.md HANDOFF.md`
is empty, except a dated history line in `HANDOFF.md` that records the removal. Commit message:
"docs(l6): the local runtime leaves the reference docs, the README and HANDOFF".

## T6b. Three comments (mechanical; only on Q5 yes)

**Files:** `engine/tests/workflows.rs` (the doc comment over
`release_downloads_tauri_cli_prebuilt_and_pinned_by_sha256_never_compiles_it`),
`.github/workflows/release.yml` (the comment above the tauri-cli step) and `scripts/ci/tauri-cli.ps1`
(the header comment, **CRLF preserved**).

**Brief.** Replace "the contract `app/src/inference.rs` holds for runtimes" (and its two siblings) with
the rule in plain words: a downloaded tool is pinned by version and SHA-256, so a bump changes both and
a wrong archive stops the run. Change **comment lines only**. No `run:`, `env:`, `uses:` or version
line moves.

**Acceptance.** `git diff` of the three files shows only comment lines. `scripts/ci/eol-check.ps1`
passes. `engine/tests/workflows.rs`'s tests are green.

## T7. Whole-branch review (reviewer)

Review `git diff main...l6-runtime-removal` against this plan. Write the report to
`docs/reports/2026-10-19-l6-runtime-removal-review.md`. Checks, beyond the usual:
1. Each §3 row landed as written. Nothing outside §5.2's file list changed. `cloud_contract.rs`'s
   diff is the two `label_opts()` lines (Q9), `mail_pass.rs`'s is the two `opts()` fields,
   `grades.rs`'s and `childproc.rs`'s are one doc line each, and `slot_argv_is_unchanged`'s
   assertions are byte for byte.
2. No test that pinned surviving behaviour was deleted or weakened. Each deleted test is in §3.4's
   "Deleted" table, and each rewritten one keeps its property.
3. The ignored smoke test was deleted, not un-ignored. The ignored count is 3.
4. `JudgePlan::Cloud`'s argv and `judgments_dir`'s folder are unchanged (the tightened pin). No
   `--runtime` or `--model` reaches any command line.
5. `commands.rs` gained nothing. The wizard list is unchanged, and `pick_file` is in both lists.
6. Rule 2: `git diff --exit-code main -- engine/tests/fixtures` is empty, and no `surface-today-*.json`
   moved.
7. Nothing on the contract list changed (CLAUDE.md's list, file by file).
8. The warning tally is 0 other. If T6b ran, its diff is comment lines only.

A PASS lets T8 go ahead. Findings are answered in this file's fidelity notes and re-reviewed.

## T8. Gate, PR, merge (main session)

- [ ] Final gate on the branch under the slot guard: engine build, workspace build, workspace test,
      eol check, `wizard-check.py`. Record the numbers in the ledger (passed, failed 0, ignored 3,
      warnings).
- [ ] Push the branch, open the PR ("L6: remove the local model runtime (ruling 10)"), and watch CI
      (`gh run watch <id> --exit-status`).
- [ ] Merge on green CI and T7 PASS (handoff §1 item 7), with a merge commit. Never over a red check
      without Quinn's word.
- [ ] Ledger rows for the merge. Tell #19's, L5's, L7's and L1's controllers that `main.rs` and
      `commands.rs` are free. Remove the worktree.
- [ ] Put §10's `CLAUDE.md` proposals and §9's follow-ups to Quinn at the next checkpoint, one at a time,
      with the PR's link.

## 7. Fidelity ledger

Every requirement this lane carries, its source, the task that carries it, and what proves it.

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Ruling 10 (Pilot), §12 C4 row | `runtime.rs` removed | T5b | tripwire `the_runtime_files_are_gone`, `nothing_names_the_local_runtime` |
| Ruling 10, C4 row | `inference.rs` removed | T5a | tripwire, both checks |
| Ruling 10, C4 row | `SUPPORTED_RUNTIMES` removed | T5a | tripwire token `SUPPORTED_RUNTIMES` |
| Ruling 10 ("the wizard offer"), C4 row | the wizard offer removed: its page half (since C1) and its back end (`offer_inference`, the marker, `settings_context`'s read) | T3b (T4 keeps the page pins) | tripwire tokens `offer_inference`/`offer-inference`; T1's static test; T3b's accepted-key test |
| C4 row ("the settings row") | the *Local judgment* row and its four commands removed | T4, T5a | T1's static and handler tests; tripwire `set-judge`, `inference_status`, `install_inference`, `remove_inference_model` |
| Handoff L6 row | the real-runtime smoke test is deleted with its module, never un-ignored | T5b | T5b's acceptance (ignored = 3, by deletion); T7 check 3 |
| CLAUDE.md "Gates" | no ignored test is un-ignored by changing its assertion | T5b | as above; no `#[ignore` line is edited |
| Handoff L6 row | "their tests" removed | T2, T3a, T3b, T4, T5a, T5b | §3.4's "Deleted" table; T7 check 2 |
| Handoff §5, p3 close-out §5.3 | `main.rs` and `commands.rs` single owner; the `main.rs` edit last; whoever is second rebases | T5a, T8 | §5.1; T5a's "before starting" rebase; T8 tells the waiting lanes |
| Task brief | the `dependency_boundary` test inventoried | T5a | §3.1's row: one doc comment reworded, every assertion unchanged (T7 check 1) |
| VISION "Nothing model-shaped ships in the app" | no local prompt, grammar, runtime driver or installer remains | T5a, T5b (Q3) | the tripwire; `GRAMMAR` and `prompt_for` gone |
| VISION "Failures are visible" | a vault with no account gets a named judge step, never silence or a failure | T2, T3a | T1's CLI test and scheduler test; `a_vault_with_no_account_records_the_judge_skip_and_stays_green` (`engine_ok`) |
| CLAUDE.md "`judge` always exits 0" | still exit 0 on every no-model path | T2 | T1's CLI test (exit 0); in-module `run_lines_on_a_vault_with_no_account_…` |
| CLAUDE.md "judgment logs never enter the vault" | `--log-dir` and `profiles\<id>\judgments` unchanged | T3a | the tightened `log_dir` equality pin |
| CLAUDE.md "commands.rs computes nothing"; handler lists | only removals; `pick_file` kept in both lists | T3b, T5a | T1's handler test; T7 check 5 |
| L5 spec, Offer B and T8b | `onboarding::pick_file` survives | T5a | T1's handler test (`pick_file` in both lists) |
| L3 plan §4 | nothing `events.rs` or `cloudmodel.rs` imports from `judge.rs` is renamed or moved | T5b | the workspace compiles; T7 check 1 |
| CLAUDE.md rule 1 | no single-user assumption added | all | T7 reads the diff; nothing new names a path, school or account |
| CLAUDE.md rule 2 | no frozen reference regenerated | all | T7 check 6 |
| CLAUDE.md contract list | no contract-list file edited (Q9 aside) | all | T7 check 7 |
| CLAUDE.md "0 warnings" | the deletion leaves no dead-code warning | every task | each task's acceptance; T8's gate line |
| CLAUDE.md "TDD" | the test first, then the code | T1 before T2-T5b | T1's red run recorded in the ledger with the reason per test |
| CLAUDE.md "tests never leave the machine" | no new test binds a socket or resolves a name | T1 | T7 reads T1's files |
| Prompt bound (deleted prompt test) | the bound now pinned where it lives, on the request body | T1 | `the_task_request_is_bounded_however_long_the_note_is` |
| Tier-1-without-a-model (deleted judge test) | re-pinned for `Missing::Service` | T1 | `a_missing_service_still_carries_tier_one` |
| Upgrade path | a page cached from an older build that still sends `offer_inference` is not refused | T3b | `a_wizard_plan_that_still_sends_offer_inference_is_accepted` |
| `CLAUDE.md` edits are Quinn's | four proposals, no agent edit | §10, T8 | the ledger row with Quinn's answer |
| Docs | reference docs, README and HANDOFF no longer describe the runtime | T6 | T6's acceptance grep |
| `no_console.rs` ("a guard that cannot fail is worse than no guard") | the console-window guard stays non-vacuous with one spawn site left | T5b | `with_spawns >= 1`; the test green at T5b |
| L0's mail offer (email branch) | the mail marker's life cycle stays pinned when the inference marker goes | T3b | `the_mail_offer_marker_is_set_and_cleared_on_create_restore_and_adopt`, mail assertions unchanged |
| Review finding 3 | the page's `manifest.json` absence stays pinned | T1, T4 | `nothing_on_the_page_reaches_a_local_model` |
| Round 2, finding 1 (L0's test) | the email branch's `slot_argv_is_unchanged` keeps pinning the slot's argv with no local plan | T3a | the test green with its assertions unchanged; T7 check 1 |
| Round 2, own sweep (L0's engine test) | `mail_pass.rs` compiles once `Options` loses its fields | T2 | the workspace builds at T2; T2's `(runtime\|model): (None\|Some)` grep |
| Round 2, findings 2 and 3, own sweep | no doc, comment or test literal in the lane's files still describes the runtime, the local arm or the inference marker | T2, T3a, T3b, T5b | the per-task greps in T2, T3a, T3b and T5b; the tripwire for `engine/src`, `app/src`, `app/static` |

## 8. Checkpoints

1. **Plan review** (`reviewer`, plan mode), before T1. Findings are answered in a fidelity-notes
   section at the end of this file, and the status line moves to REVIEWED with the report's path.
2. **Quinn**, one question at a time with its recommendation: Q1 (the skip's words), Q2 (leftover
   folders, and whether the controller checks the machine), Q3 (the local prompt and grammar), Q6 (the
   merge order, if the controller wants Quinn's word on it). Q4 (the `CLAUDE.md` edits) comes at the
   merge checkpoint, once the diff exists to show.
3. **Controller at dispatch:** Q5, Q7, Q8, Q9; T0's re-grep of the merged tree.
4. **After T1:** the red run, one reason per test, as a ledger row.
5. **T7**, before the push; then T8's merge on green CI and PASS.

## 9. Out of scope, and follow-ups this lane hands on

- **`app/src/account.rs`'s two stale comments**: `:730` (EF `:767`) cites `onboarding::offer_marker`
  as a precedent, and `:781` (EF `:818`) says a vault with no cloud config "falls through to the local
  runtime/model gate". `account.rs` is on the contract list and is L4's file in this wave. **Hand both
  lines to L4**, to reword in its own diff and review. The suggested words match L6's own: cite
  `onboarding::mail_offer_marker` (as T3b does in `grades.rs:87`), and say a vault with no cloud
  config is the named `judge (skipped: no account)`. If L4 has merged, they go to the next
  contract-list change.
- **`judge::parse_reply`'s echo-skipping** (it finds the answer past an echoed prompt) only ever served
  `llama-cli`. With JSON replies it is harmless. Simplifying it changes a function `cloud_contract.rs`
  pins, so it is not this lane's call. Listed for a later engine clean-up.
- **The cloud design's C4 row and ruling 10** keep their signed wording. If Quinn wants a "done"
  marker, it is a separate signed edit.
- **Old app-data folders** (Q2): no code; Quinn's machine only.
- **Removing `judge::Missing` as a one-variant enum**: left for L7, which may add a variant.
- **`cloud/supabase/functions/_shared/judge_prompts.ts:4`** says the server's task prompt is
  "`judge::prompt_for` moved across the boundary". After Q3 that function no longer exists. `cloud/**`
  is out of L6's scope, so **hand the comment to `cloud-engineer`** as a one-line follow-up: say the
  prompt was ported from the engine's former local prompt (removed in L6) and is the only copy now.
  Comment only; no prompt text, schema or pinned model changes, and no deploy is needed for a comment.

## 10. Proposed `CLAUDE.md` edits (Quinn's; no agent applies them)

Each one is quoted from the file as it stands on the main checkout. Apply them, if Quinn agrees, in
one docs commit after the lane merges. The facts they describe only become true at that merge.

1. **"Engine invariants", the `rank` bullet.** Now: "`src/judge.rs` is pure and the model process lives
   behind a trait in `src/runtime.rs`, so nothing under `cli.rs` can reach one." Proposed: "`src/judge.rs`
   is pure, and tier 3 is reached only through the `judge::Model` trait, whose one implementation is
   `CloudModel` (`src/cloudmodel.rs`), so nothing under `cli.rs` can reach one."
2. **"Command and app contracts", the always-exit-0 bullet.** Now: "no runtime, model, account,
   session, entitlement or network is a named outcome". Proposed: "no account, session, entitlement or
   network is a named outcome".
3. **"Command and app contracts", the runtime bullet.** Now: "`app/src/inference.rs` and
   `engine/src/runtime.rs` (the local llama.cpp runtime) stay until the Pilot's runtime-removal lane
   (Amendment 2026-09-29, ruling 10) removes them and are not extended." Proposed: delete it, or, if
   Quinn wants the guard named: "The local model runtime is gone (lane L6, ruling 10); nothing
   model-shaped returns to the app or the engine, and `engine/tests/no_local_runtime.rs` fails if it
   does."
4. **"Gates and conventions", the ignored-tests bullet.** Now: "Four tests are `#[ignore]` by design,
   each with its reason in the attribute: traps 4 and 5 in `engine/src/events.rs`, `runtime.rs`'s
   real-runtime smoke test, and `app/tests/scheduler.rs::run_slot_end_to_end`." Proposed: "Three tests
   are `#[ignore]` by design, each with its reason in the attribute: traps 4 and 5 in
   `engine/src/events.rs`, and `app/tests/scheduler.rs::run_slot_end_to_end`."

No other `CLAUDE.md` sentence names the runtime (checked: the agent table, the contract list and the
direction line do not).

## 11. Fidelity notes

### Review round 1 (2026-10-19, plan mode)

Five findings, all accepted. Docs-only revision of this file; no source, test or fixture was touched.

| # | Severity | Finding | Answer |
|---|---|---|---|
| 1 | important | Deleting `runtime.rs` leaves one non-test spawn (`runs::git_sha`), so `no_console.rs`'s `with_spawns >= 2` fails and T5b cannot go green | `no_console.rs` added to §3.4 "Rewritten", §5.2 and T5b: the floor becomes `>= 1` (still non-vacuous), the doc at `:8-9` and comment at `:80-81` name `runs::git_sha` as the one remaining spawn. `app/tests/no_git.rs:84`'s comment joins the same task. §2's TDD bullet now says a surviving test is edited only where §3.4 names the edit |
| 2 | important | The email branch's mail-marker test calls `marker_for` and asserts independence from the inference marker; three new `onboarding.rs` docs link to the removed items; T3b's acceptance grep matches `mail_offer_marker` | Named in §3.2 (the three docs), §3.4 (`marker_for`'s surviving caller; the mail-marker test as a rewrite that drops only the `:1705` assertion and the `:1691` doc clause; the email branch's `base_plan(..)` callers), §5.2 and T3b. The acceptance grep is anchored (`(^|[^_])(set_)?offer_marker`) and `marker_for` is checked with `-w`. T0's re-grep confirms these on the merged tree |
| 3 | minor | T1's static test dropped the row test's `manifest.json` absence check | Added to T1's `nothing_on_the_page_reaches_a_local_model` (§3.4 "Added" item 5) and to T4's fold-in list; T4's hand-off maps every deleted assertion to where it now lives |
| 4 | minor | `judge.rs:628` and `:1208` still cite `GRAMMAR`; `judge_prompts.ts:4` cites `prompt_for` | Both `judge.rs` docs added to §3.1's reword row and T5b, with a `-w` grep for `GRAMMAR`/`prompt_for` in T5b. `judge_prompts.ts:4` listed in §9 as a `cloud-engineer` follow-up (comment only) |
| 5 | minor | T3a's acceptance did not allow T2's two CLI tests to be red when T2 has not merged | T3a's acceptance (and T3b's, which can also run before T2 lands) now allows them, red for T1's stated reasons, and calls any other red a regression |

The email-branch line numbers in rows 2 (`onboarding.rs:290`, `:312`, `:491`; `app/tests/onboarding.rs:1691-1705`)
come from the review, not from a read of the email worktree; the implementer re-confirms them on the
lane branch, as the "Code read for this plan" note at the top requires of every `file:line`. Round 2
read them: `onboarding.rs:286-317`, `:449-491` and `app/tests/onboarding.rs:1691-1705` match.

### Review round 2 (2026-10-19, plan mode)

Three findings, all accepted. The task also asked for a full sweep of the main checkout and the
email-forwarding and p3-registrar worktrees (read-only; nothing in either worktree was edited). That
sweep found eight more items, which are answered here too. This was a docs-only revision of this file.

| # | Severity | Finding | Answer |
|---|---|---|---|
| 1 | important | The email branch's `slot_argv_is_unchanged` (EF `app/tests/scheduler.rs:1938-1954`) builds `JudgeArgs` and `JudgePlan::Local`; nothing names it, so T3a cannot compile under §2's stop-and-report rule | Confirmed at EF `:1944` and `:1946`. Added a §3.4 "Rewritten" row (`JudgePlan::Cloud { log_dir: PathBuf::from("logs") }`, the `JudgeArgs` line gone, every assertion byte for byte), and added it to T3a's Tests, §5.2's `app/tests/scheduler.rs` row, T0's confirm list, T7 check 1 and §7 |
| 2 | minor | `app/src/grades.rs:87` cites `onboarding::offer_marker`; T3b's anchored grep matches it and the file is outside T3b's list | Confirmed on `main`, EF and L2's worktree (the same line in all three). Taken into T3b as a doc-only reword citing `onboarding::mail_offer_marker`, the same precedent once L0 has merged. Added to §3.2, T3b's files and behaviour, §5.2 and §6. The reviewer's note that L2 "may also hold" the file does not hold: L2's branch edits `engine/src/grades.rs`, not `app/src/grades.rs`. T3b's grep now has one expected hit, `account.rs`, and any other hit is a stop-and-report. §9's `account.rs` hand-off to L4 carries the same suggested words and the EF line numbers (`:767`, `:818`) |
| 3 | minor | Two stale test docs in `app/tests/scheduler.rs`: the unreadable-cloud-yaml test (`:1191-1194`) and the doc at `:316-323` (`judge_state(cs)`) | Confirmed on `main` and EF. Both are §3.4 rows for T3a. The unreadable-file test's doc states its surviving property (it never collapses into the `no account` skip), and the test is renamed `…_never_reads_as_no_account` with its assertions unchanged. The feed test's doc gives the reason its `LOCALAPPDATA` seam stays. Round 2 could not confirm that `run_slot_inner` still reaches app data once `judge_state` is gone, so the plan now says the seam stays either way. The same sweep found a third stale doc, `a_machine_with_no_model_…`'s (`:213-232`), now rewritten in its existing row. The "No judge args" / "With judge args" comments in `the_slot_runs_…` are named too. T3a's acceptance grep covers all of them |
| S1 | important (own sweep) | EF `engine/tests/mail_pass.rs:157-159`'s `opts()` builds `enrich::Options { …, runtime: None, model: None, … }`. Once T2 deletes the fields, the engine's test crates do not compile | Added to §3.1, §3.4, §5.2, T2's files and tests, §6, T7 check 1 and §7. T2's acceptance adds a `git grep -nE` for `runtime` or `model` followed by `: None` or `: Some` over `engine`, which must come back empty. In EF today that pattern hits exactly the five `Options` literals (six lines) |
| S2 | important (own sweep) | `enrich.rs`'s in-module tests carry `llama-cli` literals (`one_bad_item_does_not_stop_the_batch` `:1651`; `a_broken_runtimes_stderr_…` `:1686`). The tripwire scans `engine/src/**`, tests included, so T5b could not go green, and §2's rule would stall T2 | Both are §3.4 "Rewritten" rows for T2: neutral literals, unchanged assertions, and the second test renamed `a_failed_models_detail_survives_even_when_the_seed_answers_nothing`. `a_run_stops_at_its_wall_clock_budget_…`'s doc (`runtime::CALL_TIMEOUT`) is a row too |
| S3 | minor (own sweep) | More runtime prose in non-test code that the rows did not name: `enrich.rs:231`, `:243-245`, `:314-319`, `:325-326`; `judge.rs:94`, `:131`, `:167`, `:201`, `:622`, `:633`, `:661`, `:1142`, `:1219`; `judgelog.rs:22`, `:25`, `:230`; `childproc.rs:4`; `scheduler.rs:31-32`, `:325-326`, `:466-467` | Added to §3.1 and §3.2 rows for T2, T3a and T5b. `childproc.rs` joins T5b's files. Each task gains a `-w` grep over its own files (T2: `enrich.rs`; T3a: `scheduler.rs` and its test; T5b: `judge`, `judgelog`, `cloudmodel`, `childproc`, `lib`). `scheduler.rs`'s "std thread with no runtime under it" comments mean the async runtime; they stay, and the grep is written so that they do not match |
| S4 | minor (own sweep) | `app/tests/onboarding.rs` imports `offer_marker` (EF `:7`), and `base_plan`'s doc (EF `:448`) names `offer_inference` | Added to the `base_plan` row for T3b. T3b's existing anchored grep already catches both, so this only makes the edit a named one |
| S5 | minor (own sweep) | T1's new scheduler tests could name `judge_state` in their docs (the plan's own red-run reasoning does) and trip T3a's grep | T1's brief now says the two additions name no item T3a removes |
| S6 | — (own sweep, nothing to change) | p3-registrar (#19) and `cloud/**` | p3-registrar has no runtime, model or local-judge reference beyond what `main` has (its hits are a subset, from an older base). `cloud/**` has one, `judge_prompts.ts:4`, already in §9. EF's `app/src/mail/**` names only `mail_offer_marker`, and EF's `app/tests/mail.rs:4587` uses only `mail_offer_marker`; neither is touched. EF's `release.yml` comment moved to `:117` (T6b finds it by text) |

Line numbers marked EF are from the email-forwarding worktree as it stood on 2026-10-19. As before,
the implementer re-confirms every one on the lane branch.

### Review round 3 (2026-10-19, plan mode)

Passed, with three minor findings, all accepted and closed in place without another round. This was a
docs-only revision of this file.

| # | Severity | Finding | Answer |
|---|---|---|---|
| 1 | minor | `enrich.rs:583` (EF `:589`) cites `judge (skipped: no model)`; no row names it and no grep catches it | Added to §3.1's round-2 `enrich.rs` row (it cites `judge (skipped: no account)` instead) and to T2's behaviour. T2's acceptance adds `git grep -n "skipped: no model\|skipped: no runtime" engine/src/enrich.rs`, which must be empty; on `main` today it hits only `:583` |
| 2 | minor | §5.2's `judgelog.rs` row ("a test literal, one doc line, one test fixture") undercounts §3.1's lines | The row now lists §3.1's lines: `:12`, `:22`, `:25`, `:230`, `:234`, `:256-257` |
| 3 | minor | Settles §3.4's seam question: after T3a nothing on `run_slot_inner`'s path reaches `state::app_data_root()` | Re-read on EF: the only route was `judge_state` (`scheduler.rs:314`) → `commands::inference_root` (`commands.rs:724-725`). The reviewer cited `:709` as part of that route; `:709` is in `commands::updates_dir`, which only the console's update command (`:744`) and the resident loop's update tick (`scheduler.rs:1272`, which needs the tray's `AppHandle`) reach, so the conclusion holds. The two §3.4 rows now say no call remains and that the seam stays as a guard |
