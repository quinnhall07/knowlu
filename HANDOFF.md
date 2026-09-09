# HANDOFF — where Knowlu stands and how the next session runs it

**Rewritten 2026-09-09 (evening) by the session that signed the spec, ran cut day and executed C0
Tasks 1–3.** Read this whole file, then `CLAUDE.md`, then `docs/specs/2026-09-09-knowlu-cloud-design.md`
(signed; §11a holds the rulings made after signing), then the plan you are executing. Everything
older under `docs/` is history or design authority from the previous repository (`PROVENANCE.md`).

---

## 1. State, in facts

- **This repo:** `quinnhall07/knowlu`, private. `main` is ahead of `origin/main` (`0aaa771`) by the
  commits of 2026-09-09 — **not pushed; Quinn's go is the first ask below.** They are: the signed
  spec + VISION §10 + CLAUDE direction (`48b83dc`), spec §11a rulings (`f7f1816`, `b569fc9`), the
  wizard defaults (`18cadd2`), the first slot at launch (`f8649d5`), plans C1 (`58954ef`, `c6a18ca`,
  `c95a6d1`) and C2 (`ede9b70`), the C0 merge (`d67bafa`, `--no-ff`, first parent `ede9b70`), and the
  C0 docs. Gates: PR #1's last run 34356994036 on the runner **1021 passed / 0 failed**, `warnings: 1
  accepted (.rsrc), 2 tallies, 0 other`, `eol contract holds over 223 files`; the local workspace
  run on the merged tree is recorded in the C0 ledger.
- **Cut day is done except the archive step.** Quinn quit the old Knowlu, disabled the cloud
  routine, and onboarded with the new build into `C:\Users\danie\Knowlu\Vault` (he kept the wizard's
  default name, so the profile is "Vault", `profile_a95daa1d40`; backups at `%USERPROFILE%\Knowlu\Backups`;
  Blackboard feed set; zyBooks/VHL credentials at `knowlu/profile_a95daa1d40/{zybooks,vhl}`;
  `runners.yaml` app-scheduled, slots 12:00/18:00). The installed build is at
  `%LOCALAPPDATA%\Programs\Knowlu\` (`knowlu.exe`, `knowlu-engine.exe`, `WebView2Loader.dll`) and the
  Start-menu shortcut points there; the app re-registered its own autostart entry. **A newer build
  (`f8649d5`, first slot at launch) is built in `target\release\` but not installed** — copy the three
  files in the next time Knowlu is quit. The old `quinn-ops-local-runner` task is unregistered (its
  XML, the old autostart value and the two old profiles are in `%LOCALAPPDATA%\knowlu-pre-cut-2026-09-09\`).
  **`quinn-ops` is not yet archived**: its worktrees are still there and `gh repo archive` has not run
  — Quinn's go, at the machine.
- **As of the last check no slot had run in the new vault** (no `today.md`); the page is empty until
  Quinn presses *Run now* in the tray or 12:00 arrives, and the personal calendar is not connected
  (`calendars: []` in `config/ingest.yaml`) — the current wizard never asks. He was told how to add
  the secret iCal address by hand.
- **The old repo `quinn-ops`** (`C:\Users\danie\GitHub\quinn-ops`) is now inert (no task, no routine,
  no running app) but still Quinn's old vault. **Do not build, run or edit anything there** except the
  archive step.
- **Tooling on this laptop:** as before (Rust 1.98 GNU host + WinLibs 16.1.0-r4 MSVCRT, tauri-cli
  2.11.4, Windows SDK 10.0.26100, `gh` as `quinnhall07`), plus nothing new. No Deno, no Supabase CLI,
  no Docker — C1/C2's Task 0 installs the first two with winget.
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
  touching `quinn-ops`. Everything else: rule, ledger, continue.

## 3. Sequence

1. ~~Spec review~~ — done; §11a records the rulings since.
2. **C0:** Tasks 1–3 merged. **Next:** branch `c0-release-key` from `main`; Task 4 (P4 + Quinn's
   keygen block), Task 3's first real run needs P1/P2/secrets, Task 5 (first tag `v0.1.0`, the
   updater proof), Task 6 (branch protection, docs close). CLAUDE.md and `app/README.md` already say
   the CI-only truth.
3. ~~Plans C1 and C2~~ — written, reviewed, committed.
4. **Cut day:** the archive step remains (§4). Copy the `f8649d5` build in when Knowlu is next quit.
5. **Execute C1** (branch from `main`; Task 0 installs Deno + Supabase CLI), then **C2**, merging C1
   first; apply H9 between the two merges.
6. C3 (sync; git out of the product) and C4 (remove the local runtime; the (c) toggle UI; the
   settings-panel Google connect/disconnect; the "source went quiet" surface) get plans after C2.

## 4. Quinn's queue — one at a time, when reached, with the context

| When | Ask | Context to give him |
|---|---|---|
| now | **Go to push `main`** | `main` holds the signed spec, the plans, cut-day's app fixes and the C0 merge; pushing makes `ci / test` the gate on every PR and closes PR #1 as merged. Nothing else waits on it except branch protection (C0 Task 6). |
| now | **Archive `quinn-ops`** (cut day step 6) | with him present: remove its worktrees, leave the checkout on `main`, `gh repo archive quinnhall07/quinn-ops`. The old vault survives in that archive's history. Delete `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots,…}` and the old `KnowluBackup\profile_*` mirrors after two clean slots. |
| now | **Run now + the personal calendar** | tray → *Run now* fills the empty page; the secret iCal address (Reset it first) goes into `config/ingest.yaml`'s `calendars:` list by hand until C1's wizard asks. |
| C0 T4 | **Retire the old updater key** (P4) | the exact PowerShell block in the plan's Task 4 generates the new pair, sets the two secrets, prints only the public key; then `cmdkey /delete:knowlu/updater-key{,-password}` and the OneDrive file + its version history. Old key id `C2EC981122E1D2DF`. |
| C0 T3 run | **Azure** (P1) | Trusted Signing account + certificate profile; an Entra app registration with a federated credential of **entity type Environment, value `release`** (subject `repo:quinnhall07/knowlu:environment:release`, issuer `https://token.actions.githubusercontent.com`) — **not** the tag pattern the plan first said; role *Trusted Signing Certificate Profile Signer*; then tenant/client/subscription ids, endpoint, account and profile names (none secret). |
| C0 T3 run | **Cloudflare** (P2, P3) | account, Pages project `knowlu` (direct upload), API token Pages:Edit, account id; `knowlu.com` on Cloudflare DNS → Pages. |
| C0 T3 run | **Set the secrets himself** | `gh secret set NAME --repo quinnhall07/knowlu < file` for `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`; `gh variable set` for `TS_ENDPOINT`, `TS_ACCOUNT`, `TS_PROFILE`. You name them, he sets them. |
| C1 | **Supabase, Stripe, the e-mail provider, Google consent (two scopes), the policy texts** | as the C1 plan's P1–P5 say; the e-mail provider is new (Resend free tier; Supabase Auth's built-in sender is not for production and California's annual reminder needs a sender). |
| C2 | **Anthropic key; Google OAuth client; first test user; the archive read** | as the C2 plan's P0a–P4 say. |
| soon | **Rotate the Blackboard token and the Google Calendar capability URL** | he regenerated neither yet as far as this session knows; both sat in a OneDrive mirror. |
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
