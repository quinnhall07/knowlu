# Knowlu C2 — the judgment service — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: EXECUTED 2026-09-14/15 — Tasks 0–15 complete on branch `c2-judge` (draft PR #4 → `main`, CI green on every commit); the merge itself waits on the final whole-branch review and Quinn's word.** The §5.4 m2 fidelity-ledger row and exit-gate item 11 below are both **overridden by Quinn's 2026-09-14 decision** (ruling R-C2-E12: nothing is ever read from the archived vault) — see the notes at each. Executed on a branch `c2-judge` in a worktree of this repository (`knowlu`), to be merged **after** C1 (C1 merged 2026-09-10). Written against `docs/specs/2026-09-09-knowlu-cloud-design.md` (**SIGNED** 2026-09-09) — §1 D3, D4, D6, D11, D12; §3.2; §4.3; §5.2; §5.3; §5.4; §6 row (b); §8; §9; §12's C2 row — and against Quinn's §11 rulings R5, R6 and R8 of the same day.

**Goal:** The three judgments Knowlu makes — *what is this task worth and how long will it take*, *does this event matter*, *is this email a task* — stop being a llama.cpp process on the student's laptop and become `POST /judge-task`, `/judge-event` and `/judge-email` on our Supabase project, answered by a pinned Anthropic model under a JSON schema at temperature 0, logged by model id and prompt hash, and pulled back down by the engine inside the slot it already runs. The judgment gap that opened at the repo cut closes at Task 4.

**Architecture:** One shared pipeline in Deno (`_shared/judge_pipeline.ts`): entitlement → per-account daily cap → per-account rule lookup (tier 2) → one constrained model call (tier 3) → validation against a confidence floor of 0.6 and the account's known-course list → one row in `judgments` naming the model, the prompt version, the grammar version, the prompt hash and the elapsed milliseconds → a reply carrying the verdict and the tier that answered. Three thin `index.ts` handlers sit on it. On the device, `judge::Model` gains one production implementation — `cloudmodel::CloudModel`, `ureq`, bearer = the account's Supabase session JWT out of Credential Manager, one call per item, the same 120-second per-call bound the process runtime had — and two sibling traits, `EventModel` and `EmailModel`, for the two judgments that were never tasks. **Delivery is always a pull by the engine inside a slot step**: `judge` calls `/judge-*` per item and writes every reply through `write`; `coursework` posts the raw portal payload to `/ingest-coursework` and writes the reconciled list; `judge` also pulls queued Gmail results and promoted-rule proposals. Nothing is ever pushed at the device, and nothing here waits on C3's journal sync.

**Tech Stack:** Supabase (Postgres 15 + Edge Functions on Deno) for the service; `@anthropic-ai/sdk` for the model; `deno test` for every pure module and one request/response contract test per function; Rust 1.98 `stable-x86_64-pc-windows-gnu` with `ureq` 3.4 (`cookies`, rustls) for the device half; `std::net::TcpListener` on `127.0.0.1:0` and `Deno.serve` on `127.0.0.1:0` for every test that exercises a client.

**Spec:** `docs/specs/2026-09-09-knowlu-cloud-design.md`. Read §3.2, §4.3, §5.2, §5.3, §5.4, §6 and §9 before Task 0; the parent vault schema is `docs/specs/2026-08-11-personal-ops-system-design.md` §4, and the judge seam this plan keeps is `docs/plans/2026-09-07-knowlu-judge-and-enrichment-plan.md` with its ledger `docs/reports/2026-09-07-knowlu-plan-3a-sdd-ledger.md`.

## Global Constraints

Every task's requirements implicitly include this section.

- **`rank` never calls a model** (Knowlu spec decision 11, CLAUDE.md). `rank` may fetch a feed — it already does — but no path under `cli.rs` may reach `/judge-*`. Judgment is the separate `judge` command, which writes fields into notes before `rank` reads them.
- **`knowlu-engine judge` always exits 0.** No `config/cloud.yaml`, no session in Credential Manager, no network, a 402, a 429, a 5xx, a timeout — all normal outcomes reported on stdout as named lines. A non-zero exit sets `RunSummary.engine_ok = false` in the app's scheduler, which paints the tray amber and puts the slot into retry backoff twice a day forever. `coursework` exits 0 for the same reason; `ingest` does not, and that difference stays.
- **No single-user assumptions.** Nothing in `cloud/`, `engine/` or the eval seed names a person, a vault, a machine, an account or a credential. Anything that would need hand-editing for a second user is a bug.
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` are read-only oracles; `zybooks-parsed-reference.json` and `vhl-parsed-reference.json` become the TypeScript parsers' oracle too, **read by relative path from `engine/tests/fixtures/`, never copied into `cloud/`**. The three Rust-generated surface references (`surface-today-{s1,s1-migrated,full}.json`) may be regenerated only in a commit whose diff shows the change and whose message says why (console spec §4.6) — no task here regenerates one.
- `engine/tests/oracle.rs` and `engine/tests/surface_oracle.rs` must pass **unchanged** at every task boundary. The coursework fetch/parse split must not change a byte of `today.md` on the three fixture vaults.
- **No test reaches the network.** A `TcpListener` (Rust) or `Deno.serve` (TypeScript) bound to `127.0.0.1:0` inside one test, answering that same test's own request, is not egress: no DNS, no route off the machine, no listener on a routable interface. Every such test binds `127.0.0.1`, never `0.0.0.0`, and joins its listener thread (or `await`s its server's `shutdown()`) before returning. Nothing else in this plan opens a socket in a test.
- **No engine command is ever run against the worktree root or a real vault** (ruling R-3a-12). No `--vault .`, no path inside the worktree, and never `C:\Users\danie\GitHub\quinn-ops`. Tests copy a fixture vault into a temp directory.
- **TDD, per task:** the failing test first, run it and see it fail, the minimal implementation, run it and see it pass, commit.
- `cargo build --workspace` and `cargo test --workspace` from the root at **0 warnings**. The one accepted line is the app's `.rsrc merge failure: multiple non-default manifests` linker message. `cargo test --release` will not link — test in the dev profile.
- **Line endings: LF everywhere** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` CRLF; `engine/tests/fixtures/** -text`). New `.ts`, `.sql`, `.rs` and `.md` files are LF. `git diff --stat` never shows a whole-file flip.
- **No secret in the repo, a log, a fixture, a test name, a migration or this plan.** The three the service needs are Supabase project secrets Quinn sets by hand: `ANTHROPIC_API_KEY`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`. This plan names them and never a value. A request body, a message body and an OAuth token never enter a log line on either side.
- **Migrations are applied to the STAGING project only** (§11 R6). Nothing in this plan touches the production project.
- **Commits:** specific `git add` (never `git add -A`), the message through a file (`git commit -F <file>`), and both trailers. The two literals below are **the writing session's**; the executing session substitutes its own model name and session URL, exactly as `HANDOFF.md` §5 says (ruling R-C2-6 — this is not a defect to fix, it is a template to fill):

```
Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
```

- **File ownership.** C2 owns `engine/tests/**` — not merely "new files under it". This plan **modifies** the existing `engine/tests/dependency_boundary.rs` (spec §8 asks for exactly that), so the `git diff --name-only main...c2-judge` check in Task 15 must not flag it.

- Never bare `git stash` / `git stash pop` (ruling R-3a-29): the stash stack is shared with the main checkout and every worktree.

---

## Quinn-owned preconditions (asked one at a time when a task reaches them)

| # | Needed by | What | What breaks without it |
|---|---|---|---|
| P0 | Task 1 | **The Supabase projects themselves — C1's precondition, referenced here, not re-asked.** An organisation with `knowlu-prod` and `knowlu-staging` (region US); the staging project ref, URL and public anon key (not secret); the staging **service-role key** as a secret Quinn sets. C1's plan asks for these; C2 consumes them. | Every task from 1 onward. There is nowhere to apply a migration and nothing to deploy a function to. Task 0 and the pure-module halves of Tasks 2, 5 and 6 still run. |
| P0a | Task 8 | **`SOURCES_ENC_KEY` — also C1's precondition, referenced not re-asked.** The 32-byte base64 AES-GCM key C1 sets as a staging function secret and uses in `_shared/crypto.ts` to encrypt the LMS capability URL into `sources.url_ciphertext` / `sources.url_iv`. C2's `/ingest-ics` decrypts with the same key through C1's `importAesKey` / `decryptString`. | Task 8 only. `/ingest-ics` returns a named 502 for every account and `ingest` stays on the vault's `ics_url` fallback — which still works, so nothing breaks; the URL simply never leaves the vault. |
| P1 | Task 0 step 7; hard-required by Task 3 | **`ANTHROPIC_API_KEY` in the staging project's secrets**, set by Quinn from his own shell: `supabase secrets set ANTHROPIC_API_KEY=<value> --project-ref <staging ref>`. Never typed into a session, never in a tracked file. Production gets its own key at go-live, not here. | Task 3's live smoke check and every eval run. Every `deno test` still passes — the model is behind an interface with a scripted fake — so the pipeline can be built and reviewed without it, but nothing real is ever judged. |
| P2 | Task 10 | **The Google Cloud OAuth client** on the existing Cloud project: an OAuth 2.0 **Web application** client whose authorised redirect URI is `https://<staging ref>.supabase.co/functions/v1/google-callback`. Values needed: the client id and the client secret, set as staging secrets `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`. The **consent screen** is C1's track (§11 R1) and is not re-asked here — but say one thing to Quinn when this is asked: per §11a the consent screen now needs **two** scopes listed, `calendar.readonly` (*sensitive*: lighter review, no CASA) and `gmail.readonly` (*restricted*: verification plus the annual CASA), and **the calendar one can be submitted and cleared on its own, first**. | Task 10 cannot be smoke-tested. `/google-connect` returns a named 503, the wizard's Google button stays off, and **the personal calendar reaches Knowlu by its secret iCal address instead** (C1's `calendar_ics` path, which is the fallback anyway); the Gmail step stays off. Everything else in C2 ships. |
| P3 | Task 11 | **Quinn added as the first named Gmail test user** on that consent screen while the project is in *Testing*. Google's limits then apply and the wizard says so: at most 100 test users, a tester warning screen, refresh tokens that expire after 7 days (§5.3, §9). | Task 11's end-to-end read cannot be run against a real mailbox. The endpoint and its contract tests still ship; the reader has no account to read. |
| P4 | Task 13 | **The one-time explicit go to read the `quinn-ops` archive** for the eval seed (§5.4 measure 2). Read-only, one task, one sitting: the archived vault's seven course notes, its `state/events-seen.md` verdict lines, and the journal's amend/override records. Nothing that names a person is copied; the output is `cloud/eval/seed/**`. **Until this go is given, no task in this plan opens that repository for any reason.** | Task 14's eval suite runs on corrections alone. At merge time there are none — nobody has corrected a cloud judgment yet — so the suite has no floor and the deploy gate is vacuous. The seed is what makes the gate mean something on day one. |

---

## Fidelity ledger

| # | Decision / ruling | Source | Carried by |
|---|---|---|---|
| D3 | All three judgments — enrichment, event verdicts, email triage — run in the cloud; no local runs; the cheapest model that does the job | cloud design §1 D3, §5.2 | Tasks 2, 3, 4 (`/judge-task` + `CloudModel`), 9 (`/judge-event`), 11 (`/judge-email`); `models` seeded at `claude-haiku-4-5` in Task 1, changed only by a migration row the eval justifies |
| D3a | **The judge *seam* stays; only tier 3 changes.** `judge.rs`, `judgelog.rs`, `enrich.rs`, `write::propose_amendment` unchanged in shape; the 3a scripted fakes keep working | cloud design §3.2 | Task 4 — `CloudModel` implements the existing `judge::Model`; every 3a test in `judge.rs` and `enrich.rs` still passes |
| D4 | The deterministic, offline-capable core stays in the app; everything that needs the internet moves to the cloud | cloud design §1 D4, §3.1 | Tasks 7, 8, 9: parse+reconcile for the credentialed portals moves; ranking, capacity, rendering, journal and frontmatter surgery do not move at all |
| D5(b) | Corrections to AI judgments are collected under the ToS and are the eval suite's material | cloud design §1 D5, §6 | Task 1's `corrections` table and Task 14's suite. The ingest path is **C1's `POST /telemetry`**; C2 owns the table and the query. **Narrowed by ruling R-C2-4** — see the §5.4 row below |
| §5.4 m2 | "The last 90 days of corrections per kind **plus** a frozen seed, replayed on every prompt/pin change" | cloud design §5.4 measure 2 | **Narrowed, on the record (ruling R-C2-4).** §6 (b) carries `(item_id, field, ours, theirs, kind)` and **no note body**, and §5.2 discards bodies after the call — both signed — so a correction cannot become a *replayable* eval case without storing content the user has not consented to. Ruling: corrections feed **rule promotion and the correction-rate metrics for every account** (features only, Task 12); they become **eval cases only under the (c) opt-in**, where the device sends the re-derived judge request beside the correction — and **never** for `origin = 'gmail_api'` rows (§5.3, §9). The (c) toggle's UI is C4, so **the eval suite is seed-only until C4**. Carried by: this row, Task 14's `eval_cases.source` check, the exit gate's item 11, and the optional `request` field in *Interfaces with C1* contract 4. **Overridden 2026-09-14 (ruling R-C2-E12, on Task 15's ledger): Quinn declined the archive read outright — "nothing is ever read from the archived vault... all the info should come from onboarding and running it." There is no frozen seed at all; `cloud/eval/` ships only the case schema, the scrub test and a loader that returns `[]` for an absent or empty `seed/`, and the suite stays empty (not seed-only) until C4's (c)-opt-in toggle lets consented corrections fill it** |
| §5.3 | "The service counts what it has queued so it never proposes past [the 15-a-day cap]" | cloud design §5.3 | **Narrowed, on the record.** The cap stays **the engine's**: `approvals::defer_over_budget` counts every pending card created today, whatever created it, and snoozes the surplus to tomorrow — deferred, never deleted. A second counter server-side would be a second answer to one question, and the server cannot see cards the student created by hand. Carried by Task 11, which files Gmail proposals through the ordinary card path so the existing cap applies unchanged, and by `an_over_budget_gmail_batch_is_snoozed_not_dropped` |
| §5.3 | "with user-excludable labels" on the twice-daily mailbox scan | cloud design §5.3 | **Deferred, on the record.** Task 11 scans `newer_than:7d` across the mailbox with no label filter, because there is no UI to choose a label — the settings panel is C1's and the wizard's Gmail step (C1) asks only *connect or not*. Task 11 carries the seam (`google_accounts.excluded_labels text[] not null default '{}'`, read by `gmail-read` and applied as `-label:<name>` terms) so the day C1 or C4 adds the control it is one UI change and no schema change. Carried by `excluded_labels_become_negative_label_terms` |
| D6 | Deterministic measures: rule promotion, an eval suite from corrections, grammars on every call, pinned models with logged prompt hashes | cloud design §1 D6, §5.4 | Task 2 (schema-constrained call; model id, prompt version, grammar version, prompt hash and ms on every row), Task 12 (promotion), Task 14 (the suite and its deploy gate) |
| D11 | Portal scraping stays on the device — *fetch on device, think in the cloud* | cloud design §1 D11, §4.3, §9 | Task 7: `coursework` keeps `wincred` + `signin` + `login_and_fetch_dashboard` and stops before parsing; `/ingest-coursework` never receives a portal password, and `the_coursework_payload_carries_no_credential` proves it |
| D12 | Email ingestion is Gmail OAuth (`gmail.readonly`), server-side; the refresh token never reaches the device | cloud design §1 D12, §5.3, §9 | Tasks 10 and 11: token exchanged and stored in Supabase Vault; disconnect revokes at Google. **Amended by §11a below: `gmail.readonly` is no longer the first scope asked for** |
| §11a | **"The personal calendar is connected first, together with the school one."** C1's wizard step is *Connect your calendars*; **C2 adds Google sign-in for the same panel — one Google connect for `calendar.readonly` first (a *sensitive* scope: lighter verification, no CASA) and `gmail.readonly` incrementally — with the calendar fetch server-side beside `/ingest-ics`; until C2 lands, the secret address is the path** | cloud design §11a, ruling of 2026-09-09 | Task 10 becomes **`google-connect` / `google-callback`** (renamed from `gmail-*`, because one grant now carries either or both scopes) and asks `calendar.readonly` **first**, adding `gmail.readonly` by incremental authorisation only when the wizard's Gmail step is taken; `google_accounts.scopes text[]` records what was actually granted. Task 8 gains **`/ingest-calendar`** beside `/ingest-ics`, resolving `?name=personal` to the account's `calendar_ics` **`sources`** row and `?name=google` to its **`google_accounts`** grant (ruling R-X-9 — the Google calendar is *not* a `sources` row; a grant has no URL to encrypt, and `google_calendar` stays a reserved value nobody writes), and returning **ICS** either way, so `calfeed`'s parser, its 28-day horizon, its dedup and its snapshot fallback are all unchanged. The `calendar_ics` path keeps working through the device throughout, and is the fallback until the OAuth path is verified. **Testing-mode realities apply to `calendar.readonly` exactly as to Gmail** — 100 test users, 7-day refresh tokens — and the wizard says so for both. The button and everything behind it are hand-off **H9**, in **two phases matching C1's two windows** (ruling R-X-15): (a) the **wizard** window gets `google_connect_url`, `google_connected` and `open_external` — all vault-less, on C1's `PENDING_TARGET` helpers — the panel sets `WizardPlan.google_calendar`, and `scaffold::ingest_yaml` writes `- name: google` / `ics_url: 'cloud:google'` **when the vault is born at Finish**, never before; (b) the **console** window gets `set_google_calendar(vault, connected)` for a vault that already exists, whose settings row is **C4's**. H9 touches eight files across three owners and moves C1's command counts by four, so C1's Task 21 recount is re-run rather than edited |
| D2 | An account is required; entitlement is "subscription active" | cloud design §1 D2, §5.1 | Every function's first line is C1's `requireActiveEntitlement(req)`; the engine turns a 402 into `judge (skipped: no entitlement)` and exit 0 |
| D7 | Cloudflare + Supabase; no Firebase | cloud design §1 D7 | Everything under `cloud/supabase/` |
| §10 | VISION amendment: **"Judgment is a service… Nothing model-shaped ships in the app."** | cloud design §10 | Tasks 2–4 put the prompt, the schema and the model id server-side; from Task 4's merge the engine carries no prompt text on the cloud path |
| §10 | VISION amendment: **"Credentials for the student's own portals never leave the student's machine."** | cloud design §10 | Task 7's `the_coursework_payload_carries_no_credential`: no username, no password, no `credential_target`, no filesystem path in the request body |
| §10 | VISION open decision closed: cloud inference provider = **Anthropic API**, cheapest model per kind that passes the eval | cloud design §10, §11 R8 | Task 0's `_shared/judge_anthropic.ts` is the only file in the repository that imports the SDK; the provider is swappable behind `JudgeModel` |
| §11 R5 | Global rules (cross-account promotion) are **reviewed by hand** before activation | cloud design §11 R5 | Task 12: `rules.scope = 'global'` rows are inserted `active = false`, and there is no code path that activates one; the promotion job only ever writes `scope = 'account'` |
| §11 R6 | Staging project from day one | cloud design §11 R6 | Every migration and deploy in this plan targets the staging ref; Task 15 records what production still needs |
| §11 R8 | Anthropic as the launch provider; **the eval suite picks the model, not taste** | cloud design §11 R8 | Task 1 seeds `models` at the cheapest Haiku-class id; Task 14 is the only thing allowed to change a pin, and only through a migration row |
| §11a | **R-OB-1 (C1 + C2): an unknown book on a later run is a proposal, never a silent skip** (`/ingest-coursework` reconcile) | cloud design §11a, ruling of 2026-09-09 | **Task 7a.** The reply gains `proposals`; `pickZybooks`'s `unmapped` branch stops warning, and the handler translates `parse_vhl`'s own `section … not in config` warning the same way — **so both parsers stay byte-identical to their frozen references and the policy lives where the routing already did**. The engine writes one `kind: coursework-map` card per unmapped key through `write::create` (hand-off H12 keeps `rank` from touching it), the deck answers it, and `coursework` applies it **before its next fetch**. **The mapping's home is the vault's `config/ingest.yaml`** (§4.3: *the parsers move; the credentials and the vault stay*) — it is already there, the device already sends a redacted copy on every call, and C1's wizard writes the initial one, so a server-side copy would be a second writer of one value. Recorded in *Interfaces with C1* contract 8 |
| §11a | **R-OB-3 (C2, and the device's `ingest` until then): a first ingest never creates a task already past due** — such items are recorded as seen and archived as `imported-past` | cloud design §11a, ruling of 2026-09-09 | **Task 8a**, in two halves with one of them authoritative. The **device's** (hand-off H11) is the guarantee: it works with no account, it is the only half that can write `archive/` and `state/ingest-seen.md`, and it is what a pre-C1 install runs — `sync_tasks` gains `first_run`, and a past-due item on a first run is **one `create` into `archive/`** with `status: archived` and `archived_reason: imported-past`, plus one `record_seen`. Not skipped (the uid would be unseen and the next run would create it) and not created-then-deleted (two journal records and a note that briefly ranks). The **server's** `GET /ingest-ics?first_run=1` returns `past_due_uids` as corroboration and as what the wizard counts; when they disagree the device wins, because the device knows the vault's timezone. **Strictly before today, never before *now*** — an item due at 23:59 today is today's work; the residue, stated so nobody reads exit-gate 7b as a stronger promise than it is, is that a vault created at 14:00 still imports this morning's 09:00 deadline as a live, already-overdue task, and that is the right side to err on. Later runs are unchanged, because an item that goes past due while the vault is watching is exactly what the system exists to shout about. **The `first_run` predicate is ruling R-C2-9's** (the row below), not the seen-ledger's. Carried by `ingest_on_a_fresh_vault_archives_past_due_items` |
| §11a | **R-C2-9: `first_run` is the absence of `state/today.md`, not of `state/ingest-seen.md`** | ruling of 2026-09-09, on the re-review's Critical 1 | **Task 8a / hand-off H11, Change 1.** The amendment's first draft derived the flag from `state/ingest-seen.md`. That ledger is shared: `coursework` imports `crate::ingest::record_seen` (`coursework.rs:19`) and calls it at `:310` and `:415`, and the slot order is `coursework → ingest → judge → rank` — so the file exists by the time `ingest` runs and R-OB-3 never fires on the one slot it was written for. `ingest::is_first_run(vault) = !vault.join("state").join("today.md").exists()` instead: `today.md` is written by `rank`, the last step, and it is the same predicate the app already uses to decide a vault needs its first slot (`app/src/scheduler.rs::needs_first_run`, main `f8649d5`). **Residue, accepted:** a first slot that fails before `rank` makes the next slot "first" again — nothing is double-archived (the uid is in the seen-ledger) and no page exists yet for the student to have seen the difference. Carried by `first_run_is_the_absence_of_today_md_not_of_ingest_seen` |
| §11a | **R-C2-8: `apply_map_cards` may not round-trip `config/ingest.yaml`** — the mapping is inserted at the text level and only ever ADDS a key | ruling of 2026-09-09, on CLAUDE.md's engine invariant | **Task 7a, step 7.** The first draft of `write_mapping` parsed the file with `serde_yaml_ng` and wrote it back with `to_string`, on the reading that CLAUDE.md's "no note is ever parsed and re-dumped" is a rule about *notes*. **Overruled:** the invariant is absolute, `src/yamlemit.rs` is the crate's one YAML emitter, and `scaffold::ingest_yaml` writing the file at birth is creation, not a rewrite. `write_mapping` now finds the `courses:` / `sections:` line, inserts three lines beneath it at the file's own indentation through `pystr::read_text` / `write_text`, and changes exactly one existing line — a `courses: {}` scalar replaced by the block form. **The lost ability to update an existing key costs nothing**: a mapped book never yields a card, so no card ever names a key that is already there. Carried by `a_mapping_is_inserted_under_an_existing_block`, `an_empty_flow_mapping_becomes_the_block_form`, `a_second_apply_of_the_same_key_is_a_no_op` and `every_byte_outside_the_inserted_lines_is_unchanged`; stated in *Interfaces with C1* contract 4a |
| §11a | **R-OB-2 (C1): the sign-in window also captures the enrolled course list and seeds `courses/` and `course_map`** | cloud design §11a | **C1's, and named here because C2 is the visible consequence of it not existing yet.** Until it lands, a fresh vault has no `course_map` and no `courses/` notes, so every Blackboard task arrives `course: null` with `needs_enrichment: true` — which is the accepted judgment gap, and which C2's `/judge-task` is the answer to (§11a's own last sentence). C2 builds none of it; `Heuristics::knows_course` already degrades correctly to "no course rather than a wrong one" |
| §5.2 | Confidence floor 0.6, known-course check; the log holds ids, field values and confidences and **never the body** | cloud design §5.2, §5.6 | Task 2's `validate()` and `logJudgment()`. The `judgments` table has no column a body can go in, and Task 2's `a_body_token_reaches_no_judgment_row` plants a token and proves it |
| §5.2 | Per-account daily caps (enrichment 200, events 300, email 500), a per-call timeout, a monthly budget alert | cloud design §5.2 | Task 1's `usage_daily`, Task 2's `chargeCap()`, Task 3's `CALL_TIMEOUT_MS`, Task 14's `monthly_spend` view and its alert threshold |
| §5.3 | `origin = 'gmail_api'` on every derived row; the (c) export filter excludes them; message text discarded after judgment | cloud design §5.3, §9 | Task 11: the `origin` column, `export_training_rows()` and `gmail_rows_are_excluded_from_the_training_export` |
| R6 (3a) | The prompt is built from the note's own fields, the course's grade weights and `profile/preferences.md`, **and from nothing else**; `config/ingest.yaml` never reaches it | plan 3a fidelity row R6 | Task 4 **extends** `judge.rs`'s `the_prompt_carries_the_note_and_its_grounding_and_nothing_else` from the prompt string to the HTTP request body |
| D5 (3a) | *"local models, never a cloud API… no task adds an API key, a token or a remote inference endpoint"* | plan 3a fidelity row D5 | **Deliberately reversed by D3.** Named here so the reversal is on the record and not an omission |
| R-P3a-2 | *No unverified path to executing a runtime binary* (the `SUPPORTED_RUNTIMES` digest table) | plan 3a | **Retired, deliberately.** A cloud call executes nothing, so the digest table has no analogue. The property that replaces it: the endpoint comes from the vault's own `config/cloud.yaml`; TLS is rustls/ring and never OpenSSL; no cloud SDK enters the engine (Task 4's new `dependency_boundary.rs` pin); and the session token never reaches a log, a note, a backup, a prompt or a warning line |
| R-3a-20 | `LowConfidence` carries a **structural** `LowCause`; the closed vocabulary goes to the log, the full text to stdout | plan 3a ledger | Task 4 keeps `LowCause` and adds no free-text field to `judgelog::Entry` |
| R-3a-21 | The `fields` privacy check is end-to-end: plant a token in title and body, run a full pass, assert it reaches no log line | plan 3a ledger | `enrich.rs`'s `the_judgment_log_never_carries_a_notes_title_or_body_text` survives Task 4 unchanged; Task 2 adds the server-side twin |
| R-3a-25 | *"Does the failure text survive this boundary?"* is a standing question for every remaining task | plan 3a ledger | Task 4 answers it for the two boundaries a cloud judge adds — the **HTTP status** and the **auth failure**: `a_401_says_it_is_the_session_and_not_a_bad_answer` and `a_5xx_body_survives_to_stdout` |
| R-3a-12 | No engine command is ever run against the worktree root | plan 3a ledger | Global Constraints; every task's tests copy a fixture vault to a temp directory |
| CLAUDE.md | Every note write goes through `write` — journal first, single-line frontmatter surgery second; `judgment:` stays a single-line flow mapping; judge-once and `propose_amendment` intact | CLAUDE.md | Tasks 4, 7, 11, 12: every cloud reply becomes a `write::write_literals` or a `write::create` under an `agent:knowlu.<kind>` actor. No task writes a note by hand |
| CLAUDE.md | Approvals are capped at 15 new proposals a day; overflow is snoozed, never deleted | CLAUDE.md, `approvals::defer_over_budget` | Tasks 11 and 12 file cards through the ordinary proposal path, so the existing cap applies unchanged. The service's own daily caps are cost guards and a different thing |
| §8 | `dependency_boundary.rs` gains a pin that no cloud SDK enters the engine; HTTP is `ureq` only | cloud design §8 | Task 4 |
| §4.3 | The deterministic parsers move server-side; the two frozen references become the service's oracle | cloud design §4.3 | Tasks 5, 6, 7. **Decision (a): a faithful TypeScript port, not a WASM build of the Rust.** Reasoning in *File structure* |
| §3.1 | LMS `.ics`: the URL lives server-side encrypted; the fetch needs the internet and no credential | cloud design §3.1 | Task 8. **Narrowed: the fetch moves, the parse stays.** Reasoning and recommendation in *What is NOT in this plan* |
| C1 §3 | "When `/ingest-ics` ships, **C2** removes the vault copy of `ics_url`, not C1" | C1 plan, *Interfaces with C2* 3 | **Narrowed, on the record.** C2 does **not** remove it. Two reasons: `scaffold.rs` (which writes it) is C1's file, and — the substantive one — the vault copy is the offline fallback that keeps `ingest` exiting 0 when the service is unreachable (hand-off H3). Removing it would trade a working offline path for a tidier vault. **Who removes it and when:** C3, together with the journal sync that gives the device somewhere else to get the URL from, or C4 if the fallback proves dead weight. Recorded in Task 15's *what production still needs* so it is not lost |
| §5.2 | Per-account daily caps **"enrichment 200, events 300, email 500"**, a per-call timeout, and a monthly inference budget alert per account and global | cloud design §5.2 | Tasks 1, 2 and 14. **The three cap numbers are changed, on the record: 60 / 80 / 120.** The spec calls them "far above real use", and they are — but 1,000 calls a day at `claude-haiku-4-5`'s $1/$5 per MTok and this plan's own prompt bounds is about **$0.0014 × 1,000 = $1.40 a day, ~$45 a month, against a $9.99 subscription**. The arithmetic is written out beside `DAILY_CAP` in `judge_caps.ts` so the next reader can check it; the new numbers are ~4× a plausible heavy day (10 enrichments, 20 event verdicts, 30 emails ≈ $2.50/month) and the runaway ceiling is ~$11/month, which is why `MONTHLY_CEILING_USD = 7.5` exists and is **enforced** (`enforce_budget`, refusing the call) rather than reported by a view nobody queries. `usage_daily` records the real token counts (`record_tokens`), and `monthly_spend` reads `usage_daily` and `models` alone — an earlier draft joined `judgments` too and multiplied each day's usage by that day's judgment count |
| §3.1 | Event feeds: "fetch + roster" cloud, "HTML sources finally work — the server can run a headless fetch the desktop never could" | cloud design §3.1, §12's C2 row | Task 9. **Narrowed the same way and for the same reason: the fetch moves and the HTML sources work; the roster stays on the device.** `rank`'s existing `Fetchers.events` seam takes a cloud-backed fetcher, and `eventfeed`'s four parsers, `prefilter_events`, `write_roster` and `emit_digest` — all under the golden `today.md` oracle, none with a frozen parsed reference — are untouched. What actually moves is the **verdict**, which is the part D3 is about. Recorded here so the narrowing is a decision, not a silence |

---

## File structure

### The parser move — decision (a), a faithful TypeScript port

§4.3 moves the zyBooks and VHL parsers server-side. Two ways to do it, and the plan takes **(a) a faithful TypeScript port of `zybooks::parse_assignments` and `vhl::parse_dashboard`, gated by the two frozen parsed-reference fixtures read by relative path out of `engine/tests/fixtures/`.**

(b) — compiling the Rust parsers to WebAssembly for Deno — was considered and rejected on three grounds, in order of weight:

1. **It needs a crate split the rewrite spec forbids.** `engine/Cargo.toml` says so in a comment that is itself a decision: the engine is one crate because the module graph carries four true cycles — `ingest ↔ write`, `ids → ingest → write → ids`, `coursework ↔ zybooks`, `coursework ↔ vhl`. `zybooks::parse_assignments` returns a `coursework::Assignment` and calls `ingest::slugify`, `ingest::format_due` and `coursework::parse_duration_hours`; `vhl::parse_dashboard` calls the same. There is no "just the parsers" subset to compile: extracting one pulls `coursework`, `ingest`, `write`, `ids`, `models`, `yaml`, `pystr` and `ledger` behind it — which is the whole crate — and cutting the cycles to make a `knowlu-parsers` crate is exactly the refactor of the write path that the golden-file oracle exists to catch.
2. **The toolchain cost is real and recurring.** A wasm target on a `stable-x86_64-pc-windows-gnu` host means `wasm32-unknown-unknown` in CI (C0's stream), a `wasm-bindgen` glue layer, and `jiff` compiled for wasm with a bundled tz database — the zyBooks parser converts UTC stamps to `America/Chicago`, so the tzdb is not optional. Each is a thing to keep working on every upstream bump, for a payload of about six hundred lines of pure function.
3. **The stated benefit of moving the parsers is server-side *editability*** — "a vendor markup change becomes a server deploy, not an app release" (§4.3). A WASM blob compiled from `engine/src` is edited by editing `engine/src`, which is an engine change, which is an app release for the half that still fetches. (a) actually delivers the benefit; (b) delivers the boundary without it.

The risk (a) carries is divergence: two implementations of one contract. That risk is closed the same way the Rust port itself was — **the frozen Python-written references are the port's acceptance test.** `cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts` reads `engine/tests/fixtures/zybooks-assignments.json` and `engine/tests/fixtures/zybooks-parsed-reference.json` through `new URL("../../../../engine/tests/fixtures/…", import.meta.url)`; the VHL twin reads `vhl-dashboard.html` and `vhl-parsed-reference.json`. Neither file is copied into `cloud/`, neither is written, and the Rust byte-for-byte tests stay exactly where they are.

The one deliberate difference is that the TypeScript comparison is **structural** (`assertEquals(rows, JSON.parse(reference))`) rather than byte-for-byte: JavaScript has no `1.0` — `JSON.stringify(1.0)` is `"1"` — so a byte comparison would fail a *correct* port on the first whole-number `effort_hours`. Rows in the reference are arrays, so a structural comparison still pins every field **and its order**, which is the property the file was frozen for.

### New — the service

- `cloud/supabase/migrations/20260911000100_judgment_service.sql` — `models`, `judgments`, `rules`, `rule_evidence`, `corrections`, `usage_daily`; RLS on every one.
- `cloud/supabase/migrations/20260911000200_google.sql` — `google_accounts` (Vault-backed refresh token, with the granted `scopes`), `google_state`, `gmail_seen`, `gmail_queue`.
- `cloud/supabase/migrations/20260911000300_rule_promotion.sql` — `judgment_features()`, `backfill_correction_judgments()`, `promote_rules()` and their nightly `pg_cron` schedule.
- `cloud/supabase/migrations/20260911000400_eval.sql` — `eval_cases`, `eval_runs`.
- `cloud/supabase/functions/_shared/judge_anthropic.ts` — `JudgeModel` (the interface), `AnthropicModel` (the only file in the repository that imports `@anthropic-ai/sdk`), `ScriptedModel` (the test fake).
- `cloud/supabase/functions/_shared/judge_db.ts` — a forty-line PostgREST client over `fetch`, with the service role. The only database access in `cloud/`.
- `cloud/supabase/functions/_shared/judge_handler.ts` — `judgeHandler`, the one handler all three `/judge-*` functions are, and the `Entitle` type every other handler takes.
- `cloud/supabase/functions/_shared/judge_deps.ts` — the live wiring: `sharedDb()` and `liveDeps(kind, origin)`.
- `cloud/supabase/functions/_shared/judge_models.ts` — the pinned model row for a kind.
- `cloud/supabase/functions/_shared/judge_prompts.ts` — the three prompts, the three JSON schemas, their version strings and `promptHash()`.
- `cloud/supabase/functions/_shared/judge_rules.ts` — tier 2: the per-account rule lookup, and the feature extractor the promotion job shares.
- `cloud/supabase/functions/_shared/judge_validate.ts` — the confidence floor, the known-course check, the clamps.
- `cloud/supabase/functions/_shared/judge_caps.ts` — the per-account, per-kind daily cap.
- `cloud/supabase/functions/_shared/judge_log.ts` — one `judgments` row; the type has nowhere to put a body.
- `cloud/supabase/functions/_shared/judge_pipeline.ts` — the one pipeline all three kinds share.
- `cloud/supabase/functions/judge-task/index.ts`, `judge-event/index.ts`, `judge-email/index.ts` — thin handlers.
- `cloud/supabase/functions/ingest-coursework/{handler.ts,index.ts,pyshims.ts,parse_zybooks.ts,parse_vhl.ts}` — the moved parsers and the reconciler (which lives in `handler.ts`; there is no separate `reconcile.ts`).
- `cloud/supabase/functions/ingest-ics/{handler.ts,index.ts}` — the server-side fetch of the account's stored LMS URL.
- `cloud/supabase/functions/events/index.ts` — the server-side fetch of an events source, with the browser-shaped headers the desktop could not send.
- `cloud/supabase/functions/google-connect/{handler.ts,index.ts}`, `google-callback/{handler.ts,index.ts}` — **one** Google connect: `calendar.readonly` first (a *sensitive* scope), `gmail.readonly` incrementally (§11a).
- `cloud/supabase/functions/gmail-read/{handler.ts,index.ts}` — the twice-daily Gmail read.
- `cloud/supabase/functions/ingest-calendar/{handler.ts,index.ts}` — both calendar sources as ICS (§11a).
- `cloud/supabase/functions/judge-rules/{handler.ts,index.ts}` — the promoted-rule proposals and their decisions.
- `cloud/eval/` — `seed/` (Task 13's de-identified corpus), `deidentify.ts`, `scrub_test.ts`, `score.ts` (the pure scorer), `run_eval.ts`, `run_eval_test.ts`, `thresholds.json`.

Every `_shared/` file this plan creates is prefixed `judge_`, because `_shared/entitlement.ts` and anything else unprefixed belongs to C1. The parsers live inside `ingest-coursework/` rather than `_shared/` for the same reason: only that function needs them, and the directory is wholly C2's.

### New — the device

- `engine/src/cloudmodel.rs` — `CloudConfig`, `load`, `resolve`, `CloudError`, `CloudClient`, `CloudModel`, and the implementations of `judge::Model`, `judge::EventModel` and `judge::EmailModel`.
- `engine/tests/cloud_contract.rs` — the loopback contract tests for every request shape the engine sends, including the data-minimisation assertions.

### Modified — the device

- `engine/src/judge.rs` — `Missing::Service`, `Outcome::ServiceUnavailable`, and the two new traits `EventModel` / `EmailModel` beside `Model`. The three tiers, the floor, `tier1`, `merge`, `judge_task`, `GRAMMAR`, `prompt_for` and `parse_reply` are untouched.
- `engine/src/enrich.rs` — `run_lines` prefers the service when `config/cloud.yaml` exists; the pull phases (events, Gmail, rules) run after enrichment; `ACTOR` unchanged.
- `engine/src/events.rs` — `judge_roster`: the unjudged uids in `state/events.md` become `/judge-event` calls and `eventledger::record_verdict` lines.
- `engine/src/coursework.rs` — `fetch_zybooks_payload` / `fetch_vhl_payload` (the fetch half alone) and `collect_cloud`, which posts them and returns the reconciled `Assignment`s. `sync_coursework` unchanged.
- `engine/src/zybooks.rs` — **two changes only:** a doc line on `parse_assignments` naming the TypeScript port measured against it, and `pub fn fetch_payloads` exposing the fetch half. The parser body is not touched.
- `engine/src/vhl.rs` — **one change only:** a doc line on `parse_dashboard`. Its fetch half (`login_and_fetch_dashboard`) is already public.
- `engine/tests/dependency_boundary.rs` — a new test: no cloud SDK, no HTTP client but `ureq`.

### Owned but deliberately unchanged

`engine/src/judgelog.rs`. §5.4 measure 4 asks that every judgment row name the model, the prompt version, the grammar version and the prompt hash — and that row is the **server's** `judgments` table, written in Task 2. The device log's job (§5.6) is unchanged and narrower: *why is nothing being enriched*, by id and field value. Duplicating the server's provenance into a per-profile file that never syncs would add a second place to keep true and no reader. `Entry` therefore gains nothing, which also keeps ruling R-3a-20's property intact: every field in that type is either an id, a number, or a `&'static str` from a closed enum.

---

## Interfaces with C1

**Six contracts, checked against `docs/plans/2026-09-09-c1-accounts-plan.md` on 2026-09-09.** Use these spellings verbatim; changing one is a conversation, not an edit.

**1. `config/cloud.yaml`** — written into the vault by C1's `scaffold::create_vault` at onboarding, absent on a vault that has never signed in. Exactly four keys, in this order:

```yaml
api_base: 'https://<ref>.supabase.co/functions/v1'
anon_key: '<the project anon key — public>'
session_credential_target: 'knowlu/<profile_id>/session'
account_id: '<uuid>'
```

C1 writes it through `pystr::write_text`, i.e. **whatever line endings every other vault file gets** (today CRLF, because vaults are). The "LF" in this repo's rules is about the *repository*; this file is a vault file. **C2 reads it through `pystr::read_text` + `serde_yaml_ng` and never compares raw bytes** — `cloudmodel::load` does exactly that. An absent or partial file is `None`, which is a named skip on stdout and exit 0.

**2. The session credential is a JSON object, not a bare JWT.** Windows Credential Manager, target `knowlu/<profile_id>/session`. `UserName` is the `account_id`; the **blob is one JSON object**:

```json
{"access_token": "<jwt>", "refresh_token": "<jwt>", "expires_at": 1789543210, "email": "…"}
```

`expires_at` is Unix seconds. **Refresh is C1's job** (`account::valid_access_token()` refreshes at fewer than 120 seconds remaining and rewrites the entry). **C2 only reads it**, through `wincred::read_credential`, takes `access_token`, and **tolerates an expired one** by reporting `no session` and waiting for the app's next slot rather than refreshing itself — `cloudmodel::session_token` therefore parses the blob and never treats the password field as a token.

**3. Calendars and feeds — C1's `sources` table, and the one calendar that is not in it.** C1's schema is the authority:

```
public.sources(account_id uuid, kind text, url_ciphertext text, url_iv text, added_at timestamptz)
primary key (account_id, kind)
check (kind in ('lms_ics', 'calendar_ics', 'google_calendar'))
```

There is **no column called `url`** and **no column called `name`**: the primary key is `(account_id, kind)`, so a row is found by its kind and there is at most one of each. The URL is **AES-256-GCM ciphertext plus a 12-byte IV**, encrypted by C1's account function before it reaches Postgres, with the key in the function environment as `SOURCES_ENC_KEY` (precondition P0a, C1's to set) and never in the database. `GET /account/sources` returns `[{"kind","added_at"}]` and never the URL. C2 reads a row **with the service role** and decrypts it in the function with C1's own `_shared/crypto.ts`:

```ts
export async function importAesKey(base64Key: string): Promise<CryptoKey>;
export async function decryptString(key: CryptoKey, ciphertext: string, iv: string): Promise<string>;
```

| `sources.kind` | Written by | Read by | Where it also lives on the device |
|---|---|---|---|
| `lms_ics` | **C1** — `lms_link::finish` when the school feed validates, and its back-fill for a vault adopted in place | **C2** — `/ingest-ics` | `config/ingest.yaml`'s `ics_url:` |
| `calendar_ics` | **C1** — the same panel's personal-calendar field, by its secret iCal address | **C2** — `/ingest-calendar?name=personal` | `config/ingest.yaml`'s `calendars:` list, as `- name: personal` / `ics_url:` |
| `google_calendar` | **nobody** — see below | — | — |

**`google_calendar` is a reserved value that C2 never writes** (ruling R-X-9). It is in C1's check constraint and in both `SOURCE_KINDS` lists so that neither stream ever has to edit the other's constraint, and that is the whole of its job. A Google grant has **no URL to encrypt**, so it could not fill `url_ciphertext` and `url_iv`, both of which are `not null`. **The grant lives in C2's own `google_accounts`** (Task 10) — the Vault secret's id, the Google `sub`, and the `scopes` that were actually granted — and that table is where `/ingest-calendar` looks. C1's device-side guard `lms_link::DEVICE_KINDS = ["lms_ics", "calendar_ics"]` with `validate_for(kind, url, fetch)` means no page can invent a `google_calendar` row either.

**`/ingest-calendar?name=` therefore resolves a *name*, not a row key, and the mapping is stated in the handler** — one constant, `CALENDAR_NAMES`, with `personalSource` behind the one name that is a `sources` row, and nothing else in C2 decides it:

| `?name=` | Resolves to |
|---|---|
| `personal` (the default) | the account's `sources` row where `kind = 'calendar_ics'`, decrypted and fetched server-side |
| `google` | the account's `google_accounts` row, if its `scopes` contain `https://www.googleapis.com/auth/calendar.readonly` |
| anything else | a 404 that names the name |

The two names are the two `calendars:` entries the vault carries — `- name: personal` (C1 writes it, from the secret address) and `- name: google` with `ics_url: 'cloud:google'`. Hand-off **H9** puts the second one there, and **not by writing `config/ingest.yaml` when the button is pressed**: the wizard has no vault on panel 5 (C1 creates it at Finish), so the panel sets a `WizardPlan` flag and `scaffold::ingest_yaml` writes both entries when the vault is born — exactly the route C1 already uses for the personal one. After onboarding, `set_google_calendar(vault, connected)` adds or removes the entry over a vault that exists, and its settings row is C4's. `calfeed` walks that list and hands each `ics_url` to the `Fetchers.calendar` seam; hand-off **H4** routes a `cloud:<name>` url to `/ingest-calendar?name=<name>` and every other url to `calfeed::fetch_ics`. So the name in the query string is the name in the vault, and nothing has to agree about anything else.

**One deliberate deviation from §11a's wording**, recorded here rather than left implicit: the ruling says the device pulls the calendar "in the same slot step that pulls `/ingest-ics`", i.e. `ingest`. It is pulled in **`rank`'s calendar pass** instead, because that is where `calfeed`'s snapshot fallback, its 28-day horizon and its dedup live — pulling it in `ingest` would mean a second calendar parser and a `state/calendar.md` that `rank` overwrites seconds later. It is still a device pull inside a slot, still never a push, and still transport rather than judgment (`rank_cannot_reach_a_judgment_endpoint` covers it).

**The vault copies stay.** C1 keeps writing `ics_url:` and the `- name: personal` entry, and C2 keeps both as the offline fallback rather than removing them — see the ledger row for who removes them and when. C1's interface says "when they ship, C2 removes the vault copies"; C2 does not, and says why.

**4. Telemetry, and who owns `corrections`.** (a) interaction events and (b) corrections go to **C1's `POST /telemetry`**, derived from the journal **on the device** by C1's `telemetry.rs`. C2 never posts telemetry. **C1 creates the table**, in `20260910000400_telemetry.sql`, with C1's columns:

```sql
public.corrections (
  id          bigint generated always as identity primary key,
  account_id  uuid not null references public.accounts (id) on delete cascade,
  ts          timestamptz not null,
  item_id     text not null,
  field       text not null,
  ours        text,                -- NULLABLE, and must stay so: a `course` correction carries neither
  theirs      text,                -- NULLABLE, same reason (§6's content rule)
  kind        text not null,       -- the NOTE kind (task|approval|course|info|issue|archive), no check
  request     jsonb,               -- C1 defines it; always null unless the (c) opt-in is on
  received_at timestamptz not null default now(),
  constraint corrections_once unique (account_id, ts, item_id, field)
);
```

**C2 never creates this table** (ruling R-X-2). In its own `20260911…` migration C2 adds exactly two columns and one index, and constrains nothing C1 owns:

```sql
alter table public.corrections add column if not exists judgment_id uuid null;
alter table public.corrections add column if not exists judgment_kind text null
  check (judgment_kind in ('task', 'event', 'email'));
create index if not exists corrections_judgment_kind_ts on public.corrections (judgment_kind, ts desc);
```

- **`judgment_id` and `judgment_kind` are populated by C2's own back-fill, never by C1** (ruling R-X-11). `backfill_correction_judgments()` (Task 12's migration, run by Task 12's nightly job and again by Task 14 before every eval) joins each correction to the most recent `judgments` row for the same `(account_id, item_id)` strictly before `corrections.ts`, and takes **both** the id and the judgment kind from it. C1 sends neither and needs to know nothing about them. Two things follow: the device stores no judgment id (the alternative was writing it into every judged note's `judgment:` provenance line, which is a contract with existing vaults), and **`promote_rules` runs the back-fill's output rather than a column it hopes is filled** — it joins `corrections c` to `judgments j` on `j.id = c.judgment_id`, so a correction that has not been back-filled simply does not yet block a promotion.
- **`request`** is the re-derived judge request — the exact `{kind, item, heuristics_seed}` body the device would send today for that item — and **C1 defines the column and writes `null` into it unless the request also carries `opt_in_raw: true`**, i.e. unless the account has telemetry class (c) switched on (§6: raw content, separate opt-in, default off, its own screen, revocable). It is **never** sent for a correction whose judgment had `origin = 'gmail_api'`, whatever the toggle says, because Google's Workspace API User Data policy forbids it outright (§5.3, §9). This is ruling R-C2-4: without it no correction can become a replayable eval case, because §6 (b) carries no body and §5.2 discards bodies after the call. **The (c) toggle's UI is C4, so the eval suite is seed-only until then.**
- **There is no `features` column and C2 does not ask for one** (ruling R-X-11). Rule promotion reads its four keys from `judgment_features` over `judgments.fields`, which is where `featureMap` puts them — the correction does not have to carry them.


**4a. The coursework mapping — C1 writes the first one, C2 writes the rest, and both write the same file** (§11a, R-OB-1). The mapping from a discovered zyBook code or VHL section id to a course slug lives in the **vault's `config/ingest.yaml`**, under `coursework.zybooks.courses.<code>` and `coursework.vhl.sections.<id>`, each `{course, label}`:

```yaml
coursework:
  zybooks:
    enabled: true
    credential_target: 'knowlu/<profile_id>/zybooks'
    courses:
      UACS100Fall2026: { course: cs-100, label: CS 100 }
  vhl:
    enabled: true
    credential_target: 'knowlu/<profile_id>/vhl'
    sections:
      '2102121': { course: gn-103, label: GN 103 Hausaufgaben }
```

- **The two writers derive `label:` differently, and that is a decision, not a slip.** C1's wizard carries the label the student typed or the discovery found, so a VHL section can be `GN 103 Hausaufgaben` as in the example above. A C2 card derives it mechanically from the confirmed course slug (`gn-103` → `GN 103`), because one field on a card is one decision and this one is spelling. The label prefixes every title the parser produces, so the same section mapped by the two paths yields two title prefixes — both correct, one longer. A student who wants the longer one edits the file, which is the point of it being text.
- **C1** writes the initial mapping at onboarding: after the first coursework fetch the wizard shows the discovered books and sections with a suggested course and the student confirms them (R-OB-1's other half). C1 owns that panel and `scaffold`.
- **C2** writes every later one, and only through a card the student approved: `/ingest-coursework` returns a `proposals` array for anything the mapping does not know, the engine files a `kind: coursework-map` approval, and `coursework::apply_map_cards` writes the confirmed mapping into the same file before the next fetch.
- **It is not a `sources` row and there is no server-side copy.** §4.3 is the reason — *the parsers move; the credentials and the vault stay* — and the mapping is already a request field the device sends (redacted) on every call. A second home would make two writers of one value that the student can also edit by hand, which is VISION's *every automation is editable text*.
- **The write is a text-level insertion, and it may only ever ADD a key** (ruling **R-C2-8**). CLAUDE.md's invariant has no config exemption — *no vault file is parsed and re-dumped*, and `src/yamlemit.rs` is the one YAML emitter — so `apply_map_cards` does **not** round-trip the file through `serde_yaml_ng::to_string`. `write_mapping` finds the `coursework.zybooks.courses:` (or `coursework.vhl.sections:`) line, inserts the new key's three lines directly beneath it at the file's own indentation, and touches no other byte; the single exception is a `courses: {}` / `sections: {}` scalar, replaced in place by the block form the insertion needs. C1's `scaffold::ingest_yaml` writing this file at birth is **creation**, which is a different act. **"It cannot update an existing key" costs nothing**: a book or section that is already mapped never yields a card — `route_zybook` answers `Mapped`, `parse_vhl` finds its section, and `/ingest-coursework` reports no proposal — so no card ever asks for a key that is already there. A second apply of the same key is a logged no-op that does not open the file for writing; a student who wants to change a mapping edits the file, which is the point of it being text. The **card** is a note and is journalled through `write` like everything else, which is where the audit trail lives.

**5. `cloud/supabase/functions/_shared/entitlement.ts`** — C1's, imported by every C2 function and called first:

```ts
export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }>;
```

It **throws a `Response`** — 401 on a missing or invalid bearer, 402 when the account has no `active` or `trialing` entitlement. Every C2 handler wraps it as `try { … } catch (e) { if (e instanceof Response) return e; throw e; }` and never re-implements it.

**6. The scheduler runs `judge` on a cloud vault — and that is C1's change, not a C2 hand-off** (rulings R-C2-3 and R-X-4/R-X-12). C1's Task on `app/src/scheduler.rs` adds, **verbatim**:

```rust
pub enum JudgePlan { Cloud { log_dir: PathBuf }, Local(JudgeArgs), Skip(&'static str) }
pub fn entitlement_state(cs: &ConsoleState) -> crate::account::EntitlementState;
pub fn judge_plan(cs: &ConsoleState) -> JudgePlan;
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)>;
```

**Entitlement outranks everything**: past the 72-hour grace the step is not run at all and the slot records `judge (skipped: no entitlement)` — a named step with **exit code 0**, never a failure. A vault carrying `config/cloud.yaml` runs `JudgePlan::Cloud`; only a vault with no `config/cloud.yaml` falls through to the local runtime, and that arm leaves in C4. **`JudgePlan::Cloud`'s argv is exactly**

```
knowlu-engine judge --vault <vault> --via local-runner --log-dir <the profile's logs folder>
```

and **nothing else** — no `--runtime`, no `--model`, no account id, no token. **`--log-dir` is in *both* arms**, because judgment logs never enter the vault (CLAUDE.md) and only the app knows the profile's folder: it is `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments`, the same directory `inference::judgments_dir(&cs.data_dir)` already returns. Three consequences C2 must hold:

- **There *is* a device judgment log on the cloud path**, and every C2 test that runs a slot against a cloud vault will see one written. `judgelog::record` is called exactly as it is on the local path, with the same `Entry` and the same closed vocabulary.
- **`judgelog.rs` still needs no change, but not for the reason an earlier draft gave.** The reason is that the provenance §5.4 measure 4 asks for — model id, prompt version, grammar version, prompt hash — lives in the **server's** `judgments` table, where it can be queried across accounts and where the eval reads it; the device log's job is the narrower one it already had (§5.6): *why is nothing being enriched*, by id and field value. Adding the server's four fields to a per-profile file that never syncs would be a second place to keep true with no reader. `Entry` therefore gains nothing, which also keeps ruling R-3a-20's property intact: every field in that type is an id, a number, or a `&'static str` from a closed enum.
- **C2 never edits `app/src/scheduler.rs`.** The one change C2 needs there — running `ingest` on a cloud vault whose `ics_url` is empty — is a separate one-line hand-off applied **at Task 8's merge, after C1**, against C1's merged shape and not a replacement of it. It is hand-off **H5** below.

**7. `config.toml`, `deno.json` and the one `cloud` CI job are C1's**, and C2 supplies its lines as hand-offs **H7** and **H8**, applied at C2's merge. Eleven `[functions.<name>]` entries and one `deno.json` import-map line; C1 leaves both to C2 and does not guess at them.

## Controller hand-offs

Every change below is outside C2's file ownership. **No task in this plan edits these files.** Each task that needs one names it in its report; the controller applies them on `main` at merge time, in this order. **`app/src/scheduler.rs`'s `judge` gate is deliberately not here** — it is C1's, and it is *Interfaces with C1* contract 6 (ruling R-C2-3).

### H1 — `engine/src/lib.rs` (needed by Task 4)

One `pub mod` line, after the `judge` / `runtime` / `judgelog` block:

```rust
// Knowlu C2 — the judgment service as seen from the device (cloud design §3.2, §5.2): one HTTPS
// call per item behind `judge::Model`, `judge::EventModel` and `judge::EmailModel`, with the
// account's session token out of Credential Manager. `rank` never reaches it (decision 11).
pub mod cloudmodel;
```

### H2 — `engine/Cargo.toml` (needed by Task 4)

**None, deliberately.** `cloudmodel.rs` uses `ureq` 3.4 (already a dependency, already `cookies` + rustls), `serde_json`, `serde_yaml_ng` and `crate::wincred`. If a later task believes it needs a dependency, that is a plan defect: say so in the report instead of adding one.

### H3 — `engine/src/ingest.rs` (needed by Task 8)

**Two changes, and the placement of the second is the whole point.** `run_lines` must try the service *before* it decides the vault has no feed — otherwise a cloud vault whose `ics_url` is empty (C1 writes it, but a link-capture failure or a hand-edited vault can leave it blank) hits the early return, exits **1**, sets `RunSummary.engine_ok = false`, and paints the tray amber twice a day forever. That is the exact failure this plan's Global Constraints forbid.

**Change 1** — at `engine/src/ingest.rs:741-742`, keep the first line and **delete the early return**:

```rust
    let url = config.get("ics_url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
```

**Change 2** — replace the single `let fetched = match fetch { … };` line (`engine/src/ingest.rs:748`) with the block below. The empty-URL refusal now lives *inside* it, reached only when there was no way at all to get a feed:

```rust
    // C2 (cloud design §3.1): the LMS capability URL lives in the account, encrypted, not in the
    // vault — so when this vault has an account the feed is fetched by `/ingest-ics` with the
    // service role and the URL never leaves the server. `config/ingest.yaml`'s `ics_url` stays the
    // fallback for a vault with no account and for a service that is unreachable: dead hotel
    // Wi-Fi must not stop an ingest that could have run from the URL already on disk.
    //
    // **The empty-URL refusal moved here on purpose.** It used to sit six lines above, before any
    // cloud attempt; a cloud vault with a blank `ics_url` would then exit 1 without ever asking
    // the service, and a non-zero step is retry backoff and an amber tray twice a day forever.
    // Exit 1 now means what it says: there was no way at all to get a feed.
    let cloud = crate::cloudmodel::resolve(vault).ok();
    // R-OB-3 (hand-off H11), by R-C2-9's predicate: this vault has never been through a whole slot
    // if `rank` has never written `today.md`. NOT the seen-ledger — `coursework` runs before
    // `ingest` in the same slot and calls `record_seen`, so that file exists on the very first run.
    let first_run = crate::ingest::is_first_run(vault);
    let fetched = match (fetch, &cloud) {
        (Some(f), _) => {
            if url.is_empty() { return (1, vec!["ingest: no ics_url configured".to_string()]); }
            f(&url)
        }
        // `_past` is bound and unused on purpose: the device archives on its own comparison, which
        // is the guarantee (it knows the vault's timezone), and the service's list is corroboration
        // and what the wizard counts. Binding it here is what makes the next reader ask which half
        // is authoritative; the answer is in `fetch_ics`'s doc comment.
        (None, Some(client)) => match crate::cloudmodel::fetch_ics(client, first_run) {
            Ok((text, _past)) => Ok(text),
            Err(e) if url.is_empty() => {
                return (1, vec![format!("ingest: no feed — the service is unavailable ({e}) and no ics_url is configured")]);
            }
            Err(e) => {
                println!("ingest: /ingest-ics unavailable ({e}); using the vault's ics_url");
                crate::calfeed::fetch_ics(&url)
            }
        },
        (None, None) => {
            if url.is_empty() { return (1, vec!["ingest: no ics_url configured".to_string()]); }
            crate::calfeed::fetch_ics(&url)
        }
    };
```

The existing test `ingest_main_returns_1_when_no_ics_url_is_configured` still passes: its vault has no `config/cloud.yaml`, so it takes the `(None, None)` arm and gets the identical line and the identical code. Task 8 adds `a_cloud_vault_with_no_ics_url_still_ingests` beside it.

### H4 — `engine/src/cli.rs` (needed by Task 9)

`rank`'s events pass fetches each configured source through the service when the vault has one, so the HTML sources a desktop `ureq` could never get past finally return a page. This is a **fetch proxy, not a judgment** — `rank` still never calls a model, and Task 9 adds `rank_cannot_reach_a_judgment_endpoint` to prove that structurally rather than by convention. Replace `cli::run`'s body:

```rust
pub fn run(
    vault: &Path,
    today_iso: Option<&str>,
    runner: &str,
    run_id: Option<&str>,
) -> Result<RunOutcome, RunError> {
    // C2 (cloud design §3.1): event feeds are fetched by the service, which can present a real
    // browser's headers and follow a redirect chain the desktop could not. A transport swap and
    // nothing more — the parsers, the pre-filter, the roster and the digest are the same
    // deterministic code, and `rank` still never calls a model (decision 11). A vault with no
    // account, or a service that is down, falls straight back to `eventfeed::fetch_event_source`.
    let cloud = crate::cloudmodel::resolve(vault).ok();
    let events = |url: &str| -> Result<String, String> {
        match &cloud {
            Some(client) => crate::cloudmodel::fetch_event_source(client, url)
                .or_else(|_| crate::eventfeed::fetch_event_source(url)),
            None => crate::eventfeed::fetch_event_source(url),
        }
    };
    // C2 Task 8 (cloud design §11a): a `calendars:` entry whose `ics_url` is `cloud:<name>` is
    // served by `/ingest-calendar` — the Google grant's events, or the account's stored secret
    // iCal address, rendered as ICS. Every other url is fetched on the device exactly as before,
    // so the `calendar_ics` secret-address path keeps working with no account at all. `calfeed`
    // then parses, bounds to its 28-day horizon, dedups and snapshots it like any other feed:
    // there is no second parser and no new vault file.
    let calendar = |url: &str| -> Result<String, String> {
        match (url.strip_prefix("cloud:"), &cloud) {
            (Some(name), Some(client)) => crate::cloudmodel::fetch_calendar(client, name),
            // A `cloud:` feed on a vault with no account is not an error worth failing a run for:
            // `load_calendar_events` turns this into "using snapshot" and the day still ranks.
            (Some(_), None) => Err("no account on this vault".to_string()),
            (None, _) => crate::calfeed::fetch_ics(url),
        }
    };
    run_with(
        vault,
        today_iso,
        runner,
        run_id,
        Fetchers { calendar: Some(&calendar), events: Some(&events) },
    )
}
```

**`cli::run` is the only production caller of `run_with`** (`engine/src/main.rs`'s `Command::Rank` arm), so after this hand-off every production fetch of a `cloud:` feed goes through the proxy above. `run_with(…, Fetchers::default())` is a **test seam only**: today it is called solely by `cli.rs`'s own tests, and there a `cloud:` url would reach `calfeed::fetch_ics`, fail as a malformed URL, and become one "using snapshot" warning — no panic and no egress. Say so here so the next person to add a caller knows what the seam is load-bearing for.

### H5 — `app/src/scheduler.rs`, one condition (needed by Task 8, applied **at Task 8's merge, after C1**)

C1's `slot_argv` keeps `if has_ics_url(vault)`. Once `/ingest-ics` exists, a cloud vault must run `ingest` even with a blank `ics_url`, because the URL now lives in the account. One condition, and H3 above is what makes it safe:

```rust
    // C2 Task 8: the LMS capability URL lives in the account from here on, so a cloud vault runs
    // `ingest` whether or not the vault still carries a copy. Safe only with C2's H3 applied —
    // without it, a blank `ics_url` exits 1 before the service is ever asked.
    if has_ics_url(vault) || vault.join("config").join("cloud.yaml").is_file() {
```

C1's own scheduler test `the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured` keeps passing (its vault has no `config/cloud.yaml`); the controller adds one case beside it, `a_cloud_vault_runs_ingest_with_no_ics_url`, with this hand-off.

### H6 — `engine/src/approvals.rs` (needed by Task 12)

One arm in `process_approvals`' `status == "approved"` chain, inserted **between** the `kind == "events-digest"` arm and the final `else`. The chain at `engine/src/approvals.rs:1341-1395` reads `if kind == "calendar-event" { … } else if kind == "task" { … } else if kind == "amend" { … } else if kind == "events-digest" { … } else { … }`, so this is a complete `else if` block with its own braces — the closing `} else {` line below is the existing final `else`, shown so the insertion point is unambiguous:

```rust
        } else if kind == "rule" {
            // C2 (cloud design §5.4): a promoted rule. `rank` never opens a socket for a judgment,
            // so the decision is *sent* by the next `judge` step, which then stamps this card and
            // archives it. Left exactly as it is here — deliberately not an `unknown kind` warning,
            // because a WARN line on every run until the next slot would train the reader to ignore
            // the list, which is the one thing `rank_warnings` exists to prevent.
        } else {
```

A **rejected** rule card needs no round trip: `process_approvals` archives it as it archives any rejection, and the server expires an undecided proposal after 30 days (Task 12). A re-promotion is a new proposal.

### H7 — `cloud/supabase/config.toml` and `cloud/supabase/deno.json` (C1's files; applied at C2's merge — ruling R-X-6)

**`config.toml`** — **eleven** `[functions.<name>]` entries appended to C1's eight, in this order. `verify_jwt = false` for the same reason C1 gives: each handler calls C1's `requireActiveEntitlement` first and answers 401 or 402 **in our shape**, which is the shape the engine's `CloudError::label()` maps to `no session` / `no entitlement`; Supabase's gateway rejection is a different body with a different status and would make that contract untestable.

```toml
[functions.judge-task]
verify_jwt = false
[functions.judge-event]
verify_jwt = false
[functions.judge-email]
verify_jwt = false
[functions.judge-rules]
verify_jwt = false
[functions.ingest-coursework]
verify_jwt = false
[functions.ingest-ics]
verify_jwt = false
[functions.ingest-calendar]
verify_jwt = false
[functions.events]
verify_jwt = false
[functions.google-connect]
verify_jwt = false
[functions.google-callback]
verify_jwt = false
[functions.gmail-read]
verify_jwt = false
```

`google-callback` is the one that is not a bearer endpoint at all: Google redirects a browser to it, and it is authenticated by the single-use `state` nonce it consumes (Task 10). `verify_jwt = true` there would make the OAuth round trip impossible.

**The names are `google-connect` / `google-callback`, not `gmail-*`** (§11a): one grant now carries `calendar.readonly`, `gmail.readonly`, or both, and a function called `gmail-connect` that asks for a calendar scope would be the first thing to mislead the next reader. `gmail-read` keeps its name — it reads Gmail and nothing else.

**`deno.json`** — **one** line added to C1's `imports` map, and nothing else:

```json
    "@anthropic-ai/sdk": "npm:@anthropic-ai/sdk@<pinned in Task 0 step 4>"
```

That is the whole of C2's runtime dependency surface. C1's note — *"no runtime dependency exists at all; every handler talks to Postgres and to Auth over `fetch` against the REST and Auth APIs, so `supabase-js` never enters this codebase"* — is adopted by C2 verbatim: `_shared/judge_db.ts` is a forty-line PostgREST client over `fetch`, so the only third-party import in `cloud/` is the inference SDK, in the one file that is allowed to have it.

### H8 — `.github/workflows/ci.yml` (needed by Tasks 5, 6, 14 — `.github/**` is C0's; ruling R-X-7)

**Two changes, not two new jobs.** C1 defines the single `cloud` job; C2 **amends** it (three paths and one permission) and adds `eval-gate` beside it. The controller merges both plans' hand-offs into one `ci.yml`.

**Amendment to C1's `cloud` job** — replace its three `run:` lines:

```yaml
      # `cloud/eval/` is C2's and sits outside `cloud/supabase/`, so every path is named explicitly.
      - name: deno check
        run: deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts cloud/eval/*.ts
      - name: deno lint
        run: deno lint --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
      # --allow-net=127.0.0.1 is needed from C2 Task 0 on, and only from then: `judge_anthropic_test.ts`
      # exercises the model client against a listener bound to 127.0.0.1:0 inside the test. No egress,
      # no name resolution, no listener on a routable interface (CLAUDE.md; plan 3a's loopback rule).
      - name: deno test
        run: deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
```

**And a second job:**

```yaml
  eval-gate:
    runs-on: ubuntu-latest
    needs: cloud
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@<sha pinned by C0>
        with: { fetch-depth: 0 }
      - uses: denoland/setup-deno@<sha>
        with: { deno-version: v2.x }
      - name: does this PR touch a prompt, a schema, or the model pins?
        id: touched
        shell: bash
        run: |
          files=$(git diff --name-only origin/${{ github.base_ref }}...HEAD)
          run=no
          # The three files that define what is asked and how the answer is checked...
          if echo "$files" | grep -Eq '^cloud/supabase/functions/_shared/judge_(prompts|models|validate)\.ts$'; then run=yes; fi
          # ...and ANY migration that mentions the `models` table. A pin change is "a migration row
          # with a date" (§5.2) — `20260912000100_pin_sonnet.sql`, say — so a filename pattern is
          # exactly the wrong test: it would miss the model swap the gate exists for.
          for f in $(echo "$files" | grep -E '^cloud/supabase/migrations/.*\.sql$' || true); do
            if [ -f "$f" ] && grep -qiE '\bmodels\b' "$f"; then run=yes; fi
          done
          echo "run=$run" >> "$GITHUB_OUTPUT"
      - name: eval suite
        if: steps.touched.outputs.run == 'yes'
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          SUPABASE_URL: ${{ secrets.SUPABASE_STAGING_URL }}
          SUPABASE_SERVICE_ROLE_KEY: ${{ secrets.SUPABASE_STAGING_SERVICE_ROLE_KEY }}
        run: deno run --allow-read --allow-env --allow-net --config cloud/supabase/deno.json cloud/eval/run_eval.ts --thresholds cloud/eval/thresholds.json
```

The three secrets are set by Quinn with `gh secret set NAME --repo quinnhall07/knowlu` from a file or a piped value he produces; this session never sees a value. `eval-gate` is §5.4 measure 2's deploy blocker: it fails the PR, and C0's branch protection is what stops the merge. Pin `denoland/setup-deno` by full commit SHA, as `engine/tests/workflows.rs::every_action_is_pinned_to_a_full_commit_sha` requires. Do **not** add `eval-gate` to branch protection in the commit that creates it — add it once it has run green on one PR, exactly as C1 says for `cloud`.

### H9 — the Google button, end to end, in two phases (needed by Task 10; §11a, rulings R-X-10, R-X-14, R-X-15)

**Eight files, and they do not share an owner.** Name each when the hand-off is passed:

| File | Owner | What H9 does to it |
|---|---|---|
| `app/src/account.rs` | C1 | adds three commands: two vault-less (wizard) and one with `ConsoleState` (console) |
| `app/src/onboarding.rs` | C1 | one `WizardPlan` field, and one line passing it into `VaultPlan` |
| `app/src/scaffold.rs` | C1 | one `VaultPlan` field, and the `calendars:` writer gains a second entry |
| `app/src/main.rs` | **controller** | registers two commands on the **wizard** list and one on the **console** list |
| `app/static/index.html` | C1 | removes `disabled` and the "next release" span from C1's existing `#wiz-google` |
| `app/static/console.js` | C1 | one listener, one poll, one plan field |
| `app/tests/static_assets.rs` | C1 | the placeholder assertion becomes a live-button assertion |
| `scripts/wizard-check.py` | **controller** | the headless walk's `#wiz-google[disabled]` becomes `:not([disabled])` |

**Two phases, because C1 has two windows and two moments** (ruling R-X-15). C1 is explicit: the two `generate_handler!` lists are **different windows**, and on the vault-less shell "there is no `ConsoleState` yet, so no command that needs one can be called". `#wiz-google` is on the **wizard** window. And C1 creates the vault at **Finish**, not on panel 5 — which is why it carries the personal calendar as a `WizardPlan` field rather than writing `config/ingest.yaml` when the field is filled. The Google entry travels the same way.

C1 deliberately builds no Google code at all, and its static test asserts `console.js` contains neither `"connect_google"` nor `gmail.readonly`. **Both stay true**: the commands are `google_connect_url` / `google_connected`, and the scope string never appears on the device — the service decides what `scope: "calendar"` means.

---

#### Phase (a) — the wizard: two vault-less commands, a flag on the plan, and a vault born with the entry

**(a1) `app/src/account.rs` — two commands that take no `ConsoleState`**, built on C1's own vault-less helpers (`api_base()`, `anon_key()`, `auth_base()`, `load_session()`, `valid_access_token_at()`, `PENDING_TARGET`, `open_in_browser()`) exactly as C1's eight wizard commands are:

```rust
/// The Google consent URL for the account this wizard signed in as (§11a).
///
/// **Vault-less on purpose.** `#wiz-google` is on the wizard window, which has no `ConsoleState`
/// (C1: "no command that needs one can be called" there), so the session comes from
/// `PENDING_TARGET` — the same pre-vault target `sign_in` writes and `create_vault_in` later moves
/// onto the profile — and the base URLs come from `api_base()` / `anon_key()`, never from a
/// `config/cloud.yaml` that does not exist yet.
///
/// `scope` is `"calendar"` (the default, and the wizard's) or `"gmail"` (the later, incremental
/// ask). The service decides which Google scope each means and whether the consent widens an
/// existing grant; this command carries the session bearer and nothing else, and **never sees a
/// Google token** — the exchange happens server-side in `google-callback` (D12).
#[tauri::command(async)]
pub fn google_connect_url(scope: String) -> Value {
    let scope = if scope == "gmail" { "gmail" } else { "calendar" };
    let api = api_base();
    let auth = match auth_base(&api) {
        Ok(auth) => auth,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let token = match valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, jiff::Timestamp::now().as_second()) {
        Ok(token) => token,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    match get_json(&format!("{api}/google-connect?scope={scope}"), &token) {
        Ok(v) => match v.get("url").and_then(Value::as_str) {
            Some(url) => json!({ "ok": true, "url": url }),
            None => json!({ "ok": false, "error": "the service returned no url" }),
        },
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Has the consent landed yet, and what did Google actually grant?
///
/// The calendar panel polls this after opening the consent page — the browser window closes itself
/// and there is nothing else to tell the wizard the round trip finished. `GET /google-connect?status=1`
/// answers `{connected, scopes}` from `google_accounts`, and the panel keys on the calendar scope
/// specifically, because a student can untick one on the consent screen.
#[tauri::command(async)]
pub fn google_connected() -> Value {
    let api = api_base();
    let auth = match auth_base(&api) {
        Ok(auth) => auth,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let token = match valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, jiff::Timestamp::now().as_second()) {
        Ok(token) => token,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    match get_json(&format!("{api}/google-connect?status=1"), &token) {
        Ok(v) => {
            let scopes: Vec<String> = v
                .get("scopes")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            json!({
                "ok": true,
                "connected": v.get("connected").and_then(Value::as_bool).unwrap_or(false),
                "calendar": scopes.iter().any(|s| s == "https://www.googleapis.com/auth/calendar.readonly"),
                "gmail": scopes.iter().any(|s| s == "https://www.googleapis.com/auth/gmail.readonly"),
            })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// One bearer GET against the functions base. `check_api_base` is applied first, so an
/// `KNOWLU_API_BASE` pointing anywhere but https (or loopback, for the tests) is refused here
/// rather than turned into a request to a host nobody chose.
fn get_json(url: &str, token: &str) -> Result<Value, String> {
    check_api_base(url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("apikey", anon_key())
        .call()
        .map_err(|e| format!("no network ({e})"))?;
    let code = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    if !(200..300).contains(&code) {
        // The status, not the body: this string reaches the wizard's error line.
        return Err(format!("the service refused (HTTP {code})"));
    }
    serde_json::from_str(&body).map_err(|_| "the service returned no JSON".to_string())
}
```

**(a2) `app/src/onboarding.rs` — one `WizardPlan` field and one line.** The page sets it; nothing writes a file:

```rust
pub struct WizardPlan {
    // …existing fields…
    /// Did the student connect a Google calendar on panel 5 (§11a)? A flag, not a URL: the grant
    /// lives on the account and the vault only needs to know the feed exists. `#[serde(default)]`
    /// so a page that predates C2 still deserialises.
    #[serde(default)]
    pub google_calendar: bool,
}
```

and in `create_vault_in`, in the `VaultPlan` literal, beside `personal_calendar`:

```rust
        google_calendar: plan.google_calendar,
```

**(a3) `app/src/scaffold.rs` — one `VaultPlan` field and a second `calendars:` entry.** C1's writer already turns `personal_calendar` into a block list; this adds the Google line beside it, so **the vault is born with both and nothing writes `config/ingest.yaml` before Finish**:

```rust
pub struct VaultPlan {
    // …existing fields…
    /// C2 (§11a): the account has a Google calendar grant, so the vault carries the marker feed
    /// `cloud:google`. Not a URL — hand-off H4's fetcher matches the `cloud:` prefix and routes it
    /// to `/ingest-calendar?name=google`. Deliberately unfetchable: a vault whose grant is gone
    /// gets one "using snapshot" warning and still ranks the day.
    pub google_calendar: bool,
}
```

and, replacing C1's `match &p.personal_calendar { … }` block in `ingest_yaml`:

```rust
    // The engine reads `calendars:` as a list of `{name, ics_url}` mappings (`calfeed.rs`), and an
    // empty list is why the first page of a fresh install used to show a day with no busy time in
    // it at all. Two possible entries, in a fixed order, through the same `yaml_scalar` every other
    // wizard value goes through: `personal` (the secret iCal address, C1's) and `google` (the
    // marker for the account's grant, C2's — §11a).
    let mut entries: Vec<String> = Vec::new();
    if let Some(u) = &p.personal_calendar {
        entries.push(format!("  - name: personal\n    ics_url: {}\n", yaml_scalar("personal calendar address", u)?));
    }
    if p.google_calendar {
        entries.push("  - name: google\n    ics_url: 'cloud:google'\n".to_string());
    }
    if entries.is_empty() {
        s.push_str("calendars: []\n");
    } else {
        s.push_str("calendars:\n");
        for entry in &entries {
            s.push_str(entry);
        }
    }
```

C1's own test `a_personal_calendar_becomes_the_engines_calendars_list` keeps passing (a `VaultPlan` with `google_calendar: false` produces byte-identical output); H9 adds one case beside it asserting that `google_calendar: true` with no personal address produces `calendars:\n  - name: google\n    ics_url: 'cloud:google'\n`, and that both together produce both, personal first.

**(a4) `app/static/index.html` — C1's existing element, made live.** C1 leaves, in the `#wiz-calendars` panel:

```html
<div class="wiz-row" id="wiz-google-row"><button class="b" id="wiz-google" disabled>Sign in with Google instead</button><span class="meta">Next release: one Google sign-in for your calendar, and email later if you want it.</span></div>
```

H9 removes the `disabled` attribute and replaces the span's text, and **changes no other element on that panel**:

```html
<div class="wiz-row" id="wiz-google-row"><button class="b" id="wiz-google">Sign in with Google instead</button><span class="meta">Reads your Google Calendar so Knowlu knows when you are busy. Read-only, and you can disconnect at any time. While Knowlu is in Google's testing programme this works for invited testers and the connection needs renewing about once a week.</span></div>
```

**(a5) `app/static/console.js` — one listener, one poll, one plan field**, beside the wizard's existing handlers:

```js
// C2 (§11a): one Google connect for the calendars now, Gmail later and only if asked. The scope
// named here is "calendar" — the *sensitive* one, which carries a lighter review and no CASA.
// Nothing is written to the vault here: the wizard creates it at Finish, so this sets a flag on
// the plan and `scaffold::ingest_yaml` writes the `calendars:` entry when the vault is born.
var WIZ_GOOGLE = false;

document.getElementById("wiz-google").addEventListener("click", async () => {
  var got = await invoke("google_connect_url", { scope: "calendar" });
  if (!got.ok) { showWizardError(got.error); return; }
  var opened = await invoke("open_external", { url: got.url });
  if (!opened.ok) { showWizardError(opened.error); return; }
  var note = document.getElementById("wiz-cal-note");
  note.textContent = "Finish signing in to Google in your browser — this may take a moment.";
  // The consent window closes itself, so there is nothing else to tell us the round trip finished.
  // Twenty tries at three seconds is a minute, which is longer than a consent takes and shorter
  // than a student will sit staring at it; giving up is a message, never a silent stall.
  for (var i = 0; i < 20; i++) {
    await new Promise(function (r) { setTimeout(r, 3000); });
    var status = await invoke("google_connected");
    if (status.ok && status.calendar) {
      WIZ_GOOGLE = true;
      note.textContent = "Google Calendar connected — already on your calendar.";
      document.getElementById("wiz-google").disabled = true;
      return;
    }
  }
  note.textContent = "Google did not finish connecting. You can try again, or use the secret address above.";
});
```

and one field in the plan the page already builds for `create_vault`:

```js
    var plan = { ics_url: WIZ.ics || null, personal_calendar: WIZ.cal || null,
                 google_calendar: WIZ_GOOGLE,
                 /* …the rest, unchanged… */ };
```

**(a6) `app/src/main.rs` — the WIZARD window's list** gains three commands: `account::google_connect_url`, `account::google_connected` and `account::open_external` (the counts paragraph's 44 / 26 / 59 holds only with all three on the wizard). **The vault-less shell's list, not the console's**: C1 registers 23 there and 43 on the console, and a command needing `ConsoleState` cannot run on the shell. All three need none, which is why they are shaped as they are.

---

#### Phase (b) — the console: one command, and its UI is C4's

**(b1) `app/src/account.rs` — the settings-panel command**, over a vault that already exists:

```rust
/// Add or remove the `- name: google` calendar entry on an existing vault (§11a).
///
/// **The console window's command**, because it needs `ConsoleState` for the vault path — which is
/// exactly why it cannot be the wizard's: on panel 5 there is no vault yet, and the wizard's route
/// is `WizardPlan.google_calendar` into `scaffold::ingest_yaml` instead.
///
/// `ics_url: 'cloud:google'` is not a URL — it is the marker hand-off H4's fetcher matches to route
/// this feed through `/ingest-calendar?name=google`.
#[tauri::command(async)]
pub fn set_google_calendar(state: tauri::State<'_, ConsoleState>, connected: bool) -> Value {
    let path = state.vault.join("config").join("ingest.yaml");
    let text = match knowlu_engine::pystr::read_text(&path) {
        Ok(text) => text,
        Err(e) => return json!({ "ok": false, "error": e.to_string() }),
    };
    let mut value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
        Ok(v) => v,
        Err(e) => return json!({ "ok": false, "error": e.to_string() }),
    };
    // `calendars: []` deserialises as an empty sequence; a vault whose key is missing entirely is
    // a vault this cannot repair, and saying so beats writing a key the scaffold never wrote.
    let Some(entries) = value.get_mut("calendars").and_then(serde_yaml_ng::Value::as_sequence_mut) else {
        return json!({ "ok": false, "error": "config/ingest.yaml has no calendars list" });
    };
    entries.retain(|e| e.get("name").and_then(|n| n.as_str()) != Some("google"));
    if connected {
        match serde_yaml_ng::from_str("name: google\nics_url: 'cloud:google'\n") {
            Ok(entry) => entries.push(entry),
            Err(e) => return json!({ "ok": false, "error": e.to_string() }),
        }
    }
    match crate::scaffold::rewrite_ingest_yaml(&state.vault, &value) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
```

`scaffold::rewrite_ingest_yaml(vault: &Path, value: &serde_yaml_ng::Value) -> Result<(), String>` does not exist in C1: **H9 adds it**, three lines beside `ingest_yaml`, emitting through the same `write_file` → `pystr::write_text` path so the file's line endings stay the vault's. It joins the **console** window's `generate_handler!` list.

**(b2) Its UI is C4's, and this hand-off does not build one.** There is no settings row for *Connect / Disconnect Google* in C1 (its settings panel predates the account) and none in C2. `set_google_calendar` therefore ships with **one caller in C4** and none before it — named under *What is NOT in this plan*, beside the (c) toggle it will sit next to. **What happens meanwhile is a real, bounded behaviour and not a gap:** a student who revokes the grant at Google leaves `cloud:google` in `calendars:`; `/ingest-calendar?name=google` then answers **404** (`read_google_grant` returns nothing for a revoked or scope-less grant); `calfeed::load_calendar_events` turns that per-feed failure into `google: fetch failed (…); using snapshot` and keeps the last known busy time; and the same slot's `judge` step reports the quiet source on stdout. The day still ranks, the line says why, and C4's UI removes the entry when it lands.

---

#### The rest of phase (a): `open_external`, and the two assertions

**`app/src/account.rs` — one `open_external`, and only one.** No new dependency: the check is three prefixes and a length.

```rust
/// Open a URL in the system browser — the same `open_in_browser` mechanism `open_policy` and
/// `open_checkout` already use, exposed once so the wizard's Google step needs no third private
/// path. It joins the **wizard** window's list beside the two commands above.
///
/// **`https://accounts.google.com/` and nothing else.** This command takes a URL from the page, and
/// the page takes it from the service; one that opened anything would be one indirection away from
/// opening a `file:` URL or a phishing page if either the service or the page were ever wrong.
/// There is exactly one thing it is for, and the CR/LF guard is there because a header-shaped
/// injection into a URL that reaches `explorer.exe` is the other way this goes wrong.
#[tauri::command(async)]
pub fn open_external(url: String) -> Value {
    let allowed = url.starts_with("https://accounts.google.com/")
        && !url.contains('\n')
        && !url.contains('\r')
        && url.len() < 2048;
    if !allowed {
        return json!({ "ok": false, "error": "only the Google consent page may be opened" });
    }
    match open_in_browser(&url) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
```

**The two assertions that today require the placeholder to be inert.** Both change in the same commit, or C1's suite goes red the moment this lands:

```rust
// app/tests/static_assets.rs (C1's) — C1 asserts the placeholder is present AND disabled:
    assert!(panel.contains("id=\"wiz-google\"") && panel.contains("disabled"), "the Google placeholder is present and inert");
// becomes, with C2 merged:
    assert!(panel.contains("id=\"wiz-google\""), "the Google sign-in is on the calendars panel");
    assert!(!panel.contains("id=\"wiz-google\" disabled"), "…and is live from C2 on");
```

```python
# scripts/wizard-check.py (the CONTROLLER's, not C1's) — C1's headless walk requires it inert:
    if not page.query_selector("#wiz-google[disabled]"):
        bad.append("the Google placeholder must be present and inert in C1")
# becomes:
    if not page.query_selector("#wiz-google:not([disabled])"):
        bad.append("the Google sign-in must be live from C2 on")
```

C1's third assertion — `console.js` contains neither `"connect_google"` nor `gmail.readonly` — **is untouched and stays true**: the commands are `google_connect_url` / `google_connected` / `open_external`, and the scope string `gmail.readonly` never appears on the device, because the service decides what `scope: "gmail"` means. That is worth keeping deliberately rather than by luck.

**Command counts.** H9 adds **three** commands: two on the wizard window (`google_connect_url`, `google_connected`), one on the console (`set_google_calendar`), plus `open_external` on the wizard — **four in total**, so C1's 43 / 23 / 55 becomes 44 / 26 / 59. **Do not edit those numbers by hand:** C1's Task 21 recount is re-run at C2's merge, and the count is read from `main.rs`'s two lists.

**If any part of (a) cannot land at merge, land none of it.** A button without (a2)/(a3) connects a calendar the engine never reads; (a4) without the two assertion changes turns two suites red. Phase (b) is separable and can land alone or not at all. The fallback in every case is the one this plan already relies on: **the personal calendar reaches Knowlu by its secret iCal address**, which is C1's `calendar_ics` path and works with no Google grant at all. Nothing else in C2 depends on H9 — `/ingest-calendar` still serves `?name=personal`, and `google-connect` simply has no caller until the button exists.


### H11 — `engine/src/ingest.rs`, the first-run archive (needed by Task 8a; §11a, ruling R-OB-3)

**This is the half that is the guarantee.** It works with no account, it is the only half that can write `archive/` and `state/ingest-seen.md`, and it is what a pre-C1 install runs. The service's `past_due_uids` corroborates it and is what the wizard counts; when the two disagree the device wins, because the device knows the vault's own timezone.

**Change 1 — the predicate `ingest` alone owns** (ruling **R-C2-9**). Beside `load_seen` / `record_seen`:

```rust
/// R-OB-3: is this the first ingest this vault has ever had?
///
/// **The absence of `today.md`, and deliberately not the absence of `state/ingest-seen.md`**
/// (ruling R-C2-9). That ledger is not `ingest`'s alone: `coursework` imports
/// `crate::ingest::record_seen` (`coursework.rs:19`) and calls it at `:310` and `:415`, and the
/// slot order is `coursework → ingest → judge → rank`. So on a genuinely fresh vault the
/// seen-ledger already exists by the time `ingest` looks at it, and a flag derived from it is
/// `false` on exactly the run R-OB-3 was written for. `today.md` is written by `rank`, the LAST
/// step of the slot, so the first `ingest` always sees it absent and every later one sees it
/// present.
///
/// It is also the predicate the app already uses to decide a vault needs its first slot at all
/// (`app/src/scheduler.rs::needs_first_run`, main `f8649d5`), so the two halves of "this vault has
/// never been through a slot" now agree by construction rather than by coincidence.
///
/// **The residue, and it is acceptable:** a first slot that dies before `rank` leaves `today.md`
/// absent, so the *next* slot is "first" again and archives whatever has gone past in between.
/// Nothing is double-archived — the uid is already in the seen-ledger — and no page has been
/// rendered for the student to have seen the difference, because `today.md` not existing is
/// precisely the premise.
pub fn is_first_run(vault: &Path) -> bool {
    !vault.join("state").join("today.md").exists()
}
```

**Change 2 — one constant and one parameter.** Beside `NOTE_TEMPLATE`:

```rust
/// R-OB-3: why a note went straight to `archive/` on a first ingest. A frontmatter field rather
/// than a naming convention, so a human reading the note in six months can see it, and so a future
/// `surface` view can filter on it without parsing a filename.
pub const IMPORTED_PAST: &str = "imported-past";

/// The archived twin of `NOTE_TEMPLATE`. Identical but for the two lines that say why it is here —
/// deliberately a second template rather than a substitution on the first, because the two differ
/// in what they MEAN and a reader should not have to diff them to see it.
pub const IMPORTED_PAST_TEMPLATE: &str = r#"---
title: {title}
course: {course}
domain: school
due: {due}
effort_hours: 1.0
effort_confidence: low
effort_source: inferred
importance: 3
importance_reason: "pending enrichment"
status: archived
archived_reason: imported-past
progress: 0
created_by: blackboard
source_uid: {uid}
needs_enrichment: false
---

{body}
"#;
```

`needs_enrichment: false`, deliberately: an archived item is not work, and flagging it would send every stale import to `/judge-task` at a penny a time for nothing.

**Change 3 — `sync_tasks` takes `first_run`.** Its signature gains one parameter, after `today`:

```rust
pub fn sync_tasks(
    events: &[Event],
    vault: &Path,
    course_map: &[(String, String)],
    ctx: Option<&crate::write::WriteContext>,
    journal: &mut crate::journal::Journal,
    today: Option<Date>,
    /// R-OB-3. `true` only on a vault that has never been through a whole slot — `is_first_run`
    /// above, which is the absence of `today.md` and **not** of the seen-ledger (R-C2-9).
    /// `run_lines` computes it; every caller in the tests passes `false`, which is the behaviour
    /// they were written against.
    first_run: bool,
) -> Vec<String> {
```

**Change 4 — the create branch.** Immediately after `let course = match_course(event, course_map);` and before the slug is built, insert:

```rust
        // R-OB-3: a feed's window reaches backwards, and a vault born today has no history to
        // reconcile against — Quinn's first slot imported four items already past due, one of them
        // from 2025. On a FIRST ingest such an item is recorded as seen and written straight into
        // `archive/`, so the first page a student ever sees shows the future.
        //
        // **Not skipped** — skipping leaves the uid unseen and the next run creates it. **Not
        // created-then-deleted** — that is two journal records and a note that briefly ranks. One
        // `create` into `archive/`, one `record_seen`, one line.
        //
        // Strictly before TODAY, never before *now*: an item due at 23:59 today is today's work,
        // and the one thing worse than importing a stale task is archiving a live one.
        let past_due = first_run
            && match due {
                Due::Date(d) => d < stamp_date,
                Due::DateTime(dt) => dt.date() < stamp_date,
            };
        if past_due {
            let slug = format!("{}-{}", course.clone().unwrap_or_else(|| "task".into()), slugify(&event.title));
            let archive_dir = vault.join("archive");
            let _ = std::fs::create_dir_all(&archive_dir);
            let mut path = archive_dir.join(format!("{slug}.md"));
            let mut suffix = 2;
            while path.exists() {
                path = archive_dir.join(format!("{slug}-{suffix}.md"));
                suffix += 1;
            }
            let body = IMPORTED_PAST_TEMPLATE
                .replace("{title}", &json_dumps_unicode(&event.title))
                .replace(
                    "{course}",
                    &course.as_deref().map(json_dumps_unicode).unwrap_or_else(|| "null".to_string()),
                )
                .replace("{due}", &new_due)
                .replace("{uid}", &json_dumps_unicode(&event.uid))
                .replace("{body}", &event.description);
            let rel_path = crate::ids::rel(vault, &path);
            match crate::write::create(vault, &rel_path, &body, ctx, journal, None) {
                Ok(created) => {
                    let stem = created.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    log.push(format!("archived (imported-past) {stem}"));
                    known.insert(event.uid.clone(), created);
                    let _ = record_seen(vault, &event.uid, &event.title, &stamp);
                }
                Err(err) => log.push(format!("skipped (unwritable): {err}")),
            }
            continue;
        }
```

with, beside the existing `let stamp = …` at the top of `sync_tasks`:

```rust
    let stamp_date = today.unwrap_or_else(|| jiff::Zoned::now().date());
    let stamp = stamp_date.strftime("%Y-%m-%d").to_string();
```

(`stamp` is unchanged in value; the date it was formatted from is now kept, because the comparison needs it and re-parsing a string it just printed would be the wrong shape.)

**Change 5 — `run_lines` computes the flag and passes it.** Two edits. First, beside the `cloud` binding H3 adds:

```rust
    // R-OB-3, by R-C2-9's predicate: this vault has never been through a whole slot if `rank` has
    // never written `today.md`. NOT the seen-ledger — `coursework` runs before `ingest` in the
    // same slot and calls `record_seen`, so that file exists on the very first run.
    let first_run = is_first_run(vault);
```

Second, the `sync_tasks` call gains it:

```rust
    let mut log = sync_tasks(&events, vault, &course_map, Some(&ctx), &mut journal, None, first_run);
```

and, when the service also answered (H3's `(None, Some(client))` arm), the two lists are unioned — the device's own check is the guarantee and this only adds to it:

```rust
    if first_run {
        log.push(format!("ingest: first run — {} item(s) already past were archived", 
            log.iter().filter(|l| l.starts_with("archived (imported-past)")).count()));
    }
```

**Change 6 — H3's two `fetch_ics` call sites** take the flag and the tuple (Task 8a step 6):

```rust
        (None, Some(client)) => match crate::cloudmodel::fetch_ics(client, first_run) {
            Ok((text, _past)) => Ok(text),
```

`_past` is bound and unused on purpose: the device archives on its own comparison, and binding the list here is what makes the next reader ask which half is authoritative. The answer is in `fetch_ics`'s doc comment.

**Every existing caller of `sync_tasks` passes `false`.** There are three, all in `ingest.rs`'s own test module; `false` is exactly the behaviour they were written against, so no assertion moves. `cli.rs` does not call it.

### H12 — `engine/src/approvals.rs`, one more arm (needed by Task 7a; ruling R-OB-1)

The same shape as H6's `kind: rule` arm, and for the same reason. Insert it beside that one, in `process_approvals`' `status == "approved"` chain:

```rust
        } else if kind == "coursework-map" {
            // C2 (§11a, R-OB-1): a mapping from an unmapped zyBook or VHL section to a course.
            // `rank` does not apply it — the next `coursework` step does, before it fetches, so a
            // card approved at 11am is a mapping the noon slot already uses. Left exactly as it is
            // here, and deliberately not an `unknown kind` warning: a WARN line on every run until
            // the next slot would train the reader to ignore the list.
        } else {
```

A **rejected** card is archived by `process_approvals` as any rejection is, and the mapping is simply never written. It is **not** re-proposed on the next slot, and that needs a guard rather than a hope: `write::create`'s `Exists` check only sees `approvals/`, so Task 7a's `asked_map_keys` scans `approvals/` **and** `archive/` for a `kind: coursework-map` card carrying the same `map_key`, and suppresses the proposal while that card's own `expires:` is still in the future. **Where the memory lives:** the card itself, at `archive/map-<source>-<slug>.md`, which the vault keeps forever and whose move the journal records — no new state anywhere. **For how long:** 30 days from the day it was minted (`write_map_card` sets `expires` to `today + 30`), after which a still-unmapped book is proposed once more. That is the right loop: a student who does not recognise a book is asked again next month, not never and not twice a day.


### H13 — nothing for the coursework split

Recorded so the next reader does not go looking: `coursework.rs` is wholly C2's for this stream, and the cloud branch Task 7 adds is inside it. No other file changes for the coursework split.


## The tasks

Tasks 0–4 are the **first shippable increment**: `/judge-task`, `CloudModel`, and `knowlu-engine judge` using it. Merging that closes the judgment gap the repo cut opened, and everything after it is additive.

---

### Task 0: Deno, the Supabase CLI, and the model behind an interface

The one file in this repository that imports an inference SDK, with a scripted fake beside it, proven against a loopback listener. Nothing after this task names a provider.

**Files:**
- Create: `cloud/supabase/functions/_shared/judge_anthropic.ts`
- Test: `cloud/supabase/functions/_shared/judge_anthropic_test.ts`

**Interfaces:**
- Consumes: C1's `cloud/supabase/deno.json` (its `imports` map and `lineWidth: 110` formatter settings). If C1 has not merged yet, create neither file — hand-off H7 adds C2's one import-map line at merge, and until then run `deno` commands with `--no-config` and the fully-qualified specifiers.
- Produces: `interface JudgeModel { complete(req: ModelRequest): Promise<ModelReply> }`; `ModelRequest { model: string; system: string; user: string; schema: Record<string, unknown>; maxTokens: number; sampling: Sampling }`; `type Sampling = { temperature: number } | Record<string, never>`; `ModelReply { json: Record<string, unknown>; inputTokens: number; outputTokens: number }`; `class ModelRefused extends Error`; `class AnthropicModel implements JudgeModel`; `class ScriptedModel implements JudgeModel` with a public `seen: ModelRequest[]`; `const CALL_TIMEOUT_MS = 120_000`.

- [ ] **Step 1: The tools.** Check first, install only what is missing. In PowerShell:

```powershell
if (-not (Get-Command deno -ErrorAction SilentlyContinue)) { winget install --id DenoLand.Deno --accept-package-agreements --accept-source-agreements }
if (-not (Get-Command supabase -ErrorAction SilentlyContinue)) { winget install --id Supabase.CLI --accept-package-agreements --accept-source-agreements }
$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"
deno --version; supabase --version
```

Neither is on this machine as of 2026-09-09 (checked). Expect Deno 2.x and Supabase CLI 2.x. **Quinn runs `supabase login` himself, once, in his own shell** — it stores an access token in the user profile and this session never sees it. Then, once P0 has produced the staging ref: `supabase link --project-ref <staging ref>` from the repository root. No Docker is used anywhere in this plan; `supabase db reset` and `supabase start` are never run.

- [ ] **Step 2: Write the failing test** — `cloud/supabase/functions/_shared/judge_anthropic_test.ts`:

```ts
// The model client, exercised against a listener bound to 127.0.0.1:0 inside this same test.
// That is not egress: no DNS, no route off the machine, no listener on a routable interface.
// The listener is shut down before the test returns (CLAUDE.md, plan 3a's loopback rule).
import { assert, assertEquals, assertRejects } from "@std/assert";
import { AnthropicModel, ModelRefused, ScriptedModel } from "./judge_anthropic.ts";

const SCHEMA = {
  type: "object",
  properties: {
    course: { type: ["string", "null"] },
    effort_hours: { type: "number" },
    importance: { type: "integer" },
    importance_reason: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["course", "effort_hours", "importance", "importance_reason", "confidence"],
  additionalProperties: false,
} as const;

const ANSWER =
  '{"course":"cs-100","effort_hours":2.5,"importance":4,"importance_reason":"20% of the grade","confidence":0.82}';

function messagesReply(text: string, stopReason = "end_turn"): Response {
  return Response.json({
    id: "msg_test",
    type: "message",
    role: "assistant",
    model: "claude-haiku-4-5",
    content: [{ type: "text", text }],
    stop_reason: stopReason,
    stop_sequence: null,
    usage: { input_tokens: 11, output_tokens: 22 },
  });
}

function request(overrides: Record<string, unknown> = {}) {
  return {
    model: "claude-haiku-4-5",
    system: "You estimate effort and importance.",
    user: "Title: CS 100 HW 01",
    schema: SCHEMA as unknown as Record<string, unknown>,
    maxTokens: 256,
    sampling: { temperature: 0 },
    ...overrides,
  };
}

Deno.test("the request is the pinned id, the row's sampling, and the schema — and the reply is parsed", async () => {
  let body: Record<string, unknown> = {};
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    body = await req.json();
    return messagesReply(ANSWER);
  });
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    const reply = await model.complete(request());
    assertEquals(reply.json.importance, 4);
    assertEquals(reply.json.effort_hours, 2.5);
    assertEquals(reply.inputTokens, 11);
    assertEquals(reply.outputTokens, 22);
    assertEquals(body.model, "claude-haiku-4-5");
    assertEquals(body.max_tokens, 256);
    // Constrained decoding on every call, without exception (cloud design §5.4 measure 3).
    const outputConfig = body.output_config as { format: { type: string; schema: unknown } };
    assertEquals(outputConfig.format.type, "json_schema");
    assertEquals(outputConfig.format.schema, SCHEMA);
    // No `thinking`: these are classifications, and on Haiku 4.5 omitting it is "do not think".
    assertEquals(body.thinking, undefined);
    // No `output_config.effort`: it ERRORS on Haiku 4.5 (checked with the claude-api skill).
    assertEquals((body.output_config as Record<string, unknown>).effort, undefined);
  } finally {
    await server.shutdown();
  }
});

Deno.test("temperature is sent only when the pinned row asks for it", async () => {
  // `temperature` / `top_p` / `top_k` are REMOVED and return a 400 on Sonnet 5, Opus 5, Opus 4.8,
  // Opus 4.7 and Fable 5/5.1; they remain valid on Haiku 4.5 and the 4.6 generation (checked with
  // the claude-api skill, 2026-09-09). §11 R8 says the eval picks the model — so the day it picks
  // one above Haiku, a hard-coded `temperature: 0` would 400 every call and every judgment would
  // read as `model failed`. The parameter therefore travels in the `models` row, not in this file.
  const seen: Array<Record<string, unknown>> = [];
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    seen.push(await req.json());
    return messagesReply(ANSWER);
  });
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await model.complete(request({ sampling: { temperature: 0 } }));
    await model.complete(request({ model: "claude-sonnet-5", sampling: {} }));
    assertEquals(seen[0].temperature, 0);
    assertEquals("temperature" in seen[1], false, "a model with sampling removed must not be sent one");
    assertEquals(seen[1].model, "claude-sonnet-5");
  } finally {
    await server.shutdown();
  }
});

Deno.test("a truncated reply says it was truncated, not that the model talked nonsense", async () => {
  // `stop_reason: "max_tokens"` with half a JSON object is a CONFIGURATION fault — the row's
  // `max_tokens` is too small for that schema — and it must not read as a provider fault, because
  // the pipeline logs both as `model failed` and only the words tell them apart.
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, () =>
    messagesReply('{"course":"cs-100","effort_ho', "max_tokens"));
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), Error, "hit max_tokens");
  } finally {
    await server.shutdown();
  }
});

Deno.test("a refusal is its own error class, whatever the model", async () => {
  // `stop_reason: "refusal"` is HTTP 200 with no usable content. Unreachable on Haiku 4.5, and
  // exactly what a pin change to Fable- or Opus-class would introduce — the branch exists so the
  // eval can move the pin without this file becoming the thing that breaks.
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, () =>
    messagesReply("", "refusal"));
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), ModelRefused);
  } finally {
    await server.shutdown();
  }
});

Deno.test("a reply that is not the schema's JSON is an error, not a silent empty answer", async () => {
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, () =>
    messagesReply("I am afraid I cannot answer that."));
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), Error, "did not parse as JSON");
  } finally {
    await server.shutdown();
  }
});

Deno.test("the scripted fake records what it was asked and answers in order", async () => {
  const fake = new ScriptedModel([{ importance: 3 }, new Error("boom")]);
  const first = await fake.complete(request());
  assertEquals(first.json.importance, 3);
  assertEquals(fake.seen.length, 1);
  assertEquals(fake.seen[0].user, "Title: CS 100 HW 01");
  await assertRejects(() => fake.complete(request({ user: "u2" })), Error, "boom");
  assertEquals(fake.seen.length, 2);
  assert(fake.seen[1].user === "u2");
});
```

- [ ] **Step 3: Run it and watch it fail.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_anthropic_test.ts`
Expected: FAIL — `Module not found "…/judge_anthropic.ts"`.

- [ ] **Step 4: Pin the SDK version.** Run `deno eval "const r = await fetch('https://registry.npmjs.org/@anthropic-ai/sdk/latest'); console.log((await r.json()).version)"` and put that exact version into **one** place: hand-off H7's `deno.json` import-map line, `"@anthropic-ai/sdk": "npm:@anthropic-ai/sdk@<version>"`. Record the number in this step's line: `Pinned: @anthropic-ai/sdk@<version>`. Never an unpinned or range specifier — it makes every deploy a different program. The import in `judge_anthropic.ts` is then the bare mapped name, so the pin lives in exactly one file in the repository.

- [ ] **Step 5: Write it** — `cloud/supabase/functions/_shared/judge_anthropic.ts`:

```ts
// The inference provider, and the ONLY file in this repository that imports an inference SDK
// (cloud design §10: "Nothing model-shaped ships in the app"; §11 R8: Anthropic is the launch
// provider). Everything else in the service talks to `JudgeModel`, so swapping the provider is
// this file plus one row in `models`.
//
// Three properties this file exists to hold:
//   1. **Determinism where it can exist** (§5.2): a JSON schema on every call, no `thinking` — the
//      three judgments are classifications, not reasoning tasks — and whatever sampling the pinned
//      row asks for. Sampling is the ROW's business, not this file's: `temperature`/`top_p`/`top_k`
//      are removed and return a 400 on Sonnet 5, Opus 5, Opus 4.8/4.7 and Fable 5/5.1, and remain
//      valid on Haiku 4.5 and the 4.6 generation (checked with the claude-api skill, 2026-09-09).
//      §11 R8 lets the eval move the pin; a hard-coded `temperature: 0` would make the first such
//      move 400 every call and read as a provider outage.
//   2. **Every unusable answer says which kind of unusable it was.** A truncated reply, a refusal
//      and a genuinely malformed reply are three different faults with three different fixes, and
//      the pipeline logs all three as `model failed` — so only these words tell them apart.
//   3. **The body never reaches a log.** Nothing here logs, and no error it throws quotes the
//      prompt or the reply text.
//
// Two things deliberately NOT used, so the next reader does not "fix" them:
//   - **Prompt caching.** The system prompts here are ~280 tokens; the minimum cacheable prefix is
//     512-4096 tokens depending on the model, so a `cache_control` breakpoint would silently never
//     cache. Revisit only if a prompt grows past the floor.
//   - **The Batch API** (50% cheaper, and the twice-daily slot is latency-tolerant for events and
//     email). It is the right lever if per-account spend ever becomes the binding constraint, and
//     it is a change to this file plus a queue — not to the pipeline. Recorded in Task 14's budget
//     work as the first thing to reach for.
import Anthropic from "@anthropic-ai/sdk";

/// One call's wall-clock bound. Deliberately BELOW the device's own 120 s per-call bound is not
/// possible on an edge function's budget, so it is the same number and the device is the one that
/// gives up first; `gmail-read` (Task 11), which makes many calls in one invocation, carries its
/// own wall-clock budget and a resume cursor instead of relying on this.
export const CALL_TIMEOUT_MS = 120_000;

/// What the pinned row says about sampling. `{}` means "this model has no sampling parameters" —
/// which is the correct request for every model above Haiku.
export type Sampling = { temperature: number } | Record<string, never>;

export interface ModelRequest {
  model: string;
  system: string;
  user: string;
  /** The JSON schema the reply is constrained to. */
  schema: Record<string, unknown>;
  maxTokens: number;
  sampling: Sampling;
}

export interface ModelReply {
  json: Record<string, unknown>;
  inputTokens: number;
  outputTokens: number;
}

export interface JudgeModel {
  complete(req: ModelRequest): Promise<ModelReply>;
}

/// `stop_reason: "refusal"` — HTTP 200, no usable content, a safety classifier declined. Its own
/// class so the pipeline can one day route it somewhere other than `model failed`.
export class ModelRefused extends Error {}

export class AnthropicModel implements JudgeModel {
  readonly #client: Anthropic;

  constructor(opts: { apiKey: string; baseURL?: string; timeoutMs?: number }) {
    this.#client = new Anthropic({
      apiKey: opts.apiKey,
      ...(opts.baseURL === undefined ? {} : { baseURL: opts.baseURL }),
      // Milliseconds in the TypeScript SDK (seconds in Python) — a real trap, and the reason this
      // constant is named `_MS`.
      timeout: opts.timeoutMs ?? CALL_TIMEOUT_MS,
      // One retry, not two: the caller is a slot step with its own budget, and a judgment that
      // fails is a normal outcome the device reports, never a failed run.
      maxRetries: 1,
    });
  }

  async complete(req: ModelRequest): Promise<ModelReply> {
    const response = await this.#client.messages.create({
      model: req.model,
      max_tokens: req.maxTokens,
      system: req.system,
      messages: [{ role: "user", content: req.user }],
      output_config: { format: { type: "json_schema", schema: req.schema } },
      ...req.sampling,
    });
    // Checked BEFORE `content` is read, always: on a refusal `content` is empty and on a
    // `max_tokens` stop it is half an object, and both would otherwise arrive as "bad JSON".
    if (response.stop_reason === "refusal") {
      throw new ModelRefused(`the model declined (${response.stop_details?.category ?? "no category"})`);
    }
    if (response.stop_reason === "max_tokens") {
      throw new Error(
        `the reply hit max_tokens (${req.maxTokens}) for model ${req.model}: raise the row's max_tokens`,
      );
    }
    const text = response.content
      .filter((block): block is Anthropic.TextBlock => block.type === "text")
      .map((block) => block.text)
      .join("");
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      // Deliberately does NOT quote the text: an unparsed reply is the one case where the model
      // may have echoed the item, and this message reaches a 500 log line.
      throw new Error(`the model's reply did not parse as JSON (${text.length} chars)`);
    }
    if (json === null || typeof json !== "object" || Array.isArray(json)) {
      throw new Error("the model's reply did not parse as JSON: not an object");
    }
    return {
      json: json as Record<string, unknown>,
      inputTokens: response.usage.input_tokens,
      outputTokens: response.usage.output_tokens,
    };
  }
}

/// The test double. Answers the scripted replies in order and records every request it was given,
/// so a pipeline test can assert what the prompt carried without a provider, a key or a socket.
export class ScriptedModel implements JudgeModel {
  readonly seen: ModelRequest[] = [];
  readonly #replies: Array<Record<string, unknown> | Error>;

  constructor(replies: Array<Record<string, unknown> | Error>) {
    this.#replies = [...replies];
  }

  complete(req: ModelRequest): Promise<ModelReply> {
    this.seen.push(req);
    const next = this.#replies.shift();
    if (next === undefined) {
      return Promise.reject(new Error("ScriptedModel: no reply scripted for this call"));
    }
    if (next instanceof Error) return Promise.reject(next);
    // Two token counts that are not zero, so a test that asserts usage is recorded actually can.
    return Promise.resolve({ json: next, inputTokens: 300, outputTokens: 60 });
  }
}
```

- [ ] **Step 6: Run it and watch it pass.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_anthropic_test.ts`
Expected: `ok | 6 passed | 0 failed`.
Then: `deno check --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_anthropic.ts` → no errors. If `output_config` or `stop_details` is rejected by the SDK's **types**, the pinned version predates them — bump the pin in step 4 rather than casting the parameter away.
Then: `deno fmt --config cloud/supabase/deno.json cloud/supabase/` and commit what it writes (C1 sets `lineWidth: 110`; several blocks in this plan are wider, so the committed bytes are the formatter's, not the plan's) and `deno lint --config cloud/supabase/deno.json cloud/supabase/` → clean.

- [ ] **Step 7: Confirm the pinned model actually advertises structured outputs**, before three tasks are built on the assumption. One call, no schema needed:

```
deno eval --allow-env --allow-net "const r = await fetch('https://api.anthropic.com/v1/models/claude-haiku-4-5', {headers:{'x-api-key':Deno.env.get('ANTHROPIC_API_KEY'),'anthropic-version':'2023-06-01'}}); const m = await r.json(); console.log(m.id, m.max_tokens, JSON.stringify(m.capabilities))"
```

Expected: the id, an output cap, and a `capabilities` object. **Record what it says about structured outputs in this step's line.** If the pinned model does not advertise them, the fallback is **strict tool use** — one tool whose `input_schema` is the same schema with `strict: true`, read back from the `tool_use` block instead of the text block — which is a change to `judge_anthropic.ts` alone and to nothing else. Needs P1; if the key is not set yet, mark this step blocked and let Task 3's smoke check be the first real call.

- [ ] **Step 8: Ask Quinn for P1.** *"The judgment service needs an Anthropic API key set as a Supabase project secret on `knowlu-staging`. You set it yourself, from your own shell, and I never see the value: `supabase secrets set ANTHROPIC_API_KEY=<value> --project-ref <staging ref>`. Production gets its own key at go-live, not now."* Record that it was asked; the plan continues either way until Task 3's smoke check.

- [ ] **Step 9: Commit.**

```bash
git add cloud/supabase/functions/_shared/judge_anthropic.ts cloud/supabase/functions/_shared/judge_anthropic_test.ts
git commit -F .git-commit-msg.txt   # "cloud: the inference provider behind one interface, proven against a loopback listener (C2 Task 0)"
```

---

### Task 1: The judgment service's schema

Six tables of C2's own, two columns added to one of C1's, RLS on every one, and a Docker-free static test that pins what the schema may never grow.

**Files:**
- Create: `cloud/supabase/migrations/20260911000100_judgment_service.sql`
- Test: `cloud/supabase/migrations/migrations_test.ts`

**Interfaces:**
- Consumes: C1's `public.accounts(id uuid primary key)` and C1's `public.corrections` (created by C1's `20260910000400_telemetry.sql` — **C2 never creates it**, ruling R-X-2).
- Produces: tables `models`, `judgments`, `rules`, `rule_evidence`, `usage_daily`, `budget_alerts`; the functions `charge_call` and `enforce_budget`; and two columns plus one index on C1's `corrections`. Column names are the contract every later task reads.

**Precondition P0** (the staging project). Without it, steps 1–4 still run; step 5 waits.

- [ ] **Step 1: Write the failing test** — `cloud/supabase/migrations/migrations_test.ts`:

```ts
// Static pins on the migrations. There is no Docker in this plan, so nothing here applies SQL:
// these are the invariants that would otherwise only be discovered on a project that already has
// rows in it, which is the wrong time to discover them.
import { assert, assertEquals } from "@std/assert";

const HERE = new URL(".", import.meta.url);

async function ours(): Promise<Array<[string, string]>> {
  const out: Array<[string, string]> = [];
  for await (const entry of Deno.readDir(HERE)) {
    if (entry.name.startsWith("20260911") && entry.name.endsWith(".sql")) {
      out.push([entry.name, await Deno.readTextFile(new URL(entry.name, HERE))]);
    }
  }
  out.sort();
  assert(out.length > 0, "C2's migrations are the 20260911 ones; C1's are 20260910 (R-X-8)");
  return out;
}

Deno.test("every table this stream creates has row level security enabled", async () => {
  for (const [name, sql] of await ours()) {
    for (const m of sql.matchAll(/create table if not exists (\w+)/g)) {
      assert(
        sql.includes(`alter table ${m[1]} enable row level security;`),
        `${name}: ${m[1]} must have RLS enabled. Every C2 function runs with the service role and ` +
          `scopes by account_id itself, so a table reachable by the anon key is one nobody meant to expose.`,
      );
    }
  }
});

Deno.test("the judgments table has nowhere to put a body", async () => {
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  const start = sql.indexOf("create table if not exists judgments");
  const create = sql.slice(start, sql.indexOf(");", start));
  // Column NAMES, not substrings: `item_id text not null` legitimately contains the word "text".
  for (const line of create.split("\n").slice(1)) {
    const column = /^\s{2}([a-z_]+)\s/.exec(line)?.[1];
    if (column === undefined) continue;
    assert(
      !["body", "title", "prompt", "reply", "text", "subject", "snippet", "description", "message"].includes(column),
      `cloud design §5.2/§5.6: the judgment log holds ids, field values, confidences and the ` +
        `promotion features — never the body. A column called '${column}' is how that stops being true.`,
    );
  }
});

Deno.test("no migration in this stream drops or truncates a table", async () => {
  for (const [name, raw] of await ours()) {
    const sql = raw.toLowerCase();
    for (const forbidden of ["drop table", "drop column", "truncate"]) {
      assert(!sql.includes(forbidden), `${name}: migrations are forward-only (${forbidden})`);
    }
  }
});

Deno.test("C2 never creates a table C1 owns; it only alters one", async () => {
  // Ruling R-X-2: `corrections` is C1's, with C1's columns (`ts`, `received_at`, nullable
  // `ours`/`theirs`, and a `kind` that is the NOTE kind). C2 adds two nullable columns and an
  // index and touches nothing else — a second `create table` would be two schemas racing.
  for (const [name, sql] of await ours()) {
    for (const owned of ["corrections", "accounts", "entitlements", "consents", "sources", "telemetry_events", "issues"]) {
      assert(
        !sql.includes(`create table if not exists ${owned}`) && !sql.includes(`create table public.${owned}`),
        `${name}: ${owned} is C1's table (R-X-2 / R-X-1). Alter it, never create it.`,
      );
    }
  }
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  assert(sql.includes("alter table public.corrections add column if not exists judgment_id uuid"));
  assert(sql.includes("alter table public.corrections add column if not exists judgment_kind text"));
});

Deno.test("the seeded model pins are the cheapest Haiku-class id, one per kind, with their price and sampling", async () => {
  const sql = await Deno.readTextFile(new URL("20260911000100_judgment_service.sql", HERE));
  const rows = [...sql.matchAll(/\('(task|event|email)',\s*'anthropic',\s*'([a-z0-9.-]+)'/g)];
  assertEquals(rows.length, 3, "one pinned model per kind (cloud design §5.2)");
  for (const [, , modelId] of rows) {
    // §11 R8: the eval suite picks the model, not taste. This starts at the cheapest tier and is
    // only ever changed by a migration row the eval justifies.
    assertEquals(modelId, "claude-haiku-4-5");
  }
  // The price belongs beside the pin, so a pin change reprices history correctly, and the sampling
  // belongs there too, so a pin above Haiku does not 400 every call.
  assert(sql.includes("usd_per_m_in") && sql.includes("usd_per_m_out"));
  assert(sql.includes("sampling"));
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations/migrations_test.ts`
Expected: FAIL — `NotFound: … 20260911000100_judgment_service.sql`.

- [ ] **Step 3: Write the migration** — `cloud/supabase/migrations/20260911000100_judgment_service.sql`:

```sql
-- Knowlu C2 — the judgment service (cloud design §5.2, §5.4).
--
-- RLS is enabled on every table and NO POLICY IS CREATED, on purpose: with RLS on and no policy,
-- the anon and authenticated roles can read and write nothing at all. Every C2 edge function runs
-- with the service role, which bypasses RLS, and scopes every statement by the account_id that
-- `requireActiveEntitlement` returned. No judgment row is ever served to a client, so there is
-- nothing a client policy could be for. If a later stream needs one, it adds it deliberately.
--
-- `pg_cron` is needed by 20260911000300 (nightly rule promotion) and is enabled here, once, so
-- that migration is a function plus a schedule and nothing else.
create extension if not exists pg_cron with schema extensions;

-- The pinned model per kind. Changing a pin is a migration row with a date, so every historical
-- judgment names the model that made it (cloud design §5.2, §5.4 measure 4).
--
-- `sampling` and the two prices live HERE and not in code, and both for the same reason: they are
-- properties of the pinned model, and the eval is allowed to move the pin (§11 R8).
--   * `sampling` — `temperature`/`top_p`/`top_k` are removed and return a 400 on Sonnet 5, Opus 5,
--     Opus 4.8/4.7 and Fable 5/5.1, and remain valid on Haiku 4.5 and the 4.6 generation. A pin
--     change to any of the first group sets this to `{}` in the same migration row.
--   * `usd_per_m_in` / `usd_per_m_out` — Haiku 4.5's published rates, so `monthly_spend` prices
--     each month with the model that actually ran it.
create table if not exists models (
  kind            text primary key check (kind in ('task', 'event', 'email')),
  provider        text not null default 'anthropic',
  model_id        text not null,
  prompt_version  text not null,
  grammar_version text not null,
  max_tokens      integer not null default 256 check (max_tokens between 64 and 4096),
  sampling        jsonb not null default '{"temperature": 0}'::jsonb,
  usd_per_m_in    numeric not null default 1.00,
  usd_per_m_out   numeric not null default 5.00,
  since           date not null default current_date
);
alter table models enable row level security;

-- `max_tokens` per kind, not one number: the email schema has eight fields including a 200-char
-- title and a 140-char why, and a truncated reply is a `max_tokens` stop that reads as a bad
-- answer. 256 is the classification default; email gets room.
insert into models (kind, provider, model_id, prompt_version, grammar_version, max_tokens) values
  ('task',  'anthropic', 'claude-haiku-4-5', 'task-1',  'task-1',  256),
  ('event', 'anthropic', 'claude-haiku-4-5', 'event-1', 'event-1', 256),
  ('email', 'anthropic', 'claude-haiku-4-5', 'email-1', 'email-1', 640)
on conflict (kind) do nothing;

-- One row per judgment, whatever the outcome. `fields` is field name -> the literal that was (or
-- would have been) written, plus the four promotion features (`created_by`, `title_prefix`,
-- `organizer`, `source`, `series_uid`) that tier 2 is keyed on. Those are keys, not content: a
-- three-word title prefix is already the note's filename. There is nowhere here to put a title, a
-- body, a prompt or a reply, and `migrations_test.ts` keeps it so.
create table if not exists judgments (
  id              uuid primary key default gen_random_uuid(),
  account_id      uuid not null references public.accounts(id) on delete cascade,
  kind            text not null check (kind in ('task', 'event', 'email')),
  item_id         text not null,
  tier            smallint not null check (tier between 0 and 3),
  outcome         text not null check (outcome in ('answered', 'low confidence', 'capped')),
  cause           text check (cause in ('below floor', 'incomplete', 'model failed', 'refused', 'truncated')),
  confidence      real not null default 0 check (confidence between 0 and 1),
  fields          jsonb not null default '{}'::jsonb,
  model           text,
  prompt_version  text,
  grammar_version text,
  prompt_hash     text,
  ms              integer not null default 0,
  origin          text not null default 'device' check (origin in ('device', 'gmail_api', 'events')),
  judged_at       timestamptz not null default now()
);
alter table judgments enable row level security;
create index if not exists judgments_account_day on judgments (account_id, kind, judged_at desc);
create index if not exists judgments_item on judgments (account_id, kind, item_id, judged_at desc);
create index if not exists judgments_origin on judgments (origin) where origin = 'gmail_api';

-- Ruling R-X-2: `public.corrections` is C1's table, created by C1's telemetry migration with C1's
-- columns (`ts`, `received_at`, nullable `ours`/`theirs`, and a `kind` that is the NOTE kind:
-- task|approval|course|info|issue|archive). C2 adds exactly what the eval and the promotion loop
-- need and imposes no constraint on anything C1 owns.
--   * `judgment_id`  — which judgment this correction is about. C2's reply carries the id (Task 3),
--                      the device stores it beside the field it wrote, and C1's telemetry.rs sends
--                      it back. Nullable: a correction to something no cloud judgment produced is
--                      still a correction.
--   * `judgment_kind`— the JUDGMENT kind (task|event|email), which is not C1's note kind. Both are
--                      needed and neither can be derived from the other: an event verdict and a
--                      task both live in notes of kind `task` once approved.
alter table public.corrections add column if not exists judgment_id uuid null;
alter table public.corrections add column if not exists judgment_kind text null
  check (judgment_kind in ('task', 'event', 'email'));
create index if not exists corrections_judgment_kind_ts on public.corrections (judgment_kind, ts desc);

-- Tier 2. `scope = 'account'` rows are written by the nightly promotion job; `scope = 'global'`
-- rows are inserted inactive and are activated only by a hand review (§11 R5) — there is no code
-- path in this stream that sets active = true on a global row.
create table if not exists rules (
  id          bigserial primary key,
  account_id  uuid references public.accounts(id) on delete cascade,
  scope       text not null check (scope in ('account', 'global')),
  kind        text not null check (kind in ('task', 'event', 'email')),
  feature     text not null check (feature in ('source', 'organizer', 'title_prefix', 'series', 'created_by+title_prefix')),
  value       text not null,
  verdict     jsonb not null,
  active      boolean not null default false,
  version     integer not null default 1,
  proposed_at date not null default current_date,
  expires_at  date not null default (current_date + 30),
  decided_at  timestamptz,
  constraint account_rules_name_an_account check (scope <> 'account' or account_id is not null),
  constraint global_rules_name_no_account check (scope <> 'global' or account_id is null)
);
alter table rules enable row level security;

-- **One ACTIVE rule per (account, kind, feature, value), and that is what the index says.**
-- A partial unique index on `active`: superseded versions stay for the audit trail, and two live
-- answers to one question cannot exist. (An earlier draft included `version` in the key and
-- filtered on nothing, which enforced neither.)
create unique index if not exists rules_one_active_per_feature on rules (
  coalesce(account_id, '00000000-0000-0000-0000-000000000000'::uuid), kind, feature, value
) where active;
create index if not exists rules_lookup on rules (kind, active, account_id);

-- Why a rule was promoted: the judgments that agreed. Written by `promote_rules` (Task 12) and
-- read by nothing else — it is the answer to "why does the model never get asked about this?"
create table if not exists rule_evidence (
  rule_id     bigint not null references rules(id) on delete cascade,
  judgment_id uuid not null references judgments(id) on delete cascade,
  agrees      boolean not null,
  primary key (rule_id, judgment_id)
);
alter table rule_evidence enable row level security;

-- The cost guards of §5.2. **The caps and the price were chosen together** — see the arithmetic
-- beside `DAILY_CAP` in `_shared/judge_caps.ts`. In one line: at Haiku 4.5's $1/$5 per MTok and
-- this plan's own prompt bounds, one call is about $0.0015, so a per-account daily cap of 1,000
-- calls would permit ~$45/month against a $9.99 subscription. The caps below are set at roughly
-- 4x plausible heavy use, which is ~$4.50/month at the ceiling, and `enforce_budget` is the real
-- stop.
create table if not exists usage_daily (
  account_id uuid not null references public.accounts(id) on delete cascade,
  day        date not null,
  kind       text not null check (kind in ('task', 'event', 'email')),
  calls      integer not null default 0,
  in_tokens  bigint not null default 0,
  out_tokens bigint not null default 0,
  primary key (account_id, day, kind)
);
alter table usage_daily enable row level security;

-- One statement, so two concurrent calls cannot both read `calls` below the cap and both write.
create or replace function charge_call(p_account uuid, p_kind text, p_cap integer)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  after integer;
begin
  insert into usage_daily (account_id, day, kind, calls)
       values (p_account, current_date, p_kind, 1)
  on conflict (account_id, day, kind)
    do update set calls = usage_daily.calls + 1
  returning calls into after;
  return after <= p_cap;
end;
$$;

-- The tokens a call actually used, recorded after the fact. Separate from `charge_call` because
-- the count is not known until the reply arrives, and a call that failed still spent its input.
create or replace function record_tokens(p_account uuid, p_kind text, p_in bigint, p_out bigint)
returns void
language sql
security invoker
set search_path = public, extensions
as $$
  insert into usage_daily (account_id, day, kind, calls, in_tokens, out_tokens)
       values (p_account, current_date, p_kind, 0, p_in, p_out)
  on conflict (account_id, day, kind)
    do update set in_tokens = usage_daily.in_tokens + excluded.in_tokens,
                  out_tokens = usage_daily.out_tokens + excluded.out_tokens;
$$;

-- §5.2's "monthly inference budget alert per account and global". **`usage_daily` and `models`
-- alone** — an earlier draft joined `judgments` as well, which multiplied each day's usage row by
-- that day's judgment count and reported a number several hundred times too large.
create or replace view monthly_spend as
select date_trunc('month', u.day) as month,
       u.account_id,
       sum(u.in_tokens) / 1000000.0 * m.usd_per_m_in
         + sum(u.out_tokens) / 1000000.0 * m.usd_per_m_out as usd
  from usage_daily u
  join models m on m.kind = u.kind
 group by 1, 2, m.usd_per_m_in, m.usd_per_m_out;

create table if not exists budget_alerts (
  account_id uuid not null references public.accounts(id) on delete cascade,
  month      date not null,
  usd        numeric not null,
  raised_at  timestamptz not null default now(),
  primary key (account_id, month)
);
alter table budget_alerts enable row level security;

-- The budget as an ENFORCED ceiling, not a dashboard. Called by the pipeline before every model
-- call (once per account per hour is enough, so the pipeline memoises it): over the ceiling, the
-- judgment is refused with outcome `capped` exactly as a daily cap refusal is, and one row lands
-- in `budget_alerts` so the month's overspend is visible without querying a view nobody queries.
create or replace function enforce_budget(p_account uuid, p_ceiling numeric)
returns boolean
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  spent numeric;
begin
  select coalesce(sum(usd), 0) into spent
    from monthly_spend
   where account_id = p_account and month = date_trunc('month', current_date);
  if spent < p_ceiling then
    return true;
  end if;
  insert into budget_alerts (account_id, month, usd)
       values (p_account, date_trunc('month', current_date)::date, spent)
  on conflict (account_id, month) do update set usd = excluded.usd;
  return false;
end;
$$;
```

- [ ] **Step 4: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/migrations/migrations_test.ts`
Expected: `ok | 5 passed | 0 failed`.

- [ ] **Step 5: Apply it to staging, and only staging.**

```
supabase db push --project-ref <staging ref>
```

Expected output names `20260911000100_judgment_service` and nothing else. If it names a `20260910…` file too, C1 has not been pushed yet — push C1's first, because `public.accounts` and `public.corrections` are its tables and the `alter table` here needs them. Then confirm the seed landed, using the service-role key already in the shell (never printed):

```
curl -s "$SUPABASE_URL/rest/v1/models?select=kind,model_id,max_tokens,sampling" -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
```

Expected: three rows, all `claude-haiku-4-5`, `email` at `max_tokens: 640`, each with `sampling: {"temperature": 0}`.

- [ ] **Step 6: Commit.**

```bash
git add cloud/supabase/migrations/20260911000100_judgment_service.sql cloud/supabase/migrations/migrations_test.ts
git commit -F .git-commit-msg.txt   # "cloud: the judgment service's schema — pinned models with their price and sampling, judgments, rules, usage caps, an enforced budget (C2 Task 1)"
```


---

### Task 2: The pipeline all three judgments share

The whole of §5.2's block diagram as one pure, injected, `deno test`-able module: rules → cap → budget → model → validate → log → reply. No handler, no network, no database client — every dependency arrives as a parameter.

**Files:**
- Create: `cloud/supabase/functions/_shared/judge_db.ts`, `judge_prompts.ts`, `judge_validate.ts`, `judge_rules.ts`, `judge_caps.ts`, `judge_log.ts`, `judge_models.ts`, `judge_pipeline.ts`
- Test: `cloud/supabase/functions/_shared/judge_pipeline_test.ts`, `judge_validate_test.ts`

**Interfaces:**
- Consumes: Task 0's `JudgeModel`, `ModelRequest`, `ModelReply`, `ModelRefused`, `ScriptedModel`, `Sampling`. Task 1's column names.
- Produces — **and note where each type lives, because the import graph must stay acyclic.** Every interface is declared in the module that implements it, and `judge_pipeline.ts` re-exports the lot so a handler and a test import from one place:
  - `judge_db.ts` (imports nothing of ours): `interface Db { select(path: string): Promise<unknown[]>; insert(table: string, row: Record<string, unknown>): Promise<Record<string, unknown> | null>; rpc(fn: string, args: Record<string, unknown>): Promise<unknown> }`; `serviceDb(): Db`
  - `judge_validate.ts` (imports nothing of ours): `type Kind = "task" | "event" | "email"`; `type Cause = "below floor" | "incomplete" | "model failed" | "refused" | "truncated"`; `const CONFIDENCE_FLOOR = 0.6`; `const MAX_REASON_CHARS = 140`; `EVENT_VERDICTS`; `EMAIL_TIERS`; `oneLine(text, max)`; `validate(kind, answer, seed): Validated`
  - `judge_models.ts`: `interface ModelRow { kind; provider; model_id; prompt_version; grammar_version; max_tokens; sampling: Sampling; usd_per_m_in: number; usd_per_m_out: number }`; `modelRow(db, kind): Promise<ModelRow>`
  - `judge_rules.ts`: `interface RuleTable { lookup(account, kind, item): Promise<Record<string, unknown> | null> }`; `features(kind, item): Array<[string, string]>`; `featureMap(kind, item): Record<string, string>`; `ruleTable(db): RuleTable`
  - `judge_caps.ts`: `interface CapStore { charge(account, kind): Promise<boolean>; withinBudget(account): Promise<boolean>; recordTokens(account, kind, inTokens, outTokens): Promise<void> }`; `DAILY_CAP: Record<Kind, number>`; `MONTHLY_CEILING_USD`; `capStore(db): CapStore`
  - `judge_log.ts`: `interface JudgmentRow { … }`; `interface JudgmentSink { write(row): Promise<string | null> }` — **returns the new row's id**; `judgmentSink(db): JudgmentSink`
  - `judge_prompts.ts`: `buildPrompt(kind, item, seed): Prompt`; `promptHash(kind): Promise<string>`; the three bounds
  - `judge_pipeline.ts`: `JudgeRequest`, `JudgeReply`, `PipelineDeps`, `judge(accountId, req, deps)`, `fieldsOf`, and `export type { CapStore, Cause, JudgmentRow, JudgmentSink, Kind, ModelRow, RuleTable }` plus `export { CONFIDENCE_FLOOR, DAILY_CAP }`

**No module imports `judge_pipeline.ts`** except the handlers, `judge_deps.ts` and the tests. A cycle here is not a style point: an ESM cycle whose value export is read at module scope is a temporal-dead-zone crash at deploy time, not a compile error.

- [ ] **Step 1: Write the failing validator test** — `cloud/supabase/functions/_shared/judge_validate_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { CONFIDENCE_FLOOR, validate } from "./judge_validate.ts";

Deno.test("a task answer is clamped, one-lined and accepted", () => {
  const got = validate("task", {
    course: "cs-100",
    effort_hours: 900,
    importance: 9,
    importance_reason: "  worth 20%\nof the grade  ",
    confidence: 1.4,
  }, { known_courses: ["cs-100"] });
  assertEquals(got.ok, true);
  assertEquals(got.verdict, {
    course: "cs-100",
    effort_hours: 40,
    importance: 5,
    importance_reason: "worth 20% of the grade",
    confidence: 1,
  });
});

Deno.test("a slug the vault does not know is dropped, not refused", () => {
  // Mirrors `judge::judge_task`: the effort and importance answers are still good, and a course
  // nothing renders would put the note in a group no course note explains. The DEVICE re-applies
  // `knows_course` after the reply arrives, so this check is a convenience and that one is the
  // guarantee — except on the Gmail path, where there is no device-side `judge_task` and this is
  // the only check (Task 11 says so where the list comes from).
  const got = validate("task", {
    course: "phys-999",
    effort_hours: 2,
    importance: 3,
    importance_reason: "no weights given",
    confidence: 0.9,
  }, { known_courses: ["cs-100"] });
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.course, null);
});

Deno.test("under the floor is refused and says which refusal it was", () => {
  const got = validate("task", {
    course: null, effort_hours: 2, importance: 3, importance_reason: "x", confidence: CONFIDENCE_FLOOR - 0.01,
  }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "below floor");
});

Deno.test("a missing required field is incomplete, not a bad answer", () => {
  const got = validate("task", { course: null, effort_hours: 2, confidence: 0.9 }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("an event why is shaped so the device's ledger can actually write it", () => {
  // `eventledger::why_problem` refuses a double quote, a newline and the field separator ' - '
  // written with a middle dot. A verdict the service produced and the device then cannot record
  // is a judgment thrown away.
  const got = validate("event", {
    verdict: "opportunity",
    why: 'she said "yes" \u00b7 maybe\nnext week',
    confidence: 0.8,
  }, {});
  assertEquals(got.ok, true);
  assertEquals(got.verdict?.why, "she said 'yes' - maybe next week");
});

Deno.test("an event verdict outside the ledger's three words is refused", () => {
  const got = validate("event", { verdict: "maybe", why: "unsure", confidence: 0.9 }, {});
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});

Deno.test("an email tier outside the five is refused", () => {
  const got = validate("email", { tier: "spam", why: "junk", confidence: 0.9 }, { known_courses: [] });
  assertEquals(got.ok, false);
  assertEquals(got.cause, "incomplete");
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_validate_test.ts`
Expected: FAIL — `Module not found "…/judge_validate.ts"`.

- [ ] **Step 3: Write `judge_validate.ts`.**

```ts
// Validation is the one place a bad answer is rejected (cloud design §5.2, §5.4 measure 3): the
// numbers are CLAMPED rather than refused — a 900-hour estimate is a bad answer, not a broken
// one — and the confidence floor is the single gate. This is the server-side twin of
// `judge::parse_reply` and `judge::judge_task`'s known-course rule, and it must not drift from
// them: the device applies the same clamps to whatever comes back.
export const CONFIDENCE_FLOOR = 0.6;
export const MAX_REASON_CHARS = 140;

export type Kind = "task" | "event" | "email";
/** Why an answer did not become a verdict. A CLOSED set — `judgments.cause` checks it. */
export type Cause = "below floor" | "incomplete" | "model failed" | "refused" | "truncated";

export interface Validated {
  ok: boolean;
  verdict?: Record<string, unknown>;
  cause?: Cause;
}

/** The three words `eventledger::VALID_VERDICTS` will accept. */
export const EVENT_VERDICTS = ["obligation", "opportunity", "drop"] as const;
/** The five email tiers of cloud design §5.3. */
export const EMAIL_TIERS = ["task", "borderline", "event", "opportunity", "information"] as const;

function num(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function clamp(value: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, value));
}

/**
 * One line, clipped by CHARACTERS, and safe for both writers that consume it: `write`'s
 * single-line frontmatter surgery and `eventledger`'s middle-dot-separated ledger line. A double
 * quote becomes an apostrophe and the separator becomes a dash, because `eventledger::why_problem`
 * refuses both outright and a refused why is a judgment thrown away.
 */
export function oneLine(text: string, max: number): string {
  const collapsed = text
    .replace(/["\u2028\u2029]/g, "'")
    .replace(/\s+/g, " ")
    .replace(/ \u00b7 /g, " - ")
    .trim();
  return [...collapsed].slice(0, max).join("");
}

export function validate(
  kind: Kind,
  answer: Record<string, unknown>,
  seed: Record<string, unknown>,
): Validated {
  const confidence = num(answer.confidence);
  if (confidence === null) return { ok: false, cause: "incomplete" };

  if (kind === "task") {
    const effort = num(answer.effort_hours);
    const importance = num(answer.importance);
    const reason = oneLine(typeof answer.importance_reason === "string" ? answer.importance_reason : "", MAX_REASON_CHARS);
    if (effort === null || importance === null || reason === "") return { ok: false, cause: "incomplete" };
    if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
    const known = Array.isArray(seed.known_courses) ? seed.known_courses as string[] : [];
    const raw = typeof answer.course === "string" ? answer.course.trim() : "";
    return {
      ok: true,
      verdict: {
        course: raw !== "" && known.includes(raw) ? raw : null,
        effort_hours: clamp(effort, 0.25, 40),
        importance: clamp(Math.round(importance), 1, 5),
        importance_reason: reason,
        confidence: clamp(confidence, 0, 1),
      },
    };
  }

  if (kind === "event") {
    const verdict = typeof answer.verdict === "string" ? answer.verdict : "";
    const why = oneLine(typeof answer.why === "string" ? answer.why : "", MAX_REASON_CHARS);
    if (!(EVENT_VERDICTS as readonly string[]).includes(verdict) || why === "") {
      return { ok: false, cause: "incomplete" };
    }
    if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
    return { ok: true, verdict: { verdict, why, confidence: clamp(confidence, 0, 1) } };
  }

  const tier = typeof answer.tier === "string" ? answer.tier : "";
  const why = oneLine(typeof answer.why === "string" ? answer.why : "", MAX_REASON_CHARS);
  if (!(EMAIL_TIERS as readonly string[]).includes(tier) || why === "") {
    return { ok: false, cause: "incomplete" };
  }
  if (confidence < CONFIDENCE_FLOOR) return { ok: false, cause: "below floor" };
  const known = Array.isArray(seed.known_courses) ? seed.known_courses as string[] : [];
  const course = typeof answer.course === "string" && known.includes(answer.course) ? answer.course : null;
  const effort = num(answer.effort_hours);
  const importance = num(answer.importance);
  return {
    ok: true,
    verdict: {
      tier,
      why,
      title: oneLine(typeof answer.title === "string" ? answer.title : "", 200),
      course,
      due: typeof answer.due === "string" && /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$/.test(answer.due) ? answer.due : null,
      effort_hours: effort === null ? null : clamp(effort, 0.25, 40),
      importance: importance === null ? null : clamp(Math.round(importance), 1, 5),
      confidence: clamp(confidence, 0, 1),
    },
  };
}
```

- [ ] **Step 4: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_validate_test.ts`
Expected: `ok | 7 passed | 0 failed`.

- [ ] **Step 5: Write the failing pipeline test** — `cloud/supabase/functions/_shared/judge_pipeline_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "./judge_anthropic.ts";
import {
  type CapStore, DAILY_CAP, judge, type JudgmentRow, type JudgmentSink, type ModelRow, type RuleTable,
} from "./judge_pipeline.ts";

const ROW: ModelRow = {
  kind: "task",
  provider: "anthropic",
  model_id: "claude-haiku-4-5",
  prompt_version: "task-1",
  grammar_version: "task-1",
  max_tokens: 256,
  sampling: { temperature: 0 },
  usd_per_m_in: 1.0,
  usd_per_m_out: 5.0,
};

class Sink implements JudgmentSink {
  readonly rows: JudgmentRow[] = [];
  write(row: JudgmentRow): Promise<string | null> {
    this.rows.push(row);
    return Promise.resolve(`judgment-${this.rows.length}`);
  }
}

class Caps implements CapStore {
  charged: Array<[string, string]> = [];
  tokens: Array<[string, string, number, number]> = [];
  constructor(private readonly allow = true, private readonly budget = true) {}
  charge(account: string, kind: "task" | "event" | "email"): Promise<boolean> {
    this.charged.push([account, kind]);
    return Promise.resolve(this.allow);
  }
  withinBudget(): Promise<boolean> {
    return Promise.resolve(this.budget);
  }
  recordTokens(account: string, kind: string, i: number, o: number): Promise<void> {
    this.tokens.push([account, kind, i, o]);
    return Promise.resolve();
  }
}

const NO_RULES: RuleTable = { lookup: () => Promise.resolve(null) };

const ITEM = {
  id: "task-abc123",
  title: "CS 100 HW 01 TRIPWIRE-9f2c",
  body: "Write a program that prints ASCII art. TRIPWIRE-9f2c.",
  source_uid: "zybooks:1839992",
  created_by: "zybooks",
  course: null,
  due: "2026-09-18T23:59",
};
const SEED = {
  course: null, effort_hours: null, slice_hours: 1.5,
  weights: "Homework 20%", preferences: "", known_courses: ["cs-100"],
};
const ANSWER = {
  course: "cs-100", effort_hours: 2.5, importance: 4,
  importance_reason: "homework is 20% of the grade", confidence: 0.82,
};

function deps(model: ScriptedModel, log: Sink, caps: Caps, rules: RuleTable = NO_RULES) {
  return { row: ROW, model, rules, caps, log, origin: "device" as const, now: () => 0 };
}

Deno.test("the model answer becomes a tier 3 verdict, one logged row, and a judgment id", async () => {
  const log = new Sink();
  const caps = new Caps();
  const reply = await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([ANSWER]), log, caps));
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.tier, 3);
  assertEquals(reply.verdict?.importance, 4);
  assertEquals(reply.model, "claude-haiku-4-5");
  assertEquals(reply.prompt_version, "task-1");
  // The id is returned so a caller CAN name the judgment — the Gmail queue does, and it is what a
  // human debugging one answer asks for. The device does not store it: `corrections.judgment_id`
  // is back-filled by the eval loader's join (Interfaces with C1, contract 4), because the
  // alternative was changing the bytes of every judged note's `judgment:` line.
  assertEquals(reply.judgment_id, "judgment-1");
  assertEquals(log.rows.length, 1);
  assertEquals(log.rows[0].tier, 3);
  assertEquals(log.rows[0].model, "claude-haiku-4-5");
  assert(log.rows[0].prompt_hash !== null && log.rows[0].prompt_hash.length === 64);
  // The tokens are recorded, or the monthly budget is a view over zeroes.
  assertEquals(caps.tokens, [["acct-1", "task", 300, 60]]);
});

Deno.test("a body token reaches no judgment row, and neither does a title beyond its prefix", async () => {
  // The server-side twin of `enrich.rs`'s `the_judgment_log_never_carries_a_notes_title_or_body_text`
  // (ruling R-3a-21). §5.2: "the log holds ids, field values, confidences and the four promotion
  // features — never the body." The tripwire is in the FOURTH word of the title on purpose: the
  // three-word `title_prefix` is a promotion key and does travel, and this proves the cut is where
  // it is claimed to be.
  const log = new Sink();
  await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([ANSWER]), log, new Caps()));
  const rendered = JSON.stringify(log.rows);
  assert(!rendered.includes("TRIPWIRE-9f2c"), "the note's body or full title reached a judgment row");
  assert(rendered.includes("CS 100 HW"), "the three-word title prefix is a promotion key and must travel");
  assertEquals(log.rows[0].fields.created_by, "zybooks");
  assertEquals(log.rows[0].fields.title_prefix, "CS 100 HW");
});

Deno.test("a rule answers without a model call and without charging the day's cap", async () => {
  // M2: a promoted rule must not spend the account's allowance. "Rules retire model calls" and
  // "rules still consume the model budget" cannot both be true, and the first one is the design.
  const model = new ScriptedModel([]);
  const log = new Sink();
  const caps = new Caps();
  const rules: RuleTable = {
    lookup: () => Promise.resolve({
      course: "cs-100", effort_hours: 2, importance: 2, importance_reason: "promoted rule", confidence: 1,
    }),
  };
  const reply = await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, log, caps, rules));
  assertEquals(reply.tier, 2);
  assertEquals(reply.outcome, "answered");
  assertEquals(model.seen.length, 0, "tier 2 answered, so tier 3 must never be reached");
  assertEquals(caps.charged, [], "a rule costs nothing, so it charges nothing");
  assertEquals(log.rows[0].model, null);
});

Deno.test("a model that throws is a low-confidence outcome, never a write", async () => {
  const log = new Sink();
  const reply = await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(new ScriptedModel([new Error("upstream 529")]), log, new Caps()));
  assertEquals(reply.outcome, "low confidence");
  assertEquals(reply.cause, "model failed");
  assertEquals(reply.verdict, null);
  assertEquals(log.rows[0].outcome, "low confidence");
  assertEquals(log.rows[0].cause, "model failed");
});

Deno.test("the daily cap refuses before the model is reached and says so", async () => {
  const model = new ScriptedModel([]);
  const log = new Sink();
  const reply = await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, log, new Caps(false, true)));
  assertEquals(reply.outcome, "capped");
  assertEquals(model.seen.length, 0);
  assertEquals(log.rows[0].outcome, "capped");
  assertEquals(DAILY_CAP.task, 60);
  assertEquals(DAILY_CAP.event, 80);
  assertEquals(DAILY_CAP.email, 120);
});

Deno.test("the monthly budget refuses too, and before the day's cap is charged", async () => {
  const model = new ScriptedModel([]);
  const caps = new Caps(true, false);
  const reply = await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, new Sink(), caps));
  assertEquals(reply.outcome, "capped");
  assertEquals(model.seen.length, 0);
  assertEquals(caps.charged, [], "no call was made, so nothing is charged");
});

Deno.test("the prompt carries the note and its grounding and nothing else", async () => {
  // The server-side twin of `judge.rs`'s test of the same name (plan 3a fidelity row R6).
  const model = new ScriptedModel([ANSWER]);
  await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: SEED },
    deps(model, new Sink(), new Caps()));
  const sent = model.seen[0].system + "\n" + model.seen[0].user;
  for (const forbidden of ["ingest.yaml", "credential", "acct-1", "C:\\", "http"]) {
    assert(!sent.includes(forbidden), `the prompt carried '${forbidden}'`);
  }
  assert(sent.includes("CS 100 HW 01"));
  assert(sent.includes("Homework 20%"));
  assert(sent.length < 6000, "the prompt is bounded however long the note is");
  // The pinned row's sampling reaches the client; the client decides nothing.
  assertEquals(model.seen[0].sampling, { temperature: 0 });
  assertEquals(model.seen[0].maxTokens, 256);
});

Deno.test("the prompt hash is stable across accounts with different planner slices", async () => {
  // M1: it hashes the TEMPLATE and the SCHEMA, and `slice_hours` is a per-account value that used
  // to be rendered into the hashed text — two accounts then sent different system prompts under
  // one hash, which is the one thing the hash exists to prevent.
  const a = new Sink(), b = new Sink();
  await judge("acct-1", { kind: "task", item: ITEM, heuristics_seed: { ...SEED, slice_hours: 1.5 } },
    deps(new ScriptedModel([ANSWER]), a, new Caps()));
  await judge("acct-2", { kind: "task", item: ITEM, heuristics_seed: { ...SEED, slice_hours: 3 } },
    deps(new ScriptedModel([ANSWER]), b, new Caps()));
  assertEquals(a.rows[0].prompt_hash, b.rows[0].prompt_hash);
});
```

- [ ] **Step 6: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_pipeline_test.ts`
Expected: FAIL — `Module not found "…/judge_pipeline.ts"`.

- [ ] **Step 7: Write `judge_db.ts`.** C1's posture, adopted verbatim: *"no runtime dependency exists at all; every handler talks to Postgres and to Auth over `fetch` against the REST and Auth APIs, so `supabase-js` never enters this codebase."* Forty lines, and it keeps `@anthropic-ai/sdk` the only third-party import in `cloud/`.

```ts
// PostgREST over `fetch`, with the service role. Not `supabase-js`: C1 established that this
// codebase has no runtime dependency but the inference SDK, and one HTTP client for one schema is
// smaller than a library that brings its own.
//
// **The service role bypasses RLS**, which is exactly why every C2 table has RLS on and no policy
// (Task 1) and why every call site here scopes by `account_id` in the query string. That scoping
// is the whole access control, so `judge_db_test.ts` scans this directory's `.ts` files for a
// `select`/`insert` on an account-scoped table that omits it — one missing `.eq` would be a
// cross-account read with no database backstop.
export interface Db {
  /** A PostgREST path, e.g. `models?kind=eq.task&select=*`. Returns the rows. */
  select(path: string): Promise<unknown[]>;
  /** Inserts one row and returns it (`Prefer: return=representation`), or null when asked not to. */
  insert(table: string, row: Record<string, unknown>, returning?: boolean): Promise<Record<string, unknown> | null>;
  update(path: string, patch: Record<string, unknown>): Promise<void>;
  /** `POST /rpc/<fn>`. */
  rpc(fn: string, args: Record<string, unknown>): Promise<unknown>;
}

function env(name: string): string {
  const value = Deno.env.get(name);
  if (value === undefined || value === "") throw new Error(`the function is missing ${name}`);
  return value;
}

/// Built lazily, at first use — never at module scope. A throw at module scope is a boot failure
/// with an opaque message; a throw here is caught by the handler and becomes a named 500.
export function serviceDb(): Db {
  const base = `${env("SUPABASE_URL")}/rest/v1`;
  const key = env("SUPABASE_SERVICE_ROLE_KEY");
  const headers = { apikey: key, Authorization: `Bearer ${key}`, "Content-Type": "application/json" };

  async function call(path: string, init: RequestInit): Promise<Response> {
    const response = await fetch(`${base}/${path}`, { ...init, headers: { ...headers, ...(init.headers ?? {}) } });
    if (!response.ok) {
      // The status and the PostgREST error CODE, never the body: a constraint violation's message
      // quotes the offending row, and this string reaches a log line (§5.6).
      const code = (await response.json().catch(() => ({}))).code ?? "unknown";
      throw new Error(`postgrest ${response.status} (${code}) on ${path.split("?")[0]}`);
    }
    return response;
  }

  return {
    async select(path) {
      return await (await call(path, { method: "GET" })).json();
    },
    async insert(table, row, returning = true) {
      const response = await call(table, {
        method: "POST",
        body: JSON.stringify(row),
        headers: { Prefer: returning ? "return=representation" : "return=minimal" },
      });
      if (!returning) return null;
      const rows = await response.json();
      return Array.isArray(rows) && rows.length > 0 ? rows[0] : null;
    },
    async update(path, patch) {
      await call(path, { method: "PATCH", body: JSON.stringify(patch), headers: { Prefer: "return=minimal" } });
    },
    async rpc(fn, args) {
      return await (await call(`rpc/${fn}`, { method: "POST", body: JSON.stringify(args) })).json();
    },
  };
}
```

- [ ] **Step 8: Write `judge_prompts.ts`.**

```ts
// The three prompts and the three schemas, versioned. Server-side from C2 on, so that changing
// how a judgment is asked for is a deploy and not an app release (cloud design §10).
//
// The task prompt is `judge::prompt_for` moved across the boundary, with the same four bounds —
// body 1200 characters, weights 600, preferences 600, reason 140 — and the same rules, because
// the frozen behaviour these bounds protect is "the question can never be pushed out of the
// context by a note that pasted a syllabus".
import { oneLine } from "./judge_validate.ts";

export const MAX_BODY_CHARS = 1200;
export const MAX_WEIGHTS_CHARS = 600;
export const MAX_PREFS_CHARS = 600;

export interface Prompt {
  system: string;
  user: string;
  schema: Record<string, unknown>;
}

function clip(text: string, max: number): string {
  return [...text].slice(0, max).join("");
}

const TASK_SCHEMA = {
  type: "object",
  properties: {
    course: { type: ["string", "null"] },
    effort_hours: { type: "number" },
    importance: { type: "integer", enum: [1, 2, 3, 4, 5] },
    importance_reason: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["course", "effort_hours", "importance", "importance_reason", "confidence"],
  additionalProperties: false,
};

const EVENT_SCHEMA = {
  type: "object",
  properties: {
    verdict: { type: "string", enum: ["obligation", "opportunity", "drop"] },
    why: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["verdict", "why", "confidence"],
  additionalProperties: false,
};

// `type: ["integer","null"]` rather than an `enum` carrying a null member: a mixed-type enum is
// outside the safe subset of JSON Schema for structured outputs, and the range is re-checked by
// `validate` anyway (it clamps 1-5), so nothing is lost by asking for it in prose instead.
const EMAIL_SCHEMA = {
  type: "object",
  properties: {
    tier: { type: "string", enum: ["task", "borderline", "event", "opportunity", "information"] },
    title: { type: "string" },
    course: { type: ["string", "null"] },
    due: { type: ["string", "null"] },
    effort_hours: { type: ["number", "null"] },
    importance: { type: ["integer", "null"] },
    why: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["tier", "title", "course", "due", "effort_hours", "importance", "why", "confidence"],
  additionalProperties: false,
};

function str(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function list(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
}

/**
 * The system prompt, with the per-account values left as placeholders.
 *
 * Split out from `buildPrompt` so `promptHash` can hash the TEMPLATE and mean it: `slice_hours` is
 * a per-account number, and rendering it into the hashed text would give two accounts different
 * system prompts under one hash — the one thing the hash exists to prevent (M1).
 */
export function systemTemplate(kind: "task" | "event" | "email"): string {
  if (kind === "task") {
    return [
      "You estimate effort and importance for one university assignment.",
      "Rules:",
      "- effort_hours: the realistic time in hours to finish this one assignment, 0.25 to 40.",
      "- a typical work session in this student's plan is {slice_hours} hours.",
      "- importance: 1 to 5, grounded in the grade weights given. With no weights given, answer 3 and say so in importance_reason.",
      "- importance_reason: one line, under 140 characters, no line breaks.",
      "- course: the slug given below, or null when the slug given is null.",
      "- confidence: 0 to 1, how sure you are of effort_hours and importance.",
    ].join("\n");
  }
  if (kind === "event") {
    return [
      "You decide whether one campus event is worth a student's attention.",
      "Rules:",
      "- verdict: obligation (the student is expected there), opportunity (worth offering), or drop.",
      "- an event aimed at faculty, staff, alumni or graduate students is a drop.",
      "- a standing exhibit, an office-hours block or a recurring drop-in is a drop.",
      "- why: one line, under 140 characters, no line breaks, no double quotes.",
      "- confidence: 0 to 1.",
    ].join("\n");
  }
  return [
    "You triage one email for a university student, into exactly one of five tiers.",
    "Tiers:",
    "- task: it clearly creates work with a deadline the student must do.",
    "- borderline: it might create work; a human should decide.",
    "- event: it announces something happening at a stated date and time.",
    "- opportunity: an application, a scholarship, a job, a research post, a dinner worth offering.",
    "- information: everything else, including receipts, newsletters, notifications and marketing.",
    "Rules:",
    "- title: what the resulting task or card should be called, one line, under 200 characters.",
    "- due: YYYY-MM-DD or YYYY-MM-DDTHH:MM when the email states one, else null.",
    "- effort_hours and importance: only for tier task, else null. importance is a whole number 1 to 5.",
    "- course: one of the known course slugs given below, or null.",
    "- why: one line, under 140 characters, no line breaks, no double quotes.",
    "- confidence: 0 to 1.",
  ].join("\n");
}

export function schemaFor(kind: "task" | "event" | "email"): Record<string, unknown> {
  return kind === "task" ? TASK_SCHEMA : kind === "event" ? EVENT_SCHEMA : EMAIL_SCHEMA;
}

export function buildPrompt(
  kind: "task" | "event" | "email",
  item: Record<string, unknown>,
  seed: Record<string, unknown>,
): Prompt {
  const schema = schemaFor(kind);
  if (kind === "task") {
    const slice = typeof seed.slice_hours === "number" ? seed.slice_hours : 1.5;
    const system = systemTemplate("task").replace("{slice_hours}", String(slice));
    const parts: string[] = [];
    const prefs = clip(str(seed.preferences), MAX_PREFS_CHARS);
    if (prefs !== "") parts.push(`The student's stated preferences:\n${prefs}`);
    parts.push(`Course slug: ${str(seed.course) || "null"}`);
    const weights = clip(str(seed.weights), MAX_WEIGHTS_CHARS);
    if (weights !== "") parts.push(`Grade weights:\n${weights}`);
    parts.push(`Title: ${oneLine(str(item.title), 200)}`);
    // An absent due date is absent, never the word "None": a model shown "Due: None" reliably
    // treats it as a date it failed to read rather than as an assignment without one.
    if (str(item.due) !== "") parts.push(`Due: ${str(item.due)}`);
    const body = clip(str(item.body).trim(), MAX_BODY_CHARS);
    if (body !== "") parts.push(`Body:\n${body}`);
    return { system, user: parts.join("\n"), schema };
  }

  if (kind === "event") {
    const parts = [
      `Title: ${oneLine(str(item.title), 200)}`,
      `When: ${str(item.start)} to ${str(item.end)}`,
      `Source: ${str(item.source)}`,
    ];
    if (str(item.organizer) !== "") parts.push(`Organizer: ${oneLine(str(item.organizer), 120)}`);
    if (str(item.location) !== "") parts.push(`Location: ${oneLine(str(item.location), 120)}`);
    if (list(item.categories).length > 0) parts.push(`Categories: ${list(item.categories).join(", ")}`);
    if (list(item.audiences).length > 0) parts.push(`Audiences: ${list(item.audiences).join(", ")}`);
    const description = clip(str(item.description).trim(), MAX_BODY_CHARS);
    if (description !== "") parts.push(`Description:\n${description}`);
    const interests = clip(str(seed.interests), MAX_PREFS_CHARS);
    if (interests !== "") parts.push(`The student's stated interests:\n${interests}`);
    return { system: systemTemplate("event"), user: parts.join("\n"), schema };
  }

  const known = list(seed.known_courses);
  const parts = [
    `Subject: ${oneLine(str(item.subject), 200)}`,
    `From: ${oneLine(str(item.from), 200)}`,
    `Date: ${str(item.date)}`,
  ];
  if (known.length > 0) parts.push(`Known course slugs: ${known.join(", ")}`);
  const text = clip(str(item.text).trim(), MAX_BODY_CHARS);
  if (text !== "") parts.push(`Message:\n${text}`);
  return { system: systemTemplate("email"), user: parts.join("\n"), schema };
}

/**
 * The prompt hash logged on every judgment (§5.4 measure 4).
 *
 * It hashes the **template** (placeholders unrendered) and the **schema**, and never the item or
 * any per-account value. Reproducibility is fully served: `model` + `prompt_version` +
 * `grammar_version` + `prompt_hash` names exactly which prompt shape produced a row, and the item
 * can be rebuilt from the note. Hashing the rendered prompt instead would put a commitment to a
 * note's body in a table §5.6 says may never hold one — and would give two accounts with different
 * planner slices two different hashes for one prompt.
 */
export async function promptHash(kind: "task" | "event" | "email"): Promise<string> {
  const bytes = new TextEncoder().encode(systemTemplate(kind) + "\u0000" + JSON.stringify(schemaFor(kind)));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
```

- [ ] **Step 9: Write `judge_caps.ts`, `judge_log.ts`, `judge_models.ts` and `judge_rules.ts`.**

```ts
// cloud/supabase/functions/_shared/judge_caps.ts
import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

export interface CapStore {
  charge(account: string, kind: Kind): Promise<boolean>;
  withinBudget(account: string): Promise<boolean>;
  recordTokens(account: string, kind: Kind, inTokens: number, outTokens: number): Promise<void>;
}

/**
 * §5.2's rate guards — **and the price they imply, worked out here so the next reader can check it.**
 *
 *   Haiku 4.5 is $1.00 / $5.00 per MTok (claude-api skill, 2026-09-09).
 *   This plan's own bounds: system ~280 tokens; user up to ~700 (body 1200 chars + weights 600 +
 *   preferences 600 + title 200, at ~4 chars/token); `max_tokens` 256 for task and event, 640 for
 *   email, of which a real reply uses ~80.
 *   So one call is about (980 / 1e6 * $1.00) + (80 / 1e6 * $5.00) = **$0.0014**, and email's
 *   longer cap does not change that because the cap is not what is billed.
 *
 * A plausible heavy day is ~10 enrichments, ~20 event verdicts, ~30 emails = 60 calls = $0.084,
 * i.e. **~$2.50 a month** against a $9.99 subscription. The caps below are ~4x that heavy day, so
 * a runaway loop costs at most ~$0.36 a day and ~$11 a month — still above the subscription, which
 * is why `MONTHLY_CEILING_USD` exists and is enforced rather than merely reported.
 *
 * (The earlier draft's 200/300/500 permitted 1,000 calls a day, about $45 a month per account —
 * 4.5x the price of the product. Caps and price are chosen together from here on.)
 */
export const DAILY_CAP: Record<Kind, number> = { task: 60, event: 80, email: 120 };

/**
 * The enforced monthly ceiling per account, in dollars. ~3x plausible heavy use, and about a
 * quarter of the subscription — a number the business can absorb for every account at once. Past
 * it the judgment is refused with outcome `capped` and one row lands in `budget_alerts`, which is
 * what makes the overspend visible without querying a view nobody queries.
 */
export const MONTHLY_CEILING_USD = 7.5;

export function capStore(db: Db): CapStore {
  // One budget check per account per invocation is enough: an edge function handles one request,
  // and `gmail-read` (Task 11) is the only caller that judges many items in one — it memoises.
  const budget = new Map<string, boolean>();
  return {
    async charge(account, kind) {
      // One statement (`charge_call`), so two concurrent calls cannot both read `calls` below the
      // cap and both write. A cap store that cannot be reached REFUSES: over-charging a student's
      // account is recoverable, an uncapped loop against a metered API is not.
      try {
        return await db.rpc("charge_call", { p_account: account, p_kind: kind, p_cap: DAILY_CAP[kind] }) === true;
      } catch {
        return false;
      }
    },
    async withinBudget(account) {
      const cached = budget.get(account);
      if (cached !== undefined) return cached;
      let ok = false;
      try {
        ok = await db.rpc("enforce_budget", { p_account: account, p_ceiling: MONTHLY_CEILING_USD }) === true;
      } catch {
        ok = false;
      }
      budget.set(account, ok);
      return ok;
    },
    async recordTokens(account, kind, inTokens, outTokens) {
      // Never throws: the tokens are the bill's record, and a failure to write them must not lose
      // a judgment that already happened.
      try {
        await db.rpc("record_tokens", {
          p_account: account, p_kind: kind, p_in: inTokens, p_out: outTokens,
        });
      } catch {
        // Counted nowhere on purpose — the next call's `enforce_budget` reads the same table and
        // will still see every call that did record.
      }
    },
  };
}
```

```ts
// cloud/supabase/functions/_shared/judge_log.ts
import type { Db } from "./judge_db.ts";
import type { Cause, Kind } from "./judge_validate.ts";

/// One judgment, ready to record. **There is nowhere in this type to put a title, a body, a prompt
/// or a reply**, and that is the design (§5.6): the privacy property is structural rather than a
/// matter of care at each call site. `fields` is field name -> the literal that was (or would have
/// been) written, plus the four promotion features tier 2 is keyed on.
export interface JudgmentRow {
  account_id: string;
  kind: Kind;
  item_id: string;
  tier: number;
  outcome: string;
  cause: Cause | null;
  confidence: number;
  fields: Record<string, string>;
  model: string | null;
  prompt_version: string | null;
  grammar_version: string | null;
  prompt_hash: string | null;
  ms: number;
  origin: "device" | "gmail_api" | "events";
}

export interface JudgmentSink {
  /** The new row's id, so the reply can carry it and a later correction can name it. */
  write(row: JudgmentRow): Promise<string | null>;
}

export function judgmentSink(db: Db): JudgmentSink {
  return {
    async write(row) {
      // A log that cannot be written must not stop the judgment: the judgment is the work and the
      // row is the record of it. The error is reported, never thrown, and never carries the row.
      try {
        const written = await db.insert("judgments", row as unknown as Record<string, unknown>);
        return typeof written?.id === "string" ? written.id : null;
      } catch (e) {
        console.error(`judgments insert failed: ${e instanceof Error ? e.message : "unknown"}`);
        return null;
      }
    },
  };
}
```

```ts
// cloud/supabase/functions/_shared/judge_models.ts
import type { Sampling } from "./judge_anthropic.ts";
import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

/// The pin. Changing one is a migration row with a date, so every historical judgment names the
/// model that made it — and carries that model's own sampling rules and price (§5.2, §5.4 m4).
export interface ModelRow {
  kind: Kind;
  provider: string;
  model_id: string;
  prompt_version: string;
  grammar_version: string;
  max_tokens: number;
  sampling: Sampling;
  usd_per_m_in: number;
  usd_per_m_out: number;
}

/// There is no default and no fallback: a missing row is a deployment error, and answering with an
/// unpinned model would put a judgment in the log that names a model nobody chose.
export async function modelRow(db: Db, kind: Kind): Promise<ModelRow> {
  const rows = await db.select(
    `models?kind=eq.${kind}&select=kind,provider,model_id,prompt_version,grammar_version,max_tokens,sampling,usd_per_m_in,usd_per_m_out`,
  );
  if (rows.length !== 1) throw new Error(`no pinned model for kind '${kind}'`);
  return rows[0] as ModelRow;
}
```

```ts
// cloud/supabase/functions/_shared/judge_rules.ts
import type { Db } from "./judge_db.ts";
import type { Kind } from "./judge_validate.ts";

/// Tier 2. A `null` means "no rule answered"; the pipeline then reaches the model.
export interface RuleTable {
  lookup(account: string, kind: Kind, item: Record<string, unknown>): Promise<Record<string, unknown> | null>;
}

function s(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

/// The first three words of a title. **The promotion key, and the only part of a title that ever
/// leaves this function** — it is already the note's filename, so it is not new exposure.
export function titlePrefix(title: string): string {
  return s(title).split(/\s+/).slice(0, 3).join(" ");
}

/// The promotion features of §5.4 measure 1, extracted from an item, most specific first. Shared
/// with the nightly promotion job (through `judgments.fields`, which `featureMap` fills) so a rule
/// is always looked up by the same key it was promoted on.
export function features(kind: Kind, item: Record<string, unknown>): Array<[string, string]> {
  const prefix = titlePrefix(s(item.title));
  if (kind === "task") {
    const out: Array<[string, string]> = [];
    if (s(item.created_by) !== "" && prefix !== "") {
      out.push(["created_by+title_prefix", `${s(item.created_by)}|${prefix}`]);
    }
    if (prefix !== "") out.push(["title_prefix", prefix]);
    return out;
  }
  const out: Array<[string, string]> = [];
  if (s(item.organizer) !== "") out.push(["organizer", s(item.organizer)]);
  if (s(item.source) !== "") out.push(["source", s(item.source)]);
  if (s(item.series_uid) !== "") out.push(["series", s(item.series_uid)]);
  if (prefix !== "") out.push(["title_prefix", prefix]);
  return out;
}

/// The same five values, flat, for `judgments.fields` — which is where `promote_rules` reads them
/// from, because the row carries no title and no body to recompute them from.
export function featureMap(kind: Kind, item: Record<string, unknown>): Record<string, string> {
  const out: Record<string, string> = {};
  const prefix = titlePrefix(s(item.title));
  if (prefix !== "") out.title_prefix = prefix;
  if (kind === "task") {
    if (s(item.created_by) !== "") out.created_by = s(item.created_by);
    return out;
  }
  if (s(item.organizer) !== "") out.organizer = s(item.organizer);
  if (s(item.source) !== "") out.source = s(item.source);
  if (s(item.series_uid) !== "") out.series_uid = s(item.series_uid);
  return out;
}

/// The account's own active rules, then the hand-reviewed global ones (§11 R5). First feature that
/// matches wins, in the order `features` returns them — most specific first.
export function ruleTable(db: Db): RuleTable {
  return {
    async lookup(account, kind, item) {
      const pairs = features(kind, item);
      if (pairs.length === 0) return null;
      let rows: Array<{ feature: string; value: string; verdict: Record<string, unknown>; account_id: string | null }>;
      try {
        rows = await db.select(
          `rules?kind=eq.${kind}&active=is.true&or=(account_id.eq.${account},account_id.is.null)&select=feature,value,verdict,account_id`,
        ) as typeof rows;
      } catch {
        // A rule table that cannot be read is not an error: the model answers instead, which is
        // what it did before any rule existed.
        return null;
      }
      for (const [feature, value] of pairs) {
        const own = rows.find((r) => r.feature === feature && r.value === value && r.account_id === account);
        if (own !== undefined) return own.verdict;
        const global = rows.find((r) => r.feature === feature && r.value === value && r.account_id === null);
        if (global !== undefined) return global.verdict;
      }
      return null;
    },
  };
}
```

- [ ] **Step 10: Write `judge_pipeline.ts`.**

```ts
// The pipeline of cloud design §5.2, and the only place the order of those steps is written down:
//
//   rules (tier 2, free) -> monthly budget -> daily cap -> constrained call (tier 3)
//                        -> validate -> record tokens -> log -> reply
//
// **Rules come before the cap on purpose.** "Rules retire model calls" (§5.4 measure 1) and "a
// rule still spends the account's model allowance" cannot both be true; the first one is the
// design, and it is also what makes promotion worth having on a capped account.
//
// Everything the pipeline needs arrives as a parameter, so the whole of it is exercised by
// `deno test` with a scripted model, an in-memory sink and no database at all. The handlers on top
// of it are thin by design: a handler that computed anything would be a second place to fix a bug.
import type { JudgeModel } from "./judge_anthropic.ts";
import { ModelRefused } from "./judge_anthropic.ts";
import { type CapStore, DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";
import type { JudgmentRow, JudgmentSink } from "./judge_log.ts";
import type { ModelRow } from "./judge_models.ts";
import { buildPrompt, promptHash } from "./judge_prompts.ts";
import { featureMap, type RuleTable } from "./judge_rules.ts";
import { type Cause, CONFIDENCE_FLOOR, type Kind, validate } from "./judge_validate.ts";

// One import site for a handler and one for a test. Nothing under `_shared/` imports THIS module:
// an ESM cycle whose value export is read at module scope is a temporal-dead-zone crash at deploy
// time, not a compile error.
export { CONFIDENCE_FLOOR, DAILY_CAP, MONTHLY_CEILING_USD };
export type { CapStore, Cause, JudgmentRow, JudgmentSink, Kind, ModelRow, RuleTable };

export interface JudgeRequest {
  kind: Kind;
  item: Record<string, unknown>;
  heuristics_seed: Record<string, unknown>;
}

export interface JudgeReply {
  verdict: Record<string, unknown> | null;
  tier: 0 | 2 | 3;
  outcome: "answered" | "low confidence" | "capped";
  cause?: Cause;
  model?: string;
  prompt_version?: string;
  grammar_version?: string;
  /** The `judgments` row this reply came from, so a later correction can name it. */
  judgment_id?: string;
}

export interface PipelineDeps {
  row: ModelRow;
  model: JudgeModel;
  rules: RuleTable;
  caps: CapStore;
  log: JudgmentSink;
  origin: JudgmentRow["origin"];
  now: () => number;
}

/**
 * What goes in `judgments.fields`: the verdict's own field values, plus the promotion features.
 *
 * Both halves are keys or values the note already carries in the open — a course slug, an hour
 * figure, an importance, a three-word title prefix that is the note's filename. The free-text
 * fields (`importance_reason`, `why`, `title`) are dropped here, and `a_body_token_reaches_no_
 * judgment_row` is what keeps that true.
 */
export function fieldsOf(
  verdict: Record<string, unknown> | null,
  kind: Kind,
  item: Record<string, unknown>,
): Record<string, string> {
  const out: Record<string, string> = featureMap(kind, item);
  if (verdict === null) return out;
  for (const [name, value] of Object.entries(verdict)) {
    if (["confidence", "why", "importance_reason", "title"].includes(name)) continue;
    out[name] = value === null ? "null" : String(value);
  }
  return out;
}

function itemId(item: Record<string, unknown>): string {
  for (const key of ["id", "uid", "message_id"]) {
    const value = item[key];
    if (typeof value === "string" && value !== "") return value;
  }
  return "";
}

export async function judge(
  accountId: string,
  req: JudgeRequest,
  deps: PipelineDeps,
): Promise<JudgeReply> {
  const started = deps.now();
  const base = {
    account_id: accountId, kind: req.kind, item_id: itemId(req.item),
    model: null, prompt_version: null, grammar_version: null, prompt_hash: null,
    origin: deps.origin,
  };
  const fields = (v: Record<string, unknown> | null) => fieldsOf(v, req.kind, req.item);

  // Tier 2 first, and free.
  const rule = await deps.rules.lookup(accountId, req.kind, req.item);
  if (rule !== null) {
    const checked = validate(req.kind, rule, req.heuristics_seed);
    if (checked.ok && checked.verdict !== undefined) {
      const id = await deps.log.write({
        ...base, tier: 2, outcome: "answered", cause: null,
        confidence: Number(checked.verdict.confidence ?? 1),
        fields: fields(checked.verdict), ms: deps.now() - started,
      });
      return { verdict: checked.verdict, tier: 2, outcome: "answered", ...(id === null ? {} : { judgment_id: id }) };
    }
    // A rule that no longer validates is a rule that has gone stale — fall through to the model
    // rather than answering with it, and let the promotion job's own evidence retire it.
  }

  // The monthly ceiling before the daily cap: an account over budget must not even spend its
  // allowance, and `withinBudget` is memoised per invocation so this is one query, not sixty.
  if (!await deps.caps.withinBudget(accountId)) {
    await deps.log.write({ ...base, tier: 0, outcome: "capped", cause: null, confidence: 0, fields: {}, ms: deps.now() - started });
    return { verdict: null, tier: 0, outcome: "capped" };
  }
  if (!await deps.caps.charge(accountId, req.kind)) {
    await deps.log.write({ ...base, tier: 0, outcome: "capped", cause: null, confidence: 0, fields: {}, ms: deps.now() - started });
    return { verdict: null, tier: 0, outcome: "capped" };
  }

  const prompt = buildPrompt(req.kind, req.item, req.heuristics_seed);
  const pinned = {
    model: deps.row.model_id,
    prompt_version: deps.row.prompt_version,
    grammar_version: deps.row.grammar_version,
    prompt_hash: await promptHash(req.kind),
  };

  let answer;
  try {
    answer = await deps.model.complete({
      model: deps.row.model_id,
      system: prompt.system,
      user: prompt.user,
      schema: prompt.schema,
      maxTokens: deps.row.max_tokens,
      sampling: deps.row.sampling,
    });
  } catch (e) {
    // The failure text is deliberately not carried into the row: §5.6 forbids a body in the log,
    // and a provider's error string is the one place a prompt can come back out. The CAUSE is
    // structural and that is what a later reader needs (ruling R-3a-20) — and the three kinds of
    // unusable answer are three different faults with three different fixes.
    const cause: Cause = e instanceof ModelRefused
      ? "refused"
      : e instanceof Error && e.message.includes("max_tokens")
      ? "truncated"
      : "model failed";
    const id = await deps.log.write({ ...base, ...pinned, tier: 3, outcome: "low confidence", cause, confidence: 0, fields: {}, ms: deps.now() - started });
    return { verdict: null, tier: 3, outcome: "low confidence", cause, ...pinned, ...(id === null ? {} : { judgment_id: id }) };
  }

  // Recorded whatever the verdict turns out to be: the tokens were spent either way, and the
  // monthly budget is only as honest as this line.
  await deps.caps.recordTokens(accountId, req.kind, answer.inputTokens, answer.outputTokens);

  const checked = validate(req.kind, answer.json, req.heuristics_seed);
  if (!checked.ok || checked.verdict === undefined) {
    const cause = checked.cause ?? "incomplete";
    const id = await deps.log.write({ ...base, ...pinned, tier: 3, outcome: "low confidence", cause, confidence: Number(answer.json.confidence ?? 0), fields: {}, ms: deps.now() - started });
    return { verdict: null, tier: 3, outcome: "low confidence", cause, ...pinned, ...(id === null ? {} : { judgment_id: id }) };
  }

  const id = await deps.log.write({
    ...base, ...pinned, tier: 3, outcome: "answered", cause: null,
    confidence: Number(checked.verdict.confidence ?? 0),
    fields: fields(checked.verdict), ms: deps.now() - started,
  });
  return { verdict: checked.verdict, tier: 3, outcome: "answered", ...pinned, ...(id === null ? {} : { judgment_id: id }) };
}
```

- [ ] **Step 11: Write the account-scoping guard test** — `cloud/supabase/functions/_shared/judge_db_test.ts`. Every C2 table's access control *is* the `account_id` filter, because the service role bypasses RLS; one missing filter is a cross-account read with no database backstop, and no type can catch it.

```ts
import { assert } from "@std/assert";

const SHARED = new URL(".", import.meta.url);
/** Tables whose every row belongs to one account. `models` is global and is not one. */
const SCOPED = ["judgments", "usage_daily", "rules", "budget_alerts", "corrections", "gmail_seen", "gmail_queue", "google_accounts", "sources"];

Deno.test("every read of an account-scoped table names an account", async () => {
  for await (const entry of Deno.readDir(SHARED)) {
    if (!entry.name.endsWith(".ts") || entry.name.endsWith("_test.ts")) continue;
    const source = await Deno.readTextFile(new URL(entry.name, SHARED));
    // Matches `db.select(`, `sharedDb().select(` and `client.select(` alike — the call, not the
    // receiver, because the receiver's name is the one thing a refactor changes.
    for (const m of source.matchAll(/\.select\(\s*`([^`]+)`/g)) {
      const path = m[1];
      const table = path.split("?")[0].trim();
      if (!SCOPED.includes(table)) continue;
      assert(
        path.includes("account_id=eq.") || path.includes("account_id.eq."),
        `${entry.name}: a select on '${table}' with no account_id filter. The service role bypasses ` +
          `RLS, so this filter is the whole access control (Task 2 step 7).`,
      );
    }
  }
});
```

Every function directory's `index.ts` gets the same test in Task 15's close, over `cloud/supabase/functions/*/`.

- [ ] **Step 12: Run the whole suite and watch it pass.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
Expected: `ok | 27 passed | 0 failed` (6 from Task 0, 5 from Task 1, 7 from `judge_validate_test.ts`, 8 from `judge_pipeline_test.ts`, 1 from `judge_db_test.ts` — count what you actually get and record it in this step's line).
Then: `deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts`, `deno lint --config cloud/supabase/deno.json cloud/supabase/` → clean, and `deno fmt --config cloud/supabase/deno.json cloud/supabase/` with whatever it writes committed.

- [ ] **Step 13: Commit.**

```bash
git add cloud/supabase/functions/_shared/judge_db.ts cloud/supabase/functions/_shared/judge_db_test.ts cloud/supabase/functions/_shared/judge_prompts.ts cloud/supabase/functions/_shared/judge_validate.ts cloud/supabase/functions/_shared/judge_validate_test.ts cloud/supabase/functions/_shared/judge_rules.ts cloud/supabase/functions/_shared/judge_caps.ts cloud/supabase/functions/_shared/judge_log.ts cloud/supabase/functions/_shared/judge_models.ts cloud/supabase/functions/_shared/judge_pipeline.ts cloud/supabase/functions/_shared/judge_pipeline_test.ts
git commit -F .git-commit-msg.txt   # "cloud: one pipeline for all three judgments — rules, budget, cap, one constrained call, validate, log (C2 Task 2)"
```


---

### Task 3: `/judge-task` — the first endpoint

A thin handler over Task 2's pipeline, with C1's entitlement check first and one request/response contract test that needs no Supabase project.

**Files:**
- Create: `cloud/supabase/functions/_shared/judge_handler.ts`, `cloud/supabase/functions/_shared/judge_deps.ts`, `cloud/supabase/functions/judge-task/index.ts`
- Test: `cloud/supabase/functions/_shared/judge_handler_test.ts`

**Interfaces:**
- Consumes: Task 2's `judge`, `PipelineDeps`; C1's `requireActiveEntitlement`.
- Produces: `type Entitle = (req: Request) => Promise<{ account_id: string }>`; `judgeHandler(kind: Kind, entitle: Entitle, deps: (kind: Kind) => Promise<PipelineDeps>): (req: Request) => Promise<Response>`; `liveDeps(kind: Kind, origin?: JudgmentRow["origin"]): Promise<PipelineDeps>`.

**Every function directory in this plan has the same shape**: an `index.ts` of five lines that imports C1's `_shared/entitlement.ts`, builds the live dependencies and calls `Deno.serve`, over a handler that lives in `_shared/judge_*.ts` and takes every dependency as a parameter. **The handler is `_shared/`, not a function directory** (ruling R-X-5 gives C2 the `judge_` prefix there): eight of the ten functions import it, and importing it from `judge-task/` would bundle that directory into all eight.

**`deno check` on any `index.ts` needs C1's `_shared/entitlement.ts` to be on disk.** Until C1 merges, `deno check` the `_shared/judge_*.ts` files and skip the `index.ts` files, and note in the task report that H8's `cloud` job goes green only after the C1 merge. C1 merges first, so this resolves itself.

- [ ] **Step 1: Write the failing contract test** — `cloud/supabase/functions/_shared/judge_handler_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { ScriptedModel } from "./judge_anthropic.ts";
import { judgeHandler } from "./judge_handler.ts";
import type { CapStore, JudgmentRow, JudgmentSink, ModelRow, PipelineDeps, RuleTable } from "./judge_pipeline.ts";

const ROW: ModelRow = {
  kind: "task", provider: "anthropic", model_id: "claude-haiku-4-5",
  prompt_version: "task-1", grammar_version: "task-1", max_tokens: 256,
  sampling: { temperature: 0 }, usd_per_m_in: 1.0, usd_per_m_out: 5.0,
};
const NO_RULES: RuleTable = { lookup: () => Promise.resolve(null) };
const ALWAYS: CapStore = {
  charge: () => Promise.resolve(true),
  withinBudget: () => Promise.resolve(true),
  recordTokens: () => Promise.resolve(),
};
const SINK: JudgmentSink = { write: (_row: JudgmentRow) => Promise.resolve("judgment-1") };

function deps(model: ScriptedModel): () => Promise<PipelineDeps> {
  return () => Promise.resolve({
    row: ROW, model, rules: NO_RULES, caps: ALWAYS, log: SINK, origin: "device", now: () => 0,
  });
}

const BODY = JSON.stringify({
  kind: "task",
  item: {
    id: "task-abc123", title: "CS 100 HW 01", body: "sections", source_uid: "zybooks:1",
    created_by: "zybooks", course: null, due: "2026-09-18T23:59",
  },
  heuristics_seed: {
    course: null, effort_hours: null, slice_hours: 1.5,
    weights: "Homework 20%", preferences: "", known_courses: ["cs-100"],
  },
});

const OK = () => Promise.resolve({ account_id: "acct-1" });
const ANSWER = {
  course: "cs-100", effort_hours: 2.5, importance: 4,
  importance_reason: "20% of the grade", confidence: 0.82,
};

Deno.test("a judged task comes back as a verdict, a tier, the model, and the judgment id", async () => {
  const handler = judgeHandler("task", OK, deps(new ScriptedModel([ANSWER])));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assertEquals(reply.tier, 3);
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.verdict.effort_hours, 2.5);
  assertEquals(reply.model, "claude-haiku-4-5");
  assertEquals(reply.judgment_id, "judgment-1");
});

Deno.test("the entitlement check runs before anything is parsed or charged", async () => {
  const model = new ScriptedModel([]);
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = judgeHandler("task", refuse, deps(model));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 402);
  assertEquals(model.seen.length, 0);
});

Deno.test("a body for the wrong kind is a 400 and never reaches the model", async () => {
  const model = new ScriptedModel([]);
  const handler = judgeHandler("task", OK, deps(model));
  const wrong = JSON.stringify({ kind: "event", item: {}, heuristics_seed: {} });
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: wrong }));
  assertEquals(response.status, 400);
  assertEquals(model.seen.length, 0);
});

Deno.test("a GET is a 405, so a browser cannot spend an account's cap by visiting the URL", async () => {
  const handler = judgeHandler("task", OK, deps(new ScriptedModel([])));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "GET" }));
  assertEquals(response.status, 405);
});

Deno.test("a thrown error is a 500 that names nothing from the request", async () => {
  const handler = judgeHandler("task", OK, () => Promise.reject(new Error("connect ECONNREFUSED TRIPWIRE-9f2c")));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 500);
  const text = await response.text();
  assertEquals(text.includes("TRIPWIRE-9f2c"), false);
  assertEquals(text.includes("CS 100 HW 01"), false);
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/_shared/judge_handler_test.ts`
Expected: FAIL — `Module not found "…/judge_handler.ts"`.

- [ ] **Step 3: Write `cloud/supabase/functions/_shared/judge_handler.ts`.**

```ts
// The one handler all three `/judge-*` functions are. It computes nothing: it checks the
// entitlement, reads the body, hands it to the one pipeline and returns what came back. A handler
// that computed anything would be a second place to fix a bug, and there are three of these.
import { judge, type JudgeRequest, type Kind, type PipelineDeps } from "./judge_pipeline.ts";

export type Entitle = (req: Request) => Promise<{ account_id: string }>;

export function judgeHandler(
  kind: Kind,
  entitle: Entitle,
  deps: (kind: Kind) => Promise<PipelineDeps>,
): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") {
      return Response.json({ error: "POST only" }, { status: 405 });
    }
    try {
      const { account_id } = await entitle(req);
      let body: JudgeRequest;
      try {
        body = await req.json() as JudgeRequest;
      } catch {
        return Response.json({ error: "the body is not JSON" }, { status: 400 });
      }
      if (body?.kind !== kind || typeof body.item !== "object" || body.item === null) {
        return Response.json({ error: `expected kind '${kind}' and an item object` }, { status: 400 });
      }
      const seed = typeof body.heuristics_seed === "object" && body.heuristics_seed !== null
        ? body.heuristics_seed
        : {};
      const reply = await judge(account_id, { kind, item: body.item, heuristics_seed: seed }, await deps(kind));
      return Response.json(reply);
    } catch (e) {
      // `requireActiveEntitlement` throws a Response (401 / 402); everything else is ours.
      if (e instanceof Response) return e;
      // The class, never the message: a provider's error string and a connection error's text are
      // the two places a request body can come back out, and this line reaches a log (§5.6).
      console.error(`judge-${kind}: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "judgment failed" }, { status: 500 });
    }
  };
}
```

- [ ] **Step 4: Write `cloud/supabase/functions/_shared/judge_deps.ts`.**

```ts
// The live wiring: a PostgREST client with the service role, the pinned model row, the account's
// rules, the cap store, the judgment sink. Split out of the handlers so every handler test can
// build the same `PipelineDeps` from fakes and never reach a project.
import { AnthropicModel } from "./judge_anthropic.ts";
import { capStore } from "./judge_caps.ts";
import { type Db, serviceDb } from "./judge_db.ts";
import { judgmentSink } from "./judge_log.ts";
import { modelRow } from "./judge_models.ts";
import type { JudgmentRow, Kind, PipelineDeps } from "./judge_pipeline.ts";
import { ruleTable } from "./judge_rules.ts";

/// Built at first use, never at module scope: a throw at module scope is a boot failure with an
/// opaque message, and every other failure in this codebase is a named status.
let db: Db | null = null;
export function sharedDb(): Db {
  if (db === null) db = serviceDb();
  return db;
}

export async function liveDeps(
  kind: Kind,
  origin: JudgmentRow["origin"] = "device",
): Promise<PipelineDeps> {
  const apiKey = Deno.env.get("ANTHROPIC_API_KEY");
  if (apiKey === undefined || apiKey === "") throw new Error("the function is missing ANTHROPIC_API_KEY");
  const client = sharedDb();
  return {
    row: await modelRow(client, kind),
    model: new AnthropicModel({ apiKey }),
    rules: ruleTable(client),
    caps: capStore(client),
    log: judgmentSink(client),
    origin,
    now: () => Date.now(),
  };
}
```

- [ ] **Step 5: Write `cloud/supabase/functions/judge-task/index.ts`.**

```ts
// The entry point. Five lines, and the only file in this directory — it imports C1's entitlement
// module and the shared handler, and reaches a project through neither of its own.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("task", requireActiveEntitlement, (kind) => liveDeps(kind)));
```

- [ ] **Step 6: Run it and watch it pass.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
Expected: the whole suite green, 5 more tests than at the end of Task 2.

- [ ] **Step 7: Deploy to staging and smoke it (needs P1).**

```
supabase functions deploy judge-task --project-ref <staging ref>
```

Then, with a session JWT for a staging test account in `$JWT` (produced by C1's sign-in against the staging project — never a production one):

```
curl -s -X POST "$SUPABASE_URL/functions/v1/judge-task" \
  -H "Authorization: Bearer $JWT" -H "apikey: $SUPABASE_ANON_KEY" -H "Content-Type: application/json" \
  -d '{"kind":"task","item":{"id":"task-smoke","title":"CS 100 HW 01","body":"25 zyBooks sections, 193 points.","source_uid":"zybooks:1","created_by":"zybooks","course":null,"due":"2026-09-18T23:59"},"heuristics_seed":{"course":"cs-100","effort_hours":null,"slice_hours":1.5,"weights":"Homework 20%\nExams 50%","preferences":"","known_courses":["cs-100"]}}'
```

Expected: HTTP 200, `tier` 3, `outcome` `answered`, `model` `claude-haiku-4-5`, a plausible `verdict.effort_hours`, and a `judgment_id`. **Note the URL: `functions/v1/judge-task`, hyphen, not `judge/task`** — Supabase routes `/functions/v1/<function-name>`, so a slash there reaches a function named `judge`, which nothing deploys, and the answer is a 404 the loopback tests would never catch.

Then confirm exactly one row landed, that it holds no note text, and that the tokens were recorded:

```
curl -s "$SUPABASE_URL/rest/v1/judgments?select=id,item_id,tier,outcome,model,prompt_hash,ms,fields&item_id=eq.task-smoke" \
  -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
curl -s "$SUPABASE_URL/rest/v1/usage_daily?select=calls,in_tokens,out_tokens&kind=eq.task" \
  -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
```

Expected: one judgment row, whose `fields` holds `course`, `effort_hours`, `importance`, `created_by` and `title_prefix` and **no** free text; `prompt_hash` is 64 hex characters; and `usage_daily` shows `calls: 1` with **non-zero** `in_tokens` and `out_tokens`. **Record the observed latency, the row's `ms`, and the two token counts in this step's line** — Task 14's budget arithmetic is a prediction until these are real numbers, and the caps in `judge_caps.ts` are set from them.

- [ ] **Step 8: Commit.**

```bash
git add cloud/supabase/functions/_shared/judge_handler.ts cloud/supabase/functions/_shared/judge_handler_test.ts cloud/supabase/functions/_shared/judge_deps.ts cloud/supabase/functions/judge-task/
git commit -F .git-commit-msg.txt   # "cloud: POST /judge-task — entitlement, rules, budget, cap, one constrained call, one logged row (C2 Task 3)"
```


---

### Task 4: `CloudModel`, and `knowlu-engine judge` using it

The device half of the first increment. When the vault has `config/cloud.yaml`, tier 3 is an HTTPS call; when it has none, plan 3a's local runtime is still there until C4; and every way this can go wrong is a named line on stdout and exit 0.

**Files:**
- Create: `engine/src/cloudmodel.rs`, `engine/tests/cloud_contract.rs`
- Modify: `engine/src/judge.rs` (add `Missing::Service`, `Outcome::ServiceUnavailable`, and the `EventModel` / `EmailModel` seams), `engine/src/enrich.rs` (`run_lines_with`), `engine/tests/dependency_boundary.rs` (one new test)
- **Hand-off this task needs:** H1 (`lib.rs`'s `pub mod cloudmodel;`).
- **Dependency on C1, not a hand-off** (ruling R-C2-3 / R-X-4): `app/src/scheduler.rs` is C1's file and C1's own plan adds `enum JudgePlan { Cloud { log_dir }, Local(JudgeArgs), Skip(&'static str) }`, so a vault with `config/cloud.yaml` runs `judge --vault <v> --via local-runner --log-dir <the profile's logs folder>` whatever is installed locally — **and entitlement outranks it**: past the 72-hour grace the step is a named `judge (skipped: no entitlement)` with exit code 0, which any C2 test that runs a slot must expect. **Without that C1 change the app never runs `judge` on a machine with no llama.cpp and nothing in this stream ever executes** — so this increment is not shippable until C1 has merged. C2 states the dependency (*Interfaces with C1*, contract 6) and writes none of the code.

**Interfaces:**
- Consumes: C1's `config/cloud.yaml` and the Credential Manager entry it names; Task 3's `/judge-task` reply shape.
- Produces:
  - `cloudmodel::CALL_TIMEOUT: Duration` (120 s)
  - `cloudmodel::CloudConfig { api_base, anon_key, session_credential_target, account_id }`, `cloudmodel::load(&Path) -> Option<CloudConfig>`
  - `cloudmodel::Unavailable { NoConfig, NoSession(String) }` with `label() -> &'static str`
  - `cloudmodel::CloudError { Transport(String), Status { code: u16, detail: String }, Body(String) }` with `label() -> &'static str` and `fatal() -> bool`
  - `cloudmodel::CloudClient::new(&CloudConfig, &str)`, `.post(&str, &Value) -> Result<Value, CloudError>`, `.get(&str) -> Result<Value, CloudError>`, `.account_id() -> &str`
  - `cloudmodel::resolve(&Path) -> Result<CloudClient, Unavailable>`
  - `cloudmodel::CloudModel<'a>::new(&'a CloudClient)`, `.fatal() -> Option<&'static str>`, `impl judge::Model`
  - `cloudmodel::task_request(&judge::Item, &judge::Heuristics, &judge::Verdict) -> serde_json::Value`
  - `judge::Missing::Service(&'static str)`, `judge::Outcome::ServiceUnavailable(Verdict)` (label `"service unavailable"`)
  - `judge::EventModel`, `judge::EventItem`, `judge::EventVerdict`, `judge::EmailModel`, `judge::EmailItem`, `judge::EmailVerdict` — declared here, implemented in Tasks 9 and 11
  - `enrich::run_lines_with(&Path, &Options, Option<&CloudClient>) -> (i32, Vec<String>)`

**`judgelog.rs` is deliberately not changed.** §5.4 measure 4 asks that every judgment row name the model, the prompt version, the grammar version and the prompt hash — and that row is the **server's** `judgments` table, written in Task 2. The device log's job (§5.6) is unchanged and narrower: "why is nothing being enriched", by id and field value. Duplicating the server's provenance into a per-profile file that never syncs would add a second place to keep true and no reader.

- [ ] **Step 1: Write the failing contract tests** — `engine/tests/cloud_contract.rs`:

```rust
//! What the engine actually sends the judgment service, and what it does with every answer.
//!
//! Every test here binds a `TcpListener` to `127.0.0.1:0`, serves its own request from a second
//! thread, and joins that thread before returning. That is not egress: no DNS, no route off the
//! machine, no listener on a routable interface (CLAUDE.md; plan 3a's loopback rule). No test
//! reads a real vault, and none touches Credential Manager — `CloudClient::new` takes the token,
//! so `resolve` is exercised only where it can fail without one.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use knowlu_engine::cloudmodel::{CloudClient, CloudConfig, CloudModel};
use knowlu_engine::judge::{self, Heuristics, Item, Model, Verdict};

/// A loopback server that answers `replies` in order and hands back everything it was sent.
struct Loopback {
    base: String,
    handle: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Loopback {
    fn requests(&mut self) -> Vec<String> {
        self.handle.take().expect("joined once").join().expect("the listener thread did not panic")
    }
}

fn read_request(stream: &std::net::TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
    let mut head = String::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = rest.trim().parse().unwrap_or(0);
        }
        let blank = line == "\r\n" || line == "\n";
        head.push_str(&line);
        if blank {
            break;
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        let _ = reader.read_exact(&mut body);
    }
    format!("{head}{}", String::from_utf8_lossy(&body))
}

fn loopback(replies: Vec<(u16, String)>) -> Loopback {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
    let port = listener.local_addr().expect("the listener has an address").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (code, body) in replies {
            let Ok((mut stream, _)) = listener.accept() else { break };
            seen.push(read_request(&stream));
            let response = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    Loopback { base: format!("http://127.0.0.1:{port}/functions/v1"), handle: Some(handle) }
}

fn config(base: &str) -> CloudConfig {
    CloudConfig {
        api_base: base.to_string(),
        anon_key: "anon-not-a-secret".to_string(),
        session_credential_target: "knowlu/test-profile/session".to_string(),
        account_id: "acct-1".to_string(),
    }
}

fn item() -> Item {
    Item {
        id: "task-abc123".to_string(),
        rel_path: "tasks/cs-100-hw-01.md".to_string(),
        title: "CS 100 HW 01".to_string(),
        body: "25 zyBooks sections. TRIPWIRE-9f2c.".to_string(),
        source_uid: "zybooks:1839992".to_string(),
        created_by: "zybooks".to_string(),
        course: None,
        due: Some("2026-09-18T23:59".to_string()),
        effort_hours: 1.0,
        effort_source: "inferred".to_string(),
    }
}

fn heuristics() -> Heuristics {
    let mut weights = std::collections::BTreeMap::new();
    weights.insert("cs-100".to_string(), "Homework 20%".to_string());
    Heuristics {
        course_map: vec![("CS-100".to_string(), "cs-100".to_string())],
        slice_hours: 1.5,
        weights,
        preferences: "I work best in the morning.".to_string(),
    }
}

const ANSWERED: &str = r#"{"verdict":{"course":"cs-100","effort_hours":900,"importance":9,"importance_reason":"twenty percent of the grade","confidence":0.82},"tier":3,"outcome":"answered","model":"claude-haiku-4-5","prompt_version":"task-1","grammar_version":"task-1"}"#;

#[test]
fn the_task_request_carries_the_note_and_its_grounding_and_nothing_else() {
    let mut server = loopback(vec![(200, ANSWERED.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let _ = model.judge(&item(), &heuristics(), &seed);
    let sent = server.requests().remove(0);

    // Plan 3a fidelity row R6, moved to the boundary that now exists: the request carries the
    // note, the course's weights and the preferences, and nothing else at all.
    for forbidden in ["ingest.yaml", "credential", "credential_target", "C:\\", "knowlu/test-profile", "CS-100"] {
        assert!(!sent.contains(forbidden), "the request body carried {forbidden:?}:\n{sent}");
    }
    assert!(sent.contains("POST /functions/v1/judge-task HTTP/1.1"));
    assert!(sent.contains("Authorization: Bearer jwt-not-a-secret"));
    assert!(sent.contains("\"title\": \"CS 100 HW 01\""));
    assert!(sent.contains("Homework 20%"));
    assert!(sent.contains("I work best in the morning."));
    // The course_map's SLUGS travel (they are already in every note's `course:` field); its KEYS
    // — the fragments out of `config/ingest.yaml` — never do.
    assert!(sent.contains("\"cs-100\""));
}

#[test]
fn the_reply_goes_through_parse_reply_so_the_clamps_still_apply() {
    let mut server = loopback(vec![(200, ANSWERED.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let got = model.judge(&item(), &heuristics(), &seed).expect("the service answered");
    assert_eq!(got.effort_hours, Some(40.0), "900 hours is a bad answer, not a broken one");
    assert_eq!(got.importance, Some(5));
    assert_eq!(got.tier, 3);
    let _ = server.requests();
}

#[test]
fn a_401_says_it_is_the_session_and_not_a_bad_answer() {
    // Ruling R-3a-25 at the two boundaries a cloud judge adds. A 401 that reads as "low
    // confidence" twice a day forever is exactly the failure the structural-cause split exists
    // to prevent — the words have to say "sign in again".
    let mut server = loopback(vec![(401, r#"{"error":"invalid jwt"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("a 401 is an error");
    assert!(err.to_string().contains("no session"), "{err}");
    assert_eq!(model.fatal(), Some("no session"));
    let _ = server.requests();
}

#[test]
fn a_402_stops_the_batch_after_one_call() {
    // One reply is scripted, and the second `judge` must not reach the wire at all: an account
    // without an entitlement answers every item identically, and fifty round trips to learn that
    // is fifty wasted calls and fifty log lines that all say the same thing.
    let mut server = loopback(vec![(402, r#"{"error":"no active subscription"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let _ = model.judge(&item(), &heuristics(), &seed);
    let second = model.judge(&item(), &heuristics(), &seed).expect_err("still an error");
    assert!(second.to_string().contains("no entitlement"), "{second}");
    assert_eq!(server.requests().len(), 1, "the second item must not have been sent");
}

#[test]
fn a_5xx_survives_to_the_error_the_engine_prints() {
    let mut server = loopback(vec![(503, r#"{"error":"upstream overloaded"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("a 503 is an error");
    assert!(err.to_string().contains("503"), "{err}");
    assert!(err.to_string().contains("upstream overloaded"), "{err}");
    assert_eq!(model.fatal(), None, "a 503 is worth retrying the next item");
    let _ = server.requests();
}

#[test]
fn the_session_token_never_reaches_an_error_string() {
    // A transport error's text is a third party's, and `state/runner-log.md` is written from it.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener); // nothing is listening on that port now
    let client = CloudClient::new(&config(&format!("http://127.0.0.1:{port}/functions/v1")), "jwt-super-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("nothing is listening");
    assert!(!err.to_string().contains("jwt-super-secret"), "{err}");
}

#[test]
fn a_vault_with_no_cloud_config_is_no_account_and_not_an_error() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
    assert_eq!(knowlu_engine::cloudmodel::load(&dir), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_verdict_the_service_refused_is_an_error_that_names_the_cause() {
    let refused = r#"{"verdict":null,"tier":3,"outcome":"low confidence","cause":"below floor"}"#;
    let mut server = loopback(vec![(200, refused.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("no verdict is an error");
    assert!(err.to_string().contains("below floor"), "{err}");
    let _ = server.requests();
}

#[test]
fn tier1_still_answers_without_the_service_being_reached_at_all() {
    // The seam is unchanged (cloud design §3.2): a vendor-stated effort plus a pinned course is a
    // complete tier-1 answer, and `judge_task` returns before any model, cloud or otherwise.
    let mut vendor = item();
    vendor.effort_source = "vendor".to_string();
    vendor.effort_hours = 1.52;
    vendor.course = Some("cs-100".to_string());
    let seed: Verdict = judge::tier1(&vendor, &heuristics());
    assert_eq!(seed.course.as_deref(), Some("cs-100"));
    assert_eq!(seed.effort_hours, Some(1.52));
    assert_eq!(seed.tier, 1);
}
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `cargo test -p knowlu-engine --test cloud_contract`
Expected: FAIL to compile — `unresolved import knowlu_engine::cloudmodel`.

- [ ] **Step 3: Extend `judge.rs` — the two new outcomes and the two new seams.** Three edits, all additive.

Replace `Missing`:

```rust
/// Why there is no model to call. Resolved once per run, before any item is judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    Runtime,
    Model,
    /// C2: this vault has an account, and the judgment service could not be reached or refused —
    /// no session, no entitlement, no network. The reason is a `&'static str` from a closed set
    /// (`cloudmodel::CloudError::label`), so it can be printed and logged: it can never carry a
    /// server's body, a prompt or a token.
    Service(&'static str),
}
```

Add one variant to `Outcome`, after `RuntimeNotInstalled`:

```rust
    /// C2: this vault has an account and the service did not answer. `v` is what the device's own
    /// tiers had, and it is still written — a slot with no signal must not lose tier 1's work.
    /// Normal, like the two above it, and still exit 0.
    ServiceUnavailable(Verdict),
```

and the two arms it needs, in `Outcome::verdict` and `Outcome::label`:

```rust
            Outcome::ServiceUnavailable(v) => v,
```
```rust
            Outcome::ServiceUnavailable(_) => "service unavailable",
```

and one arm in `judge_task`'s model match:

```rust
        Err(Missing::Service(_)) => return Outcome::ServiceUnavailable(seed),
```

Then add the two sibling seams at the bottom of the module, beside `Model`:

```rust
/// Tier 3 for an event (cloud design §5.2). Separate from [`Model`] because an event is not a
/// task: the answer is one of `eventledger::VALID_VERDICTS` and a one-line why, not five task
/// fields. Implemented by `cloudmodel::CloudModel` in Task 9; a scripted fake in every test.
pub trait EventModel {
    fn judge_event(&self, item: &EventItem) -> Result<EventVerdict, ModelError>;
}

/// One event, flattened out of `state/events.md` by `events::judge_roster`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventItem {
    pub uid: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub source: String,
    pub organizer: String,
    pub location: String,
    pub url: String,
    pub description: String,
    pub categories: Vec<String>,
    pub audiences: Vec<String>,
    pub series_uid: String,
    /// `profile/interests.md`, clipped. The grounding, exactly as grade weights ground a task.
    pub interests: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventVerdict {
    /// `obligation` | `opportunity` | `drop` — validated again on the device before it is written.
    pub verdict: String,
    /// One line, no double quote, no ` · ` — `eventledger::why_problem` refuses all three.
    pub why: String,
    pub confidence: f64,
    pub tier: u8,
}

/// Tier 3 for one email (cloud design §5.3's five tiers).
///
/// **The Gmail path does not call this**, and that is D12 rather than an oversight: Gmail message
/// text must never reach the device, so `/gmail-read` judges server-side and returns verdicts.
/// The seam exists because it is the same judgment, reached the same way, by the two callers that
/// are not Gmail — the eval harness's device-side parity check, and the forwarding fallback the
/// spec keeps in reserve (§13) — and because a third judgment with no trait here would be the one
/// place `judge` is not a seam.
pub trait EmailModel {
    fn judge_email(&self, item: &EmailItem) -> Result<EmailVerdict, ModelError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmailItem {
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
    pub text: String,
    pub known_courses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmailVerdict {
    /// `task` | `borderline` | `event` | `opportunity` | `information`.
    pub tier: String,
    pub title: String,
    pub course: Option<String>,
    pub due: Option<String>,
    pub effort_hours: Option<f64>,
    pub importance: Option<i64>,
    pub why: String,
    pub confidence: f64,
}
```

- [ ] **Step 4: Write `engine/src/cloudmodel.rs`.**

```rust
//! The judgment service as seen from the device (cloud design §3.2, §5.2).
//!
//! **One HTTPS call per item, and nothing model-shaped in this binary.** From C2 on, tier 3 is
//! `POST /judge/*` on our Supabase project: the prompt, the JSON schema and the pinned model id
//! live server-side, so changing how a judgment is asked for is a deploy and not an app release
//! (§10). `judge.rs` is untouched — `CloudModel` implements the same `judge::Model` the local
//! process implemented, and every scripted fake in the plan-3a tests keeps working.
//!
//! **Delivery is always a pull.** Nothing is ever pushed at the device: `judge` asks per item and
//! writes the reply through `write`, inside the slot the app already runs. C3's journal sync
//! generalises this later; nothing here waits for it.
//!
//! **Three properties this module exists to hold.**
//!
//! - *The session token never reaches a string anyone can read.* It comes out of Credential
//!   Manager through `wincred`, exactly as a portal password does, and every error text this
//!   module produces goes through `zybooks::scrub` first. `state/runner-log.md` is written from
//!   those strings.
//! - *A failure says which failure it was.* Ruling R-3a-25's standing question — "does the failure
//!   text survive this boundary?" — has two new boundaries here: the HTTP status and the auth
//!   failure. A 401 that reads as "low confidence" twice a day forever is the same defect the
//!   structural `LowCause` split was written to prevent, so [`CloudError::label`] is a closed set
//!   of words that name the user's actual problem.
//! - *A refusal that answers every item stops the batch.* An account with no entitlement answers
//!   fifty items identically; [`CloudModel::fatal`] records the first one and the rest are never
//!   sent.
//!
//! **Nothing here is reached by `rank`** (decision 11). `rank` may reach [`fetch_event_source`]
//! and [`fetch_ics`], which are transport and carry no judgment.

use std::cell::Cell;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::judge::{self, ModelError};

/// One call's wall-clock bound — the same 120 seconds `runtime::CALL_TIMEOUT` gave one local
/// completion. Spelled again here rather than borrowed, because C4 removes `runtime.rs` and this
/// bound outlives it: a judge step runs inside a slot whose own child cap is twenty minutes.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// `config/cloud.yaml`, written by the wizard at onboarding (C1). Absent on a vault that has
/// never signed in, which is a named skip and not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudConfig {
    pub api_base: String,
    pub anon_key: String,
    pub session_credential_target: String,
    pub account_id: String,
}

/// Why this vault cannot talk to the service *before* a single call is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// No `config/cloud.yaml`: this vault has no account.
    NoConfig,
    /// The file names a Credential Manager target that holds nothing readable.
    NoSession(String),
}

impl Unavailable {
    /// A closed set, so it can be printed and logged.
    pub fn label(&self) -> &'static str {
        match self {
            Unavailable::NoConfig => "no account",
            Unavailable::NoSession(_) => "no session",
        }
    }
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unavailable::NoConfig => write!(f, "no account on this vault (no config/cloud.yaml)"),
            Unavailable::NoSession(why) => write!(f, "no session in Credential Manager ({why}); sign in again"),
        }
    }
}

/// Read `config/cloud.yaml`. **Never fails**: a missing, unreadable or incomplete file is `None`,
/// because a vault with a half-written config must skip the cloud step and rank the day, not stop.
pub fn load(vault: &Path) -> Option<CloudConfig> {
    let text = crate::pystr::read_text(&vault.join("config").join("cloud.yaml")).ok()?;
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).ok()?;
    let field = |key: &str| {
        value.get(key).and_then(serde_yaml_ng::Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
    };
    Some(CloudConfig {
        api_base: field("api_base")?.trim_end_matches('/').to_string(),
        anon_key: field("anon_key")?,
        session_credential_target: field("session_credential_target")?,
        account_id: field("account_id")?,
    })
}

/// The account's **access token**, out of the credential blob C1 writes.
///
/// **The blob is one JSON object, not a bare JWT** (*Interfaces with C1*, contract 2):
/// `{"access_token","refresh_token","expires_at","email"}`, `expires_at` in Unix seconds, with the
/// `UserName` field holding the account id. **Refresh is C1's job** — `account::valid_access_token()`
/// refreshes at fewer than 120 seconds remaining and rewrites the entry — so this reads and never
/// writes, and an expired token is reported as `no session` rather than refreshed here. A device
/// that refreshed on its own would race the app's own refresh and could invalidate it.
#[cfg(windows)]
fn session_token(cfg: &CloudConfig) -> Result<String, Unavailable> {
    let credential = crate::wincred::read_credential(&cfg.session_credential_target)
        .map_err(|err| Unavailable::NoSession(format!("{err}")))?;
    let blob: serde_json::Value = serde_json::from_str(credential.password.expose())
        .map_err(|_| Unavailable::NoSession("the credential is not the JSON the app writes".to_string()))?;
    let token = blob
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Unavailable::NoSession("the credential carries no access_token".to_string()))?;
    // Read but not acted on: a token that expired ten seconds ago still gets one attempt, because
    // the server is the authority on its own tokens and a clock skew here would refuse a good one.
    // A real 401 comes back as `CloudError::Status { code: 401 }` and says "sign in again".
    Ok(token.to_string())
}

/// The credential store is Windows-only (spec §6.5), so a cloud build still compiles and simply
/// has no way to authenticate — and says so rather than pretending there is no account.
#[cfg(not(windows))]
fn session_token(_cfg: &CloudConfig) -> Result<String, Unavailable> {
    Err(Unavailable::NoSession("credential store unavailable on this platform".to_string()))
}

/// The client for this vault, or the reason there is none. Resolved **once per run**, never per
/// item: fifty items must not each re-read Credential Manager.
pub fn resolve(vault: &Path) -> Result<CloudClient, Unavailable> {
    let cfg = load(vault).ok_or(Unavailable::NoConfig)?;
    let token = session_token(&cfg)?;
    Ok(CloudClient::new(&cfg, &token))
}

/// What went wrong on the wire. The **status** is kept because it is the difference between "sign
/// in again", "your subscription lapsed" and "try later"; the **body** is kept only as the
/// server's own short `error` field, clipped, because a service reply is the one place a prompt
/// could come back out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudError {
    Transport(String),
    Status { code: u16, detail: String },
    Body(String),
}

impl CloudError {
    /// The word `Missing::Service` carries and `enrich` prints. A closed set (ruling R-3a-20's
    /// shape): there is no value here that came from a note, a model or a server body.
    pub fn label(&self) -> &'static str {
        match self {
            CloudError::Transport(_) => "no network",
            CloudError::Status { code: 401, .. } => "no session",
            CloudError::Status { code: 402, .. } => "no entitlement",
            CloudError::Status { code: 403, .. } => "not allowed",
            CloudError::Status { code: 429, .. } => "rate limited",
            CloudError::Status { .. } => "the service refused",
            CloudError::Body(_) => "an unreadable reply",
        }
    }

    /// Does this answer every remaining item the same way? A session, an entitlement or a
    /// permission problem does; a 429, a 5xx and a dropped connection do not.
    pub fn fatal(&self) -> bool {
        matches!(self, CloudError::Status { code: 401 | 402 | 403, .. })
    }
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudError::Transport(why) => write!(f, "no network ({why})"),
            CloudError::Status { code, detail } if detail.is_empty() => write!(f, "{} (HTTP {code})", self.label()),
            CloudError::Status { code, detail } => write!(f, "{} (HTTP {code}: {detail})", self.label()),
            CloudError::Body(why) => write!(f, "an unreadable reply ({why})"),
        }
    }
}

/// One agent, one bearer, one account. Built once per run.
pub struct CloudClient {
    base: String,
    anon_key: String,
    token: String,
    account_id: String,
    agent: ureq::Agent,
}

impl CloudClient {
    pub fn new(cfg: &CloudConfig, token: &str) -> CloudClient {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(CALL_TIMEOUT))
            // A non-2xx must arrive as a status AND a body, not as an opaque transport error:
            // `ureq`'s default turns both into `Error::StatusCode` with the body gone, and 401
            // and 402 are precisely the two answers the user needs named.
            .http_status_as_error(false)
            .build()
            .into();
        CloudClient {
            base: cfg.api_base.trim_end_matches('/').to_string(),
            anon_key: cfg.anon_key.clone(),
            token: token.to_string(),
            account_id: cfg.account_id.clone(),
            agent,
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Every error string this produces is scrubbed of the bearer first: `state/runner-log.md` is
    /// written from these and a third party's error text is not ours to trust.
    fn scrub(&self, text: &str) -> String {
        crate::zybooks::scrub(text, &[&self.token])
    }

    fn finish(&self, mut response: ureq::http::Response<ureq::Body>) -> Result<Value, CloudError> {
        let code = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        if !(200..300).contains(&code) {
            let detail = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default();
            return Err(CloudError::Status { code, detail: judge::clip(&self.scrub(&detail), 200) });
        }
        serde_json::from_str(&text).map_err(|e| CloudError::Body(e.to_string()))
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, CloudError> {
        // `dumps_value`, not `to_string`: every JSON this crate writes goes through the one writer
        // (CLAUDE.md), so a request body has the same separators and key order everywhere.
        let response = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("apikey", self.anon_key.clone())
            .header("Content-Type", "application/json")
            .send(crate::ledger::dumps_value(body))
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        self.finish(response)
    }

    pub fn get(&self, path: &str) -> Result<Value, CloudError> {
        let response = self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("apikey", self.anon_key.clone())
            .call()
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        self.finish(response)
    }
}

/// The body of `POST /judge-task`.
///
/// **Everything the prompt needs and nothing else** (plan 3a fidelity row R6, now measured at the
/// HTTP boundary rather than at the prompt string): the note's own fields, the attributed course's
/// grade weights, `profile/preferences.md`, the planner's slice, and the course **slugs** this
/// vault has notes or pins for. The `course_map`'s KEYS — the fragments out of
/// `config/ingest.yaml` — never travel; the slugs do, and they are already the `course:` field of
/// every note in the vault. No vault path, no profile id, no credential target, no account id.
pub fn task_request(item: &judge::Item, h: &judge::Heuristics, seed: &judge::Verdict) -> Value {
    let weights = seed
        .course
        .as_deref()
        .and_then(|c| h.weights.get(c))
        .map(String::as_str)
        .unwrap_or("");
    let mut known: Vec<&str> = h.weights.keys().map(String::as_str).collect();
    for (_, slug) in &h.course_map {
        if !known.contains(&slug.as_str()) {
            known.push(slug);
        }
    }
    json!({
        "kind": "task",
        "item": {
            "id": item.id,
            "title": judge::one_line(&item.title, 200),
            "body": judge::clip(item.body.trim(), judge::MAX_BODY_CHARS),
            "source_uid": item.source_uid,
            "created_by": item.created_by,
            "course": item.course,
            "due": item.due,
        },
        "heuristics_seed": {
            "course": seed.course,
            "effort_hours": seed.effort_hours,
            "slice_hours": h.slice_hours,
            "weights": judge::clip(weights, judge::MAX_WEIGHTS_CHARS),
            "preferences": judge::clip(&h.preferences, judge::MAX_PREFS_CHARS),
            "known_courses": known,
        }
    })
}

/// Tier 3, over HTTPS. One call per item, and a refusal that answers every item stops the rest.
pub struct CloudModel<'a> {
    client: &'a CloudClient,
    fatal: Cell<Option<&'static str>>,
}

impl<'a> CloudModel<'a> {
    pub fn new(client: &'a CloudClient) -> CloudModel<'a> {
        CloudModel { client, fatal: Cell::new(None) }
    }

    /// Set once a call comes back 401, 402 or 403. `enrich` prints it as one summary line instead
    /// of fifty identical per-item lines.
    pub fn fatal(&self) -> Option<&'static str> {
        self.fatal.get()
    }

    pub(crate) fn call(&self, path: &str, body: &Value) -> Result<Value, ModelError> {
        if let Some(reason) = self.fatal.get() {
            return Err(ModelError::Failed(format!("the judgment service: {reason}")));
        }
        self.client.post(path, body).map_err(|e| {
            if e.fatal() {
                self.fatal.set(Some(e.label()));
            }
            ModelError::Failed(format!("the judgment service: {e}"))
        })
    }
}

impl judge::Model for CloudModel<'_> {
    fn judge(
        &self,
        item: &judge::Item,
        h: &judge::Heuristics,
        seed: &judge::Verdict,
    ) -> Result<judge::Verdict, ModelError> {
        let reply = self.call("/judge-task", &task_request(item, h, seed))?;
        let verdict = reply.get("verdict").filter(|v| !v.is_null()).ok_or_else(|| {
            let cause = reply.get("cause").and_then(Value::as_str).unwrap_or("no verdict");
            ModelError::Failed(format!("the judgment service answered {cause}"))
        })?;
        // Through `judge::parse_reply`, deliberately. The clamps, the one-lined reason and the
        // blank-is-None rules are the engine's, and a reply from the service goes through exactly
        // the door a reply from a local process went through — so the service can never widen a
        // bound the vault depends on.
        let mut v = judge::parse_reply(&crate::ledger::dumps_value(verdict))?;
        if let Some(tier) = reply.get("tier").and_then(Value::as_u64) {
            v.tier = tier.min(3) as u8;
        }
        Ok(v)
    }
}
```

- [ ] **Step 5: Wire it into `enrich.rs`.** Replace `run_lines` with the seam plus the two arms:

```rust
/// Resolve the service — or, on a vault with no account, plan 3a's local runtime — and enrich.
///
/// **The cloud comes first from C2 on** (cloud design D3, §3.2). The local branch below is plan
/// 3a's and is removed by C4; until then a vault with no `config/cloud.yaml` behaves exactly as it
/// did. Every arm exits 0: no account, no session, no network and no entitlement are all normal
/// outcomes reported on stdout.
pub fn run_lines(vault: &Path, opts: &Options<'_>) -> (i32, Vec<String>) {
    match crate::cloudmodel::resolve(vault) {
        Ok(client) => run_lines_with(vault, opts, Some(&client)),
        Err(crate::cloudmodel::Unavailable::NoConfig) => run_lines_with(vault, opts, None),
        Err(why) => {
            // A vault WITH an account whose session is missing or unreadable. Not a fallback to
            // the local runtime: this is a cloud machine and the answer is "sign in again".
            let (code, mut lines) =
                enrich_with(vault, opts, Err(judge::Missing::Service(why.label())));
            lines.insert(0, format!("judge: skipped ({why})"));
            (code, lines)
        }
    }
}

/// [`run_lines`] with the service seam exposed, exactly as `ingest::run_with` exposes its fetch and
/// `coursework::main_with_fetchers` exposes its sources. Production resolves; tests pass a client
/// aimed at a listener bound to `127.0.0.1:0`, so no test needs a credential or an environment
/// variable and there is no process-global state to race.
pub fn run_lines_with(
    vault: &Path,
    opts: &Options<'_>,
    cloud: Option<&crate::cloudmodel::CloudClient>,
) -> (i32, Vec<String>) {
    let Some(client) = cloud else {
        return match crate::runtime::resolve(opts.runtime, opts.model) {
            Err(missing) => enrich_with(vault, opts, Err(missing)),
            Ok((rt, gguf)) => enrich_with(
                vault,
                opts,
                Ok(&crate::runtime::PerCall::new(&rt, &gguf, crate::runtime::CALL_TIMEOUT)),
            ),
        };
    };
    let model = crate::cloudmodel::CloudModel::new(client);
    // A session or entitlement problem answers every item identically, so the FIRST call decides
    // the batch. Asked once before the loop, the whole run then reports one honest outcome
    // (`Missing::Service`, which becomes `Outcome::ServiceUnavailable` and logs as
    // `service unavailable`) instead of fifty `model failed` lines — ruling R-3a-20's point, at
    // the boundary a cloud judge adds. Every other failure (a 429, a 5xx, a dropped connection)
    // stays per-item, because the next item genuinely may succeed.
    let (code, mut lines) = match model.probe() {
        Some(reason) => {
            let (code, mut lines) = enrich_with(vault, opts, Err(judge::Missing::Service(reason)));
            lines.insert(0, format!("judge: the service answered {reason}; nothing was sent"));
            (code, lines)
        }
        None => enrich_with(vault, opts, Ok(&model)),
    };
    if let Some(reason) = model.fatal() {
        lines.push(format!(
            "judge: the service answered {reason}, so the rest of the batch was not sent"
        ));
    }
    (code, lines)
}
```

`CloudModel::probe()` is the one extra function that buys this: it is `fatal()` after a **single** cheap round trip that the first item would have made anyway, so no call is wasted.

```rust
impl CloudModel<'_> {
    /// Ask once, before the batch, whether this account can be judged at all.
    ///
    /// `Some(reason)` for the three answers that will not change item by item — no session, no
    /// entitlement, not allowed — and `None` for everything else, including "no network", because
    /// a flaky connection is per-item and the batch should try. The probe is `GET /judge-rules`,
    /// which every account may call, costs no model tokens and charges no cap.
    ///
    /// **Before Task 12 deploys `judge-rules`, this is a 404** — which is not `fatal()`, so it
    /// answers `None` and the batch proceeds exactly as it would have. That is deliberate: the
    /// probe is an optimisation for the two answers that repeat, never a gate.
    pub fn probe(&self) -> Option<&'static str> {
        match self.client.get("/judge-rules") {
            Ok(_) => None,
            Err(e) if e.fatal() => {
                self.fatal.set(Some(e.label()));
                Some(e.label())
            }
            Err(_) => None,
        }
    }
}
```

- [ ] **Step 6: Extend `engine/tests/dependency_boundary.rs`** with the §8 pin:

```rust
/// Judgment is a service, and the engine reaches it with `ureq` and nothing else.
///
/// Cloud design §8: "`dependency_boundary.rs` now also forbids `tauri` and any cloud SDK in the
/// engine; the engine talks HTTP with `ureq` and nothing else." A provider SDK in this crate would
/// bring a runtime, a TLS stack of its own and — the actual cost — a second place where a request
/// body is assembled, out of reach of the one test that proves what a request may carry.
#[test]
fn no_cloud_sdk_enters_the_engine() {
    for (name, manifest) in MANIFESTS {
        for forbidden in [
            "anthropic", "openai", "supabase", "postgrest", "aws-sdk", "azure_", "google-cloud",
            "reqwest", "hyper", "isahc", "curl",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "`{forbidden}` must not be a dependency of the engine crate ({name}). The engine \
                 talks to the judgment service over `ureq` and nothing else (cloud design §8), so \
                 every request body is assembled in `cloudmodel.rs` where one test can see it."
            );
        }
    }
}
```

- [ ] **Step 7: Run everything and watch it pass.**

Run: `cargo test --workspace`
Expected: green at **0 warnings** (the app's `.rsrc merge failure` line is the one accepted message). In particular `oracle.rs` (3 tests), `surface_oracle.rs` (4 tests) and every plan-3a test in `judge.rs`, `enrich.rs` and `judgelog.rs` pass **unchanged** — record the totals in this step's line. If `cloud_contract` cannot resolve `knowlu_engine::cloudmodel`, H1 has not been applied; ask the controller rather than editing `lib.rs`.

- [ ] **Step 8: Prove the whole pass end to end against a loopback listener.** Add one unit test at the bottom of `enrich.rs`'s test module (it needs `pub(crate)` reach that an integration test does not have):

```rust
    /// The whole `judge` step against a service that is not there: every item still gets tier 1's
    /// answer written, the run says why, and the exit code is 0. This is the case the app actually
    /// meets on a train, and the one a non-zero exit would turn into an amber tray forever.
    #[test]
    fn a_service_that_cannot_be_reached_still_writes_tier_one_and_exits_zero() {
        let vault = super::tests::scratch_vault("cloud-down");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: format!("http://127.0.0.1:{port}/functions/v1"),
            anon_key: "anon".into(),
            session_credential_target: "knowlu/test/session".into(),
            account_id: "acct-1".into(),
        };
        let client = crate::cloudmodel::CloudClient::new(&cfg, "jwt-not-a-secret");
        let opts = Options {
            via: "local-runner", run_id: None, runtime: None, model: None,
            log_dir: None, limit: 10, budget: BATCH_BUDGET,
        };
        let (code, lines) = run_lines_with(&vault, &opts, Some(&client));
        assert_eq!(code, 0, "the judge step always exits 0");
        assert!(lines.iter().any(|l| l.contains("no network")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

`scratch_vault` is the helper `enrich.rs`'s existing tests already use to copy a fixture vault into a temp directory; reuse it rather than writing a second one, and never point it at the worktree (ruling R-3a-12).

Run: `cargo test -p knowlu-engine enrich::`
Expected: every existing `enrich` test still passes, plus this one.

- [ ] **Step 9: Name the hand-off and the dependency in the task report.** Hand-off **H1** (`engine/src/lib.rs`), verbatim. And, separately and clearly, the **C1 dependency**: `JudgePlan::Cloud` and its argv `judge --vault <v> --via local-runner --log-dir <the profile's logs folder>` (*Interfaces with C1*, contract 6). **Without C1's scheduler change the app never runs `judge` on a machine with no llama.cpp installed and the whole stream is inert** — but it is C1's change and no task here writes it, so it belongs in the report as a dependency and not in the hand-off list.

- [ ] **Step 10: Commit.**

```bash
git add engine/src/cloudmodel.rs engine/src/judge.rs engine/src/enrich.rs engine/tests/cloud_contract.rs engine/tests/dependency_boundary.rs
git commit -F .git-commit-msg.txt   # "engine: tier 3 is the judgment service — CloudModel behind judge::Model, one call per item, every failure a named line and exit 0 (C2 Task 4)"
```

**The first shippable increment ends here.** With H1 applied and C1 merged (its `JudgePlan::Cloud`), a vault carrying `config/cloud.yaml` and a session is enriched twice a day by the service, and the judgment gap the repo cut opened is closed.

---

### Task 5: The zyBooks parser, server-side, against the frozen reference

**Files:**
- Create: `cloud/supabase/functions/ingest-coursework/pyshims.ts`, `cloud/supabase/functions/ingest-coursework/parse_zybooks.ts`
- Test: `cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts`

**Interfaces:**
- Consumes: `engine/tests/fixtures/zybooks-assignments.json` and `zybooks-parsed-reference.json`, **read by relative path, never copied, never written**.
- Produces:
  - `interface Assignment { uid: string; slug: string; title: string; due: string; course: string | null; effort_hours: number; effort_confidence: string; effort_source: string; importance: number; importance_reason: string; progress: number; created_by: string; body: string }`
  - `pyshims.ts`: `round2(x: number): number`, `slugify(text: string): string`, `pyStr(v: unknown): string`, `pyInt(v: unknown): number`, `truthy(v: unknown): boolean`, `strip(s: string): string`, `dueLocal(raw: unknown, timeZone: string): string`
  - `parse_zybooks.ts`: `class NotLoggedIn extends Error`, `categoryOf(title, categories): string | null`, `routeZybook(code, courses, ignore): {kind:"mapped", mapping} | {kind:"ignored"} | {kind:"unmapped"}`, `parseAssignments(payload, courseSlug, courseLabel, cfg, timeZone, warnings): Assignment[]`

- [ ] **Step 1: Write the failing test** — `cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts`:

```ts
// The frozen Python-written reference is this port's acceptance test, exactly as it is the Rust
// port's. It is READ from engine/tests/fixtures/ by relative path — never copied into cloud/,
// never written, never regenerated (CLAUDE.md). If this fails, the port is wrong.
//
// The comparison is STRUCTURAL, not byte-for-byte, and that is deliberate: JavaScript has no
// `1.0` — `JSON.stringify(1.0)` is `"1"` — so a byte comparison would fail a correct port on the
// first whole-number effort_hours. The rows are arrays, so this still pins every field and its
// order, which is the property the reference was frozen for. The Rust byte-for-byte test
// (`zybooks::tests::the_parsed_payload_matches_python_byte_for_byte`) stays where it is.
import { assert, assertEquals } from "@std/assert";
import { type Assignment, categoryOf, NotLoggedIn, parseAssignments, routeZybook } from "./parse_zybooks.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);

const CFG = {
  categories: { HW: "hw", Lab: "lab", Project: "project" },
  effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
  importance: { hw: 2, lab: 2, project: 2 },
};

async function payload(): Promise<unknown> {
  // The capture is a real one and carries a UTF-8 BOM; the device's `decode_json` strips it there,
  // so no BOM ever travels on the wire. Here the file is read directly, so strip it here.
  const text = await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES));
  return JSON.parse(text.replace(/^\uFEFF/, ""));
}

function row(a: Assignment): unknown[] {
  return [a.uid, a.slug, a.title, a.due, a.course, a.effort_hours, a.effort_confidence,
    a.effort_source, a.importance, a.importance_reason, a.progress, a.created_by, a.body];
}

Deno.test("every field of all 24 parsed assignments matches the frozen Python reference", async () => {
  const warnings: string[] = [];
  const items = parseAssignments(await payload(), "cs-100", "CS 100", CFG, "America/Chicago", warnings);
  assertEquals(items.length, 24);
  assertEquals(warnings, []);
  const reference = JSON.parse(await Deno.readTextFile(new URL("zybooks-parsed-reference.json", FIXTURES)));
  assertEquals(items.map(row), reference);
});

Deno.test("a UTC stamp becomes 23:59 the day before, in Central", async () => {
  // 2026-08-27T04:59:00Z is 2026-08-26 23:59 CDT. Getting this wrong shifts every deadline by a
  // day, which is the whole point of the system.
  const items = parseAssignments(await payload(), "cs-100", "CS 100", CFG, "America/Chicago", []);
  const hw01 = items.find((a) => a.title.includes("HW 01"));
  assert(hw01 !== undefined);
  assertEquals(hw01.due, "2026-08-26T23:59");
});

Deno.test("a 200 with success false is a dead session, never an empty semester", () => {
  let threw: unknown = null;
  try {
    parseAssignments({ success: false, error: "invalid token" }, "cs-100", "CS 100", CFG, "America/Chicago", []);
  } catch (e) {
    threw = e;
  }
  assert(threw instanceof NotLoggedIn, "a falsy success must be NotLoggedIn, not an empty list");
});

Deno.test("an assignment with no due date is skipped with a warning, not dropped silently", () => {
  const warnings: string[] = [];
  const items = parseAssignments(
    { assignments: [{ assignment_id: 1, title: "HW 99", visible: true, due_dates: [], sections: [] }] },
    "cs-100", "CS 100", CFG, "America/Chicago", warnings,
  );
  assertEquals(items.length, 0);
  assertEquals(warnings, ["HW 99: no due date; skipped"]);
});

Deno.test("more than one due date warns and uses the earliest", () => {
  const warnings: string[] = [];
  const items = parseAssignments({
    assignments: [{
      assignment_id: 7, title: "HW 07", visible: true, sections: [],
      due_dates: [{ date: "2026-09-11T04:59:00Z" }, { date: "2026-09-05T04:59:00Z" }],
    }],
  }, "cs-100", "CS 100", CFG, "America/Chicago", warnings);
  assertEquals(items[0].due, "2026-09-04T23:59");
  assertEquals(warnings, ["HW 07: 2 due dates; using the earliest (2026-09-04T23:59)"]);
});

Deno.test("visible false is skipped, and a missing visible is not", () => {
  const hidden = parseAssignments({
    assignments: [{ assignment_id: 1, title: "HW 01", visible: false, due_dates: [{ date: "2026-09-05T04:59:00Z" }], sections: [] }],
  }, "cs-100", "CS 100", CFG, "America/Chicago", []);
  assertEquals(hidden.length, 0);
  const shown = parseAssignments({
    assignments: [{ assignment_id: 1, title: "HW 01", due_dates: [{ date: "2026-09-05T04:59:00Z" }], sections: [] }],
  }, "cs-100", "CS 100", CFG, "America/Chicago", []);
  assertEquals(shown.length, 1);
});

Deno.test("routing: mapped, ignored silently, or a warning nobody should miss", () => {
  const courses = { "cs-100-2026": { course: "cs-100", label: "CS 100" }, empty: {} };
  assertEquals(routeZybook("cs-100-2026", courses, []).kind, "mapped");
  // An empty mapping is falsy in Python and is not a course.
  assertEquals(routeZybook("empty", courses, []).kind, "unmapped");
  assertEquals(routeZybook("HowToUseZyBooks2", courses, ["HowToUseZyBooks2"]).kind, "ignored");
  assertEquals(routeZybook("new-course", courses, []).kind, "unmapped");
});

Deno.test("the first configured prefix wins, in the config's own order", () => {
  assertEquals(categoryOf("Lab 02 math_lib", CFG.categories), "lab");
  assertEquals(categoryOf("Something else", CFG.categories), null);
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts`
Expected: FAIL — `Module not found "…/parse_zybooks.ts"`.

- [ ] **Step 3: Write `pyshims.ts`.**

```ts
// The handful of Python semantics both moved parsers depend on. Named for what they are: this is
// a port of a port, and the frozen references are what say whether it is faithful.

/** Python's `round(x, 2)` — half to EVEN, matching `zybooks::round2` and `ranking::round2`. */
export function round2(x: number): number {
  const scaled = x * 100;
  const floor = Math.floor(scaled);
  const diff = scaled - floor;
  let n: number;
  if (diff > 0.5) n = floor + 1;
  else if (diff < 0.5) n = floor;
  else n = floor % 2 === 0 ? floor : floor + 1;
  return n / 100;
}

/** `ingest::slugify`: lowercase, collapse non-alphanumeric runs to `-`, trim, cap at 60 CHARACTERS, trim again. */
export function slugify(text: string): string {
  const dashed = text.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const trimmed = dashed.replace(/^-+/, "").replace(/-+$/, "");
  const capped = [...trimmed].slice(0, 60).join("").replace(/-+$/, "");
  return capped === "" ? "item" : capped;
}

/** Python's truthiness over a JSON value — `pystr::json_truthy`. */
export function truthy(value: unknown): boolean {
  if (value === null || value === undefined) return false;
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") return value !== "";
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") return Object.keys(value).length > 0;
  return true;
}

/**
 * Python's `str(x)` over a JSON value — `pystr::json_str`. `None`, `True`, `False` spelled
 * Python's way, and a float that happens to be whole keeps its `.0`: Python's `str(2.0)` is
 * `"2.0"` where JavaScript's `String(2.0)` is `"2"`. Nothing the two captured fixtures contain
 * exercises that branch (every numeric field in them is an integer), but the difference is exactly
 * the class of drift the frozen references exist to catch, so it is written correctly rather than
 * left to the fixture.
 */
export function pyStr(value: unknown): string {
  if (value === null || value === undefined) return "None";
  if (value === true) return "True";
  if (value === false) return "False";
  if (typeof value === "string") return value;
  if (typeof value === "number") {
    if (Number.isInteger(value)) return String(value);
    return String(value);
  }
  if (typeof value === "bigint") return String(value);
  return JSON.stringify(value);
}

/**
 * A JSON *float* as Python spells it. Used only where the source value is known to be a float in
 * Python's sense — `Assignment.effort_hours` on the device's side of the wire, never here.
 */
export function pyFloat(value: number): string {
  return Number.isInteger(value) ? `${value}.0` : String(value);
}

/** Python's `int(x or 0)` — `pystr::json_int`. Absent, null, false, 0 and "" are all 0. */
export function pyInt(value: unknown): number {
  if (!truthy(value)) return 0;
  if (value === true) return 1;
  if (typeof value === "number") return Math.trunc(value);
  if (typeof value === "string") {
    const n = Number(value.trim().replaceAll("_", ""));
    if (!Number.isFinite(n) || !Number.isInteger(n)) {
      throw new Error(`invalid literal for int() with base 10: '${value}'`);
    }
    return n;
  }
  throw new Error(`int() argument must be a number, not ${JSON.stringify(value)}`);
}

/**
 * Python's `str.strip()` — **whitespace only, both ends**.
 *
 * Python strips the ASCII whitespace set plus a few Unicode separators; JavaScript's `trim()`
 * strips its own `WhiteSpace` set plus line terminators, which is the same set for every character
 * either engine will meet in a course title. The class is written out rather than left implicit so
 * that the one place the two could diverge is visible — and note it does **not** strip a dash: a
 * title legitimately starts or ends with one, and stripping it would change a uid's slug.
 */
const PY_SPACE = /^\s+|\s+$/g;
export function strip(text: string): string {
  return text.replace(PY_SPACE, "");
}

/**
 * `datetime.strptime(raw, "%Y-%m-%dT%H:%M:%SZ")` in UTC, converted to `timeZone`, made naive, and
 * rendered as `ingest::format_due` renders it — `%Y-%m-%dT%H:%M`.
 *
 * `Intl.DateTimeFormat` carries the IANA database Deno already ships, so this needs no dependency.
 * The `hour12: false` "24" quirk is real: at local midnight some runtimes report hour `24`.
 */
export function dueLocal(raw: unknown, timeZone: string): string {
  if (typeof raw !== "string") {
    throw new Error(`strptime() argument 1 must be str, not ${typeof raw}`);
  }
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z$/.exec(raw);
  if (m === null) {
    throw new Error(`time data '${raw}' does not match format '%Y-%m-%dT%H:%M:%SZ'`);
  }
  const at = Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
  const fmt = new Intl.DateTimeFormat("en-CA", {
    timeZone, year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit", hour12: false,
  });
  const parts: Record<string, string> = {};
  for (const part of fmt.formatToParts(new Date(at))) parts[part.type] = part.value;
  const hour = parts.hour === "24" ? "00" : parts.hour;
  return `${parts.year}-${parts.month}-${parts.day}T${hour}:${parts.minute}`;
}
```

- [ ] **Step 4: Write `parse_zybooks.ts`.**

```ts
// zyBooks' assignment payload -> task items. A faithful port of `engine/src/zybooks.rs`'s
// `parse_assignments`, `category_of` and `route_zybook`, measured against the same frozen
// Python-written reference the Rust is measured against (cloud design §4.3).
//
// **Pure.** No fetch, no credential, no clock: the device signs in and fetches with the student's
// own password (D11) and posts the payload here. This half is what a vendor markup change moves,
// and moving it is why a fix becomes a deploy instead of an app release.
import { dueLocal, pyInt, pyStr, round2, slugify, strip, truthy } from "./pyshims.ts";

export const DEFAULT_IMPORTANCE = 3;
export const DEFAULT_FLOOR_HOURS = 0.25;

export interface Assignment {
  uid: string;
  slug: string;
  title: string;
  /** `%Y-%m-%dT%H:%M`, already local and naive — `ingest::format_due`'s spelling. */
  due: string;
  course: string | null;
  effort_hours: number;
  effort_confidence: string;
  effort_source: string;
  importance: number;
  importance_reason: string;
  progress: number;
  created_by: string;
  body: string;
}

/** A 200 whose `success` is falsy is a dead session, not an empty semester. */
export class NotLoggedIn extends Error {}

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

function arrayField(map: Obj, key: string): unknown[] {
  const value = map[key];
  if (!truthy(value)) return [];
  if (!Array.isArray(value)) throw new Error(`${key} is not a list`);
  return value;
}

function mappingField(cfg: Obj, key: string): Obj {
  const value = cfg[key];
  if (!truthy(value)) return {};
  if (Array.isArray(value) || typeof value !== "object") throw new Error(`config key "${key}" is not a mapping`);
  return value as Obj;
}

export function requireSuccess(payload: unknown, what: string): void {
  const map = obj(payload);
  if (!("success" in map)) return;
  if (truthy(map.success)) return;
  const detail = truthy(map.error) ? pyStr(map.error) : truthy(map.message) ? pyStr(map.message) : "no detail";
  throw new NotLoggedIn(`zybooks ${what} reported success=false (${detail})`);
}

/** First configured prefix the lowercased title starts with, in the config's own order. */
export function categoryOf(title: string, categories: Obj): string | null {
  const lowered = title.toLowerCase();
  for (const [prefix, kind] of Object.entries(categories)) {
    if (lowered.startsWith(prefix.toLowerCase())) return pyStr(kind);
  }
  return null;
}

export type Routing =
  | { kind: "mapped"; mapping: Obj }
  | { kind: "ignored" }
  | { kind: "unmapped" };

/**
 * `ignored` is silent on purpose — `HowToUseZyBooks2` is zyBooks' own onboarding book and a
 * warning there would fire on every healthy run. `unmapped` is a warning nobody should miss: a
 * genuinely new course appearing is something the student must be told about.
 */
export function routeZybook(code: string, courses: Obj, ignore: string[]): Routing {
  const mapping = courses[code];
  if (truthy(mapping) && typeof mapping === "object" && !Array.isArray(mapping)) {
    return { kind: "mapped", mapping: mapping as Obj };
  }
  if (ignore.includes(code)) return { kind: "ignored" };
  return { kind: "unmapped" };
}

export function parseAssignments(
  payload: unknown,
  courseSlug: string,
  courseLabel: string,
  cfg: Obj,
  timeZone: string,
  warnings: string[],
): Assignment[] {
  requireSuccess(payload, "assignment payload");
  // slugify, not the raw config value: the slug becomes tasks/<slug>.md on the device, and a
  // config typo carrying a path separator would otherwise write outside tasks/.
  const slugPrefix = slugify(courseSlug);
  const categories = mappingField(cfg, "categories");
  const effortCfg = mappingField(cfg, "effort");
  const perSection = truthy(effortCfg.minutes_per_section) || effortCfg.minutes_per_section === 0
    ? Number(effortCfg.minutes_per_section)
    : 6;
  const floors = mappingField(effortCfg, "floors");
  const importanceTable = mappingField(cfg, "importance");

  const out: Assignment[] = [];
  for (const rawItem of arrayField(obj(payload), "assignments")) {
    const raw = obj(rawItem);
    const title = strip(truthy(raw.title) ? pyStr(raw.title) : "");
    if (title === "") {
      warnings.push("assignment with no title skipped");
      continue;
    }
    // `is False`, not falsiness: a missing or null `visible` leaves the assignment visible.
    if (raw.visible === false) continue;

    const stamps = arrayField(raw, "due_dates")
      .map((d) => obj(d).date)
      .filter((d) => truthy(d));
    if (stamps.length === 0) {
      warnings.push(`${title}: no due date; skipped`);
      continue;
    }
    // Earliest is the only defensible pick from a plural due_dates[] the vendor does not order:
    // it is the deadline that binds first, and deferring one is far cheaper than missing one.
    let earliest: string | null = null;
    let unreadable: string | null = null;
    for (const stamp of stamps) {
      try {
        const parsed = dueLocal(stamp, timeZone);
        if (earliest === null || parsed < earliest) earliest = parsed;
      } catch (e) {
        unreadable = e instanceof Error ? e.message : String(e);
        break;
      }
    }
    if (unreadable !== null) {
      warnings.push(`${title}: unreadable due date (${unreadable}); skipped`);
      continue;
    }
    if (earliest === null) continue;
    if (stamps.length > 1) {
      warnings.push(`${title}: ${stamps.length} due dates; using the earliest (${earliest})`);
    }

    const sections = arrayField(raw, "sections");
    let points = 0;
    for (const section of sections) {
      try {
        points += pyInt(obj(section).total_points);
      } catch (e) {
        throw new Error(`${title}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }

    const kind = categoryOf(title, categories);
    if (kind === null) warnings.push(`${title}: uncategorised; using default importance`);
    const importance = kind !== null && kind in importanceTable ? pyInt(importanceTable[kind]) : DEFAULT_IMPORTANCE;
    const floor = kind !== null && kind in floors ? Number(floors[kind]) : DEFAULT_FLOOR_HOURS;
    const effort = round2(Math.max(sections.length * perSection / 60, floor));

    const assignmentId = "assignment_id" in raw ? pyStr(raw.assignment_id) : "None";
    const listing = sections.map((s) => {
      const section = obj(s);
      const field = (key: string) => key in section ? pyStr(section[key]) : "None";
      return `- ${field("chapter_number")}.${field("section_number")} ${field("title")}`;
    });
    const body = `${sections.length} zyBooks section(s), ${points} points.\n\n${listing.join("\n")}`;

    out.push({
      uid: `zybooks:${assignmentId}`,
      slug: `${slugPrefix}-${slugify(title)}`,
      title: `${courseLabel} ${title}`,
      due: earliest,
      course: courseSlug,
      effort_hours: effort,
      effort_confidence: "low",
      effort_source: "inferred",
      importance,
      importance_reason:
        `zyBooks ${kind !== null && kind !== "" ? kind : "item"} worth ${points} points across ` +
        `${sections.length} sections; per-category importance from config`,
      progress: 0,
      created_by: "zybooks",
      body,
    });
  }
  return out;
}
```

- [ ] **Step 5: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts`
Expected: `ok | 8 passed | 0 failed`. The first test is the one that matters: if it fails, **fix the port, never the reference** (CLAUDE.md). Read the first differing row's index and field — the array shape makes the diff say which of the thirteen it is.

- [ ] **Step 6: Confirm the fixtures were only read.**

Run: `git status --porcelain --untracked-files=all engine/tests/fixtures/`
Expected: no output. Paste it into the task report (ruling R-3a-24).

- [ ] **Step 7: Add the doc line to `engine/src/zybooks.rs`.** One sentence at the head of `parse_assignments`'s doc comment, and nothing else in that file:

```rust
/// Turn one zyBooks assignment payload into [`Assignment`]s. Pure: every field comes from the
/// payload plus config, which is what keeps this inside the engine's no-inference rule.
///
/// **This function is the reference the server-side port is measured against** (cloud design
/// §4.3): `cloud/supabase/functions/ingest-coursework/parse_zybooks.ts` is a faithful TypeScript
/// port of it, and both are gated by the same frozen `tests/fixtures/zybooks-parsed-reference.json`.
/// From C2 on, production parses on the server and this half is what the port is checked against —
/// change one and the other's test fails, which is the point.
```

- [ ] **Step 8: Commit.**

```bash
git add cloud/supabase/functions/ingest-coursework/pyshims.ts cloud/supabase/functions/ingest-coursework/parse_zybooks.ts cloud/supabase/functions/ingest-coursework/parse_zybooks_test.ts engine/src/zybooks.rs
git commit -F .git-commit-msg.txt   # "cloud: the zyBooks parser moves server-side, gated by the frozen Python reference (C2 Task 5)"
```

---

### Task 6: The VHL parser, server-side, against the frozen reference

**Files:**
- Create: `cloud/supabase/functions/ingest-coursework/parse_vhl.ts`
- Test: `cloud/supabase/functions/ingest-coursework/parse_vhl_test.ts`

**Interfaces:**
- Consumes: `pyshims.ts`, `parse_zybooks.ts`'s `Assignment` and `NotLoggedIn`; `engine/tests/fixtures/vhl-dashboard.html` and `vhl-parsed-reference.json`, read by relative path.
- Produces: `MOUNT_MARKER: string`, `parseDurationHours(text: string): number` (throws on unreadable), `parseDashboard(html: string, cfg: Record<string, unknown>, warnings: string[]): Assignment[]`.

- [ ] **Step 1: Write the failing test** — `cloud/supabase/functions/ingest-coursework/parse_vhl_test.ts`:

```ts
import { assert, assertEquals, assertThrows } from "@std/assert";
import type { Assignment } from "./parse_zybooks.ts";
import { NotLoggedIn } from "./parse_zybooks.ts";
import { parseDashboard, parseDurationHours } from "./parse_vhl.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);

const CFG = {
  sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } },
  importance: 3,
  importance_reason: "GN 103 accepts no late Hausaufgaben at all",
};

function row(a: Assignment): unknown[] {
  return [a.uid, a.slug, a.title, a.due, a.course, a.effort_hours, a.effort_confidence,
    a.effort_source, a.importance, a.importance_reason, a.progress, a.created_by, a.body];
}

Deno.test("every field of all 13 buckets matches the frozen Python reference", async () => {
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const warnings: string[] = [];
  const items = parseDashboard(html, CFG, warnings);
  assertEquals(items.length, 13);
  const reference = JSON.parse(await Deno.readTextFile(new URL("vhl-parsed-reference.json", FIXTURES)));
  assertEquals(items.map(row), reference);
  // The reference is the only one in the port carrying a non-ASCII character — the em dash in
  // every title — which makes it the test that catches a writer slipping into ASCII escaping.
  assert(items[0].title.includes("—"));
});

Deno.test("an unauthenticated page is a dead session, never an empty semester", () => {
  assertThrows(
    () => parseDashboard("<html><body>Please sign in</body></html>", CFG, []),
    NotLoggedIn,
    "dashboard mount element absent",
  );
});

Deno.test("a section the config does not name is skipped with a warning", () => {
  const html = `<div class="js-student-dashboard-app" data-assignment-summaries="[{&quot;due_date&quot;:&quot;2026-09-01&quot;,&quot;detail_url&quot;:&quot;/sections/999/x&quot;,&quot;estimated_time&quot;:&quot;10m&quot;,&quot;assignment_count&quot;:1,&quot;activities_remaining&quot;:1,&quot;percentage_complete&quot;:0}]"></div>`;
  const warnings: string[] = [];
  assertEquals(parseDashboard(html, CFG, warnings).length, 0);
  assertEquals(warnings, ["section 999 not in config; skipped"]);
});

Deno.test("durations read the way Python read them, and an unreadable one is an error", () => {
  assertEquals(parseDurationHours("1h 22m"), 1.37);
  assertEquals(parseDurationHours("27m"), 0.45);
  assertEquals(parseDurationHours("3h 4m"), 3.07);
  // The empty string matches the pattern but fires neither group: returning 0.0 would be worse
  // than failing, because a zero-effort task fits any capacity gap and never needs a slot.
  assertThrows(() => parseDurationHours(""), Error, "unparseable duration");
  assertThrows(() => parseDurationHours("about an hour"), Error, "unparseable duration");
});

Deno.test("effort_hours is the WHOLE bucket, not what is left", () => {
  // estimated_time covers what remains; `Task.remaining_hours` is effort_hours * (1 - progress/100),
  // so a bucket 10% done with 1h 22m left is a 1.52-hour bucket.
  const html = `<div class="js-student-dashboard-app" data-assignment-summaries="[{&quot;due_date&quot;:&quot;2026-08-26&quot;,&quot;detail_url&quot;:&quot;/sections/2102121/x&quot;,&quot;estimated_time&quot;:&quot;1h 22m&quot;,&quot;assignment_count&quot;:10,&quot;activities_remaining&quot;:9,&quot;percentage_complete&quot;:10}]"></div>`;
  const items = parseDashboard(html, CFG, []);
  assertEquals(items[0].effort_hours, 1.52);
  assertEquals(items[0].progress, 10);
  assertEquals(items[0].effort_confidence, "high");
  assertEquals(items[0].effort_source, "vendor");
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/parse_vhl_test.ts`
Expected: FAIL — `Module not found "…/parse_vhl.ts"`.

- [ ] **Step 3: Write `parse_vhl.ts`.**

```ts
// The VHL dashboard page -> one task item per due-date bucket. A faithful port of
// `engine/src/vhl.rs`'s `parse_dashboard` and `coursework::parse_duration_hours`, measured against
// the same frozen Python-written reference (cloud design §4.3).
//
// The device still does the whole credentialed half — the CAS login on www.vhlcentral.com, the
// one-time `lt` ticket, the cookie jar that has to survive the host change to m3a.vhlcentral.com
// — and posts the page here (D11).
import { type Assignment, NotLoggedIn } from "./parse_zybooks.ts";
import { pyInt, pyStr, round2, slugify, truthy } from "./pyshims.ts";

export const MOUNT_MARKER = "js-student-dashboard-app";

const SUMMARIES = /data-assignment-summaries=(?:"([\s\S]*?)"|'([\s\S]*?)')/;
const SECTION = /\/sections\/(\d+)\//;
const DURATION = /^\s*(?:(\d+)\s*h)?\s*(?:(\d+)\s*m)?\s*$/i;

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

/**
 * `html_escape::decode_html_entities` over an attribute value.
 *
 * An HTML attribute can only carry the five XML entities and numeric references, and the captured
 * dashboard carries exactly one of them (`&quot;`, 490 times). An unrecognised named entity is
 * left as it was rather than guessed at, and the frozen reference is what proves this is enough
 * on the real capture.
 */
function decodeEntities(text: string): string {
  return text
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
    .replace(/&#(\d+);/g, (_, dec) => String.fromCodePoint(Number(dec)))
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

/** `"1h 22m"` -> 1.37. Throws on anything it cannot read: a zero-effort task fits any gap. */
export function parseDurationHours(text: string): number {
  const m = DURATION.exec(text);
  const unreadable = `unparseable duration: '${text}'`;
  if (m === null) throw new Error(unreadable);
  const hours = m[1] === undefined ? null : Number(m[1]);
  const minutes = m[2] === undefined ? null : Number(m[2]);
  // Both groups are optional, so the pattern also matches the empty string.
  if (hours === null && minutes === null) throw new Error(unreadable);
  return round2((hours ?? 0) + (minutes ?? 0) / 60);
}

/** `str(source.get(key) or "")`. */
function fieldStr(source: Obj, key: string): string {
  return truthy(source[key]) ? pyStr(source[key]) : "";
}

const WEEKDAY = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function parseDashboard(html: string, cfg: Obj, warnings: string[]): Assignment[] {
  if (!html.includes(MOUNT_MARKER)) {
    throw new NotLoggedIn("dashboard mount element absent (login page or redirect?)");
  }
  const captured = SUMMARIES.exec(html);
  if (captured === null) throw new NotLoggedIn("data-assignment-summaries attribute absent");
  const raw = decodeEntities(captured[1] ?? captured[2] ?? "");
  let summaries: unknown;
  try {
    summaries = JSON.parse(raw);
  } catch (e) {
    throw new NotLoggedIn(`data-assignment-summaries not valid JSON (${e instanceof Error ? e.message : e})`);
  }
  if (!Array.isArray(summaries)) {
    throw new NotLoggedIn(`data-assignment-summaries is ${summaries === null ? "NoneType" : typeof summaries}, not a list`);
  }

  const sections = truthy(cfg.sections) ? obj(cfg.sections) : {};
  const importance = "importance" in cfg ? pyInt(cfg.importance) : 3;
  const importanceReason = truthy(cfg.importance_reason) ? pyStr(cfg.importance_reason) : "";

  const out: Assignment[] = [];
  for (const entry of summaries) {
    if (entry === null || typeof entry !== "object" || Array.isArray(entry)) {
      throw new Error("a data-assignment-summaries entry is not an object");
    }
    const item = entry as Obj;
    const rawDate = fieldStr(item, "due_date");
    const detail = fieldStr(item, "detail_url");
    const sectionMatch = SECTION.exec(detail);
    if (sectionMatch === null) {
      warnings.push(`${rawDate}: no section id in detail_url; skipped`);
      continue;
    }
    const sectionId = sectionMatch[1];
    const mapping = sections[sectionId];
    if (!truthy(mapping) || typeof mapping !== "object" || Array.isArray(mapping)) {
      warnings.push(`section ${sectionId} not in config; skipped`);
      continue;
    }
    const map = mapping as Obj;
    // VHL has only ever emitted YYYY-MM-DD; the narrower reading warns and skips where Python's
    // `date.fromisoformat` would also accept the basic form and ISO week dates.
    if (!/^\d{4}-\d{2}-\d{2}$/.test(rawDate)) {
      warnings.push(`unreadable due_date '${rawDate}'; skipped`);
      continue;
    }
    let remaining: number;
    try {
      remaining = parseDurationHours(fieldStr(item, "estimated_time"));
    } catch (e) {
      warnings.push(`${rawDate}: ${e instanceof Error ? e.message : e}; skipped`);
      continue;
    }

    const progress = pyInt(item.percentage_complete);
    // estimated_time covers what is LEFT; effort_hours is the whole bucket, because
    // `Task.remaining_hours` is effort_hours * (1 - progress/100).
    const effort = progress < 100 ? round2(remaining / (1 - progress / 100)) : remaining;
    const count = pyInt(item.assignment_count);
    const label = truthy(map.label) ? pyStr(map.label) : "Hausaufgaben";
    const course = truthy(map.course) ? pyStr(map.course) : null;

    // `%a` and `%m-%d` over the naive local date. Built from the parts rather than from a Date, so
    // no time zone can shift the weekday of a date that never had a time.
    const [y, mo, d] = rawDate.split("-").map(Number);
    const weekday = WEEKDAY[new Date(Date.UTC(y, mo - 1, d)).getUTCDay()];

    out.push({
      uid: `vhl:${sectionId}:${rawDate}`,
      slug: `${slugify(course ?? "task")}-hausaufgaben-${rawDate}`,
      title: `${label} — due ${weekday} ${String(mo).padStart(2, "0")}-${String(d).padStart(2, "0")} (${count} activities)`,
      due: `${rawDate}T23:59`,
      course,
      effort_hours: effort,
      effort_confidence: "high",
      effort_source: "vendor",
      importance,
      importance_reason: importanceReason,
      progress,
      created_by: "vhl",
      body: `${count} VHL activities, ${"activities_remaining" in item ? pyStr(item.activities_remaining) : "None"} outstanding.\n` +
        `VHL's own estimate for what remains: **${"estimated_time" in item ? pyStr(item.estimated_time) : "None"}**.\n\n` +
        `Detail: \`${detail}\``,
    });
  }
  return out;
}
```

- [ ] **Step 4: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/parse_vhl_test.ts`
Expected: `ok | 5 passed | 0 failed`. If the first test fails, **fix the port, never the reference**. Two likely culprits, in order: the em-dash title format (`{label} — due {%a} {%m-%d} ({n} activities)`) and the `effort_hours` division at `progress = 0`.

- [ ] **Step 5: Confirm the fixtures were only read.**

Run: `git status --porcelain --untracked-files=all engine/tests/fixtures/`
Expected: no output; paste it into the report.

- [ ] **Step 6: Add the doc line to `engine/src/vhl.rs`.** One sentence at the head of `parse_dashboard`'s doc comment, and nothing else in that file:

```rust
/// **This function is the reference the server-side port is measured against** (cloud design
/// §4.3): `cloud/supabase/functions/ingest-coursework/parse_vhl.ts` is a faithful TypeScript port
/// of it, and both are gated by the same frozen `tests/fixtures/vhl-parsed-reference.json`. From
/// C2 on, production parses on the server; the fetch half above stays here, because a portal
/// password never leaves the machine (D11).
```

- [ ] **Step 7: Commit.**

```bash
git add cloud/supabase/functions/ingest-coursework/parse_vhl.ts cloud/supabase/functions/ingest-coursework/parse_vhl_test.ts engine/src/vhl.rs
git commit -F .git-commit-msg.txt   # "cloud: the VHL parser moves server-side, gated by the frozen Python reference (C2 Task 6)"
```

---

### Task 7: `/ingest-coursework` — fetch on device, think in the cloud

**Files:**
- Create: `cloud/supabase/functions/ingest-coursework/handler.ts`, `cloud/supabase/functions/ingest-coursework/index.ts`
- Test: `cloud/supabase/functions/ingest-coursework/handler_test.ts`, and new tests inside `engine/src/coursework.rs`
- Modify: `engine/src/coursework.rs` (the fetch/parse split), `engine/src/zybooks.rs` (`fetch_payloads` — the fetch half, exposed)

**Interfaces:**
- Consumes: Tasks 5 and 6's parsers; Task 4's `CloudClient`.
- Produces:
  - Request `{ timezone: string, sources: Array<{ name: "zybooks", config: object, books: Array<{ code: string, payload: unknown }> } | { name: "vhl", config: object, html: string }> }`
  - Reply `{ assignments: Assignment[], warnings: string[] }` — warnings already carry the `"{source}: "` prefix `collect` gives them. **Task 7a adds a third field, `proposals`** (R-OB-1).
  - `zybooks::fetch_payloads(email: &str, password: &str) -> Result<Vec<(String, Json)>, SourceError>` — sign in, list the codes, fetch each book's payload. **No parsing, no routing.** It takes the two strings rather than the config mapping because the credential is read by the caller (`coursework::collect_cloud`, which owns `wincred`) and this function must never be a second place a `credential_target` is resolved. `#[cfg(windows)]`, like every function on the credentialed path.
  - `coursework::CLOUD_CONFIG_KEYS: [&str; 2]` (`["zybooks", "vhl"]`) and `coursework::redact(cfg: &Mapping, source: &str) -> Mapping`
  - `coursework::collect_cloud(config: &Mapping, warnings: &mut Vec<String>, client: &CloudClient) -> Vec<Assignment>`

- [ ] **Step 1: Write the failing server test** — `cloud/supabase/functions/ingest-coursework/handler_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { ingestHandler } from "./handler.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);
const OK = () => Promise.resolve({ account_id: "acct-1" });

Deno.test("both sources parse and reconcile in one round trip, warnings prefixed by source", async () => {
  const payload = JSON.parse((await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES))).replace(/^\uFEFF/, ""));
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [
      {
        name: "zybooks",
        config: {
          courses: { "cs-100-2026": { course: "cs-100", label: "CS 100" } },
          ignore: ["HowToUseZyBooks2"],
          categories: { HW: "hw", Lab: "lab", Project: "project" },
          effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
          importance: { hw: 2, lab: 2, project: 2 },
        },
        books: [{ code: "cs-100-2026", payload }, { code: "HowToUseZyBooks2", payload: { assignments: [] } }, { code: "surprise-101", payload: { assignments: [] } }],
      },
      {
        name: "vhl",
        config: {
          sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } },
          importance: 3,
          importance_reason: "GN 103 accepts no late Hausaufgaben at all",
        },
        html,
      },
    ],
  });
  const response = await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assertEquals(reply.assignments.length, 24 + 13);
  assertEquals(reply.warnings, ["zybooks: zybook surprise-101 not in config; skipped"]);
});

Deno.test("a dead session is a named warning and zero items, never an empty semester", async () => {
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{ name: "zybooks", config: {}, books: [{ code: "x", payload: { success: false, error: "invalid token" } }] }],
  });
  const response = await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }));
  const reply = await response.json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.warnings.length, 1);
  assertEquals(reply.warnings[0].startsWith("zybooks: session invalid"), true);
});

Deno.test("one source failing never stops the other", async () => {
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [
      { name: "zybooks", config: {}, books: [{ code: "x", payload: { success: false } }] },
      { name: "vhl", config: { sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } }, importance: 3, importance_reason: "GN 103 accepts no late Hausaufgaben at all" }, html },
    ],
  });
  const reply = await (await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }))).json();
  assertEquals(reply.assignments.length, 13);
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/handler_test.ts`
Expected: FAIL — `Module not found "…/handler.ts"`.

- [ ] **Step 3: Write `handler.ts`.**

```ts
// POST /ingest-coursework — the device's raw portal payload in, reconciled items out.
//
// Never a credential: the device signs in with the student's own password out of Credential
// Manager and stops before parsing (D11, §4.3, §9). This function has no way to fetch anything and
// no place to keep a password, and the engine's `the_coursework_payload_carries_no_credential`
// test is the other half of the proof.
//
// *An empty parse is a failure, never an empty semester* — the rule `collect` has always kept: a
// 200 yielding nothing is far more likely to be a dead session than a term with no homework.
import type { Entitle } from "../_shared/judge_handler.ts";
import { type Assignment, NotLoggedIn, parseAssignments, routeZybook } from "./parse_zybooks.ts";
import { parseDashboard } from "./parse_vhl.ts";

type Obj = Record<string, unknown>;

function obj(value: unknown): Obj {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Obj : {};
}

function pickZybooks(source: Obj, timeZone: string, warnings: string[]): Assignment[] {
  const cfg = obj(source.config);
  const courses = obj(cfg.courses);
  const ignore = Array.isArray(cfg.ignore) ? cfg.ignore.map(String) : [];
  const out: Assignment[] = [];
  for (const raw of Array.isArray(source.books) ? source.books : []) {
    const book = obj(raw);
    const code = String(book.code ?? "");
    const routing = routeZybook(code, courses, ignore);
    if (routing.kind === "ignored") continue;
    if (routing.kind === "unmapped") {
      warnings.push(`zybook ${code} not in config; skipped`);
      continue;
    }
    out.push(...parseAssignments(
      book.payload,
      String(routing.mapping.course ?? ""),
      String(routing.mapping.label ?? ""),
      cfg,
      timeZone,
      warnings,
    ));
  }
  return out;
}

export function ingestHandler(entitle: Entitle): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      await entitle(req);
      let body: Obj;
      try {
        body = obj(await req.json());
      } catch {
        return Response.json({ error: "the body is not JSON" }, { status: 400 });
      }
      const timeZone = typeof body.timezone === "string" && body.timezone !== "" ? body.timezone : "America/Chicago";
      const assignments: Assignment[] = [];
      const warnings: string[] = [];
      for (const raw of Array.isArray(body.sources) ? body.sources : []) {
        const source = obj(raw);
        const name = String(source.name ?? "");
        const own: string[] = [];
        let items: Assignment[] = [];
        try {
          if (name === "zybooks") items = pickZybooks(source, timeZone, own);
          else if (name === "vhl") items = parseDashboard(String(source.html ?? ""), obj(source.config), own);
          else {
            warnings.push(`${name}: unknown source; nothing changed`);
            continue;
          }
        } catch (e) {
          const message = e instanceof Error ? e.message : String(e);
          warnings.push(e instanceof NotLoggedIn
            ? `${name}: session invalid (${message}); nothing changed`
            : `${name}: parse failed (${message}); nothing changed`);
          continue;
        }
        warnings.push(...own.map((w) => `${name}: ${w}`));
        if (items.length === 0) {
          warnings.push(`${name}: 0 assignments parsed; treating as failure`);
          continue;
        }
        assignments.push(...items);
      }
      return Response.json({ assignments, warnings });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-coursework: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
```

and `index.ts`:

```ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { ingestHandler } from "./handler.ts";

Deno.serve(ingestHandler(requireActiveEntitlement));
```

- [ ] **Step 4: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/`
Expected: `ok | 16 passed | 0 failed` (8 + 5 + 3).

- [ ] **Step 5: Write the failing engine test** — inside `engine/src/coursework.rs`'s test module:

```rust
    /// D11 and the VISION amendment, at the boundary that now exists: the payload the device posts
    /// carries the vendor's own bytes and the parser's config, and **no credential of any kind**.
    /// The device signs in; the server never could.
    #[test]
    fn the_coursework_payload_carries_no_credential() {
        let cfg = crate::yaml::mapping_of(concat!(
            "enabled: true\n",
            "credential_target: knowlu/test-profile/zybooks\n",
            "base_url: https://www.vhlcentral.com\n",
            "courses:\n  cs-100-2026:\n    course: cs-100\n    label: CS 100\n",
            "ignore:\n  - HowToUseZyBooks2\n",
            "categories:\n  HW: hw\n",
            "effort:\n  minutes_per_section: 6\n  floors:\n    hw: 0.25\n",
            "importance:\n  hw: 2\n",
        ));
        let redacted = redact(&cfg, "zybooks");
        let rendered = crate::ledger::dumps_value(&serde_json::json!({
            "timezone": "America/Chicago",
            "sources": [{ "name": "zybooks", "config": yaml_to_json_for_request(&redacted), "books": [] }],
        }));
        for forbidden in ["credential_target", "knowlu/test-profile", "base_url", "enabled", "password"] {
            assert!(!rendered.contains(forbidden), "the request body carried {forbidden:?}: {rendered}");
        }
        // And everything the parser genuinely needs did travel.
        for needed in ["courses", "cs-100-2026", "categories", "minutes_per_section", "importance", "ignore"] {
            assert!(rendered.contains(needed), "the request body lost {needed:?}: {rendered}");
        }
    }

    /// The reconciled list the service returns is written by the same `sync_coursework` the local
    /// parse fed, so a cloud run and a local run produce the same notes.
    #[test]
    fn a_reply_row_becomes_the_same_assignment_the_local_parser_produced() {
        let row = serde_json::json!({
            "uid": "zybooks:1839992", "slug": "cs-100-hw-01", "title": "CS 100 HW 01",
            "due": "2026-08-26T23:59", "course": "cs-100", "effort_hours": 2.5,
            "effort_confidence": "low", "effort_source": "inferred", "importance": 2,
            "importance_reason": "zyBooks hw worth 193 points across 25 sections; per-category importance from config",
            "progress": 0, "created_by": "zybooks", "body": "25 zyBooks section(s), 193 points.\n\n- 1.1 x"
        });
        let got = assignment_from_row(&row).expect("a well-formed row parses");
        assert_eq!(got.uid, "zybooks:1839992");
        assert_eq!(got.due, jiff::civil::date(2026, 8, 26).at(23, 59, 0, 0));
        assert_eq!(got.effort_hours, 2.5);
        assert_eq!(got.course.as_deref(), Some("cs-100"));
    }

    /// A row the service sent that the engine cannot read is dropped with a warning, never
    /// written half-formed: a note with no due date ranks as though it had none.
    #[test]
    fn a_malformed_reply_row_is_a_warning_and_not_a_note() {
        let row = serde_json::json!({ "uid": "zybooks:1", "title": "x", "due": "not a date" });
        assert!(assignment_from_row(&row).is_none());
    }
```

- [ ] **Step 6: Run it and watch it fail.**

Run: `cargo test -p knowlu-engine coursework::`
Expected: FAIL to compile — `cannot find function redact`, `assignment_from_row`, `yaml_to_json_for_request`.

- [ ] **Step 7: Write the split in `engine/src/coursework.rs`.**

```rust
/// The keys the redacted config may carry, per source. **An allowlist, not a denylist** — a config
/// grows keys and a denylist is a list of things somebody remembered. `credential_target` is the
/// one that matters (a portal password's address is not the server's business, D11), but so are
/// `base_url` and `enabled`, which are the device's own operational settings.
pub const REDACT_ALLOW_ZYBOOKS: [&str; 5] = ["courses", "ignore", "categories", "effort", "importance"];
pub const REDACT_ALLOW_VHL: [&str; 3] = ["sections", "importance", "importance_reason"];

/// What of a source's config may travel to `/ingest-coursework`.
pub fn redact(cfg: &Mapping, source: &str) -> Mapping {
    let allow: &[&str] = match source {
        "zybooks" => &REDACT_ALLOW_ZYBOOKS,
        "vhl" => &REDACT_ALLOW_VHL,
        _ => &[],
    };
    let mut out = Mapping::new();
    for key in allow {
        if let Some(value) = crate::yaml::get(cfg, key) {
            out.insert(Yaml::String((*key).to_string()), value.clone());
        }
    }
    out
}

/// A YAML mapping as the JSON the request body carries. Only the four scalar kinds and the two
/// containers — a YAML tag, an alias or a date in a coursework config is not something the parser
/// reads, and dropping it is safer than inventing a JSON spelling for it.
pub fn yaml_to_json_for_request(value: &Mapping) -> serde_json::Value {
    fn one(value: &Yaml) -> serde_json::Value {
        match value {
            Yaml::Null => serde_json::Value::Null,
            Yaml::Bool(b) => serde_json::Value::Bool(*b),
            Yaml::Number(n) => n
                .as_i64().map(serde_json::Value::from)
                .or_else(|| n.as_f64().and_then(serde_json::Number::from_f64).map(serde_json::Value::Number))
                .unwrap_or(serde_json::Value::Null),
            Yaml::String(s) => serde_json::Value::String(s.clone()),
            Yaml::Sequence(items) => serde_json::Value::Array(items.iter().map(one).collect()),
            Yaml::Mapping(map) => serde_json::Value::Object(
                map.iter()
                    .filter_map(|(k, v)| Some((crate::pystr::yaml_str(k), one(v))))
                    .collect(),
            ),
            _ => serde_json::Value::Null,
        }
    }
    one(&Yaml::Mapping(value.clone()))
}

/// One reply row back into an [`Assignment`]. `None` for anything the engine cannot read — a note
/// written from a half-formed row would rank as though it had no deadline.
pub fn assignment_from_row(row: &serde_json::Value) -> Option<Assignment> {
    let text = |key: &str| row.get(key).and_then(serde_json::Value::as_str).map(str::to_string);
    let due = DateTime::strptime("%Y-%m-%dT%H:%M", text("due")?.as_str()).ok()?;
    Some(Assignment {
        uid: text("uid")?,
        slug: text("slug")?,
        title: text("title")?,
        due,
        course: row.get("course").and_then(serde_json::Value::as_str).map(str::to_string),
        effort_hours: row.get("effort_hours").and_then(serde_json::Value::as_f64)?,
        effort_confidence: text("effort_confidence").unwrap_or_default(),
        effort_source: text("effort_source").unwrap_or_default(),
        importance: row.get("importance").and_then(serde_json::Value::as_i64)?,
        importance_reason: text("importance_reason").unwrap_or_default(),
        progress: row.get("progress").and_then(serde_json::Value::as_i64).unwrap_or(0),
        created_by: text("created_by").unwrap_or_default(),
        body: text("body").unwrap_or_default(),
    })
}

/// *Fetch on device, think in the cloud* (D11, §4.3). Every credentialed request is made here with
/// the student's own password out of Credential Manager; the raw payload then goes to
/// `/ingest-coursework` and the reconciled list comes back.
///
/// One source's failure never stops another, and the server applies the same
/// *an-empty-parse-is-a-failure* rule `collect` applies locally.
#[cfg(windows)]
pub fn collect_cloud(
    config: &Mapping,
    warnings: &mut Vec<String>,
    client: &crate::cloudmodel::CloudClient,
) -> Vec<Assignment> {
    let block = match crate::yaml::get(config, "coursework") {
        Some(Yaml::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    let tz_name = match crate::yaml::get(config, "timezone") {
        Some(value) => crate::yaml::text(value).unwrap_or_else(|| DEFAULT_TZ.to_string()),
        None => DEFAULT_TZ.to_string(),
    };
    let mut sources: Vec<serde_json::Value> = Vec::new();

    for name in ["zybooks", "vhl"] {
        let cfg = match crate::yaml::get(&block, name) {
            Some(Yaml::Mapping(map)) => map.clone(),
            _ => continue,
        };
        if !crate::yaml::get(&cfg, "enabled").map(crate::pystr::yaml_truthy).unwrap_or(false) {
            continue;
        }
        let credential = match crate::wincred::read_credential(&cfg_str(&cfg, "credential_target", "")) {
            Ok(c) => c,
            Err(err) => {
                warnings.push(format!("{name}: fetch failed ({err}); nothing changed"));
                continue;
            }
        };
        let redacted = yaml_to_json_for_request(&redact(&cfg, name));
        match name {
            "zybooks" => match crate::zybooks::fetch_payloads(&credential.username, credential.password.expose()) {
                Ok(books) => sources.push(serde_json::json!({
                    "name": "zybooks", "config": redacted,
                    "books": books.into_iter().map(|(code, payload)| serde_json::json!({"code": code, "payload": payload})).collect::<Vec<_>>(),
                })),
                Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("zybooks: session invalid ({m}); nothing changed")),
                Err(SourceError::Failed(m)) => warnings.push(format!("zybooks: fetch failed ({m}); nothing changed")),
            },
            _ => {
                let base = cfg_str(&cfg, "base_url", "https://www.vhlcentral.com");
                match crate::vhl::login_and_fetch_dashboard(&credential.username, credential.password.expose(), &base, None) {
                    Ok(html) => sources.push(serde_json::json!({"name": "vhl", "config": redacted, "html": html})),
                    Err(SourceError::NotLoggedIn(m)) => warnings.push(format!("vhl: session invalid ({m}); nothing changed")),
                    Err(SourceError::Failed(m)) => warnings.push(format!("vhl: fetch failed ({m}); nothing changed")),
                }
            }
        }
    }

    if sources.is_empty() {
        return Vec::new();
    }
    let body = serde_json::json!({ "timezone": tz_name, "sources": sources });
    let reply = match client.post("/ingest-coursework", &body) {
        Ok(reply) => reply,
        Err(err) => {
            warnings.push(format!("coursework: the service is unavailable ({err}); nothing changed"));
            return Vec::new();
        }
    };
    for warning in reply.get("warnings").and_then(serde_json::Value::as_array).into_iter().flatten() {
        if let Some(text) = warning.as_str() {
            warnings.push(text.to_string());
        }
    }
    let mut out = Vec::new();
    for row in reply.get("assignments").and_then(serde_json::Value::as_array).into_iter().flatten() {
        match assignment_from_row(row) {
            Some(item) => out.push(item),
            None => warnings.push("coursework: a reply row could not be read; skipped".to_string()),
        }
    }
    out
}
```

plus the non-Windows stub, because `main_with_fetchers` is **not** cfg-gated and `coursework.rs` already keeps `fetch_zybooks` / `fetch_vhl` stubs for exactly this reason (spec §6.5: the module must compile away cleanly so a Linux build never needs `advapi32`):

```rust
/// The credential store is Windows-only (spec §6.5), so the credentialed fetch is too. A cloud
/// build still compiles; it simply has no way to authenticate, and says so rather than pretending
/// the semester is empty — the same shape `fetch_zybooks` and `fetch_vhl` already have.
#[cfg(not(windows))]
pub fn collect_cloud(
    _config: &Mapping,
    warnings: &mut Vec<String>,
    _client: &crate::cloudmodel::CloudClient,
) -> Vec<Assignment> {
    warnings.push("coursework: credential store unavailable on this platform".to_string());
    Vec::new()
}
```

and, in `main_with_fetchers`, one branch so the cloud path is taken when the vault has an account and the caller supplied no test fetchers:

```rust
        assignments = match (fetchers, crate::cloudmodel::resolve(vault).ok()) {
            // `fetchers` is the test seam and always wins; a vault with an account parses on the
            // server (§4.3); everything else is plan-3a's local path, unchanged until C4.
            (None, Some(client)) => collect_cloud(&config, &mut warnings, &client),
            _ => collect(vault, &config, &mut warnings, fetchers),
        };
```

- [ ] **Step 8: Expose the fetch half in `engine/src/zybooks.rs`** — one new function, and **no change to `parse_assignments`' body**:

```rust
/// The fetch half alone: sign in, list the student's zybooks, and fetch each book's raw payload.
///
/// **No parsing and no routing** — both moved server-side in C2 (§4.3). What stays here is exactly
/// what needs the student's own credentials, which never leave the machine (D11).
#[cfg(windows)]
pub fn fetch_payloads(email: &str, password: &str) -> Result<Vec<(String, Json)>, SourceError> {
    let (token, user_id) = signin(email, password, None)?;
    let mut out = Vec::new();
    for code in fetch_zybook_codes(&token, user_id, None)? {
        let payload = fetch_assignments(&token, &code, None)?;
        out.push((code, payload));
    }
    Ok(out)
}
```

- [ ] **Step 9: Run everything and watch it pass.**

Run: `cargo test --workspace`
Expected: green at 0 warnings, and **`oracle.rs` and `surface_oracle.rs` unchanged** — the split must not change a byte of `today.md` on the three fixture vaults, because nothing about the local path moved. Record both test counts.

- [ ] **Step 10: Deploy and smoke.**

```
supabase functions deploy ingest-coursework --project-ref <staging ref>
```

Then post the committed zyBooks fixture through the real endpoint with a staging JWT and assert 24 assignments come back. **Never run `knowlu-engine coursework` against a real vault to test this** (ruling R-3a-12) — the fixture and a temp copy are the whole test surface.

- [ ] **Step 11: Commit.**

```bash
git add cloud/supabase/functions/ingest-coursework/handler.ts cloud/supabase/functions/ingest-coursework/handler_test.ts cloud/supabase/functions/ingest-coursework/index.ts engine/src/coursework.rs engine/src/zybooks.rs
git commit -F .git-commit-msg.txt   # "engine+cloud: coursework fetches on the device and parses in the service; no credential ever travels (C2 Task 7)"
```

---

---

### Task 7a: R-OB-1 — an unmapped book or section is a proposal, never a silent skip

**Why this task exists.** Quinn's first slot on the fresh vault produced no zyBooks or VHL work at all. The wizard had stored both portal logins, but nothing had mapped the discovered zyBook and the discovered VHL section to a course — so `route_zybook` answered `Unmapped`, `parse_dashboard` answered `section … not in config`, and both became one `WARN` line in `state/runner-log.md` that nobody reads at 8am. **A source the student connected, that authenticated, that returned a payload, and that then produced nothing, is the failure this system exists to prevent** (VISION: *a source that later breaks fails visibly rather than silently going quiet*). It becomes a card in the deck.

**Where the mapping lives — decided, and it is the vault** (§4.3: *the parsers move; the credentials and the vault stay*). The three candidates were the vault's `config/ingest.yaml`, a server-side row beside `sources`, and both. It is the vault, for three reasons: the mapping is **already there** and the device already sends a redacted copy of it on every `/ingest-coursework` call, so nothing new travels; **C1's wizard writes the initial mapping** into that same file (R-OB-1's other half), and a second home would make two writers of one value; and a student can open and edit it, which is VISION's *every automation is editable text*. A server-side copy would be a second source of truth for a value the parser only ever sees as a request field anyway.

**And the write into it is a text-level insertion** (ruling **R-C2-8**). The obvious implementation — parse the file with `serde_yaml_ng`, insert the key, `to_string` it back — is forbidden: CLAUDE.md's *no vault file is parsed and re-dumped* has no config exemption, and `src/yamlemit.rs` is the crate's one YAML emitter. `write_mapping` (step 7) therefore finds the `courses:` / `sections:` line and splices three lines in beneath it at the file's own indentation, rewriting exactly one existing line and only when it is the birth shape `courses: {}`. **It can only add a key, and that is free**: a book that is already mapped never yields a card, so no card ever names a key the file already has.

**Files:**
- Modify: `cloud/supabase/functions/ingest-coursework/handler.ts` (the reply gains `proposals`), `engine/src/coursework.rs` (write the cards, apply an approved one)
- Test: `cloud/supabase/functions/ingest-coursework/handler_test.ts`, new tests in `engine/src/coursework.rs`
- **Hand-off this task needs:** H12 (`engine/src/approvals.rs` — one arm for `kind: coursework-map`)

**Interfaces:**
- Consumes: Task 7's `ingestHandler`, `routeZybook`, `collect_cloud`.
- Produces:
  - The reply gains `proposals: Array<{ source: "zybooks" | "vhl", key: string, label: string, suggested_course: string | null }>`.
  - `suggestCourse(name: string): string | null` in `handler.ts`.
  - `coursework::MAP_ACTOR: &str = "agent:knowlu.coursework"`, `coursework::write_map_card`, `coursework::apply_map_cards`.

- [ ] **Step 1: Write the failing server test** — append to `cloud/supabase/functions/ingest-coursework/handler_test.ts`:

```ts
Deno.test("an unknown zybook is one proposal and zero warnings", async () => {
  // R-OB-1: the old behaviour was `warnings: ["zybooks: zybook X not in config; skipped"]` and
  // nothing else — a line in a log nobody reads, on the very first run of a vault whose whole
  // point was to show the student their coursework.
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{
      name: "zybooks",
      config: { courses: {}, ignore: ["HowToUseZyBooks2"], categories: {}, effort: {}, importance: {} },
      books: [{ code: "UACS100Fall2026", payload: { assignments: [] } }],
    }],
  });
  const reply = await (await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }))).json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.warnings, [], "an unmapped book is a proposal, not a warning");
  assertEquals(reply.proposals, [{
    source: "zybooks",
    key: "UACS100Fall2026",
    label: "UACS100Fall2026",
    suggested_course: "cs-100",
  }]);
});

Deno.test("an ignored book is still silent, and a known one still yields its items", async () => {
  // `HowToUseZyBooks2` is zyBooks' own onboarding book: a proposal there would fire on every
  // healthy run for every student, which is exactly the noise R-OB-1 is trying to stop.
  const payload = JSON.parse((await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES))).replace(/^\uFEFF/, ""));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{
      name: "zybooks",
      config: {
        courses: { "cs-100-2026": { course: "cs-100", label: "CS 100" } },
        ignore: ["HowToUseZyBooks2"],
        categories: { HW: "hw", Lab: "lab", Project: "project" },
        effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
        importance: { hw: 2, lab: 2, project: 2 },
      },
      books: [{ code: "cs-100-2026", payload }, { code: "HowToUseZyBooks2", payload: { assignments: [] } }],
    }],
  });
  const reply = await (await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }))).json();
  assertEquals(reply.assignments.length, 24);
  assertEquals(reply.proposals, []);
  assertEquals(reply.warnings, []);
});

Deno.test("an unmapped VHL section becomes a proposal with no course to suggest", async () => {
  // The dashboard carries a section id and a due date and no course name at all, so there is
  // nothing to guess from — and a guess would be worse than a question. The card asks.
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{ name: "vhl", config: { sections: {}, importance: 3, importance_reason: "" }, html }],
  });
  const reply = await (await ingestHandler(OK)(new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }))).json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.proposals, [{ source: "vhl", key: "2102121", label: "VHL section 2102121", suggested_course: null }]);
  assertEquals(reply.warnings.filter((w: string) => w.includes("not in config")), []);
});

Deno.test("a course is suggested from a zybook code, or not at all", () => {
  // The institution prefix comes off only when the term suffix says the code carries one, so a
  // department that happens to be four letters keeps all four.
  assertEquals(suggestCourse("UACS100Fall2026"), "cs-100");
  assertEquals(suggestCourse("UAMATH120Fall2026"), "math-120");
  assertEquals(suggestCourse("PH106Spring2027"), "ph-106");
  assertEquals(suggestCourse("MATH125"), "math-125");
  assertEquals(suggestCourse("cs-100-2026"), "cs-100");
  // zyBooks' own onboarding book, and a VHL section id: neither is a course code.
  assertEquals(suggestCourse("HowToUseZyBooks2"), null);
  assertEquals(suggestCourse("2102121"), null);
  assertEquals(suggestCourse("SomeBookWithNoCode"), null);
  assertEquals(suggestCourse(""), null);
});
```

- [ ] **Step 2: Run it and watch it fail.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/` → `reply.proposals` is `undefined` and the warnings are the old ones.

- [ ] **Step 3: Write the server half** in `cloud/supabase/functions/ingest-coursework/handler.ts`.

```ts
/// One thing the account's mapping does not know about (R-OB-1).
export interface MapProposal {
  source: "zybooks" | "vhl";
  /** What goes in the config: the zybook code, or the VHL section id. */
  key: string;
  /**
   * What a human reads on the card — **display only**. It is the vendor's own name for the book
   * (`UACS100Fall2026`, `VHL section 2102121`), which is exactly what the config's `label:` must
   * NOT be: that one prefixes every title the parser produces (`CS 100 HW 01`), so `write_mapping`
   * derives it from the course slug the student confirmed and never from this.
   */
  label: string;
  /** A guess the student confirms or replaces, or null when there is nothing to guess from. */
  suggested_course: string | null;
}

/** zyBooks' term words. Their presence at the end is what marks an institution-hosted code. */
const TERM = /(?:Spring|Summer|Fall|Winter)\s?\d{4}\s*$/i;

/**
 * A course slug from a zybook code, or null.
 *
 * zyBooks mints an **institution-hosted** book as `<II><DEPT><number><Term><Year>` —
 * `UACS100Fall2026` is the University of Alabama's CS 100 — and a plain catalogue book as
 * `<DEPT><number>`, `MATH125`. **The term suffix is the only marker there is**: nothing inside the
 * letter run says where `UA` stops and `CS` starts, and an unconditional strip would turn `MATH125`
 * into `th-125`. So the two-letter institution prefix comes off only when the code carries a term
 * AND the letter run is longer than a two-letter department could be. That leaves `MATH125` and
 * `PH106Spring2027` alone and turns `UAMATH120Fall2026` into `math-120`.
 *
 * A three-letter institution abbreviation would still guess wrong, and that is **allowed**: the
 * card shows the guess, the student confirms or replaces it, and `write_mapping` never writes a
 * value nobody confirmed. A wrong guess costs one tap.
 *
 * `null` when nothing matches, because an invented slug on a card is worse than a blank one: the
 * student would have to notice it was wrong rather than fill it in.
 */
export function suggestCourse(name: string): string | null {
  const m = /^([A-Za-z]{2,8})[\s_-]?(\d{3,4})(.*)$/.exec(name.trim());
  if (m === null) return null;
  const [, letters, number, tail] = m;
  const dept = TERM.test(tail) && letters.length > 2 ? letters.slice(2) : letters;
  return `${dept.toLowerCase()}-${number}`;
}

/** `section <id> not in config; skipped` — the one VHL warning that is really a proposal. */
const VHL_UNMAPPED = /^section (\d+) not in config; skipped$/;
```

`pickZybooks` gains the `proposals` out-parameter and stops warning:

```ts
function pickZybooks(
  source: Obj,
  timeZone: string,
  warnings: string[],
  proposals: MapProposal[],
): Assignment[] {
  const cfg = obj(source.config);
  const courses = obj(cfg.courses);
  const ignore = Array.isArray(cfg.ignore) ? cfg.ignore.map(String) : [];
  const out: Assignment[] = [];
  for (const raw of Array.isArray(source.books) ? source.books : []) {
    const book = obj(raw);
    const code = String(book.code ?? "");
    const routing = routeZybook(code, courses, ignore);
    // `ignored` stays silent: `HowToUseZyBooks2` is zyBooks' own onboarding book, and a proposal
    // there would fire on every healthy run for every student.
    if (routing.kind === "ignored") continue;
    if (routing.kind === "unmapped") {
      // R-OB-1: never a warning. A book that authenticated and returned a payload and then
      // produced nothing is the failure the deck exists to surface.
      proposals.push({ source: "zybooks", key: code, label: code, suggested_course: suggestCourse(code) });
      continue;
    }
    out.push(...parseAssignments(
      book.payload,
      String(routing.mapping.course ?? ""),
      String(routing.mapping.label ?? ""),
      cfg,
      timeZone,
      warnings,
    ));
  }
  return out;
}
```

and the VHL branch **translates the parser's own warning** rather than changing the parser — `parse_vhl.ts` stays byte-identical to the frozen `vhl-parsed-reference.json`, and the policy lives in the handler where the zyBooks policy already does:

```ts
        if (name === "vhl") {
          items = parseDashboard(String(source.html ?? ""), obj(source.config), own);
          // The parser reports an unmapped section as a warning because that is what the Python it
          // was ported from did, and its behaviour is frozen against a reference. R-OB-1 is a
          // policy about what to DO with that, so it is applied here: the warning becomes a
          // proposal and is dropped from the list.
          for (let i = own.length - 1; i >= 0; i--) {
            const m = VHL_UNMAPPED.exec(own[i]);
            if (m === null) continue;
            proposals.push({
              source: "vhl",
              key: m[1],
              // The dashboard carries a section id and due dates and no course name at all, so
              // there is nothing to guess from — and a guess would be worse than a question.
              label: `VHL section ${m[1]}`,
              suggested_course: null,
            });
            own.splice(i, 1);
          }
        }
```

with `const proposals: MapProposal[] = []` beside `assignments` and `warnings`, `proposals` passed into `pickZybooks`, and the reply becoming `Response.json({ assignments, warnings, proposals })`.

**One consequence, and it is deliberate:** a source whose every book is unmapped now returns zero assignments, which still trips the *an-empty-parse-is-a-failure* rule and still warns `zybooks: 0 assignments parsed; treating as failure`. That warning is correct — the run really did produce nothing — and it now sits **beside** a card that says what to do about it, which is the whole difference.

- [ ] **Step 4: Run it and watch it pass.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-coursework/` → 4 more tests than at the end of Task 7.

- [ ] **Step 5: Write the failing engine tests** — in `engine/src/coursework.rs`'s test module:

```rust
    /// R-OB-1: the reply's proposals become cards in the deck, one per unmapped source.
    #[test]
    fn an_unmapped_book_becomes_a_card_the_deck_can_answer() {
        let vault = scratch_vault("map-card");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let stem = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(),
                key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(),
                suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .expect("the card writes");
        let card = std::fs::read_to_string(vault.join("approvals").join(format!("{stem}.md"))).unwrap();
        assert!(card.contains("kind: coursework-map"));
        assert!(card.contains("source: zybooks"));
        assert!(card.contains("map_key: UACS100Fall2026"));
        assert!(card.contains("course: cs-100"));
        assert!(card.contains("status: pending"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card is never minted twice for one key — the next slot fetches the same unmapped book.
    #[test]
    fn a_map_card_is_never_minted_twice_for_one_key() {
        let vault = scratch_vault("map-once");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let p = MapProposal {
            source: "vhl".into(), key: "2102121".into(),
            label: "VHL section 2102121".into(), suggested_course: None,
        };
        let today = jiff::civil::date(2026, 9, 9);
        assert!(write_map_card(&vault, &p, today, &ctx, &mut journal).is_ok());
        assert!(write_map_card(&vault, &p, today, &ctx, &mut journal).is_err());
        assert_eq!(crate::approvals::sorted_md(&vault.join("approvals")).len(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card the student REJECTED is not re-asked on the next slot — `write::create`'s `Exists`
    /// guard only sees `approvals/`, and a rejected card lives in `archive/`.
    #[test]
    fn a_rejected_map_card_is_not_re_asked_until_it_expires() {
        let vault = scratch_vault("map-rejected");
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);
        let p = MapProposal {
            source: "zybooks".into(), key: "UACS100Fall2026".into(),
            label: "UACS100Fall2026".into(), suggested_course: Some("cs-100".into()),
        };
        let stem = write_map_card(&vault, &p, today, &ctx, &mut journal).unwrap();

        // The student says no, and `process_approvals` settles the card into `archive/`.
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "rejected".to_string())],
            &ctx, &mut journal, &crate::write::WriteOpts::default(),
        )
        .unwrap();
        crate::write::delete(&vault, &format!("approvals/{stem}.md"), &ctx, &mut journal).unwrap();
        assert!(crate::approvals::sorted_md(&vault.join("approvals")).is_empty());

        // The next slot fetches the same unmapped book and asks nothing.
        let key = ("zybooks".to_string(), "UACS100Fall2026".to_string());
        assert!(asked_map_keys(&vault, today).contains(&key), "the archived card is the memory");
        // …until it expires, 30 days out, when the question is worth asking again.
        assert!(
            asked_map_keys(&vault, jiff::civil::date(2026, 10, 10)).is_empty(),
            "asked again next month, not twice a day forever"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// Approving one writes the mapping into `config/ingest.yaml` and archives the card.
    #[test]
    fn an_approved_map_card_writes_the_mapping_and_is_archived() {
        let vault = scratch_vault("map-apply");
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n",
        )
        .unwrap();
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let stem = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(), key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(), suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9),
            &ctx,
            &mut journal,
        )
        .unwrap();
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &crate::write::WriteOpts::default(),
        )
        .unwrap();

        let lines = apply_map_cards(&vault, &ctx, &mut journal);
        assert!(lines.iter().any(|l| l.contains("mapped UACS100Fall2026")), "{lines:?}");
        let cfg = crate::pystr::read_text(&vault.join("config").join("ingest.yaml")).unwrap();
        assert!(cfg.contains("\"UACS100Fall2026\":"), "{cfg}");
        assert!(cfg.contains("course: \"cs-100\""), "{cfg}");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: the insertion goes directly beneath an existing `courses:` line, at the
    /// indentation the file already uses, and the entry that was there is left exactly as it was.
    #[test]
    fn a_mapping_is_inserted_under_an_existing_block() {
        let vault = scratch_vault("map-insert");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     'UAMATH120Fall2026':\n        course: 'math-120'\n        label: 'MATH 120'\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));

        assert_eq!(
            crate::pystr::read_text(&path).unwrap(),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     \"UACS100Fall2026\":\n        course: \"cs-100\"\n        label: \"CS 100\"\n\
             \u{20}     'UAMATH120Fall2026':\n        course: 'math-120'\n        label: 'MATH 120'\n"
        );
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: `courses: {}` — what C1's `scaffold::ingest_yaml` writes at birth — becomes the
    /// block form, one line replaced in place at its own indentation. The VHL half is `sections`.
    #[test]
    fn an_empty_flow_mapping_becomes_the_block_form() {
        let vault = scratch_vault("map-flow");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n\
             \u{20} vhl:\n    enabled: true\n    sections: {}\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        assert_eq!(write_mapping(&vault, "vhl", "2102121", "gn-103"), Ok(true));

        assert_eq!(
            crate::pystr::read_text(&path).unwrap(),
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses:\n\
             \u{20}     \"UACS100Fall2026\":\n        course: \"cs-100\"\n        label: \"CS 100\"\n\
             \u{20} vhl:\n    enabled: true\n    sections:\n\
             \u{20}     \"2102121\":\n        course: \"gn-103\"\n        label: \"GN 103\"\n"
        );
        // The section id survives as a STRING key, which is the whole reason it is quoted:
        // `route_section` looks it up by the text the dashboard gave it.
        let cfg: Yaml = serde_yaml_ng::from_str(&crate::pystr::read_text(&path).unwrap()).unwrap();
        assert!(cfg["coursework"]["vhl"]["sections"]["2102121"]["course"].as_str() == Some("gn-103"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8: a second apply of the same key is a no-op with a log line — not a duplicate key,
    /// which would be a YAML load error and would take the whole config down with it.
    #[test]
    fn a_second_apply_of_the_same_key_is_a_no_op() {
        let vault = scratch_vault("map-again");
        let path = vault.join("config").join("ingest.yaml");
        crate::pystr::write_text(
            &path,
            "timezone: America/Chicago\ncoursework:\n  zybooks:\n    enabled: true\n    courses: {}\n",
        )
        .unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        let once = std::fs::read(&path).unwrap();
        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(false));
        assert_eq!(std::fs::read(&path).unwrap(), once, "the file is not even opened for writing");

        // And the card that asks for it a second time is still executed and archived, with the
        // line that says why nothing changed.
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let stem = write_map_card(
            &vault,
            &MapProposal {
                source: "zybooks".into(), key: "UACS100Fall2026".into(),
                label: "UACS100Fall2026".into(), suggested_course: Some("cs-100".into()),
            },
            jiff::civil::date(2026, 9, 9), &ctx, &mut journal,
        )
        .unwrap();
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &crate::write::WriteOpts::default(),
        )
        .unwrap();
        let lines = apply_map_cards(&vault, &ctx, &mut journal);
        assert!(lines.iter().any(|l| l.contains("was already mapped")), "{lines:?}");
        assert_eq!(std::fs::read(&path).unwrap(), once, "still byte-identical");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists());
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// R-C2-8, the invariant itself: **every byte outside the inserted lines is unchanged** —
    /// comments, blank lines, key order, the single-quoted style `scaffold::ingest_yaml` uses, and
    /// the file's own line terminator. This is the test a `serde_yaml_ng::to_string` round trip
    /// cannot pass, and it is why the ruling exists.
    #[test]
    fn every_byte_outside_the_inserted_lines_is_unchanged() {
        let vault = scratch_vault("map-bytes");
        let path = vault.join("config").join("ingest.yaml");
        let original = [
            "ics_url: 'https://lms.example.invalid/feed/a.ics'",
            "timezone: America/Chicago",
            "course_map: {}",
            "calendars: []",
            "",
            "# Passwords are NOT here. They live in Windows Credential Manager under the",
            "# credential_target names below.",
            "coursework:",
            "  zybooks:",
            "    enabled: true",
            "    credential_target: 'knowlu/p1/zybooks'",
            "    ignore:",
            "      - 'HowToUseZyBooks2'",
            "    courses:",
            "      'UAMATH120Fall2026':",
            "        course: 'math-120'",
            "        label: 'MATH 120'   # hand-edited, and it stays hand-edited",
            "  vhl:",
            "    enabled: true",
            "    sections: {}",
            "",
        ]
        .join("\n");
        // Written through `write_text`, so the fixture is in THIS platform's own terminator and the
        // comparison below is a real byte comparison rather than a line-ending artefact.
        crate::pystr::write_text(&path, &original).unwrap();
        let before = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();

        assert_eq!(write_mapping(&vault, "zybooks", "UACS100Fall2026", "cs-100"), Ok(true));
        let after = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();

        let nl = crate::pystr::NEWLINE;
        let inserted = format!(
            "      \"UACS100Fall2026\":{nl}        course: \"cs-100\"{nl}        label: \"CS 100\"{nl}"
        );
        assert!(after.contains(&format!("    courses:{nl}{inserted}      'UAMATH120Fall2026':")));
        assert_eq!(after.replacen(&inserted, "", 1), before, "three lines added and nothing else");
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// A card with no course on it is refused rather than writing an empty mapping — an empty
    /// `course` would make every item from that book `course: ""`, which reads as attributed.
    #[test]
    fn a_map_card_with_no_course_is_refused_and_kept() {
        let vault = scratch_vault("map-blank");
        std::fs::write(
            vault.join("config").join("ingest.yaml"),
            "timezone: America/Chicago\ncoursework:\n  vhl:\n    enabled: true\n    sections: {}\n",
        )
        .unwrap();
        let ctx = WriteContext::new(MAP_ACTOR, "local-runner");
        let mut journal = Journal::new(&vault);
        let stem = write_map_card(
            &vault,
            &MapProposal { source: "vhl".into(), key: "2102121".into(), label: "VHL section 2102121".into(), suggested_course: None },
            jiff::civil::date(2026, 9, 9), &ctx, &mut journal,
        )
        .unwrap();
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &crate::write::WriteOpts::default(),
        )
        .unwrap();
        let lines = apply_map_cards(&vault, &ctx, &mut journal);
        assert!(lines.iter().any(|l| l.contains("no course on the card")), "{lines:?}");
        let card = std::fs::read_to_string(vault.join("approvals").join(format!("{stem}.md")))
            .expect("the card survives to be edited");
        assert!(card.contains("status: pending"), "back to pending, so it ages and expires: {card}");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 6: Run it and watch it fail.** `cargo test -p knowlu-engine coursework::` → `cannot find function write_map_card`.

- [ ] **Step 7: Write the engine half** in `engine/src/coursework.rs`.

```rust
/// The agent actor for coursework-mapping cards. `agent:` prefix, so `provenance::is_agent` holds.
pub const MAP_ACTOR: &str = "agent:knowlu.coursework";

/// One thing the account's mapping does not know about (R-OB-1), as the service reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapProposal {
    pub source: String,
    pub key: String,
    pub label: String,
    pub suggested_course: Option<String>,
}

/// The card the deck renders. `kind: coursework-map` is a kind `approvals.rs` leaves alone
/// (hand-off H12): the decision is *applied* by the next `coursework` step, which then archives it.
///
/// `Err` when a card for this key already exists — the next slot fetches the same unmapped book and
/// must not mint a second card for it. The stem carries the key, so the guard is the filename.
/// That guard sees only `approvals/`, which is why `asked_map_keys` (above) is the real one and
/// this is defence in depth: a **rejected** card is in `archive/` and this check cannot see it.
pub fn write_map_card(
    vault: &Path,
    proposal: &MapProposal,
    today: Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(30))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let course = proposal.suggested_course.clone().unwrap_or_default();
    let text = format!(
        "---\ntype: approval\nkind: coursework-map\ntitle: {}\nstatus: pending\n\
         proposed_at: {stamp}\nfirst_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\n\
         created_by: coursework\nsource: {}\nmap_key: {}\ncourse: {}\n---\n\n\
         {} returned work, and nothing in your setup says which course it belongs to — so none of \
         it reached your list.\n\n\
         Approve to map it. **Edit `course:` above first if the guess is wrong**, or if it is \
         blank: it is the course slug the items will be filed under, the same one your course note \
         uses.\n",
        lit(&format!("Map {} to a course", proposal.label)),
        lit(&proposal.source),
        lit(&proposal.key),
        if course.is_empty() { "\"\"".to_string() } else { lit(&course) },
        proposal.label,
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| e.to_string())?;
    // The key is in the stem, so a second card for the same book is `WriteError::Exists` and not a
    // duplicate — the same guard `find_pending_amendment` gives an amend card.
    let stem = format!("map-{}-{}", proposal.source, crate::ingest::slugify(&proposal.key));
    let rel = crate::ids::rel(vault, &approvals.join(format!("{stem}.md")));
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}

/// Every `(source, map_key)` a card has already asked about and whose card has not yet expired —
/// `approvals/` for the live ones and **`archive/` for the decided ones**, which is the whole point.
///
/// The same mechanism as Task 12's `existing_rule_ids`, and for the same reason. `write_map_card`'s
/// only other guard is `write::create`'s `Exists` on `approvals/map-<source>-<slug>.md`, and a
/// **rejected** card is not there — `process_approvals` settles it into `archive/`. Without this
/// the next `coursework` run finds the book still unmapped, gets the same proposal and mints a
/// fresh card, twice a day, forever, each one charging the 15-a-day proposal budget.
///
/// **Where the memory lives:** in the card itself, at `archive/map-<source>-<slug>.md`. The vault
/// keeps `archive/` forever and the journal records the move, so nothing new has to be remembered
/// anywhere else. **For how long:** until the `expires:` that card was minted with —
/// `write_map_card` sets it to 30 days out — after which a still-unmapped book is proposed
/// again. That is the loop H12 describes, and this is what makes it true: a student who does not
/// recognise a book is asked again next month, not twice a day forever.
fn asked_map_keys(vault: &Path, today: Date) -> std::collections::BTreeSet<(String, String)> {
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let mut out = std::collections::BTreeSet::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
            if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("coursework-map") {
                continue;
            }
            // ISO dates compare correctly as text, which is why the vault writes them that way.
            let expires = crate::yaml::opt_text(crate::yaml::get(&meta, "expires")).unwrap_or_default();
            if expires < stamp {
                continue;
            }
            let source = crate::yaml::opt_text(crate::yaml::get(&meta, "source")).unwrap_or_default();
            let key = crate::yaml::opt_text(crate::yaml::get(&meta, "map_key")).unwrap_or_default();
            if !key.is_empty() {
                out.insert((source, key));
            }
        }
    }
    out
}

/// Apply every approved `kind: coursework-map` card, then archive it.
///
/// **The write into `config/ingest.yaml` is a text-level insertion, never a rewrite** (ruling
/// R-C2-8). CLAUDE.md's invariant is absolute and has no config exemption — *no vault file is
/// parsed and re-dumped*, and `src/yamlemit.rs` is the crate's one YAML emitter. (`scaffold::
/// ingest_yaml` writing this file at birth is **creation**, which is a different act from
/// rewriting one that already exists.) So `write_mapping` below finds the `courses:` /
/// `sections:` line and inserts three lines beneath it, and the *card* it came from is journalled
/// through `write` like every other note, which is where the audit trail lives.
pub fn apply_map_cards(vault: &Path, ctx: &WriteContext, journal: &mut Journal) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("approvals")) {
        let Ok(text) = pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
        if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("coursework-map") {
            continue;
        }
        if crate::yaml::opt_text(crate::yaml::get(&meta, "status")).as_deref() != Some("approved") {
            continue;
        }
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let source = crate::yaml::opt_text(crate::yaml::get(&meta, "source")).unwrap_or_default();
        let key = crate::yaml::opt_text(crate::yaml::get(&meta, "map_key")).unwrap_or_default();
        let course = crate::yaml::opt_text(crate::yaml::get(&meta, "course")).unwrap_or_default();
        if course.trim().is_empty() {
            // Kept, not archived: an empty `course` would file every item from that book under
            // `course: ""`, which reads as attributed-to-nothing rather than unattributed. The
            // student edits the card and approves it again.
            //
            // Written back to **pending**, which is what `approvals.rs` does with a recoverable
            // refusal (`approvals.rs:1360-1368`) and for the same reason: a card left `approved` on
            // disk is never counted, never escalated into Must do and never expires, so it re-logs
            // this line every run forever. Pending, it ages.
            let rel = crate::ids::rel(vault, &path);
            let pending = vec![("status".to_string(), "pending".to_string())];
            let _ = crate::write::write_literals(vault, &rel, &pending, ctx, journal, &WriteOpts::default());
            lines.push(format!("coursework: {stem} not applied (no course on the card)"));
            continue;
        }
        match write_mapping(vault, &source, &key, &course) {
            Ok(inserted) => {
                let rel = crate::ids::rel(vault, &path);
                let stamped = format!("\"{}\"", jiff::Zoned::now().strftime("%Y-%m-%d %H:%M"));
                let literals = vec![
                    ("status".to_string(), "executed".to_string()),
                    ("executed_at".to_string(), stamped),
                ];
                if crate::write::write_literals(vault, &rel, &literals, ctx, journal, &WriteOpts::default()).is_ok() {
                    let _ = crate::write::delete(vault, &rel, ctx, journal);
                }
                // R-C2-8's idempotence. `Ok(false)` is "the key was already there" — a re-approved
                // card, or a hand edit that got there first — and it is a line, not an error: the
                // file already says what the card was asking for, so the card is still archived.
                lines.push(if inserted {
                    format!("coursework: mapped {key} to {course}")
                } else {
                    format!("coursework: {key} was already mapped; card archived, file untouched")
                });
            }
            // Unwritable — there is no `coursework.<source>:` block to insert under, so the
            // mapping would be dead config. Same treatment as a blank course, and the same reason.
            Err(e) => {
                let rel = crate::ids::rel(vault, &path);
                let pending = vec![("status".to_string(), "pending".to_string())];
                let _ = crate::write::write_literals(vault, &rel, &pending, ctx, journal, &WriteOpts::default());
                lines.push(format!("coursework: {stem} not applied ({e})"));
            }
        }
    }
    lines
}

/// The indentation of a line, in spaces. Vault YAML is space-indented — a tab is invalid YAML
/// there and every reader in the crate would already have refused the file.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A line that carries no structure: blank, or a comment. Its column means nothing, so neither
/// helper below may let one end a block or set an indent.
fn is_filler(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#')
}

/// The first line in `[from, end)` at exactly `depth` whose key is `name`, or `None` if the block
/// ends first. A block ends at the first structural line indented *less* than `depth`.
fn find_key(lines: &[String], from: usize, end: usize, name: &str, depth: usize) -> Option<usize> {
    for i in from..end.min(lines.len()) {
        if is_filler(&lines[i]) {
            continue;
        }
        let ind = indent_of(&lines[i]);
        if ind < depth {
            return None;
        }
        if ind == depth {
            let rest = lines[i].trim_start();
            if rest == format!("{name}:") || rest.starts_with(&format!("{name}: ")) {
                return Some(i);
            }
        }
    }
    None
}

/// Where the block headed by `header` ends: the first structural line indented no deeper than the
/// header itself, or the end of the file.
fn block_end(lines: &[String], header: usize) -> usize {
    let depth = indent_of(&lines[header]);
    (header + 1..lines.len())
        .find(|&i| !is_filler(&lines[i]) && indent_of(&lines[i]) <= depth)
        .unwrap_or(lines.len())
}

/// **The file's own indentation**: the column of the first structural child of `header`, or the
/// header's own plus 2 when it has none yet. Never a constant — a student who indents by four
/// keeps indenting by four.
fn child_indent(lines: &[String], header: usize, end: usize) -> usize {
    (header + 1..end)
        .find(|&i| !is_filler(&lines[i]))
        .map(|i| indent_of(&lines[i]))
        .unwrap_or_else(|| indent_of(&lines[header]) + 2)
}

/// Insert one mapping into `config/ingest.yaml` — `coursework.zybooks.courses.<code>` or
/// `coursework.vhl.sections.<id>`, each `{course, label}`.
///
/// **A text-level insertion, never a parse-and-re-dump** (ruling R-C2-8). CLAUDE.md's engine
/// invariant is absolute and has no config exemption: *no vault file is parsed and re-dumped*,
/// and `src/yamlemit.rs` is the crate's one YAML emitter. A `serde_yaml_ng::to_string` round trip
/// would re-render every line of a file whose whole point is that it is editable text — comments
/// gone, quoting style changed, key order at the serialiser's mercy. (`scaffold::ingest_yaml`
/// writing this file at birth is **creation**, a different act from rewriting one that exists.)
///
/// So this finds the `courses:` / `sections:` line and inserts three lines directly beneath it at
/// the file's own indentation, and **touches no other byte**. The one shape it rewrites is a
/// `courses: {}` scalar, which becomes the block form the insertion needs — one line replaced,
/// in place, with the same leading spaces it already had.
///
/// **It can only ADD a key, and that costs nothing.** A book or section that is already mapped
/// never produces a card in the first place: `route_zybook` answers `BookRouting::Mapped` and
/// `parse_vhl` finds its section, so `/ingest-coursework` reports no proposal and there is no path
/// by which an existing key needs updating. A student who wants to change one edits the file,
/// which is the point of it being text.
///
/// `Ok(true)` inserted, `Ok(false)` the key was already there — a no-op, and the file is not
/// opened for writing at all. `Err` when the source's block is missing, because a mapping under a
/// `coursework.vhl:` nobody configured is dead config; the card is kept so the student sees why.
///
/// Line endings are the crate's usual discipline: `pystr::read_text` normalises to `\n` and
/// `pystr::write_text` writes `os.linesep` back, so a CRLF vault file stays CRLF exactly as it
/// does for every note this engine writes.
fn write_mapping(vault: &Path, source: &str, key: &str, course: &str) -> Result<bool, String> {
    let (block, field) = match source {
        "zybooks" => ("zybooks", "courses"),
        "vhl" => ("vhl", "sections"),
        other => return Err(format!("unknown coursework source {other:?}")),
    };
    let path = vault.join("config").join("ingest.yaml");
    let text = pystr::read_text(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();

    let coursework = find_key(&lines, 0, lines.len(), "coursework", 0)
        .ok_or_else(|| "config/ingest.yaml has no coursework: block".to_string())?;
    let coursework_end = block_end(&lines, coursework);
    let source_depth = child_indent(&lines, coursework, coursework_end);
    let source_line = find_key(&lines, coursework + 1, coursework_end, block, source_depth)
        .ok_or_else(|| format!("config/ingest.yaml has no coursework.{block}: block"))?;
    let source_end = block_end(&lines, source_line);
    let field_depth = child_indent(&lines, source_line, source_end);

    // Both scalars go through the crate's ONE literal writer — `write::to_literal`, the same
    // function every frontmatter value goes through, which routes anything non-scalar to
    // `yamlemit`. No second emitter is introduced. Quoting the key is what makes it work: an
    // unquoted `2102121:` is an integer key and `yaml::get(sections, "2102121")` then misses.
    let lit = |s: &str| crate::write::to_literal(&Yaml::String(s.to_string()));
    // The label prefixes every title the parser produces (`CS 100 HW 01`). Derived from the slug
    // rather than asked for: one field on the card is one decision, and this one is mechanical.
    let label = course.trim().to_uppercase().replace('-', " ");

    let (insert_at, entry_indent) = match find_key(&lines, source_line + 1, source_end, field, field_depth) {
        Some(at) => {
            let value = lines[at].trim_start()[field.len() + 1..].trim().to_string();
            if value == "{}" || value == "{ }" {
                // The birth shape `courses: {}` (C1's `scaffold::ingest_yaml`). One line replaced,
                // in place, by the block header the insertion needs — same indentation, and it is
                // the only line in the file this function ever rewrites.
                lines[at] = format!("{}{field}:", " ".repeat(field_depth));
            } else if !value.is_empty() {
                return Err(format!("coursework.{block}.{field} is not a block mapping ({value})"));
            }
            let end = block_end(&lines, at);
            let deeper = child_indent(&lines, at, end);
            // Idempotence: a key already under this field is a no-op, never a second entry — two
            // `UACS100Fall2026:` keys is a duplicate-key YAML error and the file stops loading.
            for i in (at + 1)..end {
                if is_filler(&lines[i]) || indent_of(&lines[i]) != deeper {
                    continue;
                }
                let name = lines[i].trim_start();
                let name = name.split(':').next().unwrap_or("").trim_matches(['\'', '"']);
                if name == key {
                    return Ok(false);
                }
            }
            (at + 1, deeper)
        }
        // No `courses:` line under a `zybooks:` that exists: insert the field header too. Still a
        // pure insertion — two lines added, none changed.
        None => {
            lines.insert(source_line + 1, format!("{}{field}:", " ".repeat(field_depth)));
            (source_line + 2, field_depth + 2)
        }
    };

    let pad = " ".repeat(entry_indent);
    lines.splice(
        insert_at..insert_at,
        [
            format!("{pad}{}:", lit(key)),
            format!("{pad}  course: {}", lit(course.trim())),
            format!("{pad}  label: {}", lit(&label)),
        ],
    );
    pystr::write_text(&path, &lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(true)
}
```

and, in `collect_cloud`, after the reply's warnings are read:

```rust
    // R-OB-1: one card per unmapped book or section, before the items are written, so a student
    // whose only zyBook is unmapped opens the deck to a question rather than to an empty list.
    let mut journal = Journal::new(vault);
    let card_ctx = ctx_or_default.with_actor(MAP_ACTOR);
    // Every key already asked about, live or decided — computed ONCE before the loop, exactly as
    // Task 12's `existing_rule_ids` is. `write::create`'s `Exists` guard only sees `approvals/`, and
    // a rejected card is in `archive/`; without this a book the student said no to is re-proposed
    // every slot until the end of time.
    let asked = asked_map_keys(vault, today);
    for row in reply.get("proposals").and_then(serde_json::Value::as_array).into_iter().flatten() {
        let text = |k: &str| row.get(k).and_then(serde_json::Value::as_str).map(str::to_string);
        let proposal = MapProposal {
            source: text("source").unwrap_or_default(),
            key: match text("key") { Some(k) if !k.is_empty() => k, _ => continue },
            label: text("label").unwrap_or_default(),
            suggested_course: text("suggested_course").filter(|c| !c.is_empty()),
        };
        if asked.contains(&(proposal.source.clone(), proposal.key.clone())) {
            continue;
        }
        match write_map_card(vault, &proposal, today, &card_ctx, &mut journal) {
            // `Exists` is now all but unreachable (`asked` catches the same case a step earlier)
            // and stays as defence in depth; either way it is not worth a line.
            Ok(stem) => warnings.push(format!("{}: not mapped; proposed ({stem})", proposal.source)),
            Err(e) if e.contains("already exists") => {}
            Err(e) => warnings.push(format!("{}: proposal not written ({e})", proposal.source)),
        }
    }
```

`coursework::main_with_fetchers` calls `apply_map_cards` **first**, before any fetch, so a card approved in the console at 11am is a mapping the noon slot already uses.

- [ ] **Step 8: Run everything.** `cargo test --workspace` at 0 warnings; `oracle.rs` and `surface_oracle.rs` unchanged (the fixture vaults carry no `coursework:` block, so nothing here runs on them).

- [ ] **Step 9: Name hand-off H12 and commit.**

```bash
git add cloud/supabase/functions/ingest-coursework/handler.ts cloud/supabase/functions/ingest-coursework/handler_test.ts engine/src/coursework.rs
git commit -F .git-commit-msg.txt   # "engine+cloud: an unmapped zyBook or VHL section is a card in the deck, never a warning nobody reads (C2 Task 7a, R-OB-1, R-C2-8)"
```

### Task 8: `/ingest-ics` and `/ingest-calendar` — the capability URLs leave the vault

The capability URL moves into the account, encrypted, and the feed is fetched server-side. **The ICS *parse* stays on the device** — see *What is NOT in this plan* for why, and for the recommendation.

**Files:**
- Create: `cloud/supabase/functions/ingest-ics/handler.ts`, `cloud/supabase/functions/ingest-ics/index.ts`
- Test: `cloud/supabase/functions/ingest-ics/handler_test.ts`, and one new test in `engine/tests/cloud_contract.rs`
- Modify: `engine/src/cloudmodel.rs` (`fetch_ics`)
- **Hand-off this task needs:** H3 (`engine/src/ingest.rs`)

**Interfaces:**
- Consumes: C1's `public.sources` (`account_id`, `kind = 'lms_ics'`, `url_ciphertext`, `url_iv`, `added_at`) and C1's `_shared/crypto.ts` (`importAesKey`, `decryptString`), with the key in the function secret `SOURCES_ENC_KEY` (**precondition P0a**, C1's to set).
- Produces: `GET /ingest-ics` → `{ ics: string, courses: number }`, or a 404 `{ error: "no lms_ics source for this account" }`, or a 502 whose body names no URL; `cloudmodel::fetch_ics(client: &CloudClient) -> Result<String, CloudError>`.

- [ ] **Step 1: Write the failing server test** — `cloud/supabase/functions/ingest-ics/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { icsHandler } from "./handler.ts";

const FEED = await Deno.readTextFile(new URL("../../../../engine/tests/fixtures/blackboard.ics", import.meta.url));
const OK = () => Promise.resolve({ account_id: "acct-1" });

Deno.test("the account's stored URL is fetched server-side and the URL never comes back", async () => {
  let asked = "";
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: (url: string) => {
      asked = url;
      return Promise.resolve(FEED);
    },
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assert(reply.ics.includes("BEGIN:VCALENDAR"));
  assert(typeof reply.courses === "number");
  assertEquals(asked, "https://lms.example.invalid/feed/secret-capability.ics");
  // The capability URL is a credential in all but name: it must never come back to the device,
  // into a log, or into an error body (cloud design §3.1, §9 Alabama SPII).
  assertEquals(JSON.stringify(reply).includes("secret-capability"), false);
});

Deno.test("an account with no stored feed is a 404 that says so", async () => {
  const handler = icsHandler(OK, { urlFor: () => Promise.resolve(null), fetchText: () => Promise.reject(new Error("must not fetch")) });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 404);
  assertEquals((await response.json()).error, "no lms_ics source for this account");
});

Deno.test("a feed that will not fetch is a 502 whose body carries no URL", async () => {
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: () => Promise.reject(new Error("getaddrinfo ENOTFOUND lms.example.invalid/feed/secret-capability.ics")),
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 502);
  assertEquals((await response.text()).includes("secret-capability"), false);
});
```

- [ ] **Step 2: Run it and watch it fail.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/ingest-ics/` → module not found.

- [ ] **Step 3: Write `handler.ts`.**

```ts
// GET /ingest-ics — the account's LMS calendar feed, fetched with the service role.
//
// The capability URL is a credential in all but name: anyone holding it reads the student's whole
// calendar. It lives in C1's `sources` table, encrypted at rest, is read here with the service
// role, and never appears in a reply, an error body or a log line (§3.1, §9). The device gets the
// feed text and parses it with the same `ingest::parse_ics` the golden `today.md` oracle covers.
import type { Entitle } from "../_shared/judge_handler.ts";

export interface IcsDeps {
  urlFor(accountId: string): Promise<string | null>;
  fetchText(url: string): Promise<string>;
}

/// Some LMS hosts 403 a request with no browser-shaped User-Agent, exactly as the event feeds do.
export const ICS_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

export function icsHandler(entitle: Entitle, deps: IcsDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return Response.json({ error: "GET only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const url = await deps.urlFor(account_id);
      if (url === null) return Response.json({ error: "no lms_ics source for this account" }, { status: 404 });
      let ics: string;
      try {
        ics = await deps.fetchText(url);
      } catch {
        // Deliberately swallows the cause: a transport error's text is the one place the URL comes
        // back out, and this body reaches the device and its log.
        return Response.json({ error: "the calendar feed could not be fetched" }, { status: 502 });
      }
      if (!ics.includes("BEGIN:VCALENDAR")) {
        return Response.json({ error: "the calendar feed is not an ICS response" }, { status: 502 });
      }
      // A count the wizard can show ("we found 14 events"), and nothing that identifies the feed.
      const courses = (ics.match(/^BEGIN:VEVENT/gm) ?? []).length;
      return Response.json({ ics, courses });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-ics: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "ingest failed" }, { status: 500 });
    }
  };
}
```

and `index.ts` — **the only place in C2 that decrypts anything, and it uses C1's crypto rather than its own** (rulings R-C2-5 / R-X-1). C1 stores `url_ciphertext` + `url_iv` (AES-GCM, 12-byte IV, base64) and the 32-byte base64 key is the function secret `SOURCES_ENC_KEY` (precondition P0a):

```ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { decryptString, importAesKey } from "../_shared/crypto.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { icsHandler, ICS_USER_AGENT } from "./handler.ts";

// Imported once, lazily, and kept: `importAesKey` is a `crypto.subtle` call and re-importing it
// per request is work for nothing. Built at first use, never at module scope, so a missing secret
// is the handler's named 502 and not an opaque boot failure.
let key: CryptoKey | null = null;
async function sourcesKey(): Promise<CryptoKey> {
  if (key === null) {
    const raw = Deno.env.get("SOURCES_ENC_KEY");
    if (raw === undefined || raw === "") throw new Error("the function is missing SOURCES_ENC_KEY");
    key = await importAesKey(raw);
  }
  return key;
}

Deno.serve(icsHandler(requireActiveEntitlement, {
  async urlFor(accountId: string): Promise<string | null> {
    // `account_id=eq.` is the whole access control — the service role bypasses RLS, and
    // `judge_db_test.ts` scans for a select on a scoped table that omits it.
    const rows = await sharedDb().select(
      `sources?account_id=eq.${accountId}&kind=eq.lms_ics&select=url_ciphertext,url_iv`,
    ) as Array<{ url_ciphertext: string; url_iv: string }>;
    if (rows.length === 0) return null;
    return await decryptString(await sourcesKey(), rows[0].url_ciphertext, rows[0].url_iv);
  },
  async fetchText(url: string): Promise<string> {
    const response = await fetch(url, { headers: { "User-Agent": ICS_USER_AGENT } });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.text();
  },
}));
```

The plaintext URL exists in this function's memory for the duration of one fetch and reaches nothing else: not the reply (`handler.ts` returns only the feed and a count), not an error body (the 502 branch swallows the cause deliberately), not a log line.

- [ ] **Step 4: Add `fetch_ics` to `engine/src/cloudmodel.rs`.**

```rust
/// The account's LMS calendar feed, fetched by the service (cloud design §3.1). **Transport, not
/// judgment** — `ingest` parses what comes back with the same `parse_ics` the golden `today.md`
/// oracle covers, so the vault's bytes are unchanged by the move.
pub fn fetch_ics(client: &CloudClient) -> Result<String, CloudError> {
    let reply = client.get("/ingest-ics")?;
    reply
        .get("ics")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CloudError::Body("the reply carried no ics field".to_string()))
}
```

- [ ] **Step 5: Add the contract test** to `engine/tests/cloud_contract.rs`:

```rust
#[test]
fn the_ics_fetch_is_a_get_that_sends_no_url_of_its_own() {
    let feed = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(&serde_json::json!({ "ics": feed, "courses": 0 }));
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let got = knowlu_engine::cloudmodel::fetch_ics(&client).expect("the service answered");
    assert!(got.contains("BEGIN:VCALENDAR"));
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-ics HTTP/1.1"));
    // The device does not know the URL any more and must not be able to name one.
    assert!(!sent.contains("ics_url"), "{sent}");
}
```

- [ ] **Step 6: Run it all.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/` then `cargo test --workspace`
Expected: both green; `ingest`'s existing tests unchanged (the fallback path is what they exercise, and H3 leaves it in place).

- [ ] **Step 7: Deploy and smoke.** `supabase functions deploy ingest-ics --project-ref <staging ref>`, then insert an `lms_ics` row for a staging test account against a feed you control (never a real LMS capability URL from a live account) and `curl` the endpoint with that account's JWT. Expected: 200, a body starting `BEGIN:VCALENDAR`, and a `courses` count.

- [ ] **Step 8: Add the engine test that H3 exists for** — in `engine/tests/cloud_contract.rs` (C2-owned; **`engine/src/ingest.rs` is the controller's and no task here edits it, not even its test module**). `ingest::run_lines` is `pub`, so an integration test reaches it. **This test is red until H3 is applied, and that is the point**: it is the executable form of the sentence *"a cloud vault with a blank `ics_url` must never exit 1 before the service is asked"*.

```rust
/// A cloud vault whose `ics_url` is blank must reach `/ingest-ics` before it gives up.
///
/// Without hand-off H3 this exits 1 six lines into `run_lines`, which sets
/// `RunSummary.engine_ok = false` and paints the tray amber twice a day forever — the exact
/// failure `judge`'s always-exit-0 rule exists to prevent, on the step beside it.
#[test]
fn a_cloud_vault_reaches_the_service_before_it_refuses_a_blank_ics_url() {
    let feed = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(&serde_json::json!({ "ics": feed, "courses": 0 }));
    let mut server = loopback(vec![(200, body)]);
    let dir = std::env::temp_dir().join(format!("knowlu-c2-ics-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
    std::fs::create_dir_all(dir.join("tasks")).expect("scratch vault");
    std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\nics_url: \"\"\n")
        .expect("write ingest.yaml");
    std::fs::write(
        dir.join("config").join("cloud.yaml"),
        format!(
            "api_base: '{}'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/c2-test/session'\naccount_id: 'acct-1'\n",
            server.base
        ),
    )
    .expect("write cloud.yaml");

    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, None);
    // With H3 applied and a session present this is 0 and the feed came from the service. On this
    // machine there is no credential at that target, so `resolve` fails and the run falls through
    // to the blank-url refusal — which is still exit 1, but the LINE must name the whole reason,
    // not merely "no ics_url configured", or the next reader will not know the service was asked.
    assert_eq!(code, 1);
    assert!(
        lines[0].contains("no ics_url configured"),
        "the refusal must still say what is missing: {lines:?}"
    );
    let _ = server.requests();
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 9: Name hand-off H3 in the task report**, with the note that step 8's test is red until it lands, and hand-off **H5** (`app/src/scheduler.rs`'s `|| cloud`), which is applied at this task's merge and **after C1**. Do not edit either file.

- [ ] **Step 10: `/ingest-calendar` — the same shape, for the calendars** (cloud design §11a). Create `cloud/supabase/functions/ingest-calendar/{handler.ts,handler_test.ts,index.ts}`.

It serves **both** calendar sources from one endpoint, and returns **an ICS document** in both cases — which is the whole reason this is small: `calfeed::parse_calendar_ics` then applies its 28-day horizon, its recurrence expansion, its dedup and its snapshot fallback, exactly as it does for a feed the device fetched itself. No second parser, no new vault file, nothing for the golden-`today.md` oracle to notice.

```ts
// GET /ingest-calendar?name=<feed name> — the account's calendar, as ICS.
//
// Two sources, one shape:
//   * `google_calendar` — the Google grant's own events for the engine's window, rendered as ICS.
//     Read-only, `calendar.readonly` only, and the grant is the same one Gmail uses when the
//     student has taken that step too (§11a).
//   * `calendar_ics` — the secret iCal address the wizard captured, fetched server-side. This is
//     the path C1 ships and the fallback while the OAuth scope is unverified; serving it here as
//     well means the address stops needing to be in the vault once the account has it.
//
// The secret address is a capability URL, exactly like the LMS one: it never appears in a reply,
// an error body or a log line.
import { decryptString, importAesKey } from "../_shared/crypto.ts";
import type { Entitle } from "../_shared/judge_handler.ts";

/** The engine's own window (`calfeed::HORIZON_DAYS`). Asking for more would be discarded. */
export const HORIZON_DAYS = 28;
export const CAL_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

/**
 * The `?name=` -> source mapping, and the ONLY place it is decided (ruling R-X-9).
 *
 *   `personal` (and the default) -> the account's `sources` row where `kind = 'calendar_ics'`
 *   `google`                     -> the account's `google_accounts` row, if its `scopes` carry
 *                                   `calendar.readonly`
 *   anything else                -> a 404 that names the name
 *
 * **`sources` has no `name` column** — its primary key is `(account_id, kind)` — and **the Google
 * calendar is not a `sources` row at all**: a grant has no URL, and both `url_ciphertext` and
 * `url_iv` are `not null`. `google_calendar` exists in C1's check constraint as a reserved value
 * so neither stream ever has to edit the other's constraint, and C2 never writes a row of that
 * kind. The two names are the two `calendars:` entries the vault carries, which is what makes the
 * name in the query string and the name in the vault the same word.
 */
export const CALENDAR_NAMES = { personal: "calendar_ics", google: "google_accounts" } as const;

export interface CalendarDeps {
  /** The account's `sources` row of kind `calendar_ics`, or null. */
  personalSource(accountId: string): Promise<{ ciphertext: string; iv: string } | null>;
  /** A fresh Google access token, or null when the grant is gone or lacks the calendar scope. */
  calendarTokenFor(accountId: string): Promise<string | null>;
  /** Google Calendar's events for the window, already normalised. */
  googleEvents(accessToken: string, from: Date, to: Date): Promise<Array<{
    uid: string; summary: string; start: string; end: string; allDay: boolean;
  }>>;
  fetchText(url: string): Promise<string>;
  encKey(): Promise<CryptoKey>;
  now(): Date;
}

function stamp(value: string, allDay: boolean): string {
  // `YYYYMMDD` for an all-day event, `YYYYMMDDTHHMMSSZ` otherwise — the two forms
  // `calfeed::parse_dt` reads. Google returns RFC 3339, so this is a reshape and not a parse.
  const d = new Date(value);
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}`;
  if (allDay) return day;
  return `${day}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
}

function escapeIcs(text: string): string {
  return text.replace(/\\/g, "\\\\").replace(/;/g, "\\;").replace(/,/g, "\\,").replace(/\r?\n/g, "\\n");
}

export function toIcs(events: Array<{ uid: string; summary: string; start: string; end: string; allDay: boolean }>): string {
  const lines = ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Knowlu//ingest-calendar//EN"];
  for (const e of events) {
    lines.push("BEGIN:VEVENT");
    lines.push(`UID:${escapeIcs(e.uid)}`);
    lines.push(`SUMMARY:${escapeIcs(e.summary)}`);
    lines.push(e.allDay ? `DTSTART;VALUE=DATE:${stamp(e.start, true)}` : `DTSTART:${stamp(e.start, false)}`);
    lines.push(e.allDay ? `DTEND;VALUE=DATE:${stamp(e.end, true)}` : `DTEND:${stamp(e.end, false)}`);
    lines.push("END:VEVENT");
  }
  lines.push("END:VCALENDAR");
  // CRLF, because that is what RFC 5545 says and what every other feed the engine reads uses.
  return lines.join("\r\n") + "\r\n";
}

export function calendarHandler(entitle: Entitle, deps: CalendarDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return Response.json({ error: "GET only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const name = new URL(req.url).searchParams.get("name") ?? "personal";
      if (!(name in CALENDAR_NAMES)) {
        return Response.json({ error: `no calendar named '${name}' for this account` }, { status: 404 });
      }
      if (name === "personal") {
        const source = await deps.personalSource(account_id);
        if (source === null) {
          return Response.json({ error: "no calendar named 'personal' for this account" }, { status: 404 });
        }
        const url = await decryptString(await deps.encKey(), source.ciphertext, source.iv);
        let ics: string;
        try {
          ics = await deps.fetchText(url);
        } catch {
          // The cause is swallowed: a transport error's text quotes the secret address.
          return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
        }
        if (!ics.includes("BEGIN:VCALENDAR")) {
          return Response.json({ error: "the calendar is not an ICS response" }, { status: 502 });
        }
        return Response.json({ ics, source: "calendar_ics" });
      }
      const token = await deps.calendarTokenFor(account_id);
      if (token === null) {
        // Not connected, revoked, or connected for Gmail only — all one answer, and the device
        // degrades to `using snapshot` rather than losing the day's capacity model.
        return Response.json({ error: "the Google calendar is not connected" }, { status: 409 });
      }
      const from = deps.now();
      const to = new Date(from.getTime() + HORIZON_DAYS * 86_400_000);
      try {
        return Response.json({ ics: toIcs(await deps.googleEvents(token, from, to)), source: "google_calendar" });
      } catch {
        return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
      }
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-calendar: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "calendar failed" }, { status: 500 });
    }
  };
}
```

`index.ts` wires it the way `ingest-ics/index.ts` does — `sharedDb()`, the lazily-imported `SOURCES_ENC_KEY`, `personalSource` as `sources?account_id=eq.<id>&kind=eq.calendar_ics&select=url_ciphertext,url_iv` (the primary key is `(account_id, kind)`, so that is at most one row and there is no `name` to match on), `read_google_grant(p_account, 'https://www.googleapis.com/auth/calendar.readonly')` for the token — which returns nothing when the grant was never made, was revoked, or carries only the Gmail scope — and `googleEvents` over `https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&timeMin=…&timeMax=…`, mapping each item's `start.dateTime ?? start.date` and the matching `end`, with `allDay = start.date !== undefined`. `singleEvents=true` is what makes Google expand the recurrences, so `calfeed`'s own RRULE handling never sees one.

`handler_test.ts` covers, with fakes throughout: the **name mapping** (`?name=personal` and no `name` at all both reach `personalSource` and never the Google token; `?name=google` reaches the token and never `personalSource`; `?name=work` is a 404 that names `work` and touches neither); a Google grant becomes an ICS document whose `DTSTART`/`DTEND` round-trip through `calfeed`'s two accepted forms; an all-day event uses `VALUE=DATE`; the personal source is fetched server-side and **its address appears in neither the reply nor the 502 body**; a Google grant without `calendar.readonly` is a **409, not a 404** — the account exists, the scope does not — and no fetch is attempted; and a summary containing a comma, a semicolon and a newline is escaped so the feed still parses. One more, static, because it is the mismatch this endpoint was rewritten to fix:

```ts
Deno.test("the google calendar is never looked for in the sources table", async () => {
  // Ruling R-X-9: `google_calendar` is a reserved value in C1's check constraint that nobody
  // writes. A grant has no URL, and `sources.url_ciphertext` / `url_iv` are both `not null`, so a
  // row of that kind could not be filled — and `sources` has no `name` column to look one up by.
  const source = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assertEquals(source.includes("google_calendar"), false, "the grant lives in google_accounts");
  assert(source.includes("kind=eq.calendar_ics"), "the personal calendar is the only sources row read here");
});
```

- [ ] **Step 11: Add `fetch_calendar` to `engine/src/cloudmodel.rs`, and its contract test.**

```rust
/// One calendar feed, as ICS, from the service (cloud design §11a). **Transport, not judgment** —
/// `calfeed` parses what comes back with the same `parse_calendar_ics` the golden `today.md`
/// oracle covers, bounds it to the same 28-day horizon and falls back to the same snapshot.
///
/// `Err(String)` because the caller is `Fetchers.calendar`, whose contract predates this module
/// and whose failure already degrades to "using snapshot".
pub fn fetch_calendar(client: &CloudClient, name: &str) -> Result<String, String> {
    let reply = client
        .get(&format!("/ingest-calendar?name={}", urlencode_component(name)))
        .map_err(|e| e.to_string())?;
    reply
        .get("ics")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "the reply carried no ics field".to_string())
}

/// The three characters a feed name could carry that a query string would misread. Not a general
/// percent-encoder: a feed name comes from `config/ingest.yaml`, and anything wilder than this
/// should fail loudly at the server rather than be smuggled through.
fn urlencode_component(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            other => other.encode_utf8(&mut [0u8; 4]).bytes().map(|b| format!("%{b:02X}")).collect(),
        })
        .collect()
}
```

```rust
#[test]
fn the_calendar_fetch_is_a_get_that_names_the_feed_and_no_address() {
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:g1\r\nSUMMARY:Seminar\r\n\
               DTSTART:20260909T140000Z\r\nDTEND:20260909T150000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(
        &serde_json::json!({ "ics": ics, "source": "google_calendar" }),
    );
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let got = knowlu_engine::cloudmodel::fetch_calendar(&client, "google").expect("the service answered");
    assert!(got.contains("BEGIN:VEVENT"));
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-calendar?name=google HTTP/1.1"), "{sent}");
    // The device does not know the secret address any more and must not be able to name one.
    assert!(!sent.contains("ics_url") && !sent.contains("calendar_ics"), "{sent}");
}
```

- [ ] **Step 12: Run it all.**

Run: `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/` then `cargo test --workspace`
Expected: both green; `calfeed`'s existing tests unchanged (nothing in `calfeed.rs` is touched — the `cloud:` branch lives in `cli::run`, hand-off H4), and `oracle.rs` / `surface_oracle.rs` unchanged.

- [ ] **Step 13: Deploy and smoke both.**

```
supabase functions deploy ingest-ics --project-ref <staging ref>
supabase functions deploy ingest-calendar --project-ref <staging ref>
```

For `/ingest-ics`: insert an `lms_ics` row for a staging test account against a feed you control (never a real LMS capability URL from a live account) and `curl` with that account's JWT — expect 200, a body starting `BEGIN:VCALENDAR`, and a `courses` count. For `/ingest-calendar`: the same with a `calendar_ics` row and `?name=personal` (and with no `name` at all, which defaults to it); `?name=google` on an account with no grant, expecting a **409** and not a 404; `?name=work`, expecting a **404** naming the name; then, after Task 10, `?name=google` on a connected account. **Never insert a `google_calendar` row** — nothing writes one and nothing reads one. **Record whether the returned ICS round-trips through the engine**: copy it into a scratch vault's feed and run `rank` against a temp copy — never a real vault (ruling R-3a-12).

- [ ] **Step 14: Name the hand-offs and commit.** H3 (`ingest.rs`), H4 (`cli.rs` — now the calendar proxy as well as the events one), and H5 (`scheduler.rs`'s `|| cloud`, at this task's merge, after C1).

```bash
git add cloud/supabase/functions/ingest-ics/ cloud/supabase/functions/ingest-calendar/ engine/src/cloudmodel.rs engine/tests/cloud_contract.rs
git commit -F .git-commit-msg.txt   # "cloud: GET /ingest-ics and /ingest-calendar — the capability URLs live in the account and are fetched server-side (C2 Task 8)"
```

Nothing under `engine/src/ingest.rs`, `engine/src/cli.rs` or `engine/src/calfeed.rs` is staged: those files' changes are hand-offs H3 and H4, applied by the controller.

---

---

### Task 8a: R-OB-3 — a first ingest never creates a task that is already past due

**Why this task exists.** Quinn's first slot imported four items already past due, one of them from 2025 — a feed's window reaches backwards, and a vault born today has no history to reconcile them against. The first page a student ever sees is the retention moment (VISION success criterion 5), and four overdue items they can do nothing about is the worst possible version of it.

**The rule.** On a **first** ingest — the vault has no `today.md`, so `rank` has never finished a slot on it (ruling **R-C2-9**; **not** the absence of `state/ingest-seen.md`, which `coursework` creates earlier in the very same slot) — an item whose due date is already past is **recorded as seen and written straight into `archive/`** with `status: archived` and `archived_reason: imported-past`. Not skipped: skipping would leave the uid unseen, and the next run would create it. Not created-then-deleted: that is two journal records and a note that briefly ranks. **One `create` into `archive/`, one `record_seen`, one log line.** Every later run is unchanged — an item that goes past due while the vault is watching it is exactly the item the system exists to shout about.

**Files:**
- Modify: `cloud/supabase/functions/ingest-ics/{handler.ts,index.ts,handler_test.ts}`, `engine/src/cloudmodel.rs` (`fetch_ics` returns the past-due list)
- Test: `engine/tests/cloud_contract.rs`
- **Hand-off this task needs:** H11 (`engine/src/ingest.rs` — the device half, which is the guarantee)

**Interfaces:**
- Produces: `GET /ingest-ics?first_run=1` → `{ ics, courses, past_due_uids: string[] }`; `cloudmodel::fetch_ics(client, first_run: bool) -> Result<(String, Vec<String>), CloudError>`; and, in H11, `ingest::is_first_run(&Path) -> bool`, `ingest::sync_tasks(…, first_run: bool)` and `ingest::IMPORTED_PAST`.

**Which half is the guarantee.** The **device's**, and only the device's: it is the half that works with no account at all, it is the half that can write `archive/` and `ingest-seen`, and it is the half a pre-C1 install runs. The server's `past_due_uids` is corroboration — it is what the wizard counts to say *"14 upcoming items, 4 already past"* on the finish panel, and it costs one UID-and-DTEND scan of a feed the server has just fetched anyway. **When they disagree, the device wins**, because the device is the one that knows the vault's own timezone.

- [ ] **Step 1: Write the failing device test** — in `engine/tests/cloud_contract.rs` (C2-owned; `engine/src/ingest.rs` is the controller's and no task here edits it). **This test is red until hand-off H11 is applied, and that is the point.**

```rust
/// R-OB-3: the first page a student ever sees must show the future.
///
/// A feed's window reaches backwards, and a vault born today has no history to reconcile against —
/// so Quinn's first slot imported four overdue items, one from 2025. On a FIRST ingest (no
/// `today.md`, R-C2-9) a past-due item is recorded as seen and written straight into
/// `archive/`; on every later run it is created normally, because an item that goes past due while
/// the vault is watching it is exactly the item the system exists to shout about.
#[test]
fn ingest_on_a_fresh_vault_archives_past_due_items() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-firstrun-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "tasks", "state", "archive"] {
        std::fs::create_dir_all(dir.join(sub)).expect("scratch vault");
    }
    std::fs::write(
        dir.join("config").join("ingest.yaml"),
        "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\ncourse_map:\n  CS-100: cs-100\n",
    )
    .expect("write ingest.yaml");
    // One item long past, one comfortably ahead. `parse_ics` reads `DTSTART` as the due date.
    let feed = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\nUID:bb-old\r\nSUMMARY:CS-100 Homework 1\r\nDTSTART:20250902T045900Z\r\nEND:VEVENT\r\n\
        BEGIN:VEVENT\r\nUID:bb-new\r\nSUMMARY:CS-100 Homework 9\r\nDTSTART:20991002T045900Z\r\nEND:VEVENT\r\n\
        END:VCALENDAR\r\n";
    let fetch = |_: &str| Ok(feed.to_string());

    // First run is the absence of `today.md`, NOT of the seen-ledger (R-C2-9) — and this test
    // proves the distinction by writing the ledger first, exactly as `coursework` does at
    // `coursework.rs:310` earlier in the same slot.
    std::fs::write(
        dir.join("state").join("ingest-seen.md"),
        "# header\n- zybooks:UACS100Fall2026:1.2 · Section 1.2 · first seen 2026-09-09\n",
    )
    .expect("coursework got here first");
    assert!(!dir.join("state").join("today.md").exists());
    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, Some(&fetch));
    assert_eq!(code, 0, "{lines:?}");
    assert!(
        lines.iter().any(|l| l.contains("archived (imported-past)") && l.contains("bb-old") == false),
        "the past-due item is archived and named by its stem: {lines:?}"
    );

    // The future item is a task; the past one is in `archive/` and in nothing else.
    let tasks: Vec<_> = knowlu_engine::approvals::sorted_md(&dir.join("tasks"));
    assert_eq!(tasks.len(), 1, "only the future item is a task");
    assert!(std::fs::read_to_string(&tasks[0]).unwrap().contains("Homework 9"));
    let archived: Vec<_> = knowlu_engine::approvals::sorted_md(&dir.join("archive"));
    assert_eq!(archived.len(), 1);
    let note = std::fs::read_to_string(&archived[0]).unwrap();
    assert!(note.contains("Homework 1"));
    assert!(note.contains("status: archived"));
    assert!(note.contains("archived_reason: imported-past"));
    // Recorded as seen, so a second run does not resurrect it.
    let seen = std::fs::read_to_string(dir.join("state").join("ingest-seen.md")).unwrap();
    assert!(seen.contains("bb-old") && seen.contains("bb-new"));

    // `rank` has since finished the slot and written `today.md`, which is what makes the next
    // ingest not-first. A LATER run is unchanged: a new past-due item is created as a task, loudly.
    std::fs::write(dir.join("state").join("today.md"), "# Today\n").expect("rank wrote today.md");
    let later = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\nUID:bb-late\r\nSUMMARY:CS-100 Homework 2\r\nDTSTART:20250903T045900Z\r\nEND:VEVENT\r\n\
        END:VCALENDAR\r\n";
    let fetch_later = |_: &str| Ok(later.to_string());
    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, Some(&fetch_later));
    assert_eq!(code, 0, "{lines:?}");
    assert_eq!(knowlu_engine::approvals::sorted_md(&dir.join("tasks")).len(), 2, "{lines:?}");
    assert_eq!(knowlu_engine::approvals::sorted_md(&dir.join("archive")).len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// R-C2-9: the predicate is the absence of `today.md`, not of `state/ingest-seen.md`.
///
/// The slot order is `coursework → ingest → judge → rank`, and `coursework` calls
/// `ingest::record_seen` (`coursework.rs:19`, `:310`, `:415`) — so on the very first slot the
/// seen-ledger already exists by the time `ingest` runs, and a `first_run` derived from it is
/// `false` on exactly the run R-OB-3 was written for. `today.md` is `rank`'s, the last step, so the
/// first `ingest` always sees it absent and every later one sees it present. It is also the
/// predicate the app already uses (`app/src/scheduler.rs::needs_first_run`).
#[test]
fn first_run_is_the_absence_of_today_md_not_of_ingest_seen() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-firstpred-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("state")).expect("scratch vault");

    // What `coursework` leaves behind on the first slot, before `ingest` has run at all.
    std::fs::write(dir.join("state").join("ingest-seen.md"), "# header\n").unwrap();
    assert!(
        knowlu_engine::ingest::is_first_run(&dir),
        "the seen-ledger is shared with coursework and cannot be the predicate"
    );

    // What `rank` leaves behind at the end of that same slot.
    std::fs::write(dir.join("state").join("today.md"), "# Today\n").unwrap();
    assert!(!knowlu_engine::ingest::is_first_run(&dir));

    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run it and watch it fail.** `cargo test -p knowlu-engine --test cloud_contract ingest_on_a_fresh_vault` → the past-due item is a task and `archive/` is empty.

- [ ] **Step 3: Write hand-off H11's code** (below, in *Controller hand-offs*) and hand it over. **Do not edit `engine/src/ingest.rs`.** The test stays red on this branch and the report says so.

- [ ] **Step 4: Write the failing server test** — append to `cloud/supabase/functions/ingest-ics/handler_test.ts`:

```ts
Deno.test("first_run reports which uids are already past, and otherwise reports none", async () => {
  // R-OB-3's corroborating half. The DEVICE's check is the guarantee — it is the half that works
  // with no account and the half that knows the vault's timezone — and this is what the wizard
  // counts to say "14 upcoming, 4 already past" on the finish panel.
  const feed = "BEGIN:VCALENDAR\r\n" +
    "BEGIN:VEVENT\r\nUID:bb-old\r\nSUMMARY:Old\r\nDTSTART:20250902T045900Z\r\nEND:VEVENT\r\n" +
    "BEGIN:VEVENT\r\nUID:bb-new\r\nSUMMARY:New\r\nDTSTART:20991002T045900Z\r\nEND:VEVENT\r\n" +
    "END:VCALENDAR\r\n";
  const deps = {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: () => Promise.resolve(feed),
  };
  const first = await (await icsHandler(OK, deps)(new Request("http://127.0.0.1/ingest-ics?first_run=1"))).json();
  assertEquals(first.past_due_uids, ["bb-old"]);
  assertEquals(first.courses, 2);
  const later = await (await icsHandler(OK, deps)(new Request("http://127.0.0.1/ingest-ics"))).json();
  assertEquals(later.past_due_uids, [], "only a first ingest has a past to skip");
});

Deno.test("an event with no date at all is never called past due", async () => {
  const feed = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:bb-none\r\nSUMMARY:No date\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
  const reply = await (await icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/f.ics"),
    fetchText: () => Promise.resolve(feed),
  })(new Request("http://127.0.0.1/ingest-ics?first_run=1"))).json();
  assertEquals(reply.past_due_uids, []);
});
```

- [ ] **Step 5: Write the server half** in `cloud/supabase/functions/ingest-ics/handler.ts`.

```ts
/**
 * The uids whose date is already past, for a **first** ingest only (R-OB-3).
 *
 * A deliberate 15-line scan and **not** a second ICS parser: it reads `UID` and the first of
 * `DTSTART` / `DTEND` per `VEVENT` and compares dates, and it does not unfold, unescape, expand a
 * recurrence or resolve a `TZID`. It cannot: the vault's timezone is on the device and so is
 * `parse_ics`, which is where the real reading happens and where the golden `today.md` oracle
 * covers it. **The device's own check is the guarantee**; this list is corroboration, and it is
 * what the wizard counts to say "14 upcoming items, 4 already past" on the finish panel.
 */
export function pastDueUids(ics: string, now: Date): string[] {
  const out: string[] = [];
  for (const block of ics.split(/BEGIN:VEVENT/i).slice(1)) {
    const body = block.split(/END:VEVENT/i)[0];
    const uid = /^UID:(.*)$/im.exec(body)?.[1]?.trim();
    if (uid === undefined || uid === "") continue;
    // `DTSTART` is what `ingest::parse_ics` reads as the due date; `DTEND` is the fallback for a
    // feed that carries only an end. Both forms: `20250902T045900Z` and a bare `20250902`.
    const stamp = (/^DTSTART[^:]*:(\d{8})/im.exec(body) ?? /^DTEND[^:]*:(\d{8})/im.exec(body))?.[1];
    if (stamp === undefined) continue;
    const day = new Date(Date.UTC(+stamp.slice(0, 4), +stamp.slice(4, 6) - 1, +stamp.slice(6, 8)));
    // Strictly before TODAY, never before *now*: an item due at 23:59 today is today's work, and
    // the one thing worse than importing a stale task is archiving a live one.
    const today = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
    if (day.getTime() < today.getTime()) out.push(uid);
  }
  return out;
}
```

and, in the handler, after the `BEGIN:VCALENDAR` check:

```ts
      const courses = (ics.match(/^BEGIN:VEVENT/gm) ?? []).length;
      // `first_run=1` is sent by the engine when the vault has no `today.md` (R-C2-9).
      const firstRun = new URL(req.url).searchParams.get("first_run") === "1";
      return Response.json({
        ics,
        courses,
        past_due_uids: firstRun ? pastDueUids(ics, deps.now()) : [],
      });
```

with `now(): Date` added to `IcsDeps` and wired to `() => new Date()` in `index.ts`.

- [ ] **Step 6: Widen `cloudmodel::fetch_ics`** so the flag travels and the list comes back:

```rust
/// The account's LMS calendar feed, fetched by the service (cloud design §3.1). **Transport, not
/// judgment** — `ingest` parses what comes back with the same `parse_ics` the golden `today.md`
/// oracle covers, so the vault's bytes are unchanged by the move.
///
/// `first_run` is R-OB-3: on a vault with no `today.md` (R-C2-9) the service also returns the
/// uids whose date is already past, which the wizard counts. **The device's own check is the
/// guarantee** — this list is corroboration, and `sync_tasks` archives on either.
pub fn fetch_ics(client: &CloudClient, first_run: bool) -> Result<(String, Vec<String>), CloudError> {
    let path = if first_run { "/ingest-ics?first_run=1" } else { "/ingest-ics" };
    let reply = client.get(path)?;
    let ics = reply
        .get("ics")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CloudError::Body("the reply carried no ics field".to_string()))?;
    let past = reply
        .get("past_due_uids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    Ok((ics, past))
}
```

Hand-off **H3**'s two call sites become `crate::cloudmodel::fetch_ics(client, first_run)` and take the `.0`; H3's code below carries that change, and `first_run` is the same `crate::ingest::is_first_run(vault)` H11 adds.

- [ ] **Step 7: Extend the loopback contract test** in `engine/tests/cloud_contract.rs`:

```rust
#[test]
fn the_ics_fetch_sends_first_run_and_reads_the_past_due_list() {
    let feed = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(&serde_json::json!({
        "ics": feed, "courses": 0, "past_due_uids": ["bb-old"],
    }));
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let (ics, past) = knowlu_engine::cloudmodel::fetch_ics(&client, true).expect("the service answered");
    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert_eq!(past, vec!["bb-old".to_string()]);
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-ics?first_run=1 HTTP/1.1"), "{sent}");
}
```

- [ ] **Step 8: Run everything.** `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/` then `cargo test --workspace`. `ingest_on_a_fresh_vault_archives_past_due_items` stays **red until H11 is applied** — say so in the report rather than weakening it. `oracle.rs` and `surface_oracle.rs` are unchanged for a better reason than the fixtures' contents: `oracle.rs` shells out to `rank` only (`oracle.rs:84`) and `surface_oracle.rs` to `surface`, and `first_run` is read nowhere but `ingest`. **None of the three fixture vaults has a `today.md`** — `rank` writes one every run — so the predicate would call them all first runs; it is never asked.

- [ ] **Step 9: Name hand-offs H3 (widened) and H11, and commit.**

```bash
git add cloud/supabase/functions/ingest-ics/ engine/src/cloudmodel.rs engine/tests/cloud_contract.rs
git commit -F .git-commit-msg.txt   # "engine+cloud: a first ingest archives what is already past, so the first page shows the future (C2 Task 8a, R-OB-3)"
```

### Task 9: `/judge-event` and `/events` — the feeds the desktop could never reach

Two moves, and neither is a parser move. **The fetch** goes server-side, because a server can present a real browser's headers and follow a redirect chain a desktop `ureq` could not — which is what makes the HTML sources work at last. **The verdict** goes server-side, because that is D3. The roster, the pre-filter, the digest and the ledger stay exactly where they are, under the oracle that already covers them.

**Files:**
- Create: `cloud/supabase/functions/judge-event/index.ts`, `cloud/supabase/functions/events/{handler.ts,handler_test.ts,index.ts}`
- Test: new tests in `engine/src/events.rs`, one new test in `engine/tests/cloud_contract.rs`
- Modify: `engine/src/events.rs` (`judge_roster`), `engine/src/cloudmodel.rs` (`impl judge::EventModel`, `fetch_event_source`, `event_request`), `engine/src/enrich.rs` (one pull phase)
- **Hand-off this task needs:** H4 (`engine/src/cli.rs` — the events fetch proxy)

**Interfaces:**
- Consumes: Task 2's pipeline; Task 4's `judge::EventModel`, `EventItem`, `EventVerdict`.
- Produces:
  - `POST /judge-event` → the same `JudgeReply` shape, with `verdict: { verdict, why, confidence }`.
  - `POST /events` `{ url: string }` → `{ body: string }`, or a 400 for an unfetchable URL, or a 502 whose body names no URL.
  - `cloudmodel::fetch_event_source(client: &CloudClient, url: &str) -> Result<String, String>`
  - `cloudmodel::event_request(&judge::EventItem) -> serde_json::Value`
  - `events::judge_roster(vault: &Path, model: &dyn judge::EventModel, fetch: Option<&dyn Fn(&str) -> Result<String, String>>, today: Date, cap: usize) -> Vec<String>`
  - `events::ACTOR: &str = "agent:knowlu.events"`

**`judge_roster` judges the events the FEEDS carry, not the roster's own lines.** `eventroster::read_roster` reconstructs only `uid`, `title`, `start`, `end`, `organizer` and `location`, and hard-codes `source: "roster"` — everything else comes back `Default::default()`. Judging from it would send an empty `description`, empty `categories`, empty `audiences`, an empty `series_uid` and the same constant `source` for every event on every account, which is (a) most of what the `/judge-event` prompt is written around and (b) two of the four promotion features tier 2 is keyed on. So this pass calls `eventfeed::load_discovered_events` — the same function `rank`'s events pass calls, through the same `Fetchers.events` seam and therefore through the same server-side proxy — and filters to the uids the ledger has no verdict for. The cost is one extra fetch per slot; the gain is that the judgment sees the event.

- [ ] **Step 1: Write the failing engine tests** — in `engine/src/events.rs`'s test module:

```rust
    /// A scripted event model, so the roster pass is exercised with no socket at all.
    struct Scripted(std::cell::RefCell<Vec<Result<crate::judge::EventVerdict, crate::judge::ModelError>>>);

    impl crate::judge::EventModel for Scripted {
        fn judge_event(
            &self,
            _item: &crate::judge::EventItem,
        ) -> Result<crate::judge::EventVerdict, crate::judge::ModelError> {
            self.0
                .borrow_mut()
                .pop()
                .unwrap_or_else(|| Err(crate::judge::ModelError::Failed("nothing scripted".into())))
        }
    }

    fn verdict(word: &str, why: &str) -> crate::judge::EventVerdict {
        crate::judge::EventVerdict { verdict: word.into(), why: why.into(), confidence: 0.9, tier: 3 }
    }

    /// A vault with two events on one enabled ICS source, and a verdict already recorded for the
    /// first. Never points at the worktree (ruling R-3a-12).
    fn scratch_vault_with_feed(name: &str) -> (std::path::PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("knowlu-c2-events-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
        std::fs::create_dir_all(dir.join("state")).expect("scratch vault");
        std::fs::create_dir_all(dir.join("profile")).expect("scratch vault");
        std::fs::write(
            dir.join("config").join("events.yaml"),
            "sources:\n  - name: engage\n    type: ics\n    url: https://example.invalid/e.ics\n    enabled: true\n",
        )
        .expect("write events.yaml");
        std::fs::write(dir.join("profile").join("interests.md"), "Machine learning, German.\n")
            .expect("write interests.md");
        crate::eventledger::record_verdict(
            &dir, "engage:1", "AI Club Kickoff", jiff::civil::date(2026, 8, 20), "drop", "", "not this term", "",
        )
        .expect("seed a verdict");
        let feed = "BEGIN:VCALENDAR\r\n\
             BEGIN:VEVENT\r\nUID:engage:1\r\nSUMMARY:AI Club Kickoff\r\n\
             DTSTART:20260829T230000Z\r\nDTEND:20260830T000000Z\r\n\
             DESCRIPTION:An evening for anyone curious about machine learning.\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:engage:2\r\nSUMMARY:Career Fair\r\n\
             DTSTART:20260830T150000Z\r\nDTEND:20260830T190000Z\r\n\
             DESCRIPTION:Employers across engineering and computing.\r\nEND:VEVENT\r\n\
             END:VCALENDAR\r\n"
            .to_string();
        (dir, feed)
    }

    #[test]
    fn only_unjudged_uids_are_sent_and_one_verdict_per_uid_forever() {
        // §7's one-verdict-per-uid-forever rule is what makes a drop permanent, and re-asking
        // would quietly overwrite a decision the student already lives with.
        let (vault, feed) = scratch_vault_with_feed("unjudged");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("opportunity", "matches the stated interests"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150);
        assert!(lines.iter().any(|l| l.contains("engage:2") && l.contains("opportunity")), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("engage:1")), "{lines:?}");
        let ledger = crate::eventledger::load_ledger(&vault, None);
        assert_eq!(ledger["engage:2"].verdict.as_deref(), Some("opportunity"));
        assert_eq!(ledger["engage:1"].verdict.as_deref(), Some("drop"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_judged_item_carries_the_description_and_the_real_source() {
        // The defect this pass was rewritten to avoid: judging from `read_roster` would send an
        // empty description and the literal source "roster" for every event on every account,
        // which is most of the prompt and two of the four promotion features.
        let (vault, feed) = scratch_vault_with_feed("carries");
        let fetch = |_: &str| Ok(feed.clone());
        let seen: std::cell::RefCell<Vec<crate::judge::EventItem>> = std::cell::RefCell::new(Vec::new());
        struct Recorder<'a>(&'a std::cell::RefCell<Vec<crate::judge::EventItem>>);
        impl crate::judge::EventModel for Recorder<'_> {
            fn judge_event(
                &self,
                item: &crate::judge::EventItem,
            ) -> Result<crate::judge::EventVerdict, crate::judge::ModelError> {
                self.0.borrow_mut().push(item.clone());
                Err(crate::judge::ModelError::Failed("recorded only".into()))
            }
        }
        let _ = judge_roster(&vault, &Recorder(&seen), Some(&fetch), jiff::civil::date(2026, 8, 28), 150);
        let items = seen.borrow();
        assert_eq!(items.len(), 1);
        assert!(items[0].description.contains("Employers"), "{:?}", items[0].description);
        assert_eq!(items[0].source, "engage");
        assert!(items[0].interests.contains("Machine learning"));
        drop(items);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_verdict_word_the_ledger_refuses_is_dropped_and_named() {
        let (vault, feed) = scratch_vault_with_feed("badword");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("maybe", "unsure"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150);
        assert!(lines.iter().any(|l| l.contains("refused")), "{lines:?}");
        assert!(crate::eventledger::load_ledger(&vault, None).get("engage:2").is_none());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_why_carrying_the_field_separator_is_repaired_rather_than_lost() {
        // `eventledger::why_problem` refuses a double quote, a newline and the field separator
        // outright, and a refused why throws the whole verdict away. The service one-lines it; the
        // device checks again, because a service that changed and a device that did not is the
        // case this guards.
        let (vault, feed) = scratch_vault_with_feed("why");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![Ok(verdict("drop", "she said \"no\" \u{b7} twice"))]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150);
        assert!(lines.iter().any(|l| l.contains("engage:2")), "{lines:?}");
        let ledger = crate::eventledger::load_ledger(&vault, None);
        assert_eq!(ledger["engage:2"].verdict.as_deref(), Some("drop"));
        assert!(!ledger["engage:2"].why.contains('"'));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn the_per_run_cap_bounds_the_batch_and_says_what_is_left() {
        let (vault, feed) = scratch_vault_with_feed("cap");
        let fetch = |_: &str| Ok(feed.clone());
        let model = Scripted(std::cell::RefCell::new(vec![]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 0);
        assert!(lines.iter().any(|l| l.contains("1 left for the next slot")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_feed_that_will_not_fetch_is_a_line_and_never_a_panic() {
        let (vault, _) = scratch_vault_with_feed("deadfeed");
        let fetch = |_: &str| Err("connection refused".to_string());
        let model = Scripted(std::cell::RefCell::new(vec![]));
        let lines = judge_roster(&vault, &model, Some(&fetch), jiff::civil::date(2026, 8, 28), 150);
        assert!(lines.iter().any(|l| l.contains("fetch failed")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

- [ ] **Step 2: Run it and watch it fail.** `cargo test -p knowlu-engine events::` → `cannot find function judge_roster`.

- [ ] **Step 3: Write `judge_roster` in `engine/src/events.rs`.**

```rust
/// The agent actor for event verdicts. `agent:` prefix, because `provenance::is_agent` is a plain
/// `starts_with("agent:")` test and nothing else.
pub const ACTOR: &str = "agent:knowlu.events";

/// Judge every event the feeds carry that has no verdict yet, and record what comes back.
///
/// **One verdict per uid, forever** (events spec §7). The ledger is append-only and a uid that
/// already has a verdict is never re-asked, because a drop the student has lived with for a week
/// must not silently become an opportunity.
///
/// **Judged from the FEEDS, not from `state/events.md`.** `eventroster::read_roster` reconstructs
/// six fields and hard-codes `source: "roster"`; the description, the categories, the audiences and
/// the series uid — most of what the prompt is written around, and two of the four promotion
/// features — are not in it. So this calls `eventfeed::load_discovered_events`, the same function
/// `rank`'s events pass calls, through the same `Fetchers.events` seam and therefore through the
/// same server-side proxy (hand-off H4). One extra fetch per slot buys a judgment that can see the
/// event.
///
/// **Writes only the ledger.** Nothing here writes `state/events.md` — `rank` regenerates it a few
/// seconds later, and by then the verdicts are in the ledger it reads.
///
/// Never panics and never fails a run: every failure is a line.
pub fn judge_roster(
    vault: &Path,
    model: &dyn crate::judge::EventModel,
    fetch: Option<&dyn Fn(&str) -> Result<String, String>>,
    today: Date,
    cap: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let (config, _) = load_events_config(&vault.join("config").join("events.yaml"));
    if !config.sources.iter().any(|s| s.enabled) {
        return lines;
    }
    let (discovered, warnings) = crate::eventfeed::load_discovered_events(vault, fetch);
    for warning in warnings {
        lines.push(format!("events: {warning}"));
    }
    if discovered.is_empty() {
        return lines;
    }

    let ledger = crate::eventledger::load_ledger(vault, None);
    let interests = crate::pystr::read_text(&vault.join("profile").join("interests.md"))
        .map(|t| crate::judge::clip(t.trim(), crate::judge::MAX_PREFS_CHARS))
        .unwrap_or_default();

    let pending: Vec<&DiscoveredEvent> = discovered
        .iter()
        .filter(|e| ledger.get(&e.uid).and_then(|entry| entry.verdict.as_ref()).is_none())
        .collect();
    let left = pending.len().saturating_sub(cap);
    let mut judged = 0usize;

    for event in pending.into_iter().take(cap) {
        let item = crate::judge::EventItem {
            uid: event.uid.clone(),
            title: crate::judge::one_line(&event.title, 200),
            start: event.start().strftime("%Y-%m-%dT%H:%M").to_string(),
            end: event.end().strftime("%Y-%m-%dT%H:%M").to_string(),
            source: event.source.clone(),
            organizer: crate::judge::one_line(&event.organizer, 120),
            location: crate::judge::one_line(&event.location, 120),
            url: event.url.clone(),
            description: crate::judge::clip(event.description.trim(), crate::judge::MAX_BODY_CHARS),
            categories: event.categories.clone(),
            audiences: event.audiences.clone(),
            series_uid: event.series_uid.clone(),
            interests: interests.clone(),
        };
        let verdict = match model.judge_event(&item) {
            Ok(v) => v,
            Err(e) => {
                lines.push(format!("events {}: not judged ({e})", event.uid));
                continue;
            }
        };
        // Checked again here, not only on the server: `record_verdict` refuses an unknown word and
        // a `why` carrying a quote, a newline or the field separator, and a refusal at that depth
        // would lose the verdict with no line to explain it.
        if !crate::eventledger::VALID_VERDICTS.contains(&verdict.verdict.as_str()) {
            lines.push(format!("events {}: refused ({:?} is not a verdict)", event.uid, verdict.verdict));
            continue;
        }
        let why = crate::judge::one_line(
            &verdict.why.replace('"', "'").replace(" \u{b7} ", " - "),
            140,
        );
        match crate::eventledger::record_verdict(
            vault, &event.uid, &event.title, today, &verdict.verdict, "", &why, "",
        ) {
            Ok(()) => {
                judged += 1;
                lines.push(format!("events {}: {} ({:.2})", event.uid, verdict.verdict, verdict.confidence));
            }
            Err(e) => lines.push(format!("events {}: not recorded ({e:?})", event.uid)),
        }
    }
    let mut summary = format!("events: {judged} judged");
    if left > 0 {
        summary.push_str(&format!(", {left} left for the next slot"));
    }
    lines.push(summary);
    lines
}
```

- [ ] **Step 4: Write the two functions in `engine/src/cloudmodel.rs`.**

```rust
/// The body of `POST /judge-event`. The event as the feed carries it, plus
/// `profile/interests.md` as its grounding — the same shape a task's grade weights have. No vault
/// path, no account id, no config file.
pub fn event_request(item: &judge::EventItem) -> Value {
    json!({
        "kind": "event",
        "item": {
            "uid": item.uid, "title": item.title, "start": item.start, "end": item.end,
            "source": item.source, "organizer": item.organizer, "location": item.location,
            "url": item.url, "description": item.description,
            "categories": item.categories, "audiences": item.audiences, "series_uid": item.series_uid,
        },
        "heuristics_seed": { "interests": item.interests }
    })
}

impl judge::EventModel for CloudModel<'_> {
    fn judge_event(&self, item: &judge::EventItem) -> Result<judge::EventVerdict, ModelError> {
        let reply = self.call("/judge-event", &event_request(item))?;
        let verdict = reply.get("verdict").filter(|v| !v.is_null()).ok_or_else(|| {
            let cause = reply.get("cause").and_then(Value::as_str).unwrap_or("no verdict");
            ModelError::Failed(format!("the judgment service answered {cause}"))
        })?;
        Ok(judge::EventVerdict {
            verdict: verdict.get("verdict").and_then(Value::as_str).unwrap_or_default().to_string(),
            why: judge::one_line(verdict.get("why").and_then(Value::as_str).unwrap_or(""), 140),
            confidence: verdict.get("confidence").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0),
            tier: reply.get("tier").and_then(Value::as_u64).unwrap_or(3).min(3) as u8,
        })
    }
}

/// Fetch one event feed through the service (cloud design §3.1). **Transport, and nothing else**:
/// this is what `rank`'s existing `Fetchers.events` seam is handed, and it is why the HTML sources
/// finally return a page — a server can present a real browser's headers and follow a redirect
/// chain the desktop's agent could not. `rank` still never calls a model (decision 11), and
/// `rank_cannot_reach_a_judgment_endpoint` proves it structurally.
///
/// `Err(String)` rather than `CloudError`, because the caller is `Fetchers.events`, whose contract
/// predates this module and whose failure already degrades to "keep the last known roster".
pub fn fetch_event_source(client: &CloudClient, url: &str) -> Result<String, String> {
    let reply = client.post("/events", &json!({ "url": url })).map_err(|e| e.to_string())?;
    reply
        .get("body")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "the reply carried no body field".to_string())
}
```

- [ ] **Step 5: Write the guard test that `rank` cannot reach a judgment** — in `engine/tests/cloud_contract.rs`. Hand-off H4 puts `cloudmodel::resolve` inside `cli::run`, so from here on the module that defines `CloudModel: judge::Model` is reachable from `rank`. The Global Constraint at the top of this plan says no path under `cli.rs` may reach `/judge-*`; until now that was enforced by the module graph, and now it is enforced by this:

```rust
/// `rank` never calls a model (decision 11), and after hand-off H4 that is a property of the
/// SOURCE rather than of the module graph — `cli.rs` links `cloudmodel` for the events fetch
/// proxy. So it is pinned the way the SDK boundary is pinned (`dependency_boundary.rs`): cheaply,
/// statically, and at the moment somebody writes the wrong line rather than the moment a slot
/// starts judging in the wrong step.
#[test]
fn rank_cannot_reach_a_judgment_endpoint() {
    let cli = include_str!("../src/cli.rs");
    for forbidden in ["/judge-task", "/judge-event", "/judge-email", "judge_task", "CloudModel", "EventModel", "EmailModel"] {
        assert!(
            !cli.contains(forbidden),
            "engine/src/cli.rs mentions `{forbidden}`. `rank` may reach the service for TRANSPORT \
             (cloudmodel::fetch_event_source, cloudmodel::fetch_ics) and for nothing else: judgment \
             is the separate `judge` command, which runs before `rank` and writes fields into notes \
             (Knowlu spec decision 11, CLAUDE.md)."
        );
    }
    // And the two transport functions ARE allowed, so this test fails loudly if H4 was never
    // applied rather than passing vacuously.
    assert!(
        cli.contains("cloudmodel::fetch_event_source"),
        "hand-off H4 has not been applied: `cli::run` still fetches event feeds on the device."
    );
}
```

- [ ] **Step 6: Write `cloud/supabase/functions/events/handler.ts`.**

```ts
// POST /events — fetch one event feed on the student's behalf.
//
// This exists because a desktop `ureq` cannot get past several campus HTML sources: they answer a
// non-browser agent with a challenge page or a redirect chain it will not follow. A server can
// present the headers a browser presents and follow the chain, which is the whole reason §3.1
// moves the fetch (and nothing else) into the cloud.
//
// **Transport only.** No judgment happens here, no vault is touched, and the reply is the page's
// bytes. It is also an outbound fetch from our infrastructure on a URL a client supplied, so the
// URL is checked before it is used: https only, a public hostname, never a private or loopback
// address — an SSRF against the project's own metadata endpoint is the failure this prevents.
import type { Entitle } from "../_shared/judge_handler.ts";

export const EVENT_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";
export const MAX_BODY_BYTES = 8 * 1024 * 1024;

const PRIVATE =
  /^(localhost$|127\.|10\.|192\.168\.|169\.254\.|172\.(1[6-9]|2\d|3[01])\.|0\.|\[?::1\]?$|\[?fd[0-9a-f]{2}:)/i;

export function allowedUrl(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== "https:") return false;
  if (url.hostname === "" || PRIVATE.test(url.hostname)) return false;
  // A bare label ("intranet") resolves differently per network and is never a public feed.
  if (!url.hostname.includes(".")) return false;
  return true;
}

export function eventsHandler(
  entitle: Entitle,
  fetchText: (url: string) => Promise<string> = liveFetch,
): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      await entitle(req);
      const body = await req.json().catch(() => ({})) as { url?: unknown };
      const url = typeof body.url === "string" ? body.url : "";
      if (!allowedUrl(url)) return Response.json({ error: "not a fetchable https URL" }, { status: 400 });
      try {
        return Response.json({ body: await fetchText(url) });
      } catch {
        // The cause is swallowed deliberately: a transport error's text quotes the URL, and this
        // body reaches the device and its log.
        return Response.json({ error: "the source could not be fetched" }, { status: 502 });
      }
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`events: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "fetch failed" }, { status: 500 });
    }
  };
}

async function liveFetch(url: string): Promise<string> {
  const response = await fetch(url, {
    headers: {
      "User-Agent": EVENT_USER_AGENT,
      Accept: "text/html,application/xhtml+xml,application/json;q=0.9,text/calendar;q=0.9,*/*;q=0.8",
      "Accept-Language": "en-US,en;q=0.9",
    },
    redirect: "follow",
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.byteLength > MAX_BODY_BYTES) throw new Error("body too large");
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}
```

- [ ] **Step 7: Write `cloud/supabase/functions/events/handler_test.ts`.**

```ts
import { assert, assertEquals } from "@std/assert";
import { allowedUrl, eventsHandler } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

function post(url: unknown): Request {
  return new Request("http://127.0.0.1/events", { method: "POST", body: JSON.stringify({ url }) });
}

Deno.test("a public https feed is fetched and returned whole", async () => {
  let asked = "";
  const handler = eventsHandler(OK, (url) => {
    asked = url;
    return Promise.resolve("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n");
  });
  const response = await handler(post("https://calendar.example.edu/events.ics"));
  assertEquals(response.status, 200);
  assert((await response.json()).body.includes("BEGIN:VCALENDAR"));
  assertEquals(asked, "https://calendar.example.edu/events.ics");
});

Deno.test("nothing private, plaintext, bare or malformed is ever fetched", async () => {
  // An outbound fetch on a client-supplied URL is an SSRF surface; the project's own metadata
  // endpoint is one hop away on the loopback interface.
  for (const bad of [
    "http://calendar.example.edu/e.ics",
    "https://127.0.0.1/e.ics",
    "https://localhost/e.ics",
    "https://10.1.2.3/e.ics",
    "https://192.168.0.9/e.ics",
    "https://169.254.169.254/latest/meta-data/",
    "https://172.16.4.4/e.ics",
    "https://intranet/e.ics",
    "not a url",
    "",
  ]) {
    assertEquals(allowedUrl(bad), false, `${bad} must not be fetchable`);
    let fetched = false;
    const handler = eventsHandler(OK, () => {
      fetched = true;
      return Promise.resolve("");
    });
    const response = await handler(post(bad));
    assertEquals(response.status, 400, bad);
    assertEquals(fetched, false, `${bad} reached the fetcher`);
  }
});

Deno.test("a fetch that throws is a 502 whose body names no URL", async () => {
  const handler = eventsHandler(OK, () =>
    Promise.reject(new Error("getaddrinfo ENOTFOUND calendar.example.edu/secret-token")));
  const response = await handler(post("https://calendar.example.edu/secret-token/e.ics"));
  assertEquals(response.status, 502);
  assertEquals((await response.text()).includes("secret-token"), false);
});

Deno.test("the entitlement check runs before any fetch", async () => {
  let fetched = false;
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = eventsHandler(refuse, () => {
    fetched = true;
    return Promise.resolve("");
  });
  assertEquals((await handler(post("https://calendar.example.edu/e.ics"))).status, 402);
  assertEquals(fetched, false);
});

Deno.test("a GET is a 405", async () => {
  const handler = eventsHandler(OK, () => Promise.resolve(""));
  assertEquals((await handler(new Request("http://127.0.0.1/events"))).status, 405);
});
```

- [ ] **Step 8: Write the two `index.ts` files.**

```ts
// cloud/supabase/functions/judge-event/index.ts
//
// `origin: "events"`, so a judgment made for the events pass is distinguishable in `judgments`
// from one made for a task — which is what the eval's per-kind metrics and the correction-rate
// dashboards key on.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("event", requireActiveEntitlement, (kind) => liveDeps(kind, "events")));
```

```ts
// cloud/supabase/functions/events/index.ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { eventsHandler } from "./handler.ts";

Deno.serve(eventsHandler(requireActiveEntitlement));
```

- [ ] **Step 9: Add the pull phase to `enrich.rs`'s cloud arm.** This is an **insertion** into `run_lines_with` between the `enrich_with` call and the `model.fatal()` line Task 4 put there — do not replace either:

```rust
    let model = crate::cloudmodel::CloudModel::new(client);
    let (code, mut lines) = match model.probe() {
        Some(reason) => {
            let (code, mut lines) = enrich_with(vault, opts, Err(judge::Missing::Service(reason)));
            lines.insert(0, format!("judge: the service answered {reason}; nothing was sent"));
            (code, lines)
        }
        None => enrich_with(vault, opts, Ok(&model)),
    };

    // C2 Task 9 — the events pass. Runs even when the enrichment batch was empty: a vault can have
    // nothing to enrich and forty events to judge. The feeds are fetched through the same
    // server-side proxy `rank` uses, so the HTML sources return a page here too.
    let (events_config, _) =
        crate::events::load_events_config(&vault.join("config").join("events.yaml"));
    let proxy = |url: &str| -> Result<String, String> {
        crate::cloudmodel::fetch_event_source(client, url)
            .or_else(|_| crate::eventfeed::fetch_event_source(url))
    };
    lines.extend(crate::events::judge_roster(
        vault,
        &model,
        Some(&proxy),
        jiff::Zoned::now().date(),
        events_config.judge_per_run_cap.max(0) as usize,
    ));

    if let Some(reason) = model.fatal() {
        lines.push(format!(
            "judge: the service answered {reason}, so the rest of the batch was not sent"
        ));
    }
    (code, lines)
```

`model.fatal()` stays **last** on purpose: it now also covers the events calls, so one 402 reported once covers both passes rather than fifty lines from one and forty from the other.

- [ ] **Step 10: Run everything.**

Run: `cargo test --workspace` then `deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/`
Expected: both green at 0 warnings; `oracle.rs` and `surface_oracle.rs` unchanged. `rank_cannot_reach_a_judgment_endpoint` is **red until hand-off H4 is applied** — say so in the report rather than weakening the assertion.

- [ ] **Step 11: Deploy, name hand-off H4, commit.**

```
supabase functions deploy judge-event --project-ref <staging ref>
supabase functions deploy events --project-ref <staging ref>
```

```bash
git add cloud/supabase/functions/judge-event/ cloud/supabase/functions/events/ engine/src/events.rs engine/src/cloudmodel.rs engine/src/enrich.rs engine/tests/cloud_contract.rs
git commit -F .git-commit-msg.txt   # "engine+cloud: event verdicts move to the service and the feeds are fetched server-side, so the HTML sources work (C2 Task 9)"
```

---

### Task 10: Google — connect the calendars first, Gmail incrementally, and disconnect that really revokes

**Precondition P2** (the Google OAuth client id and secret as staging project secrets).

**Files:**
- Create: `cloud/supabase/migrations/20260911000200_google.sql`, `cloud/supabase/functions/google-connect/{handler.ts,handler_test.ts,index.ts}`, `cloud/supabase/functions/google-callback/{handler.ts,handler_test.ts,index.ts}`

**Interfaces:**
- Produces: `GET /google-connect?scope=calendar|gmail` → `{ url: string }`; **`GET /google-connect?status=1` → `{ connected: boolean, scopes: string[] }`**; `DELETE /google-connect` → `{ disconnected: true }`; `GET /google-callback?code&state` → an HTML page saying the window may be closed. Tables `google_accounts` (with `scopes text[]`), `gmail_seen`, `gmail_queue`, `google_state`; the function `export_training_rows`.
- `CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly"`, `GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly"`.

**The order of the two scopes is the point of this task** (§11a). The wizard's *first* connection step is *Connect your calendars*, so the **first** Google ask is `calendar.readonly` alone — a **sensitive** scope, which means lighter verification and **no CASA assessment**. `gmail.readonly` is added **incrementally**, from the Gmail step, only if the student takes it, with `include_granted_scopes=true` so the second consent widens the same grant rather than replacing it. `google_accounts.scopes` records what Google actually returned, and every reader checks it: `/ingest-calendar` refuses without `calendar.readonly`, `gmail-read` refuses without `gmail.readonly`. Calendar *write* remains a third, later ask — only when the student approves their first calendar-event card (VISION: rare writes behind explicit approval).

**Testing-mode realities apply to `calendar.readonly` exactly as to Gmail** (§9): while the Google project is in *Testing*, at most 100 test users, a tester warning screen, and refresh tokens that expire after **7 days** — for either scope. The wizard says so on the calendar panel as well as the Gmail one (hand-off H9's copy). The difference between the two is verification, not testing mode: the sensitive scope's review is lighter and carries no annual security assessment, which is why the calendar is the one that ships first.

- [ ] **Step 1: Write `20260911000200_google.sql`.**

```sql
-- Knowlu C2 — Gmail, server-side (cloud design §5.3, D12).
--
-- The refresh token is a restricted-scope credential and never reaches the device. It is held in
-- Supabase Vault, and this table keeps only the Vault secret's id — so a dump of this table is not
-- a dump of anybody's mailbox access, and Alabama's SPII definition (§9) is satisfied by the
-- Vault's encryption rather than by our care.
create table if not exists google_accounts (
  account_id      uuid primary key references public.accounts(id) on delete cascade,
  google_sub      text not null,
  secret_id       uuid not null,
  email_hint      text,
  -- What Google actually granted, not what was asked for (§11a). The calendar scope comes first
  -- and alone; `gmail.readonly` is added incrementally and only if the student takes that step, so
  -- every reader checks this rather than assuming: `/ingest-calendar` refuses without the calendar
  -- scope, `gmail-read` refuses without the Gmail one.
  scopes          text[] not null default '{}',
  -- The seam for §5.3's "user-excludable labels". No UI chooses them yet — the settings panel is
  -- C1's and the wizard's Gmail step asks only connect-or-not — so this is empty for everyone and
  -- `gmail-read` turns each entry into a `-label:<name>` term. The day the control exists it is a
  -- UI change and no schema change. Recorded as a narrowing in the fidelity ledger.
  excluded_labels text[] not null default '{}',
  connected_at    timestamptz not null default now(),
  last_read_at    timestamptz,
  -- `quiet` is §5.3's "source went quiet": connected, but yielding nothing for 14 days.
  status          text not null default 'active' check (status in ('active', 'revoked', 'quiet'))
);
alter table google_accounts enable row level security;

-- The dedup set. `gmail:<message-id>` is the same uid the device's state/ingest-seen.md holds, so
-- a message is judged once whichever side asks.
create table if not exists gmail_seen (
  account_id uuid not null references public.accounts(id) on delete cascade,
  uid        text not null,
  seen_at    timestamptz not null default now(),
  primary key (account_id, uid)
);
alter table gmail_seen enable row level security;

-- What the device has not pulled yet. `payload` is the VERDICT and the fields to write — never
-- the message text, which is discarded the moment the judgment returns (§5.3).
create table if not exists gmail_queue (
  id           bigserial primary key,
  account_id   uuid not null references public.accounts(id) on delete cascade,
  uid          text not null,
  tier         text not null check (tier in ('task', 'borderline', 'event', 'opportunity', 'information')),
  payload      jsonb not null,
  judgment_id  uuid references judgments(id) on delete set null,
  queued_at    timestamptz not null default now(),
  delivered_at timestamptz
);
alter table gmail_queue enable row level security;
create index if not exists gmail_queue_undelivered on gmail_queue (account_id, queued_at) where delivered_at is null;

-- The single-use OAuth `state` nonce. Bound to an account, consumed by the callback, and expired
-- after ten minutes: without it the callback would take any code from anyone and attach the
-- resulting mailbox to whichever account the URL happened to name.
create table if not exists google_state (
  nonce      text primary key,
  account_id uuid not null references public.accounts(id) on delete cascade,
  issued_at  timestamptz not null default now()
);
alter table google_state enable row level security;

-- The expiry is enforced, not merely commented: `take_google_state` refuses an old nonce in the
-- same statement that consumes it, so there is no window between the check and the delete.
create or replace function take_google_state(p_nonce text)
returns uuid
language sql
security invoker
set search_path = public, extensions
as $$
  delete from google_state
   where nonce = p_nonce and issued_at > now() - interval '10 minutes'
  returning account_id;
$$;

-- Housekeeping: nonces nobody came back for. Cheap, and it runs beside the nightly promotion.
select cron.schedule('knowlu-sweep-google-state', '23 7 * * *',
  $$delete from google_state where issued_at < now() - interval '1 day';$$);

-- Telemetry class (c) — raw content, behind its own opt-in — must NEVER include anything derived
-- from the Gmail API, whatever the user has opted into. Google's Workspace API User Data policy
-- (2026-07-22) allows no training "beyond that specific user's personalized model", and per-user
-- rule promotion is the only learning we do on it (§5.3, §9). One predicate, and losing it would
-- be silent, so `gmail_rows_are_excluded_from_the_training_export` pins its text.
create or replace function export_training_rows(p_since timestamptz)
returns setof judgments
language sql
stable
security invoker
set search_path = public, extensions
as $$
  select * from judgments where origin <> 'gmail_api' and judged_at >= p_since;
$$;
```

- [ ] **Step 2: Write the failing connect test** — `cloud/supabase/functions/google-connect/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { CALENDAR_SCOPE, connectHandler, GMAIL_SCOPE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

function deps(overrides: Record<string, unknown> = {}) {
  return {
    clientId: "client-id-not-a-secret",
    redirectUri: "https://ref.supabase.co/functions/v1/google-callback",
    saveState: () => Promise.resolve("state-nonce"),
    disconnect: () => Promise.resolve(),
    grantedScopes: () => Promise.resolve([]),
    ...overrides,
  };
}

Deno.test("the DEFAULT ask is calendar.readonly alone, offline, with consent", async () => {
  // §11a: the wizard's first connection step is "Connect your calendars", so the first Google ask
  // is the *sensitive* calendar scope — lighter verification, no CASA. Gmail comes later or not
  // at all.
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect"))).json();
  const url = new URL(reply.url);
  assertEquals(url.origin + url.pathname, "https://accounts.google.com/o/oauth2/v2/auth");
  assertEquals(url.searchParams.get("scope"), CALENDAR_SCOPE);
  assertEquals(CALENDAR_SCOPE, "https://www.googleapis.com/auth/calendar.readonly");
  assert(!url.searchParams.get("scope")!.includes("gmail"), "Gmail is a separate, later, optional ask");
  assert(!url.searchParams.get("scope")!.includes("calendar.events"), "read-only; write is a third ask");
  assertEquals(url.searchParams.get("access_type"), "offline");
  assertEquals(url.searchParams.get("prompt"), "consent");
  assertEquals(url.searchParams.get("include_granted_scopes"), "false");
  assertEquals(url.searchParams.get("state"), "state-nonce");
  assertEquals(url.searchParams.get("response_type"), "code");
});

Deno.test("the gmail ask is incremental — one scope, and it widens the existing grant", async () => {
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?scope=gmail"))).json();
  const url = new URL(reply.url);
  assertEquals(url.searchParams.get("scope"), GMAIL_SCOPE);
  assertEquals(GMAIL_SCOPE, "https://www.googleapis.com/auth/gmail.readonly");
  // `include_granted_scopes=true` is what makes this ADD to the grant rather than replace it: a
  // student who connected the calendar in the wizard must not lose it by connecting Gmail later.
  assertEquals(url.searchParams.get("include_granted_scopes"), "true");
});

Deno.test("an unknown scope name is a 400, not a silent calendar grant", async () => {
  const handler = connectHandler(OK, deps());
  const response = await handler(new Request("http://127.0.0.1/google-connect?scope=drive"));
  assertEquals(response.status, 400);
});

Deno.test("?status=1 reports what was granted, and mints no nonce", async () => {
  // The wizard's poll (hand-off H9): the consent window closes itself, so this is the only thing
  // that tells the device the round trip finished. It must NOT start a second consent — a poll
  // that minted a nonce every three seconds would fill `google_state` and leak a fresh URL.
  let minted = 0;
  const handler = connectHandler(OK, deps({
    grantedScopes: () => Promise.resolve([CALENDAR_SCOPE]),
    saveState: () => { minted += 1; return Promise.resolve("n"); },
  }));
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, true);
  assertEquals(reply.scopes, [CALENDAR_SCOPE]);
  assertEquals(reply.url, undefined, "a status check is not a consent");
  assertEquals(minted, 0);
});

Deno.test("?status=1 on an account that never connected is connected:false and no scopes", async () => {
  const handler = connectHandler(OK, deps());
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, false);
  assertEquals(reply.scopes, []);
});

Deno.test("?status=1 distinguishes a gmail-only grant from a calendar one", async () => {
  // The wizard keys on the CALENDAR scope specifically, because a student can untick one on the
  // consent screen and a bare boolean would let the panel say the calendar is on when it is not.
  const handler = connectHandler(OK, deps({ grantedScopes: () => Promise.resolve([GMAIL_SCOPE]) }));
  const reply = await (await handler(new Request("http://127.0.0.1/google-connect?status=1"))).json();
  assertEquals(reply.connected, true);
  assertEquals(reply.scopes.includes(CALENDAR_SCOPE), false);
});

Deno.test("disconnect revokes at Google before it forgets the row", async () => {
  const order: string[] = [];
  const handler = connectHandler(OK, deps({
    disconnect: () => {
      order.push("revoked-then-deleted");
      return Promise.resolve();
    },
  }));
  const response = await handler(new Request("http://127.0.0.1/google-connect", { method: "DELETE" }));
  assertEquals(response.status, 200);
  assertEquals(await response.json(), { disconnected: true });
  assertEquals(order, ["revoked-then-deleted"]);
});

Deno.test("a disconnect that Google refuses is still a disconnect here", async () => {
  // The user asked to be disconnected. If the revoke call fails we must still stop reading their
  // mail: the row goes whatever Google says, and the failure is a log line, not a 500 that leaves
  // a live grant behind a UI that says "disconnected".
  const handler = connectHandler(OK, deps({ disconnect: () => Promise.reject(new Error("google 503")) }));
  const response = await handler(new Request("http://127.0.0.1/google-connect", { method: "DELETE" }));
  assertEquals(response.status, 200);
  assertEquals((await response.json()).disconnected, true);
});

Deno.test("a missing client id is a 503 that names the configuration, not the account", async () => {
  const handler = connectHandler(OK, deps({ clientId: "" }));
  const response = await handler(new Request("http://127.0.0.1/google-connect"));
  assertEquals(response.status, 503);
  assertEquals((await response.json()).error, "Gmail is not configured on this deployment");
});

Deno.test("the entitlement check runs before a nonce is minted", async () => {
  let minted = false;
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = connectHandler(refuse, deps({
    saveState: () => {
      minted = true;
      return Promise.resolve("n");
    },
  }));
  assertEquals((await handler(new Request("http://127.0.0.1/google-connect"))).status, 402);
  assertEquals(minted, false);
});
```

- [ ] **Step 3: Write `google-connect/handler.ts`.**

```ts
// GET /google-connect?scope=calendar|gmail -> the Google consent URL.
// DELETE /google-connect                    -> revoke and forget.
//
// **One Google connect, two scopes, in a deliberate order** (cloud design §11a). The wizard's
// first connection step is *Connect your calendars*, so the default and first ask is
// `calendar.readonly` **alone** — a *sensitive* scope, which means a lighter verification review
// and **no CASA security assessment**. `gmail.readonly` is *restricted*, is asked for later and
// only if the student takes the Gmail step, and is added **incrementally**
// (`include_granted_scopes=true`) so the second consent widens the same grant instead of replacing
// it. Calendar *write* is a third ask, later still, only when the student approves their first
// calendar-event card (VISION: rare writes behind explicit approval).
//
// While the Google project is in Testing (§9), and **for both scopes equally**: at most 100 test
// users, a tester warning screen, and refresh tokens that expire after 7 days. The wizard says
// exactly that on both panels — the wizard is C1's; this is the endpoint behind it, and its copy
// is hand-off H9.
import type { Entitle } from "../_shared/judge_handler.ts";

export const CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly";
export const GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly";
export const AUTH_ENDPOINT = "https://accounts.google.com/o/oauth2/v2/auth";

/** `?scope=` -> the one scope this consent asks for, and whether it widens an existing grant. */
export function scopeFor(name: string | null): { scope: string; incremental: boolean } | null {
  if (name === null || name === "" || name === "calendar") {
    return { scope: CALENDAR_SCOPE, incremental: false };
  }
  if (name === "gmail") return { scope: GMAIL_SCOPE, incremental: true };
  return null;
}

export interface ConnectDeps {
  clientId: string;
  redirectUri: string;
  /** Persists a single-use nonce bound to this account and returns it. */
  saveState(accountId: string): Promise<string>;
  /** Revokes the refresh token at Google, then deletes the Vault secret and the row — in that order. */
  disconnect(accountId: string): Promise<void>;
  /** What Google actually granted, from `google_accounts.scopes`. `[]` when there is no grant. */
  grantedScopes(accountId: string): Promise<string[]>;
}

export function connectHandler(entitle: Entitle, deps: ConnectDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      const { account_id } = await entitle(req);
      if (deps.clientId === "") {
        return Response.json({ error: "Gmail is not configured on this deployment" }, { status: 503 });
      }
      if (req.method === "DELETE") {
        try {
          await deps.disconnect(account_id);
        } catch (e) {
          // The user asked to be disconnected; a failed revoke must not leave a live grant behind
          // a UI that says "disconnected". The row goes either way, and this is the record.
          console.error(`google-connect: revoke failed (${e instanceof Error ? e.constructor.name : "unknown"})`);
        }
        return Response.json({ disconnected: true });
      }
      if (req.method !== "GET") return Response.json({ error: "GET or DELETE" }, { status: 405 });
      // `?status=1` — the only question the WIZARD can ask, because the consent window closes
      // itself and nothing else tells the device the round trip finished (hand-off H9's poll).
      // Deliberately reports the scopes rather than a bare boolean: a student can untick one on
      // the consent screen, and "connected" without "which" would be a wizard that says the
      // calendar is on when only Gmail is.
      if (new URL(req.url).searchParams.get("status") !== null) {
        const scopes = await deps.grantedScopes(account_id);
        return Response.json({ connected: scopes.length > 0, scopes });
      }
      const asked = scopeFor(new URL(req.url).searchParams.get("scope"));
      if (asked === null) {
        return Response.json({ error: "scope must be 'calendar' or 'gmail'" }, { status: 400 });
      }
      const url = new URL(AUTH_ENDPOINT);
      url.searchParams.set("client_id", deps.clientId);
      url.searchParams.set("redirect_uri", deps.redirectUri);
      url.searchParams.set("response_type", "code");
      url.searchParams.set("scope", asked.scope);
      url.searchParams.set("access_type", "offline");
      // `consent` every time: without it Google returns no refresh token on a re-connect, and a
      // re-connect is the normal case while the project is in Testing and tokens die weekly.
      url.searchParams.set("prompt", "consent");
      // Incremental only for the second ask: the Gmail consent must WIDEN the calendar grant, not
      // replace it — a student who connected the calendar in the wizard must not lose it by
      // connecting Gmail a week later.
      url.searchParams.set("include_granted_scopes", asked.incremental ? "true" : "false");
      url.searchParams.set("state", await deps.saveState(account_id));
      return Response.json({ url: url.toString() });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`google-connect: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "connect failed" }, { status: 500 });
    }
  };
}
```

- [ ] **Step 4: Write the failing callback test** — `cloud/supabase/functions/google-callback/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { callbackHandler } from "./handler.ts";

const TOKENS = {
  refresh_token: "REFRESH-TRIPWIRE-9f2c",
  access_token: "ACCESS-TRIPWIRE-9f2c",
  sub: "1029384756",
  email: "student@example.invalid",
};

function deps(overrides: Record<string, unknown> = {}) {
  return {
    clientId: "client-id-not-a-secret",
    clientSecret: "client-secret-not-a-secret",
    redirectUri: "https://ref.supabase.co/functions/v1/google-callback",
    takeState: (s: string) => Promise.resolve(s === "good" ? "acct-1" : null),
    exchange: () => Promise.resolve(TOKENS),
    storeRefreshToken: () => Promise.resolve(),
    ...overrides,
  };
}

function get(query: string): Request {
  return new Request(`http://127.0.0.1/google-callback?${query}`);
}

Deno.test("a good code stores the refresh token and says the window may be closed", async () => {
  let stored: unknown = null;
  const handler = callbackHandler(deps({
    storeRefreshToken: (accountId: string, sub: string, email: string | undefined, token: string) => {
      stored = { accountId, sub, email, token };
      return Promise.resolve();
    },
  }));
  const response = await handler(get("state=good&code=abc"));
  assertEquals(response.status, 200);
  const page = await response.text();
  assert(page.includes("close this window"));
  // The token reaches the store and NOTHING else — not the page, not a header.
  assertEquals((stored as { token: string }).token, TOKENS.refresh_token);
  assertEquals(page.includes("TRIPWIRE-9f2c"), false);
});

Deno.test("the state nonce is required and single use", async () => {
  const handler = callbackHandler(deps());
  assertEquals((await handler(get("code=abc"))).status, 400);
  assertEquals((await handler(get("state=replayed&code=abc"))).status, 400);
  assertEquals((await handler(get("state=good"))).status, 400, "a state with no code is not a connect");
});

Deno.test("a Google error is a page, not a stack trace, and names no token", async () => {
  const handler = callbackHandler(deps({ exchange: () => Promise.reject(new Error("invalid_grant REFRESH-TRIPWIRE-9f2c")) }));
  const response = await handler(get("state=good&code=abc"));
  const page = await response.text();
  assert(page.includes("try again"));
  assertEquals(page.includes("TRIPWIRE-9f2c"), false);
});

Deno.test("a user who declined at Google gets a page and no exchange is attempted", async () => {
  let exchanged = false;
  const handler = callbackHandler(deps({
    exchange: () => {
      exchanged = true;
      return Promise.resolve(TOKENS);
    },
  }));
  const response = await handler(get("state=good&error=access_denied"));
  assertEquals(response.status, 200);
  assert((await response.text()).includes("not connected"));
  assertEquals(exchanged, false);
});
```

- [ ] **Step 5: Write `google-callback/handler.ts`.**

```ts
// GET /google-callback?code=…&state=… — the one place a Google refresh token exists in our code.
//
// It goes straight into Supabase Vault and the row keeps only the secret's id. It is never
// returned, never logged, and never sent to the device (D12): the device's whole knowledge of the
// connection is "connected: true".
//
// This is the one C2 function with no bearer token at all — Google redirects a browser here — so
// it is authenticated by the single-use `state` nonce, which `take_google_state` consumes and
// expires in one statement. `config.toml` therefore sets `verify_jwt = false` here for a different
// reason than everywhere else, and hand-off H7 says so.
export interface CallbackDeps {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  /** Consumes the nonce and returns the account it was issued for, or null. Single use. */
  takeState(state: string): Promise<string | null>;
  exchange(code: string): Promise<{ refresh_token: string; access_token: string; sub: string; email?: string; scopes: string[] }>;
  /** `scopes` is what Google ACTUALLY granted, from the exchange's own `scope` field — never what
   *  was asked for. `/ingest-calendar` and `gmail-read` both check it before they read anything. */
  storeRefreshToken(
    accountId: string, sub: string, email: string | undefined, refreshToken: string, scopes: string[],
  ): Promise<void>;
}

function page(message: string, status = 200): Response {
  return new Response(
    `<!doctype html><meta charset="utf-8"><title>Knowlu</title>` +
      `<body style="font:16px system-ui;padding:3rem"><p>${message}</p></body>`,
    { status, headers: { "content-type": "text/html; charset=utf-8" } },
  );
}

export function callbackHandler(deps: CallbackDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    const state = url.searchParams.get("state") ?? "";
    const code = url.searchParams.get("code") ?? "";
    if (url.searchParams.get("error") !== null) {
      return page("Gmail was <b>not connected</b>. You can close this window and try again from Knowlu.");
    }
    const accountId = state === "" ? null : await deps.takeState(state);
    if (accountId === null || code === "") {
      return page("That link has expired. Start again from Knowlu.", 400);
    }
    try {
      const tokens = await deps.exchange(code);
      await deps.storeRefreshToken(accountId, tokens.sub, tokens.email, tokens.refresh_token, tokens.scopes);
    } catch (e) {
      // The class only. A token-exchange error body can echo the code and, on some failures, the
      // client secret's prefix.
      console.error(`google-callback: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return page("Gmail could not be connected just now. You can close this window and try again from Knowlu.");
    }
    return page("Gmail is connected. You can <b>close this window</b> — Knowlu will read it at your next slot.");
  };
}
```

- [ ] **Step 6: Write the two `index.ts` files.**

```ts
// cloud/supabase/functions/google-connect/index.ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { connectHandler } from "./handler.ts";

const REVOKE = "https://oauth2.googleapis.com/revoke";

Deno.serve(connectHandler(requireActiveEntitlement, {
  clientId: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
  redirectUri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
  async saveState(accountId) {
    const nonce = crypto.randomUUID();
    await sharedDb().insert("google_state", { nonce, account_id: accountId }, false);
    return nonce;
  },
  async grantedScopes(accountId) {
    const rows = await sharedDb().select(
      `google_accounts?account_id=eq.${accountId}&status=neq.revoked&select=scopes`,
    ) as Array<{ scopes: string[] }>;
    return rows[0]?.scopes ?? [];
  },
  async disconnect(accountId) {
    const db = sharedDb();

    // Revoke FIRST: a row deleted before the revoke is a live grant nobody can find any more.
    // Revoking the refresh token at Google revokes EVERY scope on it at once, which is what
    // "disconnect" means on this panel: the calendar and the mailbox go together, because they are
    // one grant. A future "disconnect Gmail only" would be a re-consent for the calendar alone,
    // not a partial revoke — Google has no such thing.
    const secret = await db.rpc("read_google_grant_any", { p_account: accountId });
    if (typeof secret === "string" && secret !== "") {
      await fetch(REVOKE, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({ token: secret }),
      });
    }
    await db.rpc("delete_google_grant", { p_account: accountId });
  },
}));
```

`vault_decrypted_secrets` is a view over `vault.decrypted_secrets` exposed in `public` by the same migration, and `delete_google_grant(p_account uuid)` is a `security definer` function there that deletes the Vault secret and the `google_accounts` row in one statement — both are added to `20260911000200_google.sql` in step 7, because a Vault secret cannot be reached through PostgREST otherwise and because deleting the row without the secret would orphan it.

```ts
// cloud/supabase/functions/google-callback/index.ts
import { sharedDb } from "../_shared/judge_deps.ts";
import { callbackHandler } from "./handler.ts";

const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";

Deno.serve(callbackHandler({
  clientId: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
  clientSecret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
  redirectUri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
  async takeState(state) {
    const got = await sharedDb().rpc("take_google_state", { p_nonce: state });
    return typeof got === "string" && got !== "" ? got : null;
  },
  async exchange(code) {
    const response = await fetch(TOKEN_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        code,
        client_id: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
        client_secret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
        redirect_uri: `${Deno.env.get("SUPABASE_URL") ?? ""}/functions/v1/google-callback`,
        grant_type: "authorization_code",
      }),
    });
    if (!response.ok) throw new Error(`token exchange ${response.status}`);
    const body = await response.json() as {
      refresh_token?: string;
      access_token: string;
      id_token: string;
      scope?: string;
    };
    if (body.refresh_token === undefined) throw new Error("no refresh_token in the exchange");
    // `sub` and `email` out of the id_token's payload. Not verified cryptographically here: the
    // token came from a TLS connection to Google's own endpoint in response to our own code, which
    // is the same trust the access token itself rests on.
    const claims = JSON.parse(atob(body.id_token.split(".")[1].replace(/-/g, "+").replace(/_/g, "/"))) as {
      sub: string;
      email?: string;
    };
    // `scope` is space-separated and is what was ACTUALLY granted — a student can untick one on
    // the consent screen, and on an incremental ask Google returns only the new one.
    return {
      refresh_token: body.refresh_token,
      access_token: body.access_token,
      sub: claims.sub,
      email: claims.email,
      scopes: (body.scope ?? "").split(" ").filter((s) => s !== ""),
    };
  },
  async storeRefreshToken(accountId, sub, email, refreshToken, scopes) {
    const id = await sharedDb().rpc("store_google_grant", {
      p_account: accountId, p_sub: sub, p_email: email ?? null, p_token: refreshToken, p_scopes: scopes,
    });
    if (typeof id !== "string") throw new Error("the grant was not stored");
  },
}));
```

- [ ] **Step 7: Add the three Vault helpers to `20260911000200_google.sql`.** They are `security definer` because `vault.create_secret` is not reachable by an ordinary role, and each pins `search_path` so the definer's privileges cannot be redirected:

```sql
-- The Vault is not reachable through PostgREST, so these three are the whole interface to it.
-- `security definer` with a pinned `search_path`, and `revoke execute … from public` so only the
-- service role can call them.
create or replace function store_google_grant(
  p_account uuid, p_sub text, p_email text, p_token text, p_scopes text[]
)
returns uuid
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid uuid;
begin
  select vault.create_secret(p_token, 'google:' || p_account::text, 'Google refresh token (read-only scopes)')
    into sid;
  -- `scopes` is a UNION on conflict, not a replacement: `gmail.readonly` is asked for
  -- incrementally and Google returns only the newly granted scope on that exchange, so
  -- overwriting would silently forget the calendar the student connected in the wizard (§11a).
  insert into google_accounts (account_id, google_sub, secret_id, email_hint, scopes, status)
       values (p_account, p_sub, sid, p_email, p_scopes, 'active')
  on conflict (account_id) do update
       set google_sub = excluded.google_sub, secret_id = excluded.secret_id,
           email_hint = excluded.email_hint, status = 'active', connected_at = now(),
           scopes = (
             select array_agg(distinct s)
               from unnest(google_accounts.scopes || excluded.scopes) as s
           );
  return sid;
end;
$$;

create or replace function delete_google_grant(p_account uuid)
returns void
language plpgsql
security definer
set search_path = public, vault, extensions
as $$
declare
  sid uuid;
begin
  select secret_id into sid from google_accounts where account_id = p_account;
  delete from google_accounts where account_id = p_account;
  if sid is not null then
    delete from vault.secrets where id = sid;
  end if;
end;
$$;

-- `p_scope` is checked here rather than by the caller: the grant may carry the calendar scope,
-- the Gmail scope, or both, and a reader that assumed would read a mailbox the student never
-- offered. `/ingest-calendar` passes the calendar scope; `gmail-read` passes the Gmail one.
create or replace function read_google_grant(p_account uuid, p_scope text)
returns text
language sql
security definer
set search_path = public, vault, extensions
as $$
  select s.decrypted_secret
    from google_accounts g
    join vault.decrypted_secrets s on s.id = g.secret_id
   where g.account_id = p_account
     and g.status <> 'revoked'
     and p_scope = any (g.scopes);
$$;

-- Disconnect needs the token whatever scopes it carries, so it gets its own reader rather than
-- passing a scope it does not care about.
create or replace function read_google_grant_any(p_account uuid)
returns text
language sql
security definer
set search_path = public, vault, extensions
as $$
  select s.decrypted_secret
    from google_accounts g
    join vault.decrypted_secrets s on s.id = g.secret_id
   where g.account_id = p_account;
$$;

revoke execute on function store_google_grant(uuid, text, text, text, text[]) from public;
revoke execute on function delete_google_grant(uuid) from public;
revoke execute on function read_google_grant(uuid, text) from public;
revoke execute on function read_google_grant_any(uuid) from public;
```

`google-connect/index.ts`'s `disconnect` therefore calls `read_google_grant_any` for the token, revokes it at Google, then calls `delete_google_grant`. **Revoking a refresh token revokes every scope on it** — the calendar and the mailbox go together, because they are one grant; Google has no partial revoke, and a future "disconnect Gmail only" would be a re-consent for the calendar alone. Say that in the settings copy when C1 or C4 writes it.

- [ ] **Step 8: Add the scope-order test**, in `google-connect/handler_test.ts`, and make it the one a reviewer looks for:

```ts
Deno.test("calendar_is_asked_for_before_gmail_and_never_together", async () => {
  // §11a in one assertion: the wizard's first connection step is the calendars, the calendar
  // scope is *sensitive* (lighter review, no CASA), and Gmail is restricted and optional. A
  // single consent asking for both would drag the calendar behind Gmail's verification and CASA
  // — which is the whole cost this ordering exists to avoid.
  const handler = connectHandler(OK, deps());
  const first = new URL((await (await handler(new Request("http://127.0.0.1/google-connect"))).json()).url);
  const second = new URL((await (await handler(new Request("http://127.0.0.1/google-connect?scope=gmail"))).json()).url);
  assertEquals(first.searchParams.get("scope"), CALENDAR_SCOPE);
  assertEquals(second.searchParams.get("scope"), GMAIL_SCOPE);
  for (const url of [first, second]) {
    assertEquals(url.searchParams.get("scope")!.split(" ").length, 1, "one scope per consent");
  }
  assertEquals(first.searchParams.get("include_granted_scopes"), "false");
  assertEquals(second.searchParams.get("include_granted_scopes"), "true");
});
```

- [ ] **Step 9: Run, apply, deploy, ask Quinn for P2, commit.**

```
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/google-connect/ cloud/supabase/functions/google-callback/
supabase db push --project-ref <staging ref>
supabase functions deploy google-connect --project-ref <staging ref>
supabase functions deploy google-callback --project-ref <staging ref>
```

Expected: `ok | 14 passed | 0 failed` from the two handler suites (10 + 4). Ask Quinn for P2 with the exact redirect URI to paste into the Google console (`https://<staging ref>.supabase.co/functions/v1/google-callback`), the two secret names for `supabase secrets set`, and the one sentence that matters: **the consent screen lists two scopes and the calendar one can be submitted and cleared first, on its own, with no CASA** (§11a). He sets them; this session never sees a value.

```bash
git add cloud/supabase/migrations/20260911000200_google.sql cloud/supabase/functions/google-connect/ cloud/supabase/functions/google-callback/
git commit -F .git-commit-msg.txt   # "cloud: Gmail OAuth — gmail.readonly only, the refresh token in Vault and never on the device (C2 Task 10)"
```


---

### Task 11: `/judge-email` and `/gmail-read` — the five tiers, and the pull

**Precondition P3** (Quinn as the first named Gmail test user).

**Files:**
- Create: `cloud/supabase/functions/judge-email/index.ts`, `cloud/supabase/functions/gmail-read/{handler.ts,handler_test.ts,index.ts}`
- Test: new tests in `engine/src/enrich.rs`
- Modify: `engine/src/enrich.rs` (`pull_gmail` and the two note writers), `engine/src/cloudmodel.rs` (`impl judge::EmailModel`, `email_request`, `pull_gmail_queue`)

**Interfaces:**
- Produces:
  - `POST /judge-email` → the `JudgeReply` shape with the five-tier verdict.
  - `POST /gmail-read` `{ ack: string[] }` → `{ items: Array<{ uid, tier, payload }>, read: number, quiet: boolean, more: boolean }`.
  - `cloudmodel::pull_gmail_queue(client, ack: &[String]) -> Result<(Vec<GmailItem>, bool), CloudError>` — the bool is `more`.
  - `struct GmailItem { uid, tier, title, course, due, effort_hours, importance, why, confidence }`.
  - `enrich::GMAIL_ACTOR: &str = "agent:knowlu.gmail"`, `enrich::pull_gmail(vault, client, opts) -> Vec<String>`.

**The five tiers, and what each becomes on the device** (§5.3):

| Tier | On the device |
|---|---|
| `task` | a new note in `tasks/` through `write::create`, `created_by: gmail`, `source_uid: gmail:<message-id>`, the verdict's fields, `needs_enrichment: false` |
| `borderline` | a `kind: task` approval card carrying the note in a ```` ```task ```` fence — `approvals::materialize` turns it into that same note on approval |
| `event` | a `kind: task` approval card, same shape, whose payload names the stated date and time |
| `opportunity` | a `kind: task` approval card, same shape |
| `information` | **dropped**, and the uid recorded in `state/ingest-seen.md` so it is never asked about again |

No new approval kind, so `approvals.rs` needs no hand-off for this task, and the 15-a-day proposal cap applies to these cards exactly as it applies to every other (`defer_over_budget`).

**Three things this task must get right that the earlier draft did not.**

1. **`gmail-read` cannot judge sixty messages serially inside an edge function's wall clock.** Each judgment is a model call bounded at 120 seconds, and the platform's own limit is far under sixty of those. So the handler carries a **wall-clock budget of its own** (`READ_BUDGET_MS`) and returns `more: true` when it stops early; the device calls again, up to `PULL_ROUNDS` times, inside the same slot. The dedup set is the resume cursor — there is no other state to keep.
2. **`known_courses` has to come from somewhere.** On the task path the device supplies it and re-checks the answer itself (`judge_task`'s `knows_course`). On the Gmail path there is no device-side `judge_task`, so the server's list is the only check — and nothing in C2 gives the server a course list. It is derived from the account's own history: the distinct non-null `course` values in `judgments.fields` over the last 120 days. A brand-new account has none, every `course` comes back `null`, and the first coursework sync fixes it — which is the right failure.
3. **A revoked grant has to reach the student.** `markRevoked` is server-side and `quiet: true` is a JSON field; without a line on stdout a student's mail simply stops arriving for six days until somebody looks.

- [ ] **Step 1: Write the failing server test** — `cloud/supabase/functions/gmail-read/handler_test.ts`. The whole of `ReadDeps` is fakes, so this needs no project, no Google and no key.

```ts
import { assert, assertEquals } from "@std/assert";
import { ScriptedModel } from "../_shared/judge_anthropic.ts";
import type { JudgmentRow } from "../_shared/judge_pipeline.ts";
import { type GmailApi, READ_CAP, readHandler, type ReadDeps } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const TRIPWIRE = "TRIPWIRE-9f2c";

const TASK_ANSWER = {
  tier: "task", title: "PH 106 problem set 4", course: "ph-106", due: "2026-09-11",
  effort_hours: 2.5, importance: 4, why: "the email states a Friday deadline", confidence: 0.86,
};

function fakes(replies: Array<Record<string, unknown>>, ids = ["m1"]) {
  const rows: JudgmentRow[] = [];
  const queued: Array<{ uid: string; tier: string; payload: Record<string, unknown> }> = [];
  const delivered: string[] = [];
  const seen = new Set<string>();
  const asked: string[] = [];
  let attachments = 0;
  const api: GmailApi = {
    list: (_t, q) => {
      asked.push(q);
      return Promise.resolve(ids);
    },
    message: (_t, id) => {
      if (id === "attachment") attachments += 1;
      return Promise.resolve({
        subject: "PH 106 problem set 4 is posted",
        from: "noreply@lms.example.invalid",
        date: "Wed, 09 Sep 2026 08:00:00 -0500",
        text: `Problem set 4 is due Friday. ${TRIPWIRE}`,
      });
    },
  };
  const deps: ReadDeps = {
    api,
    accessTokenFor: () => Promise.resolve("access-token-not-a-secret"),
    excludedLabels: () => Promise.resolve([]),
    markRevoked: () => Promise.resolve(),
    seen: () => Promise.resolve(seen),
    markSeen: (_a, uid) => {
      seen.add(uid);
      return Promise.resolve();
    },
    enqueue: (_a, uid, tier, payload) => {
      queued.push({ uid, tier, payload });
      return Promise.resolve();
    },
    undelivered: () => Promise.resolve(queued.filter((q) => !delivered.includes(q.uid))),
    deliver: (_a, uids) => {
      delivered.push(...uids);
      return Promise.resolve();
    },
    knownCourses: () => Promise.resolve(["ph-106"]),
    pipeline: () =>
      Promise.resolve({
        row: {
          kind: "email", provider: "anthropic", model_id: "claude-haiku-4-5",
          prompt_version: "email-1", grammar_version: "email-1", max_tokens: 640,
          sampling: { temperature: 0 }, usd_per_m_in: 1.0, usd_per_m_out: 5.0,
        },
        model: new ScriptedModel(replies),
        rules: { lookup: () => Promise.resolve(null) },
        caps: {
          charge: () => Promise.resolve(true),
          withinBudget: () => Promise.resolve(true),
          recordTokens: () => Promise.resolve(),
        },
        log: {
          write: (row: JudgmentRow) => {
            rows.push(row);
            return Promise.resolve(`judgment-${rows.length}`);
          },
        },
        origin: "gmail_api",
        now: () => 0,
      }),
    budgetMs: 60_000,
    clock: () => 0,
  };
  return { deps, rows, queued, seen, asked, attachmentsRead: () => attachments };
}

function post(body: unknown = {}): Request {
  return new Request("http://127.0.0.1/gmail-read", { method: "POST", body: JSON.stringify(body) });
}

Deno.test("the message text reaches no queue row and no judgment row", async () => {
  const { deps, rows, queued } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(queued.length, 1);
  assert(!JSON.stringify(queued).includes(TRIPWIRE), "the message body reached the queue");
  assert(!JSON.stringify(rows).includes(TRIPWIRE), "the message body reached a judgment row");
  assert(!JSON.stringify(rows).includes("PH 106 problem set 4 is posted"), "the subject reached a judgment row");
});

Deno.test("every derived row is flagged gmail_api, which is what the export filter keys on", async () => {
  const { deps, rows } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(rows.length, 1);
  assertEquals(rows[0].origin, "gmail_api");
  assertEquals(rows[0].item_id, "gmail:m1");
});

Deno.test("each of the five tiers becomes a queue row with that tier", async () => {
  for (const tier of ["task", "borderline", "event", "opportunity", "information"]) {
    const { deps, queued } = fakes([{ ...TASK_ANSWER, tier }]);
    await readHandler(OK, deps)(post());
    assertEquals(queued[0].tier, tier);
  }
  // `information` is queued too — the device records the uid and stops asking — but it carries no
  // task fields to write.
  const { deps, queued } = fakes([{
    tier: "information", title: "Weekly newsletter", course: null, due: null,
    effort_hours: null, importance: null, why: "a newsletter", confidence: 0.95,
  }]);
  await readHandler(OK, deps)(post());
  assertEquals(queued[0].payload.effort_hours, null);
});

Deno.test("a uid already seen is neither fetched nor judged", async () => {
  const { deps, rows, seen } = fakes([]);
  seen.add("gmail:m1");
  const reply = await (await readHandler(OK, deps)(post())).json();
  assertEquals(reply.read, 0);
  assertEquals(rows.length, 0);
});

Deno.test("an acknowledged row is delivered and never returned twice", async () => {
  const { deps } = fakes([TASK_ANSWER]);
  const first = await (await readHandler(OK, deps)(post())).json();
  assertEquals(first.items.length, 1);
  const second = await (await readHandler(OK, deps)(post({ ack: ["gmail:m1"] }))).json();
  assertEquals(second.items.length, 0);
});

Deno.test("attachments are never fetched, because there is no code path that could", async () => {
  const { deps, attachmentsRead } = fakes([TASK_ANSWER]);
  await readHandler(OK, deps)(post());
  assertEquals(attachmentsRead(), 0);
  // And structurally: the handler's whole Gmail surface is `list` and `message`. A third method
  // would have to be added to `GmailApi` before an attachment could be reached.
  const source = await Deno.readTextFile(new URL("./handler.ts", import.meta.url));
  assertEquals(source.includes("attachment"), false, "cloud design §5.3: attachments are never fetched");
});

Deno.test("the read stops at its wall-clock budget and says there is more", async () => {
  // An edge function's wall clock is far under sixty model calls at 120 s each. The dedup set is
  // the resume cursor: everything judged is marked seen, so the next round starts where this one
  // stopped and nothing is judged twice.
  let tick = 0;
  const { deps, queued } = fakes(
    Array.from({ length: 5 }, () => TASK_ANSWER),
    ["m1", "m2", "m3", "m4", "m5"],
  );
  const reply = await (await readHandler(OK, { ...deps, budgetMs: 10, clock: () => (tick += 8) })(post())).json();
  assert(reply.more === true, "the handler must say it stopped early");
  assert(queued.length < 5, "it must actually have stopped early");
});

Deno.test("excluded_labels_become_negative_label_terms (and the window is seven days)", async () => {
  const { deps, asked } = fakes([TASK_ANSWER]);
  await readHandler(OK, { ...deps, excludedLabels: () => Promise.resolve(["Promotions", "Social"]) })(post());
  assertEquals(asked[0], "newer_than:7d -label:Promotions -label:Social");
  assertEquals(READ_CAP, 60);
});

Deno.test("a revoked grant is quiet, not an error, and never a failed slot", async () => {
  const { deps } = fakes([]);
  let revoked = false;
  const response = await readHandler(OK, {
    ...deps,
    accessTokenFor: () => Promise.resolve(null),
    markRevoked: () => {
      revoked = true;
      return Promise.resolve();
    },
  })(post());
  assertEquals(response.status, 200);
  assertEquals((await response.json()).quiet, true);
  assertEquals(revoked, true);
});

Deno.test("gmail_rows_are_excluded_from_the_training_export", async () => {
  // A static test over the function's own text: the filter is one predicate and losing it is
  // silent, so it is pinned where it cannot be lost by an edit that looks like a refactor.
  const sql = await Deno.readTextFile(new URL("../../migrations/20260911000200_google.sql", import.meta.url));
  const body = sql.slice(sql.indexOf("create or replace function export_training_rows"));
  assert(
    body.includes("origin <> 'gmail_api'"),
    "cloud design §5.3 and §9: gmail-derived rows are excluded by the export filter",
  );
});
```

- [ ] **Step 2: Run it and watch it fail.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/functions/gmail-read/` → module not found.

- [ ] **Step 3: Write `gmail-read/handler.ts`.**

```ts
// POST /gmail-read — the twice-daily read, aligned to the student's slots because the device asks
// for it inside its own slot (cloud design §5.3). Delivery is a pull; nothing is pushed.
//
// The order matters:
//   1. acknowledge what the LAST pull actually wrote (so a crash between reply and write costs a
//      repeat, never a lost task);
//   2. get an access token from the Vault-held refresh token — a revoked grant is `quiet`, never
//      an error;
//   3. list message ids from the last 7 days across the mailbox, minus the account's excluded
//      labels (the routine's "Crimson label and the personal inbox" rule, generalised);
//   4. skip every `gmail:<message-id>` already in `gmail_seen`;
//   5. fetch each remaining message's HEADERS and TEXT PART — never an attachment, ever;
//   6. judge it through the one pipeline with origin 'gmail_api';
//   7. queue the verdict, mark the uid seen, and DISCARD the text;
//   8. stop at the wall-clock budget and say `more`, because an edge function cannot run sixty
//      120-second model calls; the dedup set is the resume cursor.
//
// The message text exists in this function's memory for the duration of one judgment and nowhere
// else: not in `gmail_queue`, not in `judgments`, not in a log line.
import { judge, type JudgeReply, type PipelineDeps } from "../_shared/judge_pipeline.ts";
import type { Entitle } from "../_shared/judge_handler.ts";

export const WINDOW = "newer_than:7d";
/** A bound on one read, so a mailbox with a thousand unread messages cannot eat a slot. */
export const READ_CAP = 60;
/** A bound on one INVOCATION, well under any edge-function wall clock. */
export const READ_BUDGET_MS = 40_000;

export interface GmailApi {
  /** Message ids only. `q` is Gmail's own query language. */
  list(accessToken: string, q: string): Promise<string[]>;
  /** Headers plus the first `text/plain` part, decoded. **Never an attachment.** */
  message(accessToken: string, id: string): Promise<{ subject: string; from: string; date: string; text: string }>;
}

export interface ReadDeps {
  api: GmailApi;
  /** A fresh access token from the Vault-held refresh token, or null when the grant is gone. */
  accessTokenFor(accountId: string): Promise<string | null>;
  excludedLabels(accountId: string): Promise<string[]>;
  markRevoked(accountId: string): Promise<void>;
  seen(accountId: string): Promise<Set<string>>;
  markSeen(accountId: string, uid: string): Promise<void>;
  enqueue(
    accountId: string,
    uid: string,
    tier: string,
    payload: Record<string, unknown>,
    judgmentId: string | null,
  ): Promise<void>;
  undelivered(accountId: string): Promise<Array<{ uid: string; tier: string; payload: Record<string, unknown> }>>;
  deliver(accountId: string, uids: string[]): Promise<void>;
  knownCourses(accountId: string): Promise<string[]>;
  pipeline(): Promise<PipelineDeps>;
  budgetMs: number;
  clock: () => number;
}

export function query(excluded: string[]): string {
  // A label with a space is quoted the way Gmail's own search does it.
  const terms = excluded.map((l) => `-label:${l.includes(" ") ? `"${l}"` : l}`);
  return [WINDOW, ...terms].join(" ");
}

export function readHandler(entitle: Entitle, deps: ReadDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    if (req.method !== "POST") return Response.json({ error: "POST only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const body = await req.json().catch(() => ({})) as { ack?: unknown };
      const ack = Array.isArray(body.ack) ? body.ack.filter((u): u is string => typeof u === "string") : [];
      if (ack.length > 0) await deps.deliver(account_id, ack);

      const token = await deps.accessTokenFor(account_id);
      if (token === null) {
        // A revoked or expired grant is not an error the slot should fail on — while the Google
        // project is in Testing the token dies every 7 days by design (§5.3, §9). The DEVICE turns
        // `quiet` into a line the student can act on; here it is one status change.
        await deps.markRevoked(account_id);
        return Response.json({ items: await deps.undelivered(account_id), read: 0, quiet: true, more: false });
      }

      const already = await deps.seen(account_id);
      const known = await deps.knownCourses(account_id);
      const pipeline = await deps.pipeline();
      const started = deps.clock();
      let read = 0;
      let more = false;
      for (const id of (await deps.api.list(token, query(await deps.excludedLabels(account_id)))).slice(0, READ_CAP)) {
        const uid = `gmail:${id}`;
        if (already.has(uid)) continue;
        if (deps.clock() - started >= deps.budgetMs) {
          more = true;
          break;
        }
        const message = await deps.api.message(token, id);
        const reply: JudgeReply = await judge(account_id, {
          kind: "email",
          item: { message_id: uid, subject: message.subject, from: message.from, date: message.date, text: message.text },
          heuristics_seed: { known_courses: known },
        }, pipeline);
        // The text is out of scope from here: nothing below this line can reach it.
        const verdict = reply.verdict ?? { tier: "information", why: "not judged", confidence: 0 };
        const tier = typeof verdict.tier === "string" ? verdict.tier : "information";
        await deps.enqueue(account_id, uid, tier, verdict, reply.judgment_id ?? null);
        await deps.markSeen(account_id, uid);
        read += 1;
      }
      return Response.json({ items: await deps.undelivered(account_id), read, quiet: false, more });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`gmail-read: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "read failed" }, { status: 500 });
    }
  };
}
```

- [ ] **Step 4: Write `gmail-read/index.ts` and `judge-email/index.ts`.**

```ts
// cloud/supabase/functions/gmail-read/index.ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps, sharedDb } from "../_shared/judge_deps.ts";
import { type GmailApi, READ_BUDGET_MS, readHandler } from "./handler.ts";

const GMAIL = "https://gmail.googleapis.com/gmail/v1/users/me";
const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";

/** The first `text/plain` part, base64url-decoded. Walks nested `multipart/*` and stops at text. */
function firstTextPart(part: Record<string, unknown> | undefined): string {
  if (part === undefined) return "";
  const mime = typeof part.mimeType === "string" ? part.mimeType : "";
  const body = part.body as { data?: string } | undefined;
  if (mime === "text/plain" && typeof body?.data === "string") {
    const b64 = body.data.replace(/-/g, "+").replace(/_/g, "/");
    return new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  }
  for (const child of (part.parts as Array<Record<string, unknown>> | undefined) ?? []) {
    const found = firstTextPart(child);
    if (found !== "") return found;
  }
  return "";
}

// There is no `attachments` method here and no call to `.../attachments/`. Adding one would mean
// adding a method to `GmailApi`, which the handler test forbids by scanning the handler's source.
const api: GmailApi = {
  async list(accessToken, q) {
    const url = `${GMAIL}/messages?q=${encodeURIComponent(q)}&maxResults=100`;
    const response = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
    if (!response.ok) throw new Error(`gmail list ${response.status}`);
    const body = await response.json() as { messages?: Array<{ id: string }> };
    return (body.messages ?? []).map((m) => m.id);
  },
  async message(accessToken, id) {
    const response = await fetch(`${GMAIL}/messages/${id}?format=full`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!response.ok) throw new Error(`gmail message ${response.status}`);
    const body = await response.json() as { payload?: Record<string, unknown> };
    const headers = (body.payload?.headers as Array<{ name: string; value: string }> | undefined) ?? [];
    const header = (name: string) =>
      headers.find((h) => h.name.toLowerCase() === name)?.value ?? "";
    return {
      subject: header("subject"),
      from: header("from"),
      date: header("date"),
      text: firstTextPart(body.payload),
    };
  },
};

Deno.serve(readHandler(requireActiveEntitlement, {
  api,
  async accessTokenFor(accountId) {
    // The Gmail scope specifically: a grant that carries only `calendar.readonly` must read no
    // mail, and `read_google_grant` refuses rather than this function remembering to check.
    const refresh = await sharedDb().rpc("read_google_grant", {
      p_account: accountId,
      p_scope: "https://www.googleapis.com/auth/gmail.readonly",
    });
    if (typeof refresh !== "string" || refresh === "") return null;
    const response = await fetch(TOKEN_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        refresh_token: refresh,
        client_id: Deno.env.get("GOOGLE_CLIENT_ID") ?? "",
        client_secret: Deno.env.get("GOOGLE_CLIENT_SECRET") ?? "",
        grant_type: "refresh_token",
      }),
    });
    if (!response.ok) return null;
    const body = await response.json() as { access_token?: string };
    return body.access_token ?? null;
  },
  async excludedLabels(accountId) {
    const rows = await sharedDb().select(
      `google_accounts?account_id=eq.${accountId}&select=excluded_labels`,
    ) as Array<{ excluded_labels: string[] }>;
    return rows[0]?.excluded_labels ?? [];
  },
  async markRevoked(accountId) {
    await sharedDb().update(`google_accounts?account_id=eq.${accountId}`, { status: "revoked" });
  },
  async seen(accountId) {
    const rows = await sharedDb().select(
      `gmail_seen?account_id=eq.${accountId}&select=uid`,
    ) as Array<{ uid: string }>;
    return new Set(rows.map((r) => r.uid));
  },
  async markSeen(accountId, uid) {
    await sharedDb().insert("gmail_seen", { account_id: accountId, uid }, false);
  },
  async enqueue(accountId, uid, tier, payload, judgmentId) {
    await sharedDb().insert(
      "gmail_queue",
      { account_id: accountId, uid, tier, payload, judgment_id: judgmentId },
      false,
    );
  },
  async undelivered(accountId) {
    return await sharedDb().select(
      `gmail_queue?account_id=eq.${accountId}&delivered_at=is.null&select=uid,tier,payload&order=queued_at`,
    ) as Array<{ uid: string; tier: string; payload: Record<string, unknown> }>;
  },
  async deliver(accountId, uids) {
    const list = uids.map((u) => `"${u}"`).join(",");
    await sharedDb().update(
      `gmail_queue?account_id=eq.${accountId}&uid=in.(${list})&delivered_at=is.null`,
      { delivered_at: new Date().toISOString() },
    );
  },
  async knownCourses(accountId) {
    // Derived from the account's own history — there is no course table server-side, and inventing
    // one would mean the device syncing its `courses/` folder, which is C3's problem. A new
    // account has none, every `course` comes back null, and the first coursework sync fixes it.
    const since = new Date(Date.now() - 120 * 86_400_000).toISOString();
    const rows = await sharedDb().select(
      `judgments?account_id=eq.${accountId}&judged_at=gte.${since}&select=fields`,
    ) as Array<{ fields: Record<string, string> }>;
    const out = new Set<string>();
    for (const row of rows) {
      const course = row.fields?.course;
      if (typeof course === "string" && course !== "" && course !== "null") out.add(course);
    }
    return [...out];
  },
  pipeline: () => liveDeps("email", "gmail_api"),
  budgetMs: READ_BUDGET_MS,
  clock: () => Date.now(),
}));
```

```ts
// cloud/supabase/functions/judge-email/index.ts
//
// `origin: "device"` here, not "gmail_api": this endpoint is the one a NON-Gmail caller uses (the
// eval harness's parity check, and §13's forwarding fallback if it is ever built). The Gmail path
// judges inside `gmail-read`, where the origin is `gmail_api` and the export filter keys on it.
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { liveDeps } from "../_shared/judge_deps.ts";
import { judgeHandler } from "../_shared/judge_handler.ts";

Deno.serve(judgeHandler("email", requireActiveEntitlement, (kind) => liveDeps(kind, "device")));
```

- [ ] **Step 5: Write the device side in `engine/src/cloudmodel.rs`.**

```rust
/// One judged message the service is holding for this device.
#[derive(Debug, Clone, PartialEq)]
pub struct GmailItem {
    pub uid: String,
    pub tier: String,
    pub title: String,
    pub course: Option<String>,
    pub due: Option<String>,
    pub effort_hours: Option<f64>,
    pub importance: Option<i64>,
    pub why: String,
    pub confidence: f64,
}

/// The body of `POST /judge-email`. Used by the eval harness's parity check and by §13's
/// forwarding fallback; **not by the Gmail path**, which judges server-side because Gmail message
/// text must never reach the device (D12).
pub fn email_request(item: &judge::EmailItem) -> Value {
    json!({
        "kind": "email",
        "item": {
            "message_id": item.message_id,
            "subject": judge::one_line(&item.subject, 200),
            "from": judge::one_line(&item.from, 200),
            "date": item.date,
            "text": judge::clip(item.text.trim(), judge::MAX_BODY_CHARS),
        },
        "heuristics_seed": { "known_courses": item.known_courses }
    })
}

impl judge::EmailModel for CloudModel<'_> {
    fn judge_email(&self, item: &judge::EmailItem) -> Result<judge::EmailVerdict, ModelError> {
        let reply = self.call("/judge-email", &email_request(item))?;
        let verdict = reply.get("verdict").filter(|v| !v.is_null()).ok_or_else(|| {
            let cause = reply.get("cause").and_then(Value::as_str).unwrap_or("no verdict");
            ModelError::Failed(format!("the judgment service answered {cause}"))
        })?;
        let text = |key: &str| verdict.get(key).and_then(Value::as_str).map(str::to_string);
        Ok(judge::EmailVerdict {
            tier: text("tier").unwrap_or_else(|| "information".to_string()),
            title: judge::one_line(&text("title").unwrap_or_default(), 200),
            course: text("course"),
            due: text("due"),
            effort_hours: verdict.get("effort_hours").and_then(Value::as_f64).map(|e| e.clamp(0.25, 40.0)),
            importance: verdict.get("importance").and_then(Value::as_i64).map(|i| i.clamp(1, 5)),
            why: judge::one_line(&text("why").unwrap_or_default(), 140),
            confidence: verdict.get("confidence").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0),
        })
    }
}

/// Pull what the service has queued, acknowledging what the last pull wrote.
///
/// **A pull, never a push** — the device asks inside its own slot, so a laptop that is off for a
/// week simply asks later and gets everything. `ack` is the previous pull's uids: a row is only
/// marked delivered once the device has actually written it, so a crash between the reply and the
/// write costs a repeat, not a lost task. The `bool` is the service's `more`: it stopped at its own
/// wall-clock budget and the device should ask again in this same slot.
pub fn pull_gmail_queue(
    client: &CloudClient,
    ack: &[String],
) -> Result<(Vec<GmailItem>, bool), CloudError> {
    let reply = client.post("/gmail-read", &json!({ "ack": ack }))?;
    if reply.get("quiet").and_then(Value::as_bool).unwrap_or(false) {
        return Err(CloudError::Status {
            code: 200,
            detail: "gmail is not connected; re-connect from settings".to_string(),
        });
    }
    let more = reply.get("more").and_then(Value::as_bool).unwrap_or(false);
    let mut out = Vec::new();
    for row in reply.get("items").and_then(Value::as_array).into_iter().flatten() {
        let p = row.get("payload").unwrap_or(&Value::Null);
        let text = |key: &str| p.get(key).and_then(Value::as_str).map(str::to_string);
        let Some(uid) = row.get("uid").and_then(Value::as_str) else { continue };
        out.push(GmailItem {
            uid: uid.to_string(),
            tier: row.get("tier").and_then(Value::as_str).unwrap_or("information").to_string(),
            title: judge::one_line(&text("title").unwrap_or_default(), 200),
            course: text("course"),
            due: text("due"),
            effort_hours: p.get("effort_hours").and_then(Value::as_f64).map(|e| e.clamp(0.25, 40.0)),
            importance: p.get("importance").and_then(Value::as_i64).map(|i| i.clamp(1, 5)),
            why: judge::one_line(&text("why").unwrap_or_default(), 140),
            confidence: p.get("confidence").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0),
        });
    }
    Ok((out, more))
}
```

`CloudError::label()` gains one arm so a quiet mailbox reads as itself rather than as "the service refused":

```rust
            CloudError::Status { code: 200, .. } => "gmail is not connected",
```

- [ ] **Step 6: Write `pull_gmail` in `engine/src/enrich.rs`.**

```rust
/// The agent actor for Gmail-derived writes. `agent:` prefix, so judge-once holds and a field the
/// student set comes back as a `kind: amend` card rather than being overwritten.
pub const GMAIL_ACTOR: &str = "agent:knowlu.gmail";

/// How many times one slot asks. The service stops at its own wall-clock budget and says `more`;
/// three rounds covers a 180-message backlog and still cannot hold the slot open.
pub const PULL_ROUNDS: usize = 3;

/// The template a `tier: task` message becomes. `needs_enrichment: false` because the service has
/// already judged effort and importance — flagging it would send it straight back for a second
/// judgment of the same thing.
const GMAIL_NOTE: &str = "---\ntitle: {title}\ncourse: {course}\ndomain: school\ndue: {due}\n\
effort_hours: {effort_hours}\neffort_confidence: low\neffort_source: inferred\n\
importance: {importance}\nimportance_reason: {why}\nstatus: active\nprogress: 0\n\
created_by: gmail\nsource_uid: {uid}\nneeds_enrichment: false\n---\n\n{body}\n";

/// Pull the service's queued Gmail judgments and write them, then acknowledge them.
///
/// Every write goes through `write` (journal first, single-line surgery second) under
/// [`GMAIL_ACTOR`]; a `tier: task` becomes a note, the three middle tiers become ordinary
/// proposals — so the 15-a-day cap applies to them exactly as it applies to every other card,
/// through `approvals::defer_over_budget` at the next `rank` — and `information` is dropped with
/// its uid recorded so it is never asked about again.
pub fn pull_gmail(
    vault: &Path,
    client: &crate::cloudmodel::CloudClient,
    opts: &Options<'_>,
) -> Vec<String> {
    let ctx = WriteContext {
        actor: GMAIL_ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let today = jiff::Zoned::now().date();
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let mut lines: Vec<String> = Vec::new();
    let (mut notes, mut cards, mut dropped) = (0usize, 0usize, 0usize);
    let mut ack: Vec<String> = Vec::new();

    for round in 0..PULL_ROUNDS {
        let (items, more) = match crate::cloudmodel::pull_gmail_queue(client, &ack) {
            Ok(got) => got,
            Err(e) => {
                lines.push(format!("gmail: skipped ({e})"));
                return lines;
            }
        };
        ack.clear();
        if items.is_empty() {
            if round == 0 {
                return lines;
            }
            break;
        }
        // Re-read per round: the previous round's `record_seen` calls are in it.
        let seen = crate::ingest::load_seen(vault);
        for item in &items {
            // The uid is the same `gmail:<message-id>` the server deduplicates on, so a message is
            // written once whichever side asked. A uid already here means an earlier slot wrote it
            // and the acknowledgement did not reach the server — acknowledge and move on.
            if seen.contains(&item.uid) {
                ack.push(item.uid.clone());
                continue;
            }
            let outcome = match item.tier.as_str() {
                // `information` is the noise tier: nothing is written, and the uid is recorded so
                // the service is never asked about that message again.
                "information" => {
                    dropped += 1;
                    Ok(String::new())
                }
                "task" => write_gmail_note(vault, item, &ctx, &mut journal).map(|stem| {
                    notes += 1;
                    format!("created {stem}")
                }),
                _ => write_gmail_card(vault, item, today, &ctx, &mut journal).map(|stem| {
                    cards += 1;
                    format!("proposed {stem}")
                }),
            };
            match outcome {
                Ok(note) => {
                    if !note.is_empty() {
                        lines.push(format!("gmail {}: {} ({})", item.uid, item.tier, note));
                    }
                    if let Err(e) = crate::ingest::record_seen(vault, &item.uid, &item.title, &stamp) {
                        lines.push(format!("gmail {}: seen ledger not written ({e})", item.uid));
                        continue;
                    }
                    ack.push(item.uid.clone());
                }
                // A single unwritable item must not end the batch, and it must NOT be
                // acknowledged: an unacknowledged row comes back next slot, which is the recovery.
                Err(e) => lines.push(format!("gmail {}: not written ({e})", item.uid)),
            }
        }
        if !more {
            break;
        }
    }

    if !ack.is_empty() {
        if let Err(e) = crate::cloudmodel::pull_gmail_queue(client, &ack) {
            lines.push(format!(
                "gmail: {} written but not acknowledged ({e}); they will come back next slot",
                ack.len()
            ));
        }
    }
    lines.push(format!(
        "gmail: {notes} task(s), {cards} proposed, {dropped} dropped as information"
    ));
    lines
}

/// A `tier: task` message as a note. Through `write::create`, so the journal record comes first.
fn write_gmail_note(
    vault: &Path,
    item: &crate::cloudmodel::GmailItem,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let text = gmail_note_text(item);
    let stem = crate::ingest::slugify(&item.title);
    let tasks = vault.join("tasks");
    std::fs::create_dir_all(&tasks).map_err(|e| e.to_string())?;
    let mut path = tasks.join(format!("{stem}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = tasks.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }
    let rel = crate::ids::rel(vault, &path);
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}

/// The frontmatter one item becomes. Every free-text field goes through `write::to_literal`, so a
/// colon or a quote in a subject line cannot produce frontmatter the loader silently drops.
fn gmail_note_text(item: &crate::cloudmodel::GmailItem) -> String {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let course = match item.course.as_deref().filter(|c| !c.is_empty()) {
        Some(c) => lit(c),
        None => "null".to_string(),
    };
    let due = match item.due.as_deref().filter(|d| !d.is_empty()) {
        Some(d) => d.to_string(),
        None => "null".to_string(),
    };
    let effort = write::to_literal(&Value::Number(serde_yaml_ng::Number::from(
        item.effort_hours.unwrap_or(1.0),
    )));
    GMAIL_NOTE
        .replace("{title}", &lit(&item.title))
        .replace("{course}", &course)
        .replace("{due}", &due)
        .replace("{effort_hours}", &effort)
        .replace("{importance}", &item.importance.unwrap_or(3).to_string())
        .replace("{why}", &lit(&item.why))
        .replace("{uid}", &lit(&item.uid))
        .replace("{body}", &format!("From email. {}", item.why))
}

/// The three middle tiers as a `kind: task` approval card. The payload is the same note text, in a
/// fenced `task` block, because that is exactly what `approvals::materialize` turns into a note
/// when the card is approved — so an approved card and a `tier: task` message produce the same
/// note, and there is one note writer rather than two.
///
/// The stem must start with `task-`: `materialize` strips that prefix to name the note.
fn write_gmail_card(
    vault: &Path,
    item: &crate::cloudmodel::GmailItem,
    today: jiff::civil::Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(14))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let stem = format!("task-{}", crate::ingest::slugify(&item.title));
    let text = format!(
        "---\ntype: approval\nkind: task\ntitle: {}\nstatus: pending\nproposed_at: {stamp}\n\
         first_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\ncreated_by: gmail\n\
         source_uid: {}\n---\n\n{}\n\n```task\n{}```\n",
        lit(&item.title),
        lit(&item.uid),
        item.why,
        gmail_note_text(item),
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| e.to_string())?;
    let mut path = approvals.join(format!("{stem}.md"));
    let mut suffix = 2;
    while path.exists() {
        path = approvals.join(format!("{stem}-{suffix}.md"));
        suffix += 1;
    }
    let rel = crate::ids::rel(vault, &path);
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}
```

and one more line in `run_lines_with`'s cloud arm, after the events pass:

```rust
    lines.extend(pull_gmail(vault, client, opts));
```

- [ ] **Step 7: Write the engine tests** — in `enrich.rs`'s test module, against a loopback listener scripted with one item per tier. Two of them are the ones that matter:

```rust
    /// An approved Gmail card must produce the SAME note a `tier: task` message produces, or there
    /// are two note writers and one of them will drift.
    ///
    /// **Compared without `id:`** — `write::create` mints a fresh opaque id into the frontmatter of
    /// every note it creates, so a byte-for-byte comparison would fail by construction. The id is
    /// the one line that is *supposed* to differ; everything else is the contract.
    #[test]
    fn an_approved_gmail_card_materialises_the_same_note_a_task_tier_would() {
        let vault = super::tests::scratch_vault("gmail-card");
        let item = crate::cloudmodel::GmailItem {
            uid: "gmail:m1".into(), tier: "borderline".into(),
            title: "PH 106 problem set 4".into(), course: Some("ph-106".into()),
            due: Some("2026-09-11".into()), effort_hours: Some(2.5), importance: Some(4),
            why: "the email states a Friday deadline".into(), confidence: 0.86,
        };
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);

        let direct = write_gmail_note(&vault, &item, &ctx, &mut journal).expect("the note writes");
        let card = write_gmail_card(&vault, &item, today, &ctx, &mut journal).expect("the card writes");
        // Approve it exactly as the deck would, then let `process_approvals` materialise it.
        let rel = format!("approvals/{card}.md");
        crate::write::write_literals(
            &vault, &rel, &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");
        let _ = crate::approvals::process_approvals(
            &vault, today, jiff::civil::date(2026, 9, 9).at(9, 0, 0, 0), &ctx, &mut journal,
        );

        let strip_id = |text: &str| {
            text.lines().filter(|l| !l.starts_with("id:")).collect::<Vec<_>>().join("\n")
        };
        let from_tier = std::fs::read_to_string(vault.join("tasks").join(format!("{direct}.md"))).unwrap();
        let materialised = crate::approvals::sorted_md(&vault.join("tasks"))
            .into_iter()
            .find(|p| p.file_stem().map(|s| s != direct.as_str()).unwrap_or(false))
            .expect("the card produced a note");
        let from_card = std::fs::read_to_string(&materialised).unwrap();
        assert_eq!(strip_id(&from_tier), strip_id(&from_card));
        let _ = std::fs::remove_dir_all(&vault);
    }

    /// The 15-a-day cap is the ENGINE's, and Gmail proposals go through the ordinary card path so
    /// it applies to them unchanged — which is the narrowing recorded in the fidelity ledger.
    #[test]
    fn an_over_budget_gmail_batch_is_snoozed_not_dropped() {
        let vault = super::tests::scratch_vault("gmail-budget");
        let ctx = WriteContext { actor: GMAIL_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let today = jiff::civil::date(2026, 9, 9);
        for n in 0..20 {
            let item = crate::cloudmodel::GmailItem {
                uid: format!("gmail:m{n}"), tier: "opportunity".into(),
                title: format!("Opportunity {n}"), course: None, due: None,
                effort_hours: None, importance: None, why: "worth a look".into(), confidence: 0.8,
            };
            write_gmail_card(&vault, &item, today, &ctx, &mut journal).expect("card");
        }
        let deferred = crate::approvals::defer_over_budget(&vault, today, 15, &ctx, &mut journal);
        assert_eq!(deferred.len(), 5, "the surplus is snoozed to tomorrow, never deleted");
        let _ = std::fs::remove_dir_all(&vault);
    }
```

…plus four more in the same shape, each against a scripted loopback listener:
`a_clear_task_email_becomes_a_note_with_created_by_gmail`;
`a_borderline_email_becomes_a_proposal_and_not_a_note`;
`an_information_email_writes_nothing_but_is_never_asked_about_twice`;
`nothing_is_acknowledged_that_was_not_written` (a listener that 500s on the second item leaves that uid out of the acknowledgement, and the summary says so).

- [ ] **Step 8: Make `quiet` reach the student.** `pull_gmail_queue` turns `quiet: true` into a `CloudError` whose label is `gmail is not connected`, so `pull_gmail`'s first line becomes `gmail: skipped (gmail is not connected; re-connect from settings)` — a line in the slot log and in `state/runner-log.md`, which is where the student's own diagnostics live. §5.3's *source went quiet* after 14 days is the server's `status = 'quiet'` and is **not** built here: recorded in Task 15's *what production still needs*, because the surface it belongs on is the console's Issues panel and that is C1's.

- [ ] **Step 9: Run, deploy, ask Quinn for P3, commit.**

```
cargo test --workspace
deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
supabase functions deploy judge-email --project-ref <staging ref>
supabase functions deploy gmail-read --project-ref <staging ref>
```

Ask Quinn for P3 in one sentence: *"Add yourself as a test user on the Google consent screen so the Gmail reader can be run end to end against your own mailbox. While the project is in Testing, Google allows at most 100 test users and the token expires every 7 days — the wizard says so."* Then connect once from the staging build and confirm the first read: a queue with rows, `judgments` rows all `origin = 'gmail_api'`, and **no message text anywhere in either table**:

```
curl -s "$SUPABASE_URL/rest/v1/gmail_queue?select=uid,tier,payload" -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
curl -s "$SUPABASE_URL/rest/v1/judgments?origin=eq.gmail_api&select=item_id,fields,model" -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
```

```bash
git add cloud/supabase/functions/judge-email/ cloud/supabase/functions/gmail-read/ engine/src/cloudmodel.rs engine/src/enrich.rs
git commit -F .git-commit-msg.txt   # "engine+cloud: Gmail read and the five email tiers — judged server-side, pulled by the slot, text discarded (C2 Task 11)"
```

---

### Task 12: Rule promotion — the loop that retires model calls

**Files:**
- Create: `cloud/supabase/migrations/20260911000300_rule_promotion.sql`, `cloud/supabase/functions/judge-rules/{handler.ts,handler_test.ts,index.ts}`
- Test: `cloud/supabase/functions/_shared/judge_rules_test.ts`, new tests in `engine/src/enrich.rs`
- Modify: `engine/src/cloudmodel.rs` (`RuleProposal`, `pull_rule_proposals`, `decide_rule`), `engine/src/enrich.rs` (the third pull phase)
- **Hand-off this task needs:** H6 (`engine/src/approvals.rs` — the `kind: rule` arm)

**Interfaces:**
- Produces: `judgment_features()`, `backfill_correction_judgments()`, `promote_rules()` and their one nightly `pg_cron` schedule; `GET /judge-rules` → `{ proposals: Array<{ id, kind, feature, value, verdict, proposed_at }> }`; `POST /judge-rules` `{ id, decision: "approved" | "rejected" }`; `cloudmodel::pull_rule_proposals(client)`, `cloudmodel::decide_rule(client, id, decision)`.

`GET /judge-rules` is also **Task 4's probe**: it is the cheapest authenticated call in the service, it charges no cap and spends no model tokens, so `CloudModel::probe()` uses it to learn once per run whether this account can be judged at all.

- [ ] **Step 1: Write the failing promotion test** — `cloud/supabase/functions/_shared/judge_rules_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { featureMap, features, titlePrefix } from "./judge_rules.ts";

Deno.test("a task is keyed on created_by plus the title prefix, most specific first", () => {
  const item = { title: "CS 100 Lab 02 math_lib", created_by: "zybooks" };
  assertEquals(features("task", item), [
    ["created_by+title_prefix", "zybooks|CS 100 Lab"],
    ["title_prefix", "CS 100 Lab"],
  ]);
  assertEquals(titlePrefix("CS 100 Lab 02 math_lib"), "CS 100 Lab");
});

Deno.test("an event is keyed on organizer, source, series and the title prefix", () => {
  const item = { title: "AI Club Kickoff", organizer: "AI Club", source: "engage", series_uid: "engage:series:9" };
  assertEquals(features("event", item).map(([f]) => f), ["organizer", "source", "series", "title_prefix"]);
});

Deno.test("featureMap and the SQL twin name the same features", async () => {
  // A rule promoted on one key and looked up by another never fires, and nothing would say so —
  // the model would simply keep being asked. So the two implementations are pinned against each
  // other rather than against a comment.
  const sql = await Deno.readTextFile(
    new URL("../../migrations/20260911000300_rule_promotion.sql", import.meta.url),
  );
  const inSql = [...sql.matchAll(/\('(created_by\+title_prefix|title_prefix|organizer|source|series)',/g)]
    .map((m) => m[1]);
  const inTs = new Set([
    ...features("task", { title: "a b c", created_by: "x" }).map(([f]) => f),
    ...features("event", { title: "a b c", organizer: "o", source: "s", series_uid: "u" }).map(([f]) => f),
  ]);
  for (const feature of inSql) assert(inTs.has(feature), `the SQL promotes on '${feature}' and the lookup does not`);
  for (const feature of inTs) assert(inSql.includes(feature), `the lookup keys on '${feature}' and the SQL does not`);
});

Deno.test("the feature map carries keys and never free text", () => {
  const map = featureMap("task", { title: "CS 100 Lab 02 TRIPWIRE-9f2c", created_by: "zybooks" });
  assertEquals(map, { title_prefix: "CS 100 Lab", created_by: "zybooks" });
  assertEquals(JSON.stringify(map).includes("TRIPWIRE"), false);
});

Deno.test("promote_rules is scheduled, deterministic, and never promotes a global rule", async () => {
  const sql = await Deno.readTextFile(
    new URL("../../migrations/20260911000300_rule_promotion.sql", import.meta.url),
  );
  // (a) `min(jsonb)` does not exist in PostgreSQL and would error on the first call.
  assert(!/min\s*\(\s*j\.fields\s*\)/.test(sql), "there is no min() aggregate for jsonb");
  assert(sql.includes("array_agg"), "the representative answer is picked deterministically");
  // (b) the schedule exists and the extension it needs was enabled in 20260911000100.
  assert(sql.includes("cron.schedule('knowlu-promote-rules'"));
  // (c) §11 R5: nothing here writes or activates a global rule.
  assert(!sql.includes("'global'"), "global rules are hand-reviewed and this job never writes one");
  assert(sql.includes("'account'"));
});
```

- [ ] **Step 2: Write `20260911000300_rule_promotion.sql`.**

```sql
-- §5.4 measure 1: a feature whose model verdicts agree at least 3 times, with NO correction
-- against THAT FEATURE in 60 days, becomes a `kind: rule` proposal. Approval writes the rule row;
-- tier 2 then answers without the model, which is the whole point — the loop retires model calls,
-- and it is also the only structural answer to the per-account cost problem.
--
-- **Only account-scoped rules are ever promoted here.** Cross-account (global) promotion is
-- hand-reviewed before activation (§11 R5), and there is deliberately no code path in this
-- function, or anywhere else in this stream, that inserts or activates a `scope = 'global'` row.
--
-- `judgment_features` is the SQL twin of `_shared/judge_rules.ts`'s `features()`; a rule promoted
-- on one key and looked up by another never fires, so `judge_rules_test.ts` pins them together.
-- It reads `judgments.fields`, which is where `featureMap` put them — the row carries no title and
-- no body to recompute them from, and that is the design.
create or replace function judgment_features(p_kind text, p_fields jsonb)
returns table (feature text, value text)
language sql
immutable
set search_path = public, extensions
as $$
  select f.feature, f.value
    from (values
      ('created_by+title_prefix', (p_fields->>'created_by') || '|' || coalesce(p_fields->>'title_prefix', '')),
      ('title_prefix',            p_fields->>'title_prefix'),
      ('organizer',               p_fields->>'organizer'),
      ('source',                  p_fields->>'source'),
      ('series',                  p_fields->>'series_uid')
    ) as f(feature, value)
   where f.value is not null and f.value <> '' and f.value <> '|'
     and ((p_kind = 'task' and f.feature in ('created_by+title_prefix', 'title_prefix'))
       or (p_kind <> 'task' and f.feature in ('organizer', 'source', 'series', 'title_prefix')));
$$;

-- `corrections.judgment_id` AND `judgment_kind` are both filled here, by C2, and by nothing else
-- (ruling R-X-11). C1 sends neither and needs to know nothing about them: it writes the correction
-- from the journal, and this joins it to the judgment it is about.
--
-- **The join is `(account_id, item_id)` and the time order** — deliberately NOT `judgment_kind`,
-- which is the column being filled and would make this a no-op forever. The judgment's own `kind`
-- is what `judgment_kind` becomes, which is also why it cannot be a join key: it is the answer.
-- `item_id` is the note's opaque id, the event uid, or `gmail:<message-id>` on both sides.
--
-- Run before every eval, and cheap enough to run nightly beside the promotion.
create or replace function backfill_correction_judgments()
returns integer
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  filled integer := 0;
begin
  update public.corrections c
     set judgment_id = j.id,
         judgment_kind = j.kind
    from lateral (
      select j2.id, j2.kind
        from judgments j2
       where j2.account_id = c.account_id
         and j2.item_id = c.item_id
         and j2.judged_at < c.ts
       order by j2.judged_at desc
       limit 1
    ) j
   where c.judgment_id is null;
  get diagnostics filled = row_count;
  return filled;
end;
$$;

/*
 * Four things this function had to get right, and the earlier draft got wrong:
 *
 *  (a) `min(j.fields)` — PostgreSQL has no `min()` aggregate for `jsonb`, so that version errored
 *      on its first call and rule promotion never ran at all. The representative answer is now
 *      `(array_agg(j.fields order by j.judged_at desc))[1]`: deterministic, and the most recent.
 *  (b) The agreement test was `count(distinct j.fields) = 1`, i.e. byte-equality of the whole
 *      field map — two 2.5-hour answers and one 2.0 would never promote, and neither would any
 *      task whose title prefix drifted. §5.4 asks for agreement on the VERDICT, so the test is now
 *      over the verdict fields only (`fields - 'created_by' - 'title_prefix' - …`).
 *  (c) The contradiction guard joined on `(account_id, kind)`, so a single correction of any task
 *      in 60 days blocked EVERY task-rule promotion for that account — permanently true for any
 *      active user. §5.4 says "no disagreement" on THAT FEATURE, so it joins on the feature and
 *      value now, through the same `judgment_features` the promotion uses.
 *  (d) `rule_evidence` was created and written by nothing. It is populated here, in the same
 *      statement, so "why does the model never get asked about this?" has an answer.
 */
create or replace function promote_rules()
returns integer
language plpgsql
security invoker
set search_path = public, extensions
as $$
declare
  promoted integer := 0;
begin
  with judged as (
    select j.id, j.account_id, j.kind, j.judged_at, j.fields,
           j.fields - 'created_by' - 'title_prefix' - 'organizer' - 'source' - 'series_uid' as verdict_fields,
           f.feature, f.value
      from judgments j
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where j.tier = 3
       and j.outcome = 'answered'
       and j.judged_at >= now() - interval '60 days'
  ),
  agreed as (
    select account_id, kind, feature, value,
           count(*) as hits,
           count(distinct verdict_fields) as distinct_answers,
           (array_agg(verdict_fields order by judged_at desc))[1] as answer,
           array_agg(id) as judgment_ids
      from judged
     group by account_id, kind, feature, value
  ),
  contradicted as (
    -- A correction against THIS feature and value, not merely against this kind.
    --
    -- Joined on `c.judgment_id`, which C2's own `backfill_correction_judgments()` fills (ruling
    -- R-X-11) — never on `judgment_kind`, which that back-fill is what *writes*. A correction that
    -- has not been back-filled yet simply does not block a promotion this run; the nightly job
    -- runs the back-fill first, so in practice it always has.
    select distinct j.account_id, j.kind, f.feature, f.value
      from public.corrections c
      join judgments j on j.id = c.judgment_id
      cross join lateral judgment_features(j.kind, j.fields) as f(feature, value)
     where c.ts >= now() - interval '60 days'
  ),
  fresh as (
    insert into rules (account_id, scope, kind, feature, value, verdict, active, proposed_at, expires_at)
    select a.account_id, 'account', a.kind, a.feature, a.value, a.answer, false,
           current_date, current_date + 30
      from agreed a
     where a.hits >= 3
       and a.distinct_answers = 1
       and not exists (
             select 1 from contradicted x
              where x.account_id = a.account_id and x.kind = a.kind
                and x.feature = a.feature and x.value = a.value)
       and not exists (
             select 1 from rules r
              where r.account_id = a.account_id and r.kind = a.kind
                and r.feature = a.feature and r.value = a.value
                and (r.active or r.decided_at is null))
    returning id, account_id, kind, feature, value
  )
  insert into rule_evidence (rule_id, judgment_id, agrees)
  select f.id, unnest(a.judgment_ids), true
    from fresh f
    join agreed a
      on a.account_id = f.account_id and a.kind = f.kind
     and a.feature = f.feature and a.value = f.value;

  get diagnostics promoted = row_count;
  -- An undecided proposal expires rather than waiting forever; a re-promotion is a new proposal.
  delete from rules where active = false and decided_at is null and expires_at < current_date;
  return promoted;
end;
$$;

-- Nightly, once, for every account at the same time: promotion is cheap and per-account scheduling
-- would be a second thing to keep true. `pg_cron` was enabled in 20260911000100.
-- The back-fill runs FIRST, in the same statement, because the contradiction guard joins on the
-- column it fills. Two schedules would be two things to keep in order.
select cron.schedule(
  'knowlu-promote-rules', '17 7 * * *',
  $$select backfill_correction_judgments(), promote_rules();$$
);
```

- [ ] **Step 3: Write `judge-rules/handler.ts`, its test and its `index.ts`.**

```ts
// GET  /judge-rules  -> the promoted-rule proposals this account has not decided.
// POST /judge-rules  -> { id, decision } — approve (activate) or reject (settle) one.
//
// This is also the service's cheapest authenticated call, so `CloudModel::probe()` uses it to
// learn once per run whether an account can be judged at all: it charges no cap, spends no model
// tokens, and answers 401/402 exactly as every other endpoint does.
import type { Entitle } from "../_shared/judge_handler.ts";

export interface RuleDeps {
  pending(accountId: string): Promise<Array<{
    id: number; kind: string; feature: string; value: string;
    verdict: Record<string, unknown>; proposed_at: string;
  }>>;
  decide(accountId: string, id: number, decision: "approved" | "rejected"): Promise<boolean>;
}

export function rulesHandler(entitle: Entitle, deps: RuleDeps): (req: Request) => Promise<Response> {
  return async (req: Request): Promise<Response> => {
    try {
      const { account_id } = await entitle(req);
      if (req.method === "GET") {
        return Response.json({ proposals: await deps.pending(account_id) });
      }
      if (req.method !== "POST") return Response.json({ error: "GET or POST" }, { status: 405 });
      const body = await req.json().catch(() => ({})) as { id?: unknown; decision?: unknown };
      const id = typeof body.id === "number" ? body.id : NaN;
      const decision = body.decision;
      if (!Number.isInteger(id) || (decision !== "approved" && decision !== "rejected")) {
        return Response.json({ error: "expected { id: integer, decision: approved|rejected }" }, { status: 400 });
      }
      // The account id is part of the update's filter, not merely checked: a decision on somebody
      // else's proposal must change nothing and say so.
      const changed = await deps.decide(account_id, id, decision);
      if (!changed) return Response.json({ error: "no such undecided proposal" }, { status: 404 });
      return Response.json({ decided: decision });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`judge-rules: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "rules failed" }, { status: 500 });
    }
  };
}
```

```ts
// cloud/supabase/functions/judge-rules/handler_test.ts
import { assertEquals } from "@std/assert";
import { rulesHandler } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const PROPOSAL = {
  id: 41, kind: "task", feature: "created_by+title_prefix", value: "zybooks|CS 100 Lab",
  verdict: { effort_hours: "0.5", importance: "2" }, proposed_at: "2026-09-11",
};

function deps(overrides: Record<string, unknown> = {}) {
  return {
    pending: () => Promise.resolve([PROPOSAL]),
    decide: () => Promise.resolve(true),
    ...overrides,
  };
}

Deno.test("GET lists this account's undecided proposals", async () => {
  const reply = await (await rulesHandler(OK, deps())(new Request("http://127.0.0.1/judge-rules"))).json();
  assertEquals(reply.proposals.length, 1);
  assertEquals(reply.proposals[0].id, 41);
});

Deno.test("a decision names the account, so one account cannot decide another's proposal", async () => {
  const seen: unknown[] = [];
  const handler = rulesHandler(OK, deps({
    decide: (account: string, id: number, decision: string) => {
      seen.push([account, id, decision]);
      return Promise.resolve(true);
    },
  }));
  const response = await handler(new Request("http://127.0.0.1/judge-rules", {
    method: "POST", body: JSON.stringify({ id: 41, decision: "approved" }),
  }));
  assertEquals(response.status, 200);
  assertEquals(seen, [["acct-1", 41, "approved"]]);
});

Deno.test("a decision on nothing is a 404, not a silent success", async () => {
  const handler = rulesHandler(OK, deps({ decide: () => Promise.resolve(false) }));
  const response = await handler(new Request("http://127.0.0.1/judge-rules", {
    method: "POST", body: JSON.stringify({ id: 99, decision: "approved" }),
  }));
  assertEquals(response.status, 404);
});

Deno.test("a malformed decision is a 400", async () => {
  const handler = rulesHandler(OK, deps());
  for (const body of ['{"id":"41","decision":"approved"}', '{"id":41,"decision":"maybe"}', "{}"]) {
    const response = await handler(new Request("http://127.0.0.1/judge-rules", { method: "POST", body }));
    assertEquals(response.status, 400, body);
  }
});
```

```ts
// cloud/supabase/functions/judge-rules/index.ts
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { sharedDb } from "../_shared/judge_deps.ts";
import { rulesHandler } from "./handler.ts";

Deno.serve(rulesHandler(requireActiveEntitlement, {
  async pending(accountId) {
    return await sharedDb().select(
      `rules?account_id=eq.${accountId}&active=is.false&decided_at=is.null&select=id,kind,feature,value,verdict,proposed_at&order=proposed_at`,
    ) as Array<{ id: number; kind: string; feature: string; value: string; verdict: Record<string, unknown>; proposed_at: string }>;
  },
  async decide(accountId, id, decision) {
    const before = await sharedDb().select(
      `rules?account_id=eq.${accountId}&id=eq.${id}&decided_at=is.null&select=id`,
    ) as Array<{ id: number }>;
    if (before.length === 0) return false;
    await sharedDb().update(
      `rules?account_id=eq.${accountId}&id=eq.${id}&decided_at=is.null`,
      { active: decision === "approved", decided_at: new Date().toISOString() },
    );
    return true;
  },
}));
```

- [ ] **Step 4: Write the device side** in `engine/src/cloudmodel.rs`:

```rust
/// One promoted-rule proposal the service is offering.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleProposal {
    pub id: i64,
    pub kind: String,
    pub feature: String,
    pub value: String,
    pub verdict: Value,
    pub proposed_at: String,
}

pub fn pull_rule_proposals(client: &CloudClient) -> Result<Vec<RuleProposal>, CloudError> {
    let reply = client.get("/judge-rules")?;
    let mut out = Vec::new();
    for row in reply.get("proposals").and_then(Value::as_array).into_iter().flatten() {
        let text = |key: &str| row.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let Some(id) = row.get("id").and_then(Value::as_i64) else { continue };
        out.push(RuleProposal {
            id,
            kind: text("kind"),
            feature: text("feature"),
            value: text("value"),
            verdict: row.get("verdict").cloned().unwrap_or(Value::Null),
            proposed_at: text("proposed_at"),
        });
    }
    Ok(out)
}

pub fn decide_rule(client: &CloudClient, id: i64, decision: &str) -> Result<(), CloudError> {
    client.post("/judge-rules", &json!({ "id": id, "decision": decision })).map(|_| ())
}
```

and in `engine/src/enrich.rs`:

```rust
/// The agent actor for rule cards. Its own name, not `agent:knowlu.enrich`, so the journal says
/// which loop filed a card.
pub const RULES_ACTOR: &str = "agent:knowlu.rules";

/// Send the decisions the student has already made, then file whatever the service is offering.
///
/// **Sending first** means a slot never proposes a rule the student answered an hour ago. The card
/// is an ordinary proposal, so it is counted, escalated, snoozed past the 15-a-day cap and expired
/// exactly as every other card is. `rank` leaves an approved one alone (hand-off H6), because
/// `rank` never opens a socket for a judgment.
pub fn pull_rules(
    vault: &Path,
    client: &crate::cloudmodel::CloudClient,
    opts: &Options<'_>,
) -> Vec<String> {
    let ctx = WriteContext {
        actor: RULES_ACTOR.to_string(),
        via: opts.via.to_string(),
        run_id: opts.run_id.map(str::to_string),
    };
    let mut journal = Journal::new(vault);
    let mut lines: Vec<String> = Vec::new();
    let today = jiff::Zoned::now().date();

    for (path, id, decision) in decided_rule_cards(vault) {
        match crate::cloudmodel::decide_rule(client, id, &decision) {
            Ok(()) => {
                let rel = crate::ids::rel(vault, &path);
                let stamped = format!("\"{}\"", jiff::Zoned::now().strftime("%Y-%m-%d %H:%M"));
                let literals = vec![
                    ("status".to_string(), "executed".to_string()),
                    ("executed_at".to_string(), stamped),
                ];
                if crate::write::write_literals(vault, &rel, &literals, &ctx, &mut journal, &WriteOpts::default()).is_ok() {
                    let _ = crate::write::delete(vault, &rel, &ctx, &mut journal);
                }
                lines.push(format!("rules {id}: {decision}"));
            }
            // NOT archived on failure: an unsent decision must come back next slot.
            Err(e) => lines.push(format!("rules {id}: not sent ({e})")),
        }
    }

    let proposals = match crate::cloudmodel::pull_rule_proposals(client) {
        Ok(p) => p,
        Err(e) => {
            lines.push(format!("rules: skipped ({e})"));
            return lines;
        }
    };
    if proposals.is_empty() {
        return lines;
    }
    let existing = existing_rule_ids(vault);
    let mut filed = 0usize;
    for proposal in &proposals {
        // A card is never minted twice for one proposal id — the id is in the card's frontmatter
        // and this is the guard, exactly as `find_pending_amendment` guards an amend card.
        if existing.contains(&proposal.id) {
            continue;
        }
        match write_rule_card(vault, proposal, today, &ctx, &mut journal) {
            Ok(stem) => {
                filed += 1;
                lines.push(format!("rules {}: proposed ({stem})", proposal.id));
            }
            Err(e) => lines.push(format!("rules {}: not written ({e})", proposal.id)),
        }
    }
    lines.push(format!("rules: {filed} proposed of {} offered", proposals.len()));
    lines
}

/// `kind: rule` cards the student has answered, as `(path, rule_id, decision)`.
fn decided_rule_cards(vault: &Path) -> Vec<(std::path::PathBuf, i64, String)> {
    let mut out = Vec::new();
    for path in crate::approvals::sorted_md(&vault.join("approvals")) {
        let Ok(text) = crate::pystr::read_text(&path) else { continue };
        let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
        if crate::yaml::opt_text(crate::yaml::get(&meta, "kind")).as_deref() != Some("rule") {
            continue;
        }
        let status = crate::yaml::opt_text(crate::yaml::get(&meta, "status")).unwrap_or_default();
        if status != "approved" && status != "rejected" {
            continue;
        }
        let Some(id) = crate::yaml::get(&meta, "rule_id").and_then(crate::yaml::i64_of) else { continue };
        out.push((path, id, status));
    }
    out
}

/// Every `rule_id` already filed — `approvals/` for the live ones and `archive/` for the settled,
/// because a card that was decided last week must not be re-offered this week.
fn existing_rule_ids(vault: &Path) -> std::collections::BTreeSet<i64> {
    let mut out = std::collections::BTreeSet::new();
    for folder in ["approvals", "archive"] {
        for path in crate::approvals::sorted_md(&vault.join(folder)) {
            let Ok(text) = crate::pystr::read_text(&path) else { continue };
            let Ok((meta, _)) = crate::models::split_frontmatter(&text) else { continue };
            if let Some(id) = crate::yaml::get(&meta, "rule_id").and_then(crate::yaml::i64_of) {
                out.insert(id);
            }
        }
    }
    out
}

/// The card the deck renders. `kind: rule` is a kind `approvals.rs` leaves alone (hand-off H6):
/// the decision is *sent* by the next `judge` step, which then stamps and archives it.
fn write_rule_card(
    vault: &Path,
    proposal: &crate::cloudmodel::RuleProposal,
    today: jiff::civil::Date,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<String, String> {
    let lit = |s: &str| write::to_literal(&Value::String(s.to_string()));
    let stamp = today.strftime("%Y-%m-%d").to_string();
    let expires = today
        .checked_add(jiff::Span::new().days(14))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let title = format!("Always judge {} the same way", proposal.value.replace('|', " "));
    let mut body = String::from(
        "Three judgments agreed and none disagreed in 60 days. Approving this stops the model \
         being asked about this pattern again; the fields it will write are below.\n\n",
    );
    if let Some(map) = proposal.verdict.as_object() {
        let mut names: Vec<&String> = map.keys().collect();
        names.sort();
        for name in names {
            body.push_str(&format!("- {name}: {}\n", crate::pystr::json_str(&map[name])));
        }
    }
    let text = format!(
        "---\ntype: approval\nkind: rule\ntitle: {}\nstatus: pending\nproposed_at: {stamp}\n\
         first_proposed_at: {stamp}\nexpires: {expires}\nsnooze_until: null\ncreated_by: rules\n\
         rule_id: {}\nfeature: {}\nvalue: {}\n---\n\n{body}",
        lit(&title),
        proposal.id,
        lit(&proposal.feature),
        lit(&proposal.value),
    );
    let approvals = vault.join("approvals");
    std::fs::create_dir_all(&approvals).map_err(|e| e.to_string())?;
    let stem = format!("rule-{}", proposal.id);
    let rel = crate::ids::rel(vault, &approvals.join(format!("{stem}.md")));
    crate::write::create(vault, &rel, &text, ctx, journal, None)
        .map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        .map_err(|e| e.to_string())
}
```

and the last line of `run_lines_with`'s cloud arm, after the Gmail pull:

```rust
    lines.extend(pull_rules(vault, client, opts));
```

- [ ] **Step 5: Write the engine tests** — in `enrich.rs`'s test module, each against a scripted loopback listener, in the shape of Task 4 step 8's test (a scratch vault copied from a fixture, a `CloudClient` aimed at the listener, the listener joined before the test returns):

```rust
    #[test]
    fn a_rule_proposal_becomes_a_card_the_deck_can_answer() {
        let vault = super::tests::scratch_vault("rules-file");
        let offered = crate::ledger::dumps_value(&serde_json::json!({
            "proposals": [{
                "id": 41, "kind": "task", "feature": "created_by+title_prefix",
                "value": "zybooks|CS 100 Lab",
                "verdict": {"effort_hours": "0.5", "importance": "2"},
                "proposed_at": "2026-09-11"
            }]
        }));
        let (client, mut server) = super::tests::loopback_client(vec![(200, offered)]);
        let opts = super::tests::opts();
        let lines = pull_rules(&vault, &client, &opts);
        assert!(lines.iter().any(|l| l.contains("rules 41: proposed")), "{lines:?}");
        let card = std::fs::read_to_string(vault.join("approvals").join("rule-41.md")).expect("the card exists");
        assert!(card.contains("kind: rule"));
        assert!(card.contains("rule_id: 41"));
        assert!(card.contains("status: pending"));
        assert!(card.contains("- effort_hours: 0.5"));
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_card_is_never_minted_twice_for_one_proposal_id() {
        let vault = super::tests::scratch_vault("rules-once");
        let offered = crate::ledger::dumps_value(&serde_json::json!({
            "proposals": [{ "id": 41, "kind": "task", "feature": "title_prefix", "value": "CS 100 Lab",
                            "verdict": {"importance": "2"}, "proposed_at": "2026-09-11" }]
        }));
        let (client, mut server) =
            super::tests::loopback_client(vec![(200, offered.clone()), (200, offered)]);
        let opts = super::tests::opts();
        let first = pull_rules(&vault, &client, &opts);
        assert!(first.iter().any(|l| l.contains("1 proposed of 1")), "{first:?}");
        let second = pull_rules(&vault, &client, &opts);
        assert!(second.iter().any(|l| l.contains("0 proposed of 1")), "{second:?}");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn an_approved_card_is_sent_once_and_then_archived() {
        let vault = super::tests::scratch_vault("rules-decide");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");

        let empty = crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }));
        let (client, mut server) = super::tests::loopback_client(vec![
            (200, crate::ledger::dumps_value(&serde_json::json!({ "decided": "approved" }))),
            (200, empty.clone()),
            (200, empty),
        ]);
        let opts = super::tests::opts();
        let lines = pull_rules(&vault, &client, &opts);
        assert!(lines.iter().any(|l| l.contains("rules 41: approved")), "{lines:?}");
        assert!(!vault.join("approvals").join(format!("{stem}.md")).exists());
        // A third pull sends nothing: the card is archived and `existing_rule_ids` still sees it.
        let again = pull_rules(&vault, &client, &opts);
        assert!(!again.iter().any(|l| l.contains("rules 41:")), "{again:?}");
        let sent = server.requests();
        assert_eq!(sent.iter().filter(|r| r.contains("POST /functions/v1/judge-rules")).count(), 1);
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_decision_that_cannot_be_sent_keeps_its_card() {
        let vault = super::tests::scratch_vault("rules-retry");
        let ctx = WriteContext { actor: RULES_ACTOR.into(), via: "local-runner".into(), run_id: None };
        let mut journal = Journal::new(&vault);
        let proposal = crate::cloudmodel::RuleProposal {
            id: 41, kind: "task".into(), feature: "title_prefix".into(), value: "CS 100 Lab".into(),
            verdict: serde_json::json!({"importance": "2"}), proposed_at: "2026-09-11".into(),
        };
        let stem = write_rule_card(&vault, &proposal, jiff::civil::date(2026, 9, 11), &ctx, &mut journal)
            .expect("the card writes");
        crate::write::write_literals(
            &vault, &format!("approvals/{stem}.md"),
            &[("status".to_string(), "approved".to_string())],
            &ctx, &mut journal, &WriteOpts::default(),
        )
        .expect("approve");
        let (client, mut server) = super::tests::loopback_client(vec![
            (503, r#"{"error":"upstream"}"#.to_string()),
            (200, crate::ledger::dumps_value(&serde_json::json!({ "proposals": [] }))),
        ]);
        let lines = pull_rules(&vault, &client, &super::tests::opts());
        assert!(lines.iter().any(|l| l.contains("not sent")), "{lines:?}");
        assert!(vault.join("approvals").join(format!("{stem}.md")).exists(), "the card must survive to retry");
        let _ = server.requests();
        let _ = std::fs::remove_dir_all(&vault);
    }
```

`super::tests::loopback_client` and `super::tests::opts` are two helpers this task adds beside `scratch_vault` in `enrich.rs`'s test module: the first is Task 4's `loopback` returning a `CloudClient` aimed at it, the second is an `Options` with `via: "local-runner"`, `limit: 50` and `budget: BATCH_BUDGET`.

- [ ] **Step 6: Run, apply, deploy, name hand-off H6, commit.**

```
cargo test --workspace
deno test --allow-read --allow-net=127.0.0.1 --config cloud/supabase/deno.json cloud/supabase/ cloud/eval/
supabase db push --project-ref <staging ref>
supabase functions deploy judge-rules --project-ref <staging ref>
```

Then force one promotion round on staging and read what it did — this is the only way to see the SQL work without Docker:

```
curl -s -X POST "$SUPABASE_URL/rest/v1/rpc/promote_rules" -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
curl -s "$SUPABASE_URL/rest/v1/rules?select=id,kind,feature,value,active,verdict" -H "apikey: $SUPABASE_SERVICE_ROLE_KEY" -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"
```

Expected: a number (0 is correct on a fresh project), no error, and no row with `scope = 'global'`. **Record the number.**

```bash
git add cloud/supabase/migrations/20260911000300_rule_promotion.sql cloud/supabase/functions/judge-rules/ cloud/supabase/functions/_shared/judge_rules.ts cloud/supabase/functions/_shared/judge_rules_test.ts engine/src/cloudmodel.rs engine/src/enrich.rs
git commit -F .git-commit-msg.txt   # "cloud+engine: rule promotion — three agreements and no contradiction become a card, and tier 2 answers next time (C2 Task 12)"
```


---

### Task 13: The eval seed — read the archive once, de-identify, never again

**Precondition P4 — Quinn's explicit go. This is the only task in the plan that opens `C:\Users\danie\GitHub\quinn-ops`, it opens it read-only, once, in one sitting, and it writes nothing there.** If P4 has not been given, stop and ask; do not start.

**Files:**
- Create: `cloud/eval/deidentify.ts`, `cloud/eval/scrub_test.ts`, `cloud/eval/seed/{tasks,events,emails}.jsonl`, `cloud/eval/seed/README.md`

**What is extracted, exactly.** Three sources in the archive, and nothing else is opened:

1. **The seven course notes** (`courses/*.md`). From each: the `## Grade weights` section text, and nothing above or below it. The slug becomes `course-1` … `course-7` in first-seen order; every line that is not `<category> <number>%`-shaped is dropped. Result: seven weights tables, in the shape a prompt actually receives.
2. **The event verdicts** (`state/events-seen.md`, ~420 lines). From each line: `title`, `organizer`, `source`, the `verdict`, and the day offset between the event's start and the line's `first seen` date. The uid becomes `ev-0001`… in file order. The `why` text is **dropped entirely** — it is free text a person wrote about a real event and no filter makes that safe. The paired description comes from `state/events.md` where one survives, clipped to 1200 characters.
3. **The journal's amend and override records** (`state/journal/*.jsonl`): a record where `provenance::is_agent` was true for the setter, and a later record with `via: dashboard` changing the same field. From each pair: `(kind, field, ours, theirs)` plus the note's `title` and `body` at the time, clipped. The note id becomes `task-0001`… in first-seen order.

- [ ] **Step 1: Ask Quinn for P4** in one sentence, with the context: *"The eval suite needs a floor on day one, and the only labelled data that exists is in the archived vault — seven course notes' grade weights, about 420 event verdicts, and the journal's amend records. I would read it once, read-only, write nothing there, and produce a de-identified corpus under `cloud/eval/seed/` whose scrub test refuses any email address, URL, path, long number or two-capitalised-word organizer. Nothing that names you or anyone else ends up in the repository. May I?"* **Do not proceed without a yes.**

- [ ] **Step 2: Write `cloud/eval/scrub_test.ts` FIRST**, and prove it fails on each rule before any archive is opened.

```ts
// The seed is committed to this repository, so what it may contain is a test and not a habit.
//
// The assertions are STRUCTURAL and name no person: a banned-name list in a repository is itself a
// single-user assumption, and it would also be the one file that leaks the name it exists to keep
// out. What is asserted instead is the shape of a record that has been through `deidentify.ts`.
import { assert, assertMatch } from "@std/assert";

const SEED = new URL("./seed/", import.meta.url);

for (const file of ["tasks.jsonl", "events.jsonl", "emails.jsonl"]) {
  Deno.test(`${file} carries nothing that identifies a person, a machine or an account`, async () => {
    const text = await Deno.readTextFile(new URL(file, SEED));
    for (const line of text.split("\n").filter((l) => l.trim() !== "")) {
      const record = JSON.parse(line);
      assertMatch(record.id, /^(task|ev|email)-\d{4}$/, "ids are synthetic and sequential");
      assert(!line.includes("@"), "no email address survives de-identification");
      assert(!/https?:\/\//.test(line), "no URL survives de-identification");
      assert(!/[A-Za-z]:\\\\/.test(line), "no Windows path survives de-identification");
      assert(!/\/(home|Users)\//.test(line), "no POSIX home path survives de-identification");
      assert(!/\d{7,}/.test(line), "no long number survives de-identification");
      if (typeof record.organizer === "string") {
        assertMatch(
          record.organizer,
          /^(Organizer \d+|[A-Za-z0-9 &'-]+)$/,
          "an organizer is an office or a placeholder",
        );
        assert(
          !/^[A-Z][a-z]+ [A-Z][a-z]+$/.test(record.organizer),
          "two capitalised words alone is a person, not an office",
        );
      }
      if (typeof record.weights === "string") {
        for (const w of record.weights.split("\n").filter((l: string) => l !== "")) {
          assertMatch(w, /^[A-Za-z ]+ \d+%$/, "a weights line is a category and a percentage, and nothing else");
        }
      }
      if (typeof record.course === "string" && record.course !== "") {
        assertMatch(record.course, /^course-[1-9]\d*$/, "course slugs are synthetic");
      }
    }
  });
}
```

Run it against three hand-written synthetic records that each break one rule (an `@`, a `C:\` path, a `Firstname Lastname` organizer). Expected: **3 failures, one per rule**, before the archive is opened at all.

- [ ] **Step 3: Write `cloud/eval/deidentify.ts`.** It is a real program, and it is the only thing that ever writes `cloud/eval/seed/`.

```ts
// Read the archived vault once, under a one-time approval, and write a de-identified corpus.
//
// Run: deno run --allow-read=<archive> --allow-write=cloud/eval/seed \
//        --config cloud/supabase/deno.json cloud/eval/deidentify.ts --archive <archive>
//
// Every rule below is applied to EVERY string in every record, and then the whole corpus is
// re-read and any record that still matches any rule is dropped with a count. A hand edit to a
// seed file is forbidden: a hand edit is a rule nobody can re-apply.
const RULES: Array<[RegExp, string | null]> = [
  // null means "drop the record", a string means "replace".
  [/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, null],
  [/https?:\/\/\S+/g, "URL"],
  [/[A-Za-z]:\\[^\s"]+/g, null],
  [/\/(home|Users)\/\S+/g, null],
  [/\d{7,}/g, "NUMBER"],
];

export interface Scrubbed {
  value: unknown;
  dropped: string | null;
}

export function scrub(value: unknown): Scrubbed {
  if (typeof value === "string") {
    let out = value;
    for (const [pattern, replacement] of RULES) {
      if (replacement === null) {
        if (pattern.test(out)) return { value: null, dropped: pattern.source };
        pattern.lastIndex = 0;
      } else {
        out = out.replace(pattern, replacement);
      }
    }
    return { value: out, dropped: null };
  }
  if (Array.isArray(value)) {
    const out: unknown[] = [];
    for (const entry of value) {
      const got = scrub(entry);
      if (got.dropped !== null) return { value: null, dropped: got.dropped };
      out.push(got.value);
    }
    return { value: out, dropped: null };
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) {
      const got = scrub(v);
      if (got.dropped !== null) return { value: null, dropped: got.dropped };
      out[k] = got.value;
    }
    return { value: out, dropped: null };
  }
  return { value, dropped: null };
}

/// An organizer that is two capitalised words is a person; an office keeps its name because an
/// office is rarely exactly two capitalised words ("Career Center", "Office of Undergraduate
/// Research"). Numbered in first-seen order, so the same person is the same placeholder across the
/// corpus and a promotion feature still means something.
export function anonymiseOrganizer(raw: string, seen: Map<string, string>): string {
  if (!/^[A-Z][a-z]+ [A-Z][a-z]+$/.test(raw.trim())) return raw.trim();
  const known = seen.get(raw);
  if (known !== undefined) return known;
  const placeholder = `Organizer ${seen.size + 1}`;
  seen.set(raw, placeholder);
  return placeholder;
}

/// A `## Grade weights` section, reduced to the lines that are a category and a percentage.
export function weightsOnly(section: string): string {
  return section
    .split("\n")
    .map((l) => l.replace(/^[-*]\s*/, "").trim())
    .map((l) => {
      const m = /^([A-Za-z][A-Za-z ]*?)\s*[:\-]?\s*(\d{1,3})\s*%$/.exec(l);
      return m === null ? "" : `${m[1].trim()} ${m[2]}%`;
    })
    .filter((l) => l !== "")
    .join("\n");
}
```

…and the extraction half, which walks the three sources, applies `anonymiseOrganizer` and `weightsOnly`, renumbers every id, calls `scrub` on each record, drops what it must, counts the drops by rule, and writes the three JSONL files plus the counts to stdout. **It never writes into the archive and never opens a path outside the `--archive` root** — `--allow-read=<archive>` is what makes that structural rather than careful.

- [ ] **Step 4: Run the extraction, once.** It prints, and this step records: how many records of each kind were read, how many were dropped, and by which rule.

- [ ] **Step 5: Run the scrub test.** `deno test --allow-read --config cloud/supabase/deno.json cloud/eval/` → green. If any assertion fails, **fix `deidentify.ts` and re-extract**; never hand-edit a seed file.

- [ ] **Step 6: Read the three files with your own eyes** — every `organizer` and `title` in `events.jsonl` and every `weights` block. The scrub test is a filter, not a reviewer.

- [ ] **Step 7: Write `cloud/eval/seed/README.md`** — the archive's commit sha, the date read, the three sources, the de-identification rules verbatim, the drop counts, and the sentence: *"Read once under a one-time approval; the archive is not opened again by anything in this repository."*

- [ ] **Step 8: Commit.**

```bash
git add cloud/eval/seed/ cloud/eval/deidentify.ts cloud/eval/scrub_test.ts
git commit -F .git-commit-msg.txt   # "eval: a de-identified seed corpus from the archive, read once, with the scrub test that proves it (C2 Task 13)"
```

---

### Task 14: The eval suite, and the deploy gate

**Files:**
- Create: `cloud/supabase/migrations/20260911000400_eval.sql`, `cloud/eval/score.ts`, `cloud/eval/run_eval.ts`, `cloud/eval/run_eval_test.ts`, `cloud/eval/thresholds.json`
- **Hand-off this task needs:** H8 (`.github/workflows/ci.yml` — the amendment to C1's `cloud` job, and `eval-gate`)

**Interfaces:**
- Consumes: Task 12's `backfill_correction_judgments()`.
- Produces: `eval_cases`, `eval_runs`; `run_eval.ts --thresholds <file> [--dry-run] [--load-seed]` exiting non-zero on a regression.

**What the suite is, as ruling R-C2-4 narrows it.** §5.4 measure 2 asks for the last 90 days of corrections **plus** a frozen seed. §6 (b) carries no note body and §5.2 discards bodies after the call — both signed — so a correction can only become a *replayable* case when the account has telemetry class (c) switched on and C1 sends the re-derived `request` beside it. **The (c) toggle's UI is C4**, so at merge the suite runs on the seed alone, and it grows real cases the day (c) ships. Corrections without a `request` are still used, for the correction-rate metrics and for rule promotion — they are simply not replayable.

- [ ] **Step 1: Write `20260911000400_eval.sql`.**

```sql
-- §5.4 measure 2, as narrowed by ruling R-C2-4: the suite replays cases against the pinned model
-- on every prompt or pin change, and a regression past a per-kind threshold blocks the deploy.
--
-- `source` says where a case came from, and it is the honest record of the narrowing: 'seed' is
-- Task 13's frozen corpus, 'correction' is a real user correction that arrived WITH a replayable
-- request under the (c) opt-in. Until C4 ships that toggle, every row is 'seed'.
create table if not exists eval_cases (
  id       bigserial primary key,
  kind     text not null check (kind in ('task', 'event', 'email')),
  request  jsonb not null,
  ours     jsonb,
  theirs   jsonb not null,
  source   text not null check (source in ('correction', 'seed')),
  added_at timestamptz not null default now()
);
alter table eval_cases enable row level security;
create index if not exists eval_cases_kind on eval_cases (kind, source);

create table if not exists eval_runs (
  id              bigserial primary key,
  ran_at          timestamptz not null default now(),
  model           text not null,
  prompt_version  text not null,
  grammar_version text not null,
  prompt_hash     text not null,
  kind            text not null check (kind in ('task', 'event', 'email')),
  metric          text not null,
  value           real not null,
  threshold       real not null,
  passed          boolean not null,
  cases           integer not null
);
alter table eval_runs enable row level security;
create index if not exists eval_runs_recent on eval_runs (kind, metric, ran_at desc);

(`backfill_correction_judgments()` is **not** here: it lives in `20260911000300_rule_promotion.sql`, because that migration's nightly job calls it and a function must exist before the schedule that names it is trustworthy. Task 14 only reads it.)
```

- [ ] **Step 2: Write `cloud/eval/score.ts`** — pure, so `run_eval_test.ts` exercises every metric with no model at all.

```ts
// The scorer. Pure, so the harness is tested with no key and no spend.
import type { Kind } from "../supabase/functions/_shared/judge_validate.ts";

export interface Case {
  kind: Kind;
  /** What the human set — the label. */
  theirs: Record<string, unknown>;
}

export interface Scored {
  metric: string;
  value: number;
  /** `true` when a HIGHER value is better; false for an error metric. */
  higherIsBetter: boolean;
}

/**
 * The asymmetry is the whole point. A missed obligation is the failure this product exists to
 * prevent, and a task that reads as a newsletter is the second: both cost three times what the
 * harmless direction costs, so a model that gets "safe" by dropping everything scores worse.
 */
export const COST: Record<string, number> = {
  "obligation->drop": 3,
  "obligation->opportunity": 1,
  "opportunity->drop": 1,
  "drop->opportunity": 1,
  "task->information": 3,
  "task->borderline": 1,
  "information->borderline": 1,
  "information->task": 2,
};

function cost(theirs: string, ours: string): number {
  if (theirs === ours) return 0;
  return COST[`${theirs}->${ours}`] ?? 1;
}

export function score(kind: Kind, cases: Case[], answers: Array<Record<string, unknown> | null>): Scored[] {
  const n = cases.length;
  if (n === 0) return [];
  if (kind === "task") {
    let absError = 0, importanceHits = 0, courseHits = 0;
    for (let i = 0; i < n; i++) {
      const theirs = cases[i].theirs, ours = answers[i] ?? {};
      absError += Math.abs(Number(theirs.effort_hours ?? 0) - Number(ours.effort_hours ?? 0));
      if (Number(theirs.importance) === Number(ours.importance)) importanceHits++;
      if ((theirs.course ?? null) === (ours.course ?? null)) courseHits++;
    }
    return [
      { metric: "effort_mae", value: absError / n, higherIsBetter: false },
      { metric: "importance_exact", value: importanceHits / n, higherIsBetter: true },
      { metric: "course_exact", value: courseHits / n, higherIsBetter: true },
    ];
  }
  const field = kind === "event" ? "verdict" : "tier";
  let earned = 0, possible = 0;
  for (let i = 0; i < n; i++) {
    const theirs = String(cases[i].theirs[field] ?? "");
    const ours = String((answers[i] ?? {})[field] ?? "");
    possible += 3;
    earned += 3 - Math.min(3, cost(theirs, ours));
  }
  return [{ metric: "weighted_exact", value: possible === 0 ? 0 : earned / possible, higherIsBetter: true }];
}

/** A metric fails when it is WORSE than its threshold. Never equal — a gate at the observed value fires on noise. */
export function failed(s: Scored, threshold: number): boolean {
  return s.higherIsBetter ? s.value < threshold : s.value > threshold;
}
```

- [ ] **Step 3: Write `cloud/eval/run_eval.ts`.**

```ts
// The eval suite (§5.4 measure 2). Loads the cases, replays each `request` through the LIVE
// pipeline with the currently pinned model, scores, writes one `eval_runs` row per metric, and
// exits 1 if any metric is worse than its threshold.
//
// Three switches:
//   --load-seed   insert Task 13's three JSONL files as `source = 'seed'` rows and exit.
//   --dry-run     score against a ScriptedModel seeded from each case's recorded answer, so the
//                 harness itself is tested with NO KEY AND NO SPEND.
//   --thresholds  the file the gate reads. Required unless --load-seed.
//
// Every run of this program without --dry-run spends real money. The per-case cost is one model
// call; `--dry-run` first, always.
import { AnthropicModel, ScriptedModel } from "../supabase/functions/_shared/judge_anthropic.ts";
import { capStore } from "../supabase/functions/_shared/judge_caps.ts";
import { serviceDb } from "../supabase/functions/_shared/judge_db.ts";
import { modelRow } from "../supabase/functions/_shared/judge_models.ts";
import { judge, type Kind } from "../supabase/functions/_shared/judge_pipeline.ts";
import { promptHash } from "../supabase/functions/_shared/judge_prompts.ts";
import { type Case, failed, score } from "./score.ts";

const KINDS: Kind[] = ["task", "event", "email"];
/** The account the eval runs as. A dedicated staging account, so its caps and budget are its own. */
const EVAL_ACCOUNT = Deno.env.get("EVAL_ACCOUNT_ID") ?? "";

interface Row {
  id: number;
  kind: Kind;
  request: { item: Record<string, unknown>; heuristics_seed: Record<string, unknown> };
  ours: Record<string, unknown> | null;
  theirs: Record<string, unknown>;
}

async function load(db: ReturnType<typeof serviceDb>, kind: Kind): Promise<Row[]> {
  // The frozen seed, always; plus the last 90 days of corrections that arrived WITH a replayable
  // request (ruling R-C2-4 — that is only ever true under the (c) opt-in, whose UI is C4).
  const since = new Date(Date.now() - 90 * 86_400_000).toISOString();
  return await db.select(
    `eval_cases?kind=eq.${kind}&or=(source.eq.seed,added_at.gte.${since})&select=id,kind,request,ours,theirs`,
  ) as Row[];
}

async function main(): Promise<number> {
  const args = new Set(Deno.args);
  const db = serviceDb();

  if (args.has("--load-seed")) {
    let loaded = 0;
    for (const [file, kind] of [["tasks", "task"], ["events", "event"], ["emails", "email"]] as const) {
      const text = await Deno.readTextFile(new URL(`./seed/${file}.jsonl`, import.meta.url));
      for (const line of text.split("\n").filter((l) => l.trim() !== "")) {
        const record = JSON.parse(line) as { request: unknown; theirs: unknown };
        await db.insert("eval_cases", {
          kind, request: record.request, ours: null, theirs: record.theirs, source: "seed",
        }, false);
        loaded += 1;
      }
    }
    console.log(`loaded ${loaded} seed cases`);
    return 0;
  }

  const thresholdIndex = Deno.args.indexOf("--thresholds");
  if (thresholdIndex < 0) {
    console.error("--thresholds <file> is required");
    return 2;
  }
  const thresholds = JSON.parse(await Deno.readTextFile(Deno.args[thresholdIndex + 1])) as
    Record<string, Record<string, number>>;

  // Corrections carry no judgment id from the device; fill them before anything reads them.
  await db.rpc("backfill_correction_judgments", {});

  const dry = args.has("--dry-run");
  const apiKey = Deno.env.get("ANTHROPIC_API_KEY") ?? "";
  if (!dry && apiKey === "") {
    console.error("ANTHROPIC_API_KEY is required unless --dry-run");
    return 2;
  }

  let worst = 0;
  for (const kind of KINDS) {
    const rows = await load(db, kind);
    if (rows.length === 0) {
      console.log(`${kind}: no cases — skipped`);
      continue;
    }
    const row = await modelRow(db, kind);
    const cases: Case[] = rows.map((r) => ({ kind, theirs: r.theirs }));
    const answers: Array<Record<string, unknown> | null> = [];
    for (const r of rows) {
      const model = dry
        ? new ScriptedModel([r.ours ?? r.theirs])
        : new AnthropicModel({ apiKey });
      const reply = await judge(EVAL_ACCOUNT, { kind, item: r.request.item, heuristics_seed: r.request.heuristics_seed }, {
        row,
        model,
        // Tier 2 is deliberately OFF for the eval: a promoted rule would score the rule, not the
        // model, and the gate exists to decide whether the MODEL still does the job.
        rules: { lookup: () => Promise.resolve(null) },
        caps: capStore(db),
        // The eval's own judgments are not the product's: they would poison rule promotion and the
        // correction rates with answers nobody ever saw.
        log: { write: () => Promise.resolve(null) },
        origin: "device",
        now: () => Date.now(),
      });
      answers.push(reply.verdict);
    }
    const hash = await promptHash(kind);
    for (const scored of score(kind, cases, answers)) {
      const threshold = thresholds[kind]?.[`${scored.metric}${scored.higherIsBetter ? "_min" : "_max"}`];
      if (threshold === undefined) {
        console.error(`${kind}.${scored.metric}: no threshold in the file`);
        worst = 2;
        continue;
      }
      const bad = failed(scored, threshold);
      await db.insert("eval_runs", {
        model: row.model_id, prompt_version: row.prompt_version, grammar_version: row.grammar_version,
        prompt_hash: hash, kind, metric: scored.metric, value: scored.value,
        threshold, passed: !bad, cases: rows.length,
      }, false);
      console.log(
        `${kind}.${scored.metric} = ${scored.value.toFixed(3)} (threshold ${threshold}, ${rows.length} cases) ${bad ? "FAIL" : "ok"}`,
      );
      if (bad) worst = 1;
    }
  }
  return worst;
}

Deno.exit(await main());
```

- [ ] **Step 4: Write `cloud/eval/run_eval_test.ts`** — the scorer, not the network:

```ts
import { assertAlmostEquals, assertEquals } from "@std/assert";
import { type Case, failed, score } from "./score.ts";

function tasks(n: number, theirs: Record<string, unknown>): Case[] {
  return Array.from({ length: n }, () => ({ kind: "task" as const, theirs }));
}

Deno.test("task metrics are mean absolute error and two exact-match rates", () => {
  const cases = tasks(2, { effort_hours: 2, importance: 4, course: "cs-100" });
  const got = score("task", cases, [
    { effort_hours: 3, importance: 4, course: "cs-100" },
    { effort_hours: 2, importance: 2, course: null },
  ]);
  assertAlmostEquals(got[0].value, 0.5);
  assertEquals(got[1].value, 0.5);
  assertEquals(got[2].value, 0.5);
});

Deno.test("a missed obligation costs three times a missed opportunity", () => {
  const obligation: Case[] = [{ kind: "event", theirs: { verdict: "obligation" } }];
  const opportunity: Case[] = [{ kind: "event", theirs: { verdict: "opportunity" } }];
  const missedObligation = score("event", obligation, [{ verdict: "drop" }])[0].value;
  const missedOpportunity = score("event", opportunity, [{ verdict: "drop" }])[0].value;
  assertEquals(missedObligation, 0);
  assertAlmostEquals(missedOpportunity, 2 / 3);
});

Deno.test("a task called information costs three times an information called borderline", () => {
  const task: Case[] = [{ kind: "email", theirs: { tier: "task" } }];
  const info: Case[] = [{ kind: "email", theirs: { tier: "information" } }];
  assertEquals(score("email", task, [{ tier: "information" }])[0].value, 0);
  assertAlmostEquals(score("email", info, [{ tier: "borderline" }])[0].value, 2 / 3);
});

Deno.test("a perfect run scores 1 and an empty set scores nothing at all", () => {
  const cases: Case[] = [{ kind: "event", theirs: { verdict: "drop" } }];
  assertEquals(score("event", cases, [{ verdict: "drop" }])[0].value, 1);
  assertEquals(score("event", [], []), []);
});

Deno.test("one point below a threshold fails and one point above passes", () => {
  const higher = { metric: "weighted_exact", value: 0.749, higherIsBetter: true };
  const lower = { metric: "effort_mae", value: 1.51, higherIsBetter: false };
  assertEquals(failed(higher, 0.75), true);
  assertEquals(failed({ ...higher, value: 0.751 }, 0.75), false);
  assertEquals(failed(lower, 1.5), true);
  assertEquals(failed({ ...lower, value: 1.49 }, 1.5), false);
  // Exactly at the threshold is a PASS: a gate at the observed value fires on noise.
  assertEquals(failed({ ...higher, value: 0.75 }, 0.75), false);
});

Deno.test("a null answer scores as wrong rather than throwing", () => {
  const cases: Case[] = [{ kind: "event", theirs: { verdict: "obligation" } }];
  assertEquals(score("event", cases, [null])[0].value, 0);
});
```

- [ ] **Step 5: Write `cloud/eval/thresholds.json`** with **placeholder-free but provisional** numbers, and replace them in step 7 from the first real run:

```json
{
  "task": { "effort_mae_max": 1.5, "importance_exact_min": 0.55, "course_exact_min": 0.85 },
  "event": { "weighted_exact_min": 0.75 },
  "email": { "weighted_exact_min": 0.70 }
}
```

- [ ] **Step 6: Load the seed and dry-run.** `--load-seed`, then `--dry-run --thresholds cloud/eval/thresholds.json`. Expected: every metric at or near 1.0 (the dry run replays each case's own recorded answer), exit 0, and one `eval_runs` row per metric. **This is the harness's own test, and it costs nothing.**

- [ ] **Step 7: Run it for real, once, and record the numbers.** This is the only step in the plan that spends money at scale — one model call per seed case, at roughly $0.0014 each (the arithmetic is in `judge_caps.ts`), so **record the case counts and multiply before running**. Then set `thresholds.json` from what you observed, each threshold **one clear step below** the measured value: a threshold above what the model does blocks every deploy, and a threshold at the measured value fires on noise. Record the observed metrics **in this step's line**.

- [ ] **Step 8: Name hand-off H8** with the exact YAML from *Controller hand-offs*, and the three secret names Quinn sets.

- [ ] **Step 9: Commit.**

```bash
git add cloud/eval/score.ts cloud/eval/run_eval.ts cloud/eval/run_eval_test.ts cloud/eval/thresholds.json cloud/supabase/migrations/20260911000400_eval.sql
git commit -F .git-commit-msg.txt   # "eval: the seed replayed on every prompt or pin change, blocking the deploy on a regression (C2 Task 14)"
```

---

### Task 15: Close

- [ ] **Step 1: The whole gate, twice.** `cargo test --workspace` at 0 warnings, and `deno check` / `deno lint` / `deno test` over `cloud/supabase/` and `cloud/eval/` with C1's config. Record every count. Then `git ls-files --eol cloud/ engine/` and confirm every `.ts`, `.sql`, `.rs` and `.md` is `i/lf` and every fixture is `attr/-text`.
- [ ] **Step 2: `git status --porcelain --untracked-files=all engine/tests/fixtures/`** → empty. The eleven references are untouched (ruling R-3a-24).
- [ ] **Step 3: The account-scoping scan, over the function directories** — Task 2's `judge_db_test.ts` covers `_shared/`; extend it to `cloud/supabase/functions/*/index.ts` so a `db.select` on `gmail_seen`, `gmail_queue`, `google_accounts`, `rules` or `sources` that omits `account_id=eq.` fails the suite. The service role bypasses RLS; this scan is the backstop.
- [ ] **Step 4: `git diff --name-only main...c2-judge`** → confirm every path is inside C2's ownership: `cloud/supabase/functions/{judge-*,ingest-*,events,gmail-*}/**`, `cloud/supabase/functions/_shared/judge_*.ts`, `cloud/supabase/migrations/20260911*.sql`, `cloud/eval/**`, `engine/src/{cloudmodel,judge,enrich,events,coursework,zybooks,vhl}.rs`, `engine/tests/**`, and this plan. **An overlap is a stop, not a rebase**: report it to the controller.
- [ ] **Step 5: The hand-off list.** One section in the task report: H1–H13, each with the exact code from *Controller hand-offs*, the task that needed it, and what breaks without it. Order them: **H7** (config.toml + deno.json — nothing deploys without the eleven `verify_jwt` entries), **H1** (`lib.rs`), **H3** (`ingest.rs` — the amber-tray fix), **H4** (`cli.rs` — the events **and calendar** proxies), **H5** (`scheduler.rs`'s `|| cloud`, after C1), **H6** (`approvals.rs`), **H8** (CI), **H9** (the Google button, in two phases across eight files and three owners: **(a)** the wizard window's `google_connect_url` / `google_connected` / `open_external`, the `WizardPlan` and `VaultPlan` flag, `scaffold::ingest_yaml`'s second `calendars:` entry, the listener on C1's existing `#wiz-google`, and the amendments to C1's `static_assets.rs` assertion and the controller's `scripts/wizard-check.py` walk — **all of (a) or none of it**; **(b)** the console window's `set_google_calendar`, whose settings row is C4's. Deferrable in whole to C4 without blocking anything, because the `calendar_ics` secret address is the fallback. It moves C1's command counts by four, so C1's Task 21 recount is re-run rather than edited). **H11** (`ingest.rs`'s first-run archive — R-OB-3's guarantee, and the test in `cloud_contract.rs` is red until it lands) and **H12** (`approvals.rs`'s `coursework-map` arm — R-OB-1). H2 is "none, deliberately"; H13 is a note that nothing is owed for the coursework split. And state plainly that **`app/src/scheduler.rs`'s judge gate is C1's, not a hand-off** — it is *Interfaces with C1* contract 6.
- [ ] **Step 6: Docs.** `HANDOFF.md` gains a `▶ C2 DONE <date>` block: the ten endpoints live on staging, the pinned model per kind with its sampling and price, the eval thresholds as measured, the Gmail verification state, and **what production still needs**:
  - its own `ANTHROPIC_API_KEY`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` and `SOURCES_ENC_KEY`, and the migrations pushed;
  - the Google OAuth client's production redirect URI, and **two** consent-screen scopes — `calendar.readonly` (sensitive) submitted and cleared first, `gmail.readonly` (restricted) behind verification and the annual CASA;
  - **calendar *write*** — the third scope, asked for only when the student approves their first calendar-event card (VISION: rare writes behind explicit approval). Not built here;
  - **§5.3's *source went quiet*** — `google_accounts.status = 'quiet'` after 14 days with no yield is modelled but nothing surfaces it; the surface is the console's Issues panel, which is C1's;
  - **the device-side class-(c) export** — `export_training_rows` proves the Gmail exclusion over `judgments`, which carries nothing trainable anyway; the real raw content lives on the device, and whoever builds the (c) export in C4 must carry the same `origin` exclusion there;
  - **the vault's `ics_url`** — kept as the offline fallback; C3 or C4 removes it (fidelity ledger);
  - **the Batch API** — 50% cheaper and the twice-daily slot is latency-tolerant for events and email; the first lever to reach for if per-account spend binds;
  - **`coursework` now fetches every zybook including the ignored ones** — routing moved server-side, so the device makes one extra authenticated round trip per ignored book per slot. Cheap, and worth a line because this plan is otherwise careful about what the device does with the student's session.

  `CLAUDE.md`'s engine-command list gains one line for the cloud path of `judge`. This plan's status line becomes **DONE**.
- [ ] **Step 7: Commit.**

```bash
git add HANDOFF.md CLAUDE.md docs/plans/2026-09-09-c2-judge-plan.md cloud/supabase/functions/_shared/judge_db_test.ts
git commit -F .git-commit-msg.txt   # "docs: C2 closed — the judgment service is live on staging and the engine pulls it in the slot (C2 Task 15)"
```

---

## Exit gate

1. `POST /judge-task`, `/judge-event` and `/judge-email` are deployed on **staging**, share one pipeline, and every reply is a verdict that passed the 0.6 floor and the known-course check, or a named refusal. **Every endpoint path is the hyphenated function name** — Supabase routes `/functions/v1/<function-name>`, and the loopback contract test asserts the same spelling the deploy uses.
2. Every judgment writes one `judgments` row naming the model id, the prompt version, the grammar version, the prompt hash and the elapsed milliseconds — and no row anywhere holds a note body, an email body or a prompt. The three tests that prove it (`a_body_token_reaches_no_judgment_row`, `the_judgment_log_never_carries_a_notes_title_or_body_text`, `the message text reaches no queue row and no judgment row`) all pass.
3. `knowlu-engine judge` on a vault with `config/cloud.yaml` enriches through the service, and **exits 0** on every one of: no config, no session, no network, 402, 429, 5xx, timeout — each as a distinct named line on stdout. A 401 or 402 discovered by `CloudModel::probe()` stops the batch and logs as `service unavailable`, not as `model failed`.
4. `cargo test --workspace` is green at **0 warnings**; `oracle.rs` and `surface_oracle.rs` pass **unchanged**; `dependency_boundary.rs` refuses a cloud SDK; `rank_cannot_reach_a_judgment_endpoint` passes; no test opens a socket that is not `127.0.0.1`.
5. `deno test` over `cloud/supabase/` and `cloud/eval/` is green, and the two frozen parsed references pass as the TypeScript parsers' oracle, read from `engine/tests/fixtures/` and unmodified (`git status --porcelain engine/tests/fixtures/` is empty).
6. `/ingest-coursework` reconciles both sources from real captured payloads, and the request body carries no username, no password, no `credential_target`, no `base_url` and no filesystem path.
7. `/ingest-ics` reads C1's `url_ciphertext` / `url_iv`, decrypts with C1's `_shared/crypto.ts`, and that URL appears in no reply, no error body and no log line. With H3 applied, a cloud vault with a blank `ics_url` **reaches the service before it exits 1**.
7a. `/ingest-calendar` resolves **`?name=personal` to the `calendar_ics` `sources` row and `?name=google` to the `google_accounts` grant** — never to a `sources` row of kind `google_calendar`, which nothing writes and `the_google_calendar_is_never_looked_for_in_the_sources_table` pins — and returns ICS either way. The secret address appears in no reply or error body; a missing grant is a 409 and an unknown name a 404. `calfeed` parses what comes back through its own horizon, dedup and snapshot with **no change to `calfeed.rs`** — the `cloud:<name>` branch lives in hand-off H4 — and the `calendar_ics` path still works on a vault with no account at all.
6a. **R-OB-1:** a payload carrying a book or section the mapping does not know yields **one proposal and zero warnings**, a known one yields its items, and zyBooks' own `HowToUseZyBooks2` yields neither. `suggestCourse` answers `cs-100` for `UACS100Fall2026`, `math-125` for `MATH125` and `null` for `HowToUseZyBooks2` and `2102121` — the institution prefix comes off only when a term suffix says there is one. **A card the student rejected is not re-minted while it is unexpired** (`asked_map_keys` scans `approvals/` and `archive/`; `a_rejected_map_card_is_not_re_asked_until_it_expires`), and a card that cannot be applied goes back to `pending` so it ages instead of logging the same line twice a day forever. The card reaches `approvals/`, `rank` leaves it alone (H12), and approving it writes `coursework.<source>.<table>.<key>` into `config/ingest.yaml` before the next fetch — with a blank `course:` refused and the card kept, never a mapping to the empty string. **That write is a text-level insertion** (R-C2-8): three lines beneath the `courses:` / `sections:` line at the file's own indentation, no `serde_yaml_ng::to_string` anywhere near it, a second apply of the same key a logged no-op, and `every_byte_outside_the_inserted_lines_is_unchanged` green.
7b. **R-OB-3:** on a vault with no `today.md` (R-C2-9's predicate — and `first_run_is_the_absence_of_today_md_not_of_ingest_seen` proves it is not the seen-ledger's, which `coursework` writes earlier in the same slot), an item already past due is written to `archive/` with `status: archived` and `archived_reason: imported-past`, is recorded in `ingest-seen`, and appears in no task list — and the **next** run creates a newly-past item as an ordinary task. `ingest_on_a_fresh_vault_archives_past_due_items` passes with H11 applied; `GET /ingest-ics?first_run=1` returns the corroborating `past_due_uids` and returns `[]` without the flag.
8. `/events` fetches a source the desktop could not, refuses every private, plaintext and bare-label URL, and `rank` uses it through the existing `Fetchers.events` seam with no model call anywhere in `rank`.
9. **The first Google consent asks for `calendar.readonly` alone** (a *sensitive* scope: lighter review, no CASA); `gmail.readonly` is a second, optional, incremental consent (`include_granted_scopes=true`) and `google_accounts.scopes` records what was actually granted, with `read_google_grant` refusing a reader whose scope is missing. `calendar_is_asked_for_before_gmail_and_never_together` passes. The refresh token exists only in Supabase Vault; disconnect revokes at Google (every scope at once — there is no partial revoke) before it forgets the row; attachments are never fetched; the Gmail read stops at its own wall-clock budget and says `more`; every derived row carries `origin = 'gmail_api'`, and `export_training_rows` excludes them. **Testing-mode limits (100 test users, 7-day tokens) are stated for both scopes**, in the wizard copy of hand-off H9 and in the Gmail step. `GET /google-connect?status=1` answers `{connected, scopes}` for the wizard's poll, mints no nonce, and distinguishes a Gmail-only grant from a calendar one.
10. `promote_rules()` runs without error on staging and returns a number; no `scope = 'global'` row is active, and no code path can activate one. A `kind: rule` card can be filed, answered in the deck, sent once and archived.
11. The eval suite runs on the **frozen seed** (and on any correction that arrived with a replayable `request` — none until C4, by ruling R-C2-4), writes `eval_runs` rows with the threshold each was measured against, and exits non-zero below one. `cloud/eval/seed/` passes its scrub test and names no person. **Overridden 2026-09-14 (ruling R-C2-E12): there is no frozen seed — Quinn declined the archive read entirely. The suite runs on `0 cases`, `run_eval.ts` exits 0, and the gate is vacuously green — by design, not omission — until consented corrections (C4's (c) toggle) fill `eval_cases`.**
12. The daily caps, the monthly ceiling and the model's price were chosen **together**, the arithmetic is written down beside `DAILY_CAP`, `usage_daily` records real token counts, and `enforce_budget` refuses a call rather than reporting one after the fact.
13. Every hand-off H1–H13 is listed in the final report with exact code, `app/src/scheduler.rs`'s judge gate is stated as **C1's dependency and not a hand-off**, and **no commit on `c2-judge` touches a file outside C2's ownership** (`git diff --name-only main...c2-judge` proves it).

## What is NOT in this plan

- **Accounts, Auth, Stripe, entitlement, the wizard (including the *Connect your calendars* panel and both `sources` rows it writes), the LMS link capture, telemetry ingest, issue reports, the privacy policy and ToS pages, the Google consent-screen submission, `cloud/supabase/{config.toml,deno.json}`, `_shared/crypto.ts`, `_shared/entitlement.ts`, and `app/**` — all C1.** C2 imports two of C1's `_shared` modules, reads C1's `sources` and `corrections` tables, and supplies hand-offs for the two C1 files it needs lines in. C2 never edits `app/`.
- **Journal sync — C3.** Delivery in C2 is always a pull by the engine inside its slot steps. Nothing here waits on C3, and C3 generalises the pull rather than replacing it.
- **Removing the local llama.cpp runtime — C4.** `engine/src/runtime.rs` and `app/src/inference.rs` are left exactly as they are; `judge` simply prefers the cloud when `config/cloud.yaml` exists. Ruling R-P3a-2's `SUPPORTED_RUNTIMES` digest table is deliberately retired rather than replaced (see the fidelity ledger), and C4 deletes its carrier.
- **Telemetry class (c)'s toggle and its UI — C4.** Until it ships, the eval suite is seed-only (ruling R-C2-4), and `corrections` still feeds rule promotion and the correction-rate metrics for every account.
- **The settings panel's *Connect / Disconnect Google* row — C4** (ruling R-X-15 phase b), beside the (c) toggle it belongs next to. C2 ships the command it will call — `set_google_calendar(vault, connected)`, hand-off H9(b) — and no caller for it, because there is no such row in C1's settings panel and inventing one is a UI decision C2 is not making. **What happens meanwhile is bounded and visible, not a gap:** a student who revokes the grant at Google leaves `cloud:google` in `calendars:`; `/ingest-calendar?name=google` then answers **404**, because `read_google_grant` returns nothing for a revoked or scope-less grant; `calfeed::load_calendar_events` turns that per-feed failure into `google: fetch failed (…); using snapshot` and keeps the last known busy time; and the same slot's `judge` step prints the quiet source. The day still ranks, the line says why, and C4's row removes the entry when it lands. **Connecting** a Google calendar after onboarding is the same row and the same C4 — during onboarding the wizard's own path (H9 phase a) covers it.
- **Email ingestion by forwarding** (a Cloudflare Email Routing inbox). Documented in §13 as the fallback if Google refuses verification; not built. `judge::EmailModel` is the seam it would use unchanged — which is also why that trait exists although the Gmail path judges server-side.
- **The Batch API, prompt caching, and a second model tier.** All three are cost levers, all three are recorded in `judge_anthropic.ts` with the reason they are not used yet, and the first one to reach for is Batch.
- **Calendar *write*.** `calendar.readonly` is the only calendar scope C2 asks for. Write is a third, later consent, requested only when the student approves their first calendar-event card (VISION: rare writes behind explicit approval), and nothing here builds it.
- **R-OB-2 — seeding `courses/` and `course_map` from the enrolled course list the sign-in window captures — is C1's**, and part of its sign-in spike's go/no-go. C2 builds none of it. Until it lands a fresh vault's Blackboard tasks arrive `course: null` with `needs_enrichment: true`, which is the accepted judgment gap and which `/judge-task` is the answer to: `Heuristics::knows_course` degrades to *no course* rather than a wrong one, and the enrichment names the course in `importance_reason` when it cannot place it.
- **Reading grades from the signed-in LMS session** (§11a, "wanted", not designed): it waits on the university-policy read and on C1's link capture proving out.
- **The console's visual redesign, mobile, a second campus, production go-live.** Task 15 records production's list.

### Where this plan narrows the spec, and the recommendation

Four places, all in the fidelity ledger as rows, and three of them share one argument: **§4.3's reason for moving a parser is that a vendor changes its markup under us, and that is true of zyBooks and VHL and of nothing else in this system.**

**One. §3.1 lists event feeds as "fetch + roster" in the cloud.** Task 9 moves the fetch — which is the sentence's own stated payoff, *"HTML sources finally work"* — and the **verdict**, which is what D3 is actually about, and leaves the roster on the device. `eventfeed`'s four parsers, `prefilter_events`, `write_roster`, `emit_digest` and `relevant_events` are ~1,400 lines under the golden `today.md` oracle with **no frozen parsed reference of their own**; porting them would be the one parser move in this plan measured against nothing. Nothing about this forecloses moving the roster later; C3's sync is when it would become natural.

**Two. §3.1 lists the LMS `.ics` "fetch + parse" as cloud.** Task 8 moves the fetch and leaves the parse on the device. Three reasons:

1. The reason §3.1 gives is *"needs the internet, no credential (a capability URL the user pastes once into their account, stored server-side encrypted)"* — that is an argument about **the URL**, and the URL does move.
2. §4.3's actual argument for moving a parser names zyBooks and VHL, whose HTML and JSON change under us. iCalendar is a twenty-eight-year-old RFC; `ingest::parse_ics` has not needed a change for a vendor in this project's history.
3. `ingest::parse_ics` and the `sync_tasks` reconciliation behind it are covered by the **golden `today.md` oracle on three fixture vaults**, and there is no frozen parsed reference for them.

**Recommendation:** leave both as they are. If Quinn wants the ICS parse moved too, the honest prerequisite is a frozen `blackboard-parsed-reference.json` generated from the Python engine in the archive — the same one-time archive read Task 13 already has approval for, at the cost of one extra extraction. Ask at Task 8, not before; the answer changes one task and nothing else.

**Three. §5.4 measure 2's "last 90 days of corrections plus a frozen seed"** becomes seed-only until C4, by ruling R-C2-4, because §6 (b) carries no body and §5.2 discards bodies after the call. **Recommendation:** as ruled. The alternative was storing raw content without consent, and Quinn can reverse it by amending D5.

**Four. §5.3's "the service counts what it has queued so it never proposes past the cap"** stays the engine's `defer_over_budget`, and **"user-excludable labels"** ships as a schema seam with no UI. Both are ledger rows. **Recommendation:** as written for the cap (one counter, and only the device can see hand-made cards); for the labels, the control belongs in C1's settings panel and is one UI change away.

## Deferred minors

Every minor from `docs/reports/2026-09-09-c2-judge-plan-review.md` that has **not** been fixed, with the reason. Everything not listed here was fixed. **The re-review of fix round 1 accepted all three deferrals** and both of round 1's unresolved items — one outright (*source went quiet*), one with the correction that became hand-off H9's five parts.

| # | Minor | Why it is deferred |
|---|---|---|
| M8 | The commit trailer hard-codes `Co-Authored-By: Claude Fable 5.1` | **Ruled not a defect** (R-C2-6): the trailers name the model and session that *execute* the plan, and the literal is the writing session's. Global Constraints now say so in one clause, which is the whole fix the ruling asks for. |
| M12 | `export_training_rows` filters a table that carries nothing trainable | True, and the fidelity row is satisfied in letter. The property is worth pinning anyway — the day someone adds a column to `judgments` the filter is already there — and the real class-(c) export is on the device and out of C2's scope. **Recorded in Task 15's *what production still needs*** so whoever builds it in C4 carries the same exclusion. |
| M13 | `fetch_payloads` fetches every zybook, including the ignored ones | One extra authenticated round trip per ignored book per slot, against a portal that has one such book. Fixing it means sending the `ignore` list to the device's fetch half, which puts routing back on the device — the thing §4.3 moved. **Recorded in Task 15's *what production still needs*** with the cost, so it is a decision and not an oversight. |
| — | The `judgments` `EXECUTE`-to-`PUBLIC` note in *Risks the plan under-weights* | Addressed rather than deferred, but worth naming: every C2 function now pins `search_path` and the three Vault helpers are `security definer` with `revoke execute … from public`. `charge_call`, `record_tokens`, `enforce_budget`, `promote_rules`, `judgment_features`, `export_training_rows` and `backfill_correction_judgments` are `security invoker`, so RLS still blocks `anon` and `authenticated` even though `EXECUTE` is granted. |
