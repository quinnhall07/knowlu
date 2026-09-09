# HANDOFF — where Knowlu stands and how the next session runs it

**Written 2026-09-09 ~05:30 CT by the session that created this repository.** Read this whole file, then
`CLAUDE.md`, then `docs/specs/2026-09-09-knowlu-cloud-design.md` (the direction; still marked DRAFT
for Quinn's review), then `docs/plans/2026-09-09-c0-ci-release-plan.md` (the first plan). Everything
older under `docs/` is history or design authority from the previous repository (`PROVENANCE.md`).

---

## 1. State, in facts

- **This repo:** `quinnhall07/knowlu`, private. `main` = `99784fb`: the orphan import `0967ac2`
  (cut from `quinn-ops` `283d710`), plan C0, and the two cloud docs. One Cargo workspace:
  `engine/` (`knowlu-engine`), `app/` (`knowlu`). Gates at the import: engine **888 passed / 3
  ignored**, app **129 / 1**, fresh clone **1017 / 0 / 4**, 0 warnings; `knowlu-engine.exe`
  4,517,376 bytes, `knowlu.exe` 7,666,176. `.gitattributes`: LF everywhere, `*.ps1` CRLF,
  `engine/tests/fixtures/** -text` (frozen bytes — never re-encode, never regenerate; see `CLAUDE.md`).
- **The old repo `quinn-ops`** (`C:\Users\danie\GitHub\quinn-ops`) is **still live until cut day**:
  it is Quinn's vault; a PowerShell scheduled task `quinn-ops-local-runner` writes it at 12:00 and
  18:00 CT with the old engine; Knowlu build `3f2d438` runs from the Start-menu shortcut against it;
  the Claude cloud routine pushes to it twice a day. **Do not build, run or edit anything there** —
  read its docs if you must (`docs/HANDOFF.md` top block explains the redirection). It becomes an
  archive on cut day.
- **Quinn's decisions of record (2026-09-09)** are the spec's §1, D1–D12. In one line: accounts +
  $9.99/month, no free tier; all judgment in the cloud (Supabase + Cloudflare + Stripe, Anthropic
  API, cheapest model that passes the eval suite); no local model; CI-built releases; the vault is
  wiped and Quinn re-onboards fresh at cut day; Obsidian dropped; Gmail by **OAuth** (Quinn's
  choice, with the verification + CASA track); portal scraping (zyBooks/VHL) stays on the device —
  *fetch on device, think in the cloud*.
- **Cancelled, never resume:** the old plan 2 Tasks 9–10, plans 3b, 3c, 4b; plan 3a's runtime is
  scheduled for removal (C4). The old `local-run.ps1`, the dual-run harness and Python are not in
  this repo by design.
- **Tooling on this laptop:** Rust 1.98 `stable-x86_64-pc-windows-gnu` + WinLibs POSIX MSVCRT mingw;
  `cargo tauri` CLI; Windows SDK 10.0.26100 (`signtool` found by `scripts/find-signtool.ps1`);
  GitHub CLI 2.100.0 **logged in as `quinnhall07`** (`repo`, `workflow` scopes); no Trusted Signing
  profile yet (Azure identity validation started 2026-09-04); the current updater keypair
  (`C2EC981122E1D2DF`) is to be **retired** by C0 Task 4 — its private half once sat in OneDrive.
- **Legal:** `docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` (sourced, not advice). The
  spec's §9 turns it into design consequences and corrects one finding against Google's current pages.

## 2. What gets built, and how it is kept apart

Work runs as **independent streams, one git worktree each, one plan each, disjoint file ownership**.
A stream never edits a file another stream owns; shared files are edited only by the controller at
merge time, in one commit, on `main`. Merge order is fixed so rebases stay trivial: **C0 → C1 → C2**.

| Stream | Plan | Worktree branch | Owns (exclusive) | Needs from Quinn |
|---|---|---|---|---|
| **C0 — CI release** | written: `docs/plans/2026-09-09-c0-ci-release-plan.md` | `c0-ci-release` | `.github/**`, `scripts/ci/**`, `scripts/release.ps1`, `scripts/sign.ps1`, `engine/tests/workflows.rs`; `app/tauri.conf.json` **only** the `plugins.updater.pubkey`, `endpoints` and `version` keys | P1 Azure app registration + federated credential + Trusted Signing names; P2 Cloudflare Pages project + token; P3 `knowlu.com` on Pages; P4 approval to retire the old updater key |
| **C1 — accounts, entitlement, the new wizard, telemetry, issue reports** | **to be written** from spec §4.1–4.2, §5.1, §6 after the spec review | `c1-accounts` | `cloud/supabase/**` (migrations, `entitlement`, `telemetry`, `issues`, `account` functions), `app/src/{onboarding,profiles,scaffold,credentials}.rs`, `app/src/account.rs` (new), `app/static/**`, `app/tests/**` except `workflows`, `site/privacy.html`, `site/terms.html` (new) | a Supabase organisation with **two** projects (prod, staging); a Stripe account (test mode first) with the $9.99 product; the Google Cloud OAuth consent screen (needs the site + privacy policy URL) submitted for restricted-scope verification; the privacy-policy and ToS text reviewed |
| **C2 — the judgment service** | **to be written** from spec §3.2, §5.2–5.4, §4.3 | `c2-judge` | `cloud/supabase/functions/judge-*`, `ingest-*`, `events`, `gmail-*`; `engine/src/{judge,enrich,cloudmodel}.rs`, `engine/src/coursework.rs` (the fetch/parse split), `engine/tests/` new files | an Anthropic API key in the Supabase project's secrets (Quinn sets it; never in this repo); the eval seed read once from the `quinn-ops` archive |
| **Cut day** | spec §7.2 — a procedure, not a plan | none (uses a laptop build of `main`) | nothing in-tree; `HANDOFF.md` afterwards | Quinn at the machine for ~30 minutes |
| **Docs** | — | controller only, on `main` | `VISION.md` (the spec's §10 diff, applied once Quinn approves), `CLAUDE.md`, `README.md`, this file | spec sign-off |

**Shared files with a single owner:** root `Cargo.toml`/`Cargo.lock` (controller; a stream that needs
a dependency says so in its report and the controller adds it on `main` first), `app/src/main.rs`
(controller — the `generate_handler!` lists; a stream adds commands in its own module and the
controller registers them at merge), `app/src/lib.rs` (controller), `app/tauri.conf.json` outside the
three keys above (controller), `engine/src/lib.rs` (controller, one `pub mod` line per new module).

**Controller rules:**
- Use `superpowers:subagent-driven-development` per plan, **one implementer at a time per worktree**;
  reviewers may run in parallel; ledgers live in `.superpowers/sdd/<plan>/progress.md` (git-ignored).
  Streams may run concurrently **because their file sets are disjoint** — verify that with
  `git diff --name-only main...<branch>` before every merge; an overlap is a stop, not a rebase.
- Plans are written before execution (`superpowers:writing-plans`), from the spec, with a fidelity
  ledger against the spec's §1 decisions; C1 and C2 plans can be written in parallel (docs only).
- Every merge to `main` goes through a PR with `ci / test` green once C0 Task 2 exists; before that,
  `cargo test --workspace` locally at 0 warnings. Merge with `--no-ff`, first parent `main`.
- Record every ruling in the stream's ledger; never park the session on a question Quinn can answer
  later — decide, record, continue; **stop only for** an irreversible action, a secret, a spend, a
  push to a shared branch, or anything that touches `quinn-ops`.

## 3. Sequence

1. **Spec review with Quinn** (§1 decisions, §10 VISION diff, §11 recommendations) — walk him
   through it in chat, one section at a time; apply his edits; flip the status line from DRAFT;
   apply the §10 diff to `VISION.md`; rewrite `CLAUDE.md`'s last line into a real "direction" section.
2. **Start C0 now** (Tasks 1–2 need nothing from Quinn; Task 3 waits on P1/P2; Task 4 on P4).
3. **Write plans C1 and C2** (parallel, docs only), review each independently (as plan 3a was),
   commit to `main`.
4. **Cut day** whenever Quinn has 30 minutes at the laptop — it does not wait on C0/C1/C2 and
   nothing waits on it except the archive of `quinn-ops`. Steps: spec §7.2. A judgment gap follows
   until C2's first endpoint; Quinn accepted it.
5. **Execute C1 and C2** in their worktrees, concurrently, merging C1 first.
6. C3 (sync; git out of the product) and C4 (remove the local runtime) get plans after C2 merges.

## 4. Quinn's queue — what needs his opinion, what needs his hands

Ask **one at a time, when reached, with the context** — never a list of chores. In rough order:

| When | Ask | Context to give him |
|---|---|---|
| first | **Spec sign-off**, §1 → §10 → §11 | §11's R3 (summer pause) is the one with real revenue consequence; R2 (7-day trial) changes the wizard; everything else has a safe default |
| first | **Cut day: pick the 30 minutes** | he quits Knowlu, you unregister the task, he disables the routine at claude.ai/code/routines, he runs the wizard into `%USERPROFILE%\Knowlu\`, you repoint the shortcut and archive the old repo; the old vault survives in that archive's history |
| C0 T3 | **Azure** (P1) | the Trusted Signing account + certificate profile he started 2026-09-04; an Entra app registration with a federated credential for `repo:quinnhall07/knowlu:ref:refs/tags/v*`; the role *Trusted Signing Certificate Profile Signer*; then tenant/client/subscription ids, endpoint, account and profile names (none secret) |
| C0 T3 | **Cloudflare** (P2, P3) | an account, a Pages project `knowlu` (direct upload), an API token scoped to Pages:Edit, the account id; `knowlu.com` moved to Cloudflare DNS and attached to Pages — the updater endpoint is baked into the app, so the domain matters early |
| C0 T3 | **Set the secrets himself** | `gh secret set NAME --repo quinnhall07/knowlu < file` — you name them, he sets them; you never see a value |
| C0 T4 | **Retire the old updater key** (P4) | the private half was in a OneDrive file; regeneration costs one `pubkey` line and this laptop's install re-installing once; give him the exact PowerShell block from the plan |
| C1 | **Supabase** | an organisation, projects `knowlu-prod` and `knowlu-staging`, region US; project URL + anon key are public, the service-role key is a secret he sets |
| C1 | **Stripe** | an account; test mode; one product "Knowlu" at $9.99/month; whether to add the $69.99/academic-year price and the summer pause (spec §11 R3); Stripe Tax with Kentucky registration before the first Kentucky sale |
| C1 | **Google OAuth consent screen** | on the existing Cloud project: app name, logo, homepage `knowlu.com`, privacy policy URL, authorised domain, `gmail.readonly` justification and a demo video; submit for restricted-scope verification — weeks; CASA afterwards; until then 100 test users and 7-day tokens |
| C1 | **Privacy policy + ToS text** | drafted by you from the legal note's "cheapest compliant path"; he reads them; a lawyer before the first non-founder paid sign-up (Alabama 18+ attestation and the cancel flow are the two questions that matter most) |
| C2 | **Anthropic API key** | set in Supabase project secrets by him; never in this repo, never in a log |
| soon | **Rotate the Blackboard token and the Google Calendar capability URL** | both sat in a OneDrive mirror on 2026-09-09; under C1 the ICS URL moves server-side anyway — rotate when he sets it there |
| later | The first friend; the lawyer; a co-founder (VISION lists it as unowned) | — |

## 5. Standing rules for this repository (the short list; `CLAUDE.md` has the rest)

- **Never touch `quinn-ops`** except to read docs and, on cut day, to archive it. Never run any
  engine binary with `--vault` pointing at a real vault; tests use `engine/tests/fixtures/` and
  temp copies (`scripts/scratch-vault.ps1 -Source <fixture>`).
- **No secret ever enters this repo, a log, a commit message, a fixture, a prompt or a test name.**
  Secrets are GitHub secrets or Supabase secrets set by Quinn. If a value is ever printed, say so
  immediately and treat it as exposed.
- **0 warnings is part of green**; the one accepted line is the app's `.rsrc merge failure:
  multiple non-default manifests`. The eight frozen references and the three surface references
  are never regenerated (the three: only in a commit whose diff shows why).
- **Releases come only from CI** once C0 lands; until then nothing is released at all.
  `scripts/release.ps1 -DryRun` is the only local use.
- **Desktop safety:** never synthetic keyboard/mouse input; screenshots by window handle
  (`PrintWindow`) only; never launch `knowlu.exe` yourself against a real profile — Quinn does.
- **Commits:** specific `git add`, message via `-F <file>`, trailers `Co-Authored-By: <your model
  name> <noreply@anthropic.com>` and `Claude-Session: <this session's URL>`.
- Line endings are the repo's: LF, `.ps1` CRLF, fixtures bytes. The **engine** still translates CRLF
  on every vault read and write because users' vaults are whatever Windows made them.

## 6. Quick reference

```
cargo test --workspace                       # both crates; ~2 min warm
cargo build --release --workspace            # target\release\knowlu-engine.exe, knowlu.exe
.\scripts\release.ps1 -DryRun                # local bundle, unsigned, publishes nothing (after C0 T3)
.\scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-full   # a throwaway vault to demo against
gh run watch / gh pr create --fill           # CI, once .github/workflows exist
```

App data: `%LOCALAPPDATA%\knowlu\` (`profiles.json`, `profiles\<id>\`). New vaults (spec §4.1):
`%USERPROFILE%\Knowlu\<Profile>\`, backups `%USERPROFILE%\Knowlu\Backups\`.
