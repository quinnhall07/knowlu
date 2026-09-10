# HANDOFF — where Knowlu stands and how the next session runs it

**Rewritten 2026-09-09 (evening) by the session that signed the spec, ran cut day and executed C0
Tasks 1–3.** Read this whole file, then `CLAUDE.md`, then `docs/specs/2026-09-09-knowlu-cloud-design.md`
(signed; §11a holds the rulings made after signing), then the plan you are executing. Everything
older under `docs/` is history or design authority from the previous repository (`PROVENANCE.md`).

---

## 1. State, in facts

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
| **C1 — accounts, entitlement, the calendars-first wizard, telemetry, issue reports** | `docs/plans/2026-09-09-c1-accounts-plan.md` — **written, reviewed (4 rounds), ready** | `c1-accounts` | `cloud/supabase/**` except C2's function dirs and `_shared/judge_*.ts`; `app/src/{onboarding,profiles,scaffold,credentials,scheduler}.rs`; new `app/src/{account,telemetry,report,lms_link}.rs`; `app/static/**`; `app/tests/**`; `site/**` | Supabase org (prod + staging); Stripe test mode with both prices, the pause, the Portal, the webhook; **an e-mail provider (Resend free tier) for Supabase Auth mail and the annual reminder**; Stripe Tax + Kentucky; the Google consent screen (two scopes); privacy/ToS read |
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
2. **C0:** Tasks 1–3 merged. **Next:** branch `c0-release-key` from `main`; Task 4 (P4 + Quinn's
   keygen block), Task 3's first real run needs P1/P2/secrets, Task 5 (first tag `v0.1.0`, the
   updater proof), Task 6 (branch protection, docs close). CLAUDE.md and `app/README.md` already say
   the CI-only truth.
3. ~~Plans C1 and C2~~ — written, reviewed, amended for what the first run taught (C1: 25 tasks
   incl. 14a/14b/14c; C2: 18 tasks incl. 7a/8a), committed.
4. **Cut day:** the reset (§1) and the archive step remain (§4).
5. **C1 is executing** on `c1-accounts` (worktree `.claude/worktrees/c1-accounts`, PR #2 draft, CI green
   on every push so far). The plan is final (`d8fd00d`, amended by `39c7a52` and `234c28b`). **Done,
   offline halves:** Task 0 (skeleton), 1 (the account tables + RLS), 2 (`_shared` REST / bearer /
   entitlement — the 402 contract C2 imports), 3 (`GET /entitlement`), 4 (Stripe Checkout, the webhook
   as the only writer of `entitlements`, the Portal — one fix round), 5 (the billing job, the invoice
   branch and the idempotency/ordering guard — one fix round, review clean), 6 (`DELETE /account`, export — one fix round: the deleted-account webhook loop, the retryable deletion, the paged export), 7 (the sources routes, AES-GCM at rest — R-C1-21 narrowed the export guard, one fix round of security pins), 8 (telemetry — one fix round: the class-(c) opt-in can no longer come from the request, R-C1-24), 9 (issues — one fix round: the second scrub now catches any URL scheme, a two-word Windows user and a pasted credential, R-C1-26). **The cloud half (Tasks 0–9) is built.** H1 (`e9db073`) is merged into the branch. **App tasks so far:** Task 10 (`account.rs` — session, sign-in, magic link; one fix round: a redacting `Debug`, `sign_out(Option<String>)`, the blob-size guard, R-C1-28; its two P1 steps — the GoTrue `/verify` type, the prod constants — wait with the batch, R-C1-25) done; Task 11 (the entitlement cache, the 72-hour grace, the `JudgePlan` gate C2 consumes — one fix round: an hour of clock-skew tolerance, an `Unreadable` state for a corrupt `cloud.yaml`, R-C1-29; and the Credential Manager test lock, R-C1-30) done; Task 12 (the vault location, `config/cloud.yaml` — one fix round: restore takes the home path like create, the personal calendar is validated on the device, R-C1-31) done; Task 13 (the sign-in window spike): scaffolding + H9a on the branch; **spike run 1 done** — the window works (SSO + Duo inside it), the share link is page content not the address (outcome A out), the course API was never reached (no address bar); **runs 2 and 3 done (2026-09-10 morning): Outcome B** — the window's cookies are readable (`cookies_for_url`) and a plain `GET /webapps/calendar/calendarFeed/url` with them answers 200 with the feed URL as its body, no nonce needed; **Courses: GO** — `/learn/api/public/v1/users/me/courses?expand=course` carries each course's name; Canvas not tested. The Outcome and Courses lines sit at the top of Task 13 in the plan; the spike surface is off the branch (Task 13 complete, review pending); the two probe patches live in the SDD workspace; the spike app, its data directory and the four campus-session directories under `%TEMP%` are deleted. **Task 15 (device telemetry) is done** — one fix round (R-C1-39: a correction's values travel only as a number or a token, on both ends; refused batches are consumed, not retried forever), re-review pending. **Task 14 (the capture) is done** — outcome B implemented, one fix round (R-C1-41: one host's cookies never go to another; `webcal://` pasted links are rewritten), re-review clean, live run 2026-09-10 with Quinn: 156 events captured and validated, the incognito cookie read confirmed; the campus session directory is wiped on the window's destroy event and at exit (R-C1-42). **H11 is generated** (4,319 schools, R-C1-37) and waits in the SDD workspace for Task 14c. **P1 is done and the staging batch has run (2026-09-10):** the two projects live in `us-east-1` — `knowlu-staging` **`brvhgbihxevrudqpulcm`**, `knowlu-prod` **`jxthohvwrijwtuwlglan`**. Staging holds six migrations (000100–000600): ten tables all with RLS, zero non-SELECT policies, `pg_cron` + `pg_net`, three cron rows; 000200 needed its column adds moved ahead of its view (R-C1-34); the billing job reads its URL and token from Vault because `alter database set` is refused on Supabase (R-C1-36, migration 000600; the URL half is in staging's Vault). All eight functions are deployed with `verify_jwt = false` (R-C1-32 proven); `entitlement`, `telemetry`, `issues` answer 401 unauthenticated, the five that need Stripe or a secret answer 500 "the function is not configured" until P2. Task 10's P1 steps are done: GoTrue's verify type is `magiclink`, the prod constants are pinned (R-C1-35). **Prod holds nothing yet** — its migrations need Quinn's `supabase link` (the database password), and the deploy needs a go. **The `cloud` CI job (H7) was pulled forward onto `main` and merged into the branch** (R-C1-23), so every push now runs `deno check/lint/test` too.
   **The staging batch (R-C1-14) ran on 2026-09-10.** What is left waits for P2: Task 4 steps 1/13/14,
   Task 5 step 8's job call and the token half of step 9, Task 6 step 7, Task 7 step 8 — the five
   secret-bearing functions answer 500 until `supabase secrets set`. Rulings R-C1-13…36 are in
   the review report (`docs/reports/2026-09-09-c1-accounts-plan-review.md`, "Execution rulings") — the
   ledger under `.superpowers/` is git-ignored scratch. **H10 is done** (`coursework-discover`, one fix round, proven against Quinn's accounts through throwaway credentials — R-C1-43). **Task 14a** (the coursework mapping) is done — one fix round (R-C1-47). **Task 16** (the report preview and the scrub twin) is done — built in a second worktree on branch `c1-task16` because the one-implementer rule is per worktree (R-C1-44), two fix rounds (R-C1-46: the twin is the cloud's six regexes, not hand-written scanners; R-C1-49: JavaScript's whitespace spelled out), rebased and fast-forwarded into `c1-accounts` at `7c39cb9`, CI green there. **Task 14b** (the enrolled courses, H9b folded in — R-C1-48) is implemented (`aeac057`) and under review. **H11 is committed** (`14acc36`: the generator script and the 4,319-row `app/campuses.json`). **Task 20 was tried early on Quinn's instruction and parked** (R-C1-50): its download-page copy reuses the sentence Task 17 writes and Task 19 pins, and `engine/tests/site.rs` holds the old sentence until then; the two Google-packet texts it prepares are in the SDD workspace (`task-20-report.md`). **Next:** 14c, then 17–18 (the wizard, adopt-in-place), 19, 20, 21 (the policies, the download page, the close). The test counts
   in the plan are +38 behind the branch after Task 9 (R-C1-16). Then **C2**, merging C1
   first; apply H9 between the two merges.
6. C3 (sync; git out of the product) and C4 (remove the local runtime; the (c) toggle UI; the
   settings-panel Google connect/disconnect; the "source went quiet" surface) get plans after C2.

## 4. Quinn's queue — one at a time, when reached, with the context

| When | Ask | Context to give them |
|---|---|---|
| done | ~~Push `main`~~ — pushed 2026-09-09 night (`ce1f9ee`). ~~Archive `quinn-ops`~~ — archived the same night; 17 worktrees removed. | Left: Delete `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots,…}` and the old `KnowluBackup\profile_*` mirrors after two clean slots. |
| done | ~~The interim page~~ — Quinn ruled 2026-09-09 night: **no re-registration**; they go without a daily page until C1's wizard lands. |
| C0 T4 | **Retire the old updater key** (P4) | the exact PowerShell block in the plan's Task 4 generates the new pair, sets the two secrets, prints only the public key; then `cmdkey /delete:knowlu/updater-key{,-password}` and the OneDrive file + its version history. Old key id `C2EC981122E1D2DF`. |
| C0 T3 run | **Azure** (P1) | Trusted Signing account + certificate profile; an Entra app registration with a federated credential of **entity type Environment, value `release`** (subject `repo:quinnhall07/knowlu:environment:release`, issuer `https://token.actions.githubusercontent.com`) — **not** the tag pattern the plan first said; role *Trusted Signing Certificate Profile Signer*; then tenant/client/subscription ids, endpoint, account and profile names (none secret). |
| C0 T3 run | **Cloudflare** (P2, P3) | account, Pages project `knowlu` (direct upload), API token Pages:Edit, account id; `knowlu.com` on Cloudflare DNS → Pages. |
| C0 T3 run | **Set the secrets himself** | `gh secret set NAME --repo quinnhall07/knowlu < file` for `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`; `gh variable set` for `TS_ENDPOINT`, `TS_ACCOUNT`, `TS_PROFILE`. You name them, they set them. |
| C1 | **Supabase, Stripe, the e-mail provider, Google consent (two scopes), the policy texts** | as the C1 plan's P1–P5 say, plus what Task 4's and Task 5's reviews added: **P2 must also set the Terms of service URL (and the privacy URL) in Stripe's public business details** — Checkout refuses the terms checkbox without it — **and create the webhook endpoint at API version `2025-03-31.basil`**; the function secrets by `supabase secrets set --project-ref <ref>`: `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `SOURCES_ENC_KEY`, `BILLING_JOBS_TOKEN`, `EMAIL_API_KEY`, and the two plain values `EMAIL_API_URL`, `EMAIL_FROM`; then the job token into Vault (R-C1-36, replaces Task 5 step 9's database settings): `select vault.create_secret('<the same token>', 'billing_jobs_token');` in each project's SQL editor — staging's URL half is already there. **Prod rollout** (after the go): `supabase link --project-ref jxthohvwrijwtuwlglan` is Quinn's (database password); the controller then pushes the six migrations, creates the Vault URL, deploys the eight functions and re-checks the 401 from `/entitlement`. The e-mail provider is new (Resend free tier). A legal call is open: whether the annual reminder must name the next charge date (R-C1-18). |
| C2 | **Anthropic key; Google OAuth client; first test user; the archive read** | as the C2 plan's P0a–P4 say. |
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
