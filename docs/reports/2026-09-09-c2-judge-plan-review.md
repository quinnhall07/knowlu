# C2 judgment service plan — review (2026-09-09)

Reviewer: Claude (opus), independent of the writer. Passes over the plan: 3.

Pass 1 was a full sequential read of all 5,908 lines. Pass 2 checked every claim the plan makes
about existing code against `engine/src/{judge,enrich,judgelog,runtime,events,eventroster,eventledger,coursework,zybooks,vhl,ingest,write,approvals,cli,wincred,ledger,pystr,ids,lib}.rs`,
`engine/tests/dependency_boundary.rs`, `engine/Cargo.toml`, the workspace `Cargo.toml`,
`app/src/scheduler.rs`, and against `docs/plans/2026-09-09-c1-accounts-plan.md` (the parallel
stream this plan contracts with). Pass 3 was consistency and arithmetic: endpoint paths across
tasks, table/column names across tasks, type signatures across sections, and the cost numbers.

The Anthropic surface (model ids, pricing, structured outputs, `temperature`, thinking,
`max_tokens`, timeouts) was checked with the `claude-api` skill rather than from memory. What I
checked and what it says is recorded under **Anthropic API — checked** below.

## Verdict

**NEEDS FIXES** — the plan's first eight tasks are excellent and largely executable as written, but
four defects block execution (a wrong endpoint path in the one call that closes the judgment gap, a
`sources` column C1 does not create, an `ingest` early-return that turns H6 into a permanently amber
tray, and an eval suite whose correction half has no data source), and Tasks 9–14 are prose where
the house standard is code.

## Fidelity to the signed spec

| Decision / ruling | Verdict | Where (task, line) | Note |
|---|---|---|---|
| **D3** all three judgments in the cloud, no local runs, cheapest model | honoured | ledger `plan.md:58`; Tasks 2–4, 9, 11 | `models` seeded `claude-haiku-4-5` (`:791-793`), one endpoint per kind, tier 3 is HTTP. The *mechanism* for "cheapest that passes the eval" is broken twice over — see Critical 4 and Important 4. |
| **D3a** the judge seam stays; only tier 3 changes | honoured | `:59`, Task 4 `:2286-2401` | Verified against `engine/src/judge.rs`: `Model`, `judge_task`, `parse_reply`, `tier1`, `merge`, `GRAMMAR`, `prompt_for` are untouched; the three `judge.rs` edits are additive; `Missing` stays `Copy` because `&'static str` is; `judgelog::entry_for` (`engine/src/judgelog.rs:73-76`) already has a `_ => None` arm, so the "`judgelog.rs` deliberately unchanged" claim (`:148`, `:2041`) holds and compiles. |
| **D4** deterministic core stays, internet-dependent work moves | honoured, narrowed twice, both recorded | `:60`, `:86-88`, `:5895-5907` | The two narrowings (ICS parse stays; event roster stays) are argued in *Where this plan narrows the spec*, exactly as R-C2-1 and R-C2-2 require. The event narrowing has a consequence the plan did not follow through — Important 5. |
| **D5(b)** corrections are the eval material | **contradicted in effect** | `:61`, `:180-191`, `:5743` | The `corrections` table has no request column, `judgments` deliberately has none, and `JudgeReply` (`:1655-1663`) carries no judgment id — so no correction can become a replayable `eval_cases.request`. Critical 4. |
| **D6** rule promotion, eval suite, grammars on every call, pinned models + prompt hash | partly honoured | `:62` | Grammars: honoured (`output_config.format` json_schema on every call, `:609`; validated at `:1077`). Pinned models + hash: honoured in the row, weakened by `promptHash` hashing a default rendering — Minor 1. Rule promotion: does not run — Important 6. Eval suite: half unbuildable — Critical 4. |
| **D11** portal scraping stays on the device | **honoured, strongly** | `:63`, Task 7 `:3899-3920`, `:3838-3862` | `redact()` is an allowlist, not a denylist; `the_coursework_payload_carries_no_credential` asserts both directions. `zybooks::signin` / `vhl::login_and_fetch_dashboard` stay on the device with `wincred`. No portal password can reach a server on this design. |
| **D12** Gmail OAuth, server-side, token never on the device | honoured | `:64`, Tasks 10–11 | `gmail.readonly` only (`:4817`, asserted `:4765-4767`); refresh token to Vault, `gmail_accounts` keeps only `secret_id` (`:4688-4696`); `gmail_queue.payload` is the verdict, never text (`:4711-4719`); `pull_gmail_queue` (`:5200-5219`) reads only verdict fields. **No Gmail text reaches the device.** Two spec details dropped silently — Important 12. |
| **D2** account required; entitlement is "subscription active" | honoured | `:65`, every handler's first line | `requireActiveEntitlement` called first and never re-implemented; the 402 becomes `no entitlement` and exit 0 (`:2543`, `:2206-2218`). |
| **D7** Cloudflare + Supabase, no Firebase | honoured | `:66` | Everything under `cloud/supabase/`. |
| **§10** "Nothing model-shaped ships in the app" | honoured | `:67`, `:2801-2825` | Prompt, schema and model id are server-side; the `dependency_boundary.rs` pin is real and would pass — I checked both manifests for every forbidden substring. |
| **§10** "Credentials for the student's own portals never leave the student's machine" | honoured | `:68` | As D11. |
| **§11 R5** global rules hand-reviewed before activation | honoured | `:70`, `:822-824`, `:5449-5451` | `promote_rules` only ever writes `scope = 'account'`; no code path sets `active = true` on a global row. (The `source` feature it would key on is broken — Important 5 — so the guarantee is currently vacuous for events.) |
| **§11 R6** staging only | honoured | `:71`, `:30`, every deploy step | No production ref anywhere. |
| **§11 R8** Anthropic as launch provider; the eval picks the model | honoured in structure, broken in mechanism | `:72`, `:69` | One SDK import site, provider behind `JudgeModel`. But `temperature: 0` is hard-coded (Important 4) and the gate's trigger misses a pin change (Important 14), so "the eval picks the model" cannot actually be exercised. |
| **§5.2** confidence floor 0.6, known-course check, no body in the log | honoured | `:73`, `:1041`, `:1097`, `:724-738`, `:1205-1219` | The `judgments` type has nowhere to put a body and a static test pins the column names. `a_body_token_reaches_no_judgment_row` is a real end-to-end tripwire. Verified the device twin (`enrich.rs`'s existing privacy test) survives untouched. |
| **§5.2** daily caps, per-call timeout, monthly budget alert | **weakened** | `:74` | Caps and timeout: present. The monthly budget alert is a view over columns nothing writes, with a fan-out join and no reader — Important 2, and the caps/price arithmetic is Important 3. |
| **§5.3** `origin = 'gmail_api'`, (c) export filter, text discarded | honoured in letter | `:75`, `:4738-4744`, `:5060-5066` | The filter is over `judgments`, which by design carries nothing trainable — Minor 12. Text-discard is real. |
| **§5.3** the service counts what it queued so it never proposes past 15/day; user-excludable labels | **dropped, unrecorded** | `:4963`, `:5081` | Important 12. Both may be defensible narrowings; neither is in the fidelity ledger or in *Where this plan narrows the spec*, where the plan's other two narrowings correctly are. |
| **R6 (3a)** prompt carries the note and its grounding and nothing else | honoured, extended | `:76`, `:1264-1279`, `:2154-2175` | Both the server-side prompt test and the device-side HTTP-body test exist. `task_request` (`:2657-2690`) sends `course_map` **slugs** and never its keys — the right call, and asserted. |
| **D5 (3a)** "local models, never a cloud API" | deliberately reversed, on the record | `:77` | Correct handling. |
| **R-P3a-2** no unverified path to executing a runtime | retired, on the record, with a replacement property | `:78` | The replacement (endpoint from the vault's own config; rustls; no cloud SDK; the token never in a log) is stated and mostly tested. The "no path under `cli.rs` reaches `/judge/*`" half has no guard — Important 15. |
| **R-3a-20** structural `LowCause`, closed vocabulary to the log | honoured, with one overstatement | `:79`, `:2537-2549` | `CloudError::label()` is a closed set. But a 401/402 discovered mid-batch reaches `judge_task` as `ModelError::Failed` → `LowCause::ModelFailed`, so the *device log* still records "model failed" for an auth failure. Only stdout says "no session". Minor 14. |
| **R-3a-21** end-to-end `fields` privacy check | honoured | `:80` | Device test survives; server twin added. |
| **R-3a-25** "does the failure text survive this boundary?" | honoured | `:81`, `:2191-2244` | The 401/402/5xx/transport tests are the right four, and `the_session_token_never_reaches_an_error_string` is the one that matters. `zybooks::scrub` signature verified. |
| **R-3a-12** no engine command against the worktree or a real vault | honoured | `:25`, `:82`, `:2247-2252`, `:4095` | Every test copies a fixture to a temp dir. |
| **CLAUDE.md** every note write through `write`; judge-once; 15/day cap | honoured | `:83-84` | `write::create` / `write_literals` signatures verified against `engine/src/write.rs`. |
| **§8** `dependency_boundary.rs` pins no cloud SDK, `ureq` only | honoured | `:85`, `:2810-2825` | Checked: none of the eleven forbidden substrings appears in `engine/Cargo.toml` or the workspace root, so the test passes today and fails on the day someone adds one. |
| **R-C2-1** ICS fetch moves, parse stays; frozen reference is the prerequisite if it ever moves | honoured | `:87`, `:5901-5907` | Carried consistently in the ledger, the task and *What is NOT in this plan*, with the `blackboard-parsed-reference.json` prerequisite named and the question deferred to Task 8. Exactly as ruled. |
| **R-C2-2** event fetch + verdict move; roster stays until C3 | honoured | `:88`, `:5899` | Carried consistently. The consequence for the data the verdict is computed from was not followed through — Important 5. |
| **R-C2-3** H6 is C1's dependency, not a controller hand-off | **contradicted** | `:283`, `:354`, `:2024`, `:2868`, `:2877`, `:5861`, `:5885`; absent from *Interfaces with C1* `:152-191` | Important 1. |

## Findings

### Critical (blocks execution)

**C1. `plan.md:2729` (and `:2167`) — the engine posts to `/judge/task`; the function deployed is `judge-task`.**
Every other endpoint in this plan uses the hyphenated function name: `/ingest-coursework` (`:4031`),
`/ingest-ics` (`:4249`), `/judge-event` (`:4496`), `/events` (`:4518`), `/gmail-read` (`:5201`); and
Task 3's own smoke check curls `$SUPABASE_URL/functions/v1/judge-task` (`:1994`). Supabase routes
`/functions/v1/<function-name>[/<subpath>]`, so `POST …/functions/v1/judge/task` reaches a function
named `judge`, which nothing in this plan deploys — a 404. *Why it matters:* the loopback contract
test at `:2167` asserts the wrong path too, so `cargo test` goes green and the failure only appears
against staging; Task 4 is the task that "closes the judgment gap" (`:2877`) and it would not.
*Fix:* `self.call("/judge-task", …)` at `:2729` and `POST /functions/v1/judge-task` at `:2167`.

**C2. `plan.md:175` and `:4229-4233` — `/ingest/ics` reads a `sources.url` column C1 does not create.**
The C2 contract text states C1's table as `(account_id uuid, kind text, url text, added_at timestamptz)`
with `url` "encrypted at rest", and `ingest-ics/index.ts` does
`db.from("sources").select("url")… return data.url as string`. C1's plan actually creates
`public.sources(account_id, kind, url_ciphertext text, url_iv text, added_at)`
(`docs/plans/2026-09-09-c1-accounts-plan.md:826-838`): AES-GCM ciphertext plus IV, written by C1's
`_shared/crypto.ts` (`encryptString`/`decryptString`/`importAesKey`, C1 plan `:3080-3090`) with the
key held as a function secret. *Why it matters:* `select("url")` errors, the handler returns a
named 404 for every account, and Task 8 ships a feature that never works — and the plan's
preconditions never ask Quinn for the AES key secret. *Fix:* restate contract 3 with the real
columns, import C1's `decryptString`/`importAesKey` in `ingest-ics/index.ts`, and add the key's
secret name to the P-table beside `ANTHROPIC_API_KEY`.

**C3. `plan.md:238` and `:320-322` versus `engine/src/ingest.rs:742` — H6 makes `ingest` run on a cloud vault, and H3 cannot rescue it.**
`ingest::run_lines` returns `(1, vec!["ingest: no ics_url configured"])` at `engine/src/ingest.rs:742`,
*six lines before* the `let fetched = match fetch { … };` at line 748 that H3 replaces. H6 changes
`slot_argv` to `if has_ics_url(vault) || cloud` (`:320`), so a cloud vault with an empty `ics_url`
runs `ingest`, hits the early return, and exits 1 — never reaching `/ingest/ics`. The plan asserts
the opposite at `:354`: "With H3 in place `ingest` exits 1 on an empty `ics_url` only when the
service call *also* failed." *Why it matters:* a non-zero step sets `RunSummary.engine_ok = false`,
which is retry backoff and an amber tray twice a day forever — the precise failure the plan's own
Global Constraints forbid (`:20`). This is reachable whenever C1's LMS link capture fails and the
paste-a-link fallback is skipped. *Fix:* H3 must relocate the empty-URL early return below the
cloud attempt (and H3 is `ingest.rs`, a controller file, so the code must be exact), or H6 must not
add `|| cloud`.

**C4. `plan.md:5743` and `:5761` — the eval suite's correction half has no data source.**
§5.4 measure 2's labelled example is `(the request we sent, the verdict we gave, the value the human
set)`, and `eval_cases` carries `request jsonb`. But `corrections` (`:180-191`) has `ours`/`theirs`
and no request; `judgments` (`:799-816`) deliberately carries no prompt, title or body — that is the
point of the table; and `JudgeReply` (`:1655-1663`) carries no judgment id, so even
`corrections.judgment_id` has no producer anywhere in C1 or C2. There is therefore no path by which
a correction becomes a replayable eval case. *Why it matters:* exit-gate item 11 (`:5884`) and the
deploy gate both claim "the last 90 days of corrections plus the seed"; in reality only Task 13's
frozen seed can ever populate the suite, so the gate never learns anything from real use — the
single deterministic measure D6 leans hardest on. *Fix:* decide where the replayable request comes
from — the device can re-derive it from the note at telemetry time, or the service can store a
content-addressed request blob outside `judgments` — and write it into the C1 telemetry contract;
or, if that is deferred, narrow §5.4 measure 2 in the fidelity ledger to "seed only until C3" as a
recorded decision rather than an exit-gate claim that cannot be met.

### Important (fix before execution)

**I1. `plan.md:283` — H6 is filed as a controller hand-off, and it contradicts C1.**
R-C2-3 rules that `app/src/scheduler.rs` is C1's and that C2 must state the dependency under
*Interfaces with C1*. The plan instead lists it as hand-off **H6** at `:283`, repeats it as a
hand-off at `:2024`, `:2868`, `:2877`, `:5861` and in exit-gate item 12's "H1–H7" (`:5885`), and
never mentions `scheduler.rs` in *Interfaces with C1* (`:152-191`). Worse, C1's plan already owns
the file (`c1-accounts-plan.md:40`, `:132`) and its Interface 4 (`c1-accounts-plan.md:165-175`)
fixes the argv as `knowlu-engine judge --vault <vault> --via local-runner` **"and nothing else"**,
where C2's H6 adds `--log-dir <judgments_dir>` (`:326`); C1 also gates the step on
`entitlement_state()` with the 72-hour grace and a `judge (skipped: no entitlement)` step, which
`JudgeState::Cloud` does not model. *Fix:* delete H6, renumber to H1–H6, add a fifth entry to
*Interfaces with C1* stating the dependency and reconciling `--log-dir` (either C1 adds it or C2
drops the device judgment log on the cloud path), and correct the five downstream references.

**I2. `plan.md:5769` and `:1738` — the monthly budget guard is inert, and its arithmetic is wrong.**
`monthly_spend` sums `usage_daily.in_tokens` / `out_tokens`, which nothing ever writes:
`charge_call` (`:885-899`) increments `calls` only, and `judge_pipeline.judge` discards
`answer.inputTokens` / `answer.outputTokens` (`:1738-1744`). The view therefore always reports `$0`.
Even with tokens recorded, the view joins `judgments j` to `usage_daily u` on `(account, kind, day)`
and then sums `u.in_tokens` — a fan-out that multiplies each day's usage row by that day's judgment
count. And nothing at runtime reads `thresholds.json`'s `monthly_usd_per_account_max` or
`monthly_usd_global_alert`; `run_eval.ts` is the only reader and only for quality metrics. §5.2's
"monthly inference budget alert per account and global" is not implemented. *Fix:* have
`judgmentSink`/`charge_call` record the two token counts, drop the `judgments` join from the view
(it needs only `usage_daily` and `models`), and give the alert an actual reader.

**I3. `plan.md:1479` and `:5757` — the caps and the budget threshold were chosen independently, and neither is enforced.**
`DAILY_CAP` is 200 + 300 + 500 = 1,000 calls per account per day. At `claude-haiku-4-5`'s published
$1.00 / $5.00 per MTok (confirmed via the `claude-api` skill) and this plan's own bounds — system
prompt ~280 tokens, user prompt up to ~700 (body 1200 chars + weights 600 + preferences 600 + title
200), `max_tokens` 256 — one call costs roughly $0.0015, so the caps permit **~$45 per account per
month against a $9.99 subscription**. Meanwhile `thresholds.json` sets
`monthly_usd_per_account_max: 0.75`, i.e. about 500 calls a month; a plausible real user (≈10
enrichments, ≈20 event verdicts, ≈30 emails a day) spends ~$2/month and trips that threshold four
times over. *Why it matters:* the caps are described as "far above real use … they bound a runaway
loop" (`:1477`), which is true, but with no spend-side enforcement the runaway loop they bound is
still 4.5× the subscription price. *Fix:* pick the numbers together — either lower the caps to
something the price supports, or make the budget an enforced ceiling (I2) rather than a view — and
record the arithmetic in the plan so the next reader can check it.

**I4. `plan.md:606` — `temperature: 0` is hard-coded, and it is a 400 on every model above Haiku.**
Checked with the `claude-api` skill: `temperature`, `top_p` and `top_k` are **removed and return a
400** on Claude Sonnet 5, Opus 5, Opus 4.8, Opus 4.7 and Fable 5/5.1; they remain valid on Haiku 4.5
and the 4.6 generation. The plan pins `claude-haiku-4-5` today, so this works — but Task 14 is
explicitly the only thing allowed to change the pin (`:72`), and §11 R8 says "the eval suite picks
the model, not taste". The moment the eval picks anything above Haiku, every call 400s and every
judgment becomes `model failed`. *Fix:* make `AnthropicModel` send `temperature` only for models
that accept it (a small allowlist, or drop it entirely and rely on the JSON schema plus
`output_config.effort`), and add a contract test that the pin and the sampling parameters are
compatible.

**I5. `plan.md:4420-4441` versus `engine/src/eventroster.rs:250-306` — `judge_roster` judges events from a source that does not carry what the prompt needs.**
`eventroster::read_roster` reconstructs only `uid`, `title`, `start`, `end`, `organizer`, `location`,
and hard-codes `source: "roster".to_string()`. Everything else comes back `Default::default()`. So
the `EventItem` the plan builds at `:4428-4442` has an empty `description`, empty `categories`, empty
`audiences`, empty `series_uid`, empty `url`, and `source == "roster"` for every event on every
account. Consequences: (a) the `/judge/event` prompt (`:1404-1417`) is written around exactly those
fields and loses all of them; (b) `judge_rules.features` for events (`:1593-1598`) keys on
`organizer`, `source`, `series`, `title_prefix` — two of the four are always empty and `source` is
the same constant for everyone, so per-account promotion is crippled and §11 R5's "same campus
source across ≥ 20 accounts" is meaningless. The engine's own comment says the unjudged roster line
carries a description excerpt *because* it is the judge's input
(`engine/src/eventroster.rs:110-116`), but `read_roster` never parses that continuation line.
*Fix:* either extend `read_roster` to recover the continuation line and the real `source`, or have
Task 9 judge from the events `rank` fetched rather than from the roster it wrote — and add a test
that asserts a non-empty `description` and a real `source` reach `event_request`.

**I6. `plan.md:5452-5495` — `promote_rules()` will not run, and would not fire if it did.**
Five separate problems in one function: (a) `min(j.fields)` at `:5463` — PostgreSQL has no `min()`
aggregate for `jsonb`, so the function errors on first call; (b) `select cron.schedule(…)` at
`:5495` with no `create extension if not exists pg_cron` anywhere in the migration; (c) the
`contradicted` guard at `:5481` joins only on `account_id` and `kind`, so a *single* correction of
kind `task` in 60 days blocks *every* task-rule promotion for that account — for any active user
that is permanently true, and §5.4 measure 1 asks for "no disagreement" on **that feature**, not on
the kind; (d) `judgment_features(p_kind, p_item_id, p_fields)` never uses `p_item_id` (`:5508`);
(e) `rule_evidence` (`:847-853`) is created in Task 1 and written by nothing in the plan, so the
"why a rule was promoted" record does not exist. *Why it matters:* rule promotion is the loop that
retires model calls, which is both a D6 measure and the only structural answer to I3's cost
problem. *Fix:* replace `min(j.fields)` with a deterministic pick (`(array_agg(j.fields order by
j.judged_at))[1]`), enable `pg_cron`, join `contradicted` on the feature and value, and either
populate `rule_evidence` or drop the table.

**I7. `plan.md:5527` — Task 12 changes `judge_pipeline.ts` in prose, with no code.**
"Task 12's first change is to `judge_pipeline.ts`: `fieldsOf` gains the feature columns from
`features(kind, item)`". `fieldsOf` currently takes only the verdict (`:1676`), so this changes a
signature and both call sites (`:1719`, `:1762`), imports `features` from `judge_rules.ts` into
`judge_pipeline.ts` (which the module-graph note at `:949` says nothing about), and changes what
lands in `judgments.fields` — the table the privacy test guards. A step that names a change of that
size and does not write it is the placeholder the plan's own standard forbids. *Fix:* write the
new `fieldsOf`, the new call sites and the extended privacy test as code.

**I8. `plan.md:3641` versus `:4073` — `zybooks::fetch_payloads` has two different signatures in one task.**
The Interfaces block declares `fetch_payloads(cfg: &Mapping) -> Result<Vec<(String, Json)>, SourceError>`;
Step 8 writes `fetch_payloads(email: &str, password: &str) -> …`; and `collect_cloud` calls the
second (`:4008`). *Fix:* delete one. (For the record: `zybooks::signin`, `fetch_zybook_codes` and
`fetch_assignments` are already `pub`, so `fetch_payloads` is a convenience rather than a necessary
exposure — worth saying so in the "two changes only" note at `:142`.)

**I9. Executability degrades sharply after Task 8.**
Tasks 0–8 are genuinely gateable: real code, real test bodies, an expected result on every command.
From Task 9 on, at least eleven artifacts are described rather than written:
`events/index.ts` and `events/handler_test.ts` (`:4618`, one sentence for both);
`gmail-connect/index.ts` and `gmail-callback/index.ts` (`:4917`, one sentence, including the Vault
RPC); `gmail-read/index.ts` (`:5154`); `impl judge::EmailModel for CloudModel` ("built exactly as
`EventModel` is", `:5222`); Task 11's five engine tests (`:5407-5411`, names only); Task 12's
`pull_rule_proposals`, `decide_rule`, `write_rule_card`, `decided_rule_cards`, `existing_rule_ids`
(`:5607-5632`, named not written); Task 12's four engine tests (`:5637-5652`, comment-only bodies);
Task 13's `deidentify.ts` (`:5678-5686`, rules not code); Task 14's `eval_cases` / `eval_runs` DDL
(`:5761`, a prose column list where `migrations_test.ts` will scan the file for
`alter table … enable row level security`); and `run_eval.ts` itself (`:5778`, the task's headline
artifact, described but never written — only the pure `score.ts` is). *Why it matters:* the
comparable house plan (`2026-09-07-knowlu-judge-and-enrichment-plan.md`, 5,856 lines / 12 tasks)
writes full code for every task; a fresh implementer cannot finish these and a reviewer cannot gate
them. *Fix:* bring Tasks 9–14 up to the standard of Tasks 0–8, or split them into a C2b plan
written after the first increment merges.

**I10. `plan.md:1470` (and `:1499`, `:1544`, `:1572`, `:1890`) — the Supabase SDK is imported unpinned, against the plan's own rule.**
Task 0 Step 4 (`:548`) states the rule in bold: "Never `npm:@anthropic-ai/sdk` without a version —
an unpinned specifier makes every deploy a different program." Every `_shared/judge_*.ts` then
imports `npm:@supabase/supabase-js@2`, a major range. *Fix:* pin the exact version the same way,
and record it in the same step.

**I11. `plan.md:5404` — the Task 11 Step 7 test cannot pass as described.**
It asks for a byte-for-byte comparison between the note `write_gmail_note` produces and the note
`approvals::materialize` produces from an approved card. `write::create`
(`engine/src/write.rs:397-411`) mints a fresh opaque `id:` into the frontmatter of every note it
creates, so the two differ by construction. *Fix:* compare everything but `id:` (and say why), or
compare the frontmatter maps rather than the bytes.

**I12. `plan.md:4963` and `:5081` — two §5.3 requirements dropped without a ledger row.**
(a) "the service counts what it has queued so it never proposes past [the 15-a-day cap]" — the plan
relies solely on the device's `defer_over_budget` and says so at `:4963`; that is probably the right
call, but it is a narrowing of a signed section. (b) "user-excludable labels" on the mailbox scan —
`WINDOW = "newer_than:7d"` (`:5081`) has no label handling and nothing mentions the omission.
*Why it matters:* the plan's two *other* narrowings (R-C2-1, R-C2-2) are argued at length in
*Where this plan narrows the spec* precisely so they are decisions and not silences; these two are
silences. *Fix:* add both to the fidelity ledger or to that section.

**I13. `plan.md:3975` versus `:4057-4062` — `collect_cloud` is `#[cfg(windows)]` and its caller is not.**
`main_with_fetchers` (`engine/src/coursework.rs:660`) is not cfg-gated, and the plan's branch calls
`collect_cloud` unconditionally. `coursework.rs` maintains non-Windows stubs for `fetch_zybooks` and
`fetch_vhl` (`engine/src/coursework.rs:532`, `:543`), and the plan itself adds a
`#[cfg(not(windows))] fn session_token` "so a cloud build still compiles" (`:2510-2515`) — so the
plan intends cross-platform compilation and then breaks it. *Fix:* add a non-Windows `collect_cloud`
stub, or cfg the branch.

**I14. `plan.md:393` — the eval gate's trigger misses the change it exists to gate.**
H7 fires only when the diff touches `_shared/judge_(prompts|models|validate).ts` or a migration whose
filename contains `judgment_service` or `model_pin`. But a pin change is "a migration row with a
date" (`:777`, `:1547`) — `20260912000100_pin_sonnet.sql`, say — which matches neither pattern, so
the model swap the gate exists for goes through ungated. *Fix:* trigger on any
`cloud/supabase/migrations/*.sql` that mentions the `models` table, or simply on any migration plus
the three named files.

**I15. `plan.md:256` — `rank` gains reachability into the judgment module with no guard.**
H4 puts `crate::cloudmodel::resolve(vault)` inside `cli::run`, so `rank` now reads the session JWT
out of Credential Manager and links the module that defines `CloudModel: judge::Model`. The Global
Constraint at `:19` says "no path under `cli.rs` may reach `/judge/*`", and I agree the *fetch
proxy* is transport and not judgment — but the constraint is now enforced by convention where it
used to be enforced by the module graph, and the plan adds no test (the way it correctly adds one
for the SDK boundary at `:2810`). *Fix:* one cheap static test — `cli.rs`'s source contains no
`/judge` literal — or move `fetch_ics` / `fetch_event_source` into a transport-only module that does
not define a `Model`.

### Minor (fix in flight or defer)

**M1. `plan.md:1458` — `promptHash` hashes a default rendering, not the template.**
`buildPrompt(kind, {}, {})` renders the task system prompt with `slice_hours` defaulted to 1.5
(`:1370`, `:1375`), so two accounts with different planner slices send different system prompts under
one hash. The doc comment claims "It hashes the TEMPLATE and the SCHEMA — the system prompt and the
JSON schema", which is not quite what it does. Hash the template with the slice as a placeholder, or
put `slice_hours` on the row.

**M2. `plan.md:1707` — the cap is charged before the rule lookup.**
A promoted rule still spends the account's daily allowance, and a capped account loses even the free
tier-2 answers (the `capped` branch returns at `:1709`, before `:1712`). §5.2's own pipeline diagram
has no cap step; the plan's architecture line (`:9`) puts it first. Deliberate or not, say why —
"rules retire model calls" (`:5447`) reads oddly against "rules still consume the model budget".

**M3. `plan.md:1045` and `:806` — `cause: 'unknown course'` is dead vocabulary.**
`validate` drops an unknown slug to `null` and returns `ok` (`:1097`), so nothing ever produces it.

**M4. `plan.md:842` — `rules_one_live_per_feature` enforces neither.**
The unique index includes `version` and does not filter on `active`, so two active rules for one
`(account, kind, feature, value)` are legal as long as their `version` differs. Rename it or add
`where active`.

**M5. `plan.md:785` — `max_tokens: 256` for all three kinds, and `stop_reason` is never checked.**
The email schema (`:1337-1351`) asks for eight fields including a 200-character title and a
140-character why. A truncated reply arrives with `stop_reason: "max_tokens"` and unparseable JSON,
which `AnthropicModel.complete` reports as "did not parse as JSON" (`:621`) and the pipeline logs as
`model failed` — a systematic failure that reads as a provider fault. (The `claude-api` skill warns
explicitly against lowballing `max_tokens`.) Raise the email row's `max_tokens` and check
`stop_reason` before parsing.

**M6. `plan.md:1931` — `Entitle` and `judgeHandler` are exported from a function directory, not `_shared/`.**
`judge-task/handler.ts` is imported by `ingest-coursework` (`:3732`), `ingest-ics` (`:4178`),
`events` (`:4559`), `gmail-connect` (`:4815`), `gmail-callback` (`:4870`) and `gmail-read` (`:5079`),
and `judge-event`/`judge-email` re-export `judgeHandler` from it as a *value* (`:4533`, `:5160`) — so
deploying any of them bundles the judge-task directory. The plan's own rule at `:129` is that
cross-function code lives in `_shared/judge_*`.

**M7. `plan.md:1772` and `:5858` — `deno fmt --check cloud/` will fail on the plan's own code.**
Several lines exceed deno fmt's 80-column default (e.g. the import at `:1152`, the `ROW` literal at
`:1806`). Harmless once `deno fmt` runs, but the committed bytes will not be the plan's bytes and
the "Expected: clean" lines are wrong as written.

**M8. `plan.md:34` — the commit trailer hard-codes `Co-Authored-By: Claude Fable 5.1`.**
`CLAUDE.md` and `HANDOFF.md` §5 say the trailer names *your* model name. An implementer on a
different model would sign it falsely. Write it as `<your model name>` the way HANDOFF does.

**M9. `plan.md:4225` — `ingest-ics/index.ts` calls `serviceClient()` at module scope.**
It throws before `Deno.serve` when an env var is missing, which is a boot failure with an opaque
message rather than the named 503 the plan uses everywhere else (`:4834`). Build it lazily.

**M10. `plan.md:4917` — `gmail_state` is described as "a two-column" table "added to migration 20260911000200".**
It has three columns and is already in that migration at `:4726-4731`. The ten-minute expiry the
comment promises (`:4724`) has no enforcement — no index, no sweep, no check in `takeState`.

**M11. `plan.md:5844` versus `:5741` — `run_eval.ts --load-seed` is not in the task's Interfaces line,**
which lists only `--thresholds <file> [--dry-run]`.

**M12. `plan.md:4738` — `export_training_rows` filters a table that carries nothing trainable.**
`judgments` holds ids, field values and confidences and never a body, so the (c) export filter and
its test (`:5060-5066`) prove a property over rows nobody would train on. The real class-(c) raw
content lives on the device and is not in C2's scope, and nothing there carries an `origin` flag.
The fidelity row is satisfied in letter; Task 15's "what production still needs" should record that
the device-side (c) export still has to carry the same exclusion when C1/C3 build it.

**M13. `plan.md:4073` — `fetch_payloads` fetches every zybook, including the ignored ones.**
Routing moved to the server (`pickZybooks`, `:3742`), so the device now makes one extra
authenticated round trip per ignored book per slot (`HowToUseZyBooks2` on every run). Cheap, but
worth a line since the plan is otherwise careful about what the device does with the student's
session.

**M14. `plan.md:79` and `:2192` — the R-3a-20 claim is half true.**
`a_401_says_it_is_the_session_and_not_a_bad_answer` checks the *error string*. In the real pass a
401 or 402 reaches `judge_task` as `ModelError::Failed` and becomes
`Outcome::LowConfidence { cause: LowCause::ModelFailed }`, so `judgelog` records `model failed` for
an auth failure — the structural cause the split exists to keep honest. `Missing::Service` /
`Outcome::ServiceUnavailable` is only reachable from `run_lines`'s no-session arm (`:2760-2767`),
never mid-batch. The summary line at `:2793` mitigates it on stdout. Either say so, or map a fatal
`CloudError` onto `ServiceUnavailable` inside `CloudModel`.

## Contracts with C1 — checked

| Contract | Verdict |
|---|---|
| `config/cloud.yaml` — four keys, `session_credential_target = knowlu/<profile_id>/session`, absent → named skip and exit 0 | **matches.** C1 plan `:150-156` writes exactly those four keys through `scaffold`; C2 reads them in `cloudmodel::load` (`:2489-2501`) and treats a missing or partial file as `None`. `wincred::read_credential` returns `Credential { username, password: Secret }` with `expose()` — verified; the plan's use is correct. |
| `_shared/entitlement.ts` exporting `requireActiveEntitlement(req): Promise<{account_id}>`, throwing a `Response` (401 / 402), called first | **matches.** C1 plan `:1277` exports exactly that signature and `:186-189` states the same throw contract. Every C2 handler calls it first and wraps it as agreed. C2 correctly does not re-implement it. |
| C1's `sources` table, read server-side by `/ingest/ics` | **does not match — Critical 2.** C1 stores `url_ciphertext` + `url_iv`, not `url`; C2 selects `url`. C2 also needs C1's `_shared/crypto.ts` and the AES key secret, neither of which is named. |
| On-device `ingest` keeps `config/ingest.yaml`'s `ics_url` until `/ingest/ics` exists | **partly matches, unresolved.** C1's Interface 3 (`c1-accounts-plan.md:163`) says "when it ships, **C2** removes the vault copy, not C1". C2's H3 keeps `ics_url` as a permanent fallback (`:216-238`, `:5901-5907`) and never removes it — and could not, since `scaffold.rs` is C1's. Somebody has to own the removal; say which of you, and when. |
| Telemetry (a) and (b) are C1's `POST /telemetry`; C2 owns the `corrections` table and the query | **matches in ownership, broken in substance.** C1 `:184` agrees the split. But `corrections.judgment_id` has no producer (C2 never returns a judgment id) and the eval query needs a request the schema does not hold — Critical 4. |
| Migration numbering: C1 `20260910*`, C2 `20260911*` | **matches** (C1 `:196`). |
| C2 never edits `app/` | **matches in the task text** — no task edits `app/`. But H6 puts C1's file in C2's hand-off list — Important 1. |
| Cloud → device delivery is always a pull; nothing waits on C3 | **matches.** `judge` pulls per item, per Gmail queue and per rule proposal; `coursework` posts and writes the reply; `ingest`/`rank` fetch through the service. No push anywhere, and *What is NOT in this plan* (`:5890`) says so explicitly. |

## Controller hand-offs — checked

| # | File | Complete? | Note |
|---|---|---|---|
| **H1** | `engine/src/lib.rs` | **complete** | One `pub mod cloudmodel;` with a comment; matches the file's existing style and its `judge`/`runtime`/`judgelog` block. |
| **H2** | `engine/Cargo.toml` | **complete** | "None, deliberately" is correct and verified: `ureq 3.4` with `cookies` is already there, `serde_json`, `serde_yaml_ng` are already there, `wincred` is in-crate. `ureq::Agent::config_builder().timeout_global(…).http_status_as_error(false)` is used the same way as `calfeed.rs:542`, `vhl.rs:308`, `zybooks.rs:395`. |
| **H3** | `engine/src/ingest.rs` | **incomplete — Critical 3** | The replacement is exact for line 748 but does not move the `url.is_empty()` early return at line 742, which makes the stated behaviour (`:354`) false and the slot permanently amber on a cloud vault with no `ics_url`. |
| **H4** | `engine/src/cli.rs` | **complete as code, under-guarded** | The `run` body matches `engine/src/cli.rs:218-225` exactly and `Fetchers` has the `events: Option<&dyn Fn(&str) -> Result<String, String>>` field the proxy needs. No guard that `cli.rs` cannot reach `/judge/*` — Important 15. |
| **H5** | `engine/src/approvals.rs` | **incomplete as written** | Placement is right: the chain at `engine/src/approvals.rs:1341-1395` is `calendar-event` / `task` / `amend` / `events-digest` / `else`, and the new arm belongs between the last two. But the snippet at `:273-279` opens `} else if kind == "rule" {` and never closes it — it is a comment with no block terminator, so a controller applying it verbatim gets a syntax error. |
| **H6** | `app/src/scheduler.rs` | **misfiled and contradictory — Important 1** | Should be an *Interfaces with C1* dependency, not a hand-off; and its argv contradicts C1's Interface 4. |
| **H7** | `.github/workflows/ci.yml` | **complete as YAML, wrong trigger** | Both jobs are real and the `--allow-read=.` / `--allow-net=127.0.0.1` scoping is right. The eval gate's file filter misses a pin migration — Important 14. `deno check cloud/supabase/functions/**/*.ts` relies on `**` in a bash `run:` without `shopt -s globstar`, so it degrades to one level — which happens to cover the function directories, but by accident. |

**Nothing hand-off-shaped is hidden inside a task.** I checked every `git add` line: Tasks 0–15 stage
only `cloud/**`, `engine/src/{cloudmodel,judge,enrich,events,coursework,zybooks,vhl}.rs`,
`engine/tests/{cloud_contract,dependency_boundary}.rs`, `cloud/eval/**`, and — Task 15 only —
`HANDOFF.md`, `CLAUDE.md` and the plan file. `engine/tests/dependency_boundary.rs` is an existing
file the ownership list allows ("new files under `engine/tests/`" is narrower than what the plan
does here — it *modifies* `dependency_boundary.rs`); §8 of the spec explicitly asks for that
modification, so I read it as sanctioned, but the ownership sentence at the top of the plan should
say `engine/tests/**` rather than "new files under `engine/tests/`" so the diff check at `:5860`
does not flag it.

## Testing rules — checked

- **`cargo test --workspace` at 0 warnings** — asserted at every task boundary (`:27`, `:2830`,
  `:4086`, `:4650`, `:5858`). Two things would actually break the build: I13 (non-Windows) and, if
  H1 is not applied, `cloud_contract` (the plan handles that at `:2831`).
- **The eight frozen references and the three surface references** — never regenerated, never
  copied. The two parsed references are read by relative path
  (`new URL("../../../../engine/tests/fixtures/…", import.meta.url)`, `:2909`, `:3379`), which
  resolves correctly to the repo root from a function directory. `git status --porcelain
  --untracked-files=all engine/tests/fixtures/` is a step in Tasks 5, 6 and 15 (`:3336`, `:3607`,
  `:5859`). The structural-vs-byte-comparison argument at `:106` is correct and well made
  (`JSON.stringify(1.0) === "1"`), and the Rust byte-for-byte tests stay where they are.
- **`oracle.rs` / `surface_oracle.rs` pass unchanged** — asserted at `:23`, `:2831`, `:4087`,
  `:4651`. Plausible: the coursework split leaves `collect` and `sync_coursework` intact and only
  adds a branch, and `enrich_with` is unmodified.
- **`dependency_boundary.rs` gains the no-cloud-SDK pin** — real and would pass today; I checked
  both manifests for all eleven forbidden substrings.
- **No test reaches the network** — honoured throughout. Rust tests bind `TcpListener` to
  `127.0.0.1:0` and join the thread (`:2060-2114`); Deno tests use `Deno.serve({hostname:
  "127.0.0.1", port: 0})` and `await server.shutdown()` in a `finally` (`:480-511`); the CI job
  passes `--allow-net=127.0.0.1`. The one test that needs *no* listener (`the_session_token_never_
  reaches_an_error_string`, `:2234`) binds and immediately drops to get a dead port — a small race
  on a busy machine, but acceptable.
- **`deno test` per module, thin `index.ts`, one contract test per function** — honoured for
  Tasks 0–8; from Task 9 several `index.ts` files and their tests are prose (Important 9).
  `judge-task`'s five contract tests (`:1828-1876`) are the right five, including "the entitlement
  check runs before anything is parsed or charged" and "a thrown error is a 500 that names nothing
  from the request".
- **No Docker, no live Supabase in a test** — honoured. `migrations_test.ts` (`:690-759`) is a
  static text scan, which is the right call given no Docker, and the plan says so. Nothing runs
  `supabase db reset` or `supabase start` (`:438`).
- **Migrations to staging only** — honoured; every `supabase db push` / `functions deploy` names
  `--project-ref <staging ref>`.
- **`judge` always exits 0** — honoured. `enrich_with` returns `(0, lines)` unconditionally
  (`engine/src/enrich.rs:299`), `run_lines`' three arms all route through it, and Task 4 step 8 adds
  the end-to-end proof. **But `ingest` does not** — and H6 now runs it on cloud vaults (Critical 3).
- **`rank` never calls a model** — honoured in behaviour, unguarded in structure (Important 15).
- **No secret anywhere** — honoured. Only names appear: `ANTHROPIC_API_KEY`, `GOOGLE_CLIENT_ID`,
  `GOOGLE_CLIENT_SECRET`, plus the three GitHub secrets in H7. Test fixtures use
  `"test-key-not-a-secret"`, `"jwt-not-a-secret"`, `"anon-not-a-secret"`. Quinn sets every value
  from his own shell. **One gap:** Critical 2's AES key secret is needed and never named.
- **Does the eval gate really block a deploy?** Not reliably. It is a PR-only job (`:382`) with a
  file filter that misses a pin migration (Important 14), it depends on branch protection that C0
  must configure, and — decisively — it has no correction-derived cases to run (Critical 4), so on
  day one it scores only the frozen seed and `thresholds.json` is set from that one run (`:5845`).

## The first increment — checked

**Do Tasks 0–4 close the judgment gap on their own?** Structurally yes; as written, no.

The dependency chain is clean: Task 0 (the SDK behind `JudgeModel`) → Task 2 (the pipeline, pure and
injected) → Task 3 (`/judge/task`, a thin handler) → Task 4 (`CloudModel`, `enrich.rs`'s cloud arm).
Task 1's schema is a real prerequisite for Tasks 2–3 and is included. Nothing in Tasks 0–4 imports
anything from Tasks 5–14: `judge_pipeline.ts` needs only `judge_anthropic`, `judge_caps`,
`judge_log`, `judge_models`, `judge_prompts`, `judge_rules`, `judge_validate` — all Task 2 — and
`cloudmodel.rs` needs only `judge.rs`, `wincred`, `ledger`, `pystr` and `ureq`, all existing. The
`ruleTable` is live from Task 2 but answers `null` until Task 12 promotes anything, which is
correct. `enrich.rs`'s events and Gmail pull phases are added in Tasks 9 and 11 as *insertions*
(`:4620`), not rewrites, so Task 4's `run_lines_with` stands alone.

Three things block it in practice:

1. **Critical 1** — `/judge/task` versus `judge-task`. The engine 404s against staging. This is the
   single defect that stops Tasks 0–4 from doing the one thing they exist to do.
2. **Important 1** — the increment needs the scheduler to run `judge` on a machine with no
   llama.cpp, and that change belongs to C1 and currently disagrees with C1's own plan. Until it is
   reconciled the increment is inert on a real machine, exactly as `:2877` warns.
3. **Critical 3** is not in Tasks 0–4 (H3/H6 arrive with Task 8), but H6 is named as a Task 4
   dependency at `:2024`, so whoever applies H6 early also applies the `|| cloud` change to
   `slot_argv` and gets the amber tray before Task 8 ever lands.

With those three fixed, Tasks 0–4 are a genuine shippable increment and I would gate them as one.

## Risks the plan under-weights

- **Cost and caps.** Important 2 and 3. The arithmetic is not in the plan at all; it should be, next
  to `DAILY_CAP`, because the caps are 4.5× the subscription price and the budget threshold is
  below plausible real use. Neither the Batch API (50% cheaper, and the twice-daily slot is entirely
  latency-tolerant for events and email) nor per-account spend enforcement is considered.
- **JWT verification in edge functions.** C2 delegates all of it to C1's `requireActiveEntitlement`
  and correctly never re-implements it — good. But C2's functions then run every statement with the
  **service role**, which bypasses RLS, and scope by an `account_id` string in application code.
  That is a defensible design (the plan argues it at `:771-775`), and the "RLS on, no policy"
  posture is right. The residual risk is that one missing `.eq("account_id", …)` is a cross-account
  read with no database-level backstop, and there is no test that scans the C2 handlers for a query
  that omits the scope. Worth one static test.
- **`charge_call` and service-role reach.** `charge_call`, `promote_rules` and
  `export_training_rows` are `EXECUTE`-granted to `PUBLIC` by default. They fail safely today
  (none is `SECURITY DEFINER`, so RLS still blocks the writes for `anon`/`authenticated`), but none
  pins `search_path` and the plan does not say the analysis was done.
- **Gmail Testing-mode realities.** The plan handles the 7-day token honestly — `prompt=consent`
  every time (`:4847-4849`), a revoked grant is `quiet: true` and never a failed slot (`:5117-5123`),
  and P3 tells Quinn the truth. What is missing: nothing tells the *user* their grant expired.
  `markRevoked` sets `status = 'revoked'` server-side and the device sees `quiet: true`, but no line
  reaches stdout, the tray or the console, so a student's mail simply stops arriving for six days
  until somebody looks. The 100-test-user ceiling is stated but has no enforcement or counter.
- **Rule-promotion thresholds.** Important 6(c) is the substantive one — the contradiction guard is
  scoped to the kind, not the feature, so promotion never fires for any user who has ever corrected
  anything. Also, "3 agreements with no disagreement in 60 days" over `count(distinct j.fields) = 1`
  requires the *whole* field map to be identical, including `effort_hours` to the digit; two 2.5-hour
  answers and one 2.0 never promote. §5.4 measure 1 probably meant agreement on the verdict, not
  byte-equality of the field map.
- **The port's fidelity to the Rust parsers.** This is the best-handled risk in the plan. The frozen
  references are the acceptance test, the structural-vs-byte argument is right, and `pyshims.ts`
  gets the hard parts right — `round2` is half-to-even, `slugify` caps at 60 *characters*,
  `pyStr(None/True/False)`, `pyInt` over Python truthiness, and `dueLocal` handles the `hour: "24"`
  quirk. Two residual risks the plan should name: `pyStr` at `:3060-3063` has an `if
  Number.isInteger(value) return String(value)` branch that is identical to its `else`, so the
  documented Python `str(2.0) === "2.0"` behaviour is *not* implemented despite the comment saying
  it is written correctly rather than left to the fixture (`:3049-3054`); and `parseDurationHours`'s
  `DURATION` regex is declared without `g` but used with `.exec` on a shared `const` — correct here,
  but `SUMMARIES` (`:3461`) is likewise shared and stateless only because it has no `g` flag. Both
  are fine; the `pyStr` one contradicts its own comment.
- **Timeouts.** `CALL_TIMEOUT_MS = 120_000` server-side and `CALL_TIMEOUT = 120s` client-side are the
  same number, so a slow model call can burn the device's timeout *and* the function's before the
  device gives up; and Supabase edge functions have their own wall-clock limit (well under 120s for
  the free/pro CPU budget) which the plan never mentions. A `gmail-read` that judges up to
  `READ_CAP = 60` messages **serially** (`:5129-5143`), each with a 120-second bound, cannot finish
  inside any edge-function limit — that loop needs a wall-clock budget of its own and a resume
  cursor, and it has neither.
- **`heuristics_seed`.** The seed is trusted by the server as the source of `known_courses`
  (`:1092`, `:1122`), which is the *only* thing standing between the model and an arbitrary
  `course:` value being written into the vault. Since the device re-applies `judge_task`'s own
  `knows_course` check (`engine/src/judge.rs:400-404`), the vault is safe — but the plan should say
  that the server-side check is a convenience and the device's is the guarantee, because for the
  Gmail path (`:5140`) there is no device-side `judge_task` and the server's `known_courses` from
  `deps.knownCourses(account_id)` is the only check. Where does that list come from? Nothing in C2
  populates it — the account has no course list server-side. Another queue-shaped gap in Task 11.

## Anthropic API — checked

Invoked the `claude-api` skill before judging any model id, parameter or price. What it confirms and
what it contradicts:

- **`claude-haiku-4-5`** is the correct, complete model id — no date suffix, which the plan gets
  right everywhere (`:791-793`, `:470`, `:500`, `:756`, `:1157`, `:1806`, `:5000`).
- **Pricing** $1.00 / $5.00 per MTok input/output for Haiku 4.5 — matches the `usd_per_m_in default
  1.00, usd_per_m_out default 5.00` seeded at `:5776`. Correct.
- **Structured outputs**: the current shape is `output_config: { format: { type: "json_schema",
  schema } }` on `messages.create()`; the old `output_format` parameter is deprecated. The plan uses
  the current shape (`:609`) — correct. The schemas use `additionalProperties: false` plus a full
  `required` list, which is what strict schema validation wants. Not verified: whether Haiku 4.5
  supports `output_config.format` at all — that is a live Models API capability lookup, not in the
  cached table. Task 0 step 6's fallback note (`:660`) is about the *SDK's types*, not model
  support; Task 3's smoke check (`:1985`) is the first thing that would catch it, three tasks late.
  Add one line to Task 0: confirm the pinned model advertises structured outputs, and name strict
  tool use as the fallback.
- **`temperature: 0`** is valid on Haiku 4.5 and **returns a 400 on Sonnet 5, Opus 5, Opus 4.8/4.7
  and Fable 5/5.1** — Important 4.
- **Omitting `thinking`** is correct for Haiku 4.5 (it takes `{type: "enabled", budget_tokens: N}`
  and does not think otherwise; `output_config.effort` errors on Haiku 4.5). The comment at `:507`
  and the assertion `assertEquals(body.thinking, undefined)` are accurate.
- **`timeout` and `maxRetries`** on the SDK client constructor are real options; `timeout` is
  milliseconds in the TypeScript SDK, which `CALL_TIMEOUT_MS = 120_000` gets right.
- **`Anthropic.TextBlock`** and `response.usage.input_tokens` / `output_tokens` are the right
  shapes. The skill recommends `client.messages.parse()` with `response.parsed_output` for
  structured outputs rather than hand-parsing `content` — the manual path at `:611-625` works, but
  the helper is the documented one and would remove the "did not parse as JSON" branch.
- **`max_tokens`** — the skill warns explicitly against lowballing it; 256 for the eight-field email
  schema is the case that will bite (Minor 5), and `stop_reason` is never checked.
- **Not considered by the plan, and worth a line each:** the Batch API at 50% cost for the
  latency-tolerant kinds; and prompt caching, which is *correctly* not used here since the ~280-token
  system prompt is below the minimum cacheable prefix (512–4096 tokens, model-dependent) — say so,
  so the next reader does not "fix" it.

## What I did not verify

- I ran nothing. No `cargo`, no `deno`, no `supabase`, no git command, and no network call. Every
  claim about existing behaviour is from reading the source, not from executing it.
- I did not open `C:\Users\danie\GitHub\quinn-ops` or anything under `.claude/worktrees/`, so the
  eval seed's three sources (seven course notes, ~420 verdict lines, the journal's amend records) and
  their real volume are unverified — Task 13's drop counts and Task 14's step-5 spend estimate rest
  on numbers I could not check.
- I did not verify Supabase-specific runtime behaviour: edge-function wall-clock and CPU limits,
  whether `supabase functions deploy` bundles a cross-function import (`../judge-task/handler.ts`)
  the way I assume, whether `supabase db push` ignores a `.ts` file sitting in `migrations/`, and
  whether `vault.create_secret` is reachable through PostgREST RPC as `:4917` assumes.
- I did not verify that Haiku 4.5 supports `output_config.format` (a live Models API lookup) or
  that the `enum` with a `null` member in `EMAIL_SCHEMA` (`:1345`) is inside the provider's
  structured-output JSON Schema subset.
- I checked C1's plan only where C2 names a contract with it. I did not review C1 as a whole, so
  divergences outside those five contracts may exist.
- I did not re-derive the frozen references or confirm the fixture files the tests read
  (`zybooks-assignments.json`, `zybooks-parsed-reference.json`, `vhl-dashboard.html`,
  `vhl-parsed-reference.json`, `blackboard.ics`) contain what the tests assert about them —
  24 assignments, 13 buckets, 490 `&quot;` entities, the em dash. I confirmed only that the paths
  resolve.
- Test counts quoted in the plan's "Expected:" lines (`ok | 3 passed`, `| 20 passed`, `| 16 passed`)
  I checked by counting `Deno.test` blocks where the code is present; where the tests are prose
  (Tasks 9–14) I could not.

## Controller rulings on this review (2026-09-09)

- **R-C2-1** (writer's gap 1): the LMS `.ics` fetch moves to `/ingest-ics`; its parse stays on the device. Accepted before the review; the plan carries it.
- **R-C2-2** (writer's gap 2): event feeds — fetch and verdict move; the roster stays until C3. Accepted.
- **R-C2-3** (I1, C3): the scheduler's `judge` gate is **C1's** (its plan carries "run `judge` when `config/cloud.yaml` exists"); the scheduler's `ingest` half (`has_ics_url || cloud`) lands with C2 Task 8 as a controller hand-off applied at that merge, after C1 — by then `scheduler.rs` is a shared file again. H3 (`engine/src/ingest.rs`) must carry the exact code that moves the empty-`ics_url` early return below the cloud attempt, so a cloud vault with no local URL never exits 1.
- **R-C2-4** (C4): §6 (b) carries no note body and §5.2 discards bodies after the call — both signed — so a correction cannot become a replayable eval case unless the account has opted into (c). Ruling: corrections feed rule promotion and the correction-rate metrics for every account (features only); they become eval cases **only under the (c) opt-in**, where the device sends the re-derived judge request alongside the correction (never for `origin = gmail_api` rows). The (c) toggle's UI is C4, so the suite is seed-only until then. §5.4 measure 2 is narrowed accordingly in the fidelity ledger and the exit gate; the C1 telemetry contract gains an optional `request` field on (b) rows, present only under (c). Cost if wrong: the eval suite learns from real use later than the spec hoped; the alternative would store raw content without consent. Quinn may reverse this by amending D5.
- **R-C2-5** (C2): C1's real `sources` schema (`url_ciphertext`, `url_iv`, AES-GCM through C1's `_shared/crypto.ts`) is the contract; C2 imports C1's `decryptString`/`importAesKey`; the AES key's secret name is C1's precondition and is referenced, not re-asked.
- **R-C2-6** (M8): the commit trailers name the model and session that execute the plan; the plan's literal is the writing session's and the executing session substitutes its own. Not a defect.
- Everything else in Critical/Important is the writer's to fix as written; Tasks 9–14 are brought to the house standard (real code in every code step).

---

## Re-review of fix round 1 (2026-09-09)

Scope as set by the controller: verdict every Critical and Important finding and every R-C2 / R-X
ruling; review the calendar addition as a new unit; run the **final** cross-check against the C1 plan
as it now stands; judge the three deferrals and the two unresolved items; and flag only breakage the
fixes themselves introduced. Plan re-read at 8,805 lines in two passes — the changed regions by
section map, then the six *Interfaces with C1* statements and the ten hand-offs against the C1 plan's
own *Interfaces with C2*, its `sources` and `corrections` DDL, its command lists and its wizard
markup. Where a fix names a symbol in the existing codebase I checked it again (`cli::run`'s only
production caller at `engine/src/main.rs:293`, `calfeed::load_calendar_events`'s fetcher seam,
`eventroster::read_roster`, `write::create`'s id minting, `judgelog::entry_for`'s catch-all arm).

**Verdict: NEEDS FIXES — all four Critical and all fifteen Important findings are addressed and every
R-C2 ruling is carried, but the calendar addition contracts against a C1 that does not define
`google_calendar`, does not write `cloud:google`, does not expose the two commands H9 invokes, and
does not populate `corrections.judgment_kind`. Nine cross-check mismatches remain, four of them
blocking.** Nothing that was right in round 0 was broken by the fixes.

### Critical — 4 of 4 ADDRESSED

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| C1 | `/judge/task` versus the deployed `judge-task` | **ADDRESSED** | The contract test asserts `POST /functions/v1/judge-task` (`plan.md:2861`); every path in the plan is now the hyphenated function name; exit-gate item 1 states the rule and the reason (`plan.md:8751`). One stale slash-form survives in prose — see *New breakage*. |
| C2 | `/ingest-ics` read a `sources.url` column C1 does not create | **ADDRESSED** | Contract 3 quotes C1's real columns and says so in bold (`plan.md:188`); `import { decryptString, importAesKey } from "../_shared/crypto.ts"` (`plan.md:4991`); Task 8's *Consumes* names `url_ciphertext` / `url_iv` (`plan.md:4886`); the AES key is a referenced precondition P0a with `SOURCES_ENC_KEY` named and C1 credited for it (`plan.md:49`). R-X-1 carried exactly. |
| C3 | `ingest` exited 1 before the service was ever asked | **ADDRESSED** | H3 is now two changes, and the plan says the placement of the second *is* the point (`plan.md:280`): the early return is deleted at `ingest.rs:741-742` and re-sited inside each arm of the new match, so exit 1 means "there was no way at all to get a feed" (`plan.md:282-322`). The existing `ingest_main_returns_1_when_no_ics_url_is_configured` is shown to still take the `(None, None)` arm and get the identical line and code (`plan.md:324`). H5 carries the `cloud` condition separately, applied at Task 8's merge after C1 (`plan.md:375-386`) — exactly as R-C2-3 directs. |
| C4 | the eval suite's correction half had no data source | **ADDRESSED, as ruled** | R-C2-4 is carried in four places that agree: the ledger row (`plan.md:65`), contract 4's `request` bullet with the Gmail exclusion (`plan.md:229`), Task 14's "what the suite is, as ruling R-C2-4 narrows it" (`plan.md:8354`), and exit-gate item 11, which now says *frozen seed* and "none until C4" (`plan.md:8759`). *What is NOT in this plan* names the (c) toggle as C4's (`plan.md:8771`) and the narrowing is argued as narrowing three of four (`plan.md:8792`). |

### Important — 15 of 15 ADDRESSED

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| I1 | H6 misfiled, contradicting C1 | **ADDRESSED** | H6 is gone; the hand-off preamble says so explicitly (`plan.md:261`); the dependency is *Interfaces with C1* contract 6 (`plan.md:239-256`); the only scheduler change C2 still needs is the one-line `ingest` condition, renumbered H5 and applied after C1. **But the argv and the enum in contract 6 contradict R-X-4 and C1's merged shape** — mismatch 9 below. |
| I2 | the budget guard was inert and its view double-counted | **ADDRESSED** | `record_tokens` as its own SQL function (`plan.md:1196`) called from the pipeline (`plan.md:2052`); `enforce_budget` (`plan.md:1234`) refuses the call rather than reporting after the fact; `monthly_spend` reads `usage_daily` and `models` alone, with the old `judgments` join named as the defect it was (`plan.md:96`). |
| I3 | caps and price chosen independently, neither enforced | **ADDRESSED** | `DAILY_CAP = { task: 60, event: 80, email: 120 }` (`plan.md:2011`) with `MONTHLY_CEILING_USD = 7.5` (`plan.md:2019`); the arithmetic is written out beside them and in the ledger (`plan.md:96`, `:1162`); exit-gate item 12 requires the three numbers to have been chosen together (`plan.md:8760`). |
| I4 | `temperature: 0` hard-coded | **ADDRESSED** | `ModelRequest` gains `sampling: Sampling` where `Sampling` is `{ temperature: number }` or the empty record (`plan.md:555`), carried per pinned row (`plan.md:613`), with `temperature is sent only when the pinned row asks for it` and the 400-on-Sonnet-5/Opus reason in the test's own comment (`plan.md:649-667`). |
| I5 | `judge_roster` judged events from a roster that carries none of the fields | **ADDRESSED, and better than the fix I proposed** | The pass is rewritten to judge **from the feeds, not from `state/events.md`** (`plan.md:5330`, `:5493`), and the defect is named in the code's own comment (`plan.md:5403`). That restores `description`, `categories`, `audiences`, `series_uid` and the real `source`, which is what the prompt and the promotion features both needed. |
| I6 | `promote_rules()` would not run and would not fire | **ADDRESSED, all five parts** | `create extension if not exists pg_cron with schema extensions` (`plan.md:1039`); `min(jsonb)` replaced by an ordered `array_agg` pick; agreement tested over the verdict fields only (`fields` minus the five feature keys); the contradiction guard joins on **feature and value** through the same `judgment_features`; `rule_evidence` populated in the same statement. Each of the five is named as a defect in the function's own header comment. |
| I7 | Task 12 changed `fieldsOf` in prose | **ADDRESSED** | `fieldsOf` moved into Task 2 with its real three-argument signature (`plan.md:2295`, called at `:2328`) and is in the module's *Produces* list (`plan.md:1304`). |
| I8 | `fetch_payloads` declared twice with different signatures | **ADDRESSED** | One signature, `(email: &str, password: &str)`, in the Interfaces line (`plan.md:4393`) and the code (`plan.md:4842`), with the reason it takes two strings rather than the config mapping written out. |
| I9 | Tasks 9–14 were prose | **ADDRESSED** | +2,897 lines, almost entirely in the back half. Spot-checked: `/ingest-calendar`'s full handler and ICS writer (`plan.md:5127-5222`), `events/handler.ts`, the Google connect/callback handlers and their Vault helpers (`plan.md:6449`), `gmail-read`'s wiring, Task 12's `judge-rules` function and its device half, Task 13's `deidentify.ts`, Task 14's `eval_cases` / `eval_runs` DDL and `run_eval.ts`. Every step I opened has code, an expected result, or both. |
| I10 | the Supabase SDK imported unpinned | **ADDRESSED, and better** | `supabase-js` is gone entirely: `_shared/judge_db.ts` is a forty-line PostgREST client over `fetch` (`plan.md:1710`), adopting C1's stated posture verbatim, so `@anthropic-ai/sdk` is the only third-party import in `cloud/` and the only `deno.json` line C2 asks for (`plan.md:437-443`). |
| I11 | the byte-for-byte Gmail-card test could not pass | **ADDRESSED** | Compared without `id:`, with the reason in the doc comment — "the id is the one line that is *supposed* to differ" (`plan.md:7410-7440`). |
| I12 | two §5.3 requirements dropped without a ledger row | **ADDRESSED** | Both are now ledger rows: the 15-a-day cap stays the engine's, argued (`plan.md:66`); user-excludable labels ship as a schema seam, `google_accounts.excluded_labels text[]` (`plan.md:5946`) applied as negative label terms and pinned by `excluded_labels_become_negative_label_terms` (`plan.md:6779`), with the UI named as C1's or C4's. Both also appear in *Where this plan narrows the spec* (`plan.md:8796`). |
| I13 | `collect_cloud` was Windows-only, its caller was not | **ADDRESSED** | The non-Windows stub is written out with the reason (`plan.md:4806-4820`), matching the shape `fetch_zybooks` / `fetch_vhl` already have. |
| I14 | the eval gate's trigger missed a pin change | **ADDRESSED** | The filename pattern is replaced by a content test — any migration in the diff that mentions the `models` table — and the comment says a filename pattern "is exactly the wrong test" (`plan.md:483-489`). |
| I15 | `rank` gained reach into the judgment module with no guard | **ADDRESSED** | `rank_cannot_reach_a_judgment_endpoint` is a real static test over `cli.rs`'s source: seven forbidden tokens, **plus** a positive assertion that `cloudmodel::fetch_event_source` is present so the test cannot pass vacuously when H4 was never applied (`plan.md:5652-5669`). |

### Rulings — R-C2-1…6 all carried; R-X-1…7, five carried outright and two partly

| Ruling | Verdict | Evidence |
|---|---|---|
| R-C2-1 ICS fetch moves, parse stays | Carried | Ledger `plan.md:94`; narrowing two, argued at `plan.md:8783-8790` with the `blackboard-parsed-reference.json` prerequisite intact. |
| R-C2-2 event fetch and verdict move, roster stays | Carried | Ledger `plan.md:97`; narrowing one at `plan.md:8781`. |
| R-C2-3 scheduler judge gate is C1's; H3 moves the early return | Carried | `plan.md:239-256` and `plan.md:261`; H3 at `plan.md:278-324`; the `ingest` half is H5, applied after C1. |
| R-C2-4 corrections become eval cases only under (c) | Carried | Four agreeing places, listed under C4 above. |
| R-C2-5 C1's real `sources` schema; import C1's crypto | Carried | `plan.md:188-195`, P0a at `plan.md:49`. |
| R-C2-6 commit trailers are a template | Carried | One clause in Global Constraints naming the ruling (`plan.md:31`) and the deferral row (`plan.md:8802`). |
| R-X-1 `sources` | Carried | As C2 above. |
| R-X-2 `corrections` created by C1, altered by C2 | **Partly** | The migration is right — the two `add column if not exists` lines and the index on `(judgment_kind, ts desc)` (`plan.md:1113-1116`), with a guard test listing `corrections` among the tables C2 may not create (`plan.md:993`, `:1001-1002`). **But contract 4's SQL block (`plan.md:212-224`) still shows C2's pre-ruling shape** — mismatch 7. |
| R-X-3 optional `request jsonb` | Carried | `plan.md:221`, `:229`; Task 14 reads it and the exit gate says "none until C4". |
| R-X-4 the scheduler is C1's | **Partly** | H6 deleted, dependency stated, `ingest` gate as a separate hand-off against C1's merged shape — all three correct. **The argv and the enum contradict C1** — mismatch 9. |
| R-X-5 `_shared/` ownership | Carried | `plan.md:142`; every new shared file is `judge_`-prefixed. |
| R-X-6 `config.toml` / `deno.json` | Carried | H7 at `plan.md:404-443`: eleven `verify_jwt = false` entries, the `google-callback` exception argued, one `deno.json` import line. |
| R-X-7 one `cloud` job, amended | Carried | H8 amends C1's job rather than adding a second, and says so in bold (`plan.md:447`); `eval-gate` is the second job. |

### The calendar addition, as a new unit

**Against §11a, D4 and the ownership list.** The shape is right, and the argument for it is the best
thing in this round. Serving both calendar sources as **ICS** (`plan.md:5127-5222`) means
`calfeed::parse_calendar_ics`, its 28-day horizon, its recurrence handling, its dedup and its
snapshot fallback are all unchanged — no second parser, no new vault file, nothing for the golden
`today.md` oracle to notice, and `calfeed.rs` is not touched at all. `singleEvents=true` on the
Google query is the right detail: it makes Google expand recurrences so `calfeed`'s own RRULE path
never sees one. D12 is honoured and improved: `calendar.readonly` is asked **first and alone**
(sensitive, no CASA), `gmail.readonly` is incremental, `google_accounts.scopes text[]` records what
was actually granted, `read_google_grant` refuses a reader whose scope is missing, and
`calendar_is_asked_for_before_gmail_and_never_together` pins it (`plan.md:8757`). P2 is restated for
two scopes and tells Quinn the calendar one can clear on its own, first (`plan.md:51`). The renaming
argument for `google-connect` / `google-callback` is sound (`plan.md:435`). Ownership is clean: the
new files are all under `cloud/supabase/functions/`, and Task 8 stages nothing under `ingest.rs`,
`cli.rs` or `calfeed.rs` and says so (`plan.md:5306`).

**The recorded deviation — pulling in `rank` rather than in `ingest` — is correct, and I would have
made the same call.** The ruling's "beside `/ingest-ics`" is about where the *fetch* lives, and it
does. Pulling into `ingest` would mean a second calendar parser and a `state/calendar.md` that `rank`
overwrites seconds later. It is still a device pull inside a slot, still never a push. The deviation
is recorded where a reader will find it (`plan.md:207`).

**Does it keep `rank` free of a judgment endpoint? Yes, and structurally.**
`rank_cannot_reach_a_judgment_endpoint` (`plan.md:5652-5669`) greps `cli.rs`'s own source for
`/judge-task`, `/judge-event`, `/judge-email`, `judge_task`, `CloudModel`, `EventModel` and
`EmailModel`. The calendar proxy calls `cloudmodel::fetch_calendar`, which is none of those, and
`/ingest-calendar` contains no model. The positive half of the test also means the guard fails loudly
if H4 was never applied rather than passing on an unmodified file.

**Can `cloud:<name>` reach `calfeed` unproxied on a vault with no `config/cloud.yaml`? Not through
production.** I traced it: `engine/src/main.rs:293` is the only production caller and it calls
`cli::run`, which H4 replaces; H4's closure routes every `cloud:`-prefixed url to the
`(Some(_), None)` arm and returns `Err("no account on this vault")` (`plan.md:361`), and
`calfeed::load_calendar_events` turns a per-feed `Err` into a warning plus the snapshot. The
unproxied path exists only through `cli::run_with(…, Fetchers::default())`, which today is called
solely by `cli.rs`'s own tests. Even there the blast radius is bounded: `calfeed::fetch_ics` would
hand `"cloud:google"` to `ureq`, get a malformed-URL error, and produce one warning and a snapshot —
no panic and no egress. Worth one sentence in H4 saying `run_with` with default fetchers is not a
production path, so the next person to add a caller knows what the seam is load-bearing for.

**Against the testing rules:** the calendar work adds no network test. `handler_test.ts` is fakes
throughout (`plan.md:5230`); the device side is the loopback contract test
`the_calendar_fetch_is_a_get_that_names_the_feed_and_no_address` (`plan.md:5273-5288`), which also
asserts the secret address is not in the request.

**Where it breaks: the whole `google_calendar` half is contracted against a C1 that does not provide
it.** Five links in the chain, four of them missing — mismatches 1 to 5 below.

### FINAL cross-check against the C1 plan — 9 mismatches

Checked name by name against `docs/plans/2026-09-09-c1-accounts-plan.md` (8,183 lines): its
*Interfaces with C2* (items 1–10), its `sources` and `corrections` DDL, its two `generate_handler!`
lists, its wizard markup and its static assertions.

| # | Mismatch | Evidence | Severity |
|---|---|---|---|
| **1** | **`sources.kind` has no `google_calendar`, and the row shape cannot hold one.** C1's DDL is `check (kind in ('lms_ics', 'calendar_ics'))` with `url_ciphertext text not null` and `url_iv text not null` (`c1:1123-1133`); `SOURCE_KINDS` is the same two values in the TypeScript validator (`c1:3597`) and the Rust (`c1:6168`); C1's own comment says "A third kind is a migration and one line in `SOURCE_KINDS`" — and neither plan does it. C2's contract 3 table says `google_calendar` is "Written by C1's wizard" (`plan.md:203`) and `/ingest-calendar` reads it (`plan.md:5140`). A grant row also has no URL to encrypt, so both `not null` columns are unfillable. | `plan.md:203`, `:5140` vs `c1:1128`, `:3597`, `:6168` | **Blocking** |
| **2** | **`sources` has no `name` column, so `/ingest-calendar?name=` cannot resolve.** The primary key is `(account_id, kind)` (`c1:1132`). C2's `sourceFor(accountId, name)` (`plan.md:5139`, called at `:5188`) and its smoke steps `?name=personal` and `?name=google` (`plan.md:5297`) assume a per-name lookup, and the plan never states the name-to-kind mapping the handler would need. | `plan.md:5139`, `:5188`, `:5297` vs `c1:1123-1133` | **Blocking** |
| **3** | **Nobody writes the `calendars:` entry `{name: google, ics_url: 'cloud:google'}`.** C2 says "C1 writes that one entry when the grant is made and removes it when it is revoked" (`plan.md:205`). C1's `scaffold::ingest_yaml` writes exactly one entry, `name: personal` with the secret address (`c1:5382`, `:5528-5538`), and the string `cloud:` appears nowhere in C1's plan. C2 never edits `scaffold.rs` or any vault config. Without the entry, `calfeed` never walks a `cloud:` url and the entire `/ingest-calendar` Google path is unreachable from the device. | `plan.md:205`, `:350-364` vs `c1:5382`, `:5538` | **Blocking** |
| **4** | **H9 invokes two commands that do not exist.** `google_connect_url` and `open_external` appear nowhere in C1 — zero hits in 8,183 lines — and neither is in either `generate_handler!` list (`c1:301`, `:307`). C1's system-browser mechanism is the private helper `account::open_in_browser(url)` (`c1:4797-4805`), surfaced only as `open_policy` and `open_checkout`. H9 asserts both are "**C1's** commands" (`plan.md:532`). | `plan.md:527-532` vs `c1:301`, `:307`, `:4339`, `:4797` | **Blocking** |
| **5** | **H9's markup does not fill C1's labelled place, and would break two C1 tests if it replaced it.** C1 leaves `<div class="wiz-row" id="wiz-google-row">` containing `<button class="b" id="wiz-google" disabled>Sign in with Google instead</button>` (`c1:7127`), asserts `id="wiz-google"` **and** `disabled` on that panel (`c1:7011`), and the headless walk requires `#wiz-google[disabled]` (`c1:493`). H9 writes a different element — `<div class="panel-row" id="google-calendar">` with `<button id="connect-google" class="secondary">` (`plan.md:511-518`). Added beside the placeholder it leaves two Google buttons, one of them lying; used to replace it, both C1 assertions fail. (H9 does *not* trip C1's "no Google connect in C1" assertion, which greps `console.js` for the literal `"connect_google"` and for `gmail.readonly` — `c1:7013` — so that one is safe either way.) | `plan.md:511-529` vs `c1:493`, `:7011`, `:7127`, `:7517` | Important |
| **6** | **`corrections.judgment_kind` has no producer.** C2's contract 4 says "C1's `telemetry.rs` derives it from the agent actor" (`plan.md:226`), and C2's `promote_rules` contradiction CTE joins `j.kind = c.judgment_kind` (`plan.md:7663`) while `backfill_correction_judgments` joins on it too (`plan.md:8397`). C1's `CorrectionIn` has no such field (`c1:3956-3967`) and `correctionRows` never writes one (`c1:4006-4022`); C1's *Interfaces with C2* mentions `judgment_kind` only as a column **C2 adds** (`c1:232`). Left as is the column is always null: the contradiction guard never matches — so I6(c) is only half-fixed in effect — and the eval loader's back-fill never matches either. | `plan.md:226`, `:7663`, `:8397` vs `c1:3956`, `:4006`, `:232` | Important |
| **7** | **Contract 4's SQL block still shows C2's pre-R-X-2 `corrections`.** `kind text not null check (kind in ('task','event','email'))`, `ours text not null`, `theirs text not null`, `corrected_at` (`plan.md:212-224`) — against C1's real `ts`, nullable `ours` and `theirs`, note-kind `kind` with no check, and `received_at` (`c1:3697-3717`). C2's own migration is right (`plan.md:1113-1116`) and its prose bullets already talk about `corrections.ts`, so the block contradicts the two paragraphs beneath it. | `plan.md:212-224` vs `c1:3697-3717`, `c1:224-233` | Minor (but it is the contract text) |
| **8** | **`corrections.features` has no provider and no reader.** C2's contract asks for `features jsonb not null default '{}'` and says it "is always sent and is what rule promotion needs" (`plan.md:220`, `:228`). C1 neither defines nor sends it; C2's migration does not add it; and `promote_rules` now takes its features from `judgment_features` over the `judgments` table, so nothing reads it. | `plan.md:220`, `:228` vs `c1:3697-3717`, `plan.md:7645-7660` | Minor |
| **9** | **The `judge` argv and `JudgePlan`'s shape contradict R-X-4 and C1.** R-X-4 fixed the Cloud arm as `judge --vault <v> --via local-runner --log-dir <profile judgments dir>`, "`--log-dir` stays in both arms because judgment logs never enter the vault"; C1 carries it — `enum JudgePlan { Cloud { log_dir }, … }` and the argv vector with `--log-dir` (`c1:4892`, `:5031`, `:5208-5210`, `:5249`), stated in its Interfaces (`c1:186-191`). C2's contract 6 states `JudgePlan { Cloud, Local(JudgeArgs), Skip }` and the argv "**and nothing else — no `--runtime`, no `--model`, and no `--log-dir`**" (`plan.md:242-253`), then builds two consequences on it: "There is no device judgment log on the cloud path" and "`enrich::Options.log_dir` is `None`, so `judgelog::record` is never called" (`plan.md:255`). Both are false against C1's merged shape. `judgelog.rs` still needs no change either way — but the *reason* given for leaving it unchanged (`plan.md:161`, `:255`) is now wrong, and any C2 test that runs a slot against a cloud vault will see a log written. | `plan.md:242-256`, `:161` vs `c1:186-191`, `:4892`, `:5031`, `:5249` | Important |

**What C1 does provide, verified and agreeing:** `config/cloud.yaml`'s four keys and their order
(`plan.md:169-176` vs `c1:150-156`); the session credential as a JSON blob with `access_token`, with
C1 owning refresh (`plan.md:180-186` vs `c1:4339`); `requireActiveEntitlement`'s signature and its
throw-a-`Response` contract (`plan.md:231-237` vs `c1:189`, `:218`); `_shared/crypto.ts`'s two
functions and `SOURCES_ENC_KEY` (`plan.md:191-195` vs `c1:3080-3090`); migration numbering and C1's
scoped numbering test (`c1:250-252`); `_shared/` ownership with the `judge_` exception (`c1:40`);
`config.toml` and `deno.json` as C1's with C2's hand-off (`c1:239-244` vs `plan.md:404`); one `cloud`
CI job amended, not duplicated (`c1:246-249` vs `plan.md:447`); and the `ingest` gate staying
`has_ics_url(vault)` in C1 with C2's condition applied against it (`c1:201` vs `plan.md:375-386`).
The `ics_url` removal, unresolved in round 0, is now settled: C2 keeps it as the offline fallback and
names C3 or C4 as the remover, with the reason (`plan.md:95`).

### The three deferred minors — accept 3, object 0

- **M8, the commit trailer** — **accept.** R-C2-6 ruled it, and the plan adds the one clause the
  ruling asks for (`plan.md:31`). A template that says it is a template is not a defect.
- **M12, `export_training_rows` filters a table that carries nothing trainable** — **accept.** The
  filter costs nothing and is already there for the day `judgments` grows a column; the real class-(c)
  export is on the device and out of C2's scope, and it is recorded for whoever builds it
  (`plan.md:8803`). My original finding said the same.
- **M13, `fetch_payloads` fetches ignored zybooks** — **accept**, and the reasoning is better than my
  finding. Sending the `ignore` list to the device's fetch half would put routing back on the device,
  which is precisely what §4.3 moved; one extra round trip per slot against one such book is the
  cheaper side of that trade, and it is recorded with its cost (`plan.md:8804`).

### The two unresolved items

- **§5.3's *source went quiet* deferred to C4 / C1's Issues panel — accept.** The server side is
  modelled (`google_accounts.status = 'quiet'` after 14 days, `plan.md:5949`) and the *actionable*
  case does reach the student today: `quiet: true` becomes `gmail: skipped (gmail is not connected;
  re-connect from settings)` in the slot log and `state/runner-log.md` (`plan.md:7483`). That is the
  line a student whose 7-day testing token expired actually needs. The 14-day signal belongs on a
  panel C2 does not own, and it is recorded in Task 15 (`plan.md:8733`).
- **H9's dependency on C1's hook — do not accept as written.** Mismatches 4 and 5 are the substance:
  the two commands do not exist and the markup does not fit the labelled place. H9's own escape hatch
  ("if C1's labelled place does not exist at merge time, this hand-off is deferred to C4",
  `plan.md:532`) does not cover this case — the place *does* exist; it is the commands that do not.
  **The fix:** H9 must carry, as exact hand-off code, (a) `app/src/account.rs`'s `google_connect_url`
  command — `GET {api_base}/google-connect` with the session bearer, returning `{url}` — built on
  C1's existing `open_in_browser`; and (b) either an `open_external` command or, far cheaper, no
  second command at all: have `google_connect_url` open the URL itself, exactly as `open_checkout`
  and `open_policy` already do, so the listener is one `invoke`. (c) The markup must reuse C1's ids —
  `#wiz-google` inside `#wiz-google-row`, class `b` — and *remove* the `disabled` attribute, which
  then requires H9 to carry the correction to C1's two assertions (`c1:7011`, `c1:493`) in the same
  hand-off. And (d) somebody must write the `cloud:google` `calendars:` entry and the
  `google_calendar` `sources` row; on the evidence that has to be a C2 hand-off into
  `app/src/account.rs` too, since C1 defines neither kind. Note that H9 is currently scoped to
  `app/static/**` only; with (a) to (d) it becomes a hand-off into `app/src/**` and into
  `app/src/main.rs`'s handler lists, which is a larger thing and should be named as such — and it
  changes C1's "23 / 43 / 55" command counts, which C1's Task 21 verifies.

### New breakage introduced by the fixes

- **Minor — `plan.md:7` still says `POST /judge/{task,event,email}`.** The Goal paragraph kept the
  slash form that Critical 1 removed from every other line, and exit-gate item 1 now states the rule
  it violates (`plan.md:8751`). A one-word fix; it matters only because the Goal is the first thing
  an implementer reads.
- **Minor — `ingest-calendar` is listed twice in *File structure*** (`plan.md:134` and `plan.md:138`),
  once as "the account's calendars as ICS" and once as "both calendar sources as ICS".
- **Minor — *File structure* omits two files the fixes created and keeps one nothing creates.**
  `_shared/judge_handler.ts` (Task 3, `plan.md:2461`) and `_shared/judge_db.ts` (Task 2,
  `plan.md:1291`) are absent from the list at `plan.md:123-131`, and
  `ingest-coursework/reconcile.ts` (`plan.md:132`) is still listed although Task 7 puts the
  reconciliation in `handler.ts` and creates no such file.
- **Minor — the migration is still `20260911000200_gmail.sql`** although it now creates
  `google_accounts`, `google_state`, `gmail_seen` and `gmail_queue` and holds the three Vault helpers
  (`plan.md:120`, `:5923`, `:6449`). The plan's own argument for renaming the functions — "a function
  called `gmail-connect` that asks for a calendar scope would be the first thing to mislead the next
  reader" (`plan.md:435`) — applies here verbatim. Renaming costs three references
  (`plan.md:120`, `:5913`, and Task 11's static test that reads the file by name).
- **Minor — mismatch 7 is itself new breakage:** the R-X-2 fix landed in the migration and in the
  guard test but not in contract 4's SQL block, so the interface text now contradicts the code
  beneath it.

### Not re-examined

The regions the fixes did not touch (Tasks 0–7's already-verified code, the two parser ports, the
eval seed's de-identification rules); C1's plan outside the ten interface statements and the four
regions the controller named. Still nothing executed — no `cargo`, `deno`, `supabase` or `git`
command — so the Deno, Supabase, Google Calendar and Vault surfaces are again assessed from reading
rather than from running. In particular I did not verify that `vault.create_secret` is reachable
through the PostgREST RPC the plan uses, that Supabase's edge runtime tolerates `_shared/judge_db.ts`
under the pinned `deno.json`, or that Google Calendar's `singleEvents=true` expansion returns the
`start.date` / `start.dateTime` shapes `toIcs` assumes for an all-day event. The Anthropic surface
was re-checked against the `claude-api` skill only where the fixes changed it — per-row `sampling`,
`stop_reason` including `refusal`, and per-kind `max_tokens` (email raised to 640) — and all three
are correct as written.

## Controller rulings on the re-review's cross-check (2026-09-09)

- **R-X-9 the Google calendar is not a `sources` row.** The grant lives in C2's `google_accounts` (scopes recorded); `/ingest-calendar` resolves `name=personal` → `sources.kind = 'calendar_ics'` and `name=google` → the account's `google_accounts` row whose scopes include `calendar.readonly`. C1's constraint may keep `google_calendar` as a reserved value; C2's contract 3 says no row of that kind is ever written by C2. The name→kind mapping is stated in the handler.
- **R-X-10 H9 carries everything the Google button needs, as exact hand-off code applied at C2's merge:** the `google_connect_url` command (`GET {api_base}/google-connect` with the session bearer → `{url}`) and an `open_external` command built on C1's `account::open_in_browser` helper, both in `app/src/account.rs`; the listener that enables and drives C1's existing `#wiz-google` in `#wiz-google-row` (no new elements); the `calendars:` entry `- name: google / ics_url: 'cloud:google'` written through C1's `scaffold::ingest_yaml` path when the grant lands and removed on disconnect; and the amendments to C1's static assertion and headless walk that today require `#wiz-google[disabled]`. Registration in `generate_handler!` is the controller's line at merge.
- **R-X-11 `corrections.judgment_id` / `judgment_kind` are populated by C2's own backfill** (`backfill_correction_judgments` joins `judgments` on `(account_id, item_id)`), never by C1. `corrections.features` is dropped from the contract: rule promotion reads `judgment_features` over `judgments`.
- **R-X-12 the scheduler contract is C1's, verbatim:** `enum JudgePlan { Cloud { log_dir }, Local(JudgeArgs), Skip }` and the Cloud argv `judge --vault <v> --via local-runner --log-dir <profile judgments dir>` (R-X-4). C2 restates contract 6 to match and touches no scheduler code.
- Mismatches 2, 3, 7 and the five new minors are the writer's to fix as the report says; the C1 plan is now at 8,339 lines (its round 3 added `google_calendar` to the constraint and `SOURCE_KINDS`, `lms_link::DEVICE_KINDS`, `validate_for(kind, url, fetch)`, and named the `#wiz-google` hook) — C2 aligns to that text.

---

## Re-review of fix round 2 (2026-09-09)

Scope as set by the controller: verdict each of the nine cross-check mismatches and the eight new
items from round 1; run the **final** cross-check against C1's committed text at the eight named
points; flag breakage from this round only; park anything new and unrelated. Read R-X-9…R-X-12
first. Plan re-read at 9,014 lines — the six *Interfaces with C1* contracts, all ten hand-offs, Task
8's `/ingest-calendar`, Task 12's migration, *File structure* and the exit gate — then C1's committed
8,382 lines at its interfaces item 3, its `sources` and `corrections` DDL, its `SOURCE_KINDS` /
`DEVICE_KINDS` / `validate_for`, its scheduler contract, its two `generate_handler!` lists, its
`config.toml` and its `cloud` CI job.

**Note on the rulings I was pointed at.** R-X-9…R-X-12 are present in this report and I worked from
them. **R-X-13 and R-X-14, and the "Controller close-out" section of the C1 review, do not exist in
either report file** — I grepped both. I have therefore taken R-X-14 from the controller's message as
authoritative and verdicted against it, and flagged the missing section so it can be written down.

**Verdict: NEEDS FIXES — all nine cross-check mismatches and all eight new items are addressed as C2
edits, R-X-9, R-X-11 and R-X-12 are carried exactly, but H9 is not yet executable: it registers its
two commands on the wrong window, gives `google_connect_url` a `ConsoleState` the wizard window does
not have, and writes `config/ingest.yaml` two panels before C1 creates the vault. Seven cross-check
mismatches remain — three of them new, in H9; two are C1-side text edits.**

### The nine cross-check mismatches — 9 of 9 ADDRESSED as C2 edits

| # | Mismatch | Verdict | Evidence |
|---|---|---|---|
| 1 | `sources.kind` had no `google_calendar` and the row could not hold one | **ADDRESSED** | Contract 3 is rewritten around C1's real schema (`plan.md:190-198`); the kind table's third row reads "Written by **nobody**" (`plan.md:209`); the paragraph beneath explains that a grant has no URL to encrypt against two `not null` columns, that the value exists only so neither stream edits the other's constraint, and that the grant lives in C2's `google_accounts` (`plan.md:211`). `handler.ts` carries the same in a comment (`plan.md:5311`) and `the_google_calendar_is_never_looked_for_in_the_sources_table` pins it (`plan.md:5416-5420`). The smoke step says "**Never insert a `google_calendar` row**" (`plan.md:5490`). R-X-9 carried exactly. **A residue remains on C1's side** — mismatch A below. |
| 2 | `sources` has no `name`, so `?name=` could not resolve | **ADDRESSED** | `export const CALENDAR_NAMES = { personal: "calendar_ics", google: "google_accounts" } as const` (`plan.md:5316`), checked with `if (!(name in CALENDAR_NAMES))` before anything else (`plan.md:5367`); `sourceFor` is gone and `personalSource(accountId)` replaces it (`plan.md:5320`, called at `:5371`); the wiring note says why there is no name to match on — "the primary key is `(account_id, kind)`, so that is at most one row" (`plan.md:5410`). The mapping is also a table in contract 3 (`plan.md:215-219`), and the missing-grant case is a **409, not a 404**, distinguished in the test list (`plan.md:5412`) and in the smoke step (`plan.md:5490`). |
| 3 | nobody wrote `calendars: - name: google / ics_url: 'cloud:google'` | **ADDRESSED in intent, not yet executable** | H9(d) carries `set_google_calendar(vault, connected)` in full (`plan.md:654-669`) plus the `scaffold::rewrite_ingest_yaml` helper it needs and the note that C1 does not have one (`plan.md:672`). The design is right — one writer, through C1's own path, with `cloud:google` explained as a marker rather than a URL. **But it cannot run where the plan calls it** — mismatch E below. |
| 4 | `google_connect_url` / `open_external` did not exist | **ADDRESSED** | H9(a) writes both in full (`plan.md:541-604`), built on C1's `account::open_in_browser`, with `open_external` restricted to `https://accounts.google.com/` and the reason given (`plan.md:587-590`). C1's committed text now names the same two commands as C2's to carry (`c1:182`). |
| 5 | H9's markup did not fit C1's labelled place | **ADDRESSED, verbatim** | H9(c) quotes C1's element exactly and changes only the `disabled` attribute and the `<span class="meta">` text (`plan.md:616-626`), which is word for word what C1's interfaces item 3 says C2 will do (`c1:184-187`). H9(e) carries the two assertion amendments — `app/tests/static_assets.rs` (`c1:7155`) and `scripts/wizard-check.py` (`c1:512`) — as exact before/after pairs (`plan.md:676-691`), and notes that C1's third assertion stays true because the command is `google_connect_url`, not `connect_google` (`plan.md:693`). I checked that against C1's literal: correct. |
| 6 | `corrections.judgment_kind` had no producer | **ADDRESSED, and the reasoning is right** | `backfill_correction_judgments()` (`plan.md:7817-7842`) joins `(account_id, item_id)` with `judged_at < c.ts` and `order by judged_at desc limit 1`, taking **both** id and kind from the judgment; the header says explicitly that the join is "deliberately NOT `judgment_kind`, which is the column being filled and would make this a no-op forever" (`plan.md:7811-7813`). `promote_rules`' contradiction CTE now joins `j.id = c.judgment_id` and says why (`plan.md:7889-7896`). The nightly schedule runs `backfill_correction_judgments(), promote_rules()` in one statement, in that order, with the reason (`plan.md:7936-7941`). Both live in `20260911000300` and Task 14 says so rather than duplicating them (`plan.md:8632`). R-X-11 carried. |
| 7 | contract 4's SQL block showed C2's pre-R-X-2 shape | **ADDRESSED** | The block is now C1's real DDL — `ts`, nullable `ours`/`theirs` with the reason on each line, note-kind `kind` with no check, `request`, `received_at`, `corrections_once` (`plan.md:229-243`) — followed by exactly the two `alter table` lines and the one index (`plan.md:247-252`). Matches C1's committed DDL (`c1:3697-3717` region) line for line. |
| 8 | `corrections.features` had no provider and no reader | **ADDRESSED** | "There is no `features` column and C2 does not ask for one" (`plan.md:256`), citing R-X-11 and pointing at `judgment_features` over `judgments.fields`. The column is gone from the contract block. |
| 9 | the `judge` argv and `JudgePlan` contradicted C1 | **ADDRESSED, verbatim** | Contract 6 now quotes C1's four signatures including `Cloud { log_dir: PathBuf }` (`plan.md:270-274`) and the argv **with** `--log-dir` (`plan.md:279`), and the three consequences are restated the right way round: "**There *is* a device judgment log on the cloud path**" (`plan.md:284`), and `judgelog.rs` is still unchanged but "not for the reason an earlier draft gave" (`plan.md:285`). R-X-12 carried. |

### The eight new items from round 1 — 8 of 8 ADDRESSED

| Item | Verdict | Evidence |
|---|---|---|
| Important — the Goal's slash form | **ADDRESSED** | `plan.md:7` now reads `POST /judge-task`, `/judge-event` and `/judge-email`. |
| Important — H9 scoped to `app/static/**` only | **ADDRESSED** | The heading and the opening line say it reaches `app/src/**` and moves C1's command counts (`plan.md:535-537`), and the ledger row says the same (`plan.md:71`). **But the heading undercounts the files it touches** — see *New breakage*. |
| Important — `run_with(…, Fetchers::default())` unguarded in prose | **ADDRESSED** | H4 gains the paragraph naming `cli::run` as the only production caller, what a `cloud:` url would do through the test seam, and why the sentence is there (`plan.md:406`). |
| Minor — `ingest-calendar` listed twice | **ADDRESSED** | One entry (`plan.md:141`). |
| Minor — *File structure* omitted `judge_handler.ts` / `judge_db.ts` | **ADDRESSED** | Both present, plus `judge_deps.ts` (`plan.md:124-126`). |
| Minor — `reconcile.ts` listed although nothing creates it | **ADDRESSED** | The `ingest-coursework` line now names the real five files and says "there is no separate `reconcile.ts`" (`plan.md:135`). |
| Minor — migration still `_gmail.sql` | **ADDRESSED** | `20260911000200_google.sql` at all six references (`plan.md:120`, `:6106`, `:6116`, `:6642`, `:6766`, `:6998`). |
| Minor — contract 4 block contradicted the migration | **ADDRESSED** | Same as mismatch 7. |

### Rulings R-X-9…R-X-12, and R-X-14

| Ruling | Verdict |
|---|---|
| **R-X-9** the Google calendar is not a `sources` row | **Carried**, and better than the ruling required — the mapping is a constant plus a static test, not only prose. |
| **R-X-10** H9 carries everything, as exact hand-off code | **Carried in structure, not yet executable.** All five parts are present with real code; three of them are aimed at the wrong window, the wrong state and the wrong moment — mismatches C, D, E. |
| **R-X-11** back-fill is C2's; `features` dropped | **Carried.** |
| **R-X-12** the scheduler contract is C1's, verbatim | **Carried.** |
| **R-X-14** a `google_connected` command polled by the calendar panel invokes `set_google_calendar`, self-contained in H9 | **NOT CARRIED.** H9(d) still says the caller is "a third tiny command, `google_connected`, **or** C1's existing post-consent poll — **the controller decides which at merge**" (`plan.md:672`). No `google_connected` is defined anywhere in either plan (I grepped both), and the alternative it offers does not exist: C1 has no post-consent poll for Google and says so — "C1 defines no Google connect command, and no Google code at all" (`c1:182`). H9(b) also still says "this hand-off adds **two**", which under R-X-14 is three. |

### FINAL cross-check against C1's committed text — 7 mismatches remain

Checked at the eight points the controller named.

| # | Point | Verdict |
|---|---|---|
| interfaces item 3 table | C1 `c1:172-176` vs C2 `plan.md:205-209` | **Mismatch A** below on the third row; the `lms_ics` and `calendar_ics` rows agree exactly, including where each lives on the device. |
| the calendar panel and its `#wiz-google` hook | C1 `c1:182-187`, `:7292` vs C2 `plan.md:616-626` | **Agrees, verbatim** — same element, same parent, same panel, same two changes, same "changes no other element". |
| `validate_for` / `DEVICE_KINDS` | C1 `c1:6294`, `:177`, `:5956-5969` vs C2 `plan.md:211` | **Agrees.** C2 cites `DEVICE_KINDS = ["lms_ics", "calendar_ics"]` and `validate_for` by name as the reason no page can invent the third kind. |
| the scheduler contract | C1 `c1:203-222` vs C2 `plan.md:267-286` | **Agrees.** Enum, four signatures, argv, `--log-dir` in both arms, entitlement outranking everything, the `has_ics_url` gate staying C1's with C2's condition applied against it. Only cosmetic drift: C1's argv placeholder says "the profile's **logs** folder" and C2's says "**judgments** folder"; both prose paragraphs resolve it to `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments`, so the path agrees and the word does not. |
| the `sources` DDL | C1 `c1:1142-1156` vs C2 `plan.md:193-196` | **Agrees** on every column, the primary key, the three-value check and the absence of `url` and `name`. |
| the `corrections` DDL | C1 `c1:3697-3717` vs C2 `plan.md:229-252` | **Agrees**, including the nullability comments and the two `alter table` lines. |
| the `cloud` CI job | C1 `c1:590-604` vs C2 `plan.md:449-462` | **Agrees.** C2 replaces C1's three `run:` lines and adds `eval-gate` as a second job; neither adds a second `cloud` job. C1's `--frozen` note is a note in both. |
| `config.toml` for eleven functions | C1 `c1:728-750` (eight entries) vs C2 `plan.md:437-476` (eleven) | **Agrees** — 8 + 11 = 19, no name collides, `google-callback`'s exception argued in both. **But C2's contract 7 still quotes C1 as saying "nine"** — mismatch B. |

| # | Remaining mismatch | Evidence | Whose fix |
|---|---|---|---|
| **A** | **C1's committed text says C2 writes a `google_calendar` row; C2 says nobody does.** C1's interfaces table: "`google_calendar` \| **C2** — its `google-callback`, from a token this machine never sees" (`c1:176`), and its migration comment: "`google_calendar` is **C2's**, written by its `google-callback` when a student signs in with Google instead" (`c1:1146-1147`). C2, under R-X-9: "Written by **nobody**" and "a reserved value that C2 never writes" (`plan.md:209`, `:211`). R-X-9 makes C2 right — a grant has no URL for two `not null` columns — so C1's two sentences are the stale half. | `c1:176`, `:1146` vs `plan.md:209-211` | **C1's** (two sentences, no code) |
| **B** | **C2 quotes a "nine functions" that C1 no longer says.** Contract 7: "C1's interface text says 'its nine functions'; C2 ships **eleven**" (`plan.md:288`). The controller already edited C1 to "its **eleven** functions" (`c1:264`), so the note now corrects something that is already right. | `plan.md:288` vs `c1:264` | **C2's** (delete the note) |
| **C** | **H9 registers its commands on the wrong window.** C1 is explicit that "the two lists are **different windows**, not one" (`c1:382-395`): the console window registers 43, and **the vault-less shell (picker or wizard) registers 23** — and `#wiz-google` is in the wizard. H9(b) puts both commands on "the **console window's** `generate_handler!` list", with a reason that misreads C1 ("the wizard, which C1 moved onto the console window's handler set alongside `onboarding::*`") — only three of `onboarding.rs`'s eleven are on the console list; the other eight and all four `lms_link` commands are on the wizard's. As written the button's `invoke` finds no command. | `plan.md:614` vs `c1:320`, `:382-395` | **C2's** |
| **D** | **`google_connect_url` takes a `ConsoleState` the wizard window does not have.** Its signature is `pub fn google_connect_url(state: tauri::State<'_, ConsoleState>, scope: String)` and it reads `state.vault` and `valid_access_token(&state)` (`plan.md:549-554`). C1: "there is no `ConsoleState` yet, so **no command that needs one can be called**" on the vault-less shell (`c1:393-395`) — which is precisely why the eight `account` commands on that list are the eight that need none. The vault-less shape already exists in C1 for C2 to copy: `api_base()`, `anon_key()`, `load_session()`, `valid_access_token_at()` and the pre-vault `knowlu/pending/session` target (`c1:4339`). | `plan.md:549-554` vs `c1:393-395`, `:4339` | **C2's** |
| **E** | **H9(d) writes `config/ingest.yaml` two panels before the vault exists.** `set_google_calendar(vault, true)` "is called when `google_connect_url`'s consent completes" (`plan.md:672`) — that is panel 5 of nine. C1 creates the vault at **Finish**: the headless walk asserts `create_vault` comes after `retarget_credentials` and fails with "Finish did not invoke `create_vault`" (`c1:563`, `:568-574`). That is exactly why C1 carries the personal calendar as a `WizardPlan` field (`personal_calendar`, `c1:5623`) and lets `scaffold::ingest_yaml` write `calendars:` at creation. The Google entry has to travel the same way in the wizard case — a `WizardPlan` flag — and `set_google_calendar` is the right shape only for the settings-panel case, over a vault that already exists. | `plan.md:672` vs `c1:563`, `:5619-5632` | **C2's** (and it adds a `WizardPlan` field to C1's struct, so H9 grows a part) |
| **F** | **R-X-14 is not carried.** No `google_connected` exists in either plan; H9(d) leaves the caller open and offers a C1 alternative that does not exist (`c1:182`). | `plan.md:672` | **C2's** |
| **G** | **C1's stated obligation to remove the vault copies is still refused, and both texts stand.** C1: "When `/ingest-ics` and `/ingest-calendar` ship, **C2** removes the vault copies, not C1" (`c1:180`). C2: it does not, names C3 or C4, and says why (`plan.md:225`, ledger `:95`). This is now an *argued* divergence rather than a silent one — C2 quotes C1's sentence before declining it — and I think C2 is right (the vault copy is the offline fallback that keeps `ingest` exiting 0). It still needs one of the two texts edited so a third reader is not left choosing. | `c1:180` vs `plan.md:225` | **Controller's** (pick one) |

### New breakage introduced by this round

- **Important — H9(a) ships two mutually exclusive `open_external` bodies.** The code block uses
  `url::Url::parse` (`plan.md:593-596`); the next paragraph says "`url::Url` is not a dependency of
  `app/`. **Use `str` instead**" and gives a different check (`plan.md:607-612`). A hand-off whose
  entire contract is "exact code the controller applies" must ship one body. Keep the `str` version,
  delete the other, and fold its `\n` / `\r` / length guards into the block.
- **Minor — H9's heading names three files; the body touches seven.** The heading is
  `app/src/account.rs`, `app/src/main.rs` and `app/static/index.html` (`plan.md:535`); the parts also
  edit `app/static/console.js` (c), `app/src/scaffold.rs` (d, the new `rewrite_ingest_yaml`),
  `app/tests/static_assets.rs` and `scripts/wizard-check.py` (e). The last two are not even C1's —
  `scripts/` is the controller's, which C1 states (`c1:7692`). Name all seven, and say which owner
  each belongs to.
- **Minor — `set_google_calendar(vault, false)` has no caller.** "…is called on disconnect"
  (`plan.md:672`) names nothing that calls it; disconnect today is `DELETE /google-connect`,
  server-side, which cannot reach the vault.
- **Minor — contract 3's prose still names the function that was renamed.** "the mapping is stated in
  the handler — one function, `sourceFor`" (`plan.md:213`), where the handler now has
  `CALENDAR_NAMES` plus `personalSource` (`plan.md:5316`, `:5320`).
- **Minor — mismatch B is itself this round's breakage:** the controller's edit to C1 landed after
  C2's note was written, so the note now describes a text that no longer exists.

Nothing else regressed. I re-checked the things this round's edits could plausibly have disturbed:
the `rank_cannot_reach_a_judgment_endpoint` token list still covers the calendar proxy (it forbids
the three `/judge-*` paths and the three trait names, and `fetch_calendar` is none of them); H4's two
closures are unchanged apart from the added paragraph; the `20260911000300` migration still enables
nothing twice (`pg_cron` is enabled in `…000100` and the comment says so, `plan.md:7936`); and moving
`backfill_correction_judgments` out of Task 14's migration left Task 14 with a pointer rather than a
duplicate (`plan.md:8632`) and a caller (`plan.md:8782`).

### Out of scope — for the controller to park, not to reopen

- **The rulings section the controller cited does not exist.** Neither report contains a "Controller
  close-out", an R-X-13 or an R-X-14; I worked from the controller's message for R-X-14. Worth
  writing down, since R-X-14 is the one ruling this round did not carry.
- **C1's `comment on table public.sources` still says "Both kinds are capability URLs"** (`c1:1157`)
  although the constraint now has three; the third is not a capability URL at all, which is the whole
  of mismatch A.
- **C1's H8 CLAUDE.md sentence will be stale the moment H9 lands.** It pins 43 / 23 / 55
  (`c1:604`-region); H9 adds two commands, and C1's own Task 21 recount is what has to be re-run.
- **The C1 plan is 8,382 lines, not the 8,384 the controller quoted** — no consequence, but the two
  numbers should not both be written down.
- **C1's scheduler argv placeholder says "logs folder" where its own prose says `…\judgments`**
  (`c1:205` vs `:209-211`); C2 says "judgments folder". Cosmetic, one word.

### Not re-examined

Everything the round did not touch: Tasks 0–7's verified code, the two parser ports, the eval seed's
de-identification, the Anthropic surface (unchanged since round 1's `sampling` / `stop_reason` /
per-kind `max_tokens` fixes, all still correct). C1 was read only at the eight named points plus the
two `generate_handler!` lists and the wizard's panel order, which mismatches C, D and E required.
Still nothing executed — no `cargo`, `deno`, `supabase` or `git` command — so the PostgreSQL
`UPDATE … FROM LATERAL` in `backfill_correction_judgments` is assessed as valid from knowledge of
PG 12+ semantics rather than by running it, and Supabase's edge runtime, Google Calendar's response
shapes and Vault's PostgREST reachability remain unverified as in earlier rounds.

## Controller rulings after the second re-review (2026-09-09)

- **R-X-15 H9 is two-phase, matching C1's two windows.** (a) In the wizard (vault-less): `google_connect_url(scope)` and `google_connected()` (R-X-14) are registered on the **wizard window's** `generate_handler!` list and take no `ConsoleState` — they use C1's vault-less helpers (`api_base()`, `anon_key()`, `load_session()`, `valid_access_token_at(…, PENDING_TARGET, …)`); `google_connected` asks C2's `GET /google-connect?status=1` → `{connected, scopes}`; when it reports the calendar scope, the panel sets `WizardPlan.google_calendar = true`, and H9 amends `WizardPlan`/`VaultPlan` and `scaffold::ingest_yaml` (exact code) so the vault is born with `- name: google` / `ics_url: 'cloud:google'` beside the personal entry — never a write before Finish. (b) After onboarding, `set_google_calendar(vault, connected)` is a **console-window** command with `ConsoleState`; its UI (connect/disconnect on the settings panel) is **C4's**, alongside the (c) toggle — named under "What is NOT in this plan". Until then a revoked grant makes `/ingest-calendar` answer 404 and `calfeed` falls back to its snapshot and logs the quiet source; the `cloud:google` line is removed by C4's UI.
- **R-X-16 (mismatch G):** the vault copies of the LMS and calendar URLs stay as the offline fallback that keeps `ingest` at exit 0; **C3 or C4 removes them, not C2.** C1's sentence is edited by the controller to match.
- **Mismatch A** is C1's stale text (two sentences plus the `comment on table`): edited by the controller — `google_calendar` is reserved, written by nobody; the grant lives in C2's `google_accounts`.
- The Important (one `open_external` body — keep the `str` version) and the four Minors are the writer's; the parked out-of-scope items: the close-out section exists now (appended after this reviewer started); C1's H8 counts move by two when H9 lands and C1's Task 21 recount is re-run at C2's merge — recorded in H9.

---

## Re-review of fix round 3 (2026-09-09)

Scope as set by the controller: verdict mismatches C, D, E and F, the one Important and the four
Minors; confirm H9(a)'s C1-file edits are exact code and that the two assertion amendments carry
their exact new text; check `google_connected`'s three contract tests assert real behaviour; re-run
the final cross-check against C1 at `c95a6d1`, including whether the new C1 sentences say what R-X-9
and R-X-16 say; flag breakage from this round only; park the unrelated. Plan re-read at 9,243 lines,
concentrated on H9 (`plan.md:535-873`), contracts 3, 6 and 7, and Task 10's connect handler. Every
C1 helper H9 calls I checked against C1's committed text rather than against H9's description of it.

**Verdict: READY TO EXECUTE, with one one-line correction named below.** All four mismatches, the
Important and all four Minors are addressed; R-X-14, R-X-15 and R-X-16 are carried; mismatches A and
G are closed on C1's side. What remains is one internal inconsistency in H9's registration step and
three stale comments in C1 — none of them blocking, none of them code.

### Mismatches C, D, E, F — 4 of 4 ADDRESSED

| # | Mismatch | Verdict | Evidence |
|---|---|---|---|
| **C** | commands registered on the wrong window | **ADDRESSED** | H9 is now explicitly two-phase and says why in C1's own words — "the two `generate_handler!` lists are **different windows**, and on the vault-less shell 'there is no `ConsoleState` yet, so no command that needs one can be called'. `#wiz-google` is on the **wizard** window" (`plan.md:550`). (a6) names the wizard list and rules out the console one (`plan.md:767`); the file table's `main.rs` row says "registers … on the **wizard** list and one on the **console** list" (`plan.md:544`); phase (b) puts `set_google_calendar` on the console list and says why (`plan.md:778-780`, `:814`). **One residue in (a6) itself** — see *New breakage*. |
| **D** | `google_connect_url` took a `ConsoleState` the wizard has none of | **ADDRESSED** | The signature is now `pub fn google_connect_url(scope: String) -> Value` with no state at all (`plan.md:574`), and the doc says why (`plan.md:563-567`). I verified every helper it calls against C1 at `c95a6d1`: `PENDING_TARGET` (`c1:4575`), `api_base()` / `anon_key()` / `check_api_base(&str)` (`c1:4390`, `:4590`), `auth_base(api_base: &str)` (`c1:4602`), `TIMEOUT` (`c1:4577`, same module), `open_in_browser` (`c1:4800`), and `valid_access_token_at(auth_base, anon, target, now_unix)` (`c1:4764`) — H9's call `valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, jiff::Timestamp::now().as_second())` is the *identical* form C1's own vault-less commands use (`c1:7936`, `:7967`). `check_api_base` only tests the scheme prefix, so passing the full request URL rather than a bare base is correct and not a misuse. |
| **E** | the vault was written two panels before it exists | **ADDRESSED, by the right route** | The entry now travels as a plan flag: `WizardPlan.google_calendar` with `#[serde(default)]` and the reason (`plan.md:658-665`), one line into the `VaultPlan` literal in `create_vault_in` (`plan.md:671` — and `create_vault_in(root, home, name, plan)` exists at `c1:5671`), a `VaultPlan` field (`plan.md:677-684`), and `ingest_yaml` rewritten to emit both entries at Finish (`plan.md:689-710`). Contract 3 states the rule in one sentence — "**not by writing `config/ingest.yaml` when the button is pressed**: the wizard has no vault on panel 5 (C1 creates it at Finish)" (`plan.md:221`). I checked the byte-compatibility claim myself: C1's test asserts `calendars: []\n` for no calendar, `calendars:\n  - name: personal\n    ics_url: '…'\n` for one, and an `Err` containing "personal calendar address" for a newline in the address (`c1:5434-5447`); H9's rewrite produces all three identically, because it reuses the same `yaml_scalar("personal calendar address", u)?` with `?` propagating. `plan.md:712`'s claim that C1's test passes unchanged is true. |
| **F** | R-X-14's `google_connected` undefined, the caller left open | **ADDRESSED** | `google_connected()` is written in full (`plan.md:600-627`), the endpoint it asks is defined (`GET /google-connect?status=1` → `{connected, scopes}`, `plan.md:6287`), the handler branch is written (`plan.md:6588-6596`), and the panel polls it (`plan.md:745-754`). Nothing is left for the controller to decide. The command returns `calendar` and `gmail` as separate booleans and the listener keys on `status.calendar`, which is what R-X-14's "self-contained in H9" needed. |

### The Important and the four Minors — 5 of 5 ADDRESSED

| Item | Verdict | Evidence |
|---|---|---|
| Important — two mutually exclusive `open_external` bodies | **ADDRESSED** | One body, the `str` one, with the CR/LF and length guards folded in and the reason for each (`plan.md:834-847`). `url::Url` appears nowhere in the plan (0 hits) and `pub fn open_external` appears once. |
| Minor — H9's heading named three files, the body touched seven | **ADDRESSED, and improved** | A table of **eight** files with an owner column (`plan.md:539-548`), adding `app/src/onboarding.rs`, and correctly marking `app/src/main.rs` and `scripts/wizard-check.py` as **the controller's**, not C1's — which matches C1's own statement that `scripts/` is the controller's (`c1:7692`). |
| Minor — `set_google_calendar(vault,false)` had no caller | **ADDRESSED** | (b2) says the UI is C4's, that C2 builds none, and — better than the finding asked — states the bounded meanwhile behaviour: a revoked grant leaves `cloud:google` in place, `/ingest-calendar` answers 404, `calfeed` logs `using snapshot` and keeps the last busy time, and the day still ranks (`plan.md:816`). It is named under *What is NOT in this plan* beside the (c) toggle (`plan.md:9209`). |
| Minor — contract 3 still named `sourceFor` | **ADDRESSED** | "one constant, `CALENDAR_NAMES`, with `personalSource` behind the one name that is a `sources` row" (`plan.md:213`). |
| Minor — contract 7 quoted a "nine functions" C1 no longer says | **ADDRESSED** | The note is gone; contract 7 is now two clauses stating eleven entries and one import-map line (`plan.md:288`). |

### H9(a)'s five C1-file edits — exact code, confirmed

| Edit | Exact code? | Note |
|---|---|---|
| `app/src/account.rs` — `google_connect_url`, `google_connected`, `get_json`, `open_external` | **Yes** | Four complete functions with bodies and doc comments (`plan.md:560-652`, `:834-847`). |
| `app/src/onboarding.rs` — `WizardPlan` field and the `VaultPlan` line | **Yes** | The struct fragment with its attribute (`plan.md:658-665`) and the one literal line (`plan.md:671`). |
| `app/src/scaffold.rs` — `VaultPlan` field and the `calendars:` writer | **Yes** | The field (`plan.md:677-684`) and the whole replacement block for C1's `match &p.personal_calendar` (`plan.md:689-710`), which is the exact region C1's Task 12 step 3 introduces (`c1:5587-5600`). |
| `app/static/index.html` — the live button | **Yes** | Before and after, both as full lines (`plan.md:717`, `:723`), and the before matches C1's committed markup at `c1:7292` character for character. |
| `app/static/console.js` — listener, poll, plan field | **Yes** | The whole listener including the poll loop and its timing rationale (`plan.md:728-757`) and the plan-literal line (`plan.md:762-764`). |

**The two assertion amendments carry their exact new text.** `app/tests/static_assets.rs`: the C1
line quoted is C1's literal (`c1:7155`) and the replacement is two assertions given in full
(`plan.md:853-857`). `scripts/wizard-check.py`: the C1 lines quoted are C1's literal (`c1:512-513`)
and the replacement is given in full (`plan.md:860-866`), correctly attributed to the controller
rather than to C1. H9 also checks the third C1 assertion still holds and explains why — the commands
are `google_connect_url` / `google_connected` / `open_external`, none of which is the literal
`"connect_google"`, and `gmail.readonly` never reaches the device (`plan.md:869`). I verified that
against C1's assertion text at `c1:7157`: it holds.

### `google_connected`'s three contract tests — they assert real behaviour

- **"reports what was granted, and mints no nonce"** (`plan.md:6448-6462`) is the one that matters,
  and it is a real negative: `saveState` is replaced by a counter and the test asserts `minted === 0`
  **and** `reply.url === undefined`, with the reason written out — "a poll that minted a nonce every
  three seconds would fill `google_state` and leak a fresh URL". That is a behaviour, not a
  restatement. I checked the handler makes it true: the `status` branch sits after the `DELETE` arm
  and **before** `scopeFor`, and returns without touching `saveState` (`plan.md:6593-6596`).
- **"on an account that never connected is `connected:false` and no scopes"** (`plan.md:6464-6469`)
  pins the empty case, which is what the wizard's poll sees for the first fifty-odd seconds.
- **"distinguishes a gmail-only grant from a calendar one"** (`plan.md:6471-6478`) asserts
  `scopes.includes(CALENDAR_SCOPE) === false` on a `[GMAIL_SCOPE]` grant, with the reason — "a
  student can untick one on the consent screen and a bare boolean would let the panel say the
  calendar is on when it is not". This is the test that makes `{connected, scopes}` the right reply
  shape rather than a boolean, and the device half keys on it (`plan.md:621`, `:748`).

The neighbouring `?scope=drive` → 400 test (`plan.md:6442-6446`) is not shadowed by the status
branch, because the status check returns before `scopeFor` is reached.

### FINAL cross-check against C1 at `c95a6d1`

| Point | Verdict |
|---|---|
| interfaces item 3 table | **Agrees.** C1's third row now reads "`google_calendar` \| **nobody** — a reserved value (R-X-9): a Google grant has no URL for `url_ciphertext`/`url_iv`, so it lives in C2's `google_accounts`, never in this table \| **C2** — `/ingest-calendar` resolves `name=google` from `google_accounts`, not from a row here" (`c1:175`). That says exactly what R-X-9 says and exactly what C2 says (`plan.md:209-211`). **Mismatch A closed** — with two comment leftovers, below. |
| the calendar panel and `#wiz-google` | **Agrees, verbatim** (`c1:184-187` vs `plan.md:714-724`). |
| `validate_for` / `DEVICE_KINDS` | **Agrees**, and C1's doc comment is now itself R-X-9-shaped: "`google_calendar` is a reserved value nobody writes (R-X-9) — a Google grant has no URL and lives in C2's `google_accounts`" (`c1:6294-6297`). C1's `putSource` 403 (`c1:3658`) and its Deno test (`c1:3556`) back it at the endpoint. |
| the scheduler contract | **Agrees**, now word for word including the placeholder: C1 `--log-dir <the profile's logs folder>` (`c1:205`) and C2 the same (`plan.md:279`). The cosmetic drift I flagged last round is gone. |
| the `sources` DDL | **Agrees** (`c1:1152-1157` vs `plan.md:193-196`), and C1's `comment on table` now says "google_calendar is reserved and never written: a Google grant has no URL and lives in google_accounts (C2)" (`c1:1159-1162`). |
| the `corrections` DDL | **Agrees**, unchanged since round 2. |
| the `cloud` CI job | **Agrees**, unchanged since round 2. |
| `config.toml`, eleven functions | **Agrees**: C1 says "its **eleven** functions" (`c1:264`), C2 supplies eleven entries appended to C1's eight (`plan.md:439`), no name collides. |
| **mismatch G** | **Closed.** C1 now reads: "the vault copies **stay** as the offline fallback that keeps `ingest` at exit 0 (R-X-16); C3 or C4 removes them, not C1 and not C2" (`c1:180`), which is R-X-16 exactly and agrees with C2 (`plan.md:225`, ledger `:95`). |

**Three mismatches remain, all C1-side comment text, none blocking and none code:**

1. **`c1:3641`** — the doc comment above the TypeScript `SOURCE_KINDS` still reads "`google_calendar`
   is written by **C2's** `google-callback` through this same endpoint", twelve lines above the
   `putSource` comment that now says the opposite (`c1:3653`: "a reserved kind **nobody writes**
   (R-X-9)") and seventeen above the 403 that enforces it (`c1:3658`).
2. **`c1:5953`** — the vocabulary test's inline comment still reads "the device writes only two of
   the three: `google_calendar` is C2's, from a token this machine never sees". The assertion beneath
   it is correct; only the sentence attributes the row.
3. **`c1:182`** — "C2's hand-off **H9** carries the button's enablement, its click listener and the
   `google_connect_url` / `open_external` commands" now names two of H9's four commands;
   `google_connected` (R-X-14) and `set_google_calendar` arrived after that sentence was written.

### New breakage introduced by this round

- **Important — H9(a6) tells the controller to register two wizard commands where three are needed.**
  "(a6) `app/src/main.rs` — the WIZARD window's list gains `account::google_connect_url` and
  `account::google_connected`" (`plan.md:767`), and the file table's `main.rs` row says the same
  ("registers **two** commands on the wizard list and one on the console list", `plan.md:544`), and
  the `account.rs` row says "adds **three** commands" (`plan.md:541`). But `open_external` is a
  fourth command in `account.rs` and its own doc says "It joins the **wizard** window's list beside
  the two commands above" (`plan.md:827`), and the counts paragraph requires it: "H9 adds **three**
  commands … plus `open_external` on the wizard — **four in total**, so C1's 43 / 23 / 55 becomes
  44 / 26 / 59" (`plan.md:871`). The arithmetic there is right — console 43+1=44, wizard 23+3=26,
  overlap unchanged at 11, so 44+26−11=59 — which is precisely what shows that (a6) is one short.
  A controller applying (a6) literally leaves `open_external` unregistered and the button opens
  nothing on its second `invoke`. **Fix:** name all three in (a6), say "three commands" in the
  `main.rs` table row and "four commands" in the `account.rs` row, and drop the "three … plus" phrasing
  at `plan.md:871` for a plain four.

Nothing else regressed. I re-checked what this round's edits could plausibly have disturbed: the
`?status=1` branch does not shadow the `?scope=` 400 or the `DELETE` arm; `var WIZ_GOOGLE` hoists to
module scope so the plan literal at `plan.md:762` sees it whichever function builds the plan; the
listener's runtime `button.disabled = true` cannot break C1's static assertion, which reads
`index.html` and not the DOM; `check_api_base` on a full URL behaves as intended; and H9's
`ingest_yaml` rewrite leaves C1's three existing assertions on that function passing.

### Out of scope — for the controller to park, not to reopen

- The three C1 comment leftovers above (`c1:3641`, `:5953`, `:182`) — three sentences, no code.
- C1's H8 CLAUDE.md text still bakes 43 / 23 / 55; H9 says not to hand-edit and to re-run C1's
  Task 21 recount at C2's merge (`plan.md:871`), which is the right instruction — but the literal in
  C1's H8 will read wrong until that recount lands.
- The controller's message cites C1 at 8,384 lines; the committed file is 8,387 at `c95a6d1`.

### Closing verdict

**READY TO EXECUTE.** Across three fix rounds this plan closed four Critical findings, fifteen
Important ones, nine cross-check mismatches with C1 and every ruling from R-C2-1 to R-X-16, and it
closed them in place rather than by argument — the two narrowings it kept (the ICS parse, the event
roster) are argued in the plan's own words and were accepted as rulings, and the two it took later
(seed-only evals, the engine-side proposal cap) are ledger rows with their costs written down. The
work that remains is one line in H9(a6) naming `open_external` alongside the other two wizard
commands, and three stale sentences in a C1 file that the controller owns; neither can produce a
wrong program if the executing session reads H9's own counts paragraph, and neither touches the
engine, the service, the frozen references or the invariants. Tasks 0–4 are still a coherent first
increment and still close the judgment gap on their own; the back half is now written to the same
standard as the front; and the two things I would still watch during execution are the ones the plan
itself names as unverified — Supabase's edge-runtime behaviour under the hand-rolled PostgREST
client, and whether the pinned Haiku model advertises `output_config.format` when Task 3's smoke
first runs.

### Not re-examined

Everything outside H9, contracts 3/6/7 and Task 10's connect handler; C1 only at the eight named
cross-check points plus the six helper signatures H9 calls. Still nothing executed — no `cargo`,
`deno`, `supabase` or `git` command beyond reading `git log` to confirm `c95a6d1` — so the runtime
questions carried forward from earlier rounds (Supabase edge behaviour, Google Calendar's response
shapes, Vault's PostgREST reachability, `UPDATE … FROM LATERAL`) remain assessed from reading.

## Controller close-out (2026-09-09)

- Three rounds; the final re-review verdict is READY TO EXECUTE with one one-line correction, applied by the controller: H9(a6) now registers all three wizard-window commands (`google_connect_url`, `google_connected`, `open_external`). The three C1-side comment sentences the cross-check named are edited by the controller in the C1 plan (R-X-9 wording in the `SOURCE_KINDS` doc comment and the vocabulary test; the interfaces prose now names the three commands and the `WizardPlan.google_calendar` flag).
- **R-X-17 (H9 at merge):** H9(a) lands as one controller commit immediately after C1's merge and before C2's own merge, with C1's `static_assets` and `scaffold` suites and `scripts/wizard-check.py` run in that commit and C1's Task 21 recount re-run; if any part cannot land cleanly, H9 is deferred whole to C4 — the personal calendar still arrives by its secret address and `google-connect` simply has no caller until then.
- Out-of-scope items parked with rulings: the close-out sections now exist in both reports; C1's H8 command counts move with H9 and Task 21's recount is the mechanism; nothing else open.
- The plan is final and is committed with this review.

---

## Re-review of the R-OB amendment (2026-09-09)

Scope: the amendment as a unit — Task 7a (R-OB-1 + R-C2-8), Task 8a (R-OB-3), hand-offs H11 and H12,
contract 4a, the three new fidelity rows and exit-gate items 6a and 7b — against §11a's newest rows,
D11, the write invariants, the ownership list, the frozen references and the testing rules; plus the
three questions the controller put (double-apply, "strictly before today", new breakage). Plan
re-read at 10,449 lines, concentrated on `plan.md:78-84`, `:270-283`, `:908-1076`, `:5419-6194` and
`:6661-…`. Every claim about existing code I checked against the source: `write::to_literal` and
`ISO_DATE_LIKE` (`engine/src/write.rs:158-181`), `write::create` (`:379-428`), `sync_tasks`'
signature and its `ctx` shadow (`engine/src/ingest.rs:560-576`), `record_seen` / `load_seen`
(`:445-470`), and `coursework.rs`'s use of both (`:19`, `:310`, `:415`).

**Verdict: NEEDS FIXES — the amendment is well-reasoned and R-C2-8 is honoured to the letter, but
R-OB-3's `first_run` flag is computed from a file that the *previous step of the same slot* creates,
so the ruling does not fire on the first slot it was written for.** One Critical, two Important, four
Minor.

### Fidelity to §11a's new rows

| Row | Verdict | Note |
|---|---|---|
| **R-OB-1** — an unknown book is a proposal, never a silent skip | **Honoured, and placed well.** The policy sits in `handler.ts` where the routing already was: `pickZybooks`'s `unmapped` branch proposes instead of warning (`plan.md:5572`), and the handler translates `parse_vhl`'s own `section … not in config; skipped` warning through `VHL_UNMAPPED` (`plan.md:5546`) rather than editing the parser. That is the right call and it is the reason **both parsers stay byte-identical to their frozen references** — which I confirmed from the Files list: Task 7a modifies `handler.ts` and `engine/src/coursework.rs` and nothing else (`plan.md:5425`-region). |
| **R-OB-3** — a first ingest never creates a past-due task | **Honoured in shape, defeated in practice** — Critical 1. The mechanism is right: one `create` into `archive/` plus one `record_seen`, never skipped and never created-then-deleted, with the reasoning for both rejections written out (`plan.md:963-970`). The device is correctly named the authority over the server's `past_due_uids` because it knows the vault's timezone (`plan.md:910`). |
| **R-OB-2** — the sign-in window seeds `courses/` and `course_map` | **Correctly not built.** Named as C1's, with C2's `/judge-task` identified as the answer meanwhile and `Heuristics::knows_course` cited as degrading to "no course rather than a wrong one" (`plan.md:83`). |
| **R-OB-4** — the IPEDS school list | Not C2's, not touched. Correct. |
| **R-C2-8** — no round-trip of `config/ingest.yaml` | **Honoured to the letter, and this is the best work in the amendment.** The first draft's `serde_yaml_ng::to_string` is named as the overruled version rather than quietly replaced (`plan.md:82`), and the replacement is a real text-level insertion with four helpers (`indent_of`, `is_filler`, `find_key`, `block_end`, `child_indent`) that respect the file's own indentation instead of a constant (`plan.md:6000-6050`). `every_byte_outside_the_inserted_lines_is_unchanged` is the assertion that makes it a property rather than an intention. |

### D11, the write invariants, ownership, the frozen references, the testing rules

- **D11 — credentials never leave the device: holds.** Nothing in 7a or 8a widens the request body.
  A proposal carries `{source, key, label, suggested_course}` and no credential; `redact`'s allowlist
  is untouched, so `the_coursework_payload_carries_no_credential` still covers the new path.
- **Journal first: holds.** Every note write in the amendment goes through `write::create` /
  `write_literals` / `delete` — the card (`plan.md:5940`), the archived import (`plan.md:1000`), the
  card's stamp-and-archive (`plan.md:5979-5985`). The config insertion is not a note and, under
  R-C2-8, deliberately not journalled; the plan says where the audit trail lives instead — "the
  *card* it came from is journalled through `write` like every other note" (`plan.md:5952`).
- **The insertion touches no other byte: holds, and is tested.** `write_mapping` splices into a
  `Vec<String>` from `text.split('\n')` and rewrites exactly one existing line, only in the
  `courses: {}` case (`plan.md:6119-6123`). `pystr::read_text`/`write_text` keep the vault's line
  endings, and the plan says so (`plan.md:6079-6082`). The `split('\n')`-not-`lines()` choice is
  correct here because `read_text` has already normalised CRLF — CLAUDE.md's trap avoided.
- **A numeric VHL section id stays a quoted string key: holds — I verified it against the real
  code.** `write::to_literal(&Yaml::String("2102121"))` takes the `Value::String` arm; `ISO_DATE_LIKE`
  is `^\d{4}-\d{2}-\d{2}(T…)?$` (`engine/src/write.rs:158-159`), which `2102121` does not match, so it
  falls to `serde_json::to_string` → `"2102121"` **with quotes**. The plan uses exactly that call
  (`plan.md:6106`), explains why ("an unquoted `2102121:` is an integer key and
  `yaml::get(sections, "2102121")` then misses"), and `an_empty_flow_mapping_becomes_the_block_form`
  proves it end to end by loading the written file with `serde_yaml_ng` and reading
  `["sections"]["2102121"]["course"]` back (`plan.md:5754-5756`). That is the right way to test it.
- **Ownership: clean.** `engine/src/ingest.rs` and `engine/src/approvals.rs` are controller files and
  are H11 and H12 with exact code (`plan.md:908-1058`, `:1059-1074`); `engine/src/coursework.rs` and
  `engine/tests/cloud_contract.rs` are C2's, so `write_map_card`, `apply_map_cards` and
  `write_mapping` correctly live in a task. **Nothing engine-shaped is hidden in a task**: I checked
  Task 7a's and Task 8a's `git add` lines and they stage only `handler.ts`, `handler_test.ts`,
  `engine/src/coursework.rs` and C2's own test files. H10 → H13 renumbering is consistent.
- **Frozen references: untouched.** No task in the amendment reads, writes or regenerates one, and
  the parser files are not modified — which is precisely why the VHL half is done by translating a
  warning in the handler.
- **Testing rules: held.** No new socket, no network. Task 8a's `ingest_on_a_fresh_vault_archives_
  past_due_items` lives in C2-owned `engine/tests/cloud_contract.rs` and the plan states it is **red
  until H11 lands** — the right disclosure for a test whose subject is a controller file. The oracle
  claim at `plan.md:5883` ("the fixture vaults carry no `coursework:` block, so nothing here runs on
  them") is sound for 7a, and 8a is safe for a different reason worth adding: `oracle.rs` exercises
  `cli::run`, and `ingest` is a separate command that the oracle never invokes.

### Critical

**1. `plan.md:1039` — `first_run` is computed from a file the previous slot step has already created,
so R-OB-3 does not fire on the first slot.**
H11 computes `let first_run = !vault.join("state").join("ingest-seen.md").exists();`. But
`state/ingest-seen.md` is **one ledger shared with `coursework`**: `coursework.rs:19` imports
`crate::ingest::record_seen` and calls it at `coursework.rs:310` and `:415`, and `record_seen`
writes that exact path (`engine/src/ingest.rs:460-470`). The slot order is
`coursework → ingest → judge → rank`. So on a genuinely fresh vault the sequence is:

1. `coursework` fetches, syncs, calls `record_seen` → **`state/ingest-seen.md` now exists**;
2. `ingest` computes `first_run` → **`false`**;
3. every past-due Blackboard item is created live — the exact symptom §11a's diagnosis names.

The flag is only ever true when `coursework` wrote nothing first — which is the state Quinn's run was
in *because* the books were unmapped. So the amendment's two halves cancel: R-OB-3 works today and
stops working the moment R-OB-1 fixes the mapping. **Fix:** compute the flag from something `ingest`
alone owns. The seen ledger already discriminates by uid shape — coursework writes `zybooks:…` and
`vhl:…:…`, Gmail writes `gmail:…`, and the ICS feed's uids are the feed's own
(`_blackboard.platform.gradebook2.GradableItem-…`, `engine/src/ingest.rs:950`) — so
`let first_run = crate::ingest::load_seen(vault).iter().all(|u| u.starts_with("zybooks:") ||
u.starts_with("vhl:") || u.starts_with("gmail:"));` is a one-line change with no new file and no new
state. Whichever form is chosen, H11's comment must stop saying "this vault has never ingested if it
has no seen-ledger", because that ledger is not `ingest`'s alone.

### Important

**2. `plan.md:1073` and `:6174` — a rejected map card is re-proposed on the next slot, not in 30 days.**
H12 says a rejected card "is archived by `process_approvals` as any rejection is … and the service
proposes it again **after the card expires in 30 days**". It cannot: `write::delete` settles the card
into `archive/`, so `approvals/map-<source>-<slug>.md` no longer exists, and `write_map_card`'s only
guard is `write::create`'s `Exists` on that path (`plan.md:5936-5938`). The next `coursework` run
finds the book still unmapped, gets the same proposal, and creates a fresh card — twice a day,
forever, each one charging the 15-a-day proposal budget through `defer_over_budget`. This is the same
"a line on every run trains the reader to ignore the list" failure H12's own comment is written to
avoid, in card form. Task 12 already solved the identical problem for rule cards with
`existing_rule_ids(vault)`, which scans `approvals/` **and** `archive/`. **Fix:** give
`write_map_card` the same guard — scan both folders for a card whose `map_key` matches — and, if
"ask again next term" is really wanted, gate the re-ask on the archived card's `decided_at` being
more than N days old rather than on nothing at all.

**3. `plan.md:5504` versus `plan.md:5540` — `suggestCourse`'s headline test fails against the
implementation the same step ships.** The test asserts
`assertEquals(suggestCourse("UACS100Fall2026"), "cs-100")`. The regex is
`/([A-Za-z]{2,4})[\s-]?(\d{3})/`, unanchored, with a **greedy** `{2,4}`: at index 0 it takes `UACS`,
the optional separator matches empty, `\d{3}` matches `100`, and the result is **`"uacs-100"`**. No
backtracking rescues it, because the match succeeds as written. The other four assertions do pass —
`PH106Spring2027` → `ph-106` (only `PH` is available), `cs-100-2026` → `cs-100`, and the two nulls.
So exactly one assertion fails, and it is the example named in the ruling and in the task's own title
sentence. **Fix, and I recommend the first:** (a) assert `"uacs-100"` and say in one line why a
mechanical suggestion is allowed to be wrong — the whole point of the card is that a human confirms
it, and `write_mapping` never writes an unconfirmed value; or (b) if the institution prefix must be
stripped, that needs a real rule (the course code is a *suffix* of `UACS`, which no regex over this
string can know), which is more machinery than a suggestion deserves.

### Minor

**4. `plan.md:6108-6110` — the card-written label and the wizard-written label differ for the same
section.** `write_mapping` derives `label` mechanically from the course slug
(`course.trim().to_uppercase().replace('-', " ")`), so `gn-103` becomes `GN 103`. C1's wizard-written
mapping in contract 4a's own example is `label: GN 103 Hausaufgaben` (`plan.md:272`), and that label
prefixes every VHL title the parser produces. Two mappings for one section therefore yield two title
prefixes depending on which path created it. The derivation is argued ("one field on the card is one
decision, and this one is mechanical") and I would keep it — but say so beside the example in
contract 4a, so the difference is a decision and not a surprise.

**5. `plan.md:5931`, `:6108` — `MapProposal.label` is carried to the card and then ignored.** The card
renders it, `write_mapping` derives its own. Either use it (it is the vendor's own name for the book)
or say why it is display-only.

**6. `plan.md:5996` — an unwritable mapping retries forever with no ageing.** When `write_mapping`
returns `Err` (no `coursework.vhl:` block), the card is kept `approved` and one line is logged, every
run, indefinitely. `approvals.rs`'s comparable case — a recoverable amendment refusal — writes the
status back to `pending` precisely so the card is counted, escalated and eventually expired
(`engine/src/approvals.rs:1360-1368`). Do the same here, or say why a permanent line is preferable.

**7. `plan.md:978-982` — R-OB-3's mid-day residue is real and should be stated where the ruling is.**
"Strictly before today, never before *now*" is the right choice and I would make it too: archiving a
9 a.m. item at 2 p.m. would hide live work. But the consequence is that a vault created at 14:00
still imports this morning's 09:00 deadline as a live, already-overdue task — which is one of the
four shapes §11a's diagnosis complains about. The fidelity row states the rule (`plan.md:81`); it
should also state the residue in the same sentence, so nobody reads exit-gate item 7b as a promise
that a first page never shows an overdue item.

### The controller's three questions, answered

- **Can `apply_map_cards` loop or double-apply?** Not through the config: `write_mapping` scans the
  target block for the key and returns `Ok(false)` before opening the file, so a re-approved card, a
  hand edit that got there first, or a card whose archive failed all converge on "already mapped;
  file untouched" (`plan.md:6126-6136`), and `a_second_apply_of_the_same_key_is_a_no_op` asserts the
  bytes are identical. The success path stamps and archives, and `sorted_md` only scans `approvals/`,
  so an archived card is never re-read. The loop that *does* exist is the proposal loop — finding 2.
- **Is "strictly before today" right?** Yes, with the residue in finding 7.
- **New breakage:** findings 1, 2 and 3 are all introduced by this amendment; nothing that was right
  before it is broken by it. I re-checked the three places the amendment reaches into settled work:
  `sync_tasks`' new parameter (its three existing callers all pass `false`, which is their current
  behaviour, `plan.md:1057`); H3's two `fetch_ics` call sites, which now destructure a tuple and bind
  `_past` deliberately (`plan.md:1052-1055`); and H12's arm, which sits beside H6's in the same
  `status == "approved"` chain and is a complete `else if` block.

### Out of scope — for the controller to park

- Exit-gate item 7b and the fidelity row for R-OB-3 will both need a word if finding 1's fix changes
  how `first_run` is derived; they currently describe the file-existence test by name.
- `IMPORTED_PAST_TEMPLATE` hard-codes `created_by: blackboard`, which is right for the ICS path it
  serves but means the constant cannot be reused if a second feed ever needs the same treatment.

### Not re-examined

Everything outside the amendment — Tasks 0–7, 8–15 and the H1–H9 hand-offs are unchanged from the
round-3 text I verdicted READY TO EXECUTE, and I re-read only the regions the amendment touches.
Still nothing executed: the regex analysis in finding 3, the slot-order analysis in finding 1 and the
`to_literal` quoting in the invariants section are all from reading the source, not from running it —
though finding 3 is the kind of thing one `deno test` would settle in a second, and finding 1 one
`ls state/` after a first slot.

---

## Re-review of the amendment fix round (2026-09-09)

Scope: verdict the seven findings from the R-OB re-review; re-derive `suggestCourse` by hand on all
nine cases against the regex as written; check `is_first_run`'s placement against the slot order and
the oracle fixtures; check that `asked_map_keys` cannot re-mint on the same slot; new breakage this
round only. Plan re-read at 10,646 lines at the changed regions (`plan.md:81-82`, `:280`, `:915-1010`,
`:5536-5600`, `:5727`, `:5971`, `:6050-6079`, `:6318-6330`, `:6874-6937`, `:7072`). Every claim about
existing code re-checked against the source: `engine/src/cli.rs:404`, `app/src/scheduler.rs:188`,
`engine/src/coursework.rs:19,310,415`, `engine/src/ingest.rs:445-470`, and the three fixture vaults'
actual contents.

**Verdict: NEEDS ONE FIX — six of the seven findings are addressed, and R-C2-9's reasoning is exactly
right, but the predicate is written against the wrong path: `today.md` lives at
`vault/state/today.md`, not at the vault root.** 1 Critical, 0 Important, 2 Minor.

### The seven findings

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| **1** | Critical — `first_run` derived from a ledger `coursework` writes first | **NOT ADDRESSED** | The diagnosis is right and beautifully written (`plan.md:919-926` names `coursework.rs:19`, `:310`, `:415` and the slot order), and the ruling R-C2-9 row exists (`plan.md:82`). **But `is_first_run` is `!vault.join("today.md").exists()` (`plan.md:939`) and the engine writes `vault/state/today.md`** (`engine/src/cli.rs:404`: `let output = vault.join("state").join("today.md");`). See below. |
| **2** | Important — a rejected map card is re-proposed every slot | **ADDRESSED** | `asked_map_keys(vault, today)` scans `approvals/` **and** `archive/`, filters on `kind: coursework-map`, drops cards whose `expires` is past, and keys on `(source, map_key)` (`plan.md:6056-6079`). It is computed **once before the loop** (`plan.md:6322`) with a comment that names the exact defect — "`write::create`'s `Exists` guard only sees `approvals/`, and a rejected card is in `archive/`". `write_map_card` mints `expires:` 30 days out (`plan.md:6009`, `:6017`), H12's sentence is corrected (`plan.md:1106`: "It is **not** re-proposed on the next slot"), and `a_rejected_map_card_is_not_re_asked_until_it_expires` (`plan.md:5727-5751`) covers both sides of the boundary. |
| **3** | Important — `suggestCourse`'s headline test failed against its own regex | **ADDRESSED** | Rewritten anchored with an institution-prefix rule (`plan.md:5594-5600`) and nine cases (`plan.md:5539-5548`). I re-derived all nine by hand — all nine pass. Working below. |
| **4** | Minor — card-derived and wizard-derived labels differ | **ADDRESSED** | A new bullet in contract 4a: "**The two writers derive `label:` differently, and that is a decision, not a slip**" (`plan.md:280`), with C1's carrying what the student typed and C2's derived mechanically. |
| **5** | Minor — `MapProposal.label` carried and ignored | **ADDRESSED** | Same bullet; the proposal's label is display-only on the card and the written label is derived, stated rather than left to be discovered. |
| **6** | Minor — an unwritable mapping retried forever with no ageing | **ADDRESSED** | The card is written back to `pending` so it ages, escalates and expires, matching `approvals.rs`'s recoverable-refusal path — `assert!(card.contains("status: pending"), "back to pending, so it ages and expires")` (`plan.md:5971`). |
| **7** | Minor — R-OB-3's mid-day residue unstated | **ADDRESSED, verbatim where I asked for it** | In the R-OB-3 fidelity row itself: "the residue, stated so nobody reads exit-gate 7b as a stronger promise than it is, is that a vault created at 14:00 still imports this morning's 09:00 deadline as a live, already-overdue task, and that is the right side to err on" (`plan.md:81`). |

### Critical 1, in full — the predicate is one word wrong

The engine's only writer of that file is `cli::run`:

```rust
let output = vault.join("state").join("today.md");        // engine/src/cli.rs:404
```

and every reader in the crate agrees (`cli.rs:936`, `:962`, `:979`, `:1080`). The amendment's
predicate looks at the vault **root**:

```rust
pub fn is_first_run(vault: &Path) -> bool { !vault.join("today.md").exists() }   // plan.md:939
```

No code in this repository ever creates `<vault>/today.md`, so `is_first_run` is **`true` on every
run of every vault, forever**. The failure has flipped rather than closed: R-OB-3 went from never
firing to always firing. Later runs are then not "unchanged" as the fidelity row promises — a feed
item that arrives already past due on run 200 is silently archived as `imported-past` instead of
being created as the live overdue task the row says "the system exists to shout about". Nothing is
double-archived (the uid is in the seen ledger), so the damage is bounded and one-directional, but it
is exactly the behaviour the ruling carved out.

**Three things hide it, and all three are worth naming:**

- **The tests manufacture the file production never writes.** `plan.md:6898` —
  `std::fs::write(dir.join("today.md"), "# Today\n").expect("rank wrote today.md")` — and
  `plan.md:6933`, with the comment "What `rank` leaves behind at the end of that same slot." `rank`
  leaves behind `state/today.md`. The suite is green and the field is broken: the same defect class
  the C1 review caught four times (a source-reading test written against a remembered file), here in
  its state-reading form.
- **The ruling's own wording invited it.** R-C2-9 is phrased "the absence of `today.md`, not of
  `state/ingest-seen.md`" (`plan.md:82`) — one path bare, the other prefixed. The writer implemented
  it literally.
- **The cited precedent is itself wrong.** `plan.md:929-931` argues the two halves "now agree by
  construction rather than by coincidence" by pointing at
  `app/src/scheduler.rs:188 — pub fn needs_first_run(vault: &Path) -> bool { !vault.join("today.md").exists() }`.
  They do agree; both are wrong. That app-side bug is out of scope for C2 (see below) but it is the
  authority this change leans on, so it cannot stay unremarked.

**Fix — one word, four sites:** `!vault.join("state").join("today.md").exists()` at `plan.md:939`;
the two test writes at `plan.md:6898` and `:6933` (and their comments); and the R-C2-9 row's own
phrasing at `plan.md:82`, so the next reader is not sent down the same path. Everything else in the
change — the diagnosis, `sync_tasks`' new parameter, `run_lines`' call, the server half, the doc
comments, the exit gate — is correct and needs nothing.

**Placement against the slot order, once the path is right:** correct, and this is the good part of
the design. `rank` is the **last** step of `coursework → ingest → judge → rank` and the only writer
of `state/today.md`, so the first `ingest` always sees it absent and every later one sees it present.
It is a file `ingest` shares with no other step, which is precisely what the seen ledger was not.
**Against the oracle fixtures:** unaffected either way — `vault-full/state/` holds `calendar.md`,
`events-seen.md`, `events.md`, `journal`, `runs`; `vault-s1-migrated/state/` holds `journal`, `runs`;
`vault-s1` has no `state/` at all — so none has a `today.md` under either reading, and `oracle.rs`
shells out to `rank` and never runs `ingest`. Step 8's claim survives the fix; see Minor 2.

### `suggestCourse` re-derived by hand — nine of nine pass

`R = /^([A-Za-z]{2,8})[\s_-]?(\d{3,4})(.*)$/` on `name.trim()`;
`TERM = /(?:Spring|Summer|Fall|Winter)\s?\d{4}\s*$/i`;
`dept = TERM.test(tail) && letters.length > 2 ? letters.slice(2) : letters`.

| Input | letters | sep | number | tail | TERM | len>2 | dept | Result | Expected |
|---|---|---|---|---|---|---|---|---|---|
| `UACS100Fall2026` | `UACS` (greedy stops at `1`) | `` | `100` (no 4th digit) | `Fall2026` | ✓ | ✓ | `CS` | **`cs-100`** | `cs-100` ✓ |
| `UAMATH120Fall2026` | `UAMATH` | `` | `120` | `Fall2026` | ✓ | ✓ | `MATH` | **`math-120`** | `math-120` ✓ |
| `PH106Spring2027` | `PH` | `` | `106` | `Spring2027` | ✓ | **✗ (2>2 false)** | `PH` | **`ph-106`** | `ph-106` ✓ |
| `MATH125` | `MATH` | `` | `125` | `` | ✗ | — | `MATH` | **`math-125`** | `math-125` ✓ |
| `cs-100-2026` | `cs` | `-` | `100` (`-` ends it) | `-2026` | ✗ | — | `cs` | **`cs-100`** | `cs-100` ✓ |
| `HowToUseZyBooks2` | greedy `HowToUse`, then `Z` is not a digit; backtracks 7→2, every next char a letter | — | — | — | — | — | — | **`null`** | `null` ✓ |
| `2102121` | `^[A-Za-z]{2,8}` fails at index 0 | — | — | — | — | — | — | **`null`** | `null` ✓ |
| `SomeBookWithNoCode` | no digits anywhere after any letter run | — | — | — | — | — | — | **`null`** | `null` ✓ |
| `""` | needs ≥2 letters | — | — | — | — | — | — | **`null`** | `null` ✓ |

The `^`/`$` anchoring is what makes case 6 correct — the old unanchored form would have matched
mid-string had digits existed — and the `letters.length > 2` guard is what keeps `PH106Spring2027`
whole. The regex is right as written and the nine assertions all hold. One untested shape, Minor 1.

### `asked_map_keys` cannot re-mint on the same slot — checked

Two independent reasons, and either alone is sufficient. (a) `apply_map_cards` runs **before** the
fetch and stamps an applied card `executed` before `write::delete` moves it to `archive/` — the
`expires:` it was minted with is untouched by that write, so a card applied minutes earlier is ≤30
days old, passes `expires < stamp`'s filter, and is in the set `collect_cloud` then consults
(`plan.md:6322`). (b) Once the mapping is in `config/ingest.yaml`, the device sends it in the
redacted config, `routeZybook` answers `Mapped` and `parse_vhl` finds its section, so the server
returns no proposal for that key at all. The set is computed once per run, not per proposal, so there
is no path by which minting a card in the loop changes what the loop sees. The `Exists` guard on the
card path remains as a third backstop for the same-folder case.

### New breakage this round — none beyond Critical 1

I checked each edit for collateral: `sync_tasks`' parameter and its three test callers are unchanged
from the round that introduced them; `run_lines`' `first_run` binding moved from a `state/` stat to
`is_first_run(vault)` and is still computed before the sync; H3's two `fetch_ics` sites still
destructure the tuple; `asked_map_keys` adds a read of `archive/` that no other caller shares; and
the `status: pending` write-back in Minor 6 reuses the literal path `apply_map_cards` already had.
The renumbering of H11's changes (old 1–5 → new 2–6) is consistent throughout.

### Minor

**1. `plan.md:5595-5598` — the case that distinguishes the rule from a coin flip is untested and
mishandled.** `MATH125Fall2026` — a department code with a term and no institution prefix — gives
letters `MATH` (4 > 2) and `TERM.test("Fall2026")` true, so `dept = "TH"` and the suggestion is
**`th-125`**. The doc's justification says the guard "leaves `MATH125` … alone", which is true only
because that example carries no term; adding one breaks it. The wrongness is explicitly allowed
(`plan.md:5587-5589`: the card is confirmed by a human and `write_mapping` never writes an
unconfirmed value), and I would keep the rule — but add `MATH125Fall2026` as a tenth case asserting
`th-125` with a one-line comment, so the limit is a decision rather than a surprise found in the
field.

**2. `plan.md:7072` — Step 8's oracle sentence now gives a reason that will read as wrong after the
fix.** "none of the fixture vaults has a `today.md`" is true under both readings, but the load-bearing
reason is the one the sentence puts second: `oracle.rs` shells out to `rank` and never invokes
`ingest`, so `is_first_run` is never evaluated on a fixture vault at all. Lead with that, and the
sentence survives any future change to the predicate.

### Out of scope — for the controller

- **`app/src/scheduler.rs:188` has the same wrong path.** `needs_first_run(vault)` is
  `!vault.join("today.md").exists()`, so it is always `true` and `scheduler::spawn` fires a slot at
  **every** console launch, not only after onboarding. That is C1/controller territory and predates
  this amendment, but §11a's calendar row leans on `needs_first_run` to make the wizard's finish show
  today's page, and the C2 amendment now cites it as its precedent — so the two should be fixed in
  one commit, not two.

### Closing verdict

**Not yet — one word.** Everything the fix round set out to do it did, and did well: the rejected-card
loop is closed with the same `approvals/`-plus-`archive/` shape Task 12 already proved, the
`suggestCourse` regex is now anchored and correct on all nine of its cases (I re-derived every one by
hand rather than taking the table on trust), all four minors are addressed in the places I asked for
them, and R-C2-9's *reasoning* about why the seen ledger cannot be the predicate is the clearest
paragraph in the amendment. But the predicate it reaches for is `vault/today.md`, and this engine
writes `vault/state/today.md` — so `is_first_run` is permanently true, R-OB-3 fires on every run
instead of none, and two tests that write the root file by hand keep the suite green while it does.
The correction is `!vault.join("state").join("today.md").exists()` at `plan.md:939`, the same prefix
in the two test writes at `:6898` and `:6933`, and the same in R-C2-9's own wording at `:82`; with
those four edits and the two minors above, the amendment is sound and the plan as a whole is READY TO
EXECUTE. I would also have the controller fix `app/src/scheduler.rs:188` in the same commit, since it
is the precedent this change cites and it is wrong for the identical reason.

### Not re-examined

Everything outside the amendment's changed regions; the plan's main body is unchanged from the text I
verdicted READY TO EXECUTE in round 3. Nothing executed — the regex derivation, the slot-order
analysis and the path check are all from reading the source, though the last of them is one
`ls <vault>` away from settling itself, and I would run `cargo test -p knowlu-engine --test
cloud_contract first_run` against a real post-slot vault before believing any predicate here again.

## Controller close-out of the amendment (2026-09-09)

- The amendment fix round's last finding — `is_first_run` checked `vault/today.md` while the engine writes `vault/state/today.md` (`cli.rs:404`) — was applied by the controller at the four sites the review named (plan lines 82, 939, 6898, 6933): **R-C2-10, the predicate is `!vault.join("state").join("today.md").exists()`.** The same wrong path was in the app's `scheduler::needs_first_run` on `main` (the precedent the plan cited); fixed there test-first the same evening.
- With that, every finding of the amendment rounds is addressed; `suggestCourse`'s nine cases were re-derived by hand by the reviewer; `asked_map_keys` cannot re-mint within a slot. READY TO EXECUTE after C1.
