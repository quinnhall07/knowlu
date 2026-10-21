# HANDOFF — where Knowlu stands and how the next session runs it

**Rewritten 2026-09-09 (evening) by the session that signed the spec, ran cut day and executed C0
Tasks 1–3. Facts refreshed 2026-10-18 for session 5 (the friends-MVP push): the lane table in §2,
the counts, §3's Integrate items and the Pilot measure; the session-5 plan is
`.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md`.** Read this whole file, then `CLAUDE.md`, then `docs/specs/2026-09-09-knowlu-cloud-design.md`
(signed; §11a holds the rulings made after signing), then the plan you are executing. Everything
older under `docs/` is history or design authority from the previous repository (`PROVENANCE.md`).

---

## 1. State, in facts

▶ **C1b and C1c MERGED 2026-09-24, C3′ MERGED 2026-09-25, each on Quinn's word, each green on the self-hosted runner. Live billing is proven on prod (2026-09-24).**
- **CI runs on GitHub's runners again (2026-09-29).** Quinn made the repository public that day, which makes Actions minutes free. The same day the controller deleted the `CI_SELF_HOSTED` variable, deregistered the `knowlu-laptop` runner and set fork-PR approval to *all external contributors*: a self-hosted runner on a public repository runs any fork's pull request on the laptop. **Do not re-register it while the repository is public.** Quinn plans to make it private again later; only then is the laptop runner safe to bring back (`C:\actions-runner\start-knowlu-runner.cmd`, labels `[self-hosted, windows, x64, knowlu-ci]`, PR #10 `312d833`).
  - `release.yml` never runs self-hosted.
  - Public since 2026-09-29: the fixtures' real coursework titles and a few personal paths in docs are visible (history included). Quinn's ruling: develop freely now, go private later; no scrub for now. No secret is in the pushed history (scanned 2026-09-29: the only token is the production anon key, public by design).
- **C1b, sign-in:** PR #9, `7477ec5`. Continue with Google or an emailed code; no password.
- **C1c, the first day:** PR #13, `f30da69`.
  - The first-run view, the per-source coursework archive, and a doubled first-day proposal cap (migration `20260923000100`).
  - The slot refreshes the account session before its first cloud step, with a 45-minute floor (R-C1c-13). Before this, a one-hour token was usually already expired when a noon or 6 pm slot began, a bug latent since C2.
  - Every refresh-and-save in the app runs under one process-wide `SESSION_REFRESH_LOCK` (R-C1c-exec-14), so two threads never replay one rotating refresh token.
- **C3′, the account vault:** PR #15, `e2ce40c` (2026-09-25); merge of main `825f5f4`. Git leaves the product.
  - `engine/src/history.rs` is deleted, and the account is the source of truth for a student's vault.
  - `knowlu-engine sync` (always exits 0) pushes this device's journal records and changed note text and pulls other desktops' records through `write`. It runs once a slot, first (`sync → coursework → ingest → judge → rank`), once on *Sync now*, and once on quit (push only). *Sync now* and the quit push refresh the session first (H17).
  - `sync_records`, `sync_notes` and `sync_usage` hold the copy in plain text, encrypted at rest, under RLS.
    - The ceiling is 200 MiB per account.
    - The journal is kept 400 days, except records that set a value or create a note other than by Knowlu's own automatic steps. The server decides `keep`.
    - `DELETE /account` purges all three tables.
  - A real conflict becomes a sync card on that device.
  - `state/seed-hashes.json` tells an untouched wizard seed from real history, so signing in on a new computer restores the account's copy into the wizard's vault without overwriting either.
  - The engine gate (`engine/src/entitle.rs`) names `sync`, `coursework`, `ingest` and `judge` as skips past the 72-hour grace. `rank` keeps running.
  - The LMS feed and the personal calendar URL leave the vault for any account (`ics_url: ''`, `cloud:personal`).
  - The privacy page is version **2026-09-24**, and `PRIVACY_VERSION` moves with it. Its wording is the controller's, delegated by Quinn. Both prod accounts that accepted the 2026-09-16 version are Quinn's, so no re-ask screen is needed.
  - Staging carries (2026-09-30) the four migrations `20260922120100`, `20260922120200`, `20260926000100` and `20260929000100`, plus `20260929000200` (Gmail); `judge-task`, `judge-event`, `judge-email`, `gmail-read`, `sync-push`, `ingest-calendar` and `google-connect` were redeployed 2026-09-30; smokes S1-S8 passed (Integrate's staging gate passed 2026-09-30).
  - Every task was reviewed. The final whole-branch review (opus) was followed by one fix wave (18 rows), and its re-review reads **ready with residuals, no open rows**.
  - The gate on the merge: 1451 / 0 / 4, 0 other; deno 461/0; wizard-check and settings-check ok.
  - The ledger, the whole-branch review with its fix wave and merge report, and Task 12's stream report are preserved as `docs/reports/2026-09-25-c3-account-vault-{sdd-ledger,whole-branch-review,stream-report}.md`. C1b's, C1c's and the runner's are beside them (`2026-09-24-c1b-sign-in-*`, `2026-09-24-c1c-first-day-*`, `2026-09-23-ci-self-hosted-sdd-ledger.md`). All four streams are closed: worktrees and branches deleted, and the PRs keep the record.
- **Live Stripe is proven end to end on prod (2026-09-24).** Quinn activated the live account. A prod `POST /billing-checkout` answered a `cs_live_` session. Quinn paid with the 100% promotion code, and the webhook wrote the entitlement about 4 s later: monthly, `trialing`, period end 2026-10-01.
- **Production is still at C1 level plus live billing.** The ordered release checklist is §4's production-parity row.

▶ **C2 MERGED 2026-09-16 (`666d458`, `--no-ff`, on Quinn's word; PR #4; CI on `main` run 35074260458 green). Executed and reviewed 2026-09-14/16 (Tasks 0–15 on branch `c2-judge`, CI green on every commit; the whole-branch review's fix wave landed as six commits `34b1d86`…`df9b91a`; re-reviewed on the most capable model with every item addressed and no new Critical or Important breakage, its Minors fixed in a small second round and re-reviewed clean, and the controller's live smoke of that round proven on staging (the device pass included)). The `c2-judge` worktree and branch are deleted; the SDD workspace is deleted after its ledger was preserved.** The judgment service: eleven edge functions live on **staging** (`brvhgbihxevrudqpulcm`) — `judge-task`, `judge-event`, `judge-email`, `judge-rules`, `ingest-coursework`, `ingest-ics`, `ingest-calendar`, `events`, `google-connect`, `google-callback`, `gmail-read` — behind the nine C2 migrations `20260911000100`…`000900` (`_judgment_service`, `_google`, `_google_privileges`, `_rule_promotion`, `_rule_promotion_fix`, `_caps_privileges`, `_eval`, `_eval_fix`, `_final_review_fixes`). **Provider swap MERGED 2026-09-16 (PR #5, `e58b2c8`, `--no-ff` on Quinn's word; CI on `main` run 35163879798 green; the ledger preserved as `docs/reports/2026-09-16-inference-provider-swap-sdd-ledger.md`).** **The pinned model, re-pinned 2026-09-16 (provider swap Task 2/4, R8 amended — `docs/specs/2026-09-09-knowlu-cloud-design.md`'s Amendment section):** from `_shared/judge_models.ts` and `cloud/supabase/migrations/20260916000100_provider_swap.sql`, every kind routes through `provider = 'openrouter'`, pinned per request to one named zero-retention upstream with fallbacks off — `task` and `event` are `ibm-granite/granite-4.2-8b` (bf16) at **CoreWeave**, `usd_per_m_in` **0.10**, `usd_per_m_out` **0.15**; `email` is `qwen/qwen3.5-35b-a3b` (fp8) at **DeepInfra**, `usd_per_m_in` **0.14**, `usd_per_m_out` **1.00**; sampling `{"temperature": 0, "reasoning": {"enabled": false}}` on all three, prompt/grammar versions `task-2`/`event-2`/`email-2`. Per-student cost: about **$0.24 a month for a typical student and $0.87 for a heavy one**, against **$1.73 and $6.44** on the previous Anthropic `claude-haiku-4-5` pin (computed from `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` §2's volumes at the pinned prices). **Device side:** `engine/src/cloudmodel.rs`'s `CloudModel` implements `judge::Model` + `EventModel` + `EmailModel` behind one HTTPS call per item; `engine/src/enrich.rs`'s `run_lines_with` runs **four cloud passes** in one slot — the tier-3 judge pass, the events pass (`events::judge_roster`), the Gmail pull (`pull_gmail`) and the rule-decision pull (`pull_rules`) — each a named line and exit 0 on every failure shape (no config, no session, no network, 402, 429, 5xx, timeout). The wizard's Google button (hand-off H9 **phase (a)** only — `google_connect_url`/`google_connected`/`open_external`, the `calendars:` entry, the `#wiz-google` listener) is on the branch; **phase (b)** (the console window's `set_google_calendar` settings row) is deferred whole to C4 by ruling R-C2-E6 — the `calendar_ics` fallback covers the gap meanwhile. **Fixed 2026-09-16 (C2 final review A-5, m59+m60):** the flow's own state (connected, mid-poll, its note) moved onto `WIZ` and is painted by `renderWizard()` on every re-render — a direct DOM write no longer loses it to a Back, a Next or a stale poll — `wizGo` cancels a poll in flight on leaving the calendars panel, and `wizFinish` re-reads `google_connected` once more before building the plan so a consent that finished after the last poll tick still births the vault with the `cloud:google` entry. **`scripts/wizard-check.py` was re-run by the controller at `df9b91a`** (a fresh Playwright venv at `.wv/`, git-ignored, the script's own convention): `ok`, exit 0 — a regression walk of the nine panels with the rewritten `console.js`; the Google button itself is covered by the static assertion in `app/tests/static_assets.rs` (`the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it`). **The whole-branch review (2026-09-15/16, four area passes on the most capable model; `docs/reports/2026-09-16-c2-whole-branch-review.md`, the ledger beside it) found three Criticals — `monthly_spend` readable with the anon key (verified live; revoked in `…000900`, and `migrations_test.ts` now guards every view), the Google callback's `id_token` dependency (the consent now asks `openid email` beside the one API scope — P2 must know), and H5 turning `ingest` into a permanently failing step for a cloud vault with no LMS feed (a 404 is now the named exit-0 skip `ingest: no LMS feed on this account — skipped`) — and fifteen Importants, every one fixed in the six-commit fix wave (rulings R-C2-E52…E54); the residual minors are listed in the report, each with a home.**

**The eval suite is EMPTY by Quinn's 2026-09-14 decision (ruling R-C2-E12): nothing is ever read from the archived vault.** `cloud/eval/seed/` holds only a README stating the decision; cases arrive later from consented corrections through C4's class-(c) opt-in toggle. `run_eval.ts` reports `0 cases` and **exits 0** while both the local seed and `eval_cases` are empty — vacuously green, by design, not by omission. **Corrected 2026-09-16 (C2 final review A-4): the gate now actually wakes up.** The old check stopped at the empty local seed unconditionally, so it could never see a real case land in `eval_cases` no matter how many consented corrections arrived — a gate that would stay vacuous forever, not merely until real cases accumulated. It now asks (only when the local seed is empty) whether `SUPABASE_SERVICE_ROLE_KEY` is set, and if so whether `eval_cases` holds anything at all; a fork PR (no secrets) still answers exactly as before, but a PR with the staging key runs the suite for real the day `eval_cases` has a row.

**Smoked live on staging by the controller:** `ingest-coursework` (24 fixture assignments + the `surprise-101` mapping proposal), `ingest-ics`/`ingest-calendar` (every named refusal — no source, no grant, unknown name, no bearer), `events` (a real page fetched through `Deno.resolveDns`; every SSRF bypass host refused), `google-connect`/`google-callback` (the 503 with no client id, the declined/expired/error pages, a connect→reconnect→read→disconnect cycle against the test account with the privilege ACLs re-verified after each fix), `gmail-read` (the named quiet reply with no grant), `judge-rules` (empty array). **The device side proven 2026-09-16 at `df9b91a`:** a scratch copy of the s1 fixture with a `config/cloud.yaml` and no `ics_url`, the session credential written as the JSON blob under the vault's `session_credential_target`, `PUT /account/sources` with a public iCal — `knowlu-engine ingest` exited 0 with 317 events from the server-fetched feed (through `_shared/guarded_fetch.ts`; 152 tasks created, 165 past items archived), and the scratch source row, credential and vault were removed after. After the fix wave: `GET /rest/v1/monthly_spend` with the anon key answers 401 (it answered 200 before), `cron.job` carries `knowlu-sweep-gmail-queue` (07:31) and `knowlu-prune-gmail-seen` (07:33), and `google-connect?scope=calendar` with a session still answers the named 503 (P2 unset). **Found by that smoke, not by any review:** Supabase's relay rewrites an HTML response to `text/plain` on the shared `*.supabase.co` domain (its docs: HTML is served only from a custom domain, a Pro add-on), so the Google callback's consent pages reached the browser as raw source; ruling R-C2-E56 made them plain sentences (`nosniff`, `default-src 'none'`), and a custom functions domain or a redirect to a page on `knowlu.com` is C4's decision for Quinn once the site is public (m148). **`judge-task`/`judge-event`/`judge-email` wait on nothing now: `OPENROUTER_API_KEY` is set on staging and in GitHub (2026-09-16, provider swap Task 4), so a real model call can be made.** P2's client id and secret are on staging (09-16); the live Google consent click and a real Gmail read wait on **P3** (Quinn as the first test user). The eval-gate job (`.github/workflows/ci.yml`) needs two GitHub secrets Quinn sets, named only: `OPENROUTER_API_KEY`, `SUPABASE_STAGING_SERVICE_ROLE_KEY` — both set 09-16; `ANTHROPIC_API_KEY` is deliberately unset everywhere. **Live check 2026-09-16 (provider swap Task 5, the controller):** the ten functions whose bundles changed redeployed; ten synthetic tasks and ten synthetic emails through `judge-task`/`judge-email` with a staging session — 20 × HTTP 200, every task verdict from `ibm-granite/granite-4.2-8b` (`task-2`) and nine of ten emails from `qwen/qwen3.5-35b-a3b` (`email-2`) with reasoning off and valid JSON on every call; four relative deadlines resolved against the Date line; the empty message below the confidence floor by the new rule; the password-reset mail classed as information. The same ten emails through the fallback `deepseek/deepseek-v4-flash-0731` at DeepInfra (a temporary row, restored to the migrations' state after): six answered, four withheld under the floor (the study-group event among them) — **Qwen stays** (ruling R-PS-10; DeepSeek remains the documented fallback; its two filled opportunity deadlines are a prompt note for the eval). Measured prompts: task ≈ 200 in / 63 out tokens a call, email ≈ 372 / 86 — about a quarter of the scoping note's ceilings, so the real per-student cost is nearer $0.06 a month than $0.24. The synthetic judgment rows were deleted from the account after the evidence was read; `usage_daily` keeps the accounting. **P3 done 2026-09-17 — the first real Google consent cycle, run by the controller from staging sessions with Quinn at the browser:** the calendar consent → `GET /ingest-calendar?name=google` (157 events over the 28-day window, all timed, CRLF ICS inside the JSON envelope) → the incremental Gmail consent (the row's scopes then carried both) → `POST /gmail-read` (25 judged in 42 s on Qwen, 1 deferred, `more: true`; a second round after the fix below judged 29 more with 0 deferred; 55 email judgments in all, under half a cent) → the calendar re-consent → the disconnect. It ended with no grant, no queue rows, no seen ids and no nonces on the account, and Knowlu absent from Google's third-party-access page. **Two defects found by that cycle and by no review (PR #6, branch `google-reconnect`; each fixed test-first and proven live from the branch before its commit):** (1) the calendar ask was sent with `include_granted_scopes=false`, so the re-consent Testing mode forces every seven days made Google issue a calendar-only refresh token while `google_accounts.scopes` (a union) still claimed Gmail — `gmail-read` answered 500 in one second on Gmail's 403, which the device prints as `gmail: skipped (…)` on every slot with nothing a student can act on; now `true` on every ask (`google-connect/handler.ts`), and a fourth consent's success page named both Calendar and Gmail with the read back at 200. (2) `serviceDb().rpc` called `.json()` on every reply and `delete_google_grant` returns void, so `DELETE /google-connect` answered 502 "could not be reached" after the revoke had happened and the delete had committed — the C2 smoke of 09-15 ran the delete through SQL, never through the deployed DELETE; `record_tokens` returns void too, its caller swallowing the throw; now `rpcReply` (`_shared/judge_db.ts`) reads an empty body as `null`, and a fresh grant's DELETE answered `{"disconnected":true}`. **PR #6 MERGED 2026-09-17 (`0b6710f`, `--no-ff` on Quinn's word; CI on `main` run 35178305871 green), and every function that bundles `_shared/judge_db.ts` redeployed to staging from `main` the same day (account, gmail-read, google-callback, google-connect, ingest-calendar, ingest-ics, judge-email, judge-event, judge-rules, judge-task) — staging runs `main`.** The 55 `judgments` rows of origin `gmail_api` (the model's own words about Quinn's mail, never the text; excluded from the training export) stay unless Quinn says otherwise.

**Production has nothing from C2 yet** — staging and production (`jxthohvwrijwtuwlglan`) are separate projects. Every one of the **ten** migrations (the nine C2 migrations plus the provider swap's `20260916000100_provider_swap.sql`) and all eleven functions must be applied there, in the same order, before launch, plus production's own `OPENROUTER_API_KEY`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` secrets, a production Google OAuth client with its own redirect URI, and the two consent-screen scopes submitted in order — `calendar.readonly` first (sensitive: lighter review, no CASA), `gmail.readonly` second (restricted: behind verification and the annual CASA). **The whole-branch review is done and the branch is merged (2026-09-16); production waits for cut day.**

▶ **C0: PR #3 MERGED 2026-09-14** (`1b7bbea`, `--no-ff`, on Quinn's word): the updater key rotated (`97D52FF797E70A1B`), the README's Releases section, Cloudflare Pages + `knowlu.com`, the Azure app registration. **Azure Artifact Signing no longer blocks `v0.1.0` (2026-09-17).** Quinn cannot complete Trusted Signing's identity verification for the time being ("no access to ID"), and ruled that the app must work without the signature until then, with signing still the goal. **Update 2026-10-16: Quinn has an ID now, so the identity check no longer blocks Azure signing (`docs/notes/2026-09-29-funding-and-seed-grant.md`); 2026-10-18: friends still get an unsigned CI release (SmartScreen click-through) and signing waits for the paid release.** PR #7 (`c0-unsigned-release`, **MERGED `dcd26aa` on Quinn's word, CI on `main` run 35179340785 green**) is the switch: the repository variable `RELEASE_AUTHENTICODE` set to the literal `off` skips the three Azure steps, the release ships without Authenticode (SmartScreen warns on the installer; the updater's minisign signature is untouched), `verify Authenticode` asserts every binary is `NotSigned` instead of skipping, and the release notes open with **Unsigned build.** Unset means signing is required, so a missing Azure line still stops a release rather than shipping unsigned by accident; `engine/tests/workflows.rs` pins the gate to the variable and to no secret. **`v0.1.0` tagged 2026-09-17 03:53Z on Quinn's word (annotated, on `c2e0eca`; the variable set 03:48Z).** Release run 35179886527 proved the unsigned path on the runner — the three Azure steps skipped, `UNSIGNED:` per file, `Knowlu_0.1.0_x64-setup.exe` 5.09 MB with its `.sig` and `latest.json` (the updater key works there), engine 4.4 MB through the size gate, `verify Authenticode` printing `UNSIGNED RELEASE` and `NotSigned` ×3, the notice atop the notes — and its first attempt FAILED at the Cloudflare Pages deploy (`upload-token` "Authentication error [code: 10000]": the 09-14 `CLOUDFLARE_API_TOKEN` was the wrong token). Quinn re-set the secret 04:40Z. **Attempt 2 (`gh run rerun --failed`, 05:24Z → 06:05Z, 41 minutes: 22 compiling tauri-cli, 16 building) PUBLISHED v0.1.0 on 2026-09-17** — verified from outside: `https://knowlu.com` answers 200 (the privacy page at `/privacy`; the `.html` form 308s to it), `https://knowlu.com/releases/latest.json` names 0.1.0 with the versioned installer URL and its minisign signature, `Knowlu_0.1.0_x64-setup.exe` and the stable `Knowlu-setup.exe` both download (5,342,137 bytes), and the GitHub Release `https://github.com/quinnhall07/knowlu/releases/tag/v0.1.0` carries the installer, the `.sig` and `latest.json` with **Unsigned build.** as its first paragraph. **The release path exists. Task 5 Step 3's install half is done (2026-09-17, by the controller on Quinn's word: the Release asset — byte-identical to the site's copy, `NotSigned` — installed silently to `%LOCALAPPDATA%\Knowlu\`, `knowlu.exe` 0.1.0 with `knowlu-engine.exe` beside it, uninstall entry `HKCU\…\Uninstall\Knowlu`, never launched; SmartScreen's More info → Run anyway was not exercised because a shell download carries no mark-of-the-web). Next: Quinn launches it — tray → *Copy diagnostics* → `Knowlu 0.1.0 build c2e0eca` — then Step 4 (the 0.1.1 bump on Quinn's word, the updater proof), then Task 6.** **PR #8 MERGED `274e137` 2026-09-17 on Quinn's word** (`--no-ff`; main CI run 35192587097; the worktree and branch removed) is the speed work Quinn asked for while the rerun ran, and 0.1.1 carries it: the Tauri CLI downloaded prebuilt and pinned by SHA-256 (`scripts/ci/tauri-cli.ps1`) instead of compiled, `cache-on-failure` on the release cache, and `release.yml` split into a `build` job and a `publish` job joined by an uploaded artifact, so a publish-only failure reruns in minutes. Azure stays on the queue as the goal: the four lines → the secret + three variables → delete the variable → the next tag ships signed. The C0 ledger's ▶ RESUME HERE block has the older steps. **Jev System One (assessed 2026-09-22; superseded the same day by six notes and stream J):** the event kind is closed on measured evidence — no corpus, and published calibration too weak to carry a threshold (`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §3); Jev's best product case is completion detection, but Blackboard's templated receipts make that evidence rule-tier, so a model would only ever see a small residue (`docs/notes/2026-09-22-completion-detection-design.md` §7b); any product use of Jev still needs the D-R8 provider gate. For development, hooks and MCP beside the unmodified Claude Code binary are permitted but not recommended unless experiment E2 (the review-triage replay, `scripts/experiments/e2-review-replay/`) shows value (`docs/notes/2026-09-22-jev-in-the-development-workflow.md`).

▶ **C1 DONE and MERGED 2026-09-10** (`82dcc04`, `--no-ff`, first parent `8c0e05a`, PR #2; 60 commits, every task reviewed, the whole-branch review said merge; the merge-time hand-offs H4/H5/H6/H8 in `512567d`; CI on `main` run 34547538926 green): accounts, entitlement and the new wizard. Supabase `knowlu-staging` and `knowlu-prod` are live; Stripe is in test mode on staging with the monthly and academic-year prices, the 7-day trial and the June–August pause, and live mode is prepared on prod (prices, portal, webhook) pending Quinn's fresh live secret key. `GET /entitlement` is cached on the device with a 72-hour grace and a slot with no subscription says `judge (skipped: no entitlement)` and stays green. The wizard is nine panels with no folder question; an install from before C1 is adopted in place. Telemetry (a) and (b) go up at each slot; issue reports are previewed and scrubbed before they are sent. The privacy policy and the terms are written and committed to `site/`, not yet live at `knowlu.com`. **Google's restricted-scope verification has not been submitted** — P4 waits for the site to be live; Gmail stays testing-mode only until it lands. LMS link capture: outcome **B** from the Task 13 spike (the sign-in window's cookies fetch the feed URL in one GET and the course list in another). The stream report: `docs/reports/2026-09-10-c1-accounts-stream-report.md`; Quinn's two packets (P4 texts, P5 facts): `docs/reports/2026-09-10-c1-quinn-packets.md`; the merge-time applications (H4, H5, H6, H8) are on `main` in `512567d`; the SDD workspace and the `c1-task16` worktree are deleted; the empty `.claude/worktrees/c1-accounts` directory is held open by the merging session and goes when it ends. **Checkout proven end to end on staging 2026-09-14** (after Quinn rolled the three exposed keys and set the sandbox's two public URLs): a fresh sign-in code, `POST /billing-checkout` with the rolled test key, the test card, the webhook writing `entitlements` (`trialing`, monthly, period end 2026-09-21) three seconds after payment, the `auto_renew` consent row at $9.99 / terms 2026-09-10, `GET /entitlement` flipped, `POST /billing-portal` opening. Sandbox mail from `onboarding@resend.dev` lands in Gmail's spam; Resend's domain verification (DNS records only, no live site needed) is the fix. **Prod rolled out 2026-09-14** on Quinn's instruction (they pasted the prod database password and the new Resend key in chat, to be rolled before launch): the seven migrations, all eight functions (`--use-api`), all ten function secrets, the auth config (SMTP), the two Vault entries (a stale 44-char `billing_jobs_token` from the 09-10 prep was replaced so it matches the function secret), three cron jobs active; every function answers its designed refusal and the job called through pg_net with the Vault token returned 200. Not yet done on prod: a real sign-up (cut day). **The Stripe live Dashboard items were done by Quinn on 2026-09-14** (the two public URLs, the live product's tax code) and **Managed Payments stays on — P3 decided**: Stripe is the merchant of record for tax and disputes at the higher per-charge fee; unverifiable from here until the first live Checkout, which also needs the site to resolve. **Step 5 done 2026-09-14**: `knowlu.com` verified in Resend (DNS hosting moved to Cloudflare; the registrar stays Namecheap); the sender is **`hello@knowlu.com`** — `EMAIL_FROM` = `Knowlu <hello@knowlu.com>` on both projects, `config.toml`'s `admin_email`; Quinn pushed the auth config on both projects (2026-09-14 ~16:30 UTC); **proven on staging**: a sign-in code from `hello@knowlu.com` reached the Gmail inbox in six seconds, not spam. Cloudflare's zone import carried Namecheap's five `eforward*.registrar-servers.com` MX records and its SPF TXT on the apex, which block Email Routing until deleted (Resend's own records sit on `send.` and `resend._domainkey.` and stay). Quinn deleted them, enabled Email Routing, verified the Gmail destination (Cloudflare's verification mail; rules stay inactive until then — the first test bounced 550 from `mx.cloudflare.net`) and a test to `hello@knowlu.com` arrived in Gmail. **Step 5 complete 2026-09-14**: `hello@`, `support@` and `security@knowlu.com` forward to Quinn's Gmail; there is no mailbox at the domain. The policies also name `support@knowlu.com` and `security@knowlu.com`: Cloudflare Email Routing forwards all three. Staging sign-in mail is proven on the rolled key (a 200 after the old key was deleted). **Step 6 decided 2026-09-14 — keep, all four**: restore-from-backup becomes a link on the picker in C3; the in-app overlay sells monthly only; no magic link in the overlay; export stays by email until a settings row in C3. The repo's CLI is linked back to **staging** (`--password ""`); prod commands take `--project-ref jxthohvwrijwtuwlglan`.

- **This repo:** `quinnhall07/knowlu`, public since 2026-09-29 (private when this block was written). `main` was pushed on 2026-09-09 night (`ce1f9ee`; it had been 49 commits ahead). The day.s commits were: the signed
  spec + VISION §10 + CLAUDE direction (`48b83dc`), spec §11a rulings (`f7f1816`, `b569fc9`), the
  wizard defaults (`18cadd2`), the first slot at launch (`f8649d5`), plans C1 (`58954ef`, `c6a18ca`,
  `c95a6d1`) and C2 (`ede9b70`), the C0 merge (`d67bafa`, `--no-ff`, first parent `ede9b70`), and the
  C0 docs. Gates: PR #1's last run 34356994036 on the runner **1021 passed / 0 failed**, `warnings: 1
  accepted (.rsrc), 2 tallies, 0 other`, `eol contract holds over 223 files`; the local workspace
  run on the merged tree is recorded in the C0 ledger.
- **Evening additions on `main` (pushed with the rest):** the C1 plan amendments `39c7a52` and `234c28b`, the
  review report's execution rulings `823fb83`, this HANDOFF. `main` and `origin/main` agree.
- **Cut day happened, then Quinn asked for a reset.** They quit the old Knowlu, disabled the cloud
  routine, onboarded with the new build into `C:\Users\danie\Knowlu\Vault` (profile "Vault",
  `profile_a95daa1d40`), pressed *Run now* — and the first slot exposed the onboarding gaps recorded
  in spec §11a (R-OB-1…4): coursework skipped as unmapped, every task course-less, four past-due
  imports. **They asked to reset and re-onboard once C1's wizard carries those rulings.** The reset
  (move the vault, `profiles.json` and `profiles\` into `%LOCALAPPDATA%\knowlu-pre-cut-2026-09-09\attempt-1\`,
  delete the two `knowlu/profile_a95daa1d40/*` credentials, remove the autostart entry, copy the
  `19a1a8d` build over `%LOCALAPPDATA%\Programs\Knowlu\`) **was run on 2026-09-09 after they quit
  Knowlu: `%LOCALAPPDATA%\knowlu\` is empty, `%USERPROFILE%\Knowlu\` holds only `Backups\`, the
  installed build is `19a1a8d`, and the next launch opens the wizard.** Nothing is scheduled: the
  old `quinn-ops-local-runner` task is unregistered (its XML is in the pre-cut folder and can be
  re-registered as the interim page until C1 — offered to Quinn, not yet answered). **`quinn-ops` is archived** (2026-09-09 night; its `main` was in sync with origin).
- **A bug of this session, fixed:** the first-run predicate (`scheduler::needs_first_run`) checked a
  root `today.md`; the engine writes `state/today.md`, so the first draft (`f8649d5`) would have run a
  slot at every launch. Fixed test-first in `19a1a8d` (the C2 plan review caught it). The installed
  build predates both commits; install `19a1a8d` with the reset.
- **The old repo `quinn-ops`** (`C:\Users\danie\GitHub\quinn-ops`) is now inert (no task, no routine,
  no running app) but still Quinn's old vault. **Do not build, run or edit anything there** except the
  archive step.
- **Tooling on this laptop:** as before (Rust 1.98 GNU host + WinLibs 16.1.0-r4 MSVCRT, tauri-cli
  2.11.4, Windows SDK 10.0.26100, `gh` as `quinnhall07`), plus **Deno 2.9.6 and Supabase CLI 2.117.0**
  (winget, 2026-09-10; the CLI is logged in and linked to `knowlu-staging`, the database password sits in
  the keyring; every CLI call takes `--workdir cloud`, R-C1-32). No Docker — function deploys use
  `--use-api`, and `db query --linked` is the read-only inspection path.
- **Legal:** unchanged (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`; spec §9).

## 2. What gets built, and how it is kept apart

**The lanes, 2026-09-30 (from the ledgers, after the Gmail merge):** Gmail connect MERGED (#26, `199cd1f`; T12 staging deploy done, T13 live proof is Quinn's). M2 editing MERGED (#27, `30b6fb7`; T10 live proof is Quinn's). Events lane: building, plan on branch `events`. Email forwarding to Knowlu: spec signed 2026-09-30, amendments and plan on main (branch `email-forwarding`); staging receiver (SES, us-east-2) being set up. **2026-10-03: INTEGRATED on `email-forwarding` at `156ee3e` (T17: the engine, app and cloud sub-branches and `main` `43d4b1b` merged, one `ci.yml` step), not merged to `main`; the reference docs (T13b) are written; Checkpoint D and T18 wait on Quinn (item 2a, Open).** Docs-only PRs now report the required checks (#28). B1 closed (event-4 did not ship). The table below is the 2026-09-29 record.

**The lanes now: session 5, the friends-MVP push (2026-10-18).** Quinn's goal: an MVP friends can test for
free, with prod parity and an unsigned CI release that installs and updates. The plan, rulings, Quinn's
block and conflict rules are in `.superpowers/handoffs/2026-10-18-START-HERE-session-5-mvp.md` (§1, §5);
each lane keeps a ledger at `.superpowers/sdd/2026-10-18-<lane>/progress.md`. A lane PR merges on green CI
plus `reviewer` (`contract-reviewer` for the contract list); the email-branch merge, tags, prod deploys and
secrets, money and rule changes still ask Quinn. `main` is `904c902` (2026-10-19: PRs #39, the
ingest time-bomb fix; #40, L2's grades score; #41, the session-5 specs, plans and reviews). The
`email-forwarding` branch is 262 commits ahead of it and 23 behind it (`git rev-list --count`, 2026-10-20),
so the merge is not a fast-forward: L0 merges `main` into the branch first. `git merge-tree` shows one
conflict, `engine/src/coursework.rs`, where both sides pinned the same time bomb (the branch's `0c8230e` and
main's #39); it is off the contract list, so `integrator` resolves it, keeping one pin per test.
`origin/email-forwarding` is stale at `b3df445` (2026-10-01), so the 262 are unpushed (HEAD `2c76305`).
**The proof-only hunks are reverted** (2026-10-20, by file name: `app/src/{main.rs,debug_port.rs,mail.rs,
mail/backfill.rs,mail/live.rs,mail/runner.rs}`). They are kept as a patch in the session's scratchpad, never
in the repo, and are re-applied only to build a proof binary, then reverted again. With them applied, two app
tests are red by design: `mail_debug_port::the_hook_is_debug_only_and_reads_one_variable` and
`mail_window::the_popup_is_filtered_opens_nothing_and_closes_its_window`. Never `git add` those files while a
proof is running. Last workspace runs: 2801 passed / 2 failed / 4 ignored at `1c15e8d` with the hunks (the two
above); the clean-tree run at `2c76305` is recorded in the session-5 ledger.
`main`'s CI is green since #39. No lane PR merges over a red check without Quinn's word. The branch stays the
base every lane below waits for.

**The two signed edits of 2026-10-16 (the record of their signature):** Quinn signed Amendments 2026-10-16 (the 6-month price) and 2026-10-16b (the pilot does not pay; the quiet-release gate) on 2026-10-16, in the Levitetz seed grant session. Their text in `docs/specs/2026-09-09-knowlu-cloud-design.md` and their two `VISION.md` markers (the price line; the pilot paragraph's quiet-release sentence) were applied to the repository on 2026-10-20, on branch `docs/cloud-amendments-1016-1018`. The L8 spec names both amendments as its authority. Their authority lines cite this paragraph, because the funding note that holds the rulings is private.

**Files this section cites that were not committed (2026-10-18 check, `git cat-file -e 0930c67:<path>`).** Untracked:
`docs/notes/2026-10-16-grades-score-not-captured.md`, `docs/specs/2026-10-06-first-day-event-allowance-design.md`,
`docs/specs/2026-10-05-syllabus-class-times-design.md`,
`docs/specs/2026-10-18-friends-testers-design.md`, `docs/plans/2026-10-18-first-day-allowance-plan.md` (L3's plan, T1-T6),
`docs/plans/2026-10-18-p3-registrar-close-out-plan.md` (PR #19's close-out: it merges before L5, see L5a) and the whole of `docs/design/`
(the session-5 handoff §2 names it) exist only untracked in the main checkout (`git status --short` there; none of the seven
(six files and `docs/design/`) passes `git cat-file -e 0930c67:<path>`; this list is the 2026-10-18 state, so re-run `git status --short` in the main
checkout before committing rather than trust it as complete);
`docs/specs/2026-10-10-m365-backfill-redesign.md` and `docs/plans/2026-10-18-c3-m365-backfill-plan.md` only
untracked in the `email-forwarding` worktree. **The controller commits all of them before any lane branches**
(with this file, on the branch that merges first), or a lane's planner cannot find its governing document.
`docs/notes/2026-09-29-funding-and-seed-grant.md` is untracked as well and stays untracked: it is a private note, kept off the
repository because it holds personal details (Amendments 2026-10-16 and 2026-10-16b cite it that way). Never `git add` it.
`.superpowers/` (the session plan, the ledgers) is git-ignored (`.gitignore` line 11) and local to this machine.

| Lane | What | Stage | Agents |
|---|---|---|---|
| L0 Email close-out | **reviewed and passed (2026-10-18, session-5 ledger): `cdeff72`, `4123749`, `68301ec`, none on the contract list; do not dispatch that review again.** Still to review: `ceae879` (the T18c Gmail script fixes and the `thread.js` account-chip parse, which decides whether a thread is the student's own) and `0c8230e` (the coursework pin that closed the review's one Important), so no PR before the reviewer passes both; revert the two uncommitted proof hooks (`app/src/main.rs`, `app/src/debug_port.rs`, see above) after the T18c proof and before the PR; commit the email spec's line-399 fix (the stamp-rule wording) and the redesign note; merge `main` into the branch first (15 behind); T18b pin check, Gmail G1-G3, T18c cleanup; PR, Quinn's go, merge | MVP | controller, reviewer; contract-reviewer if `account.rs`/`approvals.rs` appear |
| L1 C3 M365 backfill | build plan from `docs/specs/2026-10-10-m365-backfill-redesign.md` (signed by Quinn's delegated go, 2026-10-20 (session-5 ledger); all decisions ruled; plan `docs/plans/2026-10-18-c3-m365-backfill-plan.md`), then build; stays guided (D13) if it slips | MVP | planner, reviewer (plan mode), cloud-engineer (the migration and the `mail` function route), contract-engineer + contract-reviewer for `app/assets/mail` `init.js`, test-writer (plan R3a), implementer, console-ui (R11 last) |
| L2 Grades fix | `displayGrade.score` not captured (`docs/notes/2026-10-16-grades-score-not-captured.md`); its inline `json!` v2 regression test goes in `app/tests/grades.rs`, which L4 also edits (`PRIVACY_BUMP_1`), so L2 lands first and L4 branches after it (or L2's test goes in a new file) | MVP | debugger (root cause), test-writer (the failing test first), implementer, reviewer |
| L3 First-day allowance | `docs/specs/2026-10-06-first-day-event-allowance-design.md` (header signed by Quinn, 2026-10-19, session-5 ledger); migration after email's | MVP | planner, cloud-engineer, implementer |
| L4 Honest page + re-consent | `site/privacy.html` to today's app (the interim bump, no lawyer read), `PRIVACY_VERSION` bump, re-consent screen. **This interim bump is the "privacy bump #1" that every gate before the Pilot names** (Quinn's email-ledger rows of 2026-10-18 put bump #1 in wave 1, then took the lawyer read out of it): §3's Release gate and `scripts/ci/privacy-guard.ps1`, the M2 and Gmail specs' conditions, and `PRIVACY_BUMP_1` in `app/tests/grades.rs`, which L4 sets to its version. So its page carries every signed sentence this file owes bump #1, not only the session-5 list (mail, grades, syllabus): the email spec's seven §7 drafts (under "Where these sentences go (D18)", the kept sign-ins paragraph among them; T13a never copied them here; their mailbox-actions sentence is true of Gmail only until L1, because Microsoft 365's older mail is the guided step, D13); M1's grades spec §11 sentences and their pins (§3's MVP lane 1, "What privacy bump #1 owes from M1"); M2's interests sentence and the Gmail connect sentences, with the Gmail spec §8.5's four page tests (both in §4's Pilot row "Privacy bump #1: the lawyer read"); the registrar sentence (L5a); and the testers spec's Q7 sentences if Quinn signs Q7 by L4's page reading (L8). **Not L4's:** the Gmail sender line, which comes with the sender (§3's Pilot). **L4's only if their lane merged first, otherwise that lane's own PR:** L5's syllabus sentence, which L5's amended spec drafts (Q10); and the Microsoft 365 upload row (the M365 redesign's decision 18, signed 2026-10-13: its own row in the email spec §7 table, with §4.9's mailbox-actions sentence), which the C3 plan's R12b puts on the page after L4 (its Q3, open for Quinn, decides whether that edit moves `PRIVACY_VERSION`). Neither L1's upload nor L5's read reaches a release before its sentence is on the page. `privacy-guard.ps1` reads only the version, never the text, so these sentences are held by L4's pins in `engine/tests/site.rs` and by the page reviewer checking them against this list. Owns `account.rs`, **on the contract list**; also sign-out clearing `pending_setups` (the email hand-back) and `app/tests/grades.rs` | MVP | contract-engineer (edits `account.rs`), contract-reviewer, console-ui, reviewer for the page text; Quinn reads the page |
| L5a UA registrar close-out (PR #19, `p3-registrar`) | **Quinn ruled 2026-10-18: #19 merges BEFORE L5** (against the recommendation to rebase behind L5). Plan `docs/plans/2026-10-18-p3-registrar-close-out-plan.md`, tasks B1-B9 (B1 merges `main` into the branch **after the email merge**: a merge, not a rebase; B9 is the live proof at the machine, B9a the registered status codes). R0 spike **done** (2026-10-18; the rows call is `registrationHistory/reset?term`, per-meeting dates on every row, not paged), so it is not asked again. Rulings: matched rows confirmed hard as the student (R24); a hand-set field the registrar changes gets the existing `commitment-check` card (no `AMENDABLE_FIELDS` change); G2-G5 confirmed; the per-source table in `commitments.rs` is built now (B3) and L5 adds a syllabus row; the registrar's privacy sentence rides L4's page ("at schools Knowlu supports"); the CLAUDE.md sentence is added as written, in #19's merge. **The friends release carries the registrar button only if B9's live proof passes before L10's tag and L4's page has the sentence;** otherwise the UA row ships without its registrar entry. Review of `2ad1e3d` (A1) FAILED with two important findings (`withdraw_stale` misses the registrar hold; the registrar writes as the CLI default `quinn`, not the vault token), now phase B tasks. **Files it touches that other lanes own** (`git diff --name-only 0930c67...p3-registrar`): `app/src/main.rs` (L6's: #19 merges first, L6 rebases its `main.rs` edit over it), `app/static/console.js` (a queue slot, B5), `CLAUDE.md` (the sentence), `site/privacy.html` (no edit; the sentence goes through L4), and **`engine/tests/fixtures/registrar/banner-ua-registration.json`, which is on the contract list (`engine/tests/fixtures/**`): contract-engineer edits it, contract-reviewer reviews that diff, never `reviewer` or `implementer`** | MVP, before L5 | controller, planner (plan done), reviewer, contract-engineer + contract-reviewer (the fixture), implementer, mechanical (B9a), console-ui (B5) |
| L5 Syllabus read (was class times) | `docs/specs/2026-10-05-syllabus-class-times-design.md` (**signed by Quinn, 2026-10-19: revision 2, its amendment included** (the spec's status line); **S2 spike done 2026-10-18; Quinn ruled Q10: build the Simple Syllabus read (Offer A) and widen it beyond class times** (grade information, late-work policies, test and quiz dates, assignments), with #19's E1-E10 in the amended spec; its plan starts after L5a merges). The S2 facts: the structured section attributes carry no meeting time, day or room; clock times are free text in 2 of 8 documents; the plain GET outside the page is refused (403), so the read runs in the page. Owns `approvals.rs`, **on the contract list**; also touches `surface.rs` after L7; adds one row to L5a's per-source table | MVP, may slip | planner, contract-engineer (edits `approvals.rs`), contract-reviewer, implementer, cloud-engineer, console-ui |
| L6 Runtime removal | ruling 10; owns `app/src/main.rs` and `commands.rs` (other lanes hand it their Tauri-command registration lines). Deleting `engine/src/runtime.rs` deletes one of CLAUDE.md's four `#[ignore]`d tests (the real-runtime smoke test), by deletion with the module and never by un-ignoring, so the "four tests" line in CLAUDE.md becomes three; **that CLAUDE.md edit is a rule change and goes to Quinn, not into the lane PR** | MVP | planner (small plan), implementer, reviewer |
| L7 Named issues | OpenRouter 402, Gmail "source went quiet"; `surface.rs` before L5. **Built** on `l7-named-issues` (cloud, engine, read model; the console page, T5, still queued), PR pending at T8; `surface.rs` is free for L5 after T8 | MVP | implementer, console-ui |
| L8 Testers | comp entitlement for a named list, per-tester usage and cost view on prod (`docs/specs/2026-10-18-friends-testers-design.md`, signed by Quinn 2026-10-19; its Amendment 2026-10-20 signed by Quinn's Q1, Q7 and §8.3 rulings) | MVP | planner, cloud-engineer |
| L9 Prod parity | §4's checklist, with email's migrations and functions, after the key roll (§4); steps (1)-(2) are re-run after each later lane that adds a migration or function change merges, in merge-stamp order (L3, L8, L5, L1: L3's migration and judge-function redeploys, L8's `<stamp>_testers.sql` and the billing-checkout and entitlement redeploys, before any comp account exists on prod; L11's `20261020000400_mail_follow_ups.sql` and the redeploys of `mail-inbound`, `gmail-read`, `mail-process` and `mail`, its stamp recounted at its T9 under M5); steps (1)-(2) first, then L10's first tag publishes the privacy page (step (3)), then step (4)'s secrets; Quinn's go per prod deploy | MVP | cloud-engineer; the controller runs every prod push |
| L10 Release for friends | unsigned CI release v0.1.x (the first tag also publishes `site/`, so it sits between L9's steps (2) and (4)), install and updater proof, a one-page install note; Quinn's go per tag | MVP | integrator, docs-keeper |
| L11 Email cloud follow-ups | plan `docs/plans/2026-10-20-email-cloud-follow-ups-plan.md` (FINAL, Quinn's rulings Q1-Q5 of 2026-10-20), branch `email-cloud-follow-ups`: `mail-process` entitlement gate and the lapsed hold (T3), Rotate's `rotated_from` and D22 (T4), one count per SES receipt (T5), migration `20261020000400_mail_follow_ups.sql` (T2), spec markers (T7). **Owed: T6** (an out-of-credit answer spends no attempt, one-hour hold, instance pause), held until L7's cloud half is on `main` (C3), then its own small PR on `email-cloud-follow-ups-credit` from `main`; its spec marker (§4.7 step 5) goes with it. **Owed:** unlapse writes `pending` inside an open Rotate grace (plan §9). L9 redeploys `mail-inbound`, `gmail-read`, `mail-process`, `mail` | MVP | cloud-engineer, docs-keeper, reviewer |

Serialisation: `console.js` is one queue (L4, L7, L1 R11, L5a's B5, L5); migrations stamp in merge order (email, L3, L8,
L5, L1; L5a adds none; L11's stamp is `20261020000400`, rechecked at its T9); L5a (#19) merges after the email merge and before L5, and before L6 makes its `main.rs` commit (if L6 is ready first, L6 merges first and L5a's B1 re-applies #19's `main.rs` part, recounting by script); `main.rs` and `commands.rs` are L6's; `account.rs` is L4's and `approvals.rs` is L5's (both on
CLAUDE.md's contract list: contract-engineer edits, contract-reviewer reviews, never `implementer`); `surface.rs`
goes L7 then L5; `app/tests/grades.rs` is L2's first (the `displayGrade.score` test), then L4's
(`PRIVACY_BUMP_1`), though both are wave 1. Every spec a session-5 lane builds against is signed: the syllabus spec and the allowance header (block item 5), the friends-testers spec (Quinn, 2026-10-19) and the M365 redesign (Quinn's delegated go, 2026-10-20); §4's row "Sign the C3 and L8 specs" is closed.

**The lanes then (2026-09-29, from `git branch -a`, `gh pr list` and `git worktree list`; the record, since superseded: Grades #25, Credential Manager flake #21, Human actor #23 and Events #31 are merged, and the Email row's counts are in the paragraph above).** `main` is
`86a3431` and carries Integrate's merges (#11, #12, #14, #16, #17, #18). Commits ahead are `git rev-list --count main..<branch>`.

| Lane | Branch, worktree | Ahead of `main` | State | Stage |
|---|---|---|---|---|
| Grades | `m1-grades` | 4 | live | MVP |
| Credential Manager flake fix | `fix-credman-flake` | 0 (at `86a3431`) | live | Integrate |
| Human actor | `human-actor` | 1 | live | Integrate |
| Events | `j-events` | 5 | kept for the events lane (T3, waits on E1) | MVP |
| Email forwarding | `email-forwarding` (integrated 2026-10-03 at `156ee3e`; sub-branches `email-engine`, `email-app`, `email-cloud`) | 136 (at `156ee3e`, `git rev-list --count origin/main..HEAD`) | integrated, not merged; awaits Checkpoint D | MVP |
| Registrar (commitment model phase 3) | `p3-registrar`, PR #19 | 20 | draft | Pilot |
| Two desktops | `two-desktop`, PR #20 | 14 | draft | Launch |
| Relay | `c5-relay` | 0 (empty) | not started | Launch |

Counts on `main` at `86a3431`: 28 migrations in `cloud/supabase/migrations/`, 21 functions under
`cloud/supabase/functions/` (`_shared` excluded). **Recounted 2026-10-18 by script:** `main` `0930c67` has
31 migrations and 21 functions; the `email-forwarding` worktree has 33 migrations (adds `20261002000200_mail`
and `20261005000100_mail_claim_max_tokens`) and 27 functions (adds `mail`, `mail-inbound`, `mail-process`,
`mail-canary`, `mail-pull`, `mail-resolve`). `app/src/main.rs` on the `email-forwarding` tree registers 29 commands in the setup window's
`generate_handler!` list and 70 in the console's, 89 distinct; on `main` `0930c67` it is 29 and 60, 79 distinct
(the difference is the email lane's console commands, which L6 inherits at the merge). Older counts elsewhere in this file are dated records.

The table below is the record of the C-series and J streams: one worktree each
(`.claude/worktrees/<branch>`, git-ignored), one plan each, disjoint files; shared files only by the
controller at merge. Merge order was **C0 → C1 → C2**.

| Stream | Plan | Branch | Owns (exclusive) | Needs from Quinn |
|---|---|---|---|---|
| **C0 — CI release** | `docs/plans/2026-09-09-c0-ci-release-plan.md` — **Tasks 1–3 DONE and merged**; Tasks 4–6 remain | `c0-ci-release` (merged; **make `c0-release-key` from `main` for Tasks 4–6**) | `.github/**`, `scripts/ci/**`, `scripts/release.ps1`, `scripts/sign.ps1`, `engine/tests/workflows.rs`; `app/tauri.conf.json` only `plugins.updater.pubkey`, `endpoints`, `version` | P4 (retire the old updater key — Task 4's PowerShell block), P1 Azure (**federated credential entity-type Environment = `release`**, not the tag), P2 Cloudflare, P3 `knowlu.com`, the secrets |
| **C1 — accounts, entitlement, the calendars-first wizard, telemetry, issue reports** | `docs/plans/2026-09-09-c1-accounts-plan.md` — **DONE, merged 2026-09-10 (`82dcc04`)** | `c1-accounts` (merged) | `cloud/supabase/**` except C2's function dirs and `_shared/judge_*.ts`; `app/src/{onboarding,profiles,scaffold,credentials,scheduler}.rs`; new `app/src/{account,telemetry,report,lms_link}.rs`; `app/static/**`; `app/tests/**`; `site/**` | Supabase org (prod + staging); Stripe test mode with both prices, the pause, the Portal, the webhook; **an e-mail provider (Resend free tier) for Supabase Auth mail and the annual reminder**; Stripe Tax + Kentucky; the Google consent screen (two scopes); privacy/ToS read |
| **C2 — the judgment service** | `docs/plans/2026-09-09-c2-judge-plan.md` — **DONE, merged 2026-09-16 (`666d458`, PR #4)** | `c2-judge` | `cloud/supabase/functions/{judge-*,ingest-*,events,google-*,gmail-read}`, `_shared/judge_*.ts`, `migrations/20260911*.sql`, `cloud/eval/**`; `engine/src/{judge,enrich,cloudmodel,coursework,judgelog,events}.rs`; new `engine/tests/*.rs` | ~~`ANTHROPIC_API_KEY` in the staging project's secrets~~ — `OPENROUTER_API_KEY`, set 2026-09-16 (provider swap); the Google OAuth client id/secret; Quinn as the first Gmail test user; ~~the one-time go to read the `quinn-ops` archive for the eval seed~~ (declined 2026-09-14, R-C2-E12) |
| **J — judgment quality** | `docs/plans/2026-09-22-judgment-quality-plan.md` — T1, T2, T4, T6, T8, T9 and E2's prep built and reviewed on `j-judgment-quality` (PR pending; rollout order in `docs/notes/2026-09-22-stream-j-rollout.md`); T3 on `j-events` waits on E1; T0 cost matrices ratified 2026-09-22 | `j-judgment-quality`, `j-events` | the event/email regions of `_shared/judge_{prompts,validate,pipeline}.ts`, `_shared/{judge_due,lms_receipts}.ts`, `gmail-read`, `cloud/eval/**`, `engine/src/{completion,events,eventledger,enrich,cloudmodel,coursework,zybooks,vhl}.rs`, `scripts/experiments/**`, migrations `20260922120*` | the OpenRouter key in Credential Manager `knowlu/dev/openrouter` (E1, E2); where `unsure` events surface |
| **Cut day** | spec §7.2 | — | — | the archive step only |
| **Docs** | — | controller, on `main` | `VISION.md`, `CLAUDE.md`, `README.md`, this file, the spec | — |

**Shared files with a single owner:** root `Cargo.toml`/`Cargo.lock`, `app/src/main.rs` (the two
`generate_handler!` lists), `app/src/lib.rs`, `app/src/commands.rs`, `app/src/tray.rs`,
`app/src/updates.rs`, `app/src/state.rs`, `app/tauri.conf.json` outside C0's three keys,
`engine/src/lib.rs`, `engine/src/{main,cli,ingest}.rs`, `scripts/*.py`. Every plan
lists what it needs there under **"Controller hand-offs"** with exact code; the controller applies
them at merge. **H9 of C2** (the wizard's Google button, five C1 files) lands as one controller
commit between C1's and C2's merges, or is deferred whole to C4 (R-X-17).

**Rulings of record made during execution** live in the review reports
(`docs/reports/2026-09-09-c1-accounts-plan-review.md`, `…c2-judge-plan-review.md`: R-C1-*, R-C2-*,
R-X-1…17) and in the C0 ledger (`.superpowers/sdd/2026-09-09-c0-ci-release-plan/progress.md`, R-C0-*).
Two Quinn may want to reverse: **R-C2-4** (corrections become eval cases only under the (c) opt-in —
follows from D5/§6; reversing means amending D5) and **R-X-16** (the vault copies of the LMS and
calendar URLs stay as the offline fallback until C3/C4).

**Controller rules (unchanged, plus what today taught):**
- `superpowers:subagent-driven-development` per plan, one implementer at a time per worktree,
  reviewers in parallel, ledgers in `.superpowers/sdd/<plan>/`. Disjointness by
  `git diff --name-only main...<branch>` before every merge.
- **Implementer subagents stop instead of blocking on a CI run** and no notification comes for the
  run: the controller watches the run itself (`gh run watch <id> --exit-status` in a background Bash)
  and resumes the agent with the result. Every dispatch says so.
- **Every dispatch names the trailer model explicitly** (`Co-Authored-By: <this session's model>`);
  one implementer used its own model name and the commit had to be amended.
- **Never `cd` in the controller's shell**: the working directory persists and a `cd` into a worktree
  re-homes the whole session. Use `git -C <path>` and absolute paths.
- Stop only for an irreversible action, a secret, a spend, a push to a shared branch, or anything
  touching `quinn-ops` (archived). Everything else: rule, ledger, continue. **Standing rule (Quinn,
  2026-09-09 night): `main` may be pushed without asking when it holds only docs commits** (plans,
  reports, HANDOFF, CLAUDE.md); a push carrying code — a hand-off, a merge — still needs the go.

## 3. Sequence: the stages

**▶ RESUME HERE (2026-09-29).** Work moves in stages named for what the student gets, signed by Quinn
on 2026-09-29 (the cloud design's Amendment 2026-09-29, ruling 10): **MVP → Pilot → Launch → Beyond**,
entered through **Integrate**, this session's one-time entry gate (not one of ruling 10's stages).
**This section is the single ordering.** `VISION.md`'s "Build order" states the stages and
`docs/notes/2026-09-29-vision-program.md` maps the MVP's gaps and the phases after it; where either
names an older order, this section wins. The C-series is closed at C3′ and stays as history (the last
subsection); no C-number is issued again and none is renumbered.

Where it stands: **Integrate is done except three items** (below), and the **MVP** comes next,
starting with M1 grades. The controller's running state for the current session is
`.superpowers/sdd/2026-09-29-ultracode/run.md` (local, git-ignored); the lanes are in §2.
The dev app is Quinn's live staging profile (`Test`), left as it is until cut day.

| Stage | The student gets | Exit (all must hold) |
|---|---|---|
| **Integrate** | Nothing new: one trunk that every later build comes from. | #17, #11, #12, #14, #16 and #18 merged with CI green on `main` and 0 other warnings; the Credential Manager flake fixed; ruling 11's human token in place; staging's applied migrations equal the repo's list and the staging smokes pass; Amendment 2026-09-29 signed; §3–§4 in stages. |
| **MVP** | Everything the founder's quinn-ops did, plus grades from Blackboard, proven on a founder-owned scratch profile and test account on staging, with no second person. | The parity audit re-run shows no open row; the founder's grades proof has passed through the app's real gate on a dev build of `main` (the policy-read gate is suspended until 20 paying users: Amendment 2026-09-30, A13; no read or date is needed); Quinn says "MVP reached". |
| **Pilot** | A few undergraduates other than the founder, on the released app, one computer each, needing nothing from anyone. | Everything under "Pilot" below has shipped, and the measure Quinn sets at entry is met (proposed by the controller; ruling 10 names no exit, an open question below). |
| **Launch** | Any US undergraduate can pay $9.99 and use Knowlu on more than one desktop. | Everything under "Launch" below is done (proposed by the controller; ruling 10 names no Launch exit, an open question below). |
| **Beyond** | Each VISION phase that makes the morning answer better. | No stage-wide gate. Per feature: a signed spec, a plan, a review, a merge and a live proof. |

A stage's exit holds before the next stage begins (ruling 10).

### Integrate: done except one check (items 1-3 closed; the refund-migration row in §4 is still open)

**Done 2026-09-29.** Merged green, in this order: #17 (`fix-literal-dashes`), #11 (stream J, T0–T9),
#12 (J's follow-ups), #14 and #16 (the commitment model's phases 1–2, with P21's sync of
`commitments/`), then #18 (the vision, the agent roster, the settings, the signed Amendment 2026-09-29
and the VISION fixes). `main` is `86a3431`. Required checks on `main` since the same day: `test`,
`cloud`, `eval-gate` (strict off; admins not enforced, so a docs-only direct push still works; never
merge with `--admin`). `p3-registrar` and `two-desktop` are open as draft PRs #19 and #20, as a record
only. `m1-grades` is pushed with no PR.

**Merged 2026-10-01 (facts from `git log origin/main` and the ultracode run ledger):** #28 (the docs-only
CI fix: docs go by PR again), #29 (the grades policy-read gate suspended until 20 paying users, A13), #30
(docs) and #31 (events, MVP). The MVP lanes now on `main`: Gmail connect, M2 editing, events and grades.
The remaining MVP lane is email forwarding to Knowlu (item 2a below).

**Proven 2026-10-02 (live proofs on a dev build of `main` `376e8ba`, scratch profile, proof account,
staging; facts from the run ledger; each row closed by Quinn's word 2026-10-02):**
- **Gmail connect** (T13): all seven checks passed (no Gmail step in onboarding, Connect, a slot that
  proposed 11 cards, approve, revoke at Google then Reconnect, two-step Disconnect with grants 0, queue
  and seen sets 0, cleanup). Two findings, both fixed in the proof-fixes PR: the
  consent poll stopped before a ~3 minute consent finished (`49debd0`, `b167e73`), and the deck card
  did not show its Gmail attribution, D15 (`0aad6d5`).
- **M2 editing** (T10): all six checks passed (body edit and Undo, conflict refusal with the draft kept,
  preferences, interests, sync carries no `profile/` records, Not shown 34).
- **Events** (T10): passed with recorded gaps. No obligation card appeared in the feed, no carried
  series date existed (every date was card-listed), and no accepted all-day date fell on the proof day;
  the T4, T4c, T4d and T7 tests stand for those. Findings: failed model calls were charged to the
  first-day event allowance (event cap 80 a day, doubled to 160 on an account's first two judging days;
  fixed in the proof-fixes PR, `65c5e43`), and the wizard showed the "Blackboard or Canvas?" chooser
  after a curated pick (F4, fixed in the proof-fixes PR, `f680445`). Root cause of F4: the reviewer was
  right about the mechanism, but the chooser was stale and F4 does fix what the proof saw. On `main`
  `376e8ba` the chooser becomes visible only when an `open_lms_window` call fails (`console.js:3111`, in
  the `!r.ok` branch of the `#wiz-lms-open` handler, call at `:3106`). It ships `hidden`
  (`index.html:148`), `.wiz-row[hidden]` keeps CSS from leaking it (`console.css:445`, `:450`),
  `renderWizard` and `pickSchool` (`:3374-3387`) never touch it, and `lms_link.rs:230-231` answers every
  `ok:false` with a non-empty error. So a curated pick could not show it; what was seen was a chooser
  left over from an earlier failed call that the pick did not hide, and F4 now knows the curated
  campus's LMS in the wizard.
- **Grades** (A13, through the suspended gate): connect, capture, refresh (2 courses, 12 changed items),
  silent refresh and forget passed. Not run: an uncurated school's "not available" check (it needs a
  second onboarding). A scheduled slot's capture was not observed (run-slot-once cannot reach the kept
  session), and two courses answered 403.

**Left, as written 2026-09-29** (each merge or deploy on Quinn's word at the time, asked in one line with the
evidence). **Status 2026-10-18 (from `git log origin/main`): items 1 and 2 are MERGED** (#21 `97dc27b`,
#23 `6bea8af`); item 3's staging gate passed 2026-09-30 (§1, §2). Nothing of Integrate is left except §4's
refund-migration check, which no ledger row records as done (below); the entries stay as the record.
1. **The Credential Manager flake — MERGED #21 (`97dc27b`)** (worktree `fix-credman-flake`). `app/src/account.rs`'s
   `PENDING_TARGET` (`knowlu/pending/session`) is machine-global, so two processes running the app's
   tests collide on it. The fix belongs in the test helpers or a per-process target override; a change
   to `account.rs` is contract-list work (contract-engineer, then contract-reviewer). Until it merges,
   only one lane runs `cargo test -p knowlu` at a time.
2. **Ruling 11's human token — MERGED #23 (`6bea8af`)** (worktree `human-actor`). Every vault the app creates writes the human
   actor `student` (`journal::HUMAN_ACTOR`), recorded once in `config/actor.yaml`; `quinn` reads as the
   same human actor forever; a test fails on the bare literal `"quinn"` in non-test code outside the two
   constants. Existing vaults and every frozen reference stay byte-identical. Contract-list work:
   contract-engineer at xhigh, reviewed by contract-reviewer. It is also one of the Pilot's gates.
3. **Staging matches the repo — gate passed 2026-09-30.** The controller runs the staging runbook (cloud-engineer's, in this
   wave) and the smokes by OTP session. Its order comes from `docs/notes/2026-09-22-stream-j-rollout.md`,
   read first: `20260922120200` before `gmail-read` is deployed; `20260922120100` and `120200`, then
   `judge-task`, `judge-event`, `judge-email` and `gmail-read` redeployed together, straight away; the
   telemetry handler (#12's F7) before any engine that carries F8; then `20260926000100`. One
   `db push --include-all`, because J's migrations sort before `20260923000100`. Production waits for
   the Pilot (§4).

This rewrite of §3–§4 is Integrate's last docs item; docs-keeper then updates §2's lane table.

### MVP: next

**For the student:** everything the founder's quinn-ops did, plus grades from Blackboard (VISION,
"Build order"; Amendment 2026-09-29, item 9). Since session 5 (2026-10-18) the MVP is the app **given to friends to test for free**: their
non-founder accounts get a comp entitlement (L8), on prod parity (L9) and an unsigned CI release that
installs and updates (L10), so L8-L10 are MVP work, not Pilot work. Quinn's 2026-10-18 ruling (session 5, §1.2, the MVP scope extras) also moved four items out of ruling 10's Pilot list into the MVP: runtime removal (L6), the 402 and "source went quiet" issues (L7), the re-consent screen (L4) and production parity (L9). **Signed in the cloud design's Amendment 2026-10-18** (by Quinn's delegated go, 2026-10-20, session-5 ledger), which also records the UA registrar (L5a) as built in the MVP and leaves open whether the MVP's exit gains the friends' finish line. **The syllabus read (L5) is a wave-1 MVP lane**, built now on `main` but not an exit condition of the friends' release (syllabus spec Q19, signed by Quinn 2026-10-19); cloud ruling 5 and `VISION.md`'s Syllabi, onboarding, Beyond and open-decision lines carry the spec's §0 amendment (L5 T0, applied by Quinn's delegated go, 2026-10-20). The lane table in §2 is current; this
list is the 2026-09-30 record, and §3's Pilot list and §4 below keep those four items only as history. Through 2026-10-02 it was proven only on a founder-owned scratch profile
and test account on staging. It holds the commitment model's phases 1–2, merged in Integrate (their
phase-1 follow-ups, the split card and hand-set `meets`, go in an MVP schedule lane), and these lanes,
in this order (gap ids from the vision program note; **status 2026-10-18: M1 #25, Gmail #26, M2 #27 and Events #31 are merged and the email branch is integrated, so "Next: main merged into the branch" below is done; do not re-run M1's merge steps**):

1. **M1 grades** (P2; branch `m1-grades`; spec `docs/specs/2026-09-29-grades-design.md` and its plan,
   on that branch). Task 1 is built and reviewed (its fixes at `29f1542`). Next: `main` merged into the
   branch (textual conflicts in `ids.rs`, `backup.rs`, `sync_contract.rs` and `sync_rows*.ts`, plus two
   semantic updates no conflict shows: `migrations_sync_test.ts`'s newest-migration assertion and the
   backup-folder count test), the grades spec's signed edits 3–5, Tasks 2–8, then Task 9's whole-branch
   review at xhigh. **Merge** on that review, green CI and Quinn's word; then `20260929000100` goes to
   staging (applied 2026-09-30). Ruling 12's gate is suspended until 20 paying users (A13). The one predicate in
   `app/src/grades.rs` still gates Connect, refresh and the slot, and while `POLICY_READ_GATE` is
   `Suspended` it admits every curated Blackboard row, dated or not. The date-and-bump test still keeps
   any date off `main` until privacy bump #1; M1 drafts its privacy sentences in the grades spec's §11, never in
   `site/privacy.html`, and never moves `PRIVACY_VERSION`. Grades keep `ids::derived_id`
   (path-deterministic, so every desktop derives the same id); two-desktop may move them onto its
   `import_id` when it resumes. **The founder's live proof** follows the merge, with Quinn at the
   machine in the Blackboard sign-in window (the proof harness cannot drive it), on a dev build of
   `main` that waits on no policy read (A13: the gate is suspended); then the scratch profile
   is cleaned up. **What privacy bump #1 owes from M1** (the branch touches none of it): spec §11's two
   sentences on `site/privacy.html`, its Effective date, `PRIVACY_VERSION`, the sentence pins,
   `PRIVACY_BUMP_1` in `app/tests/grades.rs` set to that version, and the lawyer-packet delta; the
   Pilot's re-consent screen (R-PS-4) is still owed before a second account. No `policy_read` date is
   recorded while A13's suspension holds. `20260929000100` is applied to staging before any
   release that writes `grades/`.
2. **Gmail connect in the app** (P3, taken out of C4): the settings row's Connect and Disconnect (H9
   phase (b); `set_google_calendar` does not exist yet, so the command and its row are written
   together) and the Gmail step's incremental-consent button and copy. Research first (sized M; it
   checks what C2's `google-connect` and `gmail-read` and the wizard's phase (a) already cover) → spec
   → Quinn signs → plan → build → merge. Gmail stays in Google's Testing mode until `gmail.readonly`
   verification (Launch): 100 test users, refresh tokens that die after seven days.
   **MERGED 2026-09-30 (#26, `199cd1f`); T12 staging deploy done, T13 the founder's live proof is Quinn's.** Earlier state (2026-09-30):
   spec signed 2026-09-29 (`docs/specs/2026-09-29-gmail-connect-design.md`) and
   plan (`docs/plans/2026-09-29-gmail-connect-plan.md`); built on branch `gmail-connect`, not merged:
   the cloud lane (`gmail-read`, `google-connect`, migration `20260929000200_gmail_disconnect_purge.sql`),
   the app commands `google_status`, `google_connect` and `google_disconnect` (console list 54 on `main`,
   73 distinct), the engine's D4 gate and D7 task-to-card routing,
   and the page (Settings row, two-step Disconnect, wizard Connect Gmail button). Waiting: the transport
   stop (T8, on PQ1's answer), the staging deploy (the controller's), the founder's live proof and the
   whole-branch review; merge needs Quinn's word and the code push is asked. No release is cut from
   this branch before privacy bump #1 (the Pilot's release gate).
2a. **Email: email forwarding to Knowlu** (spec `docs/specs/2026-09-30-email-forwarding-design.md`,
   signed 2026-09-30, amendments A1-A13; plan `docs/plans/2026-09-30-email-forwarding-plan.md`). Built
   in its own worktrees (`email-engine`, `email-app`, `email-cloud`; build started 2026-10-01); the staging
   receiver is in SES us-east-2 and the S0 DMARC gate passed 2026-10-01; no release is cut from a `main` carrying it before privacy bump #1 (D18; `scripts/ci/privacy-guard.ps1` fails such a release, 2026-10-04).
   **2026-10-03:** the lane is integrated at `156ee3e` (T17; its ledger row holds the gate counts) and its
   reference docs are written (`docs/reference/app.md`'s Email bullet, `engine-commands.md`'s `mail-applied`
   and mail-pass bullets, `cloud/supabase/README.md`'s mail section and AWS runbook). PQ10 is
   ruled (2026-10-04: Blackboard rows built; the model path until the pilot for Canvas, zyBooks and
   due-date-changed; pilot task T3a.3 writes their rows from pilot students' real mail) and Checkpoint B is
   closed at `0aea395`. **Resolved 2026-10-04:** PQ13 (b), due-aware, built (`79c1742`, `c99785f`), so
   Checkpoint D is closed on it; PQ14, an unanswered `mail-resolve` claim held 3 slots then given up with
   the unmatched notice, built (`2adf0c9`); D18, the release-time check, built (`5470cd2`); Q2/G2 signed
   (T15.0's `surface` fields stay); G3 (the pace constants stay provisional, S6 runs in the pilot); G4
   (the deletion-refused copy stays as written). Still open: PQ10's Canvas and zyBooks templates, pending
   Quinn's forwarded sample emails (T3a.3); S6 (`MAX_PER_MINUTE` 10 and `MAX_PER_DAY` 600 in
   `app/src/mail_steps.rs` stay provisional for Microsoft 365, and T18c keeps a school backfill under 600
   until it runs; Gmail's day is Quinn's 2026-10-19 ruling: 1,500 originals in at most 300 messages over a
   rolling 24 hours, and 600 page loads, see the Gmail backfill note below);
   PQ3 (the digest, T16). T14 (UA's mail fields) is not built. T18
   needs Quinn: the sign-ins at the machine (S1-S3) and the merge word (the staging secrets were set
   2026-10-05, T18a).
   **Gmail backfill, as built 2026-10-19 (Quinn's rulings; plan T11.2 "Gmail's caps, as built"):** a day is
   at most 1,500 originals and 600 page loads per UTC day and 300 Gmail messages over a rolling 24 hours,
   kept by instant in `mail.json` `gmail_messages`; a backfill the 300 hold waits until 30 have freed
   (`efb5926`). Microsoft 365 stays at 600 a day. `lone_stall` is a backfill's stall record and
   `recover_stall` a slot's (`a83a2a8`). A lapsed mailbox is reported `covered` by the app after a whole
   pass (`7c917b3`, merged at `1c15e8d`). A Gmail forward whose recipient did not draw is retried once, then
   stops with "the forward's address was not accepted" (`7f33a3a`, `ae3d428`, `32d41de`). **Accepted for the
   MVP:** the backfill's `Routine` is seeded once (`app/src/mail/backfill.rs:484` on the committed tree), so
   slot sends during a 30-free wait can push the rolling count slightly past 300. **Open follow-up (after the
   MVP, Quinn):** one shared Message-ID key between `gmail-read` and the forward path, which closes the
   window where a lapse during setup files a gap twice (spec §4.11 step 3).
3. **M2, body and profile editing** (P5): a body write through `write`, the drawer's body editor, and
   profile editing (interests, preferences). Sized M/L, not small: the engine can only append a body
   line today, so replacing a body is new `write` and journal semantics, and a journal record that
   syncs is a new privacy class. The spec decides the primitive and what the record holds, and
   privacy-reviews it; contract-engineer and contract-reviewer build the primitive at xhigh; the drawer
   editor goes to console-ui. If time is short, Gmail connect goes first.
4. **Events** (P4): required events become tasks; events can be accepted and declined. Spec first
   (sized L); it builds on #12's `unsure` card and the `j-events` branch (the closed T3 experiment, kept
   as this lane's harness).
5. **P6, the small parity items:** a producer for Good to know items, conflict flags, opportunity
   proposals expiring after 14 days, the dropped-event audit list. Sized as a list; Quinn marks each do
   or cut; small "do" items go to `mechanical`.
6. **UA's university-policy read leaves the MVP** (ruling 12, suspended until 20 paying users:
   Amendment 2026-09-30, A13). The grades proof waits on no read. A13's review trigger is the 20th
   paying account (§4).

**The 2026-09-30 session's cut line, superseded by §2's session-5 lanes** (focus report §5): M1 merged is a must; Gmail connect and M2 are should
(Gmail first if time is short); the events spec (signed, not built), P6's sized list and the rewritten
production-parity runbook (§4, run only if Quinn sits for the secrets) are could.
### Pilot

**For the student:** a few undergraduates other than the founder, on the released app, one computer
each, needing nothing from anyone.

**Entry, Quinn's:** the number of students and their universities, the length, and a measure drawn from
VISION's successes 1, 2, 5 and 6. **Whether pilot students pay is ruled: they do not** (about 30 students;
Amendment 2026-10-16b, signed 2026-10-16, `docs/specs/2026-09-09-knowlu-cloud-design.md`). That amendment
also gates any outside paid sign-up (the quiet release from spring 2027): the lawyer reads the terms as
well as the privacy page, Kentucky sales-tax registration, Stripe live, a signed installer (Azure) and
`site/terms.html` carrying the 2026-10-16 prices; none of that is scheduled until Quinn sets a quiet-release date. **The measure and size are
ruled (Quinn, 2026-10-15; `docs/notes/2026-09-29-funding-and-seed-grant.md` §3c):** recruit 30, about
20-24 expected to finish; success is at least 70% of the students still in the pilot in its last month
using Knowlu 4+ days a week (the earlier 7 of 10 form stands), supporting: every student reaches a correct
"what's next" in their first session with no help, and none misses a deadline Knowlu had the information
to catch. Recruiting starts November 2026, with CAS's help. The length is still Quinn's at entry.

It holds (items marked "MVP since 2026-10-18" were moved to the MVP by Quinn's session-5 ruling, §1.2, and are
built there, not here; they stay listed as ruling 10's original list):
- **Production parity** (MVP since 2026-10-18, L9): §4's checklist, in order, less the two-desktop proof (now Launch's).
- **v0.1.1, built by CI** on Quinn's word, with the updater proof from 0.1.0 (C0 Task 5's Step 4).
  C0 Task 6's findings (recorded 2026-09-22) stand: the one `%LOCALAPPDATA%\Knowlu\` folder; the
  uninstall key by product name; the timing-sensitive vault-lock test; COM initialisation before
  `ShellExecuteW`; the app build blanking `target\<profile>\knowlu-engine.exe`.
- **Privacy bump #1, with one lawyer read** (split 2026-10-18: the bump itself is MVP since 2026-10-18, L4's interim bump with no lawyer read, and it is the bump #1 the release guard below, `PRIVACY_BUMP_1` and the specs name; the lawyer read stays here, and its version re-prompts everyone before the first Pilot student): the page names ruling 12's kept Blackboard session and
  the grades the account holds; its text, its Effective date and `PRIVACY_VERSION` move in one PR
  (`app/src/account.rs` requires it), L4's, no later than the first release from a main carrying the
  suspended grades gate, the Gmail row or email forwarding (the release guard below). The page text
  also covers spec §7's "kept sign-ins at every school, for now" paragraph, one of the email spec's
  seven §7 drafts that §2's L4 row names (T13a never listed them here). **The re-consent screen** (MVP since 2026-10-18, L4) for a `privacy_version` change (R-PS-4) ships with it.
- **The UA registrar** (the commitment model's phase 3; draft PR #19, `p3-registrar`; **MVP since 2026-10-18, lane L5a**, merging before L5): its spec
  (`docs/specs/2026-09-26-commitment-model-phase3-design.md`, on the branch) revised to apply-and-list
  (ruling 3's consequence, tested by `journal::human_edited`). Done 2026-10-18: the R0 spike, the R24 and G2-G5 rulings. Left: the A1 findings and the
  rest of tasks B1-B9 (L5a's row, §2), with B9's live proof at the machine. The registrar never shares the kept
  Blackboard profile.
- ~~**The local model runtime removed**~~ (MVP since 2026-10-18, L6): **done in lane L6** (branch `l6-runtime-removal`; PR number,
  merge sha and date are written here by the controller at the merge, T8). `engine/src/runtime.rs`, `app/src/inference.rs`,
  `SUPPORTED_RUNTIMES`, the settings row and the wizard's offer are gone; `judge` has one model, `CloudModel`, and no account
  is `judge (skipped: no account)`. `engine/tests/no_local_runtime.rs` fails if any of it returns. The ignored-test count is
  three. CLAUDE.md's four sentences stay for Quinn's edit at the merge checkpoint (the lane plan's §10).
- **Two issues shown in the app** (MVP since 2026-10-18, L7; built on branch `l7-named-issues`, cloud, engine and read
  model done, PR pending at T8; plan `docs/plans/2026-10-19-l7-named-issues-plan.md`): OpenRouter's 402 (payment required)
  as a named issue (`unavailable: "out of credit"` on the judge reply; `judge-service:out-of-credit` heads-up) instead of
  quiet model failures, and "source went quiet" (`google_accounts.status = 'quiet'` after 7 days (Quinn's Q3, 2026-10-19);
  `/gmail-read` answers `went_quiet`; `gmail-source:quiet` heads-up). Both reach the app as `issues_panel.sources` in the read
  model (`docs/surface/anatomy.md` §4.1); the console page that renders them (T5) is still to land.
- **The sender on a Gmail card** (Gmail connect spec, Q7 (c), D15): the MVP card shows its Gmail
  attribution and the model's reason, no sender; storing and showing the sender's display name lands
  with the lawyer-read version of bump #1 (a Pilot item, before any pilot student connects Gmail, D15;
  L4's interim bump does not carry it), whose page then adds "and who the message was from" (spec §6 item 5).
- **`calendar.readonly` verification submitted** (a sensitive scope: lighter review, no CASA).
- **The university-policy read for each other pilot student's university: SUSPENDED until 20 paying
  users** (Amendment 2026-09-30, A13). While it holds, the kept Blackboard session is offered at every
  curated Blackboard school; the review at the 20th paying account decides.

**Gate (reworded 2026-10-18, session 5; signed in the cloud design's Amendment 2026-10-18 by Quinn's delegated go, 2026-10-20):** no non-founder account exists before the re-consent screen,
the interim privacy bump and ruling 11's token have shipped; the token is done in Integrate. The
**lawyer-read version** of bump #1 gates the *Pilot*, not the friends' release: it re-prompts every
tester before the first Pilot (non-friend) student. Friends are non-founder accounts, so the
chat-exposed secrets (the prod database password, the Resend key, and the OpenRouter key that sits in a chat transcript) are rolled before them (§4's key-roll row). The
condition that no pilot student uses the kept Blackboard session before their own university's read
is recorded is suspended until 20 paying users (A13).
**Release gate:** no release is cut from a `main` carrying the Gmail settings row, email forwarding to
Knowlu, or the suspended grades gate (`POLICY_READ_GATE = Suspended`, which offers the kept Blackboard
session at every curated Blackboard school) until privacy bump #1 merges, which before the Pilot means
L4's interim bump carrying every sentence §2's L4 row lists (Gmail connect spec, Q8,
signed 2026-09-29; email forwarding spec D18 and D25, signed 2026-09-30). The email forwarding half is mechanical (D18, ruled 2026-10-04): `release.yml`'s first build step, `scripts/ci/privacy-guard.ps1`, fails the release while the mail code is in the tree and `PRIVACY_VERSION` is on or before `2026-09-24` (`engine/tests/workflows.rs` runs it); the Gmail row and the grades suspension are still held by hand. The guard reads only `PRIVACY_VERSION`, never the page, so it passes L4's bump whatever the text says: the sentences are held by L4's pins and page review, and checked by hand before L10's tag. **Ruled 2026-10-18 (session 5, lane L4):** friends get an honest interim bump now (the page says what the app does today, `PRIVACY_VERSION` moves so testers accept it, the re-consent screen is built), and the lawyer-read version re-prompts everyone before the Pilot; the lawyer read is no longer a gate for the friends' release.

**Open question for Quinn, at Pilot entry:** ruling 10 names the Pilot's contents, entry and gate but
no exit. This section reads the exit as "the measure set at entry is met, and Quinn says so" (the measure
is ruled, above; the exit wording is still unsigned).
Launch's exit is likewise unnamed; §3 reads it as "everything under Launch is done, and Quinn says
so". Quinn rules on both exits at Pilot entry.

### Launch

**For the student:** any US undergraduate can pay $9.99 and use Knowlu on more than one desktop.

It holds:
- **C5, the relay fetch** (`docs/specs/2026-09-17-c5-relay-fetch-design.md`; its plan, 12 tasks,
  reviewed through three fix rounds, `docs/reports/2026-09-17-c5-relay-fetch-plan-review.md`). Branch
  `c5-relay` is empty. The pre-flight (36 amendments, taken at `86a1482`) is re-run first; the
  two-desktop session's G1–G4 and C5's consent question 15(d) come due here.
- **Two-desktop, Plans 1–3 and the two-computer live proof** (`docs/specs/2026-09-25-two-desktop-design.md`;
  draft PR #20, frozen at `3e6c3f5`): deterministic ids for imported notes and the device fetch-turn
  lease (Quinn's "both" of 2026-09-24). Plan 1's Tasks 1–6 are built; Task 7 re-runs from its brief;
  the branch needs a rebase, and its whole-branch review re-reads Tasks 1–4 at xhigh. Until it ships,
  Knowlu is one computer per student. Any migration that redefines `sync_notes_path_check` is a new
  file stamped after every existing one and restates the full union, so Plan 2's `000200` re-stamps.
- **Privacy bump #2.**
- **`gmail.readonly` verification and the annual CASA**, or Quinn accepts the 100-user cap.
- **Lawyer sign-off** (unless it moved to Pilot entry); each university-policy read and the widening checklist that decides how
  reads cover every university Launch opens to (suspended until 20 paying users: Amendment
  2026-09-30, A13); tax registration; code signing (Azure, §4); the
  private-repository decision (ruling 13; the laptop runner comes back only once it is private).
- **From C4:** the telemetry class-(c) toggle UI (the eval seed stays empty until it ships), and a
  custom functions domain or a redirect to a page on `knowlu.com` for the Google callback's pages
  (m148).
### Beyond

Each VISION phase that makes the morning answer better, in the vision program note's order: the main
page (domain strip, free time, all clear, today's schedule); GPA and exam prep (the syllabus read
left Beyond for the MVP's wave-1 lane L5: cloud ruling 5 as amended 2026-10-19, see §3's MVP); quick capture; nudges and lock-in;
the assistant; the Outlook calendar and more homework platforms; the dedicated Knowlu calendar in Google and
Outlook. Also here: stream J's T0, T5 and T7, and the `origin` split for Calendar-API rows (so
ICS-derived events can rejoin the training export). J's T6, the calibration harness, stays parked until
real corrections exist. Per feature: a signed spec, a plan, a review, a merge and a live proof.

### History: the C-series, closed at C3′

§1 holds the record of each; the reports are in `docs/reports/`.
- **C0, the CI release:** PR #3 (`1b7bbea`), the unsigned switch #7 (`dcd26aa`), the release speed-up
  #8 (`274e137`); **v0.1.0 published 2026-09-17**, unsigned. Azure signing is Launch's; 0.1.1 is the
  Pilot's.
- **C1, accounts:** PR #2 (`82dcc04`). **C1b, sign-in:** #9 (`7477ec5`). **C1c, the first day:** #13
  (`f30da69`).
- **C2, the judgment service:** #4 (`666d458`), the provider swap #5 (`e58b2c8`), the Google reconnect
  fixes #6 (`0b6710f`). Production has none of it yet (§4).
- **C3, sync:** paused at Task 3 on 2026-09-17 and superseded by the Amendment 2026-09-17.
  **C3′, the account vault:** #15 (`e2ce40c`), the last C-phase.
- **C4** is no longer a phase: ruling 10 spread its rows over the stages above (Gmail connect to the
  MVP; runtime removal, re-consent, 402 and "source went quiet" to the Pilot, and on 2026-10-18 (session 5, §1.2) on to the MVP as L6, L4 and L7; the (c) toggle, the
  widening checklist and the functions domain to Launch; the Calendar-API origin split to Beyond).
  **C5**, the relay fetch, is Launch's. The instruction of 2026-09-26, "reach the beginning of C4", is
  superseded.
- **Jev System One:** closed for the judgment service (§1's C0 block).
- The sequence that stood here until 2026-09-29 (the numbered C-order and the 2026-09-25 resume point)
  is in git at `86a3431`.

## 4. Quinn's queue, by stage — one at a time, when reached, with the context

Open items first, in §3's stage order; the second table is the record of what is done or decided,
kept as it stood. Nothing is pre-authorised: a push that carries code, a merge, a deploy, a secret or
a spend is asked at the time.

### Open

| Stage | Ask | Context to give them |
|---|---|---|
| Integrate | ~~**Merge the flake-fix PR, then the token PR**~~ **Done: #21 and #23 merged** | Asked one at a time at merge time, each in one line with the evidence (the contract-reviewer's verdict, green CI). §3's Integrate items 1–2. |
| Integrate | **Check, then if missing push, migration `20261002000100_refund_failed_call.sql` (it defines `refund_call`) to staging. The ledger records only `db push 20261002000200_mail.sql` (2026-10-05) and no row for this one: run staging's `migration list` first. If it is missing, push it (`db push --include-all`). L3 redeploys none of `judge-task`, `judge-event`, `judge-email` and `gmail-read` until it shows applied. Then to prod on Quinn's word** | Refunds fail silently without it: the functions call `refund_call`, and a missing function is not an error the student sees. The controller runs it; production waits for Quinn. |
| Integrate | ~~**Go for the staging deploy**~~ **Done: the staging gate passed 2026-09-30 (§3's Integrate item 3), and email's T18a staging deploy went out 2026-10-05** | Not to be asked again. Nothing touches production without Quinn's word (the L9 rows). |
| MVP | ~~**Sign the Gmail-connect and M2 specs** (one sitting), later the events spec~~ **Done: Gmail #26, M2 #27, events #31 merged** | Each after its research and spec review. W1's research left 13 questions for these specs; they are asked inside the specs, not separately. |
| MVP | ~~**Merge M1**~~ (merged #25), then **the founder's grades proof** (passed 2026-10-02 through the suspended gate; the score fix is session 5's L2) at the machine | Merge on Task 9's whole-branch review and green CI; `20260929000100` then goes to staging. Bump #1 owes the privacy page, its date, `PRIVACY_VERSION` and `PRIVACY_BUMP_1`; The proof runs in the Blackboard sign-in window on a dev build of `main` and waits on no policy read (A13; §3's MVP lane 1). |
| Review | **A13's review at the 20th paying account** | Ruling 12's policy-read gate is suspended until then (A13); the controller checks a count-only query at each milestone and asks. "Record UA's university-policy read" has left the MVP queue. |
| MVP | **Email forwarding: what is left of T18** | Checkpoint D is closed (2026-10-04) and the staging secrets are set: the email ledger's row of 2026-10-05 (T18a) records that Quinn created `knowlu-mail-runtime`'s key and set `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` from their shell, so **do not ask for either again and do not make a second key** (a second or rotated key mid-proof would break the running one). Still Quinn's, one at a time: (1) PQ3, whether to build the digest (T16); (2) PQ10's Canvas and zyBooks templates, pending Quinn's forwarded sample emails (PQ13, PQ14, D18 and Q2 resolved 2026-10-04); (3) S6 (M365 test tenant; pace constants provisional); (4) T18b and the sign-ins at the machine (S1-S3, G1-G3); (5) T18c's cleanup; (6) the merge word. The controller generates `MAIL_ADDRESS_KEY` and `MAIL_ENC_KEY` blind. The runbook is `cloud/supabase/README.md`'s mail section. |
| MVP | **Mark each P6 item do or cut** | The four items are in §3's MVP lane 5, sized as a list first. |
| MVP | **Roll staging's `OPENROUTER_API_KEY`** | Staging's secret holds Quinn's temporary key (set 2026-10-02, after the old key of 09-16 proved dead and every event judgment failed). When that key is rolled, update staging again, and the GitHub secret the eval gate reads too if it is still the old key. |
| MVP | **Say "MVP reached"** | Once the parity audit re-run shows no open row and the grades proof has passed. |
| Pilot entry | **The pilot's shape** | Ruling 10: the number of students and their universities, the length and a measure from VISION's successes 1, 2, 5 and 6. Whether pilot students pay is ruled (no; Amendment 2026-10-16b), which also gates any outside paid sign-up (lawyer, tax, signed installer: §3's Pilot). **The measure and size are ruled (2026-10-15: 30 recruited, at least 70% of those still in using it 4+ days a week in the last month; §3's Pilot).** Still open: the length, and the Pilot's and Launch's exits, which ruling 10 leaves unnamed (§3). |
| MVP (session 5, open) | ~~**Confirm the page-publish order**~~ **Confirmed by Quinn's delegated go, 2026-10-20 (session-5 ledger, the RULING row of 19:35Z: "whatever you need my permission for, go for it")**; still open: **which Resend key is live** | The controller's ruling stands: L10's first tag publishes the interim privacy page, between L9's steps (2) and (4), because `release.yml` is the only deploy path for `site/`. Cost if wrong: that tag ships before prod holds its secrets, so a friend who installs it early sees a sign-in that cannot judge. The alternative, a site-only deploy workflow, is not taken. **Still Quinn's, as a question of fact that the go cannot answer:** which Resend key is live, the 09-14 one or the 10-0x one (the key-roll row below rolls whichever is live and deletes both). The OpenRouter roll itself is the key-roll row's. |
| ~~MVP (session 5)~~ done 2026-10-20 | ~~**Sign Amendment 2026-10-18 to the cloud design**~~ **Done: signed by Quinn's delegated go, 2026-10-20 (session-5 ledger)**, as the cloud design's Amendment 2026-10-18 (after Amendment 2026-10-19) | It records Quinn's 2026-10-18 rulings, each quoted from its dated ledger row (the email ledger's MVP-planning rows; the session-5 ledger's #19 row): ruling 10's *Gate* reworded (§3), and production parity (L9), the interim bump #1 with re-consent (L4), runtime removal (L6), the 402 and quiet issues (L7) and the UA registrar (L5a) built in the MVP, plus L8 and L10. Bump #1 is Amendment 2026-10-19. `CLAUDE.md`'s matching lines were L6's (PR #50); no other line follows. **Left open there for Quinn:** whether the MVP's exit gains the friends' finish line; whether L10's releases satisfy the Pilot's v0.1.1 item. The `VISION.md` wording for the friends' MVP (its Build order and pilot paragraph) was ruled 2026-10-20 as the recommended option under Quinn's delegated go: the amendment quotes the replacement text, and `VISION.md` carries it with its marker. |
| ~~MVP (session 5)~~ done 2026-10-20 | ~~**Sign the C3 and L8 specs**~~ **Done** | The M365 redesign (`docs/specs/2026-10-10-m365-backfill-redesign.md`) is signed by Quinn's delegated go, 2026-10-20; its status line now carries the header Quinn ruled in C3 Q6 (2026-10-19): "decisions ruled 2026-10-11/13/18; live gate closed 2026-10-18". That was C3 plan R12's header step, so R12 has nothing left to do there. The friends-testers spec was already signed by Quinn's own word, 2026-10-19 (session-5 ledger: "signs the L3 … and L8 … spec headers"), and its Amendment 2026-10-20 by Quinn's Q1, Q7 and §8.3 rulings. Its header is unchanged. |
| Before friends' accounts (ruled 2026-10-18, session 5 §1.5/§2: "roll it before friends"; was "Pilot, before parity") | **Roll the prod database password, the Resend key and the OpenRouter key.** The database password and the 09-14 Resend key were pasted in chat on 2026-09-14; the Resend key used for the W5/W7 sends was pasted on 10-0x (roll whichever is live, and delete both); the OpenRouter key is the temporary one in a chat transcript (also staging's, next row). Friends are the first non-founder accounts, so the roll comes before L8's comp accounts and L10's tag, and **before L9's step (4) sets prod's `OPENROUTER_API_KEY`: set it to the new key, never the exposed one** | Database: reset at `https://supabase.com/dashboard/project/jxthohvwrijwtuwlglan/settings/database`; nothing else references it. Resend: create the new key, then on **both** projects `supabase secrets set EMAIL_API_KEY=… SMTP_PASSWORD=… --project-ref <ref>` and `$env:SMTP_PASSWORD=…; supabase config push --workdir cloud` while linked to each (`link --project-ref <ref> --password ""`), **then** delete the old key — the other order breaks sign-in mail. `BILLING_JOBS_TOKEN` and `SOURCES_ENC_KEY` on prod were generated on this machine and never shown; they need no roll. |
| MVP (L9; was Pilot) | **Production parity** (rewritten 2026-09-29): production holds C1's seven migrations and eight functions of 2026-09-14, live billing, and nothing later that this file records | **The checklist, in order.** The controller runs it with Quinn's secrets, from a scratch copy of `cloud/supabase` linked to `jxthohvwrijwtuwlglan`; Integrate's staging runbook is its rehearsal.<br>**(1) Read prod's `migration list`, then one ordered push** (`db push --include-all`) of everything missing, in filename order: C2's `20260911000100`–`000900`; C3′'s `20260912000100`–`000400`; the provider swap's `20260916000100`; `20260917000100` (C1b) and `20260922000100`/`000200`, if the list lacks them (the checklist of 2026-09-24 did not name them, so they may already be there); J's `20260922120100` and `120200`; C1c's `20260923000100`; the commitment model's `20260926000100`; M1's `20260929000100` (M1 merged); the Gmail connect stream's `20260929000200_gmail_disconnect_purge.sql` (it sorts after M1's), which must reach production before the first release that carries the Gmail settings row, or a production Disconnect keeps `gmail_queue` and `gmail_seen` rows while bump #1's page says they are deleted; `20261002000100_refund_failed_call.sql` (PR #33, already on main, so it is not held for email: `judge-task`, `judge-event` and `judge-email` call `refund_call`, so it goes in before any of their redeploys); then email's three, in filename order: `20261002000200_mail.sql`, `20261005000100_mail_claim_max_tokens.sql` (the `mail_claim` max_tokens 640 to 1024 re-pin) and `20261020000100_mail_max_tokens_repin.sql` (T18b's re-pin: `mail_resolve` 640 to 1280, `mail_claim` 1024 to 1536; `docs/reports/2026-10-20-email-forwarding-pin-check.md`; confirm prod's two `models` rows match staging's), only once the email lane has merged; then the later lanes' migrations as each merges, in stamp order (L3, L8's `<stamp>_testers.sql`, L5, L1), each followed by its function redeploys in (2).<br>**Checked against staging runbook §9** (2026-09-30): this list already names C1b's three (runbook rows 22-24), J's two (25-26), p1's one (28), M1's (29) and Gmail's (30); the runbook's rows 8-30 are only the migrations of that date, so the email migrations and every later lane's migration are past the runbook: read prod's `migration list` against filenames, not against "rows 8-30".<br>**(2) Functions** (`functions deploy … --use-api`), each only after the migrations it reads:<br>• C2's eleven, with `gmail-read` never before `20260922120200`, and `judge-task`, `judge-event`, `judge-email` and `gmail-read` together straight after J's two;<br>• C1b's changed ones;<br>• `account`, redeployed after the migrations;<br>• `gmail-read` and `google-connect`, redeployed after `20260929000200` (the Gmail connect stream changed both);<br>• `sync-push` and `sync-pull`;<br>• `ingest-coursework`;<br>• the telemetry handler (#12's F7) before any release whose engine carries F8.<br>• email forwarding's six (`mail`, `mail-inbound`, `mail-process`, `mail-canary`, `mail-pull`, `mail-resolve`), after `20261002000200`, with the three Vault entries (`mail_process_url`, `mail_canary_url`, `mail_jobs_token`) in place first so the cron does not raise.<br>• later lanes' redeploys, after their migrations: L3's judge functions, and L8's `billing-checkout` and entitlement (`GET /entitlement` must answer the comp grant) before any friend signs in on prod; without them a friend's engine refuses coursework, ingest, grades, judge and sync after the 72-hour grace and the overlay asks for $9.99, against ruling §1.4 (friends get the app free).<br>**(3) Privacy bump** live on `knowlu.com` (the only path that deploys `site/` is `release.yml`'s Pages publish on a `v*` tag, so the page goes live with L10's first friends tag: **that tag is cut after L4 has merged and steps (1)-(2) are done, and before step (4)'s Google secrets**; L10's install and updater proofs follow step (4). Confirm the page by fetching `knowlu.com/privacy` (the `.html` form answers 308 to `/privacy`, so fetch with redirects followed) and reading its date, and that it has the Google-connection entry and no longer says "Gmail is not connected in this version", since a version check alone passes a page without them) (for the friends' release, L4's interim bump, with no lawyer read: ruled 2026-10-18; the lawyer-read version of bump #1 replaces it before the Pilot and re-prompts everyone), before any release that uploads note text, before the first non-founder account **and before step (4) sets the Gmail project's `GOOGLE_CLIENT_ID`** (Gmail connect spec §6: the released 0.1.0 can connect Calendar the moment that secret exists, and the page must disclose the stored connection first); the re-consent screen ships with it.<br>**(4) Prod settings and secrets, which Quinn owns:** `OPENROUTER_API_KEY`; the Gmail project's `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` with the prod callback added, only after step (3); C1b's two auth settings (the next row); email forwarding's secrets by name (`cloud/supabase/README.md`) **and production's own receiving domain** (its own SES domain, rule set, bucket and topic, the topic's `SignatureVersion` set to 2 before the subscription; staging's `in.knowlu.com` receiver is not reused), after step (3).<br>**(5) Proofs:**<br>• the staging smokes repeated on prod;<br>• a staging proof that a slot and *Sync now* run **more than an hour** after the last session refresh and sync rather than say "signed out" (confirm prod's `jwt_expiry` is 3600).<br>The two-desktop live proof that stood here moved to Launch (ruling 10).<br>**(6)** Then the updater's second tag (v0.1.x+1, for L10's update proof), on Quinn's word; the first tag is step (3)'s. |
| Pilot | **C1b's open sign-in settings** | From the record's C1b row: (6) on **both** projects, Authentication → Providers → Email → *Confirm email* OFF and Authentication → Rate Limits → *Emails sent* 20 per hour (still open 2026-09-22: the API showed both unchanged; the project default of 2 an hour blocks a pilot); confirm the sign-in Google client is in the new project with publishing status In production; and the two decisions ruled by the controller pending Quinn's word — the card stays required at checkout (R-C1b-2), and whether to turn on `[auth.captcha]` for the sign-in endpoints. |
| Pilot | **Privacy bump #1: the lawyer read, with P5's eleven facts** | **Since 2026-10-18 the page sentences this row drafts (M1's, the interests sentence, the Gmail connect sentences) go on the page first in L4's interim bump, with their page tests (§2's L4 row); what stays in the Pilot is the lawyer's read of that page and of the terms with P5's facts, and the lawyer-read version re-prompts everyone.** The page names ruling 12's kept Blackboard session and the grades the account holds (M1's sentences, drafted in the grades spec's §11). `docs/reports/2026-09-10-c1-quinn-packets.md` (P5 section) lists what only Quinn can settle in the drafted `site/privacy.html` and `site/terms.html`: the legal entity's name, the `support@` and `security@` mailboxes (they forward, §1), Alabama as governing law, that the draft carries **no arbitration clause** (informal resolution then the courts — the lawyer's call), and the Stripe Dashboard settings the terms assume. Hand the two pages to the lawyer with that list. **The interests sentence** (M2 editing spec §5, Q8, signed 2026-09-29): the page's list of what travels to a model must also carry, or say in the same facts, *"…your stated preferences and grade weights, and, when a campus event is judged, the event's listing and up to the first 600 characters of the interests you have told Knowlu."* Interests reach the event judge for the first time with M2's editor and no code gates it; this sentence is the guard. **The Gmail connect sentences** (Gmail connect spec §6, Q8 (a1) and Q9 (a)(ii), signed 2026-09-29; drafted there, never written into the page by that stream, which leaves `site/privacy.html` and `PRIVACY_VERSION` untouched): (1) the parenthesis "(Gmail is not connected in this version of Knowlu; the app will say when it is.)" goes, and the disconnect sentence becomes "Disconnecting (in Settings, at any time) revokes our access at Google and deletes the stored connection, our list of which emails we have already read, and any proposals from your mail that had not reached your computer yet. The record of what Knowlu decided about each email, including its message id, stays until you delete your account. Google holds Calendar and Gmail as one permission, so disconnecting one disconnects both."; (2) "What we collect" gains *Your Google connection, if you make one*: the permission Google gave us (stored encrypted, never sent to your computer), the Google account's id and email address (so the app can show which account is connected), which of Calendar and Gmail you allowed, when you connected, and whether the connection still works; deleted when you disconnect or delete your account; (4) "How long we keep it" gains a Gmail bullet: our list of the emails we have read for 30 days, a proposal waiting to reach your computer until it does and then 7 days, both deleted at once by a disconnect or an account deletion, and the record of each decision, message id included, kept until you delete your account (the PR re-reads the 30 and 7 days in `20260911000900_final_review_fixes.sql` before quoting them). (Item 3 needs no page edit: the engine now proposes every Gmail item, so the page's existing sentence is true.) **The four page tests** (spec §8.5): (a) the page no longer contains "Gmail is not connected in this version"; (b) the disconnect sentence names Calendar and what the migration deletes, says the record of each decision stays until the account is deleted, and no sentence says a disconnect deletes message ids; (c) "What we collect" has the Google-connection entry and "How long we keep it" has the Gmail bullet; (d) the page's date equals `account::PRIVACY_VERSION` (already pinned by `app/tests/static_assets.rs`, which stays green). |
| Pilot | **Submit `calendar.readonly` verification** | The sensitive scope: lighter review, no CASA. The homepage and privacy URLs are live. The P4 texts are in `docs/reports/2026-09-10-c1-quinn-packets.md`. |
| MVP (L5a) | ~~**The registrar's R0 spike, at the machine**~~ **Done 2026-10-18** (with Quinn signed in); the live proof that remains is B9 | B9 needs Quinn's own university sign-in at the machine, on a dev build of `main` after #19's merge; it decides whether the friends release carries the registrar button (ruling 8). |
| MVP (L5a) | **Two registrar choices not yet ruled, recommendation first** | (1) What a registrar final (an `EXAM` meeting beside `CLAS`) becomes: *recommend skipped and counted* until L5's amendment designs one finals rule; the alternatives are listed-never-written now, or written now as a hard dated `exam` note. (2) What approving a Google class card is stamped when the registrar has since come to hold that course's class (B4a): *recommend `superseded`* (nothing written, one warning). Both are in the close-out plan's §8 (items 10 and 12); the plan's other §8 items are ruled (L5a's row). Also whether B9a's registered status codes join ruling 8's conditions (plan §8 item 11, *recommend yes*). |
| Pilot, ongoing | **Keep the OpenRouter account funded, with its low-balance email** | OpenRouter is prepaid credit: an empty balance answers 402 on every call, which the device reports today as `model failed` for every item. Quinn enabled auto-top-up on 2026-09-16; the $2 per-account ceiling stays as the runaway guard. The in-app 402 issue is MVP work (L7; §3 lists it as ruling 10's original Pilot item). Rotating the OpenRouter key before friends is ruled (the key-roll row above); the timing after that stays Quinn's. |
| Pilot (not an ask) | **`_shared/db.ts` `authGetUser` answers `null` for any non-OK `/auth/v1/user`** | `requireUser` turns that into 401 "the session is not valid", so a 5xx or a rate limit from the auth server reads as a revoked session (seen once on 2026-09-14, transient). The app is safe (a refused or failed answer leaves the cached entitlement alone), but the honest reply for a non-401 is 503. One small change with a test, before non-founder sign-ins. |
| Launch | **Azure** (P1) — code signing, the goal since the unsigned switch (PR #7, 2026-09-17). **Unblocked 2026-10-16: Quinn has an ID now;** friends' releases stay unsigned (2026-10-18) and signing waits for the paid release | Trusted Signing account + certificate profile; an Entra app registration with a federated credential of **entity type Environment, value `release`** (subject `repo:quinnhall07/knowlu:environment:release`, issuer `https://token.actions.githubusercontent.com`) — **not** the tag pattern the plan first said; role *Trusted Signing Certificate Profile Signer*; then tenant/client/subscription ids, endpoint, account and profile names (none secret). Then the secret and the three variables, delete `RELEASE_AUTHENTICODE`, and the next tag ships signed. |
| Launch | **`gmail.readonly` verification and the annual CASA, or accept the 100-user cap** | The restricted scope; Gmail stays in Testing mode (seven-day refresh tokens) until it lands. |
| Launch | **The two-desktop live proof** (moved from production parity's step (5) by ruling 10) | Two computers on one account, after Plans 1–3 merge (§3's Launch). The lease covers the mail pass and the slot's `mail` step too (email forwarding spec §9): until it ships, forwarding is one computer per student. |
| Launch | **Lawyer sign-off, tax registration, the private-repository decision** | The lawyer's open call includes whether the annual reminder must name the next charge date (R-C1-18). The repository is public (ruling 13); while it is, no self-hosted runner is registered. |
| Any time (trigger, set 2026-09-30) | **Rule again on ruling 12's suspended policy-read gate at the 20th paying account** (email forwarding spec D25, A13) | Counted as an `active` paid subscription (`trialing` and founder-owned test accounts not counted); the controller's count-only query checks it at each milestone HANDOFF update, and Quinn rules before the next release tagged after it. Restoring the gate is one constant, `POLICY_READ_GATE = Enforced` in `app/src/grades.rs`, with its predicate tests already written. |
| Any time | **Rotate the Blackboard token and the Google Calendar capability URL** | Neither was regenerated as far as this file knows; both sat in a OneDrive mirror. |
| Any time | **Delete the old local copies** | `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots,…}` and the old `KnowluBackup\profile_*` mirrors, after two clean slots. |
| Later | **A co-founder** | Unowned, and VISION's only open decision with no next action. |

### Done and decided (the record)

| When | Ask | Context to give them |
|---|---|---|
| done | ~~Push `main`~~ — pushed 2026-09-09 night (`ce1f9ee`). ~~Archive `quinn-ops`~~ — archived the same night; 17 worktrees removed. | Left: Delete `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots,…}` and the old `KnowluBackup\profile_*` mirrors after two clean slots. |
| done | ~~The interim page~~ — Quinn ruled 2026-09-09 night: **no re-registration**; they go without a daily page until C1's wizard lands. |
| done 09-14 | ~~C0 T4, retire the old updater key~~ — the new pair's public half is in `app/tauri.conf.json` (**key id `97D52FF797E70A1B`**), the private half and its password exist only as the two GitHub secrets (set 2026-09-14 17:06 UTC, by the corrected block — PowerShell has no `<`). **Old key id `C2EC981122E1D2DF` is retired**: no install outside the laptop ever used it; the laptop's Credential Manager holds no `knowlu/updater-key*`; the OneDrive file and its version history are Quinn's to delete. |
| **decided 2026-09-17 — desktop only** (the cloud design's Amendment 2026-09-17, ruling 1; Quinn: "we're just going to ship the desktop app and orient our plan around that"; the account carries the student between desktops, ruling 2) | ~~Multi-device parity — web app and mobile with the desktop app, "complete parity, eventually"~~ | Quinn's question during C3: account data must be reachable from every device. What the signed direction already holds: VISION "desktop only at launch … mobile is a later port, and the shared Rust core is what makes it"; the business plan ("mobile becomes viable once generation is cloud-side"; "not every device needs every capability"); the market note's LTI line ("a web app … a new surface, not a port"); the cloud design's §13 parks mobile. What C3 already gives: the account's copy is opaque encrypted rows any client can push and pull (Task 3's Deno vector is the proof a JS runtime opens the envelope), and a second device restores from the journal. What is NOT designed: a browser or mobile client runtime for the engine (the shared Rust core as WASM / a mobile build; `wincred`, `ureq`, process spawning cfg-gated), a device-pairing flow so a new device gets the key from an existing one instead of a typed recovery code, and the per-device capability split (mobile reads and decides; desktop fetches portals). **P1 is unaffected:** option (a) is what every end-to-end product uses, and pairing is how new clients join it. Recommendation: a "Clients" section for the cloud design after C3 lands, signed by Quinn, before any web or mobile work; nothing in C0–C4 changes. |
| **Quinn, for C1b (asked 2026-09-17)**; its open parts are restated under Open, Pilot | **The Google OAuth client for sign-in, the two provider settings, the Stripe promotion code, the live Stripe setup task** | **(1)–(2) DONE 2026-09-17** (Quinn: "added the google oauth for supabase"; the controller verified without a key that `/auth/v1/authorize?provider=google` on staging and on prod each answer 302 to `accounts.google.com` with the project's own `/auth/v1/callback` as `redirect_uri`, scope `email profile`). Still to confirm from Quinn: that the client lives in the NEW project with publishing status In production (a Testing-status client blocks every address that is not a listed test user). **R-C1b-5:** the provider stays dashboard-managed — no `[auth.external.google]` block in `config.toml`, because the CLI (2.117.0) pushes only the properties the file declares and leaves the rest unchanged; every `config push` is preceded by `supabase config diff`, which must show no `external.google` change. Was: (1) A NEW Google Cloud project (not the Gmail one): OAuth consent screen External, publishing status **In production**, app name Knowlu, support email, homepage `https://knowlu.com`, privacy `https://knowlu.com/privacy`, scopes only `openid`, `email`, `profile` — no verification review, no 100-test-user cap; then an OAuth client of type Web application with the two redirect URIs `https://jxthohvwrijwtuwlglan.supabase.co/auth/v1/callback` (prod) and `https://brvhgbihxevrudqpulcm.supabase.co/auth/v1/callback` (staging). (2) Its client id and secret go into Supabase → Authentication → Providers → Google on BOTH projects (Quinn sets them; the plan names them). **(3) DONE 2026-09-17** — the 100-percent coupon with a promotion code exists in both Stripe modes (Quinn); the code text lives nowhere in the repo, the ledger or a log — Quinn holds it, the controller types the test-mode one into the staging Checkout for C1b's exit-gate proof. Was: (3) In the live Stripe dashboard: a coupon of 100% off, duration forever, and a promotion code on it, for Quinn's own account (the checkout function learns `allow_promotion_codes` in C1b). (4) The live Stripe account's setup-guide task ("Continue setting up your Stripe account": payouts or identity) — until it is done a live checkout session cannot be created, which is the likely cause of the failed checkout on 2026-09-17. (6) **STILL OPEN 2026-09-22 — the API reports both projects unchanged (`enable_confirmations` true, `email_sent` 2)**; Quinn set a rate limit somewhere that did not land and could not find the Confirm-email toggle; exact paths re-sent, or Quinn runs `supabase config push` in their own terminal with `SMTP_PASSWORD` exported (the file declares exactly the two settings). **Two dashboard toggles on BOTH projects, before C1b's proofs (asked 2026-09-17):** Authentication → Providers → Email → *Confirm email* OFF (the code mail is the proof of address; with it on, a new address gets a link and no code), and Authentication → Rate Limits → *Emails sent* 20 per hour (the project default of 2 blocks a pilot). The repo's `config.toml` declares both; the controller does not push it (the SMTP password is an env reference only Quinn holds). (5) **Two decisions the plan review surfaced (2026-09-17), ruled by the controller pending Quinn's word:** the card stays required at checkout for everyone (R-C1b-2; Quinn's own card sits under the 100-percent code and is never charged, or Quinn comps the subscription from the Stripe dashboard) — the alternative, a card-free 7-day trial for every student, rewrites the terms' auto-renew sentence with the lawyer and moves `TOS_VERSION`; and whether to turn on Supabase's `[auth.captcha]` for the sign-in endpoints (the anon key is public, so `/otp` is rate-limited only project-wide) — not built in C1b. |
| done 09-14 | ~~C0 T3 run, Cloudflare (P2, P3)~~ — the Pages project `knowlu` (direct upload) with `knowlu.com` as its custom domain, `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` set by Quinn (17:22 UTC); DNS has been on Cloudflare since the morning. `https://knowlu.com/` answers 404 from Pages until the first release deploys `site/`. P3 is therefore not late: the endpoint stays `https://knowlu.com/releases/latest.json` and Task 5 step 6 is not needed. |
| C0 T3 run (set 2026-09-14 except the Azure ones, which go with Open, Launch) | **Set the secrets themselves** | `gh secret set NAME --repo quinnhall07/knowlu < file` for `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`; `gh variable set` for `TS_ENDPOINT`, `TS_ACCOUNT`, `TS_PROFILE`. You name them, they set them. |
| C1 | **P2 is done on staging (2026-09-10 afternoon)** — Quinn pasted the Stripe test keys and the Resend key and asked the controller to set it up: product, two prices, a default portal configuration (cancel at period end, no survey), the webhook endpoint at `2025-03-31.basil`, all ten function secrets (the two random ones generated blind in the shell), the Vault token, the SMTP block pushed with `supabase config push`. Proven: every function answers its designed refusal, the webhook verifies signatures, the billing job ran end to end (200). **Live mode is prepared too** (R-C1-54, on Quinn's instruction with the live secret key pasted once): live product, prices, default portal, the webhook endpoint at prod's `stripe-webhook`; prod already holds `STRIPE_PRICE_MONTHLY`, `STRIPE_PRICE_YEAR`, `STRIPE_WEBHOOK_SECRET`. **Left for Quinn:** roll the **live** Stripe secret key now and set the fresh one on prod (`supabase secrets set STRIPE_SECRET_KEY=… --project-ref jxthohvwrijwtuwlglan`); roll the test secret key and the Resend key (all passed through the chat) and re-set `STRIPE_SECRET_KEY` on staging, `EMAIL_API_KEY`, `SMTP_PASSWORD`; set the Terms-of-service and privacy URLs in Stripe's Dashboard (Settings → Business → Public details) — the API has no field for them; verify `knowlu.com` in Resend once Cloudflare DNS exists (until then mail goes only to the owner's address, from `onboarding@resend.dev`). Google consent (two scopes) and the policy texts remain as the plan's P4/P5 say. **Prod rollout** (after the go): `supabase link --project-ref jxthohvwrijwtuwlglan` is Quinn's (database password); the controller then pushes the six migrations, creates the Vault URL, deploys the eight functions and re-checks the 401 from `/entitlement`. The e-mail provider is new (Resend free tier). A legal call is open: whether the annual reminder must name the next charge date (R-C1-18). |
| done 09-14 | ~~Checkout proof~~ — proven on staging (§1). The test subscription is on Quinn's account in trial until 2026-09-21; the portal or the sandbox Dashboard cancels it. | Left for real customers: the site must resolve (`knowlu.com/terms.html`, `privacy.html`) — Stripe accepted the URLs without checking them. |
| done 09-14 | ~~Step 4, the live Stripe Dashboard~~ — Quinn set the two public URLs and the tax code and **kept Managed Payments on (P3)**. | Left on Stripe's side: the live account's setup-guide task from the 09-11 mail ("Continue setting up your Stripe account" — payouts or identity), without which the live account cannot take a real payment. The first live Checkout is the verification of all of it. **Done 2026-09-24:** Quinn activated the live account, and the first live Checkout is proven end to end on prod (§1's first ▶ block). |
| done 09-14 | ~~Re-run the staging `config push`, delete the old Resend key~~ — done; the first re-run carried the wrong value (sign-in mail 500 until the second), then the two mail secrets matched and the old key was deleted. | The CLI's link lives in `<workdir>/supabase/.temp/` and was lost with the worktree; `link --password ""` needs no database password for `db query`, `config push` and `functions deploy` — only `db push` does (`SUPABASE_DB_PASSWORD`). |
| done 09-16 | ~~`OPENROUTER_API_KEY` on staging and in GitHub (P1)~~ — set by Quinn's own shell (`supabase secrets set OPENROUTER_API_KEY=<value> --project-ref brvhgbihxevrudqpulcm`) and as the GitHub secret the eval gate reads; `ANTHROPIC_API_KEY` is deliberately unset on both, the provider swap having replaced it. | Task 3's, Task 9's and Task 11's live smokes can now make a real model call through OpenRouter (`judge-task`, `judge-event`, `judge-email`). |
| done 09-17 | ~~The Google OAuth client and consent screen (P2), Quinn as the first test user (P3)~~ — P2 set by Quinn 2026-09-16 (18:08Z: `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` on staging; the Web client's redirect URI `https://brvhgbihxevrudqpulcm.supabase.co/functions/v1/google-callback`; `openid`, `email`, `calendar.readonly`, `gmail.readonly` on the consent screen; both APIs enabled); P3 done 2026-09-17 (Quinn the one test user; the live cycle is in §1). | The consent screen stays in Testing (100 test users, a warning screen, refresh tokens that die after 7 days) until verification: `calendar.readonly` first (sensitive), `gmail.readonly` behind verification and the annual CASA; the site must be live for the homepage and privacy URLs. Production needs its own client and redirect URI. |
| done 09-17 | ~~PR #6 (`google-reconnect`): the two fixes the P3 pass found~~ — merged `0b6710f` on Quinn's word; the functions that bundle the helper redeployed to staging from `main`; the worktree and the branch deleted. | `include_granted_scopes=true` on every ask (`google-connect/handler.ts`, 30cd8cb) and `rpcReply` for an empty RPC reply (`_shared/judge_db.ts`, b50d5ec); both proven live before their commits (§1). |
| done 09-16 | ~~Two eval-gate secrets~~ — `OPENROUTER_API_KEY` and `SUPABASE_STAGING_SERVICE_ROLE_KEY` (`gh secret set`), both set 09-16; the row named `ANTHROPIC_API_KEY` before the provider swap. | The eval-gate job in `ci.yml` needs both to run for real; it stays vacuously green without them because the seed is empty by decision (R-C2-E12) until C4's consented-corrections toggle fills it. |
| done 09-14 | ~~The archive read (P4)~~ — **Quinn declined: "All the info should come from onboarding and running it."** Nothing is ever read from the archived vault. | Ruling R-C2-E12 records the consequence: `cloud/eval/` ships schema + scrub + loader only, `run_eval.ts` reports `0 cases` and passes, and the gate becomes real as consented corrections accumulate. |
| split 2026-09-29 | ~~**What C2 left for C4, itemised**~~ — ruling 10 spread the rows over the stages (§3): the settings row's Google Connect/Disconnect (H9 phase (b); `set_google_calendar` is written with its row) and the incremental-consent button to the MVP; runtime removal, the re-consent screen (R-PS-4: required before any second account; the provider swap's 2026-09-16 bump shipped without one because Quinn's was the only account) and "source went quiet" to the Pilot; the class-(c) toggle to Launch; the Calendar-API `origin` split to Beyond. | — |
| decided 09-14, for C3 | **Restore from a backup: a link on the picker, built in C3** (Quinn, 2026-09-14). Was: no screen any more | §4.2's nine panels carry no restore panel and Task 17 built exactly those, so `restore_vault` has no caller; the picker's "use an existing vault" is the only route in, and Task 18 gives it a guard against pointing at a backup mirror. Decide: accept (a restore is "copy the mirror's `vault` folder somewhere and adopt it", documented), or ask for a small restore entry on the picker in C3 (sync). |
| C3 (paused; plan superseded 2026-09-17) | ~~**P1–P7 of the C3 plan**~~ — P1 settled as (c) and P2's screen removed by the amendment; P3 was asked 2026-09-17 (200 MiB, 400 days, human `op: set` records kept for ever — built to the recommendation on the paused branch); P4–P7 return in the C3′ plan where they still apply. | the plan's *Quinn-owned preconditions* table, asked one at a time when a task reaches them. |

## 5. Standing rules (the short list; `CLAUDE.md` has the rest)

- **Never touch `quinn-ops`** except the archive step. Never run any engine binary with `--vault`
  pointing at a real vault (Quinn's is `C:\Users\danie\Knowlu\Vault` now); tests use fixtures and
  temp copies.
- **No secret ever enters this repo, a log, a commit message, a fixture, a prompt or a test name.**
- **0 warnings is part of green**; the gate line `warnings: N accepted (.rsrc), N tallies, N other`
  must end in `0 other`. The eight frozen references and the three surface references are never
  regenerated.
- **Releases come only from CI** (`release.yml` on a `v*` tag); `scripts\release.ps1 -DryRun` is the
  only local use. No tag until C0 Task 4 has landed the new key.
- **Desktop safety:** never synthetic input; screenshots by `PrintWindow` only; never launch
  `knowlu.exe` against a real profile yourself — Quinn does.
- **Commits:** specific `git add`, message via `-F <file>`, trailers with **this session's** model
  name and URL.
- Line endings are the repo's: LF, `.ps1` CRLF, fixtures bytes.
- **Suite counts drift from the plan by design:** fix rounds add tests (R-C1-16). Every dispatch states
  the plan's number and the real one.
- **Stripe API `2025-03-31.basil` moved fields** — `current_period_end` onto the subscription item, the
  invoice's subscription under `parent.subscription_details`. Every new read of a Stripe object handles
  both shapes (R-C1-15, R-C1-17).
- **Subagent shells may lack Deno on PATH** (installed after the session started) — every dispatch
  carries the refresh line. `review-package` run from a worktree writes under that worktree's own
  `.superpowers/`: pass the OUTFILE argument.

## 6. Quick reference

```
cargo test --workspace                       # both crates; the gate CI runs
cargo build --release --workspace            # target\release\knowlu-engine.exe, knowlu.exe (+ WebView2Loader.dll)
.\scripts\release.ps1 -DryRun                # local bundle, unsigned, publishes nothing
.\scripts\ci\eol-check.ps1                   # the line-ending contract
.\scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-full   # a throwaway vault
gh run list --workflow ci.yml / gh run watch <id> --exit-status        # CI
```

App data: `%LOCALAPPDATA%\knowlu\` (`profiles.json`, `profiles\<id>\`). Vaults: `%USERPROFILE%\Knowlu\<Profile>\`,
backups `%USERPROFILE%\Knowlu\Backups\`. The installed app: `%LOCALAPPDATA%\Programs\Knowlu\`.
