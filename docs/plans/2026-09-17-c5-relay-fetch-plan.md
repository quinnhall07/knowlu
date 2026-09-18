# Knowlu C5 — the relay fetch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: AMENDED 2026-09-17 (fix rounds 1-3 after review), not started; blocked on C3′.** The
review is `docs/reports/2026-09-17-c5-relay-fetch-plan-review.md` — `5a2ca69` (verdict *execute after
fix round 1*, findings C1-C5, I1-I14, M1-M12) and its appended **Re-review after fix round 1**
(`9f186c7`, verdict *execute after fix round 2*, findings R1-R10 plus rulings on the author's five
concerns) and **Re-review after fix round 2** (`cca7699`, verdict *execute after fix round 3*, which
closed R1-R10 and found one gap, **S1**). Every finding of all three rounds is applied here and
named in **Fix round 1 — resolutions**, **Fix round 2 — resolutions** and **Fix round 3 —
resolutions** at the end of this file, together with the five controller rulings — R-C5-plan-1/2/3 from the first round and **R-C5-plan-4** (the console gets a Logins row,
so the pause's exit is reachable after onboarding) and **R-C5-plan-5** (`Expires`/`Max-Age` are
parsed off the raw `Set-Cookie` line, because `ureq::Cookie` exposes neither) from the second. The
spec was amended in both rounds (its Status line, §2.4, §2.5, §4, §6, §8 and four §12 rows). This plan is written from
`docs/specs/2026-09-17-c5-relay-fetch-design.md` (valid; Quinn signed the amendment it argues from on
2026-09-17) and from the cloud design's **Amendment 2026-09-17, ruling 4**
(`docs/specs/2026-09-09-knowlu-cloud-design.md`, its last section). Ruling 5 puts C5 **after C3′ and
before C4**, and the spec's own first paragraph says *nothing here starts before C3′ merges*. So:
**the branch `c5-relay` forks from `main` after C3′ merges**, in its own worktree
(`.claude/worktrees/c5-relay`, git-ignored), and Task 1 does not begin before that merge commit is
`main`'s head.

**What this plan assumes C3′ has already landed** — three outcomes, named here so a reader can check
them in one command each rather than discover one missing at Task 8:

1. **The engine gates itself on entitlement.** `engine/src/entitle.rs` exists, `engine/src/main.rs`
   calls it in front of `coursework`, `ingest`, `judge` and `sync`, and a refusal is a named line at
   exit 0 (C3′ Task 8, hand-offs H3a/H4b). C5 **inherits** that gate and re-decides nothing: a relay
   run is never started without an entitlement because `coursework` is never started without one.
   Check: `git grep -n "entitle::gate" engine/src/main.rs`.
2. **`knowlu-engine sync` is the slot's first step.** `app/src/scheduler.rs::slot_argv` produces
   `sync → coursework → [ingest] → [judge] → rank` (C3′ hand-off H8a). Every hand-off in this plan
   that touches `slot_argv` is written against **that** shape, and names the `coursework` arm by the
   match it sits in rather than by an index, so a re-ordering cannot silently move it.
   Check: `git grep -n '"sync".into()' app/src/scheduler.rs`.
3. **`base64` and `ring` are gone from `engine/Cargo.toml` and the dependency test refuses their
   return** (C3′ Task 2). C5 needs base64 for one thing only — a response body that is not valid
   UTF-8 — and therefore **hand-rolls a 20-line encoder in `relay.rs` rather than re-adding the
   crate**. The refusal list is not relaxed, not by one name. Check:
   `git grep -n 'base64 = ' engine/tests/dependency_boundary.rs`.

Two more C3′ outcomes C5 must not undo: `engine/src/history.rs` is deleted and
`engine/tests/no_console.rs`'s floor is `with_spawns >= 2` (C3′ Task 10 step 5). **C5 deletes two
files that spawn no child process**, so that floor does not move again, and Task 10 asserts it rather
than assuming it. And `site/privacy.html`'s four *"Your tasks and notes live in…"* copies are C3′'s
and are **not** edited here; C5 rewrites two *different* sentences on that page (§5 of the spec), in
Task 11.

**Goal.** The device stops knowing how to log in to anything. Today `coursework::fetch_zybooks`
(`engine/src/coursework.rs:464`) and `fetch_vhl` (`:511`) read a password out of Credential Manager
and hand it to `zybooks::signin` and `vhl::login_and_fetch_dashboard`, which own the URLs, the
headers, the CAS form and the host change. After C5 the cloud composes one request at a time with
`{{credential:…}}` placeholders in it; the device fills them from Credential Manager, sends the
request from the student's own machine with that source's cookie jar, and returns the raw response;
the cloud parses it and composes the next request. The password never leaves the machine, the vendor
still sees the student's own IP and session, `engine/src/zybooks.rs` and `engine/src/vhl.rs` are
deleted, and repairing a portal after a markup change becomes a deploy instead of a release. What the
client keeps is **one compiled-in table of which hosts a given credential may ever be sent to** — the
guarantee that survives a compromised server of ours.

**Architecture.** One new engine module, one new edge function, **no new engine command**, and one
new Tauri command — `commands::save_portal_login`, on the console window, which is how a student
changes a portal password after onboarding and how the paused-source card's *Settings → Logins*
sentence becomes true (ruling **R-C5-plan-4**, from review **R1**). On the **device**, `engine/src/relay.rs` holds `PORTAL_SOURCES` (the host allow-list), the
host check, the placeholder substitution and its four encoders, one `ureq::Agent` per source per run
with its own cookie jar, a DPAPI-sealed per-source session store under
`%LOCALAPPDATA%\knowlu\profiles\<profile_id>\sessions\`, and `run()` — the loop that posts a batch of
results and performs the batch of steps that comes back. In the **cloud**,
`cloud/supabase/functions/relay/` answers `POST /relay` behind C1's `requireActiveEntitlement`,
drives a versioned plan module per source (`plans/zybooks.ts`, `plans/vhl.ts`) over a `relay_runs`
row that holds a cursor and never a page body, and hands the accumulated payload to
`ingest-coursework`'s **existing** `ingestHandler` in process, so the parsers, the warnings, the
proposals and the frozen oracles are untouched and `done` is byte-identically the reply the device
decodes today. `relay::run` is a **library** driven from inside `coursework` and
`coursework-discover`; there is no `knowlu-engine relay` subcommand, and §7 of the spec says why.

**Tech Stack:** Rust 1.98 `stable-x86_64-pc-windows-gnu`; `ureq 3.4` with its non-default **`cookies`**
feature (already in `engine/Cargo.toml:38-42`, for a rewritten reason — see Task 10); the `windows`
crate at 0.62.2 gains **one feature**, `Win32_Security_Cryptography`, for DPAPI
(`CryptProtectData`/`CryptUnprotectData` in `windows::Win32::Security::Cryptography`, freed with
`LocalFree` from `windows::Win32::Foundation`, which is already enabled) — **no new crate anywhere**.
Supabase Edge Functions on Deno 2.9.6 with C1's `_shared/{auth,db,entitlement,http}.ts` and C2's
`ingest-coursework/{handler,parse_zybooks,parse_vhl}.ts`, reused and not re-implemented; Postgres 15
for one table. `deno test --allow-read` (this stream's own files need nothing more).

**Spec.** `docs/specs/2026-09-17-c5-relay-fetch-design.md` — read it **whole** before Task 1: §2 (the
contract), §3 (the allow-list), §4 (sessions), §5 (data handling), §6 (server side, both plans step
for step), §7 (what leaves the engine), §9 (testing), §10 (Quinn's five), §12 (the fidelity ledger
this plan's own ledger answers to). Then the cloud design's *Amendment 2026-09-17* ruling 4 in full,
with rulings 2, 3 and 5 as context. Supporting: `docs/specs/2026-09-09-knowlu-cloud-design.md` §3.1,
§4.3, §5.2, §9, §11a; `docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` §6; `CLAUDE.md`;
`VISION.md`; `HANDOFF.md` §2-§4; and the two plans this one is shaped after,
`docs/plans/2026-09-17-c3-account-vault-plan.md` and `docs/plans/2026-09-17-c1b-sign-in-plan.md`.

---

## Global Constraints

Every task's requirements implicitly include this section. It is the C3′ plan's, trimmed to what
binds here, with each C5-specific narrowing said out loud.

- **Add no single-user assumptions.** Nothing in `cloud/`, `engine/`, `app/`, a migration, a plan
  module or a fixture names a person, a vault, a machine, an account, an email address or a
  credential. A *credential target name* is a shape (`knowlu/<profile_id>/<source>`), never a value,
  and never travels: `redact` (`coursework.rs:565`) already keeps `credential_target` off the wire
  and C5 does not widen it. Anything that would need hand-editing for a second user is a bug.
  (`CLAUDE.md`, rule 1.)
- **Never regenerate a frozen reference.** The eight Python-written references in
  `engine/tests/fixtures/` are read-only oracles, and two of them —
  `zybooks-parsed-reference.json` and `vhl-parsed-reference.json` — are **exactly what this stream
  leans on**: they stay where they are as the server parsers' oracles (spec §7, ruling R4-16), read
  by `parse_zybooks_test.ts` and `parse_vhl_test.ts` over the relative path
  `../../../../engine/tests/fixtures/` they already use. **No task in this plan writes into
  `engine/tests/fixtures/`**, and `git status --porcelain --untracked-files=all
  engine/tests/fixtures/` is **empty at every task boundary**. The three Rust-generated
  `surface-today-*.json` are not read or written by anything here.
- `engine/tests/oracle.rs` and `engine/tests/surface_oracle.rs` must pass **unchanged** at every task
  boundary. None of the three fixture vaults carries a `config/cloud.yaml`, so no relay run can start
  on one and their `today.md` cannot move.
- **Every note write goes through `write` — journal record first, single-line frontmatter surgery
  second.** `src/yamlemit.rs` is the crate's one YAML emitter and **no note is ever parsed and
  re-dumped.** This plan writes exactly **one** new kind of note, through the existing
  `info::open_info` (Task 8's login card), and changes nothing about `sync_coursework`,
  `write_map_card` or `apply_map_cards`.
- **`journal::VIAS`, `journal::OPS`, run records, ledgers and note frontmatter are contracts with
  existing vaults: byte-identical, never renamed.** This plan adds no `via`, no `op`, no frontmatter
  key and no run-record field. The `coursework` run record keeps its three steps (`zybooks`, `vhl`,
  `sync`) and their counts.
- **All JSON this crate writes goes through `ledger::dumps_value`** (Python `json.dumps`
  separators), never `serde_json::to_string`. That includes every body `relay::run` posts to
  `/relay`, so a request assembled today and one assembled next year are the same bytes.
- **`rank` never calls a model, and no path under `cli.rs` may reach `/judge-*`** (Knowlu spec
  decision 11, `CLAUDE.md`). The relay is **transport**: it carries a vendor's own bytes to a parser
  and never a prompt to a model. C2's `rank_cannot_reach_a_judgment_endpoint` must still pass, and
  Task 12 re-runs it as a gate item rather than trusting that nothing moved.
- **`knowlu-engine coursework` and `coursework-discover` always exit 0.** No `config/cloud.yaml`, no
  session, no entitlement, no network, a 402, a 429, a 5xx, a timeout, a refused host, a run that
  blew a budget — every one is a normal outcome reported as a named line, and `coursework-discover`
  additionally prints exactly one JSON object on stdout whatever happened. A non-zero exit sets
  `RunSummary.engine_ok = false` in the app's scheduler, which paints the tray amber and puts the
  slot into retry backoff twice a day forever. **The rule the relay adds to it:** *an empty parse is
  a failure, never an empty semester* still holds, and now holds server-side — `ingestHandler`
  already says `0 assignments parsed; treating as failure` and C5 changes nothing about that.
- **No secret in the repo, a log, a fixture, a test name, a migration, a commit message or this
  plan.** No portal password, no session cookie, no captured token and no bearer is ever printed,
  logged, put in an error message, put in a `relay_runs` row or sent to the cloud. Every borrowed
  string that reaches a device line goes through `relay::scrub` and `judge::one_line`; every
  server-side log line is the error's **class**, never its message. The anon key and the project URL
  are public and are the only cloud values any argv or config carries.
- **No test reaches the network.** A `TcpListener` (Rust) or `Deno.serve` (TypeScript) bound to
  `127.0.0.1:0` inside one test, answering that same test's own request, is not egress: no DNS, no
  route off the machine, no listener on a routable interface. Every such test binds `127.0.0.1`,
  never `0.0.0.0`, and **joins its listener thread before returning**. This stream stands up **two**
  kinds at once — one standing in for `/relay` and one standing in for a portal — and both are
  joined. The `#[cfg(test)]`-only allow-list row `{name: "loopback", hosts: &["127.0.0.1"]}` exists
  for the portal side and **cannot escape into a real build**, which is exactly what
  `engine/tests/relay_allowlist.rs` (an *integration* test, so it links the lib without
  `cfg(test)`) is for.
- **Every test that aims at a loopback portal lives in `engine/src/relay.rs`'s own
  `#[cfg(test)] mod tests`** (review C1; spec §9's first bullet, and `coursework.rs:3095-3160` is the
  precedent it cites). The reason is the row above: Cargo compiles the library **without**
  `cfg(test)` for an integration test, so a portal suite in `engine/tests/` would see exactly what
  `relay_allowlist.rs` asserts it sees — `unknown_source` — and every portal test would fail.
  `engine/tests/relay_allowlist.rs` is the one integration test this stream adds, and it is the
  guarantee rather than a harness. **There is no `engine/tests/relay_contract.rs`.**
- **Never launch `knowlu.exe` against a real profile, and never point an engine command at a real
  vault.** Quinn's own vault, whatever `profiles.json` names, is never a target; tests copy a
  fixture vault into a temp
  directory, and the exit gate's live run uses a **scratch profile** and a scratch vault
  (`scripts\scratch-vault.ps1`), driven by the controller with Quinn at the machine.
  Desktop safety (`CLAUDE.md`): never synthetic keyboard or mouse input, screenshots by `PrintWindow`
  only.
- **Every child process spawns with `.no_console()`** (`knowlu_engine::childproc::NoConsole`).
  `engine/tests/no_console.rs` scans every non-test `engine/src/*.rs` and compares `Command::new`
  counts against `.no_console()` counts. **`relay.rs` spawns nothing**, and the two files C5 deletes
  spawned nothing either, so the floor C3′ left at `with_spawns >= 2` does not move — Task 10
  asserts that rather than assuming it.
- **TDD, per task: the failing test first, run it and watch it fail with the message the step names,
  the minimal implementation, run it and watch it pass, commit.** A step that shows implementation
  before its test is a plan defect — stop and report it.
- `cargo build --workspace` and `cargo test --workspace` from the root at **0 warnings**. The one
  accepted line is the app's `.rsrc merge failure: multiple non-default manifests` linker message;
  the gate line `warnings: N accepted (.rsrc), N tallies, N other` must end in `0 other`.
  **`cargo test --release` will not link** (`panic = "abort"` in the one release profile) — test in
  the dev profile. The four `#[ignore]`d tests stay ignored and none may be un-ignored by changing
  an assertion.
- **Line endings: LF everywhere** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` CRLF;
  `engine/tests/fixtures/** -text`). New and edited `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and
  `.md` files are LF, UTF-8, no BOM. `git diff --stat` never shows a whole-file flip.
  `.\scripts\ci\eol-check.ps1` is a gate item.
- **Two `deno test` command lines, and which is which.** A run over **C5's own files alone** is
  `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/relay/` —
  nothing C5 writes needs more, because every plan module is pure over an injected cursor and every
  handler test injects its dependencies. A run over **the whole tree** is C2's merged command and
  is **`.github/workflows/ci.yml:84`'s line, verbatim** (review M7 — CI is the gate, so the gate's
  command is CI's and not a paraphrase of it), and it must be copied exactly, because a narrower
  flag set fails on C2's tests rather than on C5's:

  ```
  deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=ANTHROPIC_WEBHOOK_SIGNING_KEY,ANTHROPIC_AUTH_TOKEN,ANTHROPIC_LOG,ANTHROPIC_CUSTOM_HEADERS --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
  ```

- **A third-party import in an edge function is an `npm:` or `jsr:` specifier on the import line**,
  never a bare name resolved through the root `deno.json` import map: the `--use-api` bundler does
  not read that file. C5 imports nothing third-party — only `@std/assert` (test-only), the
  platform's own `crypto` for nothing at all, and **two relative imports outside its own directory**:
  `../_shared/{entitlement,http,db}.ts` and `../ingest-coursework/handler.ts`. The second is the
  only novel thing here, and it is the same mechanism every function already uses for `_shared/`:
  `--use-api` bundles the whole module graph, not one directory.
- **Migrations are applied to the STAGING project only** (`brvhgbihxevrudqpulcm`), by the
  **controller**, never by an implementer subagent, and always with **`supabase db push
  --include-all`** — without the flag the CLI skips a migration stamped before the last applied one,
  and four streams now interleave days. Nothing in this plan touches `knowlu-prod`; Task 12 records
  what production still needs.
- **Migrations are stamped `cloud/supabase/migrations/20260918……_<name>.sql`.** C1 owns 2026-09-10,
  C2 owns 2026-09-11 and 2026-09-16, C3′ owns 2026-09-12, C1b owns 2026-09-17. **C5 owns
  2026-09-18**, and uses exactly one file.
- **The corpus-wide pins in `cloud/supabase/migrations/migrations_test.ts` are bumped by whichever
  stream moves them** (R-C3-exec-4), with the reason in the comment, in that stream's own commit.
  **C5 creates no function and no view**, so it expects to move **neither** pin. **The two numbers
  are not quoted anywhere in this plan** (review M6): they are C3′'s to move and this stream forks
  after it, so the rule is *unchanged from the branch point* and Task 5 step 10 records the observed
  pair. A literal here that disagreed with the branch would invite an edit to a file C5 does not own. C5's own
  guards live in `cloud/supabase/migrations_relay_test.ts`, filtered to `20260918…` (R-X-8's rule,
  applied from C5's side).
- **Tests that touch the real Credential Manager are serialised.** Windows races parallel
  `CredWriteW`/`CredReadW` calls (spurious `ERROR_NOT_FOUND`). Every **engine** test in this plan
  avoids the real store entirely: `wincred::read_credential` is reached only through a seam
  (`relay::Secrets`), and every test supplies its own. **There is exactly one exception**, added by
  ruling **R-C5-plan-4**: `app/tests/commands.rs`'s case for `save_portal_login`, which must prove
  the command writes the target `credentials::target_for` computes. It takes the file-scoped
  `CREDMAN_LOCK` (`app/tests/account.rs`'s, or its own if the file cannot reach it), uses a
  generated test id, and carries a `Drop` guard that deletes what it wrote (`CLAUDE.md`).
- **Commits:** specific `git add` (**never `git add -A`**), the message through a file
  (`git commit -F <file>`), and both trailers. The two literals below are **the writing session's**;
  the executing session substitutes its own model name and session URL, exactly as `HANDOFF.md` §5
  says (ruling R-C2-6 — this is not a defect to fix, it is a template to fill):

  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
  ```

- Never bare `git stash` / `git stash pop` (ruling R-3a-29): the stash stack is shared with the main
  checkout and every worktree.
- **File ownership is binding. C5 owns:** `cloud/supabase/functions/relay/**`,
  `cloud/supabase/migrations/20260918000100_relay.sql`,
  `cloud/supabase/migrations_relay_test.ts`, `engine/src/relay.rs` (new),
  `engine/src/coursework.rs`, `engine/src/zybooks.rs` and `engine/src/vhl.rs` (both deleted),
  `engine/Cargo.toml`, `engine/tests/relay_allowlist.rs` (new; **and no
  `engine/tests/relay_contract.rs` — review C1**), `engine/tests/dependency_boundary.rs`,
  `engine/tests/no_console.rs`, `engine/tests/site.rs`, `site/privacy.html`, and this plan file —
  plus **one line each** in `engine/src/cloudmodel.rs` (Task 1: `zybooks::scrub` becomes
  `relay::scrub`) and `app/src/account.rs` (Task 11: `PRIVACY_VERSION`), two files C2 and C1 owned
  and whose ownership rows retired with their merges; **one call** in `app/src/onboarding.rs`
  (Task 8, ruling **R-C5-plan-1**: saving a password closes that source's login card); **two named
  edits** in `cloud/supabase/functions/ingest-coursework/` — `handler.ts` (Task 5, ruling
  **R-C5-plan-2**: the per-source policy layer is *extracted* so the relay and `ingestHandler` share
  one copy) and `parse_vhl.ts` (Task 7, review **I8**: `sectionMapping` becomes an export and
  `parseDashboard` calls it) — both of which leave `ingestHandler`'s own behaviour byte-identical,
  pinned by its existing suite and by the two frozen parsed references; and **tests only** in
  `app/tests/{scheduler,onboarding,static_assets}.rs`, which exercise three hand-off files the
  controller owns; and — ruling **R-C5-plan-4** — `app/src/commands.rs` (one command),
  `app/static/index.html` and `app/static/console.js` (one settings row) with their tests in
  `app/tests/{commands,static_assets}.rs`. **`Cargo.lock` is NOT claimed** (review M12): it is a shared-single-owner file
  (`HANDOFF.md` §2) and enabling a `windows` feature adds no package, so it is expected not to
  change — which exit-gate item 12 already asserts. Everything else —
  `engine/src/{lib,main,cli}.rs`, `app/src/{scheduler,onboarding,main}.rs`,
  `cloud/supabase/config.toml`, `cloud/supabase/functions/ingest-coursework/**`,
  `scripts/*.py`, `CLAUDE.md`, `HANDOFF.md` — is the controller's and appears under **Controller
  hand-offs** with exact code. **A task that silently edits one of those files is a plan defect** —
  stop and report it instead of editing.

---

## Quinn-owned preconditions (asked one at a time, when a task reaches them, with the context)

Six. **Q1-Q5 are the spec's §10 table verbatim**; **Q6** is not one of the five — it is the privacy
wording §5 and §11 name as "drafted in the C5 plan, read by Quinn and the lawyer before merge", and
it is carried here as a precondition because a published promise that lags the code is the one defect
this stream could ship that no test can catch. **Every one of them is built to the spec's own
recommendation meanwhile**, so no task is blocked waiting for an answer; what an answer changes is
named in the row, and the answer is recorded in that task's report.

| # | Needed by | The question, the recommendation the plan is built to, and what changes on a different answer |
|---|---|---|
| **Q1** | Task 5 (the run row), re-read at Task 12 | **Raw-page retention.** Zero beyond the run, or a short diagnostic window (say 24 h on a parse failure) so a vendor markup change can be fixed from the page that broke? **Built to: zero.** The diagnostic path already exists and is consented — the issue report, with its preview-and-scrub screen (`app/src/report.rs`). A page body never enters `relay_runs.cursor`, `relay_runs.parsed`, Storage, a table or a log, and `cursor_holds_no_page_body_after_any_step` plus `parsed_holds_only_parsed_rows` are what keep it true. **What does persist for the length of a run is the parsed rows** — which is what R4-19 says may persist, and is the whole of ruling **R-C5-plan-2**: the cloud parses each body as it arrives and keeps the rows, never the page. **On "a window":** a `relay_pages` table with an `expires_at` and its own RLS, a purge entry in `account/index.ts`, a sentence in the privacy policy and a row in §9's data inventory — a day's work and a policy change, not a flag. |
| **Q2** | Task 8 (the login card) | **Does a rejected login pause the source** until the student saves a new password, or does every slot try again? **Built to: pause, and the pause ends when you save a new password** — which is the half the first draft of this plan left out (review **C2**, settled by ruling **R-C5-plan-1**). After one vendor rejection Knowlu opens the one info card of spec §4 and stops trying that source; **the moment the student saves that portal's password in the app, the app closes the card and the source is live again on the next slot**. What must be said to Quinn is exactly that sentence, because "pause" with no answer to "what re-arms it?" is "stop forever", and the card body promises the opposite. It is the legal note's *"stop on the first sign of a vendor block and never retry through a change of identity"*, and a daily retry with a wrong password is how an account gets locked. **On "retry every slot":** Task 8 step 6's paused-source check comes out and the card becomes advisory; the card and the close-on-save both stay either way. |
| **Q3** | Task 1 (the table) | **VHL's host row**: the two exact hosts ruling 4 names (`www.vhlcentral.com`, `m3a.vhlcentral.com`), or `*.vhlcentral.com`? **Built to: the two exact hosts, as ruled.** Widening is a one-line diff plus a test edit if VHL ever renumbers `m3a`; it is flagged so the choice is made knowingly rather than discovered on a broken slot at 07:00. **On "widen":** one line in `PORTAL_SOURCES` and three assertions in `relay_allowlist.rs`, and the spec's §3 argument gains a sentence. |
| **Q4** | Task 3 (the session store) | **The TTL for a captured value** — zyBooks' bearer token: 12 h as proposed, or shorter? **Built to: 12 h** (`ttl_s: 43200`), because two slots a day means a token is used at most twice before it is re-minted. **On "shorter":** one number in `plans/zybooks.ts`; the device honours whatever the plan says and pins nothing. |
| **Q5** | Task 4 (the budgets) | **The run budget of 10 minutes against the scheduler's 20-minute child timeout** (`app/src/scheduler.rs:27`). **Built to: confirm 10.** It is the number that guarantees the relay is never what the scheduler kills, so a relay that overruns says so in a warning instead of dying as an amber tray with no explanation. **On a different number:** one `const` in `relay.rs` and one assertion in its own test module; anything at or above 20 minutes is refused by the test that compares the two — and, since review **C4**, the deadline is also *enforced*, so the number means something. |
| **Q6** | Task 11, **before merge** | **The two rewritten privacy sentences**, drafted in Task 11 step 1, read by Quinn **and the lawyer** (with the C1 packet's P5 list, `docs/reports/2026-09-10-c1-quinn-packets.md`). `site/privacy.html:38`'s *Your coursework logins* `<dd>` and `:24`'s *Coursework logins* bullet under *What stays on your machine* both stop being true the moment `/relay` composes a request, because today's sentence says Knowlu "signs in to zyBooks and VHL from your PC" and after C5 our servers decide what that sign-in is. **Without it:** Tasks 1-10 and 12 all land and the branch does not merge — a published policy describing a mechanism the code no longer has is the one thing on this list that is not recoverable by a later commit. |

Nothing in this stream needs a new Supabase project secret, a new Stripe object, a new Google scope
or a new DNS record. It **does** add one Tauri command and one settings row (ruling
**R-C5-plan-4**), and it writes the Credential Manager entry the wizard already writes — the same
`knowlu/<profile_id>/<source>` target, from a second screen — rather than a new one. **`OPENROUTER_API_KEY` is
not touched and is not reachable from here**: the relay carries no judgment.

---

## Fidelity ledger

One row per decision, ruling sentence or inherited resolution C5 carries or narrows, with the
narrowing on the record and its reason. The spec's own §12 answers to ruling 4; **this table answers
to the spec**, and repeats ruling 4's rows only where this plan narrows the spec further.

| # | Decision / sentence | Source | Carried by |
|---|---|---|---|
| **C5-D1** | One endpoint, `POST /relay`, drives a run as a **batch-at-a-time step protocol**; the device never composes a request of its own | spec §2, C5-D1 | Tasks 4, 5. The device's only composition is **substitution into a step the cloud sent**; there is no code path in `relay.rs` that builds a URL, a header name or a body shape of its own, and `a_device_never_invents_a_request` scans the module for a `https://` literal outside `PORTAL_SOURCES`' patterns and finds none. |
| **C5-D2** | The host allow-list lives in the **engine** (`knowlu_engine::relay::PORTAL_SOURCES`), not the app, and the app imports it | spec §3, C5-D2; ruling R4-12 **moved** | Task 1. `app/` already depends on `knowlu_engine`, so there is one table and no copy. **Narrowed on the record:** ruling 4 said "a compiled-in table in the app"; the enforcement point must be the process that reads Credential Manager and opens the socket, and a table handed from the app to the engine over a command line is data, not a guarantee. The app's only use of it is the wizard's panel (Task 9), and `every_source_the_wizard_offers_is_in_the_table` is what keeps the two in step. |
| **C5-D3** | Every relayed request is checked against the table — **placeholder or not**, and on every redirect hop, which the device follows itself | spec §3, C5-D3; ruling R4-11 **widened** | Tasks 1, 4. A request with no placeholder still travels with the source's cookie jar, and a session cookie sent to a host of the server's choosing is the same leak by a slower route. The check runs **before the credential is read**, so a refused host never touches a password even in memory. |
| **C5-D4** | A per-source session store on the device holds the recorded `Set-Cookie` lines and plan-named captured values, DPAPI-protected, never in the vault | spec §4, C5-D4 | Task 3. `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\sessions\<source>.bin`, `CryptProtectData` with `CRYPTPROTECT_UI_FORBIDDEN` at current-user scope. **Never in the vault**, because the vault is plain text and under ruling 2 syncs to the account — a cookie must not. **Narrowed:** Credential Manager is not used, and the reason is a number: its blob cap is 2,560 bytes and a realistic CAS jar is larger. |
| **C5-D5** | A value the plan marks `capture` is kept **on the device** and redacted out of the body returned to the cloud | spec §2.3, §4, C5-D5 | Tasks 2, 4. zyBooks' `session.auth_token` is password-equivalent for its lifetime — the legal note's *Avoid* list names storing session material server-side — so the device captures it by JSON pointer, keeps it, and (because `redact: true`) replaces it in the body it returns with `"<captured:zybooks_token>"`. The cloud still reads `success` and `user.user_id` out of that body, as it must. |
| **C5-D6** | `Set-Cookie` is stripped from every response the device returns; the cloud never sees a session cookie | spec §2.4, §5, C5-D6 | Task 4, and `no_set_cookie_header_ever_reaches_the_reply`. The device's jar does the sending; the cloud has no use for a cookie value and no business holding one. |
| **C5-D7** | No new engine command: `relay.rs` is a library driven from inside `coursework` and `coursework-discover` | spec §7, C5-D7 | Tasks 8, 9, and hand-off **H4** adds **no** subcommand. A `knowlu-engine relay` subcommand would be a general-purpose *make this request with my saved password* tool sitting on the student's machine and in their process list; and the slot's step vocabulary is a contract with `slot_argv`, the run records and the tray. |
| **C5-D8** | Fetch plans are versioned Deno modules per source; the run's own state is a `relay_runs` row that never holds a page body or a secret | spec §5, §6, C5-D8 | Tasks 5, 6, 7. One row, one cursor, a 15-minute TTL, swept at the top of every call. |
| R4-1 | Credentials never leave the machine; the cloud never holds a portal password | ruling 4 | Tasks 2, 4, 10 — substitution is the only touch, and after Task 10 there is no column, no field and no code path for one. `the_relay_payload_carries_no_credential` is the successor to C2's `the_coursework_payload_carries_no_credential` and is written in Task 4. |
| R4-8 | A short sequence inside one slot, driven by the device's scheduler; nothing fetches while the laptop is closed | ruling 4 | Task 4's budgets (40 steps, 24 round trips, 10 minutes) and Task 8's placement inside `coursework`, which is a slot step and nothing else. |
| R4-15 | "Adding or repairing a portal is a cloud change with no release and no user action" | ruling 4 | **Narrowed, and the narrowing is the spec's (§6):** *repairing* a portal — the flow, the URLs, the order, the parser, the oracle, the reconcile — is a deploy. What still costs a release is **the allow-list row** and **the wizard panel that captures that source's login**. §3 argues the price and Task 1 records it in the module doc, so the next person to add a portal reads it before they discover it. |
| R4-17 | The device keeps each source's jar between slots; re-authenticate only on expiry | ruling 4 | Task 3, **by recorded `Set-Cookie` replay** rather than `ureq`'s own serialisation — and, since review **I11**, by a session report the cloud can actually decide on: per-cookie **expiry** with no name and no value (`{"cookies": {"count": N, "session_cookies": M, "earliest_expiry": <unix or null>}}`), with expired entries pruned on `load` and on `observe` so the count means something. A bare count could only ever be guessed at, and for VHL — where the session *is* the cookie and there is no capture at all — guessing means being served the login page again on every run, which works and costs a round trip and makes this row an overstatement. `null` is honest: the cloud then knows it must try and may be wrong. Two measured reasons, both in Task 3 step 1's test: `CookieJar::save_json` writes only *persistent* cookies and a CAS session cookie has no `Expires`/`Max-Age`; and `CookieJar::iter` exposes only `name` and `value`, losing the `Domain` attribute that makes one jar span `www.` and `m3a.vhlcentral.com`. Replaying the vendor's own bytes through `Cookie::parse(line, &uri)` + `jar.insert` puts RFC 6265's rules back where they were and needs no `json` feature. |
| R4-19 | Raw pages: received for the run, parsed, not retained; the parsed rows persist; the policy says so | ruling 4; Q1 | Tasks 5, 11. Zero retention, and the policy sentence is Q6. |
| R4-20 | `coursework-discover` becomes a relayed cloud job over the same contract | ruling 4 | Task 9, **plus three pre-vault flags** the wizard needs (`--cloud-base`, `--anon-key`, `--session-target`) because the wizard runs before the vault exists and so before `config/cloud.yaml` does. All three carry a public value or a target *name*, never a secret — exactly as `discovery_argv` already passes target names (`app/src/onboarding.rs:74`). |
| R4-21 | The legal posture is unchanged: the request originates from the device, with the user's credentials, at the user's instruction; the vendor sees the student's IP and session | ruling 4; spec §8 | Task 9 and hand-off **H6** — **corrected by review I10**. The spec said the wizard's one-sentence ToS disclosure "stays", gaining a clause; the checkout has no such sentence. The coursework panel (`app/static/index.html:131-141`) carries a *credential-storage* lede and nothing about the vendor's terms, and no test pins one. So C5 is **writing** that disclosure, not extending one — which makes it legal copy, which puts it in front of Quinn and the lawyer with **Q6** rather than in a controller's hand-off note. H6 carries the whole sentence once Quinn has it; Task 9 pins both halves; the spec's §8 is corrected in the same round. |
| Ruling 2 | Raw pages are the student's data | amendment ruling 2 | Task 5's `cursor` guard and Task 11's privacy sentence. The account's copy of the *vault* is C3′'s and is not touched. |
| Ruling 3 | The engine refuses a slot without entitlement past the 72-hour grace | amendment ruling 3 | **Inherited from C3′ Task 8, not re-decided.** `coursework` is already behind `entitle::gate`, so a relay run cannot start without one; and `/relay` is `requireActiveEntitlement`-gated like every other function, so a 402 mid-run is `CloudError::fatal()` and ends the run with `no entitlement` and nothing changed. Task 8 step 6 asserts both halves. |
| Ruling 5 | C5 is "the contract above, replacing the on-device fetchers", after C3′ and before C4 | amendment ruling 5 | The status block, and Task 10. C5 does not start before C3′ merges, and C5 does not touch `engine/src/runtime.rs` or `app/src/inference.rs`, which are C4's. |
| `CLAUDE.md` | Approvals are capped at 15 new proposals a day; overflow is snoozed, never deleted | `CLAUDE.md`; `approvals::defer_over_budget` | Unchanged. Map cards still come back in `done.proposals` and still go through `propose_map_cards` and `asked_map_keys` (`coursework.rs:815`, `:952`), so the cap applies exactly as it did. |
| `CLAUDE.md` | An empty parse is a failure, never an empty semester | `CLAUDE.md`; `ingestHandler` | Unchanged and already server-side. Tasks 6 and 7 add one sibling rule the relay makes possible: **a step the vendor answered with a login page is a session failure**, and the plan says so by composing the login steps once and retrying that step **once** — never twice (spec §4). |
| §8 | `dependency_boundary.rs` pins the engine's budget; TLS is rustls/ring, never OpenSSL; no cloud SDK | cloud design §8 | Task 1 and Task 10. **C5 adds no crate.** The `base64` refusal C3′ installed stands, and `relay.rs` carries its own 20-line encoder with a test against RFC 4648's own vectors; `the_relay_adds_no_crate_and_no_second_http_client` is written in Task 1 and re-run in Task 10. |
| §5.6 | Judgment logs never enter the vault | cloud design §5.6 | Unchanged, and widened in spirit: **no relay log enters the vault either**. The session store is under app data, the run log keeps its shape, and `state/runner-log.md` gets the same one-line-per-source warnings it gets today. |

---

## File structure

### What this plan does *not* move, and why — read this before Task 1

Five things a reader will look for and not find, each with its argument in the fidelity ledger or the
spec:

1. **`/ingest-coursework` is not retired and its behaviour does not change.** The endpoint stays
   public through C5 and is retired only when no shipped client posts to it — a decision the spec
   explicitly leaves open (§11). **What does change is where its per-source policy lives** (ruling
   **R-C5-plan-2**): because the relay parses each body as it arrives rather than holding raw pages
   across round trips, `pickZybooks`'s loop body, the VHL unmapped-section-to-proposal conversion
   and the `{name}: ` prefixing are **extracted** out of `ingestHandler` into three exported
   functions that `ingestHandler` then calls. Its reply is byte-identical and its own suite plus the
   two frozen references are what prove it — an extraction, not a rewrite, and not a second copy.
2. **The parsers do not move again.** `parse_zybooks.ts` and `parse_vhl.ts` are C2's and are pinned
   against the two frozen references. Two edits, both named: the comment correction in
   `parse_zybooks_test.ts` that Task 6 makes and hand-off **H7** carries (the BOM now travels; the
   test's own comment says it never does), and `parse_vhl.ts` gaining an exported
   `sectionMapping(sections, sectionId)` that `parseDashboard` itself calls (review **I8**), so
   `vhlRows` and the parser share one predicate instead of two readings of "is this section
   mapped" — the rule `vhl::section_mapping` (`engine/src/vhl.rs:300`) already stated and Task 10
   deletes.
3. **No `app/src/relay.rs`.** *(The first draft also said "no new Tauri command, no new settings
   row". Ruling **R-C5-plan-4** reverses that half: review **R1** found that the pause's exit was a
   wizard-only command with no post-onboarding caller, and that a student who changes a portal
   password had no way to tell Knowlu at all. C5 therefore adds **one** settings row and **one**
   console command — hand-off **H11** — and the counts move with them.)* The app's other change is the
   session directory on one argv, the wizard's discovery argv, and one disclosure sentence. The
   command counts **move by exactly one** — `commands::save_portal_login`, on the **console** window
   — and hand-off **H11** recounts both lists rather than quoting a number, because two streams
   merge ahead of this one. For orientation only, not to be copied into a document: at the time of
   writing `main` has 30 (picker/wizard) and 43 (console), and C1b — which merges before C5 — takes
   them to 29 and 42 by deleting a password command. **After C1b**, C5 makes them **29 and 43**;
   the recount is H11's own first step and the number that goes in `CLAUDE.md` is the one it
   observes.
4. **`engine/src/runtime.rs` and `app/src/inference.rs` are untouched.** They are C4's, and
   `CLAUDE.md` says that until C4 lands that code stays and is not extended. `SUPPORTED_RUNTIMES` is
   read in this plan only as the **pattern** `PORTAL_SOURCES` copies (`app/src/inference.rs:219` and
   `app/tests/inference.rs:177`), never edited.
5. **`engine/src/backup.rs`, `engine/src/sync.rs` and `engine/src/entitle.rs` are untouched.** C3′
   owns all three; C5 reads `entitle`'s outcome only by virtue of running after it in `main.rs`.

### New — the service

- `cloud/supabase/migrations/20260918000100_relay.sql` — one table, `public.relay_runs`, its RLS and
  its index. No function, no view, no trigger, no cron job: the sweep is one `delete` at the top of
  every call.
- `cloud/supabase/functions/relay/index.ts` — `Deno.serve(relayHandler(requireActiveEntitlement))`,
  the four-line shape every function in this codebase has.
- `cloud/supabase/functions/relay/protocol.ts` — the wire types and the validators, and the one
  place a step is proved well-formed before it is sent. **Not in `_shared/`**: C5's contract has
  exactly one consumer, and a `_shared/relay_*.ts` would invite a second.
- `cloud/supabase/functions/relay/protocol_test.ts`
- `cloud/supabase/functions/relay/handler.ts` — the protocol, the budgets, the run row, the plan
  registry, and the hand-off into `ingestHandler`.
- `cloud/supabase/functions/relay/handler_test.ts`
- `cloud/supabase/functions/relay/db.ts` — the four PostgREST calls this function makes, over C1's
  `Rest`. The only database access C5 adds.
- `cloud/supabase/functions/relay/plans/zybooks.ts`, `plans/zybooks_test.ts`
- `cloud/supabase/functions/relay/plans/vhl.ts`, `plans/vhl_test.ts`
- `cloud/supabase/functions/relay/plans/mod.ts` — the registry (`name → Plan`) and the shared
  `Plan` interface, so `handler.ts` names no source and adding one is a new file plus one line.
- `cloud/supabase/migrations_relay_test.ts` — C5's own static guards over `20260918…`, the shape
  C3′'s `migrations_sync_test.ts` established.

### New — the device (engine)

- `engine/src/relay.rs` — `PortalSource`, `PORTAL_SOURCES`, `host_allowed`, the substitution and its
  four encoders, `scrub`/`quote`/`quote_plus`/`json_escape_ascii` (moved, not rewritten), the base64
  encoder, `SessionStore` and its DPAPI seal, `Secrets` (the Credential Manager seam), `perform` (one
  step, with its own redirect follower), `run` (the loop), and the budgets. **One file**, because
  every piece of it is one guarantee and splitting them would put the host check in a different
  module from the socket it guards.
- `engine/tests/relay_allowlist.rs` — an **integration** test, so it links the lib **without**
  `cfg(test)` and the test-only `loopback` row cannot be what it sees. The table is exactly two rows
  with exactly the ruled hosts; every pattern is well-formed; the matcher accepts and rejects the
  named hosts; every `name` is a legal credential-target suffix; the wizard offers no source the
  table does not carry.
**And no second integration test.** The device end of the protocol — every request shape it sends,
every reply it tolerates, the budgets, the redirect chain, the capture redaction and the
data-minimisation assertions (no credential, no cookie, no captured value, no app-data path, no
vault path) — lives in `engine/src/relay.rs`'s own `#[cfg(test)] mod tests`, because those tests aim
at a loopback portal and the row that permits one exists only under `cfg(test)` (review **C1**; spec
§9). `coursework.rs:3095-3160` is the precedent, and it is in-module for the same reason.

### Modified — the device

- `engine/src/coursework.rs` — `collect_cloud` calls `relay::run` instead of
  `zybooks::fetch_payloads`/`vhl::login_and_fetch_dashboard`; `discover_json` becomes a relayed job;
  `fetch_zybooks`, `fetch_vhl`, `collect`, `Fetcher`, `route_zybook`, `BookRouting`, `zybooks_rows`,
  `vhl_rows` and `main_with_fetchers`'s fetcher seam are deleted (Task 10), **and so are
  `coursework_request` (`:639`), `FetchedSource` (`:630`) and the POST half of `post_coursework`
  (`:670`)** — review **I7**: after Task 8 nothing outside `#[cfg(test)]` calls them, and an item
  reachable only from a test module is a `dead_code` warning under a plain `cargo build --workspace`,
  which is the gate this stream must keep at zero. `post_coursework` is **split**: the POST wrapper
  goes, and its reply decoder stays as `decode_coursework_reply`, which is what `collect_cloud` calls
  on `done`. **Stays:** `Assignment`, `SourceError`, `sync_coursework`, `load_coursework_config`,
  `resolve_timezone`, `redact`, `yaml_to_json_for_request`, `assignment_from_row`,
  `decode_coursework_reply`, the map-card machinery, `FAILURE_MARKERS`, `rank_warnings`.
- `engine/Cargo.toml` — the `windows` dependency gains **one feature**,
  `Win32_Security_Cryptography` (its features are internal to that crate, so no package is added);
  the `ureq` `cookies` comment is rewritten to name `relay.rs`. No crate is added and none is
  removed, and **`Cargo.lock` is expected not to change at all** (review M12) — exit-gate item 12 is
  where that is checked, not claimed.
- `engine/src/lib.rs` — `pub mod relay;` added; `pub mod zybooks;` and `pub mod vhl;` removed
  (hand-off **H1**, two parts).
- `engine/src/main.rs` — `coursework` gains `--session-dir`; `coursework-discover` gains
  `--cloud-base`, `--anon-key`, `--session-target` (hand-off **H4**, two parts).
- `engine/src/cli.rs` — verified at Task 12, not edited (hand-off **H5**).
- `engine/tests/dependency_boundary.rs` — one new case, and the `ureq`-only sentence re-stated for
  the relay (Task 1); re-run at Task 10.
- `engine/tests/no_console.rs` — **verified, not edited**: the floor C3′ left at `>= 2` still holds
  because neither deleted file spawned a child (Task 10 step 5 asserts it).

### Modified — the app

- `app/src/scheduler.rs` — the `coursework` arm of `slot_argv` gains `--session-dir`
  (hand-off **H2**).
- `app/src/onboarding.rs` — `discovery_argv` gains the three pre-vault flags
  (hand-off **H3**).
- `app/static/index.html` — one sentence in the coursework panel's ToS disclosure
  (hand-off **H6**).
- `app/tests/{scheduler,onboarding,static_assets}.rs` — the three hand-offs' own tests, written **by
  the tasks** and committed with them; the source files they exercise are the controller's.
  *(This is the one place C5 writes a test for a file it does not own; it is called out here and
  again in each hand-off so the ownership scan at Task 12 does not read it as a violation.)*
- `cloud/supabase/config.toml` — one `[functions.relay] verify_jwt = false` entry
  (hand-off **H8**).
- `cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts` — one comment corrected
  (hand-off **H7**).
- `cloud/supabase/migrations/migrations_test.ts` — **verified, not edited**: C5 creates no function
  and no view, so neither pin moves (Task 5 step 6).
- `site/privacy.html` — two sentences, under **Q6**. **C5's own file.**

### Deleted

- `engine/src/zybooks.rs` (1,254 lines, parser + network + its own test module) — Task 10. Its **parser** left
  for `parse_zybooks.ts` in C2; its **network tests** are ported one-for-one into
  `plans/zybooks_test.ts` in Task 6 **before** this deletion, which is why Task 10 comes after Tasks
  6 and 7 and not before them.
- `engine/src/vhl.rs` (1,275 lines, parser + network + its own test module) — Task 10, same
  argument, with
  `parse_user_session_form`, `first_dashboard_link` and `discover_sections` ported into
  `plans/vhl.ts` in Task 7.

---

## Interfaces with C1, C2 and C3′

**Ten contracts, checked against the checkout on 2026-09-17** (`main` at `84991bc`, plus the C3′
outcomes named in the status block). Use these spellings verbatim; changing one is a conversation,
not an edit.

**1. `config/cloud.yaml`** — written into the vault by C1's `scaffold::create_vault` at onboarding,
absent on a vault that has never signed in. Exactly four keys, in this order:

```yaml
api_base: 'https://<ref>.supabase.co/functions/v1'
anon_key: '<the project anon key — public>'
session_credential_target: 'knowlu/<profile_id>/session'
account_id: '<uuid>'
```

C5 **adds no fifth key**, exactly as C3′ added none. `relay::profile_id_of` derives the profile id
from `session_credential_target` — the middle segment of `knowlu/<profile_id>/session` — which is the
same documented derivation C3′'s `entitle::profile_id` already uses, and is why the session
directory needs no new configuration on the engine side at all when the app passes `--session-dir`.
Read through `cloudmodel::load` (`engine/src/cloudmodel.rs:88`), which never fails: a missing,
unreadable or incomplete file is `None`, and `coursework` then runs its local path — which after Task
10 is *no path at all*, and says so (Task 8 step 4).

**2. The session credential is a JSON object, not a bare JWT.** Windows Credential Manager, target
`knowlu/<profile_id>/session`. `UserName` is the `account_id`; the blob is
`{"access_token","refresh_token","expires_at","email"}`, `expires_at` in Unix seconds. **Refresh is
C1's job** (`account::valid_access_token_at`). C5 reads and never writes it, through
`cloudmodel::resolve`, and never sees it at all outside that call.

**3. The portal credential is Credential Manager's, read through the engine's own reader.**

```rust
// engine/src/wincred.rs:145
pub fn read_credential(target: &str) -> Result<Credential, CredError>;
pub struct Credential { pub username: String, pub password: Secret }
impl Secret { pub fn expose(&self) -> &str }   // no Display; Debug redacts
```

`Secret` has **no `Display`**, so `{}`-printing a password is a compile error rather than a leak.
`relay.rs` touches `expose()` in exactly **one** function (`substitute_one`), and
`the_password_is_exposed_in_exactly_one_place` (Task 2) scans the module and pins the count at one.
The target comes from the vault's `coursework.<source>.credential_target` — the same read
`fetch_zybooks` does today (`coursework.rs:468`) — and the app writes it as
`knowlu/<profile_id>/<source>` (`app/src/credentials.rs:17`).

**4. `cloudmodel::CloudClient` — C2's, reused, and C5 adds no second HTTP client.**

```rust
pub struct CloudConfig { pub api_base: String, pub anon_key: String, pub session_credential_target: String, pub account_id: String }
pub fn load(vault: &Path) -> Option<CloudConfig>;
pub enum Unavailable { NoConfig, NoSession(String) }            // .label() -> &'static str
pub fn resolve(vault: &Path) -> Result<CloudClient, Unavailable>;
pub enum CloudError { Transport(String), Status { code: u16, detail: String }, Body(String), Quiet(QuietReason) }
                                                                // .label(), .fatal()
impl CloudClient {
    pub fn new(cfg: &CloudConfig, token: &str) -> CloudClient;
    pub fn account_id(&self) -> &str;
    pub fn post(&self, path: &str, body: &Value) -> Result<Value, CloudError>;
    pub fn get(&self, path: &str) -> Result<Value, CloudError>;
}
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);
```

`post` serialises through `ledger::dumps_value`, sends `Authorization: Bearer <access token>` and
`apikey: <anon key>`, sets `http_status_as_error(false)` so a 401 and a 402 arrive as a **status and
a body** rather than an opaque transport error, and scrubs the bearer out of every error string it
produces (`cloudmodel.rs:252`, which calls `zybooks::scrub` today and calls `relay::scrub` after
Task 1). **C5 calls `post("/relay", …)` and nothing else.** `CloudError::fatal()` is true for 401,
402 and 403 — the three answers that would answer every remaining round trip the same way — and that
is what ends a run instead of retrying it.

**One thing C5 must widen, and it is the only change to a C2 file on the device.** `CloudClient::new`
hard-codes `CALL_TIMEOUT` (120 s). The relay's own per-step bound is **60 s** and its whole-run bound
is **10 minutes**, and the client that talks to `/relay` is the same client `judge` uses. 120 s per
round trip × 24 round trips is 48 minutes, which is past both the run budget and
`scheduler::CHILD_TIMEOUT`. **The fix is not a second client and not a changed default**: `relay::run`
enforces its own wall clock across the loop and stops at `RUN_WALL_CLOCK`, so a slow service costs a
named warning at ten minutes rather than a killed child at twenty. `CloudClient` is not edited at
all. **And a comparison of two constants is not that test** (review **C4**): `relay::run` carries a
monotonic `deadline: Instant` set at `RUN_WALL_CLOCK` from the run's first instruction, checks it
before every step, before every redirect hop and before every round trip, and hands
`budget.remaining()` to each request as its timeout instead of a fixed 60 seconds — so
`STEP_TIMEOUT` bounds a **whole step including its redirect chain**, not one HTTP request out of
six. Task 4 step 1 writes the behavioural test that pins it: a scripted portal that stalls past the
deadline ends the run as a `budget` warning at under ten minutes.

**5. `cloud/supabase/functions/_shared/entitlement.ts`** — C1's, imported by `relay/index.ts` and
called first:

```ts
export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }>;
```

It **throws a `Response`** — 401 on a missing or invalid bearer, 402 when the account has no `active`
or `trialing` entitlement — and so does `_shared/http.ts`'s `fail(status, message)`. **Nothing
catches it inside a `handle`, and that is C1's shape, not an omission** (C3′ review finding B2):
`index.ts`'s `asResponse(e)` returns it verbatim. **The consequence is a rule for every handler test
in this plan**: a case that expects a refusal awaits the rejection —

```ts
const res = await handle(req, deps()).catch((e) => e as Response);
```

— and every such case in Tasks 5, 6, 7 and 9 is written that way.

**6. `cloud/supabase/functions/_shared/db.ts`** — C1's PostgREST client, used verbatim:

```ts
export interface Rest { url: string; serviceKey: string; fetch: typeof fetch }
export async function restSelect<T>(rest: Rest, table: string, query: string): Promise<T[]>;
export async function restUpsert(rest: Rest, table: string, rows: unknown[], onConflict?: string): Promise<void>;
export async function restPatch(rest: Rest, table: string, query: string, patch: unknown): Promise<void>;
export async function restDelete(rest: Rest, table: string, query: string): Promise<void>;
export function restFromEnv(): Rest;
```

Two things this constrains: **the service role bypasses RLS**, so `account_id=eq.<id>` in every query
string *is* the access control, and Task 12's scan asserts every sync-table query in `relay/db.ts`
carries it; and **there is no raw `rest.fetch` in this stream** — the same zero C3′ holds itself to,
and asserted in the same way.

**7. `cloud/supabase/functions/_shared/http.ts`** — C1's, used verbatim: `json(status, body)`,
`fail(status, message)` (thrown at the call site), `methodNotAllowed(allowed)`,
`readJson<T>(req, limit?)` (throws 413 over the cap, 400 for non-JSON), `asResponse(e)`. Every error
body in this codebase is `{"error": "<one sentence>"}`. **`readJson`'s default cap is 1 MiB and C5
raises it for `/relay` only — to a number it derives rather than chooses** (review **I1**, which
settles the plan's own second open decision). The derivation, written into the code beside the
call: the device's whole-run raw budget is `MAX_RUN_BYTES` = 8 MiB, a non-UTF-8 body travels base64
at 4/3, so one POST's worst case is about 10.7 MiB and the cap is **16 MiB** — `readJson(req, 16 <<
20)`. **And the device gets a matching outgoing bound so a 413 is unreachable rather than merely
unlikely**: `RunBudget` charges the serialised size of the `results` body it is about to post, and a
step that would blow it is `too_large` on the device instead of a 413 from the service.
`a_results_batch_over_the_cap_is_a_413_not_a_500` pins **the relationship** between the two numbers,
not either one of them. One thing to say out loud because the next reader will assume otherwise:
`readJson` does `await req.text()` **before** it measures, and measures `String.length` in UTF-16
code units (`_shared/http.ts:48-52`) — so the cap is a sanity bound and **not** a memory guard.

**8. `cloud/supabase/functions/ingest-coursework/handler.ts`** — C2's, and **the one C2 file whose
body C5 changes**, by extraction rather than rewrite (ruling **R-C5-plan-2**, from review **C3**).

The relay cannot call `ingestHandler` with a whole request, because a coursework run's bodies arrive
across several round trips and an edge function holds nothing between invocations — so the relay
parses **each body as it arrives** and keeps only the parsed rows. That means the per-source policy
layer has to be reachable a piece at a time. Three exports come out of `ingestHandler`'s body, and
`ingestHandler` is rewritten in terms of them so there is one copy and not two:

```ts
// unchanged, still the public endpoint, still byte-identical in and out
export function ingestHandler(entitle: Entitle): (req: Request) => Promise<Response>;

// NEW, extracted verbatim from `pickZybooks`'s loop body (handler.ts:84-109)
export function ingestZybook(book: {code: string, payload: unknown}, cfg: Obj, timeZone: string,
                            warnings: string[], proposals: MapProposal[]): Assignment[];
// NEW, extracted verbatim from the VHL arm (handler.ts:137-163): parseDashboard, then the
// VHL_UNMAPPED-to-proposal conversion with its `seenSections` de-duplication
export function ingestVhl(html: string, cfg: Obj, warnings: string[],
                         proposals: MapProposal[]): Assignment[];
// NEW, extracted verbatim from the tail of the source loop (handler.ts:177-182): the `{name}: `
// prefixing and the `0 assignments parsed; treating as failure` rule
export function finishSource(name: string, items: Assignment[], own: string[],
                            warnings: string[]): Assignment[];
// NEW (review R7), extracted verbatim from the per-source try/catch (handler.ts:135, :168-176):
// `NotLoggedIn` becomes `<name>: session invalid (<msg>); nothing changed`, anything else becomes
// `<name>: parse failed (<msg>); nothing changed`, that source's `own` is DISCARDED, the proposals
// already pushed are KEPT, and the `0 assignments parsed` line is skipped.
// It returns `{source: name, items: [], own: [], proposals: <kept>, failed: "<the one warning>"}`,
// and `appendParsed` reads `failed` as this source's retirement (review S1).
export function sourceFailure(name: string, e: unknown, proposals: MapProposal[]): Parsed;
```

**The rule the fourth function carries, stated because incremental parsing changes when it fires**
(review **R7**). One `ingestHandler` call saw a whole source at once, so a throw retired it there
and then. The relay sees book 7 of 12 throw on a **later round trip**, with six books' rows already
in `parsed`. The rule, and Task 5 asserts each clause — including this one, which needs a source
that **succeeds on an earlier round trip and fails on a later one** and therefore cannot be tested
by a source that fails on its first step:

- a throw **retires that source for the rest of the run** — no further steps are composed for it,
  and the other source finishes (ruling R-C5-plan-3);
- the source's accumulated `own` warnings are **discarded** and its accumulated `proposals` are
  **kept**, which is what `ingestHandler` does and is not obvious;
- **its already-parsed items are discarded too**, because that is what a one-shot call did: a source
  that threw contributed nothing. Six books' rows do not survive book 7's dead session — and the
  **mechanism** is that `parsed.assignments` is a map keyed by source (review **S1**), so
  `delete parsed.assignments[source]` is the whole of it. A flat unsourced array, which is what the
  second round left here, cannot identify the failing source's rows and so cannot remove them: the
  clause had a rule and no way to keep it;
- the `0 assignments parsed` line is **not** added on top of the failure line, which is the endpoint's
  own `continue`;
- and a `NotLoggedIn` on a step the plan **recognises as re-authenticable** is `reauth` (review
  **I12**) and never reaches `sourceFailure` — the module decides that before it throws, which is why
  `reauth` names its source. A second `NotLoggedIn` after a re-auth does reach it.

`Entitle` is `(req: Request) => Promise<{ account_id: string }>` (`_shared/judge_handler.ts`) and is
still how the endpoint gates itself; the relay does not call the handler at all any more, so there
is no closure to get wrong and `the_parse_handoff_passes_the_already_resolved_account_never_an_open_gate`
is replaced by `the_relay_never_re_enters_the_public_endpoint` (Task 5), which asserts `handler.ts`
under `relay/` imports the three named functions and **not** `ingestHandler`.

**What keeps `done` byte-identical.** The four functions are moved code, not re-derived code, and
**`ingest-coursework/handler_test.ts` is the oracle** — seven cases, a dead session and a
one-source-fails among them. *The two frozen parsed references are **not** the oracle for this
change* (the author's concern 2, ruled): they pin `parseAssignments` and `parseDashboard`, which the
extraction does not touch, so they would stay green over a broken extraction. They are named here
only so nobody claims more of them than they cover; and the relay assembles `done` in the order the device
used to send — `PORTAL_SOURCES` order (zyBooks, then VHL), and within zyBooks the shelf order the
item list returned — so `assignments`, `warnings` and `proposals` come out in the same sequence a
single `ingestHandler` call produced. Task 5 asserts that against the fixture's 24 assignments.

**9. `app/src/scheduler.rs`'s slot, as C3′ leaves it.**

```rust
pub enum JudgePlan { Cloud { log_dir: PathBuf }, Local(JudgeArgs), Skip(&'static str) }
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)>;
pub enum IcsState { Feed, NoUrl, Unreadable }
pub const CHILD_TIMEOUT: Duration = Duration::from_secs(20 * 60);   // app/src/scheduler.rs:27
```

producing `sync → coursework → [ingest] → [judge] → rank`, each the sibling `knowlu-engine.exe`.
C5 adds **two arguments to one existing step** and no step (hand-off **H2**). `run_slot_inner` names
a step by `args[0]`, so the Runs view and the tray are unchanged.

**10. `app/src/onboarding.rs`'s discovery, as C1 left it.**

```rust
pub fn discovery_argv(id: &str, zybooks: bool, vhl: bool) -> Vec<String>;   // app/src/onboarding.rs:74
pub fn rows_from_discovery(json: &str) -> Vec<DiscoveredRow>;
pub fn errors_from_discovery(json: &str) -> Vec<String>;
#[tauri::command(async)] pub fn discover_coursework(vault: String, zybooks: bool, vhl: bool) -> Value;
```

`discovery_argv` already passes **target names and never secrets**, and its own doc comment says so —
that is the constraint C5's three new flags are written to keep (hand-off **H3**). `rows_from_discovery`
and `errors_from_discovery` read `{zybooks, vhl, errors}` and are **not** edited: Task 9's whole
point is that the JSON object on stdout keeps its shape while everything behind it moves.

---

## Controller hand-offs

Every change below is outside C5's file ownership. **No task in this plan edits these files.**

**How and when they are applied.** Every hand-off lands **on the branch, at the task that first needs
it, as that task's own separate commit**, applied **verbatim from the hand-off text** by the
controller and reviewed together with the task (C3′'s rule R-C3-8, inherited). It is not a merge-time
activity: a hand-off that waits until merge makes every task after it untestable, and the one
non-buildable intermediate state this plan has (H1b, at Task 10) is named as such.

Each entry carries **"applied at Task N"**. The order is the task order:

| Task | Hand-off applied with it |
|---|---|
| 1 | **H1a** — `engine/src/lib.rs`'s `pub mod relay;` (**compile-blocking**) |
| 5 | **H8** — `cloud/supabase/config.toml`'s `[functions.relay]` entry (nothing deploys without it) |
| 6 | **H7** — `parse_zybooks_test.ts`'s BOM comment |
| 8 | **H4a** — `engine/src/main.rs`'s `--session-dir`; **H2** — `scheduler.rs`'s `coursework` arm; **H11** — `app/src/main.rs`'s console handler list, **with the recount** (ruling R-C5-plan-4) |
| 9 | **H4b** — `engine/src/main.rs`'s three discover flags; **H3** — `onboarding.rs`'s `discovery_argv`; **H6** — `app/static/index.html`'s disclosure sentence |
| 10 | **H1b** — `engine/src/lib.rs`'s `zybooks`/`vhl` removal (**compile-blocking with the task's own deletions; the one non-buildable intermediate state**) |
| 12 | **H5** — `engine/src/cli.rs`, verified, not edited; **H9** — `CLAUDE.md` and `HANDOFF.md`; **H10** — `scripts/wizard-check.py`, verified, not edited |

`H1` and `H4` are split into lettered parts because their halves are needed at different tasks; each
part is a separate commit and each is written out in full below.

### H1a — `engine/src/lib.rs`, the addition (applied at Task 1; **compile-blocking**)

In the module-declaration block, **after `pub mod ranking;` (`engine/src/lib.rs:72`)**. The
declarations are not alphabetical — `runs` is at `:62` and `ranking` at `:72` — so the first draft's
"after `ranking`, before `runs`" named a position that does not exist (review **M3**), and a
hand-off is applied verbatim:

```rust
/// C5 (cloud design, amendment 2026-09-17, ruling 4): the credential-substituting HTTPS relay —
/// the compiled-in host allow-list, the placeholder substitution, the per-source cookie jar and
/// session store, and the step loop the cloud drives. A library, never a subcommand.
pub mod relay;
```

**Without it:** every test in Tasks 1-4 fails to compile with `unresolved import
knowlu_engine::relay`, and `engine/tests/relay_allowlist.rs` cannot see the table at all.

### H1b — `engine/src/lib.rs`, the removal (applied at Task 10)

Delete these two declarations and their doc comments. **Both are unconditional** —
`engine/src/lib.rs:58-59` carries no `cfg` attribute on either, because both modules' parsers are
platform-independent (review **M3**: the first draft's snippet showed a `#[cfg(windows)]` on `vhl`
that is not there):

```rust
pub mod vhl;
pub mod zybooks;
```

Delete both lines and the doc comments directly above them, and nothing else.

**Compile-blocking together with Task 10's own deletions**, and that is the one non-buildable
intermediate state in this plan: `lib.rs` still declaring a file that no longer exists, or the files
still present with no declaration, are each a hard error, so H1b and Task 10 step 4's `git rm` land
in the same commit. Say so in the commit message.

**Without it:** `cargo build` fails with `file not found for module zybooks`.

### H2 — `app/src/scheduler.rs`, the session directory (applied at Task 8)

In `slot_argv`, the `coursework` step — **the one whose `args[0]` is `"coursework"`**, wherever C3′'s
`sync` step left it in the vector — gains two arguments. Today (after C3′) that line reads:

```rust
    steps.push((exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
```

and becomes:

```rust
    // C5: the per-source cookie jar and captured-session store, DPAPI-sealed under this profile's
    // app data — the same shape `judge` already gets `--log-dir`. WITHOUT the flag the engine keeps
    // its sessions in memory only, so a hand-typed `knowlu-engine coursework` logs in fresh and
    // leaves nothing behind; with it, a portal is logged into once and re-authenticated only when
    // the session expires (cloud design, amendment 2026-09-17, ruling 4). Never in the vault: the
    // vault is plain text and syncs to the account (ruling 2), and a cookie must not.
    steps.push((exe.to_path_buf(), vec![
        "coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
        // `sessions` is the new parameter, already resolved by the caller. The first draft wrote
        // `sessions_dir(&cs_data_dir)` here, and `cs_data_dir` is a binding that exists nowhere in
        // `slot_argv` — which has no `ConsoleState` at all (review I9). `sessions_dir(&cs.data_dir)`
        // belongs at the single call site in `run_slot_inner`, which is where this hand-off puts it.
        "--session-dir".into(), sessions.to_string_lossy().into_owned(),
    ]));
```

**and `slot_argv` gains the data directory it needs to compute that path.** `slot_argv(vault, exe,
judge)` has no `ConsoleState` today; `JudgePlan::Cloud { log_dir }` is how the judge step already
gets one. **Do the same thing, not a new one:** add a fourth parameter
`sessions: &Path` to `slot_argv`, filled at the one call site in `run_slot_inner` from
`sessions_dir(&cs.data_dir)`, and add beside `inference::judgments_dir`:

```rust
/// `<profile app data>\sessions` — per profile, beside `judgments\` and `logs\`, for the same
/// reason: two profiles on one machine never share a portal session.
pub fn sessions_dir(profile_data_dir: &Path) -> PathBuf {
    profile_data_dir.join("sessions")
}
```

Every existing caller of `slot_argv` in `app/tests/scheduler.rs` gains that argument; Task 8 writes
those test edits and commits them with the task (the one place this stream writes a test for a file
it does not own — see *File structure → Modified — the app*).

**Without it:** every slot logs in from scratch, which works and is slower, and VHL's CAS in
particular pays a three-request login twice a day forever. Not a failure; a regression against
ruling 4's "logged into once".

### H3 — `app/src/onboarding.rs`, the wizard's discovery argv (applied at Task 9)

`discovery_argv` learns the three pre-vault flags. The whole function after the change:

```rust
/// `coursework-discover`'s argv, pure and therefore testable on its own (review round 1, m4): which
/// flags appear when, and — the constraint that actually matters — that no argument is ever a
/// secret, only a credential TARGET name (`credentials::target_for`, never a username or password).
///
/// C5: the wizard runs **before the vault exists**, so there is no `config/cloud.yaml` for the
/// engine to resolve a `CloudClient` from — and after C5 discovery is a relayed cloud job. The three
/// extra flags are what stand in for that file: a public project URL, a public anon key, and the
/// NAME of the Credential Manager entry holding the pre-vault session (`account::PENDING_TARGET`).
/// Still no secret on a command line, and `no_argument_is_ever_a_secret` still holds.
pub fn discovery_argv(id: &str, zybooks: bool, vhl: bool) -> Vec<String> {
    let mut args: Vec<String> = vec!["coursework-discover".into()];
    if zybooks {
        args.push("--zybooks-target".into());
        args.push(crate::credentials::target_for(id, "zybooks"));
    }
    if vhl {
        args.push("--vhl-target".into());
        args.push(crate::credentials::target_for(id, "vhl"));
    }
    args.push("--cloud-base".into());
    args.push(crate::account::api_base());
    args.push("--anon-key".into());
    args.push(crate::account::anon_key());
    args.push("--session-target".into());
    args.push(crate::account::PENDING_TARGET.to_string());
    args
}
```

`account::api_base()` and `account::anon_key()` honour `KNOWLU_API_BASE` / `KNOWLU_ANON_KEY`
(`app/src/account.rs:42`, `:46`), which is how a scratch profile is pointed at staging and is
therefore how the exit gate's live run reaches it. `PENDING_TARGET` is `"knowlu/pending/session"`
(`app/src/account.rs:32`).

**Without it:** `coursework-discover` has no way to reach `/relay` before a vault exists, answers
`{"zybooks": [], "vhl": [], "errors": ["discovery needs an account"]}`, and the wizard's mapping
panel is typed by hand — which it already tolerates, so this is a degradation and not a break.

### H4a — `engine/src/main.rs`, `coursework --session-dir` (applied at Task 8)

In `enum Command`, the `Coursework` variant gains one field:

```rust
        /// Where this profile's per-source portal sessions live (the app passes
        /// `%LOCALAPPDATA%\knowlu\profiles\<id>\sessions`). Without it the store is in-memory
        /// only, so a hand-typed run logs in fresh and leaves nothing behind. Never inside the
        /// vault: the vault is plain text and syncs to the account.
        #[arg(long = "session-dir")]
        session_dir: Option<PathBuf>,
```

and the dispatch arm becomes:

```rust
        Command::Coursework { vault, dry_run, via, run_id, session_dir } => {
            match coursework::main(&vault, dry_run, &via, run_id.as_deref(), session_dir.as_deref()) {
                0 => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            }
        }
```

**The entitlement gate C3′ put in front of this arm is not moved and not re-ordered** — read
`main.rs` as C3′ left it and add the field inside whatever shape the gate gave the arm.

**Without it:** Task 8's `coursework::main` signature does not match its caller and the crate does
not build.

### H4b — `engine/src/main.rs`, the three discover flags (applied at Task 9)

In `enum Command`, the `CourseworkDiscover` variant gains three fields:

```rust
        /// The project's function base (`https://<ref>.supabase.co/functions/v1`). Public.
        #[arg(long = "cloud-base")]
        cloud_base: Option<String>,
        /// The project's anon key. Public — it is compiled into the app and shipped.
        #[arg(long = "anon-key")]
        anon_key: Option<String>,
        /// The NAME of the Credential Manager entry holding the session, for a wizard whose vault
        /// does not exist yet. A target name, never a token.
        #[arg(long = "session-target")]
        session_target: Option<String>,
```

and the dispatch arm becomes:

```rust
        Command::CourseworkDiscover { vault, zybooks_target, vhl_target, cloud_base, anon_key, session_target } => {
            println!(
                "{}",
                coursework::discover_json(
                    vault.as_deref(),
                    zybooks_target.as_deref(),
                    vhl_target.as_deref(),
                    coursework::PreVaultCloud {
                        api_base: cloud_base.as_deref(),
                        anon_key: anon_key.as_deref(),
                        session_target: session_target.as_deref(),
                    },
                )
            );
            ExitCode::SUCCESS
        }
```

**Without it:** Task 9's `discover_json` signature does not match its caller and the crate does not
build.

### H5 — `engine/src/cli.rs` (verified at Task 12; no edit)

`cli.rs` is the rank pipeline and the `Step` record shape. C5 changes neither: the `coursework` run
record still carries three steps (`zybooks`, `vhl`, `sync`) with the same counts, because
`main_with_fetchers`'s step-building block is untouched by every task here. Task 12 records that it
was read and not edited, and re-runs `rank_cannot_reach_a_judgment_endpoint`.

### H6 — `app/static/index.html`, the coursework panel's disclosure (applied at Task 9)

**The wording below is this plan's draft, not Quinn's and not the lawyer's** — it is Q6's to
  approve or replace whole, and H6 lands only after they have.

**Corrected by review I10, and this correction changes who writes it.** The spec says the wizard's
one-sentence disclosure of the ToS tension "stays", gaining a clause. **There is no such sentence in
the checkout.** The coursework panel (`app/static/index.html:131-141`) carries one lede — *"Optional.
Stored in Windows Credential Manager on this machine — never in the vault, never in a backup, and
never sent to us."* — which is about where a credential is kept, not about using a portal login this
way sitting against the vendor's terms; and no test pins a ToS literal
(`the_logins_panel_maps_what_it_finds_to_a_course`, `app/tests/static_assets.rs:508-527`, pins the
mapping block). Nothing in `app/static/` or `site/` contains "terms of service" or "on your behalf"
for the portals.

So C5 is **writing** this disclosure, not extending one — and a sentence about a vendor's terms is
legal copy. **It therefore goes to Quinn and the lawyer with Q6 (Task 11 step 1), not appended by a
controller**, and H6 lands only once Quinn has the words. The draft that goes with Q6, as a **new
sentence** directly after the existing lede, inside the same element:

```html
<p class="note">Knowlu signs in to these sites the way you would: the request is made from your own PC, on your instruction, with the login you saved here, and the site sees your computer and your session rather than ours. Some schools' sites ask you not to use an automated tool on them; if yours does, that is between you and them, and you can leave a login blank and add the work by hand.</p>
```

Task 9 pins **both halves** — the existing lede and the new sentence — in
`app/tests/static_assets.rs`, so neither can drift without the other. `no_network_reference_in_the_shipped_page`
still holds: the sentence carries no URL.

**Without it:** the panel says where the password is stored and nothing about what Knowlu does with
it, and spec §8's claim that a disclosure "stays" stays false. **If Quinn rules that the existing
lede is the disclosure**, H6 is withdrawn, Task 9 pins the lede alone, and the spec's §8 says so —
which is a legitimate outcome and not a gap, as long as the document stops describing a sentence the
repository does not have.

### H7 — `cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts`, the BOM comment (applied at Task 6)

Lines 22-23 today say:

```ts
  // The capture is a real one and carries a UTF-8 BOM; the device's `decode_json` strips it there,
  // so no BOM ever travels on the wire. Here the file is read directly, so strip it here.
```

After C5 that is false: `decode_json` is deleted with `zybooks.rs`, and the relay returns the body
byte for byte **including a leading BOM** (spec §2.4). Replace with:

```ts
  // The capture is a real one and carries a UTF-8 BOM. Since C5 the device returns a body byte for
  // byte and a BOM DOES travel on the wire — `plans/zybooks.ts`'s own `decodeJson` strips it, and
  // `a_bom_prefixed_signin_body_is_decoded` pins that. Here the file is read directly, so strip it
  // here too, for the same reason and not a different one.
```

**One comment, no code.** The test's behaviour is unchanged and the frozen reference is untouched.

**Without it:** a true test with a false comment, which is the kind of thing that survives three
years and then misleads somebody at 2 a.m.

### H8 — `cloud/supabase/config.toml`, the function entry (applied at Task 5)

After `[functions.judge-rules]`, in the deploy-order block the file already keeps:

```toml
[functions.relay]
verify_jwt = false
```

The reason is the one the file already gives at the top of that block: each handler verifies the
bearer itself and answers 401 or 402 in **our** shape, which is the shape
`_shared/entitlement.ts` promises and the app's `account.rs` parses.

**Without it:** `supabase functions deploy relay --use-api` succeeds and the gateway then rejects
every call with its own 401 body before `relayHandler` runs, so every device answer is
`an unreadable reply` and no test on staging means anything.

### H9 — `CLAUDE.md` and `HANDOFF.md` (applied at Task 12)

**`CLAUDE.md`**, four edits:

1. In the engine-command list, the `coursework` bullet gains the session flag and the relay sentence:

```
- `coursework --vault <v> [--dry-run] [--via <via>] [--session-dir <dir>] [--run-id <id>]` — zyBooks
  + VHL into `tasks/`. Always exits 0. An empty parse is a failure, never an empty semester.
  **Since C5 the device composes nothing**: the cloud drives the fetch as a step protocol over
  `POST /relay` and the device substitutes the password from Windows Credential Manager into the
  step it was handed, sends it with that source's cookie jar, and returns the raw response
  (`engine/src/relay.rs`). The one portal-specific thing the client keeps is
  `relay::PORTAL_SOURCES` — the compiled-in host allow-list, checked before every request and every
  redirect hop, so a credential only ever goes to the site it was given for. `--session-dir` is
  where the DPAPI-sealed per-source cookie jar lives; without it the store is in-memory only.
```

2. The `coursework-discover` bullet gains the three pre-vault flags and the relay sentence.
3. The **engine invariants** block gains one line after the `judge` sentence: *"**The device knows no
   login flow, no URL and no parser** (cloud design, amendment 2026-09-17, ruling 4). Adding or
   repairing a portal is a deploy; a new **host** or a new login panel is a release, and
   `relay::PORTAL_SOURCES` is why."*
4. The toolchain bullet naming `ureq`'s `cookies` feature is rewritten to name `relay.rs` instead of
   `vhl::default_opener`: *"`ureq` is built with its non-default **`cookies`** feature and VHL does
   not work without it: CAS login on `www.vhlcentral.com`, the dashboard on `m3a.vhlcentral.com`, one
   jar scoped to `.vhlcentral.com`. Since C5 the jar belongs to `relay.rs` — one agent per source per
   run, `max_redirects(0)` because the device follows the chain itself and checks every hop against
   the allow-list — and the `json` feature is deliberately **not** enabled: the store replays the
   vendor's own `Set-Cookie` lines, because `CookieJar::save_json` drops a session cookie that has no
   `Expires`."*

**`HANDOFF.md`**, §3's sequence line: C5 marked done with its branch, its PR and its CI run, and §4
gains the production row of Task 12 step 7. No other section.

### H11 — `app/src/main.rs`, the console window's `generate_handler!` list (applied at Task 8; **compile-blocking for the settings row**)

Ruling **R-C5-plan-4**. The **console** window's list (`app/src/main.rs:186`) gains one entry,
beside the other `commands::` names, in the order the list already uses:

```rust
            commands::save_portal_login,
```

**The picker/wizard list is not touched**: `onboarding::store_credentials` stays exactly where it
is, because the wizard still saves a login at onboarding and R-C5-plan-1 lives in both places.

**And the first step of this hand-off is to COUNT.** `CLAUDE.md`'s rule is *recount before quoting a
number*, and two streams merge ahead of C5, so the controller counts both lists on the branch as it
stands — `rg -c "^\s+\w+::" ` over each `generate_handler!` block, or by eye — records the pair in
the task report, adds one to the console's, and hands **those** numbers to H9's `CLAUDE.md` edit.
For orientation and not to be copied: `main` is 30 and 43 today; C1b takes it to 29 and 42; after
C1b, C5 makes it **29 and 43**. If the observed pair is neither, that is a finding about the branch
point and it is reported, not reconciled.

**Without it:** the settings row's Save button invokes a command Tauri rejects before its body runs,
and the student sees a generic failure on the one screen C5 added to stop them being stuck.

### H10 — `scripts/wizard-check.py` (verified at Task 12; no edit)

The headless wizard walk exercises the nine panels against `app/static/`. C5 changes one sentence on
the coursework panel (H6) and nothing structural, so the script's own checks still pass unedited.
Task 12 runs it and records `ok`; if it fails, that is a finding about H6 and not a licence to edit
the script.

---

## The tasks

Twelve. Tasks 1-4 are the device's guarantees, offline and unit-testable. Task 5 is the protocol
server-side. Tasks 6 and 7 are the two portals' plans, and they come **before** Task 10 because they
are where the network tests in `zybooks.rs` and `vhl.rs` are ported to — deleting those files first
would throw away the only executable record of how the two portals behave. Tasks 8 and 9 join the
two halves. Task 10 removes the on-device fetchers. Task 11 is the published promise. Task 12 closes.

Each task ends with `cargo test --workspace` at 0 warnings (or the Deno suite, for a cloud-only
task), a specific `git add`, and a message through `-F`.

**One thing to hold in mind while reading every code block below.** Nothing in this plan has been
compiled, borrow-checked, type-checked or run — not by its author and not by either review. Every
signature, every API call and every count here is **read from the sources, not observed**, and the
first task to execute is the first thing that will find out. Two rounds of review have already
turned up four defects of exactly that kind (a test that counted its own source, an arithmetic pin
that contradicted its constant, a `Cookie` API that does not expose what was asked of it, and a
hand-off snippet naming a binding that does not exist), so treat a step that will not compile as
expected traffic rather than as a surprise: fix it, and say so in the task report.

---

### Task 1: The host allow-list, and the three encoders that move with it

**Read first:** spec §3 in full, and `app/src/inference.rs:192-233` — `SupportedRuntime`,
`SUPPORTED_RUNTIMES` and `runtime_release_for` — which is the pattern this table copies, down to the
doc comment's argument for why a compiled-in table is a different kind of thing from a file.

**Why this is first.** Everything else in the module is a convenience; this is the guarantee. Writing
it first means every later piece is built on top of a check that already exists and already refuses,
rather than having the check retro-fitted around a working fetch.

**Ownership note.** This task changes one line in `engine/src/cloudmodel.rs`
(`crate::zybooks::scrub` → `crate::relay::scrub`, at `:252`, plus its module-doc mention at `:17`).
C2 is merged, so its exclusive-ownership row retired with it, and `cloudmodel.rs` is not in
`HANDOFF.md` §2's shared-single-owner list — so it is C5's, and it is named in *Global Constraints*'
ownership list for that one line and nothing more.

- [ ] **Step 1: Ask Quinn (Q3)**, in one message, when this task is reached: *VHL's row in the host
  allow-list: the two exact hosts ruling 4 names — `www.vhlcentral.com` and `m3a.vhlcentral.com` — or
  `*.vhlcentral.com`? The exact pair is what you ruled and is what I am building. The cost, plainly:
  if VHL ever renumbers `m3a` to something else, every student's VHL sync refuses on the dashboard
  hop until an app release ships, because the dashboard URL comes out of the vendor's own payload and
  is checked like any other URL. Widening to `*.vhlcentral.com` buys that case back and gives up the
  narrowest possible claim in the privacy policy. Recommendation: ship the exact pair as ruled, and
  widen the day it costs something.* Record the answer in the task report; the plan is built to the
  exact pair.

- [ ] **Step 2: Write the failing test** — `engine/tests/relay_allowlist.rs`, new. It is an
  **integration** test on purpose: it links `knowlu_engine` as a dependency, so the crate is compiled
  **without `cfg(test)`** and the test-only `loopback` row of Task 4 is invisible to it. That is the
  whole reason this file is not a `mod tests` inside `relay.rs`.

```rust
//! The compiled-in host allow-list, pinned the way `SUPPORTED_RUNTIMES` is pinned.
//!
//! **An integration test, deliberately.** This links the library the way the shipped binary links
//! it — without `cfg(test)` — so the `loopback` row `relay.rs`'s own unit tests use cannot be what
//! this file sees. A row that exists only under `cfg(test)` and a table this test says has two rows
//! are the two halves of one guarantee: there is no way to reach a host off this list from a real
//! build, and no test-only door that a refactor could leave propped open.
//!
//! Cloud design, amendment 2026-09-17, ruling 4: "a credential saved for a source is substituted
//! only into an HTTPS request to that source's registered hosts". Spec §3 widens that to *every*
//! relayed request, placeholder or not, because the cookie jar rides on all of them.

use knowlu_engine::relay::{check_host, HostRefusal, PORTAL_SOURCES};

#[test]
fn the_table_is_exactly_the_two_rows_the_ruling_names() {
    let rows: Vec<(&str, &[&str])> = PORTAL_SOURCES.iter().map(|s| (s.name, s.hosts)).collect();
    assert_eq!(
        rows,
        vec![
            ("zybooks", &["*.zybooks.com"][..]),
            ("vhl", &["www.vhlcentral.com", "m3a.vhlcentral.com"][..]),
        ],
        "the allow-list is ruling 4's, verbatim. Adding a row is a deliberate diff in a release \
         (spec §3) — if this failed because a portal was added, the release notes and the privacy \
         page have to say so too."
    );
}

#[test]
fn the_loopback_row_does_not_exist_in_a_real_build() {
    assert!(
        PORTAL_SOURCES.iter().all(|s| s.name != "loopback"),
        "the test-only row escaped into the shipped table"
    );
    assert!(matches!(check_host("loopback", "http://127.0.0.1:1/x"), Err(HostRefusal::UnknownSource)));
}

#[test]
fn every_pattern_is_well_formed() {
    for source in PORTAL_SOURCES {
        assert!(!source.name.is_empty() && source.name.chars().all(|c| c.is_ascii_lowercase()),
            "a source name is a credential-target suffix (`knowlu/<profile>/<name>`): {}", source.name);
        assert!(!source.hosts.is_empty(), "{}: a source with no hosts can never fetch", source.name);
        for pattern in source.hosts {
            assert!(pattern.is_ascii() && *pattern == pattern.to_ascii_lowercase(),
                "{pattern}: patterns are compared against an ASCII-lowercased host");
            for forbidden in ["://", "/", ":", "?", "#", " ", "@"] {
                assert!(!pattern.contains(forbidden),
                    "{pattern}: a pattern is a host, not a URL — it carries no {forbidden}");
            }
            let bare = pattern.strip_prefix("*.").unwrap_or(pattern);
            assert!(!bare.contains('*'), "{pattern}: the ONLY wildcard form is a leading `*.`");
            assert!(bare.contains('.') && !bare.starts_with('.') && !bare.ends_with('.'), "{pattern}");
        }
    }
}
```

- [ ] **Step 3: Write the matcher's own cases** — appended to the same file. Every rejection the
  spec's §3 numbers 1-4 names, and the three accept cases the two portals actually need.

```rust
#[test]
fn a_wildcard_matches_the_domain_and_its_subdomains_and_nothing_that_merely_looks_like_it() {
    for good in [
        "https://zyserver.zybooks.com/v1/signin",
        "https://learn.zybooks.com/",
        "https://zybooks.com/",                       // `*.d` matches the bare domain too
        "https://ZYSERVER.ZyBooks.COM/v1/signin",     // lowercased before matching
        "https://zyserver.zybooks.com./v1/signin",    // one trailing dot stripped
        "https://zyserver.zybooks.com:443/v1/signin", // the default port, written out
    ] {
        assert_eq!(check_host("zybooks", good).unwrap_or_else(|e| panic!("{good}: {e:?}")).contains("zybooks.com"), true, "{good}");
    }
    for bad in [
        "https://zybooks.com.evil.example/v1/signin",
        "https://evilzybooks.com/v1/signin",
        "https://notzybooks.com/v1/signin",
        "https://www.vhlcentral.com/",                // the OTHER source's host, on this credential
    ] {
        assert!(matches!(check_host("zybooks", bad), Err(HostRefusal::NotAllowed(_))), "{bad} was allowed");
    }
}

#[test]
fn vhl_gets_its_two_exact_hosts_and_no_third() {
    assert!(check_host("vhl", "https://www.vhlcentral.com/user_session").is_ok());
    assert!(check_host("vhl", "https://m3a.vhlcentral.com/courses/1/sections/2/?guids=true").is_ok());
    // The one the ruling deliberately does not carry (Q3). If Quinn widens the row, this case is
    // the one that changes, and the change is visible in the diff — which is the point.
    assert!(matches!(check_host("vhl", "https://m4a.vhlcentral.com/"), Err(HostRefusal::NotAllowed(_))));
}

#[test]
fn the_four_structural_refusals_each_name_the_host_and_never_the_url() {
    // 1. not https
    assert!(matches!(check_host("zybooks", "http://zyserver.zybooks.com/v1/signin?auth_token=t"), Err(HostRefusal::NotAllowed(_))));
    // 1. a port that is not 443
    assert!(matches!(check_host("zybooks", "https://zyserver.zybooks.com:8443/v1"), Err(HostRefusal::NotAllowed(_))));
    // 2. userinfo — `https://x@evil.example@zybooks.com/` and its simpler cousin
    assert!(matches!(check_host("zybooks", "https://evil.example@zybooks.com/"), Err(HostRefusal::NotAllowed(_))));
    // 2. an IP literal, v4 and v6
    assert!(matches!(check_host("zybooks", "https://93.184.216.34/v1"), Err(HostRefusal::NotAllowed(_))));
    assert!(matches!(check_host("zybooks", "https://[2606:2800:220:1:248:1893:25c8:1946]/v1"), Err(HostRefusal::NotAllowed(_))));
    // 2. non-ASCII, and its punycode twin, which is a DIFFERENT host and is refused on its own merits
    assert!(matches!(check_host("zybooks", "https://zybooks.cоm/"), Err(HostRefusal::NotAllowed(_))));
    assert!(matches!(check_host("zybooks", "https://xn--zybooks-8fg.com/"), Err(HostRefusal::NotAllowed(_))));
    // unparseable
    assert!(matches!(check_host("zybooks", "not a url"), Err(HostRefusal::NotAllowed(_))));
    // an unknown source is its OWN code, because "we do not know this portal" and "that host is not
    // yours" are different problems with different fixes.
    assert!(matches!(check_host("blackboard", "https://blackboard.example.edu/"), Err(HostRefusal::UnknownSource)));

    // And the property the whole module exists for: a refusal names the HOST and nothing else. The
    // URL may carry a token in its query (`get_json` puts one there today), so it is never quoted.
    let token_bearing = "https://evil.example/v1/items?auth_token=SUPERSECRETVALUE";
    let Err(HostRefusal::NotAllowed(named)) = check_host("zybooks", token_bearing) else { panic!() };
    assert_eq!(named, "evil.example");
    assert!(!named.contains("SUPERSECRETVALUE"));
}
```

- [ ] **Step 4: Run it and watch it fail.**

Run: `cargo test -p knowlu-engine --test relay_allowlist`
Expected: FAIL to compile — ``unresolved import `knowlu_engine::relay` ``. That is the right
failure: the module does not exist yet and neither does hand-off H1a.

- [ ] **Step 5: Write `engine/src/relay.rs`'s first section.** The module doc is load-bearing —
  it is where the next person reads what this file is for before they add a portal to it.

```rust
//! The credential-substituting HTTPS relay (cloud design, amendment 2026-09-17, ruling 4; C5 spec).
//!
//! **The device knows no login flow, no URL and no parser.** The cloud composes each request —
//! method, URL, headers, body — with `{{credential:<source>:username|password}}` and
//! `{{capture:<name>}}` placeholders in it. This module fills them from Windows Credential Manager,
//! sends the request from the student's own machine with that source's cookie jar, strips every
//! `Set-Cookie` and every captured value out of the answer, and hands the rest back. The password
//! never leaves the machine; the vendor sees the student's own IP and session; and repairing a
//! portal after a vendor changes its markup is a deploy with no release and no user action.
//!
//! # The one portal-specific thing the client keeps
//!
//! [`PORTAL_SOURCES`] — which hosts a given credential may ever be sent to. Compiled in, the way
//! `SUPPORTED_RUNTIMES` pins runtimes, so it cannot be edited by whatever also edited the thing
//! being checked. It is the only claim in the privacy policy that does not rest on trusting our own
//! servers: **a password goes to the site it was given for or nowhere**, and that holds if our
//! project is compromised, if a plan is wrong, and if someone replaces a plan.
//!
//! **The price, plainly, and read this before adding a portal.** Everything about a portal is a
//! cloud change — the flow, the URLs, the order, the parser, the oracle, the reconcile — *except*
//! two things: a new **host** (this table) and the wizard panel that captures that source's login.
//! Those cost an app release. A vendor that changes its markup costs nothing; a vendor that moves to
//! a new host costs a release.
//!
//! # Four properties this module exists to hold
//!
//! - *Every relayed request is checked, placeholder or not, and on every redirect hop.* A request
//!   with no placeholder still travels with the source's cookie jar, and a session cookie sent to a
//!   host of the server's choosing is the same leak by a slower route. The device follows redirects
//!   itself — `max_redirects(0)` on the agent — precisely so each hop passes through [`check_host`].
//! - *The check runs before the credential is read.* A refused host never touches a password, not
//!   even in memory.
//! - *No secret ever reaches a warning line.* [`scrub`] runs over every borrowed string, in all
//!   three forms a secret can take (raw, percent-encoded, JSON-escaped), and no error message this
//!   module produces ever quotes a URL — a URL carries a token in its query.
//! - *A value the plan marks `capture` stays here.* zyBooks' `session.auth_token` is
//!   password-equivalent for its lifetime, so it is redacted out of the body returned to the cloud.

use std::time::Duration;

use ureq::http::Uri;

/// One portal this build is prepared to send a saved credential to.
///
/// `name` is both the config key under `coursework:` and the last segment of the Credential Manager
/// target the app writes (`knowlu/<profile_id>/<name>`, `app/src/credentials.rs:17`), so it is
/// lowercase ASCII and `relay_allowlist.rs` pins that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalSource {
    pub name: &'static str,
    /// Exact hosts, or `*.<domain>` — the ONLY wildcard form there is. A pattern that is neither is
    /// a bug `relay_allowlist.rs` catches.
    pub hosts: &'static [&'static str],
}

/// **The root of trust for sending a credential** (C5-D2). Exactly the two rows ruling 4 names.
///
/// Adding a row is a code change and a release of this app, which is the point: it is a deliberate
/// act with a diff, not a plan someone deployed.
pub const PORTAL_SOURCES: &[PortalSource] = &[
    PortalSource { name: "zybooks", hosts: &["*.zybooks.com"] },
    PortalSource { name: "vhl", hosts: &["www.vhlcentral.com", "m3a.vhlcentral.com"] },
];

/// Why a URL was refused. Two variants, not one: "we do not know this portal" and "that host is not
/// this portal's" are different problems with different fixes, and the run log says which.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRefusal {
    UnknownSource,
    /// The offending host, ASCII-lowercased, or an empty string when the URL had none to name.
    /// **Never the URL** — a relayed URL carries a token in its query.
    NotAllowed(String),
}

pub fn source_named(name: &str) -> Option<&'static PortalSource> {
    PORTAL_SOURCES.iter().find(|s| s.name == name).or_else(|| extra_source(name))
}

/// The one test-only door, and it is a door in the **test build only**.
///
/// `engine/tests/relay_allowlist.rs` is an integration test: it links this library without
/// `cfg(test)`, asserts the table is two rows and asserts `check_host("loopback", …)` is
/// `UnknownSource`. Those two halves are what stop this from being a way in.
#[cfg(test)]
fn extra_source(name: &str) -> Option<&'static PortalSource> {
    const LOOPBACK: PortalSource = PortalSource { name: "loopback", hosts: &["127.0.0.1"] };
    (name == "loopback").then_some(&LOOPBACK)
}

#[cfg(not(test))]
fn extra_source(_name: &str) -> Option<&'static PortalSource> {
    None
}

/// The head of a source file, up to its `#[cfg(test)] mod tests` and no earlier (review **R5**).
///
/// Splitting on `#[cfg(test)]` alone truncates this very file at [`extra_source`], which sits
/// mid-module, and would silently hand a scanner the top third of `relay.rs`. Two tests depend on
/// reading real non-test code — the `.expose()` count and the portal-URL scan — and a scan that
/// reads the wrong half does not fail, it just stops meaning anything.
#[cfg(test)]
pub(crate) fn split_before_test_module(src: &str) -> &str {
    // Tolerant of the whitespace rustfmt actually emits between the attribute and the item.
    for marker in ["#[cfg(test)]\nmod tests", "#[cfg(test)]\r\nmod tests", "#[cfg(test)] mod tests"] {
        if let Some(at) = src.find(marker) {
            return &src[..at];
        }
    }
    src
}

/// `host == pattern`, or — for `*.d` — `host == d || host.ends_with(".d")`.
///
/// The `.` in `ends_with(".d")` is the whole test: without it `evilzybooks.com` matches
/// `*.zybooks.com`, and that is the bug this function exists to not have.
fn pattern_matches(pattern: &str, host: &str) -> bool {
    match pattern.strip_prefix("*.") {
        Some(domain) => host == domain || host.ends_with(&format!(".{domain}")),
        None => host == pattern,
    }
}

/// A host that is an address rather than a name. Refused: a name is what a certificate and an
/// allow-list are both about, and an IP literal is how a plan would reach a host off the list
/// without ever writing its name down.
fn is_address_literal(host: &str) -> bool {
    host.starts_with('[')
        || host.contains(':')
        || (!host.is_empty() && host.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
}

/// **Before a socket is opened, for every request and every redirect hop.** Returns the normalised
/// host on success, so the caller can log which host it talked to without re-parsing.
///
/// The five rules, in order, and each of them is one line of spec §3:
/// 1. the URL parses, the scheme is `https`, and the port is absent or 443;
/// 2. there is no userinfo, the host is not an address literal, and the host is pure ASCII;
/// 3. the host is ASCII-lowercased and one trailing `.` is stripped;
/// 4. it matches a pattern of **this step's own source** — so a value or a jar belonging to one
///    portal can never be aimed at another's host;
/// 5. anything else is `NotAllowed`, naming the host and nothing else.
pub fn check_host(source: &str, url: &str) -> Result<String, HostRefusal> {
    let Some(entry) = source_named(source) else { return Err(HostRefusal::UnknownSource) };
    // `cfg!(test)` and not a field on the row: the relaxation is a property of THIS BUILD, and a
    // field would be a thing a future row could set. A loopback stand-in is plain http on a high
    // port, which rules 1 and 2 would otherwise refuse.
    let loopback = cfg!(test) && entry.name == "loopback";

    let Ok(uri) = url.parse::<Uri>() else { return Err(HostRefusal::NotAllowed(String::new())) };
    let refuse = |host: &str| Err(HostRefusal::NotAllowed(host.to_ascii_lowercase()));

    let Some(authority) = uri.authority() else { return Err(HostRefusal::NotAllowed(String::new())) };
    let host = authority.host();
    if !loopback && uri.scheme_str() != Some("https") {
        return refuse(host);
    }
    // `Authority::as_str()` keeps userinfo; `host()` drops it. `https://x@evil.example@zybooks.com/`
    // is the attack: a parser that read the LAST `@` and one that read the first disagree about the
    // host, so neither is trusted and the URL is simply refused.
    if authority.as_str().contains('@') {
        return refuse(host);
    }
    match authority.port_u16() {
        None => {}
        Some(443) => {}
        Some(_) if loopback => {}
        Some(_) => return refuse(host),
    }
    if !host.is_ascii() || host.is_empty() {
        return refuse(host);
    }
    if !loopback && is_address_literal(host) {
        return refuse(host);
    }
    let host = host.to_ascii_lowercase();
    let host = host.strip_suffix('.').unwrap_or(&host).to_string();
    if entry.hosts.iter().any(|pattern| pattern_matches(pattern, &host)) {
        Ok(host)
    } else {
        Err(HostRefusal::NotAllowed(host))
    }
}
```

- [ ] **Step 6: Move `scrub`, `quote`, `quote_plus` and `json_escape_ascii` — move, not rewrite.**
  Cut them from `engine/src/zybooks.rs:434`, `:454`, `:471` and `engine/src/vhl.rs:518` into
  `relay.rs`, **byte for byte including their doc comments**, make all four `pub(crate)` except
  `scrub` which is `pub` (`cloudmodel.rs` calls it), and cut their four tests into `relay.rs`'s own
  `#[cfg(test)] mod tests` under the names they already have:
  `scrub_replaces_the_raw_percent_encoded_and_json_escaped_forms` (`zybooks.rs:1075`, **widened
  below**),
  `quote_matches_python_urllib_quote_with_no_safe_characters` (`:1093`),
  `json_escape_matches_pythons_ensure_ascii_dumps` (`:1102`), and
  `urlencode_uses_quote_plus_exactly_as_urllib_does` (`vhl.rs:1247`, which becomes
  `quote_plus_matches_python_urllib_quote_plus` — `urlencode` itself goes to the plans, and this is
  the one test name that changes because the function it named no longer exists here).

  **Why they move rather than being rewritten:** these four are Python-compatibility functions whose
  exact behaviour is pinned against Python's `urllib.parse.quote`, `quote_plus` and
  `json.dumps(ensure_ascii=True)`. Re-deriving them would be re-deriving a decision, and the tests
  that pin them are the reason nobody has to.

  **The one change to `scrub`, and it is a security fix** (review **I2**). `zybooks.rs:434-451`
  replaces each secret in **three** forms — raw, `quote` (percent-encoded), `json_escape_ascii` — and
  the relay adds a **fourth**: `quote_plus`, which renders a space as `+` rather than `%20`. A portal
  password containing a space, sent in the VHL CAS form POST, reaches a `ureq` transport error as
  `hun+ter` and survives a three-form scrub untouched — and that string goes into `warnings`, into
  the run record, into `state/runner-log.md` and into the Runs view. This is a pre-existing gap in
  `vhl.rs`; C5 is where it stops being incidental and becomes a stated guarantee, because the module
  doc this task writes claims "all three forms a secret can take". So: add `quote_plus(secret)` to
  `scrub`'s form list as it moves, rename the doc's "three" to "four", rename the test to
  `scrub_replaces_the_raw_percent_encoded_plus_encoded_and_json_escaped_forms`, and give it a fourth
  case with a space-bearing secret. Task 4 adds the other half: a form-body transport failure whose
  error text carries `hun+ter` and comes back `<redacted>`.

  Then, in `engine/src/zybooks.rs` and `engine/src/vhl.rs`, replace the removed definitions with
  `use crate::relay::{quote, scrub};` and `use crate::relay::{quote_plus, scrub};` respectively —
  **`quote_plus` too** (review **M4**): `vhl::urlencode` (`engine/src/vhl.rs:534-540`) calls it, and
  an import of `scrub` alone leaves that file unable to compile. Both still compile because they
  are deleted in Task 10, and a branch that does not build between Task 1 and Task 10 would make
  every task in between untestable.

  And in `engine/src/cloudmodel.rs`, two edits: `:252`'s `crate::zybooks::scrub(text, &[&self.token])`
  becomes `crate::relay::scrub(text, &[&self.token])`, and `:17`'s module-doc mention of
  `zybooks::scrub` becomes `relay::scrub`.

- [ ] **Step 7: Apply hand-off H1a** (`engine/src/lib.rs`'s `pub mod relay;`) — **compile-blocking**,
  so the controller applies it before the next run.

- [ ] **Step 8: Add the dependency case** — in `engine/tests/dependency_boundary.rs`, at the end:

```rust
/// The relay adds no crate, and there is still exactly one HTTP client in this engine.
///
/// C5 moves the fetch **sequence** to the cloud and leaves the device sending requests it was
/// handed. The temptation at that moment is a "better" HTTP client — one with a redirect policy, a
/// cookie store with a serialiser, a URL type. Every one of those would be a second place a request
/// is assembled, out of reach of the one test that proves what a request may carry, and a second
/// TLS stack to keep off OpenSSL. `ureq` with `cookies` is what we have and what we keep.
///
/// `base64` in particular: `relay.rs` encodes a non-UTF-8 response body itself, in twenty lines with
/// RFC 4648's own vectors as the test, because C3′ removed the crate with the envelope and a crypto
/// crate that comes back for a data encoding is a dependency nobody re-audits.
#[test]
fn the_relay_adds_no_crate_and_no_second_http_client() {
    assert!(MANIFEST.contains("ureq = { version = \"3.4.0\", features = [\"cookies\"] }"),
        "the relay's one jar per source is ureq's `cookies` feature; the `json` feature is \
         deliberately NOT enabled (CookieJar::save_json drops a session cookie with no Expires)");
    for (name, manifest) in MANIFESTS {
        for forbidden in ["url = ", "cookie_store = ", "reqwest", "hyper", "attohttpc", "minreq"] {
            assert!(!manifest.contains(forbidden),
                "`{forbidden}` must not be a dependency of the engine crate ({name}). The relay \
                 parses a URL with `ureq::http::Uri` and keeps its jar with ureq's own, so every \
                 request is assembled in `relay.rs` where one test can see it.");
        }
    }
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("relay.rs"),
    ).expect("engine/src/relay.rs");
    assert!(!src.contains("base64::") && !src.contains("use base64"),
        "C3′ removed `base64` with the envelope; the relay encodes its own (spec §2.4)");
}
```

- [ ] **Step 9: Run, then commit.**

Run: `cargo test -p knowlu-engine --test relay_allowlist --test dependency_boundary`, then
`cargo test --workspace`.
Expected: green, 0 warnings, and the pass count up by **eight** (three table tests, three matcher
tests, one dependency test, and the four moved tests minus the four that left `zybooks`/`vhl` — net
+7 from this file plus the one new dependency case; record the real number, not this one).

```bash
git add engine/src/relay.rs engine/src/zybooks.rs engine/src/vhl.rs engine/src/cloudmodel.rs \
        engine/tests/relay_allowlist.rs engine/tests/dependency_boundary.rs
git commit -F .git-commit-msg.txt   # "feat(engine): the relay's host allow-list, compiled in and pinned (C5 Task 1)"
```

---

### Task 2: A step, its substitution, and the four things substitution may not do

**Read first:** spec §2.2 and §2.3, including the encoding table and its "Today's code it
reproduces" column, and `engine/src/zybooks.rs:556` (`get_json`, which is what puts a token in a
query today) and `engine/src/vhl.rs:534` (`urlencode`, which is what builds the CAS POST body).

- [ ] **Step 1: Write the failing tests** — appended to `relay.rs`'s `#[cfg(test)] mod tests`.

```rust
    fn secrets() -> Secrets {
        // The Credential Manager seam. Every test in this module supplies its own, so nothing here
        // reads the real store and `CREDMAN_LOCK` is not needed (CLAUDE.md).
        Secrets::fixed(&[("zybooks", "student@example.edu", "p@ss w/rd"), ("vhl", "vhluser", "hunter2")])
    }

    fn step(id: &str, source: &str, method: &str, url: &str) -> Step {
        Step { id: id.into(), source: source.into(), method: method.into(), url: url.into(),
               headers: vec![], body: None, capture: vec![], follow_redirects: true, max_bytes: MAX_BYTES }
    }

    #[test]
    fn a_credential_reaches_a_json_body_escaped_by_the_one_serialiser() {
        let mut s = step("zybooks.signin", "zybooks", "POST", "https://zyserver.zybooks.com/v1/signin");
        s.body = Some(Body::Json(serde_json::json!({
            "email": "{{credential:zybooks:username}}", "password": "{{credential:zybooks:password}}"
        })));
        let filled = fill(&s, &secrets(), &Captures::default()).expect("substituted");
        // `dumps_value`, so the bytes are the crate's one JSON writer's — sorted keys, Python
        // separators — and the password's `/` and space are the serialiser's problem, not ours.
        assert_eq!(
            String::from_utf8(filled.body_bytes.clone().unwrap()).unwrap(),
            "{\"email\": \"student@example.edu\", \"password\": \"p@ss w/rd\"}"
        );
    }

    #[test]
    fn a_capture_reaches_a_query_percent_encoded_and_a_header_verbatim() {
        let mut captures = Captures::default();
        captures.put("zybooks", "zybooks_token", "tok en/+=");
        let mut s = step("zybooks.items", "zybooks", "GET",
            "https://zyserver.zybooks.com/v1/user/7/items?items=%5B%22zybooks%22%5D&auth_token={{capture:zybooks_token}}");
        s.headers = vec![("Authorization".into(), "Bearer {{capture:zybooks_token}}".into())];
        let filled = fill(&s, &secrets(), &captures).expect("substituted");
        // `quote(value, safe="")` — exactly what `zybooks::get_json` does today, and for the same
        // reason: `http.client` rejects a URL containing a raw space by raising with the whole URL
        // in the message.
        assert!(filled.url.ends_with("&auth_token=tok%20en%2F%2B%3D"), "{}", filled.url);
        // A header value is verbatim: a bearer token is not percent-encoded by anybody.
        assert_eq!(filled.headers[0].1, "Bearer tok en/+=");
    }

    #[test]
    fn a_form_value_is_quote_plus_exactly_as_the_cas_login_needs() {
        let mut s = step("vhl.login", "vhl", "POST", "https://www.vhlcentral.com/user_session");
        s.body = Some(Body::Form(vec![
            ("authenticity_token".into(), "AbC+/=".into()),
            ("lt".into(), "LT-1787747389rB844A8D064F2F719D0".into()),
            ("user_session[username]".into(), "{{credential:vhl:username}}".into()),
            ("user_session[password]".into(), "{{credential:vhl:password}}".into()),
        ]));
        let filled = fill(&s, &secrets(), &Captures::default()).expect("substituted");
        assert_eq!(
            String::from_utf8(filled.body_bytes.clone().unwrap()).unwrap(),
            "authenticity_token=AbC%2B%2F%3D&lt=LT-1787747389rB844A8D064F2F719D0\
             &user_session%5Busername%5D=vhluser&user_session%5Bpassword%5D=hunter2"
        );
        assert_eq!(filled.content_type.as_deref(), Some("application/x-www-form-urlencoded"),
            "urllib sets this whenever `data` is present; ureq does not — vhl.rs:362 had to, and so \
             does this");
    }
```

- [ ] **Step 2: Write the refusal tests** — the four things substitution may not do, in the same
  module. These are the reason this task exists as its own task.

```rust
    #[test]
    fn a_header_value_that_would_carry_cr_or_lf_is_refused_not_sanitised() {
        // The one thing substitution could otherwise buy an attacker who controls a plan: a value
        // with a newline in it splits one header into two, or one request into two. Refused, not
        // stripped — a silently-repaired request is a request nobody can reason about.
        let mut captures = Captures::default();
        captures.put("zybooks", "evil", "ok\r\nX-Injected: yes");
        let mut s = step("zybooks.items", "zybooks", "GET", "https://zyserver.zybooks.com/v1/x");
        s.headers = vec![("Authorization".into(), "Bearer {{capture:evil}}".into())];
        let err = fill(&s, &secrets(), &captures).unwrap_err();
        assert_eq!(err.code, "bad_step");
        assert!(err.detail.contains("header"), "{}", err.detail);
        assert!(!err.detail.contains("X-Injected"), "the refusal never quotes the value: {}", err.detail);
    }

    #[test]
    fn a_capture_from_one_source_can_never_be_substituted_into_another_sources_step() {
        let mut captures = Captures::default();
        captures.put("zybooks", "zybooks_token", "SECRET");
        let mut s = step("vhl.login", "vhl", "POST", "https://www.vhlcentral.com/user_session");
        s.headers = vec![("Authorization".into(), "Bearer {{capture:zybooks_token}}".into())];
        let err = fill(&s, &secrets(), &captures).unwrap_err();
        assert_eq!(err.code, "bad_step");
        assert!(!err.detail.contains("SECRET"));
    }

    #[test]
    fn a_credential_placeholder_naming_another_source_is_refused_even_when_that_credential_exists() {
        // The attack this closes: a VHL step asking for `{{credential:zybooks:password}}` and being
        // pointed at a VHL host. Both halves are legal on their own; together they are exfiltration.
        let mut s = step("vhl.login", "vhl", "POST", "https://www.vhlcentral.com/user_session");
        s.body = Some(Body::Form(vec![("x".into(), "{{credential:zybooks:password}}".into())]));
        let err = fill(&s, &secrets(), &Captures::default()).unwrap_err();
        assert_eq!(err.code, "bad_step");
        assert!(!err.detail.contains("p@ss"));
    }

    #[test]
    fn an_unknown_placeholder_form_is_refused_and_never_passed_through() {
        for bad in ["{{credential:zybooks:token}}", "{{session:zybooks}}", "{{capture:}}", "{{credential:zybooks}}"] {
            let mut s = step("zybooks.items", "zybooks", "GET", "https://zyserver.zybooks.com/v1/x");
            s.headers = vec![("X-Test".into(), bad.into())];
            let err = fill(&s, &secrets(), &Captures::default()).unwrap_err();
            assert_eq!(err.code, "bad_step", "{bad}");
        }
        // And a literal that merely LOOKS like one is passed through untouched: the vendor's own
        // pages contain braces, and a body is not a template.
        let mut s = step("zybooks.items", "zybooks", "GET", "https://zyserver.zybooks.com/v1/x");
        s.headers = vec![("X-Test".into(), "{{ not a placeholder }}".into())];
        assert!(fill(&s, &secrets(), &Captures::default()).is_ok());
    }

    #[test]
    fn a_missing_credential_is_its_own_code_because_the_fix_is_the_students() {
        let mut s = step("vhl.login", "vhl", "POST", "https://www.vhlcentral.com/user_session");
        s.body = Some(Body::Form(vec![("u".into(), "{{credential:vhl:username}}".into())]));
        let err = fill(&s, &Secrets::fixed(&[]), &Captures::default()).unwrap_err();
        assert_eq!(err.code, "no_credential");
        assert_eq!(err.detail, "vhl", "the SOURCE is named, never the target name and never a value");
    }

    #[test]
    fn a_method_other_than_get_or_post_is_refused() {
        // Plans are read-only at the vendor (spec §8): a POST may be a login or a query, never a
        // submission, and PUT/DELETE/PATCH have no reading of them that is read-only.
        for method in ["PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "get"] {
            let s = step("zybooks.x", "zybooks", method, "https://zyserver.zybooks.com/v1/x");
            assert_eq!(fill(&s, &secrets(), &Captures::default()).unwrap_err().code, "bad_step", "{method}");
        }
    }

    #[test]
    fn the_password_is_exposed_in_exactly_one_place() {
        // `Secret` has no Display, so printing one is a compile error; `expose()` is the one call
        // that can defeat that, and this is the count that keeps it at one.
        //
        // **Counted over the pre-`cfg(test)` half only** (review I6). The first draft counted the
        // whole file, which contains the real call site AND the `".expose()"` literal inside this
        // very assertion — so it read 2 and failed on the day it was written, and the comment then
        // invited the implementer to "change the number", which is how a pin gets defeated. The
        // split is the shape `the_engine_holds_no_portal_url_and_no_login_flow` already uses.
        // **Split on the attribute FOLLOWED BY `mod tests`** (review **R5**), not on the attribute
        // alone: Task 1 puts a `#[cfg(test)] fn extra_source` MID-FILE, before `substitute_one`
        // exists, so a bare split truncates the head to code containing no `.expose()` at all — the
        // assertion reads 0, fails, and the obvious fix is to change the number, which is the
        // defeat this test exists to prevent.
        let src = include_str!("relay.rs");
        let code = split_before_test_module(src);
        assert_eq!(code.matches(".expose()").count(), 1,
            "engine/src/relay.rs must reach a password in exactly ONE function (`substitute_one`). \
             This number is a guarantee, not a tally: a second call site is a second place a \
             password can be printed, and the fix is to route through the first one.");
    }
```

- [ ] **Step 3: Run them and watch them fail.**

Run: `cargo test -p knowlu-engine relay::tests`
Expected: FAIL to compile — `cannot find type Step`, `cannot find function fill`, `cannot find type
Secrets`, `cannot find type Captures`, `cannot find value MAX_BYTES`.

- [ ] **Step 4: Write the step types and the substitution.**

```rust
/// One request the cloud composed. **The device never builds one of these** — it decodes one.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// Ours, never a vendor URL: `zybooks.signin`, `vhl.dashboard`. It is what the run log names and
    /// what the server counts, so it must be safe to print.
    pub id: String,
    pub source: String,
    /// `GET` or `POST` and nothing else (spec §8: plans are read-only at the vendor).
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Body>,
    pub capture: Vec<Capture>,
    pub follow_redirects: bool,
    pub max_bytes: u64,
}

/// Exactly one of four shapes. **The cookie jar is never named**: a step uses its `source`'s jar,
/// always, so a plan cannot reach another source's jar and cannot ask for none.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Json(serde_json::Value),
    /// Ordered pairs, urlencoded by the device with `quote_plus` — the order is the vendor's own
    /// form order, which `parse_user_session_form` preserved for the same reason.
    Form(Vec<(String, String)>),
    Text(String),
}

/// A value the device keeps and (when `redact`) removes from the body it returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Capture {
    pub name: String,
    /// `"json"` and nothing else. A closed set of one, on purpose: a regex extractor would be a
    /// general-purpose scraper the server could point at anything in a response. What a plan needs
    /// from a page it reads server-side from the body it was handed.
    pub from: String,
    /// RFC 6901 JSON pointer, e.g. `/session/auth_token`.
    pub pointer: String,
    pub redact: bool,
    pub persist: bool,
    pub ttl_s: u64,
}

/// A request with every placeholder filled. Produced only by [`fill`].
#[derive(Debug, Clone)]
pub struct Filled {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body_bytes: Option<Vec<u8>>,
    pub content_type: Option<&'static str>,
    /// Every secret that went into this request, for [`scrub`] to run over any error it produces.
    /// Dropped with the request; never logged, never returned.
    pub secrets: Vec<String>,
}

/// A failure, from the closed set spec §2.4 names: `host_not_allowed`, `unknown_source`,
/// `no_credential`, `bad_step`, `transport`, `timeout`, `too_large`, `budget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepError {
    pub code: &'static str,
    /// Already scrubbed and one-lined. Never a credential, never a captured value, never a URL.
    pub detail: String,
    /// The offending host, for `host_not_allowed` only.
    pub host: Option<String>,
}
```

- [ ] **Step 5: Write `Secrets`, `Captures` and `fill`.**

```rust
/// The Credential Manager seam. **The only place a portal password is read**, and on a non-Windows
/// build there is nothing to read — the credential store is Windows-only (spec §6.5), so a cloud
/// build compiles and simply cannot authenticate, which is the shape `fetch_zybooks` already had.
pub struct Secrets {
    #[cfg(windows)]
    targets: std::collections::BTreeMap<String, String>,
    fixed: Option<std::collections::BTreeMap<String, (String, String)>>,
}

impl Secrets {
    /// From the vault's own `coursework.<source>.credential_target` values — the same read
    /// `fetch_zybooks` did at `coursework.rs:468`, and the reason `credential_target` stays
    /// device-side and out of `redact`'s allowlist.
    pub fn from_targets(targets: &[(String, String)]) -> Secrets { /* … */ }

    /// A test seam: nothing in this crate's tests touches the real Credential Manager.
    pub fn fixed(rows: &[(&str, &str, &str)]) -> Secrets { /* … */ }

    /// `(username, password)`, or `None` when this machine holds no credential for this source.
    fn get(&self, source: &str) -> Option<(String, String)> { /* … */ }
}

/// Values captured during this run, namespaced by source. `persist`ed entries are loaded from the
/// session store at the start of the run and written back at the end (Task 3).
#[derive(Debug, Default, Clone)]
pub struct Captures {
    by_source: std::collections::BTreeMap<(String, String), String>,
}

impl Captures {
    pub fn put(&mut self, source: &str, name: &str, value: &str) { /* … */ }
    fn get(&self, source: &str, name: &str) -> Option<&str> { /* … */ }
    /// Every value held, for [`scrub`] and for the reply redaction. Never printed.
    fn all(&self) -> Vec<&str> { /* … */ }
}

/// The three placeholder forms, and only these.
static PLACEHOLDER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"\{\{(credential:[a-z0-9_-]+:(?:username|password)|capture:[A-Za-z0-9_-]+)\}\}").unwrap()
});

/// Where a value is going, which decides how it is encoded (spec §2.3's table).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot { Url, FormValue, JsonString, HeaderValue, Text }

/// Resolve one placeholder and encode it for `slot`.
///
/// **The one function in this crate that touches a portal password** — `the_password_is_exposed_in_
/// exactly_one_place` pins that at exactly one `.expose()`.
fn substitute_one(
    token: &str, step_source: &str, secrets: &Secrets, captures: &Captures, slot: Slot,
) -> Result<String, StepError> {
    let raw = if let Some(rest) = token.strip_prefix("credential:") {
        let (source, field) = rest.split_once(':').ok_or_else(|| bad_step("a malformed credential placeholder"))?;
        // Namespaced: a step may only ask for ITS OWN source's credential. Both halves of this are
        // legal on their own and together they are exfiltration (see the test).
        if source != step_source {
            return Err(bad_step("a step asked for another source's credential"));
        }
        let (user, pass) = secrets.get(source).ok_or_else(|| StepError {
            code: "no_credential", detail: source.to_string(), host: None })?;
        match field { "username" => user, _ => pass }
    } else if let Some(name) = token.strip_prefix("capture:") {
        captures.get(step_source, name)
            .ok_or_else(|| bad_step("a step referenced a value this run has not captured"))?
            .to_string()
    } else {
        return Err(bad_step("an unknown placeholder form"));
    };
    Ok(match slot {
        Slot::Url => quote(&raw),
        Slot::FormValue => quote_plus(&raw),
        // The serialiser escapes it: the value goes back into a `serde_json::Value` and
        // `ledger::dumps_value` writes the bytes, so there is no second escaping rule here.
        Slot::JsonString | Slot::Text => raw,
        Slot::HeaderValue => {
            if raw.contains('\r') || raw.contains('\n') {
                return Err(bad_step("a header value would carry a line break"));
            }
            raw
        }
    })
}

/// Substitute every placeholder in a step and produce the bytes that go on the wire.
///
/// **Nothing else in this module composes a request.** `fill` is total over a step: it either
/// returns something sendable or refuses with a code from the closed set.
pub fn fill(step: &Step, secrets: &Secrets, captures: &Captures) -> Result<Filled, StepError> {
    if step.method != "GET" && step.method != "POST" {
        return Err(bad_step("a method other than GET or POST"));
    }
    // … url through Slot::Url; header VALUES through Slot::HeaderValue and header NAMES not
    // substituted at all (a name is not a place a value belongs); `Body::Json` walked to its string
    // leaves through Slot::JsonString and serialised with `ledger::dumps_value`; `Body::Form`
    // through Slot::FormValue and joined with `=`/`&` after `quote_plus` on BOTH halves, exactly as
    // `vhl::urlencode` did; `Body::Text` through Slot::Text.
    //
    // `content_type` is `application/json` for Json, `application/x-www-form-urlencoded` for Form,
    // `None` for Text (the plan sets one in `headers` if it wants one) — and a `Content-Type` the
    // plan set in `headers` always wins, so a plan can send a JSON body as `text/plain` if some
    // vendor needs that.
}

fn bad_step(why: &'static str) -> StepError {
    StepError { code: "bad_step", detail: why.to_string(), host: None }
}
```

  **Two decisions inside `fill`, stated so a reviewer can disagree with them.** (1) **Header names
  are never substituted.** A placeholder in a header name has no legitimate use and would let a plan
  turn a captured value into a header the device would then send to a host that logs header names.
  (2) **A refusal never quotes the value.** Every `detail` above is a `&'static str` chosen at the
  call site, which is why none of them can carry a secret even by accident.

- [ ] **Step 6: Run, then commit.**

Run: `cargo test -p knowlu-engine relay::`, then `cargo test --workspace`.
Expected: green at 0 warnings.

```bash
git add engine/src/relay.rs
git commit -F .git-commit-msg.txt   # "feat(engine): the relay's step shape, substitution and its four refusals (C5 Task 2)"
```

---

### Task 3: The session store — DPAPI, and the vendor's own `Set-Cookie` lines

**Read first:** spec §4 in full; `engine/src/vhl.rs:340-352` (`default_opener`'s doc comment, "the
single most important line in the module's network half"); and
`~/.cargo/registry/.../ureq-3.4.0/src/cookies.rs` lines 104-155 — `CookieJar::get/remove/insert/
iter/save_json/load_json` — which is where the two measured reasons for replaying raw lines come
from.

- [ ] **Step 1: Ask Quinn (Q4)**, in one message, when this task is reached: *The session store keeps
  two things per portal: the cookies the vendor set, and any value a plan asked the device to keep —
  today that is one, zyBooks' bearer token, which is password-equivalent for its lifetime. I have it
  at a 12-hour TTL, which means two slots a day use a token at most twice before it is re-minted.
  Shorter costs one extra sign-in per slot and buys a smaller window if the sealed file were ever
  read on the student's own machine by something running as them. 12 hours, or shorter?*
  Recommendation: 12 hours. Record the answer; the plan is built to 12 (`ttl_s: 43200`, set by the
  plan and never pinned by the device).

- [ ] **Step 2: Write the failing tests** — in `relay.rs`'s test module. **These are the tests that
  justify not using `ureq`'s own serialisation**, so both reasons are measured rather than asserted.

```rust
    #[test]
    fn a_cas_session_cookie_survives_a_round_trip_through_the_store_and_ureqs_own_jar_would_drop_it() {
        // The line VHL's CAS actually sets: no Expires, no Max-Age — a SESSION cookie — and a
        // Domain that spans www. and m3a.
        let line = "_vhl_session=abc123; path=/; domain=.vhlcentral.com; HttpOnly; Secure";
        let origin = "https://www.vhlcentral.com/user_session";

        // (a) `ureq`'s own jar drops it on save: `save_json` writes only PERSISTENT cookies.
        //     Proven here rather than believed, because the whole store design rests on it.
        let agent: ureq::Agent = ureq::Agent::config_builder().build().into();
        {
            let mut jar = agent.cookie_jar_lock();
            jar.insert(ureq::Cookie::parse(line, &origin.parse::<ureq::http::Uri>().unwrap()).unwrap(),
                       &origin.parse::<ureq::http::Uri>().unwrap()).unwrap();
            assert_eq!(jar.iter().count(), 1, "the jar holds it in memory");
        }
        // (b) …and `iter()` exposes only name and value, so a hand-rolled save through it loses the
        //     Domain that makes one jar span two hosts.
        {
            let jar = agent.cookie_jar_lock();
            let only = jar.iter().next().unwrap();
            assert_eq!((only.name(), only.value()), ("_vhl_session", "abc123"));
            // There is no `.domain()`, and that is the finding. If ureq ever grows one, this test
            // is where to reconsider the whole design.
        }

        // (c) What the store does instead: replay the vendor's own bytes into a fresh jar, and RFC
        //     6265's rules — domain, path, expiry, overwrite — come back with them.
        let mut rec = SessionRecord::default();
        rec.observe(line, origin, now());
        let fresh: ureq::Agent = ureq::Agent::config_builder().build().into();
        rec.replay_into(&fresh);
        let jar = fresh.cookie_jar_lock();
        assert!(jar.get(".vhlcentral.com", "/", "_vhl_session").is_some()
             || jar.get("vhlcentral.com", "/", "_vhl_session").is_some(),
            "the replayed cookie is scoped to the domain the vendor set, so the dashboard GET on \
             m3a.vhlcentral.com presents it — vhl.rs:342-344's 'single most important line'");
    }
```

```rust
    #[test]
    fn the_file_is_dpapi_sealed_and_holds_no_readable_cookie_or_token() {
        let dir = temp_dir("relay-session");
        let mut rec = SessionRecord::default();
        rec.observe("_vhl_session=abc123; domain=.vhlcentral.com; path=/", "https://www.vhlcentral.com/", now());
        rec.remember("zybooks_token", "TOKENVALUE", 43_200, now());
        SessionStore::at(&dir).save("vhl", &rec).expect("sealed");

        let raw = std::fs::read(dir.join("vhl.bin")).expect("the sealed file exists");
        let as_text = String::from_utf8_lossy(&raw);
        assert!(!as_text.contains("abc123"), "a cookie value is readable in the file");
        assert!(!as_text.contains("TOKENVALUE"), "a captured token is readable in the file");

        let back = SessionStore::at(&dir).load("vhl", now()).expect("unsealed");
        assert_eq!(back.cookies.len(), 1);
        assert_eq!(back.captured("zybooks_token", now()), Some("TOKENVALUE".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_captured_value_past_its_ttl_is_gone_and_a_corrupt_file_is_an_empty_session_not_a_failure() {
        let dir = temp_dir("relay-session-ttl");
        let mut rec = SessionRecord::default();
        rec.remember("zybooks_token", "TOKENVALUE", 60, now());
        SessionStore::at(&dir).save("zybooks", &rec).expect("sealed");
        let back = SessionStore::at(&dir).load("zybooks", now()).expect("unsealed");
        assert_eq!(back.captured("zybooks_token", now() + std::time::Duration::from_secs(61)), None);

        // A file written by another Windows user, another machine, or a half-finished write: DPAPI
        // refuses it. That is a session this device does not have, which is a LOGIN, not a failure —
        // the alternative is a slot that stops because a cache went bad.
        std::fs::write(dir.join("zybooks.bin"), b"not a sealed blob").unwrap();
        let back = SessionStore::at(&dir).load("zybooks", now()).expect("an unreadable store is empty, never an error");
        assert!(back.cookies.is_empty() && back.captures.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn with_no_session_dir_the_store_is_memory_only_and_writes_nothing() {
        // A hand-typed `knowlu-engine coursework` logs in fresh and leaves nothing behind (spec §4).
        let store = SessionStore::in_memory();
        let mut rec = SessionRecord::default();
        rec.remember("zybooks_token", "TOKENVALUE", 43_200, now());
        store.save("zybooks", &rec).expect("a memory store always succeeds");
        assert!(store.load("zybooks", now()).unwrap().captures.is_empty(), "nothing persisted");
    }

    #[test]
    fn the_report_the_cloud_sees_carries_names_and_expiries_and_no_values() {
        let mut rec = SessionRecord::default();
        rec.observe("_vhl_session=abc123; domain=.vhlcentral.com", "https://www.vhlcentral.com/", now());
        rec.observe("pref=x; domain=.vhlcentral.com; Max-Age=3600", "https://www.vhlcentral.com/", now());
        rec.remember("zybooks_token", "TOKENVALUE", 43_200, now());
        let json = crate::ledger::dumps_value(&rec.report(now()));
        // Review I11: a count alone cannot support "re-authenticate only on expiry" for a portal
        // whose session IS a cookie, so the report says how many are session cookies and when the
        // earliest of the rest dies.
        assert!(json.contains("\"count\": 2"), "{json}");
        assert!(json.contains("\"session_cookies\": 1"), "{json}");
        assert!(json.contains("\"earliest_expiry\": "), "{json}");
        assert!(json.contains("\"zybooks_token\""), "{json}");
        assert!(!json.contains("abc123") && !json.contains("TOKENVALUE"), "{json}");
        assert!(!json.contains("_vhl_session") && !json.contains("vhlcentral"),
            "not a cookie NAME and not a domain: {json}");
    }

    #[test]
    fn the_two_expiry_attributes_are_parsed_off_the_raw_line_with_max_age_winning() {
        // Ruling R-C5-plan-5 (review R3): `ureq::Cookie` exposes `name` and `value` and nothing
        // else, so I11's whole report rests on these twenty lines. RFC 6265 §5.2.1 / §5.2.2.
        let t0 = now();
        let at = |line: &str| cookie_expiry(line, t0);
        assert_eq!(at("a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT"),
                   Some(jiff::civil::datetime(2026, 10, 21, 7, 28, 0, 0)
                        .to_zoned(jiff::tz::TimeZone::UTC).unwrap().timestamp().as_second()));
        assert_eq!(at("a=1; Max-Age=60"), Some(t0.as_second() + 60));
        assert_eq!(at("a=1; path=/; HttpOnly"), None, "a session cookie — what CAS sets");
        assert_eq!(at("a=1; MAX-AGE=60"), Some(t0.as_second() + 60), "attribute names are case-insensitive");
        assert_eq!(at("a=1; Max-Age=not-a-number; Expires=Wed, 21 Oct 2026 07:28:00 GMT").is_some(), true,
                   "an unreadable Max-Age is IGNORED, not fatal (§5.2.2), so Expires stands");
        assert_eq!(at("a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT; Max-Age=60"), Some(t0.as_second() + 60),
                   "Max-Age wins over Expires whenever both are present (§4.1.2.2)");
        assert_eq!(at("a=1; Expires=nonsense"), None, "unreadable Expires reads as a session cookie");
    }

    #[test]
    fn an_expired_cookie_stops_being_counted_and_a_deleted_one_leaves() {
        // The other half of I11: `observe` that only ever pushed made the count meaningless, and
        // the count is what the cloud decides on.
        let mut rec = SessionRecord::default();
        rec.observe("a=1; domain=.vhlcentral.com; Max-Age=60", "https://www.vhlcentral.com/", now());
        rec.observe("b=2; domain=.vhlcentral.com; Max-Age=60", "https://www.vhlcentral.com/", now());
        rec.observe("b=; domain=.vhlcentral.com; Max-Age=0", "https://www.vhlcentral.com/", now());
        assert_eq!(rec.report(now())["cookies"]["count"], 1, "Max-Age=0 is a deletion");
        let later = now() + std::time::Duration::from_secs(61);
        assert_eq!(rec.report(later)["cookies"]["count"], 0);
    }
```

- [ ] **Step 3: Run them and watch them fail.**

Run: `cargo test -p knowlu-engine relay::tests::`
Expected: FAIL to compile — `cannot find type SessionRecord`, `cannot find type SessionStore`.

- [ ] **Step 4: Write the session store.**

```rust
/// What this device remembers about one portal between slots.
///
/// **Cookies and captured values — never a password.** A password is read from Credential Manager at
/// the moment of substitution and written nowhere.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionRecord {
    /// The raw `Set-Cookie` lines the run observed, each with the origin it came from, in order.
    pub cookies: Vec<ObservedCookie>,
    pub captures: Vec<CapturedValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservedCookie {
    pub line: String,
    pub origin: String,
    /// The cookie's own `Expires`/`Max-Age` as Unix seconds, or `None` for a **session** cookie
    /// (which is what CAS sets). Parsed once, when the line is observed, by [`cookie_expiry`].
    ///
    /// **Not through `ureq::Cookie`** (review **R3**, ruling **R-C5-plan-5**): `ureq::Cookie` is a
    /// newtype over `cookie_store::Cookie` with a private inner, and its whole public surface is
    /// `parse`, `name`, `value` and a `Display` of `name=value` (`ureq-3.4.0/src/cookies.rs:44-92`;
    /// `as_cookie_store` is `#[cfg(test)]`). `Expires` and `Max-Age` are unreadable through it, and
    /// `cookie_store` as a direct dependency is refused by this stream's own
    /// `dependency_boundary.rs` case. So the module parses the two attributes off the raw line it
    /// already owns — twenty lines and a test each — and `ureq::Cookie::parse` keeps the job it is
    /// good at, which is replaying the line into a jar under RFC 6265's storage rules.
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedValue { pub name: String, pub value: String, pub expires_at: i64 }

impl SessionRecord {
    /// Record one `Set-Cookie` line exactly as the vendor wrote it.
    ///
    /// **Why the raw line and not `ureq`'s own serialisation** (both halves measured in this task's
    /// first test): `CookieJar::save_json` writes only *persistent* cookies, and a CAS session
    /// cookie has no `Expires`/`Max-Age` — it would simply be dropped; and `CookieJar::iter`
    /// exposes only `name` and `value`, losing the `Domain` attribute that makes one jar span
    /// `www.` and `m3a.vhlcentral.com`. Replaying the vendor's own bytes through
    /// `Cookie::parse(line, &origin)` + `jar.insert` puts RFC 6265's rules — expiry, host-only vs
    /// domain, path, overwrite, `Max-Age=0` deletion — back where they were, and needs no `json`
    /// feature.
    /// Pushes, **replacing** any entry this line supersedes by (name, domain, path) and **dropping**
    /// one the vendor deleted with `Max-Age=0`, and prunes anything already expired at `now`
    /// (review **I11**). A list that only ever grows makes the count in [`report`] meaningless,
    /// and the count is what the cloud decides on. Bounded at `MAX_COOKIES`.
    pub fn observe(&mut self, line: &str, origin: &str, now: Timestamp) { /* … */ }

    pub fn remember(&mut self, name: &str, value: &str, ttl_s: u64, now: Timestamp) { /* … */ }

    /// `Expires` and `Max-Age` off a raw `Set-Cookie` line, as Unix seconds — or `None` for a
    /// session cookie, which is the one CAS sets and the whole reason [`report`] distinguishes them.
    ///
    /// **The rule, stated because we are implementing it rather than borrowing it** (RFC 6265
    /// §5.2.1 and §5.2.2):
    /// - attributes are the `;`-separated parts after the first `name=value` pair, each trimmed,
    ///   with the attribute name compared ASCII-case-insensitively;
    /// - **`Max-Age` wins over `Expires`** whenever both are present (§4.1.2.2), and is a decimal
    ///   count of seconds relative to `now`; a leading `-` or a value of `0` is a **deletion**,
    ///   which [`observe`] turns into a removal rather than an entry;
    /// - a `Max-Age` that is not a valid integer is **ignored**, not an error (§5.2.2);
    /// - `Expires` is an IMF-fixdate / RFC 1123 stamp (`Wed, 21 Oct 2026 07:28:00 GMT`), which is
    ///   what every vendor in reach actually emits; a value this parser cannot read is ignored and
    ///   the cookie counts as a **session** cookie, which is the conservative answer — the cloud
    ///   then composes the login steps and is at worst served the login page again.
    ///
    /// Pure, no I/O, and tested five ways: `Expires` alone, `Max-Age` alone, neither (a session
    /// cookie), `Max-Age=0` (a deletion), and both present with `Max-Age` winning.
    fn cookie_expiry(line: &str, now: Timestamp) -> Option<i64> { /* … */ }

    pub fn captured(&self, name: &str, now: Timestamp) -> Option<String> { /* … unexpired only */ }

    /// Replay into a fresh agent's jar. A line the vendor's own server wrote and this parser cannot
    /// read is skipped, not fatal: one bad cookie is a re-login, not a dead slot.
    ///
    /// **`Cookie::parse` takes an owned string here** (review **M8**): in ureq 3.4.0 it is
    /// `parse<S: Into<Cow<'a, str>>>(...) -> Cookie<'a>` and `CookieJar::insert` wants
    /// `Cookie<'static>` (`cookies.rs:63`, `:123`), so a `&String` borrowed out of `&self` yields a
    /// borrowed cookie that will not insert. `rec.line.clone()`, not `&rec.line`. The test above
    /// happens to pass a `&'static str` literal and therefore compiles either way, which is exactly
    /// how this would be discovered late.
    pub fn replay_into(&self, agent: &ureq::Agent) { /* … Cookie::parse(rec.line.clone(), &uri) … */ }

    /// **Expiries only — capture names, and for cookies not even that** (spec §2.1). This is what
    /// lets the cloud skip a plan's login steps while a session is still good, and it is the whole
    /// of what the cloud learns about a session.
    ///
    /// **Why a bare count is not enough** (review **I11**). For zyBooks the captured token carries
    /// its own `expires_at` and the cloud can decide. For VHL there is no capture at all — the
    /// session *is* the cookie — so a count was the only input, and a count that never falls (an
    /// `observe` that only pushes) cannot say whether a session is alive. So: how many cookies, how
    /// many of them are **session** cookies with no expiry at all, and the **earliest** expiry among
    /// the rest. No name, no value, no domain. `earliest_expiry: null` with `session_cookies > 0` is
    /// the honest answer "this may be alive and may not" — the cloud then composes the login steps
    /// and is right or is served the login page again, which is the same cost it pays today.
    /// Exit-gate 15(c) is what turns that into a measured number.
    pub fn report(&self, now: Timestamp) -> serde_json::Value {
        let live: Vec<&ObservedCookie> =
            self.cookies.iter().filter(|c| c.expires_at.is_none_or(|e| e > now.as_second())).collect();
        serde_json::json!({
            "cookies": {
                "count": live.len(),
                "session_cookies": live.iter().filter(|c| c.expires_at.is_none()).count(),
                "earliest_expiry": live.iter().filter_map(|c| c.expires_at).min(),
            },
            "captures": self.captures.iter().filter(|c| c.expires_at > now.as_second())
                .map(|c| serde_json::json!({"name": c.name, "expires_at": c.expires_at}))
                .collect::<Vec<_>>(),
        })
    }
}

/// `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\sessions\<source>.bin`, DPAPI-sealed — or nothing at
/// all when the caller passed no `--session-dir`.
///
/// **Never in the vault.** The vault is plain text and, under ruling 2, syncs to the account; a
/// cookie must not. **Not Credential Manager either**, and the reason is a number: its blob cap is
/// 2,560 bytes and a realistic CAS jar is larger.
pub enum SessionStore { Dir(std::path::PathBuf), Memory }
```

```rust
impl SessionStore {
    pub fn at(dir: &std::path::Path) -> SessionStore { SessionStore::Dir(dir.to_path_buf()) }
    pub fn in_memory() -> SessionStore { SessionStore::Memory }

    /// `None` is never an error: a store that cannot be read is a session this device does not have,
    /// which is a login.
    /// Prunes expired cookies and expired captures on the way out (review **I11**), so a record
    /// read from disk reports the same thing a record built this run would.
    pub fn load(&self, source: &str, now: Timestamp) -> Option<SessionRecord> { /* … */ }
    pub fn save(&self, source: &str, record: &SessionRecord) -> Result<(), String> { /* … */ }
}

/// DPAPI, current-user scope, no UI, no extra entropy.
///
/// `CRYPTPROTECT_UI_FORBIDDEN` because this runs inside a scheduled slot with no window: a prompt
/// here would hang the step until `CHILD_TIMEOUT` killed it. Current-user scope is what makes the
/// file useless copied to another machine or opened by another Windows user on this one.
#[cfg(windows)]
fn seal(plain: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    let mut input = CRYPT_INTEGER_BLOB { cbData: plain.len() as u32, pbData: plain.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` points at `plain`, which outlives the call; `out` is a valid out-parameter and
    // its buffer is freed on every path below.
    unsafe { CryptProtectData(&mut input, windows::core::PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out) }
        .map_err(|e| format!("the session store could not be sealed ({})", e.code().0))?;
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { LocalFree(Some(HLOCAL(out.pbData as *mut core::ffi::c_void))) };
    Ok(bytes)
}
```

  `unseal` is `CryptUnprotectData` with the same shape and the same `LocalFree`, returning
  `Result<Vec<u8>, String>`; on a non-Windows build both are `Err("the session store is
  Windows-only")` and `SessionStore::Dir` degrades to `Memory`, which is the same
  compiles-but-cannot-authenticate shape `fetch_zybooks` already had.

  The sealed plaintext is `ledger::dumps_value` over `{"version": 1, "cookies": [...],
  "captures": [...]}` — the crate's one JSON writer, so the file has the same separators as every
  other JSON this engine writes, and a version field because a store shape that changes should read
  as "no session" rather than as garbage.

- [ ] **Step 5: Run, then commit.**

Run: `cargo test -p knowlu-engine relay::`, then `cargo test --workspace`.

```bash
git add engine/src/relay.rs engine/Cargo.toml
git commit -F .git-commit-msg.txt   # "feat(engine): the relay's DPAPI-sealed per-source session store (C5 Task 3)"
```

  `engine/Cargo.toml`'s only change is the `windows` feature list, which gains
  `"Win32_Security_Cryptography"`. **No crate is added, and `Cargo.lock` is expected not to move at
  all** (review M12 — that crate's features are internal), so it is **not** in the `git add`; if
  `git status` shows it dirty, stop and say why before staging a shared-single-owner file.
  `dependency_boundary.rs` is re-run in this step's `cargo test --workspace` to prove the first half.

---

### Task 4: The executor — one agent per source, the redirect chain the device follows itself, and the budgets

**Read first:** spec §2.4 and §2.5; `engine/src/zybooks.rs:400-426` (`default_send`, the 60-second
`timeout_global` §2.5 borrows) and `engine/src/vhl.rs:351-379` (`default_opener`, the one-agent rule);
`app/src/scheduler.rs:27` (`CHILD_TIMEOUT`, 20 minutes).

- [ ] **Step 1: Ask Quinn (Q5)**, in one message: *The relay's whole-run budget is 10 minutes, half
  the scheduler's 20-minute child timeout. That is the number that guarantees the relay is never what
  the scheduler kills — a slow portal ends the run with a named warning and a green tray instead of a
  killed child and an amber one. Confirm 10?* Record the answer; the plan is built to 10, and the
  test that compares the two refuses anything at or above 20.

- [ ] **Step 2: Write the failing tests** — appended to **`engine/src/relay.rs`'s own
  `#[cfg(test)] mod tests`**, not to a file under `engine/tests/`. **This is review C1 and it is not
  a preference.** Cargo compiles the library **without** `cfg(test)` for an integration test, so a
  portal suite in `engine/tests/relay_contract.rs` would see exactly what `relay_allowlist.rs`
  asserts it sees — `check_host("loopback", …) == Err(UnknownSource)` — and every test below would
  fail with `unknown_source`. Spec §9's first bullet says in-module and cites
  `coursework.rs:3095-3160` as the precedent; the first draft of this plan moved away from it and
  contradicted itself in the file's own module doc. **There is no `engine/tests/relay_contract.rs`
  in this plan**, and `Captures::get_for_test` goes with it: a `pub` accessor on a struct holding
  captured bearer tokens is a door that should not exist, and an in-module test needs no accessor at
  all.

  The loopback harness is `engine/tests/cloud_contract.rs:15-60`'s, copied into the module verbatim
  (a second harness would be a second thing to keep true) — exactly as `coursework.rs:3095-3160`
  already copies it for `post_coursework` — with a second listener standing in for the portal. Both
  threads are joined.

```rust
    //! (module-doc comment at the top of `mod tests`)
    //! The device end of the relay protocol, over two real loopback sockets: one standing in for
    //! `POST /relay`, one standing in for a portal. No DNS, no route off the machine; both listener
    //! threads are joined before each test returns.
    //!
    //! The portal side is reachable because this module carries a `#[cfg(test)]` row
    //! `{name: "loopback", hosts: &["127.0.0.1"]}` — which is why these tests are HERE and not in
    //! `engine/tests/`: an integration test links this library without `cfg(test)` and would not
    //! see the row. `engine/tests/relay_allowlist.rs` is that integration test, and it asserts the
    //! row's absence from a real build.

#[test]
fn a_redirect_chain_is_followed_by_the_device_and_every_hop_is_checked() {
    // 302 to a host that is NOT on this source's list. The device follows the chain itself
    // (`max_redirects(0)` on the agent) precisely so this hop is checked before a socket opens.
    let portal = portal(vec![redirect(302, "https://evil.example/landing")]);
    let result = perform_one(&step_to(&portal, "loopback.home"), &secrets(), &mut Captures::default(), &jar());
    assert_eq!(result.error.as_ref().unwrap().code, "host_not_allowed");
    assert_eq!(result.error.as_ref().unwrap().host.as_deref(), Some("evil.example"));
}

#[test]
fn a_set_cookie_on_a_302_is_recorded_because_cas_sets_one_there() {
    // ureq's own follower would neither consult our table nor let us see this header — which is the
    // whole reason `max_redirects(0)` is not a preference (spec §2.2).
    let portal = portal(vec![
        with_header(redirect(302, "/next"), "Set-Cookie", "_sess=abc; path=/"),
        ok_body("done"),
    ]);
    let mut record = SessionRecord::default();
    let result = perform_one_recording(&step_to(&portal, "loopback.login"), &mut record);
    assert!(result.ok);
    assert_eq!(record.cookies.len(), 1, "the 302's cookie was recorded");
    assert_eq!(result.redirects.len(), 1);
    // …and it is NOT in the reply the cloud sees.
    assert!(result.headers.iter().all(|(k, _)| k != "set-cookie"), "C5-D6");
}

#[test]
fn more_than_five_hops_is_a_refusal_not_a_loop() { /* 6 redirects → code "transport", detail names the hop cap */ }

#[test]
fn a_body_over_max_bytes_is_refused_and_carries_no_body_at_all() {
    // A half page parses silently wrong, which is the failure this module exists to avoid.
    let portal = portal(vec![ok_bytes(&vec![b'x'; 2 * 1024 * 1024 + 1])]);
    let result = perform_one(&step_to(&portal, "loopback.big"), &secrets(), &mut Captures::default(), &jar());
    let err = result.error.unwrap();
    assert_eq!(err.code, "too_large");
    assert!(result.body.is_none() && result.body_b64.is_none());
}

#[test]
fn a_body_that_is_not_valid_utf8_comes_back_base64_and_a_bom_comes_back_whole() {
    let bom_json = [b"\xEF\xBB\xBF{\"success\": true}".to_vec()].concat();
    let r = perform_one_on(&portal(vec![ok_bytes(&bom_json)]));
    assert_eq!(r.body.as_deref(), Some("\u{feff}{\"success\": true}"),
        "byte for byte, BOM included — the plan's own decoder strips it now (spec §2.4)");
    let r = perform_one_on(&portal(vec![ok_bytes(&[0xff, 0xfe, 0x00])]));
    assert!(r.body.is_none());
    assert_eq!(r.body_b64.as_deref(), Some("//4A"));
}

#[test]
fn a_captured_value_is_kept_here_and_redacted_out_of_the_body_the_cloud_sees() {
    // C5-D5: zyBooks' bearer token is password-equivalent for its lifetime. The cloud still reads
    // `success` and `user.user_id` out of this body, as it must; it never sees the token.
    let body = r#"{"success": true, "session": {"auth_token": "TOKENVALUE"}, "user": {"user_id": 7}}"#;
    let mut captures = Captures::default();
    let mut s = step_to(&portal(vec![ok_body(body)]), "loopback.signin");
    s.capture = vec![Capture { name: "tok".into(), from: "json".into(), pointer: "/session/auth_token".into(),
                               redact: true, persist: true, ttl_s: 43_200 }];
    let r = perform_one(&s, &secrets(), &mut captures, &jar());
    assert_eq!(r.captured, vec!["tok".to_string()]);
    assert!(!r.body.as_deref().unwrap().contains("TOKENVALUE"));
    assert!(r.body.as_deref().unwrap().contains("<captured:tok>"));
    assert!(r.body.as_deref().unwrap().contains("\"user_id\": 7"));
    // In-module (review C1), so this reads `Captures`'s private accessor directly and no `pub`
    // test door exists on a struct that holds captured bearer tokens.
    assert_eq!(captures.get("loopback", "tok"), Some("TOKENVALUE"));
}

#[test]
fn the_relay_payload_carries_no_credential() {
    // The successor to C2's `the_coursework_payload_carries_no_credential`, at the boundary that now
    // exists: every byte the device POSTs to `/relay` across a whole run, scanned.
    let sent = run_against_scripted_relay();          // the two-round-trip happy path
    for forbidden in ["p@ss w/rd", "hunter2", "student@example.edu", "TOKENVALUE", "_sess=abc",
                      "knowlu/", "credential_target", "AppData"] {
        assert!(!sent.contains(forbidden), "the relay sent `{forbidden}`");
    }
}

#[test]
fn the_budget_constants_are_the_specs_nine_rows() {
    // A pin on the numbers, and nothing more than that — which is why the test below exists.
    assert!(RUN_WALL_CLOCK < std::time::Duration::from_secs(20 * 60),
        "the relay must never be what the scheduler kills (app/src/scheduler.rs:27)");
    assert_eq!(RUN_WALL_CLOCK, std::time::Duration::from_secs(10 * 60));
    assert_eq!(STEP_TIMEOUT, std::time::Duration::from_secs(60), "zybooks.rs:402 and vhl.rs:353");
    assert_eq!((MAX_STEPS, MAX_BATCH, MAX_ROUND_TRIPS, MAX_REDIRECTS, MAX_BYTES, MAX_RUN_BYTES),
               (40, 8, 24, 5, 2 << 20, 8 << 20));
    assert_eq!(MAX_POST_BYTES, MAX_RUN_BYTES * 2,
        "the outgoing bound is DERIVED from the run's raw budget and JSON ESCAPING, not base64 \
         (review R2 correcting I1: `readJson` measures the JSON text, and a quote-dense page \
         approaches 2x, not 4/3) — and the server's readJson cap is the same number, so a 413 is \
         unreachable rather than unlikely");
    assert_eq!(MAX_POST_BYTES, 16 << 20, "16 MiB, the number the first round reached by the wrong route");
}

#[test]
fn a_stalled_portal_ends_the_run_at_the_deadline_and_never_at_the_scheduler_s() {
    // Review C4: the constants test above proves nothing about BEHAVIOUR, and exit-gate item 6
    // states the property as though it did. This is the property.
    //
    // The budget is constructed with a deadline a few milliseconds out (`RunBudget::at` is the
    // test-only constructor that takes one), the portal accepts and never answers, and the run must
    // come back on its own rather than on ureq's per-request timeout — which is what would have
    // taken 6 x 60 s per step, 8 steps to a batch.
    let portal = portal_that_accepts_and_stalls();
    let started = std::time::Instant::now();
    let out = run_with_deadline(std::time::Duration::from_millis(250), &portal);
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    assert!(out.done.is_none(), "a run that ran out of time writes nothing");
    assert!(out.warnings.iter().any(|w| w.contains("fetch failed") && w.contains("ran out of time")),
        "{:?}", out.warnings);
}

#[test]
fn a_redirect_chain_cannot_outlive_its_step_even_with_the_run_clock_wide_open() {
    // Review R4: the first round's version passed a RUN deadline of 250 ms, so it tested the run
    // bound twice and the step bound never. Here the run has ten minutes — the real number — and the
    // STEP clock is the only thing that can stop five stalling hops. Without `StepClock` this hangs
    // for five requests and fails on elapsed time, which is the failure C4 and R4 both describe.
    let portal = portal_that_redirects_and_stalls(5);
    let started = std::time::Instant::now();
    let r = perform_one_with_clocks(
        std::time::Duration::from_secs(600),      // the run: wide open
        std::time::Duration::from_millis(250),    // the step: what must bite
        &portal);
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
    assert_eq!(r.error.unwrap().code, "timeout", "the STEP's clock, not the run's budget");
}
```

- [ ] **Step 3: Write the protocol tests** — appended to the same `mod tests`: the loop, not the step.

```rust
#[test]
fn the_first_call_reports_sources_hosts_config_timezone_and_session_names_only() {
    let sent = first_request_of(run_against_scripted_relay());
    let body: serde_json::Value = serde_json::from_str(&sent).unwrap();
    assert_eq!(body["protocol"], 1);
    assert_eq!(body["job"], "coursework");
    assert!(body["run"].is_null());
    let source = &body["client"]["sources"][0];
    assert_eq!(source["name"], "zybooks");
    assert_eq!(source["hosts"][0], "*.zybooks.com");
    assert_eq!(source["has_credential"], true);
    assert_eq!(source["session"]["cookies"], 0);
    assert_eq!(body["client"]["timezone"], "America/Chicago");
    // `redact`'s allowlist, unchanged: `credential_target`, `base_url` and `enabled` never travel.
    assert!(source["config"].get("credential_target").is_none());
    assert!(source["config"].get("courses").is_some());
}

#[test]
fn the_device_stops_the_batch_at_the_first_failure_and_returns_what_it_has() {
    // Three steps, the second refused. Two results go back, not three, and the third is never sent.
    let sent = run_with(vec![batch_of_three_with_a_bad_host()]);
    let results = serde_json::from_str::<serde_json::Value>(&sent[1]).unwrap()["results"].clone();
    assert_eq!(results.as_array().unwrap().len(), 2);
    assert_eq!(results[1]["ok"], false);
    assert_eq!(results[1]["error"]["code"], "host_not_allowed");
}

#[test]
fn a_409_out_of_step_is_one_warning_and_not_a_re_drive() {
    // A replayed `results` for a run whose cursor has advanced. The device abandons it rather than
    // re-driving a half-finished plan (spec §2.6). The vault write is idempotent anyway, which is
    // why abandoning is safe and re-driving is the risk.
    let out = run_with_replies(vec![(200, steps_json()), (409, r#"{"error":"relay run out of step"}"#)]);
    assert!(out.warnings.iter().any(|w| w.contains("the run was abandoned")), "{:?}", out.warnings);
    assert!(out.done.is_none());
}

#[test]
fn every_service_failure_shape_is_a_named_line_and_nothing_changed() {
    for (code, expect) in [(401, "no session"), (402, "no entitlement"), (429, "rate limited"),
                           (500, "the service refused")] {
        let out = run_with_replies(vec![(code, r#"{"error":"x"}"#)]);
        assert!(out.done.is_none());
        assert!(out.warnings.iter().any(|w| w.contains(expect)), "{code}: {:?}", out.warnings);
        // `post_coursework` already owns this phrasing and `FAILURE_MARKERS` already sorts it to the
        // top of the run log (coursework.rs:1437). C5 adds no marker.
        assert!(out.warnings.iter().any(|w| w.contains("the service is unavailable")));
    }
}

#[test]
fn a_run_that_blows_a_budget_ends_as_a_warning_that_sorts_to_the_top_of_the_run_log() {
    // Review I14. Only ONE warning reaches `state/runner-log.md` — the rest collapse into `(+N
    // more)` — and which one wins is `rank_warnings`, which sorts by `FAILURE_MARKERS`
    // (coursework.rs:1437). The first draft's phrasings ("too many round trips", "batch too large",
    // "too much data") contain none of the seven markers, so a blown budget would sort BELOW a
    // benign "uncategorised; using default importance" and the only durable diagnostic under Task
    // Scheduler would hide it — the exact failure `rank_warnings`'s own doc comment exists to
    // prevent. So every budget warning is phrased with an existing marker and NO marker is added,
    // which also leaves `assert_eq!(FAILURE_MARKERS.len(), 7)` (coursework.rs:2671) alone.
    let cases = [
        (twenty_five_round_trips(), "coursework: fetch failed (the run hit its round-trip budget); nothing changed"),
        (nine_steps_in_one_batch(),  "coursework: fetch failed (the service sent more steps than a batch allows); nothing changed"),
        (bodies_totalling_nine_mib(), "coursework: fetch failed (the run hit its data budget); nothing changed"),
        (forty_one_steps(),          "coursework: fetch failed (the run hit its step budget); nothing changed"),
    ];
    for (replies, expected) in cases {
        let out = run_with_replies(replies);
        assert!(out.warnings.contains(&expected.to_string()), "{expected}: {:?}", out.warnings);
        assert!(out.done.is_none(), "a blown budget writes nothing");
        // And it wins the one line that survives.
        let mut mixed = vec!["zybooks: uncategorised; using default importance".to_string()];
        mixed.extend(out.warnings.clone());
        assert_eq!(crate::coursework::rank_warnings(&mixed)[0], expected);
    }
}
```

- [ ] **Step 4: Run them and watch them fail.**

Run: `cargo test -p knowlu-engine relay::tests`
Expected: FAIL to compile — `cannot find function perform`, `cannot find function run`, `cannot find
value RUN_WALL_CLOCK`, `cannot find type RunBudget`, `cannot find type SourceContext`.

- [ ] **Step 5: Write the executor.**

```rust
/// Spec §2.5, derived rather than chosen. Every one of these is a bound that turns a pathological
/// run into a named warning instead of a hung slot or an unbounded read.
pub const MAX_STEPS: usize = 40;            // 2 + n zyBooks calls + 3 VHL calls, with room for a third portal
pub const MAX_BATCH: usize = 8;             // one round trip per book would double a slot's latency on student wifi
pub const MAX_ROUND_TRIPS: usize = 24;      // 40 steps / a small batch, plus the final `done`
pub const MAX_REDIRECTS: usize = 5;         // CAS uses two; ten (ureq's default) is a loop
pub const MAX_BYTES: u64 = 2 << 20;         // ~38x the largest payload measured (zyBooks 54,874 B; VHL 7,495 B)
pub const MAX_RUN_BYTES: u64 = 8 << 20;
/// A bound on a WHOLE STEP, redirect chain included — not on one HTTP request (review C4). Today's
/// `timeout_global` (zybooks.rs:402, vhl.rs:353) was per request because the device made one.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Half of `scheduler::CHILD_TIMEOUT` (app/src/scheduler.rs:27, 20 minutes), and ENFORCED as a
/// monotonic deadline rather than compared to it in a test.
pub const RUN_WALL_CLOCK: Duration = Duration::from_secs(10 * 60);
/// What the device may put on the wire in one `results` POST, and it is **twice** `MAX_RUN_BYTES`,
/// not 4/3 of it (review **R2** correcting **I1**).
///
/// The expansion that matters is not base64's. `readJson` measures the JSON **text**
/// (`_shared/http.ts:49-52`), and a UTF-8 page travels JSON-**escaped**: every `"` becomes `\"`,
/// every backslash doubles, and a control character becomes six characters — worse than base64's
/// 4/3 and approaching 2x on a quote-dense HTML page. So the bound is `MAX_RUN_BYTES * 2`, which is
/// the same 16 MiB the first round chose by the wrong route, and the pin below says the factor.
pub const MAX_POST_BYTES: u64 = MAX_RUN_BYTES * 2;

/// Everything a run may spend, and the only thing that can end one early.
///
/// **The deadline is monotonic and is consulted, not compared** (review C4). The first draft ended
/// the loop "on `RUN_WALL_CLOCK`" between round trips while `STEP_TIMEOUT` sat on the agent as
/// `timeout_global` — which bounds ONE request, and the device issues up to `MAX_REDIRECTS + 1 = 6`
/// per step because it follows the chain itself. One step could take six minutes, a batch of eight
/// forty-eight, and the deadline was not consulted once inside it: the relay could be exactly what
/// `scheduler::CHILD_TIMEOUT` killed, which is the outcome this whole budget exists to prevent.
pub struct RunBudget {
    /// `Instant::now() + RUN_WALL_CLOCK`, taken once at the run's first instruction.
    deadline: std::time::Instant,
    steps_used: usize,
    round_trips: usize,
    bytes_used: u64,
}

impl RunBudget {
    pub fn new() -> RunBudget { /* … */ }
    /// What is left of the RUN, as a `Duration`. Never handed to a request on its own — see
    /// [`StepClock`].
    pub fn remaining(&self) -> Duration { /* … */ }
    /// `Err("budget")` when the deadline has passed. Called before every step, before every
    /// redirect hop and before every round trip.
    pub fn check_deadline(&self) -> Result<(), StepError> { /* … */ }
    pub fn charge_step(&mut self) -> Result<(), StepError> { /* … MAX_STEPS */ }
    pub fn charge_round_trip(&mut self) -> Result<(), StepError> { /* … MAX_ROUND_TRIPS */ }
    pub fn charge_bytes(&mut self, n: u64) -> Result<(), StepError> { /* … MAX_RUN_BYTES */ }
    /// The outgoing half of review I1: the serialised `results` body is measured before it is
    /// posted.
    ///
    /// **A batch that would exceed `MAX_POST_BYTES` is SPLIT, never trimmed** (review **R2**). The
    /// first round trimmed it and reported the dropped results as `too_large` — which calls a page
    /// the vendor successfully served "too large" and loses a book, and a missing assignment that
    /// no warning reports is the worst outcome this module has. Instead `run` posts the results
    /// that fit, then posts the rest as a second `results` call for the same `seq` — the protocol
    /// already allows a batch of one, and the extra round trip is charged like any other.
    /// `Err` here means a SINGLE result is over the bound on its own, which `MAX_BYTES` already
    /// makes impossible and which is therefore a bug, reported as `budget`.
    pub fn split_post(&self, results: &[StepResult]) -> Vec<usize> { /* … boundary indices … */ }
}

/// **The second deadline, and the one review R4 says the first round did not have.**
///
/// `budget.remaining()` clamped at `STEP_TIMEOUT` is a bound on one **request**: six hops of
/// fifty-nine seconds is a six-minute step, which is C4's arithmetic one level down, and the first
/// round's claim that "the whole chain is bounded by `STEP_TIMEOUT`" had no mechanism behind it.
/// So `perform` opens one of these at its first instruction and every request in the chain gets
/// `min(step.remaining(), budget.remaining())`. Two deadlines, each enforced once, and neither
/// expressible as the other.
pub struct StepClock {
    deadline: std::time::Instant,   // `Instant::now() + STEP_TIMEOUT`
}

impl StepClock {
    pub fn start() -> StepClock { /* … */ }
    pub fn remaining(&self) -> Duration { /* … */ }
    /// `Err(StepError { code: "timeout" })` when this step's own clock has run out — a distinct
    /// code from the run's `budget`, because "this portal stalled" and "the slot ran out of time"
    /// are different problems and the run log says which.
    pub fn check(&self) -> Result<(), StepError> { /* … */ }
}

/// One source, as the device describes it to the cloud on a run's first call. Built by
/// `collect_cloud` (Task 8) and by `discover_json` (Task 9); consumed by [`run`].
pub struct SourceContext {
    pub name: String,
    /// From `PORTAL_SOURCES`, so the cloud composes only for hosts this build would accept.
    pub hosts: &'static [&'static str],
    /// `yaml_to_json_for_request(&redact(&cfg, name))` — `credential_target` never travels.
    pub config: serde_json::Value,
    /// Whether Credential Manager holds one, so a plan can skip a source it could not fill.
    pub has_credential: bool,
    /// `SessionRecord::report(now)` — counts and expiries, no names and no values.
    pub session: serde_json::Value,
}

/// One agent per source per run — and therefore **one cookie jar per source per run**.
///
/// This is `vhl::default_opener`'s rule, moved and generalised: VHL's session cookie is set on the
/// login POST to `www.vhlcentral.com` and has to be presented to `m3a.vhlcentral.com` two requests
/// later; the cookie is scoped to `.vhlcentral.com`, so one jar covers both. An agent per request
/// would silently return an unauthenticated page with HTTP 200.
///
/// `max_redirects(0)` and `max_redirects_will_error(false)`: the device follows the chain itself, so
/// every hop passes through [`check_host`] and every hop's `Set-Cookie` is seen. ureq's own follower
/// would do neither — and CAS sets a cookie on a 302.
/// **No `timeout_global` on the agent** (review C4): a fixed 60 seconds here would bound one HTTP
/// request out of the six a redirect chain can issue. `perform` sets the timeout per request from
/// `budget.remaining()` clamped at `STEP_TIMEOUT`, so a step and a run are each bounded once.
fn agent_for_source(record: &SessionRecord) -> ureq::Agent {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .max_redirects_will_error(false)
        .http_status_as_error(false)
        .build()
        .into();
    record.replay_into(&agent);
    agent
}

/// Perform one step: check, fill, send, follow, cap, capture, shape.
///
/// The order is the guarantee. **`check_host` runs before `fill`**, so a refused host never touches
/// a password; and it runs again on every hop, on the URL the vendor's own `Location` gave us.
pub fn perform(
    step: &Step, agent: &ureq::Agent, secrets: &Secrets, captures: &mut Captures,
    record: &mut SessionRecord, budget: &mut RunBudget,
) -> StepResult { /* … */ }
```

  `perform`'s body, in order, with each line's reason:

  0. `let step_clock = StepClock::start();` — this step's own deadline, `now + STEP_TIMEOUT`
     (review **R4**).
  1. `budget.check_deadline()` → `budget`. **First**, so a run that is already out of time does not
     open one more socket (review C4).
  2. `check_host(&step.source, &step.url)` → `host_not_allowed` / `unknown_source`. **Before the
     credential is read.**
  3. `fill(step, secrets, captures)` → `bad_step` / `no_credential`.
  3a. **`check_host(&step.source, &filled.url)` again, and assert the two agree** (review **I4**).
     The URL that was checked must be the URL that is sent. Today the gap is probably not
     exploitable — a placeholder does not parse as an `http::Uri` authority and `Slot::Url`
     percent-encodes the delimiters — but "probably not exploitable" is not the standard this module
     is held to, and neither property was tested. Two cases in step 2: a placeholder in the
     authority is refused, and a substituted value cannot change the host.
  4. `budget.charge_step()` → `budget` when `MAX_STEPS` is spent.
  5. Send with `agent`, `GET` or `POST`, `Content-Type` from `Filled::content_type` unless the plan
     set one, and
     **`.config().timeout_global(Some(step.remaining().min(budget.remaining())))` per request** —
     the **smaller of two deadlines** (review **R4**), so neither a request nor a whole redirect
     chain can outlive its step, and no step can outlive the run. A fixed `STEP_TIMEOUT` per request
     would have been a six-minute step. On `Err`, `transport` with
     `scrub(&e.to_string(), &filled.secrets)` — and `scrub` replaces the raw, percent-encoded,
     **plus-encoded** and JSON-escaped forms of every secret, which is the four ways one reaches a
     third party's error text (review I2).
  6. For each `Set-Cookie` in the response, `record.observe(line, &current_url, now)` — **on every
     hop**.
  7. If `3xx` and `follow_redirects` and hops `< MAX_REDIRECTS`: **`step.check()` and
     `budget.check_deadline()`**, resolve `Location` against the current URL, `check_host` it, push
     `{status, location}` onto `redirects`, and loop. The chain is bounded by `STEP_TIMEOUT` because
     `StepClock` is checked on every hop and caps every request in it, not because a constant is
     named nearby.
     Past the hop cap, `transport` naming it. **The resolution rule, stated because there is no
     library for it** (review **M9**: `http::Uri` has no relative-resolution API and the `url` crate
     is forbidden by this stream's own dependency test): an **absolute** `Location` is used as-is; a
     **root-relative** one (`/next`) is a path-and-query swap on the current `Uri`'s scheme and
     authority; **anything else** — `next`, `../next`, an empty `Location` — is `transport` naming
     "an unresolvable redirect", and is tested. A refusal here is safe: every hop is re-checked
     anyway, so the risk of guessing would be a wrong fetch, not a wrong host.
  8. `body_mut().with_config().limit(step.max_bytes.min(MAX_BYTES) + 1).read_to_vec()`. A read that
     errors with ureq's `BodyExceedsLimit` becomes `too_large` **with no body at all** — over the cap
     is a refusal, not a truncation.
  9. `budget.charge_bytes(n)` → `budget` past `MAX_RUN_BYTES`.
  10. Captures: for each `Capture` with `from == "json"`, parse the body as JSON (a body that is not
      JSON simply captures nothing — it is the plan's error, reported as the step succeeding with an
      empty `captured`, because the plan is what will notice), `pointer()` it, store it in `captures`
      under this step's source, and — when `persist` — `record.remember(name, value, ttl_s, now)`.
  11. Shaping, in this order:
      - drop **every** `Set-Cookie` from `headers` (C5-D6) and lower-case the header names that
        remain;
      - **redact at the pointer, not by text replace** (review **I5**). The captured value came from
        a JSON pointer into the *parsed* body, so its textual form in the raw bytes may differ — a
        token containing a slash, a quote or a non-ASCII character is escaped on the wire and a text
        replace misses it, letting the token travel while `captured: […]` claims it did not; and a
        short value (a numeric id) matches elsewhere and corrupts what the parser reads. `Capture.from`
        is a closed set of one, so the body is already known to be JSON: replace the value **in the
        parsed `serde_json::Value`** and re-serialise through `ledger::dumps_value`. **Re-serialise
        only when at least one `redact: true` capture actually fired** — otherwise the bytes pass
        through untouched, which is what keeps the BOM test true. One test with a token carrying a
        character the serialiser escapes.
      - **scrub the BYTES, before the decode decision** (review **I3**, corrected by review **R9**).
        Nothing scrubbed the body the device returns, and there is a concrete case: VHL's rejected
        login **serves the login page again** (`vhl.rs:1077`), and a re-served form commonly carries
        the submitted username back in a `value=` attribute — so a credential half travels to our
        servers verbatim. The first round scrubbed the decoded `String` and then chose UTF-8 or
        base64, which left the body that most needs it — a login page re-served in a legacy encoding,
        exactly I3's own case — travelling base64 and unscrubbed. So: for each secret in
        `Filled::secrets`, and for **each of `scrub`'s four encodings of it**, search the raw byte
        slice for that encoding's UTF-8 bytes and replace them with `<redacted>`; then decode.
        A secret whose bytes are not present costs one `memmem` over the body and changes nothing.
        Two tests: a portal that echoes the posted form as UTF-8, and one that echoes it as
        Latin-1 — the second travels `body_b64` and the base64 must not contain the username.
      - decode the (scrubbed) bytes as UTF-8 — **BOM kept** — into `body`, or, when they are not
        valid UTF-8, into `body_b64`.

```rust
/// The reply the cloud sees. Spec §2.4's shape, and nothing that is not in it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepResult {
    pub id: String,
    pub ok: bool,
    pub status: u16,
    pub final_url: String,
    pub redirects: Vec<Redirect>,
    /// Lower-cased names, **never `set-cookie`**.
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_b64: Option<String>,
    pub bytes: u64,
    /// The NAMES captured, never the values.
    pub captured: Vec<String>,
    pub elapsed_ms: u64,
    pub error: Option<StepError>,
}

/// RFC 4648 §4, standard alphabet, padded. Twenty lines, because C3′ removed the `base64` crate with
/// the envelope and a crypto crate that comes back for a data encoding is a dependency nobody
/// re-audits. Pinned against RFC 4648 §10's own vectors in `b64_matches_rfc4648`.
fn b64(bytes: &[u8]) -> String { /* … */ }
```

  **The loop**, `relay::run`:

```rust
pub enum Job { Coursework, Discover }

pub struct RunOutcome {
    /// The `done` payload, whatever the job's own shape is. `None` when the run did not reach it.
    pub done: Option<serde_json::Value>,
    /// Named lines, already scrubbed and one-lined, for the caller's `warnings` vector.
    pub warnings: Vec<String>,
}

/// Drive one run. **This is the only function in the crate that talks to `/relay`.**
///
/// Nothing here writes anything anywhere. A run that dies mid-way writes **nothing** — no note, no
/// journal record, no partial ingest — because the only write path is `sync_coursework` on the
/// `done` reply, and the abandoned `relay_runs` row expires. A retry is a **new run**; there is no
/// resume, and the vault write is idempotent already (`sync_coursework` keys on `source_uid`).
pub fn run(
    client: &crate::cloudmodel::CloudClient, job: Job, sources: &[SourceContext],
    secrets: &Secrets, store: &SessionStore, timezone: &str,
) -> RunOutcome { /* … */ }
```

  The loop: build `client` (the first body), `post("/relay", …)`, and then while the reply carries
  `steps`: perform them in order stopping at the first failure, post the results, count the round
  trip. It ends on `done`, on `MAX_ROUND_TRIPS`, on `RUN_WALL_CLOCK`, on a `CloudError` (fatal ends
  it; non-fatal ends it too, because there is no resume and a retry is the next slot), or on a 409.
  At the end, for each source whose `SessionRecord` changed, `store.save(source, record)`.

- [ ] **Step 6: Run, then commit.**

Run: `cargo test -p knowlu-engine relay:: --test relay_allowlist`, then `cargo test --workspace`.

```bash
git add engine/src/relay.rs
git commit -F .git-commit-msg.txt   # "feat(engine): the relay executor, its redirect chain and its budgets (C5 Task 4)"
```

---

### Task 5: `POST /relay` — the protocol server-side, the run row, and the parse hand-off

**Read first:** spec §5 and §6; `cloud/supabase/functions/ingest-coursework/{index.ts,handler.ts}`;
`cloud/supabase/functions/_shared/{entitlement,http,db}.ts`; `cloud/supabase/config.toml`'s function
block and its comment about `verify_jwt = false`.

- [ ] **Step 1: Ask Quinn (Q1)**, in one message: *Raw pages — the zyBooks JSON and the VHL dashboard
  HTML the device sends up for parsing. They live in the edge function's memory for one call, are
  parsed, and are dropped: zero retention. The alternative is a short diagnostic window, say 24 hours
  on a parse failure, so that when a vendor changes its markup I can fix the parser from the page
  that actually broke instead of from a guess. Zero is what I have built and what the privacy page
  will say. The reason I recommend it: the diagnostic path already exists and is consented — the
  issue report, with its preview-and-scrub screen — and a table of other people's coursework pages is
  a thing a breach would find. Zero, or a window?* Record the answer; the plan is built to zero.

- [ ] **Step 2: Write the failing migration test** — `cloud/supabase/migrations_relay_test.ts`, new,
  filtered to `20260918…` the way C3′'s own guard file is filtered to `20260912…`.

```ts
// Static pins on C5's one migration. Nothing here applies SQL (there is no Docker in this plan):
// these are the invariants that would otherwise only be discovered on a project that already has
// rows in it, which is the wrong time to discover them.
import { assert, assertEquals } from "@std/assert";

const HERE = new URL("./migrations/", import.meta.url);

async function ours(): Promise<Array<[string, string]>> {
  const out: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(HERE)) {
    if (entry.name.startsWith("20260918") && entry.name.endsWith(".sql")) {
      out.push([entry.name, await Deno.readTextFile(new URL(entry.name, HERE))]);
    }
  }
  out.sort();
  assert(out.length === 1, "C5 owns exactly one migration, stamped 20260918 (R-X-8)");
  return out;
}

Deno.test("the run row has row level security and a policy that is select-only", async () => {
  const [[name, sql]] = await ours();
  assert(/alter table public\.relay_runs enable row level security/i.test(sql), name);
  assert(/create policy[\s\S]*relay_runs[\s\S]*for select[\s\S]*account_id = \(select auth\.uid\(\)\)/i.test(sql),
    "the service role writes; a signed-in account may only read its own rows");
  assert(!/for (insert|update|delete)/i.test(sql), "no write policy: the service role is the only writer");
});

Deno.test("the row cascades from accounts, expires, and holds no column a page body could go in", async () => {
  const [[name, sql]] = await ours();
  assert(/references public\.accounts\s*\(\s*id\s*\)\s*on delete cascade/i.test(sql), name);
  assert(/expires_at timestamptz not null default now\(\) \+ interval '15 minutes'/i.test(sql), name);
  // Q1, zero retention: `cursor` is indices, codes, scraped form fields and the dashboard link —
  // never a page, never a cookie, never a captured value. The Deno test in handler_test.ts asserts
  // the SHAPE after every step of both plans; this asserts there is nowhere else for one to go.
  for (const forbidden of ["body", "html", "payload", "page", "cookie", "token", "secret"]) {
    assert(!new RegExp(`^\\s+${forbidden}\\s`, "im").test(sql), `${name}: a column named ${forbidden}`);
  }
});

Deno.test("C5 creates no function and no view, so C1's corpus pins do not move", async () => {
  const [[name, sql]] = await ours();
  assert(!/create (or replace )?function/i.test(sql), `${name}: see migrations_test.ts's pinned counts`);
  assert(!/create (or replace )?(materialized )?view/i.test(sql), name);
  assert(!/drop table|truncate/i.test(sql), `${name}: C5 drops nothing`);
});
```

- [ ] **Step 3: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations_relay_test.ts`
Expected: FAIL — `C5 owns exactly one migration, stamped 20260918`.

- [ ] **Step 4: Write the migration** — `cloud/supabase/migrations/20260918000100_relay.sql`:

```sql
-- C5 (cloud design, amendment 2026-09-17, ruling 4): the relay's own run state, and nothing else.
--
-- A run is a short sequence of steps inside one slot. The row holds where the plan is — indices,
-- zybook codes, the form fields scraped off the login page, the dashboard link the vendor's payload
-- gave us — and a handful of counters. It NEVER holds a page body, a cookie or a captured value:
-- raw pages are the student's data (ruling 2), received for the run, parsed, and not retained.
-- There is no cron job: the sweep is one delete at the top of every call, which is cheaper than a
-- schedule and cannot silently stop running.

create table public.relay_runs (
  id uuid primary key default gen_random_uuid(),
  account_id uuid not null references public.accounts (id) on delete cascade,
  -- The JOB, not a source: one run drives every source the device offered (ruling R-C5-plan-3),
  -- so 'coursework' and 'coursework-discover' are the only two values there are.
  plan text not null,
  -- A stable COMPOSITE of the per-source module versions (`zybooks@1+vhl@1`), so a deploy that
  -- changes either module mid-run still fires the 409 the device answers by starting over.
  plan_version text not null,
  -- Per source: `{"zybooks": {...}, "vhl": {...}}`. Indices, zybook codes, the scraped form
  -- fields, the dashboard link, and each source's own `stage`/`failed`/`reauthed` flags.
  cursor jsonb not null default '{}'::jsonb,
  -- The PARSED rows, and the whole of ruling R-C5-plan-2. An edge function holds nothing between
  -- invocations and a coursework run spans several round trips, so the rows a body parsed into
  -- have to live somewhere; raw pages may not (ruling 2, R4-19, Q1) and parsed rows may. This is
  -- that somewhere, appended to as each body arrives and deleted with the row at `done`.
  --
  -- The shape is `{assignments, own, proposals}` and ALL THREE are MAPS of source name to that
  -- source's rows (review R8 for `own`, review S1 for the other two: a flat `assignments` array
  -- could not be un-merged when a source is retired, and could not be ordered by PORTAL_SOURCES
  -- once two sources' round trips interleave. A default that disagrees with the writer is a column
  -- that silently starts life wrong.) `finishSource` turns `own[source]` into the prefixed
  -- `warnings` of the reply, once per source, at the end; `finishRun` flattens the other two.
  --
  -- WITHIN A SOURCE the array is in arrival order — append, never splice — because that is the
  -- order the vendor returned the shelf in, and `done` has to reproduce it.
  --
  -- ITS SIZE IS ALREADY BOUNDED, and by the budget that matters: `MAX_RUN_BYTES` (8 MiB of raw
  -- body per run) bounds everything that can ever be parsed into it. A twelve-book shelf is about
  -- 200 rows and a few hundred KB of jsonb. That is why a shelf is not capped and a thirteenth book
  -- is not a product limit. `parsed_holds_only_parsed_rows` (Task 5 step 5) is what keeps a page
  -- out, and it caps every string at 4 KiB for the same reason `checkCursor` does.
  parsed jsonb not null default '{"assignments": {}, "own": {}, "proposals": {}}'::jsonb,
  seq int not null default 0,
  steps_used int not null default 0,
  round_trips int not null default 0,
  bytes_used bigint not null default 0,
  started_at timestamptz not null default now(),
  expires_at timestamptz not null default now() + interval '15 minutes'
);

create index relay_runs_account_idx on public.relay_runs (account_id);
create index relay_runs_expires_idx on public.relay_runs (expires_at);

alter table public.relay_runs enable row level security;

-- The service role writes; a signed-in account may read its own rows and nothing else. There is no
-- insert, update or delete policy at all, which is this project's standing shape: the only writer
-- any table has is the service role behind an edge function that has already entitled the caller.
create policy relay_runs_own_select on public.relay_runs
  for select to authenticated
  using (account_id = (select auth.uid()));
```

- [ ] **Step 5: Write the failing protocol tests** — `cloud/supabase/functions/relay/protocol_test.ts`.
  The validators are what stop a malformed device reply from reaching a plan, and a malformed plan
  from reaching a device.

```ts
Deno.test("a start request is protocol 1, a known job, no run, and a client block", async () => {
  await assertRefusal(() => readStart({ protocol: 2, job: "coursework", client: {} }), 400, "protocol");
  await assertRefusal(() => readStart({ protocol: 1, job: "grades", client: {} }), 400, "job");
  const start = readStart({ protocol: 1, job: "coursework", run: null,
    client: { timezone: "America/Chicago", sources: [{ name: "zybooks", hosts: ["*.zybooks.com"],
              has_credential: true, config: { courses: {} }, session: { cookies: 0, captures: [] } }] } });
  assertEquals(start.client.sources[0].name, "zybooks");
});

Deno.test("a results request names a run and carries at most MAX_BATCH results", async () => {
  await assertRefusal(() => readResults({ protocol: 1, job: "coursework", run: "not-a-uuid", results: [] }), 400, "run");
  await assertRefusal(() => readResults({ protocol: 1, job: "coursework", run: UUID,
    results: new Array(9).fill(okResult()) }), 400, "batch");
});

Deno.test("the outgoing bound and the readJson cap are the same number, and the factor is escaping", async () => {
  // Review R2: `a_results_batch_over_the_cap_is_a_413_not_a_500` was prose in the Interfaces section
  // and lived in no task. This is it, and it pins the RELATIONSHIP rather than either number — the
  // device must never be able to post what this handler has to refuse.
  assertEquals(MAX_POST_BYTES, MAX_RUN_BYTES * 2);
  const rs = await Deno.readTextFile(new URL("../../../../engine/src/relay.rs", import.meta.url));
  assert(/MAX_POST_BYTES: u64 = MAX_RUN_BYTES \* 2;/.test(rs), "the device's bound is the same one");
  // One byte over, and it is a 413 in our shape — never a 500, and never a truncated parse.
  const res = await handle(postOf("x".repeat(MAX_POST_BYTES + 1)), deps()).catch((e) => e as Response);
  assertEquals(res.status, 413);
  assertEquals((await res.json()).error.includes("characters"), true);
});

Deno.test("a step a plan composes is validated before it is sent, and a bad one is a 500 not a bad request to a vendor", () => {
  // The plans are ours, so a malformed step is OUR bug — but it must not reach the device, because
  // the device's own refusal would be reported to the student as a portal problem.
  for (const bad of [{ method: "PUT" }, { url: "http://zyserver.zybooks.com/v1" }, { source: "" },
                     { headers: [["X", "a\r\nb"]] }, { body: { json: {}, form: [] } }]) {
    assertThrows(() => checkStep({ ...validStep(), ...bad }));
  }
  assertEquals(checkStep(validStep()).id, "zybooks.signin");
});

Deno.test("parsed holds only parsed rows, and no string in it is longer than a cursor's", () => {
  // Review R8. `checkCursor` caps strings at 4 KiB; `checkParsed` said only "rows only, never
  // bytes", and a `parse failed (<vendor message>)` warning can quote page text — the one way
  // something page-shaped reaches this column, and from there `state/runner-log.md`. Same cap, same
  // reason. And this test exists at all because the first round named it in the migration comment
  // and wrote it into no task.
  checkParsed({ assignments: [{ uid: "zybooks:1", title: "HW 01" }], own: { zybooks: [] }, proposals: [] });
  assertThrows(() => checkParsed({ assignments: [], own: { vhl: ["parse failed (" + "x".repeat(5000) + ")"] },
                                   proposals: [] }), Error, "parsed");
  assertThrows(() => checkParsed({ assignments: [], own: {}, proposals: [], html: "<html>" }), Error, "parsed");
  assertThrows(() => checkParsed({ assignments: [], warnings: [], proposals: [] }), Error,
    "the shape is {assignments, own, proposals} — `own` is per source and `warnings` is what " +
    "finishSource produces from it");
});

Deno.test("a cursor never carries a page body, and the guard is a scan and not a promise", () => {
  // Run after every step of both plans in handler_test.ts; here, the primitive.
  assertThrows(() => checkCursor({ books: [{ code: "UACS100Fall2026", payload: { a: 1 } }] }),
    Error, "cursor");
  assertThrows(() => checkCursor({ html: "<html>" }), Error, "cursor");
  checkCursor({ index: 2, codes: ["UACS100Fall2026"], form: [["lt", "LT-1"]], dashboard: "https://m3a…" });
});
```

- [ ] **Step 6: Write `protocol.ts`, `db.ts` and `handler.ts`.**

  **`protocol.ts`** — the wire types (`Step`, `Body`, `Capture`, `StepResult`, `StartRequest`,
  `ResultsRequest`), `readStart`/`readResults` (throwing `fail(400, …)` at the call site, C1's shape),
  `checkStep` (method in `{GET, POST}`, `url` starts `https://`, `source` non-empty, no CR/LF in any
  header value, exactly one body shape or none, `capture[].from === "json"`), `checkCursor` (the
  scan: no string value over 4 KiB, no key in `{body, html, payload, page, cookie, token}`), and the
  budget constants **spelled again here and pinned against the device's**:

```ts
// Spec §2.5. These are the SERVER's half of the same table `engine/src/relay.rs` carries; the two
// are pinned against each other by `the_budgets_the_server_enforces_are_the_ones_the_device_does`
// in handler_test.ts, which reads relay.rs by relative path and parses its constants. Two copies of
// a number is a defect unless something compares them, so something does — and since review R2 the
// comparison covers MAX_POST_BYTES too, which the first round left out of the reader.
export const MAX_STEPS = 40, MAX_BATCH = 8, MAX_ROUND_TRIPS = 24, RUN_TTL_MINUTES = 15;
// `MAX_RUN_BYTES * 2` — exported, because `handler.ts` names it at its `readJson` call and the
// first round named a constant this file did not export (review R2). The factor is JSON escaping.
export const MAX_RUN_BYTES = 8 << 20, MAX_POST_BYTES = MAX_RUN_BYTES * 2;
```

  **`db.ts`** — four calls over C1's `Rest`, every one scoped `account_id=eq.<id>`:
  `sweepExpired(rest)` (`restDelete(rest, "relay_runs", "expires_at=lt.now")` — **`now`, not
  `now()`** (review **I13**): PostgREST passes the filter value to Postgres as a literal, and
  Postgres accepts the special datetime input `now` but not the string `now()`. `restDelete` calls
  `ok()` (`_shared/db.ts:95-101`), which **throws** on a non-2xx, and this is step 3 of the handler —
  before `readJson` and before any run logic — so a wrong literal 500s **every** `/relay` call. No
  function in this repository does a time-filtered PostgREST call, so there is no precedent to copy
  and Task 5 step 9 gains a fifth staging proof that exercises it. It is also the one call
  deliberately *not* account-scoped, because it is a sweep of everybody's expired rows; Task 12's
  scan names it as the one exception, with the reason), `startRun`, `loadRun`, `advanceRun`,
  `appendParsed`, `endRun`.

  **`handler.ts`** — `relayHandler(entitle)`, and its body in order:
  1. `POST` only, else `methodNotAllowed(["POST"])`.
  2. `await entitle(req)` → `{account_id}` (throws 401/402, propagated by `index.ts`'s `asResponse`).
  3. `await sweepExpired(rest)`.
  4. `readJson(req, MAX_POST_BYTES)` — **16 MiB, and `protocol.ts` must EXPORT that constant**
     (review **R2**: the first round named it here and exported only three). It is
     `MAX_RUN_BYTES * 2`, and **the factor is JSON escaping, not base64**: `readJson` measures the
     JSON *text* (`_shared/http.ts:49-52`), and a UTF-8 page travels escaped — every `"` becomes
     `\"`, every backslash doubles, a control character becomes six characters — which is worse than
     base64's 4/3 and approaches 2x on a quote-dense page. The device charges the same bound
     outgoing, so a 413 is unreachable rather than merely unlikely. Say beside it that `readJson`
     does `await req.text()` **before** it measures and measures UTF-16 code units — it is a sanity
     bound, **not** a memory guard.

     **And the platform's own ceiling is measured, not assumed** (the author's concern 3, ruled a
     finding). Neither this plan nor the review can read Supabase's edge request-body limit from
     here, so Task 5 step 9 measures it on staging with a padded body at the cap and records the
     status and whether it reached the function at all. **The fallback is named now** so nobody has
     to invent one under time pressure: if the platform refuses below the cap, `MAX_BYTES` drops
     from 2 MiB to **1 MiB** — still about 19x the largest payload ever measured — and
     `MAX_POST_BYTES` falls with it, both numbers moving together because the pin ties them.
     **And the device gets a runtime fallback regardless**: a 413 mid-run is already
     `the service refused`, so `run` **halves the batch and continues** rather than ending the run —
     one extra round trip, charged like any other, and a run that would have died reports
     everything it fetched.
  5. `run === null` → `startRun`: `PLANS[job]` — **keyed by job, not by source** (ruling
     **R-C5-plan-3**) — call `plan.start(ctx)`, `checkStep` each step, `checkCursor`, insert the row
     with the composite `plan_version`, reply `{run, steps}`.
  6. otherwise → `loadRun` (`id=eq.<run>&account_id=eq.<id>`), refuse `404` when absent or expired;
     refuse **`409 "relay run out of step"`** when `seq` **or `plan_version`** does not match (a
     deploy mid-run changes the composite, and starting over is cheaper than reasoning about a plan
     that changed under a cursor); call `plan.next(ctx, results)`; **`appendParsed`** (below); either
     `{run, steps}` after `advanceRun`, or `{run, done}` after `endRun` (which deletes the row and
     its `parsed` with it).
  6a. **`appendParsed`, stated rather than implied** (review **S1**: the first draft said "whatever
     it parsed", which is not a rule and left R7's retirement clause with nothing to act on). For
     each `Parsed` the arrival produced, keyed by **its own `source` field** — never by position and
     never by a literal source name:
     - `parsed.assignments[source].push(...items)`, `parsed.own[source].push(...own)` and
       `parsed.proposals[source].push(...proposals)` — **append, never splice**, so within a source
       the order is arrival order, which is the order the vendor returned the shelf in;
     - when `failed` is set, **`delete parsed.assignments[source]` and `delete parsed.own[source]`,
       and keep `parsed.proposals[source]`** — which is R7's rule, and is now a line of code rather
       than a paragraph: a source that threw contributed no items and no warnings to a single
       `ingestHandler` call, and the proposals it had already pushed survived. Six books' rows do
       not survive book 7's dead session, and the partition is what makes "six books' rows"
       identifiable at all;
     - the failure sentence itself goes into `parsed.own[source]` **after** the delete, as that
       source's one remaining line, so `finishSource` reports it and nothing else.
  7. **`reauth` is composed by the plan, not by the handler** (review **I12**). `PlanResult`'s
     `reauth` carries no steps, and `handler.ts` names no source, so nothing could have composed the
     login steps. The contract, two lines in `mod.ts`: on `{kind: "reauth", source}` the handler
     writes `reauthed: true` into **that source's** cursor and calls `plan.start(ctx)` again, which
     fans out to that source's module; the module's own `next` is then responsible for re-issuing the
     step that failed once the login completes, and for answering `failed` rather than `reauth` the
     second time. `a_dead_session_on_the_item_list_is_not_an_empty_shelf` then tests a real path
     rather than a sentinel.
  8. budgets: `steps_used + steps.length > MAX_STEPS`, `round_trips + 1 > MAX_ROUND_TRIPS`,
     `steps.length > MAX_BATCH` → the run **ends** with `done` carrying the job's own warning channel
     (`warnings` for coursework, `errors` for discover) and a sentence **phrased with an existing
     `FAILURE_MARKER`** — `coursework: fetch failed (the run hit its round-trip budget); nothing
     changed` and its three siblings (review **I14**), because only one warning reaches
     `state/runner-log.md` and `rank_warnings` decides which. Never a 500, never a non-zero exit
     downstream.

- [ ] **Step 7: The incremental parse — ruling R-C5-plan-2, and the finding it answers.**

  **The problem (review C3).** The first draft said the `books` stage accumulated `{code, payload}`
  "into the payload accumulator the handler holds". **An edge function holds nothing between
  invocations.** The only state is the `relay_runs` row; `checkCursor` refuses a page body and any
  string over 4 KiB; Q1 rules zero retention anywhere else. Yet a twelve-book shelf yields eight
  steps with four more on the next round trip, so eight payloads had to survive a round trip with
  nowhere to be — and it bites below eight books too, because VHL's three steps are strictly
  sequential, so `vhl.dashboard`'s HTML lands in a later batch than `zybooks.book:*` on any shelf
  that fills a batch, and a single `ingestHandler` call needed both.

  **The ruling: parse incrementally.** Each raw body is parsed **as it arrives**, per source, and
  only the parsed rows are kept — in `relay_runs.parsed`, deleted with the row. Raw bodies are never
  stored, which is the whole of Q1 and R4-19: *raw pages are received for the run, parsed, and not
  retained; the parsed rows are what persists.* **A shelf is not capped** — the rejected alternative
  was "one batch or bust", which makes a thirteenth book a product limit, and a product limit is not
  something to discover on a student's laptop in week one.

  **What replaces the `ingestHandler` call.** The three functions hand-off-free in
  `ingest-coursework/handler.ts`, extracted there in this same task (*Interfaces*, contract 8):

```ts
import { sourceFailure } from "../ingest-coursework/handler.ts";
import { SOURCE_PLANS } from "./plans/mod.ts";

/**
 * One body, parsed the moment it arrives, appended to the run's parsed rows (ruling R-C5-plan-2).
 *
 * **The handler names no source** (review **R6**). The first round switched on
 * `source === "zybooks"` here, three paragraphs after stating three times that it does not — and a
 * third portal would have had to edit this function. Each module owns a `parse(ctx, result)`; this
 * calls it. `own` is that source's own warnings, accumulated across round trips in
 * `parsed.own[source]` so `finishSource`'s `{name}: ` prefixing and its
 * `0 assignments parsed; treating as failure` rule see exactly what one `ingestHandler` call saw.
 *
 * **And the throw is where the warning bytes are decided** (review **R7**). `ingestHandler` wraps
 * each source in a `try/catch` (`handler.ts:135`, `:168-176`) that turns `NotLoggedIn` into
 * `session invalid (…)` and anything else into `parse failed (…)`, **discards that source's `own`
 * while keeping the proposals already pushed**, and skips the `0 assignments parsed` line. An
 * equality test cannot see that, because it only ever exercises the happy path. So the catch is the
 * **fourth** extracted function, `sourceFailure`, and the rule it carries across round trips is
 * stated below.
 */
function parseArrival(run: RunRow, source: string, result: StepResult): Parsed {
  const sub = run.cursor.sources[source];
  try {
    return SOURCE_PLANS[source].parse(planCtx(run, source), result);
  } catch (e) {
    // `{source, items: [], own: [], proposals: <kept>, failed: "<the one warning>"}` — exactly what
    // the endpoint's own catch produces, and `appendParsed` reads `failed` as this source's
    // retirement: it deletes `parsed.assignments[source]` and `parsed.own[source]`, keeps
    // `parsed.proposals[source]`, and records the one warning. `source` is on the `Parsed` itself
    // (review S1) so no caller has to remember which arrival it came from.
    return sourceFailure(source, e, sub.proposals);
  }
}

/**
 * `done`, assembled from the parsed rows at the end of the run.
 *
 * **The order is the contract.** One `ingestHandler` call produced sources in the order the device
 * sent them, which was always `["zybooks", "vhl"]`, and within zyBooks the shelf order the item
 * list returned. Round-robin execution must not change that.
 *
 * **This is the ONLY place `parsed` is flattened** (review **S1**). Everywhere else it stays
 * partitioned by source, because a flat array cannot be un-merged when a source is retired and
 * cannot be ordered by `PORTAL_SOURCES` once two sources' round trips interleave. Here the map is
 * read in table order and each source's array is already in arrival order, so the concatenation is
 * the order one `ingestHandler` call produced — by construction, not by sorting.
 */
function finishRun(run: RunRow): { assignments: Assignment[]; warnings: string[]; proposals: MapProposal[] } {
  const assignments: Assignment[] = [], warnings: string[] = [], proposals: MapProposal[] = [];
  for (const { name } of PORTAL_SOURCES) {                 // table order, never Object.keys order
    if (!(name in run.parsed.own)) continue;               // a source this run never offered
    // `finishSource` applies the `{name}: ` prefixing and the `0 assignments parsed` rule to what
    // is left after any retirement — which for a retired source is an empty item list and its one
    // failure line, exactly as the endpoint's own `continue` produced.
    assignments.push(...finishSource(name, run.parsed.assignments[name] ?? [],
                                     run.parsed.own[name] ?? [], warnings));
    proposals.push(...(run.parsed.proposals[name] ?? []));
  }
  return { assignments, warnings, proposals };
}
```

  and the tests, in `handler_test.ts`:

```ts
Deno.test("the relay never re-enters the public endpoint", async () => {
  // It calls the three extracted functions, not `ingestHandler` — so there is no synthetic Request,
  // no entitle closure to get wrong, and no second door into a public handler.
  const src = await Deno.readTextFile(new URL("./handler.ts", import.meta.url));
  assert(!src.includes("ingestHandler"), "the relay must not call the endpoint's own handler");
  for (const f of ["ingestZybook", "ingestVhl", "finishSource"]) assert(src.includes(f), f);
});

Deno.test("done is byte-identical to what one ingest call produced, on a shelf that spans round trips", async () => {
  // TWELVE books, so the shelf crosses MAX_BATCH and VHL's dashboard lands in a later batch than
  // some of the books — which is the case C3 says the first draft could not represent at all.
  const out = await driveWholeRun(scriptedTwelveBookShelfAndVhl());
  assertEquals(Object.keys(out.done).sort(), ["assignments", "proposals", "warnings"]);
  const oneShot = await ingestAllAtOnce(sameBodies());     // the C2 path, same inputs, one call
  assertEquals(out.done, oneShot, "the incremental parse must produce the same object, in order");
  // And the ORDER is what the partition buys (review S1): zyBooks' twelve books first, in shelf
  // order, then VHL's — even though VHL's dashboard arrived in an earlier round trip than books
  // 9-12. A flat array appended to as arrivals landed would interleave them.
  const sources = out.done.assignments.map((a: Assignment) => a.uid.split(":")[0]);
  assertEquals(sources, [...sources].sort((x, y) =>
    PORTAL_SOURCES.findIndex((s) => s.name === x) - PORTAL_SOURCES.findIndex((s) => s.name === y)),
    "PORTAL_SOURCES order, then arrival order within a source");
});

Deno.test("the fixture still yields its 24 assignments through the incremental path", async () => {
  const out = await driveWholeRun(scriptedZybooksResults());
  assertEquals(out.done.assignments.length, 24);   // the frozen reference is the oracle behind it
});

Deno.test("the cursor holds no page body, and neither does parsed", async () => {
  for (const plan of ["coursework", "coursework-discover"]) {
    for (const cursor of await everyCursorOf(plan)) checkCursor(cursor);   // throws on a body
    for (const parsed of await everyParsedOf(plan)) checkParsed(parsed);   // rows only, never bytes
  }
});

Deno.test("the budgets the server enforces are the ones the device does", async () => {
  const rs = await Deno.readTextFile(new URL("../../../../engine/src/relay.rs", import.meta.url));
  for (const [name, value] of [["MAX_STEPS", 40], ["MAX_BATCH", 8], ["MAX_ROUND_TRIPS", 24]]) {
    assert(new RegExp(`${name}: usize = ${value};`).test(rs), `${name} disagrees with relay.rs`);
  }
});
```

- [ ] **Step 8: Apply hand-off H8** (`config.toml`'s `[functions.relay]`), then **Step 9 (controller,
  not the implementer): apply the migration to staging and deploy.**

```
supabase db push --include-all --workdir cloud
supabase functions deploy relay --use-api --workdir cloud
```

  Then read it back, **five** proofs, appended to the task report: `relay_runs` exists with RLS on
  and exactly one policy; `POST /relay` with no bearer answers **401** in our shape
  (`{"error":"…"}`), with a session but no subscription **402**, with `{"protocol":2}` **400**, and
  with a `run` nobody owns **404**. Nothing is inserted by any of those four.

  **The fifth is the sweep** (review **I13**), and it is the one the other four cannot catch: 401
  and 402 both return *before* the sweep runs, and `restDelete` throws on a non-2xx, so a wrong
  filter literal 500s every entitled call and only an entitled call would show it. Insert a row by
  hand with `expires_at` in the past, call `/relay` once with a valid session, and read the row
  **gone** — and the call itself answering 200 rather than 500:

```
insert into public.relay_runs (account_id, plan, plan_version, expires_at)
values ('<the staging test account>', 'coursework', 'zybooks@1+vhl@1', now() - interval '1 hour');
```

  **The sixth is the platform's body ceiling, and it is a measurement** (the author's concern 3,
  ruled a finding). Nothing offline can read Supabase's edge request-body limit, and
  `MAX_POST_BYTES` is 16 MiB. So: POST **one padded body at the cap** — a well-formed
  `{protocol:1, job:"coursework", run:"<a uuid>", results:[…]}` whose one result's `body` is
  padding — with a valid session, and record two things: **the status**, and **whether it reached
  the function at all** (a platform refusal is a gateway shape, not our `{"error": …}`; the function
  logs are how to tell). Write both into the task report and into `HANDOFF.md`.

  **If it is refused below the cap**, the fallback is already decided and is two constants:
  `MAX_BYTES` drops from 2 MiB to **1 MiB** (still about 19x the largest payload ever measured —
  zyBooks 54,874 bytes, VHL 7,495) and `MAX_POST_BYTES` follows it, because the pin ties them. That
  is a one-line change in `relay.rs`, a one-line change in `protocol.ts`, and no design change.

- [ ] **Step 10: Run both Deno suites and commit.**

Run: C5's own line, then C2's full flag set over `cloud/supabase/`, then
`deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations/migrations_test.ts`
— and confirm its two parse counts are **unchanged from the branch point** (review **M6**: read the
pair off `migrations_test.ts` at the fork commit and record **that** in the task report; do not
quote a literal here, because the numbers are C3′'s to move and a stale one invites an edit to a
file C5 does not own). A moved count is evidence C5 created a function or a view it did not intend;
stop and read the diff.

```bash
git add cloud/supabase/migrations/20260918000100_relay.sql cloud/supabase/migrations_relay_test.ts \
        cloud/supabase/functions/relay/{index.ts,protocol.ts,protocol_test.ts,handler.ts,handler_test.ts,db.ts}
git commit -F .git-commit-msg.txt   # "feat(cloud): POST /relay — the step protocol, the run row and the parse hand-off (C5 Task 5)"
```

---

### Task 6: The zyBooks plan, step for step from the code it replaces

**Read first:** spec §6's zyBooks table, and `engine/src/zybooks.rs` lines 34, 42, 525, 537, 556, 574,
603, 629, 651 and its whole `mod tests` from `:1008` down — because **every network-layer test in
that module ports here, one for one**, and this is the task where that record is preserved before
Task 10 deletes the file.

**The composite driver, first** (ruling **R-C5-plan-3**, from review **C5**). A `relay_runs` row
holds one `plan`, the handler selects by **job**, and a coursework run drives **two** sources with
interleaved steps and a shared batch budget — so `PLANS` cannot be keyed by source, and the first
draft's `{ zybooks: zybooksPlan, vhl: vhlPlan }` made `PLANS[job]` undefined for both jobs. What
this task builds, in `plans/mod.ts`, before the zyBooks module itself:

- **`PLANS` is keyed by job**: `{ "coursework": courseworkPlan, "coursework-discover": discoverPlan }`.
- **`courseworkPlan` fans out** to the per-source modules (`SOURCE_PLANS = { zybooks, vhl }`), which
  keep the `Plan` shape and know nothing about each other.
- **`cursor.sources = { zybooks: {…}, vhl: {…} }`**, one sub-cursor per source, plus `timezone` and
  each source's redacted `config` taken from the first call's `client` block.
- **A batch is filled round-robin** across the sources that still have steps, up to `MAX_BATCH` — so
  a twelve-book shelf never starves VHL's three sequential steps, and VHL never starves the shelf.
- **A per-source `{kind: "failed"}` is a warning that retires that source**, not the run: it is
  appended to `parsed.own[source]`, that source's sub-cursor is marked done, and the other source
  keeps going. The run ends when every source is done or retired. (Task 9 step 4 requires exactly
  this for discover — *"a source that fails contributes an `errors` entry and never stops the
  other"* — and it is the same rule for coursework, because `collect` has always had it:
  *one source's failure never stops another* (`coursework.rs:1383`).)
- **`plan_version` is a stable composite** of the per-source module versions — `zybooks@1+vhl@1`,
  sources in `PORTAL_SOURCES` order — so a deploy that changes either module mid-run changes the
  string, the 409 fires, and the device starts a new run next slot (which is M4's case, and is why
  the column is `text`).

Its own tests, in `plans/mod_test.ts`:

```ts
Deno.test("PLANS is keyed by job, and every job the device can send resolves", () => {
  for (const job of ["coursework", "coursework-discover"]) assert(PLANS[job], job);
  assertEquals(PLANS["zybooks"], undefined, "keyed by JOB, not by source (review C5)");
});

Deno.test("one run drives both sources, round-robin, within one batch budget", () => {
  const r = courseworkPlan.start(ctxBothSources());
  assertEquals(r.steps.map((s) => s.id), ["zybooks.signin", "vhl.home"]);
  const shelf = courseworkPlan.next(ctxAfter({ zybooks: "items", vhl: "home" }), twelveBooksAndAForm());
  assert(shelf.steps.length <= 8, "MAX_BATCH is shared, not per source");
  assert(shelf.steps.some((s) => s.source === "vhl"), "a full shelf must not starve VHL");
});

Deno.test("a source that fails on its first step retires and the other finishes", () => {
  const r = courseworkPlan.next(ctxBothSources(), [vhlLoginRejectedTwice(), zybooksItemsOk()]);
  assert(r.steps.every((s) => s.source === "zybooks"), "VHL retired, zyBooks continues");
  assert(r.parsed.own.vhl.some((w: string) => w.includes("session invalid")));
});

Deno.test("a source that fails on a LATER round trip loses the rows it contributed on earlier ones", async () => {
  // Review S1, and R7's own example — which the test above cannot reach, because VHL fails on its
  // very first step, before it has contributed a single row. This is the clause that needed the
  // partition: a flat `assignments` array could not say which six rows were zyBooks'.
  //
  // Round trip 1: zyBooks signs in and lists twelve books; VHL reaches its login form.
  // Round trip 2: books 1-6 come back parsed, VHL logs in.
  // Round trip 3: book 7 answers `success: false` TWICE (so it is `failed`, not `reauth`), VHL's
  //               dashboard comes back.
  const out = await driveWholeRun(zybooksSixBooksThenDeadOnSeven_andVhlFinishes());

  // Every zyBooks row is gone — all six, not just book 7's — and VHL's are all there.
  assert(out.done.assignments.length > 0, "VHL contributed");
  assert(out.done.assignments.every((a: Assignment) => a.uid.startsWith("vhl:")),
    "six books' rows do not survive book 7's dead session: " +
    JSON.stringify(out.done.assignments.map((a: Assignment) => a.uid)));

  // zyBooks reports exactly one line, and it is the failure — not six books' worth of per-item
  // notes, and not a `0 assignments parsed` line on top of it.
  const zy = out.done.warnings.filter((w: string) => w.startsWith("zybooks: "));
  assertEquals(zy.length, 1, JSON.stringify(zy));
  assert(zy[0].includes("session invalid"), zy[0]);
  assert(!zy.some((w: string) => w.includes("0 assignments parsed")));

  // ...and the proposals it pushed before it died survive, which is what `ingestHandler`'s own
  // catch does and is the half of the rule that is not obvious.
  assert(out.done.proposals.some((p: MapProposal) => p.source === "zybooks"),
    "a retired source keeps the proposals it had already pushed");
});

Deno.test("parsed is partitioned by source, and only finishRun flattens it", async () => {
  // The structural half of S1: if `appendParsed` ever writes a flat array again, the retirement
  // above becomes untestable and this fails first, where the cause is visible.
  for (const row of await everyRunRowOf("coursework")) {
    assert(!Array.isArray(row.parsed.assignments), "a map keyed by source, never a flat array");
    assert(!Array.isArray(row.parsed.proposals));
    for (const key of Object.keys(row.parsed.assignments)) {
      assert(PORTAL_SOURCES.some((s) => s.name === key), `parsed is keyed by source: ${key}`);
    }
  }
  const src = await Deno.readTextFile(new URL("../handler.ts", import.meta.url));
  assertEquals([...src.matchAll(/\.assignments\.push\(/g)].length, 1,
    "exactly one flattener, and it is `finishRun`");
});

Deno.test("plan_version is a stable composite and moves when a module moves", () => {
  assertEquals(courseworkPlan.version(), `zybooks@${zybooksPlan.version}+vhl@${vhlPlan.version}`);
});
```

- [ ] **Step 1: Write the failing plan tests** — `cloud/supabase/functions/relay/plans/zybooks_test.ts`.
  A plan is pure over its cursor and the step results, so every test is: give it a cursor and a
  scripted result, assert the exact request it composes — method, URL, every header, the body, the
  placeholders — or the exact failure it reports.

```ts
import { assert, assertEquals, assertThrows } from "@std/assert";
import { zybooksPlan } from "./zybooks.ts";

const CFG = { courses: { UACS100Fall2026: { course: "cs-100", label: "CS 100" } }, ignore: ["HowToUseZyBooks2"] };
const ctx = (cursor = {}) => ({ cursor, config: CFG, hasCredential: true, session: { cookies: 0, captures: [] } });

Deno.test("step 1 is the signin POST, with the four headers zybooks 403s without", () => {
  const { steps } = zybooksPlan.start(ctx());
  assertEquals(steps.length, 1);
  const s = steps[0];
  assertEquals(s.id, "zybooks.signin");
  assertEquals(s.method, "POST");
  assertEquals(s.url, "https://zyserver.zybooks.com/v1/signin");          // zybooks.rs:34 SIGNIN_URL
  assertEquals(s.headers, [
    ["Content-Type", "application/json"],
    ["Accept", "application/json, text/javascript, */*; q=0.01"],
    ["Origin", "https://learn.zybooks.com"],
    ["Referer", "https://learn.zybooks.com/"],
    ["User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu"],   // zybooks.rs:42 USER_AGENT
  ]);
  assertEquals(s.body, { json: { email: "{{credential:zybooks:username}}",
                                 password: "{{credential:zybooks:password}}" } });
  assertEquals(s.capture, [{ name: "zybooks_token", from: "json", pointer: "/session/auth_token",
                             redact: true, persist: true, ttl_s: 43200 }]);
});

Deno.test("the user agent is an honest identifying string", () => {
  // Ported from zybooks.rs:1175. Without a User-Agent zyBooks returns a hard 403 on EVERY request,
  // signin included — a total outage, not a degradation, and the most likely cause of a future
  // sudden failure. No offline test can observe a header on the wire, so it is asserted here.
  const ua = zybooksPlan.start(ctx()).steps[0].headers.find(([k]) => k === "User-Agent")![1];
  assert(ua.includes("Knowlu") && !ua.includes("Chrome/"), ua);
});

Deno.test("signin returns the token and user id, and step 2 carries the token in BOTH query and header", () => {
  // Ported from `signin_returns_the_token_and_user_id` (zybooks.rs:1205) and `fetch_zybook_codes`
  // (`:603`). The token in both places is deliberate: the query-param form is reportedly
  // deprecated, and sending both survives the transition in either direction at no cost.
  const r = zybooksPlan.next(ctx({ stage: "signin" }), [signinResult(7, "TOK")]);
  const s = r.steps[0];
  assertEquals(s.id, "zybooks.items");
  assertEquals(s.url, "https://zyserver.zybooks.com/v1/user/7/items?items=%5B%22zybooks%22%5D&auth_token={{capture:zybooks_token}}");
  assertEquals(s.headers[0], ["Authorization", "Bearer {{capture:zybooks_token}}"]);
});
```

```ts
Deno.test("a dead session on the item list is not an empty shelf", () => {
  // Ported from zybooks.rs:1230. A 200 with `success: false` is a revoked token, and the rule this
  // whole module exists for is that it must never read as "0 assignments parsed".
  const r = zybooksPlan.next(ctx({ stage: "items" }), [jsonResult({ success: false })]);
  assertEquals(r.kind, "reauth");   // compose the login steps and retry this step ONCE, not twice
  const again = zybooksPlan.next(ctx({ stage: "items", reauthed: true }), [jsonResult({ success: false })]);
  assertEquals(again.kind, "failed");
  assert(again.warning.includes("session invalid"), again.warning);
});

Deno.test("a dead session on an assignment fetch names the book", () => {
  // Ported from zybooks.rs:1243, and the reason `requireSuccess` is called BEFORE routing in
  // `ingestHandler` (handler.ts:86): a session can die between the item list and one book's fetch.
  const r = zybooksPlan.next(ctx({ stage: "books", codes: ["UACS100Fall2026"] }), [jsonResult({ success: false })]);
  assert(r.warning.includes("UACS100Fall2026"), r.warning);
});

Deno.test("a bom prefixed signin body is decoded", () => {
  // Ported from `get_json_decodes_a_bom_prefixed_response` (zybooks.rs:1129). The device used to
  // strip the BOM (`decode_json`, zybooks.rs:519); since C5 the body travels byte for byte and the
  // plan's own decoder strips it — hand-off H7 corrects `parse_zybooks_test.ts`'s comment to match.
  const r = zybooksPlan.next(ctx({ stage: "signin" }), [rawResult("﻿" + JSON.stringify(signinBody(7, "TOK")))]);
  assertEquals(r.steps[0].id, "zybooks.items");
});

Deno.test("routing decides which books step 3 asks for, and an ignored book is silent", () => {
  // `routeZybook` already lives in ingest-coursework/parse_zybooks.ts and is imported, NOT
  // re-implemented: two interpretations of "is this book mapped" is exactly the drift `route_zybook`
  // was extracted to prevent (coursework.rs:454).
  const r = zybooksPlan.next(ctx({ stage: "items" }),
    [jsonResult({ success: true, items: { zybooks: [
      { zybook_code: "UACS100Fall2026" }, { zybook_code: "HowToUseZyBooks2" }, { zybook_code: "MATH125" }] } })]);
  // mapped and unmapped are both fetched — an unmapped book's payload is what `ingestHandler` turns
  // into a mapping PROPOSAL (R-OB-1), and skipping it here would put the card back out of reach.
  assertEquals(r.steps.map((s: Step) => s.id), ["zybooks.book:UACS100Fall2026", "zybooks.book:MATH125"]);
  assertEquals(r.steps[0].url,
    "https://zyserver.zybooks.com/v1/zybook/UACS100Fall2026/assignments?auth_token={{capture:zybooks_token}}");
});

Deno.test("a full shelf is batched, never one round trip per book", () => {
  const codes = Array.from({ length: 12 }, (_, i) => `BOOK${i}`);
  const r = zybooksPlan.next(ctx({ stage: "items" }), [jsonResult(shelfOf(codes))]);
  assertEquals(r.steps.length, 8, "MAX_BATCH — the rest come on the next round trip");
});

Deno.test("the plan carries a version, and it is a term of the run row's composite", () => {
  assertEquals(typeof zybooksPlan.version, "number");
  assert(zybooksPlan.version >= 1);
  assert(courseworkPlan.version().includes(`zybooks@${zybooksPlan.version}`), "ruling R-C5-plan-3");
});

Deno.test("a book's payload is parsed the moment it arrives and never held as bytes", () => {
  // Ruling R-C5-plan-2. The rows come back for `appendParsed`; the cursor keeps codes and an index.
  const r = zybooksPlan.next(ctx({ stage: "books", codes: ["UACS100Fall2026"], index: 0 }),
                             [jsonResult(fixturePayload())]);
  assert(r.parsed!.items.length > 0, "parsed rows, not a payload");
  checkCursor(r.cursor);
  assert(!JSON.stringify(r.cursor).includes("assignments"), "no page and no payload in the cursor");
});
```

  The remaining ports from `zybooks.rs::tests`, each one line in the same file and each named after
  the test it replaces: `signin_rejects_a_failed_login_without_naming_the_password` (`:1191` — the
  plan's failure text names neither the email nor the password),
  `fetch_zybook_codes_reads_the_nested_item_list` (`:1219`),
  `get_json_appends_with_an_ampersand_when_the_url_already_has_a_query` (`:1182` — the item-list URL
  already has `?items=`, so the token joins with `&`), and
  `payload_without_a_success_key_is_still_parsed` (`:935`, which is `requireSuccess`'s and already
  lives in `parse_zybooks_test.ts` — named here so the audit of what moved where is complete).

- [ ] **Step 2: Run them and watch them fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/relay/plans/zybooks_test.ts`
Expected: FAIL — `Module not found "./zybooks.ts"`.

- [ ] **Step 3: Write `plans/zybooks.ts` and `plans/mod.ts`.**

```ts
/**
 * The zyBooks fetch plan — the sequence that used to live in `engine/src/zybooks.rs`.
 *
 * Pure over its cursor and the step results: no fetch of its own, no clock beyond the request's, no
 * database read. Everything it knows about the vendor is in this file, which is the point — a
 * markup change or a URL change is a deploy of this file and nothing else.
 *
 * Three facts this plan encodes, carried over verbatim from the module it replaces:
 *  1. **A `User-Agent` on every request.** zyBooks 403s a request without one, signin included, so
 *     the symptom is a total outage, not a degradation.
 *  2. **The token goes in BOTH the query string and the Authorization header.** The query-param form
 *     is reportedly deprecated; sending both survives the transition in either direction at no cost.
 *  3. **A 200 with `success: false` is a dead session, not an empty semester.** It is answered with
 *     one re-authentication and, if that fails too, a named source failure — never an empty parse.
 */
import { requireSuccess, routeZybook } from "../../ingest-coursework/parse_zybooks.ts";
import type { Plan, PlanCtx, PlanStep } from "./mod.ts";

const BASE = "https://zyserver.zybooks.com/v1";
const SIGNIN = `${BASE}/signin`;
const UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu";

/** `zybooks_headers` (zybooks.rs:525), in the order that module built them. */
const HEADERS: Array<[string, string]> = [
  ["Accept", "application/json, text/javascript, */*; q=0.01"],
  ["Origin", "https://learn.zybooks.com"],
  ["Referer", "https://learn.zybooks.com/"],
  ["User-Agent", UA],
];

export const zybooksPlan: Plan = { name: "zybooks", version: 1, start, next };
```

  `start` returns the signin step. `next` switches on this source's own `cursor.sources.zybooks.stage`:
  `signin` → read `/session/auth_token` presence from `captured` and `/user/user_id` from the body,
  then the items step; `items` → `requireSuccess`, then one `zybooks.book:<code>` step per routed
  code, **batched at `ctx.budget`** — what the driver's round-robin left of `MAX_BATCH`, handed in
  rather than guessed at, and the module advances its cursor by exactly the number it returns
  (review **R6**); `books` →
  **parse each arriving payload immediately** with `ingestZybook` and return its rows for
  `appendParsed` (ruling **R-C5-plan-2**), keeping only the codes and the index in the cursor —
  `checkCursor` enforces that and `checkParsed` enforces that the rows are rows — and when the last
  code is in, mark this source done.

  **`plans/mod.ts`** is the `Plan` interface, the `PlanResult` union, the composite `courseworkPlan`
  and `discoverPlan`, and the two registries:

```ts
export interface Plan {
  name: string;
  /** A number on a source module; a composite STRING on a job plan (`zybooks@1+vhl@1`). */
  version: number | (() => string);
  start(ctx: PlanCtx): PlanResult;
  next(ctx: PlanCtx, results: StepResult[]): PlanResult;
}

export type PlanResult =
  | { kind: "steps"; steps: PlanStep[]; parsed?: Parsed }
  /** Review I12: `reauth` NAMES ITS SOURCE, and the handler answers it by writing `reauthed: true`
   *  into that source's sub-cursor and calling `plan.start(ctx)` again — the module composes its
   *  own login steps, because `handler.ts` names no source and could not. The module's `next` then
   *  re-issues the step that failed, and answers `failed` rather than `reauth` the second time:
   *  once, never twice (spec §4). */
  | { kind: "reauth"; source: string }
  | { kind: "failed"; source: string; warning: string }
  | { kind: "done"; parsed?: Parsed };

/**
 * What a plan is handed. **Declared, not assumed** (review **R6**: `PlanCtx`, `PlanStep`, `Parsed`
 * and `RunRow` were consumed in four places and defined nowhere — M5's shape, fixed in Rust and
 * reintroduced in TypeScript).
 */
export interface PlanCtx {
  /** This source's sub-cursor — `cursor.sources[name]` — never the whole cursor. */
  cursor: Record<string, unknown>;
  /** The redacted config the device sent for this source on the run's first call. */
  config: Record<string, unknown>;
  /** The resolved IANA name, from the `client` block. */
  timezone: string;
  hasCredential: boolean;
  session: { cookies: { count: number; session_cookies: number; earliest_expiry: number | null };
             captures: Array<{ name: string; expires_at: number }> };
  /**
   * **How many steps this module may return right now** — what the driver's round-robin has left of
   * `MAX_BATCH` (review **R6**). Without it a module returns what it likes and the driver trims,
   * which silently drops a book the module has already advanced its cursor past: a missing
   * assignment no warning reports. With it the module returns at most this many and advances
   * exactly that far, and `a_full_shelf_is_batched_never_one_round_trip_per_book` asserts both.
   */
  budget: number;
}

/** One request, before substitution. The device's `Step` minus `id`'s source prefix. */
export interface PlanStep {
  id: string; source: string; method: "GET" | "POST"; url: string;
  headers: Array<[string, string]>;
  body?: { json: unknown } | { form: Array<[string, string]> } | { text: string } | null;
  capture?: Array<{ name: string; from: "json"; pointer: string; redact: boolean; persist: boolean; ttl_s: number }>;
  follow_redirects?: boolean; max_bytes?: number;
}

/**
 * What one arrival parsed into, and what `appendParsed` merges. Rows only — never bytes.
 *
 * **It names its source** (review **S1**). R7's hardest clause is that a source which throws on
 * round trip N loses the items it contributed on round trips 1..N-1, and nothing can remove rows it
 * cannot identify. `own` was partitioned by source in round 2 and `items` was not, which left that
 * clause with a rule and no mechanism.
 */
export interface Parsed {
  /** The source this arrival belongs to — `parseArrival`'s own argument, carried through. */
  source: string;
  items: Assignment[];
  own: string[];
  proposals: MapProposal[];
  /** Set by `sourceFailure` (review R7): this source is retired and this is its one warning. */
  failed?: string;
}

/** The `relay_runs` row as the handler holds it. */
export interface RunRow {
  id: string; account_id: string; plan: string; plan_version: string;
  cursor: { timezone: string; sources: Record<string, Record<string, unknown>> };
  /**
   * **Partitioned by source, all three of them** (review **S1**). `assignments` was one flat,
   * unsourced array while `own` was already a map, so a retirement could not find the failing
   * source's rows to drop and `finishRun` could not order by `PORTAL_SOURCES` then arrival once two
   * sources' round trips interleave. Within a source the array is in **arrival order**, which is
   * the order the shelf came back — push, never splice.
   */
  parsed: {
    assignments: Record<string, Assignment[]>;
    own: Record<string, string[]>;
    proposals: Record<string, MapProposal[]>;
  };
  seq: number; steps_used: number; round_trips: number; bytes_used: number;
}

/** Keyed by JOB (ruling R-C5-plan-3). `PLANS[job]`, never `PLANS[source]`. */
export const PLANS: Record<string, Plan> = {
  "coursework": courseworkPlan,
  "coursework-discover": discoverPlan,
};
/**
 * What the two job plans fan out to. Adding a portal is a new file plus one line HERE.
 *
 * A source module carries one method the job plans do not: **`parse(ctx, result)`**, which turns
 * one arriving `StepResult` into `Parsed`. It is what keeps `handler.ts` from naming a source
 * (review **R6**) — `parseArrival` calls `SOURCE_PLANS[source].parse`, and a third portal is a file
 * and a line here rather than an edit to the driver.
 */
export const SOURCE_PLANS: Record<string, SourcePlan> = { zybooks: zybooksPlan, vhl: vhlPlan };
export interface SourcePlan extends Plan { parse(ctx: PlanCtx, result: StepResult): Parsed; }
```

- [ ] **Step 4: Apply hand-off H7** (`parse_zybooks_test.ts`'s BOM comment), then run and commit.

```bash
git add cloud/supabase/functions/relay/plans/{mod.ts,mod_test.ts,zybooks.ts,zybooks_test.ts} \
        cloud/supabase/functions/relay/handler.ts cloud/supabase/functions/ingest-coursework/handler.ts
git commit -F .git-commit-msg.txt   # "feat(cloud): the zyBooks fetch plan, step for step from the module it replaces (C5 Task 6)"
```

---

### Task 7: The VHL plan — CAS, the one-time `lt` ticket, and the host change

**Read first:** spec §6's VHL table; `engine/src/vhl.rs`'s module doc (lines 1-27, the three
live-rollout facts) and `:404` `parse_user_session_form`, `:468` `first_dashboard_link`, `:319`
`discover_sections`, `:518` `quote_plus`, `:534` `urlencode`, `:549` `login_and_fetch_dashboard`, and
its whole `mod tests` from `:1013` down, including the synthetic `login_page()` (`:662`) and
`landing_page()` (`:677`) builders — **which port with the tests, so no new fixture capture is
needed.**

- [ ] **Step 1: Write the failing plan tests** — `cloud/supabase/functions/relay/plans/vhl_test.ts`.

```ts
Deno.test("step 1 is a plain GET of the login host, with the user agent", () => {
  const { steps } = vhlPlan.start(ctx());
  assertEquals(steps[0].id, "vhl.home");
  assertEquals(steps[0].method, "GET");
  assertEquals(steps[0].url, "https://www.vhlcentral.com/");     // vhl.rs:549, `{base}/`
  assertEquals(steps[0].headers, [["User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu"]]);
});

Deno.test("login post body carries lt and service from the parsed form", () => {
  // Ported from vhl.rs:1042. **This is the test the whole module exists for.** Omitting `lt` makes
  // the POST fail SILENTLY — HTTP 200, the login page again — so the form scrape is generic: EVERY
  // named input and button, in order, not a fixed list, so a future hidden field keeps working.
  const r = vhlPlan.next(ctx({ stage: "home" }), [htmlResult(loginPage({ lt: LOGIN_TICKET, service: "" }))]);
  const s = r.steps[0];
  assertEquals(s.id, "vhl.login");
  assertEquals(s.method, "POST");
  assertEquals(s.url, "https://www.vhlcentral.com/user_session");
  assertEquals(s.headers, [["Content-Type", "application/x-www-form-urlencoded"],
                           ["User-Agent", UA]]);
  const form = s.body.form as Array<[string, string]>;
  assertEquals(form.map(([k]) => k), ["authenticity_token", "lt", "service", "commit",
                                      "user_session[username]", "user_session[password]"]);
  assertEquals(form.find(([k]) => k === "lt")![1], LOGIN_TICKET);
  assertEquals(form.find(([k]) => k === "user_session[username]")![1], "{{credential:vhl:username}}");
  assertEquals(form.find(([k]) => k === "user_session[password]")![1], "{{credential:vhl:password}}");
});

Deno.test("every named field in the form is scraped, not a fixed list", () => {
  // Ported from vhl.rs:1200. A hidden field VHL adds tomorrow keeps working with no deploy.
  const r = vhlPlan.next(ctx({ stage: "home" }), [htmlResult(loginPageWith({ surprise: "value" }))]);
  assert((r.steps[0].body.form as Array<[string, string]>).some(([k, v]) => k === "surprise" && v === "value"));
});

Deno.test("no user_session form, and a form with no authenticity_token, each raise not logged in", () => {
  // Ported from vhl.rs:1143 and :1154, and they are DIFFERENT messages on purpose: a missing form is
  // a page-shape change, a missing token is a half-rendered page, and the fixes differ.
  assertEquals(vhlPlan.next(ctx({ stage: "home" }), [htmlResult("<html></html>")]).kind, "failed");
  const a = vhlPlan.next(ctx({ stage: "home" }), [htmlResult("<html></html>")]).warning;
  const b = vhlPlan.next(ctx({ stage: "home" }), [htmlResult(loginPageWithoutToken())]).warning;
  assert(a !== b, `${a} / ${b}`);
});

Deno.test("login rejected serves the login page again and raises not logged in", () => {
  // Ported from vhl.rs:1077. HTTP 200 with the login form back is the silent failure.
  const r = vhlPlan.next(ctx({ stage: "login" }), [htmlResult(loginPage({ lt: LOGIN_TICKET }))]);
  assertEquals(r.kind, "reauth");                    // once, and only once
  const again = vhlPlan.next(ctx({ stage: "login", reauthed: true }), [htmlResult(loginPage({}))]);
  assertEquals(again.kind, "failed");
  assert(again.warning.includes("session invalid"));
});

Deno.test("the dashboard link comes out of the vendor's payload and is never constructed", () => {
  // Ported from vhl.rs:1013 and :468. The dashboard is on ANOTHER HOST at a per-enrollment URL that
  // is not known ahead of time — and that is exactly where `m3a.vhlcentral.com` earns its row in the
  // allow-list, because this URL is the vendor's and is checked like any other.
  const r = vhlPlan.next(ctx({ stage: "login" }), [htmlResult(landingPage(openEnrollment(DASHBOARD_LINK)))]);
  assertEquals(r.steps[0].id, "vhl.dashboard");
  assertEquals(r.steps[0].url, DASHBOARD_LINK);
  assert(DASHBOARD_LINK.startsWith("https://m3a.vhlcentral.com/"));
});

Deno.test("a payload with no open enrollment, and one with no dashboard link, report distinctly", () => {
  // Ported from vhl.rs:1093, :1106 and :1120 — three failure modes, three messages, because each is
  // a different problem for the student (dead session vs no active enrollment vs a page change).
  const none = vhlPlan.next(ctx({ stage: "login" }), [htmlResult(landingPage([]))]).warning;
  const noLink = vhlPlan.next(ctx({ stage: "login" }), [htmlResult(landingPage(enrollmentWithoutLink()))]).warning;
  assert(none !== noLink, `${none} / ${noLink}`);
});

Deno.test("the dashboard html is handed to the parser unchanged and the plan is done", () => {
  const r = vhlPlan.next(ctx({ stage: "dashboard" }), [htmlResult(dashboardFixtureHtml())]);
  assertEquals(r.kind, "done");
  assertEquals(r.payload.name, "vhl");
  assertEquals(typeof r.payload.html, "string");
});
```

- [ ] **Step 2: Run them and watch them fail.** `Module not found "./vhl.ts"`.

- [ ] **Step 3: Write `plans/vhl.ts`.** Three steps, and the three helpers `vhl.rs` owned, ported:

  - `parseUserSessionForm(html)` — the `id="user_session"` form's every named input and button, in
    order, HTML entities decoded, an empty `name` skipped, an `authenticity_token` required. A direct
    port of `vhl.rs:404`, including its regex tolerance for single- **and** double-quoted attribute
    values and its button-with-no-`value` inner-text rule (`:1261`, `:1227`, `:1238`).
  - `firstDashboardLink(html)` — `data-schools-payload`, JSON-decoded, walked
    `schools[].programs[].enrollments.open[]`, first `dashboard_link` that is truthy. Three distinct
    failures (`vhl.rs:468`).
  - `discoverSections(html)` — `/courses/(\d+)/sections/(\d+)/`, first-seen order, deduplicated
    (`vhl.rs:319`). Used by Task 9, written here.

  **And one edit to a C2 file** (review **I8**), because the first draft named an export that does
  not exist. `parse_vhl.ts` exports `MOUNT_MARKER`, `parseDurationHours` and `parseDashboard` only,
  and does the section lookup **inline** at `:99-101` (`:81` is the `sections` extraction, not the
  lookup — review **R10**); there is no `sectionMapping` in any form, and the Rust original (`vhl::section_mapping`, `engine/src/vhl.rs:300`) is deleted by Task
  10. So this task adds it, in `parse_vhl.ts`, and has `parseDashboard` call it:

```ts
/**
 * The ONE predicate for "is this section mapped" — `parseDashboard` and `vhlRows` both call it,
 * rather than each keeping its own reading of "an empty mapping is falsy" to drift apart later.
 * The port of `vhl::section_mapping` (engine/src/vhl.rs:300), which said exactly this and which
 * C5 deletes; mirrors `routeZybook`'s role on the zyBooks side.
 */
export function sectionMapping(sections: Obj, sectionId: string): Obj | null {
  const inner = sections[sectionId];
  return inner !== null && typeof inner === "object" && !Array.isArray(inner) &&
    Object.keys(inner).length > 0 ? inner as Obj : null;
}
```

  `parseDashboard`'s two inline lookups become calls to it, so the behaviour is unchanged and the
  frozen `vhl-parsed-reference.json` proves it. Task 9's `vhlRows` imports the same function. **Do
  not let Task 9 discover this at the import line** — which is what would have happened.

  The plan's own doc comment carries `vhl.rs`'s three live-rollout facts verbatim, plus a fourth that
  is now the relay's: **steps 2 and 3 depend on one jar spanning both hosts**, which the device gives
  them because a step uses its source's jar always and `relay::agent_for_source` builds exactly one
  per source per run.

- [ ] **Step 4: Run, then commit.**

```bash
git add cloud/supabase/functions/relay/plans/{vhl.ts,vhl_test.ts,mod.ts} \
        cloud/supabase/functions/ingest-coursework/parse_vhl.ts
git commit -F .git-commit-msg.txt   # "feat(cloud): the VHL fetch plan — CAS, the lt ticket and the host change (C5 Task 7)"
```

---

### Task 8: `coursework` drives the relay, and a rejected login is one card

**Read first:** spec §2.1, §2.6 and §4's last two paragraphs; `engine/src/coursework.rs:745`
(`collect_cloud`), `:815` (`propose_map_cards`), `:952` (`asked_map_keys`), `:1437`
(`FAILURE_MARKERS`); `engine/src/info.rs:27` (`KINDS`), `:56` (`NewInfo`), `:69` (`open_info`),
`:108` (`list_info`), `:157` (`close_info`).

- [ ] **Step 1: Ask Quinn (Q2)**, in one message: *When a portal rejects the saved password — not a
  network problem, the vendor saying no — Knowlu opens one info card, "Your zyBooks password no
  longer works", and then **stops trying that source**. **It starts again the moment you save that
  portal's password in the app**: saving is what closes the card and un-pauses the source, and the
  card says so. The alternative is trying again every slot. I recommend the pause: it is the legal
  briefing's "stop on the first sign of a vendor block and never retry through a change of
  identity", and a wrong password retried twice a day is how an account gets locked out. Pause with
  that exit, or retry every slot?* Record the answer; the plan is built to pause.

  **Why the exit has to be named, and where it lives** (review **C2**, settled by ruling
  **R-C5-plan-1**). The first draft skipped a paused source before the relay ran and closed the card
  only "for each source that authenticated" — so a paused source could never authenticate, the card
  could never close, and the pause was permanent. `info::list_info` returns open items only
  (`engine/src/info.rs:108`), so nothing else re-opened the path either; the card body promised the
  opposite; and `a_later_run_that_authenticates_closes_the_card_and_unpauses` was unsatisfiable as
  written. The exit is the write the student actually performs: **`onboarding::store_credentials`**
  (`app/src/onboarding.rs:850-856`) already knows the vault and the source, so it closes the card
  through the engine's `write` with `console_ctx()` — the existing shape — on
  `close_key: login:<source>`. Step 5 below is that call.

- [ ] **Step 2: Write the failing tests** — appended to `engine/src/relay.rs`'s `mod tests` (review
  **C1**: they drive a loopback portal, so they live where the `loopback` row is visible).

```rust
#[test]
fn a_rejected_login_opens_exactly_one_card_and_pauses_that_source() {
    let vault = scratch_vault();                 // a temp copy of a fixture, never a real vault
    let out = collect_cloud_against(&vault, relay_that_reports("zybooks", "login rejected"));
    let cards: Vec<_> = knowlu_engine::info::list_info(&vault).into_iter()
        .filter(|i| text(i, "close_key").as_deref() == Some("login:zybooks")).collect();
    assert_eq!(cards.len(), 1);
    assert_eq!(text(&cards[0], "kind").as_deref(), Some("notice"));
    assert_eq!(text(&cards[0], "opened_by").as_deref(), Some("agent:knowlu.coursework"));
    assert!(text(&cards[0], "title").unwrap().contains("zyBooks password"));
    // A second run mints no second card — the same shape `propose_map_cards` gets from
    // `asked_map_keys`, and for the same reason: twice a day forever is how a deck becomes noise.
    let _ = collect_cloud_against(&vault, relay_that_reports("zybooks", "login rejected"));
    assert_eq!(open_cards_for(&vault, "login:zybooks"), 1);
    // And the source is paused: the next run does not even start a relay run for it.
    assert!(sources_offered_to_the_relay(&vault).iter().all(|s| s != "zybooks"));
}

#[test]
fn saving_the_password_closes_the_card_and_un_pauses_the_source() {
    // Ruling R-C5-plan-1, and the property the first draft could not have: the pause has an exit,
    // and the exit is the thing the card tells the student to do. The engine half is here; the app
    // half (`store_credentials` calling it) is `app/tests/onboarding.rs`'s case in step 5.
    let vault = scratch_vault();
    let _ = collect_cloud_against(&vault, relay_that_reports("zybooks", "login rejected"));
    assert_eq!(open_cards_for(&vault, "login:zybooks"), 1);
    assert!(sources_offered_to_the_relay(&vault).iter().all(|s| s != "zybooks"));

    // What the app does when the student saves a new password for that source.
    knowlu_engine::info::close_info(&vault, Some("login:zybooks"), None, "dashboard",
                                    &console_ctx(), None, None).expect("closed");

    assert_eq!(open_cards_for(&vault, "login:zybooks"), 0);
    assert!(sources_offered_to_the_relay(&vault).iter().any(|s| s == "zybooks"),
        "the source is live again on the next slot");
    let _ = collect_cloud_against(&vault, relay_that_succeeds());
    assert_eq!(open_cards_for(&vault, "login:zybooks"), 0);
}

#[test]
fn a_paused_source_never_stops_the_other_one() {
    // `collect`'s oldest rule (coursework.rs:1383) and ruling R-C5-plan-3's per-source retirement,
    // at the device end: VHL paused must not cost the student their zyBooks.
    let vault = scratch_vault();
    let _ = collect_cloud_against(&vault, relay_that_reports("vhl", "login rejected"));
    let offered = sources_offered_to_the_relay(&vault);
    assert!(offered.iter().any(|s| s == "zybooks") && offered.iter().all(|s| s != "vhl"));
}

#[test]
fn a_vault_with_no_account_says_so_and_ranks_the_day() {
    // After Task 10 there is no local fetch path at all, and the honest line says which thing is
    // missing rather than pretending the semester is empty.
    let vault = scratch_vault_without_cloud_yaml();
    let out = coursework_main(&vault);
    assert_eq!(out.exit, 0);
    assert!(out.lines.iter().any(|l| l.contains("coursework: no account on this vault")), "{:?}", out.lines);
    assert!(!vault.join("tasks").read_dir().unwrap().next().is_some() || nothing_changed(&vault));
}

#[test]
fn a_dry_run_reaches_the_relay_and_writes_nothing() {
    // Unchanged from C2: a dry run has never skipped the network half — `collect`'s local path did
    // not either — but it writes no note, no journal record and no card (R-C2-E18 fix 1).
    let vault = scratch_vault();
    let before = fingerprint(&vault);
    let _ = coursework_main_dry(&vault);
    assert_eq!(fingerprint(&vault), before);
}

#[test]
fn the_session_dir_is_used_when_given_and_nothing_is_written_when_not() {
    let vault = scratch_vault();
    let dir = temp_dir("relay-slot-sessions");
    let _ = coursework_main_with_session_dir(&vault, Some(&dir), relay_that_sets_a_cookie());
    assert!(dir.join("zybooks.bin").exists());
    let bare = temp_dir("relay-slot-none");
    let _ = coursework_main_with_session_dir(&vault, None, relay_that_sets_a_cookie());
    assert!(std::fs::read_dir(&bare).unwrap().next().is_none());
    // And never in the vault, under any circumstances (spec §4).
    assert!(!vault.join("state").join("sessions").exists());
    assert!(walk(&vault).all(|p| p.extension().map(|e| e != "bin").unwrap_or(true)));
}
```

- [ ] **Step 3: Run them and watch them fail.** `collect_cloud` does not take a relay; there is no
  `--session-dir`; `coursework::main` takes four arguments.

- [ ] **Step 4: Rewrite `collect_cloud`.** Its shape after the change, with the deletions Task 10
  finishes:

  1. Read the coursework block and `resolve_timezone` exactly as today.
  2. For each of `["zybooks", "vhl"]` that is `enabled`, build a `SourceContext`:
     `{name, hosts: relay::source_named(name).hosts, config: yaml_to_json_for_request(&redact(&cfg, name)),
       has_credential: <the target reads>, session: store.load(name, now).report(now)}` — and **skip
     a source that is paused** by an open `info` item with `close_key: login:<name>` (Q2, and the
     pause's exit is step 5's app-side call).
  3. `relay::run(client, Job::Coursework, &sources, &secrets, &store, &tz_name)`.
  4. On `done`, decode with **`decode_coursework_reply`** — the surviving half of `post_coursework`
     (review **I7**: the POST wrapper loses its only non-test caller here and is deleted in Task 10
     with `coursework_request` and `FetchedSource`; the decoder is renamed and kept). `done` *is*
     the `/ingest-coursework` reply, so `assignment_from_row`, the `warnings` clip through
     `judge::one_line`, and the `proposals` decode all stay exactly where they are.
  5. `propose_map_cards(vault, &result.proposals, today, ctx, dry_run, warnings)` — unchanged.
  6. For each source the run reported as login-rejected: `open_login_card`, which checks `list_info`
     for an open item with the same `close_key` first, exactly as `propose_map_cards` checks
     `asked_map_keys`. For each source that **authenticated**: `close_info(key = "login:<source>")` —
     which is the belt to step 5's braces, and covers the case where the student fixed the password
     outside the app.

  **Both `cfg` arms** (review **M11**). `collect_cloud` has a `#[cfg(not(windows))]` twin at
  `coursework.rs:872` whose signature must change with the `#[cfg(windows)]` one, or the non-Windows
  build breaks. It stays what it is — one warning, an empty list — and only its parameter list moves.

```rust
/// One card, never a retry loop (ruling 4; spec §4; Q2).
///
/// `kind: "notice"` — one of `info::KINDS` — `close_key: "login:<source>"`, and
/// `opened_by: "agent:knowlu.coursework"`, which is the actor `MAP_ACTOR` already uses and which
/// `provenance::is_agent` recognises by its `agent:` prefix.
fn open_login_card(vault: &Path, source: &str, ctx: &WriteContext, warnings: &mut Vec<String>) {
    let key = format!("login:{source}");
    if crate::info::list_info(vault).iter().any(|i| text(i, "close_key").as_deref() == Some(&key)) {
        return;
    }
    let pretty = match source { "zybooks" => "zyBooks", "vhl" => "VHL", other => other };
    let item = crate::info::NewInfo {
        title: &format!("Your {pretty} password no longer works"),
        kind: "notice",
        body: &format!(
            "{pretty} refused the login Knowlu has saved, so nothing from it reached your list \
             today.\n\nOpen **Settings → Logins** and save the password again. Knowlu will not try \
             that login again until you do — a wrong password retried twice a day is how an \
             account gets locked."),
        opened_by: "agent:knowlu.coursework",
        close_key: Some(&key),
        expires: None,
    };
    match crate::info::open_info(vault, &item, ctx, None, None) {
        Ok(_) => warnings.push(format!("{source}: session invalid (the login was rejected); a card is open")),
        Err(e) => warnings.push(format!("{source}: session invalid; the card could not be written ({e})")),
    }
}
```

  **`session invalid` is already a `FAILURE_MARKER`** (`coursework.rs:1437`), so this line sorts to
  the top of `state/runner-log.md` with no new marker — which is why the phrasing is not free.

- [ ] **Step 5: The pause's exit — the app closes the card when the password is saved** (ruling
  **R-C5-plan-1**). In `app/src/onboarding.rs`'s `store_credentials` (`:850-856`), after the
  credential is written and for **that source only**:

```rust
    // C5 (review C2, ruling R-C5-plan-1): a rejected login pauses that source and opens one info
    // card; saving a new password is what ends the pause, because the engine cannot see a
    // Credential Manager write and a paused source never runs again to discover it worked. This is
    // the ONE app-side write C5 adds, it goes through the engine's `write` like every other console
    // write (`console_ctx()`, `via: "dashboard"`), and closing a key with no open item is a no-op.
    let _ = knowlu_engine::info::close_info(
        vault, Some(&format!("login:{source}")), None, "dashboard", &console_ctx(), None, None);
```

  and one case in `app/tests/onboarding.rs`: a vault with an open `login:zybooks` card, a
  `store_credentials` for `zybooks`, the card closed and a `login:vhl` card left alone.

- [ ] **Step 5a: — and make that exit reachable** (ruling **R-C5-plan-4**, from review **R1**).

  **Why step 5 alone is not enough.** `onboarding::store_credentials` is registered **only in the
  wizard window's** handler list (`app/src/main.rs:107`), is absent from the console's
  (`:186`), and is called by exactly one thing: the wizard's panel-5 `storeCredentials()`
  (`app/static/console.js:1485-1497`), on a vault that **does not exist yet**. There is no
  post-onboarding path to it: Settings has ten rows and none is a login
  (`app/static/index.html:156-166`), *adopt* opens the profile directly and *Add* starts the wizard
  on a **new** vault. So the card's *"Open Settings and save the password again"* is false, and the
  student's only exit is closing the card by hand through `commands::close_info`
  (`app/src/commands.rs:246`) — which un-pauses with **the same wrong password**, which is the loop
  Q2 exists to prevent. C2's deadlock survived R-C5-plan-1 by one hop.

  **And it is a product gap on its own**, which is why option (a) was chosen over comparing the
  credential's `LastWritten` (which would hide the gap) or ruling that closing the card is the exit
  (which is the loop): **a student who changes their zyBooks password has no way to tell Knowlu**,
  card or no card. So:

  1. **A Logins row in Settings** — `app/static/index.html`, beside the ten that are there, and
     `app/static/console.js`'s settings render. One row per source the table carries
     (`relay::PORTAL_SOURCES`, so the page offers exactly what the build can authenticate): the
     source's name, whether a credential exists, and **Save password…**, which reveals a username
     and a password field and a Save button. No URL, no vendor link —
     `no_network_reference_in_the_shipped_page` still holds.
  2. **One console-window command**, `commands::save_portal_login(source, username, password)`, in
     `app/src/commands.rs` beside the seven that mutate notes. It writes the **same**
     `knowlu/<profile_id>/<source>` credential the wizard writes (`credentials::target_for`,
     `app/src/credentials.rs:17`) and, on success, closes `login:<source>` exactly as step 5 does —
     so **R-C5-plan-1 lives in two places**, the wizard's and the console's, and neither is the
     only one. It takes a password across the IPC and therefore never logs, never returns and never
     formats it; the reply is `{ok, error}` and nothing else.
  3. **The card body becomes true**: *"Open **Settings → Logins** and save the password again."*
  4. **The command is registered by hand-off H11** (below), in the **console** window's
     `generate_handler!` list — which is a new command, so Quinn's own count moves and the
     hand-off recounts **both** lists rather than quoting a number.
  5. **Exit-gate 15(d) runs from the console**, not the wizard: a scratch profile is made *by* the
     wizard, so a wizard-only exit would have passed 15(d) while leaving every real student stuck.

  Tests, in `app/tests/commands.rs` and `app/tests/static_assets.rs`: the command writes the target
  `credentials::target_for` computes and closes only that source's card; a failed write leaves the
  card open and returns `ok: false`; the settings panel carries one row per `PORTAL_SOURCES` entry
  and no more; and the card body's sentence and the panel's heading are the same words.
  The credential-store test takes the file-scoped `CREDMAN_LOCK` with a generated test id and a
  `Drop` guard — **the one place this stream touches the real store**, and it is named here and in
  *Global Constraints*.

- [ ] **Step 6: Apply hand-offs H4a and H2**, then re-run `cargo test --workspace`. `coursework::main`
  gains its fifth parameter; `slot_argv` gains `--session-dir`; `app/tests/scheduler.rs`'s five
  `slot_argv` call sites gain the argument and one new case asserts the flag is on the `coursework`
  step and on no other.

- [ ] **Step 7: Assert the two entitlement halves** in the task report rather than assuming them:
  `git grep -n "entitle::gate" engine/src/main.rs` shows `coursework` behind C3′'s gate, and
  `relay::tests`' `every_service_failure_shape_is_a_named_line_and_nothing_changed` covers the
  402 mid-run. **Neither is re-implemented here.**

- [ ] **Step 8: Run, then commit.**

```bash
git add engine/src/coursework.rs engine/src/relay.rs app/src/onboarding.rs app/tests/scheduler.rs app/tests/onboarding.rs
git commit -F .git-commit-msg.txt   # "feat(engine): coursework drives the relay; a rejected login is one card (C5 Task 8)"
```

---

### Task 9: `coursework-discover` over the relay — the wizard, before the vault exists

**Read first:** spec §6's *`coursework-discover` as a relayed job*; `engine/src/coursework.rs:1231`
(`zybooks_rows`), `:1259` (`vhl_rows`), `:1290` (`discover_json`); `app/src/onboarding.rs:62`
(`errors_from_discovery`), `:74` (`discovery_argv`), `:93` (`discover_coursework`);
`app/src/account.rs:18`, `:32`, `:43`, `:47`.

**The problem this task exists for.** The wizard runs **before the vault exists**, so there is no
`config/cloud.yaml` for `cloudmodel::resolve` to read — and after C5, discovery is a cloud job. The
three flags are what stand in for that file, and every one of them carries a public value or a target
*name*.

- [ ] **Step 1: Write the failing tests** — in `engine/src/relay.rs`'s `mod tests` (review **C1**)
  and `app/tests/onboarding.rs`.

```rust
// engine/src/relay.rs, in `mod tests`
#[test]
fn discover_is_one_json_object_on_stdout_with_the_shape_the_wizard_reads() {
    let out = discover_against(relay_that_returns_discover_rows());
    let v: serde_json::Value = serde_json::from_str(&out.stdout).expect("exactly one JSON object");
    assert_eq!(out.exit, 0);
    assert!(v["zybooks"].is_array() && v["vhl"].is_array() && v["errors"].is_array());
    assert_eq!(v["zybooks"][0]["code"], "UACS100Fall2026");
    assert_eq!(v["zybooks"][0]["mapped"], false);            // no vault, so `mapped` is always false
    assert_eq!(v["vhl"][0]["section"], "2102121");
}

#[test]
fn discover_with_no_cloud_and_no_vault_is_an_errors_entry_and_still_exit_zero() {
    let out = discover_with_no_flags();
    assert_eq!(out.exit, 0);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(v["zybooks"].as_array().unwrap().len(), 0);
    assert!(v["errors"][0].as_str().unwrap().contains("no account"));
    // The wizard's answer to "we could not look up your courses" is to let the student type the
    // mapping, not to stop (coursework.rs:1290's doc comment). That is unchanged.
}

#[test]
fn discover_writes_nothing_anywhere() {
    let vault = scratch_vault();
    let before = fingerprint(&vault);
    let _ = discover_with_vault(&vault, relay_that_returns_discover_rows());
    assert_eq!(fingerprint(&vault), before, "discovery never writes — not a note, not a run record");
}
```

```rust
// app/tests/onboarding.rs
#[test]
fn the_discovery_argv_carries_a_public_base_a_public_key_and_a_target_name_and_no_secret() {
    let args = knowlu::onboarding::discovery_argv("profile_deadbeef", true, true);
    assert!(args.contains(&"--cloud-base".to_string()));
    assert!(args.contains(&"--anon-key".to_string()));
    assert_eq!(args[args.iter().position(|a| a == "--session-target").unwrap() + 1],
               "knowlu/pending/session");
    // The constraint the function's own doc comment names, and the one that must survive C5: every
    // argument is a NAME or a public value. An anon key is compiled into the shipped app.
    for a in &args { assert!(!a.contains("Bearer ") && !a.starts_with("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIi")); }
}

#[test]
fn the_coursework_panel_carries_both_halves_of_its_copy() {
    // Review I10: the first draft assumed a ToS-tension sentence already existed and that H6 would
    // "append a clause" to it. There is none — the panel has a credential-STORAGE lede and nothing
    // else — so C5 writes the disclosure, it goes to Quinn and the lawyer with Q6, and this pins
    // both halves so neither can drift without the other.
    let html = include_str!("../static/index.html");
    assert!(html.contains("Stored in Windows Credential Manager on this machine"),
        "the existing lede, unchanged");
    assert!(html.contains("the request is made from your own PC, on your instruction"),
        "hand-off H6, once Quinn has approved the wording (Q6)");
}
```

- [ ] **Step 2: Run them and watch them fail.**

- [ ] **Step 3: Rewrite `discover_json`.** New signature and new body:

```rust
/// The three values a wizard has and a vault does not. All public or a name; **never a token**.
pub struct PreVaultCloud<'a> {
    pub api_base: Option<&'a str>,
    pub anon_key: Option<&'a str>,
    pub session_target: Option<&'a str>,
}

pub fn discover_json(
    vault: Option<&Path>,
    zybooks_target: Option<&str>,
    vhl_target: Option<&str>,
    cloud: PreVaultCloud<'_>,
) -> String
```

  The body keeps everything about **shaping** that is device-side today — the config load, its
  `config: <reason>` error entry, the credential-target resolution from the flags or the vault, and
  the "a missing target means that source is simply skipped, not an error" rule — and replaces the
  two credentialed closures with one `relay::run(&client, Job::Discover, …)`. The client is
  `cloudmodel::resolve(vault)` when there is a vault, or `CloudClient::new` from the three flags when
  there is not. **`mapped` is computed server-side** (`zybooks_rows`/`vhl_rows` move into
  `plans/`, spec §7) from the redacted config the `client` block already carries, or is `false` when
  there is no vault — which is exactly what `discover_json` does today, moved.

  **Still read-only, still exit 0, still one JSON object on stdout.** `rows_from_discovery` and
  `errors_from_discovery` in the app are not edited, and that is the test above.

  **Both `cfg` arms** (review **M11**). `discover_json` has a `#[cfg(windows)]` body at
  `coursework.rs:1290` and a `#[cfg(not(windows))]` twin at `:1366`, and both take three parameters
  today. **Both gain the fourth**, or the non-Windows build breaks — the twin keeps its one-line
  body (`"credential store unavailable on this platform"` in `errors`) and only its signature moves.
  Hand-off **H4b** supplies one dispatch arm and the crate must match it on every target.

- [ ] **Step 4: Write the discover plan's server half.** In `plans/mod.ts`, the `discover` job runs
  zyBooks steps 1-2 and VHL steps 1-3 from the **same two plan modules** — no third plan, no second
  interpretation of how to log in — and its `done` is `{zybooks, vhl, errors}` shaped by
  `zybooksRows`/`vhlRows`, ported from `coursework.rs:1231`/`:1259` into `plans/zybooks.ts` and
  `plans/vhl.ts` beside the flows they belong to. `routeZybook` and `sectionMapping` are the **one**
  predicate each caller shares, imported from `parse_zybooks.ts` and `parse_vhl.ts`, never
  re-implemented — the rule `vhl_rows`'s own doc comment already states
  (`coursework.rs:1255-1257`), and which **I8** makes true on the VHL side by exporting
  `sectionMapping` from `parse_vhl.ts` and having `parseDashboard` call it (Task 7).

  **`discoverPlan` is a job plan like `courseworkPlan`** (ruling **R-C5-plan-3**): the same composite
  driver, the same `cursor.sources`, the same round-robin batch, and the same rule that a per-source
  `failed` is a warning which retires that source and lets the other finish — which is the behaviour
  discovery has always needed and which the first draft named here without a mechanism to deliver
  it. `done` for this job is `{zybooks, vhl, errors}`, and a retired source contributes its sentence
  to `errors`.

  Its Deno test drives the whole job and asserts the rows, the `mapped` computation with a config and
  without one, and that a source that fails contributes an `errors` entry and never stops the other.

- [ ] **Step 5: Apply hand-offs H4b, H3 and H6**, then `cargo test --workspace` and
  `python scripts/wizard-check.py` → `ok` (hand-off H10, verified not edited).

- [ ] **Step 6: Run, then commit.**

```bash
git add engine/src/coursework.rs engine/src/relay.rs app/tests/onboarding.rs \
        cloud/supabase/functions/relay/plans/{mod.ts,zybooks.ts,vhl.ts,zybooks_test.ts,vhl_test.ts}
git commit -F .git-commit-msg.txt   # "feat: coursework-discover becomes a relayed cloud job (C5 Task 9)"
```

---

### Task 10: The on-device fetchers leave

**Read first:** spec §7 in full. **This task comes after Tasks 6 and 7 on purpose**: the network
tests in `zybooks.rs` and `vhl.rs` are the only executable record of how the two portals behave, and
they are ported before the files are deleted, not after.

- [ ] **Step 1: Write the failing test** — `engine/tests/dependency_boundary.rs`, one new case, and
  one amendment to `engine/tests/no_console.rs`'s comment (not its assertion).

```rust
/// The device knows no login flow, no URL and no parser (ruling 4).
///
/// The named exception ruling 4 itself carves out is the host table, which is why this scan allows
/// `relay.rs`'s patterns and nothing else. A `https://` literal anywhere under `engine/src/` outside
/// `PORTAL_SOURCES` is a URL the device decided on its own, and after C5 there is no such decision
/// to make.
#[test]
fn the_engine_holds_no_portal_url_and_no_login_flow() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(!src.join("zybooks.rs").exists(), "engine/src/zybooks.rs is C5's to delete");
    assert!(!src.join("vhl.rs").exists(), "engine/src/vhl.rs is C5's to delete");
    for entry in std::fs::read_dir(&src).unwrap().flatten() {
        let path = entry.path();
        if path.extension().map(|x| x != "rs").unwrap_or(true) { continue; }
        let text = std::fs::read_to_string(&path).unwrap();
        // The same rule as `the_password_is_exposed_in_exactly_one_place` (review **R5**), and for
        // the same reason: a bare `#[cfg(test)]` split would read only the head of `relay.rs` — the
        // one file in this crate that could legitimately hold a portal host — and this scan would
        // stop meaning anything without failing. `split_before_test_module` splits on the attribute
        // followed by `mod tests`, which is the module boundary and not any inner `cfg(test)` item.
        let code = split_before_test_module(&text);
        for needle in ["zybooks.com", "vhlcentral.com", "auth_token", "user_session", "authenticity_token",
                       "data-schools-payload", "js-student-dashboard-app"] {
            let allowed = path.ends_with("relay.rs") && (needle == "zybooks.com" || needle == "vhlcentral.com");
            assert!(allowed || !code.contains(needle),
                "{}: the device still knows `{needle}` — the flow, the URLs and the order are the \
                 cloud's (amendment 2026-09-17, ruling 4); only the host ALLOW-LIST stays",
                path.display());
        }
    }
}
```

- [ ] **Step 2: Run it and watch it fail.** `engine/src/zybooks.rs is C5's to delete`.

- [ ] **Step 3: Move the three parser-side helpers that stay in the crate.** Before deleting
  anything, confirm nothing still needs it:
  - `zybooks::parse_assignments`, `category_of`, `require_success` — **deleted**: the parser left for
    `parse_zybooks.ts` in C2 and nothing in the crate calls them after Task 8.
  - `vhl::parse_dashboard`, `section_mapping`, `discover_sections` — **deleted**, same reason; their
    server twins are `parse_vhl.ts` and Task 7's plan.
  - `coursework::parse_duration_hours` (`:103`) — **deleted** and its behaviour already lives in
    `parse_vhl.ts`'s `parseDurationHours`; `git grep -n parse_duration_hours` must come back empty
    outside the deleted files before the `git rm`.
  - `scrub`, `quote`, `quote_plus`, `json_escape_ascii` — already moved in Task 1.

- [ ] **Step 4: Delete, and apply hand-off H1b in the same commit.**

```bash
git rm engine/src/zybooks.rs engine/src/vhl.rs
```

  and from `engine/src/coursework.rs`: `fetch_zybooks` and `fetch_vhl` (both `cfg(windows)` arms and
  both `cfg(not(windows))` arms, `:464`-`:555`), `collect` (`:1383`), `pub type Fetcher` (`:426`),
  `BookRouting` and `route_zybook` (`:443`, `:454`), `zybooks_rows` and `vhl_rows` (`:1231`, `:1259`),
  `parse_duration_hours` (`:103`) and its `DURATION` regex (`:91`), and `main_with_fetchers`'s
  `fetchers` parameter — `main` calls the body directly, and the `(fetchers, resolve(vault).ok())`
  match at `:1540` collapses to `resolve(vault)` with a named skip on `Err`.

  **And the three review I7 named**, which lose their only non-test callers at Task 8 and would
  otherwise be `dead_code` warnings under a plain `cargo build --workspace` — the gate this stream
  keeps at zero, and one that `#[cfg(test)]` suppresses nothing for:
  - `coursework_request` (`:639`) — deleted. The relay composes nothing; the cloud does.
  - `FetchedSource` (`:630`) — deleted with it.
  - `post_coursework` (`:670`) — **split, not deleted**: the POST wrapper goes, the reply decoder
    stays as `decode_coursework_reply(reply, warnings) -> PostCourseworkResult`, which Task 8's
    `collect_cloud` already calls on `done`. `PostCourseworkResult` keeps its name and its fields.

  The tests at `coursework.rs:3262` and its neighbours move with the half they exercise: the ones
  that assert the **request** shape (`the_coursework_payload_carries_no_credential` and its
  loopback harness) are superseded by `relay.rs`'s own `the_relay_payload_carries_no_credential`
  and go; the ones that assert the **reply** decoding stay and call `decode_coursework_reply`
  directly, which needs no socket at all. Say in the task report which test went where, because a
  reader of the diff will see a loopback harness disappear and should not have to guess why.

  **`main_with_fetchers` keeps its name and its other four parameters.** It is the seam the run-log
  summary is tested through, and renaming it would churn tests that have nothing to do with C5; what
  goes is the one parameter that no longer has a meaning.

  **H1b** (`lib.rs`'s two declarations) lands in this same commit, because a `lib.rs` declaring a
  file that no longer exists and a file with no declaration are each a hard error. Say so in the
  message.

- [ ] **Step 5: The two floors that must not move, asserted rather than assumed.**
  - `engine/tests/no_console.rs`'s `with_spawns >= 3` was lowered to `>= 2` by C3′ when `history.rs`
    went. **Neither file C5 deletes spawns a child process** (`git grep -n "Command::new"
    engine/src/zybooks.rs engine/src/vhl.rs` is empty at HEAD), so the floor stays at 2 and the file
    is **not edited**. Record the grep's output in the task report. **If C3′ has not in fact lowered
    it** — it is `>= 3` on `main` today and this stream forks after C3′ — that is a finding about the
    branch point, not a licence to edit the file: report it and stop.
  - `engine/tests/dependency_boundary.rs`'s `ureq` assertion from Task 1 still passes: the `cookies`
    feature is still needed, for `relay.rs` now instead of `vhl.rs`, and Task 12's `CLAUDE.md` edit
    (H9) rewrites the sentence that says why.

- [ ] **Step 6: Rewrite `engine/Cargo.toml`'s `ureq` comment**, the one at `:38-42`:

```toml
# `cookies` is not a default feature and VHL does not work without it: CAS login is on
# www.vhlcentral.com and the dashboard is on m3a.vhlcentral.com, so the session cookie has to
# survive both the redirect chain and the host change. Session cookies are scoped to
# .vhlcentral.com, which is exactly why one jar spans both hosts. Since C5 the jar belongs to
# `src/relay.rs` — one agent per source per run — and `max_redirects(0)`, because the device
# follows the chain itself and checks every hop against `relay::PORTAL_SOURCES`. The `json`
# feature is deliberately NOT enabled: `CookieJar::save_json` writes only persistent cookies and a
# CAS session cookie has none, so the store replays the vendor's own `Set-Cookie` lines instead.
ureq = { version = "3.4.0", features = ["cookies"] }
```

- [ ] **Step 7: Run everything, then commit.**

Run: `cargo test --workspace` at 0 warnings — and record the pass count, which **falls** by roughly
the number of `#[test]`s in the two deleted files — **count them at the branch point rather than
quoting a literal** (review M1: the plan's first draft said 44 and 39 and the real numbers at
`141b12d` were 38 and 36) — and rises by whatever Tasks 1-9 added. A falling count
is expected here and nowhere else in this plan; the task report states both numbers and the net.
Then `git status --porcelain --untracked-files=all engine/tests/fixtures/` → **empty**.

```bash
git add engine/src/lib.rs engine/src/coursework.rs engine/Cargo.toml engine/tests/dependency_boundary.rs
git rm engine/src/zybooks.rs engine/src/vhl.rs
git commit -F .git-commit-msg.txt   # "refactor(engine): the on-device fetchers leave; the device knows no login flow (C5 Task 10, hand-off H1b)"
```

---

### Task 11: The published promise

**Read first:** spec §5's last bullet; `site/privacy.html:24` (the *Coursework logins* bullet under
*What stays on your machine*), `:38` (the `<dd>` of the *Your coursework logins* definition — the
`<dt>` above it is `:37` and does not change) and `:108` (*Security*); C3′ Task 11,
which moved **four different copies of a different sentence** and is not re-opened here.

**What is C3′'s and not C5's, stated first so nobody moves it twice.** The one-line promise at
`site/privacy.html:13`, its twin at `site/index.html:16`, `app/static/console.js`'s `var PRIVACY` and
`engine/tests/site.rs:15`'s `const PRIVACY` are about **note bodies** and were rewritten by C3′ under
its own P1. C5 touches none of them, and `the_wizards_privacy_sentence_is_the_sites_privacy_sentence`
must still pass unchanged.

- [ ] **Step 1 (Q6): draft the two sentences and ask Quinn.** The drafts, for Quinn and the lawyer:

  **`site/privacy.html:38`, the `<dd>` of *Your coursework logins*** — today:

  > **On your machine only, in Windows Credential Manager, never on our servers.** Knowlu signs in to
  > zyBooks and VHL from your PC, with your credentials, and reads the assignment list that comes
  > back; that list — never the password — may be read on our servers so that one parser can serve
  > everyone. The password stays on the machine, and there is no column in our database for one.

  becomes:

  > **On your machine only, in Windows Credential Manager, never on our servers.** Our servers decide
  > what to ask zyBooks and VHL for; **your PC is what asks them.** Your password is filled in on your
  > machine, at the last moment, and only ever into a request to that site itself — Knowlu carries a
  > list of which addresses each login may be sent to, built into the app, and refuses to send it
  > anywhere else, including to us. The page that comes back is read on our servers so that one
  > parser can serve everyone, and is thrown away as soon as it has been read. The password stays on
  > the machine, and there is no column in our database for one.

  **`site/privacy.html:24`, the *Coursework logins* bullet under *What stays on your machine*** —
  today it names the Credential Manager path; it gains its second half:

  > **Coursework logins** — Windows Credential Manager, under `knowlu/<profile>/<source>`. Never in
  > the vault, never in a backup, never in a log, and never on our servers. **The sign-in cookies
  > those logins produce stay here too**, sealed so that only this Windows account on this machine
  > can read them, so that Knowlu signs in to a site once rather than on every run.

  *Security* (`:108`) — *"Portal passwords are the clearest case of all: they are never sent to us, so
  there is no server anywhere holding one"* — **stands unchanged and is now stronger**, and the task
  report says so rather than editing a true sentence.

  Ask Quinn: *these two, before merge, and they go to the lawyer with the C1 packet's P5 list. The
  substantive change a lawyer needs to see: our servers now decide which request is made, and the
  claim we keep is narrower and more checkable than the old one — the password goes to the site it
  was given for or nowhere, enforced by a list compiled into the app.*

- [ ] **Step 2: Write the failing test** — in `engine/tests/site.rs`, beside its existing cases:

```rust
/// The published promise and the code agree about who composes a request.
#[test]
fn the_privacy_page_says_the_cloud_composes_and_the_device_asks() {
    let page = include_str!("../../site/privacy.html");
    assert!(page.contains("your PC is what asks them") || page.contains("your PC</strong> is what asks them"),
        "the coursework-logins definition still describes the pre-C5 mechanism");
    assert!(page.contains("built into the app"), "the allow-list is what makes the claim checkable");
    assert!(page.contains("thrown away as soon as it has been read"), "zero retention (Q1)");
    assert!(page.contains("sign-in cookies"), "the What-stays-on-your-machine bullet");
    // The sentence that does NOT move, and the one C3′ owns, both still there.
    assert!(page.contains("Portal passwords are the clearest case of all"));
    assert!(!page.contains("Knowlu signs in to zyBooks and VHL from your PC, with your credentials, and reads"));
}
```

- [ ] **Step 3: Run it, watch it fail, make the two edits, run it again.** The page's `<p class="date">`
  version line and `account::PRIVACY_VERSION` **move together or not at all** — that is
  `account.rs`'s own standing rule and `app/tests/static_assets.rs` pins it. C1b moved both to
  `2026-09-17`; if C5 merges on a later day, both move to that day in this commit, and the task
  report says which day and why. **Ownership:** that is one constant in `app/src/account.rs`, which C1
  owned and which retired with C1's merge; it is not in `HANDOFF.md` §2's shared-single-owner list, so
  it is C5's for that one line — named here and in the ownership scan at Task 12 so the diff is not
  read as a stray edit.

- [ ] **Step 4: Commit.**

```bash
git add site/privacy.html engine/tests/site.rs app/src/account.rs
git commit -F .git-commit-msg.txt   # "docs(site): the privacy page says who composes a request and who makes it (C5 Task 11, Q6)"
```

---

### Task 12: Close

- [ ] **Step 1: The whole gate, twice.** `cargo test --workspace` at 0 warnings — record the pass
  count and the exact `warnings: N accepted (.rsrc), N tallies, N other` line, which must end in
  `0 other`. Then `deno check`, `deno lint` and `deno test` **with CI's own line, verbatim**
  (`.github/workflows/ci.yml:84` — review **M7**: CI is the gate, so the gate's command is CI's and
  not a paraphrase of it; the plan's first draft dropped `--allow-write=cloud/eval` and the
  `cloud/eval/` path), recording each count. Then `git ls-files --eol cloud/ engine/ app/ site/ docs/` and
  confirm every `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` is `i/lf`, every `.ps1` is
  `i/crlf`, and every fixture is `attr/-text`. Then `.\scripts\ci\eol-check.ps1`. Then
  `python scripts/wizard-check.py` → `ok` (hand-off **H10**, verified) and
  `python scripts/settings-check.py`.

- [ ] **Step 2: The frozen references.**
  `git status --porcelain --untracked-files=all engine/tests/fixtures/` → **empty**, and
  `git diff --stat main...c5-relay -- engine/tests/fixtures/` → **empty**. The eight Python-written
  references and the three Rust-generated surface references are untouched; `oracle.rs` and
  `surface_oracle.rs` pass unchanged, and the reason is structural: none of the three fixture vaults
  carries a `config/cloud.yaml`, so no relay run can start on one. Confirm in the report that
  `zybooks-parsed-reference.json` and `vhl-parsed-reference.json` are still read by
  `parse_zybooks_test.ts` and `parse_vhl_test.ts` over the same relative path — they are C5's
  oracles as much as C2's, and a stream that deleted the last Rust reader of a reference is exactly
  the moment somebody decides it is unused.

- [ ] **Step 3: The account-scoping scan — C5's own, in C5's own file.** Added to
  `cloud/supabase/functions/relay/protocol_test.ts`:

```ts
Deno.test("every relay query is scoped to one account, nothing bypasses C1's helpers, and the sweep says why it is not", async () => {
  const src = await Deno.readTextFile(new URL("./db.ts", import.meta.url));
  for (const call of src.matchAll(/"relay_runs",\s*`([^`]*)`/g)) {
    const q = call[1];
    const isSweep = q.includes("expires_at=lt.");
    assert(q.includes("account_id=eq.") || isSweep, `a relay_runs query without account_id=eq.: ${q}`);
    if (isSweep) {
      // The ONE exception, and it is deliberate: the sweep deletes everybody's expired rows, which
      // is what makes the cron job unnecessary. It can delete nothing that is still live and it
      // returns nothing, so it reads no account's data. Named here so the exception is a decision.
      assert(src.includes("// sweep: deliberately not account-scoped"), "the sweep must say why");
    }
  }
  assertEquals([...src.matchAll(/rest\.fetch\(/g)].length, 0, "C1's helpers are the only path");
});

Deno.test("the device's own JSON goes through one writer", async () => {
  const rs = await Deno.readTextFile(new URL("../../../../engine/src/relay.rs", import.meta.url));
  assertEquals([...rs.matchAll(/serde_json::to_string/g)].length, 0,
    "every JSON this crate writes goes through `ledger::dumps_value` (CLAUDE.md)");
});
```

- [ ] **Step 4: The ownership check.** `git diff --name-only main...c5-relay` → confirm every path is
  inside C5's ownership as *Global Constraints* lists it, plus: the ten hand-off files as their own
  commits; `app/tests/{scheduler,onboarding,static_assets}.rs`, which this stream writes tests into
  for files the controller owns (named in *File structure*); and the one line each in
  `engine/src/cloudmodel.rs` (Task 1) and `app/src/account.rs` (Task 11). **An overlap is a stop, not
  a rebase**: report it to the controller.

- [ ] **Step 5: The hand-off list.** One section in the task report: **ten entries** — **H1a**
  (Task 1), **H8** (Task 5), **H7** (Task 6), **H4a** and **H2** (Task 8), **H4b**, **H3** and **H6**
  (Task 9), **H1b** (Task 10, compile-blocking with the deletions), **H5**, **H9** and **H10**
  (Task 12) — each with the exact code from *Controller hand-offs*, the task it landed beside, its
  commit sha, and what would have broken without it.

- [ ] **Step 6: The commands recount — by counting.** C5 adds **one** console command
  (`save_portal_login`, ruling R-C5-plan-4), so the counts move and `CLAUDE.md`'s sentence is
  rewritten from what H11 observed at the branch point plus one. **Do not carry a number in from
  anywhere**: C1b merges ahead of this stream and moves both lists. The old sentence said: **43** on the console window,
  **30 (+3)** on the picker/wizard window, **62 distinct**. Say that in the report and in `CLAUDE.md`
  only if C3′ moved them; otherwise leave the sentence exactly as it is. `CLAUDE.md`'s rule is
  *recount before quoting a number* — the recount here is `git diff main...c5-relay --
  app/src/main.rs` being empty.

- [ ] **Step 7: Apply hand-off H9** (`CLAUDE.md`'s four edits and `HANDOFF.md` §3/§4) and write
  **what production still needs**, in the `HANDOFF.md` block:

  - `20260918000100_relay.sql` pushed to `knowlu-prod` **with `--include-all`**, and
    `select count(*) from public.relay_runs` read back as 0;
  - `supabase functions deploy relay --use-api --project-ref jxthohvwrijwtuwlglan`, and the same four
    refusals smoked there that Task 5 step 9 smoked on staging;
  - **the privacy page republished** (Q6) — the one item that must not lag the release;
  - **the lawyer's read** of the two rewritten sentences, with the C1 packet's P5 list;
  - **`/ingest-coursework` stays public** and is retired only when no shipped client posts to it. A
    student on 0.1.x will keep posting to it for as long as they do not update; the endpoint costs
    nothing to leave up and a 404 to an old build is a silent empty semester;
  - **`relay_runs` is swept at the top of every call, not by cron.** If `/relay` is ever not called
    for a long period, expired rows simply sit there; they carry no page and no secret and the next
    call removes them. Worth a look at the first hundred accounts, not before;
  - **the plan versions are in the row**, so a deploy that changes a plan mid-run leaves a row whose
    `plan_version` no longer matches: the handler answers 409 and the device starts a new run next
    slot. That is by design and has never been exercised on a real deploy;
  - **no real portal has ever been reached through the relay from CI** and never will be. The only
    proof is the exit gate's supervised live run;
  - **the session store has never survived a Windows account change or a machine move**, because
    DPAPI is exactly what stops it. A student who reimages gets one extra login per portal;
  - **Q1-Q5's answers**, each with the date and the task it was asked at.

- [ ] **Step 8: Commit.**

```bash
git add HANDOFF.md CLAUDE.md docs/plans/2026-09-17-c5-relay-fetch-plan.md \
        cloud/supabase/functions/relay/protocol_test.ts
git commit -F .git-commit-msg.txt   # "docs: C5 closed — the cloud composes, the device asks, and the fetchers are gone (C5 Task 12)"
```

---

## Exit gate

1. **`POST /relay` is deployed on staging**, behind C1's `requireActiveEntitlement`, answering its
   designed refusals: 401 with no bearer, 402 with no subscription, 400 on `protocol: 2` / an unknown
   `job` / a malformed result batch, 404 on a run nobody owns, 409 on a replayed `results`, 405 on
   anything but POST, 413 past the 10 MiB body cap. Every endpoint path is the hyphenated function
   name — Supabase routes `/functions/v1/<function-name>`.
2. **The host allow-list is two rows in a real build.** `engine/tests/relay_allowlist.rs` links the
   library without `cfg(test)` and asserts the table is exactly `zybooks: *.zybooks.com` and
   `vhl: www.vhlcentral.com, m3a.vhlcentral.com`, that `source_named("loopback")` is `None`, and that
   every one of spec §3's rejections refuses: not-https, a port that is not 443, userinfo, an IP
   literal (v4 and v6), a non-ASCII host, punycode, `zybooks.com.evil.example`, `evilzybooks.com`,
   and the other source's host on this credential. **A refusal names the host and never the URL.**
3. **A credential reaches exactly one function and exactly one destination.**
   `the_password_is_exposed_in_exactly_one_place` pins `.expose()` at one call site in `relay.rs`;
   `the_relay_payload_carries_no_credential` scans every byte the device POSTs across a whole run for
   a password, a username, a captured token, a cookie, a credential-target shape and an app-data
   path, and finds none; and `check_host` runs **before** `fill`, so a refused host never touches a
   password.
4. **The cloud never sees a session cookie, a captured token or a credential echoed back at us.**
   Every `Set-Cookie` is stripped from the reply (C5-D6) and recorded on the device instead; every
   `redact: true` capture is replaced **at its JSON pointer and re-serialised** — not by a text
   replace, which misses an escaped token and corrupts a body on a short one (review **I5**) — and
   the reply is then run through `scrub` against the request's own secrets, because a vendor that
   serves the login page again commonly echoes the submitted username back in a `value=` attribute
   (review **I3**); while the fields the plan must read stay readable
   (C5-D5). Both are tested against a body that carries `success`, `user_id` and the token together.
5. **The device follows redirects itself and checks every hop.** A 302 to a host off the list is
   `host_not_allowed` naming that host; a `Set-Cookie` set on a 302 is recorded, which is the case
   ureq's own follower would lose and the case CAS actually produces; six hops is a refusal, not a
   loop.
6. **Every budget ends a run as a named warning, never a 500 and never a non-zero exit.** 40 steps,
   8 per batch, 24 round trips, 5 hops, 2 MiB per response (a refusal, not a truncation), 8 MiB per
   run, 60 s per **whole step including its redirect chain**, 10 minutes per run, and 16 MiB per
   outgoing POST — the last **derived** from `MAX_RUN_BYTES` and base64's 4/3 and matched by the
   server's `readJson` cap, so a 413 is unreachable rather than unlikely (review **I1**).
   **And the ten minutes is enforced, not compared** (review **C4**): the deadline is monotonic and
   is checked before every step, every redirect hop and every round trip, each request's timeout is
   `budget.remaining()`, and `a_stalled_portal_ends_the_run_at_the_deadline_and_never_at_the_scheduler_s`
   proves a portal that accepts and never answers ends the run in under ten minutes of simulated
   time — not six requests of sixty seconds per step, eight steps to a batch. **Every budget warning
   carries an existing `FAILURE_MARKER`** and is asserted to sort first through `rank_warnings`
   (review **I14**), because only one warning reaches `state/runner-log.md`.
7. **`knowlu-engine coursework` still always exits 0** on every one of: no `config/cloud.yaml`, no
   session, no entitlement, no network, 401, 402, 409, 413, 429, 5xx, timeout, a refused host, a blown
   budget — each as a distinct named line, with `session invalid` / `fetch failed` / `the service is
   unavailable` still sorting to the top of `state/runner-log.md` through the unchanged
   `FAILURE_MARKERS`. **A run that does not reach `done` writes nothing** — no note, no journal
   record, no partial ingest.
8. **The parsers, their oracles and the reply shape are untouched.** `done` for a coursework job is
   `{assignments, warnings, proposals}` and is **byte-identical to what one `ingestHandler` call
   produced from the same bodies** — asserted on a **twelve-book shelf that spans round trips**, the
   case ruling **R-C5-plan-2** exists for and the one the first draft could not represent at all;
   `ingestHandler`'s own suite passes unchanged after the three-function extraction;
   `assignment_from_row`, `decode_coursework_reply`, `propose_map_cards` and `asked_map_keys` are
   unchanged; the 15-a-day proposal cap still applies; `zybooks-parsed-reference.json` and
   `vhl-parsed-reference.json` are byte-identical to `main`. **A shelf is not capped**, and a
   thirteenth book is not a product limit.
9. **Every network-layer test in `zybooks.rs` and `vhl.rs` has a successor**, named in Tasks 6 and 7,
   including `signin_returns_the_token_and_user_id`,
   `a_dead_session_on_the_item_list_is_not_an_empty_shelf`,
   `login_post_body_carries_lt_and_service_from_the_parsed_form`,
   `login_rejected_serves_login_page_again_raises_not_logged_in` and
   `payload_with_no_open_enrollment_raises_not_logged_in`. The task report lists the old name, the new
   name and the file for every one, and **any test with no successor is named and argued** rather
   than dropped quietly.
10. **The device knows no login flow, no URL and no parser.**
    `the_engine_holds_no_portal_url_and_no_login_flow` scans every non-test file under `engine/src/`
    for `zybooks.com`, `vhlcentral.com`, `auth_token`, `user_session`, `authenticity_token`,
    `data-schools-payload` and `js-student-dashboard-app`, and allows only the two host patterns in
    `relay.rs`. `engine/src/zybooks.rs` and `engine/src/vhl.rs` do not exist.
10a. **One run drives both sources, and the run row says which job it is.** `PLANS` is keyed by job;
    `courseworkPlan` and `discoverPlan` fan out to the two source modules; `cursor.sources` holds a
    sub-cursor each; a batch is filled round-robin inside one `MAX_BATCH`; a per-source failure is a
    warning that retires that source and lets the other finish; and `plan_version` is a stable
    composite so a mid-run deploy still fires the 409 (ruling **R-C5-plan-3**, review **C5**).
    **`reauth` names its source** and is answered by the handler re-entering `plan.start` with
    `reauthed: true` in that source's sub-cursor, so the module composes its own login steps — once,
    never twice (review **I12**).
10b. **Raw pages live for one parse and nowhere else.** Each body is parsed as it arrives and only
    the rows are kept, in `relay_runs.parsed`, deleted with the row; `checkCursor` refuses a
    raw-page key in the cursor and `checkParsed` refuses bytes in `parsed` (ruling **R-C5-plan-2**,
    review **C3**). **`parsed` is partitioned by source** — `assignments`, `own` and `proposals` are
    each a map keyed by a `PORTAL_SOURCES` name, appended to in arrival order and flattened only by
    `finishRun` — which is what lets a source that fails on round trip N lose the rows it
    contributed on 1..N-1, the way a single `ingestHandler` call did (review **S1**). `a source that
    fails on a LATER round trip loses the rows it contributed on earlier ones` is the test, and it
    is a different test from the one that fails a source on its first step. The sweep is `expires_at=lt.now` — **`now`, not `now()`** — and a fifth staging
    proof inserts an expired row, calls `/relay` once and reads it gone, because `restDelete` throws
    on a non-2xx and the sweep runs before any run logic (review **I13**).
11. **The session store is sealed, device-local, and never in the vault.** The file is DPAPI-sealed at
    current-user scope with `CRYPTPROTECT_UI_FORBIDDEN`; no cookie value or captured token is readable
    in it; a corrupt or foreign file reads as an empty session and never as a failure; a TTL past its
    expiry yields nothing; with no `--session-dir` nothing is written at all; and no `.bin` exists
    anywhere under a vault.
12. **The relay adds no crate and no second HTTP client.** `engine/Cargo.toml` names no `url`, no
    `cookie_store`, no `reqwest`, and no `base64` — `relay.rs` carries its own RFC 4648 encoder with
    RFC 4648 §10's vectors as its test; the `windows` dependency gains one feature and nothing else;
    `Cargo.lock` shows no new package. C3′'s crypto refusals still pass unrelaxed.
13. **The published promise is true.** `site/privacy.html` says our servers compose each request and
    the student's PC makes it, that the password is filled in on the machine and only ever into a
    request to that site, that the list of permitted addresses is built into the app, that the page is
    thrown away as soon as it is read, and that the sign-in cookies stay on the machine. Quinn and the
    lawyer have read the two sentences (Q6). The page's date and `account::PRIVACY_VERSION` agree, and
    C3′'s four note-body copies are untouched.
14. `cargo test --workspace` is green at **0 warnings** with the gate line ending `0 other`;
    `oracle.rs` and `surface_oracle.rs` pass **unchanged**; `deno check`, `deno lint` and `deno test`
    over `cloud/supabase/` are green with C2's full flag set; `migrations_test.ts`'s two corpus pins
    are **unmoved**; `.\scripts\ci\eol-check.ps1` passes; `python scripts/wizard-check.py` prints
    `ok`; no test opens a socket that is not `127.0.0.1`; and every hand-off **H1a, H1b, H2, H3, H4a,
    H4b, H5, H6, H7, H8, H9, H10** is listed in the task report with its commit.

### 15. The controller's live proof, on staging, with Quinn at the machine

**This is the item the rest of the gate cannot replace**, and it is the only place a real portal is
reached. Run by the **controller**, never by an implementer subagent, from a **scratch profile**
pointed at staging (`KNOWLU_API_BASE`, `KNOWLU_ANON_KEY`) with a **scratch vault**
(`.\scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-full`), never Quinn's own vault and
never a live profile. Quinn saves their own zyBooks and VHL passwords into that scratch profile's
Credential Manager entries themselves; this session never sees one.

- (a) **One supervised slot.** `knowlu-engine coursework --vault <scratch> --session-dir <scratch
  app data>\sessions --via local-runner`. Expected: exit 0; assignments created from **both** portals;
  a `relay_runs` row that appeared and is gone; `select count(*) from public.relay_runs` back to its
  starting value.
- (b) **The wire, read.** Supabase's function logs for `relay` across that run: step ids
  (`zybooks.signin`, `zybooks.items`, `zybooks.book:*`, `vhl.home`, `vhl.login`, `vhl.dashboard`),
  statuses, byte counts and milliseconds — and **no page body, no cookie, no URL carrying a token, no
  credential, and no error message from a third party**. A log line that carries any of those is a
  stop, not a note.
- (c) **The spec's own open question, answered here, and this item is BLOCKING for Task 12** — not a
  note (review's *planner decision 1*): **how long does each portal's session actually last?** Q4's
  `ttl_s: 43200` and the privacy draft's "signs in to a site once rather than on every run" both
  rest on it, and a published promise that outruns the measurement is the one thing on this list a
  later commit cannot fix.

  **Do it after I11's report shape is in, not before.** The measurement only means anything if the
  cloud's skip decision is expiry-driven; with a bare cookie *count* the plan would compose the
  login steps every time and both runs would measure the re-auth path and tell you nothing.

  Run a second slot **immediately** and confirm from the logs that the plan **skipped its login
  steps** on the strength of the session report — and, for VHL, that the report said
  `session_cookies > 0` with `earliest_expiry: null` and the plan tried anyway and was right. Then
  run a third the **next morning**, and record for each portal whether it skipped or
  re-authenticated. Write it into `HANDOFF.md` whatever it turns out to be — including "zyBooks
  re-authenticated every time", which would be a finding and not a failure, and would send Q4 back
  to Quinn with a real number.
- (d) **A wrong password, once — and the repair goes through the CONSOLE, not the wizard** (ruling
  **R-C5-plan-4**). This ordering is the point of the item, not a detail of it: a scratch profile is
  made **by the wizard**, so a wizard-only exit would pass this gate while every real student stayed
  stuck — which is exactly what review **R1** found.

  With Quinn's consent and one of their own accounts: save a deliberately wrong password, run one
  slot, and confirm exit 0, **one** info card titled *Your zyBooks password no longer works* whose
  body says *Open **Settings → Logins** and save the password again*, the source paused, and **no
  second attempt**. Then, **without re-running the wizard and without closing the card by hand**,
  open the console's **Settings → Logins**, save the right password there, and confirm the card
  closes on that save; run one more slot and confirm the source is live again and the assignments
  arrive. Do this on **one** portal only, once, and stop at the first sign of a vendor block (the
  legal briefing's rule).
- (e) **Nothing left behind.** The scratch vault, the scratch profile, its session files, its
  Credential Manager entries and the staging rows are removed afterwards, and the report says so.

---

## What is NOT in this plan

- **Retiring `/ingest-coursework` as a public endpoint.** The relay calls its handler in process and
  the endpoint stays up for clients that still post to it. Retiring it is a decision about shipped
  builds, not about this code (spec §11), and it is in Task 12's production block.
- **A third portal.** Nothing here names one, and §6 says exactly what adding one costs: a deploy for
  the flow, the parser and the oracle; a **release** for the host row and the login panel. The plan
  registry is one line per source so that the cost is where the argument says it is.
- **Reading grades from the signed-in LMS session.** Still §13's, still wanted, still unscheduled.
- **C4's removal of the local llama.cpp runtime.** `engine/src/runtime.rs` and `app/src/inference.rs`
  are untouched and unextended; `SUPPORTED_RUNTIMES` is read here only as the pattern `PORTAL_SOURCES`
  copies.
- **Anything about the account vault.** `engine/src/sync.rs`, `engine/src/entitle.rs`,
  `engine/src/backup.rs` and the two sync endpoints are C3′'s. C5 inherits the entitlement gate and
  re-decides nothing about it.
- **A diagnostic window for raw pages.** Q1, recommended as zero. If Quinn answers otherwise it is a
  table, an RLS policy, a purge entry, a privacy sentence and a §9 inventory row — a day's work and a
  policy change, and it does not belong hidden inside a task.
- **A general-purpose `knowlu-engine relay` subcommand.** C5-D7, argued in spec §7: it would be a
  *make this request with my saved password* tool sitting on the student's machine and in their
  process list, and the slot's step vocabulary is a contract with `slot_argv`, the run records and
  the tray.
- **Capturing anything but a JSON pointer.** `Capture.from` is a closed set of one. A regex or
  XPath extractor would be a general-purpose scraper the server could point at any part of a
  response; what a plan needs from a page it reads server-side from the body it was handed.
- **Resuming a run.** A retry is a new run with a new id. The vault write is idempotent
  (`sync_coursework` keys on `source_uid`) and map cards are guarded by `asked_map_keys`, so nothing
  is lost by starting over — and a resume would be a second correctness property to keep.
- **Real-time or out-of-slot fetching.** A run lives inside one slot, so nothing fetches while the
  laptop is closed. That is ruling 4's sentence and it is a product promise, not an implementation
  detail.
- **Any change to the ranking, the journal, `write`, the approvals cap or the run-record shape.**
- **Production deployment.** Staging only, by the controller. Production is Task 12 step 7's list and
  the pre-pilot parity checklist `HANDOFF.md` §4 already carries.

---

## Deferred minors

| # | Minor | Why it is deferred |
|---|---|---|
| M1 | The device sends the whole `client` block — hosts, redacted config, session report — on **every** run's first call, even when nothing has changed | It is about 1 KB and it is what makes the run stateless on the device side. A cached-config hash would be a second thing to keep true for a saving that is smaller than one HTTP header block. |
| M2 | A `results` batch can carry up to 8 × 2 MiB, so the 10 MiB body cap is not much headroom | The real numbers are 54,874 bytes for the largest zyBooks payload and 7,495 for the VHL dashboard, so a realistic batch is under 500 KB. The cap is a sanity bound; if a vendor ever returns a megabyte a page, `MAX_BATCH` is the lever, not the cap. |
| M3 | `relay_runs` is swept opportunistically, so a project with no traffic keeps expired rows | They hold no page and no secret, the row is tiny, and a cron job is a thing that can silently stop. Named in Task 12's production block. |
| M4 | A plan deployed mid-run leaves a row whose `plan_version` no longer matches, and the device starts over next slot | Correct and cheap: a run writes nothing until `done`. Worth watching after the first deploy that changes a plan while students are in a slot window. |
| M5 | The session store is per profile and per machine, so a student who reimages logs in again | That is what DPAPI at current-user scope buys, and it is the right trade. Syncing a cookie jar through the account would put a session cookie server-side, which C5-D6 and the legal note's *Avoid* list both refuse. |
| M6 | `perform` re-parses the body as JSON once per `Capture` | One capture exists today. If a plan ever wants four, parse once and pointer four times; it is three lines and a measurement nobody has needed yet. |
| M7 | A vendor that answers 200 with a login page on a step the plan does not recognise as a login page reads as a parse failure | This is exactly the failure `require_success` and `parse_user_session_form` were written for, and both ports carry their recognisers. A third portal will need its own, and §6 says the plan is where it goes. |
| M8 | The commit trailer hard-codes a model name | **Ruled not a defect** (R-C2-6): the trailers name the model and session that *execute* the plan; the literal in Global Constraints is the writing session's. |

---

## Fix round 1 — resolutions (2026-09-17)

Against `docs/reports/2026-09-17-c5-relay-fetch-plan-review.md` (`5a2ca69`, 600 lines, verdict
*execute after fix round 1*). **Every finding is applied**; three asked for a decision and were
settled by the controller as **R-C5-plan-1/2/3**, recorded first because they are what changed the
shape of the plan rather than its wording. The spec was amended in the same round (its Status line,
§2.4, §6, §8 and two §12 rows).

### The three rulings

**R-C5-plan-1 — the pause has an exit, and the app is what opens it (from C2).** A rejected login
pauses that source; **saving that portal's password closes the card and un-pauses it**.
`onboarding::store_credentials` (`app/src/onboarding.rs:850-856`) calls `info::close_info` on
`close_key: login:<source>` through the engine's `write` with `console_ctx()` — the existing shape.
Task 8 step 5 is that call and its test; `app/src/onboarding.rs` enters the ownership list for that
one call; Q2's text to Quinn now says what re-arms the source, because "pause" with no answer to that
is "stop forever". *Cost if wrong: one call.*

**R-C5-plan-2 — parse incrementally; parsed rows only between round trips; no shelf cap (from C3).**
The cloud parses each raw body **as it arrives**, per source, and keeps only the parsed rows — in
`relay_runs.parsed`, deleted with the row. Raw bodies are never stored, which is Q1 and R4-19 exactly.
`done` is assembled from those rows at the end. **Option (b), "one batch or bust", is refused: a
thirteenth book must not be a product limit**, and the spec's §6 now says so. The `ingestHandler`
hand-off is restructured: three functions — `ingestZybook`, `ingestVhl`, `finishSource` — are
**extracted** from `ingest-coursework/handler.ts` and `ingestHandler` is rewritten in terms of them,
so there is one copy and not two; `warnings` and `proposals` stay byte-identical because the code is
moved rather than re-derived, the endpoint's own suite is unchanged, the two frozen parsed references
are the oracles, and `finishRun` concatenates in `PORTAL_SOURCES` order (zyBooks, then VHL) and, within
a source, by arrival index — which is the order one `ingestHandler` call produced. A twelve-book shelf
spanning round trips is a test, and so is equality with the one-shot path.

**R-C5-plan-3 — the composite driver (from C5).** `PLANS` is keyed by **job**;
`courseworkPlan` and `discoverPlan` fan out to `SOURCE_PLANS = {zybooks, vhl}`;
`cursor.sources = {zybooks: {…}, vhl: {…}}`; a batch is filled **round-robin** across the sources that
still have steps, inside one shared `MAX_BATCH`; a per-source `{kind:"failed"}` is a warning that
**retires that source and lets the other finish** (which is `collect`'s oldest rule, `coursework.rs:1383`);
and `plan_version` is a stable composite string (`zybooks@1+vhl@1`), so a mid-run deploy still fires
the 409. Spec §6 states it; Tasks 5 and 6 implement it.

### Critical

| # | Finding | Resolution |
|---|---|---|
| **C1** | The loopback portal row is invisible to the file that needs it — `relay_contract.rs` is an integration test and links the lib without `cfg(test)`, so every portal test would fail `unknown_source` | **`engine/tests/relay_contract.rs` does not exist.** Every test that aims at a loopback portal moved into `engine/src/relay.rs`'s own `#[cfg(test)] mod tests`, which is what spec §9 said and what `coursework.rs:3095-3160` precedes. `engine/tests/relay_allowlist.rs` is unchanged and is the one integration test this stream adds. `Captures::get_for_test` is gone with the file — an in-module test reads the private accessor, and a `pub` door on a struct holding bearer tokens should not exist. Global Constraints, the ownership list, *File structure*, Tasks 4, 8 and 9 and every command line all follow. |
| **C2** | The rejected-login pause has no exit; Task 8's own test was unsatisfiable | **R-C5-plan-1.** Task 8 step 5 adds the app-side close; the test is rewritten as `saving_the_password_closes_the_card_and_un_pauses_the_source` and a second case proves a paused source never stops the other; exit-gate 15(d) is now demonstrable. |
| **C3** | Accumulated page payloads had nowhere to live between round trips | **R-C5-plan-2.** The migration gains a `parsed` column with its own guard (`checkParsed`), `checkCursor`'s ban on raw-page keys stays, and *Interfaces* contract 8 carries the three extracted functions. |
| **C4** | The ten-minute budget was checked only between round trips and `STEP_TIMEOUT` bounded one HTTP request out of six; the test compared two constants | `RunBudget` gains a monotonic `deadline`, `remaining()` is each request's timeout, `STEP_TIMEOUT` bounds a **whole step including its chain**, the agent carries no `timeout_global`, and two behavioural tests (a stalled portal, a stalled chain) replace the comparison — which is kept beside them as a pin on the numbers and renamed so it does not claim more than it proves. Exit-gate item 6 restated. |
| **C5** | One run, one `plan` column, two sources: `PLANS[job]` was undefined for both jobs | **R-C5-plan-3.** Task 6 builds the composite driver before the zyBooks module, with `plans/mod_test.ts` proving the key, the round-robin, the per-source retirement and the composite version. |

### Important

| # | Finding | Resolution |
|---|---|---|
| **I1** | The 10 MiB `readJson` cap was below what the device's own budgets let it send | Derived, not chosen: `MAX_POST_BYTES = MAX_RUN_BYTES * 4/3 + 1 MiB` = **16 MiB**, the same number on both sides; the device charges it outgoing so a 413 is unreachable rather than unlikely; `a_results_batch_over_the_cap_is_a_413_not_a_500` pins the **relationship**; and both *Interfaces* contract 7 and Task 5 step 6 now say that `readJson` reads the whole body before it measures, in UTF-16 code units — a sanity bound, not a memory guard. **This settles the plan's own open decision 2.** |
| **I2** | `scrub` knew three encodings and the relay adds a fourth (`quote_plus`) | `quote_plus(secret)` joins `scrub`'s form list as it moves in Task 1, the test is renamed and gains a fourth case with a space-bearing secret, the module doc says "four", and Task 4 adds a form-body transport failure whose error text carries `hun+ter`. A pre-existing gap in `vhl.rs`; C5 is where it becomes a stated guarantee. |
| **I3** | The body the device returns was never scrubbed of the request's own secrets — and VHL's rejected login serves the login page back, commonly echoing the username | `scrub(&body, &filled.secrets)` is the last shaping step, after the capture redaction and before the UTF-8 decision, with a test against a portal that echoes the posted form. Spec §2.4's "byte for byte" is amended to say when it is not. **Taken in full as a security finding.** |
| **I4** | The URL that was checked was not the URL that was sent | `perform` step 3a: `check_host` again on `filled.url`, asserting the two agree, with two cases — a placeholder in the authority is refused, and a substituted value cannot change the host. |
| **I5** | Capture redaction was a text replace, so it could miss an escaped token or corrupt a body | Redact **at the pointer** in the parsed `serde_json::Value` and re-serialise through `dumps_value`, **only when a `redact: true` capture actually fired** — so the non-redacting case keeps its bytes and the BOM test stays true. One test with a token carrying a character the serialiser escapes. **Taken in full as a security finding.** |
| **I6** | `the_password_is_exposed_in_exactly_one_place` counted its own source and could never pass | Counted over the pre-`cfg(test)` half, the shape `the_engine_holds_no_portal_url_and_no_login_flow` already uses, and the message now says the number is a guarantee and not a tally. |
| **I7** | Three symbols listed under "Stays" lose their only non-test callers, so 0 warnings goes red | `coursework_request` and `FetchedSource` are deleted in Task 10; `post_coursework` is **split** — the POST wrapper goes, the decoder stays as `decode_coursework_reply` and is what `collect_cloud` calls on `done`. The "Stays" list is corrected, and Task 10 says which of `coursework.rs`'s tests move with which half and why a loopback harness disappears from the diff. |
| **I8** | `sectionMapping` is not an export of `parse_vhl.ts` — it does not exist in any form | Task 7 adds it, `parseDashboard`'s two inline lookups call it, and the frozen `vhl-parsed-reference.json` proves the behaviour did not move. Named as one of C5's two edits to `ingest-coursework/`, in the ownership list and in *File structure*, so Task 9 does not discover it at the import line. |
| **I9** | Hand-off H2's code did not compile (`cs_data_dir` exists nowhere in `slot_argv`) | The line is `sessions.to_string_lossy().into_owned()`; `sessions_dir(&cs.data_dir)` stays at the one call site in `run_slot_inner`, where the hand-off already put it. The comment says what went wrong, because hand-offs are applied verbatim. |
| **I10** | H6 asked the controller to find a sentence the repository does not have | The disclosure is being **written**, not extended — so it is legal copy: it goes to Quinn and the lawyer with **Q6**, H6 carries the whole sentence and lands only after that, and Task 9 pins **both halves** of the panel's copy. The ledger row and spec §8 are corrected; if Quinn rules the existing lede is the disclosure, H6 is withdrawn and the spec says so. |
| **I11** | The session report could not support "re-authenticate only on expiry" for a cookie session | The report becomes `{"cookies": {"count", "session_cookies", "earliest_expiry"}, "captures": [{name, expires_at}]}` — no name, no value, no domain; `observe` supersedes, honours `Max-Age=0` and prunes, and `load` prunes on the way out. `ObservedCookie` gains `expires_at`. Two tests. The R4-17 ledger row says what is now true rather than "yes". **This settles the plan's own open decision 1**, together with 15(c) becoming blocking. |
| **I12** | `{kind:"reauth"}` carried no steps and the handler names no source, so nothing could compose the login | The `Plan` contract is explicit: `reauth` **names its source**, and the handler answers it by writing `reauthed: true` into that source's sub-cursor and re-entering `plan.start(ctx)`, so the module composes its own login steps and its `next` re-issues the failed step — once, never twice. Two lines of interface, written out in Task 6. |
| **I13** | `expires_at=lt.now()` is unverified PostgREST and a wrong literal 500s every call | **`expires_at=lt.now`**, with the reason (Postgres accepts the datetime input `now`, not the string `now()`), and a **fifth** staging proof that inserts an expired row, calls `/relay` once and reads it gone — because 401 and 402 both return before the sweep and only an entitled call would show it. |
| **I14** | A blown budget could be the warning that loses the run log | Every budget warning is phrased with an existing `FAILURE_MARKER` — `coursework: fetch failed (the run hit its round-trip budget); nothing changed` and three siblings — so **no marker is added** and `assert_eq!(FAILURE_MARKERS.len(), 7)` is untouched; the test asserts each one sorts first through `rank_warnings` against a benign per-item note. **This settles the plan's own open decision 3**, which the review agreed with and corrected. |

### Minor

| # | Resolution |
|---|---|
| **M1** | The two test-count literals (44, 39) and the predicted fall of "roughly 83" are gone; Task 10 counts `#[test]`s at the branch point and records the real number, and the review's own measurement (38 and 36 at `141b12d`) is cited as why. |
| **M2** | Corrected throughout both documents: `coursework_request` `:639`; `post_coursework` `:670`; `cloudmodel.rs` `:252`; `account::api_base` `:42` and `anon_key` `:46`; `privacy.html` `:24` (bullet) and `:38` (the `<dd>`, with `:37` named as the `<dt>` that does not move); `onboarding.rs:74`; the "one predicate" doc as `vhl_rows`'s own comment at `coursework.rs:1255-1257`; and `vhl.rs:342-344` for the "single most important line". |
| **M3** | H1a places `pub mod relay;` after `pub mod ranking;` (`lib.rs:72`) and says why the first draft's "before `pub mod runs;`" named a position that does not exist; H1b's snippet drops the `#[cfg(windows)]` that is not on either declaration. |
| **M4** | `vhl.rs` gets `use crate::relay::{quote_plus, scrub};` — `urlencode` calls `quote_plus` and an import of `scrub` alone leaves the file unable to compile. |
| **M5** | `RunBudget` and `SourceContext` are declared in full in Task 4, with every field and the reason for each. |
| **M6** | The corpus pins are no longer quoted as literals anywhere: the rule is "unchanged from the branch point", read off `migrations_test.ts` at the fork and recorded in Task 5's report. |
| **M7** | Task 12 step 1 and Global Constraints both use `.github/workflows/ci.yml:84`'s line verbatim, `--allow-write=cloud/eval` and `cloud/eval/` included. |
| **M8** | `replay_into` passes `rec.line.clone()`: `Cookie::parse` yields `Cookie<'a>` and `CookieJar::insert` wants `Cookie<'static>`, and the doc says the test compiles either way, which is how this would be found late. |
| **M9** | The `Location` rule is stated and tested: absolute is used as-is, root-relative is a path-and-query swap, **anything else** is `transport` naming "an unresolvable redirect". A refusal is safe because every hop is re-checked anyway. |
| **M10** | The real user path is gone: *"Quinn's own vault, whatever `profiles.json` names, is never a target."* Both documents were grepped for this machine's user name, `C:\Users`, `USERPROFILE` and a named home directory; that line was the only hit and there are none now. |
| **M11** | Both `cfg` arms named for `discover_json` (`:1290` / `:1366`) and for `collect_cloud` (`:745` / `:872`) — the non-Windows twins keep their one-line bodies and only their signatures move. |
| **M12** | `Cargo.lock` is out of the ownership list and out of Task 3's `git add`: it is a shared-single-owner file and the `windows` feature adds no package, so it is **expected not to change** and exit-gate item 12 is where that is checked. |

### The planner's three open decisions

Settled exactly as the review settled them: **decision 1** (session lifetime measured at the exit
gate) by **I11** plus making 15(c) blocking for Task 12 and ordering it after the report shape lands;
**decision 2** (the `readJson` cap) by **I1**, derived on both sides; **decision 3** (budget warnings
folded into `done.warnings`/`done.errors`) agreed, with **I14**'s correction to the phrasing so the
one durable log line is the budget line and not a benign per-item note.

---

## Fix round 2 — resolutions (2026-09-17)

Against the **Re-review after fix round 1** appended to
`docs/reports/2026-09-17-c5-relay-fetch-plan-review.md` (`9f186c7`, verdict *execute after fix round
2*, findings **R1-R10**, plus rulings on the author's five concerns). The re-review closed C1, C3
(in shape), C5 (in shape), I2, I4, I5, I7, I8, I9, I10, I12, I13, I14 and M1-M12, and left ten. Two
of the ten blocked; both are settled by controller rulings, recorded first.

### The two rulings

**R-C5-plan-4 — the pause's exit must be reachable (from R1).** `onboarding::store_credentials` is
registered **only in the wizard window** (`app/src/main.rs:107`), absent from the console's (`:186`),
and called by one thing: the wizard's panel-5 `storeCredentials()` on a vault that does not exist
yet. Settings has ten rows and none is a login. So the card's *"Open Settings and save the password
again"* was false, and the student's only exit was dismissing the card by hand
(`commands::close_info`) — which un-pauses with the same wrong password, the loop Q2 exists to
prevent. C2's deadlock had survived R-C5-plan-1 by one hop, and 15(d) would still have passed because
a scratch profile is made *by the wizard*. **Option (a):** the console's Settings gains a **Logins**
row, one per `relay::PORTAL_SOURCES` entry, backed by a new console-window command
`commands::save_portal_login` that writes the same `knowlu/<profile_id>/<source>` credential the
wizard writes and closes `login:<source>` on success — so R-C5-plan-1 now lives in the console as
well as the wizard. The card body becomes *"Open **Settings → Logins** and save the password again"*
and is true; exit-gate 15(d) is run **from the console**, without re-running the wizard and without
dismissing the card. Options (b) and (c) were refused: (b) — comparing the credential's `LastWritten`
— leaves a student with no way to change a portal password after onboarding at all, which is a
product gap regardless of the card; (c) — ruling that dismissal is the exit — is the loop.
Task 8 gains step 5a; `app/src/commands.rs`, `app/static/{index.html,console.js}` and
`app/tests/{commands,static_assets}.rs` enter the ownership list; hand-off **H11** registers the
command **and recounts both `generate_handler!` lists** rather than quoting a number, because C1b
merges ahead of this stream (`main` is 30/43 today, C1b takes it to 29/42, and **after C1b** C5 makes
it 29/43 — orientation only; the number that reaches `CLAUDE.md` is the one H11 observes).
*Cost if wrong: one settings row and one command.*

**R-C5-plan-5 — the expiry is parsed off the raw line (from R3).** I11's whole session report —
`expires_at`, the pruning, the `Max-Age=0` deletion, `earliest_expiry`, `session_cookies` — was to be
built "through the same `ureq::Cookie::parse` the replay uses". It cannot be: `ureq::Cookie` is a
newtype over `cookie_store::Cookie` with a private inner, and its public surface is `parse`, `name`,
`value` and a `Display` of `name=value` (`ureq-3.4.0/src/cookies.rs:44-92`; `as_cookie_store` is
`#[cfg(test)]`). `cookie_store` as a direct dependency is refused by this stream's own
`dependency_boundary.rs` case. **Option (a):** `Expires` and `Max-Age` are read off the raw
`Set-Cookie` line under **RFC 6265 §5.2.1 and §5.2.2** by twenty lines the module owns — `Max-Age`
winning where both appear (§4.1.2.2), an unreadable value ignored rather than fatal (§5.2.2), a
cookie whose expiry cannot be read counted as a **session** cookie — with tests for both attributes,
a missing one, a `Max-Age=0` deletion, case-insensitivity and precedence. **No `cookie_store` and no
other crate.** `earliest_expiry` and `session_cookies` stay meaningful and exit-gate 15(c) still
measures expiry rather than measuring the re-auth path twice. Options (b) (a bare count, which I11
rejected) and (c) (an extra round trip for VHL forever) were refused.
*Cost if wrong: a parser the size of a test.*

### The eight bounded findings

| # | Finding | Resolution |
|---|---|---|
| **R2** | `MAX_POST_BYTES = 16 << 20` contradicted the pin its own test asserts (`(MAX_RUN_BYTES*4)/3 + 1 MiB` = 12,233,386), the factor was base64's rather than JSON escaping's, `protocol.ts` did not export the constant `handler.ts` names, the reader compared only the other three, `a_results_batch_over_the_cap_is_a_413_not_a_500` was prose in no task, and `check_post` trimmed a fetched page away | The constant is now `MAX_RUN_BYTES * 2` — the same 16 MiB by the right route — with the escaping argument in its doc (`readJson` measures the JSON *text*; every `"` doubles, a control character becomes six characters, approaching 2× on a quote-dense page). `protocol.ts` exports `MAX_RUN_BYTES` and `MAX_POST_BYTES`, the cross-language reader covers all five, and the named test is written into Task 5 step 5 asserting the **relationship** plus a real 413. `check_post` becomes **`split_post`**: a batch over the bound is **posted as two** — the protocol already allows a batch of one — because calling a page the vendor served `too_large` loses a book, and a missing assignment no warning reports is the worst outcome this module has. |
| **R3** | — | **R-C5-plan-5**, above. |
| **R4** | `budget.remaining()` clamped at `STEP_TIMEOUT` is a bound on one **request**, so six hops of 59 s is a six-minute step; "the whole chain is bounded by `STEP_TIMEOUT`" had no mechanism; and both new tests passed a *run* deadline, so neither tested the step bound | A second deadline: `StepClock::start()` at `now + STEP_TIMEOUT`, opened as `perform`'s step 0, checked on every hop beside `budget.check_deadline()`, and every request gets `min(step.remaining(), budget.remaining())`. Its failure code is `timeout`, distinct from the run's `budget`, because "this portal stalled" and "the slot ran out of time" are different problems. The chain test now passes a **ten-minute run deadline and a 250 ms step deadline**, so only the step bound can stop it. |
| **R5** | `src.split("#[cfg(test)]").next()` truncates `relay.rs` at Task 1's own mid-file `#[cfg(test)] fn extra_source`, so the `.expose()` count reads 0 and fails — I6's defect one task over — and the portal-URL scan would read only the head of the one file that could hold a portal host | `split_before_test_module(src)` splits on the attribute **followed by `mod tests`**, tolerant of the whitespace rustfmt emits, declared in Task 1 beside the matcher and used by both scans. Both call sites carry the reason in a comment, because the failure mode is a scan that stops meaning anything without failing. |
| **R6** | `PlanCtx`, `PlanStep`, `Parsed` and `RunRow` were consumed in four places and declared nowhere — M5's shape reintroduced in TypeScript — `next()` carried no batch budget while the zyBooks module was told to batch "at whatever the round-robin left", and `parseArrival` switched on `source === "zybooks"` against the rule stated three times that the handler names no source | All four declared in full in `plans/mod.ts`, with `PlanCtx.budget` carrying **how many steps this module may return right now** and the module advancing its cursor by exactly what it returns — so the driver never trims past a cursor a module has advanced, which would drop a book silently. `SourcePlan extends Plan` with a `parse(ctx, result)`, and `parseArrival` calls `SOURCE_PLANS[source].parse`: a third portal is a file and a line, not an edit to the driver. |
| **R7** | The extraction took the happy path and left `ingestHandler`'s per-source `try/catch` behind — which turns `NotLoggedIn` into `session invalid (…)` and anything else into `parse failed (…)`, **discards that source's `own` while keeping its proposals**, and skips the `0 assignments parsed` line — and incrementally a dead session on book 7 of 12 must do all of that across round trips | The catch is the **fourth** extracted function, `sourceFailure(name, e, proposals) -> Parsed`, and the rule it carries across round trips is written out in five clauses: the source is retired for the run, its `own` is discarded, its proposals are kept, **its already-parsed items are discarded too** (a source that threw contributed nothing to a one-shot call), no `0 assignments parsed` line is added on top, and a re-authenticable `NotLoggedIn` is `reauth` and never reaches it. **Corrected in round 3 (review S1):** this row claimed "Task 5 asserts each clause", which was **not true of the third** — `parsed.assignments` was a flat unsourced array, so nothing could identify a retired source's earlier rows to drop, and the one retirement test failed VHL on its first step, before it had contributed a row. Round 3 partitions `parsed` by source and adds the test the clause's own example names. |
| **R8** | `parsed`'s default said `{assignments, warnings, proposals}` while the handler writes `parsed.own[source]`, and `checkParsed` was "rows only, never bytes" with no length rule — so a `parse failed (<vendor message>)` warning could carry page text into the column and from there into `state/runner-log.md` | The column's default and the `RunRow` type both say `{assignments, own, proposals}` with `own` a map of source to un-prefixed warnings; `checkParsed` gets **`checkCursor`'s 4 KiB string cap** and a real test in Task 5 step 5 (a shape check, an over-long warning, a page-shaped key, and the wrong-shape default) — the first round named `parsed_holds_only_parsed_rows` in the migration comment and wrote it into no task. |
| **R9** | I3's scrub ran on the decoded `String`, so a body that is not valid UTF-8 travelled base64 **unscrubbed** — a vendor re-serving a login page in a legacy encoding being I3's own case | The scrub moves **before** the decode decision and runs over the **bytes**: for each secret and each of `scrub`'s four encodings, the raw slice is searched and replaced. Two tests — a portal echoing the posted form as UTF-8, and one echoing it as Latin-1, where the assertion is that the base64 does not contain the username. |
| **R10** | Three residual cites | `privacy.html:37`/`:22` in Q6's row → `:38`/`:24`; `handler.ts:78-110` → `:84-109`, `:179-186` → `:177-182`, the VHL arm `:133` → `:137`; `parse_vhl.ts:81` → `:99-101` (`:81` is the `sections` extraction, not the lookup). |

### The author's five concerns, as ruled

1. **`parsed` unmeasured — accepted, sentence added.** It is already bounded by `MAX_RUN_BYTES`
   (8 MiB of raw body per run bounds everything that can be parsed into it); the migration comment
   now says so, in the plan and in the spec.
2. **The extraction's oracle — shape accepted, claim corrected.**
   `ingest-coursework/handler_test.ts` (seven cases, a dead session and a one-source-fails among
   them) is the oracle. The two frozen references pin `parseAssignments`/`parseDashboard`, which the
   extraction does not touch, so they would stay green over a broken one — the plan now says that
   rather than claiming them. **R7** is what the suite still missed.
3. **The edge body ceiling — ruled a finding, with the offline answer.** Measured at Task 5 step 9 by
   the controller, who has staging and a session: one padded body at the cap, recording the status
   **and whether it reached the function at all** (a platform refusal is a gateway shape, not our
   `{"error": …}`). The fallback is named now — `MAX_BYTES` 2 MiB → **1 MiB** and `MAX_POST_BYTES`
   with it, both moving together because the pin ties them — and the device gets a runtime one: a 413
   mid-run is already `the service refused`, so `run` **halves the batch and continues**.
4. **H6's wording — accepted.** It is Q6's, marked as this plan's draft for Quinn and the lawyer to
   approve or replace whole, and H6 lands only after they have.
5. **Nothing executed — accepted.** A paragraph at the head of *The tasks* says it plainly: every
   signature, API call and count here is read, not observed; two rounds have already found four
   defects of exactly that kind; a step that will not compile is expected traffic, to be fixed and
   recorded in the task report.

---

## Fix round 3 — resolutions (2026-09-17)

Against the **Re-review after fix round 2** appended to
`docs/reports/2026-09-17-c5-relay-fetch-plan-review.md` (`cca7699`, verdict *execute after fix round
3*). That round closed **R1-R10**, both controller rulings and concern 3, each checked against the
code it names — `commands::save_portal_login` registered console-only and the 30/43 → 29/42 → 29/43
arithmetic verified against C1b's own plan; `ureq::Cookie`'s public surface confirmed against the
vendored source; `MAX_POST_BYTES` matched to `readJson`'s actual `text.length` behaviour and its
"body over N characters" message; `StepClock`; `split_before_test_module`; the four declared
interfaces; `checkParsed`'s cap; the byte-level scrub; all four citations. **One gap remained.**

### S1 — R7's hardest clause had a rule and no mechanism, and no test

**The finding.** R7's third clause says that when a source throws on round trip N, the items it
contributed on round trips 1..N-1 — already merged into `relay_runs.parsed` — are **removed**,
because that is what a single `ingestHandler` call did. Round 2 wrote the rule and did not build it:

- `RunRow.parsed.assignments` was one **flat, unsourced `Assignment[]`**, and `Parsed.items` carried
  no source tag, so nothing in the merge — described only as "`appendParsed` whatever it parsed" —
  could say which entries were the failing source's. `own` had been partitioned in round 2;
  `assignments` had not.
- `finishRun`'s own comment promised to "concatenate by `PORTAL_SOURCES` order and, within a source,
  by arrival index", which a flat array cannot do once two sources' round trips interleave — and they
  do, by construction, because the batch is filled round-robin.
- The one retirement test failed VHL on its **very first step**, before it had contributed a row, so
  it could not and did not exercise "six books' rows do not survive book 7" — the rule's own example.
- And the round-2 R7 resolution row claimed "Task 5 asserts each clause", which was untrue of this
  one.

**The fix, as the reviewer wrote it.**

1. **`parsed` is partitioned by source, all three of it.** `assignments`, `own` and `proposals` are
   each `Record<string, …>` keyed by a `PORTAL_SOURCES` name — the shape `own` already had. The
   column default and the `RunRow` type say so in the plan and in the spec's §6 SQL block, and the
   migration comment says why.
2. **`Parsed` names its source.** A `source: string` field, set by `parseArrival` from its own
   argument and by `sourceFailure` from its `name`, so no caller has to remember which arrival a
   result came from.
3. **`appendParsed` is stated rather than implied.** It appends each `Parsed` under its own `source`
   — never by position, never by a literal name — and on `failed` it **deletes
   `parsed.assignments[source]` and `parsed.own[source]`, keeps `parsed.proposals[source]`**, then
   records the one failure sentence. R7's clause is now a line of code.
4. **`finishRun` is the only flattener**, and it is written out: `PORTAL_SOURCES` order for the outer
   loop, each source's array already in arrival order, `finishSource` per source. The order is right
   by construction rather than by sorting.
5. **The test the rule's own example names** — `a source that fails on a LATER round trip loses the
   rows it contributed on earlier ones`: zyBooks signs in and lists twelve books, books 1-6 come back
   parsed over two round trips, book 7 answers `success: false` twice (so it is `failed`, not
   `reauth`), and VHL finishes. The reply carries VHL's rows and **none** of zyBooks'; zyBooks
   reports exactly one warning and it is the failure, with no `0 assignments parsed` line on top; and
   the proposals zyBooks had already pushed survive. The first-step failure keeps its own test, under
   a name that now says which case it is.
6. **Two structural guards beside it**: `parsed is partitioned by source, and only finishRun flattens
   it` (every key is a `PORTAL_SOURCES` name, neither map is an array, and exactly one
   `.assignments.push(` exists in `handler.ts`), and an order assertion added to the twelve-book
   equality test — zyBooks' rows first, then VHL's, even though VHL's dashboard arrived in an earlier
   round trip than books 9-12, which a flat array appended to as arrivals landed would interleave.
7. **The round-2 R7 resolution row is corrected on the record** rather than quietly rewritten: it
   says which clause its "Task 5 asserts each clause" did not cover, and what round 3 did about it.
8. **Exit-gate item 10b** now states the partition and names the new test beside the old one.

Nothing else in the plan or the spec changed in this round. The spec's §6 was touched only where its
SQL block spelled the flat default.
