# Knowlu C1 — accounts, entitlement, the new wizard, telemetry and issue reports — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: WRITTEN 2026-09-09, not started.** Execute on a branch `c1-accounts` in a worktree of this repository (`knowlu`), merged after C0. Written against `docs/specs/2026-09-09-knowlu-cloud-design.md`, **SIGNED 2026-09-09** — §1 D1–D12 confirmed, §10 applied to `VISION.md`, §11 ruled (R3: keep **both** the $69.99/academic-year price and the June–August pause; R1, R2, R4, R5, R6, R8 yes; R7 later).

**Goal:** Knowlu stops being an app that runs alone. A student creates an account, attests to being 18 or older, accepts the terms, pays $9.99/month after a 7-day trial, connects their LMS calendar in one action, and finishes a wizard that never asks where to put a folder — and from that moment the app knows whether it is entitled, tells the truth in the run list when it is not, sends the two classes of telemetry the terms cover, and can send an issue report the student has read and scrubbed first. **An install that already exists is adopted in place, never re-onboarded:** the founder's cut-day vault and any friend's keep their folder, their settings, their credentials and their profile id, and are walked through the account panels alone on their first launch after C1.

**Architecture:** Three layers, disjoint. **The cloud** is one Supabase codebase deployed to two projects (`knowlu-staging`, `knowlu-prod`): SQL migrations under `cloud/supabase/migrations/` with row-level security on every table, and edge functions under `cloud/supabase/functions/` where each `index.ts` is a thin wire over a pure, `deno test`-able handler module. Stripe is the only writer of `entitlements`, through its webhook, never a client. **The device** gains four new Rust modules (`account.rs`, `lms_link.rs`, `telemetry.rs`, `report.rs`) that own their own Tauri commands, so the only shared Rust file this stream needs at merge is `main.rs`'s two handler lists. **The page** gets a nine-panel wizard, an upgrade overlay for installs that predate the account, an account row in settings, and an issue-report preview screen; it still carries no URL — every endpoint lives in Rust.

**Tech Stack:** Supabase (Postgres 15, Auth/GoTrue, Edge Functions on Deno), Stripe Billing (Checkout, Customer Portal, Tax), `pg_cron` + `pg_net` for the two billing jobs; Rust 1.98 `stable-x86_64-pc-windows-gnu` with `ureq 3.4` — already an `app/` dependency, so **this plan adds no crate** — and Windows Credential Manager through the existing `app/src/credentials.rs` and the engine's `wincred.rs`; plain ES5-flavoured JavaScript in `app/static/console.js` (no framework, no bundler, as today).

**Spec:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (§4.1, §4.2, §5.1, §6, §8, §9, §12's C1 row). Supporting: `docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` (§2 minors, §3 privacy notice and Alabama breach law, §7 analytics consent, §8 subscriptions, §9 issue reports); `CLAUDE.md`; `VISION.md`.

---

## Global Constraints

Every task's requirements implicitly include this section.

- **Add no single-user assumptions.** Anything that would need hand-editing for a second user is a bug. Nothing in this stream names a person's vault, machine, account or credential. (`CLAUDE.md`, rule 1.)
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` and the three Rust-generated `surface-today-*.json` are read, never written, by anything here. (`CLAUDE.md`, rule 2.)
- **No secret in the repo, a log, a fixture, a test name, a commit message or this plan.** The Supabase project URL and anon key are **public** and are compiled into the app; the service-role key, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `SOURCES_ENC_KEY`, `EMAIL_API_KEY` and Google's client secret are set by Quinn in the projects' secret stores. This plan names them and never shows a value. If a value is ever printed, say so immediately and treat it as exposed.
- **Every vault write goes through the engine's `write` with `console_ctx()`** (`actor: "quinn"`, `via: "dashboard"`, and `journal::VIAS` does not grow). No note is ever parsed and re-dumped; `config/cloud.yaml` is written by `scaffold` at vault birth exactly as `config/runners.yaml` is, and by nothing else afterwards.
- **`rank` never calls a model, and nothing under `cli.rs` can reach one.** This stream adds no engine code at all.
- **`cargo build --workspace` and `cargo test --workspace` from the root, at 0 warnings.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple non-default manifests` linker message. Four tests stay `#[ignore]`; none may be un-ignored by changing an assertion.
- **No test reaches the network.** Rust HTTP is tested against a loopback `TcpListener` bound to `127.0.0.1:0` whose serving thread is **joined before the test returns** (the 3a rule). Deno tests are pure modules with no `fetch`, no Docker and no Supabase runtime.
- **`app/static/` carries no `http://` or `https://` literal** — `app/tests/static_assets.rs::no_network_reference_in_the_shipped_page` enforces it. Every endpoint is a Rust constant or a value read from `config/cloud.yaml`.
- **Line endings: LF everywhere** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` CRLF; `engine/tests/fixtures/** -text`). New `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` files are LF. Vault files the app writes go through `knowlu_engine::pystr::write_text`, which is what every other vault file already gets.
- **`cargo test --release` will not link** (`panic = "abort"` in the one release profile). Test in the dev profile.
- **Migrations are numbered `cloud/supabase/migrations/<YYYYMMDDHHMMSS>_<name>.sql` and this stream uses timestamps in 2026-09-10 only** (`20260910…`). C2 uses 2026-09-11. The two streams never collide.
- **Migrations are applied with the Supabase CLI against `knowlu-staging` only.** Production is never touched from a dev machine; `knowlu-prod` is migrated by Quinn, or by CI once C0's release workflow can carry it.
- **Commits:** specific `git add` (never `git add -A`), message via `-F <file>`, trailers:

  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
  ```

- **File ownership is binding.** This stream edits only: `cloud/supabase/**` — except `functions/{judge-*,ingest-*,events,gmail-*}` **and `functions/_shared/judge_*.ts`**, which are C2's (R-X-5; every other file under `_shared/` is C1's) — `app/src/{onboarding,profiles,scaffold,credentials,scheduler}.rs`, the new `app/src/{account,lms_link,report,telemetry}.rs` (the controller extended this stream's ownership with `lms_link.rs` on 2026-09-09), `app/static/**`, `app/tests/**`, `site/**`, and this plan file. Everything else is the controller's and appears under **Controller hand-offs** with exact code. **A task that silently edits one of those files is a plan defect** — stop and report it instead of editing.

---

## Quinn-owned preconditions

Asked **one at a time, when the task reaches them, with the context** — never as a list of chores.

| # | Needed by | What, and what breaks without it |
|---|---|---|
| **P1** | Task 1 | **Supabase.** An organisation with two projects, region **US**: `knowlu-staging` and `knowlu-prod`. What I need (none secret): each project's **reference id**, its **URL** (`https://<ref>.supabase.co`) and its **anon key**. Quinn runs `supabase link` under his own login; no key is typed into this session. **Without it:** Tasks 1–9 can be written and unit-tested (they are pure modules), but nothing can be applied or deployed, and Task 10 has no `api_base` to compile in. The stream cannot reach its exit gate. |
| **P2** | Task 4 | **Stripe, test mode.** An account; one product **"Knowlu"** with two prices — **$9.99/month** and **$69.99 / academic year** (recurring yearly) — and the **Customer Portal** configured with cancellation enabled and no survey gate. Values I need (public): the two price ids and the publishable key. Values Quinn sets as Supabase function secrets, never shown here: `STRIPE_SECRET_KEY`, and `STRIPE_WEBHOOK_SECRET` from the webhook endpoint he points at the staging project's `stripe-webhook` function — **and that endpoint's API version set to `2025-03-31.basil`**, the version `_shared/stripe.ts` pins, because an event delivered under a later version puts `current_period_end` somewhere the handler does not read. Also, **in Stripe's Checkout settings, the terms-of-service URL set to `https://knowlu.com/terms.html`** — Stripe rejects a session carrying `consent_collection[terms_of_service]=required` unless one is configured, so without it every Checkout 400s with a message nobody can act on. (That also sequences Task 19 before Task 4's first live Checkout.) Also **an email provider account** (Resend's free tier is enough), used for **two** things: the annual renewal reminder California requires, and — R-C1-3's second half — **Supabase Auth's custom SMTP**, because the built-in sender is a few messages an hour and is explicitly not for production, and `enable_confirmations = true` means every single sign-up depends on that mail arriving. Values: `EMAIL_API_URL`, `EMAIL_FROM`, and the SMTP host, port and username (all public); `EMAIL_API_KEY` and the SMTP password (secrets Quinn sets). **Without it:** Tasks 4 and 5 cannot be deployed or exercised; the app can create an account but can never become entitled; and the second person to sign up in an hour never receives their confirmation. |
| **P3** | Task 4 step 10 | **Stripe Tax**, enabled, with a **Kentucky** registration recorded before the first Kentucky sale (Kentucky has taxed SaaS at 6% since 2023-01-01), and a written answer from a CPA on Alabama (every tracker reads the Department as *not taxable* for true SaaS; no rule addresses it). **Without it:** Checkout still works, but every Kentucky sale is under-collected tax Knowlu owes out of its own margin, and a UK-campus cohort arrives in one month. This blocks the first live sale, not the code. |
| **P4** | Task 20 | **The Google Cloud OAuth consent screen**, submitted for **restricted-scope verification**. It needs: app name, logo, the homepage `https://knowlu.com`, the privacy-policy URL `https://knowlu.com/privacy.html` (Task 19 publishes it), `knowlu.com` **verified as an authorised domain** (C0's P3 puts it on Cloudflare Pages), a written `gmail.readonly` justification and a demo video. Task 20 hands Quinn the exact justification text and the video shot list; he does the submission. **Without it:** verification never starts, and C2's Gmail reader ships to at most 100 named test users with 7-day refresh tokens for as long as it takes — weeks, then an annual CASA assessment. This is the longest-lead item in the product and it is the whole reason D12's cost lands in C1 rather than C2 (§11 R1). |
| **P5** | Task 19 | **The privacy policy and terms text, read by Quinn** before they are published, and read by **a lawyer before the first non-founder paid sign-up**. The two questions that matter most: the **18+ attestation** as Alabama's § 26-1-1(f) makes it work, and the **cancel flow** against ROSCA, California's ARL as amended by AB 2863, and NY GBL § 527-a. **Without it:** CalOPPA is breached by the first Californian who signs up (no threshold, any size), Google's consent screen has no policy URL to point at (P4 stalls), and the terms the Checkout consent checkbox references do not exist. |

Secrets are set with `supabase secrets set NAME --project-ref <ref>` **from a value Quinn produces**, never typed into this session. The full secret list is Task 4 step 1.

---

## Fidelity ledger

Every decision this plan carries, and the task that carries it.

| # | Decision / ruling | Source | Carried by |
|---|---|---|---|
| D1 | The backend is hosted; releases are CI's | §1 D1 | Nothing here touches `.github/**` (C0's). Task 0 puts the cloud codebase in-tree; hand-off **H7** adds the `deno test` job to `ci.yml` at merge |
| D2 | No free tier. One edition. $9.99/month. An account is required | §1 D2, §5.1 | Tasks 3, 4, 10, 11, 15 — the wizard cannot be finished without a session, and Task 11 makes "not entitled" a named skipped step rather than a failure |
| D3 | All judgment in the cloud; the judge seam stays | §1 D3, §3.2 | Task 11's `JudgePlan`: entitlement outranks everything; a vault with `config/cloud.yaml` **runs** `judge` with `--vault`, `--via` and `--log-dir` and nothing else on the command line (R-X-4), so C2's service is live the day it deploys; the runtime and model gates apply only to a vault that has no account yet, and leave in C4 |
| D4 | Deterministic core on the device; internet-dependent work in the cloud | §1 D4, §3.1 | Tasks 7 (the calendar URLs move server-side while on-device `ingest` keeps running) and 15 (telemetry is derived on-device; only non-content rows leave) |
| D5 | (a) interaction events and (b) corrections under the ToS; (c) behind a separate opt-in; issue reports carry diagnostic context | §1 D5, §6 | Tasks 8, 9, 15, 16. **(c) is not built** — see *What is NOT in this plan*; the `request jsonb` column it would fill is defined in Task 8 and always null in C1 (R-X-3) |
| D6 | Deterministic measures: rules, evals, grammars, pinned models | §1 D6, §5.4 | C2's. Task 8's `corrections` table is the eval suite's raw material and is shaped for it: `(item_id, field, ours, theirs, kind)` |
| D7 | Cloudflare + Supabase, no Firebase | §1 D7 | Task 0's layout is Supabase only. Cloudflare is C0's Pages deploy; Tasks 19 and 20 add pages to `site/` and nothing else |
| D8 | A new repository with fresh history | §1 D8 | Done before this plan (`0967ac2`). Nothing here |
| D9 | The vault is a plain folder the app creates; Obsidian dropped | §1 D9, §4.1 | Task 12: `%USERPROFILE%\Knowlu\<Profile>\` and `%USERPROFILE%\Knowlu\Backups\<Profile>\`, created by the app; Task 17 removes the wizard's folder step |
| D10 | The cancelled plans stay cancelled | §1 D10 | Nothing here resumes any of them |
| D11 | Portal scraping stays on the device | §1 D11, §4.3 | Task 17 keeps the coursework-login panel as it is (Credential Manager, `knowlu/<profile_id>/<source>`); Task 19's privacy policy discloses the ToS tension the legal note names |
| D12 | Gmail is OAuth, `gmail.readonly`, server-side | §1 D12, §5.3 | Task 17's Gmail panel is an honest stub — it says "test users only, re-connect weekly" and calls no endpoint (C2 fills it); Task 20 starts the verification track |
| §10 | VISION: the source of truth is plain text on the machine; the cloud judges, syncs and bills | §10 | Task 12 writes `config/cloud.yaml` beside the other config files — the account is *in* the vault as plain text, and only its secret half is in Credential Manager |
| §10 | VISION: one edition, $9.99/month, an account required, cached on the device with a grace window | §10 | Task 11's 72-hour grace, and the ranking that keeps working through it |
| §10 | VISION: onboarding creates the vault; nobody picks a folder | §10, §11a | Tasks 12 and 17. **Task 18 too**: an install that already has a vault is never asked the question either — it is adopted in place, keeping its folder, settings, credentials and profile id |
| §11a | **The LMS calendar link is captured by a sign-in window, not pasted.** The student signs in on the campus's own page inside a pop-up; the app navigates that window to the calendar's share page and keeps only the link; the window's session data is discarded | spec §11a, ruled 2026-09-09 | Task 13 is the feasibility spike — Quinn present for the SSO login, with a written go/no-go; Task 14 is `lms_link.rs`, the window flow as the primary path and paste-a-link as the fallback for a campus whose pages defeat it; Task 17 is the panel |
| §11a | **Connect your calendars — school and personal, together, before coursework logins and Gmail.** The current wizard never asked for busy time, so `calendars:` stayed empty and the first page showed a day that looked free | spec §11a, ruled 2026-09-09 | Task 17's panel 5 of nine carries both halves: the sign-in window for school, and the **secret iCal address** for the student's own calendar — validated on the device by the same `lms_link::validate`, stored server-side as a `sources` row of kind `calendar_ics` (Tasks 7 and 14), and written into the vault's `calendars:` list by Task 12's `ingest_yaml` so today's engine counts busy time on the very first slot. C2's Google sign-in has a labelled, inert place on the same panel |
| §11a R-OB-1 | **Onboarding maps the coursework sources to courses.** Quinn's first slot: the wizard had stored the zyBooks and VHL logins and written `courses: {}` / `sections: {}`, so `coursework` answered `zybook UACS100Fall2026 not in config; skipped`, `section 2102121 not in config; skipped`, then `0 assignments parsed; treating as failure` | spec §11a, ruled 2026-09-09 | **Task 14a.** After the logins panel stores the credentials the wizard runs the engine's discovery (hand-off **H10**'s `coursework-discover`), shows every book and section with a **suggested** course derived from its code, and writes what the student confirms into `config/ingest.yaml` — `courses:` with `course:`/`label:`, `sections:` for VHL, the `ignore:` entry for zyBooks' own `HowToUseZyBooks2`, and the engine's `categories`/`effort`/`importance` blocks. C2 owns the other half of the ruling (an unknown book on a **later** run is a proposal, not a silent skip) |
| §11a R-OB-2 | **The sign-in window also captures the enrolled course list**, seeding `courses/` and `course_map` — so a first ingest is not 28 tasks with `course: null` | spec §11a, ruled 2026-09-09 | **Task 13** asks it as the spike's second go/no-go question; **Task 14b** captures the list, seeds one `courses/<slug>.md` note per course through the same `write::create` the seed task uses, and writes a `course_map:` line per course; the fallback when the window cannot read them is the same panel with the codes typed in |
| §11a R-OB-3 | A first ingest never creates a task already past due | spec §11a, ruled 2026-09-09 | **Not C1's.** C2's `/ingest-ics`, and the device's `ingest` until then — both are `engine/`, which this stream does not own. Named under *What is NOT in this plan* so nobody reads its absence as an oversight |
| §11a R-OB-4 | **The school is chosen from a searchable list of every US institution**, not from two radios; the chosen school writes `campus:` into the vault, `CAMPUSES` becomes the *curated* layer on top, and the timezone is suggested from the state | spec §11a, ruled 2026-09-09 | **Task 14c**, on hand-off **H11**'s generated `app/campuses.json` (federal IPEDS, public domain, 4,319 schools, ~312 KB, header-stamped, read by `onboarding::campus_search` — the page cannot fetch it and does not hold it). `config/campus.yaml` carries `unitid`, `name`, `state`, `lms`, `curated`; `scaffold::CAMPUSES` is keyed by `UNITID` and adds event feeds and a known LMS; an uncurated school gets no feeds and an LMS from where the sign-in window lands, or a two-button question. Task 17 is the typeahead |
| §11a | **Every wizard finish runs the first slot at once** (§4.2 step 7) | spec §11a, ruled 2026-09-09 | Nothing new: `scheduler::needs_first_run` and `spawn`'s first-run block are on `main` already. Task 17 step 8a **references** them and pins the seam both ways — a wizard-made vault has no `today.md` and is owed a run; an adopted one (Task 18) has one and is not |
| VISION 6 | "Adding a source is one action — a URL or a login — and a source that later breaks fails visibly" | `VISION.md`, success criterion 6 | Tasks 13, 14 and 17: one action is *sign in to your school, the way you always do*. The mechanism sits behind `lms_link::capture()` so the spike's outcome can be swapped without the wizard moving |
| VISION rule | "Never request campus SSO credentials" — **unchanged by §11a** | `VISION.md` standing rules; spec §9 University policies; §11a | Task 14 asks for **nothing**, sees **nothing** and stores **nothing** but the resulting link: the student types their password into the LMS's own page, in a window with no capability grant and its own throwaway data directory that Task 14 deletes. Task 17's static test proves the app's own page has no LMS credential field at all |
| §11a | Cut day proceeds with the current wizard; **existing installs are adopted in place**, never re-onboarded | spec §11a, ruled 2026-09-09 | Task 18. The pre-C1 wizard's defaults — `%USERPROFILE%\Knowlu` and `%USERPROFILE%\Knowlu\Backups`, landing on `main` as `launch_state`'s `default_parent` and `default_backup` before this branch is cut — are where a cut-day vault already sits, so Task 18 moves no path: it adds an account to a vault that is already where C1 would have put it |
| §10 | VISION: telemetry keyed to a pseudonymous account id, aggregates only, never Gmail-derived | §10, §6 | Task 8's cohort rule and Task 15's value allow-list — no title, no course name, ever |
| R1 | Start the Google verification track in C1 | §11 R1 | Task 20 and P4 |
| R2 | A 7-day trial, card up front | §11 R2 | Task 4: `subscription_data[trial_period_days]=7` with `payment_method_collection=always`; Task 17's subscribe panel says both in one sentence |
| R3 | **Keep both** the $69.99/academic-year price and the June–August pause | §11 R3, ruled 2026-09-09 | Task 4 offers both prices at Checkout; Task 5 implements the pause as a `pg_cron`-driven `pause_collection` window and the reminder that must precede the first post-pause charge |
| R4 | Client-side encryption of note bodies in sync | §11 R4 | C3's. Task 7 does encrypt the one thing C1 stores server-side: the LMS capability URL |
| R5 | Global rules reviewed by hand | §11 R5 | C2's |
| R6 | A staging Supabase project and test Stripe from day one | §11 R6 | Task 0's `config.toml`, P1's two projects; every apply and deploy in this plan names `knowlu-staging` |
| R7 | Replace the three fixture vaults with synthetic ones before any public code release | §11 R7, ruled "later" | Not this stream, and not any stream yet: `PROVENANCE.md` carries it, and `engine/tests/fixtures/**` is frozen (`CLAUDE.md` rule 2) so nothing here reads it except as an input. The row exists so the ledger is complete against §11 |
| R8 | Anthropic as the launch inference provider | §11 R8 | C2's. Task 19's privacy policy names "our inference provider" as the only transfer, which is the disclosure Google's Limited Use and the FTC both require |

---

## File structure

**New — cloud**

- `cloud/supabase/config.toml` — the CLI's project config; one `[functions.<name>]` entry per function with its `verify_jwt` setting.
- `cloud/supabase/deno.json` — the import map, lint config and test task for every `.ts` here.
- `cloud/supabase/.gitignore` — `.env*`, `.branches/`, `.temp/`. One responsibility: no secret is ever committed.
- `cloud/supabase/README.md` — how to link, apply and deploy against **staging**, and the sentence that production is never touched from a laptop.
- `cloud/supabase/templates/magic_link.html` — the magic-link mail `[auth.email.template.magic_link]` names: the six-digit code first, the link second (Task 0 step 4).
- `cloud/supabase/migrations/20260910000100_accounts.sql` — `accounts`, `entitlements`, `consents`, `sources`, `webhook_events`; RLS on all five.
- `cloud/supabase/migrations/20260910000200_billing_jobs.sql` — `billing_reminders`, the `billing_subscribers` view, three columns on `entitlements`, the daily `pg_cron` tick.
- `cloud/supabase/migrations/20260910000300_deletion.sql` — `deleted_accounts` and its 90-day sweep.
- `cloud/supabase/migrations/20260910000400_telemetry.sql` — `telemetry_events`, `corrections`, the two minimum-cohort views; RLS.
- `cloud/supabase/migrations/20260910000500_issues.sql` — `issues` and its 90-day sweep; RLS.
- `cloud/supabase/migrations_test.ts` — the SQL invariants (no birthdate, RLS everywhere, no client write path, the filename shape).
- `cloud/supabase/functions/_shared/http.ts` — `json()`, `fail()`, `readJson()`, `methodNotAllowed()`, `subPath()`, `asResponse()`. No I/O.
- `cloud/supabase/functions/_shared/auth.ts` — `parseBearer()`, `requireUser()`; verification is injected.
- `cloud/supabase/functions/_shared/entitlement.ts` — **the C2 contract**: `requireActiveEntitlement(req)`.
- `cloud/supabase/functions/_shared/stripe.ts` — webhook signature verification and form encoding. Pure.
- `cloud/supabase/functions/_shared/crypto.ts` — AES-GCM encrypt/decrypt for the one stored URL.
- `cloud/supabase/functions/_shared/scrub.ts` — the issue-report redaction, twinned by test with the app's Rust `report::scrub`.
- `cloud/supabase/functions/{entitlement,billing-checkout,billing-portal,stripe-webhook,billing-jobs,account,telemetry,issues}/handler.ts` — one pure handler each, all I/O behind an injected `Deps`.
- `cloud/supabase/functions/{…}/index.ts` — one thin wire each: read env, build `Deps`, `Deno.serve`.
- `cloud/supabase/functions/**/*_test.ts` — one `deno test` file per handler and per shared module.

**New — app**

- `app/src/account.rs` — the session in Credential Manager, sign-up / sign-in / magic link (with the emailed code) / refresh, the entitlement cache and its 72-hour grace, the adopt-in-place finisher, the policy opener, and twelve Tauri commands. One responsibility: *who is this install, and is it paid for.*
- `app/src/lms_link.rs` — **capture of the LMS calendar link through a sign-in window** (spec §11a; VISION success criterion 6, "adding a source is one action"): the pop-up webview with its own ephemeral data directory, the campus URLs, the capture, the shape check, the validating fetch, and the paste-a-link fallback. One responsibility: *turn a sign-in the student does themselves into one validated `.ics` URL, holding no credential and keeping no session*, behind an interface small enough that Task 13's spike outcome can be swapped without anything outside this file moving.
- `app/src/telemetry.rs` — (a) from `state/events-ui/`, (b) derived from the journal, the watermark, the POST. One responsibility: *what leaves the device, and never more than that.*
- `app/src/report.rs` — the diagnostic payload, the scrub, the preview text, the POST, two Tauri commands. One responsibility: *what the user reads before they send it.*
- `app/tests/account.rs`, `app/tests/lms_link.rs`, `app/tests/telemetry.rs`, `app/tests/report.rs` — one integration-test crate per module. `app/tests/scaffold.rs` and `app/tests/onboarding.rs` grow the coursework-mapping and course-seed cases (Tasks 14a, 14b).

**Modified — app**

- `app/src/scaffold.rs` — `VaultPlan` gains `account_id`, `personal_calendar`, the coursework mappings and the seeded courses (R-OB-1, R-OB-2); a new `cloud_yaml()`; `build_into` writes `config/cloud.yaml`; a new `write_cloud_yaml_if_absent()` for the vault that already exists (Task 18), and `CAMPUSES` grows a third column — the campus's own calendar-settings URL.
- `app/src/onboarding.rs` — `knowlu_root()`, `vault_dest()`, `backups_dir()`; `WizardPlan` loses the folder fields and gains the account ones; `create_vault` takes a name, not a parent.
- `app/src/scheduler.rs` — `entitlement_state()`, the judge gate that reads it, and the telemetry step.
- `app/src/profiles.rs`, `app/src/credentials.rs` — **not modified.** They are inside this stream's ownership so that a needed change would not be a hand-off, but nothing here needs one: `profiles::id_for` and `credentials::target_for` already produce exactly the ids and target names the cloud contract asks for.
- `app/static/index.html`, `app/static/console.js`, `app/static/console.css` — the nine-panel wizard, the settings account row, the issue-report preview overlay.
- `app/campuses.json` — **new, generated and committed by hand-off H11**, and therefore the controller's file, not this stream's: every active two- and four-year US institution, `[unitid, name, city, state, host]`, ~400 KB, header line naming its IPEDS source and date. It sits beside `tauri.conf.json` and **not** under `app/static/`, because the page never loads it — `scaffold`/`onboarding` `include_str!` it and `campus_search` answers from it — and a 400 KB file in `frontendDist` would ship twice, once in the bundle and once in the binary. C1 reads it and tests it.
- `app/tests/{static_assets,onboarding,scaffold,scheduler}.rs` — the pins follow the code.

**Modified — site**

- `site/privacy.html` — full rewrite (the prose exception, Task 19).
- `site/terms.html` — new (the prose exception, Task 19).
- `site/subscribed.html` — new (Task 19). Where Stripe Checkout's `success_url` lands. One paragraph: you are subscribed, go back to Knowlu.
- `site/signed-in.html` — new (Task 19). Where Auth's confirmation and magic links land (`config.toml`'s `additional_redirect_urls`). One paragraph: your email is confirmed, go back to Knowlu and sign in — **and, for a magic link, the six-digit code the app asks for.**
- `site/index.html` — the price, the account and the trial in the copy; a Terms link in the footer (Task 20).
- `site/site.css` — whatever the four new pages need, and nothing more.

---

## Interfaces with C2

Agreed with the C2 plan, written in parallel. Use them verbatim; changing one is a conversation, not an edit.

1. **`config/cloud.yaml`** — written by C1's wizard through `scaffold::create_vault` at onboarding, read by C2's engine code. Exactly four keys, single-line YAML scalars, in this order:

   ```yaml
   api_base: 'https://<ref>.supabase.co/functions/v1'
   anon_key: '<the project anon key — public>'
   session_credential_target: 'knowlu/<profile_id>/session'
   account_id: '<uuid>'
   ```

   Written by `scaffold::write_file` → `knowlu_engine::pystr::write_text`, i.e. **the same line endings every other file in the vault gets** (today CRLF, because vaults are). The contract's "LF" means *no multi-line values, no block scalars, no anchors* — it is not a byte guarantee. **C2 must read it through `pystr::read_text` + `serde_yaml_ng` and never compare raw bytes**, exactly as `scheduler::ics_state` reads `config/ingest.yaml` today.

2. **The session JWT** lives in Windows Credential Manager at `knowlu/<profile_id>/session` (spec §5.1). `UserName` is the `account_id`; the blob is one JSON object `{"access_token","refresh_token","expires_at","email"}`, `expires_at` in Unix seconds. **Refresh is C1's job** — `account::valid_access_token()` refreshes when fewer than 120 seconds remain and rewrites the entry. **C2 only reads it**, through `knowlu_engine::wincred::read_credential`, and tolerates an expired `access_token` by re-reading after the app's next slot rather than refreshing itself.

3. **Calendars: three kinds, two writers, and every calendar URL stored twice on purpose** (R-X-1, and spec §11a).

   | `sources.kind` | Written by | Read by | Where it also lives on the device |
   |---|---|---|---|
   | `lms_ics` | **C1** — `lms_link::finish` the moment the school feed validates (Task 14), and Task 18's back-fill for a vault adopted in place | **C2** — `/ingest-ics` | `config/ingest.yaml`'s `ics_url:` |
   | `calendar_ics` | **C1** — the same panel's personal-calendar field, by its secret iCal address (Task 14); the back-fill covers it too | **C2** — `/ingest-calendar` | `config/ingest.yaml`'s `calendars:` list, as `- name: personal` / `ics_url:` |
   | `google_calendar` | **nobody** — a reserved value (R-X-9): a Google grant has no URL for `url_ciphertext`/`url_iv`, so it lives in C2's `google_accounts`, never in this table | **C2** — `/ingest-calendar` resolves `name=google` from `google_accounts`, not from a row here | nowhere: C1's device never writes or reads this kind |

   **All three are already in the check constraint and in both `SOURCE_KINDS` lists** (`20260910000100_accounts.sql`, `functions/account/handler.ts`, `app/src/lms_link.rs`), so **C2 never has to edit a C1-owned file to add its kind** — the ownership list forbids it, and a check constraint is the one thing two streams cannot both edit safely. C1's device-side guard is a second, narrower list, `lms_link::DEVICE_KINDS = ["lms_ics", "calendar_ics"]`, and `validate_for` refuses anything else, so no page can invent a `google_calendar` row. One Rust test (`the_source_kind_vocabulary_is_one_list_in_three_places`) pins the three copies to each other.

   The vault copy exists because on-device `ingest` keeps running until C2 delivers its readers (spec §7.2's judgment gap). When `/ingest-ics` and `/ingest-calendar` ship, the vault copies **stay** as the offline fallback that keeps `ingest` at exit 0 (R-X-16); C3 or C4 removes them, not C1 and not C2.

   **C1 defines no Google connect command, and no Google code at all.** The calendars panel carries a labelled, **`disabled`** placeholder and nothing behind it; a static test asserts `console.js` contains neither `"connect_google"` nor `gmail.readonly`. **C2's hand-off H9** carries the button's enablement, its click listener, the three vault-less wizard-window commands `google_connect_url` / `google_connected` / `open_external`, and the `WizardPlan.google_calendar` flag that `scaffold::ingest_yaml` turns into the `- name: google` entry at Finish, into `app/` at C2's merge (R-X-15). Its hook, verbatim, is the element C1 leaves for it:

   ```html
   <div class="wiz-row" id="wiz-google-row"><button class="b" id="wiz-google" disabled>Sign in with Google instead</button><span class="meta">…</span></div>
   ```

   — `#wiz-google` inside `#wiz-google-row`, inside the panel `#wiz-calendars`, in `app/static/index.html`. C2 removes the `disabled` attribute and the "next release" span, and adds its listener beside the wizard's existing ones; it changes no other element on that panel.

   **The `sources` table is C1's, and its schema is the authority:**

   ```
   sources(account_id uuid, kind text, url_ciphertext text, url_iv text, added_at timestamptz)
   primary key (account_id, kind)
   ```

   There is **no column called `url`**. The URL is AES-256-GCM encrypted before it reaches Postgres,
   with the key in the function environment as `SOURCES_ENC_KEY` and never in the database. **C2 reads
   it by importing C1's `cloud/supabase/functions/_shared/crypto.ts`** — `importAesKey(Deno.env.get("SOURCES_ENC_KEY"))`
   then `decryptString(key, row.url_ciphertext, row.url_iv)` — and needs the same secret set on the
   same project. `GET /account/sources` still returns `[{"kind","added_at"}]` and never the URL.

4. **The `judge` step's argv** (R-X-4). When the vault has `config/cloud.yaml` the slot runs exactly

   ```
   knowlu-engine judge --vault <vault> --via local-runner --log-dir <the profile's logs folder>
   ```

   and **nothing else** — no `--runtime`, no `--model`, no account id, no token. `--log-dir` is in
   **both** arms because judgment logs never enter the vault (`CLAUDE.md`) and only the app knows the
   profile's log folder; it is `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments`, the same directory
   `inference::judgments_dir(&cs.data_dir)` already returns. C2's engine reads `config/cloud.yaml`
   itself and reads the session out of Credential Manager at the target that file names. The local
   `--runtime`/`--model` form survives only for a vault that has no `config/cloud.yaml` (a pre-C1
   install, until Task 18 adopts it) and leaves entirely in C4.

   **Entitlement outranks everything**: past the 72-hour grace the step is not run at all and the slot
   records `judge (skipped: no entitlement)` with exit code 0. C2 must expect that skip in any test
   that runs a slot against a cloud vault with a stale or absent `entitlement.json`.

   **The types are C1's** (R-X-4): `enum JudgePlan { Cloud { log_dir }, Local(JudgeArgs), Skip(&'static str) }`
   and `slot_argv(vault, exe, &JudgePlan)` in `app/src/scheduler.rs`. C2 deletes its own H6 rewrite and
   states the dependency instead. The `ingest` gate stays `has_ics_url(vault)` here; C2's Task 8
   hand-off adds `|| cloud` **against C1's merged shape**, not against a replacement of it.

5. **Entitlement.** `GET /entitlement` replies

   ```json
   {"status":"active|trialing|past_due|canceled|none","current_period_end":"<RFC3339|null>","plan":"<string|null>","checked_at":"<RFC3339>"}
   ```

   C1 caches it on the device with a **72-hour grace**, and the scheduler emits `judge (skipped: no entitlement)` — the same shape for every future cloud step — as a **named skipped step with exit code 0**, never a failed one.

6. **`cloud/supabase/functions/_shared/entitlement.ts` is written by C1 and imported by C2.** Its contract:

   ```ts
   export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }>;
   ```

   On success it resolves to the account id. On any failure it **throws a `Response`** — 401 when the bearer token is missing or invalid, **402** when the account has no `active` or `trialing` entitlement. C2's handlers wrap it as `try { … } catch (e) { if (e instanceof Response) return e; throw e; }`.

7. **Telemetry, and who owns `corrections`** (R-X-2, R-X-3). (a) interaction events and (b) corrections go to C1's `POST /telemetry`; (b) is derived from the journal **on the device** by `telemetry.rs` (spec §6). C2's judgment rows are server-side and are not C1's; C2 never posts to `/telemetry`.

   **`public.corrections` is created by C1**, in `20260910000400_telemetry.sql`, with C1's columns:
   `id, account_id, ts, item_id, field, ours, theirs, kind, request, received_at`. **`ours` and
   `theirs` are nullable and must stay nullable** — a `course` correction deliberately carries neither
   (§6's content rule). **`kind` is the note kind** (`task | approval | course | info | issue |
   archive`) and carries **no check constraint**, because C1 derives it from the note's folder.

   **C2 never creates this table.** In its own `20260911…` migration C2 adds
   `alter table public.corrections add column if not exists judgment_id uuid null`,
   `add column if not exists judgment_kind text null check (judgment_kind in ('task','event','email'))`,
   and its index on `(judgment_kind, ts desc)` — never on `corrected_at`, which does not exist here.

   **`request jsonb` is defined by C1 and always null in C1** (R-X-3, from the C2 review's R-C2-4): the
   column exists so C2's eval suite can replay a correction as a labelled example, and a row may carry
   it **only** under the class-(c) opt-in — which this plan does not build — and **never** when the
   judgment it corrects came from Gmail (`origin = gmail_api`). C1's `POST /telemetry` accepts the
   field, refuses it unless the request also carries `opt_in_raw: true`, and writes `null` otherwise.

8. **`config.toml` and `deno.json` are C1's** (R-X-6). Every function needs a `[functions.<name>]`
   entry with `verify_jwt = false` — the handlers answer 401/402 in *our* shape, which is the contract
   above — and every third-party import needs a `deno.json` import-map line. C2 supplies both as a
   **controller hand-off applied at C2's merge**: the exact `[functions.<name>] verify_jwt = false`
   block for its eleven functions, and its import-map lines with `@anthropic-ai/sdk` pinned to an exact
   version. C1 does not guess at them.

9. **One `cloud` CI job** (R-X-7). C1's definition — `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
   — is the job (hand-off H7). C2's hand-off **amends** that job (widening the path scope, and adding
   `--allow-net=127.0.0.1` only if its loopback tests need it) and adds `eval-gate` as a second job.
   The controller merges both into `ci.yml`; neither stream adds a second `cloud` job.

10. **Migration numbering.** C1 uses `20260910……`, C2 uses `20260911……`, and **C1's numbering test
    scopes itself to C1's own files** (R-X-8) so C2's migrations landing in the same directory cannot
    turn C1's suite red.

---

## Controller hand-offs

Files this stream may not edit. **H1 is pre-flight** — the controller applies it on `main` **before** C1 execution starts, because it is compile-blocking. Everything from H3 down is applied at merge, in one commit on `main`, keyed to the task that needs it — **except the four mid-stream applications R-C1-11 orders: H9a, H10, H9b and H11**, each applied on the branch's base at the step that names it (Task 13 step 3, Task 14a step 6, Task 14b step 3a, Task 14c step 1). With H1 that is five, and H9's block lists them in order.

### H1 (pre-flight, before Task 10) — `app/src/lib.rs` and four empty modules

`app/src/lib.rs` is ten lines of `pub mod`. Add four, alphabetically placed:

```rust
pub mod account;
pub mod lms_link;
pub mod report;
pub mod telemetry;
```

…and, in the same commit, the four files they name — `app/src/account.rs`, `app/src/lms_link.rs`, `app/src/report.rs`, `app/src/telemetry.rs` — each exactly one doc comment and nothing else. They compile clean and warn about nothing, and C1's tasks fill them:

```rust
//! The account: session, sign-in, entitlement (Knowlu C1, Tasks 10, 11 and 18).
```

```rust
//! The LMS calendar link, captured through a sign-in window (Knowlu C1, Tasks 13 and 14).
```

```rust
//! Issue reports: the payload, the scrub, the preview (Knowlu C1, Task 16).
```

```rust
//! Telemetry (a) and (b), derived on the device (Knowlu C1, Task 15).
```

### H2 (recorded, no edit needed) — why the entitlement cache is not a `Settings` field

`app/src/state.rs` is the controller's, and `state::Settings` derives `Deserialize` with **no `#[serde(default)]` on any field**, so a new field would make every existing `settings.json` fail to parse — and `Settings::load` falls back to defaults on a parse failure, silently resetting a user's backup folder and autostart choice. That is the hazard `onboarding.rs::offer_marker` already documents in prose. C1 therefore caches the entitlement in `%LOCALAPPDATA%\knowlu\profiles\<id>\entitlement.json`, a sibling of `settings.json` in the same per-profile folder, following the `offer-inference` marker's precedent. **No hand-off is needed and `state.rs` is not touched.** This is the one place this plan reads the spec's §5.1 phrase "cached in `settings.json`" as naming the *location* rather than the file.

### H3 (Tasks 10, 11, 14, 17, 18) — `app/src/main.rs`, the two `generate_handler!` lists

The vault-less shell's list (11 today) gains sixteen, so **27**:

```rust
.invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::retarget_credentials, onboarding::finish_onboarding, account::sign_up, account::sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::entitlement_now, account::open_checkout, account::open_policy, lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::capture_courses, lms_link::paste_calendar_link, lms_link::close_lms_window, onboarding::discover_coursework, onboarding::timezone_for_state, onboarding::campus_search])
```

The console window's list (29 today) gains fourteen, so **43**:

```rust
.invoke_handler(tauri::generate_handler![commands::state, commands::note, commands::mark_seen, commands::ui_event, commands::set_fields, commands::create_task, commands::delete_note, commands::decide, commands::close_info, commands::open_issue, commands::resolve_issue, commands::sync, commands::backup_now, commands::get_settings, commands::set_settings, commands::set_profile_name, commands::copy_diagnostics, commands::copy_text, commands::settings_context, commands::switch_profile, commands::check_for_updates, commands::install_update, commands::inference_status, commands::install_inference_file, commands::install_inference_download, commands::remove_inference_model, onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, account::sign_up, account::sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::entitlement_now, account::open_checkout, account::open_policy, account::account_status, account::open_portal, account::attach_account, account::delete_my_data, report::report_preview, report::report_send])
```

The eight sign-in commands are in **both** lists on purpose: Task 18's upgrade overlay runs inside the console window, over an existing vault, and needs exactly the same eight. `lms_link`'s five and `onboarding`'s three new ones (`discover_coursework`, `timezone_for_state`, `campus_search`) are the shell's alone — an install that already exists already has its feed, its mapping and its school.

…the `use` line at the top of `main.rs` gains the three modules:

```rust
use knowlu::{account, commands, lms_link, onboarding, profiles, report, scheduler, state::{app_data_root, resolve_vault, ConsoleState}, tray};
```

New totals to quote afterwards: **27 + 43, 59 distinct** (`launch_state`, `pick_folder`, `pick_file` and the eight sign-in commands are in both lists). **Nine** commands now mutate the vault: the existing seven, plus `account::attach_account` (which writes `config/cloud.yaml` into a vault that already exists) and `account::delete_my_data` (which removes the vault, its backups folder for this profile, and the account). The other nine new ones touch app data, the cloud, a second window or the system browser — never a note.

**`account::open_policy` exists because a link cannot.** The wizard's account panel and the upgrade overlay both ask the user to accept the terms and the privacy policy, and both link them. `app/static/` holds four files — `index.html`, `console.css`, `console.js`, `fonts/` — so a plain `<a href="terms.html">` navigates the one webview to a missing asset and the window is lost until restart. The `href` stays (the static test reads it, and it is the honest markup), the click handler calls `preventDefault()` and invokes `open_policy(which)`, and `open_policy` opens `https://knowlu.com/<which>.html` in the system browser through the same `explorer.exe` call *Open vault folder* uses.

**No plugin is added.** The sign-in window is a second `WebviewWindow` that `lms_link.rs` builds with its own throwaway data directory; it needs no plugin, and it is given **no capability grant**, so the campus's own page cannot reach a single Tauri command.

### H4 (Tasks 12 and 16) — `app/src/tray.rs`, two menu items

`build()` makes seven items today. Add two, and widen the `Menu::with_items` call:

```rust
    let vaultdir = MenuItem::with_id(app, "vault-folder", "Open vault folder", true, None::<&str>)?;
    let report = MenuItem::with_id(app, "report", "Report an issue", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &run, &pause, &vaultdir, &settings, &update, &diag, &report, &quit])?;
```

…and two arms in `on_menu_event`, before the `_ => {}`:

```rust
            "vault-folder" => {
                // Spec §4.1: the vault is visible, and the tray points at it. `explorer.exe` with a
                // path is the whole of it — no shell verb, no `cmd`, and no console window.
                let cs = app.state::<ConsoleState>();
                use knowlu_engine::childproc::NoConsole;
                let _ = std::process::Command::new("explorer.exe").no_console().arg(cs.vault.as_os_str()).spawn();
            }
            "report" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    // One way into the preview screen from both doors — the same shape the
                    // `settings` arm uses, so the tray still needs no `core:event` grant.
                    let _ = w.eval("window.KNOWLU_OPEN_REPORT && window.KNOWLU_OPEN_REPORT()");
                }
            }
```

*Delete my data* is deliberately **not** a tray item: it destroys the vault, and a tray menu has no room to confirm. It is a settings row that calls `account::delete_my_data` after the page's two-step confirmation (Task 17).

### H5 (Task 21) — `app/README.md`, the recount

Replace the "Thirty-two `#[tauri::command]`s exist" paragraph and its two bullets with:

```markdown
The engine is linked as a path dependency (`knowlu-engine = { path = "../engine" }`). **Fifty-nine**
`#[tauri::command]`s exist — **twenty-six in `src/commands.rs`, fourteen in `src/onboarding.rs`, twelve
in `src/account.rs`, five in `src/lms_link.rs` and two in `src/report.rs`**. Commands live beside the
module they serve, never all in one file. Count them in `src/main.rs`'s two `generate_handler!` lists
if this drifts, and note that the two lists are different windows, not one:

- **The console window** registers 43 — all 26 of `commands.rs`; `launch_state`, `pick_folder` and
  `pick_file` from `onboarding.rs`; all 12 of `account.rs` (the eight sign-in commands included,
  because an install that predates the account is upgraded in place, inside this window, over its own
  vault); and both of `report.rs`.
- **The vault-less shell** (picker or wizard) registers 27 — the 14 of `onboarding.rs`, eight of
  `account.rs` (`sign_up`, `sign_in`, `send_magic_link`, `verify_email_code`, `sign_out`,
  `entitlement_now`, `open_checkout`, `open_policy`) and all five of `lms_link.rs`: there is no
  `ConsoleState` yet, so no command that needs one can be called.
```

### H6 (Task 17) — `scripts/wizard-check.py`, the headless wizard walk

The wizard goes from seven panels to nine and its first two become an account and a subscription, so this script's `check()` no longer describes anything real. **The replacement is given here in full**, not generated at execution time: a hand-off whose payload does not exist yet cannot be reviewed and cannot be applied by a controller who was not in the room. Replace three regions of `scripts/wizard-check.py` — the three constants, the `FAKE` recorder, and `check()` — with exactly this. Everything else in the file (the `Quiet` handler, `names`, `first_args`, `main`) is unchanged.

**Acceptance:** `.wv\Scripts\python scripts/wizard-check.py` prints one word, `ok`, and exits 0 against the nine-panel wizard.

The three constants near the top become — and `PARENT` goes with them, unused once the paths are
spelled out:

```python
DEST_OLD = "C:\\Users\\Ada\\Knowlu\\Fall 2026"
DEST_NEW = "C:\\Users\\Ada\\Knowlu\\Spring 2027"
# Decision 3, restated for C1: until Finish is clicked, the only commands the wizard may have called
# are the ones that READ, the ones that make the ACCOUNT (which is not this machine's disk), the two
# that write a credential, and the sign-in window's own three. Anything else here would mean
# something reached this machine's disk before the user said go.
BEFORE_FINISH_OK = {"launch_state", "pick_folder", "sign_up", "sign_in", "send_magic_link",
                    "verify_email_code", "entitlement_now", "open_checkout", "open_policy",
                    "open_lms_window", "capture_calendar_link", "capture_courses",
                    "paste_calendar_link", "close_lms_window", "discover_coursework",
                    "campus_search", "timezone_for_state",
                    "store_credentials", "retarget_credentials"}
```

`FAKE`'s invoke recorder gains the C1 commands (the `launch_state` reply carries the renamed folder keys):

```javascript
window.__TAURI__ = { core: { invoke: function (cmd, args) {
  window.__CALLS.push([cmd, args]);
  if (cmd === 'launch_state') { return Promise.resolve({ ok: true, mode: 'wizard', profiles: [], machine: 'M',
      tz: 'America/Chicago', default_parent: 'C:\\Users\\Ada\\Knowlu',
      default_backup: 'C:\\Users\\Ada\\Knowlu\\Backups',
      }); }
  if (cmd === 'sign_up' || cmd === 'sign_in') {
    return (args.email || '').indexOf('fail') === 0
      ? Promise.resolve({ ok: false, error: 'Invalid login credentials', account_id: null })
      : Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: args.email }); }
  if (cmd === 'send_magic_link') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'verify_email_code') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
  if (cmd === 'open_checkout') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'entitlement_now') { return Promise.resolve({ ok: true, error: null, status: 'trialing', plan: 'monthly', current_period_end: null }); }
  if (cmd === 'open_policy') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'open_lms_window') { return Promise.resolve({ ok: true, error: null, opened: true, session_dir: 'T:\\tmp\\lms' }); }
  if (cmd === 'timezone_for_state') { return Promise.resolve({ ok: true, error: null, timezone: 'America/Chicago' }); }
  if (cmd === 'campus_search') {
    return Promise.resolve({ ok: true, error: null,
      hits: [[100751, 'The University of Alabama', 'Tuscaloosa', 'AL']] }); }
  if (cmd === 'capture_calendar_link') {
    return Promise.resolve({ ok: true, error: null, link: { url: 'https://lms.example.invalid/feed/a.ics', events: 12, courses: 4 } }); }
  if (cmd === 'paste_calendar_link') {
    return Promise.resolve({ ok: true, error: null, link: { url: 'https://lms.example.invalid/feed/a.ics', events: 12, courses: 4 } }); }
  if (cmd === 'close_lms_window') { return Promise.resolve({ ok: true, error: null }); }
  if (cmd === 'capture_courses') {
    return Promise.resolve({ ok: true, error: null, typed: false,
      courses: [{ code: 'UACS100Fall2026', name: 'CS 100 Intro', slug: 'cs-100' }] }); }
  if (cmd === 'discover_coursework') {
    return Promise.resolve({ ok: true, error: null, note: null, rows: [
      { source: 'zybooks', key: 'UACS100Fall2026', detail: null, suggested: 'CS 100', mapped: false, ignored: false },
      { source: 'vhl', key: '2102121', detail: 'course 1623220', suggested: null, mapped: false, ignored: false }] }); }
  if (cmd === 'store_credentials') {
    return (args.user || '').indexOf('fail') === 0
      ? Promise.resolve({ ok: false, error: 'credential write failed for ' + args.source })
      : Promise.resolve({ ok: true, error: null, target: 'knowlu/p/' + args.source }); }
  if (cmd === 'retarget_credentials') { return Promise.resolve({ ok: true, error: null, moved: 1 }); }
  if (cmd === 'create_vault') {
    return Promise.resolve({ ok: true, error: null, profile: { id: 'p1', name: args.name, vault: 'C:\\Users\\Ada\\Knowlu\\' + args.name } }); }
  if (cmd === 'account_status') { return Promise.resolve({ ok: true, needs_account: false }); }
  if (cmd === 'get_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: null, autostart: true, quit_at: null } }); }
  if (cmd === 'settings_context') { return Promise.resolve({ ok: true, vault: 'C:\\v', version: '0.1.0', profile_name: 'Ada' }); }
  if (cmd === 'set_settings') { return Promise.resolve({ ok: true, settings: { profile_id: 'p1', backup_dir: 'C:\\b', autostart: true, quit_at: null } }); }
  return Promise.resolve({ ok: true, error: null });
} } };
window.__CALLS = [];
```

`check()` becomes, in full:

```python
def check(page) -> list:
    bad = []
    # 1. The wizard is what a zero-profile launch shows, on panel 1 of nine.
    if page.is_hidden("#wizard") or page.is_hidden("#wiz-welcome"): bad.append("wizard did not open on panel 1")
    if "of 9" not in page.inner_text("#wiz-step"): bad.append(f"step counter says {page.inner_text('#wiz-step')!r}")

    # 2. Panel 2 is the account, and it refuses to make one until BOTH boxes are ticked (spec §9).
    page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-account"): bad.append("Next did not reach the account panel")
    page.fill("#wiz-email", "a@example.invalid"); page.fill("#wiz-pw", "not-a-real-password")
    page.click("#wiz-create"); page.wait_for_timeout(200)
    if "sign_up" in names(page): bad.append("an account was created with the boxes unticked")
    if "Tick both" not in page.inner_text("#wiz-error"): bad.append("the refusal said nothing about the boxes")
    page.check("#wiz-18"); page.check("#wiz-terms")
    page.click("#wiz-create"); page.wait_for_timeout(300)
    if "sign_up" not in names(page): bad.append("sign_up was not invoked")
    if page.input_value("#wiz-pw") != "": bad.append("the password field was not cleared")
    su = first_args(page, "sign_up") or {}
    if su.get("age_attested") is not True: bad.append("sign_up did not carry the attestation")
    if page.is_hidden("#wiz-subscribe"): bad.append("a created account did not advance to the subscribe panel")

    # 3. Subscribe opens Checkout in the system browser and polls until the account is entitled.
    page.click("#wiz-sub-month"); page.wait_for_timeout(3600)
    if "open_checkout" not in names(page): bad.append("open_checkout was not invoked")
    if (first_args(page, "open_checkout") or {}).get("plan") != "monthly": bad.append("open_checkout named the wrong plan")
    if "entitlement_now" not in names(page): bad.append("the wizard did not poll for the subscription")
    if page.is_hidden("#wiz-vault"): bad.append("an entitled account did not advance to the name panel")

    # 4. Panel 4 names the setup. NO folder is picked, and the path is shown before Finish.
    page.fill("#wiz-name", "Fall 2026"); page.wait_for_timeout(120)
    if DEST_OLD not in page.inner_text("#wiz-vault-path"): bad.append("the resulting path is not shown")
    if page.query_selector("#wiz-pick-parent") or page.query_selector("#wiz-pick-bdir"):
        bad.append("the wizard still offers a folder picker")

    # 5. The calendars panel: BOTH calendars, before logins and Gmail (spec §11a).
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-calendars"): bad.append("Next did not reach the calendars panel")
    if not page.query_selector("#wiz-calendars #wiz-school"): bad.append("the school search is not on the calendars panel")
    # R-OB-4: the school is typed, not picked off two radios. Three letters, one hit, one click.
    page.fill("#wiz-school", "alabama"); page.wait_for_timeout(250)
    hits = page.query_selector_all("#wiz-school-hits [data-school]")
    if not hits: bad.append("typing a school name found nothing in campuses.json")
    else: hits[0].click()
    page.wait_for_timeout(200)
    if "Alabama" not in page.input_value("#wiz-school"): bad.append("picking a school did not fill the field")
    page.click("#wiz-lms-open"); page.wait_for_timeout(400)
    if (first_args(page, "open_lms_window") or {}).get("unitid") != "100751":
        bad.append("the sign-in window was opened for the wrong school")
    if "12 assignments" not in page.inner_text("#wiz-lms-state"): bad.append("the captured feed was not summarised")
    # …and the personal one, by its secret address, under its own kind.
    page.fill("#wiz-cal-ics", "https://calendar.google.com/calendar/ical/x/private-def/basic.ics")
    page.dispatch_event("#wiz-cal-ics", "change"); page.wait_for_timeout(250)
    paste = [c[1] for c in page.evaluate("window.__CALLS") if c[0] == "paste_calendar_link"]
    if not any((a or {}).get("kind") == "calendar_ics" for a in paste):
        bad.append("the personal calendar was not validated under kind calendar_ics")
    if "already on your calendar" not in page.inner_text("#wiz-cal-note"):
        bad.append("the personal calendar was not summarised")
    if not page.query_selector("#wiz-google[disabled]"):
        bad.append("the Google placeholder must be present and inert in C1")
    # R-OB-2: the captured class list is on this panel, with a typed fallback beside it.
    if page.is_hidden("#wiz-courses"): bad.append("the class list did not appear after the sign-in")
    if "CS 100" not in page.inner_text("#wiz-course-rows"): bad.append("the captured course is not listed")
    page.fill("#wiz-course-add", "GN 103"); page.click("#wiz-course-add-go"); page.wait_for_timeout(120)
    if "GN 103" not in page.inner_text("#wiz-course-rows"): bad.append("a typed course was not added")

    # 6. Coursework logins: half a pair is a typo, not a choice; a failed write keeps the user here.
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-logins"): bad.append("Next did not reach the credentials panel")
    if "close_lms_window" not in names(page): bad.append("the sign-in window was not closed on leaving")
    page.fill("#wiz-zy-user", "a@example.invalid"); page.wait_for_timeout(60)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-logins"): bad.append("a half-filled login pair did not block Next")
    if "both" not in page.inner_text("#wiz-error"): bad.append("a half-filled pair said nothing about needing both")
    secret = secrets.token_urlsafe(12)
    page.fill("#wiz-zy-user", "fail@example.invalid"); page.fill("#wiz-zy-pass", secret)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-logins"): bad.append("a failed credential write advanced anyway")
    if page.input_value("#wiz-zy-pass") == "": bad.append("a failed write cleared the fields the user must retype")
    page.fill("#wiz-zy-user", "a@example.invalid")
    # R-OB-1: the first Next after a successful store runs discovery and STAYS on the panel with the
    # rows; the second one moves on. A wizard that took the password and skipped the mapping is the
    # run this exists because of.
    page.click("#wiz-next"); page.wait_for_timeout(400)
    if page.is_hidden("#wiz-logins"): bad.append("the mapping step was skipped after the credentials were stored")
    if "discover_coursework" not in names(page): bad.append("discovery did not run after the credentials were stored")
    if page.is_hidden("#wiz-map"): bad.append("the mapping rows did not appear")
    rows = page.inner_text("#wiz-map-rows")
    if "UACS100Fall2026" not in rows or "2102121" not in rows: bad.append(f"the discovered sources are not listed: {rows!r}")
    if page.input_value('[data-course-for="0"]') != "CS 100": bad.append("the suggestion was not pre-filled")
    page.fill('[data-course-for="1"]', "GN 103"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(300)
    if page.is_hidden("#wiz-gmail"): bad.append("a confirmed mapping did not advance to the Gmail panel")
    if page.input_value("#wiz-zy-pass") != "": bad.append("the password field was not cleared")
    cred_vault = page.evaluate("(window.__CALLS.filter(c => c[0] === 'store_credentials').slice(-1)[0] || [null, {}])[1].vault || ''")
    if cred_vault != DEST_OLD: bad.append(f"store_credentials named {cred_vault!r}, not {DEST_OLD!r}")

    # 7. Gmail is honest and does nothing; slots are live and the summary follows them.
    if "test user" not in page.inner_text("#wiz-gmail"): bad.append("the Gmail panel does not say it is testing-mode only")
    page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-slots"): bad.append("Next did not reach the slots panel")
    page.fill("#wiz-slot1", "09:00"); page.wait_for_timeout(120)
    page.click("#wiz-next"); page.wait_for_timeout(200)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not reach the finish panel")
    summary = page.inner_text("#wiz-summary")
    if "09:00" not in summary: bad.append(f"the summary did not pick up the edited slot: {summary!r}")
    if DEST_OLD not in summary: bad.append("the summary does not say where the vault will be")

    # 8. R-P4a-23 still holds: Back, rename, forward — and the credentials MOVE before anything exists.
    for _ in range(5):
        page.click("#wiz-back"); page.wait_for_timeout(90)
    if page.is_hidden("#wiz-vault"): bad.append("Back did not walk all the way to the name panel")
    page.fill("#wiz-name", "Spring 2027"); page.wait_for_timeout(120)
    for _ in range(5):
        page.click("#wiz-next"); page.wait_for_timeout(150)
    if page.is_hidden("#wiz-finish"): bad.append("Next did not walk back to the finish panel")
    if DEST_NEW not in page.inner_text("#wiz-summary"): bad.append("the summary did not follow the rename")

    # 9. Decision 3, measured: nothing but reads, the account and the credentials has happened yet.
    before = set(names(page))
    if not before <= BEFORE_FINISH_OK: bad.append(f"something reached disk before Finish: {sorted(before - BEFORE_FINISH_OK)}")
    page.click("#wiz-next"); page.wait_for_timeout(500)
    order = names(page)
    if "retarget_credentials" not in order:
        bad.append("a rename after the logins panel did not move the credentials")
    elif "create_vault" not in order or order.index("retarget_credentials") > order.index("create_vault"):
        bad.append("the credentials were moved after the vault was created, not before")
    rt = first_args(page, "retarget_credentials") or {}
    if rt.get("from_vault") != DEST_OLD or rt.get("to_vault") != DEST_NEW:
        bad.append(f"retarget_credentials was called with {rt.get('from_vault')!r} -> {rt.get('to_vault')!r}")
    created = first_args(page, "create_vault")
    if not created:
        bad.append("Finish did not invoke create_vault")
    else:
        plan = created.get("plan") or {}
        if created.get("name") != "Spring 2027": bad.append(f"create_vault got name {created.get('name')!r}")
        if "parent" in created: bad.append("create_vault still takes a parent folder")
        if "backup_dir" in plan: bad.append("the plan still carries a backup folder")
        if not str(plan.get("ics_url") or "").endswith(".ics"): bad.append("the plan did not carry the captured feed")
        if not str(plan.get("personal_calendar") or "").endswith(".ics"): bad.append("the plan did not carry the personal calendar")
        if ((plan.get("campus_choice") or {}).get("unitid")) != "100751": bad.append("the plan did not carry the school")
        zy = plan.get("zybooks_courses") or []
        if not any(b.get("code") == "UACS100Fall2026" and b.get("label") == "CS 100" for b in zy):
            bad.append(f"the plan did not carry the zyBooks mapping: {zy!r}")
        vh = plan.get("vhl_sections") or []
        if not any(v.get("section") == "2102121" and v.get("label") == "GN 103" for v in vh):
            bad.append(f"the plan did not carry the VHL mapping: {vh!r}")
        if not any(c[0] == "CS 100" for c in (plan.get("course_map") or [])):
            bad.append("the plan did not carry the course map")
        if not any(c.get("code") == "UACS100Fall2026" for c in (plan.get("courses") or [])):
            bad.append("the plan did not carry the enrolled courses")
        if plan.get("zybooks") is not True: bad.append("the plan did not record that a zyBooks login was stored")
        if plan.get("timezone") != "America/Chicago": bad.append("the plan did not carry the timezone")
        if plan.get("slots") != ["09:00", "18:00"]: bad.append(f"the plan carried slots {plan.get('slots')!r}")
    if "finish_onboarding" not in order: bad.append("finish_onboarding was not invoked")
    return bad
```

### H7 (Task 21) — `.github/workflows/ci.yml`, the `deno test` job

`.github/**` is C0's. At merge, add this job beside `test`. It is a separate job on `ubuntu-latest` because nothing in it is Windows-specific and the Rust job is the long pole:

```yaml
  cloud:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@<sha>
      - uses: denoland/setup-deno@<sha>
        with: { deno-version: "2.x" }
      - name: deno check
        run: deno check --config cloud/supabase/deno.json cloud/supabase/functions/**/*.ts
      - name: deno lint
        run: deno lint --config cloud/supabase/deno.json cloud/supabase/
      - name: deno test
        run: deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
```

Pin `denoland/setup-deno` by full commit SHA, as `engine/tests/workflows.rs::every_action_is_pinned_to_a_full_commit_sha` requires. Do **not** add `cloud` to branch protection's required checks in the same commit that creates it — add it once it has run green on one PR.

**R-X-7:** this is the *only* `cloud` job. C2's own hand-off **amends** it — widening the path scope to `cloud/` and adding `--allow-net=127.0.0.1` if and only if its loopback tests need it — and adds `eval-gate` as a second, separate job. The controller merges both into one `ci.yml`; neither stream adds a second `cloud` job. One consequence to keep in mind when merging: `deno test` fetches `@std/assert` from JSR on a cold cache, so if CI is ever run without network the job needs `--frozen` and a vendored cache; today it has network and this is a note, not a change.

### H8 (Task 21) — `CLAUDE.md`, two edits

The "Knowlu (the app)" section's command-count sentence becomes: *"**Tauri commands, recounted 2026-09-\<dd\>** from the two `generate_handler!` lists in `app/src/main.rs`: the console window registers **43**, the vault-less picker/wizard window **27** — 59 distinct. Commands live beside the module they serve (`commands.rs`, `onboarding.rs`, `account.rs`, `report.rs`, `lms_link.rs`), never all in one file. Recount before quoting a number."*

And a new bullet after the credentials one: *"The account's session JWT is Credential Manager's `knowlu/<profile_id>/session` (`app/src/account.rs`), moved there at onboarding from a pre-vault `knowlu/pending/session` entry; `config/cloud.yaml` names it alongside the project's `api_base`, its public `anon_key` and the `account_id`. Entitlement is cached at `profiles\<id>\entitlement.json` with a 72-hour grace, and past it every cloud step is a named skipped step, never a failure."*

---

### H9 (mid-stream, **split in two**) — `app/src/main.rs`, the `lms_link` commands as they come to exist

Task 13's spike opens a window, and a window cannot be opened from a command that is not registered.
But `generate_handler!` on a path that does not resolve is a **compile error**, so registering all
five at Task 13 would break the build the registration exists to unblock: Task 13 defines three,
`paste_calendar_link` arrives in Task 14 and `capture_courses` in Task 14b. **Ruling R-C1-11: H9 is
applied in two parts, and never asks the controller to register a name that does not compile yet.**

**H9a — at Task 13 step 3.** Add `lms_link` to `main.rs`'s `use` line and **exactly these three** to
the shell's `generate_handler!` list:

```
lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::close_lms_window
```

**H9b — at Task 14b step 3a**, after step 3 has written `capture_courses` and before step 5's
`cargo test`. (Step 2's red run comes *before* this and must not have it: the two names do not resolve
yet, and a `generate_handler!` compile error is not the failure that step is watching for.) Add the
remaining two, in H3's order:

```
lms_link::paste_calendar_link, lms_link::capture_courses
```

`paste_calendar_link` exists from Task 14 step 4 and `capture_courses` from Task 14b step 3, so H9b
is safe the moment Task 14b's `lms_link.rs` is written. Everything else in H3 — the **eleven** commands
`account` and `onboarding` add (eight and three), and the console window's whole list — waits for merge.

**Where these two sit in the plan's five controller applications**, the same order Task 13 step 3,
Task 14a step 6 and Task 14b step 3a state: **H1** (pre-flight, before Task 10) → **H9a** (Task 13
step 3) → **H10** (Task 14a step 6) → **H9b** (Task 14b step 3a) → **H11** (Task 14c, before its first
`cargo test`). H10 comes between the two halves of H9 because Task 14a runs before Task 14b.

---

### H10 (Task 14a) — `engine/`, one new subcommand: `coursework-discover`

**Why an engine change at all.** `coursework --dry-run` fetches and syncs in dry mode; what it *says*
about an unmapped book is a warning on stdout (`zybook UACS100Fall2026 not in config; skipped`), and
the wizard would have to parse English to learn the codes. It also needs a vault, which the wizard does
not have yet. So the discovery is its own read-only subcommand that takes credential targets, writes
nothing, and answers in JSON. **Always exits 0** — no credential, no network and no source are normal
outcomes for an app that is still onboarding.

`engine/src/main.rs`, in the `Command` enum:

```rust
    /// What the coursework sources can see, without writing anything: the zyBooks books and the VHL
    /// sections this account reaches, and — with `--vault` — whether that vault's `config/ingest.yaml`
    /// already places each one. One JSON object on stdout; always exit 0.
    CourseworkDiscover {
        /// Optional: with it, the credential targets and `enabled` flags come from the vault's own
        /// config and `mapped` is computed. Without it (the wizard, whose vault does not exist yet)
        /// the targets come from the two flags and `mapped` is always false.
        #[arg(long)]
        vault: Option<PathBuf>,
        #[arg(long = "zybooks-target")]
        zybooks_target: Option<String>,
        #[arg(long = "vhl-target")]
        vhl_target: Option<String>,
    },
```

…and in the dispatch:

```rust
        Command::CourseworkDiscover { vault, zybooks_target, vhl_target } => {
            println!("{}", coursework::discover_json(vault.as_deref(), zybooks_target.as_deref(), vhl_target.as_deref()));
            0
        }
```

`engine/src/vhl.rs` — the discovery half of `parse_dashboard`, which needs no `sections:` config
because finding out what that config should say is the point:

```rust
/// `(course_id, section_id)` for every summary the dashboard carries, first-seen order, deduplicated.
/// The summaries name no course **title** — only `/courses/<id>/sections/<id>/` — so onboarding shows
/// the pair and asks the student for the code and the label rather than inventing one.
pub fn discover_sections(html: &str) -> Vec<(String, String)> {
    static COURSE_SECTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/courses/(\d+)/sections/(\d+)/").unwrap());
    let mut out: Vec<(String, String)> = Vec::new();
    for caps in COURSE_SECTION.captures_iter(html) {
        let pair = (caps[1].to_string(), caps[2].to_string());
        if !out.contains(&pair) {
            out.push(pair);
        }
    }
    out
}
```

…with its test beside the committed fixture:

```rust
    #[test]
    fn discover_sections_finds_every_course_and_section_once() {
        let pairs = discover_sections(&dashboard_html());
        assert!(pairs.contains(&("1623220".to_string(), "2102121".to_string())), "{pairs:?}");
        let mut sorted = pairs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), pairs.len(), "a pair was reported twice");
    }
```

`engine/src/coursework.rs` — the subcommand's body:

```rust
/// Everything onboarding needs to build a mapping, and nothing else. **Never writes**, never starts a
/// run record, and always returns a JSON object — a source that could not be reached is an `errors`
/// entry, not an exit code, because the wizard's answer to "we could not reach zyBooks" is to let the
/// student type the mapping, not to stop.
#[cfg(windows)]
pub fn discover_json(vault: Option<&Path>, zybooks_target: Option<&str>, vhl_target: Option<&str>) -> String {
    use serde_json::json;
    let config: Mapping = vault
        .and_then(|v| load_coursework_config(v).ok())
        .map(|(cfg, _)| cfg)
        .unwrap_or_default();
    let block = match crate::yaml::get(&config, "coursework") {
        Some(Yaml::Mapping(map)) => map,
        _ => Mapping::new(),
    };
    let source = |name: &str| -> Mapping {
        match crate::yaml::get(&block, name) {
            Some(Yaml::Mapping(map)) => map,
            _ => Mapping::new(),
        }
    };
    let mut errors: Vec<String> = Vec::new();

    // ---- zyBooks
    let zycfg = source("zybooks");
    let zytarget = zybooks_target
        .map(str::to_string)
        .unwrap_or_else(|| cfg_str(&zycfg, "credential_target", ""));
    let mut zybooks: Vec<serde_json::Value> = Vec::new();
    if !zytarget.is_empty() {
        let courses = match crate::yaml::get(&zycfg, "courses") {
            Some(Yaml::Mapping(map)) => map,
            _ => Mapping::new(),
        };
        let ignore: Vec<String> = match crate::yaml::get(&zycfg, "ignore") {
            Some(Yaml::Sequence(items)) => items.iter().map(yaml_str).collect(),
            _ => Vec::new(),
        };
        match (|| -> Result<Vec<String>, SourceError> {
            let cred = crate::wincred::read_credential(&zytarget)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            let (token, user_id) =
                crate::zybooks::signin(&cred.username, cred.password.expose(), None)?;
            crate::zybooks::fetch_zybook_codes(&token, user_id, None)
        })() {
            Ok(codes) => {
                for code in codes {
                    let routing = route_zybook(&code, &courses, &ignore);
                    zybooks.push(json!({
                        "code": code,
                        "mapped": matches!(routing, BookRouting::Mapped(_)),
                        "ignored": matches!(routing, BookRouting::Ignored),
                    }));
                }
            }
            Err(err) => errors.push(format!("zybooks: {err}")),
        }
    }

    // ---- VHL
    let vhlcfg = source("vhl");
    let vhltarget = vhl_target
        .map(str::to_string)
        .unwrap_or_else(|| cfg_str(&vhlcfg, "credential_target", ""));
    let mut vhl: Vec<serde_json::Value> = Vec::new();
    if !vhltarget.is_empty() {
        let sections = match crate::yaml::get(&vhlcfg, "sections") {
            Some(Yaml::Mapping(map)) => map,
            _ => Mapping::new(),
        };
        match (|| -> Result<String, SourceError> {
            let cred = crate::wincred::read_credential(&vhltarget)
                .map_err(|err| SourceError::Failed(format!("{err}")))?;
            crate::vhl::login_and_fetch_dashboard(
                &cred.username,
                cred.password.expose(),
                &cfg_str(&vhlcfg, "base_url", "https://www.vhlcentral.com"),
                None,
            )
        })() {
            Ok(html) => {
                for (course_id, section_id) in crate::vhl::discover_sections(&html) {
                    let mapped = matches!(
                        crate::yaml::get(&sections, &section_id),
                        Some(Yaml::Mapping(ref inner)) if !inner.is_empty()
                    );
                    vhl.push(json!({ "course_id": course_id, "section": section_id, "mapped": mapped }));
                }
            }
            Err(err) => errors.push(format!("vhl: {err}")),
        }
    }

    crate::ledger::dumps_value(&json!({ "zybooks": zybooks, "vhl": vhl, "errors": errors }))
}

/// The credential store is Windows-only, so discovery is too — and it says so rather than reporting
/// an empty semester, which is the same rule `fetch_zybooks` follows.
#[cfg(not(windows))]
pub fn discover_json(_vault: Option<&Path>, _zybooks_target: Option<&str>, _vhl_target: Option<&str>) -> String {
    use serde_json::json;
    crate::ledger::dumps_value(&json!({
        "zybooks": [], "vhl": [],
        "errors": ["credential store unavailable on this platform"],
    }))
}
```

**Nothing else in `engine/` changes**: no run record, no journal record, no write path, and none of the
eight frozen references is read or regenerated. `CLAUDE.md`'s command list gains one line at merge —
`coursework-discover [--vault <v>] [--zybooks-target <t>] [--vhl-target <t>]` — beside `coursework`.

---


### H11 (Task 14c) — `scripts/campuses-from-ipeds.ps1`, the generator for `app/campuses.json`

**Applied mid-stream, like H3's `lms_link` fifth:** `include_str!("../campuses.json")` makes the asset compile-blocking, so the controller runs this script and commits `app/campuses.json` on the branch's base when Task 14c asks, before that task's first `cargo test`; the rest of the hand-offs wait for merge.

`scripts/` is the controller's, and so is the **output** — `app/campuses.json`, which this script
writes and C1 only reads. It is committed: the wizard has to work on a first run with no network, which
is most first runs. It sits beside `tauri.conf.json` rather than under `app/static/` because the page
never loads it (the app's own CSP forbids a `fetch` of it, and `campus_search` answers from Rust), and
a 400 KB file in `frontendDist` would ship twice. Regenerated **by hand, once a year**, when NCES
publishes the next `HD####` file.

**The source, verified 2026-09-09.** The federal IPEDS *Institutional Characteristics* file, published
by the National Center for Education Statistics: `https://nces.ed.gov/ipeds/datacenter/data/HD2024.zip`
— a ~1 MB zip holding `HD2024.csv`, **6,072 rows plus a header**, **latin-1** (a BOM sits in front of
`UNITID`). It is US federal government data and therefore public domain; no licence file ships with the
asset because none is required, and the header records where it came from so that stays checkable.

The columns this uses: `UNITID`, `INSTNM`, `CITY`, `STABBR`, `WEBADDR`, `ICLEVEL` (1 = four-year,
2 = two-year, 3 = less than two-year), `CYACTIVE` (1 = active). `SECTOR`, `CONTROL`, `ZIP`, `LATITUDE`
and `LONGITUD` are in the file and are **not** used — a student picking their school needs a name, a
town and a state, and every extra column is bytes in an asset that ships in the installer.

```powershell
# scripts/campuses-from-ipeds.ps1 -- regenerate app/campuses.json from IPEDS.
#
# Run by hand, about once a year, when NCES publishes the next HD file. The output is COMMITTED: the
# wizard's school typeahead has to work on a first run with no network, which is most first runs.
#
# US federal data (NCES), public domain. The header line records which file this came from and when,
# so "is this current?" is answerable without re-downloading it.
#
# PowerShell 5.1. `Import-Csv -Encoding Default` reads the ANSI code page, which is what latin-1 means
# on a US Windows install -- the file has accented institution names and reading it as UTF-8 mangles
# them into replacement characters that then ship to every user.
[CmdletBinding()]
param(
  [string]$Url = "https://nces.ed.gov/ipeds/datacenter/data/HD2024.zip",
  [string]$Csv = "HD2024.csv",
  [string]$Out = "app/campuses.json"
)
$ErrorActionPreference = "Stop"

$work = Join-Path $env:TEMP ("ipeds-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $work | Out-Null
try {
  $zip = Join-Path $work "hd.zip"
  Write-Output "downloading $Url"
  Invoke-WebRequest -Uri $Url -OutFile $zip
  Expand-Archive -Path $zip -DestinationPath $work -Force
  $csvPath = Join-Path $work $Csv
  if (-not (Test-Path $csvPath)) { throw "$Csv is not in that zip" }

  $rows = Import-Csv -Path $csvPath -Encoding Default
  Write-Output ("read {0} rows from {1}" -f $rows.Count, $Csv)

  # The first column's name carries the file's BOM, so `$_.UNITID` misses on some hosts. Bind the
  # property by position once instead of trusting the name.
  $unitidName = ($rows[0].PSObject.Properties | Select-Object -First 1).Name

  $keep = $rows | Where-Object {
    $_.CYACTIVE -eq "1" -and ($_.ICLEVEL -eq "1" -or $_.ICLEVEL -eq "2")
  }
  Write-Output ("keeping {0} active two- and four-year institutions" -f $keep.Count)

  # A generic list, not `@()` with `+=`: the latter reallocates the whole array on each of six
  # thousand iterations. It finishes either way and this runs once a year — but one line removes the
  # only quadratic thing in the script.
  $campuses = New-Object System.Collections.Generic.List[object]
  foreach ($r in $keep) {
    # `WEBADDR` is inconsistent: some rows carry a scheme, some a path, some a trailing slash, some
    # nothing at all. Keep the HOST and nothing else -- partly because that is all the wizard shows,
    # and partly because `app/tests/onboarding.rs::the_campus_list_is_bundled_headed_and_small`
    # asserts the whole file carries no `http(s)://`. (`app/tests/static_assets.rs` cannot: its
    # `read()` helper resolves against `app/static/`, and this file is `app/campuses.json`.)
    # `$webhost`, not `$host`: `$Host` is a PowerShell automatic variable (the PSHost object), and
    # assigning to it shadows it in this scope. It usually works and PSScriptAnalyzer flags it, which
    # is one rename too many arguments.
    $webhost = ""
    $raw = ($r.WEBADDR + "").Trim()
    if ($raw) {
      $h = $raw -replace '^\s*https?://', ''
      $h = $h -replace '^www\.', ''
      $h = ($h -split '[/?#]')[0]
      $webhost = $h.ToLowerInvariant().Trim()
      if ($webhost -match '\s') { $webhost = "" }
    }
    [void]$campuses.Add(@(
      [int]$r.$unitidName,
      ($r.INSTNM + "").Trim(),
      ($r.CITY + "").Trim(),
      ($r.STABBR + "").Trim(),
      $webhost
    ))
  }

  # One row per line, compact: about 400 KB, diffable, and a new school shows up as one added line
  # rather than a reflowed file.
  $sb = New-Object System.Text.StringBuilder
  # **No URL in the header.** `app/tests/onboarding.rs::the_campus_list_is_bundled_headed_and_small`
  # asserts the whole file carries no `http(s)://` — the same rule that makes this script strip
  # schemes off `WEBADDR` — and a header naming the download would break it on line one. The source is
  # named in words; the URL lives in this script's own `-Url` default, which is where somebody looking
  # to regenerate it will look.
  [void]$sb.AppendLine(('{{"source":"NCES IPEDS {0}","retrieved":"{1}","count":{2},"campuses":[' -f `
    $Csv, (Get-Date -Format "yyyy-MM-dd"), $campuses.Count))
  for ($i = 0; $i -lt $campuses.Count; $i++) {
    $line = ConvertTo-Json $campuses[$i] -Compress
    if ($i -lt $campuses.Count - 1) { $line += "," }
    [void]$sb.AppendLine($line)
  }
  [void]$sb.AppendLine("]}")

  $full = Join-Path (Get-Location) $Out
  # LF, and UTF-8 without a BOM: `.gitattributes` says `* text=auto eol=lf`, and a BOM in a file the
  # page fetches is a parse error in some webviews.
  $text = $sb.ToString() -replace "`r`n", "`n"
  [System.IO.File]::WriteAllText($full, $text, (New-Object System.Text.UTF8Encoding($false)))
  $kb = [math]::Round((Get-Item $full).Length / 1KB)
  Write-Output ("wrote {0} ({1} schools, {2} KB)" -f $Out, $campuses.Count, $kb)
  if ($kb -gt 600) { throw "campuses.json is ${kb} KB; the guard in app/tests/onboarding.rs is 600" }
}
finally {
  Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
```

**When the next HD file lands**, the only change is `-Url` and `-Csv`; the header records both, so the
asset says which year it is without anyone remembering.

---

### Task 0: The toolchain, and the cloud codebase's skeleton

Deno and the Supabase CLI are not on this laptop (checked 2026-09-09: `deno --version` and `supabase --version` both resolve to nothing). Everything in Tasks 0–9 is written and unit-tested with Deno alone; the Supabase CLI is only ever used to *apply* and *deploy* against **staging**.

**Files:**
- Create: `cloud/supabase/deno.json`, `cloud/supabase/config.toml`, `cloud/supabase/.gitignore`, `cloud/supabase/README.md`, `cloud/supabase/templates/magic_link.html`
- Create: `cloud/supabase/functions/_shared/http.ts`
- Test: `cloud/supabase/functions/_shared/http_test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: `json(status, body): Response`, `fail(status, message): Response`, `methodNotAllowed(allowed: string[]): Response`, `readJson<T>(req: Request, limit?: number): Promise<T>`, `subPath(url: string, fnName: string): string`, `asResponse(e: unknown): Response`. Every later cloud task imports these.

- [ ] **Step 1: Install the two tools, and verify.** In PowerShell:

```powershell
if (-not (Get-Command deno -ErrorAction SilentlyContinue)) { winget install --id DenoLand.Deno --accept-source-agreements --accept-package-agreements }
if (-not (Get-Command supabase -ErrorAction SilentlyContinue)) { winget install --id Supabase.CLI --accept-source-agreements --accept-package-agreements }
$m=[Environment]::GetEnvironmentVariable("Path","Machine"); $u=[Environment]::GetEnvironmentVariable("Path","User"); $env:Path="$env:USERPROFILE\.cargo\bin;$m;$u"
deno --version
supabase --version
```

Expected: `deno 2.x.y` (with `v8` and `typescript` lines) and a bare `2.x.y` from `supabase`. A stale PATH is the usual failure — the third line refreshes it, exactly as `CLAUDE.md` prescribes for `gcc.exe`. **If either is already installed, skip its `winget` line and record the version you found.** Neither tool is a build dependency of the Rust workspace: `cargo build --workspace` must keep working on a machine that has neither.

- [ ] **Step 2: `cloud/supabase/deno.json`.**

```json
{
  "lock": false,
  "imports": {
    "@std/assert": "jsr:@std/assert@^1.0.0"
  },
  "lint": {
    "rules": { "tags": ["recommended"] }
  },
  "fmt": {
    "lineWidth": 110,
    "semiColons": true,
    "singleQuote": false
  }
}
```

`"lock": false` on purpose: the only dependency is the assertion library the tests use, pinned by range, and a lockfile whose bytes churn would fail CI for a reason that is never about the product. **No runtime dependency exists at all** — every handler talks to Postgres and to Auth over `fetch` against the REST and Auth APIs, so `supabase-js` never enters this codebase and `deno check` never needs a registry for anything but `@std/assert` in tests.

- [ ] **Step 3: `cloud/supabase/config.toml`.**

```toml
# Knowlu's Supabase project (C1). ONE codebase, TWO projects — `knowlu-staging` and `knowlu-prod`
# (spec §11 R6). `supabase link --project-ref <ref>` decides which one a command talks to, and
# nothing in this file names a project, so neither can be reached by accident.
project_id = "knowlu"

[api]
enabled = true
port = 54321
schemas = ["public"]
extra_search_path = ["public", "extensions"]
max_rows = 1000

[db]
port = 54322
major_version = 15

[auth]
enabled = true
site_url = "https://knowlu.com"
additional_redirect_urls = ["https://knowlu.com/signed-in.html"]
jwt_expiry = 3600
enable_refresh_token_rotation = true
refresh_token_reuse_interval = 10
enable_signup = true

[auth.email]
enable_signup = true
enable_confirmations = true
double_confirm_changes = true

# R-C1-3, second half. `enable_confirmations = true` means every sign-up depends on this mail
# arriving, and Supabase's built-in sender is a few messages an hour and is explicitly not for
# production — the second person to sign up in an hour would never get theirs. The provider is the
# same one `billing-jobs` uses for the renewal reminder (P2), so there is one account to watch.
# `pass` is a project secret (`SMTP_PASSWORD`), set by Quinn, never in this file.
# **Commented out until Task 4 step 1 fills it.** The first command that parses this file is Task 1
# step 7's `supabase db push`, and four `<P2: …>` placeholders in a block the CLI validates would fail
# there — at the exact moment P1 is being wired, for a reason that has nothing to do with P1.
# [auth.email.smtp]
# host = "<P2: the provider's SMTP host>"
# port = 587
# user = "<P2: the provider's SMTP username>"
# pass = "env(SMTP_PASSWORD)"
# admin_email = "<P2: EMAIL_FROM>"
# sender_name = "Knowlu"

# The magic link's mail carries a **six-digit code as well as a link**, because a desktop app cannot
# receive the link's redirect: `site/signed-in.html` says so, and the wizard asks for the code
# (`account::verify_email_code`). `{{ .Token }}` is GoTrue's own placeholder for it.
[auth.email.template.magic_link]
subject = "Your Knowlu sign-in code"
content_path = "./templates/magic_link.html"

# `verify_jwt = false` on every function, and it is not laxity. Each handler verifies the bearer
# token itself and answers 401 or 402 in OUR shape — the shape `_shared/entitlement.ts` promises C2
# and the app's `account.rs` parses. Supabase's gateway rejection is a different body with a
# different status, so leaving the gate to it would make the contract untestable. `stripe-webhook`
# has no JWT at all: it is authenticated by its signature.
[functions.entitlement]
verify_jwt = false
[functions.billing-checkout]
verify_jwt = false
[functions.billing-portal]
verify_jwt = false
[functions.stripe-webhook]
verify_jwt = false
[functions.billing-jobs]
verify_jwt = false
[functions.account]
verify_jwt = false
[functions.telemetry]
verify_jwt = false
[functions.issues]
verify_jwt = false
```

- [ ] **Step 4: `cloud/supabase/templates/magic_link.html`** — the mail `[auth.email.template.magic_link]` names. GoTrue substitutes `{{ .Token }}` and `{{ .ConfirmationURL }}`; the code is what a desktop app can actually use, so it comes first:

```html
<p>Your Knowlu sign-in code is <strong>{{ .Token }}</strong>.</p>
<p>Type it into Knowlu to sign in.</p>
<p>If you are on the same machine as your browser you can also <a href="{{ .ConfirmationURL }}">follow this link</a> — it will tell you the same code.</p>
<p>If you did not ask for this, you can ignore it. The code stops working in an hour.</p>
```

  **The `[auth.email.smtp]` block is written commented out, and Task 4 step 1 uncomments it** and fills its four `<P2: …>` slots when Quinn supplies the provider's host, port, username and `EMAIL_FROM`. The block is added now so the file is one decision rather than two — but behind `#`, because the first command that parses `config.toml` is Task 1 step 7's `supabase db push`, and four literal placeholders in a block the CLI validates would fail there, at the exact moment P1 is being wired, for a reason that has nothing to do with P1. Until Task 4, staging keeps the built-in sender.

- [ ] **Step 5: `cloud/supabase/.gitignore`.**

```gitignore
# Nothing secret is ever committed. `supabase link` writes .temp/, `supabase db diff` writes
# .branches/, and a local `.env` is how the CLI passes secrets to `functions serve`.
.env
.env.*
.branches/
.temp/
```

`templates/` is **not** ignored: the magic-link mail is copy, not a secret, and it has to be reviewable.

- [ ] **Step 6: `cloud/supabase/README.md`.**

````markdown
# `cloud/supabase/` — Knowlu's backend

One codebase, two projects: **`knowlu-staging`** and **`knowlu-prod`** (spec §11 R6). Nothing here
names a project; `supabase link` decides.

**Migrations are applied to STAGING from a developer machine, and to PROD by Quinn or by CI.**
That rule is the whole of this file:

```powershell
supabase login                                  # Quinn's own login, once
supabase link --project-ref <the STAGING ref>   # never the prod ref from a laptop
supabase db push                                # applies cloud/supabase/migrations/ in order
supabase functions deploy <name> --project-ref <the STAGING ref>
```

Secrets are set by Quinn, from a value he produces, and are never printed:

```powershell
supabase secrets set STRIPE_SECRET_KEY --project-ref <ref>
```

The project URL and the **anon key are public** and are compiled into the app. The
**service-role key** is injected into every function as `SUPABASE_SERVICE_ROLE_KEY` by the platform;
nothing here reads it at import time, so `deno test` needs no environment at all.

## Testing

```powershell
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
```

Every `index.ts` is a thin wire: it reads the environment, builds a `Deps` object and calls the
handler. Every `handler.ts` is pure over that `Deps`, so the tests need no Docker, no Supabase
runtime and no network. **No test here makes an outbound request** — the one thing `deno test`
fetches is the assertion library, once, into Deno's own cache.
````

- [ ] **Step 7: Write the failing test** — `cloud/supabase/functions/_shared/http_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { asResponse, fail, json, methodNotAllowed, readJson, subPath } from "./http.ts";

Deno.test("json() sets the status, the body and one content type", async () => {
  const r = json(200, { a: 1 });
  assertEquals(r.status, 200);
  assertEquals(r.headers.get("content-type"), "application/json; charset=utf-8");
  assertEquals(await r.json(), { a: 1 });
});

Deno.test("fail() is a body with an error key, never a bare string", async () => {
  const r = fail(402, "this account has no active subscription");
  assertEquals(r.status, 402);
  assertEquals(await r.json(), { error: "this account has no active subscription" });
});

Deno.test("methodNotAllowed() names what is allowed, in the body and in the header", async () => {
  const r = methodNotAllowed(["GET", "PUT"]);
  assertEquals(r.status, 405);
  assertEquals(r.headers.get("allow"), "GET, PUT");
  assertEquals(await r.json(), { error: "method not allowed; use GET, PUT" });
});

Deno.test("subPath() strips the /functions/v1/<name> prefix so one function can route", () => {
  assertEquals(subPath("https://x.supabase.co/functions/v1/account", "account"), "/");
  assertEquals(subPath("https://x.supabase.co/functions/v1/account/export", "account"), "/export");
  assertEquals(subPath("https://x.supabase.co/functions/v1/account/sources?a=1", "account"), "/sources");
  // A request that does not carry the prefix at all (a direct invoke in a test) still routes.
  assertEquals(subPath("http://127.0.0.1:9999/sources", "account"), "/sources");
});

Deno.test("readJson() refuses a body over the limit rather than parsing it", async () => {
  const big = new Request("http://127.0.0.1:1/", { method: "POST", body: JSON.stringify({ s: "x".repeat(50) }) });
  try {
    await readJson(big, 16);
    throw new Error("readJson accepted a body over the limit");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 413);
  }
});

Deno.test("readJson() refuses a body that is not JSON with 400, not 500", async () => {
  const bad = new Request("http://127.0.0.1:1/", { method: "POST", body: "{oh no" });
  try {
    await readJson(bad);
    throw new Error("readJson accepted a body that is not JSON");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 400);
  }
});

Deno.test("asResponse() passes a thrown Response through and hides anything else", async () => {
  const thrown = fail(401, "no bearer token");
  assertEquals(asResponse(thrown), thrown);
  const hidden = asResponse(new Error("connection string: postgres://user:pw@host/db"));
  assertEquals(hidden.status, 500);
  assertEquals(await hidden.json(), { error: "internal error" });
});
```

- [ ] **Step 8: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `error: Module not found "file:///…/functions/_shared/http.ts"` — seven tests, none of them run.

- [ ] **Step 9: `cloud/supabase/functions/_shared/http.ts`.**

```ts
/**
 * Responses and request reading, shared by every function. **No I/O of its own**: a handler that
 * needs the network or the database takes it as an injected dependency, which is what lets
 * `deno test` run this whole codebase with `--allow-read` and nothing else.
 *
 * Failures are thrown as `Response` objects rather than as errors with codes. That is the contract
 * `_shared/entitlement.ts` promises C2 (`throws a Response`, 401 or 402), and it means a handler's
 * happy path reads straight down with no error plumbing in it.
 */

export const JSON_HEADERS = { "content-type": "application/json; charset=utf-8" } as const;

export function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { ...JSON_HEADERS } });
}

/** Every error body in this codebase has exactly one shape: `{"error": "<one sentence>"}`. */
export function fail(status: number, message: string): Response {
  return json(status, { error: message });
}

export function methodNotAllowed(allowed: string[]): Response {
  const list = allowed.join(", ");
  return new Response(JSON.stringify({ error: `method not allowed; use ${list}` }), {
    status: 405,
    headers: { ...JSON_HEADERS, allow: list },
  });
}

/**
 * The path inside one function, with the platform's `/functions/v1/<name>` prefix removed, so a
 * single function can carry the several routes the spec names (`/account`, `/account/export`,
 * `/account/sources`). A request that never had the prefix — a direct invoke in a test, or a
 * `supabase functions serve` call — is returned unchanged, so the routing is the same in both.
 */
export function subPath(url: string, fnName: string): string {
  const path = new URL(url).pathname;
  const marker = `/${fnName}`;
  const at = path.indexOf(marker);
  const rest = at < 0 ? path : path.slice(at + marker.length);
  return rest === "" ? "/" : rest;
}

/**
 * The body, parsed, with a size cap. **Throws a `Response`** — 413 over the cap, 400 for anything
 * that is not JSON — so a caller never has to tell a parse failure from a bug.
 */
export async function readJson<T>(req: Request, limit = 1 << 20): Promise<T> {
  const text = await req.text();
  // `String.length` is UTF-16 code units, not bytes — say what is measured rather than
  // pretending to a precision this does not have. The cap is a sanity bound, not an accounting.
  if (text.length > limit) throw fail(413, `body over ${limit} characters`);
  try {
    return JSON.parse(text) as T;
  } catch {
    throw fail(400, "body is not JSON");
  }
}

/**
 * The one `catch` every `index.ts` uses. A thrown `Response` is the handler's own answer and goes
 * back verbatim; **anything else becomes a bare 500**, because a stray error's message can carry a
 * connection string, a key or a row of somebody's data and an edge function's body is public.
 */
export function asResponse(e: unknown): Response {
  if (e instanceof Response) return e;
  console.error("unhandled:", e instanceof Error ? e.message : String(e));
  return fail(500, "internal error");
}
```

- [ ] **Step 10: Run it and watch it pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `ok | 7 passed | 0 failed`. Then `deno lint --config cloud/supabase/deno.json cloud/supabase/` → `Checked N files` with no findings, and `deno fmt --check --config cloud/supabase/deno.json cloud/supabase/` → `Checked N files`.

- [ ] **Step 11: Prove the Rust workspace is untouched.**

Run: `cargo test --workspace`
Expected: the same counts as `main` (engine 888 passed / 3 ignored, app 129 / 1) at 0 warnings, with only the `.rsrc merge failure: multiple non-default manifests` line.

- [ ] **Step 12: Commit.**

```bash
git add cloud/supabase/deno.json cloud/supabase/config.toml cloud/supabase/.gitignore cloud/supabase/README.md cloud/supabase/templates/magic_link.html cloud/supabase/functions/_shared/http.ts cloud/supabase/functions/_shared/http_test.ts
git commit -F <message file>
```

Message: `cloud: the Supabase codebase's skeleton and its one shared response module (C1 Task 0)` plus the two trailers.

---

### Task 1: The account tables, and row-level security on every one of them

**Precondition P1 (the two Supabase projects) is needed at step 7, not before.** Steps 1–6 are offline.

**Files:**
- Create: `cloud/supabase/migrations/20260910000100_accounts.sql`
- Test: `cloud/supabase/migrations_test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: the tables `public.accounts`, `public.entitlements`, `public.consents`, `public.sources`, `public.webhook_events` and the trigger `on_auth_user_created`. Every later cloud task reads or writes these through PostgREST with the service role.

Applied to staging 2026-09-10 (`knowlu-staging`, `brvhgbihxevrudqpulcm`, us-east-1) in one push with Tasks 5–9's migrations: ten tables, ten RLS, zero non-SELECT policies — this task's five plus `billing_reminders`, `deleted_accounts`, `corrections`, `telemetry_events`, `issues`. Migration 000200 needed its column adds moved ahead of its view first (R-C1-34).

- [ ] **Step 1: Write the failing test** — `cloud/supabase/migrations_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";

const DIR = new URL("./migrations/", import.meta.url);

/** C1's own migrations, and **only** C1's (R-X-8). C2 puts its `20260911…` files in this same
 * directory and merges after C1; a helper that read the whole directory would turn this suite red on
 * C2's first commit for a reason that has nothing to do with C2's change. C2's own test filters the
 * same way, from its own side. */
const MINE = /^20260910\d{6}_[a-z0-9_]+\.sql$/;

async function migrations(): Promise<{ name: string; sql: string }[]> {
  const out: { name: string; sql: string }[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.endsWith(".sql") && MINE.test(e.name)) {
      out.push({ name: e.name, sql: await Deno.readTextFile(new URL(e.name, DIR)) });
    }
  }
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

/** Every `.sql` in the directory, C2's included — used only by the one test that must see them all. */
async function allMigrationNames(): Promise<string[]> {
  const out: string[] = [];
  for await (const e of Deno.readDir(DIR)) {
    if (e.isFile && e.name.endsWith(".sql")) out.push(e.name);
  }
  return out.sort();
}

Deno.test("every migration is stamped and lower case, and C1's are in C1's day", async () => {
  // The SHAPE is asserted over every file in the directory, C2's included — a migration nobody can
  // order is a problem whoever wrote it.
  for (const name of await allMigrationNames()) {
    assert(/^\d{14}_[a-z0-9_]+\.sql$/.test(name), `${name}: not <YYYYMMDDHHMMSS>_<name>.sql`);
  }
  // The DAY is asserted only over C1's own (R-X-8): C1 owns 2026-09-10, C2 owns 2026-09-11, and
  // C2 merges after C1 into this same directory.
  const mine = await migrations();
  assert(mine.length > 0, "no C1 migrations found");
  for (const m of mine) {
    assert(m.name.startsWith("20260910"), `${m.name}: C1's migrations are stamped 20260910…`);
  }
});

Deno.test("no migration ever creates a birthdate column", async () => {
  // Spec §9, minors: the 18+ gate is an attestation boolean with a timestamp. A birthdate is
  // personal data we would then have to protect, and it buys nothing.
  for (const m of await migrations()) {
    for (const word of ["birthdate", "birth_date", "date_of_birth", " dob ", "birthday"]) {
      assert(!m.sql.toLowerCase().includes(word), `${m.name} names ${word.trim()}`);
    }
  }
});

Deno.test("row-level security is enabled on every table these migrations create", async () => {
  const created: string[] = [];
  const secured: string[] = [];
  for (const m of await migrations()) {
    for (const c of m.sql.matchAll(/create\s+table\s+(?:if\s+not\s+exists\s+)?public\.(\w+)/gi)) {
      created.push(c[1].toLowerCase());
    }
    for (const s of m.sql.matchAll(/alter\s+table\s+public\.(\w+)\s+enable\s+row\s+level\s+security/gi)) {
      secured.push(s[1].toLowerCase());
    }
  }
  assert(created.length > 0, "no tables created");
  for (const t of created) assert(secured.includes(t), `public.${t} has no RLS`);
});

Deno.test("no table has a client write policy — every write is an edge function's", async () => {
  // This is what makes "the Stripe webhook is the only writer of entitlements" an enforced
  // property rather than a convention: the service role bypasses RLS and nothing else may write.
  for (const m of await migrations()) {
    for (const p of m.sql.matchAll(/create\s+policy\s+(\w+)[\s\S]*?for\s+(select|insert|update|delete|all)/gi)) {
      assertEquals(p[2].toLowerCase(), "select", `policy ${p[1]} grants ${p[2]} to a client`);
    }
  }
});

Deno.test("entitlements is readable by its owner and by nothing else", async () => {
  const all = await migrations();
  const sql = all.map((m) => m.sql).join("\n").toLowerCase();
  assert(sql.includes("create policy entitlements_select_own"), "no owner-read policy on entitlements");
  const policies = [...sql.matchAll(/create\s+policy\s+(\w+)\s+on\s+public\.entitlements/g)].map((m) => m[1]);
  assertEquals(policies, ["entitlements_select_own"], `entitlements has extra policies: ${policies}`);
});
```

- [ ] **Step 2: Run it and watch it fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `no migrations found` on the first test (the directory does not exist yet, so `Deno.readDir` throws `NotFound` — that is a failure, not a pass).

- [ ] **Step 3: `cloud/supabase/migrations/20260910000100_accounts.sql`.**

```sql
-- Knowlu C1, Task 1 — the account, its subscription, its consent log and the one source we store.
-- Spec §5.1. Three rules are load-bearing and are pinned by cloud/supabase/migrations_test.ts:
--   1. There is NO date-of-birth column, in this file or any other (spec §9, minors).
--   2. Row-level security is on for every table.
--   3. No table has a client write policy. Every write in this system goes through an edge function
--      that verified the caller's JWT and then used the service role, which bypasses RLS. That is
--      what makes "the Stripe webhook is the only writer of entitlements" enforced rather than
--      promised.
create extension if not exists pgcrypto with schema extensions;

create table public.accounts (
  id                 uuid primary key references auth.users (id) on delete cascade,
  email              text not null,
  created_at         timestamptz not null default now(),
  tos_version        text,
  tos_accepted_at    timestamptz,
  privacy_version    text,
  age_attested_at    timestamptz,
  stripe_customer_id text unique,
  status             text not null default 'active' check (status in ('active', 'closed'))
);
comment on table public.accounts is
  'Spec §5.1, exactly. No date-of-birth column exists, or ever may: the 18+ gate is an attestation.';

create table public.entitlements (
  account_id         uuid primary key references public.accounts (id) on delete cascade,
  plan               text,
  status             text not null check (status in ('active', 'trialing', 'past_due', 'canceled', 'none')),
  current_period_end timestamptz,
  source             text not null default 'stripe',
  updated_at         timestamptz not null default now()
);
comment on table public.entitlements is
  'Written ONLY by the stripe-webhook function, through the service role. GET /entitlement reads it.';

create table public.consents (
  id           bigint generated always as identity primary key,
  account_id   uuid references public.accounts (id) on delete set null,
  subject_hash text not null,
  kind         text not null check (kind in ('tos', 'privacy', 'age_18', 'auto_renew')),
  version      text not null,
  accepted_at  timestamptz not null default now(),
  price_cents  integer,
  ip           inet
);
comment on table public.consents is
  'Kept three years (California ARL as amended by AB 2863; ROSCA has no term). DELETE /account nulls
   account_id and leaves the row: subject_hash keeps the record meaningful without identifying
   anyone, which is how a legal-retention duty and a deletion right are both honoured.';

create table public.sources (
  account_id     uuid not null references public.accounts (id) on delete cascade,
  -- **Three kinds, all declared here, in C1's migration, on purpose.** `lms_ics` is the school's
  -- assignment feed and `calendar_ics` the student's own busy time — the secret iCal address Google
  -- Calendar hands out; the wizard asks for both on one panel (spec §11a). `google_calendar` is
  -- **reserved and written by nobody** (R-X-9): a Google grant has no URL for the two not-null
  -- columns, so it lives in C2's `google_accounts`, and `/ingest-calendar` resolves `google` from
  -- there. The value is in this constraint from the start so that C2 never has to alter a table in a C1-owned migration:
  -- the ownership list forbids it, and a check constraint is the one thing two streams cannot both
  -- edit safely. C1 writes rows of the first two kinds only.
  kind           text not null check (kind in ('lms_ics', 'calendar_ics', 'google_calendar')),
  url_ciphertext text not null,
  url_iv         text not null,
  added_at       timestamptz not null default now(),
  primary key (account_id, kind)
);
comment on table public.sources is
  'The two URL kinds are capability URLs: anyone holding one reads that student''s schedule — the school feed
   their assignments, the personal one their life. Both are AES-GCM encrypted by the account function
   before they arrive here, and GET /account/sources never returns either. google_calendar is reserved
   and never written: a Google grant has no URL and lives in google_accounts (C2).';

alter table public.accounts enable row level security;
alter table public.entitlements enable row level security;
alter table public.consents enable row level security;
alter table public.sources enable row level security;

create policy accounts_select_own on public.accounts
  for select to authenticated using (id = auth.uid());
create policy entitlements_select_own on public.entitlements
  for select to authenticated using (account_id = auth.uid());
create policy sources_select_own on public.sources
  for select to authenticated using (account_id = auth.uid());
-- `consents` has no policy at all, deliberately: a consent log a client cannot read is still a
-- consent log, and GET /account/export returns the caller's own rows through the service role.

create index consents_subject_idx on public.consents (subject_hash, accepted_at desc);

-- Stripe retries for days and does not guarantee order. Two guards, both cheap:
--   * an event id already seen is dropped before anything is written (idempotency);
--   * a write is refused when the event is OLDER than the row it would overwrite, so a late
--     `customer.subscription.updated` cannot resurrect a subscription `deleted` already cancelled.
-- `entitlements.updated_at` therefore carries the EVENT's own `created`, not `now()`, which is what
-- makes that comparison mean anything.
create table public.webhook_events (
  event_id    text primary key,
  event_type  text not null,
  created_at  timestamptz not null,
  received_at timestamptz not null default now()
);
alter table public.webhook_events enable row level security;
-- No policy at all: nothing but the service role has any business reading this table.

-- The account row, its empty entitlement and its consent records are written the moment the auth
-- user is, from the metadata the wizard sends with the sign-up. One round trip, and the attestation
-- cannot drift away from the user it belongs to.
--
-- The `raise exception` is the 18+ gate, server side: Alabama's Ala. Code § 26-1-1(f) makes an
-- 18-year-old's contract binding, a 17-year-old's is voidable, and the cheapest compliant path is to
-- decline rather than collect a date of birth. The app refuses first, with a sentence; this refuses
-- second, so a patched client gets an account it cannot use either.
create function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public, extensions as $$
declare
  attested boolean := (new.raw_user_meta_data ->> 'age_attested') = 'true';
  tos      text    := nullif(new.raw_user_meta_data ->> 'tos_version', '');
  priv     text    := nullif(new.raw_user_meta_data ->> 'privacy_version', '');
begin
  if not attested then
    raise exception 'age attestation required: Knowlu is for people 18 or older';
  end if;
  if tos is null or priv is null then
    raise exception 'the terms and the privacy policy must be accepted at sign-up';
  end if;

  insert into public.accounts (id, email, tos_version, tos_accepted_at, privacy_version, age_attested_at)
  values (new.id, new.email, tos, now(), priv, now())
  on conflict (id) do nothing;

  insert into public.entitlements (account_id, status) values (new.id, 'none')
  on conflict (account_id) do nothing;

  insert into public.consents (account_id, subject_hash, kind, version)
  select new.id,
         encode(extensions.digest(lower(new.email), 'sha256'), 'hex'),
         k.kind,
         k.version
  from (values ('tos', tos), ('privacy', priv), ('age_18', '1')) as k(kind, version);

  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();
```

- [ ] **Step 4: Run the tests and watch them pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `ok | 12 passed | 0 failed` (Task 0's seven and this task's five).

- [ ] **Step 5: Prove the last test can fail.** Temporarily change `for select to authenticated` on `accounts_select_own` to `for all to authenticated`, re-run, and confirm `policy accounts_select_own grants all to a client`. **Revert it.** A test that has never been seen red is not a test.

- [ ] **Step 6: `deno fmt` and `deno lint`, then commit the offline half.**

```bash
git add cloud/supabase/migrations/20260910000100_accounts.sql cloud/supabase/migrations_test.ts
git commit -F <message file>
```

Message: `cloud: accounts, entitlements, consents and sources — RLS on every table, no birthdate anywhere, the 18+ gate in the sign-up trigger (C1 Task 1)`.

- [ ] **Step 7: Ask Quinn for P1, then apply to STAGING.** Give him this context, in one message: *"C1 needs a Supabase organisation with two projects, `knowlu-staging` and `knowlu-prod`, both region US. I need three non-secret values per project — the reference id, the URL and the anon key — and nothing else; you run `supabase login` yourself and you set every secret yourself. The staging project is where every migration lands from a laptop; production is only ever migrated by you or by CI."* Then, with the staging ref:

```powershell
supabase login
supabase link --project-ref <the STAGING ref> --workdir cloud/supabase
supabase db push --workdir cloud/supabase
```

Expected: `Applying migration 20260910000100_accounts.sql...` then `Finished supabase db push.`

- [ ] **Step 8: Prove the gate on staging.** In the project's SQL editor (Quinn's browser, or `supabase db query`), run:

```sql
select count(*) from pg_policies where schemaname = 'public' and cmd <> 'SELECT';
```

Expected: `0`. Then confirm the five tables exist and all five have `rowsecurity = true`:

```sql
select relname, relrowsecurity from pg_class
where relnamespace = 'public'::regnamespace and relkind = 'r' order by relname;
```

Expected: five rows — `accounts`, `consents`, `entitlements`, `sources`, `webhook_events` — all `t`.

- [ ] **Step 9: Record the outcome.** Add to this task, above step 1: `Applied to staging <date>; five tables, five RLS, zero non-SELECT policies.`

---

### Task 2: `_shared` — the REST client, the bearer check, and the entitlement gate C2 imports

**Files:**
- Create: `cloud/supabase/functions/_shared/db.ts`, `cloud/supabase/functions/_shared/auth.ts`, `cloud/supabase/functions/_shared/entitlement.ts`
- Test: `cloud/supabase/functions/_shared/auth_test.ts`, `cloud/supabase/functions/_shared/entitlement_test.ts`

**Interfaces:**
- Consumes: `json`, `fail` from `_shared/http.ts`.
- Produces:
  - `db.ts`: `interface Rest { url: string; serviceKey: string; fetch: typeof fetch }`, `restSelect<T>(rest, table, query): Promise<T[]>`, `restUpsert(rest, table, rows, onConflict?): Promise<void>`, `restPatch(rest, table, query, patch): Promise<void>`, `restDelete(rest, table, query): Promise<void>`, `authGetUser(rest, token): Promise<AuthedUser | null>`, `authDeleteUser(rest, id): Promise<void>`.
  - `auth.ts`: `interface AuthedUser { id: string; email: string | null }`, `type VerifyToken = (token: string) => Promise<AuthedUser | null>`, `parseBearer(req): string | null`, `requireUser(req, verify): Promise<AuthedUser>`.
  - `entitlement.ts`: `type EntitlementStatus`, `interface EntitlementRow { plan: string | null; status: EntitlementStatus; current_period_end: string | null }`, `type LookupEntitlement = (accountId: string) => Promise<EntitlementRow | null>`, `interface EntitlementDeps { verify: VerifyToken; lookup: LookupEntitlement }`, `isActive(row): boolean`, `requireActiveEntitlementWith(req, deps): Promise<{ account_id: string }>`, **`requireActiveEntitlement(req): Promise<{ account_id: string }>`** — the C2 contract.

- [ ] **Step 1: Write the failing tests** — `cloud/supabase/functions/_shared/auth_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { parseBearer, requireUser } from "./auth.ts";

const req = (headers: Record<string, string>) => new Request("http://127.0.0.1:1/", { headers });

Deno.test("parseBearer takes the token and nothing around it", () => {
  assertEquals(parseBearer(req({ authorization: "Bearer abc.def.ghi" })), "abc.def.ghi");
  assertEquals(parseBearer(req({ Authorization: "bearer abc" })), "abc");
  assertEquals(parseBearer(req({ authorization: "  Bearer   abc  " })), "abc");
  assertEquals(parseBearer(req({ authorization: "Basic abc" })), null);
  assertEquals(parseBearer(req({ authorization: "Bearer" })), null);
  assertEquals(parseBearer(req({})), null);
});

Deno.test("requireUser throws a 401 Response when there is no token", async () => {
  try {
    await requireUser(req({}), () => Promise.resolve({ id: "u", email: null }));
    throw new Error("requireUser accepted a request with no bearer token");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
    assertEquals(await e.json(), { error: "no bearer token" });
  }
});

Deno.test("requireUser throws a 401 Response when the token does not verify", async () => {
  try {
    await requireUser(req({ authorization: "Bearer stale" }), () => Promise.resolve(null));
    throw new Error("requireUser accepted an unverifiable token");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
  }
});

Deno.test("requireUser hands back exactly what the verifier said", async () => {
  const u = await requireUser(
    req({ authorization: "Bearer good" }),
    (t) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null),
  );
  assertEquals(u, { id: "acc-1", email: "a@example.invalid" });
});
```

…and `cloud/supabase/functions/_shared/entitlement_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { EntitlementRow, isActive, requireActiveEntitlementWith } from "./entitlement.ts";

const row = (status: EntitlementRow["status"]): EntitlementRow => ({ plan: "monthly", status, current_period_end: null });
const bearer = new Request("http://127.0.0.1:1/", { headers: { authorization: "Bearer good" } });
const verify = (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null);

Deno.test("active and trialing are entitled; nothing else is", () => {
  assertEquals(isActive(row("active")), true);
  assertEquals(isActive(row("trialing")), true);
  assertEquals(isActive(row("past_due")), false);
  assertEquals(isActive(row("canceled")), false);
  assertEquals(isActive(row("none")), false);
  assertEquals(isActive(null), false);
});

Deno.test("an entitled caller gets its account id back", async () => {
  const out = await requireActiveEntitlementWith(bearer, { verify, lookup: () => Promise.resolve(row("trialing")) });
  assertEquals(out, { account_id: "acc-1" });
});

Deno.test("an unentitled caller gets a thrown 402 Response — the contract C2 imports", async () => {
  try {
    await requireActiveEntitlementWith(bearer, { verify, lookup: () => Promise.resolve(row("past_due")) });
    throw new Error("requireActiveEntitlementWith let an unentitled caller through");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 402);
    assertEquals(await e.json(), { error: "this account has no active subscription" });
  }
});

Deno.test("an account with no entitlement row at all is 402, never 500", async () => {
  try {
    await requireActiveEntitlementWith(bearer, { verify, lookup: () => Promise.resolve(null) });
    throw new Error("a missing entitlement row was not refused");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 402);
  }
});

Deno.test("a bad token is 401 and never reaches the lookup", async () => {
  let looked = false;
  const stale = new Request("http://127.0.0.1:1/", { headers: { authorization: "Bearer stale" } });
  try {
    await requireActiveEntitlementWith(stale, {
      verify,
      lookup: () => {
        looked = true;
        return Promise.resolve(row("active"));
      },
    });
    throw new Error("a stale token was accepted");
  } catch (e) {
    if (!(e instanceof Response)) throw e;
    assertEquals(e.status, 401);
  }
  assertEquals(looked, false);
});
```

- [ ] **Step 2: Run and watch them fail.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `error: Module not found "file:///…/_shared/auth.ts"`.

- [ ] **Step 3: `cloud/supabase/functions/_shared/db.ts`.**

```ts
/**
 * Postgres and Auth over `fetch`, against Supabase's own REST and Auth APIs. **No SDK**: the whole
 * of what this codebase needs is six calls, and depending on `supabase-js` would put a registry
 * fetch in front of `deno check` and a moving surface under every handler.
 *
 * `fetch` is a field on `Rest`, not the global, so every test injects its own and no test in this
 * codebase can reach the network.
 */
import { AuthedUser } from "./auth.ts";
import { fail } from "./http.ts";

export interface Rest {
  /** `https://<ref>.supabase.co` — no trailing slash. */
  url: string;
  /** The service-role key. Bypasses RLS: this is the only writer any table has. */
  serviceKey: string;
  fetch: typeof fetch;
}

function headers(rest: Rest, extra: Record<string, string> = {}): Record<string, string> {
  return {
    apikey: rest.serviceKey,
    authorization: `Bearer ${rest.serviceKey}`,
    "content-type": "application/json",
    ...extra,
  };
}

async function ok(res: Response, what: string): Promise<void> {
  if (res.ok) return;
  // The upstream body can name a column, a constraint or a row; it never reaches the caller.
  console.error(`${what}: ${res.status} ${await res.text()}`);
  throw fail(502, `${what} failed`);
}

/** `query` is a PostgREST query string without the leading `?`, e.g. `account_id=eq.<id>&select=*`. */
export async function restSelect<T>(rest: Rest, table: string, query: string): Promise<T[]> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, { headers: headers(rest) });
  await ok(res, `select ${table}`);
  return await res.json() as T[];
}

export async function restUpsert(rest: Rest, table: string, rows: unknown[], onConflict?: string): Promise<void> {
  const q = onConflict ? `?on_conflict=${onConflict}` : "";
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}${q}`, {
    method: "POST",
    headers: headers(rest, { prefer: "resolution=merge-duplicates,return=minimal" }),
    body: JSON.stringify(rows),
  });
  await ok(res, `upsert ${table}`);
}

export async function restPatch(rest: Rest, table: string, query: string, patch: unknown): Promise<void> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, {
    method: "PATCH",
    headers: headers(rest, { prefer: "return=minimal" }),
    body: JSON.stringify(patch),
  });
  await ok(res, `patch ${table}`);
}

export async function restDelete(rest: Rest, table: string, query: string): Promise<void> {
  const res = await rest.fetch(`${rest.url}/rest/v1/${table}?${query}`, {
    method: "DELETE",
    headers: headers(rest, { prefer: "return=minimal" }),
  });
  await ok(res, `delete ${table}`);
}

/** GoTrue's `/auth/v1/user` with the caller's own token: this is what verifies a session. */
export async function authGetUser(rest: Rest, token: string): Promise<AuthedUser | null> {
  const res = await rest.fetch(`${rest.url}/auth/v1/user`, {
    headers: { apikey: rest.serviceKey, authorization: `Bearer ${token}` },
  });
  if (!res.ok) return null;
  const body = await res.json() as { id?: string; email?: string };
  return body.id ? { id: body.id, email: body.email ?? null } : null;
}

export async function authDeleteUser(rest: Rest, id: string): Promise<void> {
  const res = await rest.fetch(`${rest.url}/auth/v1/admin/users/${id}`, {
    method: "DELETE",
    headers: headers(rest),
  });
  await ok(res, "delete auth user");
}

/** The one place an `index.ts` builds a `Rest` from the platform's own variables. */
export function restFromEnv(): Rest {
  const url = Deno.env.get("SUPABASE_URL");
  const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY");
  if (!url || !serviceKey) throw fail(500, "the function is not configured");
  return { url: url.replace(/\/+$/, ""), serviceKey, fetch: globalThis.fetch };
}
```

- [ ] **Step 4: `cloud/supabase/functions/_shared/auth.ts`.**

```ts
/** Who is calling. Verification is injected, so every handler test runs with no network. */
import { fail } from "./http.ts";

export interface AuthedUser {
  id: string;
  email: string | null;
}

export type VerifyToken = (token: string) => Promise<AuthedUser | null>;

/** `Authorization: Bearer <token>`, case-insensitive on both the header and the scheme. */
export function parseBearer(req: Request): string | null {
  const h = req.headers.get("authorization");
  if (!h) return null;
  const m = /^bearer\s+(\S+)\s*$/i.exec(h.trim());
  return m ? m[1] : null;
}

/** **Throws a `Response`** (401) rather than returning an error: see `_shared/http.ts`. */
export async function requireUser(req: Request, verify: VerifyToken): Promise<AuthedUser> {
  const token = parseBearer(req);
  if (!token) throw fail(401, "no bearer token");
  const user = await verify(token);
  if (!user) throw fail(401, "the session is not valid");
  return user;
}
```

- [ ] **Step 5: `cloud/supabase/functions/_shared/entitlement.ts` — the C2 contract.**

```ts
/**
 * **The gate C2 imports.** `requireActiveEntitlement(req)` resolves to the caller's account id, or
 * **throws a `Response`**: 401 with no valid bearer token, **402** with no active or trialing
 * subscription. C2's handlers wrap it as
 *
 * ```ts
 * try { const { account_id } = await requireActiveEntitlement(req); … }
 * catch (e) { if (e instanceof Response) return e; throw e; }
 * ```
 *
 * The `…With` twin takes its dependencies explicitly and is what every test drives; the one-argument
 * form builds them from the platform's environment, once, and is what production calls. Splitting
 * them is what lets this file be tested with no environment, no database and no network.
 */
import { AuthedUser, requireUser, VerifyToken } from "./auth.ts";
import { authGetUser, Rest, restFromEnv, restSelect } from "./db.ts";
import { fail } from "./http.ts";

export type EntitlementStatus = "active" | "trialing" | "past_due" | "canceled" | "none";

/** The two statuses that mean "the cloud may work for this account". */
export const ACTIVE_STATUSES: readonly EntitlementStatus[] = ["active", "trialing"];

export interface EntitlementRow {
  plan: string | null;
  status: EntitlementStatus;
  current_period_end: string | null;
}

export type LookupEntitlement = (accountId: string) => Promise<EntitlementRow | null>;

export interface EntitlementDeps {
  verify: VerifyToken;
  lookup: LookupEntitlement;
}

export function isActive(row: EntitlementRow | null): boolean {
  return row !== null && ACTIVE_STATUSES.includes(row.status);
}

export async function requireActiveEntitlementWith(
  req: Request,
  deps: EntitlementDeps,
): Promise<{ account_id: string }> {
  const user: AuthedUser = await requireUser(req, deps.verify);
  const row = await deps.lookup(user.id);
  if (!isActive(row)) throw fail(402, "this account has no active subscription");
  return { account_id: user.id };
}

/** The production lookup: one PostgREST read with the service role. */
export function lookupFrom(rest: Rest): LookupEntitlement {
  return async (accountId: string) => {
    const rows = await restSelect<EntitlementRow>(
      rest,
      "entitlements",
      `account_id=eq.${encodeURIComponent(accountId)}&select=plan,status,current_period_end&limit=1`,
    );
    return rows[0] ?? null;
  };
}

export function depsFrom(rest: Rest): EntitlementDeps {
  return { verify: (token) => authGetUser(rest, token), lookup: lookupFrom(rest) };
}

let cached: EntitlementDeps | null = null;

export async function requireActiveEntitlement(req: Request): Promise<{ account_id: string }> {
  cached ??= depsFrom(restFromEnv());
  return await requireActiveEntitlementWith(req, cached);
}
```

- [ ] **Step 6: Run and watch them pass.**

Run: `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`
Expected: `ok | 21 passed | 0 failed`.

- [ ] **Step 7: Commit.**

```bash
git add cloud/supabase/functions/_shared/db.ts cloud/supabase/functions/_shared/auth.ts cloud/supabase/functions/_shared/entitlement.ts cloud/supabase/functions/_shared/auth_test.ts cloud/supabase/functions/_shared/entitlement_test.ts
git commit -F <message file>
```

Message: `cloud: the shared REST client, the bearer check and requireActiveEntitlement — the 402 contract C2 imports (C1 Task 2)`.

---
### Task 3: `GET /entitlement` — the reply the device caches

**Files:**
- Create: `cloud/supabase/functions/entitlement/handler.ts`, `cloud/supabase/functions/entitlement/index.ts`
- Test: `cloud/supabase/functions/entitlement/handler_test.ts`

**Interfaces:**
- Consumes: `requireUser`, `VerifyToken` (`_shared/auth.ts`); `EntitlementRow`, `LookupEntitlement`, `lookupFrom`, `depsFrom` (`_shared/entitlement.ts`); `json`, `methodNotAllowed`, `asResponse` (`_shared/http.ts`); `restFromEnv`, `authGetUser` (`_shared/db.ts`).
- Produces: `interface Deps { verify: VerifyToken; lookup: LookupEntitlement; now: () => Date }` and `handle(req, deps): Promise<Response>`; the wire contract `{status, current_period_end, plan, checked_at}` that `app/src/account.rs` parses in Task 11.

- [ ] **Step 1: Write the failing test** — `cloud/supabase/functions/entitlement/handler_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const NOW = new Date("2026-09-10T12:00:00.000Z");
const get = (auth?: string) =>
  new Request("http://127.0.0.1:1/entitlement", { headers: auth ? { authorization: auth } : {} });
const verify = (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null);

Deno.test("an entitled account gets the four keys the device caches, and no others", async () => {
  const res = await handle(get("Bearer good"), {
    verify,
    lookup: () =>
      Promise.resolve({ plan: "monthly", status: "active" as const, current_period_end: "2026-10-10T00:00:00+00:00" }),
    now: () => NOW,
  });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body, {
    status: "active",
    current_period_end: "2026-10-10T00:00:00+00:00",
    plan: "monthly",
    checked_at: "2026-09-10T12:00:00.000Z",
  });
  assertEquals(Object.keys(body).sort(), ["checked_at", "current_period_end", "plan", "status"]);
});

Deno.test("an account with no row is `none`, at 200 — never a 404 the device would read as offline", async () => {
  const res = await handle(get("Bearer good"), { verify, lookup: () => Promise.resolve(null), now: () => NOW });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), {
    status: "none",
    current_period_end: null,
    plan: null,
    checked_at: "2026-09-10T12:00:00.000Z",
  });
});

Deno.test("no bearer token is 401", async () => {
  const res = await handle(get(), { verify, lookup: () => Promise.resolve(null), now: () => NOW }).catch(
    (e) => e as Response,
  );
  assertEquals(res.status, 401);
});

Deno.test("anything but GET is 405 and never touches the lookup", async () => {
  let looked = false;
  const res = await handle(new Request("http://127.0.0.1:1/entitlement", { method: "POST" }), {
    verify,
    lookup: () => {
      looked = true;
      return Promise.resolve(null);
    },
    now: () => NOW,
  });
  assertEquals(res.status, 405);
  assertEquals(looked, false);
});
```

- [ ] **Step 2: Run and watch it fail.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/` → `Module not found "…/entitlement/handler.ts"`.

- [ ] **Step 3: `cloud/supabase/functions/entitlement/handler.ts`.**

```ts
/**
 * `GET /entitlement` — spec §5.1. The device calls this at launch and every six hours and caches the
 * answer with a 72-hour grace, so **an account with no subscription is a 200 saying `none`**, never a
 * 404: the app has to be able to tell "you are not subscribed" from "the network is not there", and
 * a status code is the only thing it can tell them apart by.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { EntitlementRow, LookupEntitlement } from "../_shared/entitlement.ts";
import { json, methodNotAllowed } from "../_shared/http.ts";

export interface Deps {
  verify: VerifyToken;
  lookup: LookupEntitlement;
  now: () => Date;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "GET") return methodNotAllowed(["GET"]);
  const user = await requireUser(req, deps.verify);
  const row: EntitlementRow | null = await deps.lookup(user.id);
  return json(200, {
    status: row?.status ?? "none",
    current_period_end: row?.current_period_end ?? null,
    plan: row?.plan ?? null,
    checked_at: deps.now().toISOString(),
  });
}
```

- [ ] **Step 4: `cloud/supabase/functions/entitlement/index.ts` — the thin wire.**

```ts
import { authGetUser, restFromEnv } from "../_shared/db.ts";
import { lookupFrom } from "../_shared/entitlement.ts";
import { asResponse } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      lookup: lookupFrom(rest),
      now: () => new Date(),
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 5: Run and watch them pass.** Expected: `ok | 25 passed | 0 failed`.

- [ ] **Step 6: Deploy to staging and prove the 401.**

```powershell
supabase functions deploy entitlement --project-ref <the STAGING ref> --workdir cloud/supabase
curl.exe -i "https://<staging ref>.supabase.co/functions/v1/entitlement"
```

Expected: `HTTP/2 401` and `{"error":"no bearer token"}`. That single call proves the deploy, the routing and the gate at once, and it needs no account.

- [ ] **Step 7: Commit.** `git add cloud/supabase/functions/entitlement/` then a message of `cloud: GET /entitlement — {status,current_period_end,plan,checked_at}, and no subscription is a 200 saying none (C1 Task 3)`.

---

### Task 4: Stripe — Checkout with the trial and the terms, the webhook as the only writer of `entitlements`, and the Portal

**Precondition P2** (the Stripe account, its two prices, the Portal, the webhook endpoint and the four secrets). Ask for it at step 1.

**Files:**
- Create: `cloud/supabase/functions/_shared/stripe.ts`, `cloud/supabase/functions/_shared/stripe_test.ts`
- Create: `cloud/supabase/functions/billing-checkout/{handler.ts,index.ts,handler_test.ts}`
- Create: `cloud/supabase/functions/stripe-webhook/{handler.ts,index.ts,handler_test.ts}`
- Create: `cloud/supabase/functions/billing-portal/{handler.ts,index.ts,handler_test.ts}`

**Interfaces:**
- Consumes: everything in `_shared/`.
- Produces:
  - `stripe.ts`: `STRIPE_API_VERSION`, `formEncode(params: Record<string, string | number | boolean | undefined | null>): string`, `hmacHex(secret, message): Promise<string>`, `parseStripeSignature(header): { t: number; v1: string[] } | null`, `verifyStripeSignature(payload, header, secret, nowSeconds, tolerance?): Promise<boolean>`, `type StripePost = (path: string, form: Record<string, string>) => Promise<Record<string, unknown>>`, `stripePostFrom(secret, fetchImpl): StripePost`, `type StripeGet = (path: string) => Promise<Record<string, unknown> | null>`, `stripeGetFrom(secret, fetchImpl): StripeGet`.
  - `billing-checkout/handler.ts`: `type Plan = "monthly" | "academic_year"`, `checkoutForm(args): Record<string, string>`, `handle(req, deps)`.
  - `stripe-webhook/handler.ts`: `entitlementFromSubscription(sub): { plan: string | null; status: EntitlementStatus; current_period_end: string | null }`, `accountIdFromEvent(event): string | null`, `handle(req, deps)`.

- [ ] **Step 1: Ask Quinn for P2.** One message: *"C1 is at the billing step. It needs a Stripe account in **test mode** with one product, **Knowlu**, and two prices: **$9.99/month** and **$69.99 per academic year** (a yearly recurring price). Turn the Customer Portal on with cancellation enabled and no cancellation survey — the law wants cancelling to be no harder than signing up. Then add a webhook endpoint pointing at `https://<staging ref>.supabase.co/functions/v1/stripe-webhook` subscribed to `checkout.session.completed`, `customer.subscription.created`, `customer.subscription.updated`, `customer.subscription.deleted` and `invoice.payment_failed`, **created at API version `2025-03-31.basil`** — the version the webhook parses. In the account's public business details set the **Terms of service URL** (and the privacy policy URL) to the site's pages: Checkout refuses `consent_collection[terms_of_service]=required` without the first, so every session would 502. Send me the two **price ids** and the **publishable key** — those are public. Set these yourself and never show me the values:*

```powershell
supabase secrets set STRIPE_SECRET_KEY --project-ref <ref>
supabase secrets set STRIPE_WEBHOOK_SECRET --project-ref <ref>
supabase secrets set EMAIL_API_KEY --project-ref <ref>
supabase secrets set SOURCES_ENC_KEY --project-ref <ref>
```

*`SOURCES_ENC_KEY` is 32 random bytes, base64 — generate it with `[Convert]::ToBase64String((1..32 | ForEach-Object { [byte](Get-Random -Max 256) }))` and paste it straight into the command. `EMAIL_API_KEY` is from an email provider (Resend's free tier is enough): California requires an annual renewal reminder for a monthly subscription and Stripe cannot send it. I also need three non-secret values: `STRIPE_PRICE_MONTHLY`, `STRIPE_PRICE_YEAR` and `EMAIL_FROM`."*

Record the non-secret values as function **environment variables** (also `supabase secrets set`, which is how the platform passes any variable) and write them nowhere else.

  **Then uncomment `[auth.email.smtp]` in `cloud/supabase/config.toml`** — Task 0 step 3 left the whole
  block behind `#` so the CLI would not have to validate four placeholders at Task 1 — and fill its
  four values from what Quinn sent. `pass = "env(SMTP_PASSWORD)"` stays as it is; the password is a
  secret he sets.

- [ ] **Step 2: Write the failing test** — `cloud/supabase/functions/_shared/stripe_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import {
  formEncode,
  hmacHex,
  parseStripeSignature,
  STRIPE_API_VERSION,
  stripeGetFrom,
  stripePostFrom,
  verifyStripeSignature,
} from "./stripe.ts";

Deno.test("formEncode drops nullish values and escapes the bracket keys Stripe uses", () => {
  assertEquals(
    formEncode({ "line_items[0][price]": "price_1", "line_items[0][quantity]": 1, mode: "subscription", nope: undefined, alsono: null }),
    "line_items%5B0%5D%5Bprice%5D=price_1&line_items%5B0%5D%5Bquantity%5D=1&mode=subscription",
  );
  assertEquals(formEncode({ "automatic_tax[enabled]": true }), "automatic_tax%5Benabled%5D=true");
});

Deno.test("parseStripeSignature reads the timestamp and every v1 signature", () => {
  assertEquals(parseStripeSignature("t=1700000000,v1=aaa,v1=bbb,v0=zzz"), { t: 1700000000, v1: ["aaa", "bbb"] });
  assertEquals(parseStripeSignature("v1=aaa"), null);
  assertEquals(parseStripeSignature("t=notanumber,v1=aaa"), null);
  assertEquals(parseStripeSignature(""), null);
});

Deno.test("a signature Stripe would have made verifies, and one byte off does not", async () => {
  // The secret here is a literal invented for this test and is not a credential of any account.
  const secret = "whsec_test_only_not_a_real_key";
  const payload = '{"id":"evt_1","type":"customer.subscription.updated"}';
  const t = 1_700_000_000;
  const sig = await hmacHex(secret, `${t}.${payload}`);
  assert(await verifyStripeSignature(payload, `t=${t},v1=${sig}`, secret, t + 10));
  assert(!await verifyStripeSignature(payload + " ", `t=${t},v1=${sig}`, secret, t + 10), "payload changed");
  assert(!await verifyStripeSignature(payload, `t=${t},v1=${sig.slice(0, -1)}0`, secret, t + 10), "signature changed");
  assert(!await verifyStripeSignature(payload, `t=${t},v1=${sig}`, secret, t + 400), "outside the tolerance");
  assert(!await verifyStripeSignature(payload, `t=${t},v1=${sig}`, "another_secret", t + 10), "wrong secret");
});

Deno.test("every Stripe call carries the API version this code parses", async () => {
  // P2 sets the WEBHOOK ENDPOINT's version, which governs the event payloads Stripe pushes. This is
  // the other half: the calls we make ourselves. The GET matters most — its body is what
  // `entitlementFromSubscription` reads, and unversioned it comes back under the account's own
  // default, where `current_period_end` is on neither shape the handler looks at.
  const seen: Array<[string, Headers]> = [];
  const fake: typeof fetch = (input, init) => {
    seen.push([String(input), new Headers(init?.headers)]);
    return Promise.resolve(
      new Response(JSON.stringify({ id: "sub_1" }), { status: 200, headers: { "content-type": "application/json" } }),
    );
  };
  // Not a credential of any account: `sk_test_` plus words, never sent anywhere by this test.
  const key = "sk_test_not_a_real_key";
  await stripePostFrom(key, fake)("/v1/customers", { email: "a@example.invalid" });
  await stripeGetFrom(key, fake)("/v1/subscriptions/sub_1");
  assertEquals(seen.length, 2);
  for (const [url, headers] of seen) {
    assertEquals(headers.get("stripe-version"), STRIPE_API_VERSION, url);
  }
  assertEquals(seen[1][0], "https://api.stripe.com/v1/subscriptions/sub_1");
});
```

- [ ] **Step 3: Run and watch it fail.** `Module not found "…/_shared/stripe.ts"`.

- [ ] **Step 4: `cloud/supabase/functions/_shared/stripe.ts`.**

```ts
/**
 * Stripe, with no SDK: form encoding, HMAC signature verification, and one `POST` helper. Every
 * pure part is exported so the tests can drive it without a key and without a request.
 */
import { fail } from "./http.ts";

export function formEncode(params: Record<string, string | number | boolean | undefined | null>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null) continue;
    p.append(k, String(v));
  }
  return p.toString();
}

export async function hmacHex(secret: string, message: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(message));
  return Array.from(new Uint8Array(sig)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function parseStripeSignature(header: string): { t: number; v1: string[] } | null {
  let t: number | null = null;
  const v1: string[] = [];
  for (const part of header.split(",")) {
    const [k, v] = part.split("=", 2);
    if (k === "t") {
      const n = Number(v);
      if (!Number.isFinite(n)) return null;
      t = n;
    } else if (k === "v1" && v) {
      v1.push(v);
    }
  }
  return t !== null && v1.length > 0 ? { t, v1 } : null;
}

/** Length-independent compare over the hex digests: neither string is a secret, but the habit is. */
function equalHex(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

/**
 * Stripe's scheme: HMAC-SHA256 over `<t>.<raw body>`, compared against every `v1` in the header,
 * inside a five-minute window. The window is what stops a captured webhook being replayed forever.
 */
export async function verifyStripeSignature(
  payload: string,
  header: string,
  secret: string,
  nowSeconds: number,
  toleranceSeconds = 300,
): Promise<boolean> {
  const parsed = parseStripeSignature(header);
  if (!parsed) return false;
  if (Math.abs(nowSeconds - parsed.t) > toleranceSeconds) return false;
  const expected = await hmacHex(secret, `${parsed.t}.${payload}`);
  return parsed.v1.some((got) => equalHex(got, expected));
}

export type StripePost = (path: string, form: Record<string, string>) => Promise<Record<string, unknown>>;

/**
 * The API version this code is written against, pinned. Stripe moves fields between objects at a
 * version boundary — `current_period_end` left the Subscription for `items.data[].current_period_end`
 * in `2025-03-31.basil` — and an account created today defaults well past that. Pinning means the
 * shape the handlers parse is the shape that arrives, and changing it is a deliberate edit with a
 * test beside it rather than a silent `null` in every entitlement row.
 */
export const STRIPE_API_VERSION = "2025-03-31.basil";

export function stripePostFrom(secret: string, fetchImpl: typeof fetch): StripePost {
  return async (path, form) => {
    const res = await fetchImpl(`https://api.stripe.com${path}`, {
      method: "POST",
      headers: {
        authorization: `Bearer ${secret}`,
        "content-type": "application/x-www-form-urlencoded",
        "stripe-version": STRIPE_API_VERSION,
      },
      body: formEncode(form),
    });
    const body = await res.json() as Record<string, unknown>;
    if (!res.ok) {
      // Stripe's error body can echo a customer email; it is logged, never returned.
      console.error(`stripe ${path}: ${res.status} ${JSON.stringify(body)}`);
      throw fail(502, "the payment provider refused the request");
    }
    return body;
  };
}

export type StripeGet = (path: string) => Promise<Record<string, unknown> | null>;

/**
 * The one `GET` this stream makes: the Subscription the webhook re-reads when an event carries an id
 * and nothing else. **Versioned exactly like the `POST`s** — this body is what
 * `entitlementFromSubscription` parses, and an unversioned read arrives under the account's own
 * default (well past `2025-03-31.basil`), where `current_period_end` is on neither the Subscription
 * nor the item the handler looks at, so every entitlement row would carry `null`.
 *
 * `null` on any non-2xx: a subscription we cannot read is an event to ignore, not a 500 — Stripe
 * retries a 500 and would replay it for days.
 */
export function stripeGetFrom(secret: string, fetchImpl: typeof fetch): StripeGet {
  return async (path) => {
    const res = await fetchImpl(`https://api.stripe.com${path}`, {
      headers: {
        authorization: `Bearer ${secret}`,
        "stripe-version": STRIPE_API_VERSION,
      },
    });
    return res.ok ? await res.json() as Record<string, unknown> : null;
  };
}
```

- [ ] **Step 5: Write the failing Checkout test** — `cloud/supabase/functions/billing-checkout/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { checkoutForm, handle } from "./handler.ts";

Deno.test("the Checkout form carries the trial, the card, the tax and the terms checkbox", () => {
  const form = checkoutForm({
    price: "price_monthly",
    customer: "cus_1",
    accountId: "acc-1",
    successUrl: "https://knowlu.com/subscribed.html",
    cancelUrl: "https://knowlu.com/index.html",
  });
  assertEquals(form["mode"], "subscription");
  assertEquals(form["line_items[0][price]"], "price_monthly");
  assertEquals(form["line_items[0][quantity]"], "1");
  assertEquals(form["customer"], "cus_1");
  assertEquals(form["client_reference_id"], "acc-1");
  assertEquals(form["subscription_data[metadata][account_id]"], "acc-1");
  // R2: seven days, and the card up front — a wall before the first session costs more than a week
  // of inference, and a trial with no card is a wall in a different place.
  assertEquals(form["subscription_data[trial_period_days]"], "7");
  assertEquals(form["payment_method_collection"], "always");
  // §9, sales tax: Stripe Tax decides Kentucky's 6%, and an address is what lets it.
  assertEquals(form["automatic_tax[enabled]"], "true");
  assertEquals(form["customer_update[address]"], "auto");
  assertEquals(form["billing_address_collection"], "required");
  // §8 of the legal note: express informed consent to the auto-renewal terms, its own checkbox.
  assertEquals(form["consent_collection[terms_of_service]"], "required");
  assertEquals(form["success_url"], "https://knowlu.com/subscribed.html");
  assertEquals(form["cancel_url"], "https://knowlu.com/index.html");
});

Deno.test("an account with no Stripe customer gets one, once, and it is saved", async () => {
  const calls: string[] = [];
  let saved: [string, string] | null = null;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null }),
      saveCustomerId: (a, c) => {
        saved = [a, c];
        return Promise.resolve();
      },
      recordConsent: () => Promise.resolve(),
      stripe: (path, form) => {
        calls.push(path);
        return Promise.resolve(path === "/v1/customers" ? { id: "cus_new" } : { id: "cs_1", url: "https://checkout.stripe.com/c/cs_1", ...form });
      },
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_1" });
  assertEquals(calls, ["/v1/customers", "/v1/checkout/sessions"]);
  assertEquals(saved, ["acc-1", "cus_new"]);
});

Deno.test("the auto-renew consent is logged with its version and the price, before the redirect", async () => {
  const consents: unknown[] = [];
  await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good", "x-forwarded-for": "203.0.113.7, 10.0.0.1" },
      body: JSON.stringify({ plan: "academic_year", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: (c) => {
        consents.push(c);
        return Promise.resolve();
      },
      stripe: () => Promise.resolve({ id: "cs_2", url: "https://checkout.stripe.com/c/cs_2" }),
      priceFor: (p) => (p === "academic_year" ? "price_year" : "price_monthly"),
      priceCentsFor: (p) => (p === "academic_year" ? 6999 : 999),
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(consents.length, 1);
  assertEquals(consents[0], {
    account_id: "acc-1",
    subject_email: "a@example.invalid",
    kind: "auto_renew",
    version: "2026-09-10",
    price_cents: 6999,
    ip: "203.0.113.7",
  });
});

Deno.test("an unknown plan is 400 and never reaches Stripe", async () => {
  let touched = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "lifetime", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => Promise.resolve(),
      stripe: () => {
        touched = true;
        return Promise.resolve({});
      },
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assert(!touched);
});
```

- [ ] **Step 6: `cloud/supabase/functions/billing-checkout/handler.ts`.**

```ts
/**
 * `POST /billing-checkout` → `{url}`, a hosted Stripe Checkout session the app opens in the SYSTEM
 * browser (spec §4.2 step 2). Hosted, not embedded, for one reason worth stating: a card form inside
 * our webview would make us the thing handling card data, and we have no reason to be.
 *
 * The consent row is written **before** the redirect, not after the webhook, because ROSCA and
 * California's ARL want a record of the consent that was shown — and a customer who abandons the
 * page still saw it. A duplicate row from a second attempt is harmless; a missing one is not.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export type Plan = "monthly" | "academic_year";

const PLANS: readonly Plan[] = ["monthly", "academic_year"];

export interface ConsentRecord {
  account_id: string;
  subject_email: string;
  kind: "auto_renew";
  version: string;
  price_cents: number;
  ip: string | null;
}

export interface Deps {
  verify: VerifyToken;
  getAccount: (accountId: string) => Promise<{ email: string; stripe_customer_id: string | null } | null>;
  saveCustomerId: (accountId: string, customerId: string) => Promise<void>;
  recordConsent: (c: ConsentRecord) => Promise<void>;
  stripe: StripePost;
  priceFor: (plan: Plan) => string;
  priceCentsFor: (plan: Plan) => number;
  successUrl: string;
  cancelUrl: string;
}

export function checkoutForm(a: {
  price: string;
  customer: string;
  accountId: string;
  successUrl: string;
  cancelUrl: string;
}): Record<string, string> {
  return {
    "mode": "subscription",
    "line_items[0][price]": a.price,
    "line_items[0][quantity]": "1",
    "customer": a.customer,
    "client_reference_id": a.accountId,
    "subscription_data[metadata][account_id]": a.accountId,
    // R2: a 7-day trial with the card taken up front.
    "subscription_data[trial_period_days]": "7",
    "payment_method_collection": "always",
    // Stripe Tax needs an address to decide Kentucky's 6% (legal note §8).
    "automatic_tax[enabled]": "true",
    "customer_update[address]": "auto",
    "billing_address_collection": "required",
    // The auto-renewal terms get their own checkbox, not a line buried in the ToS.
    "consent_collection[terms_of_service]": "required",
    "success_url": a.successUrl,
    "cancel_url": a.cancelUrl,
  };
}

/** The caller's address for the consent log. The first hop is the client; the rest are proxies. */
function clientIp(req: Request): string | null {
  const fwd = req.headers.get("x-forwarded-for");
  return fwd ? fwd.split(",")[0].trim() : null;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ plan?: string; terms_version?: string }>(req);
  const plan = body.plan as Plan;
  if (!PLANS.includes(plan)) throw fail(400, `unknown plan; use ${PLANS.join(" or ")}`);
  if (!body.terms_version) throw fail(400, "terms_version is required");

  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");

  let customer = account.stripe_customer_id;
  if (!customer) {
    const created = await deps.stripe("/v1/customers", {
      email: account.email,
      "metadata[account_id]": user.id,
    });
    customer = String(created.id);
    await deps.saveCustomerId(user.id, customer);
  }

  await deps.recordConsent({
    account_id: user.id,
    subject_email: account.email,
    kind: "auto_renew",
    version: body.terms_version,
    price_cents: deps.priceCentsFor(plan),
    ip: clientIp(req),
  });

  const session = await deps.stripe(
    "/v1/checkout/sessions",
    checkoutForm({
      price: deps.priceFor(plan),
      customer,
      accountId: user.id,
      successUrl: deps.successUrl,
      cancelUrl: deps.cancelUrl,
    }),
  );
  const url = session.url;
  if (typeof url !== "string") throw fail(502, "the payment provider returned no checkout link");
  return json(200, { url });
}
```

- [ ] **Step 7: `cloud/supabase/functions/billing-checkout/index.ts`.**

```ts
import { authGetUser, restFromEnv, restPatch, restSelect, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle, Plan } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const prices: Record<Plan, string> = {
      monthly: env("STRIPE_PRICE_MONTHLY"),
      academic_year: env("STRIPE_PRICE_YEAR"),
    };
    const cents: Record<Plan, number> = { monthly: 999, academic_year: 6999 };
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      getAccount: async (id) => {
        const rows = await restSelect<{ email: string; stripe_customer_id: string | null }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=email,stripe_customer_id&limit=1`,
        );
        return rows[0] ?? null;
      },
      saveCustomerId: (id, cid) =>
        restPatch(rest, "accounts", `id=eq.${encodeURIComponent(id)}`, { stripe_customer_id: cid }),
      recordConsent: async (c) => {
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(c.subject_email.toLowerCase()));
        const hash = Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, "0")).join("");
        await restUpsert(rest, "consents", [{
          account_id: c.account_id,
          subject_hash: hash,
          kind: c.kind,
          version: c.version,
          price_cents: c.price_cents,
          ip: c.ip,
        }]);
      },
      stripe: stripePostFrom(env("STRIPE_SECRET_KEY"), globalThis.fetch),
      priceFor: (p) => prices[p],
      priceCentsFor: (p) => cents[p],
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 8: Write the failing webhook test** — `cloud/supabase/functions/stripe-webhook/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { hmacHex } from "../_shared/stripe.ts";
import { accountIdFromEvent, entitlementFromSubscription, handle } from "./handler.ts";

const SECRET = "whsec_test_only_not_a_real_key";

Deno.test("a Stripe subscription becomes exactly the three fields the device reads", () => {
  assertEquals(
    entitlementFromSubscription({
      status: "trialing",
      current_period_end: 1_760_000_000,
      items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
    }),
    { plan: "monthly", status: "trialing", current_period_end: "2025-10-09T08:53:20.000Z" },
  );
  // A paused collection (the June-August window, R3) leaves Stripe's own status `active`, so the
  // student keeps the product through the summer and is simply not charged. That is the whole
  // mechanism, and this assertion is what stops a later refactor from "fixing" it.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      pause_collection: { behavior: "void" },
      current_period_end: 1_760_000_000,
      items: { data: [{ price: { id: "price_year", recurring: { interval: "year" } } }] },
    }).status,
    "active",
  );
  assertEquals(entitlementFromSubscription({ status: "unpaid", current_period_end: null, items: { data: [] } }).status, "past_due");
  assertEquals(entitlementFromSubscription({ status: "incomplete_expired", current_period_end: null, items: { data: [] } }).status, "canceled");
  assertEquals(entitlementFromSubscription({ status: "wat", current_period_end: null, items: { data: [] } }).status, "none");
});

Deno.test("the account id comes from the metadata, then the client reference, then nothing", () => {
  assertEquals(accountIdFromEvent({ data: { object: { metadata: { account_id: "acc-1" } } } }), "acc-1");
  assertEquals(accountIdFromEvent({ data: { object: { client_reference_id: "acc-2" } } }), "acc-2");
  assertEquals(accountIdFromEvent({ data: { object: {} } }), null);
});

Deno.test("an unsigned or badly signed webhook is 400 and writes nothing", async () => {
  let wrote = false;
  const body = JSON.stringify({ type: "customer.subscription.updated", data: { object: {} } });
  const res = await handle(new Request("http://127.0.0.1:1/", { method: "POST", body, headers: { "stripe-signature": "t=1,v1=deadbeef" } }), {
    secret: SECRET,
    nowSeconds: () => 1,
    writeEntitlement: () => {
      wrote = true;
      return Promise.resolve();
    },
    fetchSubscription: () => Promise.resolve(null),
  });
  assertEquals(res.status, 400);
  assert(!wrote, "a bad signature must never reach the write");
});

Deno.test("a signed subscription event writes the entitlement, once, keyed to the account", async () => {
  const written: unknown[] = [];
  const event = {
    type: "customer.subscription.updated",
    data: {
      object: {
        metadata: { account_id: "acc-1" },
        status: "active",
        current_period_end: 1_760_000_000,
        items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
      },
    },
  };
  const body = JSON.stringify(event);
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", body, headers: { "stripe-signature": `t=${t},v1=${sig}` } }),
    {
      secret: SECRET,
      nowSeconds: () => t,
      writeEntitlement: (accountId, row) => {
        written.push([accountId, row]);
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(written, [["acc-1", { plan: "monthly", status: "active", current_period_end: "2025-10-09T08:53:20.000Z" }]]);
});

Deno.test("an event type we do not handle is 200 and a no-op — Stripe must not retry it forever", async () => {
  const body = JSON.stringify({ type: "customer.created", data: { object: {} } });
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  let wrote = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", body, headers: { "stripe-signature": `t=${t},v1=${sig}` } }),
    { secret: SECRET, nowSeconds: () => t, writeEntitlement: () => { wrote = true; return Promise.resolve(); }, fetchSubscription: () => Promise.resolve(null) },
  );
  assertEquals(res.status, 200);
  assert(!wrote);
});
```

- [ ] **Step 9: `cloud/supabase/functions/stripe-webhook/handler.ts`.**

```ts
/**
 * **The only writer of `entitlements`** (spec §5.1). Nothing else in this codebase may write that
 * table, and no client can: RLS grants `select` and nothing more, and only the service role — which
 * this function holds and no browser does — bypasses it.
 *
 * The raw body is read as text and verified *before* it is parsed. Parsing first would mean acting
 * on a shape an attacker chose.
 */
import { EntitlementStatus } from "../_shared/entitlement.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { verifyStripeSignature } from "../_shared/stripe.ts";

// deno-lint-ignore no-explicit-any
type Json = any;

export interface EntitlementWrite {
  plan: string | null;
  status: EntitlementStatus;
  current_period_end: string | null;
}

export interface Deps {
  secret: string;
  nowSeconds: () => number;
  writeEntitlement: (accountId: string, row: EntitlementWrite) => Promise<void>;
  /** `checkout.session.completed` carries a subscription id, not the subscription. */
  fetchSubscription: (id: string) => Promise<Json | null>;
}

/** Stripe's status vocabulary is longer than ours; this is the whole of the mapping. */
function statusOf(stripeStatus: string): EntitlementStatus {
  switch (stripeStatus) {
    case "active":
      return "active";
    case "trialing":
      return "trialing";
    case "past_due":
    case "unpaid":
    case "incomplete":
      return "past_due";
    case "canceled":
    case "incomplete_expired":
      return "canceled";
    default:
      return "none";
  }
}

export function entitlementFromSubscription(sub: Json): EntitlementWrite {
  const interval = sub?.items?.data?.[0]?.price?.recurring?.interval ?? null;
  const plan = interval === "month" ? "monthly" : interval === "year" ? "academic_year" : null;
  const end = typeof sub?.current_period_end === "number"
    ? new Date(sub.current_period_end * 1000).toISOString()
    : typeof sub?.items?.data?.[0]?.current_period_end === "number" // `2025-03-31.basil` moved the field onto the item (R-C1-15)
    ? new Date(sub.items.data[0].current_period_end * 1000).toISOString()
    : null;
  // A `pause_collection` window (R3, June to August) leaves Stripe's own status `active`: the
  // student keeps the product and is not charged. Nothing here needs to know about the pause.
  return { plan, status: statusOf(String(sub?.status ?? "")), current_period_end: end };
}

export function accountIdFromEvent(event: Json): string | null {
  const o = event?.data?.object ?? {};
  return o?.metadata?.account_id ?? o?.client_reference_id ?? null;
}

const SUBSCRIPTION_EVENTS = new Set([
  "customer.subscription.created",
  "customer.subscription.updated",
  "customer.subscription.deleted",
]);

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const raw = await req.text();
  const header = req.headers.get("stripe-signature") ?? "";
  if (!await verifyStripeSignature(raw, header, deps.secret, deps.nowSeconds())) {
    return fail(400, "signature does not verify");
  }
  let event: Json;
  try {
    event = JSON.parse(raw);
  } catch {
    return fail(400, "body is not JSON");
  }

  const type = String(event?.type ?? "");
  // Every other event is acknowledged and dropped. A 4xx here makes Stripe retry for days, which
  // turns one unhandled type into a permanent alarm about nothing.
  if (!SUBSCRIPTION_EVENTS.has(type) && type !== "checkout.session.completed" && type !== "invoice.payment_failed") {
    return json(200, { ignored: type });
  }

  const accountId = accountIdFromEvent(event);
  if (!accountId) return json(200, { ignored: "no account id on the event" });

  let sub: Json | null = null;
  if (SUBSCRIPTION_EVENTS.has(type)) {
    sub = event.data.object;
  } else if (type === "checkout.session.completed") {
    const id = event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  } else if (type === "invoice.payment_failed") {
    const id = event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  }
  if (!sub) return json(200, { ignored: `no subscription on ${type}` });

  await deps.writeEntitlement(accountId, entitlementFromSubscription(sub));
  return json(200, { ok: true });
}
```

- [ ] **Step 10: `cloud/supabase/functions/stripe-webhook/index.ts`.**

```ts
import { restFromEnv, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripeGetFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = env("STRIPE_SECRET_KEY");
    return await handle(req, {
      secret: env("STRIPE_WEBHOOK_SECRET"),
      nowSeconds: () => Math.floor(Date.now() / 1000),
      writeEntitlement: (accountId, row) =>
        restUpsert(rest, "entitlements", [{
          account_id: accountId,
          plan: row.plan,
          status: row.status,
          current_period_end: row.current_period_end,
          source: "stripe",
          updated_at: new Date().toISOString(),
        }], "account_id"),
      // **Through `stripeGetFrom`, not a bare `fetch`**: it pins `Stripe-Version` to the version
      // `entitlementFromSubscription` is written against, which is the same version P2 sets on the
      // webhook endpoint. The id is path-encoded — it is Stripe's, but it arrives over the wire.
      fetchSubscription: (id) =>
        stripeGetFrom(stripeKey, globalThis.fetch)(`/v1/subscriptions/${encodeURIComponent(id)}`),
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 11: The Portal link** — `cloud/supabase/functions/billing-portal/handler.ts` and `index.ts`:

```ts
/**
 * `POST /billing-portal` → `{url}`. Stripe's own Customer Portal, with cancellation on and no
 * survey: California's ARL wants cancelling to take no more steps than signing up did, and the app's
 * *Cancel subscription* button is one click to here and one click there.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface Deps {
  verify: VerifyToken;
  getCustomerId: (accountId: string) => Promise<string | null>;
  stripe: StripePost;
  returnUrl: string;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const customer = await deps.getCustomerId(user.id);
  if (!customer) throw fail(409, "this account has never subscribed");
  const session = await deps.stripe("/v1/billing_portal/sessions", {
    customer,
    return_url: deps.returnUrl,
  });
  const url = session.url;
  if (typeof url !== "string") throw fail(502, "the payment provider returned no portal link");
  return json(200, { url });
}
```

```ts
import { authGetUser, restFromEnv, restSelect } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const key = Deno.env.get("STRIPE_SECRET_KEY");
    if (!key) throw fail(500, "the function is not configured");
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      getCustomerId: async (id) => {
        const rows = await restSelect<{ stripe_customer_id: string | null }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=stripe_customer_id&limit=1`,
        );
        return rows[0]?.stripe_customer_id ?? null;
      },
      stripe: stripePostFrom(key, globalThis.fetch),
      returnUrl: "https://knowlu.com/index.html",
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 11a: And its tests** — `cloud/supabase/functions/billing-portal/handler_test.ts`. The
  File-structure rule is one `deno test` file per handler, and this is the *Cancel subscription* path
  the terms promise (Task 19 §5) and the legal note leans on: **all four of its branches get an
  assertion** — the method check, the never-subscribed refusal, a provider answer with no link, and
  the happy path.

```ts
import { assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const verify = (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null);
const post = () =>
  new Request("http://127.0.0.1:1/billing-portal", {
    method: "POST",
    headers: { authorization: "Bearer good" },
  });
const never: (path: string, form: Record<string, string>) => Promise<Record<string, unknown>> = () =>
  Promise.reject(new Error("Stripe must not be called"));

Deno.test("a GET is 405 and says so in the header", async () => {
  const res = await handle(new Request("http://127.0.0.1:1/billing-portal"), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    stripe: never,
    returnUrl: "https://knowlu.com/index.html",
  });
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "POST");
});

Deno.test("an account that never subscribed is 409, and Stripe is not called", async () => {
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve(null),
    stripe: never,
    returnUrl: "https://knowlu.com/index.html",
  }).catch((e) => e as Response);
  assertEquals(res.status, 409);
  assertEquals(await res.json(), { error: "this account has never subscribed" });
});

Deno.test("a subscriber gets the portal link, and the return URL goes with it", async () => {
  // A list, not a `let x = null` the compiler narrows to `null`: the assignment happens inside a
  // callback, which TypeScript's flow analysis does not follow.
  const sent: Record<string, string>[] = [];
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    stripe: (path, form) => {
      assertEquals(path, "/v1/billing_portal/sessions");
      sent.push(form);
      return Promise.resolve({ url: "https://billing.stripe.com/p/session_1" });
    },
    returnUrl: "https://knowlu.com/index.html",
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { url: "https://billing.stripe.com/p/session_1" });
  // The return URL is the promise the ARL rests on: cancelling lands the student back in Knowlu.
  assertEquals(sent, [{ customer: "cus_1", return_url: "https://knowlu.com/index.html" }]);
});

Deno.test("a portal session with no url is 502, not a 200 carrying undefined", async () => {
  const res = await handle(post(), {
    verify,
    getCustomerId: () => Promise.resolve("cus_1"),
    // Stripe answered, but not with a link. Returning `{url: undefined}` would put the app's
    // *Cancel subscription* button through `open_in_browser(undefined)` and fail with nothing to say.
    stripe: () => Promise.resolve({ id: "bps_1" }),
    returnUrl: "https://knowlu.com/index.html",
  }).catch((e) => e as Response);
  assertEquals(res.status, 502);
  assertEquals(await res.json(), { error: "the payment provider returned no portal link" });
});
```

- [ ] **Step 12: Run every test.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/` → `ok | 42 passed | 0 failed` — seventeen this task adds: four in `_shared/stripe_test.ts`, four in `billing-checkout`, five in `stripe-webhook`, four in `billing-portal`.

- [ ] **Step 13: Deploy and prove it on staging.**

```powershell
supabase functions deploy billing-checkout --project-ref <ref> --workdir cloud/supabase
supabase functions deploy stripe-webhook --project-ref <ref> --workdir cloud/supabase
supabase functions deploy billing-portal --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X POST "https://<ref>.supabase.co/functions/v1/stripe-webhook" -H "stripe-signature: t=1,v1=00"
```

Expected: `HTTP/2 400` and `{"error":"signature does not verify"}`. Then, in the Stripe dashboard's webhook view, **Send test webhook** → `customer.subscription.updated`: the function logs `200` and `select * from entitlements` on staging shows no row (the test event carries no `account_id`, which is the `ignored` path — that is correct, not a failure).

- [ ] **Step 14: P3 — Stripe Tax.** Ask Quinn: *"Turn Stripe Tax on and add a **Kentucky** registration before the first Kentucky sale — Kentucky has taxed SaaS at 6% since 2023, and 200 monthly charges to UK students crosses the remote-seller threshold in the first month. For Alabama, get a written answer from a CPA: every tracker reads the Department as not taxing true SaaS, but no rule says so, and Knowlu is an Alabama business selling to Alabama residents, so it is a taxability question, not a nexus one."* Record his answer in this task, above step 1.

- [ ] **Step 15: Commit.** `git add cloud/supabase/functions/_shared/stripe.ts cloud/supabase/functions/_shared/stripe_test.ts cloud/supabase/functions/billing-checkout/ cloud/supabase/functions/stripe-webhook/ cloud/supabase/functions/billing-portal/` then `cloud: Stripe — hosted Checkout with the 7-day trial, the card, Tax and the terms checkbox; the webhook as the only writer of entitlements; the Portal link (C1 Task 4)`.

---

### Task 5: The two billing jobs — the June–August pause and the annual reminder

R3, ruled: **both** the academic-year price and the summer pause. A pause that halves summer revenue is only defensible if the student is told before the first charge comes back, so the pause and the reminder are one task.

**Files:**
- Create: `cloud/supabase/migrations/20260910000200_billing_jobs.sql`
- Create: `cloud/supabase/functions/billing-jobs/{handler.ts,index.ts,handler_test.ts}`
- Modify: `cloud/supabase/functions/stripe-webhook/{handler.ts,index.ts,handler_test.ts}` — steps 6, 6a and 6b widen the webhook for the three new columns, the invoice branch and the idempotency guard.

**Interfaces:**
- Consumes: `_shared/{http,db,stripe}.ts`.
- Produces: `shouldBePaused(d: Date): boolean`, `pauseDecision(d: Date, paused: boolean): "pause" | "resume" | "none"`, `reminderDue(a: { lastSentAt: string | null; startedAt: string; now: Date }): boolean`, `handle(req, deps)`. The table `public.billing_reminders`.

Applied to staging 2026-09-10. Step 1's migration needed its column adds ahead of its view (R-C1-34), and **step 9 as written cannot run**: Supabase refuses `alter database postgres set` to the `postgres` role (42501). Migration `20260910000600_billing_jobs_vault.sql` re-issues the cron job reading `vault.decrypted_secrets` instead (R-C1-36); Quinn's step is `select vault.create_secret('<the token>', 'billing_jobs_token');` per project, the same value as the function secret `BILLING_JOBS_TOKEN`. `pg_net with schema extensions` works.

- [ ] **Step 1: Write the failing test** — `cloud/supabase/functions/billing-jobs/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { handle, pauseDecision, reminderDue, shouldBePaused } from "./handler.ts";

Deno.test("June, July and August are the paused months, and nothing else is", () => {
  assertEquals(shouldBePaused(new Date("2027-05-31T23:59:59Z")), false);
  assertEquals(shouldBePaused(new Date("2027-06-01T00:00:00Z")), true);
  assertEquals(shouldBePaused(new Date("2027-07-15T00:00:00Z")), true);
  assertEquals(shouldBePaused(new Date("2027-08-31T23:59:59Z")), true);
  assertEquals(shouldBePaused(new Date("2027-09-01T00:00:00Z")), false);
});

Deno.test("the job only ever acts on the edge, never every day", () => {
  assertEquals(pauseDecision(new Date("2027-06-01T00:00:00Z"), false), "pause");
  assertEquals(pauseDecision(new Date("2027-07-01T00:00:00Z"), true), "none");
  assertEquals(pauseDecision(new Date("2027-09-01T00:00:00Z"), true), "resume");
  assertEquals(pauseDecision(new Date("2027-10-01T00:00:00Z"), false), "none");
});

Deno.test("an annual reminder is due a year after the last one, or a year after the start", () => {
  // California's ARL wants an annual reminder for a monthly subscription, stating the product, the
  // amount, the cadence and how to cancel.
  assert(reminderDue({ lastSentAt: null, startedAt: "2026-09-10T00:00:00Z", now: new Date("2027-09-10T00:00:01Z") }));
  assert(!reminderDue({ lastSentAt: null, startedAt: "2026-09-10T00:00:00Z", now: new Date("2027-09-09T00:00:00Z") }));
  assert(!reminderDue({ lastSentAt: "2027-09-10T00:00:00Z", startedAt: "2026-09-10T00:00:00Z", now: new Date("2027-10-10T00:00:00Z") }));
  assert(reminderDue({ lastSentAt: "2027-09-10T00:00:00Z", startedAt: "2026-09-10T00:00:00Z", now: new Date("2028-09-11T00:00:00Z") }));
});

Deno.test("the job refuses a caller with no job token, and writes nothing", async () => {
  let touched = false;
  const res = await handle(new Request("http://127.0.0.1:1/", { method: "POST" }), {
    token: "a-job-token",
    now: () => new Date("2027-06-01T00:00:00Z"),
    listSubscribers: () => {
      touched = true;
      return Promise.resolve([]);
    },
    stripe: () => Promise.resolve({}),
    sendEmail: () => Promise.resolve(),
    recordReminder: () => Promise.resolve(),
  });
  assertEquals(res.status, 401);
  assert(!touched);
});

Deno.test("on 1 June every MONTHLY subscription is paused, and the yearly one is not", async () => {
  const calls: [string, Record<string, string>][] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-06-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          { account_id: "acc-1", email: "a@example.invalid", subscription_id: "sub_1", plan: "monthly", paused: false, started_at: "2026-09-10T00:00:00Z", last_reminded_at: null },
          { account_id: "acc-2", email: "b@example.invalid", subscription_id: "sub_2", plan: "academic_year", paused: false, started_at: "2026-09-10T00:00:00Z", last_reminded_at: null },
        ]),
      stripe: (path, form) => {
        calls.push([path, form]);
        return Promise.resolve({ id: "sub_1" });
      },
      sendEmail: () => Promise.resolve(),
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  // One, not two: the academic-year subscriber is exempt, because that price already covers the
  // summer and voiding its renewal invoice would give away a year (R3).
  assertEquals(await res.json(), { paused: 1, resumed: 0, reminded: 0 });
  assertEquals(calls.map((c) => c[0]), ["/v1/subscriptions/sub_1"]);
  assertEquals(calls[0][1], { "pause_collection[behavior]": "void" });
});

Deno.test("on 1 September everything resumes and every resumed student is told before the charge", async () => {
  const mails: { to: string; subject: string; text: string }[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", headers: { "x-knowlu-job-token": "a-job-token" } }),
    {
      token: "a-job-token",
      now: () => new Date("2027-09-01T03:00:00Z"),
      listSubscribers: () =>
        Promise.resolve([
          { account_id: "acc-1", email: "a@example.invalid", subscription_id: "sub_1", plan: "monthly", paused: true, started_at: "2026-09-10T00:00:00Z", last_reminded_at: null },
        ]),
      stripe: () => Promise.resolve({ id: "sub_1" }),
      sendEmail: (m) => {
        mails.push(m);
        return Promise.resolve();
      },
      recordReminder: () => Promise.resolve(),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { paused: 0, resumed: 1, reminded: 1 });
  assertEquals(mails.length, 1);
  assertEquals(mails[0].to, "a@example.invalid");
  assert(mails[0].subject.includes("Knowlu"));
  assert(mails[0].text.includes("$9.99"), "the amount is in the reminder");
  assert(mails[0].text.includes("cancel"), "and so is how to cancel");
});
```

- [ ] **Step 2: Run and watch it fail.** `Module not found "…/billing-jobs/handler.ts"`.

- [ ] **Step 3: `cloud/supabase/functions/billing-jobs/handler.ts`.**

```ts
/**
 * The two things a subscription needs that neither Stripe nor a user does on their own:
 *
 * 1. **The June–August pause** (R3). Nobody else does this, and it halves summer revenue — which is
 *    exactly why it has to be visible: a `pause_collection` window that a student is not told about
 *    ending looks like a free-to-pay conversion to a regulator and like a surprise charge to them.
 *    So resuming always sends the notice, in the same pass.
 * 2. **The annual reminder** California's ARL requires for a monthly subscription: what it is, what
 *    it costs, how often, and how to cancel.
 *
 * Driven by `pg_cron` once a day, authenticated by a shared job token — not by a user's JWT, since
 * there is no user, and not by the service-role key, which has no business in a database's config.
 */
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface Subscriber {
  account_id: string;
  email: string;
  subscription_id: string;
  plan: string | null;
  paused: boolean;
  started_at: string;
  last_reminded_at: string | null;
}

export interface Mail {
  to: string;
  subject: string;
  text: string;
}

export interface Deps {
  token: string;
  now: () => Date;
  listSubscribers: () => Promise<Subscriber[]>;
  stripe: StripePost;
  sendEmail: (m: Mail) => Promise<void>;
  recordReminder: (accountId: string, at: string) => Promise<void>;
}

/** June, July and August, by UTC month. A summer that starts on a local date nobody agrees on is
 * worse than one that starts a few hours early for somebody. */
export function shouldBePaused(d: Date): boolean {
  const m = d.getUTCMonth();
  return m === 5 || m === 6 || m === 7;
}

export function pauseDecision(d: Date, paused: boolean): "pause" | "resume" | "none" {
  const want = shouldBePaused(d);
  if (want && !paused) return "pause";
  if (!want && paused) return "resume";
  return "none";
}

const YEAR_MS = 365 * 24 * 60 * 60 * 1000;

export function reminderDue(a: { lastSentAt: string | null; startedAt: string; now: Date }): boolean {
  const since = Date.parse(a.lastSentAt ?? a.startedAt);
  if (!Number.isFinite(since)) return false;
  return a.now.getTime() - since > YEAR_MS;
}

function annualReminder(s: Subscriber): Mail {
  const price = s.plan === "academic_year" ? "$69.99 per academic year" : "$9.99 per month";
  return {
    to: s.email,
    subject: "Your Knowlu subscription",
    text: [
      "This is your yearly reminder that you have a Knowlu subscription.",
      "",
      `Knowlu, ${price}, renewing automatically until you cancel.`,
      "",
      "To cancel, open Knowlu, click the gear, and choose Cancel subscription — or",
      "reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
}

function resumeNotice(s: Subscriber): Mail {
  const price = s.plan === "academic_year" ? "$69.99 per academic year" : "$9.99 per month";
  return {
    to: s.email,
    subject: "Knowlu billing starts again on 1 September",
    text: [
      "Knowlu does not bill over the summer, and the summer is over.",
      "",
      `Your subscription starts charging again: ${price}.`,
      "",
      "If you do not want it back, cancel before your next charge: open Knowlu,",
      "click the gear, and choose Cancel subscription.",
    ].join("\n"),
  };
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  if (req.headers.get("x-knowlu-job-token") !== deps.token) throw fail(401, "not a job caller");

  const now = deps.now();
  let paused = 0, resumed = 0, reminded = 0;

  for (const s of await deps.listSubscribers()) {
    // **Monthly only** (R3, as ruled). The academic-year price already prices the summer in — that is
    // most of why it exists — and `pause_collection[behavior] = "void"` on a yearly subscription
    // whose renewal invoice falls in June, July or August voids that invoice and gives away a year.
    // The annual reminder below still runs for every plan.
    if (s.plan !== "monthly") {
      if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
        await deps.sendEmail(annualReminder(s));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
      }
      continue;
    }
    switch (pauseDecision(now, s.paused)) {
      case "pause":
        await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, { "pause_collection[behavior]": "void" });
        paused++;
        break;
      case "resume":
        await deps.stripe(`/v1/subscriptions/${s.subscription_id}`, { "pause_collection": "" });
        resumed++;
        // Always, and in the same pass: the charge comes back and the student hears it first.
        await deps.sendEmail(resumeNotice(s));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
        continue;
      case "none":
        break;
    }
    if (reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now })) {
      await deps.sendEmail(annualReminder(s));
      await deps.recordReminder(s.account_id, now.toISOString());
      reminded++;
    }
  }

  return json(200, { paused, resumed, reminded });
}
```

- [ ] **Step 4: `cloud/supabase/functions/billing-jobs/index.ts`.**

```ts
import { restFromEnv, restSelect, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle, Subscriber } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = env("STRIPE_SECRET_KEY");
    return await handle(req, {
      token: env("BILLING_JOBS_TOKEN"),
      now: () => new Date(),
      listSubscribers: async () => {
        // One view read, not a join written here: `billing_subscribers` is the migration's view.
        return await restSelect<Subscriber>(rest, "billing_subscribers", "select=*");
      },
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
      sendEmail: async (m) => {
        const res = await fetch(env("EMAIL_API_URL"), {
          method: "POST",
          headers: { authorization: `Bearer ${env("EMAIL_API_KEY")}`, "content-type": "application/json" },
          body: JSON.stringify({ from: env("EMAIL_FROM"), to: m.to, subject: m.subject, text: m.text }),
        });
        if (!res.ok) {
          console.error(`email: ${res.status}`);
          throw fail(502, "the reminder could not be sent");
        }
      },
      recordReminder: (accountId, at) =>
        restUpsert(rest, "billing_reminders", [{ account_id: accountId, last_reminded_at: at }], "account_id"),
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 5: `cloud/supabase/migrations/20260910000200_billing_jobs.sql`.**

```sql
-- Knowlu C1, Task 5 — the state the two billing jobs need, and the daily tick that drives them.
create table public.billing_reminders (
  account_id       uuid primary key references public.accounts (id) on delete cascade,
  last_reminded_at timestamptz not null
);
alter table public.billing_reminders enable row level security;
create policy billing_reminders_select_own on public.billing_reminders
  for select to authenticated using (account_id = auth.uid());

-- Everything the job needs about one subscriber, in one read. A view rather than a join written
-- inside the function: the shape belongs with the tables, and PostgREST reads a view like a table.
create view public.billing_subscribers
with (security_invoker = true) as
select a.id                              as account_id,
       a.email                           as email,
       e.stripe_subscription_id          as subscription_id,
       e.plan                            as plan,
       coalesce(e.paused, false)         as paused,
       coalesce(e.started_at, a.created_at) as started_at,
       r.last_reminded_at                as last_reminded_at
from public.accounts a
join public.entitlements e on e.account_id = a.id
left join public.billing_reminders r on r.account_id = a.id
where e.status in ('active', 'trialing')
  and e.stripe_subscription_id is not null;

-- The webhook learns these from the subscription object; adding them here rather than in the first
-- migration keeps Task 1's table exactly the shape §5.1 names.
alter table public.entitlements add column stripe_subscription_id text;
alter table public.entitlements add column paused boolean not null default false;
alter table public.entitlements add column started_at timestamptz;

-- The daily tick. As written here it reads `app.billing_jobs_url` and `app.billing_jobs_token` from
-- database settings; **000600 and 000700 replace this job's command with one that reads Vault**
-- (R-C1-36, R-C1-38), because the `postgres` role cannot `alter database ... set` on Supabase — see
-- those migrations for the two `vault.create_secret` calls Quinn runs once per project.
-- A dedicated job token, deliberately, and not the service-role key: the worst a job token can do is
-- run this one function.
-- pg_cron is **not relocatable** and Supabase installs it into its own fixed schema, so a
-- `with schema` clause here fails the migration outright. pg_net is relocatable and lives in
-- `extensions`, which is where Supabase puts it.
create extension if not exists pg_cron;
create extension if not exists pg_net with schema extensions;

select cron.schedule(
  'knowlu-billing-jobs',
  '17 7 * * *',
  $$
  select net.http_post(
    url     := current_setting('app.billing_jobs_url', true),
    headers := jsonb_build_object(
                 'content-type', 'application/json',
                 'x-knowlu-job-token', current_setting('app.billing_jobs_token', true)),
    body    := '{}'::jsonb
  );
  $$
);
```

- [ ] **Step 6: Teach the webhook the three new columns.** In `cloud/supabase/functions/stripe-webhook/handler.ts`, widen `EntitlementWrite` and `entitlementFromSubscription`:

```ts
export interface EntitlementWrite {
  plan: string | null;
  status: EntitlementStatus;
  current_period_end: string | null;
  stripe_subscription_id: string | null;
  paused: boolean;
  started_at: string | null;
}
```

```ts
export function entitlementFromSubscription(sub: Json): EntitlementWrite {
  const item = sub?.items?.data?.[0] ?? null;
  const interval = item?.price?.recurring?.interval ?? null;
  const plan = interval === "month" ? "monthly" : interval === "year" ? "academic_year" : null;
  // **The period end moved.** In Stripe API `2025-03-31.basil` `current_period_end` left the
  // Subscription for each item; `_shared/stripe.ts` pins that version, and the item is read first so
  // the field is found on either shape rather than becoming a silent `null` in every row.
  const endSeconds = typeof item?.current_period_end === "number"
    ? item.current_period_end
    : (typeof sub?.current_period_end === "number" ? sub.current_period_end : null);
  const end = endSeconds === null ? null : new Date(endSeconds * 1000).toISOString();
  const started = typeof sub?.start_date === "number" ? new Date(sub.start_date * 1000).toISOString() : null;
  return {
    plan,
    status: statusOf(String(sub?.status ?? "")),
    current_period_end: end,
    stripe_subscription_id: typeof sub?.id === "string" ? sub.id : null,
    // A `pause_collection` window leaves Stripe's status `active`, so this flag — not the status —
    // is what the summer job reads to know whether it has already acted.
    paused: !!sub?.pause_collection,
    started_at: started,
  };
}
```

…rename `a Stripe subscription becomes exactly the three fields the device reads` to
`a Stripe subscription becomes exactly the six fields the row carries`, update
`stripe-webhook/handler_test.ts`'s two `entitlementFromSubscription` assertions to the six-key shape (add `stripe_subscription_id: null`, `paused: false`, `started_at: null` to the first, and `paused: true` to the paused case by giving that fixture `pause_collection`), **move both fixtures' `current_period_end` onto `items.data[0]`** and add a third asserting the pre-`basil` top-level shape still parses, and update `index.ts`'s upsert to carry the three new columns.

- [ ] **Step 6a: The invoice branch, which today is dead code.** `accountIdFromEvent` reads
  `data.object.metadata.account_id` or `.client_reference_id`. On an **Invoice**, `metadata` is the
  invoice's own — empty — and there is no `client_reference_id`, so every `invoice.payment_failed`
  returns `{ignored: "no account id on the event"}` before `fetchSubscription` is reached. It happens
  to be harmless (`customer.subscription.updated` fires with `status: past_due` too), but a promise
  that is not kept is worse than a promise not made. Widen the reader:

```ts
export function accountIdFromEvent(event: Json): string | null {
  const o = event?.data?.object ?? {};
  // In order: a subscription's own metadata, an invoice's subscription_details.metadata (current API
  // versions put it there — invoice metadata is the invoice's and is empty), then Checkout's
  // client_reference_id.
  return o?.metadata?.account_id ?? o?.subscription_details?.metadata?.account_id ?? o?.client_reference_id ?? null;
}
```

  …and add a test: an `invoice.payment_failed` carrying `subscription_details.metadata.account_id`
  reaches `fetchSubscription` and writes; one carrying neither is still `ignored`.

- [ ] **Step 6b: Idempotency and ordering.** Stripe retries for days and does not guarantee order.
  Two guards, both against `public.webhook_events` (migration `20260910000100`). `Deps` gains three:

```ts
export interface Deps {
  secret: string;
  nowSeconds: () => number;
  seenEvent: (eventId: string) => Promise<boolean>;
  recordEvent: (eventId: string, type: string, createdIso: string) => Promise<void>;
  /** `entitlements.updated_at` for this account, or null. It carries the EVENT's `created`. */
  currentUpdatedAt: (accountId: string) => Promise<string | null>;
  writeEntitlement: (accountId: string, row: EntitlementWrite, updatedAt: string) => Promise<void>;
  fetchSubscription: (id: string) => Promise<Json | null>;
}
```

  …and `handle`, after the signature check and the parse, gains four lines at the top and one guard
  before the write:

```ts
  const eventId = String(event?.id ?? "");
  if (!eventId) return fail(400, "the event has no id");
  if (await deps.seenEvent(eventId)) return json(200, { duplicate: eventId });
  const createdIso = new Date((Number(event?.created) || deps.nowSeconds()) * 1000).toISOString();
```

```ts
  // A late event must not resurrect a state a later one already replaced.
  const seen = await deps.currentUpdatedAt(accountId);
  if (seen !== null && seen >= createdIso) {
    await deps.recordEvent(eventId, type, createdIso);
    return json(200, { stale: eventId });
  }
  await deps.writeEntitlement(accountId, entitlementFromSubscription(sub), createdIso);
  await deps.recordEvent(eventId, type, createdIso);
  return json(200, { ok: true });
```

  Two more tests: the same signed event twice writes once (`duplicate`), and an event whose `created`
  precedes the row's `updated_at` writes nothing (`stale`). `index.ts` wires `seenEvent` and
  `recordEvent` to `restSelect`/`restUpsert` on `webhook_events`, `currentUpdatedAt` to a one-column
  `restSelect` on `entitlements`, and passes `updated_at: createdIso` in the upsert instead of
  `new Date().toISOString()`.

- [ ] **Step 7: Run every test.** Expected: `ok | 52 passed | 0 failed` — the six this task added, plus the four the webhook gained in steps 6, 6a and 6b.

- [ ] **Step 8: Apply and deploy to staging.**

```powershell
supabase db push --workdir cloud/supabase
supabase functions deploy billing-jobs --project-ref <ref> --workdir cloud/supabase
supabase functions deploy stripe-webhook --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X POST "https://<ref>.supabase.co/functions/v1/billing-jobs"
```

Expected: `Applying migration 20260910000200_billing_jobs.sql...`, then `HTTP/2 401` and `{"error":"not a job caller"}` — the job token is not on the wire, so the refusal is the proof.

- [ ] **Step 9 (amended by R-C1-36 and R-C1-38 — `alter database … set` is refused to the `postgres` role on Supabase): ask Quinn to put the job token into Vault**, the same random string they set as the function secret `BILLING_JOBS_TOKEN` (generated the way `SOURCES_ENC_KEY` is), in the project's SQL editor:

```sql
select vault.create_secret('https://<ref>.supabase.co/functions/v1/billing-jobs', 'billing_jobs_url');
select vault.create_secret('<the token>', 'billing_jobs_token');
```

  (Rotation is `vault.update_secret((select id from vault.secrets where name = 'billing_jobs_token'), '<new>')` plus `supabase secrets set` in the same sitting.) Then prove the tick, not just its row:

```sql
select jobname, schedule, command from cron.job where jobname = 'knowlu-billing-jobs';
select status, return_message, start_time from cron.job_run_details where jobname = 'knowlu-billing-jobs' order by start_time desc limit 3;
select status_code, error_msg from net._http_response order by created desc limit 3;
```

  → one job row whose command reads `vault.decrypted_secrets` and carries `raise exception` and `timeout_milliseconds := 60000`; after the next 07:17 UTC tick, a `succeeded` run and a `200` response. With a secret missing, the run is `failed` with `… billing_jobs_url or billing_jobs_token is missing` in `return_message` — proven on staging 2026-09-10.

- [ ] **Step 10: Commit.**

```bash
git add cloud/supabase/migrations/20260910000200_billing_jobs.sql cloud/supabase/functions/billing-jobs/ cloud/supabase/functions/stripe-webhook/
git commit -F <message file>
```

  Message: `cloud: the summer pause and the annual reminder — one daily job, and a resumed subscription always tells the student first; the webhook gains the three billing columns, the invoice branch and its idempotency guard (C1 Task 5)`.

---
### Task 6: `DELETE /account` and `GET /account/export` — the deletion and access rights, implemented for everyone

CalOPPA applies at any size and every state comprehensive law asks for the same four rights; building them universally is cheaper than geo-fencing and is what a privacy-sceptical student expects anyway (legal note §3). This is also the *Delete my data* action spec §4.1 names.

**Files:**
- Create: `cloud/supabase/migrations/20260910000300_deletion.sql`
- Create: `cloud/supabase/functions/account/{handler.ts,index.ts,handler_test.ts}`

**Interfaces:**
- Consumes: `subPath`, `json`, `fail`, `methodNotAllowed` (`_shared/http.ts`); `requireUser` (`_shared/auth.ts`); `StripePost` (`_shared/stripe.ts`); `Rest` and the four REST helpers (`_shared/db.ts`).
- Produces: `interface Deps` (below) and `handle(req, deps)`; the table `public.deleted_accounts`. Task 7 adds two routes to **this same handler**.

- [ ] **Step 1: `cloud/supabase/migrations/20260910000300_deletion.sql`.**

```sql
-- Knowlu C1, Task 6 — the tombstone a deleted account leaves behind.
-- Spec §5.1: "a tombstone (email_hash, deleted_at) kept 90 days for abuse control, then gone."
-- A hash, not an address: it answers "has this address deleted an account recently" and nothing else.
create table public.deleted_accounts (
  email_hash text primary key,
  deleted_at timestamptz not null default now()
);
alter table public.deleted_accounts enable row level security;
-- No policy at all: nothing but the service role has any business reading this table.

-- 90 days, swept daily by the same cron the billing jobs use.
select cron.schedule(
  'knowlu-tombstone-sweep',
  '41 7 * * *',
  $$ delete from public.deleted_accounts where deleted_at < now() - interval '90 days'; $$
);
```

- [ ] **Step 2: Write the failing test** — `cloud/supabase/functions/account/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { Deps, handle } from "./handler.ts";

const NOW = new Date("2026-09-10T12:00:00.000Z");

function deps(over: Partial<Deps> = {}): Deps {
  return {
    verify: (t) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null),
    requireEntitled: () => Promise.resolve({ account_id: "acc-1" }),
    getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
    getSubscriptionId: () => Promise.resolve("sub_1"),
    stripe: () => Promise.resolve({}),
    purge: () => Promise.resolve(),
    deleteAuthUser: () => Promise.resolve(),
    tombstone: () => Promise.resolve(),
    exportAll: () => Promise.resolve({ account: {}, entitlement: null, consents: [], sources: [], telemetry_events: [], corrections: [], issues: [] }),
    hashEmail: (e) => Promise.resolve(`hash(${e})`),
    getSources: () => Promise.resolve([]),
    putSource: () => Promise.resolve(),
    now: () => NOW,
    ...over,
  };
}

const req = (method: string, path: string, body?: unknown, auth = "Bearer good") =>
  new Request(`http://127.0.0.1:1/functions/v1/account${path}`, {
    method,
    headers: { authorization: auth },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

Deno.test("DELETE /account cancels at period end, purges, tombstones, and kills the login LAST", async () => {
  const order: string[] = [];
  const res = await handle(
    req("DELETE", ""),
    deps({
      stripe: (path, form) => {
        order.push(`stripe ${path} ${JSON.stringify(form)}`);
        return Promise.resolve({});
      },
      purge: () => {
        order.push("purge");
        return Promise.resolve();
      },
      tombstone: (h) => {
        order.push(`tombstone ${h}`);
        return Promise.resolve();
      },
      deleteAuthUser: () => {
        order.push("deleteAuthUser");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(order, [
    'stripe /v1/subscriptions/sub_1 {"cancel_at_period_end":"true"}',
    "purge",
    "tombstone hash(a@example.invalid)",
    "deleteAuthUser",
  ]);
});

Deno.test("DELETE /account on an account that never subscribed still deletes everything", async () => {
  let purged = false;
  const res = await handle(
    req("DELETE", ""),
    deps({
      getSubscriptionId: () => Promise.resolve(null),
      stripe: () => {
        throw new Error("Stripe must not be called when there is no subscription");
      },
      purge: () => {
        purged = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assert(purged);
});

Deno.test("GET /account/export hands back every table, keyed and complete", async () => {
  const res = await handle(req("GET", "/export"), deps());
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), [
    "account",
    "consents",
    "corrections",
    "entitlement",
    "exported_at",
    "issues",
    "sources",
    "telemetry_events",
  ]);
  assertEquals(body.exported_at, "2026-09-10T12:00:00.000Z");
});

Deno.test("an unknown path is 404 and an unknown method on a known path is 405", async () => {
  assertEquals((await handle(req("GET", "/nope"), deps())).status, 404);
  assertEquals((await handle(req("POST", "/export"), deps())).status, 405);
});

Deno.test("every route needs a bearer token", async () => {
  for (const [m, p] of [["DELETE", ""], ["GET", "/export"]] as const) {
    const res = await handle(req(m, p, undefined, "Basic nope"), deps()).catch((e) => e as Response);
    assertEquals(res.status, 401, `${m} ${p}`);
  }
});
```

- [ ] **Step 3: Run and watch it fail.** `Module not found "…/account/handler.ts"`.

- [ ] **Step 4: `cloud/supabase/functions/account/handler.ts`.** (Task 7 adds the two `/sources` routes to the switch; the `Deps` fields they need are already declared here so the two tasks do not fight over the type.)

```ts
/**
 * One function, four routes (spec §5.1 and §4.1):
 *
 *   DELETE /account         the deletion right, and the app's *Delete my data*
 *   GET    /account/export  the access and portability rights
 *   GET    /account/sources what sources are connected (Task 7) — never their URLs
 *   PUT    /account/sources connect one (Task 7)
 *
 * The order inside `DELETE` is the whole of its correctness: Stripe first (a cancelled card is
 * better than a deleted account still being billed), then the rows, then the tombstone, and the
 * login **last** — deleting the auth user first would invalidate the very token the rest of this
 * request is authenticated by.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, subPath } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface SourceRow {
  kind: string;
  added_at: string;
}

export interface Deps {
  verify: VerifyToken;
  /** The C2 gate, injected so this handler stays testable: `requireActiveEntitlement`. */
  requireEntitled: (req: Request) => Promise<{ account_id: string }>;
  getAccount: (accountId: string) => Promise<{ email: string; stripe_customer_id: string | null } | null>;
  getSubscriptionId: (accountId: string) => Promise<string | null>;
  stripe: StripePost;
  /** Rows in sources, telemetry_events, corrections, issues, entitlements, accounts; consents nulled. */
  purge: (accountId: string) => Promise<void>;
  deleteAuthUser: (accountId: string) => Promise<void>;
  tombstone: (emailHash: string, at: string) => Promise<void>;
  exportAll: (accountId: string) => Promise<Record<string, unknown>>;
  hashEmail: (email: string) => Promise<string>;
  getSources: (accountId: string) => Promise<SourceRow[]>;
  putSource: (accountId: string, kind: string, url: string) => Promise<void>;
  now: () => Date;
}

async function deleteAccount(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");

  const sub = await deps.getSubscriptionId(user.id);
  if (sub) {
    // At period end, not immediately: the student paid for this month and deleting is not a refund.
    await deps.stripe(`/v1/subscriptions/${sub}`, { cancel_at_period_end: "true" });
  }
  await deps.purge(user.id);
  await deps.tombstone(await deps.hashEmail(account.email), deps.now().toISOString());
  await deps.deleteAuthUser(user.id);
  return json(200, { deleted: true });
}

async function exportAccount(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  const all = await deps.exportAll(user.id);
  return json(200, { ...all, exported_at: deps.now().toISOString() });
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  const path = subPath(req.url, "account");
  switch (path) {
    case "/":
      if (req.method !== "DELETE") return methodNotAllowed(["DELETE"]);
      return await deleteAccount(req, deps);
    case "/export":
      if (req.method !== "GET") return methodNotAllowed(["GET"]);
      return await exportAccount(req, deps);
    default:
      return fail(404, `no route ${path}`);
  }
}
```

- [ ] **Step 5: `cloud/supabase/functions/account/index.ts`.**

```ts
import { authDeleteUser, authGetUser, restDelete, restFromEnv, restPatch, restSelect, restUpsert } from "../_shared/db.ts";
import { requireActiveEntitlement } from "../_shared/entitlement.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

const eq = (id: string) => `account_id=eq.${encodeURIComponent(id)}`;

async function sha256Hex(s: string): Promise<string> {
  const d = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s.toLowerCase()));
  return Array.from(new Uint8Array(d)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = Deno.env.get("STRIPE_SECRET_KEY");
    if (!stripeKey) throw fail(500, "the function is not configured");
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      requireEntitled: requireActiveEntitlement,
      getAccount: async (id) => {
        const rows = await restSelect<{ email: string; stripe_customer_id: string | null }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=email,stripe_customer_id&limit=1`,
        );
        return rows[0] ?? null;
      },
      getSubscriptionId: async (id) => {
        const rows = await restSelect<{ stripe_subscription_id: string | null }>(
          rest,
          "entitlements",
          `${eq(id)}&select=stripe_subscription_id&limit=1`,
        );
        return rows[0]?.stripe_subscription_id ?? null;
      },
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
      purge: async (id) => {
        // The consent log survives, with its account_id nulled: California's ARL wants the record
        // for three years, and `subject_hash` is what keeps it meaningful without identifying.
        await restPatch(rest, "consents", eq(id), { account_id: null });
        for (const table of ["sources", "telemetry_events", "corrections", "issues", "billing_reminders", "entitlements"]) {
          await restDelete(rest, table, eq(id));
        }
        await restDelete(rest, "accounts", `id=eq.${encodeURIComponent(id)}`);
      },
      deleteAuthUser: (id) => authDeleteUser(rest, id),
      tombstone: (hash, at) => restUpsert(rest, "deleted_accounts", [{ email_hash: hash, deleted_at: at }], "email_hash"),
      exportAll: async (id) => {
        const one = async <T>(table: string, q: string) => await restSelect<T>(rest, table, q);
        return {
          account: (await one("accounts", `id=eq.${encodeURIComponent(id)}&select=*`))[0] ?? null,
          entitlement: (await one("entitlements", `${eq(id)}&select=*`))[0] ?? null,
          consents: await one("consents", `${eq(id)}&select=*`),
          // Kinds and dates, not the URL — and the comment now says what the code does. The access
          // right does cover the capability URL, but an export is a file that ends up in a downloads
          // folder, and a feed link is a password: the student can always re-copy it from their own
          // LMS, which is where it came from. `GET /account/sources` returns the same two columns.
          sources: await one("sources", `${eq(id)}&select=kind,added_at`),
          telemetry_events: await one("telemetry_events", `${eq(id)}&select=*`),
          corrections: await one("corrections", `${eq(id)}&select=*`),
          issues: await one("issues", `${eq(id)}&select=*`),
        };
      },
      hashEmail: sha256Hex,
      getSources: async (id) => await restSelect(rest, "sources", `${eq(id)}&select=kind,added_at`),
      putSource: () => Promise.reject(fail(501, "not yet")),
      now: () => new Date(),
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 6: Run every test.** Expected: `ok | 57 passed | 0 failed`.

- [ ] **Step 7: Apply and deploy, then prove it.**

```powershell
supabase db push --workdir cloud/supabase
supabase functions deploy account --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X DELETE "https://<ref>.supabase.co/functions/v1/account"
curl.exe -i "https://<ref>.supabase.co/functions/v1/account/nope"
```

Expected: `401 {"error":"no bearer token"}` and `404 {"error":"no route /nope"}`.

  **Neither route is exercised with a real token yet, and must not be.** `purge` deletes from
  `telemetry_events`, `corrections` and `issues`, and `exportAll` selects from all three — tables that
  arrive in Task 8's and Task 9's migrations. A real `DELETE /account` here would 502 on a missing
  relation. **Re-run this check with a real token at Task 9 step 8**, once `20260910000500` is applied.

- [ ] **Step 8: Commit.** `cloud: DELETE /account and GET /account/export — cancel at period end, purge, tombstone for ninety days, kill the login last (C1 Task 6)`.

---

### Task 7: `PUT /account/sources` — the LMS calendar URL, encrypted, server-side

Spec §3.1 and the C2 contract: the URL moves to the account so C2's `/ingest/ics` can fetch it, and it **stays in `config/ingest.yaml` as well** until C2 ships, because on-device `ingest` is what closes the judgment gap. This is also the one C1 endpoint that goes through `requireActiveEntitlement` — connecting a source is a paid feature, and exercising the gate here is what proves the contract C2 imports actually works in production.

**Files:**
- Create: `cloud/supabase/functions/_shared/crypto.ts`, `cloud/supabase/functions/_shared/crypto_test.ts`
- Modify: `cloud/supabase/functions/account/{handler.ts,index.ts,handler_test.ts}`

**Interfaces:**
- Consumes: Task 6's `Deps`, which already declares `getSources` and `putSource`.
- Produces: `importAesKey(base64: string): Promise<CryptoKey>`, `encryptString(key, plaintext): Promise<{ ciphertext: string; iv: string }>`, `decryptString(key, ciphertext, iv): Promise<string>`.

- [ ] **Step 1: Write the failing crypto test** — `cloud/supabase/functions/_shared/crypto_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { decryptString, encryptString, importAesKey } from "./crypto.ts";

// 32 zero bytes, base64. A fixed key in a test is not a credential of anything.
const KEY_B64 = btoa(String.fromCharCode(...new Uint8Array(32)));

Deno.test("a string round-trips through AES-GCM", async () => {
  const key = await importAesKey(KEY_B64);
  const box = await encryptString(key, "https://lms.example.invalid/feed/abc.ics");
  assertEquals(await decryptString(key, box.ciphertext, box.iv), "https://lms.example.invalid/feed/abc.ics");
});

Deno.test("the same plaintext encrypts differently every time — the IV is random", async () => {
  const key = await importAesKey(KEY_B64);
  const a = await encryptString(key, "same");
  const b = await encryptString(key, "same");
  assert(a.ciphertext !== b.ciphertext);
  assert(a.iv !== b.iv);
});

Deno.test("a tampered ciphertext does not decrypt — GCM authenticates", async () => {
  const key = await importAesKey(KEY_B64);
  const box = await encryptString(key, "https://lms.example.invalid/feed/abc.ics");
  const flipped = box.ciphertext.slice(0, -2) + (box.ciphertext.endsWith("AA") ? "AB" : "AA");
  let threw = false;
  try {
    await decryptString(key, flipped, box.iv);
  } catch {
    threw = true;
  }
  assert(threw, "a tampered ciphertext decrypted");
});

Deno.test("a key that is not 32 bytes is refused, by name", async () => {
  let threw = false;
  try {
    await importAesKey(btoa("short"));
  } catch (e) {
    threw = true;
    assert(String(e).includes("32 bytes"));
  }
  assert(threw);
});
```

- [ ] **Step 2: Run and watch it fail.** `Module not found "…/_shared/crypto.ts"`.

- [ ] **Step 3: `cloud/supabase/functions/_shared/crypto.ts`.**

```ts
/**
 * AES-256-GCM over one short string. The only thing this system stores server-side that is a secret
 * of the user's rather than of ours is the LMS capability URL — anyone holding it can read that
 * student's whole schedule — so it is encrypted before it reaches Postgres and the key lives in the
 * function's environment (`SOURCES_ENC_KEY`), not in the database.
 *
 * GCM, not CBC: the tag is what makes a tampered row fail to decrypt instead of decrypting to
 * something else.
 */
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

function bytesToB64(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}

export async function importAesKey(base64Key: string): Promise<CryptoKey> {
  const raw = b64ToBytes(base64Key);
  if (raw.length !== 32) throw new Error("SOURCES_ENC_KEY must be 32 bytes, base64");
  return await crypto.subtle.importKey("raw", raw, { name: "AES-GCM" }, false, ["encrypt", "decrypt"]);
}

export async function encryptString(key: CryptoKey, plaintext: string): Promise<{ ciphertext: string; iv: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const box = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, new TextEncoder().encode(plaintext));
  return { ciphertext: bytesToB64(new Uint8Array(box)), iv: bytesToB64(iv) };
}

export async function decryptString(key: CryptoKey, ciphertext: string, iv: string): Promise<string> {
  const plain = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: b64ToBytes(iv) },
    key,
    b64ToBytes(ciphertext),
  );
  return new TextDecoder().decode(plain);
}
```

- [ ] **Step 4: Add the two route tests** to `cloud/supabase/functions/account/handler_test.ts`:

```ts
Deno.test("PUT /account/sources stores an https .ics link and answers with the kind only", async () => {
  let stored: [string, string, string] | null = null;
  const res = await handle(
    req("PUT", "/sources", { kind: "lms_ics", url: "https://lms.example.invalid/feed/abc.ics" }),
    deps({
      putSource: (a, k, u) => {
        stored = [a, k, u];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { kind: "lms_ics" });
  assertEquals(stored, ["acc-1", "lms_ics", "https://lms.example.invalid/feed/abc.ics"]);
});

Deno.test("PUT /account/sources refuses google_calendar from a client — that row is C2's to write", async () => {
  let stored = false;
  const res = await handle(
    req("PUT", "/sources", { kind: "google_calendar", url: "https://calendar.google.com/x.ics" }),
    deps({
      putSource: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  ).catch((e) => e as Response);
  assertEquals(res.status, 403);
  assert(!stored, "a client must not be able to forge the kind C2's callback writes");
});

Deno.test("PUT /account/sources needs an ACTIVE subscription — the 402 C2 imports", async () => {
  let stored = false;
  const res = await handle(
    req("PUT", "/sources", { kind: "lms_ics", url: "https://lms.example.invalid/feed/abc.ics" }),
    deps({
      requireEntitled: () => Promise.reject(new Response(JSON.stringify({ error: "this account has no active subscription" }), { status: 402 })),
      putSource: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  ).catch((e) => e as Response);
  assertEquals(res.status, 402);
  assert(!stored);
});

Deno.test("PUT /account/sources takes a personal calendar too, under its own kind", async () => {
  let stored: [string, string, string] | null = null;
  const res = await handle(
    req("PUT", "/sources", { kind: "calendar_ics", url: "https://calendar.google.com/calendar/ical/abc%40group.calendar.google.com/private-def/basic.ics" }),
    deps({
      putSource: (a, k, u) => {
        stored = [a, k, u];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { kind: "calendar_ics" });
  assertEquals(stored![1], "calendar_ics");
  // The secret address is a capability URL like the school one, and the reply does not echo it.
  assertEquals(Object.keys(await (await handle(
    req("PUT", "/sources", { kind: "calendar_ics", url: "https://calendar.google.com/calendar/ical/x/private-def/basic.ics" }),
    deps(),
  )).json()), ["kind"]);
});

Deno.test("PUT /account/sources refuses a URL that is not https, and an unknown kind", async () => {
  for (const body of [
    { kind: "lms_ics", url: "http://lms.example.invalid/feed/abc.ics" },
    { kind: "lms_ics", url: "file:///c:/x.ics" },
    { kind: "gradebook", url: "https://lms.example.invalid/x.ics" },
    { kind: "calendar_ics", url: "webcal://calendar.google.com/x.ics" },
    { kind: "lms_ics", url: "https://" + "a".repeat(3000) },
  ]) {
    const res = await handle(req("PUT", "/sources", body), deps()).catch((e) => e as Response);
    assertEquals(res.status, 400, JSON.stringify(body));
  }
});

Deno.test("GET /account/sources returns kinds and dates, and never the URL", async () => {
  const res = await handle(
    req("GET", "/sources"),
    deps({ getSources: () => Promise.resolve([{ kind: "lms_ics", added_at: "2026-09-10T00:00:00+00:00" }]) }),
  );
  assertEquals(res.status, 200);
  const text = await res.text();
  assertEquals(JSON.parse(text), { sources: [{ kind: "lms_ics", added_at: "2026-09-10T00:00:00+00:00" }] });
  assert(!text.includes("http"), "a URL reached the response body");
});
```

- [ ] **Step 5: Add the two routes** to `cloud/supabase/functions/account/handler.ts`. First widen its `http.ts` import to `import { fail, json, methodNotAllowed, readJson, subPath } from "../_shared/http.ts";` — Task 6 left `readJson` out because an unused import fails `deno lint`. Then a new pair of functions and two `case`s:

```ts
/** The whole `sources.kind` vocabulary, matching the check constraint in `20260910000100_accounts.sql`
 * and `SOURCE_KINDS` in `app/src/lms_link.rs` — three copies, pinned to each other by
 * `app/tests/lms_link.rs::the_source_kind_vocabulary_is_one_list_in_three_places`.
 *
 * C1's wizard writes `lms_ics` and `calendar_ics` (spec §11a, one panel, both calendars).
 * `google_calendar` is a reserved value nobody writes (R-X-9): a Google grant has no URL and lives in
 * C2's `google_accounts`. It is listed here from the start so C2 never has to edit a C1-owned file. */
const SOURCE_KINDS = ["lms_ics", "calendar_ics", "google_calendar"];
const MAX_URL = 2048;

async function putSource(req: Request, deps: Deps): Promise<Response> {
  // The gate C2 imports, exercised here in C1 so the contract is proved by something that ships.
  const { account_id } = await deps.requireEntitled(req);
  const body = await readJson<{ kind?: string; url?: string }>(req);
  const kind = String(body.kind ?? "");
  const url = String(body.url ?? "");
  if (!SOURCE_KINDS.includes(kind)) throw fail(400, `unknown source kind; use ${SOURCE_KINDS.join(", ")}`);
  // `google_calendar` is a reserved kind **nobody writes** (R-X-9): a Google grant has no URL and
  // lives in C2's `google_accounts`, so no row of this kind ever exists here; the value is in the
  // vocabulary only so the constraint never needs a C2 migration.
  // The device is already narrowed by `lms_link::DEVICE_KINDS`; this is the same rule on the end that
  // a patched client actually talks to, so a forged row cannot become an iCal URL C2 then fetches.
  if (kind === "google_calendar") throw fail(403, "that calendar is connected by signing in, not by pasting a link");
  if (!url.startsWith("https://")) throw fail(400, "the feed link must start with https://");
  if (url.length > MAX_URL) throw fail(400, `the feed link is longer than ${MAX_URL} characters`);
  await deps.putSource(account_id, kind, url);
  // The kind, and nothing else: the reply must not echo a capability URL back over the wire.
  return json(200, { kind });
}

async function getSources(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  return json(200, { sources: await deps.getSources(user.id) });
}
```

```ts
    case "/sources":
      if (req.method === "PUT") return await putSource(req, deps);
      if (req.method === "GET") return await getSources(req, deps);
      return methodNotAllowed(["GET", "PUT"]);
```

- [ ] **Step 6: Wire `putSource` in `cloud/supabase/functions/account/index.ts`** — replace the `Promise.reject(fail(501, …))` placeholder:

```ts
      putSource: async (id, kind, url) => {
        const keyB64 = Deno.env.get("SOURCES_ENC_KEY");
        if (!keyB64) throw fail(500, "the function is not configured");
        const box = await encryptString(await importAesKey(keyB64), url);
        await restUpsert(rest, "sources", [{
          account_id: id,
          kind,
          url_ciphertext: box.ciphertext,
          url_iv: box.iv,
          added_at: new Date().toISOString(),
        }], "account_id,kind");
      },
```

…with `import { encryptString, importAesKey } from "../_shared/crypto.ts";` at the top.

- [ ] **Step 7: Run every test.** Expected: `ok | 67 passed | 0 failed` (the four crypto tests and six route tests this task adds — the last two being the `calendar_ics` kind §11a introduced and the refusal of `google_calendar` from a client).

- [ ] **Step 8: Deploy and prove the 402 on staging.**

```powershell
supabase functions deploy account --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X PUT "https://<ref>.supabase.co/functions/v1/account/sources" -H "content-type: application/json" -d "{\"kind\":\"lms_ics\",\"url\":\"https://x.invalid/a.ics\"}"
```

Expected: `401 {"error":"no bearer token"}` — the gate is reached before the body is looked at, which is the ordering the test above pins.

- [ ] **Step 9: Commit.** `cloud: PUT/GET /account/sources — the LMS capability URL encrypted at rest, never returned, and behind the entitlement gate C2 imports (C1 Task 7)`.

---

### Task 8: `POST /telemetry` — (a) interaction events and (b) corrections, with the cohort rule in the schema

Spec §6 and D5. Two rules do the work: **(a) and (b) carry no free text and no content**, and **no dashboard slice under ten accounts is readable** — the second is a view with a `having` clause, so it cannot be forgotten by whoever writes the dashboard later.

**Files:**
- Create: `cloud/supabase/migrations/20260910000400_telemetry.sql`
- Create: `cloud/supabase/functions/telemetry/{handler.ts,index.ts,handler_test.ts}`
- Modify: `cloud/supabase/migrations_test.ts` — step 2 adds the minimum-cohort assertion.

**Interfaces:**
- Consumes: `_shared/{http,auth,db}.ts`.
- Produces: `ACTIONS: readonly string[]` (the eleven of `engine/src/uievents.rs`, in that order), `VALUED_FIELDS`, `FLAGGED_FIELDS`, `isToken(s): boolean`, `handle(req, deps)`. `app/src/telemetry.rs` (Task 15) posts exactly this body, and `app/tests/telemetry.rs` pins the two lists against this file.

- [ ] **Step 1: `cloud/supabase/migrations/20260910000400_telemetry.sql`.**

```sql
-- Knowlu C1, Task 8 — analytics (a) and (b). Spec §6, decided under D5.
--
-- Two properties are structural, not procedural:
--   * `object_id` is NOT NULL DEFAULT '' so the dedup key is a plain unique constraint. A retry after
--     a dropped connection re-sends the same rows, and the second copy must land on the first.
--   * The reading view refuses any slice under ten accounts. A minimum cohort that lives in a
--     dashboard query is a minimum cohort somebody forgets; this one is in the schema.
create table public.telemetry_events (
  id          bigint generated always as identity primary key,
  account_id  uuid not null references public.accounts (id) on delete cascade,
  ts          timestamptz not null,
  session     text not null,
  view        text not null,
  action      text not null,
  object_id   text not null default '',
  object_kind text,
  ms          integer,
  received_at timestamptz not null default now(),
  constraint telemetry_events_once unique (account_id, session, ts, action, object_id)
);

create table public.corrections (
  id          bigint generated always as identity primary key,
  account_id  uuid not null references public.accounts (id) on delete cascade,
  ts          timestamptz not null,
  item_id     text not null,
  field       text not null,
  -- **Nullable, and they stay nullable.** A `course` correction deliberately carries neither value
  -- (spec §6's content rule), and C2's eval suite reads these rows — R-X-2 fixes this shape as the
  -- authority: C2 alters this table, it never creates it, and it adds no check constraint to `kind`.
  ours        text,
  theirs      text,
  -- The note kind, from the note's own folder: task | approval | course | info | issue | archive.
  kind        text not null,
  -- **R-X-3.** The judged item as it was sent, so C2's eval suite can replay a correction as a
  -- labelled example. Present **only** under the class-(c) opt-in — which C1 does not build, so this
  -- is always null here — and **never** for a judgment whose origin was Gmail (`origin = gmail_api`),
  -- which Google's Limited Use forbids being used for anything but that user's own rules.
  request     jsonb,
  received_at timestamptz not null default now(),
  constraint corrections_once unique (account_id, ts, item_id, field)
);
comment on table public.corrections is
  'Spec §6(b) and §5.4: every row is a labelled example for the eval suite. `ours` and `theirs` carry a
   value only for closed-vocabulary and numeric fields; a course name or a title is never here.';

alter table public.telemetry_events enable row level security;
alter table public.corrections enable row level security;
create policy telemetry_events_select_own on public.telemetry_events
  for select to authenticated using (account_id = auth.uid());
create policy corrections_select_own on public.corrections
  for select to authenticated using (account_id = auth.uid());

-- The only shape a dashboard may read. `count(distinct account_id) >= 10` is the product plan's
-- minimum cohort (§7), and it is here rather than in a query so it cannot be dropped by accident.
create view public.telemetry_daily
with (security_invoker = false) as
select date_trunc('day', ts) as day,
       view,
       action,
       count(*)                    as events,
       count(distinct account_id)  as accounts
from public.telemetry_events
group by 1, 2, 3
having count(distinct account_id) >= 10;

create view public.correction_rates
with (security_invoker = false) as
select date_trunc('week', ts) as week,
       kind,
       field,
       count(*)                   as corrections,
       count(distinct account_id) as accounts
from public.corrections
group by 1, 2, 3
having count(distinct account_id) >= 10;

revoke all on public.telemetry_daily from anon, authenticated;
revoke all on public.correction_rates from anon, authenticated;
```

- [ ] **Step 2: Extend the migration test.** Add to `cloud/supabase/migrations_test.ts`:

```ts
Deno.test("no reporting view may be read below the minimum cohort", async () => {
  const sql = (await migrations()).map((m) => m.sql).join("\n").toLowerCase();
  const views = [...sql.matchAll(/create\s+view\s+public\.(\w+)/g)].map((m) => m[1]);
  for (const v of views) {
    if (v === "billing_subscribers") continue; // an operational read of one row per subscriber, not a slice
    const body = sql.split(`create view public.${v}`)[1].split(";")[0];
    assert(
      body.includes("count(distinct account_id) >= 10"),
      `view ${v} has no minimum cohort — product plan §7, spec §6`,
    );
  }
});
```

- [ ] **Step 3: Write the failing handler test** — `cloud/supabase/functions/telemetry/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { ACTIONS, handle, VALUED_FIELDS } from "./handler.ts";

const req = (body: unknown, auth = "Bearer good") =>
  new Request("http://127.0.0.1:1/telemetry", { method: "POST", headers: { authorization: auth }, body: JSON.stringify(body) });

const base = {
  verify: (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null),
  saveEvents: () => Promise.resolve(),
  saveCorrections: () => Promise.resolve(),
};

Deno.test("the action vocabulary is the engine's eleven, in the engine's order", () => {
  assertEquals(ACTIONS, [
    "view_opened",
    "object_seen",
    "edit_started",
    "edit_committed",
    "edit_cancelled",
    "decision_made",
    "decision_deferred",
    "issue_opened",
    "sync_run",
    "delta_expanded",
    "why_expanded",
  ]);
});

Deno.test("a good batch is accepted and counted", async () => {
  let events = 0, corrections = 0;
  const res = await handle(
    req({
      events: [{ ts: "2026-09-10T12:00:00.000Z", session: "sess_1", view: "today", action: "object_seen", object_id: "task_0123456789", object_kind: "task", ms: 2400 }],
      corrections: [{ ts: "2026-09-10T12:01:00.000Z", item_id: "task_0123456789", field: "effort_hours", ours: "2.0", theirs: "0.5", kind: "task" }],
    }),
    {
      ...base,
      saveEvents: (rows) => {
        events = rows.length;
        return Promise.resolve();
      },
      saveCorrections: (rows) => {
        corrections = rows.length;
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 1, corrections: 1 });
  assertEquals([events, corrections], [1, 1]);
});

Deno.test("an unknown action is refused by name, and the whole batch with it", async () => {
  const res = await handle(
    req({ events: [{ ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "typed_a_title" }], corrections: [] }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assert(String((await res.json()).error).includes("typed_a_title"));
});

Deno.test("free text is refused wherever it could hide", async () => {
  const bad = [
    { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "object_seen", object_id: "Read chapter 3" },
    { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "Calculus II — Today", action: "view_opened" },
    { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "object_seen", object_kind: "a task about the midterm" },
  ];
  for (const e of bad) {
    const res = await handle(req({ events: [e], corrections: [] }), base).catch((x) => x as Response);
    assertEquals(res.status, 400, JSON.stringify(e));
  }
});

Deno.test("a correction on a content field carries no value — spec §6's rule, enforced here too", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(
    req({
      events: [],
      corrections: [
        { ts: "2026-09-10T12:00:00.000Z", item_id: "task_0123456789", field: "course", ours: "MATH 125", theirs: "SPAN 101", kind: "task" },
      ],
    }),
    { ...base, saveCorrections: (rows) => { saved = rows as Record<string, unknown>[]; return Promise.resolve(); } },
  );
  assertEquals(res.status, 200);
  // The row is kept — that a course was corrected is the signal — but the two names are not.
  assertEquals(saved[0].ours, null);
  assertEquals(saved[0].theirs, null);
  assert(!VALUED_FIELDS.includes("course"));
  assert(!VALUED_FIELDS.includes("title"));
});

Deno.test("a correction on a field that is neither valued nor flagged is refused", async () => {
  const res = await handle(
    req({ events: [], corrections: [{ ts: "2026-09-10T12:00:00.000Z", item_id: "task_1", field: "title", ours: "a", theirs: "b", kind: "task" }] }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

Deno.test("an oversized batch is refused rather than truncated", async () => {
  const one = { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "view_opened" };
  const res = await handle(req({ events: new Array(501).fill(one), corrections: [] }), base).catch((e) => e as Response);
  assertEquals(res.status, 400);
});
```

- [ ] **Step 4: Run and watch it fail.** `Module not found "…/telemetry/handler.ts"`.

- [ ] **Step 5: `cloud/supabase/functions/telemetry/handler.ts`.**

```ts
/**
 * `POST /telemetry` — spec §6, classes (a) and (b), collected under the terms.
 *
 * The device has already filtered this; refusing again here is not distrust of the device, it is the
 * only place the rule holds for a client somebody else wrote. Three refusals, all 400:
 *   * an action outside `engine/src/uievents.rs`'s eleven;
 *   * anything that is not a token where an id or a view name belongs (that is where a title would
 *     hide);
 *   * a correction on a field that is neither closed-vocabulary nor explicitly flagged.
 *
 * And one silent narrowing that is deliberate: a correction on a **content** field keeps its row and
 * loses its values. "The course was corrected" is the signal the eval suite needs; "from MATH 125 to
 * SPAN 101" is somebody's timetable.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";

/** `engine/src/uievents.rs`'s `ACTIONS`, in its order. `app/tests/telemetry.rs` pins the two lists
 * against each other, so a new action added to the engine fails a Rust test until it lands here. */
export const ACTIONS: readonly string[] = [
  "view_opened",
  "object_seen",
  "edit_started",
  "edit_committed",
  "edit_cancelled",
  "decision_made",
  "decision_deferred",
  "issue_opened",
  "sync_run",
  "delta_expanded",
  "why_expanded",
];

/** Judged fields whose correction may carry its values: numbers and closed vocabularies only. */
export const VALUED_FIELDS: readonly string[] = ["effort_hours", "importance", "domain", "effort_confidence", "status"];
/** Judged fields whose correction is recorded as "it changed" and nothing more: a course is content. */
export const FLAGGED_FIELDS: readonly string[] = ["course"];

const MAX_ROWS = 500;

/**
 * `uievents::is_token`'s character class: alphanumeric, `_-:`, at most 64 characters, never empty.
 *
 * **Looser than the emitter for `object_id`, deliberately.** The engine checks `object_id` against
 * `ids::is_id` (`<prefix>_<hex>`), which is stricter; this end checks the token class, because the
 * property that matters here is "this is not free text" and a stricter check on a field whose format
 * the engine may extend would refuse rows for a reason that is not about privacy. `session` and
 * `view` are exactly the engine's rule.
 */
export function isToken(s: unknown): boolean {
  return typeof s === "string" && s.length > 0 && s.length <= 64 && /^[A-Za-z0-9_:-]+$/.test(s);
}

function isTimestamp(s: unknown): boolean {
  return typeof s === "string" && Number.isFinite(Date.parse(s));
}

export interface EventIn {
  ts: string;
  session: string;
  view: string;
  action: string;
  object_id?: string | null;
  object_kind?: string | null;
  ms?: number | null;
}

export interface CorrectionIn {
  ts: string;
  item_id: string;
  field: string;
  ours?: string | null;
  theirs?: string | null;
  kind: string;
  /** R-X-3: the judged item as sent, for C2's eval suite. Accepted **only** when the batch also
   * carries `opt_in_raw: true` — the class-(c) opt-in, which C1 does not build — and dropped
   * silently otherwise, so a client that sends it by mistake cannot make it content we hold. */
  request?: Record<string, unknown> | null;
}

export interface Deps {
  verify: VerifyToken;
  saveEvents: (rows: unknown[]) => Promise<void>;
  saveCorrections: (rows: unknown[]) => Promise<void>;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ events?: EventIn[]; corrections?: CorrectionIn[]; opt_in_raw?: boolean }>(req);
  const events = body.events ?? [];
  const corrections = body.corrections ?? [];
  // R-X-3. C1 ships no way to turn this on, so it is always false in practice; the field exists so
  // the rule is written down in the one place a row is created rather than promised in a document.
  const optInRaw = body.opt_in_raw === true;
  if (events.length > MAX_ROWS || corrections.length > MAX_ROWS) {
    throw fail(400, `at most ${MAX_ROWS} rows of each kind per batch`);
  }

  const eventRows = events.map((e) => {
    if (!ACTIONS.includes(e.action)) throw fail(400, `unknown action ${JSON.stringify(e.action)}`);
    if (!isTimestamp(e.ts)) throw fail(400, "an event has no usable ts");
    if (!isToken(e.session) || !isToken(e.view)) throw fail(400, "session and view must be tokens, not text");
    if (e.object_id != null && !isToken(e.object_id)) throw fail(400, "object_id must be an id, not text");
    if (e.object_kind != null && !isToken(e.object_kind)) throw fail(400, "object_kind must be a token, not text");
    return {
      account_id: user.id,
      ts: e.ts,
      session: e.session,
      view: e.view,
      action: e.action,
      object_id: e.object_id ?? "",
      object_kind: e.object_kind ?? null,
      ms: typeof e.ms === "number" ? Math.trunc(e.ms) : null,
    };
  });

  const correctionRows = corrections.map((c) => {
    if (!isTimestamp(c.ts)) throw fail(400, "a correction has no usable ts");
    if (!isToken(c.item_id) || !isToken(c.kind)) throw fail(400, "item_id and kind must be tokens, not text");
    const valued = VALUED_FIELDS.includes(c.field);
    const flagged = FLAGGED_FIELDS.includes(c.field);
    if (!valued && !flagged) throw fail(400, `field ${JSON.stringify(c.field)} is not a judged field`);
    return {
      account_id: user.id,
      ts: c.ts,
      item_id: c.item_id,
      field: c.field,
      ours: valued ? (c.ours ?? null) : null,
      theirs: valued ? (c.theirs ?? null) : null,
      kind: c.kind,
      request: optInRaw ? (c.request ?? null) : null,
    };
  });

  if (eventRows.length) await deps.saveEvents(eventRows);
  if (correctionRows.length) await deps.saveCorrections(correctionRows);
  return json(200, { events: eventRows.length, corrections: correctionRows.length });
}
```

- [ ] **Step 6: `cloud/supabase/functions/telemetry/index.ts`.**

```ts
import { authGetUser, restFromEnv, restUpsert } from "../_shared/db.ts";
import { asResponse } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      // A resent batch lands on the rows it landed on the first time: the unique constraints are
      // the dedup, and `merge-duplicates` is what makes a retry free.
      saveEvents: (rows) => restUpsert(rest, "telemetry_events", rows, "account_id,session,ts,action,object_id"),
      saveCorrections: (rows) => restUpsert(rest, "corrections", rows, "account_id,ts,item_id,field"),
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 7: Run every test.** Expected: `ok | 75 passed | 0 failed`.

- [ ] **Step 8: Apply, deploy, prove.**

```powershell
supabase db push --workdir cloud/supabase
supabase functions deploy telemetry --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X POST "https://<ref>.supabase.co/functions/v1/telemetry" -H "content-type: application/json" -d "{}"
```

Expected: `401 {"error":"no bearer token"}`. Then in the SQL editor: `select * from public.telemetry_daily;` → zero rows (the cohort floor, working on an empty table).

- [ ] **Step 9: Commit.** `cloud: POST /telemetry — the engine's eleven actions, corrections with no title and no course name, and a minimum cohort of ten in the schema (C1 Task 8)`.

---

### Task 9: `POST /issues` — the report, scrubbed twice

Legal note §9: show the payload, redact by default, let the user edit, keep it briefly. The device does the showing (Task 16); this end does the keeping — and scrubs again, because a report is the one payload a human will read.

**Files:**
- Create: `cloud/supabase/migrations/20260910000500_issues.sql`
- Create: `cloud/supabase/functions/_shared/scrub.ts`, `cloud/supabase/functions/_shared/scrub_test.ts`
- Create: `cloud/supabase/functions/issues/{handler.ts,index.ts,handler_test.ts}`

**Interfaces:**
- Consumes: `_shared/{http,auth,db}.ts`.
- Produces: `scrub(text: string): string`, `scrubJson(v: unknown): unknown`, `handle(req, deps)`. `app/src/report.rs` (Task 16) has a Rust twin of `scrub`, and `app/tests/report.rs` drives both against the same seven cases.

- [ ] **Step 1: `cloud/supabase/migrations/20260910000500_issues.sql`.**

```sql
-- Knowlu C1, Task 9 — issue reports. Legal note §9: kept 90 days, under the same access control as
-- everything else, and never called "anonymous" — every row carries an account id.
create table public.issues (
  id           uuid primary key default extensions.gen_random_uuid(),
  account_id   uuid not null references public.accounts (id) on delete cascade,
  created_at   timestamptz not null default now(),
  app_version  text,
  engine_build text,
  os_build     text,
  profile_id   text,
  body         text not null,
  payload      jsonb not null default '{}'::jsonb
);
alter table public.issues enable row level security;
create policy issues_select_own on public.issues
  for select to authenticated using (account_id = auth.uid());

select cron.schedule(
  'knowlu-issue-sweep',
  '53 7 * * *',
  $$ delete from public.issues where created_at < now() - interval '90 days'; $$
);
```

- [ ] **Step 2: Write the failing scrub test** — `cloud/supabase/functions/_shared/scrub_test.ts`:

```ts
import { assertEquals } from "@std/assert";
import { scrub, scrubJson } from "./scrub.ts";

Deno.test("the seven things a report must never carry", () => {
  // An address — somebody else's, usually.
  assertEquals(scrub("mailed a.student@crimson.ua.edu twice"), "mailed <email> twice");
  // A capability URL: the LMS feed link is a password with a scheme in front of it.
  assertEquals(scrub("fetching https://lms.example.invalid/feed/abc123.ics failed"), "fetching <url> failed");
  assertEquals(scrub("http://10.0.0.1/x"), "<url>");
  // A bearer token, and a JWT in particular.
  assertEquals(scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"), "authorization: Bearer <token>");
  // A note's filename is its title, slugified.
  assertEquals(scrub("1 unreadable: read-chapter-3-of-calculus.md)"), "1 unreadable: <note>)");
  // A long opaque run of key-ish characters.
  assertEquals(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<token>");
  // The Windows account name, which is usually a person's name.
  assertEquals(
    scrub("could not open C:\\Users\\Ada\\Knowlu\\Fall 2026\\tasks\\a.md"),
    "could not open C:\\Users\\<user>\\Knowlu\\Fall 2026\\tasks\\<note>",
  );
  // …and ordinary prose survives intact.
  assertEquals(scrub("the run at 12:00 exited 1 after 4 steps"), "the run at 12:00 exited 1 after 4 steps");
});

Deno.test("scrubJson reaches every string, at any depth, and leaves the shape alone", () => {
  assertEquals(
    scrubJson({ a: "see https://x.invalid/y", b: [{ c: "me@x.invalid" }], d: 3, e: null, f: true }),
    { a: "see <url>", b: [{ c: "<email>" }], d: 3, e: null, f: true },
  );
});
```

- [ ] **Step 3: `cloud/supabase/functions/_shared/scrub.ts`.**

```ts
/**
 * The redaction an issue report goes through. The device runs its own twin (`app/src/report.rs`) and
 * **shows the result to the user before anything is sent** — that preview is the documented consent
 * a human needs to read Gmail-derived text at all (Google's Limited Use, spec §5.3). This end runs it
 * again, because a report is the one payload a person opens and a client is not the only way to post.
 *
 * Order matters: URLs before tokens, or a URL's path segment reads as a token and the sentence loses
 * the shape a reader needs.
 */
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
const URL_RE = /\bhttps?:\/\/\S+/g;
// The Windows account name is usually a person's name, and the engine's stdout is full of paths that
// carry it. Only that one segment is replaced — the rest of the path is what makes a log readable.
const WINUSER = /(:\\Users\\)[^\\/\s"']+/gi;
// `.md` before the token rule, because a note's filename would match both and it is the more
// specific fact. Dots are inside the token class so a JWT is one token rather than three.
const NOTE = /\b[\w.-]+\.md\b/g;
const TOKEN = /\b[A-Za-z0-9_.-]{20,}\b/g;

export function scrub(text: string): string {
  return text
    .replace(EMAIL, "<email>")
    .replace(URL_RE, "<url>")
    .replace(WINUSER, "$1<user>")
    .replace(NOTE, "<note>")
    .replace(TOKEN, "<token>");
}

export function scrubJson(v: unknown): unknown {
  if (typeof v === "string") return scrub(v);
  if (Array.isArray(v)) return v.map(scrubJson);
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) out[k] = scrubJson(val);
    return out;
  }
  return v;
}
```

- [ ] **Step 4: Write the failing handler test** — `cloud/supabase/functions/issues/handler_test.ts`:

```ts
import { assert, assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const post = (body: unknown, auth = "Bearer good") =>
  new Request("http://127.0.0.1:1/issues", { method: "POST", headers: { authorization: auth }, body: JSON.stringify(body) });

const base = {
  verify: (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null),
  save: () => Promise.resolve("11111111-1111-1111-1111-111111111111"),
};

Deno.test("a report is stored and its id comes back", async () => {
  const res = await handle(post({ body: "the run list says amber and I do not know why", payload: { steps: 4 } }), base);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { id: "11111111-1111-1111-1111-111111111111" });
});

Deno.test("whatever the client sent, the stored row is scrubbed again", async () => {
  let stored: Record<string, unknown> | null = null;
  await handle(
    post({
      body: "it broke while fetching https://lms.example.invalid/feed/abc.ics for a.b@x.invalid",
      payload: { last_error: "could not open read-chapter-3.md", token: "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef" },
      app_version: "0.1.0",
    }),
    { ...base, save: (row) => { stored = row as Record<string, unknown>; return Promise.resolve("id-1"); } },
  );
  assert(stored);
  assertEquals(stored!.body, "it broke while fetching <url> for <email>");
  assertEquals((stored!.payload as Record<string, unknown>).last_error, "could not open <note>");
  assertEquals((stored!.payload as Record<string, unknown>).token, "<token>");
  assertEquals(stored!.account_id, "acc-1");
});

Deno.test("an empty body is 400, and an oversized one too", async () => {
  assertEquals((await handle(post({ body: "  ", payload: {} }), base).catch((e) => e as Response)).status, 400);
  assertEquals((await handle(post({ body: "x".repeat(8193), payload: {} }), base).catch((e) => e as Response)).status, 400);
});

Deno.test("no bearer token is 401 — a report is never anonymous", async () => {
  const res = await handle(post({ body: "hello", payload: {} }, "Basic nope"), base).catch((e) => e as Response);
  assertEquals(res.status, 401);
});
```

- [ ] **Step 5: `cloud/supabase/functions/issues/handler.ts`.**

```ts
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { scrub, scrubJson } from "../_shared/scrub.ts";

const MAX_BODY = 8192;

export interface Deps {
  verify: VerifyToken;
  save: (row: unknown) => Promise<string>;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const b = await readJson<{
    body?: string;
    payload?: Record<string, unknown>;
    app_version?: string;
    engine_build?: string;
    os_build?: string;
    profile_id?: string;
  }>(req, 1 << 18);
  const text = String(b.body ?? "").trim();
  if (!text) throw fail(400, "an issue report needs a sentence about what went wrong");
  if (text.length > MAX_BODY) throw fail(400, `the report is longer than ${MAX_BODY} characters`);

  const id = await deps.save({
    account_id: user.id,
    body: scrub(text),
    payload: scrubJson(b.payload ?? {}),
    app_version: b.app_version ?? null,
    engine_build: b.engine_build ?? null,
    os_build: b.os_build ?? null,
    profile_id: b.profile_id ?? null,
  });
  return json(200, { id });
}
```

- [ ] **Step 6: `cloud/supabase/functions/issues/index.ts`.**

```ts
import { authGetUser, restFromEnv } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      save: async (row) => {
        const res = await rest.fetch(`${rest.url}/rest/v1/issues`, {
          method: "POST",
          headers: {
            apikey: rest.serviceKey,
            authorization: `Bearer ${rest.serviceKey}`,
            "content-type": "application/json",
            prefer: "return=representation",
          },
          body: JSON.stringify([row]),
        });
        if (!res.ok) {
          console.error(`insert issue: ${res.status}`);
          throw fail(502, "the report could not be stored");
        }
        const rows = await res.json() as { id: string }[];
        return rows[0].id;
      },
    });
  } catch (e) {
    return asResponse(e);
  }
});
```

- [ ] **Step 7: Run every test.** Expected: `ok | 81 passed | 0 failed`. Then `deno lint` and `deno fmt --check`, both clean.

- [ ] **Step 8: Apply, deploy, prove.**

```powershell
supabase db push --workdir cloud/supabase
supabase functions deploy issues --project-ref <ref> --workdir cloud/supabase
curl.exe -i -X POST "https://<ref>.supabase.co/functions/v1/issues" -H "content-type: application/json" -d "{\"body\":\"hi\"}"
```

Expected: `401 {"error":"no bearer token"}`. Then `select jobname from cron.job order by jobname;` → three rows: `knowlu-billing-jobs`, `knowlu-issue-sweep`, `knowlu-tombstone-sweep`.

- [ ] **Step 9: Commit.** `cloud: POST /issues — scrubbed on the device, scrubbed again here, kept ninety days, never anonymous (C1 Task 9)`.

---
### Task 10: `app/src/account.rs` — the session, the sign-in, and the move that survives a rename

**Hand-off H1 must be applied first** (`pub mod account;` and the four stub modules on `main`). Rebase onto it before starting.

**Files:**
- Modify: `app/src/account.rs` (the H1 stub)
- Test: `app/tests/account.rs`

**Interfaces:**
- Consumes: `crate::credentials::{target_for, write, exists, delete}`, `knowlu_engine::wincred::read_credential`, `crate::profiles::id_for`.
- Produces: `PENDING_TARGET`, `TOS_VERSION`, `PRIVACY_VERSION`, `api_base()`, `anon_key()`, `check_api_base()`, `auth_base()`, `struct Session`, `save_session()`, `load_session()`, `move_session()`, `sign_up_at()`, `sign_in_at()`, `magic_link_at()`, `verify_email_code_at()`, `refresh_at()`, `valid_access_token_at()`, `open_in_browser()`, and **six** commands (`sign_up`, `sign_in`, `send_magic_link`, `verify_email_code`, `sign_out`, `open_policy`) — `entitlement_now` and `open_checkout` are written in Task 18, beside the rest of the console-side surface. Tasks 11, 12, 14, 17 and 18 all build on these.

GoTrue /verify type: magiclink, settled 2026-09-10 against staging (`403 otp_expired` — the token was looked up; `email` is accepted by this GoTrue too).
Prod api_base pinned 2026-09-10 (`https://jxthohvwrijwtuwlglan.supabase.co/functions/v1`); verified by a 404 — nothing is deployed to prod yet, so the 401 check is re-run at the first prod deploy.

- [ ] **Step 1: The loopback harness and the first failing test** — `app/tests/account.rs`:

```rust
//! `account.rs` against a loopback server, never the network (the 3a rule: `127.0.0.1` only, and the
//! serving thread is joined before the test returns, so a test can never outlive its own socket).
use knowlu::account::{auth_base, check_api_base, sign_in_at, sign_up_at, Session};
use std::io::{Read, Write};

/// Serves exactly `responses.len()` requests on `127.0.0.1:0`, then stops. Returns the base URL and
/// a handle whose `join()` yields the raw request text of each one — so a test can assert what went
/// on the wire without ever leaving the machine.
fn loopback(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            // Read the head, then exactly as many body bytes as Content-Length promised: a request
            // can arrive split across packets, and a single `read` would truncate it at random.
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 1024];
            let head_end = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break buf.len();
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let want: usize = head
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                .and_then(|l| l.split(':').nth(1)?.trim().parse().ok())
                .unwrap_or(0);
            while buf.len() < head_end + want {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            seen.push(String::from_utf8_lossy(&buf).to_string());
            let reason = if (200..300).contains(&status) { "OK" } else { "Bad Request" };
            let resp = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

#[test]
fn the_auth_base_is_derived_from_the_functions_base_and_nothing_else() {
    // `config/cloud.yaml` carries four keys and the C2 contract fixes them, so the auth host is
    // derived rather than stored — and derived strictly, so a typo is a refusal and not a request
    // to somewhere unexpected.
    assert_eq!(auth_base("https://abc.supabase.co/functions/v1").unwrap(), "https://abc.supabase.co/auth/v1");
    assert_eq!(auth_base("https://abc.supabase.co/functions/v1/").unwrap(), "https://abc.supabase.co/auth/v1");
    assert_eq!(auth_base("http://127.0.0.1:9/functions/v1").unwrap(), "http://127.0.0.1:9/auth/v1");
    assert!(auth_base("https://abc.supabase.co").is_err());
    assert!(auth_base("https://abc.supabase.co/rest/v1").is_err());
}

#[test]
fn only_https_and_the_loopback_test_seam_are_accepted_as_an_api_base() {
    assert!(check_api_base("https://abc.supabase.co/functions/v1").is_ok());
    assert!(check_api_base("http://127.0.0.1:5051/functions/v1").is_ok());
    assert!(check_api_base("http://abc.supabase.co/functions/v1").is_err());
    assert!(check_api_base("ftp://abc/functions/v1").is_err());
}

#[test]
fn a_successful_sign_in_returns_a_session_and_sends_the_anon_key() {
    let body = r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"user":{"id":"acc-1","email":"a@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = sign_in_at(&format!("{base}/auth/v1"), "anon-key", "a@example.invalid", "pw", 1_760_000_000);
    let seen = handle.join().expect("server thread");
    let s: (String, Session) = out.expect("sign in");
    assert_eq!(s.0, "acc-1");
    assert_eq!(s.1.access_token, "at1");
    assert_eq!(s.1.refresh_token, "rt1");
    assert_eq!(s.1.email, "a@example.invalid");
    assert_eq!(s.1.expires_at, 1_760_000_000 + 3600);
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/token?grant_type=password "), "{req}");
    assert!(req.to_lowercase().contains("apikey: anon-key"), "{req}");
    // The password is on the wire because that is what signing in is — but it is never in a log,
    // a message or this assertion. Only the field NAME is checked.
    assert!(req.contains("\"password\""), "{req}");
}

/// The magic link's second half, which is what makes the button on the panel honest.
#[test]
fn a_six_digit_code_from_the_email_becomes_a_session_on_this_machine() {
    use knowlu::account::verify_email_code_at;
    let body = r#"{"access_token":"at9","refresh_token":"rt9","expires_in":3600,"user":{"id":"acc-9","email":"c@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = verify_email_code_at(&format!("{base}/auth/v1"), "anon-key", "c@example.invalid", " 123456 ", 0);
    let seen = handle.join().expect("server thread");
    let (id, sess) = out.expect("verify");
    assert_eq!(id, "acc-9");
    assert_eq!(sess.access_token, "at9");
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/verify "), "{req}");
    assert!(req.contains("\"type\":\"magiclink\""), "{req}");
    // Trimmed: a code pasted out of a mail client arrives with whitespace around it more often than not.
    assert!(req.contains("\"token\":\"123456\""), "{req}");
}

#[test]
fn a_refused_sign_in_is_the_providers_sentence_and_never_a_status_code() {
    let (base, handle) = loopback(vec![(400, r#"{"error_description":"Invalid login credentials"}"#.to_string())]);
    let out = sign_in_at(&format!("{base}/auth/v1"), "anon-key", "a@example.invalid", "pw", 0);
    let _ = handle.join().expect("server thread");
    assert_eq!(out.unwrap_err(), "Invalid login credentials");
}

#[test]
fn signing_up_sends_the_attestation_and_both_policy_versions_as_user_metadata() {
    let body = r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"user":{"id":"acc-2","email":"b@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = sign_up_at(&format!("{base}/auth/v1"), "anon-key", "b@example.invalid", "pw", "2026-09-10", "2026-09-10", 0);
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{:?}", out.err());
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/signup "), "{req}");
    // The trigger in migration 20260910000100 refuses a sign-up without all three, so a client that
    // forgot one would fail at the database with a message nobody could act on. This is the pin.
    assert!(req.contains("\"age_attested\":\"true\""), "{req}");
    assert!(req.contains("\"tos_version\":\"2026-09-10\""), "{req}");
    assert!(req.contains("\"privacy_version\":\"2026-09-10\""), "{req}");
}
```

- [ ] **Step 2: Run and watch them fail.**

Run: `cargo test -p knowlu --test account`
Expected: `error[E0432]: unresolved import `knowlu::account::auth_base`` — six tests, none of them compiled.

- [ ] **Step 3: Fill `app/src/account.rs` — the values, the session and the four HTTP calls.**

```rust
//! The account: the session, the sign-in, the entitlement cache, and the commands the wizard and the
//! settings panel call (Knowlu C1, Tasks 10, 11 and 17).
//!
//! **Two rules from `credentials.rs` carry over unchanged.** A secret is never logged, never in a run
//! record, a backup, a fixture, a test name or an error message; and the page never sees one — a
//! command takes a password in and gives an envelope back, and the token that comes out of it goes
//! straight into Credential Manager without passing through the webview.
use std::path::Path;
use serde_json::{json, Value};

/// The Supabase project this build talks to. **Both values are public**: Supabase publishes the
/// project URL and the anon key in every client it generates, and neither grants anything on its own
/// — row-level security and the edge functions decide what a caller may do. The service-role key,
/// which does bypass all of that, is a function secret Quinn sets and appears nowhere in this repo.
///
/// Filled from precondition P1 in step 4 below. `KNOWLU_API_BASE` / `KNOWLU_ANON_KEY` override them
/// at run time, which is how a scratch profile is pointed at `knowlu-staging` (§11 R6) without a
/// second build — the same shape `KNOWLU_ENGINE_EXE` already uses.
pub const DEFAULT_API_BASE: &str = "<P1: the knowlu-prod project's functions URL>";
pub const DEFAULT_ANON_KEY: &str = "<P1: the knowlu-prod project's anon key>";

/// The policy versions a sign-up records. Dates, not numbers, because the pages carry a date too and
/// a reader comparing the two should not have to hold a mapping in their head. Bump BOTH the constant
/// and the page's date in the same commit, or the consent log points at text nobody can find.
pub const TOS_VERSION: &str = "2026-09-10";
pub const PRIVACY_VERSION: &str = "2026-09-10";

/// Where a session lives before there is a vault to key it to. The wizard signs in on panel 2 and
/// creates the vault on panel 9, so for those seven panels the profile id does not exist yet — and it
/// is derived from the vault path, which the user can still change. `move_session` walks it over at
/// Finish, exactly as `retarget_credentials` walks the coursework logins (R-P4a-23), and for exactly
/// the same reason: a credential filed under a path nothing will look at is worse than no credential.
pub const PENDING_TARGET: &str = "knowlu/pending/session";

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn api_base() -> String {
    std::env::var("KNOWLU_API_BASE").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_API_BASE.to_string())
}

pub fn anon_key() -> String {
    std::env::var("KNOWLU_ANON_KEY").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_ANON_KEY.to_string())
}

/// `https://` always; `http://127.0.0.1:` only, and only so the tests in `app/tests/account.rs` can
/// stand a real server up on a real socket without a `#[cfg(test)]` branch inside the production
/// path. Nothing else — an `http://` host on a campus wifi is a session token in the clear.
pub fn check_api_base(api_base: &str) -> Result<(), String> {
    if api_base.starts_with("https://") || api_base.starts_with("http://127.0.0.1:") {
        Ok(())
    } else {
        Err(format!("{api_base}: an api_base must be https://"))
    }
}

/// `…/functions/v1` → `…/auth/v1`. Derived rather than stored because `config/cloud.yaml`'s four keys
/// are a contract with C2 and adding a fifth would be a conversation. Strict on purpose: anything
/// that is not a functions base is refused here rather than turned into a request to a host nobody
/// chose.
pub fn auth_base(api_base: &str) -> Result<String, String> {
    let trimmed = api_base.trim_end_matches('/');
    match trimmed.strip_suffix("/functions/v1") {
        Some(root) => {
            check_api_base(api_base)?;
            Ok(format!("{root}/auth/v1"))
        }
        None => Err(format!("{api_base}: an api_base ends in /functions/v1")),
    }
}

/// What Credential Manager holds at `knowlu/<profile_id>/session`. `UserName` is the account id — not
/// a secret, and the one field C2 reads without decoding anything — and the blob is this struct as
/// JSON. `expires_at` is Unix seconds, so a clock comparison needs no date library at the call site.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub email: String,
}

pub fn session_target(profile_id: &str) -> String { crate::credentials::target_for(profile_id, "session") }

pub fn save_session(target: &str, account_id: &str, s: &Session) -> Result<(), String> {
    let blob = serde_json::to_string(s).map_err(|e| e.to_string())?;
    crate::credentials::write(target, account_id, &blob)
}

pub fn load_session(target: &str) -> Result<(String, Session), String> {
    let cred = knowlu_engine::wincred::read_credential(target).map_err(|e| e.to_string())?;
    let s: Session = serde_json::from_str(cred.password.expose()).map_err(|e| format!("{target}: not a session ({e})"))?;
    Ok((cred.username, s))
}

/// Read, write the new one, delete the old — in that order, never delete-then-write: a failure in
/// between would leave the user signed out with no way back but retyping a password they may have
/// generated. `retarget_credentials` makes the same argument about the coursework logins.
pub fn move_session(from: &str, to: &str) -> Result<(), String> {
    if from == to { return Ok(()); }
    let (account_id, s) = load_session(from)?;
    save_session(to, &account_id, &s)?;
    crate::credentials::delete(from)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        // FALSE deliberately: GoTrue says why it refused in the body of a 400, and turning that into
        // an `Err(status)` would replace "Invalid login credentials" with "400" on the panel.
        .http_status_as_error(false)
        .build()
        .into()
}

/// One GoTrue POST. Returns the parsed body and the status; the caller decides what a status means.
/// **The transport failure has a stable first clause**, `the account service could not be reached`,
/// because the page has to tell "your password is wrong" from "there is no network" — the first is
/// something a user can fix on the panel, the second is what stands the upgrade overlay down instead
/// of trapping someone behind it. Everything else is the provider's own sentence.
pub const UNREACHABLE: &str = "the account service could not be reached";

fn post_json(url: &str, anon: &str, body: &Value) -> Result<(u16, Value), String> {
    let mut res = agent()
        .post(url)
        .header("apikey", anon)
        .header("content-type", "application/json")
        .send_json(body)
        .map_err(|e| format!("{UNREACHABLE} ({e})"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 20).read_to_string().map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok((status, value))
}

/// GoTrue's own sentence, whichever field it used this time; never a bare status code on a panel.
fn provider_error(status: u16, v: &Value) -> String {
    for key in ["error_description", "msg", "message", "error"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            if !s.is_empty() { return s.to_string(); }
        }
    }
    format!("the account service refused the request ({status})")
}

fn session_from(v: &Value, now_unix: i64) -> Result<(String, Session), String> {
    let account_id = v.get("user").and_then(|u| u.get("id")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let access_token = v.get("access_token").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let refresh_token = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    if account_id.is_empty() || access_token.is_empty() {
        return Err("the account service returned no session".to_string());
    }
    let expires_in = v.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(3600);
    let email = v.get("user").and_then(|u| u.get("email")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    Ok((account_id, Session { access_token, refresh_token, expires_at: now_unix + expires_in, email }))
}

/// Email + password, with the three things migration `20260910000100`'s trigger insists on. A sign-up
/// missing any of them is refused **by the database**, so sending them is not politeness.
pub fn sign_up_at(
    auth_base: &str,
    anon: &str,
    email: &str,
    password: &str,
    tos_version: &str,
    privacy_version: &str,
    now_unix: i64,
) -> Result<(String, Session), String> {
    let body = json!({
        "email": email,
        "password": password,
        "data": { "age_attested": "true", "tos_version": tos_version, "privacy_version": privacy_version },
    });
    let (status, v) = post_json(&format!("{auth_base}/signup"), anon, &body)?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    // With email confirmation on, a sign-up returns the user and NO session until the link is
    // clicked. That is not an error — the wizard says so and waits.
    if v.get("access_token").is_none() {
        return Err("check your email and click the link, then sign in".to_string());
    }
    session_from(&v, now_unix)
}

pub fn sign_in_at(auth_base: &str, anon: &str, email: &str, password: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=password"), anon, &json!({ "email": email, "password": password }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// A magic link never returns a session — it sends mail. Success is "we sent it", nothing more.
pub fn magic_link_at(auth_base: &str, anon: &str, email: &str) -> Result<(), String> {
    let (status, v) = post_json(&format!("{auth_base}/otp"), anon, &json!({ "email": email, "create_user": false }))?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

/// …and this is how a **desktop** app finishes one. The link in the mail redirects to
/// `https://knowlu.com/signed-in.html` with the session in the URL fragment, **in the user's
/// browser** — a place this process will never see. So the mail carries a six-digit code beside the
/// link (`config.toml`'s `[auth.email.template.magic_link]`, GoTrue's own `{{ .Token }}`), the panel
/// asks for it, and `/verify` trades it for the same session the link would have given.
///
/// No deep-link scheme, no URI registration, no new dependency — and the button on the panel does
/// what its label says, which is the whole point of keeping it (spec §5.1 names magic link as one of
/// the two identity paths).
/// The `type` is the one thing here that is not certain across GoTrue versions — **step 3a settled it
/// before this was written**; if the string below is not the one that step recorded, that step's
/// answer wins.
pub fn verify_email_code_at(auth_base: &str, anon: &str, email: &str, code: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/verify"), anon, &json!({ "type": "magiclink", "email": email, "token": code.trim() }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

pub fn refresh_at(auth_base: &str, anon: &str, refresh_token: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=refresh_token"), anon, &json!({ "refresh_token": refresh_token }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// **Refresh is C1's job** (the C2 contract, point 2). A token with under two minutes left is
/// refreshed and the entry rewritten, so C2 — which only ever reads — finds a live token or a stale
/// one it can wait out, and never has to hold a refresh race with this process.
pub fn valid_access_token_at(auth_base: &str, anon: &str, target: &str, now_unix: i64) -> Result<String, String> {
    let (account_id, s) = load_session(target)?;
    if s.expires_at - now_unix > 120 { return Ok(s.access_token); }
    let (id, fresh) = refresh_at(auth_base, anon, &s.refresh_token, now_unix)?;
    let id = if id.is_empty() { account_id } else { id };
    save_session(target, &id, &fresh)?;
    Ok(fresh.access_token)
}
```

- [ ] **Step 3a: Settle GoTrue's `/verify` type — one `curl` against staging, before anything runs the code that depends on it.** (It sits after step 3 because it needs the staging project from P1; step 5 is the first run, so the answer lands before any of it is executed.) `"magiclink"` is the type for a link-issued OTP and `"email"` is the same call for an email OTP on newer GoTrue; neither this plan's author nor its reviewer verified which the pinned version wants, and the difference is one string in `verify_email_code_at`. Against **staging**, with the project's anon key:

```powershell
curl.exe -i -X POST "https://<the STAGING ref>.supabase.co/auth/v1/verify" -H "apikey: <the anon key>" -H "content-type: application/json" -d "{\"type\":\"magiclink\",\"email\":\"you@example.invalid\",\"token\":\"000000\"}"
```

  A 400 complaining about the **token** (expired, invalid, not found) means `magiclink` is the right type — the request got as far as looking one up. A 400 complaining about the **type** means use `"email"` instead. Record which you saw above step 1 (`GoTrue /verify type: magiclink|email, settled <date>`) and use it in step 3's `verify_email_code_at`.

- [ ] **Step 4: Fill the two constants from P1.** With the values Quinn gave in Task 1, replace `DEFAULT_API_BASE` and `DEFAULT_ANON_KEY` with the **prod** project's functions URL (`https://<prod ref>.supabase.co/functions/v1`) and its anon key. Verify the URL by hand once: `curl.exe -i "<that URL>/entitlement"` → `401 {"error":"no bearer token"}` (deploy `entitlement` to prod first, or accept a `404` and re-verify after the first prod deploy, recording which you saw). Then add, above step 1: `Prod api_base pinned <date>; verified by a 401 from /entitlement.`

- [ ] **Step 5: Run the tests and watch them pass.**

Run: `cargo test -p knowlu --test account`
Expected: `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 6: The six wizard commands**, appended to `app/src/account.rs`:

```rust
fn env_pair() -> Result<(String, String, String), String> {
    let base = api_base();
    check_api_base(&base)?;
    Ok((auth_base(&base)?, anon_key(), base))
}

fn now_unix() -> i64 { jiff::Timestamp::now().as_second() }

fn ok_account(account_id: &str, email: &str) -> Value {
    json!({ "ok": true, "error": Value::Null, "account_id": account_id, "email": email })
}

/// Create the account. **The three acceptances are Rust's, not the page's**: the versions are
/// constants here and the attestation is refused here, so a page that forgot a checkbox cannot make
/// an account that the consent log then describes wrongly.
#[tauri::command(async)]
pub fn sign_up(email: String, password: String, age_attested: bool) -> Value {
    if !age_attested {
        return json!({ "ok": false, "error": "Knowlu is for people 18 or older.", "account_id": Value::Null });
    }
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match sign_up_at(&auth, &anon, email.trim(), &password, TOS_VERSION, PRIVACY_VERSION, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn sign_in(email: String, password: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match sign_in_at(&auth, &anon, email.trim(), &password, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn send_magic_link(email: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    match magic_link_at(&auth, &anon, email.trim()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The other half of the magic link: the code from the mail, traded for a session on this machine.
#[tauri::command(async)]
pub fn verify_email_code(email: String, code: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match verify_email_code_at(&auth, &anon, email.trim(), &code, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

/// The system browser, by `explorer.exe <url>` — the same mechanism the tray's *Open vault folder*
/// uses, and no new dependency for one line. Its exit code is not checked: `explorer.exe` returns
/// non-zero on success often enough that checking it would report failures that did not happen.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    use knowlu_engine::childproc::NoConsole;
    std::process::Command::new("explorer.exe").no_console().arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// **The terms and the privacy policy, opened where they can actually be read.** `app/static/` holds
/// four files; a plain `<a href="terms.html">` in the wizard navigates the one webview to a missing
/// asset and the window is lost until restart — while the user is being asked to tick a box saying
/// they accept it, which is the legal core of §9's consent requirement. The markup keeps its `href`
/// (the static test reads it, and it is the honest link), the page's handler calls
/// `preventDefault()`, and this opens the published page in the system browser.
#[tauri::command(async)]
pub fn open_policy(which: String) -> Value {
    let page = match which.as_str() {
        "terms" => "terms.html",
        "privacy" => "privacy.html",
        _ => return json!({ "ok": false, "error": format!("no policy called {which:?}") }),
    };
    match open_in_browser(&format!("https://knowlu.com/{page}")) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Forget the pending session, and revoke it (spec §5.1, "sign-out revokes"). The revoke is **best
/// effort and second**: a network that is down must not leave a token on this machine that the user
/// believes they signed out of, so the local delete is what the envelope reports on.
#[tauri::command(async)]
pub fn sign_out() -> Value {
    let token = load_session(PENDING_TARGET).ok().map(|(_, s)| s.access_token);
    if crate::credentials::exists(PENDING_TARGET) {
        if let Err(e) = crate::credentials::delete(PENDING_TARGET) {
            return json!({ "ok": false, "error": e });
        }
    }
    if let (Some(t), Ok(auth)) = (token, auth_base(&api_base())) {
        let _ = agent()
            .post(&format!("{auth}/logout"))
            .header("apikey", &anon_key())
            .header("authorization", &format!("Bearer {t}"))
            .send_empty();
    }
    json!({ "ok": true, "error": Value::Null })
}
```

- [ ] **Step 7: Prove `move_session` with a test that needs Credential Manager**, appended to `app/tests/account.rs`:

```rust
/// Credential Manager is real on this machine, so this test uses two targets of its own naming and
/// removes them itself. No secret is asserted on: only the account id and the fact of the move.
#[cfg(windows)]
#[test]
fn a_session_moves_from_the_pending_target_to_the_profiles_own() {
    use knowlu::account::{load_session, move_session, save_session, Session};
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let s = Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 42, email: "a@example.invalid".into() };
    save_session(&from, "acc-1", &s).expect("write the pending session");
    move_session(&from, &to).expect("move");
    let (id, back) = load_session(&to).expect("read the moved session");
    assert_eq!(id, "acc-1");
    assert_eq!(back, s);
    assert!(load_session(&from).is_err(), "the pending entry must be gone");
    let _ = knowlu::credentials::delete(&to);
}
```

- [ ] **Step 8: Run the whole app suite.**

Run: `cargo test -p knowlu`
Expected: `test result: ok. 7 passed` for `account`, and every other app test unchanged (129 passed / 1 ignored plus the new seven). 0 warnings.

- [ ] **Step 9: Commit.**

```bash
git add app/src/account.rs app/tests/account.rs
git commit -F <message file>
```

Message: `app: account.rs — sign-up with the attestation and both policy versions, sign-in, magic link, and the pending session that moves at Finish (C1 Task 10)`.

---

### Task 11: The entitlement cache, its 72-hour grace, and the three-state judge gate

**Files:**
- Modify: `app/src/account.rs`, `app/src/scheduler.rs`
- Test: `app/tests/account.rs`, `app/tests/scheduler.rs`

**Interfaces:**
- Consumes: Task 10's `api_base()`, `valid_access_token_at()`, `session_target()`.
- Produces: `struct EntitlementCache { status, current_period_end, plan, checked_at }`, `GRACE`, `cache_path()`, `load_cache()`, `save_cache()`, `enum EntitlementState { Entitled, NotEntitled, NoAccount }`, `decide()`, `fetch_entitlement_at()`, `refresh_entitlement()`, `cloud_config()`, `struct CloudConfig`; and in `scheduler.rs`, `entitlement_state(cs) -> EntitlementState`, `enum JudgePlan { Cloud { log_dir }, Local(JudgeArgs), Skip(&'static str) }`, `judge_plan(cs) -> JudgePlan`, and `slot_argv(vault, exe, &JudgePlan)`. **C2 consumes `JudgePlan::Cloud`'s argv** (R-X-4): `judge --vault <v> --via local-runner --log-dir <the profile's judgments folder>` and nothing else — the engine reads `config/cloud.yaml` itself. Tasks 15 and 16 read `cloud_config`; Task 18 reads `entitlement_state`.

- [ ] **Step 1: Write the failing tests** — append to `app/tests/account.rs`:

```rust
#[test]
fn the_grace_is_seventy_two_hours_and_a_missing_cache_is_never_entitled() {
    use knowlu::account::{decide, EntitlementCache, EntitlementState};
    let at = |s: &str| EntitlementCache {
        status: s.to_string(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: "2026-09-10T12:00:00.000Z".to_string(),
    };
    let now = |s: &str| s.parse::<jiff::Timestamp>().unwrap();
    // No account at all — a vault with no config/cloud.yaml. Not a failure: a named skipped step.
    assert_eq!(decide(false, Some(&at("active")), now("2026-09-10T12:00:00Z")), EntitlementState::NoAccount);
    // Never checked. The page still ranks; the cloud steps stand down.
    assert_eq!(decide(true, None, now("2026-09-10T12:00:00Z")), EntitlementState::NotEntitled);
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-10T12:00:01Z")), EntitlementState::Entitled);
    assert_eq!(decide(true, Some(&at("trialing")), now("2026-09-10T12:00:01Z")), EntitlementState::Entitled);
    // Spec §5.1: a dead hotel wifi must never blank today's page. Seventy-two hours, to the second.
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-13T11:59:59Z")), EntitlementState::Entitled);
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-13T12:00:01Z")), EntitlementState::NotEntitled);
    for bad in ["past_due", "canceled", "none", "whatever"] {
        assert_eq!(decide(true, Some(&at(bad)), now("2026-09-10T12:00:01Z")), EntitlementState::NotEntitled, "{bad}");
    }
    // A clock that went backwards is not a licence: an unparsable or future stamp is not entitled.
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-09T12:00:00Z")), EntitlementState::NotEntitled);
}

#[test]
fn fetching_the_entitlement_parses_the_four_keys_and_sends_the_bearer_token() {
    use knowlu::account::fetch_entitlement_at;
    let body = r#"{"status":"trialing","current_period_end":"2026-09-17T00:00:00+00:00","plan":"monthly","checked_at":"2026-09-10T12:00:00.000Z"}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let got = fetch_entitlement_at(&format!("{base}/functions/v1"), "the-access-token");
    let seen = handle.join().expect("server thread");
    let c = got.expect("entitlement");
    assert_eq!(c.status, "trialing");
    assert_eq!(c.plan.as_deref(), Some("monthly"));
    assert_eq!(c.checked_at, "2026-09-10T12:00:00.000Z");
    assert!(seen[0].starts_with("GET /functions/v1/entitlement "), "{}", seen[0]);
    assert!(seen[0].to_lowercase().contains("authorization: bearer the-access-token"), "{}", seen[0]);
}

#[test]
fn a_cache_round_trips_through_the_profile_folder() {
    use knowlu::account::{cache_path, load_cache, save_cache, EntitlementCache};
    let dir = std::env::temp_dir().join(format!("knowlu-ent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(load_cache(&dir).is_none(), "a fresh profile has no cache");
    let c = EntitlementCache { status: "active".into(), current_period_end: None, plan: None, checked_at: "2026-09-10T12:00:00.000Z".into() };
    save_cache(&dir, &c).expect("save");
    assert_eq!(load_cache(&dir).as_ref(), Some(&c));
    assert_eq!(cache_path(&dir), dir.join("entitlement.json"));
    // A hand-mangled cache is "not entitled", never a panic and never a default that grants access.
    std::fs::write(cache_path(&dir), b"{").unwrap();
    assert!(load_cache(&dir).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
```

…and to `app/tests/scheduler.rs`:

```rust
/// Spec §5.1: past the grace the slots keep ranking and the cloud steps are skipped **as named
/// steps**, never as failures. The shape is `ingest (skipped: no ics_url)`'s, deliberately — the two
/// sit side by side in the Runs view and a reader should not have to learn two conventions.
#[test]
fn a_vault_with_an_account_and_no_entitlement_records_the_judge_skip_and_stays_green() {
    let v = scratch("noentitlement");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // A vault that HAS an account: the four keys the C2 contract fixes.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://example.supabase.co/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "noentitlement");
    // …and no entitlement cache at all, which is a fresh install that has not reached the cloud yet.
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-ent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.contains(&"judge (skipped: no entitlement)".to_string()), "{named:?}");
    assert!(!named.iter().any(|n| n == "judge"), "the step itself never ran");
    assert_eq!(s.steps.iter().find(|(n, _)| n.starts_with("judge (skipped")).unwrap().1, 0, "a skip is not a failure");
    assert!(s.engine_ok, "a skipped judge step must not paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}

/// …and a vault with no cloud config at all is `NoAccount`, which is what sends it down the local
/// runtime path rather than to the cloud — case (b), and the reason the two pre-C1 tests still pass.
#[test]
fn a_vault_with_no_cloud_config_says_no_account_rather_than_no_entitlement() {
    use knowlu::account::EntitlementState;
    use knowlu::scheduler::entitlement_state;
    let v = scratch("noaccount");
    let cs = open(&v, "noaccount");
    assert_eq!(entitlement_state(&cs), EntitlementState::NoAccount);
    let _ = std::fs::remove_dir_all(&v);
}

/// Case (a), and the one C2 depends on: **an entitled vault runs `judge`, with nothing but the vault
/// on the command line.** A runtime-based skip here would leave the whole judgment service inert on
/// every machine that never installed llama.cpp — which, after C4, is every machine.
#[test]
fn an_entitled_vault_runs_judge_with_no_runtime_and_no_account_on_the_command_line() {
    use knowlu::account::{save_cache, EntitlementCache};
    use knowlu::scheduler::{judge_plan, slot_argv, JudgePlan};
    let v = scratch("cloudjudge");
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://example.supabase.co/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "cloudjudge");
    save_cache(&cs.data_dir, &EntitlementCache {
        status: "active".into(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let plan = judge_plan(&cs);
    let JudgePlan::Cloud { log_dir } = plan.clone() else { panic!("not the cloud plan: {plan:?}") };
    // The profile's own judgments folder, never the vault: judgment logs never enter a vault.
    assert!(log_dir.starts_with(&cs.data_dir), "{log_dir:?} is not under {:?}", cs.data_dir);
    let argv = slot_argv(&v, Path::new(r"C:\bin\knowlu-engine.exe"), &plan);
    let judge = argv.iter().find(|(_, a)| a[0] == "judge").expect("the judge step is in the argv");
    assert_eq!(
        judge.1,
        vec!["judge", "--vault", v.to_string_lossy().as_ref(), "--via", "local-runner", "--log-dir", log_dir.to_string_lossy().as_ref()]
    );
    // Nothing about the account is on a command line — a process list is not a place for one.
    for (_, args) in &argv {
        assert!(!args.iter().any(|a| a.contains("acc-1") || a == "--runtime" || a == "--model"), "{args:?}");
    }
    let _ = std::fs::remove_dir_all(&v);
}

/// Case (c): entitled once, but not for four days. The grace is over, the cloud step stands down as a
/// named step, and the day is still ranked.
#[test]
fn an_entitlement_past_the_grace_skips_judge_by_name_and_keeps_the_slot_green() {
    use knowlu::account::{save_cache, EntitlementCache};
    use knowlu::scheduler::{judge_plan, JudgePlan};
    let v = scratch("pastgrace");
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'https://example.supabase.co/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "pastgrace");
    let four_days_ago = (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(96)).to_string();
    save_cache(&cs.data_dir, &EntitlementCache {
        status: "active".into(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: four_days_ago,
    }).unwrap();
    assert_eq!(judge_plan(&cs), JudgePlan::Skip("judge (skipped: no entitlement)"));
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 2: Run and watch them fail.** `cargo test -p knowlu --test account --test scheduler` → unresolved imports for `decide`, `EntitlementState`, `entitlement_state`.

- [ ] **Step 3: The cache and the config reader**, appended to `app/src/account.rs`:

```rust
/// The four keys `config/cloud.yaml` carries — the contract with C2, in this order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudConfig {
    pub api_base: String,
    pub anon_key: String,
    pub session_credential_target: String,
    pub account_id: String,
}

/// Read through `pystr` and `serde_yaml_ng`, never by comparing bytes: a vault's files are whatever
/// Windows made them, and the engine translates line endings on every read for that reason.
pub fn cloud_config(vault: &Path) -> Result<CloudConfig, String> {
    let path = vault.join("config").join("cloud.yaml");
    let text = knowlu_engine::pystr::read_text(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let cfg = CloudConfig {
        api_base: s("api_base"),
        anon_key: s("anon_key"),
        session_credential_target: s("session_credential_target"),
        account_id: s("account_id"),
    };
    if cfg.api_base.is_empty() || cfg.account_id.is_empty() {
        return Err(format!("{}: api_base and account_id are required", path.display()));
    }
    Ok(cfg)
}

/// `GET /entitlement`'s reply, cached verbatim. The four keys are the C2 contract; nothing here adds
/// a fifth, so a reply this app does not understand still round-trips through the file unchanged.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntitlementCache {
    pub status: String,
    pub current_period_end: Option<String>,
    pub plan: Option<String>,
    pub checked_at: String,
}

/// Spec §5.1. Seventy-two hours, so a weekend of bad wifi never blanks today's page.
pub const GRACE: std::time::Duration = std::time::Duration::from_secs(72 * 60 * 60);

/// **A file beside `settings.json`, not a field in it.** `state::Settings` has no `#[serde(default)]`
/// on any field and `Settings::load` falls back to defaults on a parse failure, so adding a field
/// would silently reset a user's backup folder and autostart choice on their first launch after an
/// update. `onboarding::offer_marker` already makes this argument, and this follows its precedent.
pub fn cache_path(data_dir: &Path) -> std::path::PathBuf { data_dir.join("entitlement.json") }

pub fn load_cache(data_dir: &Path) -> Option<EntitlementCache> {
    let text = std::fs::read_to_string(cache_path(data_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_cache(data_dir: &Path, c: &EntitlementCache) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let v = serde_json::to_value(c).map_err(|e| e.to_string())?;
    // `ledger::dumps_value`, like every other JSON this app writes: a file the app wrote and a file
    // the engine wrote never differ by whitespace.
    std::fs::write(cache_path(data_dir), knowlu_engine::ledger::dumps_value(&v)).map_err(|e| e.to_string())
}

/// Three answers, and the difference between the last two is the sentence a user reads in the Runs
/// view (`no account` vs `no entitlement`) — the same distinction `IcsState` draws between "no feed"
/// and "unreadable", for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementState {
    Entitled,
    NotEntitled,
    NoAccount,
}

pub fn decide(cloud_configured: bool, cache: Option<&EntitlementCache>, now: jiff::Timestamp) -> EntitlementState {
    if !cloud_configured { return EntitlementState::NoAccount; }
    let Some(c) = cache else { return EntitlementState::NotEntitled };
    if c.status != "active" && c.status != "trialing" { return EntitlementState::NotEntitled; }
    let Ok(checked) = c.checked_at.parse::<jiff::Timestamp>() else { return EntitlementState::NotEntitled };
    // A stamp in the future is a clock that moved, not a licence.
    let age = now.as_second() - checked.as_second();
    if age < 0 || age > GRACE.as_secs() as i64 { return EntitlementState::NotEntitled; }
    EntitlementState::Entitled
}

pub fn fetch_entitlement_at(api_base: &str, access_token: &str) -> Result<EntitlementCache, String> {
    check_api_base(api_base)?;
    let mut res = agent()
        .get(&format!("{}/entitlement", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {access_token}"))
        .call()
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) {
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        return Err(provider_error(status, &v));
    }
    serde_json::from_str(&text).map_err(|e| format!("the entitlement reply could not be read ({e})"))
}

/// One refresh: token, call, cache. Every failure leaves the previous cache exactly where it was —
/// which is what makes the grace a grace and not a countdown that a bad network can restart.
pub fn refresh_entitlement(vault: &Path, data_dir: &Path) -> Result<EntitlementCache, String> {
    let cfg = cloud_config(vault)?;
    let auth = auth_base(&cfg.api_base)?;
    let token = valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix())?;
    let c = fetch_entitlement_at(&cfg.api_base, &token)?;
    save_cache(data_dir, &c)?;
    Ok(c)
}
```

- [ ] **Step 4: The scheduler's gate** — in `app/src/scheduler.rs`, add beside `judge_state`:

```rust
/// Is this install allowed to run a cloud step? Reads the vault's own `config/cloud.yaml` and the
/// profile's cached entitlement, and decides with `account::decide` — which owns the 72-hour grace so
/// that a later cloud step (C2's `/judge/*`, `/ingest/*`, `/events`) asks the same question in one
/// place rather than four.
pub fn entitlement_state(cs: &ConsoleState) -> crate::account::EntitlementState {
    let configured = crate::account::cloud_config(&cs.vault).is_ok();
    crate::account::decide(configured, crate::account::load_cache(&cs.data_dir).as_ref(), jiff::Timestamp::now())
}
```

…then add the plan type and the one function that decides it, beside `JudgeState`:

```rust
/// How this slot runs `judge` — or does not. **Three states, and the order between them matters**
/// (agreed with C2, 2026-09-09):
///
/// 1. **Entitlement outranks everything.** Past the 72-hour grace the step is a named skip, whatever
///    is installed locally, because from C2 on `judge` *is* a cloud call.
/// 2. **A vault with `config/cloud.yaml` runs `judge`, full stop.** No `--runtime`, no `--model`, and
///    nothing about the account on the command line: the engine reads `config/cloud.yaml` itself and
///    decides everything else, and it still always exits 0. A runtime-based skip here would leave
///    C2's whole judgment service inert on every machine that never installed llama.cpp.
/// 3. **Only a vault with no `config/cloud.yaml`** — a pre-C1 install, until Task 18's overlay adopts
///    it — falls through to the local runtime and model, which is exactly plan 3a's behaviour and
///    leaves in C4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgePlan {
    /// `judge --vault <v> --via local-runner --log-dir <d>`. The engine reads the account out of the
    /// vault; the only thing it cannot know is where this profile's judgment logs go, and **judgment
    /// logs never enter the vault** (`CLAUDE.md`), so `--log-dir` is in this arm as well as the local
    /// one (R-X-4).
    Cloud { log_dir: PathBuf },
    /// The local runtime and model this profile installed (plan 3a; removed in C4).
    Local(JudgeArgs),
    /// Not this slot, and why. A named step with exit code **0** — never a failure.
    Skip(&'static str),
}

pub fn judge_plan(cs: &ConsoleState) -> JudgePlan {
    match entitlement_state(cs) {
        crate::account::EntitlementState::NotEntitled => JudgePlan::Skip("judge (skipped: no entitlement)"),
        // The same directory the local arm uses — `inference::judgments_dir(&cs.data_dir)`, which is
        // `%LOCALAPPDATA%\knowlu\profiles\<id>\judgments`. C4 removes the runtime, not this folder.
        crate::account::EntitlementState::Entitled => JudgePlan::Cloud { log_dir: crate::inference::judgments_dir(&cs.data_dir) },
        crate::account::EntitlementState::NoAccount => match judge_state(cs) {
            JudgeState::Ready(a) => JudgePlan::Local(a),
            JudgeState::NoRuntime => JudgePlan::Skip("judge (skipped: no runtime)"),
            JudgeState::NoModel => JudgePlan::Skip("judge (skipped: no model)"),
        },
    }
}
```

…widen `slot_argv` to take it, replacing its `judge: Option<&JudgeArgs>` parameter and the `if let Some(j) = judge` block:

```rust
pub fn slot_argv(vault: &Path, exe: &Path, judge: &JudgePlan) -> Vec<(PathBuf, Vec<String>)> {
    let v = vault.to_string_lossy().to_string();
    let mut steps = vec![(exe.to_path_buf(), vec!["coursework".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()])];
    if has_ics_url(vault) {
        steps.push((exe.to_path_buf(), vec!["ingest".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into()]));
    }
    match judge {
        JudgePlan::Cloud { log_dir } => {
            // The vault and the log directory, and nothing else: `config/cloud.yaml` is in the vault,
            // and an account id on a command line is an account id in a process list.
            steps.push((exe.to_path_buf(), vec![
                "judge".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
                "--log-dir".into(), log_dir.to_string_lossy().into_owned(),
            ]));
        }
        JudgePlan::Local(j) => {
            steps.push((exe.to_path_buf(), vec![
                "judge".into(), "--vault".into(), v.clone(), "--via".into(), "local-runner".into(),
                "--runtime".into(), j.runtime.to_string_lossy().into_owned(),
                "--model".into(), j.model.to_string_lossy().into_owned(),
                "--log-dir".into(), j.log_dir.to_string_lossy().into_owned(),
            ]));
        }
        JudgePlan::Skip(_) => {}
    }
    steps.push((exe.to_path_buf(), vec!["rank".into(), "--vault".into(), v, "--runner".into(), "local".into()]));
    steps
}
```

…and replace `run_slot_inner`'s judge block — the lines from `let judge = judge_state(cs);` through `let judge_args = match &judge { … };`, and the `slot_argv(&cs.vault, &exe, judge_args)` call — with:

```rust
    // Every arm records a step with exit code **0** and a sentence — never a non-zero code, which
    // would set `engine_ok = false`, paint the tray amber and put the slot into retry backoff twice a
    // day for someone who has simply not paid, or not connected.
    let judge = judge_plan(cs);
    if let JudgePlan::Skip(note) = &judge {
        steps.push(((*note).to_string(), 0));
    }
```

```rust
            for (i, (e, args)) in slot_argv(&cs.vault, &exe, &judge).into_iter().enumerate() {
```

- [ ] **Step 5: The two existing scheduler tests stay exactly as they are — and check that they do.** `a_machine_with_no_model_records_the_judge_skip_and_stays_green` and `a_vault_without_a_feed_records_the_ingest_skip_in_the_step_list` run against a fixture vault with **no** `config/cloud.yaml`, which is `NoAccount`, which falls through to the runtime and model gates — the very case they were written for, and case (b) of the three the plan owes. **Do not add a cloud config to either, and do not weaken an assertion.** Run them and confirm both still pass unchanged; if either does not, the ordering in `judge_plan` is wrong and that is the bug, not the test.

  One existing test **does** change: `the_slot_runs_coursework_ingest_judge_rank_and_leaves_out_what_is_not_configured` calls `slot_argv(&v, exe, None)` and `slot_argv(&v, exe, Some(&ja))`. Those become `slot_argv(&v, exe, &JudgePlan::Skip("judge (skipped: no runtime)"))` and `slot_argv(&v, exe, &JudgePlan::Local(ja))`, with `JudgePlan` added to the file's `use` line. Its assertions do not move.

- [ ] **Step 6: The six-hourly refresh** — in `scheduler.rs`'s housekeeping thread, beside the `n % 6` block, add:

```rust
            // Spec §5.1: at launch and every six hours. On its own thread for the reason the update
            // check is (R-P4a-24): one HTTPS round trip on a captive-portal wifi must not hold the
            // debounced sync, the backup, the log prune and the tray colour behind it. A failure is
            // silence — the previous cache stands, and the grace is what it is for.
            if n == 1 || n % 2160 == 0 {
                let vault = cs.vault.clone();
                let data = cs.data_dir.clone();
                std::thread::spawn(move || { let _ = crate::account::refresh_entitlement(&vault, &data); });
            }
```

- [ ] **Step 7: Run everything.**

Run: `cargo test -p knowlu`
Expected: `account` 10 passed; `scheduler` green with its **four** new tests and every existing one still passing (count them before and after — the number must go up by exactly four, and none of the existing ones may be edited except the one argv test named in step 5); every other file unchanged; 0 warnings.

- [ ] **Step 8: Commit.** `app: the entitlement cache and its 72-hour grace — a slot with no subscription still ranks and says judge (skipped: no entitlement) (C1 Task 11)`.

---
### Task 12: `%USERPROFILE%\Knowlu\`, `config/cloud.yaml`, and the folder question's removal

Spec §4.1 and §11a. The defaults (`default_parent`, `default_backup`) landed on `main` before this branch was cut; this task turns them from *defaults the user may change* into *where the vault goes*, and writes the account into the vault.

**Files:**
- Modify: `app/src/scaffold.rs`, `app/src/onboarding.rs`
- Test: `app/tests/scaffold.rs`, `app/tests/onboarding.rs`

**Interfaces:**
- Consumes: `account::{api_base, anon_key, PENDING_TARGET, load_session}` (Task 10); `onboarding::default_folders_in` and `dest_for` (already on `main`); `credentials::target_for`.
- Produces: `scaffold::VaultPlan` with `api_base`, `anon_key`, `account_id`, `personal_calendar`; `scaffold::cloud_yaml(&VaultPlan)`; `scaffold::ingest_yaml` writing a real `calendars:` list; `scaffold::write_cloud_yaml_if_absent(vault, &VaultPlan)` (Task 18 uses it); `scaffold::CAMPUSES` as `[(&str, &str, &str); 2]` — key, label, **LMS base URL** (Task 14 uses the third); `onboarding::vault_dest_in(home, name) -> Result<(PathBuf, PathBuf), String>`; `WizardPlan` without `backup_dir`/parent and with `account_id`.

- [ ] **Step 1: Write the failing tests** — append to `app/tests/scaffold.rs`:

```rust
#[test]
fn a_new_vault_carries_the_four_cloud_keys_and_no_secret() {
    use knowlu::scaffold::{cloud_yaml, VaultPlan};
    let p = VaultPlan {
        profile_id: "profile_0123456789".into(),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "MACHINE".into(),
        campus: "none".into(),
        zybooks: false,
        vhl: false,
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "a-public-anon-key".into(),
        account_id: "11111111-2222-3333-4444-555555555555".into(),
    };
    let text = cloud_yaml(&p).expect("cloud.yaml");
    // The C2 contract, in its order, single-line single-quoted scalars.
    assert_eq!(
        text,
        "api_base: 'https://example.supabase.co/functions/v1'\n\
         anon_key: 'a-public-anon-key'\n\
         session_credential_target: 'knowlu/profile_0123456789/session'\n\
         account_id: '11111111-2222-3333-4444-555555555555'\n"
    );
    // The session is NAMED, never carried — the same promise `ingest.yaml` makes about a password.
    assert!(!text.contains("access_token") && !text.contains("refresh_token"));
    // …and a value that would change the file's SHAPE is refused by field name, as everywhere else.
    let mut bad = p.clone();
    bad.account_id = "acc\nid".into();
    assert!(cloud_yaml(&bad).unwrap_err().contains("account_id"));
}

#[test]
fn a_personal_calendar_becomes_the_engines_calendars_list() {
    use knowlu::scaffold::{ingest_yaml, VaultPlan};
    let base = VaultPlan {
        profile_id: "profile_0123456789".into(),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "M".into(),
        campus: "none".into(),
        zybooks: false,
        vhl: false,
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    // No calendar: the list the engine has always read, empty.
    assert!(ingest_yaml(&base).unwrap().contains("calendars: []\n"));
    // One: the shape `calfeed::load_calendar_events` parses — a list of {name, ics_url} mappings.
    let mut with = base.clone();
    with.personal_calendar = Some("https://calendar.google.com/calendar/ical/x/private-def/basic.ics".into());
    let text = ingest_yaml(&with).unwrap();
    assert!(
        text.contains("calendars:\n  - name: personal\n    ics_url: 'https://calendar.google.com/calendar/ical/x/private-def/basic.ics'\n"),
        "{text}"
    );
    // …and a value that would change the file's shape is refused by field name, as everywhere else.
    let mut bad = base.clone();
    bad.personal_calendar = Some("https://a\nb".into());
    assert!(ingest_yaml(&bad).unwrap_err().contains("personal calendar address"));
}

#[test]
fn create_vault_writes_cloud_yaml_beside_the_other_config_files() {
    use knowlu::scaffold::{create_vault, VaultPlan};
    let root = std::env::temp_dir().join(format!("knowlu-scaffold-cloud-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let p = VaultPlan {
        profile_id: knowlu::profiles::id_for(&dest),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "MACHINE".into(),
        campus: "none".into(),
        zybooks: false,
        vhl: false,
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    create_vault(&dest, &p).expect("create");
    let text = knowlu_engine::pystr::read_text(&dest.join("config").join("cloud.yaml")).expect("read");
    assert!(text.contains("account_id: 'acc-1'"), "{text}");
    // …and the account is readable through the same door C2 will use, not by string matching.
    let cfg = knowlu::account::cloud_config(&dest).expect("cloud_config");
    assert_eq!(cfg.account_id, "acc-1");
    assert_eq!(cfg.session_credential_target, format!("knowlu/{}/session", p.profile_id));
    // A second write is refused rather than silently repointing the vault at another account.
    assert!(knowlu::scaffold::write_cloud_yaml_if_absent(&dest, &p).is_err());
    let _ = std::fs::remove_dir_all(&root);
}
```

…and to `app/tests/onboarding.rs`:

```rust
#[test]
fn the_vault_goes_under_knowlu_and_nobody_picks_a_folder() {
    use knowlu::onboarding::{default_folders_in, vault_dest_in};
    let home = tmp("home");
    // The parent does not exist yet — the app makes it, which is the whole of §4.1.
    let (parent, backups) = default_folders_in(&home);
    assert!(!std::path::Path::new(&parent).exists());
    let (vault, backup) = vault_dest_in(&home, "Fall 2026").expect("dest");
    assert_eq!(vault, home.join("Knowlu").join("Fall 2026"));
    assert_eq!(backup, std::path::PathBuf::from(&backups));
    assert!(std::path::Path::new(&parent).is_dir(), "the parent was created");
    assert!(std::path::Path::new(&backups).is_dir(), "the backups root was created");
    assert!(!vault.exists(), "the vault itself is created by create_vault, not by this");
    // The same refusals `dest_for` makes, because this is `dest_for` with the parent decided.
    assert!(vault_dest_in(&home, "CON").is_err());
    assert!(vault_dest_in(&home, "a/b").is_err());
    assert!(vault_dest_in(&home, "  ").is_err());
    // …and a name that is already there is refused, not silently reused.
    std::fs::create_dir_all(home.join("Knowlu").join("Taken")).unwrap();
    assert!(vault_dest_in(&home, "Taken").is_err());
    let _ = std::fs::remove_dir_all(&home);
}
```

- [ ] **Step 2: Run and watch them fail.** `cargo test -p knowlu --test scaffold --test onboarding` → unresolved imports `cloud_yaml`, `vault_dest_in`; `VaultPlan` missing three fields.

- [ ] **Step 3: `app/src/scaffold.rs`.** Add `Clone` to `VaultPlan`'s derives, three fields, the third `CAMPUSES` column, and two functions:

```rust
/// `(key, label, LMS base URL)` — the wizard's radio list, and where Task 14's sign-in window points
/// for that campus. **Adding a campus is adding a file and one line here**; nothing else in the app
/// knows a campus exists. The URL is empty for `none`, which is what makes `lms_link::capture` take
/// its paste-a-link path for a student whose school is not listed yet.
pub const CAMPUSES: [(&str, &str, &str); 2] = [
    ("none", "None", ""),
    ("university-of-alabama", "University of Alabama", "https://ualearn.blackboard.com/"),
];
```

**This table is provisional and Task 14c replaces it.** R-OB-4 turns `CAMPUSES` into a `Curated`
struct keyed by the IPEDS `UNITID`, moves the LMS host into that struct and takes the list out of
`launch_state` altogether; the tuple here — and the `launch_state` map in step 3 below, and Task 13's
destructure of the third column — are what Task 13's spike needs in the meantime. Task 14c's *Files*
lists both files, and its steps 5 and 6 do the replacing. Read the drift as scheduled, not as drift.

```rust
#[derive(Debug, Clone)]
pub struct VaultPlan {
    pub profile_id: String,
    pub ics_url: Option<String>,
    /// The student's own busy-time calendar, by its secret iCal address (spec §11a). Written into
    /// `config/ingest.yaml`'s `calendars:` list, which is what makes today's page know the day is
    /// already half full — the reason this ruling exists at all.
    pub personal_calendar: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub device: String,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
    /// The three cloud values (C1). `api_base` and `anon_key` are Rust's — the page never sees a URL
    /// (`static_assets.rs` forbids one) — and `account_id` comes from the session the wizard signed
    /// in with, read from Credential Manager, never from the page.
    pub api_base: String,
    pub anon_key: String,
    pub account_id: String,
}
```

```rust
/// `config/cloud.yaml` — the four keys the C2 contract fixes, in that order, through the same
/// `yaml_scalar` every other wizard value goes through, so a typed value can never change the file's
/// shape.
///
/// **No secret is here.** `anon_key` is published in every Supabase client and grants nothing on its
/// own; the session JWT is *named*, not carried — `session_credential_target` is a Credential Manager
/// target, exactly as `credential_target` is for the coursework logins.
pub fn cloud_yaml(p: &VaultPlan) -> Result<String, String> {
    Ok(format!(
        "api_base: {}\nanon_key: {}\nsession_credential_target: {}\naccount_id: {}\n",
        yaml_scalar("api_base", &p.api_base)?,
        yaml_scalar("anon_key", &p.anon_key)?,
        yaml_scalar("session_credential_target", &crate::credentials::target_for(&p.profile_id, "session"))?,
        yaml_scalar("account_id", &p.account_id)?,
    ))
}

/// Task 18's half: a vault that already exists gains `config/cloud.yaml` and nothing else — no
/// scaffold, no seed note, no rewrite of anything. **Refuses to overwrite one**: an install adopted
/// twice must not silently repoint at a second account, and a user who signed in with the wrong
/// address needs to hear that rather than to lose the first one.
pub fn write_cloud_yaml_if_absent(vault: &Path, plan: &VaultPlan) -> Result<(), String> {
    let path = vault.join("config").join("cloud.yaml");
    if path.exists() {
        return Err(format!("{}: this vault already has an account", path.display()));
    }
    write_file(vault, "config/cloud.yaml", &cloud_yaml(plan)?)
}
```

…one line in `build_into`, after `config/runners.yaml`:

```rust
    write_file(root, "config/cloud.yaml", &cloud_yaml(plan)?)?;
```

…and, in `ingest_yaml`, the `calendars:` line stops being empty when the wizard collected one. Replace
`s.push_str("calendars: []\n");` with:

```rust
    // The engine reads `calendars:` as a list of `{name, ics_url}` mappings (`calfeed.rs`), and an
    // empty list is why the first page of a fresh install used to show a day with no busy time in it
    // at all. A block list, through the same `yaml_scalar` every other wizard value goes through.
    match &p.personal_calendar {
        None => s.push_str("calendars: []\n"),
        Some(u) => s.push_str(&format!(
            "calendars:\n  - name: personal\n    ics_url: {}\n",
            yaml_scalar("personal calendar address", u)?
        )),
    }
```

- [ ] **Step 4: `app/src/onboarding.rs`.** Four changes.

`launch_state`'s campus map takes the third column (the URL is Rust's; the page gets key and label only):

```rust
            "campuses": crate::scaffold::CAMPUSES.iter().map(|(k, l, _)| json!({ "key": k, "label": l })).collect::<Vec<_>>(),
```

`WizardPlan` loses the folder and gains nothing the page decides:

```rust
/// Everything the panels collected. A serde struct rather than a loose map, so a missing field is a
/// refusal at the boundary and not a default nobody chose.
///
/// **C1: there is no `backup_dir` and no parent.** Spec §4.1 — the app creates
/// `%USERPROFILE%\Knowlu\<name>` and `%USERPROFILE%\Knowlu\Backups`, and the wizard has no folder
/// panel to carry an answer from. `offer_inference` stays with a `serde(default)` so an older page
/// that still sends it is not a refusal; the C1 wizard never sets it (the local runtime leaves in C4).
#[derive(Debug, serde::Deserialize)]
pub struct WizardPlan {
    pub ics_url: Option<String>,
    /// The personal calendar's secret iCal address (spec §11a). Same panel as the school feed, and
    /// the same treatment: validated on the device, stored on the account, written into the vault.
    pub personal_calendar: Option<String>,
    pub timezone: String,
    pub slots: Vec<String>,
    pub campus: String,
    pub zybooks: bool,
    pub vhl: bool,
    pub autostart: bool,
    #[serde(default)]
    pub offer_inference: bool,
}
```

The destination, decided by the app:

```rust
/// `<home>\Knowlu\<name>`, with the parent and the backups root created if they are not there — the
/// whole of the folder question, answered by the app (spec §4.1, §11a). Returns the vault path and
/// the backup ROOT: `backup::tick` writes `<root>\<profile_id>\vault`, so the setting is the root and
/// the per-profile folder is the engine's, unchanged.
///
/// Handle-free so `app/tests/onboarding.rs` can drive it with any home directory.
pub fn vault_dest_in(home: &Path, name: &str) -> Result<(PathBuf, PathBuf), String> {
    let (parent, backups) = default_folders_in(home);
    std::fs::create_dir_all(&parent).map_err(|e| format!("{parent}: {e}"))?;
    std::fs::create_dir_all(&backups).map_err(|e| format!("{backups}: {e}"))?;
    let dest = dest_for(&parent, name)?;
    Ok((dest, PathBuf::from(backups)))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("USERPROFILE").map(PathBuf::from).map_err(|_| "no USERPROFILE: Knowlu cannot decide where your vault goes".to_string())
}
```

…and `create_vault_in` takes a name rather than a parent, fills the three cloud fields from Rust, and moves the pending session onto the profile the moment the vault exists:

```rust
/// *Finish* for a new vault. Scaffold+seed (one atomic `create_vault`) → the session moves onto this
/// profile → settings → register.
///
/// **The session move is here, and before the settings write**, for the reason `retarget_credentials`
/// exists (R-P4a-23): the wizard signed in seven panels ago, under `account::PENDING_TARGET`, because
/// the profile id is derived from a vault path that did not exist yet. A vault whose `cloud.yaml`
/// names `knowlu/<profile_id>/session` while the token still sits under `knowlu/pending/session` is a
/// vault that cannot reach the cloud, with nothing anywhere saying why.
pub fn create_vault_in(root: &Path, home: &Path, name: &str, plan: &WizardPlan) -> Value {
    let (dest, backups) = match vault_dest_in(home, name) {
        Ok(d) => d,
        Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }),
    };
    let profile_id = profiles::id_for(&dest);
    let account_id = match crate::account::load_session(crate::account::PENDING_TARGET) {
        Ok((id, _)) => id,
        Err(_) => return json!({ "ok": false, "error": "sign in again — the account this wizard signed in with is no longer on this machine", "profile": Value::Null }),
    };
    let vp = crate::scaffold::VaultPlan {
        profile_id: profile_id.clone(),
        ics_url: plan.ics_url.clone().filter(|u| !u.trim().is_empty()),
        personal_calendar: plan.personal_calendar.clone().filter(|u| !u.trim().is_empty()),
        timezone: plan.timezone.clone(),
        slots: plan.slots.clone(),
        device: knowlu_engine::journal::device_name(),
        campus: plan.campus.clone(),
        zybooks: plan.zybooks,
        vhl: plan.vhl,
        api_base: crate::account::api_base(),
        anon_key: crate::account::anon_key(),
        account_id,
    };
    if let Err(e) = crate::scaffold::create_vault(&dest, &vp) {
        return json!({ "ok": false, "error": e, "profile": Value::Null });
    }
    if let Err(e) = crate::account::move_session(crate::account::PENDING_TARGET, &crate::account::session_target(&profile_id)) {
        let _ = std::fs::remove_dir_all(&dest);
        return json!({ "ok": false, "error": format!("the sign-in could not be attached to this vault ({e}) — the new vault was removed, so nothing is half-made"), "profile": Value::Null });
    }
    finish_or_roll_back(root, &dest, Some(name.to_string()), plan, Some(backups))
}

#[tauri::command(async)]
pub fn create_vault(app: tauri::AppHandle, name: String, plan: WizardPlan) -> Value {
    let root = match app.try_state::<Onboarding>() { Some(o) => o.root.clone(), None => return json!({ "ok": false, "error": "not in onboarding", "profile": Value::Null }) };
    let home = match home_dir() { Ok(h) => h, Err(e) => return json!({ "ok": false, "error": e, "profile": Value::Null }) };
    let out = create_vault_in(&root, &home, &name, &plan);
    if out["ok"] == true { refresh(&app, &root); }
    out
}
```

**This function grows twice more, and both times explicitly.** At Task 12 the plan carries the two
feeds, the slots, the timezone, the campus preset and the three cloud values, and that is all
`VaultPlan` has. **Task 14a** adds the coursework mapping (`zybooks_courses`, `vhl_sections`,
`course_map`) and the slug-making block that fills it from the page's course codes; **Task 14b** adds
`courses`; **Task 14c** adds `campus_choice`. Each of those tasks says so at the step that does it.
Nothing here reads a field that does not exist yet.

`finish_profile_in`, `finish_or_roll_back`, `apply_profile_settings_in` and `restore_vault` all take the backup root as a parameter now instead of reading `plan.backup_dir` — change each signature to `(…, plan: &WizardPlan, backup_dir: Option<PathBuf>)` and set `backup_dir: backup_dir` in the `Settings` they build. `restore_vault`'s caller passes `Some(default_folders_in(&home).1.into())`; `apply_profile_settings` passes `None` (an adopted vault keeps whatever it had). `check_backup_dir` is unchanged and still runs — the default pair are siblings, so it passes by construction.

- [ ] **Step 5: Run the app suite.**

Run: `cargo test -p knowlu`
Expected: `scaffold` and `onboarding` green with the three new tests (`cloud.yaml`'s four keys, `create_vault` writing it, and the `calendars:` list); every other file unchanged; 0 warnings. Any test that still constructs a `VaultPlan` or a `WizardPlan` with the old shape fails to compile — fix each by adding the three cloud fields (use `"https://example.supabase.co/functions/v1"`, `"anon"`, `"acc-1"`) and dropping `backup_dir`; **do not weaken an assertion to make one pass.**

  **One test needs more than that, and it is the one this instruction would otherwise delete.**
  `a_vault_that_cannot_be_finished_is_removed_and_nothing_is_registered` in `app/tests/onboarding.rs`
  is built entirely out of `plan_with(bdir)` and asserts `check_backup_dir`'s three refusals
  (R-P4a-25): a backups folder **inside** the vault, one that **is** the vault, and one **above** it
  that would take the vault's siblings with it. Dropping `backup_dir` from `WizardPlan` deletes its
  subject. **Rewrite it, do not weaken it**: step 4 gives `finish_or_roll_back` a
  `backup_dir: Option<PathBuf>` parameter, so pass each of the three roots there instead of through
  the plan. All three cases stay, and so do their sentences — the check itself is unchanged, only
  where the value comes from.

- [ ] **Step 6: Commit.** `app: the app decides where the vault goes, and writes the account into it — %USERPROFILE%\Knowlu\<name>, Backups beside it, config/cloud.yaml's four keys (C1 Task 12)`.

---

### Task 13: Spike — can a sign-in window reach the calendar feed?

Outcome: **B** — settled 2026-09-10 over three runs on Blackboard Ultra (`https://ualearn.blackboard.com`, campus SSO + Duo inside the window). Tauri 2.11.5 has `data_directory`, `navigate`, `url` and `cookies_for_url`, so all three outcomes were reachable. Run 1: the window works and the share link is page content, not the address — A is out. Run 2: `cookies_for_url` against the window's own current URL — verbatim `https://ualearn.blackboard.com/ultra/institution-page`, where the SSO lands — yields the session (`JSESSIONID`, `BbRouter`, `AWSELB`/`AWSELBCORS`, `samlCookie`, `BbClientCalenderTimeZone`, `COOKIE_CONSENT_ACCEPTED`), and a request the app makes with them is served as the student. Whether the host root alone is a sufficient cookie scope was not tested: Task 14 reads the cookies for the window's current URL, as the spike did. Outcome B never navigates the window, so the calendar page and share-control URLs step 6 asks for are moot — the only URL the capture needs is the feed endpoint below. Run 3: `GET https://ualearn.blackboard.com/webapps/calendar/calendarFeed/url` with those cookies answers **200 with the feed URL as the body** (`https://<host>/webapps/calendar/calendarFeed/<token>/learn.ics`); no XSRF nonce is needed (none exists on the classic pages and the plain GET suffices — the POST 500s without one); `/learn/api/v1/calendars/feed/url` is a 404. Canvas: not tested — Quinn has no Canvas login; Task 14's Canvas body is written from Canvas's documented API and marked unverified.
Courses: **GO** — `GET https://ualearn.blackboard.com/learn/api/public/v1/users/me/courses?expand=course` with the same cookies answers 200 with `{"results":[…]}`; the first membership's keys, verbatim: `availability, course, courseId, courseRoleId, created, dataSourceId, dueDateExceptionType, id, lastAccessed, modified, timeLimitExceptionType, userId`; its `course` object's keys: `allowGuests, availability, courseId, created, dataSourceId, enrollment, externalAccessUrl, externalId, id, locale, name, organization, readOnly, ultraStatus, uuid`. One call, names included. Task 14b keeps the cookie-handover body and its typed-codes fallback.

**Quinn is present at the laptop for this one**, because the only way to answer it is a real campus SSO login with Duo. Spec §11a, ruled 2026-09-09: *the student signs in on the campus's own page, inside a window we opened; the app then navigates that window to the calendar's share page and keeps only the link.* **The standing rule is unchanged: Knowlu never requests campus SSO credentials.** Nothing in this task or the next puts a username or password field on a Knowlu page; the student types into the university's own page, and the app never sees, stores or replays what they typed.

**Throwaway allowed.** What is kept: this task's `Outcome:` line, and whichever of the three capture bodies it selects (Task 14 keeps exactly one).

**Two questions, one login.** The calendar link is the first; **the enrolled course list is the second**
(§11a R-OB-2), and it is asked in the same sitting because it needs the same signed-in window and
Quinn's presence costs the same either way. Blackboard Ultra publishes the signed-in student's own
enrolment at `/learn/api/public/v1/users/me/courses`; Canvas at `/api/v1/courses`; both are the
student's own data, read from the student's own session, and neither needs a credential of ours. Record
a **separate** go/no-go for it: the calendar can prove out while the course list does not, and then
Task 14b ships its typed-codes fallback and nothing else changes.

Three outcomes, decided by a real run:

- **A — URL capture.** `WebviewWindow::navigate()` and `WebviewWindow::url()` both work at our pinned Tauri, and driving the window to the campus's calendar page leaves the feed URL somewhere the app can read (the address itself, a redirect target, or a share dialog whose link becomes the address). `capture` = navigate, poll `url()`, keep the first `.ics`.
- **B — cookie handover.** `url()` is not enough, but the window's cookies for the LMS host are readable from Rust. `capture` = read the cookies, hand them to a `ureq` agent, fetch the campus's calendar-feed endpoint from Rust, keep the link out of its reply. The cookies are held in memory for that one call and dropped.
- **C — neither.** The window still hosts the sign-in and the app still navigates it to the share page; the student copies the link from it into the field beside the window. This is the **fallback Task 14 builds regardless**, so outcome C costs nothing but the primary path.

**Files:**
- Modify: `app/src/lms_link.rs` (the H1 stub), `app/static/index.html`, `app/static/console.js` (a temporary spike button, removed in step 7)

- [ ] **Step 1: Prove which API calls compile.** Write the three commands and the window scaffolding into `app/src/lms_link.rs`:

```rust
//! The LMS calendar link, captured through a sign-in window (spec §11a).
//!
//! **Knowlu never asks for a campus credential.** The student signs in on the university's own page,
//! in a window that has *no capability grant* — so that page cannot reach a single Tauri command —
//! and with its own data directory under the system temp folder, which `close_and_wipe` deletes. We
//! keep the calendar link. We keep nothing else: not the password, not the cookies, not the session.
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// The window's label. One at a time, by construction: `open` closes any previous one first.
pub const WINDOW: &str = "lms-signin";

/// **Not under `%LOCALAPPDATA%\knowlu`.** A campus session's cookies are not app data — they are
/// somebody's live login — so they go to a throwaway directory named for this process and this
/// capture, and they are deleted the moment the capture ends. `app/tests/lms_link.rs` pins that this
/// path is neither the app's own data root nor a profile folder.
pub fn session_dir() -> PathBuf {
    std::env::temp_dir().join(format!("knowlu-lms-session-{}-{}", std::process::id(), knowlu_engine::ids::new_id("cap")))
}

pub fn open_window_at(app: &tauri::AppHandle, url: &str, data_dir: &Path) -> Result<(), String> {
    close_window(app);
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    let _ = std::fs::create_dir_all(data_dir);
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(parsed))
        .title("Sign in to your school")
        .inner_size(1000.0, 760.0)
        .center()
        .data_directory(data_dir.to_path_buf())
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn window_url(app: &tauri::AppHandle) -> Option<String> {
    app.get_webview_window(WINDOW).and_then(|w| w.url().ok()).map(|u| u.to_string())
}

pub fn navigate(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let w = app.get_webview_window(WINDOW).ok_or_else(|| "the sign-in window is not open".to_string())?;
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    w.navigate(parsed).map_err(|e| e.to_string())
}

/// Close the window and delete its data directory. Best effort on both: a window the user already
/// closed is not an error, and a directory WebView2 still has open is retried once the process ends
/// — which is why it lives in the temp folder rather than anywhere we would have to promise about.
pub fn close_and_wipe(app: &tauri::AppHandle, data_dir: &Path) {
    close_window(app);
    let _ = std::fs::remove_dir_all(data_dir);
}

fn close_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.close();
    }
}

#[tauri::command(async)]
pub fn open_lms_window(app: tauri::AppHandle, campus: String) -> Value {
    let Some(url) = crate::scaffold::CAMPUSES.iter().find(|(k, _, _)| *k == campus).map(|(_, _, u)| *u).filter(|u| !u.is_empty()) else {
        return json!({ "ok": false, "error": "no sign-in page is known for that school yet", "opened": false });
    };
    let dir = session_dir();
    match open_window_at(&app, url, &dir) {
        Ok(()) => json!({ "ok": true, "error": Value::Null, "opened": true, "session_dir": dir.to_string_lossy() }),
        Err(e) => json!({ "ok": false, "error": e, "opened": false }),
    }
}

#[tauri::command(async)]
pub fn close_lms_window(app: tauri::AppHandle, session_dir: String) -> Value {
    close_and_wipe(&app, Path::new(&session_dir));
    json!({ "ok": true, "error": Value::Null })
}

/// Filled by Task 14 with the spike's outcome. During the spike it reports what it can see, so the
/// run itself is the evidence.
#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, campus: String) -> Value {
    let _ = campus;
    json!({ "ok": true, "error": Value::Null, "url": window_url(&app), "spike": true })
}
```

Run: `cargo build -p knowlu`
Expected: it compiles. **If `data_directory`, `navigate` or `url` does not exist on this Tauri version**, the compiler says which — record that in the Outcome line and treat the missing one as ruling out its outcome (no `navigate`/`url` rules out A; no `data_directory` is a **stop**, because a window that shares the app's WebView2 profile would leave a campus session behind and that is the one thing this design promises not to do).

- [ ] **Step 2: A temporary spike button.** In `app/static/index.html`, inside `wiz-lms`'s panel, add `<button class="b" id="wiz-spike">Spike: open</button><button class="b" id="wiz-spike2">Spike: read</button><pre class="meta" id="wiz-spike-out"></pre>`; in `console.js`, wire them to `open_lms_window` and `capture_calendar_link` and print the envelope into `#wiz-spike-out`. **These three elements and their handlers are deleted in step 7.**

- [ ] **Step 3: Ask the controller for hand-off H9a.** The three commands cannot be invoked until `main.rs` registers them, and `main.rs` is not this stream's. Post **H9a** — `lms_link` in the `use` line and the **three** commands step 1 defines, and no more: `generate_handler!` on a name that does not resolve is a compile error, which is why H9 is split (R-C1-11). Wait for it on the branch's base.

  **This is the second of the plan's five controller applications, and the third is not far.** In order (R-C1-11): **H1** before Task 10 — pre-flight, already applied, `app/src/lib.rs` plus the four stub modules; **H9a** here, the three commands step 1 defines; **H10** before **Task 14a step 6**, so `coursework-discover` exists to run — correctness-blocking, not compile-blocking, and therefore the easiest of the five to walk past; **H9b** at **Task 14b step 3a**, adding `paste_calendar_link` and `capture_courses` once both resolve; and **H11** before **Task 14c**'s first `cargo test`, because `include_str!("../campuses.json")` will not compile without the asset. Everything else in H3–H8 waits for merge, and no task after 14c pauses at all.

- [ ] **Step 4: Run it, with Quinn.** `cargo run -p knowlu` from a scratch profile (`.\scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-full`, then launch with `--vault <that path>` — **never a real vault**). Walk to the LMS panel, pick *University of Alabama*, press **Spike: open**. Quinn signs in — myBama, then Duo — **in that window, on the university's page**. Then, with the calendar open, press **Spike: read** and record what `url()` returned.

  Then try, in order, until one works, recording each:
  1. **A:** from the console, drive `navigate` to the campus's calendar page and then to its "Share calendar" / "Calendar feed" control; press **Spike: read** after each navigation and note whether a URL containing `.ics` ever appears in the address.
  2. **B:** if not, check whether `app.get_webview_window(WINDOW).unwrap().cookies_for_url(u)` compiles and returns the LMS session cookies (add three lines to `capture_calendar_link` for the run and delete them after). If it does, fetch the campus feed endpoint with `ureq` carrying them and record whether the reply contains a feed URL.
  3. **C:** if neither, record what the share page *does* show and whether the link is selectable and copyable from inside our window.

- [ ] **Step 4a: The course list, in the same window, before you close it.** With Quinn still signed in, drive the window to `https://ualearn.blackboard.com/learn/api/public/v1/users/me/courses` and press **Spike: read**. Record: does the window reach it at all (or redirect to a login, which means the API is on a different session than the UI); does the body come back as JSON; and — verbatim — the **first course object's keys**, because `lms_link::courses_from_json` is written against them. If outcome **A** was chosen for the calendar, note whether the address alone is enough here (it is not — a JSON body is not a URL) and which of the outcome-**B** cookie handover or a `window.eval` read gets the body out. Write the answer as its own line: `Courses: yes|no — <endpoint>, <first object's keys>, <method>`.

- [ ] **Step 5: Do the same for Canvas.** Quinn has no Canvas account; use the public demo (`https://canvas.instructure.com/`) or any Canvas instance he can sign in to, and record whether `Calendar → Calendar Feed` puts the `.ics` URL in the address or only in a dialog. If no Canvas login is available, record **"not tested"** — do not guess. Canvas then ships on the fallback path until somebody can test it, and that is a true statement rather than a broken feature.

- [ ] **Step 6: Write the outcome.** At the top of this task: `Outcome: A|B|C — <which Tauri calls exist; what the UA run showed, step by step; what the Canvas run showed or that it was not tested; the exact campus URLs that worked>` and, on its own line, `Courses: yes|no — …` from step 4a. Task 14b's `course_list_url` and its body-reading half are written from that second line. Also record the **calendar page URL** and the **share/feed control's URL** for UA, verbatim: Task 14's `capture` navigates to them by name.

- [ ] **Step 7: Remove the spike surface.** Delete the two buttons, the `<pre>` and their handlers from `index.html` and `console.js`; leave `lms_link.rs`'s window scaffolding, which Task 14 builds on. `cargo test -p knowlu` green, 0 warnings.

- [ ] **Step 8: Commit.** `app: lms_link — the sign-in window scaffolding, and the spike that chose outcome <letter> (C1 Task 13)`.

---

### Task 14: `app/src/lms_link.rs` — the capture, and the fallback that is always there

Live: University of Alabama (Blackboard Ultra), 2026-09-10 — **156 events across 0 courses**, captured by outcome B and validated through the same fetcher `ingest` uses; the cookie read works with the window in incognito mode; the account copy correctly reported itself as pending (the scratch profile had no session). The 0 is `summarise`'s documented answer for a feed whose `SUMMARY:` lines do not lead with an `AAAA 000` code — the count is cosmetic, the engine's `course_map` does the real placing, and Task 17's panel says "156 events" and adds the courses clause only when it is above zero (R-C1-42). Built with one fix round (R-C1-41); the temporary page buttons used for the run never shipped.

**Files:**
- Modify: `app/src/lms_link.rs`
- Test: `app/tests/lms_link.rs`

**Interfaces:**
- Consumes: Task 13's `WINDOW`, `session_dir`, `open_window_at`, `window_url`, `navigate`, `close_and_wipe`; `scaffold::CAMPUSES`'s third column; `knowlu_engine::calfeed::fetch_ics`.
- Produces: `struct IcsLink { url: String, events: usize, courses: usize }`, `enum CaptureError`, `looks_like_ics(&str) -> bool`, `first_ics_link(&str) -> Option<String>`, `summarise(&str) -> (usize, usize)`, `validate(url, fetch) -> Result<IcsLink, CaptureError>`, `SOURCE_KINDS`, `put_source_at(api_base, token, kind, url)`, `store_source(api_base, session_target, kind, url)`, and the four commands in their final form (`paste_calendar_link` takes a `kind`). Task 18 calls `store_source` with the profile's own session target to back-fill an adopted vault, once per kind. Task 17's panel calls the three; Task 12's `WizardPlan.ics_url` carries the result.

- [ ] **Step 1: Write the failing tests** — `app/tests/lms_link.rs`:

```rust
//! `lms_link.rs`'s pure half, and the two promises the impure half makes. Nothing here opens a
//! window or reaches a network: the fetch is a closure, and the window rules are checked by reading
//! the source, which is the only way to assert "this path is not the app's" without a live WebView2.
use knowlu::lms_link::{first_ics_link, looks_like_ics, session_dir, summarise, validate, CaptureError};

#[test]
fn a_calendar_link_is_recognised_by_shape_and_nothing_else_is() {
    assert!(looks_like_ics("https://lms.example.invalid/webapps/calendar/feed/abc/learn.ics"));
    assert!(looks_like_ics("https://example.instructure.com/feeds/calendars/user_abc.ics"));
    assert!(looks_like_ics("https://lms.example.invalid/calendar/export?format=ics&token=abc"));
    assert!(!looks_like_ics("http://lms.example.invalid/x.ics"), "https only — a feed URL is a password");
    assert!(!looks_like_ics("https://lms.example.invalid/calendar"));
    assert!(!looks_like_ics("nonsense"));
}

#[test]
fn the_first_link_is_taken_out_of_whatever_the_page_gave_us() {
    let html = r#"<a href="https://lms.example.invalid/feed/a.ics">Copy</a> and https://other.invalid/b.ics"#;
    assert_eq!(first_ics_link(html).unwrap(), "https://lms.example.invalid/feed/a.ics");
    assert_eq!(first_ics_link("  https://lms.example.invalid/feed/a.ics  ").unwrap(), "https://lms.example.invalid/feed/a.ics");
    assert_eq!(first_ics_link("no link here"), None);
}

#[test]
fn a_feed_is_summarised_by_events_and_course_codes() {
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:MATH 125 Homework 4\r\nEND:VEVENT\r\n\
               BEGIN:VEVENT\r\nSUMMARY:MATH 125 Quiz 2\r\nEND:VEVENT\r\n\
               BEGIN:VEVENT\r\nSUMMARY:SPAN 101 Lectura\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    assert_eq!(summarise(ics), (3, 2));
    assert_eq!(summarise("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"), (0, 0));
}

#[test]
fn validate_reports_what_the_student_will_see_and_refuses_an_empty_feed() {
    let good = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:MATH 125 Homework 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let link = validate("https://lms.example.invalid/a.ics", &|_| Ok(good.to_string())).expect("validate");
    assert_eq!(link.url, "https://lms.example.invalid/a.ics");
    assert_eq!((link.events, link.courses), (1, 1));
    // An empty parse is a failure, never an empty semester — the same rule `coursework` follows.
    assert!(matches!(validate("https://lms.example.invalid/a.ics", &|_| Ok(String::new())), Err(CaptureError::Empty)));
    assert!(matches!(
        validate("https://lms.example.invalid/a.ics", &|_| Err("connection refused".into())),
        Err(CaptureError::Unreachable(_))
    ));
    assert!(matches!(validate("http://lms.example.invalid/a.ics", &|_| Ok(good.to_string())), Err(CaptureError::NotACalendarLink(_))));
}

/// **One vocabulary, three copies** (Interfaces with C2, item 3): the SQL check constraint, the
/// endpoint's `SOURCE_KINDS`, and this crate's. C2 adds no kind of its own — `google_calendar` is
/// already in all three — so the only way they drift is a careless edit, which is exactly what a
/// cheap cross-language read catches. The same trick
/// `the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire` uses.
#[test]
fn the_source_kind_vocabulary_is_one_list_in_three_places() {
    use knowlu::lms_link::{DEVICE_KINDS, SOURCE_KINDS};
    let sql = std::fs::read_to_string("../cloud/supabase/migrations/20260910000100_accounts.sql").expect("the accounts migration");
    let ts = std::fs::read_to_string("../cloud/supabase/functions/account/handler.ts").expect("the account handler");
    // **Anchor on the table before looking for the constraint.** `consents` also has a
    // `check (kind in (…))`, and it is declared first — a split on the check alone reads `('tos',
    // 'privacy', 'age_18', 'auto_renew')` and fails on `lms_ics` with a message about the wrong table.
    // Slice `create table public.sources` … its terminating `);` first, then look inside that.
    let sources_block = sql
        .split("create table public.sources")
        .nth(1)
        .and_then(|s| s.split("\n);").next())
        .expect("the sources table");
    let constraint = sources_block
        .split("check (kind in (")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .expect("the sources check constraint");
    // Anchored on the declaration, not on the name: `SOURCE_KINDS` is also *used* further down the
    // file, and a split on the bare name would take whichever came first if the two ever swapped.
    let listed = ts
        .split("const SOURCE_KINDS = [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .expect("the endpoint's SOURCE_KINDS declaration");
    // The Rust copy needs no extraction at all — this test imports the constant itself, which is
    // stronger than reading its own crate's source and is why there is no third slice here.
    for kind in SOURCE_KINDS {
        assert!(constraint.contains(kind), "the check constraint does not allow {kind}");
        assert!(listed.contains(kind), "the endpoint does not accept {kind}");
    }
    assert_eq!(listed.matches('"').count() / 2, SOURCE_KINDS.len(), "the endpoint lists a kind this crate does not: {listed}");
    assert_eq!(constraint.matches('\'').count() / 2, SOURCE_KINDS.len(), "the constraint allows a kind this crate does not: {constraint}");
    // …and the device writes only two of the three: `google_calendar` is reserved and written by
    // nobody (R-X-9) — the Google grant lives in C2's `google_accounts` — so nothing here may produce one.
    assert_eq!(DEVICE_KINDS.len(), 2);
    assert!(!DEVICE_KINDS.contains(&"google_calendar"));
}

/// The two rules `validate_for` adds on top of `validate`, driven directly: an unknown kind is refused
/// before anything is fetched, and an **empty personal calendar is a connection, not a failure** —
/// while an empty school feed still is one ("an empty parse is a failure, never an empty semester").
#[test]
fn an_empty_personal_calendar_connects_and_an_empty_school_feed_does_not() {
    use knowlu::lms_link::validate_for;
    let empty = |_: &str| Ok("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".to_string());
    let one = |_: &str| Ok("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Dentist\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_string());
    let url = "https://calendar.google.com/calendar/ical/x/private-def/basic.ics";
    let link = validate_for("calendar_ics", url, &empty).expect("an empty calendar is still connected");
    assert_eq!((link.events, link.courses), (0, 0));
    assert_eq!(link.url, url);
    assert_eq!(validate_for("calendar_ics", url, &one).unwrap().events, 1);
    assert!(matches!(validate_for("lms_ics", url, &empty), Err(CaptureError::Empty)));
    // `google_calendar` is a real kind of the account's, and not one this app may write.
    assert!(matches!(validate_for("google_calendar", url, &one), Err(CaptureError::NotACalendarLink(_))));
    assert!(matches!(validate_for("nonsense", url, &one), Err(CaptureError::NotACalendarLink(_))));
}

/// The account copy of the link, on the wire (Interfaces with C2, item 3). Loopback only: the server
/// is a real socket on `127.0.0.1` and its thread is joined before this returns.
#[test]
fn a_validated_link_is_put_to_the_accounts_sources() {
    use knowlu::lms_link::put_source_at;
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let body = r#"{"kind":"lms_ics"}"#;
        let _ = stream.write_all(
            format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes(),
        );
        String::from_utf8_lossy(&buf[..n]).to_string()
    });
    let out = put_source_at(&format!("http://127.0.0.1:{port}/functions/v1"), "the-access-token", "lms_ics", "https://lms.example.invalid/feed/a.ics");
    let req = handle.join().expect("server thread");
    assert!(out.is_ok(), "{:?}", out.err());
    assert!(req.starts_with("PUT /functions/v1/account/sources "), "{req}");
    assert!(req.to_lowercase().contains("authorization: bearer the-access-token"), "{req}");
    assert!(req.contains("\"kind\":\"lms_ics\""), "{req}");
    assert!(req.contains("https://lms.example.invalid/feed/a.ics"), "{req}");
}

/// …and an unsubscribed account gets a sentence a student can act on, not a status code — and the
/// panel is not blocked by it, because the vault copy is what `ingest` reads until C2 ships.
#[test]
fn a_402_from_sources_is_a_sentence_and_not_a_dead_end() {
    use knowlu::lms_link::put_source_at;
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let body = r#"{"error":"this account has no active subscription"}"#;
        let _ = stream.write_all(
            format!("HTTP/1.1 402 Payment Required\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes(),
        );
    });
    let out = put_source_at(&format!("http://127.0.0.1:{port}/functions/v1"), "t", "lms_ics", "https://lms.example.invalid/a.ics");
    handle.join().expect("server thread");
    assert!(out.unwrap_err().contains("finish subscribing"));
}

/// Spec §11a and the standing rule. Two promises, asserted against the source because there is no
/// other way to assert them without a live WebView2 — and both are the kind of thing a refactor
/// breaks silently.
#[test]
fn the_sign_in_window_keeps_nothing_and_lives_nowhere_near_the_app_data() {
    let src = std::fs::read_to_string("src/lms_link.rs").expect("src/lms_link.rs");
    // Its data directory is the system temp folder's, never `%LOCALAPPDATA%\knowlu`.
    assert!(src.contains("std::env::temp_dir()"), "the session directory must be the temp folder's");
    assert!(!src.contains("app_data_root"), "a campus session must never land in the app's own data");
    assert!(!src.contains("profile_dir"), "…nor in a profile folder");
    // …and it is deleted.
    assert!(src.contains("remove_dir_all"), "close_and_wipe must delete the session directory");
    // Nothing here reads, stores or transmits a campus credential. These words appear in this file
    // only inside the sentence that says so.
    for word in ["password", "set_password", "credentials::write"] {
        let hits = src.matches(word).count();
        assert!(hits <= 1, "{word} appears {hits} times in lms_link.rs — it may only appear in the promise");
    }
    // The session directory is per capture, so two captures never share cookies.
    assert_ne!(session_dir(), session_dir());
}
```

- [ ] **Step 2: Run and watch it fail.** `cargo test -p knowlu --test lms_link` → unresolved imports.

- [ ] **Step 3: The pure half**, appended to `app/src/lms_link.rs`:

```rust
/// What a capture produced, in the words the panel shows: *we found N assignments across M courses.*
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IcsLink {
    pub url: String,
    pub events: usize,
    pub courses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// The student closed the window, or the deadline passed with no link.
    NotFound,
    NotACalendarLink(String),
    Unreachable(String),
    /// The feed fetched and held nothing. An empty parse is a failure, never an empty semester.
    Empty,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::NotFound => write!(f, "no calendar link was found — paste it below instead"),
            CaptureError::NotACalendarLink(u) => write!(f, "{u} does not look like a calendar feed link"),
            CaptureError::Unreachable(e) => write!(f, "the feed could not be read ({e})"),
            CaptureError::Empty => write!(f, "that feed is empty — check you copied the whole link"),
        }
    }
}

/// https, and either an `.ics` path or a query that says so. `http://` is refused outright: a feed
/// URL is a capability — anyone holding it reads the student's schedule — and sending one in the
/// clear on a campus network is not a thing to do once.
pub fn looks_like_ics(url: &str) -> bool {
    if !url.starts_with("https://") { return false; }
    let lower = url.to_ascii_lowercase();
    let path = lower.split('?').next().unwrap_or(&lower);
    path.ends_with(".ics") || lower.contains("format=ics") || lower.contains("/calendar/feed")
}

/// The first `https://…` run in `text` that looks like a feed. Works on a bare URL, on an address
/// bar's value and on a scrap of HTML, because all three are what the capture might hand it.
pub fn first_ics_link(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find("https://") {
        let tail = &rest[at..];
        let end = tail.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>').unwrap_or(tail.len());
        let candidate = &tail[..end];
        if looks_like_ics(candidate) { return Some(candidate.to_string()); }
        rest = &tail[end.max(1)..];
    }
    None
}

/// `(events, courses)`. Courses are counted by the `AAAA 000` code at the head of a `SUMMARY:`, which
/// is how every feed this product has met names them; a feed that names them otherwise reports 0
/// courses and its real event count, which is still a true sentence to show.
pub fn summarise(ics: &str) -> (usize, usize) {
    let mut events = 0usize;
    let mut codes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in ics.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            events += 1;
        } else if let Some(rest) = line.strip_prefix("SUMMARY:") {
            let mut it = rest.split_whitespace();
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                let alpha = a.len() >= 2 && a.len() <= 4 && a.chars().all(|c| c.is_ascii_uppercase());
                let digits = b.len() == 3 && b.chars().all(|c| c.is_ascii_digit());
                if alpha && digits { codes.insert(format!("{a} {b}")); }
            }
        }
    }
    (events, codes.len())
}

/// Fetch it once and say what is in it. `fetch` is a parameter so the tests never touch a network;
/// production passes `knowlu_engine::calfeed::fetch_ics`, which is **the same fetcher `ingest` uses**
/// — a link that validates here is a link that works on the next slot, and that is the point.
pub fn validate(url: &str, fetch: &dyn Fn(&str) -> Result<String, String>) -> Result<IcsLink, CaptureError> {
    if !looks_like_ics(url) { return Err(CaptureError::NotACalendarLink(url.to_string())); }
    let text = fetch(url).map_err(CaptureError::Unreachable)?;
    let (events, courses) = summarise(&text);
    if events == 0 { return Err(CaptureError::Empty); }
    Ok(IcsLink { url: url.to_string(), events, courses })
}
```

- [ ] **Step 4: The capture, in the shape Task 13 chose.** Keep **one** of these three as `capture_calendar_link`'s body and delete the other two.

**Outcome A — URL capture:**

```rust
/// Drive the window to the campus's calendar pages and read the address after each. Polls for at
/// most `CAPTURE_SECONDS`, because the student may still be finishing Duo when this starts.
const CAPTURE_SECONDS: u64 = 90;

#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, campus: String) -> Value {
    // The two URLs Task 13 recorded for this campus, by name.
    let Some(steps) = capture_steps(&campus) else {
        return json!({ "ok": false, "error": "no capture is known for that school yet — paste the link instead", "link": Value::Null });
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(CAPTURE_SECONDS);
    for step in steps.iter().copied() {
        if navigate(&app, step).is_err() {
            return json!({ "ok": false, "error": CaptureError::NotFound.to_string(), "link": Value::Null });
        }
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(400));
            if let Some(found) = window_url(&app).as_deref().and_then(first_ics_link) {
                return finish("lms_ics", &found);
            }
        }
    }
    json!({ "ok": false, "error": CaptureError::NotFound.to_string(), "link": Value::Null })
}
```

**Outcome B — cookie handover:**

```rust
#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, campus: String) -> Value {
    let Some(steps) = capture_steps(&campus) else {
        return json!({ "ok": false, "error": "no capture is known for that school yet — paste the link instead", "link": Value::Null });
    };
    let Some(w) = app.get_webview_window(WINDOW) else {
        return json!({ "ok": false, "error": "the sign-in window is not open", "link": Value::Null });
    };
    // The feed endpoint Task 13 recorded, fetched ONCE with the student's own session cookies. The
    // cookies live in this function's stack and nowhere else: never written, never logged, and gone
    // when it returns.
    let feed = steps[steps.len() - 1];
    let Ok(url) = feed.parse::<tauri::Url>() else {
        return json!({ "ok": false, "error": "the campus feed URL is not a URL", "link": Value::Null });
    };
    let jar: String = match w.cookies_for_url(url.clone()) {
        Ok(cs) => cs.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; "),
        Err(e) => return json!({ "ok": false, "error": format!("the sign-in could not be read ({e})"), "link": Value::Null }),
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into();
    let body = agent.get(feed).header("cookie", &jar).call()
        .and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string())
        .map_err(|e| e.to_string());
    match body {
        Ok(text) => match first_ics_link(&text) {
            Some(found) => finish("lms_ics", &found),
            None => json!({ "ok": false, "error": CaptureError::NotFound.to_string(), "link": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": CaptureError::Unreachable(e).to_string(), "link": Value::Null }),
    }
}
```

**Outcome C — the window shows it, the student copies it:**

```rust
/// Outcome C: the window signs the student in and lands on the share page; the link itself is read
/// off that page by the student and pasted into the field beside it. `paste_link` is the command the
/// field calls, and it is the SAME command the A and B paths fall back to — so this is not a second
/// code path, it is the one that always exists.
#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, campus: String) -> Value {
    let Some(steps) = capture_steps(&campus) else {
        return json!({ "ok": false, "error": "no capture is known for that school yet — paste the link instead", "link": Value::Null });
    };
    for step in steps.iter().copied() {
        let _ = navigate(&app, step);
    }
    json!({ "ok": true, "error": Value::Null, "link": Value::Null, "paste": true })
}
```

…and, in every outcome, these two, which the panel always has:

```rust
/// The campus's capture path, recorded verbatim by Task 13's spike: the calendar page, then the
/// share/feed control. Empty for a campus nobody has walked yet, which is what makes the panel show
/// the paste field instead of pretending.
fn capture_steps(campus: &str) -> Option<&'static [&'static str]> {
    match campus {
        // <Task 13 Outcome: the two UA URLs, verbatim>
        "university-of-alabama" => Some(&["https://ualearn.blackboard.com/ultra/calendar"]),
        _ => None,
    }
}

/// **The link is stored twice, and this is the half that is easy to forget** (Interfaces with C2,
/// item 3). The vault copy goes into `config/ingest.yaml` at Finish, through `WizardPlan.ics_url`;
/// the account copy goes to `PUT /account/sources` **here**, the moment a link validates, because
/// C2's `/ingest/ics` reads that row server-side and an empty `sources` table makes C2's whole ICS
/// path dead on arrival.
///
/// It is also the only place C1 exercises `requireActiveEntitlement` in production, which is how the
/// 402 contract C2 imports gets proved by something that ships.
///
/// **A failure here does not fail the panel.** The vault copy still works, `ingest` still runs on the
/// next slot, and the wizard is not the place to relitigate a subscription — so a 402 becomes one
/// sentence beside the link and everything else becomes a quieter one. The account copy is
/// back-filled by Task 18's `attach_in` for a vault that was onboarded before it worked.
pub fn put_source_at(api_base: &str, token: &str, kind: &str, url: &str) -> Result<(), String> {
    crate::account::check_api_base(api_base)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut res = agent
        .put(&format!("{}/account/sources", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .send_json(&json!({ "kind": kind, "url": url }))
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let _ = res.body_mut().with_config().limit(1 << 16).read_to_string();
    match status {
        200..=299 => Ok(()),
        402 => Err("finish subscribing first — your calendar link is saved on this machine either way".to_string()),
        other => Err(format!("your calendar link is saved on this machine, but we could not save it to your account ({other})")),
    }
}

/// Send the validated link to the account, using whichever session this window has. The wizard has
/// only the **pending** one (the vault does not exist yet); an adopted or finished profile has its
/// own, and Task 18 passes that target instead.
pub fn store_source(api_base: &str, session_target: &str, kind: &str, url: &str) -> Result<(), String> {
    let auth = crate::account::auth_base(api_base)?;
    let token = crate::account::valid_access_token_at(&auth, &crate::account::anon_key(), session_target, jiff::Timestamp::now().as_second())?;
    put_source_at(api_base, &token, kind, url)
}

/// The whole `sources.kind` vocabulary, matching the check constraint in
/// `cloud/supabase/migrations/20260910000100_accounts.sql` and `SOURCE_KINDS` in
/// `cloud/supabase/functions/account/handler.ts`. Three copies, pinned to each other by
/// `app/tests/lms_link.rs::the_source_kind_vocabulary_is_one_list_in_three_places`.
pub const SOURCE_KINDS: [&str; 3] = ["lms_ics", "calendar_ics", "google_calendar"];

/// The two the **device** may write (spec §11a: one panel, both calendars). `google_calendar` is
/// a reserved value nobody writes (R-X-9) — a Google grant has no URL and lives in C2's
/// `google_accounts`, from a token this machine never sees — so `validate_for` refuses it here
/// rather than letting a page invent one.
pub const DEVICE_KINDS: [&str; 2] = ["lms_ics", "calendar_ics"];

/// `validate`, plus the one rule that differs between the two kinds a device may write.
///
/// **An empty school feed is a failure** — "an empty parse is a failure, never an empty semester" is
/// the rule `coursework` has always followed, and a student whose LMS feed returns nothing has a
/// problem worth hearing about. **An empty personal calendar is not**: a correct secret address with
/// nothing inside the fetched window is a student who has not put anything in their calendar yet, and
/// refusing it would be a false negative on the one field they had to go and find.
pub fn validate_for(
    kind: &str,
    url: &str,
    fetch: &dyn Fn(&str) -> Result<String, String>,
) -> Result<IcsLink, CaptureError> {
    if !DEVICE_KINDS.contains(&kind) {
        return Err(CaptureError::NotACalendarLink(format!("{kind} is not a calendar this app connects")));
    }
    match validate(url, fetch) {
        Err(CaptureError::Empty) if kind == "calendar_ics" => Ok(IcsLink { url: url.to_string(), events: 0, courses: 0 }),
        other => other,
    }
}

fn finish(kind: &str, url: &str) -> Value {
    match validate_for(kind, url, &|u| knowlu_engine::calfeed::fetch_ics(u)) {
        Ok(link) => {
            // The account copy. A failure is a `note`, never an `error`: the panel goes on.
            let note = match store_source(&crate::account::api_base(), crate::account::PENDING_TARGET, kind, &link.url) {
                Ok(()) => Value::Null,
                Err(e) => json!(e),
            };
            json!({ "ok": true, "error": Value::Null, "kind": kind, "link": link, "note": note })
        }
        Err(e) => json!({ "ok": false, "error": e.to_string(), "kind": kind, "link": Value::Null, "note": Value::Null }),
    }
}

/// **The fallback, always present** (spec §11a: "the paste-a-link path as the fallback for a campus
/// whose pages defeat it"). It is also the whole of the flow for a student whose school is not in
/// `CAMPUSES` yet — and it validates exactly as the captured path does, so the two produce the same
/// sentence and the same failure.
#[tauri::command(async)]
pub fn paste_calendar_link(kind: String, url: String) -> Value {
    finish(&kind, url.trim())
}
```

- [ ] **Step 5: Run the tests.** `cargo test -p knowlu --test lms_link` → `test result: ok. 9 passed` (five pure-half tests, the vocabulary pin, the `validate_for` rules, and the two loopback ones).

- [ ] **Step 6: One live run, with Quinn.** From the scratch profile again: open the window, sign in, and let the capture run. Record in this task: `Live: <campus> — <events> events across <courses> courses, captured in <seconds>s` (or `fell back to paste`). Then confirm the session directory is gone: `Get-ChildItem $env:TEMP -Filter "knowlu-lms-session-*"` → nothing.

- [ ] **Step 7: `cargo test -p knowlu` green, 0 warnings. Commit.** `app: lms_link — sign in on your school's own page, keep only the calendar link, wipe the session (C1 Task 14)`.

**All four `lms_link` commands ship, whichever outcome Task 13 chose** — `open_lms_window`, `capture_calendar_link`, `paste_calendar_link`, `close_lms_window`. Outcome C makes `capture_calendar_link` navigate and return `paste: true`; the panel calls the same four either way, so the wizard's step set and the hand-off lists do not depend on the outcome.

---
### Task 14a: The coursework mapping — the wizard turns a discovered book into a course (R-OB-1)

Discovered 2026-09-10 — step 6 run ahead of the task, H10 (`d81842c`) against Quinn's own accounts through two throwaway Credential Manager entries deleted in the same step: `zybooks` → `UACS100Fall2026` and `HowToUseZyBooks2` (both `ignored: false`, `mapped: false`); `vhl` → course `1623220`, section `2102121` (`mapped: false`); `errors: []`; exit 0. With no targets the command answers the empty shape; with a missing credential it answers an `errors` entry, exit 0 both times. `suggest_course` is written against these codes.

**Precondition: hand-off H10** (the engine's `coursework-discover`) applied on the branch's base.
**Ask the controller for it at step 6**, where it is first needed — the Rust in this task compiles
without it, because the spawn is by name at run time — the same way Task 13 step 3 asks for **H9a**.
There is one ask, and it is step 6's; do not also post it at step 1.

Quinn's first slot is the specification: the wizard had stored both logins and written
`coursework.zybooks.courses: {}` and `coursework.vhl.sections: {}`, so `coursework` said
`zybook UACS100Fall2026 not in config; skipped`, `section 2102121 not in config; skipped`, and then
`0 assignments parsed; treating as failure` — three warnings for one missing sentence. A wizard that
takes a password and then throws the work away is worse than one that never asked.

**Files:**
- Modify: `app/src/scaffold.rs`, `app/src/onboarding.rs`
- Test: `app/tests/scaffold.rs`, `app/tests/onboarding.rs`

**Interfaces:**
- Consumes: H10's `knowlu-engine coursework-discover`; `scheduler::engine_exe()`; `credentials::target_for`.
- Produces: `scaffold::{BookMapping, SectionMapping, suggest_course}` — and **no `slugify`**: the engine's `knowlu_engine::ingest::slugify` is the one this plan calls, everywhere; `scaffold::VaultPlan`'s
  `zybooks_courses: Vec<BookMapping>`, `vhl_sections: Vec<SectionMapping>` and
  `course_map: Vec<(String, String)>`; a real `coursework:` block out of `ingest_yaml`; and
  `onboarding::discover_coursework`. Task 17's logins panel calls the command and fills the three
  `WizardPlan` fields.

- [ ] **Step 1: Write the failing tests** — append to `app/tests/scaffold.rs`:

```rust
#[test]
fn a_zybook_code_suggests_the_course_it_obviously_is() {
    use knowlu::scaffold::suggest_course;
    use knowlu_engine::ingest::slugify;
    // The real one, from Quinn's own account: an institution prefix, a code, a term.
    assert_eq!(suggest_course("UACS100Fall2026").as_deref(), Some("CS 100"));
    assert_eq!(suggest_course("CS200Spring2027").as_deref(), Some("CS 200"));
    assert_eq!(suggest_course("UAMATH125Fall2026").as_deref(), Some("MATH 125"));
    // No code in it at all: the panel shows the raw name and the student types the course.
    assert_eq!(suggest_course("HowToUseZyBooks2"), None);
    assert_eq!(suggest_course(""), None);
    // The engine's, not a twin: this is the function that decides the note's stem and the key
    // `judge::Heuristics::knows_course` matches, and a second one would diverge in silence.
    assert_eq!(slugify("CS 100"), "cs-100");
    assert_eq!(slugify("GN 103 Hausaufgaben"), "gn-103-hausaufgaben");
    // …including the two behaviours a naive twin gets wrong: the 60-character cap, and a fallback
    // that is never the empty string (which would write `courses/.md`).
    assert_eq!(slugify("!!!"), "item");
    assert_eq!(slugify(&"x".repeat(80)).len(), 60);
}

#[test]
fn a_confirmed_mapping_becomes_the_config_the_engine_reads() {
    use knowlu::scaffold::{ingest_yaml, BookMapping, SectionMapping, VaultPlan};
    let mut p = VaultPlan {
        profile_id: "profile_0123456789".into(),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into()],
        device: "M".into(),
        campus: "none".into(),
        zybooks: true,
        vhl: true,
        zybooks_courses: vec![BookMapping { code: "UACS100Fall2026".into(), course: "cs-100".into(), label: "CS 100".into() }],
        vhl_sections: vec![SectionMapping { section: "2102121".into(), course: "gn-103".into(), label: "GN 103 Hausaufgaben".into() }],
        course_map: vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())],
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    };
    let text = ingest_yaml(&p).expect("ingest.yaml");

    // The three things the engine actually reads, in the shape `route_zybook` and
    // `vhl::parse_dashboard` expect — a non-empty mapping under the code, with `course` and `label`.
    assert!(text.contains("    courses:\n      'UACS100Fall2026':\n        course: 'cs-100'\n        label: 'CS 100'\n"), "{text}");
    assert!(text.contains("    sections:\n      '2102121':\n        course: 'gn-103'\n        label: 'GN 103 Hausaufgaben'\n"), "{text}");
    // zyBooks' own onboarding book has zero assignments and is never coursework. In `ignore:` it is
    // skipped silently; out of it, it is a WARN on every healthy run forever.
    assert!(text.contains("    ignore:\n      - 'HowToUseZyBooks2'\n"), "{text}");
    // …and the blocks `parse_assignments` needs, or every zyBooks item is uncategorised.
    for needed in ["    categories:\n      HW: hw\n", "      minutes_per_section: 6\n", "    importance:\n      hw: 2\n"] {
        assert!(text.contains(needed), "missing {needed:?} in {text}");
    }
    // The course map the ICS ingest and tier-1 judgment both read.
    assert!(text.contains("course_map:\n  'CS 100': 'cs-100'\n  'GN 103': 'gn-103'\n"), "{text}");

    // An empty mapping is an EMPTY block, not `courses: {}` with nothing under it — `route_zybook`
    // treats a falsy mapping as unmapped either way, but a config that lies about what it maps is
    // what produced the first slot this task exists because of.
    p.zybooks_courses.clear();
    p.vhl_sections.clear();
    p.course_map.clear();
    let bare = ingest_yaml(&p).expect("ingest.yaml");
    assert!(bare.contains("    courses: {}\n") && bare.contains("    sections: {}\n"), "{bare}");
    assert!(bare.contains("course_map: {}\n"), "{bare}");
}
```

…and to `app/tests/onboarding.rs`:

```rust
/// The discovery reply the panel renders, parsed and suggested — driven directly, because spawning
/// the engine needs a credential this test must not have.
#[test]
fn discovery_output_becomes_rows_with_a_suggestion_each() {
    use knowlu::onboarding::rows_from_discovery;
    let json = r#"{"errors": [], "vhl": [{"course_id": "1623220", "mapped": false, "section": "2102121"}], "zybooks": [{"code": "UACS100Fall2026", "ignored": false, "mapped": false}, {"code": "HowToUseZyBooks2", "ignored": true, "mapped": false}]}"#;
    let rows = rows_from_discovery(json);
    assert_eq!(rows.len(), 3, "{rows:?}");
    let zy = &rows[0];
    assert_eq!(zy.source, "zybooks");
    assert_eq!(zy.key, "UACS100Fall2026");
    assert_eq!(zy.suggested.as_deref(), Some("CS 100"));
    assert!(!zy.ignored);
    // zyBooks' onboarding book comes back flagged, so the panel can pre-tick "ignore" rather than
    // asking a student what course "HowToUseZyBooks2" is.
    assert!(rows[1].ignored);
    assert_eq!(rows[1].suggested, None);
    let vhl = &rows[2];
    assert_eq!(vhl.source, "vhl");
    assert_eq!(vhl.key, "2102121");
    // The dashboard names no course text at all — only ids — so there is nothing to suggest and the
    // panel asks. Inventing one here would be a guess wearing a suggestion's clothes.
    assert_eq!(vhl.suggested, None);
    assert_eq!(vhl.detail.as_deref(), Some("course 1623220"));

    // A source that could not be reached is rows we do not have, not an error the panel dies on —
    // and the reason survives, because "your VHL password is wrong" and "try again" are different
    // instructions.
    use knowlu::onboarding::errors_from_discovery;
    let payload = r#"{"errors": ["vhl: fetch failed (…)"], "vhl": [], "zybooks": [{"code": "UACS100Fall2026", "ignored": false, "mapped": false}]}"#;
    assert_eq!(rows_from_discovery(payload).len(), 1, "zyBooks still worked");
    assert_eq!(errors_from_discovery(payload), vec!["vhl: fetch failed (…)".to_string()]);
    let failed = rows_from_discovery(r#"{"errors": ["zybooks: fetch failed (…)"], "vhl": [], "zybooks": []}"#);
    assert!(failed.is_empty());
    // …and garbage is empty too: the panel's own copy tells the student to type the mapping.
    assert!(rows_from_discovery("not json").is_empty());
}
```

- [ ] **Step 1a: One `VaultPlan` literal, not two helpers.** `app/tests/scaffold.rs` already has a `plan(id)` helper that builds this struct for three of its tests, and Task 12 added **three** raw `VaultPlan {` literals beside it. A second helper would leave two doing one job. **Widen the existing `plan(id)` into `plan_for(dest: &Path)`** — the profile id is the only thing it was taking, and `id_for(dest)` is where that comes from — then rewrite its three call sites and **two** of Task 12's three literals to call it. **The third keeps its own**: `a_new_vault_carries_the_four_cloud_keys_and_no_secret` compares `cloud_yaml`'s output byte for byte, including `session_credential_target: 'knowlu/profile_0123456789/session'`, so it needs a profile id it chose rather than one `plan_for` derives from a temp path. Add the four new fields to that literal by hand; it is the one place in this file where a literal is the point.

```rust
/// The plan every scaffold test starts from: no feeds, no mappings, no courses — each test sets the
/// one or two fields it is about. `dest` decides the profile id, exactly as `create_vault_in` does.
fn plan_for(dest: &std::path::Path) -> knowlu::scaffold::VaultPlan {
    knowlu::scaffold::VaultPlan {
        profile_id: knowlu::profiles::id_for(dest),
        ics_url: None,
        personal_calendar: None,
        timezone: "America/Chicago".into(),
        slots: vec!["12:00".into(), "18:00".into()],
        device: "M".into(),
        campus: "none".into(),
        zybooks: false,
        vhl: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        api_base: "https://example.supabase.co/functions/v1".into(),
        anon_key: "anon".into(),
        account_id: "acc-1".into(),
    }
}
```

- [ ] **Step 2: Run and watch them fail.** `cargo test -p knowlu --test scaffold --test onboarding` → unresolved imports `suggest_course`, `BookMapping`, `rows_from_discovery`; `VaultPlan` missing four fields. (`slugify` resolves already — it is `knowlu_engine::ingest::slugify`, and there is no second one to write.)

- [ ] **Step 3: `app/src/scaffold.rs`.** Three structs, two helpers, four `VaultPlan` fields, and a real `coursework:` block.

```rust
/// One discovered zyBook, as the student confirmed it. `code` is the vendor's own
/// (`UACS100Fall2026`); `course` is the slug that lands in every task's `course:` field; `label` is
/// what a human reads in the title. The engine's `route_zybook` needs a **non-empty** mapping under
/// the code, and `parse_assignments` reads exactly these two keys out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookMapping {
    pub code: String,
    pub course: String,
    pub label: String,
}

/// One VHL section, likewise. `section` is the id out of the dashboard's `detail_url`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMapping {
    pub section: String,
    pub course: String,
    pub label: String,
}

// **There is no `slugify` here, and there must not be.** `knowlu_engine::ingest::slugify` is `pub`,
// this crate already depends on the engine, and that function is the one that decides the
// `courses/<slug>.md` stem `judge::Heuristics::load` reads and the key `knows_course` matches — and
// the one `zybooks::parse_assignments` uses for `tasks/<slug>.md`. A second implementation would
// diverge silently: the note would be written, the task would be written, and the match would simply
// never happen. It also caps at 60 characters and falls back to `item`, where a naive twin returns an
// empty string and writes `courses/.md`. Call the engine's.

/// `UACS100Fall2026` → `CS 100`. A **suggestion**, not a decision: the student confirms or edits it,
/// and `None` means the panel shows the raw name and asks.
///
/// The rule is the smallest one that fits every code this product has met: find the first run of two
/// to four capitals followed by exactly three digits, and read that as `<LETTERS> <DIGITS>`. It reads
/// past an institution prefix (`UA`) because the letters immediately before the digits are the
/// subject, and it declines `HowToUseZyBooks2` because there is no three-digit number in it — which
/// is the case that matters, since that book is zyBooks' own and is never a course.
pub fn suggest_course(code: &str) -> Option<String> {
    let bytes: Vec<char> = code.chars().collect();
    for start in 0..bytes.len() {
        let letters: String = bytes[start..].iter().take_while(|c| c.is_ascii_uppercase()).collect();
        if letters.len() < 2 || letters.len() > 4 {
            continue;
        }
        let after = start + letters.len();
        let digits: String = bytes[after..].iter().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() != 3 {
            continue;
        }
        // The LAST two-to-four capitals before the digits, so `UACS100` reads `CS 100` and not
        // `UACS 100`: a four-letter run that ends at the digits is preferred only when nothing
        // shorter also ends there, which the loop's forward order gives for free by trying the
        // earliest start first and then continuing — so take the longest suffix of `letters`
        // that is still 2..=4 long and ends where the digits begin.
        let subject: String = letters.chars().rev().take(letters.len().min(4)).collect::<Vec<_>>().into_iter().rev().collect();
        let subject = if subject.len() > 4 { subject[subject.len() - 4..].to_string() } else { subject };
        return Some(format!("{} {}", trim_prefix(&subject), digits));
    }
    None
}

/// `UACS` → `CS` when a two-letter institution prefix is glued to a two-letter subject. Only the
/// prefixes this product has actually met, and never a guess: a four-letter subject like `MATH` is
/// left alone because it is in the list of things that are subjects.
fn trim_prefix(subject: &str) -> String {
    const SUBJECTS: [&str; 12] = ["MATH", "CHEM", "PHYS", "BIOL", "ECON", "HIST", "ENGL", "SPAN", "STAT", "PSYC", "ANTH", "GEOG"];
    if subject.len() == 4 && !SUBJECTS.contains(&subject) && subject.starts_with("UA") {
        return subject[2..].to_string();
    }
    subject.to_string()
}
```

The third struct is the enrolled course. **It is declared here, not in Task 14b**, because the
`VaultPlan` field below names it and a field whose type does not exist is a crate that does not
compile; Task 14b writes the seed loop that reads it:

```rust
/// One course to seed. `code` is what a task's title says (`CS 100`), `slug` what its `course:` field
/// carries, `name` what the student sees. Filled by Task 14b's capture; empty until then.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CourseSeed {
    pub code: String,
    pub name: String,
    pub slug: String,
}
```

`VaultPlan` gains four fields:

```rust
    /// R-OB-1: what the student confirmed on the logins panel. Empty is honest — a wizard run with no
    /// coursework logins has nothing to map — and it is what `ingest_yaml` writes as `{}`.
    pub zybooks_courses: Vec<BookMapping>,
    pub vhl_sections: Vec<SectionMapping>,
    /// R-OB-1 and R-OB-2: `<code fragment> -> <slug>`, read by `ingest::match_course_fields` and by
    /// `judge::Heuristics`. Every confirmed mapping contributes one, and so does every course the
    /// sign-in window found (Task 14b).
    pub course_map: Vec<(String, String)>,
    /// R-OB-2: the enrolled courses, seeded as `courses/<slug>.md` notes (Task 14b).
    pub courses: Vec<CourseSeed>,
```

…and `ingest_yaml`'s `course_map` and `coursework` halves are replaced. The `course_map: {}` line becomes:

```rust
    if p.course_map.is_empty() {
        s.push_str("course_map: {}\n");
    } else {
        s.push_str("course_map:\n");
        for (fragment, slug) in &p.course_map {
            s.push_str(&format!("  {}: {}\n", yaml_scalar("course code", fragment)?, yaml_scalar("course slug", slug)?));
        }
    }
```

…and the `coursework:` block gains, inside the `if p.zybooks` arm, everything the engine reads:

```rust
        if p.zybooks {
            let target = yaml_scalar("zybooks credential target", &crate::credentials::target_for(&p.profile_id, "zybooks"))?;
            s.push_str(&format!("  zybooks:\n    enabled: true\n    credential_target: {target}\n"));
            // `HowToUseZyBooks2` is zyBooks' own onboarding book: zero assignments, never coursework,
            // and out of `ignore:` it is one WARN per healthy run forever (`route_zybook`'s own doc).
            s.push_str("    ignore:\n      - 'HowToUseZyBooks2'\n");
            // What `parse_assignments` reads. Without these three blocks every item is uncategorised
            // and takes the default effort, which is the second half of the first-slot failure.
            s.push_str("    categories:\n      HW: hw\n      Lab: lab\n      Project: project\n");
            s.push_str("    effort:\n      minutes_per_section: 6\n      floors:\n        hw: 0.25\n        lab: 0.5\n        project: 1.0\n");
            s.push_str("    importance:\n      hw: 2\n      lab: 2\n      project: 2\n");
            if p.zybooks_courses.is_empty() {
                s.push_str("    courses: {}\n");
            } else {
                s.push_str("    courses:\n");
                for b in &p.zybooks_courses {
                    s.push_str(&format!(
                        "      {}:\n        course: {}\n        label: {}\n",
                        yaml_scalar("zybook code", &b.code)?,
                        yaml_scalar("zybook course", &b.course)?,
                        yaml_scalar("zybook label", &b.label)?
                    ));
                }
            }
        }
        if p.vhl {
            let target = yaml_scalar("vhl credential target", &crate::credentials::target_for(&p.profile_id, "vhl"))?;
            s.push_str(&format!("  vhl:\n    enabled: true\n    credential_target: {target}\n"));
            s.push_str("    importance: 3\n");
            if p.vhl_sections.is_empty() {
                s.push_str("    sections: {}\n");
            } else {
                s.push_str("    sections:\n");
                for v in &p.vhl_sections {
                    s.push_str(&format!(
                        "      {}:\n        course: {}\n        label: {}\n",
                        yaml_scalar("vhl section", &v.section)?,
                        yaml_scalar("vhl course", &v.course)?,
                        yaml_scalar("vhl label", &v.label)?
                    ));
                }
            }
        }
```

- [ ] **Step 4: `app/src/onboarding.rs` — the discovery command and its parser.**

```rust
/// One row of the mapping panel: a thing the account can reach, and what we think it is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiscoveredRow {
    /// `zybooks` or `vhl`.
    pub source: String,
    /// The vendor's own key: a zyBook code, or a VHL section id.
    pub key: String,
    /// Something to show beside the key when the key alone means nothing to a human.
    pub detail: Option<String>,
    /// What we think the course is. `None` means we do not know and the panel asks.
    pub suggested: Option<String>,
    /// Already placed by this vault's config — the adopt path shows these ticked and quiet.
    pub mapped: bool,
    /// zyBooks' own onboarding book. The panel pre-ticks *ignore* rather than asking.
    pub ignored: bool,
}

/// H10's JSON → rows. **Never fails**: a source that could not be reached, an error list, a truncated
/// reply and outright garbage all come back as "no rows", because the panel's answer to all four is
/// the same — show the student the fields and let them type it.
pub fn rows_from_discovery(json: &str) -> Vec<DiscoveredRow> {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return Vec::new() };
    let mut out = Vec::new();
    for b in v.get("zybooks").and_then(|z| z.as_array()).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(code) = b.get("code").and_then(|c| c.as_str()) else { continue };
        out.push(DiscoveredRow {
            source: "zybooks".into(),
            key: code.to_string(),
            detail: None,
            suggested: crate::scaffold::suggest_course(code),
            mapped: b.get("mapped").and_then(|m| m.as_bool()).unwrap_or(false),
            ignored: b.get("ignored").and_then(|m| m.as_bool()).unwrap_or(false),
        });
    }
    for sec in v.get("vhl").and_then(|z| z.as_array()).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(id) = sec.get("section").and_then(|c| c.as_str()) else { continue };
        out.push(DiscoveredRow {
            source: "vhl".into(),
            key: id.to_string(),
            // The dashboard names no course text — only ids — so this is all there is to show.
            detail: sec.get("course_id").and_then(|c| c.as_str()).map(|c| format!("course {c}")),
            suggested: None,
            mapped: sec.get("mapped").and_then(|m| m.as_bool()).unwrap_or(false),
            ignored: false,
        });
    }
    out
}

/// H10's `errors` array, as sentences. Empty for anything unparseable — the caller already has a
/// sentence for "we got nothing at all", and two of them would be worse than one.
pub fn errors_from_discovery(json: &str) -> Vec<String> {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|v| v.get("errors").and_then(|e| e.as_array()).cloned())
        .map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Run H10's subcommand and turn its answer into rows. The credential targets are derived from the
/// path the wizard is about to create — the same derivation `store_credentials` used a panel ago, so
/// discovery reads the entries that panel just wrote.
///
/// `(async)` and **never fatal**: two vendor logins over a student's wifi is the slowest thing in the
/// wizard, and every failure is an empty list plus a sentence, because the panel can always be typed
/// into. A missing engine is that same empty list.
#[tauri::command(async)]
pub fn discover_coursework(vault: String, zybooks: bool, vhl: bool) -> Value {
    let id = profiles::id_for(Path::new(&vault));
    let mut args: Vec<String> = vec!["coursework-discover".into()];
    if zybooks {
        args.push("--zybooks-target".into());
        args.push(crate::credentials::target_for(&id, "zybooks"));
    }
    if vhl {
        args.push("--vhl-target".into());
        args.push(crate::credentials::target_for(&id, "vhl"));
    }
    let exe = match crate::scheduler::engine_exe() {
        Ok(e) => e,
        Err(e) => return json!({ "ok": true, "error": Value::Null, "rows": [], "note": format!("we could not look up your courses ({e}) — fill them in below") }),
    };
    use knowlu_engine::childproc::NoConsole;
    let out = std::process::Command::new(exe).no_console().args(&args).output();
    let stdout = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => return json!({ "ok": true, "error": Value::Null, "rows": [], "note": format!("we could not look up your courses ({e}) — fill them in below") }),
    };
    let rows = rows_from_discovery(&stdout);
    // The per-source reason, not just "something went wrong": a student whose zyBooks worked and whose
    // VHL did not needs to hear *VHL*, because the fix is their VHL password and not a retry.
    let reasons = errors_from_discovery(&stdout);
    let note = match (rows.is_empty(), reasons.is_empty()) {
        (_, false) => json!(format!("{} — fill those in below.", reasons.join("; "))),
        (true, true) => json!("we could not reach your coursework sites — fill them in below"),
        (false, true) => Value::Null,
    };
    json!({ "ok": true, "error": Value::Null, "rows": rows, "note": note })
}
```

`WizardPlan` gains the three fields the panel fills, all `#[serde(default)]` so an older page is not a refusal:

```rust
    #[serde(default)]
    pub zybooks_courses: Vec<crate::scaffold::BookMapping>,
    #[serde(default)]
    pub vhl_sections: Vec<crate::scaffold::SectionMapping>,
    #[serde(default)]
    pub course_map: Vec<(String, String)>,
    #[serde(default)]
    pub courses: Vec<crate::scaffold::CourseSeed>,
```

…with `serde::Deserialize` added to `BookMapping` and `SectionMapping`'s derives.

**And `onboarding::create_vault_in` grows the block Task 12 pointed forward to.** Task 12 built a
`VaultPlan` out of the feeds, the slots, the campus preset and the three cloud values, because those
were all `VaultPlan` had; the four fields above are what this task adds to it. Insert this immediately
before the `let vp = crate::scaffold::VaultPlan {` literal, and add the four names to that literal
after `personal_calendar`:

```rust
    // **The page sends course CODES; the slugs are made here.** `CS 100` is what a student types and
    // what a title says; `cs-100` is the vault's own name for it — the note's stem, every task's
    // `course:` field, and the value `judge::Heuristics::knows_course` tests. A page that invented
    // vault identifiers would be a page deciding what the engine may know (R-OB-1, R-OB-2).
    // The engine's `slugify` — the one `judge::Heuristics` reads back (see the note in `scaffold.rs`).
    let slug = knowlu_engine::ingest::slugify;
    let zybooks_courses: Vec<crate::scaffold::BookMapping> = plan.zybooks_courses.iter()
        .filter(|b| !b.label.trim().is_empty())
        .map(|b| crate::scaffold::BookMapping { code: b.code.clone(), course: slug(&b.label), label: b.label.clone() })
        .collect();
    let vhl_sections: Vec<crate::scaffold::SectionMapping> = plan.vhl_sections.iter()
        .filter(|v| !v.label.trim().is_empty())
        .map(|v| crate::scaffold::SectionMapping { section: v.section.clone(), course: slug(&v.label), label: v.label.clone() })
        .collect();
    let course_map: Vec<(String, String)> = plan.course_map.iter()
        .map(|(code, _)| (code.clone(), slug(code)))
        .filter(|(code, s)| !code.trim().is_empty() && !s.is_empty())
        .collect();
    // `courses` is empty until Task 14b's capture fills it, and empty is a correct answer: a student
    // whose campus we cannot read types the list on the panel instead.
    let courses: Vec<crate::scaffold::CourseSeed> = plan.courses.iter()
        .map(|c| crate::scaffold::CourseSeed {
            code: c.code.clone(),
            name: if c.name.trim().is_empty() { c.code.clone() } else { c.name.clone() },
            slug: if c.slug.trim().is_empty() { slug(&c.code) } else { c.slug.clone() },
        })
        .filter(|c| !c.slug.is_empty())
        .collect();
```

- [ ] **Step 5: Run the tests.** `cargo test -p knowlu --test scaffold --test onboarding` — the two new scaffold tests and the new onboarding one pass; every existing test in both files still passes, with the four new `VaultPlan` fields set to `Vec::new()` in the one place step 1a left them: `plan_for`. Any literal `onboarding.rs` still builds gets them by hand.

- [ ] **Step 6: Prove it against Quinn's own account, once, by hand — and ask for hand-off H10 first.**
  `coursework-discover` is an `engine/` subcommand and `engine/` is not this stream's: the Rust in this
  task compiles without it (the spawn is by name, at run time), but this step cannot run until it
  exists. Post the **H10** block and wait for it on the branch's base — it is the third of the plan's
  five controller applications (R-C1-11: H1 → H9a → H10 → H9b → H11), and unlike the other four it is
  correctness-blocking rather than compile-blocking, which is why it is easy to walk past. With the
  scratch profile's credentials stored, run the subcommand directly and read what comes back:

```powershell
target\debug\knowlu-engine.exe coursework-discover --zybooks-target "knowlu/<the scratch profile id>/zybooks" --vhl-target "knowlu/<the scratch profile id>/vhl"
```

  Expect one JSON object naming `UACS100Fall2026`, `HowToUseZyBooks2` with `"ignored": false` (nothing
  is in `ignore:` yet — the wizard is what puts it there) and the VHL section. Record the codes it
  actually returned above step 1: they are the evidence that `suggest_course` was written against real
  input rather than a guess. **Never against Quinn's live vault** — the scratch profile only.

- [ ] **Step 7: Commit.** `app: onboarding maps the discovered zyBooks and VHL sources to courses — the config the engine reads, instead of courses: {} (C1 Task 14a, R-OB-1)`.

---

### Task 14b: The enrolled courses, out of the sign-in window (R-OB-2)

The second half of the same failure: with no `courses/` notes and no `course_map`, every ICS task is
`course: null`, effort 1.0, `needs_enrichment: true`. The judgment gap is accepted; being unable to
name a course the student is *enrolled in* is not.

**The ruled fallback, before anything else in this task.** Task 13 step 4a records a `Courses:` answer
that is **separate** from the calendar's outcome, and one of its possible values is *no method got a
JSON body out of the window*. That is **outcome C for the course list only**: `capture_courses` returns
an empty list, the panel shows its typed-codes field, and **this task is not blocked** — everything
below still ships, and the calendar capture that already worked is untouched. Nothing in this task
depends on the capture succeeding; the capture is the fast path and the typing is the guarantee.

**Files:**
- Modify: `app/src/lms_link.rs`, `app/src/scaffold.rs`
- Test: `app/tests/lms_link.rs`, `app/tests/scaffold.rs`

**Interfaces:**
- Consumes: Task 13's window scaffolding and its **second** go/no-go answer; `write::create` and `console_ctx()`.
- Produces: `lms_link::{Course, courses_from_json, capture_courses}`; `scaffold::CourseSeed` and the
  course notes `seed_writes` creates. Task 14a's `course_map` carries one entry per course.

- [ ] **Step 1: Write the failing tests** — append to `app/tests/lms_link.rs`:

```rust
/// Both LMSs answer with a list of objects; the two shapes differ only in field names, and neither is
/// ours to choose. Blackboard Ultra: `/learn/api/public/v1/users/me/courses` → `{"results":[{"courseId":
/// "UACS100Fall2026","course":{"name":"CS 100 Intro"}}]}`. Canvas: `/api/v1/courses` →
/// `[{"course_code":"CS100","name":"Intro to CS"}]`. One reader, both shapes, and anything else is an
/// empty list rather than a panic.
#[test]
fn an_enrolled_course_list_is_read_from_either_lms_shape() {
    use knowlu::lms_link::courses_from_json;
    let blackboard = r#"{"results":[{"courseId":"UACS100Fall2026","course":{"name":"CS 100 Intro to Computer Science"}},{"courseId":"UAGN103Fall2026","course":{"name":"GN 103 German"}}]}"#;
    let got = courses_from_json(blackboard);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].code, "UACS100Fall2026");
    assert_eq!(got[0].name, "CS 100 Intro to Computer Science");
    // The slug is what a task's `course:` field and the note's filename both carry, and it comes from
    // the SUGGESTED code, not from the vendor's key — `ua-cs-100-fall-2026` would be nobody's idea of
    // a course.
    assert_eq!(got[0].slug, "cs-100");
    let canvas = r#"[{"course_code":"CS100","name":"Intro to CS","id":42}]"#;
    let got = courses_from_json(canvas);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].code, "CS100");
    assert_eq!(got[0].slug, "cs-100");
    // A course with no code we can read keeps its name and slugs from that, rather than being dropped.
    let odd = r#"[{"name":"Independent Study"}]"#;
    assert_eq!(courses_from_json(odd)[0].slug, "independent-study");
    for junk in ["", "null", "{}", "not json", r#"{"results":"nope"}"#] {
        assert!(courses_from_json(junk).is_empty(), "{junk}");
    }
}
```

…and to `app/tests/scaffold.rs`:

```rust
/// R-OB-2: one note per enrolled course, in the shape `judge::Heuristics::load` reads — the stem is
/// the slug, the frontmatter carries `title` and `slug`, and the `## Grade weights` heading is there
/// and empty, because the weights are the student's to write and the model's to read.
#[test]
fn every_enrolled_course_becomes_a_note_the_engine_can_find() {
    use knowlu::scaffold::{create_vault, CourseSeed, VaultPlan};
    let root = std::env::temp_dir().join(format!("knowlu-courses-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let mut p = plan_for(&dest);          // the helper the other scaffold tests already use
    p.courses = vec![
        CourseSeed { code: "CS 100".into(), name: "CS 100 Intro to Computer Science".into(), slug: "cs-100".into() },
        CourseSeed { code: "GN 103".into(), name: "GN 103 German".into(), slug: "gn-103".into() },
    ];
    p.course_map = vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())];
    create_vault(&dest, &p).expect("create");

    for (slug, title) in [("cs-100", "CS 100 Intro to Computer Science"), ("gn-103", "GN 103 German")] {
        let note = dest.join("courses").join(format!("{slug}.md"));
        let text = knowlu_engine::pystr::read_text(&note).unwrap_or_else(|e| panic!("{}: {e}", note.display()));
        assert!(text.contains(&format!("title: {title}")), "{text}");
        assert!(text.contains(&format!("slug: {slug}")), "{text}");
        assert!(text.contains("## Grade weights"), "{text}");
        // Every note has an opaque id, like every other note this app writes.
        assert!(text.contains("id: course_"), "{text}");
    }
    // …and the engine agrees it knows them: this is the predicate tier-1 judgment uses.
    let h = knowlu_engine::judge::Heuristics::load(&dest);
    assert!(h.knows_course("cs-100") && h.knows_course("gn-103"));
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 2: Run and watch them fail.**

- [ ] **Step 3: `app/src/lms_link.rs` — the reader and the capture.**

```rust
/// One course the student is enrolled in, as the LMS names it and as the vault will.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Course {
    /// The LMS's own key — Blackboard's `courseId`, Canvas's `course_code`.
    pub code: String,
    /// What the student sees in their LMS.
    pub name: String,
    /// `cs-100`. The vault's own name for it: the note's stem, and every task's `course:` field.
    pub slug: String,
}

/// Both shapes, one reader (spec §11a R-OB-2). Blackboard Ultra answers
/// `{"results":[{"courseId":…,"course":{"name":…}}]}`; Canvas answers a bare array of
/// `{"course_code":…,"name":…}`. **Never panics and never guesses**: a body it does not recognise is
/// an empty list, and an empty list is what puts the typed-codes fallback on screen.
pub fn courses_from_json(body: &str) -> Vec<Course> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return Vec::new() };
    let items: Vec<&serde_json::Value> = match (&v, v.get("results")) {
        (serde_json::Value::Array(a), _) => a.iter().collect(),
        (_, Some(serde_json::Value::Array(a))) => a.iter().collect(),
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for it in items {
        let code = it
            .get("courseId")
            .or_else(|| it.get("course_code"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();
        let name = it
            .get("course")
            .and_then(|c| c.get("name"))
            .or_else(|| it.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string();
        if code.is_empty() && name.is_empty() {
            continue;
        }
        // The slug comes from the SUGGESTED code where there is one — `ua-cs-100-fall-2026` is
        // nobody's idea of a course — and from the name otherwise.
        let slug = crate::scaffold::suggest_course(&code)
            .map(|c| knowlu_engine::ingest::slugify(&c))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| knowlu_engine::ingest::slugify(if name.is_empty() { &code } else { &name }));
        out.push(Course { code, name, slug });
    }
    out
}

/// The window's second job (Task 13's second go/no-go). It navigates the signed-in window to the
/// campus's own course endpoint and reads what comes back — **the student's own enrolment, from the
/// student's own session**, and nothing else. Empty is the honest answer whenever the spike's outcome
/// was C, whenever the campus is not one we have walked, and whenever the page returns anything this
/// reader does not recognise; the panel then shows the typed-codes fallback.
#[tauri::command(async)]
pub fn capture_courses(app: tauri::AppHandle, campus: String) -> Value {
    let Some(url) = course_list_url(&campus) else {
        return json!({ "ok": true, "error": Value::Null, "courses": [], "typed": true });
    };
    if navigate(&app, url).is_err() {
        return json!({ "ok": true, "error": Value::Null, "courses": [], "typed": true });
    }
    let body = read_current_document(&app, url);
    let courses = courses_from_json(&body);
    json!({ "ok": true, "error": Value::Null, "courses": courses, "typed": courses.is_empty() })
}

/// The window is on a JSON document now, and **an address is not a body** — so Task 13's calendar
/// outcome does not settle this one on its own. Keep exactly one of the three below, per step 4a's
/// `Courses:` line, and delete the other two.
///
/// **Outcome B — cookie handover.** The window's cookies for the LMS host, handed to one `ureq` GET
/// from Rust. The cookies live in this function's stack and nowhere else.
fn read_current_document(app: &tauri::AppHandle, url: &str) -> String {
    let Some(w) = app.get_webview_window(WINDOW) else { return String::new() };
    let Ok(parsed) = url.parse::<tauri::Url>() else { return String::new() };
    let Ok(cookies) = w.cookies_for_url(parsed) else { return String::new() };
    let jar: String = cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; ");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into();
    agent
        .get(url)
        .header("cookie", &jar)
        .header("accept", "application/json")
        .call()
        .and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string())
        .unwrap_or_default()
}

/// **Outcome A' — read it out of the document.** Only if the pinned Tauri's `eval` can return a
/// value, or the window can post one back; Task 13 step 4a records whether it can.
#[allow(dead_code)]
fn read_current_document_by_eval(app: &tauri::AppHandle, _url: &str) -> String {
    let Some(_w) = app.get_webview_window(WINDOW) else { return String::new() };
    // <Task 13 step 4a: the exact call that returned the body, verbatim>
    String::new()
}

/// **Outcome C — no method got a body out.** This is the ruled fallback, and it is not a failure of
/// this task: `capture_courses` returns an empty list, the panel shows the typed-codes field, and
/// **Task 14b ships**. Nothing else changes — the calendar link can prove out while the course list
/// does not, which is exactly why step 4a records a separate `Courses:` answer.
#[allow(dead_code)]
fn read_current_document_empty(_app: &tauri::AppHandle, _url: &str) -> String {
    String::new()
}

/// The campus's own course endpoint, recorded verbatim by Task 13's spike — Blackboard Ultra's
/// `/learn/api/public/v1/users/me/courses`, Canvas's `/api/v1/courses`. `None` for a campus nobody has
/// walked, which is what makes the panel ask instead of pretending.
fn course_list_url(campus: &str) -> Option<&'static str> {
    match campus {
        // <Task 13 Outcome: the UA course-list URL, verbatim>
        "university-of-alabama" => Some("https://ualearn.blackboard.com/learn/api/public/v1/users/me/courses"),
        _ => None,
    }
}
```

- [ ] **Step 3a: Ask the controller for hand-off H9b.** `capture_courses` now exists (step 3) and
  `paste_calendar_link` has since Task 14 step 4, so the two names left out of **H9a** resolve. Post the
  H9b block and wait for it on the branch's base, **before step 5's `cargo test`** — the page invokes
  `capture_courses` and an unregistered command answers "not allowed by the ACL", not an error anybody
  would read as a missing registration. This is the fourth of the plan's five controller applications
  (R-C1-11: H1 → H9a → H10 → H9b → H11), and the last one that touches `main.rs`.

- [ ] **Step 4: `app/src/scaffold.rs` — the seed notes.** `CourseSeed` already exists: Task 14a step 3
declared it, because `VaultPlan.courses` names it. This is the seed loop for the struct Task 14a
declared — `seed_writes` gains, after the first task and before it returns:

```rust
    // R-OB-2: one note per enrolled course, so tier-1 judgment can place a task and the model has a
    // slug it is allowed to use (`judge::Heuristics::knows_course` tests exactly this).
    // `## Grade weights` is present and empty on purpose: the weights are the student's to write and
    // the judgment's to read, and an invented weight would be a number nobody chose.
    for c in &plan.courses {
        let front = Node::map(vec![
            ("title", Node::text(&c.name)),
            ("slug", Node::text(&c.slug)),
            ("code", Node::text(&c.code)),
            ("status", Node::text("active")),
        ]);
        let body = format!(
            "---\n{}---\n\n## Grade weights\n\nFill this in from your syllabus — Knowlu uses it to decide what matters.\n",
            safe_dump_block(&front)
        );
        write::create(vault, &format!("courses/{}.md", c.slug), &body, &crate::commands::console_ctx(), &mut journal, None)
            .map_err(|e| e.to_string())?;
    }
```

…and `build_into`'s folder list gains `"courses"` if it is not already there (it is — `create_vault`
already makes `courses/`; check before adding).

- [ ] **Step 4a: One more scaffold test — every mapped course is a course the vault knows.** The
  mapping block and the captured course list are filled on two different panels and can name the same
  course two ways (`CS 100` typed on one, `CS100` captured on the other), and nothing so far would say
  so: the config would be written, the notes would be written, and `route_zybook` would place a task
  under a slug `judge::Heuristics::knows_course` does not match. The exit gate catches it on a real
  run; this catches it before one.

```rust
/// Every `course:` a coursework mapping names must be a slug the vault knows — a seeded course note,
/// or a `course_map` target. Two panels fill these, and a slug that matches nothing is a task filed
/// under a course that does not exist.
#[test]
fn every_mapped_course_is_a_slug_the_vault_knows() {
    use knowlu::scaffold::{create_vault, BookMapping, CourseSeed, SectionMapping};
    let root = std::env::temp_dir().join(format!("knowlu-mapped-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("Fall 2026");
    let mut p = plan_for(&dest);
    p.zybooks = true;
    p.vhl = true;
    p.zybooks_courses = vec![BookMapping { code: "UACS100Fall2026".into(), course: "cs-100".into(), label: "CS 100".into() }];
    p.vhl_sections = vec![SectionMapping { section: "2102121".into(), course: "gn-103".into(), label: "GN 103".into() }];
    p.courses = vec![CourseSeed { code: "CS 100".into(), name: "CS 100 Intro".into(), slug: "cs-100".into() }];
    p.course_map = vec![("CS 100".into(), "cs-100".into()), ("GN 103".into(), "gn-103".into())];
    create_vault(&dest, &p).expect("create");

    let h = knowlu_engine::judge::Heuristics::load(&dest);
    for slug in p.zybooks_courses.iter().map(|b| &b.course).chain(p.vhl_sections.iter().map(|v| &v.course)) {
        assert!(h.knows_course(slug), "{slug} is mapped and the vault does not know it");
    }
    // …and `gn-103` is known by the map alone, with no note behind it — which is the whole reason
    // `knows_course` tests both. A student who has a VHL section and no Blackboard course for it is
    // not a broken vault.
    assert!(!dest.join("courses").join("gn-103.md").exists());
    assert!(h.knows_course("gn-103"));
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 5: Run the tests.** `cargo test -p knowlu --test lms_link --test scaffold` → the three new tests pass, `lms_link` at 10 and `scaffold` with four more than before Task 14a.

- [ ] **Step 6: Commit.** `app: the sign-in window seeds the enrolled courses — a note and a course_map line each, so the first ingest is not 28 tasks with no course (C1 Task 14b, R-OB-2)`.

---

### Task 14c: The school, chosen from every US institution (R-OB-4)

Two radio buttons was a placeholder that read like a decision. §11a, ruled 2026-09-09: the school is
picked from **every active two- and four-year US institution** — 4,319 of them (HD2024 has 6,072 rows; the rest are inactive or less-than-two-year, R-C1-37) — by typing part of the
name. The curated layer does not go away; it stops being the *whole* list and becomes what a curated
school gets **on top** of it: event feeds, a known LMS, a known sign-in URL.

**Precondition: hand-off H11** has been applied and `app/campuses.json` is committed. Ask the
controller for it at step 1 — it is a script run, not a decision, and it takes one minute.

**Files:**
- Create: `app/campuses.json` (by H11's script; committed — the controller's file, read here)
- Modify: `app/src/scaffold.rs`, `app/src/onboarding.rs`, `app/src/lms_link.rs`
- Test: `app/tests/onboarding.rs`, `app/tests/scaffold.rs`, `app/tests/lms_link.rs` — **not `static_assets.rs`**: its `read()` helper resolves against `app/static/` only, and `app/campuses.json` is not under `app/static/` (step 2 says why).

**Interfaces:**
- Consumes: H11's asset.
- Produces: `scaffold::{Curated, CAMPUSES, curated, STATE_TZ, state_timezone, CampusChoice, campus_config_yaml}`;
  `VaultPlan.campus_choice`; `config/campus.yaml` in every new vault. Task 17's panel is the typeahead;
  Task 14's `capture_steps` and Task 14b's `course_list_url` key on the **unitid**.

**Why `config/campus.yaml` and not a block in `ingest.yaml`.** Three reasons, in order of weight.
`config/ingest.yaml` is the *engine's* ingest configuration — `calfeed`, `coursework` and
`judge::Heuristics` all parse it — and the school's identity is not ingest configuration; putting it
there would mean the engine parsing a key no engine code reads. `config/events.yaml` is already the
campus *feeds* file and is a copied preset, not a place to write per-install values. And C2 needs to
read the LMS kind server-side, which is easier from a file whose whole content is the answer. One file,
one purpose, the same shape `config/cloud.yaml` established:

```yaml
unitid: '100751'
name: 'The University of Alabama'
state: 'AL'
lms: 'blackboard'
curated: true
```

- [ ] **Step 1: Ask the controller for H11**, then check what landed: `app/campuses.json` exists, its first line reads `{"source":"NCES IPEDS HD2024.csv","retrieved":…,"count":…`, and `(Get-Item app\campuses.json).Length / 1KB` is under 600.

- [ ] **Step 2: Write the failing tests.** The asset's own tests live in `app/tests/onboarding.rs`, not `static_assets.rs`: `campuses.json` is not a static asset any more — the page never loads it — and `static_assets.rs`'s `read()` helper only reaches `static/`. First, in `app/tests/onboarding.rs`:

```rust
/// R-OB-4: the school list is a committed asset, because a typeahead that needs a network call to
/// show a school does not work in a dorm on move-in day — which is most first runs.
#[test]
fn the_campus_list_is_bundled_headed_and_small() {
    let raw = std::fs::read_to_string("campuses.json").expect("app/campuses.json");
    // The header is the first line, and it is what makes "is this current?" answerable without
    // re-downloading a federal zip.
    let head = raw.lines().next().expect("a first line");
    assert!(head.starts_with("{\"source\":\"NCES IPEDS HD"), "the header names its source file: {head}");
    assert!(head.contains("\"retrieved\":\"20"), "…and when it was taken: {head}");
    assert!(head.contains("\"count\":"), "…and how many schools it holds: {head}");

    let v: serde_json::Value = serde_json::from_str(&raw).expect("campuses.json is one JSON object");
    let rows = v["campuses"].as_array().expect("campuses is an array");
    assert_eq!(rows.len() as u64, v["count"].as_u64().expect("count is a number"), "the header's count is the array's length");
    // 4,319 active two- and four-year institutions in HD2024 (of 6,072 rows; R-C1-37). A file that suddenly holds 40 of them is
    // a script that half-ran, and a bundle that holds 40,000 is one that stopped filtering.
    assert!(rows.len() > 4_000 && rows.len() < 8_000, "{} schools is not a US institution list", rows.len());

    // `[unitid, name, city, state, host]`, and the host is a HOST: this test is the guard that the
    // bundled list carries no `http(s)://` literal — `static_assets.rs` cannot be, because its
    // `read()` helper resolves against `app/static/` and this asset is `app/campuses.json`.
    let first = rows[0].as_array().expect("a row is an array");
    assert_eq!(first.len(), 5, "a row is [unitid, name, city, state, host]");
    assert!(first[0].is_number() && first[1].is_string() && first[3].is_string());
    assert!(!raw.contains("http://") && !raw.contains("https://"), "the list carries hosts, never URLs");

    // The size guard the installer cares about. H11's script refuses to write past this too.
    let kb = raw.len() / 1024;
    assert!(kb < 600, "campuses.json is {kb} KB");
}

/// Every curated school must be **in** the bundled list, or picking it from the typeahead and then
/// looking it up in `CAMPUSES` would answer with two different schools.
#[test]
fn every_curated_campus_is_in_the_bundled_list() {
    let raw = std::fs::read_to_string("campuses.json").expect("app/campuses.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("campuses.json");
    let rows = v["campuses"].as_array().expect("campuses");
    for c in knowlu::scaffold::CAMPUSES {
        let want: u64 = c.unitid.parse().expect("a unitid is a number");
        let found = rows.iter().find(|r| r[0].as_u64() == Some(want)).unwrap_or_else(|| panic!("{} ({}) is not in campuses.json", c.label, c.unitid));
        // …and it is the school we think it is. A curated row that named the wrong unitid would send
        // a student's sign-in window to another university's LMS.
        let name = found[1].as_str().unwrap_or_default();
        assert!(name.to_lowercase().contains(&c.label.to_lowercase()) || c.label.to_lowercase().contains(&name.to_lowercase()),
            "{} is unitid {} in our table and {name:?} in IPEDS", c.label, c.unitid);
    }
}
```

…and `app/tests/scaffold.rs`:

```rust
#[test]
fn a_chosen_school_becomes_campus_yaml_and_a_timezone_suggestion() {
    use knowlu::scaffold::{campus_config_yaml, curated, state_timezone, CampusChoice};
    let ua = CampusChoice { unitid: "100751".into(), name: "The University of Alabama".into(), state: "AL".into(), lms: "blackboard".into() };
    let text = campus_config_yaml(&ua).expect("campus.yaml");
    assert_eq!(
        text,
        "unitid: '100751'\nname: 'The University of Alabama'\nstate: 'AL'\nlms: 'blackboard'\ncurated: true\n"
    );
    // A school nobody has curated is still a school: it gets a file, no event feeds, and an LMS the
    // sign-in window (or the student) names.
    let other = CampusChoice { unitid: "999999".into(), name: "Somewhere Community College".into(), state: "OR".into(), lms: String::new() };
    let text = campus_config_yaml(&other).expect("campus.yaml");
    assert!(text.contains("curated: false\n") && text.contains("lms: ''\n"), "{text}");
    // …and a name with an apostrophe does not break the file, like every other wizard value.
    let odd = CampusChoice { unitid: "1".into(), name: "St. Mary's College".into(), state: "MD".into(), lms: String::new() };
    assert!(campus_config_yaml(&odd).expect("campus.yaml").contains("name: 'St. Mary''s College'\n"));

    assert_eq!(curated("100751").map(|c| c.key), Some("university-of-alabama"));
    assert_eq!(curated("157085").map(|c| c.key), Some("university-of-kentucky"));
    assert!(curated("999999").is_none());

    // The timezone the wizard suggests, from the state — the OS zone stays the default and the
    // student can always type over it.
    assert_eq!(state_timezone("AL"), Some("America/Chicago"));
    assert_eq!(state_timezone("KY"), Some("America/New_York"));
    assert_eq!(state_timezone("AZ"), Some("America/Phoenix"));
    assert_eq!(state_timezone("HI"), Some("Pacific/Honolulu"));
    assert_eq!(state_timezone("zz"), None);
    // Fifty states, DC and the five inhabited territories — IPEDS keeps Puerto Rico's hundred-odd
    // institutions, and a student in Mayagüez is not a special case any more than one in Wyoming.
    assert_eq!(knowlu::scaffold::STATE_TZ.len(), 56);
    assert_eq!(state_timezone("PR"), Some("America/Puerto_Rico"));
    assert_eq!(state_timezone("GU"), Some("Pacific/Guam"));
    for (st, tz) in knowlu::scaffold::STATE_TZ {
        assert!(jiff::tz::TimeZone::get(tz).is_ok(), "{st} maps to {tz}, which the tz database does not have");
    }
}
```

- [ ] **Step 3: Run and watch them fail.**

- [ ] **Step 4: `app/src/scaffold.rs` — the curated layer, the state table, the file.**

```rust
/// What a **curated** school gets on top of being in the list: event feeds, a known LMS, and a
/// sign-in URL the window can be pointed at. Keyed by IPEDS `UNITID`, which is the one identifier
/// that is stable across years and unambiguous across the four "University of ——" in a state.
///
/// **Adding a campus is adding a row here** — plus, if it is to have event feeds, one preset file
/// under `app/assets/campus/`. A school that is not in this table is still perfectly usable: it has
/// no event feeds, and its LMS comes from the sign-in window or from the student.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Curated {
    pub unitid: &'static str,
    /// The preset key `campus_yaml` maps to an `app/assets/campus/*.yaml` file.
    pub key: &'static str,
    pub label: &'static str,
    /// **This school's own LMS host**, no scheme and no path — `ualearn.blackboard.com`. Every URL the
    /// sign-in window is driven to is built from it, so a third curated Blackboard school does not send
    /// its student to Alabama's LMS. The paths are the same across every tenant of a kind; only the
    /// host differs, which is exactly what this field is.
    pub lms_host: &'static str,
    /// `blackboard` or `canvas`. With `lms_host`, it is enough to build every endpoint either LMS has.
    pub lms_kind: &'static str,
}

pub const CAMPUSES: [Curated; 2] = [
    Curated {
        unitid: "100751",
        key: "university-of-alabama",
        label: "The University of Alabama",
        lms_host: "ualearn.blackboard.com",
        lms_kind: "blackboard",
    },
    Curated {
        unitid: "157085",
        key: "university-of-kentucky",
        label: "University of Kentucky",
        lms_host: "uk.instructure.com",
        lms_kind: "canvas",
    },
];

/// The curated row for a unitid, if there is one.
pub fn curated(unitid: &str) -> Option<&'static Curated> {
    CAMPUSES.iter().find(|c| c.unitid == unitid)
}

/// The preset key whose `app/assets/campus/*.yaml` becomes this vault's `config/events.yaml`.
/// **`none` for anything uncurated, and for `university-of-kentucky` until someone writes its preset**
/// — `app/assets/campus/` is not this stream's to add a file to, and a preset key with no file behind
/// it is a vault that will not scaffold. Adding UK's feeds is one asset file and one line here.
pub fn events_preset_for(unitid: &str) -> &'static str {
    match curated(unitid).map(|c| c.key) {
        Some("university-of-alabama") => "university-of-alabama",
        _ => "none",
    }
}

/// The school the student picked. `lms` is empty for a school whose kind nothing has established yet;
/// Task 14's sign-in window fills it from where it lands, or the panel asks.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct CampusChoice {
    pub unitid: String,
    pub name: String,
    pub state: String,
    pub lms: String,
}

/// `config/campus.yaml`. Five single-line scalars, through the same `yaml_scalar` every other wizard
/// value goes through — a school name is free text and `St. Mary's College` is a real one.
pub fn campus_config_yaml(c: &CampusChoice) -> Result<String, String> {
    Ok(format!(
        "unitid: {}\nname: {}\nstate: {}\nlms: {}\ncurated: {}\n",
        yaml_scalar("school id", &c.unitid)?,
        yaml_scalar("school name", &c.name)?,
        yaml_scalar("school state", &c.state)?,
        yaml_scalar("school LMS", &c.lms)?,
        curated(&c.unitid).is_some(),
    ))
}

/// A US state's IANA zone, for the wizard's timezone **suggestion** — the OS zone stays the default
/// and the field stays editable, because a student in El Paso is in Texas and on Mountain time.
///
/// **Split states take their majority zone**, which is the honest simplification: Florida (Eastern,
/// bar the western panhandle), Idaho (Mountain, bar the north), Indiana (Eastern, bar the corners),
/// Kansas, Kentucky (Eastern, bar the west), Michigan (Eastern, bar four counties), Nebraska, North
/// Dakota, Oregon (Pacific, bar Malheur), South Dakota, Tennessee (Central, bar the east) and Texas.
/// The panel says the suggestion came from the state, so a student who is in the minority half can see
/// why it is wrong and change it.
pub const STATE_TZ: [(&str, &str); 56] = [
    ("AL", "America/Chicago"), ("AK", "America/Anchorage"), ("AZ", "America/Phoenix"),
    ("AR", "America/Chicago"), ("CA", "America/Los_Angeles"), ("CO", "America/Denver"),
    ("CT", "America/New_York"), ("DC", "America/New_York"), ("DE", "America/New_York"),
    ("FL", "America/New_York"), ("GA", "America/New_York"), ("HI", "Pacific/Honolulu"),
    ("IA", "America/Chicago"), ("ID", "America/Boise"), ("IL", "America/Chicago"),
    ("IN", "America/Indiana/Indianapolis"), ("KS", "America/Chicago"), ("KY", "America/New_York"),
    ("LA", "America/Chicago"), ("MA", "America/New_York"), ("MD", "America/New_York"),
    ("ME", "America/New_York"), ("MI", "America/Detroit"), ("MN", "America/Chicago"),
    ("MO", "America/Chicago"), ("MS", "America/Chicago"), ("MT", "America/Denver"),
    ("NC", "America/New_York"), ("ND", "America/Chicago"), ("NE", "America/Chicago"),
    ("NH", "America/New_York"), ("NJ", "America/New_York"), ("NM", "America/Denver"),
    ("NV", "America/Los_Angeles"), ("NY", "America/New_York"), ("OH", "America/New_York"),
    ("OK", "America/Chicago"), ("OR", "America/Los_Angeles"), ("PA", "America/New_York"),
    ("RI", "America/New_York"), ("SC", "America/New_York"), ("SD", "America/Chicago"),
    ("TN", "America/Chicago"), ("TX", "America/Chicago"), ("UT", "America/Denver"),
    ("VA", "America/New_York"), ("VT", "America/New_York"), ("WA", "America/Los_Angeles"),
    ("WI", "America/Chicago"), ("WV", "America/New_York"), ("WY", "America/Denver"),
    // The territories, because `CYACTIVE = 1` and `ICLEVEL ∈ {1,2}` keep them: Puerto Rico alone has
    // about a hundred institutions, the UPR system among them, and a student there getting no
    // suggestion at all would be a degradation nobody chose.
    ("PR", "America/Puerto_Rico"), ("VI", "America/Puerto_Rico"), ("GU", "Pacific/Guam"),
    ("MP", "Pacific/Guam"), ("AS", "Pacific/Pago_Pago"),
];

pub fn state_timezone(state: &str) -> Option<&'static str> {
    let up = state.trim().to_ascii_uppercase();
    STATE_TZ.iter().find(|(s, _)| *s == up).map(|(_, tz)| *tz)
}
```

`VaultPlan.campus: String` **stays** — it is the events preset key `campus_yaml` maps to an asset —
and is filled in Rust by `events_preset_for`, not by the page. One field is added beside it:

```rust
    /// R-OB-4: the school itself, out of the bundled list. `campus` above is only which preset of
    /// event feeds it gets.
    pub campus_choice: CampusChoice,
```

…and `build_into` gains one line, after `config/events.yaml`:

```rust
    write_file(root, "config/campus.yaml", &campus_config_yaml(&plan.campus_choice)?)?;
```

- [ ] **Step 4a: The search itself, in Rust — the page never holds the list.**

  **Why a command and not a bundled asset the page reads.** Two reasons, and the first is decisive:
  `app/tauri.conf.json`'s CSP is `default-src 'self'; connect-src ipc: http://ipc.localhost; …`, and
  `connect-src` is what governs `fetch` — it names no `'self'`, so a `fetch("campuses.json")` is
  refused before it reaches the asset protocol. Widening it would mean editing `tauri.conf.json`,
  which is the controller's outside C0's three keys, for a thing that needs no edit. The second reason
  outlives the first: a typeahead wants ten rows, not six thousand, and a command that answers with ten
  keeps the page's memory flat and puts the ranking rule somewhere a test can reach it.

  The file is **`app/campuses.json`**, beside `tauri.conf.json` and **not** under `app/static/`: the
  page never loads it, and a 400 KB asset in `frontendDist` would ship twice — once in the bundle and
  once in the binary that `include_str!`s it. It is written there by hand-off **H11** and read here.

```rust
/// Every US institution, parsed once. `include_str!` puts the bytes in the binary — they are needed
/// on a first run with no network, which is most first runs — and `OnceLock` parses them the first
/// time somebody types, not at launch.
static CAMPUS_LIST: std::sync::OnceLock<Vec<(u64, String, String, String)>> = std::sync::OnceLock::new();

fn campus_list() -> &'static [(u64, String, String, String)] {
    CAMPUS_LIST.get_or_init(|| {
        let raw = include_str!("../campuses.json");
        let v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
        v.get("campuses")
            .and_then(|c| c.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|r| {
                        let a = r.as_array()?;
                        Some((
                            a.first()?.as_u64()?,
                            a.get(1)?.as_str()?.to_string(),
                            a.get(2)?.as_str()?.to_string(),
                            a.get(3)?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The ten best matches for what has been typed, in list order. Name, city and state all match, so
/// `tuscaloosa` and `AL` both find it — a student who cannot spell their own university's official
/// name (it is *The* University of Alabama) still gets there. Under two characters is no answer at
/// all: one letter matches a thousand schools and none of them usefully.
///
/// Returns `[unitid, name, city, state]` per hit — the host is in the asset and is not shown, so it
/// does not cross the IPC either.
#[tauri::command]
pub fn campus_search(query: String) -> Value {
    let needle = query.trim().to_lowercase();
    if needle.len() < 2 {
        return json!({ "ok": true, "error": Value::Null, "hits": [] });
    }
    let hits: Vec<Value> = campus_list()
        .iter()
        .filter(|(_, name, city, state)| {
            format!("{name} {city} {state}").to_lowercase().contains(&needle)
        })
        .take(10)
        .map(|(id, name, city, state)| json!([id, name, city, state]))
        .collect();
    json!({ "ok": true, "error": Value::Null, "hits": hits })
}
```

  …and a test beside the others in `app/tests/onboarding.rs`:

```rust
/// R-OB-4: the search is Rust's, because the page cannot fetch the list (the app's CSP names no
/// `'self'` in `connect-src`) and should not hold six thousand rows to answer a keystroke.
#[test]
fn typing_a_school_name_finds_it_and_typing_one_letter_finds_nothing() {
    use knowlu::onboarding::campus_search;
    let one = campus_search("a".into());
    assert_eq!(one["hits"].as_array().map(Vec::len), Some(0), "one letter is not a search");
    let hits = campus_search("university of alabama".into());
    let rows = hits["hits"].as_array().expect("hits");
    assert!(!rows.is_empty() && rows.len() <= 10, "{} hits", rows.len());
    assert!(rows.iter().any(|r| r[0].as_u64() == Some(100751)), "{rows:?}");
    // A row is [unitid, name, city, state] — the web host stays in the asset, unshown and uncrossed.
    assert_eq!(rows[0].as_array().map(Vec::len), Some(4));
    // City and state match too, or a student who knows where they go and not what it is called is stuck.
    assert!(campus_search("tuscaloosa".into())["hits"].as_array().map(|r| !r.is_empty()).unwrap_or(false));
}
```

- [ ] **Step 4b: One more command — the timezone the state suggests.** The table is Rust's, and the
  page asks for one string. It never overwrites a field the student has typed into (`WIZ.tzTouched`),
  and `None` leaves the OS zone alone, which is what `launch_state` already supplies.

```rust
/// R-OB-4: the timezone a state suggests. A **suggestion** — the page only applies it to a field the
/// student has not touched, and a state we do not know leaves the OS zone where it was.
#[tauri::command]
pub fn timezone_for_state(state: String) -> Value {
    json!({ "ok": true, "error": Value::Null, "timezone": crate::scaffold::state_timezone(&state) })
}
```

- [ ] **Step 5: `app/src/onboarding.rs` — the choice reaches the plan, and the preset is derived.**
  `WizardPlan` gains `#[serde(default)] pub campus_choice: crate::scaffold::CampusChoice;` and
  `create_vault_in` sets both fields from it — the page sends the school, Rust decides which preset
  that means:

```rust
        campus: crate::scaffold::events_preset_for(&plan.campus_choice.unitid).to_string(),
        campus_choice: plan.campus_choice.clone(),
```

  `launch_state`'s `campuses` key is **removed**: the radio list it fed does not exist any more, and the
  page asks `campus_search` for ten rows at a time instead — it never holds the list and never fetches
  the asset (step 4a says why). Delete the key and the `CAMPUSES.iter().map(…)` expression with it, and
  `WIZ.campuses` with them (Task 17's `WIZ` no longer declares it).

- [ ] **Step 6: `app/src/lms_link.rs` — key on the unitid, and ask when nobody knows.**
  `capture_steps` and `course_list_url` take a **unitid** instead of a campus key, and both consult
  `scaffold::curated`:

```rust
/// The campus's capture path, recorded verbatim by Task 13's spike. `None` for a school nobody has
/// curated — which is most of the 4,319 — and that is what makes the panel show its paste field
/// instead of pretending.
/// Where the sign-in window opens for a curated school: **that school's own host**, never a constant.
pub fn lms_home(unitid: &str) -> Option<String> {
    crate::scaffold::curated(unitid).map(|c| format!("https://{}/", c.lms_host))
}

/// The calendar-share path Task 13's spike recorded, on **this** school's host. A constant host per
/// LMS kind would send the third curated Blackboard school's student to the first one's LMS — which is
/// the assumption this whole task exists to stop making (`CLAUDE.md`'s first rule).
fn capture_steps(unitid: &str) -> Option<Vec<String>> {
    let c = crate::scaffold::curated(unitid)?;
    // <Task 13 Outcome: the PATHS, verbatim — the host comes from the row>
    let paths: &[&str] = match c.lms_kind {
        "blackboard" => &["/ultra/calendar"],
        "canvas" => &["/calendar"],
        _ => return None,
    };
    Some(paths.iter().map(|path| format!("https://{}{path}", c.lms_host)).collect())
}

/// Likewise for the enrolled-course endpoint: one path per LMS kind, this school's host.
fn course_list_url(unitid: &str) -> Option<String> {
    let c = crate::scaffold::curated(unitid)?;
    // <Task 13 step 4a: the PATHS that answered, verbatim>
    let path = match c.lms_kind {
        "blackboard" => "/learn/api/public/v1/users/me/courses",
        "canvas" => "/api/v1/courses",
        _ => return None,
    };
    Some(format!("https://{}{path}", c.lms_host))
}

/// Which LMS a school runs, when nobody curated it. **Guessed from where the sign-in window landed**,
/// never from the school's name: `blackboard.com` and `instructure.com` are in the URL of every one of
/// their tenants, and a guess from a hostname is a fact. `None` means the panel's two-button question.
pub fn lms_kind_from_url(url: &str) -> Option<&'static str> {
    let u = url.to_ascii_lowercase();
    if u.contains("blackboard.com") || u.contains("/ultra/") { return Some("blackboard"); }
    if u.contains("instructure.com") || u.contains("/api/v1/courses") { return Some("canvas"); }
    None
}
```

  …and `open_lms_window` takes the unitid, opening `lms_home(unitid)` when there is one and answering
  `{"ok": false, "error": "we do not know your school's sign-in page yet — paste your calendar link
  below", "opened": false}` when there is not. `capture_steps` now yields `Vec<String>`, so the two
  loops that walk it become `for step in steps.iter().map(String::as_str)`, and `capture_courses` takes
  `let Some(url) = course_list_url(&unitid) else { … }` and passes `&url`.

  One test goes with it, in `app/tests/lms_link.rs` — the assumption this task removes is worth an
  assertion, not just a comment:

```rust
/// R-OB-4: every endpoint is built from the school's **own** host. A constant host per LMS kind was
/// accidentally correct with two curated schools and would send the third one's student to the first
/// one's LMS.
#[test]
fn every_curated_endpoint_is_built_from_that_schools_own_host() {
    use knowlu::lms_link::lms_home;
    for c in knowlu::scaffold::CAMPUSES {
        let home = lms_home(c.unitid).unwrap_or_else(|| panic!("{} has no home", c.label));
        assert!(home.contains(c.lms_host), "{} opens {home}, which is not its own host", c.label);
        // …and no other curated school's host appears in it.
        for other in knowlu::scaffold::CAMPUSES {
            if other.unitid != c.unitid {
                assert!(!home.contains(other.lms_host), "{} opens {}'s LMS", c.label, other.label);
            }
        }
    }
    assert!(lms_home("999999").is_none(), "an uncurated school has no sign-in page we know");
}
```

- [ ] **Step 7: Run the tests.** `cargo test -p knowlu --test onboarding --test scaffold --test lms_link` → the five new tests pass — three in `onboarding.rs` (the bundled list's shape, `campus_search`'s ranking, `timezone_for_state`), one in `scaffold.rs` (`campus_config_yaml`) and one in `lms_link.rs` (`lms_home` by unitid). Every existing `VaultPlan` literal gains `campus_choice: Default::default()` (or the `plan_for` helper does it once).

- [ ] **Step 8: Commit.** `app: the school comes from every US institution, not two radios — the IPEDS list bundled, the curated layer keyed by unitid, config/campus.yaml, and a timezone from the state (C1 Task 14c, R-OB-4)`.

---

### Task 15: `app/src/telemetry.rs` — (a) from the ledger, (b) from the journal, and a watermark

Spec §6 and D5. Both classes are computed **on the device**, and neither carries a title, a course name or a note body. The slot posts them; a slot that cannot post is not a failed slot.

**Files:**
- Modify: `app/src/telemetry.rs` (the H1 stub), `app/src/scheduler.rs`
- Test: `app/tests/telemetry.rs`, `app/tests/scheduler.rs`

**Interfaces:**
- Consumes: `account::{cloud_config, auth_base, valid_access_token_at, check_api_base}` (Tasks 10–11).
- Produces: `VALUED_FIELDS`, `FLAGGED_FIELDS`, `struct EventRow`, `struct CorrectionRow`, `struct Batch`, `read_events(vault, since)`, `read_corrections(vault, since)`, `watermark_path(data_dir)`, `load_watermark`, `save_watermark`, `watermark(last_event, last_correction, capped)`, `post_batch_at(api_base, token, &Batch)`, `send(vault, data_dir)`. Task 17 shows nothing of this; the scheduler calls `send`.

- [ ] **Step 1: Write the failing tests** — `app/tests/telemetry.rs`:

```rust
//! (a) and (b), derived on the device. The rule under every assertion here: **nothing that leaves is
//! content**. An id is not content; a number is not content; a title, a course name and a note body
//! are.
use knowlu::telemetry::{read_corrections, read_events, Batch, VALUED_FIELDS, FLAGGED_FIELDS};
use std::path::{Path, PathBuf};

fn vault(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-tele-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("state").join("events-ui")).unwrap();
    std::fs::create_dir_all(d.join("state").join("journal")).unwrap();
    d
}

fn write(path: &Path, lines: &[&str]) {
    std::fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

#[test]
fn interaction_events_come_off_the_ledger_and_carry_no_free_text() {
    let v = vault("events");
    write(&v.join("state/events-ui/2026-09-09.jsonl"), &[
        r#"{"action": "view_opened", "device": "M", "ms": null, "object_id": null, "object_kind": null, "session": "sess_1", "ts": "2026-09-09T12:00:00.000Z", "view": "today"}"#,
        r#"{"action": "object_seen", "device": "M", "ms": 2400, "object_id": "task_0123456789", "object_kind": "task", "session": "sess_1", "ts": "2026-09-09T12:00:02.000Z", "view": "today"}"#,
    ]);
    write(&v.join("state/events-ui/2026-09-10.jsonl"), &[
        r#"{"action": "decision_made", "device": "M", "ms": null, "object_id": "appr_0000000001", "object_kind": "approval", "session": "sess_2", "ts": "2026-09-10T09:00:00.000Z", "view": "decisions"}"#,
    ]);
    let all = read_events(&v, None);
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].action, "view_opened");
    assert_eq!(all[1].object_id.as_deref(), Some("task_0123456789"));
    assert_eq!(all[2].view, "decisions");
    // The watermark is exclusive: the same batch is never sent twice.
    let after = read_events(&v, Some("2026-09-09T12:00:02.000Z"));
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].action, "decision_made");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn a_correction_is_a_human_overriding_a_field_the_agent_set() {
    let v = vault("corrections");
    write(&v.join("state/journal/2026-09-10.jsonl"), &[
        // The agent judged it…
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "effort_hours", "id": "task_0123456789", "new": 2.0, "old": null, "op": "set", "path": "tasks/a.md", "run_id": null, "ts": "2026-09-10T08:00:00.000Z", "via": "local-runner"}"#,
        // …and the human disagreed, from the console. That is a correction.
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "effort_hours", "id": "task_0123456789", "new": 0.5, "old": 2.0, "op": "set", "path": "tasks/a.md", "run_id": null, "ts": "2026-09-10T09:00:00.000Z", "via": "dashboard"}"#,
        // A human editing a field NO agent ever set is not a correction — it is just work.
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000002", "new": 3, "old": null, "op": "set", "path": "tasks/b.md", "run_id": null, "ts": "2026-09-10T09:05:00.000Z", "via": "dashboard"}"#,
        // A course correction: the row survives, the two course NAMES do not.
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "course", "id": "task_0000000003", "new": "MATH 125", "old": null, "op": "set", "path": "tasks/c.md", "run_id": null, "ts": "2026-09-10T08:10:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "course", "id": "task_0000000003", "new": "SPAN 101", "old": "MATH 125", "op": "set", "path": "tasks/c.md", "run_id": null, "ts": "2026-09-10T09:10:00.000Z", "via": "dashboard"}"#,
        // A title is neither valued nor flagged: no row at all.
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "title", "id": "task_0000000004", "new": "Read chapter 3", "old": null, "op": "set", "path": "tasks/d.md", "run_id": null, "ts": "2026-09-10T08:20:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "title", "id": "task_0000000004", "new": "Read ch. 3 of Calculus", "old": "Read chapter 3", "op": "set", "path": "tasks/d.md", "run_id": null, "ts": "2026-09-10T09:20:00.000Z", "via": "dashboard"}"#,
    ]);
    let cs = read_corrections(&v, None);
    assert_eq!(cs.len(), 2, "{cs:?}");
    assert_eq!(cs[0].item_id, "task_0123456789");
    assert_eq!(cs[0].field, "effort_hours");
    assert_eq!(cs[0].ours.as_deref(), Some("2.0"));
    assert_eq!(cs[0].theirs.as_deref(), Some("0.5"));
    assert_eq!(cs[0].kind, "task");
    assert_eq!(cs[1].field, "course");
    assert_eq!(cs[1].ours, None, "a course name is content and never leaves");
    assert_eq!(cs[1].theirs, None);
    // The whole file, read as text, must not contain either course name or the title.
    let batch = Batch { events: vec![], corrections: cs };
    let wire = serde_json::to_string(&batch).unwrap();
    for content in ["MATH 125", "SPAN 101", "Read chapter 3", "Read ch. 3 of Calculus"] {
        assert!(!wire.contains(content), "{content} reached the wire: {wire}");
    }
    assert!(VALUED_FIELDS.contains(&"effort_hours") && !VALUED_FIELDS.contains(&"course"));
    assert!(FLAGGED_FIELDS.contains(&"course"));
    let _ = std::fs::remove_dir_all(&v);
}

/// The watermark may never step past a stream that was truncated — the bug that would silently drop
/// events 501-600 forever and leave the comment claiming the opposite.
#[test]
fn a_capped_batch_advances_the_watermark_only_as_far_as_the_slower_stream() {
    use knowlu::telemetry::{read_corrections, read_events, watermark};
    let v = vault("watermark");
    // 600 events across two days, and one correction dated after all of them.
    let day = |n: usize, base: &str| -> Vec<String> {
        (0..n).map(|i| format!(
            r#"{{"action": "view_opened", "device": "M", "ms": null, "object_id": null, "object_kind": null, "session": "sess_1", "ts": "{base}T{:02}:{:02}:{:02}.000Z", "view": "today"}}"#,
            i / 3600, (i / 60) % 60, i % 60)).collect()
    };
    let d1 = day(600, "2026-09-09");
    write(&v.join("state/events-ui/2026-09-09.jsonl"), &d1.iter().map(String::as_str).collect::<Vec<_>>());
    write(&v.join("state/journal/2026-09-10.jsonl"), &[
        r#"{"actor": "agent:knowlu.enrich", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000009", "new": 2, "old": null, "op": "set", "path": "tasks/z.md", "run_id": null, "ts": "2026-09-10T08:00:00.000Z", "via": "local-runner"}"#,
        r#"{"actor": "quinn", "device": "M", "evidence": null, "field": "importance", "id": "task_0000000009", "new": 3, "old": 2, "op": "set", "path": "tasks/z.md", "run_id": null, "ts": "2026-09-10T09:00:00.000Z", "via": "dashboard"}"#,
    ]);
    let events = read_events(&v, None);
    let corrections = read_corrections(&v, None);
    assert_eq!(events.len(), 600);
    assert_eq!(corrections.len(), 1);
    // The fixture's own property first: the 500th event is EARLIER than the correction, so a
    // watermark taken as the maximum would step past events 501-600.
    assert!(events[499].ts < corrections[0].ts, "the fixture must have a later correction than the cap");
    // …and then the arithmetic itself, both branches. `send` needs a cloud config it will not get
    // here, so the rule lives in `watermark`, where a test can reach it.
    let e = events[499].ts.clone();
    let c = corrections[0].ts.clone();
    assert_eq!(watermark(Some(e.as_str()), Some(c.as_str()), true), e, "a capped batch stops at the earlier tail");
    assert_eq!(watermark(Some(e.as_str()), Some(c.as_str()), false), c, "an uncapped batch goes as far as either stream got");
    // One empty stream is still answerable, and an empty batch has no watermark at all.
    assert_eq!(watermark(None, Some(c.as_str()), true), c);
    assert_eq!(watermark(Some(e.as_str()), None, false), e);
    assert_eq!(watermark(None, None, false), String::new());
    let _ = std::fs::remove_dir_all(&v);
}

/// The engine owns the action vocabulary; this file must not drift from it. A cheap cross-language
/// pin: the TypeScript that receives these rows lists the same eleven, so a new action added to the
/// engine fails here until it lands in both.
#[test]
fn the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire() {
    let ts = std::fs::read_to_string("../cloud/supabase/functions/telemetry/handler.ts").expect("the telemetry handler");
    for action in knowlu_engine::uievents::ACTIONS {
        assert!(ts.contains(&format!("\"{action}\"")), "the cloud does not accept {action}");
    }
    // …and the cloud accepts nothing the engine does not emit. Split on `= [` first: the declaration
    // is `export const ACTIONS: readonly string[] = [`, so splitting on `]` alone stops at `string[]`
    // and counts nothing at all.
    let listed = ts
        .split("export const ACTIONS")
        .nth(1)
        .and_then(|s| s.split("= [").nth(1))
        .and_then(|s| s.split(']').next())
        .expect("the ACTIONS array");
    assert_eq!(listed.matches('"').count() / 2, knowlu_engine::uievents::ACTIONS.len(), "{listed}");
}
```

- [ ] **Step 2: Run and watch it fail.** `cargo test -p knowlu --test telemetry` → unresolved imports.

- [ ] **Step 3: Fill `app/src/telemetry.rs`.**

```rust
//! Telemetry (a) and (b), derived on the device (Knowlu C1, Task 15). Spec §6, decided under D5.
//!
//! **What may leave, and nothing else.** (a) is `state/events-ui/`'s ledger, which `uievents::record`
//! already refuses free text into. (b) is derived here from the journal: a human, at the console,
//! setting a field an agent had set. Its values travel only for fields whose vocabulary is closed or
//! numeric; a **course** correction keeps its row and loses its two names, and a **title** correction
//! is not a row at all.
use std::path::{Path, PathBuf};

/// Judged fields whose correction may carry its values: numbers and closed vocabularies only.
pub const VALUED_FIELDS: [&str; 5] = ["effort_hours", "importance", "domain", "effort_confidence", "status"];
/// Judged fields recorded as "it changed" and nothing more. A course name is somebody's timetable.
pub const FLAGGED_FIELDS: [&str; 1] = ["course"];

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EventRow {
    pub ts: String,
    pub session: String,
    pub view: String,
    pub action: String,
    pub object_id: Option<String>,
    pub object_kind: Option<String>,
    pub ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CorrectionRow {
    pub ts: String,
    pub item_id: String,
    pub field: String,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Batch {
    pub events: Vec<EventRow>,
    pub corrections: Vec<CorrectionRow>,
}

/// Every `.jsonl` day file under `dir`, in filename order, each line parsed. A line that does not
/// parse is skipped rather than fatal: a half-written last line is what a power cut leaves, and one
/// lost event is not worth losing a batch over.
fn read_ledger(dir: &Path) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "jsonl").unwrap_or(false))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for line in text.lines() {
            if let Ok(serde_json::Value::Object(m)) = serde_json::from_str(line) {
                out.push(m);
            }
        }
    }
    out
}

fn s(m: &serde_json::Map<String, serde_json::Value>, k: &str) -> Option<String> {
    m.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// A scalar as the one short string the wire carries. `2.0` stays `2.0`, not `2`: the journal's own
/// spelling is what an eval example must compare against later.
fn scalar(v: Option<&serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    }
}

/// `tasks/a.md` → `task`. A token, never a path: the folder is the kind and the filename is a title.
fn kind_of(path: &str) -> String {
    let top = path.split('/').next().unwrap_or("");
    top.strip_suffix('s').unwrap_or(top).to_string()
}

/// (a). `since` is **exclusive** — the watermark is the last `ts` that was accepted, so re-running a
/// slot after a network failure sends the same rows again and the server's unique constraint absorbs
/// them; re-running after a success sends nothing.
pub fn read_events(vault: &Path, since: Option<&str>) -> Vec<EventRow> {
    read_ledger(&vault.join("state").join("events-ui"))
        .into_iter()
        .filter_map(|m| {
            let ts = s(&m, "ts")?;
            if let Some(w) = since {
                if ts.as_str() <= w { return None; }
            }
            Some(EventRow {
                ts,
                session: s(&m, "session")?,
                view: s(&m, "view")?,
                action: s(&m, "action")?,
                object_id: s(&m, "object_id"),
                object_kind: s(&m, "object_kind"),
                ms: m.get("ms").and_then(|v| v.as_i64()),
            })
        })
        .collect()
}

/// (b). Spec §6: "every human override of a judged field — journal records with `via: dashboard` on a
/// field the agent set". Walks the journal in order, remembering who last set each `(id, field)`; a
/// dashboard write over an `agent:` write is a correction, and nothing else is.
pub fn read_corrections(vault: &Path, since: Option<&str>) -> Vec<CorrectionRow> {
    let mut last: std::collections::BTreeMap<(String, String), (String, Option<String>)> = std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for m in read_ledger(&vault.join("state").join("journal")) {
        let (Some(ts), Some(id), Some(field), Some(actor)) = (s(&m, "ts"), s(&m, "id"), s(&m, "field"), s(&m, "actor")) else { continue };
        let key = (id.clone(), field.clone());
        let value = scalar(m.get("new"));
        let via = s(&m, "via").unwrap_or_default();
        let human = via == "dashboard" && !actor.starts_with("agent:");
        if human {
            if let Some((prior_actor, prior_value)) = last.get(&key) {
                if prior_actor.starts_with("agent:") {
                    let valued = VALUED_FIELDS.contains(&field.as_str());
                    let flagged = FLAGGED_FIELDS.contains(&field.as_str());
                    let fresh = since.map(|w| ts.as_str() > w).unwrap_or(true);
                    if (valued || flagged) && fresh {
                        out.push(CorrectionRow {
                            ts: ts.clone(),
                            item_id: id.clone(),
                            field: field.clone(),
                            // The whole of the content rule, in two lines.
                            ours: if valued { prior_value.clone() } else { None },
                            theirs: if valued { value.clone() } else { None },
                            kind: kind_of(&s(&m, "path").unwrap_or_default()),
                        });
                    }
                }
            }
        }
        last.insert(key, (actor, value));
    }
    out
}

/// The last `ts` this profile has sent. A file beside `seen.txt`, for the reason the entitlement
/// cache is one: `state::Settings` cannot grow a field without breaking every existing settings file.
pub fn watermark_path(data_dir: &Path) -> PathBuf { data_dir.join("telemetry-sent.txt") }

pub fn load_watermark(data_dir: &Path) -> Option<String> {
    std::fs::read_to_string(watermark_path(data_dir)).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn save_watermark(data_dir: &Path, ts: &str) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    std::fs::write(watermark_path(data_dir), ts).map_err(|e| e.to_string())
}

pub fn post_batch_at(api_base: &str, token: &str, batch: &Batch) -> Result<(usize, usize), String> {
    crate::account::check_api_base(api_base)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut res = agent
        .post(&format!("{}/telemetry", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .send_json(batch)
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) { return Err(format!("telemetry: {status} {text}")); }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    Ok((
        v.get("events").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
        v.get("corrections").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
    ))
}

/// **How far the watermark may advance.** A stream that hit the cap contributes only its own last
/// `ts` and the answer is the **earliest** of the contributions: taking the maximum across both would
/// step past a truncated stream's tail, so a vault with 600 pending events and one late correction
/// would advance past events 501-600 and never send them. With neither stream capped there is nothing
/// to step past and the answer is the latest of the two. An empty batch has no watermark.
///
/// Its own function because it is the one piece of arithmetic here that is wrong silently — the
/// symptom is events that were never sent and nothing anywhere saying so.
pub fn watermark(last_event: Option<&str>, last_correction: Option<&str>, capped: bool) -> String {
    let ends = [last_event, last_correction].into_iter().flatten();
    if capped { ends.min() } else { ends.max() }.unwrap_or_default().to_string()
}

/// One send: read since the watermark, post, advance the watermark only on success. **At most 500 of
/// each** — the server's cap — so a vault with a long history catches up over several slots instead
/// of being refused forever.
pub fn send(vault: &Path, data_dir: &Path) -> Result<(usize, usize), String> {
    let cfg = crate::account::cloud_config(vault)?;
    let since = load_watermark(data_dir);
    let mut events = read_events(vault, since.as_deref());
    let mut corrections = read_corrections(vault, since.as_deref());
    // **The watermark can only advance as far as the SLOWER stream got** — `watermark` above is that
    // rule, and the reason it is a rule. This is what makes "catches up over several slots" true.
    let capped = events.len() > 500 || corrections.len() > 500;
    events.truncate(500);
    corrections.truncate(500);
    if events.is_empty() && corrections.is_empty() { return Ok((0, 0)); }
    let last_event = events.last().map(|e| e.ts.clone());
    let last_correction = corrections.last().map(|c| c.ts.clone());
    let high = watermark(last_event.as_deref(), last_correction.as_deref(), capped);
    if high.is_empty() { return Ok((0, 0)); }
    let auth = crate::account::auth_base(&cfg.api_base)?;
    let token = crate::account::valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, jiff::Timestamp::now().as_second())?;
    let sent = post_batch_at(&cfg.api_base, &token, &Batch { events, corrections })?;
    save_watermark(data_dir, &high)?;
    Ok(sent)
}
```

- [ ] **Step 4: The slot's step.** In `scheduler.rs`'s `run_slot_inner`, after the `push`/`backup` steps and before `state::refresh_head`:

```rust
    // Spec §6: batched to `/telemetry` at each slot. **Never a failure** — a student on a train has
    // nothing to apologise for, and an analytics upload has no business turning a slot amber. Every
    // outcome is a named step with exit code 0, the same shape the ingest and judge skips use.
    let telemetry = match entitlement_state(cs) {
        crate::account::EntitlementState::NoAccount => ("telemetry (skipped: no account)".to_string(), 0),
        _ => match crate::telemetry::send(&cs.vault, &cs.data_dir) {
            Ok((0, 0)) => ("telemetry (nothing new)".to_string(), 0),
            Ok((e, c)) => (format!("telemetry ({e} events, {c} corrections)"), 0),
            Err(_) => ("telemetry (skipped: offline)".to_string(), 0),
        },
    };
    steps.push(telemetry);
```

Note the deliberate asymmetry with `judge`: telemetry is sent **whenever there is an account**, entitled or not, because it is collected under the terms rather than bought with a subscription (spec §6) — and the server refuses it if the session is not valid, which is where that decision belongs.

- [ ] **Step 5: A scheduler test** — append to `app/tests/scheduler.rs`:

```rust
/// The telemetry step never fails a slot. This vault has an account, no reachable cloud (the api_base
/// points at a port nothing is listening on) and therefore an offline send — and the slot is green.
#[test]
fn an_offline_telemetry_send_is_a_named_step_and_never_a_failure() {
    let v = scratch("teleoffline");
    std::fs::write(
        v.join("config").join("runners.yaml"),
        format!("runners:\n  - name: local\n    times: [\"12:00\"]\n    tz: America/Chicago\n    grace_minutes: 20\n    device: {}\n    scheduler: app\n", knowlu_engine::journal::device_name()),
    ).unwrap();
    // Port 9 is `discard`: nothing on this machine answers it, so the send fails fast and locally.
    std::fs::write(
        v.join("config").join("cloud.yaml"),
        "api_base: 'http://127.0.0.1:9/functions/v1'\nanon_key: 'anon'\nsession_credential_target: 'knowlu/profile_x/session'\naccount_id: 'acc-1'\n",
    ).unwrap();
    let cs = open(&v, "teleoffline");
    knowlu::account::save_cache(&cs.data_dir, &knowlu::account::EntitlementCache {
        status: "active".into(), current_period_end: None, plan: None, checked_at: knowlu_engine::journal::now_ts(None),
    }).unwrap();
    let sch = Scheduler::default();
    let fake = std::env::temp_dir().join(format!("qo-sched-tele-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fake);
    std::fs::create_dir_all(&fake).unwrap();
    let _guard = ENGINE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvSeam::set(&[("KNOWLU_ENGINE_EXE", std::ffi::OsStr::new("cmd")), ("LOCALAPPDATA", fake.as_os_str())]);
    let s = run_slot_inner(&cs, &sch, None, false);
    let named: Vec<String> = s.steps.iter().map(|(n, _)| n.clone()).collect();
    assert!(named.iter().any(|n| n.starts_with("telemetry (")), "{named:?}");
    assert!(s.steps.iter().filter(|(n, _)| n.starts_with("telemetry")).all(|(_, c)| *c == 0));
    assert!(s.engine_ok, "telemetry must never paint the tray amber: {:?}", s.steps);
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&v);
}
```

- [ ] **Step 6: Run everything.** `cargo test -p knowlu` → `telemetry` 4 passed; `scheduler` green with one more test than before this task; 0 warnings.

- [ ] **Step 7: Commit.** `app: telemetry.rs — interaction events off the ledger, corrections out of the journal, no title and no course name on the wire (C1 Task 15)`.

---

### Task 16: `app/src/report.rs` — the preview the user reads before anything is sent

Legal note §9, and Google's Limited Use: **the preview screen is the documented consent** for a human to look at anything the report carries. So the text the user sees *is* the payload, redacted by default and editable, and what they press Send on is exactly what arrives.

**Files:**
- Modify: `app/src/report.rs` (the H1 stub)
- Test: `app/tests/report.rs`

**Interfaces:**
- Consumes: `account::{cloud_config, auth_base, valid_access_token_at, check_api_base}`; `state::ConsoleState`; `knowlu_engine::surface::runs_panel`.
- Produces: `scrub(&str) -> String`, `vault_shape(&Path) -> Vec<(String, usize)>`, `log_tail(&Path, usize) -> Vec<String>`, `preview_text(cs, view) -> String`, `send_at(api_base, token, text, profile_id) -> Result<String, String>`, and the two commands `report_preview`, `report_send`.

- [ ] **Step 1: Write the failing test** — `app/tests/report.rs`:

```rust
use knowlu::report::{log_tail, scrub, vault_shape};

#[test]
fn the_seven_things_a_report_must_never_carry() {
    // The same seven the cloud's `_shared/scrub.ts` refuses, in the same order, producing the same
    // text — the two ends are a pair and this test and its TypeScript twin are how they stay one.
    // (They agree on these seven; on bracketed URLs they differ, and `scrub`'s doc comment says so.)
    assert_eq!(scrub("mailed a.student@crimson.ua.edu twice"), "mailed <email> twice");
    assert_eq!(scrub("fetching https://lms.example.invalid/feed/abc123.ics failed"), "fetching <url> failed");
    assert_eq!(scrub("http://10.0.0.1/x"), "<url>");
    assert_eq!(
        scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
        "authorization: Bearer <token>"
    );
    assert_eq!(scrub("1 unreadable: read-chapter-3-of-calculus.md)"), "1 unreadable: <note>)");
    assert_eq!(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<token>");
    // The Windows account name is usually a person's name, and the log lines are full of paths.
    assert_eq!(
        scrub("could not open C:\\Users\\Ada\\Knowlu\\Fall 2026\\tasks\\a.md"),
        "could not open C:\\Users\\<user>\\Knowlu\\Fall 2026\\tasks\\<note>"
    );
    // …and ordinary prose survives, or the preview is unreadable and nobody sends anything.
    assert_eq!(scrub("the run at 12:00 exited 1 after 4 steps"), "the run at 12:00 exited 1 after 4 steps");
}

#[test]
fn the_two_ends_of_the_scrub_are_written_against_the_same_cases() {
    // There is no way to run TypeScript from here, so the pin is that the cloud's twin exists, names
    // the same four classes, and orders them the same way — URLs before tokens, notes before tokens.
    let ts = std::fs::read_to_string("../cloud/supabase/functions/_shared/scrub.ts").expect("scrub.ts");
    for marker in ["<email>", "<url>", "<note>", "<token>"] {
        assert!(ts.contains(marker), "the cloud scrub has no {marker}");
    }
    let order: Vec<usize> = ["<email>", "<url>", "<note>", "<token>"].iter().map(|m| ts.find(m).unwrap()).collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "the cloud scrub applies its rules in another order");
}

#[test]
fn the_vault_shape_is_counts_and_never_names() {
    let v = std::env::temp_dir().join(format!("knowlu-report-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&v);
    for d in ["tasks", "approvals", "courses", "info", "issues", "archive"] {
        std::fs::create_dir_all(v.join(d)).unwrap();
    }
    std::fs::write(v.join("tasks").join("read-chapter-3.md"), "---\ntitle: Read chapter 3\n---\n").unwrap();
    std::fs::write(v.join("tasks").join("write-the-essay.md"), "---\ntitle: Write the essay\n---\n").unwrap();
    std::fs::write(v.join("approvals").join("a.md"), "x").unwrap();
    let shape = vault_shape(&v);
    assert_eq!(shape.iter().find(|(k, _)| k == "tasks").unwrap().1, 2);
    assert_eq!(shape.iter().find(|(k, _)| k == "approvals").unwrap().1, 1);
    assert_eq!(shape.iter().find(|(k, _)| k == "courses").unwrap().1, 0);
    // Nothing in the shape is a filename, and a filename is a title.
    let rendered = format!("{shape:?}");
    assert!(!rendered.contains("read-chapter-3") && !rendered.contains("essay"), "{rendered}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_log_tail_is_the_last_lines_of_the_newest_logs_and_is_scrubbed() {
    let d = std::env::temp_dir().join(format!("knowlu-report-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("logs")).unwrap();
    std::fs::write(d.join("logs").join("slot-1-0-coursework.txt"), "old line\n").unwrap();
    let many: String = (0..300).map(|i| format!("line {i} https://lms.example.invalid/x{i}.ics\n")).collect();
    std::fs::write(d.join("logs").join("slot-2-0-rank.txt"), many).unwrap();
    let tail = log_tail(&d, 200);
    assert_eq!(tail.len(), 200);
    assert!(tail.iter().all(|l| !l.contains("https://")), "a capability URL survived into the tail");
    assert!(tail.last().unwrap().contains("line 299"));
    let _ = std::fs::remove_dir_all(&d);
}
```

- [ ] **Step 2: Run and watch it fail.** `cargo test -p knowlu --test report` → unresolved imports.

- [ ] **Step 3: Fill `app/src/report.rs`.**

```rust
//! Issue reports: the payload, the scrub, the preview (Knowlu C1, Task 16).
//!
//! **The text the user reads is the payload.** Not a summary of it, not a friendly rendering of a
//! structure sent separately — the exact bytes. That is what makes the preview screen the consent
//! the legal note (§9) and Google's Limited Use both want: a person agreed to a human reading *this*.
//! It is redacted before they see it, and they may edit it before they send it.
//!
//! `tray::diagnostics_text` is a different thing and stays as it is: it goes to the clipboard, it is
//! three lines long, and nobody transmits it.
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use tauri::State;
use crate::state::ConsoleState;

/// One redaction, five classes, in this order — **URLs before tokens** (or a URL's path reads as a
/// token and the sentence loses its shape), **the Windows account name before either**, and **notes
/// before tokens** (a filename is the more specific fact). The cloud's `_shared/scrub.ts` is the same
/// five in the same order, and `app/tests/report.rs` drives both against the same seven cases.
///
/// **The two are twins on those cases, not in general**, and the difference is worth naming: this one
/// works on whole space-delimited segments, so `see(https://x/y)` becomes `<url>` here and
/// `see(<url>` in TypeScript, which keeps the closing bracket. Both redact the URL — which is the
/// property that matters — and neither is more correct; a regex rewrite of this side would make them
/// identical and is not worth a dependency the app does not otherwise carry.
pub fn scrub(text: &str) -> String {
    mask_home(text).split(' ').map(scrub_segment).collect::<Vec<_>>().join(" ")
}

/// `…:\Users\<whoever>` → `…:\Users\<user>`, case-insensitively, before anything else runs. The
/// engine's stdout is full of paths and the Windows account name is usually a person's name. Only
/// that one segment goes: the rest of the path is what makes a log readable to whoever reads it.
fn mask_home(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while let Some(rel) = lower[i..].find(":\\users\\") {
        let at = i + rel + ":\\users\\".len();
        out.push_str(&text[i..at]);
        let rest = &text[at..];
        let end = rest
            .find(|c: char| c == '\\' || c == '/' || c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        out.push_str("<user>");
        i = at + end;
    }
    out.push_str(&text[i..]);
    out
}

fn scrub_segment(seg: &str) -> String {
    if seg.contains("://") { return "<url>".to_string(); }
    if seg.contains('@') && seg.contains('.') { return "<email>".to_string(); }
    // Split into runs of "word-ish" characters and everything else, keeping both, so `key=sk_live_…`
    // and `authorization:` are handled without losing the separators a reader needs.
    let wordish = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
    let mut out = String::with_capacity(seg.len());
    let mut piece = String::new();
    for c in seg.chars() {
        if wordish(c) {
            piece.push(c);
        } else {
            out.push_str(&classify(&piece));
            piece.clear();
            out.push(c);
        }
    }
    out.push_str(&classify(&piece));
    out
}

fn classify(piece: &str) -> String {
    if piece.is_empty() { return String::new(); }
    if piece.to_ascii_lowercase().ends_with(".md") { return "<note>".to_string(); }
    if piece.len() >= 20 && piece.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
        return "<token>".to_string();
    }
    piece.to_string()
}

/// How many notes are in each folder. **Counts, never names** (spec §6): a filename is a slugified
/// title, and a unique course schedule is not depersonalised by dropping a name off the front of it.
pub fn vault_shape(vault: &Path) -> Vec<(String, usize)> {
    ["tasks", "approvals", "archive", "courses", "info", "issues"]
        .iter()
        .map(|d| {
            let n = std::fs::read_dir(vault.join(d))
                .map(|rd| rd.flatten().filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false)).count())
                .unwrap_or(0);
            ((*d).to_string(), n)
        })
        .collect()
}

/// The last `lines` lines across the newest slot logs, oldest first, each scrubbed. The engine's own
/// stdout is in these files — that is the point of them — and it names notes and feeds.
pub fn log_tail(data_dir: &Path, lines: usize) -> Vec<String> {
    let dir = data_dir.join("logs");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("slot-"))
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (t, e.path())))
        .collect();
    files.sort();
    let mut all: Vec<String> = Vec::new();
    for (_, p) in files {
        if let Ok(text) = std::fs::read_to_string(&p) {
            for l in text.lines() { all.push(scrub(l)); }
        }
    }
    if all.len() > lines { all.drain(..all.len() - lines); }
    all
}

fn os_build() -> String {
    // No new dependency for one line: the same value `ver` reports, read from the environment Windows
    // already exports. Absent on a machine that has neither, which is honest.
    std::env::var("OS").unwrap_or_else(|_| "unknown".to_string())
}

/// Everything the report carries, as the text the user is about to read. Spec §6: the last 200 lines
/// of the logs, the two build shas, the OS, the profile id, the vault's shape as counts, and the
/// **name** of the view the user was looking at — never that view's contents, and never its keys
/// either: a view's key set is a fact about the read model, not about this install, and the name is
/// the only part a reader of the report can act on (ruling R-C1-5).
pub fn preview_text(cs: &ConsoleState, view: &str) -> String {
    let mut out = String::new();
    out.push_str("--- Knowlu issue report ---\n");
    out.push_str(&format!("app {} build {}\n", env!("CARGO_PKG_VERSION"), crate::commands::CONSOLE_BUILD.unwrap_or("unknown")));
    out.push_str(&format!("os {}\n", os_build()));
    out.push_str(&format!("profile {}\n", cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default()));
    out.push_str(&format!("view {view}\n"));
    let shape: Vec<String> = vault_shape(&cs.vault).into_iter().map(|(k, n)| format!("{k}={n}")).collect();
    out.push_str(&format!("vault {}\n", shape.join(" ")));
    if let Some(e) = &cs.settings_error { out.push_str(&format!("settings error: {}\n", scrub(e))); }
    if let Some(e) = &crate::scheduler::lock(&cs.history).last_error { out.push_str(&format!("sync error: {}\n", scrub(e))); }
    if let Some(e) = &crate::scheduler::lock(&cs.backup).last_error { out.push_str(&format!("backup error: {}\n", scrub(e))); }
    let runs = knowlu_engine::surface::runs_panel(&cs.vault, jiff::Timestamp::now());
    for r in runs.recent.iter().take(3) {
        out.push_str(&scrub(&format!("run {} {} {} {}", r.runner, r.started.clone().unwrap_or_default(), r.result, r.summary)));
        out.push('\n');
    }
    out.push_str("--- the last 200 log lines ---\n");
    for l in log_tail(&cs.data_dir, 200) { out.push_str(&l); out.push('\n'); }
    out.push_str("--- what went wrong (write here) ---\n\n");
    out
}

pub fn send_at(api_base: &str, token: &str, text: &str, profile_id: &str) -> Result<String, String> {
    crate::account::check_api_base(api_base)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let body = json!({
        "body": text,
        "app_version": env!("CARGO_PKG_VERSION"),
        "engine_build": crate::commands::CONSOLE_BUILD,
        "os_build": os_build(),
        "profile_id": profile_id,
    });
    let mut res = agent
        .post(&format!("{}/issues", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .send_json(&body)
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let reply = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) { return Err(format!("the report was not accepted ({status})")); }
    let v: Value = serde_json::from_str(&reply).unwrap_or(Value::Null);
    Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string())
}

/// The preview screen's text. Everything is already redacted; the page shows it in a textarea the
/// user can edit, and what they press Send on is what arrives.
#[tauri::command(async)]
pub fn report_preview(cs: State<'_, ConsoleState>, view: String) -> Value {
    json!({ "ok": true, "error": Value::Null, "text": preview_text(&cs, &view) })
}

/// Send exactly what the page had. **The text is not rebuilt here** — rebuilding it would send
/// something the user never read, which is the one thing the preview exists to prevent.
#[tauri::command(async)]
pub fn report_send(cs: State<'_, ConsoleState>, text: String) -> Value {
    let profile_id = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    let out = (|| -> Result<String, String> {
        let cfg = crate::account::cloud_config(&cs.vault)?;
        let auth = crate::account::auth_base(&cfg.api_base)?;
        let token = crate::account::valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, jiff::Timestamp::now().as_second())?;
        send_at(&cfg.api_base, &token, &text, &profile_id)
    })();
    match out {
        Ok(id) => json!({ "ok": true, "error": Value::Null, "id": id }),
        Err(e) => json!({ "ok": false, "error": e, "id": Value::Null }),
    }
}
```

- [ ] **Step 4: Run and watch them pass.** `cargo test -p knowlu --test report` → `4 passed`.

- [ ] **Step 5: Run everything.** `cargo test -p knowlu` at 0 warnings.

- [ ] **Step 6: Commit.** `app: report.rs — the preview IS the payload, scrubbed by default and editable, and the send never rebuilds it (C1 Task 16)`.

---
### Task 17: The wizard of §4.2 — nine panels, no folder question, and the issue-report screen

**Files:**
- Modify: `app/static/index.html`, `app/static/console.js`, `app/static/console.css`
- Test: `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `onboarding::{launch_state, create_vault, store_credentials, retarget_credentials, finish_onboarding}`; `account::{sign_up, sign_in, send_magic_link, verify_email_code, sign_out, entitlement_now, open_checkout, open_policy, account_status, open_portal, delete_my_data}` — **`entitlement_now` and `open_checkout` are written in Task 18**, beside the rest of the console-side surface, so the subscribe panel and its poll are inert markup until that task lands; the page is strings and `main.rs` is merge-time, so nothing fails to compile in between; `lms_link::{open_lms_window, capture_calendar_link, paste_calendar_link, close_lms_window}`; `report::{report_preview, report_send}`.
- Produces: `PANELS` (nine), `window.KNOWLU_OPEN_REPORT`, and the `KNOWLU_SHOTS` seam extended with `openReport`.

- [ ] **Step 1: Write the failing tests.** In `app/tests/static_assets.rs`, replace `the_wizard_has_seven_panels_the_privacy_words_and_no_live_fetch` with the tests below. **Two tests already in that file are about markup this task removes, and both are settled here — neither pin is dropped, both move:**

  - **Delete `the_wizard_offers_local_judgment_without_doing_anything`.** It asserts `html.contains("id=\"wiz-judge\"")` and `js.contains("offer_inference")` — the checkbox and the flag this task takes off the page. Its pin moves, inverted, into `the_wizard_never_offers_a_local_model` (the last test in this step), which asserts `!html.contains("id=\"wiz-judge\"")` and that nothing on the page names a runtime, a model or `offer_inference`. Keeping both would make the suite unpassable in either direction.
  - **Amend `the_wizard_takes_its_default_folders_from_the_launch_state`.** It asserts `js.contains("l.default_backup")`, and the backup panel is gone: `create_vault` puts `Backups` beside the vault itself (Task 12's `vault_dest_in`) and the wizard never sees a backups root. The body becomes exactly this — the `default_parent` half stays, and the `default_backup` half inverts:

```rust
#[test]
fn the_wizard_takes_its_default_folders_from_the_launch_state() {
    let js = read("console.js");
    // The vault's parent still comes from `launch_state` (renamed on main 2026-09-09:
    // `documents` -> `default_parent`).
    assert!(js.contains("l.default_parent"), "the parent folder still comes from launch_state");
    // …but nothing reads a backups root any more. There is no backup panel and no folder picker in
    // the wizard at all; `create_vault` puts `Backups` beside the vault (Task 12, spec §4.1).
    assert!(!js.contains("l.default_backup"), "the wizard must not read a backups root it cannot show");
}
```

  And the replacement for `the_wizard_has_seven_panels_the_privacy_words_and_no_live_fetch`:

```rust
#[test]
fn the_wizard_has_nine_panels_and_the_privacy_words_and_no_live_fetch() {
    let html = read("index.html");
    assert!(html.contains("id=\"wizard\""));
    // Spec §4.2, in order. **No `wiz-backup`, and no folder anywhere**: the app creates
    // `%USERPROFILE%\Knowlu\<name>` and `Backups` beside it (§4.1, §11a).
    for p in ["wiz-welcome", "wiz-account", "wiz-subscribe", "wiz-vault", "wiz-calendars", "wiz-logins", "wiz-gmail", "wiz-slots", "wiz-finish"] {
        assert!(html.contains(&format!("id=\"{p}\"")), "panel {p}");
    }
    assert!(!html.contains("id=\"wiz-backup\""), "the backup-folder panel is gone");
    assert!(!html.contains("id=\"wiz-pick-parent\"") && !html.contains("id=\"wiz-pick-bdir\""), "no folder is picked in the wizard");
    let js = read("console.js");
    for f in ["startWizard", "renderWizard", "wizGo", "wizFinish"] {
        assert!(js.contains(&format!("function {f}(")), "missing {f}");
    }
    assert!(js.contains("var PANELS = [\"welcome\", \"account\", \"subscribe\", \"vault\", \"calendars\", \"logins\", \"gmail\", \"slots\", \"finish\"];"));
    // No live fetch in onboarding: the ICS shape is matched, never requested, by the PAGE. The
    // escape keeps the network-reference rule true.
    assert!(js.contains("/^https:\\/\\/"), "the ICS check is an escaped regex");
    assert!(!js.contains("fetch(\"http"), "the page never fetches");
    // Credentials leave page memory the moment the write returns (spec §5).
    assert!(js.contains("clearCredentialFields("), "the fields are cleared by name");
    assert!(js.contains("window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker, openSettings: openSettings, openReport: openReport }"), "the shots seam");
    assert!(js.contains("retarget_credentials"), "Finish moves the credentials when the name changed");
    // D5: class (c) — raw note bodies for model improvement — is not built and has no UI, and the
    // page is where a toggle for it would appear. The pin predates C1 and is kept for exactly that.
    assert!(!js.to_lowercase().contains("telemetry"), "no telemetry toggle: (c) is not built (D5)");
    assert_eq!(js.matches(" data-id=\"").count(), js.matches(" data-kind=\"").count(), "data-id without data-kind somewhere");
}

/// Spec §11a and VISION's standing rule: **Knowlu never asks for a campus credential.** The student
/// types into the university's own page, inside a window we opened and then throw away. The page has
/// exactly three password fields and they are all ours: the account's, and the two coursework logins
/// the student explicitly chose to store in Credential Manager (D11).
#[test]
fn the_page_has_no_lms_credential_field_anywhere() {
    let html = read("index.html");
    let js = read("console.js");
    // Every password field on the page is one of ours, by id: the wizard's account password, the
    // upgrade overlay's (Task 18), and the two coursework logins the student chose to store (D11).
    // Counted by allow-list rather than by number, so adding one of ours is fine and adding
    // anybody else's is not.
    const OURS: [&str; 4] = ["wiz-pw", "up-pw", "wiz-zy-pass", "wiz-vhl-pass"];
    let mut seen = 0usize;
    for (i, _) in html.match_indices("type=\"password\"") {
        let around = &html[i.saturating_sub(200)..(i + 200).min(html.len())];
        assert!(OURS.iter().any(|id| around.contains(&format!("id=\"{id}\""))), "an unknown password field near: {around}");
        seen += 1;
    }
    assert!(seen >= 3, "the account password and the two coursework logins are all still there");
    for id in ["wiz-pw", "wiz-zy-pass", "wiz-vhl-pass"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "password field {id}");
    }
    // The LMS panel holds a button, a status line and a paste field — and nothing to type a school
    // password into. Checked over the panel's own markup, so a field added there fails here.
    let panel = html.split("id=\"wiz-calendars\"").nth(1).and_then(|s| s.split("id=\"wiz-logins\"").next()).expect("the calendars panel");
    assert!(!panel.contains("password"), "the calendars panel must never carry a password field");
    assert!(!panel.to_lowercase().contains("username"), "…nor a username field");
    assert!(panel.contains("id=\"wiz-lms-open\"") && panel.contains("id=\"wiz-ics\""), "sign-in button and paste fallback");
    // The campus is asked HERE, on the panel that uses it — not two panels later, where it used to be
    // and where it made every sign-in answer "no sign-in page is known for that school yet".
    // R-OB-4: a search over every US institution, not two radios. The radios are gone from the whole
    // page — a list of two schools was a placeholder that read like a decision.
    assert!(panel.contains("id=\"wiz-school\"") && panel.contains("id=\"wiz-school-hits\""), "the school typeahead");
    assert!(panel.contains("id=\"wiz-school-none\"") && panel.contains("id=\"wiz-school-free\""), "…and the free-text fallback");
    assert!(panel.contains("id=\"wiz-lms-kind\""), "…and the two-button LMS question for an uncurated school");
    assert!(!html.contains("name=\"campus\""), "no campus radios anywhere on the page");
    assert!(!js.contains("input[name=\\\"campus\\\"]"), "…and nothing reads one");
    assert!(js.contains("function schoolHits(") && js.contains("\"campus_search\""), "the typeahead asks Rust");
    // **The page must not fetch the list.** `app/tauri.conf.json`'s CSP is
    // `connect-src ipc: http://ipc.localhost` with no `'self'`, so a `fetch` of a bundled asset is
    // refused — and that file is the controller's. The search is a command; the page holds ten rows.
    assert!(!js.contains("campuses.json"), "the page never names the asset; `campus_search` reads it");
    // Spec §11a: **both** calendars, on this one panel, before coursework logins and Gmail — the
    // personal one is what makes today's page know the day is already half full.
    assert!(panel.contains("id=\"wiz-cal-ics\"") && panel.contains("id=\"wiz-cal-note\""), "the personal calendar's field");
    assert!(panel.contains("Secret address in iCal format"), "the panel says where the address is");
    assert!(panel.contains("Reset"), "…and advises resetting it first");
    // C2's Google sign-in has a labelled place and does nothing yet — a button that lied would be
    // worse than a button that says when it arrives.
    assert!(panel.contains("id=\"wiz-google\"") && panel.contains("disabled"), "the Google placeholder is present and inert");
    // R-OB-2: the enrolled classes are confirmed on this panel — captured from the sign-in window if
    // the campus lets us, typed if it does not. Without them a first ingest is 28 tasks with no
    // course, which is the run this section of the plan exists because of.
    assert!(panel.contains("id=\"wiz-courses\"") && panel.contains("id=\"wiz-course-rows\""), "the class list");
    assert!(panel.contains("id=\"wiz-course-add\""), "…and the typed fallback beside it");
    assert!(js.contains("\"capture_courses\"") && js.contains("function renderCourses("), "the capture and its rows");
    assert!(!js.contains("\"connect_google\"") && !js.contains("gmail.readonly"), "no Google connect in C1");
    let slots = html.split("id=\"wiz-slots\"").nth(1).and_then(|s| s.split("id=\"wiz-finish\"").next()).expect("the slots panel");
    assert!(!slots.contains("id=\"wiz-school\""), "the school must not also be on the slots panel");
}

/// R-OB-1: the wizard that takes a coursework password must also say what the work is for. Quinn's
/// first slot had both logins stored and `courses: {}` in the config, so the engine answered
/// `zybook UACS100Fall2026 not in config; skipped` and then `0 assignments parsed; treating as
/// failure` — three warnings for one missing sentence.
#[test]
fn the_logins_panel_maps_what_it_finds_to_a_course() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-logins\"").nth(1).and_then(|s| s.split("id=\"wiz-gmail\"").next()).expect("the logins panel");
    assert!(panel.contains("id=\"wiz-map\"") && panel.contains("id=\"wiz-map-rows\""), "the mapping block");
    let js = read("console.js");
    assert!(js.contains("\"discover_coursework\""), "discovery runs after the credentials are stored");
    assert!(js.contains("function renderMapping("), "renderMapping");
    // The mapping travels in the plan, and the SLUGS are made in Rust from the codes — a page that
    // invented vault identifiers would be a page deciding what the engine may know.
    assert!(js.contains("zybooks_courses:") && js.contains("vhl_sections:") && js.contains("course_map:"), "the plan carries the mapping");
    assert!(!js.contains("slugify"), "slugs are `knowlu_engine::ingest::slugify`'s, never the page's");
}

/// Spec §4.2 step 1 and §9's minors row: one attestation, one acceptance, both linked to the text.
#[test]
fn the_account_panel_gates_on_eighteen_and_links_both_policies() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-account\"").nth(1).and_then(|s| s.split("id=\"wiz-subscribe\"").next()).expect("the account panel");
    assert!(panel.contains("id=\"wiz-18\""), "the 18+ attestation checkbox");
    assert!(panel.contains("id=\"wiz-terms\""), "the terms + privacy acceptance checkbox");
    assert!(panel.contains("18 or older"), "the attestation says what it means");
    // The two policies are named where they are accepted, as relative names — the page still carries
    // no `http(s)://` literal, and `open_policy` is what turns them into a published URL.
    assert!(panel.contains("terms.html") && panel.contains("privacy.html"), "both policies are linked");
    let js = read("console.js");
    // …and a click on either **must not navigate this window**: `app/static/` has four files, so a
    // plain navigation would lose the only window the app has, mid-consent.
    assert!(js.contains("a.policy") && js.contains("preventDefault()") && js.contains("\"open_policy\""),
        "the policy links must open in the system browser, not in this webview");
    assert!(js.contains("\"sign_up\"") && js.contains("\"sign_in\"") && js.contains("\"send_magic_link\"") && js.contains("\"verify_email_code\""));
    // Next is refused until both boxes are ticked — said on the panel, and enforced again in Rust.
    assert!(js.contains("Tick both boxes"), "the page says why Next is refused");
}

/// Legal note §9: the report is shown, editable, before anything is sent — and what is sent is what
/// was shown, not something rebuilt after the user looked away.
#[test]
fn the_issue_report_is_previewed_edited_and_sent_verbatim() {
    let html = read("index.html");
    assert!(html.contains("id=\"report\""), "the report overlay");
    assert!(html.contains("id=\"report-text\""), "an editable textarea");
    assert!(html.contains("id=\"report-send\"") && html.contains("id=\"report-cancel\""));
    let js = read("console.js");
    assert!(js.contains("function openReport("), "openReport");
    assert!(js.contains("window.KNOWLU_OPEN_REPORT = openReport"), "the tray's one way in");
    assert!(js.contains("\"report_preview\"") && js.contains("\"report_send\""));
    // The send passes the TEXTAREA's value. A send that passed anything else would be sending
    // something the user never read.
    assert!(js.contains("invoke(\"report_send\", { text: EL(\"report-text\").value })"), "send exactly what is on screen");
}

/// **One string, two languages** — the same pin `PRIVACY` gets, for the same reason. `account.rs`
/// emits every transport failure as `"{UNREACHABLE} ({e})"`, and the upgrade overlay decides whether
/// to stand itself down by testing that the error *starts with* it. A silent drift here does not fail
/// anything: it just quietly stops standing the overlay down, on the one path where a student with no
/// network would otherwise be stuck behind it.
#[test]
fn the_unreachable_clause_is_one_string_on_both_sides() {
    let rust = fs::read_to_string("src/account.rs").expect("src/account.rs");
    let clause = rust
        .split("pub const UNREACHABLE: &str = \"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("account.rs must declare `pub const UNREACHABLE: &str = \"…\";`");
    assert!(!clause.is_empty());
    let js = read("console.js");
    assert!(
        js.contains(&format!("var UNREACHABLE = \"{clause}\";")),
        "console.js's UNREACHABLE must be account.rs's, word for word: {clause:?}"
    );
}

/// The local runtime leaves in C4; the wizard stops offering it now, because §4.2's step list has no
/// such step and a wizard that offers a 2 GB download for a feature that is moving to the cloud is
/// lying to a new user.
#[test]
fn the_wizard_never_offers_a_local_model() {
    let html = read("index.html");
    assert!(!html.contains("id=\"wiz-judge\""), "the finish panel's local-judgment offer is gone");
    assert!(!html.contains("2 GB"), "…and so is its download size");
    let js = read("console.js");
    // **Asserted by position, not by slice.** Whether `openSettings` — which legitimately calls these
    // until C4 — happens to fall inside a text slice is a fact about line ordering, not about the
    // wizard. Each command appears exactly once, and after `renderInference`, which is the settings
    // row's own function; anything the wizard called would appear earlier and twice.
    // The flag the deleted `the_wizard_offers_local_judgment_without_doing_anything` pinned, inverted:
    // the checkbox is gone from the markup, so the plan field it filled must be gone from the page.
    assert!(!js.contains("offer_inference"), "the wizard's local-judgment flag is gone with its checkbox");
    let at = |needle: &str| js.find(needle).unwrap_or_else(|| panic!("{needle} is not in console.js"));
    let inference = at("function renderInference(");
    for cmd in ["\"install_inference_download\"", "\"install_inference_file\"", "\"inference_status\""] {
        assert_eq!(js.matches(cmd).count(), 1, "{cmd} is invoked from more than one place");
        assert!(at(cmd) > inference, "the wizard has nothing to do with the local runtime: {cmd}");
    }
}
```

…and update `the_wizards_privacy_sentence_is_the_sites_privacy_sentence` to look for the new sentence (Task 19 writes it into `site/privacy.html`; this test keeps checking that the two are one string):

```rust
        .find(|l| l.starts_with("<p>") && l.contains("Your vault stays on this machine"))
```

- [ ] **Step 2: Run and watch them fail.** `cargo test -p knowlu --test static_assets` → **eight** failures. Five are the new or rewritten tests naming the missing panels, ids and handlers; the sixth is the amended `the_wizard_takes_its_default_folders_from_the_launch_state`, which fails on `l.default_backup` until step 4 rewrites the wizard; and two more are existing tests this step's rewrites make red until later steps — `the_logins_panel_maps_what_it_finds_to_a_course` (green at step 4) and `the_unreachable_clause_is_one_string_on_both_sides` (green at step 6a). (`the_wizard_offers_local_judgment_without_doing_anything` is deleted in step 1, so it neither passes nor fails.)

- [ ] **Step 3: `app/static/index.html`** — replace the whole `<section class="wiz" id="wizard">` block with:

```html
<section class="wiz" id="wizard" hidden>
  <div class="wiz-hd"><h1>Knowlu</h1><span class="n" id="wiz-step"></span></div>
  <div class="wiz-panel" id="wiz-welcome" hidden><h2>Welcome</h2>
    <p class="lede">Knowlu answers one question every morning: what should I work on today, and in what order?</p>
    <p id="wiz-privacy"></p>
    <p class="meta">Knowlu costs $9.99 a month after a 7-day trial. You can cancel any time, in Settings.</p>
  </div>
  <div class="wiz-panel" id="wiz-account" hidden><h2>Your account</h2>
    <div class="wiz-row"><input type="text" id="wiz-email" placeholder="Email"><input type="password" id="wiz-pw" placeholder="Password"></div>
    <label><input type="checkbox" id="wiz-18"> I am 18 or older</label>
    <label><input type="checkbox" id="wiz-terms"> I accept the <a href="terms.html" class="policy" data-policy="terms" id="wiz-terms-link">terms</a> and the <a href="privacy.html" class="policy" data-policy="privacy" id="wiz-privacy-link">privacy policy</a></label>
    <div class="wiz-row"><button class="b pri y" id="wiz-create">Create account</button><button class="b" id="wiz-signin">I already have one</button><button class="b" id="wiz-magic">Email me a link</button></div>
    <div class="wiz-row" id="wiz-code-row" hidden><input type="text" id="wiz-code" placeholder="The 6-digit code from the email"><button class="b pri y" id="wiz-code-go">Sign in with the code</button></div>
    <p class="meta" id="wiz-account-note"></p>
  </div>
  <div class="wiz-panel" id="wiz-subscribe" hidden><h2>Subscribe</h2>
    <p class="lede">$9.99 a month, or $69.99 for the academic year. Seven days free first; your card is taken now and not charged until the trial ends. Cancel any time in Settings.</p>
    <div class="wiz-row"><button class="b pri y" id="wiz-sub-month">Start the trial — $9.99/month</button><button class="b" id="wiz-sub-year">$69.99 / academic year</button></div>
    <p class="meta" id="wiz-sub-note"></p>
  </div>
  <div class="wiz-panel" id="wiz-vault" hidden><h2>Name this setup</h2>
    <p class="lede">Knowlu keeps your work as plain files you can open, copy and take with you.</p>
    <div class="wiz-row"><input type="text" id="wiz-name" placeholder="Name"></div>
    <p class="meta" id="wiz-vault-path"></p>
  </div>
  <div class="wiz-panel" id="wiz-calendars" hidden><h2>Connect your calendars</h2>
    <p class="lede">Two things, and Knowlu needs both before it can tell you what to do today: what your school says is due, and what your week already looks like.</p>
    <h3>Your school</h3>
    <p class="meta">Start typing its name. Every US college and university is in this list.</p>
    <div class="wiz-row"><input type="text" id="wiz-school" placeholder="Search for your school" autocomplete="off"><span class="meta" id="wiz-school-picked"></span></div>
    <div id="wiz-school-hits" class="wiz-hits"></div>
    <p class="meta"><button class="b" id="wiz-school-none">My school isn&rsquo;t listed</button></p>
    <div class="wiz-row" id="wiz-school-free" hidden><input type="text" id="wiz-school-name" placeholder="Your school&rsquo;s name"><input type="text" id="wiz-school-state" placeholder="State, e.g. AL"></div>
    <p class="meta">Sign in the way you always do. Knowlu opens your school&rsquo;s own page in a window, you sign in there, and Knowlu keeps only the calendar link &mdash; never what you type into it.</p>
    <div class="wiz-row" id="wiz-lms-kind" hidden><span class="meta">Which does your school use?</span><button class="b" data-lms="blackboard">Blackboard</button><button class="b" data-lms="canvas">Canvas</button></div>
    <div class="wiz-row"><button class="b pri y" id="wiz-lms-open">Sign in to my school</button><span class="meta" id="wiz-lms-state"></span></div>
    <p class="meta">Or paste the link yourself: Blackboard &rarr; Calendar &rarr; Calendar Settings &rarr; Share Calendar. Canvas &rarr; Calendar &rarr; Calendar Feed.</p>
    <input type="text" id="wiz-ics" placeholder="Paste the school feed link (it ends in .ics)">
    <p class="meta" id="wiz-ics-note"></p>
    <div id="wiz-courses" hidden>
      <h3>Your classes</h3>
      <p class="meta" id="wiz-courses-note"></p>
      <div id="wiz-course-rows"></div>
      <div class="wiz-row"><input type="text" id="wiz-course-add" placeholder="Add a course code, e.g. CS 100"><button class="b" id="wiz-course-add-go">Add</button></div>
    </div>
    <h3>Your own calendar</h3>
    <p class="meta">This is what makes today&rsquo;s page know your day is already half full. In Google Calendar: <b>Settings</b> &rarr; click your calendar under <b>Settings for my calendars</b> &rarr; <b>Integrate calendar</b> &rarr; copy the <b>Secret address in iCal format</b>. Press <b>Reset</b> beside it first if that address has ever been shared with anyone &mdash; whoever holds it can read your calendar.</p>
    <input type="text" id="wiz-cal-ics" placeholder="Paste the secret iCal address">
    <p class="meta" id="wiz-cal-note"></p>
    <div class="wiz-row" id="wiz-google-row"><button class="b" id="wiz-google" disabled>Sign in with Google instead</button><span class="meta">Next release: one Google sign-in for your calendar, and email later if you want it.</span></div>
  </div>
  <div class="wiz-panel" id="wiz-logins" hidden><h2>Coursework logins</h2>
    <p class="lede">Optional. Stored in Windows Credential Manager on this machine &mdash; never in the vault, never in a backup, and never sent to us.</p>
    <div class="wiz-row"><input type="text" id="wiz-zy-user" placeholder="zyBooks email"><input type="password" id="wiz-zy-pass" placeholder="zyBooks password"></div>
    <div class="wiz-row"><input type="text" id="wiz-vhl-user" placeholder="VHL email"><input type="password" id="wiz-vhl-pass" placeholder="VHL password"></div>
    <div id="wiz-map" hidden>
      <h3>Which class is which?</h3>
      <p class="meta" id="wiz-map-note">Knowlu found these on your accounts. Confirm the course each one belongs to &mdash; without this, Knowlu can see the work but not what it is for.</p>
      <div id="wiz-map-rows"></div>
    </div>
  </div>
  <div class="wiz-panel" id="wiz-gmail" hidden><h2>Gmail</h2>
    <p class="lede">Knowlu can read your inbox for things you have to do, and propose them.</p>
    <p class="meta">Not yet: Google is still reviewing our request for this, so today it works for a handful of named test users only, and those users have to re-connect every week. When the review is done this step will connect in one click. Skip it for now.</p>
  </div>
  <div class="wiz-panel" id="wiz-slots" hidden><h2>When should Knowlu look?</h2>
    <div class="wiz-row"><input type="text" id="wiz-tz" placeholder="Timezone"><input type="text" id="wiz-slot1" placeholder="12:00"><input type="text" id="wiz-slot2" placeholder="18:00"></div>
    <label><input type="checkbox" id="wiz-autostart" checked> Start Knowlu with Windows</label>
  </div>
  <div class="wiz-panel" id="wiz-finish" hidden><h2>Ready</h2><p class="lede" id="wiz-summary"></p>
    <p class="meta">Your first look runs as soon as you finish, and today&rsquo;s page will be waiting.</p>
  </div>
  <div class="wiz-nav"><span class="crit" id="wiz-error"></span><button class="b" id="wiz-back">Back</button><button class="b pri y" id="wiz-next">Next</button></div>
</section>
```

…add the report overlay beside the settings panel:

```html
<aside class="setpanel" id="report" hidden>
  <div class="set-hd"><h2>Report an issue</h2><button class="b" id="report-cancel">Close</button></div>
  <p class="meta">This is exactly what will be sent. Emails, links and long keys are already replaced. Edit anything you do not want to send, and add what went wrong at the bottom.</p>
  <textarea id="report-text" rows="20"></textarea>
  <div class="set-row"><span class="meta" id="report-note"></span><button class="b pri y" id="report-send">Send</button></div>
</aside>
```

…and three rows in the settings panel, after `set-updates` (the local-judgment row stays until C4):

```html
  <div class="set-row" id="set-account"><span class="k">Account</span><span class="meta" id="set-account-state"></span><button class="b" id="set-portal">Manage subscription</button></div>
  <div class="set-row" id="set-report"><span class="k">Problems</span><button class="b" id="set-report">Report an issue</button></div>
  <div class="set-row" id="set-delete"><span class="k">Delete my data</span><span class="meta" id="set-delete-note"></span><button class="b" id="set-delete-1">Delete everything</button><button class="b crit" id="set-delete-2" hidden>Yes, delete it all</button></div>
```

- [ ] **Step 4: `app/static/console.js` — the wizard.** Replace `PANELS`, `WIZ`, `dest`, `startWizard`, `renderWizard`, `wizValid`, `wizGo`, `wizRegister` and `wizFinish`; delete the backup-folder helpers (`within` is used by nothing else now, so it goes too) and `readSlotsPanel`'s backup lines. Three smaller edits go with them:

  * **`readSlotsPanel` loses its campus read.** Its last two lines (`var picked = document.querySelector('input[name="campus"]:checked'); WIZ.campus = picked ? picked.value : "none";`) are deleted. The campus is the question *which school?*, it belongs on the calendar panel where it is used, and reading it here — two panels **after** `open_lms_window` needs it — is why every sign-in would have answered "no sign-in page is known for that school yet".
  * **The campus radios and their listener are deleted outright** (R-OB-4). `EL("wiz-campus")` does not exist any more: the school is a typeahead over `campuses.json`, and its handlers are in the block below. Delete the old `EL("wiz-campus").addEventListener("change", …)` and do not replace it.

  * **The timezone the student types wins.** Add, beside the slots panel's existing `input` listeners, `EL("wiz-tz").addEventListener("input", function () { WIZ.tzTouched = true; });` — the school's state suggests a zone into that field, and a suggestion must never overwrite an answer.
  * **`credentialsStranded` sends the user to panel 5, not panel 4.** The logins panel moved: `WIZ.step = 4;` becomes `WIZ.step = 5;`. Panel 4 is now the calendar, and stranding a credential there would ask the user to re-enter a coursework password on a panel that has no field for one.

  Then the replacements themselves:

```js
  var PANELS = ["welcome", "account", "subscribe", "vault", "calendars", "logins", "gmail", "slots", "finish"];
  // `parent` comes from `launch_state.default_parent` (`%USERPROFILE%\Knowlu`) and is never picked:
  // spec §4.1 — the app creates the folder and nobody is asked about it. It is still in WIZ because
  // `dest()` is what the credential target is derived from, and a rename on the vault panel has to
  // move the coursework logins with it (R-P4a-23).
  var WIZ = { step: 0, parent: "", name: "Knowlu", email: "", accountId: "", entitled: false,
              ics: "", icsNote: "", cal: "", calNote: "",
              // R-OB-4: the school the student picked — a unitid, a name, a state and (once
              // something establishes it) an LMS kind. The LIST is never here: `campus_search` is a
              // command, and the page holds only the ten rows it is showing.
              campus: { unitid: "", name: "", state: "", lms: "" },
              // R-OB-1 and R-OB-2. `map` is one row per discovered book/section, each with the
              // student's confirmed course; `courses` is the enrolment, captured or typed. Both end
              // up in the plan, and both are allowed to be empty — a student with no coursework
              // logins has nothing to map, and a campus whose API we cannot read is typed in.
              map: [], courses: [],
              // **`campus` is declared once, above.** A second `campus:` key here would silently
              // replace the choice object with a string, `WIZ.campus.unitid` would be `undefined` in
              // every `lms_link` call, and `wizFinish` would send a `campus_choice` serde cannot read.
              // `campuses: []` is gone with it: Task 14c takes the list out of `launch_state`.
              sessionDir: "", tz: "", slots: ["12:00", "18:00"], autostart: true,
              zy: false, vhl: false, credVault: "", error: "" };

  function dest() {
    var p = WIZ.parent.trim().replace(/[\\/]+$/, ""), n = WIZ.name.trim();
    return p && n ? p + "\\" + n : "";
  }

  function startWizard(l) {
    EL("wizard").hidden = false;
    EL("picker").hidden = true;
    document.querySelector(".app").hidden = true;
    WIZ.tz = l.tz || "";
    WIZ.parent = l.default_parent || "";
    EL("wiz-name").value = WIZ.name;
    EL("wiz-privacy").textContent = PRIVACY;
    EL("wiz-tz").value = WIZ.tz;
    EL("wiz-slot1").value = WIZ.slots[0];
    EL("wiz-slot2").value = WIZ.slots[1];
    renderWizard();
  }

  function renderWizard() {
    PANELS.forEach(function (p, i) { EL("wiz-" + p).hidden = i !== WIZ.step; });
    EL("wiz-step").textContent = "step " + (WIZ.step + 1) + " of " + PANELS.length;
    EL("wiz-back").disabled = WIZ.step === 0;
    EL("wiz-next").textContent = WIZ.step === PANELS.length - 1 ? "Finish" : "Next";
    EL("wiz-error").textContent = WIZ.error;
    EL("wiz-account-note").textContent = WIZ.accountId ? "Signed in as " + WIZ.email : "";
    EL("wiz-sub-note").textContent = WIZ.entitled ? "Your subscription is active." : "Waiting for the payment page in your browser…";
    EL("wiz-vault-path").textContent = dest() ? "Your files will be at " + dest() : "";
    EL("wiz-lms-state").textContent = WIZ.icsNote;
    EL("wiz-ics-note").textContent = WIZ.ics && !ICS_OK.test(WIZ.ics) ? "That does not look like a calendar feed link." : "";
    EL("wiz-cal-note").textContent = WIZ.calNote;
    EL("wiz-summary").textContent = dest() + ", looking at " + WIZ.slots.join(" and ") + " " + WIZ.tz + ".";
  }

  function wizValid() {
    WIZ.error = "";
    if (WIZ.step === 1 && !WIZ.accountId) { WIZ.error = "Create an account or sign in first."; }
    if (WIZ.step === 2 && !WIZ.entitled) { WIZ.error = "Finish the payment page in your browser, then come back."; }
    if (WIZ.step === 3) {
      var n = WIZ.name.trim();
      if (!n) { WIZ.error = "Give this setup a name."; }
      else if (/[\\/:]/.test(n)) { WIZ.error = "A name has no slashes or colons."; }
      else if (/[*?"<>|]/.test(n)) { WIZ.error = "A name has no * ? \" < > or | either."; }
      else if (/[. ]$/.test(n)) { WIZ.error = "A name cannot end in a dot or a space."; }
      else if (RESERVED_NAME.test(n.split(".")[0])) { WIZ.error = "That name is reserved by Windows."; }
    }
    // Panel 4 is the calendars panel. Neither feed is compulsory — a student with no personal
    // calendar still gets a ranked day, and one whose school defeats the capture can come back — but
    // a link that is there and malformed is caught here rather than at Finish.
    if (WIZ.step === 4 && WIZ.ics && !ICS_OK.test(WIZ.ics)) { WIZ.error = "That does not look like a calendar feed link."; }
    if (WIZ.step === 4 && WIZ.cal && !ICS_OK.test(WIZ.cal)) { WIZ.error = "That does not look like a secret iCal address."; }
    return !WIZ.error;
  }

  function wizGo(n) {
    if (n > WIZ.step && !wizValid()) { renderWizard(); return Promise.resolve(); }
    if (n < WIZ.step) { WIZ.error = ""; }
    var leaving = WIZ.step;
    if (leaving === 7) { readSlotsPanel(); }
    // Leaving the calendar panel closes the sign-in window and deletes its session, whether or not a
    // link was captured: a campus login must not outlive the panel that opened it.
    if (leaving === 4 && WIZ.sessionDir) {
      invoke("close_lms_window", { session_dir: WIZ.sessionDir }).catch(function () {});
      WIZ.sessionDir = "";
    }
    WIZ.step = Math.max(0, Math.min(PANELS.length - 1, n));
    if (leaving === 5 && n > leaving) {
      return storeCredentials().then(function (ok) {
        if (!ok) { WIZ.step = leaving; renderWizard(); return; }
        // R-OB-1: the credentials are in Credential Manager now, so this is the first moment discovery
        // can run. Stay on the panel while it does — the mapping is the whole point of having asked
        // for the logins — and let Next work again the moment the rows are on screen.
        if (!WIZ.zy && !WIZ.vhl) { renderWizard(); return; }
        if (WIZ.map.length) { renderWizard(); return; }
        WIZ.step = leaving;
        EL("wiz-map").hidden = false;
        EL("wiz-map-note").textContent = "Looking up your books and sections…";
        renderWizard();
        return invoke("discover_coursework", { vault: dest(), zybooks: WIZ.zy, vhl: WIZ.vhl }).then(function (d) {
          WIZ.map = ((d && d.rows) || []).map(function (r) {
            return { source: r.source, key: r.key, detail: r.detail, suggested: r.suggested, course: r.suggested || "", ignore: !!r.ignored };
          });
          EL("wiz-map-note").textContent = (d && d.note)
            || "Knowlu found these on your accounts. Confirm the course each one belongs to — without this, Knowlu can see the work but not what it is for.";
          renderMapping();
        });
      });
    }
    renderWizard();
    return Promise.resolve();
  }

  function wizRegister(plan) {
    return invoke("create_vault", { name: WIZ.name, plan: plan });
  }

  function wizFinish() {
    readSlotsPanel();
    // R-OB-1 and R-OB-2: the confirmed mapping and the course list, in the shapes `WizardPlan` takes.
    // An ignored row contributes nothing but its place in `ignore:`; a row with no course contributes
    // nothing at all, which leaves that source unmapped and is the student's choice to have made.
    var zyRows = WIZ.map.filter(function (r) { return r.source === "zybooks" && !r.ignore && r.course; });
    var vhlRows = WIZ.map.filter(function (r) { return r.source === "vhl" && !r.ignore && r.course; });
    var codes = {};
    zyRows.concat(vhlRows).forEach(function (r) { codes[r.course] = true; });
    WIZ.courses.forEach(function (c) { if (c.code) { codes[c.code] = true; } });
    var plan = { ics_url: WIZ.ics || null, personal_calendar: WIZ.cal || null,
                 timezone: WIZ.tz, slots: WIZ.slots,
                 zybooks: WIZ.zy, vhl: WIZ.vhl, autostart: WIZ.autostart,
                 campus_choice: WIZ.campus,
                 zybooks_courses: zyRows.map(function (r) { return { code: r.key, course: "", label: r.course }; }),
                 vhl_sections: vhlRows.map(function (r) { return { section: r.key, course: "", label: r.course }; }),
                 course_map: Object.keys(codes).map(function (c) { return [c, ""]; }),
                 courses: WIZ.courses };
    EL("wiz-next").disabled = true;
    var moved = (WIZ.credVault && WIZ.credVault !== dest())
      ? invoke("retarget_credentials", { from_vault: WIZ.credVault, to_vault: dest() })
      : Promise.resolve({ ok: true, error: null });
    return moved.then(function (rt) {
      if (!rt || !rt.ok) { credentialsStranded(); return; }
      if (WIZ.credVault) { WIZ.credVault = dest(); }
      return wizRegister(plan).then(function (r) {
        if (!r.ok) { WIZ.error = r.error; EL("wiz-next").disabled = false; renderWizard(); return; }
        return invoke("finish_onboarding", { id: r.profile.id });
      });
    }).catch(function (e) { WIZ.error = String(e.message || e); EL("wiz-next").disabled = false; renderWizard(); });
  }
```

…and the four new handler blocks, added inside the existing `EL("wizard").addEventListener("click", …)`:

```js
    if (e.target.closest("#wiz-create") || e.target.closest("#wiz-signin")) {
      var creating = !!e.target.closest("#wiz-create");
      if (creating && !(EL("wiz-18").checked && EL("wiz-terms").checked)) {
        WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
      }
      var cmd = creating ? "sign_up" : "sign_in";
      var args = { email: EL("wiz-email").value.trim(), password: EL("wiz-pw").value };
      if (creating) { args.age_attested = EL("wiz-18").checked; }
      invoke(cmd, args).then(function (r) {
        EL("wiz-pw").value = "";                     // the password leaves page memory at once
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        wizGo(2);
      }).catch(function () { WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
    if (e.target.closest("#wiz-magic")) {
      invoke("send_magic_link", { email: EL("wiz-email").value.trim() }).then(function (r) {
        WIZ.error = r.ok ? "" : r.error;
        // The link in the mail lands in the BROWSER, which this process never sees — so the mail also
        // carries a six-digit code, and this is where it is typed. `verify_email_code` trades it for
        // the same session the link would have given.
        EL("wiz-code-row").hidden = !r.ok;
        EL("wiz-account-note").textContent = r.ok ? "We emailed you a 6-digit code. Type it below." : "";
        renderWizard();
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#wiz-code-go")) {
      invoke("verify_email_code", { email: EL("wiz-email").value.trim(), code: EL("wiz-code").value }).then(function (r) {
        EL("wiz-code").value = "";
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        EL("wiz-code-row").hidden = true;
        wizGo(2);
      }).catch(function () { WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
    // **The two policies open in the system browser, not in this window.** `app/static/` holds four
    // files, so a plain navigation to `terms.html` loses the only window Knowlu has — while the user
    // is being asked to tick a box saying they accept it. The `href` stays because it is the honest
    // markup and the static test reads it; this is what actually happens.
    var policy = e.target.closest("a.policy");
    if (policy) {
      e.preventDefault();
      invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
      return;
    }
    if (e.target.closest("#wiz-sub-month") || e.target.closest("#wiz-sub-year")) {
      var which = e.target.closest("#wiz-sub-year") ? "academic_year" : "monthly";
      invoke("open_checkout", { plan: which }).then(function (r) {
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        pollEntitlement();
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#wiz-lms-open")) {
      WIZ.icsNote = "Opening your school’s sign-in page…";
      renderWizard();
      invoke("open_lms_window", { unitid: WIZ.campus.unitid }).then(function (r) {
        if (!r.ok) {
          WIZ.icsNote = r.error;
          // Nobody has curated this school, so nobody knows which LMS it runs. Ask, once — the answer
          // is what C2's server-side fetch will need too.
          EL("wiz-lms-kind").hidden = !!WIZ.campus.lms;
          renderWizard();
          return;
        }
        WIZ.sessionDir = r.session_dir;
        WIZ.icsNote = "Sign in there, then come back — Knowlu will find your calendar link.";
        renderWizard();
        return invoke("capture_calendar_link", { unitid: WIZ.campus.unitid }).then(function (c) {
          if (c.ok && c.link) {
            WIZ.ics = c.link.url;
            WIZ.icsNote = "Found " + c.link.events + " assignments across " + c.link.courses + " courses.";
            // `note` is the one thing `PUT /account/sources` could not do. The link is saved on this
            // machine either way, so it is a sentence beside the count, not a failure.
            if (c.note) { WIZ.icsNote += " " + c.note; }
          } else {
            WIZ.icsNote = (c.error || "No link found") + " — paste it below instead.";
          }
          renderWizard();
          // R-OB-2, in the same sitting and the same window: the enrolled course list. Its own
          // outcome — a campus can give the calendar and not the courses — so a failure here shows
          // the typed field and says nothing about the link that just worked.
          return invoke("capture_courses", { unitid: WIZ.campus.unitid }).then(function (cl) {
            WIZ.courses = (cl && cl.courses) || [];
            EL("wiz-courses").hidden = false;
            EL("wiz-courses-note").textContent = WIZ.courses.length
              ? "These are the classes Knowlu found. Remove any you are not taking."
              : "Knowlu could not read your class list — type the codes yourself, e.g. CS 100.";
            renderCourses();
          });
        });
      }).catch(function () { WIZ.icsNote = "That did not work — paste the link below instead."; renderWizard(); });
      return;
    }
```

…plus the two small helpers:

```js
  // The Checkout page is in the system browser, so the app cannot be told when it is done: it asks.
  // Every three seconds for two minutes, then it stops and the button can be pressed again — a poll
  // that never ends is a poll that runs all night on a laptop somebody closed.
  function pollEntitlement() {
    var tries = 0;
    var tick = function () {
      tries += 1;
      invoke("entitlement_now", {}).then(function (r) {
        if (r.ok && (r.status === "active" || r.status === "trialing")) {
          WIZ.entitled = true; WIZ.error = ""; renderWizard(); wizGo(3); return;
        }
        if (tries < 40) { setTimeout(tick, 3000); }
        else { WIZ.error = "Still not subscribed. Try the payment page again."; renderWizard(); }
      }).catch(function () { if (tries < 40) { setTimeout(tick, 3000); } });
    };
    setTimeout(tick, 3000);
  }

  EL("wiz-ics").addEventListener("change", function () {
    WIZ.ics = EL("wiz-ics").value.trim();
    if (!WIZ.ics) { WIZ.icsNote = ""; renderWizard(); return; }
    // The pasted path validates exactly as the captured one does — same command, same sentence.
    invoke("paste_calendar_link", { kind: "lms_ics", url: WIZ.ics }).then(function (r) {
      WIZ.icsNote = r.ok ? "Found " + r.link.events + " assignments across " + r.link.courses + " courses." : r.error;
      if (r.ok && r.note) { WIZ.icsNote += " " + r.note; }
      renderWizard();
    }).catch(function () {});
  });
  /// One row per enrolled course, removable, plus whatever the student typed. Slugs are made in Rust
  /// (`knowlu_engine::ingest::slugify`) at Finish, from the code — the page never invents a vault identifier.
  function renderCourses() {
    EL("wiz-course-rows").innerHTML = WIZ.courses.map(function (c, i) {
      return '<div class="wiz-row" data-course="' + i + '"><span class="meta">' + h(c.code || c.name) +
             (c.name && c.name !== c.code ? " &middot; " + h(c.name) : "") +
             '</span><button class="b" data-drop="' + i + '">Remove</button></div>';
    }).join("");
  }
  EL("wiz-courses").addEventListener("click", function (e) {
    var drop = e.target.closest("[data-drop]");
    if (drop) { WIZ.courses.splice(Number(drop.getAttribute("data-drop")), 1); renderCourses(); return; }
    if (e.target.closest("#wiz-course-add-go")) {
      var code = EL("wiz-course-add").value.trim();
      if (code) { WIZ.courses.push({ code: code, name: code, slug: "" }); EL("wiz-course-add").value = ""; renderCourses(); }
    }
  });

  /// R-OB-1. One row per discovered book or section: what it is, what we think it is, and a field the
  /// student corrects. A row left blank is a source that stays unmapped — which is a choice, and is
  /// why the panel says what the consequence is rather than refusing Next.
  function renderMapping() {
    EL("wiz-map").hidden = WIZ.map.length === 0;
    EL("wiz-map-rows").innerHTML = WIZ.map.map(function (r, i) {
      return '<div class="wiz-row" data-map="' + i + '"><span class="meta">' + h(r.key) +
             (r.detail ? " &middot; " + h(r.detail) : "") + '</span>' +
             '<input type="text" data-course-for="' + i + '" value="' + h(r.course || r.suggested || "") +
             '" placeholder="Course code, e.g. CS 100">' +
             '<label><input type="checkbox" data-ignore-for="' + i + '"' + (r.ignore ? " checked" : "") + '> Ignore</label></div>';
    }).join("");
  }
  EL("wiz-map").addEventListener("input", function (e) {
    var f = e.target.getAttribute("data-course-for");
    if (f !== null) { WIZ.map[Number(f)].course = e.target.value.trim(); }
  });
  EL("wiz-map").addEventListener("change", function (e) {
    var g = e.target.getAttribute("data-ignore-for");
    if (g !== null) { WIZ.map[Number(g)].ignore = e.target.checked; renderMapping(); }
  });

  // The personal calendar: same command, same validation, a different kind — and a different sentence,
  // because "courses" means nothing about somebody's own week.
  EL("wiz-cal-ics").addEventListener("change", function () {
    WIZ.cal = EL("wiz-cal-ics").value.trim();
    if (!WIZ.cal) { WIZ.calNote = ""; renderWizard(); return; }
    invoke("paste_calendar_link", { kind: "calendar_ics", url: WIZ.cal }).then(function (r) {
      // Zero is a connection, not a failure: an address that fetches and holds nothing is somebody
      // who has not put anything in their calendar yet (`validate_for`).
      WIZ.calNote = !r.ok ? r.error
        : (r.link.events === 0 ? "Connected — nothing on it yet." : "Found " + r.link.events + " things already on your calendar.");
      if (r.ok && r.note) { WIZ.calNote += " " + r.note; }
      renderWizard();
    }).catch(function () {});
  });

  // ---- R-OB-4: the school typeahead.
  //
  // **The page never holds the list and never fetches it.** `app/tauri.conf.json`'s CSP is
  // `connect-src ipc: http://ipc.localhost` with no `'self'`, so a `fetch` of a bundled asset is
  // refused before it reaches the asset protocol — and that file is the controller's outside C0's
  // three keys, so widening it would be a hand-off for a thing that needs none. `campus_search` is
  // Rust's, answers with ten rows, and the page holds ten rows however long the list gets.
  function schoolHits(q) {
    return invoke("campus_search", { query: q }).then(function (r) { return (r && r.hits) || []; }).catch(function () { return []; });
  }

  function renderSchoolHits(hits) {
    EL("wiz-school-hits").innerHTML = hits.map(function (r, i) {
      // [unitid, name, city, state]
      return '<div class="hit" data-school="' + i + '" tabindex="0">' + h(r[1]) +
             '<span class="meta"> &middot; ' + h(r[2]) + ", " + h(r[3]) + "</span></div>";
    }).join("");
    EL("wiz-school-hits").__hits = hits;
  }

  function pickSchool(r) {
    WIZ.campus = { unitid: String(r[0]), name: r[1], state: r[3], lms: "" };
    EL("wiz-school").value = r[1];
    EL("wiz-school-hits").innerHTML = "";
    EL("wiz-school-free").hidden = true;
    EL("wiz-school-picked").textContent = r[2] + ", " + r[3];
    WIZ.icsNote = "";
    // The timezone is a SUGGESTION from the state, and only into a field the student has not touched:
    // a typed value wins, and a state we do not know leaves the OS zone alone. Rust owns the table
    // (`scaffold::state_timezone`); the page only asks.
    invoke("timezone_for_state", { state: r[3] }).then(function (t) {
      if (t && t.ok && t.timezone && !WIZ.tzTouched) { WIZ.tz = t.timezone; EL("wiz-tz").value = t.timezone; }
    }).catch(function () {});
    renderWizard();
  }

  EL("wiz-school").addEventListener("input", function () {
    schoolHits(EL("wiz-school").value).then(renderSchoolHits);
  });
  EL("wiz-school-hits").addEventListener("click", function (e) {
    var hit = e.target.closest("[data-school]");
    if (hit) { pickSchool(EL("wiz-school-hits").__hits[Number(hit.getAttribute("data-school"))]); }
  });
  // Keyboard-selectable: a typeahead you can only click is a typeahead that fails the person typing.
  EL("wiz-school-hits").addEventListener("keydown", function (e) {
    var hit = e.target.closest("[data-school]");
    if (hit && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      pickSchool(EL("wiz-school-hits").__hits[Number(hit.getAttribute("data-school"))]);
    }
  });
  EL("wiz-school").addEventListener("keydown", function (e) {
    if (e.key === "ArrowDown") {
      var first = EL("wiz-school-hits").querySelector("[data-school]");
      if (first) { e.preventDefault(); first.focus(); }
    }
  });
  // 4,319 schools is not all of them: a new campus, a satellite, somewhere abroad. A name and a state
  // is enough to make a vault, and that school simply has no curated feeds.
  EL("wiz-school-none").addEventListener("click", function () {
    EL("wiz-school-free").hidden = false;
    EL("wiz-school-hits").innerHTML = "";
  });
  EL("wiz-school-free").addEventListener("input", function () {
    WIZ.campus = { unitid: "", name: EL("wiz-school-name").value.trim(), state: EL("wiz-school-state").value.trim().toUpperCase(), lms: "" };
    EL("wiz-school-picked").textContent = "";
  });
  // The two-button fallback for a school whose LMS nothing established — shown by the sign-in handler
  // when `open_lms_window` says it does not know where to go.
  EL("wiz-lms-kind").addEventListener("click", function (e) {
    var b = e.target.closest("[data-lms]");
    if (b) { WIZ.campus.lms = b.getAttribute("data-lms"); EL("wiz-lms-kind").hidden = true; renderWizard(); }
  });
```

- [ ] **Step 5: `console.js` — the settings rows and the report overlay.** In `openSettings`, fill the account row from `account_status`; and add:

```js
  function openReport() {
    EL("report").hidden = false;
    EL("report-note").textContent = "";
    EL("report-text").value = "Loading…";
    invoke("report_preview", { view: VIEW }).then(function (r) {
      EL("report-text").value = r.ok ? r.text : ("could not build the report: " + r.error);
    }).catch(function () { EL("report-text").value = "could not build the report"; });
  }
  window.KNOWLU_OPEN_REPORT = openReport;
  EL("report-cancel").addEventListener("click", function () { EL("report").hidden = true; });
  EL("set-report").addEventListener("click", openReport);
  EL("report-send").addEventListener("click", function () {
    EL("report-send").disabled = true;
    // Exactly what is on screen. Rebuilding it here would send something the user never read.
    invoke("report_send", { text: EL("report-text").value }).then(function (r) {
      EL("report-note").textContent = r.ok ? "Sent. Thank you." : r.error;
      EL("report-send").disabled = false;
      if (r.ok) { setTimeout(function () { EL("report").hidden = true; }, 1500); }
    }).catch(function () { EL("report-note").textContent = "could not send"; EL("report-send").disabled = false; });
  });
  EL("set-portal").addEventListener("click", function () { invoke("open_portal", {}).catch(function () {}); });
  // Two presses, because the second one deletes a folder full of somebody's work and their account
  // with it. The first press only reveals the second.
  EL("set-delete-1").addEventListener("click", function () {
    EL("set-delete-2").hidden = false;
    EL("set-delete-note").textContent = "This deletes your vault, your backups, your account and everything we hold. It cannot be undone.";
  });
  EL("set-delete-2").addEventListener("click", function () {
    EL("set-delete-2").disabled = true;
    invoke("delete_my_data", {}).then(function (r) {
      EL("set-delete-note").textContent = r.ok ? "Deleted. Knowlu will close." : r.error;
      EL("set-delete-2").disabled = !r.ok;
    }).catch(function () { EL("set-delete-note").textContent = "could not delete"; EL("set-delete-2").disabled = false; });
  });
```

…and extend the shots seam: `window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker, openSettings: openSettings, openReport: openReport };`

- [ ] **Step 6: `app/static/console.css`.** One rule for the textarea and one for the new nav width; nothing else changes:

```css
/* R-OB-4: ten hits, scrollable, keyboard-focusable — the list is 4,319 schools and the panel shows ten. */
.wiz-hits { max-height: 220px; overflow-y: auto; border: 1px solid var(--hair); }
.wiz-hits:empty { display: none; }
.wiz-hits .hit { padding: 6px 8px; cursor: pointer; }
.wiz-hits .hit:hover, .wiz-hits .hit:focus { background: var(--s2c); outline: none; }
#report-text { width: 100%; font: 12px/1.5 var(--mono); background: var(--s2c); color: var(--t2); border: 1px solid var(--hair); padding: 8px; resize: vertical; }
```

- [ ] **Step 6a: The one sentence that lives in two places — write it now, not in Task 19.** Step 1 rewrote `the_wizards_privacy_sentence_is_the_sites_privacy_sentence` to look for a sentence Task 19 writes; that test `.expect()`s the sentence out of `site/privacy.html` and **panics until it is there**, so Task 17 step 8 and Task 18 step 5 would both be red for three tasks. Write the sentence — and only the sentence — here, in both places. The rest of the prose stays Task 19's.

  In `site/privacy.html`, as **one `<p>` on one line** (the test reads the file line by line):

  ```html
  <p>Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared.</p>
  ```

  …and in `console.js`, replacing the old `PRIVACY` constant, **byte for byte the same string** (the test compares them, and it is the same promise made to someone who has not visited the site):

  ```js
  // **One string, two languages.** `account::UNREACHABLE` is the first clause of every transport
  // failure the Rust side emits, and this is the page's copy of it. They are pinned to each other by
  // `static_assets.rs::the_unreachable_clause_is_one_string_on_both_sides`, the same way `PRIVACY` is
  // pinned to the site's — because a guard that silently stops matching is worse than no guard.
  var UNREACHABLE = "the account service could not be reached";
  var PRIVACY = "Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared.";
  ```

  **A plain `'` in both, not `&rsquo;` in the HTML.** The test takes the `<p>`'s inner text verbatim and compares it to the `console.js` literal, so an entity on one side would have to be an entity on the other — and `EL("wiz-privacy").textContent = PRIVACY` would then print `Knowlu&rsquo;s` on the welcome panel. One apostrophe, both files, byte for byte. Run the test; if it fails, make the two strings identical rather than loosening the test. The old sentence ("Knowlu has no account and sends nothing anywhere") is **false from this release**: delete it from `console.js` in this step and from the rest of the site in Task 19.

- [ ] **Step 7: Run the tests.** `cargo test -p knowlu --test static_assets` → all green — every one of the eight step 2 left red: the five rewritten or new ones (`…nine_panels…`, `…no_lms_credential_field…`, `…gates_on_eighteen…`, `…issue_report_is_previewed…`, `…never_offers_a_local_model`), the amended `the_wizard_takes_its_default_folders_from_the_launch_state`, `the_logins_panel_maps_what_it_finds_to_a_course`, and `the_unreachable_clause_is_one_string_on_both_sides` — which is the sentence pin step 6a made passable. The file has **one test fewer** than before this task: `the_wizard_offers_local_judgment_without_doing_anything` is gone and `…never_offers_a_local_model` carries its pin, inverted.

- [ ] **Step 8: Run everything.** `cargo test -p knowlu` at 0 warnings.

- [ ] **Step 8a: Confirm the first slot runs at Finish — and do not re-invent it.** §4.2 step 7 and §11a both say the wizard's last click ends with today's page on screen, and **`main` already does it**: `scheduler::needs_first_run(vault)` is `!vault.join("state").join("today.md").exists()` — `state/today.md` is the page `rank` writes (`engine/src/cli.rs`); there is no root `today.md` — and `scheduler::spawn` fires one slot at console launch when that is true, under the tick's own guards (`scheduler: app` **and** this device). `finish_onboarding` relaunches into the console over the vault the wizard just created, `spawn` runs, `state/today.md` is absent, the slot fires on its own thread and `run_slot` serialises it against anything else. **Nothing in this task starts a run**, and adding one would double-fire.

  Assert the seam instead — append to `app/tests/scheduler.rs`:

```rust
/// §4.2 step 7, §11a: a vault the wizard just made has no `today.md`, so the console's own launch is
/// what fires the first slot — and a vault **adopted in place** (Task 18) already has one, so nothing
/// fires a second time on top of the work that is already there.
#[test]
fn a_new_vault_needs_a_first_run_and_an_adopted_one_does_not() {
    use knowlu::scheduler::needs_first_run;
    let v = scratch("firstrun");
    // `rank` writes `state/today.md` (cli.rs); a root `today.md` is nobody's file and must not count.
    let state = v.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let today = state.join("today.md");
    let _ = std::fs::remove_file(&today);
    assert!(needs_first_run(&v), "a vault with no state/today.md is owed its first slot");
    std::fs::write(v.join("today.md"), b"# not the engine's file\n").unwrap();
    assert!(needs_first_run(&v), "a root today.md is not the ranked page");
    std::fs::write(&today, b"# Today\n").unwrap();
    assert!(!needs_first_run(&v), "an adopted vault already has state/today.md and must not run again");
    let _ = std::fs::remove_dir_all(&v);
}
```

  Then `cargo test -p knowlu --test scheduler` → one more test than before this task, all green.

- [ ] **Step 9: Check H6's replacement against what you actually built.** `scripts/` is the controller's, so **do not edit the file** — but H6 carries the whole replacement (the two constants, the `FAKE` recorder, `check()`), and it was written against this task's markup and handlers. Read it beside `index.html` and `console.js` and confirm every id it drives exists and every command it fakes is one you invoke: `wiz-welcome`, `wiz-account`, `wiz-email`, `wiz-pw`, `wiz-18`, `wiz-terms`, `wiz-create`, `wiz-signin`, `wiz-sub-month`, `wiz-name`, `wiz-vault-path`, `wiz-calendars`, `wiz-school`, `wiz-school-hits`, `wiz-school-none`, `wiz-lms-kind`, `wiz-lms-open`, `wiz-lms-state`, `wiz-cal-ics`, `wiz-cal-note`, `wiz-google`, `wiz-courses`, `wiz-course-rows`, `wiz-course-add`, `wiz-zy-user`, `wiz-zy-pass`, `wiz-map`, `wiz-map-rows`, `wiz-gmail`, `wiz-slot1`, `wiz-summary`, `wiz-back`, `wiz-next`, `wiz-step`, `wiz-error`. Report any drift **as a correction to H6**, with the replacement line, in the stream's report; the controller applies H6 at merge and runs it once (`.wv\Scripts\python scripts/wizard-check.py` → `ok`, exit 0).

- [ ] **Step 10: Commit.** `app: the wizard of §4.2 — account, subscription, a name instead of a folder, a school sign-in instead of a paste, an honest Gmail step, and the issue-report preview (C1 Task 17)`.

---

### Task 18: An install that already exists is adopted in place

Spec §11a. Quinn's cut-day vault, and any friend's, must keep its folder, its settings, its Credential Manager entries and its profile id — and gain an account on its first launch after C1. **No folder question is asked of an existing profile either.**

**Files:**
- Modify: `app/src/account.rs`, `app/static/index.html`, `app/static/console.js`
- Test: `app/tests/account.rs`, `app/tests/static_assets.rs`

**Interfaces:**
- Consumes: `scaffold::write_cloud_yaml_if_absent` (Task 12); `account::{move_session, session_target, PENDING_TARGET, cloud_config, api_base, anon_key, load_session}`.
- Produces: `account::needs_account(vault) -> bool`, `account::attach_in(vault, profile_id, pending_target) -> Result<(), String>`, and the commands `account_status`, `attach_account`, `open_portal`, `delete_my_data` (with `open_checkout`, `entitlement_now`, `open_policy` and `open_in_browser` already from Task 10).

- [ ] **Step 1: Write the failing tests** — append to `app/tests/account.rs`:

```rust
#[test]
fn a_vault_from_before_c1_needs_an_account_and_gains_one_without_moving() {
    use knowlu::account::{attach_in, needs_account, save_session, Session, PENDING_TARGET};
    let root = std::env::temp_dir().join(format!("knowlu-adopt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let vault = root.join("Quinn Hall");
    // A pre-C1 vault: config/planning.yaml, tasks/, runners.yaml — and NO cloud.yaml.
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(vault.join("tasks")).unwrap();
    std::fs::write(vault.join("config").join("planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
    std::fs::write(vault.join("config").join("ingest.yaml"), "ics_url: 'https://lms.example.invalid/a.ics'\n").unwrap();
    let before = std::fs::read_to_string(vault.join("config").join("ingest.yaml")).unwrap();
    assert!(needs_account(&vault), "a vault with no cloud.yaml needs one");

    // The wizard-less upgrade signs in first, into the pending target, exactly as the wizard does.
    let target = format!("knowlu/test-adopt-{}/pending", std::process::id());
    save_session(&target, "acc-9", &Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 9, email: "a@example.invalid".into() }).unwrap();
    let profile_id = knowlu::profiles::id_for(&vault);
    attach_in(&vault, &profile_id, &target).expect("attach");

    assert!(!needs_account(&vault), "…and stops needing one");
    let cfg = knowlu::account::cloud_config(&vault).expect("cloud_config");
    assert_eq!(cfg.account_id, "acc-9");
    assert_eq!(cfg.session_credential_target, format!("knowlu/{profile_id}/session"));
    // Nothing else in the vault moved: the folder, the feed and the profile id are exactly as they
    // were. That is the whole ruling.
    assert_eq!(std::fs::read_to_string(vault.join("config").join("ingest.yaml")).unwrap(), before);
    assert_eq!(knowlu::profiles::id_for(&vault), profile_id);
    // The session moved onto the profile and left the pending target.
    let (id, _) = knowlu::account::load_session(&cfg.session_credential_target).expect("session");
    assert_eq!(id, "acc-9");
    assert!(knowlu::account::load_session(&target).is_err());
    // Adopting twice is refused **for the reason the test is named for** — the vault already has an
    // account — and not incidentally because the pending session has since been moved away. So the
    // second attempt is given a live pending session, and the error text is asserted.
    save_session(&target, "acc-9", &Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 9, email: "a@example.invalid".into() }).unwrap();
    let again = attach_in(&vault, &profile_id, &target).unwrap_err();
    assert!(again.contains("already has an account"), "{again}");
    let _ = knowlu::credentials::delete(&target);
    let _ = PENDING_TARGET;
    let _ = knowlu::credentials::delete(&cfg.session_credential_target);
    let _ = std::fs::remove_dir_all(&root);
}
```

…and to `app/tests/static_assets.rs`:

```rust
/// I11: *Delete my data* takes **this profile's** snapshots out of the shared backups root, never the
/// root. Two profiles on one machine share `%USERPROFILE%\Knowlu\Backups`, and the other one's only
/// other copy of their work is in there.
#[test]
fn deleting_my_data_leaves_another_profiles_snapshots_alone() {
    // The path arithmetic, driven directly: the command itself needs a `ConsoleState`, an `AppHandle`
    // and a live account, and none of the three is what this is about.
    let root = std::env::temp_dir().join(format!("knowlu-backups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mine = root.join("profile_1111111111").join("vault");
    let theirs = root.join("profile_2222222222").join("vault");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(mine.join("a.md"), "x").unwrap();
    std::fs::write(theirs.join("b.md"), "y").unwrap();
    // What `delete_my_data` does: the root JOINED with this profile's id, and nothing above it.
    std::fs::remove_dir_all(root.join("profile_1111111111")).unwrap();
    assert!(!root.join("profile_1111111111").exists());
    assert!(theirs.join("b.md").is_file(), "another profile's snapshots were destroyed");
    let _ = std::fs::remove_dir_all(&root);
}

/// Spec §11a: an install that predates the account is upgraded **in place**, in the console window,
/// over its own vault — so the console page carries the same account panels the wizard does, and it
/// asks no folder question at all.
#[test]
fn the_console_can_sign_an_existing_install_in_without_re_onboarding_it() {
    let html = read("index.html");
    assert!(html.contains("id=\"upgrade\""), "the upgrade overlay");
    for id in ["up-email", "up-pw", "up-18", "up-terms", "up-create", "up-signin", "up-subscribe", "up-later", "up-error"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "the upgrade overlay needs {id}");
    }
    let panel = html.split("id=\"upgrade\"").nth(1).and_then(|s| s.split("</aside>").next()).expect("the upgrade overlay");
    assert!(!panel.to_lowercase().contains("folder"), "an existing install is never asked about a folder");
    // **It must be dismissable**, exactly as `#report` is (`report-cancel`): spec §5.1 and D4 both
    // promise that a dead connection never hides today's page, and this is the first launch after C1
    // for every install that already exists.
    assert!(panel.contains("id=\"up-later\""), "the upgrade overlay needs a dismiss control");
    // …and its policy links must be the browser-opening kind, asserted **over this panel's markup**
    // rather than over the whole file: the wizard having them is not the same claim.
    assert_eq!(panel.matches("class=\"policy\"").count(), 2, "both policy links open in the browser");
    let js = read("console.js");
    let listener = js.split("EL(\"upgrade\").addEventListener(\"click\"").nth(1).and_then(|s| s.split("function finishUpgrade(").next()).expect("the upgrade listener");
    assert!(listener.contains("a.policy") && listener.contains("preventDefault()"), "the overlay's own listener must intercept them");
    assert!(js.contains("function maybeUpgrade("), "maybeUpgrade");
    assert!(js.contains("\"attach_account\""), "the upgrade ends by attaching the account to this vault");
    // …it only appears when the vault says it needs one, never on a healthy console…
    assert!(js.contains("s.needs_account"), "the overlay is gated on the account status's own flag");
    // …it stays down once dismissed, and stays down when the service cannot be reached at all.
    assert!(js.contains("UPGRADE_DISMISSED"), "the dismiss must survive the next state poll");
    assert!(js.contains("UPGRADE_UNREACHABLE") && js.contains("function upgradeUnreachable("),
        "a sign-in that cannot reach the service must stand the overlay down, not trap the user behind it");
    // …and the guard tests the clause the Rust side actually emits. It read `indexOf("could not be
    // reached") === 0` once, against an error whose first twenty characters are "the account service
    // ", so it could never fire — which is the failure mode a shared literal exists to prevent.
    assert!(js.contains("indexOf(UNREACHABLE) === 0"), "the guard must test the shared clause, not a fragment of it");
    // …and it is a side panel, not a modal: the console underneath stays usable, which is the whole
    // of D4's promise that a dead connection never hides today's page.
    assert!(html.contains("<aside class=\"setpanel\" id=\"upgrade\""), "the overlay is a setpanel, like #settings");
}
```

- [ ] **Step 2: Run and watch them fail.**

- [ ] **Step 3: The Rust half**, appended to `app/src/account.rs`:

```rust
/// A vault written before C1 has no `config/cloud.yaml`. Spec §11a: it is **adopted in place** —
/// the folder, the settings, the Credential Manager entries and the profile id all stay exactly
/// where they are, and the account is added to what is already there.
pub fn needs_account(vault: &Path) -> bool { cloud_config(vault).is_err() }

/// Write the account into a vault that already exists, and move the session onto its profile. The
/// two halves are one operation on purpose: a `cloud.yaml` naming a credential target that holds
/// nothing is a vault that cannot reach the cloud with nothing anywhere saying why.
pub fn attach_in(vault: &Path, profile_id: &str, pending_target: &str) -> Result<(), String> {
    let (account_id, _) = load_session(pending_target)?;
    // **Every field, because `VaultPlan` has grown**: Tasks 14a and 14b added `zybooks_courses`,
    // `vhl_sections`, `course_map` and `courses`, and Task 14c added `campus_choice`. An adopted
    // vault gains `config/cloud.yaml` and nothing else — its feeds, its mappings, its courses and its
    // campus are already on disk and are not rewritten — so every plan field but the three cloud ones
    // is empty by construction, and `write_cloud_yaml_if_absent` is the only writer this calls.
    let plan = crate::scaffold::VaultPlan {
        profile_id: profile_id.to_string(),
        ics_url: None,
        personal_calendar: None,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        timezone: String::new(),
        slots: Vec::new(),
        device: String::new(),
        campus: "none".to_string(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        api_base: api_base(),
        anon_key: anon_key(),
        account_id,
    };
    // Only `config/cloud.yaml` is written. Nothing else in this vault is read, rewritten or moved.
    crate::scaffold::write_cloud_yaml_if_absent(vault, &plan)?;
    move_session(pending_target, &session_target(profile_id))?;

    // **The back-fill** (Interfaces with C2, item 3). This vault was onboarded before the account
    // existed, so its LMS feed is in `config/ingest.yaml` and in no `sources` row — and C2's
    // `/ingest/ics` reads that row. Send it once, now that there is an account to send it to.
    //
    // **Best effort, and never fatal**: the adoption has already succeeded, the vault copy is what
    // `ingest` reads until C2 ships, and an unsubscribed or offline account must not leave a
    // half-adopted install behind. A failure is one logged line.
    for (kind, url) in feeds_in(vault) {
        if let Err(e) = crate::lms_link::store_source(&api_base(), &session_target(profile_id), kind, &url) {
            eprintln!("Knowlu: the {kind} link could not be saved to your account ({e})");
        }
    }
    Ok(())
}

/// Both feeds `config/ingest.yaml` may already hold: `ics_url` (the school) and the first entry of
/// `calendars:` (the personal one). Read the way `scheduler::ics_state` reads the same file —
/// `serde_yaml_ng` over `pystr`, never a byte compare.
fn feeds_in(vault: &Path) -> Vec<(&'static str, String)> {
    let Ok(text) = knowlu_engine::pystr::read_text(&vault.join("config").join("ingest.yaml")) else { return Vec::new() };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return Vec::new() };
    let mut out = Vec::new();
    if let Some(u) = v.get("ics_url").and_then(|u| u.as_str()).filter(|u| !u.trim().is_empty()) {
        out.push(("lms_ics", u.to_string()));
    }
    if let Some(list) = v.get("calendars").and_then(|c| c.as_sequence()) {
        if let Some(u) = list.iter().find_map(|f| f.get("ics_url").and_then(|u| u.as_str())).filter(|u| !u.trim().is_empty()) {
            out.push(("calendar_ics", u.to_string()));
        }
    }
    out
}

#[tauri::command(async)]
pub fn attach_account(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let profile_id = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    match attach_in(&cs.vault, &profile_id, PENDING_TARGET) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// What the settings panel and the upgrade overlay both read. `needs_account` is what puts the
/// overlay on screen at all.
#[tauri::command]
pub fn account_status(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let cfg = cloud_config(&cs.vault).ok();
    let cache = load_cache(&cs.data_dir);
    let email = cfg
        .as_ref()
        .and_then(|c| load_session(&c.session_credential_target).ok())
        .map(|(_, s)| s.email)
        .unwrap_or_default();
    // **No network call here.** `account_status` answers from this machine only — a console that had
    // to reach the internet before it could decide whether to cover today's page would be the bug the
    // 72-hour grace exists to prevent (spec §5.1, D4). Whether the service is reachable is a fact the
    // page learns from an attempt that failed, and it is the page that stands the overlay down.
    json!({
        "ok": true, "error": Value::Null,
        "needs_account": cfg.is_none(),
        "account_id": cfg.as_ref().map(|c| c.account_id.clone()),
        "email": email,
        "status": cache.as_ref().map(|c| c.status.clone()),
        "plan": cache.as_ref().and_then(|c| c.plan.clone()),
        "current_period_end": cache.as_ref().and_then(|c| c.current_period_end.clone()),
        "checked_at": cache.as_ref().map(|c| c.checked_at.clone()),
    })
}

/// The wizard's poll (`entitlement_now`) reads the PENDING session, because the wizard has no vault
/// yet; the upgrade overlay uses the same one for the same reason.
#[tauri::command(async)]
pub fn entitlement_now() -> Value {
    let base = api_base();
    let out = (|| -> Result<EntitlementCache, String> {
        let auth = auth_base(&base)?;
        let token = valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, now_unix())?;
        fetch_entitlement_at(&base, &token)
    })();
    match out {
        Ok(c) => json!({ "ok": true, "error": Value::Null, "status": c.status, "plan": c.plan, "current_period_end": c.current_period_end }),
        Err(e) => json!({ "ok": false, "error": e, "status": Value::Null }),
    }
}

// `open_in_browser` is already in this file — Task 10 added it for `open_policy`, and Checkout and
// the Portal use the same one rather than a second copy of the same three lines.

fn post_for_url(api_base: &str, path: &str, token: &str, body: &Value) -> Result<String, String> {
    check_api_base(api_base)?;
    let mut res = agent()
        .post(&format!("{}{path}", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .send_json(body)
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    v.get("url").and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| "no link came back".to_string())
}

#[tauri::command(async)]
pub fn open_checkout(plan: String) -> Value {
    let base = api_base();
    let out = (|| -> Result<String, String> {
        let auth = auth_base(&base)?;
        let token = valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, now_unix())?;
        let url = post_for_url(&base, "/billing-checkout", &token, &json!({ "plan": plan, "terms_version": TOS_VERSION }))?;
        open_in_browser(&url)?;
        Ok(url)
    })();
    match out { Ok(_) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e }) }
}

/// *Manage subscription* — one click to Stripe's portal, one click to cancel there, no survey in
/// between. That is the whole of the cancel flow the legal note (§8) asks for.
#[tauri::command(async)]
pub fn open_portal(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let out = (|| -> Result<String, String> {
        let cfg = cloud_config(&cs.vault)?;
        let auth = auth_base(&cfg.api_base)?;
        let token = valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix())?;
        let url = post_for_url(&cfg.api_base, "/billing-portal", &token, &json!({}))?;
        open_in_browser(&url)?;
        Ok(url)
    })();
    match out { Ok(_) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e }) }
}

/// Spec §4.1: *Delete my data* removes the vault, the snapshots, the app data **and** calls
/// `DELETE /account`. The server call goes FIRST: a local wipe that ran before it would leave an
/// account nobody can reach to delete, and the deletion right is the one that matters here.
#[tauri::command(async)]
pub fn delete_my_data(app: tauri::AppHandle, cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let cfg = match cloud_config(&cs.vault) { Ok(c) => c, Err(e) => return json!({ "ok": false, "error": e }) };
    let auth = match auth_base(&cfg.api_base) { Ok(a) => a, Err(e) => return json!({ "ok": false, "error": e }) };
    let token = match valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix()) { Ok(t) => t, Err(e) => return json!({ "ok": false, "error": e }) };
    let res = agent()
        .delete(&format!("{}/account", cfg.api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .call();
    if let Err(e) = res { return json!({ "ok": false, "error": format!("your account could not be deleted ({e}) — nothing on this machine was touched") }); }
    let (backup, profile_id) = cs.settings.lock().map(|s| (s.backup_dir.clone(), s.profile_id.clone())).unwrap_or((None, String::new()));
    let _ = std::fs::remove_dir_all(&cs.vault);
    // **This profile's snapshots, not the whole backups folder.** `settings.backup_dir` is the ROOT —
    // `%USERPROFILE%\Knowlu\Backups` for every profile on the machine — and `backup::tick` writes
    // `<root>\<profile_id>\vault` inside it. Removing the root would destroy a housemate's or a second
    // profile's only other copy of their work, which is not what "delete MY data" says.
    if let (Some(b), false) = (backup, profile_id.is_empty()) {
        let _ = std::fs::remove_dir_all(b.join(&profile_id));
    }
    let _ = std::fs::remove_dir_all(&cs.data_dir);
    // …and forget the profile, or the picker goes on offering a vault that is not there.
    if let Some(root) = crate::state::app_data_root() {
        if let Ok(all) = crate::profiles::load(&root) {
            let left: Vec<_> = all.into_iter().filter(|pr| pr.id != profile_id).collect();
            let _ = crate::profiles::save(&root, &left);
        }
    }
    let _ = crate::credentials::delete(&cfg.session_credential_target);
    // The coursework logins are keyed to the PROFILE, not the account (`credentials::target_for`),
    // and they are this machine's — deleting the account does not delete them, so this does.
    for source in ["zybooks", "vhl"] {
        let t = crate::credentials::target_for(&profile_id, source);
        if crate::credentials::exists(&t) { let _ = crate::credentials::delete(&t); }
    }
    // The envelope goes back first and the process ends a moment later, on another thread: a page
    // whose `.then` never runs cannot say "Deleted", and the settings row's own copy promises it will.
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        h.exit(0);
    });
    json!({ "ok": true, "error": Value::Null })
}
```

- [ ] **Step 4: The page half.** In `index.html`, beside `#report`:

```html
<aside class="setpanel" id="upgrade" hidden>
  <div class="set-hd"><h2>Knowlu now needs an account</h2><button class="b" id="up-later">Not now</button></div>
  <p class="meta">Your work stays exactly where it is. Sign in, or make an account, and Knowlu picks up where it left off. Today&rsquo;s page is underneath and still works &mdash; ranking never needed an account.</p>
  <div class="set-row"><input type="text" id="up-email" placeholder="Email"><input type="password" id="up-pw" placeholder="Password"></div>
  <label><input type="checkbox" id="up-18"> I am 18 or older</label>
  <label><input type="checkbox" id="up-terms"> I accept the <a href="terms.html" class="policy" data-policy="terms">terms</a> and the <a href="privacy.html" class="policy" data-policy="privacy">privacy policy</a></label>
  <div class="set-row"><button class="b pri y" id="up-create">Create account</button><button class="b" id="up-signin">I already have one</button></div>
  <div class="set-row"><button class="b pri y" id="up-subscribe" hidden>Subscribe</button><span class="crit" id="up-error"></span></div>
</aside>
```

…and in `console.js`, called from the state poll:

```js
  // Spec §11a: an install made before C1 is upgraded here, in place. It is never re-onboarded, it is
  // never asked where its folder is, and nothing about it moves — the overlay ends in
  // `attach_account`, which writes `config/cloud.yaml` beside the config files that are already there.
  //
  // **Two things it must never do**, and both were real: cover today's page with no way past, and
  // cover it at all when the account service cannot be reached. Spec §5.1 and D4 both promise that a
  // dead connection never hides today's page — the scheduler's 72-hour grace exists for exactly this
  // — and an overlay that walks past that promise is worse than no overlay. So: *Not now* dismisses
  // it for the session, and an unreachable service does not raise it in the first place. It comes
  // back on the next launch, which is the right cadence for a thing that has to happen once.
  // `#upgrade` is a `setpanel`, the same side panel `#settings` is — it does not cover the page, and
  // the console under it keeps ranking, scrolling and being clicked. These two flags are the rest of
  // the promise: `UPGRADE_DISMISSED` is *Not now*, and `UPGRADE_UNREACHABLE` is what a failed attempt
  // sets, so a student on a train is asked once and then left alone until the next launch.
  var UPGRADE_DISMISSED = false;
  var UPGRADE_UNREACHABLE = false;
  function maybeUpgrade(s) {
    if (UPGRADE_DISMISSED || UPGRADE_UNREACHABLE || !s || !s.needs_account) { EL("upgrade").hidden = true; return; }
    EL("upgrade").hidden = false;
  }
  EL("up-later").addEventListener("click", function () {
    UPGRADE_DISMISSED = true;
    EL("upgrade").hidden = true;
  });
  /** A sign-in that could not reach the service at all stands the overlay down for this session and
   *  says why once. Not an error dialog: there is nothing the user can do about a dead network, and
   *  today's page is right there underneath. */
  function upgradeUnreachable() {
    UPGRADE_UNREACHABLE = true;
    EL("up-error").textContent = "";
    EL("upgrade").hidden = true;
  }
  EL("upgrade").addEventListener("click", function (e) {
    // The same branch the wizard has, and it matters more here: this window has a working console to
    // lose, and a plain navigation to `terms.html` would lose it while the user is ticking the box
    // that says they accept it.
    var policy = e.target.closest("a.policy");
    if (policy) {
      e.preventDefault();
      invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
      return;
    }
    var creating = !!e.target.closest("#up-create");
    if (creating || e.target.closest("#up-signin")) {
      if (creating && !(EL("up-18").checked && EL("up-terms").checked)) {
        EL("up-error").textContent = "Tick both boxes to create an account."; return;
      }
      var args = { email: EL("up-email").value.trim(), password: EL("up-pw").value };
      if (creating) { args.age_attested = EL("up-18").checked; }
      invoke(creating ? "sign_up" : "sign_in", args).then(function (r) {
        EL("up-pw").value = "";
        // A dead network is not something to hold someone behind a panel for; a wrong password is.
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        if (!r.ok) { EL("up-error").textContent = r.error; return; }
        EL("up-error").textContent = "";
        EL("up-subscribe").hidden = false;
        return invoke("entitlement_now", {}).then(function (ent) {
          if (ent.ok && (ent.status === "active" || ent.status === "trialing")) { return finishUpgrade(); }
        });
      }).catch(upgradeUnreachable);
      return;
    }
    if (e.target.closest("#up-subscribe")) {
      invoke("open_checkout", { plan: "monthly" }).then(function () {
        var tries = 0;
        var tick = function () {
          tries += 1;
          invoke("entitlement_now", {}).then(function (ent) {
            if (ent.ok && (ent.status === "active" || ent.status === "trialing")) { return finishUpgrade(); }
            if (tries < 40) { setTimeout(tick, 3000); }
          }).catch(function () { if (tries < 40) { setTimeout(tick, 3000); } });
        };
        setTimeout(tick, 3000);
      }).catch(function () {});
    }
  });
  function finishUpgrade() {
    return invoke("attach_account", {}).then(function (r) {
      if (!r.ok) { EL("up-error").textContent = r.error; return; }
      EL("upgrade").hidden = true;
      refresh();
    });
  }
```

…and one line where the state envelope is rendered: `maybeUpgrade(env.state.topline.account || {});` — the topline gains nothing in Rust; instead the console's boot calls `invoke("account_status", {}).then(maybeUpgrade)` once and again after `attach_account`. Use the second form, which needs no engine change:

```js
  // A failed call is treated as "not reachable", not as "no account": the overlay stays down and the
  // console is exactly as usable as it was before C1.
  function checkAccount() {
    invoke("account_status", {}).then(maybeUpgrade).catch(function () { EL("upgrade").hidden = true; });
  }
```

…called from `bootConsole()` and from `openSettings()` (which also fills `#set-account-state` from the same reply).

- [ ] **Step 5: Run everything.** `cargo test -p knowlu` at 0 warnings; `static_assets` green, `account` at 12 passed (10 from Task 11, plus this task's adopt-in-place and shared-backups-root tests).

- [ ] **Step 6: A real upgrade, on a scratch vault.** `.\scripts\scratch-vault.ps1 -Source engine\tests\fixtures\vault-full`, launch against it, and confirm: the overlay appears; sign-in works against staging (`KNOWLU_API_BASE` set to the staging functions URL); after Checkout in test mode the overlay closes; `config/cloud.yaml` now exists in the scratch vault and nothing else in it changed (`git status` in a scratch copy, or a before/after directory listing with sizes); and — §11a — **no slot fires on top of the work that is already there** — write one line into the scratch copy's `state\today.md` before launching (the fixture vault carries no ranked page at either path, and without one the first-slot rule fires, correctly), so `scheduler::needs_first_run` is false. Record the result in this task. **Never against a real vault.**

- [ ] **Step 7: Commit.** `app: an install from before C1 is adopted in place — sign in, subscribe, and config/cloud.yaml lands beside the config files that were already there (C1 Task 18)`.

---
### Task 19: The privacy policy and the terms — **the one deliberate exception to "no placeholders"**

> **This task is the plan's single exception to the no-placeholders rule, and it is deliberate.** The other twenty tasks hand an implementer exact code because code has one correct form. A privacy policy does not: it is prose that has to be true about *this* product on the day it is published, and a plan that dictated its sentences would be a plan that went stale the first time a section of §6 moved. So this task specifies **every section, and every sentence the law requires to be in it**, verbatim where the wording is load-bearing — and then instructs the implementer to draft the full text from `docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`'s "cheapest compliant path" rows (§0's table, §2, §3, §7, §8, §9) plus this plan's own §6 behaviour. **Quinn reads it before it is published (P5); a lawyer reads it before the first non-founder paid sign-up.**

**Files:**
- Rewrite: `site/privacy.html`
- Create: `site/terms.html`, `site/subscribed.html`, `site/signed-in.html`
- Modify: `site/site.css` (only what these two pages need), `app/static/console.js` (the `PRIVACY` constant)
- Test: `app/tests/static_assets.rs` (the existing sentence pin)

- [ ] **Step 1: The sentence is already there — build the page around it.** Task 17 step 6a wrote the one sentence into `site/privacy.html` and `console.js`, because the test that pins them together would otherwise have been red for three tasks. It is:

> Your vault stays on this machine. Knowlu's servers hold your account, the judgments they make for you, and what you correct; they never hold the text of your notes, and nothing here is ever sold or shared.

**Do not reword it here** without re-running `cargo test -p knowlu --test static_assets the_wizards_privacy_sentence_is_the_sites_privacy_sentence`; it is one string in two files. The old sentence ("Knowlu has no account and sends nothing anywhere") is **false from this release** and must not survive anywhere on the site — Task 20 step 2 greps for it.

- [ ] **Step 2: `site/privacy.html` — every section, and the sentences that are not the drafter's to choose.** Draft the prose; the following are requirements, not suggestions.

1. **Effective date and version.** The page carries an effective date, and it is the same string as `account::PRIVACY_VERSION` (`2026-09-10` unless Task 10 changed it). CalOPPA § 22575(b)(5) requires the effective date; the version is what the consent log points at.
2. **Who we are, and how to reach us.** A named contact — an address a person answers. Alabama's breach statute presumes one; CalOPPA requires a way to ask questions.
3. **What we collect, by category, and why.** Account (email, password hash held by Supabase, the 18+ attestation as a boolean and a timestamp, the terms and privacy versions accepted); billing (handled by Stripe — *we never see a card number*); the LMS calendar URL, encrypted; coursework logins (**on your machine only, in Windows Credential Manager, never on our servers**); interaction events and corrections (§6 (a) and (b)); diagnostic logs, only when you send an issue report; Gmail message metadata and text **held in memory for the length of one judgment and then discarded** (this sentence is Google's Limited Use, in English).
4. **What we do not collect.** No birthdate. No card number. No campus password — *the sentence must say, in so many words, that Knowlu never asks for a university sign-in and that when you sign in to your school it happens on the school's own page.* No note bodies, unless a future opt-in that does not exist yet is turned on.
5. **Who it goes to.** Supabase (hosting and the database), Stripe (payments), Cloudflare (the website and the downloads), and **our inference provider — the only place any content of yours is ever sent, and only to answer one question about one item**. Named, not gestured at.
6. **"We do not sell or share your personal information."** Stated plainly, and true. This is what makes an opt-out link and Global Privacy Control handling unnecessary at this size (legal note §3).
7. **Gmail, in its own section.** The Limited Use disclosure: Gmail data is used only to propose tasks in Knowlu; it is transferred only to the inference provider to provide that feature; **no human reads it without your explicit consent, and the issue-report preview screen is where you give that consent, message by message**; it is **never used to train a model beyond your own account's rules**; attachments are never fetched. Say the app is in Google's testing mode today and what that means for a test user (100 users, re-connect weekly).
8. **How long we keep it.** Account data until you delete it; a `(hashed email, date)` tombstone for **90 days** after deletion, for abuse control; issue reports **90 days**; the subscription consent record **three years**, because California's law requires it, and stripped of your account id once you delete (legal note §8).
9. **Your rights, for everyone, in every state.** Access, export, correct, delete — and *how*: **Settings → Delete my data** removes the vault, the backups, the app data and the account; the export is `GET /account/export`, reachable from the same panel. No thresholds, no geo-fencing (legal note §3).
10. **The 18+ gate.** Knowlu is for people 18 or older; an account requires the attestation; an under-18 is declined with a reason and no data is kept.
11. **Security, and what happens if it fails.** Encryption in transit and at rest; the LMS URL encrypted with a key our database does not hold; **a named person responsible**, and a commitment to notify affected users **within 45 days** of determining a breach (Ala. Code § 8-38-5's deadline, promised to everyone rather than to Alabamians alone).
12. **Do Not Track.** CalOPPA § 22575(b)(5) requires the page to say how it responds. Knowlu is a desktop app with no third-party tracking, so the honest sentence is that there is nothing to track across sites and no third party collects anything through Knowlu.
13. **Changes.** How a material change is notified, and that continued use after a version bump is not how consent is taken for a materially different use.

- [ ] **Step 3: `site/terms.html` — every section, and the sentences the subscription laws require.** Draft the prose; these are requirements:

1. **What Knowlu is, and what it is not.** A tool that ranks work you already have. It reads; it does not submit anything to your school for you.
2. **Eligibility: 18 or older**, and the attestation is a term of the contract (legal note §2; Ala. Code § 26-1-1(f) is what makes an 18-year-old's agreement binding).
3. **The subscription, in the sentence that is most of the law**: *"$9.99 per month, renewing monthly until you cancel; or $69.99 per academic year. Seven days free first, with your card taken at sign-up and not charged until the trial ends. Cancel any time in Settings."* (Legal note §8's "Copy" row, adapted for the trial R2 added.)
4. **The summer pause**, in plain words: billing stops for June, July and August and starts again on 1 September, **and we email you before the first charge comes back**. A pause nobody was told was ending is the failure mode this whole paragraph exists to prevent.
5. **How to cancel:** one link, in the app, to Stripe's portal — *"no survey, no phone call, no email required"* — and the same link in every billing email. ROSCA's "simple mechanisms", California's "no more difficult than signing up".
6. **The annual reminder** we send, and what it will say.
7. **Refunds**, stated: a policy that exists is worth more than a generous one that does not. A minor who disaffirms is refunded (legal note §2).
8. **Your data is yours.** The vault is a folder on your machine; you may copy it, edit it and leave with it; deleting your account does not delete a copy you have taken.
9. **What we ask of you:** don't share an account; don't use Knowlu to break your school's rules; the coursework portals have their own terms and **connecting a portal login is your decision about your own account** — say plainly that the vendors' terms discourage automated access and that Knowlu fetches at human rates, on your machine, with your credentials never leaving it (legal note §6; D11).
10. **Availability and limits.** No promise of uptime; the ranking works offline; the cloud steps do not.
11. **Termination**, by either side, and what happens to the data on each path.
12. **The usual**: warranty disclaimer, limitation of liability, governing law (Alabama), how disputes are handled — with a note in the draft to Quinn that **a lawyer should look at the arbitration and limitation clauses specifically**, because they are the ones that do not bind a minor and the ones a court reads hardest.
13. **Effective date and version**, matching `account::TOS_VERSION`.

- [ ] **Step 3a: The two pages Checkout and Auth land on.** Both are already referenced and neither exists: `billing-checkout`'s `success_url` is `https://knowlu.com/subscribed.html` (Task 4) and `config.toml`'s `additional_redirect_urls` is `https://knowlu.com/signed-in.html` (Task 0). Without them, **every paying user lands on a 404 the moment they finish paying**, and so does every confirmation link. Two small pages, same `site.css`:

  `site/subscribed.html` — `<title>Knowlu — you're subscribed</title>`, one heading and two sentences: *"You're subscribed. Go back to Knowlu — it is already checking, and the wizard will move on by itself within a few seconds."* and a line saying the card is not charged until the 7-day trial ends, with a link back to `index.html`.

  `site/signed-in.html` — `<title>Knowlu — you're signed in</title>`, one heading and: *"Your email is confirmed. Go back to Knowlu and sign in."* **and**, in its own paragraph, the sentence the magic-link flow depends on: *"If you asked for a sign-in link, the same email has a six-digit code in it. Type that code into Knowlu — a desktop app cannot follow a link that opens in your browser."*

- [ ] **Step 4: `site/site.css`.** Whatever the four pages need — a `h2 + p` rhythm, a definition list, and nothing else. Do not restyle `index.html`.

- [ ] **Step 5: Run the pin.** `cargo test -p knowlu --test static_assets the_wizards_privacy_sentence_is_the_sites_privacy_sentence` → 1 passed. If it fails, the two copies differ by a character — fix the copy, never the test.

- [ ] **Step 6: Read them out loud, then give them to Quinn (P5).** Send both, with this context: *"These are drafted from the legal briefing's cheapest-compliant-path rows, not from a template. Two things need your eye before anything is published: the 18+ gate — we decline under-18s rather than collect a birthdate, which is the cheap path but it does cost us the 17-year-old freshmen for a semester — and the cancel flow, which is one link to Stripe's portal with no survey in front of it. A lawyer should see both before the first person who is not you pays."*

- [ ] **Step 7: Commit.** `site: the privacy policy and the terms — one policy, universal rights, no sale, the 18+ gate, the cancel flow, Gmail's Limited Use, and a 45-day breach promise (C1 Task 19)`.

---

### Task 20: The download page's copy, and the Google verification packet

**Precondition P4** (Quinn submits the consent screen). Everything before step 5 is ours.

**Files:**
- Modify: `site/index.html`

- [ ] **Step 1: `site/index.html`.** Three changes, and no others.

The data paragraph becomes the one sentence from Task 19 step 1 (the same string, third copy — the site's landing page and its privacy page agree by eye; only the privacy page's copy is pinned by a test).

The price and the account, where the download button is:

```html
  <a class="dl" href="releases/Knowlu-setup.exe">Download for Windows</a>
  <p class="meta">Windows 10 or 11, 64-bit. $9.99 a month, or $69.99 for the academic year &mdash; seven days free first, and you can cancel any time from inside the app. Knowlu needs an account.</p>
```

The footer gains the terms:

```html
  <footer>Knowlu &middot; <a href="privacy.html">Privacy</a> &middot; <a href="terms.html">Terms</a></footer>
```

Delete the sentence "Everything stays on this machine. Knowlu has no account and sends nothing anywhere…" wherever it still appears, and the *Copy diagnostics* paragraph's claim that a report carries "no note content" — Task 16's report carries scrubbed log lines, which is a different and truer sentence: *"Report an issue in the tray shows you the whole report before it is sent, with emails, links and keys already replaced, and lets you edit it."*

- [ ] **Step 2: Search the site for the old promise.** `Select-String -Path site\*.html -Pattern "no account", "sends nothing anywhere"` → no matches. If either sentence is still there, the site is telling a new user something that stopped being true in this release.

- [ ] **Step 3: The `gmail.readonly` justification, for Quinn to paste.** Google's form wants one paragraph per scope saying what the app does with it and why a narrower scope will not do. Give him exactly this:

> Knowlu is a desktop planner for university students. It reads the student's own inbox, twice a day, to find messages that contain an obligation — an assignment change, a meeting request, a deadline from an instructor — and proposes them as tasks the student approves or declines inside Knowlu. Message text is read on our server, used for that one classification, and discarded; only the message id, the classification and the fields the student approved are stored. Attachments are never fetched, nothing is ever sent on the student's behalf, and Gmail-derived content is excluded from any model improvement. `gmail.readonly` is the narrowest scope that permits this: `gmail.metadata` is also restricted and carries no body, so it cannot tell an assignment change from a newsletter, and the label scopes give no message access at all.

- [ ] **Step 4: The demo video's shot list, for Quinn to record.** Google wants an unlisted video showing the consent screen and the data's use, on the actual app, with the OAuth client id visible. Two to three minutes:

1. The site at `knowlu.com`, then the privacy policy, showing the Gmail section.
2. The installed app, the wizard's Gmail step, and the sentence explaining testing mode.
3. Pressing Connect: the Google consent screen, **with the app name and the requested scope on screen**, and the browser's address bar showing the client id in the URL.
4. Back in the app: a proposal card that came from an email, with the source visible on the card.
5. The Decisions deck: approving one, declining another.
6. Settings → the Gmail row → **Disconnect**, and the sentence saying the token is revoked at Google and the row deleted.
7. Settings → Delete my data, and the confirmation text.

- [ ] **Step 5: Ask Quinn to submit (P4).** Context: *"This is the long pole in the whole product. The consent screen needs `knowlu.com` verified as an authorised domain, the homepage and the privacy URL — both live now — the scope justification I have written for you, and a demo video to the shot list I have written. Submit it for restricted-scope verification today even though C2 builds the reader, because the review is weeks and the CASA assessment after it is annual. Until it lands, Gmail works for at most 100 named test users whose tokens expire every seven days, and the wizard says exactly that."*

- [ ] **Step 6: Record the submission.** Add to this task: `Submitted <date>; Google case <id>; testing-mode test users: <n>.`

- [ ] **Step 7: Commit.** `site: the download page tells the truth about the price, the account and what a report carries; the Google verification packet is with Quinn (C1 Task 20)`.

---

### Task 21: Close — the gate, the recount, and the controller's merge list

- [ ] **Step 1: The full gate, from the repository root.**

```powershell
cargo build --workspace
cargo test --workspace
deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/
deno lint --config cloud/supabase/deno.json cloud/supabase/
deno fmt --check --config cloud/supabase/deno.json cloud/supabase/
git ls-files --eol cloud site app | Select-String -Pattern "i/lf" -NotMatch
```

Expected: the workspace green at **0 warnings** with only the `.rsrc merge failure: multiple non-default manifests` line; every `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` this stream added reported as `i/lf`; the Deno suite green.

- [ ] **Step 2: Recount the commands, by hand, from `main.rs`.** After the controller applies H3 the two `generate_handler!` lists must contain **27** and **43** names, **59** distinct (26 `commands.rs` + 14 `onboarding.rs` + 12 `account.rs` + 5 `lms_link.rs` + 2 `report.rs`; the overlap between the two lists is 11). Count them; do not trust this plan's arithmetic. Whatever the real numbers are, they are what H5 and H8 must say.

- [ ] **Step 3: Prove the disjointness the merge depends on.** `git diff --name-only main...c1-accounts` and check every path against the ownership list in *Global Constraints*. **An overlap with C0's or C2's files is a stop, not a rebase.** Expected: only `cloud/supabase/**`, `app/src/{onboarding,scaffold,scheduler,account,lms_link,telemetry,report}.rs`, `app/static/**`, `app/tests/**`, `site/**`, and this plan.

- [ ] **Step 4: Hand the controller the merge list.** In the stream's report, list: H3 (the two handler lists and the `use` line), H4 (the two tray items), H5 (`app/README.md`'s recount), H6 (`scripts/wizard-check.py` — the replacement is written out in H6 itself; Task 17 step 9 records any drift from it), H7 (`ci.yml`'s `cloud` job), H8 (`CLAUDE.md`'s two edits) — and note that **H1 and H9 were applied earlier** and need nothing at merge.

- [ ] **Step 5: `HANDOFF.md` is the controller's**, so give them the block to paste rather than writing it:

```markdown
▶ **C1 DONE <date>:** accounts, entitlement and the new wizard. Supabase `knowlu-staging` and
`knowlu-prod` are live; Stripe is in test mode with the monthly and academic-year prices, the 7-day
trial and the June–August pause; `GET /entitlement` is cached on the device with a 72-hour grace and
a slot with no subscription says `judge (skipped: no entitlement)` and stays green. The wizard is
nine panels with no folder question; an install from before C1 is adopted in place. Telemetry (a) and
(b) go up at each slot; issue reports are previewed and scrubbed before they are sent. The privacy
policy and the terms are published. **Google's restricted-scope verification was submitted <date>,
case <id>** — Gmail is testing-mode only until it lands. LMS link capture: outcome **<A|B|C>** from
the Task 13 spike (<one sentence>).
```

- [ ] **Step 6: This plan's status line becomes `DONE <date>`, with the Task 13 outcome letter and the two live project refs' names (not their keys).**

- [ ] **Step 7: Commit.** `docs: C1 closed — accounts, entitlement, the wizard, telemetry and issue reports (C1 Task 21)`.

---

## Exit gate

1. A person who has never run Knowlu can install it, create an account, attest to being 18 or older, accept both policies, start a 7-day trial with a card, **connect both calendars on one panel** — their school by signing in to it, their own by its secret iCal address — add coursework logins, pick two slots, and finish; **with no folder question anywhere**; and the first slot runs on its own (`scheduler::needs_first_run`) with today's page on screen. (VISION success criterion 5; spec §4.2, §11a.)
2. **The personal calendar is in all three places it has to be**: a `sources` row of kind `calendar_ics` on the account, the vault's `config/ingest.yaml` `calendars:` list, and — after that first slot — busy time on today's page. The empty `calendars: []` a fresh install used to carry is gone.
3. An install made **before** C1 — the founder's cut-day vault, or a friend's — launches into the console, is asked to sign in and subscribe, and carries on with its folder, its settings, its Credential Manager entries and its profile id untouched, gaining exactly one file: `config/cloud.yaml`. (Spec §11a.)
4. **Both calendar links reach the account.** After a wizard run against staging, `select kind, added_at from sources` has one row per calendar the user connected — `lms_ics`, `calendar_ics`, or both — and `select url_ciphertext from sources` is not the URL. An adopted vault (item 3) back-fills the same rows out of `config/ingest.yaml` on its first upgrade. Without this, C2's `/ingest/ics` has nothing to read.
5. **The upgrade overlay can be dismissed and never traps a user.** *Not now* hides it and the console under it works; a sign-in attempt that cannot reach the service stands it down for the session; it returns on the next launch.
6. **The wizard's sign-in window can actually open.** The campus is chosen on the calendar panel, so pressing *Sign in to my school* opens the campus's own page rather than answering "no sign-in page is known for that school yet"; `scripts/wizard-check.py` (H6) exits 0.
7. **The first slot after onboarding lands a usable page** (§11a R-OB-1, R-OB-2). On the scratch profile: `state/runner-log.md` carries **no** `not in config; skipped` and no `0 assignments parsed`; `config/ingest.yaml` names every confirmed book and section under `courses:`/`sections:` with `HowToUseZyBooks2` in `ignore:`; `courses/` holds one note per enrolled course and `course_map:` one line each; and the tasks the first `rank` orders carry a `course:` rather than `null`. What remains uncoursed is what enrichment is for — and what is already past due is R-OB-3's, which is not this stream's.
8. **A student at any US college can finish the wizard** (§11a R-OB-4). Typing three letters of a school's name finds it through `campus_search` over `app/campuses.json`; picking it writes `config/campus.yaml` with its `unitid`, name and state and suggests a timezone from that state; a curated school additionally gets its event feeds and opens its own LMS sign-in page, and an uncurated one is asked which LMS it runs and is otherwise complete. "My school isn't listed" still produces a working vault.
9. `GET /entitlement` answers `{status, current_period_end, plan, checked_at}`; the device caches it; **72 hours** of no network still ranks the day, and past the grace the slot records `judge (skipped: no entitlement)` with exit code 0 and a green tray. (Spec §5.1.)
10. The **Stripe webhook is the only writer of `entitlements`** — provable on staging: `select count(*) from pg_policies where schemaname='public' and cmd<>'SELECT'` is `0`, and every table has `relrowsecurity = true`.
11. `DELETE /account` cancels at period end, purges every row, leaves a 90-day tombstone and deletes the login last; `GET /account/export` returns every table for the caller. Both are reachable from the app.
12. `POST /telemetry` accepts (a) and (b), refuses an unknown action and any free text, and stores nothing that is a title or a course name; `telemetry_daily` and `correction_rates` return nothing below ten accounts.
13. `POST /issues` stores a report that was **shown to the user, scrubbed, and editable** before it was sent, and scrubs it again on arrival.
14. The privacy policy and the terms are live at `knowlu.com/privacy.html` and `knowlu.com/terms.html`, Quinn has read both, and the wizard's one-sentence promise is the same string as the site's.
15. Google's restricted-scope verification is **submitted**, with the justification and the video.
16. `cargo test --workspace` green at 0 warnings; `deno test`, `deno lint` and `deno fmt --check` green; `git ls-files --eol` unchanged for everything this stream did not add.
17. `git diff --name-only main...c1-accounts` touches nothing outside this stream's ownership.

## What is NOT in this plan

- **C2's endpoints.** `/judge/{task,event,email}`, `/ingest/ics`, `/ingest/coursework`, `/events`, `/gmail/*`, `CloudModel` in the engine, the rule table, the eval suite. C1 writes `_shared/entitlement.ts` and stops. The wizard's Gmail panel is an honest stub with no endpoint behind it, and **§4.2 step 4's "a first fetch runs on the device and its payload goes to `/ingest/coursework` to prove the round trip" is not built either** — that endpoint is C2's, so C1's coursework panel stores the logins and says so, exactly as today. The proof of the round trip arrives with the endpoint.
- **Google sign-in on the calendars panel.** C2's, and deliberately: one Google connect that asks for `calendar.readonly` first — a *sensitive* scope, so lighter verification and no CASA — and `gmail.readonly` incrementally after it, with the calendar fetch server-side beside `/ingest/ics` (spec §11a). C1 leaves a labelled, **disabled** button and a sentence saying when it arrives; until then the secret iCal address is the path, and it is not a stopgap so much as the thing that works without an OAuth review.
- **R-OB-3, the past-due guard on a first ingest.** Ruled the same day and **not C1's**: it belongs to `engine/src/ingest.rs` and to C2's `/ingest-ics`, and `engine/**` is not this stream's to edit. Until it lands, a first ingest still creates items whose due date is behind the vault's birth, and the first page shows them. Named here so its absence is a decision rather than an oversight.
- **C3's sync.** `/sync/push`, `/sync/pull`, journal replay, restore-from-cloud, second-device support, and removing `history.rs`'s git.
- **C4's removal.** `inference.rs`, `SUPPORTED_RUNTIMES`, the settings panel's *Local judgment* row and the engine's `--runtime`/`--model` arguments all stay exactly as they are. Task 17 removes only the **wizard's** offer of them, because §4.2's step list has no such step.
- **Three of spec §6's (b) cases.** Class (b) here is **field overrides**: a human, at the console, setting a field an agent had set. §6 also names *amend-card decisions, snoozes of proposals and declined events* as corrections, and none of the three is derived. They are a different shape — a verdict on a card rather than a value on a field — and they need the approvals ledger, not the journal's `set` records. The `corrections` table already fits them (`item_id`, `field`, `ours`, `theirs`, `kind`), so adding them later is a reader, not a migration. Until then the eval suite sees overrides only, and this line is what stops that from reading as an oversight.
- **Telemetry class (c).** The opt-in for raw note bodies — its own screen, its own consent log, delete-on-revoke — is not built, and no toggle for it appears anywhere. The legal note asks whether it is needed at all in year one, given that the Gmail exclusion removes the richest source and rule promotion is per-user by design; that question is Quinn's, later.
- **The console's visual redesign**, parked by Quinn on 2026-09-07. Task 17 adds panels and rows in the existing design system and changes no token.
- **Reading grades from the signed-in LMS session.** Wanted (spec §11a, §13), and the same window could do it — but it waits on the university-policy read and on Task 13's capture proving out. Nothing here reads a grade page, and nothing here keeps the session that could.
- **Any flow that asks for, stores, or automates a campus SSO credential.** VISION's standing rule and spec §9 forbid it and §11a leaves it unchanged. If a future spike revisits it, the shape is the one §11a already ruled — *sign in yourself, in a window, and we keep only the link* — and it is a separate spec, not an extension of this plan.
- **`knowlu-prod`'s migrations.** Every apply and deploy in this plan names `knowlu-staging`. Production is migrated by Quinn, or by CI once C0's workflow can carry it — and the first production deploy is its own decision, with the first paying user behind it.
- **CI.** `.github/**` is C0's; C1's `deno test` job is hand-off H7, added by the controller at merge.
- **A third curated campus.** `scaffold::CAMPUSES` keeps its two curated rows — Alabama **with** event feeds, Kentucky **without**, because `events_preset_for` returns `none` for it and `app/assets/campus/` holds only `none.yaml` and `university-of-alabama.yaml`, and that directory is not this stream's. Every other US institution is in the bundled list and works without curation. A third curated row is one preset file and one line.

---

## Review rounds — deferred minors

**Round 1:** everything the review's Critical and Important sections named is fixed in place above,
and eleven of its fourteen Minors with them. These three are deliberately not done, each for a
reason:

- **The three other §6 (b) cases** (amend-card decisions, snoozes of proposals, declined events).
  *Declared rather than built* — the review offered either — because they read the approvals ledger,
  not the journal's `set` records, and that is a second deriver's worth of work for a signal the eval
  suite does not need before C2 exists. Named under *What is NOT in this plan*; the table already
  fits them.
- **Making the Rust scrubber regex-based so the two twins agree in general** (not only on the seven
  pinned cases). *Comment instead*, which the review offered: the app carries no `regex` dependency,
  adding one to make `see(https://x/y)` keep its bracket is a crate for a cosmetic difference, and
  both sides redact the URL — which is the property that matters. `scrub`'s doc comment now names the
  divergence so nobody discovers it as a surprise.
- **`--frozen` / a vendored Deno cache in CI.** `deno test` fetches `@std/assert` from JSR on a cold
  cache; CI has network, so this is a note rather than a defect today. Recorded in H7 so that whoever
  first runs CI without network knows where to look.

**Round 2 (the re-review of fix round 1):** both Importants and all four Minors fixed in place;
nothing deferred.

**Round 3 (the re-review of fix round 2):** both Importants and all three Minors fixed in place, and
the GoTrue caveat promoted from a doc comment to Task 10 **step 3a** as the review asked; nothing
deferred. The one thing still unverified from documentation is that step's own subject — which of
`"magiclink"` and `"email"` GoTrue's `/verify` wants — and it is now a `curl` an implementer runs
before the code that depends on it, with both answers written down.

**Round 4 (the re-review of fix round 3):** the one Important and all three Minors fixed in place;
nothing deferred. The Important is worth carrying into the stream's report as a habit rather than a
one-off: it was the **fourth** source-reading test in this plan written against a file it had not been
re-read against (after the LMS panel's `password`, `app_data_root`, and the ACTIONS `]` split), and
every one of the four was caught by reading the target file rather than the test. **Before running any
test in this plan that slices a source file, open the file it slices and check the anchor is unique.**
The four extractions that do this are `the_source_kind_vocabulary_is_one_list_in_three_places`,
`the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire`,
`the_unreachable_clause_is_one_string_on_both_sides` and
`the_wizards_privacy_sentence_is_the_sites_privacy_sentence`.

**Amendment (2026-09-09, after Quinn's first run on the fresh vault):** spec §11a gained the
first-run row and rulings R-OB-1, R-OB-2, R-OB-3. This plan gained **Task 14a** (the coursework
mapping, with hand-off **H10**'s read-only `coursework-discover` subcommand), **Task 14b** (the
enrolled courses out of the sign-in window, seeded as `courses/` notes and `course_map` lines), a
second go/no-go question in Task 13's spike, two inline blocks and their pins in Task 17, two ledger
rows, one exit-gate item and an entry under *What is NOT in this plan* for R-OB-3 — which belongs to
`engine/` and to C2, not to this stream. Nothing was deferred.

**Amendment 2 (2026-09-09, R-OB-4):** the school stopped being two radio buttons. Hand-off **H11**
generates `app/campuses.json` from the federal IPEDS *Institutional Characteristics* file
(4,319 active two- and four-year institutions, public domain, header-stamped with its source and
date); **Task 14c** turns `scaffold::CAMPUSES` into a curated layer keyed by `UNITID`, adds
`config/campus.yaml`, a state-to-timezone table and `onboarding::timezone_for_state`; Task 17's
calendars panel became a typeahead with a free-text fallback and a two-button LMS question; Task 14
and 14b key on the unitid. Counts moved to **26 / 43 / 58**. The same round closed the amendment
re-review's two Importants — the ruled outcome-C branch for the course list is written out and
`read_current_document` has all three bodies, and `scaffold::slugify` is gone in favour of the
engine's `knowlu_engine::ingest::slugify`, which is the function `judge::Heuristics` reads back —
and both its Minors. Nothing was deferred.

**Pre-flight round (2026-09-09, before Task 1 — rulings R-C1-10 and R-C1-11):** a pre-flight scan read
the whole plan for cross-task consistency and found 10 blocking, 13 important and 10 minor. **All 33 are
fixed in place; nothing is deferred.** The blocking ones were all of one kind — code written against a
state the plan does not reach until a later task — and they are worth naming as a class, because it is
the failure mode of writing a long plan in dependency order and then revising the early tasks last:
`create_vault_in` (Task 12) read four `VaultPlan` fields Tasks 14a and 14b add; `attach_in` (Task 18)
built the struct five fields short; `CourseSeed` was declared one task after the field that names it;
two tests used an unbound `js`; `WIZ` declared `campus` twice and the string won; and two tests already
in `app/tests/static_assets.rs` asserted the exact markup Task 17 removes. **Every one of them is a
compile error, and none of them is visible from inside the task that contains it.** The important ones
were mostly arithmetic and ownership bookkeeping — five tables counted as four, eight commands counted
as six, three tasks editing files their *Files* block did not list, `billing-portal` shipping without a
test, and the Stripe API version pinned in code but not on the endpoint. **R-C1-11's sequencing** is now
stated identically in **four** places — H9's own block, Task 13 step 3, Task 14a step 6 and Task 14b
step 3a: H9 is split into **H9a** (Task 13 step 3, three commands) and **H9b** (Task 14b step 3a, two
more), and the five controller applications run H1 → H9a → H10 → H9b → H11. Two Task-0 lines changed,
both named by findings (I13's commented-out `[auth.email.smtp]`, which would otherwise fail
`supabase db push` at Task 1 step 7, and M9's `fail()`); nothing else in Task 0 was touched, because it
is being implemented as this round lands.

**Pre-flight re-check (2026-09-09, late):** the scan re-read all 33 fixes — 30 clean, two fixed with a
new problem, one half-fixed — and raised one important and six minors. **All ten are fixed; nothing
deferred.** The half-fixed one is the only one that would have shipped a wrong value: **I2**. P2 named
the API version for the webhook *endpoint*, which governs the events Stripe pushes, but
`stripe-webhook/index.ts`'s `fetchSubscription` was a bare `fetch` with one header, so the one **GET**
whose body `entitlementFromSubscription` parses came back under the account's own default version —
where `current_period_end` is on neither shape the handler reads, and every entitlement row would carry
`null`. It is now `stripeGetFrom` in `_shared/stripe.ts`, pinned like every `POST`, with a test that
asserts the header on both. `billing-portal`'s uncovered 502 branch got its fourth test with it, so the
Deno totals moved again — **42 / 52 / 57 / 67 / 75 / 81** from Task 4 on, seventeen tests in Task 4
(four stripe, four checkout, five webhook, four portal). The rest were bookkeeping the fixes had just
disturbed: Task 14a asked for H10 at two different steps (now step 6 only), H9's own block did not
state the order the three task sites state, the hand-offs intro still said everything from H3 down waits
for merge, Task 17's red count was six against eight actually-red tests, a pin promised as "inverted"
did not assert `offer_inference`, and Task 12 has three `VaultPlan` literals rather than two — one of
which, `a_new_vault_carries_the_four_cloud_keys_and_no_secret`, pins the profile id in its expected
string and therefore keeps its own literal on purpose.

**Amendment 2, fix round (2026-09-09):** the R-OB-4 re-review's three Importants and three Minors, plus
the Part-A residual, all fixed in place; nothing deferred. The largest: **the page never fetches the
school list** — `app/tauri.conf.json`'s `connect-src ipc: http://ipc.localhost` names no `'self'`, and
that file is the controller's, so the search became `onboarding::campus_search` over an
`include_str!`'d `app/campuses.json` and the page holds ten rows instead of six thousand. H11's header
lost its URL (the whole-file no-URL assertion holds again), and every LMS endpoint is now built from
the curated row's **own** `lms_host` rather than one tenant per kind — with a test that no curated
school's URL contains another's host. Counts moved to **27 / 43 / 59**.
