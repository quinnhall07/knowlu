# HANDOFF — where Knowlu stands and how the next session runs it

**Rewritten 2026-09-09 (evening) by the session that signed the spec, ran cut day and executed C0
Tasks 1–3.** Read this whole file, then `CLAUDE.md`, then `docs/specs/2026-09-09-knowlu-cloud-design.md`
(signed; §11a holds the rulings made after signing), then the plan you are executing. Everything
older under `docs/` is history or design authority from the previous repository (`PROVENANCE.md`).

---

## 1. State, in facts

▶ **C2 IN FLIGHT (started 2026-09-14 evening)** on branch `c2-judge` (worktree `.claude/worktrees/c2-judge`, draft PR #4 → `main`, CI green on every task so far; head 2176778): Tasks 0–6 (Task 7 in its fix round) done and reviewed — the model client behind one interface (`npm:@anthropic-ai/sdk@0.125.0`, imported by its pinned specifier because the `--use-api` bundler does not read `deno.json`'s import map), the judgment schema on staging, the shared pipeline, `POST /judge-task` deployed, `CloudModel` in the engine (`judge` exits 0 on every failure), the two parser ports gated by the frozen references, `POST /ingest-coursework` deployed and smoked with the zyBooks fixture (24 assignments). The ledger with every ruling: `.superpowers/sdd/2026-09-09-c2-judge-plan/progress.md` + `preflight-rulings.md` (git-ignored on this laptop). **Quinn's 2026-09-14 decision: nothing is ever read from the `quinn-ops` archive** — the eval suite is empty at merge and fills from consented corrections. Still Quinn's: `ANTHROPIC_API_KEY` on staging (P1, unset); the Google OAuth client + consent screen (P2) and the first test user (P3).

▶ **C0: PR #3 MERGED 2026-09-14** (`1b7bbea`, `--no-ff`, on Quinn's word): the updater key rotated (`97D52FF797E70A1B`), the README's Releases section, Cloudflare Pages + `knowlu.com`, the Azure app registration. **The one blocker for `v0.1.0` is still Azure Artifact Signing** (no subscription); the four lines Quinn hands back → the secret + three variables → the tag (Task 5) → the close (Task 6). The C0 ledger's ▶ RESUME HERE block has the exact steps.

▶ **C1 DONE and MERGED 2026-09-10** (`82dcc04`, `--no-ff`, first parent `8c0e05a`, PR #2; 60 commits, every task reviewed, the whole-branch review said merge; the merge-time hand-offs H4/H5/H6/H8 in `512567d`; CI on `main` run 34547538926 green): accounts, entitlement and the new wizard. Supabase `knowlu-staging` and `knowlu-prod` are live; Stripe is in test mode on staging with the monthly and academic-year prices, the 7-day trial and the June–August pause, and live mode is prepared on prod (prices, portal, webhook) pending Quinn's fresh live secret key. `GET /entitlement` is cached on the device with a 72-hour grace and a slot with no subscription says `judge (skipped: no entitlement)` and stays green. The wizard is nine panels with no folder question; an install from before C1 is adopted in place. Telemetry (a) and (b) go up at each slot; issue reports are previewed and scrubbed before they are sent. The privacy policy and the terms are written and committed to `site/`, not yet live at `knowlu.com`. **Google's restricted-scope verification has not been submitted** — P4 waits for the site to be live; Gmail stays testing-mode only until it lands. LMS link capture: outcome **B** from the Task 13 spike (the sign-in window's cookies fetch the feed URL in one GET and the course list in another). The stream report: `docs/reports/2026-09-10-c1-accounts-stream-report.md`; Quinn's two packets (P4 texts, P5 facts): `docs/reports/2026-09-10-c1-quinn-packets.md`; the merge-time applications (H4, H5, H6, H8) are on `main` in `512567d`; the SDD workspace and the `c1-task16` worktree are deleted; the empty `.claude/worktrees/c1-accounts` directory is held open by the merging session and goes when it ends. **Checkout proven end to end on staging 2026-09-14** (after Quinn rolled the three exposed keys and set the sandbox's two public URLs): a fresh sign-in code, `POST /billing-checkout` with the rolled test key, the test card, the webhook writing `entitlements` (`trialing`, monthly, period end 2026-09-21) three seconds after payment, the `auto_renew` consent row at $9.99 / terms 2026-09-10, `GET /entitlement` flipped, `POST /billing-portal` opening. Sandbox mail from `onboarding@resend.dev` lands in Gmail's spam; Resend's domain verification (DNS records only, no live site needed) is the fix. **Prod rolled out 2026-09-14** on Quinn's instruction (they pasted the prod database password and the new Resend key in chat, to be rolled before launch): the seven migrations, all eight functions (`--use-api`), all ten function secrets, the auth config (SMTP), the two Vault entries (a stale 44-char `billing_jobs_token` from the 09-10 prep was replaced so it matches the function secret), three cron jobs active; every function answers its designed refusal and the job called through pg_net with the Vault token returned 200. Not yet done on prod: a real sign-up (cut day). **The Stripe live Dashboard items were done by Quinn on 2026-09-14** (the two public URLs, the live product's tax code) and **Managed Payments stays on — P3 decided**: Stripe is the merchant of record for tax and disputes at the higher per-charge fee; unverifiable from here until the first live Checkout, which also needs the site to resolve. **Step 5 done 2026-09-14**: `knowlu.com` verified in Resend (DNS hosting moved to Cloudflare; the registrar stays Namecheap); the sender is **`hello@knowlu.com`** — `EMAIL_FROM` = `Knowlu <hello@knowlu.com>` on both projects, `config.toml`'s `admin_email`; Quinn pushed the auth config on both projects (2026-09-14 ~16:30 UTC); **proven on staging**: a sign-in code from `hello@knowlu.com` reached the Gmail inbox in six seconds, not spam. Cloudflare's zone import carried Namecheap's five `eforward*.registrar-servers.com` MX records and its SPF TXT on the apex, which block Email Routing until deleted (Resend's own records sit on `send.` and `resend._domainkey.` and stay). Quinn deleted them, enabled Email Routing, verified the Gmail destination (Cloudflare's verification mail; rules stay inactive until then — the first test bounced 550 from `mx.cloudflare.net`) and a test to `hello@knowlu.com` arrived in Gmail. **Step 5 complete 2026-09-14**: `hello@`, `support@` and `security@knowlu.com` forward to Quinn's Gmail; there is no mailbox at the domain. The policies also name `support@knowlu.com` and `security@knowlu.com`: Cloudflare Email Routing forwards all three. Staging sign-in mail is proven on the rolled key (a 200 after the old key was deleted). **Step 6 decided 2026-09-14 — keep, all four**: restore-from-backup becomes a link on the picker in C3; the in-app overlay sells monthly only; no magic link in the overlay; export stays by email until a settings row in C3. The repo's CLI is linked back to **staging** (`--password ""`); prod commands take `--project-ref jxthohvwrijwtuwlglan`.

- **This repo:** `quinnhall07/knowlu`, private. `main` was pushed on 2026-09-09 night (`ce1f9ee`; it had been 49 commits ahead). The day.s commits were: the signed
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

Streams, one worktree each (`.claude/worktrees/<branch>`, git-ignored), one plan each, disjoint
files; shared files only by the controller at merge. Merge order **C0 → C1 → C2**.

| Stream | Plan | Branch | Owns (exclusive) | Needs from Quinn |
|---|---|---|---|---|
| **C0 — CI release** | `docs/plans/2026-09-09-c0-ci-release-plan.md` — **Tasks 1–3 DONE and merged**; Tasks 4–6 remain | `c0-ci-release` (merged; **make `c0-release-key` from `main` for Tasks 4–6**) | `.github/**`, `scripts/ci/**`, `scripts/release.ps1`, `scripts/sign.ps1`, `engine/tests/workflows.rs`; `app/tauri.conf.json` only `plugins.updater.pubkey`, `endpoints`, `version` | P4 (retire the old updater key — Task 4's PowerShell block), P1 Azure (**federated credential entity-type Environment = `release`**, not the tag), P2 Cloudflare, P3 `knowlu.com`, the secrets |
| **C1 — accounts, entitlement, the calendars-first wizard, telemetry, issue reports** | `docs/plans/2026-09-09-c1-accounts-plan.md` — **DONE, merged 2026-09-10 (`82dcc04`)** | `c1-accounts` (merged) | `cloud/supabase/**` except C2's function dirs and `_shared/judge_*.ts`; `app/src/{onboarding,profiles,scaffold,credentials,scheduler}.rs`; new `app/src/{account,telemetry,report,lms_link}.rs`; `app/static/**`; `app/tests/**`; `site/**` | Supabase org (prod + staging); Stripe test mode with both prices, the pause, the Portal, the webhook; **an e-mail provider (Resend free tier) for Supabase Auth mail and the annual reminder**; Stripe Tax + Kentucky; the Google consent screen (two scopes); privacy/ToS read |
| **C2 — the judgment service** | `docs/plans/2026-09-09-c2-judge-plan.md` — **written, reviewed (3 rounds), ready after C1** | `c2-judge` | `cloud/supabase/functions/{judge-*,ingest-*,events,google-*,gmail-read}`, `_shared/judge_*.ts`, `migrations/20260911*.sql`, `cloud/eval/**`; `engine/src/{judge,enrich,cloudmodel,coursework,judgelog,events}.rs`; new `engine/tests/*.rs` | `ANTHROPIC_API_KEY` in the staging project's secrets; the Google OAuth client id/secret; Quinn as the first Gmail test user; the one-time go to read the `quinn-ops` archive for the eval seed |
| **Cut day** | spec §7.2 | — | — | the archive step only |
| **Docs** | — | controller, on `main` | `VISION.md`, `CLAUDE.md`, `README.md`, this file, the spec | — |

**Shared files with a single owner:** root `Cargo.toml`/`Cargo.lock`, `app/src/main.rs` (the two
`generate_handler!` lists), `app/src/lib.rs`, `app/src/commands.rs`, `app/src/tray.rs`,
`app/src/updates.rs`, `app/src/state.rs`, `app/tauri.conf.json` outside C0's three keys,
`engine/src/lib.rs`, `engine/src/{main,cli,ingest,runtime,inference}.rs`, `scripts/*.py`. Every plan
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

## 3. Sequence

1. ~~Spec review~~ — done; §11a records the rulings since.
2. **C0:** Tasks 1–3 merged; Tasks 4–6's branch **merged as PR #3 on 2026-09-14** (`1b7bbea`). Left: Azure Artifact Signing (Quinn's Cowork session hands back `AZURE_SUBSCRIPTION_ID`, `TS_ENDPOINT`, `TS_ACCOUNT`, `TS_PROFILE`), then Task 5 (the `v0.1.0` tag, Quinn's word — it makes `knowlu.com` public) and Task 6 (branch protection, the close). Steps: the C0 ledger's ▶ RESUME HERE block.
3. ~~Plans C1 and C2~~ — written, reviewed, committed. **C2 is executing** (§1's ▶ block; draft PR #4).
4. **Cut day:** the reset (§1) and the archive step remain (§4).
5. **C1 is merged** into `main` (2026-09-10 evening: `82dcc04`, `--no-ff`, first parent `8c0e05a`, PR #2
   marked merged; the controller's merge-time hand-offs H4, H5, H6 and H8 in `512567d`). The gate on the
   merged tree: `cargo test --workspace` at 0 unaccepted warnings, deno check/lint/test (125 passed), the
   eol contract over 294 files, `python scripts/wizard-check.py` → `ok`; CI on `main` run 34547538926 green.
   The task-by-task history is the stream report (`docs/reports/2026-09-10-c1-accounts-stream-report.md`);
   rulings R-C1-9…60 are in the review report. The `c1-task16` worktree, both local branches and the SDD workspace are deleted (the empty
   `.claude/worktrees/c1-accounts` directory is held open by the merging session — delete it after); branch `c1-accounts` stays on origin as PR #2's record. What still waits on
   Quinn is §4 (the key roll, the two Stripe dashboards, the prod link, Resend's domain, P5, P4, the
   product calls). Then **C2** (`docs/plans/2026-09-09-c2-judge-plan.md`, written 2026-09-09, not
   started): branch `c2-judge` from `main`, in a worktree, subagent-driven like C1.
6. **C3's plan is written and reviewed (2026-09-14/15):** `docs/plans/2026-09-14-c3-sync-plan.md` (14 tasks; `/sync-push`, `/sync-pull`, the sealed envelope with a device-held key, restore from the cloud copy on the picker, the export row, git out of the product, the two capability URLs out of the vault) with `docs/reports/2026-09-14-c3-sync-plan-review.md` (two fix rounds; the report's last section carries the verdict). Its seven Quinn-owned preconditions (P1–P7: the key's derivation and recovery code, the opt-in switch and policy text, the ceiling and retention, the go to delete `history.rs`, …) are in the plan's own table. Gate to start: C2 merged. C4 (remove the local runtime; the (c) toggle UI; the settings-panel Google connect/disconnect; the "source went quiet" surface) is still unplanned.

## 4. Quinn's queue — one at a time, when reached, with the context

| When | Ask | Context to give them |
|---|---|---|
| done | ~~Push `main`~~ — pushed 2026-09-09 night (`ce1f9ee`). ~~Archive `quinn-ops`~~ — archived the same night; 17 worktrees removed. | Left: Delete `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots,…}` and the old `KnowluBackup\profile_*` mirrors after two clean slots. |
| done | ~~The interim page~~ — Quinn ruled 2026-09-09 night: **no re-registration**; they go without a daily page until C1's wizard lands. |
| done 09-14 | ~~C0 T4, retire the old updater key~~ — the new pair's public half is in `app/tauri.conf.json` (**key id `97D52FF797E70A1B`**), the private half and its password exist only as the two GitHub secrets (set 2026-09-14 17:06 UTC, by the corrected block — PowerShell has no `<`). **Old key id `C2EC981122E1D2DF` is retired**: no install outside the laptop ever used it; the laptop's Credential Manager holds no `knowlu/updater-key*`; the OneDrive file and its version history are Quinn's to delete. |
| C0 T3 run — **the one blocker for v0.1.0** (2026-09-14: "signer isn't up yet") | **Azure** (P1) | Trusted Signing account + certificate profile; an Entra app registration with a federated credential of **entity type Environment, value `release`** (subject `repo:quinnhall07/knowlu:environment:release`, issuer `https://token.actions.githubusercontent.com`) — **not** the tag pattern the plan first said; role *Trusted Signing Certificate Profile Signer*; then tenant/client/subscription ids, endpoint, account and profile names (none secret). |
| done 09-14 | ~~C0 T3 run, Cloudflare (P2, P3)~~ — the Pages project `knowlu` (direct upload) with `knowlu.com` as its custom domain, `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` set by Quinn (17:22 UTC); DNS has been on Cloudflare since the morning. `https://knowlu.com/` answers 404 from Pages until the first release deploys `site/`. P3 is therefore not late: the endpoint stays `https://knowlu.com/releases/latest.json` and Task 5 step 6 is not needed. |
| C0 T3 run | **Set the secrets himself** | `gh secret set NAME --repo quinnhall07/knowlu < file` for `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`; `gh variable set` for `TS_ENDPOINT`, `TS_ACCOUNT`, `TS_PROFILE`. You name them, they set them. |
| C1 | **P2 is done on staging (2026-09-10 afternoon)** — Quinn pasted the Stripe test keys and the Resend key and asked the controller to set it up: product, two prices, a default portal configuration (cancel at period end, no survey), the webhook endpoint at `2025-03-31.basil`, all ten function secrets (the two random ones generated blind in the shell), the Vault token, the SMTP block pushed with `supabase config push`. Proven: every function answers its designed refusal, the webhook verifies signatures, the billing job ran end to end (200). **Live mode is prepared too** (R-C1-54, on Quinn's instruction with the live secret key pasted once): live product, prices, default portal, the webhook endpoint at prod's `stripe-webhook`; prod already holds `STRIPE_PRICE_MONTHLY`, `STRIPE_PRICE_YEAR`, `STRIPE_WEBHOOK_SECRET`. **Left for Quinn:** roll the **live** Stripe secret key now and set the fresh one on prod (`supabase secrets set STRIPE_SECRET_KEY=… --project-ref jxthohvwrijwtuwlglan`); roll the test secret key and the Resend key (all passed through the chat) and re-set `STRIPE_SECRET_KEY` on staging, `EMAIL_API_KEY`, `SMTP_PASSWORD`; set the Terms-of-service and privacy URLs in Stripe's Dashboard (Settings → Business → Public details) — the API has no field for them; verify `knowlu.com` in Resend once Cloudflare DNS exists (until then mail goes only to the owner's address, from `onboarding@resend.dev`). Google consent (two scopes) and the policy texts remain as the plan's P4/P5 say. **Prod rollout** (after the go): `supabase link --project-ref jxthohvwrijwtuwlglan` is Quinn's (database password); the controller then pushes the six migrations, creates the Vault URL, deploys the eight functions and re-checks the 401 from `/entitlement`. The e-mail provider is new (Resend free tier). A legal call is open: whether the annual reminder must name the next charge date (R-C1-18). |
| done 09-14 | ~~Checkout proof~~ — proven on staging (§1). The test subscription is on Quinn's account in trial until 2026-09-21; the portal or the sandbox Dashboard cancels it. | Left for real customers: the site must resolve (`knowlu.com/terms.html`, `privacy.html`) — Stripe accepted the URLs without checking them. |
| done 09-14 | ~~Step 4, the live Stripe Dashboard~~ — Quinn set the two public URLs and the tax code and **kept Managed Payments on (P3)**. | Left on Stripe's side: the live account's setup-guide task from the 09-11 mail ("Continue setting up your Stripe account" — payouts or identity), without which the live account cannot take a real payment. The first live Checkout is the verification of all of it. |
| done 09-14 | ~~Re-run the staging `config push`, delete the old Resend key~~ — done; the first re-run carried the wrong value (sign-in mail 500 until the second), then the two mail secrets matched and the old key was deleted. | The CLI's link lives in `<workdir>/supabase/.temp/` and was lost with the worktree; `link --password ""` needs no database password for `db query`, `config push` and `functions deploy` — only `db push` does (`SUPABASE_DB_PASSWORD`). |
| before launch | **Roll the prod database password and the Resend key** (both were pasted in chat on 2026-09-14) | Database: reset at `https://supabase.com/dashboard/project/jxthohvwrijwtuwlglan/settings/database`; nothing else references it. Resend: create the new key, then on **both** projects `supabase secrets set EMAIL_API_KEY=… SMTP_PASSWORD=… --project-ref <ref>` and `$env:SMTP_PASSWORD=…; supabase config push --workdir cloud` while linked to each (`link --project-ref <ref> --password ""`), **then** delete the old key — the other order breaks sign-in mail. `BILLING_JOBS_TOKEN` and `SOURCES_ENC_KEY` on prod were generated on this machine and never shown; they need no roll. |
| C2 | **`_shared/db.ts` `authGetUser` answers `null` for any non-OK `/auth/v1/user`** | `requireUser` turns that into 401 "the session is not valid" — a 5xx or a rate limit from the auth server reads as a revoked session (seen once on 2026-09-14, transient). The app is safe (a refused or failed answer leaves the cached entitlement alone), but the honest reply for a non-401 is 503. One small change with a test; C2 owns the next pass over `_shared`. |
| C1 (P5) | **Confirm eleven facts in the drafted policies** | Task 19 drafted `site/privacy.html` and `site/terms.html` from the legal briefing, reviewed for truth against the code (R-C1-56). `docs/reports/2026-09-10-c1-quinn-packets.md` (P5 section) lists what only Quinn can settle: the legal entity's name, the `support@` and `security@` mailboxes (must exist before publication), Alabama as governing law, that the draft carries **no arbitration clause** (informal resolution then the courts — the lawyer's call), and the Stripe Dashboard settings the terms assume (portal cancel with no survey — done; the billing-email portal link). Hand the two pages to the lawyer with that list. |
| decided 09-14, for C3 | **Restore from a backup: a link on the picker, built in C3** (Quinn, 2026-09-14). Was: no screen any more | §4.2's nine panels carry no restore panel and Task 17 built exactly those, so `restore_vault` has no caller; the picker's "use an existing vault" is the only route in, and Task 18 gives it a guard against pointing at a backup mirror. Decide: accept (a restore is "copy the mirror's `vault` folder somewhere and adopt it", documented), or ask for a small restore entry on the picker in C3 (sync). |
| C2, now | **`ANTHROPIC_API_KEY` on staging (P1)** — `supabase secrets set ANTHROPIC_API_KEY=<value> --project-ref brvhgbihxevrudqpulcm` from Quinn's own shell | Task 3's live smoke and every eval run wait on it; nothing else does. |
| C2, long lead | **The Google OAuth client and consent screen (P2), Quinn as the first test user (P3)** | Web application client, redirect URI `https://brvhgbihxevrudqpulcm.supabase.co/functions/v1/google-callback`; `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` as staging secrets; two scopes on the consent screen, `calendar.readonly` submitted first. Needs the site live for the homepage and privacy URLs. |
| decided 09-14 | ~~The archive read (P4)~~ — **declined by Quinn**: nothing is ever read from `quinn-ops`; every dataset comes from onboarding and real use. | The eval suite ships empty and fills from consented corrections (C4's (c) toggle). |
| C3 | **P1–P7 of the C3 plan** | the plan's *Quinn-owned preconditions* table, asked one at a time when a task reaches them. |
| soon | **Rotate the Blackboard token and the Google Calendar capability URL** | they regenerated neither yet as far as this session knows; both sat in a OneDrive mirror. |
| later | The first friend; the lawyer; a co-founder | — |

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
