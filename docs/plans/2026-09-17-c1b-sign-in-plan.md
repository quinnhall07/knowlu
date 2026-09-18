# Knowlu C1b — sign-in: Continue with Google, the code without a password, Back and Next, and a promotion code — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: AMENDED 2026-09-17 (fix rounds 1 and 2 after review), not started.** Execute on a branch `c1b-sign-in` in a worktree of this repository, merged into `main` before C3', C5 and C4 (HANDOFF §3: Quinn, 2026-09-17, "as soon as possible", before every other stream). Written against `docs/specs/2026-09-17-c1b-sign-in-design.md`.

**Goal:** A student who has never heard of Knowlu presses **Continue with Google**, picks their account in the browser they are already signed in to, and comes back to a wizard that has moved on — no password, no code, nothing pasted. A student who would rather not use Google types their address, presses **Email me a code** once, types the code, and is in. Whichever they chose, **Back always goes back and Next either moves or says in one sentence what is missing**. And the founder can finish the subscribe step with a 100-percent promotion code and no card.

**Architecture:** Three layers, the same three C1 used, and no new one. **The cloud:** one new migration that lets an OAuth sign-up create its `accounts` row (today's trigger raises on it), one new route `POST /account/consent` on the existing `account` function that records the 18+ attestation and the two policy versions after the fact, and one form field on `billing-checkout` (R-C1b-2: `allow_promotion_codes`; the card stays) — plus the attestation gate that replaces the `raise` the trigger gives up. **The device:** `app/src/account.rs` gains a loopback `TcpListener` on `127.0.0.1:0`, a PKCE verifier and challenge built from `sha2` (already a dependency) and a hand-rolled base64url, and one command `google_sign_in`; `sign_up` and `sign_in` are deleted with the password they took. **The page:** the account panel and the upgrade overlay lose the password field and the create/sign-in split, gain one Google button, and the wizard's nav learns that a disabled button must look disabled and that `#wiz-next`'s disabled state belongs on `WIZ` like every other wizard field.

**Tech Stack:** Supabase (Postgres 15, Auth/GoTrue, Edge Functions on Deno), Stripe Checkout; Rust 1.98 `stable-x86_64-pc-windows-gnu` with `ureq 3.4` and `sha2 0.10` — both already in `app/Cargo.toml`, so **this plan adds no crate**; plain ES5-flavoured JavaScript in `app/static/console.js`, no framework and no bundler.

**Spec:** `docs/specs/2026-09-17-c1b-sign-in-design.md` (D1-D8, §2 the flow, §4 the allow-list finding, §5 the accounts-row finding, §7 Back and Next, §8 the promotion code, §9 Quinn's four, §10 the privacy sentences). Supporting: `docs/specs/2026-09-09-knowlu-cloud-design.md` §4.2 and §5.1 and its *Amendment 2026-09-17*; `CLAUDE.md`; `VISION.md`; `HANDOFF.md` §3.

---

## Global Constraints

Every task's requirements implicitly include this section.

- **Add no single-user assumptions.** Nothing in this stream names a person's vault, machine, account, email address, OAuth client, promotion code or credential. The loopback port is chosen by Windows at bind time and is never written down. (`CLAUDE.md`, rule 1.)
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` and the three Rust-generated `surface-today-*.json` are not read or written by anything here. (`CLAUDE.md`, rule 2.)
- **No secret in the repo, a log, a fixture, a test name, a commit message or this plan.** The Supabase project URL and anon key are **public** and are compiled into the app. **The Google client id and secret appear nowhere at all** (ruling R-C1b-5): the provider is configured in the Supabase dashboard on both projects, `config.toml` declares no `[auth.external.google]` block, and no `GOOGLE_*` value is ever needed in a shell, a session or this repo. `SMTP_PASSWORD` keeps its `env(...)` shape and is the only secret this file references. The PKCE verifier, the authorisation code and both tokens are never logged, never in an error message, and never cross the IPC to the page. If a value is ever printed, say so immediately and treat it as exposed.
- **`rank` never calls a model, and nothing under `cli.rs` can reach one.** This stream adds no engine code at all; `engine/**` is not this stream's to edit.
- **Every vault write still goes through the engine's `write` with `console_ctx()`.** This stream writes no note and touches no vault file: `config/cloud.yaml` is still `scaffold`'s at vault birth, and `journal::VIAS` does not grow.
- **`cargo build --workspace` and `cargo test --workspace` from the root, at 0 warnings.** The one accepted line is the app's pre-existing `.rsrc merge failure: multiple non-default manifests` linker message. The four `#[ignore]`d tests stay ignored; none may be un-ignored by changing an assertion.
- **TDD, always: the test first, then the code.** Every step below is written in that order, and a step that shows implementation before its test is a plan defect — stop and report it.
- **No test reaches the network.** Rust HTTP is tested against a loopback `TcpListener` bound to `127.0.0.1:0` whose serving thread is **joined before the test returns** (C1's 3a rule, and `app/tests/account.rs`'s `loopback()` helper is already there). **A loopback listener inside a test is not the network** — it is the same machine, the same process tree, and nothing leaves it; the PKCE tests stand a server up on `127.0.0.1:0` and drive the real code against it. Deno tests are pure modules over an injected `Deps`: no `fetch`, no Docker, no Supabase runtime.
- **Tests that touch the real Credential Manager take the file lock.** `app/tests/account.rs` holds `CREDMAN_LOCK`, and every test that writes, reads or deletes a real credential takes it, under a generated test id with a `Drop` guard that deletes what it wrote (`CLAUDE.md`). A new test file that touches the store carries its own lock.
- **`app/static/` carries no `http://` or `https://` literal** — `app/tests/static_assets.rs::no_network_reference_in_the_shipped_page` enforces it. The authorize URL, the redirect URL and the token URL are all built in Rust; the page sends one argument-less `invoke` and receives an account id.
- **Line endings: LF everywhere** (`.gitattributes`: `* text=auto eol=lf`; `*.ps1` CRLF; `engine/tests/fixtures/** -text`). New and edited `.ts`, `.sql`, `.rs`, `.html`, `.css`, `.js` and `.md` files are LF, UTF-8, no BOM.
- **`cargo test --release` will not link** (`panic = "abort"` in the one release profile). Test in the dev profile.
- **Migrations are `cloud/supabase/migrations/<YYYYMMDDHHMMSS>_<name>.sql` and this stream uses timestamps in 2026-09-17 only** (`20260917…`). No other stream has used that day.
- **Migrations are applied with the Supabase CLI against `knowlu-staging` from a developer machine.** Production is Quinn's, at the close (Task 7), and only after staging is green.
- **Commits:** specific `git add` (**never `git add -A`**), message via `-F <file>`, trailers:

  ```
  Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_018EXZqBCHaJBKtYtkNfjj1Z
  ```

- **File ownership is binding.** This stream edits only: `cloud/supabase/config.toml`, `cloud/supabase/migrations/20260917000100_oauth_consent.sql` (new), `cloud/supabase/migrations_test.ts`, `cloud/supabase/migrations/migrations_test.ts` (one line — the corpus pin, Task 1 step 1(b)), `cloud/supabase/functions/account/**`, `cloud/supabase/functions/billing-checkout/**`, `cloud/supabase/functions/_shared/config_toml_test.ts` (the `config.toml` pins, Task 4 step 3b), `app/src/account.rs`, `app/static/**`, `app/tests/{account.rs,static_assets.rs}`, `site/privacy.html`, and this plan file. Everything else — `app/src/main.rs`, `scripts/wizard-check.py`, `CLAUDE.md`, `HANDOFF.md` — is the controller's and appears under **Controller hand-offs** with exact code. **A task that silently edits one of those files is a plan defect** — stop and report it instead of editing.

---

## Quinn-owned preconditions

Asked **one at a time, when the task reaches them, with the context** — never as a list of chores.

| # | Needed by | What, and what breaks without it |
|---|---|---|
| **P1** | **DONE 2026-09-17** | **A new Google Cloud project, publishing status "In production", with one OAuth 2.0 client of type "Web application"** — consent-screen scopes `openid`, `email`, `profile` and nothing else, so Google asks for no verification review and imposes no 100-test-user cap (spec §3); authorised redirect URIs for **both** projects' `/auth/v1/callback`. **This is a second project on purpose:** the existing one carries `gmail.readonly` and `calendar.readonly`, which are restricted and sensitive, and one client carrying a restricted scope drags the whole client through review — a sign-in button that works for 100 named testers is not a sign-in button. **Done on 2026-09-17:** Quinn created it and enabled the provider **in the Supabase dashboard on both projects**, and the controller verified it without holding a key — `/auth/v1/authorize?provider=google` answers 302 to `accounts.google.com` on staging and on prod, each carrying its own project's `/auth/v1/callback` as `redirect_uri` and `email profile` as the scope. **No value from it enters this session, this repo or any shell here** (R-C1b-5). |
| **P2** | Task 7 step 3 — **and it is the controller's, not Quinn's** | **`supabase config diff`, then `supabase config push`, against staging and later prod** — carrying exactly two things: `enable_confirmations = false` (spec D5) and `[auth.rate_limit] email_sent` (review I2). **No `GOOGLE_CLIENT_ID` and no `GOOGLE_SECRET` anywhere** (R-C1b-5): `config.toml` declares no provider, and the CLI (2.117.0) "pushes the properties your local config.toml declares … properties the file does not declare are left unchanged", so the dashboard's provider is untouched — which the `config diff` is run first to prove. **Without it:** the email path still sends the stock *Confirm your signup* mail, with a link and no code, and the `/otp` rate limit stays at whatever the platform defaults to. |
| **P3** | Task 2, and Task 7's live proof | **A 100-percent promotion code in Stripe, in BOTH modes.** A coupon at 100% off, a promotion code attached to it, duration Quinn's choice — in **test** mode so the exit gate can prove the flow, and in **live** mode so Quinn can actually finish their own subscribe step. Values: none that this session needs; the code is typed into Stripe's own page by whoever uses it. **Without it:** `allow_promotion_codes` renders a *Add promotion code* link with nothing valid to type into it, and Quinn still cannot get past the subscribe panel. |
| **P4** | Task 7 step 4 | **Quinn reads the three changed privacy sentences** (spec §10) before they are published: the password clause out of *Your account*, the password-hash clause out of the *Supabase* bullet, and the new opening sentence on the Google section. The page's date and `account::PRIVACY_VERSION` move together in the same commit, which is `account.rs`'s own standing rule. **Without it:** the published policy describes a password Knowlu no longer has, while the app records consent to that version. |

Secrets are set from a value Quinn produces, never typed into this session.

---

## Controller hand-offs

### H1 (Task 3, before the command can be called) — `app/src/main.rs`, the two `generate_handler!` lists

The wizard/picker window's list (**30** today) loses `account::sign_up` and `account::sign_in` and gains `account::google_sign_in`, so **29**:

```rust
.invoke_handler(tauri::generate_handler![onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, onboarding::adopt_vault, onboarding::open_profile, onboarding::create_vault, onboarding::restore_vault, onboarding::apply_profile_settings, onboarding::store_credentials, onboarding::retarget_credentials, onboarding::finish_onboarding, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::google_connect_url, account::google_connected, account::open_external, lms_link::open_lms_window, lms_link::capture_calendar_link, lms_link::capture_courses, lms_link::paste_calendar_link, lms_link::close_lms_window, onboarding::discover_coursework, onboarding::timezone_for_state, onboarding::campus_search])
```

The console window's list (**43** today) loses the same two and gains the same one, so **42**:

```rust
.invoke_handler(tauri::generate_handler![commands::state, commands::note, commands::mark_seen, commands::ui_event, commands::set_fields, commands::create_task, commands::delete_note, commands::decide, commands::close_info, commands::open_issue, commands::resolve_issue, commands::sync, commands::backup_now, commands::get_settings, commands::set_settings, commands::set_profile_name, commands::copy_diagnostics, commands::copy_text, commands::settings_context, commands::switch_profile, commands::check_for_updates, commands::install_update, commands::inference_status, commands::install_inference_file, commands::install_inference_download, commands::remove_inference_model, onboarding::launch_state, onboarding::pick_folder, onboarding::pick_file, account::google_sign_in, account::send_magic_link, account::verify_email_code, account::sign_out, account::open_policy, account::entitlement_now, account::open_checkout, account::account_status, account::open_portal, account::attach_account, account::delete_my_data, report::report_preview, report::report_send])
```

`google_sign_in` is in **both** lists for the reason the other sign-in commands are: the upgrade overlay runs inside the console window, over an existing vault, and offers the same two doors the wizard does.

New totals to quote afterwards: **29 + 42, 61 distinct** (`launch_state`, `pick_folder`, `pick_file` and the **seven** sign-in commands — `google_sign_in`, `send_magic_link`, `verify_email_code`, `sign_out`, `open_policy`, `entitlement_now`, `open_checkout` — are in both lists; 71 registrations, 10 of them twice). Recounted by hand against both `generate_handler!` lists (review M2). The seven note-mutating commands are unchanged. **Apply H1 before Task 3 step 5** — a command that is not registered is rejected before its body runs, and the page's `.catch` then paints a transport error for a call that never left the process.

### H2 (Task 5) — `scripts/wizard-check.py`, the headless wizard walk

Three edits, all in `check(page)` and its fake:

1. `BEFORE_FINISH_OK` loses `"sign_up"` and `"sign_in"` and gains `"google_sign_in"`. The set's meaning is unchanged — the only commands the wizard may have called before Finish are reads, the account, and the two that write a credential.
2. The fake's `sign_up`/`sign_in` branch is replaced by one that answers the new command:

   ```javascript
   if (cmd === 'google_sign_in') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
   ```

   …and `send_magic_link` keeps its `{ ok: true, error: null }` answer.
3. Check 2's body — today it fills `#wiz-email` and `#wiz-pw`, clicks `#wiz-create` twice and asserts on `sign_up` — becomes: assert `#wiz-pw` does not exist at all; click **`#wiz-google-signin`** with the boxes unticked and assert `google_sign_in` was **not** called and the error names the boxes; tick both, click `#wiz-google-signin`, assert `google_sign_in` was called and the panel advanced to `#wiz-subscribe`. The exact replacement text is Task 5 step 5.

   **`#wiz-google-signin` throughout, never `#wiz-google`** (review I8): `#wiz-google` is the Google **Calendar** connect button on the calendars panel, pinned by `static_assets.rs:426` and `:465`, and clicking it here would drive the wrong control on the wrong panel and pass by accident.

Check 8 (Back five panels, Next five panels) is **unchanged and is the point** — it is what proves §7's fix did not break the walk.

### H3 (Task 7) — `CLAUDE.md`, two edits

1. In *Knowlu (the app)*, the recount sentence: **43** becomes **42** for the console window, **30** becomes **29** for the picker/wizard window, **62** becomes **61** distinct, and the date becomes **2026-09-17 (C1b Task 7)**. The "+3 from C2's hand-off H9" clause stays; a new clause records that `sign_up` and `sign_in` are gone with the password and `google_sign_in` arrived.
2. In the same section, one new sentence after the session line: *"There is no password on a Knowlu account: sign-in is `account::google_sign_in` (a loopback PKCE round trip on `127.0.0.1:0`, one listener per sign-in) or the emailed six-digit code; `/auth/v1/signup` and `grant_type=password` are called by nothing."*

### H4 (Task 7) — `HANDOFF.md` §3, the sequence

One edit: C1b's row moves from "spec + plan drafting" to merged, with the branch and the merge commit, and the order after it is unchanged (C3' → C5 → C4 → production parity → the pilot). The controller writes it at merge, not the stream.

---

### Task 1: The cloud — an OAuth sign-up that can create its `accounts` row, `POST /account/consent`, and the attestation gate that replaces the `raise`

**Files:**
- New: `cloud/supabase/migrations/20260917000100_oauth_consent.sql`
- Modify: `cloud/supabase/functions/account/handler.ts`, `cloud/supabase/functions/account/index.ts`
- Test: `cloud/supabase/functions/account/handler_test.ts`, `cloud/supabase/migrations_test.ts` (the per-file pins) **and `cloud/supabase/migrations/migrations_test.ts`** (C1/C2's corpus pins — a **different, 788-line file**, and the one review finding C1 is about)

**Interfaces:**
- Consumes: `_shared/auth.ts::requireUser`, `_shared/http.ts::{json, fail, readJson, methodNotAllowed, subPath}`.
- Produces: the route `POST /account/consent`, and `Deps.hasConsent` / `Deps.recordAccountConsent` (both added to `deps()` in `handler_test.ts` first — I5). Task 2 consumes the `age_attested_at` column this task now leaves null on **every** path; Task 3's `google_sign_in` and Task 4's `verify_email_code` both call the route.

Spec §5.1 is the finding this task exists for: today's `handle_new_user()` raises `the terms and the privacy policy must be accepted at sign-up` on a Google sign-up, because Google's ID-token claims carry none of the three metadata keys, and the `auth.users` insert rolls back with `Database error saving new user`.

- [ ] **Step 1: The migration tests first** — **two files**, because the pins live in two.

  **(a)** Add to `cloud/supabase/migrations_test.ts` (the per-file pins), beside the three invariants it already holds:

```typescript
Deno.test("the OAuth migration adds no table, no policy, no birthdate column — and no raise", async () => {
  const sql = await Deno.readTextFile(
    new URL("./migrations/20260917000100_oauth_consent.sql", import.meta.url),
  );
  // **The `--` lines come off first.** This migration's comment block explains at length what the
  // function no longer reads, so an assertion over the raw text would be an assertion about the
  // prose. `migrations/migrations_test.ts` strips comments before scanning for the same reason.
  const code = sql.split("\n").filter((l) => !l.trim().startsWith("--")).join("\n");
  // C1's three rules are pinned over the whole directory elsewhere; this one is about THIS file:
  // it replaces one function and nothing else, so a reviewer never has to diff schema to be sure.
  assert(!/create\s+table/i.test(code), "this migration creates no table");
  assert(!/create\s+policy/i.test(code), "…and no policy: RLS is C1's and stays as it is");
  assert(!/\bbirth|\bdob\b|date_of_birth/i.test(code), "no birthdate column, in this file or any other");
  assert(code.includes("create or replace function public.handle_new_user()"), "the trigger's function is replaced");
  // R-C1b-3. The function reads NOTHING out of the sign-up's metadata and raises nothing: `/otp`
  // with `create_user: true` is reachable by anyone holding the public anon key, so a trigger that
  // believed that request's `data` would stamp an `age_18` consent row for an address whose owner
  // never attested to anything. The consent row is `POST /account/consent`'s to write, behind a
  // session, and the 18+ tooth is `billing-checkout`'s 403.
  assert(!/raise\s+exception/i.test(code), "no raise survives in the replaced function");
  // Asserted over the SOURCE of the values, never their names: `age_attested_at` and `tos_version`
  // are columns this migration still writes (as nulls), so banning those words would ban the insert.
  assert(!code.includes("raw_user_meta_data"), "the trigger reads none of the sign-up's own metadata");
  assert(!code.includes("public.consents"), "…and writes no consent row: that is the route's, behind a session");
});
```

  **(b)** Bump C1's corpus pin in `cloud/supabase/migrations/migrations_test.ts` — **the other file**, 788 lines, holding the counts C2 left behind (review C1). `assertExecuteRevoked` increments `parsed` **before** the `returns trigger` exemption (`migrations_test.ts:139-144`), and it runs over `everyMigrationFile()` — every `.sql` in the directory, not only C2's `20260911…` ones — so this task's `create or replace function public.handle_new_user()` is the **nineteenth** definition the scan parses and `:304`'s `assertEquals(parsed, 18, …)` goes red on this task's own `deno test` gate (step 6). Task 1's new per-file test is scoped to one file and cannot see it. One number, one sentence, the treatment the provider swap got in that same comment:

```typescript
  // replace function export_training_rows` in 20260916000100 (still SECURITY INVOKER,
  // non-writing, so it needs no new revoke — it is still one more definition this scan parses),
  // plus C1b's `create or replace function public.handle_new_user()` in 20260917000100 (a trigger
  // function, so exempt from the revoke by kind — and still one more definition parsed) —
  // counted by hand against today's corpus.
  assertEquals(parsed, 19, "today's corpus should parse exactly 19 function creations");
```

  Nothing else in that file moves: the view pin at `:320` counts views and this migration creates none, and the trigger-exemption test at `:462` asserts `exemptedTriggers.includes("handle_new_user")`, so a second definition under the same name is what it wants. **No test anywhere pins the raise text** — `grep -rn "age attestation required\|accepted at sign-up" cloud app` finds only `20260910000100_accounts.sql:120` and `:123` themselves — so the raise can go without a test rewrite beyond this one.

- [ ] **Step 2: The migration** — `cloud/supabase/migrations/20260917000100_oauth_consent.sql`:

```sql
-- Knowlu C1b, Task 1 — an OAuth sign-up can create its account row (spec §5.1, ruling R-C1b-3).
--
-- Google's ID-token claims carry no `age_attested`, `tos_version` or `privacy_version`, so today's
-- function raises on the second `if` and the whole `auth.users` insert rolls back: GoTrue answers
-- `Database error saving new user` and redirects with `error=server_error`.
--
-- The replacement reads NOTHING out of `raw_user_meta_data`, on either path. Not because Google
-- cannot send it, but because the email path's `/otp` is reachable by anyone holding the public anon
-- key and, with `enable_confirmations = false`, GoTrue creates the user before the code is ever
-- typed: a trigger that believed that request's own `data` would write an `age_18` consent row
-- asserting an attestation the address's owner never made, for any address a stranger chose. So
-- every new user starts with the four consent columns null and no `consents` rows, and
-- `POST /account/consent` — behind a session whose address has been proved — is the only writer of a
-- consent row in the system.
--
-- The 18+ gate does not leave the server with the raise: `billing-checkout` answers
-- `403 the 18+ attestation is missing` while `age_attested_at` is null, before the Stripe customer
-- and before the consent row. A patched client that skips the consent call gets an account it can
-- never subscribe with.
--
-- Nothing else changes: no table, no policy, no column. The three rules `migrations_test.ts` pins
-- are untouched.
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public, extensions as $$
begin
  insert into public.accounts (id, email, tos_version, tos_accepted_at, privacy_version, age_attested_at)
  values (new.id, new.email, null, null, null, null)
  on conflict (id) do nothing;

  insert into public.entitlements (account_id, status) values (new.id, 'none')
  on conflict (account_id) do nothing;

  return new;
end;
$$;
```

The trigger itself is not recreated: `create or replace function` swaps the body under the existing `on_auth_user_created`. The `declare` block goes with the metadata it read, and `extensions.digest` goes with the `consents` insert — the hash is `POST /account/consent`'s to compute now, through the same `sha256Hex` the rest of the `account` function uses (step 5).

- [ ] **Step 3: The route's test first** — add to `cloud/supabase/functions/account/handler_test.ts`:

**Both new `Deps` fields land in `deps()` first** (review I5). `handler_test.ts:6`'s `deps(over: Partial<Deps> = {}): Deps` returns a **complete** literal and `deno test` type-checks, so two new required fields on the interface are a type error in every existing test in this file until that one literal carries them. Add them there, with the defaults a test that is not about consent wants:

```typescript
    // Task 1: the two consent fields. Default to "this account has not consented and the write is a
    // no-op", so every DELETE/export/sources test in this file is untouched by the new route.
    hasConsent: () => Promise.resolve(false),
    recordAccountConsent: () => Promise.resolve(),
```

…and build the consent fixture **from** `deps()` rather than as a second full literal — the one place this plan would otherwise duplicate an interface verbatim:

```typescript
function consentDeps(recorded: unknown[], already: boolean): Deps {
  return deps({
    hasConsent: () => Promise.resolve(already),
    recordAccountConsent: (c) => {
      recorded.push(c);
      return Promise.resolve();
    },
    now: () => new Date("2026-09-17T12:00:00Z"),
  });
}

Deno.test("POST /account/consent records the attestation and both versions", async () => {
  const recorded: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: true }),
    }),
    consentDeps(recorded, false),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { ok: true });
  assertEquals(recorded, [{
    account_id: "acc-1",
    email: "a@example.invalid",
    tos_version: "2026-09-10",
    privacy_version: "2026-09-17",
    at: "2026-09-17T12:00:00.000Z",
  }]);
});

Deno.test("a second call is a no-op — the app calls it after every sign-in", async () => {
  const recorded: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: true }),
    }),
    consentDeps(recorded, true),
  );
  assertEquals(res.status, 200);
  assertEquals(recorded.length, 0, "an account that already consented is not written again");
});

Deno.test("age_attested false, or a missing version, is 400 and writes nothing", async () => {
  for (
    const body of [
      { tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: false },
      { tos_version: "", privacy_version: "2026-09-17", age_attested: true },
      { tos_version: "2026-09-10", age_attested: true },
    ]
  ) {
    const recorded: unknown[] = [];
    const res = await handle(
      new Request("http://127.0.0.1:1/account/consent", {
        method: "POST",
        headers: { authorization: "Bearer good" },
        body: JSON.stringify(body),
      }),
      consentDeps(recorded, false),
    ).catch((e) => e as Response);
    assertEquals(res.status, 400);
    assertEquals(recorded.length, 0);
  }
});

Deno.test("GET /account/consent is 405, not 404 — the route exists", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", { method: "GET", headers: { authorization: "Bearer good" } }),
    consentDeps([], false),
  );
  assertEquals(res.status, 405);
});
```

- [ ] **Step 4: The route** — in `cloud/supabase/functions/account/handler.ts`, two `Deps` fields and one handler, then one `case` in `handle`:

```typescript
export interface ConsentWrite {
  account_id: string;
  email: string;
  tos_version: string;
  privacy_version: string;
  at: string;
}

// …added to `interface Deps`:
  /** True when this account already has a `tos` consent row — the idempotence test. */
  hasConsent: (accountId: string) => Promise<boolean>;
  /** Fills the four null columns on `accounts` and inserts the three `consents` rows. */
  recordAccountConsent: (c: ConsentWrite) => Promise<void>;

/**
 * **The attestation, after the fact — and the only place a consent row is ever written.** Migration
 * `20260917000100` stopped the auth trigger reading the sign-up's own metadata at all, on BOTH
 * paths, because `/otp` with `create_user: true` is reachable by anyone holding the public anon key:
 * an attestation taken out of that request would be an `age_18` row the address's owner never made.
 * So every new account arrives here with its four consent columns null, and the app calls this route
 * after EVERY sign-in — `google_sign_in` and `verify_email_code` alike — which is why a second call
 * must be silent rather than a conflict.
 *
 * The 18+ gate has not moved off the server: `billing-checkout` refuses an account whose
 * `age_attested_at` is still null, so a client that skips this route gets an account that can never
 * subscribe. That refusal is the whole of the tooth the migration's `raise` used to be.
 */
async function recordConsent(req: Request, deps: Deps): Promise<Response> {
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ tos_version?: string; privacy_version?: string; age_attested?: boolean }>(req);
  if (body.age_attested !== true) throw fail(400, "age attestation required: Knowlu is for people 18 or older");
  const tos = (body.tos_version ?? "").trim();
  const priv = (body.privacy_version ?? "").trim();
  if (!tos || !priv) throw fail(400, "the terms and the privacy policy must be accepted at sign-up");
  if (await deps.hasConsent(user.id)) return json(200, { ok: true });
  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");
  await deps.recordAccountConsent({
    account_id: user.id,
    email: account.email,
    tos_version: tos,
    privacy_version: priv,
    at: deps.now().toISOString(),
  });
  return json(200, { ok: true });
}

// …and in `handle`'s switch, between `/export` and `/sources`:
    case "/consent":
      if (req.method !== "POST") return methodNotAllowed(["POST"]);
      return await recordConsent(req, deps);
```

- [ ] **Step 5: The wire** — in `cloud/supabase/functions/account/index.ts`, two `Deps` entries beside the existing ones. **PostgREST, not raw SQL** (review M6): this file is `restSelect` / `restPatch` / `restUpsert` throughout, and all three — and `sha256Hex` — are already imported at `index.ts:1-11`, so **no new import**:

```typescript
      hasConsent: async (id) => {
        const rows = await restSelect<{ id: number }>(
          rest,
          "consents",
          `account_id=eq.${encodeURIComponent(id)}&kind=eq.tos&select=id&limit=1`,
        );
        return rows.length > 0;
      },
      recordAccountConsent: async (c) => {
        // `age_attested_at=is.null` in the filter, not just in the handler's guard: two sign-ins
        // racing each other must not restamp an attestation this account already made.
        await restPatch(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(c.account_id)}&age_attested_at=is.null`,
          {
            tos_version: c.tos_version,
            tos_accepted_at: c.at,
            privacy_version: c.privacy_version,
            age_attested_at: c.at,
          },
        );
        // `sha256Hex` lower-cases, exactly as the C1 trigger's `lower(new.email)` did, so a row
        // written here and a row written before this migration hash the same address the same way.
        const hash = await sha256Hex(c.email);
        await restUpsert(rest, "consents", [
          { account_id: c.account_id, subject_hash: hash, kind: "tos", version: c.tos_version },
          { account_id: c.account_id, subject_hash: hash, kind: "privacy", version: c.privacy_version },
          // `'1'` — the literal the C1 trigger stamped an `age_18` row with. The attestation has no
          // document and no date to version, and the log has to read the same on both sides of this
          // migration (review M6).
          { account_id: c.account_id, subject_hash: hash, kind: "age_18", version: "1" },
        ]);
      },
```

- [ ] **Step 6: Green, then commit.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`, `deno lint`, `deno fmt --check`. Commit `cloud/supabase/migrations/20260917000100_oauth_consent.sql`, `cloud/supabase/migrations_test.ts`, `cloud/supabase/migrations/migrations_test.ts` (the bumped corpus pin) and `cloud/supabase/functions/account/{handler.ts,handler_test.ts,index.ts}` by name.

---

### Task 2: Checkout takes a promotion code, still takes the card, and refuses an account that never attested

**Files:**
- Modify: `cloud/supabase/functions/billing-checkout/handler.ts`, `cloud/supabase/functions/billing-checkout/index.ts`
- Test: `cloud/supabase/functions/billing-checkout/handler_test.ts`

**Interfaces:**
- Consumes: Task 1's `accounts.age_attested_at`, now nullable in practice as well as in the schema.
- Produces: nothing new for another task. Quinn's P3 code is what exercises it.

Spec D8 and §8, as amended by **ruling R-C1b-2** (review C6). One line is added to `checkoutForm` (`handler.ts:39`) — `"allow_promotion_codes": "true"` — and `"payment_method_collection"` **stays `"always"`** (`handler.ts:55`).

**Why the plan no longer moves it to `"if_required"`.** Stripe collects a payment method under `if_required` only when the first invoice has an amount due, and `subscription_data[trial_period_days]` is `"7"` (`handler.ts:54`), so the first invoice is zero for **every** subscription: the change would have taken a card from nobody, not merely from the student holding a 100-percent code. That repeals §11 R2 rather than relaxing it, falsifies the bolded auto-renew sentence on `site/terms.html:24` and on the subscribe panel (`app/static/index.html:95`) — the text whose `TOS_VERSION` every `auto_renew` consent row is stamped with, and which Task 7 step 2 deliberately does not move — and leaves each trial ending with no payment method, which Stripe resolves by invoicing into `past_due` and the webhook writes straight into `entitlements`. Quinn's own subscription is solved outside the product: a card typed once under the 100-percent code and never charged, or a comped subscription from the Stripe dashboard. A card-free trial **for everyone** is option (b) of the review — a terms rewrite with the lawyer and a `TOS_VERSION` move — and is recorded for Quinn in *Fix round 1 — resolutions*, not built here.

- [ ] **Step 1: The test first** — in `handler_test.ts`, extend the form test and add two:

```typescript
  // R-C1b-2. `always` STAYS: `if_required` collects a card only when the first invoice has an
  // amount due, and `subscription_data[trial_period_days]` makes that invoice zero for every
  // subscription — so it would take a card from nobody and falsify `site/terms.html`'s bolded
  // "your card taken at sign-up", the version each `auto_renew` consent row is stamped with.
  assertEquals(form["payment_method_collection"], "always");
  // Spec §8: the promotion-code field on Stripe's own page. The code is Quinn's to create in the
  // dashboard (P3); nothing in this repo names a code, a coupon id or a percentage.
  assertEquals(form["allow_promotion_codes"], "true");
```

```typescript
Deno.test("an account that never attested to being 18 cannot reach Stripe", async () => {
  let touched = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      // Migration 20260917000100 lets an OAuth sign-up land here with a null attestation; this is
      // the server-side tooth that replaced the trigger's `raise`.
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null, age_attested_at: null }),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => {
        touched = true;
        return Promise.resolve();
      },
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
  assertEquals(res.status, 403);
  assert(!touched, "no customer, no consent row, no session");
});

Deno.test("an account that did attest goes through exactly as before", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: "cus_1",
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => Promise.resolve(),
      stripe: () => Promise.resolve({ id: "cs_4", url: "https://checkout.stripe.com/c/cs_4" }),
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(res.status, 200);
});
```

**Four** existing inline `getAccount` stubs gain `age_attested_at: "2026-09-17T12:00:00Z"` — `handler_test.ts:43`, `:79`, `:114` and `:142` (review I5: they are contextually typed literals, so the widened return type in step 3 is a type error in each until it carries the field). Nothing else about those tests moves.

- [ ] **Step 2: The form** — in `checkoutForm` (`handler.ts:39`), **add one line and change none**. `"payment_method_collection": "always"` (`:55`) and `"subscription_data[trial_period_days]": "7"` (`:54`) are both left exactly as they are:

```typescript
    // §8, R-C1b-2: Stripe's own promotion-code field, beside a card that is still taken up front.
    // The code is typed on Stripe's page; nothing in this repo names a code, a coupon or a percentage.
    "allow_promotion_codes": "true",
```

- [ ] **Step 3: The gate** — widen `Deps.getAccount`'s return type to `{ email: string; stripe_customer_id: string | null; age_attested_at: string | null }`, and immediately after the existing `if (!account) throw fail(404, "no such account");`:

```typescript
  // Spec §5.1: migration 20260917000100 stopped the auth trigger raising on an OAuth sign-up, so
  // the 18+ gate lives here now — before the Stripe customer, before the consent row, before the
  // session. `POST /account/consent` is what fills it, and the app calls that on every sign-in.
  if (!account.age_attested_at) throw fail(403, "the 18+ attestation is missing — sign in again");
```

- [ ] **Step 4: The wire** — `index.ts`'s `getAccount` select adds `age_attested_at`. One column, no new import.

- [ ] **Step 5: Green, then commit.** `deno test`, `deno lint`, `deno fmt --check`. Commit the three `billing-checkout` files by name.

---

### Task 3: `app/src/account.rs` — the loopback listener, the PKCE pair, and `google_sign_in`

**Hand-off H1 must be applied before step 5.**

**Files:**
- Modify: `app/src/account.rs`
- Test: `app/tests/account.rs` (its `loopback()` helper is already there, from C1 Task 10 step 1)

**Interfaces:**
- Consumes: `account::{api_base, anon_key, auth_base, check_api_base, agent, post_json, session_from, save_session, open_in_browser, PENDING_TARGET, TOS_VERSION, PRIVACY_VERSION}` — every one of them already in this file.
- Produces: `b64url()`, `pkce_pair()`, `authorize_url()`, `code_from_request_line()`, `CALLBACK_PAGE`, `NOT_FOUND_PAGE`, `serve_one_callback()`, `exchange_pkce_at()`, `post_no_reply()`, `record_consent_at()`, `checkout_url_at()`, and the command `google_sign_in`. Task 4's `verify_email_code` calls `record_consent_at` too; Task 5's page calls the command; Task 7's gate proves it live.

**No new crate.** `sha2 0.10` is already an `app/Cargo.toml` dependency (it verifies the runtime download), `ureq` is the one `agent()` builds, and `std::net::TcpListener` is the standard library. base64url is a table lookup written here.

- [ ] **Step 1: The PKCE tests first** — `app/tests/account.rs`:

```rust
use knowlu::account::{authorize_url, b64url, code_from_request_line, pkce_pair};

/// RFC 7636 Appendix B, verbatim. The one place in this file where a literal is not ours: it is the
/// standard's own worked example, and matching it is what says our challenge is a PKCE challenge
/// rather than a hash of something adjacent.
#[test]
fn the_challenge_is_rfc_7636_appendix_bs_worked_example() {
    use sha2::{Digest, Sha256};
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn base64url_is_unpadded_and_uses_the_url_alphabet() {
    // The three bytes that produce a `+` and a `/` in standard base64 — the two characters that
    // would be re-encoded or mis-read inside a query string, which is the whole reason for -url.
    assert_eq!(b64url(&[0xfb, 0xff, 0xbe]), "-_--");
    assert_eq!(b64url(&[]), "");
    assert_eq!(b64url(&[0x00]), "AA");
    assert_eq!(b64url(&[0x00, 0x00]), "AAA");
    assert!(!b64url(&[0x00]).contains('='), "no padding: the query string is not the place for it");
}

#[test]
fn a_verifier_is_43_characters_of_the_unreserved_alphabet_and_never_repeats() {
    let (v1, c1) = pkce_pair();
    let (v2, _) = pkce_pair();
    // RFC 7636 §4.1: 43 to 128 characters. 32 random bytes is 43 unpadded base64url characters.
    assert_eq!(v1.len(), 43);
    assert!(v1.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "{v1}");
    assert_eq!(c1.len(), 43);
    assert_ne!(v1, c1, "the challenge is the hash, never the verifier itself");
    assert_ne!(v1, v2, "a fresh pair every sign-in");
}

#[test]
fn the_authorize_url_names_google_pkce_and_the_loopback_port_and_nothing_else() {
    let url = authorize_url("https://abc.supabase.co/auth/v1", 54321, "CHAL");
    assert_eq!(
        url,
        "https://abc.supabase.co/auth/v1/authorize?provider=google\
         &redirect_to=http%3A%2F%2F127.0.0.1%3A54321%2Fcallback\
         &code_challenge=CHAL&code_challenge_method=s256"
            .replace(' ', "")
    );
    // `state` is in GoTrue's `reservedOAuthParams` and is stripped from the query before the
    // provider is called — sending one would be a line of code that does nothing (spec §4).
    assert!(!url.contains("state="), "{url}");
    // No `flow_type`: GoTrue infers PKCE from the presence of `code_challenge`.
    assert!(!url.contains("flow_type"), "{url}");
}

#[test]
fn the_callback_request_line_yields_the_code_or_the_providers_own_sentence() {
    assert_eq!(code_from_request_line("GET /callback?code=abc123 HTTP/1.1").unwrap(), "abc123");
    // Percent-decoded, because GoTrue query-encodes what it puts there.
    assert_eq!(code_from_request_line("GET /callback?code=a%2Bb HTTP/1.1").unwrap(), "a+b");
    let e = code_from_request_line("GET /callback?error=access_denied&error_description=You+said+no HTTP/1.1")
        .expect_err("an error is not a code");
    assert!(e.contains("You said no"), "{e}");
    assert!(code_from_request_line("GET /favicon.ico HTTP/1.1").is_err(), "no code, no session");
    assert!(code_from_request_line("garbage").is_err());
}
```

- [ ] **Step 2: The pure half** — `app/src/account.rs`, above the commands:

```rust
/// Base64url without padding (RFC 4648 §5), written here rather than taken as a crate: it is a
/// table lookup, and the workspace's crate budget is a product line. `-` and `_` instead of `+` and
/// `/` is the whole point — the challenge travels in a query string.
pub fn b64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        // 4 characters for 3 bytes, 3 for 2, 2 for 1 — and no `=`, which is what "unpadded" means.
        for i in 0..(chunk.len() + 1) {
            out.push(A[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
        }
    }
    out
}

/// A verifier and its challenge (RFC 7636 §4.1, §4.2). 32 bytes of OS entropy is 43 unpadded
/// base64url characters, the low end of the standard's 43..=128 range and the length every client
/// library uses.
///
/// **The entropy is `knowlu_engine::ids::new_id`'s**, five bytes at a time. `getrandom` is the
/// engine's dependency and not the app's, and seven calls to a function that already asks the OS is
/// a smaller change than a new crate or a new `windows` feature for one buffer. The hex it returns
/// is decoded back to bytes rather than used as text, so the verifier really is 256 bits and not
/// 256 bits' worth of hex digits.
pub fn pkce_pair() -> (String, String) {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::with_capacity(35);
    while bytes.len() < 32 {
        let id = knowlu_engine::ids::new_id("pkce");
        let hex = id.rsplit('_').next().unwrap_or_default().to_string();
        for pair in hex.as_bytes().chunks(2) {
            if let Ok(b) = u8::from_str_radix(&String::from_utf8_lossy(pair), 16) {
                bytes.push(b);
            }
        }
    }
    bytes.truncate(32);
    let verifier = b64url(&bytes);
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// The loopback redirect, percent-encoded as a query value. Only the six characters that appear in
/// `http://127.0.0.1:<port>/callback` need it, so this is not a general encoder and does not pretend
/// to be one.
fn redirect_to(port: u16) -> String {
    format!("http%3A%2F%2F127.0.0.1%3A{port}%2Fcallback")
}

/// `<auth>/authorize?…` — the URL the system browser is sent to.
///
/// **No `state`.** GoTrue owns it: a client-supplied `state` is deleted from the query before the
/// provider is called (`reservedOAuthParams`, `internal/api/external.go`), and the flow state it
/// creates instead is what carries the challenge across the round trip. **No `flow_type`** either:
/// the presence of `code_challenge` is what selects PKCE (`getFlowFromChallenge`).
pub fn authorize_url(auth_base: &str, port: u16, challenge: &str) -> String {
    format!(
        "{}/authorize?provider=google&redirect_to={}&code_challenge={challenge}&code_challenge_method=s256",
        auth_base.trim_end_matches('/'),
        redirect_to(port),
    )
}

/// Percent-decoding, for the one query value this file reads back.
fn pct_decode(s: &str) -> String {
    let b = s.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&String::from_utf8_lossy(&b[i + 1..i + 3]), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The authorisation code out of `GET /callback?code=… HTTP/1.1`, or the provider's own sentence.
///
/// Pure, and taking the request line rather than a socket, so the parsing is tested directly — the
/// shape `google_error_for_status` uses in this same file, and for the same reason. A request that
/// is not the callback (a browser's `/favicon.ico`, a probe) is an `Err`, never an empty `Ok`.
pub fn code_from_request_line(line: &str) -> Result<String, String> {
    let target = line.split_whitespace().nth(1).unwrap_or_default();
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("code", v)) => code = Some(pct_decode(v)),
            Some(("error_description", v)) => error = Some(pct_decode(v)),
            Some(("error", v)) if error.is_none() => error = Some(pct_decode(v)),
            _ => {}
        }
    }
    match (code, error) {
        (Some(c), _) if !c.is_empty() => Ok(c),
        (_, Some(e)) if !e.is_empty() => Err(e),
        _ => Err("the browser came back without a sign-in code".to_string()),
    }
}
```

- [ ] **Step 3: The listener — the test first**, in `app/tests/account.rs`:

```rust
use knowlu::account::{serve_one_callback, CALLBACK_PAGE};

/// A loopback listener on `127.0.0.1:0` is not the network: the same machine, the same process
/// tree, nothing that leaves it. The serving side is the production code; the client side is this
/// test's own thread, joined before the test returns (the 3a rule).
#[test]
fn the_callback_listener_serves_exactly_one_browser_and_answers_in_one_sentence() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let browser = std::thread::spawn(move || {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        s.write_all(b"GET /callback?code=xyz789 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .expect("write");
        let mut got = String::new();
        let _ = s.read_to_string(&mut got);
        got
    });
    let code = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect("a code");
    let page = browser.join().expect("browser thread");
    assert_eq!(code, "xyz789");
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("You are signed in to Knowlu. You can close this window."), "{page}");
    assert!(page.contains(CALLBACK_PAGE), "the served body is the constant, not a second copy");
}

#[test]
fn a_refusal_in_the_query_is_the_providers_sentence_and_the_browser_still_gets_a_page() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let browser = std::thread::spawn(move || {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        s.write_all(b"GET /callback?error=access_denied&error_description=You+said+no HTTP/1.1\r\n\r\n")
            .expect("write");
        let mut got = String::new();
        let _ = s.read_to_string(&mut got);
        got
    });
    let err = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect_err("a refusal");
    let page = browser.join().expect("browser thread");
    assert!(err.contains("You said no"), "{err}");
    // A browser left staring at a connection reset is a worse answer than a sentence.
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
}

#[test]
fn a_browser_that_never_comes_back_times_out_instead_of_holding_the_wizard_forever() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let out = serve_one_callback(listener, std::time::Duration::from_millis(150));
    let err = out.expect_err("nothing connected");
    assert!(err.contains("not finished"), "{err}");
}

/// **Review I3.** Anything on this machine may reach an open loopback port first: a browser
/// preconnect, a favicon fetch, security software, a port scanner, a second process. Answering it
/// and returning would end the sign-in with *the browser came back without a sign-in code* while the
/// real redirect was still in flight — and that redirect would then meet a closed socket. The
/// listener answers 404 and keeps waiting; the FIRST real callback is still the only one served.
#[test]
fn a_stray_local_connection_gets_404_and_the_real_callback_still_lands() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let noise_then_browser = std::thread::spawn(move || {
        let mut probe = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        probe.write_all(b"GET /favicon.ico HTTP/1.1\r\nConnection: close\r\n\r\n").expect("write");
        let mut probe_got = String::new();
        let _ = probe.read_to_string(&mut probe_got);
        let mut browser = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect again");
        browser
            .write_all(b"GET /callback?code=late123 HTTP/1.1\r\nConnection: close\r\n\r\n")
            .expect("write");
        let mut page = String::new();
        let _ = browser.read_to_string(&mut page);
        (probe_got, page)
    });
    let code = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect("the real code");
    let (probe_got, page) = noise_then_browser.join().expect("the client thread");
    assert_eq!(code, "late123", "the stray request must not consume the sign-in");
    assert!(probe_got.starts_with("HTTP/1.1 404"), "{probe_got}");
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("You are signed in to Knowlu."), "{page}");
}
```

- [ ] **Step 4: The listener** — `app/src/account.rs`:

```rust
/// What the browser is left looking at. One sentence, no styling, no script, no link back — the
/// student's next move is the Knowlu window that is already open behind it.
pub const CALLBACK_PAGE: &str =
    "<!doctype html><meta charset=\"utf-8\"><title>Knowlu</title>\
     <p style=\"font:16px system-ui;margin:3rem\">You are signed in to Knowlu. You can close this window.</p>";

/// The other page: anything on this machine that is not the sign-in.
pub const NOT_FOUND_PAGE: &str =
    "<!doctype html><meta charset=\"utf-8\"><title>Knowlu</title>\
     <p style=\"font:16px system-ui;margin:3rem\">Nothing here. You can close this window.</p>";

/// One reply, on a socket this function is finished with.
fn write_page(stream: &mut std::net::TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    let resp = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()
}

/// Serve exactly **one sign-in**: wait for the callback, answer it, and give the socket back to the
/// OS.
///
/// `listener` is taken **by value** on purpose: it is dropped when this returns, so there is no way
/// to leave a port open on a student's machine after a sign-in — successful, refused or abandoned.
/// The deadline is enforced by polling `accept` on a non-blocking listener rather than by a second
/// thread, so nothing outlives the call.
///
/// **It loops to the deadline rather than returning on the first connection** (review I3). An open
/// loopback port is reachable by everything else on the machine — a browser preconnect, a favicon
/// fetch, security software, a port scanner — and taking the first socket as the answer meant a
/// stray request ended the sign-in while the real redirect was still in flight, which then met a
/// closed port. Anything whose target is not `/callback` gets a 404 and the wait continues; the
/// first real callback returns, and the guarantee is unchanged, because the listener dies with this
/// call.
///
/// **The compensating control against a *forged* callback is the verifier, not the socket.** GoTrue
/// owns `state` — a client-supplied one is stripped (`reservedOAuthParams`) — so this cannot carry a
/// nonce of its own, and it does not need one: the verifier is minted per attempt and never leaves
/// the process, so a code minted under any other challenge fails the exchange. A local process that
/// guesses the port can therefore end a sign-in with a refusal sentence, and can never take one over.
///
/// The reply is sent on **every** path. A refusal is still a browser window a person is looking at,
/// and a connection reset is not an explanation.
pub fn serve_one_callback(listener: std::net::TcpListener, wait: std::time::Duration) -> Result<String, String> {
    use std::io::Read;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + wait;
    loop {
        let mut stream = match listener.accept() {
            Ok((s, _)) => s,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err("the Google sign-in was not finished — try again".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        // The request LINE is all this needs, and a browser sends it in the first packet. Reading to
        // the first newline rather than to EOF is also what keeps a keep-alive connection from hanging.
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        let line = head.lines().next().unwrap_or_default().to_string();
        let target = line.split_whitespace().nth(1).unwrap_or_default();
        if target.split('?').next().unwrap_or_default() != "/callback" {
            let _ = write_page(&mut stream, "404 Not Found", NOT_FOUND_PAGE);
            continue;
        }
        let _ = write_page(&mut stream, "200 OK", CALLBACK_PAGE);
        return code_from_request_line(&line);
    }
}
```

`code_from_request_line` keeps its `/favicon.ico` case and its test: the path check above decides which connection is the sign-in, and the parser still refuses a `/callback` with no `code` and no `error` in it.

- [ ] **Step 5: The exchange and the consent — the test first**, in `app/tests/account.rs`, on the existing `loopback()` helper:

```rust
use knowlu::account::{exchange_pkce_at, record_consent_at};

#[test]
fn the_code_and_the_verifier_are_traded_for_a_session_at_grant_type_pkce() {
    let body = r#"{"access_token":"atG","refresh_token":"rtG","expires_in":3600,"user":{"id":"acc-g","email":"g@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = exchange_pkce_at(&format!("{base}/auth/v1"), "anon-key", "the-code", "the-verifier", 1_760_000_000);
    let seen = handle.join().expect("server thread");
    let (id, s) = out.expect("exchange");
    assert_eq!(id, "acc-g");
    assert_eq!(s.access_token, "atG");
    assert_eq!(s.email, "g@example.invalid");
    assert_eq!(s.expires_at, 1_760_000_000 + 3600);
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/token?grant_type=pkce "), "{req}");
    assert!(req.to_lowercase().contains("apikey: anon-key"), "{req}");
    // GoTrue's `PKCEGrantParams` has exactly these two fields; `code` or `verifier` would be
    // silently empty and answer 400 `invalid request: both auth code and code verifier should be non-empty`.
    assert!(req.contains("\"auth_code\":\"the-code\""), "{req}");
    assert!(req.contains("\"code_verifier\":\"the-verifier\""), "{req}");
}

#[test]
fn a_bad_verifier_is_gotrues_own_sentence_and_never_a_status_code() {
    let (base, handle) = loopback(vec![(400, r#"{"error_description":"code challenge does not match previously saved code verifier"}"#.to_string())]);
    let out = exchange_pkce_at(&format!("{base}/auth/v1"), "anon-key", "c", "v", 0);
    let _ = handle.join();
    let e = out.expect_err("refused");
    assert!(e.contains("code challenge does not match"), "{e}");
    assert!(!e.contains("400"), "the panel reads a sentence, not a status: {e}");
}

#[test]
fn the_consent_call_carries_the_attestation_and_both_compiled_in_versions() {
    let (base, handle) = loopback(vec![(200, r#"{"ok":true}"#.to_string())]);
    let out = record_consent_at(&format!("{base}/functions/v1"), "the-token", "2026-09-10", "2026-09-17");
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{out:?}");
    let req = &seen[0];
    assert!(req.starts_with("POST /functions/v1/account/consent "), "{req}");
    assert!(req.to_lowercase().contains("authorization: bearer the-token"), "{req}");
    assert!(req.contains("\"age_attested\":true"), "{req}");
    assert!(req.contains("\"tos_version\":\"2026-09-10\""), "{req}");
    assert!(req.contains("\"privacy_version\":\"2026-09-17\""), "{req}");
}

/// **Review I4.** The consent call after a sign-in is best effort, so `open_checkout` retries it
/// **before** the checkout POST: an account whose attestation never landed meets
/// `billing-checkout`'s 403 — *the 18+ attestation is missing — sign in again* — and signing in
/// again would take the same failing path. The route writes only when the account has none, so the
/// retry costs one request and can never double-record. A retry that fails again is logged and does
/// not stand in the way: the 403 is the honest answer, and it is the server's to give.
#[test]
fn the_checkout_retries_the_consent_first_and_a_failed_retry_still_reaches_stripe() {
    use knowlu::account::checkout_url_at;
    let (base, handle) = loopback(vec![
        (500, r#"{"msg":"the consent route is down"}"#.to_string()),
        (200, r#"{"url":"https://checkout.example.invalid/c/cs_1"}"#.to_string()),
    ]);
    let out = checkout_url_at(&format!("{base}/functions/v1"), "the-token", "monthly");
    let seen = handle.join().expect("server thread");
    assert_eq!(out.expect("a checkout link"), "https://checkout.example.invalid/c/cs_1");
    assert!(seen[0].starts_with("POST /functions/v1/account/consent "), "the retry comes first: {}", seen[0]);
    assert!(seen[1].starts_with("POST /functions/v1/billing-checkout "), "{}", seen[1]);
}
```

- [ ] **Step 6: The exchange, the consent and the command** — `app/src/account.rs`:

```rust
/// The PKCE half of `/token`. GoTrue's `PKCEGrantParams` names `auth_code` and `code_verifier`
/// (`internal/api/token.go`) — not `code`, not `verifier` — and either one empty is a 400 with a
/// sentence about both being non-empty.
pub fn exchange_pkce_at(
    auth_base: &str,
    anon: &str,
    code: &str,
    verifier: &str,
    now_unix: i64,
) -> Result<(String, Session), String> {
    let body = json!({ "auth_code": code, "code_verifier": verifier });
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=pkce"), anon, &body)?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// **One authenticated POST**, returning the status and the parsed body. `post_for_url` and
/// `post_no_reply` are its two readings: a route that answers with a link to open, and a route whose
/// success is the status itself. Split out for review M5 — `record_consent_at` used to detect
/// success by string-matching `"no link came back"`, a literal private to `post_for_url` and free to
/// be reworded, which would have turned every recorded consent into a silent failure.
fn post_authed(api_base: &str, path: &str, token: &str, body: &Value) -> Result<(u16, Value), String> {
    check_api_base(api_base)?;
    let text = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut res = agent()
        .post(&format!("{}{path}", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .send(text)
        .map_err(|e| format!("{UNREACHABLE} ({e})"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// A route whose success has no link in it. (`post_for_url` keeps its own signature and its own
/// `"no link came back"`; both now go through [`post_authed`].)
fn post_no_reply(api_base: &str, path: &str, token: &str, body: &Value) -> Result<(), String> {
    let (status, v) = post_authed(api_base, path, token, body)?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

/// The 18+ attestation and the two policy versions, recorded after the session exists.
///
/// **Why it is a second call and not metadata.** `/authorize` has no field for user metadata, and
/// migration `20260917000100` stopped the trigger reading metadata on either path — a `/otp` request
/// is anyone's to send, so an attestation taken from one would be an `age_18` row nobody made. This
/// is the only writer of a consent row; `billing-checkout` refuses an account where `age_attested_at`
/// is still null, so skipping this call is not a way around the gate.
///
/// Called after **both** sign-in paths — `google_sign_in` and `verify_email_code` — and idempotent on
/// the service side, so it needs no "is this a new account" question the app has no honest way to
/// answer, and `open_checkout` can retry it for free.
pub fn record_consent_at(api_base: &str, token: &str, tos: &str, privacy: &str) -> Result<(), String> {
    let body = json!({ "tos_version": tos, "privacy_version": privacy, "age_attested": true });
    post_no_reply(api_base, "/account/consent", token, &body)
}

/// The checkout link, with the consent retry in front of it (review I4). Split from the command so
/// the order — consent, then Stripe — is a test rather than a claim; the command is what opens the
/// browser.
pub fn checkout_url_at(api_base: &str, token: &str, plan: &str) -> Result<String, String> {
    // Free when the first call worked (the route writes only where the account has none), and the
    // difference between a student who can subscribe and one whose only advice is "sign in again"
    // when it did not. A retry that fails again is logged, never fatal: `billing-checkout`'s 403 is
    // the honest answer and the server is the one entitled to give it.
    if let Err(e) = record_consent_at(api_base, token, TOS_VERSION, PRIVACY_VERSION) {
        eprintln!("Knowlu: the consent retry before checkout did not land ({e})");
    }
    post_for_url(api_base, "/billing-checkout", token, &json!({ "plan": plan, "terms_version": TOS_VERSION }))
}

/// **Continue with Google** (spec D1, D2). One loopback listener, one browser window, one exchange.
///
/// Vault-less, like `google_connect_url` beside it: the session goes to [`PENDING_TARGET`], which is
/// where `create_vault_in` looks for it at Finish and where the upgrade overlay's `attach_account`
/// looks for it over an existing vault. Every panel after this one is unchanged, because it cannot
/// tell this path from the emailed code.
///
/// **Nothing here reaches the page.** The verifier, the code and both tokens stay in this function
/// and in Credential Manager; what crosses the IPC is an account id and an email address, the same
/// envelope `verify_email_code` returns.
#[tauri::command(async)]
pub fn google_sign_in() -> Value {
    let api = api_base();
    let out = (|| -> Result<(String, Session), String> {
        let auth = auth_base(&api)?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("no local port for the sign-in ({e})"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let (verifier, challenge) = pkce_pair();
        open_in_browser(&authorize_url(&auth, port, &challenge))?;
        // Three minutes: long enough to pick an account and read a consent screen, short enough
        // that a wizard nobody came back to is not holding a socket at bedtime.
        let code = serve_one_callback(listener, std::time::Duration::from_secs(180))?;
        let (id, s) = exchange_pkce_at(&auth, &anon_key(), &code, &verifier, now_unix())?;
        save_session(PENDING_TARGET, &id, &s)?;
        // Best effort, and second — but never silent (review I4). The session is already on this
        // machine and an attestation that did not land is a subscribe step that says so, not a
        // sign-in to be redone; `open_checkout` retries it before the checkout POST. The log line is
        // `attach_in`'s shape, the file's existing way of saying "this part did not land"
        // (`account.rs:602`).
        if let Err(e) = record_consent_at(&api, &s.access_token, TOS_VERSION, PRIVACY_VERSION) {
            eprintln!("Knowlu: the sign-up consent could not be recorded ({e})");
        }
        Ok((id, s))
    })();
    match out {
        Ok((id, s)) => ok_account(&id, &s.email),
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}
```

…and `open_checkout` loses its two POST lines to the new helper, so the retry is on the one path the page uses:

```rust
#[tauri::command(async)]
pub fn open_checkout(plan: String) -> Value {
    let base = api_base();
    let out = (|| -> Result<String, String> {
        let auth = auth_base(&base)?;
        let token = valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, now_unix())?;
        let url = checkout_url_at(&base, &token, &plan)?;
        open_in_browser(&url)?;
        Ok(url)
    })();
    match out { Ok(_) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e }) }
}
```

**One note on the runtime cost** (review M9). `#[tauri::command(async)]` runs the body on a Tauri async-runtime worker, and this body can hold one for the whole 180-second deadline — six times longer than any command in this crate holds one today. It is acceptable and deliberate: the alternative is a command that returns immediately and an event the page has to subscribe to, which is a second protocol for one button. The console's own commands are short, the wizard has nothing else in flight while the browser is open, and the deadline is what bounds it.

- [ ] **Step 7: Green, then commit.** `cargo test --workspace` at 0 warnings. Commit `app/src/account.rs` and `app/tests/account.rs` by name. **Ask the controller to apply H1 before this task's own smoke run** — an unregistered command is rejected before its body runs.

---

### Task 4: The email path loses the password

**Files:**
- Modify: `app/src/account.rs`, `cloud/supabase/config.toml`
- Test: `app/tests/account.rs`, `app/tests/static_assets.rs` (the `sign_up` argument guard at `:890` — review C3), `cloud/supabase/functions/_shared/config_toml_test.ts` (the `config.toml` pins — review C5 as amended by R-C1b-5)

**Interfaces:**
- Removes: `sign_up_at`, `sign_in_at`, and the commands `sign_up` and `sign_in`. Nothing else in the crate calls them — `grep -rn "sign_up\|sign_in_at\|sign_in(" app/src` before deleting, and the only hits must be in this file and in `main.rs`'s two lists (hand-off H1).
- Produces: `magic_link_at` with `create_user: true` and **no consent keys at all** (R-C1b-3), and `verify_email_code` calling Task 3's `record_consent_at` after `save_session`.

Spec D4 and D5. Existing password accounts are not locked out: `/otp` to a confirmed address sends a code to that address, and Google links its identity to an existing verified email rather than creating a second user.

- [ ] **Step 1: The test first** — in `app/tests/account.rs`, replace the `sign_up`/`sign_in` tests with:

```rust
#[test]
fn the_code_request_creates_the_account_and_carries_no_consent_key_at_all() {
    use knowlu::account::magic_link_at;
    let (base, handle) = loopback(vec![(200, "{}".to_string())]);
    let out = magic_link_at(&format!("{base}/auth/v1"), "anon-key", "n@example.invalid");
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{out:?}");
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/otp "), "{req}");
    // `create_user: false` is what made a new student's first press answer "Signups not allowed for
    // otp". One field, one button, one code — there is no separate create step to fall back to.
    assert!(req.contains("\"create_user\":true"), "{req}");
    // R-C1b-3: nothing about consent travels with a request that anyone holding the public anon key
    // can send. `data` would land as `raw_user_meta_data`, and after migration 20260917000100 the
    // trigger reads none of it — the attestation is recorded by `POST /account/consent` once the
    // code has proved the address, on this path exactly as on Google's.
    for key in ["age_attested", "tos_version", "privacy_version", "\"data\""] {
        assert!(!req.contains(key), "{key} must not travel with /otp: {req}");
    }
    // And nothing that smells of a password, on any path in this file any more.
    assert!(!req.to_lowercase().contains("password"), "{req}");
}

/// The deletion, pinned. A dead command that can still make a password account is a second door.
#[test]
fn there_is_no_password_path_left_in_the_crate() {
    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    for gone in ["fn sign_up_at", "fn sign_in_at", "pub fn sign_up(", "pub fn sign_in(", "grant_type=password", "/signup"] {
        assert!(!src.contains(gone), "{gone} must be gone with the password");
    }
    // **NOT a ban on the word** (review C2). `load_session` reads `cred.password.expose()`
    // (`account.rs:130`) and `password` there is a field name on `knowlu_engine::wincred`'s
    // credential struct — engine-owned, and `engine/**` is not this stream's to edit, so the old
    // catch-all could never pass without deleting the one function that reads a stored session.
    // Two claims that are true AND load-bearing instead: no request body carries a password field,
    // and no function in this file takes one.
    assert!(!src.contains("\"password\""), "no request body may carry a password field");
    assert!(!src.contains("password: &str") && !src.contains("password: String"),
        "no function in account.rs takes a password");
}
```

Both new assertions depend on the prose being rewritten in the same step, so it is part of this step and not a tidy-up: `account.rs:181` quotes `` `"password"` `` as an example of what the loopback tests assert, and `:6`, `:135` and `:173` describe a password the product no longer has.

- **`:6`** — *"a command takes a password in and gives an envelope back"* becomes *"a command takes an email address or a six-digit code in and gives an envelope back"*.
- **`:135`** — *"with no way back but retyping a password they may have generated"* becomes *"with no way back but another emailed code or another trip through the browser"*.
- **`:173`** — *"the page has to tell \"your password is wrong\" from \"there is no network\""* becomes *"… tell \"that code is wrong\" from \"there is no network\""*.
- **`:181`** — the example substrings become `` `"type":"magiclink"` `` and `` `"create_user":true` ``, which is what the tests assert after this task.

- [ ] **Step 1b: The tests that pin the deleted functions go with them** (review M10, C3). In `app/tests/account.rs`, delete `a_successful_sign_in_returns_a_session_and_sends_the_anon_key` (`:102`), `a_refused_sign_in_is_the_providers_sentence_and_never_a_status_code` (`:141`) and `signing_up_sends_the_attestation_and_both_policy_versions_as_user_metadata` (`:149`, its `sign_up_at` call at `:152`), and narrow the `use` at `:3` to `use knowlu::account::{auth_base, check_api_base, Session};`. Then rewrite `app/tests/static_assets.rs:890` — **it pins `pub fn sign_up(email: String, password: String, age_attested: bool)` and `args.ageAttested = …` and dies with this task** (review C3) — keeping R-C1-55's trap over the command that still carries a multi-word argument:

```rust
/// **R-C1-55, C1 — Tauri v2 lower-camel-cases every argument key** (tauri-macros' `ArgumentCase::Camel`)
/// unless the command opts out with `rename_all = "snake_case"`. `send_magic_link` does not opt out, so
/// a page that sends `age_attested` is rejected *before* the command body runs; the handler's `.catch`
/// then paints `UNREACHABLE` — *the account service could not be reached* — and `wizValid`'s step-1 gate
/// refuses Next forever. **No new student could ever get a code**, and the sentence they were shown
/// blamed the network. C1b moved the trap one command over: `sign_up` is gone with the password, and
/// `send_magic_link(email, age_attested)` is now the crate's multi-word command argument.
#[test]
fn the_code_request_spells_its_argument_the_way_tauri_delivers_it() {
    let rust = fs::read_to_string("src/account.rs").expect("src/account.rs");
    // The signature itself, so a renamed or added parameter fails HERE and not in a student's first
    // five minutes.
    assert!(rust.contains("pub fn send_magic_link(email: String, age_attested: bool)"),
        "account::send_magic_link's signature changed — re-derive the keys the page must send");
    assert!(!rust.contains("rename_all"), "account.rs opts no command out of Tauri's camelCase");
    let js = read("console.js");
    // Either spelling of the assignment, because Task 5 is what moves it from the deleted
    // `args.ageAttested = …` into the `send_magic_link` invoke — and this test has to be green at
    // THIS task gate, not only at the next one. What it owns is the half that never changes: the
    // page says `ageAttested` and never `age_attested`, whichever branch carries it.
    assert!(js.contains("ageAttested"), "the attestation travels camel-cased");
    assert!(!js.contains("age_attested"), "the snake_case spelling must not appear on the page at all");
    // The two consent versions are `account.rs`'s constants and are stamped into the consent call
    // there (spec §9): a page that sent its own could make the consent log wrong.
    assert!(rust.contains("TOS_VERSION") && rust.contains("PRIVACY_VERSION"), "the versions are Rust's");
    for own in ["tos_version", "tosVersion", "privacy_version", "privacyVersion"] {
        assert!(!js.contains(own), "the consent versions are Rust's, never the page's: {own}");
    }
}
```

The scan at `static_assets.rs:956` (`checked >= 6` multi-word command arguments) is unaffected: it counts arguments across the crate, and this task removes two commands and keeps `send_magic_link`'s.

- [ ] **Step 2: The implementation** — in `app/src/account.rs`:

```rust
/// One button: the code that both creates the account and signs in. `create_user: true` is the whole
/// difference from C1's version — and **nothing about consent travels with it** (R-C1b-3). `data`
/// would land as `raw_user_meta_data`, and migration `20260917000100` stopped the trigger reading
/// it, precisely because this request is one anyone holding the public anon key can send for any
/// address they like. The attestation is recorded after the code is typed, by `POST /account/consent`.
pub fn magic_link_at(auth_base: &str, anon: &str, email: &str) -> Result<(), String> {
    let (status, v) = post_json(&format!("{auth_base}/otp"), anon, &json!({ "email": email, "create_user": true }))?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

/// The attestation is still refused **here**, before a single mail is sent — it is the consent the
/// wizard's two checkboxes stand for. What changed is where it is recorded: in the account, by the
/// route, once the code has proved the address.
#[tauri::command(async)]
pub fn send_magic_link(email: String, age_attested: bool) -> Value {
    if !age_attested {
        return json!({ "ok": false, "error": "Knowlu is for people 18 or older." });
    }
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    match magic_link_at(&auth, &anon, email.trim()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
```

…and delete `sign_up_at`, `sign_in_at`, `sign_up` and `sign_in` outright, with their doc comments. `verify_email_code_at` is untouched — `VERIFY_TYPE` is still `magiclink`, settled against staging on 2026-09-10, and the code in the mail is still GoTrue's `{{ .Token }}` — but the **command** gains the consent call, in the same place and the same shape `google_sign_in` has it (R-C1b-3: the route is the one consent path, so both callers make the call):

```rust
#[tauri::command(async)]
pub fn verify_email_code(email: String, code: String) -> Value {
    let (auth, anon, base) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match verify_email_code_at(&auth, &anon, email.trim(), &code, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => {
                // The same call `google_sign_in` makes, for the same reason and with the same
                // handling: the trigger records no consent on either path any more, this is the one
                // writer, and a failure is logged rather than swallowed — `open_checkout` retries it
                // and `billing-checkout` refuses an account that still has no attestation.
                if let Err(e) = record_consent_at(&base, &s.access_token, TOS_VERSION, PRIVACY_VERSION) {
                    eprintln!("Knowlu: the sign-up consent could not be recorded ({e})");
                }
                ok_account(&id, &s.email)
            }
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}
```

(`env_pair()` already returns the API base as its third member — `account.rs:296` — so the consent call needs no second `api_base()` read and no new helper.)

  Add the matching test to `app/tests/account.rs` in this step, beside Task 3's: `record_consent_at` is what Task 3 pinned on the wire, so what is new here is only that this path calls it. Assert it where it is cheap — the static pin over `src/account.rs` — rather than by standing up a Credential Manager write inside a loopback test:

```rust
/// R-C1b-3: one route, both paths. A `verify_email_code` that saved a session and recorded nothing
/// would leave the emailed-code student with a null attestation and a 403 at the subscribe step.
#[test]
fn both_sign_in_paths_record_the_consent() {
    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    for owner in ["pub fn google_sign_in(", "pub fn verify_email_code("] {
        let body = src.split(owner).nth(1).expect(owner);
        let body = body.split("\n#[tauri::command").next().unwrap_or(body);
        assert!(body.contains("record_consent_at("), "{owner} must record the consent it just took");
    }
}
```

- [ ] **Step 3: `config.toml`** — `[auth.email] enable_confirmations` from `true` to `false`, with the reason beside it:

```toml
# **FALSE from C1b (spec D5), and it is what makes the one-button email path real.** `/otp` with
# `create_user: true` against an address with no CONFIRMED user hands the request to `Signup` and,
# with confirmations on, stops there — GoTrue's own comment reads "otherwise confirmation email
# already contains 'magic link'" (internal/api/magic_link.go). That sends the CONFIRMATION template,
# and `templates/` holds exactly one file: `magic_link.html`. A new student would get Supabase's
# stock "Confirm your signup" mail, which carries `{{ .ConfirmationURL }}` and no `{{ .Token }}` —
# a link a desktop app can never receive and no code to type. With confirmations off the same
# handler signs the user up and re-enters `MagicLink`, which sends OUR template, code first.
#
# Nothing is weakened by it: there is no password on a Knowlu account any more (spec D4), so the
# code sent to the address IS the proof of the address, at the only moment that proof matters.
enable_confirmations = false
```

`double_confirm_changes = true` stays: an email CHANGE is a different question and still needs both addresses to agree.

  …and the comment at `config.toml:31-35` is rewritten in the same step (review R6b), because its first sentence — *"R-C1-3, second half. `enable_confirmations = true` means every sign-up depends on this mail arriving"* — is stale the moment the line above it flips. The SMTP block's reason has not gone away, it has moved: **`enable_confirmations = false` means every sign-up depends on the CODE mail arriving**, which is the same dependency on the same provider, and Supabase's built-in sender is still a few messages an hour and still not for production. Keep the rest of the block (Resend, the verified domain, `SMTP_PASSWORD` never in this file) word for word.

  …and the rate limit that bounds what D5 costs (review I2) — **a sibling TOML table, not a key inside `[auth.email]`** (review R3). It goes **after `double_confirm_changes = true` (`config.toml:29`) and before the SMTP comment block at `:31`**, at the same level as `[auth]` and `[auth.email]`. Dropped in where the old wording pointed — straight under `enable_confirmations` — the `[auth.rate_limit]` header would swallow `double_confirm_changes` into the rate-limit table, and `config push` would either fail or push an `[auth.email]` that had quietly lost a setting:

```toml
# **Set here, explicitly, rather than left at the platform default** (review I2). With no password in
# the product, `POST /auth/v1/otp` with `create_user: true` is in practice an unauthenticated
# endpoint — `DEFAULT_ANON_KEY` ships in every build — and with confirmations off GoTrue creates the
# `auth.users` row before the code is ever typed. This number is the whole of the cap on that.
#
# **20 an hour, project-wide.** Sign-in happens once per student per device and the pilot is tens of
# students, so a group onboarding together — with a mistyped address and a retry each — still fits,
# while a stranger holding the public key gets at most 20 empty accounts an hour and, since migration
# 20260917000100 records no consent at all from a sign-up, **zero** fabricated consent rows: an
# `accounts` row with a null attestation, which `billing-checkout` refuses.
#
# The trade-off runs both ways and is worth saying out loud: the same 20 is the budget a flood would
# spend, and a spent budget is a real student who cannot sign in for the rest of the hour. The
# Supabase-native answer to that is `[auth.captcha]`, which is a separate decision (recorded in this
# plan's resolutions), not a bigger number here. Raising it is one line and one `config push`.
[auth.rate_limit]
email_sent = 20
```

  …and one comment, and **no block**, for the Google provider (**R-C1b-5**, 2026-09-17 — this replaces the review's C5 fix):

```toml
# **The Google provider is the DASHBOARD's, on both projects.** Quinn enabled it there for staging
# and for prod on 2026-09-17, and it was verified without holding a key: `/auth/v1/authorize?provider=google`
# answers 302 to `accounts.google.com` on each project, carrying that project's own
# `/auth/v1/callback` as `redirect_uri` and `email profile` as the scope. So this file declares **no**
# `[auth.external.google]` block on purpose: the client id and the secret never enter this repo, this
# file, or any shell in this session. `supabase config push` "pushes the properties your local
# config.toml declares … properties the file does not declare are left unchanged" (CLI 2.117.0), so
# pushing the two settings above leaves the dashboard's provider exactly as it is — and `supabase
# config diff` before each push is what proves it (Task 7 step 3).
```

- [ ] **Step 3b: The `config.toml` pins** — `cloud/supabase/functions/_shared/config_toml_test.ts` already reads this file (it is where the `verify_jwt` guard lives, and `CONFIG_TOML` is resolved at `:4`), so the new decisions are pinned there rather than in a new file:

```typescript
Deno.test("config.toml declares no auth provider, and says why — the Google provider is the dashboard's", async () => {
  // R-C1b-5. A block here would put a client id in the repo and need `GOOGLE_SECRET` in every
  // pushing shell — and a push from a shell without it would DISABLE a provider that works. The
  // file therefore declares none, and the comment is what a reviewer reads instead: "we chose not
  // to write a block" is otherwise invisible on a diff.
  const toml = await Deno.readTextFile(CONFIG_TOML);
  assert(!/^\[auth\.external\./m.test(toml), "no auth provider may be declared in the repo");
  assert(
    toml.includes("The Google provider is the DASHBOARD's, on both projects"),
    "…and the file has to say so, or the next reader adds one",
  );
  // The other two halves of the same decision: D5, and the number that bounds what D5 costs (I2).
  assert(toml.includes("enable_confirmations = false"), "D5: email confirmations are off");
  assert(/^\[auth\.rate_limit\]/m.test(toml), "[auth.rate_limit] is declared, not defaulted");
  assert(/^email_sent = \d+$/m.test(toml), "…with a number on the diff");
  // **And the table starts AFTER `double_confirm_changes`** (review R3). A `[auth.rate_limit]`
  // header placed one line too early does not fail to parse — it silently adopts the key below it,
  // so `[auth.email]` loses `double_confirm_changes` and a push carries that loss to the project.
  assert(
    toml.indexOf("double_confirm_changes") < toml.indexOf("[auth.rate_limit]"),
    "[auth.rate_limit] must not capture double_confirm_changes",
  );
});
```

- [ ] **Step 4: Green, then commit.** `cargo test --workspace` at 0 warnings, **and `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`, `deno lint` and `deno fmt --check`** — step 3b's assertions are this task's own test and must be green at this task's gate, not first executed at exit gate 11 (review R4), exactly as Tasks 1 and 2 run theirs. Commit `app/src/account.rs`, `app/tests/account.rs`, `app/tests/static_assets.rs`, `cloud/supabase/config.toml` and `cloud/supabase/functions/_shared/config_toml_test.ts` by name. The page still calls `send_magic_link` with one argument at this point and will fail on the new `ageAttested` — Task 5 is what fixes it, and the two land in the same branch.

---

### Task 5: The account panel and the upgrade overlay — Continue with Google first, one email field, one button

**Files:**
- Modify: `app/static/index.html`, `app/static/console.js`, `app/static/console.css`
- Test: `app/tests/static_assets.rs`
- Hand-off: **H2** (`scripts/wizard-check.py`), applied by the controller during this task

**Interfaces:**
- Consumes: Task 3's `google_sign_in`, Task 4's `send_magic_link(email, ageAttested)`.
- Produces: `#wiz-google-signin`; and in the upgrade overlay `#up-google`, `#up-magic`, `#up-code-row`, `#up-code`, `#up-code-go` (review I6 — the overlay had no email-code path at all, and Task 5 is what gives it one). `#wiz-pw`, `#up-pw`, `#wiz-create`, `#wiz-signin`, `#up-create` and `#up-signin` are gone.

**Tauri v2 lower-camel-cases every argument key** (`tauri-macros`' `ArgumentCase::Camel`) unless the command opts out, and exactly one command in this crate does. `send_magic_link`'s new second argument is therefore `ageAttested` on the page — sent snake_case the invoke is rejected before the body runs and the `.catch` paints a transport error for a call that never left the process. C1's R-C1-55 C1 is the same trap, one command over.

- [ ] **Step 1: The static pins first** — in `app/tests/static_assets.rs`, rewrite `the_account_panel_gates_on_eighteen_and_links_both_policies` and tighten the password allow-list:

```rust
#[test]
fn the_account_panel_leads_with_google_asks_for_no_password_and_still_gates_on_eighteen() {
    let html = read("index.html");
    let panel = html.split("id=\"wiz-account\"").nth(1).and_then(|s| s.split("id=\"wiz-subscribe\"").next()).expect("the account panel");
    // Spec D1: the Google button is the FIRST control on the panel, not an alternative buried
    // under a form. Proved by position, because "present" is not the claim.
    let g = panel.find("id=\"wiz-google-signin\"").expect("the Google button");
    let e = panel.find("id=\"wiz-email\"").expect("the email field");
    assert!(g < e, "Continue with Google comes before the email field");
    assert!(panel.contains("Continue with Google"), "…and says so in words");
    // Spec D4: no password, anywhere on this panel or in the overlay.
    assert!(!panel.contains("password"), "the account panel must never carry a password field again");
    assert!(!html.contains("id=\"wiz-pw\"") && !html.contains("id=\"up-pw\""), "both account passwords are gone");
    // One button for the email path. The create/sign-in split went with the password.
    assert!(panel.contains("id=\"wiz-magic\"") && panel.contains("Email me a code"), "one button, and it says what it does");
    assert!(!panel.contains("id=\"wiz-create\"") && !panel.contains("id=\"wiz-signin\""), "no create/sign-in split");
    assert!(panel.contains("id=\"wiz-code-row\"") && panel.contains("id=\"wiz-code-go\""), "the code row stays");
    // §9's minors row and the consent log are unchanged: both boxes, both policies, both gates.
    assert!(panel.contains("id=\"wiz-18\"") && panel.contains("18 or older"));
    assert!(panel.contains("id=\"wiz-terms\"") && panel.contains("terms.html") && panel.contains("privacy.html"));
    let js = read("console.js");
    assert!(js.contains("\"google_sign_in\"") && js.contains("\"send_magic_link\"") && js.contains("\"verify_email_code\""));
    assert!(!js.contains("\"sign_up\"") && !js.contains("\"sign_in\""), "the password commands are gone from the page too");
    // Tauri v2 camel-cases argument keys; `age_attested` here is a rejected invoke and a wizard
    // whose Next never unlocks.
    assert!(js.contains("ageAttested:"), "send_magic_link carries the attestation as ageAttested");
    assert!(js.contains("Tick both boxes"), "the page still says why a sign-in was refused");
}
```

…and a second test, over the overlay, because the console window's surface is not the wizard's and nothing else pins it (review I6):

```rust
/// Spec §6, last line: the upgrade overlay gets the same two doors. It is a **second** sign-in
/// surface, in the console window, over an existing vault — and it is not painted by `renderWizard`,
/// so the `WIZ.busy` guard that protects `#wiz-google-signin` does not reach it.
#[test]
fn the_upgrade_overlay_offers_the_same_two_doors_and_no_password() {
    let html = read("index.html");
    let panel = html.split("id=\"upgrade\"").nth(1).and_then(|s| s.split("</aside>").next()).expect("the upgrade overlay");
    let g = panel.find("id=\"up-google\"").expect("the overlay's Google button");
    let e = panel.find("id=\"up-email\"").expect("the overlay's email field");
    assert!(g < e, "Continue with Google comes first here too");
    assert!(!panel.contains("password"), "the overlay must never carry a password field again");
    for id in ["up-magic", "up-code-row", "up-code", "up-code-go"] {
        assert!(panel.contains(&format!("id=\"{id}\"")), "the overlay needs #{id}");
    }
    assert!(!html.contains("id=\"up-create\"") && !html.contains("id=\"up-signin\""), "no create/sign-in split");
    let js = read("console.js");
    let listener = js.split("EL(\"upgrade\").addEventListener(\"click\"").nth(1)
        .and_then(|s| s.split("function finishUpgrade(").next()).expect("the upgrade listener");
    for sel in ["#up-google", "#up-magic", "#up-code-go"] {
        assert!(listener.contains(sel), "the overlay's listener must handle {sel}");
    }
    // Two presses on a button that opens a browser are two listeners, two loopback ports and two
    // tabs. The wizard's guard is `WIZ.busy`, painted by renderWizard; the overlay carries its own.
    assert!(js.contains("function upBusy("), "the overlay has a busy guard of its own");
    assert!(listener.contains("UP_BUSY"), "…and the listener reads it before starting a sign-in");
    // **And the flag is declared OUTSIDE the listener** (review R2). Declared inside, it is
    // re-initialised to `false` on every press and guards nothing — while both assertions above
    // still pass. So the claim is about position: `var UP_BUSY` appears in the text BEFORE
    // `EL("upgrade").addEventListener`, where `UPGRADE_DISMISSED` and `UPGRADE_UNREACHABLE` live.
    let before = js.split("EL(\"upgrade\").addEventListener(\"click\"").next().expect("the file before the listener");
    assert!(before.contains("var UP_BUSY"), "UP_BUSY must be declared at IIFE scope, not inside the click handler");
}
```

…and in `the_page_has_no_lms_credential_field_anywhere`, `const OURS: [&str; 4] = ["wiz-pw", "up-pw", "wiz-zy-pass", "wiz-vhl-pass"];` (`static_assets.rs:384`) becomes `const OURS: [&str; 2] = ["wiz-zy-pass", "wiz-vhl-pass"];` — **the length annotation moves with the list** (review R6a), or the file does not compile — `seen >= 3` becomes `assert_eq!(seen, 2, "the two coursework logins are the only passwords Knowlu ever asks for")`, and the `for id in [...]` loop drops `wiz-pw`.

- [ ] **Step 1b: The two id lists that pin what this task deletes** (review C3) — both are shipped tests that go red the moment step 2 lands, and neither is optional:
  - `static_assets.rs:980`, in `every_control_this_task_added_is_in_the_markup_and_named_by_the_page`: `"upgrade", "up-email", "up-pw", "up-18", "up-terms", "up-create", "up-signin",` becomes `"upgrade", "up-email", "up-google", "up-magic", "up-code-row", "up-code", "up-code-go", "up-18", "up-terms",` — every one of the new ids is in `index.html` and named by `console.js`, which is what that test asks of each.
  - `static_assets.rs:1032`, in `the_console_can_sign_an_existing_install_in_without_re_onboarding_it`: the `for id in [...]` list drops `"up-pw"`, `"up-create"` and `"up-signin"` and gains `"up-google"`, `"up-magic"`, `"up-code-row"`, `"up-code"`, `"up-code-go"`. The rest of that test — the dismiss control, the two `class="policy"` links, `<aside class="setpanel" id="upgrade">`, `UPGRADE_DISMISSED`, `UPGRADE_UNREACHABLE` — is unchanged and must stay passing.

- [ ] **Step 2: The markup** — in `app/static/index.html`, the account panel becomes:

```html
  <div class="wiz-panel" id="wiz-account" hidden><h2>Your account</h2>
    <label><input type="checkbox" id="wiz-18"> I am 18 or older</label>
    <label><input type="checkbox" id="wiz-terms"> I accept the <a href="terms.html" class="policy" data-policy="terms" id="wiz-terms-link">terms</a> and the <a href="privacy.html" class="policy" data-policy="privacy" id="wiz-privacy-link">privacy policy</a></label>
    <div class="wiz-row"><button class="b pri y" id="wiz-google-signin">Continue with Google</button></div>
    <p class="meta">Knowlu asks Google for your email address and your name, and nothing else. Your calendar and your mail are separate permissions, asked for later, and you can say no to both.</p>
    <div class="wiz-row"><input type="text" id="wiz-email" placeholder="Or use your email address"><button class="b" id="wiz-magic">Email me a code</button></div>
    <div class="wiz-row" id="wiz-code-row" hidden><input type="text" id="wiz-code" placeholder="The 6-digit code from the email"><button class="b pri y" id="wiz-code-go">Sign in with the code</button></div>
    <p class="meta" id="wiz-account-note"></p>
  </div>
```

…and the upgrade overlay (`index.html:174-182`) becomes, in full — it has **no** email-code path today, so this is new markup, not a rename (review I6):

```html
<aside class="setpanel" id="upgrade" hidden>
  <div class="set-hd"><h2>Knowlu now needs an account</h2><button class="b" id="up-later">Not now</button></div>
  <p class="meta">Your work stays exactly where it is. Sign in and Knowlu picks up where it left off. Today&rsquo;s page is underneath and still works &mdash; ranking never needed an account.</p>
  <label><input type="checkbox" id="up-18"> I am 18 or older</label>
  <label><input type="checkbox" id="up-terms"> I accept the <a href="terms.html" class="policy" data-policy="terms">terms</a> and the <a href="privacy.html" class="policy" data-policy="privacy">privacy policy</a></label>
  <div class="set-row"><button class="b pri y" id="up-google">Continue with Google</button></div>
  <div class="set-row"><input type="text" id="up-email" placeholder="Or use your email address"><button class="b" id="up-magic">Email me a code</button></div>
  <div class="set-row" id="up-code-row" hidden><input type="text" id="up-code" placeholder="The 6-digit code from the email"><button class="b pri y" id="up-code-go">Sign in with the code</button></div>
  <div class="set-row"><button class="b pri y" id="up-subscribe" hidden>Subscribe</button><span class="crit" id="up-error"></span></div>
</aside>
```

Unchanged on purpose, because `static_assets.rs:1029-1061` pins each: the `<aside class="setpanel" id="upgrade">` shape, `#up-later`, exactly two `class="policy"` links, `#up-subscribe`, `#up-error`, and no word "folder" anywhere in it.

- [ ] **Step 3: The page's handlers** — in `app/static/console.js`'s `EL("wizard")` click listener, the `#wiz-create`/`#wiz-signin` branch (today at lines 1712-1733) is deleted and replaced by:

```javascript
    // Spec D1. One command, no arguments: the URL, the loopback port, the verifier and both tokens
    // are Rust's, and the page never sees any of them. The browser round trip can take a minute, so
    // the button says what is happening — `WIZ.busy` is what renderWizard paints (Task 6).
    if (e.target.closest("#wiz-google-signin")) {
      if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
        WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
      }
      WIZ.busy = true; WIZ.error = ""; WIZ.accountNote = "Finish signing in, in your browser…";
      renderWizard();
      invoke("google_sign_in", {}).then(function (r) {
        WIZ.busy = false; WIZ.accountNote = "";
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        wizGo(2);
      }).catch(function () { WIZ.busy = false; WIZ.accountNote = ""; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
```

…and the `#wiz-magic` branch gains the same gate and the camel-cased second argument:

```javascript
    if (e.target.closest("#wiz-magic")) {
      if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
        WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
      }
      // **`ageAttested`, not the Rust spelling.** Tauri v2 camel-cases every argument key; sent
      // snake_case the invoke is rejected before the command's body runs.
      invoke("send_magic_link", { email: EL("wiz-email").value.trim(), ageAttested: true }).then(function (r) {
        WIZ.error = r.ok ? "" : r.error;
        EL("wiz-code-row").hidden = !r.ok;
        WIZ.accountNote = r.ok ? "We emailed you a 6-digit code. Type it below." : "";
        renderWizard();
      }).catch(function () { WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
```

`#wiz-code-go` is unchanged. `WIZ` gains two fields beside `error`: `busy: false` and `accountNote: ""` — **state on `WIZ`, painted by `renderWizard`**, which is the file's own A-5 rule and the reason the Google panel's status line was moved there in C2. `renderWizard` paints it:

```javascript
    EL("wiz-account-note").textContent = WIZ.accountId ? "Signed in as " + WIZ.email : WIZ.accountNote;
    EL("wiz-google-signin").disabled = WIZ.busy;
```

The console's own upgrade-overlay handlers are replaced the same way — but **written out here**, because the overlay is a second sign-in surface with no `renderWizard` behind it (review I6). The `#up-create`/`#up-signin` branch (`console.js:1252-1273`) goes, and what replaces it lands in **two different scopes**:

**(a) At IIFE scope, beside `upgradeUnreachable()` (`console.js:1237`) and before `EL("upgrade").addEventListener` — never inside the listener.** A flag declared inside the handler is re-initialised to `false` on every press, which is not a guard at all: two presses on Continue with Google would again be two commands, two loopback listeners and two browser tabs (review R2). `UPGRADE_DISMISSED` and `UPGRADE_UNREACHABLE` already live at that scope for the same reason.

```javascript
  /** The overlay is the console window's own surface and nothing paints it, so its busy state is one
   *  flag and two writes rather than a `WIZ` field. Without it, two presses on Continue with Google
   *  are two commands, two loopback listeners and two browser tabs — and the second callback meets a
   *  closed port. */
  var UP_BUSY = false;
  function upBusy(on) {
    UP_BUSY = on;
    EL("up-google").disabled = on;
    EL("up-magic").disabled = on;
  }
  /** What both doors do once a session exists — the tail the old branch ended with, now that two
   *  branches share it. */
  function afterUpgradeSignIn() {
    EL("up-error").textContent = "";
    EL("up-code-row").hidden = true;
    EL("up-subscribe").hidden = false;
    return invoke("entitlement_now", {}).then(function (ent) {
      if (ent.ok && (ent.status === "active" || ent.status === "trialing")) { return finishUpgrade(); }
    });
  }
```

**(b) Inside the same `EL("upgrade").addEventListener("click", …)`, after the `a.policy` branch** — the three branches, and only the three branches:

```javascript
    if (e.target.closest("#up-google")) {
      if (UP_BUSY) { return; }
      if (!(EL("up-18").checked && EL("up-terms").checked)) {
        EL("up-error").textContent = "Tick both boxes to create an account."; return;
      }
      upBusy(true);
      EL("up-error").textContent = "Finish signing in, in your browser…";
      invoke("google_sign_in", {}).then(function (r) {
        upBusy(false);
        // A dead network stands the overlay down rather than trapping someone behind it (D4).
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        if (!r.ok) { EL("up-error").textContent = r.error; return; }
        return afterUpgradeSignIn();
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
    if (e.target.closest("#up-magic")) {
      if (UP_BUSY) { return; }
      if (!(EL("up-18").checked && EL("up-terms").checked)) {
        EL("up-error").textContent = "Tick both boxes to create an account."; return;
      }
      upBusy(true);
      // **`ageAttested`, not the Rust spelling** — Tauri v2 camel-cases every argument key, and the
      // wizard's own call carries the same comment for the same reason.
      invoke("send_magic_link", { email: EL("up-email").value.trim(), ageAttested: true }).then(function (r) {
        upBusy(false);
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        EL("up-error").textContent = r.ok ? "We emailed you a 6-digit code. Type it below." : r.error;
        EL("up-code-row").hidden = !r.ok;
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
    if (e.target.closest("#up-code-go")) {
      if (UP_BUSY) { return; }
      upBusy(true);
      invoke("verify_email_code", { email: EL("up-email").value.trim(), code: EL("up-code").value.trim() }).then(function (r) {
        upBusy(false);
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        if (!r.ok) { EL("up-error").textContent = r.error; return; }
        return afterUpgradeSignIn();
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
```

`verify_email_code` is already in the console window's `generate_handler!` list, so the overlay's code row needs no hand-off of its own; `google_sign_in` is what H1 adds there.

- [ ] **Step 4: The CSS** — `console.css` gains one rule beside the two that already exist for `.flagpop` and `.set-row`:

```css
/* Spec §7 defect 1: the wizard's nav had NO disabled rule, so a disabled Back or Next was full
   contrast, full colour and silently inert — which is what "why are there even back and next
   buttons if they don't work" was looking at. */
.wiz-nav button.b[disabled], .wiz-row button.b[disabled] { opacity: .5; cursor: not-allowed; }
.wiz-nav button.b[disabled]:hover, .wiz-row button.b[disabled]:hover { border-color: var(--hair-2); color: var(--t2); }
/* Task 6 hides Back at step 0 instead of disabling it, and `button.b` (line 180) sets no `display` —
   so the UA stylesheet's `[hidden]` is enough today. This rule is here anyway, because this file has
   taught the same lesson three times (`.app[hidden]`, `.drawer[hidden]`, `.wiz-row[hidden]`): the
   day someone gives `.wiz-nav button` a `display`, a hidden Back comes back. (Review M8.) */
.wiz-nav button.b[hidden] { display: none; }
/* …and the same lesson, live rather than pre-emptive: `.set-row` IS `display: flex` (line 470) and
   the sheet has no `.set-row[hidden]` rule, so the overlay's new `#up-code-row` would show through
   its own `hidden` attribute without this. */
.set-row[hidden] { display: none; }
```

- [ ] **Step 5: Hand-off H2, applied by the controller** — `scripts/wizard-check.py`. Check 2's body becomes:

```python
    # 2. Panel 2 is the account. Continue with Google leads it, there is no password field anywhere,
    #    and neither door opens until both boxes are ticked (spec §9's minors row).
    page.click("#wiz-next"); page.wait_for_timeout(120)
    if page.is_hidden("#wiz-account"): bad.append("Next did not reach the account panel")
    if page.query_selector("#wiz-pw"): bad.append("the account panel still has a password field")
    if not page.query_selector("#wiz-google-signin"): bad.append("there is no Continue with Google button")
    page.click("#wiz-google-signin"); page.wait_for_timeout(200)
    if "google_sign_in" in names(page): bad.append("a Google sign-in ran with the boxes unticked")
    if "Tick both" not in page.inner_text("#wiz-error"): bad.append("the refusal said nothing about the boxes")
    page.check("#wiz-18"); page.check("#wiz-terms")
    page.click("#wiz-google-signin"); page.wait_for_timeout(300)
    if "google_sign_in" not in names(page): bad.append("google_sign_in was not invoked")
    if page.is_hidden("#wiz-subscribe"): bad.append("a signed-in account did not advance to the subscribe panel")
```

…with `BEFORE_FINISH_OK` and the fake's branches as H2 gives them. Check 8's Back-and-forward walk is **not** touched.

- [ ] **Step 6: Green, then commit.** `cargo test --workspace` at 0 warnings, and `.wv\Scripts\python scripts/wizard-check.py` prints `ok`. Commit `app/static/{index.html,console.js,console.css}` and `app/tests/static_assets.rs` by name; the controller commits `scripts/wizard-check.py`.

---

### Task 6: Back always goes back, and Next either moves or says what is missing

**Files:**
- Modify: `app/static/index.html`, `app/static/console.js`
- Test: `app/tests/static_assets.rs`

Spec §7 names three real defects. Defect 1 (a disabled button that looks live) was fixed by Task 5's CSS rule. This task is defects 2 and 3.

- [ ] **Step 1: The test first** — `app/tests/static_assets.rs`:

```rust
/// Spec §7. Quinn, 2026-09-17: "why are there even back and next buttons if they don't work?"
/// Three answers, and this pins all three.
#[test]
fn the_wizards_nav_is_rendered_state_and_never_a_dead_control() {
    let js = read("console.js");
    let render = js.split("function renderWizard(").nth(1).and_then(|s| s.split("\n  }").next()).expect("renderWizard");
    // (a) Back is never `disabled` — at step 0 it is simply not there, so there is no dead control
    // to press. `hidden` on a button the UA stylesheet hides is enough; `disabled` was the bug.
    assert!(render.contains("EL(\"wiz-back\").hidden = WIZ.step === 0"), "Back is hidden at step 0, never disabled");
    assert!(!render.contains("EL(\"wiz-back\").disabled"), "Back is never disabled anywhere");
    // (b) Next's disabled state is a WIZ field renderWizard paints, like every other wizard field.
    // Set only by a direct DOM write, it survived every re-render that did not re-set it — which is
    // a Finish that resolved without relaunching leaving Next dead forever.
    assert!(render.contains("EL(\"wiz-next\").disabled = WIZ.busy"), "Next's disabled state is rendered from WIZ.busy");
    let go = js.split("function wizGo(").nth(1).and_then(|s| s.split("function wizRegister(").next()).expect("wizGo");
    let fin = js.split("function wizFinish(").nth(1).and_then(|s| s.split("\n  // The Checkout page").next()).expect("wizFinish");
    for (name, body) in [("wizGo", go), ("wizFinish", fin)] {
        assert!(!body.contains("EL(\"wiz-next\").disabled"), "{name} sets WIZ.busy, never the DOM property directly");
        assert!(body.contains("WIZ.busy"), "{name} still latches re-entry, through WIZ.busy");
    }
    // …and one count over the WHOLE file, because the two slices above do not cover the file
    // (review R5). `credentialsStranded` (`console.js:1617-1626`) sits between `wizGo` and
    // `wizFinish`, so its write at `:1623` is in neither slice — and a missed one is the worst of
    // the six: `renderWizard()` fires on the very next line and repaints `disabled = WIZ.busy`, so a
    // stranded-credentials recovery would re-enable Next and then immediately kill it again.
    assert_eq!(
        js.matches("EL(\"wiz-next\").disabled").count(),
        1,
        "renderWizard is the ONLY writer of Next's disabled state"
    );
    // (c) A refused Next says what is missing, in a sentence, and says it again on a second press —
    // a red line that was already on screen does not read as a new answer.
    // Review M7: the gates themselves, not merely the name — a `wizValid` that still exists and no
    // longer refuses an unsigned-in step is exactly the regression this is here to catch.
    let valid = js.split("function wizValid(").nth(1).and_then(|s| s.split("function wizGo(").next()).expect("wizValid");
    assert!(valid.contains("WIZ.step === 1 && !WIZ.accountId"), "step 1 is still gated on a session");
    assert!(valid.contains("WIZ.step === 2 && !WIZ.entitled"), "…and step 2 on an entitlement");
    assert!(go.contains("!wizValid()"), "wizGo refuses a forward step wizValid refuses");
    assert!(js.contains("flashError("), "a repeated refusal is re-announced, not silently unchanged");
    for sentence in ["Sign in first.", "Finish the payment page in your browser, then come back."] {
        assert!(js.contains(sentence), "the refusal names what is missing: {sentence}");
    }
}
```

- [ ] **Step 1b: The two shipped tests that demand the opposite** (review C4) — both are red the moment step 2 lands, and each keeps its claim in the new vocabulary. Rewrite them **in this step**, before the code:
  - `static_assets.rs:523-526`, in `the_logins_panel_maps_what_it_finds_to_a_course` (R-C1-55 I3's guard that a second Next cannot start a second `discover_coursework`): `go.contains("EL(\"wiz-next\").disabled = true")` and `… = false` become `go.contains("WIZ.busy = true")` and `go.contains("WIZ.busy = false")`. `go.contains("WIZ.discovering")` above them is untouched.
  - `static_assets.rs:494-501`, in `the_wizard_google_flow_keeps_its_state_on_wiz_and_renders_it` (R2-3, the double-Finish race): `let disabled_write = finish.find("EL(\"wiz-next\").disabled = true").expect("wizFinish disables wiz-next");` becomes `let busy_write = finish.find("WIZ.busy = true").expect("wizFinish latches WIZ.busy");`, and the ordering assertion — that it sits **before** the `invoke("google_connected"` await — stays word for word. Note the `.expect`: it **panics**, so this is not a soft failure to notice later.

- [ ] **Step 2: `renderWizard`** — two lines change:

```javascript
    // Spec §7 (a): hidden, not disabled. A greyed Back that does nothing is the control Quinn
    // pressed; a Back that is not on screen at step 0 asks no question.
    EL("wiz-back").hidden = WIZ.step === 0;
    EL("wiz-next").textContent = WIZ.step === PANELS.length - 1 ? "Finish" : "Next";
    // Spec §7 (b): the ONE writer of this property. `wizGo`'s discovery latch and `wizFinish`'s
    // re-entry guard both set `WIZ.busy` and call renderWizard, so a path that forgets to clear it
    // is a path renderWizard still recovers from on the next render — which a direct DOM write was
    // not. The file's own A-5 rule, applied to the last field that escaped it.
    EL("wiz-next").disabled = WIZ.busy;
```

`WIZ` gains `busy: false` (Task 5 adds it for the Google button; the two uses are the same flag — the wizard is doing something and Next must wait). **There are six `EL("wiz-next").disabled` writes, not two owners and three re-enables** (review C4), and every one becomes a `WIZ.busy` write with a `renderWizard()` behind it:

| `console.js` | Today | Becomes |
|---|---|---|
| `:1584` | `wizGo`'s discovery latch, `= true` | `WIZ.busy = true` |
| `:1601` | `wizGo`'s release when discovery settles, `= false` | `WIZ.busy = false` |
| `:1623` | `credentialsStranded`'s re-enable, `= false` | `WIZ.busy = false`, before its existing `renderWizard()` |
| `:1631` | `wizFinish`'s first statement, `= true` | `WIZ.busy = true` — still the **first** statement, before the `google_connected` await (R2-3) |
| `:1676` | `wizFinish`'s refusal path, `= false` | `WIZ.busy = false` |
| `:1688` | `wizFinish`'s `.catch`, `= false` | `WIZ.busy = false` |

After this task `grep -n 'EL("wiz-next").disabled' app/static/console.js` returns exactly one line — `renderWizard`'s — and step 1's count assertion is what enforces that, including for `credentialsStranded`, which neither of that test's two function slices reaches (review R5).

- [ ] **Step 3: The refusal that is heard twice** — `console.js`, beside `wizValid`:

```javascript
  // A refusal belongs on screen AND has to register as an answer to THIS press. `#wiz-error` sits
  // at the left of the nav row, so a second Next against the same unmet gate rewrote the same red
  // sentence and looked like nothing happened at all. One re-flow, one animation frame, and the
  // sentence arrives again.
  function flashError() {
    var el = EL("wiz-error");
    el.classList.remove("flash");
    void el.offsetWidth;
    el.classList.add("flash");
  }
```

…called from `wizGo`'s refusal branch:

```javascript
  function wizGo(n) {
    if (n > WIZ.step && !wizValid()) { renderWizard(); flashError(); return Promise.resolve(); }
```

…with one CSS rule in `console.css` beside the `.wiz` block:

```css
.wiz .crit.flash { animation: wiz-flash 420ms ease-out; }
@keyframes wiz-flash { 0% { opacity: 0; transform: translateY(-2px); } 100% { opacity: 1; transform: none; } }
```

- [ ] **Step 4: The sentences** — `wizValid`'s step-1 message becomes `"Sign in first."` (there is no "create an account" any more; both doors sign in). Step 2's stays word for word: `"Finish the payment page in your browser, then come back."` — Task 2 is what opens that wall, not a change of wording.

- [ ] **Step 5: Green, then commit.** `cargo test --workspace` at 0 warnings and `scripts/wizard-check.py` → `ok`, **including its check 8** — five Backs to the name panel, a rename, five Nexts to Finish, and the summary following the rename. Commit `app/static/{index.html,console.js,console.css}` and `app/tests/static_assets.rs` by name.

---

### Task 7: Close — the privacy sentences, the version bump, the two pushes, and the gate

**Files:**
- Modify: `site/privacy.html`, `app/src/account.rs`
- Test: `app/tests/static_assets.rs`
- Hand-offs: **H3** (`CLAUDE.md`), **H4** (`HANDOFF.md`), applied by the controller

- [ ] **Step 1: The privacy policy — Quinn's P4 first.** Ask Quinn to read the three changed sentences (spec §10) before they are published. Then, in `site/privacy.html`:
  - *What we collect → Your account*: delete the clause *"your password, which Supabase holds as a hash and which we never see;"* and add, after *"the email address you sign up with"*: *"and, if you choose Continue with Google, the fact that this account signs in with Google and the Google account id that identifies it — Knowlu never sees your Google password and never asks for one. There is no password on a Knowlu account at all: you sign in with Google, or with a code we email you."*
  - *Who else touches it → Supabase*: *"Your account row and every row above live there, and it is what checks the code we email you or the sign-in Google confirms."*
  - *Google*, one new sentence before the Gmail paragraph: *"Signing in with Google tells us three things and only three: your email address, your name, and a link to your profile picture. It asks Google for nothing else — not your mail, not your calendar, not your files — and it is a different permission from the Calendar and Gmail connections below, which you grant separately and can refuse."*
  - The `<p class="date">` line: both dates move to **2026-09-17**.

- [ ] **Step 2: The version, and the pin.** `account::PRIVACY_VERSION` moves from `"2026-09-16"` to `"2026-09-17"` in the same commit — `account.rs`'s own comment requires the constant and the page's date to move together, or the consent log points at text nobody can find. `TOS_VERSION` does **not** move: `site/terms.html` describes a subscription and an account, not an authentication method, nothing in it changed, and R-C1b-2 kept the card-up-front sentence true. **This is a NEW test, not an extension of an existing pin** (review M3): `static_assets.rs` pins the wizard's privacy *sentence* against the site's (`:617`, `the_wizards_privacy_sentence_is_the_sites_privacy_sentence`), and nothing anywhere pins `PRIVACY_VERSION` against the page's date — which is the rule `account.rs:21-23` states and the one this stream is about to depend on. Add to `app/tests/static_assets.rs`:

```rust
/// The version constant and the page's own date are one fact in two files (`account.rs`'s rule).
#[test]
fn the_privacy_version_constant_is_the_published_pages_date() {
    let rust = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    let version = rust
        .split("pub const PRIVACY_VERSION: &str = \"").nth(1)
        .and_then(|s| s.split('"').next())
        .expect("account.rs must declare `pub const PRIVACY_VERSION: &str = \"…\";`");
    let page = std::fs::read_to_string("../site/privacy.html").expect("site/privacy.html");
    assert!(page.contains(&format!("Effective {version}.")), "the page's Effective date is {version}");
    assert!(page.contains(&format!("This page is version <strong>{version}</strong>")), "…and so is its version line");
    // Spec §10: the policy must not describe a password the product no longer has.
    assert!(!page.contains("password hash"), "the password-hash clause is gone");
    assert!(page.contains("There is no password on a Knowlu account at all"), "…and the page says so");
    assert!(page.contains("Signing in with Google tells us three things"), "Google sign-in is disclosed");
}
```

- [ ] **Step 3: Staging — apply, deploy, and prove the findings live.** Against `knowlu-staging` only:

```powershell
supabase link --project-ref <the STAGING ref>
supabase db push
supabase functions deploy account --project-ref <the STAGING ref>
supabase functions deploy billing-checkout --project-ref <the STAGING ref>
```

Then the config push, **diff first** (R-C1b-5). The file declares `enable_confirmations = false` and `[auth.rate_limit]` and declares **no** provider, and the CLI (2.117.0) "pushes the properties your local config.toml declares … properties the file does not declare are left unchanged" — so the dashboard's Google provider must not appear in the diff at all:

```powershell
supabase config diff --project-ref <the STAGING ref>   # must show NO external.google change
supabase config push --project-ref <the STAGING ref>
```

**If the diff names `external.google` in any way, stop and report it** — that is the CLI disagreeing with the ruling this stream is built on, and a push would disable a provider that works.

With that pushed, the two findings are checked against the live service rather than against a reading of a source file:

- **The allow-list (spec §4).** `GET <staging>/auth/v1/authorize?provider=google&redirect_to=http%3A%2F%2F127.0.0.1%3A49999%2Fcallback&code_challenge=x&code_challenge_method=s256` must answer `302` with a `Location` on `accounts.google.com`. If it answers `302` back to `https://knowlu.com` instead, the loopback rule did not apply on this GoTrue build and `additional_redirect_urls` gains `"http://127.0.0.1:*/callback"` — **stop and report it**, because it changes the spec's finding and Quinn should hear it.
- **The mail (spec §5.2).** `POST <staging>/auth/v1/otp` for an address that has never signed up, then read the mail: it must be Knowlu's template with a six-digit code, not Supabase's *Confirm your signup*.
- **Identity linking, in BOTH states** (review I7) — arranged, not assumed. GoTrue links a Google identity to an existing user only when **both** sides' addresses are confirmed, and C1 accounts were created under `enable_confirmations = true`: one that never clicked the link is **unconfirmed**, so a Google sign-in on that address makes a *second* `auth.users` row, a second `accounts` row with a null attestation, and a second unentitled subscribe surface — while the money is on the first one. Prove it on staging with two accounts of the controller's own making:
  1. a **confirmed** pre-C1b address → Google sign-in must land on the same `accounts.id`, and `select count(*) from auth.users where email = …` must stay 1;
  2. an **unconfirmed** pre-C1b address → whatever GoTrue actually does, recorded and **reported to Quinn before merge**.
  The remedy either way is one sentence and needs no code: **one email-code sign-in confirms the address** (that is what `/verify` does), and Google links on the next attempt. Quinn's own pending production account is in that population, so the order matters for them personally: sign in once with an emailed code, then press Continue with Google.

- [ ] **Step 4: The live proof, by the controller, on a scratch profile.** Spec D1 end to end, against staging, never a live vault (`CLAUDE.md`, desktop safety):

```powershell
$env:KNOWLU_API_BASE = "https://<staging-ref>.supabase.co/functions/v1"
$env:KNOWLU_ANON_KEY  = "<the staging anon key>"
scripts\scratch-vault.ps1 -Source <a scratch vault>
```

…launch the app into the wizard, press **Continue with Google**, complete the consent in the browser that opens, and confirm: the browser shows one sentence; the wizard advances to the subscribe panel with *Signed in as …*; `knowlu/pending/session` exists in Credential Manager; `select id, email, age_attested_at from accounts` on staging has the row with a non-null `age_attested_at`; and `select kind, version from consents where account_id = …` has the three rows. Then press the subscribe button, type **P3's test-mode 100-percent promotion code** on Stripe's page, and confirm **Stripe still asks for a card** (R-C1b-2 — `payment_method_collection` is `"always"` and this proof is the check that it stayed), the total reads **$0.00**, **nothing is charged**, the webhook writes `entitlements`, and the wizard's poll advances to the name panel. A Checkout that asked for **no** card would mean the form value moved: stop and report it rather than reading it as a pass.

- [ ] **Step 5: The gate, the hand-offs, the close.** Run the whole gate below. Ask the controller for **H3** (`CLAUDE.md`'s recount and the no-password sentence) and **H4** (`HANDOFF.md` §3). Production — `supabase db push`, the two `functions deploy` against `knowlu-prod`, and a second `supabase config diff` (which must again show no `external.google` change) before the `config push` — is **Quinn's**, after staging is green, and is the one place in this plan where prod is named. Nothing in that sequence needs a Google client id or secret in the shell (R-C1b-5).

---

## Fidelity ledger

Every decision this plan carries, and the task that carries it. The spec's own ledger against the cloud design's §4.2 and §5.1 is `docs/specs/2026-09-17-c1b-sign-in-design.md` §11.

| # | Decision / ruling | Source | Carried by |
|---|---|---|---|
| D1 | Continue with Google is the primary sign-in | spec §1 D1 | Tasks 3 and 5. Proved by position in `static_assets.rs`, not by presence, and live in Task 7 step 4 |
| D2 | A loopback listener on `127.0.0.1`, ephemeral port, one sign-in | spec §1 D2, §2 | Task 3 steps 3-4. The listener is taken by value and dropped when the call returns, so no port outlives a sign-in |
| D3 | The session lands at `PENDING_TARGET`, unchanged | spec §1 D3 | Task 3 step 6. No panel after the account panel is edited, and `move_session`, `attach_account` and `create_vault_in` are not touched at all |
| D4 | The email path loses the password | spec §1 D4, §6 | Task 4 (`sign_up`/`sign_in` deleted, `create_user: true`), Task 5 (the field and the split gone), pinned both in Rust and over the shipped page |
| D5 | `enable_confirmations = false` | spec §1 D5, §5.2 | Task 4 step 3, with the GoTrue branch that forces it in the comment; proved live in Task 7 step 3 |
| D6 | Consent moves out of the trigger into `POST /account/consent` on **both** paths; `billing-checkout` keeps the tooth | spec §1 D6, §5.1, ruling **R-C1b-3** | Tasks 1 and 2 (the trigger that reads nothing, the route, the 403), Task 3 step 6 and Task 4 step 2 (both callers). The migration creates no table and no policy, so `migrations_test.ts`'s three invariants are untouched — but it does add a function definition, so C1's corpus pin goes 18 → 19 in Task 1 step 1(b) |
| D7 | A new Google Cloud project, "In production", basic scopes only | spec §1 D7, §3, ruling **R-C1b-5** | P1, **done 2026-09-17**: the project exists and the provider is enabled **in the Supabase dashboard on both projects**, verified by a keyless `/auth/v1/authorize?provider=google` answering 302 to `accounts.google.com`. `config.toml` declares **no** `[auth.external.google]` block on purpose — the id and the secret never enter the repo or a shell here — and Task 4 step 3's comment plus `config_toml_test.ts` are what keep it that way. The restricted-scope project and C1's P4 are not touched |
| D8 | `allow_promotion_codes`, and the card still taken up front | spec §1 D8, §8, ruling **R-C1b-2** | Task 2, one added line. `payment_method_collection` stays `"always"`, so §11 R2 is untouched and `site/terms.html`'s bolded disclosure stays true; the handler test keeps the `"always"` assertion and gains the new field's, with the reason beside both |
| §7 (a) | Back always goes back | spec §7 | Task 6 step 2: hidden at step 0, never `disabled`, and `wizGo`'s backward path already clears the error |
| §7 (b) | Next's disabled state is rendered from `WIZ` | spec §7 | Task 6 steps 1-2: `WIZ.busy`, one writer, pinned by a test that reads `wizGo` and `wizFinish` for a direct DOM write |
| §7 (c) | A refused Next says what is missing, every time | spec §7 | Task 6 steps 3-4: `flashError()` and two sentences |
| §7 (d) | A disabled wizard button looks disabled | spec §7 | Task 5 step 4: the `.wiz-nav` / `.wiz-row` rule the CSS never had |
| §10 | The privacy policy stops describing a password | spec §10 | Task 7 steps 1-2, with `PRIVACY_VERSION` and the page's date moving in one commit and a test that pins them to each other |
| Cloud design §5.1 | "No social login at launch" | §5.1 | **Superseded** by Quinn 2026-09-17 (spec §0). The path count does not grow: the password goes as Google arrives |
| `CLAUDE.md` | 0 warnings, LF, TDD, no `git add -A`, the Credential Manager lock | `CLAUDE.md` | Global Constraints, and every task's last step |
| `CLAUDE.md` | Recount the Tauri commands before quoting a number | `CLAUDE.md` | H1 counts them (29 + 42, 61 distinct) and H3 rewrites the sentence |
| Amendment 2026-09-17 | Desktop only; the account is the source of truth | the amendment | Nothing here assumes a second device, a browser session or a sync |

---

## Exit gate

1. **A student who has never run Knowlu signs in with one press.** On a scratch profile pointed at staging (`KNOWLU_API_BASE`, `KNOWLU_ANON_KEY`), **Continue with Google** opens the system browser, the consent screen names only email and profile, the browser is left on one sentence, and the wizard is on the subscribe panel saying *Signed in as …* — with nothing typed and nothing pasted. (Task 7 step 4.)
2. **The account exists server-side and is complete.** On staging: the `accounts` row for that sign-up has a non-null `age_attested_at`, `tos_version` and `privacy_version`; `consents` has its three rows; `entitlements` has its `none` row. **All of it written by `POST /account/consent`, not by the trigger** (R-C1b-3) — the trigger's insert leaves those four columns null and writes no consent row on any path. Without Task 1's migration this sign-up is a 500, so item 2 is what proves the finding was real and is fixed.
3. **The email path is one field and one button.** A fresh address, **Email me a code**, one mail — **Knowlu's template, with a six-digit code** — the code typed, signed in. No password field exists anywhere on the page (`the_page_has_no_lms_credential_field_anywhere` now counts exactly two, both coursework), and no password path exists in the crate (`there_is_no_password_path_left_in_the_crate`).
4. **An account made before C1b still works, in both of its states** (review I7). An address that signed up with a password in C1 receives a code and signs in with it. A Google sign-in on that address, **already confirmed**, links an identity to the existing user — one `auth.users` row, one `accounts` row, the same id. A Google sign-in on a pre-C1b address that was **never confirmed** is tried too, on staging, and whatever GoTrue does is reported to Quinn **before merge**; the remedy needs no code — one email-code sign-in confirms the address, after which Google links — and Quinn's own pending production account is in that population, so they hear the order to use.
5. **Back and Next behave.** `scripts/wizard-check.py` exits `ok`, its check 8 included — five Backs to the name panel, a rename, five Nexts to Finish, the summary following the rename. In the app: Back is absent at step 0 and present everywhere else; a refused Next paints a sentence that names what is missing and re-announces it on a second press; no wizard button is ever greyed without looking greyed.
6. **Checkout takes a promotion code, and still takes the card** (R-C1b-2). With P3's **test-mode** 100-percent code, Stripe's *Add promotion code* link accepts it, the card is still collected, **nothing is charged**, the webhook writes `entitlements`, and the wizard's poll advances to the name panel. `site/terms.html`'s bolded *your card taken at sign-up* is therefore still true of every student, coded or not, which is the whole of why `payment_method_collection` did not move.
7. **The 18+ gate still has a server-side tooth.** A staging account whose `age_attested_at` is null gets `403 "the 18+ attestation is missing"` from `billing-checkout`, and no Stripe customer, no consent row and no session are created. (Driven by nulling the column on a scratch account.)
8. **A never-verified address leaves an empty account and nothing else** (R-C1b-3, review I2). `POST <staging>/auth/v1/otp` with `create_user: true` for an address on the controller's own test domain, and then **no code typed**: `select tos_version, privacy_version, age_attested_at from accounts where email = …` is three nulls and `select count(*) from consents where account_id = …` is **0** — nothing in the log claims an attestation nobody made, which is exactly what the trigger stopped being able to write. The same account gets `403` from `billing-checkout`. And the blast radius is the number in `config.toml`, not a platform default: `[auth.rate_limit] email_sent` is pushed and `supabase config diff` is empty after the push.
9. **The two live findings are confirmed, not assumed.** The loopback `redirect_to` on an arbitrary high port answers `302` to `accounts.google.com` with no allow-list entry (spec §4), and `/otp` to a never-seen address sends Knowlu's template, not Supabase's (spec §5.2). Either one failing is reported to Quinn before the branch is closed.
10. **The policy and the app agree.** `site/privacy.html` is version `2026-09-17`, names Google sign-in, and no longer mentions a password hash; `account::PRIVACY_VERSION` is the same string; the cross-file pin passes. Quinn has read the three sentences (P4).
11. `cargo test --workspace` green at **0 warnings** (the `.rsrc` line accepted); `deno test`, `deno lint` and `deno fmt --check` green; `git ls-files --eol` unchanged for everything this stream did not add; `python scripts/wizard-check.py` prints `ok`.
12. `git diff --name-only main...c1b-sign-in` touches nothing outside this stream's ownership except the four hand-off files, each applied by the controller and named in H1-H4.

---

## What is NOT in this plan

- **Production deployment of C2.** Production is at C1 level — its eight functions of 2026-09-14, none of C2's eleven and no C2 migration (HANDOFF §4). Getting prod to parity is **a separate pre-pilot checklist**, not this stream's. C1b deploys its own one migration and its own two functions to both projects, because its own gate needs them, and touches nothing else on prod.
- **Anything about sync.** C3 is paused at Task 3's commit on `c3-sync`; C3' is unwritten. No file this stream owns is C3's, and `history.rs` is not touched.
- **A second social provider.** Apple, Microsoft and the rest are a new decision, not an extension of D1. One provider, chosen because every student in the pilot already has it.
- **Touching the restricted-scope Google client.** Project A, C1's P4, `google-connect`, `google-callback`, `gmail-read` and the Calendar grant on the wizard's calendars panel are all exactly as they are. C1b's client asks for `openid email profile` and can never be widened without becoming a review, which is why it is a second project.
- **Removing `/auth/v1/signup` server-side.** GoTrue's own endpoint stays enabled — `enable_signup = true` is what lets `/otp` create a user at all. What goes is every caller of it in this repo: a password account can no longer be made *by Knowlu*, which is the product claim.
- **The console's visual redesign**, parked by Quinn on 2026-09-07. This stream adds one CSS rule and one keyframe and changes no token.
- **Telemetry for the sign-in path.** No event is added for "signed in with Google". D5's class (a) is the console's interaction events, and a new event kind is a schema decision, not a side effect of a button.
- **C4's removal of the local runtime.** `inference.rs`, `SUPPORTED_RUNTIMES` and the settings panel's *Local judgment* row stay exactly as they are (`CLAUDE.md`: until C4 lands, that code stays and is not extended).
- **A second device, a browser build or a mobile app.** The Amendment 2026-09-17 ruled desktop only, and a loopback listener is a desktop mechanism. Nothing here assumes otherwise.

---

## Fix round 1 — resolutions (2026-09-17)

Against `docs/reports/2026-09-17-c1b-sign-in-plan-review.md` (verdict: *execute after fix round 1*).
Every finding below is answered in the plan above, or in
`docs/specs/2026-09-17-c1b-sign-in-design.md` where the decision was the spec's. **Nothing has been
executed**: this is still a plan, and its status line says so.

### The controller's rulings

- **R-C1b-2 (review C6) — `payment_method_collection` stays `"always"`; only `allow_promotion_codes`
  is added.** `if_required` collects a card only when the first invoice has an amount due, and the
  7-day trial makes that invoice zero for every subscription, so it would have taken a card from
  nobody — repealing §11 R2 rather than relaxing it, falsifying `site/terms.html:24` and
  `index.html:95` (the text each `auto_renew` consent row is stamped against), and ending every trial
  in `past_due`. Quinn's own subscription is solved outside the product: a card typed once under the
  100-percent code and never charged, or a comped subscription in the Stripe dashboard. **Where:**
  spec D8, §8 and the §11 R2 ledger row; plan Task 2 (preamble, steps 1 and 2), the D8 ledger row,
  exit gate 6.
- **R-C1b-3 (review I1, I2) — consent is recorded by `POST /account/consent` on BOTH paths, behind a
  verified session; the trigger reads nothing and never raises.** A conditional raise would still
  have believed `/otp`'s `data`, and `/otp` is reachable by anyone holding the public anon key: the
  `age_18` row it wrote could assert an attestation the address's owner never made. The trigger now
  inserts the `accounts` row with four nulls and the empty `entitlements` row and returns; the route
  is the only writer of a consent row; the tooth is `billing-checkout`'s 403. **Where:** spec D6,
  §5.1, §5.2, §2 step 8, the §4.2-step-1 and new §9 ledger rows; plan Task 1 steps 1-5, Task 3
  step 6, Task 4 steps 1-2, exit gate 2 and the new exit gate 8.
- **R-C1b-4 (review I2) — the `/otp` rate limit is set explicitly.** `[auth.rate_limit] email_sent =
  20` in `config.toml`, with the reason and the trade-off in the comment beside it: 20 an hour is far
  above a pilot of tens of students signing in once per device (a group onboarding together, with a
  mistyped address and a retry each, still fits), and it caps a stranger holding the public key at 20
  empty accounts an hour and zero consent rows. The same 20 is also the budget a flood would spend,
  which is a real student unable to sign in for the rest of that hour — and that, not a bigger
  number, is what `[auth.captcha]` would answer. **Where:** plan Task 4 step 3 and 3b, exit gate 8.
- **R-C1b-5 (review C5) — the Google provider is the dashboard's; `config.toml` declares no
  `[auth.external.google]` block.** Quinn enabled it on both projects on 2026-09-17 and it was
  verified keylessly (`/auth/v1/authorize?provider=google` → 302 to `accounts.google.com`, each
  project's own callback, scope `email profile`); the CLI (2.117.0) pushes only the properties the
  local file declares. A block would have put a client id in the repo, needed `GOOGLE_SECRET` in
  every pushing shell, and made a push from a shell without it an outage. What stands in its place is
  a comment where the block would have gone, a pin over both the absence and the comment, and a
  `supabase config diff` before every push. **Where:** spec §3, §9 Q1 and Q2, §13; plan Global
  Constraints, P1 (now **done**), P2 (now the controller's), Task 4 step 3 and 3b, Task 7 steps 3
  and 5, the D7 ledger row.

### Critical

- **C1 — the corpus pin nobody bumped.** Task 1 step 1 is now two parts: (a) the per-file test in
  `cloud/supabase/migrations_test.ts`, (b) `cloud/supabase/migrations/migrations_test.ts:304`'s
  `assertEquals(parsed, 18, …)` → **19**, with a sentence naming C1b's `create or replace` as the
  nineteenth. The two files are named separately everywhere, including the Files list, the ownership
  list and the commit line, because they are different files with different jobs.
- **C2 — the unsatisfiable `"password"` assertion.** Dropped. `there_is_no_password_path_left_in_the_crate`
  keeps its six-name loop and gains two claims that are true and load-bearing: no `"password"` JSON
  key, and no `password:` parameter in `account.rs`. The four prose lines (`:6`, `:135`, `:173`,
  `:181`) are rewritten **in the same step**, since `:181` quotes the literal the new assertion bans.
- **C3 — three shipped tests the tasks deleted out from under.** `static_assets.rs:890` is rewritten
  in Task 4 step 1b, over `send_magic_link(email, age_attested)` — R-C1-55's trap, one command over —
  and the two overlay id lists (`:980`, `:1032`) are swapped in Task 5 step 1b for `up-google`,
  `up-magic`, `up-code-row`, `up-code`, `up-code-go`. Each is a named step before the code it guards.
- **C4 — the two tests that demanded the opposite.** Task 6 step 1b rewrites `static_assets.rs:523-526`
  (`WIZ.busy = true` / `= false` inside `wizGo`) and `:494-501` (`finish.find("WIZ.busy = true")`,
  still asserted **before** the `google_connected` await — the `.expect` there panics, so it is not a
  soft failure). Step 2 now names all **six** `EL("wiz-next").disabled` write sites in a table
  (`:1584`, `:1601`, `:1623`, `:1631`, `:1676`, `:1688`), not two owners and three re-enables.
- **C5 — the missing Google configuration.** Answered by **R-C1b-5** above: not a block, a comment
  and a pin, plus `supabase config diff` in Task 7 steps 3 and 5.
- **C6 — `if_required` repeals R2.** Answered by **R-C1b-2** above, option (a).

### Important

- **I1 — the trigger's terms/privacy raise.** Superseded by **R-C1b-3**: no raise at all, and no
  consent row from the trigger on either path, so the §9 log cannot be half-written by a sign-up.
- **I2 — `create_user: true` + confirmations off.** Answered twice: **R-C1b-3** (a fabricated
  attestation is now impossible — the trigger reads nothing) and **R-C1b-4** (the blast radius is a
  number in `config.toml`). Exit gate 8 proves the empty account, and `[auth.captcha]` is flagged
  below as Quinn's separate decision.
- **I3 — the listener's first-connection-wins.** Task 3 steps 3-4: it loops to the deadline, answers
  `404` to any target that is not `/callback`, and returns on the first real one. The doc comment now
  names the compensating control against a *forged* code — the verifier is per attempt and never
  leaves the process — which is also the honest answer to "where is `state`?". A new test drives a
  stray `/favicon.ico` in front of the real callback.
- **I4 — fire-and-forget consent, and three comments that were not true.** The comments are true now
  (R-C1b-3: both paths call the route). The call is logged rather than discarded, in `google_sign_in`
  and in `verify_email_code`, the way `attach_in`'s back-fill logs (`account.rs:602`); `open_checkout`
  retries it once before the checkout POST through the new `checkout_url_at`, and a test pins the
  order — consent first, Stripe second, and a failed retry still reaches Stripe so the server's 403
  is what answers.
- **I5 — the widened `Deps` interfaces.** Task 1 step 3 puts both new fields in `deps()`
  (`handler_test.ts:6`) first and builds `consentDeps` from `deps({…})`; Task 2 step 1 says **four**
  inline `getAccount` stubs (`:43`, `:79`, `:114`, `:142`), not three.
- **I6 — the upgrade overlay by reference.** Task 5 now writes its markup in full (it had no
  email-code path at all), its three click branches in full, its own `UP_BUSY` flag and `upBusy()`
  (nothing paints that panel, so `WIZ.busy` cannot reach it), the shared `afterUpgradeSignIn()` tail,
  the five new ids in Produces, a test over `#upgrade` that proves order, absence of a password,
  the three branches and the busy guard — and the `.set-row[hidden]` CSS rule the new code row needs,
  since `.set-row` is `display: flex` and the sheet had no such rule.
- **I7 — identity linking assumed.** Exit gate 4 and Task 7 step 3 prove it on staging in **both**
  states, confirmed and unconfirmed, and report the unconfirmed result to Quinn before merge. The
  remedy needs no code — one email-code sign-in confirms the address, after which Google links — and
  the plan says Quinn's own pending production account is in that population.
- **I8 — H2's `#wiz-google`.** H2 says `#wiz-google-signin` throughout, and says why: `#wiz-google`
  is the Google **Calendar** button, pinned at `static_assets.rs:426` and `:465`.

### Minor

- **M1** line cites corrected: `checkoutForm` `:39`, `payment_method_collection` `:55`,
  `trial_period_days` `:54`, the account panel `index.html:86-93` (spec §6), the overlay `:174-182`,
  the corpus pin `migrations_test.ts:304`. **M2** seven sign-in commands in both lists, with the
  seven named and the arithmetic shown (71 registrations, 10 twice, 61 distinct). **M3** the
  privacy-version pin is a **new** test, not an extension — `static_assets.rs:617` pins a different
  sentence. **M4** P1 and P2 now point at Task 7 step 3 (and P1 is done, P2 is the controller's).
  **M5** `record_consent_at` no longer string-matches `post_for_url`'s private `"no link came back"`:
  both go through a new `post_authed`, and `post_no_reply` is the reading for a route that answers no
  link. **M6** Task 1 step 5 is PostgREST (`restSelect`/`restPatch`/`restUpsert`, all already
  imported) and states the `age_18` version literal `'1'` and why `sha256Hex` matches the old
  trigger's `lower(email)`. **M7** the `wizValid` assertion reads its two gates and `wizGo`'s call,
  not just the name. **M8** `.wiz-nav button.b[hidden] { display: none; }` added beside the new
  `[disabled]` rule, with the reason it is pre-emptive. **M9** one sentence in Task 3 on
  `#[tauri::command(async)]` holding an async-runtime worker for up to 180 s, why it is accepted and
  what bounds it. **M10** Task 4 step 1b deletes `app/tests/account.rs:102`, `:141` and `:149` and
  narrows the `use` at `:3`.

### Open for Quinn — not decided by this stream

1. **A card-free trial for everyone** (review C6, option (b)). Today's plan keeps the card: the
   terms, the subscribe panel and `TOS_VERSION` all stay as they are, and the founder's own
   subscription is handled with a comped or 100-percent-coded card. If Quinn would rather sell a
   trial that asks for no card at all, that is a terms rewrite with the lawyer,
   `subscription_data[trial_settings][end_behavior][missing_payment_method]` set deliberately, a
   `TOS_VERSION` move and a new consent version — a separate change, cheap to make and not this
   stream's to assume.
2. **`[auth.captcha]`** (review I2). The Supabase-native answer to `/otp` abuse, and the only one
   that separates a flood from real demand — a rate limit can only choose which of them to refuse.
   Not built here: it needs a provider account, a site key in the app and a token on the request, and
   it is a decision about what a student is asked to do before they can sign in.

---

## Fix round 2 — resolutions (2026-09-17)

Against the re-review appended to `docs/reports/2026-09-17-c1b-sign-in-plan-review.md`
(*"execute after fix round 2"*; all twenty original findings resolved, six new ones). No new ruling
was needed: R1 is R-C1b-2 reaching one paragraph round 1 missed, and the rest are placement, gates
and wording. The spec needed no change — every finding landed in this file.

- **R1 — Task 7 step 4's live proof asked for the card-free Checkout R-C1b-2 rejected.** The step now
  reads: Stripe **still asks for a card**, the total reads **$0.00**, nothing is charged, the webhook
  writes `entitlements`, the poll advances — and a Checkout that asks for *no* card means the form
  value moved and is **stopped and reported**, not passed. It was the last sentence anywhere in
  either document written from the (b)-world.
- **R2 — `UP_BUSY` was placed inside the click listener, where it guards nothing.** Task 5 step 3 now
  splits the replacement in two: **(a)** the flag, `upBusy()` and `afterUpgradeSignIn()` at IIFE
  scope beside `upgradeUnreachable()` (`console.js:1237`), where `UPGRADE_DISMISSED` and
  `UPGRADE_UNREACHABLE` already live; **(b)** the three branches, and only those, inside the
  listener. The pin now fails on the wrong placement: the text **before**
  `EL("upgrade").addEventListener` must contain `var UP_BUSY` — the two assertions round 1 added pass
  either way, which is exactly why a third was needed.
- **R3 — `[auth.rate_limit]` is a sibling table, not a key in `[auth.email]`.** Task 4 step 3 names
  the insertion point: after `double_confirm_changes = true` (`config.toml:29`), before the SMTP
  comment block at `:31`, at `[auth]` sibling level — and says what the old wording would have cost
  (the header adopting `double_confirm_changes` into the rate-limit table, so a push carries away a
  setting nobody meant to move). Step 3b asserts `double_confirm_changes` appears **before** the
  first `[auth.rate_limit]` line.
- **R4 — Task 4's gate did not run Task 4's Deno test.** Step 4 now runs
  `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/` and `deno lint` beside
  `cargo test --workspace` and `deno fmt --check`, as Tasks 1 and 2 already do; step 3b's assertions
  are green at their own gate rather than first executed at exit gate 11.
- **R5 — nothing tested the `console.js:1623` conversion, and the plan claimed something did.** Step
  1's pin gains a count over the whole file —
  `assert_eq!(js.matches("EL(\"wiz-next\").disabled").count(), 1, …)` — because `credentialsStranded`
  (`:1617-1626`) sits between the `wizGo` and `wizFinish` slices and is in neither. The "from the
  other direction" clause in step 2 is replaced by what is actually true: the count assertion is what
  enforces the single writer, `credentialsStranded` included.
- **R6 — two nits.** (a) Task 5 step 1 now moves the length annotation with the list:
  `const OURS: [&str; 4]` → `const OURS: [&str; 2]` (`static_assets.rs:384`), or the file does not
  compile. (b) Task 4 step 3 rewrites `config.toml:31-35`'s opening sentence, stale the moment
  `enable_confirmations` flips: the SMTP block's reason is the same dependency on the same provider,
  now for the **code** mail, and the rest of the block stands word for word.
