# Knowlu C1b — sign-in: Continue with Google, the code without a password, Back and Next, and a promotion code — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status: WRITTEN 2026-09-17, not started.** Execute on a branch `c1b-sign-in` in a worktree of this repository, merged into `main` before C3', C5 and C4 (HANDOFF §3: Quinn, 2026-09-17, "as soon as possible", before every other stream). Written against `docs/specs/2026-09-17-c1b-sign-in-design.md`.

**Goal:** A student who has never heard of Knowlu presses **Continue with Google**, picks their account in the browser they are already signed in to, and comes back to a wizard that has moved on — no password, no code, nothing pasted. A student who would rather not use Google types their address, presses **Email me a code** once, types the code, and is in. Whichever they chose, **Back always goes back and Next either moves or says in one sentence what is missing**. And the founder can finish the subscribe step with a 100-percent promotion code and no card.

**Architecture:** Three layers, the same three C1 used, and no new one. **The cloud:** one new migration that lets an OAuth sign-up create its `accounts` row (today's trigger raises on it), one new route `POST /account/consent` on the existing `account` function that records the 18+ attestation and the two policy versions after the fact, and two form fields on `billing-checkout` — plus the attestation gate that replaces the `raise` the trigger gives up. **The device:** `app/src/account.rs` gains a loopback `TcpListener` on `127.0.0.1:0`, a PKCE verifier and challenge built from `sha2` (already a dependency) and a hand-rolled base64url, and one command `google_sign_in`; `sign_up` and `sign_in` are deleted with the password they took. **The page:** the account panel and the upgrade overlay lose the password field and the create/sign-in split, gain one Google button, and the wizard's nav learns that a disabled button must look disabled and that `#wiz-next`'s disabled state belongs on `WIZ` like every other wizard field.

**Tech Stack:** Supabase (Postgres 15, Auth/GoTrue, Edge Functions on Deno), Stripe Checkout; Rust 1.98 `stable-x86_64-pc-windows-gnu` with `ureq 3.4` and `sha2 0.10` — both already in `app/Cargo.toml`, so **this plan adds no crate**; plain ES5-flavoured JavaScript in `app/static/console.js`, no framework and no bundler.

**Spec:** `docs/specs/2026-09-17-c1b-sign-in-design.md` (D1-D8, §2 the flow, §4 the allow-list finding, §5 the accounts-row finding, §7 Back and Next, §8 the promotion code, §9 Quinn's four, §10 the privacy sentences). Supporting: `docs/specs/2026-09-09-knowlu-cloud-design.md` §4.2 and §5.1 and its *Amendment 2026-09-17*; `CLAUDE.md`; `VISION.md`; `HANDOFF.md` §3.

---

## Global Constraints

Every task's requirements implicitly include this section.

- **Add no single-user assumptions.** Nothing in this stream names a person's vault, machine, account, email address, OAuth client, promotion code or credential. The loopback port is chosen by Windows at bind time and is never written down. (`CLAUDE.md`, rule 1.)
- **Never regenerate a frozen reference.** The eight Python-written references in `engine/tests/fixtures/` and the three Rust-generated `surface-today-*.json` are not read or written by anything here. (`CLAUDE.md`, rule 2.)
- **No secret in the repo, a log, a fixture, a test name, a commit message or this plan.** The Supabase project URL and anon key are **public** and are compiled into the app. `GOOGLE_CLIENT_ID` and `GOOGLE_SECRET` are resolved by the Supabase CLI from the pushing shell's environment (`env(...)` in `config.toml`, the shape `SMTP_PASSWORD` already uses) and appear nowhere else. The PKCE verifier, the authorisation code and both tokens are never logged, never in an error message, and never cross the IPC to the page. If a value is ever printed, say so immediately and treat it as exposed.
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

- **File ownership is binding.** This stream edits only: `cloud/supabase/config.toml`, `cloud/supabase/migrations/20260917000100_oauth_consent.sql` (new), `cloud/supabase/functions/account/**`, `cloud/supabase/functions/billing-checkout/**`, `app/src/account.rs`, `app/static/**`, `app/tests/{account.rs,static_assets.rs}`, `site/privacy.html`, and this plan file. Everything else — `app/src/main.rs`, `scripts/wizard-check.py`, `CLAUDE.md`, `HANDOFF.md` — is the controller's and appears under **Controller hand-offs** with exact code. **A task that silently edits one of those files is a plan defect** — stop and report it instead of editing.

---

## Quinn-owned preconditions

Asked **one at a time, when the task reaches them, with the context** — never as a list of chores.

| # | Needed by | What, and what breaks without it |
|---|---|---|
| **P1** | Task 3 step 4, and Task 7's live proof | **A new Google Cloud project, publishing status "In production", with one OAuth 2.0 client of type "Web application".** Consent screen scopes: **`openid`, `email`, `profile` and nothing else** — basic scopes, so Google asks for no verification review and imposes no 100-test-user cap (spec §3). Authorised redirect URIs, both of them: `https://<staging-ref>.supabase.co/auth/v1/callback` and `https://<prod-ref>.supabase.co/auth/v1/callback`. **This is a second project on purpose:** the existing one carries `gmail.readonly` and `calendar.readonly`, which are restricted and sensitive, and one client carrying a restricted scope drags the whole client through review — a sign-in button that works for 100 named testers is not a sign-in button. Values I need: the **client id** (public, and it goes nowhere but the pushing shell either). The **client secret** stays with Quinn. **Without it:** `/authorize?provider=google` answers `400 Unsupported provider` on both projects, and D1 cannot be proved on anything. |
| **P2** | Task 7 step 2 | **`supabase config push` run against `knowlu-staging`, then against `knowlu-prod`, from a shell that has `GOOGLE_CLIENT_ID` and `GOOGLE_SECRET` in its environment** — the same mechanism `pass = "env(SMTP_PASSWORD)"` already uses (`cloud/supabase/config.toml`, and the 2026-09-10 stream report records the controller doing exactly this for SMTP). Quinn runs it, or hands the controller a shell that already carries the two values. It also carries this stream's `enable_confirmations = false` (spec D5) to both projects. **Without it:** the provider is off, and the email path still sends the stock *Confirm your signup* mail with a link and no code. |
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

New totals to quote afterwards: **29 + 42, 61 distinct** (`launch_state`, `pick_folder`, `pick_file` and the six sign-in commands are in both lists). The seven note-mutating commands are unchanged. **Apply H1 before Task 3 step 5** — a command that is not registered is rejected before its body runs, and the page's `.catch` then paints a transport error for a call that never left the process.

### H2 (Task 5) — `scripts/wizard-check.py`, the headless wizard walk

Three edits, all in `check(page)` and its fake:

1. `BEFORE_FINISH_OK` loses `"sign_up"` and `"sign_in"` and gains `"google_sign_in"`. The set's meaning is unchanged — the only commands the wizard may have called before Finish are reads, the account, and the two that write a credential.
2. The fake's `sign_up`/`sign_in` branch is replaced by one that answers the new command:

   ```javascript
   if (cmd === 'google_sign_in') { return Promise.resolve({ ok: true, error: null, account_id: 'acc-1', email: 'a@example.invalid' }); }
   ```

   …and `send_magic_link` keeps its `{ ok: true, error: null }` answer.
3. Check 2's body — today it fills `#wiz-email` and `#wiz-pw`, clicks `#wiz-create` twice and asserts on `sign_up` — becomes: assert `#wiz-pw` does not exist at all; click `#wiz-google` with the boxes unticked and assert `google_sign_in` was **not** called and the error names the boxes; tick both, click `#wiz-google`, assert `google_sign_in` was called and the panel advanced to `#wiz-subscribe`. The exact replacement text is Task 5 step 5.

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
- Test: `cloud/supabase/functions/account/handler_test.ts`, `cloud/supabase/migrations_test.ts`

**Interfaces:**
- Consumes: `_shared/auth.ts::requireUser`, `_shared/http.ts::{json, fail, readJson, methodNotAllowed, subPath}`.
- Produces: the route `POST /account/consent`, and `Deps.recordAccountConsent` / `Deps.getConsentState`. Task 2 consumes the `age_attested_at` column this task starts leaving null; Task 3 calls the route.

Spec §5.1 is the finding this task exists for: today's `handle_new_user()` raises `the terms and the privacy policy must be accepted at sign-up` on a Google sign-up, because Google's ID-token claims carry none of the three metadata keys, and the `auth.users` insert rolls back with `Database error saving new user`.

- [ ] **Step 1: The migration test first** — add to `cloud/supabase/migrations_test.ts`, beside the three invariants it already pins:

```typescript
Deno.test("the OAuth migration adds no table, no policy and no birthdate column", async () => {
  const sql = await Deno.readTextFile(
    new URL("./migrations/20260917000100_oauth_consent.sql", import.meta.url),
  );
  // C1's three rules are pinned over the whole directory elsewhere; this one is about THIS file:
  // it replaces one function and nothing else, so a reviewer never has to diff schema to be sure.
  assert(!/create\s+table/i.test(sql), "this migration creates no table");
  assert(!/create\s+policy/i.test(sql), "…and no policy: RLS is C1's and stays as it is");
  assert(!/\bbirth|\bdob\b|date_of_birth/i.test(sql), "no birthdate column, in this file or any other");
  assert(sql.includes("create or replace function public.handle_new_user()"), "the trigger's function is replaced");
  // The explicit refusal survives. An absent attestation is the OAuth case; a `false` one is a
  // client saying "I am not 18", and that still fails at the database.
  assert(sql.includes("age attestation required"), "an explicit false still raises");
});
```

- [ ] **Step 2: The migration** — `cloud/supabase/migrations/20260917000100_oauth_consent.sql`:

```sql
-- Knowlu C1b, Task 1 — an OAuth sign-up can create its account row (spec §5.1).
--
-- Google's ID-token claims carry no `age_attested`, `tos_version` or `privacy_version`, so today's
-- function raises on the second `if` and the whole `auth.users` insert rolls back: GoTrue answers
-- `Database error saving new user` and redirects with `error=server_error`. The attestation itself
-- does not go away — it moves to `POST /account/consent`, which the app calls the moment a session
-- exists, and `billing-checkout` refuses an account whose `age_attested_at` is still null. A patched
-- client still gets an account it cannot use; the refusal just happens one layer further out.
--
-- Nothing else changes: no table, no policy, no column. The three rules `migrations_test.ts` pins
-- are untouched.
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public, extensions as $$
declare
  raw      text    := new.raw_user_meta_data ->> 'age_attested';
  attested boolean := raw = 'true';
  tos      text    := nullif(new.raw_user_meta_data ->> 'tos_version', '');
  priv     text    := nullif(new.raw_user_meta_data ->> 'privacy_version', '');
begin
  -- An explicit 'false' is a client saying it is not 18. Refused here exactly as before.
  if raw is not null and not attested then
    raise exception 'age attestation required: Knowlu is for people 18 or older';
  end if;

  insert into public.accounts (id, email, tos_version, tos_accepted_at, privacy_version, age_attested_at)
  values (
    new.id, new.email, tos,
    case when tos is not null then now() end,
    priv,
    case when attested then now() end
  )
  on conflict (id) do nothing;

  insert into public.entitlements (account_id, status) values (new.id, 'none')
  on conflict (account_id) do nothing;

  -- The consent rows only where there is something to record. An OAuth sign-up records none here
  -- and three at `POST /account/consent`, with the same kinds and the same versions.
  if attested and tos is not null and priv is not null then
    insert into public.consents (account_id, subject_hash, kind, version)
    select new.id,
           encode(extensions.digest(lower(new.email), 'sha256'), 'hex'),
           k.kind,
           k.version
    from (values ('tos', tos), ('privacy', priv), ('age_18', '1')) as k(kind, version);
  end if;

  return new;
end;
$$;
```

The trigger itself is not recreated: `create or replace function` swaps the body under the existing `on_auth_user_created`.

- [ ] **Step 3: The route's test first** — add to `cloud/supabase/functions/account/handler_test.ts`:

```typescript
function consentDeps(recorded: unknown[], already: boolean) {
  return {
    verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
    getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null }),
    hasConsent: () => Promise.resolve(already),
    recordAccountConsent: (c: unknown) => {
      recorded.push(c);
      return Promise.resolve();
    },
    now: () => new Date("2026-09-17T12:00:00Z"),
  } as unknown as Deps;
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
 * **The attestation, after the fact.** A Google sign-up reaches `auth.users` with none of the three
 * metadata keys the email path sends, so migration `20260917000100` lets the trigger write an
 * `accounts` row with `age_attested_at` null rather than raising — and this is where that null is
 * filled. The app calls it after EVERY sign-in, on both paths, which is why the second call must be
 * silent rather than a conflict.
 *
 * The 18+ gate has not moved off the server: `billing-checkout` refuses an account whose
 * `age_attested_at` is still null, so a client that skips this route gets an account that can never
 * subscribe. That is the same promise the migration's `raise` used to make.
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

- [ ] **Step 5: The wire** — in `cloud/supabase/functions/account/index.ts`, two `Deps` entries beside the existing ones, through the service-role client the file already builds: `hasConsent` is `select 1 from consents where account_id = $1 and kind = 'tos' limit 1`; `recordAccountConsent` is one `update public.accounts set tos_version, tos_accepted_at, privacy_version, age_attested_at where id = $1 and age_attested_at is null`, then one insert of the three `consents` rows with `subject_hash` computed the same way the trigger computes it (`encode(digest(lower(email), 'sha256'), 'hex')`, through the same helper `deleteAccount` already uses for the tombstone). No new import.

- [ ] **Step 6: Green, then commit.** `deno test --allow-read --config cloud/supabase/deno.json cloud/supabase/`, `deno lint`, `deno fmt --check`. Commit `cloud/supabase/migrations/20260917000100_oauth_consent.sql`, `cloud/supabase/migrations_test.ts`, `cloud/supabase/functions/account/{handler.ts,handler_test.ts,index.ts}` by name.

---

### Task 2: Checkout takes a promotion code, asks for a card only when one is owed, and refuses an account that never attested

**Files:**
- Modify: `cloud/supabase/functions/billing-checkout/handler.ts`, `cloud/supabase/functions/billing-checkout/index.ts`
- Test: `cloud/supabase/functions/billing-checkout/handler_test.ts`

**Interfaces:**
- Consumes: Task 1's `accounts.age_attested_at`, now nullable in practice as well as in the schema.
- Produces: nothing new for another task. Quinn's P3 code is what exercises it.

Spec D8 and §8. `payment_method_collection` is `"always"` today (`handler.ts:56`), which asks for a card even when a 100-percent code means nothing will ever be charged.

- [ ] **Step 1: The test first** — in `handler_test.ts`, extend the form test and add two:

```typescript
  // Spec §8. `if_required` is what makes a 100-percent promotion code work end to end: with
  // `always`, Stripe collects a card for a subscription it will never charge. For every full-price
  // trial Stripe still asks — §11 R2's "card up front" is relaxed only where Stripe itself decides
  // no payment method is needed.
  assertEquals(form["payment_method_collection"], "if_required");
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

The three existing tests gain `age_attested_at: "2026-09-17T12:00:00Z"` in their `getAccount` answers; nothing else about them moves.

- [ ] **Step 2: The form** — in `checkoutForm`, change one line and add one:

```typescript
    // §8: a 100-percent promotion code needs no card, and `always` would demand one anyway.
    "payment_method_collection": "if_required",
    // The code is typed on Stripe's own page. Nothing in this repo names a code or a coupon.
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
- Produces: `b64url()`, `pkce_pair()`, `authorize_url()`, `code_from_request_line()`, `CALLBACK_PAGE`, `serve_one_callback()`, `exchange_pkce_at()`, `record_consent_at()`, and the command `google_sign_in`. Task 5's page calls the command; Task 7's gate proves it live.

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
```

- [ ] **Step 4: The listener** — `app/src/account.rs`:

```rust
/// What the browser is left looking at. One sentence, no styling, no script, no link back — the
/// student's next move is the Knowlu window that is already open behind it.
pub const CALLBACK_PAGE: &str =
    "<!doctype html><meta charset=\"utf-8\"><title>Knowlu</title>\
     <p style=\"font:16px system-ui;margin:3rem\">You are signed in to Knowlu. You can close this window.</p>";

/// Accept **one** connection, read its request line, answer it, and give the socket back to the OS.
///
/// `listener` is taken **by value** on purpose: it is dropped when this returns, so there is no way
/// to leave a port open on a student's machine after a sign-in — successful, refused or abandoned.
/// The deadline is enforced by polling `accept` on a non-blocking listener rather than by a second
/// thread, so nothing outlives the call.
///
/// The reply is sent on **both** paths. A refusal is still a browser window a person is looking at,
/// and a connection reset is not an explanation.
pub fn serve_one_callback(listener: std::net::TcpListener, wait: std::time::Duration) -> Result<String, String> {
    use std::io::{Read, Write};
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + wait;
    let mut stream = loop {
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err("the Google sign-in was not finished — try again".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    // The request LINE is all this needs, and a browser sends it in the first packet. Reading to
    // the first newline rather than to EOF is also what keeps a keep-alive connection from hanging.
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0);
    let head = String::from_utf8_lossy(&buf[..n]).to_string();
    let line = head.lines().next().unwrap_or_default().to_string();
    let body = CALLBACK_PAGE;
    let resp = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
    code_from_request_line(&line)
}
```

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

/// The 18+ attestation and the two policy versions, recorded after the session exists.
///
/// **Why it is a second call and not metadata.** `/authorize` has no field for user metadata, and
/// migration `20260917000100` therefore lets a Google sign-up create its `accounts` row with
/// `age_attested_at` null rather than raising. This is what fills it; `billing-checkout` refuses an
/// account where it is still null, so skipping this call is not a way around the gate.
///
/// Called after **both** sign-in paths and idempotent on the service side, so it needs no "is this
/// a new account" question the app has no honest way to answer.
pub fn record_consent_at(api_base: &str, token: &str, tos: &str, privacy: &str) -> Result<(), String> {
    let body = json!({ "tos_version": tos, "privacy_version": privacy, "age_attested": true });
    post_for_url(api_base, "/account/consent", token, &body).map(|_| ()).or_else(|e| {
        // `post_for_url` insists on a `url` in the reply and this route answers `{ok:true}`; the
        // absence of a link is not a failure here, and every real refusal keeps its own sentence.
        if e == "no link came back" { Ok(()) } else { Err(e) }
    })
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
        // Best effort, and second: the session is already on this machine, and an attestation that
        // did not land is a subscribe step that says so — not a sign-in that has to be redone.
        let _ = record_consent_at(&api, &s.access_token, TOS_VERSION, PRIVACY_VERSION);
        Ok((id, s))
    })();
    match out {
        Ok((id, s)) => ok_account(&id, &s.email),
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}
```

- [ ] **Step 7: Green, then commit.** `cargo test --workspace` at 0 warnings. Commit `app/src/account.rs` and `app/tests/account.rs` by name. **Ask the controller to apply H1 before this task's own smoke run** — an unregistered command is rejected before its body runs.

---

### Task 4: The email path loses the password

**Files:**
- Modify: `app/src/account.rs`, `cloud/supabase/config.toml`
- Test: `app/tests/account.rs`

**Interfaces:**
- Removes: `sign_up_at`, `sign_in_at`, and the commands `sign_up` and `sign_in`. Nothing else in the crate calls them — `grep -rn "sign_up\|sign_in_at\|sign_in(" app/src` before deleting, and the only hits must be in this file and in `main.rs`'s two lists (hand-off H1).
- Produces: `magic_link_at` with `create_user: true` and the three consent keys.

Spec D4 and D5. Existing password accounts are not locked out: `/otp` to a confirmed address sends a code to that address, and Google links its identity to an existing verified email rather than creating a second user.

- [ ] **Step 1: The test first** — in `app/tests/account.rs`, replace the `sign_up`/`sign_in` tests with:

```rust
#[test]
fn the_code_request_creates_the_account_and_carries_the_three_things_the_trigger_wants() {
    use knowlu::account::magic_link_at;
    let (base, handle) = loopback(vec![(200, "{}".to_string())]);
    let out = magic_link_at(&format!("{base}/auth/v1"), "anon-key", "n@example.invalid", "2026-09-10", "2026-09-17");
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{out:?}");
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/otp "), "{req}");
    // `create_user: false` is what made a new student's first press answer "Signups not allowed for
    // otp". One field, one button, one code — there is no separate create step to fall back to.
    assert!(req.contains("\"create_user\":true"), "{req}");
    // `data` becomes `raw_user_meta_data` on the user GoTrue creates, which is exactly what
    // `handle_new_user()` reads. The email path therefore still takes the trigger's original,
    // unchanged branch; only Google takes the new one.
    assert!(req.contains("\"age_attested\":\"true\""), "{req}");
    assert!(req.contains("\"tos_version\":\"2026-09-10\""), "{req}");
    assert!(req.contains("\"privacy_version\":\"2026-09-17\""), "{req}");
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
    assert!(!src.to_lowercase().contains("password"), "not even the word");
}
```

- [ ] **Step 2: The implementation** — in `app/src/account.rs`:

```rust
/// One button: the code that both creates the account and signs in. `create_user: true` is the whole
/// difference from C1's version, and `data` is what the auth trigger reads out of
/// `raw_user_meta_data` — the same three keys the deleted `sign_up` used to send, now sent by the
/// only path that remains.
pub fn magic_link_at(auth_base: &str, anon: &str, email: &str, tos: &str, privacy: &str) -> Result<(), String> {
    let body = json!({
        "email": email,
        "create_user": true,
        "data": { "age_attested": "true", "tos_version": tos, "privacy_version": privacy },
    });
    let (status, v) = post_json(&format!("{auth_base}/otp"), anon, &body)?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

#[tauri::command(async)]
pub fn send_magic_link(email: String, age_attested: bool) -> Value {
    if !age_attested {
        return json!({ "ok": false, "error": "Knowlu is for people 18 or older." });
    }
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    match magic_link_at(&auth, &anon, email.trim(), TOS_VERSION, PRIVACY_VERSION) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}
```

…and delete `sign_up_at`, `sign_in_at`, `sign_up` and `sign_in` outright, with their doc comments. `verify_email_code` is untouched: `VERIFY_TYPE` is still `magiclink`, settled against staging on 2026-09-10, and the code in the mail is still GoTrue's `{{ .Token }}`.

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

- [ ] **Step 4: Green, then commit.** `cargo test --workspace` at 0 warnings; `deno fmt --check`. Commit `app/src/account.rs`, `app/tests/account.rs`, `cloud/supabase/config.toml` by name. The page still calls `send_magic_link` with one argument at this point and will fail on the new `ageAttested` — Task 5 is what fixes it, and the two land in the same branch.

---

### Task 5: The account panel and the upgrade overlay — Continue with Google first, one email field, one button

**Files:**
- Modify: `app/static/index.html`, `app/static/console.js`, `app/static/console.css`
- Test: `app/tests/static_assets.rs`
- Hand-off: **H2** (`scripts/wizard-check.py`), applied by the controller during this task

**Interfaces:**
- Consumes: Task 3's `google_sign_in`, Task 4's `send_magic_link(email, ageAttested)`.
- Produces: `#wiz-google-signin`, `#up-google`; `#wiz-pw` and `#up-pw` are gone.

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

…and in `the_page_has_no_lms_credential_field_anywhere`, `OURS` becomes `["wiz-zy-pass", "wiz-vhl-pass"]`, `seen >= 3` becomes `assert_eq!(seen, 2, "the two coursework logins are the only passwords Knowlu ever asks for")`, and the `for id in [...]` loop drops `wiz-pw`.

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

The upgrade overlay (`id="upgrade"`) gets the same three rows in the same order — `#up-google`, `#up-email` with `#up-magic`, `#up-code-row` with `#up-code` and `#up-code-go` — and loses `#up-pw`, `#up-create` and `#up-signin`. Its two checkboxes (`#up-18`, `#up-terms`) stay exactly where they are.

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

The console's own upgrade-overlay handlers (the `#up-create`/`#up-signin` branch around line 1259) are replaced the same way, calling `google_sign_in` and `send_magic_link` and then the overlay's existing `attach_account` path.

- [ ] **Step 4: The CSS** — `console.css` gains one rule beside the two that already exist for `.flagpop` and `.set-row`:

```css
/* Spec §7 defect 1: the wizard's nav had NO disabled rule, so a disabled Back or Next was full
   contrast, full colour and silently inert — which is what "why are there even back and next
   buttons if they don't work" was looking at. */
.wiz-nav button.b[disabled], .wiz-row button.b[disabled] { opacity: .5; cursor: not-allowed; }
.wiz-nav button.b[disabled]:hover, .wiz-row button.b[disabled]:hover { border-color: var(--hair-2); color: var(--t2); }
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
    // (c) A refused Next says what is missing, in a sentence, and says it again on a second press —
    // a red line that was already on screen does not read as a new answer.
    assert!(js.contains("function wizValid("), "wizValid");
    assert!(js.contains("flashError("), "a repeated refusal is re-announced, not silently unchanged");
    for sentence in ["Sign in first.", "Finish the payment page in your browser, then come back."] {
        assert!(js.contains(sentence), "the refusal names what is missing: {sentence}");
    }
}
```

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

`WIZ` gains `busy: false` (Task 5 adds it for the Google button; the two uses are the same flag — the wizard is doing something and Next must wait). `wizGo`'s `EL("wiz-next").disabled = true` / `= false` (console.js:1584, :1601) become `WIZ.busy = true` / `WIZ.busy = false` with a `renderWizard()` after each; `wizFinish`'s line 1631 and its three re-enable sites do the same; `credentialsStranded`'s re-enable becomes `WIZ.busy = false` before its `renderWizard()`.

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

- [ ] **Step 2: The version, and the pin.** `account::PRIVACY_VERSION` moves from `"2026-09-16"` to `"2026-09-17"` in the same commit — `account.rs`'s own comment requires the constant and the page's date to move together, or the consent log points at text nobody can find. `TOS_VERSION` does **not** move: `site/terms.html` describes a subscription and an account, not an authentication method, and nothing in it changed. Extend the existing cross-file pin in `app/tests/static_assets.rs`:

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

Then, with **P2 done** (Quinn's `supabase config push` carrying `GOOGLE_CLIENT_ID`, `GOOGLE_SECRET` and `enable_confirmations = false`), the two findings are checked against the live service rather than against a reading of a source file:

- **The allow-list (spec §4).** `GET <staging>/auth/v1/authorize?provider=google&redirect_to=http%3A%2F%2F127.0.0.1%3A49999%2Fcallback&code_challenge=x&code_challenge_method=s256` must answer `302` with a `Location` on `accounts.google.com`. If it answers `302` back to `https://knowlu.com` instead, the loopback rule did not apply on this GoTrue build and `additional_redirect_urls` gains `"http://127.0.0.1:*/callback"` — **stop and report it**, because it changes the spec's finding and Quinn should hear it.
- **The mail (spec §5.2).** `POST <staging>/auth/v1/otp` for an address that has never signed up, then read the mail: it must be Knowlu's template with a six-digit code, not Supabase's *Confirm your signup*.

- [ ] **Step 4: The live proof, by the controller, on a scratch profile.** Spec D1 end to end, against staging, never a live vault (`CLAUDE.md`, desktop safety):

```powershell
$env:KNOWLU_API_BASE = "https://<staging-ref>.supabase.co/functions/v1"
$env:KNOWLU_ANON_KEY  = "<the staging anon key>"
scripts\scratch-vault.ps1 -Source <a scratch vault>
```

…launch the app into the wizard, press **Continue with Google**, complete the consent in the browser that opens, and confirm: the browser shows one sentence; the wizard advances to the subscribe panel with *Signed in as …*; `knowlu/pending/session` exists in Credential Manager; `select id, email, age_attested_at from accounts` on staging has the row with a non-null `age_attested_at`; and `select kind, version from consents where account_id = …` has the three rows. Then press the subscribe button, type **P3's test-mode 100-percent promotion code** on Stripe's page, and confirm Checkout completes **with no card asked for** and the wizard's poll advances to the name panel.

- [ ] **Step 5: The gate, the hand-offs, the close.** Run the whole gate below. Ask the controller for **H3** (`CLAUDE.md`'s recount and the no-password sentence) and **H4** (`HANDOFF.md` §3). Production — `supabase db push` and the two `functions deploy` against `knowlu-prod`, plus Quinn's second `config push` — is **Quinn's**, after staging is green, and is the one place in this plan where prod is named.

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
| D6 | Consent moves out of the trigger into `POST /account/consent`; `billing-checkout` keeps the tooth | spec §1 D6, §5.1 | Tasks 1 and 2. The migration creates no table and no policy, so `migrations_test.ts`'s three invariants are untouched |
| D7 | A new Google Cloud project, "In production", basic scopes only | spec §1 D7, §3 | P1, and `[auth.external.google]` in `config.toml` pushed by P2. The restricted-scope project and C1's P4 are not touched |
| D8 | `allow_promotion_codes` and `payment_method_collection: if_required` | spec §1 D8, §8 | Task 2. §11 R2's card-up-front is relaxed only where Stripe itself decides no method is needed, and the test says so beside the assertion |
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
2. **The account exists server-side and is complete.** On staging: the `accounts` row for that sign-up has a non-null `age_attested_at`, `tos_version` and `privacy_version`; `consents` has its three rows; `entitlements` has its `none` row. Without Task 1's migration this sign-up is a 500, so item 2 is what proves the finding was real and is fixed.
3. **The email path is one field and one button.** A fresh address, **Email me a code**, one mail — **Knowlu's template, with a six-digit code** — the code typed, signed in. No password field exists anywhere on the page (`the_page_has_no_lms_credential_field_anywhere` now counts exactly two, both coursework), and no password path exists in the crate (`there_is_no_password_path_left_in_the_crate`).
4. **An account made before C1b still works.** An address that signed up with a password in C1 receives a code and signs in with it; a Google sign-in on that same verified address links an identity to the existing user rather than creating a second one. Checked on staging against an account created before this branch.
5. **Back and Next behave.** `scripts/wizard-check.py` exits `ok`, its check 8 included — five Backs to the name panel, a rename, five Nexts to Finish, the summary following the rename. In the app: Back is absent at step 0 and present everywhere else; a refused Next paints a sentence that names what is missing and re-announces it on a second press; no wizard button is ever greyed without looking greyed.
6. **Checkout takes a code and asks for no card when none is owed.** With P3's **test-mode** 100-percent promotion code, Checkout completes with no payment method collected, the webhook writes `entitlements`, and the wizard's poll advances to the name panel. With no code, Stripe still collects a card for the 7-day trial.
7. **The 18+ gate still has a server-side tooth.** A staging account whose `age_attested_at` is null gets `403 "the 18+ attestation is missing"` from `billing-checkout`, and no Stripe customer, no consent row and no session are created. (Driven by nulling the column on a scratch account.)
8. **The two live findings are confirmed, not assumed.** The loopback `redirect_to` on an arbitrary high port answers `302` to `accounts.google.com` with no allow-list entry (spec §4), and `/otp` to a never-seen address sends Knowlu's template, not Supabase's (spec §5.2). Either one failing is reported to Quinn before the branch is closed.
9. **The policy and the app agree.** `site/privacy.html` is version `2026-09-17`, names Google sign-in, and no longer mentions a password hash; `account::PRIVACY_VERSION` is the same string; the cross-file pin passes. Quinn has read the three sentences (P4).
10. `cargo test --workspace` green at **0 warnings** (the `.rsrc` line accepted); `deno test`, `deno lint` and `deno fmt --check` green; `git ls-files --eol` unchanged for everything this stream did not add; `python scripts/wizard-check.py` prints `ok`.
11. `git diff --name-only main...c1b-sign-in` touches nothing outside this stream's ownership except the four hand-off files, each applied by the controller and named in H1-H4.

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
